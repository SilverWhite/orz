# 双检索模式验证与并发评估（2026-08-29）

> 性质：设计思考记录。2026-08-30 检索问题最终评判收口（§7），§6 悬置项
> 全部定案。背景：2026-08-29 用户提出
> 三点——①「API 原生检索结果本身就是再加工结果」的调研结论是否成立；
> ②双检索模式（framework_fallback vs local_browser）带来的究竟是正面
> 还是负面结果，必须确定；③时间问题用高并发处理是否可行。用户指示：
> 先想清楚再动手，controller 拆分暂缓。本文记录事实核对与验证设计。

## 1. 「API 原生检索是再加工结果」：结论成立，双重依据

### 1.1 设计文档原文（当前设计权威）

[`RETRIEVAL_SOURCE_WEIGHTING_DESIGN_2026-08-12.md`](RETRIEVAL_SOURCE_WEIGHTING_DESIGN_2026-08-12.md)
§1 原文：

> DeepSeek 原生服务端 web search（Responses API）返回的是**服务端自主
> 多轮搜索后综合的 `output_text` + 引用 URL 列表**；该内容由服务端
> DeepSeek 生成，runtime 不可控。子代理对返回内容的使用本质是**二次
> 筛选与判断**。

§2 模式二分：

- **framework_fallback**：`web_search`（DeepSeek 原生）为检索入口；
  引用原文核验用 `web_fetch`；完整三层结构。
- **local_browser**：`browser_read` **直接读取原文**；`web_search`
  显式关闭；无服务端综合层，直接分级加权。

### 1.2 实测证据（2026-08-11 真实 key 审计）

[`ADR_0006_WEB_SEARCH_CREDENTIAL_AND_C2_1_UNBLOCK_IMPL_AUDIT`](audits/ADR_0006_WEB_SEARCH_CREDENTIAL_AND_C2_1_UNBLOCK_IMPL_AUDIT_2026-08-11.md)
实测：

- DeepSeek `/v1/responses` + `tools:[{type:"web_search"}]` 可用；
  服务端自主多轮搜索（`web_search_call`：search queries / open_page url）
  → 综合答案 `output_text`。
- 响应字段：`output_text` → content；`url_citation`/`web_search_call`
  → citations。
- 「open_page 失败率由服务端承担（实测 x.ai 等反爬页 failed）」——服务端
  打开页面也有失败，模型拿到的综合文本可能已经丢失失败页信息。

结论：**调研没有错**。API 原生检索 = 服务端已加工的综合答案（模型看到
的是二手文本 + URL 列表），local_browser = 子代理自己读原文（一手
渲染文本）。「本地检索替换 API 检索增加准确性」的动机成立——但要验证
它是否真的带来正收益（§3）。

## 2. 时间问题的事实基础：串行执行 + 全局并发 1

### 2.1 工具调用串行执行（源码确认）

`agent_loop.rs` 1975：`for tc in &response.tool_calls` —— 模型一次声明
的多个 tool_calls **逐个执行**，最慢的拖死整轮。

实测（torch-pipeline-parallelism run，1478s 超时）：

- 主代理一次 `web_search` → 外部检索子代理多轮会话 493s；
- 子代理轮 1：3 个并行声明的 web_search 串行执行（各 50–60s）；
- 子代理轮 2：连续 8 个 web_fetch 串行（每个 1s + 每页 1 张票据往返）；
- 合计：1478s 中 1446s（98%）花在 web_search 相关路径。

### 2.2 全局并发 1（ADR-0010 §3.7.7/§11.3，源码确认）

`orz-host/src/lib.rs` 110：`web_search_semaphore: Arc<Semaphore>`——
主 agent 与外部检索子代理共享**同一把信号量，permit=1**。web_search
全局并发固定为 1；web_fetch 不在此列（可并发，但当前执行循环串行）。

## 3. 双检索模式验证设计（确定正/负面）

### 3.1 验证目标

回答一个问题：**同一模型、同一任务集，framework_fallback（API 检索）
与 local_browser（本地浏览器原文检索）相比，是正收益还是负收益？**
分三个维度度量：

| 维度 | framework_fallback | local_browser | 预期差异 |
|---|---|---|---|
| 通过率/reward | 基线（8/31 实测解出部分） | 待测 | 若原文检索减少「二手综合」偏差 → 更高 |
| 引用准确性 | 服务端综合文本 + 引用列表 | 子代理自己读原文 | 若子代理能读原文 → 更可靠 |
| 时间成本 | 单次 15–70s 正常 / 无超时曾挂 22min | 浏览器启动 + 页面渲染 | 可能更慢（需并发/预算补偿） |

### 3.2 验证方法（建议 A/B 同题对照）

1. **任务集**：从 TB 2.1 检索密集题中选 3–5 道（mteb-leaderboard /
   path-tracing-reverse / rstan-to-pystan / configure-git-webserver /
   mteb-retrieve——均为历史超时/检索依赖题），k=1 或 k=2。
2. **双跑**：同任务、同模型、同 effort，分别强制
   `framework_fallback`（现状）与 `local_browser`（§4 docker 方案），
   各跑一遍。
3. **测量**：reward、解出/超时、引用 URL 与最终答案的绑定（source
   ledger 交叉核对）、单题墙钟、工具轮数、token 成本。
4. **判定阈值**（建议，未裁决）：若 local_browser 通过率 ≥
   framework_fallback 且时间在预算内（≤ 官方 timeout 的 80%）→ 正面，
   升级为默认模式；若通过率持平但时间超支 → 保留 framework_fallback
   默认、local_browser 按需；若通过率下降 → 否决本地优先，维持现状。

### 3.3 前置条件（local_browser 评测可用）

- 浏览器二进制注入：orz adapter install 层 apt 装 chromium +
  `ORZ_BROWSER_PATH` / `ORZ_BROWSER_HEADLESS=1`（见
  [`CONTROLLER_SPLIT_DESIGN_2026-08-29.md`](CONTROLLER_SPLIT_DESIGN_2026-08-29.md)
  §4.3）。
- 启动参数补 `--no-sandbox` / `--disable-dev-shm-usage` / `--disable-gpu`
  （docker 必需，当前 `browser_launch_args` 缺失）。
- 注：**浏览器不是超时解药**——官方 82.7% minimal mode 无 web_search
  也无浏览器；本地检索的价值在「一手原文准确性」，验证聚焦准确性而非
  救超时。

## 4. 高并发处理时间问题：评估

### 4.1 用户提议：高并发

方向：把串行工具调用（§2.1）和全局 web_search 并发 1（§2.2）放开，
用并发掩盖单次延迟。

### 4.2 拆成两个独立问题

| 问题 | 现状 | 高并发建议 | 风险 |
|---|---|---|---|
| 同一轮多个 tool_calls 串行 | `for tc in` 逐个执行 | **同一轮内的独立调用并行**（FuturesUnordered / join_all），慢者不拖死整轮；同一工具的共享资源（web_search semaphore）仍排队 | 写类工具并行有竞态（search_replace 锚点/文件写）——只对读类/检索类并行，写类保持串行 |
| web_search 全局并发 1 | `Semaphore::new(1)` | 按需放开（2–4），或保持 1 但让**同一轮内多个搜索共享一次服务端调用**（合并查询） | DeepSeek 服务端多轮搜索本身慢（15–70s），并发会放大后端负载与成本；官方 minimal mode 压根不用 web_search |

### 4.3 判断（待用户裁决）

1. **同轮并行**：方向正确且低风险——只对读类/检索类工具并行，写类
   保持串行；能直接解决「3 个并行声明的搜索串行执行、最慢拖死整轮」
   的形态。改动集中在 agent_loop 工具批次执行处（§2.1）。
2. **web_search 并发 >1**：**不建议作为默认**——DeepSeek 服务端搜索
   是计算密集的生成式检索，并发放大延迟/成本且与官方 minimal 的
   「无 web_search」对照背离；若要做，先做 4.2 的「同轮并行 + 信号量
   排队」再看是否需要提升 permit。
3. **更根本的替代**：检索通道预算（子代理 run 级轮数/墙钟双层预算）
   + 结果有界化（blackboard 指针摘要已有，防止子代理拉 8 页串行
   fetch）比「无脑并发」更对症——并发解决「等待」，预算解决「过度
   检索」。

## 5. 与 controller 拆分的关系

- 拆分暂缓（用户裁决）；但 §3 验证与 §4 并发评估的落点仍在
  `run_retrieval_subagent` / 工具批次执行处——这两个位置无论拆不拆
  都是检索效率的关键路径。
- 若验证/并发改动先做，controller 拆分设计中的 B5（dispatch）/
  工具批次并行将吸收这些改动，拆分时保持行为不变即可。

## 6. 下一步（未裁决，等用户拍板）

1. 是否按 §3.2 启动双检索模式 A/B 验证（需先做 §3.3 docker 浏览器
   前置）。
2. 是否实施 §4.3-1 同轮读类并行（独立小改动，可先于验证落地，
   与检索模式无关）。
3. web_search 并发是否保持 1（建议保持，理由见 §4.3-2）。

## 7. 定案收口（2026-08-30 最终评判，用户确认无异议）

1. **不做全量 A/B**：双模式定案后 A/B「选边」功能消失；§3.2 的度量维度
   （reward/超时/引用绑定/墙钟/工具轮数/token）转用于 **Google 门禁观察实验**
   （小批检索密集题 k=1、个人使用强度），一次完成「引擎可用性验证 + pacing
   校准 + Google/Bing 主序裁决」（见
   [`RETRIEVAL_AND_SUBAGENT_COMMUNICATION_BORROW_RESEARCH_2026-08-30.md`](RETRIEVAL_AND_SUBAGENT_COMMUNICATION_BORROW_RESEARCH_2026-08-30.md)
   §8.4/§9.3）。
2. **同轮读类并行：批准，第一批第一项**（§4.3-1 采纳；FuturesUnordered 并行
   读/检索类，写类保持串行，web_search 信号量维持 1）。
3. **web_search 并发维持 1**（§4.3-2 维持建议；收益从同轮并行 + 检索通道预算
   获得，不从提升 permit 获得）。
4. 第一批检索编排机械层共五项（含子代理 run 级预算/超时提前至第一批）、第二批
   三项、暂缓/否决清单与顺序纪律，登记于调研文档 §9.3–§9.5 / BACKLOG 0k /
   TODO P0-0k / ADR-0010 §14.43。
