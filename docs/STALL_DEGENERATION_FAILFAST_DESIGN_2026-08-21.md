# 哨兵退化 fail-fast 化设计（2026-08-21 设计定稿；纯文档登记、未实施；2026-08-28 修订：触发源=复读哨兵，stall 已物理删除）

> 状态：`implemented`（S0 证据门通过 + S1 代码 + S2 测试已闭合 2026-08-21；
> S3 重建 → S4 复验待续。2026-08-21 用户裁决：harness fail-fast 方向有道理
> ——失败本身意味着当前模式不适合当前任务；前置证据门=确认空转/重复与架构
> 本身无关；确认后实施，显式标明终止原因。**全面审查处理（2026-08-21）**：
> ①per-run 隔离正式路径实现——长驻进程（ACP server）跨 run 共享 transport，
> 原「仅 run 边界重置=新 transport」假设仅在一键 CLI 成立；现 `ModelGateway::
> for_new_run()` 每 run 换新实例（controller `run_turn_inner` 开头调用，
> 主 agent 与检索子代理共享同一 run 实例），计数/档位零跨 run 泄漏；
> ②S4 验收口径修正——单 run 哨兵预算上限为「≤3 次触发 × 单次预算」
> （设计 §2.2 效果段的「首档 64K 一次」为 schemelike 类场景期望，
> 非硬上限）。）
> **2026-08-28 修订（THIN-HARNESS-REDESIGN 用户裁决）**：fail-fast 机制本身
> 保留（会话档位持久化、单调计数、disabled 档即终止），但 reasoning-stall
> 触发源已物理删除（官方 max 只等待不杀，复读判定已足够，无保留价值），
> 降级梯由 reasoning/content 复读哨兵触发。详见
> `THIN_HARNESS_REDESIGN_DESIGN_2026-08-27.md` §4.6。
> 性质：P0-0d 后续（输出健康哨兵跨请求语义修订）+ 0e 观察延伸（sweep
> r1-g1 12 次哨兵归因）。关联：
> [DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md](DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)
> （哨兵/阶梯现状）、[ADR-0010 §14.35/§14.36](../adr/ADR-0010-vol-14-addenda-index.md)、
> deepseek-harness 源码对照（tmp_dsh_review：agent-loop step 边界重试、
> llm-retry 默认 2 次、无降级/无生成期哨兵）。
> 实施路由：S0 证据门 → S1 代码 → S2 测试 → S3 重建 → S4 复验。
> **S0（2026-08-21）：通过（放行 S1）**——见 §2.1 中间判定。**S1/S2
> 已闭合**（orz 子模块 a96faab 之上未提交批次；orz-loop 544 通过 /
> 0 失败 / 3 ignored、Python conformance 230 通过）。设计轮不动计数（27）。

## 1. 背景与证据

### 1.1 sweep r1-g1 哨兵数据（2026-08-21，冻结版 cf0be20）

| 题目 | Reward | 哨兵明细 | 归因 |
|---|---|---|---|
| schemelike-metacircular-eval | 1 | 4× reasoning_stall | 每次降级恢复后继续、最终通过 |
| build-pov-ray | 1 | 0 | — |
| dna-assembly | 0 | 2× stall + 4× rep | 哨兵链→Disabled 档解码错误非零退出 |
| llm-inference-batching-scheduler | 0 | 2× stall | 随后网络零 chunk 重试耗尽非零退出 |
| feal-linear-cryptanalysis | 0 | 0 | 1800s 超时 |

合计 12 次哨兵（8× stall + 4× rep），按 ¥4.592/M output 折算约 **¥2.5**。
stall 占大头（8×64K）；rep 均为「连续 5 个相同 delta」秒级触发、成本可忽略。
零 HTTP 400、零 run_invalidated、零折叠。

> 注（2026-08-21 S0 采集期核对）："零 run_invalidated" 在事件级不准确——g1
> schemelike 与 g2 qemu-startup/vulnerable-secret 的 journal 末尾均存在
> `runtime_stagnation_guard{restart_requested, STAGNATION-NGRAM-REPEAT}` →
> `run_invalidated{status:restart_requested}`（终答停滞守卫：counterexample_gate
> 触发重复终答后重启，verifier 仍通过，属良性重启、非终止失败）。该守卫与
> 传输层输出健康哨兵（reasoning_stall/repetition）不同机制，不在本设计范围。

### 1.1a sweep r1-g2 哨兵数据（S0 证据门采集批次，2026-08-21，冻结版 cf0be20）

| 题目 | Reward | 哨兵（触发对） | 触发上下文 | 归因 |
|---|---|---|---|---|
| vulnerable-secret | 1 | 0 | 29 轮，无哨兵 | 干净通过 |
| qemu-startup | 1 | 0 | 46 轮，1 次折叠（seq345，40 轮后），无哨兵 | 干净通过（终答良性重启） |
| sqlite-db-truncate | 1 | 0 | 19 轮，无哨兵 | 干净通过 |
| feal-differential-cryptanalysis | 0 | 1× reasoning_stall | **首请求即触发**（374s/64K，consec=1→low）；低档重试约 18 分钟后 `error decoding response body` 非零退出 | 失败链=stall→解码错误（同 g1 dna-assembly） |
| polyglot-c-py | 1* | 0 | 首请求 900s 未产出任何轮次、无哨兵（流在产 token 但低于阈值），900s 超时 | 模型侧慢生成；*verifier 通过系容器内交付物既成，非代理成果 |

合计 1 次 stall 触发（1×64K）。零 HTTP 400。失败 run（feal-differential）
无 journal，触发上下文仅 orz.txt + 容器实时日志（启动 21:53:47 → stall
22:00:12，间隔 374s，无任何工具事件，确认首请求）。

### 1.1b S0 采集口径与方法（原 `sweep-s0/collect_s0.ps1`，产物原 `sweep-s0/`；2026-09-13 一次性产物归档至 [`存档/root-artifacts-2026-09-13/sweep-s0/`](../存档/root-artifacts-2026-09-13/sweep-s0/)）

- 每对 WARN（stream interrupted + degrading to）计 1 次哨兵触发；记录 kind、
  consecutive、降档目标、持续秒数、估算 token。
- 有 journal 时按时间戳对齐到工具轮：触发轮次 = 该请求完成对应的 model_output
  轮；前 3 事件 = 该轮 model_output 之前的 model_output/tool_started/
  tool_completed 序列；旗标 = 紧邻 blackboard_read(session/actions)、折叠/
  压缩后首请求、截断指针后（后者在持久化产物中不可观测，以折叠事件为代理）。
- 基率对照：每个 run 全部轮次中「紧邻 bb(session/actions)」占比，用于判断
  触发点分布是否显著高于常态（g1 schemelike 基率 36% 紧邻 / 52% 前3）。

### 1.2 跨请求烧 stall 的机制根因（实现核对）

现状机制（transport.rs 实现核对）：

- 降级梯（high → low → disabled → 失败）是 **`generate_stream` 调用内
  局部变量**——每次模型请求（每个工具轮）从 config 默认档（high）重新
  开始，无跨请求档位状态。
- 会话级防循环只有 `degeneration_consecutive` 计数器（三族共享、
  `DEGENERATION_LIMIT=3`），且**成功请求即清零**。
- 因此病态序列可以反复发生：请求 N 烧 64K stall → 降 low 恢复成功 →
  计数器清零 → 请求 N+1 回到 high 再烧 64K……（schemelike 4 次、
  dna 2+4、llm-batching 2）。
- 计数器只在「连续失败无成功插入」时递增；stall→成功→stall 模式永远
  到不了 3，run 可以无限期烧预算直到 wallclock/工具轮上限。

### 1.3 deepseek-harness 对照（tmp_dsh_review 源码核对）

- 每 step = 一次模型请求；空响应/错误 → `agent/request-error` →
  llm-retry 在 **step 边界**重试（durable retryId、从 session log 重建
  同一请求、退避 500ms→10s+10% jitter、**normal 默认 maxRetries=2**——
  README 注释"five retries"与实际代码不符）；仍失败 → **step 抛错、
  run 失败**。
- **无降级梯、无生成期哨兵、无跨请求 thinking 状态**。stall 要么在预算
  内出活，要么 step 失败收场——不存在"恢复续跑→下轮回 high 再烧"模式。
- 用户裁决方向（2026-08-21）：fail-fast 语义有道理——失败本身意味着
  当前模式不适合当前任务；但需先确认空转/重复非架构诱因。

## 2. 决策

### 2.1 前置证据门（S0，实施前必须通过）

判定标准：**空转/重复与架构机械结构块无稳定相关**。

采集（下一批扫描 r1-g2 起，每次哨兵触发时记录）：

1. 触发轮次（tool round 序号）与触发前最近 3 个事件（工具序列）；
2. 触发点上下文组成：是否紧邻 blackboard_read（session/actions）大分区
   重读、是否折叠/压缩后首请求、是否工具输出超长截断指针后；
3. 跨任务分布（与任务难度/题型的相关性）。

判定：

- 通过（放行 S1）：哨兵触发点与机械结构块无稳定相关（不集中在
  blackboard 重读、折叠/压缩后首请求等），且跨任务分布不均（集中在
  难题）；结合既有证据——P0-0e 退役机械块后 S4（make-doom）零哨兵、
  本轮哨兵均发生在任务中段（首 stall 在 20–33 次模型输出之后）、
  2026-08-20 API 探针已钉死触发=256K×high×**输出/上下文重复循环**
  （8K max_tokens 同上下文 3.7s 自然收尾）——判定为模型/任务侧原因。
- 不通过（暂停）：出现 ≥2 例哨兵与机械块强相关（如折叠后首请求重复
  触发、blackboard 大分区重读后连续 stall），先修架构侧诱因再回来。

**S0 中间判定（r1-g1 + r1-g2 两批、10 题、7 次触发对=5 stall+2 rep）：**

- 机械结构相关性：**0 例强相关**。g2 feal-differential 的 stall 为**首请求**
  （启动后 374s，无任何工具事件前置），直接构成机械块无关的反例；g1
  schemelike 两次 stall（round 34/47）虽紧邻 blackboard_read(actions)，
  但该 run 紧邻基率 36% / 前3基率 52%，与随机分布一致，非稳定相关；两批
  均为 0 例「折叠/压缩后首请求触发」。
- 跨任务分布：触发集中于复杂实现类任务（schemelike 求值器、dna 组装、
  llm batching、feal 差分密码分析），简单/中等问题（build-pov-ray、
  vulnerable-secret、qemu-startup、sqlite-db-truncate）零触发，分布不均。
- 口径修正：g1 文档「首 stall 在 20–33 次模型输出之后」被 g2 首请求 stall
  证伪——stall 可出现在任意轮（含第 1 轮），进一步支持「非机械门控」。
- **结论：通过（放行 S1）**。触发点与机械结构块无稳定相关、跨任务分布不
  均，且与既有探针结论（256K×high×输出/上下文重复循环、8K max_tokens 同
  上下文自然收尾）一致，判定为模型/任务侧原因。待用户确认后实施 S1。

### 2.2 主案：跨请求 fail-fast 化（哨兵预算有界 + 显式终止）

> **实施注记（2026-08-21 全面审查处理）**：主案 ①-④ 已实施，其中「仅
> run 边界重置（新 transport）」按正式路径修正为 `ModelGateway::
> for_new_run()` per-run 换新实例（见头部状态注记），并发会话之间零
> 干扰；「达 `DEGENERATION_LIMIT` 即终止」在 `generate_stream` 内为
> 独立分支（detail 带 `degeneration_limit_reached` 前缀，不依赖档位
> 判定与分支顺序）。disabled 档哨兵即终止（consecutive<3 时补前缀）
> 与达限终止（consecutive≥3 时前缀自带）两条路径均映射
> `run_invalidated{status: degeneration, detail}`。

在保留「请求内阶梯模式适配」的前提下，封死跨请求反复烧预算：

1. **会话级 thinking 档位**：reasoning 族哨兵触发后，本 run 后续请求的
   thinking 起始档 = 当前降级档（high 触发→后续请求从 low 起；low 再
   触发→后续请求从 disabled 起），**不再回到 config 默认档**。档位随
   run 单调下降，仅 run 边界重置。
2. **哨兵计数单调**：`degeneration_consecutive` 不再因成功请求清零
   （改为 run 内单调递增，仅 run 边界重置）；达 `DEGENERATION_LIMIT`
   （3）→ `run_invalidated{status: degeneration}`，detail 显式标明
   族（reasoning_stall / reasoning_repetition）+ consecutive + 触发轮次。
3. **disabled 档哨兵即终止**：降级到 disabled 后若哨兵再触发（无更低
   档可降）→ 立即显式终止（同上 run_invalidated），不再重试。
4. **显式标记纪律**：终止原因进入 journal 事件与 TUI（既有
   `degeneration_limit_reached` 前缀 + run_invalidated 映射复用）；
  不做静默空答案，不吞错误。

效果：单 run 的哨兵预算从「无界（每轮可烧 64K）」变为「有界（首档
64K 一次 + 低档快速 rep 拦截 + 达限终止）」。schemelike 类场景变为
1×64K stall + low 档继续工作（若 low 档可用则仍可通过，成本 1/4）；
持续病态 run 在第 3 次哨兵显式终止。

### 2.3 严格案（对照，暂不实施）：harness 原样 step fail-fast

reasoning 族哨兵触发即 step 失败（不做降级续跑），run 以显式
`run_failed{status: degeneration}` 收场。保留为对照项：若证据门通过后
主案 S4 显示「降级恢复后仍高失败率/仍烧预算」，再升级到严格案。

## 3. 参数与边界

- `DEGENERATION_LIMIT = 3` 不变（语义从"连续"改为"run 内累计"）；
- `STALL_REASONING_BUDGET_TOKENS = 64K` / `STALL_FIRST_CONTENT_TIMEOUT =
  600s` 不变（本设计不调兜底值；64K→32K 调校另议，见方案 C 讨论记录）；
- `REQUEST_MAX_TOKENS = 256K` 不变（方案 C 维持）；
- 空响应 D-6 链（完成型空响应快速重试 ≤2 → 降级）不变；
- content 族复读中断（已见输出）纪律不变：不重试、计数单调同本条
  设计、达限 run_invalidated。
- 与 [MIDSTREAM_DECODE_RETRY_DESIGN_2026-08-21.md](MIDSTREAM_DECODE_RETRY_DESIGN_2026-08-21.md)
  正交：解码重试管「无副作用传输失败」、本设计管「退化哨兵语义」；
  解码重试耗尽后的显式错误不受计数影响。

## 4. 实施路由与验证

1. S0 证据门：下一批扫描（r1-g2 起）采集哨兵上下文，按 §2.1 判定；
   **已通过（2026-08-21，§2.1 中间判定）**；
2. S1 代码：会话级 thinking 状态（transport/loop 共享）、计数单调化、
   disabled 档哨兵即终止、detail 显式化；**已实施**——`session_thinking`
   会话档位 + 计数单调（成功不清零）+ disabled 档终止 + detail 显式化
   （族 + consecutive + round）+ `for_new_run` per-run 隔离；
3. S2 测试：跨请求档位保持、计数不重置、达限 run_invalidated、
   disabled 档终止、回归全绿；**已闭合**（新增 3 项 + 既有回归，
   orz-loop 544 通过）；
4. S3 重建（Linux musl，ORZ-BUILD-MOUNT-001 契约；与窗口 180s +
   解码兜底批次合并一次到位）；
5. S4 复验：单 run 哨兵预算有界（**≤3 次触发 × 单次预算**，即最坏
   3×64K/3×600s——2026-08-21 全面审查修正口径，原「≤ 首档 64K + 低档
   快速拦截」为 schemelike 类场景期望非硬上限）、显式终止可观测
   （`run_invalidated{status: degeneration, detail}` + TUI 消息）、
   命中率不因误杀显著下降（≥90% 不变量）、零 400。

## 5. 关联登记

- 本设计定稿后：ADR-0010 §14.37、BACKLOG 0d、TODO P0-0d 后续、
  CLI_PROJECT_INDEX 登记。
