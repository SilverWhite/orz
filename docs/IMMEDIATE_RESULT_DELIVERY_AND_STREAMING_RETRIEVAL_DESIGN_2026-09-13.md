# 即时结果回报与流式检索设计（v0.1，2026-09-13）

> 状态：**设计稿，未实施**。本稿只做设计，不改代码、不改预算值、不改契约、不重建、不重跑，也不触碰 `D:\AGI`。
>
> 用户裁决（2026-09-13）：① 10 s = **检索请求发出后等“首个结果”** 的上限，不是检索任务总时限；总预算是另一本账；② 检索路径**先选流式**，不做实现，先完成设计；③ **D7 改掉，新增机制，明确的缺口必须补**，唯一约束= 不打断当前思维链（可选更极端：监控思维链文字，结果到达后在第一个句号处即时回报）；④ **FP-2 不改**，能力级不可达的如实汇报不是例外；⑤ 第 0 轮**全部重新跑**，本轮账面只作摩擦证据。
>
> 输入：[`框架时间预算语义审计 v1.4`](audits/FRAMEWORK_TIME_BUDGET_SEMANTICS_AUDIT_2026-09-13.md)；第 0 轮起跑记录 §6.1–§6.13；`MODEL_RESIDUAL_PRESSURE_DISCUSSION_2026-08-31.md`；`THIN_HARNESS_REDESIGN_V2_DESIGN_2026-08-28.md` §9.7；`COMMAND_TIMEOUT_AND_WEB_SEARCH_TIMEOUT_RESEARCH_2026-08-29.md`；`RETRIEVAL_SUBAGENT_DUAL_LANE_DESIGN_2026-09-09.md`。

## 1. 问题与一手证据

### 1.1 要修的两个缺口

1. **检索首个结果不可观测**：`web_search` 当前是非流式整包返回（`orz-tools/src/implementations/web_search/client.rs:222-232` 的 `response.bytes()`），健康时也要 15–70 s 才出结果；第 0 轮 n=116 条带 `wall_ms` 的完成记录 p50 = 29.9 s、96.6% ≥10 s。因此“请求发出后 10 s 内没有首个结果 → 判网络不通”目前**无法表达**：不是判据错，是通道不可观测。
2. **结果出现后仍要等**：后台任务完成、子代理已有部分结果、检索已返回一部分时，机械层没有“结果一出现就作为新的结果动作回报”的通用路径；D7 的“下一次工具结果带回”是 2026-08-28 为“不新增唤醒”而复用的既有机制，不是理想形态。

### 1.2 “工具一返回就报给模型”到底有多大压力

先分开三件常被混在一起的事：

| 情形 | 模型当时在做什么 | 是否算打断 | 框架已有证据/纪律 |
|---|---|---|---|
| **工具结果返回** | 模型已结束这一轮思考、发出 `tool_calls`，正等结果 | **不算打断**：模型本来就停在边界上等 | 这是现有主循环默认行为；每条工具结果立即回报，含失败/拒绝/空结果。R0 622 次工具调用全部按此形态回传 |
| **常驻/周期注入** | 模型每轮或长任务反复经历 | 构成持续注意力切换 | `MODEL_RESIDUAL_PRESSURE_DISCUSSION_2026-08-31.md` §2.2：12 类注入块仍注册，“每次注入 = 一次注意力切换 + token 消耗 + 偏离主任务风险”；`THIN_HARNESS_REDESIGN` 结论：摩擦=注意力稀释（工具面/提示词/注入块）+ 仪式往返，模型能力不是瓶颈 |
| **生成中插入** | 模型正在一条未结束的思维链/回答中间 | 才是结构性的打断问题 | 流式 API 不允许在**进行中的 assistant 消息**内插文本；框架对生成期的处理一直是“守住/止损”（复读守卫触发即打断并烧轮，`MODEL_RESIDUAL_PRESSURE` §2.4） |

**结论（回答“压力和影响大不大”）**：

- **“工具一返回就报给模型”压力很小**——那正是模型自己等待的合法边界，不构成打断，也不增加“常驻块”；这是框架现在就在做的事，应当继续并加强（失败/缺内容/空结果尤其要即时）。
- **真正贵的是“常驻/周期注入”和“无边界地在生成中插话”**。框架近两年的主方向是**减少常驻注入**（PUSH→PULL、prompt 近零、LIF fires 不注入、F6 push ≤3 次/run、机械审计“只报事实不给建议”），不是“禁止一切即时结果”。
- **“不打断当前思维链”应定位为优先偏好，不是绝对约束**：默认优先在合法边界（工具返回、轮结束）即时回报；当结果价值足够高（确定不可达、内容缺失、空结果、任务前提被推翻、模型正在等的后台任务完成），允许**在句号边界分段续写**把结果尽早交给模型；硬中断只留给高价值失效，并单独做成本/质量 A/B。
- **决策规则**：`即时注入当且仅当“避免的浪费（错误继续、空转、错误前提下的动作）> 打断成本（注意力切换 + token + 偏离 + 可能丢弃的半截生成）”`。缺内容/确定失败这类结果的“避免浪费”通常远大于成本；纯进度装饰则相反。

## 2. 结果投递策略（Result Delivery Policy）

### 2.1 边界定义

| 边界 | 含义 | 注入成本 |
|---|---|---|
| B1 工具结果边界 | 模型已发出 `tool_calls`，某条工具结果返回 | 最低（模型本就在等） |
| B2 轮结束边界 | 模型刚结束一轮（有/无工具调用） | 低（下一轮请求必然发生） |
| B3 生成中边界 | 模型正在流式生成思维链/回答，尚无工具调用 | 中—高（需要句号边界分段续写或硬中断） |
| B4 pending 工具等待 | 模型已发出 `tool_calls`，工具尚未返回 | 无合法注入点（协议要求先有工具结果）；只能等 B1，或让工具自身做中途回报 |

### 2.2 投递分级

| 级别 | 内容 | 投递时点 | 原有机制 |
|---|---|---|---|
| **I1 IMMEDIATE_MATERIAL** | 确定不可达（能力级/端点/凭据）、内容缺失/空结果、工具失败稳定码、任务前提被推翻的结果、模型正在等的后台任务完成 | 最早可用边界；若处于 B3 → 句号边界即时注入 | 工具结果已即时；后台完成/生成中注入需新增 |
| **I2 IMMEDIATE_PROGRESS** | 长任务的部分结果、检索已到的分段结果、可决策的进度读数 | B1/B2；必要时按“段”即时，不按“字节” | terminal 180 s 中间态先例；检索流式分段新增 |
| **I3 DEFERRED/DIGEST** | 装饰性进度、重复状态、无可决策价值的噪音 | 下一合法边界或摘要合并 | F6 ≤3/run、机械审计覆盖写 |

### 2.3 文案与去重纪律（承 FP-2 精神）

- 中性事实，不给建议、不教学、不做“勿重试”提示；
- 单事件自描述：稳定码 + cause + 已等待 ms + 结果摘要/指针；
- 同一结果只投递一次；去重键 =（来源类型, 任务/调用 id, 结果摘要或内容 sha256）；
- 投递必须有 journal 事件与模型可见载荷两处（可审计、可回放）；
- **不新增常驻块**：所有新增投递都是事件驱动、一次性、可去重的。

## 3. 流式检索设计（首选路径）

### 3.1 选型对比（用户 2026-09-13 已定：先做流式）

| 方案 | 改什么 | 首个结果 | 后续结果 | 代价 | 裁决 |
|---|---|---|---|---|---|
| **流式检索（选定）** | 结果怎么回来：一个请求、SSE/分块持续到达 | 首个事件/首段即可判活 | 每段立即转发 | 需后端暴露中间事件；SSE 解析/断流处理 | **先做** |
| 分段检索（后备） | 检索怎么拆：候选 → 逐页抓取 → 逐段抽取 | 第一段结果 | 每段完成即回传 | 需框架自有检索后端；多段编排与质量差异 | 流式不可行时启用 |
| 通道判活 + 操作预算（b） | 只改怎么判断通道死 | **不产生首个结果** | 仍整包返回 | 解决不了“分批到达却等整包” | 只作不可流式化操作的补充护栏 |

### 3.2 现状与目标

| 项 | 现状 | 目标 |
|---|---|---|
| 接口 | `POST /responses`，非流式，`response.bytes()` 整包读 | `POST /responses` + `stream: true`，SSE 逐事件读 |
| 首观测点 | 整包（p50 29.9 s） | **首个检索进度/结果事件 ≤10 s** |
| 结果交付 | 整包一次 | 每段结果/每页结果即时作为新的结果动作回报；不静默积压 |
| 总时限 | 客户端 120 s 整包超时 | `first_result_deadline` ≤10 s + `total_budget` 另计 + 无进度 stall 报告 |
| 失败 | 超时结构化错误 | 稳定码分类：`capability_unreachable` / `network_no_response` / `network_error` / `empty_result` |

### 3.3 事件模型（待 S1 实测确认）

候选 SSE 事件（DeepSeek `/responses` + `web_search`，实际名称/时序由 S1 探针确认）：

- 通道/生成级：`response.created`、`response.in_progress`、`response.completed`、`[DONE]`；
- 检索级（可能性最高）：`response.web_search_call.in_progress` / `searching` / `completed`，`response.output_item.added` / `.done`（`web_search_call` 项，含 `action.open_page` / `action.search`），`response.output_text.delta`（含 citation annotation）。

规则（设计目标，不依赖具体名称，S1 后落到具体枚举）：

1. **判活**：请求发出后 10 s 内出现任一“检索进度”事件（searching/in_progress/open_page 等）→ 通道判活；0 事件 → `network_no_response`，立即返回、不再等。
2. **首个结果**：出现首个结果项（open_page 完成的 URL / 结果文本段）→ 立即投递（I1/I2）。
3. **后续结果**：每个结果项/段到达即投递；同一 URL 去重；候选 cap 仍按激活计数。
4. **总预算**：判活后按检索任务总预算继续；无新事件超过 stall 窗口 → `no_progress` 事件（不是静默等满）。
5. **终态**：`completed` / `[DONE]` → 最终结果 + 汇总；失败 → 稳定码。

### 3.4 失败分类（稳定码）

| 码 | 触发 | 行为 |
|---|---|---|
| `capability_unreachable` | 端点/凭据/后端明确缺失（构造期或首个事件前明确判定） | 立即返回；如实汇报；不教学 |
| `network_no_response` | 请求发出后 10 s 内无任何检索进度事件 | 立即返回；本 run 不再等待该请求 |
| `network_error` | 连接/传输错误（DNS/拒绝/断流） | 立即返回真实类别 |
| `empty_result` | 通道判活但结果为空 | 立即返回（空结果本身是 I1 信息） |
| `no_progress` | 判活后超过 stall 窗口无新事件 | 回报当前已得结果 + 无进度事实；由总预算兜底 |

### 3.5 后备路径

- **S1 探针若证明流式不暴露可用中间事件**：转**分段检索**（框架自有检索前端：候选搜索 → 逐页抓取 → 逐段抽取；每段独立结果/重试）。
- **不可流式化的操作**（第三方黑盒、非 SSE 接口）：保留 b——10 s 通道判活 + 操作预算；明确登记“本操作不提供首个结果，只提供通道判活”。
- **不选**：把 120 s 直接改成 10 s（完成口径误杀）；保留 10 s 内零观测的现状。

### 3.6 官方口径边界

只改 agent 侧实现（检索客户端 + 事件 + 投递策略）；不改 harness 墙钟、不改题目、不改 verifier、不改数据集 pin；流式化前后做 A/B 记录（首个事件到达时间、结果段数、总耗时、失败分类、任务表现）。

## 4. D7 即时回报机制（后台完成 / 生成中结果）

### 4.1 机制分层

| 机制 | 适用 | 说明 |
|---|---|---|
| **M1 句号边界分段续写** | B3 生成中（无 pending 工具） | 监控 reasoning/text delta；结果到达置 `pending_injection`；到第一个句号边界**优雅停止当前流**，保存部分 assistant 内容（含 reasoning）；下一请求携带该部分内容 + 注入的机械结果事实，要求模型从句号后继续。思维链内容不被改写，请求被切成多段 |
| **M2 合法边界投递** | B1/B2；或存在 pending `tool_calls`（B4） | 工具结果/轮结束即投递；这是现有机制的加强版（不再依赖“下一次工具结果”这一特定点，任何合法边界都投） |
| **M3 长工具中途回报** | B4 且工具可中断/可读进度 | 以 terminal 180 s 中间态为样板：工具自身在边界返回一次“运行 + 自身情况”，模型无需等到完成 |

### 4.2 必须处理的边界

- **pending 工具窗口**：若模型已发出 `tool_calls`，在任一工具结果返回前没有合法续写点；“不打断当前思维链”在此窗口内只能做到“工具结果一返回就投递”，或让工具自身做 M3。
- **句号判定**：默认 `。`/`.`/`!`/`?` + 换行或空白；中文/英文/代码块三类形态需分别处理（代码块内句号不触发）；阈值/开关可配（A/B）。
- **reasoning_content 回传**：DeepSeek 对带 reasoning 的 assistant turn 有回传要求；分段续写必须验证“部分 assistant + reasoning + 注入事实”的继续生成是否被接受、是否产生重复/退化。
- **去重与抑制**：当前 `consumed_completion_ids` 会在模型自己读取产物时把任务标记为已报告（TB40 §10.1 实测轮询型 agent 注入 0 次）。新机制必须区分“模型自己读到了”与“框架投递了”；框架投递成功也要有独立标记，避免“投了但被抑制”。
- **事件与审计**：每次投递落一条自描述事件（来源、结果摘要/指针、投递边界类型 B1–B4、是否分段续写、稳定码）；保持 run 终态/工具调用配对完整。

### 4.3 验收判据

1. B1/B2：结果到达后立即投递（无额外等待）；
2. B3：结果到达后 ≤ 一个句号边界 + 小延迟（目标 p95 ≤ 1 个句子/≤2 s，实测校准）完成投递；
3. 投递不产生：重复工具执行、assistant 消息/工具调用配对破损、reasoning 丢失、run 终态缺失；
4. 轮询型 agent 不再出现“注入 0 次”的抑制；同一结果只投一次；
5. A/B：投递 vs 不投递在任务表现、token、轮数、退化事件上的差异可核对。

## 5. 机器合约与事件面（S0，先行）

按仓库纪律，**机器合约先行**（schema/verifier/fixtures 先于实现）：

| 契约 | 内容 |
|---|---|
| 检索进度/结果事件 | 新增（或扩展 `tool_running`）`retrieval_progress` / `retrieval_result`：稳定码、阶段、已等待 ms、结果摘要/指针、是否分段、去重键 |
| 投递事件 | `result_delivered`：来源（后台任务/子代理/检索段）、边界类型 B1–B4、投递方式（直接/句号续写）、是否被抑制 |
| `tool_completed` 失败载荷 | 补 cause（单事件自描述），不再只给 `browser_launch_failed` 这类壳码 |
| 探针 | `tool_availability_check` 扩 `retrieval_family`（浏览器/搜索引擎/web 通道），run 起始一次 |
| 稳定码 | 上面 §3.4 五码 + 投递相关码；进入 assurance 家族与法官 |

## 6. 阶段（S1–S4；本轮只做 S1/S2 的设计门）

| 阶段 | 内容 | 产出 |
|---|---|---|
| **S1 探针** | ① 流式 `/responses` + `web_search`：记录 SSE 事件序列/时间戳，确认 10 s 内是否有可用首事件；② 分段续写探针：部分 assistant + reasoning + 注入事实，验证继续生成、无重复、无配对破损 | 探针记录（不写生产代码）；用于确认流式或转分段 |
| **S2 机器合约** | schema/verifier/fixtures 先行；事件/稳定码/投递契约定稿 | S0 契约 diff + 放行签名 |
| **S3 实现** | 流式检索客户端、检索进度/结果事件、投递策略（I1–I3）、D7 机制（M1–M3）、子代理提前收口、semaphore acquire 截止、检索族探针、cause 自描述；全部带开关 + A/B | 代码 + 单测 + 集成测试 + A/B 记录 |
| **S4 复验** | 小任务集实机复验（不评正式分）；通过后按新载体整轮重跑 89 题 | 实机记录；第 0 轮全部重跑 |

## 7. 风险与开放项

1. **DeepSeek 流式是否暴露中间检索事件未知** → S1 探针定性；不暴露则转分段检索。
2. **分段续写的质量/成本**：额外请求、缓存 miss、半截生成丢弃（仅到句号前）、可能的复读/退化；必须 A/B，必要时只对 I1 使用 M1。
3. **注入文案被当成新指令**：统一中性事实框、不给建议；与机械审计层同纪律。
4. **句子边界在代码块/公式/URL 内**：需要白名单/状态机，避免在代码中间切断。
5. **pending 工具窗口**：M1 不可用，只能 M2/M3；要让模型知道“结果会在工具返回时一并到达”。
6. **子代理墙钟**：确定不可达/连续失败要提前收口，不能把 240/600/900 s 当等待预算（审计 D1）。
7. **不新增常驻块**：所有投递事件驱动、可去重、可审计；F6 push 的次数纪律不被放大。
8. **官方口径不变**：流式化只改 agent 侧；A/B 记录必须保留。

## 8. 本轮不做项

- 不实施任何代码；不改预算值；不改 FP-2；不移除车道；不新增容器内浏览器；不重建；不重跑；不触碰 `D:\AGI`。
- 设计完成的门 = S1 探针记录 + S2 机器合约 + 本稿的验收判据全部可机械核对。

> 账本入口（2026-09-13 待办入账轮同步）：BACKLOG **0ac** `GAP-MECH-IMMEDIATE-FEEDBACK`（**29 → 30**）/ TODO `P0-0ac` / 索引 `GAP-MECH-IMMEDIATE-FEEDBACK`（§6 路由）+ `DESIGN-IMMEDIATE-RESULT-STREAMING-RETRIEVAL`（§3.1 路由）。S1 探针为第一门，**待用户放行**（要发一次真实 API 调用）。

---

## 9. 修订：检索路径改定本地分段检索（2026-09-13 晚，用户裁决）

> 触发 = S1 探针深挖发现 **DeepSeek 已将 Responses API 服务端 web_search 下架**（官方文档明载"内置工具忽略"；flash 路由 4/4 静默失绑；v4-pro 残余通道 2026-09-14 12:00 起随路由切换预计关闭）。证据与实测：[`0AC_S1_PROBE_RECORD_2026-09-13` §5/§6](audits/0AC_S1_PROBE_RECORD_2026-09-13.md)。

1. **路径改定**：§3.1 的"流式检索（选定）"降级为**历史证据 + 若服务端检索恢复则复用**；**主路径 = §3.5 的分段检索（本地）**——框架自有检索前端：SERP 引擎链 → 逐页抓取 → 逐段抽取；每段独立结果/重试/截止。0v 的 SERP 语义资产全部复用，CDP 依赖替换为纯 HTTP。（引擎集二次修订见第 4 条。）
2. **用户裁决原文**（2026-09-13）：「不违反（官方口径）就行，直接登记吧，大不了只做本地，好好优化一下就行」「我们关键的就是衡量 orz，进行跑分只是为了完善 orz 和给出一个可供参考的框架能力」「pro 太贵了不可以用的」。
3. **偏离登记（检索后端）**：web_search 服务端检索（`/responses` + web_search 工具，orz `WebSearchClient`）→ **本地分段检索**；原因 = 服务端下架（非我方选择）；影响面 = agent 侧检索能力，**TB 2.1 官方口径四要素（harness / 数据集 pin / verifier / 墙钟）不动，不构成口径违反**；可比性注记 = 与官方 90.6 参照线本就非逐项同构（v3.03 已登记），此偏离入分批偏离台账并在 A/B 记录中留「服务端（历史）/本地」对照。
4. **首结果截止在本地路径的实现形态（2026-09-13 晚二次修订，S1′ 代理假象更正）**：宿主机系统代理（`127.0.0.1:7890`）污染了首测读数——**直连（= 评测容器形态）下 Bing HTML TTFB 0.4 s**；DDG/Google **直连不可达**（与 0v R4「duckduckgo 本地不可达 20 s 失败」同源，正是引擎链事实排除 DDG 的原因，用户口径获得实证）⇒ **默认引擎集 = Bing HTML 直连单引擎**（提取器按现行 `b_algo` 结构重写）；DDG/Google 仅在显式代理配置下可用，容器无代理不进默认集。**截止按引擎单独计时（不共用一个钟），整体兜底 30 s**（用户裁决原文：「10s不够就提升到30s兜底，每个搜索引擎单独计时，不一起计时」）；`ORZ_RETRIEVAL_DEADLINE_MS` 可配 + A/B 记录保留；默认值仍待容器内复验后定。更正详见探针记录 §7。
5. §5/§6 阶段表相应修订：S2 机器合约按**双路径覆盖**（本地分段为主、流式为恢复预留）；S3 实现清单中"流式检索客户端"替换为"本地分段检索前端（SERP 竞速 + 页面抓取 + 段抽取）"，其余（探针扩面、稳定码、投递策略、M1–M3、semaphore 截止）不变。
6. **引擎选择与工具面（2026-09-13 晚，用户裁决）**：① **cn.bing.com（CN Bing）为无代理形态的默认引擎**（直连稳定可达；`www.bing.com` 直连实测 TTFB 0.4 s 同族，具体 cn 域名以容器内复验为准）；② **有代理配置时引擎交由模型自选**（接 0v 设计 §6 留存的 `engine ∈ {auto,google,bing,duckduckgo}` 引擎自选方案——当时"只留文档"，此处升格为代理形态下的实施路径）；③ **工具面保留 `web_search` 名称不变**（8 工具面冻结不破、模型提示词与调用习惯零迁移），实现明确改指本地分段检索：`web_search` = 本地 SERP（引擎按上两条选路）+ 逐页抓取 + 段抽取；模型可见描述如实标注数据来源形态（本地检索，非服务端）。

---

## 10. S2 机器合约落档（2026-09-13 晚；用户放行「直接进行」）

> 放行记录：用户 2026-09-13 指令「当前需要处理的是 0ac S2 部分，请直接进行」= **S2（机器合约）放行**；S1/S1′ 探针记录已入档（[`0AC_S1_PROBE_RECORD_2026-09-13`](audits/0AC_S1_PROBE_RECORD_2026-09-13.md)）。本节的判据：§5 的五项契约逐条落到可机械核对的 schema / fixture / 门禁映射；**S2 只定契约，不写生产代码**（S3 才实现生产者与法官规则）。

### 10.1 契约 diff（§5 五项 → 落档产物）

| §5 契约 | 落档产物（新增/扩展） | 覆盖 |
|---|---|---|
| 检索进度/结果事件 | **新增** `retrieval_progress`（`runtime/retrieval-progress-event-payload-v0.2.schema.json`，slug `retrieval-progress`）：`retrieval_path`（`local_segmented`主路径 / `server_streaming`恢复预留）、`stage`（dispatched/channel_alive/progress/no_progress/failed/finished）、`waited_ms`、`since_last_event_ms`、`deadline_ms`、**稳定码**、`result_count`、`dedupe_key` | 判活（channel_alive）/无进度（no_progress）/失败（failed）+ 双路径 |
| 检索结果（到达面） | **新增** `retrieval_result_segment`（`runtime/retrieval-result-segment-event-payload-v0.2.schema.json`，slug `retrieval-result-segment`）：`segment_index`、`is_partial`、`segment_count_hint`、`waited_ms`、`dedupe_key`、`result_summary{visibility, source_url, content_sha256, observed_scope, byte_len}` | 每个结果项/段到达即入账（规则 3）；与既有 `retrieval_result_committed`（slug `retrieval-result`，账本提交/可见性面）**分立**，到达面 ≠ 提交面 |
| 投递事件 | **新增** `result_delivered`（`runtime/result-delivered-event-payload-v0.2.schema.json`，slug `result-delivered`）：`result_source`（background_task/subagent_result/retrieval_segment/retrieval_progress/tool_result）、`boundary` B1–B3、`delivery_mode`（direct/sentence_resume/digest）、`delivery_class` I1–I3、`suppressed` + `suppressed_reason`（duplicate/model_read_directly/class_capped）、`latency_ms`、`dedupe_key`、`delivered_at` | §2.2 分级 + §4.2 边界/去重/抑制：**框架投递与「模型自己读到」分开记账** |
| `tool_completed` 失败载荷补 cause | **扩展** `runtime/tool-completed-event-payload-v0.2.schema.json`：新增可选 `cause`（真实类别，单事件自描述）；**壳码集合 `{browser_launch_failed, tool_failed, failed, error, unknown_error}` 由 schema `not.enum` 机械拒绝** | 不再只给壳码；cause 与 `failure_target` 并行（identity 与 cause 两面） |
| 探针扩 `retrieval_family` | **扩展** `runtime/tool-availability-check-event-payload-v0.2.schema.json`：`probe_scope` 由 `const` 放开为 `enum{main_agent_work_tools, retrieval_family}`；新增 `retrieval_family{browser, search_engine, web_channel}`，每类 `present` + `present=false ⇒ reason` / `present=true` 可带 `detail`（如实汇报，FP-2 精神） | run 起始一次三类硬设施在位读数；与 23 工具面探针各自成事件 |
| 稳定码进 assurance 家族 | 五码 `capability_unreachable` / `network_no_response` / `network_error` / `empty_result` / `no_progress` 已闭枚举进 schema（`retrieval_progress.stable_code`）；投递侧抑制码进 `result_delivered.suppressed_reason` | 枚举层面封闭；**法官族规则随 S3 实现**（见 10.3） |

### 10.2 机器核对证据（本批实测）

- 注册表 `runtime/run-event-payload-registry-v0.1.json` v02 轨新增 3 条（`retrieval_progress` / `retrieval_result_segment` / `result_delivered`），Python 视图（`assurance/run_event_journal_validation.py` 派生）与 Rust 法官（`orz-assurance .../journal/conformance.rs` 直读）自动跟进，**无镜像改动**。
- 信封枚举 `runtime/run-event-v0.2.schema.json#/properties/event_type` 新增同名三型。
- fixture：三个 slug 的 `minimal.valid` + `constraint.invalid` 自动派生；另登记 7 个契约锁（`retrieval-progress.no-progress.valid`、`retrieval-result-segment.partial.valid`、`result-delivered.suppressed.valid`、`tool-completed.cause.valid`、`tool-completed.cause-shellcode.constraint.invalid`、`tool-availability-check.retrieval-family.valid`、`tool-availability-check.retrieval-family-missing.constraint.invalid`）+ 3 个信封正例。
- 门禁：`python scripts/check_repository.py` → `error_count: 0`；`run_event_v02_payload_positive_contracts: 67`、`run_event_v02_payload_negative_contracts: 49`、`run_event_v02_envelope_positive_contracts: 66`。

### 10.3 明确留给 S3（契约已定、实现未做）

1. **生产者**：三个新事件的实际写点（本地分段检索前端 / 投递策略 I1–I3 / M1–M3 / 探针扩面）与开关、A/B 记录；全部带开关，默认关闭直至复验。
2. **法官规则**：① 每个 `call_id` 的 `dedupe_key` 唯一性（同一事实只入账一次）；② `result_delivered` 的 `dedupe_key` 与真实投递一一对应（不许"投了但被抑制"混记）；③ `retrieval_family` 探针 run 起始一次（不多不少）；④ `cause` 与 `failure_target` 的失败形状一致性；⑤ 首个结果 `wall_ms ≤ deadline_ms` 的判据族（索引 0ac 判据 ①）。
3. **回归钉子**：「及时且有信息量」的框架契约随 S3 落进机械审查层与回归集。

### 10.4 判据现状

索引 0ac 判据 ①（检索类首个结果 `wall_ms` p99 ≤ 10 s，本地路径按 §9.4 引擎单独计时 + 30 s 兜底）与 ②（`subagent_wallclock_timeout_mid_tool` = 0）**均为 S3/S4 实测判据**，S2 不作数值结论；S2 的完成门 = 上文 10.1/10.2 全部可机械核对。

### 10.5 回写：S3 落地后的口径终态（2026-09-14 追加；**不改契约**）

> 来源：0ac S3 实现审记 §2.3「口径注记 A（文档口径滞后，P3）」——本节是它的回写面。回写只钉**口径与状态**，不改 10.1/10.2 的契约产物。

1. **法官规则③（上文 10.3-2③）的终态 = 宽口径**：字面「`retrieval_family` 探针 run 起始一次（**不多不少**）」按 F-007 裁决(a) 落地为「**present ⇒ 校验（不多于一次/位置/读数完整）；absent 不判**」（历史 17 份语料不误判）。S3-a 首版以开关 `REQUIRE_RETRIEVAL_FAMILY_PROBE=false` 兜住；`4c892951` 已**移除该开关及其执法分支**（实现面无开关，全 crates 仅存 2 处文档注释 `…/journal/immediate_feedback.rs:41/:50`）⇒ **本节口径以「存在即校验」为终态**，等待生产者落地后新 run 均带探针，实际执法面即等于该口径。
2. **口径注记 B 回写（上文 10.3-1 的例外）**：生产者面的**探针事件本身无开关**——run 起始无条件 +1 条 `tool_availability_check`（`probe_scope=retrieval_family`，`gate_decision` 恒 `pass`，读数事件无阻断语义）；「全部带开关」适用于其余生产者件（本地分段检索前端 / 投递侧）。
3. **生产者面状态（截至 2026-09-14）**：三事件写点（`retrieval_progress` / `retrieval_result_segment` / `result_delivered`）与 I1–I3 / M1–M3 **仍无产品码写点**（全 crates 扫描仅 `orz-assurance` 法官读取面命中）⇒ §10.3-1 未闭合（台账 F-017）；本地分段检索前端与 `cause` 自描述已落（`ORZ_WEB_SEARCH_LOCAL` 默认关）。
4. **回归钉子状态**：法官五族已注册、Python 镜像在位；本轮修复后 `orz-tools --lib` 2869/0/6、`orz-loop --lib` 771/0/3、`orz-assurance --lib` 226/0/0（修复详情：[`0AC_S3_FIX_REPORT_2026-09-14`](audits/0AC_S3_FIX_REPORT_2026-09-14.md)）。
5. **0ac 状态**：**仍 open**（G3 未闭）；S3 不得按「已闭合」读（与审记 §6/§8 同口径）。

