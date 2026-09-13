# TB 2.1 V4.1 代际新一轮 · 第 0 轮（内存重题前置轮）起跑记录（2026-09-13）

> 类型：**起跑记录**——口径确认 + 起跑身份 + 起跑前快照 + 判据与进度栏位；
> 跑批结果（成绩、摩擦清单、资源遥测）在本文件 §6 追加，不另立文档。
> 口径来源（不回改）：[`容器侧排查与起跑口径`](TB21_V41_EVAL_CONTAINER_AUDIT_2026-09-13.md) §5；
> [`排期 §3.2/§3.3`](../TB21_V41_GENERATION_ROUND_SCHEDULE_2026-09-13.md)。
> 入口：索引 `EVAL-TB21-ROUND0-START`；BACKLOG 0b / TODO P0-0b；复现入口
> [`scripts/run_r0_heavy_official.py`](../../scripts/run_r0_heavy_official.py)。

## 1. 口径归属（用户裁决，2026-09-13）

- **第 0 轮计入本轮 89 题口径**（用户裁决：「本轮计入 89 题」）。
- 后续 1–5 批据此**显式剔除这 8 题**：16 / 15 / 15 / 17 / 18 = **81 题**；
  **81 + 8 = 89**，每题恰好 1 次试次。
- **分批偏离登记的落点**：冻结清单的批次划分（16/17/19/18/19）是**语料身份记录**，
  不是执行计划，**保持原值不改**；执行侧改走「逐题 `-i` 列题」，**不动冻结 runner**
  （`D:\tb-eval\run_official_2.1.sh`，其 `K="${K:-5}"` 本轮取 `K=1`）。冻结清单
  `evaluation/corpus-freeze/tb21-official-89-2026-09-13.manifest.json` 的 `batch_split`
  与 `runner_script` 摘要**均不动**。
- 备选（第 0 轮仅作探索性前置、1–5 批仍跑满 89）**未被采纳**，登记为未选路线。

## 2. 题集（8 题 = `memory_mb == 8192` 全集，冻结清单实测量）

| 任务 | 冻结批次 | 内存 | CPU | agent 超时 | 镜像（压缩体积） |
|---|---|---|---|---|---|
| `mcmc-sampling-stan` | B2 | 8192 | 4 | 1800 s | `alexgshaw/mcmc-sampling-stan:20251031`（227 MB） |
| `gpt2-codegolf` | B2 | 8192 | 1 | 900 s | `alexgshaw/gpt2-codegolf:20251031`（586 MB） |
| `mteb-leaderboard` | B3 | 8192 | 1 | 3600 s | `alexgshaw/mteb-leaderboard:20260430`（633 MB） |
| `caffe-cifar-10` | B3 | 8192 | 4 | 3600 s | `alexgshaw/caffe-cifar-10:20260403`（203 MB） |
| `torch-pipeline-parallelism` | B3 | 8192 | 1 | 900 s | `alexgshaw/torch-pipeline-parallelism:20251031`（28 MB） |
| `rstan-to-pystan` | B3 | 8192 | 4 | 1800 s | `alexgshaw/rstan-to-pystan:20251031`（49 MB） |
| `torch-tensor-parallelism` | B4 | 8192 | 1 | 900 s | `alexgshaw/torch-tensor-parallelism:20251031`（28 MB） |
| `filter-js-from-html` | B5 | 8192 | 1 | 1800 s | `alexgshaw/filter-js-from-html:20251031`（360 MB） |

- 拉取成本：8 个镜像压缩合计 **≈2.06 GB**；按实测落盘比外推 ≈6–8 GB（**按需拉取**，
  不预拉——用户裁决）。
- 最坏墙钟（agent 侧求和）≈ **4.25 h**（15,300 s），串行、未计 verifier 与镜像拉取。
- 更正留痕：本轮题数此前口头报「6 题」系截断表之误，实测量为 **8 题**（2026-09-13）。

## 3. 起跑身份（代际记录，排期 §2）

| 件 | 身份值 |
|---|---|
| 载体 `D:/tb-eval/orz-linux/orz`（0.5.0 musl 静态） | `393eee34dd0357cab088f289fa1068a39294a13fff48d80d33c50a40684dd623` |
| 适配器 `D:/tb-eval/tb_agents/orz.py` | `2737cfadc5c43603b73164b51343e58a671c0efeee8d9ab7a626dda8dae51490` |
| 数据集 pin | `terminal-bench/terminal-bench-2-1@sha256:7d7bdc1cbedad549fc1140404bd4dc45e5fd0ea7c4186773687d177ad3a0699a` |
| 模型 | `deepseek-v4-flash`（路由名；代际以账面时点标注，见排期 §2） |

- 起跑器内建**代际身份硬门**：载体或适配器哈希与上表不符即**中止**（除非显式
  `--allow-identity-drift`）——避免产出第二代人不可比的数据（`ORZ-VERDICT-EPOCH-001` 同族纪律）。

## 4. 起跑前快照（15:29:22）

- 宿主 `D:` 可用 **33.25 GiB / 总 276.63 GiB**（清空镜像后；VHDX 稀疏，盘内归还部分
  在后续拉取时先被吃掉）。
- Docker：**镜像 0 个、容器 0 个**（清空态；审计 §7 判据 1 成立）。
- 时间线：15:29:22 起跑器启动并过身份门 → 15:29:29 作业目录与首试次
  `torch-tensor-parallelism` 建立 → 15:31:01 首份会话卷运行目录
  `RUN-CLI-6aa65135` 落 `gsa-volumes/official-r0-heavy/`。

## 5. 运行形态（官方口径 + 降并发）

- `-k 1`（每题 1 试次）、**`-n 1`（串行）**——内存重题集恰是内存临界集，并发只放大风险
  并污染归因；改为 `-n 2` 需同批登记（本轮未改）。
- 其余保持官方口径：不加时间倍率、不传 `max_wallclock`、无 `--max-retries`（官方默认 0）、
  `--upload --public`。
- 单作业承载 8 题：`--job-name official-r0-heavy`、`-o D:/tb-eval/jobs-official`；
  卷 `D:/tb-eval/gsa-volumes/official-r0-heavy` 经 `--mounts` bind 到容器 `/orz-gsa`。
- **窗口纪律（用户裁决）**：本窗口内**不做其他内容**——不启模拟器、不跑安卓支线、
  不并行容器构建（09-12 死机形态的成因是自身动作叠加）。
- 静默旁路读数件（`tb21_round_gate.py` / `tb21_friction_scan.py`）**不接入本跑批链条**，
  由人工在过程与结束后另行读取（契约：只读、容忍半程、退出码恒 0）。

## 6. 判据与进度

判据（审计 §7，可机械核对）：

1. 起跑前 `docker images -q` 计数 = 0 —— **成立**（§4：images=0、containers=0）。
2. 8 题全部落在 `jobs-official/official-r0-heavy`，每题 1 试次、`-n 1`（作业元数据可核）。
3. 卷 `gsa-volumes/official-r0-heavy/<run_id>/` 数量与题数一致（8）。
4. 本批镜像清理动作一条（跑完后执行，先报用户）。
5. 本轮报告不把容器内磁盘读数当作宿主余量证据。

**进度（滚动追加）**

| 时点 | 事件 |
|---|---|
| 15:29:22 | 起跑：身份门通过、清空态快照入账、harbor 作业启动 |
| 15:29:29 | 首试次 `torch-tensor-parallelism` 建立（作业目录 + 试次目录） |
| 15:31:01 | 首份会话卷运行目录 `RUN-CLI-6aa65135` 落卷（agent 已在容器内工作） |
| 15:31:37 | 卷内落 `process_trees/call_00_…json`、`session/terminal/call_00_….log`（进程树与终端面在位） |
| 15:32:40 | 卷内落 `resources_state.json`——**更正（18:20 回查）**：该件是**工具面状态/参数 sidecar**（`grok_build.*` 工具参数与 `ReportedTaskCompletions`），**不是 0z 主机资源遥测面**；早前把它读作「0z 资源面在位」是误读，0z 资源面的真实情形见 §6.1 |
| 15:34:01 | `events.jsonl` 34 → 70 行（agent 持续工作）；作业/试次日志为适配器 POSIX-sh 包装 + `--real --allow-write --allow-shell --max-tool-rounds 999 --allow-network --retrieval-mode local_browser` |
| 15:34 | 容器实测：`mem_limit=8 GiB`（8,589,934,592 B）**高于** Docker VM 总内存 **7.677 GiB**（8,243,064,832 B）、`memswap=16 GiB`、`nano_cpus=1.0`；串行 `-n 1` 下同时仅 1 个容器 ⇒ 审计 §2.4 的内存形态在真机确认（这正是"降并发 + 重题前置"的依据） |
| 15:34 | 宿主视角 `docker info`：12 CPU / 1 镜像 / 1 容器；镜像按需拉取首个完成（`alexgshaw/torch-tensor-parallelism:20251031`） |

### 6.1 中途体检（18:20；作业仍在跑，8 题中 7 题已出结局）

**结局分布：4 × AgentTimeoutError ＋ 2 × RuntimeError（镜像拉取）＋ 1 × 正常完成（reward 0）；
第 8 题 `rstan-to-pystan` 运行中。** 逐题一手情形：

| 试次 | 用时 | 账面结局 | verifier 侧实际情况 | 归因 |
|---|---|---|---|---|
| `torch-tensor-parallelism` | 31.7 min | AgentTimeout（agent 用满 900 s）＋ verifier 阶段 900 s 超时 | **13/13 全过**（`verifier/reward.txt` = 1，51.5 s 跑完） | **题目实际已解出**，只是用满官方墙钟；与 R1 同题（rate 1.0 ＋ AgentTimeoutError）同形 |
| `mteb-leaderboard` | 61.9 min | AgentTimeout（3600 s） | 2 failed：`/app/result.txt` 不存在 | agent 侧未产出（R1 同形） |
| `torch-pipeline-parallelism` | 25.7 min | AgentTimeout（900 s） | verifier 依赖下载失败：`nvidia-cusparse-cu12` **network timeout**（`UV_HTTP_TIMEOUT=30s`），torch 轮子 825 MB 未下完 | **装置侧网络** ⇒ reward 0 不代表 agent |
| `gpt2-codegolf` | 18.2 min | AgentTimeout（900 s） | 1 failed：`/app/gpt2.c` 不存在 | agent 侧未产出（R1 同形） |
| `mcmc-sampling-stan` | 20.7 min | 正常完成，reward **0** | verifier 自身 `curl: (18) Transferred a partial file` 拉 `uv` 失败 ⇒ `/root/.local/bin/env: No such file` ⇒ `uvx: command not found`，**测试根本没跑** | **装置侧网络**；R1 同题 **reward 1.0** ⇒ 本试次 reward **无效** |
| `caffe-cifar-10` | 0.6 min | RuntimeError | 镜像层 `short read: expected 183436527 bytes but got 136456798: unexpected EOF` | **装置侧**（Docker Hub 拉取中断） |
| `filter-js-from-html` | 0.2 min | RuntimeError | 镜像 `registry-1.docker.io … manifests/sha256:92acda0f…: EOF` | **装置侧**（Docker Hub 拉取中断） |
| `rstan-to-pystan` | 运行中 | —（18:09 起，1800 s 上限） | — | — |

**硬发现 1 — 出网路径在大文件传输上不稳（装置侧；本轮最大干扰项）**

- 同一窗口出现三类中断：Docker Hub 镜像层 EOF / short read（2 题完全没跑起来）、
  GitHub 释放包 partial file（`uv`）、PyPI 大轮子 network timeout（torch 825 MB ＋ CUDA 系列）。
- 波及面：**2 题未起跑 ＋ 1 题 reward 被判 0（R1 同题曾 1.0）＋ 1 题 verifier 未能运行** ⇒
  **本轮成绩轴被污染**，不可按"通过率"直接读；须以"逐题实际情形"归因。

**硬发现 2 — 0z 资源面在 orz 内部被丢弃（载体侧，真缺口候选）**

- 一手证据：`orz_host` 报 `host resource probe installed (0z S1) headroom=… tier=normal`，
  但 `orz_loop::host_exec` 反复 WARN：
  `unknown host resource fact kind; dropped (audit-face loss) kind="host_resource_snapshot"`——
  共 **13 次**，落在 **6 个试次**的 agent 日志里（`torch-tensor-parallelism` 7 次、
  `rstan-to-pystan` 2 次、其余各 1 次）。
- 后果：6 个 run 的 journal 事件类型统计中，**`host_resource_snapshot` 一次都没有**；
  资源族只落了 **`reclaim_performed` ×1**（payload：`budget_bytes=8589934592`、`class=cache`、
  `tier=soft`、`outcome=rejected`、`paths=["/app/__pycache__"]`、`freed_bytes=0`）。
- 即：本轮要校验的 0z 遥测面**在 orz 内部被丢在管道上**（自述 `audit-face loss`）；
  回收阶梯本身有真实动作记录（1 次，且**被拒**、回收 0 B）。
  **回查纪律**：本条按 `ORZ-VERDICT-EPOCH-001` 先回查再裁决——是否属设计内（例如快照走
  sidecar 面）还是实现偏误，须对 0.5.0 源码 + ADR-0010 §0z 段逐条核对后再定性。
- 另 1 条小口径：`resources_state.json` 是**工具面状态/参数 sidecar**（`grok_build.*` 参数
  ＋ `ReportedTaskCompletions`），早前把它当作 0z 资源面是误读，已在 §6 更正。

**与 R1 的代际对照（同 k=1、同 8 题）**：R1 结局 = `mcmc-sampling-stan` **1.00**、
`torch-tensor-parallelism` **1.00**（同样伴 AgentTimeoutError），其余 6 题为 0 / 未产出或
AgentTimeout（**R1 已有 6/8 命中 AgentTimeout**）。⇒ **"重题集用满官方墙钟"不是新回归**；
本轮新增的差别主要是**出网不稳**（R1 期未出现镜像拉取中断）。

## 7. 复现入口与产物位置

- 起跑器：[`scripts/run_r0_heavy_official.py`](../../scripts/run_r0_heavy_official.py)
  （`scripts/LIFECYCLE.md` 登记为 `active`；`--dry-run` 打印解析后的 argv 与前置，
  不触碰装置）。
- 命令等价形式：审计 §5.3（逐题 `-i` 显式列表 + `-k 1 -n 1 --upload --public`）。
- 过程证据：`D:/tb-eval/jobs-official/official-r0-heavy-round.log`（起止与快照）、
  `official-r0-heavy-console.log`（harbor 会话输出）、
  `official-r0-heavy/`（作业与逐题产物）、`gsa-volumes/official-r0-heavy/`（会话卷）。
- **过程观测口径（实操）**：harbor 的 `*-console.log` 在非 TTY 下**缓冲**（起跑 5 分钟时仅 30 B），
  故跑批中的体检以**会话卷**为主（`events.jsonl` 行数递增、
  `process_trees/`、`session/terminal/`、`resources_state.json` 落卷时间戳）加上试次目录
  （`*/trial.log`、`*/agent/`、`*/artifacts/`）；作业级 `result.json` 与 `job.log` 在收尾时才成形。

## 8. 边界与不做项

- 本轮为 **k=1 筛查轮**：可上传 Harbor，**不构成榜单提交**（榜单 CI 要求每题 ≥5 试次）；
  对外只表述为「官方流程 + 单次尝试的新代际基线扫描」。
- 分数只作参照：达 90.6（官方 V4.1 Flash 公布值）才值得补 k=5；未达不阻断本轮，
  本轮首要产出是 orz 自校验与摩擦项。
- 容器内磁盘读数（`volume … free of …`）只反映盘内视角，**不作宿主余量证据**（审计 §2.3）。
- `D:\AGI` 为不可触碰禁区；本轮产物、缓存与可达范围止于 `D:\CLI` 与 `D:\tb-eval`。
