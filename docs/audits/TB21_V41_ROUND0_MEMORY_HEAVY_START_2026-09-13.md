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

### 6.2 作业中止（18:26–18:31，用户裁决）

**裁决**：第 8 题（`rstan-to-pystan`）已被外部影响（§6.6 操作事故自记）⇒ **不再放行，直接杀掉
并与其题一起重跑**。

**中止动作与结果**

| 时点 | 动作 | 结果 |
|---|---|---|
| 18:26 | 终止 launcher（python `21288`）、harbor（`10736`/`400`/`8024`）、该试次 `docker-compose`（`20352`） | 进程清点 0；**无其他 docker 进程被牵连**（见 §6.6 操作事故自记） |
| 18:30 | `docker rm -f f9760a5f41a0`（被中止试次容器，显式目标 ID） | 容器 0；镜像 8 个在位（caffe / filter-js 已预拉） |

**中止时刻账面**：7 完成（其中 6 报错）+ 1 运行中；被中止试次的 journal 已写到 **463 行**
（末次 18:28:51）⇒ **该试次不纳入本轮有效试次**，与四个网络因素题一起重跑。
**过程证据保留不删**：6 个 gsa run 目录 + 8 个试次目录（含 `torch-tensor-parallelism` 的
verifier 13/13 通过记录、三条 agent 日志、`reclaim_performed` 事件）。

**本轮 89 题口径的账（更正后）**

- **R0 主作业 `official-r0-heavy` 中止、不上传**（账面 6/8 报错、1 题被中止，作为 Harbor
  记录无意义；本地过程证据保真）。
- **有效试次构成**：补跑作业 `official-r0-netretry` 5 题（§6.5）
  ＋ R0 本地保留 3 题（`torch-tensor-parallelism` 已解出但超墙钟 / `mteb-leaderboard` /
  `gpt2-codegolf`）= 8 题，仍满足"8 题各 1 次"的口径；**代价**：保留的 3 题没有 Harbor 记录
  （k=1 本就不构成榜单提交，此代价已登记）。

### 6.3 深挖一：本轮模型动作实况（V4.1 Flash）

数据源 = 会话卷 `events.jsonl`（6 个 run，622 次工具调用 / 379 次 model_output）。

| 题 | model 轮 | 工具调用 | 工具构成（前几） | 工具墙钟 | 模型时延 p50/max |
|---|---|---|---|---|---|
| `torch-tensor-parallelism` | 71 | 112 | web_search 31 / run_terminal_cmd 28 / web_fetch 24 / search_replace 9 | 17.3 min | 3.9 s / 149.3 s |
| `torch-pipeline-parallelism` | 51 | 95 | web_search 35 / web_fetch 24 / run_terminal_cmd 14 | 17.0 min | 3.7 s / 103.4 s |
| `mteb-leaderboard` | 155 | 245 | run_terminal_cmd 122 / web_search 41 / web_fetch 30 / grep 18 | 34.3 min | 7.1 s / 41.6 s |
| `gpt2-codegolf` | 30 | 76 | web_search 25 / web_fetch 15 / blackboard_read 11 / read_file 11 | 8.8 min | 8.9 s / 33.5 s |
| `mcmc-sampling-stan` | 43 | 51 | run_terminal_cmd 38 / search_replace 9 | 16.1 min | 2.3 s / 26.9 s |
| `rstan-to-pystan`（被中止） | 32 | 51 | run_terminal_cmd 51（全程终端） | 11.6 min | 3.4 s / 37.8 s |

**解读 1 — 检索倾向暴涨，且直接吃掉官方预算**：检索类调用占工具调用比
`torch-pipeline 68%` / `gpt2 56%` / `torch-tensor 53%` / `mteb 31%` / `mcmc 0%` / `rstan 0%`
（本轮合计 **239/622 = 38%**；R1 全局基线 **119/2837 = 4.2%**）。四个检索型试次里
**`web_search` 一项独占官方 agent 预算**：torch-tensor **807 s / 900 s（90%）**、
torch-pipeline **817/900（91%）**、gpt2 **488/900（54%）**、mteb **977/3600（27%）**
——**超时的主要成因是检索耗时，不是模型慢**（模型时延 p50 仅 2.3–8.9 s）。
**对照边界**：R1 可得的同题 journal 只有 2 题（`filter-js` 66 次调用 0 检索、
`mcmc` 46 次 0 检索），故"V4.1 更爱检索"成立但要等第二轮全量同题对照收口。

**解读 2 — 浏览器通道 100% 死，而模型反复去撞**：`browser_launch_result` **14 次全 failure**
（`browser_not_found: no browser executable found (ORZ_BROWSER_PATH unset; searched: chrome,
google-chrome, … chromium …)`），连带 `browser_control` 13 次、`browser_read` 5 次全部失败。
R1 试次的 `config.json` 同样只有 gsa 一个 mount ⇒ **不是新回归**（容器内本就没有浏览器），
但**V4.1 比 V4 Flash 更常走浏览器车道**，代价因此放大。

**解读 3 — 失败/拒绝分类（6 run 合计）**：命令非零退出 20（`run_terminal_cmd`）/
**角色门拒绝 16**（`retrieval_role_write_denied`；检索子代理写盘被拒，设计内）/
**容器无浏览器 14** / **检索候选上限 13**（`candidate_cap_exceeded`，机械层 cap）/ 网络 5 /
检索子代理墙钟 5 / 工作区沙箱拒绝 5（`outside_workspace`，含 `/tmp`）/
**模型习惯 4**（命令里带 `&`，orz 回"Remove the background '&' …, set `is_background=true`"）/
`.gsa` 两段门首读通知 3 / DNS 失败 1 / 非文本内容 1。
⇒ 其中**装置侧 14+5+1=20 次**、**设计内门 16+13+5+3=37 次**、
**模型习惯 4 次**——三者须分开归因，不得混算成"工具失败率"。

**解读 4 — 两条契约漂移（适配器 → 0.5.0）**：除已登记的 `--max-tool-rounds 999`
（0.5.0 无此旗标、静默忽略）外，本轮实测第二条：适配器传
`--retrieval-mode local_browser`，0.5.0 回
`warning: --retrieval-mode / ORZ_RETRIEVAL_MODE is deprecated (0t γ, ADR-0010 §14.65)`
——**已弃用且被忽略**（车道选择自主）。两条都只记录、本轮不处理（用户裁决口径）。

### 6.4 深挖二定案：0z 资源面在 orz 内部被丢弃（**生产车道实现漏接**）

**结论**：`host_resource_snapshot` **不是设计内走别的面**，而是 **orz 内部两 crate 之间的
映射表漏项**——`orz-host` 发射的事实被 `orz-loop` 丢弃。

| 环节 | 一手证据 |
|---|---|
| 设计要求 | [`HOST_RESOURCE_SAFETY_DESIGN_2026-09-12`](../HOST_RESOURCE_SAFETY_DESIGN_2026-09-12.md) §4.5 明列 `host_resource_snapshot`（"档位变化时的读数…低频，跨档才落"）；`0Z_S2_COMPREHENSIVE_REVIEW_HANDLING` 记 F-EV-7"producer 已补（run_start + 跨档 tier_change 两触发）" |
| 发射端（有） | `crates/orz-host/src/lib.rs:463`（run_start 一次）与 `:1284`（跨档 tier_change）`push(json!({"event": "host_resource_snapshot", …}))` |
| 事件类型（有） | `crates/orz-assurance/src/journal/event.rs:45` `EventType::HostResourceSnapshot`；`orz-tui/src/bridge.rs:473` 亦处理该类型；assurance 侧有 `verify_host_resource_snapshot` 校验器与 v0.2 注册 |
| **接收端（缺）** | `crates/orz-loop/src/host_exec.rs:154` 的 `EVENT_TYPE_BY_FACT` **只有 4 项**：`reclaim_performed` / `resource_exhausted` / `host_resource_denied` / `resource_limit_hit`——**没有 `host_resource_snapshot`** ⇒ 命中 `180` 行 `else` 分支，WARN `unknown host resource fact kind; dropped (audit-face loss)` |
| 运行时后果 | 6 个 run 共 **13 次 WARN**（agent 日志），journal 事件统计里 `host_resource_snapshot` **0 次**（资源族仅 `reclaim_performed` 1 次）；即 **每个 run 至少应有的 1 行 run_start 读数全部丢失** |

**为什么既有核证没抓到**：0z S3 的接线核证是**符号在位**（`host_resource_snapshot` 3/3+，
[`0Z_S3_DUAL_PLATFORM_REBUILD`](0Z_S3_DUAL_PLATFORM_REBUILD_2026-09-13.md)），**不是
「事实 → 判定表 → journal」的端到端**；`host_exec.rs` 的判定表**没有覆盖该 kind 的测试**
（同文件 3647 行的调用点在工具执行边界，测试覆盖的是别族）。与
`ORZ-PLATFORM-TARGET-001` 同族放大器：**"在位"≠"接线"**。

**登记与边界**：本条按 `ORZ-VERDICT-EPOCH-001` 纪律先回查再判断——已核对设计文档、发射端、
事件类型、接收端四处一手来源，**判定为实现漏接（生产车道）**，非设计内。处置建议：
立案 GAP（补判定表一项）＋ 端到端钉子（`run_start` 必落 1 行 `host_resource_snapshot`
进 journal）＋ 与"符号核证"分开标注。**本轮不修载体**（用户对重建的既有裁决：不影响框架
实际动作则不重建；此处影响的是**取证面**而非动作面）。

### 6.5 补跑作业起跑（`official-r0-netretry`，18:32）

- **题集 5 题**（用户裁决）：4 个网络因素题（`caffe-cifar-10` / `filter-js-from-html` /
  `mcmc-sampling-stan` / `torch-pipeline-parallelism`）＋ 被中止的 `rstan-to-pystan`。
- **预拉（本轮新纪律）**：起跑前逐题 `docker pull`，**5/5 全绿**，解析后 digest 入档
  （`caffe-cifar-10@sha256:929a6d63…`、`filter-js-from-html@sha256:92acda0f…`、
  `mcmc-sampling-stan@sha256:073fc36a…`、`torch-pipeline-parallelism@sha256:3cb7b39d…`、
  `rstan-to-pystan@sha256:b23d4883…`）；**预拉不全绿即不带残批起跑**（执行器 `return 3`）。
- **口径**：单作业、`-k 1`、`-n 1`、`--upload --public`、官方数据集 pin、无时间倍率、
  无 `--max-retries`；卷 `gsa-volumes/official-r0-netretry`、产物 `jobs-official/official-r0-netretry`。
- **代际身份门**同前（载体 `393eee34…` + 适配器 `2737cfad…`）。
- **起跑前快照**：宿主 `D:` 可用 **30.79 GiB**、镜像 8、容器 0。
- **已知残余风险（登记）**：`torch-pipeline-parallelism` 的 verifier 需现下 torch（825 MB）
  与 CUDA 系列轮子，上一轮即在此处 `UV_HTTP_TIMEOUT=30s` 崩；若再次发生，属**装置侧网络**
  导致该题 reward 无效（不改官方参数规避，必要时由用户裁决是否放宽 verifier 侧网络容忍）。

### 6.6 操作事故自记（本轮两处自伤，须登记）

排障期间由**本代理自己的进程筛选方式**引入两处误伤：

1. **18:26（第一处）**：改用「可观察的重试式预拉」前，为停掉卡死的拉取，用
   `Get-Process -Name docker` 宽匹配逐个终止——`docker` 这个**进程名**同时命中
   **正在跑批试次的 `docker compose exec` CLI（pid 17872）**，该试次的 agent 阶段流被切断
   ⇒ `rstan-to-pystan` 试次作废（容器内 orz 仍存活、journal 继续写到 463 行，故**结果不可用**）。
2. **18:30（第二处）**：改按命令行过滤终止时，过滤条件里包含 `official-r0-heavy` 等字样，
   **未排除本 shell 自身**，于是把**正在执行该命令的 shell（pid 6032）**一并杀掉，
   命令以 `-1` 中断（后续清点步骤未跑完，已另起命令补齐）。

**影响与处置**：仅 `rstan-to-pystan` 一个试次受影响，且已按裁决并入补跑
（§6.2 / §6.5）；无其他数据损失，本地过程证据完整。

**更正纪律（本轮新增，供后续批沿用）**

- 终止进程一律**按完整命令行 + 白名单式显式目标**筛选，并**强制排除本 shell 及其祖先**；
  `docker` / `harbor` 等**进程名匹配不得用于杀进程**（会牵连跑批中的 exec）。
- 容器一律**用显式 ID** `docker rm -f <id>`，不用模式匹配。
- 排障动作前先取一次「谁是跑批进程」快照，排障后再取一次，逐项对比。
- 泛化候选：本条属 `harness_environment` 类的**操作摩擦**（与"缺件伪装成失败"同族：
  装置操作动作污染跑批结果），可入案例库。

## 7. 复现入口与产物位置

- 起跑器：[`scripts/run_r0_heavy_official.py`](../../scripts/run_r0_heavy_official.py)
  （`scripts/LIFECYCLE.md` 登记为 `active`）。参数：`--dry-run`（只打印解析后的 argv、镜像与
  前置，不触碰装置）／`--tasks a,b,c`（题集子集，批次与超时取自冻结清单，供补跑复用）／
  `--job-name`（作业名）／**预拉默认开**（逐题 `docker pull`、最多 4 次重试、实时落盘
  `preroll-images.log`、记录解析后 digest，**不全绿即 `return 3` 中止**）／
  `--no-pre-pull`（仅在确认镜像全在位时）／`--pull-only`（只拉不跑）。
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
