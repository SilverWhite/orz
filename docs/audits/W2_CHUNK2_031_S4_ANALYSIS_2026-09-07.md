# W2 批次 chunk2（w2-chunk2-031）事件面分析（2026-09-07）

> **上级**：[`S3/S4 集中实机验证批排期 §4.2/§7 C`](../LIVE_VERIFICATION_BATCH_SCHEDULE_2026-09-07.md)
> / [`chunk1 分析`](W2_CHUNK1_031_S4_ANALYSIS_2026-09-07.md) /
> [`v4-pro 路由判定`](W2_PRO_MODEL_ROUTING_VERDICT_2026-09-07.md)。
> **批次状态**：chunk2 完成后按用户指示**暂停**（chunk3
> filter-js-from-html / model-extraction-relu-logits / dna-insert 待后续
> 指示）；chunk2 客机工作区数据保留不清。
> **载体**：同 chunk1（orz 0.3.1 + high-nist 臂 + F6 push on + 官方墙钟
> 唯一：1800/3600/900）。

## 1. 执行事实

- 3/3 任务 `attempt=ran / orz_exit=0 / observation=compliant`，
  `bootstrap_ok=True`、`secret_files_left=0`、零 runner errors。
- 证据：`_windows_high_nist/formal-2026-09-02/evidence-agent-tb2.1-w2-chunk2-031-high-nist/`
  （3 journal + 3 run-observation + baseline JSON +
  `artifacts-adaptive/`）。

## 2. 逐任务结果

| 任务 | 墙钟 | 事件 | 工具 | 结局 | 关键事实 |
|---|---|---|---|---|---|
| path-tracing | 1800s | 957 | 109 次/全短（最长 10s） | run_invalidated(wallclock) @1800s | 工具极快、模型面耗时形态（同 gcode），撞官方墙钟收尾 |
| train-fasttext | 3600s | 2471 | 283 次（最长 34s） | **run_finished @3443s**，submit 两阶段（requested→confirmed） | 本批最大会话；deliverable 随 submit 确认 |
| adaptive-rejection-sampler | 900s | 395 | 42 次 | **run_finished @889s**（压线 11s） | 40 轮；final-answer 收尾（无 submit 调用）；交付 `ars.R`（35KB）+ 两个样本文件（已拉回宿主 `artifacts-adaptive/`） |

## 3. 新发现并修复：App junction 穿透删除（工件证据毁灭缺陷）

- **现象**：batch 收尾后 path-tracing / train-fasttext 工作区被整个清空
  （连 runner 预置 instruction.md/task.json/.gsa 都没了），仅最后任务
  adaptive 完好。
- **根因**：`run_agent_arm.ps1` `Reset-AppJunction`（F2，2026-09-03
  引入）在重指向 junction 前执行 `Get-ChildItem C:\app -Force |
  Remove-Item -Recurse`——此刻 C:\app 仍指向上一个任务的工作区，递归
  清空**穿透 junction 删除前一任务的全部文件**。每个任务切换即毁灭前
  一任务工件；chunk1 的 make-doom/gcode（及更早批次）同理在批内即毁，
  chunk1 工作区之后才被 chunk2 开跑的 F9 跨批清理整体删除（两机制
  叠加，先前误判证据毁于 F9，实为 junction 先毁）。
- **影响面**：journal/run-observation 每任务收尾即回收宿主，**判据面
  证据无损**；毁的是工件级核验面（make-doom ELF / path-tracing 渲染
  图 / train-fasttext 模型文件不可再核）。
- **修复**：`Reset-AppJunction` 分支处理——ReparsePoint 只 `rmdir` 删
  链接本身，真目录（legacy F2 fallback）才清内容。已入本提交；chunk3
  及后续多任务批生效。

## 4. 判据核对（排期 §7 C 组顺带）

- **T3.1（tool_running/auto-bg）**：本批仍无 ≥180s 前台命令（最长
  34s），真实模型面未触发后台化/idle-kill；机制断言继续由批次 W1
  fake 覆盖，提醒可达性继续挂起。三任务累计 6/9 题无长前台命令——
  该判据在真实面能否触发取决于任务形态，chunk3 前评估是否保留。
- **T3.2（墙钟唯一）**：path-tracing 存活至官方 1800s 整并被
  `run_invalidated(wallclock)` 收尾（无 840 派生硬杀）——与 chunk1
  gcode/make-doom 一致，结构性判据三题复证。
- **T3.3（F6 push）**：path-tracing cue 3 / train-fasttext 2 /
  adaptive 3，payload（remaining_seconds/threshold 600s）逐条可审；
  ≤4 判据全过。
- **通用统计**：零真实 400、零哨兵、零 transport_retry（断连/解码
  错误仍未复现→观察登记顺延）；缓存命中率 **96.81%**
  （hit 16,617,984 / miss 547,598）——chunk1+chunk2 两批口径均 ≥90%。

## 5. v4-pro 全量对账（闭环）

chunk2 三任务 journal 的 tool_calls **零** `api.deepseek.com`/`v4-pro`
引用——平台 9/7 的 4 次 v4-pro 请求**全部归属 chunk1 mteb**（2 条
chat 自调 + 重试/拆分；详见路由判定文档）。9/4 单峰早于本批。orz 主
车道 = v4-flash（源码 + env + 流量三方证据）结论不变。

## 6. 观察与缺口（非阻塞）

1. adaptive `run_finished(turn_count=1, tool_rounds=40)` 的
   turn_count 语义（=1 与 40 轮并存）待核，疑为跨 run 计数口径，
   归 0l ⑦ 统一分析。
2. path-tracing 两轮（0303 与本批）均撞墙钟未出图：该题在 Windows
   墙 + 该模型下可能属「模型面耗时型」任务，与 gcode 同类，记账时
   标注。
3. 工件级证据改进项（chunk3 前）：runner 收尾增加任务根目录工件清单
   快照（文件名+大小入 baseline JSON），避免再依赖批后人工取证。
