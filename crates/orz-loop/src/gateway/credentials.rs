//! Windows Credential Manager access for API keys (ADR-0006 registry).
//!
//! Credential target names are registered in the main repo's
//! `adr/ADR-0006-credential-target-registry.md`; this module reads the
//! main-agent target (`orz-deepseek/agent`) — the only target the Rust side
//! needs today (live transport tests). Non-Windows is fail-closed (mirrors
//! the orz-host keystore's platform posture).
//!
//! The credential blob is zeroed in place before `CredFree` releases the
//! buffer — same discipline as the Python GAK-CRED-001 reader.

use std::fmt;

/// Main-agent DeepSeek credential target (ADR-0006).
pub const AGENT_CREDENTIAL_TARGET: &str = "orz-deepseek/agent";

/// Failure reading a credential — message-only error.
#[derive(Debug)]
pub struct CredentialError(pub String);

impl fmt::Display for CredentialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for CredentialError {}

/// Read the main-agent DeepSeek API key from Windows Credential Manager.
/// Returns a `CredentialError` on any failure (incl. non-Windows).
#[cfg(windows)]
pub fn read_agent_api_key() -> Result<String, CredentialError> {
    use windows::core::PCWSTR;
    use windows::Win32::Security::Credentials::{
        CREDENTIALW, CRED_TYPE_GENERIC, CredFree, CredReadW,
    };

    let target: Vec<u16> = AGENT_CREDENTIAL_TARGET
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let mut pcred: *mut CREDENTIALW = std::ptr::null_mut();
    if let Err(e) = unsafe {
        CredReadW(
            PCWSTR::from_raw(target.as_ptr()),
            CRED_TYPE_GENERIC,
            None,
            &mut pcred,
        )
    } {
        return Err(CredentialError(format!(
            "CredReadW({AGENT_CREDENTIAL_TARGET}) failed: {}",
            e.message()
        )));
    }
    if pcred.is_null() {
        return Err(CredentialError(format!(
            "CredReadW({AGENT_CREDENTIAL_TARGET}) returned a null credential"
        )));
    }

    let cred = unsafe { &*pcred };
    let blob =
        unsafe { std::slice::from_raw_parts(cred.CredentialBlob, cred.CredentialBlobSize as usize) };
    // The credential blob is UTF-16-LE encoded text (the API key).
    let mut key = match std::char::decode_utf16(
        blob.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])),
    )
    .collect::<Result<String, _>>()
    {
        Ok(key) => key,
        Err(e) => {
            unsafe {
                std::ptr::write_bytes(cred.CredentialBlob, 0u8, cred.CredentialBlobSize as usize);
                CredFree(pcred as *const std::ffi::c_void);
            }
            return Err(CredentialError(format!(
                "credential blob is not valid UTF-16: {e}"
            )));
        }
    };
    unsafe {
        std::ptr::write_bytes(cred.CredentialBlob, 0u8, cred.CredentialBlobSize as usize);
        CredFree(pcred as *const std::ffi::c_void);
    }
    // Trim a trailing NUL if the stored value carries one.
    if key.ends_with('\0') {
        key.pop();
    }
    Ok(key)
}

#[cfg(not(windows))]
pub fn read_agent_api_key() -> Result<String, CredentialError> {
    Err(CredentialError(
        "Windows Credential Manager is only available on Windows".into(),
    ))
}
