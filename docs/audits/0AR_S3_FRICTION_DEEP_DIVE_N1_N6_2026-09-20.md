# 0ar S3 摩擦项深挖（N1–N6，2026-09-20）

> 用户令：「0ar 可闭合；请深挖一轮 N1–N6 摩擦项吧」。
> 本文为**只读取证**（journal／retrieval-results 归档件／orz 源码）+ 一次静态分类器复算，
> **零代码改动、零子仓改动、不动计数**；N1–N6 的立项与处置留用户裁决。
> 取证面：`official-verify-timeout3-s3` 三 run 全部工件（含 `.quarantine` 侧）、r1–r3 九 run 对照、
> `orz/crates/orz-loop/src/retrieval/{batch_close,evidence}.rs`、`orz/crates/orz-host/src/{resource_gate,acp_server}.rs`。

## §0 摘要

| # | 现象（S3 实测） | 根因（代码级） | 量级 | 处置候选（待裁决） |
|---|---|---|---|---|
| **N1** | 合并批逐 query 可用计数系统性小于批级（4 例欠归因 3–6 条，占批级 60–100 %） | `per_query_usable_counts` 按**逐字 query 串**匹配 `EvidenceRecord::search_query`；子代理自查串与派发 query 不同、派生证据（web_fetch/read_file）无 query ⇒ 不归因；归因依据也不进 `source_ledger` | 多 query 批 4/4 失真（0／1／2 条），单 query 批 3/3 正确 | ①谱系字段 `origin_query_id` ②`unattributed_usable_count` 显式缺口 ③口径改文 |
| **N2** | run 尾部仍可派发检索，被 run 墙钟直接截断 | 派发前只判「合并上限 3」，**无 run 级墙钟余量判定**（`agent_loop.rs` 预扫描无 wallclock 项） | 4 轮共 7 次「余量 < 300 s」派发；S3 两次（torch 117 s／extract **26 s**），5 次 trailing 无 close | A2：派发前 `remaining < batch_wallclock + margin` ⇒ 结构化未派发＋尾部保留 |
| **N3** | 倒数行／β 注入块在**任何落盘面**零命中 | 倒数行只入模型面 `messages`；headless 路径唯一载体是会话侧车，而 `headless_session_archive` 在未达 **500 K** 归档里程碑时**直接 return**（代码注释＋ADR-0010 §14.68 明示设计边界） | S3 三 run 皆未达 500 K（torch 触发压缩时 212 K）⇒ 无侧车、无归档、无倒数行 | ①机械审查面落一行 `retrieval_batch` 读数 ②run 尾无条件落侧车（或评测开关） |
| **N4** | 资源门 deny 常态化且量级上升 | `HEAVY_RELEASE_COMMIT_HEADROOM_PERCENT = 25`；heavy＝按**程序名**（tar/gzip/uv/gcc/pip/apt…）或 heavy 工具判定；本机 run 起始 commit 已 86–87 % used（余量 12–14 %）⇒ **25 % 门结构性不可满足** | heavy 类 shell 命令 **28/29 被拒**；gpt2 **11/11 次 gcc 全部被拒、0 次成功**；2 条为引号盲切分伪触发 | ①阈值按环境可满足性重定（绝对余量/分级）②heavy 判定参数感知（`gcc --version`、`tar -t`、`gzip -t` 非重活）③引号感知分段 |
| **N5** | extract-elf 相对 r3 由 1.0 变 0（未落判分物） | r3 的 1.0 系**公网泄漏页**（含 `test_outputs.py`＋内嵌 REF）；S3 搜索链返回**无 URL 合成文本**（9/9 条无 URL、子代理自判"不可引用"），grep.app 被 Vercel 拦截 ⇒ 无 fetch 目标 ⇒ 无泄漏路径；且**宽口径把无 URL 合成片段计入可用证据**，两批由合成片段凑满 5 条达标收尾 | act 01 的 5 条可用**全部**是无 URL 合成文本；web_fetch 仅 2 次（r3 为 7 次） | ①可用口径增"可引用性"维度 ②合成答案单列计数（不改 FP-2） |
| **N6** | 浏览器车道恒死（三轮至 S3 全败） | 子代理系统提示「本地浏览器检索｜推荐首选」⇒ 每批首回合先试死车道；探针 `retrieval_family.browser.present=true` 只判"工具在位"不判"可执行件在位" | S3 8/8 失败（`browser_not_found`→`capability_unreachable`，8–11 ms）；7 批中 4 批首回合即含 browser_control | 探针补"可执行件在位"位（`ORZ_BROWSER_PATH` unset 且 PATH 无浏览器 ⇒ present=false）；不启用 `eval_browser`（既定裁决） |

**跨项共性**（§8）：①「可用」口径缺**可引用／可执行性**维度（N1／N5／N6 同源）；
②**环境参数 vs 本机资源基线**不匹配（N4 结构性、N2 时序性）；③**观测面缺口**（N3 与 N1 的二次事实）。

## §1 取证口径

- journal：`D:\tb-eval\gsa-volumes\.quarantine\official-verify-timeout3-s3\{582d0289,7f781ed9}\runs\` ＋
  末位试次 `D:\tb-eval\gsa-volumes\official-verify-timeout3-s3\51180b0a-…\runs\RUN-CLI-6aaeba24\`。
- 结构化结果归档：同 run 的 `retrieval-results/*.json`（7 件：torch 2、gpt2 2、extract 3）。
- 代码：`retrieval/batch_close.rs`（计数／倒数）、`retrieval/evidence.rs`（ledger／query_summary 装配）、
  `agent_loop.rs`（派发预扫描／倒数注入点）、`host/resource_gate.rs`（heavy 分类／25 % 门）、
  `host/acp_server.rs`（侧车与归档门槛）。
- 静态分类器复算：`D:\tb-eval\_heavy_probe.py`（按 `split_segments`／`segment_is_heavy` 口径重跑被拒命令，
  并标注触发 token 是否位于引号内）。

## §2 N1 逐 query 可用计数欠归因

### 2.1 实测

| 激活 | query 数 | 逐 query 可用 | 合计 | 批级可用 | 未归因 | 未归因构成 |
|---|---:|---|---:|---:|---:|---|
| torch-00 | 2 | 1／0 | 1 | 5 | **4** | web_search_result 3＋web_page 1 |
| torch-01 | 2 | 0／0 | 0 | 6 | **6** | local_file 3＋web_search_result 2＋web_page 1 |
| gpt2-00 | 2 | 1／1 | 2 | 5 | **3** | web_search_result 3 |
| extract-02 | 2 | 0／0 | 0 | 6 | **6** | local_file 5＋web_search_result 1 |
| gpt2-01／extract-00／extract-01 | 1 | 5 | 5 | 5 | 0 | —（单 query 全额归属，设计如此） |

### 2.2 根因（两层）

1. **匹配键是逐字 query 串**：`batch_close::per_query_usable_counts` 对多 query 批逐条
   `filter(|ev| ev.search_query.as_deref() == Some(q))`。派发时写入 `query_summary` 的 query 是
   **主代理声明的 goal 串**；子代理随后自查的串是它**改写过的**（torch-01 声明串
   `"train_step_pipeline_afab" OR "afab" pipeline …` vs 实际搜索
   `DeepSpeed _scale_loss_by_gas …`）⇒ 逐字匹配落空。
2. **派生证据无 query 归属**：`web_fetch`／`read_file`／`browser_read` 产出的 `EvidenceRecord`
   `search_query = None`（代码注释已登记该边界），在多 query 批里一律落到"未归因"。

**二次事实（可审计缺口）**：`source_ledger` 条目**不含** query 归属字段（键集实测无 `search_query`／`query_id`），
即归档结果里只留算出来的 `usable_source_count` 数字，**无法事后复核**这个数字是怎么归因的。

### 2.3 影响

设计稿 v1.1 §5.4 把批级阈值的**覆盖保障**明确寄托在「逐 query 披露 ＋ 主代理据此续派」。
实测该披露在多 query 批里**4/4 失真**（最大欠归因 6 条、最低 0 条），主代理若按它续派会得到错误结论
（"这批没覆盖 q2" 实为"q2 的证据挂在子代理自查串下"）。⇒ **该保障机制当前不成立**。

### 2.4 处置候选（不立项，待裁决）

1. **谱系字段（根治）**：`EvidenceRecord`／ledger 落 `origin_query_id`——子代理自查一律归其所属派发 query
   （leader 谱系），派生证据归"发起它的那次检索"。契约面＝`retrieval-result-event-payload-v0.2` 的
   `source_entry` 增可选字段（按 S1 先例"新增字段先立契约变更"），并同步 Python 冻结参照。
2. **显式缺口（最小改）**：`query_summary` 增 `unattributed_usable_count`（＝批级－逐 query 合计），
   让主代理至少看到"有多少条没归到任何 query"，不再误读为"未覆盖"。
3. **口径改文**：把"逐 query 覆盖可核"降级为"批级可用＋未归因数"，与设计稿 §5.4 同步。

## §3 N2 run 尾部派发

### 3.1 四轮横向（派发时剩余 run 墙钟；leader 为准）

「余量」＝**派发时刻到 run 终点**的秒数（墙钟到点的 run 即剩余预算；自然收工的 run 为到收工点的距离，已在下表标注）。

**摩擦口径**：派发时不足以容纳一个完整 `batch_wallclock`（extended 300 s），且该批**被 run 墙钟截断（trailing、无 close）**；
「余量」＝派发时刻到 run 终点的秒数。

| 轮次 | 题（run 结局） | 派发余量 | 摩擦例 |
|---|---|---|---|
| r1 | torch（wallclock）829／573／**183**；gpt2（wallclock）751／**153**；extract（传输早死 350 s）280 | 183／153 | **2**（r1 torch、r1 gpt2，均 trailing） |
| r2 | extract（wallclock）828／**191**；gpt2（wallclock）699／**123**；torch（wallclock）**39** | 191／123／39 | **3**（全部 trailing） |
| r3 | torch（wallclock）668／462；extract（自然收工 408 s）369 | 462 | 0（无 trailing；462 批正常收口） |
| **S3** | torch（自然收工 816 s）780／**117**；gpt2（wallclock）642／400；extract（wallclock）848／466／224／**26** | 117／26 | **1**（extract 26 s trailing；torch 117 侥幸在 78 s 内收口＝近失） |

- **trailing 无 close（被 run 墙钟截断）**共 5 例：r1 torch 183、r2 torch 39、r2 gpt2 123、r2 extract 191、
  **S3 extract 26**（该 run 末态含 `tool_running` 1 条、无 close record）——**S3 不是最差轮，但含最极端的一例（26 s）**。
- S3 torch 的 117 s 派发侥幸在 78 s 内收口（余量 39 s 时 close）——**低于 300 s 门仍成功属运气**（近失例）。

### 3.2 根因

`agent_loop.rs` 的派发前预扫描只做「同轮合并上限 3」判定（`MERGE_MAX_QUERIES`），
**没有任何 run 级墙钟余量项**（该文件内 wallclock 仅出现在收尾臂与失败文本）。档位表只决定
**批**墙钟（180/300/450），不看**run**还剩多少。

### 3.3 影响与候选

尾部派发大概率白烧一个主回合＋一批半成品（S3 extract 的 26 s 派发产出为零）。
候选（A2，分析件 §5.1 原案）：

1. 派发前判定 `remaining_run_wallclock < batch_wallclock + close_round_margin`
   ⇒ 复用 D3 的"未派发"拒绝模板（`ToolStarted` 不作，cause 另立，如
   `retrieval_dispatch_wallclock_reserved`），文案只报事实；
2. **尾部保留**：距 run 墙钟 < 保留额（建议 90–120 s）时不派发新批，把尾部留给落盘；
3. 参数与 D2 档位表绑定（standard 180／extended 300／deep 450），避免两套尺。

## §4 N3 倒数行／β 注入块无落盘面

### 4.1 现象

三 run 全部产物（journal、`retrieval-results`、harness trajectory、`agent/orz.txt`、卷内 `session/`）
全库扫描 `本批可用证据` **零命中**；`gsa-volumes` 全域亦无 `conversations/` 与 `archives/` 目录。

### 4.2 根因链（代码级，含既登记边界）

1. 倒数行由 `append_countdown_to_tool_message(messages, call_id, …)` 追加进**模型面消息链**
   （`agent_loop.rs` 两处执行点），**不进 journal**（`ToolCompleted` payload 无该字段——S2 已按设计登记）；
2. 模型面消息的唯一落盘载体是**会话侧车** `<cwd>/.gsa/conversations/<session8>.json`；
3. harness 适配器走 `orz -p … --real`（**headless**）→ `headless_session_archive()` 在
   `!incremental_archive_due()` 时**直接 return**：门槛为 `ARCHIVE_INCREMENT_TOKENS`（**500 K** token 里程碑），
   且头文件注释明示「未到期不留 `.gsa/conversations/` 产物——与 ACP 车道『每个成功 prompt 都落侧车』的差异
   属设计边界，ADR-0010 §14.68」；
4. S3 三 run 皆未达 500 K（torch 触发压缩时 212 K、消息 111 条）⇒ 无侧车、无 `archives/`、无倒数行。

### 4.3 影响

判据 7 后段（倒数行与 `source_counts` 同口径一致）在**任何未达 500 K 的 run 都不可核**——
这不是"本轮没看"，而是**结构性无面**；F10（投递族事件在官方口径下不可测）同家族。

### 4.4 处置候选

1. **机械面落一行（推荐）**：在既有 `mechanical_audit_update` 家族增 `kind="retrieval_batch"`，
   payload 带 `activation_id／usable／cap／retrieval_calls／terminal_reason`——journal 可核、模型面零改动、
   不触模型可见面（符合"不加说明性内容"的既有裁决）；契约面按需在 mechanical audit payload 文档登记。
2. **侧车门槛（备选）**：run 尾无条件落侧车（或加 `ORZ_SESSION_SIDECAR_ALWAYS=1` 供评测轮启用），
   让整条模型面消息（含倒数行、β 注入块）成为可审计面；代价是每 run 一份对话 JSON。

## §5 N4 资源门 deny 常态化与量级上升

### 5.1 参数与环境基线

| 项 | 值（代码常量） | 本机实测 |
|---|---|---|
| heavy 放行门 | `HEAVY_RELEASE_COMMIT_HEADROOM_PERCENT = 25`（commit 余量占限额百分比） | run 起始 commit 余量 **12.0–13.8 %**（0.71–0.83 GiB of 5.84 GiB，86–87 % used） |
| tier 梯 | watch 70 %／soft 85 %／hard 95 % used | 全程 **soft**；torch 16:10:38 瞬时 `hard`（used＝limit，free＝0）后回落 |
| heavy 判定 | `HEAVY_TOOLS`（run_tests）＋`HEAVY_PROGRAMS`（109 个程序名）＋`CONDITIONAL_PROGRAMS`（python/node/… 仅 install/build 类子命令）＋heavy 写目标 | — |

**结构性结论**：25 % 门要求 commit 余量 ≥1.46 GiB，而本机 run 起始余量仅 0.71–0.83 GiB
⇒ **门不可满足**。torch 的**第一条 deny 出现在 run 开始 +11 s**（`ls /usr/bin | grep -i -E "python|pip|conda"`），
即模型尚未做任何重活就已撞门——否认"是模型把资源用光"的假设。

### 5.2 拒绝面实测（静态分类器复算）

| 题 | `run_terminal_cmd` 总数 | heavy 类 | 被拒 | 执行成功 | 触发程序（摘要） |
|---|---:|---:|---:|---:|---|
| torch | 35 | 14 | **13** | 1 | tar／gzip／uv／apt-get（含 2 条引号伪触发：`"python\|pip\|conda"`、`nohup … uv …`） |
| gpt2 | 28 | 11 | **11** | 0 | **gcc（11/11 全拒，含 2 条 `gcc --version`）** |
| extract | 35 | 4 | **4** | 0 | apt／npm／pip／gcc（`gcc -o /tmp/test_hi hi.c`） |
| 合计 | 98 | 29 | **28（96.6 %）** | 1 | — |

- **参数不感知**：`tar --version`／`tar -tzf`（列表）／`gzip -t`（完整性测试）／`gcc --version` 与真正的
  重活同判为 heavy ⇒ 纯只读校验也被拒。
- **引号盲切分**：`grep -i -E "python|pip|conda"` 按 `|` 切开后首词成 `pip` ⇒ 只读 grep 被判 heavy（torch 1 条、
  extract 1 条，脚本已复算）。
- **deny 轮次占比**：主车道工具轮 torch 14/63＝22 %、gpt2 11/41＝27 %、extract 4/26＝15 %。

### 5.3 影响

gpt2 的核心策略是"编译 C 探针验证 ckpt 解析/前向"——**11 次 gcc 全部被拒、0 次成功**，
该路线被整体封死；模型只能改用编辑器工具（12 次 `search_replace`）但仍未落 `/app/gpt2.c`。
即：**该项摩擦与"判分物未落盘"的结局存在直接因果通路**，不只是噪声。

### 5.4 处置候选

1. **阈值可满足性重定**：门改「`headroom ≥ min(25 %, 绝对余量下限)`」或按 tier 分级放行
   （soft 降门／watch 全放），并在 deny 信封照实报"本机基线余量"；
2. **heavy 判定参数感知**：`--version`／`-t`／`-l` 等只读形态降级为 light；
3. **引号感知分段**（`split_segments` 尊重引号状态），消掉伪触发；
4. deny 信封只报事实（现状已如此），但可补一行"未启动"证据（现文已有 `Nothing was started.`）——
   已足够，无需教学式建议。

## §6 N5 extract-elf 回归（r3 1.0 → S3 0）

### 6.1 证据链

| 面 | r3（0.6.2） | S3（0.6.3） |
|---|---|---|
| web_search | 5 次全成功 | 16 次发起／14 完成 |
| web_fetch | 7 次全成功 | **2 次**（grep.app API 被 `Vercel Security Checkpoint` 拦；duckduckgo html 无正文） |
| 搜索结果形态 | 4 条无 URL 合成 + 12 条 web_page（含**泄漏页**） | **9 条 web_search_result 全部无 URL**（子代理自判"不可引用"） |
| 判分物 | `/app/extract.js` 复刻泄漏页 REF ⇒ reward 1.0 | 未落盘 ⇒ reward 0 |
| 末段行为 | 复刻→自检→submit | 第 3 批收尾后 17 s 读黑板、跑 1 次终端，随即**开第 4 批**（余量 26 s）→ 被 run 墙钟截断 |

### 6.2 根因（两层）

1. **r3 的通过不可复现**：其判分物来自公网单页（含本题 `test_outputs.py`＋内嵌 REF 实现，r3 §10.3 已登记为
   F2 评测泄漏）。S3 同一路径不存在——搜索链返回**无 URL 的合成文本**，没有可 fetch 的目标；
   唯一两条 fetch 均被反爬拦。⇒ 该题在 S3 的"潜在解法"退化回自主解题。
2. **宽口径把合成片段当可用证据**：两批达标收尾的 5 条可用里，act 01 的 5/5、act 00 的 3/5 都是无 URL 合成文本。
   阈值 5 因此被"不可引用的文本"凑满，子代理按规则收尾（它自己在总结里写明"不可引用"）——
   **N1 的"可用口径"与 N5 的结局在此合流**：口径缺"可引用性"维度。

### 6.3 附带的其它摩擦（同 run 观测）

- 检索车道`run_terminal_cmd`被 `retrieval_role_write_denied` 拒（SRC-020／SRC-035 记录）——
  子代理读不到终端车道产物（OBS-GSA-READ-LANE-ASYMMETRY 家族）；
- 读 `/app/.gsa/reference.json` 被 `session_volume_agent_invisible` 拒（0m 设计内）。

### 6.4 处置候选

1. 可用口径增**可引用性**维度（无 URL／无正文的合成答案不计入 5 条阈值，或单列 `synthetic_answer` 计数）；
2. 不改 FP-2（检索失败不抑制、模型自主）与 §9.2 裁决（模型习惯不干预）——本条只动**口径**，不动行为门；
3. 泄漏面维持 r3 F2 登记（判分物不入公网可检索面的装置侧议题）。

## §7 N6 浏览器车道恒死

### 7.1 实测

- S3 共 8 次 `browser_control`，**8/8 失败**：`browser_not_found: no browser executable found
  (ORZ_BROWSER_PATH unset; searched: chrome, google-chrome, …)` → `capability_unreachable`，
  `wall_ms` 8–11 ms（零预算烧蚀）；
- **7 批中 4 批**的首个子代理回合即含 `browser_control`（extract act0 更是**首个工具**）；
  子代理随后自述"本地浏览器车道不可用，改走原生道"——该首选来自检索子代理系统提示
  「本地浏览器检索｜推荐首选」；
- **探针不同步**：三 run 的 `tool_availability_check.retrieval_family.browser.present=true`（`detail: browser_read`），
  与实际"无可执行件"不符（r3 §10.8 已按"工具在位≠可执行件在位"记注）。

### 7.2 影响

墙钟成本≈0，但每批多一个子代理回合与一次失败叙事；若按 7 批计，约 4 个额外子代理回合。
**这不是超时主因**（0ac 判据面已早解决死车道烧预算），属**噪声级摩擦**。

### 7.3 处置候选

1. 探针补"可执行件在位"位：`ORZ_BROWSER_PATH` unset 且 PATH 扫描不到浏览器 ⇒ `present=false`，
   子代理不再首选死车道（不动任务镜像、不启用 `eval_browser`，与既定裁决一致）；
2. 或维持现状（F3 裁决：官方 minimal 环境即如此，探针语义记注）。

## §8 跨项共性

1. **"可用"口径缺可引用／可执行维度**（N1／N5／N6）：`visibility ∈ {full,partial}` 把无 URL 合成文本、
   派生证据、死车道噪声一律算作"可用"，既让阈值可被凑满（N5），又让逐 query 归因失真（N1）；
2. **环境参数 vs 本机基线**（N4／N2）：25 % commit 门在本机结构性不可满足；run 级墙钟余量在派发前无人过问；
3. **观测面缺口**（N3／N1 二次事实）：模型面消息与逐 query 归因依据均无落盘面，
   使"判据可核"退化为"只能核到汇总量"。

## §9 处置候选汇总（待裁决）

| 优先级建议 | 候选 | 影响面 | 代价 |
|---|---|---|---|
| P1 | N4-②③ heavy 判定参数感知＋引号感知分段 | 解除"只读命令／版本查询被拒" | 小（纯分类器） |
| P1 | N4-① 25 % 门按环境可满足性重定 | 解除 gcc/tar 等真实重活的常态封禁 | 中（需定新参数组与审计） |
| P2 | N1-② `unattributed_usable_count` | 主代理不再误读覆盖度 | 小（契约加字段） |
| P2 | N3-① `mechanical_audit_update{kind:"retrieval_batch"}` | 判据 7 后段可核 | 小（journal 面） |
| P2 | N2-①② 尾部保留＋未派发拒绝 | 消除尾部白烧、保落盘窗口 | 中（派发层） |
| P3 | N1-① `origin_query_id` 谱系 | 根治逐 query 归因 | 中大（证据模型＋契约） |
| P3 | N5-① 可引用性维度 | 阈值不被合成文本凑满 | 中（口径定义） |
| P3 | N6-① 探针可执行件位 | 去噪声 | 小 |

## §10 证据物

- 深挖脚本（仓外件）：`D:\tb-eval\_heavy_probe.py`；读数核算脚本：`D:\tb-eval\_s3_analysis.py`
- 三 run journal 与 `retrieval-results`：见 [S3 复验报告](0AR_S3_THREE_TASK_VERIFY_2026-09-20.md) §9
- 基线对照：r1–r3 九 run（[`TB21_V41_TIMEOUT3_VERIFY_2026-09-18`](TB21_V41_TIMEOUT3_VERIFY_2026-09-18.md) §10）
- 上游分析（A2 原案）：[`TB21_RETRIEVAL_VOLUME_CONVERSION_DEEP_ANALYSIS_2026-09-19`](TB21_RETRIEVAL_VOLUME_CONVERSION_DEEP_ANALYSIS_2026-09-19.md) §5.1
