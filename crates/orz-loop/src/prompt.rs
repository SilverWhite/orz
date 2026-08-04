//! PromptBuilder — assembles system prompt + assurance context blocks.
//!
//! IP2a: TOOL_AVAILABILITY block injection (format aligned with Python
//! `_build_context_block`).
//! IP2b: ORIENTATION_CHECKPOINT block (injected per-turn by the controller via
//! the orientation monitor; the builder provides the composition helper).
//! IP2c: INFO_SUFFICIENCY — reserved; retrieval sufficiency triggers land in
//! the ToolDispatcher (Phase 2 scope keeps the trigger mechanical).

/// Base system prompt — runtime-neutral orientation.
pub const BASE_SYSTEM_PROMPT: &str = "你是 orz——保证优先的 CLI agent workbench。\
遵循注入的 assurance 上下文块执行任务；工具可用性由运行时声明，不得自行推断。";

/// Tool availability context block format — aligned with Python
/// `_build_context_block` (`[TOOL_AVAILABILITY v0.1]` ... `[/TOOL_AVAILABILITY]`).
pub fn build_tool_availability_block(
    available: &[String],
    unavailable: &[String],
    degraded: &[String],
    unprobed: &[String],
) -> String {
    // Sorted per category, matching Python's `sorted(...)` joins.
    let mut available = available.to_vec();
    let mut unavailable = unavailable.to_vec();
    let mut degraded = degraded.to_vec();
    let mut unprobed = unprobed.to_vec();
    available.sort();
    unavailable.sort();
    degraded.sort();
    unprobed.sort();

    let mut lines = vec!["[TOOL_AVAILABILITY v0.1]".to_string()];
    if !available.is_empty() {
        lines.push(format!("AVAILABLE: {}", available.join(", ")));
    }
    if !unavailable.is_empty() {
        lines.push(format!("UNAVAILABLE: {}", unavailable.join(", ")));
    }
    if !degraded.is_empty() {
        lines.push(format!("DEGRADED: {}", degraded.join(", ")));
    }
    if !unprobed.is_empty() {
        lines.push(format!("UNPROBED: {}", unprobed.join(", ")));
    }
    lines.push(String::new());
    lines.push("GATE: runtime_probe_authoritative".to_string());
    lines.push("STATUS: descriptive_projection_only".to_string());
    lines.push(
        "ENFORCEMENT: tool calls, provider errors, and capability claims are \
checked by runtime gates and observers outside this text block."
            .to_string(),
    );
    lines.push("[/TOOL_AVAILABILITY]".to_string());
    lines.join("\n")
}

/// Builds prompts with injected assurance context.
#[derive(Debug, Clone, Default)]
pub struct PromptBuilder;

impl PromptBuilder {
    pub fn new() -> Self {
        PromptBuilder
    }

    /// IP2a: assemble the system prompt with the tool availability block
    /// injected (when present) above the base instructions.
    pub fn build_system_prompt(&self, tool_availability_block: Option<&str>) -> String {
        match tool_availability_block {
            Some(block) => format!("{block}\n\n{BASE_SYSTEM_PROMPT}"),
            None => BASE_SYSTEM_PROMPT.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_availability_block_format_matches_python() {
        let block = build_tool_availability_block(
            &["read_file".to_string(), "bash".to_string()],
            &[],
            &["web_search".to_string()],
            &[],
        );
        assert!(block.starts_with("[TOOL_AVAILABILITY v0.1]"));
        assert!(block.ends_with("[/TOOL_AVAILABILITY]"));
        assert!(block.contains("AVAILABLE: bash, read_file"));
        assert!(block.contains("DEGRADED: web_search"));
        assert!(block.contains("GATE: runtime_probe_authoritative"));
        assert!(!block.contains("UNAVAILABLE:"));
    }

    #[test]
    fn system_prompt_injects_block() {
        let builder = PromptBuilder::new();
        let block = build_tool_availability_block(&[], &[], &[], &["grep".to_string()]);
        let prompt = builder.build_system_prompt(Some(&block));
        assert!(prompt.contains("[TOOL_AVAILABILITY v0.1]"));
        assert!(prompt.contains(BASE_SYSTEM_PROMPT));
        let bare = builder.build_system_prompt(None);
        assert_eq!(bare, BASE_SYSTEM_PROMPT);
    }
}
