# DeepSeek one-shot 真实 development probe（2026-07-23）

状态：实现、离线测试与一次 hardened real conformance PASS。固定请求得到 HTTP 200，
resolved model 和预期 marker 均匹配；恰好 1 request、0 retry。provider 控制台计数与
最终计费仍需在账户侧确认。

## 1. 固定边界

新增的两阶段入口只服务于一次 transport conformance：

1. `deepseek-real-development-plan` 离线构造固定请求和脱敏 confirmation summary；
2. `deepseek-real-development-execute` 重新构造请求并核对 summary/digest；
3. 现场输入 `ALLOW-<confirmation digest 前12位>` 后签发一次性 permit；
4. permit 只绑定 `https://api.deepseek.com/chat/completions` 和精确 request SHA-256；
5. 从当前用户 Credential Manager 固定 target `FEP-Agent/DeepSeek` 取得 key；
6. 最多发出一个 HTTP attempt，不 retry、不调用工具、不接 model loop。

请求固定为 `deepseek-v4-pro`、thinking disabled、两条无项目数据 message、stream +
usage、`max_tokens=16`。输出只允许保存 response content/reasoning 的长度与 digest、
usage、固定 marker 比对、TLS/transport metadata 和 hash-chained approval ledger。
prompt、回复正文、provider-private reasoning、confirmation token 与 Authorization 都
不落盘。

## 2. 离线验证

- one-shot 成功路径使用无 socket injected connection，确认 request
  count 恰为 1，并扫描结果和 ledger 不含 marker、假 key 或 confirmation token；
- 真实启动前发现 250ms socket poll timeout 被误当作整体 deadline；hardened attempt
  又证明 `HTTPResponse.readline()` 的 buffered reader 在一次短 timeout 后可能不能
  继续使用；最终修正为按 120 秒 first-semantic / 180 秒 total 的实际剩余期限读取，
  timeout 后不重用同一 buffered reader；
- real broker 与内建 in-process fake provider 在构造期互斥；
- 原有 fake-only broker 仍不能接真实 connection factory；
- `fep-script-validation` legacy task board：8 PASS、14 WARN、0 FAIL、0 catastrophic；
  WARN 主要来自通用训练/实验脚本字段不适用于固定 transport probe；
- plan artifact board：3 PASS、2 WARN、0 FAIL；项目专用 schema 和逐字段 digest
  对账承担其非表格 artifact 的验证。

## 3. 本机实际计划与 attempt

本地忽略目录：

```text
.observed-runs/deepseek-real-development-live-20260723/
```

- request body：291 bytes；
- request SHA-256：`33ad318cf496285c101e36fc1332a89a6b9dd796c63ae08c92cb528ae9d5b533`；
- confirmation summary SHA-256：
  `44b7d362c67941b264a346c3d1c5adfd7c47cea9815eb13eabd04a5faf809d87`；
- endpoint/model：固定 DeepSeek chat endpoint / `deepseek-v4-pro`；
- max tokens / tools / retries：16 / 0 / 0；
- plan 中 `network_attempted=false`、`credential_read=false`、
  `billable_request_made=false`。

execute 阶段重新构造并对上同一 request/summary digest，写入一条有效 allow event：

- ledger event count / allow count：1 / 1；
- terminal event SHA-256：
  `cff432db23ad01229086bb7b8214887764f61c26acc53831462d3a2b78a5070a`；
- execute outcome：`DeepSeek HTTPS transport failed (OSError)`；
- retry count：0；
- `result.json`：不存在，因为没有 provider success。

用户随后报告 DeepSeek 控制台调用次数仍为 0。该外部观察与“POST 可能未抵达”一致，
但未由本项目 API 独立读取，且控制台可能存在延迟，因此只记录为 user-observed
corroboration，不把 billing=`unknown` 改写为确定的 `not_billed`。

旧实现只保留了异常类型，没有保留失败 stage/errno，因此不能从该 attempt 追溯
connect、request、response-header 或 response-body 的精确阶段；也不能断言 provider
已收到请求或产生费用。

## 4. 独立诊断与后续保护

- 固定 Credential Manager target 在沙箱外可读取、可释放；
- key 只检查到常见 `sk-` 前缀与合理长度，未输出、散列或保存 key；
- DNS 可解析、TCP 443 成功；
- 无 HTTP、无凭据 TLS probe 成功协商 TLS 1.3 /
  `TLS_AES_128_GCM_SHA256`；
- 没有再次访问 `/chat/completions`、`/models` 或余额接口。

transport 现已把后续安全错误压缩为 `stage + exception type + numeric errno`，仍不记录
异常正文；deadline timeout 和“不得复用 timed-out buffered reader”测试通过。任何后续
provider attempt 都必须创建新计划、签发新 permit，并取得新的费用授权，不能把既有
allow 当作 retry budget。

后续 credential hardening 又要求下一次 execute 在读 key 前验证 WER `NOHEAP`、关闭
Python faulthandler；固定无 proxy/redirect/HTTP debug；无论成功或失败都生成 terminal
artifact，并在写入前对全部 output artifacts 做不读取真实 key 的 common-secret-pattern
scan。详见
[`DEEPSEEK_CREDENTIAL_HARDENING_2026-07-23.md`](DEEPSEEK_CREDENTIAL_HARDENING_2026-07-23.md)。

### 4.1 Hardened attempt 记录

新增的本地忽略目录：

```text
.observed-runs/deepseek-real-development-hardened-20260723-01/
.observed-runs/deepseek-real-development-hardened-20260723-02/
.observed-runs/deepseek-real-development-hardened-20260723-03/
```

- `-01` 在 Codex 沙箱账户 `SWITCH\CodexSandboxOnline` 内执行；该账户看不到桌面用户
  Credential Manager，因此在 credential acquire 阶段停止，没有建立 provider 连接；
- 桌面用户上下文只列出凭据 metadata，确认存在
  `LegacyGeneric:target=FEP-Agent/DeepSeek`、Generic、user=`HL`，没有显示 secret；
- `-02` 在桌面用户上下文执行，WER `NOHEAP` 与 Python faulthandler 控制均已验证；
- request/confirmation digest 与前述固定值一致，ledger 恰好 1 allow、0 retry；
- terminal failure 为 `stage=response_body`、`error_type=OSError`、无异常正文；
- 该 stage 表示 credential、connect、request、response headers 以及 headers validation
  已执行；但不能仅由客户端断言 provider 控制台计数或计费；
- terminal artifact scan 完整，3 个文件，0 命中；没有保存 Authorization、prompt、
  response body、confirmation token 或异常正文。
- 修复提交 `5ca6906` 后，`-03` 在桌面用户上下文完成一次新授权 attempt：
  HTTP 200、resolved model=`deepseek-v4-pro`、marker matched、finish reason=`stop`；
- `-03` request count / retry count=`1 / 0`，duration=`1125 ms`，response body
  `3480 bytes`，11 个 data event、9 个 semantic event；
- provider usage 回执为 prompt/completion/total=`26 / 9 / 35` tokens；该回执证明本次
  provider 调用成功，但账户控制台展示和最终计费仍属于外部状态；
- `result.json` 通过项目 Schema；通用 artifact board 为 3 PASS、2 个非表格结构
  WARN、0 FAIL、0 catastrophic；
- `-03` terminal artifact scan 同样完整，3 个文件、0 命中；raw content、raw
  reasoning 和 Authorization 均未记录。

## 5. 结论边界

直接证明的是：固定请求可被离线计划、摘要绑定；真实 credential boundary、DNS/TCP/TLS、
账户认证、固定模型、SSE 解析和 provider marker 在这一次请求上成功；allow permit
恰好消费一次且没有 retry。它不证明长期可靠性、限流/余额状态、通用 prompt、tool loop、
Agent 质量、evaluation、holdout 或任何 FEP/LIF claim。
