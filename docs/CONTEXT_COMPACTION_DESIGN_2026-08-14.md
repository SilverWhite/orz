# ORZ 上下文压缩机制重设计（2026-08-14）

> 状态：`implemented`（2026-08-14 S1-S4 全部闭合；实施审计见
> `docs/audits/GAP_COMPACTION_REDESIGN_IMPL_AUDIT_2026-08-14.md`）
> 权威：ADR-0010 §3.6 / §14.10（v1.10 补写）；本文件是设计投影与实施入口，不新增与 ADR 冲突的语义。
> 取代范围：本文推翻并取代 2026-08-08「LLM 摘要否决（零模型摘要）」与「节奏压缩仅最终答案间隙」
> 裁决；存档材料 `存档/docs/implementation-history/INQUIRY_FIX_AND_BLACKBOARD_PARTITION_2026-08-08.md`
> 保持 provenance，不随本设计回填。

## 1. 设计目标与边界

- 目标：在 LIF 研究类长任务中，以「明确记录 + 具体回查」替代「精细信息保留」；控制上下文增长、
  降低注意力稀释，同时保证压缩产物可审计、可回查、不引入不确定语义。
- 边界：本文只定义压缩机制；工具可用性探针、单轮注入预算（`ORZ_MAX_INJECT_TOKENS_PER_ROUND`
  默认 50K）、请求 header 留痕（v1.9）等既有机制不变。
- 三个 Agent（主 Agent + 内外检索子代理）使用同一策略（FUS-AGENT-TOPOLOGY）；检索子代理
  上下文通常远小于触发阈值，机制一致、触发按实测用量自然发生，实施 S3 时与 activation
  预算交互一并复核。

## 2. 窗口与触发（已冻结参数）

| 参数 | 值 | 依据 |
|---|---|---|
| 有效窗口 | 384K | DeepSeek V4 论文检索质量 MRCR/MMR 塌陷最低值（384K-512K 区间下沿；2026-08-07 校准注释 legal max=384K）；不做新 live probe |
| 普通触发 | ≥160K 实测 prompt tokens（≈42%） | 注意力与精确度相关，落在质量塌陷带之前 |
| 兜底触发 | ≥200K，无视摘要冷却 | 384K − 160K completion 预算 − 24K 余量；旧 250K 因 prompt+completion 窗口账不成立而废止 |
| 摘要冷却 | ≥3 工具轮 | 复用 Grok `min_steps_before_compact=3`；工具记录坍缩不受冷却限制 |
| 缩减守卫 | `min_compactable`=5K；缩减门 40%（`max_reduction_ratio`=0.6） | 复用 orz-compaction 守卫；防抖与成本护栏 |

- 触发测量：以 provider 实测 `usage.prompt_tokens` 为准（复用 A6 既有路径）；估算函数仅用于
  恢复预检与压缩目标选择。
- 兜底触发必须可被紧急机械路径执行：摘要失败/超时时，不允许停留在 200K+ 上下文继续请求（见 §6）。

## 3. 第一层：工具调用记录机械坍缩（零模型调用、无冷却）

- 触发：每个模型工具轮完成后（loop-top/批次间隙，不打断当前批次），对该轮内新增内容执行坍缩；
  不设冷却。
- 产物：一轮一条动作台账行——工具名、目标路径/URL、结果指针（`.gsa`/journal 产物路径或 digest）、
  非空最终回复原文；多结果轮按条登记。
- 纪律：
  - 协议安全：坍缩必须整轮或按工具配对整体处理，禁止产生孤儿 tool result（复用
    orz-compaction `select.rs` 工具配对边界语义；与既有 `compact_messages` 不变量一致）。
  - 审计双轨：完整工具调用/结果保留在 journal、conversation 侧车与 P4 审计面，模型上下文
    只保留台账行；台账行必须给出可回查指针。
  - 依赖完整性：被坍缩的工具结果若仍被后续引用且无持久化产物，禁止坍缩（保留有界尾部）或
    要求模型先落盘。
- 零模型调用：台账行由 controller 从结构化 tool 事件确定性生成，不调用摘要模型。

## 4. 第二层：五段模板摘要（LLM 调用，受冷却约束）

### 4.1 模板结构（固定槽位）

| 槽位 | 来源 | 字符上限 |
|---|---|---|
| 目的 | 黑板/plan 机械填充（当前任务目的，随任务更新） | 3K |
| 计划 | 黑板/plan 机械填充（当前步骤与软约束） | 3K |
| 变动文件路径 | 黑板 edit actions 机械填充（路径+行范围+时间戳） | 5K |
| 注意事项 | 模型生成（derived_unverified） | 3K |
| 后续衔接 | 模型生成（derived_unverified） | 3K |
| 合计 | — | 17K |

- 不设「用户原问题」槽：最近用户消息保留在最近尾；目的槽取自当前黑板，避免旧语义惯性/混淆
  （用户纠错后新摘要天然反映新目的）。
- 摘要链：旧摘要只进审计存档（`.gsa/compaction/`），不继承语义；每次新摘要是「当前黑板目的 +
  本窗口增量」的全新产物；存档摘要带 digest，marker 只保留滚动单指针。

### 4.2 校验与失败处理

- 机械校验：五槽齐全、每槽 ≤ 上限、合计 ≤17K 字符、路径槽条目必须真实存在于该窗口动作台账、
  digest 绑定、derived_unverified 标记。
- LLM 槽超限/缺失/退化：拒绝并重做 ≤3 次（复用 Grok 退化摘要拒绝与重试参数：超时 120s、
  max_attempts 2-3）。
- 终止态（重试仍失败）：保留机械段（目的/计划/路径）+ 扩大最近尾 + marker 标
  `summary_incomplete` + journal 记录；不静默截断、不卡死 run。
- 机械槽超限（路径 >5K 字符）：Top-N + 「其余 N 条见存档摘要 <id>」指针；全量路径在黑板上可查
  （索引化，非内容截断）。

### 4.3 摘要调用约束

- 摘要模型：会话模型（DeepSeek V4）覆盖 orz-compaction crate 默认 grok-4.20；纯文本 chat 调用，
  无工具面，不进入探针/压缩回路。
- 摘要输入 = 已坍缩的历史前缀（工具记录已先被第一层处理），控制调用成本。
- 输出契约：`derived_unverified` + digest；prompt 明确「只能引用台账/黑板中可验证事实，
  推断性内容标注未验证」。

## 5. 回查清单 marker（滚动单 marker）

- 每个压缩点后注入一个 marker（复用 `[前文上下文已压缩]` 前缀），内容=被压轮次范围、动作台账
  摘要、涉及文件/证据路径、摘要存档位置（`.gsa/compaction/<id>.md`）与 digest、`blackboard_read`
  分区范围。
- 多次压缩只保留最新 marker（旧 marker 随内容进入存档），防止 marker 累积。
- marker 与白名单块必须通过恢复过滤保留（D3-1 前置）；marker 不进停滞守卫 ngram
  （既有 `is_injected_block_text` 语义保留）。

## 6. 恢复路径前置（D2-2 / D3-1）

- D2-2：恢复加载 conversation 后、首次请求前，用估算函数预检；估算超窗（有效输入预算 224K
  与兜底线 200K 取保守）时，复用整轮丢弃逻辑截到目标，preamble/白名单/最近轮保留，完整侧车
  保留审计，journal 记录恢复截断事件（前后估算、丢弃轮数）。
- D3-1：恢复回写过滤只放行 marker（`[前文上下文已压缩` 前缀）与白名单块，其余注入块继续过滤；
  恢复后首请求必须可见 marker 与白名单。
- 顺序：D2-2/D3-1 先于压缩机制接线闭合。

## 7. 复用边界（orz-compaction crate）

- 直接复用：`select.rs`（工具配对安全选择）、`min_compactable_tokens`、`max_reduction_ratio`、
  退化摘要拒绝（`MIN_SUMMARY_SEED_CHARS`=500）、采样超时/重试、用户查询保留与截断、
  `compaction_version` 审计字段。
- 必须适配：模式=HistoryThenSteps 形态（旧前缀摘要+最近尾保留），禁用 FullReplace 默认；
  摘要模型名覆盖为 DeepSeek V4；摘要 prompt 替换为 §4.1 五段模板；宿主触发/持久化/事件由
  orz-loop 承担。
- 不引入：Grok 的 two-pass 预热摘要与 memory flush（额外模型调用，本期不做）。

## 8. 推翻与保留（治理清单）

推翻：
- 2026-08-08「LLM 摘要否决（零模型摘要）」（INQUIRY_FIX_AND_BLACKBOARD_PARTITION §5 A6/§8）；
- 2026-08-08「节奏压缩仅最终答案间隙一次」；
- A6 参数集 160K 触发 / 90K 目标 / 20 轮冷却 / 250K 兜底（新参数见 §2）。

保留：
- 轮/工具配对纪律（D2-1）；preamble 与白名单优先级；`compaction_whitelist_add` 16K 字符上限；
- 50K 单轮注入预算与策略化读取（v1.9，第一道防线）；
- derived_unverified 审计契约与 digest 绑定（P4）；journal/侧车审计双轨；
- D2-2/D3-1 作为实施前置。

## 9. 实施切片（登记于 TODO/BACKLOG，待放行）

- S1：D2-2 恢复预检截断 + D3-1 marker/白名单恢复保留（含测试）——**已闭合**。
- S2：工具调用记录机械坍缩（动作台账行 + 配对纪律 + 指针完整性 + 测试）——**已闭合**。
- S3：五段模板摘要接线（orz-compaction 复用、模型覆盖、17K 校验、重做/终止态、
  `context_compressed` 事件 Schema 扩展 + verifier/fixtures）——**已闭合**。
- S4：实施审计、ADR/索引状态同步、README 表述更新——**已闭合**。

实施详情见 [`GAP_COMPACTION_REDESIGN_IMPL_AUDIT_2026-08-14.md`](audits/GAP_COMPACTION_REDESIGN_IMPL_AUDIT_2026-08-14.md)。

## 10. 未决/边界

- 检索子代理的压缩触发与 activation 预算的交互——**已复核（2026-08-14）**：共享
  loop 同构触发；摘要调用不消耗工具轮预算、不计数 orientation 轮，互不干扰。
- marker 与摘要存档的 retention——**已实施**：`.gsa/compaction/` 纳入既有 7 天
  retention 清扫（rebuildable/audit → sweepable）。
- 路径槽 Top-N——**已定案**：N=40、按黑板编辑插入序（时间序）；溢出行给
  blackboard_read 分区指针，全量路径索引化不截断。
