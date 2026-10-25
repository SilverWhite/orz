# `protocol/` 状态与权威边界

> 更新：2026-10-25。本文是目录导航与状态声明，不自行产生协议语义。若正文、ADR、生产源码和测试冲突，按仓库治理层级裁决；不得仅因文件位于 `protocol/` 就推定为 ORZ 生产规范。

## 文件状态

| 文件 | 规范性与状态 | 已知消费者 | 与 ORZ 生产面的关系 |
|---|---|---|---|
| `PROTOCOL_DRAFT_v0.1.md` / `agent-protocol-v0.1.schema.json` | **非 ORZ 生产规范；Draft** | 设计文档引用；schema 供设计/校验材料使用 | 未由 Rust Agent loop、Host 或 journal 生产链直接消费。当前生产设计以 ADR-0010/0011、run-event schema/registry 与源码测试为准 |
| `reason-codes-v0.1.yaml` / `gate-matrix-v0.1.yaml` | **冻结的保障/评测设计候选**，不是 ORZ runtime 控制协议 | `assurance/` 的 evidence、case-retrieval、preflight/evaluation 路径及 `scripts/check_repository.py` | Python 保障与评测链会读取或固定摘要；ORZ Rust 生产执行链不据此决定工具权限、会话状态或 run 终态 |
| `global-progress-reason-code-migration-v0.1.{yaml,schema.json}` | **design-only migration** | `scripts/check_repository.py` 与 Global Progress Sentinel 设计 | 只描述本地 sentinel code 到冻结 reason code 的投影；候选 code 不因此进入现行 reason registry，也不成为 ORZ runtime 协议 |
| `structured-operation-protocol-v0.1.md` 及两个 schema | **非生产基线；Draft/reference** | `assurance/ops_executor.py`、`scripts/ops_executor.ps1`、桥脚本等参考/实验路径 | 2026-08-13 已裁决“不按原样生产接线”；其平行执行层被否决。删除安全、跨环境桥接等想法仅可作为后续裁剪输入 |

## 生产权威入口

- Agent/runtime、工具面、终态、上下文与会话语义：[`../adr/ADR-0010-fusion-runtime-and-agent-architecture.md`](../adr/ADR-0010-fusion-runtime-and-agent-architecture.md) 及 [`../architecture/current/`](../architecture/current/)。
- 动作授权与票据：[`../adr/ADR-0011-authenticated-control-and-action-fabric.md`](../adr/ADR-0011-authenticated-control-and-action-fabric.md)。
- 事件线格式：[`../runtime/run-event-v0.2.schema.json`](../runtime/run-event-v0.2.schema.json)、[`../runtime/run-event-payload-registry-v0.1.json`](../runtime/run-event-payload-registry-v0.1.json) 及同目录的 payload schemas，以及 Rust verifier 与契约测试。
- 实际可执行行为最终必须由当前源码与通过的测试证明；审计档和本目录草案不能覆盖实现事实。

## 取代与保留关系

`structured-operation-protocol-v0.1.md` 没有被另一份同形协议整体取代；准确关系是：**原方案不进入生产，正文保留为参考输入**。其授权部分由 ADR-0011/ACAF 承担，生产动作执行由 Rust 工具面承担，宿主灾难写保底与维护命令约束由当前写控设计和实现承担。若未来产出裁剪后的正式协议，应在此表登记 `superseded_by`，并同时列出生产消费者；在此之前不得把 v0.1 Draft 写成已接线能力。

`PROTOCOL_DRAFT_v0.1.md` 与机器可读 reason/gate 文件也不是互相完整取代：前者是早期总协议草案，后两者仅在 Python 保障/评测范围内有实际消费者。任何把它们提升为 ORZ runtime 规范的变更，都需要 ADR、消费者清单、迁移与符合性测试同时落地。
