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

保留缺口：appender 是 disposable 单进程 fixture，没有跨进程文件锁；并发 writer 仍可能在 replay 与 append 之间
竞争。它尚未接入真实 runtime transition controller，`GPS-*` 也没有 protocol reason-code revision。下一步应先
设计原子 journal writer/锁边界与 runtime-local control code 到正式协议的迁移策略。
