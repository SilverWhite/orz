//! Session bootstrap — workspace trust, envelope init, journal creation,
//! IP5 snapshot store, permit keystore.
//!
//! This is where orz-host wires the Grok providers into the assurance journal.
//! Every run starts here: trust verification → journal creation → run_preflight.
//!
//! See: INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.2 §3.3

use std::path::{Path, PathBuf};
use std::sync::Arc;

use orz_assurance::gates::ipg::WorkspaceTrust;
use orz_assurance::journal::JournalRecorder;
use orz_assurance::permit::PermitSigner;
use orz_assurance::session::snapshot::SnapshotStore;
use orz_assurance::{EventType, Redaction, RunEvent};

use crate::keystore::{
    KeystoreError, MemoryInstallationKeyStore, WindowsDpapiInstallationKeyStore,
};

/// Error during session bootstrap.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("workspace not trusted: {0}")]
    TrustFailed(String),
    #[error("journal error: {0}")]
    Journal(#[from] orz_assurance::JournalRecorderError),
    #[error("snapshot store error: {0}")]
    Snapshot(#[from] orz_assurance::session::snapshot::SnapshotError),
    #[error("keystore error: {0}")]
    Keystore(#[from] KeystoreError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Trust enforcement policy for bootstrap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustPolicy {
    /// Enforce workspace trust, fail-closed. In headless mode a `Prompt`
    /// outcome is treated as untrusted.
    Enforce,
    /// Skip the trust gate (tests / explicit opt-out).
    Skip,
}

/// Result of bootstrapping a new session.
pub struct SessionHandle {
    pub run_id: String,
    pub run_manifest_sha256: String,
    pub journal: JournalRecorder,
    pub journal_dir: PathBuf,
    /// Sequence number of the next event to append (continues the chain).
    pub next_sequence: u64,
    /// SHA-256 of the last appended event (chain link for the next event).
    pub last_event_sha256: Option<String>,
    /// Workspace trust observation (feeds the IPG `trusted_project` rule).
    pub workspace_trust: WorkspaceTrust,
    /// IP5 pre-mutation snapshot store, scoped to the session worktree
    /// (root: `{cwd}/.gsa/snapshots/`). Wired into the loop's ToolDispatcher.
    pub snapshot_store: Arc<SnapshotStore>,
    /// Keystore-backed permit signer (DPAPI install key under `Enforce`;
    /// test-only memory store under `Skip`). Consumed by permit issuance.
    pub permit_signer: Arc<dyn PermitSigner>,
}

/// Evaluate workspace trust for `cwd` (fail-closed, non-interactive).
///
/// Uses `decide(feature_enabled = true, …)` with an explicit flag — the
/// orz-workspace default `feature_enabled()` returns `false` on local/dev
/// builds (no `GROK_VERSION` release stamp), which auto-trusts everything
/// and would silently void the fail-closed guarantee.
///
/// Known provider limitation (2026-08-04 review P2-9): `folder_trust`
/// records `key_recordable=false` (e.g. cwd = `$HOME` or non-absolute paths)
/// as `Trusted` even when repo configs are present. This is inherited
/// provider semantics, not a bug in this wrapper — Phase 3 policy work
/// should decide whether orz-host overrides it.
pub fn check_workspace_trust(cwd: &Path) -> Result<WorkspaceTrust, SessionError> {
    let key = orz_workspace::trust::workspace_key(cwd);
    let inputs = orz_workspace::folder_trust::decide_inputs_with_interactive(cwd, &key, false);
    match orz_workspace::folder_trust::decide(true, &inputs) {
        orz_workspace::folder_trust::TrustOutcome::Trusted => Ok(WorkspaceTrust::ObservedTrusted),
        orz_workspace::folder_trust::TrustOutcome::Prompt
        | orz_workspace::folder_trust::TrustOutcome::Untrusted => Err(SessionError::TrustFailed(
            format!("workspace not trusted: {}", cwd.display()),
        )),
    }
}

/// Bootstrap a new agent session.
///
/// 1. (Enforce policy) verify workspace trust, fail-closed
/// 2. Determine journal directory (`.gsa/runs/{run_id}/`)
/// 3. Create the JournalRecorder
/// 4. Record run_preflight event
/// 5. Return SessionHandle ready for AgentLoopController
pub async fn bootstrap_session(
    run_id: &str,
    base_dir: Option<PathBuf>,
    policy: TrustPolicy,
) -> Result<SessionHandle, SessionError> {
    // `base_dir` is the session working directory; journals always live under
    // `{cwd}/.gsa/runs/{run_id}/` (CLI default: `./.gsa/runs/…`).
    let cwd = base_dir.unwrap_or_else(|| PathBuf::from("."));
    let workspace_trust = if policy == TrustPolicy::Enforce {
        check_workspace_trust(&cwd)?
    } else {
        WorkspaceTrust::NotObserved
    };
    let base = cwd.join(".gsa");
    let journal_dir = base.join("runs").join(run_id);

    // IP5 pre-mutation snapshot store — session-scoped, root under
    // `{cwd}/.gsa/snapshots/` (the `.gsa` tree is gitignored and excluded
    // from the agent-visible read scope).
    let snapshot_store = Arc::new(SnapshotStore::new(base.join("snapshots"), cwd.clone())?);

    // Permit signing key — an install-level keystore under `Enforce`
    // (Windows DPAPI-backed, fail-closed elsewhere); the test-only memory
    // store under `Skip` (mirrors the Python dev bridge, which also uses
    // `MemoryInstallationKeyStore`). The key is workspace-scoped
    // (`{cwd}/.gsa/keystore/`), so it persists across runs of the same
    // workspace; an install-level location is a later refinement (2026-08-05
    // review P3-3).
    let permit_signer: Arc<dyn PermitSigner> = match policy {
        TrustPolicy::Enforce => Arc::new(WindowsDpapiInstallationKeyStore::create_or_load(
            &base.join("keystore"),
        )?),
        TrustPolicy::Skip => Arc::new(MemoryInstallationKeyStore::new()),
    };

    let journal = JournalRecorder::new(journal_dir.clone());

    // Compute a manifest SHA-256
    let manifest = serde_json::json!({
        "run_id": run_id,
        "created_at": chrono::Utc::now().to_rfc3339(),
        "schema_version": "0.1.0-draft",
    });
    let manifest_sha256 =
        orz_assurance::sha256_hex(&orz_assurance::canonical_json(&manifest).unwrap_or_default());

    // Record run_preflight as event 0
    let mut preflight = RunEvent::new(
        run_id.into(),
        0,
        EventType::RunPreflight,
        manifest_sha256.clone(),
        None,
        "run-event-v0.1.schema.json".into(),
        manifest,
        Redaction::None,
        chrono::Utc::now().to_rfc3339(),
    );
    orz_assurance::seal_event(&mut preflight)
        .map_err(|e| SessionError::Journal(orz_assurance::JournalRecorderError::Serde(e)))?;

    // The chain continues from the sealed preflight event (read before move).
    let next_sequence = preflight.sequence + 1;
    let last_event_sha256 = Some(preflight.event_sha256.clone());

    journal.record_async(preflight).await?;

    Ok(SessionHandle {
        run_id: run_id.into(),
        run_manifest_sha256: manifest_sha256,
        journal,
        journal_dir,
        next_sequence,
        last_event_sha256,
        workspace_trust,
        snapshot_store,
        permit_signer,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn test_dir() -> PathBuf {
        let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "orz-host-session-test-{}-{}",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn bootstrap_creates_journal_with_preflight() {
        let base = test_dir();
        let run_id = "RUN-BOOTSTRAP-TEST";

        let handle = bootstrap_session(run_id, Some(base.clone()), TrustPolicy::Skip)
            .await
            .unwrap();

        assert_eq!(handle.run_id, run_id);
        assert!(!handle.run_manifest_sha256.is_empty());
        // Skip policy → trust unobserved (fail-closed default for IPG).
        assert_eq!(handle.workspace_trust, WorkspaceTrust::NotObserved);

        // Flush first to ensure events are written (journal writes are async)
        handle.journal.flush_async().await.unwrap();

        // Verify preflight event was recorded
        let replay = orz_assurance::replay_journal(
            &handle.journal_dir.join("events.jsonl"),
            Some(run_id),
            None,
            false, // don't require terminal — session is still open
        );
        assert!(replay.valid, "journal errors: {:?}", replay.errors);
        assert_eq!(replay.event_count, 1);

        let _ = std::fs::remove_dir_all(&base);
    }

    /// IP5 + P1 wiring at bootstrap: the snapshot store is session-scoped to
    /// the worktree (root `{cwd}/.gsa/snapshots/`) and the permit signer is a
    /// keystore-backed key id (`KEY-…`).
    #[tokio::test]
    async fn bootstrap_wires_snapshot_store_and_permit_signer() {
        let base = test_dir();
        let handle = bootstrap_session("RUN-WIRED", Some(base.clone()), TrustPolicy::Skip)
            .await
            .unwrap();

        // IP5: store scoped to the session worktree, root under `.gsa`.
        assert_eq!(handle.snapshot_store.worktree(), &base);
        assert!(
            handle.snapshot_store.root().starts_with(base.join(".gsa")),
            "store root: {}",
            handle.snapshot_store.root().display()
        );
        // Usable immediately — explicit target track round-trips.
        let target = base.join("a.txt");
        std::fs::write(&target, "original").unwrap();
        let record = handle
            .snapshot_store
            .track(&[PathBuf::from("a.txt")])
            .await
            .unwrap();
        assert_eq!(record.entries.len(), 1);
        assert_eq!(
            handle
                .snapshot_store
                .verify(&record.snapshot_hash)
                .await
                .unwrap(),
            orz_assurance::session::snapshot::SnapshotVerifyOutcome::Clean
        );

        // P1: keystore-backed signer injected.
        assert!(
            handle.permit_signer.key_id().starts_with("KEY-"),
            "key id: {}",
            handle.permit_signer.key_id()
        );
        let signature = handle.permit_signer.sign(b"payload").unwrap();
        assert!(handle.permit_signer.verify(b"payload", &signature));

        let _ = std::fs::remove_dir_all(&base);
    }

    /// Folder-trust gate semantics, asserted on the pure `decide` combination.
    ///
    /// orz-workspace's folder-trust gate is inverted vs. naive intuition:
    /// a directory WITHOUT repo-controlled trust-sensitive config is trusted;
    /// a directory CARRYING such config (`.mcp.json`, `.grok/config.toml`
    /// permission rules, `.envrc`, `.grok/roles`, …) is gated — non-interactive
    /// headless runs fail closed. `check_workspace_trust` must pass the
    /// `feature_enabled = true` flag explicitly, because the default
    /// `feature_enabled()` returns `false` on dev builds (no `GROK_VERSION`
    /// release stamp) — which would auto-trust everything, silently voiding
    /// the fail-closed guarantee.
    #[test]
    fn trust_gate_fails_closed_and_flag_is_explicit() {
        let clean_inputs = orz_workspace::folder_trust::DecideInputs {
            store_trusted: false,
            repo_configs_present: false,
            is_interactive: false,
            key_recordable: true,
        };
        // Clean dir (no repo config) → trusted, with or without the flag.
        assert_eq!(
            orz_workspace::folder_trust::decide(true, &clean_inputs),
            orz_workspace::folder_trust::TrustOutcome::Trusted
        );

        // Repo-config dir → gated. Dev-build default would silently pass.
        let config_inputs = orz_workspace::folder_trust::DecideInputs {
            repo_configs_present: true,
            ..clean_inputs
        };
        assert_eq!(
            orz_workspace::folder_trust::decide(false, &config_inputs),
            orz_workspace::folder_trust::TrustOutcome::Trusted,
            "dev-build auto-trust would silently pass"
        );
        // Our explicit flag must reject (non-interactive → Untrusted).
        assert_eq!(
            orz_workspace::folder_trust::decide(true, &config_inputs),
            orz_workspace::folder_trust::TrustOutcome::Untrusted
        );

        // store-trusted always passes.
        let store_trusted = orz_workspace::folder_trust::DecideInputs {
            store_trusted: true,
            ..config_inputs
        };
        assert_eq!(
            orz_workspace::folder_trust::decide(true, &store_trusted),
            orz_workspace::folder_trust::TrustOutcome::Trusted
        );
    }

    #[test]
    fn clean_workspace_accepts() {
        // A clean temp dir (no repo-controlled config) passes the gate.
        let dir = test_dir();
        let result = check_workspace_trust(&dir);
        assert!(
            result.is_ok(),
            "clean workspace should be trusted, got {result:?}"
        );
        assert_eq!(result.unwrap(), WorkspaceTrust::ObservedTrusted);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn repo_config_workspace_fails_closed() {
        // A directory carrying repo-controlled trust-sensitive config
        // (`.grok/roles/`, detected directly at cwd without a git chain) is
        // gated; headless (non-interactive) runs fail closed.
        let dir = test_dir();
        std::fs::create_dir_all(dir.join(".grok").join("roles")).unwrap();
        let result = check_workspace_trust(&dir);
        assert!(result.is_err(), "expected TrustFailed, got {result:?}");
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not trusted"), "{err}");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
