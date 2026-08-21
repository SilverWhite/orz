# 流式中段解码错误有界重试设计（2026-08-21 设计定稿；纯文档登记、未实施）

> 状态：`implemented`（S1 代码 + S2 测试已闭合 2026-08-21；S3 重建 →
> S4 复验待续。2026-08-21 用户裁决：按「无完整 tool_calls 即重试
> （有界）」实施，无异议。**全面审查处理（2026-08-21）**：事件面重试
> 计数已随本设计一并实施——新增 v0.2 事件 `transport_retry`
> （`outcome=recovered|exhausted`、`kind=zero_chunk|midstream`、
> `retries`、`reason`；成功路径随 `ModelResponse.transport_retry` 上报、
> 失败路径随 `GatewayError::StreamInterrupted` 上报，run 层 agent_loop
> 记录，run-event enum 54 项、schema/fixtures/conformance 同步；TUI
> 事件协议同步）。）
> 性质：P0-0d 后续（transport 重试分类修订，ADR-0007 §4 边界 ④ 扩展）。
> 关联：[ADR-0007](../adr/ADR-0007-transport-retry-policy.md) §2.1/§4、
> [STALL_DEGENERATION_FAILFAST_DESIGN_2026-08-21.md](STALL_DEGENERATION_FAILFAST_DESIGN_2026-08-21.md)
> （正交：解码重试管无副作用传输失败、fail-fast 管退化哨兵语义）、
> ADR-0010 §14.36（窗口 50s→180s，已实施 S1/S2）。
> 实施路由：S1 代码 → S2 测试 → S3 重建 → S4 复验（与窗口 180s +
> fail-fast 批次合并一次到位）。设计轮不动计数（27）。

## 1. 背景与证据

### 1.1 dna-assembly Disabled 档解码错误（sweep-r1-g1，2026-08-21）

dna-assembly trial 时序（orz.txt 核对）：

1. 19:16:49 `reasoning_stall`（64K/362s）→ 降 low → low 重试成功；
2. 19:20:57 `reasoning_repetition`（5 个相同 delta，consecutive=1）→
   降 low → **low 重试本身再陷 rep**（consecutive=2，37s 后被秒抓）→
   降 disabled；
3. disabled 重试 → `error: model error: transport error: error
   decoding response body` → **无重试日志、run 非零退出**、reward 0。

归因：`error decoding response body` 是 DeepSeek 流式端截断签名
（2026-08-12 TB job1 同签名，当时归因为服务侧中断）。当前纪律
（ADR-0007 §4 / GAP-STREAM-RETRY）：**零 chunk** 的 Transport/Timeout
中断 wrap 为 `StreamInterrupted` 重试（窗口/次数双上限）；
**已见 chunk** 的中断不重试（"已见输出不重试"，防重复工具调用）。
dna 本次为已见 chunk 后截断 → 不重试 → 直接杀 run。

### 1.2 幂等性重新核对：错误路径无副作用

架构核对：工具调用只在 `generate_stream` 完整成功返回后才执行
（`ModelResponse` 全量装配 → controller 发放订单）；失败流的半截
content/reasoning/tool_calls 全部丢弃，**从未执行**。因此"已见输出
不重试"的原始动机（重发会重复执行工具）在错误路径上并不成立——
重发同一请求体是幂等的（模型输出无持久副作用）。真正需要保守的
边界是「已解码出完整 tool_calls 的失败流」：即便我们的执行路径不会
跑它，保留不重试纪律作为双保险。

## 2. 决策（用户裁决：无完整 tool_calls 即重试（有界））

### 2.1 重试判定标准统一为「无完整 tool_calls」

- **可重试**（Transport/Timeout 类中断，含 `error decoding response
  body` 截断签名、`stream ended without finish_reason`）：失败流**未
  解码出任何完整 tool_calls**——零 chunk 与已见 chunk 统一纳入，重发
  幂等。
- **不重试**：失败流已解码出完整 tool_calls（双保险，即便执行路径
  不会运行它）；Model/Parse/Cancelled 类错误（永久/用户取消）维持不
  重试。
- 实现形态：`wrap_zero_chunk` 语义扩展为
  `wrap_no_tool_side_effects(saw_complete_tool_calls, e)`——saw_chunk
  不再是重试判据，完整 tool_calls 才是。

### 2.2 重试预算（有界）

- **零 chunk**：维持既有纪律——180s 窗口 / 10 次上限（ADR-0010
  §14.36，已实施），指数退避 500ms→10s + 10% jitter。
- **已见 chunk 但无完整 tool_calls**：**至多 1 次额外重试**（有界，
  防"已烧数十 K 再重试烧一轮"的病态放大），随后显式失败；重试沿用
  同参同上下文（byte-identical）。
- 重试耗尽后的错误保留 attempts 计数（既有 `StreamInterrupted`
  detail 形态），journal 可见。

### 2.3 与既有机制交互

- D-6 空响应链（完成型空响应快速重试 ≤2 → 降级）不变；本设计只扩
  传输/解码中断的判定面。
- 降级梯各档（high/low/disabled）的请求同样适用本重试规则——dna
  场景下 disabled 档解码错误会先有界重试 1 次，仍失败才显式终止。
- 退化哨兵计数（fail-fast 设计）与本设计正交：解码重试不增减
  `degeneration_consecutive`。
- 可观测性：重试计数入事件面（P1 既有登记项，随本设计一并实施——
  transport 重试尝试/耗尽事件，schema 先行扩展）。

## 3. 参数与边界

- 新增（或复用既有参数面）：chunked 中段重试次数上限 = 1；
- `request_retry_window` 180s / `request_max_retries` 10（零 chunk
  路径，ADR-0010 §14.36）；
- 不新增运行期旋钮（编译期常量，遵循"无新增旋钮"纪律）；
- 错误分类边界：仅 Transport/Timeout 类进入新判定；JSONDeserialize
  （Parse）类仍不重试（若 `error decoding response body` 在实测中
  以 Parse 形态出现，S1 时按实测分类修正映射后纳入）。

## 4. 实施路由与验证

1. S1 代码：重试判定改「无完整 tool_calls」、chunked 有界 1 次重试、
   事件面尝试计数；**已实施**——`wrap_no_tool_side_effects` +
   `has_complete_tool_call`、`CHUNKED_MIDSTREAM_MAX_RETRIES=1`、
   v0.2 `transport_retry` 事件（schema/fixtures/TUI 同步）；
2. S2 测试：中段截断重试成功、中段截断重试耗尽显式失败、已见完整
   tool_calls 不重试、Parse 不重试、与降级梯交互、既有零 chunk 纪律
   回归全绿；**已闭合**（新增 recovered/exhausted 事件集成测试 +
   既有回归，orz-loop 544 通过 / Python conformance 230 通过）；
3. S3 重建（Linux musl，ORZ-BUILD-MOUNT-001 契约；与窗口 180s +
   fail-fast 批次合并一次到位）；
4. S4 复验：dna-assembly 类场景不再因解码错误杀 run（有界重试后
   显式失败）、零 400、命中率 ≥90% 不变量。

## 5. 关联登记

- 本设计定稿后：ADR-0010 §14.37、ADR-0007 修订注记、BACKLOG 0d、
  TODO P0-0d 后续、CLI_PROJECT_INDEX 登记。
