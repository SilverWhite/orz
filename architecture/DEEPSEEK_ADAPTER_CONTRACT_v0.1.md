# DeepSeek 专项 Adapter 契约 v0.1

状态：官方文档约束下的 development-only implementation contract；无真实 API 调用。

检查日期：2026-07-21。适配目标是当前官方 `deepseek-v4-pro` 与 `deepseek-v4-flash`，不是旧模型别名。

## 1. 边界

DeepSeek adapter 位于通用 runner 与 provider transport 之间。它负责把 provider 特有行为显式化，但不得：

- 改写 EvidenceKernel、gate、claim 或 oracle isolation 语义；
- 把 API 请求成功解释为 evidence/claim 成功；
- 静默切换模型、thinking mode、transport 或工具能力；
- 把 provider `reasoning_content` 暴露给 tested Agent、普通日志或非密封 reviewer 输出；
- 在没有独立 leak/evidence gate 的情况下授权 evaluation/holdout。

旧 INDEX/MAP/self-check 不属于本 adapter 的启动依赖。只有具体 LIF claim-bearing 任务才跨目录显式读取这些来源。

## 2. 官方接口事实与本地裁决

### 2.1 模型与 transport

官方当前提供 OpenAI Chat Completions 与 Anthropic Messages 两种兼容入口：

- OpenAI：`https://api.deepseek.com`
- Anthropic：`https://api.deepseek.com/anthropic`
- 模型：`deepseek-v4-pro`、`deepseek-v4-flash`

`deepseek-chat` 与 `deepseek-reasoner` 将于 2026-07-24 15:59 UTC 退役。adapter preflight 必须拒绝旧别名，运行前还应通过 `/models` 或版本化 capability snapshot 确认请求模型存在。

Anthropic 兼容入口会把未知模型名自动映射为 `deepseek-v4-flash`。本项目不得依赖该回退；未知模型在本地直接失败，manifest 中的 requested/resolved model 必须一致。

### 2.2 Thinking 与工具调用

thinking 默认开启；官方提供 `low`/`high`/`max` 三档 effort（2026-08 初补齐；`medium`→`high`、`xhigh`→`max` 静默映射）。为了保持 resolved configuration 可审计，profile 只接受显式档位。

**运行裁决（2026-08-07，FIX_PLAN D-6）**：orz 采用 `thinking: {type: "enabled"}` + `reasoning_effort: "max"` + `max_tokens: 160_000`（三实例统一：主 agent + 两检索子代理）。160K 是总量（128K 思考 + 32K content 的期望分配；DeepSeek 无子预算参数，上限 384K）。实测校准（2026-08-07 live probe）：首个 reasoning delta ~559ms、content 首 delta ~30s（满负荷 thinking 下 content 迟到是常态，不是挂死）、每轮 `usage.completion_tokens_details.reasoning_tokens` 可得。例外（2026-08-07 审查 F-07）：`-p` plan gate 是快速预检轮（max_tokens=1024），经 `ModelRequest.thinking` 请求级覆盖**显式禁用** thinking——快决策轮不做深度推理，避免 1024 预算被思考吃光触发空 content 链 ×3。

thinking 模式会忽略 `temperature`、`top_p`、`presence_penalty` 和 `frequency_penalty`，且不返回错误。本地 preflight 因此拒绝同时声明这些参数，避免把未执行的 sampling 设置写入 provenance。

**空 content 重试链（D-6）**：满负荷 thinking 可能烧光预算留下空 content（finish=length、零输出）。区分两类空 content：

1. 工具轮 `content==''` + 有 tool_calls = **合法**（官方样例），不重试；
2. 最终轮 `content==''` + finish=length = **异常** → 字节级重试一次（同一请求；DeepSeek 要求带原 `reasoning_content` 回传）→ 仍空则 `thinking: disabled` 降级重试一次（等价 effort=none）→ 链终点仍空则显式失败（「预算耗尽零输出」，非静默）。

thinking 模式发生 tool call 时，assistant 的完整 `reasoning_content` 必须在后续请求中回传，否则 API 返回 400（**空串回传 200、空对象/缺失 400**——回放保留 `""` 而非 null/缺省）。adapter 必须：

1. 在 provider-private transcript 中保留原值；
2. 在普通 journal 只记录存在性、长度与 digest，不记录原文；
3. 在每次带 tool result 的后续请求前机械确认对应 assistant message 仍含 `reasoning_content`；
4. 不把该字段当作 EvidenceKernel 的 reasoning precommitment；
5. `reasoning_content` 回放逻辑单点化（一个适配层）——DeepSeek 可能放宽回放要求（LangChain 2026-06 记录），未来反转时单点可控。

### 2.3 Anthropic 兼容层

官方兼容矩阵明确存在 ignored 或 unsupported 字段。V0 采取 fail-fast：

- 不以 `anthropic-version`、`anthropic-beta`、`service_tier`、`container` 或 `cache_control` 证明能力；
- 不依赖 `disable_parallel_tool_use`，因为服务端会忽略；需要串行时由本地 ToolBroker 强制；
- 不把 `tool_result.is_error` 作为唯一错误信号，因为服务端忽略；journal 仍记录独立 terminal state；
- image、document、MCP content block、container/code-execution result 在 preflight 直接拒绝；
- 未支持字段不能只记 WARN 后继续形成“已执行”记录。

### 2.4 JSON 输出

官方 JSON Output 需要 `response_format={"type":"json_object"}`，prompt 中出现 JSON 要求并提供期望示例；`max_tokens` 过小会截断，而且服务端偶尔可能返回空内容。

因此 adapter 只有在 profile 同时声明 prompt contract、示例和正 `max_tokens` 时才启用 JSON mode。空 content、截断或 parse failure 都是无有效产物，不得生成 succeeded artifact；是否重试由冻结的 retry policy 决定。

### 2.5 长等待、stream 与错误

DeepSeek 排队期间可能返回：

- 非 streaming：空行；
- streaming：SSE `: keep-alive` comment。

这些只表示连接仍存活，不是 token、tool event 或 terminal event。若 10 分钟仍未开始 inference，服务端会关闭连接；runner 必须把“未收到首个语义事件”与“生成中断”分开记录。

HTTP 分类采用窄策略：

- 400/422：请求或参数错误，不自动重试；
- 401：认证失败，不重试；
- 402：余额不足，不重试；
- 429/500/503：仅在 manifest 已冻结有限重试策略时重试；
- 其他状态：unknown，不能猜成 retryable。

### 2.6 隔离与 cache

官方 context cache 默认开启，并通过 `prompt_cache_hit_tokens` / `prompt_cache_miss_tokens` 报告命中。cache 命中是 provider 计费/性能信息，不构成独立 evidence。

`user_id` 影响安全、KV cache 和调度隔离。它必须是非隐私 opaque token；不得包含 case ID、用户名、路径或原始研究标识。evaluation 以后应从 run identity 派生，并保存在密封 identity map。

## 3. V0 profile 与 spike

`runtime/deepseek-adapter-profile-v0.1.schema.json` 固定无网络 preflight 输入。`prototype/fep_agent_proto/deepseek_adapter.py` 只实现：

- 当前模型、transport/base URL、thinking/sampling、JSON mode、timeout 与 retry policy 检查；
- tool transcript 中 reasoning continuity、tool-call ID 与 arguments JSON 检查；
- SSE blank/keep-alive/data/[DONE] 分类；
- HTTP 状态的 retryability 分类。

它不保存 API key、不发网络请求、不执行工具、不消费真实 CoT，也不是生产 adapter。

`prototype/fep_agent_proto/model_transport.py`、`deepseek_client.py` 与
`model_loop.py` 现已补充下一层 mock-only integration spike：构造真实形状的
OpenAI Chat Completions body、组装 SSE delta、在内存中回传 thinking tool-call
所需的 `reasoning_content`，并把脱敏 metadata 写入既有 journal。该实现的
transport 固定 `real_network=false`，不改变本节的“无真实 API 调用”边界。

`deepseek_https.py`、`external_network.py` 与 `credentials.py` 又补充了一个
real-network-capable 但 offline-only 验证的 transport 安全骨架。它固定官方
endpoint，要求单次 request-digest permit，并只从当前用户 Windows Credential
Manager 的固定 target 读取 key。该 transport 尚未接入 model loop 或任何可执行
联网 CLI，测试只使用 injected fake connection。

## 4. 后续实现顺序

1. 以 Grok Build 的 model/session 分离方式定义 provider-neutral `AdapterCapabilities`。
2. 先接 OpenAI Chat Completions transport；Anthropic transport 保留为兼容性测试面，而非默认捷径。
3. 加入 `/models` capability preflight、requested/resolved manifest 和无密钥 mock server contract tests。
4. 实现 provider-private transcript + redacted journal digest，并测试 tool call 多轮 reasoning continuity。
5. 实现 streaming parser、取消、首事件 timeout、bounded retry 和 terminal-state mapping。
6. 完成 sandbox/tool broker 后才运行 development scenario；evaluation/holdout 仍需独立密封域和语义 leak review。

## 5. 官方来源

- [DeepSeek first API call and current models](https://api-docs.deepseek.com/)
- [DeepSeek Thinking Mode](https://api-docs.deepseek.com/guides/thinking_mode/)
- [DeepSeek Anthropic API compatibility matrix](https://api-docs.deepseek.com/guides/anthropic_api/)
- [DeepSeek JSON Output](https://api-docs.deepseek.com/guides/json_mode/)
- [DeepSeek Tool Calls](https://api-docs.deepseek.com/guides/tool_calls/)
- [DeepSeek Rate Limit & Isolation](https://api-docs.deepseek.com/quick_start/rate_limit/)
- [DeepSeek Error Codes](https://api-docs.deepseek.com/quick_start/error_codes/)
- [DeepSeek Models endpoint](https://api-docs.deepseek.com/api/list-models/)
- [Grok Build official repository](https://github.com/xai-org/grok-build)
