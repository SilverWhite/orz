# Action Kernel implementation contract v0.1

状态：development-only integration spike；复用 protocol v0.1，不增加或改写协议状态、gate 或 reason-code 语义。

## 1. 目的

本 spike 把四个已经独立验证的部件串成一条最小执行链：

1. immutable no-model `RunManifest`；
2. append-only、SHA-256 链式 `RunEvent` journal；
3. Windows Job Object process runner；
4. `SessionRecord` 及其机械语义校验器。

它只回答“一个本地 action 是否被可回放地执行和记录”。它不调用模型，不判断科学产物，不产生 EvidenceRecord 或 ClaimCandidate，也不是 OS sandbox。

普通工程 smoke 不依赖旧研究工作区的 INDEX/MAP/self-check。出现 LIF claim-bearing action 时，必须另行恢复当前来源路由，不能把本 smoke 当作研究证据。

## 2. 固定执行顺序

```text
freeze manifest
  -> run_preflight
  -> run_started
  -> tool_proposal
  -> permission_decision
  -> tool_started
  -> Windows process result
  -> tool_completed
  -> session snapshot
  -> artifact_registered
  -> exactly one run terminal event
  -> replay and cross-file verification
```

不得因为进程 exit code 为 0 而跳过 artifact/session/journal 校验；不得因为 action 失败或超时而省略 terminal event。

## 3. 权限边界

- CLI v0.1 只暴露固定的 `action-kernel-smoke`，其 executable 是当前 Python 解释器；
- 不接受 shell 字符串，不公开任意 `--command`；
- manifest 的 tool allowlist 固定为 `local_process_smoke`；
- filesystem profile 明示为 `custom`，因为 Job Object 只提供进程树 containment，不提供文件系统 sandbox；
- network profile 固定为 `disabled`，但本 spike 不把这一字段冒充内核级网络隔离证明；
- 原始 arguments 和 stdout/stderr 不进入 session/journal，只记录数量、digest、byte count 和状态。

## 4. 状态映射

Windows process result 到协议 action state 的映射保持一对一：

- `succeeded` -> action `succeeded` -> `run_finished`；
- `failed` -> action `failed` -> `run_failed`；
- `cancelled` -> action `cancelled` -> `run_cancelled`；
- `unknown` -> action `unknown` -> `run_invalidated`，并记录已有的 `PROC-EXIT-UNKNOWN-001`。

非零退出和 timeout 不是 kernel 结构失败。只要 session、schema、journal chain 和 artifact digest 自洽，kernel report 仍可 `valid=true`，同时保留实际 action state。

## 5. 产物

每个 output root 至少包含：

- `run-manifest.json`：运行前冻结；
- `events.jsonl`：append-only journal；
- `windows-process-result.json`：进程控制事实；
- `session-record.json`：协议派生视图；
- `action-kernel-result.json`：跨文件验证摘要；
- prompt/rules/context/redaction-policy 的固定 no-model 输入文件。

所有 JSON 使用原子写入且默认拒绝覆盖。`verify-action-kernel` 重新验证 schema、journal hash chain、session invariant、artifact producer/linkage 和文件 digest；摘要不是 journal 的替代品。

## 6. 已接受限制

- journal append 暂无多进程锁，只支持单 kernel writer；
- session 是一次派生快照，其 `journal_head` 指向生成该快照前最后一个 event，而不是后续 run terminal event；
- process start-to-Job-assignment race 继承 Windows runtime spike 的已知限制；
- permission decision 当前是固定 allowlist 的机械决定，还没有交互审批器；
- 没有模型 adapter loop、通用 tool broker、filesystem/network sandbox 或崩溃恢复。
