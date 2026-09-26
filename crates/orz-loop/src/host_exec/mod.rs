//! Host 工具执行家族（0ai 2026-09-16 拆分自单文件 `host_exec.rs`）。
//!
//! 机械搬移：代码逐行搬移、按需升 `pub(crate)`；事件序列 / journal 链 / 行为不变。
//!
//! | 模块 | 职责域 |
//! |---|---|
//! | [`facts`] | 宿主事实 → journal 事件（browser launch / 进程树回收 / 宿主资源事实 / idle kill） |
//! | [`failure`] | 失败信封与漏斗（`ToolFailureOutcome` / 失败聚合 / LIF deny 通道 / 订单失败回写） |
//! | [`serp`] | SERP 预算结算与取证（预算码 / 导航计数 / attempt 落盘 / 拒绝路径） |
//! | [`candidate`] | 候选计数门（`candidate_gate` / `refuse_candidate`） |
//! | [`dep_graph`] | 依赖图事实（P2-11 锚点链上报） |
//! | [`tool_run`] | host 工具执行编排三件套（`run_host_tool` / plan gate / 超时主循环） |
//!
//! `crate::host_exec::X` 路径保持不变（见下方 re-export）。

mod candidate;
mod dep_graph;
mod facts;
mod failure;
mod serp;
mod tool_run;

pub(crate) use failure::ToolFailureOutcome;
pub(crate) use serp::{
    browser_serp_navigations_from_structured, is_serp_search_call, serp_navigations_from_output,
};
