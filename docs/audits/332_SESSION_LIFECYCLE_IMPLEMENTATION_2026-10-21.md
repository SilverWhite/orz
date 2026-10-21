# 332 批：0dk S2 会话生命周期落码（close／有界 32／删除不复活／占用者提示）（2026-10-21）

> **用户令**：「请开始进行0dk落码部分吧」。
> **本批性质**：332 = 实现批（S2 落码＋11 条单测＋前端 smoke）；**不重建载体、不换装、不发行**（331 计划件 §1 的批次序）。训练进程在批次开始时仍存活（PID 13616），收尾前已自然结束；本批仍未跑 `cargo build --release`／docker／载体重建。
> **来源**：331 的 0dk S1（[`设计稿`](../SESSION_LIFECYCLE_OWNERSHIP_AND_CLOSE_DESIGN_2026-10-21.md)）；331 补记的批次序（[`332–333 计划`](../WEB_CONSOLIDATION_BATCH_SCHEDULE_2026-10-21.md) §3）；330 契约基线（[`330 档`](330_SURFACE_CONTRACT_BASELINE_2026-10-21.md)）S2/S3。
> **计数**：**64 不变**（实现批；0dk 仍 `partial`，S4 真机收口后才可能翻 `implemented`）。

## §0 结论速览

| # | 332 范围 | 结果 | 关键证据 |
|---|---|---|---|
| 1 | `_orz/session_close`（先 cancel、10s、幂等、busy 不半关） | **落码**：`session_ops::close` 传输无关驱动＋`session_close_with_timeout`；ACP 薄适配器 | 6 条 `session_ops` 单测；`close_idle`／`close_active`／`close_times_out`／`close_is_idempotent` |
| 2 | 会话表有界 32＋LRU 驱逐 idle（`ORZ_SESSION_CAP`） | **落码**：插入后按 `last_prompt_at` 驱逐无在途 run 的 LRU；active 永不驱逐 | `sessions_cap_evicts_lru_idle_only` |
| 3 | 删除不复活（prompt 禁隐式物化＋`.deleted` tombstone） | **落码**：prompt/recall/queue/restore 前执法；侧车写入拦截；CLI 删除先写墓碑再删数据 | `prompt_after_delete_returns_not_found_without_materializing`／`delete_active_is_rejected` |
| 4 | 占用者提示（桥 409 带原因＋Web 文案） | **落码**：409 JSON 四要素；Web 侧可行动文案（原生 WS 读不到 409 body） | `server.rs` 409；`acp.js` 连接失败文案 |
| 5 | `_orz/session_state` 状态通知 | **落码**：close/evict 经 ACP gateway 发 `ExtNotification`（线格式 `_orz/session_state`） | `session_state_notification_emitted_on_transition` |
| 6 | 关闭入口 | **落码**：工具栏「关闭会话」（运行/恢复中禁用）；换会话走 `close(switch)` | `session_close_ext_method_roundtrip`；smoke 4b/14.8 |

## §1 范围与不做

- **做**：`session_ops` 状态机与错误段；ACP 适配器（close/state/delete）；会话表上限与 LRU；删除墓碑与多路径执法；占用者 409 原因；Web 关闭入口/状态通知；11 条 Rust 单测＋smoke 断言。
- **不做**（设计 §13／计划 §3.3）：跨进程恢复、force kill、ACP v1 升级、TUI 改动、多写者、文件级撤回、真实载体重建/真机。
- **边界**：不动 vendored `xai-acp-lib`；`_orz/*` 只做 orz 自有扩展；未触碰 `D:\AGI`。

## §2 改动明细

### §2.1 新模块 `orz/crates/orz-host/src/session_ops.rs`

- `SessionState`（created/idle/active/closing/closed/archived/deleted）、`CloseReason`（user_close/switch/evict/shutdown）、`SessionCommand`（Close/State/Delete）、`SessionOutcome`（camelCase 线格式）。
- `SessionError`：`session_not_found`(-32010)／`session_busy`(-32011)／`invalid_state`(-32012)／`lease_conflict`(-32013)／`attach_conflict`(-32014)／`internal`(-32603)；每项带 `class/hint/evidence` 四要素，`data()` 直接产出 `error.data`。
- `SessionHost` trait：`state_of/cancel/release/run_evidence/is_deleted/is_closed/lease_held_elsewhere`——不出现任何 ACP 类型（换底座只重写适配器，设计 §11）。
- `close(host, id, reason, timeout)`：active 先 cancel；25ms 轮询到终态；超时返回 busy 且**不移除**；内存缺失时按 tombstone＞lease＞closed marker＞not_found 的顺序裁决；`release` 的 `Busy` 也映射 busy（查态与移除之间的 run 窗口）。
- `state_of(host, id)`：tombstone→deleted；内存态→原样；closed marker/侧车→already closed；否则 not_found。
- 6 条模块内单测（idle/active/timeout/idempotent/tombstone/lease＋四要素 data）。

### §2.2 `orz/crates/orz-host/src/acp_server.rs`

- `AcpError::SessionLifecycle(#[from] SessionError)`——ACP 层只透传。
- `AcpServer` 新字段：`session_cap: usize`（`ORZ_SESSION_CAP`，默认 32，0＝不限）、`default_base`（本 child 归属根，供未知 id 查磁盘标记）。
- `StoredSession` 新字段：`last_prompt_at`（LRU 刻度，prompt 起点刷新）、`closing`（关闭登记，prompt 让路）。
- 持久标记（`.gsa/sessions/<s8>.*`）：`.closed`（close/evict 的幂等依据）与 `.deleted`（删除墓碑）；`session_lease_held_elsewhere` 只探测不创建 lease 文件。
- `LifecycleHost` 适配 `SessionHost`；`release_session` 在 sessions 锁内复核 runs（sessions→runs 锁序，prompt 的 guard 不跨锁等待）。
- `handle_session_close` / `_with_timeout` / `handle_session_state` / `handle_session_delete`；`evict_over_cap` 在 attach 插入后执行，走同一 `finish_closed`（不归档、发状态通知、写 closed 标记）。
- 删除不复活：`attach_session_with_options` 先查 tombstone；`handle_session_prompt`／`handle_queue_append`／`handle_recall_preview`／`handle_recall`／`restore_snapshot` 入口统一 `refuse_deleted_session`；在途 run 不拆（只拒新操作，侧车由 `persist_conversation_sidecar` 的 tombstone 检查拦截）；`delete_session_on_demand` 先写墓碑再删数据，并把 `.closed` 纳入删除口径。
- 11 条验收单测（见 §3）＋既有 `close_session` 语义不变（codex 线程仍走归档路径）。

### §2.3 `orz/crates/orz-host/src/stdio.rs`

- `_orz/session_close`／`_orz/session_state`／`_orz/session_delete` 三个 ExtRequest 分支；参数兼容 `sessionId`（设计线格式）与 `session_id`（0di 既有面）；`SessionError` → JSON-RPC 错误（`code` + `data` 四要素）；未知 `_orz/*` 仍 `method_not_found`。

### §2.4 `orz/crates/orz-web`

- `src/acp_pump.rs`：`_orz/session_close|delete` 成功后清槽的 `session_id`/`prompt_in_flight`——刷新后的 attach preamble 不再把已关闭 id 当活跃会话。
- `src/server.rs`：409 响应带 `reason=workspace_occupied`、`class=connection`、`hint`、`evidence=workspace=<cwd>`；浏览器原生 WebSocket 读不到握手响应体，故前端另有同义文案。
- `assets/app/state.js`：工具栏「关闭会话」（紧随「停止」，不破坏「新建会话→后退」冻结序）；`syncToolbar` 在运行/恢复/无绑定时禁用。
- `assets/app/acp.js`：`sessionClose`／`sessionState`／`sessionDelete`；`_orz/session_state` 通知回调；连接失败文案改四要素可行动文案。
- `assets/app/main.js`：`ui.closeSession`（关闭→解绑→清场→提示）；`acp.on('state')`（被驱逐/关闭的当前绑定就地解绑，避免下一条 prompt 撞 not_found）；`ui.newSession` 换会话先 `close(switch)`，失败只解绑并如实提示。
- `assets/app/widgets.js`：工具栏动作映射 `close`。
- `tests/frontend_smoke.mjs`：mock 三个 ext 方法；4b 断言入口存在/运行中禁用；14.8 断言 `close(user_close)`／`close(switch)`／解绑；新增 1 条 smoke 行。

## §3 验证读数

| 命令 | 读数 |
|---|---|
| `cargo check -p orz-host --tests --offline -j 1` | **exit 0**（10.66s） |
| `cargo test -p orz-host --lib session_ops::` | **7 passed / 0 failed**（6 条状态机＋data 四要素） |
| 11 条 0dk 验收单测（逐条过滤跑） | **11 passed / 0 failed**（含 `close_times_out` 首跑因真实 run 被 cancel 提前收敛而改写为注入 run marker，见 §4） |
| `cargo test -p orz-host --lib -- --test-threads=1` | **421 passed / 0 failed / 6 ignored**（49.37s；含全部既有测试） |
| `cargo test -p orz-web --lib -- --test-threads=1` | **63 passed / 0 failed**（0.45s） |
| `node crates/orz-web/tests/frontend_smoke.mjs` | **44 条 ok，全部通过**（新增关闭会话断言） |
| `cargo fmt --all -- --check` | **clean**（先 `cargo fmt --all` 后复查） |
| `python scripts/check_repository.py` | **`valid: true`，`error_count: 0`**（见 §7） |

> 并行档备注：`--test-threads=2` 时 `run_tests_timeout_kills_process_tree`／`call_tool_timeout_kills_process_tree` 两条既有的进程树超时测试在本机负载下假红；单跑各自 4.0s 通过，串行全绿——与 329 档记录的同类负载敏感一致，非本批改动。

## §4 设计偏差与理由（必须留档）

1. **journal 事件 vs 持久标记**：设计 §6.6/§4.8 写「journal 记 `session_closed`/`session_deleted`」。orz 的 journal 是「一 run 一链、单终态」的完整性单元（verifier 拒绝多终态），会话级事件不能混写；本批用 `.gsa/sessions/<s8>.closed|deleted` 标记承载持久状态，`tracing` 承载审计事实，**run journal 面零改动**。若 S4 需要会话级事件流，应另立 `.gsa/sessions/<s8>.lifecycle.jsonl` 并在审计档登记，不塞 run 链。
2. **close 不归档**：设计 `closed` 状态明确「侧车保留」；`_orz/session_close` 因此不触发 0cy 归档（归档是独立动词）。既有 `close_session`（codex 线程）仍保持归档语义，未改。
3. **驱逐把 `created` 视作可驱逐**：设计写「驱逐 idle」；`created` 无在途 run、侧车可能为空，与 idle 同为安全驱逐对象（测试名仍按设计叫 idle_only）。active（含 restore marker）一律跳过。
4. **超时测试用注入 run marker**：真实 `cancel` 会让 fake provider 在下一个 checkpoint 立刻收敛，60ms 超时测不到；`close_times_out` 直接注入不被消费的 `RunInFlight::Prompt`，验证「超时→busy→会话不半关→可重试」。真实取消路径由 `close_active_cancels_then_closes` 覆盖。
5. **浏览器读不到 409 body**：原生 WebSocket 握手失败只给 `onerror`，无法读取 JSON；服务端 409 仍带四要素（HTTP 客户端/日志可取证），前端用合并了「占用/令牌过期」的可行动文案。
6. **`default_base` 的作用域**：桥每个 workspace 一个 child，故「最近一次 attach 的根」可代表本 child 的归属根；若从未 attach 过任何会话，未知 id 的 close 只能返回 not_found（无法安全猜测根）。该边界写入 §5 残留。

## §5 未做与顺延

1. **0.9.8 双平台重建／换装／ACAF re-provision／发布**：本批不做；训练进程已结束，下一批（计划 §5）可启动。
2. **0dn S2 真机十条**：刷新重连、双标签 409、撤回后重发、删除不复活、保活过期提示、交付面板、审批并发、上下文读数、后台可见、版本偏斜——需 0.9.8 载体。
3. **跨进程恢复**：会话随进程死；桥保活窗口外回收后队列/后台任务/审批不恢复（设计非目标）。
4. **`_orz/session_delete` 的 Web 接线**：本批只落服务端原语；Web 既有删除仍走 CLI 维护面（已写墓碑），333 再决定是否改直连。
5. **会话级审计事件流**：见 §4.1；S4 若需要再立文件，不混 run journal。
6. **ACP v1 `session/close` 适配器**：保持 0.10.4；未来升级只加映射，不改状态机。

## §6 自查（按 `self_check_protocol.md`）

| 检查 | 状态 | 证据 / 说明 |
|---|---|---|
| 数据核查 | 已核实 | 读数均来自本机实跑；日志在 `orz/target/{check,test,smoke}-332*.log`（本地产物，不入库） |
| 代码链路 | 已核实 | `session_ops` 不 import ACP；`stdio` 只做映射；`acp_server` 复用同一 `release_session` 路径 |
| 参数/实验 | 不适用 | 零训练、零跑分、零载体构建 |
| 环境核查 | 已核实 | 批次开始训练 PID 13616 在跑；全程 `-j 1`；收尾时训练已结束，未借机重建 |
| 严谨性 | 已分级 | 单测/编译/冒烟是事实；真机行为、跨进程语义仍为未验证 |
| 事实-指标映射 | 已拆开 | 421/63/44/11 是实测；「close 可用」只指单测与协议面，不含真机 |
| 独立性 | 已执行 | 主代理逐文件实现、修测试、复跑；未把失败测试写成通过 |
| 偏误性 | 已检查 | 未把「编译通过」写成「S4 通过」；未把超时测试的注入 marker 说成真实 run |
| 表达检查 | 已执行 | 路径/命令/读数逐项核对；无 emoji |
| 未核风险 | 保留 | 真机 close/驱逐、浏览器 409 观感、`default_base` 边界、会话级审计缺口、删除与在途 run 的竞态 |

## §7 路由与台账

| 文件 | 改动 |
|---|---|
| `docs/audits/332_...md` | 本档 |
| `CLI_PROJECT_INDEX.md` | v4.301 → **v4.302**；头行 332 摘要；新增 `AUTH-332-SESSION-LIFECYCLE-IMPL`（`implemented`） |
| `TODO.md` | 0dk 节 S2 勾选；P1 路由行补 332 |
| `docs/BACKLOG_AND_PRIORITIES.md` | 0dk 状态/下一步指针补 332 |
| `docs/BACKLOG_AND_PRIORITIES_2.md` | §1.263 332 批流水 |
| `architecture/current/surfaces/02|03-*.md` | S2/S3 关闭/有界/删除口径补「332 已落码」一句 |
| `orz_source_manifest.sha256` | orz 提交后重生成（本批有 orz 源码改动） |
| 提交 | orz `feat/fusion-architecture`；父仓 `origin/main` |

## §8 入口

- 本档：`docs/audits/332_SESSION_LIFECYCLE_IMPLEMENTATION_2026-10-21.md`
- 前批：[`331 设计`](../SESSION_LIFECYCLE_OWNERSHIP_AND_CLOSE_DESIGN_2026-10-21.md) / [`331 补记`](../WEB_CONSOLIDATION_BATCH_SCHEDULE_2026-10-21.md) / [`330 基线`](330_SURFACE_CONTRACT_BASELINE_2026-10-21.md)
- 代码：`orz/crates/orz-host/src/session_ops.rs` / `orz/crates/orz-host/src/{acp_server,stdio}.rs` / `orz/crates/orz-web/src/{acp_pump,server}.rs` / `orz/crates/orz-web/assets/app/{state,widgets,acp,main}.js` / `orz/crates/orz-web/tests/frontend_smoke.mjs`
- 路由：[`CLI_PROJECT_INDEX.md`](../../CLI_PROJECT_INDEX.md) / [`TODO.md`](../../TODO.md) / [`BACKLOG`](../BACKLOG_AND_PRIORITIES.md)
