# GAP-SUBAGENT-RUNTIME 实施审计（2026-08-10）

- 范围：ADR-0010 §3.1–3.4（同构 Agent runtime）、§3.2（允许差异与写域）、§3.3（生命周期）、§4.2（orientation 车道）、§4.3/§4.4（信息充分性与 disposition/close 链）、§4.6（Diagnostic Coverage）——Phase C 前置序轨道 A 第二切片
- 用户裁决：① DC 机制纳入本切片（完整 ADR §4.6）② 抽取共享 AgentLoop，主 agent 迁移到共享循环（同一套代码跑三 agent）③ 结构化结果 schema 暂缓（保持 `[DOC]`/`[SOURCE]` 行协议写契约）
- 迁移纪律（§5.3）：Schema/fixture 先行（Phase B 已完成）→ producer 与 verifier 同切片 → 旧版本仅 replay → 删除锁定旧逻辑的测试

## 1. 已变更（核心）

### 1.1 共享 AgentLoop（M1/M2，`agent_loop.rs` 新建）

- `run_turn_inner` 的 model↔tool 循环体（~925 行）整段迁入 `run_agent_loop`（自由函数）；`LoopProfile` 承载角色语义差异面（role/counterexample_gate/orientation_role/system_kind/tool_filter/dc_enabled/max_tool_rounds）；`SharedLoopServices` 收束 `&self` 字段子集；`RoundAgent` trait 统一模型轮入口（MainAgent + RetrievalSubagent 实现）
- 主 agent 迁移后事件序列与迁移前逐字节一致（M1/M2 门槛：conformance 重捕获 fixtures 零 diff）
- 主 agent 专属语义条件化：counterexample gate（§4.5）、plan 状态行、DC、文本 delta 转发（子代理无 live consumer，delta 丢弃——F-03 语义保留）

### 1.2 子代理同构（M3）

- `retrieval.rs` 的 one-shot scripted pass 删除（FUS-IMPL-003 明令禁止形态关闭）：`RetrievalSubagent` 实现 `RoundAgent`，子代理经 `run_agent_loop(profile=Retrieval)` 运行——同一工具 registry（投影去 `compaction_whitelist_add`/`retrieval_disposition`）、独立 120 轮预算（`tool_rounds` 循环实例局部变量）、同一 journal hash 链（不写 run 终态——run 终态唯一性归父侧）、子代理自有停滞守卫（`evaluate_stagnation` 抽取共享）
- **身份正规化（D3-3 升级）**：`activation_id = retrieval-{role}-{session8}-{seq:02}`（session 内单调）、`subagent_session_id = SUB-{role}-{session8}`、`contract_id = retrieval-contract-{role}`；`parent_session_id` = orientation session id（退化 run_id）；旧 `{target}-{call_id}` 临时身份删除
- **activation 注册表**（controller 字段）：`Active/AwaitingDisposition/Closed` 状态机；运行期间从注册表取出（std Mutex guard 不跨 await），路径上重插；conversation 跨轮/跨 continue 持久（§4.4 永不因重置删除）
- **写域 deny-only 门禁（§3.2 占位）**：`ToolFilter::Retrieval` 下 `modifies_files`/`SandboxEscape`/执行类（`run_tests`，§3.8.2 受控代码执行）工具在 host permission bridge 之前结构化拒绝（`retrieval_role_write_denied`/`retrieval_role_shell_denied`/`retrieval_role_execution_denied`），ToolStarted→ToolCompleted(status=error) 审计入链，DenialKey 汇入共享 3 轮熔断；嵌套检索派发拒绝（`nested_subagent_dispatch_refused`——每角色一席不递归）；ACAF 绝对路径解析 seam 预留
- 子代理结果形成：`[DOC]`/`[SOURCE]` 行协议（`parse_retrieval_text`/`write_section` 纯函数）+ 机械 assessment（身份用注册表值）

### 1.3 disposition/close 链（M4）

- **`retrieval_disposition` 控制工具**（main 专属，§4.4 唯一结构化提交路径）：`{role, decision∈{close,continue}, requirement_delta?}`；每 turn 常驻声明（前缀缓存稳定）；无 pending 激活时结构化拒绝（不触发熔断）；子代理 lane 内拒绝（`control_tool_lane_denied`）
- **判定（镜像 verifier §4.4，优先级序）**：① 重放幂等（disposition_id 从**前置 outcome 的调用参数**（role/decision/delta）+ call_id 派生——与状态无关，Closed 后重试仍识别；原 payload 逐字节重发，状态不动）② stale（pending.expected != act.contract_revision——continue 后迟到处置可达）③ conflicting（pending.decided 已设——防御性，当前流中 continue 先撞 stale）④ accepted close/continue
- **close record**：`normal_close`（validated_disposition_id+assessment_id+64-hex digest 全绑定）+ 终止 authority：`user_cancelled`（run_turn_with_guards 取消路径，run_cancelled 前关闭全部激活）、`subagent_failed`/`subagent_cancelled`（子循环 Err 分支）、`budget_exhausted`（部分结果先 assessment 后 close）；`live_state_reset=true`、`resumable=true` 统一（conversation 保留）、`archive_ref = run-journal:{run_id}`
- **continue 重入**：revision+1、requirement_delta 成为下一任务 goal、激活保持 Active；pending 保留为 consumed 标记（迟到处置被拒并记录）；重放追踪挂 activation 级（跨 assessment）
- **awaiting 语义**：assessment 后激活转 AwaitingDisposition——再次检索被结构化拒绝（`activation_awaiting_disposition`）

### 1.4 orientation 车道投喂 + DC（M5）

- 子代理 profile `orientation_role = Some(Internal/External)`，嵌套循环线程化共享 `OrientationSessionState`——子代理逻辑模型轮计入对应车道，7 轮触发注入 `orientation_checkpoint`（`agent_role=internal_retrieval/external_retrieval`）；侧车持久化零改动（三车道同构单文件）
- **DC producer（ADR §4.6 机制完整；硬信号 4/6 产出）**（`diagnostic_coverage.rs` 新建）：`DebugEpisodeState`（episode=`DC-{run_id}`，阈值 2→3→4→5 封顶，count 清零即"每阶段恰一次"）；硬信号消费点=共享循环 run_host_tool 之后（main lane only）：run_tests 非零退出（同 fingerprint→`consecutive_same_failure` 否则 `repeated_pattern`）、输出 error class（`unabsorbed_new_evidence`）、测试仍失败时编辑新文件（`large_scope_low_diag`——以编辑动作自身近似 §4.6.2"扩大 mutation scope"信号，非"准备扩大"阶段）；`evidence_identity = {call_id}:{fingerprint8}` 去重（journal replay 天然不重复——信号只在进程内消费点产生一次）；达阈值在 loop-top/post-tool-batch 两 gap 注入 `diagnostic_coverage_checkpoint`（12 字段全填，`signals[]` minItems 1，中性块列出覆盖/缺失面+一个最小补诊断动作）；run_tests 首次 exit 0 → 阈值回 2、count/残留信号清零；块前缀 `[DIAGNOSTIC_COVERAGE` 注册 `is_injected_block_text`（停滞守卫排除）；非 hard gate（注入后循环继续）；DC 回答轮经 feed_round(Main) 计入七轮计数；`same_module_no_evidence`/`key_surface_unexamined` 两类信号需检索/读取类别统计，enum 保留未产出（见 §4 边界）

## 2. 已删除

- `SubagentSpec`/`budget_turns` 死字段（retrieval.rs）
- 子代理 one-shot `run_retrieval` 特殊路径（FUS-IMPL-003 明令禁止形态）
- `Agents/mod.rs` SubagentSpec 再导出
- 旧 `{target}-{call_id}` 激活临时身份（D3-3）

## 3. 决策登记

| ID | 决策 | 依据 |
|---|---|---|
| D-1 | 主/子代理同 run 共享 denial breaker 状态（写域拒绝计入 3 轮熔断） | ADR-0010 §3.5.4 轮次单位；主/子同 run 串行无并发 |
| D-2 | 子代理停滞 non-continue = subagent_failed close（子代理无 handoff 目标） | §3.3 terminal 语义 |
| D-3 | `session_cancelled` 映射 `user_cancelled`（ACP 取消路径不可区分） | §4.4 终止 authority；seam 在 close_activation reason 入参预留 |
| D-4 | 子代理 model_output 无 producer 字段（payload `additionalProperties:false`）——producer 身份由父侧 `tool_started{target}` 包夹标识 | 信封 schema 不可扩展 |
| D-5 | `archive_ref = "run-journal:{run_id}"` 占位（检索文档归档路径随真实工具切片） | §3.3.5 |
| D-6 | `capability_gate = "not_applicable"`（scope 扩大重走 task-contract gate 随 capability receipt 切片） | §4.4 |
| D-7 | retrieval mode authority（V11-IMPL-001）不接线——mode 枚举/transition 事件需真实检索工具语义落地 | 用户裁决范围 |
| D-8 | DC episode 每 run 一个（`DC-{run_id}`；跨 prompt 不持久——ACP 每 prompt 一 controller） | 进程内状态；注册边界 |
| D-9 | activation 注册表 turn 级（跨 turn 持久化留真实检索切片）；run 结束未处置激活=合法终态（verifier 无 close 强制） | 注册边界 |
| D-10 | 重放幂等检查先于 pending 检查（Closed 后 pending 已清，重试仍识别） | §4.4 幂等重放 |
| D-11 | disposition_id 从调用参数（role/decision/delta）+call_id 派生（不含 assessment/revision——与状态无关） | 幂等重放需状态无关 id |
| D-12 | conflicting 判定为防御性分支（当前流 continue 先撞 stale；pending.decided 单 decision 语义保留） | verifier 同 assessment 单 decision |
| D-13 | `resumable=true` 统一（conversation/journal/ledger 全部保留；§3.3 closed_resumable） | §4.4 |
| D-14 | wallclock close record 不落地——wallclock 终止由 orz-bin 看门狗直接丢 future 写 run_invalidated，controller 无钩子 | 注册边界 |
| D-15 | DC journal capture 场景暂缓——DC 信号需 test-runner 宿主，capture 环境（真实 CLI host）无固定 runner；机制由 Rust 单测+E2E 覆盖，payload schema 由 fixture 树覆盖 | 注册边界 |
| D-16 | subagent-run-close/continue 独立 capture 不建——orientation-fire-run 单 fixture 已覆盖 4 continue+1 close 全链 + 7 轮跨越 | 合并覆盖 |
| D-17 | 子代理预算耗尽：部分结果先 assessment 后 close（verifier 顺序要求） | §4.4 |
| D-18 | continue 重入是同一检索 session 的延续——预算跨 dispatch 累计（`ActivationState.tool_rounds_used` 读入 `LoopProfile.initial_tool_rounds`，循环结束后写回），仅 activation 关闭后新激活从 0 起；主 agent 维持 per-run 既有语义（ADR-0008 未改） | 用户裁决 2026-08-10（F5 升级）；§3.4.6 |

## 4. 边界（明确未做，登记给后续切片）

- 真实检索工具（project doc index、local_browser/web）——`[DOC]`/`[SOURCE]` 行协议为稳定接口（GAP-RETRIEVAL-TOOLS）
- 结构化结果 schema（query_summary/source_ledger/filtering_log/organized_response/raw_source_refs）——用户裁决暂缓
- 跨 turn activation 持久化（controller turn 间无状态）
- capability receipt / ACAF 写域绝对路径解析（ADR-0011 独立切片；seam `(role, tool, args) -> WriteDomainError` 预留）
- retrieval mode authority（V11-IMPL-001 保持开放）
- `pre_handoff` orientation trigger（schema 已注册未接线）
- `session_cancelled` 独立 close reason（D-3 映射）
- wallclock close record（D-14）
- DC 的 `same_module_no_evidence`/`key_surface_unexamined` 信号（需检索/读取类别统计，enum 保留）；`large_scope_low_diag` 以"测试仍失败时编辑新文件"近似 §4.6.2"扩大 mutation scope"信号（以编辑动作自身触发，非"准备扩大"阶段）
- v0.1 轨任何改动（replay-only 历史冻结）

## 5. 验证

- `cargo test -p orz-loop`：**139 passed / 3 ignored**（M1 迁移回归 118 → M3 +7 → M4 +7 → M5 +7）
- `cargo test -p orz-host -p orz-bin`：98 / 9+1 全绿（API 表面未破坏）
- conformance capture 7/7：orientation-fire-run 61 事件（子代理 model_output+stagnation+assessment+disposition+close+orientation fire 全链），其余 6 场景与 M4 语义字节一致（仅 per-run 环境值差异）
- Python：`test_run_event_journal_validation.py` 57 passed；`check_repository.py` valid（EXPECTED_SEQUENCES_V02/ALL_JOURNALS_V02 已同步）
- clippy：变更文件零新增（pre-existing 5 条全在未变更 crate：orz-config 1 / xai-fast-worktree 2 / orz-loop 1（agents/main.rs 固有方法 too_many_arguments）/ orz-host 1）
- verifier 零逻辑改动（§4.4 链规则本就覆盖 disposition/close 生产形态）

## 6. 审查修复批（2026-08-10 三面审查）

三面审查（设计合理性/实现合理性/设计与实现符合性）发现与处置：

| ID | 审查发现 | 处置 |
|---|---|---|
| F1 | 全轨道未提交，索引/审计已标 `implemented` | 本批提交闭环（orz 仓 + 主仓同批） |
| F2 | "ADR §4.6 完整"措辞过强——硬信号实际 4/6 产出 | 措辞限缩为"机制完整；硬信号 4/6 产出"；§1.4 与索引条目同改；`large_scope_low_diag` 近似语义在 §1.4/§4 注明 |
| F3 | retrieval `max_tokens` 双源字面量（agent_loop.rs:457 vs `main_agent_max_tokens`） | 收敛为 `agent_loop::REQUEST_MAX_TOKENS` 单常量；controller/agent_loop/retrieval 测试同源（§3.4.2 三 agent 同注入） |
| F4 | `DebugEpisodeState.resolved` 死字段（只写不读） | 删除（阈值回 2 由 run_tests exit 0 路径直接完成） |
| F5 | 子代理预算按 dispatch 重置（continue 重入后从 0 计） | **用户裁决升级（2026-08-10）**：continue 重入是同一检索 session，预算跨 dispatch 累计——`ActivationState.tool_rounds_used`（新建激活 0 起，循环结束写回）+ `LoopProfile.initial_tool_rounds` 读入，仅 Closed 后新激活重置（D-18）；主 agent 维持 per-run 既有语义；判别测试 `subagent_budget_accumulates_across_continue` |
| F6 | ① clippy 声称"pre-existing 1 条"实际 5 条全 pre-existing ② run_tests 以 `retrieval_role_write_denied` 拒绝语义不精确 | ① 措辞修正（§5）② 新增 `retrieval_role_execution_denied` 执行类拒绝 reason（§3.8.2 受控代码执行）+ write_gate 单测 |
| F7 | `handle_parent_disposition` 错误路径不重插 activation（journal 写失败时注册表丢失活激活） | 提交块重构：disposition+close+状态切换单 commit，任何错误路径先重插再返回（journal 失败 run-fatal，注册表不静默丢激活） |

回归（修复后）：`cargo test -p orz-loop` 141 passed / 3 ignored（+1 write_gate 单测 +1 预算累计单测）；orz-host/orz-bin 全绿；conformance capture 7/7；Python 57 passed。
