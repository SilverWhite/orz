# ORZ 上下文压缩机制重设计（2026-08-14）

> 状态：`implemented`（2026-08-14 S1-S4 闭合、S5 审查修复闭合、S6 复查文档对齐闭合；实施审计见
> `docs/audits/GAP_COMPACTION_REDESIGN_IMPL_AUDIT_2026-08-14.md`）
> 权威：ADR-0010 §3.6 / §14.10 / §14.14（v1.10 + v1.14 补写）；本文件是设计投影与实施入口，不新增与 ADR 冲突的语义。
> 取代范围：本文推翻并取代 2026-08-08「LLM 摘要否决（零模型摘要）」与「节奏压缩仅最终答案间隙」
> 裁决；存档材料 `存档/docs/implementation-history/INQUIRY_FIX_AND_BLACKBOARD_PARTITION_2026-08-08.md`
> 保持 provenance，不随本设计回填。
>
> v1.15 注（2026-08-14 用户裁决，实施已闭合）：「黑板 edit 窗口随压缩滚动（用后擦净）」
> 机制废止——黑板生命周期改按 plan epoch 轮换（见
> `BLACKBOARD_PLAN_EPOCH_DESIGN_2026-08-14.md` / ADR-0010 §14.15）。本文 v1.14 相关小节
> 保留为已实施状态记录；当前代码行为为 v1.15（压缩不再清空黑板，路径槽为本 plan epoch
> 增量，溢出指针指向 epoch 快照；实施审计见
> `docs/audits/GAP_BLACKBOARD_PLAN_EPOCH_IMPL_AUDIT_2026-08-14.md`）。
>
> v1.29 注（2026-08-18 用户裁决：B 定案，D1=(b)；ADR-0010 §14.29）：「五段模板摘要的
> LLM 调用」撤销——压缩改纯机械、零模型调用（账单对账：压缩摘要以独立系统提示词重付
> 整段视图 miss，22:17 复验 10 个账单请求无 journal 对应、额外 miss ≈ 676K、账单口径
> 命中率 89.71% <90%，且摘要调用两轮全部失败零产出）。五段结构/冷却/守卫/存档/事件面
> 保留；注意事项/后续衔接为固定机械占位（阶段 (c) HA 结构化事实聚合落地前由主模型按
> marker 回查入口自行承接）；事件 `mode=mechanical`；存档恒写入、marker 恒带 digest。
>
> v1.30 注（2026-08-19 用户裁决：D1=(c) 设计定稿，先设计、不直接动作；ADR-0010
> §14.30）：阶段 (c) HA 结构化事实聚合设计定稿（纯文档登记、未实施，见 §4.4）——
> 注意事项槽升级为 HA 结构化事实聚合（助理层唯一新增输出，机械、零模型、有界）；
> 后续衔接槽**不交助理层**（固定中性占位 + 回查入口，后续衔接由主模型自行判断，
> 避免机械性误导）；五槽结构/17K 上限/存档恒写入/marker 恒带 digest/schema 不变。

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
| 摘要冷却 | ≥2 模型轮（v1.14 审查修复：3→2） | 冷却按完成模型轮计数（含纯文本/门禁轮），避免长动作累积；160K/200K 双阈值仍是主要安全网；工具记录坍缩不受冷却限制 |
| 缩减守卫 | `min_compactable`=5K；缩减门 40%（`max_reduction_ratio`=0.6） | 守卫不满足时跨触发轮重试 ≤3 次，仍失败则强制执行一轮压缩并以 `guard_failed` 报告（v1.14 审查修复） |

- 触发测量：以 provider 实测 `usage.prompt_tokens` 为准（复用 A6 既有路径）；估算函数仅用于
  恢复预检与压缩目标选择。
- 兜底触发必须可被紧急机械路径执行：摘要失败/超时时，不允许停留在 200K+ 上下文继续请求（见 §6）。
- 守卫失败路径（v1.14 审查修复）：守卫不满足**不是**跳过条件——连续 3 个触发轮重试守卫
  （每轮之间上下文自然增长、守卫重估），仍失败则强制执行一轮模板压缩，完成后在事件与 marker
  中明确报告机制失败（`guard_failed`），由后续处理跟进；该路径不使用原始机械截断。

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
- 窗口滚动（v1.14 审查修复）：压缩成功后清空黑板 edit 窗口（用后擦净）；路径槽因此天然是
  "本窗口增量"，全量窗口路径由摘要存档承载，黑板上不再累积历史编辑（v1.15 已废止：黑板
  改按 plan epoch 轮换，见 `BLACKBOARD_PLAN_EPOCH_DESIGN_2026-08-14.md`；本条保留为
  v1.14 实施记录）。

## 4. 第二层：五段模板摘要（机械模式，零模型调用）

> v1.29 修订（2026-08-18 B 定案，ADR-0010 §14.29）：本层不再调用模型——原
> `summary_system_prompt`/`summary_user_prompt`/`parse_model_output`/重试循环全部
> 退役；注意事项/后续衔接为固定机械占位（阶段 (c) HA 结构化事实聚合落地前）。

### 4.1 模板结构（固定槽位）

| 槽位 | 来源 | 字符上限 |
|---|---|---|
| 目的 | 黑板/plan 机械填充（当前任务目的，随任务更新） | 3K |
| 计划 | 黑板/plan 机械填充（当前步骤与软约束） | 3K |
| 变动文件路径 | 黑板 edit actions 机械填充（路径+行范围+时间戳）；Top-40 条 + 5K 字符双上限，溢出指针指向本次摘要存档 | 5K |
| 注意事项 | HA 结构化事实聚合（阶段 (c) 已实施 2026-08-19，见 §4.4） | 3K |
| 后续衔接 | 固定中性占位（阶段 (c) 设计定稿：不交助理层，由主模型自行判断；见 §4.4） | 3K |
| 合计 | — | 17K |

- v1.15 注：路径槽语义随黑板解耦改为「本 plan epoch 增量」；Top-40 条 + 5K 字符双上限不变，
  溢出指针指向当前 plan epoch 快照（见 `BLACKBOARD_PLAN_EPOCH_DESIGN_2026-08-14.md` §5）。
- 不设「用户原问题」槽：最近用户消息保留在最近尾；目的槽取自当前黑板，避免旧语义惯性/混淆
  （用户纠错后新摘要天然反映新目的）。
- 摘要链：旧摘要只进审计存档（`.gsa/compaction/`），不继承语义；每次新摘要是「当前黑板目的 +
  本窗口增量」的全新产物；存档摘要带 digest，marker 只保留滚动单指针。

### 4.2 校验与失败处理

- 机械校验：五槽齐全、每槽 ≤ 上限、合计 ≤17K 字符、路径槽条目来自当前黑板编辑窗口（压缩后窗口
  滚动，天然为本窗口增量；v1.15 起改为本 plan epoch 增量，压缩不再滚动黑板，见
  `BLACKBOARD_PLAN_EPOCH_DESIGN_2026-08-14.md`）、digest 绑定。
- v1.29（2026-08-18 B 定案）：无 LLM 槽——校验/重试/退化门/`summary_incomplete` 终止态
  全部退役；存档恒写入（机械槽 + 占位），marker 恒携带真实 digest/路径。
- 存档写失败（v1.14 审查修复）：显式重试 ≤3 次；仍失败时事件带 `archive_write_failed=true`、
  marker 附"存档写入失败：摘要未落盘，需处理"，不再静默。
- 机械槽超限（路径 >5K 字符或 >40 条）：Top-40 + 「其余 N 条见本次摘要存档 <path>」指针；
  全量窗口路径由摘要存档承载。

### 4.3 机械模式约束（v1.29，2026-08-18 B 定案）

- 零模型调用：`run_template_compact` 不发起任何 provider 请求——压缩的缓存代价仅剩
  「marker 起的重写 miss」一次，且不再有换前缀的整段视图 miss 与失败调用的纯浪费。
- 事件：`context_compressed.mode=mechanical`、`summary_incomplete=false`、
  `summary_id/digest/path` 恒非空；schema enum 保留 `template_summary` 供旧 journal 回放。
- 口径：压缩零模型调用后，账单请求数 = 主循环 `model_output` 数——事件口径与账单口径对齐，
  可用账单 CSV 直接验证 DoD 命中率。
- fallback 紧急机械截断保留（D2-2）：200K 兜底触发时常规 drain 后再按
  `recovery_target_tokens` 截断，run 绝不滞留窗口之上。
- 事件 reason：`rhythm`（160K 普通触发）/ `fallback`（200K 兜底）/ `session_end`（会话结束
  自动压缩，v1.14 审查修复）。
- 审查处理（2026-08-19）：fallback 双段截断时「被压轮次」= 常规 drain +
  紧急截断之和（事件/存档/marker 同口径）；事件估计在 marker 插入后按实际
  数组重算（含 marker，与 schema「marker + preamble + recent tail」口径
  一致）。

### 4.4 HA 结构化事实聚合（阶段 (c)，2026-08-19 设计定稿，ADR-0010 §14.30；
2026-08-19 S1/S2 实施闭合，S3/S4 待验证）

> 状态：`implemented (S1/S2)`（2026-08-19 用户放行实施——S1 代码
> `render_facts_notes` 聚合 + `run_template_compact` 接线，S2 summary
> 单测 6 项 + 压缩 e2e 1 项 + 空态 e2e 断言；orz-loop 473 通过 / 0 失败、
> clippy 与基线一致；S3 Linux musl 重建与 S4 命中复验待验证）。
> 范围遵守用户既有边界：助理层不做任何动作、不调用模型、不做语义理解；
> 只把已经结构化登记的事实机械聚合进「注意事项」槽；「后续衔接」槽
> **不交助理层**。

#### 4.4.1 注意事项槽（助理层唯一新增输出）

- 数据源（全部为 controller 已机械写入的结构化记录，单一写者、零模型参与）：
  - `plan.steps`：`Failed(receipt_id)` / `Blocked` 的步骤（step id + 目标 +
    receipt_id）；
  - `exec.errors`：最近 5 条（每条截断约 200 字符）；
  - `actions.results`：最近 3 条失败 receipt（order_id / step / code /
    trace_id）。
- 排序：计划面失败/阻塞 → 执行错误 → 动作失败（计划面优先，影响最大）。
- 空时显示「（无注意事项）」，不再用「机械模式无模型槽位」措辞。
- 上限 ≤3K；超限截断，末尾给「其余 N 条见 blackboard_read 分区/摘要存档」指针。
- 同一失败事件可同时以「步骤行（receipt）」与「动作失败行（同 order_id）」
  两条呈现——计划面与执行面双视角、信息互补，属有意冗余（有界，接受；
  2026-08-19 全面审查登记）。
- 截断不对称：仅执行错误单条截 200 字符；步骤目标/动作 receipt 不单条截断
  ——单条超长（>3K）整行放不下时退化为仅指针（该条计入 N），由
  blackboard_read 回查恢复（已测；2026-08-19 全面审查登记）。
- 压缩内部失败（`guard_failed` / `archive_write_failed` / 外挂台账写入失败）
  继续走 marker 既有独立标注，不进本槽（避免重复）。

#### 4.4.2 后续衔接槽（不交助理层）

- 不聚合任何「当前步/下一步/待办」内容——避免限制或机械性误导（用户裁决
  2026-08-19；助理层不变量=不理解语义，任何机械建议都可能与主模型实际评估冲突，
  如计划修订后待办失效、当前步应重做而非推进）。
- 保持固定中性占位，措辞显式声明「后续衔接由主模型自行判断」，仅保留回查入口
  （`blackboard_read` 分区 + 摘要存档 + 外挂台账路径）。

#### 4.4.3 不变项与已知边界

- 不变项：零模型调用；五槽结构与 17K 合计上限；存档恒写入、marker 恒带 digest；
  schema 无变化（两槽仍是字符串）；仅 `summary.rs` 聚合渲染与
  `run_template_compact` 接线变化。
- 已知边界：同一 plan epoch 内多次压缩时 `exec.errors`/失败步骤会跨 marker
  重复——marker 为滚动单 marker，模型只见最新一份；重复的是仍然成立的事实，
  可接受（设计定稿裁决）。
- `exec.errors` 黑板侧无界累积（渲染取最近 5）为已知边界；控制器侧加保留
  上限属可选后续，不占计数（2026-08-19 全面审查登记）。
- 事实行内嵌换行时，指针腾位按行段挤出、N 按段计数——仅溢出且含换行时出现，
  有界可忽略（审查登记）。
- 实施路由：S1 代码（`summary.rs` 聚合函数 + `run_template_compact` 接线）
  ✅ 闭合（2026-08-19）→ S2 测试 ✅ 闭合（2026-08-19）→ S3 重建
  ⬜ 待验证 → S4 命中复验 ⬜ 待验证；登记于 BACKLOG 0c / TODO P0-0c。

## 5. 回查清单 marker（滚动单 marker）

- 每个压缩点后注入一个 marker（复用 `[前文上下文已压缩]` 前缀），内容=被压轮次范围、动作台账
  摘要、涉及文件/证据路径、摘要存档位置（`.gsa/compaction/<id>.md`）与 digest、`blackboard_read`
  分区范围。
- v1.15 起 marker 携带 `plan_epoch`，随恢复过滤保留（见
  `BLACKBOARD_PLAN_EPOCH_DESIGN_2026-08-14.md` §5）。
- 多次压缩只保留最新 marker（旧 marker 随内容进入存档），防止 marker 累积。
- marker 与白名单块必须通过恢复过滤保留（D3-1 前置）；marker 不进停滞守卫 ngram
  （既有 `is_injected_block_text` 语义保留）。
- v1.14 审查修复：marker 在守卫强制压缩时附"机制失败：缩减守卫连续不满足，已强制压缩，需处理"；
  存档写失败时附"存档写入失败：摘要未落盘，需处理"。

## 6. 恢复路径前置（D2-2 / D3-1）

- D2-2：恢复加载 conversation 后、首次请求前，用估算函数预检；估算超窗（有效输入预算 224K
  与兜底线 200K 取保守）时，复用整轮丢弃逻辑截到目标，preamble/白名单/最近轮保留，完整侧车
  保留审计，journal 记录恢复截断事件（前后估算、丢弃轮数）。
- 会话结束压缩治本（v1.14 审查修复；S6 复查补写触发阈值）：每次成功 run 结束（主车道与检索
  子代理一致；grill 除外）在全量消息估算超过 `session_end_trigger_tokens`（默认 160K）时，
  于 terminal 事件前、sidecar 写回前自动执行一轮压缩（`reason=session_end`、强制、不受缩减
  守卫约束），摘要 marker 随 sidecar 一起存档；恢复时直接加载固定摘要。D2-2 恢复预检保留为
  旧侧车/未走该路径的兜底。
- D3-1：恢复回写过滤只放行 marker（`[前文上下文已压缩` 前缀）与白名单块，其余注入块继续过滤；
  恢复后首请求必须可见 marker 与白名单。
- 顺序：D2-2/D3-1 先于压缩机制接线闭合。

## 7. 复用与内联边界（v1.14 修订，S6 复查对齐）

- 语义等价内联（S5 审查修复，2026-08-14）：工具配对安全选择、缩减守卫
  （`min_compactable`=5K / `max_reduction_ratio`=0.6）、用户查询保留与截断由 orz-loop
  自行承担（`action_ledger.rs` / `compact_messages`），不再依赖 orz-compaction crate；
  退化摘要拒绝为 ORZ 自定 300 等效字符门（CJK 表意字一字折算 2，替代 orz-compaction
  英文向 500 字符门）。
- 必须适配：模式=HistoryThenSteps 形态（旧前缀摘要+最近尾保留），禁用 FullReplace 默认；
  摘要模型=会话模型（DeepSeek V4）纯文本 chat 调用（无工具、120s 专用超时）；摘要 prompt
  替换为 §4.1 五段模板；宿主触发/持久化/事件由 orz-loop 承担。
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
- S5：审查修复（2026-08-14 用户裁决）——守卫失败重试/强制压缩 + `guard_failed`、会话结束压缩
  治本（`session_end`）、存档写失败显式重试报告（`archive_write_failed`）、退化守卫 300 等效
  字符 + 中文折算、黑板 edit 窗口滚动、冷却 3→2 模型轮、120s 摘要超时、路径槽 Top-40 双上限、
  Schema/verifier/fixtures 扩展——**已闭合**（v1.15 注：其中「黑板 edit 窗口滚动」已废止，
  黑板按 plan epoch 轮换，见 `BLACKBOARD_PLAN_EPOCH_DESIGN_2026-08-14.md`；本条保留为
  已闭合历史记录）。
- S6：二次复查处理（2026-08-14，用户要求处理复查全部问题）——§6/§7 与 ADR 口径对齐
  （session_end 160K 阈值补写、复用边界内联化）、schema/代码注释冷却残留修正、终止态 marker
  占位 digest 改显式"（未生成）"、fixtures 生成器回写对齐、chars/2 中文低估登记 P1 校准——
  **已闭合**。

实施详情见 [`GAP_COMPACTION_REDESIGN_IMPL_AUDIT_2026-08-14.md`](audits/GAP_COMPACTION_REDESIGN_IMPL_AUDIT_2026-08-14.md)。

## 10. 未决/边界

- 检索子代理的压缩触发与 activation 预算的交互——**已复核（2026-08-14）**：共享
  loop 同构触发；摘要调用不消耗工具轮预算、不计数 orientation 轮，互不干扰。
- marker 与摘要存档的 retention——**已实施**：`.gsa/compaction/` 纳入既有 7 天
  retention 清扫（rebuildable/audit → sweepable）。
- 路径槽 Top-N——**已定案并修订（v1.14 审查修复）**：N=40、按黑板编辑插入序（时间序）；
  溢出行指向本次摘要存档（黑板 edit 窗口随压缩滚动，不再承担累积查询；v1.15 已废止该机制，
  改为本 plan epoch 增量 + 溢出指向 epoch 快照，见
  `BLACKBOARD_PLAN_EPOCH_DESIGN_2026-08-14.md`）。
