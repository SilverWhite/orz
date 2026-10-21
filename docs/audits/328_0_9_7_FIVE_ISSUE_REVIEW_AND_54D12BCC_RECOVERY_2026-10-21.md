# 328 批：0.9.7 真机五问题复核与 54d12bcc 恢复取证（只读，2026-10-21）

> **用户本轮反馈**：「这些问题全部都是 0.9.7 中遇到的」；新增两条观察——模型把几个月前的讨论认成「几年前」；会话 `54d12bcc` 之前报票据不完全、现在又可以继续。
> **本批性质**：只读取证与归因复核，**不改源码、不重跑真机、不改研究区文档**；计数 **63 不变**。
> **版本口径**：用户已确认 `orz-build-info: version=0.9.7 os=windows arch=x86_64 profile=release`；本批另行核对在役进程与二进制时间，确认不是 0.9.6 旧映射。

## §0 结论速览

| 面 | 结论 | 关键证据 |
|---|---|---|
| 0.9.7 是否在役 | **是**；Web 进程与 `--stdio` 子进程均在 0.9.7 二进制落盘后启动 | `orz.exe` 写入 17:43:11；`orz web --real` PID 30168 启动 17:53:17；`--stdio` PID 7700 启动 20:14:52 |
| 54d12bcc「能继续」 | **不是恢复，是失败 run 被重置**：侧车/归档各只剩 1 条 user，0 条 assistant/tool | `RUN-54d12bcc-0` 130 events；9/9 ticket；17 `tool_started`/16 `tool_completed`；无 `run_finished` |
| 时间误判 | **不是时钟/时区 bug**；是索引条目无日期时，模型用轮次差外推并写成事实 | R140 日期 2026-06-14；ROUND232 日期 2026-10-20；实际 128 天，「六年前」出现 3 次 |
| `/rewind` 标记栏 | 0.9.7 仍存在：前端只加系统消息、不截断 items/marker；同文重发被去重；旧 journal 全量重放 | `main.js:845` / `state.js:146` / `main.js:540` |
| 长程进程 | 仍是 call-scoped 进程树＋默认 600s 回收；0dh 的 S4 真机未收口 | `acp_pump` 非根因；0dh S4 待续；用户实测 600s 杀 |
| CDP discovery refused | 0.9.7 仍是「拿到端口文件后只探一次 `/json/version`」；Chrome 端点尚未就绪即 10061 | `cdp.rs:617-622`；端口文件先写、HTTP 后就绪 |
| 切会话不保活 | WS 一侧断开即 `start_kill`；一 WS 对一 child；0.9.7 只修了断后恢复，未修「不断」 | `acp_pump.rs:156-161`；`kill_on_drop(true)`；单工作区 409 |
| `session/new` 自动归档 | 仍固定 `sweep_enabled=true`；非当前进程 live 集的会话都会被收卷删源 | `acp_server.rs:3511-3518` |
| ZCode 借鉴 | **能借设计，不能直接搬 Electron 代码**；Apache-2.0 允许复用；orz Web 需 CDP screencast＋真实窗口接管 | [ZCode](https://github.com/zai-org/ZCode) / [LICENSE](https://github.com/zai-org/ZCode/blob/main/LICENSE) |

## §1 用户原文（本轮）

> 我最开始提到的
> “1. 使用/rewind撤回消息并重新发送后标记栏出现bug，显示与定位均存在问题，以前的消息无法再定位
> 2. 模型当前使用进程工具时对长程进程的启动和使用疑似不顺手
> 3. 本地chrome使用不稳定，存在回报CDP discovery endpoint failed: connect: 由于目标计算机积极拒绝的问题，我看到浏览器主页提示使用模式不安全，不过后续还是再次成功使用了本地浏览器通道。
> 得整一个zcode那样的内置浏览器才行，也方便我辅助过人机验证，zcode前一阵子开源了，我们能直接借鉴吗？
> 4.一切换会话，会话就会被中断，不会保活，必须刷新网页后才能恢复并继续，此时标记栏中又可见对旧输入输出的追踪
> 而切换一旦发生在票据进程不全的时候，会话一被打断就无法被成功鉴权，彻底无法继续”
> 这些问题全部都是0.9.7中遇到的。
> 哦对还会存在一个问题，模型会认错时间，会将几个月前的讨论内容认成几年前的，单文件都有时间戳，这是纯粹的幻觉问题吗？我记得ROUND230后存在一处“六年前”的错误，当时我没有额外指出并对其进行修正。
> 54d12bcc会话又可以继续了，之前不可以来着，报票据不完全

后续用户裁决（同轮）：

> index本身就是方便模型快速查找的，在index中找到可用内容后不进一步查找原文件是正常行为，不是幻觉就好，可惜index本身需要尽可能精简，就不加时间戳了，反正不是大问题
> 请对本轮问题的反馈和你做出的审查及回应按照本工作区的纪律落一份审计文档吧，方便后续查看
> 本工作区存有的是CLI_PROJECT_INDEX.md路由和todo/backlog

## §2 审查契约与来源

| 项 | 内容 |
|---|---|
| 任务 | 复核 0.9.7 五问题；分析时间误判；解释 54d12bcc 恢复；给出 ZCode 借鉴判断；落审计并接回路由/TODO/BACKLOG |
| 执行面 | 只读：进程查询、journal 解析、源码阅读、文档检索、ZCode README/LICENSE 核验 |
| 不做 | 不改 orz 源码；不重跑 54d12bcc；不启动新训练；不改 `LIF_CURRENT_INDEX.md` / ROUND232；不新增计数 |
| 工作区 | `D:\CLI`（CLI 项目）；用户真机会话在 `C:\Users\1\OneDrive\Desktop\新建文件夹\.gsa` |
| 证据边界 | CDP 竞态未在本批真机复现；时间误判机制为 agent-inferred；ZCode 内置浏览器结构依据 README/前序调研，未在本批重跑 ZCode |

### §2.1 主要来源

- 进程：`Get-CimInstance Win32_Process` / `Get-Process` / `Get-Item`（本批实测，见 §3）
- 会话：`C:\Users\1\OneDrive\Desktop\新建文件夹\.gsa\runs\RUN-54d12bcc-0\events.jsonl`
- 侧车/归档：`.gsa\conversations\54d12bcc.json`、`.gsa\archives\54d12bcc.json.gz`
- 时间：`R8_DISCUSSION_ROUND140_2026-06-14.md`、`R8_DISCUSSION_ROUND232_..._2026-10-20.md`、`LIF_CURRENT_INDEX.md`
- 代码：`orz/crates/orz-web/assets/app/{main,state}.js`、`orz/crates/orz-web/src/acp_pump.rs`、`orz/crates/orz-host/src/{acp_server,local_browser/cdp}.rs`
- 前批：`docs/audits/327_RECOVERY_TOOL_PAIRING_AND_ARCHIVE_RESUME_2026-10-21.md`
- ZCode：[README](https://github.com/zai-org/ZCode/blob/main/README.en.md) / [LICENSE](https://github.com/zai-org/ZCode/blob/main/LICENSE)

## §3 0.9.7 在役确认（不是旧 0.9.6 映射）

| 对象 | 值 |
|---|---|
| `orz.exe` 文件 | `C:\Users\1\AppData\Local\orz\bin\orz.exe`，58,833,408 B，写入 **2026-10-21 17:43:11** |
| Web 进程 | PID **30168**，`orz web --real`，启动 **17:53:17** |
| Agent 子进程 | PID **7700**，`orz --stdio`，启动 **20:14:52** |
| 构建信息 | `version=0.9.7 os=windows arch=x86_64 profile=release` |

结论：Web 进程和子进程都在 0.9.7 文件落盘之后启动，**不是 327 档 §5 所说的「旧 `orz web` 仍持有 0.9.6 映射」**。327 档的「待重启 web」边界在本批已不成立；后续以本节为准。

## §4 54d12bcc：能继续 ≠ 恢复

### §4.1 现场计数

| 项 | 数值 |
|---|---|
| `RUN-54d12bcc-0` 事件 | **130** |
| command ticket 签发/消费 | **9 / 9** |
| `tool_started` / `tool_completed` | **17 / 16** |
| 悬空调用 | `call_00_z0tymqOgFKnLTv0gYi7l9339`（最后一条 pdftotext 批量抽取） |
| `run_finished` | **0** |
| 侧车 `54d12bcc.json` | **1 条消息，只有 user prompt** |
| 归档 `54d12bcc.json.gz` | **1 条消息，只有 user prompt** |
| 归档标记 | `reconstructed_from_journal: true`；`archived_at 2026-10-21T12:56:47Z` |

### §4.2 票据为什么「不完全」

最后一张 command ticket 在 12:13:03.591 签发、.596 消费，随后 `tool_started` 在 .600 出现；`tool_completed` 永远没有回来，整个 run 也没有 `run_finished`。旧恢复看到「票已消费、对应工具结果缺失」，就报票据不完全。

### §4.3 0.9.7 为什么现在能继续

327 批的重构规则是：只有 `run_finished` 的 run 的助手输出进入重构对话；失败/取消 run 只保留用户输入，助手输出一律丢弃；悬空 tool_calls 不再重放（见 327 档 §3）。因此这次 run 的：

- 10 条 `model_output` 全部不进上下文（其中 9 条是纯 tool_calls，1 条有 23 字正文）；
- 17 次工具调用、16 条结果全部不进上下文；
- 侧车和归档都只剩最初 1 条 user prompt。

**它能继续，是因为失败 run 被过滤掉了，不是因为票据被补全。** 这是「可继续」与「恢复原现场」的差别，也是问题 4 的精确样本：切换/关闭发生在票据未闭环时，旧路径拒绝，新路径靠丢历史绕过。

### §4.4 残留物

- `plan_write` 只留下 732 字符的元数据，正文不在 journal；黑板若未落盘，计划正文也不可重放。
- `_LIT_R2_EXTRACT` 目录仍有 4 篇 PDF 提取文本（`arXiv-2606.03935/23532/2607.14672/16017`），属于磁盘产物，模型可重读；但它不会记得前 17 次工具调用的中间结论。
- 继续该会话时，应显式说明「这是被重置的会话，从磁盘重读」，不要假设它记得。

## §5 时间误判：不是时钟错，是「无日期路由＋轮次外推」

> 本节 R140 / ROUND232 / `LIF_CURRENT_INDEX.md` 均指研究区 `C:\Users\1\OneDrive\Desktop\新建文件夹\`，不是本仓 `CLI_PROJECT_INDEX.md`。

### §5.1 事实

| 项 | 值 |
|---|---|
| R140 文档日期 | **2026-06-14**（`R8_DISCUSSION_ROUND140_2026-06-14.md` 第 3 行） |
| ROUND232 文档日期 | **2026-10-20**（标题与第 3 行） |
| 实际间隔 | **128 天，约 4 个月** |
| 「六年前」出现 | ROUND232 **L85 / L152 / L186**，共 3 次；L186 是自查表 |
| 索引 I-R140 条目 | `LIF_CURRENT_INDEX.md:224`，只有 `(R140, 线索)`，**无日期** |

### §5.2 归因

1. ROUND232 的「实际读取」列的是 `LIF_CURRENT_INDEX.md`，没有列 R140 文档；模型只拿到 `I-R140` 这个编号。
2. 索引条目没有日期；R140 到 R232 相差 **92 轮**。若按「一轮约一个月」估，就是 7 年多，取整成「六年」；实际 92 轮只过了 128 天（约 1.4 天/轮）。
3. 这是 **agent-inferred** 的最像机制，不是模型内部推理的实证；但「六年前」这个断言本身是事实错误，可以确定。

### §5.3 定性

- **不是 orz 的时钟/时区 bug**：run 时间戳、文档日期、文件 mtime 都正确。
- **不是「文件有时间戳却读错」**：文件 mtime 不会自动进模型上下文；模型读的是索引，不是文件属性。
- **不是纯随机幻觉**：它有可解释的来源缺口（索引无日期）和外推路径（轮次差）。
- **但仍是模型侧错误**：在没有日期时，它不该把「六年」写成事实；应写「日期未知」或回查 R140。
- 索引 L224 没有把「六年前」写进去（`C-R232-SMALL-VARIANCE-NOT-SELECTED` 只写「已记 cond≈1.07」），错误主要污染 ROUND232 正文与自查表。

### §5.4 用户裁决与本批处置

- 用户裁决：**index 保持精简，不加时间戳**；「在 index 中找到可用内容后不进一步查找原文件是正常行为」。
- 因此本批**不把时间误判列为待修项**，也不改 `LIF_CURRENT_INDEX.md` 或 ROUND232；只登记为可接受风险。
- 后续引用口径：`I-R140` 的日期按 **2026-06-14** 计；若再出现「六年前」类表述，以本档 §5.1 为准。
- 若未来愿意加一行低成本护栏，可选「索引条目只对 dated 结论写相对时间」；本批不实施。

## §6 五问题根因（0.9.7）

### §6.1 `/rewind` 标记栏

- `main.js:845` 的 `ui.rewindRecall` 只调用 `sessionRecall` 并 `addSystemMessage`；**不截断 `state.content.items` / `state.marker`**。
- `state.js:146` 的 `addUserMessage` 只对「最近一条用户消息」去重；撤回后重发同文，新输入被吞，因而没有新的 `▸ Tn 输入` 锚点。
- `main.js:540` 的 `openConversation` 重置 items/marker/turnCounter 后，`main.js:577` 对所有 run journal **全量重放**；被撤回的 run 也会回来。
- marker 存的是 `itemIndex`（数组下标）；截断/重放后下标漂移，`scrollToItem` 定位到错卡。
- 327 批只修了「恢复后 turn 号取 `RUN-...-n`」，**没有修撤回截断**；所以 0.9.7 仍有此 bug。

### §6.2 长程进程

- `run_terminal_cmd` 是 call-scoped 进程树：调用结束/超时回收；默认 600s；`is_background` 不显式 `timeout:0` 仍会被默认超时杀掉。
- 模型不是「不会用」，而是在用一个不适合 4 小时任务的接口；它退化成 `Start-Process` 是为了保命，代价是 PID/日志跟踪丢失。
- 0dh 的 S1–S3 已达成（会话级 terminal backend、`lifetime=session`、30h 兜底），但 **S4 真机未收口**；本批不把它写成已修。

### §6.3 CDP discovery refused

- `cdp.rs:617-622`：`poll_devtools_active_port` 拿到端口文件后，只调用一次 `http_get_json(port, "/json/version")`；Chrome 先写 `DevToolsActivePort`，HTTP 端点尚未就绪，于是 `connect ... 10061`。
- 用户日志显示：端口文件 19:55:04 已写，19:55:06 失败，19:58:25 同一端口成功——典型 readiness race。
- 「不安全」提示是 Chrome 对 `--remote-debugging-port` / `--remote-allow-origins=*` 的 infobar，不是 orz 通道被劫持。
- 修法（未实施）：deadline 内重试 `/json/version`；校验端口文件 mtime/PID；能用 `--remote-debugging-pipe` 就用 pipe；否则在 UI 里解释 infobar。

### §6.4 切会话不保活

- `acp_pump.rs:156-161`：WS 或 agent 任一侧断开，立即 `child.start_kill()` + `wait`；`spawn_agent` 设 `kill_on_drop(true)`。
- 一 WS 对一 `orz --stdio` child；切会话关 WS，child 死；刷新走 `session/load`，所以看起来「刷新能恢复」。
- 工作区单会话 409（`该工作区已有活动连接`）阻止第二个 WS 接管。
- 327 批修的是「断后恢复」（tool pairing / archive auto-load / marker turn），**不是保活**。
- 修法（未实施）：child 生命周期与 WS 解耦；session supervisor；断线 detach，重连 attach；心跳；child 死了再 `session/load`。

### §6.5 `session/new` 自动归档

- `acp_server.rs:3511-3518`：`handle_session_new_with_options` 固定传 `sweep_enabled=true`。
- `attach_session_with_options` 的 live 集合只取 `self.sessions.keys()`；当前进程未 attach 的会话都被当 leftover，收卷进 `archives/` 并删源。
- `ORZ_GSA_HYGIENE=0` 只关启动 sweep，不关 close/on-demand 打包。
- 这与用户「会话生命周期控制归用户手动」的主权原则冲突；0dd/0cy 已有相关裁决，但 `session/new` 这一刀未收。
- 修法（未实施）：交互式 `session/new` 传 `sweep_enabled=false`；sweep 只留给 CLI/临时/显式清理。

### §6.6 统一投影层判断

问题 1 和问题 4 的标记栏是同一个投影层缺陷：前端把 run journal 当唯一真相全量重放，marker 存数组下标；侧车被 `/rewind` 截断、或会话被重建后，下标/turn 都变了，但 marker 没重建。只修一个不够，projection 要 recall/session aware。

## §7 ZCode：能借鉴，不能直接搬

- **许可证**：ZCode 为 **Apache-2.0**（[LICENSE](https://github.com/zai-org/ZCode/blob/main/LICENSE)），允许修改、商用、再分发；保留 LICENSE/NOTICE、修改文件加标注即可。
- **架构**：ZCode 桌面端是 Electron（README 的 `packages/desktop`）；据前序调研，内置浏览器依赖 Electron 的 `WebContentsView`＋CDP＋Browser Use 插件。orz Web 是普通浏览器应用，页面里嵌不了一个外部 Chrome 窗口；**这部分没有可直接搬的组件**。
- **可借的**：UX 与协议——标签页、地址栏、截图/标注、接管按钮、会话绑定；CDP 的 `Page.startScreencast`＋`Input.dispatchMouseEvent/KeyEvent`。
- **落地路径**：`<canvas>` 显示 screencast 帧；输入注入回 CDP；检测到验证码/登录时暂停，把真实 Chrome 窗口前置，用户完成后恢复。真实窗口比 canvas 更可靠，CDP 点击仍可能被风控识别。
- **「不安全」提示**：ZCode 用 Electron 自带 Chromium，所以没有你本机 Chrome 的 remote-debugging infobar；orz 用的是你本机 Chrome，这个提示是 Chrome 的。要彻底消掉，得走 `--remote-debugging-pipe` 或换壳；否则接受它并解释。
- **结论**：借设计/协议，不移植 Electron 代码；先做「CDP screencast 面板＋真实窗口接管」，成本最低。

## §8 本轮回应与裁决

| 项 | 处置 |
|---|---|
| 0.9.7 记账 | 确认；此前任何 0.9.6 对照推断作废；327 档「待重启 web」边界就地更新 |
| 54d12bcc | 记录为「失败 run 重置」；不写成恢复；继续时从磁盘重读 |
| 时间误判 | 记录为可接受风险；按用户裁决不加 index 时间戳；不改 ROUND232 |
| 五问题 | 根因已定位；本批不修；归属既有 0dk/0di/0dh/0cy/EVIDENCE-LOCAL-BROWSER |
| ZCode | 记录为设计参考；不立项、不搬码；未来单独立项 |
| 计数 | **63 不变**（本批零新增项、零关闭项） |
| 代码 | **零改动**；未提交、未推送 |

修复优先级建议（待用户裁决，不在本批实施）：

1. **P0** 会话保活/重连（0dk）＋`/rewind` 截断（0di）。
2. **P1** CDP 重试（EVIDENCE-LOCAL-BROWSER）＋`session/new` 不自动 sweep（0cy/0dd）。
3. **P2** 进程注册表/显式长程通道（0dh S4）＋时间溯源护栏（本批不实施）。
4. **P2** ZCode 式浏览器面板（单独立项，先 CDP screencast＋接管）。

## §9 路由与台账

| 文件 | 改动 |
|---|---|
| `docs/audits/328_0_9_7_FIVE_ISSUE_REVIEW_AND_54D12BCC_RECOVERY_2026-10-21.md` | 本档 |
| `CLI_PROJECT_INDEX.md` | v4.297 → v4.298；头行 328 摘要；新增 `AUTH-0.9.7-FIVE-ISSUE-REVIEW`（`reference`） |
| `TODO.md` | 新增 328 复核行（只读指针；不改计数） |
| `docs/BACKLOG_AND_PRIORITIES.md` | 本批指针行 328；保留 327/326 前指针 |
| `docs/BACKLOG_AND_PRIORITIES_2.md` | 新增 §1.259 328 批流水 |
| `LIF_CURRENT_INDEX.md` / ROUND232 | **不改**（用户裁决：index 不加时间戳） |

## §10 自查（按 `self_check_protocol.md`）

| 检查 | 状态 | 证据 / 说明 |
|---|---|---|
| 数据核查 | 已核实 | 54d12bcc 计数由 Python 逐行解析 JSONL 得出；进程时间由 CIM/Get-Process 实测 |
| 代码链路 | 已核实 | 逐段读 `main.js` / `state.js` / `acp_pump.rs` / `cdp.rs` / `acp_server.rs`；未执行修复 |
| 参数/实验 | 不适用 | 本批零实验、零训练、零源码改动 |
| 环境核查 | 已核实 | `D:\CLI` 为 CLI 项目；`.gsa` 在用户研究区；0.9.7 二进制与进程一致 |
| 严谨性 | 已分级 | 54d12bcc/时间日期/进程时间为事实；CDP race 与时间外推为推断；ZCode 结构依据 README/前序调研 |
| 事实-指标映射 | 已拆开 | 「1 条 user、0 assistant」直接测到；「失败 run 过滤」由 327 档规则＋计数一致支撑；「模型用轮次外推」为 agent-inferred |
| 独立性 | 已执行 | 用户「0.9.7」经进程/二进制复核；用户「不是幻觉就好」作为裁决记录，不作为事实证据；「六年前」仍标为错误断言 |
| 偏误性 | 已检查 | 未因用户接受而删除错误；未把 327 档的「待重启 web」当现状 |
| 表达检查 | 已执行 | 路径、日期、计数、单位逐项核对；未使用 emoji |
| 未核风险 | 保留 | CDP 竞态未在本批真机复现；ZCode 内置浏览器结构未在本批重跑 ZCode；0.9.7 Web UI 未做本轮手工复测；54d12bcc 未重跑验证「继续后模型实际看到什么」 |

## §11 入口

- 本档：`docs/audits/328_0_9_7_FIVE_ISSUE_REVIEW_AND_54D12BCC_RECOVERY_2026-10-21.md`
- 前批：[`327 档`](327_RECOVERY_TOOL_PAIRING_AND_ARCHIVE_RESUME_2026-10-21.md)
- 会话：`C:\Users\1\OneDrive\Desktop\新建文件夹\.gsa\runs\RUN-54d12bcc-0\events.jsonl`
- 侧车/归档：`...\.gsa\conversations\54d12bcc.json` / `...\.gsa\archives\54d12bcc.json.gz`
- 代码：[`main.js`](../../orz/crates/orz-web/assets/app/main.js) / [`state.js`](../../orz/crates/orz-web/assets/app/state.js) / [`acp_pump.rs`](../../orz/crates/orz-web/src/acp_pump.rs) / [`cdp.rs`](../../orz/crates/orz-host/src/local_browser/cdp.rs) / [`acp_server.rs`](../../orz/crates/orz-host/src/acp_server.rs)
- 路由：[`CLI_PROJECT_INDEX.md`](../../CLI_PROJECT_INDEX.md) / [`TODO.md`](../../TODO.md) / [`BACKLOG`](../BACKLOG_AND_PRIORITIES.md)
