//! Model configuration types.

/// Configuration for a single model provider endpoint.
#[derive(Debug, Clone)]
pub struct ModelConfig {
    pub provider: String,
    pub model_id: String,
    pub api_base: String,
    pub api_key: String,
    pub max_tokens: u32,
}
