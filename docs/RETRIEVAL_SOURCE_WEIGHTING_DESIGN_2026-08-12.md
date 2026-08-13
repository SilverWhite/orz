# 检索来源加权与原文核验设计（2026-08-12）

> 状态：`current-design`（2026-08-12 用户裁决；**实现已闭合 2026-08-13**，
> GAP-SOURCE-WEIGHTING-IMPL `implemented`，实施审计见
> [`GAP_SOURCE_WEIGHTING_IMPL_AUDIT_2026-08-13.md`](audits/GAP_SOURCE_WEIGHTING_IMPL_AUDIT_2026-08-13.md)）。
> 权威：ADR-0010 §3.7 条 12（v1.6 补写，2026-08-12）。
> 边界：本机制属**检索结果质量层**，管"信什么、怎么标注"；不属授权层，不改变 ACAF
> 票据边界（ADR-0011 D-12：web_search 无票据映射；D-13：子代理动作票绑定 activation）。

## 1. 背景

DeepSeek 原生服务端 web search（Responses API）返回的是服务端自主多轮搜索后综合的
`output_text` + 引用 URL 列表；该内容由服务端 DeepSeek 生成，runtime 不可控。子代理对
返回内容的使用本质是**二次筛选与判断**。因此质量层不能只审查返回文本，必须对来源 URL
与（可得的）原文做逐来源加权与核验。

D 项目旧机制（SearxNG 元数据评分：engine/publishedDate/citations 三维鉴权）依赖每条结果
的结构化元数据，DeepSeek 原生返回不提供，故**不迁移旧评分**，按三层结构重新设计。

## 2. 模式二存一

`framework_fallback` 与 `local_browser` 是互斥显式模式（ADR-0010 §3.7 条 1，禁止隐式切换）：

- **framework_fallback**：`web_search`（DeepSeek 原生）为检索入口；本地浏览器显式关闭；
  引用原文核验使用 `web_fetch`，**不混用 `browser_read`**；使用完整三层结构；
- **local_browser**：`browser_read` 直接读取原文；`web_search` 显式关闭；无服务端综合层，
  直接分级加权（第一层+第三层，无第二层原文核验）；
- **off**：无检索。

禁止同一次检索任务混用两条通道；任何模式切换都必须产生机械 transition 事件（沿用 §3.7 条 1）。

## 3. 三层结构

`web_search`（framework_fallback）使用完整三层结构；`local_browser` 直接分级加权
（第一层+第三层），第二层不适用（`browser_read` 页面读取即原文）。

### 第一层：机械来源梯队（零额外请求）

- 输入：`web_search` 引用 URL（completed `open_page`，已排除失败页并去重）或
  `browser_read` 已读页面 URL；
- 白名单内 → weight **1.1**；白名单 = **政府与机关单位**（固定名单、从严），命中即满足
  权威性、子代理可直接采纳；
- 白名单外（默认档：普通来源，无需逐源分类）→ weight **1.0**；
- 劣质源 → weight **0.7**：初始名单含 **CSDN、知乎、百家号、哔哩哔哩个人专栏、
  微博、各家独立新闻媒体、自媒体新闻号与"XX财经"类自媒体号、小型个人站点**，另含
  黑名单/营销域名与个人新闻号/非认证号（平台/域名级可机械识别，账号级由第三层
  模型判断，URL 层无法机械识别）；
- 输出：每条来源的 mechanical weight + tier 标签，写入 evidence ledger。
- 语义：weight 是**相对排序乘数**（允许 >1），不是 0-1 置信度；1.1 表示白名单相对
  普通来源的优先乘数（直接采纳语义），后续如需概率/置信度语义须另设字段。
- 白名单为域名级：政府/机关单位官网（gov 类域名 + 机关事业单位官网），**不硬编码**，
  初始域名清单与配置入口在实现前落地（§6 待定项）；白名单命中不豁免原文证据纪律
  （ADR-0010 §3.7 条 5：未读全文不得生成全文级归因）。

### 第二层：选择性原文核验

- **禁止全量抓取引用 URL**（工具轮与网络成本约束）；
- 本层**仅适用于 `web_search`（framework_fallback）**；`local_browser` 不使用本层
  （`browser_read` 读取即原文，直接分级加权）；
- **framework_fallback**：机械预筛 + 子代理初选后，仅对高价值或结论依赖的候选引用 URL
  用 `web_fetch` 抓原文核验；本地浏览器在该模式显式关闭，不混用 `browser_read`；
- 抓到的原文走**同一套加权**，并进入现有 evidence ledger / visibility 分级
  （ADR-0010 §3.7 条 4/5 语义：不得用综合文本/片段生成全文级归因）。

### 第三层：子代理模型加权标注

- 账号类来源（个人新闻号、非认证号）的认证状态判断（URL 无法机械识别）；
- 每条来源输出：weight + 理由 + 采纳/标注状态；
- v0 语义：**标注 + 排序，不硬拦截**——结论优先采信高权重来源，低权重来源可用但必须
  显式标注；台账积累后再决定是否增加拦截阈值。
- 状态语义（producer 与 Python verifier 同一机械规则，2026-08-13 审查修复登记）：
  `annotated`（低质量显式标注）必须 weight=0.7；`adopted`（结论采纳）必须 weight≥1.0；
  机械 `low_quality` 来源不得标 `adopted`；同一 source_id 的重复标注丢弃并记
  filtering_log；被引用的 `low_quality` 来源缺少合法 `annotated` 标注时，整个结构化
  结果按验证失败显式降级（不静默丢弃）。

## 4. 共享判定器

- 一个来源加权判定器同时服务 `framework_fallback` 与 `local_browser` 两个模式；
- evidence ledger / visibility 复用现有机制，不另造平行记账；
- 与 `web_search` 执行器**解耦**：不把模型判断塞进工具内部（semaphore/超时/重试约束），
  判断发生在检索子代理的结果形成阶段。

## 5. 边界

- 非授权层：不改变权限门/票据门（web_search 无票 D-12；web_fetch/browser_read 持
  network 票且子代理动作票绑定 activation D-13）；
- `output_text` 只作线索不作证据；引用 URL 与抓取原文才是可核验证据；
- 检索内容默认可能有毒：加权不替代注入防线，注入防护仍由权限/票据硬门禁承担。

## 6. 待定项（实现前确定）

1. 白名单/劣质源初始域名清单与配置入口——**已定**：机器可读名单落地为
   [`runtime/source-quality-seed-lists-v0.1.json`](../runtime/source-quality-seed-lists-v0.1.json)
   （二进制 include 为默认，`ORZ_SOURCE_WEIGHTING_CONFIG` env 覆盖替换；
   官方媒体 1.0、公众号与个人主页/博客入劣质源见种子文档 §3）；
2. 单次检索候选核验数量上限——**已定**：≤5 写进检索提示词合同（软约束，
   120 轮工具预算是硬背板）；
3. 结构化结果 schema weight/tier 字段——**已加**：source_entry 新增
   tier/mechanical_weight/weight_reason/model_weight/model_weight_reason/
   annotation_status，organized_response 新增 source_annotations，
   filter reason 新增 annotation_invalid（先 Schema/fixture/verifier 再 producer）。

## 7. 实施状态（GAP-SOURCE-WEIGHTING-IMPL，`implemented`；2026-08-13 闭合）

1. **机械来源梯队判定器**——`orz-assurance/src/source_weighting.rs`：
   三档 + 白名单后缀/显式域 + 劣质平台/媒体域 + URL 形态 + edu.cn 个人主页标记；
   输出 tier/weight/reason 进 source_ledger；
   边界：当前加权面为 web_fetch/[SOURCE]/URL 形态 PDF 证据；web_search 引用 URL
   未透传 loop（审计 B-1）、browser_read 主车道无 ledger（审计 B-2），候选上限为
   软约束（审计 B-3）。
2. **web_search 第二层**——检索提示词合同：机械预筛 + 模型初选 + `web_fetch`
   候选原文核验，候选 ≤5、禁 `browser_read`（候选上限为软约束，见边界 B-3）；
3. **第三层模型加权标注**——`[RESULT_JSON].source_annotations` 校验后合并进
   ledger（model_weight/reason/annotation_status）并回写
   `organized_response.source_annotations`；非法标注（未知 source_id、weight 越界、
   缺 reason、status 非法、annotated≠0.7、adopted<1.0、low_quality adopted、重复
   source_id）丢弃并记 filtering_log；被引用 low_quality 无合法标注时整块显式降级；
4. **local_browser 直接分级加权**——共享同一判定器；`browser_read` 证据进入
   检索车道 ledger 时即加权（主车道无 ledger 的边界见审计 B-2）；
5. **结构化结果 schema**——weight/tier 字段已加（见 §6.3）；
6. **测试**——判定器单测 11 个（三档/名单/URL 形态）、web_page 加权 e2e、
   source_annotations 合并/丢弃/一致性/重复/降级 e2e、提示词二存一合同、
   Python verifier 9 条新规则测试、真实 journal 重捕；
7. **收尾**——三面审查 + 实施审计
   [`GAP_SOURCE_WEIGHTING_IMPL_AUDIT_2026-08-13.md`](audits/GAP_SOURCE_WEIGHTING_IMPL_AUDIT_2026-08-13.md)，
   索引状态 `pending` → `implemented`。
