# 机械层数学计算体正式设计（阶段 1 定稿，2026-08-30）

> 状态：`current-design`（2026-08-30 阶段 1 定稿；ADR-0010 §14.47 转录后由
> `pending` 转 `current-design`）。阶段 2 实施切片（I1–I6）与阶段 3 验证
> （V1–V3）待续，放行入账按既有纪律。
> 权威：自然语言设计权威 = ADR-0010（转录见 §14.47）；本文件为当前设计投影
> （阶段 1 产物），取代讨论稿的草案地位。
> 来源：讨论稿
> [`MECHANICAL_LAYER_MATH_CALCULUS_DISCUSSION_2026-08-30.md`](MECHANICAL_LAYER_MATH_CALCULUS_DISCUSSION_2026-08-30.md)
> （设计过程记录；其 §9 草案由本文件正式化，§9.7/§9.11 用户裁决为本文件约束）。
> 关联：MECHANICAL-AUDIT-LAYER（2026-08-24）／THIN-HARNESS-REDESIGN V1/V2
> （2026-08-27/28）／DEEPSEEK_CAPABILITY_AND_SUBTRACTION_DIRECTION_RESEARCH
> （2026-08-29）。
> 排期登记：BACKLOG P2-10 / TODO P2-10 / CLI_PROJECT_INDEX
> （AUTH-MECHANICAL-LAYER-MATH-CALCULUS）。

## 0. 定稿范围

阶段 1 范围 = F1–F6（TODO P2-10 勾选清单）：

| 项 | 内容 | 本文件章节 |
|---|---|---|
| F1 | 8 工具类型签名正式契约草案（Result 信封 / GetPut 律 / View / exit_code 值语义） | §2 |
| F2 | temporal 分区渲染规格正式化（运行时行 + 存档 spike + 查询面 + 渲染约束） | §3 |
| F3 | LIF 时间外挂计算规格（通道清单 / 特征集 / 闭式解 / T̂ / 四对照门） | §4 |
| F4 | 失败目标身份入事件面设计（目标身份字段 + trace→事件链同构核对） | §5 |
| F6 | 参考系重定位与观测面扩充定稿（双尺度基线 / 每轮校准 / P1–P3） | §6 |
| F5 | ADR-0010 转录（§14.47） | ADR-0010 |

阶段 0 决策（D1–D7，2026-08-30 用户确认）为本稿前置约束，不再复述裁决过程；
逐项明细见 BACKLOG P2-10 / TODO P2-10。

## 1. 总体形态（定稿）

三层结构：

1. **模型** = 调用者/决策者（固定策略，只发 term、只读表面；下一步动作决策是
   唯一保留给模型的成分）。
2. **类型化项重写系统** = 计算核心（纯函数确定性归约 + 效应过监视自动机；依赖图
   作为内部状态随归约维护）。
3. **LIF** = 全局时间监控外挂（独立于重写语义，挂最外面；只做时间特征计算）。

唯一耦合面 = **事件流**：重写系统每次归约、效应结果、模型轮均吐事件；LIF 消费
该流。

不变量：

- **term 不能写 LIF**；LIF 只读事件、只按自身动力学演化、只发 fires（运行起始
  配置除外）——守住 P2 观察/动作分离。
- **fires 仅内部留痕**（§9.7 用户裁决）：不入事件面 schema/verifier、不进模型
  表面、不渲染、不注入；round50 维持现有软门提醒形态，不扩展为强制模板轮。
- **模型无感硬边界**：一切计算全落机械层；模型只经
  `blackboard.read partition="temporal"` PULL 渲染结果（零常驻 token、无注入、
  无模板轮、无自动动作）。

模型可见面 = 类型化函数库（§2）+ temporal 分区（§3）+ 既有探针注入面；全部由
结构生成、非标定。

逻辑侧兄弟组件（不重叠、不实现于本阶段）：依赖图（重写系统状态内，锚点/实体边
随归约维护）+ LTL 时序验证（独立验证器，消费同一事件流）。LIF 负责"时间事件"，
两者负责"逻辑"。

## 2. F1 工具类型契约（正式契约草案）

### 2.1 共享类型声明（信封 / 指针 / 失败）

```text
type Offset   = { n: Int ≥ 0 }
type Length   = { n: Int ≥ 1 }
type Pattern  = Text（正则；编译失败 = 类型化错误值）
type Anchor   = { sha256: SHA256, size: Int }               -- GetPut 写前锚点
                                                              （运行时形状；初稿
                                                              {path, hash, size}，
                                                              2026-08-31 审查处理
                                                              F13 对齐实现；mtime
                                                              为工具 schema 可选
                                                              附加字段）
type Pointer  = FilePtr {path, hash, offset?}
              | BoardPtr {partition, selector}
              | EvidencePtr {canonical_url, fetch_id}
              | CmdPtr {log_path, span}
type Fail     = { step: arg_validation | gate | execution | delivery,
                  code: Symbol, message: Text ≤ 200 B, trace_id: TraceId }
type Result a = Ok { summary: Text ≤ 200 B, cap: Cap, payload: a, pointer: Pointer? }
              | Fail Fail
```

规则：

- 成功一律"摘要 + 上限 + 指针"三件套：模型读摘要做决策，全文经 pointer 按需取；
  cap 显式给出成本上界。
- 纯子项错误语义（D2）：参数校验/正则编译失败/no_match/ambiguous = 类型化错误值
  （Result 信封 Fail，`step=arg_validation`）；效应错误（未启动/超时杀/门拒绝）=
  fail-closed 信封不变；terminal `exit_code ≠ 0` 是结构化**值**，不是 Fail。
- trace_id 与事件面同构：归约留痕 = receipt，可直接映射到事件链（§5.4）。

### 2.2 8 工具类型签名

**① file.read（纯函数）**

```text
file.read : File → Offset? → Length? → Result View
View = { path, hash, size, window: {offset, content: Text ≤ 16 KiB, truncated: Bool} }
```

- 缺省 offset=0、length=cap；超窗截断并置 truncated=true，不静默裁剪。
- 律：返回的 (hash, size) 即写前锚点（search_replace 的 GetPut 前提）；同文件
  同状态同结果；解码按 GAP-ENCODING-GATE 契约（BOM 剥离 → UTF-8 严格 →
  GB18030 → lossy，命中编码随事件留痕）。

**② file.grep（纯函数）**

```text
file.grep : File? → Pattern → Path? → HeadLimit? → Result Grep
Grep = { pattern, files_searched: Int, matches: [Match ≤ 64], truncated: Bool }
Match = { path, line_no, span: {start, end}, text: Text ≤ 200 B }
```

- file 缺省 = 工作区；files_searched 是信封字段——模型看到"搜了多少"而非文件
  清单爆炸。
- 律：纯函数；结果上限 64 条 + truncated 标志，超出即停。

**③ file.search_replace（效应：写）**

```text
file.search_replace : File → Old → New → Anchor? → Result Replace
Replace = { ok, diff: {minus: Int, plus: Int}, new_anchor: Anchor }
```

- GetPut 律：未给 anchor → 取依赖图中最近 read 的锚点；给了 anchor 且 hash ≠
  当前文件 hash → Fail(code=anchor_mismatch)。
- old 匹配数 n：n=0 → Fail(no_match)；n=1 → 替换；n>1 → Fail(ambiguous)——
  歧义必须 fail-closed，禁止静默替换首处。
- 返回 new_anchor，使下一轮 read 校验为 O(1) 哈希比较。

**④ terminal.run（效应：执行）**

```text
terminal.run : Process? → Cmd → Timeout? → Cwd? → Result Cmd
Cmd = { exit_code: Int?, wall_ms: Int, timed_out: Bool,
        stdout_tail: Text ≤ 8 KiB, stderr_tail: Text ≤ 8 KiB }
```

- process 缺省 = 新进程；host 拥有 cwd/env/超时/资源（模型不可指定 env、不可
  绕权限）；分层超时（tool 级 timeout、run 级 wallclock）。
- wall_ms 是一等字段（时间特征层依赖它，§8.7 发现 1）；中间回报（S5-2）为同
  签名可选增量。
- 失败语义分层：exit_code ≠ 0 是值（结构化 Cmd），不是 Fail；Fail 只覆盖
  未启动、超时杀、门拒绝（ACAF/权限）。

**⑤ retrieval.web_search（效应：检索）**

```text
retrieval.web_search : Query → MaxResults? → Result Search
Search = { query, sources: [Source ≤ 10], count, pointer: EvidencePtr }
Source = { title, url, snippet: Text ≤ 200 B, weight, tier }
```

- 效应链：子代理 relay → 120s 超时 → 候选计数 → MMR 子模选择（相关性 −
  λ×多样性）→ 机械来源梯队加权。
- local_browser 模式隐藏本工具与 web_fetch，显示 browser_read（7 工具面）。

**⑥ retrieval.web_fetch（效应：检索）**

```text
retrieval.web_fetch : URL → Result Fetch
Fetch = { url: CanonicalURL, status, size: Int, pointer: EvidencePtr }
```

- URL 规范化（http/https、userinfo 拒绝，ACAF Slice2B 目标解析）；大小上限 +
  全文留盘，模型只收 EvidencePtr。
- browser_read = 同族 evidence 变体（签名同构，返回 EvidencePtr）。

**⑦ blackboard.read（纯查询）**

```text
blackboard.read : Partition → Selector? → Result Board
Board = { partition, entries: [Entry ≤ 8 KiB], total_cap: Int }
```

- selector 类型化（key 路径/前缀），不引入自由查询语言。
- temporal 分区 = 时间特征查询口（§3）：potentials + fires≤t 约束（fires 不渲染，
  见 §3.4）。

**⑧ delivery.submit（效应：交付）**

```text
delivery.submit : Draft? → Result Submit
Submit = { stage: 1 | 2, checklist: [Check], state: State }
```

- 两阶段：stage 1 = 信息展示（非硬门）→ stage 2 = 确认置 done；receipt 绑定
  终答（AGENT-DELIVERY-FLOW 形态不变）。
- 时序不变量（LTL 例）：submit(2) 后无工具事件；终答前机械审计同轮注入。

### 2.3 组合与律

- 类型化管线：read → (anchor) → search_replace 是透镜管线；grep → read 是
  过滤→取窗管线。
- 折中档（D1）允许一层组合（如 `pipe(file.read, file.grep)`）在同一往返归约，
  返回组合值或指针；顶层保留 tool_calls 形态，工具契约函数化。
- 依赖图边随归约维护：read→write 锚点边、工具→实体变更边。
- 确定性：纯服务（file.read/grep/blackboard.read）同状态下结果确定；效应服务
  只返回结构化信封，不返回自由文本。

### 2.4 归约边界（D1）

- 单轮归约 ≤4 步（最小项集 = 应用 + 单层 pipe；无 let/条件/嵌套）。
- 效应数复用候选计数/预算硬门，不引入部分结果注入。
- 激进档（表达式求值）维持已登记未来路径，不实施。

## 3. F2 temporal 分区渲染规格

定位：**观测记录面**——时间轴上的特征域事实序列。域 = 模型进入不同思维/行为
动力学阶段的观测标记；明确为纯观测性结果、仅记录、不承诺用途。PULL 查询、
零常驻 token、无注入、不给建议。

### 3.1 数据模型（运行时查询行 + 存档 spike 点位）

```text
type TemporalRecord = {
  t: Time,                      -- 决策轮时刻（model_output 且 tool_calls 非空）
  domain: Start | Normal | Pressure | LowProgress | Stuck,
  entry_round: Int,             -- 进入当前域的轮次（相对 run 起始）
  dwell_rounds: Int,            -- 当前域驻留轮数
  u_prog, u_err, u_stuck: Float, T̂: Float,
  err10, succ10: Float          -- 最近 10 个工具事件中的失败/成功占比
}
```

记录不同域仅需 spike：域内状态行是**运行时**查询面（Now/Recent/Feature 取最近
k ≤ 20），不落盘；持久化只写域切换点位：

```text
type DomainSpike = { t: Time, domain: Domain }   -- 域切换事件；驻留轮数由相邻
                                                 -- spike 推导，不单独存储
```

### 3.2 域定义（语义谓词，非学习）

```text
low_progress = u_prog < 0.5                     -- 最近成功 > ~5.5 轮前（τ_prog=8·T̂）
pressure     = (u_err ≥ 2) ∨ (u_stuck ≥ 1.5·T̂) -- 近 3 分钟窗 ≥2 错误，或卡住证据满 1.5 轮

Start         has_success = false               -- 首次成功前，不并入任何域
Normal         ¬low_progress ∧ ¬pressure
Pressure       ¬low_progress ∧ pressure         -- 有近期成功但仍高压（错误爆发未停）
LowProgress    low_progress ∧ ¬pressure         -- 长时间无成功、无错误压力（静默/方向错误）
Stuck          low_progress ∧ pressure          -- 长时间无成功 + 高压（真卡住）
Recovery       （轨迹属性，非静态格）卡住/低进度 → 正常 的迁移，记入迁移日志
```

启动态（首次成功前）单独标记，不并入任何域；run 首轮不产生伪域。

### 3.3 查询面（经 §2.1 信封）

```text
blackboard.read : Partition="temporal" → Selector → Result Board
Selector = Now | Recent(k ≤ 20) | History | Feature(name, k ≤ 20)
Board = { partition, entries ≤ 1 KiB, total_cap }
```

- Now：当前域状态行（域、入域轮、驻留轮 + u_prog/u_err/u_stuck/T̂/err10/succ10）；
- Recent(k)：最近 k 轮状态行序列（时间倒序）；
- History：域迁移日志（domain, 入域轮, 驻留轮），有界（最近 ≤ 20 条迁移）；
- Feature(name, k)：单特征紧凑序列（如 `0.3,0.5,1.2`）。

渲染示例（caffe FMaD2Ce @2658s，示意）：

```text
temporal.now → [2658s | Stuck | 入域 2 轮前 | 驻留 2 轮]
  u_prog=0.27 u_err=1.2 u_stuck=15.0 T̂=8.7s err10=0.3 succ10=0.1
```

### 3.4 渲染约束

- 只渲染 ≤ 查询时刻（无未来）；fires 不渲染（内部留痕，§4.6）；
- 启动态单独标记，不并入域统计；run 首轮不产生伪域；
- 有界内存：运行时仅保留查询窗（状态行 k ≤ 20）+ 当前域状态，O(1)/决策；
  迁移历史由 spike 存档推导，不单独缓存；
- 更新时机：每决策轮重算一次，确定性、零标签；
- 无注入、无建议：temporal 是 PULL 面，不进 prompt（除非模型/用户显式查询）。

### 3.5 存档载体（D7）

域切换 spike 随会话侧车一同存档：StoredConversation envelope 追加可选字段
`temporal_spikes?: Vec<DomainSpike>`（`serde(default)`、旧侧车零迁移、随 7 天
retention 一并清理）；跨 prompt 恢复时随侧车载入，域状态可重建；子代理不同构，
不持久化。

### 3.6 记录价值与阈值初值

- 已确认价值：域序列 = "模型进入不同思维/行为动力学阶段"的跨轮可查观测记录；
- 阈值初值（2026-08-30 用户确认，语义推导、非拟合，仅作记录内容）：
  `u_err ≥ 2`、`u_prog < 0.5`、`θ_stuck = 1.5·T̂`、`err10/succ10` 窗口 10；
- 候选使用方向（未定，仅登记）：错误预判断与重试（低进度/卡住域驻留时长 ×
  后续域类型）为优先候选，机制未定（§9.10），依赖 F4 目标身份与域标签稳定。

## 4. F3 LIF 时间外挂计算规格

### 4.1 特征集（决策点反推，§8.2/§8.4）

特征向量（逐决策轮）：`(u_prog, u_err, u_stuck, T̂, err10, succ10)`；可选长窗
基线/短窗偏离/比值（§6.2 双尺度）。

特征 ↔ 模型决策点映射（每个特征对应一个真实决策，不做穷举）：

| 特征 | 服务的决策 |
|---|---|
| u_err（错误压力） | 继续做当前方向，还是先换方法/查根因？ |
| u_prog（进度新鲜度） | 我最近是否在推进？要不要收尾或转向？ |
| u_stuck（二阶联合证据） | 当前是否处于"卡住"动力学？ |
| T̂（节奏估计） | 当前一轮约多久？剩余预算换算成几轮？ |
| err10/succ10（近 10 事件成败比） | 短窗内工具健康度；是否重复同一失败目标？ |

### 4.2 一阶 LIF 更新律（事件驱动、O(1)/事件）

```text
u(t) = u(t₀)·exp(−(t−t₀)/τ) + w        -- 事件到达时
```

- 无事件时电位自然衰减 = "新鲜度/压力"连续特征；时间性由 τ 显式承载，无额外
  时间窗口机制；
- leak=0 极限 = 现有复读计数（纯积分器）；stall 检测 = 距最近事件时间；
- 每通道一个一阶漏积分器 + 阈值 + 复位 + 不应期；u ≥ θ → fire（结构化内部
  事件）+ 复位 + 不应期内不再 fire；
- 复位语义是通道契约的一部分（外部无复位积分器错误模式的教训，§9.4.1），
  逐通道见 §4.3，并纳入 LTL 验证。

### 4.3 通道登记（(w, τ, θ, refractory) 语义推导；§8.7/§9.6.2 定稿）

| 通道 | 层/语义 | 输入 | w | τ（语义→物理） | θ | refractory | 复位 |
|---|---|---|---|---|---|---|---|
| err | 一阶/时间语义 | 工具错误事件 | 1/事件 | τ_err = **180s 固定** | θ_err = 4 事件 | 60s | full-reset |
| stall | 一阶/时间语义 | 事件间隔 | 1/事件 | 阈值 90s 固定墙钟间隔探测（LIF 实例 τ=600s） | θ=2 | 120s | full-reset |
| slow | 一阶/时间语义 | 长工具调用（wall_ms>60s） | min(5, wall_ms/60s) | τ=600s | θ=3 | 60s | full-reset |
| deny | 一阶/时间语义 | 拒绝事件 | 1/事件 | τ=120s | θ=4 | 60s | full-reset |
| prog | 一阶/轮语义 | 成功动作 | 成功置 1 | τ_prog = **8·T̂** | —（∈[0,1]） | — | 成功置 1 |
| stuck | 二阶/轮语义 | I(t)=u_err/θ_err·(1−u_prog) | 1 | τ_stuck = **3·T̂** | θ_stuck = **1.5·T̂** | 8·T̂ | full-reset（fire 后清零） |

**err 输入定义（2026-08-31 审查处理 R1 裁定，生产口径）**：「工具错误事件」
= 宿主级错误（工具未启动/宿主异常；事件面 status=error 且无 exit_code 值）
∪ 超时（timed_out=true）；terminal `exit_code ≠ 0` 是 D2 结构化**值**，归类
Other（中性），不喂 err；锚点拒单、候选门拒绝等 status=error 拒绝事件走
deny 通道（R2 接线后）或接线前暂不喂入。离线复验 lif_replay 与生产
host_exec 喂入同一谓词（102-run 重跑：err 1 run/1 fire）。

语义分层规则（§9.6.1/§9.6.5 混合确认）：

- **速率/间隔探测器（err、stall）用固定时间语义**：精确的墙钟速率/间隔检测，
  不随节奏伸缩（err 时间语义严格不劣于轮语义——初稿首轮口径实证 22 run/24
  fires vs 11 run/8 fires；R1 生产口径重跑后为 1 run/1 fire，且无"轮触发而
  时间不触发"的 run）；
- **持续/新鲜度通道（stuck、prog）用轮语义**：τ_sec = k_τ·T̂；θ_sec、
  refractory_sec 由各自语义常数 × T̂ 归一（非学习，仅单位换算）；w 固定；
- 不变量：θ/τ = 0.5 与"示例到达时间 = 4.05 轮（I=0.675）"在任意节奏下成立。

### 4.4 二阶 stuck 通道（闭式解，零拟合）

输入电流（归一化乘积，无新超参）：

```text
I(t) = w·(u_err/θ_err)·(1 − u_prog),  w = 1
```

语义：错误压力高 ∧ 近期无成功 → I 大；正常调试（有成功穿插）→ (1−u_prog)≈0
→ I≈0；单次错误尖峰不持续 → 不触发。

精确事件驱动更新（带泄漏核，两阶指数乘积的积分闭式解；右端点离散化会产生
7–21× 伪迹，禁止使用）：

```text
α = 1/τ_err − 1/τ_stuck
β = 1/τ_err + 1/τ_prog − 1/τ_stuck

u_stuck(t) = u_stuck(t₀)·exp(−Δt/τ_stuck)
           + (u_err(t₀)/θ_err)·exp(−Δt/τ_stuck)
             ·[ (1 − exp(−Δt·α))/α − u_prog(t₀)·(1 − exp(−Δt·β))/β ]

随后：u_err(t₀) → u_err(t₀)·exp(−Δt/τ_err)；u_prog 同理；再叠加事件 spike。
实现用 expm1 保数值稳定（(1−exp(−x))/x = −expm1(−x)/x）。
```

- 无需 min(1,·) 截断：err 通道 full-reset 下 u_err ≤ θ_err 恒成立，u_err/θ_err
  ∈ [0,1] 是结构性事实；
- 与 0.05s 步长 ODE 数值参考对 102 runs 全量对照最大偏差 0.006；
  O(1)/事件、零新参数、确定性；
- 触发条件解读（I > 0.5 ⇔ u_err·(1−u_prog) > 2）：u_prog < 0.5 且 u_err
  累积 ≈2–4 次于 3 分钟窗——"错误压力高 ∧ 近期无成功"的持续版本；
- θ_stuck=1.5·T̂ 下 stuck 是近不触发的精度通道（实证 0/102 触发、峰值 θ 比
  0.65）：零噪音、零价值演示；早期干预主张由 err 一阶通道承担，round50 兜底
  长任务周期性。注记（2026-08-30 用户裁决）：0.65 为估计器定稿前 per-run
  近似口径的初步读数；定稿规格（§4.5 在线钳制估计）下 102 runs 读数为
  ~0.99（仍 <1，0/102 触发不变）。该峰值比为诊断读数非标定值，按初步
  验证值处理，不专门复现（V2 不再复核）。

### 4.5 T̂ 估计器规格（确定性、在线、零标签）

- **决策轮定义**：相邻"决策输出"（model_output 且 tool_calls 非空）的间隔；
  一轮 = 一次模型决策到下一次（含思考与工具执行）；text 输出与 tool_calls 输出
  同属一轮，不算间隔；
- **算法**：有界内存储层（最近 8–32 个决策间隔——采样 ≥8 启用、缓冲 ≤32，
  2026-08-31 审查处理 F14 措辞统一）+ 稳健中心估计（中位数），
  每事件 O(1)、确定性；长间隔截尾（>300s 计 300s）防单次长工具拉偏；
- **初始化与收敛**：T̂₀ = 8s（典型轮实测上界）；采样 ≥ 8 个间隔后启用；
  钳制 T̂ ∈ [3s, 600s]；
- **事实依据**（102 runs）：决策间隔重尾双峰——典型轮 3–8s（caffe FMaD2Ce
  median 5.4s）、长工具轮 100–1000s（mean 39.7s）；单标量不稳定
  （median 5.4 vs mean 39.7），必须用在线局部估计，不用 per-run 标量。

### 4.6 fires 出口（§9.7 用户裁决）

- fires 仅作内部事件留痕（日志/审计/验证），不入 schema/verifier、不进模型
  表面、不渲染、不注入；信息面全部落在连续特征域（temporal 分区）；
- round50 维持现有软门提醒形态，不扩展为强制模板轮；
- 准确性要求（§9.4.3）以"汇报噪声预算"形态保留：更新必须用 §4.4 闭式解；
  复位语义写入契约并纳入 LTL 验证；refractory 防风暴。

### 4.7 四对照门（V2 离线复验验收）

| 对照 | 操作 | 通过标准 |
|---|---|---|
| C1 时间打乱 | 保留事件类型/载荷序列，随机重排事件间隔（10 次置换） | fires 数量/时机显著变化；若打乱下不变 → 只是计数 |
| C2 spike 移除 | 用膜电位跨阈（u ≥ θ，无复位/无不应期）替代 fires 判定 | fires 时机与 membrane 跨阈不同（阈值事件结构在起作用） |
| C3 leak=0 计数 | τ→∞（纯计数）同 θ | fires 与计数窗口触发不同（泄漏/时间窗在起作用） |
| C4 EMA 对照 | 同 τ 的一阶 EMA 对同一信号做同 θ 判定 | fires 不劣于 EMA；非线性增量由 C2 单独衡量 |

- 注意：C1 置换必须在"同事件数、同事件类型序列"下进行；C2 须在同一查询时刻点
  比较；二阶通道上 C2 与 C4 重合（EMA 乘积 ≡ 膜电位，单极点同构）——独立对照
  实际为 C1 / C2≡C4 / C3 三个；
- 通过 = "积分 + spike 捕捉时间特征"成立；任一不通过 → 该通道降级为普通统计
  特征，不再保留 fires 判定（§9.7 后 fires 无注入面，降级 = 只保留连续电位）；
- 首轮运行证据（2026-08-30，102 runs）：C1 部分通过（22 触发 run 中 14 个 fires
  数量改变、8 个不变——时间结构起作用但存在计数成分）；C2≡C4 对二阶重合且本批
  stuck 0 fires 0 膜跨阈——非线性面未做功（未决）；正式判定保留至 V2。

### 4.8 参数纪律（§8.6）

- τ/θ/w 是带语义的设计常量（τ = 决策相关记忆窗口，θ = 构成告警的命中数），
  不是数据标定值；θ 上界由架构边界给定（必须在墙钟耗尽/预算爆/输出预算烧穿
  等灾难条件前触发）；
- 计算正确性与特征值零模型依赖；通道集与阈值仅"模型类"级依赖（类内跨版本
  稳定），做成小型显式 profile（模型类 → 通道集/θ 偏好），不混入计算层；
- 版本级拟合（按具体版本行为拟合的常量）禁止——orientation 阈值 50 的反例
  模式不得复制；
- 可学习 = 在线自调：仅 τ 的秒值经 T̂ 换算（无标签、无梯度、无跨 run 拟合），
  反向传播式学习不引入；通道集不因可学习自动扩张。

## 5. F4 失败目标身份入事件面

### 5.1 动机与现状缺口

工具错误的标量长线聚合（短窗/长积分/结构重复）不是"真正需要处理的错误"的可靠
代理（§9.6.5.7：102 runs 长积分与 reward 的 Spearman |ρ| ≤ 0.15）；缺失的观测
面 = **失败目标的身份**——同一命令/锚点反复失败 vs 错误但推进，标量无法区分。
现状 `tool_completed` 仅 tool/call_id/exit_code，无 cmd 文本、无目标身份。

### 5.2 目标身份分类（确定性 digest，不落自由文本）

| kind | 载体 | 身份 |
|---|---|---|
| `cmd_target` | terminal.run 失败 | `cmd_digest = sha256(canonical(cmd))` + `cmd_preview: Text ≤ 80 B` |
| `anchor_target` | file.search_replace 失败 | `{path, anchor_hash, size}`（GetPut 锚点） |
| `file_target` | file.read/file.grep 失败 | `{path}` |
| `url_target` | retrieval.web_fetch/browser_read 失败 | canonical_url（ACAF 网络目标规范化） |

### 5.3 事件面字段设计

- 失败事件（tool_completed status=error 或同语义失败路径）增可选
  `failure_target?: { kind, id }`；id 为 §5.2 确定性 digest；
- 同一目标身份重复失败可在时间特征层按 target 维度聚合（I2/I3 机械实现范围，
  本阶段只定事件面字段）；
- 边界：journal 不落完整命令原文；只写 digest + 有界预览；verifier 校验 kind
  合法、id 格式与事件内可复核载荷一致（digest 由执行侧计算后写入，verifier
  不做原文重算）。

### 5.4 trace→事件链同构核对

- 归约留痕 receipt 每段（arg_validation/gate/execution/delivery）须对应事件链
  中的事件（§3/§5 同构）；
- 失败目标身份字段在 receipt 与事件链中一致；verifier 交叉核对 receipt 段与
  事件链事件一一映射（现有事件链校验扩展）；
- 该同构核对为 §9.10 错误预判/重试方向（观测面）的转正式设计前置。
- **状态（2026-08-31 审查处理 F11 挂账）**：该同构核对当前**未实现、未排期**；
  正式登记为未闭合项（TODO P2-10 阶段 2 全面审查处理），随 V1 FakeProvider
  验证面一并排期实施，未闭合计数不变。

### 5.5 依赖与边界

- 依赖：F4 → I2（Schema + verifier + fixtures 先行）；I2 是阶段 2 实施切片；
- 与 D3 一致：命令/检索副作用**不建图**；失败目标身份仅以事件面字段记录；
- 文件锚点链（read→write 锚点边 + 文件实体变更边）是依赖图先行范围，本阶段
  只登记身份语义，图实现随 I 切片。

## 6. F6 参考系重定位与观测面扩充

### 6.1 定位与模型无感边界

LIF 从"探测器"重定位为**时间性参考系**：低精度容忍（对个人普通用户/API 用户，
模型内部不可访问；读数成本与注意力预算有界 → 优化目标是"对决策的互信息"而非
估计精度）；已确认的窗口/阈值均为语义推导初值、非精确性窗口，允许相当误差余量，
目标只是帮助模型粗决策（继续/转向/收尾）。

模型无感硬边界：全部计算落机械层；模型只经 `blackboard.read partition="temporal"`
拿到渲染结果并使用（PULL、零常驻 token、无注入、无模板轮、无自动动作）；模型面
不新增任何机制。

### 6.2 双尺度基线结构（吞环境波动、不吞退化）

- **长窗（整 run 在线估计，10–30 分钟级）= 环境基线**：本 run 自己的背景错误率，
  归一化网络抖动/超时/DNS 等干扰；
- **短窗（≈3 分钟）= 当前偏离**：最近发生了什么；
- **渲染 = 基线 + 当前 + 比值**（如"当前错误率 = 基线 ×2.1"）；
- 结构定位：CUSUM 的 baseline+drift 原语；旧证据不污染当前读数；模型自身退化
  以相对偏离显现，不被基线平均掉。

### 6.3 每轮校准分层（每任务轮重算，不固定全局值）

- 动因：家庭网络/任务随时间漂移，全局常量会过期；每轮重算是概念漂移应对；
- 语义固定、秒值自适应：θ 轮数语义（θ_err=4 事件、θ_stuck=1.5 轮、refractory
  等）不随 run 变，秒值经本 run T̂ 换算；"重算"是单位换算，不是每轮拟合阈值；
- 自指反馈环防制：行为变慢 → T̂ 变大 → 窗口变宽 → "慢"被归一化掩盖——err 保留
  墙钟 180s 绝对锚，T̂ 钳制 [3,600]s，同时记录原始秒值与归一化轮值，两者都可查；
- 早期小样本噪声：run 开头（T̂ 未收敛，≥8 间隔启用前）域标签/读数标注可信度与
  样本量，模型不过度信任前几轮。

### 6.4 观测面扩充优先级（逐项过决策点反推）

1. **失败目标身份入事件面（F4）**——唯一区分"反复失败"与"错误但推进"的观测，
   信息增益最大且便宜；
2. **stated-vs-done 探针一致性**——探针自述下一步 vs 实际动作，合规内部探针，
   直接服务耦合试验；
3. **环境进展接地（测试通过/文件 diff）**——解耦"模型在忙"与"任务在推进"
   （u_prog 最大盲区）；
4. **序结构（熵率/动作转移/自相关）**——现有事件流上零成本计算；
5. **混杂控制**——验证纪律：同难度、同模型类下再谈判别（轨迹长度-失败关联是
   难度混杂）。

### 6.5 动作层耦合试验 P1–P3（可证伪、102 runs 离线可跑）

TRACED 几何搬到动作层——用命令/编辑目标语义相似度定义动作位移（progress）与
方向变化率（curvature），检验三个命题：

- **P1**：成功 run 动作级 progress 显著更高、curvature 显著更低（须控难度）；
- **P2**：域序列提升对"下一步动作类型"的预测（转移似然）——域携带行为信息的
  直接检验；
- **P3**：stated-vs-done 一致性高的 run，后续动作更可预测。

判定：P1–P3 成立 → "时间性 + 全局视野外挂"值得做；不成立 → 参考工具保留
（自我定速），但不得宣称携带超出事件流的内部信息。

四支柱（落点）：低精度参考（成本低）+ 双尺度基线（吞环境波动不吞退化）+ 每轮
自适应（概念漂移免疫）+ 可证伪耦合试验（决定投入边界）。

## 7. 阶段 2/3 排期与依赖

实施切片（依赖前序；放行时按 BACKLOG 纪律升 P0 排期并入账）：

| 项 | 内容 | 依赖 |
|---|---|---|
| I1 | T̂ 估计器 + LIF 时间特征计算器（纯计算；`D:\tb-eval\jobs-official` 102 runs 离线复验） | F3 |
| I2 | 失败目标身份入事件面（Schema + verifier + fixtures 先行） | F4 |
| I3 | temporal 分区运行时（每决策轮域标签 + 查询面接线 `blackboard.read partition="temporal"`） | F2/I1 |
| I4 | 域 spike 存档随会话侧车（StoredConversation envelope 可选字段 + 7 天 retention + 跨 prompt 恢复重建） | F2/I3/D7 |
| I5 | 工具类型化信封（8 工具落地：file.read View / search_replace GetPut 律 / terminal.run 值语义 / blackboard temporal 截断） | F1/D1 |
| I6 | 一层组合/pipe（折中档，最小项集 + 归约边界 §2.4） | I5/D1 |

验证闭环：

| 项 | 内容 | 依赖 |
|---|---|---|
| V1 | FakeProvider 测试面验证（信封/组合语义 + §6.6 模型熟悉度近零提示验证，无需实机） | I5/I6 |
| V2 | 离线 102 runs 复验（T̂/域标签/spike 序列 vs §9.8 聚类对照、四对照门、零误干预） | I1/I3 |
| V3 | S4 冒烟复验 + BACKLOG/TODO/索引状态同步（pending → implemented 视切片范围） | — |

## 8. 已知边界与未决项

- **域是事实描述，不是结局信号**（§9.8 判别力：末域/域迁移数与 reward 无判别；
  §9.6.5.9 分叉特征不携带结局方向）；temporal 分区不承诺任何自动化用途；
- **fires 取消注入**：spike 仅内部留痕；连续特征域是唯一信息面；round50 软门
  形态不变；
- **外部 LIF 项目**：具体公式不外落、不搬运；本设计独立（外部可用性结论仅作
  方向线索：LIF 非普遍更优、spike 非越多越好）；
- **可学习边界**：仅生理参数秒值经 T̂ 在线自调；无反向传播/无监督标定；通道集
  仍按决策点反推；
- **残余模型依赖**：计算正确性与特征值零依赖；通道集与阈值属类级依赖（显式
  profile）；版本级拟合禁止；
- **数据不可靠性**：设计不依赖模型版本标定；特征由 run 自身事件确定性计算；
- **存档**：temporal_spikes 随会话侧车、7 天 retention、子代理不同构不持久化；
- **未决（登记，不阻塞阶段 2）**：错误预判断/重试方向（观测面 vs 机制面）待
  §9.10 探针；stated-vs-done 探针形态；环境进展接地（测试通过/文件 diff）特征
  定义——均以 §6.4 优先级在后续阶段另裁。
