# 模型主控上下文设计稿——上下文管理常驻主工具＋黑板全量编辑＋机制兜底联动提醒

> 版本 v1.0（S1 设计稿，待放行）；日期 2026-10-11；立项权威＝[`236 批档`](audits/236_0CZ_MODEL_CONTEXT_CONTROL_2026-10-11.md)（用户两段裁决＋S1 三要点）。
> 本文是 `MODEL-CONTEXT-CONTROL`（0cz）的 S1 设计权威草案；放行后按 §10 批序进入 S2 落码。

## §0 定位与哲学（用户裁决，转录）

上下文是模型所能看到的一切——模型不应被控制上下文，而应**控制上下文**。开放的是主动性
（随时、随地、自选方式与形态），机制本身也是服务于模型的。分工：

- **模型管「看什么」**：主动清零/压缩（它的眼睛）＋黑板全量编辑（它的笔记）；
- **`.gsa` 管「什么是对的」**：客观不可编辑的连续性与科学性承载——journal / LIF / RLI /
  ？？？/ 机械层全依赖 `.gsa`（runs / snapshots / compaction / orientation 各目录＋写控保留面，
  236 批已核实在位）。

现有压缩机制（滑块 v8 阶梯：软提醒 → H1 硬打断 → T1 必定压缩 → 700K 守卫）**全量不退役**，
转为兜底；兜底触发时联动一步权限提醒。

## §1 canonical ID 与注册定位

- canonical ID：`MODEL-CONTEXT-CONTROL`（BACKLOG `0cz`，P1；236 批立项 57 → 58）。
- 新常驻工具：**`context_manage`**（第十一主工具，见 §3.1 盘点）；名字单一源落
  `orz-assurance::tool_names`（`CONTEXT_MANAGE_TOOL_NAME`，沿 0ao 收敛纪律与 0ap 同批三处先例）。
- 8 工具面冻结纪律：本工具＝**用户主导显式例外 +3**（blackboard_write +1〔0ae〕、
  context_compress +2〔0ap〕先例顺延），ADR 转录随 S2。

## §2 用户已裁要点（S1 前提，不重开）

| # | 裁决 | 出处 |
|---|---|---|
| ① | 上下文管理＝常驻主工具（序号＝第十一）；「清零」＝**移出视野而非销毁** | 236 批 §2-① |
| ② | 黑板全量编辑＝除 `.gsa` 以外的模型写入面全量；机械留痕不变 | 236 批 §4 |
| ③ | 现有压缩机制全量不退役转兜底；理由＝通道与物理（模型无自发弃置通道） | 236 批 §2-③ |
| ④ | 兜底联动一步提醒＝跟着现有压缩提醒走，改叙述、不加新注入点 | 236 批 §4 |
| ⑤ | 工具形态＝**单工具带 mode**（清零/压缩为一具的模式参数，不拆双工具） | 236 批 §4 |
| ⑥ | 护栏＝清零前置**强制交接写入**（板摘要先行，防裸清） | 236 批 §2-⑥ |

## §3 S1 现场盘点（2026-10-11，源码实证）

### 3.1 工具注册表现场与序号结论

主代理声明面权威名单（[`architecture/current/README.md`](../architecture/current/README.md) §2.1，
2026-09-18 用户裁决「冻结 10 工具、十工具地位平等」，注册路径史 ADR §14.72 第 11 条）：

`read_file`、`grep`、`search_replace`、`run_terminal_cmd`、`web_search`、`web_fetch`、
`blackboard_read`、`submit`、`blackboard_write`（0ae，例外 +1）、`context_compress`（0ap，例外 +2）。

装配路径（`orz-loop/src/controller.rs` ≈L3619 起）：基面＝宿主
`tools_registry().list()`（orz-host `ToolsetRegistry`）→ controller 顺序注入
`blackboard_read`（≈L3667）→ `blackboard_write`（≈L3818）→ `context_compress`（≈L3859，
含 0am FR4 `whitelist` 参数）。plan_write / submit 等其余工具按门休眠或经配置裁剪，
生产主面即上列 10 具。

**序号结论**：现役 10 具 → 新工具入列＝**第十一主工具**，与用户裁决吻合。

**在役压缩交互面重叠（开放项 O1）**：新工具带 `mode=compress` 后，`context_compress`
（第九批注册、现第十位）与之功能重叠——即用户「双具冗余」关注点的现成实例。两案：

- **方案 A（推荐）**：`context_manage` 入列第十一；`context_compress` **同批退役吸收**
  （声明面移除；压缩管线、journal 历史族、`whitelist` 参数由 `context_manage` 承继；
  WORK_TOOLS 三处同批先例：判官表＋探针面＋Python 镜像，0aj/0ap 双先例可照抄）。
  净面维持 10；「第十一」＝注册史序数，净计数不变如实登记。
- **方案 B**：`context_compress` 保留，`context_manage` 纯增量入列（面 11 具）。
  压缩通道出现双入口，与「双具冗余」裁决动机相悖，仅在「缓存面/外部习惯连续性」
  有实证价值时才选。

O1 随本稿放行一并裁决（默认按方案 A 执行）。

### 3.2 压缩机制现状（`context_scale.rs` / `model_face.rs` / `compact.rs`）

- **两个面**：本地面（`messages` 逐字全量，压缩只追加 restore-retained marker，永不 drain，
  不变量 I3）；模型面（投影层）＝前置＋固定指针＋各分块（原文｜机械摘要行）＋主滑块 x
  （160K 估算，永不被压缩/截断，不变量 I1）＋尾部（D4 段＋分块表）。
- **阶梯**（模型面估算刻度）：R1 192K / R2 256K 软提醒（每档每会话一次）→ **H1 320K**
  硬打断（压缩窗口 ≤3 轮）→ **T1 500K** 必定压缩（两级询问，第三次机械截断＝「仅保留
  工作现场，其余已闭合分块移出窗口」）→ 700K 守卫（异常保险，强制截断到线上）。
- **模型自压通道**：回复文本 `[SEMANTIC_SUMMARY…]…[/SEMANTIC_SUMMARY]` 块（六段结构，
  ≥2 段命中）⇒ 机械层下一 loop-top 按块压缩；`context_compress` 工具＝知情发起（三态：
  Requested / InProgress 防连点 / NothingToCompress 中性说明）。
- **回放面**：被压缩/截断的块逐字落 `{cwd}/.gsa/compaction/blocks/<session8>-b<k>.md`，
  marker 与分块表行带 `read_file` 分页指针——**「移出视野而非销毁」的既有物理承载**。
- **一次性键**：`context_scale_notified`（controller 侧 `Mutex<Vec<String>>`，随会话侧车
  持久化：prompt 起始注入、fire 时回写、新会话独立、恢复不重发）；T1 截断后**按越线
  重新武装**。

### 3.3 黑板写入面现状（`blackboard.rs` / controller 注册处）

- 模型可写三分区：`plan.model_notes` / `notes` / `findings`（0cu，机制照抄 notes）——皆为
  `Vec<NoteEntry>` **追加式**，经 `push_model_note` 写入，逐条 (round, domain) 盖章、
  分区版本计数（`revisions`）同落点 bump；单次 ≤8K；ReadOnly 风险类。
- plan 分区的结构化步骤（`PlanStep` 状态机 pending→in_progress→done/failed）由助手层
  机械管理，**不在模型写入面**。
- 折叠不灭：黑板不在上下文窗口内，跨压缩/截断/清零均不失效（`blackboard_read` 头带
  水位读数 【x.xM/10M】）。

## §4 工具规格：`context_manage`

### 4.1 参数面

```json
{
  "mode": "compress" | "clear",        // 必填；单工具双模式（用户裁决⑤）
  "whitelist": ["…"],                  // mode=compress 可选；0am FR4 承继（≤16 条、16K 帽不变）
  "handover": "…",                     // mode=clear 必填；交接摘要（纯文本，≤8K，防裸清⑥）
  "keep_recent_rounds": 0              // mode=clear 可选；默认 0＝仅留当前轮
}
```

- `mode=compress`：语义与响应与现 `context_compress` 完全同管线（知情发起 → 下一 loop-top
  开模型参与压缩窗口；InProgress 防连点返回读数；NothingToCompress 中性说明）。
  **压缩的「方式与形态由模型自选」**不受影响：工具发起与 `[SEMANTIC_SUMMARY]` 文本块
  两通道并存（0ap 定案不动）。
- `mode=clear`：见 §5。`handover` 缺失/空/超长 ⇒ 显式文本错误、**不生效**（fail-closed）。
- 风险类：**ReadOnly**（纯内存投影状态操作＋黑板合法写路径；无文件/网络外部副作用——
  §5.2 的块落盘属 `.gsa` 客观承载面，与既有压缩落盘同性质）→ 所有策略自动放行，沿
  context_compress 先例。
- 声明形态：无条件声明（沿 blackboard_write 注册形态）；描述自包含教学、控常驻长度
  （~140 字符，沿 context_compress 口径）；参数描述中文（沿 blackboard_write 同款）。

### 4.2 描述草案（自包含教学，S2 可微调）

> Manage your own context window. mode=compress: open a compression window (≤3 rounds)
> to fold older context; optional `whitelist` entries survive compaction. mode=clear:
> move the entire prior context out of view and continue on the ledger — requires
> `handover` (≤8K), which is written to the blackboard first. Nothing is destroyed:
> cleared content stays in .gsa and can be re-read. Returns a before/after readout.

## §5 清零机械语义（mode=clear）

### 5.1 投影层边界——第三动作

模型面收缩动作从两个扩为三个：模型自压（`[SEMANTIC_SUMMARY]`）、T1 硬截断、
**模型主动清零（`context_manage mode=clear`）**。

- 机械形态＝在会话追加一枚 **restore-retained 清零 marker**（`[前文上下文已清零 …]`，
  注册进 `prompt::is_injected_block_text` 注入文本纪律同款——既不写回持久化会话本体、
  又随侧车留存供恢复重建；与压缩 marker 同一机制，**零新侧车字段**）。装配器遇
  marker ⇒ 边界之前的全部分块与主滑块轮次不再渲染（含分块表重置），以 marker 内联
  存根替代（§5.4）。
- **不变量 I3 不动**：本地面 `messages` 逐字全量零改写，清零只发生在投影装配层——
  「移出视野而非销毁」由 I3＋`.gsa` 双承载。
- **不变量 I1（主滑块永不被压缩/截断）登记一处显式例外**：`mode=clear` 可将边界前的
  主滑块轮次一并移出。例外三条件＝模型主动发起＋强制交接写入前置＋范围以 marker
  边界为界可回放。ADR 转录随 S2。
- 现役轮（本次调用与响应）不在边界内，照常渲染。

### 5.2 回放面（照抄 T1 截断先例）

被清块逐字落 `.gsa/compaction/blocks/<session8>-b<k>.md`（同既有压缩/截断落盘路径与
陈旧性标注），marker 内存根带指针清单（「完整内容见〈路径〉，用 read_file 分页」）；
`.gsa` 会话卷（conversations/journal）本就逐字流式在案（0ak），0ct 读全开放可回读。
**开放项 O3**：块级落盘对 clear 是否冗余（会话卷已有全量）——默认**做**（与 T1 同契约、
模型回读成本最低），S4 实测后若零命中可裁。

### 5.3 水位与一次性键

- 清零后模型面估算坍缩至近静态 overhead ⇒ 软档一次性键（R1/R2）与
  `FLAG_FIRST_BLOCK` **重置**（新注意力周期理应重新获得软提醒；键集复位沿会话侧车
  既有通道，零新机制）。
- H1/T1 不需要显式重置：沿「按越线重新武装」语义自然生效。
- 700K 守卫、压缩窗口状态（若在程中）**不受 clear 影响**：窗口照常收口（clear 不豁免
  兜底义务——机制兜底与主动清零正交）。
- 白名单内容跨 clear 保留（压缩层结构，clear 是超集动作）。

### 5.4 marker 存根与响应信封

marker 内联存根（注入面，术语纪律：只用「当前上下文窗口／工作现场／台账」，不用
滑块等内部沿革词）：

> [前文上下文已清零 · 自你主动发起] 此前的上下文内容已整体移出当前窗口（未销毁，
> 逐字留档可回查）。你的工作计划与结论在黑板（blackboard_read），完整历史在 .gsa
> 会话记录（读工具可回查）。请从台账继续。

工具响应信封：清零前/后模型面估算读数（沿 0ap 读数表渲染器）＋被清块计数与回放
指针清单首行＋交接写入确认（落点分区与盖章轮次）。

## §6 黑板全量编辑

### 6.1 范围（用户裁决②的机械界定）

- 可全量编辑＝`plan.model_notes` / `notes` / `findings` 三分区（模型写入面全量）；
- 不可触碰＝`.gsa` 侧机械分区（edits/exec/actions/processes/temporal/session 等单写者
  分区、侧车、journal）与 plan 结构化步骤状态机（助手层管理）——**边界即「模型写入面」
  本身，无需新拦截**（blackboard_write 分区枚举本就不含机械分区；直接改 `.gsa` 文件
  仍被写控拦截，不变）。

### 6.2 形态：`blackboard_write` 增 `op` 参数（零回归扩展）

```json
{ "section": "plan|notes|findings", "op": "append|replace_all", "content": "…" }
```

- `op=append`（默认，缺省等价）：现状追加逐字不变（旧调用零回归）。
- `op=replace_all`：`content`（≤8K）原子整节替换该分区模型内容——三分区对应
  `plan.model_notes` / `notes` / `findings` 整体替换为新单条目；分区版本计数 bump
  （「恢复＝可见内容整体替换计 1」先例同款）；盖章照旧 (round, domain)。
- 细粒度（逐条删改）不做：整节重写已覆盖「模型自由整理笔记」语义，避免选择器参数面
  膨胀（沿 0cu「照抄不膨胀」纪律）。旧内容不丢——`op=append` 的历史条目全部在 journal
  与会话卷逐字在案。

### 6.3 journal 呈现（残余待决②的定案）

- **零新事件类型**：每次编辑（append/replace_all）＝既有 `blackboard_write` 工具调用
  事件链，params 全量入账 ⇒ journal 重放即审计轨迹（「机械留痕不变」的落实：留痕面
  是工具调用事件本身，不是新增对账结构）。
- `replace_all` 的调用事件在渲染面显示为「整节重写」条目（原条目数→新 1 条），读面
  不做隐藏——模型看到自己的整理动作，历史仍可经 journal/会话卷回查。
- 压缩边界承接、PULL 增量头（revisions bump）等既有消费面零改动（replace_all 即一次
  普通可见变化计 1）。

## §7 兜底联动一步提醒（残余待决③的定案）

落点＝现有提醒块函数各扩写**一句**（不加注入点、不加新块）：

- `soft_reminder_block`（R1/R2 软提醒）
- `compression_window_block`（H1 硬打断窗口块）
- `mandatory_compression_window_block`（T1 必定压缩两级询问块）
- `guard_truncation_notice_block`（700K 守卫截断告知）

扩写句草案（模型面术语纪律同款；若 O1 定方案 A 则工具名以 `context_manage` 为准）：

> 你拥有上下文管理权限：可随时调用 `context_manage` 主动压缩；也可在把必要结论固化到
> 黑板后直接清零当前窗口，仅凭台账继续工作。

- 一次性固化提醒（`first_block_reminder_block`）**不加**：它教的是「固化到黑板」，
  与权限告知无关，避免文案膨胀。
- 黑板 guide（0cl【名词】/【组件关系】）随 S2 增一句 context_manage 词条（属既有
  说明书的通用修正，非特化）。

## §8 与 0cy 的接口（残余待决④的定案）

- **代码级零耦合**：clear 不触发会话收卷/台账轮换——会话身份与 `.gsa` 台账连续性不变
  （清零是**对话内**的视野操作，不是新对话）。逐字上下文已随 0ak 会话持久化流式落
  `.gsa`，清零无需额外归档动作（S2 设一枚核实钉：clear 前后 `.gsa` 会话卷字节单调
  不减、无重写）。
- **依赖在测量面**：S4「清零后成绩维持」的干净读数依赖 0cy 卫生前置（否则重演 0cw
  跨档污染）——236 批既定依赖不变；0cy S4 真机核证含与 0cz 的联动复验点（既定）。
- orz 侧实现 0cy 时无需为本接口预留任何新原语：两线只在 S4 评测会合。

## §9 护栏汇总

| 护栏 | 机制 |
|---|---|
| 防裸清 | `handover` 必填非空 ≤8K；机械层**先**将交接写入黑板（notes 分区，条目带
  「交接摘要」标注与盖章），**后**施加边界；写入失败 ⇒ 清零中止（fail-closed） |
| 机制不退役 | 阶梯/H1/T1/700K 守卫/窗口义务零改动；clear 不豁免在程压缩窗口 |
| 双通道自压并存 | `[SEMANTIC_SUMMARY]` 文本块通道不动（0ap 定案）；工具发起与文本块等价 |
| 写控边界 | `.gsa` 直改仍拦；黑板机械分区仍不可写（枚举面既有边界，零新拦截） |
| 审计 | 每次清零/压缩/全量编辑均为工具调用事件链，params 全量入账，journal 重放可审计 |

## §10 批序与验收判据

- **S1 设计稿**（本稿）：残余四悬项定案（mode 参数面 §4 / journal 呈现 §6.3 / 序号盘点
  §3.1 / 0cy 接口 §8）＋开放项 O1–O4 列明 → **放行门＝用户**（含 O1 吸收裁决）。
- **S2 落码＋钉子**：`context_manage` 注册（tool_names 单源＋三处 WORK_TOOLS 同批）；
  clear 投影边界＋marker＋回放落盘；`blackboard_write op=` 扩展；提醒四块扩写；
  guide 词条。钉子：① 防裸清负例（无 handover 拒绝）② 清零后本地面零 diff（I3）
  ③ marker 恢复重建（侧车往返）④ replace_all 版本计数与渲染 ⑤ 提醒句单源一次
  ⑥ `.gsa` 字节单调核实钉（§8）。若 O1 方案 A：context_compress 退役面恰减一、
  journal 历史族保留。
- **S3 载体重建进体**：双平台、字节判据（声明面 +1／退役面 −1 或 +1，随 O1）。
- **S4 狗粮长轮实测**：「用不用／用得对不对」——使用率与时机（是否贴着 H1/T1 触发点）、
  清零后成绩维持（对照 235 批仅台账组近零损失基线）、handover 质量、与兜底触发交互
  （提醒→选择的分布）、replace_all 使用形态。**前置＝0cy S1+S2 落地**（236 批既定）。

## §11 边界与不做

- 不对外披露（Reddit/README 沿既定口径）；动力学深入探究不做（231 批 0cx 分工同款）。
- 不动 0cy（留给 orz）；本线不阻塞于 0cy 的仅 S2 之后的落地段（S1/S2 码面可先行，
  载体与真机随 0cy）。
- 不做细粒度笔记编辑（§6.2）；不做清零粒度选择器（§4.1 `keep_recent_rounds` 仅留
  轮数一个自由度，默认 0；再复杂的范围裁剪不做）。
- 机械层不替模型决定「何时」清零/压缩（沿「机械层不替模型收缩模型面」v8 纪律）；
  兜底只提醒不代劳。

## §12 开放项（随放行裁决）

| # | 悬项 | 本稿立场 |
|---|---|---|
| O1 | `context_compress` 吸收退役（方案 A）或双具并存（方案 B） | **推荐 A**（§3.1） |
| O2 | 软档一次性键随 clear 重置的幅度 | 重置 R1/R2＋first_block；H1/T1 沿越线重武装（§5.3） |
| O3 | clear 的块级回放落盘是否冗余 | 默认做（与 T1 同契约）；S4 零命中可裁（§5.2） |
| O4 | 提醒扩写句与工具描述终稿 | §4.2/§7 草案；S2 落码时按模型面文案纪律终审 |

## §13 关联与关键词

[`236 批档`](audits/236_0CZ_MODEL_CONTEXT_CONTROL_2026-10-11.md)（立项权威）／
[`CONTEXT_SLIDER_V8_DESIGN_2026-09-16.md`](CONTEXT_SLIDER_V8_DESIGN_2026-09-16.md)（兜底机制设计根）／
[`BLACKBOARD_CONVERSATION_SCOPE_FOLD_DESIGN_2026-09-03.md`](BLACKBOARD_CONVERSATION_SCOPE_FOLD_DESIGN_2026-09-03.md)（黑板会话作用域/存档）／
0cu findings 分区／0ct 读全开放／0ak 会话持久化／235 批证据底座（仅台账组 n=70）。

关键词：0cz S1 设计稿、context_manage、第十一主工具、单工具带 mode、主动清零、
投影层第三动作、清零 marker、移出视野非销毁、强制交接写入、防裸清、黑板全量编辑、
replace_all、兜底联动一步提醒、context_compress 吸收裁决、I1 显式例外、索引 v4.210。
