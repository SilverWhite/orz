# 宿主资源面：OS 委派执行设计（N4／0aw 定稿 v1.1，2026-09-20）

> **本档＝N4 的设计权威**。它**改写** [`HOST_RESOURCE_SAFETY_DESIGN_2026-09-12`](HOST_RESOURCE_SAFETY_DESIGN_2026-09-12.md)
> 的**子项 A（派发前资源预检门）**：A 由「准入拒绝」改为「**观测 ＋ 软提示**」；
> 子项 **B（进程树生命周期）／C（状态链抗饿死）／E（回收）／F（资源硬上限）不动**。
> 用户裁决链（2026-09-20，四问四答）：① 卷软水位 = **4 GiB**；② 裁决点 B **采纳**（Job commit 上限保留推导、**只作上限、不作拒绝**）；
> ③ 裁决点 C = **直接删除分类器**（不留观测标签）；④ **两条 0z S2 动作臂一并退役**（§10 裁决点：hard 档树杀 ＋ 回收阶梯触发）。
> 上游取证：[`N4 重定案`](audits/N4_OS_SCHEDULING_DELEGATION_2026-09-20.md)（§7 运行侧因果复核／§8 卷轴裁决）、
> [`N4 首版调研`](audits/N4_RESOURCE_GATE_INDUSTRY_RESEARCH_2026-09-20.md)（业界对照）。
> 状态：**设计定稿、未实施**（0aw 维持开放；零代码、零子仓改动）。

## §1 一句话设计

**orz 不做资源准入。** 内存与 CPU 的调度和限额交给操作系统（Windows：内核强制的 run 级 Job Object；Linux：部署给的
cgroup／systemd 限额），**磁盘余量降为派发前的机械软提示**；orz 只负责**发进程、收结果、把操作系统的答复如实转达**。

## §2 三层职责（定案）

| 层 | 谁 | 职责 |
|---|---|---|
| **裁决与执行** | 操作系统内核 | CPU 调度、提交记账与限额、分配失败／OOM／磁盘写失败的**答复** |
| **执行者** | orz | ① 建进程、挂进 run 级 Job、收结果、超时与进程树回收；② 派发前取一次**目标卷**读数，低于软水位时附**软提示**（不阻断）；③ 把 OS 的答复如实转成工具结果（`ERROR_COMMITMENT_LIMIT`／`ERROR_DISK_FULL`／`ENOSPC`／OOM 迹象） |
| **决策者** | 模型 | 读到软提示与真实失败后自行改方案；**不再承担资源预检**（机械层既不替模型预判，也不要求模型自觉） |

## §3 各资源轴定案

| 轴 | 定案 | 承载机制 |
|---|---|---|
| **内存（提交）** | **不做准入**；run 级 Job commit 上限**保留**（内核强制） | `JOB_OBJECT_LIMIT_JOB_MEMORY`——「进程试图提交超过作业总额的内存时，**它失败**」（微软原文口径）＝ OS 的答复，**不是** orz 被杀 |
| **CPU** | 不做准入 | `JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP` = 80 % |
| **进程数** | 不做准入 | `JOB_OBJECT_LIMIT_ACTIVE_PROCESS` = `2 × cores + 8`（≥16） |
| **磁盘** | **不做准入**；派发前取目标卷读数，**低于 4 GiB 附软提示**（不阻断、不改变动作） | 读：`GetDiskFreeSpaceExW`／`statvfs`；答复：`ERROR_DISK_FULL` 112／`ERROR_HANDLE_DISK_FULL` 39／`ENOSPC` |
| **进程树回收** | 不动 | `KILL_ON_JOB_CLOSE`（0z B） |
| **状态链（盘满）** | 不动 | ENOSPC 分类 → 退避 → Degraded 骨架模式 → `degraded_complete`（0z C） |
| **回收阶梯**（soft／reclaim_direct 档机械回收缓存） | **退役触发**（用户裁决 ④；回收**能力**降为「只在真 ENOSPC 面用」的候选，当前批不接线） | 原触发点＝门汇点 |
| **hard 档树杀**（`terminate_heavy_call_jobs` ＋ `resource_exhausted`） | **退役**（用户裁决 ④；它只在拒绝臂内触发，与「不做准入」不可两立） | 原触发点＝拒绝臂 |

**为什么 Job 上限是承重件（保留依据）**：2026-09-12 Run B 的死因是「满盘 ⇒ 系统托管页面文件无法增长 ⇒
commit 在**名义上限的 73 %** 处失败 ⇒ **orz 自身** Rust `handle_alloc_error` abort」。
orz **不在** Job 内（它是 Job 的持有者），所以只有 Job 上限能把「耗尽」关在**子工具**身上。
⇒ **退役准入门可以，退役 Job 上限不行。**

## §4 参数表（定案）

| 参数 | 值 | 备注 |
|---|---|---|
| 卷软水位 `VOLUME_HINT_FREE_BYTES` | **4 GiB** | 用户裁决；触发条件＝目标卷 `free < 4 GiB` |
| run Job commit 上限 | `min(min(80 % × limit, limit − 4 GiB), 装配期 commit 余量 − 1 GiB)`，下限 2 GiB；算得 ≤0 ⇒ 不设 | 用户裁决（点 B）：**保留推导、只作上限** |
| run Job 进程数 | `2 × cores + 8`，≥16 | 保留 |
| run Job CPU rate | 80 % | 保留 |
| 25 % commit headroom 门 | **删除** | `HEAVY_RELEASE_COMMIT_HEADROOM_PERCENT` |
| `HEAVY_RELEASE_FREE_BYTES`（8 GiB 放行门） | **删除** | 放行／拒绝语义退役 |
| 动作分类器（`ActionClass`／`HEAVY_TOOLS`／`HEAVY_PROGRAMS`／`CONDITIONAL_PROGRAMS`／`classify_action`／`classify_command`／`WRAPPER_PROGRAMS`／`COMMAND_FLAGS`／`strip_payload_bodies` 等**仅服务分类**的件） | **整体删除** | 用户裁决（点 C） |
| tier 阶梯（`normal`／`watch`／`soft`／`reclaim_direct`／`hard`／`unknown`） | **保留，但只作观测标签** | 它是读数摘要，不是分类器；`host_resource_snapshot` 的 `tier` 字段与软提示语境继续用它（删除会牵动事件族＋verifier＋e2e 期望集且无收益） |

## §5 三个面的去向（定案）

| 面 | 现状 | 定案 |
|---|---|---|
| **模型面（工具结果）** | 拒绝信封：`resource_insufficient` ＋ 中文定案句 ＋ 读数 | **软提示**：动作照跑；结果头部附一行 `[资源软提示]`（中文短句 ＋ 英文机械读数，沿用 0af 的混排定案）。**每 run 每卷只提示一次**（机械去重，避免刷屏，与 0ar 去噪纪律同向） |
| **journal 面** | `host_resource_denied`（pre-issue 拒绝）＋ `resource_exhausted`（hard 档树杀 planned／executed）＋ `reclaim_performed`（回收）＋ `host_resource_snapshot`（跨档） | 前三条**生产端退役**（不再产生新事件）；**schema 与 verifier 全部保留**（历史 journal 仍可校验，`journal/families.rs` 不动）。`host_resource_snapshot` **保留**（跨档落盘） |
| **进程面** | run 级 Job（内核强制）＋ 每次调用子 Job（kill-on-close） | **不动** |

## §6 代码落点（定案）

分类器与准入面的引用面**只有两个文件**（`rg` 实测）：

| 文件 | 改动 |
|---|---|
| `orz/crates/orz-host/src/resource_gate.rs` | **重命名为 `resource_hint.rs`**（「gate」语义已退役，留名会误导）；删掉分类器与准入面；**保留** `HostCapacitySnapshot`／`CapacityProbe`／`SystemCapacityProbe`／`write_targets`（目标卷解析）／`tier_for`／`gib`；新增软提示构造与每卷去重 |
| `orz/crates/orz-host/src/lib.rs` | 接线改写：`install_resource_safety` 保留（探针＋`default_job_limits`＋`install_global_run_job`）；删除 `evaluate`／`Refuse` 分支、`host_resource_denied` 注入、**hard 档树杀臂与回收触发**（裁决 ④）、`terminate_heavy_call_jobs`、`LiveCallJob::action_class` 字段；**保留** `live_call_jobs` 登记表（`DispatchGuard::drop` 的 per-call job 句柄关闭）、跨档快照与 `drain_host_resource_facts`；测试面按 §7 改写 |
| `orz/crates/orz-host/src/process_tree.rs` | 登记行去掉 `action_class` 字段（其注释写明「供 hard 档面枚举重档 call_id 集」——该面已退役）；其余三条件扫除逻辑不动 |

> **落点勘误（2026-09-20 实施前复核 → 同日裁决 ④ 收口）**：上表首版说「引用面只有两个文件」只对**分类器符号**成立；
> 门的**动作臂**另有耦合面（`live_call_jobs` 登记、`process_tree` 登记行、回收阶梯）。经裁决 ④ 一并退役后，
> 受影响文件＝上表三个 ＋ `journal/families.rs`（**只保留、不改**）。

**不动的**：`xai-tty-utils/src/resource_job.rs`（run 级 Job 与 `default_job_limits` 调用面）、
`orz-assurance/src/sandbox/job_object.rs`、`orz-assurance/src/journal/families.rs`（两个事件的 verifier）、
`orz-loop/src/{controller.rs, host_exec/tool_run.rs}` 的 `journal_pending_host_resource_facts` 排空链。

**方法名保留** `OrzHost::with_host_resource_safety`（生产调用点 = `orz-bin/src/main.rs` 与 `orz-host/src/acp_server.rs` 各一处）；
语义由「门 ＋ 上限」改为「上限 ＋ 卷软提示」——改名牵动三处调用与测试且无收益。

## §7 判据（验收）

1. **拒绝归零**：同语料回放 `host_resource_denied` 事件数 = **0**，工具结果里不再出现 `resource_insufficient`；
2. **软提示可核**：注入 `free < 4 GiB` 读数下，派发**仍执行**且结果带一次软提示；同一卷第二次派发**不再重复**提示；
3. **内核上限在位**：Windows `RunResourceJob` 读回非空（commit／active_process／cpu_rate），既有端到端钉子保持绿；
4. **无 OOM 杀**、run 墙钟不因等待恶化（对照 `host_resource_snapshot` 与 run 墙钟）；
5. **观测面保留**：`host_resource_snapshot` 按跨档落盘；既有 verifier（`host_resource_denied`／`resource_exhausted`／
   `reclaim_performed`）对历史 journal 仍绿；
6. **动作臂归零**：同语料回放 `resource_exhausted = 0` 且 `reclaim_performed = 0`（S3 基线本就为 0，改造后不得回涨）；
7. **边界不动**：不动 0z B/C/E/F 中**未被本裁决触及**的部分（进程树生命周期、状态链降级、Job 硬上限），
   不动官方环境口径（`task.toml`／镜像／verifier／数据集 pin）。

## §8 边界与风险

- **「没有限额」≠「OS 保证 orz 存活」**：Linux 真机裸跑无强制面，多进程争抢的后果由内核决定（OOM killer 可杀机器上任何参与者）。
- **内存轴没有「预派发软提示」**：OS 对内存的答复以分配失败形式到达（时点在派发之后），故内存轴只有观测与 Job 上限，没有派发前门——这是**有意**的（避免再造一个预判）。
- **撤掉准入会改变个别重活的行为**：低余量下重活可能变慢或失败，这是把「预判」换成「如实失败」的语义变更。
- **`host_resource_denied` 退役后事件面变窄**：判据 1 正是用它归零来验收；历史 journal 的可校验性由 verifier 保留兜住。

## §9 与既有权威的关系

- 本档改写 [`HOST_RESOURCE_SAFETY_DESIGN_2026-09-12`](HOST_RESOURCE_SAFETY_DESIGN_2026-09-12.md) 的**子项 A**；
  该档 §4.7 的 F 子项（硬上限）**继续有效**，仅其 commit 上限的**用途**由「准入依据」明确为「只作上限」。
- ADR-0010 §14 转录（A 子项的语义改写）**随实施批**，不在本档内做。
- 索引 `GAP-HOST-RESOURCE-ADMISSION-CALIBRATION`（0aw）状态维持 `pending`（设计定稿、实现未落）。

## §10 实施耦合点（2026-09-20 实施前复核发现；**同日裁决 ④ 收口**）

门的**拒绝臂**里挂着两条 0z S2 的动作机制，它们与准入拒绝共用同一个触发点；删掉准入，这两条就失去触发器：

| 机制 | 现触发点 | 证据 |
|---|---|---|
| **hard 档树杀**：`terminate_heavy_call_jobs`（只杀在跑**重档**调用的 call job，2026-09-13 用户裁决的 (a) 形态）＋ `resource_exhausted` 事件 | **只在 `GateDecision::Refuse` 且 `tier == hard` 臂内** | `lib.rs` ~1403–1460 |
| **回收阶梯**：`run_reclaim_pass`（soft／reclaim_direct 档机械回收缓存）＋ `reclaim_performed` 事件 | 只在门汇点（`lib.rs` ~1357）与 hard 臂（~1460） | 同上 |

**S3 实测（2026-09-20）**：三题 journal 中 `reclaim_performed = 0`、`resource_exhausted = 0`
（同期 `host_resource_denied` 14／11、`host_resource_snapshot` 3／1）⇒ **两条动作臂在真机上都没动过**，
门实际产出的只有拒绝。

**不受影响的面（复核确认）**：`live_call_jobs` 登记表**保留**——`DispatchGuard::drop` 用它关掉本次调用的
per-call job 句柄（`KILL_ON_JOB_CLOSE` 的调用级拆树），与分类器无关。受分类器牵连的只有登记行与结构体上的
`action_class` 字段（注释写明「供 hard 档面枚举重档 call_id 集」）。

### §10.1 裁决（2026-09-20 用户令「这两条 0z S2 动作臂一同退役吧」）

**两条臂一并退役**（选项 A）。定案内容：

1. **hard 档树杀退役**：删 `terminate_heavy_call_jobs` 与 `resource_exhausted` 事件的生产端
   （planned／executed 两条事实不再产生）。理由＝它只在**拒绝臂**内触发，与「不做准入」不可两立，
   且属「预判 + 替代 OS 动手」。
2. **回收阶梯的触发退役**：删门汇点与 hard 臂内的 `run_reclaim_pass` 调用与 `reclaim_performed` 生产端。
   回收**能力**降为候选（「只在真 ENOSPC 面用」，0z C 已有 ENOSPC 降级链），**当前批不接线**。
3. **schema 与 verifier 保留**：`host_resource_denied`／`resource_exhausted`／`reclaim_performed`
   三条的 schema、fixtures、Python 镜像与 `journal/families.rs` 全部**不动**——历史 journal 继续可校验。
4. **登记面**：`live_call_jobs` 登记表**保留**（`DispatchGuard::drop` 依赖它关本次调用的 per-call job 句柄）；
   只删其上与分类器绑定的 `action_class` 字段（`LiveCallJob` 与 `process_tree` 登记行两处）。
5. **证据支持**：S3 三题 journal 中两条臂的产出本就为 0（`resource_exhausted = 0`、`reclaim_performed = 0`），
   退役不改变真机既成事实，只是把「未触发」变成「不再存在」。
