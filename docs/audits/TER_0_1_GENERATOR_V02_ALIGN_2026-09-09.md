# TER-0.1 生成器 v0.2 表全面对齐（2026-09-09）

> 上级：[BACKLOG2 TER-0.1](../BACKLOG2.md)；改动入口：
> [generate_run_event_fixtures.py](../../scripts/generate_run_event_fixtures.py)。
> 状态：TER-0.1 专项完成——生成器全树重建与已验收夹具树 **0 差异**
> （含 canonical_cli 与 v0.2 README）；此后夹具改动一律先进生成器表再重建。

## 1. 对齐前缺口（试跑对拍取证，2026-09-09）

以提交态夹具树为基准，把生成器复制到临时目录干跑后逐目录对拍
（payloads / envelope / README / canonical_cli），缺口清单：

- **事件枚举**：`V02_EVENT_TYPES` 54 项 vs `run-event-v0.2.schema.json`
  enum 55 项——缺 `session_archive`（BLACKBOARD-CONVERSATION-SCOPE-FOLD
  B3，2026-09-03，ADR-0010 §14.52；含 payloads minimal/constraint 与
  envelope 三个文件）。
- **payload 额外夹具未入表（15 个文件）**：`retrieval-close-record`
  auto-close / subagent-timeout（正例）与 bad-effort（反例）；
  `tool-completed.dep-graph-*` read / write（正例）与
  bad-kind / extra-field（反例，P2-11 依赖图主线，ADR-0010 §14.51）；
  `tool-completed.failure-target-*` anchor / cmd / file / url（正例）与
  bad-id / bad-kind（反例，0q 统一失败事件管线，ADR-0010 §14.63）。
- **信封样例漂移（4 个文件）**：`mechanical-audit-update`（run_id +
  时间戳 + payload 样例分轨）、`tool-running`（时间戳 2026-08-29）、
  `transport-retry`（payload 样例 recovered/midstream 与 minimal
  exhausted/zero_chunk 分轨）；`retrieval-close-record` payload 最小正例
  已含 `effort` 字段而信封样例未跟进（P2-13 B3 契约形态）。
- **README**：`FIXTURES_README_V02` 文案落后于已验收 README（P2-11 DC
  清理条目、0t browser_launch_result 条目、事件计数 52→55、缩进与
  journal 表尾行差异）。

## 2. 生成器表改动

- `V02_EVENT_TYPES` / `SLUGS_V02` / `V02_PAYLOAD_EVENTS` 补
  `session_archive`（按 schema enum 顺序插在 `epoch_archive_write_failed`
  与 `plan_write` 之间）。
- `PAYLOAD_GOOD_V02` / `PAYLOAD_BAD_V02` 补 `session_archive` 正反例；
  `PAYLOAD_GOOD_V02["retrieval_close_record"]` 补 `effort: "standard"`。
- `EXTRA_V02_PAYLOAD_POSITIVES` +8（retrieval-close ×2、dep-graph ×2、
  failure-target ×4）；`EXTRA_V02_PAYLOAD_BADS` +5（bad-effort、
  dep-graph bad-kind / extra-field、failure-target bad-id / bad-kind）。
- `V02_ENVELOPE_TIMESTAMP_OVERRIDES` 补 `tool_running`（2026-08-29）与
  `session_archive`（2026-09-03）；`V02_ENVELOPE_IDENTITY_OVERRIDES` 补
  `mechanical_audit_update`（RUN-CONF-MECH-AUDIT + 2026-08-24）。
- 新增 `V02_ENVELOPE_PAYLOAD_OVERRIDES`：信封样例与 payload 最小正例
  分轨的三个事件（mechanical_audit_update / retrieval_close_record /
  transport_retry）在生成信封时优先取本表，注释说明各样例出处；
  信封装配回落逻辑同步更新。
- `FIXTURES_README_V02` 与已验收 `README.md` 逐字节同源（含 P2-11 DC
  清理、0t browser_launch_result、55 事件计数与 journal 表）。

## 3. 验证

- 临时目录干跑 + 仓库内实跑均 `GEN_EXIT=0`：
  `payloads: 68 files, envelope: 48 files, canonical_cli: 7 files,
  v0.2 payloads: 85 files, v0.2 envelope: 71 files`。
- 重建后对拍 **0 差异**：v0.1/v0.2 payloads + envelope + README 与
  canonical_cli 全部逐字节一致；`git status` 在实跑后除生成器脚本外
  零变更（即提交态夹具树 == 生成器输出）。
- `python -B -m pytest -p no:cacheprovider -q
  runtime/tests/test_run_event_conformance.py
  runtime/tests/test_run_event_journal_validation.py` → **273 passed**。
- `git diff --check` exit 0。

## 4. 纪律注记

TER-0.1 定义的目标自此生效：夹具改动（含 payload 样例、信封身份/时间戳/
样例分轨与 README 文案）一律先进生成器表再重建，不再手工维护夹具文件。
journals 目录由 orz conformance capture 产出，不属生成器管理范围（既有
纪律不变）。
