//! PromptBuilder — assembles system prompt + tool definitions + injection points.
//!
//! IP2a: TOOL_AVAILABILITY injection
//! IP2b: ORIENTATION_CHECKPOINT placeholder
//! IP2c: INFO_SUFFICIENCY placeholder
//!
//! Phase 1: skeleton only. Full implementation in Step 3.

/// Builds prompts with injected assurance context.
pub struct PromptBuilder;

impl PromptBuilder {
    pub fn new() -> Self {
        PromptBuilder
    }
}

impl Default for PromptBuilder {
    fn default() -> Self {
        Self::new()
    }
}
