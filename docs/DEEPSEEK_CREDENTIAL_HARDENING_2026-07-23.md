# DeepSeek credential 边界加固（2026-07-23）

状态：offline implementation/test PASS；本阶段没有读取 Windows Credential Manager，
没有访问 DeepSeek endpoint，也没有产生模型调用或费用。

## 1. 目标

API authentication 无法做到“完全不读 key”：本地 transport 必须短暂读取 key 并在
TLS 内发送 Authorization header，provider 认证层也必须处理它。本阶段的可验证目标是：

- 只在明确批准的 execute 阶段读取；
- 只发送到固定 `api.deepseek.com:443/chat/completions`；
- 不通过环境 proxy、不 follow redirect、不启用 HTTP debug；
- 不把 key 放进 prompt、result、failure、ledger、异常正文或普通 metadata；
- 减少崩溃报告与 artifact 落盘泄露；
- 保留不能绝对保证的边界，不把测试 PASS 写成密码学零化证明。

## 2. 新增控制

### 2.1 当前短进程的 crash-reporting 边界

`configure_secret_process_security()` 在 credential access 前：

1. 对当前一次性 Python CLI 调用 `WerSetFlags(WER_FAULT_REPORTING_FLAG_NOHEAP)`；
2. 用 `WerGetFlags` 读回并验证该 bit；
3. 关闭并确认 Python `faulthandler`；
4. 任一步失败都在读取 credential、签发 permit 或联网前 fail closed。

该设置只作用于当前短生命周期进程，不修改系统全局 WER 策略。

### 2.2 固定 transport

真实 transport 继续直接构造 `HTTPSConnection`：

- host/port/path 固定；
- 系统信任库、hostname verification、TLS minimum 1.2；
- proxy environment unused；
- redirects followed=false；
- HTTP debug output=false；
- retry budget=0；
- Authorization local header map 在 request 后立即 clear。

### 2.3 成功与失败 artifact

成功保留 `result.json`。任何已进入批准链的 `PrototypeError` 现在写入
`failure.json`，只记录：

- allow/deny 与 confirmation digest；
- authorized attempt count 和 retry count；
- provider receipt / billing=`unknown` 或 `not_attempted`；
- sanitized category、stage、exception type、numeric errno；
- `raw_exception_recorded=false`。

failure 不记录异常正文，因此底层异常即使意外带入 credential，也不会进入 artifact。

### 2.4 不读取真实 key 的模式扫描

terminal artifact 原子写入前，对 output root 内所有现有 regular files 和 pending JSON
执行 bounded scan：

- 最多 32 files / 4 MiB；
- 拒绝 symlink/reparse/non-regular artifact；
- 检查常见 `sk-...` 与 Bearer credential 形状；
- 命中只报告 pattern ID，不报告匹配文本；
- receipt 固定 `actual_credential_read_for_scan=false`、`hit_count=0`。

这避免为了“证明没泄漏”再读取一次真实 key。代价是非标准 key 形状可能不被模式扫描
覆盖，因此它是防御层，不是绝对证明。

### 2.5 减少本地 immutable copy

Windows provider 不再使用 `ctypes.string_at -> bytes -> UTF-16 str -> ASCII bytes`
链。现在直接从 `CredReadW` 返回的 UTF-16LE buffer 逐 code unit 验证并写入一个
owned `bytearray`；`CredentialLease` 接管同一 buffer，退出时原地覆盖。生成
Authorization 时仍不可避免地产生一个 Python string 和 HTTP/内核副本，但删除了读取
阶段的三个额外 immutable copy。Windows 返回块仍在 finally 中 `CredFree`。

## 3. 离线验证

- injected success：恰好一次 request，成功 artifact 无 fake key/marker/token；
- injected transport `OSError`：生成 `failure.json`，只保留 stage/type，异常正文与
  fake key 均不存在；
- deliberate `sk-...` artifact：scan fail closed，错误只含 pattern ID；
- Windows WER：`NOHEAP` 设置与 read-back verification PASS；
- owned credential bytearray：lease close 后原 buffer 全零；
- endpoint/proxy/redirect/debug metadata 均为固定安全值；
- repository schema 新增 failure schema，并继续由 deterministic repository check 覆盖。

## 4. 仍不能绝对覆盖

- Python immutable string、HTTP 库和内核 socket buffer 副本；
- Windows pagefile、hibernation、管理员/debugger、恶意软件与外部 dump 工具；
- provider API gateway/认证系统对 Authorization 的必要处理；
- 硬件、内核、TLS trust store 或 provider compromise；
- 非标准 secret 格式未命中 pattern scan；
- 用户或外部 shell 主动重定向 stdout/stderr 到不受本项目控制的位置。

因此正确表述是“额外泄露面被显著缩小并机械验证”，不是“key 绝对不会被读取或残留”。
一次新授权的 real conformance 已在 `2026-07-23` 成功。正常使用前剩余正式 Grok
launcher 的同等级 credential injection、session/tool/permission/sandbox 与 leak
验证。
