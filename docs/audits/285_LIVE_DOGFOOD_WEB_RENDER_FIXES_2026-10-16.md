# 285 批：真机走查（画图轮）三项渲染处置＋0dd／0de 真机读数登记（2026-10-16）

> 性质：**落码批**（web 层三件修正＋TUI 同族同步＋钉子）＋**只读取证**（用户个人真机使用轮）。
> 零跑批（本批未发起任何 orz 会话）；orz 子树落码提交（不推送），父仓登记面待提交批；
> 未重建、未发行，装机位保持 0.9.1。

## §1 由来（用户四项反馈）

用户 2026-10-16 个人真机使用后反馈（原文要点）：

1. 当前轮对话结束后会话会自动折叠，还要手动打开，比较麻烦；
2. 反例问询前的最终回复没有被 web 层直接静默折叠，而是依旧渲染出来了——实际上只要最后的输出即可，
   反例问询前的这一部分像其他的内容一样自动折叠即可；
3. 偶尔会冒出一串「机械审查」，不确定是实际需要还是渲染问题（随附原文片段：一串
   `[机械审查] （轮 0）`，两侧夹着 `请求头变化`／`[权限]`／`[票据签发]`／`[票据消费]` 行）；
4. 本轮测试落点＝`C:\Users\1\OneDrive\Desktop\画图`。

## §2 取证（只读；画图轮 journal）

落点 `.gsa/runs/RUN-e689a7ea-{0,1,2,3}`（同一会话 4 个 run，21:53–23:10，`turn_count` 1–4；
events.jsonl 168–629 KB）。

- **重放方法**：把 `orz-web/assets/app/{state.js,projection.js}` 的投影对真实 `events.jsonl`
  逐事件重放（最小 DOM stub，同 `frontend_smoke.mjs` 口径），打印内容区条目与折叠态——
  三条现象逐条复现。
- **事件面读数**：`mechanical_audit_update` 78／36／49／31 条；`counterexample_gate` 每 run 恰 1 条
  （`position=final_answer`、`once_only=true`）；`run_finished` ×4（其中 2 次 `intent=pause`／
  `reason=awaiting_response`）；`session_archive` **0 条**；`.gsa` 下无 `archives/`。

三处渲染事实（修复前重放实测）：

| 用户条目 | 重放实测 | 根因 |
|---|---|---|
| ① 收尾全折 | run3 终态后 70 张消息卡 `collapsed=true`、展开的模型卡 0 张（正式输出亦被折起） | `collapseNonWarnings()` 连对话卡一并折——与 CONTENT_PANE「折叠策略」原文「结束后：工具调用默认折叠」不符 |
| ② 门前草稿照常展开 | run3 `seq67`＝1,461 字符草稿在门前整篇展开渲染；门后 1,571 字符正式输出另起一卡 | 投影无「门口折叠」语义（`counterexample_gate` 只加一行系统行） |
| ③ 机械审查串 | `[机械审查] （轮 0）` 幽灵行（run3 内容区 12+ 条逐字同形） | 本事件 payload 是 `{kind, payload:{key,round,summary,anomaly}}`——比其余事件多一层嵌套；web 按平铺读 ⇒ 键空串、轮号缺省 0 |

真值对照（同批 journal 内层字段）：`cmd:call_00_Rl1FYugp8wEBx3r9WvCA4947（轮 0）：exit 1`、
`lif.domain_migration（轮 83）low_progress→normal@r83；累计 17 次`。

## §3 处置（三件落码）

- **F1 收尾折叠面**（用户第 1 条）：`collapseNonWarnings()` 改为**只折机器行**（工具清单），
  对话卡（用户／模型）收尾不再折——回归形态权威 CONTENT_PANE §7；阻塞/告警面本就不折。
  落点＝`orz-web/assets/app/state.js`＋同族 `orz-tui/src/view_model.rs`（双实现同步纪律）。
- **F2 反例门静默折叠**（用户第 2 条）：新增 `foldSupersededBeforeGate()`——门到即把**本 turn**
  的非告警模型卡与全部工具行静默折起（无提示行、不留痕），流式指针复位使门后正式输出自成新卡；
  用户输入卡不折、前几轮输出不动。接缝＝`projection.js` `counterexample_gate`＋
  `orz-tui/src/projection.rs` `TuiEvent::CounterexampleGate`。
- **F3 机械审查载荷**（用户第 3 条）：`mechanical_audit_update` 改按**嵌套读**（内层
  `key/round/summary/anomaly`），与 orz-tui `/payload/key` 指针同源；幽灵行根除，改为真键真轮号
  （例：`[机械审查] file:scripts/fig_lib.py（轮 1）search_replace 成功`）。
- **钉子**：`frontend_smoke.mjs` 新增两节（F3 嵌套载荷异常形／无异常形；F1＋F2 门折＋收尾只折
  机器行＋两轮锚点）；orz-tui 改写 `collapse_non_warnings_folds_only_tool_traces`＋新增
  `gate_fold_folds_current_turn_models_and_tool_traces_only`（view_model）与
  `counterexample_gate_folds_pre_gate_draft`／`terminal_finished_folds_machine_rows_only`（projection）。

## §4 本轮真机读数（0dd／0de S4 证据登记；不作闭合裁决）

- **0dd（run 尾自动归档退役）**：同一会话 4 个 run 零 `session_archive` 事件、`.gsa` 下无
  `archives/`；跨提示词（turn 1→4）且跨 77 分钟连续会话无自动归档 ⇒ 判据前段真机成立。
  **本轮未取证**：手动归档／回档／删除三键（用户未操作，留后续真机轮）。
- **0de（上下文治理摩擦综合）**：run2 压缩事件 `mode=model_summary`／`reason=model_selected`／
  `estimated_tokens_after=196332` ⇒ 投影估数非 0（D6 面真机成立）；全轮零
  `form=reminder_only_no_compressible`（无空闲开窗）；收束段无逼轮；`context_scale:first_block`
  与 `context_scale:192k` 两档模型面估算照常读数（175,748／192,217）。
- **本轮检索门读数（事实登记）**：四次运行的检索家族探针均为 `retrieval_enabled=false`
  （`incomplete.reason`＝「retrieval family disabled for this run (retrieval_enabled=false)」），
  `.gsa/activations/e689a7ea.json` 亦为 `retrieval_enabled: false`／`activations: []`／内外两角色皆空；
  全轮 journal 工具面只有 `run_terminal_cmd`（80）／`blackboard_read`（1）／`blackboard_write`（10）／
  `search_replace`（30）／`read_file`（17）／`grep`（1）——零 `browser_launch_result`、零
  `web_search`／`web_fetch`／`browser_read`。⇒ **该轮检索既非本地浏览器车道、也非检索子代理 HTTP 兜底，
  而是模型自驱终端的原始 HTTP**：会话文件内 40 条网络命令（`curl.exe`＋Python 后处理），主机含
  `doi.org`／`api.crossref.org`／`api.openalex.org`／`api.unpaywall.org`／`eutils.ncbi.nlm.nih.gov`／
  `api.semanticscholar.org`／`r.jina.ai`，与手工 HTML 搜索端点（`bing.com`／`html.duckduckgo.com`／
  `so.com`／`xueshu.baidu.com`／`sogou.com`／`search.cnki.com.cn`／`s.wanfangdata.com.cn`）及出版商页
  （`electrochemsci.org`／`sciencedirect.com`／`pubs.acs.org`／`patents.google.com`）；摩擦读数＝工具结果内
  429×10／404×9／403×4，终端非零退出 13 次（限流与付费墙由模型自行绕行）。
- 上列两条**均待用户裁决**；本批只作证据登记，计数不变 62。

## §5 勘误与观察项

- **勘误（索引）**：`CLI_PROJECT_INDEX.md` 尾部（413／414 行）两条 284 批头行副本系误加
  （同批头行本已在第 1 行），本批删除，恢复「head 面只留最新 1 条」纪律。
- **维护项（台账行龄，日界触发）**：门禁 `_check_ledger_slimming` 的 21 天行龄规则在 2026-10-17
  首次触发——`BACKLOG_AND_PRIORITIES.md` 第 12 行（分卷口径，内容自 2026-09-25 起未改）行龄 22 天；
  本批本条即按「复核维持」就地刷新（补 2026-10-17 复核标记，语义零改），门禁恢复 `valid: true`。
- **观察项 A（候选，不计数）LIF 域迁移抖动**：run1／2／3 开头各有一串 `lif.domain_migration`
  突发（5／9／16 条，同一轮内交替 `normal↔low_progress`；run3 为 `seq25–40` 全在轮 83）——
  这是用户所见「一串机械审查」的主要体量来源。机理（会话续跑时的域状态追平、抑或域机抖动）
  未勘定；是否立项待用户裁决。
- **观察项 B（候选，不计数）门后中间输出仍展开**：门后若仍有多段模型输出（run2：`seq291`
  ＝1,211 字符＋`seq300`＝1,374 字符），两卡均保持展开——本批只按用户令折「门前」部分。
  若需「任何时刻只留最后一张模型卡」，可再扩一条 supersede 规则（风险面：正式输出后若还有
  短补注，会把正文折起——故本轮不取）。
- **观察项 C（候选，不计数）机器行体量**：权限／票据／快照／机械审查逐条进内容区（run1 单轮
  66 次工具调用 ⇒ 百行级机器行）；本批只修 F3 的字段读取，未改体量口径。收束手段（如仅异常
  显行、或收尾整组折叠）待用户裁决。
- **观察项 D（候选，不计数）Web 桥未暴露检索启用门**：`retrieval_enabled` 系 fail-closed 默认**关**
  （唯一授权开关＝`--retrieval-enabled`／`ORZ_RETRIEVAL_ENABLED`），而 `orz web` 桥 spawn
  `orz --stdio` 时不携带该开关、前端亦无入口 ⇒ Web 工作台轮在未显式导出环境变量时恒走「无检索车道」
  （本轮即其形态）。是否给 Web 加开关（或把启用门提升为会话级可切）待用户裁决；本批不动。

## §6 验证读数

- `node crates/orz-web/tests/frontend_smoke.mjs` ⇒ **全部通过**（含新增两节）。
- `cargo test -p orz-web` ⇒ **47/47 通过**（与 278 批基线同数）。
- `cargo test -p orz-tui --lib` ⇒ **172 过／9 败**——9 条＝未触碰的 ACAF 预存失败集合
  （与 279 批记录的基线同集）；本批新增／改写 4 条测试全绿。
- `cargo fmt --check -p orz-tui` 触碰区净；`cargo clippy -p orz-tui` 零新增（唯一告警在未触碰的
  `acp_client.rs:330`）。
- 重放复验（画图轮 run3）：门前草稿 `collapsed=true`、正式输出保持展开、标记栏锚指向门后正式
  输出；机械审查行逐条带真键真轮号。
- 门禁 `python scripts/check_repository.py` ⇒ **valid: true**。

## §7 台账

- 本档；索引头行 v4.257 → **v4.258**（并删尾部重复头行 413／414 两行）；BACKLOG 本批指针行＋
  计数行（62 不变）；TODO 计数行＋P1 路由行 0dd／0de 注记；[`BACKLOG 第二卷 §1.225`](../BACKLOG_AND_PRIORITIES_2.md)。
- 形态面登记：`docs/UI_FORM_CONSOLIDATED_DESIGN_2026-09-24.md` §7 增 R-8（反例门静默折叠＝新增语义）
  与 R-9（收尾折叠施行面＝工具行，对话卡不折；此前两实现属偏差）。
- orz 子树（`crates/orz-web`／`crates/orz-tui`）落码提交，**不推送**；父仓登记面待提交批。
- 零跑批、未重建、未发行；装机位（`C:\Users\1\AppData\Local\orz\bin`）保持 0.9.1 不动。

## §8 关键词

285 批、真机走查、画图轮、RUN-e689a7ea、收尾只折机器行、反例门静默折叠、
counterexample_gate、机械审查幽灵行、嵌套载荷、payload.payload、折叠策略、
CONTENT_PANE §7、双实现同步、frontend_smoke、LIF 域迁移抖动观察项、
0dd S4 真机读数、0de S4 真机读数、检索启用门、retrieval_enabled=false、
模型自驱 curl 原始 HTTP、检索子代理未启用、Web 桥开关缺口、索引 v4.258、计数 62 不变。
