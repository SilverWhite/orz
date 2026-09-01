# P2-11 依赖图主线实施审计（S1 代码 + S2 测试）

> 日期：2026-09-01；主题：P2-11 第 4 项依赖图实施（文件锚点链最小范围）；
> 状态：**S1 代码 + S2 测试完成**（`partial`——S3 重建 + S4 实机复验待放行）；
> 裁决来源：MODEL_RESIDUAL_PRESSURE_DISCUSSION_2026-08-31 §8 第 6 项
> （用户确认"下一轮主线"）+ MECHANICAL-LAYER-MATH-CALCULUS D3 + 机械层设计
> §1/§2.3/§5.5；设计：`docs/DEPENDENCY_GRAPH_MAINLINE_DESIGN_2026-09-01.md`；
> 权威转录：ADR-0010 §14.51。

## 1. 实施范围

| 项 | 内容 | 落点 |
|---|---|---|
| S1a | 依赖图运行时（ReadFact/WriteFact、锚点边匹配、容量淘汰、revision、渲染） | `orz/crates/orz-loop/src/dep_graph.rs`（新模块） |
| S1b | 黑板接入（`Blackboard.dep_graph` + 分区版本计数） | `orz/crates/orz-loop/src/blackboard.rs` |
| S1c | PULL 查询面（`render_blackboard_section section=deps` live-only 分支 + 工具定义 section 枚举/描述 + 增量头固定序） | `orz/crates/orz-loop/src/controller.rs` |
| S1d | 建图接线（通用执行路径成功分支：read_file/search_replace 成功建图 + 事实随 ToolCompleted 入事件面） | `orz/crates/orz-loop/src/host_exec.rs` |
| S1e | 事件面 schema（`dep_graph` 可选属性 + `$defs.dep_anchor`） | `runtime/tool-completed-event-payload-v0.1.schema.json` |
| S1f | verifier（`_verify_v02_dep_graph_events`：形状/工具/非成功/锚点边交叉核对） | `assurance/run_event_journal_validation.py` |
| S2a | 单元测试（锚点匹配/边链接/容量/revision/渲染确定性/JSON 解析） | `dep_graph.rs` tests（5 项） |
| S2b | 集成测试（真实工具链 read→write→PULL；失败/D3 不建图） | `host_exec.rs` tests（2 项） |
| S2c | verifier 测试（合法链/悬空边/路径不符/锚点不符/工具不符/非成功/坏 kind/无事实忽略） | `test_run_event_journal_validation.py` DepGraphEventCrossCheckTests（8 项） |
| S2d | schema fixtures（read/write 正例 + bad-kind/extra-field 反例） | `runtime/fixtures/run-event-v0.2/payloads/`（4 个） |

## 2. 关键裁决与实现口径

- **建图仅成功**：`exit_code == 0` 的 read_file/search_replace 才建图；失败/
  拒绝（含 content_anchor_mismatch 拒单）不建图——失败目标身份已由 F4/I2
  事件面字段承担，不重复。
- **D3 边界**：terminal.run（run_terminal_cmd/run_tests）与检索族
  （web_search/web_fetch/browser_read/retrieve_project_*/pdf_read）从不建图
  ——`record_dep_graph_fact` 只对 read_file/search_replace 分派。
- **锚点边匹配**：sha256 权威（双方都有时）；read 缺 sha256 时降级
  size+mtime 快筛（与 `verify_content_anchor` 口径一致）。匹配取**同路径
  最晚**（seq 最大）的 read 事实；无匹配/未带 expected_anchor → 无边
  （`consumed_read = None`，如实记录）。
- **GetPut 写前自动补锚不改**：本批只记录锚点链事实，不改变写路径行为
  （裁决 4 维持现状：未带 expected_anchor 直接执行、0 拒单实证）。
- **事件面可选事实**：`dep_graph` 为可选增强面（向后兼容既有语料/归档
  replay），只对已登记事实做 verifier 一致性核对。
- **live-only**：不进 epoch 快照、不持久化（同 entities/temporal 纪律）；
  revision `#[serde(skip)]` 不进序列化面。

## 3. 验证结果（2026-09-01 实测）

| 项 | 结果 |
|---|---|
| orz-loop lib | **642 passed / 0 failed / 3 ignored**（含新 dep_graph 5 + host_exec 集成 2） |
| orz-assurance | 199 lib + 9 fake_provider 全绿（无改动，回归确认） |
| Python conformance | 15 passed（新增 4 fixtures 经 schema 校验） |
| Python journal validation | 240 passed（含 DepGraphEventCrossCheckTests 8 项） |
| fmt | `cargo fmt -p orz-loop -- --check` 干净 |
| clippy | 无新增（host_exec 4 条为既有基线：dead_code/collapsible_if/too_many_arguments） |

## 4. F11 收口（顺带闭合）

- 事件链侧 receipt↔事件同构核对 V1c 已于 2026-08-31 实现（verifier 245
  passed）；本批把依赖图事实纳入同一核对面：read/write 事实随 ToolCompleted
  入链，`_verify_v02_dep_graph_events` 交叉核对形状、工具族、非成功完成、
  锚点边悬空/路径/锚点一致性。
- 机械层设计 §5.4 状态行与阶段 2 审查审计 F11 行同步更新为已闭合（见
  `docs/MECHANICAL_LAYER_MATH_CALCULUS_DESIGN_2026-08-30.md` §5.4 /
  `docs/audits/MECHANICAL_LAYER_MATH_CALCULUS_PHASE2_REVIEW_AUDIT_2026-08-31.md`）。

## 5. 剩余步骤

- S3：Linux musl 重建（ORZ-BUILD-MOUNT-001 契约，build_orz_aliyun.sh
  三件套）+ bookworm 冒烟——待用户放行。
- S4：实机冒烟（read→write 锚点边经真实二进制端到端可见 + 事件链
  verifier 0 错误）——待用户放行。
- 闭合时计数 28 → 27（TODO/BACKLOG 同步）。

## 6. S1 全面审查处理（2026-09-01）

对当前实现的全面检查（设计合理性 / 实现合理性 / 设计与实现符合性）后
收口，不改变设计语义：

- 渲染截断 footer 字节预算修复（`dep_graph.rs` `render_text`：正文预算
  扣除 footer 长度，字节超限时 footer 完整落盘且总长仍 ≤8 KiB）+ 补
  字节超限截断路径单测；
- `host_exec.rs` 无效 section 文案补齐 deps/temporal（与工具定义枚举
  一致）；
- verifier 措辞修正（锚点形状职责归 schema）+ 移除 `reads` 未用 index；
- 设计措辞统一：§2 锚点匹配降级（任一方缺 sha256）、§4 事实排序与
  truncated 语义、§5 verifier 形状职责；
- 边界测试补充：read 锚点 None 不链接、deps 带 epoch/receipt_id 错误
  形状、verifier size+mtime 回退正反用例；
- 登记：锚点计算成本（全量重读 + sha256 与实体登记重复）与序列化面
  （`dep_graph` `#[serde(default)]` 与 entities 先例一致，当前无整黑板
  序列化路径）。

详见 `docs/audits/P2-11_DEPENDENCY_GRAPH_S1_REVIEW_AUDIT_2026-09-01.md`。
