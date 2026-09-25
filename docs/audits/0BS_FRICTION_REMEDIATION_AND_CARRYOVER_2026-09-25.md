# 0bs 摩擦修复与承接勘定（2026-09-25）

> 批次：**P1-0bs**（0bm 轮摩擦与承接大杂项；「可处理七件＋同载五件」三轮）。
> 本报告＝**执行面交付**：七件摩擦修复（落码＋钉子）＋同载五件 S1 勘定＋摩擦台账。
> run＝`RUN-CLI-6ab6570c`（0.6.14 载体 dogfood 轮内实施）；基线＝orz @ `e22c0dba` 起跑工作区。
> 口径（用户令）：**不提交、不推送、不重建载体**——改动留两仓工作区，待下一轮 S3 重建收编；台账更新（索引/BACKLOG/TODO 的 0bs 条目状态）不在本件范围。

## 0. 交付摘要

| 件 | 处置 | 钉子/验证 |
|---|---|---|
| ① F8 结束自述通道告知面 | model_stop 常驻尾行（单一来源）＋模型面尾部挂载 | `end_channel_resident_line_rides_the_face_tail_after_the_block_table` ✓ |
| ② F6 输出编码链全谱 | dogfood_launch.ps1 编码钉（消费入口纪律）＋自检断言 | DryRun 复验（消费者侧设 UTF-8 后中文可读）✓ |
| ③ F3 lsp e2e flaky | 根因定位＋`drain_lsp_diagnostics` 循环等待修复 | lsp 子集 3×（19/0）；全量 2941/0/6 ✓ |
| ④ F5 分块表说明行重复 | 三通知去内嵌整表，单源指向窗口尾表 | `notices_point_to_the_tail_table_instead_of_embedding_it` ✓ |
| ⑤ F11 rustfmt 版本噪声 | 口径定稿（fmt 非交付门）＋toolchain 注释钉＋本文件域漂移清收 | `cargo fmt --check` 实核漂移清单 ✓ |
| ⑥ F12 压缩握手不可观测 | 未落地回执（两静默路径）＋审计键＋三态口径 | `summary_not_landed_receipt_is_informative_and_registered` ✓ |
| ⑦ F13 状态符号白名单窗口（用户令） | 26 段手选窗口＋整体放行（含 VS16）＋运行边界 | `status_symbol_window_passes_with_vs16_and_never_counts` 等 ✓ |

**测试总账**：orz-loop 全量 851/0/3；orz-tools 全量 2941/0/6（全量并行＝③判据）；emoji 子集 17/0；lsp 子集 3×19/0。
**同载五件（⑧-⑫）**：S1 勘定随本件 §4（按要求**不强行落码**）。
**载体推进**：S3 重建＋S4 真机复验随下一轮（⑦ 的实现轮亲历剥离为 S4 复验点，见 §2-⑦）。

## 1. 依据与范围

- 用户令链路：0bm 轮报告 §4（摩擦台账 F3/F5/F6/F8/F11/F12/F13）→ 用户裁决「可处理七件」＋「将 0bp 检索形态 B 和 0ax S3 部分加进 0bs」＋「检索部分的处理也并进 0bs」→ 立项登记＝BACKLOG 第一卷/第二卷 §1.16-§1.17、TODO `P1-0bs`（L1005-1015）。
- 本批边界：只做「可处理七件」的落码与验证＋同载五件的 S1 勘定；**不闭合 0bp／0ax 条目**（闭合随各自史，同轮接口先例、不设两套账）；启动器摩擦除②编码链外不动。
- 七件来源均＝0bm 轮实测摩擦（0.6.13 载体），处置面向 0.6.15+ 载体验证。

## 2. 七件逐件处置

### ① F8 结束自述通道告知面（`[RUN_END]` 零常驻告知）

- **判据**（TODO）：告知面收口（常驻或收束轮专用告知）＋钉。
- **落码**：
  - `orz-loop/src/model_stop.rs`：新增 `model_stop_resident_line()`（≈230 字常驻单行；`ORZ_MODEL_STOP_AWAIT` 仅切暂停广告、不动披露）。
  - `orz-loop/src/model_face.rs`：`build_model_face` 在**分块表之后**追加该尾行（每轮尾随；**无分块不注入**，保「无分块字节不变量」）；`estimate_model_face_tokens`／`model_face_message_count` 同步。
  - `orz-loop/src/agent_loop.rs`：5 处调用点同步（含第二截断点「无已闭合分块」占位参数删除）。
- **钉**：`end_channel_resident_line_rides_the_face_tail_after_the_block_table`（model_ 过滤 25/0）。
- **背景承接**：0bj⑥「实施核心」争议＝机制在位而模型面零常驻；本件把披露挂到每轮尾部（单一来源＝model_stop.rs），pull 面 guide 不再承担唯一告知。

### ② F6 输出编码链全谱（PS 5.1 管道/Tee 捕获乱码）

- **判据**：输出编码链收口（能力面自报或入口纪律）＋钉。
- **落码**（`D:\CLI\scripts\dogfood_launch.ps1`，消费入口纪律）：
  - 装配段起手＝`[Console]::OutputEncoding = [System.Text.Encoding]::UTF8`＋`$OutputEncoding = [System.Text.Encoding]::UTF8`（读原生输出所用编码＋管道发给原生命令所用编码，两件成套）；
  - 自检钉＝`Assert-True ([Console]::OutputEncoding.WebName -eq 'utf-8') ...`；
  - 文件头 ⑮ 注（动机与链位：载体侧 orz 已自钉 `SetConsoleOutputCP(65001)`，本处补**捕获链**）。
- **复验（本轮实做）**：`-DryRun` 装配全通过；消费者侧未设编码时捕获呈 GBK 乱码、设 UTF-8 后中文可读（「链条层级」实证）——「能力面自报」留后续 S1 观察项（本轮选入口纪律一侧）。
- **同族实读**（供给侧）：本会话 `git diff` 中文 GBK 乱码（m4）；`[emoji 剥离]` 告示行亦有同链现象。

### ③ F3 lsp e2e 并行 flaky（`e2e_restart_replay_requeues_pending_diagnostics`）

- **判据**：全量并行连跑零失败，或根因隔离（端口／临时目录／共享状态）＋钉。
- **复现**：`cargo test -p orz-tools --lib lsp` 连跑 3 次＝RUN1 过、**RUN2/RUN3 败**（`tests.rs:1364` panic「replayed document should still produce diagnostics」；`drain_lsp_diagnostics` 15s 预算内返回 None）。
- **根因（隔离到共享状态）**：`diagnostics_ready` 是**全客户机共用**的 Notify；`drain_lsp_diagnostics`（`manager.rs:477`）原为「**注册 waiter → 单次等待 → build 一次即返**」。restart 场景里被替换客户机的 1s 迟到诊断会先触发 **stray notify**，drain 被唤醒后 build=None **立即返回**——15s 预算根本没被用上（解释了 0bm 轮「加预算到 15s 仍 flaky」）。时序决定成败 ⇒ 偶发。客户机侧次序已核（`client.rs:84-92`：先落表后 `notify_one`，无需改）。
- **修复**：drain 改为**循环等待至摘要可建或 deadline**（stray notify 只触发一次快速重试、绝不再终止等待；超时路径保留「pending 保底＋debug 日志」语义）。生产调用者（`dispatch.rs:213`）同向受益。
- **验证**：lsp 子集 3×＝19/0/0 全过；**全量 `cargo test -p orz-tools --lib`＝2941 通过/0 失败/6 ignored**。**判据面**：全量并行 1 次零失败＋lsp 子集 3 连跑零失败（全量连跑在 S3/S4 复跑追加）。
- **钉**：以全量零失败＋本段根因记录代钉（原测试体不改；「预算注释」保留）。

### ④ F5 分块表说明行同源重复

- **判据**：单源一次（与 0bj② 的「指针可见性口径」不重叠——本件管「同文重复」）。
- **落码**：
  - `context_scale.rs`：H1 硬提醒／截断告知／必定窗口三处通知**去掉内嵌整表**，统一改引 `BLOCK_TABLE_POINTER_LINE`（新常量，`model_face.rs`）＝「分块表见**窗口尾部**（逐轮刷新；压缩/回放均按块号指定）。」；
  - `agent_loop.rs`：5 处调用点同步（含第二截断点占位参数删除）。
- **钉**：`notices_point_to_the_tail_table_instead_of_embedding_it`（断言三通知不含整表、必含指针行）。
- **背景**：0bm F5 实读＝说明行在表尾与通知两处重复且措辞旧（「可按块回放（指针见上）」而当时无指针列）；0bj② 修后表尾已是新措辞，本件把**第二来源消除**。

### ⑤ F11 rustfmt 版本噪声

- **判据**：工具链版本钉或口径落文档＋fmt 检查门不误伤。
- **勘定（本轮实核）**：`orz/rust-toolchain.toml` 已 pin `channel = "1.97.1"`（components 含 `rustfmt`／`clippy`）；本机 `rustfmt 1.9.0-stable` 与 toolchain 一致 ⇒ F11 的「版本差」实为**仓库档案与现行 rustfmt 输出的三处漂移**：`read_file/mod.rs:3531`（`panic!` 折行）、`grok_build_concise/search_replace.rs:204`、`emoji_strip_ranges.rs` 尾空行。
- **口径（定稿）**：**fmt 非交付门**——`cargo fmt` 仅整理**本批改动文件**；全仓 fmt 只在 rustfmt 版本变更时**一次性收编**；`cargo fmt --check` 仅作提示、不拒绝批次。
- **落码**：`rust-toolchain.toml` 注释（口径＋在案漂移清单＋bump 前置命令追加 `cargo fmt --check`）；**本批域漂移清收**＝`emoji_strip_ranges.rs` 尾空行已清（该文件本就属于本批改动），上列前两处属他文件、未随批动。

### ⑥ F12 压缩窗口摘要消费时机不可观测

- **判据**：窗口与摘要的握手时点对模型可见（回执／告知面时点收口）。
- **落码**：
  - `context_scale.rs` 新增 `summary_not_landed_notice(reason, compressible)`（与 0bk `block_selection_unrecognized_notice` 同形：引用原因＋当前可压区间＋重投指引＋声明行）；
  - `agent_loop.rs` `compress_blocks_now` 把两条**原静默**路径改为「回执推送＋审计落账（`context_scale:summary_not_landed`；reason=no_selection_hit／ledger_write_failed）」：①筛选后 `selected` 为空（区间无命中或无可压分块）；②外挂台账写失败。可压集合（号列表＋渲染）提前到消费点固定、供各分支复用。
- **三态口径**：**landed**＝压缩 marker（既有告知面）；**not_landed**＝本回执；**unrecognized**＝既有区间未识别告知——握手三态全部对模型可见。
- **钉**：`summary_not_landed_receipt_is_informative_and_registered`（含空集合渲染「（无）」）。
- **本会话实证**：两次窗口内压缩（1-2、3-6 及后续）均 `model_selected`／`model_summary` 即时消费，回执＝下一轮 marker＋分块表状态；未落地回执为本件新增面（S4 复验点）。

### ⑦ F13 状态符号白名单窗口（用户令定案）

- **判据**（用户令）：只放行状态符号（含 VS16 变体，整体放行、不拆序列、不拦截、不计数告知），其余照旧拦截；集合与判据由本项 S1 定稿（不另立计数）。
- **集合（S1 定稿）**：`STATUS_SYMBOL_WINDOW`＝**26 段手选白名单**——HOURGLASS／HOURGLASS WITH FLOWING SAND／PAUSE-STOP-RECORD／PLAY／WARNING（+VS16 随基座）／HIGH VOLTAGE／WHITE-BLACK CIRCLE／NO ENTRY／WHITE HEAVY CHECK MARK／CHECK-HEAVY CHECK／BALLOT X-HEAVY BALLOT X／CROSS-NEGATIVE SQUARED CROSS／QUESTION-EXCLAMATION ORNAMENTS／HEAVY EXCLAMATION／STAR／CHEQUERED FLAG／PUSHPIN-ROUND PUSHPIN／REPEAT-REPEAT ONE／ANTICLOCKWISE ARROWS／LOCKED WITH KEY／LOCK-OPEN LOCK／RED-BLUE CIRCLE／CONSTRUCTION／TRIANGULAR FLAG／NO ENTRY SIGN／ORANGE-YELLOW-GREEN-PURPLE CIRCLE。**判据**＝手选白名单（非自动推导）＋经验面（`docs/audits` 全量扫描：U+2705 凡 178 次／U+26D4 4／U+2757 1／U+2714 19／U+274C 6／U+23F3 等）＋与 strip 表交集按**白名单优先**裁决。**不入窗**（留 S1 复议）：箭头族（U+2190 区间）、龟兔等拟物记号。
- **落码**：
  - `emoji_strip_ranges.rs`：新表＋维护纪律注释；
  - `emoji_strip.rs`：`is_status_symbol()`（二分）＋主循环**整体放行**（基座后随 FE0E/FE0F 变体一并放行，不拆序列）＋**运行边界**（白名单符不被并入 emoji 运行、不计数）＋模块文档条目。
- **钉**：`status_symbol_window_passes_with_vs16_and_never_counts`（整体放行＋混排计数只算 emoji）、`status_symbol_window_ranges_sorted_and_disjoint`（排序不重叠）；emoji 过滤集 **17/0**（含全部既有回归：ZWJ 整段清除、VS16/肤色/键帽、空白折叠、告知行计数、search_replace/hashline 写面测试）。
- **实现轮亲历（F13 正例，S4 复验点）**：本修复的实现文本含**字面状态符号**，在 0.6.14 旧剥离面上写盘时被剥离（`[emoji 剥离]` 31 处／14 处两次告示）⇒ 夹具改用 `\u{}` 转义承载、注释改 ASCII 名称。**S4 复验**：同内容进新载体应不再出现这些剥离计数。

## 3. 验证与实测（本轮读数）

- **测试矩阵**：orz-loop 全量 851/0/3；orz-tools 全量 2941/0/6；lsp 子集 3×19/0；emoji 子集 17/0；model_ 25/0；context_scale 11/0（含 ④⑥ 新钉）；① 新钉单测 1/0。
- **launcher DryRun**：装配与断言全通过（含新自检钉）；消费者侧设 UTF-8 后输出中文可读。
- **fmt 实核**：`cargo fmt --check -p orz-tools` 报三处漂移（read_file/mod.rs:3531、grok_build_concise/search_replace.rs:204、emoji_strip_ranges.rs:127＝尾空行）；本批域（emoji 文件）已清。
- **会话观测（0.6.14 载体实读，报告素材）**：H1 硬窗口两开（322,446tk／322,628tk）；三次分块压缩全部 `model_selected`／`model_summary` 即时消费（1-2、3-6、7-11）；**窗口参与落账另有 `model_not_participated` 一次**（首次 H1 窗口内未产出摘要；第二次窗口内产出并消费——⑥ 面动力的本会话实证）；`[回退窗口]` 每次编辑落快照、全程正常；资源软提示 1 次；`[emoji 剥离]` 告示两次（31 处／14 处——⑦ 实现轮亲历两次，见 §2-⑦）。
- **工作区状态**（收口实查）：orz 侧 8 文件修改（+415/−87）：`model_stop.rs`／`model_face.rs`／`context_scale.rs`／`agent_loop.rs`／`emoji_strip.rs`／`emoji_strip_ranges.rs`／`lsp/manager.rs`／`rust-toolchain.toml`；父仓 `scripts/dogfood_launch.ps1`＋本报告（另 `0BM_...md` 为前批未提交增量、未动；`orz` 子模块指针 dirty）。**未提交、未推送、未重建**（用户令）。

## 4. 同载五件 S1 勘定（⑧-⑫；本轮不落码）

> 依据：TODO `P1-0bs`（L1005-1015）＋[`0BS 检索线路线调研`](0BS_RETRIEVAL_ROUTE_SURVEY_2026-09-25.md)（含第二/三/四令修正）＋代码实勘（本轮 grep/orz 树）。

### ⑧ 0bp 检索形态 B 执行面（四子件）

- **四子件**：①共 per-activation SERP 预算；②降级序 browser→本地分段 HTTP→provider；③时限「首个结果 ≤10 s」；④S1 设计定稿（含 ADR-0010 §14.65 邻域转录裁决）。批序 S1→S2 落码＋钉子→S3 载体重建→S4 真机。
- **现状（实勘）**：`ORZ_WEB_SEARCH_LOCAL` 仍为**整条开关语义**（`orz-tools/src/implementations/web_search/local_segmented.rs:20`（×2）、`orz-tools/src/registry/types.rs:1072`、`orz-loop/src/retrieval/projection.rs:118`）；本地分段硬化 G1–G4 已落码（0ac 批，`7e151ed1` 起）；browser_control search 共账未见落码；设计档＝`docs/RETRIEVAL_LOCAL_SEGMENTED_HARDENING_DESIGN_2026-09-15.md`；ADR 面＝`orz/adr/ADR-0010-fusion-runtime-and-agent-architecture.md`（§14.65 脚注待转录裁决）。
- **S1 结论**：子件①-③为**执行面收口**（把整条开关改造成按次预算＋降级序＋首果时限三键）；子件④＝设计转录（是否并入 ADR §14.65 邻域）。**建议落码批＝0bs 后继 S2a**（四子件小步），S3/S4 随载体重建。

### ⑨ 0ax S3 落码面（fetch 侧兜底）

- **范围**：fetch 侧指纹兜底（`0x676e67/netty` 等——**更正项**：原 `rquest` 措辞须同步更正至 0x676e67/netty，见调研 §5）或 reader 服务；＋反爬站点复测；S4 真机仍留官方跑批面。
- **现状（实勘）**：`wreq` 零命中；`rquest` 仅余一处记事（`orz-workspace/src/permission/auto_mode.rs:352`）；兜底引擎**未落码**。反爬复测素材＝调研 §2 直连实测（百度/搜狗/夸克/Bing 四态读数）。
- **S1 结论**：与 ⑩ 共传输层（TLS/HTTP2 指纹伪装）——**建议并入 ⑩ 的 S2 批次**（同一伪装底座），避免两处实现；reader 服务形态留设计期二择一。

### ⑩ HTTP 检索路线（三件，兜底定位）

- **令链**：兜底定位（「http 本身终究还是兜底」）→ 第二令（四引擎小集＋只做伪装、不引 SearXNG runtime）→ **第三令（现口径）**：「bing 也不留了」⇒ **HTTP 集＝三件 `duckduckgo`／`baidu`／`360search`；Bing 家族整体移除**（依据＝双态皆降级）；DDG 直连不可达 ⇒ 有代理时的质量源。清洗件自写**否**（SearXNG `360search.py` 114 行等仅作**移植参考、留来源标注**）；结果质量判断交**检索子代理**（机械层取回＋如实回传、不设强相关性门）。
- **现状（实勘）**：`local_segmented.rs` 现有 DDG 面（duckduckgo ×11）；`360search` 零命中（待移植）；夸克 `tmd/punish` 风险、Bing 降级读数在调研 §2。
- **S1 结论**：落点＝`web_search/local_segmented.rs` 邻域扩三引擎；传输层与 ⑨ 共用。**注意（勘定发现）**：TODO `P1-0bs` ⑩ 措辞仍是第二令「四引擎小集」——与调研 §4.5 第三令「三件、Bing 出集」存在**措辞漂移**，待台账轮同步（本案记录、本批不动台账）。

### ⑪ 本地浏览器车道（主战场：真机 CDP＋输入拟真 v1）

- **令链**：真机接入＋反检测底座 → 第二令（真机＝无沙箱、指纹天然为真、**不需 camoufox 类底座**）→ 第三令（**CDP 直连真机、不复制 profile**；占用判定交模型；动作面放开；**输入拟真常驻、机械层自动施加**）→ **第四令结清四项**：①动作全放开、**唯二门禁＝下载与脚本执行**（弹验证＋用户明确批准）；②无头／真机**不混用**（单次子代理进程只用一类）＋**每次记录所用浏览器类型留档**；③占用判定**不另设机械事实面**（控制面已开放）；④输入拟真＝**统一固定标准、单一源**。
- **参数表 v1（已定稿，单一源）**（调研 §4.6 登记）：逐字符键入 200 ms±40%（截断 120–400）；句间停顿 1.5 s（0.8–2.5）；提交前 0.6 s；错误率 ~2%、回退 1–3 击；鼠标轨迹 WindMouse 类（峰值 800 px/s）；点击 Fitts 律下限；滚动 110 px/格、格间 20 ms、段间 0.8 s；页面停留 ≥0.8 s；会话级沿用既有 pacing ≥5 s＋0–50% 抖动。数据来源已备（Aalto 136M／CMU／KeyRecs／Balabit／Fitts；**不回算**、取论文区间自行取值）。
- **现状（实勘）**：`orz-host/src/local_browser/`＝`cdp.rs`（≈2.8k 行）＋`serp.rs`（含既有 pacing＋jitter 前身）＋`discovery.rs`；`browser_launch` 事实族已在 assurance（`journal/families.rs` 等）与 `orz-bin/main.rs`／`orz-host` 多处接线；`orz-loop/src/host_exec/serp.rs` 有 DDG 直连痕迹。
- **S1 结论**：底座已在，S2 面＝①输入拟真机械层常驻（读单一源参数表）②动作白名单放开（唯二门禁＝下载/脚本，走审批）③无头/真机不混用＋浏览器类型留档事件④占用判定事实形状（控制栏读数）收口。批序建议＝⑪ 先行（主战场）→ ⑩/⑨ → ⑧ → ⑫。

### ⑫ 检索源补充

- **范围**：直连友好高质量垂直源（GitHub API／Stack Exchange／arXiv／OpenAlex／Crossref／PyPI／npm）纳入检索源面（技术类质量高于百度、零反爬）。
- **现状（实勘）**：树内仅 arXiv 痕迹（`orz-host/src/pdf_evidence.rs`／`tools.rs`／`orz-loop/src/relay.rs`）；其余零命中。与 ⑩ 的「引擎面」相区＝**API 型垂直源**（无 SERP 形态），接入形态建议＝保留原格式＋来源标注，权重面复用既有检索源加权设计。
- **S1 结论**：源集与接入形态随 ⑩ S2 之后的独立小批；先把「源注册＋最小取回」做出来再谈加权。

**五件共性**：均为「已定案、待落码」的承接面（裁决与读数齐备）；本批只勘定不落码（用户令「勘定并记录」＋不强行落码的批内口径）。

## 5. 摩擦台账（本会话 m1-m15）

| # | 摩擦 | 形态／读数 | 对策／备注 |
|---|---|---|---|
| m1 | 超长单行文件读取 | `read_file` 对超长单行返回截断（CLI_PROJECT_INDEX/TODO 均中招） | 本会话拐棍 `.tmp-lineview.py`（按行窗切片）；建议工具面「行窗模式」 |
| m2 | shell 引号层 `$` 展开被吞 | 跨 PowerShell 层级时 `$VAR` 在到达内层前被展开 | 避免 `$`（改 Python）或单引号包裹；命令首行设编码 |
| m3 | `cd /d` 在 PS 5.1 不适用 | CMD 语法在 PS 下报错 | 用 `cd D:\...` 或 `-LiteralPath` |
| m4 | `git diff` 输出 GBK 乱码 | 中文 diff 呈乱码（② 同链、供给侧实证） | 消费侧设 `[Console]::OutputEncoding`；② 修复同向 |
| m5 | 超长单行 JSON 分析 | `.gsa/conversations/*.json` 巨型单行、直接读取不可行 | Python 切片扫描 |
| m6 | 320K 硬窗口实读 | H1 两次实读 322,446／322,628tk（含「窗口尾部」新措辞） | 设计内行为；本会话三次分块压缩正常消费 |
| m7 | 回退窗口 | 每次编辑落 `.gsa/rollback/<hash8>/` 快照、附回退指引 | 全程正常（本会话含坏文一次：⑦ 剥离修复场景，快照可回） |
| m8 | ⑦ 实现轮亲历写面剥离 | 实现文本的字面状态符号被 0.6.14 旧面剥离（31／14 处告示） | 夹具改 `\u{}` 转义；S4 复验点（本修复的必要性正例） |
| m9 | 终端无 `grep`／`head`／`sed` | 宿主 shell 禁用常规 Unix 工具 | 工具层提供 grep 工具＋Python 替代（命令侧须改写） |
| m10 | 摘要投递时机体感 | 三次分块压缩（1-2、3-6、7-11）均 `model_selected`／`model_summary` 即时消费；**首次 H1 窗口 `model_not_participated` 一次**（未在窗内及时产出）；7-11 于第二次窗口内产出 | ⑥ 面动力记录；与 0bm F12 同族应力（本会话实证） |
| m11 | 摘要握手「同轮不可见」残余 | 投递当轮看不到结果、回执在下一轮（marker／表态） | ⑥ 已把「未落地」变可见；同轮即时性仍不可得——低优先观察项 |
| m12 | CRLF 变动警告 | `manager.rs` 编辑后 `git` 提示 CRLF→LF 转换 | 无实害、如实登记（S3 进件时留意） |
| m13 | 会话档分析摩擦 | 对话档（`.gsa/conversations`）结构需自写切片器 | 与 m5 同族；已在 0bm 报告有先例 |
| m14 | 终端消费侧默认 ANSI | 捕获子进程输出乱码（本会话 DryRun 首读即中） | ② 修复的反身实证；调用首行设 UTF-8 |
| m15 | 启动器 DryRun 显示 carrier＝unknown | 版本标注缺位（非阻断） | 观察项；S3 后回看 |

（0bm 轮既有摩擦 F3/F5/F6/F8/F11/F12/F13 的处置见 §2；本表为本会话新增读数与残余。）

## 6. 未竟与移交

- **S3 载体重建**（下一轮）：本批 8＋1 文件进件（`git` CRLF 提示留意）；台账（索引/BACKLOG/TODO 的 0bs 状态）随台账轮更新（含 **TODO ⑩ 措辞对齐第三令**）。
- **S4 真机复验点清单**：① 结束自述常驻尾行实读（尾部、每轮）；② 日志中文编码实读（launcher 链）；③ lsp 全量在重建后复跑；④ ⑦ 剥离计数不复发（同内容进新载体）；⑤ ⑥ 未落地回执实读（可构造：给不存在的块号区间）。
- **⑧-⑫ S2 落码批**（建议序）：⑪ 真机浏览器车道（输入拟真常驻＋动作放开＋留档）→ ⑩/⑨ 三件引擎＋传输层 → ⑧ 四子件收口 → ⑫ 垂直源。⑧ 的 S1 设计转录（ADR-0010 §14.65 邻域）先裁后码。
- **低优先观察**：⑥ 的 run 尾消费点不升级（0bn 已裁）；同轮即时回执（m11）；⑤ 全仓 fmt 一次性收编时机。
- **环境面**：调研 §5 已登记 `curl_cffi==0.16.3` 安装（非本批动作、如实引用；卸载可复原）。

## 7. 附录

### A. 改动清单（本批，未提交）

**orz（8 文件，+415/−87）**：`crates/orz-loop/src/{model_stop.rs, model_face.rs, context_scale.rs, agent_loop.rs}`；`crates/codegen/orz-tools/src/util/{emoji_strip.rs, emoji_strip_ranges.rs}`；`crates/codegen/orz-tools/src/implementations/lsp/manager.rs`；`rust-toolchain.toml`。
**D:\CLI（父仓）**：`scripts/dogfood_launch.ps1`；本报告（新增）。
**关键锚点（本会话末次读取/编辑前）**：`model_stop.rs`(9d40ffab)／`model_face.rs`(b218574c)／`context_scale.rs`(08840b33)／`agent_loop.rs`(412e4225)／`emoji_strip.rs`(62a8e088)／`emoji_strip_ranges.rs`(6e032a1b)／`manager.rs`(23fadff0)／`dogfood_launch.ps1`(b25860d1)。

### B. 验证命令（存档形态）

- `cargo test -p orz-loop --lib`（851/0/3）；`cargo test -p orz-tools --lib`（2941/0/6）；`cargo test -p orz-tools --lib lsp`（3×19/0）；`cargo test -p orz-tools --lib emoji`（17/0）；`cargo fmt --check -p orz-tools`（三处在案漂移）；`dogfood_launch.ps1 -DryRun`（装配全通过）。

### C. 本会话拐棍（未入库）

`.tmp-lineview.py`（超长单行文件行窗切片器）；分析脚本若干（会话档/事件档扫描，一次性）。

---

*报告落稿：2026-09-25；run=RUN-CLI-6ab6570c；载体 0.6.14（0bm／0bs 轮实施面）。*

