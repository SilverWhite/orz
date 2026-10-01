//! AGENTS.md、skills 与 workspace user 目录的发现/解析面。
//!
//! 0ce（2026-10-01）：system prompt 组装模板族（context/template/
//! subagent_prompts/user_message）随死代码清退退役；本模块只保留在役
//! 消费面（agents_md / skills，消费方 orz-workspace::discovery）。

pub mod agents_md;
pub mod ignore;
pub mod skills;
pub mod workspace_user;
