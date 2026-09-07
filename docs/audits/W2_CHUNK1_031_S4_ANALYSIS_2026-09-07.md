# W2 批次 chunk1（w2-chunk1-031）事件面分析（2026-09-07）

> **上级**：[`S3/S4 集中实机验证批排期 §4.2/§7 C`](../LIVE_VERIFICATION_BATCH_SCHEDULE_2026-09-07.md)
> / BACKLOG 0o / TODO P0-0o T3。
> **定位**：批次 W2 第 1/3 批的实机事实与判据核对记录；正式闭合登记仍在
> T6 统一收口落各条目小节。Windows 墙下摩擦/事件诊断，非 TB2.1 官方计分
> （§16.8 口径）。
> **载体**：orz 0.3.1（`d21b883e`）+ Windows 三件套（VM Program Files）
> + high-nist 臂（non-admin + LOW IL + Job + TEMP 重定向 + egress
> allowlist 221.204.163.76 + `--no-appcontainer`）+ `ORZ_F6_PUSH=on`
> + 官方 agent_timeout_seconds 唯一墙钟（900/900/3600）。

## 1. 执行事实

- 3/3 任务 `attempt=ran / orz_exit=0 / observation=compliant`，journal 齐
  收，`bootstrap_ok=True`、`secret_files_left=0`、零 runner errors。
- 证据：`_windows_high_nist/formal-2026-09-02/evidence-agent-tb2.1-w2-chunk1-031-high-nist/`
  （3 journal + 3 run-observation + baseline JSON）。
- 首跑作废件：GBK 注释吞换行致 `$workdir` 空值 + elev 退出码假绿，双修复
  后重跑（父仓库提交 `173ade6`；客机作废日志
  `elev-job-d81a54fad889.log`，有效日志 `elev-job-d010a2afc871.log`）。

## 2. 逐任务结果

| 任务 | 墙钟 | 事件/轮次 | 工具时长 | 结局 | 关键事实 |
|---|---|---|---|---|---|
| make-doom-for-mips | 900s 官方 | 816 事件 | 112 调用/214s（最长 99.9s） | **run_finished @862s** | 存活越过 840s 旧硬杀线=840 硬杀确除；submit 双阶段 |
| gcode-to-text | 900s 官方 | 671 事件 | 77 调用/57s（最长 13.1s） | run_invalidated(wallclock) @900s | 官方墙钟唯一；工具极快、耗时在模型面（73 model_output） |
| mteb-leaderboard | 3600s 官方 | 173 事件 | 24 调用/256s（150.9s+89.6s 两长调用） | **run_finished @409s** | 答案 `Qwen/Qwen3-Embedding-8B`（≠0303 基线 Snowflake/arctic-embed2-v1.5，web 阻断下参数化知识路径方差，入 0l ⑥ 记账） |

对比基线 chunk1-0303（0.3.0 pre-TER）：make-doom 654 事件/77 轮死于 840s、
gcode 404 事件/44 轮死于 840s、mteb 348s 完成——本批 TER 形态（去硬杀 +
官方墙钟唯一）直接改变 make-doom 结局（完成 vs 被杀）。

## 3. 判据核对（排期 §7 C 组）

- **T3.1（gcode）**：部分达成+边界——本轮最长前台命令仅 13.1s，无 ≥180s
  触发 auto-bg/idle-kill 场景，`tool_running` 真实模型面未发生（机制断言
  已由批次 W1 fake 场景覆盖）；官方墙钟收尾形态验证（`run_invalidated`
  payload `status=wallclock`，无 840 派生硬杀）。完成提醒文本可达性继续
  挂起（无后台化即无提醒面），随 chunk2/3 观察或顺延。
- **T3.2（make-doom）**：核心达成——`--timeout` 900 官方唯一墙钟
  （DryRun `max_wallclock_arg=none`），run 存活 862s>840s 并
  run_finished=840 硬杀除名的决定性实证。连通探测：2 条多目标
  PowerShell 探测（github/musl.cc/dl.google/ziglang 等）各 12.5s/12.6s，
  折合 ≈2–3s/目标（墙 DROP 模式；≤2s/目标为理想口径，登记边界）。
  blackboard `section=processes` kill 面模型侧未动用（未观测，非失败）。
- **T3.3（mteb）**：达成+边界——HF 探测（Invoke-WebRequest HEAD
  huggingface.co，TimeoutSec 15）**12.5s 判定失败**（≤2s 理想未达，
  DROP 模式下 SYN 重传决定；零泄漏，模型随后改走 DeepSeek API 参数化
  路径）。F6 push：make-doom/gcode 各 `budget_cue_injected`=3（payload
  `remaining_seconds`/`threshold_seconds=600` 逐条可审），mteb=0（3600s
  预算 remaining 始终 >600s，按阈值设计零触发=设计自洽非缺失）；≤4 判据
  全过。
- **T3.4（W-F13）**：达成——make-doom 对 vm.js 的 read_file 调用
  **0 次**（0303 基线 20+ 轮）；长输出经终端日志面。
- **通用统计（0d 后续 3/4、0j W1-R1 journal 半边）**：零真实 400（无
  错误族事件）、零哨兵触发（无降级族事件）、无 transport_retry 需求
  （断连未复现→按排期 §1 边界观察登记顺延）；缓存命中率（provider
  口径）**96.82%**（hit 6,073,472 / miss 199,777）≥90%。

## 4. P2-11 三项 S4 证据（顺带）

- **依赖图 D6**：`dep_graph` 事件字段随 ToolCompleted 落 journal 实证
  ——make-doom read 边 47 / write 边 3，gcode read 6，mteb read 5 /
  write 1；read 边带 anchor（sha256/size/mtime），write 边带
  new_anchor；4 条 write 边均为新建文件（consumed_read=null 语义自洽）。
  `output_encoding` 字段同链可见。`section=deps`/增量头属响应面，
  journal 侧仅证 section 字段在链（blackboard_read 16/12/6 次）。
- **F6 push cue**：见 T3.3。
- **retryable 位分布**：本轮零错误信封，无分布样本（观察登记）。

## 5. 观察与缺口（非阻塞）

1. egress DROP 模式下探测失败时延 12.5s（非 ≤2s）：fail-fast 理想依赖
   REJECT 语义，属墙形态边界，不作 TER T3.2/T3.3 阻塞项。
2. mteb 答案跨 run 方差（Qwen3 vs Snowflake）：TB2 verifier 口径的
   reward 判定归 0l ⑥ 记账，本批仅登记事实。
3. gcode 900s 模型面耗尽（57s 工具 vs ~840s 模型时间）：模型侧节奏
   观察，随 chunk2/3 与 0l ⑦ §7 判据 1–6 统一分析。
4. chunk1 真跑前的 hollow-DryRun 教训：DryRun 对空路径插值零 errors
   （已随 `173ade6` 修复退出码传播），后续批次 DryRun 需抽查
   `--workspace/--output` 插值非空。
