//! Retrieval tool-surface projection helpers — batch B1 of the controller
//! split (CONTROLLER_SPLIT_DESIGN_2026-08-29).
//!
//! Moved verbatim from `controller.rs`; pure functions, no state access.

use crate::agents::SubagentRole;
use crate::controller::AgentLoopController;
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
        // 0t S2-R P3 / P1-2b：browser_control 同属本地浏览器车道，主面
        // 封存（外部检索 lane 由 subagent_tool_projection 恢复）。
        "browser_control",
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

    /// 0t (2026-09-09, ADR-0010 §14.65 / 设计 §3.1/§3.2, v1.3): 主面检索
    /// 工具面投影——模式分支退役后由**启用门**单参数决定。启用会话主面
    /// 恒以现行 local_browser 形态投影：保留裸 `web_search` **单一派发
    /// 入口**（模型调用即派发外部检索子代理），剔除其余 web 族
    /// （web_fetch / web_search_* 变体），`browser_read` 由
    /// [`R1_SEALED_MAIN_TOOLS`] 前置剔除（主车道不执行候选计数工具）；
    /// 入口标注（纯机械侧英文句）沿既有语义保留。未启用会话的检索族已在
    /// 调用方（run-start 投影）剔除，本函数不做重复处理。
    pub(crate) fn apply_retrieval_surface_projection(
        tool_defs: &mut Vec<ToolDef>,
        retrieval_enabled: bool,
    ) {
        if !retrieval_enabled {
            return;
        }
        // 保留裸 `web_search`（外部检索派发入口），剔除其余 web 族
        // （web_fetch / web_fetch_* / web_search_*）——单入口语义不变。
        tool_defs
            .retain(|t| t.name == "web_search" || !crate::relay::is_web_retrieval_tool(&t.name));
        // 入口标注（纯机械侧、仅主面）：web_search = 外部检索子代理派发
        // 入口，检索在子代理 lane 内执行、完成后返回。
        for t in tool_defs.iter_mut().filter(|t| t.name == "web_search") {
            t.description.push_str(
                " External retrieval entry: dispatches the external retrieval \
                 subagent; the search executes in the subagent lane and results \
                 return when it completes.",
            );
        }
    }

    /// RET-B3（2026-09-15）：search_engine 探针 detail 单一源化。快照命中
    /// 时读数形如
    /// `web_search local_segmented=on chain_detail="bing_cn,…; proxy=on host:port"`
    /// （`chain_detail` = orz-tools `engine_chain_detail()`，已含引擎链与
    /// proxy=on|off + 脱敏端点）；快照为 None（未装配）时回退既有读 env
    /// 逻辑、保持原 detail 格式。present/missing 语义不受影响。
    pub(crate) fn search_engine_probe_detail(
        name: &str,
        assembled: Option<
            &orz_tools::implementations::web_search::local_segmented::LocalSegmentedConfig,
        >,
    ) -> String {
        if let Some(config) = assembled {
            return format!(
                "{name} local_segmented={} chain_detail=\"{}\"",
                if config.is_enabled() { "on" } else { "off" },
                config.engine_chain_detail(),
            );
        }
        // 回退（未装配）：既有读 env 逻辑原样保留。
        let engines = std::env::var("ORZ_RETRIEVAL_ENGINES")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "builtin".to_string());
        let local = std::env::var("ORZ_WEB_SEARCH_LOCAL")
            .map(|v| {
                matches!(
                    v.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "on" | "yes"
                )
            })
            .unwrap_or(false);
        format!("{name} engines={engines} local_segmented={local}")
    }

    /// 0ac S3①（2026-09-13，设计稿 §9 / §10.2）：检索族探针读数——与 23 工作
    /// 工具面探针**各自成事件**（`probe_scope="retrieval_family"`），run-start
    /// 恰好一次、先于 `run_started`。
    ///
    /// 三个成员（browser / search_engine / web_channel）取**registry 声明面**
    /// 读数（未投影的 base 列表——检索 lane 面的 `browser_read` 在主面被 R1
    /// 封存，主面列表会把「子代理看得到」读成 absent，故不取投影后列表）：
    /// * browser——`browser_read` / `local_browser` 任一在册即为 present；
    /// * search_engine——`web_search` 在册 + 本地分段检索族的引擎链/开关/
    ///   代理读数。RET-B3（2026-09-15）单一源化：优先消费 orz-tools 装配期
    ///   快照 `assembled_local_segmented()`（detail 带引擎链 +
    ///   `proxy=on|off` + 脱敏端点，不再另行重读 env，防漂移）；未装配
    ///   （registry 未建）时回退既有读 env 逻辑、保持原 detail 格式；
    /// * web_channel——`web_fetch` 族（本地 HTTP 通道）在册。
    ///
    /// 检索族对本 run 关闭（`retrieval_enabled=false`）时三个成员一律读
    /// absent + 中性原因（家族不在本 run 的声明面上——读数只报事实）。
    /// `gate_decision` 恒为 `"pass"`：探针不阻断（设计不变式 2）。
    ///
    /// F-007 口径裁决 (a) 宽松口径（2026-09-13）：探针缺失不构成违规，法官
    /// 仅在探针存在时校验其内容
    /// （`orz-assurance::journal::immediate_feedback::verify_retrieval_family_probe`）。
    pub(crate) fn retrieval_family_payload(
        base: &[ToolDef],
        retrieval_enabled: bool,
    ) -> serde_json::Value {
        fn declared<'a>(base: &[ToolDef], names: &[&'a str]) -> Option<&'a str> {
            names
                .iter()
                .copied()
                .find(|name| base.iter().any(|t| t.name == *name))
        }
        let disabled = || {
            serde_json::json!({
                "present": false,
                "reason": "retrieval family disabled for this run (retrieval_enabled=false)",
            })
        };
        let missing = |reason: &str| serde_json::json!({ "present": false, "reason": reason });

        let browser = if !retrieval_enabled {
            disabled()
        } else {
            match declared(base, &["browser_read", "local_browser"]) {
                Some(name) => serde_json::json!({ "present": true, "detail": name }),
                None => missing("no local browser tool in this build's registry"),
            }
        };
        let search_engine = if !retrieval_enabled {
            disabled()
        } else {
            match declared(base, &["web_search"]) {
                Some(name) => {
                    // RET-B3：单一源——优先消费 orz-tools 装配期快照（链 +
                    // proxy 读数同源）；未装配时回退读 env（原格式）。
                    let detail = Self::search_engine_probe_detail(
                        name,
                        orz_tools::implementations::web_search::local_segmented::assembled_local_segmented(
                        ),
                    );
                    serde_json::json!({
                        "present": true,
                        "detail": detail,
                    })
                }
                None => missing("web_search not declared by this build's registry"),
            }
        };
        let web_channel = if !retrieval_enabled {
            disabled()
        } else {
            match declared(base, &["web_fetch", "web_fetch_url", "read_url", "fetch"]) {
                Some(name) => serde_json::json!({ "present": true, "detail": name }),
                None => missing("no local HTTP channel tool in this build's registry"),
            }
        };

        let mut complete: Vec<&str> = Vec::new();
        let mut incomplete: Vec<serde_json::Value> = Vec::new();
        for (member, reading) in [
            ("browser", &browser),
            ("search_engine", &search_engine),
            ("web_channel", &web_channel),
        ] {
            if reading.get("present").and_then(serde_json::Value::as_bool) == Some(true) {
                complete.push(member);
            } else {
                incomplete.push(serde_json::json!({
                    "tool": member,
                    "reason": reading
                        .get("reason")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("absent"),
                }));
            }
        }
        serde_json::json!({
            "probe_scope": "retrieval_family",
            "probe_timestamp": crate::controller::chrono_utc_now(),
            "retrieval_enabled": retrieval_enabled,
            "complete": complete,
            "incomplete": incomplete,
            "gate_decision": "pass",
            "retrieval_family": {
                "browser": browser,
                "search_engine": search_engine,
                "web_channel": web_channel,
            },
        })
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
    /// 已不广告但检索车道仍须可用的 host 路由检索工具（`browser_read`；
    /// host registry 未声明时不得发明）。
    /// RETRIEVAL-SUBAGENT-WIRING (2026-08-25, ADR-0010 §14.40)：内部
    /// lane（InternalRetrieval）工具面仅读族——web 族（web_search/web_fetch
    /// 及变体）与 browser_read 不进入内部 lane（内部检索对象=工作区/项目
    /// 文档，不是外部网络）。
    /// 0t (2026-09-09, ADR-0010 §14.65 / 设计 §3.1, v1.3)：外部 lane
    /// （ExternalRetrieval）在检索启用会话**双族恒在**——web 族 +
    /// browser_read 并存、换道由模型自主（γ 退役「二存一」模式裁剪与机械
    /// 降级；`framework_fallback`/`local_browser` 值不再进入本函数）。
    /// S2-R P3 / P2-2 + P1-2b（2026-09-09）：外部 lane 声明集 =
    /// 浏览器车道（browser_read + browser_control）+ 原生车道（web_search
    /// + web_fetch）——web_fetch 随主面隐藏后从 registry 恢复（与设计
    /// §3.1 双族措辞及激活提示的 web_fetch 验证句一致）。
    /// 未启用会话任何 role 均无检索工具（fail-closed belt and braces）。
    /// 静态车道标注（设计 §3.2 固化措辞）在此落点：browser_read /
    /// browser_control 携带 `[车道:本地浏览器检索|推荐首选]`、web_search /
    /// web_fetch 携带 `[车道:原生检索]`——只进一次性子代理工具声明，无
    /// 新增常驻 token。
    pub(crate) fn subagent_tool_projection(
        parent_tools: &[ToolDef],
        registry: &dyn ToolRegistry,
        role: SubagentRole,
        retrieval_enabled: bool,
    ) -> Vec<ToolDef> {
        let mut defs: Vec<ToolDef> = parent_tools
            .iter()
            .filter(|t| {
                t.name != "compaction_whitelist_add"
                    && t.name != "retrieval_disposition"
                    // 0ap（2026-09-18，设计 §0「常驻主面」）：压缩交互第九
                    // 工具 main-lane only——D3 压缩窗口是主车道结构（DP-7；
                    // H1 开窗点与请求位消费点均 gate Main role），且 controller
                    // 请求位跨车道共享，子代理声明会在主车道误触发开窗。
                    && t.name != orz_assurance::tool_names::CONTEXT_COMPRESS_TOOL_NAME
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
                    // 检索族（web_* 与 browser_read / browser_control）；
                    // 外部 lane 保留。
                    && !(role == SubagentRole::InternalRetrieval
                        && (crate::relay::is_web_retrieval_tool(&t.name)
                            || t.name == "browser_read"
                            || t.name == "browser_control"))
                    // 0t: 检索未启用会话对任何 role 剔除全部检索工具
                    // （retrieval dispatch 族 + host 路由检索工具）。
                    && !(!retrieval_enabled
                        && (crate::relay::is_retrieval_dispatch_name(&t.name)
                            || crate::relay::is_retrieval_mode_gated_host_tool(
                                &t.name,
                            )))
                    // 内部检索派发工具（retrieve_project_*）是主车道专属
                    // 入口——子代理不派发内部检索（nested-dispatch gate
                    // 兜底外，声明面也剔除，杜绝递归/自我派发）。web 族
                    // 保留（外部 lane 自执行）。
                    && !t.name.starts_with("retrieve_project_")
            })
            .cloned()
            .collect();
        // 外部 lane（仅检索启用）恢复主车道已不广告的 host 路由检索工具
        // （browser_read / browser_control / web_fetch）；内部 lane 不恢复
        // （工具面仅读族）。registry 缺 def 时不发明（EmptyRegistry 语义
        // 不变）。恢复后追加静态车道标注（§3.2 固化措辞；两族并存为常态，
        // 非互斥）。
        if role == SubagentRole::ExternalRetrieval && retrieval_enabled {
            for name in ["browser_read", "browser_control", "web_fetch"] {
                if !defs.iter().any(|t| t.name == name) {
                    if let Some(def) = registry.get(name) {
                        defs.push(def);
                    }
                }
            }
            for t in defs.iter_mut() {
                if matches!(t.name.as_str(), "browser_read" | "browser_control")
                    && !t.description.contains("[车道:")
                {
                    t.description.push_str(" [车道:本地浏览器检索|推荐首选]");
                } else if matches!(t.name.as_str(), "web_search" | "web_fetch")
                    && !t.description.contains("[车道:")
                {
                    t.description.push_str(" [车道:原生检索]");
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
    async fn r1_main_surface_enabled_shape() {
        let dir = test_dir();
        let host = R1SurfaceHost {
            journal: JournalRecorder::new(dir.clone()),
        };

        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        // 0t (2026-09-09, ADR-0010 §14.65 / 设计 §3.1): 检索启用会话主面
        // 恒以现行 local_browser 形态投影——裸 web_search 单一派发入口
        // （web_fetch / web_search_* 变体不在主面）；console_default 启用
        // submit（生产路径同款）。
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
        // 硬保留面：web_search 单一入口 + 工作读/写/终端 + 黑板/submit；
        // web_fetch 与 browser_read 一律不出现。
        let expected = [
            "run_terminal_cmd",
            "read_file",
            "grep",
            "search_replace",
            "web_search",
            "blackboard_read",
            "blackboard_write",
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
            "web_fetch",
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
            "blackboard_write",
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
                "bash",             // non-work tool — untouched
                "blackboard_read",  // storage chain complete
                "blackboard_write", // 0ae D0: model notes write (ReadOnly class)
                "context_compress", // 0ap: compression window request (ReadOnly class)
                "read_file",        // read chain complete
            ],
            "R1 single-face list projection (todo_write / compaction_whitelist_add \
             sealed): {declared:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 0aj-review（2026-09-16）：**声明面分类护栏**——进入主车道声明面的工具
    /// 必须要么是工作工具（⇒ 有机械链探针、按探针完整集投影），要么属于**规则式
    /// 非工作族**（shell 族 / 外部检索族 / 交付 `submit` / 检索模式门工具）。
    ///
    /// 为什么需要：0aj 摩擦的另一半正是「`blackboard_write` 被无条件声明
    /// （0ae D0）却不在 `WORK_TOOLS`」——声明面有、探针面无（该 run 的
    /// `request_header_change` 7 件 vs `tool_availability_check` 5 件），记账
    /// 不完整，且工作区不可读时它仍留在可见面（看得见摸不着）。此前无机械可查：
    /// 本测试即补上该面——新增声明面工具若既不进 `WORK_TOOLS` 又不属上述族，
    /// 此处报红（豁免是**族规则**，不是"再列一个名字就过"）。
    #[tokio::test]
    async fn declared_face_tools_are_work_tools_or_rule_based_non_work_families() {
        use crate::tool_probe::is_main_agent_work_tool;

        fn is_declared_non_work_family(name: &str) -> bool {
            // shell 族：非工作工具，声明规则不受探针管辖（IP2a 名级排除）。
            matches!(
                name,
                "bash" | "sh" | "cmd" | "powershell" | "pwsh" | "run_terminal_cmd"
            )
                // 外部检索族（web_search / web_fetch 及其变体）：检索 lane 自执行。
                || crate::relay::is_web_retrieval_tool(name)
                // 终端交付工具（交付期声明，机械链即交付出口本身）。
                || name == "submit"
                // 检索模式门工具（宿主路由检索族；主面由模式门与 R1 封存管辖）。
                || name == "pdf_read"
                || name.starts_with("retrieve_project_")
        }
        let assert_classified = |tool: &str, source: &str| {
            assert!(
                is_main_agent_work_tool(tool) || is_declared_non_work_family(tool),
                "declared tool {tool} ({source}) is neither a work tool (no probe verdict) \
                 nor a rule-based non-work family — add it to tool_probe::WORK_TOOLS with a \
                 probe criterion, or classify it here"
            );
        };

        // ① 真实 run 形态：跑一轮，取模型实际看到的声明面。
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
            .run_turn(&host, "hi", "RUN-PROJ-CLASS", MANIFEST, 0, None, None, None)
            .await
            .unwrap();
        let received = fake.received_requests();
        for tool in received[0].tools.iter().map(|t| t.name.as_str()) {
            assert_classified(tool, "live run-start face");
        }
        let _ = std::fs::remove_dir_all(&dir);

        // ② 生产 `-p` 车道冻结样本（狗粮 run `RUN-CLI-6aa999d6` 的
        //    `request_header_change.tools`，seq=5 实测 7 件）——0aj 漏网形态
        //    的回归样本：`blackboard_write` 当时在这一面却不在探针面。
        for tool in [
            "grep",
            "run_terminal_cmd",
            "read_file",
            "search_replace",
            "blackboard_read",
            "blackboard_write",
            "submit",
        ] {
            assert_classified(tool, "frozen RUN-CLI-6aa999d6 header sample");
        }

        // ③ 0ap（2026-09-18，设计 §4-⑤）：负例钉补新工具样本——
        //    `context_compress`（压缩交互第九工具）经 WORK_TOOLS 分类
        //    （工作工具 ⇒ 探针面有机械链判据）。它若从探针面漏网即在此
        //    报红，防 0aj「声明面有、探针面无」同形第三例。
        assert_classified(
            orz_assurance::tool_names::CONTEXT_COMPRESS_TOOL_NAME,
            "0ap ninth-tool sample",
        );
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

    /// FUS-RETRIEVAL-MECH P0-B 步骤 4 前置裁决（2026-08-14 用户裁决）+
    /// 0t（2026-09-09, ADR-0010 §14.65 / 设计 §3.1）：外部 lane 在启用会话
    /// 双族恒在——继承的 web 族保留，browser_read 从 host registry 恢复
    /// （registry 未声明时不得发明）；主车道专属控制工具继续剔除。
    /// 未启用会话任何 role 均无检索工具（fail-closed）。
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
            true,
        );
        let mut names: Vec<&str> = projected.iter().map(|t| t.name.as_str()).collect();
        names.sort();
        assert_eq!(
            names,
            vec![
                "browser_control",
                "browser_read",
                "read_file",
                "web_fetch",
                "web_search"
            ],
            "dual-lane external projection keeps the inherited web family, \
             restores browser_read + browser_control + web_fetch from the \
             registry, and strips console/main-only tools: {names:?}"
        );
        // 静态车道标注（§3.2 固化措辞）：浏览器族与原生族分别携带车道文本。
        for t in projected.iter() {
            match t.name.as_str() {
                "browser_read" | "browser_control" => assert!(
                    t.description.contains("[车道:本地浏览器检索|推荐首选]"),
                    "{} must carry the local-browser lane tag: {}",
                    t.name,
                    t.description
                ),
                "web_search" | "web_fetch" => assert!(
                    t.description.contains("[车道:原生检索]"),
                    "{} must carry the native-retrieval lane tag: {}",
                    t.name,
                    t.description
                ),
                _ => {}
            }
        }

        let absent = AgentLoopController::subagent_tool_projection(
            &parent,
            &EmptyRegistry,
            SubagentRole::ExternalRetrieval,
            true,
        );
        let mut absent_names: Vec<&str> = absent.iter().map(|t| t.name.as_str()).collect();
        absent_names.sort();
        assert_eq!(
            absent_names,
            vec!["read_file", "web_search"],
            "no invention when the host registry lacks the host-routed \
             retrieval tools; the inherited web family stays (dual-lane): \
             {absent_names:?}"
        );

        // 未启用会话：外部 lane 无任何检索工具（web 族与 browser_read 均
        // 剔除，fail-closed belt and braces）。
        let disabled = AgentLoopController::subagent_tool_projection(
            &parent,
            &BrowserDeclaringRegistry,
            SubagentRole::ExternalRetrieval,
            false,
        );
        let mut disabled_names: Vec<&str> = disabled.iter().map(|t| t.name.as_str()).collect();
        disabled_names.sort();
        assert_eq!(
            disabled_names,
            vec!["read_file"],
            "disabled session external lane carries no retrieval tools: \
             {disabled_names:?}"
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
            "browser_control",
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
            true,
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
            "internal lane is read-family only (no web/browser control / \
             retrieve dispatch): {names:?}"
        );
    }

    /// FUS-RETRIEVAL-MECH P0-B 步骤 4 前置裁决（2026-08-14 用户裁决）：
    /// 父侧投影已含 `browser_read` 时恢复逻辑不重复追加。
    #[test]
    fn subagent_projection_keeps_browser_read_singleton() {
        let parent = ["read_file", "browser_read", "browser_control", "web_fetch"]
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
            true,
        );
        assert_eq!(
            projected
                .iter()
                .filter(|t| {
                    matches!(
                        t.name.as_str(),
                        "browser_read" | "browser_control" | "web_fetch"
                    )
                })
                .count(),
            3,
            "host-routed retrieval tools must each appear exactly once \
             (no duplicates): {projected:?}"
        );
    }

    /// 0t (2026-09-09, ADR-0010 §14.65 / 设计 §3.1)：主面检索投影只由
    /// 启用门单参数决定——启用会话恒以现行 local_browser 形态投影（裸
    /// web_search 单一派发入口 + 英文入口标注，其余 web 族隐藏）；未启用
    /// 会话本函数不动（检索族剔除由 run-start 投影承担）。
    #[test]
    fn retrieval_surface_projection_follows_enable_gate() {
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

        // 启用：web_fetch 族与 web_search 变体隐藏，裸 web_search 保留为
        // 外部检索派发入口（R1 封存前置剔除由 project_main_agent_tool_defs
        // 承担，本函数只管启用面）。
        let mut enabled = defs(&[
            "web_search",
            "web_fetch",
            "web_search_x",
            "browser_read",
            "retrieve_project_docs",
            "read_file",
        ]);
        AgentLoopController::apply_retrieval_surface_projection(&mut enabled, true);
        assert_eq!(
            names(&enabled),
            vec![
                "browser_read",
                "read_file",
                "retrieve_project_docs",
                "web_search"
            ],
            "enabled session keeps bare web_search as the dispatch entry and \
             hides web_fetch / web_search variants: {enabled:?}"
        );
        // 入口标注：主面 web_search 描述声明外部检索子代理派发语义。
        let ws = enabled
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

        // 未启用：本函数不动（检索族整体剔除是 run-start 投影的唯一职责面）。
        let mut disabled = defs(&[
            "web_search",
            "web_fetch",
            "browser_read",
            "retrieve_project_docs",
            "read_file",
        ]);
        AgentLoopController::apply_retrieval_surface_projection(&mut disabled, false);
        assert_eq!(
            names(&disabled),
            vec![
                "browser_read",
                "read_file",
                "retrieve_project_docs",
                "web_fetch",
                "web_search"
            ],
            "disabled session is untouched by the surface helper (the \
             caller strips the retrieval family): {disabled:?}"
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

    /// A host registry that declares the host-routed retrieval tools the
    /// external lane restores (browser_read / browser_control / web_fetch).
    struct BrowserDeclaringRegistry;
    impl ToolRegistry for BrowserDeclaringRegistry {
        fn get(&self, name: &str) -> Option<ToolDef> {
            matches!(name, "browser_read" | "browser_control" | "web_fetch").then(|| ToolDef {
                name: name.to_string(),
                description: format!("tool {name}"),
                parameters: serde_json::json!({}),
            })
        }
        fn list(&self) -> Vec<ToolDef> {
            ["browser_read", "browser_control", "web_fetch"]
                .iter()
                .map(|n| ToolDef {
                    name: n.to_string(),
                    description: format!("tool {n}"),
                    parameters: serde_json::json!({}),
                })
                .collect()
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
            vec![
                // 0aj（2026-09-16）：`blackboard_write` 现已入工作工具表并
                // 走 `probe_storage`（黑板/台账/journal 的持久化面均在会话
                // 工作区 `.gsa/` 下，与 `blackboard_read` 同判据）⇒ 工作区
                // 不可读时与其余工作工具一同从可见面移除，不再豁免。
                "bash", // non-work tool — untouched
            ],
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

    /// 0ac S3①（2026-09-13, 设计稿 §9 / §10.2）：检索族探针读数形状钉子——
    /// 三成员键恒在（browser/search_engine/web_channel），`probe_scope`
    /// 为 `retrieval_family`，`gate_decision` 恒 pass（探针不阻断）；
    /// 家族在本 run 关闭时三成员一律 absent + 中性原因（读数只报事实）。
    #[test]
    fn retrieval_family_payload_pins_three_members() {
        let base = R1SurfaceRegistry::all();
        let enabled = AgentLoopController::retrieval_family_payload(&base, true);
        assert_eq!(enabled["probe_scope"], "retrieval_family");
        assert_eq!(enabled["gate_decision"], "pass");
        let family = &enabled["retrieval_family"];
        for member in ["browser", "search_engine", "web_channel"] {
            assert!(
                family.get(member).is_some(),
                "retrieval_family.{member} must always be present: {family}"
            );
            assert!(
                family[member]
                    .get("present")
                    .and_then(|p| p.as_bool())
                    .is_some(),
                "retrieval_family.{member}.present must be a boolean"
            );
        }
        // R1SurfaceRegistry 只在册 web_search / web_fetch：search_engine 与
        // web_channel 读 present，browser 读 absent（中性原因、无教学句）。
        assert_eq!(family["search_engine"]["present"], true);
        assert_eq!(family["web_channel"]["present"], true);
        assert_eq!(family["browser"]["present"], false);
        assert!(
            family["browser"]["reason"]
                .as_str()
                .is_some_and(|r| !r.is_empty())
        );

        let disabled = AgentLoopController::retrieval_family_payload(&base, false);
        for member in ["browser", "search_engine", "web_channel"] {
            assert_eq!(
                disabled["retrieval_family"][member]["present"], false,
                "retrieval family off ⇒ {member} must read absent"
            );
        }
    }

    /// RET-B3（2026-09-15）：探针读数单一源化——装配期快照命中时 detail
    /// 带 `local_segmented=on|off` 与 `chain_detail`（含 proxy=on|off +
    /// 脱敏端点）；构造用 `from_env_with`（env 无关 seam），与进程级快照
    /// 无关、确定性。
    #[test]
    fn search_engine_probe_detail_reads_assembled_config_with_proxy_state() {
        use orz_tools::implementations::web_search::local_segmented::{
            self as ls, LocalSegmentedConfig,
        };
        // 无代理（缺省直连 360search,baidu 双引擎；0bs c 轮 Bing 出集）：
        // proxy=off。
        let direct = LocalSegmentedConfig::from_env_with(|_| None);
        let detail = AgentLoopController::search_engine_probe_detail("web_search", Some(&direct));
        assert_eq!(
            detail, "web_search local_segmented=off chain_detail=\"360search,baidu; proxy=off\"",
            "无代理快照读数：{detail}"
        );
        // 有代理（开关开 + 代理三链）：proxy=on + 脱敏端点（scheme/凭据剥除）。
        let proxied = LocalSegmentedConfig::from_env_with(|key| match key {
            ls::ENV_SWITCH => Some("on".to_string()),
            ls::ENV_PROXY => Some("http://user:pass@127.0.0.1:7890".to_string()),
            _ => None,
        });
        let detail = AgentLoopController::search_engine_probe_detail("web_search", Some(&proxied));
        assert_eq!(
            detail,
            "web_search local_segmented=on chain_detail=\"360search,baidu,duckduckgo; \
             proxy=on 127.0.0.1:7890\"",
            "有代理快照读数带 proxy=on 与脱敏端点：{detail}"
        );
    }

    /// RET-B3：快照为 None（registry 未装配）时回退既有读 env 逻辑、原
    /// detail 格式（读数随环境变化，这里只钉形状，不钉具体值）。
    #[test]
    fn search_engine_probe_detail_falls_back_to_env_without_assembly() {
        let detail = AgentLoopController::search_engine_probe_detail("web_search", None);
        assert!(
            detail.starts_with("web_search engines=") && detail.contains(" local_segmented="),
            "回退读数保持既有格式：{detail}"
        );
    }
}
