# P0-C S3 前置实施审计：结构化策略拒绝（2026-08-15）

> 范围：CLASSICAL-EXEC-ASSISTANT 内嵌集成 S3 前置（P1-2 定案）——拒绝路径
> 在 `run_host_tool` 边界统一返回结构化 `ToolResult.policy_denial =
> {source, code, reason}`（source ∈ permission | acaf | retrieval_mode |
> taint）；console 适配层只按结构化信号映射 `step=policy`；`ToolCompleted`
> 事件增可选 `policy_denial`（Schema/verifier/fixtures 先行）；「稳定输出
> 前缀」判定（`console_policy_refusal`）退役；内容碰撞回归测试。
> 权威：设计 [CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md](../CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md) §7；
> S2 审计补记 P1-2；待办路由：[BACKLOG](../BACKLOG_AND_PRIORITIES.md) 3 与
> [TODO](../../TODO.md) P0-C。

## 1. 验收点 → 实现映射

| 验收点（P1-2 定案） | 实现位置 | 证据 |
|---|---|---|
| 拒绝路径在 `run_host_tool` 边界返回结构化信号 | `host.rs` 新增 `PolicyDenialSource`（serde snake_case）+ `PolicyDenial`；`ToolResult.policy_denial`；五条路径接线——权限门（deny/defer）、ACAF 票据门（`refuse_ticketed_tool`）、检索模式门 ×3（off / framework_fallback / local_browser） | `mode_off_gate_covers_lane_web_search`（`result.policy_denial` source/code 断言）；`console_s2_permission_denial_maps_to_policy_receipt`（receipt detail source=permission）；`fail_closed_verify_rpc_failure_journals_once_and_blocks`（acaf 事件断言） |
| `ToolCompleted` 增可选 `policy_denial`（Schema/verifier/fixtures 先行） | `runtime/tool-completed-event-payload-v0.1.schema.json`（source 枚举 + code/reason）；`assurance/run_event_journal_validation.py` `_verify_v02_policy_denial`；fixture 生成器新增正/反样例；`check_repository.py` 映射 | 仓库门禁 valid；`PolicyDenialCrossCheckTests` 8 项；`runtime/tests` 209 项通过 |
| console 适配层只按结构化信号映射 | `run_console_target` 改为唯一判定 `result.policy_denial`，detail 携带 source/code/reason；`console_policy_refusal` 函数与测试删除 | `console_s2_structured_denial_sources_map_to_policy_step`（四种 source → `step=policy` + detail 明细） |
| 内容碰撞回归（成功输出含旧拒绝前缀必须判成功） | 适配层不再读输出文案 | `console_s2_old_refusal_prefix_in_success_output_is_not_policy`（exit_code=0 + 旧前缀 → Ok） |
| 交叉规则：policy_denial 存在时 exit_code 非 0 且工具命中已知拒绝路径 | verifier `_verify_v02_policy_denial`（acaf→票据工具、retrieval_mode→检索族、permission/taint→工作工具） | `PolicyDenialCrossCheckTests` |

## 2. 实现要点

- 权限门拒绝保持既有审计契约：**不产生 ToolCompleted 事件**（`denied:
  no tool_started/completed in the journal`），结构化信号只随
  `ToolResult` 返回，console 适配层据此映射 `step=policy`；ACAF 与检索
  模式门拒绝本就写 ToolCompleted，事件 payload 现在携带 `policy_denial`
  对象（source/code/reason 与 `error` 同源）。
- 派发级检索模式拒绝（`retrieve_project_*` 子代理派发门，非
  `run_host_tool` 边界）不在本切片范围内，维持既有事件形态
  （`retrieval_mode_off` 无 policy_denial）——verifier 仅做前向规则，
  不反向要求每个拒绝错误码必须携带 policy_denial。
- taint 为预留 source：运行时无生产接线，适配层与 verifier 已接受该
  source（工作工具族），未来 taint 禁令接入时直接复用同一映射。
- `PolicyDenial` 随 host 返回结果透传（`run_host_tool` Ok 分支不再丢弃
  `res.policy_denial`），host 级结构化拒绝同样写入 ToolCompleted 事件。
- 与 denial breaker 的统一（ACAF/模式门也产生 `PolicyFeedback::Denied`）
  按定案为**可选实施决策**，本切片不实施。

## 3. 测试证据

- `cargo test -p orz-loop`：**366 passed / 0 failed / 3 ignored**（新增：
  结构化来源映射、内容碰撞回归、DC fire/commit 单元测试；更新：权限拒绝
  receipt detail、模式门事件断言）。
- `cargo test -p orz-bin --test acaf_e2e`：**21 passed / 0 failed**
  （`fail_closed_verify_rpc_failure_journals_once_and_blocks` 新增
  policy_denial 事件断言）。
- `cargo test -p orz-host --lib permission`：18 passed / 0 failed。
- `cargo check -p orz-host -p orz-tui -p orz-bin --tests` 通过；
  `cargo fmt --all -- --check` 通过；clippy 无新增告警（lib 基线 17 项）。
- Python：`runtime/tests/test_run_event_journal_validation.py` +
  `test_run_event_conformance.py` **209 passed**（含新增
  `PolicyDenialCrossCheckTests` 8 项）。
- `python scripts/check_repository.py`：**valid、0 errors**（fixture 树
  含新增 `tool-completed.policy-denial.valid.json` 与
  `tool-completed.policy-denial-bad-source.constraint.invalid.json`）。

## 4. 边界与登记

- 已捕获 journals（如 `mode-off-refusal.jsonl`）为历史生产形态，未携带
  policy_denial；verifier 前向规则不拒绝。下次真实运行重捕后，
  `run_host_tool` 边界拒绝将自然携带结构化字段。
- 派发级检索拒绝、候选计数门（cap）、注入预算门、runner 可用性拒绝均
  不属 source 枚举（permission/acaf/retrieval_mode/taint），不加
  policy_denial。
- S2 审计表格中「策略表（taint/模式门）」行的稳定前缀判定已被本切片
  取代：适配层只认结构化信号，`console_policy_refusal` 退役。

## 5. 补记（2026-08-15 工具事故与恢复）

本切片实施过程中，批量机械改写 `ToolResult` 字面量（补
`..Default::default()`）的脚本存在括号配对缺陷，导致
`diagnostic_coverage.rs` 生产实现被覆盖且该目录不在 git 跟踪内（orz/
 整体 gitignore），无法从版本库恢复。该模块已按以下契约重建：
控制器/单元测试（DC 信号语义、fire/commit 阶段机、证据身份去重）、
`agent_loop.rs`/`controller.rs`/`checkpoint.rs` 的 API 调用面、Schema
（`diagnostic-coverage-checkpoint-event-payload-v0.2.schema.json`）与
`DIAGNOSTIC_COVERAGE_PREFIX` 引用。重建后全量 orz-loop 366 项（含 4 项
DC 控制器测试与 7 项模块单元测试）通过，行为契约与测试一致；但该文件
非原始文本逐字节恢复，后续如需比对历史实现细节，只能以测试与调用面为准。
其余 8 个受影响文件经确定性逆变换恢复并全量编译/测试验证。

## 6. 审计结论

S3 前置的 4 项验收点全部有实现入口与测试证据：结构化信号覆盖五条拒绝
路径、ToolCompleted 契约扩展（Schema/verifier/fixtures 先行）、适配层
退役文案判定、内容碰撞回归锁定。未发现静默降级路径。S3
（`assistant.trace` 接线、`workspace.run_script` 生产化、Profile/Bundle）
与 S4（端到端 + 正式组件决策门材料）按 TODO/BACKLOG 继续。
