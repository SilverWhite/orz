//! Digest-bound sensitive action permit (P1) — one-shot lifecycle.
//!
//! Ported from Python `assurance/permit.py` + `sensitive-action-permit-v0.1.schema.json`
//! (spec authority). A permit binds a single sensitive action
//! (`action_sha256` + `target_sha256` + `impact_scope_sha256` + `attempt`) to
//! a confirmation digest and a security envelope; it is issued once, consumed
//! once, and cannot be replayed. Every receipt is signed with
//! HMAC-SHA256 (RFC8785 canonical payload); the consumed receipt chains to the
//! issued receipt via `issued_permit_sha256`.
//!
//! The schema-authority fields (conversation/envelope identity, expiry
//! clamping) come from a caller-supplied [`PermitEnvelope`] — in the host this
//! is the session's active security envelope; the signing key comes from an
//! install-level keystore (see [`PermitSigner`]; Windows DPAPI-backed storage
//! is a host-side concern, tests use [`HmacSha256Signer::from_secret`]).

use std::io::Write;
use std::path::PathBuf;

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use crate::journal::{canonical_json, sha256_hex};

/// Schema constant values (sensitive-action-permit-v0.1.schema.json consts).
pub const SCHEMA_VERSION: &str = "0.1.0-draft";
pub const RECEIPT_KIND: &str = "sensitive_action_permit";
pub const AUTHORITY: &str = "local-interactive-user";
pub const CANONICALIZATION: &str = "RFC8785";
pub const SIGNATURE_ALGORITHM: &str = "hmac-sha256";

/// RFC8785 canonical payload + HMAC-SHA256 integrity block
/// (schema `integrity` object).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PermitIntegrity {
    pub canonicalization: String,
    pub key_id: String,
    pub signature_algorithm: String,
    pub signed_payload_sha256: String,
    /// base64url (URL_SAFE_NO_PAD) of the 32-byte HMAC-SHA256 tag.
    pub signature: String,
}

impl Default for PermitIntegrity {
    fn default() -> Self {
        Self {
            canonicalization: CANONICALIZATION.to_string(),
            key_id: String::new(),
            signature_algorithm: SIGNATURE_ALGORITHM.to_string(),
            signed_payload_sha256: String::new(),
            signature: String::new(),
        }
    }
}

/// The digest binding of a permit (schema `binding` object).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PermitBinding {
    pub action_sha256: String,
    pub target_sha256: String,
    pub impact_scope_sha256: String,
    pub attempt: u32,
}

/// Permit lifecycle state.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PermitState {
    Issued,
    Consumed,
}

/// The security envelope a permit is bound to (host-provided).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermitEnvelope {
    pub conversation_id: String,
    pub envelope_id: String,
    pub expires_at: DateTime<Utc>,
}

/// A sensitive-action permit receipt, serialised exactly per
/// `sensitive-action-permit-v0.1.schema.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SensitiveActionPermit {
    pub schema_version: String,
    pub receipt_kind: String,
    pub receipt_id: String,
    pub permit_id: String,
    pub state: PermitState,
    pub conversation_id: String,
    pub envelope_id: String,
    pub created_at: String,
    pub expires_at: String,
    pub authority: String,
    pub confirmation_sha256: String,
    pub binding: PermitBinding,
    pub issued_permit_sha256: Option<String>,
    pub raw_confirmation_recorded: bool,
    pub raw_action_recorded: bool,
    pub integrity: PermitIntegrity,
}

impl SensitiveActionPermit {
    fn new(
        signer: &dyn PermitSigner,
        envelope: &PermitEnvelope,
        receipt_id: String,
        permit_id: String,
        state: PermitState,
        created_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
        confirmation_sha256: String,
        binding: PermitBinding,
        issued_permit_sha256: Option<String>,
    ) -> Result<Self, PermitError> {
        let mut receipt = Self {
            schema_version: SCHEMA_VERSION.to_string(),
            receipt_kind: RECEIPT_KIND.to_string(),
            receipt_id,
            permit_id,
            state,
            conversation_id: envelope.conversation_id.clone(),
            envelope_id: envelope.envelope_id.clone(),
            created_at: format_utc(created_at),
            expires_at: format_utc(expires_at),
            authority: AUTHORITY.to_string(),
            confirmation_sha256,
            binding,
            issued_permit_sha256,
            raw_confirmation_recorded: false,
            raw_action_recorded: false,
            integrity: PermitIntegrity::default(),
        };
        receipt.sign(signer)?;
        Ok(receipt)
    }

    /// Sign the canonical body (all fields except `integrity`).
    fn sign(&mut self, signer: &dyn PermitSigner) -> Result<(), PermitError> {
        let payload = canonical_body(self)?;
        let signature = signer.sign(&payload)?;
        self.integrity.key_id = signer.key_id().to_string();
        self.integrity.signed_payload_sha256 = sha256_hex(&payload);
        self.integrity.signature = base64url_encode(&signature);
        Ok(())
    }
}

/// Verification outcome (mirrors Python `verify_sensitive_action_permit` return).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermitVerification {
    pub valid: bool,
    pub errors: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum PermitError {
    #[error("{label} must be a SHA-256 digest")]
    InvalidSha256 { label: String },

    #[error("permit attempt must be positive")]
    InvalidAttempt,

    #[error("permit TTL must be positive")]
    InvalidTtl,

    #[error("cannot issue a permit from an expired envelope")]
    ExpiredEnvelope,

    #[error("sensitive action permit is expired")]
    ExpiredPermit,

    #[error("sensitive action does not match permit binding")]
    BindingMismatch,

    #[error("sensitive action permit already consumed by {receipt_id}")]
    AlreadyConsumed { receipt_id: String },

    #[error("sensitive action permit consumption is already claimed")]
    ConsumptionClaimed,

    #[error("permit verification failed: {errors:?}")]
    VerificationFailed { errors: Vec<String> },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("signing error: {0}")]
    Signing(String),
}

/// Signing seam for permit receipts — install-level keystore in production.
pub trait PermitSigner: Send + Sync {
    /// `KEY-...` install key identifier (schema pattern `^KEY-[A-Z0-9._-]+$`).
    fn key_id(&self) -> &str;
    /// Produce an HMAC-SHA256 tag over `payload`.
    fn sign(&self, payload: &[u8]) -> Result<Vec<u8>, PermitError>;
    /// Constant-time verify of a tag over `payload`.
    fn verify(&self, payload: &[u8], signature: &[u8]) -> bool;
}

/// HMAC-SHA256 signer over an install-level key (ring).
pub struct HmacSha256Signer {
    key_id: String,
    key: ring::hmac::Key,
}

impl HmacSha256Signer {
    /// Build a signer with an explicit key id.
    pub fn new(key_id: &str, key_bytes: &[u8]) -> Self {
        Self {
            key_id: key_id.to_string(),
            key: ring::hmac::Key::new(ring::hmac::HMAC_SHA256, key_bytes),
        }
    }

    /// Build a signer whose key id derives from the key material:
    /// `KEY-<UPPERHEX(sha256(key))[..16]>` (schema-compatible).
    pub fn from_secret(key_bytes: &[u8]) -> Self {
        let digest = sha256_hex(key_bytes);
        let key_id = format!("KEY-{}", digest[..16].to_uppercase());
        Self::new(&key_id, key_bytes)
    }
}

impl PermitSigner for HmacSha256Signer {
    fn key_id(&self) -> &str {
        &self.key_id
    }

    fn sign(&self, payload: &[u8]) -> Result<Vec<u8>, PermitError> {
        Ok(ring::hmac::sign(&self.key, payload).as_ref().to_vec())
    }

    fn verify(&self, payload: &[u8], signature: &[u8]) -> bool {
        ring::hmac::verify(&self.key, payload, signature).is_ok()
    }
}

impl std::fmt::Debug for HmacSha256Signer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HmacSha256Signer")
            .field("key_id", &self.key_id)
            .field("key", &"[REDACTED]")
            .finish()
    }
}

/// Artifact store for one-shot permits (`<root>/one_shot_permit/`), mirroring
/// the Python namespace artifact layout.
#[derive(Debug, Clone)]
pub struct PermitStore {
    root: PathBuf,
}

impl PermitStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn dir(&self) -> PathBuf {
        self.root.join("one_shot_permit")
    }

    /// Issue a permit: validate digests, clamp TTL to the envelope expiry,
    /// sign, self-verify, and persist `<permit_id>.issued.json`.
    pub fn issue(
        &self,
        signer: &dyn PermitSigner,
        envelope: &PermitEnvelope,
        confirmation_sha256: &str,
        action_sha256: &str,
        target_sha256: &str,
        impact_scope_sha256: &str,
        attempt: u32,
        ttl_seconds: u64,
        now: Option<DateTime<Utc>>,
    ) -> Result<SensitiveActionPermit, PermitError> {
        validate_sha256(confirmation_sha256, "confirmation")?;
        validate_sha256(action_sha256, "action")?;
        validate_sha256(target_sha256, "target")?;
        validate_sha256(impact_scope_sha256, "impact scope")?;
        if attempt < 1 {
            return Err(PermitError::InvalidAttempt);
        }
        if ttl_seconds < 1 {
            return Err(PermitError::InvalidTtl);
        }
        // Reject TTLs that overflow the chrono duration range instead of
        // silently wrapping to a negative offset (which would mint an
        // already-expired permit).
        let ttl = i64::try_from(ttl_seconds).map_err(|_| PermitError::InvalidTtl)?;
        let selected_now = now.unwrap_or_else(Utc::now);
        if selected_now >= envelope.expires_at {
            return Err(PermitError::ExpiredEnvelope);
        }
        let expires_at = (selected_now + chrono::Duration::seconds(ttl)).min(envelope.expires_at);

        let permit_id = format!("PERMIT-{}", uuid::Uuid::new_v4().simple().to_string().to_uppercase());
        let receipt_id = format!("SAPR-{}", uuid::Uuid::new_v4().simple().to_string().to_uppercase());
        let binding = PermitBinding {
            action_sha256: action_sha256.to_string(),
            target_sha256: target_sha256.to_string(),
            impact_scope_sha256: impact_scope_sha256.to_string(),
            attempt,
        };
        let receipt = SensitiveActionPermit::new(
            signer,
            envelope,
            receipt_id,
            permit_id,
            PermitState::Issued,
            selected_now,
            expires_at,
            confirmation_sha256.to_string(),
            binding,
            None,
        )?;

        let verification = verify_sensitive_action_permit(signer, envelope, &receipt, None);
        if !verification.valid {
            return Err(PermitError::VerificationFailed {
                errors: verification.errors,
            });
        }
        self.write_artifact(
            &format!("{}.issued.json", receipt.permit_id),
            canonical_json(&receipt)?,
        )?;
        Ok(receipt)
    }

    /// Consume a permit: verify the issued receipt, bind the supplied action,
    /// guard against double consumption, then persist claim + consumed receipt.
    pub fn consume(
        &self,
        signer: &dyn PermitSigner,
        envelope: &PermitEnvelope,
        issued: &SensitiveActionPermit,
        action_sha256: &str,
        target_sha256: &str,
        impact_scope_sha256: &str,
        attempt: u32,
        now: Option<DateTime<Utc>>,
    ) -> Result<SensitiveActionPermit, PermitError> {
        let verification = verify_sensitive_action_permit(signer, envelope, issued, None);
        if !verification.valid {
            return Err(PermitError::VerificationFailed {
                errors: verification.errors,
            });
        }
        let supplied = PermitBinding {
            action_sha256: action_sha256.to_string(),
            target_sha256: target_sha256.to_string(),
            impact_scope_sha256: impact_scope_sha256.to_string(),
            attempt,
        };
        if supplied != issued.binding {
            return Err(PermitError::BindingMismatch);
        }
        let selected_now = now.unwrap_or_else(Utc::now);
        let issued_expiry = parse_utc(&issued.expires_at)?;
        if selected_now >= issued_expiry {
            return Err(PermitError::ExpiredPermit);
        }

        let dir = self.dir();
        let consumed_path = dir.join(format!("{}.consumed.json", issued.permit_id));
        let claim_path = dir.join(format!("{}.consumption.claim", issued.permit_id));
        if consumed_path.exists() {
            let existing: SensitiveActionPermit = serde_json::from_slice(&std::fs::read(&consumed_path)?)?;
            return Err(PermitError::AlreadyConsumed {
                receipt_id: existing.receipt_id,
            });
        }
        if claim_path.exists() {
            return Err(PermitError::ConsumptionClaimed);
        }

        let issued_sha256 = sha256_hex(&canonical_json(issued)?);
        let receipt = SensitiveActionPermit::new(
            signer,
            envelope,
            format!("SAPR-{}", uuid::Uuid::new_v4().simple().to_string().to_uppercase()),
            issued.permit_id.clone(),
            PermitState::Consumed,
            selected_now,
            issued_expiry,
            issued.confirmation_sha256.clone(),
            issued.binding.clone(),
            Some(issued_sha256.clone()),
        )?;

        let consumed_verification =
            verify_sensitive_action_permit(signer, envelope, &receipt, Some(issued));
        if !consumed_verification.valid {
            return Err(PermitError::VerificationFailed {
                errors: consumed_verification.errors,
            });
        }

        // Claim first, then the receipt (mirrors Python claim→write ordering).
        // The claim is created with O_CREAT|O_EXCL (Python
        // `exclusive_create_bytes` semantics): under contention only one
        // consumer can create it, so the one-shot guarantee survives
        // concurrent consume calls.
        let claim = serde_json::json!({
            "permit_id": issued.permit_id,
            "issued_permit_sha256": issued_sha256,
            "consumed_receipt_sha256": sha256_hex(&canonical_json(&receipt)?),
        });
        let claim_bytes = canonical_json(&claim)?;
        std::fs::create_dir_all(&dir)?;
        let mut claim_file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&claim_path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    PermitError::ConsumptionClaimed
                } else {
                    PermitError::Io(e)
                }
            })?;
        claim_file.write_all(&claim_bytes)?;
        self.write_artifact(
            &format!("{}.consumed.json", issued.permit_id),
            canonical_json(&receipt)?,
        )?;
        Ok(receipt)
    }

    fn write_artifact(&self, file_name: &str, bytes: Vec<u8>) -> Result<(), PermitError> {
        std::fs::create_dir_all(self.dir())?;
        std::fs::write(self.dir().join(file_name), bytes)?;
        Ok(())
    }
}

/// Verify a permit receipt against its envelope and (for consumed receipts)
/// the issued receipt it chains to. Mirrors Python `verify_sensitive_action_permit`.
pub fn verify_sensitive_action_permit(
    signer: &dyn PermitSigner,
    envelope: &PermitEnvelope,
    receipt: &SensitiveActionPermit,
    issued: Option<&SensitiveActionPermit>,
) -> PermitVerification {
    let mut errors: Vec<String> = Vec::new();

    // Schema consts (sensitive-action-permit-v0.1.schema.json).
    if receipt.schema_version != SCHEMA_VERSION {
        errors.push(format!("schema_version mismatch: {}", receipt.schema_version));
    }
    if receipt.receipt_kind != RECEIPT_KIND {
        errors.push(format!("receipt_kind mismatch: {}", receipt.receipt_kind));
    }
    if receipt.authority != AUTHORITY {
        errors.push(format!("authority mismatch: {}", receipt.authority));
    }
    if receipt.integrity.canonicalization != CANONICALIZATION {
        errors.push("integrity canonicalization mismatch".to_string());
    }
    if receipt.integrity.signature_algorithm != SIGNATURE_ALGORITHM {
        errors.push("integrity signature_algorithm mismatch".to_string());
    }
    if receipt.raw_confirmation_recorded || receipt.raw_action_recorded {
        errors.push("raw confirmation/action must not be recorded".to_string());
    }
    if !(receipt.receipt_id.starts_with("SAPR-")
        && receipt.receipt_id.len() > 5
        && receipt.receipt_id[5..]
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_lowercase()))
    {
        errors.push("receipt_id does not match SAPR-[A-F0-9]+".to_string());
    }
    if !(receipt.permit_id.starts_with("PERMIT-")
        && receipt.permit_id.len() > 7
        && receipt.permit_id[7..]
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_lowercase()))
    {
        errors.push("permit_id does not match PERMIT-[A-F0-9]+".to_string());
    }

    let payload = match canonical_body(receipt) {
        Ok(payload) => payload,
        Err(e) => {
            errors.push(format!("cannot canonicalise permit body: {e}"));
            return PermitVerification {
                valid: false,
                errors,
            };
        }
    };
    if receipt.integrity.key_id != signer.key_id() {
        errors.push("permit integrity key mismatch".to_string());
    }
    if receipt.integrity.signed_payload_sha256 != sha256_hex(&payload) {
        errors.push("permit signed payload digest mismatch".to_string());
    }
    match base64url_decode(&receipt.integrity.signature) {
        Ok(signature) => {
            if !signer.verify(&payload, &signature) {
                errors.push("permit signature verification failed".to_string());
            }
        }
        Err(e) => errors.push(format!("permit signature is not valid base64url: {e}")),
    }
    if receipt.conversation_id != envelope.conversation_id {
        errors.push("permit conversation mismatch".to_string());
    }
    if receipt.envelope_id != envelope.envelope_id {
        errors.push("permit envelope mismatch".to_string());
    }
    match parse_utc(&receipt.expires_at) {
        Ok(expires) => {
            if expires > envelope.expires_at {
                errors.push("permit outlives security envelope".to_string());
            }
        }
        Err(e) => errors.push(format!("permit expires_at invalid: {e}")),
    }

    match receipt.state {
        PermitState::Issued => {
            if issued.is_some() {
                errors.push("issued permit unexpectedly references another receipt".to_string());
            }
            if receipt.issued_permit_sha256.is_some() {
                // Schema allOf: issued state requires issued_permit_sha256: null.
                errors.push("issued permit must not reference an issued receipt".to_string());
            }
        }
        PermitState::Consumed => {
            if receipt.issued_permit_sha256.is_none() {
                // Schema allOf: consumed state requires issued_permit_sha256.
                errors.push("consumed permit lacks issued-receipt digest".to_string());
            }
            let Some(issued_receipt) = issued else {
                errors.push("consumed permit lacks issued receipt".to_string());
                return PermitVerification {
                    valid: errors.is_empty(),
                    errors,
                };
            };
            let issued_verification =
                verify_sensitive_action_permit(signer, envelope, issued_receipt, None);
            if !issued_verification.valid {
                errors.extend(
                    issued_verification
                        .errors
                        .into_iter()
                        .map(|item| format!("issued permit: {item}")),
                );
            }
            if receipt.permit_id != issued_receipt.permit_id {
                errors.push("consumed permit ID mismatch".to_string());
            }
            if receipt.binding != issued_receipt.binding {
                errors.push("consumed permit binding mismatch".to_string());
            }
            if receipt.confirmation_sha256 != issued_receipt.confirmation_sha256 {
                errors.push("consumed permit confirmation mismatch".to_string());
            }
            if receipt.expires_at != issued_receipt.expires_at {
                errors.push("consumed permit expiry mismatch".to_string());
            }
            match canonical_json(issued_receipt) {
                Ok(issued_bytes) => {
                    let expected = sha256_hex(&issued_bytes);
                    if receipt.issued_permit_sha256.as_deref() != Some(expected.as_str()) {
                        errors.push("consumed permit issued-receipt digest mismatch".to_string());
                    }
                }
                Err(e) => errors.push(format!("issued permit canonicalisation failed: {e}")),
            }
        }
    }

    PermitVerification {
        valid: errors.is_empty(),
        errors,
    }
}

// ── helpers ─────────────────────────────────────────────────────────────────

fn validate_sha256(value: &str, label: &str) -> Result<(), PermitError> {
    if value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()) {
        Ok(())
    } else {
        Err(PermitError::InvalidSha256 {
            label: label.to_string(),
        })
    }
}

/// Canonical bytes of the receipt body (everything except `integrity`).
fn canonical_body(receipt: &SensitiveActionPermit) -> Result<Vec<u8>, PermitError> {
    let mut value = serde_json::to_value(receipt)?;
    if let serde_json::Value::Object(map) = &mut value {
        map.remove("integrity");
    }
    Ok(canonical_json(&value)?)
}

fn format_utc(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Micros, true)
}

fn parse_utc(value: &str) -> Result<DateTime<Utc>, PermitError> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| PermitError::Signing(format!("invalid timestamp {value}: {e}")))
}

fn base64url_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn base64url_decode(value: &str) -> Result<Vec<u8>, PermitError> {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|e| PermitError::Signing(format!("invalid base64url: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn test_envelope() -> PermitEnvelope {
        PermitEnvelope {
            conversation_id: "CONV-TEST-01".to_string(),
            envelope_id: "ENV-TEST-01".to_string(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
        }
    }

    fn test_sha(seed: u8) -> String {
        // Contains letters so uppercase/lowercase distinction is testable.
        format!("{:064x}", 0xabcde00000000000u64 | (seed as u64))
    }

    fn signer() -> HmacSha256Signer {
        HmacSha256Signer::from_secret(b"test-install-secret-key-0123456789abcdef")
    }

    fn issue_one(store: &PermitStore, now: DateTime<Utc>) -> SensitiveActionPermit {
        store
            .issue(
                &signer(),
                &test_envelope(),
                &test_sha(1),
                &test_sha(2),
                &test_sha(3),
                &test_sha(4),
                1,
                300,
                Some(now),
            )
            .unwrap()
    }

    #[test]
    fn issue_creates_valid_receipt_and_artifact() {
        let dir = tempfile::tempdir().unwrap();
        let store = PermitStore::new(dir.path().to_path_buf());
        let receipt = issue_one(&store, Utc::now());

        assert_eq!(receipt.schema_version, "0.1.0-draft");
        assert_eq!(receipt.receipt_kind, "sensitive_action_permit");
        assert_eq!(receipt.state, PermitState::Issued);
        assert_eq!(receipt.issued_permit_sha256, None);
        assert_eq!(receipt.authority, "local-interactive-user");
        assert!(!receipt.raw_confirmation_recorded);
        assert!(!receipt.raw_action_recorded);
        assert!(receipt.receipt_id.starts_with("SAPR-"));
        assert!(receipt.permit_id.starts_with("PERMIT-"));
        assert_eq!(receipt.integrity.canonicalization, "RFC8785");
        assert_eq!(receipt.integrity.signature_algorithm, "hmac-sha256");
        assert_eq!(receipt.integrity.signature.len(), 43); // base64url of 32 bytes
        assert!(receipt.integrity.key_id.starts_with("KEY-"));

        let verification = verify_sensitive_action_permit(&signer(), &test_envelope(), &receipt, None);
        assert!(verification.valid, "{:?}", verification.errors);

        // Artifact written.
        let artifact = dir
            .path()
            .join("one_shot_permit")
            .join(format!("{}.issued.json", receipt.permit_id));
        assert!(artifact.exists());
        let bytes = std::fs::read(artifact).unwrap();
        let loaded: SensitiveActionPermit = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(loaded, receipt);
    }

    #[test]
    fn consume_chains_to_issued_and_is_one_shot() {
        let dir = tempfile::tempdir().unwrap();
        let store = PermitStore::new(dir.path().to_path_buf());
        let now = Utc::now();
        let issued = issue_one(&store, now);

        let consumed = store
            .consume(
                &signer(),
                &test_envelope(),
                &issued,
                &test_sha(2),
                &test_sha(3),
                &test_sha(4),
                1,
                Some(now + chrono::Duration::seconds(10)),
            )
            .unwrap();

        assert_eq!(consumed.state, PermitState::Consumed);
        assert_eq!(consumed.permit_id, issued.permit_id);
        assert_eq!(consumed.binding, issued.binding);
        assert_eq!(consumed.expires_at, issued.expires_at);
        assert_eq!(
            consumed.issued_permit_sha256.as_deref(),
            Some(sha256_hex(&canonical_json(&issued).unwrap()).as_str())
        );

        let verification = verify_sensitive_action_permit(&signer(), &test_envelope(), &consumed, Some(&issued));
        assert!(verification.valid, "{:?}", verification.errors);

        // Claim + consumed artifacts exist.
        let dir_path = dir.path().join("one_shot_permit");
        assert!(dir_path.join(format!("{}.consumption.claim", issued.permit_id)).exists());
        assert!(dir_path.join(format!("{}.consumed.json", issued.permit_id)).exists());

        // Second consume must fail (already consumed / claimed).
        let err = store
            .consume(
                &signer(),
                &test_envelope(),
                &issued,
                &test_sha(2),
                &test_sha(3),
                &test_sha(4),
                1,
                Some(now + chrono::Duration::seconds(20)),
            )
            .unwrap_err();
        assert!(matches!(err, PermitError::AlreadyConsumed { .. }));

        // Issued receipt itself remains verifiable and unmodified.
        let verification = verify_sensitive_action_permit(&signer(), &test_envelope(), &issued, None);
        assert!(verification.valid);
    }

    #[test]
    fn consume_rejects_binding_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let store = PermitStore::new(dir.path().to_path_buf());
        let now = Utc::now();
        let issued = issue_one(&store, now);

        let err = store
            .consume(
                &signer(),
                &test_envelope(),
                &issued,
                &test_sha(9), // wrong action digest
                &test_sha(3),
                &test_sha(4),
                1,
                Some(now + chrono::Duration::seconds(10)),
            )
            .unwrap_err();
        assert!(matches!(err, PermitError::BindingMismatch));
    }

    #[test]
    fn consume_rejects_expired_permit() {
        let dir = tempfile::tempdir().unwrap();
        let store = PermitStore::new(dir.path().to_path_buf());
        let now = Utc::now();
        let issued = issue_one(&store, now);

        let err = store
            .consume(
                &signer(),
                &test_envelope(),
                &issued,
                &test_sha(2),
                &test_sha(3),
                &test_sha(4),
                1,
                Some(now + chrono::Duration::seconds(301)), // past the 300s TTL
            )
            .unwrap_err();
        assert!(matches!(err, PermitError::ExpiredPermit));
    }

    #[test]
    fn tampered_signature_or_binding_fails_verification() {
        let dir = tempfile::tempdir().unwrap();
        let store = PermitStore::new(dir.path().to_path_buf());
        let mut receipt = issue_one(&store, Utc::now());

        // Tamper with the signature.
        let mut tampered = receipt.clone();
        tampered.integrity.signature = "A".repeat(43);
        let verification = verify_sensitive_action_permit(&signer(), &test_envelope(), &tampered, None);
        assert!(!verification.valid);
        assert!(verification.errors.iter().any(|e| e.contains("signature")));

        // Tamper with the binding after signing.
        receipt.binding.attempt = 2;
        let verification = verify_sensitive_action_permit(&signer(), &test_envelope(), &receipt, None);
        assert!(!verification.valid);
        assert!(verification
            .errors
            .iter()
            .any(|e| e == "permit signed payload digest mismatch"));
    }

    #[test]
    fn state_and_issued_digest_must_be_consistent() {
        let dir = tempfile::tempdir().unwrap();
        let store = PermitStore::new(dir.path().to_path_buf());
        let now = Utc::now();
        let mut issued = issue_one(&store, now);

        // Issued receipt must not carry an issued-permit digest (schema allOf).
        issued.issued_permit_sha256 = Some("a".repeat(64));
        let verification = verify_sensitive_action_permit(&signer(), &test_envelope(), &issued, None);
        assert!(!verification.valid);
        assert!(verification
            .errors
            .iter()
            .any(|e| e == "issued permit must not reference an issued receipt"));

        // Consumed receipt must carry one.
        let mut consumed = store
            .consume(
                &signer(),
                &test_envelope(),
                &issue_one(&store, now),
                &test_sha(2),
                &test_sha(3),
                &test_sha(4),
                1,
                Some(now + chrono::Duration::seconds(10)),
            )
            .unwrap();
        consumed.issued_permit_sha256 = None;
        let verification = verify_sensitive_action_permit(&signer(), &test_envelope(), &consumed, None);
        assert!(!verification.valid);
        assert!(verification
            .errors
            .iter()
            .any(|e| e == "consumed permit lacks issued-receipt digest"));
    }

    #[test]
    fn concurrent_consume_claims_exactly_once() {
        // The claim file is created with O_CREAT|O_EXCL (Python
        // exclusive_create_bytes): two racing consumes must result in exactly
        // one success and one ConsumptionClaimed.
        let dir = tempfile::tempdir().unwrap();
        let store = PermitStore::new(dir.path().to_path_buf());
        let now = Utc::now();
        let issued = Arc::new(issue_one(&store, now));
        let s = Arc::new(signer());
        let env = Arc::new(test_envelope());

        let barrier = Arc::new(std::sync::Barrier::new(2));
        let handles: Vec<_> = (0..2)
            .map(|_| {
                let issued = Arc::clone(&issued);
                let s = Arc::clone(&s);
                let env = Arc::clone(&env);
                let barrier = Arc::clone(&barrier);
                let store = PermitStore::new(dir.path().to_path_buf());
                std::thread::spawn(move || {
                    barrier.wait();
                    store.consume(
                        s.as_ref(),
                        env.as_ref(),
                        &issued,
                        &test_sha(2),
                        &test_sha(3),
                        &test_sha(4),
                        1,
                        Some(now + chrono::Duration::seconds(10)),
                    )
                })
            })
            .collect();

        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        let successes = results.iter().filter(|r| r.is_ok()).count();
        let claimed = results
            .iter()
            .filter(|r| matches!(r, Err(PermitError::ConsumptionClaimed)))
            .count();
        assert_eq!(successes, 1, "exactly one consume must succeed: {results:?}");
        assert_eq!(claimed, 1, "the racing consume must hit the claim: {results:?}");
    }

    #[test]
    fn ttl_overflow_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let store = PermitStore::new(dir.path().to_path_buf());
        let err = store
            .issue(
                &signer(),
                &test_envelope(),
                &test_sha(1),
                &test_sha(2),
                &test_sha(3),
                &test_sha(4),
                1,
                u64::MAX,
                Some(Utc::now()),
            )
            .unwrap_err();
        assert!(matches!(err, PermitError::InvalidTtl));
    }

    #[test]
    fn wrong_signer_key_fails_verification() {
        let dir = tempfile::tempdir().unwrap();
        let store = PermitStore::new(dir.path().to_path_buf());
        let receipt = issue_one(&store, Utc::now());

        let other_signer = HmacSha256Signer::from_secret(b"some-other-install-key-9876543210");
        let verification = verify_sensitive_action_permit(&other_signer, &test_envelope(), &receipt, None);
        assert!(!verification.valid);
        assert!(verification.errors.iter().any(|e| e == "permit integrity key mismatch"));
    }

    #[test]
    fn issue_rejects_invalid_inputs() {
        let dir = tempfile::tempdir().unwrap();
        let store = PermitStore::new(dir.path().to_path_buf());
        let now = Utc::now();
        let s = signer();
        let envelope = test_envelope();

        // Non-hex digest.
        let err = store
            .issue(&s, &envelope, "xyz", &test_sha(2), &test_sha(3), &test_sha(4), 1, 300, Some(now))
            .unwrap_err();
        assert!(matches!(err, PermitError::InvalidSha256 { .. }));

        // Uppercase hex digest (Python pattern is lowercase-only).
        let err = store
            .issue(&s, &envelope, &test_sha(1).to_uppercase(), &test_sha(2), &test_sha(3), &test_sha(4), 1, 300, Some(now))
            .unwrap_err();
        assert!(matches!(err, PermitError::InvalidSha256 { .. }));

        // Zero attempt.
        let err = store
            .issue(&s, &envelope, &test_sha(1), &test_sha(2), &test_sha(3), &test_sha(4), 0, 300, Some(now))
            .unwrap_err();
        assert!(matches!(err, PermitError::InvalidAttempt));

        // Zero TTL.
        let err = store
            .issue(&s, &envelope, &test_sha(1), &test_sha(2), &test_sha(3), &test_sha(4), 1, 0, Some(now))
            .unwrap_err();
        assert!(matches!(err, PermitError::InvalidTtl));
    }

    #[test]
    fn issue_clamps_to_envelope_expiry() {
        let dir = tempfile::tempdir().unwrap();
        let store = PermitStore::new(dir.path().to_path_buf());
        let now = Utc::now();
        let mut envelope = test_envelope();
        envelope.expires_at = now + chrono::Duration::seconds(30); // shorter than TTL 300

        let receipt = store
            .issue(
                &signer(),
                &envelope,
                &test_sha(1),
                &test_sha(2),
                &test_sha(3),
                &test_sha(4),
                1,
                300,
                Some(now),
            )
            .unwrap();
        // Receipt timestamps are serialised at microsecond precision, so the
        // parsed expiry must equal the envelope expiry within 1 µs.
        let parsed = parse_utc(&receipt.expires_at).unwrap();
        let delta_micros = (envelope.expires_at - parsed).num_microseconds().unwrap_or(0);
        assert!(
            (0..=1).contains(&delta_micros),
            "permit expiry must be clamped to the envelope expiry (delta {delta_micros}µs)"
        );
        // And verification against the same envelope passes.
        let verification = verify_sensitive_action_permit(&signer(), &envelope, &receipt, None);
        assert!(verification.valid, "{:?}", verification.errors);
    }

    #[test]
    fn expired_envelope_cannot_issue() {
        let dir = tempfile::tempdir().unwrap();
        let store = PermitStore::new(dir.path().to_path_buf());
        let now = Utc::now();
        let mut envelope = test_envelope();
        envelope.expires_at = now - chrono::Duration::seconds(1);

        let err = store
            .issue(
                &signer(),
                &envelope,
                &test_sha(1),
                &test_sha(2),
                &test_sha(3),
                &test_sha(4),
                1,
                300,
                Some(now),
            )
            .unwrap_err();
        assert!(matches!(err, PermitError::ExpiredEnvelope));
    }

    #[test]
    fn consumed_with_wrong_issued_receipt_fails() {
        let dir = tempfile::tempdir().unwrap();
        let store = PermitStore::new(dir.path().to_path_buf());
        let now = Utc::now();
        let issued_a = issue_one(&store, now);
        let other_store = PermitStore::new(tempfile::tempdir().unwrap().path().to_path_buf());
        let issued_b = issue_one(&other_store, now);

        // A consumed receipt from A verified against B's issued receipt → ID mismatch.
        let consumed = store
            .consume(
                &signer(),
                &test_envelope(),
                &issued_a,
                &test_sha(2),
                &test_sha(3),
                &test_sha(4),
                1,
                Some(now + chrono::Duration::seconds(10)),
            )
            .unwrap();
        let verification = verify_sensitive_action_permit(&signer(), &test_envelope(), &consumed, Some(&issued_b));
        assert!(!verification.valid);
        assert!(verification.errors.iter().any(|e| e == "consumed permit ID mismatch"));
        assert!(verification
            .errors
            .iter()
            .any(|e| e == "consumed permit issued-receipt digest mismatch"));
    }
}
