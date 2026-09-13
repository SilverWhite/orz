# TB 2.1 跑批容器侧排查与起跑口径（2026-09-13）

> 类型：**只读排查记录 + 一处用户授权清理的实测结果**；不含任何跑批结果。
> 范围：`D:\tb-eval` 评测装置（runner / 适配器 / 载体 / Harbor CLI / Docker 运行时 /
> 任务镜像 / gsa 卷）+ 容器内 orz 运行前提。
> 一句话结论：**容器侧主体链路实测可用；两处会直接影响跑批的问题（镜像缺口、磁盘余量）
> 已按用户裁决处置（不预拉 + 清空镜像），并定出「内存重题前置轮 + 降并发」的起跑形态。**
> 入口：排期 §3.2 / 索引 `EVAL-TB21-EVAL-CONTAINER-AUDIT` / BACKLOG 0b、P2-15 / TODO。

## 1. 核验通过清单（逐项实测）

| 项 | 手段 | 结果 |
|---|---|---|
| 载体身份 | 比对载体哈希记录 + 容器内执行 | `orz-linux/orz` = `393eee34…`（0.5.0 musl 静态）；三个任务镜像内均可执行，不依赖镜像 glibc ✓ |
| 适配器 ↔ 载体契约 | 按适配器真实命令行在任务镜像内冒烟 | `-p … --real --allow-write --allow-shell --max-tool-rounds 999` 正常启动并进入下一阶段 ✓ |
| ACAF 供应链 | 故意不给 signer 配置 | orz 拒绝启动（`ACAF fail-closed … no signer client`）⇒ fail-closed 生效；适配器负责灌入 manifest/keystore/环境变量 ✓ |
| 容器出网 | 容器内 DNS/TCP/HTTPS | `api.deepseek.com` 解析 + TCP 443 通 + HTTPS 返回 401（无 key 的正常应答）✓ |
| 镜像工具链 | 抽查任务镜像 | `/bin/sh` 与 `bash` 在位；适配器脚本为 POSIX-sh 写法；orz 自带静态 ripgrep，不依赖镜像 `rg` ✓ |
| gsa 卷接线 | 读适配器 + 卷目录 | `/orz-gsa` bind + 每试次软链 `$PWD/.gsa → /orz-gsa/<run_id>`；卷内自清理仅限本作业目录（14 天窗口）✓ |
| 运行配置 | `.env` / runner 参数 | key + `ORZ_TOOL_TIMEOUT_SECS=900` + `ORZ_STALL_TIMEOUT=360`（0.5.0 支持心跳）✓；runner 未加时间倍率与 `max_wallclock`（官方口径）✓ |
| Harbor | 版本与登录态 | 0.20.0，已登录（SilverWhite）✓；`official-b1…b5` 与 `official-r0-heavy` 作业名均未占用 ✓ |
| 数据集 | pin 比对冻结清单 | `terminal-bench/terminal-bench-2-1@sha256:7d7bdc1c…` 与冻结清单一致 ✓ |
| 宿主网络 | 到 Harbor Hub | 非沙箱环境 `https://hub.harborframework.com` 200 ✓ |
| 数字孪生 | 残留容器 | 0 个容器 ✓ |

## 2. 发现的问题

### 2.1 镜像缺口（89 题仅 20 个在位，缺 69 个）

按批次缺口：B1 缺 15 / B2 缺 14 / B3 缺 15 / B4 缺 13 / B5 缺 12。拉取通道本身正常
（实测 `docker pull hello-world` 与两个真实任务镜像均成功）。

### 2.2 磁盘余量不足（清空前的判断依据）

按 Docker Hub 元数据，89 题镜像压缩合计 **≈19.24 GB**；按实测落盘比（44 MB→181 MB、
163 MB→580 MB；镜像总占用 +0.46 GB / 压缩 +0.21 GB）外推，补拉 69 个镜像需
**≈35–55 GB**，而 `D:` 当时只剩 **31.81 GB** ⇒ 直接把 89 题跑完会在中途写满磁盘。

### 2.3 容器内磁盘预检门是盲区（0z 磁盘轴在容器内不可观测）

容器内实测 orz 读数：`volume 937.17 GiB free of 1006.85 GiB`——读到的是 Docker 虚拟盘的
稀疏上限，**看不到宿主 `D:` 的真实余量**。故 0z 的「盘满预检」在评测容器内不可能提前拦住，
磁盘只能由宿主侧管理。**这一条同时是 0z S4 自校验的边界**：磁盘轴在本轮容器形态下无法取证。

### 2.4 内存上限与并发

- Docker VM：12 CPU / **7.677 GiB** 内存；`.wslconfig` 未设 `memory=`（WSL2 默认上限 8 GB），
  且显式 `autoProxy=false`（NAT 模式无法镜像 localhost 代理）。
- 任务侧：**8 题要求 8192 MB**（见 §5.1）、21 题要求 ≥4096 MB ⇒ 单题请求已超过 VM 总量。
- 容器内 orz 硬上限读数：`commit=1.84 GiB`、`active_process=16`（按 cgroup 核数推导）、
  `cpu_rate=80%`；装载期读数 `commit 3.78 GiB free of 5.84 GiB`、`tier=normal`。
- Harbor 默认 `n_concurrent_trials=4`（模型 `models/job/config.py`），R1 那次也未显式传 `--n-concurrent`
  ⇒ 同口径 4。**风险**：8 GB 级任务在 7.68 GiB 机器上并发放大内存压力，且重活被 orz 自身门
  拒掉时会被记成「orz 侧摩擦」——归因污染。

### 2.5 契约漂移：`--max-tool-rounds` 已被静默忽略（仅记录）

适配器（冻结件 `2737cfad…`）固定传 `--max-tool-rounds <N>`（默认 999），但 orz 0.5.0 的 CLI
**没有这个旗标**，未知旗标被静默忽略、不报错。TER 之后主车道本就是「轮限 0 = 无硬限」，
故语义未变。风险条件：若将来改成「拒绝未知旗标」，整轮会起不来。
**本轮按用户裁决仅记录、不处理**（处置时机：下一次适配器变更批）。

### 2.6 工具面怪癖：客户端 registry 查询走 Desktop 代理超时

`docker manifest inspect` / `docker buildx imagetools inspect` 走 `http.docker.internal:3128`
会超时；而 `docker pull`（守护进程侧）正常。**排查时不要用前两者做前置判断**，会得出「拉不动」的
错误结论。

## 3. 用户裁决（2026-09-13）

1. **不预拉镜像**——系统代理（规则模式）当前稳定，镜像改为跑批时按需拉取；**清空全部镜像**省空间。
2. **第一轮先跑内存重题并降低并发**，该窗口内**不做其他内容**（不启模拟器、不跑安卓支线、不并行重活）。
3. **§2.5 的契约漂移仅记录，暂不处理。**
4. 落本记录。

> **更正（本记录一手）**：我此前口头报的「6 题要求 8192 MB」是**截断表造成的错**；
> 实测全集为 **8 题**。§5 按 8 题口径执行。

## 4. 本次执行的容器侧动作与实测结果

| 动作 | 命令 | 结果 |
|---|---|---|
| 清空镜像 | `docker image prune -a -f` + `docker builder prune -f` | 回收 **18.27 GB**；镜像 36 → **0**；容器 0；`docker system df` 全 0 |
| 磁盘余量 | `Get-PSDrive D` | 清空前后均 **31.81 GB**——VHDX 稀疏：空间在盘内归还，后续拉取先吃这 ≈18 GB，宿主实盘在 VHDX 越过 ≈43 GB 前不再增长 |

**可重建性登记**（清理的连带影响已核对）：arm 三臂探针镜像的配方在位
（`_linux_arm_dryrun/`：各任务 `environment/Dockerfile` + `build/`、`run/`、
`policy/` 脚本 + `PREP_RECORD_2026-09-01.md`）；`rust:1.97-slim`、`debian:bookworm*`、
`alpine`、`harborframework/terminal-bench` 等构建与探针基座**需重新拉取**（下次容器构建/探针批的前置项）。

## 5. 第 0 轮执行口径（内存重题前置轮）

### 5.1 题集（8 题 = `memory_mb == 8192` 全集，冻结清单实测）

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

- 拉取成本：8 个镜像压缩合计 **≈2.06 GB**（落盘预计 ≈6–8 GB）——第 0 轮自带的前置开销。
- 最坏墙钟（agent 侧求和）≈ **4.25 h**（15300 s），按串行且未计 verifier。

### 5.2 运行形态

- `-k 1`；**`-n 1`（串行）**——本轮降并发的落点：重题集恰是内存临界集，并发只会放大风险并污染归因；
  如需提速可一行改为 `-n 2`（记录在案，改则同批登记）。
- 其余保持官方口径：不加时间倍率、不传 `max_wallclock`、`--upload --public`。
- 卷与产物：`gsa-volumes/official-r0-heavy`、`jobs-official/official-r0-heavy`。
- 窗口纪律（用户裁决）：本窗口内**不做其他内容**——不启模拟器、不跑安卓支线、不并行容器构建。

### 5.3 命令（复现用，逐题显式列，不动冻结 runner）

```bash
HARBOR="D:/tb-eval/venv/Scripts/harbor.exe"
VOL="D:/tb-eval/gsa-volumes/official-r0-heavy"; mkdir -p "$VOL"
MOUNTS="[{\"type\":\"bind\",\"source\":\"$VOL\",\"target\":\"/orz-gsa\"}]"
"$HARBOR" run \
  -d "terminal-bench/terminal-bench-2-1@sha256:7d7bdc1cbedad549fc1140404bd4dc45e5fd0ea7c4186773687d177ad3a0699a" \
  -i terminal-bench/mcmc-sampling-stan -i terminal-bench/gpt2-codegolf \
  -i terminal-bench/mteb-leaderboard -i terminal-bench/caffe-cifar-10 \
  -i terminal-bench/torch-pipeline-parallelism -i terminal-bench/rstan-to-pystan \
  -i terminal-bench/torch-tensor-parallelism -i terminal-bench/filter-js-from-html \
  -a tb_agents.orz:Orz -m deepseek-v4-flash \
  --ak orz_binary=D:/tb-eval/orz-linux/orz --ak model_id=deepseek-v4-flash \
  --ak gsa_volume="$VOL" --mounts "$MOUNTS" \
  --env-file D:/tb-eval/.env --job-name official-r0-heavy \
  -o D:/tb-eval/jobs-official -k 1 -n 1 --upload --public -y
```

### 5.4 口径归属（推荐 + 备选）

- **推荐（默认）**：第 0 轮**计入**本轮 89 题口径；后续 1–5 批**显式剔除这 8 题**
  （16/15/15/17/18 = 81 题 + 第 0 轮 8 题 = 89，每题恰好 1 次）。
  实施方式 = 逐题 `-i` 列题或按批列题，**不修改冻结 runner**（清单的批次划分是冻结记录，
  执行分批允许不同，但需在台账登记该偏离）。
- 备选：第 0 轮仅作探索性前置，1–5 批仍按冻结划分跑满 89（代价：这 8 题多 1 次试次）。

## 6. 后续 1–5 批的磁盘纪律

- 剩余题集镜像压缩 ≈17 GB，落盘预计 35–55 GB；可用余量 ≈31.81 GB 宿主 + ≈18 GB 盘内归还。
  **纪律：每批「判分 + 取证进 `jobs-official/` 与 `gsa-volumes/`」完成后，删除该批任务镜像**，
  把峰值维持在一个批次内（B3 最重，压缩 5.38 GB）。
- 每批的资源遥测里，**磁盘读数只反映盘内视角**（§2.3），宿主侧余量须另取并同批登记。
- `_linux_arm_dryrun` 探针等非本轮镜像**不再提前拉取**，需要时按配方重建/重拉。

## 7. 判据（可机械核对）

1. `docker images -q | measure` = 0（§4 清空态）。
2. 第 0 轮 8 题全部落在 `jobs-official/official-r0-heavy`，每題 1 试次、`-n 1`（作业元数据可核）。
3. 第 0 轮卷 `gsa-volumes/official-r0-heavy/<run_id>/` 与题数一致（8）。
4. 后续批次镜像删除动作与批次数一致（每批一条清理记录）。
5. 本轮报告不把容器内磁盘读数当作宿主余量证据。

## 8. 边界与不做项

- **未做**：上调 WSL/Docker VM 内存（`.wslconfig` `memory=`，需重启 Docker Desktop——本轮改走「降并发 + 重题前置」路线）、VHDX 压实（需停 Docker）、预拉镜像、镜像常驻保留。
- 不把本记录当作 0z 设计或容器资源安全的符合性结论；§2.3/§2.4 是**观测边界与风险登记**。
- §2.5 只记录不处理（用户裁决）；§2.6 只作操作提醒。
