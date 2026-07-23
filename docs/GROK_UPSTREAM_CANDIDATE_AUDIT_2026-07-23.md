# Grok Build upstream candidate 审计（2026-07-23）

状态：`0.2.111` 已完成本地身份、ACP initialize、fake-tool allow/cancel、fake DeepSeek reasoning
continuity 与完整 repository regression；尚未提升为默认目标，Windows child-tree timeout 门禁待完成。

## 1. 发现

- 官方 stable pointer：`0.2.111`
- 公开 changelog 当时最新条目：`0.2.106`
- 官方 GitHub `main`：`a5727c5960452e7527a154b25cb5bf00cda0545e`
- `SOURCE_REV`：`30192d2eef5d91a8fff0e53957de5bd05b43398c`
- 相对 baseline source commit：ahead 2 commits

GitHub compare API 返回了上限 300 个文件，因此 `12302/2070/14372` additions/deletions/changes 只是返回集合
的部分聚合，不能描述成完整 diff。相关 surface 包括 ACP、background tasks、compaction、managed config、
permission、session persistence 与 worktree。

## 2. Windows binary identity

| 字段 | 观察值 |
|---|---|
| version | `grok 0.2.111 (94172f2aa4)` |
| bytes | `137980232` |
| MD5 | `ab3b37ddc85e8fa9c7bdb2633bdfcf19` |
| SHA-256 | `a3614df24080471709d096a2e47ce7f6c84443af1cb59b7363967858858bc9bc` |
| PE | `MZ` |
| Authenticode | `Valid`, `X.AI LLC` |
| local path | `.tools/grok/0.2.111/grok.exe`（git-ignored） |

安装器没有执行；PATH、用户 Grok 配置和 baseline binary 均未改变。源码对应关系仍为 `unverified`。

## 3. ACP initialize

最终 run：`.observed-runs/acp-initialize-0.2.111-v2`

- probe ID：`ACPINIT-b5a89923423944d2b149dbcbce53d4cb`
- result SHA-256：`64fe15b51cee2825aa8f04563ed8e86b02957173882b34d209744c71be0299fb`
- ACP protocol：`1`
- result 与独立 verification：`valid=true`
- 临时 firewall residue：0

相对 `0.2.106`，capability path 新增：

- `x_keyword_search`
- `x_semantic_search`
- `x_thread_fetch`
- `x_user_search`
- hook `stopSignals`

没有观察到既有 capability path 被删除。第一次 run 的协议和安全检查全部通过，但旧 schema 将 version 写死为
`0.2.106` 而失败；schema 现只约束语义版本形状，实际身份继续由 release receipt 的 hash/size/version 强制。

## 4. Fake tool 双场景

| 场景 | run | probe ID | result SHA-256 |
|---|---|---|---|
| allow | `acp-fake-tool-allow-0.2.111-v2` | `ACPTOOL-553c2985703341309f43053b3529be3d` | `75124534e90e0809492fb0cde9efca43019f6cc62f61226ef383cdfa7d50c274` |
| cancel | `acp-fake-tool-cancel-0.2.111-v1` | `ACPTOOL-3d96929956634c4fa40e9acf1972a657` | `2fc1abb68a08705ad22121c4e395f4cef6471401c9b63ab43a807748e17f13ee` |

两次 result 与独立 verifier 均 `valid=true`，`unexpected_message_count=0`，fake provider 仅绑定 loopback，
`real_model_invoked=false`，临时 firewall residue 为 0。

allow 保持 completed terminal、2 次 primary request、reasoning/tool ID/result continuity 和 1 次
session `tool_completed`。cancel 表现为唯一 failed terminal、1 次 primary request 和 0 次
session `tool_completed`。

## 5. 兼容差异

`0.2.111` 允许 `_x.ai/mcp_initialized` 在 `session/new` response 之前到达，并给 running queue notification
增加 `runningKind`/`runningText`。旧探针把响应顺序写死，第一次 allow run 因此 fail closed。修复后：

- early notification 暂存 session ID，并强制与 `session/new` result 相同；
- queue 同时接受旧形状和新增字段的精确形状；
- 没有把任意扩展字段或任意乱序通知加入宽泛白名单。

这属于 adapter 兼容性修复，不修改 ACP/LIF 协议语义。

## 6. 当前裁决

`0.2.111` 是优先候选，但还不是默认版本。它已经证明比 baseline 多出可用 capability，且核心 fake-only
ACP/DeepSeek continuity 未退化。仓库检查为 42 schemas、40 cases、0 errors；44 个 prototype、31 个
Grok integration 和 6 个 runtime tests 通过。下一门禁是 Windows child-tree timeout；通过后再决定是否签发
独立 promotion 提交。
