# 252 批 τ²-bench 评测线立项与指标口径审计（2026-10-11）

> **批次**：252（前档＝[`251 批 R3 SCBench 官方口径首题`](251_R3_SCB_P01_CFGPIPE_OFFICIAL_RESET_2026-10-11.md)）。
> 领号说明＝原拟领 251，落档时发现 251 已由并行轮占用（R3 SCBench 首题），故改领 252。
> **日期**：2026-10-11。**用户令**：「后续就跑τ吧」／「请落一份审计文档吧」。
> **性质**：选型与口径审计——**零源码、零载体、零记账改动**（索引／TODO／BACKLOG 未动，登记另行裁决）。
> 前件＝同批选型审计 [`EVAL_SELECTION_GAIA_AIDER_TAU_2026-10-11`](EVAL_SELECTION_GAIA_AIDER_TAU_2026-10-11.md)。
> **结论先行**：τ²-bench 为本轮选定评测线；**k=1 不适用**（该基准的设计目的即否掉单次跑法），
> 官方可靠性口径为 **pass^k**、论文与官方博文使用 **k=8**；建议对外 **k=8**、内部摩擦 **k=3–5**；
> 成本量级 k=8 下约 **¥48–240**；提交面为社区驱动（带轨迹经独立验证标 `verified`），
> 但**官方提交要求的 k 值未核实**，列为 S0 第一项。

---

## §0 背景与前件裁决

选型审计（同日）对本批三候选的裁决：

1. **GAIA：降为自跑专用**——HF 提交有账号活跃度门槛（本项目账号不满足），且 Princeton HAL 榜已停更；
   结果无法进入公共榜，只剩自跑用途。
2. **Aider Polyglot：不采纳**——用户裁决「GPT-5 跑 88%，模型能力代差太大，跑了也没用，没办法干净归因」。
   （边界注记见前件 §9.5：该 88.0% 系 2026-03 快照，榜上未见 DeepSeek V4.x 读数。）
3. **τ²-bench：选定先行**——本批对象。

本批目的：把 τ² 这条线从「选定」推进到「口径可执行」，即钉死指标口径、成本量级与 S0 待核项。

---

## §1 立项对象

- **名称**：τ²-bench（tau-squared-bench），前身为 τ-bench（Sierra，2024-06；ICLR 2025）。
- **来源**：Sierra Research，仓库 `sierra-research/tau2-bench`。
- **形态**：**双控制**环境（用户亦可调用工具、改变世界状态），Telecom 域建模为 Dec-POMDP；
  配组成式任务生成器与可靠用户模拟器。
- **域**：airline／retail／telecom／banking（含 knowledge 子域）。
- **能力面**：工具调用＋策略遵循＋知识检索（约 700 文档知识库检索后应用政策）＋人机协调沟通；
  提供「推理错误 vs 沟通／协调错误」的细分归因。
- **评分**：判分看**最终世界状态**（数据库与要求输出对注释目标比对），不看对话文本。
- **版本纪律**：v1.0.1 判分更新（修 banking_knowledge 题错）——**跨版本结果不可比**，须 pin。

---

## §2 指标口径（本批核心）

### §2.1 pass^k 定义

τ-bench 的招牌指标是 **pass^k**：**同一任务连续 k 次全部成功**的概率。它度量**可靠性**，
不是平均表现，也不是 pass@k（k 次中至少一次成功）。

- 官方博文（Sierra）：「即使 GPT-4 级 SOTA agent，**重复同一任务八次**时成功率只剩约 **25%**」
  ⇒ 官方可靠性读数的口径为 **k=8**。
- 原论文（ICLR 2025）引入 pass^k，明确指出「平均分好看」的 agent 在重复跑时极不稳定。
- 榜单注记：**trial 数会改变 pass^k**，跨榜比较必须对齐试次数。

### §2.2 结论：k=1 不适用

在 τ 这条线上跑 k=1 等于自废指标——k=1 的 pass^k 退化为普通单次成功率，
而这正是该基准被设计出来要否定的东西。**至少须按 pass^k 形态报告**。

### §2.3 建议口径

| 目的 | k | 说明 |
|---|---|---|
| 对外可比／上榜 | **k=8** | 对齐官方 pass^8 口径 |
| 内部摩擦与回归 | **k=3–5** | 仍需按 pass^k 形态报告，不得只报平均分 |

**报告要求**：必须写明 k 值、用户模拟器配置与版本 pin。否则数字不可比。

**易踩的坑**：pass^k 是「k 次全过」的概率，**平均 80% 与 pass^8=20% 可以同时成立**。
后者才是暴露稳定性的读数——这与本项目「找摩擦、要干净归因」的目标同向。

---

## §3 成本量级

### §3.1 内账标尺（一手，项目既有轮次实收）

| 轮次 | 规模 | 实收 | 单位成本 |
|---|---|---|---|
| SCBench V1（0cr） | 36 题／196 档 | ¥157.49 ≈ $22.0 | ¥0.80/档、$0.112/档 |
| SCBench V2（0cw） | 196 档 | ¥212.41 ≈ $30.4 | ¥1.08/档 |
| TB 2.1 | 89 题／33.8h | 账面无 RMB | — |

由 SCBench V1 反推混合单价 ≈ **¥0.109/百万 token**（低谷价、缓存命中 47.9%）。
TB 2.1 未记 RMB 花销，为跨线对照缺口（前件 §9.1 已登记）。

### §3.2 τ² 估算（派生，非账面）

方法：成本 ≈ 跑量 × 每题 token × ¥0.109/M。主不确定项＝每题 token（含**用户模拟器另计一份模型调用**）。
题量待勘（按约 300 假设）。

| k | 跑量（题×k） | 估算 |
|---|---|---|
| 5 | 1,500 | ¥30–150 |
| **8** | 2,400 | **¥48–240** |

- τ² **不依赖 Harbor 容器编排**（需 API 数据库与用户模拟器），成本结构与 TB／SCBench 不同。
- 未决：用户模拟器是否复用同一模型与配额、是否计入同一次预算。

---

## §4 提交政策

| 项 | 状态 |
|---|---|
| 提交面 | **开放**——官方有 `docs/leaderboard-submission.md` |
| 榜单形态 | Sierra 新榜为**社区驱动**；带完整轨迹提交，经独立验证后标 `verified` |
| 独立榜 | OpenRouter（Airline 域连续跑）、Artificial Analysis（Telecom）、steel.dev、Codesota |
| **待核项** | **官方提交要求的 k 值**（本批三次取回失败：raw／CDN 返回空段、GitHub 页 HTTP 503） |

对照（前件）：TB 2.1 关闭社区提交（只收维护者）；GAIA 有 HF 活跃度门槛；SCBench 属数据集发布形态。

---

## §5 饱和度

| 域／来源 | 头部读数 | 判断 |
|---|---|---|
| Airline（OpenRouter，2026-10-10） | **80.0%**（Claude Fable 5） | 约 20pp 余量 |

- OpenRouter 自述选 τ² Airline 因其**地板高**、用于测 **provider／基建差异**而非模型能力
  ⇒ 对 harness 层差异敏感，与「找摩擦」目标同向。
- 其余域（retail／telecom／banking）饱和度未逐项取回，列 S0。

---

## §6 S0 勘定清单（按序）

1. **官方提交要求的 k 值**（本批唯一硬待核项）。
2. **版本 pin**：repo tag ≥ v1.0.1＋数据集 revision；注意判分口径跨版本不可比。
3. **题量与域清单**：airline／retail／telecom／banking 各自题数。
4. **判分器与用户模拟器**：模型依赖、是否可离线、模拟器配额与成本归属。
5. **可跑性**：是否需要容器、出网范围、是否可在无头环境运行。
6. **驱动脚本**：本项目为**非 Harbor 系**，须自建 runner（沿 0cj S2 先例）。
7. **与既有线分账**：与 SCBench／TB 线不合并统计。

---

## §7 与项目能力面的匹配

| τ² 面 | 对应项目面 |
|---|---|
| 工具调用与订单编排 | CLASSICAL-EXEC-ASSISTANT（操作台模型） |
| 策略遵循与拒绝 | `policy_denial`（FUS-CONSOLE-POLICY-DENIAL） |
| 知识检索（约 700 文档） | 检索车道、结构化结果（GAP-RETRIEVAL-STRUCTURED-RESULT） |
| 多轮协调与沟通 | 会话面、提醒投递、中性终态 |

长期看，τ² 的 pass^k 读数可与本项目的会话连续性／稳定性工作（0cy 会话卫生、0cz 上下文主控）
形成对照——两者都在问「同一任务重复跑，是否稳定」。

---

## §8 边界与未决项

- **未做**：任何真机跑批、任何源码／载体改动、任何记账文件改动。
- **未领记账**：索引／TODO／BACKLOG 登记未做（本批仅落档）；是否立项编号与登记待裁决。
- **未核实**：官方提交 k 值；retail／telecom／banking 域饱和度；τ² 题量精确值。
- **来源分级**：Sierra 官方站／仓库、OpenRouter／Artificial Analysis 榜为一级；
  qaskills／benchlm／steel.dev 为二级（交叉核对用）。

---

## §9 来源

一级（官方／论文）：

- 仓库与提交文档：<https://github.com/sierra-research/tau2-bench>
- 官方站：<https://taubench.com/>
- 原论文（τ-bench，ICLR 2025）：<https://arxiv.org/abs/2406.12045>
- τ² 论文：<https://arxiv.org/pdf/2506.07982>
- 官方博文（pass^8 出处）：<https://sierra.ai/blog/tau-bench-shaping-development-evaluation-agents>
- 榜单：<https://openrouter.ai/benchmarks/tau2-bench-airline>；
  <https://artificialanalysis.ai/evaluations/tau2-bench>；<https://leaderboard.steel.dev/leaderboards/tau-bench/>

二级（聚合，交叉核对）：

- <https://qaskills.sh/blog/tau-bench-agent-evaluation-guide-2026>
- <https://hippocampus-garden.com/pass_k/>
- <https://benchlm.ai/benchmarks/tau-bench>
