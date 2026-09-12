# 0z S2 全面复审与处置记录（2026-09-13）

> 范围：0z S2 批次（orz `b3479716`）的设计合理性、实现合理性、设计与实现符合性。
> 方法：三路独立并行审查（C 状态链 / B+E 进程树与回收 / 事件面与合约），只读取证 +
> 真实子进程实验 + jsonschema 实证复验 + 41 族 × 251 场景对拍复核。
> 结论：**原 S2 批次未通过全面复审**（5 P0 / 11 P1 / 14 P2 / 10 P3）；本批为返工收口。
> 处置提交：orz `<S2R>`（见 git log）；未闭合项逐条登记于 §4。

## 1. P0（五项，全部返工）

| # | finding | 处置 |
|---|---|---|
| F-C-1 | `repair_partial_tail` 4 KiB 尾窗：撕裂行 >4 KiB 时找不到换行 → `set_len(0)` 整卷清零 | 重写为 64 KiB 倒序分块扫描；`keep=0` 仅当全文件无换行；`set_len`/`sync` 失败记 warn。钉子：`torn_long_write_is_repaired_without_losing_earlier_events`（80 KiB 撕裂行 + 恢复，断言前事件存活 + 链可重放） |
| F-C-2 | 部分写入后 BufWriter 残留未写后缀，重试拼接撕裂残片进下一行 | 错误路径先 `self.file = None` 丢弃缓冲再 repair（repair 以独立句柄操作路径）；注入缝新增 `partial_bytes` 维度（`new_with_torn_write_faults_for_tests`）产生真实撕裂 |
| F-BE-1 | finalize 扫除跨 run 误杀：共享 `.gsa/process_trees/` + `run_id` 恒空 + 父检查跳过 → 杀并行实例在跑子进程 | 登记行补 `run_id`（宿主从 journal_dir 派生）与 `action_class`；`plan_sweep` 改 `SweepMode::{Start,Finalize}`——Finalize 只候选 `run_id==own` 行（其余 `ForeignRun` 拒绝不触碰），own 行仅豁免自身 pid 的父检查。钉子：`finalize_sweep_never_touches_foreign_rows` |
| F-BE-2 | 回收在跑保护未接线（`in_flight` 恒空）+ hard 档先回收后树杀（违反 §4.7.1 末段裁决 14 次序） | 每次调用在门判定前把静态写入目标登记入 `in_flight_targets`（guard 摘除）；`plan` 与 `execute_expiry` 均吃在跑面（到期行在跑则重排队）；hard 档次序改 planned→树杀→回收；门只 evaluate 一次（附带修 F-BE-12 双探针分歧） |
| F-EV-1 | `resource_exhausted` producer 缺必填 `call_ids`（jsonschema 实证 planned/executed 双双 INVALID）——hard 档唯一真实路径必产法官判废事件 | planned/executed 行枚举本 run 登记表在跑记录的 call_id 集（§4.8 表 1 原文「含将杀的 call_id 集」）；登记缺席以当前被拒调用兜底（minItems:1） |

## 2. P1（返工收口的七项 + 留裁决两项）

- **F-C-3** `families_s2c::TERMINAL_TYPES` 漏 `run_terminated`（与 Python 镜像分叉，Python 误判降级卷 invalid）→ 两侧同步（Rust 清单 + 豁免条件 `run_invalidated|run_terminated`；Python 同式）。
- **F-C-4** acp_server `RunRecorder` 把 DegradedDropped 当硬失败（restore/archive run 无终态无侧车）→ `record` 识别 DegradedDropped：链簿记不动、终态照常尝试（落链或侧车由 writer 保证）。
- **F-EV-2** Python 七镜像是死代码（未接 `validate_journal_text`）→ 七调用接入入口 `payload_errors` 门内；函数块迁回入口之前（模块风格对齐）。
- **F-EV-3** `run_shutdown` 扫除行链上不可达（finalize 在 journal shutdown 后；Err 臂/终态前缺 drain）→ 新增 `LoopHost::finalize_process_trees`（默认无操作），loop 成功臂与 Err 臂在终态事件前 finalize + 三路 drain；main.rs 的 post-shutdown 调用保留为旁路 backstop。
- **F-BE-5** 指纹降级（argv+env → 镜像哈希）未补偿 → 新增创建时间守卫（`GetProcessTimes` vs `started_at`，容差 5 s，`CreationTimeMismatch`）；NotRunning 拒绝行随扫除清理（不再喂未来 sweep）。argv+env 全指纹留作 S4 观察项（跨进程 argv 读取脆弱，补偿后防护等价）。
- **F-BE-6** 回收在事故场景失效 → 候选发现改深度 2 嵌套 workspace 根扫描（上限 64 候选，阶梯排序）；预算口径不变（默认 8 GiB 可 env 覆盖），run 总量上限（下条）+ 在跑保护共同兜住误删面。
- **F-BE-7** 回收阶梯次序偏离 §4.6 → 候选按 scratch→包缓存→构建缓存排序（次序即裁决 14 的风险排序）；「限窗更早 run 的 scratch」整层**未实现**，登记 S4（需 run-scratch 台账，见 §4）。
- **F-BE-3（留裁决）** hard 树杀爆半径 = 全 run 树 vs §4.8 表 1 ③「只杀重档」——TerminateJobObject 无法分轻重。两案待裁决：(a) per-call-job 杀面（宿主按 call_id 持重档 call job 句柄，只 terminate 重档）；(b) 重释 ③ 为「重档拒绝触发、全树爆半径」并转录设计。另：是否收敛回 TER kill 面（`kill_foreground_commands`/`kill_all_background_tasks`）一并裁决。当前实现为 (b) 的事实形态 + `action_class` 已入登记表（(a) 的数据面就绪）。
- **F-C-8（已裁决，2026-09-13 用户裁决 16：fail-closed 维持）** bootstrap 的 run_preflight 在 DegradedDropped 时拒绝 run 启动——裁决 = 接受 fail-closed（盘满不起新 run，避免写入面在满盘下扩张），已显式登记设计 §4.3 第 8 条 + §11 裁决 16；bootstrap 行为零改动。

## 3. P2/P3（随批处理或登记）

- **已随批**：F-C-7（guard 白名单补 run_terminated）；F-C-9（截断自旋——已截断串不再重选；16 轮软界登记）；F-BE-8（窗口轮 = 工具调用轮，round 在 call_tool_inner 推进；expiry 摆脱冷却短路）；F-BE-11（run 总量上限 = 3× 预算，`ORZ_RECLAIM_RUN_TOTAL_BYTES` 可覆盖）；F-BE-12（审计行 `refusal:"reap"` 标签 + 门单次 evaluate）；F-BE-14（`dir_size_sync` 深度界 3）；F-EV-5（Python snake_case 首字符规则）；F-EV-8（stale 注释 v02=35）；F-EV-9（fixtures `measured`→`available` + unavailable 正例入 `check_repository` 注册）；F-EV-10（信封 schema 描述补 0z 来源注记）；F-EV-11（drain impl 注释归位 + 未知 fact 落 warn 不再静默丢）。
- **F-EV-4（裁决已定，无需返工）**：独立 facts-drain 通道**不构成对 0q 单漏斗的违反**——0q 漏斗域是完成信封盖章（拒绝信封在工具面恰过一次 `stamp_failure`），drain 事实行与 TER idle-kill 先例同构同点且在工具边界同刻落盘。ADR-0010 定稿转录时按实登记（设计 §5「走 stamp_failure 单漏斗」字面表述修正为「工具面过漏斗 + 事实行走 drain 通道」）。
- **F-EV-7（部分随批）**：`host_resource_snapshot` producer 已补（run_start + 跨档 tier_change 两触发）；`run_terminated{resource_exhausted}` 终态转换仍缺（hard 档杀后 run 继续跑，以普通终态收尾）——与 F-BE-4 同源，登记 §4。
- **登记不动（S3/S4 或观察）**：F-C-5（reserve 仅证据字段无实预占——设计原文即「间接保证」，不越设计；S4 满盘注入覆盖验证）；F-C-6（空卷+侧车角隅判 `degraded_complete` 未覆盖）；F-C-10（注入缝 doc-hidden 常驻可接受）；F-C-11（降级卷冻结 fixture 缺）；F-BE-10（guard 路径/ACP finalize 覆盖缺口——旁路 backstop 已留）；F-BE-13（§4.5 session 面读数行归 S4）；F-BE-15（判据面澄清：判据 4 承重件是 S1.1 两级 Job）；F-EV-6（freed_bytes 负值域外分歧，schema 前置拦截）。

## 4. 未闭合项（登记，不阻塞本批收口）

1. **F-BE-3 / F-C-8 用户裁决**（§2 尾）。
2. **F-BE-4 / F-EV-7 残段**：hard 档杀后 `run_terminated{resource_exhausted}` 终态转换 + 被杀调用的可读资源失败标注 + `resource_limit_hit` producer——归 S4（判据 12 实机 violation 读回同批）。
3. **F-BE-7 残段**：「限窗更早 run 的 scratch」回收层（需 run-scratch 台账）——归 S4。
4. **F-BE-6 残段**：候选内（目录一级子路径）超预算缩减未实现（跨候选取舍已实现）——归 S4 观察（run 总量上限 + 嵌套发现已缓解失效面）。
5. F-C-5/F-C-6/F-C-11/F-BE-10/F-BE-13（§3 末条）——S3/S4 对应批。

## 5. 复验证据

- 回归：orz-assurance 219/0、orz-loop 769/0/3、orz-host 324/0/5（单线程）；`journal::recorder` 13/13（含撕裂端到端钉子）、`process_tree` 6/6（真实进程 + F-BE-1/5 新钉子）、`reclaim` 3/3；对拍 `families::tests` 7/7（Rust↔Python 判决逐格零差，含新七族）。
- fmt 干净；本批文件 clippy 零告警（dunce::canonicalize 替代禁用的 std canonicalize）。
- `check_repository` `valid: true` / manifest 重算（随批）。
