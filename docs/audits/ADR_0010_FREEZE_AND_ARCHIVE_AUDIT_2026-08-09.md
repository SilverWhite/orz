# ADR-0010 冻结与首批文档归档审计（2026-08-09）

> 后续补充：ADR-0010 已在同日按用户复核升级并重新冻结为 v1.1；Information Sufficiency、检索模式、轮次和拒绝机制的最新裁决见 `ADR_0010_V1_1_SUPPLEMENT_REVIEW_2026-08-09.md`，本文件其余归档事实仍有效。

## 1. 结论

ADR-0010 已接受并冻结，成为当前唯一自然语言设计权威。首批 17 份被取代或降级的设计输入、实现计划与 runtime spike 已归档，当前索引和文档引用已切换到新路径。

本次只冻结设计并整理文档，**不宣称现有 Rust 实现已经符合 ADR-0010**。机械对照确认仍有必须在后续实现重构中闭合的高优先级偏差。

## 2. 冻结裁决摘要

- ORZ 采用成熟组件优先的融合架构，不再坚持“production loop 一律不得自研”的旧边界。
- 一个主 Agent、一个内部检索子代理和一个外部检索子代理复用同一架构；模型、thinking、transport、工具 registry、context、compaction 与单 session 预算默认值相同。
- 内外检索角色可双并发，各角色最多一个 active instance；全局 `web_search` concurrency 为 1。
- 子代理只允许写自身 blackboard 检索分区、当前任务检索文档和检索记录归档。
- 默认工具轮预算为 120，三个 session 独立计账。
- Orientation 只做 7 轮 session-level 方向检查；信息充分性、子代理关闭确认、Counterexample 与 Runtime Stagnation Guard 各自独立。
- 输出重复只属于停滞守卫；`tool_calls`、`tool_variety` 和 semantic action 不再触发 Orientation。
- Windows process spike、产品事故台账和精选案例库分开治理。

## 3. 首批归档结果

| 归档区 | 数量 | 内容 |
|---|---:|---|
| `存档/architecture/pre-adr-0010/` | 10 | agent loop、fork、融合 v0.1/v0.2、检索和 session 历史设计 |
| `存档/architecture/runtime-spikes/` | 1 | Windows process/runtime spike |
| `存档/docs/design-inputs/` | 3 | runtime-first 评估、反例问询、全文可见性设计输入 |
| `存档/docs/implementation-history/` | 3 | 修复计划、问询/blackboard 偏差、停滞守卫计划 |

每份归档文件头均登记 `original_path`、`archived_at`、`final_status`、`superseded_by` 与历史权威范围。归档原文保留，不以改写历史来制造一致性。

## 4. 当前实现偏差

| ID | 机械观察 | 与 ADR-0010 的差距 | 后续处理 |
|---|---|---|---|
| FUS-IMPL-001 | `orz/crates/orz-loop/src/controller.rs` 仍定义 `MAX_TOOL_ROUNDS = 40` | 冻结值为 120 | 修改常量及预算相关测试；保留 ADR-0008 的其余语义 |
| FUS-IMPL-002 | ~~`orz/crates/orz-loop/src/inquiry.rs` 仍以 `output_repeats/tool_calls/actions/rounds` 四计数和 8 轮阈值驱动同一 inquiry；controller 仍产生 `NeutralInquiry`~~ | ~~冻结设计要求 Orientation 仅 7 轮，信息充分性独立，重复只归停滞守卫，其他三项不触发 Orientation~~ | **已解决（2026-08-10，GAP-INQUIRY-SPLIT）**：`inquiry.rs` 与混合机制删除；Orientation 7 轮会话状态机 + 真实注入 + v0.2 事件；信息充分性机械 assessment；重复只归停滞守卫；producer 整轨翻 v0.2。见 [`GAP_INQUIRY_SPLIT_IMPL_AUDIT_2026-08-10.md`](GAP_INQUIRY_SPLIT_IMPL_AUDIT_2026-08-10.md) |
| FUS-IMPL-003 | `orz/crates/orz-loop/src/agents/retrieval.rs` 明确写着 one scripted model pass，`tools: Vec::new()`，`budget_turns` 未形成同构 session loop | 子代理必须复用主 Agent runtime、工具、journal、生命周期和预算，仅任务合同与写域不同 | 将 retrieval role 接入共享 Agent runtime；移除一次性零工具特殊路径及其误导测试 |
| FUS-IMPL-004 | 当前 retrieval completion 仍把 free-form response 当作 best-effort evidence | 冻结设计要求信息充分性状态与关闭确认分离，并具有明确事件、producer、consumer 与 verifier | 扩展事件 Schema 和结构化 receipt，迁移旧 journal 只读 replay |
| FUS-IMPL-004 后续 | ~~retrieval completion 仍把 free-form response 当作 best-effort evidence~~ | 信息充分性与关闭确认分离 | **部分解决（2026-08-10）**：free-form 自报已删，`information_sufficiency_assessment` 机械事件落地（producer=controller 单写者、verifier 双轨已就位、v0.2 真实 journals 已捕获）；关闭确认的 disposition/close 链留子代理同构切片（GAP-SUBAGENT-RUNTIME） |
| FUS-DOC-001 | Windows 事故与案例目录已建立，但尚无首批结构化条目 | ADR-0010 明确此时只能称 planned/not started 或 maintainer-observed | 后续按 provenance、分类、脱敏和回归门槛迁移，不提前宣称案例闭环 |

## 5. 整理边界

- `README.md`、`CLI_PROJECT_INDEX.md` 与 ADR 状态已经指向 ADR-0010。
- 被移动文档的当前引用已更新；历史文档内部的演进叙述可以保留，但不能重新获得 current-authority 地位。
- Schema、源码和锁定旧逻辑的测试尚未在本次文档整理中修改；它们属于 ADR-0010 Phase B/C，不得把“ADR 已冻结”误写成“实现已完成”。
