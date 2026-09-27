> 状态：`current`；批次＝**0br S2 全面审查与处置**（BACKLOG/TODO P1-0br；2026-09-25 用户令「请对审查出的全部问题进行处理」；审查令「请对当前实现的0br进行全面检查，包括设计合理性、实现合理性、设计与实现的符合性」）；性质＝审查处置回执（三路并行只读深查＋主会话亲核＋全部问题处置）。**未提交／未推送／未重建载体**（沿 S2 口径，记账随提交批办理）。

# 0br S2 审查处置回执（设计·实现·符合性三面 × 全量问题处置）

## 1. 审查范围与方法

- **审查面**：设计＝综合稿＋三稿＋ADR §2.4/§2.6/§14.78＋BACKLOG/TODO 0br；实现＝`orz-web` 七文件 1,483 行 Rust＋前端 12 文件约 2,270 行＋orz-bin 接线；符合性＝S1/S2 账面主张逐条对账＋协议合规（vendored digest 重算、NOTICES 逐字、组件登记 65→69）。
- **方法**：三路并行子代理（设计合理性／实现合理性〔含本机复跑测试与实机冒烟〕／符合性对账），P0 与关键 P2 由主会话逐条亲核（读码＋grep＋实机探针）后才落处置。

## 2. P0 处置（两件，均已修复并实证）

| # | 发现（审查原文） | 处置 | 实证 |
|---|---|---|---|
| P0-1 | `main.js:213` 对未声明变量 `actions` 赋值——ES module 严格模式抛 ReferenceError，模块求值中断，`bindDialogActions`/`bindKeyActions`/输入行监听/`boot()` 全不执行，**前端整体死加载**；S2 验证止步 HTTP/WS 协议层、UI 从未在浏览器加载的实证 | `const actions = { ... }` 声明补齐（main.js）；**新增前端最低冒烟门** `orz-web/tests/frontend_smoke.mjs`（零依赖 node 直跑：全部 10 个应用模块 DOM-stub 导入断言＋纯逻辑钉子），此类缺陷从此机器可捕 | 冒烟门输出「全部 10 个应用模块导入无求值期异常」；`node --check` 10/10 净 |
| P0-2 | 桥打印 `http://bind/#<hex>`（裸 hex fragment）而前端按 `#token=<hex>` 键值解析（`main.js` boot／`journal.js` 尾通道同病）——**按启动指示打开无法进入工作台** | `lib.rs` 打印改 `#token={token}`；注释双向锁定（lib.rs ↔ main.js）；另修 `--addr …:0` 端口 0 误导——URL 打印移入 `serve()` 实际端口确定之后（server.rs） | 实机输出 `打开 http://127.0.0.1:54960/#token=48b9…`；用打印 URL 的令牌直连 `/api/boot`＝200 |

## 3. P1/P2 修复清单（全部落码）

| 级 | 发现 | 处置（file） |
|---|---|---|
| P1 | Markdown 净空：escape-first 封不住 Markdown 链接语法（方括号文本＋圆括号地址指向 javascript: 协议），经 parse 仍产出可点击的 a 标签 href；唯一防线 `a[href]` 前缀剥除被注释误标「纵深兜底」；img src 无净空；外链图片与「零 CDN、离线可用」相抵 | md.js 重写：**净空升级为承重防线并如实注释**——链接 scheme 白名单（非 http/https 一律剥成纯文本）、`rel=noopener noreferrer`＋`target=_blank`、`<img>` 整体剥成 alt 文本、`script/style/iframe/object/embed` 移除、`on*` 属性清除；浏览器级复验列进 S3 判据增补 |
| P1 | 代码块双重转义：escape-first 使围栏内 `<b>` 显示为 `&lt;b&gt;`，「按字面呈现」不成立 | md.js：`<code>` 内容经 RCDATA textarea 做一次实体还原（安全：textarea 内容不按 HTML 解析） |
| P1 | 权限弹窗 Esc 死锁：Esc 一律 `closeModal()` 而 Promise 只能由按钮 settle ⇒ agent 的 `session/request_permission` 永不应答（ACP 无超时字段）；并发第二请求覆盖第一请求同样永挂 | dialogs.js：权限应答结算语义——活动权限请求离开弹窗（Esc 关闭/被其他弹窗替换/被并发请求顶掉）一律**如实结算为 `cancelled`** 回传 agent；选项/取消按钮显式应答优先；不排队（v1 语义一次一个） |
| P1 | 回放后实时尾永不恢复：`openRunReplay` 不清 `currentRunId`，`startTail` 早退；「后退」与探索器「当前」均失效 | main.js：`openRunReplay` 复位 `currentRunId`；新增 `returnToLive()` 统一清场＋重灌（后退/`workspace://live` 两路径共用） |
| P2 | `/assets/{*rest}` 实际无 Host 门（账面与 `server.rs` 注释均称有） | server.rs：assets 处理器补 `host_ok`；模块头注释同步——账面口径自此为真 |
| P2 | `/ws/journal` 升级无 Origin 校验（判据⑦口径过宽） | server.rs：gate 链抽成可直测的 `ws_gate`（token＋Host＋Origin），两 WS 路由共用；axum `WebSocketUpgrade` 在 oneshot 下因缺 hyper `OnUpgrade` 必 426，故门链以纯函数钉住＋实机 WS 探针复核 |
| P2 | 入站 WS 帧无显式上限（axum 默认 64 MiB，超大粘贴可杀会话）；agent stdout 单行内存无界 | server.rs：`/ws/acp` 显式 16 MiB、`/ws/journal` 1 MiB（`max_message_size`/`max_frame_size`）；acp_pump.rs：`read_line_bounded` 手写有界行读取（64 MiB cap，超限拆对）＋尾随 `\r` 处理＋未终结尾行不转发 |
| P2 | 会话槽非 RAII：泵任务 panic 时槽永持、服务永久 409 | acp_pump.rs：新增 `SlotGuard`（Drop→spawn release；releaser 钩子幂等），server.rs 升级回调改用守卫 |
| P2 | 无界渲染：每事件全量重绘＋条目无上限，长会话冻结风险 | state.js：`MAX_CONTENT_ITEMS=3000`/`MAX_TOOL_ENTRIES=2000` 封顶、裁最旧计数（`droppedItems`/`droppedEntries`）＋widgets 顶部提示；main.js：journal 尾与 ACP 流式回调经 `requestAnimationFrame` 合帧节流；journal.js：回放字节上限 64 MiB。形态豁免登记进综合稿 §8 |
| P2 | 用户消息全历史去重过宽（隔轮发相同消息被吞） | state.js：去重窗口收窄为「最近一条用户消息」（journal 双投递防护语义保留） |
| P2 | `findInConversation` 未约束用户正则（ReDoS/抛错面） | main.js：正则元字符转义，字面匹配；dialogs.js placeholder 同步 |
| P2 | `orz web --stdio` 被全局 `--stdio` 匹配吞掉、静默进 stdio | orz-bin main.rs：web 分发提前到 `--stdio` 全局匹配之前；`orz web --stdio` 现由 parse_args 显式报「未知参数」（实机验证） |
| P2 | 刷新＝杀活动 run 无任何提示（设计面 B2） | main.js：`beforeunload` 守卫（有活动 ACP 连接时浏览器确认）；「v1 已知限制＋恢复路径留 S3/S4」登记进综合稿 §8 |
| P2 | 判据④缺口：Tab/Shift+Tab 无处理、Alt+字母仅 Alt+H、九菜单无访问键 | keymap.js：Tab/Shift+Tab 区内焦点移动（`cycleFocusInRegion`，五区）；Alt+F/E/K/D/O/S/R/V 菜单访问键（Alt+H 保持直达帮助）；main.js/widgets.js：`openMenuByName` 承载；Help ←/→ 切页已有（核实） |
| P2 | 对象 URI 六 scheme 仅两种有分支、`conversation://` 不可点（判据①口径） | main.js：`conversation://` 只读说明分支；`source|claim|adapter|artifact://` 显式「v1 无投影面」答复不静默；widgets.js：会话行可点。AddressBar/FindBar 弹窗化形态选择登记进综合稿 §7 |
| P3 | 令牌比较非常量时间 | security.rs：XOR 折叠常量时间比较（成本为零，消除争议） |
| P3 | `authority_host` 不校验端口数值段（`127.0.0.1:1.evil.com` 过门）；`[::]` 多余放行 | security.rs：端口段必须全数字否则整体拒绝；`[::]` 移出白名单；钉子测试 5 例 |
| P3 | run_id 不拒 Windows 保留设备名（`NUL` 可经字符集） | security.rs：CON/PRN/AUX/NUL/COM1-9/LPT1-9 大小写不敏感拒绝（`open(".gsa/runs/NUL/…")` 会绑 NUL 设备）；实机探针 `/api/run/NUL/events`＝400 |
| P3 | `/api/runs` 每请求对全部 run 探针（IO 放大）；终态探测窗 64 KB 有漏报 | runs.rs：先元数据排序截断、再仅对最近 50 个 run 内容探针，其余 `status:"archived"`；终态窗扩到 256 KB；探针上限钉子测试（55 个 run → 恰 50 个带终态、5 个 archived） |
| P3 | vendored 钉子不完整：XP.css 无版本钉子；MANIFEST digest 无构建期校验 | assets.rs：XP.css v0.2.6 头钉子＋**MANIFEST.sha256.txt 逐条 digest 对嵌入字节重算校验**（sha2 为既有 workspace 依赖，零新增外部件） |
| P3 | `replayRun` 死条件（自比较恒真） | journal.js：循环改为 `next_offset` 推进判定（无进度/超上限即止） |
| P3 | 英文残留：`· success`；aria-label 英文；`turn N` | projection.js：`· 成功`；index.html：窗钮补中文 `title`（aria-label 值为 98.css 属性选择器锚点、保留英文，头注释登记）；`turn N` 为三稿字面格式（`┌ [角色] · turn N`）**保留不改**，S3 词汇定稿时统一 |
| P3 | 菜单「编辑」与补充稿 §13 冻结转换项（`Edit`→「编辑模式」）冲突 | state.js/widgets.js：菜单改「编辑模式」（§13 延期面仅覆盖最终词汇、不覆盖该转换；S3 词汇定稿收口） |

## 4. P3／观察项登记为接受（不落码，理由在案）

- **令牌经 URL query（`?token=`）**：本 crate 无任何请求 URI 日志（axum 无 TraceLayer，tracing 只打 pid/关闭原因/文件路径）；残余面＝浏览器历史/同机进程，回环单用户信任域内可接受。升级路径（WS 首消息认证/子协议头）留 S4。
- **`slice_events` 静默丢坏行**：append-only 假设下可接受，写方为我方 journal；检测面不建。
- **符号链接跟随**：`.gsa` 为本地产物面、只读 API，构造符号链接需本地文件系统权限；不建 canonical 解析。
- **`request()` 无超时**：`session/prompt` 天然长活（整个任务），超时反而破坏语义；WS 关闭路径已有 reject 兜底。
- **`btnQuit` 的 `window.close()`**：非脚本打开页无效——已有横幅兜底文案。
- **快照选择器不建面板**：数据面非 ACP 协议（S2 §2 偏差 3 已登记）；账面表述收窄为「提示性横幅」，接线随 S3/S4 裁决（综合稿 §7 形态选择登记）。
- **XP.css 引用的 Perfect DOS VGA 字体未 vendored**：XP 主题 v1 未接线，无实际影响；接线时随搬随登记。
- **无障碍**：v1 暂缓（retro 主题低对比、焦点可见性），S4 盘点时定档。
- **98.css 上游低活跃／前端无框架可维护性**：vendored 已缓解；观察项。

## 5. 登记面（文档收口，全部完成）

1. **ADR-0010 §14.78 条 4 勘误（v1.81）**：MIT 件声明落点「父仓 `THIRD-PARTY-NOTICES`」→「分发仓（orz 子仓）`orz/THIRD-PARTY-NOTICES`」——父仓根无该文件，材质上 S2 落点正确（许可声明随分发单元走），字面偏离已按补记惯例勘误；判据⑥基础自此成立。
2. **BACKLOG 0br**：协议与登记款同步勘误；新增「S2 全面审查处置」段（本批摘要＋读数＋指针）。
3. **综合稿 §1**：「原文口径」→「五句逐字转录以 ADR §14.78 为准」（原表述实为转写）。**§7**：增 R-7（语法高亮后置＝对 ContentPane 稿 §3.1 的显式豁免；LaTeX/mermaid 三稿原文未要求）＋两处形态选择登记（AddressBar/FindBar 弹窗化；快照横幅）。**§8**：三稿豁免清单点名（Win98 不复刻条款、Windows-only/视口等 TUI 约束、Edit→编辑模式已落码、历史实现决策）；刷新/断线已知限制；marked 供应链策略；投影双实现同步纪律（projection.rs↔projection.js 同批同步）；渲染内存上限形态豁免。
4. **TODO P1-0br**：S1/S2 勾选＋处置批新勾选项＋判据①–⑦定稿口径与 S3 增补（IME 组词、刷新提示、长会话渲染读数、md.js 净空浏览器复验）。
5. **THIRD-PARTY-NOTICES**：marked 版权行补上游括注 URL（`MarkedJS (https://github.com/markedjs/marked)`），恢复逐字。
6. **S2 回执勘误指针**（沿「补勘误指针、不改原回执」惯例，本档即指针）：§1.1「嵌入表 19 项」实为 20 条 `asset!`（/与/index.html 同文件，唯一文件 19）；§1.1「静态…无令牌但 Host 门」在处置前不成立（本批补齐）；§3 冒烟读数真实但只覆盖协议层（两个 P0 所在层未覆盖）；§4 判据①③④「已落码/壳已齐」表述过宽（以本档 §7 重述为准）。

## 6. 验证读数（本机 Windows，2026-09-25）

- **Rust**：`cargo test -p orz-web`＝**31 通过 / 0 失败**（S2 时 21＋新增 10 钉子：常量时间与令牌门、端口段/保留名/`[::]` 拒绝、assets 伪 Host 401、`ws_gate` Origin 三态、有界行读取换行/EOF/cap、SlotGuard drop 释放、探针上限 archived 计数、MANIFEST digest 重算）；`cargo clippy -p orz-web --all-targets`＝**0 警告**；`cargo fmt -- --check`＝净；`cargo build -p orz-bin`（debug，`PROTOC=orz/bin/protoc.exe`）通过。
- **前端冒烟门**：`node tests/frontend_smoke.mjs`＝全绿（模块导入 10/10、去重窗口、内容/清单上限与计数、键盘分发、MenuBar「编辑模式」词表、权限三条应答路径、renderMarkdown 求值）；`node --check` 10/10。
- **实机 HTTP 探针**（`orz web` 真进程）：`/`＝200、`/assets/app.css`＝200；**`/assets/app.css` 伪 Host＝401**（新门）；`/api/boot` 无令牌＝401、带令牌＝JSON；URL 编码遍历 run_id＝400、`NUL`＝400（新门）；伪 Host＋端口段 junk（`127.0.0.1:59168.evil.com`）＝401（新门）。
- **实机 WS 探针**（node 24）：journal 尾伪 Origin＝**拒**（新门）；合法令牌无 Origin＝连上且 2/2 行逐字到达（首行 `run_started`）；ACP 泵 `initialize`→真回路 `{"id":1,"protocolVersion":1,…}`；连接关闭后无孤儿（tasklist 仅服务器自身进程）。
- **分发序**：`orz web --stdio` 显式报「未知参数 --stdio」（不再静默进 stdio）。

## 7. 判据状态重述（S1 §7 ①–⑦；S3 放行前置门＝本批）

| 判据 | 处置后状态 |
|---|---|
| ① 三稿区块齐备 | 修正口径：九全局区域＋模态面在位；AddressBar/FindBar 以弹窗化形态承载（综合稿 §7 登记）；对象 URI 六 scheme 显式路由（四种无投影答复不静默）；快照为横幅。语义/观感对照仍归 S3 |
| ② 来源件可核 | ✅（文件头标注＋MANIFEST＋**嵌入表 digest 构建期校验**＋组件登记＋NOTICES） |
| ③ 默认中文＋Help 分页 | 已落码（`· success`→`· 成功`；窗钮中文 title；`turn N` 为三稿字面保留）；S3 浏览器复验 |
| ④ 核心操作全键盘可达 | 已补齐（Tab/Shift+Tab 区内、Alt+字母菜单、F6/Esc/Ctrl+L/F/Z、←→ Help 页）；S3 复验 |
| ⑤ UI 零执行事实 | ✅（权限选项透传、无本地判定、restore 不在前端；核查方式见 TODO 判据款） |
| ⑥ 协议面齐备 | ✅（落点＝orz 分发仓；ADR v1.81 勘误后账面一致） |
| ⑦ 分发面零外部可达 | ✅（回环-only＋令牌＋Host/Origin 双门全覆盖＋WS 帧上限；本批复验实测拒绝面） |

## 8. 实机走查补笔（2026-09-25 用户令「现在的UI形式的单独拉起来看一下」；浏览器级首读）

真实浏览器（IAB Chromium，1440×900，`orz web` @ `D:\CLI` 真实 30-run 工作区）逐区块走查，**又抓到三件代码审查层测不出的问题并当场修复**：

| 级 | 发现 | 处置 |
|---|---|---|
| P0 级 | **四个弹窗正文游离**——`openModal` 创建 `window-body` 后从未挂进窗口节点，`showPermission`/`openFind`/`openProperties`/`openCommand` 在真浏览器只渲染空标题栏（Help 恰好自挂内容故幸免；stub 冒烟测不出"挂载"语义，**实机截图抓到**） | dialogs.js：`openModal` 统一 `win.appendChild(body)`；openHelp 改渲染进 body；冒烟门新增"正文挂载"回归钉（打开态断言 window-body 在窗口节点内） |
| P2 | **静态资源无缓存策略**——载体重建换前端后浏览器仍启发式缓存旧 JS（走查中实锤：重建后弹窗仍为旧码） | server.rs：`serve_asset` 响应补 `Cache-Control: no-store`（嵌入资产随载体变更，本地回环无缓存收益） |
| P3 | Esc 关弹窗不收起打开中的菜单下拉（模态层后残留） | main.js `actions.closeModal` 顺带 `closeMenus()`（widgets 导出） |

**走查通过面**（截图逐项）：98.css 窗体/窗钮、九菜单（含「编辑模式」）、两行顶栏＋右对齐摘要＋地址（`workspace://live`↔`run://…` 随导航切换）、探索器 30 run 实数据（completed/running 状态）、「有边框卡片＝对话（点击标题展开，框内 Markdown 加粗/列表/换行正常）／无边框工具行（`· 成功` 中文）」、pin 展开、标记栏、六段状态栏、输入行；Help 六页（F1）、命令/位置弹窗（Ctrl+L，修复后输入＋按钮齐全）、属性弹窗（七行只读元数据）；**历史 run 回放→「后退」→实时整段重灌**（C-2 修复实机闭环）；Alt+D 菜单访问键； Bridges「桥接已连接」横幅。刷新守卫（beforeunload）在受控浏览器不可视验证，留 S3。

**走查反馈批（同日，用户看 UI 后三点反馈，全部落码并实机复验）**：

1. **标记栏锚点扩展（用户令"锚定每个用户输入和模型输出，便于快捷跳转"）**——原实现仅锚用户输入（`▸ L{n}`）＋搜索命中；现每条模型输出各产 `◆ T{n} 输出` 锚点（同轮多段加 `#序号`），用户锚点改 `▸ T{n} 输入`，三级配色（藏青/紫/橙）；点击跳转**顺带展开折叠卡**；`scrollToItem` 修正截断提示行的偏移；`clearHits` 只清命中、保留锚点。注：主稿 ContentMarker 原口径"用户位置＋搜索命中"由本用户令扩展。
2. **探索器分组折叠（用户令）**——"运行历史/会话"两组表头可点击/Enter 切换 `▾/▸`，行列表随折收展（`state.explorer` 会话内态）。
3. **空白模型卡根因排查（用户问"UI 问题还是空白内容被算成输出"）**——**都不是**：journal 实证该 run 恰 2 条空正文 `model_output`（各带 1 个工具调用，text 为真空非空白字符），与 2 张空白卡一一对应；根因＝投影层 `model_output` 分支对"纯工具调用、无正文"的轮次**主动创建占位空卡**（`else if (calls.length) → addModelMessage('')`）。已改为不留空卡（仅复位流式指针），冒烟门新增空卡根除＋锚点钉子。

**走查反馈批·二（同日晚，用户复看 UI 后三点反馈，全部落码并实机复验）**：

1. **输出锚点改色（用户反馈"红色有警告意味、晚上瘆人"）**——`mk-model` 由紫红 `#600060` 改深绿 `#006040`；输入保持藏青、命中保持橙。
2. **输出锚点只标最终正式输出（用户令"忽略流式输出"）**——中间轮/流式段不再产锚点（`addModelMessage` 去锚），五个终态分支（finished/failed/cancelled/invalidated/terminated）统一调 `vm.anchorFinalOutput()` 回溯为最后一张有正文的模型卡定**「◆ T{n} 最终输出」**锚；锚点栏从"每轮一条"收敛为"每任务一条"，点击仍展开＋跳转。冒烟钉同步改写（运行中零输出锚／终态恰一锚／指向最后正文卡）。
3. **Markdown 表格窗口适配（用户反馈表格溢出）**——md.js 净空阶段把每张 `<table>` 包进 `.table-wrap` 横向滚动容器（`overflow-x:auto`＋`min-width:100%`），宽表在卡片内滚动不撑破窗口；实机回放含表格 run（3 张表）验证通过。

**走查反馈批·三（同日晚二轮，用户再两点反馈，已落码并实机复验）**：

1. **输出锚点换深蓝（用户令"统一界面颜色"）**——`mk-model` 深绿 `#006040` → 界面主色深蓝 `#000080`（与输入锚同色异符号，98.css 同族主色）。
2. **表格改单元格内换行（用户令"表格内部不能换行吗"）**——横向滚动降为兜底：`table` 锁 `width:100%`，`td/th` 设 `white-space: normal`＋`overflow-wrap: anywhere`，整个表格直接装进当前窗口。**根因注意**：98.css 表格原语自带 `white-space: nowrap`（vendor 不改），须在本层样式覆盖——首轮只加 `overflow-wrap` 未覆盖 nowrap 故无效，实机 `getComputedStyle` 诊断后补齐。
3. 附带实机确认观察项①（探索器 5 s 刷新前 run 列表为空）的真实影响，维持 S3 批消化。

**走查反馈批·四（同日晚三轮，用户再两点反馈，已落码并实机复验）**：

1. **弹窗统一关闭"×"（用户令"弹窗类都没有明确的关闭的 x…这是 web 端"）**——新增 `modalTitle()` 统一标题栏构造器：98.css `title-bar-controls` 原语＋`aria-label="Close"` 钮（vendor 属性选择器锚定样式，中文提示走 `title`），五个弹窗（权限/Help/查找/属性/命令）全部接入；关闭＝Esc 同语义，权限弹窗按"取消"如实回传 agent。冒烟钉：关闭钮存在＋点击＝cancelled。
2. **弹窗灰色遮罩边收窄至 1/3（用户令）**——`.modal-window` 560px/82vh → **80vw/94vh**（1440×900 下四周灰边 440px→144px、81px→27px，恰约 1/3）。过程中实机截图又抓到一处回归：body 先挂导致标题栏被挤到弹窗底部——`openModal` 挂载顺序改为"buildBody（标题入 win）→ 再挂 body"。
3. **Help"快捷键"页表格化（用户令"和其他部分的设计统一"）**——原为等宽文本块（`white-space:pre`），改与其他五页一致的两列表格；顺手把审查批新增的 **Tab/Shift+Tab 区内移动**、**Alt+字母 开菜单** 补进表（原表缺），并新增 `help-table` 样式（首列随内容自适应，不用属性弹窗的 40% 定宽）；`content` 文本块渲染分支随之退役。

**走查反馈批·五（同日晚四轮，用户提出会话/运行定位设计问题，已考证原设计并落码）**：

用户问题：① `RUN-aa18ef7e-0/1` 同一会话的两次运行被割裂显示；② "运行历史"与"会话"两分组定位重复（绝大部分运行本就是无头对话）——原设计咋设计的？

**考证**：三稿交互模型稿 §ExplorerPane 原文＝"stable navigation over workspaces, sources, **runs**, artifacts…"，是**对象树**（候选根组 Workspaces/Tasks/Sources/Runs/Artifacts…），不是平铺清单；会话语义按 ADR §14.52/§14.56（会话卷）＝一个对话跨多次运行（侧车 `.gsa/conversations/{session8}.json` 存 messages/黑板/LIF），run_id 自带 `{session8}` 即归组键。**S2 实现把对象树拍平成了两个清单，且 `/api/conversations` 的前端调用（`fetchConversations`）从未接线——"会话（0）"是接线遗漏而非真无会话**（实况 11 个侧车），两处均为实现偏差，审查对账时未查出（只对了 API 存在性，没对前端调用链）。

**处置（前端会话树）**：探索器改**按 session8 归组的会话树**——组＝会话（`会话（20）`，原 30 行平铺），组头 `▾ {session8}（k 运行 · 侧车）`，组内＝该会话全部运行（新→旧，点击回放不变）＋有侧车者尾行 `└ 侧车 conversation://{s8}`（只读元数据）；默认最新会话展开、其余折叠，点选状态记忆；ARC/RUN-CLI 同 session8 的狗粮对（9 对）随此自然归组。**同时修复 fetchConversations 接线遗漏**并让数据到达即重渲探索器——**走查观察项①（探索器 5s 滞后）就此闭合**。bridge 零改动。

**走查反馈批·六（同日晚五轮，用户对会话树再三点修正，已落码并实机复验）**：

1. **会话合并视图（用户令"同会话应该在同窗口中，是一整个长窗口，现在还是一个运行一个窗口"）**——`conversation://{session8}` 从"只读说明占位"升级为**真实投影**：把该会话全部运行按时间序（旧→新）连续重放进同一内容区，一次对话一整个长窗口；`后退`仍回实时。探索器会话组点击即进入合并视图（导航叶）。
2. **树内不再列运行/侧车（用户令"已经有标记栏了，完全不用展示出具体运行和侧车，用户层不会需要这个"）**——会话组行只显 `{session8}（k 次运行[ · 进行中]）`，运行行与侧车行移除；内容定位全权交给标记栏。
3. **锚点语义随合并视图调整**——`anchorFinalOutput` 不再清除其他运行的锚：合并视图下每个运行终态各定一个「◆ T{n} 最终输出」，同卡重复终态去重；配合每运行一个「▸ 输入」锚，整段会话的每次对话均可跳转。冒烟钉改写（两运行两锚并存、互不挤掉）。
4. **会话标题增强（侧车首条消息作组头可读名）按用户令搁置**（"后续可选增强这一项不急"），S3 批再看。
5. **会话归档投影面入 S3（用户令 2026-09-25"会话的归档和查看归档会话都要做的，请将其归入S3项"）**——agent 侧归档**已实现且在工作**（`orz-host/acp_server.rs`：`incremental_archive_due` 增量判定 → `.gsa/archives/{s8}.json.gz` 三键单包 ＋ `.milestones.json` 水位 ＋ `session_archive` 事件；实况 18 个文件），Web 端目前仅回放事件行"路过展示"。S3 补**只读入口**：桥归档 API（列 `.gsa/archives` 清单＋gzip 内摘要）＋会话组行"归档"标记＋归档内容只读浏览面板；与快照选择器、会话标题增强同批。已登记 BACKLOG/TODO 0br S3 款。

**走查观察项（登记不落码，S3 批消化）**：① 同文档锚点跳转（仅换 `#token`）不触发重载，旧实例持死连接仅横幅提示——令牌轮换 UX 留 S3。

## 9. 批序与边界

- 本批为 **S3 真机首读的前置门**：P0×2＋功能死锁两件＋净空收口修复后，Web 前端首次具备「可在浏览器打开并完成会话」的前提。S3 判据增补（IME/刷新提示/渲染读数/净空复验）已入 TODO。
- 依赖口径更新：orz-web 新引 `sha2`（既有 workspace 依赖，Cargo.lock 零新增外部包）；MANIFEST 校验测试首次消费。
- 本批**未提交／未推送／未重建载体**（沿 S2 用户令「暂时不提交不重建」；索引/BACKLOG/TODO 记账随提交批一并核）。

## 10. 入口

- 账面：[`0BR_S1 勘定`](0BR_S1_WEB_FORM_SURVEY_2026-09-24.md) / [`0BR_S2 回执`](0BR_S2_IMPLEMENTATION_2026-09-24.md) / [`综合稿`](../UI_FORM_CONSOLIDATED_DESIGN_2026-09-24.md)
- 裁决：[`ADR-0010 §14.78 ＋ v1.81 勘误`](../../adr/ADR-0010-vol-14-addenda-index.md)
- 实现：`orz/crates/orz-web/`（桥＋前端＋`tests/frontend_smoke.mjs` 冒烟门）
- 待办：BACKLOG/TODO `P1-0br`

关键词：0br 审查处置、P0×2 死加载与令牌通道、净空收口、权限应答结算、returnToLive、ws_gate、SlotGuard、有界行读取、探针上限、MANIFEST 钉子、前端冒烟门、ADR v1.81 勘误、三稿豁免清单、投影双实现同步。
