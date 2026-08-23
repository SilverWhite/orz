# DeepSeek 输出预算恢复与空流止损设计（2026-08-20 设计定稿；S1-S4 已闭合，修订：默认 high + 三级降级梯已实施闭环）

> 状态：`S1-S4 全部闭合（max 基线）`；**2026-08-20 修订定稿（用户裁决：
> 方案 B + 中间档）——默认 thinking 档 max → high（官方默认），降级梯
> 插入 low 中间档（high → low → disabled → 失败），max 保留为可选档；
> S1 代码 + S2 测试（§4.5）、S1 全面审查处理（§4.6）、S3 重建 + S4 复验
> （§4.7，换题 make-doom-for-mips）已全部闭合；计数 29 → 28**。
> **2026-08-21 修订（用户裁决：同意滚动哈希任意偏移复读检测；S1 代码
> + S2 测试已实施（§4.9/§4.10）、S3-S4 待续）**：退化检测器复读判定
> 路径①（连续相同
> delta N=5）替换为滑动窗口滚动哈希任意偏移检测（§3.3 修订 / §4.8/§4.9）
> ——dna-assembly 复跑误杀实证（低熵 DNA 文本 + 小 chunk 粒度天然命中
> 5 相同 delta）；3-gram 路径②保留兜底；stall 兜底与 fail-fast 纪律
> 不变。S1 实施放行入账 28 → 29；S2 不改变计数。
> 实施登记见 §4.1–§4.3/§4.5/§4.9/§4.10。
> **2026-08-22 修订（用户裁决：灵敏层复读判定再校准——L=200 + 流内
> 累计 3 次命中才中断+降级；content/reasoning 统一；S1/S2 已实施
> （§3.3 修订 / §4.11）、S3-S4 待续）**：缺口 A 实证 48 字符粒度对
> 正常任务内容引用误杀（DNA 序列等式/技术短语 span），再校准
> `REPETITION_MIN_RUN_CHARS` 48→**200**、`REPETITION_WINDOW_CHARS`
> 96→**400**（缓冲 144→600）、新增 `REPETITION_HIT_LIMIT=3`——滚动
> 窗口命中后继续喂入、1–2 次命中仅审计留痕 WARN、计数随流结束丢弃；
> 3-gram/stall 兜底与 fail-fast 纪律不变。S1/S2 实施登记见 §4.11；
> 实施不改变计数（仍 29，S3/S4 闭环后 28）。
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
| content_repetition（现有） | 滑动窗口滚动哈希任意偏移：800 字符窗口内出现与记录区相同 400 字符 L-gram（起点距离 ∈ [400,800]；哈希命中后字符级比对），命中候选须过二级「标点块内部重复确认」（按标点+空白切块、内部重复块覆盖占比 ≥0.50 或无可切分点）才计命中，**流内累计确认命中 ≥3 次才触发（1–2 次仅审计留痕）** * / 1K token 窗口 3-gram 重复率 >70%（保留兜底，**每次 feed 超阈值计 1 次流内命中、累计 ≥3 才触发、1–2 次仅审计留痕**、不经二级） | 中断、不重试（已见输出）、会话计数+1 |
| reasoning_repetition（新增，**灵敏层**） | 同一算法作用于 reasoning delta（仅 content/tool_calls 全空时启用）；滚动哈希任意偏移（同 content 规则：L=400/W=800、二级确认、流内累计 ≥3 次确认命中）+ 1K token 窗口 3-gram 重复率 >70%（同 content：流内累计命中 ≥3 才触发、1–2 次仅审计留痕）——空转若为思考循环可在数 K 内识别，不依赖大预算阈值 | 中断、直接降级（无可见输出、重试安全但不原样）、会话计数+1 |
| reasoning_stall（新增，**预算兜底层**） | 自首 chunk 起 600s（初值，S4 校准 300–900s）无 content/tool_calls 且 reasoning 在流动；或 reasoning 估算累计 ≥64K tokens（初值，S4 校准 32–128K）仍无 content/tool_calls——OR 触发 | 中断、直接降级、会话计数+1 |

> *2026-08-21 修订：路径①（连续相同 delta N=5）由滚动哈希任意偏移检测
> 取代，详下。
> **2026-08-22 修订（再校准，用户裁决）：L 48→**200**、比较区 96→**400**
> （缓冲 144→600）、触发门槛改**流内累计命中 ≥3 次**（间隔不重置；
> 1–2 次仅审计留痕），content/reasoning 统一，详下。**
> **2026-08-23 修订（二级再校准，用户裁决）：L 200→**400**、比较区
> 400→**800**（缓冲 600→1200）、新增二级「标点块内部重复确认」（标点+
> 空白切块、重复块覆盖占比 ≥0.50 初值才计命中、每对都过、无切分点直接
> 判真），详下。**
> **2026-08-23 修订（NGRAM-GUARD-CALIBRATION，用户裁决）：3-gram 路径②
> 阈值 0.60→**0.70**（`>` 严格大于保留）+ 流内累计命中 ≥`NGRAM_HIT_LIMIT=3`
> 才 trip（1–2 次仅审计留痕=ratio+窗口 token 数+族、间隔不重置、流结束
> 丢弃）+ WARN 口径 `{:.2}`→`{:.3}`，详下；设计权威=
> `NGRAM_GUARD_CALIBRATION_DESIGN_2026-08-23.md`。**

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

**2026-08-21 修订：复读判定粒度改滚动哈希任意偏移（用户裁决：同意实现；
S1 代码已实施（§4.9）、S2 测试待续）**

**误杀实证（dna-assembly 复跑，RUN-CLI-6a885faa，11m57s、reward 0）**：
触发点 seq 95 完整输出 6439 字符，为**连贯正常**的 DNA 组装分析
（finish_reason=length、无 tool_calls）；无 ≥20 字符连续重复，但含 DNA
低熵特征（`ttttt`、`aaaaa`×多处、`ggggg`、`N N N N N`、`GGTCTC`）。
现有路径①「连续 5 个相同 content delta」在 DeepSeek 小 chunk 粒度 +
低熵文本下天然命中（detail=`5 identical content deltas in a row`）——
5 个相邻 chunk 内容相同即可触发，与真实复读无关。用户裁决：**误杀必须
处理，判定粒度提高，防真正的复读**。

**决策链**：

- 否掉「拉长 N」：治标不治本——阈值提高只是推迟触发，真复读同样更迟钝。
- 否掉「内容熵过滤」：真复读（同一段 DNA 反复输出）同样低熵，会被放过。
- 否掉「重复字符后自动切块」：语义不自洽——DNA 恒 4 字符多样性导致块长
  内容依赖；短周期（2–4 字符）重复可漏网。
- 否掉「固定 16 字符切块 ×3 相同 = 48 字符」：**相位对齐缺陷**——周期
  10 的短语循环在 16 字符窗口下相邻窗口永不相同（10 与 16 不同相），
  固定偏移比较对周期性复读系统性漏检；固定偏移 L=48 比较同样有此缺陷。
- **定案：滑动窗口 + 滚动哈希任意偏移检测**。

**判定语义**：流中存在 ≥48 字符内容与邻近之前的 48 字符完全相同（两个
48 字符 L-gram 起点距离 ≥48）。

**实现要点**：

- 维护最近 L+W=144 字符缓冲（尾部 48 字符 L-gram + 最近 96 字符比较区），
  记录比较区内（起点偏移 ∈ [48, 96]）全部 48 字符 L-gram 的滚动哈希；
  每新字符到达，尾部新 L-gram 的哈希若在比较区内已见（起点偏移 ≥48）→
  触发；哈希命中后以字符级直接比对确认（防碰撞误报）。
- **窗口内任意周期可命中（p ≤ 96）**：周期 p 的循环内容，任取同相且
  起点差 ∈ [48, 96] 的两个 L-gram 即触发，不受固定偏移相位错开影响
  （修复对齐缺陷）。
- **同字符连串（poly-A）需 ≥96 个连续相同字符**（两个相邻 48 字符块
  完全相同）才触发；少于 48 的连串天然免疫。
- **DNA 正常序列免疫**：96 字符窗口内出现两个完全相同 48 字符子串的概率
  趋近于零（随机 DNA ≈ 4⁻⁴⁸ 量级）；低熵短特征不构成 48 字符重复块。
- 复杂度：滚动哈希 O(1)/字符（摊销），内存 O(W)。

**作用域**：

- content 与 reasoning 两族共用同一算法（`feed_repetition` 共用核心，
  feed 签名不变）；reasoning 灵敏层语义不变（仅 content/tool_calls 全空
  时启用）——循环型空转（模型反复输出同段落）仍在数 K 内可识别。
- 3-gram 路径②（累计 ≥1K token 且最近 1K token 内 3-gram 重复率 >60%）
  **保留为兜底**（OR 关系；是否同步调整另行裁决）。
- stall 预算兜底（600s/64K）与 fail-fast 纪律（会话级档位/计数单调/
  disabled 即终止/显式 detail）**不变**——本修订只改复读判定粒度，不动
  终止纪律与计数语义。
- 新常量：`REPETITION_MIN_RUN_CHARS=48`（L）、
  `REPETITION_WINDOW_CHARS=96`（=2L）；`DEGENERATION_CONSECUTIVE_DELTAS=5`
  路径①语义被取代。
- detail 前缀不变（`content_repetition` / `reasoning_repetition`），触发
  描述文本更新为「48-char repeated span（滚动哈希任意偏移）」语义。

**成熟产品佐证（2026-08-21 调研）**：openclaw text-repetition-guard
（PR #39961）的 suffix cycle 策略=任意偏移重复检测（阈值建议 ≥40 字符
模式重复 ≥5 次），与滚动哈希同思路、量级一致；Pi loop-guard 流式
thinking 用「连续相似行 + 滑动窗口重复密度」两级升级（warn→abort）
避免误杀合法重复，与我们的 fail-fast 降级/终止阶梯等价，不引入新机制。

**2026-08-22 修订：灵敏层复读判定再校准（用户裁决：L=200 + 流内累计
3 次命中才中断+降级；content/reasoning 统一；先落设计；S1/S2 已实施
2026-08-22，见 §4.11）**

**缺口 A 实证（DNA 重跑，RUN-CLI-6a88905f，26m26s；审计留痕见 §4.10
缺口 A）**：reasoning 层 2 次触发（consecutive 1→2，降级梯
EnabledMax→EnabledLow→Disabled），触发内容确认均为**正常思考对任务
内容的重复引用**——① 重复 48 字符 span = DNA 序列等式
`"tgaggatcccgggaattctcgagtaag..." = "...gggttaa"`（偏移 47/97）；
② 重复 48 字符 span = 技术短语 "bases to the 3' side of the
recognition sequence"（偏移 16/97）。模型降级后仍完成全部工作
（177 工具轮、136 请求、`run_finished completed`）——**误杀坐实，
非病态复读**。

**定案（用户裁决）**：

- **L=48 → 200**（`REPETITION_MIN_RUN_CHARS`）；**W=96 → 400**（=2L）；
  缓冲 L+W=144 → **600**。窗口内任意周期 p ≤ 400 可命中；同字符连串
  在 3 次命中门槛下需 ≥2L+2=**402** 才触发（400 字符=1 次、401=2 次、
  402=3 次命中；旧表述「≥400 即触发」为单次命中语义下的等价条件）；
  DNA 低熵正常序列仍免疫（随机 DNA 200 字符重复 ≈ 4⁻²⁰⁰ 量级）。
- **触发门槛：流内累计命中 ≥3 次才中断 + 降级**——检测器在同一流内
  持续喂入，命中计数**不因中间未命中内容重置**（「间隔不重置」）；
  1–2 次命中仅审计留痕（WARN 输出触发 span + 窗口片段，缺口 A），不
  中断、不降级；第 3 次命中中断流并按族处置（reasoning → 降级一档 /
  content → 不重试透传）。命中计数随流结束丢弃（不跨请求累积）；
  会话级 consecutive 与 `DEGENERATION_LIMIT=3` 语义不变。**命中口径**：
  每次尾部 L-gram 与记录区匹配计 1 次命中（周期内容每字符位置可各计
  1 次——真循环在首命中后 2 字符内即达 3 次，快速触发）。
- content/reasoning 两族统一（同一 L、同一命中门槛）。
- 3-gram 路径②（≥1K token >60%）保留兜底；stall 兜底（600s/64K）
  与 fail-fast 纪律（会话级档位/计数单调/disabled 即终止/显式
  detail）不变。
- **判定语义更新**：流内出现 ≥3 次完全相同的 200 字符 span（任意
  偏移、起点距离 ≥200、间隔不重置）才触发。

**漏判边界（如实登记）**：短周期且总量小的循环（反复输出 <200 字符
内容仅 2–3 次）不再触发——该类循环输出量小、危害可控；真循环一旦
持续（累积出 200 字符相同块）仍触发。

**成熟产品佐证（追加）**：openclaw 阈值「≥40 字符模式重复 ≥5 次」与
本定案「≥200 字符 × 3 次命中」处于「更长 span、更少次数」象限，
量级一致；200 字符完全重复在正常思考中不可达，3 次命中进一步过滤
稀疏引用——组合即「仅循环特征」的简单形态，不引入周期/密度精细判定。

**2026-08-23 修订：复读判定「L=400 + 二级标点块内部重复确认」设计定稿
（用户裁决；G4 冒烟实证——代码引用型误杀仍存在；先落设计、未实施）**

**实证（第四轮冒烟 sweep-r1-g4-official，2026-08-22/23，官方方式
k=1、无 max_wallclock）**：sam-cell-seg reasoning 层 2 次触发
（consecutive 1→2、降级梯 EnabledMax→EnabledLow→Disabled），触发
span 实测 203/204 字符，均为模型推理中**完整引用读过的 Python 代码块**
（`_merge_collinear` / `_dedupe_consecutive` 段落；窗口尾部可见
"hmm no, let me look again" / "Hmm wait the grep earlier showed"
引用-再确认模式）；portfolio-optimization 3-gram 兜底触发 1 次
（ratio 0.60 边界）。降级后两题均完成（reward 1.0）——**误杀坐实**：
L=200/3 命中无法豁免「同流内完整重复引用 ≥200 字符内容 ≥2 遍」的
结构性模式（非概率性偶然——L 拉长仅推迟触发、门槛提高仅需多贴一遍）。
经查证非设计泄露（系统提示词/请求无哨兵参数，模型无规避行为、降级后
再次原样触发），根因=DeepSeek thinking 模式的引用-再确认循环习惯 +
检测器参数口径（触发 span 为 200+ 字符引用片段，200/3 为观察窗口与
门槛）。

**定案（用户裁决）**：

- **L=200 → 400**（`REPETITION_MIN_RUN_CHARS`）；**W=400 → 800**（=2L）；
  缓冲 600 → **1200**。命中门槛 3 不变；同字符连串触发线 402 → **802**
  （800 字符=1 次、801=2 次、802=3 次命中）。短引用（<400 字符，含
  exec 行截断 200 字符粒度）第一级即不命中。
- **新增二级「标点块内部重复确认」**（滚动哈希路径①命中后）：对被判定
  重复的一对大块（字符级相等）按**标点 + 空白（空格/换行）**切分为
  子块；要求**大块内部存在相同的标点块重复**——重复子块覆盖字符占比
  ≥ `REPETITION_PUNCT_BLOCK_MIN_RATIO=0.50`（初值，S2 用真实样本校准；
  切分符集合=ASCII 标点 + 中文标点 + 空白，`_` 除外保持标识符完整；覆盖
  占比分母=span 总字符数、含切分符；重复块全部出现次数的字符都计入覆盖。
  **边界（2026-08-23 审查处理登记）**：切分符集合为实现常用子集——全角
  变体（％＃＆〈〉〔〕〖〗等）与生僻中文标点未包含，含此类字符的重复块
  在该处不切分、仍按整块计入覆盖，S2 校准时可视需要扩集合）。
  **每一对命中都必须通过二级确认才计入该次命中**（任一不过仅审计留痕、
  不计数）；**切不出子块（无标点无空白单块）→ 二级不起作用 → 直接判真**。
- 语义升级：复读判定从「整块字符级相等」细化为「**重复内容由重复的
  块单元构成**」（结构性机械重复）——代码/文本引用（内部标点块多样）
  判非复读；`the the the` / `aaa, aaa, aaa`（内部标点块重复）判真重复；
  `aaaa`（无切分点）直接判真。
- 3-gram 路径②（≥1K token >60%）与 stall 兜底（600s/64K）**不变**
  （3-gram 不经二级确认；含标点整段循环仍由它兜底）。
- detail/审计语义不变：命中计数仍按族累计（content/reasoning 独立）、
  ≥3 才 trip、1–2 次仅审计留痕；二级不过的命中候选也留痕（span +
  标点块统计）供事后判定。**2026-08-23 审查处理（用户裁决）：拒绝候选
  审计按 delta 聚合为「一条摘要 + 命中计数」**——滑动窗口「每对」语义
  下同一底层重复内容在每个移位窗口各成一候选且 span 文本互异（1200
  字符流内可达 401 对），逐条留痕会刷屏；聚合后每个 delta 至多一条
  rejected 审计（代表 span + 标点块统计 + 本 delta 候选对数）。边界：
  同一 delta 内多个不同重复内容合并为一条（代表取最后一条候选；SSE
  delta 通常数百字符，多重复内容同 delta 罕见），登记为已接受。

**判定语义更新**：流内出现 ≥3 次「**400 字符完全相同的 span 且 span
内部由重复的标点块构成（或无切分点）**」（任意偏移、起点距离
∈ [400, 800]、间隔不重置）才触发。

**漏判/误判边界（如实登记）**：
- 短周期小量循环（<400 字符仅 2–3 次）不再触发（原 <200）——输出量小、
  危害可控；
- 含标点且内部标点块不重复的内容引用（正常代码/文本引用）不触发——
  本修订目标；
- 结构性重复内容（如 `1,2,3,1,2,3`、重复数据列表）内部标点块重复 →
  触发——与复读难分，登记为已接受边界；
- 无标点长引用（400+ 连续 DNA/数字串原样贴多遍）直接判真——罕见，
  概率可控（用户裁决）。

**验证方案**：
- 离线回放 G4 真实样本：sam-cell-seg 两段触发 span（203/204 字符）→
  L=400 第一级不命中（滚动路径静默）；构造 400+ 代码引用 → 第一级命中
  但二级不过（内部标点块重复占比 <50%）→ 不计数；构造 `"the "`×N
  （≥400 字符）→ 二级过 → 触发；构造 `"aaa, "`×N → 二级过 → 触发；
  构造无切分点 `"a"`×802 → 直接判真；
- S2 测试：上述 5 类 + 覆盖占比阈值边界（49%/51%）+ 每对命中二级确认
  （第一对不过、第二对过 → 累计 1 次）；
- S3 重建（Linux musl，ORZ-BUILD-MOUNT-001）→ S4 冒烟复验（G4/G5
  对照：sam-cell-seg 类代码引用零触发、构造真循环仍触发、零真实 400、
  命中率 ≥90%）。

**实施路由**：设计轮不动计数（仍 29）；S1 实施 + S2 测试闭合后待用户
放行重建与冒烟复验，闭环后登记 29 → 28。

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
| `REPETITION_MIN_RUN_CHARS` | 无（路径①=连续 5 相同 delta） | **48**（可调；滚动哈希 L-gram 长度） | transport.rs |
| `REPETITION_WINDOW_CHARS` | 无 | **96**（=2L，可调） | transport.rs |
| 2026-08-22 修订 | — | `REPETITION_MIN_RUN_CHARS` **48→200**、`REPETITION_WINDOW_CHARS` **96→400**（缓冲 144→600）、新增**流内累计命中门槛 3**（`REPETITION_HIT_LIMIT`；1–2 次命中仅留痕） | transport.rs |
| 2026-08-23 修订 | — | `REPETITION_MIN_RUN_CHARS` **200→400**、`REPETITION_WINDOW_CHARS` **400→800**（缓冲 600→1200）；新增二级「标点块内部重复确认」（`REPETITION_PUNCT_BLOCK_MIN_RATIO=0.50` 初值；切分符=ASCII+中文标点+空白、`_` 除外；无切分点直接判真；每对命中须过二级才计数；3-gram/stall 兜底不变） | transport.rs |
| 2026-08-23 NGRAM 校准修订 | — | `DEGENERATION_NGRAM_REPEAT_RATIO` **0.60→0.70**（`>` 严格大于保留）；新增 `NGRAM_HIT_LIMIT=3`（3-gram 路径②流内累计命中——每次 feed 超阈值计 1 次、1–2 次仅审计留痕、间隔不重置、流结束丢弃）；WARN 精度 `{:.2}`→`{:.3}` | transport.rs |
| `REPETITION_PUNCT_BLOCK_MIN_RATIO` | 无 | **0.50**（初值，S2 用真实样本校准；二级标点块重复覆盖占比下限——分母=span 总字符含切分符；无可切分点直接判真） | transport.rs |
| `DEGENERATION_CONSECUTIVE_DELTAS` | 5 | **路径①语义被取代**（滚动哈希任意偏移 48 字符；3-gram 路径②保留兜底） | transport.rs |
| `stream_idle_timeout` | 50s（STREAM-RETRY-RHYTHM 定值，未实施） | **30s**（校准 20–30s） | model.rs RetryPolicy |

> **2026-08-20 修订（用户裁决：方案 B + 中间档，见 §3.6）**：新增两行——
> `ThinkingMode` 默认档改 **high**（max 保留为可选档）；降级梯
> **high → low → disabled → 失败**（原 max 直跳 disabled 基线）。表中
> 其余参数不变。

| `ThinkingMode` 默认档 | `EnabledMax`（S1-S4 基线） | **`EnabledHigh`**（官方默认；`EnabledMax` 显式可选） | model.rs / transport.rs |
| 降级梯 | max 直跳 disabled（S1-S4 基线） | **high → low → disabled → 失败**（空响应与 reasoning 族哨兵共用） | transport.rs generate_stream |

### 3.6 默认档与降级梯修订（2026-08-20 用户裁决：方案 B + 中间档；已实施闭环）

**决策**：

- **默认 thinking 档 max → high**（官方 deepseek-harness 默认
  `reasoning_effort=high`，官方工作点即 256K+high）；`EnabledMax` 保留为
  显式可选档（难题专用，仍受哨兵保护）。
- **降级梯插入 low 中间档**：**high → low → disabled → 失败**（空响应
  快速重试与 reasoning 族哨兵跳转共用；「middle」映射为 DeepSeek
  `reasoning_effort=low`，官方四档 off/low/high/max 中的中间档）。

**依据（2026-08-20 用户讨论 + S4 实测）**：

- S4（256K+max）实测：完成型空流 0、命中率 95.28%、复读灵敏层拦截 1/85
  并降级收尾、stall 兜底零误杀——**机制已稳，max 不再是必要工作点**；
  官方默认 high，常态延迟/成本更低（S4 reasoning 占 output 约 89%，
  high 可明显削减）。
- 思考禁用本身质量影响大（模型退化为「快答模式」）；low 保留浅思考链，
  多数病态可被低深度救回，比直跳禁用更温和。
- 代价：失败路径多一轮完整思考（每级受 64K/600s 兜底保护；空响应完成在
  兜底下不出现）。病态率低（S4 1/85）且复读在数 K 内被抓，额外成本可
  接受；最坏情形=2 个病态轮 + 1 秒降级（原 1 个病态轮 + 1 秒降级）。

**变更**：

- `ThinkingMode` 增加 `EnabledLow`（映射 `reasoning_effort: "low"`）；
  默认改 `EnabledHigh`（映射 `reasoning_effort: "high"`）；`EnabledMax`
  保留。`Disabled` 不变（最终降级档 + 测试/基准路径）。
- D-6 空流链（§3.2 修订）：阶段 1 正常（**high** + 256K）→ 完成型空响应
  快速有界重试 ≤2 次（high，500ms→10s+10% jitter）→ 仍空 → **low** →
  仍空 → **disabled** → 仍空 → 失败。
- 哨兵跳转（§3.3 修订）：reasoning 族中断（复读/stall）不原样重试，
  **逐级 high → low → disabled**；content 族仍不重试透传；
  `DEGENERATION_LIMIT=3` 三族共享不变；detail 前缀/分类器不变。
- 兜底与重试节奏全部不变：stall 600s/64K（与 max_tokens 解耦）、idle
  30s、`EMPTY_RESPONSE_MAX_RETRIES=2`、退避 500ms→10s+10% jitter、
  `REASONING_CHARS_PER_TOKEN=2`。
- 请求头指纹含 thinking 档 → 部署后首次请求一次性指纹变化（既有纪律）。

**实施路由**：S1 代码（`ThinkingMode` 三档 + 默认 high + 三级梯接线 +
  注释/fingerprint 同步）→ S2 测试（high/low 请求头断言、三级梯路径、
  回归全绿）→ S3 重建 → S4 复验（难题单题 + high vs max 成本/产出对照）。
**计数：设计轮不动（28）**；实施放行 28 → 29，验证闭环 29 → 28。

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

### 4.4 修订实施路由（2026-08-20 用户裁决：默认 high + 三级降级梯；
S1/S2 已完成——见 §4.5；S3/S4 待续）

- **S1 代码**：`ThinkingMode` 增 `EnabledLow`（`reasoning_effort=low`）、
  默认改 `EnabledHigh`（`reasoning_effort=high`）；`build_request` 映射
  high/low；`generate_stream` 降级梯接线 **high → low → disabled → 失败**
  （空响应快速重试与 reasoning 族哨兵跳转共用）；注释/fingerprint 同步。
- **S2 测试**：high/low 请求头断言（默认 high、low 档映射，原 max 默认
  断言改 high）；三级梯路径（空响应 high→low→disabled→失败、哨兵
  high→low、low 级哨兵→disabled）；既有测试核对；回归全绿（fmt/clippy
  基线）。
- **S3 重建**：Linux musl（ORZ-BUILD-MOUNT-001 契约）。
- **S4 复验**：难题单题 + **high vs max 成本/产出对照**（output tokens、
  命中率、空流率、stall 触发、首轮延迟）；判定=空流 0、零 400、命中率
  ≥90%、无 stall 误杀；计数 29 → 28。

### 4.5 S1 代码 + S2 测试实施登记（2026-08-20，用户指示进行 S1 与 S2；
orz b72a0a4 已推送）

- **S1 代码**：
  - `ThinkingMode` 增 `EnabledLow`（`reasoning_effort=low`）；默认档
    `EnabledMax` → **`EnabledHigh`**（官方默认档）；`EnabledMax` 保留
    显式可选档（难题专用）。`Disabled` 不变（最终降级档）。
  - 新增 `apply_thinking`：thinking 块 + `reasoning_effort` 双旋钮统一
    按档位覆盖（`create_once` / `stream_once` 共用）——修复「high 配置
    降级到 low 时 effort 仍为 high」的隐患（原实现只按 config 映射
    effort、仅 Disabled 才清空）。
  - `build_request` 映射补 high/low（thinking enabled + 对应 effort）。
  - `generate_stream` 降级梯接线 **high → low → disabled → 失败**：
    空响应每档快速有界重试 ≤2 次（换档重置计数与退避——每档独立
    「快速 500ms→10s+10% jitter」节奏，设计 §3.2 按阶段重试语义）；
    reasoning 族哨兵逐级下降一档。**`EnabledMax` 显式档保留 S4 验证
    基线**：哨兵命中/空流链耗尽直跳 disabled（三级梯按默认 high 起定义；
    max 不额外多烧 high/low 两轮——避免超出已裁决的「失败路径多一轮」
    成本）。
  - 请求头指纹 thinking 映射含 high/low（部署后首次请求一次性变化）；
    live 探针注释同步（探针为 max 显式档实测路径）。
- **S2 测试**：
  - 请求头断言：默认 high（原 `build_request_sets_thinking_enabled_max_d6`
    改为默认档 high）；max/low 显式档映射断言（新增）。
  - 三级梯 e2e 3 项：空响应 high→low→disabled→显式失败（3+3+1=7 次
    请求、每档 0.5s→1s 退避重置）；哨兵 high→low（low 档产出）；low 级
    哨兵→disabled（disabled 档产出，无 reasoning 旋钮）。
  - 既有 max 基线测试核对：max 直跳 disabled 语义不变（S4 基线保留）。
  - 回归：orz-loop lib **531 通过 / 0 失败 / 3 ignored**（+4 项）；fmt
    干净；clippy 与基线一致（lib 21 / test 28 均既有位置，transport.rs
    零告警）；`cargo check --workspace` 通过（orz-bin/orz-tui 等下游
    无破坏）。
- 计数：S1 实施放行入账 1 项（**28 → 29**）；S2 不改变未闭合计数（仍
  29）；S3/S4 验证闭环后 29 → 28。
- 待续：S3 重建（Linux musl，ORZ-BUILD-MOUNT-001 契约）→ S4 复验（难题
  单题 + high vs max 成本/产出对照）。

### 4.6 S1 全面审查处理登记（2026-08-20，用户指示处理审查全部问题；
orz 1651f59，已提交、未推送）

审查结论（用户指示对本轮 S1 代码全面检查）：**未发现功能缺陷，设计合理、
实现合理、设计与实现符合**；提出 5 项观察级建议，处理如下：

- **O1（已知边界登记）**：非流式 `generate` 链保持既有「原样重试 1 次 →
  thinking 禁用」基线，不引入 low 档——本设计三级梯作用域明确为
  `generate_stream`（§3.2/§3.6）；`generate` 仅服务 preflight/gate
  （max_tokens=1024 快轮），低风险路径。登记为**有意不对称（已知边界）**，
  非遗漏。
- **O2（代码处理）**：`build_request` 与 `apply_thinking` 双份 thinking
  映射——`build_request` 直调仅测试场景，生产路径均再经 `apply_thinking`
  按梯级/覆盖档覆盖双旋钮；已补同步注释（改档位两处须同时更新，P3 维护性）。
- **O3（参数登记）**：`empty_response_backoff` 的 `max_elapsed_time=60s`
  为设计参数表外实现细节——每档 ≤2 次重试（500ms→1s）合计约 1.5s，60s
  纯兜底不可达；注释已说明，参数表不加项。
- **O4（测试补强）**：新增 `config_fingerprint_reflects_thinking_tier`
  ——默认档（EnabledHigh）指纹与显式 EnabledHigh 一致，且与
  EnabledLow / EnabledMax / Disabled 各档互异（设计 §3.6「请求头指纹含
  thinking 档 → 部署后首次请求一次性变化」补断言）；orz-loop lib
  **532 通过 / 0 失败 / 3 ignored**（+1 项）、fmt 干净、clippy 与基线
  一致（lib 21 均既有位置，transport.rs 零告警）。
- **O5（已接受登记）**：三级梯 e2e 请求体子串匹配（`"\"high\""` 等）登记
  为已接受边界——mock 请求体可控、当前断言集合无实际误匹配风险。
- 计数：审查处理不改变未闭合计数（仍 29），S3/S4 闭环后 29 → 28。

### 4.7 S3 重建 + S4 复验登记（2026-08-20，用户指示推送后重建、换题复验；
orz 1651f59 已推送；S3/S4 验证闭环 29 → 28）

- **S3 重建**：Linux musl（ORZ-BUILD-MOUNT-001 契约，`build_orz_aliyun.sh`，
  阿里云镜像源；输出 `D:/tb-eval/orz-linux`）；三件套时间戳更新 12:08
  （orz 104,375,344 B / orz-signer 1,388,544 B / orz-acaf-provision
  1,206,536 B）；构建日志 `build-20260820.log`。
- **S4 复验（换题 make-doom-for-mips，harbor job `2026-08-20__20-08-50`，
  trial `make-doom-for-mips__rTheTXd`，RUN-CLI-6a86ee6c，
  12:09:16→12:38:16，wallclock 1740s 跑满、reward 0.0、零异常）**：
  - **完成型空流：0**（194 个 model_output 中 text/tool_calls 双空 = 0；
    orz.txt 零「completed empty response」重试警告）——32K 截断空流链
    根因在换题后同样不出现。
  - **零 HTTP 400、零 idle 死线、零 timeout**（events 中 126 处 "400"
    均为字段数值：时间戳/effective_head_limit，非 HTTP 状态）。
  - **命中率 journal 口径 92.18%**（hit 8,122,240 / miss 689,249，
    194 请求，DoD ≥90% 达成）。
  - **哨兵/stall 观测**：`reasoning_repetition` / `reasoning_stall`
    / idle 死线全部 0 触发——本次无病态轮、无需降级；**stall 兜底零误杀**
    （600s/64K 初值继续维持不调）。
  - **首输出延迟 5.5s**（对照 max 基线 gpt2-codegolf 首请求约 66s——
    high 档思考更短，首轮即出 blackboard_read 计划轮）。
  - **用量/成本（journal 口径）**：194 请求；output tokens 169,182
    （含 reasoning 130,689，占 77%，对照 max 基线 89%）；prompt
    8,811,489（hit 8,122,240 / miss 689,249）；单请求最大 completion
    7,082（无 >100K 异常、无预算放大）；按 ¥4.592/M 估算输出成本
    ≈ **¥0.78**（对照 max 基线 ¥0.88，同口径下降约 12%）。
  - **high vs max 跨题参照**（口径注意：任务不同——make-doom 源码探索
    密集型 vs gpt2-codegolf 写作型，非严格同题对照）：high 档在相同
    30 分钟预算内跑出 194 请求 / 333 工具轮（max 基线 85 请求），
    每轮更快（首输出 5.5s vs 66s）、reasoning 占比更低（77% vs 89%）、
    输出成本更低（¥0.78 vs ¥0.88）；命中率 92.18%（≥90% 达标，低于
    max 基线 95.28%，属跨题差异，非档位回归）。input 侧跨题不可直接
    对照（本轮 prompt 8.8M vs 基线 3.3M，来源=工具轮次多 2.3 倍）。
- **校准结论**：600s/64K/30s 初值维持不调（零漏判零误杀）；默认 high
  档本轮全链路无退化、无空流、无 400，机制稳定。
- 计数：**S3/S4 验证闭环 29 → 28**。

### 4.8 修订实施路由（2026-08-21 用户裁决：滚动哈希任意偏移复读检测；
设计轮、未实施）

- **S1 代码**（transport.rs，`DegenerationDetector`）：`feed_repetition`
  路径①（连续相同 delta N=5）替换为滑动窗口滚动哈希任意偏移检测——新增
  最近 96 字符缓冲 + 48 字符 L-gram 哈希集（滚动哈希 O(1)/字符，哈希命中
  后字符级比对防碰撞）；新常量 `REPETITION_MIN_RUN_CHARS=48` /
  `REPETITION_WINDOW_CHARS=96`；`DEGENERATION_CONSECUTIVE_DELTAS` 路径①
  语义退役；3-gram 路径②（≥1K token 窗口 >60%）保留兜底；content/
  reasoning 两族共用核心（feed 签名不变）；detail 前缀不变、触发描述更新。
- **S2 测试**：更新既有断言——`degeneration_detector_trips_on_five_
  identical_deltas`、`detector_reasoning_repetition_trips_on_five_
  identical_deltas`、`degenerate_sse_body` 相关用例（5×"same"/"hi"/
  "think" 不再触发，改 96+ 字符重复串）；新增——短低熵块不触发（5×"a"、
  DNA 样本 6439 字符）、周期 10 短语循环触发（对齐缺陷回归）、poly-A
  ≥96 连续相同字符触发、单一大 chunk 不触发、哈希碰撞字符级比对确认；
  回归全绿（fmt/clippy 基线）。
- **S3 重建**：Linux musl（ORZ-BUILD-MOUNT-001 契约；与 fail-fast/
  解码兜底/180s 窗口批次合并一次到位）。
- **S4 复验**：dna-assembly 重跑——低熵误杀消除（正常 DNA 分析不再被
  判复读）、真复读仍能触发、fail-fast 终止语义不变、零 400、命中率
  ≥90%。
- 计数：设计轮不动（28）；S1 实施放行入账 **28 → 29**，S3/S4 验证闭环
  后 **29 → 28**。

### 4.9 S1 代码实施登记（2026-08-21 用户放行实施；orz 工作树未提交）

- `transport.rs` `DegenerationDetector` 复读判定路径①改造：
  - 退役 `DEGENERATION_CONSECUTIVE_DELTAS`（连续相同 delta N=5）与
    `recent_content_deltas` / `recent_reasoning_deltas` 字段；
  - 新增 `RollingRepetitionWindow`（滑动窗口滚动哈希任意偏移）：最近
    L+W=144 字符缓冲 + 记录区（起点偏移 ∈ [48, 96]）48 字符 L-gram 哈希
    集；新尾部 L-gram 哈希命中后以字符级比对防碰撞；新常量
    `REPETITION_MIN_RUN_CHARS=48` / `REPETITION_WINDOW_CHARS=96`（=2L）/
    `REPETITION_BUFFER_CHARS=144`（=L+W）/ 哈希基数 B=1_000_003；
  - `feed_repetition` 路径①改为字符流级判定（与 delta 切块粒度无关）：
    流中出现两个起点距离 ≥48 的相同 48 字符 span 即触发；路径②（累计
    ≥1K token 且最近 1K token 内 3-gram 重复率 >60%）保留兜底；
    content/reasoning 两族共用（feed 签名不变）；
  - detail 前缀不变（`content_repetition` / `reasoning_repetition`），
    触发描述更新为「48-char repeated span in the recent 96-char window
    (rolling hash, arbitrary offset)」。
- 既有断言同步（S1 部分）：5×"same"/"think"/"hi" 不再触发——
  `degeneration_detector_trips_on_five_identical_deltas` →
  `degeneration_detector_trips_on_repeated_48_char_span`（24×"same" =
  96 字符重复 span）、`detector_reasoning_repetition_trips_on_five_
  identical_deltas` → `detector_reasoning_repetition_trips_on_repeated_
  48_char_span`；e2e 五处（content 中断不重试、reasoning 复读直跳降级、
  三级梯 high→low / low→disabled、空响应重试中哨兵）改单个 96 字符重复
  span；3-gram 路径测试内容重构（共享核心 39 字符 < L + 互异长尾部——
  滚动路径保持静默、只测 3-gram 兜底路径）。
- 验证：orz-loop lib **550 通过 / 0 失败 / 3 ignored**；`cargo fmt
  --check` 干净；clippy 无新增告警（transport.rs 新代码零告警，仅 2 条
  既有 doc 告警）；`cargo check --workspace` 通过（下游无破坏）。
- 计数：实施放行入账 1 项（**28 → 29**），S2 不改变计数，S3/S4 验证
  闭环后 **29 → 28**。
- 待续：S2 测试（短低熵块不触发 / 周期 10 短语循环触发（对齐缺陷回归）/
  poly-A ≥96 触发与 <96 不触发 / 单一大 chunk 不触发 / 哈希碰撞比对）
  → S3 重建 → S4 复验（dna-assembly 低熵误杀消除 + 真复读仍触发）。

### 4.10 S2 测试实施登记（2026-08-21 用户指示进行 S2；orz 工作树未提交）

- 新增 8 项测试（orz-loop lib 550 → **558 通过 / 0 失败 / 3 ignored**）：
  - 短低熵块不触发——`degeneration_detector_short_low_entropy_deltas_
    do_not_trip`（5×"a" / 5×"same"）；
  - DNA 低熵样本不触发——`degeneration_detector_dna_low_entropy_output_
    does_not_trip`（6439 字符 ACGT + 散点 ttttt/aaaaa/ggggg/
    N N N N N/GGTCTC 短特征，确定性伪随机；与误杀样本同量级）；
  - poly-A 精确阈值——`degeneration_detector_poly_a_threshold`
    （95 不触发 / 96 触发）；
  - 周期 10 短语循环触发——`degeneration_detector_periodic_phrase_trips_
    any_phase`（对齐缺陷回归：固定偏移系统性漏检、任意偏移经距离 50
    同相命中）+ `detector_reasoning_repetition_periodic_cycle_trips`
    （灵敏层同算法）；
  - 单一大 chunk 不触发——`degeneration_detector_single_large_distinct_
    chunk_does_not_trip`（2000 字符宽字母表互异内容）；
  - 哈希碰撞字符级比对——`rolling_window_spans_equal_char_level_
    verification` 直接验证 `spans_equal`（相同判等 / 单字符差异判不等）；
    真实 u64 多项式碰撞构造不可行（B=1_000_003 奇数、48 位置、字符字母
    表，2-adic 差异上界远小于 2^64），**登记为已接受边界**；
  - 近重复不误杀补充——`degeneration_detector_near_repeat_does_not_trip_
    then_exact_repeat_trips`（48 字符 span 单字符差异不触发、随后精确
    复读触发）。
- 回归：fmt 干净、clippy 无新增告警（transport.rs 仅 2 条既有 doc
  告警）、`cargo check --workspace` 通过。
- 计数：S2 不改变计数（仍 29），S3/S4 验证闭环后 29 → 28。
- 待续：S3 重建（Linux musl；与 fail-fast/解码兜底/180s 窗口批次合并）
  → S4 复验（dna-assembly 低熵误杀消除 + 真复读仍触发）。

### 4.11 S3 重建 + S4 复验 + 停滞守卫退役 + 缺口 A + 灵敏层再校准设计
登记（2026-08-22）

**S3 重建**：Linux musl（ORZ-BUILD-MOUNT-001 契约，`build_orz_aliyun.sh`，
两轮：cc77efe 基线轮 + 缺口 A 留痕轮，三件套时间戳更新）。

**S4 复验（dna-assembly 单题，job `dna-assembly-s4`，RUN-CLI-6a887a1a，
15m6s、118 请求、reward 0.0 无异常）**：命中率 98.2%（hit 4,379,264 /
miss 80,340）≥90%；真实 HTTP 400 为 0（journal 中 10 处 "400" 均为
订单号/UUID 子串）；content 层滚动哈希零触发（旧「5 identical deltas」
误杀源消除）。**发现缺口 A**（reasoning 层滚动哈希触发 2 次，run 早期
36s/96s，consecutive 1→2、降级梯 EnabledMax→EnabledLow→Disabled，但
触发内容无日志留痕，无法判定真循环还是误杀）；**发现缺口 B**（会话级
停滞守卫 `STAGNATION-NGRAM-REPEAT` 在 run 结束时以 max_ngram_repeat=40
判 `restart_requested`——跨历史 run 核对几乎所有长会话均触发
（12–341，全部 run_invalidated restart_requested），为普遍误杀）。

**停滞守卫退役（用户裁决：一并全部退役，含失败轮次审计）**：生成期
检测（滚动哈希 + stall 兜底 + fail-fast）覆盖实际退化面；复读主要显
现在思维链，reasoning 灵敏层已接管；0d 记录停滞守卫「未拦截」make-doom
真实退化；S4 实证其所有触发均为正常长会话误杀。退役范围=主/子代理
链路 `evaluate_stagnation` 终止判定、失败轮次审计（agent_loop.rs）、
`stagnation.rs` 模块、`RuntimeStagnationGuard` 事件（v0.2 面）、TUI
投影、Python reference/verifier/schema/doctor 清单；v0.1 冻结面保留
（历史 replay）；`tokenize` 迁移至 transport（3-gram 兜底依赖）。
验证：orz-loop 557 通过 / 0 失败 / 3 ignored、fmt 干净、clippy 无新增、
verifier 230 通过、assurance 1588 通过、doctor 仅剩 orz 未提交 dirty。

**缺口 A（审计留痕）实施 + 现场定性**：transport 退化触发时 WARN 输出
触发上下文（重复 48 字符 span + 两匹配偏移 + 窗口尾部）——新增
`RollingRepetitionWindow::last_match`/`match_context` 与
`DegenerationDetector::trigger_context`，单测
`degeneration_trigger_context_records_repeated_span`（orz-loop
556→557）。**DNA 重跑（job `dna-assembly-s4b`，RUN-CLI-6a88905f，
26m26s、136 请求、reward 0.0 无异常）**：命中率 98.64%（hit
6,954,240 / miss 96,065）；reasoning 层 2 次触发均留痕——① 重复
48 字符 span = DNA 序列等式
`"tgaggatcccgggaattctcgagtaag..." = "...gggttaa"`（偏移 47/97）；
② 重复 48 字符 span = 技术短语 "bases to the 3' side of the
recognition sequence"（偏移 16/97）——**均为正常思考对任务内容的
重复引用，误杀坐实非病态复读**；降级后模型继续完成全部工作（177
工具轮、`run_finished completed`——停滞守卫退役后不再
`run_invalidated restart_requested`）。

**灵敏层再校准设计定稿（用户裁决：L=200 + 流内累计 3 次命中才中断+
降级；content/reasoning 统一；先落设计；S1/S2 已实施 2026-08-22）**：
详 §3.3 修订——
`REPETITION_MIN_RUN_CHARS` 48→200、`REPETITION_WINDOW_CHARS` 96→400
（缓冲 144→600）；触发门槛改流内累计命中 ≥3 次（间隔不重置；1–2 次
仅留痕；计数随流结束丢弃；会话级 consecutive 与 `DEGENERATION_LIMIT`
不变）；判定语义=流内 ≥3 次完全相同的 200 字符 span（任意偏移、起点
距离 ≥200）。漏判边界=短周期小量循环（<200 字符仅 2–3 次）不再触发。

**2026-08-22 S1 代码实施 + S2 测试实施登记（用户指示进行 S1 与 S2；
orz 工作树未提交；计数不变仍 29）**：

- **S1（transport.rs）**：`REPETITION_MIN_RUN_CHARS` 48→**200**、
  `REPETITION_WINDOW_CHARS` 96→**400**（缓冲 144→600）；新增
  `REPETITION_HIT_LIMIT=3`（流内累计命中门槛，间隔不重置）。滚动窗口
  **命中后继续喂入**（不早停）——`feed_chars` 返回 delta 内新命中数；
  记录区结构性保证起点距离 ∈ [200, 400]。`DegenerationDetector` 新增
  每族流内命中计数（content/reasoning 独立）+ 非触发命中审计留痕
  `audit_hits`（1–2 次命中逐条登记触发上下文）；`feed_repetition`
  单族状态聚合为 `RepetitionFamilyState`（滚动窗口 + 3-gram 窗口 +
  命中计数），自由函数保持 7 参数（clippy 阈值内）。触发门槛：累计
  ≥3 次命中才置 trip（detail 带 `{hits}/{REPETITION_HIT_LIMIT}`）；
  1–2 次命中仅留痕不中断——流循环 `take_audit_hits` 逐条 WARN
  （`output-health guard repetition hit (audit only, not tripping)`）。
  3-gram 路径②（≥1K token >60%）与 stall 兜底（600s/64K）不变。
- **S2（测试）**：poly-A 阈值 399 不触发 / 400=1 次命中（audit only）/
  401=2 次命中（audit only）/ 402=3 次命中触发；周期 10 短语循环
  `repeat(41)`=410 字符 3 次命中触发（对齐缺陷回归）；近重复（单字符
  差异）不计数、精确复读 1 次命中仅审计、第 3 次触发；新增
  `degeneration_detector_trips_after_three_stream_hits`（1–2 次仅审计、
  第 3 次触发）与 `degeneration_detector_hit_counter_survives_gaps`
  （间隔不重置：命中 1 与命中 2/3 之间插入 50 个互异字符仍累计）；
  缺口 A 单测更新为 200 字符 span；3-gram 兜底用例重构（每 feed 追加
  唯一 4 位标记，杜绝 200 字符 span 复现——滚动路径保持静默，仅测
  3-gram 路径）；e2e 五处（content 中断不重试、reasoning 复读直跳
  降级、三级梯 high→low / low→disabled、空响应重试中哨兵）改 402 同
  字符；新增 e2e `generate_stream_reasoning_subthreshold_hits_do_not_
  interrupt`（401 同字符=2 次命中，流正常完成、单连接、不降级）。
- 验证：orz-loop lib **560 通过 / 0 失败 / 3 ignored**；`cargo fmt
  --check` 干净；clippy 无新增告警（transport.rs 仅 2 条既有 doc 告警）；
  `cargo check --workspace` 通过（下游无破坏）。
- **2026-08-22 S1 全面审查处理（审查发现 4 项全部处理；实施不改变
  计数，仍 29）**：① 文档——§3.3 信号表旧参数（48/96）同步为再校准值
  （L=200/W=400/命中门槛 3）并补脚注，头部状态块补 2026-08-22 修订
  条目；② 实现——新增 `feed_chars_capped`（以「剩余门槛=门槛-已累计」
  封顶单次喂入：同一 delta 内累计命中达 `REPETITION_HIT_LIMIT` 后该流
  必触发，停止消费超大退化帧的滚动哈希/字符比对；子门槛 delta 全量
  消费、窗口状态与旧语义一致；`feed_chars` 保留为测试便捷方法，lib
  目标下豁免 dead_code）；③ 实现——审计 WARN 与触发判定同 chunk 聚合
  （先取走审计条目、再做 stall/触发判定；触发时审计条目并入 trip WARN
  的 trigger_context，不再单独输出「audit only, not tripping」误导
  文案）；④ 实现——3-gram/stall 触发时清空 `trigger_context`（两路径
  均无「重复 span」语义，防止残留旧命中上下文误标为本次触发）。验证：
  orz-loop **560 通过 / 0 失败 / 3 ignored**、`cargo fmt --check` 干净、
  clippy transport.rs 仍仅 2 条既有 doc 告警（与基线一致）。
- 计数：S1/S2 不改变未闭合计数（仍 29）；S3 重建 + S4 复验（DNA
  reasoning 正常引用不误杀、真循环仍触发）验证闭环后 29 → 28。
- 待续：S3 重建（Linux musl，ORZ-BUILD-MOUNT-001）→ S4 复验。

**2026-08-23 复读判定二级再校准设计定稿登记（用户裁决：L=400 + 二级
标点块内部重复确认；G4 冒烟实证——代码引用型误杀仍存在；先落设计；
2026-08-23 **S1 实施**（transport.rs 常量 L=400/W=800 + 二级确认逻辑，
含单元测试与既有阈值测试适配），S2 待放行；设计轮不动计数，仍 29）**：
G4 官方方式冒烟（sweep-r1-g4-
official，5/5 reward 1.0、零真实 400、命中率 93.66–98.48%）实证
sam-cell-seg reasoning 层 2 次触发均为**代码引用误杀**（203/204 字符
span，模型推理中完整引用读过的代码块 ≥2 遍）；经查证非设计泄露。
**定案**：`REPETITION_MIN_RUN_CHARS` 200→**400**、`REPETITION_WINDOW_
CHARS` 400→**800**（缓冲 1200）；新增二级确认——命中后按标点+空白
切块，大块内部标点块重复覆盖占比 ≥50%（初值）才计命中（每对都过）、
无切分点直接判真；3-gram/stall 兜底与命中门槛 3 不变。判定语义=流内
≥3 次「400 字符完全相同且内部由重复标点块构成（或无切分点）」的 span
才触发。漏判边界=短循环（<400 字符 2–3 次）、结构性重复内容、无标点
长引用（概率可控）。验证方案=G4 样本离线回放 + 构造样本 + S3 重建 +
S4 冒烟复验。详 §3.3 修订 / §3.5 参数表；登记于 ADR-0010 §14.35
第 18 项 / BACKLOG 0d / TODO P0-0d 后续 6 / CLI_PROJECT_INDEX。

**2026-08-23 S1 全面审查处理（审查发现 5 项全部处理；实施不改变计数，
仍 29）**：① 文档——§3.3 信号表两行仍为 L=200/W=400 旧参数，同步为
L=400/W=800 + 二级确认并补 2026-08-23 脚注；② 实现——拒绝候选审计
按 delta 聚合为「一条摘要 + 命中计数」（用户裁决：按重复 span 内容去重
聚合；实现探明滑动窗口每对语义下同一直复内容的移位 span 文本互异、按
精确 span/重复块签名均无法稳定折叠，改为每 delta 一条代表 span + 标点
块统计 + 候选对数；同 delta 多重复内容合并为一条已登记边界）；③ 实现
——触发 chunk 内已取走的审计条目（二级不过候选 + 子门槛确认命中）此前
被 trip 分支提前 return 丢弃，改为 trip 分支同样逐条 WARN（文案
「same chunk as trip」，不复用 not-tripping 误导措辞）；④ 边界——切分符
集合为实现常用子集（全角变体与生僻中文标点未含），常量注释 + §3.3 登记；
⑤ 测试——补三项（覆盖占比 49.6%/50.0%/50.8% 阈值边界、每对确认累计
（第一对不过第二对过 → 累计 1）、G4 短引用形态等价离线回放（203 字符
代码引用 ×3 静默）），既有聚合语义测试适配。验证：orz-loop lib **565
通过 / 0 失败 / 3 ignored**、`cargo fmt --check` 干净、clippy 无新增
（transport.rs 仍仅 2 条既有 doc 告警）。

**2026-08-23 S2 测试实施登记（用户放行 S2；正式 S2 轮=离线回放记录 +
结果归档；实施不改变计数，仍 29）**：新增字节级真实回放测试
`repetition_second_stage_g4_real_span_replay_stays_silent`——源证据=
`D:\tb-eval\jobs-sweep\sweep-r1-g4-official\sam-cell-seg__CT5JrD3
\agent\orz.txt` WARN 记录（2026-08-21T21:35:29 / 21:36:00）：旧 L=200
检测器两次触发的真实匹配 span 各 **200 字符**（检测器打印口径；设计早前
203/204 为完整重复引用区域口径），以 reasoning 族 feed 按「引用-再确认
循环」形态原样引用 3 遍（间隔为与观察一致的互异再确认文本）回放——首级
L=400 均不命中：**无候选、无审计、无触发**（G4 代码引用误杀消除的字节级
离线证据；此前 S1 审查处理的三项测试为构造/形态等价覆盖，本次补真实字节）。
构造样本正/负对照由 S1 审查处理已覆盖（代码引用不计数、`aaa, aaa` 触发、
无切分点直接判真、覆盖占比 49.6%/50.0%/50.8% 阈值边界、每对确认累计、
G4 短引用形态回放）。验证：orz-loop lib **566 通过 / 0 失败 / 3 ignored**
（565 → 566，新增 1 项）、`cargo fmt --check` 干净、clippy 无新增
（transport.rs 仍仅 2 条既有 doc 告警）。

**2026-08-23 S3 重建登记（用户放行；Linux musl，ORZ-BUILD-MOUNT-001
契约，`build_orz_aliyun.sh`；计数不变仍 29）**：容器增量构建
（`rust:1.97-slim`；挂载 `D:\CLI:/orz`、工作目录 `/orz/orz`；apt 阿里云
镜像 + 官方 static.rust-lang.org + 静态 rg 15.0.0 源码安装；`-j 1`）
**BUILD_EXIT=0**；三件套产物时间戳 **2026-08-23 02:01**（orz 104,521,992
B / orz-signer 1,388,592 B / orz-acaf-provision 1,206,568 B；SHA256=orz
E327B840A867B5AFEA69EA141841072006D13B604B37743CBB7DFF95C1F6CE27 /
signer 229AFF0D88BAC89C6F873BB1A015AB0702D45FBDB15910BD01C5721F0DE16D95 /
provision 7D102976444891EBB20A7B4D547455C5BE7DD8E46EF62EEA687D0495A8CB80EB）；
最小可执行冒烟=三件均正常加载执行（orz 无 TTY 报 TUI io error 属预期——
headless 真机面由 S4 任务容器验证；provision 打印 usage；signer 报 manifest
缺失）。对应源码=orz 6178050（S2 回放测试入库）+ 父仓库 dfe39a1。待续：
S4 冒烟复验（G4/G5 对照：sam-cell-seg 类代码引用零触发、构造真循环仍
触发、零真实 400、命中率 ≥90%）。

**2026-08-23 S4 冒烟复验登记（用户放行；G4/G5 对照，sweep-s4-g4 /
sweep-s4-g5，k=1、官方方式（无 max_wallclock）、n-concurrent=1；**S3/S4
验证闭环 29 → 28**）**：新二进制（orz 6178050 构建轮）实机运行——

- **G4（git-multibranch / sam-cell-seg / portfolio-optimization /
  video-processing / mcmc-sampling-stan，1h48m，4/5 reward 1.0）**：
  **sam-cell-seg 零复读触发**（对照旧 L=200 二进制 G4 官方轮同题 2 次
  reasoning_repetition 触发 + 2 次降级——误杀消除，reward 仍 1.0）；
  portfolio-optimization 零触发（旧轮 1 次 3-gram 0.60 边界触发，本轮
  无）；**零真实 400**；命中率 96.30%–98.34% 全 ≥90%。
- **G5（path-tracing-reverse / mteb-retrieve / code-from-image /
  break-filter-js-from-html / sanitize-git-repo，1h02m，2/5 reward 1.0）**：
  零触发、零 400；命中率 96.54%–97.07%（有 journal 4/5；
  path-tracing-reverse AgentTimeoutError=官方超时，机制无异常、无 journal
  拷出）。
- **构造真循环仍触发（判定层）**：S2 测试套件覆盖——`aaa, ` 周期 ×3
  触发、802 同字符无切分点直接判真、周期 10 短语循环触发、e2e 退化流
  中断+降级，566 项全绿；3-gram 兜底实机在线（见观察项）。
- **观察项（登记，不改本次范围）**：video-processing 3-gram 路径②触发
  1 次（ratio 0.60 边界、consecutive=1 → 降级 EnabledLow 一次，任务继续
  至 135 请求、reward 0）——与旧轮 portfolio-optimization 同界（0.60）；
  3-gram 兜底不经过二级确认、设计保留为含标点整段循环兜底，本轮按设计
  接受，登记为后续可能的 3-gram 边界校准观察项。

- 计数：**S3/S4 验证闭环 29 → 28**（0d 计数 29 的 S3/S4 闭环此前因缺口 A
  推迟，缺口 A 定性 + 灵敏层再校准 + 二级再校准全部闭环后登记）。

## 5. 验收标准（DoD）

- S2 全绿、fmt 干净、clippy 与基线一致；
- S3 重建成功、三件套时间戳更新；
- S4：无完成型空流链（或 stall 在预算内提前中断并降级收尾）、零 400、命中率
  ≥90%、账单对账无异常尖峰（单请求废弃 output ≤ 预算内）、stall 双信号与
  idle 30s 按实测校准一次（误杀与漏判平衡）；
- 计数：实施放行时入账 1 项（28 → 29），验证闭环后 29 → 28。
- 修订轮（2026-08-20：默认 high + 三级梯）：S2 含 high/low 请求头断言与
  三级梯路径（high→low→disabled→失败）全绿；S4 复验含 high vs max 成本/
  产出对照（output tokens、命中率、空流率、stall 触发、首轮延迟）。

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
