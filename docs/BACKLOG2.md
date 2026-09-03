# BACKLOG2 — 工具执行层改革（TER）开放项

> 用途：TER 专用未闭合项路由（P0）；主
> [BACKLOG_AND_PRIORITIES.md](BACKLOG_AND_PRIORITIES.md) 只保留指针，
> 本文为 TER 明细权威。状态：`current`（2026-09-03 设计定稿；M0 设计门
> T0.1–T0.4 已放行；M1 实施中——T1.1–T1.6 已闭合，见 TODO2.md）。
> 入口：设计稿
> [TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md](TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md)
> / 实施步骤 [TODO2.md](../TODO2.md) / S4_PROGRESS §16.17/§16.18。

## TER-0 工具执行层改革（P0；2026-09-03 设计定稿 + 分步计划落盘；M0
设计门 T0.1–T0.4 已放行，M1 实施中——T1.1–T1.6 已闭合）

一句话定义：orz 工具执行层“独立可用 + 无自身硬超时”改革——常驻能力
默认开启（S5-2/180s 首报/模型面封闭）、去硬杀（删 runner 自加墙钟、
工具层超时改 auto-bg）、5min idle+CPU 兜底、黑板 processes live 视图、
F6 预算三档（默认 off）、W-F11 环境快照 PULL 面、W-F12 本地透明层、
W-F13 阅读面大修（64KB + 输出检索对象 + 截断标记）；轮预算撤默认硬限
（保留可配逃生阀）。分步：M0 设计门 → M1 orz 主线 → M2 Windows
runner/VM → M3 回归复验（明细在 TODO2.md）。边界：15min 长档、待审圈
系统审计、fold 状态摘要不在首轮。

### 决策记录（2026-09-03）

- 常驻能力默认全开；180s 首报/后台化为主线默认（远期分档加 15min）。
- 仅留官方评测墙钟；orz/runner 自加硬杀墙钟全部删除。
- 5min 无活跃兜底含 CPU 活跃辅助（不误杀无输出计算）。
- 轮预算撤默认 120 硬限，保留可配上限逃生阀。
- W-F12 先做本地透明层；W-F13 暂定 64KB，实施后压测复核。
- F11 不做 brief 注入；压测信息最小化，环境侧快速确定性失败。

### 状态

- 设计：`TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md`（§1–§9
  定稿，§10 阶段拆分，§7 参数已定案）。
- 实施：M0 设计门 T0.1–T0.4 已闭合 2026-09-03（签名见 TODO2.md
  T0.4）；M1（T1.1–T1.13）实施中——T1.1 S5-2 常驻默认、T1.2 首报/
  后台化预算默认 180s（含 2026-09-04 复审处理）与 T1.3 模型面封闭
  已闭合；T1.4 去硬杀语义（timeout 分层改 auto-bg deadline、后台化后
  原解析超时退役、模型面删 kill 宣示）已于 2026-09-04 闭合（见 TODO2.md
  与审计 TER_T1_1 / TER_T1_2 / TER_T1_3_MODEL_FACE_CLOSURE_2026-09-04 /
  TER_T1_4_NO_HARD_TIMEOUT_KILL_2026-09-04）；T1.5 idle+CPU 兜底
  （ActivitySampler：输出字节 + 进程树 CPU，300s 无活跃 → idle_killed +
  自描述提醒，阈值参数化；Linux/Windows CPU 读数 + 其它平台安全侧）已于
  2026-09-04 闭合（审计 TER_T1_5_IDLE_CPU_BACKSTOP_2026-09-04）；
  T1.6 黑板 processes live 分区（读取时现算 TaskLiveSnapshot →
  LoopHost 事实 → section=processes 渲染 ≤8KiB；epoch/receipt_id 越权
  显式报错；kill=进程行 pid + 既有 PID 中断语义，不暴露 &）已于
  2026-09-04 闭合（审计 TER_T1_6_PROCESSES_LIVE_PARTITION_2026-09-04）。
- 专项：TER-0.1 生成器 v0.2 表全面对齐已登记（见下），不阻塞 M1。

## TER-0.1 生成器 v0.2 表全面对齐（专项；P0；2026-09-03 登记，不阻塞
T0.4/M1）

一句话定义：run-event v0.2 夹具树与生成器表脱节——P2-11/P2-13 起的约
20 项夹具增量仍为手工维护、未同步进
`scripts/generate_run_event_fixtures.py` 的表（`session_archive` 事件与
payloads、`tool_completed` dep-graph ×4 / failure-target ×6 extras、
`retrieval_close_record` extras 与 minimal 演进、`tool_running` /
`mechanical_audit_update` / retrieval-close-record 等 envelope 时间戳与
身份 override、FIXTURES_README_V02 文本自身亦会被生成器改写）；目标 =
生成器全树重建与已验收夹具树 **0 差异**（含 canonical_cli 与 v0.2
README），此后夹具改动一律先进生成器表再重建。边界：不阻塞 T0.4/M1，
M1 期间可并行抽做；重跑生成器前先读
[`TER_T0_3`](audits/TER_T0_3_ADR_CANDIDATE_REGISTRATION_2026-09-03.md)
§4/§5（试运行会重建三棵夹具树，已跟踪文件还原用 git checkout-index）。

入口：[生成器](../scripts/generate_run_event_fixtures.py) /
[T0.3 审计 §4/§5](audits/TER_T0_3_ADR_CANDIDATE_REGISTRATION_2026-09-03.md)
/ 夹具树 `runtime/fixtures/run-event-v0.2/` +
`assurance/fixtures/canonical_cli/` / 测试基线 273 passed（T0.2）。
