# Grok workspace trust/discovery 审计（2026-07-21）

状态：Windows 本地、无模型、无 credential 的 `grok inspect` observation；不修改 protocol v0.1 语义。

## 1. 目的与边界

锁定的 Grok `0.2.106 (bde89716f6)` 文档同时说明了两件事：

- project hook、repo-local MCP/LSP 与 project plugin code 由 unified folder-trust 保护；
- project permission/config 可在没有独立 trust prompt 时参与配置合并。

本轮使用 checked-in inert workspace fixture 和 disposable Git workspace，确认 `projectTrusted=false` 时
`grok inspect --json` 实际报告哪些控制面。只运行 `inspect` 子命令；没有启动模型、tool session、hook command
或真实网络 transport。

## 2. 实现产物

[`new_grok_workspace_trust_receipt.ps1`](../scripts/new_grok_workspace_trust_receipt.ps1) 在任何 Grok 进程前
静态扫描 repo root 到 cwd 的候选控制面：instruction/rules、`.grok/config.toml`、Claude/Cursor settings、
hook、plugin、skill、agent 与 MCP/LSP 定义。它：

- 只记录相对路径、类别、能力等级、长度和 SHA-256，不保存正文；
- 不跟随 reparse point，并对候选数量/总字节设置上限；
- 默认 `restricted`；发现任一候选即 `launch_permitted=false`；
- `trusted` 必须提供当前 aggregate SHA-256 和非默认 actor；文件变化使旧 digest 失效；
- 不写 `trusted_folders.toml`，不冒充 Grok trust grant。

对应 schema、example 和 5 个 Windows 单元测试位于 [`integration/grok`](../integration/grok/README.md)。

[`invoke_grok_workspace_discovery_probe.ps1`](../scripts/invoke_grok_workspace_discovery_probe.ps1) 将 5 个 inert
control files 复制到一次性 Git workspace，先生成 restricted receipt，再用 clean child environment、隔离
profile/temp 与 fail-closed proxy 运行普通 inspect 和 `--trust` inspect 对照。

## 3. Observation-first 失败链

| 版本 | 直接观察 | 处置 |
|---|---|---|
| v1–v2 | 当前 Windows 环境同时含大小写不同的 `Path`/`PATH`，.NET Framework 惰性复制父环境失败 | 不读取父环境字典；显式构造大小写不敏感的最小 child environment |
| v3 | 嵌套 source fixture 没有独立 project root；inspect 只返回路径/计数，未返回正文 marker | probe 复制 fixture 并 `git init`；按返回路径和计数判断 |
| v4 | 空 hook array 的 JSON 投影为 null，marker 检查调用失败 | 显式把空投影归一为 `''`，不推测 hook |
| v5 | 核心 discovery 观察成功，但错误地把 `inspect --trust` 当成 trust grant，result 因该期望为 false | 把第二次运行重新分类为 non-mutating trust-flag comparison |
| v6 | 全部机械 checks 通过，`valid=true` | 作为当前 observed baseline |

这些修复只处理 probe mechanics，没有为了迎合期望修改 Grok 输出或 fixture 语义。

## 4. v6 最终观察

- run ID：`DISCOVERY-007f45fdb5044f68bb217b55c465d6b6`
- result SHA-256：`b13030d8d52256b780154423464e19ae8d417d77718c311bd18b00573711cbf0`
- static candidate count：`5`
- static aggregate：`a84d4674541b559ff710ea69789d4604cec7195913fd2d59e081659be3b8d2bc`
- restricted receipt：`launch_permitted=false`
- ordinary inspect 与 `inspect --trust` stdout SHA-256 均为
  `1270d2e6155662e21f885d95338c0fbaa3f8c27ad82742f65dc845507afd08e3`
- 两次 stderr 均为空文件 SHA-256
  `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`

`projectTrusted=false` 时的直接计数：

| surface | ordinary inspect | `--trust` inspect | 结论边界 |
|---|---:|---:|---|
| project instructions | 1 | 1 | 未信任时已进入 discovery metadata |
| project skills | 1 | 1 | 未信任时已进入 discovery metadata |
| permission/config sources | 2 | 2 | `.grok/config.toml` 与 `.claude/settings.json` 均被加载/列出 |
| project hooks | 0 | 0 | 与文档中的 folder-trust gate 一致 |
| `projectTrusted` | false | false | `inspect` 子命令上的隐藏 `--trust` 不签发 trust |

普通与 `--trust` inspect 的原始 JSON 逐字节一致。这个结果只证明 `inspect` 的 non-mutating 行为；不能外推到
真实 agent session 的 `--trust` 路径。

同一脚本随后对当前仓库根 `D:\CLI` 做 restricted preflight：candidate count `0`、aggregate 为标准空内容
SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`、
`launch_permitted=true`；receipt SHA-256 为
`3d12220320b56ce53b31e61d81b0e6b6571f0d0f118cee67f09afb6d8f2419f6`。该结论只对扫描时的 root-to-cwd
控制面成立；新增或修改候选文件后必须重新签发。

## 5. 机械验证与限制

- trust receipt schema、discovery result schema 和实际 v6 `result.json` 均通过 Draft 2020-12 + format check；
- 5 个 trust 单元测试通过：空 restricted、控制面 fail closed、digest-bound trusted、变更失效、no-overwrite；
- PowerShell AST syntax、仓库检查和后续全回归是提交前 gate；
- `fep-script-validation` 的 script board 只报告 `.ps1` 不是它支持的 `.py`，因此没有用该结果替代 PowerShell
  parser；
- result artifact board：JSON/sidecar/finite PASS，artifact shape WARN；已由本仓库专用 schema 和逐字段人工审计
  补充，但这仍不是安全或科学证明；
- fail-closed proxy 已配置，但没有 packet capture/防火墙计量，不能声称机械证明零网络 attempt；
- inspect 进程有 timeout，但本 probe 尚未接入 Job Object；
- preflight 当前尚未成为 fake-provider launcher 的强制入口，这是下一小步。

## 6. 下一步

1. 在 fake-provider launcher 创建 workspace 后、启动 Grok 前生成并复核 receipt；digest/decision 不匹配即不启动。
2. 为现有 fake result schema 做显式版本升级，记录 trust receipt artifact 与 checks，不原地改变旧 schema 语义。
3. 之后接 local trace/export event-completeness bridge；真实 session trust grant 另设无 provider fixture，不与
   DeepSeek 调用混在一起。
