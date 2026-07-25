# Global Progress journal-derived state reducer 审计（2026-07-25）

状态：no-model、disposable runtime integration、尚未接成熟 CLI session controller。

## 本轮补全

新增 `global_progress_state.py` 与 `reduce_global_progress_state.py`，从完整、已机械重放的 JSONL journal 确定性归约：

- `run_started` 锚定唯一初态 `executing`；
- 当前 task 与 controller state；
- transition 总数、实际应用数与最后 transition；
- 最近一次 gate 实际接受的 checkpoint ID、checkpoint digest 与 verification digest；
- run/manifest、初态锚点、receipt schema/语义、task continuity、state continuity 与 completed 终态检查。

`current_state` 仍保留在 transition request 中作为调用方声明与候选 event 输入，但不再具有最终决定权。专用 appender
在同一独占锁内先 replay 与归约既有 journal，再比较 candidate 的 `task_id/state_before`，append 后重新 replay 与归约。
controller 成功 receipt 的 `state_after` 直接来自锁内 reducer 结果。精确链尾幂等重试也返回归约态，而不是重新信任请求。

## 反例与回归

定向测试覆盖：

1. 仅含 `run_started` 时归约为 `executing`；
2. 合法 `executing → reviewing` 后归约为 `reviewing`，并保留最近 gate-accepted checkpoint binding；
3. `run_started` 后直接自报 `reviewing → completed` 被 controller 拒绝，journal 不写入；独立 verifier 能确认该 conflict；
4. 手工构造 hash/sequence 均成立、但第二条 transition 的 `state_before` 与链上归约态不一致时，reducer 标记 invalid；
5. 仅有 `run_preflight` 而缺少 `run_started` 时，reducer 与 appender 均 fail closed；
6. 合法 completion fixture 必须先实际记录 `executing → reviewing`，随后才允许进入 `completed`；
7. 并发 stale candidate、block event、append→receipt 崩溃窗口、torn-tail 与旧 receipt snapshot 反例继续通过。

定向 transition/controller 测试为 21/21。全量回归为 193/193（prototype 58、Grok integration 44、
assurance 48、runtime 43）；仓库机械检查覆盖 103 个 schema，返回 0 errors。

## 保留边界

- `run_started → executing` 是本版 controller state machine 的协议假设，不是从模型文本、工具执行或外部进程状态推断而来；
- “最近 gate-accepted checkpoint”只表示 transition gate 接受该 binding，不证明 checkpoint 中的科学判断正确；
- advisory lock 只约束遵守同一 sidecar lock 的本机 writer；
- reducer 不建立成熟 CLI 的 session、取消、用户授权、工具副作用或多机一致性；
- LIF 当前 INDEX/MAP/研究产物不在本独立 CLI 工作树中；本轮是通用科研 agent 控制面工程补全，不作 LIF claim promotion。

下一步才适合设计一个有限 CLI adapter：把真实 session 的启动、回合边界与终止 lifecycle 映射到已冻结的
`run_started`/journal 状态语义，并保持 adapter 只做映射、不复制第二套状态机。
