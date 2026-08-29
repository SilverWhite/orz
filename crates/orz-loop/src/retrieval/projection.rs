//! Retrieval tool-surface projection helpers — batch B1 of the controller
//! split (CONTROLLER_SPLIT_DESIGN_2026-08-29).
//!
//! Moved verbatim from `controller.rs`; pure functions, no state access.

use crate::agents::SubagentRole;
use crate::controller::{AgentLoopController, RetrievalMode};
use crate::host::{ToolDef, ToolRegistry};

impl AgentLoopController {
    /// P0-A-2 (design §4 v0.2): rebuild the model-visible list projection
    /// from the base registry list + the CURRENT probe snapshot:
    /// 探针完整集 ∩ 会话声明集 + 非工作工具 — names only, no status
    /// annotations. Faces A/C are revoked: every work tool is removed
    /// unless its probe is complete, even when the registry declares it.
    /// THIN-HARNESS-REDESIGN R1 (2026-08-27, §4.1): `run_tests` 从主代理
    /// 声明面删除（bash 可达；官方验证独立于 agent），不再按探针补充。
    /// 其余工作工具仍按探针完整集 ∩ 会话声明集投影；registry-absent tools
    /// 永不发明。
    ///
    /// R1 封存/删除工具：无论 registry/探针如何声明，主代理面一律不出现
    /// （接口封存——代码模块保留，从本表移除名称即配置恢复；plan_write
    /// 刻意不在本表——休眠 plan_first 门路径启用时仍需声明）。
    pub(crate) const R1_SEALED_MAIN_TOOLS: &[&str] = &[
        // 边界三项（§4.1 边界项）。
        "todo_write",
        "update_goal",
        "compaction_whitelist_add",
        // 删除项（§4.1 删除表）。
        "list_dir",
        "run_tests",
        "search_tool",
        "project_doc_index",
        "pdf_read",
        "retrieval_disposition",
        "retrieve_project_docs",
    ];

    pub(crate) fn project_main_agent_tool_defs(
        base: &[ToolDef],
        snapshot: &crate::tool_probe::ToolProbeSnapshot,
    ) -> Vec<ToolDef> {
        let mut tool_defs = base.to_vec();
        tool_defs.retain(|t| !Self::R1_SEALED_MAIN_TOOLS.contains(&t.name.as_str()));
        tool_defs.retain(|t| {
            !crate::tool_probe::is_main_agent_work_tool(&t.name)
                || snapshot.complete.iter().any(|c| c == &t.name)
        });
        tool_defs
    }

    /// RETRIEVAL-SUBAGENT-WIRING 审查处理 (2026-08-25, ADR-0010 §14.40)：
    /// 检索工具面跟随模式 A 定档——local_browser 下隐藏 web 族（检索通道
    /// 仅 browser_read），framework_fallback 下隐藏 browser_read（检索
    /// 通道仅 web 族）。主面 base 投影与子代理父面共用（子代理从父面继承，
    /// 外部 lane 的 browser_read 恢复另行按模式门控），避免「声明面同时
    /// 出现两个通道、调用期 mode 门拒绝其一」的假 available 形态
    /// （ADR v1.5：声明层与执行层不一致对模型不可预测）。off 模式由既有
    /// 检索族投影剔除（mode=off 无检索工具），本函数不重复处理。
    pub(crate) fn apply_retrieval_surface_projection(
        tool_defs: &mut Vec<ToolDef>,
        mode: RetrievalMode,
    ) {
        match mode {
            RetrievalMode::LocalBrowser => {
                tool_defs.retain(|t| !crate::relay::is_web_retrieval_tool(&t.name));
            }
            RetrievalMode::FrameworkFallback => {
                tool_defs.retain(|t| t.name != "browser_read");
            }
            RetrievalMode::Off => {}
        }
    }

    /// RETRIEVAL-SUBAGENT-WIRING 审查处理 (2026-08-25)：检索任务契约构建
    /// ——`query` 必填（缺省回退 prompt），可选 `scope`/`max_results` 机械
    /// 并入 goal 文本。子代理只收到 goal（`SystemPromptKind::Retrieval`），
    /// 声明面承诺的参数若不入契约会被静默丢弃——并入后语义与 ToolDef
    /// 描述一致（scope 收窄检索范围、max_results 上限结果数）。
    pub(crate) fn build_retrieval_task_goal(arguments: &serde_json::Value, prompt: &str) -> String {
        let mut goal = arguments
            .get("query")
            .and_then(|q| q.as_str())
            .unwrap_or(prompt)
            .to_string();
        if let Some(scope) = arguments
            .get("scope")
            .and_then(|s| s.as_str())
            .filter(|s| !s.trim().is_empty())
        {
            goal.push_str("\nscope: ");
            goal.push_str(scope.trim());
        }
        if let Some(max) = arguments.get("max_results").and_then(|m| m.as_u64()) {
            goal.push_str(&format!("\nmax_results: {max}"));
        }
        goal
    }

    /// FUS-RETRIEVAL-MECH P0-B 步骤 4 前置裁决（2026-08-14 用户裁决）：
    /// 子代理工具投影 = 父侧 registry 投影去掉主车道专属控制工具
    /// （`compaction_whitelist_add` / `retrieval_disposition`），并恢复主车道
    /// 已不广告但检索车道仍须可用的 host 路由检索工具（`browser_read`——
    /// local_browser 模式由外部子代理执行；host registry 未声明时不得发明）。
    /// RETRIEVAL-SUBAGENT-WIRING (2026-08-25, ADR-0010 §14.40)：内部
    /// lane（InternalRetrieval）工具面仅读族——web 族（web_search/web_fetch
    /// 及变体）与 browser_read 不进入内部 lane（内部检索对象=工作区/项目
    /// 文档，不是外部网络）；外部 lane（ExternalRetrieval）维持 web 族 +
    /// browser_read（可用时）。审查处理 (2026-08-25)：恢复动作按模式 A
    /// 定档门控——仅 local_browser 模式恢复 browser_read（framework_fallback
    /// 下外部 lane 只走 web 族，与 DoD「工具面跟随模式」一致；web 族在
    /// local_browser 下由 `apply_retrieval_surface_projection` 从父面剔除）。
    pub(crate) fn subagent_tool_projection(
        parent_tools: &[ToolDef],
        registry: &dyn ToolRegistry,
        role: SubagentRole,
        retrieval_mode: RetrievalMode,
    ) -> Vec<ToolDef> {
        let mut defs: Vec<ToolDef> = parent_tools
            .iter()
            .filter(|t| {
                t.name != "compaction_whitelist_add"
                    && t.name != "retrieval_disposition"
                    // P0-C S2 (2026-08-15): the console write button is
                    // main-lane only — subagents never write action orders.
                    && t.name != "blackboard_action_write"
                    // PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): the
                    // plan-gate write surface is main-lane only — subagents
                    // never write plans (P2-1 审查收口).
                    && t.name != "plan_write"
                    // PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱):
                    // console 双模式控制工具 main-lane only — subagents 无
                    // 操作台、不参与 direct 证据门。
                    && t.name != "console_step_done"
                    && t.name != "console_return_to_console"
                    // RETRIEVAL-SUBAGENT-WIRING：内部 lane 不暴露外部
                    // 检索族（web_* 与 browser_read）；外部 lane 保留。
                    && !(role == SubagentRole::InternalRetrieval
                        && (crate::relay::is_web_retrieval_tool(&t.name)
                            || t.name == "browser_read"))
                    // 内部检索派发工具（retrieve_project_*）是主车道专属
                    // 入口——子代理不派发内部检索（nested-dispatch gate
                    // 兜底外，声明面也剔除，杜绝递归/自我派发）。web 族
                    // 保留（外部 lane 自执行）。
                    && !t.name.starts_with("retrieve_project_")
            })
            .cloned()
            .collect();
        // 外部 lane 恢复主车道已不广告的 host 路由检索工具
        // （browser_read——local_browser 模式由外部子代理执行）；仅
        // local_browser 模式恢复（framework_fallback 下外部 lane 只走
        // web 族）；内部 lane 不恢复（工具面仅读族）。
        if role == SubagentRole::ExternalRetrieval && retrieval_mode == RetrievalMode::LocalBrowser
        {
            for name in ["browser_read"] {
                if !defs.iter().any(|t| t.name == name) {
                    if let Some(def) = registry.get(name) {
                        defs.push(def);
                    }
                }
            }
        }
        defs
    }
}
