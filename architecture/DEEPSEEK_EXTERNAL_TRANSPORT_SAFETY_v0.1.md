# DeepSeek External HTTPS Transport 安全契约 v0.1

状态：development-only、offline-verified implementation spike；实现真实外网 transport 的构造边界，但没有用户可执行的真实联网命令，也没有读取真实凭据或发起真实请求。

## 1. 目标与非目标

本层把 loopback transport 后仍缺少的 TLS、Bearer 认证与外网授权拆成独立组件。它不改变 protocol v0.1 的 action、permission、evidence、claim 或 oracle-isolation 语义；provider 成功响应仍不等于研究证据成立。

旧 INDEX/MAP/self-check 不属于 transport 初始化依赖。只有具体 claim-bearing 任务才按需跨工作区读取。

## 2. 固定 endpoint 与 TLS

`DeepSeekHttpsTransport` 不接受任意 URL，只使用：

- host：`api.deepseek.com`；
- port：`443`；
- path：`/chat/completions`；
- endpoint identity：`https://api.deepseek.com/chat/completions`。

transport 直接构造 `HTTPSConnection`，不读取 proxy URL、不实现 redirect、不接受调用者注入 header。TLS context 使用系统信任库、强制证书验证和 hostname verification，并把最低版本固定为 TLS 1.2。request/response 使用 strict UTF-8 JSON/SSE 和双向 byte limit。

## 3. 外网批准 capability

`OneShotNetworkPermit` 绑定：

- provider=`deepseek`；
- 精确 chat endpoint；
- strict JSON request body 的 SHA-256；
- 最多 300 秒 TTL；
- 单次消费状态。

请求体变化、endpoint/provider 不同、过期或二次使用都会 fail closed。许可只存在内存，不能序列化或写入配置。日志只可记录随机 permit ID 的 SHA-256 receipt。

permit 是调用方完成用户交互后传下来的窄 capability，不是“用户已经理解请求内容”的证明。新增的 scripted broker 只服务于内建 fake provider；本阶段没有签发真实 permit 的 CLI，因此普通命令无法触发真实 transport。

## 4. Windows 凭据边界

默认 `UnavailableCredentialProvider` 总是失败。Windows 实现只读取当前用户 Credential Manager 中固定名称的 Generic Credential：

```text
FEP-Agent/DeepSeek
```

本项目不接受 API key CLI 参数、配置文件字段或环境变量。credential blob 契约为 UTF-16LE 保存的 printable ASCII key；读取后包装成短生命周期 `CredentialLease`，退出 context 时对其 bytearray 做 best-effort 覆盖。Python string、HTTP 库和内核缓冲可能产生不可控副本，因此这不宣称密码学意义上的内存零化。

`CredReadW` 返回的 credential 必须由 `CredFree` 释放。错误信息不包含 key，普通 metadata 只记录 provider source ID，并固定 `authorization_recorded=false`。

## 5. 调用顺序

```text
normalized request
  -> strict JSON bytes + SHA-256
  -> consume exact one-shot permit
  -> acquire short-lived credential
  -> direct verified TLS connection
  -> POST fixed /chat/completions
  -> bounded SSE read
  -> close response/connection and credential lease
```

许可在凭据读取前消费。即使凭据缺失、TLS 失败或服务端拒绝，本次许可也不能重放。有限 HTTP retry 若未来开放，必须为每次实际请求重新取得 permit；不能把一次批准扩张成未界定的重试预算。

## 6. 离线验证面

`deepseek-external-readiness` 只检查 profile、TLS context 和静态阻断条件：

- 不读取 Credential Manager；
- 不签发 permit；
- 不做 DNS、socket、TLS handshake、`/models` 或余额检查；
- 输出 `ready_for_network=false` 与三个显式 blocker。

HTTPS transport 单元测试注入 fake connection，验证 host、port、path、Bearer header、SSE、permit 一次性和 metadata 脱敏。`NETWORK_PERMIT_BROKER_CONTRACT_v0.1.md` 进一步用内建 in-process fake provider 接入完整两轮模型循环及一次 503 retry；生产默认 connection factory 没有在测试中调用。

## 7. 尚未开放

- 没有真实联网 CLI 或自动 permit issuer；
- 没有真实 credential provisioning 命令；
- 没有 provider `/models` capability discovery；
- 没有对真实证书、DNS、限流、余额或账户权限的验证；
- real-network-capable transport 只通过明确的 in-process fake factory 接入 model loop；默认真实 connection factory 仍被拒绝；
- 没有批准真实计费或外部数据披露。

## 8. 官方来源

- [DeepSeek first API call and Bearer authentication](https://api-docs.deepseek.com/)
- [DeepSeek Create Chat Completion](https://api-docs.deepseek.com/api/create-chat-completion/)
- [DeepSeek Error Codes](https://api-docs.deepseek.com/quick_start/error_codes/)
- [Python SSL default context](https://docs.python.org/3/library/ssl.html#ssl.create_default_context)
- [Microsoft CredReadW](https://learn.microsoft.com/en-us/windows/win32/api/wincred/nf-wincred-credreadw)
- [Microsoft CREDENTIALW](https://learn.microsoft.com/en-us/windows/win32/api/wincred/ns-wincred-credentialw)
- [Microsoft CredFree](https://learn.microsoft.com/en-us/windows/win32/api/wincred/nf-wincred-credfree)
