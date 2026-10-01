//! Skills/plugins/agents-md discovery surfaces and workspace toolset
//! assembly, shared with `orz-workspace`.
//!
//! 0ce (2026-10-01) 死代码清退后的保留面：本 crate 曾承载 Agent 构建、
//! 模板渲染与 agent 定义机制（Agent/AgentBuilder/AgentDefinition/
//! PromptContext/加密模板族）——服务消费面为零，已整体退役；在役消费面
//! 仅 plugins / skills / agents_md / repo / 目录发现 / 工具集装配六面
//! （消费方 orz-workspace：folder_trust、project_config、discovery、handle）。
//! TB 无头路径零提示词姿态不变（0cd 已钉）。

pub mod config;
pub mod discovery;
pub mod plugins;
pub mod prompt;
pub mod repo;
pub mod timing;

pub use config::workspace_grok_build_toolset;
