//! A5 (2026-08-08): `.gsa` record-tree retention — default 7 days,
//! configurable via `ORZ_RETENTION_DAYS` (0 disables the sweep).
//!
//! Sweep scope (all best-effort — a sweep failure never fails the caller):
//! - `runs/` — run journals (`RUN-*` / `RST-*` dirs) older than the cutoff
//!   are removed whole (the current run's dir is protected by name).
//! - `snapshots/` — manifests older than the cutoff are removed, then
//!   objects NOT referenced by any remaining manifest are GC'd (content-
//!   addressed objects are shared across snapshots, so age alone is never
//!   enough — a fresh snapshot may reference an old object).
//! - `one_shot_permit/` — consumed single-use claim files older than the
//!   cutoff are removed.
//! - `conversations/` (GAP-CONVERSATION-RESTORE 2026-08-10) — session
//!   conversation sidecars older than the cutoff are removed (the largest
//!   `.gsa` files; a LIVE session's sidecar is rewritten on every successful
//!   prompt, so its mtime is always fresh and it naturally survives).
//! - `project-doc-index/` (GAP-PROJECT-DOC-INDEX-CACHE 2026-08-11) — the
//!   doc-index cache file, swept by age like conversations. The cache is
//!   100% rebuildable and written best-effort, so deleting it is always
//!   safe (the next query rebuilds it in one pass) — the contrast with
//!   `keystore/` below, which must NEVER be swept, is exactly the
//!   "rebuildable → sweepable" judgement.
//! - `compaction/` (P0-D S3 2026-08-14) — five-section template summary
//!   archives (`.gsa/compaction/compaction-<run>-<seq>.md`). The archive is
//!   an audit copy (the journal carries the same summary digest); the
//!   rolling marker is only a pointer, so sweeping old archives is safe —
//!   the same "rebuildable/audit → sweepable" judgement as conversations.
//! - `keystore/` is NEVER swept: deleting the installation key would
//!   silently invalidate every permit signed under it (a permit issued
//!   under key A fails verification under key B).
//!
//! The sweep runs at session bootstrap (every entry point funnels through
//! `session::bootstrap_session`); the debug value of run journals stands on
//! its own and does not depend on the compaction design (design §5 A5).

use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;

/// Default retention window (design §5 A5: `.gsa` 默认保留 7 天).
pub const DEFAULT_RETENTION_DAYS: u64 = 7;

/// Env override for the retention window (`ORZ_RETENTION_DAYS`; 0 disables
/// the sweep). Invalid values fall back to the default.
pub fn retention_days_override() -> Option<u64> {
    std::env::var("ORZ_RETENTION_DAYS")
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

/// Effective retention window: override or default.
pub fn retention_days() -> u64 {
    retention_days_override().unwrap_or(DEFAULT_RETENTION_DAYS)
}

/// Cutoff for a retention window: `now` minus `days`. A cutoff computed
/// with `now` in the past/future is the test seam (deterministic age
/// sweeps without mtime manipulation).
///
/// Overflow-safe (review P2-1, 2026-08-08): a pathological `days` value
/// (e.g. a mis-typed env like `ORZ_RETENTION_DAYS=99999999999999999`)
/// would make `now - days` panic — a sweep must NEVER fail its caller,
/// so the cutoff clamps to the Unix epoch (everything is old) instead.
pub fn retention_cutoff(now: SystemTime, days: u64) -> SystemTime {
    now.checked_sub(std::time::Duration::from_secs(
        days.saturating_mul(24 * 60 * 60),
    ))
    .unwrap_or(std::time::UNIX_EPOCH)
}

/// What a sweep removed (for logging / tests).
#[derive(Debug, Default, PartialEq, Eq)]
pub struct PruneReport {
    pub removed_run_dirs: Vec<String>,
    pub removed_manifests: Vec<String>,
    pub removed_objects: Vec<String>,
    pub removed_permit_files: Vec<String>,
    /// local_browser (2026-08-10): orphaned headless-browser profiles
    /// (`chrome-profile-*` dirs under `.gsa/`) — swept by age like runs.
    pub removed_browser_profiles: Vec<String>,
    /// GAP-CONVERSATION-RESTORE (2026-08-10): session conversation sidecars
    /// (`conversations/<session8>.json`) — swept by age like permit claims.
    pub removed_conversation_sidecars: Vec<String>,
    /// GAP-PROJECT-DOC-INDEX-CACHE (2026-08-11): the doc-index cache file
    /// (`project-doc-index/cache.json`) — rebuildable, so swept by age.
    pub removed_project_doc_caches: Vec<String>,
    /// P0-D S3 (2026-08-14): compaction summary archives
    /// (`compaction/*.md`) — audit copies, swept by age (the journal and
    /// the rolling marker carry the digest/pointer).
    pub removed_compaction_archives: Vec<String>,
    /// v1.15 (2026-08-14) / v1.15⑧ (2026-08-15): blackboard plan-epoch
    /// snapshots (`blackboard/epoch-*.json`) — full path/action archives of
    /// rotated epochs; swept by age (retention 7 days) EXCEPT the
    /// highest-numbered epoch, which is the restore entry for a live board
    /// and is therefore always kept.
    pub removed_blackboard_epoch_archives: Vec<String>,
    /// PDF evidence (2026-08-11): content-addressed PDF evidence documents
    /// (`pdf-evidence/{p2}/{full64}/`) — rebuildable from the source URL, so
    /// swept by age (document ids referenced by older runs lapse — explicit
    /// `[pdf_read_not_found]`, same semantics as lapsed conversations).
    pub removed_pdf_evidence_dirs: Vec<String>,
    /// PDF evidence (2026-08-11): browser download staging dirs
    /// (`pdf-downloads-*`) — disposable, swept by age.
    pub removed_pdf_download_dirs: Vec<String>,
}

/// Best-effort retention sweep over `gsa_root` (the `.gsa` directory).
/// `cutoff` = oldest acceptable mtime; `keep_run_id` = the active run dir,
/// protected by name. Individual IO errors are skipped (fail-safe: a
/// directory that cannot be read is KEPT), never propagated.
pub fn prune_old_records(
    gsa_root: &Path,
    cutoff: SystemTime,
    keep_run_id: Option<&str>,
) -> PruneReport {
    let mut report = PruneReport::default();
    prune_run_dirs(&mut report, &gsa_root.join("runs"), cutoff, keep_run_id);
    prune_snapshots(&mut report, &gsa_root.join("snapshots"), cutoff);
    prune_old_files(
        &mut report.removed_permit_files,
        &gsa_root.join("one_shot_permit"),
        cutoff,
    );
    // GAP-CONVERSATION-RESTORE (2026-08-10): conversation sidecars swept by
    // age (the largest `.gsa` files; a live session's sidecar is rewritten
    // on every successful prompt — mtime fresh, naturally kept).
    prune_old_files(
        &mut report.removed_conversation_sidecars,
        &gsa_root.join("conversations"),
        cutoff,
    );
    // GAP-PROJECT-DOC-INDEX-CACHE (2026-08-11): the doc-index cache is
    // rebuildable and written best-effort, so age-sweeping it is always
    // safe — the next query rebuilds it in one pass.
    prune_old_files(
        &mut report.removed_project_doc_caches,
        &gsa_root.join("project-doc-index"),
        cutoff,
    );
    // P0-D S3 (2026-08-14): compaction summary archives — the marker is a
    // rolling single pointer and the journal carries the digest, so old
    // archive files are sweepable by age (retention 7 days).
    prune_old_files(
        &mut report.removed_compaction_archives,
        &gsa_root.join("compaction"),
        cutoff,
    );
    // v1.15⑧ (2026-08-15): blackboard plan-epoch snapshots — swept by age
    // EXCEPT the highest-numbered epoch (the restore entry). Keeping the
    // latest archive also keeps `latest_epoch_snapshot` working for
    // long-lived boards whose current epoch file is older than the cutoff.
    prune_blackboard_epochs(
        &mut report.removed_blackboard_epoch_archives,
        &gsa_root.join("blackboard"),
        cutoff,
    );
    // local_browser (2026-08-10): orphaned browser profiles — a session
    // whose close_session ran after a hard crash leaves its `chrome-profile-*`
    // dir behind; age-based sweep covers it (a LIVE session's profile is
    // recent by definition; the active run's own browser stays untouched).
    prune_old_dirs(
        &mut report.removed_browser_profiles,
        gsa_root,
        cutoff,
        "chrome-profile-",
    );
    // PDF evidence (2026-08-11): content-addressed evidence docs
    // (`pdf-evidence/{p2}/{full64}/`) — the two-level layout is swept
    // deepest-first; the evidence dir itself and the `{p2}` shard dirs are
    // never removed (they have no age semantics of their own).
    prune_pdf_evidence(&mut report, &gsa_root.join("pdf-evidence"), cutoff);
    // PDF evidence (2026-08-11): browser download staging dirs — crash
    // leftovers from interrupted downloads (ingest deletes them on success;
    // retention is the backstop).
    prune_old_dirs(
        &mut report.removed_pdf_download_dirs,
        gsa_root,
        cutoff,
        "pdf-downloads-",
    );
    report
}

/// Sweep `pdf-evidence/{p2}/{full64}/` document dirs older than `cutoff`.
/// Shard dirs (`.gsa/pdf-evidence/{p2}`) are container entries — swept
/// inside-out per document, never removed themselves. Any over-age `{full64}`
/// dir is removed regardless of its contents (evidence is 100% rebuildable
/// from the source URL — review 2026-08-11 C3-4: the fail-safe only applies
/// to a dir that cannot be statted at all, via `entry_older_than`).
fn prune_pdf_evidence(report: &mut PruneReport, evidence_dir: &Path, cutoff: SystemTime) {
    let Ok(shards) = std::fs::read_dir(evidence_dir) else {
        return; // no evidence tree yet
    };
    for shard in shards.flatten() {
        let shard_path = shard.path();
        let Ok(docs) = std::fs::read_dir(&shard_path) else {
            continue;
        };
        for doc in docs.flatten() {
            let path = doc.path();
            if !entry_older_than(&path, cutoff) {
                continue;
            }
            let name = doc.file_name().to_string_lossy().into_owned();
            if std::fs::remove_dir_all(&path).is_ok() {
                report.removed_pdf_evidence_dirs.push(name);
            }
        }
    }
}

/// Prune first-level dirs under `root` whose names start with `prefix` and
/// whose mtime predates `cutoff` (fail-safe: unreadable entries are kept).
fn prune_old_dirs(removed: &mut Vec<String>, root: &Path, cutoff: SystemTime, prefix: &str) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with(prefix) {
            continue;
        }
        let path = root.join(&name);
        if !entry_older_than(&path, cutoff) {
            continue;
        }
        if std::fs::remove_dir_all(&path).is_ok() {
            removed.push(name);
        }
    }
}

/// Whether `path` (a run dir or file) is older than `cutoff`. Run dirs are
/// judged by their `events.jsonl` mtime when present (journal writes touch
/// it on every event) and by the dir mtime otherwise; a path that cannot
/// be statted is kept (fail-safe).
fn entry_older_than(path: &Path, cutoff: SystemTime) -> bool {
    let journal_mtime = std::fs::metadata(path.join("events.jsonl"))
        .and_then(|m| m.modified())
        .ok();
    let mtime = match journal_mtime {
        Some(t) => Some(t),
        None => std::fs::metadata(path).and_then(|m| m.modified()).ok(),
    };
    mtime.is_some_and(|t| t < cutoff)
}

fn prune_run_dirs(
    report: &mut PruneReport,
    runs_dir: &Path,
    cutoff: SystemTime,
    keep_run_id: Option<&str>,
) {
    let Ok(entries) = std::fs::read_dir(runs_dir) else {
        return; // no runs tree yet
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        // `GRILL-` (2026-08-08 grill mode): each grill turn bootstraps a
        // dir holding only the mechanical run_preflight (open chain — a
        // grill turn is not a run); swept by age like RUN-/RST- so long
        // grill sessions cannot accumulate unbounded .gsa dirs.
        if !(name.starts_with("RUN-") || name.starts_with("RST-") || name.starts_with("GRILL-")) {
            continue;
        }
        if keep_run_id.is_some_and(|k| k == name) {
            continue; // the active run is never swept
        }
        if entry_older_than(&entry.path(), cutoff) && std::fs::remove_dir_all(entry.path()).is_ok()
        {
            report.removed_run_dirs.push(name);
        }
    }
}

fn prune_snapshots(report: &mut PruneReport, snapshots_dir: &Path, cutoff: SystemTime) {
    let manifests_dir = snapshots_dir.join("manifests");
    let objects_dir = snapshots_dir.join("objects");
    // Remaining manifests' referenced object hashes (the GC keep-set).
    let mut referenced: HashSet<String> = HashSet::new();
    // Review P3-1 (2026-08-08): a KEPT manifest that fails to parse could
    // be the only referencer of objects — GC-ing them would break even
    // manual repair. Fail-safe: when any kept manifest is unreadable, the
    // whole object GC is skipped this sweep (age-based manifest pruning
    // still runs).
    let mut kept_manifest_unreadable = false;
    if let Ok(entries) = std::fs::read_dir(&manifests_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.ends_with(".json") {
                continue;
            }
            if entry_older_than(&path, cutoff) {
                if std::fs::remove_file(&path).is_ok() {
                    report.removed_manifests.push(name);
                }
                continue;
            }
            // Keep this manifest — collect the objects it references.
            match std::fs::read(&path).and_then(|bytes| {
                serde_json::from_slice::<Vec<orz_assurance::session::snapshot::SnapshotEntry>>(
                    &bytes,
                )
                .map_err(std::io::Error::other)
            }) {
                Ok(snapshot_entries) => {
                    referenced.extend(snapshot_entries.iter().map(|e| e.object_sha256.clone()));
                }
                Err(e) => {
                    // Review D2-1: visibility — a half-written (non-atomic
                    // manifest write) or version-mismatched manifest must
                    // not vanish silently.
                    tracing::warn!(
                        manifest = %name,
                        error = %e,
                        "kept snapshot manifest unreadable — object GC skipped this sweep"
                    );
                    kept_manifest_unreadable = true;
                }
            }
        }
    }
    if kept_manifest_unreadable {
        return;
    }
    // Object GC (review D2-1, 2026-08-08): delete only objects that are
    // BOTH unreferenced by any remaining manifest AND older than the
    // cutoff. The age gate closes a concurrent-session race: a snapshot
    // being tracked writes its objects BEFORE its manifest (manifest
    // writes are non-atomic `fs::write`), so another session's sweep
    // running between the two would otherwise see fresh-but-unreferenced
    // objects and delete them out from under the in-progress snapshot.
    // Fresh objects survive one sweep; if the snapshot completes they
    // become referenced, and if the session died they are cleaned after
    // the window.
    if let Ok(entries) = std::fs::read_dir(&objects_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if !referenced.contains(&name)
                && entry_older_than(&path, cutoff)
                && std::fs::remove_file(&path).is_ok()
            {
                report.removed_objects.push(name);
            }
        }
    }
}

/// Remove files (not dirs) in `dir` older than `cutoff` (permit claims,
/// snapshot objects etc. are flat files).
fn prune_old_files(removed: &mut Vec<String>, dir: &Path, cutoff: SystemTime) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry_older_than(&path, cutoff) && std::fs::remove_file(&path).is_ok() {
            removed.push(name);
        }
    }
}

/// Sweep blackboard plan-epoch snapshots by age, EXCEPT the highest-numbered
/// one (v1.15⑧, 2026-08-15). The newest archive is the restore entry —
/// `epoch::latest_epoch_snapshot` loads the max file — so it must survive
/// even when older than the retention cutoff (a long-lived live board).
/// Epoch numbers are timestamp-stamped monotonic (2026-08-15), so the
/// highest number is always the most recent identity.
fn prune_blackboard_epochs(removed: &mut Vec<String>, dir: &Path, cutoff: SystemTime) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<(u64, PathBuf)> = Vec::new();
    let mut latest: Option<u64> = None;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Some(rest) = name
            .strip_prefix("epoch-")
            .and_then(|r| r.strip_suffix(".json"))
        else {
            continue;
        };
        let Ok(epoch) = rest.parse::<u64>() else {
            continue;
        };
        if latest.map_or(true, |m| epoch > m) {
            latest = Some(epoch);
        }
        files.push((epoch, path));
    }
    for (epoch, path) in files {
        if latest == Some(epoch) {
            continue;
        }
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        if entry_older_than(&path, cutoff) && std::fs::remove_file(&path).is_ok() {
            removed.push(name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn test_dir() -> PathBuf {
        let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "orz-host-retention-test-{}-{}",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Backdate a file's mtime by `days` days (deterministic age control).
    fn backdate(path: &Path, days: u64) {
        let past = filetime::FileTime::from_system_time(
            std::time::SystemTime::now() - std::time::Duration::from_secs(days * 24 * 60 * 60),
        );
        filetime::set_file_mtime(path, past).unwrap();
    }

    /// Default-window cutoff from the real clock (now - 7 days).
    fn default_cutoff() -> SystemTime {
        retention_cutoff(std::time::SystemTime::now(), DEFAULT_RETENTION_DAYS)
    }

    /// local_browser (2026-08-10): orphaned headless-browser profiles are
    /// swept by age; a fresh profile is kept.
    #[test]
    fn sweep_removes_old_browser_profiles_keeps_fresh() {
        let base = test_dir();
        let gsa = base.join(".gsa");
        std::fs::create_dir_all(gsa.join("chrome-profile-OLDRUN")).unwrap();
        std::fs::create_dir_all(gsa.join("chrome-profile-FRESHRUN")).unwrap();
        std::fs::write(
            gsa.join("chrome-profile-OLDRUN").join("DevToolsActivePort"),
            "1",
        )
        .unwrap();
        std::fs::write(
            gsa.join("chrome-profile-FRESHRUN")
                .join("DevToolsActivePort"),
            "2",
        )
        .unwrap();
        // entry_older_than judges a profile dir by the DIR mtime (no
        // events.jsonl) — backdate the dir itself.
        backdate(&gsa.join("chrome-profile-OLDRUN"), 10);
        // A non-profile dir never matches the prefix.
        std::fs::create_dir_all(gsa.join("snapshots")).unwrap();

        let report = prune_old_records(&gsa, default_cutoff(), None);
        assert_eq!(
            report.removed_browser_profiles,
            vec!["chrome-profile-OLDRUN"]
        );
        assert!(!gsa.join("chrome-profile-OLDRUN").exists());
        assert!(gsa.join("chrome-profile-FRESHRUN").exists());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn retention_window_defaults_and_overrides() {
        assert_eq!(retention_days(), DEFAULT_RETENTION_DAYS);
        // Cutoff math: now - N days — a LONGER window cuts EARLIER.
        let cutoff = retention_cutoff(std::time::SystemTime::now(), 7);
        let cutoff10 = retention_cutoff(std::time::SystemTime::now(), 10);
        assert!(cutoff10 < cutoff);
    }

    #[test]
    fn cutoff_never_panics_on_pathological_days() {
        // Review P2-1 (2026-08-08): a pathological env value must not panic
        // the sweep — clamp to the epoch instead (everything is old).
        let cutoff = retention_cutoff(std::time::SystemTime::now(), u64::MAX);
        assert!(cutoff <= std::time::UNIX_EPOCH + std::time::Duration::from_secs(1));
    }

    #[test]
    fn sweep_removes_old_run_dirs_but_keeps_current_and_fresh() {
        let base = test_dir();
        let gsa = base.join(".gsa");
        // Old run dirs — events.jsonl backdated beyond the window.
        std::fs::create_dir_all(gsa.join("runs").join("RUN-OLD1")).unwrap();
        std::fs::create_dir_all(gsa.join("runs").join("RUN-OLD2")).unwrap();
        for name in ["RUN-OLD1", "RUN-OLD2"] {
            let events = gsa.join("runs").join(name).join("events.jsonl");
            std::fs::write(&events, "{}").unwrap();
            backdate(&events, 10);
        }
        // The active run — protected by NAME even when genuinely old.
        std::fs::create_dir_all(gsa.join("runs").join("RUN-ACTIVE")).unwrap();
        let active_events = gsa.join("runs").join("RUN-ACTIVE").join("events.jsonl");
        std::fs::write(&active_events, "{}").unwrap();
        backdate(&active_events, 10);
        // Unrelated directory inside runs/ — never swept.
        std::fs::create_dir_all(gsa.join("runs").join("OTHER")).unwrap();

        let report = prune_old_records(&gsa, default_cutoff(), Some("RUN-ACTIVE"));

        assert_eq!(report.removed_run_dirs, vec!["RUN-OLD1", "RUN-OLD2"]);
        assert!(
            gsa.join("runs").join("RUN-ACTIVE").exists(),
            "active run kept"
        );
        assert!(gsa.join("runs").join("OTHER").exists(), "unrelated kept");
        assert!(!gsa.join("runs").join("RUN-OLD1").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn sweep_keeps_everything_within_the_window() {
        let base = test_dir();
        let gsa = base.join(".gsa");
        std::fs::create_dir_all(gsa.join("runs").join("RUN-FRESH")).unwrap();
        std::fs::write(
            gsa.join("runs").join("RUN-FRESH").join("events.jsonl"),
            "{}",
        )
        .unwrap();

        // Default cutoff — the fresh dir (mtime now) stays.
        let report = prune_old_records(&gsa, default_cutoff(), None);
        assert!(report.removed_run_dirs.is_empty());
        assert!(gsa.join("runs").join("RUN-FRESH").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn snapshot_gc_removes_old_unreferenced_objects_only() {
        let base = test_dir();
        let gsa = base.join(".gsa");
        let manifests = gsa.join("snapshots").join("manifests");
        let objects = gsa.join("snapshots").join("objects");
        std::fs::create_dir_all(&manifests).unwrap();
        std::fs::create_dir_all(&objects).unwrap();
        // One fresh manifest referencing object A; one old manifest (swept
        // by age) referencing object B; object C unreferenced entirely
        // (old); object D unreferenced but FRESH (an in-progress snapshot —
        // the review D2-1 race — must survive one sweep).
        let entry = |path: &str, hash: &str| {
            serde_json::json!([{
                "path": path,
                "object_sha256": hash,
                "size": 1,
                "mtime_nanos": 0,
            }])
            .to_string()
        };
        std::fs::write(manifests.join("fresh.json"), entry("a.txt", "aaaa")).unwrap();
        let old_manifest = manifests.join("old.json");
        std::fs::write(&old_manifest, entry("b.txt", "bbbb")).unwrap();
        backdate(&old_manifest, 10);
        for obj in ["aaaa", "bbbb", "cccc"] {
            let path = objects.join(obj);
            std::fs::write(&path, "x").unwrap();
            backdate(&path, 10);
        }
        // Object D: fresh — must NOT be GC'd even though unreferenced.
        std::fs::write(objects.join("dddd"), "x").unwrap();

        let report = prune_old_records(&gsa, default_cutoff(), None);

        // Old manifest swept; object B died with it; object C (old +
        // unreferenced) GC'd; object A survives (referenced); object D
        // (fresh + unreferenced — in-progress snapshot) survives the age
        // gate.
        assert_eq!(report.removed_manifests, vec!["old.json"]);
        assert!(manifests.join("fresh.json").exists());
        assert!(!manifests.join("old.json").exists());
        assert!(objects.join("aaaa").exists(), "referenced object kept");
        assert!(!objects.join("bbbb").exists(), "orphaned old object GC'd");
        assert!(
            !objects.join("cccc").exists(),
            "old unreferenced object GC'd"
        );
        assert!(
            objects.join("dddd").exists(),
            "fresh unreferenced object kept (D2-1)"
        );
        // read_dir order is unspecified — compare as sets.
        let mut removed = report.removed_objects.clone();
        removed.sort();
        assert_eq!(removed, vec!["bbbb".to_string(), "cccc".to_string()]);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn corrupt_kept_manifest_skips_object_gc() {
        // Review P3-1 (2026-08-08): a KEPT manifest that cannot be parsed
        // could be the only referencer of its objects — the object GC is
        // skipped entirely (fail-safe), while age-based manifest pruning
        // still runs.
        let base = test_dir();
        let gsa = base.join(".gsa");
        let manifests = gsa.join("snapshots").join("manifests");
        let objects = gsa.join("snapshots").join("objects");
        std::fs::create_dir_all(&manifests).unwrap();
        std::fs::create_dir_all(&objects).unwrap();
        // An old manifest (swept by age) + a kept but CORRUPT manifest.
        let old_manifest = manifests.join("old.json");
        std::fs::write(&old_manifest, "[]").unwrap();
        backdate(&old_manifest, 10);
        std::fs::write(manifests.join("corrupt.json"), "{not json").unwrap();
        std::fs::write(objects.join("orphan"), "x").unwrap();

        let report = prune_old_records(&gsa, default_cutoff(), None);

        assert_eq!(report.removed_manifests, vec!["old.json"]);
        assert!(manifests.join("corrupt.json").exists(), "corrupt kept");
        assert!(
            objects.join("orphan").exists(),
            "object GC skipped — fail-safe"
        );
        assert!(report.removed_objects.is_empty());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn sweep_removes_old_permit_claims_but_never_keystore() {
        let base = test_dir();
        let gsa = base.join(".gsa");
        std::fs::create_dir_all(gsa.join("one_shot_permit")).unwrap();
        std::fs::create_dir_all(gsa.join("keystore")).unwrap();
        let claim = gsa.join("one_shot_permit").join("claim-1");
        std::fs::write(&claim, "used").unwrap();
        backdate(&claim, 10);
        std::fs::write(gsa.join("keystore").join("installation-key.dpapi"), "key").unwrap();

        let report = prune_old_records(&gsa, default_cutoff(), None);

        assert_eq!(report.removed_permit_files, vec!["claim-1"]);
        assert!(gsa.join("keystore").join("installation-key.dpapi").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn sweep_never_fails_on_missing_tree() {
        let base = test_dir();
        // No .gsa at all — no-op, no panic.
        let report = prune_old_records(&base, default_cutoff(), None);
        assert!(report.removed_run_dirs.is_empty());
        let _ = std::fs::remove_dir_all(&base);
    }

    /// GAP-CONVERSATION-RESTORE (2026-08-10): old conversation sidecars are
    /// swept by age; fresh ones (a live session rewrites its sidecar on
    /// every successful prompt) survive.
    #[test]
    fn sweep_removes_old_conversation_sidecars_keeps_fresh() {
        let base = test_dir();
        let gsa = base.join(".gsa");
        let conv = gsa.join("conversations");
        std::fs::create_dir_all(&conv).unwrap();
        let old = conv.join("sess-old.json");
        std::fs::write(&old, "{}").unwrap();
        backdate(&old, 10);
        std::fs::write(conv.join("sess-fresh.json"), "{}").unwrap();

        let report = prune_old_records(&gsa, default_cutoff(), None);

        assert_eq!(report.removed_conversation_sidecars, vec!["sess-old.json"]);
        assert!(!old.exists());
        assert!(conv.join("sess-fresh.json").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    /// v1.15 (2026-08-14): old blackboard plan-epoch snapshots are swept by
    /// age; fresh ones survive (the current epoch is still live).
    #[test]
    fn sweep_removes_old_blackboard_epoch_archives_keeps_fresh() {
        let base = test_dir();
        let gsa = base.join(".gsa");
        let bb = gsa.join("blackboard");
        std::fs::create_dir_all(&bb).unwrap();
        let old = bb.join("epoch-1.json");
        std::fs::write(&old, "{}").unwrap();
        backdate(&old, 10);
        std::fs::write(bb.join("epoch-2.json"), "{}").unwrap();

        let report = prune_old_records(&gsa, default_cutoff(), None);

        assert_eq!(report.removed_blackboard_epoch_archives, vec!["epoch-1.json"]);
        assert!(!old.exists());
        assert!(bb.join("epoch-2.json").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    /// v1.15⑧ (2026-08-15): the highest-numbered blackboard epoch archive
    /// survives even when older than the cutoff — it is the restore entry
    /// for a long-lived live board.
    #[test]
    fn sweep_keeps_latest_blackboard_epoch_even_when_old() {
        let base = test_dir();
        let gsa = base.join(".gsa");
        let bb = gsa.join("blackboard");
        std::fs::create_dir_all(&bb).unwrap();
        let old1 = bb.join("epoch-1.json");
        let old2 = bb.join("epoch-2.json");
        std::fs::write(&old1, "{}").unwrap();
        std::fs::write(&old2, "{}").unwrap();
        backdate(&old1, 10);
        backdate(&old2, 10);

        let report = prune_old_records(&gsa, default_cutoff(), None);

        assert_eq!(
            report.removed_blackboard_epoch_archives,
            vec!["epoch-1.json"]
        );
        assert!(!old1.exists());
        assert!(old2.exists(), "latest epoch archive is the restore entry");

        let _ = std::fs::remove_dir_all(&base);
    }

    /// GAP-PROJECT-DOC-INDEX-CACHE (2026-08-11): an old doc-index cache file
    /// is swept by age; a fresh one (a working workspace rewrites it on
    /// changed queries) survives.
    #[test]
    fn sweep_removes_old_project_doc_cache_keeps_fresh() {
        let base = test_dir();
        let gsa = base.join(".gsa");
        let caches = gsa.join("project-doc-index");
        std::fs::create_dir_all(&caches).unwrap();
        let old = caches.join("cache.json");
        std::fs::write(&old, "{}").unwrap();
        backdate(&old, 10);
        std::fs::write(caches.join("fresh.json"), "{}").unwrap();

        let report = prune_old_records(&gsa, default_cutoff(), None);

        assert_eq!(report.removed_project_doc_caches, vec!["cache.json"]);
        assert!(!old.exists());
        assert!(caches.join("fresh.json").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    /// P0-D S3 (2026-08-14): old compaction summary archives are swept by
    /// age; fresh ones (a live session's recent summaries) survive.
    #[test]
    fn sweep_removes_old_compaction_archives_keeps_fresh() {
        let base = test_dir();
        let gsa = base.join(".gsa");
        let archives = gsa.join("compaction");
        std::fs::create_dir_all(&archives).unwrap();
        let old = archives.join("compaction-RUN-OLD-0001.md");
        std::fs::write(&old, "# 摘要").unwrap();
        backdate(&old, 10);
        std::fs::write(
            archives.join("compaction-RUN-FRESH-0002.md"),
            "# 摘要",
        )
        .unwrap();

        let report = prune_old_records(&gsa, default_cutoff(), None);

        assert_eq!(
            report.removed_compaction_archives,
            vec!["compaction-RUN-OLD-0001.md"]
        );
        assert!(!old.exists());
        assert!(archives.join("compaction-RUN-FRESH-0002.md").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    /// PDF evidence (2026-08-11): old content-addressed evidence docs are
    /// swept by age (rebuildable from the source URL); fresh ones survive;
    /// the shard dirs are container entries, never removed themselves.
    #[test]
    fn sweep_removes_old_pdf_evidence_keeps_fresh_and_shards() {
        let base = test_dir();
        let gsa = base.join(".gsa");
        let evidence = gsa.join("pdf-evidence");
        let shard = evidence.join("ab");
        std::fs::create_dir_all(&shard).unwrap();
        let old = shard.join("ab".repeat(32));
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join("metadata.json"), "{}").unwrap();
        backdate(&old, 10);
        let fresh = shard.join("cd".repeat(32));
        std::fs::create_dir_all(&fresh).unwrap();
        std::fs::write(fresh.join("metadata.json"), "{}").unwrap();

        let report = prune_old_records(&gsa, default_cutoff(), None);

        assert_eq!(report.removed_pdf_evidence_dirs, vec!["ab".repeat(32)]);
        assert!(!old.exists());
        assert!(fresh.exists());
        assert!(shard.exists(), "shard dirs are containers — never swept");

        let _ = std::fs::remove_dir_all(&base);
    }

    /// PDF evidence (2026-08-11): crash-leftover download staging dirs are
    /// swept by age; a live one survives.
    #[test]
    fn sweep_removes_old_pdf_download_staging() {
        let base = test_dir();
        let gsa = base.join(".gsa");
        std::fs::create_dir_all(&gsa).unwrap();
        let old = gsa.join("pdf-downloads-OLD1234");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join("paper.pdf"), b"%PDF-1.4\n").unwrap();
        backdate(&old, 10);
        let fresh = gsa.join("pdf-downloads-FRESH5678");
        std::fs::create_dir_all(&fresh).unwrap();
        std::fs::write(fresh.join("paper.pdf"), b"%PDF-1.4\n").unwrap();

        let report = prune_old_records(&gsa, default_cutoff(), None);

        assert_eq!(
            report.removed_pdf_download_dirs,
            vec!["pdf-downloads-OLD1234"]
        );
        assert!(!old.exists());
        assert!(fresh.exists());

        let _ = std::fs::remove_dir_all(&base);
    }
}
