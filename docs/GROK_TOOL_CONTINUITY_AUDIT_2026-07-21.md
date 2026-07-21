# Grok tool/reasoning continuity 与 Job Object 审计（2026-07-21）

状态：Windows fake-only development conformance PASS；不含真实 DeepSeek、真实 credential、任意写工具或
科学结论。

## 1. 测试来源与固定边界

前一阶段捕获的真实 Grok request 显示，CLI `--tools Read` 在 Chat Completions body 中实际暴露为
`read_file`，required 参数是 `target_file`；同一 body 还包含 `search_tool` 与 `use_tool`。本 spike 不按 CLI
显示名猜协议，只调用实际 schema 中的 `read_file`。

fake provider 首轮返回：

- provider-private `reasoning_content` marker；
- 固定 tool call ID；
- `read_file`，目标固定为隔离 workspace 内的只读 fixture。

第二轮只有在 Grok 已执行工具后才会发生。provider 机械检查第二个 primary request 中是否同时存在原始
reasoning marker、tool call ID、`role=tool` 消息与 fixture marker，然后返回固定 final SSE。

Grok 使用上一阶段相同的 clean environment、隔离 profile/temp/workspace、临时出站防火墙和 credential
artifact 扫描。新增 kill-on-close Windows Job Object；Grok 启动后立即分配到 Job。由于使用普通
`Process.Start` 而非 suspended-create，仍明确保留一个很小的 start-to-assignment race。

## 2. Observation-first 失败链

1. 首次 run 收到首个 primary request 后，Grok 并发发起 `model=grok-4.5`、tool=`session_title` 的辅助
   请求。provider 将其误判为第二个 primary 并提前退出；主 tool continuation 随后等待至 timeout。
   Job Object 的 create/assign/close 均成功，防火墙无残留。
2. provider 改为按 request model/class 区分 primary 与 auxiliary，只以两个 `deepseek-v4-pro` 请求满足
   continuity。第二次 run 的全部协议检查通过，但最初错误地把“stdout 必须出现 tool event”设为硬门禁。
3. 实际 Grok `streaming-json` 只有 `thought`、`text`、`end`，没有显式 tool event；工具执行已由第二次
   provider request 中的 `role=tool` 直接观察。该缺口被移动到 observation 字段，而不是删除或伪装 PASS。
4. 第三次 run 使用最终 schema 与记录边界执行。

## 3. 最终观测

run：`FAKE-8a9a65f1009a433396391431a722cd1c`；session：
`59e5831d-b41e-41dc-a3a4-fa7e2be17d18`。

- Grok exit `0`，未 timeout，duration `8555.14 ms`；
- primary request `2`，auxiliary request `0`；两次均为 loopback `/chat/completions`；
- reasoning marker、tool call ID、tool result 与 fixture marker 均在第二轮保留；
- tool fixture before/after digest 相同；
- Job Object created/assigned/kill-on-close/closed 均为 true；
- `thought` 与 terminal event 出现在 streaming-json；显式 tool event 为 false；
- 60 个 run artifact 中假 credential value 与常见真实 secret pattern 均为 0 命中；
- 临时 firewall rule 最终残留 0。

artifact SHA-256：

| artifact | SHA-256 |
|---|---|
| `result.json` | `7696d75de08a98b7c6f78b5b76cb077df71d071889cd6ba6614978653a65f3b2` |
| `grok.stdout.streaming.jsonl` | `6e56c6bd049dec20a3f2570eec0bb65b2e27d803a41a442e933e070b76ff8a47` |
| `provider/provider-result.json` | `5cd5ab8e361263e097084895d9da0ae3ffbadecd1c00664834a5aa794ebd28cb` |
| `provider/requests.private.jsonl` | `a9017f9176fe3ea4a66d6b4f3cac7b386c438aed820b712a35643523c383a97b` |
| `workspace/tool-fixture.txt` | `e9804725f30165f4b02b550aab7fd154a40865c33c383afd5bd91ed649cf869c` |

实际文件保留在本机 `.observed-runs/tool-continuity-smoke-final-20260721/`，不进入 Git。

## 4. 结论边界与下一步

直接证明：在锁定 build 与固定 fake SSE 下，Grok 能把 DeepSeek-style `reasoning_content` 和 tool call
连续到第二个请求，并把一次 `read_file` 结果放入 tool message；Job Object 能被创建、分配和关闭。

没有证明：真实 DeepSeek thinking/tool 协议、真实 TLS、任意其他工具、并发工具、retry/compaction、无 race
的 suspended launch、子进程外连阻断、trace/export 完整性或科学正确性。下一步应增加 local trace/export、
workspace delta 与 hash-chain event bridge，并为 timeout 使用一个显式 child-process fixture 验证整树终止。
