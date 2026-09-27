# P0-0x S1/S2 全面复审处理 — 文档一致性 + 测试补强（2026-09-11）

> **范围**：对 0x 初始轮中立问询的 S1/S2 落地做**设计合理性 / 实现合理性 /
> 设计—实现符合性**三面全面复审，并处理复审发现的问题（2 处文档一致性 +
> 2 处测试覆盖补强）。**无代码语义改动**。
> **设计权威**：[`INITIAL_ORIENTATION_DECLARATION_DESIGN_2026-09-11`](../INITIAL_ORIENTATION_DECLARATION_DESIGN_2026-09-11.md)
> / [`ADR-0010 §14.66`](../../adr/ADR-0010-vol-14-addenda-index.md) / BACKLOG 0x / TODO P0-0x。
> **前序**：[`0X S1 实施记录`](0X_S1_INITIAL_ROUND_INQUIRY_IMPL_2026-09-11.md) /
> [`0X S2 实施记录`](0X_S2_EVENT_FACE_AND_SIGNER_2026-09-11.md)。
> **状态**：复审与处理完成（门禁 `valid: true`）；**S3 双平台重建 / S4 实机复验
> 仍未放行**。子模块 `dcf4a774`。

## 1. 复审结论

三面复审结论：**S1/S2 实现成立、可复现，设计—实现符合度高**。逐点复核：

- **设计合理性**：时机取 B（首个动作批次结束）站得住——保证模型在已有一次真实
  动作上下文后才「想一次」，且远早于长任务收尾（对照 TB 4.0 等第 776 轮才出报告）。
  边界如实：这是「尽早」而非「先于动作」，第一个动作批次本身的方向偏差不会被拦下；
  按用户口径「一边说一边动作、只要让模型想了就够」，取舍可接受。三问措辞纪律
  （要评估不要交代、不设结构化字段、不追责）与落地文本一致。
- **实现合理性**：分派落点收敛进唯一出口 `maybe_fire_orientation`；commit 语义按
  `is_initial_round()` 分派（只置标志 + 推进序号，不重置周期计数）；注入块前缀
  登记进 `is_injected_block_text` 并实际作用于会话回写过滤（`controller.rs:3539`）；
  侧车 `#[serde(default)]` 兼容旧会话；事件面 trigger↔block↔position 三点耦合 +
  会话内恰好一次由家族法官（Rust 执法 / Python 冻结镜像）逐格零差执法；签名侧第二
  模板摘要按同一 trigger 选摘要比对，trigger 同时进入 `canonical_arguments_sha256`。
- **设计与实现符合性**：设计 §5 的 7 项触点逐项有落点与测试证据（见 §2 的补强项）。

## 2. 发现与处理

### 2.1 [文档一致性] `TODO.md` 开放项路由行状态滞后

- **现象**：`TODO.md` 开放项路由行仍写 0x「设计定稿、无待裁决项、**未实施**」，
  与同文件 P0-0x 小节（「S1–S2 已实施，S3–S4 待续」）及 BACKLOG 0x / 索引
  `AUTH-INITIAL-ROUND-INQUIRY` 矛盾。
- **处理**：路由行更正为「0x 初始轮中立问询（S1–S2 已实施（2026-09-11）、
  S3–S4 待续；ADR-0010 §14.66 / v1.67）」。

### 2.2 [文档指针错误] 「Python 镜像」指错文件

- **现象**：设计 §5-6 与 ADR-0010 §14.66 第 4 项⑥把本族的 Python 镜像写成
  `assurance/orientation_runtime_guard.py`；该文件是 **v0.1 冻结 reference**
  （其 `ORIENTATION_BLOCK` 至今仍是 `[ORIENTATION_CHECKPOINT v0.1]`），本就不应改动。
  本族真正的 Python 冻结镜像在
  `assurance/run_event_journal_validation.py::_verify_v02_initial_round_inquiry`
  （S2 实施记录本身也是这样写的）。
- **处理**：两处文档更正指针，并显式登记「v0.1 `orientation_runtime_guard.py`
  保持冻结、不动」，避免后续照文档去改冻结件。

### 2.3 [测试补强] 「中断后恢复重触发」缺端到端钉子（设计 §5-7）

- **现象**：设计 §5-7 测试矩阵含「中断后恢复重触发」，S1 落地的 9 项只覆盖到
  状态层「commit 前可重复构建」，没有端到端。
- **处理**：新增集成钉子
  `initial_round_refires_next_run_when_the_fire_was_not_consumed`
  （`orz-loop/src/orientation.rs`）：run1 首个动作批次 → 初始轮 fire 已落 journal、
  pending 未消费即中断（生成失败）→ 一次性标志**不提交**（`initial_round_fired`
  仍为 false、`sequence` 仍为 0）→ run2 首个含工具调用的批次结束**再次触发一次**
  （`trigger=initial_round`、`post_tool_batch_gap`）→ 回答被软消费后照常收尾。
  两个 run 共用同一份会话级 orientation 状态，run2 用独立网关与 journal。
- **边界**：恢复形态与 §3.4 登记一致——初始轮注入位置固定为
  `post_tool_batch_gap`，故恢复是「下次 run 首个动作批次」而非 `loop_top_gap`。

### 2.4 [测试补强] 签名侧缺跨 trigger 负例

- **现象**：`acaf_e2e` 只钉了「同 trigger 签发 + 校验可消费」与「两摘要不同」，
  没有「周期票以 `initial_round` 校验被拒」的跨 trigger 负例（机制层
  `template_mismatch_rejects_orientation` 已覆盖 check 2 本身）。
- **处理**：在 `signer_process_full_lifecycle` 内补负例——以缺省 trigger 签发的
  周期票（摘要等于周期内置）改用 `trigger=initial_round` 校验 → `Rejected
  { code: TemplateMismatch }`，且**不消费**（验证前即拒，ledger 未动）。

## 3. 复核证据（本刀复跑，均为独立执行）

- `cargo test -p orz-loop`：**763 passed / 0 failed**（3 ignored；S2 基线 762 +
  本刀新增 1）。
- `cargo test -p orz-bin`：全目标绿（bin orz 12 / acaf-provision 2 /
  signer 15 / acaf_e2e 23 / real_flag 2 / stdio_e2e 1）。
- `cargo clippy -p orz-loop -p orz-bin --all-targets`：**新增代码零告警**
  （`orientation.rs` / `acaf_e2e.rs` 无任何告警；既有告警全落在未改动行）。
- `python scripts/generate_orz_source_manifest.py --check` → `valid`（1441 条目）。
- `python scripts/check_repository.py` → `valid: true` / `error_count: 0`。
- `python -m pytest runtime/tests/test_run_event_journal_validation.py
  runtime/tests/test_run_event_conformance.py` → 273 passed。
- 参考（未改动，S2 基线复跑）：orz-assurance 全绿（211 lib + 9 + 4 + 9 + 1）、
  orz-host lib 单线程 286。

## 4. 变更清单

| 变更 | 落点 |
|---|---|
| 路由行状态更正 | `TODO.md` |
| 设计 §5-6「Python 镜像」指针更正 + 冻结说明；§5-7 矩阵补登；状态头补复审指针 | `docs/INITIAL_ORIENTATION_DECLARATION_DESIGN_2026-09-11.md` |
| ADR §14.66 第 4 项⑥指针更正 + 新增第 7 项「S1/S2 全面复审处理」 | `adr/ADR-0010-fusion-runtime-and-agent-architecture.md` |
| 本记录 | `docs/audits/0X_S2_REVIEW_HANDLING_2026-09-11.md` |
| 监听/回执测试补强 | `orz` 子模块 `dcf4a774`（`crates/orz-loop/src/orientation.rs`、`crates/orz-bin/tests/acaf_e2e.rs`） |
| 清单重算 | `orz_source_manifest.sha256`（子模块提交后重算，1441 条目） |

## 5. 未做（明确边界）

- **未**跑实机、**未**重建双平台载体（S3/S4 需另行走放行门）。
- **未**改代码语义：仅新增测试与更正文档指针；周期问询阈值/三问、终答前审查报告
  与反例门、票据 schema 与 `VerifyContext` 形状、`OrientationV1` 种类均未改动。
- **未**改 `assurance/orientation_runtime_guard.py`（v0.1 冻结 reference，
  有意保持不动）。
