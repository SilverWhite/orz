# Global Progress controller verifier 审计（2026-07-25）

状态：no-model、read-only verifier、disposable integration fixture。

## 可重建事实

`verify_global_progress_controller_receipt.py` 不调用 controller，也不写 journal。它重新运行 transition builder，并在共享
sidecar lock 内交叉检查：

- controller receipt schema；
- request/checkpoint/holistic 文件的当前 SHA-256；
- 候选 transition event 与 source binding；
- journal 的 run/manifest/sequence/hash-chain、当前文件 SHA-256 和事件数；
- 候选 event 是否唯一且位于 receipt 所声明的链尾快照；
- `state_before`、`state_after`、`transition_applied` 与 gate payload 是否一致；
- recovery/conflict/rejected 状态是否与当前机械条件一致。

## 故障注入

测试先直接把候选 event 追加到 journal，但故意不生成 controller receipt，模拟 append 成功后 controller 进程退出。
相同 request 随后经 controller 重试得到 `already_recorded`，没有重复事件；新 receipt 再由独立 verifier 验证通过。

另有两个反例：

1. receipt 的 `state_after` 被改写时，schema 仍可成立，但状态投影检查失败；
2. receipt 生成后 journal 继续追加事件时，旧 receipt 的 journal SHA-256 与事件数失效。

专用 transition/controller 测试为 17/17。全量回归为 189/189（prototype 58、Grok integration 44、
assurance 48、runtime 39）；仓库机械检查覆盖 102 个 schema，并返回 0 errors。

## 不可重建事实

只有最终 journal 与外部 receipt 时，无法独立区分“本次 controller 刚追加 event”和“controller 发现相同链尾 event 已
存在”。因此 `event_appended` 只能做一致性检查，不能升级为独立历史证明。verifier 也不证明科学正确性、模型行为或真实
CLI 生命周期。
