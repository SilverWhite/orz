# 0ap 设计稿：压缩交互第九工具与滑块数可见化（`context_compress`；DESIGN-COMPRESSION-INTERACTION）

> 日期：2026-09-18；状态：**current-design（2026-09-18 用户审稿通过定稿；S0/S1 实施待放行）**；来源＝[`处理批报告 §1.7`](audits/FRICTION_INVENTORY_TREATMENT_2026-09-17.md)（FR-A06）＋ 0ap 立项批。
> 上游设计：[`AUTH-CONTEXT-DYNAMIC-SLIDER`](CONTEXT_SLIDER_V8_DESIGN_2026-09-16.md)（v8 主滑块/分块）／[`AUTH-CONTEXT-SOFT-GATE`](CONTEXT_SOFT_GATE_MODEL_PARTICIPATED_COMPRESSION_DESIGN_2026-09-15.md)（D3 模型参与压缩）。
> **用户裁决（2026-09-18，三点）**：①工具语义＝**直接常驻第九工具，工具面由八增长至九**（明确不取「压缩窗口轮换面」变体；用户确认工具面变动的坑〔0aj 权限桥 deny／探针面脱同步／工具名模式〕已知，按先例清单化硬防）；②计数口径＝同意（仅数量与估算 token，分块内容不流出模型面，advisory 不阻断）；③同步面＝同意（主面＝`blackboard_read` 响应头读时现算，搭水位标先例；不取常驻状态行，留档不采纳）。
> **用户否决（同日）**：候选②「工具结果尾徽标」——注入次数＝工具轮数×频次，TER 后均不可控 ⇒ 注意力干扰不可控；与 2026-08-21 `CONTEXT-SCAFFOLDING-PULL-REDESIGN` 退役的逐轮尾随注入 PUSH 形态同族。
> 性质边界：本稿只动**交互/可见层**；H1/T1、阶梯档位、折叠与驱逐、D3 窗口 ≤3 轮与统一出口、`context_compressed` 事件契约、现存 8 工具语义**全部不动**（「其他机制照旧」）。8 工具面冻结例外 +2 属设计变更，ADR-0010 §14 转录随实施批登记（沿 0ak §14.68 先例）。

---

## 0. 结论速览

| 项 | 定案 |
|---|---|
| 第九工具 | **`context_compress`**（终版，2026-09-18 用户定名；常驻主面） |
| 语义 | 模型主动请求开模型参与压缩窗口（D3 既有机制）＋响应自带滑块读数表；**不新增第二套摘要格式** |
| 摘要产出通道 | 照旧＝压缩窗口轮内模型 `[SEMANTIC_SUMMARY]` 输出块（D3 既有）；本工具只负责「知情发起」，不改摘要载体 |
| 计数口径 | 主滑块以外**未压缩分块数 N＋估算 token**（`context_scale` 已有 `blocks=N` 账）；只出数量与估算，分块内容不流出模型面 |
| 同步面 | 主＝`blackboard_read` 响应头（读时现算，搭【x.xM/10M】水位标先例）；辅＝本工具响应信封自带同表；I6 前缀纪律天然满足（响应面非常驻前缀面） |
| 权限/探针 | 内存类 arm（沿 0aj 先例）＋`READ_ONLY_EXEMPT_TOOLS` 单源表收录＋探针面 WORK_TOOLS +1（三处同步，S0 复核现值）；声明面分类护栏（遍历式）自动纳入 |
| 契约面 | **零新事件族、零 schema 改动、零 Python 镜像改动**（复用 `context_compressed` 与既有事件链；计数走响应文本面） |
| 防抖 | 窗口已在程中 ⇒ 调用 no-op 返回当前读数（`in_progress` 态，防连点）；读数不可得 ⇒ 中性说明返回（沿 Unknown 档文案纪律），不报错不阻断 |

## 1. 工具语义（主案）

**名称**：`context_compress`（终版，2026-09-18 用户定名；`^[a-zA-Z0-9_-]+$` 合规）。

**调用效果**：
1. 机械层校验（预算/尺寸/窗口状态）后，按 **D3 既有机制**在下一个安全边界开模型参与压缩窗口——`PendingCheckpoint::ModelCompression` ≤3 轮、`finalize_model_compression_close` 统一出口、`context_compressed{mode=model_summary, reason=model_selected}` 事件、四项原文定位指针**全部照旧**；
2. 响应信封**每次**携带滑块读数表（见 §3）；
3. 窗口已在程中 ⇒ no-op 返回当前读数＋`in_progress`（幂等防连点）；窗口开不起（守卫越线等）⇒ 返回结构化原因（复用既有拒绝/降级文案纪律），不停手不报错。

**不做什么**（边界）：
- 不携带摘要内容——摘要仍由窗口轮 `[SEMANTIC_SUMMARY]` 产出（禁第二套摘要格式；若未来要「带目标分块区间的结构化摘要写入」，牵动 D3 面与块轴，**留 v8 尾批另议**，本稿不做）；
- 不改窗口开窗的机械判据（阶梯/H1/T1/守卫照旧）——本工具只是把「模型知情后的主动发起」变成一等动作（现行 model_selected 压缩的触发路径为 S0 回查项 (a)，本工具与其关系＝收编为显式入口，S0 取证后若存在第二触发路径则二合一）；
- 无文件/网络/黑板外部副作用（纯内存压缩状态操作）。

## 2. 计数口径（已裁决）

- **定义**：主滑块（H 窗）以外、**尚未被摘要替换**的分块数 N＋其估算 token 合计＝「可压缩量」；已压缩分块不计入。
- **数据源**：折叠/分块状态既有账（`context_scale` 机械审计已带 `blocks=N`；分块表 0ah 实现更正批已在码）——**零新增记账**，读时现算。
- **纪律**：只暴露数量与估算；分块内容/原文不流出模型面（v8「仅分块、不流出模型面」不变）；advisory、fail-soft、**不联动任何硬门**（H1/T1/轮预算/资源门/orientation 全不接）。

## 3. 同步面（已裁决）

- **主面＝`blackboard_read` 响应头**：现头行水位标【x.xM/10M】旁增一段「滑块外 N 块 ≈ est K」——**读时现算**，无刷新调度问题（分块数只在驱逐/分块边界离散变化，粒度天然内禀）；响应面非常驻前缀面，I6 前缀纪律天然满足。
- **辅面＝`context_compress` 响应信封**自带同表（PULL 即得，模型发起动作的当轮就有读数反馈）。
- **不采纳留档**：常驻状态行（0am 轮次行先例）——用户未取；若日后要「不拉也可见」，沿该先例另批（离散渲染＋fail-soft＋锁序纪律现成）。

## 4. 工具面注册清单（8 → 9；0aj 教训点位化）

| # | 点位 | 动作 |
|---|---|---|
| 1 | controller `run_turn_inner` 工具注册处 | 常驻注册（无条件，沿 `blackboard_write` 注册形态）；工具描述**自包含教学**（≤120 字符目标，控常驻成本）＋提示词 ≤1 句教学（检索子代理先例） |
| 2 | 权限桥 `access_kind` | 新增内存类 arm（沿 0aj `blackboard_write` 先例；ReadOnly 类全策略自动放行） |
| 3 | `READ_ONLY_EXEMPT_TOOLS` 单源表 | 收录（遍历式护栏自动覆盖 risk_class 与代表参数表断言） |
| 4 | 探针面 `WORK_TOOLS` | +1 三处同步（23→24 为 blackboard_write 先例；S0 复核现值后 24→25 或按实值） |
| 5 | 声明面分类护栏 | 遍历式自动纳入（0aj 复核批机制），负例钉子补新工具样本 |
| 6 | journal/conformance | **零新事件族**（复用 `context_compressed`＋`tool_started/completed`）；Python 镜像零改动 |
| 7 | schema | **零改动**（计数走响应文本面/既有信封；若 S1 判定需结构化字段 ⇒ 契约漂移流程另行登记，不静默加字段） |
| 8 | 压缩窗口轮工具面 | `blackboard_write`＋`context_compress` 并存（窗口内两工具各司其职：黑板固化 vs 压缩发起/读数；S1 可调） |
| 9 | 工具名 | `^[a-zA-Z0-9_-]+$`（GAP-CONSOLE-TOOLNAME-PATTERN 纪律） |

## 5. S0 现行面回查清单（实施前取证，只读）

- (a) **model_selected 开窗现行触发路径**：run `RUN-CLI-6aac0af5` 4 次 `reason=model_selected` 压缩（est 230K–290K）的精确触发代码路径（`agent_loop.rs` 压缩窗口装配区）——本工具收编该路径为显式入口，若存在多路径则二合一；
- (b) **blocks=N 账源**：`context_scale.rs`／折叠状态中「未压缩分块」的现成判定与「已压缩」标记现状（若无标记则需最小补账——登记为 S1 实现项）；
- (c) `blackboard_read` 响应头水位标渲染点（`blackboard.rs` 读路径）；
- (d) 8 工具注册 census 与 `WORK_TOOLS` 现值复核（controller 注册块＋orz-host 探针面）；
- (e) 锁序纪律位：blackboard/LIF/压缩状态锁的获取顺序（沿 0am Part A 注记与 2026-08-31 B2 复审登记），新工具响应装配不得倒锁。

## 6. 判据与验收（S1–S3，各步独立放行）

- **S0 回查**：上列 (a)–(e) 取证落档（设计稿补 §附记或实施回执），无「符号在位≠接线」 assumptions；
- **S1 落码**：钉子至少四枚——①读数表与折叠状态一致性（blocks=N 对账）；②响应信封形态（含 in_progress/fail-soft 态）；③窗口轮并存与端到端链（调用→开窗→`[SEMANTIC_SUMMARY]`→收口→`context_compressed` 事件链）＝0aj 同形端到端钉；④权限桥放行链（ReadOnly 决策 allow）。读数＝orz-loop/orz-host 全量绿（**清 env 口径**：先清 `ORZ_*`/`GROK_HOME`/`GROK_AGENT`；FR-N02）＋`RUST_MIN_STACK=134217728`（FR-N03）＋orz-host 串行＋clippy 与基线持平＋fmt 干净＋单独工作树复核（ORZ-BUILD-MOUNT-001 挂载）；
- **S2 载体**：重建换装＋字面量核证（沿 052/061 先例）；
- **S3 狗粮实证**：滑块读数与 journal `blocks=N` 一致；模型可用新工具完成至少一次 model_selected 压缩；上下文/token 读数照收（不做严格 A/B，单轮如实标注——2026-09-16 口径）。

## 7. 开放与留档

- ~~工具名终版（S0）~~ **已定（2026-09-18 用户定名 `context_compress`）**；窗口轮并存形态微调（S1）；「带目标分块区间的结构化摘要写入」＝**不做**，与 v8 尾批块轴合流另议；
- 常驻成本登记：第 9 工具的 tool_defs 每请求常驻开销（描述控长后预计 ~百 token 级），S3 读数照收评估；
- 本稿不修 ADR-0010（转录随实施批）；不动 `FUS-TOOL-PROBE` 探针语义本身（只加声明集）。

> 维护口径：本稿为 0ap 的设计评估稿；主裁（§头部三点＋否决项）与工具名终版均经用户裁决（2026-09-18），**状态＝current-design**；实施回执落审计件后转录 ADR-0010 §14（8 工具面冻结例外 +2）。
