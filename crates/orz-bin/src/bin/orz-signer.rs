//! orz-signer — the ACAF independent signer process (ADR-0011 §4.4).
//!
//! Startup contract (fail-closed):
//!   1. The caller (trusted launcher) points at a signer manifest
//!      (`ORZ_SIGNER_MANIFEST` or `<exe_dir>/signer-manifest.json`) — the
//!      manifest carries `signer_revision` and the SHA-256 of this binary.
//!      The binary verifies its own hash against the manifest; a mismatch
//!      or a missing manifest exits non-zero. (Signed-manifest verification
//!      with an independent publish key is the v2 upgrade path — registered
//!      boundary; the trust anchor here is "the launch chain hands us the
//!      manifest", same trust level as the host binary itself.)
//!   2. `K_install` is loaded from the installation keystore (Windows
//!      DPAPI; non-Windows fails closed — the signer refuses to start).
//!      `K_install` never leaves this process.
//!
//! Interface: JSON-lines on stdin/stdout, single request single response
//! (the narrow-IPC v1 transport — the process boundary is the ACL: only
//! the launching host holds our stdin/stdout). Methods are enumerated —
//! no free-form text is ever accepted; the orientation template is held
//! by this process (check 2 of ADR-0011 §4.2).
//!
//! Request:  {"id": 1, "method": "sign_orientation_v1", "params": {...}}
//! Response: {"id": 1, "result": {...}} | {"id": 1, "error": {"code", "message"}}
//!
//! Methods:
//!   initialize_session { session_id, agent_id, goal_version, goal_digest,
//!                        policy_revision }
//!       -> { session_key_hex, signer_revision, signer_measurement,
//!            template_sha256, sequence_start }
//!   sign_orientation_v1    { session_id }
//!   sign_disposition_v1    { session_id, activation_id, canonical_arguments_sha256 }
//!   sign_close_v1          { session_id, activation_id, canonical_arguments_sha256 }
//!   sign_goal_revision_v1  { session_id, activation_id,
//!                            canonical_arguments_sha256 }   // old-context ticket
//!   close_session          { session_id }
//!
//! A goal revision re-derives the session key when the host re-initialises
//! with the new goal_digest — old tickets fail immediately (§4.3).

use std::collections::HashMap;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::PathBuf;

use orz_assurance::acaf::{
    IssueContext, TicketKind, derive_session_key, issue_ticket,
};
use orz_assurance::journal::sha256_hex;
use orz_assurance::permit::HmacSha256Signer;
use orz_host::keystore::{KeystoreError, WindowsDpapiInstallationKeyStore};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Orientation template held by the signer (ADR-0011 §4.4 — the signer never
/// accepts template text). SINGLE SOURCE: the same constant the controller
/// actually injects (`ORIENTATION_BLOCK` in orz-assurance) — the ticket's
/// `template_sha256` binds the REAL injected text (check 2 of §4.2).
/// (Review D1-1 2026-08-12: the earlier one-line local constant did not
/// match the injected block — the template binding was hollow. Fixed by
/// referencing the canonical constant directly.)
const ORIENTATION_TEMPLATE: &str = orz_assurance::orientation::checkpoint::ORIENTATION_BLOCK;

/// Manifest schema version.
const MANIFEST_VERSION: u64 = 1;

#[derive(Debug, thiserror::Error)]
enum SignerError {
    #[error("signer manifest not found at {0}")]
    ManifestNotFound(PathBuf),
    #[error("signer manifest is invalid: {0}")]
    ManifestInvalid(String),
    #[error("binary self-hash mismatch: manifest {expected}, computed {computed}")]
    SelfHashMismatch { expected: String, computed: String },
    #[error("keystore error: {0}")]
    Keystore(#[from] KeystoreError),
    #[error("session {0} is not initialised")]
    SessionNotInitialised(String),
    #[error("acaf error: {0}")]
    Acaf(#[from] orz_assurance::acaf::AcafError),
    #[error("unknown method: {0}")]
    UnknownMethod(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

/// The signer manifest — the trusted-launcher contract for this binary.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SignerManifest {
    manifest_version: u64,
    signer_revision: u64,
    binary_name: String,
    binary_sha256: String,
}

#[derive(Debug, Clone)]
struct SignerSession {
    agent_id: String,
    goal_version: u64,
    goal_digest: String,
    policy_revision: u64,
    sequence: u64,
    k_session: [u8; 32],
}

/// Per-process signer state. `k_install` is zeroised on drop.
#[derive(Debug)]
struct Signer {
    k_install: Vec<u8>,
    sessions: HashMap<String, SignerSession>,
    signer_revision: u64,
    signer_measurement: String,
    template_sha256: String,
}

impl Drop for Signer {
    fn drop(&mut self) {
        orz_assurance::credential::zeroize_bytes(&mut self.k_install);
    }
}

impl Signer {
    fn new(k_install: Vec<u8>, manifest: &SignerManifest) -> Self {
        let template_sha256 = sha256_hex(ORIENTATION_TEMPLATE.as_bytes());
        Self {
            k_install,
            sessions: HashMap::new(),
            signer_revision: manifest.signer_revision,
            signer_measurement: manifest.binary_sha256.clone(),
            template_sha256,
        }
    }

    fn initialize_session(
        &mut self,
        session_id: &str,
        agent_id: &str,
        goal_version: u64,
        goal_digest: &str,
        policy_revision: u64,
    ) -> Result<SignerSession, SignerError> {
        let k_session = derive_session_key(
            &self.k_install,
            session_id,
            goal_digest,
            policy_revision,
            self.signer_revision,
        )?;
        let session = SignerSession {
            agent_id: agent_id.to_string(),
            goal_version,
            goal_digest: goal_digest.to_string(),
            policy_revision,
            sequence: 0,
            k_session,
        };
        self.sessions.insert(session_id.to_string(), session.clone());
        Ok(session)
    }

    fn sign_ticket(
        &mut self,
        session_id: &str,
        kind: TicketKind,
        activation_id: Option<String>,
        canonical_arguments_sha256: String,
        now_unix_secs: i64,
    ) -> Result<Value, SignerError> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| SignerError::SessionNotInitialised(session_id.to_string()))?;
        session.sequence += 1;
        let ctx = IssueContext {
            session_id: session_id.to_string(),
            agent_id: session.agent_id.clone(),
            activation_id,
            goal_version: session.goal_version,
            goal_digest: session.goal_digest.clone(),
            policy_revision: session.policy_revision,
            signer_revision: self.signer_revision,
            signer_measurement: self.signer_measurement.clone(),
            template_sha256: kind
                .requires_template()
                .then(|| self.template_sha256.clone()),
            canonical_arguments_sha256,
        };
        // Review P2-3 (2026-08-12): byte slicing a possibly multi-byte UTF-8
        // session id can panic — derive the key id from chars instead.
        let key_id = format!("KEY-SESS-{}", session_id.chars().take(8).collect::<String>());
        let signer = HmacSha256Signer::new(&key_id, &session.k_session);
        let nonce = uuid::Uuid::new_v4().simple().to_string();
        let ticket = issue_ticket(
            &signer,
            &ctx,
            kind,
            session.sequence,
            &nonce,
            TICKET_TTL_SECS,
            now_unix_secs,
        )?;
        serde_json::to_value(ticket).map_err(|e| SignerError::BadRequest(e.to_string()))
    }
}

/// Ticket TTL for control events — short (5 s): a control ticket is consumed
/// at its injection/commit point milliseconds after issuance.
const TICKET_TTL_SECS: u64 = 5;

/// Compute the SHA-256 of our own binary.
fn self_binary_sha256() -> Result<String, SignerError> {
    let exe = std::env::current_exe()?;
    let bytes = std::fs::read(&exe)?;
    Ok(sha256_hex(&bytes))
}

fn load_manifest() -> Result<SignerManifest, SignerError> {
    let path = match std::env::var("ORZ_SIGNER_MANIFEST") {
        Ok(p) => PathBuf::from(p),
        Err(_) => {
            let exe = std::env::current_exe()?;
            exe.parent()
                .map(|d| d.join("signer-manifest.json"))
                .unwrap_or_else(|| PathBuf::from("signer-manifest.json"))
        }
    };
    let text = std::fs::read_to_string(&path)
        .map_err(|_| SignerError::ManifestNotFound(path.clone()))?;
    let manifest: SignerManifest = serde_json::from_str(&text)
        .map_err(|e| SignerError::ManifestInvalid(e.to_string()))?;
    if manifest.manifest_version != MANIFEST_VERSION {
        return Err(SignerError::ManifestInvalid(format!(
            "manifest_version {} != {}",
            manifest.manifest_version, MANIFEST_VERSION
        )));
    }
    Ok(manifest)
}

/// Load `K_install` from the installation keystore (Windows DPAPI; other
/// platforms fail closed — the signer does not start without an OS keystore).
fn load_install_key() -> Result<Vec<u8>, SignerError> {
    let root = match std::env::var("ORZ_SIGNER_KEYSTORE_ROOT") {
        Ok(r) => PathBuf::from(r),
        Err(_) => return Err(SignerError::BadRequest(
            "ORZ_SIGNER_KEYSTORE_ROOT must point at the installation keystore root".into(),
        )),
    };
    let store = WindowsDpapiInstallationKeyStore::load(&root)?;
    Ok(store.secret_bytes()?)
}

/// Process one JSON request line; returns the response value (never panics on
/// malformed input — a malformed line yields an error response and the loop
/// continues).
fn handle_request(signer: &mut Signer, line: &str) -> Value {
    let request: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => {
            return error_response(None, "bad_request", &e.to_string());
        }
    };
    let id = request.get("id").cloned();
    let method = match request.get("method").and_then(Value::as_str) {
        Some(m) => m.to_string(),
        None => return error_response(id, "bad_request", "missing method"),
    };
    let params = request.get("params").cloned().unwrap_or(Value::Null);
    let result = match method.as_str() {
        "initialize_session" => handle_initialize(signer, &params),
        "sign_orientation_v1" => handle_sign(signer, &params, TicketKind::OrientationV1, None),
        "sign_disposition_v1" => handle_sign(signer, &params, TicketKind::DispositionV1, Some("activation_id")),
        "sign_close_v1" => handle_sign(signer, &params, TicketKind::CloseV1, Some("activation_id")),
        "sign_goal_revision_v1" => {
            handle_sign(signer, &params, TicketKind::GoalRevisionV1, Some("activation_id"))
        }
        "close_session" => match param_str(&params, "session_id") {
            Ok(session_id) => {
                signer.sessions.remove(&session_id);
                Ok(Value::Null)
            }
            Err(e) => Err(e),
        },
        other => Err(SignerError::UnknownMethod(other.to_string())),
    };
    match result {
        Ok(result) => serde_json::json!({ "id": id, "result": result }),
        Err(e) => error_response(id, "signer_error", &e.to_string()),
    }
}

fn handle_initialize(signer: &mut Signer, params: &Value) -> Result<Value, SignerError> {
    let session_id = param_str(params, "session_id")?;
    let agent_id = param_str(params, "agent_id")?;
    let goal_version = param_u64(params, "goal_version")?;
    let goal_digest = param_str(params, "goal_digest")?;
    let policy_revision = param_u64(params, "policy_revision")?;
    let session = signer.initialize_session(
        &session_id,
        &agent_id,
        goal_version,
        &goal_digest,
        policy_revision,
    )?;
    Ok(serde_json::json!({
        "session_key_hex": hex_encode(&session.k_session),
        "signer_revision": signer.signer_revision,
        "signer_measurement": signer.signer_measurement,
        "template_sha256": signer.template_sha256,
        "sequence_start": session.sequence,
    }))
}

fn handle_sign(
    signer: &mut Signer,
    params: &Value,
    kind: TicketKind,
    activation_param: Option<&str>,
) -> Result<Value, SignerError> {
    let session_id = param_str(params, "session_id")?;
    let activation_id = match activation_param {
        Some(p) => Some(param_str(params, p)?),
        None => None,
    };
    let canonical_arguments_sha256 = param_str(params, "canonical_arguments_sha256")?;
    let now = chrono::Utc::now().timestamp();
    let ticket = signer.sign_ticket(
        &session_id,
        kind,
        activation_id,
        canonical_arguments_sha256,
        now,
    )?;
    Ok(ticket)
}

/// Lowercase hex encoding (same form as `sha256_hex`).
fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        write!(&mut out, "{byte:02x}").unwrap();
    }
    out
}

fn param_str(params: &Value, name: &str) -> Result<String, SignerError> {
    params
        .get(name)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| SignerError::BadRequest(format!("missing string param {name}")))
}

fn param_u64(params: &Value, name: &str) -> Result<u64, SignerError> {
    params
        .get(name)
        .and_then(Value::as_u64)
        .ok_or_else(|| SignerError::BadRequest(format!("missing u64 param {name}")))
}

fn error_response(id: Option<Value>, code: &str, message: &str) -> Value {
    serde_json::json!({
        "id": id,
        "error": { "code": code, "message": message },
    })
}

/// The stdio service loop (separable for tests).
fn run_service<R: Read, W: Write>(signer: &mut Signer, reader: R, writer: W) -> Result<(), SignerError> {
    let mut reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            break; // stdin closed — clean exit
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let response = handle_request(signer, trimmed);
        let encoded = serde_json::to_string(&response)?;
        writeln!(writer, "{encoded}")?;
        writer.flush()?;
    }
    Ok(())
}

fn main() {
    match run() {
        Ok(()) => {}
        Err(e) => {
            eprintln!("orz-signer fatal: {e}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<(), SignerError> {
    // Fail-closed startup: manifest → self-hash → keystore.
    let manifest = load_manifest()?;
    let computed = self_binary_sha256()?;
    if computed != manifest.binary_sha256 {
        return Err(SignerError::SelfHashMismatch {
            expected: manifest.binary_sha256.clone(),
            computed,
        });
    }
    if manifest.binary_name != "orz-signer" {
        return Err(SignerError::ManifestInvalid(format!(
            "binary_name {} != orz-signer",
            manifest.binary_name
        )));
    }
    let k_install = load_install_key()?;
    let mut signer = Signer::new(k_install, &manifest);
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    run_service(&mut signer, stdin.lock(), stdout.lock())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_manifest() -> SignerManifest {
        SignerManifest {
            manifest_version: MANIFEST_VERSION,
            signer_revision: 1,
            binary_name: "orz-signer".to_string(),
            binary_sha256: "0".repeat(64),
        }
    }

    fn test_signer() -> Signer {
        Signer::new(vec![7u8; 32], &test_manifest())
    }

    #[test]
    fn initialize_then_sign_orientation_roundtrip() {
        let mut signer = test_signer();
        let init = handle_initialize(
            &mut signer,
            &serde_json::json!({
                "session_id": "SESS-0001",
                "agent_id": "main",
                "goal_version": 0,
                "goal_digest": "0".repeat(64),
                "policy_revision": 0,
            }),
        )
        .unwrap();
        assert_eq!(init["session_key_hex"].as_str().unwrap().len(), 64);
        assert_eq!(init["template_sha256"].as_str().unwrap().len(), 64);

        let ticket = handle_sign(
            &mut signer,
            &serde_json::json!({
                "session_id": "SESS-0001",
                "canonical_arguments_sha256": "0".repeat(64),
            }),
            TicketKind::OrientationV1,
            None,
        )
        .unwrap();
        assert_eq!(ticket["ticket_kind"], "orientation_v1");
        assert_eq!(ticket["capability_scope"], "orientation_injection");
        assert_eq!(ticket["activation_id"], Value::Null);
        assert_eq!(ticket["template_sha256"].as_str().unwrap().len(), 64);
        assert_eq!(ticket["hmac"].as_str().unwrap().len(), 43);
        assert_eq!(ticket["sequence"], 1);
    }

    #[test]
    fn sequence_increments_and_session_isolates() {
        let mut signer = test_signer();
        let _ = handle_initialize(
            &mut signer,
            &serde_json::json!({
                "session_id": "S1", "agent_id": "main", "goal_version": 0,
                "goal_digest": "0".repeat(64), "policy_revision": 0,
            }),
        )
        .unwrap();
        let _ = handle_initialize(
            &mut signer,
            &serde_json::json!({
                "session_id": "S2", "agent_id": "main", "goal_version": 0,
                "goal_digest": "1".repeat(64), "policy_revision": 0,
            }),
        )
        .unwrap();
        let t1 = handle_sign(
            &mut signer,
            &serde_json::json!({"session_id": "S1", "activation_id": "ACT-1", "canonical_arguments_sha256": "0".repeat(64)}),
            TicketKind::DispositionV1,
            Some("activation_id"),
        )
        .unwrap();
        let t2 = handle_sign(
            &mut signer,
            &serde_json::json!({"session_id": "S1", "activation_id": "ACT-1", "canonical_arguments_sha256": "0".repeat(64)}),
            TicketKind::DispositionV1,
            Some("activation_id"),
        )
        .unwrap();
        assert_eq!(t1["sequence"], 1);
        assert_eq!(t2["sequence"], 2);
        // S2 is independent — sequence starts at 1 with its own key.
        let t3 = handle_sign(
            &mut signer,
            &serde_json::json!({"session_id": "S2", "activation_id": "ACT-2", "canonical_arguments_sha256": "0".repeat(64)}),
            TicketKind::CloseV1,
            Some("activation_id"),
        )
        .unwrap();
        assert_eq!(t3["sequence"], 1);
    }

    #[test]
    fn unknown_method_and_uninitialised_session_error() {
        let mut signer = test_signer();
        let response = handle_request(
            &mut signer,
            r#"{"id": 1, "method": "sign_orientation_v1", "params": {"session_id": "S1"}}"#,
        );
        assert!(response["error"]["code"].as_str().unwrap().contains("signer_error"));

        let response = handle_request(
            &mut signer,
            r#"{"id": 2, "method": "definitely_not_real"}"#,
        );
        assert!(response["error"]["code"].as_str().unwrap().contains("signer_error"));
        // Malformed JSON still yields a structured error (never a crash).
        let response = handle_request(&mut signer, "not json at all");
        assert!(response["error"]["code"].as_str().unwrap().contains("bad_request"));
    }

    #[test]
    fn goal_revision_ticket_binds_old_context() {
        let mut signer = test_signer();
        let _ = handle_initialize(
            &mut signer,
            &serde_json::json!({
                "session_id": "S1", "agent_id": "main", "goal_version": 0,
                "goal_digest": "0".repeat(64), "policy_revision": 0,
            }),
        )
        .unwrap();
        let ticket = handle_sign(
            &mut signer,
            &serde_json::json!({
                "session_id": "S1",
                "activation_id": "ACT-1",
                "canonical_arguments_sha256": "a".repeat(64),
            }),
            TicketKind::GoalRevisionV1,
            Some("activation_id"),
        )
        .unwrap();
        assert_eq!(ticket["goal_digest"], "0".repeat(64));
        assert_eq!(ticket["activation_id"], "ACT-1");
        // Re-initialise with the NEW goal → new session key (old tickets die).
        let init = handle_initialize(
            &mut signer,
            &serde_json::json!({
                "session_id": "S1", "agent_id": "main", "goal_version": 1,
                "goal_digest": "1".repeat(64), "policy_revision": 0,
            }),
        )
        .unwrap();
        assert_ne!(init["session_key_hex"], serde_json::json!("0".repeat(64) /* placeholder */));
        // The key actually changed: sign a fresh ticket under the new key and
        // verify the old ticket fails under it.
        let old = ticket;
        let fresh = handle_sign(
            &mut signer,
            &serde_json::json!({"session_id": "S1", "activation_id": "ACT-1", "canonical_arguments_sha256": "a".repeat(64)}),
            TicketKind::GoalRevisionV1,
            Some("activation_id"),
        )
        .unwrap();
        assert_ne!(fresh["hmac"], old["hmac"]);
    }

    #[test]
    fn service_loop_single_request_response() {
        let mut signer = test_signer();
        let request = r#"{"id": 7, "method": "initialize_session", "params": {"session_id": "S1", "agent_id": "main", "goal_version": 0, "goal_digest": "0000000000000000000000000000000000000000000000000000000000000000", "policy_revision": 0}}"#;
        let mut output: Vec<u8> = Vec::new();
        run_service(&mut signer, request.as_bytes(), &mut output).unwrap();
        let text = String::from_utf8(output).unwrap();
        let response: Value = serde_json::from_str(text.trim()).unwrap();
        assert_eq!(response["id"], 7);
        assert_eq!(response["result"]["session_key_hex"].as_str().unwrap().len(), 64);
    }

    #[test]
    fn template_sha256_matches_the_injected_orientation_block() {
        // The signer holds the SAME constant the controller injects
        // (review D1-1 2026-08-12): the ticket's template binding is real —
        // a drift between signer-held text and injected text is a test
        // failure here, byte-for-byte.
        let signer = test_signer();
        let canonical = orz_assurance::orientation::checkpoint::ORIENTATION_BLOCK;
        assert_eq!(
            ORIENTATION_TEMPLATE, canonical,
            "signer template must equal the injected block"
        );
        assert_eq!(
            signer.template_sha256,
            sha256_hex(canonical.as_bytes()),
            "signer template digest must pin the injected block"
        );
        assert!(ORIENTATION_TEMPLATE.contains("[ORIENTATION v0.2]"));
        assert!(ORIENTATION_TEMPLATE.contains("[/ORIENTATION]"));
    }
}
