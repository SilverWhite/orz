# 检索引擎 SERP 接入与 `browser_control` 车道分类修正设计（2026-09-10）

> 来源：0u R4 实机证据（[`OFFICIAL_R4_UNSOLVED15_RERUN_2026-09-10`](audits/OFFICIAL_R4_UNSOLVED15_RERUN_2026-09-10.md)
> §5A）+ 2026-09-10 用户裁决（无代理环境下后端检索超时、Google 不可用时，
> 满足模型搜索引擎需求——按既有 SERP 设计补实现，候选 Bing）。
> 设计基线：[`RETRIEVAL_AND_SUBAGENT_COMMUNICATION_BORROW_RESEARCH_2026-08-30`](RETRIEVAL_AND_SUBAGENT_COMMUNICATION_BORROW_RESEARCH_2026-08-30.md)
> §8.1/§8.3（引擎链裁决 + 机械防护清单）、索引 `FUS-RETRIEVAL-ENGINE-SERP`
> （current-design；0t 后模式语义退役、引擎链与 pacing 保留）、ADR-0010
> §14.65（0t 双车道）。状态：**设计定稿，实施待用户放行**。

## 0. 用户裁决（2026-09-10 登记）

1. `browser_control` navigate 在 external 检索车道被拒（`retrieval_role_write_denied`）：
   模型在无代理 + 后端检索超时场景下有「导航到搜索引擎」的真实需求，**应满足**。
2. 昨晚 web_search 超时必须**坐实**为 DeepSeek 后端检索问题（已完成，见 §2，
   证据三柱闭合）。
3. **增加一个国内可用的搜索引擎**：检索层已有设计（引擎 SERP 链）未实现，
   候选 Bing——按设计补实现。

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
防护与失败态按既有设计。

**非目标**：
- 不加新工具（8 工具面冻结）——能力以 `browser_control` 新动作承载。
- 不做跨引擎 RRF 共识/域名质量加权（§8.3 第 2/3 项清单另批；本期单引擎
  顺序链 + 失败备忘）。
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

- 引擎链（调研 §8.1 保底顺序，常量）：**Google → Bing → DuckDuckGo**；
  SERP URL：`google.com/search?q=` / `bing.com/search?q=` /
  `html.duckduckgo.com/html/?q=`（均为公开 http(s)，过既有 URL gate）。
- **会话级失败备忘**（机械内存，模型不可见）：某引擎网络级失败（dns/
  connection_reset/timeout）记入会话备忘，同会话后续 search 跳到队尾——
  无代理环境的 Google 首次失败后不再逐次付超时成本；状态存
  `CdpBrowserSession`（会话生命周期，随会话销毁）。
- **SERP 提取**：宿主自有固定表达式（CDP `Runtime.evaluate` 既有白名单
  纪律，模型永不供表达式）按引擎取有机结果——Bing `li.b_algo`（`h2 a` +
  `b_caption p`）、Google `#search a h3` 锚点、DDG `a.result__a` +
  `.result__snippet`；提取 0 条 = 该引擎失败态（blocked，CAPTCHA/consent
  同类），按链切下一引擎。
- **结构化信封**（有界）：`{action:"search", action_status, engine, results:
  [{title,url,snippet} ≤10], error_class?, nav_phase, log}`——snippet ≤200
  字符、总载荷 ≤8KiB 机械截断。全部引擎失败 = `action_status:error` +
  `error_class:"all_engines_failed"` + 每引擎失败类摘要入 log（显式、无
  静默混用，FP-2 纪律）。
- **机械 pacing**（调研 §8.3/索引 FUS-RETRIEVAL-ENGINE-SERP「pacing/频率
  上限保留」）：同会话两次 search 最小冷却 5s；会话上限 10 次，超限显式
  `browser_control_search_cap_exceeded`（与 web_fetch 候选 cap 同族形态）。
- 落点：`local_browser/serp.rs` 新模块（引擎表 + URL 构造 + 备忘/上限状态
  机 = 纯函数可单测）；`cdp.rs` 白名单追加三条固定提取表达式；`mod.rs`
  动作解析 + `CdpBrowserSession::control_search` 编排。

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
   结果（结构化、有界），Google 失败被备忘后续不再逐次尝试。
2. 检索车道 navigate 不再出现 `retrieval_role_write_denied`。
3. 冷却/上限机械生效（超限显式失败态）。
4. 通用统计：零 400、命中率 ≥90%、search 动作延迟有界（≤3×引擎超时）。
5. 旧 journal 回放兼容不受影响（无 schema 变更）。

## 5. 排期建议

S1 代码（tool.rs 分类 + serp.rs + mod.rs/cdp.rs 接线）→ S2 测试（纯函数
单测 + 车道门测试）→ S3 双平台重建 → S4 实机（可搭 0t S4 判据 6 宿主机
日常可用性验证同场执行）。计数：立项登记不动计数，闭合按本条目入账。

## 6. 调查记录（本轮只查证、未实施）

- 已核实 `local_browser/` 现有模块构成与 CDP 白名单/固定表达式纪律；
  `risk_class` 兜底分支与 `browser_read` 先例注释；`handle_browser_control`
  严格解析结构；`ToolFilter::Retrieval.write_gate` 分类逻辑。
- 已核实设计权威链：调研 §8.1（链序裁决）/§8.3（防护清单）、索引
  FUS-RETRIEVAL-ENGINE-SERP（0t 后边界注记）、ADR-0010 §14.65。
- orz 两侧工作树保持全净，无任何代码变动。
