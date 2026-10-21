# 333 批：Web 细节必闭 3–10（保活过期／队列额度／审批并发／上下文读数／后台可见／交付空态／doctor／版本偏斜／错误四要素）（2026-10-21）

> **用户令**：「请进333档吧」。
> **本批性质**：333 = Web 细节批（S1 落码＋S2 冒烟/单测），覆盖 330 契约基线「0.9.8 必闭十条」中的 3–10；**不重建载体、不换装、不发布**（0.9.8 在下一批）。
> **来源**：331 补记的批次序（[`332–333 计划`](../WEB_CONSOLIDATION_BATCH_SCHEDULE_2026-10-21.md) §4）；330 契约基线（[`330 档`](330_SURFACE_CONTRACT_BASELINE_2026-10-21.md)）；S3/S4/S5/S7/S9/S10/S11/S12 契约页。
> **计数**：**64 不变**（Web 细节批；不新增未闭合项）。

## §0 结论速览

| # | 必闭项（330 必闭 3–10） | 结果 | 证据 |
|---|---|---|---|
| 1 | 保活窗口外不恢复的显式提示 | **落码**：桥记 grace 回收的会话事实，下一次 attach 前置帧带 `keepalive_expired/expired_session_id/expired_secs_ago`；Web 显式上屏 | `keepalive_expiry_is_reported_on_next_attach`；smoke 14.10a |
| 2 | 队列剩余额度／「不跨进程」说明 | **落码**：`_orz/queue_append` 返回 `capacity/remaining/per_message_cap/scope/persists_across_pages`；回执与属性面板显示深度/额度/生命周期 | smoke 14.10b/14.11；属性面板 |
| 3 | 审批并发可见 | **落码**：第二个请求顶掉第一个时，新弹窗顶部提示「上一条已回 cancelled」＋本页累计次数 | `dialogs.js`；smoke 14.10b |
| 4 | 上下文占用／最近一次压缩读数 | **落码**：`GET /api/context` 只读投影（侧车 chars/2 估算＋最近 8 个 run 的压缩事件＋存档计数）＋工具栏「上下文」弹窗 | `context.rs` 3 单测；smoke 14.10b |
| 5 | 后台任务列表或「仅模型可见」 | **落码**：属性面板明确「仅模型可见；本页不提供任务表，刷新/关闭不恢复」，有读数时显示计数 | smoke 14.11 |
| 6 | 交付面板空态＋`terminal_source` 可见 | **落码**：空态文案已在；冒烟钉住空态与 `terminal_source` 原码＋中文标注（面板原有能力，本批补验证） | smoke 14.11 |
| 7 | `orz doctor`（报告式，不自动修） | **落码**：8 节报告（版本/cwd/.gsa 可写/lease/run/侧车/压缩/载体完整性）＋`summary`；有 fail ⇒ exit 1 | `doctor_report` 2 单测；实跑 `ok=7 warn=1 fail=0` |
| 8 | `--version`＋桥/child 版本偏斜 | **落码**：`orz --version` 稳定单行；`/api/boot` 带 `bridge_version/child_version/version_skew`；偏斜横幅＋系统行 | `parse_version_output` 单测；smoke 14.11 |
| 9 | 错误面四要素（`code/class/hint/evidence`＋`copy.js`） | **落码**：`copy.js` 表＋`describeError`；acp 错误经四要素合成，未知码 internal 兜底 | smoke 14.11 |

## §1 范围与不做

- **做**：上表九项的源码、前端接线、Rust 单测与前端冒烟；对应契约页与台账。
- **不做**：0.9.8 双平台重建/换装/ACAF 重 provision/发布；0dn S2 真机十条；真实 Chrome 渲染走查；后台任务**实时面板**（本批按契约允许的「明确仅模型可见」落地）；手动压缩入口/上下文滑块（S9 明确不做）。
- **边界**：只改 orz 子仓与本仓文档；未触碰 `D:\AGI`；未新增依赖。

## §2 改动明细

### §2.1 保活过期（S3）

- `orz-web/src/acp_pump.rs`：`BridgeInner` 新增 `expired: HashMap<key, ExpiredSession>`；`reap_once` 在 grace 回收「曾绑定会话」的槽时记下 `session_id`；`claim_attachment` 前置帧新增 `keepalive_expired/expired_session_id/expired_secs_ago`；只有新建 child 的 attach 消费该事实（复用保活 child 不谎报）。
- `assets/app/main.js`：`onBridgeAttached` 先于 `session_id` 判空处理过期事实，显式提示「上一会话未恢复、排队补充/后台任务已失效、下一条输入将新建」。

### §2.2 队列额度（S5）

- `orz-host/src/acp_server.rs`：`handle_queue_append` 回执新增 `capacity/remaining/per_message_cap/scope=session_process/persists_across_pages=false`（额度来自 `SUPPLEMENT_QUEUE_MAX_DEPTH`，前端不再硬编码）。
- `assets/app/main.js`：入队回执显示「第 N/上限 条，剩余 M 个额度；仅本会话进程，刷新/换页不保留」。
- `assets/app/state.js`：`queueCapacity`（初值 16 镜像，响应回填）；`dialogs.js` 属性面板显示 `深度/上限（剩余）`。
- 冒烟 mock 的 `_orz/queue_append` 补额度字段。

### §2.3 审批并发（S8）

- `assets/app/dialogs.js`：`settlePermissionCancelled` 返回是否真的顶掉；新弹窗顶部黄色提示「上一条权限请求已被本次请求顶掉（已回 cancelled）……本页累计顶掉 N 次」；旧请求仍如实回 `cancelled`，不排队（v1 语义不变）。

### §2.4 上下文读数（S9）

- `orz-web/src/context.rs`（新）：只读投影——侧车 `messages` 的 `chars/2` 估算（标注 `estimate_method`，不冒充 tokenizer）、`context_scale_notified` 水位、最近 8 个本会话 run journal 尾部（各 256 KiB）的 `context_compressed/context_compacted` 事件、压缩存档/回放块计数；缺数据写 `notes`。
- `orz-web/src/server.rs`：`GET /api/context`（`session8` 8 位小写 hex；`root` 走既有信任根解析）。
- `assets/app/api.js`：`fetchContext`；`dialogs.js`：`openContext()`（无绑定/加载/错误/迟到作废/空态）；工具栏新增「上下文」；`main.js` 接线。

### §2.5 后台可见（S7）

- `assets/app/dialogs.js` 属性面板新增「后台任务」行：有计数显示「n 个（仅模型可见；本页不提供任务表，刷新/关闭不恢复）」，无计数显示「仅模型可见……」。不虚构一个 Web 读不到的任务表（契约允许的「明确仅模型可见」分支）。

### §2.6 交付面板（S10）

- 面板在 329 已落 `terminal_source` 与空态；本批补冒烟：无侧车/无 run 时必须出现「尚无可展示的交付事实」；有 run 时必须出现 `run_end（模型 [RUN_END] 声明）`（原码＋中文标注）。

### §2.7 `orz doctor`（S11）

- `orz-bin/src/main.rs`：`doctor` 子命令（在 `web` 之前分派，不装配会话/网络）；`doctor_report` 纯函数检查——版本、cwd、`.gsa` 可写探针（写删临时文件，不修复既有文件）、lease 总数/被占数（复用 `orz_host::acp_server::leased_sessions`，本批改为 `pub`）、run journal 数＋最新可读、会话侧车数、压缩存档/块档数、载体清单完整性；输出 `[ok]/[warn]/[fail]`＋`summary`，有 fail ⇒ exit 1。
- 实跑（开发树）：`ok=7 warn=1 fail=0`（warn＝无 carrier-manifest，开发树正常），exit 0。

### §2.8 版本偏斜（S12）

- `orz-bin/src/main.rs`：`--version`/`-V` 在 home 重定向与任何装配之前打印 `orz <version> (<os>/<arch>/<profile>)` 并退出 0。
- `orz-web/src/server.rs`：`parse_version_output`＋`binary_version`；`/api/boot` 新增 `bridge_version/child_version/version_skew`（任一侧无 `--version` ⇒ 对应字段/偏斜为 null，不妄判）。
- `assets/app/main.js`：`applyBootFacts` 在 `version_skew=true` 时上连接横幅＋系统行，给两侧版本号与「重装同版本后重启」；`ui.applyBootFacts` 供冒烟注入。

### §2.9 错误四要素（S4/S11）

- `assets/app/copy.js`（新）：六类错误表（session/connection/permission/provider/internal；tool 不上浮故不收录）＋`describeError(data)` 合成「发生了什么（分类）→ 下一步；证据」。
- `assets/app/acp.js`：`error.data.code` 存在时走 `describeError`；其余 data 保持原 JSON 化。
- `orz-web/src/assets.rs`：内嵌 `copy.js`（缺条目会让 acp.js 404）。
- 冒烟：`session_busy` 四要素齐全、未知码走 `internal`。

## §3 验证读数

| 命令 | 读数 |
|---|---|
| `cargo check -p orz-web -p orz-bin --tests --offline -j 1` | **exit 0** |
| `cargo test -p orz-web --lib -- --test-threads=1` | **68 passed / 0 failed**（原 63 → 68：`context` 3、`acp_pump` 1、`server` 1） |
| `cargo test -p orz-host --lib -- --test-threads=1` | **421 passed / 0 failed / 6 ignored**（49.86s） |
| `cargo test -p orz-bin --bin orz -- --test-threads=1` | **19 passed / 0 failed / 12 ignored**（含 doctor 2 条） |
| `cargo run -p orz-bin --bin orz -- --version` | `orz 0.9.7 (windows/x86_64/debug)`，exit 0 |
| `cargo run -p orz-bin --bin orz -- doctor` | `ok=7 warn=1 fail=0`，exit 0（warn＝开发树无 carrier 清单） |
| `node crates/orz-web/tests/frontend_smoke.mjs` | **47 条 ok，全部通过**（14.10a/14.10b/14.11 三条新钉） |
| `cargo fmt --all -- --check` | **clean** |
| `python scripts/check_repository.py` | 见 §7（提交前跑） |

## §4 偏差与边界（必须留档）

1. **保活过期事实只在桥进程内存**：桥进程重启后，`expired` 表清空；刷新页面仍会看到 boot 的通用「窗口外不恢复」提示，但不会再点名上一会话。跨桥进程的过期审计需要持久化，属后续项。
2. **上下文估算是近似**：`chars/2` 与 orz-loop 估算同口径但不等于真实 tokenizer；`last_compaction` 只扫最近 8 个 run 的 256 KiB 尾部，更早的压缩事件可能为 null，`notes` 明示。
3. **后台任务选择「仅模型可见」分支**：没有把 agent 进程内的 terminal task table 经 ACP 暴露到 Web；属性面板只声明边界，不虚构实时任务列表。
4. **版本探针成本**：每次 `/api/boot` 会 spawn 两次 `--version`（桥自身与 agent binary），均为早退路径（<100ms 量级）；旧载体没有 `--version` 时字段为 null，不误报偏斜。
5. **doctor 不自动修**：会创建 `.gsa` 目录与写删一个探针文件（会话本来也会建目录），但不会修复权限/清单/lease；发现项只报告并给下一步。
6. **审批顶掉提示是「新弹窗内」提示**：旧请求仍只会收到 `cancelled`，不会在对话里补一条系统行（避免把权限面写进模型可读上下文）。
7. **`copy.js` 只覆盖已知码**：未知 `data.code` 走 `internal` 兜底；工具失败不上浮、不进表。

## §5 未做与顺延

1. **0.9.8 双平台重建/换装/ACAF re-provision/发布**：下一批；本批只做源码与验证。
2. **0dn S2 真机十条**：需 0.9.8 载体（含本批 Web 细节）。
3. **真实浏览器渲染/观感**：frontend smoke 是 DOM stub；真机走查归 0dn S2。
4. **后台任务实时面板**：本批未选该分支；若真机后认为必要，另立设计。
5. **跨桥进程的保活过期审计**：见 §4.1。
6. **doctor 的 ACAF/信任存储深度检查**：当前只检查载体清单/目录/lease；ACAF fabric 与授信清单的主动探测未纳入（避免 doctor 触发网络/签名器）。

## §6 自查（按 `self_check_protocol.md`）

| 检查 | 状态 | 证据 / 说明 |
|---|---|---|
| 数据核查 | 已核实 | 读数均来自本机实跑；日志在 `%TEMP%\*333*.log`（本地产物，不入库） |
| 代码链路 | 已核实 | 九个必闭项逐项对应文件/测试；`cargo check` 两包与 bin 全过 |
| 参数/实验 | 不适用 | 零训练、零跑分、零载体构建（仅 debug `cargo run` 验证 CLI） |
| 环境核查 | 已核实 | 训练进程已结束；全程 `-j 1`；未做 release 构建 |
| 严谨性 | 已分级 | 单测/冒烟/CLI 实跑是事实；浏览器观感与真机为未验证 |
| 事实-指标映射 | 已拆开 | 68/421/19/47 是实测；「上下文读数可用」只指只读投影与 stub 渲染 |
| 独立性 | 已执行 | 主代理逐文件实现、修冒烟、复跑；未把失败写成通过 |
| 偏误性 | 已检查 | 未把「读数存在」写成「读数准确」；未把 doctor 的 warn 写成 fail |
| 表达检查 | 已执行 | 路径/命令/读数逐项核对；无 emoji |
| 未核风险 | 保留 | 保活过期跨桥进程、上下文估算偏差、后台任务不可见、版本探针旧载体、doctor 深度 |

## §7 路由与台账

| 文件 | 改动 |
|---|---|
| `docs/audits/333_...md` | 本档 |
| `CLI_PROJECT_INDEX.md` | v4.302 → **v4.303**；头行 333 摘要；新增 `AUTH-333-WEB-DETAILS`（`reference`） |
| `TODO.md` | 计数行本批摘要；0dn 节补 333 预落 |
| `docs/BACKLOG_AND_PRIORITIES.md` | 指针行/计数行；0dn 节补 333 |
| `docs/BACKLOG_AND_PRIORITIES_2.md` | §1.264 333 批流水 |
| `architecture/current/surfaces/{03,05,07,09,10,11,12}-*.md` | 对应面补「333 已落码」与测试映射 |
| `orz_source_manifest.sha256` | orz 提交后重生成 |
| 提交 | orz `feat/fusion-architecture`；父仓 `origin/main` |

## §8 入口

- 本档：`docs/audits/333_WEB_DETAILS_MUST_CLOSE_3_TO_10_2026-10-21.md`
- 前批：[`332 档`](332_SESSION_LIFECYCLE_IMPLEMENTATION_2026-10-21.md) / [`331 设计`](../SESSION_LIFECYCLE_OWNERSHIP_AND_CLOSE_DESIGN_2026-10-21.md) / [`批次序`](../WEB_CONSOLIDATION_BATCH_SCHEDULE_2026-10-21.md) / [`330 基线`](330_SURFACE_CONTRACT_BASELINE_2026-10-21.md)
- 代码：`orz/crates/orz-web/src/{acp_pump,server,context,assets}.rs` / `orz/crates/orz-web/assets/app/{main,dialogs,acp,api,state,widgets,copy}.js` / `orz/crates/orz-web/tests/frontend_smoke.mjs` / `orz/crates/orz-bin/src/main.rs` / `orz/crates/orz-host/src/acp_server.rs`
- 路由：[`CLI_PROJECT_INDEX.md`](../../CLI_PROJECT_INDEX.md) / [`TODO.md`](../../TODO.md) / [`BACKLOG`](../BACKLOG_AND_PRIORITIES.md)
