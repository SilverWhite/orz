# 0ac S1 探针记录：流式检索与分段续写（2026-09-13）

> 状态：**S1 探针完成**（`GAP-MECH-IMMEDIATE-FEEDBACK` 第一门）。本记录只做探针与结论，不改生产代码、不改预算值、不改契约。
> 设计稿：[`即时结果回报与流式检索设计`](../IMMEDIATE_RESULT_DELIVERY_AND_STREAMING_RETRIEVAL_DESIGN_2026-09-13.md) §6 S1；放行 = 用户 2026-09-13 指示「请开始进行 0ac S1 探针部分」。
> 探针件：`D:\tb-eval\probe-0ac-s1\`（一次性探针脚本 + 全量事件落盘，不入仓）；API = `https://api.deepseek.com`，key 取自 `D:\tb-eval\.env`（与生产同一凭据）。

## 1. 探针①：流式 `/responses` + `web_search`（SSE 事件序列与时间戳）

### 1.1 实测事件序列（`deepseek-v4-pro`，stream=true，t 从请求发出计）

| t (s) | 事件 |
|---|---|
| 9.28 | HTTP 200，`Content-Type: text/event-stream`；`response.created`、`response.in_progress` |
| 9.95 | `response.output_item.added`（`type=reasoning`）→ `reasoning_text.delta` ×N |
| 10.49 | reasoning 完成（`reasoning_text.done` → `output_item.done`）|
| **10.49** | **`response.web_search_call.in_progress`**（首个检索进度事件）|
| 11.13 | `response.web_search_call.searching` → `response.web_search_call.completed` → `output_item.done`（第 1 次检索，~0.6 s）|
| 13.98 / 17.80 | 第 2、3 次 web_search_call（各 ~2–3 s，与 reasoning 交替）|
| 18.31–19.57 | `message` 项：`output_text.delta` ×N → `output_text.done` → `output_item.done` |
| 19.57 | `response.completed`（**未观察到 `[DONE]` 哨兵**，流以 `response.completed` 终止后 EOF）|

全量事件已落盘 `p1_stream_pro_events.jsonl`（探针件目录）。事件名与设计 §3.3 候选枚举**吻合**：`response.web_search_call.in_progress / searching / completed` 实测存在；本轮未见 `open_page` 动作项（三次均为 search 类 completed）。

### 1.2 对设计判据的回答

1. **10 s 内是否有可用首事件？——是，但有两个观测点要分清**：SSE 通道级首事件（`response.created`）随 HTTP 响应头到达（本次 9.28 s；另一跑 4.79 s，TTFB 波动大）；**检索级首个进度事件**（`web_search_call.in_progress`）在 10.49 s——超过 10 s 一点，原因是 TTFB（~9.3 s）+ 首段 reasoning（~1.2 s）。**结论：判活判据应落在「SSE 首字节/通道首事件 ≤10 s」（TTFB 主导）而非「首个检索事件 ≤10 s」（受 reasoning 阶段摆布）**；首个检索进度事件在判活后再等（其到达时间稳定，reasoning 完成后 ~0–1 s 内）。
2. **流式路径可行**：检索进度事件真实暴露、逐事件带时间戳可测 ⇒ **不需要转分段检索后备路径**（设计 §3.5 后备不启用）。
3. **首个结果事件**：`web_search_call.completed`（t≈11.1 s）即可作为首个可投递结果锚点；文本段在全部检索完成后才流出。
4. **fail 分类的实测锚点**：10 s 内无 SSE 首字节 → `network_no_response`（TTFB 主导）；有通道无检索事件 → 结合 §1.4 的模型绑定发现归 `capability_unreachable`（工具未绑定属能力级）。

### 1.3 模型间差异（重大运行面发现）

同一 payload（`tools=[{"type":"web_search","filters":{}}]`，与生产 `WebSearchToolArgs` 序列化形态一致）：

| 模型名 | 流式 | 非流式对照 | web_search_call |
|---|---|---|---|
| `deepseek-v4-pro` | 有（3 次检索） | 有（3 次检索） | **正常绑定** |
| `deepseek-v4-flash`（生产默认） | 无（模型 reasoning 明说"没有 web 搜索工具"，随后编造来源） | 无（4/4 次重复对照均不触发） | **确定性不绑定** |
| `deepseek-flash` | — | 无 | 不绑定 |

**矛盾与登记**：第 0 轮跑批（同日早些时候，`run_official_2.1.sh` `MODEL="deepseek-v4-flash"`）实测有 116 条真实 web_search（p50 29.9 s）⇒ 当日早些时候 flash 兼容路由**能**绑定；本探针（同日晚些时候）**4/4 不绑定**。两种可能：服务端兼容路由行为当日变化，或绑定间歇性。**影响**：若 flash 路由已稳定失去 web_search 绑定，则当前生产检索通道正在静默退化为"模型凭内部知识作答"（比慢检索更危险——无任何错误信号）。**需在 S3 实现前复验一次并把「web_search_call 存在性」纳入探针 `retrieval_family` 读数**（这正是 0ac 探针扩面的价值实证）。

### 1.4 其余实测细节

- `store:false`、`temperature:0.1`、`max_output_tokens:8192` 与生产一致；`filters:{}` 缺省形态在两个模型上行为一致（绑定与否只随模型名变）。
- flash 模型流式跑（`p1_run2.log`）：SSE 机制本身正常（首事件 0.23 s），但 `max_output_tokens:2048` 被 reasoning 烧满 → `response.incomplete`（**流式下 reasoning 也计入 output tokens**，探针脚本首版教训）。

## 2. 探针②：分段续写（部分 assistant + reasoning + 注入事实）

### 2.1 构造

`/chat/completions`，messages = [user 任务, **assistant（content=中断在分号后的半句 + `reasoning_content`=半截思维链）**, user（`[机械事实注入]` + 从中断处继续的指令）]，三个模型名各一次，max_tokens=2048。

### 2.2 结果（3/3 通过）

| 模型 | 耗时 | finish | 接受部分 assistant+reasoning 输入 | 从句号边界继续 | 重复已说内容 |
|---|---|---|---|---|---|
| `deepseek-flash` | 5.5 s | stop | 是（HTTP 200） | 是（直接续写第二要点） | 否 |
| `deepseek-v4-flash` | 4.9 s | stop | 是 | 是 | 否 |
| `deepseek-v4-pro` | 14.9 s | stop | 是 | 是 | 否 |

- **无重复**：续写文本不含被注入的部分句原文；
- **无配对破损**：API 接受带 `reasoning_content` 的 assistant 输入消息并正常返回（未复现 DeepSeek reasoner 老文档"不接受 reasoning_content 回传"的限制；v4 系列实测无此约束）；
- **注入事实被当上下文**（模型复述了注入内容，未被当成新指令执行危险动作）；
- **教训一条**：max_tokens=512 时 reasoning 烧满、content 为空（`finish=length`）——分段续写请求的 token 预算必须为"续写 + 重新 reasoning"两部分留量。

## 3. S1 结论与对 S2/S3 的输入

1. **流式检索路线确认**（设计 §3.1 选型维持）：SSE 逐事件可观测，`web_search_call.in_progress/searching/completed` 即设计 §3.3 需要的枚举；分段检索后备路径不启用。
2. **判活口径修正**：10 s 判活锚点 = SSE 通道首字节（TTFB 主导，实测 4.8–10.3 s 波动）；首个检索进度事件作为第二锚点（reasoning 后 ~1 s），不承担 10 s 判活。
3. **新增必查项**：`web_search_call` 存在性必须进探针 `retrieval_family` 读数与 S3 稳定码（flash 路由静默失绑是 `capability_unreachable` 的真实形态——模型侧无任何错误，只有 reasoning 自述"没有工具"）。
4. **分段续写机制（M1）可行**：部分 assistant + reasoning_content + 注入事实被接受、续写无重复；S3 需处理的残余 = 句号边界判定（本轮用分号人工切分，真实切分器按设计 §4.2）与续写 token 预算。
5. **`[DONE]` 哨兵不存在**：终态判据用 `response.completed` 事件 + EOF，不能等 `[DONE]`。

## 4. 边界与未做项

- 探针未覆盖：多轮 web_search + tool 结果回传的流式形态（S3 客户端实现时覆盖）；断流/重连行为；`open_page` 动作事件的实测（本轮三次均为 search 类）。
- 未改任何生产代码/契约/预算；探针件不入仓不注册（`D:\tb-eval\probe-0ac-s1\`）。
- 下一步：**S2 机器合约**（schema/verifier/fixtures 先行，事件与稳定码按本记录 §3 落枚举），待放行。

## 5. 深挖补记（2026-09-13 晚，flash 失绑定性 + 外部证据）

### 5.1 变体实验（flash 上 8/8 变体零 `web_search_call`）

在 `deepseek-v4-flash` 上逐一排除请求侧因素：`tool_choice:"auto"` / `"required"`、工具类型 `web_search_preview`、`search_context_size:"high"`、`input` 换 messages 数组形态、"必须联网才能答"的问题（BTC 实时价）、流式 + `tool_choice:required`——**全部不产生 web_search_call**。其中 `tool_choice=required` / `web_search_preview` 三例还出现 reasoning `incomplete`（15–27 s 后无 message 收尾）。同 payload 的 `deepseek-v4-pro` 对照（BTC 实时价）正常发起 11 次 web_search_call（5 completed / 6 failed）。结论：**失绑与请求构造无关，是服务端按模型/路由的行为差异。**

### 5.2 外部证据：Responses API 服务端 web_search 已被官方下架

- **官方文档（一手，2026-09-13 晚核对）**[`api-docs.deepseek.com/zh-cn/guides/responses_api/`](https://api-docs.deepseek.com/zh-cn/guides/responses_api/)：Tools 兼容表明载 **「web_search / file_search / code_interpreter / computer_use / mcp 等内置工具忽略」**；模型支持仅 `deepseek-flash`；对历史 `web_search_call` item 只做"还原拼接进上下文"。即：**字段被静默忽略是官方文档化行为，非故障**。
- **社区合订本**：linux.do《DeepSeek API 文档更新合订本》同口径记载「Responses API 服务端联网搜索 web_search 全部下架，相关字段被静默忽略」（直连被墙，经检索摘要转引）。
- **时间线吻合**：web_search 曾于 V4 Flash 时代在 `/v1/responses` 短暂上线（知乎《悄悄上线联网搜索》），本日早些时候第 0 轮跑批 `deepseek-v4-flash` 仍有 116 条真实检索；当晚 flash 路由 4/4 失绑 + `deepseek-v4-pro` 仍能触发 ⇒ **下架是按路由灰度推进的，v4-pro 是尚未关停的残余通道**。
- **`deepseek-v4-pro` 通道的死刑时间**：官方公告（[`news260910`](https://api-docs.deepseek.com/zh-cn/news/news260910/)）明载 **2026-09-14 12:00 起 `deepseek-v4-pro` 请求全部路由到 V4.1 Flash 并按 Flash 计价**——届时唯一还能触发服务端 web_search 的模型名也将进入不支持 web_search 的 Flash 后端，**服务端检索通道预计全量关闭**。

### 5.3 对 0ac / 本轮的影响（登记，不改实现）

1. **设计 §3 的流式检索路线（服务端 `web_search` + SSE）的地基正在被服务端撤走**：S1 实测的流式事件序列只在 v4-pro 残余通道上成立，且预计 2026-09-14 12:00 后不可用。**分段检索（设计 §3.5 后备：框架自有检索前端 + web_fetch）从"后备"升格为"必选"**；S2 机器合约必须同时覆盖两条路径（流式若回归则复用，分段为主）。
2. **运行面风险升级**：生产 `web_search`（`WebSearchClient` → `/responses` + web_search）当前处于"flash 路由静默忽略、v4-pro 残余可用"状态，模型侧零错误信号、照常编造来源——**比慢检索危险一个量级**。在 0ac 探针 `retrieval_family`（含 `web_search_call` 存在性）落地前，跑批结果里的检索引用不可信。
3. **用户"降智"体感的旁证**：社区同日有对 V4.1 Flash 质量的成规模负面反馈（Reddit r/LocalLLaMA「Is anyone else finding DeepSeek-V4-Flash unreliable?」、r/DeepSeek「DeepSeek Flash 4.1, the worst version of DeepSeek」、r/LocalLLM「brutal kill line」等；另有 GitHub `deepseek-ai/DeepSeek-V3` #1244 记录 V4 系工具调用走 plain text 的 ~11% 回退）；加上本次实测的检索工具静默失效，体感与外部证据一致。


## 6. S1′ 本地检索探针（2026-09-13 晚，用户裁决"全面转向本地检索"后的可行性实测）

> 用户裁决（2026-09-13）：服务端检索下架 ⇒ **全面转向本地检索**；pro 不用（贵）；检索后端切换作偏离登记；口径确认 = 不违反 TB 2.1 官方口径（agent 侧能力，装置四要素不动）。探针件 `probe_local_retrieval.py` + `local_retrieval_results.json`（同目录）。

### 6.1 P1：纯 HTTP 直取 SERP（12 技术查询 × 3 引擎，无浏览器、无 cookie、Chrome UA）

| 引擎 | HTTP 200 | 可解析 | 命中数均值 | TTFB p50 | 备注 |
|---|---|---|---|---|---|
| **DuckDuckGo html** | 12/12 | **12/12** | **9.8** | **11.0 s（冷）/ 5.8 s（热）** | `html.duckduckgo.com/html/` 静态 HTML，`uddg` 重定向可解码——**当前唯一开箱可用的主引擎** |
| Bing HTML | 12/12 | 0/12 → 可修复 | （9 个 `b_algo` 块在位） | 9.5 s | 首轮解析失败是探针正则过期（h2/a 属性序变了），**非 bot 墙**；换结构化提取即可用 |
| Bing RSS | — | 9 items | — | 10.3 s | 机器可解析但**多词技术查询相关性差**（近似只按首词匹配），不作主通道 |
| Google | 12/12 | 0/12 | — | 7.1 s | JS 墙，**无浏览器不可用**（与 0v 浏览器车道结论一致） |

### 6.2 P2：串行流水线（Bing 先 → DDG 兜底 → top-3 页面抓取）

- 首个 SERP 结果 p50 **21.0 s**（Bing 白等 ~9.5 s 后才轮到 DDG）；全流水线 p50 39.1 s / max 43.4 s；
- 页面抓取成功率 **30/36（83%）**（top-3 中平均 2.5 页可用，>500 字符正文）；
- 结论：**串行 + 错误先验引擎 = 把 10 s 首结果截止必炸**；可行形态是**并行竞速**（多引擎同时发起，取最先可解析者——min TTFB ≈ 5–7 s）+ 连接预热 + DDG 优先。

### 6.3 与 10 s 首结果截止的关系（诚实读数）

- 本机（= 跑批宿主机）到搜索引擎的网络路径偏慢且波动大（DDG 冷 11 s / 热 5.8 s；Bing 9.5 s；连接复用测试还遇到 connect 超时）；
- 10 s 截止若锚在**首字节/TTFB**：并行竞速下 p50 可达标（≈5–7 s）；若锚在**首个可解析 SERP**：冷路径下 p50 会越线，需要引擎竞速 + 预热 + 截止可配（设计 §3.4 的 `network_no_response` 语义保留，放宽开关保留）；
- **重要边界**：本探针在宿主机执行，未在评测容器内复测（容器走同一物理网络但 DNS/MTU 可能不同）——S3/S4 必须容器内复验后再定截止默认值。

### 6.4 定案输入（给 S2/S3）

1. **主路径 = 本地分段检索**：DDG html（主）+ Bing HTML（备，修提取器）+ Google（弃，需浏览器）；0v 的 SERP 语义资产（引擎链软备忘、`uddg`/`/ck/a` 重定向解码、低质量域名加权、`SERP_*_MAX_CHARS` 边界、引擎失败信封）**全部可复用**——其 CDP 依赖替换为纯 HTTP 抓取即可。
2. **首结果截止的实现形态**：多引擎并行竞速 + 首个可解析结果即返回 + 其余引擎取消；截止值默认 10 s、`ORZ_RETRIEVAL_DEADLINE_MS` 可配、A/B 记录。
3. 服务端 `web_search` 流式（§1 实测）留作历史证据与"若服务端恢复"的复用路径，不投入实现。

## 7. S1′ 更正（2026-09-13 晚，用户指出 DDG 早已排除 ⇒ 复核发现代理假象）

用户质疑「ddg 我不是明确排除了吗」⇒ 复核：**§6 的引擎结论被系统代理污染**——宿主机系统代理 `127.0.0.1:7890` 开启，Python urllib 默认走代理，探针读数是代理路径而非容器形态。直连/代理对照实测：

| 目标 | 走系统代理 | **直连（无代理 = 评测容器形态）** |
|---|---|---|
| Bing HTML | 200，9.6 s | **200，TTFB 0.4 s** |
| DDG html | 200，5.7 s | **不可达**（15 s 超时） |
| Google | 200，10.5 s | **不可达**（15 s 超时） |
| api.deepseek.com | 401（连通），5.3 s | 401（连通），0.17 s |

**更正结论**：
1. §6「DDG 主引擎」作废——DDG 直连不可达（与 0v 设计 §R4 实测「duckduckgo 本地不可达 20 s 失败」同源，正是当年引擎链事实排除 DDG 的原因）。
2. **评测容器形态下的本地检索引擎集 = Bing HTML 单引擎**（直连 0.4 s TTFB，远优于 10 s 截止；提取器需按现行 `b_algo` 结构重写）；DDG/Google 仅在显式代理配置下可用，容器无代理 ⇒ 不进默认引擎集。
3. §6「本机网络偏慢」的读数实为代理路径读数；直连下 Bing 通道毫无时间压力。
4. §6.4 第 1 条更正：主引擎 Bing HTML；竞速形态在单引擎直连下不需要，简化为「Bing 直连 + 10 s 每次尝试截止 + 重试」；用户同轮裁决：**截止按引擎单独计时（不共用一个钟），整体兜底 30 s，10 s 不够就升 30 s 兜底**。
5. 教训登记：凡涉及外网可达性的探针，必须显式声明并对照「代理/直连」两条路径（评测容器 = 直连），宿主机默认代理环境会系统性伪造可达性与时延。
