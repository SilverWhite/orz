# Global Progress Sentinel 跨检查点整体性门禁审计（2026-07-25）

状态：no-model、fixture-only、通用 agent control-plane。未调用 Grok/DeepSeek，未读取 LIF 研究工作区，也未把
LIF 内部任务用作复杂模型测试题。

## 1. 本轮补全

原 GPS 能从单次 plan/journal snapshot 发现方向集中、验收未覆盖和验证债务，但不能判断同一现象是否跨回看周期
持续，也不能阻止局部 PASS 被误当作整体完成。本轮增加独立的跨检查点历史层：

- hash-chain checkpoint 输入：稳定引用 prior review/disposition digest，记录 review cycle、journal head、
  显式方向 action count、证据新增、关键延期与关键 acceptance 状态；
- holistic review：派生方向预算占比、同方向无证据空转、关键延期债务和整体完成资格；
- holistic disposition：逐条处置 warning，并为合法集中推进提供有边界的 permission reversal；
- independent verifier：不导入 builder，独立重建 review，并检查历史链、artifact linkage、warning 全覆盖、
  聚焦窗口和完成门确认。

所有方向与 acceptance 来自任务计划/合同投影。实现不从文本语气猜测“投入程度”，也不把方向均匀视为正确性。

## 2. 固定 fixture 观察

四个 checkpoint 共记录 13 个已观测动作：

| direction | actions | share | latest state |
|---|---:|---:|---|
| `runtime_acp` | 12 | 92.3077% | active, critical |
| `evidence_gates` | 1 | 7.6923% | active, critical |
| `windows_containment` | 0 | 0% | deferred, critical |

机械派生结果：

1. `direction_budget_dominance`：`runtime_acp` 超过冻结的 70% 阈值，且另有 active critical direction；
2. `evidence_stagnation`：连续四个 checkpoint 选择同方向 `continue`，没有新增 evidence ref 或 newly verified
   acceptance；
3. `critical_direction_deferral_debt`：`windows_containment` 连续四个 checkpoint 被显式延期；
4. `local_pass_global_incomplete`：`ACC-ACP` 已为 `verified`，但 `ACC-EVIDENCE` 仍为 `planned`、
   `ACC-WINDOWS` 仍为 `deferred`，所以整体完成门为 `ineligible`。

这一区分刻意保留：新增证据可以终止“停滞”连续计数，但不会自动消除预算集中、延期债务或整体完成 blocker。

## 3. Permission reversal

正例仍允许继续 `runtime_acp`，但必须把聚焦窗口限制为：

- 明确 critical path 和 `ACC-ACP`；
- 最多两个追加 action；
- cycle 5 前重新回看；
- 给出退出条件；
- 显式列出被挤出的 active critical direction `evidence_gates`；
- 对整体完成记录 `completion_withheld`。

该处置通过 verifier。把 `bounded_focus` 改为 `null` 后，schema 仍合法，但语义验证 fail closed；把完成确认错误改成
`eligible` 也会独立失败。这证明系统允许有理由的短期集中，但不能一次签发无限期授权；若下一周期续期，仍会重新
触发预算、停滞与延期债务审查。

## 4. 回归与篡改

新增 8 个 unittest：

1. 四类跨检查点失效模式、预算数值和整体 blocker；
2. 有界聚焦 permission reversal 正例；
3. 无界 `reasoned_continue` 反例；
4. checkpoint action count 篡改破坏 digest/hash chain；
5. 新 evidence ref 只清除 stagnation，不隐藏 dominance；
6. `ineligible` 完成门必须精确确认 `completion_withheld`；
7. review 数值篡改即使重新绑定 disposition digest，仍无法通过独立重建；
8. 所有关键 acceptance 均 verified 且约束已清除时，完成门 permission reversal 为 `eligible`。

本轮专用结果为 8/8。完整仓库测试和 repository checker 在提交前另行运行。

## 5. 保留边界

- checkpoint 内的 prior review/disposition digest 当前是稳定 provenance reference；fixture 没有携带原 artifact，
  因此尚未证明投影字段与 prior artifact 内容一致。
- `max_additional_actions` 当前只被验证为有界声明；尚无 runtime adapter 在下一 checkpoint 核算实际消耗并拒绝
  超限续行。
- action count 是已观测的离散代理，不等于 token、时间、认知成本或方向价值；阈值必须冻结在 run manifest。
- `eligible` 只表示整体性字段无 blocker，不证明科学结论正确、安全门已通过或用户会接受结果。
- 本层尚未接入真实 runtime checkpoint adapter，也未验证提示注入对模型行为的净收益。
- 正式复杂测试任务应使用独立于 LIF 内部研究的问题集；本 fixture 只测试通用控制面机械语义。

后续状态：no-model checkpoint adapter 已在
[`GLOBAL_PROGRESS_CHECKPOINT_ADAPTER_AUDIT_2026-07-25.md`](GLOBAL_PROGRESS_CHECKPOINT_ADAPTER_AUDIT_2026-07-25.md)
中完成；它从 prior artifacts、plan/journal 与冻结 policy 机械生成记录，并实际核算有界聚焦消耗。下一门槛转为
runtime event integration 和 policy revision protocol，仍不直接调用真实模型。
