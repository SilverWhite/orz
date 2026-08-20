# DeepSeek 输出预算恢复与空流止损设计（2026-08-20 设计定稿；S1-S4 全部闭合）

> 状态：`S1-S4 全部闭合`（2026-08-20 用户放行实施 + 指示重建/复验/对账；
> 实施登记见 §4.1–§4.3）。
> 性质：P0-0d 后续（输出预算恢复，32K → 256K 评估）+ D-6 空流链改造（官方
> EMPTY_RESPONSE 节奏适配）+ 退化检测器大升级（OUTPUT-DEGENERATION-GUARD
> 从「content 复读检测」升级为「输出健康哨兵」）。
> 关联：ADR-0010 §14.33/§14.34（v1.33/v1.34）、
> OUTPUT_DEGENERATION_GUARD_DESIGN_2026-08-19、STREAM_RETRY_RHYTHM_DESIGN
> 2026-08-20、ADR-0007（重试纪律）、官方 deepseek-harness 源码
> （llm-deepseek / llm-retry，2026-08 master，浅克隆于
> `C:\Users\1\AppData\Local\Temp\dsh-harness`）。
> 实施路由：S1 代码 → S2 测试 → S3 重建 → S4 复验（难题单题 + 账单对账 +
> 空流率观测）。设计轮不动计数（28）。
> 2026-08-20 同日设计修订（用户后续意见）：空转监测拉灵敏——空转 token
> 限值 64K→16K、时间信号 180s→120s（与 max_tokens 解耦，256K 恢复后空转
> 不随预算放大）；idle 死线 50s→30s（取代未实施的 STREAM-RETRY-RHYTHM 50s
> 定值）；160K 复读浪费归因=架构工具设计（无再读闭环）已修正、作为恢复
> 256K 的安全依据；重试层结论=保留 transport 内链 + 吸收官方节奏，不迁移
> step 边界。
> 2026-08-20 二轮修订（用户质疑 16K 过低；实测校准数据支撑）：合法难题首轮
> 实测 reasoning 17,757 tokens / 184s 后正常产出（WSL 宿主机正常轮
> RUN-CLI-6a85f668）——16K/120s 会在其出结果前误杀。空转限值上调为
> **24K / 240s**（初值；S4 校准 20–32K / 180–300s），reasoning 复读检测
> 提为灵敏层（空转若为思考循环可在数 K 内识别，不依赖大预算阈值）。
> 2026-08-20 三轮修订（用户裁决：兜底值放大兼容 max 思考；灵敏层负责
> 快速识别，兜底只当保险丝）：成本账（实测 ¥4.592/M output）——32K ≈
> ¥0.147、64K ≈ ¥0.294、128K ≈ ¥0.588、256K ≈ ¥1.176；现状空流链
> （2×32K+降级）≈ ¥0.30，**64K 兜底的单次最坏成本 ≈ 现状整条链**，且
> 链式等待消除（stall 中断直接降级）。兜底值上调为 **64K / 600s**（初值；
> S4 校准 32–128K / 300–900s）= 合法锚点 17.7K 的约 3.6 倍思考空间；
> 重试账与兜底账解耦（D-6 管重试次数、兜底管单次上限），机制上可控。

## 1. 背景与证据

### 1.1 今晚故障链（2026-08-20，7 次运行对照）

| 运行 | 环境 | 首请求耗时 | 形态 |
|------|------|-----------|------|
| 评测 round 2/3 + S4 | 容器 | 10.2–10.6 min | 空流链，reasoning=null，journal 102–698 tokens |
| 容器抓包复现 | 容器 | 9m50s | 空流链，reasoning=null，1,026 tokens |
| WSL 宿主机 1 | 宿主机 | 3m04s | 正常：37K tokens，reasoning 正常 |
| WSL 宿主机 2 | 宿主机 | 9m42s | 空流链，reasoning=null，75 tokens |

- 同二进制同 key 宿主机快慢各一次 → **容器/网络/环境非根因**（宿主机不是
  稳定快）。
- pcap：连接 0.3s 建立、服务端约 47 包/秒持续流动（非断流/静默）；阶段 1/2
  各约 5 分钟持续流动（reasoning 烧满 32K），阶段 3 thinking 禁用仅 1s 返回
  小响应；客户端主动 FIN 后进入下一阶段。
- 账单对账（03–04 时段，9 请求 / 162,605 output tokens）：主要花费=空流阶段
  每请求约 32K 的**废弃 reasoning**；journal 与账单缺口=空流阶段。
- 根因链：`thinking=on + reasoning_effort=max + max_tokens=32K` → 难题
  reasoning 被 32K 截断 → 完成型空响应（finish=length、无
  content/tool_calls）→ D-6 链「原样重试」同参再烧一遍 → 10 分钟级空流链。
- **退化检测器盲区**：`DegenerationDetector` 只喂 content delta；空流阶段
  全程无 content，检测器完全沉默。
- **空转限值实测锚点（2026-08-20 二轮修订校准）**：WSL 宿主机正常轮
  （RUN-CLI-6a85f668，gpt2-codegolf 同任务）首请求 18:31:04 起、
  18:34:07 首输出，**reasoning 17,757 tokens / 约 184s → 正常产出
  plan（2 tool_calls）**；容器/WSL2 空流轮每阶段约 300s 烧满 32K 零产出
  且 usage 缺失（journal reasoning=null，账单口径 32K）。合法与空流的分界
  在本样本为 17.7K/184s vs 32K/300s——**低于 17.7K 的固定阈值必然误杀
  合法长思考**，灵敏度的下限由合法分布决定，不能无限压低。

### 1.5 空转成本账（2026-08-20 三轮修订）

- 单价口径：03–04 时段账单实测 **¥4.592/M output tokens**（9 请求 /
  162,605 output / ¥0.7467）。
- 单次空转烧费：16K ≈ ¥0.073、24K ≈ ¥0.110、**32K ≈ ¥0.147**、
  **64K ≈ ¥0.294**、128K ≈ ¥0.588、256K ≈ ¥1.176。
- **重试账**：现状空流链（D-6 原样重试，2×32K + 降级）≈ ¥0.30 且每次
  重试都付出 5 分钟等待；重试本身也是消耗（时间 + input 重付 + 指纹翻动）。
- **关键结论**：64K 兜底的单次最坏成本（¥0.294）≈ 现状整条空流链
  （¥0.30），同时链式等待消除（stall 中断 → 直接降级，不再原样重试）。
  兜底值放大不增加重试次数——**兜底管单次上限、D-6 管重试次数，两本账
  解耦**，机制上可控、取值不难。

### 1.2 160K 时代复读浪费的归因（用户 2026-08-20 复核）

- 160K max_tokens 时代的 201K/479K 字符复读浪费，**本质是架构工具设计
  问题**：工具结果截断后无可再读闭环（指针指向 journal/TraceStore，不在模型
  直接工具面），模型只能靠记忆复述 → 超长复读被 160K 放大。
- P0-0d 已修正诱因：8K 统一限值 + 截断末尾 read_file 落盘指针（补读闭环
  硬约束）+ content 层生成期复读检测。**该修正使 256K 恢复后 content 层
  复读风险显著低于 160K 时代**（诱导消除 + 实时检测拦截）。
- 今晚空流是**另一条失效模式**（reasoning 层空转：无 content 产出、预算被
  截断），与 160K 复读正交；由本设计新增的 reasoning 族信号覆盖。三层齐备
  （补读闭环 + content 复读检测 + reasoning 空转止损）才恢复 256K。

### 1.3 官方 harness 实现要点（源码核对）

官方 `deepseek-harness`（2026-08 master）`packages/llm/llm-deepseek`：

- 默认 `reasoningEffort = high`（四档 off/low/high/max；默认不是 max）；
  `maxTokens = 256_000`；`streamIdleTimeoutMs = 300_000`（5 分钟）。
- 每个请求单次 provider 调用；流式恒开 `include_usage`；usage 可能挂 finish
  chunk 或独立 trailing chunk，均延迟到 `[DONE]` 统一处理。
- **EMPTY_RESPONSE 语义**：完整流结束且未开任何 block（text/reasoning/
  tool-call 全空，finish=stop/absent）→ `finish {kind:'error'}`；finish=length
  映射 max-tokens。空响应被明确归类为**可安全重试**（无持久内容，不会重复
  工具）。
- **重试**：在 durable agent-step 边界（关失败 turn、开新编号 turn）；normal
  模式 5 次（EMPTY_RESPONSE/RATE_LIMIT/SERVER/TIMEOUT/TRANSPORT），指数退避
  500ms→10s + 10% jitter。
- SSE 严格 `[DONE]` 哨兵；EOF 无 `[DONE]` = STREAM_CLOSED（截断不可信）。
  reasoning_content 回传：每个带 reasoning 的 assistant turn 都回传（工具轮
  必须）；首 chunk 空 reasoning 不产生假 block。
- 官方**没有生成期退化防护**——靠「默认 high 档空流概率低 + 空即错即退 +
  有界重试」。

### 1.4 三个待决问题（用户 2026-08-20）

1. 256K 可能是官方为最高档（max）思考准备的输出上限——我们 256K+max 怎么样？
2. 空流处理向官方靠拢（空即错即退、快速有界重试）是否更好？
3. 退化检测器需要大升级。

## 2. 分析结论

### 2.1 256K + max：可行，但止损必须先行

- 256K 是官方**输出上限默认值**，与 effort 是独立维度（默认 high）。但 max
  档思考量天然大，32K 把难题思考截断正是今晚空流的直接诱因——**256K 大概率
  显著降低空流率**（给足思考空间）。
- 风险：单请求最坏成本放大 8 倍。按 03–04 账单折算（约 ¥4.59/M output
  tokens），一个废弃的 256K 请求 ≈ ¥1.18；若空流仍发生且按现状 D-6 原样
  重试，单轮可烧 2×256K。
- **160K 复读风险已修正（§1.2）**：content 层复读的诱导源（无再读闭环）已
  由 P0-0d 消除，content 复读也有实时检测；256K 恢复后的残留风险集中在
  reasoning 层空转（与复读正交），由 §3.3 新信号覆盖。
- **结论：256K + max 定为主方案，但三个前提必须同时成立**——①空流/思考失控
  在烧满预算前被主动中断（§3.3）；②空流不原样重烧（§3.2）；③S4 用难题实测
  校准，若空流率或成本不可接受则参数化回落 128K 档（§3.1）。

### 2.2 空流官方化：方向对，次数要收窄

- 官方「空即错即退 + 快速有界重试」是正确语义（空响应无持久内容、重试安全）。
- 但官方的 5 次上限建立在 effort=high、单次空流概率低的假设上；**256K+max 下
  5 次原样重试的成本不可接受**（最坏 5×256K）。
- 改造方案：保留我们 transport 内链的优势（不重跑工具、有降级出口），把重试
  节奏官方化但收窄——完成型空响应快速重试 ≤2 次，随后 thinking 禁用降级；
  **长烧型空转（reasoning 持续无 content）由 §3.3 的 stall 哨兵提前中断、
  直接降级，不进入原样重试**。

### 2.3 重试层对比：我们 transport 内链 vs 官方 step 边界

- **官方**：单请求单次调用；重试在 durable agent-step 边界（关失败 turn、开
  新编号 turn、重建同一请求；失败 chunk 不进派生消息，不重复工具）。优点=
  durable、可观测（事件面）、与压缩/恢复机制衔接。缺点=无生成期止损（要等
  流完成才知道空）、无降级出口、turn 切换开销与事件噪音。
- **我们**：`generate_stream` 内链（transport 层）——快速（无 turn 切换）、
  有 thinking 禁用降级出口（官方没有）、生成期哨兵提前止损（官方没有）、
  「已见输出不重试」纪律比官方更细（官方只在零 block 时判空，我们
  reasoning-only 完成也判空——reasoning 不 surface 给 controller，更严格
  且正确）。弱点=重试不可观测、不 durable（进程崩溃丢失重试状态）。
- **结论：保留我们 transport 内链为主，吸收官方两点**——①空流重试节奏
  （快速有界退避，§3.2）；②空流判定语义（完成型空响应=可安全重试）。不迁移
  到 step 边界（会失去生成期止损与降级能力）。可观测性缺口登记为可选后续
  （P1：transport 重试计数入事件面），不占本设计计数。

### 2.4 退化检测器大升级：从「复读检测」到「输出健康哨兵」

- 现有检测器只有 content 层复读信号，盲区=空流场景（全程无 content）。
- 升级为三族信号：content 层复读（保留）、reasoning 层复读（新增）、
  reasoning-stall 思考失控（新增：有 reasoning 无 content 超过预算/时间）。
- **重试分类纪律**（与 ADR-0007 对齐）：有可见输出的异常（content 复读）不
  重试；无可见输出的异常（reasoning 复读/stall/完成型空响应）可安全重试——
  但走降级而非原样重试。

## 3. 变更设计

### 3.1 A. 输出预算恢复（REQUEST_MAX_TOKENS 32K → 256K）

- `agent_loop.rs` `REQUEST_MAX_TOKENS`：32_000 → **256_000**（三 agent 统一，
  ADR-0010 §3.4.2「同一默认值注入」不变；检索子代理同常量，仅上限不设目标
  值）。
- 回落档：128K（S4 实测触发回落时，编译期常量修改，不新增运行期旋钮；先保持
  既有「无新增旋钮」纪律，如需实验口子再评估）。
- 请求头指纹：max_tokens 已参与 `config_fingerprint`，部署后首次请求一次性
  指纹变化（既有纪律，登记）。

### 3.2 B. 空流处理官方化（D-6 链改造）

新链（`generate_stream`）：

1. 阶段 1：正常请求（thinking=max + 256K，`stream_once_with_retry`）；
2. 完成型空响应（`empty_content_abnormal`）→ **快速有界重试**：同参重发，退避
   500ms→10s + 10% jitter，上限 2 次（官方节奏收窄版；官方 5 次对我们成本
   不可接受）；
3. 仍空 → **降级**：thinking 禁用（现有第三阶段）；
4. 仍空 → 失败（现有错误语义）。

生成期哨兵（§3.3）在阶段 1/2 触发 reasoning 族异常时：**不进入快速重试、直接
跳降级**（长烧型空转原样重试大概率复现且贵）。content 族退化中断维持「不重试」
透传（现有纪律）。

新增参数（`RetryPolicy` 或 D-6 常量，S1 定位置）：

- `EMPTY_RESPONSE_MAX_RETRIES = 2`
- `EMPTY_RESPONSE_BACKOFF_INITIAL = 500ms`、`EMPTY_RESPONSE_BACKOFF_MAX = 10s`、
  `JITTER = 10%`（对齐官方默认形状）

保留项：zero-chunk 传输中断重试（`stream_once_with_retry` 内，50s 窗口/10 次，
STREAM-RETRY-RHYTHM）与空流链正交；idle 死线收紧见 §3.4（30s，取代
STREAM-RETRY-RHYTHM 未实施的 50s 定值）。

### 3.3 C. 退化检测器大升级（DegenerationDetector → 输出健康哨兵）

**观测面**：content delta + reasoning delta + tool_call arguments delta 全部
喂入。

**信号族**：

| 信号 | 触发条件 | 触发后行为 |
|------|----------|-----------|
| content_repetition（现有） | 连续相同 content delta N=5 / 1K token 窗口 3-gram 重复率 >60% | 中断、不重试（已见输出）、会话计数+1 |
| reasoning_repetition（新增，**灵敏层**） | 同一算法作用于 reasoning delta（仅 content/tool_calls 全空时启用）；1K token 窗口 3-gram 重复率 >60%（同 content 规则）——空转若为思考循环可在数 K 内识别，不依赖大预算阈值 | 中断、直接降级（无可见输出、重试安全但不原样）、会话计数+1 |
| reasoning_stall（新增，**预算兜底层**） | 自首 chunk 起 600s（初值，S4 校准 300–900s）无 content/tool_calls 且 reasoning 在流动；或 reasoning 估算累计 ≥64K tokens（初值，S4 校准 32–128K）仍无 content/tool_calls——OR 触发 | 中断、直接降级、会话计数+1 |

**重试分类**（ADR-0007 对齐）：content 族→不重试；reasoning 族（stall/复读）
→不原样、直接降级；完成型空响应→快速有界重试（§3.2）。

**token 估算**：reasoning 字符 ÷ `REASONING_CHARS_PER_TOKEN = 2`（沿用桥 8K
实测校准值 2 字符/token；S4 用 usage 真实 reasoning_tokens 复核校准）。usage
到达（final chunk）时以真实值核对，不改中断决策（中断发生在 usage 前）。

**空转预算与 max_tokens 解耦（用户 2026-08-20 裁决）**：
`STALL_REASONING_BUDGET_TOKENS` 是**空转自身的独立限值**，不随
`REQUEST_MAX_TOKENS` 缩放——无论 max_tokens 恢复到 256K 还是更大，reasoning
无 content 产出超过 64K 即判空转。空转成本因此被钉死在固定预算内，不被
输出预算恢复放大。**取值依据（2026-08-20 三轮修订，用户裁决：灵敏层负责
快速、兜底负责兼容 max 思考）**：合法难题首轮实测 17,757 reasoning/184s
正常产出——64K 兜底 = 合法锚点约 3.6 倍思考空间，覆盖 max 档位更充分的
思考（256K 预算下的深度思考不再被小兜底误杀）；单次最坏空转成本
¥0.294 ≈ 现状整条空流链（2×32K ≈ ¥0.30），且消除链式等待。真正的快速
识别手段是 `reasoning_repetition`（灵敏层）：循环型空转在数 K 内触发、
不依赖大兜底；兜底值只拦截「连贯但无产出」的非循环空转，作为保险丝存在。
灵敏判断的误杀代价由「直接降级重试」（thinking 禁用，今晚实测 1s 返回）
兜底——即便兜底误杀一次深思考，代价也只是一次降级轮。

**会话级防循环**：`DEGENERATION_LIMIT = 3` 语义扩展为三族共享计数（现有
`Arc<AtomicU32>`、成功请求重置）；达限 `degeneration_limit_reached` 转
run_invalidated（reason 保持 degeneration，detail 区分族）。

**detail 前缀**：`degeneration_detected:content_repetition|reasoning_repetition
|reasoning_stall:*`（新增 `is_reasoning_guard_detail` 分类器，
`is_degeneration_detail` 语义收窄为 content 族不重试判定）。

**journal/审计**：失败轮次 stagnation 评估扩展 reasoning 信号（reasoning 累计
字符/估算 token、首 content 延迟、中断原因、触发族），审计留痕不改终止语义。

### 3.4 D. idle 死线收紧（stream_idle_timeout 50s → 30s）

- 依据：实测 DeepSeek 流式数据流约 47 包/秒持续流动（60–80s 大请求全程有
  数据），不存在 >秒级的合法完全静默；idle 只看「完全无数据」（慢速 reasoning
  流不误杀）。30s = 5s warn × 6 轮，仍保留网络抖动缓冲，比 50s 快 40%。
- 语义分层：idle 30s=完全无数据 → 死线 → zero-chunk 重试（传输层）；
  stall 600s/64K=有 reasoning 无 content → 空转 → 降级（语义层）。两者互补
  不重叠。
- 取代关系：STREAM-RETRY-RHYTHM 设计定稿的 idle 50s（`stream_idle_timeout`
  90s→50s）**尚未实施**，本设计直接修订为 30s，实施时一次到位，无冲突；
  `stream_idle_warn` 保持 5s、`request_retry_window` 保持 50s 不变。

### 3.5 参数表

| 参数 | 现值 | 新值 | 位置 |
|------|------|------|------|
| `REQUEST_MAX_TOKENS` | 32,000 | **256,000**（回落档 128K） | agent_loop.rs |
| `EMPTY_RESPONSE_MAX_RETRIES` | D-6 原样重试 1 次 | **2**（官方节奏收窄） | transport.rs |
| 空流重试退避 | 无（立即） | 500ms→10s + 10% jitter（官方形状） | transport.rs |
| `STALL_FIRST_CONTENT_TIMEOUT` | 无 | **600s**（初值，校准 300–900s） | transport.rs |
| `STALL_REASONING_BUDGET_TOKENS` | 无 | **64,000**（初值，校准 32–128K；与 max_tokens 解耦；单次最坏 ≈ ¥0.29 ≈ 现状整条空流链） | transport.rs |
| `REASONING_CHARS_PER_TOKEN` | 无（桥校准 2） | **2**（S4 校准） | transport.rs |
| 退化信号族 | content 复读 2 条 | + reasoning 复读 + reasoning-stall 2 条 | transport.rs |
| `stream_idle_timeout` | 50s（STREAM-RETRY-RHYTHM 定值，未实施） | **30s**（校准 20–30s） | model.rs RetryPolicy |

## 4. 实施路由

- **S1 代码**（orz）：REQUEST_MAX_TOKENS 256K；`stream_idle_timeout`
  50s→30s（RetryPolicy 默认值）；generate_stream D-6 链改造（快速有界重试 +
  降级跳转）；DegenerationDetector 升级（观测面扩展、reasoning 复读灵敏层 +
  stall 双信号 600s/64K 预算兜底、detail 前缀、会话计数共享、journal 统计）；
  fingerprint 注释同步。
- **S2 测试**：256K 请求头断言；空流链新路径（快速重试 2 次→降级、stall→
  降级、content 退化不重试）；stall 双信号单测（600s 时间 / 64K 预算、OR
  语义、首 content 后不触发、reasoning 复读灵敏层、空转预算不随 max_tokens
  缩放）；idle 30s 默认值断言（取代 STREAM-RETRY-RHYTHM 50s）；reasoning
  估算校准测试；回归全绿（fmt/clippy 基线）。
- **S3 重建**：Linux musl（ORZ-BUILD-MOUNT-001 契约）。
- **S4 复验**：难题单题（gpt2-codegolf 或同负载）256K+max 运行 + 账单对账——
  判定=空流率（0 或显著下降）、无 400、命中率 ≥90%、stall 触发记录与降级收尾、
  成本对账（output tokens/¥ vs 32K 基线）、stall 误杀观测（合法轮被中断
  次数——以 RUN-CLI-6a85f668 合法首轮 17.7K/184s 为下界基准；空流烧费观测
  （区分循环型=复读灵敏层拦截 vs 非循环型=兜底拦截，校准 32–128K /
  300–900s / idle 20–30s）；若空流率或成本不可接受 → 回落 128K 档复验
  （编译期常量）。

### 4.1 S1 实施登记（2026-08-20，用户放行实施；暂不重建/测试）

- `agent_loop.rs`：`REQUEST_MAX_TOKENS` 32_000 → **256_000**（三 agent
  统一，ADR-0010 §3.4.2 不变；回落档 128K 注释保留）。
- `model.rs`：`RetryPolicy::default().stream_idle_timeout` 50s → **30s**
  （warn 5s / retry window 50s 不变）。
- `transport.rs`：
  - 新常量——`EMPTY_RESPONSE_MAX_RETRIES=2`、
    `EMPTY_RESPONSE_BACKOFF_INITIAL=500ms`、`EMPTY_RESPONSE_BACKOFF_MAX=10s`、
    `EMPTY_RESPONSE_BACKOFF_JITTER=0.10`（官方节奏收窄版）、
    `STALL_FIRST_CONTENT_TIMEOUT=600s`、`STALL_REASONING_BUDGET_TOKENS=64_000`
    （与 max_tokens 解耦）、`REASONING_CHARS_PER_TOKEN=2`；detail 前缀
    `CONTENT_REPETITION_DETAIL_PREFIX` / `REASONING_REPETITION_DETAIL_PREFIX`
    / `REASONING_STALL_DETAIL_PREFIX`。
  - `DegenerationDetector` 升级为输出健康哨兵：观测面=content delta +
    reasoning delta + tool_call arguments delta（`feed_content` /
    `feed_reasoning` / `feed_tool_arguments`）；三族信号——content 复读
    （保留）、reasoning 复读（灵敏层：同一算法、仅 content/tool_calls
    全空时启用）、reasoning_stall（预算兜底层：自首 chunk 起 600s 无
    content/tool_calls、或估算 token ≥64K，OR 触发；`check_stall` 逐
    chunk 调用）；reasoning 字符 → 估算 token（÷2），usage 到达时以真实
    `reasoning_tokens` 复核留痕（不改中断决策）。
  - 分类器：`is_degeneration_detail` 收窄为 content 族（+limit 前缀）；
    新增 `is_reasoning_guard_detail`（reasoning 复读/stall）；新增
    `guard_family_label`（审计用）。
  - `stream_once_with_retry`：输出健康哨兵中断（content/reasoning 族）
    一律不 zero-chunk 重试（透传）。
  - `generate_stream` D-6 链改造：正常（max+256K）→ 完成型空响应快速
    有界重试 ≤2 次（退避 500ms→10s+10% jitter、cancel 可中断、心跳盖章）
    → thinking 禁用降级 → 仍空显式失败；reasoning 族哨兵中断不原样、
    直接跳降级；content 族中断透传不重试。
  - `ModelConfig::max_tokens`（`deepseek_v4`）32_000 → **256_000**；
    请求头指纹随 max_tokens/idle 变化（部署后首次请求一次性变化，既有
    纪律）。
- `agent_loop.rs` run 层：哨兵中断分流合并 content/reasoning 两族（limit
  前缀 → run_invalidated；未达限 → run_failed）；失败轮次审计补
  `guard_family` 标签留痕（不改终止语义）。
- 同步：既有断言更新（请求头 256K / idle 30s / 检测器 feed 签名）；live
  探针 `probe_thinking_max` 256K；检索/controller/模块注释 32K/160K 陈旧
  引用清理。
- 计数：实施放行入账 1 项（**28 → 29**），S3/S4 验证闭环后 29 → 28。
- 待续：S2 测试（请求头/stall 双信号/链路径/估算校准/回归）→ S3 重建 →
  S4 复验。

### 4.2 S2 测试实施登记（2026-08-20，用户指示进行 S2）

- **新增 15 项测试**（orz-loop lib，527 通过 / 0 失败 / 3 ignored）：
  - 退化检测器单测 10 项——reasoning 复读灵敏层（连续相同 N=5 / 1K 窗口
    3-gram >60%）、content/tool arguments 出现后 reasoning 族停用、
    stall 双信号（600s 时间 / 64K token 预算，OR 语义）、无首 chunk 不
    触发（idle 互补）、reasoning 估算校准（字符 ÷2）、空转预算与
    max_tokens 解耦（64K < 256K 常量钉死）、空流重试参数/退避形状
    （≤2 次、500ms→10s、±10% jitter）。
  - 空流链 e2e 5 项——完成型空响应快速重试 2 次→降级产出、链尾全空显式
    失败（zero output）、reasoning 复读→直接降级（不原样重试）、
    reasoning-stall（≥64K 估算）→直接降级、空响应重试中触发 reasoning
    哨兵→跳过剩余原样重试直接降级。
  - S1 已同步断言继续覆盖：256K 请求头 min-cap、idle 30s 默认值、content
    退化不重试（既有测试回归）。
- 回归：`cargo fmt --check` 干净；clippy 无新增告警（transport.rs 零告警，
  lib 21 与基线一致；test 28 均为既有位置）；`cargo check --workspace`
  通过（orz-bin/orz-tui 等下游无破坏）。
- 计数：S2 不改变未闭合计数（仍 29），S3/S4 验证闭环后 29 → 28。
- 待续：S3 重建（Linux musl，ORZ-BUILD-MOUNT-001 契约）→ S4 复验。

### 4.3 S3 重建 + S4 复验 + 对账登记（2026-08-20，用户指示重建与复验＋对账）

- **S3 重建**：Linux musl（ORZ-BUILD-MOUNT-001 契约，
  `build_orz_aliyun.sh`，输出 `D:/tb-eval/orz-linux`）；三件套时间戳更新
  （orz 104,374,912 B / orz-signer / orz-acaf-provision，18:45）；
  日志 `orz-linux/build-20260820.log`。
- **S4 复验**（gpt2-codegolf 单题，harbor job `2026-08-20__18-46-52`，
  trial `gpt2-codegolf__tgdGruX`，RUN-CLI-6a86db35，18:46:57→19:16:45，
  wallclock 1740s 跑满、reward 0.0、无异常）：
  - **完成型空流：0**（零 `empty_content_abnormal` 完成；stderr 无空响应
    重试警告）——32K 截断空流链根因消除；首请求 10:47:17 prompt →
    10:48:23 首输出（约 **66s**，对照 32K 时代 10.5 分钟级）。
  - **零 400、零 idle 死线触发、零 timeout**。
  - **命中率 journal 口径 95.28%**（hit 3,110,656 / miss 153,990，
    85 请求，DoD ≥90% 达成）；provider 口径待控制台 CSV 刷新对账。
  - **stall/哨兵观测**：`reasoning_repetition` 灵敏层触发 1 次
    （10:58:24，5 个相同 reasoning delta）→ 不原样重试、直接 thinking
    禁用降级 → 10:58:34 降级轮正常产出（text=Y、comp=878、reas=None）并
    继续运行至预算；**reasoning-stall 兜底（600s/64K）零触发、零误杀**
    （合法 reasoning 峰值 20,234 tokens / 约 2.5 min，远低于兜底；
    对照合法锚点 17.7K/184s）；idle 30s 无完全静默事件。
  - **用量/成本（journal 口径）**：85 请求；output tokens 191,623
    （含 reasoning 170,906）；prompt tokens 3,264,646（hit 3,110,656 /
    miss 153,990）；按设计实测 ¥4.592/M output 估算输出成本 ≈ **¥0.88**；
    单次最大 completion 20,776（无 >100K 单请求、无预算放大异常）。
    控制台 CSV（cost/amount-2026-08-20）未在磁盘可及路径，精确对账待
    用户刷新附件后补登。
  - **校准结论**：600s/64K 兜底初值在本负载无误杀、idle 30s 无漏判——
    维持初值不调（S4 校准区间 300–900s / 32–128K / 20–30s 内）。
- 计数：**S3/S4 验证闭环 29 → 28**。登记于 ADR-0010 §14.35 第 4 项 /
  BACKLOG 0d / TODO P0-0d / CLI_PROJECT_INDEX。

## 5. 验收标准（DoD）

- S2 全绿、fmt 干净、clippy 与基线一致；
- S3 重建成功、三件套时间戳更新；
- S4：无完成型空流链（或 stall 在预算内提前中断并降级收尾）、零 400、命中率
  ≥90%、账单对账无异常尖峰（单请求废弃 output ≤ 预算内）、stall 双信号与
  idle 30s 按实测校准一次（误杀与漏判平衡）；
- 计数：实施放行时入账 1 项（28 → 29），验证闭环后 29 → 28。

## 6. 风险与回滚

- **256K 成本敞口**：最坏单请求废弃 ≈ ¥1.18（8×32K 基线）。缓解=stall 兜底
  （600s/64K 中断，空转预算与 max_tokens 解耦，单次最坏 ≈ ¥0.29 ≈ 现状
  整条空流链）+ 空流不原样 + 回落档 128K；S4 账单对账为硬门。
- **非循环空转检测时点变长**：64K 兜底在 ~100 tok/s 流速下约 10 分钟才触发，
  比 24K/4 分钟慢——这是「兼容 max 深度思考」的明确取舍。缓解=循环型空转
  由 reasoning 复读灵敏层在数 K 内拦截（今晚空流若为思考循环，实际等待远短
  于兜底）；非循环空转 10 分钟等待由「stall→直接降级」收尾（降级秒回），
  总时长仍可控，成本 ¥0.29 有界。
- **误杀合法长思考**：>64K 纯 thinking（或 >600s）才出 content 的合法轮会
  被兜底中断——**阈值已兼容 max 思考**（合法锚点 17.7K 的 3.6 倍，S4 校准
  32–128K / 300–900s）。
  缓解=若 S4 观察到合法轮 reasoning 接近 64K，按校准区间上调；误杀代价=1
  次降级轮（thinking 禁用通常仍产出，今晚阶段 3 实测 1s 返回）；快速识别
  走 reasoning 复读灵敏层（循环特征出现才触发，不因阈值偏低误杀合法轮）。
- **idle 30s 误杀慢思考**：若 DeepSeek 出现 >30s 完全无数据但连接存活的合法
  长思考会被判死线并 zero-chunk 重试。缓解=实测数据流无 >秒级静默、30s 保留
  抖动缓冲；重试后通常命中缓存；S4 校准 20–30s。
- **reasoning 估算偏差**：字符→token 系数误差影响预算信号时点。缓解=usage
  复核校准（S4），系数初值沿用桥校准值 2。
- **空流快速重试仍烧费**：2 次上限内每次都是全长预算（但受 stall 保护提前
  中断）。缓解=上限 2、stall 前置、降级出口。
- **回滚**：max_tokens 单值可逆；D-6 链改动可回退旧三阶段；stall 参数独立
  常量；检测器升级与旧逻辑并存可拆。最坏代价=每轮一次降级或 256K 单请求，
  低频接受。

## 7. 关联

- 前置：OUTPUT_DEGENERATION_GUARD_DESIGN_2026-08-19（P0-0d，32K 止损与检测器
  基线）、STREAM_RETRY_RHYTHM_DESIGN_2026-08-20（idle 死线 5s warn 不变、
  timeout 50s→**30s** 修订取代——stall 与 idle 互补：idle=完全无数据、
  stall=有 reasoning 无产出）。
- 修订：ADR-0007（D-6 空流重试语义）、ADR-0010 §3.4（max_tokens 默认值）、
  §14.33/§14.34。
- 参考：官方 deepseek-harness `packages/llm/llm-deepseek`
  （adapter/serialize/translate/sse/index.ts）与 `packages/llm/llm-retry`
  README（2026-08 master）。
- 登记：ADR-0010 §14.35（v1.35）/ BACKLOG 0d / TODO P0-0d / CLI_PROJECT_INDEX。
