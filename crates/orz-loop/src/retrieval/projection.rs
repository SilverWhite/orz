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
        // P0-B 步骤 4（2026-08-14 用户裁决）+ 门禁观察 P1 修复（2026-08-30，
        // 方向 A）：browser_read 主面封存——主车道不执行检索任务（candidate
        // gate 对主车道 fetch_candidates=None fail-closed unbound），声明面
        // 保留会让模型看到"看得见摸不着"的工具（ADR v1.5 声明/执行不一致）。
        // 检索经 web 族外部子代理派发（framework_fallback）或终端内访问；
        // 外部检索 lane 的 browser_read 由 subagent_tool_projection 从 host
        // registry 独立恢复（本表不影响子代理面）。
        "browser_read",
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
    /// 检索工具面跟随模式 A 定档——local_browser 下隐藏 web 族，
    /// framework_fallback 下隐藏 browser_read（检索通道仅 web 族）。
    /// 主面 base 投影与子代理父面共用（子代理从父面继承，外部 lane 的
    /// browser_read 恢复另行按模式门控），避免「声明面同时出现两个通道、
    /// 调用期 mode 门拒绝其一」的假 available 形态（ADR v1.5：声明层与
    /// 执行层不一致对模型不可预测）。off 模式由既有检索族投影剔除
    /// （mode=off 无检索工具），本函数不重复处理。
    ///
    /// 门禁观察 P1 修复（2026-08-30，方向 A）：local_browser 分支的
    /// "保留 browser_read"不再构成主面检索通道——主面 browser_read 由
    /// [`R1_SEALED_MAIN_TOOLS`] 前置剔除（主车道不执行候选计数工具）；
    /// 子代理外部 lane 的 browser_read 经 `subagent_tool_projection`
    /// 从 registry 恢复。
    ///
    /// 门禁观察第四轮收尾（2026-08-30，用户裁决"先恢复外部"）：主面
    /// local_browser 保留 `web_search` **单一派发入口**——模型调用即
    /// 派发外部检索子代理（`relay::route` → ExternalRetrieval），执行
    /// 在子代理面完成；web_fetch 族与 web_search_* 变体不在主面出现
    /// （单入口语义，最小模型面变化）。执行面"二存一"由
    /// [`subagent_tool_projection`] 保证：外部 lane 在 local_browser
    /// 下剔除继承的 web 族、仅恢复 browser_read（引擎 SERP 通道），
    /// 原生 web_search 兜底是机械路径（`retrieval_mode_transition`，
    /// authority=mechanical_probe），不是子代理模型的自由选择。
    pub(crate) fn apply_retrieval_surface_projection(
        tool_defs: &mut Vec<ToolDef>,
        mode: RetrievalMode,
    ) {
        match mode {
            RetrievalMode::LocalBrowser => {
                // 保留裸 `web_search`（外部检索派发入口），剔除其余
                // web 族（web_fetch / web_fetch_* / web_search_*）。
                tool_defs.retain(|t| {
                    t.name == "web_search" || !crate::relay::is_web_retrieval_tool(&t.name)
                });
                // 入口标注（纯机械侧、仅主面）：web_search = 外部检索
                // 子代理派发入口，检索在子代理 lane 内执行、完成后返回。
                for t in tool_defs.iter_mut().filter(|t| t.name == "web_search") {
                    t.description.push_str(
                        " External retrieval entry: dispatches the external retrieval \
                         subagent; the search executes in the subagent lane and results \
                         return when it completes.",
                    );
                }
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
    /// RETRIEVAL-ORCHESTRATION-MECHANICAL 0k 第二批 (2026-08-30)：
    /// 委托契约复杂度分档——`effort` 提供 `max_results` 档位默认（模型未
    /// 显式传参时机械并入 goal 文本；显式传参优先）。
    pub(crate) fn build_retrieval_task_goal(
        arguments: &serde_json::Value,
        prompt: &str,
        effort: Option<crate::retrieval::effort::EffortTier>,
    ) -> String {
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
        let explicit_max = arguments.get("max_results").and_then(|m| m.as_u64());
        if let Some(max) = explicit_max.or_else(|| effort.map(|e| e.max_results_default())) {
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
                    // 门禁观察第四轮收尾（2026-08-30，用户裁决"先恢复
                    // 外部"）：外部 lane 执行面二存一——local_browser 下
                    // 仅 browser_read（引擎 SERP）为检索通道；主面
                    // web_search 是派发入口，不进入子代理执行面（原生
                    // web_search 兜底是机械路径 retrieval_mode_transition，
                    // 非子代理模型自由选择）。framework_fallback 下
                    // 外部 lane 维持 web 族（既有语义）。
                    && !(role == SubagentRole::ExternalRetrieval
                        && retrieval_mode == RetrievalMode::LocalBrowser
                        && crate::relay::is_web_retrieval_tool(&t.name))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller_test_support::*;
    use crate::gateway::fake::{FakeProvider, ScriptedResponse};
    use crate::gateway::model::ModelGateway;
    use crate::host::{LoopHost, PermitDecision, PermitError, RiskClass, ToolError, ToolResult};
    use async_trait::async_trait;
    use orz_assurance::{EventType, JournalRecorder};
    use std::sync::Arc;

    /// THIN-HARNESS-REDESIGN R1 (§4.1): 主代理工具面收敛为硬保留 8 工具
    /// （framework_fallback 检索模式下）——删除项（list_dir/run_tests/
    /// search_tool/project_doc_index/pdf_read/plan_write/
    /// retrieval_disposition/retrieve_project_docs）与边界三项
    /// （todo_write/update_goal/compaction_whitelist_add）一律不在首轮
    /// 声明面。
    struct R1SurfaceRegistry;
    impl ToolRegistry for R1SurfaceRegistry {
        fn get(&self, name: &str) -> Option<ToolDef> {
            Self::all().into_iter().find(|t| t.name == name)
        }
        fn list(&self) -> Vec<ToolDef> {
            Self::all()
        }
    }
    impl R1SurfaceRegistry {
        fn all() -> Vec<ToolDef> {
            [
                "run_terminal_cmd",
                "read_file",
                "grep",
                "search_replace",
                "web_search",
                "web_fetch",
            ]
            .iter()
            .map(|n| ToolDef {
                name: n.to_string(),
                description: format!("tool {n}"),
                parameters: serde_json::json!({}),
            })
            .collect()
        }
    }
    struct R1SurfaceHost {
        journal: JournalRecorder,
    }
    #[async_trait]
    impl LoopHost for R1SurfaceHost {
        fn journal(&self) -> &JournalRecorder {
            &self.journal
        }
        fn tools_registry(&self) -> &dyn ToolRegistry {
            &R1SurfaceRegistry
        }
        fn tool_policy(&self) -> crate::host::ToolPolicy {
            // Interactive：run_terminal_cmd 探针完整（Benchmark 会剔除）。
            crate::host::ToolPolicy::Interactive
        }
        fn terminal_available(&self) -> bool {
            true
        }
        fn session_cwd(&self) -> std::path::PathBuf {
            self.journal.journal_dir().to_path_buf()
        }
        async fn request_permission(
            &self,
            _risk: RiskClass,
            _tool: &str,
            _args: &serde_json::Value,
        ) -> Result<PermitDecision, PermitError> {
            Ok(PermitDecision::AllowOnce)
        }
        async fn call_tool(
            &self,
            _name: &str,
            _args: serde_json::Value,
            _call_id: &str,
        ) -> Result<ToolResult, ToolError> {
            Ok(ok_result())
        }
    }

    #[tokio::test]
    async fn r1_main_surface_is_eight_tools() {
        let dir = test_dir();
        let host = R1SurfaceHost {
            journal: JournalRecorder::new(dir.clone()),
        };

        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        // framework_fallback：web 族在面内（与生产 CLI 跑分一致）。
        // framework_fallback：web 族在面内（与生产 CLI 跑分一致）；
        // console_default 启用 submit（生产路径同款）。
        let controller = with_retrieval_enabled(
            AgentLoopController::with_gateway(gateway).with_console_default_enabled(true),
        );
        controller
            .run_turn(&host, "完成任务", "RUN-SURF", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        let events = events(&dir);
        let initial = events
            .iter()
            .find(|e| {
                e.event_type == EventType::RequestHeaderChange
                    && e.payload.get("reason").and_then(|v| v.as_str()) == Some("initial")
            })
            .expect("initial request header");
        let tools: Vec<String> = initial.payload["tools"]
            .as_array()
            .expect("tools list")
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        // 硬保留 8 工具（framework_fallback：web 族可见；browser_read 不
        // 在面内）。删除项与边界项一律不出现。
        let expected = [
            "run_terminal_cmd",
            "read_file",
            "grep",
            "search_replace",
            "web_search",
            "web_fetch",
            "blackboard_read",
            "submit",
        ];
        for tool in &expected {
            assert!(
                tools.iter().any(|t| t == tool),
                "retained tool '{tool}' must be declared: {tools:?}"
            );
        }
        for banned in [
            "list_dir",
            "run_tests",
            "search_tool",
            "project_doc_index",
            "pdf_read",
            "plan_write",
            "retrieval_disposition",
            "retrieve_project_docs",
            "todo_write",
            "update_goal",
            "compaction_whitelist_add",
            "browser_read",
        ] {
            assert!(
                !tools.iter().any(|t| t == banned),
                "banned tool '{banned}' must NOT be declared: {tools:?}"
            );
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 门禁观察第四轮收尾（2026-08-30，用户裁决"先恢复外部"）：local_browser
    /// 主面恢复 `web_search` 单一派发入口——web_fetch 隐藏、browser_read
    /// 保持 R1 封存；web_search 描述标注外部检索子代理派发语义（纯机械
    /// 侧，模型面仅此一项变化）。
    #[tokio::test]
    async fn local_browser_main_surface_restores_web_search_entry() {
        let dir = test_dir();
        let host = R1SurfaceHost {
            journal: JournalRecorder::new(dir.clone()),
        };
        let fake = Arc::new(FakeProvider::from_texts(vec!["完成", "完成"]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = with_local_browser_enabled(
            AgentLoopController::with_gateway(gateway).with_console_default_enabled(true),
        );
        controller
            .run_turn(&host, "完成任务", "RUN-LBS", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        let received = fake.received_requests();
        let declared: Vec<&str> = received[0].tools.iter().map(|t| t.name.as_str()).collect();
        for tool in [
            "read_file",
            "grep",
            "search_replace",
            "run_terminal_cmd",
            "web_search",
            "blackboard_read",
            "submit",
        ] {
            assert!(
                declared.contains(&tool),
                "tool '{tool}' must be declared under local_browser: {declared:?}"
            );
        }
        for banned in [
            "web_fetch",
            "browser_read",
            "retrieve_project_docs",
            "retrieval_disposition",
            "list_dir",
            "run_tests",
        ] {
            assert!(
                !declared.contains(&banned),
                "tool '{banned}' must NOT be declared under local_browser: {declared:?}"
            );
        }
        let ws = received[0]
            .tools
            .iter()
            .find(|t| t.name == "web_search")
            .expect("web_search declared");
        assert!(
            ws.description.contains("External retrieval entry"),
            "web_search must carry the external-subagent dispatch annotation: {}",
            ws.description
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A host over `MixedProjectionRegistry` with a configurable interactive
    /// signal and session cwd; no test runner (Benchmark policy).
    struct MixedProjectionHost {
        journal: JournalRecorder,
        interactive: bool,
        cwd: std::path::PathBuf,
    }
    #[async_trait]
    impl LoopHost for MixedProjectionHost {
        fn journal(&self) -> &JournalRecorder {
            &self.journal
        }
        fn tools_registry(&self) -> &dyn ToolRegistry {
            &MixedProjectionRegistry
        }
        fn tool_policy(&self) -> crate::host::ToolPolicy {
            crate::host::ToolPolicy::Benchmark
        }
        fn session_cwd(&self) -> std::path::PathBuf {
            self.cwd.clone()
        }
        fn interactive_user(&self) -> bool {
            self.interactive
        }
    }

    /// P0-A-2 (v0.2 single probe face): the model-visible list is
    /// 探针完整集 ∩ 会话声明集 + 非工作工具 — every work tool with an
    /// incomplete mechanical chain is removed even when the registry
    /// declares it (headless ask_user_question, Benchmark run_terminal_cmd,
    /// unconfigured image_gen, no-activation retrieval_disposition), and
    /// registry-absent tools are never invented.
    #[tokio::test]
    async fn list_projection_applies_face_partition() {
        let dir = test_dir();
        let host = MixedProjectionHost {
            journal: JournalRecorder::new(dir.clone()),
            interactive: false,
            cwd: dir.clone(),
        };
        let fake = Arc::new(FakeProvider::from_texts(vec!["完成", "完成"]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        controller
            .run_turn(&host, "hi", "RUN-PROJ", MANIFEST, 0, None, None, None)
            .await
            .unwrap();
        let received = fake.received_requests();
        let mut declared: Vec<&str> = received[0].tools.iter().map(|t| t.name.as_str()).collect();
        declared.sort();
        assert_eq!(
            declared,
            vec![
                "bash",            // non-work tool — untouched
                "blackboard_read", // storage chain complete
                "read_file",       // read chain complete
            ],
            "R1 single-face list projection (todo_write / compaction_whitelist_add \
             sealed): {declared:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 门禁观察 P1 修复（2026-08-30，方向 A）：主面封存 `browser_read`——
    /// P0-B 步骤 4（2026-08-14）"主 Agent 不执行检索任务、主车道投影移除
    /// browser_read"在 local_browser 主面上重新生效。relay::route 将
    /// browser_read 路由 Host 直执行、主车道 fetch_candidates=None（candidate
    /// gate fail-closed unbound），声明面保留会造成"看得见摸不着"的
    /// 声明/执行不一致（ADR v1.5 禁止形态；门禁观察实测
    /// browser_read_candidate_count_unbound 拒绝）。外部检索 lane 的
    /// browser_read 由 subagent_tool_projection 从 registry 恢复，不受影响。
    #[test]
    fn main_lane_projection_seals_browser_read() {
        let base = ["read_file", "browser_read", "bash"]
            .iter()
            .map(|n| ToolDef {
                name: n.to_string(),
                description: format!("tool {n}"),
                parameters: serde_json::json!({}),
            })
            .collect::<Vec<_>>();
        let snapshot = crate::tool_probe::ToolProbeSnapshot {
            complete: vec!["read_file".to_string()],
            incomplete: vec![],
        };
        let projected = AgentLoopController::project_main_agent_tool_defs(&base, &snapshot);
        let mut names: Vec<&str> = projected.iter().map(|t| t.name.as_str()).collect();
        names.sort();
        assert_eq!(
            names,
            vec!["bash", "read_file"],
            "browser_read sealed from the main-lane projection: {names:?}"
        );
    }

    /// FUS-RETRIEVAL-MECH P0-B 步骤 4 前置裁决（2026-08-14 用户裁决）：
    /// 检索子代理投影从 host registry 恢复主车道已移除的 `browser_read`
    /// （local_browser 模式由外部子代理执行；registry 未声明时不得发明），
    /// 并继续剔除主车道专属控制工具。RETRIEVAL-SUBAGENT-WIRING
    /// （2026-08-25）：内部 lane 仅读族——web 族与 browser_read 不进入
    /// 内部子代理工具面。
    #[test]
    fn subagent_projection_restores_browser_read() {
        let parent = [
            "read_file",
            "web_search",
            "compaction_whitelist_add",
            "retrieval_disposition",
            "blackboard_action_write",
            "plan_write",
            "retrieve_project_docs",
        ]
        .iter()
        .map(|n| ToolDef {
            name: n.to_string(),
            description: format!("tool {n}"),
            parameters: serde_json::json!({}),
        })
        .collect::<Vec<_>>();

        let projected = AgentLoopController::subagent_tool_projection(
            &parent,
            &BrowserDeclaringRegistry,
            SubagentRole::ExternalRetrieval,
            RetrievalMode::LocalBrowser,
        );
        let mut names: Vec<&str> = projected.iter().map(|t| t.name.as_str()).collect();
        names.sort();
        assert_eq!(
            names,
            vec!["browser_read", "read_file"],
            "subagent projection restores browser_read under local_browser, \
             strips inherited web_search (execution lane is browser_read-only \
             SERP), and strips console/main-only tools: {names:?}"
        );

        let absent = AgentLoopController::subagent_tool_projection(
            &parent,
            &EmptyRegistry,
            SubagentRole::ExternalRetrieval,
            RetrievalMode::LocalBrowser,
        );
        let mut absent_names: Vec<&str> = absent.iter().map(|t| t.name.as_str()).collect();
        absent_names.sort();
        assert_eq!(
            absent_names,
            vec!["read_file"],
            "no invention when the host registry lacks browser_read; inherited \
             web_search still stripped under local_browser: {absent_names:?}"
        );

        // RETRIEVAL-SUBAGENT-WIRING 审查处理：framework_fallback 下外部
        // lane 不恢复 browser_read（工具面跟随模式 A 定档——仅 web 族）。
        let fallback = AgentLoopController::subagent_tool_projection(
            &parent,
            &BrowserDeclaringRegistry,
            SubagentRole::ExternalRetrieval,
            RetrievalMode::FrameworkFallback,
        );
        let mut fallback_names: Vec<&str> = fallback.iter().map(|t| t.name.as_str()).collect();
        fallback_names.sort();
        assert_eq!(
            fallback_names,
            vec!["read_file", "web_search"],
            "no browser_read restore under framework_fallback: {fallback_names:?}"
        );
    }

    /// RETRIEVAL-SUBAGENT-WIRING (2026-08-25, ADR-0010 §14.40)：内部
    /// lane 工具面仅读族——web 族与 browser_read 被剔除，读族保留。
    #[test]
    fn internal_lane_projection_strips_web_and_browser() {
        let parent = [
            "read_file",
            "list_dir",
            "grep",
            "search_tool",
            "project_doc_index",
            "web_search",
            "web_fetch",
            "browser_read",
            "retrieve_project_docs",
        ]
        .iter()
        .map(|n| ToolDef {
            name: n.to_string(),
            description: format!("tool {n}"),
            parameters: serde_json::json!({}),
        })
        .collect::<Vec<_>>();
        let projected = AgentLoopController::subagent_tool_projection(
            &parent,
            &BrowserDeclaringRegistry,
            SubagentRole::InternalRetrieval,
            RetrievalMode::FrameworkFallback,
        );
        let mut names: Vec<&str> = projected.iter().map(|t| t.name.as_str()).collect();
        names.sort();
        assert_eq!(
            names,
            vec![
                "grep",
                "list_dir",
                "project_doc_index",
                "read_file",
                "search_tool"
            ],
            "internal lane is read-family only (no web/browser/retrieve dispatch): {names:?}"
        );
    }

    /// FUS-RETRIEVAL-MECH P0-B 步骤 4 前置裁决（2026-08-14 用户裁决）：
    /// 父侧投影已含 `browser_read` 时恢复逻辑不重复追加。
    #[test]
    fn subagent_projection_keeps_browser_read_singleton() {
        let parent = ["read_file", "browser_read"]
            .iter()
            .map(|n| ToolDef {
                name: n.to_string(),
                description: format!("tool {n}"),
                parameters: serde_json::json!({}),
            })
            .collect::<Vec<_>>();
        let projected = AgentLoopController::subagent_tool_projection(
            &parent,
            &BrowserDeclaringRegistry,
            SubagentRole::ExternalRetrieval,
            RetrievalMode::LocalBrowser,
        );
        assert_eq!(
            projected
                .iter()
                .filter(|t| t.name == "browser_read")
                .count(),
            1,
            "browser_read must appear exactly once: {projected:?}"
        );
    }

    /// RETRIEVAL-SUBAGENT-WIRING 审查处理 (2026-08-25)：检索工具面跟随
    /// 模式 A 定档——local_browser 隐藏 web 族（仅 browser_read 通道）、
    /// framework_fallback 隐藏 browser_read（仅 web 族通道）；off 由既有
    /// 检索族投影剔除，本函数不动（保持单一职责）。
    #[test]
    fn retrieval_surface_projection_follows_mode() {
        let defs = |names: &[&str]| -> Vec<ToolDef> {
            names
                .iter()
                .map(|n| ToolDef {
                    name: n.to_string(),
                    description: format!("tool {n}"),
                    parameters: serde_json::json!({}),
                })
                .collect()
        };
        let names = |v: &[ToolDef]| -> Vec<String> {
            let mut n: Vec<String> = v.iter().map(|t| t.name.clone()).collect();
            n.sort();
            n
        };

        // local_browser：web_fetch 族与 web_search 变体隐藏，裸 web_search
        // 保留为外部检索派发入口，browser_read 保留（R1 封存前置剔除由
        // project_main_agent_tool_defs 承担，本函数只管模式面）。
        let mut lb = defs(&[
            "web_search",
            "web_fetch",
            "web_search_x",
            "browser_read",
            "retrieve_project_docs",
            "read_file",
        ]);
        AgentLoopController::apply_retrieval_surface_projection(
            &mut lb,
            RetrievalMode::LocalBrowser,
        );
        assert_eq!(
            names(&lb),
            vec![
                "browser_read",
                "read_file",
                "retrieve_project_docs",
                "web_search"
            ],
            "local_browser keeps bare web_search as the dispatch entry and \
             hides web_fetch / web_search variants: {lb:?}"
        );
        // 入口标注：主面 web_search 描述声明外部检索子代理派发语义。
        let ws = lb
            .iter()
            .find(|t| t.name == "web_search")
            .expect("web_search");
        assert!(
            ws.description.contains("External retrieval entry")
                && ws.description.contains("external retrieval subagent"),
            "main-surface web_search must carry the external-subagent dispatch \
             annotation: {}",
            ws.description
        );

        // framework_fallback：browser_read 隐藏，web 族保留。
        let mut ff = defs(&[
            "web_search",
            "web_fetch",
            "browser_read",
            "retrieve_project_docs",
            "read_file",
        ]);
        AgentLoopController::apply_retrieval_surface_projection(
            &mut ff,
            RetrievalMode::FrameworkFallback,
        );
        assert_eq!(
            names(&ff),
            vec![
                "read_file",
                "retrieve_project_docs",
                "web_fetch",
                "web_search"
            ],
            "framework_fallback hides browser_read only: {ff:?}"
        );

        // off：本函数不动（既有检索族投影剔除是 off 的唯一职责面）。
        let mut off = defs(&["web_search", "browser_read"]);
        AgentLoopController::apply_retrieval_surface_projection(&mut off, RetrievalMode::Off);
        assert_eq!(
            names(&off),
            vec!["browser_read", "web_search"],
            "off is untouched by the mode-surface helper: {off:?}"
        );
    }

    /// RETRIEVAL-SUBAGENT-WIRING 审查处理 (2026-08-25)：检索任务契约
    /// 构建——query 必填（缺省回退 prompt）；scope/max_results 机械并入
    /// goal（子代理只收到 goal 文本，声明面承诺的参数必须进契约）。
    #[test]
    fn retrieval_task_goal_folds_scope_and_max_results() {
        let args = serde_json::json!({
            "query": "调研缓存层",
            "scope": "src/cache",
            "max_results": 5,
        });
        let goal = AgentLoopController::build_retrieval_task_goal(&args, "fallback prompt", None);
        assert_eq!(goal, "调研缓存层\nscope: src/cache\nmax_results: 5");

        // 仅 query（常见形态）→ 原样。
        let args = serde_json::json!({ "query": "调研缓存层" });
        let goal = AgentLoopController::build_retrieval_task_goal(&args, "fallback prompt", None);
        assert_eq!(goal, "调研缓存层");

        // query 缺失 → 回退 prompt；空 scope 不入契约。
        let args = serde_json::json!({ "scope": "   " });
        let goal = AgentLoopController::build_retrieval_task_goal(&args, "fallback prompt", None);
        assert_eq!(goal, "fallback prompt");

        // 非法 max_results（字符串）→ 忽略，不 panic。
        let args = serde_json::json!({ "query": "q", "max_results": "10" });
        let goal = AgentLoopController::build_retrieval_task_goal(&args, "fallback prompt", None);
        assert_eq!(goal, "q");

        // 第二批分档（2026-08-30）：模型未传 max_results → 档位默认并入；
        // 显式传参优先于档位默认。
        let args = serde_json::json!({ "query": "调研缓存层" });
        let goal = AgentLoopController::build_retrieval_task_goal(
            &args,
            "fallback prompt",
            Some(crate::retrieval::effort::EffortTier::Standard),
        );
        assert_eq!(goal, "调研缓存层\nmax_results: 5");
        let args = serde_json::json!({ "query": "q", "max_results": 3 });
        let goal = AgentLoopController::build_retrieval_task_goal(
            &args,
            "fallback prompt",
            Some(crate::retrieval::effort::EffortTier::Deep),
        );
        assert_eq!(goal, "q\nmax_results: 3");
    }

    /// A host registry that declares `browser_read` (local_browser capable).
    struct BrowserDeclaringRegistry;
    impl ToolRegistry for BrowserDeclaringRegistry {
        fn get(&self, name: &str) -> Option<ToolDef> {
            (name == "browser_read").then(|| ToolDef {
                name: "browser_read".to_string(),
                description: "tool browser_read".to_string(),
                parameters: serde_json::json!({}),
            })
        }
        fn list(&self) -> Vec<ToolDef> {
            vec![ToolDef {
                name: "browser_read".to_string(),
                description: "tool browser_read".to_string(),
                parameters: serde_json::json!({}),
            }]
        }
    }

    /// P0-A step 4: with an interactive session, a registry-declared
    /// `ask_user_question` survives the projection (probe complete).
    #[tokio::test]
    async fn list_projection_keeps_ask_user_question_when_interactive() {
        let dir = test_dir();
        let host = MixedProjectionHost {
            journal: JournalRecorder::new(dir.clone()),
            interactive: true,
            cwd: dir.clone(),
        };
        let fake = Arc::new(FakeProvider::from_texts(vec!["完成", "完成"]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        controller
            .run_turn(&host, "hi", "RUN-PROJ-INT", MANIFEST, 0, None, None, None)
            .await
            .unwrap();
        let received = fake.received_requests();
        let declared: Vec<&str> = received[0].tools.iter().map(|t| t.name.as_str()).collect();
        assert!(
            declared.iter().any(|t| *t == "ask_user_question"),
            "ask_user_question kept with an interactive session: {declared:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P0-A-2 review cleanup: with an unreadable session cwd every work
    /// tool's chain is incomplete (read/storage/write/goal chains all hang
    /// off the workspace) and only the non-work tool stays — the probe
    /// never invents tools and never keeps a broken mechanical chain
    /// visible.
    #[tokio::test]
    async fn list_projection_removes_unreadable_read_tools() {
        let dir = test_dir();
        let missing = std::env::temp_dir().join(format!(
            "orz-proj-missing-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&missing);
        let host = MixedProjectionHost {
            journal: JournalRecorder::new(dir.clone()),
            interactive: false,
            cwd: missing.clone(),
        };
        let fake = Arc::new(FakeProvider::from_texts(vec!["完成", "完成"]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        controller
            .run_turn(&host, "hi", "RUN-PROJ-RO", MANIFEST, 0, None, None, None)
            .await
            .unwrap();
        let received = fake.received_requests();
        let mut declared: Vec<&str> = received[0].tools.iter().map(|t| t.name.as_str()).collect();
        declared.sort();
        assert_eq!(
            declared,
            vec!["bash"],
            "unreadable workspace removes every work tool: {declared:?}"
        );
        let all_events = events(&dir);
        let availability = all_events
            .iter()
            .find(|e| e.event_type == EventType::ToolAvailabilityCheck)
            .unwrap();
        assert!(
            availability.payload["incomplete"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v["tool"] == "read_file" && v["reason"] == "工作区路径不可读"),
            "probe reasons: {:?}",
            availability.payload["incomplete"]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// THIN-HARNESS-REDESIGN R1 (§4.1)：主车道投影保留保留集内的检索
    /// 工具（web_search / web_fetch / browser_read —— 非工作工具不参与
    /// 探针过滤），且绝不发明已删除工具（retrieve_project_docs /
    /// list_dir / run_tests 等）。
    #[test]
    fn main_lane_projection_keeps_retained_retrieval_only() {
        let base = [
            "read_file",
            "web_search",
            "web_fetch",
            "browser_read",
            "bash",
        ]
        .iter()
        .map(|n| ToolDef {
            name: n.to_string(),
            description: format!("tool {n}"),
            parameters: serde_json::json!({}),
        })
        .collect::<Vec<_>>();
        let snapshot = crate::tool_probe::ToolProbeSnapshot {
            complete: vec!["read_file".to_string()],
            incomplete: vec![],
        };
        let projected = AgentLoopController::project_main_agent_tool_defs(&base, &snapshot);
        let names: Vec<&str> = projected.iter().map(|t| t.name.as_str()).collect();
        for tool in ["web_search", "web_fetch"] {
            assert!(
                names.contains(&tool),
                "retrieval tool {tool} must be declared on the direct surface: {names:?}"
            );
        }
        // 门禁观察 P1 修复（2026-08-30，方向 A）：browser_read 主面封存
        // （主车道不执行候选计数工具；外部 lane 从 registry 恢复）。
        assert!(
            !names.contains(&"browser_read"),
            "browser_read must NOT be declared on the main-lane surface: {names:?}"
        );
        for retired in [
            "retrieve_project_docs",
            "list_dir",
            "run_tests",
            "todo_write",
            "browser_read",
        ] {
            assert!(
                !names.contains(&retired),
                "retired tool {retired} must not be invented by the projection: {names:?}"
            );
        }
    }

    /// THIN-HARNESS-REDESIGN R1 (§4.1)：主面声明面不再包含
    /// `retrieve_project_docs` / `retrieval_disposition`（内部子代理与
    /// disposition 仪式退役）与边界三项（todo_write / update_goal /
    /// compaction_whitelist_add）——framework_fallback 与 mode=off 皆然；
    /// （正断言见 r1_main_surface_is_eight_tools——本测试只做负断言，
    /// TestHost 注册面为空。）
    #[tokio::test]
    async fn main_surface_hides_retired_and_sealed_tools() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: Some(ok_result()),
        };
        let fake = Arc::new(FakeProvider::from_texts(vec!["完成", "完成"]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(gateway));
        controller
            .run_turn(&host, "hi", "RUN-RPD-DECL", MANIFEST, 0, None, None, None)
            .await
            .unwrap();
        let received = fake.received_requests();
        let declared: Vec<&str> = received[0].tools.iter().map(|t| t.name.as_str()).collect();
        for hidden in [
            "retrieve_project_docs",
            "retrieval_disposition",
            "todo_write",
            "update_goal",
            "compaction_whitelist_add",
            "plan_write",
            "list_dir",
            "run_tests",
            "search_tool",
            "pdf_read",
            "browser_read",
        ] {
            assert!(
                !declared.contains(&hidden),
                "{hidden} must NOT be declared on the direct surface: {declared:?}"
            );
        }
        // mode=off（默认）：检索族整体剔除（含 web 族）。
        let dir2 = test_dir();
        let journal2 = JournalRecorder::new(dir2.clone());
        let host2 = TestHost {
            journal: journal2,
            tool_result: Some(ok_result()),
        };
        let fake2 = Arc::new(FakeProvider::from_texts(vec!["完成", "完成"]));
        let gateway2: Arc<dyn ModelGateway> = fake2.clone();
        let controller2 = AgentLoopController::with_gateway(gateway2);
        controller2
            .run_turn(&host2, "hi", "RUN-RPD-OFF", MANIFEST, 0, None, None, None)
            .await
            .unwrap();
        let received2 = fake2.received_requests();
        let declared2: Vec<&str> = received2[0].tools.iter().map(|t| t.name.as_str()).collect();
        assert!(
            !declared2.iter().any(|t| {
                *t == "retrieve_project_docs"
                    || *t == "retrieval_disposition"
                    || *t == "web_search"
                    || *t == "web_fetch"
            }),
            "retrieval family must be hidden under mode=off: {declared2:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&dir2);
    }
}
