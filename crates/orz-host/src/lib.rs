//! orz-host — Thin core that bridges Grok providers to the self-built agent loop.
//!
//! Responsibilities:
//! 1. ACP JSON-RPC stdio server (session/new, session/prompt, list_tools, ...)
//! 2. LoopHost trait implementation — delegates to Grok provider crates
//! 3. Session lifecycle — workspace trust, security envelope, journal creation
//! 4. Interactive approval — bridges orz-workspace permission manager to ACP ApprovalRequest
//! 5. Journal writer — hash-chained JSONL event recording
//!
//! Dependency direction (Codex discipline):
//!   orz-host → {orz-loop, orz-assurance, orz-tools, orz-workspace, ...}
//!   No Grok crate depends on orz-host.
//!   orz-loop never imports orz-host — communicates through LoopHost trait.

pub mod acp_server;
pub mod session;
pub mod approval;

use std::sync::Arc;

pub struct OrzHost {
    // Agent loop controller (self-built)
    pub loop_controller: orz_loop::AgentLoopController,

    // Journal recorder (hash-chained JSONL)
    pub journal: orz_assurance::journal::JournalRecorder,

    // Grok provider handles
    pub tools: Arc<orz_tools::ToolRegistry>,
    pub workspace: Arc<orz_workspace::Workspace>,
    pub sandbox: Arc<orz_sandbox::Sandbox>,
    pub mcp_manager: Arc<orz_mcp::McpManager>,
    pub chat_state: Arc<orz_chat_state::ChatState>,
    pub hooks: Arc<orz_hooks::HookSystem>,
    pub auth: Arc<orz_auth::AuthProvider>,
    pub secrets: Arc<orz_secrets::SecretsManager>,
    pub memory: Arc<orz_memory::MemoryManager>,
    pub config: Arc<orz_config::Config>,
}
