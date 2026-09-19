# N4 重定案：OS 委派执行（orz 只发进程、只收结果）可行性（2026-09-20）

> 用户澄清（2026-09-20）：「资源管理器那条路实际上我不是这个意思，我的意思是 orz 就和普通软件一样只发进程，
> 怎么接收和怎么派发资源和运行进程交给 windows 资源管理器自己进行调度，orz 只收结果，相当于 orz 使用 windows
> 资源管理器，这样能做吗？反正我们的 orz 都已经进入真机环境，不在沙箱里，那么使用真机环境的配套设施应该是可以的吧？
> linux 侧主要是不仅仅是 linux 容器内部，我考虑主要是需要顾及 linux 真机环境」。
>
> 本档＝对 [`N4 资源门调研`](N4_RESOURCE_GATE_INDUSTRY_RESEARCH_2026-09-20.md) 首版方向的**更正与重定案**。
> 首版 §1（两个运行面的一手实测）与 §2（业界机制对照）**仍然有效并复用**；首版 §3 的结论
> 「运行时当资源管理器 ＋ orz 当限额内调度器」**作废**——它把用户的意思读成了「orz 自管配额」，
> 用户要的是「**orz 不做资源管理，交给操作系统**」。
> 零代码、零子仓改动；未闭合计数不变（40），0aw 维持开放。

## §0 裁决（先给结论）

1. **能做，两个平台都成立，而且 Windows 面已经落地了大半。** 但要把「交给资源管理器」的语义说准：
   操作系统**从来不提供**「你把进程交给我、我替你决定何时运行」的准入服务。进程一旦创建
   （`CreateProcess` / `fork+exec`），它就已经在内核运行队列里、提交就已经被内核记账——**这部分本来就是 OS 在管**。
   真正多余的只有 **orz 自己加的那一层准入拒绝**。
2. **Windows 面没有可接的「资源管理器」服务**：WSRM（Windows System Resource Manager，曾按应用/用户分配 CPU
   与内存）**自 Windows Server 2012 弃用、2012 R2 移除**，微软给出的替代路径正是 **Job Objects ＋ 原生资源控制
   API**（或 Hyper-V 做隔离）；Task Scheduler 按时间/事件/条件触发，**不是**资源调度器。
   而 orz **已经持有**带内核强制的 run 级 Job Object（`RunResourceJob`：commit／active_process／cpu_rate／
   kill-on-close）——即「OS 在管」这件事在 Windows 面**已经存在**，缺的只是**别再自己拦**。
3. **Linux 面（含真机）**：默认形态（真机裸跑）＝ 内核全局调度 ＋ 启发式 overcommit，**没有任何强制面**；
   强制面只在**部署给了**的时候存在（容器 `memory.max`／systemd slice／cgroup 委托）。
   orz 侧因此**不能**承诺限额，能做的只有：读信号（`MemAvailable`／PSI／cgroup 读数若有）、
   做进程级礼貌（`nice`／`ionice`／并发=1／`setrlimit`／退避）、把 OS 的失败码如实上报。
4. **一个必须拆掉的读数**：`CommitLimit`／`Committed_AS` 在 Linux 默认（`vm.overcommit_memory=0` 启发式）下
   **不是强制上限**（内核文档：这两项只在 `vm.overcommit_memory=2` 时生效）。S3 实测正好证明它会把
   「一半 RAM 的会计口径」误报成「宿主机内存即将耗尽」。
5. **0aw 重定案**：由「准入校准」改为「**去准入、留内核上限、读数降为观测**」。
6. **运行侧根因复核（同日追加，§7）**：2026-09-12 的两次死亡**不是「被连带杀」**——方向相反，
   是「orz 自己死 → job 句柄关闭 → kill-on-close 带走工具树」；其中磁盘那次（A）**已由 0z 子项 C 修掉**，
   commit 那次（B）的根因是 A（满盘 → 系统托管页面文件无法增长 → commit 在名义上限 73 % 就失败）
   加上 orz 自身**没有为自己预留预算**（Rust alloc 失败即 abort）。**结论：退役准入门可以，退役 Job 上限不行。**
7. **卷轴裁决已落定（§8）**：降为观测 ＋ **预派发机械软提示**，不阻断。

## §1 现场证据（2026-09-20 实测）

### 1.1 Windows 宿主面（当前这台机器，实读）

| 读数 | 值 | 判读 |
|---|---|---|
| `\Memory\Commit Limit` | 26.19 GiB | 分页文件＋RAM 的提交上限 |
| `\Memory\Committed Bytes` | 21.82 GiB | **83.3 % used**、headroom 4.37 GiB |
| 25 % 门所需 | 6.55 GiB | ⇒ **结构性不可满足** |

⇒ 此刻 orz 在这台机器上的 heavy 类调用（`cargo`／`gcc`／`pip`／`tar`／解包）**全部会被拒**。
同一读数还**泄漏进内核强制面**：`default_job_limits` 用 `headroom − 1 GiB` 推导 run 级 Job 的 commit 上限
⇒ 今天这台机器给整个 run 的预算是 **3.37 GiB**——把「别的软件占了多少」记在 orz 头上。

### 1.2 Linux 评测容器面（S3 三题 journal 原样读数）

- **25 条 `host_resource_denied`**（torch `RUN-CLI-6aaeb160` 14／gpt2 `RUN-CLI-6aaeb62b` 11）。
- `commit_limit_bytes` **恒为 6,269,009,920 B ≈ 5.84 GiB**（＝ 50 % × RAM 的会计口径，不是可用内存），
  `commit_used_bytes` 在 5.43–5.63 GiB 之间 ⇒ 文案报「commit headroom 12.0 % (0.71 GiB of 5.84 GiB) < 25 % required」。
- 同一时刻的**真实**环境：卷 **943.23 GiB free / 1006.85 GiB**（磁盘不短）；gpt2 题容器自身限额 8 GiB。
  ⇒ 拒绝句「宿主机内存资源即将耗尽」**是不实陈述**：25 次被拒调用里没有一次是真缺内存。
- **会计抖动的证据**：16:10:38 一瞬 `used = limit`（tier hard）、两秒后回到 87 %（tier soft）。
- **拒绝是 pre-issue 的**：14 条被拒 call_id 在 `session/terminal/` 下**没有任何** `call_*.log`
  （同日实际执行的调用共 20 条日志）⇒ 什么都没跑。

### 1.3 代码面（三层，各自职责）

| 层 | 位置 | 现状 |
|---|---|---|
| **准入层（要退役的）** | `orz/crates/orz-host/src/resource_gate.rs` | `HEAVY_RELEASE_COMMIT_HEADROOM_PERCENT = 25`、`HEAVY_PROGRAMS` 109 项、读数不可得即 fail-closed 拒绝 |
| **强制层（OS 在管）** | `orz/crates/codegen/xai-tty-utils/src/resource_job.rs` | Windows：run 级 Job Object（commit／active_process／cpu_rate／kill-on-close，**内核强制**）；Linux：无 job 等价物 ⇒ `enforced: false`（诚实的非强制面） |
| **装配点** | `orz/crates/orz-host/src/lib.rs::install_resource_safety` | 探针 → `default_job_limits` → `install_global_run_job` → 挂门 |

## §2 Windows 上「资源管理器」的可得性（用户问题的直接回答）

| 候选 | 是什么 | 能否接管「何时运行」 | 结论 |
|---|---|---|---|
| **WSRM**（Windows System Resource Manager） | 按应用/用户/会话分配 CPU 与内存的管理工具 | 曾能做「资源策略」，但**已不在产品内**（2012 弃用、2012 R2 移除） | **不可用**；微软给的替代＝ Job Objects ＋ 原生资源控制 API，或 Hyper-V |
| **Task Scheduler** | 时间/事件/条件（空闲、交流电）触发任务 | **不能**：不按资源余量派发 | 不是资源调度器 |
| **Job Objects** | 内核强制 commit／CPU rate／进程数上限、kill-on-close | 不决定「何时跑」，但决定「能跑多大」 | ✅ **orz 已用**（`RunResourceJob`） |
| **优先级／EcoQoS／memory priority／affinity** | `SetPriorityClass`、`SetProcessInformation`、`SetProcessAffinityMask` | 只影响调度权重，不拒绝 | 可作可选礼貌手段 |
| **内存管理器**（commit／page file） | 需求分页 ＋ 提交记账；耗尽时分配失败 `ERROR_COMMITMENT_LIMIT` | **就是**内存侧的接管方 | ✅ 已在接管；失败应在**结果面**处理 |

⇒ Windows 面「orz 只发进程」的全部内容 ＝ ① 不再预检拒绝；② 保留 Job Object 上限（修正其推导）；
③ 把 `ERROR_COMMITMENT_LIMIT` 当**普通工具失败**上报，而不是提前替模型拒绝。

## §3 Linux 面（真机与容器分别说清）

| 场景 | 强制面 | orz 能做的 |
|---|---|---|
| **官方评测容器** | Docker/harness 依 `task.toml` 给 `memory.max` ＋ `cpus`；cgroupfs 只读、无委托 | 只读消费：`memory.current|max`、PSI；**不做**准入拒绝 |
| **真机 ＋ systemd** | 可由**部署**给 slice（`MemoryMax/MemoryHigh/CPUQuota/IOWeight`）或用户委托（`systemd-run --user --scope -p MemoryMax=…`） | 检测到就用（读限额/压力）；这是部署侧动作，不是 orz 的能力 |
| **真机裸跑（默认可达形态）** | **无强制面**：内核全局调度 ＋ 启发式 overcommit | 进程级礼貌（`nice`／`ionice` best-effort 免特权／`setrlimit`／并发=1／退避）＋ 失败如实上报 |

补充两条机械事实：

- **信号面可用**：`/proc/meminfo` 的 `MemAvailable`（真实余量）、`/proc/pressure/memory`（PSI，免特权可读）。
- **`CommitLimit`／`Committed_AS` 不得作阈值**：内核文档明确这两项只在 `vm.overcommit_memory=2` 下生效；
  默认（0，启发式）下它只是 `(RAM − hugeTLB) × overcommit_ratio / 100 + swap` 的**会计口径**，
  既不强制、也不代表可用余量。`RLIMIT_AS` 同样不是好工具（gcc／JVM 会预留巨量虚拟地址）。

⇒ **Linux 真机上「交给 OS」完全成立**，但诚实表述是「**没有限额，也就没有 orz 侧的拒绝**」，
而不是「OS 会替 orz 保活」。保活若真是需求，只能由**部署**（容器／systemd slice）提供；
检测不到时如实记 `enforced=false`（**现行为已是如此**，无需新增机制）。

## §4 0aw 重定案

| 项 | 原（首版方向） | 修正后 |
|---|---|---|
| 准入拒绝 | heavy 需 `free ≥ 8 GiB` 且 `commit headroom ≥ 25 %`，否则 pre-issue 拒绝 | **退役**；heavy 拒绝数判据从「≤1/3」改为「**→ 0**（不含真无余量）」 |
| 分类器 | `HEAVY_PROGRAMS` 109 项决定是否拒绝 | 降级为**观测标签**（保留读数价值、不再拦人）；引号/参数感知问题随之消失 |
| 内存读数 | `CommitLimit`／`Committed_AS` 作阈值 | **禁用为阈值**，仅保留为观测字段 |
| run 级 Job Object | 保留；commit 上限＝`headroom − 1 GiB` | **保留**（Windows 面内核强制）；commit 上限**不再**从宿主 commit 百分比推导（改固定/可配预算或默认不设）；`active_process`（2×cores+8）与 `cpu_rate` 80 % 可保留 |
| 卷余量轴 | 与内存轴同门 | **单列**：Job Object 覆盖不到磁盘，且 2026-09-12 的死亡在磁盘侧（`os error 112`）——建议降为观测＋软提示，真死亡面改为「journal 写失败不得致命」 |
| Linux 无强制面 | fail-closed 拒绝 | **不拒绝**；只记 `enforced=false` ＋ 信号读数 |

**判据（0aw 更新后）**：

1. 同语料回放 **heavy 类拒绝数 = 0**（或仅剩「读数与真实余量同时短」这一种，且带可核读数）；
2. 无 OOM 杀、run 墙钟不因等待恶化；
3. 每次调用仍留下可核观测（tier／readings／`enforced`），但**不再阻断**；
4. 只改 orz 侧判定，不动官方环境口径（不改 `task.toml`／镜像／verifier ／数据集 pin）。

## §5 裁决点（留用户）

- **A. 卷余量轴** —— **已裁决（2026-09-20，见 §8）**：降为观测 ＋ **预派发机械软提示**（不阻断）。
- **B. run 级 Job 的 commit 上限**：默认不设／固定预算（如 4 GiB）／保留推导但只作上限、不作拒绝（推荐）。
- **C. 分类器**：保留为观测标签（推荐，成本最低）／随门一起删。

## §6 风险与边界

- 「没有限额」＝「没有 orz 侧的拒绝」，**不等于**「OS 会保证 orz 存活」；Linux 真机上多进程争抢的后果由内核决定
  （OOM killer 可能杀掉机器上任何参与者，不只 orz）。
- Windows 面真正会「拒绝分配」的是 commit 耗尽（`ERROR_COMMITMENT_LIMIT`）：这是**结果面**事件，
  必须能被工具结果识别并如实上报，否则会退化成「神秘失败」。
- 移除拒绝门会让个别重活在低余量时**变慢甚至失败**——这是**有意的语义变更**（把「预判」换成「如实失败」）。
- 与 0z（真机资源安全边界）的边界：0z 的进程树回收（kill-on-close）与 job 上限**不动**；
  本项只拆**准入层**，不改内核强制面。

## §7 运行侧根因复核：2026-09-12 两次死亡的真实因果（2026-09-20 追加取证）

用户追问：「那当前的资源门直接退役怎么样？……但之前爆内存/磁盘的两次都直接连带带走了 orz 整体，
是不是 orz 和派发的进程的树不是独立的所以一起被杀了？根本上难道是运行侧的问题吗？」

一手记录（[`HOST_RESOURCE_SAFETY_DESIGN_2026-09-12`](../HOST_RESOURCE_SAFETY_DESIGN_2026-09-12.md) §1／§3.4 F-4／§4.2）：

| run | 时长 | 死法 | 直接证据 |
|---|---|---|---|
| **A** `RUN-CLI-6aa4f384` | 98.2 min | `error: journal error` **致命退出** | `resources_state.json` 写失败（WARN）→ journal 写失败 `os error 112`（**盘满**） |
| **B** `RUN-CLI-6aa50fdf` | 20.2 min | 进程 **abort，无终止事件** | `memory allocation of 200720 bytes failed`（commit 耗尽）；同批工具面另见 `os error 1455`（页面文件太小） |

**判定一：不是「被连带杀」，方向相反。** orz 是 run 级 Job 的**持有者**，不在 Job 里——
`ProcessGroup` 只把**子进程** `AssignProcessToJobObject`（`xai-tty-utils/src/lib.rs::attach_pid`），
而 0z S1 实施记录①明写「把 `orz.exe` 自身纳入带限 job 会让 commit 上限与 `KILL_ON_JOB_CLOSE`
作用在 agent 本体上」，故**不纳入**。真实次序是：**orz 自己死了 → 句柄关闭 → `KILL_ON_JOB_CLOSE`
带走整棵工具树**；设计档 §4.2 的原话也是这个口径（「父进程 abort 不再留孤儿」，因为
`std::process::abort` 不跑析构）。⇒ **树不是凶手**。

**判定二：磁盘那次（A）的根因是 orz 自己的策略，且已经修掉了。** 「journal 写失败 ⇒ 致命」是当时的选择，
不是外部谁杀了它。当前载体已由 0z 子项 **C（状态链抗饿死）** 修复：`orz-assurance/src/journal/recorder.rs`
的 `is_storage_full_error`（Windows 112／39、POSIX `ENOSPC`／`EDQUOT`）→ 退避梯 → **Degraded 骨架模式**
（只写链骨架、单事件 ≤8 KiB）＋ 终局 `degraded_complete` 分类。⇒ **这条已不是开放风险**。

**判定三：commit 那次（B）的根因是 A，且机理正是「orz 与工具树共用一个宿主 commit 预算」。**
设计档 F-4 已登记**盘—内存轴间耦合**：Run A 把 `D:` 写到 0 字节 ⇒ **系统托管页面文件在满盘上无法增长**
⇒ commit 在**名义上限的 73 %** 处即失败。Run B 那句 `memory allocation of 200720 bytes failed` 是
**orz 自己**在分配上失败，而 Rust 的默认行为是 `handle_alloc_error` → `abort`。⇒ 这条**仍然开放**，
且它恰好由 run 级 Job 上限（`JOB_OBJECT_LIMIT_JOB_MEMORY`）兜住：上限把「耗尽」关在树里，
让分配失败发生在**子工具**身上而不是 orz 身上。

**对用户问题的直接回答**：

- 「是不是不独立所以一起被杀」→ **不是**。是「树因 orz 死而死」，不是反方向。
- 「根本上是运行侧的问题吗」→ **是，但要分开说**：A 属 orz 自身的**状态链健壮性**（已修）；
  B 属 orz 自身进程**没有为自己预留预算**（未修），而它唯一的机械手段就是 Job 上限。
- ⇒ **退役准入门可以；退役 Job 上限不行。** Job 上限不是「准入」，它是
  「**把 OS 的答复交给子进程、同时保住 orz 自己**」的那一层。

「让系统回报结果」在 Windows 上可以直接落到内核原语：

| OS 原语 | 语义（微软原文口径） | 替代原本的哪一级 |
|---|---|---|
| `JOB_OBJECT_LIMIT_JOB_MEMORY` | 「进程试图提交超过作业总额的内存时，**它失败**」 | 硬档**拒绝** |
| `JobObjectNotificationLimitInformation` | 「注册作业**超过峰值内存但允许进程继续提交**的通知」＝ OS 级**软水位** | 软档降速 |
| `GetDiskFreeSpaceExW` ＋ 写入失败码（`ERROR_DISK_FULL` 112／`ERROR_HANDLE_DISK_FULL` 39） | 余量读数 ＋ OS 的失败答复 | 卷轴门 |

## §8 卷轴裁决落定（用户 2026-09-20）

用户裁决：「卷余量轴我同意可将为观测＋软提示，**余量到达一定限制的时候在进程下发前机械返回余量软提示**吧」。

⇒ 定案（并入 0aw）：

1. **卷轴不再阻断**——`HEAVY_RELEASE_FREE_BYTES = 8 GiB` 的「放行／拒绝」语义退役；
2. 每次重活派发**前**仍机械取一次**目标卷**读数（复用既有 `write_targets` ＋ `SystemCapacityProbe`），
   低于**软水位**时在派发结果里**附一条软提示**（机械读数 ＋ 短句），**不阻断、不改变动作**；
3. 软水位取值建议沿用既有阶梯档（`WATCH_FREE_BYTES = 16 GiB` 或 `SOFT_FREE_BYTES = 8 GiB`，
   **一字未定，待用户点选**）；
4. 真失败面回到 OS：写入失败（`ERROR_DISK_FULL`／`ENOSPC`）由既有 ENOSPC 分类链（0z C）承担；
5. 归属：**只动准入层**，不动 0z C 的降级链与 job 上限。

## §9 证据物

- 现场读数（Windows，2026-09-20）：`\Memory\Commit Limit` 26.19 GiB ／ `\Memory\Committed Bytes` 21.82 GiB。
- S3 拒绝原样（Linux 容器，15:59–16:24 UTC）：
  `D:\tb-eval\gsa-volumes\.quarantine\official-verify-timeout3-s3\{582d0289…\runs\RUN-CLI-6aaeb160, 7f781ed9…\runs\RUN-CLI-6aaeb62b}\events.jsonl`
  （`host_resource_denied` 25 条 ＋ `host_resource_snapshot` 3 条）。
- Linux 内核文档：Overcommit Accounting（`CommitLimit`／`Committed_AS` 仅 `vm.overcommit_memory=2` 生效）——
  <https://www.kernel.org/doc/html/v6.13/mm/overcommit-accounting.html>；`proc_sys_vm(5)`（CommitLimit 公式）——
  <https://www.man7.org/linux/man-pages/man5/proc_sys_vm.5.html>。
- 微软文档：WSRM 概述（**deprecated beginning with Windows Server 2012**）——
  <https://learn.microsoft.com/en-us/previous-versions/windows/it-pro/windows-server-2012-R2-and-2012/hh997019(v=ws.11)>；
  Windows Server 2012 R2 移除/弃用清单 ——
  <https://learn.microsoft.com/en-us/previous-versions/windows/it-pro/windows-server-2012-R2-and-2012/dn303411(v=ws.11)>。
- 代码面：`orz/crates/orz-host/src/resource_gate.rs`、`orz/crates/orz-host/src/lib.rs::install_resource_safety`、
  `orz/crates/codegen/xai-tty-utils/src/resource_job.rs`、`orz/crates/orz-assurance/src/sandbox/job_object.rs`。
- 首版调研（§1 实测／§2 业界对照仍有效）：[`N4 资源门调研`](N4_RESOURCE_GATE_INDUSTRY_RESEARCH_2026-09-20.md)。
