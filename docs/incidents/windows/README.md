# Windows 事故台账（追加式）

本目录接收 Windows 运行时事故与未闭合限制的原始登记。记录允许存在 `unknown`，
保留观察环境、复现条件、证据位置、归因状态和后续处置；不得用尚未闭合的事故
记录直接推广平台成熟度结论（ADR-0010 §6/§11.7）。

## 未闭合限制（open limitations）

### WIN-LIM-001 Windows Native Sandbox raw TCP 残留

- **状态**：open limitation
- **观察**：AppContainer 沙箱经 elevated netsh 防火墙可达 compliant 状态，但
  raw TCP 残留；host firewall 补偿不能被重写为 AppContainer 自身完成网络隔离。
- **来源证据**：`docs/GAK_SBX_001_WINDOWS_NATIVE_SANDBOX_AUDIT_2026-07-27.md`
- **归因**：未闭合；live probe 通过前 selection 保持 noncompliant。

### WIN-INC-001 Observer 子进程泄漏

- **状态**：incident，保留在事故路由（ADR-0010 §11.7：待 raw artifact identity 和
  结构化 verification 补齐后再晋级，不能只凭修复说明与测试总数进入案例库）
- **来源证据**：`docs/GROK_OBSERVER_SUBPROCESS_LEAK_2026-07-30.md`
- **后续**：补齐 raw artifact identity + 结构化 verification 后重新评估晋级。

## 晋级规则

事故 → 精选案例需满足 ADR-0010 §11.7 门槛：明确 fixture、result/verification
Schema、baseline/candidate 对照、digest 和零残留检查；整理时不得合并模糊案例。
