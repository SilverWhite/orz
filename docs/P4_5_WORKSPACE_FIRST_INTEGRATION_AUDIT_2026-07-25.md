# P4.5 workspace-first integration audit — 2026-07-25

## 结论

P4.5 已完成一条 runtime-neutral、无模型、无网络、无子进程、无 Docker 的标准模式纵向切片：

`P3 provenance → action authorization → fixed in-process workspace roundtrip → P4 audit seal → P1 archive → independent verification`

标准模式的 backend 是 `workspace_guarded`，安全声明严格限制为
`logical_workspace_boundary_only`。它不是容器、VM、AppContainer 或 restricted-token 级物理隔离。
strict 模式仍要求 P2 observed-compliant Docker，禁止隐式启动/停止和静默降级。

## 后端策略

`assurance/execution-backend-policy-v0.1.json` 冻结两种模式：

| 模式 | backend | Docker | fallback | 声明 |
| --- | --- | --- | --- | --- |
| standard | `workspace_guarded` | 不需要 | 不适用 | 仅逻辑工作区边界 |
| strict | `docker_strict` | 显式、按需、外部管理 | 禁止退回 standard | 必须有 observed profile |

本策略针对已重复观察到的 Docker Desktop 长时间开启后 CPU 异常占用：日常任务不再维持 Docker
常驻，保障内核也不接管 Docker Desktop 的全局启停。需要 strict 时由用户显式启动，重新取得 P2
observation 后执行，并在完成后由用户决定是否关闭。P4.5 没有尝试修改 Docker Desktop 的全局配置，
也没有声称解决其资源异常根因。

## 固定动作与授权

工作区必须含精确 `.assurance-p45-workspace.json` marker，明确承认标准模式只有逻辑边界。
动作固定为：

- 在工作区排他创建 `.p4-5-workspace-roundtrip-probe`；
- 写入固定 payload，读取并逐字节核对；
- 在 `finally` 中删除 probe；
- 要求执行前后工作区 manifest SHA-256 相同；
- 不接受用户命令、shell string、模型输出、endpoint 或任意文件路径。

动作请求先成为 P3 structured normal candidate，所需能力精确为
`action.fixed_no_model`、`filesystem.workspace_read` 和 `filesystem.workspace_write`。只有当前 signed
envelope 允许全部能力时，kernel action authorization 才为 `allow`。

## 审计与归档

P4 ledger 记录三个完整来源事件：

1. kernel `action.authorized`；
2. runtime `action.completed`；
3. supervisor `run.terminal`，且为唯一最终 terminal。

ACP/session/provider 未接入，明确声明为 `unknown`，不把“未观察到”冒充 complete。原始授权收据、
动作结果和 lifecycle payload 进入 archive 必删类别；metadata journal、P3 retained signed receipts
和 integrated receipt 进入 `redacted_conversation`。归档后 verifier 重新验证：

- DPAPI installation key 与所有 retained HMAC；
- provenance → action authorization → audit seal 的 digest 绑定；
- audit hash chain、source counts 和 terminal exactly-once；
- archive deletion receipt 与 raw-payload absence；
- 工作区 path/marker/manifest 和 probe residue；
- `docker_invoked/process_spawned/model_invoked/network_requested` 均为 false。

## Observed run

本机运行目录（被 `.gitignore` 排除）：

`D:\CLI\.observed-runs\p4-5-workspace-first-20260725`

运行结果：

- conversation：`CONV-7CF771084937441EA4EBE3601F1EDF14`
- mode/backend：`standard / workspace_guarded`
- archive complete：`true`
- audit valid：`true`
- raw payload absence proven：`true`
- independent verification valid：`true`
- Docker/model/network/child process invoked：全部 `false`
- workspace residue：只有授权 marker，固定 probe 不存在
- 事后 Windows 进程表：无 `assurance.p45_cli` 残留进程

该实测刻意没有运行任何 Docker 命令，因此不会因“检查 Docker”反过来触发或维持 Docker backend。
入口运行在当前 Codex `workspace-write` 文件系统沙箱中；这是本次编排环境的额外限制，不写入 P4.5
receipt，也不能据此声称其他宿主运行同样拥有该物理边界。

## 验证结果

仓库合同检查：

- schemas：75
- errors：0

组合测试：

- prototype：50 passed
- Grok reference integration：44 passed
- runtime control plane：6 passed
- assurance（含 P4.5 3 项）：32 passed
- 合计：132 passed

P4.5 只增加 3 个组合测试：

1. 完整标准链、归档和无 Docker/模型/网络/子进程事实；
2. workspace-first policy 与 strict 永不静默降级；
3. 缺失 marker 和 retained receipt 篡改 fail closed。

## 边界与下一步

P4.5 是 development/conformance 纵向切片，不是通用生产沙箱。下一阶段按 gap register 进入 P5，
但先做内部合成用户任务，不直接接真实项目或凭据：

- 在 disposable repo、fake credential、network-off 条件下验证新手/熟练用户任务；
- 测试标准模式能否清楚表达逻辑边界，以及 strict 请求不可用时是否可理解地 fail closed；
- 只为测试所需动作增加封闭 tool contract，不开放任意 shell；
- 为每类测试动作定义 canonical target、capability、side-effect class 和结果 redaction；
- 保持 Grok CLI 为参考 adapter，不把它提升为强制框架。
