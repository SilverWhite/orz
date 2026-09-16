# LIF 动力学投影与轮次预算换算设计（设计草案 v1.0，2026-09-16）

> **性质**：正式设计草案（待用户裁决排期、立项编号与 ADR-0010 §14 转录；本文不回填 ADR）。
> **背景**：2026-09-16 专利查新线（五路检索，报告待立档）对 LIF 部件的结论——数学核心为
> 教科书级在先技术、stuck 通道 0/102 无效果证据、生产形态为"无消费者的参考系"。用户裁决
> 将 LIF 从纯观测面升级为机械层内的具体数学元素，并先行放行 P1（T̂ 轮次预算换算）。
> **裁决记录（2026-09-16，用户）**：
> ① 同意动力学投影的边界裁决与警告（连续特征可为元素、离散 domain 仅标注、0q 身份键不动）；
> ② P1（T̂ → 墙钟↔轮次换算）放行，约束：**明确给模型、仅仅只是粗粒度参考、不阻断**；
> ③ P2（卡死域→干预）、P3（deny→breaker）挂起登记为待细考虑项——明确干预模型动作的
> 设计，含预测化路线的开放问题与硬先例约束，见 §6.4；不在本设计范围。
> **上游**：[`机械层数学计算体正式设计 §3/§4/§6`](MECHANICAL_LAYER_MATH_CALCULUS_DESIGN_2026-08-30.md)（LIF 规格）/
> [`0q 失败事件管线`](0Q_FAILURE_EVENT_PIPELINE_DESIGN_2026-09-08.md)（漏斗与键边界）/
> [`黑板会话折叠`](BLACKBOARD_CONVERSATION_SCOPE_FOLD_DESIGN_2026-09-03.md)（(round,domain) 盖章先例）/
> [`压缩语言学形式层讨论`](COMPRESSION_LINGUISTIC_FORMAL_LAYER_DISCUSSION_2026-09-02.md)（domain 降键裁决）/
> [`上下文 PULL 重设计`](CONTEXT_SCAFFOLDING_PULL_REDESIGN_DESIGN_2026-08-21.md)（PUSH→PULL 纪律）。

## 0. 定稿范围

- **Part A（先行批，S1）**：T̂ → 墙钟↔轮次换算面。纯渲染扩展，零 schema、零契约面。
- **Part B（第二批，S2）**：动力学投影与盖章（Dyn）——把 LIF 时间特征作为确定性数学元素
  加入机械层的事件面、失败聚合面与 temporal 查询面。
- **不含**：P2/P3（挂起）；u_stuck 二阶通道的盖章（v1 不含，见 §6）；任何向模型的新注入机制
  （Part A 仅搭载既有 resident 状态行与 SESSION PULL 面）。

## 1. 继承的边界裁决（本设计不得违反）

1. **0q 身份键不动**：失败目标身份 `(kind, id)` 确定性 digest 是 funnel-v1 契约，
   `failure_agg_coverage` 法官族依赖；Dyn 只加上下文字段，不进键、不改漏斗形状谓词
   （「形状谓词集中一处、不扩不缩」）。
2. **domain 不进键**（2026-09-02 压缩语言学裁决的扩展适用）：LIF 域判定在切换中间部分存在
   已知误差，domain 在本设计中**只作标注列、永不作聚合键/身份键/去重键**。
3. **fires 无注入面不变**（机械层数学体 §4.6）：fires 仍仅内部留痕；Dyn 盖章与 fires 无关，
   取的是连续特征值，不引入任何新的模型面注入通道。
4. **PUSH→PULL 纪律**：不复活已退役的 `[TOOL_ROUND_BUDGET] REMAINING` 每轮尾随注入形态；
   Part A 搭载的两个渲染面均为既有面（resident `[任务状态]` 行与 `blackboard_read
   section=session` PULL 面）。
5. **契约纪律**：schema 先行 + Python 冻结镜像同批 + 法官族扩展 + grandfather 锚；
   针对本仓三次契约漂移前科（GAP-EVENT-SCHEMA-DRIFT、`mechanical_audit_update` 换枚举复发、
   GAP-EVAL-RESULT-SCHEMA-DRIFT）逐项列防（§3.4）。
6. **参数纪律**（§4.8）：一切阈值为语义推导常数；版本级拟合禁止；确定性舍入规则进契约。

## 2. Part A：T̂ → 墙钟↔轮次换算面（S1 先行批）

### 2.1 语义

```text
remaining_rounds_raw = WALLCLOCK_REMAINING_secs / T̂        （T̂ 为在线中位数节奏估计）
```

无新计算组件：T̂ 估计器已随 LIF 在线运行（有界 8–32 间隔缓冲、中位数、>300s 截尾、
钳制 [3s, 600s]、采样 ≥8 启用）。本部件只做**投影渲染**：把既有墙钟档换算成轮次量纲。

### 2.2 渲染面（恰好两处，均既有）

| 面 | 载体 | 可见性 | 修改 |
|---|---|---|---|
| SESSION PULL 面 | `[SESSION v0.1]`（`prompt.rs` `session_face_block_with_wallclock`，TER T1.8 形态） | 模型按需查询 | `WALLCLOCK_REMAINING` 行下追加一行 `WALLCLOCK_REMAINING_ROUNDS` |
| resident 状态行 | `[任务状态 v0.1]`（`prompt.rs` `STATUS_LINE_PREFIX`） | **常驻注入，模型必见**——满足「明确给模型」 | 追加一行轮次预算档（仅当墙钟已施加且 T̂ 就绪） |

其余面（journal、机械审查、黑板其他分区）不渲染本字段。

### 2.3 粗粒度与中性纪律

- **1-2-5 阶梯向下取整**：渲染值 `≤N`，N ∈ {1, 2, 5, 10, 20, 50, 100, 200, 500, 1000, 2000,
  5000, …}（×10^k 延伸），恒取 ≤ 真值的最大阶梯值——**渲染值永不高估剩余轮数**（保守方向）。
- **静态免责短语**：行尾固定缀 `(coarse estimate; long tool rounds consume faster)`——
  T̂ 是中位数而决策间隔呈双峰重尾（典型轮 3–8s vs 长工具轮 100–1000s），换算天然粗，
  本设计如实定位为参考量，不虚构精度。
- **中性事实、不给建议**：不出现「建议收尾」类措辞；决策留给模型（机械审查层「不给建议」
  纪律同源）。
- **取整方向教训**（0af）：单行内不混排会产生字面自相矛盾的口径（如「≤5 轮」与同面
  「remaining 30s」并存时须换算自洽——30s/T̂<5 则轮次行取 ≤2/≤5，以同一 T̂ 值计算）。

### 2.4 不阻断声明（用户约束的落点）

- 不产生任何拒绝信封，不接 `budget_insufficient` 预检，不与 `ORZ_MAX_WALLCLOCK` 硬超时、
  0z 资源门、orientation 阈值、轮预算硬门**任何一处联动**。
- 硬墙钟到期语义、`run_invalidated`、预算耗尽通知（`tool_round_budget_exhaustion_block`）
  全部维持原语义；本面是 advisory（与 SESSION 面「advisory, never a correctness premise」
  注记同格）。

### 2.5 fail-soft 与省略规则

- 墙钟未施加（`limit=None`）→ 不渲染轮次行（SESSION 面维持既有 `WALLCLOCK_LIMIT: none` 文案）。
- T̂ 未就绪（采样 <8）→ 不渲染轮次行（不虚构估计值）。
- T̂ 钳制边界（[3,600]s）触发时照常渲染（钳制值即契约值）。
- 任何计算异常 → 省略整行，不渲染占位符、不报错、不打断（fail-soft，与阻断面无关）。

### 2.6 实现落点与成本

- `prompt.rs`：`session_face_block_with_wallclock` 增加一个可选参数（预渲染的轮次行字符串
  或轻量 snapshot 结构体，避免 prompt.rs 依赖 LIF 内部类型）；`[任务状态]` 装配点追加一行。
- 调用方从 LIF 平面读 T̂ snapshot（O(1) 读数）。
- **零 schema、零 Python 镜像、零法官族变更**；token 成本 = 每渲染周期一行（~15 token），
  resident 状态行本为动态内容，无新增缓存失效面。

## 3. Part B：动力学投影与盖章（Dyn，S2）

### 3.1 数学定位

LIF 平面是事件驱动、确定性、零标签的计算：**给定 journal 事件流（类型、时间戳、顺序），
任意历史时刻的特征值可精确重算**（102-run 回放实证：与 0.05s 步长 ODE 参考最大偏差 0.006）。
据此定义：

```text
Dyn : Journal × Position → DynCtx        （确定性时间动力学投影）
DynCtx = { domain, u_err, u_prog, t_hat_s, err10, succ10 }
```

- **追溯 = 重放**：Dyn(任何历史坐标) 可由 journal 重放精确复原，不需要额外持久化特征序列
  （域骨架已由 `DomainSpike` 侧车持久化）。
- **标注同态**：DynCtx 作为项的附加标注分量参加信封装配，**不改变归约**——GetPut 律、
  pipe 归约、类型签名的判定全部不受影响；盖章失败（特征未就绪）时项照常构造，
  `dyn_ctx` 缺省（optional + `serde(default)`）。

### 3.2 DynCtx 分量（v1 定稿）

| 字段 | 类型 | 值域/舍入 | 说明 |
|---|---|---|---|
| `domain` | enum | `start/normal/pressure/low_progress/stuck`（闭枚举） | **仅标注**（边界裁决②）；首次成功前 = `start` |
| `u_err` | f32 | [0, θ_err=4]，3 位小数 | 错误压力电位 |
| `u_prog` | f32 | [0, 1]，3 位小数 | 进度新鲜度 |
| `t_hat_s` | f32 | [3, 600]，3 位小数 | 节奏估计钳制值 |
| `err10` | f32 | [0, 1]，3 位小数 | 近 10 工具事件失败占比 |
| `succ10` | f32 | [0, 1]，3 位小数 | 近 10 工具事件成功占比 |

- **确定性舍入**：盖章时按 fixd 运算（ milli-units 整数或 3 位小数定点）落值；重放侧按同
  一规则舍入——保证 §4 B1 的全等判据可成立。
- **u_stuck 不进 v1**：0/102 触发、无信息量，省字节；挂起重评（§6）。
- 数值字段不携带任何自由文本，脱敏漏斗无数值改写路径（secret 命中不可能），journal 链
  seal-ack 语义不受影响（GAP-JOURNAL-CHAIN-DOUBLE-SEAL 修复语义保持）。

### 3.3 落点三处

1. **盖章装配**（事件面）：`tool_completed`（及失败路径事件）新增可选字段
   `dyn_ctx: DynCtx?`，在 `stamp_failure` 漏斗同点/ToolCompleted 完成装配点写入
   （`host_exec/failure.rs`、`host_exec/tool_run.rs`）。缺特征（run 起始极早期）→ 缺省。
2. **失败聚合**（定位）：`FailureTargetRow`（`failure_agg.rs`）新增上下文**列**（非键）：
   `dyn_ctx_latest: DynCtx?` 与 `pressure_failures: u32`（落在 pressure/stuck 域的失败计数）。
   聚合由此可区分「同一目标反复失败于正常域」与「失败集中在卡住 episode」两类病因——
   机械审查层可渲染有坐标的陈述（渲染措辞中立，不给建议）。
3. **temporal 查询**（追溯）：`blackboard_read section=temporal` 增加 `at(selector)` 语义——
   `(journal run+seq | 台账 [seq] | round)` → 该时刻 DynCtx 状态行（重放重算，≤1 KiB 渲染
   上限沿用）。跨进程恢复：spike 侧车载入域骨架 + journal 重放复原特征，恢复后首个盖章
   与不中断重放对照一致（§4 B2）。

### 3.4 契约面（防漂移清单）

- **schema 先行**：`run-event` schema 增 `dyn_ctx` 可选字段定义（含值域注记），随批同步
  Python 冻结镜像 `_verify_*`；两处值域表述逐字同源。
- **法官族**：`journal-conformance` 新增 `dyn_ctx_wellformed`——锚存在时校验：`domain`
  闭枚举、各浮点值域、3 位小数、盖章事件族覆盖（锚存在 ⇒ 全部 tool 终态事件必带
  `dyn_ctx` 或显式 `dyn_ctx_absent` 标记，XOR 纪律沿 0q `failure_agg_absent` 先例）。
- **grandfather 锚**：`run_started` 增可选字段 `dyn_projection: "dyn-v1"`；缺省 = 旧 run
  不作 dyn 校验（行集纯增量零迁移，沿 0q `failure_pipeline: "funnel-v1"` 先例）。
- **枚举防复发**：`domain` 枚举变更视为契约漂移（`mechanical_audit_update` 换枚举前科），
  闭枚举钉子测试双向（schema↔Rust↔Python 镜像三处逐字对照进 CI）。

### 3.5 零注入不变

`dyn_ctx` 只进 journal、聚合行与 temporal PULL 查询面；**不渲染进任何 prompt 块**；
fires 出口语义不变（§4.6）。模型获得动力学坐标的唯一途径 = temporal 面按需查询
（与 Part A 的 resident 可见面分属两个纪律域：Part A 是信息面裁决放行的显式渲染，
Part B 维持零注入）。

## 4. 评估方案（102-run 离线回放语料，判据进钉子）

| # | 对象 | 方法 | 通过标准 |
|---|---|---|---|
| A1 | Part A 方向安全 | 逐决策轮比较渲染桶 `≤N` 与真值剩余轮数（按实际消耗回算） | 渲染值 ≤ 真值，100% 成立 |
| A2 | Part A 桶保真 | 真值所在阶梯桶与渲染桶一致率；按长工具轮占比分层如实报告 | 不设阈值，读数入审计（双峰分布下粗是设计属性） |
| B1 | Part B 追溯精确 | 全部盖章事件由 journal 重放重算 DynCtx，与盖章值逐字段对照 | **bit-exact 全等**（同代码路径同舍入规则） |
| B2 | Part B 恢复连续 | 侧车恢复场景 vs 不中断重放，恢复后首个盖章值对照 | 一致 |
| B3 | 覆盖法官 | 锚存在 run 的 tool 终态事件 `dyn_ctx`/`dyn_ctx_absent` XOR 完备 | 法官 0 错误 |

字节成本读数随批报告：每盖章事件 ~60–100 B（gzip 前），按 run 工具终态事件数量级估算
入账，不单独立项。

## 5. 阶段、排期与治理（待用户裁决）

- **S1**：Part A 落码 + A1/A2 钉子（零契约面，可独立提交）。
- **S2**：Part B——schema/镜像 → 盖章 → 聚合列 → `at()` 查询 → 法官族 + B1–B3 钉子，
  单批提交，门禁全绿。
- **S3**：双平台重建；**S4**：实机复验（读数搭下一轮狗粮/跑批 run 收取，不单开 run）。
- **排期位置**：建议 0ah S2 之后的独立批，不插队；**立项编号、放行批次、ADR-0010 §14
  转录时机均待用户裁决**。转正前本稿状态为设计草案，不进索引条目。

## 6. 已知边界与未决项

1. **换算天然粗**：T̂ 中位数对双峰重尾分布的代表性有限；长工具占比高的 run 偏差放大。
   方向安全（A1）保证偏差只会「少报剩余」，不会误导模型超耗。
2. **domain 切换误差**：标注列在切换窗口附近的值不可作精确断言使用；连续特征不受影响。
3. **u_stuck 挂起**：若后续出现触发样本或补做压力实验，重评入 v2。
4. **P2/P3 挂起（登记为待细考虑项，用户 2026-09-16 两次裁决）**：卡死域→干预、
   deny→breaker 收紧。**性质定性（用户）**：这是明确干预模型动作的设计，与 Part A/B
   的纯信息面分属不同慎重级。**类别分界（用户提出，本稿确认）**：Part A/B 中 LIF 是
   **滤波/量测**——τ 等分化参数作语义常数、只描述过去，沿用 §4.8 纪律即可；P2/P3 本质是
   **预测**——要对未来轨迹下判断，与现有参数（θ_stuck=1.5·T̂ 设计为近不触发的精度通道、
   0/102）用途不同，不能直接挪用。**开放问题（用户列出，原样登记）**：①是否需要引入
   邻接 LIF 项目的自有 ANN 公式（预测化路线）；②预测时间步（horizon）如何确定；
   ③确定 horizon 所需的实验设计；④公式引入与项目边界（ADR-0010 §2.2 自研准入、
   §14.60 自研面不膨胀、发表时序）联动。**未来任何 P2/P3 设计稿必须回答的先例与约束**：
   a. **FUS-STAGNATION 前科**（2026-08-22 退役）：跨轮停滞守卫因对正常长会话普遍误杀且
   从未拦截真实退化而整体退役——P2 必须论证为何不重蹈误杀覆辙，包括与生成期哨兵的
   分工边界；b. **可重放资格线**：机械层确定性/可重放是 journal 审计与 B1 判据的承重墙，
   任何在线自适应预测器若引入，其状态必须可序列化入侧车并可精确重放，否则不合格；
   c. **正类稀薄**：102-run 语料中卡住 episode 样本极少（stuck 0/102），预测实验需先经
   狗粮/TB run 补充数据再判可行性；d. **触发即行为塑造常数**：干预阈值比渲染阈值承担
   更高纪律负担（P1 渲染值错了只是参考，P2 触发错了改写模型轨迹）；e. **干预词汇表
   已有先例**（orientation 软门、TOOL_POLICY_BREAKER、TER 自动后台化），P2/P3 的新意
   仅在触发信号，不在干预动作本身——设计稿应显式复用既有干预动作，降低新增面。
5. **论文衔接**：本设计不构成论文内容承诺；B1 的确定性可重放性质、C1 打乱对照门方法学
   可作学术素材，发表内容切割与时机红线（滑块/资源门专利选项已按 2026-09-16 裁决放弃，
   公开自由度相应放大）另行讨论。
6. **Entry round 不入 DynCtx**：journal 位置即可推导，不冗余存储；查询面按需渲染。
