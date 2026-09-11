# P0-0x S2 实施记录 — 事件面收口与签名侧第二模板摘要（2026-09-11）

> **范围**：S2（事件面 fixtures 正负例 / 法官族 / Python 镜像 / `orz-signer`
> 第二模板摘要 / `check_repository` 全绿收口）。
> **设计权威**：[`INITIAL_ORIENTATION_DECLARATION_DESIGN_2026-09-11`](../INITIAL_ORIENTATION_DECLARATION_DESIGN_2026-09-11.md)
> / [`ADR-0010 §14.66`](../../adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / BACKLOG 0x / TODO P0-0x。
> **前序**：[`0X S1 实施记录`](0X_S1_INITIAL_ROUND_INQUIRY_IMPL_2026-09-11.md)。
> **状态**：S2 完成（门禁 `valid: true`）；**S3 双平台重建 / S4 实机复验未放行**。
> **后序**：S1/S2 全面复审已处理（2026-09-11，同日）——2 处文档一致性更正 +
> 2 处测试补强（本记录 §1.2 的跨 trigger 负例、§3 未含的「中断后恢复重触发」
> 端到端钉子）；见 [`0X S2 复审处理`](0X_S2_REVIEW_HANDLING_2026-09-11.md)，
> 子模块 `dcf4a774`。

## 1. 事件面（四处）

### 1.1 payload 正例（负例沿用既有）

- 新增 `runtime/fixtures/run-event-v0.2/payloads/orientation-checkpoint.initial-round.valid.json`：
  `trigger=initial_round` + `[INITIAL_ROUND_INQUIRY v0.1]` 块 + `post_tool_batch_gap`
  三点对齐（一次性初始轮问询真正的载荷形状）。
- **负例沿用既有** `orientation-checkpoint.constraint.invalid.json`（`trigger=manual`
  非枚举值）——schema 枚举收紧后它仍是同一约束下的负例，不需要第二份。
- 生成器 `scripts/generate_run_event_fixtures.py` 增补 `EXTRA_V02_PAYLOAD_POSITIVES`
  条目（fixture 可重复生成）；`scripts/check_repository.py` 显式登记该正例契约（否则
  v0.2 payload fixture 集合完备性检查会判 unmapped）。

### 1.2 法官族 `initial_round_inquiry`（第 34 族）

schema 表达不了「trigger ↔ message_block ↔ injection_position」与「会话内恰好一次」，
故按 Task D 纪律做成**家族级**规则，双方同源：

| 侧 | 落点 | 内容 |
|---|---|---|
| Rust（执法） | `orz-assurance/src/journal/families_s2c.rs::verify_initial_round_inquiry`；注册进 `verify_s2c_family`、`S2C_FAMILIES`、`ALL_FAMILIES`（33 → **34** 族） | ①`trigger=initial_round` ⇒ 块前缀 `[INITIAL_ROUND_INQUIRY` + 位置 `post_tool_batch_gap`；②其它 trigger 不得携带初始轮块；③同一 (`session_id`,`agent_role`) 在单本期刊内至多一次初始轮 fire |
| Python（冻结对照） | `assurance/run_event_journal_validation.py::_verify_v02_initial_round_inquiry`；在 `validate_journal_text` 中紧随 `_verify_v02_inquiry_kind` 调用 | 同规则 |

**跨执法面对拍**：`s2b_family_verdicts_match_python`（Rust ↔ Python 逐格判决一致，
语料 = 合成场景 + 全部真实 v0.2 期刊）在 34 族 × 语料上零差。

**合成场景正负例**（`families.rs` scenarios + expected_violations）：
`initial_round_ok`（期望零违规）、`initial_round_wrong_block`、
`initial_round_wrong_position`、`initial_round_under_periodic_trigger`、
`initial_round_twice`（后四条各锁一条违规）。场景总数登记值 244 → **249**。

### 1.3 期刊重捕 `orientation-fire-run.jsonl`

0t 重捕后本场景是 auto_close-only（无 disposition 往返）。S1 改变了该场景行为
（首个动作批次多一次初始轮 fire），故按「重捕小批」纪律**重捕本期刊**：

- Rust capture 期望序列补齐：第 1 轮动作批次后新增一条 `orientation_checkpoint`
  （与周期问询同位置：夹在两条 `mechanical_audit_update` 之间）；载荷断言按
  `trigger` 取（不再取「第一条」）。
- 重捕结果：**两条** `orientation_checkpoint`——`initial_round`
  （`completed_turns_since_orientation=1`）与 `completed_turns_interval`
  （`=7`）；其余 11 本期刊不动。
- Python 侧同步：`_orientation_fire_v02_sequence()`（本轮新增期望）、
  `test_orientation_fire_payload_is_v02_shape`（按 trigger 取）、
  `test_orientation_fires_exactly_once_in_seven_rounds` →
  `test_orientation_fires_once_per_trigger`（两条 fire 各自恰好一次）。

### 1.4 门禁

`scripts/check_repository.py` → `valid: true` / `error_count: 0`
（`orz_source_manifest_files: 1441`）；全部 v0.2 期刊逐行过 envelope + payload
schema + Rust 法官。

**顺序注意**：`orz_source_manifest.sha256` 哈希的是子模块 **HEAD blob**
（`git cat-file HEAD:<path>`，与本地 autocrlf 无关），因此必须在 orz 子仓库
**提交之后**再重算——脏工作树上生成的 manifest 会在提交后立刻判为 digest 漂移。

## 2. 签名侧第二模板摘要（`kind + 摘要匹配`）

### 2.1 机制

初始轮问询复用 `OrientationV1` 票据类型（用户裁决），但**注入的块与周期问询不同**——
原实现只钉一个 `ORIENTATION_TEMPLATE`，初始轮块的票据绑定会是空的。S2 让签名侧持有
**两块内置模板**，并按下述方式做到「票据校验按 kind + 摘要匹配」：

1. `orz-signer` 新增 `INITIAL_ROUND_INQUIRY_TEMPLATE` 常量（单一来源 =
   `orz_assurance::orientation::checkpoint::INITIAL_ROUND_INQUIRY_BLOCK`）与
   `template_sha256_initial_round` 会话字段；
2. `sign_orientation_v1` 请求新增**可选** `trigger` 参数：`initial_round` → 初始轮摘要，
   其余/缺省 → 周期摘要（非 orientation 票种无模板字段，trigger 无意义）；
3. `initialize_session` 响应新增 `template_sha256_initial_round`（旧字段 `template_sha256`
   保留为周期摘要，向后兼容）；
4. 客户端（`orz-loop/src/acaf.rs`）缓存两个摘要；`verify_and_consume` 新增 `trigger` 参数，
   **check 2 用与签发同一个 trigger 选出的内置摘要**做比对——既不是票据自证，也不再
   只有周期块被绑定；
5. `ticket_flow` 从 args 取 `trigger` 同时传给签发与校验（单一输入 → 两侧一致）；
   `maybe_fire_orientation` 把 `rec.trigger` 放进票据 args（并因此进入
   `canonical_arguments_sha256`）。

### 2.2 边界

- **fail-closed 版本错配**：若某环境里 loop 与其自带 signer 不同版本（signer 无第二摘要），
  初始轮票的 check 2 会拒（`template_mismatch`），fail-closed 下该次注入被拦。签发与校验
  同源（同一 signer 会话字段）故同版本路径不可达——这是**有意的 fail-closed 姿态**：
  签名侧不认识该块时不该放行。
- **不改**票据 schema、不加票据种类、不改 `VerifyContext` 形状（check 2 仍比对单一值，
  只是该值按 trigger 选取）。

## 3. 顺带修复（本刀发现，均非 0x 语义）

1. **`family_stage_tamper_detected_end_to_end` 预存在失败**（改动前干净树即失败）：
0t 重捕后期刊不再含 `retrieval_parent_disposition`，原篡改循环空转 → 「篡改后仍
valid」。现改篡改**初始轮问询的 `injection_position`**（schema 合法、仅家族阶段能抓），
并把断言改为匹配该族错误文案——同时为新的 `initial_round_inquiry` 族提供端到端
（stage-5 门控）负证据。转绿。
2. **fixture 生成器缺失 0v 两条条目**：`tool-completed.serp-budget-exceeded.valid` 与
`tool-completed.serp-session-reserved.valid`（0v P2-3/P2-4 落地时只加了文件、没登记
生成器），**重跑生成器会静默删除它们**——本刀重跑时暴露并已补登（生成器现幂等）。
3. **capture 路径的 ACAF 环境说明**：无 signer fabric 时 capture 必须
`ORZ_ACAF_FAIL_CLOSED=0`（否则 fail-closed 默认在 bin 测试二进制里拒跑）；已在
`conformance_capture` 模块头登记，重捕可复现。

## 4. 验证证据

- `cargo test -p orz-assurance`：lib 211 + fixture_journal_conformance 9 +
  journal_conformance_cli 4 + 其余 10 全绿（**含此前失败的篡改端到端**）。
- `cargo test -p orz-loop`：762 全绿。
- `cargo test -p orz-bin`：12 / 2 / 15 / 23 / 2 / 1 全绿（signer 单测 15 含新增触发
  选择用例；acaf_e2e 23 含新增初始轮票据签验用例）。
- `cargo test -p orz-host --lib -- --test-threads=1`：286 全绿。
- `python -m pytest runtime/tests/test_run_event_journal_validation.py
  runtime/tests/test_run_event_conformance.py`：273 passed。
- `cargo clippy` 新增代码零告警（`sign_ticket` 第 8 参数按先例登记 allow；
  其余告警落在未改动的既有行）。
- 重捕自检：`capture_orientation_fire_run` 的链校验 + 精确事件序列全过。

## 5. 未做（明确边界）

- **未**跑实机、**未**重建双平台载体（S3/S4 需另行走放行门）。
- **未**改周期问询阈值/三问；**未**改终答前审查报告与反例门；**未**新增 consumption 事件。
- 其余 11 本 v0.2 期刊**未**重捕（仅行为变化的 `orientation-fire-run` 换新）。
