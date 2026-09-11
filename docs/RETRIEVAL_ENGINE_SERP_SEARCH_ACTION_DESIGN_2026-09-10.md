# 检索引擎 SERP 接入与 `browser_control` 车道分类修正设计（2026-09-10）

> 来源：0u R4 实机证据（[`OFFICIAL_R4_UNSOLVED15_RERUN_2026-09-10`](audits/OFFICIAL_R4_UNSOLVED15_RERUN_2026-09-10.md)
> §5A）+ 2026-09-10 用户裁决（无代理环境下后端检索超时、Google 不可用时，
> 满足模型搜索引擎需求——按既有 SERP 设计补实现，候选 Bing）。
> 设计基线：[`RETRIEVAL_AND_SUBAGENT_COMMUNICATION_BORROW_RESEARCH_2026-08-30`](RETRIEVAL_AND_SUBAGENT_COMMUNICATION_BORROW_RESEARCH_2026-08-30.md)
> §8.1/§8.3（引擎链裁决 + 机械防护清单）、索引 `FUS-RETRIEVAL-ENGINE-SERP`
> （current-design；0t 后模式语义退役、引擎链与 pacing 保留）、ADR-0010
> §14.65（0t 双车道）。状态：**S1–S3′ 完成（S3 随 0x S3 同批重建进载体；S3′ 载体重建
> 0.4.1 → 0.4.2 见 [0v S4 复跑重建记录](audits/0V_S4_REFRESH_REBUILD_2026-09-11.md)，2026-09-11）；
> **S4 复跑（2026-09-12）已完成：F1/F2 实机确认修复**（`browser_control` 8/8 执行、
> `browser_launch_result=success`），**但 0v 本体仍未取得**——判据 6 ✓、判据 7 机制面 ✓、
> 判据 2 改善至 1、判据 4 命中率 94.36% ✓，判据 1 仅「部分」、5/9/10 无样本；
> **F3**：抽取竞态已排除（load 时点 Bing SERP 已有 `li.b_algo`×10）、通用出口正常，
> 形态与「会话级失败备忘」一致；**缺口**：`engine_attempts`/`error_class`/`low_quality`
> 无持久化面 → 判据 1/5/7 不可事后取证（待裁决，见
> [0v S4 复跑记录](audits/0V_S4_RERUN_2026-09-12.md) §6）；
> **2026-09-12 用户裁决**：采纳 §6 所列备选「**软备忘＝只调整顺序**」为当前方向
> （**设计定稿、待实施**，见 §8.1–§8.5）；§6 的「完全去备忘」被否（本环境下
> 其代价为**每次**付 Google 超时）。同时登记两项取证待办 **0v-A 引擎级取证面**
> 与 **0v-B 定向探针**（§8.6）。
> S4 实机复验未通过——受阻于两项**：**F1 权限门「双面修一面」**（`risk_class` 已改
> ReadOnly，但宿主 `orz-host/src/permission.rs::access_kind` 未补 `browser_control`
> 映射 → 落 `Edit` 兜底 → 无头模式下确定性拒；实测 3/3 次 `browser_control search`
> 被 `permission_decision=deny`）、**F2 装置侧容器无可用浏览器**（Chromium 引导
> 600s 超时 + PATH 回落命中 snap 桩）。**两项已按用户裁决处理（2026-09-11）**：
> **F1 修复落码**（orz `340fe4a7`：`access_kind` 增 `browser_control` 按 action 分档
> 映射——七种现行动作 → `Read(None)`，未知/Phase 2 交互动作 → `Edit` fail-closed；
> 同刀补跨表护栏测试 `read_only_tools_never_fall_into_the_edit_bucket`）；
> **F2 改造为宿主侧供给真实 Chromium**（一次性取官方快照 rev `1696156` 落
> `D:\tb-eval\browser\chrome-linux\`，跑批只读挂 `/opt/chrome-linux`，容器内实测
> `Chromium 155.0.8053.0` 启动正常）。判据 1/5/6/7 待**载体已完成重建、复跑放行后**取证；
> 判据 2 部分成立、判据 4 部分成立。详见
> [0X/0V S4 实机复验记录](audits/0X_0V_S4_LIVE_VERIFICATION_2026-09-11.md)
> （[0X S3 重建记录](audits/0X_S3_DUAL_PLATFORM_REBUILD_2026-09-11.md)）；
> 2026-09-10 两轮全面复审后的修复均已落地（首轮 P1-1/P2-1/P2-2/P2-3 +
> P2-4 方案 A，见 §5.3；P2-4 专项轮的 P1-1/P2-1/P2-2/P2-3/P2-6/P3，见
> §5.4）；引擎自选与去备忘为**设计留存、未实施**（§6）。

## 0. 用户裁决（2026-09-10 登记）

1. `browser_control` navigate 在 external 检索车道被拒（`retrieval_role_write_denied`）：
   模型在无代理 + 后端检索超时场景下有「导航到搜索引擎」的真实需求，**应满足**。
2. 昨晚 web_search 超时必须**坐实**为 DeepSeek 后端检索问题（已完成，见 §2，
   证据三柱闭合）。
3. **增加一个国内可用的搜索引擎**：检索层已有设计（引擎 SERP 链）未实现，
   候选 Bing——按设计补实现。
4. **区域固定 en-US**：Bing/Google/DDG 搜索 URL 统一使用 en-US 市场参数，
   减少区域化污染；参数由机械层固定，模型不可见。
5. **Bing 必须专项优化**：无登录态 Bing 的广告/推广/CAPTCHA/重定向污染
   必须在本期机械层解决。
6. **补上低质量域名加权**：复用既有 `SourceWeightConfig` 判定器，对 SERP
   结果做低质量标注与稳定排序；不硬过滤，不承担恶意域识别。

## 1. 背景与证据

### 1.1 R4 实机暴露的两个缺口

- **分类缺口**：`risk_class("browser_control")` 落入兜底分支 = `LocalMutation`
  （orz-loop `tool.rs`），检索车道写门整工具拒绝（`retrieval_role_write_denied`）；
  2026-08-10 起同一豁免名单已有 `browser_read` 先例（会话浏览器导航 + 取文 =
  纯读），0t P1-2b 新增 browser_control 时未同步。连带动面：权限门按
  LocalMutation 询问/拒绝、黑板动作区折叠为 "edit"、预变更快照误触发面。
- **搜索入口缺口**：external 车道双族里没有搜索引擎能力——模型只能自己猜
  搜索页 URL（R4 实测猜 duckduckgo，本地不可达 20s 失败）；引擎 SERP 链
  （Google → Bing → DDG）在调研 §8 定案但从未落码（`local_browser/` 仅
  cdp/discovery/url_gate，无 SERP 模块）。

### 1.2 DeepSeek 后端检索慢——证据闭合（调查结论，无实施件）

三柱证据（详见 [R4 审计 §5A](audits/OFFICIAL_R4_UNSOLVED15_RERUN_2026-09-10.md)
+ 本设计 §2 调查记录）：

| 证据 | 数据 | 指向 |
|---|---|---|
| 历史基线（8 月，320 卷 74 次调用） | 成功中位 **46.9s**、p90 56.3s、max 94.6s；超时率 **1/74** | 该通道固有慢（服务端搜索+合成）；预算余量本就仅 ~1.3×max |
| R4 当晚（15 题 22 次调用） | 成功 **46–79s（与基线同分布）**；超时 **7 次（32%）** | 变的不是速度，是尾延迟穿越 120s 预算的比率 |
| 同夜对照 | 主模型流式（同 api.deepseek.com、同 transport）整夜健康，零 stall | 本地链路无恙；慢段定位在服务侧搜索路径 |
| 次日活体对照（r4b，同无代理环境） | 15 次成功 **21.6–38.1s**、零超时 | 非永久性问题——服务端尾毛刺 |

**结论**：昨晚超时 = DeepSeek 服务端搜索尾延迟毛刺（服务侧），非本地代理、
非 orz 机制。附记：120s 客户端预算（0j W4-R4 S5-1 常量）对该通道头部余量
薄，尾毛刺极易打穿——预算值可调（如 180s）属参数观察项，**不随本设计实施**
（FP-3 裁决「节点用户自理、不作 orz 处置」不重开；是否调预算待用户单独裁决）。

## 2. 目标与非目标

**目标**：无代理环境（Google 不可用）+ 后端检索超时场景下，模型在
external 车道拥有可用的搜索引擎能力（Bing 为有效主引擎），引擎链、
防护与失败态按既有设计；Bing 的无登录态污染在本期做专项机械治理。

**非目标**：
- 不加新工具（8 工具面冻结）——能力以 `browser_control` 新动作承载。
- 不做跨引擎 RRF 共识（§8.3 第 2 项清单另批；本期单引擎顺序链 + 失败
  备忘）。
- 域名加权仅做低质量来源的机械标注与稳定排序，不硬过滤；不承担恶意域
  检测。恶意域/钓鱼/银狐类内容识别不属于来源质量层，后续若要处理须独立
  立项为威胁情报能力。
- 不做 Google 键入模拟人化（§8.1 pacing 的键入模拟部分缓期；本期仅冷却 +
  会话上限）。
- 不新增事件面（ToolCompleted 即事实）；不加动态健康字段；Phase 2 交互
  动作（click/type/eval）维持不行动。
- 不动 web_search 的 120s 预算与 DeepSeek 通道。

## 3. 设计

### 3.1 `browser_control` ReadOnly 分类修正

- `orz-loop/src/tool.rs` `risk_class`：豁免名单增加 `browser_control`
  （与 `browser_read` 同注释先例：会话浏览器导航/状态观测 = 纯读，无
  worktree/宿主副作用；可达性由检索启用门管，每次导航 URL gate 照过）。
- 一处修正同时修复四个面：检索车道写门（R4 观察的拒绝消失）、ReadOnly
  策略权限门、黑板动作区折叠（"edit" → "read"）、预变更快照误触发面。
- Phase 2 交互动作（click/type/eval）如未来立项，须重审分类（届时不再纯读）。
- 测试：`risk_class("browser_control") == ReadOnly`；
  `ToolFilter::Retrieval.write_gate("browser_control") == None`。

### 3.2 `browser_control` `search` 动作（引擎 SERP 链）

**接口**（`BrowserControlAction::Search { query }`；工具 def action 枚举追加
`"search"`、新增 `query` 参数、`url` 对 search 拒绝）：

- 引擎链（调研 §8.1 保底顺序，常量）：**Google → Bing → DuckDuckGo**。
  机械层不暴露引擎切换；某引擎失败后同会话备忘，后续 search 跳过该引擎。
- SERP URL（全部过既有 URL gate；`query` 百分号编码）：
  - Google：`https://www.google.com/search?q=<query>&hl=en-US&gl=us`
  - Bing：`https://www.bing.com/search?q=<query>&setmkt=en-US&count=10&first=1`
  - DDG：`https://html.duckduckgo.com/html/?q=<query>&kl=us-en`
- **会话级失败备忘**（机械内存，模型不可见）：网络级失败（dns/
  connection_reset/timeout）、CAPTCHA/consent、空结果三类均记入会话备忘，
  同会话后续 search 跳到队尾——无代理环境的 Google 首次失败后不再逐次付
  超时成本；状态存 `CdpBrowserSession`（会话生命周期，随会话销毁）。
- **真实浏览器执行**：`search` 只能通过现有 `CdpBrowserSession` 执行，
  不得另起 HTTP/requests 抓取。TLS 指纹与页面执行保持真实 Chromium 语义。

#### 3.2.1 Bing 专项解析（无登录态污染治理）

固定选择器以“只取有机结果”为原则，显式排除广告：

- 结果容器：`#b_results > li.b_algo`
- 显式排除：`li.b_ad`
- 标题主选择器：`h2 a`
- 标题兜底：`h2`、`a[aria-label]`
- 摘要主选择器：`div.b_caption p`
- 摘要兜底：`div.b_caption div`、`p.b_algoSlug`、`.b_caption .ipText`
- CAPTCHA/consent 检测：`div.captcha`、`div.captcha_header`

Bing 结果 URL 若为 `/ck/a?...&u=...` 或同类跳转地址，必须在 `serp.rs`
纯函数中解析 `u` 参数并百分号解码后再进入结果；不把 Bing 跳转链接直接
交给模型。DDG 的 `uddg` 跳转参数同规则解码。

**解码保真（2026-09-10 复审修复，P1-1）**：实测 Bing 实响应（10/10 条
有机结果）`u` 值一律是 `a1<base64url>`——2 字符标记 + base64 体；把整串
直接当 base64 解码只会得到移位乱码或 UTF-8 失败。解码顺序固定为
「原值 → 去 `a1` 标记」，并逐候选做 URL 校验：只有能解析为 http(s) 绝对
URL（或引擎源下的 protocol-relative／绝对路径）才算解码成功，否则回落
原始链接。DDG `uddg` 同规则校验。**绝不把「解码出的任意文本」当 URL
交给模型**（旧实现在无标记 fixture 上通过、对真实链接 100% 失效，且连带
使 §3.2.4 的域名加权失去域名可判性）。

#### 3.2.2 无头环境适配（Bing 反污染前置）

`CdpBrowserSession` 的 search 路径需满足以下最小集合；实施前先审计现有
CDP 会话是否已经覆盖，未覆盖者补齐：

- **启动层（浏览器自身生效）**：`--disable-blink-features=
  AutomationControlled` 抑制自动化标记位；headless `--window-size=
  1280,800` 保证窗口非退化。
- **会话层（一次性、尽力而为）**：`Emulation.setDeviceMetricsOverride`
  固定视口与屏幕指标（headless 无真实屏幕，`--window-size` 只覆盖
  inner/outer，`screen.*` 仍需 emulation）；UA 若含 `HeadlessChrome`
  标记，则用浏览器**自身** UA 改写成 `Chrome`（保留平台与版本；对
  Chromium/Edge 分支不做跨浏览器伪装）。
- 加固失败只落 `[browser_log]` 特征，**不得连带失败** navigate/search
  （P2-3：加固不是动作本体，且不应把一个搜索引擎的加固前置变成整个控制
  面的硬前置）。

**实测基线（2026-09-10，headless Chrome 153）**：`window.chrome` 已存在
（无需注入存根；`Page.addScriptToEvaluateOnNewDocument` 已从 CDP 白名单
移除）；`navigator.webdriver` 在纯 `--remote-debugging-port` 启动下即为
false（只有 `--enable-automation` 启动会置真，故该启动参数保留为未来
`--enable-automation` 的兜底）；UA 仍带 `HeadlessChrome` 标记，UA 覆写
保留。

#### 3.2.3 通用 SERP 提取、信封与 pacing

- **SERP 提取**：宿主自有固定表达式（CDP `Runtime.evaluate` 既有白名单
  纪律，模型永不供表达式）按引擎取有机结果。Google 保留 `#search a h3`
  锚点；DDG 使用 `a.result__a` + `.result__snippet`。提取 0 条、CAPTCHA/
  consent 或网络失败均按该引擎失败处理并切下一引擎。
- **结构化信封**（有界）：`{action:"search", action_status, engine,
  engine_attempts:[{engine,status,error_class?,reason?} ≤3], results:
  [{title,url,snippet,tier,weight,reason} ≤10], error_class?, nav_phase,
  log}`。全部引擎失败 = `action_status:error` +
  `error_class:"all_engines_failed"` + 每引擎失败类摘要入 log（显式、无
  静默混用，FP-2 纪律）。
- **引擎标注（2026-09-10 复审修复，P2-1）**：`engine_attempts` 在成功与
  失败信封都携带，失败时必须能看出**是哪个引擎、以何类别失败**；被会话
  备忘跳过的引擎以 `status:"skipped"` + 备忘类别显式列出，不静默消失
  （旧实现在「三引擎全部已备忘」时只回一句 all_engines_failed，模型无从
  判断该换引擎还是放弃）。
- **字段上限（2026-09-10 复审修复，P2-2）**：title/snippet ≤200 字符、
  url ≤2048 字符、reason ≤200 字符；总载荷由字段上限机械界定（典型
  ≤8KiB、上界约 26KiB），**不再做按字节截断**。理由：真实 SERP href 远
  低于该上限，而按字节丢尾部结果既不可预测、又会把「引擎可控文本」的
  唯一无界入口留着；字段上限是挡住它、同时保持 10 条结果完整的那一层。
- **机械 pacing**（调研 §8.3/索引 FUS-RETRIEVAL-ENGINE-SERP「pacing/频率
  上限保留」）：同会话两次 search 最小冷却 5s，并在 5s 基础上叠加 0–50%
  随机抖动（会话首次立即执行）；浏览器会话物理兜底上限 = **40 次引擎
  导航**（P2-4 起计数单位从 search 调用改为导航——一次 `auto` 调用最多
  打三个引擎），超限显式稳定码
  `browser_control_search_cap_exceeded`（与 web_fetch 候选 cap 同为显式
  稳定码；载体不同——这里是 search 动作内的状态化失败，web_fetch 那条是
  派发前拒绝，S4 判据按本文件的载体读）。超限发生在任何引擎被触达之前，
  属动作级失败，不带 `engine_attempts`（无引擎参与）。
  **判定口径（2026-09-10 复审 P2-1 裁定：检查点式，不改实现）**：上限在
  **search 调用入口**判定一次，判定通过后本次调用仍可依次打最多 3 个
  引擎，故会话实际最坏为 **40 + 2** 次导航。这是物理兜底（反检测面），
  不是账本——精确记账的权威在 §3.2.5 的车道预算层。逐引擎判定需要在
  引擎循环中途失败并给信封引入半填状态，收益不足以抵消复杂度，故不改。
- **车道预算（2026-09-10 P2-4 方案 A）**：见 §3.2.5——会话兜底是物理面，
  车道额度在 loop 层。
- 落点：`local_browser/serp.rs` 新模块（引擎表 + URL 构造 + 失败备忘/上限
  状态机 + 结果 URL 解码 + 字段上限，全部纯函数可单测；**信封截断已于
  P2-2 取消**，改为 title/snippet 200、url 2048、reason 200 的字段上限）；
  `cdp.rs` 白名单
  追加固定提取表达式；`mod.rs` 动作解析 +
  `CdpBrowserSession::control_search` 编排。

#### 3.2.4 低质量域名加权（复用既有判定器）

- 每个 SERP 结果在 URL 解码后调用
  `orz_assurance::source_weighting::SourceWeightConfig::classify()`，复用
  `runtime/source-quality-seed-lists-v0.1.json` 与
  `ORZ_SOURCE_WEIGHTING_CONFIG` 覆盖通道。
- 结果信封中每条增加 `tier`（`authoritative/default/low_quality`）、
  `weight`（1.1/1.0/0.7）、`reason` 三个机械字段；不新增 journal 事件，
  不写 evidence schema。
- 稳定排序：`authoritative` 在前、`default` 次之、`low_quality` 在后；
  同 tier 保持原引擎顺序。
- **不硬过滤** `low_quality`：它只是低质量来源信号，不是恶意信号；模型
  仍可见该结果，但已明确降序/标注。
- 配置实例由 search 编排路径持有；`CdpBrowserSession` 或
  `LocalBrowserManager` 只接收 `Arc<SourceWeightConfig>`，不在每次 search
  重复读 env/文件。
- 权重仅作用于工具返回的排序与标注，不进入 ACAF 授权、URL gate、检索
  启用门或失败备忘。

### 3.2.5 车道 SERP 预算（P2-4 方案 A，2026-09-10）

**问题**：宿主工具接口只有 `call_tool(name, args, call_id)`——**不带车道
身份**；而浏览器是进程级单实例。P2-4 复审前，SERP 的会话上限与失败备忘被
主车道和 external 检索车道共用，主车道的探索会挤占检索车道的额度。

**裁决（用户，2026-09-10）**：预算放在能看到车道身份的 loop 层，按车道
独立；检索车道按 effort 档位给额度（检索要有预算，但不该被过度限制）。

**实现形状**（与既有 `candidate_gate` 同族）：

- `LoopProfile.serp_budget: Option<Arc<Mutex<SerpSearchBudget>>>`——主车道/
  grill 每 run 一张（`MAIN_SERP_NAVIGATION_BUDGET = 8`）；external 检索每个
  activation 一张（`EffortTier::serp_navigation_budget()` = standard 8 /
  extended 16 / deep 32）；internal 检索恒 `None`（声明面本就无 browser_control）。
- **每 activation 一张是结构事实，不是每派发一张（2026-09-10 复审 P2-2
  裁定）**：用量作为 `ActivationState.serp_navigations_used` 与
  `candidate_urls` **同形**——每激活累计、随 activation sidecar 存活、
  `continue` 重入沿用、close 才清零（新激活从 0 起）；派发时作为预算初始
  用量（`SerpSearchBudget::with_used`，档位下调时钳到新上限），loop 结束后
  每条路径回写。旧 sidecar 无该键时 `#[serde(default)]` 恢复为 0。
  这样"额度按激活"在结构上成立，而不是只在"每次调用即闭环"的当前生产流
  下偶然等价（`continue` 休眠分支复活或 sidecar 恢复都不会静默重置）。
- **计数单位 = 引擎导航次数**：派发前预留 1；调用返回后按信封
  `engine_attempts` 里 `status ∈ {ok, failed}` 的实际条数结算差额（`skipped`/
  `pending` 不计）。多引擎 `auto` 调用最多短暂超发 ≤2，由会话物理兜底封住。
- **耗尽 = 派发前拒绝**：无 ToolStarted、中性陈述、Denied 反馈进连续拒绝
  断路器（ADR-0010 §3.5.4）、LIF deny 通道、结构化错误码
  `browser_control_search_budget_exceeded` + `serp_budget_used/cap`。
- **不消耗预算的路径**：权限／模式门拒绝 → 预留回滚（候选门 rollback 同义）；
  `navigate/back/forward/refresh/wait_load/snapshot` 与 `web_fetch`/
  `browser_read` 一律不适用。
- **会话底线保留（2026-09-10 复审 P2-3 裁定）**：会话上限跨车道共享，而
  主车道每个 run 都有独立额度、一个会话可以有多个 run——不设底线时主车道
  的探索会把整会话额度吃光，检索车道随后只剩
  `browser_control_search_cap_exceeded`。故 **`SERP_SESSION_RETRIEVAL_FLOOR
  = 16`**：主车道／grill 在派发前读取会话头寸，头寸 ≤ 16 即拒绝（稳定码
  `browser_control_search_session_reserved` + 结构化
  `serp_session_navigations/serp_session_ceiling`），保证检索车道至少拿得到
  一次 standard（8）或 extended（16）档激活的量。判定同为检查点式（最坏再
  侵蚀 ≤2 次导航）。**边界**：① deep 档（32/40）超出该底线，属尽力而为；
  ② 会话 = 浏览器实例生命周期——进程被杀后自愈重启会重置该计数器（会话
  上限同此性质，两层同源）。
- **事实与策略分层（P2-3 落点约束）**：宿主接口没有车道身份，所以"保留
  额度"这类**车道策略**只能在 loop 层施加；loop 需要的唯一事实由宿主上报
  （`LoopHost::serp_session_facts` → 会话已用导航 / 会话上限；宿主实现只在
  有浏览器会话时返回 `Some`）。宿主只报事实、不猜车道；无浏览器会话时不
  施加底线规则（调用按普通失败回传，不由额度面兜）。
- **两层分工**：车道额度管「这条车道能用多少 SERP 机会」（公平 + 预算，
  含给检索车道留的底线）；会话兜底（40 次导航）管「这台浏览器一共能打
  多少次 SERP」（反检测物理上限，跨车道共享，不可被任何单一车道抬高）。
- **Schema 先行（2026-09-10 复审 P1 修复）**：两条拒绝信封的结构化字段
  （`serp_budget_used`/`serp_budget_cap`、`serp_session_navigations`/
  `serp_session_ceiling`）必须先在 `tool-completed-event-payload-v0.2`
  注册（`additionalProperties: false`），并各自配正例 fixture 登记进
  `check_repository` 门禁——否则含该拒绝事件的 journal 会被
  `journal-conformance` 判 `payload schema violation`（GAP-EVENT-SCHEMA-DRIFT
  同类）。
- 落点：`agent_loop.rs`（`SerpSearchBudget` + profile 字段）、
  `retrieval/effort.rs`（档位映射）、`retrieval/dispatch.rs`（按车道构造）、
  `retrieval/activation.rs`（用量随 sidecar 存活）、`host_exec.rs`（门 +
  会话底线 + 结算 + 拒绝信封）、`host.rs`（会话事实 seam）+
  `orz-host` 的 `local_browser`（事实出口）/`lib.rs`（上报实现）。

### 3.3 静态标注

`browser_control` 工具描述追加 search 动作说明（引擎链、有界结果、失败
态），静态文本、无动态字段（0t 静态标注纪律）；external 车道投影路径
自动携带。

### 3.4 不变量

8 工具面冻结（动作扩展非新工具）；零新事件；URL gate 每导航照过（ACAF
network 票据面与 browser_read 同级不变）；FP-2 失败回传纪律；evidence 面
不扩（有界摘要不构成 full_text_observed 语义，沿用 P1-2b「正文读取仍走
browser_read」边界）。

## 4. S4 复验判据（实施后）

1. 检索会话 `browser_control search` 可用：无代理环境下返回 Bing 有机
   结果（结构化、有界、无 `li.b_ad`、URL 已解码），Google 失败被备忘
   后续不再逐次尝试。
2. 检索车道 navigate 不再出现 `retrieval_role_write_denied`。
3. 冷却/上限机械生效（超限显式失败态）。
4. 通用统计：零 400、命中率 ≥90%、search 动作延迟有界（≤3×引擎超时）。
5. CAPTCHA/consent 与空结果被识别为**失败**（`engine_attempts.error_class`
   为 `captcha` / `empty`，或 log 中的同类摘要；`blocked` 仅用于 URL gate
   拦截），不会把污染页当作成功，也不会静默混入结果。
6. `en-US` 参数生效，Bing 请求走 `setmkt=en-US`。
7. 低质量域名加权生效：`low_quality` 结果被标注并排到同引擎尾部，但不
   被硬过滤；`authoritative/default/low_quality` 字段与既有判定器一致；
   Bing 结果的 `url` 是解码后的真实目标域（P1-1 修复后判定器才有域名可
   判）。
8. 旧 journal 回放兼容不受影响（无 schema 变更）。
9. **车道预算（P2-4）**：同一 activation 内超过档位额度（standard 8 /
   extended 16 / deep 32）后，`browser_control search` 在**派发前**被拒
   （无 ToolStarted、稳定码 `browser_control_search_budget_exceeded` +
   `serp_budget_used/cap`）；主车道额度用尽不影响外部检索车道（反之亦然），
   且激活用量跨 `continue`／sidecar 恢复不重置。
10. **会话底线（P2-3）**：会话头寸降到 16 时，主车道的 search 被拒（稳定码
    `browser_control_search_session_reserved` + 会话字段），检索车道仍能继续
    使用保留额度；带这两种拒绝事件的 journal 通过 `journal-conformance`
    （P1 修复后 schema 已登记）。

## 5. 排期建议

S1 代码（tool.rs 分类 + serp.rs + 低质量域名加权接线 + mod.rs/cdp.rs
编排）→ S2 测试（纯函数单测 + 车道门测试 + 加权排序/标注测试）→ S3
双平台重建 → S4 实机（可搭 0t S4 判据 6 宿主机日常可用性验证同场执行）。
计数：立项登记不动计数，闭合按本条目入账。

### 5.1 S1 实施记录（2026-09-10）

> 注：本节是 S1 当时的落码记录；其中"pacing 与 10 次上限""8KiB 有界信封"
> 已被 §5.3（P2-2 字段上限；P2-4 会话上限改为 40 次导航）取代，现行口径以
> §3.2.3／§3.2.5 为准。

- `orz-loop/src/tool.rs`：`risk_class("browser_control") = ReadOnly`；
  `agent_loop` 检索写门测试补 `write_gate("browser_control") == None`。
- `orz-host/src/local_browser/serp.rs`：引擎链/en-US URL/会话失败备忘/
  pacing 与 10 次上限/Bing `ck/a` 与 DDG `uddg` 解码/低质量加权与稳定
  排序/8KiB 有界信封（纯函数）。
- `orz-host/src/local_browser/cdp.rs`：`BrowserControlAction::Search`
  编排 + Bing 广告排除/CAPTCHA/空结果/网络失败备忘 + 无头反污染前置
  （viewport、`AutomationControlled`、`window.chrome` 存根、UA 去
  HeadlessChrome 标记）；固定 SERP 提取表达式仍由主机持有。
- 依赖：`base64` 为既有 workspace 依赖，零新增传递项。

### 5.2 S2 测试记录（2026-09-10）

> 注：本节是 S2 当时的测试记录；"10 次会话上限""8KiB 结果载荷截断"两项已被
> §5.3（P2-2 字段上限；P2-4 会话上限改为 40 次导航）取代，现行测试以代码中的
> `weighted_results_are_bounded_per_field` 与
> `session_state_memoizes_failures_and_caps_navigations` 为准。

- 纯函数矩阵：引擎固定顺序与标签、en-US URL 编码、Bing/DDG 跳转解码、
  解析保留 CAPTCHA、畸形/空 URL 过滤、结果 ≤10、失败备忘覆盖、可用引擎
  跳过、pacing 首次立即/后续 5s+0–50% 抖动、10 次会话上限、加权稳定
  排序、展示字段截断、8KiB 结果载荷截断。
- 宿主动作面：固定 SERP 表达式与 Bing 广告排除、search 信封、参数校验、
  工具定义形状、加权字段透传、搜索上限在触达浏览器前显式拒绝。
- 车道门面：`browser_control` ReadOnly、`action_category=read`、
  retrieval `write_gate None`。
- 回归口径：`orz-host` lib 串行 283 passed / 0 failed（并行跑时的
  `env_snapshot` 与 `call_tool_timeout_kills_process_tree` 两个机器负载型
  flaky 单独复跑均过）；`orz-loop` lib 747 passed / 0 failed。
- S2 发现并修复：首次 search 误取 5s 冷却基值导致先等 5–7.5s；现改为
  会话首次立即执行，后续 search 才计算最小冷却与抖动。

### 5.3 S1 全面复审与修复（2026-09-10）

复审范围：设计合理性、实现合理性、设计与实现的符合性；证据 = 逐文件
对读 + 聚焦测试复跑（`orz-host` local_browser 80 通过/4 忽略）+
一次 Bing 只读实测（真实响应 200、10 条有机结果、4 个广告容器）+
一次 headless Chrome 153 行为探针（`--dump-dom`）。

- **P1-1（修复）** Bing `/ck/a` `u` 参数实测为 `a1<base64url>`；旧解码只
  处理无标记形态，对真实有机结果 100% 失效（回落成 base64 串，连带破坏
  §3.2.4 域名加权）。改为「原值 → 去 `a1`」候选 + http(s) URL 校验 +
  原始链接回落；夹具换成实测形态（`a1aHR0cHM6…` → `https://www.104.com.tw/…`、
  相对路径 `a1L2ltYWdlcy9zZWFyY2g_cT1ydXN0` → `https://www.bing.com/images/search?q=rust`）。
- **P2-1（修复）** search 成功/失败信封新增 `engine_attempts`（引擎 +
  status + error_class + reason）；失败时模型必须能分辨是哪个引擎失败，
  「全部已备忘」路径以 `status:"skipped"` 显式列出而非只回一句笼统失败。
- **P2-2（改设计并实现）** 取消整包 8KiB 截断，改为字段上限
  （title/snippet 200、url 2048、reason 200）+ ≤10 条；URL 上限是替代
  整包截断的那一层（引擎可控的 href 是唯一无界入口）。
- **P2-3（修复）** 反污染拆成启动层（`--window-size`、自动化标记位）与
  会话层一次性加固；加固改为尽力而为（失败只落 `[browser_log]`，不再让
  navigate/back/refresh 连带失败）。实测 `window.chrome` 已存在、
  `navigator.webdriver` 在纯 CDP 启动下为 false，故删除
  `Page.addScriptToEvaluateOnNewDocument` 存根及其 CDP 白名单项（白名单
  收窄而非扩张）；UA 覆写保留（Chrome 153 无头 UA 仍带 HeadlessChrome）。
- **P2-4（修复，方案 A）**：SERP 预算按车道独立——`LoopProfile.serp_budget`
  由 loop 层注入（主/grill 每 run 8；external 检索每 activation 按 effort
  档位 8/16/32；internal 恒 `None`），单位 = 引擎导航次数，派发前预留 1 +
  调用返回后按 `engine_attempts` 结算；耗尽 = 派发前拒绝（无 ToolStarted +
  Denied + 稳定码 `browser_control_search_budget_exceeded` + used/cap）。
  会话上限改为**物理兜底 40 次导航**（跨车道共享、反检测用），不再承担
  车道额度语义。见 §3.2.5。
- **回归**：`orz-host` lib 284 通过 / 0 失败 / 5 忽略；`orz-loop` lib 753
  通过 / 0 失败 / 3 忽略；`cargo fmt --all -- --check` 通过。
- **未闭合（设计留存、暂不实施）**：引擎自选 + 去备忘——见 §6（用户
  2026-09-10 裁决：方案留在文档，不改当前行为）。

### 5.4 P2-4 车道预算专项复审与修复（2026-09-10）

复审范围：P2-4 方案 A（车道 SERP 预算）的**设计合理性、实现合理性、设计
与实现的符合性**；证据 = 逐文件对读 + 聚焦复跑（`orz-loop` serp 过滤 6 →
10 通过、全量 757 通过；`orz-host` lib 286 通过 / 5 忽略）+ 一次
jsonschema 实测（拒绝 payload 对 v0.2 schema）。

- **P1-1（修复，机器合约）**：`browser_control_search_budget_exceeded` 拒绝
  信封带 `serp_budget_used/cap`，但 v0.2 payload schema 是
  `additionalProperties: false` 且未登记该二字段——实测
  `Additional properties are not allowed`。生产事件轨是 V02，而
  `journal-conformance`（Rust 法官）按 registry 选 schema 逐事件校验 payload，
  故任何含该拒绝事件的 journal 都会判 `payload schema violation`（S4 实机
  一旦触发即红）。修复 = 登记 4 个字段（含 P2-3 二字段）+ 两个正例 fixture
  并入 `check_repository` 门禁（与候选门 `candidate_count/cap` 先例对齐）。
  设计侧同步：§3.2.5 补"Schema 先行"条。
- **P2-1（只改措辞，用户裁定）**：会话上限按 search **调用入口**检查点判定，
  判定后单次 `auto` 最多再打 3 个引擎 → 实际最坏 40 + 2。判定方式本身无
  问题（物理兜底 ≠ 账本；精确记账在车道预算层），逐引擎判定要引入信封
  半填状态、收益不足。→ §3.2.3 措辞改为"检查点式 + 最坏 40+2"。
- **P2-2（修复，用户裁定）**：用量挂到激活（`serp_navigations_used`，与
  `candidate_urls` 同形：sidecar／continue／每条路径回写／旧档 default 0），
  派发时以 `with_used` 承接。修复前是"每次派发一张"——在当前"每次调用即
  闭环"的生产流下等价，但 `continue` 休眠分支与 sidecar 恢复会静默重置。
- **P2-3（修复，用户裁定）**：会话层为检索车道保留底线额度
  `SERP_SESSION_RETRIEVAL_FLOOR = 16`（≥ 一次 standard 或 extended 激活；
  deep 32/40 属尽力而为，已登记边界）。落点约束：宿主没有车道身份，故
  由宿主上报事实（`LoopHost::serp_session_facts` /
  `BrowserSession::serp_session_navigations`，缺省 `None`，其它宿主零改动）、
  由 loop 施加策略；拒绝码 `browser_control_search_session_reserved` +
  会话结构化字段。
- **P2-6（防御性修复）**：ACAF 票据拒绝分支此前只回滚候选占位；补上 SERP
  预留回滚（当前 `browser_control` 不在票据映射内，该分支对它不可达——
  形态对齐，防日后纳入票据面时静默多计）。
- **P3-1／P3-2／P3-3（注释与边界登记）**：结算 Err 路径只发生在"浏览器未
  启动/未导航"场景，保留 1 次预算是保守诚实的一侧（加注释说明）；`new(0)`
  归一为 1 的语义（加注释/断言）；console 发放路径传 `None`——当前 console
  动作登记表无以 `browser_control` 为目标的动作，故不可达（登记为边界）。
- **符合性复核结果**：4 处落点与 §3.2.5 逐条对上（字段/门/结算/回滚/会话
  兜底/不适用路径），并发面确认 `browser_control` 不在同轮读类并行集内
  （search 恒串行，`Mutex` 无竞态语义问题）。
- **测试补齐**：主车道 vs 检索车道额度独立（结构断言 + 端到端承接用例）、
  会话底线拒绝与放行边界、激活用量跨 sidecar 往返、宿主事实出口、旧 sidecar
  向后兼容。
- **回归**：`cargo fmt --all -- --check` 通过；`orz-loop` lib 757 通过 /
  0 失败 / 3 忽略；`orz-host` lib 286 通过 / 0 失败 / 5 忽略；
  `cargo check --workspace` 无告警；`runtime/tests/test_run_event_conformance.py`
  15 通过；`scripts/check_repository.py` 唯一报错为"orz 子模块工作树脏"
  （本轮未提交，预期）。

## 6. 待裁决设计：引擎自选与去备忘（**未实施**，2026-09-10 用户提议）

> 状态：**设计留存，暂不落码**（用户 2026-09-10 裁决）。本节不改当前行为；
> 当前实现仍是 §3.2 的固定引擎链 + 会话失败备忘。
>
> **2026-09-12 更新**：本节的「**完全去备忘**」被否；采纳的是本节所列
> **备选「软备忘＝只调整顺序」**——见 §8（设计定稿、待实施）。本节其余
> （引擎自选 `engine` 参数）仍为设计留存、未实施。

**目标语义**：三引擎同时开放、保留推荐顺序、由模型自己操控。

**工具面**：`browser_control search` 增加可选参数 `engine ∈ {auto, google,
bing, duckduckgo}`（默认 `auto`）。描述写死推荐顺序「Google → Bing →
DuckDuckGo」，静态文本（0t 静态标注纪律），两条车道一致。`auto` = 单次
调用内按推荐顺序尝试、首个成功即返回；显式 `engine` = 只打该引擎。

**去备忘**：删除 `SerpSessionState` 的失败备忘（不再跨调用记住失败，`engine_attempts`
随之只剩「本次真实发生」的状态，不再出现 `skipped`）。三引擎始终可试、模型
始终可显式指定，符合「零隐藏状态」的整洁性。

**代价与补偿**：无代理环境下 `auto` 每次都会先付一次 Google 超时（默认
`timeout_secs` 下每次最多 +30s）。补偿手段是模型侧：第一次从
`engine_attempts` 看到 `google = network/timeout` 后，后续显式用
`engine=bing`；`timeout_secs` 仍由模型掌控。

**备选（若日后要收回重复超时代价）**：「软备忘＝只调整顺序」——失败的引擎
在 `auto` 里排到队尾但**从不删除**，把「每次 +30s」降为「首次一次」，代价是
保留一份只影响顺序的隐藏状态。

**落码前需同步**：§2 非目标里「机械层不暴露引擎切换」须作废；§3.2.3 信封
与 §4 判据补 `engine` 参数项（含 S4 判据 6「显式引擎参数生效」）；工具面
变更属模型面，需先登记再实施。

## 7. 调查记录（本轮只查证、未实施）

- 已核实 `local_browser/` 现有模块构成与 CDP 白名单/固定表达式纪律；
  `risk_class` 兜底分支与 `browser_read` 先例注释；`handle_browser_control`
  严格解析结构；`ToolFilter::Retrieval.write_gate` 分类逻辑。
- 已核实设计权威链：调研 §8.1（链序裁决）/§8.3（防护清单）、索引
  FUS-RETRIEVAL-ENGINE-SERP（0t 后边界注记）、ADR-0010 §14.65。
- 已检索并权衡 Bing/DDG 无登录态污染方案：`GUILessBingSearch`（真实
  Chromium/TLS 根因 + 无头适配）、`OpenSERP`（Bing 有机/广告/CAPTCHA
  选择器）、`SearchParser`（Bing/DDG ad/result 分离）、`bingc`（API 分面
  参考）、`metasearch`（Bing 摘要兜底选择器）。仅吸收工程事实，不引入外部
  抓取依赖；许可兼容性在实施时单独核对。
- orz 两侧工作树保持全净，无任何代码变动。

## 8. 用户裁决：引擎链「软备忘」（2026-09-12；**设计定稿、待实施**）

> 触发证据：[`0v S4 复跑记录`](audits/0V_S4_RERUN_2026-09-12.md)（载体 0.4.2，
> 588 事件 / 68 模型轮 / 1800s 官方墙钟）。本节取代 §6 的「完全去备忘」成为
> 当前采纳方向；§6 保留作对照与「引擎自选」的留存设计。

### 8.1 裁决与证据

用户 2026-09-12 裁决：采纳 §6 所列**备选方案「软备忘＝只调整顺序」**。

| 证据 | 事实 | 含义 |
|---|---|---|
| 首搜（seq=108） | wall **44.7s**，Chrome History 留下真实 Bing 导航 | 引擎链跑满一轮 |
| 其后 3 次 search | wall **1,673 / 3,944 / 907 ms**，History **零新增导航** | 三引擎已在首轮被备忘，链被短路 |
| Bing 页面本身 | load 时点即有 `#b_results` ×1、`li.b_algo` **×10**、无 challenge | Bing 可用，失败不在页面 |

即：**一次瞬时首败被固化成一整段 46 分钟会话的引擎全黑**，代价与失败原因
不成比例。同时，本环境 Google 被 URL gate **确定性**拦截（DNS 污染），使
§6「完全去备忘」的代价从当时估计的「首次一次 +30s」变为**每次 +30s**——
故完全去备忘被否。

### 8.2 语义（规范）

1. **引擎集合恒定**：`auto` 的候选恒为三引擎全集（Google / Bing / DuckDuckGo），
   任何失败都**不把引擎移出候选**。
2. **失败只影响顺序**：一次失败的引擎**置队尾**；已置尾者再次失败**保持队尾**
   （不叠加惩罚、不倒序、不产生第二种惩罚位）。
3. **单次调用语义不变**：按当前顺序逐个尝试，**首个成功即返回**；同一引擎在
   单次调用内最多尝试一次；全部失败 → `all_engines_failed`。
4. **代价上界**：只有在「排在其之前的引擎全部失败」时才会尝试到被置尾的引擎
   → 被确定性拦截的引擎不再出现在成功路径上，其超时只在无可用引擎时才付出。
5. **作用域不变**：顺序状态仍随 `SerpSessionState`（会话级、机械内存、模型
   不可见）；**不再存在「引擎被禁用」这一状态**。
6. **护栏不变**：会话上限 40 次引擎导航（物理兜底）、冷却 5s、P2-4 车道预算、
   P2-3 会话底线全部不动——防重试风暴由这些上限承担，不由备忘承担。

### 8.3 可观测面变更

- `engine_attempts[].status`：**新增 `not_attempted`**（本次未尝试：链已在更前
  的引擎成功，或该引擎排在未被触及的位置）；**退役 `skipped`**——其唯一含义
  「被备忘跳过」在软备忘下不存在。
- `engine_attempts` 仍**显式列出全部三个引擎**（不静默消失，沿 P2-1 纪律）；
  `all_engines_failed` 的含义**收紧**为「本次调用三引擎均被真实尝试且均失败」。
- 失败类别 `error_class` 语义不变（`blocked` / `dns` / `connection_reset` /
  `certificate` / `timeout` / `captcha` / `empty`）。

### 8.4 落点（实施时）

| 文件 | 变更 |
|---|---|
| `crates/orz-host/src/local_browser/serp.rs` | `available_engines()`（剔除式）→ `ordered_engines()`（全量 + 失败者置尾）；`SerpSessionState` 的失败记录降级为**纯排序依据**；`SerpEngineAttempt` 状态枚举 `skipped` → `not_attempted` |
| `crates/orz-host/src/local_browser/cdp.rs` | 引擎链循环改为「按顺序尝试直至成功」，不再因备忘跳过；`all_engines_failed` 判定改为「三引擎均真实失败」 |
| 测试 | `available_engines_skip_memoized_failures_in_fixed_order` → 改断言**顺序**而非集合；`session_state_memoizes_failures_and_caps_navigations` → 改断言「失败不删除 + 上限仍在」；新增「置尾后再失败保持队尾」「成功时后续引擎记 `not_attempted`」「三引擎全失败才 `all_engines_failed`」 |
| schema / fixture | `browser_control` 结果形状若被 fixture 固化，同步 `not_attempted`；`check_repository` 全绿为收口 |

**边界**：本变更**不触碰**工具面签名（不加 `engine` 参数，§6 的引擎自选仍未
实施）；不改模型可见状态；不发版前无兼容负担（0.4.2 为过程验证版本）。

### 8.5 S4 判据修订

- 判据 1 原文「…Google 失败**被备忘**后续不再逐次尝试」→ 改为
  「**Google 失败后置队尾，在成功路径上不再被尝试**（表现为后续调用的
  `engine_attempts` 中 `google = not_attempted`）」。
- 判据 1/5/7 的取证**依赖 §8.6 的 0v-A**——在取证面补齐前，这三条判据在实机
  批次中不可事后复核（本批实证）。

### 8.6 两项取证待办（用户 2026-09-12 指示登记）

**0v-A 引擎级取证面（框架侧；判据 1/5/7 可取证的前提）**

- **问题**：`engine_attempts` / `error_class` / `low_quality` **没有任何持久化面**
  ——事件面按设计只落 `ToolCompleted` 事实；会话卷只有 `events.jsonl` /
  `ledger/current.md` / `retrieval-results/*.json` / Chrome profile；
  `agent/trajectory.json` 只有模型侧消息、不含工具结果。→ 判据 1/5/7 **在设计上
  不可事后取证**（机制再正确也拿不到证据）。
- **目标**：让这三类字段落到可事后复核的面。候选（实施时定案）：
  ①事件面扩展（`ToolCompleted` 附加摘要字段，或新增引擎尝试事实事件）；
  ②会话卷落盘（如 `runs/<run>/serp-attempts/*.json`，与 `retrieval-results/` 同形）。
- **边界**：不改模型面工具签名；不新增模型可见常驻状态；`engine_attempts`
  对模型的可见内容不缩水。

**0v-B 定向探针（装置侧，不改 orz）**

- **目的**：一次取证判据 1/5/7 与 **9/10**（9/10 需单次激活 ≥9 次引擎导航，
  自然任务不产生：最重的 dna-assembly 也只给到 4 次 search）。
- **形态**：同一评测容器、固定查询集（含内容农场域查询与可能触发
  CAPTCHA/consent 的查询）、显式要求多次 `browser_control search` 与引擎导航，
  打穿档位额度（8/16/32）与会话底线（16）。
- **依赖**：与 0v-A 配套——探针产出的引擎级证据必须有落盘面才能复核；
  在 0v-A 未落地前，探针只能产出「模型声明」级证据。
