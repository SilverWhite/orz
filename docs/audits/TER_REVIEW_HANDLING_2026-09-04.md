# TER M1/M2 全面审查处理（2026-09-04）

> 上游：[`TER_M1_M2_COMPREHENSIVE_REVIEW_2026-09-04.md`](TER_M1_M2_COMPREHENSIVE_REVIEW_2026-09-04.md)
> （有条件 PASS：P1 × 2、P2 × 5、P3 ~24）。用户裁决（2026-09-04）：
> **P1-1 补生产者＋单测；P1-2 转正/取代清单无异议；全部审查问题处理**。
> 本文为逐项处置账本；代码批 = orz `35db6741`（`feat/fusion-architecture`，
> 已推送 cli）；正式裁决 = ADR-0010 §14.55。分片原始报告保留于
> [`../../.ter_review_2026-09-04/`](../../.ter_review_2026-09-04/)。

## 1. P1（×2，已定案）

| ID | 处置 | 证据 |
|---|---|---|
| M1L-1 idle-kill journal 生产者缺位 | **补生产者＋单测**（用户指定方案）：`LoopHost::drain_terminal_idle_kills` 事实源 + orz-host 映射（signal/status 过滤 + 去重 + `get_task` 输出路径）+ loop 在工具执行/run 收尾边界 drain，按 (task_id, run_id) 只对同 run mid-run 调用补记 `tool_running(status=idle_killed + reason)` | orz `35db6741`：host.rs / host_exec.rs / controller.rs / orz-host lib.rs / terminal.rs / types.rs；单测 `idle_kill_after_mid_run_journals_lifecycle_tool_running`（事件序/载荷/去重）与 `terminal_idle_kill_facts_from_dedupes_and_formats_reason` 通过 |
| M1L-1 附：T1.13 门审计 §1.3 过度声明 | 历史文档不改写；本文与 ADR §14.55 条目 5 登记修正——生产者于 2026-09-04 补入（原“由单测锁定”指 fixture 层，非 Rust 生产者） | 本文 §1 / ADR §14.55 条目 5 |
| D-1 ADR 候选未转正 | **转正/取代清单落盘**（用户无异议）：ADR-0010 §14.55（v1.55）正式取代 §4.5 300s kill_active、120 轮默认硬限、S5-2 接线、F6 push 例外等旧条款；§14.53 标注“保留为候选登记历史” | ADR-0010 §14.55 + §14.53 头注 |

## 2. P2（×5，已处理）

| ID | 处置 |
|---|---|
| a1 S1 10h 绝对兜底例外 | 登记例外句（设计稿 §3.1 状态机第 4 步 + ADR §14.55 条目 1）；代码注释既有口径复核 |
| a1 S2a daemonize 后 idle 采样停止 | 登记限制（terminal.rs 采样守卫注释）：该形态由 10h 兜底 + 会话清理负责；不改判为 idle |
| a1 S2b Linux pgrp CPU 记账 | 登记“部分未知、宁可少杀不误杀”（terminal.rs 注释 + ADR §14.55 条目 7）；跨 pgrp 全树记账留待后续评估 |
| D-2/M2W-1 no-AC 生产墙无权威落点 | 登记：TER 设计稿 §4 + ADR §14.55 条目 6（no-AC+allowlist 生产墙；AC 仅对照基线） |
| a2 F6 processes completed elapsed 增长 | 修复：live 快照 completed 行 elapsed 冻结为 end−start（terminal.rs） |

## 3. P3 / 提示（已处理/登记）

已代码修复：

- S3 idle 提醒固定 300s → 完成提醒不再硬编码阈值（task_completion.rs；精确阈值随事件 reason/tracing）。
- S4 types.rs “typically 15s”残留 → 改 180s 单源口径。
- S5 内部 auto-bg 模板单一默认数值 → 只保留“命令形态默认 300/600s、仅为 auto-bg deadline”口径；schema 静态 default 移除（由生效默认渲染）；legacy 分支保留配置数字（配置跟踪测试不回归）。
- F3 controller.rs 陈旧 “MAX_TOOL_ROUNDS=120” 注释 → 0=unlimited 口径。
- F4 `ORZ_MAX_WALLCLOCK` 非法值 → controller 层留 warning（0/缺失/非法 = unlimited 语义不变，拼错不再静默）。
- F7 不支持 live 读取渲染“（无）” → `terminal_live_capable` + 渲染层显式「不可用」标注（host/test host 覆盖）。
- M1R-2 命中整行无长度钳制 → `MAX_OUTPUT_OBJECT_HIT_LINE_CHARS=4K` 截断 + 标注 + 单测。
- M2W-2 DNS UDP-only 边界 → wf12 README 覆盖边界登记。
- M2W-3 dns_refusal fail-fast → 空 allowlist 段报错 exit 2；bind/权限 OSError/ValueError 启动预检（py_compile + selftest + fail-fast 本地通过）。
- M2W-4 fake loader 三种形状 → text+tool_calls 并存 / 空 tool_calls / 非字符串 call_id / 非对象 arguments 全部 fail-closed；单测补 4 形状（orz-bin `fake_scenario_loader_round_trip_and_fail_closed` 通过）。

文档/口径统一（随主仓提交）：

- D-3/M1L-4 push 次数 → 文档统一“实现 ≤3/run、verifier 上限 ≤4/run”（设计稿 §3.3/§10、ADR §14.55 条目 4）。
- D-4/S7 env override → 设计稿 §3.1 核对项 + ADR §14.55 条目 3（override≠默认源）。
- D-5 设计稿状态行滞后 → 更新为正式裁决/实施状态。
- M1L-2 runner 60s floor → 登记为待注释项（run_agent_arm.ps1 有 0l 并行未提交差异，不在本批混改；随 0l 合入或 T2.4 批补注释）。
- M1L-3 “≤80B” → 契约按 ≤80 字符登记（T0.2/审计口径随 M3 收口统一）。
- M1L-6 0 与未配置渲染不可区分 → 登记为提示级（session 面渲染语义不改；来源行随 M3 视需）。
- M1R-1/3 read_file CJK 估算与行前提 → 设计稿 §3.6 边界登记（M3 压测复核）。
- M2W-5 T2.3 验收依赖 → TODO2 T2.3 验收行补“idle-kill 事件断言依赖已就绪”注记（见 §5）。
- a2 F7 之外的口头补充（F3/F4/F6 已含上表）；a2 F8 时钟偏差、M1L-7/M2W-6 env 穿 sandbox、M1R-4 墙内计时、M1R-5 计数复跑 → NOT-VERIFIED/待实机（§5）。

## 4. 验证

- orz-tools：output_object 5 passed、idle_killed 2 passed、description 97 passed（含
  `tool_description_timeout_numbers_track_config`）。
- orz-loop：`idle_kill_after_mid_run_journals_lifecycle_tool_running` 1 passed
  （journal 序/载荷/去重 + replay valid）。
- orz-host：`terminal_idle_kill_facts_from_dedupes_and_formats_reason` 1 passed。
- orz-bin：`fake_scenario_loader_round_trip_and_fail_closed` 1 passed；
  `cargo check -p orz-bin --bin orz --offline` 全绿（clean 后重建，消除此前
  测试构建的工件不一致）。
- dns_refusal.py：py_compile OK、`--selftest` OK、空段 fail-fast exit 2。
- fmt/diff check 净（CRLF 提示除外）。

## 5. NOT-VERIFIED / 待实机 / 未闭（登记，非缺陷）

- T2.3 实机验收（Job/LOW IL 跨调用存活 + idle-kill journal 断言——生产者已
  就绪，脚本与断言待 T2.3 批）；T2.4 同步接线；M3 T3.1–T3.5；TER-0.1
  生成器表对齐；no-AC 三项 FAIL 结转（T2.4/0l）。
- 干净工作树全量计数复核（本轮只做定向测试；计数口径随 M3 门复跑）。
- ORZ_MAX_WALLCLOCK 穿 sandbox（T2.4 DryRun 断言）、LIF run-origin 与
  sandbox 时钟偏差（F8）、env 快照墙内 ≤5s、10h 兜底实触发。
- S2a/S2b 后续增强（daemonize 采样退化方案、Linux 全树 CPU 记账）——已
  登记边界，未实施增强。
- M1L-2 注释与 M1L-3/M1L-6 口径统一——随 0l 合入 / M3 收口批补。

## 6. 入口

- 代码批：orz `35db6741`（已推送 cli/feat/fusion-architecture）。
- 正式裁决：ADR-0010 §14.55（v1.55）。
- 本批文档登记：TODO2（审查处理行）、BACKLOG2 TER-0 状态、CLI_PROJECT_INDEX
  v2.51（AUTH-TOOL-EXECUTION-REFORM → `current-design`）。
