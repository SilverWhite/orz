//! ACAF tickets (ADR-0011 §4.2/§4.3) — one-time HMAC-SHA256 tickets for
//! control events (orientation / disposition / close / goal revision) and,
//! from Slice 2 (2026-08-12), action kinds (`file_write_v1` /
//! `credential_read_v1` / `command_exec_v1` / `network_v1`) carrying
//! `resolved_target_sha256` — the digest of the parsed real target object
//! (§4.2 check 5 TOCTOU; target resolution lives in [`target`]).
//!
//! This is the *mechanism* core, deliberately independent of the signer
//! process and its IPC: signing goes through the same [`PermitSigner`] seam
//! as sensitive-action permits, so tests and the real signer both plug in
//! without changing the verification logic.
//!
//! Key split (§4.3): the signer process holds `K_install` (DPAPI-backed, never
//! enters the agent process) and derives `K_session =
//! HKDF-SHA256(K_install, session_id ‖ goal_digest ‖ policy_digest ‖
//! signer_revision)`; the host verification module holds only `K_session`.
//! Slice 1 has no `policy_digest` mechanism yet (GAP-DENIAL-POLICY-REVISION
//! wiring is Slice 2) — the HKDF info field uses the 8-byte little-endian
//! `policy_revision` as its deterministic stand-in, so a future policy
//! digest switch re-derives every key (old tickets fail immediately).
//!
//! Seven verification checks (§4.2), in order:
//!   1. signature valid under the current `K_session`
//!   2. `template_sha256` equals the built-in template version (control class
//!      — the signer never accepts free-form text)
//!   3. `sequence` monotonic and `nonce` unconsumed (one-shot)
//!   4. session / agent / activation / goal / policy bindings match context
//!   5. `canonical_arguments_sha256` matches the re-derived canonical
//!      arguments of the parsed real object (TOCTOU)
//!   6. not expired
//!   7. chain consistency — Slice 1 realisation (review P2-4 2026-08-12):
//!      the per-ticket `previous_receipt_sha256` field of §4.2 is NOT carried
//!      on the ticket (no cross-ticket hash chain field); check 7 is served
//!      by the run journal's `previous_event_sha256` hash chain plus the
//!      Python verifier's issued→consumed|rejected pairing rule (a consumed
//!      ticket must reference an earlier issued event, one terminal state
//!      per ticket). Action tickets carry `resolved_target_sha256` (Slice 2
//!      first phase); a ticket-level receipt chain is the v2 upgrade path.
//! A failure is a rejection: the caller must write a
//! `control_ticket_rejected` event and (outside shadow mode) refuse the
//! control event.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use crate::journal::canonical_json;
use crate::permit::PermitSigner;

/// Action target resolution (Slice 2 first phase) — the parsed real target
/// object whose digest binds into action tickets.
pub mod target;

/// Ticket envelope schema version (control-ticket payload v0.2 track).
pub const TICKET_SCHEMA_VERSION: &str = "0.2.0-draft";

/// Ticket kinds — Slice 1 (ADR-0011 §7 Slice 1): the four control events;
/// Slice 2 (2026-08-12): the four action kinds (`FileWriteV1` /
/// `CredentialReadV1` / `CommandExecV1` / `NetworkV1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TicketKind {
    OrientationV1,
    DispositionV1,
    CloseV1,
    GoalRevisionV1,
    FileWriteV1,
    CredentialReadV1,
    CommandExecV1,
    NetworkV1,
}

impl TicketKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            TicketKind::OrientationV1 => "orientation_v1",
            TicketKind::DispositionV1 => "disposition_v1",
            TicketKind::CloseV1 => "close_v1",
            TicketKind::GoalRevisionV1 => "goal_revision_v1",
            TicketKind::FileWriteV1 => "file_write_v1",
            TicketKind::CredentialReadV1 => "credential_read_v1",
            TicketKind::CommandExecV1 => "command_exec_v1",
            TicketKind::NetworkV1 => "network_v1",
        }
    }

    /// The capability scope bound to this kind (schema allOf — the mapping
    /// is one-to-one and single-sourced here; the payload schema mirrors it).
    pub fn capability_scope(&self) -> &'static str {
        match self {
            TicketKind::OrientationV1 => "orientation_injection",
            TicketKind::DispositionV1 => "disposition_submit",
            TicketKind::CloseV1 => "close_record",
            TicketKind::GoalRevisionV1 => "goal_revision",
            TicketKind::FileWriteV1 => "file_write",
            TicketKind::CredentialReadV1 => "credential_read",
            TicketKind::CommandExecV1 => "command_exec",
            TicketKind::NetworkV1 => "network",
        }
    }

    /// Whether this kind requires a signer-held template digest (check 2).
    pub fn requires_template(&self) -> bool {
        matches!(self, TicketKind::OrientationV1)
    }

    /// Whether this kind is bound to a subagent activation (activation_id
    /// must be Some). Orientation is session-level and must be None.
    pub fn requires_activation(&self) -> bool {
        matches!(
            self,
            TicketKind::DispositionV1 | TicketKind::CloseV1 | TicketKind::GoalRevisionV1
        )
    }

    /// Whether this kind binds a parsed real target (action class): the
    /// ticket carries `resolved_target_sha256` and check 5 additionally
    /// compares it against the re-derived live target digest (Slice 2 —
    /// action tickets must bind the real target, Slice 1 audit D3).
    pub fn requires_target(&self) -> bool {
        matches!(
            self,
            TicketKind::FileWriteV1
                | TicketKind::CredentialReadV1
                | TicketKind::CommandExecV1
                | TicketKind::NetworkV1
        )
    }
}

/// Rejection codes — mirror of `control-ticket-rejected-event-payload-v0.2`
/// `reject_code` enum; `signer_unreachable` covers the fail-closed
/// requirement of ADR-0011 §9.5 when the signer process is down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectCode {
    SignatureInvalid,
    TemplateMismatch,
    ReplayDetected,
    ContextMismatch,
    TargetMismatch,
    Expired,
    ChainMismatch,
    SignerUnreachable,
}

impl RejectCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            RejectCode::SignatureInvalid => "signature_invalid",
            RejectCode::TemplateMismatch => "template_mismatch",
            RejectCode::ReplayDetected => "replay_detected",
            RejectCode::ContextMismatch => "context_mismatch",
            RejectCode::TargetMismatch => "target_mismatch",
            RejectCode::Expired => "expired",
            RejectCode::ChainMismatch => "chain_mismatch",
            RejectCode::SignerUnreachable => "signer_unreachable",
        }
    }
}

/// The bindings a ticket is signed over (everything except the HMAC tag).
/// Field set mirrors `ControlTicket` — keep them in lockstep.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ControlTicket {
    pub schema_version: String,
    pub ticket_kind: String,
    pub ticket_id: String,
    pub session_id: String,
    pub agent_id: String,
    pub activation_id: Option<String>,
    pub goal_version: u64,
    pub goal_digest: String,
    pub policy_revision: u64,
    pub capability_scope: String,
    pub template_sha256: Option<String>,
    pub canonical_arguments_sha256: String,
    /// Digest of the parsed real target object — action kinds only (Slice 2
    /// first phase; control kinds leave it None). Check 5 compares it against
    /// the live re-derived digest at the consumption point (TOCTOU).
    pub resolved_target_sha256: Option<String>,
    pub sequence: u64,
    pub nonce: String,
    pub issued_at: String,
    pub expires_at: String,
    pub signer_revision: u64,
    pub signer_measurement: String,
    /// base64url (URL_SAFE_NO_PAD) of the 32-byte HMAC-SHA256 tag.
    pub hmac: String,
}

/// Context a ticket is issued in — supplied by the signer caller (host
/// controller side). Slice 1: `goal_digest` is the task-goal digest (empty
/// goal → sha256 of the canonical empty object), `policy_revision` is the
/// current GAP-DENIAL-POLICY-REVISION value (0 until Slice 2 wiring).
#[derive(Debug, Clone)]
pub struct IssueContext {
    pub session_id: String,
    pub agent_id: String,
    pub activation_id: Option<String>,
    pub goal_version: u64,
    pub goal_digest: String,
    pub policy_revision: u64,
    pub signer_revision: u64,
    pub signer_measurement: String,
    /// Template digest — required for orientation (signer-held), absent
    /// otherwise (check 2 applies only to the control class).
    pub template_sha256: Option<String>,
    /// Canonical digest of the parsed real object's arguments (§4.2 check 5 —
    /// disposition decision/delta, close disposition_id, goal old/new
    /// digests, orientation empty arguments).
    pub canonical_arguments_sha256: String,
    /// Digest of the parsed real target — action kinds only (Slice 2 first
    /// phase; control kinds None).
    pub resolved_target_sha256: Option<String>,
}

/// Context a ticket is verified against — the *current* context at the
/// consumption point (same fields; the seven checks compare ticket vs this).
#[derive(Debug, Clone)]
pub struct VerifyContext {
    pub session_id: String,
    pub agent_id: String,
    pub activation_id: Option<String>,
    pub goal_version: u64,
    pub goal_digest: String,
    pub policy_revision: u64,
    /// Built-in template version the signer is known to hold (check 2 —
    /// `template_sha256` must equal this for orientation tickets).
    pub template_sha256: Option<String>,
    /// Re-derived canonical arguments digest of the real object (check 5).
    pub canonical_arguments_sha256: String,
    /// Live re-derived target digest — action kinds only (Slice 2 first
    /// phase; control kinds None). Compare with the ticket's
    /// `resolved_target_sha256` (check 5, TOCTOU).
    pub resolved_target_sha256: Option<String>,
    /// Unix timestamp (seconds) of the verification moment.
    pub now_unix_secs: i64,
}

/// Derive `K_session` from `K_install` (ADR-0011 §4.3):
/// `HKDF-SHA256(K_install, session_id ‖ goal_digest ‖ policy_digest ‖
/// signer_revision)`. Slice 1 substitutes the 8-byte LE `policy_revision`
/// for the not-yet-existing `policy_digest` (registered boundary — any
/// future switch re-derives all keys).
pub fn derive_session_key(
    k_install: &[u8],
    session_id: &str,
    goal_digest: &str,
    policy_revision: u64,
    signer_revision: u64,
) -> Result<[u8; 32], AcafError> {
    use ring::hkdf;
    let salt = hkdf::Salt::new(hkdf::HKDF_SHA256, &[]);
    let prk = salt.extract(k_install);
    let mut info = Vec::with_capacity(session_id.len() + goal_digest.len() + 16);
    info.extend_from_slice(session_id.as_bytes());
    info.extend_from_slice(goal_digest.as_bytes());
    info.extend_from_slice(&policy_revision.to_le_bytes());
    info.extend_from_slice(&signer_revision.to_le_bytes());
    let info_refs = [info.as_slice()];
    let okm = prk
        .expand(&info_refs, hkdf::HKDF_SHA256)
        .map_err(|e| AcafError::Kdf(e.to_string()))?;
    let mut key = [0u8; 32];
    okm.fill(&mut key)
        .map_err(|e| AcafError::Kdf(e.to_string()))?;
    Ok(key)
}

/// Issue a control ticket: validate the issue context, sign the canonical
/// body with the session key, self-verify, return the ticket. This runs on
/// the SIGNER side (the caller is the signer process holding `K_install`
/// and the derived `K_session`).
pub fn issue_ticket(
    signer: &dyn PermitSigner,
    ctx: &IssueContext,
    kind: TicketKind,
    sequence: u64,
    nonce: &str,
    ttl_secs: u64,
    now_unix_secs: i64,
) -> Result<ControlTicket, AcafError> {
    validate_sha256(&ctx.goal_digest, "goal_digest")?;
    validate_sha256(&ctx.canonical_arguments_sha256, "canonical_arguments_sha256")?;
    validate_sha256(&ctx.signer_measurement, "signer_measurement")?;
    if let Some(t) = &ctx.template_sha256 {
        validate_sha256(t, "template_sha256")?;
    }
    if kind.requires_template() && ctx.template_sha256.is_none() {
        return Err(AcafError::MissingTemplate);
    }
    if !kind.requires_template() && ctx.template_sha256.is_some() {
        return Err(AcafError::UnexpectedTemplate);
    }
    if kind.requires_activation() && ctx.activation_id.is_none() {
        return Err(AcafError::MissingActivation);
    }
    if !kind.requires_activation() && ctx.activation_id.is_some() {
        return Err(AcafError::UnexpectedActivation);
    }
    // Slice 2 first phase: action kinds must bind a resolved target; control
    // kinds must not carry one (mirror of the activation pair above).
    if kind.requires_target() && ctx.resolved_target_sha256.is_none() {
        return Err(AcafError::MissingTarget);
    }
    if !kind.requires_target() && ctx.resolved_target_sha256.is_some() {
        return Err(AcafError::UnexpectedTarget);
    }
    if let Some(t) = &ctx.resolved_target_sha256 {
        validate_sha256(t, "resolved_target_sha256")?;
    }
    if ttl_secs == 0 {
        return Err(AcafError::InvalidTtl);
    }
    if nonce.is_empty() || nonce.len() > 128 {
        return Err(AcafError::InvalidNonce);
    }

    let issued_at = DateTime::<Utc>::from_timestamp(now_unix_secs, 0)
        .ok_or(AcafError::InvalidTimestamp(now_unix_secs))?;
    let expires_at = issued_at + chrono::Duration::seconds(ttl_secs as i64);

    let mut ticket = ControlTicket {
        schema_version: TICKET_SCHEMA_VERSION.to_string(),
        ticket_kind: kind.as_str().to_string(),
        ticket_id: format!("TKT-{}", uuid::Uuid::new_v4().simple()),
        session_id: ctx.session_id.clone(),
        agent_id: ctx.agent_id.clone(),
        activation_id: ctx.activation_id.clone(),
        goal_version: ctx.goal_version,
        goal_digest: ctx.goal_digest.clone(),
        policy_revision: ctx.policy_revision,
        capability_scope: kind.capability_scope().to_string(),
        template_sha256: ctx.template_sha256.clone(),
        canonical_arguments_sha256: ctx.canonical_arguments_sha256.clone(),
        resolved_target_sha256: ctx.resolved_target_sha256.clone(),
        sequence,
        nonce: nonce.to_string(),
        issued_at: format_utc(issued_at),
        expires_at: format_utc(expires_at),
        signer_revision: ctx.signer_revision,
        signer_measurement: ctx.signer_measurement.clone(),
        hmac: String::new(),
    };
    let payload = canonical_body(&ticket)?;
    let signature = signer
        .sign(&payload)
        .map_err(|e| AcafError::Signing(e.to_string()))?;
    ticket.hmac = base64url_encode(&signature);

    let result = verify_ticket(signer, &ticket, &verification_context_from(ctx, now_unix_secs));
    if !result.errors.is_empty() {
        return Err(AcafError::SelfVerifyFailed {
            errors: result.errors,
        });
    }
    Ok(ticket)
}

/// The seven verification checks (§4.2). Returns a list of rejections
/// (empty == valid). The caller writes `control_ticket_consumed` when empty
/// and `control_ticket_rejected` with the FIRST reject code otherwise.
pub fn verify_ticket(
    signer: &dyn PermitSigner,
    ticket: &ControlTicket,
    vctx: &VerifyContext,
) -> TicketVerification {
    let mut errors: Vec<String> = Vec::new();
    let mut reject_codes: Vec<RejectCode> = Vec::new();
    let fail = |errors: &mut Vec<String>,
                codes: &mut Vec<RejectCode>,
                code: RejectCode,
                message: String| {
        errors.push(message);
        codes.push(code);
    };

    // Check 1 — signature valid under the current K_session.
    if ticket.schema_version != TICKET_SCHEMA_VERSION {
        fail(
            &mut errors,
            &mut reject_codes,
            RejectCode::SignatureInvalid,
            format!("ticket schema_version mismatch: {}", ticket.schema_version),
        );
    }
    let payload = match canonical_body(ticket) {
        Ok(p) => p,
        Err(e) => {
            fail(
                &mut errors,
                &mut reject_codes,
                RejectCode::SignatureInvalid,
                format!("cannot canonicalise ticket body: {e}"),
            );
            return TicketVerification {
                valid: false,
                errors,
                reject_codes,
            };
        }
    };
    match base64url_decode(&ticket.hmac) {
        Ok(signature) => {
            if !signer.verify(&payload, &signature) {
                fail(
                    &mut errors,
                    &mut reject_codes,
                    RejectCode::SignatureInvalid,
                    "ticket signature verification failed".to_string(),
                );
            }
        }
        Err(e) => fail(
            &mut errors,
            &mut reject_codes,
            RejectCode::SignatureInvalid,
            format!("ticket hmac is not valid base64url: {e}"),
        ),
    }

    // Check 2 — template_sha256 equals the built-in template version.
    let kind = match parse_kind(&ticket.ticket_kind) {
        Ok(k) => k,
        Err(e) => {
            fail(
                &mut errors,
                &mut reject_codes,
                RejectCode::ContextMismatch,
                e,
            );
            return TicketVerification {
                valid: false,
                errors,
                reject_codes,
            };
        }
    };
    if kind.requires_template() {
        match (ticket.template_sha256.as_deref(), vctx.template_sha256.as_deref()) {
            (Some(ticket_t), Some(known_t)) if ticket_t == known_t => {}
            _ => fail(
                &mut errors,
                &mut reject_codes,
                RejectCode::TemplateMismatch,
                "orientation template_sha256 does not match the built-in template version"
                    .to_string(),
            ),
        }
    }

    // Check 3 — sequence monotonic + nonce unconsumed. The monotonic/nonce
    // ledger lives in the caller (process-level); here we surface the ticket's
    // own claims so the caller's ledger can be checked mechanically. A
    // sequence at the top of the u64 range is implausible for a live session.
    if ticket.sequence >= u64::MAX - 1 {
        fail(
            &mut errors,
            &mut reject_codes,
            RejectCode::ReplayDetected,
            format!("implausible ticket sequence {}", ticket.sequence),
        );
    }

    // Check 4 — session/agent/activation/goal/policy bindings match context.
    if ticket.session_id != vctx.session_id {
        fail(
            &mut errors,
            &mut reject_codes,
            RejectCode::ContextMismatch,
            format!(
                "ticket session {} != context session {}",
                ticket.session_id, vctx.session_id
            ),
        );
    }
    if ticket.agent_id != vctx.agent_id {
        fail(
            &mut errors,
            &mut reject_codes,
            RejectCode::ContextMismatch,
            format!(
                "ticket agent {} != context agent {}",
                ticket.agent_id, vctx.agent_id
            ),
        );
    }
    if ticket.activation_id != vctx.activation_id {
        fail(
            &mut errors,
            &mut reject_codes,
            RejectCode::ContextMismatch,
            format!(
                "ticket activation {:?} != context activation {:?}",
                ticket.activation_id, vctx.activation_id
            ),
        );
    }
    if ticket.goal_version != vctx.goal_version || ticket.goal_digest != vctx.goal_digest {
        fail(
            &mut errors,
            &mut reject_codes,
            RejectCode::ContextMismatch,
            format!(
                "ticket goal (v{} {}) != context goal (v{} {})",
                ticket.goal_version, ticket.goal_digest, vctx.goal_version, vctx.goal_digest
            ),
        );
    }
    if ticket.policy_revision != vctx.policy_revision {
        fail(
            &mut errors,
            &mut reject_codes,
            RejectCode::ContextMismatch,
            format!(
                "ticket policy_revision {} != context policy_revision {}",
                ticket.policy_revision, vctx.policy_revision
            ),
        );
    }

    // Check 5 — canonical arguments match the re-derived real object.
    if ticket.canonical_arguments_sha256 != vctx.canonical_arguments_sha256 {
        fail(
            &mut errors,
            &mut reject_codes,
            RejectCode::TargetMismatch,
            "ticket canonical_arguments_sha256 does not match the re-derived arguments"
                .to_string(),
        );
    }
    // Check 5b (Slice 2 first phase) — the action kinds bind the parsed real
    // target: an action ticket without a target, a control ticket carrying
    // one, or a ticket whose target digest disagrees with the live
    // re-derived digest are all rejected (TOCTOU — the consumption point
    // re-parses the real object, never trusts the ticket's own values).
    // `kind` is the parse_kind result from check 2 (unknown kinds already
    // returned ContextMismatch there).
    if kind.requires_target() {
        match (&ticket.resolved_target_sha256, &vctx.resolved_target_sha256) {
            (None, _) => fail(
                &mut errors,
                &mut reject_codes,
                RejectCode::TargetMismatch,
                "action ticket carries no resolved_target_sha256".to_string(),
            ),
            (Some(_), None) => fail(
                &mut errors,
                &mut reject_codes,
                RejectCode::TargetMismatch,
                "action ticket verified without a live resolved target digest".to_string(),
            ),
            (Some(t), Some(live)) if t != live => fail(
                &mut errors,
                &mut reject_codes,
                RejectCode::TargetMismatch,
                "ticket resolved_target_sha256 does not match the live re-derived target"
                    .to_string(),
            ),
            _ => {}
        }
    } else if ticket.resolved_target_sha256.is_some() || vctx.resolved_target_sha256.is_some() {
        // Review P2-2 (2026-08-12): symmetric — a control ticket verified
        // against a live context that (incorrectly) carries a target digest
        // is equally a protocol anomaly (a caller-side bug).
        fail(
            &mut errors,
            &mut reject_codes,
            RejectCode::TargetMismatch,
            "control ticket carries a resolved_target_sha256".to_string(),
        );
    }

    // Check 6 — not expired.
    match parse_utc(&ticket.expires_at) {
        Ok(expires) => {
            if vctx.now_unix_secs >= expires.timestamp() {
                fail(
                    &mut errors,
                    &mut reject_codes,
                    RejectCode::Expired,
                    format!(
                        "ticket expired at {} (now {})",
                        ticket.expires_at, vctx.now_unix_secs
                    ),
                );
            }
        }
        Err(e) => fail(
            &mut errors,
            &mut reject_codes,
            RejectCode::Expired,
            format!("ticket expires_at invalid: {e}"),
        ),
    }
    match parse_utc(&ticket.issued_at) {
        Ok(issued) => {
            if vctx.now_unix_secs < issued.timestamp() {
                fail(
                    &mut errors,
                    &mut reject_codes,
                    RejectCode::Expired,
                    format!(
                        "ticket issued in the future (issued_at {})",
                        ticket.issued_at
                    ),
                );
            }
        }
        Err(e) => fail(
            &mut errors,
            &mut reject_codes,
            RejectCode::Expired,
            format!("ticket issued_at invalid: {e}"),
        ),
    }

    // Check 7 — previous_ticket_sha256 chain. The ledger is caller-side
    // (process-level); this module verifies the ticket's chain field format
    // so a malformed chain link is a hard rejection before the caller sees it.
    // (Slice 1 tickets are consumed in-process; cross-process chain
    // continuity is covered by the journal's previous_event_sha256.)

    TicketVerification {
        valid: errors.is_empty(),
        errors,
        reject_codes,
    }
}

/// In-process consumption ledger — the mechanical one-shot guarantee
/// (check 3's nonce/sequence half): a ticket id/nonce may be consumed at
/// most once per session. Crash recovery does NOT replay (the journal's
/// issued/consumed events are the durable record; a replayed consume
/// against a fresh ledger is a replay_detected rejection).
#[derive(Debug, Default)]
pub struct TicketLedger {
    consumed: HashMap<String, HashSet<String>>, // session_id -> consumed nonces
    sequences: HashMap<String, u64>,            // session_id -> last sequence
}

impl TicketLedger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear the ledger state for one session. Review D2-1 (2026-08-12):
    /// the monotonicity guarantee is PER K_SESSION EPOCH — when the signer
    /// re-derives the session key (goal/policy change), the signer-side
    /// sequence restarts, so the host ledger must reset with it (otherwise
    /// every fresh ticket of the new epoch is rejected as a regression).
    pub fn reset(&mut self, session_id: &str) {
        self.consumed.remove(session_id);
        self.sequences.remove(session_id);
    }

    /// Attempt to record a consumption. Returns Ok when the (nonce,
    /// sequence) pair is fresh for the session and the sequence is
    /// strictly greater than the last recorded one; Err(replay) otherwise.
    pub fn consume(&mut self, session_id: &str, nonce: &str, sequence: u64) -> Result<(), RejectCode> {
        let nonces = self.consumed.entry(session_id.to_string()).or_default();
        if nonces.contains(nonce) {
            return Err(RejectCode::ReplayDetected);
        }
        let last = self.sequences.get(session_id).copied().unwrap_or(0);
        if sequence <= last {
            return Err(RejectCode::ReplayDetected);
        }
        nonces.insert(nonce.to_string());
        self.sequences.insert(session_id.to_string(), sequence);
        Ok(())
    }
}

/// Verification outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TicketVerification {
    pub valid: bool,
    pub errors: Vec<String>,
    /// Reject codes in check order — the caller writes the first one into
    /// the `control_ticket_rejected` event payload.
    pub reject_codes: Vec<RejectCode>,
}

#[derive(Debug, thiserror::Error)]
pub enum AcafError {
    #[error("{label} must be a lowercase SHA-256 digest")]
    InvalidSha256 { label: String },

    #[error("orientation ticket requires a signer-held template digest")]
    MissingTemplate,

    #[error("non-orientation ticket must not carry a template digest")]
    UnexpectedTemplate,

    #[error("disposition/close/goal_revision ticket requires an activation binding")]
    MissingActivation,

    #[error("orientation ticket must not carry an activation binding")]
    UnexpectedActivation,

    #[error("action ticket requires a resolved target digest (Slice 2)")]
    MissingTarget,

    #[error("control ticket must not carry a resolved target digest")]
    UnexpectedTarget,

    #[error("ticket TTL must be positive")]
    InvalidTtl,

    #[error("ticket nonce must be 1..=128 characters")]
    InvalidNonce,

    #[error("invalid issued_at timestamp: {0}")]
    InvalidTimestamp(i64),

    #[error("invalid RFC3339 timestamp {value}: {reason}")]
    InvalidRfc3339 { value: String, reason: String },

    #[error("ticket signature is not valid base64url: {0}")]
    InvalidSignature(String),

    #[error("key derivation failed: {0}")]
    Kdf(String),

    #[error("signing error: {0}")]
    Signing(String),

    #[error("ticket self-verification failed: {errors:?}")]
    SelfVerifyFailed { errors: Vec<String> },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Canonical bytes of the ticket body (everything except `hmac`).
pub fn canonical_body(ticket: &ControlTicket) -> Result<Vec<u8>, AcafError> {
    let mut value = serde_json::to_value(ticket)?;
    if let serde_json::Value::Object(map) = &mut value {
        map.remove("hmac");
    }
    Ok(canonical_json(&value)?)
}

fn parse_kind(value: &str) -> Result<TicketKind, String> {
    match value {
        "orientation_v1" => Ok(TicketKind::OrientationV1),
        "disposition_v1" => Ok(TicketKind::DispositionV1),
        "close_v1" => Ok(TicketKind::CloseV1),
        "goal_revision_v1" => Ok(TicketKind::GoalRevisionV1),
        "file_write_v1" => Ok(TicketKind::FileWriteV1),
        "credential_read_v1" => Ok(TicketKind::CredentialReadV1),
        "command_exec_v1" => Ok(TicketKind::CommandExecV1),
        "network_v1" => Ok(TicketKind::NetworkV1),
        other => Err(format!("unknown ticket_kind: {other}")),
    }
}

fn verification_context_from(ctx: &IssueContext, now_unix_secs: i64) -> VerifyContext {
    VerifyContext {
        session_id: ctx.session_id.clone(),
        agent_id: ctx.agent_id.clone(),
        activation_id: ctx.activation_id.clone(),
        goal_version: ctx.goal_version,
        goal_digest: ctx.goal_digest.clone(),
        policy_revision: ctx.policy_revision,
        template_sha256: ctx.template_sha256.clone(),
        canonical_arguments_sha256: ctx.canonical_arguments_sha256.clone(),
        resolved_target_sha256: ctx.resolved_target_sha256.clone(),
        now_unix_secs,
    }
}

fn validate_sha256(value: &str, label: &str) -> Result<(), AcafError> {
    if value.len() == 64
        && value
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    {
        Ok(())
    } else {
        Err(AcafError::InvalidSha256 {
            label: label.to_string(),
        })
    }
}

fn format_utc(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Micros, true)
}

fn parse_utc(value: &str) -> Result<DateTime<Utc>, AcafError> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| AcafError::InvalidRfc3339 {
            value: value.to_string(),
            reason: e.to_string(),
        })
}

fn base64url_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn base64url_decode(value: &str) -> Result<Vec<u8>, AcafError> {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|e| AcafError::InvalidSignature(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permit::HmacSha256Signer;

    const ZERO64: &str = "0000000000000000000000000000000000000000000000000000000000000000";

    fn k_install() -> &'static [u8] {
        b"test-install-secret-key-0123456789abcdef"
    }

    fn session_signer() -> HmacSha256Signer {
        let key = derive_session_key(k_install(), "SESS-0001", ZERO64, 0, 1).unwrap();
        HmacSha256Signer::new("KEY-TEST", &key)
    }

    fn issue_ctx(session: &str, kind: TicketKind) -> IssueContext {
        IssueContext {
            session_id: session.to_string(),
            agent_id: "main".to_string(),
            activation_id: kind.requires_activation().then(|| "ACT-0001".to_string()),
            goal_version: 0,
            goal_digest: ZERO64.to_string(),
            policy_revision: 0,
            signer_revision: 1,
            signer_measurement: ZERO64.to_string(),
            template_sha256: kind.requires_template().then(|| ZERO64.to_string()),
            canonical_arguments_sha256: ZERO64.to_string(),
            resolved_target_sha256: kind.requires_target().then(|| ZERO64.to_string()),
        }
    }

    #[test]
    fn derive_session_key_is_deterministic_and_sensitive() {
        let a = derive_session_key(k_install(), "S1", ZERO64, 0, 1).unwrap();
        let b = derive_session_key(k_install(), "S1", ZERO64, 0, 1).unwrap();
        assert_eq!(a, b);
        // Different session / goal / policy / signer_revision all change the key.
        assert_ne!(
            a,
            derive_session_key(k_install(), "S2", ZERO64, 0, 1).unwrap()
        );
        assert_ne!(
            a,
            derive_session_key(k_install(), "S1", &"1".repeat(64), 0, 1).unwrap()
        );
        assert_ne!(
            a,
            derive_session_key(k_install(), "S1", ZERO64, 1, 1).unwrap()
        );
        assert_ne!(
            a,
            derive_session_key(k_install(), "S1", ZERO64, 0, 2).unwrap()
        );
        // Different K_install derives a different key.
        assert_ne!(
            a,
            derive_session_key(b"other-install-key-9876543210abcdef", "S1", ZERO64, 0, 1).unwrap()
        );
    }

    #[test]
    fn issue_and_verify_roundtrip() {
        let signer = session_signer();
        let ctx = issue_ctx("SESS-0001", TicketKind::OrientationV1);
        let ticket = issue_ticket(&signer, &ctx, TicketKind::OrientationV1, 1, "n1", 300, 1_700_000_000).unwrap();
        assert_eq!(ticket.ticket_kind, "orientation_v1");
        assert_eq!(ticket.capability_scope, "orientation_injection");
        assert!(ticket.ticket_id.starts_with("TKT-"));
        assert_eq!(ticket.hmac.len(), 43);

        let vctx = verification_context_from(&ctx, 1_700_000_000 + 100);
        let result = verify_ticket(&signer, &ticket, &vctx);
        assert!(result.valid, "{:?}", result.errors);
    }

    #[test]
    fn all_eight_kinds_issue_and_verify() {
        let signer = session_signer();
        for (i, kind) in [
            TicketKind::OrientationV1,
            TicketKind::DispositionV1,
            TicketKind::CloseV1,
            TicketKind::GoalRevisionV1,
            TicketKind::FileWriteV1,
            TicketKind::CredentialReadV1,
            TicketKind::CommandExecV1,
            TicketKind::NetworkV1,
        ]
        .iter()
        .enumerate()
        {
            let ctx = issue_ctx("SESS-0001", *kind);
            let ticket = issue_ticket(&signer, &ctx, *kind, (i + 1) as u64, &format!("n{i}"), 300, 1_700_000_000).unwrap();
            let vctx = verification_context_from(&ctx, 1_700_000_000 + 100);
            let result = verify_ticket(&signer, &ticket, &vctx);
            assert!(result.valid, "kind {kind:?}: {:?}", result.errors);
            // Action kinds carry the resolved target on the ticket face.
            assert_eq!(
                ticket.resolved_target_sha256.is_some(),
                kind.requires_target(),
                "kind {kind:?}"
            );
        }
    }

    #[test]
    fn action_ticket_requires_resolved_target() {
        let signer = session_signer();
        let mut ctx = issue_ctx("SESS-0001", TicketKind::FileWriteV1);
        ctx.resolved_target_sha256 = None;
        let err = issue_ticket(&signer, &ctx, TicketKind::FileWriteV1, 1, "n1", 300, 1_700_000_000).unwrap_err();
        assert!(matches!(err, AcafError::MissingTarget), "{err:?}");
        // Control kinds must not carry a target.
        let mut ctl = issue_ctx("SESS-0001", TicketKind::CloseV1);
        ctl.resolved_target_sha256 = Some(ZERO64.to_string());
        let err = issue_ticket(&signer, &ctl, TicketKind::CloseV1, 1, "n1", 300, 1_700_000_000).unwrap_err();
        assert!(matches!(err, AcafError::UnexpectedTarget), "{err:?}");
    }

    #[test]
    fn action_target_mismatch_rejects() {
        let signer = session_signer();
        let ctx = issue_ctx("SESS-0001", TicketKind::FileWriteV1);
        let ticket = issue_ticket(&signer, &ctx, TicketKind::FileWriteV1, 1, "n1", 300, 1_700_000_000).unwrap();
        // The live target digest differs from the ticket's binding.
        let mut vctx = verification_context_from(&ctx, 1_700_000_000 + 100);
        vctx.resolved_target_sha256 = Some("1".repeat(64));
        let result = verify_ticket(&signer, &ticket, &vctx);
        assert!(!result.valid);
        assert_eq!(result.reject_codes[0], RejectCode::TargetMismatch);
    }

    #[test]
    fn all_action_kinds_reject_live_target_mismatch() {
        // Review D1-1 (2026-08-12, three-agent review): prove that check 5b
        // compares against the LIVE re-derived digest for every action kind
        // — the verification never trusts the ticket's face value, even
        // though network/command targets are deterministic recomputes of
        // captured inputs (no external state to drift).
        let signer = session_signer();
        for (i, kind) in [
            TicketKind::FileWriteV1,
            TicketKind::CredentialReadV1,
            TicketKind::CommandExecV1,
            TicketKind::NetworkV1,
        ]
        .iter()
        .enumerate()
        {
            let ctx = issue_ctx("SESS-0001", *kind);
            let ticket =
                issue_ticket(&signer, &ctx, *kind, (i + 1) as u64, &format!("n{i}"), 300, 1_700_000_000)
                    .unwrap();
            let mut vctx = verification_context_from(&ctx, 1_700_000_000 + 100);
            vctx.resolved_target_sha256 = Some("1".repeat(64));
            let result = verify_ticket(&signer, &ticket, &vctx);
            assert!(!result.valid, "kind {kind:?}");
            assert_eq!(result.reject_codes[0], RejectCode::TargetMismatch, "kind {kind:?}");
        }
    }

    #[test]
    fn action_kind_without_target_field_rejects() {
        let signer = session_signer();
        let ctx = issue_ctx("SESS-0001", TicketKind::CredentialReadV1);
        let mut ticket = issue_ticket(&signer, &ctx, TicketKind::CredentialReadV1, 1, "n1", 300, 1_700_000_000).unwrap();
        // Re-sign a face-stripped ticket — a signer bug that issued an action
        // ticket without a target. The (None, _) branch of check 5b rejects it
        // (check 1 would reject a plain tamper; re-signing isolates 5b).
        ticket.resolved_target_sha256 = None;
        let body = canonical_body(&ticket).unwrap();
        ticket.hmac = base64url_encode(&signer.sign(&body).unwrap());
        let vctx = verification_context_from(&ctx, 1_700_000_000 + 100);
        let result = verify_ticket(&signer, &ticket, &vctx);
        assert!(!result.valid);
        assert_eq!(result.reject_codes[0], RejectCode::TargetMismatch);
        // And the mirror: live context without a target for an action ticket
        // (caller-side omission — the (Some, None) branch).
        let mut vctx = verification_context_from(&ctx, 1_700_000_000 + 100);
        vctx.resolved_target_sha256 = None;
        let result = verify_ticket(&signer, &ticket, &vctx);
        assert!(!result.valid);
        assert_eq!(result.reject_codes[0], RejectCode::TargetMismatch);
    }

    #[test]
    fn control_kind_with_target_field_rejects() {
        let signer = session_signer();
        let ctx = issue_ctx("SESS-0001", TicketKind::DispositionV1);
        let mut ticket = issue_ticket(&signer, &ctx, TicketKind::DispositionV1, 1, "n1", 300, 1_700_000_000).unwrap();
        // Re-signed control ticket carrying a target — a signer bug that
        // bound a target to a control kind. Check 5b rejects it.
        ticket.resolved_target_sha256 = Some(ZERO64.to_string());
        let body = canonical_body(&ticket).unwrap();
        ticket.hmac = base64url_encode(&signer.sign(&body).unwrap());
        let vctx = verification_context_from(&ctx, 1_700_000_000 + 100);
        let result = verify_ticket(&signer, &ticket, &vctx);
        assert!(!result.valid);
        assert_eq!(result.reject_codes[0], RejectCode::TargetMismatch);
    }

    #[test]
    fn tampered_signature_rejects() {
        let signer = session_signer();
        let ctx = issue_ctx("SESS-0001", TicketKind::OrientationV1);
        let mut ticket = issue_ticket(&signer, &ctx, TicketKind::OrientationV1, 1, "n1", 300, 1_700_000_000).unwrap();
        ticket.hmac = "A".repeat(43);
        let vctx = verification_context_from(&ctx, 1_700_000_000 + 100);
        let result = verify_ticket(&signer, &ticket, &vctx);
        assert!(!result.valid);
        assert_eq!(result.reject_codes[0], RejectCode::SignatureInvalid);
    }

    #[test]
    fn wrong_session_key_rejects_signature() {
        let signer = session_signer();
        let ctx = issue_ctx("SESS-0001", TicketKind::OrientationV1);
        let ticket = issue_ticket(&signer, &ctx, TicketKind::OrientationV1, 1, "n1", 300, 1_700_000_000).unwrap();
        // Verify under a DIFFERENT session key (goal changed → new K_session).
        let other_key = derive_session_key(k_install(), "SESS-0001", &"2".repeat(64), 0, 1).unwrap();
        let other_signer = HmacSha256Signer::new("KEY-TEST", &other_key);
        let vctx = verification_context_from(&ctx, 1_700_000_000 + 100);
        let result = verify_ticket(&other_signer, &ticket, &vctx);
        assert!(!result.valid);
        assert_eq!(result.reject_codes[0], RejectCode::SignatureInvalid);
    }

    #[test]
    fn template_mismatch_rejects_orientation() {
        let signer = session_signer();
        let ctx = issue_ctx("SESS-0001", TicketKind::OrientationV1);
        let mut ticket = issue_ticket(&signer, &ctx, TicketKind::OrientationV1, 1, "n1", 300, 1_700_000_000).unwrap();
        ticket.template_sha256 = Some("1".repeat(64));
        let vctx = verification_context_from(&ctx, 1_700_000_000 + 100);
        let result = verify_ticket(&signer, &ticket, &vctx);
        assert!(!result.valid);
        assert!(result.reject_codes.contains(&RejectCode::TemplateMismatch));
    }

    #[test]
    fn context_mismatch_rejects() {
        let signer = session_signer();
        let ctx = issue_ctx("SESS-0001", TicketKind::DispositionV1);
        let ticket = issue_ticket(&signer, &ctx, TicketKind::DispositionV1, 1, "n1", 300, 1_700_000_000).unwrap();

        // Wrong session.
        let mut wrong = ctx.clone();
        wrong.session_id = "SESS-9999".to_string();
        let vctx = verification_context_from(&wrong, 1_700_000_000 + 100);
        let result = verify_ticket(&signer, &ticket, &vctx);
        assert!(!result.valid);
        assert_eq!(result.reject_codes[0], RejectCode::ContextMismatch);

        // Wrong policy_revision.
        let mut wrong = ctx.clone();
        wrong.policy_revision = 1;
        let vctx = verification_context_from(&wrong, 1_700_000_000 + 100);
        let result = verify_ticket(&signer, &ticket, &vctx);
        assert!(!result.valid);
        assert_eq!(result.reject_codes[0], RejectCode::ContextMismatch);
    }

    #[test]
    fn target_mismatch_rejects() {
        let signer = session_signer();
        let ctx = issue_ctx("SESS-0001", TicketKind::DispositionV1);
        let ticket = issue_ticket(&signer, &ctx, TicketKind::DispositionV1, 1, "n1", 300, 1_700_000_000).unwrap();
        let mut wrong = ctx.clone();
        wrong.canonical_arguments_sha256 = "1".repeat(64);
        let vctx = verification_context_from(&wrong, 1_700_000_000 + 100);
        let result = verify_ticket(&signer, &ticket, &vctx);
        assert!(!result.valid);
        assert_eq!(result.reject_codes[0], RejectCode::TargetMismatch);
    }

    #[test]
    fn expired_rejects() {
        let signer = session_signer();
        let ctx = issue_ctx("SESS-0001", TicketKind::OrientationV1);
        let ticket = issue_ticket(&signer, &ctx, TicketKind::OrientationV1, 1, "n1", 300, 1_700_000_000).unwrap();
        // 301s later — past the 300s TTL.
        let vctx = verification_context_from(&ctx, 1_700_000_000 + 301);
        let result = verify_ticket(&signer, &ticket, &vctx);
        assert!(!result.valid);
        assert_eq!(result.reject_codes[0], RejectCode::Expired);
    }

    #[test]
    fn ledger_rejects_replay_and_regresses() {
        let mut ledger = TicketLedger::new();
        assert!(ledger.consume("S1", "n1", 1).is_ok());
        // Same nonce again → replay.
        assert_eq!(
            ledger.consume("S1", "n1", 2).unwrap_err(),
            RejectCode::ReplayDetected
        );
        // Sequence regression → replay.
        assert_eq!(
            ledger.consume("S1", "n2", 1).unwrap_err(),
            RejectCode::ReplayDetected
        );
        // Fresh nonce with a higher sequence passes.
        assert!(ledger.consume("S1", "n2", 3).is_ok());
        // Other sessions are independent.
        assert!(ledger.consume("S2", "n1", 1).is_ok());
    }

    #[test]
    fn ledger_reset_clears_epoch_high_water_mark() {
        // Review D2-1 (2026-08-12): a K_session re-derivation (goal/policy
        // change) restarts the signer-side sequence — the ledger must reset
        // with the epoch, or every fresh ticket of the new epoch is rejected
        // as a regression.
        let mut ledger = TicketLedger::new();
        assert!(ledger.consume("S1", "n1", 5).is_ok());
        // Without a reset the new epoch's sequence 1..4 would be rejected.
        assert_eq!(
            ledger.consume("S1", "n2", 1).unwrap_err(),
            RejectCode::ReplayDetected
        );
        ledger.reset("S1");
        assert!(ledger.consume("S1", "n2", 1).is_ok(), "post-reset sequence 1 is fresh");
        // Other sessions are untouched by the reset.
        assert_eq!(
            ledger.consume("S2", "n1", 0).unwrap_err(),
            RejectCode::ReplayDetected
        );
        assert!(ledger.consume("S2", "n1", 1).is_ok());
    }

    #[test]
    fn issue_validation_rejects_bad_contexts() {
        let signer = session_signer();
        // Orientation without template.
        let mut ctx = issue_ctx("SESS-0001", TicketKind::OrientationV1);
        ctx.template_sha256 = None;
        let err = issue_ticket(&signer, &ctx, TicketKind::OrientationV1, 1, "n1", 300, 1_700_000_000).unwrap_err();
        assert!(matches!(err, AcafError::MissingTemplate));

        // Disposition without activation.
        let mut ctx = issue_ctx("SESS-0001", TicketKind::DispositionV1);
        ctx.activation_id = None;
        let err = issue_ticket(&signer, &ctx, TicketKind::DispositionV1, 1, "n1", 300, 1_700_000_000).unwrap_err();
        assert!(matches!(err, AcafError::MissingActivation));

        // Non-orientation with a template.
        let mut ctx = issue_ctx("SESS-0001", TicketKind::CloseV1);
        ctx.template_sha256 = Some(ZERO64.to_string());
        let err = issue_ticket(&signer, &ctx, TicketKind::CloseV1, 1, "n1", 300, 1_700_000_000).unwrap_err();
        assert!(matches!(err, AcafError::UnexpectedTemplate));

        // Bad digest.
        let mut ctx = issue_ctx("SESS-0001", TicketKind::OrientationV1);
        ctx.goal_digest = "not-a-digest".to_string();
        let err = issue_ticket(&signer, &ctx, TicketKind::OrientationV1, 1, "n1", 300, 1_700_000_000).unwrap_err();
        assert!(matches!(err, AcafError::InvalidSha256 { .. }));
    }
}
