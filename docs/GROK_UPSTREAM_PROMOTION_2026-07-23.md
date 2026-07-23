# Grok Build `0.2.111` promotion 裁决（2026-07-23）

状态：已选择为项目默认 Grok release；旧版 `0.2.106` 继续作为历史对照与显式重放锚点。

## 裁决

将 `upstream/grok-build.lock.json` 从 `0.2.106 (bde89716f6)` 更新为
`0.2.111 (94172f2aa4)`。`upstream/grok-build.candidate.json` 保持 `status=eligible`，并登记
`selected_as_default=true`；其中的 `baseline` 字段仍记录本次 promotion 所比较的 `0.2.106`，不会被重写成
新默认值。

新默认 binary identity：

| 字段 | 值 |
|---|---|
| version | `0.2.111` |
| build ID | `94172f2aa4` |
| bytes | `137980232` |
| SHA-256 | `a3614df24080471709d096a2e47ce7f6c84443af1cb59b7363967858858bc9bc` |
| Authenticode | `Valid`, `X.AI LLC` |
| installed path | `.tools/grok/0.2.111/grok.exe` |

## 依据

promotion 前七项 gate 均为 `passed`：

- binary identity；
- ACP initialize；
- fake-tool allow；
- fake-tool cancel；
- Windows child-tree timeout、background task cancel 与 parent exit；
- DeepSeek reasoning/tool continuity；
- repository regression。

Windows candidate 三场景均由独立 verifier 重放通过，触发后 nonce process residue 与临时 firewall residue
均为零。对照版 `0.2.106` 的 task cancel 与 parent exit 通过，但 tool timeout 在已输出终态后仍未退出，
最终由外层 60 秒 timeout 与 kill-on-close Job 收束。完整 run digest 与失败链见
[`GROK_WINDOWS_CHILD_TREE_PROBE_2026-07-23.md`](GROK_WINDOWS_CHILD_TREE_PROBE_2026-07-23.md)。

## 工程变化

- 未显式传入 release metadata 的 launcher 与 inspection 默认读取新的 checked-in lock；
- ACP initialize probe 不再硬编码版本路径，而是从所选 release metadata 的 `installed_path` 解析 binary；
- repository checker 在 candidate 尚未选择时要求 baseline 与 lock 一致，在选择后要求 candidate release 与
  lock 一致，同时继续要求候选版本高于 receipt 中的历史 baseline、全部 gate 已通过；
- 不修改 PATH、用户级 Grok 配置、登录状态、LIF protocol、reason code、gate 或 claim 语义。

## Promotion 后验证

- 默认 `inspect_grok_install.ps1` 从 checked-in lock 解析到
  `.tools/grok/0.2.111/grok.exe`，version、长度、SHA-256、Authenticode 与签名者检查全部通过；
- repository checker：44 个 Schema、40 个 regression case，0 error；
- Python 回归：prototype 44、integration 36、runtime 6，共 86 项通过；
- 10 个 PowerShell 脚本 AST parse 与 Python compileall 通过。

## 回退

若后续出现回归，不覆盖或删除任何 binary，也不改写本次 observed artifacts。回退应使用新的独立提交恢复
上一个 `upstream/grok-build.lock.json` revision 中的 `0.2.106` identity，并把 candidate receipt 的
`selected_as_default` 改回 `false`，同时登记触发回退的具体证据。历史 `0.2.106` binary 仍位于
`.tools/grok/0.2.106/grok.exe`，但使用前仍须按 lock/receipt 重新核验摘要与签名。
