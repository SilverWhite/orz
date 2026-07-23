# DeepSeek External HTTPS Transport 安全契约 v0.1

状态：development-only。底层 transport 已离线验证；两阶段 one-shot probe
提供唯一真实联网入口。2026-07-23 首个真实 permit 已消费，transport 以脱敏
`OSError` 终止；没有重试，provider receipt 与计费状态未知。

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

permit 是调用方完成用户交互后传下来的窄 capability，不是“用户已经理解请求内容”的证明。scripted broker 只服务于内建 fake provider。真实 development probe
先离线生成固定请求的 confirmation summary；execute 阶段必须重新构造并比对同一
request/summary digest，再要求输入 `ALLOW-<摘要前12位>`。真实 broker 拒绝 retry，
且不能与内建 fake provider 配对。

## 4. Windows 凭据边界

默认 `UnavailableCredentialProvider` 总是失败。Windows 实现只读取当前用户 Credential Manager 中固定名称的 Generic Credential：

```text
FEP-Agent/DeepSeek
```

本项目不接受 API key CLI 参数、配置文件字段或环境变量。credential blob 契约为 UTF-16LE 保存的 printable ASCII key；读取后包装成短生命周期 `CredentialLease`，退出 context 时对其 bytearray 做 best-effort 覆盖。Python string、HTTP 库和内核缓冲可能产生不可控副本，因此这不宣称密码学意义上的内存零化。

Windows provider 直接从 `CredReadW` buffer 逐 UTF-16LE code unit 填充 owned
`bytearray`，不再创建中间 blob bytes、Unicode string 和第二份 ASCII bytes。
`CredentialLease` 接管同一 buffer 并在 close 时原地覆盖；Authorization Python string
和 HTTP/内核副本仍不可避免。

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

`deepseek-external-readiness` 保留为原始离线 readiness 检查，只检查 profile、TLS context 和静态阻断条件：

- 不读取 Credential Manager；
- 不签发 permit；
- 不做 DNS、socket、TLS handshake、`/models` 或余额检查；
- 输出 `ready_for_network=false` 与三个显式 blocker。

HTTPS transport 单元测试注入 fake connection，验证 host、port、path、Bearer header、SSE、permit 一次性和 metadata 脱敏。`NETWORK_PERMIT_BROKER_CONTRACT_v0.1.md` 进一步用内建 in-process fake provider 接入完整两轮模型循环及一次 503 retry；生产默认 connection factory 没有在测试中调用。

## 7. One-shot development probe

`deepseek-real-development-plan` / `deepseek-real-development-execute` 固定：

- model=`deepseek-v4-pro`；
- thinking disabled；
- 两条无项目数据的固定 message；
- `max_tokens=16`；
- stream + usage；
- 工具数 0、retry 数 0、HTTP attempt 上限 1；
- 当前用户 Credential Manager target=`FEP-Agent/DeepSeek`。

plan 阶段不读 credential、不联网。execute 在确认后消费一次 permit，缺 credential、
TLS/HTTP/provider 失败都不能重放。成功结果不保存 prompt、回复正文、provider-private
reasoning、confirmation token 或 Authorization，只保存 digest、长度、usage、固定
marker 比对和脱敏 transport metadata。该入口不接 model loop、工具或研究场景。

execute 在任何 credential access 前必须为当前短生命周期 CLI 进程设置并读回验证
Windows WER `NOHEAP`，同时关闭 Python `faulthandler`。transport metadata 机械声明并
由测试覆盖：endpoint pinned、proxy environment unused、redirect count 0、HTTP debug
disabled。成功 `result.json` 和失败 `failure.json` 都在原子写入前扫描当前 output root
及 pending document 的常见 `sk-...` / Bearer credential pattern；扫描不读取真实 key。

沙箱内预检曾得到 WinError 1168；同一凭据在沙箱外通过固定 provider 成功读取并释放，
说明前者来自 credential-vault 隔离，不是 target 缺失。首个 execute 消费一条 allow
ledger event 后以通用脱敏 `OSError` 退出，没有 `result.json`，也没有 retry。随后独立
诊断确认 DNS、TCP 443、TLS 1.3 以及 key 的 `sk-` 前缀/长度范围正常；这些检查不证明
POST 已抵达 provider，也不能判定是否计费。

首个 attempt 还暴露了 socket 轮询缺口：250ms poll timeout 原本会被误当作整体超时。
第一次修正试图在短 timeout 后继续复用 `HTTPResponse.readline()`；后续 hardened real
attempt 在 `response_body` 阶段证明该假设不成立，因为 Python buffered `SocketIO`
可能在一次 timeout 后永久进入 timed-out 状态并改抛 `OSError`。实现现改为每次读取前
设置冻结 first-semantic/total deadline 的实际剩余时间；底层 timeout 直接映射为对应
deadline，绝不重用已 timeout 的 buffered reader。错误包装只保留脱敏 stage、exception
type 和 numeric errno，不记录异常正文。

## 8. 仍未开放

- 没有真实 credential provisioning 命令；
- 没有 provider `/models` capability discovery；
- 没有对真实证书、DNS、限流、余额或账户权限的成功验证；
- model loop 仍拒绝 real-network transport；
- 没有 session-wide、retry-wide 或自动批准；
- 没有 evaluation/holdout、研究数据或 claim-bearing 调用。

WER `NOHEAP`、短进程和 artifact pattern scan 不能阻止管理员、debugger、恶意软件、
pagefile、hibernation、外部 dump 工具或 provider 认证系统读取内存/Authorization。
Python immutable string、HTTP 库和内核缓冲仍无法做密码学零化，因此不得宣称“API key
绝对未被读取”或“绝对无任何残留”。

## 9. 官方来源

- [DeepSeek first API call and Bearer authentication](https://api-docs.deepseek.com/)
- [DeepSeek Create Chat Completion](https://api-docs.deepseek.com/api/create-chat-completion/)
- [DeepSeek Error Codes](https://api-docs.deepseek.com/quick_start/error_codes/)
- [Python SSL default context](https://docs.python.org/3/library/ssl.html#ssl.create_default_context)
- [Microsoft CredReadW](https://learn.microsoft.com/en-us/windows/win32/api/wincred/nf-wincred-credreadw)
- [Microsoft CREDENTIALW](https://learn.microsoft.com/en-us/windows/win32/api/wincred/ns-wincred-credentialw)
- [Microsoft CredFree](https://learn.microsoft.com/en-us/windows/win32/api/wincred/nf-wincred-credfree)
- [Microsoft WerSetFlags / WER NOHEAP](https://learn.microsoft.com/en-us/windows/win32/api/werapi/nf-werapi-wersetflags)
