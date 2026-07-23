# DeepSeek one-shot 真实 development probe（2026-07-23）

状态：实现与离线测试 PASS；用户已授权一次最小真实请求及对应小额费用，但固定
Windows Credential Manager 项不存在，因此没有发出 DNS/TLS/HTTP 请求，也没有计费。

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

- 新增两个 unit test：one-shot 成功路径使用无 socket injected connection，确认 request
  count 恰为 1，并扫描结果和 ledger 不含 marker、假 key 或 confirmation token；
- real broker 与内建 in-process fake provider 在构造期互斥；
- 原有 fake-only broker 仍不能接真实 connection factory；
- `fep-script-validation` legacy task board：8 PASS、14 WARN、0 FAIL、0 catastrophic；
  WARN 主要来自通用训练/实验脚本字段不适用于固定 transport probe；
- plan artifact board：3 PASS、2 WARN、0 FAIL；项目专用 schema 和逐字段 digest
  对账承担其非表格 artifact 的验证。

## 3. 本机固定计划

本地忽略目录：

```text
.observed-runs/deepseek-real-development-plan-20260723/
```

- request body：291 bytes；
- request SHA-256：`33ad318cf496285c101e36fc1332a89a6b9dd796c63ae08c92cb528ae9d5b533`；
- confirmation summary SHA-256：
  `44b7d362c67941b264a346c3d1c5adfd7c47cea9815eb13eabd04a5faf809d87`；
- endpoint/model：固定 DeepSeek chat endpoint / `deepseek-v4-pro`；
- max tokens / tools / retries：16 / 0 / 0；
- plan 中 `network_attempted=false`、`credential_read=false`、
  `billable_request_made=false`。

## 4. 当前 blocker

通过项目的 `WindowsCredentialManagerProvider` 只做存在性预检，系统返回 WinError
1168（找不到固定 target）。预检没有输出、散列或记录 key，也没有尝试环境变量、
配置文件或命令行 secret fallback。

因此本阶段没有把缺失 credential 误写成 provider 失败，也没有消耗实际 permit。
下一步只有一个：由用户在 Windows Credential Manager 安全建立 Generic Credential
`FEP-Agent/DeepSeek` 后，基于全新计划执行一次明确确认的 probe。不能复用环境变量
或把 key 放入 Git、shell history、命令行参数。

## 5. 结论边界

直接证明的是：固定请求可被离线计划、摘要绑定和独立测试；代码路径能在测试 connection
上执行一次并生成脱敏结果。没有证明真实 DNS、证书链、账户认证、余额、模型可用性、
SSE shape 或 provider marker。即使后续真实 probe 成功，也只证明当次 development
transport conformance，不构成 Agent 质量、evaluation、holdout 或 FEP/LIF claim。
