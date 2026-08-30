# 检索与子代理沟通外部借鉴调研记录（2026-08-30）

> 性质：调研记录与设计输入（外部实践借鉴清单 + 本地现状核对）。
> 2026-08-30 追加：§8 含用户裁决记录（双模式保留 + 原生兜底）与 Bing 增强调研；
> §9 为 2026-08-30 检索问题最终评判收口（用户确认无异议）——裁决项以
> §8/§9 登记为准，其余仍为调研输入。
> 背景：2026-08-30 用户指示——①将「多代理沟通 + 本地检索」可借鉴内容落盘；
> ②再补一轮「本地浏览器调起检索」的具体实现与可借鉴项（当前本地浏览器调用较慢）。
> 用户方向：机械层可适当加厚、模型层不做额外变动与设计。
> 纪律：**本次仅写入单一文档**（隔壁窗口正在拆分 controller，避免交叉污染）；
> 未改动 CLI_PROJECT_INDEX / TODO / 源码 / 其他设计文档；本文不登记索引。
> 权威关系：不替代 ADR-0010 与既有设计；外部实践仅作借鉴输入，落点映射均标注
> 「待设计轮裁决」。
> 本地上下文：`DEEPSEEK_CAPABILITY_AND_SUBTRACTION_DIRECTION_RESEARCH_2026-08-29`、
> `FRAMEWORK_EFFECTIVE_DESIGN_INVENTORY_2026-08-29`、
> `DUAL_RETRIEVAL_MODE_VALIDATION_DESIGN_2026-08-29`、
> `RETRIEVAL_SUBAGENT_WIRING_DESIGN_2026-08-25`、
> `MECHANICAL_AUDIT_LAYER_DESIGN_2026-08-24`、`CONTROLLER_SPLIT_DESIGN_2026-08-29`。

## 1. 背景与动机

### 1.1 最近跑分与失败画像

- S4 复验 31 题 k=1：8/31 解出；23 失败 = 18 墙钟超时（B1 web 检索时间黑洞
  12/18：web_search 无超时单次最高 1365s；B2 慢命令 6/18）+ 2 fold 桥剥
  reasoning_content 框架 400（A）+ 3 真实交付质量（C）。
- S5-1 已落地 web_search 客户端 120s 总超时 + connect 10s
  （`orz/crates/codegen/orz-tools/src/implementations/web_search/client.rs`）；
  S5-2 终端分层超时（普通 300s / 程序脚本 600s）+ 5 分钟中间回报已 S1/S2，
  S3 重建待续。
- 含义：上一轮失败主体不是本地检索或子代理沟通，而是外部检索黑洞与命令效率；
  本轮「本地检索可调用性/速度 + 子代理沟通」是下一批增量，不是旧失败的药方。

### 1.2 用户方向与本轮范围

- 机械层加厚、模型层不动：所有可借鉴项必须落在机械层（工具效率/索引/预算/
  并行/超时/结构化契约），prompt/注入/模型面零改动。
- 三个对象：①子代理↔主代理沟通；②本地检索（工作区索引/搜索）；③本地浏览器
  调起检索（local_browser lane 的启动/读取速度）。

### 1.3 当前源码断点（2026-08-30 核对）

| 断点 | 位置 | 说明 |
|---|---|---|
| 内部检索触发面被封存 | `orz/crates/orz-loop/src/retrieval/projection.rs` `R1_SEALED_MAIN_TOOLS` | `retrieve_project_docs` 与 `project_doc_index` 在封存表内，主面不可调用（注释：移除名称即配置恢复） |
| 同轮工具串行 | `orz/crates/orz-loop/src/agent_loop.rs` `for tc in &response.tool_calls` | 同轮多个声明逐个执行，最慢拖死整轮 |
| 子代理无 run 级预算/超时 | `retrieval/dispatch.rs`（拆分后落点） | CONTROLLER_SPLIT §7 已登记为后续项 |
| 子代理回传无硬预算 | blackboard 指针摘要（默认） | `[DOC]` 结果无机械截断上限 |
| local_browser 每调用建/关 tab | `orz/crates/orz-host/src/local_browser/cdp.rs` `read_page_inner` | createTarget → navigate → wait loadEventFired → innerText → closeTarget |
| local_browser 缺容器参数 | `discovery.rs` `browser_launch_args` | 无 `--no-sandbox` / `--disable-dev-shm-usage` / `--disable-gpu`（CONTROLLER_SPLIT §4.2 已登记） |

## 2. 子代理↔主代理沟通可借鉴

### 2.1 外部实践清单

| 来源 | 做法 | 借鉴点 |
|---|---|---|
| Anthropic 多代理研究系统（2025-06） | orchestrator-worker；子代理完整产物落外部存储（文件系统），回传仅轻引用——避免「传话游戏」；主代理 3–5 子代理并行 + 子代理内 3+ 工具并行；委托契约=目标+输出格式+工具/来源+任务边界；按查询复杂度分档 effort（简单 1 agent 3–10 次调用 / 复杂 10+ agents）；工具描述即 UX（打磨描述降 40% 任务耗时） | ①子代理只回「结论+指针」，全文留 evidence；②委托契约四要素已在 `build_retrieval_task_goal`，可补复杂度分档；③并行双层；④工具描述打磨属机械层投入 |
| Claude Code 子代理（官方 SDK / Tembo） | 上下文隔离（只回摘要）、子代理工具白名单、后台 vs 前台、fork 继承父上下文省 prompt cache；SendMessage 直连（agent teams）；对子代理最终消息做指令形状扫描（防注入） | ①回传有界；②子代理只给所需工具（内部 lane 已只读族）；③返回面防伪控制块（IPG 已有等价语义） |
| A2A 协议（Google 2025-04） | JSON-RPC 2.0 + 任务状态机（submitted/running/input-required/completed/canceled）+ 结构化 artifacts | 与 activation→assessment→disposition→close 同构；借鉴=通信信封保持结构化、控制面不让自然语言混入 |
| LangGraph 多代理模式 | supervisor（中心路由）vs swarm（平级 handoff）；共享 state 用 reduced channel 合并多 worker 结果 | 同轮并行结果合并的机械语义（先完成者优先/合并去重） |

### 2.2 与 ORZ 现状对照

- 已有：blackboard 分区单写者、relay 纯函数路由、`[DOC]`/`[SOURCE]` 结构化结果、
  activation 生命周期、内部/外部双 lane、证据内容寻址落盘。
- 缺口：①回传预算（无硬上限）；②子代理 run 级预算/超时（未实施）；
  ③同一轮内检索类工具并行（未实施）；④双 lane 并发=2 的利用（外部 lane 多个
  独立检索任务并行，未启用）；⑤委托契约复杂度分档（未做）。

### 2.3 可借鉴落点（机械层，待裁决）

1. 子代理结果回传预算：`[DOC]` 结果机械截断（结构化头部 + evidence 指针 +
   超限截断标注），对齐 Anthropic「只回引用」。
2. 子代理 run 级预算/超时（轮数+墙钟双层），落点 `retrieval/dispatch.rs`。
3. 同轮读类/检索类工具并行（写类保持串行）。
4. 委托契约按查询复杂度分档（query 长度/scope 机械映射 effort 档）。

## 3. 本地检索（索引/搜索）可借鉴

### 3.1 外部实践清单

| 来源 | 做法 | 借鉴点 |
|---|---|---|
| Cursor Instant Grep（2026-03） | 稀疏 n-gram 倒排 + 双文件（postings + mmap 查找表）；git commit 基线 + 工作树增量层（模型刚写的代码立即可搜）；候选集上再做全文匹配；容忍 hash 碰撞（假阳性只扩候选） | 索引新鲜度是 agent 专用失效模式：陈旧索引 → 模型搜不到自己刚写的内容 → 空转烧 token；正是 size+mtime 盲区要补的 |
| codedb（2026） | 启动时建一次索引 + 文件 watcher 单文件增量重索引；查询永不重扫文件系统；结构化输出（symbol/outline/callers），一次搜索约 20 token vs grep 原始行 32K token；便携 snapshot 秒启 | 结构化返回比索引本身更值钱；索引驻留消除进程启动/全树 stat |
| AIKD（2026） | Rust BM25（Tantivy）+ 向量（ONNX）混合 + RRF；Blake3 哈希增量（零冗余扫描）；SQLite；10K chunk 查询 0.21ms；文件 watcher 自动重索引 | Blake3 内容哈希补 size+mtime 盲区；纯机械 BM25 贴合「机械层加厚」 |
| Aider repo map | tree-sitter 符号提取 + PageRank 引用排序 + 按 token 预算二分装填 | 符号级检索（「谁调用它」）是 grep 全文之上的一层 |
| Claude Code 源码实测（2026-05） | 4,500 文件/95 万行 rg 全量 ~0.1s | 小中型仓库索引收益低；索引价值在超大/单调仓库 + 高频调用 + 结构化输出 |

### 3.2 与 ORZ 现状对照

- 已有：`project_doc_index` size+mtime 增量扫描 + 跨 run 持久化快照 +
  逃生阀（GAP-PROJECT-DOC-INDEX-CACHE，implemented）；grep 结构化搜索信封
  （FUS-TOOL-SCOPE-CONTRACT）。
- 缺口：①内容哈希（同 size+mtime 修改不可察，仅逃生阀）；②索引驻留
  （每 query 全树 stat）；③符号级结构（无 outline/callers）；④可调用入口
  （`project_doc_index` / `retrieve_project_docs` 被封存）。

### 3.3 可借鉴落点（机械层，待裁决）

1. `project_doc_index` v2：git HEAD 基线快照 + 工作树增量层 + Blake3 内容哈希；
   索引驻留（一次启动扫描、query 零 stat）。
2. 大仓库场景再上 trigram/稀疏 n-gram 倒排（Cursor 方案）；小 workspace 保持 grep。
3. 结构化返回扩 symbol 维度（codedb / repo map 方向），作为后续通用场景增强。

## 4. 本地浏览器调起检索：现状核对与外部实现

### 4.1 ORZ 现状（2026-08-30 源码核对）

- 架构：`CdpBrowserSession`（每 session 一个浏览器进程、隔离 profile）+
  `LocalBrowserManager` + URL 门 + PDF 证据管线 + `browser_read`（full/preview/
  keywords）。
- 启动参数（`discovery.rs` `browser_launch_args`）：`--headless=new`（可选）、
  `--remote-debugging-port=0`、`--remote-allow-origins=*`、`--user-data-dir`（隔离）、
  `--no-first-run`、`--no-default-browser-check`、`--disable-extensions`、
  `--disable-background-networking`、`--disable-sync`、`--disable-translate`。
  缺：`--no-sandbox` / `--disable-dev-shm-usage` / `--disable-gpu`（docker 必需，
  CONTROLLER_SPLIT §4.2 已登记）。
- 单次读取路径（`cdp.rs` `read_page_inner`）：`check_navigation_url`（DNS 预检）→
  browser ws 懒连接（会话级一次）→ `Target.createTarget(about:blank)` →
  page ws 连接 → `Page.navigate` → `wait_for_load`（loadEventFired 终态）→
  `Runtime.evaluate` innerText → `Target.closeTarget`（每调用一 tab，全程 5s 收尾）。
- 慢的根因拆解：
  1. 每次调用建 tab + 连 page ws + 关 tab（固定往返 3+ 次 CDP 消息）；
  2. 等 loadEventFired（完整加载，含慢三方脚本/资源），不是「可用文本就绪」；
  3. 未拦截图片/字体/媒体/样式等无关资源；
  4. 单 tab 串行（同轮多页面读取逐个来）；
  5. 每次调用 DNS 预检（跨 host 无缓存）；
  6. docker 下缺参数导致启动失败/崩溃（TB 场景 probe 直接降级）。

### 4.2 外部实现与提速清单

| 来源 | 做法 | 借鉴点 |
|---|---|---|
| browser-use Fast Agent（2026-03） | `minimum_wait_page_load_time=0.1`、`wait_between_actions=0.1`、headless、flash_mode | 「不等完整加载、按最小可用等待」的等待策略参数化 |
| Jina Reader 架构 | 混合引擎：CURL（静态页直取 HTML）→ headless Chrome（JS 渲染）→ Cloudflare Browser Rendering（兜底）；Auto 模式按内容特征智能选择；readability 清洗 HTML→Markdown；节点过深回退 HTML-to-text | 静态页别用浏览器：先直连抓取，浏览器只留给 JS 渲染/反爬页；「能用更简单工具就不用浏览器」是提速第一原则 |
| Cloudflare Browser Run（reuse sessions） | `browser.disconnect()` 代替 `browser.close()` 保持浏览器存活，下次请求 reconnect；空闲超时自动回收 | 会话级浏览器复用消除冷启动；断开连接而非销毁 |
| OpenChrome（2026） | 单 Chrome 进程 + 多隔离 tab（20 并行 lane ≈300MB）；持久 profile（cookie/localStorage 复用免重登）；首个工具调用自动启动；daemon 模式 + idle-timeout；`read_page mode=dom` token 高效序列化（5–15x 少 token） | 单进程多 tab 并行是提速主杠杆；profile 复用免冷启/免重登；token 高效序列化 |
| Playwright / Chrome DevTools MCP | 连接已打开 Chrome（CDP）、上下文状态持久化（cookies/localStorage）、tab 会话隔离 | 连接复用 + 状态持久化的具体实现路径 |
| headless 资源优化（通用爬虫实践） | 参数：`--disable-gpu`、`--disable-dev-shm-usage`、`--no-sandbox`、`--disable-setuid-sandbox`、`--no-zygote`、`--disable-software-rasterizer`、`--mute-audio`、`--hide-scrollbars`、`--metrics-recording-only`、`--blink-settings=imagesEnabled=false`；资源拦截（image/stylesheet/font/media）；waitUntil=domcontentloaded 而非 load；显式超时 | 直接对应 ORZ 当前缺项：容器参数、资源拦截、等待语义 |
| 并行 tab 池（chromux / openchrome） | worker-tab 池替代「一 URL 一 tab」；多 tab 并发 | 有界 tab 池 + 并行读取（写面不变） |

### 4.3 可借鉴落点（机械层，待裁决；分三类）

A. 低风险（参数/等待语义，不动架构）：

1. 容器启动参数补齐：`--no-sandbox`、`--disable-dev-shm-usage`、`--disable-gpu`
   （与 CONTROLLER_SPLIT §4.3 注入方案合并）。
2. 启动参数再补：`--disable-software-rasterizer`、`--mute-audio`、
   `--hide-scrollbars`、`--metrics-recording-only`；文本读取面加
   `--blink-settings=imagesEnabled=false`（preview/keywords 模式适用）。
3. 等待语义：等待「可用文本就绪」而非 loadEventFired——load 事件后立即
   `Runtime.evaluate` innerText，或按短间隔轮询 innerText 非空即返回；
   preview/keywords 模式尤其受益；full 模式保持现有 load 终态。
4. 资源拦截：`Network.setBlockedURLs` 或 Fetch 域拦截图片/字体/媒体/
   三方脚本（preview/keywords 模式启用，full 模式可配）。

B. 中风险（架构内改动，需设计轮）：

5. 会话级 tab 池：有界 N 个 tab 常驻（create 一次、navigate 复用、LRU 回收），
   替代「每调用 create+close」；保留「owned tab」隔离语义（每 lane/调用独占
   tab、不可跨调用共享）——需与 ADR-0010 §3.7.6「one tab lives exactly for
   this call」的裁决对齐。
6. 同轮多页面并行读取：`browser_read` 读类并行（配合 §2.3-3 同轮并行项），
   tab 池 + 并行 = 8 页串行 fetch 场景分钟级收敛。
7. DNS 预检结果会话级缓存（按 host + TTL），保留 redirect 重检门。

C. 需裁决的方向性选项（可能与 ADR-0010 §3.7.12「二存一」冲突）：

8. 模式内混合路由：local_browser 模式内先 web_fetch 直连（静态页），
   JS 渲染/反爬页才用 browser_read——Jina/Firecrawl 的 Auto 模式；
   若采纳需修订「禁止混用/隐式切换」语义，属 ADR 变更，不默认推荐。
9. 浏览器保持常驻跨 run（daemon 化）——与本项目「每 run 收口进程」
   生命周期冲突，暂缓。

### 4.4 诚实边界

- local_browser 的价值是一手原文准确性（渲染型页面/登录站），不是超时解药
  （官方 minimal mode 无浏览器；12/18 超时挂在 web_search 而非浏览器缺失）。
- 上述提速目标是「把浏览器读取从分钟级压到秒级」，不是「让浏览器快过
  web_fetch 静态直连」；静态页永远直连更快。

## 5. 落点映射汇总（全部机械层、模型面零改动）

### 第一批（低风险，可并入现有 S5-2 批次节奏）

1. 恢复 `retrieve_project_docs` 可调用：从 `R1_SEALED_MAIN_TOOLS` 移除
   （一行配置；注释已写明恢复方式）。待设计轮确认与 R1 薄化意图的关系。
2. 同轮读类/检索类工具并行（FuturesUnordered，写类保持串行）。
3. 子代理结果回传预算（`[DOC]` 机械截断 + evidence 指针）。
4. 浏览器容器参数 + 资源拦截 + 等待语义（§4.3 A 组）。
5. web_search 并发保持 1（S5-1 已加 120s 超时）。

### 第二批（独立设计轮）

6. `project_doc_index` v2（git HEAD 基线 + 增量层 + Blake3 内容哈希 + 驻留索引）。
7. 子代理 run 级预算/超时（`retrieval/dispatch.rs`）。
8. 会话级 tab 池 + 同轮多页面并行读取（§4.3 B 组）。
9. 委托契约复杂度分档。

### 暂缓/不建议

- 向量语义检索（模型/资源敏感、小工作区收益低、与模型层不动冲突）。
- web_search 并发 >1（DUAL_RETRIEVAL_MODE_VALIDATION 已论证）。
- 子代理异步后台化 / 浏览器 daemon 跨 run 常驻（与现有生命周期差异大）。
- 模式内混合路由（除非专门修订 ADR-0010 §3.7.12）。

### 纪律提醒

- 未闭合链先行：W1-R1 S4、W3-R3、W4-R4 S4 勾选、S5-2 S3/S4 +
  acaf_e2e 7 项回归（FRAMEWORK_EFFECTIVE_DESIGN_INVENTORY §8）。
- 新增机制一律视为债务（THIN-HARNESS-REDESIGN V2 P4/P6），逐条证明必要性。

## 6. 参考链接

- Anthropic 多代理研究系统：https://www.anthropic.com/engineering/multi-agent-research-system
- Claude Code Subagents in the SDK：https://code.claude.com/docs/en/agent-sdk/subagents
- Claude Code Subagents 实践指南（Tembo）：https://www.tembo.io/blog/claude-code-subagents
- A2A 协议（Google）：https://developers.googleblog.com/en/a2a-a-new-era-of-agent-interoperability/
- LangGraph 多代理模式：https://github.com/SpillwaveSolutions/mastering-langgraph-agent-skill/blob/main/references/multi-agent-patterns.md
- Cursor Instant Grep 解读（ZenML）：https://www.zenml.io/llmops-database/fast-regex-search-indexing-for-ai-agent-tool-performance
- Cursor Instant Grep 中文摘要：https://www.theblockbeats.info/flash/337764
- codedb：https://github.com/justrach/codedb
- AIKD：https://github.com/gelutjari/aikd
- Aider Repo Map 模式：https://github.com/agentpatterns-ai/website/blob/main/context-engineering/repository-map-pattern.md
- grep vs 索引实测：https://www.bestblogs.dev/article/3dcf5158
- browser-use Fast Agent：https://docs.browser-use.com/open-source/examples/templates/fast-agent
- Jina Reader 架构：https://github.com/jina-ai/reader/blob/main/architecture.md
- Cloudflare Browser Run 会话复用：https://developers.cloudflare.com/browser-run/features/reuse-sessions/
- OpenChrome：https://github.com/shaun0927/openchrome
- headless 资源优化：https://www.searchcans.com/blog/optimize-headless-browser-resource-usage-scraping/
- SearXNG（自托管元搜索）：https://docs.searxng.org/ / https://github.com/searxng/searxng
- metasearch2-mcp（Rust 元搜索 + MCP，无 key）：https://www.npmjs.com/package/metasearch2-mcp
- metasearch-rust（SearXNG 风格 Rust，RRF 共识排序）：https://github.com/MikeLuu99/metasearch-rust
- clean-search-mcp（spam 过滤 + 多引擎兜底）：https://himcp.ai/server/clean-search-mcp
- search-engine-parser（SERP 结构化解析参考）：https://github.com/strogo/search-engine-parser
- Bing Search API 下线与 HTML 抓取（2025-08-11）：https://apiserpent.com/blog/scrape-bing-search-for-free

## 7. 关联文档（本地）

- `docs/DUAL_RETRIEVAL_MODE_VALIDATION_DESIGN_2026-08-29.md`
- `docs/RETRIEVAL_SUBAGENT_WIRING_DESIGN_2026-08-25.md`
- `docs/RETRIEVAL_MECHANICAL_CONTROLS_DESIGN_2026-08-13.md`
- `docs/CONTROLLER_SPLIT_DESIGN_2026-08-29.md`
- `docs/FRAMEWORK_EFFECTIVE_DESIGN_INVENTORY_2026-08-29.md`
- `docs/THIN_HARNESS_REDESIGN_V2_DESIGN_2026-08-28.md`
- `adr/ADR-0010-fusion-runtime-and-agent-architecture.md`

## 8. 定案与搜索引擎增强调研（2026-08-30 用户裁决）

### 8.1 用户裁决：双模式保留，原生 API 兜底

- 本地引擎检索（local_browser 模式，浏览器直驱 SERP）与原生 API 检索
  （framework_fallback，DeepSeek web_search）**双保留**；原生 web_search
  作为兜底。
- 引擎主序（2026-08-30 追加裁决）：**Google 为主**，先跑一轮观察 Google
  门禁/限制（个人使用强度，非高强度爬虫）；若 Google 限制过强再回退
  Bing 主序。
- 语义：local_browser 可用 → 引擎 SERP（Google 优先）为主；浏览器不可用
  （probe 失败）→ 模式 A 机械降级 framework_fallback → 原生 web_search
  兜底（既有 `retrieval_mode_transition` 事件）。
- **SERP 页面级失败裁决（2026-08-30）**：SERP 页面级失败（CAPTCHA /
  PAGE_BLOCKED / 429 / consent 等）**直接触发原生兜底**，显式记录——走
  `retrieval_mode_transition`（authority=mechanical_probe、reason 显式），
  沿用模式 A 语义，不静默混用；candidate 页面读取（browser_read）仍保持
  §3.7.2 显式失败不降级。

### 8.2 Bing 增强相关项目（调研）

| 项目 | 定位 | 对 orz 的借鉴点 |
|---|---|---|
| SearXNG | 自托管元搜索引擎（AGPL-3.0，29.3K★），聚合 70+ 引擎（Google/Bing/DDG/Brave…），插件 `filter_func` 过滤、JSON API、按引擎超时 | 最成熟的「引擎之上机械过滤层」参考；Python 服务，组件化较重 |
| metasearch2（Rust，mat-1） | Rust 元搜索，聚合 Google/Bing/Brave，去重+排序；有 MCP 包装（metasearch2-mcp），无 key | 语言栈与 orz 对齐，可作组件或参考 |
| metasearch-rust（MikeLuu99） | SearXNG 风格 Rust：并发 fan-out、URL 归一化去重、**RRF 共识排序（score=Σ1/(60+rank)，多引擎命中者排前）** | 对抗「Bing 单引擎污染」的核心机械件：被污染结果在多引擎共识中不存活 |
| clean-search-mcp | MCP 服务，过滤 content farms/恶意站/SEO 垃圾，Yandex+Bing+DDG 自动兜底 | 「spam-free 检索 + 多引擎兜底」的功能规格参考 |
| search-engine-parser（Python） | SERP 结构化解析：Bing 有机结果 `li.b_algo`、广告 `b_ad` 块 | SERP 解析参考；选择器脆弱性已知（引擎改 DOM 即断），仅作参考不做依赖 |

事实：微软 2025-08-11 下线 Bing Search API（无官方有机结果 API）；Bing HTML
是主流引擎中反爬最轻的，纯 HTTP 常返回干净 HTML——「优化 Bing」= SERP 解析 +
机械过滤，走浏览器/HTTP 即可，无需 API key。

### 8.3 引擎污染防护机械清单（Google 主序 / Bing 通用）

1. SERP 结构化解析：只取有机结果块（`li.b_algo`），剔除广告（`b_ad`）、
   相关搜索/视频/购物块。
2. 跨引擎共识：Google + Bing（+ DuckDuckGo 可选）并发 fan-out → URL 归一化
   去重 → RRF 排序；单引擎污染结果降权。
3. 域名质量清单：复用 `SOURCE_QUALITY_SEED_LISTS`（劣质 0.7：CSDN/知乎/百家号/
   微博等）+ content-farm/恶意域名单；白名单（gov/学术）1.1。
4. URL 归一化 + 重定向链解析（防 cloaking），复用 URL gate canonical 化。
5. 新鲜度/形态启发式（可选）：标题长度异常、URL 深度、parked 域特征。
6. 显式失败态：CAPTCHA / consent / 429 → 显式状态（§3.7.2 词汇），
   按 §8.1 裁决路径（SERP 失败 → 原生兜底）兜底。
7. 保底顺序：Google → Bing → DuckDuckGo → 原生 web_search（用户裁决原生兜底）。

### 8.4 关联待裁决与验证项

- ADR-0010 §3.7.10 语义登记：引擎 SERP 通道非「第二 API 供应商」（无 key/计费），
  但检索来源从 DeepSeek 服务端搜索变为引擎 SERP，需登记修订。
- 引擎 SERP 工具（`search_engine_search`）的 host 直执行 vs 子代理派发。
- **Google 门禁观察实验（2026-08-30 用户裁决先行）**：以 Google 为主跑一轮
  个人使用强度的检索（小批、k=1 或构造题），观测 consent cookie 处理、
  CAPTCHA 频率、IP 节流/429、页面结构稳定性；结果决定是否回退 Bing 主序，
  并校准 SERP 失败兜底的触发阈值。

### 8.5 人化输入延迟裁决（2026-08-30）

- 用户裁决：Google 主序下，接受「每次引擎 SERP 查询注入人化输入延迟」的成本；
  只要结果质量足够，延迟收益足够（个人使用强度，非高强度爬虫）。
- 设计点（host 机械层，模型不可见、不进 prompt、不计模型轮次）：
  1. 键入模拟：焦点搜索框 → 逐字符键入（50–150ms 抖动）→ 提交前停顿
     （300–800ms）→ Enter；替代直接 URL 导航（URL 直达更易被识别为 bot）。
  2. 查询间冷却：同引擎查询间随机冷却（默认 2–5s，`ORZ_ENGINE_QUERY_PACING_MS`
     与 jitter 可配），自然串行化（与 web_search 并发=1 纪律一致）。
  3. 会话级频率上限：单 run/session 引擎查询次数硬上限（待定，建议 20–30），
     超限走显式失败 → 原生兜底。
  4. candidate 页读取保持正常频次（普通浏览形态），不套 SERP 级冷却。
- 校准：Google 门禁观察实验（§8.4）同时产出 pacing 校准数据——CAPTCHA/429
  率与延迟的关系，作为参数默认值依据；若延迟容忍后仍高频触发门禁，回退
  Bing 主序。
- 边界：pacing 不影响原生 web_search 兜底路径；键入模拟仅用于 Google 主路径，
  Bing/DDG 可先用 URL 直达 + 冷却（其反爬更轻，待观察后对齐）。

## 9. 检索问题最终评判与定案收口（2026-08-30 用户确认）

> 本窗口代理综合裁决（依据：§8 用户裁决 + DUAL_RETRIEVAL_MODE_VALIDATION_DESIGN
> 度量维度 + controller 拆分完成后的落点核验），用户 2026-08-30 确认无异议。

### 9.1 最终评判

- 双模式定案成立（引擎 SERP 为主、原生 web_search 机械兜底）；检索问题的真正
  病灶不在「选哪条检索通道」，而在**编排效率**——同轮串行执行、子代理无 run
  级预算、[DOC] 结果无界、浏览器生命周期开销。方向已由 §8 用户裁决收口，本节
  收口剩余悬置项。

### 9.2 三个悬置项的最终裁决

1. **不做全量 A/B**：双模式定案后 A/B 的「选边」功能消失；改为 **Google 门禁
   观察实验**（小批检索密集题 k=1、个人使用强度），套用 DUAL 文档的度量维度
   （reward/超时/引用绑定/墙钟/工具轮数/token），一次完成「引擎可用性验证 +
   pacing 校准 + Google/Bing 主序裁决」。
2. **同轮读类并行：批准，第一批第一项**：FuturesUnordered 并行读/检索类工具，
   写类保持串行；web_search 信号量仍为 1，并行只消除「最慢拖死整轮」。
3. **web_search 并发维持 1**：生成式检索昂贵 + §8.5 pacing 已自然串行化；
   收益从「同轮并行 + 预算」拿，不从提升 permit 拿。

### 9.3 第一批实施清单（收口排序）

1. 同轮读类并行（`agent_loop.rs` 工具批次执行处）；
2. 子代理 run 级预算/超时（从 §5 第二批提前至第一批——12/18 超时挂在子代理
   web_search 链路上，落点 `retrieval/dispatch.rs`，controller 拆分已完成边界）；
3. [DOC] 回传机械截断（结构化头部 + evidence 指针）；
4. 浏览器容器参数 + 资源拦截 + 等待语义（§4.3 A 组）；
5. Google 门禁观察实验（与 1–4 并行准备）。

第二批（独立设计轮）：`project_doc_index` v2（Blake3 增量 + 驻留索引）、
会话级 tab 池 + 同轮多页并行读取、委托契约复杂度分档。

### 9.4 暂缓/否决

- `retrieve_project_docs` **维持封存**（`R1_SEALED_MAIN_TOOLS` 不动；解封是
  模型可见面变化，与「机械层加厚、模型层零改动」纪律冲突，且与 R1 薄化意图的
  关系未定）；
- 向量语义检索、web_search 并发 >1、浏览器 daemon 跨 run 常驻、模式内混合路由
  ——暂缓/不建议。
- **模式内混合路由（2026-08-30 修订，用户裁决「先恢复外部」）**：由暂缓改为
  「主面 web_search 派发入口」——local_browser 下主面保留 `web_search`
  单一入口（描述标注派发外部检索子代理），模型调用即派发 ExternalRetrieval
  子代理；执行面二存一不变（外部 lane 仅 browser_read 引擎 SERP，原生
  web_search 兜底为机械路径 retrieval_mode_transition）。ADR-0010 §3.7
  条 12 已按 v1.9 修订，索引 §14.44。内部检索维持封存。

### 9.5 顺序纪律

- 检索批次不得插队到 S5-2 S3/S4 重建 + acaf_e2e 7 项回归（ad5f9ee 遗留）闭合
  之前——基线不干净时检索改动无法归因。顺序：未闭合链闭合 → 第一批检索机械层
  → Google pilot。

### 9.6 登记

- ADR-0010 §14.43（引擎 SERP 通道语义、SERP 失败兜底、web_search 并发维持 1、
  `retrieve_project_docs` 封存维持、机械层第一批）；ADR-0010 §14.44（主面
  恢复外部 web_search 派发入口，2026-08-30 用户裁决）；
- BACKLOG 0k / TODO P0-0k（实施批次与勾选）；
- CLI_PROJECT_INDEX（FUS-RETRIEVAL-ENGINE-SERP）。
