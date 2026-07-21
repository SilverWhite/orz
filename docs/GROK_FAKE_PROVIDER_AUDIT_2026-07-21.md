# Grok loopback fake-provider 审计（2026-07-21）

状态：锁定 Grok `0.2.106 (bde89716f6)` 的 Windows development conformance；没有真实 provider、真实
credential、模型费用或科学结论。

## 1. 运行边界

- Grok binary 先经长度、SHA-256、PE、Authenticode signer 与版本核验。
- custom-model profile、workspace、HOME/USERPROFILE、APPDATA/LOCALAPPDATA、TEMP/TMP 全部位于被 Git
  忽略的新 run 目录。
- Grok 子进程使用 clean environment；不继承 DeepSeek、xAI 或其他用户 credential 变量，只注入固定假值。
- 假 provider 只绑定 `127.0.0.1`，接受一次请求后退出。
- 启动 Grok 前，以管理员令牌为该 binary 路径添加临时出站 block：排除 `127.0.0.0/8` 后覆盖全部
  IPv4，并覆盖全部 IPv6；任何规则创建失败都 fail-closed。
- 所有规则在 `finally` 中按 run UUID 删除；最终复核残留数为 0。

## 2. 失败链

以下失败均发生在 development fixture 中，不是 DeepSeek 请求：

1. 非管理员启动在第一条 firewall rule 前以 Access denied 终止，`grok_started=false`；
2. 首次管理员启动发现 Windows Firewall 不接受地址范围写法，`grok_started=false`，改为 CIDR 集合；
3. pre-fix loopback 请求成功，但全目录扫描发现假 Authorization value 被 Grok `--debug-file` 明文记录；
4. wrapper 随即禁用 `--debug-file`，新增 credential-value-absent 硬检查；一次 UAC 被用户误取消，没有创建 run；
5. 用户重新授权后执行最终 post-fix run。

第 3 项证明当前锁定 build 的 debug log 不能进入普通 audit 层。它不是“因为假 key 所以可忽略”的问题；
真实 credential 会继承同一泄漏面。上游修复或启动即进入 sealed-private 层之前，wrapper 保持 debug disabled。

## 3. 最终 post-fix 观测

run：`FAKE-579cb2e74c134cdca3024cb3c977f40b`；session：
`3ff25354-dbb4-482d-8b5f-f9de3a9fae42`。

- Grok exit `0`，未 timeout，wrapper duration `8463.366 ms`；
- provider 收到恰好一个来自 `127.0.0.1` 的 `POST /chat/completions`；
- request model=`deepseek-v4-pro`，stream=`true`，message count=`4`；
- Authorization header 存在，但捕获器只保存 presence/长度/digest，不保存值；
- response marker 与 `end/EndTurn` streaming-json event 均被观察到；
- 没有观察到 tool event；这只针对该固定响应成立；
- 59 个 run artifact 中，假 credential value 命中 `0`，常见真实 secret pattern 命中 `0`；
- 临时 firewall rule 最终残留 `0`。

artifact SHA-256：

| artifact | SHA-256 |
|---|---|
| `result.json` | `ab4ab78b3c8096690719abb639548638a41df3f1627ab2e57f608a3a3a08b44a` |
| `grok.stdout.streaming.jsonl` | `6859acd518c753d4fce3ab9099e1504e034a36e6c4b5c015d0137049af559a29` |
| `provider/provider-result.json` | `bd80b54818c012044211e14e6b53e15e6b2f65a7e831fb6999fa59ea961b6839` |
| `provider/requests.private.jsonl` | `a36e38070b0908851cade17943e24bf0e8492605c65ec38a24e7f65982e287ed` |

这些路径的实际文件留在本机 `.observed-runs/fake-provider-smoke-final2-20260721/`，不进入 Git。表中 digest
是审计 locator，不使本地 artifact 成为科学证据或跨机器可用 fixture。

## 4. 已证明与未证明

本次直接证明：在上述 Windows、防火墙、binary、配置和固定 SSE 条件下，Grok custom model 会把
`base_url` 解析为 `/chat/completions`，发送 streaming request，并将固定响应转成可解析的 terminal event。

本次没有证明：真实 DeepSeek TLS/认证、thinking/reasoning continuity、tool-call continuation、retry、长上下文、
usage 准确性、Grok 子进程的网络继承、真实 prompt 的 sealed-private 存储，或任何 FEP/LIF claim。下一 spike
应增加第二轮 tool transcript 与 Windows Job Object supervisor，仍保持 fake-only。
