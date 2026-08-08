//! Tool wiring — Grok `ToolRegistryBuilder` → `FinalizedToolset`.
//!
//! The toolset is the Grok provider side of the LoopHost contract:
//! `tools_registry()` serves its definitions; `call_tool()` dispatches
//! through `FinalizedToolset::call`.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use orz_tools::computer::local::file_system::LocalFs;
use orz_tools::computer::local::terminal::LocalTerminalBackend;
use orz_tools::computer::types::{AsyncFileSystem, TerminalBackend};
use orz_tools::registry::types::{
    FinalizedToolset, SessionContext, ToolRegistryBuilder, ToolServerConfig,
};

/// Build a finalized toolset for a session working directory.
///
/// SessionContext is constructed with minimal-but-functional defaults:
/// local terminal backend, local fs rooted at `cwd`, noop notification
/// handle, all optional backends (memory/MCP/LSP/image/web) disabled.
pub fn build_toolset(cwd: &Path) -> Result<Arc<FinalizedToolset>, String> {
    let backend: Arc<dyn TerminalBackend> = Arc::new(LocalTerminalBackend::new());
    let fs: Arc<dyn AsyncFileSystem> = Arc::new(LocalFs);

    let ctx = SessionContext {
        backend,
        fs,
        cwd: cwd.to_path_buf(),
        session_folder: cwd.join(".gsa").join("session"),
        session_env: Arc::new(HashMap::new()),
        notification_handle: Default::default(),
        owner_session_id: None,
        subagent: None,
        parent_scheduler_handle: None,
        skills: Vec::new(),
        state_path: cwd.join(".gsa").join("state.json"),
        memory_backend: None,
        web_search_config: Default::default(),
        web_fetch_config: Default::default(),
        lsp: None,
        image_gen_config: Default::default(),
        video_gen_config: Default::default(),
        app_builder_deployer_config: Default::default(),
        api_key_provider: None,
        auth_provider: None,
        attribution_callback: None,
        system_reminder_tag: "system-reminder",
    };

    let builder = ToolRegistryBuilder::new();
    // finalize only enables the tools listed in the config. The builder
    // pre-registers multiple tool packs (GrokBuild / Codex / OpenCode / …)
    // whose default client names collide — enable the GrokBuild namespace only.
    let tools: Vec<_> = builder
        .known_tool_ids()
        .into_iter()
        .filter(|id| id.starts_with("GrokBuild:"))
        .map(|id| orz_tools::registry::types::ToolConfig {
            id,
            params: None,
            name_override: None,
            params_name_overrides: None,
            description_override: None,
            behavior_version: None,
            kind: None,
        })
        .collect();
    let config = ToolServerConfig {
        tools,
        behavior_preset: None,
    };
    builder
        .finalize(config, ctx)
        .map(Arc::new)
        .map_err(|e| format!("toolset finalize failed: {e:?}"))
}

/// Adapt `FinalizedToolset` definitions to the orz-loop `ToolRegistry` view.
pub struct ToolsetRegistry {
    toolset: Arc<FinalizedToolset>,
}

impl ToolsetRegistry {
    pub fn new(toolset: Arc<FinalizedToolset>) -> Self {
        Self { toolset }
    }

    pub fn toolset(&self) -> &Arc<FinalizedToolset> {
        &self.toolset
    }
}

impl orz_loop::host::ToolRegistry for ToolsetRegistry {
    fn get(&self, name: &str) -> Option<orz_loop::host::ToolDef> {
        self.toolset
            .tool_definitions()
            .into_iter()
            .find(|d| d.function.name == name)
            .map(|d| orz_loop::host::ToolDef {
                name: d.function.name,
                description: d.function.description.unwrap_or_default(),
                parameters: d.function.parameters,
            })
    }

    fn list(&self) -> Vec<orz_loop::host::ToolDef> {
        self.toolset
            .tool_definitions()
            .into_iter()
            .map(|d| orz_loop::host::ToolDef {
                name: d.function.name,
                description: d.function.description.unwrap_or_default(),
                parameters: d.function.parameters,
            })
            .collect()
    }
}

/// Map a `xai_tool_runtime::ToolError` to the orz-loop contract error.
pub fn map_tool_error(err: &xai_tool_runtime::ToolError) -> orz_loop::host::ToolError {
    // The runtime error carries a name/message; surface it as execution failure.
    orz_loop::host::ToolError::ExecutionFailed(err.to_string())
}

/// 2026-08-08 blackboard-partition review closure: derive the exit-code
/// semantics of a structured tool output for the orz-loop `ToolResult`
/// contract (the controller's edit-action gate keys on `Some(0)` = the edit
/// actually happened).
///
/// - `bash` carries its real exit code (the terminal-backend captures it).
/// - `search_replace` reports "applied" only via the `EditsApplied` variant;
///   every other variant (`NoMatchesFound`, `MultipleMatchesFound`,
///   `FileNotFound`, `InvalidInput`, `FileAlreadyExists`, `FilenameTooLong`)
///   is an Ok output that changed NOTHING — non-zero, so the "实际变动" gate
///   stays closed for them.
/// - every other successful output is `0`.
pub fn exit_code_from_output(output: &orz_tools::types::output::ToolOutput) -> Option<i32> {
    use orz_tools::types::output::SearchReplaceOutput;
    match output {
        orz_tools::types::output::ToolOutput::Bash(bash) => Some(bash.exit_code),
        orz_tools::types::output::ToolOutput::SearchReplace(sr) => {
            Some(match sr {
                SearchReplaceOutput::EditsApplied(_) => 0,
                _ => 1,
            })
        }
        _ => Some(0),
    }
}
