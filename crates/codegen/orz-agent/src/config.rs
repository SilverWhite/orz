//! Workspace toolset assembly (0ce 保留面).
//!
//! 0ce（2026-10-01）死代码清退后的唯一保留职责：`workspace_grok_build_toolset`
//! ——proxy 模式下 workspace server 可执行的全量工具集（消费方
//! `orz-workspace::handle`）。原 agent 定义机制（AgentDefinition/预设注册表/
//! `ORCHESTRATOR_PROMPT_BODY`/内置 agent 定义）零服务消费，已整体退役。

use orz_tools::implementations::{grok_build, memory, opencode, search_tool, use_tool};
use orz_tools::registry::types::{ToolConfig, ToolServerConfig};

/// Bash tool with clearer model-facing names:
/// `run_terminal_cmd` → `run_terminal_command`, `is_background` → `background`.
fn bash_tool_config() -> ToolConfig {
    ToolConfig::from(&grok_build::BashTool)
        .with_name("run_terminal_command")
        .with_param_rename("is_background", "background")
}
/// Task/subagent tool with clearer model-facing names:
/// `task` → `spawn_subagent`, `run_in_background` → `background`.
fn task_tool_config() -> ToolConfig {
    ToolConfig::from(&grok_build::TaskTool)
        .with_name("spawn_subagent")
        .with_param_rename("run_in_background", "background")
}
/// Task output tool renamed for clarity:
/// `get_task_output` → `get_command_or_subagent_output`.
fn task_output_tool_config() -> ToolConfig {
    ToolConfig::from(&grok_build::TaskOutputTool).with_name("get_command_or_subagent_output")
}
/// `wait_tasks` → `wait_commands_or_subagents`.
fn wait_tasks_tool_config() -> ToolConfig {
    ToolConfig::from(&grok_build::WaitTasksTool).with_name("wait_commands_or_subagents")
}
/// `kill_task` → `kill_command_or_subagent`.
fn kill_task_tool_config() -> ToolConfig {
    ToolConfig::from(&grok_build::KillTaskTool).with_name("kill_command_or_subagent")
}
/// Complete workspace-executable toolset for hub registration.
///
/// In proxy mode, the workspace server executes ALL tools — the shell has
/// zero local dispatch.
pub fn workspace_grok_build_toolset() -> ToolServerConfig {
    let mut tools = default_grok_build_toolset().tools;
    tools.push((&opencode::OpenCodeWriteTool).into());
    tools.push((&grok_build::EnterPlanModeTool).into());
    tools.push((&grok_build::ExitPlanModeTool).into());
    tools.push((&grok_build::AskUserQuestionTool).into());
    tools.push((&grok_build::WebSearchTool).into());
    tools.push((&grok_build::ImageGenTool).into());
    tools.push((&grok_build::ImageToVideoTool).into());
    tools.push((&grok_build::ReferenceToVideoTool).into());
    tools.push((&grok_build::WebFetchTool).into());
    tools.push((&memory::search_tool::MemorySearchImpl).into());
    tools.push((&memory::get_tool::MemoryGetImpl).into());
    tools.push((&grok_build::LspTool).into());
    ToolServerConfig {
        tools,
        behavior_preset: None,
    }
}
fn default_grok_build_toolset() -> ToolServerConfig {
    ToolServerConfig {
        tools: vec![
            bash_tool_config(),
            (&grok_build::ReadFileTool).into(),
            (&grok_build::SearchReplaceTool).into(),
            (&grok_build::ListDirTool).into(),
            (&grok_build::GrepTool).into(),
            kill_task_tool_config(),
            (&grok_build::TodoWriteTool).into(),
            task_output_tool_config(),
            wait_tasks_tool_config(),
            task_tool_config(),
            (&grok_build::SchedulerCreateTool).into(),
            (&grok_build::SchedulerDeleteTool).into(),
            (&grok_build::SchedulerListTool).into(),
            (&grok_build::MonitorTool).into(),
            (&search_tool::SearchTool).into(),
            (&use_tool::UseTool).into(),
            (&grok_build::UpdateGoalTool).into(),
            (&grok_build::WorkflowTool).into(),
        ],
        behavior_preset: None,
    }
}
