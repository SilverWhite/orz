# 官方 R3 未通过 20 题复跑记录（official-r3-unsolved20，2026-09-08/09）

> 日期：2026-09-08 排期启动，2026-09-09 全部收尾。
> 批次：`official-r3-unsolved20`（一题一作业；dna-assembly 保留整批轮有效结果）。
> 口径：**继承成绩**——不跑全量 89 题，只重跑 R2 后仍未通过的 20 题，看哪些新通过；
> k=1、不做多次尝试（2026-08-31 用户裁决延续）。
> 载体：orz 0.3.2 + 0q（orz `4dfb3d77`，父仓库 `d491253` 基线）Linux musl 三件套，
> 2026-09-08 18:09 构建（日志 `D:\tb-eval\orz-linux\build-20260908-0q.log`）；
> harbor 官方数据集 pin `terminal-bench/terminal-bench-2-1@sha256:7d7bdc1c…`；
> deepseek-v4-flash、eval_browser=true、官方墙钟唯一（无超时覆盖）、Docker 容器。

## 1. 背景与执行结构

- R1（89 题官方全量）：58 通过（≈65.2%）、31 未通过。
- R2（31 题失败集多波次重跑）：档案逐题 best-reward 计 11 题曾通过；2026-08-31 最新波次
  登记口径为已解 9（剩余未解约 22，含现行 9 题错题集）。20 题"从未通过/最新波次未解"
  为本轮对象。
- 2026-09-08 用户裁决：不跑全量，按继承成绩口径跑 20 题未通过集；消耗优先。
- 启动方式演进（过程事实，执行器修复见 §4）：
  1. 整批 c1（10 题/作业）两次因网络抖动中止（Docker Hub 大镜像拉取 EOF、注册表
     ConnectError），整批重跑会把已完成题归零 → 用户裁决改**一题一作业**；
  2. dna-assembly 在整批轮已取得有效官方结果（reward 0.0 / AgentTimeout），按 k=1
     纪律保留、不再重复跑；
  3. 其余 19 题由
     [`run_r3_unsolved20_per_task.ps1`](../../scripts/run_r3_unsolved20_per_task.ps1)
     串行执行：每题独立 harbor job（`official-r3-unsolved20-<task>`）、完成即跳过、
     无效/失败题独立重试至多 3 轮。

## 2. 最终成绩：5/20 新通过

### 新通过（reward 1.0，5 题）

| 任务 | 结束方式 | 起 | 止 |
|---|---|---|---|
| count-dataset-tokens | 通过 | 09-08 20:05 | 09-08 20:25 |
| mteb-leaderboard | 通过（R2 两轮均 3600s 超时） | 09-09 01:58 | 09-09 02:59 |
| raman-fitting | 通过 | 09-09 04:27 | 09-09 04:37 |
| tune-mjcf | 通过 | 09-09 05:46 | 09-09 05:57 |
| write-compressor | 通过 | 09-09 05:57 | 09-09 06:05 |

### 仍未通过（reward 0.0，15 题）

| 任务 | 结束方式 | 起 | 止 |
|---|---|---|---|
| dna-assembly（整批轮保留） | AgentTimeoutError | 09-08 18:55 | 09-08 19:30 |
| adaptive-rejection-sampler | AgentTimeoutError | 09-08 19:44 | 09-08 20:04 |
| dna-insert | 自然结束 | 09-08 20:25 | 09-08 20:48 |
| extract-elf | AgentTimeoutError | 09-08 20:48 | 09-08 21:10 |
| extract-moves-from-video | AgentTimeoutError | 09-08 21:10 | 09-08 22:00 |
| filter-js-from-html | 自然结束（第三次为有效试次） | 09-08 23:33 | 09-08 23:59 |
| gpt2-codegolf | AgentTimeoutError | 09-09 00:24 | 09-09 00:42 |
| path-tracing | AgentTimeoutError | 09-09 02:59 | 09-09 03:31 |
| path-tracing-reverse | AgentTimeoutError | 09-09 03:31 | 09-09 04:02 |
| protein-assembly | 自然结束 | 09-09 04:02 | 09-09 04:26 |
| train-fasttext | AgentTimeoutError | 09-09 04:37 | 09-09 05:45 |
| gcode-to-text | AgentTimeoutError（第二轮补跑） | 09-09 06:05 | 09-09 06:22 |
| make-doom-for-mips | AgentTimeoutError（第二轮补跑） | 09-09 06:22 | 09-09 06:39 |
| make-mips-interpreter | AgentTimeoutError（第二轮补跑） | 09-09 06:39 | 09-09 07:12 |
| model-extraction-relu-logits | 自然结束（第二轮补跑） | 09-09 07:12 | 09-09 07:22 |

**新通过（5）**：count-dataset-tokens、mteb-leaderboard、raman-fitting、tune-mjcf、
write-compressor。

**仍未通过（15）**：adaptive-rejection-sampler、dna-assembly、dna-insert、extract-elf、
extract-moves-from-video、filter-js-from-html、gcode-to-text、gpt2-codegolf、
make-doom-for-mips、make-mips-interpreter、model-extraction-relu-logits、path-tracing、
path-tracing-reverse、protein-assembly、train-fasttext。

## 3. 与 R2 对比

- R2 后"从未通过/最新波次未解"约 20–22 题 → R3 后剩余 **15 题未通过**（档案无通过记录
  口径：原 20 题中 5 题翻案）。
- 值得注意：mteb-leaderboard（R2 连续两轮 3600s 墙钟超时）本轮通过；train-fasttext
  （同为 3600s 长题、R2/W2 多轮未过）本轮仍 AgentTimeout。
- 未通过画像：多数为撞官方墙钟（AgentTimeoutError）；dna-insert / filter-js /
  model-extraction / protein-assembly 为自然结束但 verifier 不通过。

## 4. 执行器与基建修复（过程登记）

1. `docker pull`/注册表网络抖动处置：镜像拉取改并发 1 + 每试次重试 3；注册表域名
   （hub.harborframework.com）直连/代理双路径验证；中断整批目录归档
   （`*-aborted-20260908*` / `*-batch-interrupted-20260908`）。
2. 执行器启动兼容：Windows PowerShell 5.1 原生参数 JSON 内嵌引号被破坏
   （`JSONDecodeError at char 2`）→ 改 PowerShell 7 启动（`da3c2a1`）。
3. 完成判据修正：harbor exit 0 会掩盖 AgentSetupTimeout 等基建错误（filter-js
   AgentSetupTimeout 72 分钟无效试次教训）→ 判据改为 job result.json 真实 schema：
   `finished_at` 非空且 `stats.evals[*].reward_stats.reward` 有非空数值键
   （`045f91a`）。
4. 同名 job 重跑陷阱：harbor 会把既有错误试次直接计入而不重新执行 → 重跑前将旧作业
   目录移存为 `<job>-stale-<ts>`（`d77c986`）；gcode/make-doom/make-mips-interpreter/
   model-extraction 首轮无效、第二轮补跑成功，验证修复生效。
5. 镜像预拉：11 个缺失镜像按各自 task.toml `docker_image` 字段预拉（tag 不统一：
   mteb `:20260430`、protein-assembly `:20260403`，其余 `:20251031`），完成后
   20 题镜像全在本地（`ALL_20_IMAGES_PRESENT`），夜间跑批不再依赖 Docker Hub。
6. 孤儿容器清理：整批中断遗留 gpt2-codegolf env 容器（挂起约 1 小时）强制移除。
7. 用户 2026-09-08 深夜关闭 Clash 系统代理；此后未再出现 registry 级整批中断
   （观察性登记，非因果结论）。

## 5. 过程观察（细节分析待续，仅登记事实）

- dna-assembly 轮 journal 实证：DeepSeek 流式通道抖动（zero-chunk/idle/
  midstream decode error、`stream interrupted after 0 re-sends`）；检索子代理
  600s wallclock 耗尽以结构化 error 信封回传（`retrieval subagent wallclock
  exceeded` + `retrieval_close_record terminal_reason=subagent_timeout`），
  模型随后重开检索激活——超时均有回传，卡顿源于失败-重试循环消耗预算。
  素材供 0d 后续 3/5 与检索判据分析。
- filter-js-from-html 前两轮 AgentSetupTimeout（第一轮 72 分钟）属基建/网络形态，
  第三轮有效（26 分钟自然结束 reward 0.0）。
- 每任务约 1.1–1.5M 原始模型 token（缓存命中占大头），计费口径约 12–24 万/题，
  节奏 ~3 题/小时，未见异常消耗。

## 6. 证据边界

- 逐题结果：`D:\tb-eval\jobs-official\official-r3-unsolved20-<task>\result.json`
- gsa 卷：`D:\tb-eval\gsa-volumes\official-r3-unsolved20-<task>\`
- 控制台/摘要：`D:\tb-eval\jobs-official\official-r3-unsolved20-*-console.log`、
  `official-r3-unsolved20-per-task-summary.log`
- 配置：`scripts/configs/official-r3-unsolved20-c1/c2-config.json`
- 执行器：`scripts/run_r3_unsolved20_per_task.ps1`
- 中断/旧目录归档：`D:\tb-eval\jobs-official\official-r3-unsolved20-*-aborted-*`、
  `*-stale-*`、`*-batch-interrupted-*`

## 7. 后续

- 细节分析（每题失败归因、轮次/token/检索观察、与新通过题的机制关联）另行产出。
- 结果登记：BACKLOG 0s（本批条目）。
