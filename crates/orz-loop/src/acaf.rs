//! ACAF host-side client (ADR-0011 §4.4/§4.5) — the agent process's bridge
//! to the independent `orz-signer` process.
//!
//! The client owns the signer child process (spawned with the signer
//! manifest + keystore root), speaks the narrow stdio JSON-lines protocol,
//! holds the derived `K_session` for the current session, runs the seven
//! verification checks (§4.2) and the in-process one-shot ledger (check 3),
//! and exposes the ticket lifecycle to the controller so the control-event
//! producers can journal `control_ticket_issued` / `_consumed` /
//! `_rejected`.
//!
//! Slice 1 runs in SHADOW mode (ADR-0011 §2.12): a failed issuance or
//! verification is journaled as `control_ticket_rejected` (signer
//! unreachable → `signer_unreachable`) but the control event itself is NOT
//! blocked. Fail-closed flips with Slice 2's action tickets. An unconfigured
//! client (`None` on the controller) means zero behaviour change — no
//! ticket events are journaled at all.

use std::path::PathBuf;
use std::process::Stdio;

use orz_assurance::acaf::{
    ControlTicket, RejectCode, TicketKind, TicketLedger, VerifyContext, verify_ticket,
};
use orz_assurance::permit::HmacSha256Signer;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};

/// Request timeout for one signer round-trip (the signer is a local process
/// doing HMAC work — 2 s is generous; a stall means the signer is wedged and
/// the request fails fast).
const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

/// Signer spawn configuration.
#[derive(Debug, Clone)]
pub struct AcafConfig {
    /// Path to the signer manifest (the launch chain's trust anchor).
    pub manifest_path: PathBuf,
    /// Installation keystore root (`ORZ_SIGNER_KEYSTORE_ROOT`).
    pub keystore_root: PathBuf,
    /// The signer binary (defaults to `orz-signer` on PATH).
    pub signer_binary: Option<PathBuf>,
}

#[derive(Debug, thiserror::Error)]
pub enum AcafClientError {
    #[error("signer spawn failed: {0}")]
    Spawn(String),
    #[error("signer request timed out")]
    Timeout,
    #[error("signer closed the channel: {0}")]
    Closed(String),
    #[error("signer error response: {code}: {message}")]
    Signer { code: String, message: String },
    #[error("signer protocol error: {0}")]
    Protocol(String),
    #[error("acaf error: {0}")]
    Acaf(#[from] orz_assurance::acaf::AcafError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("session not initialised")]
    SessionNotInitialised,
}

/// The verified ticket lifecycle outcome a control producer journals.
#[derive(Debug)]
pub enum TicketOutcome {
    /// Issued and passed all seven checks — journal `_consumed`.
    Consumed { ticket_id: String, kind: TicketKind },
    /// Issued but verification failed — journal `_rejected` with the first
    /// reject code (shadow mode: the control event still proceeds).
    Rejected {
        ticket_id: Option<String>,
        kind: TicketKind,
        code: RejectCode,
        detail: String,
    },
    /// The signer was unavailable — journal `_rejected(signer_unreachable)`.
    SignerUnreachable { kind: TicketKind, detail: String },
}

#[derive(Debug)]
struct ClientSession {
    session_id: String,
    agent_id: String,
    goal_version: u64,
    goal_digest: String,
    policy_revision: u64,
    k_session: HmacSha256Signer,
    template_sha256: String,
    /// Signer version + binary measurement downloaded at initialize_session
    /// (ADR-0011 §4.4) — recorded for audit/upgrade-wiring (Slice 2 uses them
    /// in the ticket-issued payload audit fields).
    #[allow(dead_code)]
    signer_revision: u64,
    #[allow(dead_code)]
    signer_measurement: String,
}

/// The host-side signer client. One client per agent process (the controller
/// holds `Option<Arc<Mutex<AcafClient>>>` — `None` = ACAF disabled).
///
/// Self-healing (review P1-1 2026-08-12): a failed request (timeout, channel
/// closed, protocol desync) kills the signer child and respawns it on the
/// next call — the JSON-lines stream can never stay misaligned across
/// requests.
pub struct AcafClient {
    cfg: AcafConfig,
    child: tokio::process::Child,
    stdin: BufWriter<tokio::process::ChildStdin>,
    stdout: BufReader<tokio::process::ChildStdout>,
    next_id: u64,
    ledger: TicketLedger,
    session: Option<ClientSession>,
}

/// Spawn a signer child and take its stdio (shared by `spawn` and `respawn`).
async fn spawn_child(
    cfg: &AcafConfig,
) -> Result<
    (
        tokio::process::Child,
        tokio::process::ChildStdin,
        tokio::process::ChildStdout,
    ),
    AcafClientError,
> {
    let binary = cfg
        .signer_binary
        .clone()
        .unwrap_or_else(|| PathBuf::from("orz-signer"));
    let mut command = tokio::process::Command::new(&binary);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .env("ORZ_SIGNER_MANIFEST", &cfg.manifest_path)
        .env("ORZ_SIGNER_KEYSTORE_ROOT", &cfg.keystore_root);
    let mut child = command
        .spawn()
        .map_err(|e| AcafClientError::Spawn(e.to_string()))?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| AcafClientError::Spawn("no stdin on signer child".into()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AcafClientError::Spawn("no stdout on signer child".into()))?;
    Ok((child, stdin, stdout))
}

impl AcafClient {
    /// Spawn the signer process (fail-closed startup is the signer's own
    /// job — a manifest/keystore/self-hash failure makes it exit non-zero,
    /// which surfaces here as a spawn error).
    pub async fn spawn(cfg: &AcafConfig) -> Result<Self, AcafClientError> {
        let (child, stdin, stdout) = spawn_child(cfg).await?;
        Ok(Self {
            cfg: cfg.clone(),
            child,
            stdin: BufWriter::new(stdin),
            stdout: BufReader::new(stdout),
            next_id: 1,
            ledger: TicketLedger::new(),
            session: None,
        })
    }

    /// Kill the current signer child and spawn a fresh one, re-initialising
    /// the cached session on the new process (review P1-1 2026-08-12). The
    /// session ledger is preserved — a respawn must not replay-reject the
    /// continuing session's tickets.
    async fn respawn(&mut self) -> Result<(), AcafClientError> {
        // Snapshot the session context BEFORE the child is killed (the
        // re-initialisation replays the deterministic HKDF on the new
        // process — same inputs → same K_session).
        let reinit = self.session.as_ref().map(|s| {
            (
                s.session_id.clone(),
                s.agent_id.clone(),
                s.goal_version,
                s.goal_digest.clone(),
                s.policy_revision,
            )
        });
        let _ = self.child.kill().await;
        let _ = self.child.wait().await;
        let (child, stdin, stdout) = spawn_child(&self.cfg).await?;
        self.child = child;
        self.stdin = BufWriter::new(stdin);
        self.stdout = BufReader::new(stdout);
        if let Some((session_id, agent_id, goal_version, goal_digest, policy_revision)) = reinit {
            // Direct protocol call (never self-healing — that would recurse).
            let result = self
                .call_raw(
                    "initialize_session",
                    serde_json::json!({
                        "session_id": session_id,
                        "agent_id": agent_id,
                        "goal_version": goal_version,
                        "goal_digest": goal_digest,
                        "policy_revision": policy_revision,
                    }),
                )
                .await?;
            self.apply_initialize_response(
                &session_id,
                &agent_id,
                goal_version,
                &goal_digest,
                policy_revision,
                &result,
            )?;
        }
        Ok(())
    }
}

impl AcafClient {
    /// Ensure the signer session is initialised with the current goal /
    /// policy context. Re-initialisation with a different goal_digest or
    /// policy_revision derives a new `K_session` — old tickets die (check 1
    /// fails against the new key).
    ///
    /// Review D2-1 (2026-08-12): the one-shot ledger's monotonicity is PER
    /// K_SESSION EPOCH, not per session — a re-derivation resets the
    /// signer-side sequence, so the ledger's high-water mark for the session
    /// is cleared on re-initialisation (otherwise every fresh ticket of the
    /// new epoch would be rejected as a sequence regression).
    pub async fn ensure_initialized(
        &mut self,
        session_id: &str,
        agent_id: &str,
        goal_version: u64,
        goal_digest: &str,
        policy_revision: u64,
    ) -> Result<(), AcafClientError> {
        if let Some(s) = &self.session
            && s.session_id == session_id
            && s.goal_version == goal_version
            && s.goal_digest == goal_digest
            && s.policy_revision == policy_revision
        {
            return Ok(());
        }
        // A context change re-derives K_session (new epoch) — the ledger's
        // per-session sequence resets with it.
        self.ledger.reset(session_id);
        let result = self
            .call(
                "initialize_session",
                serde_json::json!({
                    "session_id": session_id,
                    "agent_id": agent_id,
                    "goal_version": goal_version,
                    "goal_digest": goal_digest,
                    "policy_revision": policy_revision,
                }),
            )
            .await?;
        self.apply_initialize_response(
            session_id,
            agent_id,
            goal_version,
            goal_digest,
            policy_revision,
            &result,
        )
    }

    /// Decode an `initialize_session` response into the session cache
    /// (shared by `ensure_initialized` and `respawn`'s re-initialisation).
    fn apply_initialize_response(
        &mut self,
        session_id: &str,
        agent_id: &str,
        goal_version: u64,
        goal_digest: &str,
        policy_revision: u64,
        result: &Value,
    ) -> Result<(), AcafClientError> {
        let session_key_hex = result
            .get("session_key_hex")
            .and_then(Value::as_str)
            .ok_or_else(|| AcafClientError::Protocol("missing session_key_hex".into()))?;
        let k_bytes = hex_decode(session_key_hex).ok_or_else(|| {
            AcafClientError::Protocol("session_key_hex is not valid hex".into())
        })?;
        // Review P2-3 (2026-08-12): byte slicing a possibly multi-byte UTF-8
        // session id can panic — derive the key id from chars instead.
        let key_id = format!("KEY-SESS-{}", session_id.chars().take(8).collect::<String>());
        let session = ClientSession {
            session_id: session_id.to_string(),
            agent_id: agent_id.to_string(),
            goal_version,
            goal_digest: goal_digest.to_string(),
            policy_revision,
            k_session: HmacSha256Signer::new(&key_id, &k_bytes),
            template_sha256: result
                .get("template_sha256")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            signer_revision: result
                .get("signer_revision")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            signer_measurement: result
                .get("signer_measurement")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        };
        self.session = Some(session);
        Ok(())
    }

    /// The full Slice-1 lifecycle for one control event: sign → the caller
    /// journals `control_ticket_issued` with the returned ticket's binding
    /// fields → `verify_and_consume` → journal `_consumed` or `_rejected`.
    ///
    /// `canonical_arguments_sha256` is the digest of the parsed real object
    /// (check 5 — the controller re-derives it from the live arguments).
    /// `resolved_target_sha256` binds the parsed real target — action kinds
    /// only (Slice 2 first phase; control kinds pass None).
    pub async fn sign_ticket(
        &mut self,
        kind: TicketKind,
        activation_id: Option<String>,
        canonical_arguments_sha256: &str,
        resolved_target_sha256: Option<String>,
    ) -> Result<ControlTicket, AcafClientError> {
        let session = self
            .session
            .as_ref()
            .ok_or(AcafClientError::SessionNotInitialised)?;
        let mut params = serde_json::json!({
            "session_id": session.session_id,
            "canonical_arguments_sha256": canonical_arguments_sha256,
        });
        if let Some(activation) = &activation_id {
            params["activation_id"] = serde_json::json!(activation);
        }
        if let Some(target) = &resolved_target_sha256 {
            params["resolved_target_sha256"] = serde_json::json!(target);
        }
        let method = match kind {
            TicketKind::OrientationV1 => "sign_orientation_v1",
            TicketKind::DispositionV1 => "sign_disposition_v1",
            TicketKind::CloseV1 => "sign_close_v1",
            TicketKind::GoalRevisionV1 => "sign_goal_revision_v1",
            TicketKind::FileWriteV1 => "sign_file_write_v1",
            TicketKind::CredentialReadV1 => "sign_credential_read_v1",
        };
        let result = self.call(method, params).await?;
        serde_json::from_value(result).map_err(|e| AcafClientError::Protocol(e.to_string()))
    }

    /// Verify a freshly-issued ticket against the live context (checks 1-6)
    /// and consume it in the one-shot ledger (check 3). Returns the outcome
    /// for the journal. Under shadow mode the caller proceeds regardless.
    ///
    /// `live_activation_id` is the REAL activation at the consumption point
    /// (review D2-2 2026-08-12): check 4 must compare the ticket's binding
    /// against the live context — using the ticket's own field would be a
    /// self-referential tautology that lets an activation-A ticket verify
    /// on activation-B's path.
    /// `live_resolved_target_sha256` (Slice 2 first phase) is the digest
    /// re-derived from the live parsed real target — action kinds only;
    /// control kinds pass None (check 5b).
    pub async fn verify_and_consume(
        &mut self,
        ticket: &ControlTicket,
        canonical_arguments_sha256: &str,
        live_activation_id: Option<String>,
        live_resolved_target_sha256: Option<String>,
    ) -> Result<TicketOutcome, AcafClientError> {
        let session = self
            .session
            .as_ref()
            .ok_or(AcafClientError::SessionNotInitialised)?;
        let vctx = VerifyContext {
            session_id: session.session_id.clone(),
            agent_id: session.agent_id.clone(),
            activation_id: live_activation_id,
            goal_version: session.goal_version,
            goal_digest: session.goal_digest.clone(),
            policy_revision: session.policy_revision,
            // Check 2 compares against the SIGNER-DOWNLOADED built-in
            // template version (session field), never the ticket's own claim
            // — a tampered template field already fails check 1 (it is part
            // of the canonical body), this is the independent second layer.
            template_sha256: Some(session.template_sha256.clone()),
            canonical_arguments_sha256: canonical_arguments_sha256.to_string(),
            resolved_target_sha256: live_resolved_target_sha256,
            now_unix_secs: chrono::Utc::now().timestamp(),
        };
        let verification = verify_ticket(&session.k_session, ticket, &vctx);
        // Review P2-6 (2026-08-12): an unknown kind is a protocol anomaly,
        // not a ticket verdict — surfacing it as a rejected orientation_v1
        // would journal a WRONG kind. Error out instead (the caller journals
        // signer_unreachable — a shadow-mode rejection either way).
        let kind = match ticket.ticket_kind.as_str() {
            "orientation_v1" => TicketKind::OrientationV1,
            "disposition_v1" => TicketKind::DispositionV1,
            "close_v1" => TicketKind::CloseV1,
            "goal_revision_v1" => TicketKind::GoalRevisionV1,
            "file_write_v1" => TicketKind::FileWriteV1,
            "credential_read_v1" => TicketKind::CredentialReadV1,
            other => {
                return Err(AcafClientError::Protocol(format!(
                    "unknown ticket_kind {other}"
                )));
            }
        };
        if verification.valid {
            match self
                .ledger
                .consume(&session.session_id, &ticket.nonce, ticket.sequence)
            {
                Ok(()) => Ok(TicketOutcome::Consumed {
                    ticket_id: ticket.ticket_id.clone(),
                    kind,
                }),
                Err(code) => Ok(TicketOutcome::Rejected {
                    ticket_id: Some(ticket.ticket_id.clone()),
                    kind,
                    code,
                    detail: "nonce already consumed or sequence regressed".to_string(),
                }),
            }
        } else {
            let code = verification
                .reject_codes
                .first()
                .copied()
                .unwrap_or(RejectCode::ContextMismatch);
            Ok(TicketOutcome::Rejected {
                ticket_id: Some(ticket.ticket_id.clone()),
                kind,
                code,
                detail: verification.errors.first().cloned().unwrap_or_default(),
            })
        }
    }

    /// Close the signer session and terminate the child process.
    pub async fn shutdown(&mut self) {
        if let Some(session) = &self.session {
            let _ = self
                .call(
                    "close_session",
                    serde_json::json!({ "session_id": session.session_id }),
                )
                .await;
        }
        let _ = self.child.kill().await;
    }

    /// One JSON-lines request/response round-trip (self-healing wrapper).
    ///
    /// Review P1-1 (2026-08-12): a failed request may leave the response
    /// stream misaligned (a late reply from the timed-out request, a
    /// half-written line, a dead child) — the channel can never recover on
    /// its own. Any failure except a healthy `signer_error` response kills
    /// and respawns the signer (re-initialising the cached session via the
    /// deterministic HKDF) so the next call starts from a clean stream.
    async fn call(&mut self, method: &str, params: Value) -> Result<Value, AcafClientError> {
        match self.call_raw(method, params).await {
            Ok(result) => Ok(result),
            Err(e @ AcafClientError::Signer { .. }) => Err(e),
            Err(e) => {
                let _ = self.respawn().await;
                Err(e)
            }
        }
    }

    /// The raw JSON-lines round-trip (no self-healing — used by `call` and
    /// by `respawn`'s re-initialisation, which must not recurse).
    async fn call_raw(&mut self, method: &str, params: Value) -> Result<Value, AcafClientError> {
        let id = self.next_id;
        self.next_id += 1;
        let request = serde_json::json!({ "id": id, "method": method, "params": params });
        let line = serde_json::to_string(&request)
            .map_err(|e| AcafClientError::Protocol(e.to_string()))?;
        match tokio::time::timeout(REQUEST_TIMEOUT, async {
            self.stdin.write_all(line.as_bytes()).await?;
            self.stdin.write_all(b"\n").await?;
            self.stdin.flush().await?;
            let mut response_line = String::new();
            let n = self.stdout.read_line(&mut response_line).await?;
            if n == 0 {
                return Err(AcafClientError::Closed("signer exited".into()));
            }
            let response: Value = serde_json::from_str(&response_line)
                .map_err(|e| AcafClientError::Protocol(e.to_string()))?;
            if response.get("id").and_then(Value::as_u64) != Some(id) {
                return Err(AcafClientError::Protocol(format!(
                    "response id {} != request id {id}",
                    response.get("id").and_then(Value::as_u64).unwrap_or_default()
                )));
            }
            if let Some(error) = response.get("error") {
                return Err(AcafClientError::Signer {
                    code: error
                        .get("code")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown")
                        .to_string(),
                    message: error
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                });
            }
            response
                .get("result")
                .cloned()
                .ok_or_else(|| AcafClientError::Protocol("response missing result".into()))
        })
        .await
        {
            Ok(inner) => inner,
            Err(_) => Err(AcafClientError::Timeout),
        }
    }
}

fn hex_decode(hex: &str) -> Option<Vec<u8>> {
    if !hex.len().is_multiple_of(2) {
        return None;
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect()
}

/// Re-derive the canonical arguments digest for a control event (check 5 —
/// "the parsed real object"). The controller calls this with the LIVE
/// arguments at the consumption point, so any tampering between signing and
/// consumption changes the digest.
pub fn canonical_arguments_digest(kind: TicketKind, args: &serde_json::Value) -> String {
    let value = serde_json::json!({
        "ticket_kind": kind.as_str(),
        "args": args,
    });
    // Review P2-7 (2026-08-12): canonical_json over a serde_json::Value is
    // infallible — silently degrading to sha256(empty) on failure would
    // sign a WRONG digest; expect() makes the invariant loud.
    let canonical = orz_assurance::journal::canonical_json(&value)
        .expect("canonical_json over a serde_json::Value is infallible");
    orz_assurance::journal::sha256_hex(&canonical)
}

/// Build the `control_ticket_issued` event payload from a ticket (schema
/// `control-ticket-issued-event-payload-v0.2` — binding fields only, never
/// the HMAC tag).
pub fn issued_payload(ticket: &ControlTicket) -> Value {
    serde_json::json!({
        "ticket_id": ticket.ticket_id,
        "ticket_kind": ticket.ticket_kind,
        "session_id": ticket.session_id,
        "agent_id": ticket.agent_id,
        "activation_id": ticket.activation_id,
        "goal_version": ticket.goal_version,
        "goal_digest": ticket.goal_digest,
        "policy_revision": ticket.policy_revision,
        "sequence": ticket.sequence,
        "capability_scope": ticket.capability_scope,
        "template_sha256": ticket.template_sha256,
        "canonical_arguments_sha256": ticket.canonical_arguments_sha256,
        "resolved_target_sha256": ticket.resolved_target_sha256,
        "issued_at": ticket.issued_at,
        "expires_at": ticket.expires_at,
        "signer_revision": ticket.signer_revision,
        "signer_measurement": ticket.signer_measurement,
    })
}

/// Build the `control_ticket_consumed` event payload.
pub fn consumed_payload(outcome: &TicketOutcome, now: &chrono::DateTime<chrono::Utc>) -> Value {
    match outcome {
        TicketOutcome::Consumed { ticket_id, kind } => serde_json::json!({
            "ticket_id": ticket_id,
            "ticket_kind": kind.as_str(),
            "consumed_at": now.to_rfc3339(),
            "outcome": "accepted",
        }),
        _ => Value::Null,
    }
}

/// Build the `control_ticket_rejected` event payload.
pub fn rejected_payload(outcome: &TicketOutcome, now: &chrono::DateTime<chrono::Utc>) -> Value {
    match outcome {
        TicketOutcome::Rejected {
            ticket_id,
            kind,
            code,
            detail,
        } => serde_json::json!({
            "ticket_id": ticket_id,
            "ticket_kind": kind.as_str(),
            "rejected_at": now.to_rfc3339(),
            "reject_code": code.as_str(),
            "detail": detail,
        }),
        TicketOutcome::SignerUnreachable { kind, detail } => serde_json::json!({
            "ticket_id": Value::Null,
            "ticket_kind": kind.as_str(),
            "rejected_at": now.to_rfc3339(),
            "reject_code": "signer_unreachable",
            "detail": detail,
        }),
        _ => Value::Null,
    }
}

/// Reject codes are a subset of the schema enum — surface `signer_unreachable`
/// and the seven check codes.
pub fn reject_code_str(code: &RejectCode) -> &'static str {
    code.as_str()
}

/// Map a host tool name to its action ticket kind (Slice 2 first phase, D5 —
/// ONE place to extend when command_exec / network join in later phases).
pub(crate) fn action_kind_for_tool(tool: &str) -> Option<TicketKind> {
    match tool {
        "search_replace" => Some(TicketKind::FileWriteV1),
        _ => None,
    }
}

/// Classify the file_write operation against the REAL target (review P1-1
/// 2026-08-12): the tool's create path is "empty `old_string` AND the
/// target does not exist or is empty" — an empty `old_string` on an
/// existing non-empty file is a FULL OVERWRITE (search_replace
/// `handle_new_file_creation`, `empty_old_string_does_not_override=false`
/// by default), which must bind as `modify`. The consumption point calls
/// this again against the live file state (symmetric probe — a file
/// created between issue and verify flips the operation and the ticket
/// mismatches, which is the honest TOCTOU signal).
pub fn file_write_operation(effective: &std::path::Path, args: &Value) -> &'static str {
    let create_path = args
        .get("old_string")
        .and_then(Value::as_str)
        .is_some_and(|s| s.is_empty());
    if !create_path {
        return "modify";
    }
    let empty_or_missing = std::fs::metadata(effective)
        .map(|md| md.len() == 0)
        .unwrap_or(true); // missing → create
    if empty_or_missing {
        "create"
    } else {
        "modify"
    }
}

/// Canonical arguments for a `file_write_v1` action ticket (Slice 2 first
/// phase, D5): the resolved absolute path, the create/modify operation
/// (`file_write_operation`) and the content digest all bind into the
/// ticket. The consumption point re-derives this from the LIVE tool
/// arguments (check 5) — a tampered `new_string` or a retargeted
/// `file_path` is a `target_mismatch`.
pub fn file_write_canonical_args(
    tool: &str,
    resolved_file_path: &str,
    operation: &str,
    args: &Value,
) -> Value {
    let content_sha256 = orz_assurance::sha256_hex(
        args.get("new_string").and_then(Value::as_str).unwrap_or_default().as_bytes(),
    );
    serde_json::json!({
        "tool": tool,
        "file_path": resolved_file_path,
        "operation": operation,
        "content_sha256": content_sha256,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use orz_assurance::acaf::{IssueContext, issue_ticket};

    fn test_ticket(kind: TicketKind, session_key: &[u8; 32], nonce: &str) -> ControlTicket {
        let signer = HmacSha256Signer::new("KEY-TEST", session_key);
        let ctx = IssueContext {
            session_id: "SESS-0001".to_string(),
            agent_id: "main".to_string(),
            activation_id: kind.requires_activation().then(|| "ACT-1".to_string()),
            goal_version: 0,
            goal_digest: "0".repeat(64),
            policy_revision: 0,
            signer_revision: 1,
            signer_measurement: "0".repeat(64),
            template_sha256: kind.requires_template().then(|| "0".repeat(64)),
            canonical_arguments_sha256: "0".repeat(64),
            resolved_target_sha256: kind.requires_target().then(|| "0".repeat(64)),
        };
        issue_ticket(&signer, &ctx, kind, 1, nonce, 300, 1_700_000_000).unwrap()
    }

    fn test_key() -> [u8; 32] {
        [7u8; 32]
    }

    fn temp_dir() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "orz-acaf-fw-test-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn hex_roundtrip() {
        let bytes = [0u8, 1, 255, 16];
        assert_eq!(hex_decode(&hex_encode_for_test(&bytes)).unwrap(), bytes);
        assert_eq!(hex_decode("xyz"), None);
        assert_eq!(hex_decode("a"), None);
    }

    fn hex_encode_for_test(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn canonical_arguments_digest_is_stable_and_sensitive() {
        let a = canonical_arguments_digest(
            TicketKind::DispositionV1,
            &serde_json::json!({"decision": "close", "delta": null}),
        );
        let b = canonical_arguments_digest(
            TicketKind::DispositionV1,
            &serde_json::json!({"decision": "close", "delta": null}),
        );
        assert_eq!(a, b);
        let c = canonical_arguments_digest(
            TicketKind::DispositionV1,
            &serde_json::json!({"decision": "continue", "delta": "more"}),
        );
        assert_ne!(a, c);
        // Kind is part of the digest — same args under another kind differ.
        let d = canonical_arguments_digest(
            TicketKind::CloseV1,
            &serde_json::json!({"decision": "close", "delta": null}),
        );
        assert_ne!(a, d);
    }

    #[test]
    fn payload_builders_are_schema_shaped() {
        let key = test_key();
        let ticket = test_ticket(TicketKind::OrientationV1, &key, "n1");
        let issued = issued_payload(&ticket);
        assert_eq!(issued["ticket_id"], ticket.ticket_id);
        assert!(issued.get("hmac").is_none(), "HMAC must never be journaled");

        let outcome = TicketOutcome::Consumed {
            ticket_id: ticket.ticket_id.clone(),
            kind: TicketKind::OrientationV1,
        };
        let now = chrono::Utc::now();
        let consumed = consumed_payload(&outcome, &now);
        assert_eq!(consumed["outcome"], "accepted");
        assert_eq!(consumed["ticket_id"], ticket.ticket_id);

        let rejected = TicketOutcome::Rejected {
            ticket_id: Some(ticket.ticket_id.clone()),
            kind: TicketKind::DispositionV1,
            code: RejectCode::ReplayDetected,
            detail: "nonce reused".to_string(),
        };
        let payload = rejected_payload(&rejected, &now);
        assert_eq!(payload["reject_code"], "replay_detected");

        let unreachable = TicketOutcome::SignerUnreachable {
            kind: TicketKind::CloseV1,
            detail: "spawn failed".to_string(),
        };
        let payload = rejected_payload(&unreachable, &now);
        assert_eq!(payload["reject_code"], "signer_unreachable");
        assert!(payload["ticket_id"].is_null());

        // Slice 2: an action-kind issued payload carries the target digest;
        // control-kind payloads leave it null (D11).
        let action = test_ticket(TicketKind::FileWriteV1, &key, "n2");
        let issued = issued_payload(&action);
        assert_eq!(issued["resolved_target_sha256"], "0".repeat(64));
        let control = test_ticket(TicketKind::CloseV1, &key, "n3");
        let issued = issued_payload(&control);
        assert!(issued["resolved_target_sha256"].is_null());
        assert!(issued.get("hmac").is_none(), "HMAC must never be journaled");
    }

    #[test]
    fn file_write_operation_matches_tool_semantics() {
        // Review P1-1 (2026-08-12): the tool's create path is empty
        // `old_string` AND a missing-or-empty target. An existing non-empty
        // file with an empty `old_string` is a FULL OVERWRITE → modify.
        let dir = temp_dir();
        let existing = dir.join("main.rs");
        std::fs::write(&existing, "original").unwrap();
        let empty_file = dir.join("empty.rs");
        std::fs::write(&empty_file, "").unwrap();
        let missing = dir.join("new.rs");
        let create_args = serde_json::json!({"old_string": "", "new_string": "x"});
        let modify_args = serde_json::json!({"old_string": "a", "new_string": "b"});

        assert_eq!(file_write_operation(&existing, &create_args), "modify");
        assert_eq!(file_write_operation(&existing, &modify_args), "modify");
        assert_eq!(file_write_operation(&empty_file, &create_args), "create");
        assert_eq!(file_write_operation(&missing, &create_args), "create");
        assert_eq!(file_write_operation(&missing, &modify_args), "modify");
        // Missing old_string → modify (conservative; the tool call itself
        // fails on its own).
        let no_old = serde_json::json!({"new_string": "x"});
        assert_eq!(file_write_operation(&missing, &no_old), "modify");
    }

    #[test]
    fn file_write_canonical_args_shape() {
        let resolved = r"C:\worktree\src\main.rs";
        let create = file_write_canonical_args("search_replace", resolved, "create", &serde_json::json!({
            "file_path": "src/main.rs",
            "old_string": "",
            "new_string": "fn main() {}",
        }));
        assert_eq!(create["operation"], "create");
        assert_eq!(create["file_path"], resolved);
        assert_eq!(create["content_sha256"].as_str().unwrap().len(), 64);
        let modify = file_write_canonical_args("search_replace", resolved, "modify", &serde_json::json!({
            "file_path": "src/main.rs",
            "old_string": "fn main() {}",
            "new_string": "fn main() { println!(\"hi\"); }",
        }));
        assert_eq!(modify["operation"], "modify");
        // The content digest is stable for identical input and sensitive to
        // the new_string bytes.
        let modify_again = file_write_canonical_args("search_replace", resolved, "modify", &serde_json::json!({
            "file_path": "src/main.rs",
            "old_string": "fn main() {}",
            "new_string": "fn main() { println!(\"hi\"); }",
        }));
        assert_eq!(modify["content_sha256"], modify_again["content_sha256"]);
        assert_ne!(create["content_sha256"], modify["content_sha256"]);
    }

    #[test]
    fn action_kind_for_tool_mapping() {
        assert_eq!(
            action_kind_for_tool("search_replace"),
            Some(TicketKind::FileWriteV1)
        );
        assert_eq!(action_kind_for_tool("read_file"), None);
        assert_eq!(action_kind_for_tool("bash"), None);
        assert_eq!(action_kind_for_tool("run_tests"), None);
    }
}
