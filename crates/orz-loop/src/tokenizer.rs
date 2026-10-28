//! Provider tokenizer — the single source for every hard M1 token budget.
//!
//! 0ds M1 v1 (2026-10-28, user decision): the old `chars/2` estimate
//! under-counts CJK and therefore cannot back a hard `state <= 200`,
//! `context_map <= 1K`, `recenter <= 1K` contract. This module embeds the
//! provider's own tokenizer (DeepSeek-V4.1-Flash, the production transport) and
//! exposes it through an object-safe counter. Callers must propagate
//! [`TokenCountError`]: there is deliberately **no** silent chars/2 fallback
//! on a budget path.

use std::sync::OnceLock;

use tokenizers::Tokenizer;

/// Name recorded in journal payloads / diagnostics.
pub const PROVIDER_TOKENIZER_NAME: &str = "deepseek-v4.1-flash";

/// Embedded HF tokenizer for the production DeepSeek transport.
///
/// The file is the provider-published
/// `deepseek-ai/DeepSeek-V4.1-Flash/tokenizer.json` (vocab 128000); it is
/// compiled in so offline runs never depend on HF Hub.
const DEEPSEEK_TOKENIZER_JSON: &[u8] =
    include_bytes!("../assets/deepseek-v4.1-flash-tokenizer.json");

static DEEPSEEK_TOKENIZER: OnceLock<Result<Tokenizer, String>> = OnceLock::new();

/// Token counting failure. Budget paths fail closed (write/read is rejected)
/// rather than falling back to an estimate that could exceed the contract.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TokenCountError {
    #[error("provider tokenizer unavailable: {0}")]
    Unavailable(String),
    #[error("provider tokenizer encode failed: {0}")]
    Encode(String),
}

/// Object-safe provider tokenizer contract.
pub trait TokenCounter: Send + Sync {
    /// Count provider tokens for `text` (no special tokens added).
    fn count_tokens(&self, text: &str) -> Result<u64, TokenCountError>;

    /// Stable identifier for journal/diagnostic payloads.
    fn name(&self) -> &'static str;
}

/// DeepSeek provider tokenizer (the v1 production transport).
#[derive(Debug, Clone, Copy, Default)]
pub struct DeepSeekTokenizer;

impl DeepSeekTokenizer {
    pub fn new() -> Self {
        Self
    }

    fn tokenizer() -> Result<&'static Tokenizer, TokenCountError> {
        let cell = DEEPSEEK_TOKENIZER.get_or_init(|| {
            Tokenizer::from_bytes(DEEPSEEK_TOKENIZER_JSON)
                .map_err(|err| format!("embedded deepseek-v4.1-flash tokenizer: {err}"))
        });
        match cell {
            Ok(tokenizer) => Ok(tokenizer),
            Err(err) => Err(TokenCountError::Unavailable(err.clone())),
        }
    }
}

impl TokenCounter for DeepSeekTokenizer {
    fn count_tokens(&self, text: &str) -> Result<u64, TokenCountError> {
        let encoding = Self::tokenizer()?
            .encode(text, false)
            .map_err(|err| TokenCountError::Encode(err.to_string()))?;
        Ok(encoding.len() as u64)
    }

    fn name(&self) -> &'static str {
        PROVIDER_TOKENIZER_NAME
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_tokenizer_counts_ascii_and_cjk() {
        let counter = DeepSeekTokenizer::new();
        assert_eq!(counter.count_tokens("").unwrap(), 0);
        let ascii = counter.count_tokens("hello world").unwrap();
        assert!(ascii >= 2, "ascii tokens: {ascii}");
        // The old chars/2 estimator would say 200 for 401 CJK chars; the
        // provider tokenizer exposes the real cost. Use a repeated common
        // character so the assertion is stable across tokenizer revisions.
        let cjk = counter.count_tokens(&"一".repeat(401)).unwrap();
        assert!(cjk > 200, "CJK must not be under-counted by chars/2: {cjk}");
        assert!(cjk <= 401, "provider tokenizer must be sane: {cjk}");
    }

    #[test]
    fn provider_tokenizer_is_deterministic_and_named() {
        let counter = DeepSeekTokenizer::new();
        let a = counter.count_tokens("state v1\ngoal: 测试").unwrap();
        let b = counter.count_tokens("state v1\ngoal: 测试").unwrap();
        assert_eq!(a, b);
        assert_eq!(counter.name(), PROVIDER_TOKENIZER_NAME);
    }
}
