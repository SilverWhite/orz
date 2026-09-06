# P2-10 阶段 2 I1 实施记录（T̂ 估计器 + LIF 时间特征计算器）

> 日期：2026-08-30；切片：I1（TODO P2-10 / BACKLOG 10）；依赖：F3（已定稿）。
> 状态：S1 代码 + S2 测试完成，102 runs 离线复验已跑（正式四对照门与聚类对照属
> 阶段 3 V2，本记录只做口径复验与锚点核对）。

## 1. 实现落点

- 纯计算模块：[`orz/crates/orz-assurance/src/lif/`](../../orz/crates/orz-assurance/src/lif/mod.rs)
  - `estimator.rs` — T̂ 在线估计器（16–32 有界窗中位数、>300s 截尾、T̂₀=8s、
    ≥8 采样启用、钳制 [3,600]s，§4.5）；
  - `channels.rs` — 一阶通道 err/stall/slow/deny/prog + 二阶 stuck 闭式解
    （expm1_ratio，α/β 零除防护，§4.2–4.4）+ 峰值 θ 比跟踪；
  - `temporal.rs` — TemporalRecord / Domain 语义谓词 / DomainSpike / Migration
    （Recovery）/ Now/Recent/History/Feature 查询面（有界窗，§3）；
  - `mod.rs` — LifEngine（事件驱动编排、决策轮校准、stall 间隔探测、err τ 模式
    固定时间 vs 轮语义反事实）。
- 离线复验 harness：[`orz/crates/orz-assurance/examples/lif_replay.rs`](../../orz/crates/orz-assurance/examples/lif_replay.rs)
  （递归扫描 events.jsonl、逐 run 回放、C1 十次种子置换、聚合报告）。

## 2. 测试与静态门

- `cargo test -p orz-assurance --lib`：167 passed（新增 lif 测试 23 项，含闭式解
  与 0.05s ODE 参考对照 ≤0.006、右端点离散化伪迹 ≥2× 分离、不应期/复位/域网格/
  err10/succ10 窗口/恢复重建；计数勘误 F9：`cargo test --list` 实测 23 项，
  审查记录一度写 24，本记录原写 30 均为误）。
- `cargo clippy -p orz-assurance --lib`：无新增告警。
- `git diff --check`（orz）：通过。

## 3. 102 runs 离线复验（`D:\tb-eval\jobs-official`，全量 102）

报告：`../../存档/root-artifacts-2026-09-06/LIF_102RUNS_REPLAY_2026-08-30.json`（harness 输出，含逐 run
fires/域计数/T̂/spikes/migrations/C1 置换）。

| 指标 | 设计锚点（2026-08-30 首轮） | 本实现复验 | 核对 |
|---|---|---|---|
| 决策点总数 | 4157 | 4157 | ✓ 精确一致 |
| err 固定时间语义 | 22 run / 24 fires | 22 run / 24 fires | ✓ 精确一致 |
| err 轮语义反事实（k=3 档） | 11 run / 8 fires | 3 run / 3 fires（k=8 → 6/6；k=15 → 15/17） | 方向一致：轮 ⊂ 时间；绝对数依赖反事实 k（设计未定稿该参数，V2 再校准） |
| 无「轮触发而时间不触发」 | 成立 | 成立（k=3/8/15 均成立） | ✓ |
| stuck θ=1.5·T̂ | 0/102 触发 | 0/102 触发 | ✓ 精确一致 |
| stuck 峰值 θ 比 | 0.65 | 0.991 | 差值归因 T̂ 估计口径（本实现=在线钳制估计 vs 首轮 per-run 近似）；**2026-08-30 用户裁决：0.65 仅为估计器定稿前的初步验证值，不专门复现**；定稿规格读数 ~0.99 仍 <1，操作性断言（0/102 触发）不受影响 |
| slow | 2/102 | 2/102 | ✓ 精确一致 |
| stall | ≥6/102 | 8/102 | ✓ |
| deny | 0/102 | 0/102 | ✓ |
| C1 时间打乱（err 通道，10 置换） | 14 变 / 8 不变 | 17 变 / 5 不变 | 方向一致（时间结构在起作用、存在计数成分）；置换种子不同致绝对数差异 |

## 4. 结论与边界

- I1 S1/S2 闭合：计算器与估计器实现、测试、离线回放三件齐全；102 runs 全量可
  跑（约 2s），4157 决策点与 err 22/24、stuck 0/102、slow 2/102、deny 0 精确复现。
- fires 仅内部留痕（引擎 fire_times），未入任何事件面/渲染面（D5/F6 约束）。
- 未闭合项（留给 V2 / 后续切片）：轮语义反事实 k 的正式口径；四对照门
  C2/C3/C4 正式判定；聚类对照（§9.8）。（2026-08-30 用户裁决：stuck 峰值
  θ 比 0.65 属初步验证值，不做口径复现；设计稿引用数字按注记处理。）
