# 流式中断重试 实施审计（2026-08-12）

- 审计对象：GAP-STREAM-RETRY — 流式路径零 chunk 产出中断重试（TB job1 实测缺口，ACAF 前排期）
- 日期：2026-08-12
- 范围：`orz/crates/orz-loop`（`gateway/model.rs` 新错误变体 + `gateway/transport.rs` 流式路径）+ 主仓 ADR-0007 修订（§2.1 条款 + §4 标记闭合）+ 索引登记
- 用户裁决：① **流式中断需要能够重试**（2026-08-12）② **边界**：仅零 chunk 产出中断可重试（重发同一请求体零副作用、模型调用幂等）；已产出 chunk 保持不重试（防重复工具调用）
- 验证摘要：orz-loop **181/0/3**（transport 37 全绿，+7 新测试）；workspace 167 目标 **0 failed**；clippy `-D warnings` 零新增；capture 13/13；pytest **1606/14**；check_repository **valid**；diff --check 干净

## 1. 背景与缺口

| 缺口 | 登记出处 | 本切片状态 |
|---|---|---|
| 流式路径不重试（§2.1）对零 chunk 中断同样生效 → 偶发中断直接失败 | `ADR-0007 §4` 登记（2026-08-12） | **闭合** |
| TB job1 评测 4 trial 中 3 个死于 `transport error: error decoding response body`（流式中断，agent exit 1，两题 0 分） | `docs/TERMINAL_BENCH_2_JOB1_EVAL_2026-08-12.md` §6 | **闭合**（重试后该签名错误可恢复） |

TB job1 实测细节（两次运行，gpt2-codegolf + make-doom-for-mips）：运行 1~8 分钟后某轮模型请求报 `transport error: error decoding response body`；历史基线 8月11 全部 job 零此错误、8月8 出现 1 次——pre-existing 偶发，指向 DeepSeek API 流式端点服务侧中断，与 orz 代码/容器网络无关（宿主 curl 流式单请求正常）。

## 2. 用户裁决（2026-08-12）

- **D-1 重试**：流式中断需要能够重试。
- **D-2 边界**：仅**零 chunk 产出**的中断纳入重试——连接握手/首字节前失败，重发同一请求体零副作用（模型调用幂等，ADR-0007 §3 已知边界的同一前提）；**已产出 chunk** 的中断保持不重试（重发会重复工具调用，幂等性不可保证）。
- **D-3 排期**：ACAF Slice 1 之前实施（TB job1 结论「修复将直接提升评测可靠性」）。

## 3. 设计决策

| 决策 | 内容 | 依据 |
|---|---|---|
| D-1 chunk 计数口径 | 任何成功解码的 SSE item 计为 chunk——**reasoning delta 含入**（provider 已为该请求产生输出）；仅零 chunk 尝试可重试 | D-2 用户裁决字面语义 |
| D-2 可重试错误类 | 零 chunk 且错误类 ∈ {Transport, Timeout}（wire 级失败：解码错误/连接断/握手挂死/空闲看门狗/总预算）；**Cancelled / Model / Parse 不重试** | Cancelled=用户取消永不复发；Model（400 等）=permanent；Parse=解码缺陷。§2.1 非流式瞬态分类同构 |
| D-3 429/5xx 握手拒绝 | **不重试**（Model 错误）——fork `ApiError` 无 HTTP status 字段（仅 message/type/param/code），瞬态不可靠分类；非流式路径的 429/5xx 重试由 fork 内部承担（§2.1） | 可靠分类缺失；避免对 permanent 拒绝无意义重试烧配额 |
| D-4 参数面 | **零新增字段/env**：复用 `request_max_retries`（10）+ `request_retry_window`（32s），§2.3 参数化面 | ADR-0007 §4 登记要求（修订前原文「接入 RetryPolicy 参数化面（§2.3）」，修订后表述「参数面零新增（§2.3 复用）」） |
| D-5 退避节奏 | `backoff::ExponentialBackoff { max_elapsed_time: Some(request_retry_window), ..Default }`——与 fork `execute_raw` 逐字一致（**次数上限与退避窗口双约束，先到者止**） | §2.1 与 fork client.rs execute_raw 语义镜像 |
| D-6 结构 | 新增 `GatewayError::StreamInterrupted { attempts, detail }`；`stream_once` 零 chunk 失败 wrap（attempts=0）；`generate_stream` 三 D-6 stage 各经 `stream_once_with_retry` 循环，耗尽后返回带 attempts 计数的变体 | journal 呈现最终错误文本含重试历史（可观测性） |
| D-7 cancel | 退避 sleep 经 select!（biased cancel 优先）→ 退避期间 /stop 立即 Cancelled；每次尝试前 stream_once 的 pre-cancel 检查既有 | /stop 不被重试退避阻塞 |
| D-8 heartbeat | 每次退避醒来 stamp（P1-1） | 360s stall 看门狗 vs 32s 窗口本不冲突，stamp 为一致性正确性 |
| D-9 与 D-6 空内容链正交 | 三个 stage（正常→字节相同重试→thinking 禁用降级）各自带零 chunk 重试；stage 成功=流完整跑完（含其重试） | 现有 D-6 链语义不变 |
| D-10 总预算 | 每次尝试独立 `total_deadline`（30min）；链级最坏时长=(max_retries+1)×单尝试上限 | 与 §2.2 单请求预算语义一致；最坏时长由双参数收紧（登记 §6.5） |

## 4. 实现清单（orz 仓）

### 4.1 `gateway/model.rs`

- `GatewayError` 新增变体：`StreamInterrupted { attempts: u32, detail: String }`，Display `stream interrupted before any chunk (after {attempts} attempts): {detail}`；`attempts=0` = 未重试即逃逸（或首次尝试失败）；变体为 additivity，agent_loop 仅 match `Cancelled` 不受影响

### 4.2 `gateway/transport.rs`

- `wrap_zero_chunk(saw_chunk, e)`：零 chunk 且 e ∈ {Transport, Timeout} → `StreamInterrupted{attempts:0, detail}`；已产出 chunk 或 Model/Parse/Cancelled → 原样返回。注释锁定裁决边界
- `stream_once`：`saw_chunk` 在第一个解码成功的 SSE item 后置位；六处错误出口全部经 wrap（pre-cancel 检查除外——Cancelled 永不 wrap）：
  1. 握手挂死（F-08 timeout，恒零 chunk）→ wrap
  2. 握手 `map_error`（恒零 chunk）→ wrap（Model 类经 wrap 原样返回=不重试）
  3. select! 循环内空闲看门狗超时 → 按 `saw_chunk` wrap
  4. select! 循环内总预算超时 → 按 `saw_chunk` wrap
  5. `stream.next()` 错误 → 按 `saw_chunk` wrap（**TB 签名 `error decoding response body` 落点**）
  6. 截断（EOF 无 finish_reason，D1-1）→ 按 `saw_chunk` wrap
- `stream_once_with_retry`（新助手）：循环调用 `stream_once`；`StreamInterrupted` → 次数 < `request_max_retries` 且退避窗口未耗尽 → `backoff.next_backoff()`（None=窗口耗尽即止）→ select! cancel 打断 sleep → stamp 心跳 → 重试；耗尽 → 返回带 attempts 计数的 `StreamInterrupted`。三次失败路径各 `tracing::warn`（重试进行中/次数耗尽/窗口耗尽，detail 含错误原文）
- `generate_stream`：三处 `stream_once` 调用换 `stream_once_with_retry`（D-6 链注释同步说明正交性）

### 4.3 测试（orz-loop，+7 新 / 1 适配）

| 测试 | 覆盖 |
|---|---|
| `generate_stream_zero_chunk_transport_error_retries_then_succeeds` | **TB 签名**：首连 content-length 截断 → `error decoding response body` 零 chunk → 重试后成功；断言成功+文本+连接数≥2（见 §6.1 fork 重连说明） |
| `generate_stream_zero_chunk_eof_retries_then_succeeds` | 零 chunk EOF 截断（`data: [DONE]`）→ 重试恰好一次（EOF 不触发 fork 重连，计数精确） |
| `generate_stream_after_chunk_interruption_is_not_retried` | 已产出 chunk 后截断 → Transport 原样，**连接数恰 1**（重试即重复工具调用） |
| `generate_stream_zero_chunk_retry_capped_by_max_retries` | max=3 → 恰 4 次连接 + `StreamInterrupted{attempts:3}`（journal 可观测） |
| `generate_stream_zero_chunk_retry_window_caps_before_max_retries` | 窗口 250ms / max=10 → 尝试 ≤3（窗口独立封顶） |
| `generate_stream_cancel_during_retry_backoff_aborts_promptly` | 退避 500ms 期间 50ms 取消 → 400ms 预算内 Cancelled、无第二次连接 |
| `generate_stream_handshake_hang_retried_then_succeeds` | 握手挂死（pre_delay 2s > idle 150ms）零 chunk Timeout → 重试成功 |
| `generate_stream_idle_watchdog_aborts_silence`（**适配**） | 零 chunk 空闲现为可重试：max=1 → 重试 1 次后 `StreamInterrupted{attempts:1, detail 含 idle}` + 连接数恰 2 |

- mock 基建扩展：`MockResponse.write_truncated`（响应头声明完整 content-length、只写前一半 body 后断连 → 客户端读错误）+ `truncated()` builder + `sse_delayed()`（握手挂死场景）

## 5. 验证（全绿）

- orz-loop：**181 passed / 0 failed / 3 ignored**（transport 模块 37/0/1；`gateway::transport` 组 +7 全过）
- workspace：167 目标 `cargo test --workspace -j2` **exit 0 零失败**（-j2 低并行防 0xc0000409 rustc 栈溢出惯例）
- clippy `-p orz-loop --all-targets`：零 warning
- capture：`cargo test -p orz-bin -- --ignored conformance_capture` **13/13**（journals 重捕，EXPECTED_SEQUENCES 字节未动——零事件/schema 变更）
- pytest（assurance 目录收集，C2-3 惯例注明范围）：**1606 passed / 14 skipped / 0 failed**（总数 1620 与基线一致；根目录全量另计）
- check_repository：**valid**（235 schemas / 13 journals）
- `git diff --check`：干净

## 6. 登记边界

1. **fork EventSource 内部自动重连（pre-existing）**：fork `post_stream` 的 `reqwest_eventsource::EventSource` 对读错误有内部重连逻辑（0.6.0：300ms 起、×2 递增、上限 5s、**无限次**；fork 的 `stream()` 错误分支不 break 继续 poll，rx 已丢后任务在重连首事件即退出）——与 orz 显式重试**叠加为双保险**，连接数可能超出 orz 重试次数（**运行观测值，非契约断言**：T1 实测 3 连接 = 初始 + fork 重连 + orz 重试；提交的测试仅断言 ≥2）。**不修改 fork**（TB 实测其重连未救回，orz 重试是第二层；对生产是正面冗余）。精确连接计数断言仅用于 EOF/握手挂死场景（不触发 fork 重连）。
2. **429/5xx 握手拒绝不重试**：fork `ApiError` 无 status_code，瞬态不可靠分类（D-3 决策）；该类错误以 Model 显式浮出，与 §2.1 非流式分类的行为差异由 fork 内部承担（非流式才走 fork 重试路径）。
3. **Parse 错误不重试**：SSE item 级 JSON 反序列化失败（JSONDeserialize → Parse）——解码缺陷类，重试无益；TB 签名是连接级读错误（Transport），不受影响。
4. **重试与 D-6 链的交互**：零 chunk 重试成功但内容为空 → 仍走 D-6 空内容链（字节相同重试→thinking 禁用降级）；重试次数不跨 stage 累计（每 stage 独立窗口）。
5. **链级最坏时长（可达界，审查 D3-2 修正）**：每次尝试独立总预算（30min idle 兜底 90s/次）；可重试失败受 idle 看门狗（90s）与共享退避窗口（32s，含尝试时长）共同约束——**单 stage 链级最坏 ≈ 窗口 + 尾尝试 ≈ 2min，三 stage 独立窗口 ≈ 6min**；数学上界 (max_retries+1)×单尝试上限（≈5.5h）仅对含 chunk 流动的不可重试路径可达。可由 `request_max_retries`/`request_retry_window` 收紧（§2.3 参数化面）。
6. **零事件/schema 变更**：重试为 transport 内部行为；journal 呈现最终错误文本（含 re-sends 计数）——捕获的 13 个 journals 结构零 diff。
7. **saw_chunk 为解码级计数（审查 D3-1）**：任何成功解码的 SSE item 都翻转标志，含零实质输出的空 delta 帧（`{"content":""}` / 空 `{}`）——空帧后中断不重试，实际产出为零也保守放弃；安全方向（宁可少重试不可重复工具调用），与裁决字面一致，若未来想恢复覆盖此处是唯一收紧点。
8. **Parse 不重试的确定性假设（审查 D3-6）**：SSE item 级 JSON 损坏 → Parse 不重试（确定性损坏假设——若系代理随机损坏则丢失一次机会，保守方向可接受）；半帧损坏由 eventsource 层丢弃（EOF 无错）→ 走截断检查 → Transport 可重试。
9. **`generate_stream_idle_watchdog_aborts_silence` 名实不符（审查 P3-1，pre-existing）**：`json_delayed` 整包扣发头部触发的是**握手超时**分支，非循环内 idle 分支（150ms 内无头）；in-loop idle 的零 chunk wrap 路径仅被间接覆盖。补 in-loop idle 直测（头部到达后静默）列为后续。
10. **挂死握手中的 cancel（审查 P3-4，pre-existing）**：握手期无 cancel 分支（F-08 既有边界）——挂死中 /stop 最坏延迟 = 一个 idle_timeout（90s 生产默认），与切片前持平，无回归。
11. **usage-only 最终帧计 chunk（审查 P3-6）**：唯一解码项为 usage-only 帧（无 finish_reason）的流 → saw_chunk=true → 不重试，尽管零用户可见输出；与「任何 item 即输出归属」裁决一致（防重复计费方向）。

## 7. 实施记录

- 2026-08-12：ADR-0007 §2.1 流式条款修订（零 chunk 例外）+ §4 登记标记闭合；`CLI_PROJECT_INDEX.md` GAP-STREAM-RETRY `pending` → `implemented`（条目更新 + 状态速查同步 + §0.5 更新检查完成）
- **三面审查（2026-08-12，用户发起"全面检查"）**：设计/实现/符合性三独立代理——**无 P0/P1/D1/C1**；修复批：**D2-1** StreamInterrupted Display 措辞 "attempts" → "re-sends"（journal 不再 off-by-one 误读）；**P2-1** 窗口封顶测试 250ms→1s + 精确断言（250ms 下 backoff 首次 `next_backoff` 即 None 使旧断言空转——注释说明 backoff 0.4.0 `elapsed + randomized_interval <= max_elapsed_time` 判定）；**P2-2** `truncated()` 切点 `len()/2` → 固定 100 字节（与帧结构解耦，正文改动不再翻转测试语义）；**P3-5** cancel 测试注释退避区间 [250,750]ms；**C2-1** 审计 pytest 范围标注（assurance 目录，C2-3 惯例）；**C3-1~7** ADR §2.1 错误类约束/§3 fork 限定/§4 轮次措辞 + 审计 D-4 依据/§4.2 列表拆分/§6.1 观测值标注；**D2-2 用户裁决**（窗口含尝试时长 → 慢速失败单次逃逸）：登记边界、保持 fork 镜像（ADR §4 边界③）；**D3 系列**登记：空 delta 帧/可达界/降级链中止/429 恢复路径/Parse 假设。复验：transport 37/0/1 全绿、clippy `-D warnings` 零警告、workspace 167 目标 0 failed、capture 13/13、pytest 1606/14、check_repository valid、diff --check 干净
- 未提交（用户手动推送惯例）；`orz/` 在 .gitignore（Rust 修复不入 git，推送只含 assurance/文档层）
