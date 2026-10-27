# 367 批：0ds M1-S1 state 分区与 state_read/state_write 落码

> 日期：2026-10-27
> 用户令：「请继续，这个应该是修改后出现的bug，后续得再改一下，不过当前先继续M1-S1吧」。
> 上游：[`366 章程`](366_AGENT_OS_V1_PROJECT_CHARTER_2026-10-27.md) / [`架构设计`](../ORZ_AGENT_OS_ARCHITECTURE_V1_2026-10-27.md) / [`状态/回中设计`](../STATE_RECENTER_AND_SYSTEM_LAYER_DESIGN_2026-10-27.md)。
> 后续：[`368 批`](368_SEMANTIC_SUMMARY_CONTROL_TURN_2026-10-27.md) 修复「纯语义摘要控制轮被当终答」的 run 半路收口；本批 state 语义不变。
> 性质：M1 基础内核的 **S1 切片**（state 落码 + 测试）；`context_map` / `recenter` / capability / resident 清理留给后续切片。工作树改动，**未提交、未推送、未重建载体、未换装**。
> 审查：[`370 M1-S1 与 369 审查`](370_M1_S1_AND_369_REVIEW_2026-10-27.md)（条件通过；state_history 表达 P2、人类 pane P3）。

## §0 结论

- **S1 完成**：`state` 分区（6 字段、≤200 token、合并写、系统 `updated` 盖章）＋ `state_read/state_write` 两工具＋`blackboard_read/write section=state` 兼容入口＋journal `state_write` 事件。
- **验收读数**：`cargo test -p orz-loop --lib` **932 passed / 0 failed / 3 ignored**；`cargo test -p orz-assurance` **310 + 9 + 4 + 9 + 1 全绿**；`cargo test -p orz-host --lib -- --test-threads=1` **453 passed / 0 failed / 6 ignored**；定向 Python **123 passed**；`cargo check --workspace` Finished；新增 schema 单测与 Python 参照 payload 复核 0 错。
- **M1 未完**：`context_map`（≤1K）、`context_manage(mode=recenter)`（只读 ≤1K）、capability 薄层、其余 journal 事件、resident 清理仍在后续切片。

## §1 落地内容

### 1.1 state 模型（`orz-loop/src/state.rs`，新文件）

- `StateSection`：固定序 `goal / now / next / open / constraints / updated`；每字段 serde default（旧板/旧会话快照零迁移）。
- `StateUpdate`：**合并语义**——只改提供的字段；空串 / `none` / `无` 清空单字段；`open`/`constraints` 限 0–3 条、单条 ≤200 字符。
- 预算：`render()` 的 chars/2 估算 ≤ **200 token**（含 `state v1` 表头与字段名；与 `compact::estimate_message_tokens` 同尺）。预算拒绝**不改变原状态**（先克隆再提交，原子写）。
- 文档形态：`blackboard_write section=state` 用 `key: value` 行；允许 `state v1` 头、空行、`#` 注释；未知 key / 缺冒号 fail-closed；`updated` 出现时忽略（系统写）。
- `updated` 只在成功写入时由机械层盖章：`r{round}@{domain} {ts}`。

### 1.2 黑板与工具面

- `Blackboard.state` 字段 + `PartitionRevisions.state`；`render_state_section` / `write_state` / `write_state_document` / `replace_state` / `clear_state`。
- `state_read`：窄读面（无参数），与服务端 `blackboard_read section=state` 共用渲染/信封单源。
- `state_write`：窄 schema（goal/now/next/open/constraints；未知字段拒绝）；无条件声明，ReadOnly 类。
- `blackboard_write section=state`：`append/replace_all` 写入、`clear` 清空；`section=all` 仍只清 plan/notes/findings，**state 不在 all 内**（显式 `section=state` 清）。
- `blackboard_read section=state`：live 渲染；空分区返回 `（无）`；非法 section 错误文案补 state。
- 三处工具表同步：`tool_probe::WORK_TOOLS`（26→28）、assurance `toolsets::WORK_TOOLS`、Python `_WORK_TOOLS`；`tool_probe` 判据共用 `probe_storage`。
- 权限：`ToolDispatcher::READ_ONLY_EXEMPT_TOOLS` 与 host `permission.rs::access_kind` 两表同批登记；跨表护栏样本补两项。
- action 折叠：`state_write` 归 `other`（内存簿记），`state_read` 归 `read`。
- capability 清单：`orz-host/tools.rs` `CAPABILITY_COVERAGE` ④ 段点名 `state_read/state_write` 未覆盖（host 无 handler，schema 单源在 loop controller）。

### 1.3 journal

- 新 `EventType::StateWrite`；v0.2 envelope enum + `runtime/state-write-event-payload-v0.2.schema.json` + payload registry 条目。
- 成功写入 payload：`outcome/fields/goal(≤120)/content_chars/estimated_tokens(≤200)/updated`；`clear` 为 `outcome=cleared, fields=[], chars/tokens=0`。
- 失败写入不落 `state_write`（ToolCompleted exit 1 留痕），不写半条。
- TUI 对 StateWrite 降级 Unknown（无专用卡片，机械事实族同姿态）。

## §2 测试与验证

| 面 | 读数 |
|---|---|
| `orz-loop --lib` | 932 passed / 0 failed / 3 ignored |
| `orz-assurance` 全套 | 310 lib + 9 fixture + 4 CLI + 9 v1 + 1 doctest，全绿 |
| `orz-host --lib` 串行 | 453 passed / 0 failed / 6 ignored |
| Python 定向 | 123 passed（probe accuracy / tool availability / plan mode / p0 contracts） |
| Python 参照 payload | `_payload_errors(state_write)` = `[]` |
| `cargo check --workspace` | Finished（exit 0） |
| `cargo fmt` | 本批文件已格式化；仓库级 `--check` 仍有 2 个**既有**文件漂移（`grok_build/read_file/mod.rs`、`agent_loop.rs`），非本批引入，未纳入改动 |

新增钉子：
- `state::tests` 7 条：空态、合并/盖章、清空、预算原子拒绝、文档回环、未知字段/畸形、条目上限。
- `blackboard::tests` 2 条：写入/替换/清空/会话快照回环；`state_write → state_read` 真实工具链 + journal 事件。
- `conformance::tests` 1 条：registry→schema 真校验（accepted/cleared/超预算/未知字段）。
- 既有面预期更新：Benchmark/ReadOnly/retrieval projection 工具列表、`tool_probe` 26→28、`blackboard_write` section enum 五值、会话快照 state 版本计数。

## §3 边界与已知问题

- **未提交**：按项目纪律等用户令再提交/推送/重建/换装。
- **宿主全量并行**：7 条超时/截断/ACP 用例在并行下互相干扰（单独跑或串行全绿）；本批未改宿主超时/截断链，判为测试环境并行抖动，不作为回归。
- **TUI 全量**：9 条 ACP/e2e 用例因本机未设 `ORZ_ACAF_MANIFEST`/`ORZ_ACAF_KEYSTORE` 被 ACAF fail-closed 拒绝；环境前置缺失，非本批改动。
- **fmt 基线**：仓库级 `cargo fmt --check` 有 2 个既有漂移文件；本批未顺手改，避免混入无关 diff。
- `state` 不常驻注入；模型只在任务变化时主动写、在需要时 pull。`context_manage` 本批未动，M1.5 再加 map/recenter/edit。
- `state` 不进 plan epoch 快照（跨轮换保留）；会话快照/侧车携带 state（跨压缩保留）。

## §4 下一步

1. **M1-S2**：`context_map`（块号/轮次/token/状态/回放路径；≤1K token；不常驻）＋块 ledger/marker 对账测试。
2. **M1-S3**：`context_manage(mode=recenter)`（state + 最近 N 轮摘要 + next；只读；≤1K）＋“无残留常驻文本”验收。
3. **M1-S4**（可与 S2/S3 并行）：capability registry 薄层＋`capability_*` journal 事件；`checkpoint` 事件。
4. 之后进 M2 resident 清理（permission / plan_framework / context permission）与 M3 验收。

## §5 入口

- 代码：`orz/crates/orz-loop/src/state.rs`、`blackboard.rs`、`controller.rs`、`host_exec/tool_run.rs`、`tool.rs`、`tool_probe.rs`；`orz/crates/orz-assurance/src/journal/{event,conformance}.rs`、`tool_names.rs`；`orz/crates/orz-host/src/permission.rs`；`runtime/state-write-event-payload-v0.2.schema.json`、`runtime/run-event-payload-registry-v0.1.json`、`runtime/run-event-v0.2.schema.json`；`assurance/run_event_journal_validation.py`。
- 状态文档：`TODO.md` `P1-0ds`；`docs/BACKLOG_AND_PRIORITIES.md` `0ds`；`CLI_PROJECT_INDEX.md` `DESIGN-AGENT-OS-V1`。
