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
    FinalizedToolset, SessionContext, ToolServerConfig, ToolRegistryBuilder,
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
