//! Credential lifecycle guard and leakage auditor — GAK-CRED-001.
//!
//! Ported from Python `assurance/credential_scrub.py` (spec reference):
//!
//! - [`CredentialGuard`] — RAII guard: holds a credential for the duration of
//!   its lifetime, zeroes the owned buffer on release (on drop even under
//!   panic). Rust owns the only copy, so this is real zeroisation — stronger
//!   than the Python best-effort string overwrite.
//! - [`sanitize_child_environment`] — strip credential-bearing variables
//!   before passing an environment to a child process.
//! - [`scan_value_for_credentials`] / [`assert_no_credential_in_value`] —
//!   scan a JSON value for candidate credential values before it is written
//!   to a receipt / journal / artifact.
//! - [`audit_container_mount`] / [`assert_safe_container_mount`] — never
//!   bind-mount credential-bearing host paths into a sandbox/container.
//!
//! Design constraints (GAK-CRED-001): scrubbing is **defense-in-depth**, not
//! cryptographic erasure proof — pagefile/hibernation/WER dumps are outside
//! the process's control. Scrub failures are surfaced (audit record error),
//! never swallowed. The Python static AST call-site audit
//! (`audit_credential_scrub_sites`) has no Rust equivalent: ownership +
//! zeroise-on-drop is enforced by the type system instead.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{SecondsFormat, Utc};
use wildmatch::WildMatch;

/// Minimum length for a candidate credential-like string (avoids false
/// positives on short hex strings).
const MIN_CREDENTIAL_LENGTH: usize = 20;

#[derive(Debug, thiserror::Error)]
pub enum CredentialError {
    #[error("credential target must be a non-empty string")]
    InvalidTarget,

    #[error("failed to acquire credential: {0}")]
    AcquisitionFailed(String),

    #[error("credential leakage detected in {label}: {detail}")]
    LeakageDetected { label: String, detail: String },

    #[error("credential leak risk: {detail}")]
    MountLeakRisk { detail: String },

    #[error("credential already released")]
    AlreadyReleased,
}

// ── audit record ────────────────────────────────────────────────────────────

/// A single credential scrub lifecycle event (in-memory only, never persisted).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrubRecord {
    pub credential_target: String,
    /// ISO-8601 (UTC, millisecond precision).
    pub acquired_at: String,
    /// ISO-8601; empty while the guard is still active.
    pub released_at: String,
    pub scrub_method: &'static str,
    pub scrub_succeeded: bool,
    pub error: String,
}

/// Session-scoped in-memory audit log (mirrors Python module-global `_scrub_audit`).
static SCRUB_AUDIT: std::sync::OnceLock<Mutex<Vec<ScrubRecord>>> = std::sync::OnceLock::new();

fn scrub_audit() -> &'static Mutex<Vec<ScrubRecord>> {
    SCRUB_AUDIT.get_or_init(|| Mutex::new(Vec::new()))
}

/// All scrub lifecycle records (for test inspection).
pub fn get_scrub_audit() -> Vec<ScrubRecord> {
    scrub_audit().lock().unwrap_or_else(|e| e.into_inner()).clone()
}

fn utc_now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// Sequence counter so each guard can be identified in the audit log.
static GUARD_SEQUENCE: AtomicU64 = AtomicU64::new(0);

// ── scrub utilities ─────────────────────────────────────────────────────────

/// Zero a byte buffer in place, then clear it.
///
/// Stronger than the Python overwrite heuristic: this is the actual backing
/// store of the credential (the guard owns the only copy).
pub fn zeroize_bytes(bytes: &mut [u8]) {
    bytes.fill(0);
}

fn zeroize_vec(vec: &mut Vec<u8>) {
    zeroize_bytes(vec.as_mut_slice());
    vec.clear();
}

// ── CredentialGuard ─────────────────────────────────────────────────────────

/// RAII guard that holds a credential and zeroes it on release.
///
/// ```no_run
/// use orz_assurance::credential::CredentialGuard;
///
/// let guard = CredentialGuard::acquire("FEP-Agent/DeepSeek", |target| {
///     orz_assurance::credential::read_windows_credential(target)
/// })?;
/// let key = guard.secret();
/// // key is zeroed when `guard` drops — even on panic
/// # Ok::<(), orz_assurance::credential::CredentialError>(())
/// ```
///
/// The credential is held as `Vec<u8>` (raw bytes as read from the store);
/// use [`CredentialGuard::secret_string`] only when an owned `String` is
/// unavoidable — it creates an unzeroed copy.
pub struct CredentialGuard {
    sequence: u64,
    target: String,
    credential: Option<Vec<u8>>,
    record: ScrubRecord,
}

impl CredentialGuard {
    /// Acquire a credential by reading it through `reader`.
    ///
    /// On read failure the guard is still recorded in the audit log with
    /// `scrub_succeeded = true` (nothing to scrub), mirroring the Python
    /// `CredentialGuard.__enter__`.
    pub fn acquire(
        target: &str,
        reader: impl FnOnce(&str) -> Result<Vec<u8>, CredentialError>,
    ) -> Result<Self, CredentialError> {
        if target.is_empty() {
            return Err(CredentialError::InvalidTarget);
        }
        let sequence = GUARD_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let mut record = ScrubRecord {
            credential_target: target.to_string(),
            acquired_at: utc_now_iso(),
            released_at: String::new(),
            scrub_method: "rust-in-place-zeroize",
            scrub_succeeded: false,
            error: String::new(),
        };
        let credential = match reader(target) {
            Ok(bytes) => bytes,
            Err(error) => {
                record.scrub_succeeded = true; // nothing acquired, nothing to scrub
                record.released_at = utc_now_iso();
                scrub_audit().lock().unwrap_or_else(|e| e.into_inner()).push(record);
                return Err(error);
            }
        };
        record.acquired_at = utc_now_iso();
        Ok(Self {
            sequence,
            target: target.to_string(),
            credential: Some(credential),
            record,
        })
    }

    /// The credential bytes (the only live copy; zeroed on release).
    pub fn secret(&self) -> Option<&[u8]> {
        self.credential.as_deref()
    }

    /// A UTF-8-lossy owned copy. Last resort: creates an unzeroed copy.
    pub fn secret_string(&self) -> Option<String> {
        self.credential
            .as_ref()
            .map(|b| String::from_utf8_lossy(b).into_owned())
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    /// Zeroise now and record the scrub (idempotent).
    pub fn release(&mut self) {
        if self.record.released_at.is_empty() {
            self.record.scrub_succeeded = match self.credential.as_mut() {
                Some(bytes) => {
                    zeroize_vec(bytes);
                    true
                }
                None => true, // nothing held
            };
            self.record.released_at = utc_now_iso();
            if let Some(credential) = self.credential.take() {
                debug_assert!(
                    credential.is_empty(),
                    "credential must be zeroed before release"
                );
            }
            scrub_audit().lock().unwrap_or_else(|e| e.into_inner()).push(self.record.clone());
        }
    }

    pub fn scrub_succeeded(&self) -> bool {
        self.record.scrub_succeeded
    }
}

impl Drop for CredentialGuard {
    fn drop(&mut self) {
        self.release();
    }
}

impl std::fmt::Debug for CredentialGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CredentialGuard")
            .field("sequence", &self.sequence)
            .field("target", &self.target)
            .field("secret", &"[REDACTED]")
            .finish()
    }
}

// ── Windows Credential Manager ──────────────────────────────────────────────

/// Read a generic credential from Windows Credential Manager.
///
/// Mirrors Python `_read_windows_credential` (CredReadW, CRED_TYPE_GENERIC).
/// The credential-manager buffer is zeroed before `CredFree` releases it;
/// the returned `Vec<u8>` is owned by the caller (use a
/// [`CredentialGuard`] to zero it on drop).
#[cfg(windows)]
pub fn read_windows_credential(target: &str) -> Result<Vec<u8>, CredentialError> {
    use windows::Win32::Security::Credentials::{
        CRED_TYPE_GENERIC, CREDENTIALW, CredFree, CredReadW,
    };
    use windows::core::PCWSTR;

    let target_wide: Vec<u16> = target.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let mut credential: *mut CREDENTIALW = std::ptr::null_mut();
        CredReadW(
            PCWSTR(target_wide.as_ptr()),
            CRED_TYPE_GENERIC,
            None,
            &mut credential,
        )
        .map_err(|e| CredentialError::AcquisitionFailed(format!("CredReadW({target}): {e}")))?;
        if credential.is_null() {
            return Err(CredentialError::AcquisitionFailed(format!(
                "no credential found for {target}"
            )));
        }
        let cred = &*credential;
        let blob_size = cred.CredentialBlobSize as usize;
        let owned = if blob_size > 0 && !cred.CredentialBlob.is_null() {
            let blob = std::slice::from_raw_parts(cred.CredentialBlob, blob_size);
            let owned = blob.to_vec();
            // Zero the credential manager's buffer before releasing it.
            std::slice::from_raw_parts_mut(cred.CredentialBlob, blob_size).fill(0);
            owned
        } else {
            Vec::new()
        };
        CredFree(credential as *const core::ffi::c_void);
        Ok(owned)
    }
}

#[cfg(not(windows))]
pub fn read_windows_credential(_target: &str) -> Result<Vec<u8>, CredentialError> {
    Err(CredentialError::AcquisitionFailed(
        "Windows Credential Manager is only available on Windows".to_string(),
    ))
}

// ── child environment sanitization ──────────────────────────────────────────

/// Environment variable names that commonly carry credentials.
/// These are NEVER passed to child processes (Python `_CREDENTIAL_ENV_NAMES`).
pub const CREDENTIAL_ENV_NAMES: &[&str] = &[
    // Generic
    "API_KEY",
    "API_TOKEN",
    "SECRET",
    "PASSWORD",
    "PASS",
    "TOKEN",
    "AUTH_TOKEN",
    "ACCESS_TOKEN",
    "BEARER_TOKEN",
    "CREDENTIAL",
    "DEEPSEEK_API_KEY",
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    // Cloud / CI
    "AWS_SECRET_ACCESS_KEY",
    "AWS_ACCESS_KEY_ID",
    "AZURE_CLIENT_SECRET",
    "GCP_SERVICE_ACCOUNT_KEY",
    // Git / VCS
    "GITHUB_TOKEN",
    "GITLAB_TOKEN",
    "GIT_ASKPASS",
    // Windows
    "DPAPI_ENTROPY",
    "CREDENTIAL_TARGET",
    // Project-specific
    "FEP_AGENT_DEEPSEEK_KEY",
    "GSA_CREDENTIAL_TARGET",
];

/// Substrings that flag an env var name as potentially credential-bearing
/// (case-insensitive, Python `_CREDENTIAL_ENV_NAME_PATTERNS`).
pub const CREDENTIAL_ENV_NAME_PATTERNS: &[&str] =
    &["key", "secret", "token", "password", "credential", "auth"];

/// True if `name` looks like it could carry a credential.
pub fn env_name_looks_like_credential(name: &str) -> bool {
    let upper = name.to_uppercase();
    if CREDENTIAL_ENV_NAMES
        .iter()
        .any(|n| n.eq_ignore_ascii_case(&upper))
    {
        return true;
    }
    CREDENTIAL_ENV_NAME_PATTERNS
        .iter()
        .any(|pattern| upper.contains(&pattern.to_uppercase()))
}

/// Environment with credential-bearing variables stripped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SanitizedEnvironment {
    /// The surviving variables, in input order.
    pub vars: Vec<(String, String)>,
    /// Names that were removed.
    pub removed: Vec<String>,
}

/// Return a copy of `env` with credential-bearing variables removed.
/// Call this before passing an environment to a child process.
pub fn sanitize_child_environment(env: &[(String, String)]) -> SanitizedEnvironment {
    let mut vars = Vec::new();
    let mut removed = Vec::new();
    for (key, value) in env {
        if env_name_looks_like_credential(key) {
            removed.push(key.clone());
        } else {
            vars.push((key.clone(), value.clone()));
        }
    }
    SanitizedEnvironment { vars, removed }
}

/// Audit dict describing credential exposure in `env` (does NOT modify it).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ChildEnvironmentAudit {
    pub audit_kind: &'static str,
    pub total_vars: usize,
    pub flagged_credential_vars: Vec<String>,
    pub flagged_count: usize,
    pub safe: bool,
}

pub fn audit_child_environment(env: &[(String, String)]) -> ChildEnvironmentAudit {
    let mut flagged = Vec::new();
    for (key, _) in env {
        if env_name_looks_like_credential(key) {
            flagged.push(key.clone());
        }
    }
    flagged.sort();
    let flagged_count = flagged.len();
    ChildEnvironmentAudit {
        audit_kind: "child_environment_credential_audit",
        total_vars: env.len(),
        flagged_credential_vars: flagged,
        flagged_count,
        safe: flagged_count == 0,
    }
}

// ── artifact credential leakage scanner ─────────────────────────────────────

/// Heuristic: does `value` look like it could be a credential?
///
/// Conservative — may have false positives but very few false negatives for
/// typical API key formats. Ported verbatim from Python `_looks_like_api_key`.
pub fn looks_like_api_key(value: &str) -> bool {
    if value.len() < MIN_CREDENTIAL_LENGTH {
        return false;
    }
    // Skip values that are clearly file paths, URLs, or structured data.
    if value.starts_with("http://")
        || value.starts_with("https://")
        || value.starts_with('/')
        || value.starts_with('\\')
        || value.starts_with('{')
        || value.starts_with('[')
        || value.starts_with('<')
    {
        return false;
    }
    // Skip SHA-256 digests (64 hex chars).
    if value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit()) {
        return false;
    }
    // Skip ISO-8601 timestamps (e.g. "2026-07-28T12:00:00Z").
    if value.len() >= 20 {
        let b = value.as_bytes();
        if b[4] == b'-'
            && b[7] == b'-'
            && (b[10] == b'T' || b[10] == b' ')
            && b[13] == b':'
            && b[16] == b':'
        {
            return false;
        }
    }
    // Skip natural-language sentences (contain whitespace + common words).
    if value.contains(char::is_whitespace) && value.split_whitespace().count() >= 3 {
        return false;
    }
    // Key-like: reasonable length, mixed content with sufficient entropy —
    // at least 3 digits AND 3 letters (avoids "caspase-3" / dates in filenames).
    let digit_count = value.chars().filter(|c| c.is_ascii_digit()).count();
    let letter_count = value.chars().filter(|c| c.is_ascii_alphabetic()).count();
    digit_count >= 3 && letter_count >= 3
}

/// One finding from a credential scan: JSONPath-like location + reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialFinding {
    pub path: String,
    pub key: String,
    pub reason: String,
}

/// Recursively scan `value` for strings that look like credentials.
/// An empty result means no suspicious values were found.
pub fn scan_value_for_credentials(value: &serde_json::Value) -> Vec<CredentialFinding> {
    let mut findings = Vec::new();
    scan_inner(value, "$", &mut findings);
    findings
}

fn scan_inner(obj: &serde_json::Value, path: &str, findings: &mut Vec<CredentialFinding>) {
    match obj {
        serde_json::Value::Object(map) => {
            for (key, value) in map {
                let child_path = format!("{path}.{key}");
                match value {
                    serde_json::Value::String(s) if looks_like_api_key(s) => {
                        findings.push(CredentialFinding {
                            path: child_path,
                            key: key.clone(),
                            reason: "value resembles an API key / credential".to_string(),
                        });
                    }
                    serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
                        scan_inner(value, &child_path, findings);
                    }
                    _ => {}
                }
            }
        }
        serde_json::Value::Array(items) => {
            for (idx, item) in items.iter().enumerate() {
                let child_path = format!("{path}[{idx}]");
                match item {
                    serde_json::Value::String(s) if looks_like_api_key(s) => {
                        findings.push(CredentialFinding {
                            path: child_path,
                            key: format!("[{idx}]"),
                            reason: "value resembles an API key / credential".to_string(),
                        });
                    }
                    serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
                        scan_inner(item, &child_path, findings);
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

/// Raise [`CredentialError::LeakageDetected`] if `value` contains a
/// credential-like string. Call before writing any receipt, answer packet,
/// journal event, or other artifact to disk.
pub fn assert_no_credential_in_value(
    value: &serde_json::Value,
    label: &str,
) -> Result<(), CredentialError> {
    let findings = scan_value_for_credentials(value);
    if findings.is_empty() {
        return Ok(());
    }
    let detail = findings
        .iter()
        .take(5)
        .map(|f| format!("{} ({})", f.path, f.reason))
        .collect::<Vec<_>>()
        .join("; ");
    let detail = if findings.len() > 5 {
        format!("{detail} ... and {} more", findings.len() - 5)
    } else {
        detail
    };
    Err(CredentialError::LeakageDetected {
        label: label.to_string(),
        detail,
    })
}

// ── container mount credential exclusion ────────────────────────────────────

/// Host paths under which credentials or keystore material may reside on
/// Windows. These must NEVER be bind-mounted into a container/sandbox
/// (Python `_FORBIDDEN_HOST_MOUNT_ROOTS`).
pub const FORBIDDEN_HOST_MOUNT_ROOTS: &[&str] = &[
    // Windows Credential Manager storage
    r"C:\Users\*\AppData\Roaming\Microsoft\Credentials",
    r"C:\Users\*\AppData\Local\Microsoft\Credentials",
    r"C:\Users\*\AppData\Roaming\Microsoft\Protect",
    r"C:\Users\*\AppData\Local\Microsoft\Protect",
    // DPAPI master key directory
    r"C:\Users\*\AppData\Roaming\Microsoft\Crypto",
    r"C:\Users\*\AppData\Local\Microsoft\Crypto",
    // General secret storage
    r"C:\Users\*\.ssh",
    r"C:\Users\*\.gnupg",
    // Project keystore
    "**/keystore/**",
    "**/*.dpapi",
    "**/*.pem",
    "**/*.key",
];

/// fnmatch-style match (`*` = any run, `?` = one char) — Python fnmatch with
/// Windows normcase (case-insensitive).
fn fnmatch(pattern: &str, value: &str) -> bool {
    let pattern = pattern.to_lowercase();
    let value = value.to_lowercase();
    WildMatch::new(&pattern).matches(&value)
}

/// Result of checking whether `host_path` could expose credential material.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ContainerMountAudit {
    pub audit_kind: &'static str,
    pub host_path: String,
    pub label: String,
    pub safe: bool,
    pub warnings: Vec<String>,
}

/// Check whether `host_path` could expose credential material into a
/// container. Does NOT raise — callers decide based on `safe`.
pub fn audit_container_mount(host_path: &str) -> ContainerMountAudit {
    let mut warnings = Vec::new();
    let normalized = host_path.replace('/', "\\");
    let normalized = normalized.trim_end_matches('\\');

    for forbidden in FORBIDDEN_HOST_MOUNT_ROOTS {
        let fnorm = forbidden.replace('/', "\\");
        let fnorm_without_glob_tail = fnorm.strip_suffix("\\**").unwrap_or(&fnorm);
        if fnmatch(&fnorm, normalized)
            || fnmatch(&format!("{fnorm}\\*"), normalized)
            || fnmatch(fnorm_without_glob_tail, normalized)
            || fnmatch(&format!("{fnorm_without_glob_tail}\\*"), normalized)
        {
            warnings.push(format!(
                "host path '{host_path}' matches credential-sensitive pattern '{forbidden}'"
            ));
            break;
        }
    }

    // Also check parent dirs.
    let mut parent = std::path::Path::new(host_path).parent();
    while let Some(p) = parent {
        let pstr = p.to_string_lossy().replace('/', "\\");
        let pstr = pstr.trim_end_matches('\\').to_string();
        for forbidden in FORBIDDEN_HOST_MOUNT_ROOTS {
            let fnorm = forbidden.replace('/', "\\");
            if fnmatch(&fnorm, &pstr) {
                warnings.push(format!(
                    "host path '{host_path}' is inside credential-sensitive directory '{pstr}' (pattern '{forbidden}')"
                ));
                break;
            }
        }
        parent = p.parent();
    }

    ContainerMountAudit {
        audit_kind: "container_mount_credential_audit",
        host_path: host_path.to_string(),
        label: "container mount".to_string(),
        safe: warnings.is_empty(),
        warnings,
    }
}

/// Raise [`CredentialError::MountLeakRisk`] if `host_path` could expose
/// credentials inside a container/sandbox.
pub fn assert_safe_container_mount(host_path: &str) -> Result<(), CredentialError> {
    let audit = audit_container_mount(host_path);
    if !audit.safe {
        return Err(CredentialError::MountLeakRisk {
            detail: audit.warnings.join("; "),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_holds_and_zeroes_secret() {
        let guard = CredentialGuard::acquire("test-target", |_| {
            Ok(b"sk-0123456789abcdef0123456789abcdef".to_vec())
        })
        .unwrap();
        assert_eq!(guard.target(), "test-target");
        assert_eq!(
            guard.secret().unwrap(),
            &b"sk-0123456789abcdef0123456789abcdef"[..]
        );
        assert_eq!(
            guard.secret_string().unwrap(),
            "sk-0123456789abcdef0123456789abcdef"
        );
        assert!(!guard.scrub_succeeded()); // not yet released — nothing scrubbed so far

        // Explicit release zeroes the buffer.
        let mut guard = guard;
        guard.release();
        assert_eq!(guard.secret(), None);
        assert!(guard.scrub_succeeded());
    }

    #[test]
    fn zeroize_bytes_wipes_all() {
        let mut buf = vec![1u8, 2, 3, 4, 5];
        zeroize_bytes(&mut buf);
        assert_eq!(buf, vec![0u8, 0, 0, 0, 0]);
    }

    #[test]
    fn drop_releases_and_records_audit() {
        let target = format!("drop-target-{}", std::process::id());
        {
            let guard = CredentialGuard::acquire(&target, |_| Ok(vec![b'x'; 32])).unwrap();
            assert_eq!(guard.secret().unwrap().len(), 32);
        } // drop → zeroise + audit
        let records: Vec<_> = get_scrub_audit()
            .into_iter()
            .filter(|r| r.credential_target == target)
            .collect();
        assert_eq!(records.len(), 1);
        let record = &records[0];
        assert!(record.scrub_succeeded);
        assert!(!record.released_at.is_empty());
        assert!(record.released_at >= record.acquired_at);
        assert_eq!(record.scrub_method, "rust-in-place-zeroize");
    }

    #[test]
    fn failed_acquisition_records_clean_release() {
        let target = format!("fail-target-{}", std::process::id());
        let err = CredentialGuard::acquire(&target, |_| {
            Err(CredentialError::AcquisitionFailed(
                "no such credential".into(),
            ))
        })
        .unwrap_err();
        assert!(matches!(err, CredentialError::AcquisitionFailed(_)));
        let records: Vec<_> = get_scrub_audit()
            .into_iter()
            .filter(|r| r.credential_target == target)
            .collect();
        assert_eq!(records.len(), 1);
        assert!(records[0].scrub_succeeded); // nothing to scrub
    }

    #[test]
    fn empty_target_rejected() {
        let err = CredentialGuard::acquire("", |_| Ok(vec![])).unwrap_err();
        assert!(matches!(err, CredentialError::InvalidTarget));
    }

    /// Windows 凭据存储语义（Linux 上不可用——明确按平台门控）。
    #[cfg(windows)]
    #[test]
    fn windows_credential_read_fails_cleanly_for_missing_target() {
        // No real credential manager entry can be assumed in CI; the contract
        // is: missing target → error (never panic), and the error carries the
        // target name.
        let target = format!(
            "GSA-TEST-NO-SUCH-CREDENTIAL-{}-{}",
            std::process::id(),
            GUARD_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        );
        let err = read_windows_credential(&target).unwrap_err();
        assert!(matches!(err, CredentialError::AcquisitionFailed(_)));
        assert!(err.to_string().contains(&target));
    }

    #[test]
    fn sanitize_removes_exact_and_pattern_matches() {
        let env = vec![
            ("SAFE_VAR".to_string(), "ok".to_string()),
            ("API_KEY".to_string(), "secret".to_string()),
            ("MY_PASSWORD2".to_string(), "secret".to_string()),
            ("GITHUB_TOKEN".to_string(), "secret".to_string()),
            ("GSA_CREDENTIAL_TARGET".to_string(), "secret".to_string()),
            ("PATH".to_string(), "C:\\bin".to_string()),
        ];
        let sanitized = sanitize_child_environment(&env);
        assert_eq!(
            sanitized.removed,
            vec![
                "API_KEY".to_string(),
                "MY_PASSWORD2".to_string(),
                "GITHUB_TOKEN".to_string(),
                "GSA_CREDENTIAL_TARGET".to_string(),
            ]
        );
        assert_eq!(
            sanitized.vars,
            vec![
                ("SAFE_VAR".to_string(), "ok".to_string()),
                ("PATH".to_string(), "C:\\bin".to_string())
            ]
        );
    }

    #[test]
    fn audit_flags_credential_vars() {
        let env = vec![
            ("PATH".to_string(), "C:\\bin".to_string()),
            ("DEEPSEEK_API_KEY".to_string(), "x".to_string()),
        ];
        let audit = audit_child_environment(&env);
        assert!(!audit.safe);
        assert_eq!(audit.total_vars, 2);
        assert_eq!(
            audit.flagged_credential_vars,
            vec!["DEEPSEEK_API_KEY".to_string()]
        );
        assert!(audit_child_environment(&[("PATH".to_string(), "C:\\bin".to_string())]).safe);
    }

    #[test]
    fn scan_finds_nested_credential() {
        let value = serde_json::json!({
            "safe": "plain text",
            "credentials": {
                "api_key": "sk-0123456789abcdef0123456789abcdef",
                "nested": [
                    {"token": "ghp_0123456789abcdefghijklmnopqrstuvwxyz"}
                ]
            }
        });
        let findings = scan_value_for_credentials(&value);
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].path, "$.credentials.api_key");
        assert_eq!(findings[1].path, "$.credentials.nested[0].token");
        assert!(assert_no_credential_in_value(&value, "test artifact").is_err());
    }

    #[test]
    fn scan_ignores_non_credential_strings() {
        let value = serde_json::json!({
            "sha": "a".repeat(64),
            "timestamp": "2026-07-28T12:00:00Z",
            "sentence": "the quick brown fox jumps over the lazy dog",
            "path": "/usr/local/bin/very/long/absolute/path/here",
            "short": "abc123",
            "caspase-3": "protein",
            "numbers": 12345,
            "list": [1, 2, {"x": "2026-07-28 12:00:00"}],
        });
        let findings = scan_value_for_credentials(&value);
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
        assert!(assert_no_credential_in_value(&value, "clean artifact").is_ok());
    }

    #[test]
    fn mount_audit_detects_forbidden_roots() {
        let audit = audit_container_mount(r"C:\Users\me\AppData\Roaming\Microsoft\Protect");
        assert!(!audit.safe);
        assert_eq!(audit.warnings.len(), 1);

        let audit = audit_container_mount(r"C:\Users\me\.ssh");
        assert!(!audit.safe);

        let audit = audit_container_mount(r"C:\Users\me\.ssh\id_rsa");
        assert!(!audit.safe);

        let audit = audit_container_mount(r"D:\project\keystore\keys");
        assert!(!audit.safe);

        let audit = audit_container_mount(r"D:\project\config\secrets.pem");
        assert!(!audit.safe);
    }

    #[test]
    fn mount_audit_passes_safe_paths() {
        for path in [
            r"C:\Users\me\project",
            r"C:\Users\me\project\src\main.rs",
            r"D:\work\repo\.git\config",
            r"C:\Users\me",
        ] {
            let audit = audit_container_mount(path);
            assert!(
                audit.safe,
                "expected {path} to be safe: {:?}",
                audit.warnings
            );
        }
    }

    #[test]
    fn mount_audit_is_case_insensitive() {
        let audit = audit_container_mount(r"c:\users\ME\appdata\roaming\microsoft\protect");
        assert!(!audit.safe);
    }

    #[test]
    fn assert_safe_mount_raises_on_leak() {
        assert!(assert_safe_container_mount(r"C:\Users\me\project").is_ok());
        assert!(
            assert_safe_container_mount(r"C:\Users\me\AppData\Local\Microsoft\Crypto").is_err()
        );
    }
}
