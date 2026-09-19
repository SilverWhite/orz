# N4 资源门调研：业界资源管理器／沙箱分配机制（2026-09-20）

> **修订指针（2026-09-20 同日）**：用户澄清「不是让 orz 自管配额，而是 **orz 只发进程、把资源与运行交给操作系统**」⇒
> 本档 **§1 一手实测** 与 **§2 业界机制对照** 仍有效并复用；**§3 结论作废**（它把用户的意思读成了「orz 自管」）。
> 更正后的定案见 [`N4 重定案：OS 委派执行`](N4_OS_SCHEDULING_DELEGATION_2026-09-20.md)。

> 用户令：「N1/2/3/5 只出方案（GLM 实施，当前立项粒度足够）；**请调研 N4 即可**」。
> 本档＝**N4（0aw）专项调研**：先给一手实测（orz 两个真实运行面的资源管理现状），再对照业界成熟机制，
> 最后给 0aw 的**修正方案与判据**。零代码、零子仓改动；立项条目与计数不动（0aw 维持开放）。
> 关联：[`N1–N6 深挖`](0AR_S3_FRICTION_DEEP_DIVE_N1_N6_2026-09-20.md) §5／[`立项与调研档`](N1_N5_ITEM_REGISTRATION_AND_N4_N5_RESEARCH_2026-09-20.md) §3。

## §0 摘要

1. **orz 有两个运行面，资源管理权的归属不同**：
   - **Windows 宿主面**（狗粮／本机真机）：orz **已经持有进程容器**——`orz-assurance/src/sandbox/job_object.rs`
     的 Job Object（当前只开 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`，用于进程树回收）。
     Windows Job Object 可挂内存上限（`JOB_OBJECT_LIMIT_JOB_MEMORY`／`ProcessMemoryLimit`），
     **不需要特权** ⇒ orz 在这一面**就是可以自管的资源管理器**。
   - **Linux 评测容器面**（官方跑批）：容器已由运行时限额（实测 `memory.max = 8 GiB`、`cpus = 1`），
     但容器内 **cgroupfs 只读、`subtree_control` 为空** ⇒ orz **不能**自建子 cgroup／自设 `memory.high`；
     能读的是 `memory.current|max` 与 **PSI**（`memory.pressure`）。
2. **业界共识＝"限额＋软限限流＋回收＋分级驱逐"，不是"静态百分比二值拒绝"**：
   cgroup v2（`memory.high` 软限／`memory.max` 硬限／`memory.reclaim`／PSI）、systemd 三档、
   k8s QoS＋软/硬驱逐、Docker 软限、Borg 回收＋超卖＋准入。
3. **0aw 定案方向**：**运行时当资源管理器（硬限＋保活），orz 当限额内的调度器**——
   判据改「限额内余量＋PSI 压力」；可自主动作限进程级（队列并发／nice／ionice／rlimit／回收／退避）；
   Windows 侧可选把 Job Object 升级为**带内存预算的**作业容器（"+资源管理器自管"的正解）；
   **不引入 privileged 容器、不申请 cgroup 委托、不改 `task.toml`**。

## §1 一手实测（2026-09-20）

### 1.1 Linux 评测容器面

按 harness 同参数起容器（`alexgshaw/gpt2-codegolf:20251031`，`--memory 8192m --cpus 1`）：

| 探针 | 读数 | 判读 |
|---|---|---|
| `mount \| grep cgroup` | `cgroup2 on /sys/fs/cgroup type cgroup2 (ro,nosuid,nodev,noexec,relatime)` | **只读挂载** |
| `mkdir /sys/fs/cgroup/x` | `Read-only file system` | 不能自建子 cgroup |
| `cgroup.subtree_control` | 空 | 无委托（父层未把控制器下放） |
| `memory.max` | `8589934592`（8 GiB） | 硬限**在位**（由 harness 按 `task.toml` 的 `memory_mb=8192` 施加；`harbor/environments/docker/docker.py` 里 `memory=f"{memory_mb}M"` 即此处来源） |
| `memory.high` | `max` | **无软限**（未被设置） |
| `memory.current` | 1.6 MB（空载） | 可读，可用于"限额内余量" |
| `memory.pressure`（PSI） | `some/full avg10/60/300=0.00 total=…` | **可读**（容器内即可读，不必特权） |
| `/proc/pressure/memory` | 同样可读 | 备选读数面 |

⇒ 结论：**评测容器里的"资源管理器"就是 Docker/harness**（硬限＋CPU 配额＝保活底座）；
orz 在容器内的权限边界是**只读消费**，不是自管。

### 1.2 Windows 宿主面（orz 现实持有的那一面）

| 事实 | 代码位置 | 判读 |
|---|---|---|
| orz 已为工具进程树创建 Job Object，且开 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`（进程树回收，0z） | `crates/orz-assurance/src/sandbox/job_object.rs` | orz **已经在做进程级资源管理**（kill-on-close） |
| 该 Job Object 目前**未**设内存上限 | 同上（只设 kill-on-close 位） | 可扩展点：`JOB_OBJECT_LIMIT_JOB_MEMORY`／`JOBOBJECT_EXTENDED_LIMIT_INFORMATION.ProcessMemoryLimit` |
| 资源门读的是**宿主 commit**（`commit_limit_bytes`／`commit_used_bytes`）＋卷余量 | `crates/orz-host/src/resource_gate.rs`（`HEAVY_RELEASE_COMMIT_HEADROOM_PERCENT = 25`、`HEAVY_RELEASE_FREE_BYTES = 8 GiB`） | 这是 **Windows 提交量**语义；本机基线 86–87 % used ⇒ 25 % 门结构性不可满足（深挖 §5） |

⇒ 结论：Windows 面的"接入资源管理器"**不需要新组件**——把已有的 Job Object 升级为
**带内存预算的作业容器**即可（内核负责在该预算内限流/终止，而不是靠入口处拒绝）。

## §2 业界机制对照

| 体系 | 关键机制 | 与 orz 现状的差距 | 可借鉴点 |
|---|---|---|---|
| **Linux cgroup v2**（[kernel doc](https://docs.kernel.org/admin-guide/cgroup-v2.html)） | `memory.max`＝硬限（超则 OOM）、**`memory.high`＝软限（超限即限流＋回收，不杀）**、`memory.reclaim`＝主动回收、`memory.low`＝保护 | orz 只有"硬拒绝"，无软限/回收 | 动作分级：**先限流/回收，最后才拒绝** |
| **PSI**（[kernel doc](https://docs.kernel.org/accounting/psi.html)、[facebookmicrosites PSI](https://facebookmicrosites.github.io/psi/docs/overview)） | `some/full avg10/60/300`＝真实压力读数（与"用了多少"解耦） | orz 判据用**占用百分比**，不用压力 | 判据换压力读数，天然适配"别人也在用这台机"的场景 |
| **systemd 资源控制**（[resource-control](https://www.freedesktop.org/software/systemd/man/systemd.resource-control.html)、[CGROUP_DELEGATION](https://systemd.io/CGROUP_DELEGATION/)） | `MemoryLow/High/Max` 三档；**委托（Delegate=yes）**才允许单元自建子 cgroup | orz 无委托（容器内 ro）；三档只实现成"拒绝/放行" | 三档语义照搬；委托只在**运行时**给（评测口径下否） |
| **Kubernetes**（[node-pressure eviction](https://kubernetes.io/docs/concepts/scheduling-eviction/node-pressure-eviction/)、[QoS](https://kubernetes.io/docs/concepts/workloads/pods/pod-qos/)） | 按 QoS 分级；软阈值（`eviction-soft`＋宽限）／硬阈值；kubelet 侧"系统预留＋驱逐" | orz 一刀切 | **软阈值给宽限窗口**＋按负载分级 |
| **Docker 资源约束**（[docs](https://docs.docker.com/engine/containers/resource_constraints/)） | `--memory`（硬限）＋`--memory-reservation`（**软限**，紧张时才生效） | 官方容器只给了硬限 | 若需软限，**由 harness 起容器时**加（装置侧） |
| **Borg**（[paper](https://research.google/pubs/large-scale-cluster-management-at-google-with-borg/)） | 资源**回收（reclamation）＋超卖（overcommit）**、best-effort 任务、准入控制看"估算需求 vs 可用余量" | orz 无回收、无优先级 | 准入用"需求 vs 余量"，并允许回收低优负载 |
| **gVisor**（[Resource Model](https://gvisor.dev/docs/architecture_guide/resources/)、[github](https://github.com/google/gvisor)） | 沙箱内**显式资源模型**（把限额当成契约下发给沙箱），线程/内存按模型记账 | orz 记账是"事后读数" | 把预算**显式**写给执行体（而非只在门口拒绝） |
| **Windows Job Objects**（[learn.microsoft.com](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects)、[JOBOBJECT_EXTENDED_LIMIT_INFORMATION](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_extended_limit_information)） | `JOB_OBJECT_LIMIT_JOB_MEMORY`＝作业总提交上限、`ProcessMemoryLimit`＝单进程上限、`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`＝保活/回收 | orz 只用了 kill-on-close | **Windows 面自管的落点**：给 Job Object 加内存预算，超额由内核限/杀，而非入口拒绝 |

## §3 0aw 修正方案（替代"orz 自管 cgroup"）**【本节定案已作废，见顶部修订指针】**

### 3.1 架构定案

| 角色 | 谁 | 做什么 |
|---|---|---|
| **资源管理器** | **运行时**（Linux：Docker/harness；Windows：宿主内核＋orz 的 Job Object） | 硬限（`memory.max`／Job 内存位）、CPU 配额、OOM/杀进程保护、进程树回收——即**保活底座** |
| **限额内调度器** | **orz** | ① 读限额与压力（Linux：`memory.current|max`＋`memory.pressure`；Windows：commit 读数＋Job Object 记账）；② 用**进程级手段**执行调度：作业队列并发上限、`nice`／`ionice`、`setrlimit`、退避重试、回收（关进程树）；③ 判定参数化（heavy 分类） |
| **软限（可选，需裁决）** | harness | 起容器时加 `--memory-reservation`（或等价 `memory.high`）——**装置侧改动**，会改变与官方 minimal 口径的可比性，默认不做 |

**明确否决**：容器内自建 cgroup／申请委托（需要 privileged 或显式委托，评测口径下不可接受）；
不为本项改 `task.toml`／镜像／verifier。

### 3.2 判据修正（相对原方案）

| 原 | 修正后 |
|---|---|
| heavy 放行门＝宿主 commit headroom ≥ 25 %（本机不可满足） | **主判据**＝① 容器限额内余量（`memory.max − memory.current`）≥ 绝对下限（建议 256–512 MiB）；② 有 PSI 时压力不超阈（`some avg60` 连续超阈 ⇒ 拦截）；**宿主 commit 百分比降为观测读数**（Windows 面它仍是真实约束，保留为 secondary 提示） |
| 二值：放行／拒绝 | **三级**：正常放行 → **软档降速执行**（串行化、nice/ionice、缩小并发、退避一次）→ **硬档拒绝** |
| heavy 判定按程序名 | **参数感知**（`--version`/`-t`/`-l`/`--dry-run` 降级）＋**引号感知分段**（消 `grep -E "python|pip|conda"` 伪触发） |

### 3.3 Windows 面可选项（"资源管理器自管"的正解）

把 `job_object.rs` 的 Job Object 升级为**带预算的作业容器**：
`JOBOBJECT_EXTENDED_LIMIT_INFORMATION` 的 `JOB_OBJECT_LIMIT_JOB_MEMORY`（作业总提交上限）
与/或 `ProcessMemoryLimit`（单进程上限）——heavy 工具（编译/解包/安装）在**自己的预算内**跑，
超额由内核限流/终止；orz 侧只需把预算写进 Job 并记读数。收益＝把"入口拒绝"换成"**执行期约束**"，
与 cgroup `memory.high` 同语义，且**不需要任何特权**（Job Object 是普通 Win32 API）。

### 3.4 判据与验收（0aw 更新后）

1. 同一 S3 语料回放：heavy 类 shell 拒绝率从 **28/29** 降到「真实重活且真无余量才拒」（目标 ≤ 1/3），
   且每次拒绝都带可核读数（限额内余量／压力／commit）；
2. 无 OOM 杀、无"等待导致墙钟恶化"（用 run 墙钟＋`host_resource_snapshot` 复核）；
3. Windows 面（若实施 3.3）：Job 预算生效可核（超额进程被限/杀，且 run 不整体失败）；
4. 只改 orz 侧判定与进程约束，**不动官方环境口径**。

## §4 风险与边界

- **PSI 在 Windows 宿主不可得**（PSI 是 Linux 机制）：Windows 面继续用 commit 读数＋Job 记账，
  两面的判据字段必须**各自可核**（不得用一个平台的读数去代表另一个平台）。
- **容器内 `memory.current` 是"本容器"视角**，不含同机其它容器/宿主开销 ⇒ 与宿主 commit 读数**不可互相替代**，
  两者都要记（观测）。
- 「软档降速」会**拉长**重活耗时：必须以 run 墙钟读数复核"不恶化"（判据 2），否则回退到直接拒绝。
- 本项**不改** 0ar 的检索阈值/墙钟参数，也不改官方镜像、verifier、数据集 pin。

## §5 证据物

- 实测命令与读数：本文 §1（容器 cgroup/PSI 探针、harness `docker.py` 限额来源）
- 代码面：`orz/crates/orz-host/src/resource_gate.rs`（25 % 门与 heavy 分类）、
  `orz/crates/orz-assurance/src/sandbox/job_object.rs`（Job Object 现状）
- 一手摩擦读数：[`N1–N6 深挖`](0AR_S3_FRICTION_DEEP_DIVE_N1_N6_2026-09-20.md) §5（28/29 拒绝、gpt2 11/11 gcc 全拒）
- 外部参考：见 §2 各条超链接（kernel cgroup-v2／PSI、systemd、k8s、Docker、Borg、gVisor、MS Job Objects）
