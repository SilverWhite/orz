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

## 7. 复现入口与产物位置

- 起跑器：[`scripts/run_r0_heavy_official.py`](../../scripts/run_r0_heavy_official.py)
  （`scripts/LIFECYCLE.md` 登记为 `active`；`--dry-run` 打印解析后的 argv 与前置，
  不触碰装置）。
- 命令等价形式：审计 §5.3（逐题 `-i` 显式列表 + `-k 1 -n 1 --upload --public`）。
- 过程证据：`D:/tb-eval/jobs-official/official-r0-heavy-round.log`（起止与快照）、
  `official-r0-heavy-console.log`（harbor 会话输出）、
  `official-r0-heavy/`（作业与逐题产物）、`gsa-volumes/official-r0-heavy/`（会话卷）。

## 8. 边界与不做项

- 本轮为 **k=1 筛查轮**：可上传 Harbor，**不构成榜单提交**（榜单 CI 要求每题 ≥5 试次）；
  对外只表述为「官方流程 + 单次尝试的新代际基线扫描」。
- 分数只作参照：达 90.6（官方 V4.1 Flash 公布值）才值得补 k=5；未达不阻断本轮，
  本轮首要产出是 orz 自校验与摩擦项。
- 容器内磁盘读数（`volume … free of …`）只反映盘内视角，**不作宿主余量证据**（审计 §2.3）。
- `D:\AGI` 为不可触碰禁区；本轮产物、缓存与可达范围止于 `D:\CLI` 与 `D:\tb-eval`。
