# 哨兵退化 fail-fast 化设计（2026-08-21 设计定稿；纯文档登记、未实施）

> 状态：`designed`（2026-08-21 用户裁决：harness fail-fast 方向有道理——
> 失败本身意味着当前模式不适合当前任务；**前置证据门**=确认空转/重复与
> 架构本身无关；确认后实施，显式标明终止原因）。
> 性质：P0-0d 后续（输出健康哨兵跨请求语义修订）+ 0e 观察延伸（sweep
> r1-g1 12 次哨兵归因）。关联：
> [DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md](DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)
> （哨兵/阶梯现状）、[ADR-0010 §14.35/§14.36](adr/ADR-0010-fusion-runtime-and-agent-architecture.md)、
> deepseek-harness 源码对照（tmp_dsh_review：agent-loop step 边界重试、
> llm-retry 默认 2 次、无降级/无生成期哨兵）。
> 实施路由：S0 证据门 → S1 代码 → S2 测试 → S3 重建 → S4 复验。设计轮
> 不动计数（27）。

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

### 2.2 主案：跨请求 fail-fast 化（哨兵预算有界 + 显式终止）

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
2. S1 代码：会话级 thinking 状态（transport/loop 共享）、计数单调化、
   disabled 档哨兵即终止、detail 显式化；
3. S2 测试：跨请求档位保持、计数不重置、达限 run_invalidated、
   disabled 档终止、回归全绿；
4. S3 重建（Linux musl，ORZ-BUILD-MOUNT-001 契约；与窗口 180s +
   解码兜底批次合并一次到位）；
5. S4 复验：单 run 哨兵预算有界（≤ 首档 64K + 低档快速拦截）、
   显式终止可观测、命中率不因误杀显著下降（≥90% 不变量）、零 400。

## 5. 关联登记

- 本设计定稿后：ADR-0010 §14.37、BACKLOG 0d、TODO P0-0d 后续、
  CLI_PROJECT_INDEX 登记。
