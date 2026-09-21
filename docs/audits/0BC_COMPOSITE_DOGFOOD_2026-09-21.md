# 0BC 复合狗粮轮 — 资源层收口与可失败分配 · 总结档（2026-09-21）

- **状态**：代码面全部落码并本地验证全绿（**工作树未提交、不推送、不重建**——按任务令）；S3 资源面新特性读数（CPU 去顶后墙钟／commit 临限通知事件／进程上限读数）**待下一载体重建后收取**（本轮载体 0.6.5 不含 0bc 代码）；Linux 编译面待重建批核验；S4 判据①–⑥对账见 §5；**RLI 组件可用性裁决见 §6**。
- **run**：`RUN-CLI-6ab00c8a`（本会话即 0bc 复合狗粮轮观测场；载体 0.6.5）。
- **来源**：TODO `P1-0bc` / BACKLOG `0bc`（判据①–⑥）/ 设计 [`HOST_RESOURCE_OS_DELEGATION_DESIGN_2026-09-20`](../HOST_RESOURCE_OS_DELEGATION_DESIGN_2026-09-20.md) §11·§7 / [`0AM_RLI_RETROFIT_REMAINDER`](0AM_RLI_RETROFIT_REMAINDER_2026-09-20.md) §5·§8。
- **范围**：S1 勘定 → S2 落码（三面改造＋可失败分配＋降级链）→ S3 复合狗粮轮（RLI 观测已收；资源面新特性读数延后）→ S4 收口；长杂轮 FR-3/5/6/7；杂项 FR1/FR2。

## §1 交付清单（代码面，全部落码）

### orz 仓（17 文件）

| 文件 | 改动 | 验证 |
|---|---|---|
| `crates/codegen/xai-tty-utils/src/resource_job.rs` | commit 临限**通知设施**：`CommitNotificationMode{Low,High}`＋`COMMIT_NOTIFICATION_MODE=High`；`install_commit_notification`（完成端口＋`JOBOBJECT_NOTIFICATION_LIMIT_INFORMATION_2`＋`JobObjectAssociateCompletionPortInformation`）；`readback_commit_notification`（查询式，不属天花板轴）；`drain_job_notifications`（`GetQueuedCompletionStatus` 非阻塞，只收 msg=11）；`query_commit_violation`（Violation2→v1 兜底）；`JobNotification`；`RunResourceJob::{drain_notifications,notification_installed}`＋端口随 Drop 释放；`drain_global_run_notifications()`；测试缝 `install_with_notification_mode`/`replace_global_run_job_for_tests_with_mode` | 测试 4 条（readback 往返／**fires**：1 GiB 子进程 vs 64 MiB 阈值→1 条、`limit_bytes=256 MiB`、`flags=0x200`、子进程照常 COMMITTED／silent 负对照／probe 双向探针）＋**32/32 绿** |
| `crates/codegen/xai-tty-utils/src/lib.rs` | 导出面扩（`CommitNotificationMode`／`JobNotification`／`drain_global_run_notifications`／`COMMIT_NOTIFICATION_MODE`，windows 门控） | cargo check 绿（Windows；Linux 面待重建批） |
| 根 `Cargo.toml` | windows features 补 `Win32_System_IO` | check 绿 |
| `crates/orz-host/src/resource_hint.rs` | `default_job_limits` 改**通知式**：`commit_limit_bytes=None`（无硬顶）；`commit_notification_bytes`＝原推导算式（`min(80%×limit, limit−4 GiB)`＋headroom−1 GiB、floor 2 GiB，语义改为**警报线**）；CPU 默认**去顶**（`cpu_rate_limit_from_env()`，`ORZ_JOB_CPU_RATE_PERCENT` 1..=100 可选档）；`run_active_process_limit()` 加 `ORZ_JOB_ACTIVE_PROCESS_LIMIT`≥1 覆盖（`active_process_limit_from_env()`）；`commit_notification_hint()` 渲染（`[资源软提示]` 中文短句＋英文读数） | 测试重写（通知语义＋ENV_LOCK＋restore_env）＋2 条 env 测试→**16/16 绿** |
| `crates/orz-host/src/lib.rs` | `commit_hint_emitted: AtomicBool`（每 run 软提示一次去重）；派发头 drain 循环→落 `host_resource_snapshot` rows（`tier="unknown"`、`trigger="commit_notification"`、readings`{commit_notification_bytes,commit_used_bytes,limit_flags,violation_flags}`）；结果头装配 head 列表（volume_hint＋commit_hint） | check 绿；host 全量 **322/0/5**（串行） |
| `crates/orz-assurance/src/journal/recorder.rs` | S2④ journal 装配路径**可失败分配**：`is_alloc_failure`（`ErrorKind::OutOfMemory`）；`FallibleBuf`（`try_reserve` 生长缓冲；`append_event`／`prepare_degraded_terminal` 经 `serde_json` `io_error_kind()` 保 Kind）；`WriteFaultScript` 增 alloc 故障类＋三构造器（含 `new_with_alloc_faults_for_tests`）；`append_with_backoff` 顶部 alloc 检查**直返 Err（不走退避梯）**；`run()` Err 臂 guard＝`is_storage_full_error‖is_alloc_failure`→`enter_degraded`（终态＋degraded 摘要＋不 abort） | journal **43/43 绿**（含新测试 `alloc_failure_enters_degraded_and_chain_stays_replayable`：构造注入→RunStarted 拒收→骨架过滤→RunFinished 带 degraded marker→replay valid） |
| `crates/orz-assurance/src/journal/families.rs` | `verify_host_resource_snapshot` trigger 允许集＝`tier_change\|run_start\|commit_notification`（**不新增事件族**，S1 定案） | assurance 全量 **259/0** 绿 |
| `crates/orz-assurance/src/lif/rli.rs` | FR-6：`RliChannel::restore` 采纳 sanitize 后 `zeta`（finite 检查＋钳 [0,4]，NaN/越界回代码默认；快照字段本已存在，无需 schema bump）；FR-7：`RliDomainRow.v_err/env_err`→`Option<f64>`（`skip_serializing_if`） | lif **59/59 绿** |
| `crates/orz-assurance/src/lif/mod.rs` | FR-7 测试适配（`matches!`/`is_some_and`） | 同上 |
| `crates/orz-loop/src/action_ledger.rs` | S2④ ledger 装配路径：`append_ledger_rows_range` 行缓冲增长改 `try_reserve`（失败→OutOfMemory「ledger assembly allocation failed」；不推 fold＝既有 fail-soft 契约） | check 绿 |
| `crates/orz-loop/src/model_face.rs` | S2④ 模型面装配路径：`build_model_face`→`Result<Vec<Message>,io::Error>`＋`try_clone_str/message/messages` 族（`try_reserve_exact`）＋`model_face_message_count`（不物化计数面）＋`FORCE_ALLOC_FAILURE` 测试缝＋新测试 | check 绿；loop 全量 **822/0/3** |
| `crates/orz-loop/src/agent_loop.rs` | S2④ 接线：:1398 改计数面（免为条数物化整面）；:2700 失败上抛 `AgentLoopError::AllocFailure`（走 run 终态，不 abort）；**FR-3 全量**：删 `compression_window_tool_defs`、删 `current_tool_defs` 收窄分支、消费分支去过滤/丢弃/提示（窗口轮动作全部落穿正常派发；延迟收口保留）、`pending_keeps_tools` 扩入 `ModelCompression`、两处注释；测试 12 处改写（含 drops 测试重写为 `…_dispatches_non_write_calls_without_notice`） | 同上 |
| `crates/orz-loop/src/controller.rs` | `AgentLoopError::AllocFailure` 变体＋终态映射；**FR-5**：工具 schema `name` enum 由 `TemporalState::known_feature_names()` 链 `RliShadow::known_feature_names()` 动态拼（去重保序），双源合一；**FR-7** 渲染：now/recent 两处 `map_or_else("—")`；rli 渲染测试隔离既有注入门（`with_max_inject_tokens_per_round(1_000_000)`） | 同上 |
| `crates/orz-loop/src/context_scale.rs` | FR-3：窗口块文案改「打断式提醒（不锁工具面、动作照常）」；**删** `window_dropped_calls_notice` | 同上 |
| `crates/orz-loop/src/checkpoint.rs` / `prompt.rs` | FR-3：注释与 doc 更新（窗口轮＝常规工具面） | 同上 |
| `crates/codegen/orz-tools/src/implementations/grok_build/search_replace/mod.rs` | **FR1**：新 `is_combining_mark`（0x0300–036F/1AB0–1AFF/1DC0–1DFF/20D0–20FF/FE20–FE2F）＋`strip_combining_with_map`＋`codepoints_preview`＋`build_combining_mark_hint`（匹配失败时按「忽略组合记号」定位最近似区段，回显双方**码点**＋NFC/NFD 提示；早退防虚假提示）＋失败装配接线＋3 测试 | search_replace 全模块 **113/0** 绿 |

### D:\CLI 仓（非 orz）

| 文件 | 改动 |
|---|---|
| `scripts/run_orz_tests.ps1`（新增） | **FR2** 测试入口脚本：清 7 键（`ORZ_ACAF_FAIL_CLOSED/MANIFEST/KEYSTORE/BINARY`＋`ORZ_LIF_RLI_SHADOW`＋`GROK_HOME/GROK_AGENT`）＋`RUST_MIN_STACK` 默认值＋透传 `cargo test` 参数（用法：`powershell -File scripts\run_orz_tests.ps1 test -p orz-loop --lib -j 2`） |
| `docs/cases/harness_environment/ORZ-ENV-POLLUTION-001-env-pollution-fake-red.md` | 0bc 落点登记＋两条同族实例（RLI_SHADOW／GROK_HOME+GROK_AGENT） |
| `assurance/run_event_journal_validation.py` | journal verifier Python 镜像：`host_resource_snapshot` trigger 允许集同步扩（与 `families.rs` 逐值一致） |
| `TODO.md` | `P1-0bc` 条目收口注记（含本档入口） |
| 本档 | 总结与裁决 |

## §2 S1 勘定记录（要点）

- **可失败分配三路径**（S1 定案，全部落码）：journal 装配（`recorder.rs`）／模型面装配（`model_face.rs build_model_face`）／ledger 装配（`action_ledger.rs`）。
- **env 名**：`ORZ_JOB_CPU_RATE_PERCENT`（可选档 1..=100）／`ORZ_JOB_ACTIVE_PROCESS_LIMIT`（≥1）。
- **临限事件形状**：复用 `host_resource_snapshot`＋新 trigger＝`commit_notification`（**不新增事件族**；Rust verifier＋Python 镜像同步扩）——与 S1 定案一致。
- 语义裁定（真机实证）：**HIGH＝「超过上限」方向**（violation flags=0x200）；设计 §11 裁决② 原语名 **LOW→HIGH 修订**（见 §7-7）。

## §3 S2 落码要点与真机语义勘定

- **commit 通知设施（真机语义，探针实证）**：HIGH＝「超过上限」方向——1 GiB 子进程跨 256 MiB 阈值 → 收到 1 条 `{limit_bytes:268435456, used_bytes:86016, limit_flags:512(0x200), violation_flags:512}`；LOW 方向装置静默（不合「临限」语义，不采用）。**设计 §11 裁决② 原语名修订**：设计文写的 `JOB_OBJECT_LIMIT_JOB_MEMORY_LOW`，实证应按 **HIGH**（`JOB_OBJECT_LIMIT_JOB_MEMORY_HIGH=0x200`）实现——本批按实证落码，设计文档勘误待随提交批。通知**投递有延迟**（取件时子进程可能已退出；`used_bytes` 为取回时刻读数）且**不阻断分配**（子进程照常 COMMITTED）——正合「只观测不硬拒」。
- **阈值算式**沿用原推导（`min(80%×limit, limit−4 GiB)`＋headroom−1 GiB、floor 2 GiB），语义由「硬顶」改为「**警报线**」（只产生通知）。
- **软提示**：每 run 一次（`commit_hint_emitted` 去重）；结果头部 `[资源软提示]` 中文短句＋英文机械读数（沿用 0af 混排定案）。
- **CPU 去顶**：默认 `cpu_rate_percent=None`（不再设置）；`ORZ_JOB_CPU_RATE_PERCENT`（1..=100）为可选档。
- **进程上限**：默认不变（`2×cores+8`、≥16）；`ORZ_JOB_ACTIVE_PROCESS_LIMIT` 覆盖。
- **可失败分配**：三落点（journal／模型面／ledger）；分配失败＝`ErrorKind::OutOfMemory`，与 ENOSPC **同形**进降级链（DegradedDropped＋degraded 摘要＋终态，不 abort），但**不走退避梯**（内存不会因等待回来）；journal 侧构造性注入测试 `alloc_failure_enters_degraded_and_chain_stays_replayable` 为判据④的直接证据。残余登记：`serde_json::Value`（`ToolCall::arguments`）无 `try_reserve`，深克隆保留普通克隆（注释在案）。

## §4 S3 复合狗粮轮读数

> 本会话（`RUN-CLI-6ab00c8a`）即观测场；载体 0.6.5（含 0am 全线，**不含 0bc 代码**——工作树未提交、未重建）。

### 4.1 RLI 观测（0am 实际表现；复合口径「读数随轮收取」）

- **自判域**：当前 `normal`（入域 r304；驻留 18+ 轮）；全 run 51 次切换——早期（r22–r51 段）normal↔low_progress 密集抖动（dwell 1–3 轮、大量 recovery），自 r304（7865 s）起稳定 normal。
- **通道读数（now）**：`slow: u=0.70 v=-0.014 pred(10T̂)=-0.44 E=1.32 r=2 θ=2.60 hits=64`；`prog: u=1.00 v=-0.007 pred(10T̂)=0.13 E=1.00 r=3 θ=1.01 hits=46`；`stall θ=0.82 hits=42`；err/deny 近零。
- **趋势（recent 20 / feature 16）**：u_prog 0.95→0.93（u_err≡0）；feature 序列 `0.901…1.000…0.707`（成功事件 1.000 与组合轮 0.8x–0.9x 交替）。
- **与 LIF 对照**：`temporal.recent(10)` 同域（normal／入域 r304）、u_prog 同量级（0.88–0.94）、T̂=16.0 s、err10=0 succ10=1——**两独立通道同面一致**。

### 4.2 资源面读数（如实延后）

- 载体 0.6.5 **不含 0bc 新代码** ⇒ 「CPU 去顶后墙钟」「commit 临限通知事件」「进程上限读数」三项**待下一载体重建后收取**（本批令含「不重建」）。
- 本会话运行侧事实（供下批对照基线）：全 run 正常完成、无 orz abort、无 OOM；journal 无 `degraded` 终态。

## §5 S4 判据对账（BACKLOG `0bc` 判据①–⑥）

| # | 判据 | 结论 | 证据 |
|---|---|---|---|
| ① | CPU 速率上限不再设置（读数面零出现） | 代码面 ✔；真机读数面待重建 | `default_job_limits` `cpu_rate_percent=None`（常量删除）；`ORZ_JOB_CPU_RATE_PERCENT` 仅可选档（1..=100） |
| ② | commit 越线只产生通知事件＋软提示，无 orz 侧硬拒 | ✔ | 通知设施真机测试（fires／silent／probe 双向）；`commit_limit_bytes=None`；派发头 drain→`host_resource_snapshot(trigger=commit_notification)`；软提示每 run 一次 |
| ③ | 活动进程上限受 env 覆盖且默认不变 | ✔ | `ORZ_JOB_ACTIVE_PROCESS_LIMIT`；默认 `2×cores+8`／≥16 不动；env 测试 2 条 |
| ④ | 构造性注入分配失败→降级＋终态（不 abort） | ✔ | journal 注入测试（43/43，含终态与 replay 校验）；ledger `try_reserve`；模型面 `AllocFailure`→run 终态 |
| ⑤ | 双平台构建与既有钉子不回归 | Windows 面 ✔（Linux 面待重建批） | 全链绿：loop 822/0/3、host 322/0/5（串行）、assurance 259/0、xai-tty-utils 32/32、orz-tools search_replace 113/0、fmt 干净、四包 check=0（`-j 2`） |
| ⑥ | 官方口径（`task.toml`／镜像／verifier／pin）零改动 | ✔ | 未动评测侧任何文件；manifest 重算随提交批；`assurance/run_event_journal_validation.py` 的同步属**产品内 journal verifier 镜像**（0bc「不新增事件族」的一致性改动），非评测口径面 |

## §6 RLI 组件可用性裁决（0bc 复合口径 · 任务指定末段检查）

### 6.1 组件具体情况（检查结果）

- 组件在 0.6.5 载体上**全程开启**（`ORZ_LIF_RLI_SHADOW=1`，本会话即观测场）且**零故障**：读数连续（步数 917）、持久（侧车）、跨压缩可查（PULL 面：now/recent/history/feature 四面均实读）。
- 数据面自洽：域机行为合理（早期探索期判 low_progress、稳定期 normal）；与 LIF 独立通道同面一致（§4.1，同域同量级）；err 面如实（`u_err≡0` 与全程无错误链一致）。
- 子面状态：`feature` 序列面可用（FR-5 后 enum 单源）；`history` 的 round/dwell 定位标注可用（0am 域级定位面）。

### 6.2 三判据读数（按 2026-09-20 用户令读法；本会话为零标签证据）

- **①消费率（低）**：全程**未在任务执行中自发拉取** RLI 面；仅在本轮收口检查时读取 4 次（now/recent/history/feature）。自述原因：a) 零注入面——无触发器提示「何时值得看」；b) 本任务为工程落码型，无「转向决策」点需要信号输入；c) `pred(10T̂)` 预测面不产出可执行建议（预测的是信号自身演化，不是「该干什么」）。
- **②转向相关（弱正向、定性）**：RLI 判 low_progress 的时段对应早期慢节奏探索轮，后段稳定 normal 对应高效落码期——定性对齐；因消费率≈0，本会话无「因 RLI 而转向」的因果实例可谈。
- **③域一致性（正向）**：自判 `normal` ↔ 框架实际（工具大多成功、无错误链）一致；与 0am 批既有读数（压力轴 95.6%／进度轴 82.2%／joint 80.6%）同向。

### 6.3 裁决（本组件对我的实际使用价值）

- **可用性**：✔ **实际可用**——机制运转、读数可核、跨压缩连续、与 LIF 独立同面。作为**状态自述面**（「我在哪个域、进度信号如何、何时卡过并恢复」），它提供的信息是我在长会话压缩断裂后**无法自行重建**的（我的上下文被折叠，RLI 记忆全 run）——本轮我实际使用了 `history` 的 dwell/迁移标注来回看全 run 节奏。
- **使用价值**：**有条件的、当前偏低**——在**收口／诊断／跨压缩定位**场景价值明确；在**任务执行中**价值未被兑现（消费率≈0：无触发器、无行动映射）。
- **建议**：**维持影子（默认关）不转正**；若要提升消费率，最小实验＝在**压缩窗口／硬提醒事件**处附带一行 `rli` 摘要（把「何时值得看」从模型自觉改为事件相关提示）——涉零注入纪律，需用户裁决。预测面若要真正可用，需先定义「信号→行动」映射（当前不存在）。

## §7 摩擦登记（本批实测）

1. **构建纪律 `-j 2`**（F14 相关）：默认并行 `cargo` 构建触发 commit 耗尽（cc `0xc000012d`／rustc `0xc0000409`）；`-j 2` 全批可用。本机 16 GB、commit 水位基线本就高（设计载 83–87%）。F14 状态更新＝「条件性可构建」。
2. **H1 硬提醒＋压缩窗口 ≥6 次触发**（本会话）：窗口内动作面被收窄（旧实现）＝FR-3 的现场样本；修复后窗口轮＝常规工具面（本批测试已绿）。
3. **env 假红三族**（均已在 `ORZ-ENV-POLLUTION-001` 案例登记＋脚本化处置）：ACAF 四键（0AM 批 19+30 例同族）／`ORZ_LIF_RLI_SHADOW`（rli 渲染测试首断言）／`GROK_HOME+GROK_AGENT`（grok_home 四测试）。清理列表集中到 `scripts/run_orz_tests.ps1`。
4. **round_inject_budget × 窗口测试交互**：默认 50 K 轮内注入预算下，fat read（≈150 K 结果）如实拒绝同轮后续调用（非缺陷）；相关测试用显式预算（`with_max_inject_tokens_per_round`）隔离。
5. **`--` 经 PowerShell 传脚本被吞**：`& script test … -- --test-threads=1` 会抛 `unexpected argument`；需直接调 cargo 或改脚本接口。
6. **遗留警告（HEAD 预置、非本批）**：`xai-tty-utils::process_alive` dead_code；`orz-tools types/resources.rs` 测试 `unused variable: journal`。
7. **设计 §11 裁决② 原语名修订（LOW→HIGH）**：真机探针实证（§3）；设计文档勘误待随提交批（`HOST_RESOURCE_OS_DELEGATION_DESIGN` §11-2 修订原语名＋通知方向注）。
8. **T1 硬截断 ×2**（本会话块 #19–24、#37–56 区段）：长会话机制如实落账（回放路径可用）。
9. **env 污染面比 0AM 批登记更广**：0AM §5 记 4 键，本批实测再增 3 键——案例文档已扩记。

## §8 遗留与下一步

- **S3 资源面新特性读数**（待下一载体重建）：CPU 去顶后墙钟／commit 临限通知事件（`trigger=commit_notification`）真机首现／进程上限读数／`ORZ_JOB_CPU_RATE_PERCENT` 档位冒烟。
- **Linux 编译面**：重建批 Docker 时核；重点核 `#[cfg(windows)]` 导出面与新增 `Win32_System_IO` feature 的非 Windows 分支（0z 有同类断裂先例）。
- **提交批**：manifest 重算、设计文档 §11② 勘误、TODO/BACKLOG 计数与勾选同步（本批不动）。
- **RLI**：维持影子；§6.3 的「事件相关提示」实验待用户裁决。
- 本批工作树：**未提交、未推送、未重建**（按任务令）。
