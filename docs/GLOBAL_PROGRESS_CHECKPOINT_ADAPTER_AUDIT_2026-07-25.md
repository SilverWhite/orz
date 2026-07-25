# Global Progress 来源绑定 checkpoint adapter 审计（2026-07-25）

状态：no-model、fixture-only、通用 agent control-plane。未调用真实模型，未读取 LIF 研究工作区。

## 1. 解决的缺口

跨检查点 holistic gate 先前可以验证 history hash chain，但 checkpoint 字段仍可能由调用方自行填写；有界聚焦中的
`max_additional_actions` 也只是声明，没有下一周期的实际消费核算。本轮增加来源绑定 adapter，把 checkpoint 变成
下列已登记事实的确定性投影：

1. 原始 GPS input（task contract、plan、journal）；
2. 确定性 GPS review；
3. 与 review digest 绑定的 disposition；
4. 既有独立 verification report；
5. task/contract-bound critical-direction policy；
6. 第二周期起的前一 checkpoint；
7. 有聚焦授权时的 prior history、holistic review/disposition/verification。

adapter 会重新运行既有 GPS/holistic 独立 verifier，并要求重算报告与提供的 verification report 完全一致。因此
仅把 JSON 中的 `valid` 改成 `true`，或同步伪造若干 digest，不能成为 checkpoint 来源。

## 2. 派生规则

- `direction_action_counts`：只统计前一 journal head 之后的 `action_terminal`，成功、失败、取消都消耗动作预算；
- `novel_evidence_refs`：来自同一增量 span 内的 `artifact_registered` event ID；
- critical direction/acceptance：只能来自冻结 policy，并且必须存在于当前 plan/task contract；
- critical acceptance state、unresolved constraint：从已验证 review 投影；
- selected direction：由 disposition 的 selected step 映射，不能直接填 direction；
- `previous_checkpoint_sha256`：形成跨周期 hash chain；
- policy digest：第二周期起必须与前一 checkpoint 一致，防止静默移除关键方向。

当前没有 policy revision protocol；确需改变 critical scope 时必须先停下并显式设计 revision artifact，不能通过换
policy 文件绕过。

## 3. 有界聚焦核算

adapter 要求 focus history 的最后一个 checkpoint 与 `--previous-checkpoint` 完全一致，assessment window 也必须终止
于该 checkpoint。随后对授权方向统计增量 action：

- `observed_actions <= max_additional_actions` 且当前 cycle 不晚于 `review_by_cycle`：checkpoint 与独立 verification
  均有效；
- 动作或期限超限：仍 create-new 写出带 `within_action_limit=false` 或
  `within_review_deadline=false` 的 checkpoint，保留审计事实；builder 返回 2，独立 verifier 也返回 invalid。

这种区分避免“因为越界所以不留记录”，也避免把结构正确误写成继续授权。

## 4. 回归结果

新增 8 个 unittest：

1. 首 checkpoint 从五件套来源派生并通过独立重建；
2. verification digest 篡改被上游 verifier 重算拒绝；
3. checkpoint 数值即使重新计算自身 hash，仍无法通过独立重建；
4. critical-direction policy 不能跨 checkpoint 静默改变；
5. 两个追加 action 恰好达到上限时 permission reversal 仍合法；
6. 第三个追加 action 被记录并阻断；
7. failed action 同样消耗预算，不能用失败重试绕过；
8. 超过 `review_by_cycle` 即使动作数未超限也阻断。

专用回归为 8/8。完整仓库回归为 164/164：

- prototype 50；
- Grok integration 44；
- runtime 22；
- assurance 48。

repository checker 覆盖 94 个 schema、0 errors；Python compileall 通过。

## 5. 保留边界

- adapter 当前面向 GPS artifact v0.1；尚未接入真实 runtime event stream 或 artifact registry。
- policy 只支持冻结，不支持带授权、理由和前后差异的显式 revision。
- action count 是离散控制代理，不等价于 token、时间、成本或方向价值。
- 超限 checkpoint 的非零退出码仍需未来 runtime transition gate 消费，当前没有真实模型回合可被停止。
- 本轮证明 provenance、重放和预算执行语义，不证明 holistic warning 能改善模型结果，也不构成科学有效性验证。

下一步应先做 disposable runtime-event integration fixture：把 adapter 放到步骤边界和 completion transition 前，
验证非零/invalid 结果确实阻止状态推进。该 fixture 仍应使用通用非 LIF 任务，不调用真实模型。
