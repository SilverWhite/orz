# Global Progress disposable transition gate 审计（2026-07-25）

状态：no-model、disposable integration fixture。

本轮首次把 GPS 控制结果接到状态投影，而不只生成报告。transition request 绑定 checkpoint 与 verification 文件
digest；builder 生成符合 `run-event-v0.1` 的 `gate_decision` event，payload 记录 `state_before`、
`requested_state`、`state_after`、`transition_applied` 和 runtime-local control code。

已验证八个边界：

1. 有效 checkpoint 允许 `executing → reviewing`；
2. invalid/focus-overrun verification 阻断并保持 `executing`；
3. 不允许的状态对阻断并保持原状态；
4. `reviewing → completed` 缺少 holistic artifacts 时 fail closed，且不生成事件；
5. holistic completion `ineligible` 时保持 `reviewing`；
6. holistic completion `eligible` 时才推进到 `completed`，且四件套由独立 verifier 重算；
7. event 只能追加一次到匹配 run/manifest/sequence/previous digest 的 JSONL journal，追加后通过既有 replay；
8. appender 拒绝非 `gate_decision` 或非 `global-progress-transition-receipt-v0.1.schema.json` 的候选事件。

专用测试为 8/8；全量回归为 172/172（prototype 50、Grok integration 44、assurance 48、runtime 30）。
仓库完整性检查覆盖 97 个 schema，结果为 0 errors。控制码暂不冒充 protocol reason code；event verifier 会重建
事件并检查 payload/event hash 与状态投影。

后续补全：跨进程 writer 竞争与 reason-code 迁移边界已在
[`GLOBAL_PROGRESS_ATOMIC_JOURNAL_AND_REASON_MIGRATION_AUDIT_2026-07-25.md`](GLOBAL_PROGRESS_ATOMIC_JOURNAL_AND_REASON_MIGRATION_AUDIT_2026-07-25.md)
实现和复核。本 transition gate 仍未接入真实 runtime controller；新增设计不改变其 disposable 状态。
