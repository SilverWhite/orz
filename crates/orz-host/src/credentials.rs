//! Credential access for the web_search executor (ADR-0006 / ADR-0010 §3.4).
//!
//! The web_search tool executes against the DeepSeek Responses API
//! (`/v1/responses`, server-side web search) — the SAME provider and SAME
//! key as the main transport (2026-08-11 direction correction: the earlier
//! xAI Grok search backend — a second provider + second key — was
//! withdrawn by user adjudication; retrieval must come from the current
//! provider, not an external search API).
//!
//! The key therefore comes from the existing DeepSeek credential channel
//! (orz-loop `gateway/credentials.rs` `read_agent_api_key`: Windows
//! Credential Manager `orz-deepseek/agent` with in-place blob zeroing;
//! non-Windows `ORZ_DEEPSEEK_API_KEY` env, the ADR-0006 §ext 2026-08-07
//! Linux container channel). No new registry target, no new env var.
//!
//! The `CredentialReader` trait is the host's test seam: an injected
//! failing reader fixes web_search to Disabled regardless of the platform
//! (semaphore and routing tests must never hit a real API).

use std::fmt;
use std::sync::Arc;

/// Failure reading the DeepSeek key.
#[derive(Debug, Clone)]
pub struct CredentialError {
    pub message: String,
}

impl fmt::Display for CredentialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CredentialError {}

/// Credential reader — the injection seam for tests. `read` returns the
/// DeepSeek API key; failures disable web_search (the capability probe
/// reports it; never a silent fallback).
pub trait CredentialReader: Send + Sync {
    fn read(&self) -> Result<String, CredentialError>;
}

/// Production reader — delegates to the existing DeepSeek channel
/// (orz-loop `read_agent_api_key`: Windows Credential Manager + non-Windows
/// env; blob zeroing is that module's discipline, mirrored from GAK-CRED-001).
pub struct DeepSeekCredentialReader;

impl CredentialReader for DeepSeekCredentialReader {
    fn read(&self) -> Result<String, CredentialError> {
        orz_loop::gateway::credentials::read_agent_api_key().map_err(|e| CredentialError {
            message: e.0,
        })
    }
}

/// The default (production) reader.
pub fn web_search_reader() -> Arc<dyn CredentialReader> {
    Arc::new(DeepSeekCredentialReader)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The seam contract: a failing reader disables web_search (the host
    /// tests rely on this to stay offline).
    #[test]
    fn reader_seam_contract() {
        struct Fail;
        impl CredentialReader for Fail {
            fn read(&self) -> Result<String, CredentialError> {
                Err(CredentialError {
                    message: "test: no credential".into(),
                })
            }
        }
        assert!(Fail.read().is_err());
    }
}
