# 真机资源安全边界与崩溃收尾设计（2026-09-12）

> 登记：BACKLOG **0z** / TODO **P0-0z** / 索引 `FUS-HOST-RESOURCE-SAFETY` +
> `GAP-READ-FILE-TEXT-ENCODING`。状态：**设计（待用户裁决放行实施）**。
> 入口证据（本轮实机长任务）：
> [`0v 第二批 S4 实机复验记录`](audits/0V_S4_BATCH2_LIVE_VERIFICATION_2026-09-12.md) §4；
> run `RUN-CLI-6aa4f384`（`.gsa/runs/`，4,982 事件 / 98.2 min，**盘满致命退出**）；
> run `RUN-CLI-6aa50fdf`（798 事件 / 20.2 min，**commit 耗尽 abort**）；
> 复扫 `tmp0vc/rescan_stdout.txt`（622 份 journal；2026-09-13 取证现场归档至 `存档/root-artifacts-2026-09-13/tmp0vc/`）。
> 边界：本设计不修改 ADR-0010 已冻结语义，只在其机械层上新增「宿主资源」这一
> 事实面与门控面；不改变模型工具面语义（除 §4.4 的编码宽容）。

## 1. 触发事实（2026-09-12 真机自举首跑）

- **无沙箱事实（用户 2026-09-12 确认）**：两轮运行均**未挂沙箱**，是真机直跑。
  本设计据此在框架内重建最小等价边界（见 §2 第 6 条与 §3.4）。

同一台宿主机上，orz 0.4.3 以工作区 `D:\CLI` 自举修复 0v-C，两轮均**非正常终止**：

| run | 窗口 | 规模 | 终止形态 | 直接错因 |
|---|---|---|---|---|
| A `RUN-CLI-6aa4f384` | 14:39–16:17（98.2 min） | 4,982 事件 / 515 模型轮 / 665 工具 / 53 失败（8%） | `error: journal error` 致命退出 | `resources_state.json` 写失败（WARN）→ journal 写失败 `os error 112`（盘满） |
| B `RUN-CLI-6aa50fdf` | 16:39–17:00（20.2 min） | 798 事件 / 79 模型轮 / 110 工具 | 进程 abort，无终止事件 | `memory allocation of 200720 bytes failed`（commit 耗尽；同批工具日志另见 `os error 1455` 页面文件太小） |

关键量：

- 写盘面：`D:\CLI\orz\target` 事后 **30.57 GB / 44,025 文件**（debug 全量），
  编译期另见 `LLVM ERROR: IO failure on output stream: no space on device`。
- 内存面：宿主 commit limit **30.97 GB**、峰值已提交 **22.84 GB**、可用物理内存 **4.8 GB**
  （`Get-Counter` 实测，构建仍在跑时）。
- 动作面：Run A 的 379 条终端命令中 **54 条为 cargo 族**（16 test / 5 build / 5 run /
  2 clippy / 2 fmt）——模型把真机当开发机自我编译，资源画像与评测路径完全不同
  （对照：0v S4 评测 run 334 事件 / 31 轮，强度差约 **16 倍**）。
- Run B 的**最后一条动作**是模型自发执行的「Check disk and memory headroom」
  （`Get-PSDrive C,D` + `Get-CimInstance Win32_OperatingSystem`）——该由机械层做的事，
  模型自己想到并做了，然后死在这一步。
- 并发事实（用户裁决 2026-09-12）：两轮卡死时隔壁 GLM/ZCode 会话**仅在监听**
  （`sleep` 轮询），未参与负载——压力源是 orz 自身动作，不是外部竞争。

机制面按设计工作的部分（本批不改）：权限门 665 请求 / 665 决策 / **0 拒绝**、
票据链 415+415、机械审查 919 条更新、orientation 11 次（515 轮 ÷ 50 阈值，节拍准确）、
TER 自动后台化在 180 s 边界命中 2 次（183 s / 186 s）。

## 2. 设计边界与不变量（用户裁决）

1. **不换盘、不换卷**：重活（编译、测试、大写入）**必须**在动作发起时所在的工作区
   与卷上完成；框架不得把重活"改道"到其他卷、临时盘或缓存卷。
2. **框架不接管宿主调度**：不排队宿主其他进程、不调整页面文件、不清理用户数据、
   不杀非本 run 的进程。框架只做四件事：**预检、降级、收尾、观测**。
3. **回收只限可再生面**（用户裁决 2026-09-12，及同日追加）：构建产物/依赖缓存/本 run scratch
   可按 §4.6 白名单自动回收；`.gsa` 会话卷、journal、证据目录、用户源码与文档、备份
   **永不自动回收**，其余清理只由用户显式发起；框架最多给出读数与建议。回收采用**延迟删除**
   （轮数窗口，默认 2 轮）且**不使用系统回收站**；撤销面与边界见 §4.6.1。
4. **fail-closed 优先于可用性**：余量读数不可得时按"不足"处理（拒绝重活），
   而非放行。
5. **审计链优先于一切**：链完整性是这套系统的价值锚；资源耗尽时允许丢事件，
   **不允许丢链骨架与终止形态**。
6. **不挂沙箱**（2026-09-12 用户裁决方向）：不引入容器/cgroup 前提；以「机械硬门 +
   回收兜底 + 必在收尾」三条共同替代沙箱，放弃项与残余风险在 §3.4 显式登记。
7. **机械层优先 + 信息返回**（用户 2026-09-12 裁决；沿用 `OPS-PROTOCOL` 既有正典
   "**模型只表达意图，判断全部下沉到机械层**"）：凡能机械化的判断一律机械化（读数、分档、
   准入、回收、收尾），机械层把读数与结论**作为信息返回**给模型面；不得设计成"要求模型自觉
   检查/自觉收敛"的机制。反例证据：Run B 的最后一条动作就是模型自发执行磁盘/内存检查。

## 3. 成熟产品怎么做（借镜与不借镜）

> 调研方式：本章为 2026-09-12 一手来源检索的摘要；完整来源清单与原文摘录见
> [`HOST_RESOURCE_MANAGEMENT_BORROW_RESEARCH_2026-09-12`](HOST_RESOURCE_MANAGEMENT_BORROW_RESEARCH_2026-09-12.md)
> （12 份官方文档/官方 man 源；未取得项亦在该文件 §5 列出）。本章每条机制断言均可回溯到该文件。

### 3.1 五条主流路线

| 路线 | 代表机制 | 要点 |
|---|---|---|
| 内核/容器配额 | Linux cgroup v2（`memory.max`/`memory.high`、`pids.max`、`io.max`）、K8s ephemeral-storage、Docker `--memory`/`--storage-opt`、Firecracker/gVisor | 资源边界**外包给内核**，进程越界先被限流（`high`）再被杀（`max`），应用自身不写资源逻辑 |
| 压力信号驱动 | PSI（`/proc/pressure/{memory,io,cpu}` `some/full avg10`）、`memory.events`、systemd-oomd、Windows 低内存/低磁盘事件 | 用**压力**而非绝对值触发，响应谱固定为 throttle → reclaim → kill |
| 准入控制与排队 | K8s QoS（Guaranteed/Burstable/BestEffort）+ 驱逐顺序、admission webhook、Slurm/PBS、Ray、Temporal worker slot | request/limit 契约 + 队列 + 抢占；**重活不并发抢同一资源** |
| 写面保留与降级 | journald `SystemMaxUse`（默认 10% 卷、上限 4 G）+ RateLimit + **满时停写但仍运行**；数据库 WAL 满 → 拒绝写入/只读；K8s 默认驱逐阈 `nodefs.available<10%` | 日志/状态面的满不是进程死；是**降级 + 保留量** |
| 构建系统纪律 | cargo target 目录锁（同 target 串行）、Bazel 磁盘缓存上限 + LRU、sccache | 缓存可弃、锁化并发；**磁盘预算**由构建系统自己管 |

编码类 agent 产品（Codex / Claude Code 等）主流做法是**把资源边界推给沙箱**：
容器或 microVM 里跑，配额由内核执行，agent loop 本身不做资源调度。orz 的真机模式
（无容器、直连宿主、工作区就是宿主仓库）**没有这层沙箱**，因此必须在框架内
重建一个最小等价物——这正是本设计的立意。

### 3.2 可借鉴（落到 orz 的映射）

1. **分级契约 + 准入**（K8s QoS / Slurm）→ 动作分档（轻/重）+ 重活派发前读数准入，
   **不做改道**（受 §2 约束）。
2. **压力信号 + 三级响应**（cgroup v2 + PSI）→ 卷余量/commit 余量读数驱动
   watch → soft（拒新重活）→ hard（树杀 + 收尾）。
3. **写面保留量 + 满时降级**（journald / WAL）→ 状态链 reserve + ENOSPC 降级，
   **不因审计面写失败杀死 run**。
4. **生命周期归 OS**（Job Object / cgroup）→ Windows Job Object `KILL_ON_JOB_CLOSE`
   回收工具进程树，父进程 abort 不再留孤儿。
5. **观测面读数进模型可见面**（K8s metrics-server / PSI 暴露）→ 余量读数进
   `blackboard_read section=session`；但**硬门仍是机械门**，不依赖模型自觉。

### 3.3 明确不借鉴

- 不做全局调度器（不排队/不抢占宿主进程，不跨 agent 协调）。
- 不引入容器/cgroup 化前提（真机直跑是产品形态，不是缺陷）。
- 不做"自动清缓存/自动删产物"式自救（与 §2 第 3 条冲突）。
- 不采用"绝对值单阈值 + 杀进程"式粗响应（会误杀长构建，且无法收尾）。

### 3.4 为什么「机械层 + 回收 + 收尾」可以替代沙箱（用户提问 2026-09-12）

沙箱实际提供三样东西：①**内核强制**的资源上限（不依赖被管进程配合）；②爆炸半径限制
（写/执行范围被约束）；③可复现环境。本设计不引入沙箱（真机直跑是产品形态），因此逐条给出替代，
并显式登记放弃的残余风险：

| 沙箱提供的 | orz 的替代 | 等价性 |
|---|---|---|
| 资源上限（cgroup / Job 内存上限） | §4.1 预检门 + 三档响应 + §4.7 并发限流 | **近似**：依赖框架自身读数与注入，非内核强制；读数不可得即 fail-closed |
| 进程树生命周期回收 | §4.2 内层 Job Object（Windows 内核强制）+ 归属三条件扫除 | Windows 等价；Linux 侧稍弱（进程组 + 登记兜底） |
| 写面受控 | §4.6 回收机制（可再生白名单 + 审计先写 + 有界冷却） | **不等价**：回收只在阈值下触发，仍存在"写满瞬间"窗口 |
| 状态面不受损 | §4.3 抗饿死（reserve + Degraded 骨架 + 显式终止形态） | 就审计链而言等价 |

残余风险（用户授权方向下接受，登记在案）：单租户真机上，模型发起的进程仍可写到工作区之外、
可长时间占满 CPU/IO、可超出本框架声明的 commit 上限；框架只保证**自己发起、自己登记**的动作
可被预检、回收与收尾。

**结论**：不挂沙箱成立的前提是三条同时成立——机械门是**硬门**、回收是**机械且预授权**的、
收尾形态**必在**；缺任何一条，该结论不成立。

## 4. 机制设计

### 4.1 缺口 A：派发前资源预检门（fail-closed）

**接缝**：`OrzHost::call_tool`（权限门、`web_search` semaphore 同一汇点）+
`orz-tools/src/types/resources.rs`（已存在 `SessionVolumeRoot` /
`SessionVolumeAccess` / `SessionVolumeReadVerdict` 一族）。

**新增只读资源**：`HostCapacitySnapshot`（host 装配期注入探针句柄，同 SessionVolume
形态）：`{ collected_at, volumes: [{ mount, free_bytes, total_bytes }], commit: { limit_bytes,
used_bytes, free_bytes }, source_quality }`。读数不可得 → `source_quality=unavailable`
→ 按 §2 第 4 条 fail-closed。

**动作分档**（静态、可机械核对，不由模型自述）：

- 轻档（默认）：读类工具（`read_file`/`grep`/`blackboard_read`/`list_dir`）、
  小写入（`search_replace`）、无重定向终端命令。
- 重档：终端命令按**静态命令分类器**命中（`cargo`、`rustc`、`dotnet`、`msbuild`、
  `docker`、`npm|pnpm|yarn install`、`pip install`、`go build`、`cmake`、`make`、
  大输出重定向 `> file`、`7z`/`tar` 解压等）。
- 未命中且无法分类 → 默认**轻档**；命中重档模式集合的一律重档（白名单轻、黑名单重）。

**门判据**（阈值可配；**2026-09-12 用户裁决照此取值**）：

```
重档放行条件 = 目标卷 free_bytes ≥ max(8 GiB, 3 × 预计写入)      # 估计缺失时用 8 GiB
             ∧ commit.free_bytes ≥ 25% × commit.limit_bytes
不足 → 拒绝并回传读数（resource_insufficient，pre-issue 家族，与 budget_insufficient 同族）
```

**运行中采样与三级响应**（仅在存在在跑重活时启用，5 s 周期，跨档位变化才落事件）：

| 档 | 触发 | 响应 |
|---|---|---|
| watch | free < 16 GiB 或 commit 使用 > 70% | 落事件 + session 面可见（不改行为） |
| soft | free < 8 GiB 或 commit 使用 > 85% | 拒绝新的重档派发；在跑重活允许完成 |
| **reclaim-direct** | **free < 5 GiB** | **跳过轮数窗口直删 `cache` 类**（§4.6.1；审计先行） |
| hard | free < 2 GiB 或 commit 使用 > 95% | 树杀重活（§4.2）→ 落 `resource_exhausted` → 走 §4.3 收尾 |

档位的**机器键**统一为 snake_case（`normal` / `watch` / `soft` / `reclaim_direct` / `hard` /
`unknown`）：散文名 `reclaim-direct` 不进入事件、fixture 与法官比对（2026-09-12 S1.1 命名收口；
`unknown` 见 §4.7.1 第 6 条）。

**与 TER 的关系**：TER 管时间轴（去自身硬超时、自动后台化、idle-kill），本条管空间轴；
两者正交，互不覆盖。hard 档的树杀走同一个 kill 面（`kill_foreground_commands` /
`kill_all_background_tasks`），不新开杀进程路径。

### 4.2 缺口 B：进程树生命周期（崩溃路径回收）

**现状缺陷**：`spawn_shell_command` 用 `CREATE_BREAKAWAY_FROM_JOB |
CREATE_NEW_PROCESS_GROUP` + `kill_on_drop(true)`，回收依赖 Rust 析构；
`std::process::abort`（alloc 失败路径）**不跑析构** → 子构建可能变孤儿。

**方案**：

1. **Windows 两级 Job（2026-09-12 裁决）**：**每个 run 一个顶层 Job** 承载 §4.7 的资源上限
   （commit / 进程数 / CPU，跨工具调用汇总计量）；**每次工具调用一个子 Job** 承载
   `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`（树杀粒度）。子进程先 `CREATE_BREAKAWAY_FROM_JOB`
   逃逸外层（Codex app 宿主 job，保留现状），再 `AssignProcessToJobObject` 进顶层 Job，
   工具调用时挂入对应子 Job（Windows 8+ 支持 Job 嵌套）。**父进程无论怎么死，job 句柄关闭即整树回收**。
   **指派顺序 = 先根后子**（S1.1 复核补注，2026-09-12）：子进程先指派进 run 级顶层 Job，
   再指派进本次调用的子 Job——这正是微软 Nested Jobs 页给出的层级构建规则
   （"first assign all processes to the job at the root of the hierarchy, then assign a
   subset of processes to the immediate child job object"）。**手工把 job 句柄当进程句柄
   传给 `AssignProcessToJobObject` 与嵌套能力无关**：S1 曾以该调用报 `ERROR_INVALID_HANDLE`
   推断"嵌套不可用"并降级为一级形态，独立复核（[`0Z_S1_INDEPENDENT_REVIEW`](audits/0Z_S1_INDEPENDENT_REVIEW_2026-09-12.md) F-1）
   已实测同一进程"先普通 job、后带限 job"两次指派均成功且上限仍咬合。聚合语义有一手来源：
   `JOB_OBJECT_LIMIT_JOB_MEMORY` 原文为 *"causes all processes associated with the job to
   limit the job-wide sum of their committed memory"*（Job Objects / 限项结构页），
   故顶层 Job 上的 commit / 进程数 / CPU 即**本 run 的汇总上限**。实现落点：
   `xai-tty-utils/src/resource_job.rs`（`RunResourceJob` 持 run 句柄到进程生命周期）
   + `ProcessGroup::attach_pid`（root → call 两次指派）。
2. **Linux/macOS**：保留进程组（`setsid`）+ 会话卷登记（下条）作为兜底。
3. **会话卷登记**：`.gsa/process_trees/<call_id>.json` 记
   `{ pid, parent_chain, cmd_sha256, started_at, job_name }`。
4. **孤儿扫除（sweep，2026-09-12 裁决：三条件 + 三条硬化）**：run 启动时与异常收尾时执行，
   **三条件同时满足才杀**：①父链已死；②`cmd_sha256` 与本 run 登记记录匹配；③`started_at`
   落在本 run 窗口内。三条硬化：**(a) 只处理登记表内的 pid**（无登记不杀）；**(b) 指纹按
   "规范化后的 argv 序列 + 关键 env"哈希**（避免引号/路径写法差异导致误判或漏判）；**(c) 先落审计
   `process_tree_reaped(sweep_planned)` 再杀，且保留一个可回退间隔**（默认 0——即立即执行，但事件
   先于动作）。该设计是上一轮误杀外部构建（归属判断错误）的机械修正——**归属不明一律不杀**。

**事件面**：`process_tree_reaped { call_id, pids, reason: parent_abort | run_shutdown }`。

### 4.3 缺口 C：状态链抗饿死（ENOSPC 降级与显式收尾）

**现状缺陷**：journal 写失败一路冒泡（`orz-assurance/src/lib.rs` 的
`journal error: ...`）到 CLI 致命退出；Run A 即此形态——**盘满把审计链自己打死了**。

**方案**：

1. **错因分类**：`JournalRecorder` 写错误区分 `Io(StorageFull)` 与其他 IO。
2. **ENOSPC 三退**：退避重试（10 / 40 / 160 ms）仍失败 → 进入 **Degraded 模式**。
3. **Degraded 模式语义**：
   - 只允许落**链骨架必要事件**：`sequence`/`previous_event_sha256` 连续、终止事件、
     degraded 摘要；
   - 丢弃可再生面（mechanical_audit / snapshot / ledger fold / 大 payload）；
   - 单事件有界（默认 ≤ 8 KiB，超出以「摘要 + 指针」形态落盘）。
4. **保留量（reserve）**：状态链所在卷预留 `ORZ_JOURNAL_RESERVED_BYTES`
   （默认 64 MiB），保证 hard 档仍能落终止事件；reserve 内不得被构建产物占用——
   由 §4.1 的重活判据（free ≥ 8 GiB）间接保证。
5. **显式终止形态**：`run_terminated { reason: resource_exhausted | journal_degraded }`
   **必须存在**；若连它都写不下 → 落固定形态侧车 `.gsa/runs/<run>/TERMINAL.json`
   （< 4 KiB）+ stderr，同卷（受 §2 不换卷约束）。
6. **只读语义**：`journal-conformance` 对 degraded 卷给出新分类
   `degraded_complete`（链骨架完整、事件不完整），**不得**与 `invalid` 混判；
   历史 622 份 journal 的既有分类不变。
7. **与 0v-C 的关系**：0v-C（`ba934af8`）修的是"链写入的内容哈希与推进值分叉"；
   本条修的是"链能否存在、能否闭合"。两条独立，互为前提。
8. **入口 fail-closed（2026-09-13 用户裁决，S2R 裁决 16）**：run 装配期的
`run_preflight` 写入遇 DegradedDropped（降级拒绝）时**拒绝 run 启动**——
盘满时不起新 run，避免写入面在满盘状态下扩张（不换卷约束下不向任何位置
新增写入压力）；此为既有 bootstrap 行为的显式确认，非降级语义破洞。

### 4.4 缺口 D：工具面文本编码宽容（摩擦项）

**既有接缝（2026-09-12 回查）**：机械编码门**已经存在**——索引 `GAP-ENCODING-GATE`
（`implemented`，2026-08-13）：`orz-tools/src/util/encoding.rs` 已实现
「BOM 剥离 → UTF-8 严格 → GB18030 → lossy」解码链并记录命中编码，工具面已有
`tool_completed.output_encoding` 可选字段。**本子项不新造字段**，只做一件事：
把 `read_file` 的 `is_binary` 判定**接到该门之后**（先试解码，解码失败才判二进制），
返回值复用既有 `output_encoding`。下方 §4.4 原文中的 `encoding_detected` 一律以
既有 `output_encoding` 为准。

**现状摩擦**：模型把命令输出重定向到文件后，`read_file` 以
`crate::util::binary::is_binary(extension, bytes)` 判定拒读并回报
`Cannot read binary file`（console 实证 Run B 3 例；journal 内 0 条 → 机械审查与
评估面完全看不见这类摩擦）。PowerShell 5 的 `>` 重定向默认 UTF-16LE，被
`content_inspection` 误判为二进制。

**方案**（`orz-tools/src/implementations/grok_build/read_file/mod.rs` 判据前插一层）：

1. 扩展名属**文本族**（`.txt .log .md .json .jsonl .csv .tsv .rs .toml .yaml .yml .py .ps1 .sh .diff .patch` 等）
   时先做编码嗅探：UTF-8 BOM → UTF-16LE/BE BOM → 无 BOM 的 NUL 交替启发式
   （UTF-16）→ GB18030 试解码。
2. 解码成功 → **按 UTF-8 返回**，工具结果附 `encoding_detected`（原编码）与
   原始字节 `source_sha256`；大文件读取契约（16 KiB 粗门 / 句柄信封）不变。
3. 解码失败或命中真二进制特征 → 维持现有拒读语义，文案不变。
4. 非文本族扩展名行为不变（`BINARY_EXTENSIONS` 仍优先）。
5. 事件面：工具结果字段 `encoding_detected` 进机械审查汇聚行（使该类摩擦
   **可被评估面看见**，这是本条的一半价值）。

### 4.5 观测面

- `blackboard_read section=session` 新增只读行：目标卷余量、commit 余量、
  当前档位（normal/watch/soft/hard）、在跑重活计数。
- 读数**只作信息**；硬门在 §4.1，不由模型自觉替代。

### 4.6 缺口 E：回收机制（承接 OPS-PROTOCOL 删除安全）

**回查结论（2026-09-12）**：该机制**不是新发明**，项目已有登记在先——
`OPS-PROTOCOL`（`pending`，2026-08-13，[`protocol/structured-operation-protocol-v0.1.md`](../protocol/structured-operation-protocol-v0.1.md)）
§6「删除策略：分类 + 超限」已定：**删除默认回收站；缓存分类放行；非缓存且超回收站容量 → 拒绝
（fail-closed）**；2026-08-13 审查判定保留「删除安全」（回收站 + 缓存机械分类 + 容量
fail-closed）为 host-owned 工具；2026-08-16 成熟复用评估认定 Windows 回收站（BitBucket）
与 XDG trash 为成熟 OS 约定。本子项**承接该正典的分类与 fail-closed 精神**，但**不采纳其
回收站实现**（理由见 §4.6.1：回收站同卷不释放空间，且轮数窗口已覆盖安全需求）。

**分类与动作**（沿用 OPS 口径）：

| 分类 | 判定 | 动作 |
|---|---|---|
| `cache` | 可再生成内容（本工作区构建产物、依赖/包缓存、本 run scratch） | **延迟删除**（进入待删集合，窗口到期真删；记录分类证据） |
| `unknown` / 归属存疑 | 无法归类或指纹不匹配 | **拒绝**（fail-closed；不进回收站，见 §4.6.1） |
| 证据面 / 用户面 | `.gsa` 会话卷、journal、证据目录、用户源码与文档、备份 | **拒绝**（永不自动回收） |

**关键工程事实（回查所得，用于解释为何最终不采纳回收站）**：

1. **回收站是卷内目录：同卷"进回收站"不释放空间**。Windows 官方 API 文档明确：
   `SHFileOperation` 默认**永久删除**，置 `FOF_ALLOWUNDO` 才"送到回收站"；
   要保证不进回收站则用 `DeleteFile`。因此**回收站只解决"可恢复"，不解决空间压力**——
   真正腾空间只能靠 `cache` 分类的直接删除（或回收站 FIFO 挤出旧条目）。
2. **进回收站不等于可恢复**（OPS §6 原文）：回收站满时旧条目会被挤出，审计必须区分
   `trash` / `permanent` / `rejected` 三种结果。
3. **容量口径**（沿用 OPS）：Windows 查目标卷回收站 `MaxCapacity`
   （注册表 `BitBucket\Volume\{GUID}`），缺失时按卷容量 **10%** 估算；POSIX 侧用策略阈值
   `OPS_TRASH_MAX_BYTES`（默认 5 GiB）。

**触发与顺序**：soft 档（free < 8 GiB）由机械层发起回收；hard 档（< 2 GiB）回收 + §4.2 树杀。
顺序：本 run scratch → 本工作区构建/包缓存 →（限时间窗）更早 run 的 scratch。

**四条纪律（防删错）**：

1. **先落审计**：`reclaim_performed { class, paths, fingerprints, outcome, freed_bytes }`
   必须先于删除落盘（回收行为本身不得破坏链）。
2. **白名单 + 归属指纹**：路径规范化后必须落在白名单根内且指纹匹配；**归属不明一律不删**；
   拒绝把允许根、盘根、文件系统根作为目标（OPS 原文纪律）。
3. **不裸删**：破坏性原语必须走执行器安全 API（作用域校验 + 审计先行），不裸 `rm` / `Remove-Item`。
4. **有界 + 冷却**：单次回收量上限、回收间隔冷却、单 run 回收总量上限——避免删-建-删震荡。

**回报**：分类、动作、`outcome`（`pending_delete` / `permanent` / `rejected`）与回收后余量
进 session 面读数（§4.5）。

#### 4.6.1 迭代与终裁（用户 2026-09-12 二轮/三轮裁决）：延迟删除 + git 兜底，**取消回收站**

**问题**：容量溢出策略会把"该不该清"推回模型（回收站满 → 要么让模型二次裁决，要么静默挤出旧条目），
既费轮次，又违反 §2 第 7 条（判断下沉机械层）。用户洞察：**误删只要没把系统干碎，往往下一轮就发现**
——说明需要的不是"回收站容量"，而是"**发现窗口**"。

**终裁设计**：

1. **保留策略 = 轮数窗口（默认 2 轮，最多扩至 3）**：回收先进入"待删集合"，经过窗口轮次仍无异常
   反馈才真删；**取消容量溢出、取消回收站 FIFO 挤出**。
2. **回收站整体取消**：不设容量口径（`BitBucket MaxCapacity` / `OPS_TRASH_MAX_BYTES` 仅作历史回查
   记录）、不设 `trash` 态、不设恢复入口、`unknown` 不走回收站。理由：**同卷回收站不释放空间**，
   且"延迟窗口"已覆盖回收站原本提供的可恢复价值（用户裁决：回收站面太窄，无需额外处理与保障）。
3. **`unknown` → 拒绝**（fail-closed）：不猜测、不入回收站；模型显式删除请求仍走既有工具面纪律
   （不在本子项扩面）。
4. **超预算 → 缩减或拒绝，不询问**：候选集超出回收预算时取预算内最小集或直接拒绝本次回收，把读数
   与原因作为信息返回；**不向模型发起"要不要删"的二次确认**。
5. **主撤销面 = git 兜底**（用户裁决）：三层对应——**tracked 内容**（源码/文档）由 git 兜底；
   **可再生内容**（`cache` 类）本就重建得出、无需兜底；**证据面**一律拒绝回收。工作区 git 仓库与
   框架既有 `snapshot_created`（本轮 Run A 实测 25 次）共同承担误删/误改的撤销。
6. **reclaim-direct 例外（生存优先，2026-09-12 用户裁决取值）**：**free < 5 GiB** 即跳过轮数窗口
   直接删 `cache` 类（仍受白名单/指纹/审计约束），证据面仍拒绝；free < 2 GiB 的 hard 档在其上
   叠加"树杀重活 + 收尾"。阶梯：watch 16 GiB → soft 8 GiB（拒新重活）→ **reclaim-direct 5 GiB**
   → hard 2 GiB / commit 95%。

**"超容量删除问题"是否一步到位（用户提问 2026-09-12）**：**是**。回收站取消后，"容量溢出"这个问题
本身不存在了；延迟删除只留下**一个**界——**单次回收预算**（同时承担防震荡与"待删集合体积过大"的
保护）：候选超预算即缩减/拒绝（见终裁第 4 条），不做任何溢出决策。

**成熟做法对照**：面向人类的桌面删除才用回收站（且不解决空间）；面向机器/CI 的回收用"可再生即删 +
缓存分类"（Bazel/journald 式）；编码 agent 的撤销面普遍是版本控制/检查点。本场景三者分别对应
`cache` / 证据面 / git 兜底，**回收站被完全替代**。

**代价登记**：轮数窗口使空间回收延迟 2 轮（watch/soft 档可接受；free < 5 GiB 起走终裁第 6 条
直删，hard 档再叠加树杀）；
窗口内被删内容仍占空间；`unknown` 类不可回收（fail-closed 的必然代价）。

### 4.7 缺口 F：资源硬上限（内核强制）替代动态并发限流

**裁决取向（用户 2026-09-12）**：用「**资源硬上限 + 派发前预检**」替代动态并发限流，
以确定性换取动态灵活性。

**机制（Windows，内核强制）**：每个 run 建一个 Job Object，对工具进程树施加硬上限；
上限由内核执行，**不依赖框架采样、不依赖模型配合**：

| 限项 | Windows 常量 | 作用 | 建议取值（待裁决） |
|---|---|---|---|
| commit 上限 | `JOB_OBJECT_LIMIT_JOB_MEMORY` | 该 job 可提交的虚拟内存上限（正是本轮被打穿的那条线） | `min(80% × 运行起点 commit limit, commit limit − 4 GiB)` |
| 并发进程数 | `JOB_OBJECT_LIMIT_ACTIVE_PROCESS` | **静态并发上限**（替代原"动态注入 `CARGO_BUILD_JOBS`"） | 由核数派生的静态值（如 `核数`，可配） |
| CPU 速率 | `JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP` | "达到本调度区间上限后，该 job 的线程在本区间内不再运行" | 80% |
| 每进程内存 | `JOB_OBJECT_LIMIT_PROCESS_MEMORY` | 防单进程爆量 | 默认关闭（易误伤 rustc），需要时再开 |

**因此**：原 §4.7 的"按档位注入环境变量改并发"**降级为可选柔性手段**（soft 档仍可注入
`CARGO_BUILD_JOBS` 作为额外缓冲，默认关），主机制改为上表硬上限。

**与预检的分工**：硬上限是"防爆底"（内核兜住，绝不越界）；§4.1 预检是"准入"
（不把注定超限的重活派出去，避免跑到一半被硬限打断的浪费）。二者叠加即用户提出的
"资源上限 + 派发前资源预检"。

**盘侧例外（必须说清）**：Job Object **管不了磁盘**。盘侧仍由 §4.1 预检 + §4.6 回收 +
保留量承担（≥ 20% 或 ≥ 8 GiB，取大者）。

**失败语义**：触限时子进程的分配/运行被内核拒绝 → 表现为 cargo/rustc 失败（FAILED 工具结果）。
框架必须把这类失败**识别并标注为"因资源上限失败"**（含命中的限项与读数），并保证 §4.3 的
链收尾完好——"工具失败"是可接受的，"机器死"不可接受。

**边界**：不接管非本框架进程；不做全局调度；不动态调参（采样只用于观测与回收触发）。

#### 4.7.1 S1.1 复核收口裁决（2026-09-12，用户授权工程裁决）

| # | 议题 | 裁决 | 依据 |
|---|---|---|---|
| 1 | 两级形态是否保留 | **保留并恢复**（顶层 run Job 持限项 + 调用级子 Job 持 `KILL_ON_JOB_CLOSE`，先根后子） | 独立复核 F-1 实测 + 微软 Nested Jobs 的层级构建顺序；S1 的降级依据（把 job 句柄传入进程参数位）不成立 |
| 2 | `ACTIVE_PROCESS` 取值 | 由"核数"改为 **`2 × 核数 + 8`（下限 16）** | 核数 = cargo 默认 `-j`，加上 cargo 自身、链接器与测试 harness，峰值必越界 → 内核拒绝 `CreateProcess` 把**合法重活**打成失败；上限的职责是兜住失控，不是调度（§11 裁决 4 该臂由此修订） |
| 3 | commit 上限口径 | 保留 `min(80% × limit, limit − 4 GiB)` 作为**上限帽**，实际取值改为 `min(帽, 装配期 commit 余量 − 1 GiB)`，下限 2 GiB | Run B 在名义 limit 的 **73%** 处死亡（`os error 1455` 页面文件太小，满盘使页面文件无法增长），且真机日常基线已被邻位占到 80% 上下（本轮检索轮实测 25.51 GB / 30.97 GB ≈ 82%）——上限必须表达"本 run 还能占多少"，而不是"宿主 limit 的百分比" |
| 4 | `run_tests` 是否入门 | **是**：host-owned 固定命令路径接同一预检门 + 改经 `ProcessGroup` 挂进 run Job | 独立复核 F-2：它是唯一"被判重档却门/上限两侧都不覆盖"的工具，评测 harness 经 `ORZ_TEST_RUNNER` 可达 |
| 5 | 目标卷判据 | **按动作的静态写入目标判定**（cwd + 重定向目标 + `--target-dir`/`--out-dir` + `cd`/`Set-Location`/`pushd` 操作数 + `CARGO_TARGET_DIR` 一类缓存变量），**所有卷全过才放行**，拒绝信封列出逐卷读数 | 独立复核 F-5：原实现只探会话 cwd，与 §4.1"目标卷"字面不符；路径不存在时上溯到最近存在祖先再探，避免"新目录 → 读数不可得 → 误拒" |
| 6 | 读数不可得时的档位 | 新增 **`unknown`** 档，**不得折叠成 `hard`** | 独立复核 F-6：S2 将以档位驱动回收/树杀，探针故障不得伪装成满盘 |
| 7 | 分档器对 here-string / heredoc | **剥体留头**（保留含真实重定向的首行；找不到终止符则原样保留，避免吞掉后续命令） | 独立复核 F-7：脚本体内的 `x = 1 > 0` 被读成文件重定向；方向虽 fail-closed，但压力下多发拒绝要付轮次 |

**盘—内存轴间耦合（必须登记，2026-09-12 独立复核 F-4）**：Run B 的两条记录放在一起看——
工具面先失败于 `页面文件太小 (os error 1455)`，随后进程 abort 于 `memory allocation of
200720 bytes failed`，而当天 Run A 已把 `D:` 写到 0 字节。**系统托管页面文件在满盘上无法增长**，
于是 commit 在名义 limit 之下即告失败（记录峰值 22.84 GB / 名义 30.97 GB ≈ 73%）。
含义：**盘轴与内存轴不独立**——§4.7 只登记了"Job 管不了磁盘"，反向耦合（盘满 → commit 失败）
同样成立。故 F 是次级防线，**盘侧（预检 + reserve + 回收阶梯）才是承重件**。

**回收阶梯的动作次序（S2 承重，2026-09-12 补登记）**：`reclaim-direct` 与 hard 档回收
**不得删除在跑重活的产物面**（例如正在构建的 `target/`）。次序必须是：先判在跑重活 →
在跑时只回收**不在用**的 cache 根，或先走 hard 档树杀再回收；`soft` 档的正常回收次序
沿用 §4.6（本 run scratch → 本工作区构建/包缓存 → 限窗更早 run 的 scratch）。

### 4.8 主代理技术裁决（2026-09-12，用户授权"这四项请你确定"）

| # | 议题 | 裁决 | 理由 |
|---|---|---|---|
| 1 | hard 档是否允许树杀在跑重活 | **允许**，限四条：①仅 hard 档触发；②**审计先行**（先落 `resource_exhausted(planned)`，含将杀的 `call_id` 集）；③**只杀重档**（动作分档判定为重者，轻活不杀）；④杀后必须产出可读失败（`resource_limit_hit` / `resource_exhausted`）并走 §4.3 收尾 | 机器死 = 全盘损失（含审计链），工具失败 = 可重跑；预检已挡住多数情形，该路径预期极少走到。**2026-09-13 用户裁决（S2R，裁决 15）**：③的落法 = **per-call-job 杀面（选项 a）**——宿主持在跑调用的 call job 复制句柄（SpawnObservation 下发），hard 档只 `TerminateJobObject` 重档调用的 call job（内核整树粒度 = 单调用树），run job 不整体终止，轻活/后台任务不在爆半径；爆半径由内核级测试钉死 |
| 2 | 孤儿扫除归属三条件 | **三条件保留 + 三条硬化**：只处理登记表内 pid；指纹 = 规范化 argv 序列 + 关键 env 哈希；审计先于动作（默认无延迟、保留可回退间隔） | 上一轮误杀外部构建的教训——宽口径识别、严口径执行 |
| 3 | Job 限项清单 | **启用** `KILL_ON_JOB_CLOSE`（§4.2）+ `JOB_OBJECT_LIMIT_JOB_MEMORY` + `JOB_OBJECT_LIMIT_ACTIVE_PROCESS` + `JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP`；**不启用** `JOB_OBJECT_LIMIT_PROCESS_MEMORY`（易误伤 rustc）与 working set 类限项（引发抖动且非 commit 级防爆） | 四条限项恰好覆盖本轮两次事故的两条线（commit/盘）+ 并发与 CPU；不启用项都是"会误伤正常重活"的 |
| 4 | soft 档柔性降级开关 | **默认关闭**（不注入 `CARGO_BUILD_JOBS` 等），保留为可配逃生阀 | Job 硬限已覆盖该功能；少一个"改写模型命令环境"的争议面与调试面 |

配套已裁决项：回收阶梯含 **reclaim-direct = free < 5 GiB 直删**（用户 2026-09-12）；重活放行阈值
8 GiB / commit 25%；三档 16 / 8 / 2 GiB（commit 70% / 85% / 95%）；硬上限 80% 类取值
（commit = `min(80% × 起点 limit, limit − 4 GiB)`、CPU 80%、并发进程数 = 核数）。

## 5. 事件面与合约变更

| 事件/字段 | 用途 | 落点 |
|---|---|---|
| `host_resource_snapshot` | 档位变化时的读数（含 source_quality） | 低频，跨档才落 |
| `host_resource_denied` | 预检/soft 拒绝（含读数与动作分档） | 走 `stamp_failure` 单漏斗，pre-issue 家族 |
| `resource_exhausted` | hard 档触发（含读数与被杀 call_id 集） | 终止前 |
| `run_terminated { reason }` | 显式终止形态 | 每个 run 必在 |
| `process_tree_reaped` | 孤儿回收审计 | sweep 后 |
| `reclaim_performed` | 回收审计（先于删除落盘；`outcome ∈ pending_delete / permanent / rejected`） | soft/hard 档回收时 |
| 工具结果字段 `output_encoding`（**既有**，`GAP-ENCODING-GATE`） | 编码命中可见 | read_file 结果信封 |
| `resource_limit_hit` | 命中 Job 硬上限（限项 + 读数 + call_id） | 超限失败时 |

合约纪律：新增事件族按既有「第 34 族」模式执法（Rust 法官 + Python 冻结镜像逐格零差），
fixture 正负例同批；`check_repository` 门禁与 manifest 重算同批。

## 6. 判据（可机械核对）

1. 重活余量不足时**机械拒绝**，拒绝事件含读数与动作分档（可逐条复核）。
2. 余量充足时**零误拒**：对照批（轻活 + 重活）全放行。
3. 读数不可得时按不足处理（fail-closed），并落 `source_quality=unavailable`。
4. 父进程 abort（模拟 alloc 失败）后 **60 s 内**子进程树无残留（对照实验，Windows Job）。
5. 归属三条件不满足的进程**不被杀**（负例：外部同形命令进程存活）。
6. ENOSPC 注入下 run 仍产出**可枚举终止形态**：链骨架连续（`sequence` 无洞、
   `previous_event_sha256` 不断）+ `run_terminated` 或 `TERMINAL.json` 二者必居其一。
7. degraded 卷被判为 `degraded_complete`，历史 622 份 journal 分类不变。
8. UTF-16LE/BE/GB18030 日志**读取成功且内容等价**；真二进制仍拒；大文件句柄契约不变。
9. 观测面读数与实测一致（同刻 `Get-PSDrive` / commit 读数对拍）。
10. 回归：orz-assurance / orz-loop / orz-host / orz-bin 测试全绿，fmt 干净，
    新增代码 clippy 零告警，`check_repository valid: true`。

11. **回收机制**：白名单外与归属不明的路径**零删除**（负例矩阵）；`reclaim_performed`
    先于删除落盘；**轮数窗口语义**（入待删集合后经过窗口轮次才真删，窗内可回退）；
    **超预算缩减/拒绝且不产生询问**；**无回收站依赖**（无 `trash` 态、无恢复入口）。
    **git 兜底边界**：tracked 内容窗内可回退、`cache` 类可再生、证据面拒绝回收。
12. **资源硬上限**：Job 生效后，工具进程树的 commit / 并发进程数 / CPU 速率**不突破配置值**
    （宿主侧读数对拍）；触限时产出 `resource_limit_hit` 且 run 仍以显式终止形态收尾。
13. **机械层优先**：资源读数与档位由机械层提供（session 面可读），**不要求模型自发检查**；
    对照 Run B 的"模型自发 df/内存检查"形态不再出现。

## 7. 测试与钉子

- **A**：命令分类器纯函数逐格测试（重/轻/未命中）；门判据边界（恰好等于阈值）；
  读数不可得路径；拒绝事件形状（单漏斗对拍）。
- **B**：Windows Job 回收端到端（真实子进程树 + 父 abort）；登记/扫除三条件
  正负例矩阵（含"外部同形进程不被杀"）。
- **C**：`JournalRecorder` ENOSPC 注入（受限卷/故障注入层）——退避、degraded、
  骨架连续、终止事件、`TERMINAL.json` 兜底；`journal-conformance` 新分类回归。
- **D**：UTF-16LE/BE BOM、无 BOM UTF-16、GB18030、UTF-8 四族读回等价；
  真 PNG/`.exe` 仍拒；`encoding_detected` 字段进机械审查汇聚行。

## 8. 实施分段（S1–S4）

| 段 | 内容 | 出口 |
|---|---|---|
| S1 | 代码面：§4.1（读数探针 + 分档 + 门 + 三档响应）、§4.4（编码门复用）、§4.7（Job 硬上限） | 落码 + 单测绿 |
| S2 | 合约面：§5 事件族/fixture/法官 + §4.3（ENOSPC 降级 + 终止形态）+ §4.2（Job + sweep）+ §4.6（回收机制） | 测试矩阵绿 + 门禁 `valid: true` |
| S3 | 载体重建：版本 bump **0.4.3 → 0.4.4** 双平台三件套 + staging/哈希/manifest | 双平台重建记录 |
| S4 | 实机复验：真机长任务复跑（含重活路径）+ 满盘注入 + abort 注入 + 编码样本 | S4 记录（判据 1–10） |

依赖：不依赖 0v-C 的修复（已 `ba934af8`），但 **S4 复跑同时承担 0v-C 实机复验**
（同一载体、同一批），把"0v-C 只经单测未实机复验"这一遗留项一并收口。

## 9. 风险与不做项

- 误判重档 → 多余读数与偶发拒绝：接受（fail-closed 优先），靠分类器白名单收敛。
- hard 档树杀会杀死模型正在跑的构建并产生 FAILED 工具结果：**这是设计意图**
  （宁可工具失败，不可机器死），但阈值需留足余量。
- 资源硬上限会让**原本可能跑完的重活中途失败**（80% 以内用不满）：接受（用户取向——以确定性
  换动态灵活性），代价是吞吐损失。**用户裁决理由（2026-09-12）**：叠加派发前预检后，最坏情形是
  模型偶尔判断失误 → 表现为**偶发的、可审计的上限事件**，而不是"概率必杀"；故该代价可接受。
- **本设计不依赖回收站**（用户终裁取消）：回查已证同卷回收站不释放空间，故回收站既不解决空间
  也不承担撤销；`unknown` 类改为 fail-closed 拒绝，撤销由 git / `snapshot_created` 承担。
- 轮数窗口使回收延迟 2 轮（hard 档例外为立即删）；窗口内被删内容仍占空间。
- 回收只在 soft/hard 档触发，仍存在"写满瞬间"窗口；证据面不自动回收（用户裁决），
  因此证据面膨胀仍需人工介入。
- **盘—内存轴间耦合**（2026-09-12 独立复核 F-4 登记）：盘满会使系统托管页面文件无法增长，
  commit 在名义 limit 之下即失败（Run B 实测 73%）。资源硬上限（§4.7）因此只是次级防线，
  盘侧 reserve 与回收阶梯承担主要保护；S1.1 已把 commit 上限改为"装配期余量 − 1 GiB"口径。
- **上限会打断合法重活**（S1.1 复核承认的代价）：`ACTIVE_PROCESS = 2 × 核数 + 8` 与
  "余量派生 commit 上限"都会在极端负载下让重活失败而不是让机器死；该取舍与 §9 前述
  用户取向一致（工具失败可重跑，机器死不可恢复）。
- 不做：全局调度器、跨 agent 协调、自动清理/自动改道、容器化前提、页面文件调整。

## 10. 与既有设计的关系

- **ADR-0010**：不新增语义，只在机械层补事实面与门控面（定稿时按惯例转录 §14.x）。
- **TER（AUTH-TOOL-EXECUTION-REFORM，§14.55）**：时间轴归 TER，空间轴归本设计；
  hard 档复用其 kill 面。
- **AUTH-GSA-SESSION-VOLUME（§14.56）**：`HostCapacitySnapshot` 与
  `SessionVolume*` 同族注入；`.gsa` 仍是状态链落盘面；reserve 与降级落在该面。
- **0q 统一失败事件管线（§14.63）**：`host_resource_denied` 走同一写入漏斗，
  不新增散点。
- **0p S2 B5 脱敏漏斗 / 0v-C**：同链路相邻面；本设计不改变 seal 语义。
- **机械审查层（§14.35）**：`encoding_detected` 与资源读数进其汇聚面，
  使"看得见的摩擦"与"看不见的摩擦"都可见。

## 11. 裁决记录（2026-09-12 封闭；开放项：无）

| # | 议题 | 裁决 |
|---|---|---|
| 1 | 阈值取值 | 重活放行 free ≥ 8 GiB 且 commit 余量 ≥ 25%；阶梯 16 / 8 / **5（reclaim-direct）** / 2 GiB（commit 70% / 85% / — / 95%） |
| 2 | 回收保留策略 | 轮数窗口默认 **2 轮**、最多扩至 **3**；超预算缩减/拒绝、**不询问**；**回收站整体取消**；`unknown` 拒绝 |
| 3 | reclaim-direct 触发 | **free < 5 GiB 即跳过窗口直删 `cache`**（用户 2026-09-12 定值） |
| 4 | 硬上限取值 | commit = `min(80% × 运行起点 commit limit, commit limit − 4 GiB)`；CPU 80%；并发进程数 = 核数。（**2026-09-12 裁决 11 修订**：commit 上限叠加"装配期余量 − 1 GiB"条款，并发进程数改 `2 × 核数 + 8`（下限 16）——见 §4.7.1） |
| 5 | Job 限项清单 | 启用 commit + 并发进程数 + CPU + `KILL_ON_JOB_CLOSE`；不启用每进程内存与 working set（§4.8 表 3） |
| 6 | hard 档树杀在跑重活 | **允许**（四条限制：仅 hard 档、审计先行、只杀重档、产出可读失败并收尾）——§4.8 表 1 |
| 7 | 孤儿扫除三条件 | 三条件保留 + 三条硬化（登记表内 / 规范化 argv+关键 env 指纹 / 审计先行）——§4.8 表 2 |
| 8 | soft 档柔性降级开关 | **默认关闭**，保留为可配逃生阀——§4.8 表 4 |
| 9 | 转码原始字节 | 只回传 `output_encoding` + `source_sha256`，**不额外落盘** |
| 10 | 与 0v-C 的关系 | **不合批**：0v-C 修复已落码（`ba934af8`）并**闭合**，0z 单独做；S4 不再搭车 0v-C |

| 11 | S1 独立复核的三项工程裁决 | **两级 Job 形态保留**（先根后子）/ **`ACTIVE_PROCESS = 2 × 核数 + 8`**（下限 16，修订裁决 4 该臂）/ **commit 上限改"装配期余量 − 1 GiB"口径**（上限帽不变，下限 2 GiB）——§4.7.1 |
| 12 | 其余复核项的处置 | `run_tests` 入门 + 挂 Job；目标卷按静态写入目标判定；新增 `unknown` 档；here-string/heredoc 剥体留头——§4.7.1 / §4.6 次序补注 |
| 13 | 盘—内存轴间耦合 | **登记**（盘满 → 页面文件不增长 → commit 提前失败）；F 为次级防线，盘侧为承重件——§4.7.1 / §9 |
| 14 | 回收阶梯与在跑重活 | 回收**不得删除在跑重活的产物面**；次序（在跑判定 → 不在用 cache / 先树杀后回收）由 S2 落码——§4.7.1 末段 |
| 15 | hard 档树杀爆半径（S2R，2026-09-13） | **per-call-job 杀面（选项 a）**：宿主持在跑调用 call job 复制句柄，hard 档只终止重档调用的 call job（单调用树粒度），run job 不整体终止；轻活/后台任务不在爆半径——§4.8 表 1 ③ |
| 16 | 盘满起新 run（S2R，2026-09-13） | **fail-closed 维持**：run_preflight 遇降级拒绝即拒绝 run 启动（避免写入面在满盘下扩张）——§4.3 第 8 条 |

**设计状态：完结（无开放裁决项；S1.1 复核收口已并入 §4.7.1）**；下一步为 **S2**
（事件族/fixture/法官 + C + B + E + §4.7.1 第 14 条的回收次序）。
