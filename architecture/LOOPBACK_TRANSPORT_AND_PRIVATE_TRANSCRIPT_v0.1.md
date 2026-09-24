> **pre-ADR-0010 冻结状态头（2026-09-24，审查修复批）**：本文件属 pre-ADR-0010 时期冻结 fixture／development-only spike——**非 `current`、仅历史基线**，不作为实现依据；当前设计权威＝[`ADR-0010`](../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)（本状态头为其 §7.2 归档规则的**就地冻结**轻量替代，不移档不改正文，2026-09-24 批登记）。

# Loopback HTTP Transport 与 Windows Private Transcript 契约 v0.1

状态：development-only pre-network spike；不修改 protocol v0.1 语义，不允许外部模型请求。

## 1. 目的

本层验证真实 HTTP framing、SSE 增量读取、取消/timeout 和 provider-private transcript 崩溃恢复，但把服务端限制在当前进程启动的 `127.0.0.1` mock server。

它位于 scripted transport 与未来 HTTPS DeepSeek transport 之间：比纯内存 fixture 更接近真实 I/O，但仍没有 API key、DNS、TLS、proxy、redirect、外部 endpoint 或真实计费风险。

## 2. Loopback endpoint 不变量

- scheme 只能是 `http`；
- hostname 必须是字面量 `127.0.0.1`，拒绝 `localhost`，避免 DNS/hosts-file 解释；
- 必须显式提供 1–65535 端口；
- base URL 不得含用户名、密码、path、query 或 fragment；
- request path 在代码内固定为 `/chat/completions`；
- 不读取系统 proxy 环境，不实现 redirect 或 CONNECT；
- 不接受 Authorization/header 注入面；
- request 使用 strict JSON，response 拒绝压缩，200 必须是 `text/event-stream`；
- request 和 response 都有固定 byte limit。

`http.server` 只作为测试 fixture，不作为生产 server 或安全边界。

## 3. 时间与取消

`TransportControl` 冻结：

- connect timeout；
- 从 request 发出到首个非空、非 SSE comment 行的 first-semantic timeout；
- total timeout；
- response byte limit；
- cooperative cancellation event。

空行和 `: keep-alive` 不满足 first-semantic。取消在 connect 后和每次 response line 之间检查；关闭 response/connection 是最终清理。Python 标准库的同步 HTTP API 不能保证在任意内核阻塞点即时抢占，因此 V0 只宣称 bounded/cooperative cancellation，不宣称 hard real-time cancellation。

## 4. Mock server

- 绑定 `127.0.0.1:0`，由 OS 选择临时端口；
- 每个 exchange 预先冻结 status、SSE lines 和 delay；
- 只接受 POST `/chat/completions` 和有限 Content-Length；
- request 只保存在测试进程内存，用于确认第二轮实际携带 thinking continuity；
- client 取消或 timeout 引发的 broken/reset/aborted connection 被视为预期结束，不打印 server traceback。

## 5. Windows DPAPI 存储

`provider-private-transcript.dpapi` 使用：

- `CryptProtectData` / `CryptUnprotectData`；
- 当前 Windows 用户作用域，不设置 `CRYPTPROTECT_LOCAL_MACHINE`；
- `CRYPTPROTECT_UI_FORBIDDEN`，禁止任何交互提示；
- 固定非秘密 optional entropy，区分本项目存储用途；
- 文件 magic/version header；
- DPAPI 保护的 envelope 内另含 payload SHA-256；
- 临时文件 + `os.replace` 原子更新；
- run ID、payload shape、message count 和 application digest 在恢复时复核。

DPAPI 通常把解密能力绑定到同一用户和同一机器，并提供 MAC 完整性检查；本项目仍增加应用层 digest，且不使用 machine scope。管理员重置密码、用户配置损坏或机器迁移可能导致历史 transcript 无法恢复，因此 journal 必须把这种情况记录为 degraded/failed，不能伪造恢复成功。

Python 对象可能在 GC 前保留 plaintext 副本，V0 不宣称内存零化。密文也不是跨用户共享或备份格式；跨机器需求以后应使用明确密钥管理或 DPAPI-NG，而不是降低为 machine scope。

## 6. Model-loop 集成

`deepseek-loopback-http-smoke`：

1. 启动临时 loopback mock server；
2. 通过 `LoopbackHttpTransport` 完成两次真实 HTTP POST/SSE 读取；
3. Windows 上在第一轮 tool result 后原子写入 DPAPI transcript；
4. 第二轮完成后覆盖为五条消息的最终加密快照；
5. journal 只登记 ciphertext digest，不登记 raw reasoning；
6. verifier 在 Windows 上实际解密、核对 run ID、message count 和 digest；
7. 关闭 server，不留下监听端口。

Ubuntu CI 会执行 loopback HTTP、timeout 和 cancellation 测试，但明确不伪装 DPAPI；DPAPI 往返和 tamper 测试只在 Windows 执行。

## 7. 官方来源

- [Python http.client](https://docs.python.org/3/library/http.client.html)
- [Python http.server](https://docs.python.org/3/library/http.server.html)
- [Python threading.Event](https://docs.python.org/3/library/threading.html#threading.Event)
- [Microsoft CryptProtectData](https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptprotectdata)
- [Microsoft CryptUnprotectData](https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptunprotectdata)
- [Microsoft DPAPI example and scope notes](https://learn.microsoft.com/en-us/windows/win32/seccrypto/example-c-program-using-cryptprotectdata)
