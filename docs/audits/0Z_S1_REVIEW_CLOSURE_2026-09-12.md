# 0z S1.1 复核收口审计（独立复核三项工程裁决 + 其余审查项逐条处置）

> 日期：2026-09-12；批次：**0z S1.1（S1 复核收口）**；对象：orz 子模块提交
> **`ea794f90`**（工作区 0 dirty）；上游：独立复核
> [`0Z_S1_INDEPENDENT_REVIEW_2026-09-12`](0Z_S1_INDEPENDENT_REVIEW_2026-09-12.md)（F-1…F-10）；
> 设计权威：[`HOST_RESOURCE_SAFETY_DESIGN_2026-09-12`](../HOST_RESOURCE_SAFETY_DESIGN_2026-09-12.md)
> §4.2 / §4.7 / §4.7.1 / §9 / §11。
> 授权：用户 2026-09-12 裁决——「F1/2/3 是纯粹的工程问题，请你进行裁决；其余审查出来的问题
> 也都需要进行处理」；**按惯例登记**。
> 出口：落码 + 单测绿 + 门禁 `valid: true` + manifest 重算（S1 出口口径的延续）。

## 1. 裁决摘要（工程裁决，用户授权；一手来源见 §3.7）

| # | 议题 | 裁决 | 一句话依据 |
|---|---|---|---|
| 1 | 两级 Job 形态（F-1） | **恢复设计原形**：run 级 job 持限项（活到进程生命周期）+ 每次调用 job 持 `KILL_ON_JOB_CLOSE`，指派顺序**先根后子** | S1 的降级依据是把 job 句柄放进进程参数位；嵌套在 Win8+ 是**进程指派**属性，实测可用且上限真的咬合；`JOB_OBJECT_LIMIT_JOB_MEMORY` 官方原文即 job-wide 汇总 |
| 2 | `run_tests` 双漏（F-2） | **入门 + 挂 Job**：同一分档器/探针/拒绝文案；spawn 改经 `ProcessGroup`（run 级限项 + 调用级树杀） | 它是唯一「被判重档却两侧都不覆盖」的工具，评测 harness 经 `ORZ_TEST_RUNNER` 可达 |
| 3 | `ACTIVE_PROCESS` 取值（F-3） | **`2 × 核数 + 8`（下限 16）**，修订 §11 裁决 4 该臂 | 核数 = cargo 默认 `-j`；加 cargo 自身/链接器/测试 harness 必越界 → 内核拒绝创建进程把**合法重活**打成失败；上限职责是兜住失控，不是调度 |
| 4 | commit 上限口径（F-4） | 保留 `min(80%×limit, limit−4 GiB)` 作**上限帽**，实际取值 `min(帽, 装配期 commit 余量 − 1 GiB)`（下限 2 GiB）；**轴间耦合登记** | Run B 死在名义 limit 的 **73%**（`os error 1455` 页面文件太小——满盘使页面文件无法增长）；真机日常基线已被邻位占到 ~80%，上限必须表达"本 run 还能占多少" |

## 2. 变更清单（orz 子模块 `ea794f90`，6 文件 +1234 / −178）

| 文件 | 内容 |
|---|---|
| `crates/codegen/xai-tty-utils/src/resource_job.rs` | `RunResourceJob` **真持 run 级 job 句柄**（限项 + `KILL_ON_JOB_CLOSE`，Drop 即整树回收）+ `assign_process_handle` / `contains_process`；全局槽改 `Mutex<Option<_>>`（生产仍 first-wins）+ `replace_global_run_job_for_tests` 测试接缝；`record_attach_failure` / `attach_failure_count`；模块文档重写为"两级、先根后子"并带官方引文；单测：嵌套拓扑 + run 级咬合 + 计数 |
| `crates/codegen/xai-tty-utils/src/lib.rs` | `ProcessGroup::new_with_limits` 只承载**调用级显式**限项（限项改由 run job 承担）；`attach_pid` **先 run 后 call** 两次指派（run 失败计数不致命，call 失败照旧报错）；`readback_limits`（实际受制限项，调用级优先）与 `readback_call_job_limits`（调用 job 自身，用于证明限项未重复） |
| `crates/orz-host/src/resource_gate.rs` | 目标卷：`write_targets`（cwd + 重定向 + `--target-dir`/`--out-dir` + `cd`/`Set-Location`/`pushd` + `CARGO_TARGET_DIR` 类；引号包裹的 wrapper 命令再分词）+ `evaluate_for_volumes`（全卷全过、逐卷读数）+ 探针**最近存在祖先**取卷；档位：`ResourceTier::Unknown`、`GateDecision::Refuse.tier` 改实值；上限：`run_active_process_limit`（2×核数+8）+ commit 余量条款；精度：`strip_payload_bodies`（here-string / heredoc 剥体留头）；12 条新单测 |
| `crates/orz-host/src/lib.rs` | 汇点：`write_targets` + `evaluate_for_volumes`，拒绝信封新增 `write_targets[]`；`run_tests`：**预检门**（不足即拒、不启动）+ spawn 改经 `ProcessGroup`（超时先 job 杀再 TaskKill）；装配：抽出 `install_resource_safety` + 测试接缝 `with_host_resource_safety_limits`；单测：`run_tests_is_gated_like_any_heavy_action`、端到端 `tool_spawn_is_really_bounded_by_the_run_job`（轻档调用 → 终端 spawn → run 级 commit 上限拒绝子进程） |
| `crates/codegen/orz-tools/src/computer/local/terminal.rs` | attach 失败日志 debug → **warn**（计数在 `attach_pid` 内）；breakaway 回退注释更正为"Win8+ 仍可经先根后子关联" |
| `crates/codegen/orz-tools/src/implementations/grok_build/read_file/mod.rs` | 门注释更正为收窄后的真实链（UTF-16 BOM → 无 BOM 奇偶启发式 → 无 NUL UTF-8；GB18030 在历史路径） |

父仓库（文档面，随本批）：`docs/audits/0Z_S1_INDEPENDENT_REVIEW_2026-09-12.md`（复核记录 + §8 处置回执）、
`docs/HOST_RESOURCE_SAFETY_DESIGN_2026-09-12.md`（§4.2 先根后子补注、§4.7.1 裁决表、§9 耦合与代价、
§11 裁决 11–14）、`docs/audits/0Z_S1_IMPLEMENTATION_AUDIT_2026-09-12.md`（三处计数更正 + §3.3 更正块）、
本文件、[`BACKLOG 0z`](../BACKLOG_AND_PRIORITIES.md) / [`TODO P0-0z`](../../TODO.md) / `CLI_PROJECT_INDEX.md`。

## 3. 逐条处置回执

| 复核项 | 处置 | 证据 |
|---|---|---|
| **F-1** 两级形态 | 已恢复（见 §1 行 1 / §2） | `per_call_group_nests_under_the_run_job_and_reports_its_ceilings`（含 `contains_process` 真机断言）、`run_job_ceiling_bounds_a_child_of_a_per_call_group`（run 级上限约束调用级子进程）、orz-host 端到端 `tool_spawn_is_really_bounded_by_the_run_job` |
| **F-2** `run_tests` | 已入门 + 已挂 Job | `run_tests_is_gated_like_any_heavy_action`（`exit_code=1`、`timed_out=false`、文案含 "refused before dispatch" + "Nothing was started"） |
| **F-3** 进程数上限 | 已改 `2×核数+8`（下限 16） | `job_limits_use_the_eighty_percent_or_reserve_rule`（含 `> 核数` 断言，本机 12 核 → 32） |
| **F-4** commit 上限 + 耦合 | 已改余量口径 + 已登记耦合 | 同一单测（忙碌 21 GiB / 空闲 80 GiB / 濒临耗尽落 2 GiB 地板）；设计 §4.7.1 / §9 |
| **F-5** 目标卷 | 已实现（静态写入目标 + 全卷全过 + 祖先取卷） | `write_targets_follow_the_real_corpus_shapes`（含 Run A 的 `cd …; cargo test`、`--target-dir`、`cmd /c "cd /d …"` 形态）、`write_targets_ignore_reads_and_null_redirection`、`write_targets_capture_redirection_and_build_cache_env`、`any_short_write_target_volume_refuses_the_heavy_action` |
| **F-6** `hard` 假档 | 已加 `Unknown` | `tiers_follow_the_ladder`（unavailable → unknown）、`unavailable_readings_fail_closed_for_heavy_only` |
| **F-7** here-string / heredoc | 已剥体留头 | `here_string_bodies_are_not_command_syntax`、`heredoc_bodies_are_not_command_syntax`、`unterminated_payload_markers_do_not_swallow_commands`；**真实语料回放**：判重 56 → 49（消除 7 条脚本体内 `>` 误判），**零漏判**（Run A 的 cargo/rustc 调用仍全部判重） |
| **F-8** 命名/计数/注释 | 机器键统一 snake_case（设计 §4.1 显式登记）；S1 实施审计三处计数更正 + §3.3 更正块；`read_file` 注释更正 | 文本核对 + `cargo fmt --check` |
| **F-9** 端到端与可见度 | 已加端到端钉子 + attach 失败计数 + 日志升 warn | `tool_spawn_is_really_bounded_by_the_run_job`、`attach_failures_are_counted` |
| **F-10** 观测面 | 维持 S2 归属（本批只把 run 句柄与读回值开放给该面） | 设计 §5 / BACKLOG 0z S2 行 |
| 回收次序（复核外溢项） | **登记为 S2 承重**：回收不得删在跑重活的产物面 | 设计 §4.7.1 末段 + §11 裁决 14 |

## 4. 验证证据

| 命令 | 结果 |
|---|---|
| `cargo test --release -p xai-tty-utils --all-targets` | **27 passed / 0 failed**（S1 为 25；新增嵌套拓扑、run 级咬合、attach 计数；`commit_ceiling_actually_stops_an_over_committing_child` 保留） |
| `cargo test --release -p orz-host --lib -- --test-threads=1` | **317 passed / 0 failed / 5 ignored**（S1 为 308；`resource_gate` 模块 22 条 + 汇点/装配/端到端 5 条） |
| `cargo test --release -p orz-tools --lib util::encoding` | **20 passed / 0 failed** |
| `cargo test --release -p orz-tools --lib implementations::grok_build::read_file` | **121 passed / 0 failed** |
| `cargo fmt -p orz-host -p xai-tty-utils -p orz-tools -- --check` | 干净（未触碰 `orz-bin/tests/acaf_e2e.rs` 的既有差异） |
| `python scripts/generate_orz_source_manifest.py` | 重算 **1444 条**（文件数不变，本批只改内容） |
| `python scripts/check_repository.py` | **`valid: true` / `error_count: 0`**（`orz_source_manifest_files: 1444`） |
| 真实语料回放（10 个 run / 458 条终端命令） | 判重 **49** / 判轻 409（Run A 40/379、Run B 9/60）；7 条 here-string 误判消除、零漏判 |

复跑纪律（沿用 S1 教训）：全部 release 复用既有产物 + 磁盘余量守护（本轮实测最低 **6.91 GB**，
第一次守护在 10 GB 处误杀了一次构建，遂下调阈值到 6 GB 后跑完——本身即"真机磁盘带宽"的现场样本）。
未复跑：`orz-bin --bins`、clippy（分离 fingerprint 的整包重编，磁盘风险高于收益）——如实标注，不计入结论。

## 5. 与设计/登记的关系

- 设计 §4.2 第 1 条、§4.7、§4.6、§9、§11 已按本批裁决更新（§4.7.1 为新裁决表，§11 裁决 11–14 为索引）。
- 上游设计与登记不变：六子项结构（A–F）、S2–S4 分段、不换盘不换卷、不挂沙箱、机械层优先 + 信息返回、
  回收站取消与轮数窗口终裁**均未改动**。
- S1 实施审计的 §3.3 已加更正块（原文保留、更正显式），三处计数已更正——审计作为时点记录的属性不变。

## 6. 边界

- 本批**不**触碰 S2 范围：事件族/fixture/法官、C（ENOSPC 降级 + 终止形态）、B（登记 + 孤儿扫除）、
  E（回收机制与轮数窗口）、观测面接线。
- §4.7.1 第 14 条（回收与在跑重活的次序）是**设计登记**，落码属 S2。
- 非 Windows 仍无内核等价物（`kernel_enforced=false`），两级形态的收益集中在 Windows 真机路径。
