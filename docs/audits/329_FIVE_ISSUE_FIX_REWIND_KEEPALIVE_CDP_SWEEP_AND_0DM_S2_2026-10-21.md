# 329 批：0.9.7 五问题修复（/rewind 投影截断／会话保活／session-new 不收卷／CDP 就绪重试）＋ 0dm S2 交付状态面板（2026-10-21）

> **用户令**：「329就按照你的裁决来吧，0dm S2同车即可。做完以后暂时不重建，目前后台有训练进程，优先给h=128的训练进程，避免重建抢占CPU和内存」。
> **本批性质**：329 = 修复批（S1 落码＋S2 测试）；**不重建载体、不换装、不发行**（用户令）；S3/S4 顺延到训练结束后的批次。
> **来源**：328 档（[`328`](328_0_9_7_FIVE_ISSUE_REVIEW_AND_54D12BCC_RECOVERY_2026-10-21.md)）§6 五问题根因＋§8 修复排序；0dm 设计稿 §4.2（S2 交付状态按需查询）。
> **计数**：**63 不变**（修复批；0dm 仍 `partial`，S4 收口才翻 `implemented`）。
> **训练进程约束（事实）**：`D:\AGI\training\train_lif3d.py --predict-horizon 128 --device cpu` 常驻；12 逻辑核／16 GiB，可用内存约 2.2–2.6 GiB。本批**零 `cargo build --release`、零载体重建、零 Linux musl**；仅做单 job（`-j 1`）增量 `cargo check`／定向测试与 `node` 前端冒烟。

## §0 结论速览

| 面 | 本批结果 | 关键证据 |
|---|---|---|
| ① `/rewind` 投影截断（0di） | **落码**：撤回后本地 items/marker/turn 同步截断；重放按侧车 `recall_log` 过滤被撤 tail；marker 改稳定锚 | 前端 §15 十条钉子；`/api/recall` 端点 |
| ② 会话保活/重连（0dk） | **落码**：桥持有 child，WS 断开只 detach（默认保活 1800s），同区重连 attach 并发 `_orz/bridge_attached`；backlog 有界补发 | `acp_pump` 7 单测；`AGENT_BRIDGE` 状态机 |
| ③ `session/new` 交互式不收卷（0cy 口径） | **落码**：ACP 交互路径改走 `handle_session_new_with_options_no_sweep`；sweep 只留启动侧扫描/显式维护 | 新增钉子 `interactive_session_new_does_not_sweep_sibling_sessions` |
| ④ CDP 就绪重试（EVIDENCE-LOCAL-BROWSER） | **落码**：`/json/version` 在载入预算剩余时间内有界重试（100→200ms 退避），确定性错误不重试 | 3 单测；最坏尾巴＝预算＋单次在途请求（≤5s） |
| ⑤ 0dm S2 交付状态面板 | **落码**：`/api/delivery` 只读投影＋工具栏「交付状态」弹窗（侧车 plan/末步、最近 run 终态/自查、撤回审计） | `delivery.rs` 7 单测；`delivery.js` 纯渲染 |
| 载体/真机 | **未做（用户令）**：0.9.8 重建/换装/S4 全部顺延 | 本档 §4 |

## §1 范围与不做

- **做**：①–⑤ 的源码、测试、前端接线、README 口径同步；328 台账提交。
- **不做**：载体重建/换装/发行；真机五场景复验；`0dh` S4（跨 run 过夜训练）；ZCode 式浏览器面板（未立项）；时间溯源护栏（328 裁决记录为可接受风险）。
- **边界**：所有改动只在 orz 子仓与本仓文档；未触碰 `D:\AGI`（训练目录零写入）。

## §2 改动明细

### §2.1 ① `/rewind` 投影截断（0di；328 §6.1）

问题：服务端 `_orz/recall` 只截断侧车；前端只追加一条系统行，被撤回的卡片与 marker 留在窗内；重开会话按 journal 全量重放，旧 tail 复活；marker 存数组下标，截断/重放后漂移。

- `assets/app/state.js`：条目携带 `item.anchor`；新增 `vm.rebuildMarkers()`；内存裁剪时删除越界锚点（旧实现把下标钳到 0 会指错卡）；新增 `content.droppedUserCards` 对齐撤回 ordinal 轴。
- `assets/app/main.js`：`ui.rewindRecall` 成功后按用户卡 ordinal 截断 `items`、过滤 `marker` 并重建、`currentModelIndex=null`、`turnCounter` 回退；本地缺该序号时只提示不瞎截；新增 `filterEventsByRecall`／`replayConversationMerged`（有 `recall_log` 才缓冲过滤，无水位保持原流式重放；多 recall 按 `recalled_at` 级联；>20 万事件放弃过滤并如实提示）；`openConversation`/`openRunReplay` 重放后 `rebuildMarkers`；`ui.scrollToItem` 越界显式提示；实时尾对「撤回前已结束的最新 run」不再从头重放。
- `assets/app/api.js`：新增 `fetchRecall(session8, root)`（端点不可用时调用方按无水位继续）。
- `src/runs.rs` + `src/server.rs`：新增 `GET /api/recall`（只读侧车 `recall_log`；8 位小写 hex 校验；缺侧车/坏 JSON 显式错误）。
- `tests/frontend_smoke.mjs`：§15 十条钉子（截断/锚点/turn、同文重发、本地缺序号、越界、内存裁剪 ordinal 轴、单/多 recall 过滤、撤回后无新事件、端点不可用）。

**残留边界**：服务端 ordinal 空间（可撤回用户消息）与 journal 用户事件空间在双投递/重构侧车/`RUN-CLI` 场景可能错位（越界保护只能防“找不到”，不能防 off-by-one）；坏 `timestamp` 事件按“早于 recall”处理；有水位时最多缓冲 20 万事件。

### §2.2 ② 会话保活/重连（0dk；328 §6.4）

问题：旧桥「一 WS 对一 `orz --stdio` child，任一侧断开即杀 child」；刷新/切视图/短时断线都杀 run；327 只修“断后恢复”，没有修“不断”。

- `src/acp_pump.rs`（重写）：`AgentBridge` 按工作区（cwd display 串）持有 `AgentHandle`；WS 关闭＝detach（不杀 child，stdout 继续收进 8 MiB 有界 backlog，溢出丢最旧并置 truncated）；同区重连＝attach（先发 `_orz/bridge_attached` 前置帧：`session_id`/`reused`/`prompt_in_flight`/`grace_secs`/`cwd`，再补发 backlog）；已 attach 时第二个 WS 仍 409；reaper 每 15s 回收 dead 或超过 grace 的 detached 槽；桥只做两处最小观察（`session/new|load` 的 sessionId、`session/prompt` 在途事实），不改写 ACP 语义。
- `src/server.rs`：`ServerState.session: SessionSlot` → `bridge: AgentBridge`；`ws_acp` 改 attach（Busy→409，SpawnFailed→502），upgrade 后 `run_connection`。
- `src/lib.rs`：构造 bridge＋启动 reaper；启动台打印保活窗口；`ORZ_WEB_KEEPALIVE_SECS`（默认 1800s，0＝旧行为，硬顶 6h）。
- `assets/app/acp.js`：新增 `attached`/`notice` 回调（bridge_attached / child_exited / backlog_truncated）。
- `assets/app/main.js`：`onBridgeAttached` 直接绑定仍在跑的会话（不再 `session/new`/`session/load`），按 `prompt_in_flight` 复位运行态并重挂 journal 尾；`syncLiveTail` 对撤回前已结束 run 不重放；`beforeunload`/boot 文案改为“刷新可重连、超保活窗口回收、队列/后台任务随之不恢复”。
- `README.md`：会话与计划、审计状态两处口径同步。

**验证边界**：单测覆盖 attach 独占/backlog 补发/session 观察/grace 解析；**未做**真实 WS＋真实 `orz --stdio` 的端到端（需重建载体，用户令不做）。无心跳；半开连接（进程被杀但 TCP 未断）可能占住 sink 直到 OS 发现，属已知残留。

### §2.3 ③ `session/new` 交互式不收卷（0cy/328 §6.5）

`src/acp_server.rs` 新增 `handle_session_new_with_options_no_sweep`（`sweep_enabled=false`）；`src/stdio.rs` 的 ACP `session/new` 改走该入口。启动侧遗留扫描与显式维护命令仍保留 sweep。新增钉子：交互式 `session/new` 不得收卷兄弟会话的侧车/run 目录。

### §2.4 ④ CDP 就绪重试（EVIDENCE-LOCAL-BROWSER；328 §6.3）

`src/local_browser/cdp.rs`：`launch` 在端口文件轮询前取 `discovery_deadline = now + DEFAULT_LOAD_TIMEOUT`，`/json/version` 改为 `discover_version_with_retry(port, deadline)`；100ms 起、翻倍、上限 200ms；只对 `connect:`/`write:`/`read:`/`http GET`（超时）类重试；非 200/非法 Content-Length/超 16KB/JSON 解析失败立即失败；失败返回最后一次真实 cause。

**预算尾巴**：deadline 只决定“还能不能再发起一次尝试”，不抢占在途请求；最坏＝剩余排期＋单次 `HTTP_JSON_TIMEOUT`（≤5s）。永久坏端点时首调失败从 ~5s 变为 ~30s 量级（持 `browser_launch_lock`），属可接受代价。

### §2.5 ⑤ 0dm S2 交付状态面板（0dm；设计 §4.2）

- `src/delivery.rs`（新）：`delivery_summary(cwd, session8)` 只读投影——侧车 `session_id/messages/blackboard.plan/recall_log` ＋ 最近 run 尾部 `run_finished`/`tool_completed{self_check}`；`RUN-<s8>-<n>` 按数值最大、回落 `RUN-CLI-<s8>`；缺数据 null/空数组并写 `notes`；`compute_delivery_status` 依赖 loop 内存基线，**空闲 UI 不重算**。
- `src/server.rs`：新增 `GET /api/delivery`。
- `assets/app/delivery.js`（新）：纯渲染 `renderDeliveryStatus(body, data)`；`assets/app/dialogs.js`：`openDeliveryStatus()` 弹窗（无绑定/加载/错误/迟到作废/空态）；
- `src/assets.rs`：内嵌 `delivery.js`（缺此条目会让 dialogs 整块 404）。
- 工具栏：`state.js` 增「交付状态」；`widgets.js` 映射 `ui.openDeliveryStatus`；`main.js` 接线。
- **边界**：`delivery_status` 只取侧车已存值（submit 退役后多为 null）；256 KiB 尾部窗口可能低估超长 journal 的 self_check 次数（`run_finished` 字段为准，notes 注明）；不识别 `RST-<s8>-*`。

## §3 验证读数

| 命令 | 读数 |
|---|---|
| `cargo check -p orz-web --lib --tests -j 1 --offline` | **exit 0**（5.57s；含 `delivery.rs`、`acp_pump` 测试面） |
| `cargo check -p orz-host --lib --tests -j 1 --offline` | **exit 0**（1m22s；唯一 warning 为既有 `permission.rs` unused imports，非本批） |
| `cargo test -p orz-web --lib -j 1 --offline` | **63 passed / 0 failed / 0 ignored**（原 52 → 63：`acp_pump` 7、`delivery` 7、`runs::recall_log` 1，另移除旧 `SessionSlot` 2 项） |
| `cargo test -p orz-host --lib discover_version_with_retry -- --test-threads=1` | **3 passed**（首跑 1 例因断言 `<2s` 与实际 connect ~2.02s 冲突而失败，已把上界改为 6s 并注明在途请求尾巴，复跑全过） |
| `cargo test -p orz-host --lib interactive_session_new_does_not_sweep_sibling_sessions session_load_does_not_sweep_sibling_sessions` | **2 passed** |
| `node orz/crates/orz-web/tests/frontend_smoke.mjs` | **43 条 ok，全部通过**（新增 §15 十条） |
| `rustfmt --edition 2024 --check`（9 个触碰文件） | **clean**（先对 `orz-web/src/lib.rs` 跑一次 rustfmt 后全清） |
| 载体/真机 | **未跑**（用户令：训练进程优先，暂不重建） |

## §4 未做与顺延

1. **0.9.8 双平台重建／装机位换装／ACAF re-provision／发布**：顺延（训练结束、用户放行后）。
2. **S4 真机**：五问题复验（撤回重发/切会话/刷新重连/Chrome/CDP）＋0dm 三场景；需新载体。
3. **0dh S4**：跨 run 过夜训练存活判据，单独真机轮。
4. **ZCode 式浏览器**：未立项（328 结论：借设计/协议，不搬 Electron）。
5. **时间溯源护栏**：按 328 记录为可接受风险，不加 index 时间戳。
6. **保活心跳**：未实现；半开 WS 的 sink 占用依赖 OS 断开，作为残留登记。

## §5 自查（按 `self_check_protocol.md`）

| 检查 | 状态 | 证据 / 说明 |
|---|---|---|
| 数据核查 | 已核实 | 测试读数来自本机实跑；未引二手账面 |
| 代码链路 | 已核实 | 逐文件改动与 328 根因一一对应；`cargo check` 两 crate 全过 |
| 参数/实验 | 不适用 | 零训练、零跑分、零载体构建 |
| 环境核查 | 已核实 | 训练进程 PID 13616（`train_lif3d.py --predict-horizon 128 --device cpu`）在跑；全程单 job 增量编译 |
| 严谨性 | 已分级 | 代码/测试为事实；“端到端保活可重连”为未验证推断（需 S3/S4） |
| 事实-指标映射 | 已拆开 | 63/43/3/2 是实测；0dm S2 “可用”只指后端契约与前端纯渲染，不含真机观感 |
| 独立性 | 已执行 | 子代理 A/B/C 分件交付，主代理逐件复核、编译与集成；B 的测试失败由主代理修正后复跑 |
| 偏误性 | 已检查 | 未把“编译通过”写成“真机通过”；未把 328 的推断写成事实 |
| 表达检查 | 已执行 | 路径、命令、读数、单位逐项核对；无 emoji |
| 未核风险 | 保留 | 保活端到端、半开心跳、多 recall ordinal 空间、CDP 永久坏端点首调 30s、0dm 面板观感 |

## §6 路由与台账

| 文件 | 改动 |
|---|---|
| `docs/audits/329_...md` | 本档 |
| `CLI_PROJECT_INDEX.md` | v4.298 → **v4.299**；头行 329 摘要；新增 `AUTH-329-FIX-BATCH`（`reference`）；0dm 条目补 S2 达成 |
| `TODO.md` | 批次指针；0dm S2 勾选；0di/0dk/0cy 行补 329 进展 |
| `docs/BACKLOG_AND_PRIORITIES.md` | 指针行＋计数行；0di/0dk/0cy/0dm/EVIDENCE-LOCAL-BROWSER 各节补 329 |
| `docs/BACKLOG_AND_PRIORITIES_2.md` | §1.260 329 批流水 |
| `README.md` | 会话保活与 `session/new` 不收卷口径同步 |
| `orz_source_manifest.sha256` | orz 提交后重生成 |

## §7 入口

- 本档：`docs/audits/329_FIVE_ISSUE_FIX_REWIND_KEEPALIVE_CDP_SWEEP_AND_0DM_S2_2026-10-21.md`
- 前批：[`328 档`](328_0_9_7_FIVE_ISSUE_REVIEW_AND_54D12BCC_RECOVERY_2026-10-21.md) / [`327 档`](327_RECOVERY_TOOL_PAIRING_AND_ARCHIVE_RESUME_2026-10-21.md)
- 设计：[`0dm 设计稿`](../TERMINAL_DECLARATION_AND_SELF_CHECK_PULL_DESIGN_2026-10-21.md) §4.2 / [`0di 设计稿`](../0DI_RECALL_SUPPLEMENT_DESIGN_2026-10-18.md)
- 代码：`orz/crates/orz-web/src/{acp_pump,server,runs,delivery,lib,assets}.rs` / `orz/crates/orz-web/assets/app/{main,state,acp,api,dialogs,delivery,widgets}.js` / `orz/crates/orz-host/src/{acp_server,stdio}.rs` / `orz/crates/orz-host/src/local_browser/cdp.rs`
- 路由：[`CLI_PROJECT_INDEX.md`](../../CLI_PROJECT_INDEX.md) / [`TODO.md`](../../TODO.md) / [`BACKLOG`](../BACKLOG_AND_PRIORITIES.md)
