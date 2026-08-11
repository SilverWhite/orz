# ADR-0007：transport 重试/超时策略显式化（有限退避 + 看门狗）

- 状态：accepted
- 日期：2026-08-07
- 关联：`存档/docs/implementation-history/FIX_PLAN_2026-08-06.md` D-7（跑分 P1/LOOP-16、P8）、`architecture/DEEPSEEK_ADAPTER_CONTRACT_v0.1.md` §2.5/§4（首事件 timeout、bounded retry、首事件 vs 中断分开记录）、`integration/grok/README.md`（08-02「零重试边界」记录）、`orz-loop/src/gateway/transport.rs`、`orz-loop/src/gateway/model.rs`（`RetryPolicy`）、fork `third_party/async-openai/async-openai/src/client.rs`

## 1. 背景

Polyglot 摸底（2026-08-06）暴露：

- **P8**：非流式 `generate` 路径无超时——16384 预算探测 201 秒不收敛；fork reqwest 客户端无读超时，大 max_tokens 下 `-p` 可长时间阻塞。
- **P1/LOOP-16**：fork 的非流式 create 路径在 429/5xx 时按 `backoff::ExponentialBackoff` 重试。审计初判为「无限重试」，**交叉复核修正**：fork 默认 `max_elapsed_time = Some(15min)`（backoff-0.4.0 `default.rs`），即「15 分钟窗口内的指数退避」——不是字面无限，但对单请求而言 15 分钟窗口远超合理等待，且**未获 ADR 授权**地突破了 08-02「零重试边界」记录（`integration/grok/README.md`: "遵守零重试边界，没有自动发起第二次真实请求"）。

用户裁决（FIX_PLAN D-7）：有限指数退避 10 次/32s 封顶 + 流空闲看门狗为主、总超时放宽为辅助；重试策略与 08-02 的关系需本 ADR 显式化。

## 2. 决策

### 2.1 重试策略：有限指数退避（双上限，先到者止）

- 非流式 create 路径（fork `execute_raw`）：429（非 insufficient_quota）/5xx 视为 transient，按指数退避重试；
  - **次数上限**：`request_max_retries = 10`（fork 新增 `max_retries` 字段 + `with_max_retries`，默认 `None` 保持历史窗口语义——继承 crate 零影响）；
  - **时间上限**：`request_retry_window = 32s`（backoff `max_elapsed_time` 注入）；
  - 400/401/402/422 及其他状态：permanent，不重试（维持 fork 既有分类，与契约 §2.5 一致）。
- **流式路径不重试**：fork `post_stream` 天然单次连接（无 backoff）；「已见输出不重试」由此自动满足——流一旦开始产出 chunk，任何中断只返回错误，绝不重发整个请求（重发会重复工具调用，幂等性不可保证）。
- 重试策略是**协议层容错**（provider 瞬时故障自愈），不是证据层纪律：08-02 记录是 **Grok launcher 证据层**的「真实请求失败后不自动发起第二次」约束。两层不冲突——本 ADR 授权协议层有限重试，证据层零重试记录保持有效（其适用范围限定于 Grok launcher 的一次性 conformance 请求）。

### 2.2 超时策略（取消 ≠ 超时）

- 新增 `GatewayError::Timeout(String)` 变体，与 `Cancelled` **严格区分**：超时是 transport 失败（loop 以模型错误呈现），取消永远不是超时（Gemini #21546 教训：Ctrl+C 被误判 ETIMEDOUT 静默重试）。
- **非流式**：`tokio::time::timeout(request_timeout = 18min)` 包住整个 create（含重试）——`request_timeout` 是总预算，`request_retry_window` 是重试窗口，两者独立、先到者生效。**2026-08-07 审查修正（F-03）**：10min→18min——160K max 档 thinking 合法耗时可超 10min，18min 覆盖慢思考+网络波动；生产路径已全部流式化后此参数覆盖非流式（测试/兜底）路径。
- **流式**（`generate_stream` select! 循环内）：
  - **空闲看门狗为主**：`stream_idle_timeout = 90s` 无任何数据 → 强制中止（TCP 超时抓不到挂死连接）；`stream_idle_warn = 20s` 先告警一次（tracing）。2026-08-07 实测校准：max 档 thinking 流 559ms 即出首个 reasoning delta、29.4s 才出 content——**有进展的慢推理不算超时，只有死线才算**；看门狗按「无任何 chunk」计数，reasoning delta 流动即重置。
  - **总预算为辅助**：`stream_total_timeout = 30min` 兜底整个流（含数据持续流动但永不终结的极端情况）。
- **P8 落点**：`-p` plan gate 由非流式 `generate` 改走 `generate_stream`（`orz-bin/src/main.rs`）——**所有模型轮次统一流式路径**（2026-08-07 B 批 F-03 落地：两检索子代理亦迁流式，此句从计划变为事实），看门狗/总预算/取消自动覆盖；连接握手（`create_stream` HTTP/TLS）同样包 `stream_idle_timeout`（F-08——握手无中间信号，90s 无响应头即死线）。

### 2.3 参数化

- 新 `RetryPolicy`（`orz-loop/src/gateway/model.rs`，`Default` = 本节定稿值）：`request_max_retries` / `request_retry_window` / `request_timeout` / `stream_idle_warn` / `stream_idle_timeout` / `stream_total_timeout`，全部经 `ModelConfig::retry` 注入；消费者可按 Codex 式配置收紧/放宽，零代码改动。

## 3. 后果

- fork：`Client` 加 `max_retries: Option<u32>`（默认 None）+ `with_max_retries()`；`execute_raw` 手写有限循环替代 `backoff::future::retry`（reset → 尝试 → transient 且未达双上限 → sleep → 再试；窗口耗尽或达次数上限 → 返回最后一次错误）。流式 `post_stream` 不变（本就无重试）。
- orz-loop：`RetryPolicy` + `GatewayError::Timeout` + `generate` 超时包装 + `generate_stream` 看门狗；测试 +6（非流式超时、5xx 重试上限、429 重试后成功、空闲看门狗中止、总预算兜底、窗口先于次数封顶）。
- orz-bin：plan gate 改流式。
- 08-02 记录：保持有效（证据层），本 ADR 将其适用范围明确为 Grok launcher 一次性 conformance 请求；协议层重试由 2.1 授权。
- 已知边界：非流式 create 的重试会重复发送**同一请求体**（幂等，因为响应未到达即无副作用）；若未来引入会产生服务端副作用的非幂等调用，需逐调用评估。

## 4. 已登记缺口（2026-08-12 评测实测）

- **流式中断无重试（GAP-STREAM-RETRY；实施待排期，用户裁决已下）**：§2.1「流式路径
  不重试」对**零 chunk 产出**的流式中断同样生效，导致偶发中断直接失败。TB job1 评测
  （2026-08-12 两次运行，gpt2-codegolf + make-doom-for-mips）实测：运行 1~8 分钟后某轮
  模型请求报 `transport error: error decoding response body`，agent 以 exit 1 退出，
  两题均 0 分；历史基线：8月11 全部 job 零此错误、8月8 出现 1 次——pre-existing 偶发，
  指向 DeepSeek API 流式端点服务侧中断，与 orz 代码/容器网络无关（宿主 curl 流式
  单请求正常）。**用户裁决（2026-08-12）**：流式中断需要能够重试。**缺口边界**：
  仅**零 chunk 产出**的中断纳入重试（连接握手/首字节前失败——重发同一请求体零副作用，
  模型调用幂等，§3 已知边界的同一前提）；**已产出 chunk** 的中断保持不重试（§2.1
  理由不变：重发会重复工具调用，幂等性不可保证）。实施需在流式路径区分两种中断
  （chunk 计数），并接入 RetryPolicy 参数化面（§2.3），走"先 Schema/fixture 后 producer"
  与三面审查惯例。
