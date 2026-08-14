# 检索侧机械控制设计（2026-08-13，v0.1 定稿）

> 状态：`approved`（设计已冻结；用户已裁决放行实施；步骤 1-5 已闭合，
> 剩余步骤 6 实施中）。
> 范围：设计定稿；B-1 已实施闭合，批次剩余步骤按 BACKLOG P0-B 执行。
> 关联：ADR-0010 §3.7 条 12（来源加权）/ §3.7.9（引用纪律）/ §3.7.1（mode
> 门禁）；[`TOOL_AVAILABILITY_PROBE_DESIGN`](TOOL_AVAILABILITY_PROBE_DESIGN_2026-08-13.md)。
> 决策记录：2026-08-13 用户逐条裁决（web_fetch 计数反馈、机械预筛以 B-1 闭合为
> 前提、引用纪律机械化、检索工具不参与主探针）。

## 1. web_fetch 候选机械计数与计数反馈

目标：把提示词里的"候选 ≤5"软约束升级为机械硬门，并把计数结果反馈给模型，
让模型在已知预算下决定全量抓取还是关键词抓取。

### 1.1 计数语义

- 计数单位：按 **activation 累计**、按**去重后 URL** 计数（同一页重复读取不计新候选）；
- 计数时机：工具执行前由机械门禁先计数，先于任何抓取动作（2026-08-14 审查修复：
  门禁只做决策，计数消费在权限/ACAF 票据门禁通过后、执行前提交——被后置门禁拒绝的
  调用不消耗预算，其拒绝事件不携带计数字段）；
- 计数域：web_fetch 候选核验（framework_fallback）；local_browser 第二段读取
  （页面已打开时）复用同一计数域；
- 阈值：外置为 `ORZ_WEB_FETCH_CANDIDATE_CAP`（**2026-08-14 用户裁决定档
  8**；实施审计见 `docs/audits/GAP_RETRIEVAL_MECH_STEP2_WEB_FETCH_CANDIDATE_COUNT_IMPL_AUDIT_2026-08-14.md`），
  不再写死在提示词；
- 与预算一致：continue 重入不重置计数，仅 activation 关闭后新起（与工具轮
  预算的 session 累计语义一致）。

### 1.2 反馈与拒绝

- 未超限：工具结果携带计数反馈（"候选 N/M，剩余 K"），模型据此决定本次是
  全文读取还是关键词提取；
- 超限：拒绝执行，返回中性陈述（如"候选核验数量已达上限 X"），记事件，
  不给模型重试空间（与连续拒绝熔断同一处理面）；
- 拒绝形态与 mode 门一致：无 ToolStarted 的显式拒绝。

### 1.3 两段式（全量 vs 关键词）

- local_browser：页面已打开，门禁计完数后模型可决定全文读取或关键词提取；
  需要 `browser_read` 支持范围/模式参数（全文/预览/关键词提取）；
  > 契约（2026-08-14 步骤 4 定稿）：`mode` 可选枚举
  > `full`/`preview`/`keywords`，默认 `full`；`keywords` 为字符串数组
  > （仅 mode=keywords 必需，1..16 个非空、去重、每词 ≤64 字符）。
  > `full` 保持既有 ≤100K 字符读取；`preview` 返回前 4_000 字符
  > （`PREVIEW_READ_CHARS`，短页未截断时证据按全文处理，截断打机械页脚）；
  > `keywords` 机械提取命中上下文摘录（纯 ASCII 文本/词大小写不敏感、
  > 其余精确子串；每词 ≤3 段、半径 160 字符、总输出 ≤12_000 字符，
  > 恒 `truncated=true` 并打页脚），证据恒为 `partial_text_observed`。
  > 注（2026-08-14 用户裁决）：`browser_read` 是 host 路由工具（route=Host、
  > 不进 external dispatch 车道），但主 Agent 不执行检索任务——主车道模型
  > 可见投影移除 `browser_read`，local_browser 读取只由检索子代理执行；第二段
  > 计数域直接复用 web_fetch 的 per-activation 语义（activation 累计、去重
  > URL 计数、continue 重入不重置、activation 关闭清零），随子代理循环传参并
  > 回写激活侧车。步骤 4 按此实施（browser_read 范围/模式参数扩展 + 同一计数
  > 域门禁、拒绝码 `browser_read_candidate_*`、`tool_completed` 计数字段覆盖
  > browser_read），本裁决不影响 web_fetch 门禁。
  > 步骤 4 已闭合（2026-08-14）：browser_read 范围/模式参数与第二段计数域
  > 复用实施完成（契约见上；拒绝码 `browser_read_candidate_*`、
  > `tool_completed` 计数字段覆盖 browser_read、证据按 mode 降级、
  > Schema/verifier/fixtures 同步、ACAF e2e browser_read 场景迁移到子代理
  > 车道）。实施审计见
  > `docs/audits/GAP_RETRIEVAL_MECH_STEP4_BROWSER_READ_MODES_IMPL_AUDIT_2026-08-14.md`。
  > 复核修复（2026-08-14）：keywords 摘录正文严格 ≤12K（分隔符/省略号计入预算）、
  > 提取输入按 100K 截断并打页脚说明、工具定义补 `maxLength=64`、页脚 terms 数
  > 改为实际输出词数；候选计数消费改为权限/票据通过后、执行前提交（§1.1）。
- framework_fallback：无第二段（local_browser 专属）；web_fetch 的计数门禁
  仍适用，但只有"抓或不抓"。

> B-1 闭合（2026-08-13）：`web_search` 的引用 URL 已结构化透传进 loop——
> host 经 `ToolResult.structured` 转发 citations，controller 机械提取后写入
> 证据账本 `source_ledger[].candidate_urls` 与 `raw_source_refs` 投影
> （Schema `retrieval-result-event-payload-v0.2` 先行扩展，Python verifier
> 同步校验镜像一致性）。候选池为 metadata-grade，机械预筛（§2）将在其上
> 附加 tier/weight 与形态原因。

## 2. 机械预筛（候选池净化）

目标：提高每次 fetch 的命中率。原理是基数效应——候选池中"值得抓"的比例决定
模型同等判断力下的期望命中数；机械预筛用廉价、确定性的规则提升该比例。

### 2.1 前提

**B-1 已闭合（2026-08-13）**：`web_search` 的引用 URL（citations）已结构化
透传进 loop（`source_ledger[].candidate_urls` / `raw_source_refs`），机械预筛
现在有候选池可操作。B-1 闭合前，预筛只存在于提示词合同，不对候选池实际生效。

### 2.2 预筛规则（保守，只去掉明确差的）

- 来源档位（白名单 1.1 / 默认 1.0 / 劣质 0.7）作为**排序信号**，不是硬过滤；
- 去重：canonical URL / host 级去重；
- 已知失败形态：登录墙、超大页、坏 URL、重定向链、robots 排除；
- 相关性粗筛：标题/URL 与查询的关键词重叠（机械词法，非语义判断）。

### 2.3 边界

- 机械只净化与排序，**不拦截**；最终选择权在子代理模型（v0 标注排序语义不变）；
- 预筛规则必须保守，避免误杀：只移除"明确差"的候选，未知保持保留；
- 预筛结果（候选池 + 每候选 tier/weight/形态原因）进入结构化结果，可审计。

## 3. 引用纪律机械化（输出级校验）

目标：把引用纪律从"模型守约"变成"verifier 机械校验"，prompt 中的长段引用
规则可缩短，降低模型负担。

### 3.1 现状

已有机机械侧：ledger 绑定、visibility 分级、claim_strength 上限 verifier
（§3.7.5 矩阵）；缺的是**最终输出级**的引用标记校验。

### 3.2 新增：输出级引用校验器

- 扫描主 Agent 最终回答中的 `[来源: ...]` 标记（结构化解析，非 grep 文本）；
- 标记必须解析到合法来源：ledger `source_id`、可定位的 `path:line`、或
  URL/document identity + observed scope；
- 来源身份必须真实存在于本次检索/读取证据，不得凭空；
- 引用强度不得超过该来源可见性允许的 claim 上限（复用 §3.7.5 矩阵）；
- 校验失败 → 显式降级/阻止交付，返回机械 reason code；
- 校验位置：最终回答形成后、交付前（与 counterexample gate 同一交付边界）。

### 3.3 语义

- 标记仍是写入侧绑定，不构成验证；verifier 检查来源身份/可见性/claim 上限，
  不依赖文本措辞（ADR-0010 §3.7.9）；
- 校验器通过后，prompt 中的引用纪律文本可缩减为"标记格式 + 由 verifier 校验"。

> 步骤 5 已闭合（2026-08-14）：输出级引用校验器与交付边界接线实施完成——
> 主 Agent 最终回答在 counterexample gate 同一交付边界机械校验 `[来源: ...]`
> 标记（结构化解析、ledger source_id / 主车道 path:line / URL+observed scope /
> 文档身份绑定、§3.7.5 claim 上限），失败时以 `[CITATION_VALIDATION_FAILED]`
> 机械降级块替代交付并 journal `citation_validation`（reason codes + marker
> 明细，Schema/verifier/fixtures 先行，v0.2 事件枚举 42→43），通过时零事件。
> 边界：裸 `SRC-###` 仅绑定不强检、裸外部 URL 必须带 observed scope、预算耗尽
> 部分输出与跨 run restore 证据不参与本校验。实施审计见
> `docs/audits/GAP_RETRIEVAL_MECH_STEP5_CITATION_VALIDATION_IMPL_AUDIT_2026-08-14.md`。
> 复核修复（2026-08-14）：`SRC-###` 为 run 级唯一分配（跨同 run 多次 commit
> 不重号，消除 first-match 绑定歧义）；`[来源：...]` 全角冒号变体同样解析，
> 围栏/行内代码块中的字面标记跳过；URL 归一化复用共享 canonical 化并接受
> 主车道自身 web 证据；文档 `§` 锚点为定位符不参与 claim 上限；行界校验
> 读取交付时文件状态（TOCTOU 显式登记，身份仍为 observation-time）。

## 4. 与既有机制的关系

- mode 门禁（§3.7.1）已机械禁止检索通道混用，本设计不重复造门禁；
- 来源加权（§3.7 条 12）提供机械档位，预筛复用它作为排序信号；
- 检索工具不参与主探针矩阵（子代理确定性留痕），本设计是检索 lane 内部机械面；
- OPS-PROTOCOL（pending）覆盖更广的结构化操作审计，本设计不与其冲突。

## 5. 实施待办（登记）

> 优先级：P0（当前工作集）。实施待办已统一迁至 [`BACKLOG_AND_PRIORITIES.md`](BACKLOG_AND_PRIORITIES.md)（P0/检索机械控制批次，含 6 项实施序列、依赖顺序与 DC 信号并入建议）；本文件不再单独维护待办明细，设计内容仍以本文为准。
