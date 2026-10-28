# 375 批：M1-S4 journal context/checkpoint + M1 验收矩阵

> 后续批：**376** 已把 checkpoint producer、capability registry、provider tokenizer 与 resident 清理补齐；本批中“checkpoint schema-only / capability 留 M2”的表述以 [`376 批档`](376_M1_V1_FULL_REMEDIATION_2026-10-28.md) 为准。

> 日期：2026-10-28
> 上游：[`366 章程`](366_AGENT_OS_V1_PROJECT_CHARTER_2026-10-27.md) §1 item 5 / [`ORZ_AGENT_OS_ARCHITECTURE_V1_2026-10-27.md`](../ORZ_AGENT_OS_ARCHITECTURE_V1_2026-10-27.md) §13-5 / [`373`](373_CONTEXT_MAP_2026-10-28.md) / [`374`](374_RECENTER_2026-10-28.md)。
> 性质：M1 基础内核 **S4 切片**＋M1 路线最小闭包验收。工作树改动，**未提交、未重建、未换装**。capability/resident 按 366 §2 留 M2。

## §0 结论

1. **`context_pulled` 已落码并有 producer**：`context_manage(mode=map|recenter)` 成功分支各写一条只读拉取审计行。payload 只带模式、预算读数、块/轮计数与输出摘要，**不复制渲染文本**。
2. **`checkpoint_created` 已登记、无 producer**：v0.2 enum、payload registry、payload schema、正/反 fixtures 全齐；按 366 §2/M4 边界，checkpoint 落盘与 producer 等 orz-system 切片接线。
3. **M1 路线最小闭包完成**：state（S1）、context_map（S2）、recenter（S3）、journal context/checkpoint（S4）全部落码；M1 验收矩阵见 §3。366 §1 六项全量读法下的 capability/resident 仍按 §2 留 M2，不在本批越界。

## §1 payload 形态

### §1.1 `context_pulled`（producer：map/recenter）

```json
{
  "mode": "map",
  "read_only": true,
  "token_estimate": 210,
  "blocks": 3,
  "recent_rounds": 0,
  "output_sha256": "<64 hex>",
  "output_chars": 420
}
```

- `mode=map`：`blocks`＝紧凑分块表实际块数，`recent_rounds` 固定 0。
- `mode=recenter`：`blocks` 固定 0，`recent_rounds`＝实际完整轮数（0..5）。
- `token_estimate` ≤1024（M1 硬预算，chars/2）。
- `output_sha256` 对模型实际收到的渲染文本取 SHA-256；文本仍在工具结果里，journal 不存正文。
- `read_only` 恒 true；schema `additionalProperties:false`，并用 if/then 钉死 map⇒recent_rounds=0、recenter⇒blocks=0。

### §1.2 `checkpoint_created`（schema-only，M4 producer）

```json
{
  "checkpoint_id": "CKPT-CONF-0001",
  "generation": 1,
  "state_sha256": "<64 hex>",
  "path": ".gsa/checkpoints/CKPT-CONF-0001.json"
}
```

- 本批只登记 schema/registry/fixtures；`EventType::CheckpointCreated` 已存在，但**没有任何代码写它**。
- 批档明确：producer 与真实 checkpoint 存储由 M4 orz-system 切片接线；M1 不伪造落盘事实。

## §2 落码

| 文件 | 改动 |
|---|---|
| `orz/crates/orz-assurance/src/journal/event.rs` | `EventType` 增 `ContextPulled`、`CheckpointCreated`；serde snake_case 自动得到 `context_pulled` / `checkpoint_created` |
| `runtime/run-event-v0.2.schema.json` | event_type enum 增两项 |
| `runtime/context-pulled-event-payload-v0.2.schema.json` | 新建：mode/read_only/token_estimate/blocks/recent_rounds/output_sha256/output_chars；≤1024、if/then 模式耦合 |
| `runtime/checkpoint-created-event-payload-v0.2.schema.json` | 新建：checkpoint_id/generation/state_sha256/path |
| `runtime/run-event-payload-registry-v0.1.json` | v02 增 `context_pulled`、`checkpoint_created` |
| `runtime/fixtures/run-event-v0.2/payloads/` | 新增 2 正例 + 2 反例（反例各只违反一条约束） |
| `runtime/fixtures/run-event-v0.2/envelope/` | 新增 2 个完整 v0.2 envelope |
| `orz/crates/orz-loop/src/host_exec/tool_run.rs` | map/recenter 成功分支写 `ContextPulled`；输出摘要/字符数由实际输出计算 |
| `orz/crates/orz-loop/src/agent_loop.rs` | map/recenter 端到端测试增断言：事件存在、mode/read_only/blocks/recent_rounds、output_sha256/output_chars 与实际输出一致 |
| `orz/crates/orz-assurance/src/journal/conformance.rs` | 新测：registry 解析 + 两 schema 正/反例（预算、read_only、模式耦合、generation） |
| `orz/crates/orz-tui/src/bridge.rs` | 两事件归 `TuiEvent::Unknown`（机械审计事实，无专用卡片） |
| `scripts/generate_run_event_fixtures.py` | 补 `state_write`（M1-S1 欠账）＋两项新事件的 generator 表；`_assert_generator_tables_match_sources()` 通过（v02=46） |
| `runtime/fixtures/run-event-v0.2/README.md` | 增 M1-S1/S4 事件说明 |
| `assurance/run_event_journal_validation.py` | registry 计数注校准 v02=46（M1-S1 state_write + M1-S4 两项） |

## §3 M1 验收矩阵

| 验收项 | 结果 | 证据 |
|---|---|---|
| state 分区 6 字段、空值显式、≤200 token | ✅ | 367/371 批；`state.rs` 单测；loop 全量 |
| state 合并/清空/原子写/跨压缩保留 | ✅ | 367/371 批；`blackboard`/`state` 测试 |
| `state_read` / `blackboard_read section=state` 读回环 | ✅ | 367 批；loop 全量 |
| context_map 块号/轮次/token/状态/seq/pointer/回放路径 | ✅ | 373 批；`context_manage_map_is_read_only_and_returns_compact_map` |
| context_map ≤1K、不常驻、只读、参数拒绝 | ✅ | 373 批；map 端到端断言 |
| recenter state + 最近 5 完整轮 + 显式 next | ✅ | 374 批；`context_manage_recenter_returns_state_recent_and_next_read_only` |
| recenter ≤1K、不常驻、无 `ContextCompressed`、参数拒绝 | ✅ | 374 批；recenter 端到端断言 |
| journal `state_write` 已登记并有 producer | ✅ | 367/371 批；assurance conformance |
| journal `context_pulled` 已登记并有 producer | ✅ | 本批；map/recenter 端到端断言 + Rust schema 测试 |
| journal `checkpoint_created` 已登记、无 producer | ✅（M4 接线） | 本批；schema/registry/fixture + 批档声明 |
| v0.2 enum / registry / payload schema / fixtures 四面对齐 | ✅ | Python 定向 292；runtime conformance 17 |
| 故障消融：state 缺失/满、map 漂移、recenter 循环/预算、state 膨胀 | ✅ 记录现状 | state 空/满/清空与预算测试；map 对账；recenter 预算/最近轮断言；未新增未证实故障路径 |
| capability 薄层 / resident 清理 | ⏸ M2（366 §2） | 本批不碰权限、工具面、prompt 指纹 |

## §4 测试

| 门禁 | 结果 |
|---|---|
| `cargo test -p orz-loop --lib` | **958 passed / 0 failed / 3 ignored** |
| `cargo test -p orz-host --lib -- --test-threads=1`（`RUST_MIN_STACK=16777216`） | **453 passed / 0 failed / 6 ignored** |
| `cargo test -p orz-assurance` | **311 passed / 0 failed**；fixtures **9 / 4 / 9**；doctest **1** |
| `python -m pytest runtime/tests/test_run_event_journal_validation.py runtime/tests/test_run_event_conformance.py -q` | **292 passed** |
| TUI `cargo test -p orz-tui --lib bridge::tests` | **8 passed / 0 failed** |
| `cargo fmt -p orz-loop -p orz-assurance -p orz-tui -- --check` | 全绿 |
| `cargo check --workspace` | `Finished` |
| `git diff --check`（两仓） | 干净 |

说明：
- host 的 `session_terminal_backend_survives_host_rebuild_and_close_teardown` 在默认测试线程栈上会 `STATUS_STACK_OVERFLOW`（并发/串行均可复现）；提高 `RUST_MIN_STACK` 后该用例与全量串行套件通过。判定为测试线程栈环境问题，与本批改动无因果关系（本批未触碰 host 终端后端）。
- 全量 `python -m pytest assurance/tests` 在本机超过 7 分钟未收口（存在环境依赖型慢测），本轮以 journal/conformance 定向 292 与 assurance Rust 311 为门禁；未把未收口套件写成全绿。

## §5 边界与未决

- 不实现 capability 薄层、preopen、resident 清理；不新增/改权限表。
- 不实现 checkpoint 真实落盘/恢复；`checkpoint_created` 仅 schema 登记，M4 接线。
- 不新增常驻 prompt 文本；`context_pulled` 不进模型面，只是 journal 审计行。
- 未提交、未重建、未换装；0.9.14-3 在役载体不含 S2/S3/S4。
- 已知文档债：`runtime/fixtures/run-event-v0.2/README.md` 的“69 events”历史计数未逐字重写（本批只加 M1 说明）；后续统一整理时校准。

## §6 入口与下一步

- 代码：`event.rs`、`conformance.rs`、`tool_run.rs`（map/recenter 分支）、`agent_loop.rs`（端到端）、`bridge.rs`。
- 合约：`runtime/context-pulled-event-payload-v0.2.schema.json`、`runtime/checkpoint-created-event-payload-v0.2.schema.json`、registry、fixtures。
- 下一步：M1 路线最小闭包已收口；按 366 §2 进入 M1.5（decisions/context edit/slider）或 M2（capability/resident），由用户裁决。提交/重建/换装等用户令。
