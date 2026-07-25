# CLI session lifecycle adapter 审计（2026-07-25）

状态：no-model、厂商无关的有限 ingress、尚未接真实 CLI 私有 event stream。

## 接入决策

现有 Grok post-run event bridge 只保证每个来源内部顺序，并明确声明
`cross_source_runtime_order=not_established`。因此本轮没有把该离线 bridge 直接转换成 canonical run journal；那会把
确定性拼接顺序误写成真实生命周期。

新增的有限 adapter 以单条、已排序 `cli-session-lifecycle-observation` 为边界。厂商专用 normalizer 负责从自己的权威
source 产生 observation；通用 adapter 不读取 vendor-private payload，只绑定 source record SHA-256，并映射：

- session start → `run_started`；
- turn start/complete → `model_request` / `model_output`；
- session complete/fail/cancel →对应 terminal run event。

## 原子边界与状态复用

`append_cli_session_lifecycle_event.py` 与其他 journal writer 使用同一个 sidecar lock。在锁内完成 replay、CLI lifecycle
归约、source sequence/identity/turn 校验、append+fsync 与再次归约。它维护的唯一局部状态是
`new → active ↔ in_turn → terminal`，没有复制 GPS 的 executing/reviewing/completed 状态机。

非 session-start observation 会调用既有 journal-derived GPS reducer。若 task 已为 `completed`，新的 turn start
fail closed。精确相同且仍位于链尾的 observation 可幂等收敛；旧重试、ID 内容冲突、session/runtime/adapter 身份漂移、
乱序 turn 和 terminal 后追加均不写 journal。

`verify_cli_session_lifecycle_receipt.py` 只读并独立检查 observation、receipt、journal hash/count、事件唯一性、canonical
映射和 lifecycle projection。最终 journal 不能证明 event 是本次刚追加还是此前已经存在，因此 verifier 不把
`appended` 当作独立历史事实。

## 实测

定向 adapter 测试覆盖九项：

1. start、turn start、turn complete、session complete 的完整 canonical 映射与 terminal replay；
2. turn 内取消映射为 `run_cancelled` 并清除 active turn；
3. active session 失败映射为 `run_failed`；
4. 无 turn start 的 turn complete 被拒绝且 journal 不变；
5. 精确链尾重试幂等，journal 推进后的旧重试 conflict；
6. runtime session identity 漂移被拒绝；
7. GPS journal-derived `completed` 后禁止新 turn；
8. `run_preflight` 可合法先于 `session_started`；
9. verifier 检测 receipt 状态篡改与 journal snapshot 推进。

定向测试为 9/9。全量回归为 202/202（prototype 58、Grok integration 44、assurance 48、runtime 52）；仓库机械检查
覆盖 107 个 schema，返回 0 errors。

## 保留边界

- observation 排序与 vendor source 解释由未来的 CLI-specific normalizer 负责；当前 adapter 只验证标准化序列自身；
- source record digest 防止绑定对象静默变化，但不证明 normalizer 解释正确；
- observation 与 receipt 尚无厂商签名或进程身份认证；
- advisory lock 不提供多机 writer 共识；
- event metadata 不复制 prompt、private reasoning 或 tool content；
- 本轮没有使用 LIF 内部任务、MAP 或实验结果，机械 PASS 不构成科学研究结论。

下一步应选择一个已有单一有序 lifecycle source 的 CLI，先实现 read-only normalizer 与 fixture verifier；若只能获得多个
无法证明全序的来源，应继续保持 partial，而不是合成权威顺序。
