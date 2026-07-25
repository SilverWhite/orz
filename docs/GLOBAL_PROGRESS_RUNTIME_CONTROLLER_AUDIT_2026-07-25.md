# Global Progress disposable runtime controller 审计（2026-07-25）

状态：no-model、disposable controller adapter、未接成熟 CLI 核心。

## 组合边界

`run_global_progress_controller.py` 复用既有 transition builder、共享 journal lock、strict appender 与 recovery inspector。
它没有新增模型判断，也不把 controller receipt 当作状态来源；状态只来自成功记录或已存在的
`global-progress-transition-receipt-v0.1` event payload。

执行结果分为：

- `transition_applied`：pass event 首次入链，状态推进；
- `transition_blocked`：block event 首次入链，状态保持；
- `already_recorded`：相同 event ID 与 hash 已存在且仍为链尾，不重复追加；若链已继续推进则返回 conflict，避免把旧
  `state_after` 误报为当前状态；
- `recovery_required`：journal 只有可证明的 torn tail，保持原字节并要求显式恢复；
- `journal_unrecoverable`：历史链不可机械恢复；
- `conflict`：陈旧链头、不同内容复用 event ID、terminal 后追加或锁竞争；
- `rejected`：transition request 或 source binding 在构建阶段无效。

## 已验证边界

专用 transition/controller 测试覆盖十五项，其中 controller 新增六项：

1. step-boundary pass 只追加一次，链尾精确重试幂等，journal 推进后的旧重试转为 conflict；
2. checkpoint block event 入链但状态不前移；
3. torn journal 返回恢复需求且 journal 字节不变；
4. 陈旧 sequence/head 返回 conflict；
5. eligible completion 推进到 `completed`；
6. checkpoint source mismatch 在任何 journal 写入前 rejected。

全量回归为 187/187（prototype 58、Grok integration 44、assurance 48、runtime 37）；仓库机械检查覆盖
101 个 schema，并返回 0 errors。

## 限制

controller receipt 在 event append 之后写出，因此两者之间仍有进程崩溃窗口；相同 request 重试可借助 event hash 收敛，
但外部 receipt 本身不是事务日志。fixture 没有接入真实 session state、模型回合、工具执行、用户授权或成熟 CLI 的取消/
恢复生命周期，也不处理多机 writer。机械 PASS 不证明科学任务正确或完成。

后续自证与 append→receipt 崩溃窗口故障注入见
[`GLOBAL_PROGRESS_CONTROLLER_VERIFIER_AUDIT_2026-07-25.md`](GLOBAL_PROGRESS_CONTROLLER_VERIFIER_AUDIT_2026-07-25.md)。
