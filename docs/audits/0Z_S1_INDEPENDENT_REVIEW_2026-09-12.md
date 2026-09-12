# 0z S1 独立复核（设计合理性 / 实现合理性 / 设计与实现符合性）

> 日期：2026-09-12；批次：0z S1（A 派发前预检门 + D 编码门复用 + F 资源硬上限）。
> 复核对象：orz 子模块提交 **`73a8f25c`**（工作区 0 dirty）；设计权威
> [`HOST_RESOURCE_SAFETY_DESIGN_2026-09-12`](../HOST_RESOURCE_SAFETY_DESIGN_2026-09-12.md)；
> 实施记录 [`0Z_S1_IMPLEMENTATION_AUDIT_2026-09-12`](0Z_S1_IMPLEMENTATION_AUDIT_2026-09-12.md)。
> 复核方式：**只读**代码核验 + **独立自建探针**（Win32 作业对象真机实测）+
> **受控复跑**（release 复用既有产物，磁盘余量 < 10 GB 自动中止）+ 真实 run 语料回放。
> 边界：本记录不改实现、不改登记；结论分「符合 / 偏离 / 需处置」三类，处置归属见 §7。

## 1. 结论摘要

**主口径符合。** A（分档 + 读数 + 准入门 + 四档计算）、D（文本族 decode-first、
复用既有 `output_encoding`、真二进制仍拒）、F（四条限项 + 内核读回 + 非 Windows
诚实自报）三面逐条可达，设计条款对照见 §4。事故动作面覆盖经**真实语料回放**验证：
两个事故 run 的 439 条终端命令中 56 条被判重档，Run A 的全部 `cargo`/`rustc`
调用均在门内；两轮事故里 `run_tests` 不可用（工具面只有 5 件），故 S1 的门覆盖了
本轮实际致命动作。

**独立复跑与实施记录逐项吻合**（§3.2）：xai-tty-utils 25/0（含内核 commit 咬合）、
orz-host lib 308/0/5 单线程、orz-tools `util::encoding` 20/0 与 `read_file` 121/0、
`fmt` 唯一差异仍是未触碰文件、`check_repository` `valid: true / error_count: 0`。

**但有三项需要处置的发现（其中两项为「设计—实现不一致」，一项为取值风险）：**

| # | 等级 | 发现 | 归属 |
|---|---|---|---|
| **F-1** | **高** | S1 用「job 套 job 不可用」作为降级为一级形态的依据，**该实验方式无效**（把 job 句柄当进程句柄传给 `AssignProcessToJobObject`）。本机独立实测：**嵌套作业可用且真的咬合**。实现因此与设计 **§4.2 第 1 条 / §4.7 两级形态**不符，并把「跨调用汇总计量」这一设计目标整体推给 S2 | S2 起手前裁决 |
| **F-2** | **高** | `run_tests` 是唯一「被分档器判重档、却同时漏过 A 门与 F 上限」的工具（`OrzHost::run_tests` 是独立 spawn 路径，走另一套 job 实现）。评测/长任务一旦设 `ORZ_TEST_RUNNER`，重档测试面两侧皆无保护 | S2 内处理 |
| **F-3** | **中高** | `ACTIVE_PROCESS = 核数`（本机 12）对真实重构建偏紧：cargo 默认 `-j = 核数`，加上 cargo 自身、链接器与测试 harness，峰值进程数必然越过 12 → 内核拒绝 `CreateProcess` → **合法重活报失败**。该值取自用户裁决，故属设计风险而非实现缺陷 | S2/S4 前裁决 |

其余为中等与登记项（F-4…F-10，见 §5）：commit 上限高于本机观测到的真实死亡点、
且「盘满 → 页面文件无法增长 → commit 失败」这条**轴间耦合**在设计里未登记；
门只探测会话 `cwd` 所在卷（设计的「目标卷」未实现）；`tier_for` 把「读数不可得」
折叠成 `hard`（S2 若以档位驱动树杀会踩）；分档器对 here-string/heredoc 体误判
（fail-closed 方向，代价是压力下多发一次拒绝）；命名/计数/注释三处小偏差；
缺「工具调用 → 终端 spawn → 子进程确实在带限 job 内」的端到端测试；观测面尚未接线。

## 2. 复核方法与本机事实

### 2.1 三段式方法

1. **静态核验**：逐条读设计 §2/§3.4/§4.1/§4.2/§4.4/§4.7/§4.8/§5/§6/§8 与 S1 全部
   落码点（`resource_gate.rs` 1295 行、`resource_job.rs` 489 行、`read_file/mod.rs`
   与 `encoding.rs` 增量、两处生产装配点、`call_tool_inner` 汇点），核对计数、命名、
   注释与实现是否同口。
2. **独立探针**（不复用实现代码，自写 Win32 FFI）：实测本机作业对象语义与嵌套行为。
3. **受控复跑 + 语料回放**：release 复用既有产物复跑本批相关测试目标；把 10 个真实
   run 的 journal 里 458 条 `run_terminal_cmd` 命令喂给**从源码逐行切出的**分类器
   （切片保证与 `classify_command` 逐字节同源）回放。

### 2.2 本机现场读数（复核时刻，判据 1/9 的活体面）

| 读数 | 值 | 由门/阶梯导出的判定 |
|---|---|---|
| commit limit | 30,973,833,216 B（28.85 GB；早一次读数 31.79 GB，**limit 本身随页面文件伸缩**） | — |
| committed | 23,655,751,680 B（22.03 GB） | used = **76.37%** → 阶梯 **watch** |
| commit headroom | **23.63%** | 重活放行需 ≥ 25% → **此刻重活必被拒** |
| 物理可用 | 6.37 GB | — |
| `D:` free / used | 19.59 GB / 257.03 GB | ≥ 8 GiB → 卷轴不拦 |
| 逻辑核数 | 12 | F 的 `ACTIVE_PROCESS` 上限 = 12 |

两个结论直接来自这张表：①**判据 1 的机械面成立**（机械层读得到、算得出、判得了，
与实测 `Get-Counter` 对拍一致）；②**commit 轴是宿主机全局量**，此刻 22 GB 已提交
里含 Codex 本体与本次复核的编译——「邻居压上来即拒本 run 重活」是该设计的既有取向
（§2 第 2 条不接管宿主调度），但它在真机常态下的实际代价应由 S4 观测（见 F-4 建议）。

## 3. 独立证据

### 3.1 独立探针：嵌套作业可用且真的咬合（直接对应 F-1）

探针 A（作业语义）：自写 `rustc` 单文件，只用 `CreateJobObjectW` /
`AssignProcessToJobObject` / `OpenProcess` / `IsProcessInJob`，子进程用
`CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP [| CREATE_BREAKAWAY_FROM_JOB]` 拉起：

```
probe_process_in_job = true
spawn_plain = OK
spawn_breakaway = OK
create_job_a = OK
create_job_b = OK
assign_to_job_a = true
assign_to_job_b_nested = true
```

探针 B（限项是否在嵌套下咬合）：`JOBOBJECT_EXTENDED_LIMIT_INFORMATION`
（x64 实测 144 字节）设 `JOB_OBJECT_LIMIT_JOB_MEMORY = 300 MiB` +
`KILL_ON_JOB_CLOSE`，子进程 `$a = New-Object byte[] 1073741824`：

```
case C (control): no job at all
case C: refused_by_kernel=false   child: COMMITTED
case A: single ceiling-carrying job
   assigned = true
case A: refused_by_kernel=true    child: COMMITTED + New-Object … System.OutOfMemoryException
case B: plain job first, then the ceiling-carrying job (nested)
   assigned_plain = true assigned_nested = true
case B: refused_by_kernel=true    child: COMMITTED + New-Object … System.OutOfMemoryException
```

对照（case C）证明 1 GiB 提交在无上限时成功；case A/B 证明**上限在嵌套形态下同样
拒绝超额提交**。也就是说：设计 §4.7 的「run 级 Job 承载上限 + 每次调用子 Job」在本机
**可实现**，S1 认领的「跨调用汇总计量」残余不是平台限制，只是形态选择。

（附注：S1 实施记录援引的 `AssignProcessToJobObject(job, job) → ERROR_INVALID_HANDLE`
是把 **job 句柄**放进了 process 参数位——该调用与嵌套能力无关；Windows 的嵌套是
「把一个**已在 job 内的进程**再指派给第二个 job」，探针 B 的 case B 正是这条路径。）

### 3.2 受控复跑（release，磁盘余量 < 10 GB 自动中止；未触发）

| 命令 | 结果 | 磁盘增量 |
|---|---|---|
| `cargo test --release -p xai-tty-utils --all-targets` | **25 passed / 0 failed**（含 `commit_ceiling_actually_stops_an_over_committing_child`） | −0.13 GB |
| `cargo test --release -p orz-host --lib -- --test-threads=1` | **308 passed / 0 failed / 5 ignored**（含 `resource_gate::tests::*` 15 条 + 汇点 2 条 + 装配面 1 条，逐条见日志） | +0.57 GB |
| `cargo test --release -p orz-tools --lib util::encoding` | **20 passed / 0 failed** | −1.10 GB |
| `cargo test --release -p orz-tools --lib implementations::grok_build::read_file` | **121 passed / 0 failed** | 0.00 GB |
| `cargo fmt --all -- --check` | 唯一差异 `crates/orz-bin/tests/acaf_e2e.rs:324`（**本批未触碰**，与实施记录一致） | — |
| `python scripts/check_repository.py` | **`valid: true` / `error_count: 0`**（247 schemas 等） | — |

未复跑项与理由：`orz-bin --bins` 41/0 与 clippy（需分离 fingerprint 的整包重编，
在 D: 余量约 20 GB 的现状下风险高于收益）；`generate_orz_source_manifest.py`
（写操作，属登记动作，不应由只读复核触发）。二者均以「未复核」如实标注，不计入结论。

### 3.3 真实语料回放（判据 1/2 的事故面覆盖）

分类器切片回放 10 个真实 run 的 458 条 `run_terminal_cmd`：

| run | 命令数 | 判重档 | 判轻档 | 含 `cargo` 字样 |
|---|---|---|---|---|
| `RUN-CLI-6aa4f384`（Run A，盘满致命） | 379 | **47** | 332 | 54 |
| `RUN-CLI-6aa50fdf`（Run B，commit 耗尽 abort） | 60 | **9** | 51 | 12 |
| `6aa4f2cb` / `6aa51e61` | 18 / 1 | 0 / 0 | 18 / 1 | 0 / 0 |
| 其余 6 个 run | 0 | 0 | 0 | 0 |

- **无漏判**：Run A 的全部 `cargo`/`rustc` 调用（`cargo check/test/build/run/clippy/fmt`、
  `rustc --version`、路径形态 `…\.cargo\bin\cargo.exe`）均判重档；wrapper 穿透
  （`cmd /c "cd /d … && cargo test"`、`powershell -Command "cargo test …"`）亦命中。
- **轻档零误伤**：`grep -n "cargo" …`、`echo "--- Cargo member ---"` 等含重程序
  **字面量**但非调用的命令全部保持轻档（回放实测 2 例，均正确排除）。
- **误判方向安全**：56 条重档里 40 条由重程序表命中，16 条由「输出重定向到真实文件」
  命中；其中一部分是 PowerShell here-string / bash heredoc **体内**的 `>` 被当成
  重定向（见 F-7）。方向是 fail-closed（多发一次拒绝），不产生漏判。

## 4. 设计与实现符合性矩阵

| # | 设计条款 | 实现落点 | 判定 |
|---|---|---|---|
| 1 | §4.1 接缝：`call_tool` 汇点、任何进程启动之前 | `OrzHost::call_tool_inner`（权限门 / web_search semaphore 同一处），早退分支（`project_doc_index`/browser/pdf）在其前 | **符合** |
| 2 | §4.1 读数面：`HostCapacitySnapshot` + 探针 + `source_quality` | `SystemCapacityProbe`（`GetDiskFreeSpaceExW` + `GlobalMemoryStatusEx`；Linux `statvfs` + `/proc/meminfo`）；部分读数即整体 `unavailable` | **符合** |
| 3 | §4.1 动作分档：工具名优先、静态命令分类器、未命中默认轻 | `classify_action` / `classify_command`（重程序表 100+、wrapper 穿透、管道分段、重定向识别、条件程序） | **符合**（回放见 §3.3） |
| 4 | §4.1 门判据：`free ≥ 8 GiB ∧ commit 余量 ≥ 25%` | `ResourceGate::evaluate`（边界含等号；不符即 `resource_insufficient` + 读数） | **符合**；设计公式中「`3 × 预计写入`」未实现（恒退化为 8 GiB，实施记录 §3.1 已按简化口径登记） |
| 5 | §4.1 阶梯：16 / 8 / 5 / 2 GiB（commit 70/85/95%） | `tier_for` + 六个常量；S1 **只算档 + 报档** | **符合**（动作面属 S2） |
| 6 | §4.1 三级响应动作（watch 只记录 / soft 拒新重活 / reclaim-direct 直删 / hard 树杀） | S1 实现「soft 语义」的拒新重活；reclaim 与树杀未落 | **批次边界**（§8 分段表 S1 行不含），但 S2 起手须补（见 §7 第 3 条） |
| 7 | §2 第 4 条 fail-closed | 读数不可得 → 重活拒、轻活照过（两条独立测试） | **符合** |
| 8 | §4.4 编码链（UTF-8 BOM → UTF-16LE/BE BOM → 无 BOM 奇偶启发式 → GB18030） | `sniff_text_bytes` 前三段入门前移；GB18030 **留在历史路径** | **有据偏离**（前移会把「文本扩展名 + 真二进制」从拒读变乱码，单测 `sniff_rejects_binary_blobs` / `sniff_declines_gb18030_to_the_historical_path` 钉住）；**但 `read_file/mod.rs` 注释仍把 GB18030 列进门链（F-8）** |
| 9 | §4.4 + §11 裁决 9：只回传 `output_encoding` + `source_sha256`、不额外落盘 | 复用既有 `output_encoding`（新增 `utf-16le`/`utf-16be` 标签）；原始字节摘要由**既有读锚**承载（`read_anchor.sha256`，UTF-16 读回测试断言 `anchor.size` 为原始字节数） | **符合** |
| 10 | §4.4「真二进制仍拒、文案不变」「非文本族顺序不变」 | `decode-first` 仅对文本族；`is_binary` 后置；端到端测试 `read_file_text_family_binary_content_still_rejected` | **符合** |
| 11 | §4.7 + §4.8 裁决 3：限项集合 | `JOB_OBJECT_LIMIT_JOB_MEMORY` + `ACTIVE_PROCESS` + `CPU_RATE_CONTROL_HARD_CAP` + `KILL_ON_JOB_CLOSE`；**不启用** 每进程内存 / working set | **符合** |
| 12 | §4.7 + §11 裁决 4：取值（commit `min(80%×limit, limit−4 GiB)`、CPU 80%、并发 = 核数） | `default_job_limits` 逐式落地；内核读回为权威（页取整、reserve 压零视为不设上限） | **符合公式**；取值风险见 F-3 / F-4 |
| 13 | §4.2 第 1 条：run 级顶层 Job（跨调用汇总计量）+ 每次调用子 Job（`KILL_ON_JOB_CLOSE`） | 只做「run 级一次决策 + **每次调用** job 承载全部限项」 | **不符合**（F-1；实施记录 §3.3 的降级依据经独立实测不成立） |
| 14 | 非 Windows 不假装强制 | `RunResourceJob` 记意图、自报 `kernel_enforced=false` | **符合** |
| 15 | §6 判据 1/2/3 的 S1 半段、判据 12 的 S1 半段（内核读回 + 真实咬合） | 汇点拒绝含读数与分档；轻活零误拒样例；fail-closed 两条；`commit_ceiling_actually_stops_an_over_committing_child` | **符合** |
| 16 | §5 事件面（`host_resource_denied` 等 5 族 + fixture + 法官） | 未落（S1 不新增事件；拒绝仅以 `ToolResult` + `structured` 回传） | **批次边界**（判据 1 的「事件含读数」需 S2 才完全成立） |
| 17 | §8 S1 出口：落码 + 单测绿 | 复跑见 §3.2 | **符合** |
| 18 | 门禁与 manifest 收口 | `check_repository valid: true`（复跑）；manifest 未由本次复核重算 | **符合**（复核不触发写操作） |

## 5. 发现

### F-1（高）F 的两级形态裁决依据不成立，且与设计 §4.2 明条冲突

- **事实**：设计 §4.2 第 1 条明确「每个 run 一个顶层 Job 承载 §4.7 的资源上限（跨工具
  调用汇总计量）；每次工具调用一个子 Job 承载 `KILL_ON_JOB_CLOSE`」。S1 实现只保留
  「每次调用 job 承载全部限项」，理由是「job 套 job 不可用」。
- **证据**：该结论来自 `AssignProcessToJobObject(job, job)`（§3.1 附注：参数位错误，
  与嵌套无关）；本机独立探针显示 `assign_to_job_b_nested = true`，且嵌套下的 commit
  上限**确实拒绝超额提交**（case B）。另：「必须把 `orz.exe` 自己放进 job」并非该形态
  的前提——S1 已经在为每个子进程建 job，同样可以在建子 job 前把该子进程先指派进
  run 级 job。
- **影响**：①「跨调用汇总计量」这一设计目标当前完全缺失（N 个并发调用各持一份
  全量上限，合计可超宿主）；②「两级形态不可实现」作为技术事实写入提交信息与审计，
  会污染后续判断（S2 可能据此不再尝试）。
- **建议**：S2 起手前裁决——恢复两级形态（子进程先入 run job、再入 per-call job，
  两处均读回），或在设计里显式撤回 §4.2 第 1 条并改登记「并发聚合上限不做」。
  无论选哪条，都应把本次探针结论回填实施记录（更正 §3.3）。

### F-2（高）`run_tests` 同时漏过 A 门与 F 上限

- **事实**：`HEAVY_TOOLS = ["run_tests"]`（分档器承认它重），但 `OrzHost::run_tests`
  是独立方法（`orz-host/src/lib.rs:1203`），直接 `tokio::process::Command` 拉起，
  并用 `orz_assurance::sandbox::job_object::JobObjectSupervisor` 建**另一套** job
  （只有 `KILL_ON_JOB_CLOSE`，**不带** §4.7 的三条限项），既不过 `call_tool_inner`
  汇点，也不走 `ProcessGroup::new()` 的注入面。
- **现实可达性**：`orz-bin` 的 `-p` 路径读 `ORZ_TEST_RUNNER`（`main.rs:1298`）后
  `.with_test_runner(...)`，评测/长任务 harness 正是这条（D-9「harness feedback loop」）。
  本次两个事故 run 未开启（`tool_availability_check` 只有 `read_file`/`grep`/
  `search_replace`/`blackboard_read`/`run_terminal_cmd` 五件），因此**未暴露**——
  即：这是「下一次换 harness 就会踩」的洞。
- **建议**：S2 内处理——`run_tests` 走同一门（拒绝形态接 `TestRunResult`），
  spawn 改经 `ProcessGroup::new_with_limits`；并把「工具面枚举与门/上限覆盖的一一对应」
  做成机械护栏（新增重档工具时两侧必须同批改）。

### F-3（中高）`ACTIVE_PROCESS = 核数` 对真实重构建偏紧

- **事实**：本机逻辑核数 12 → job 活跃进程上限 12。cargo 默认 `-j = 核数`，同一时刻
  在跑的有 cargo 自身 + 最多 12 个 `rustc`；每个 `rustc` 还会拉起链接器，`cargo test`
  另会拉起测试二进制。峰值进程数**必然**越过 12，越过即 `CreateProcess` 被 job 拒绝。
- **性质**：取值来自用户裁决（§11 裁决 4「并发进程数 = 核数」），实现与设计一致；
  风险在**设计取值**：把「防失控上限」取成了「等于正常并发度」，会把合法重活打成失败。
- **建议**：S2/S4 前裁决为「2×核数」或「核数 + 8」一类留余量取值（上限的职责是兜住
  失控，不是调度）；并让 S2 的 `resource_limit_hit` 识别这条失败形态（子进程创建被拒
  的报错文案不自明，模型会把内核拒绝读成代码错误）。

### F-4（中）commit 上限高于本机真实死亡点；「盘满 → commit 失败」耦合未登记

- **事实**：Run B 的 journal 里**最后一条工具结果**是
  `run_terminal_cmd` → `Terminal error: IO Error: 页面文件太小，无法完成操作。 (os error 1455)`
  （`RUN-CLI-6aa50fdf` seq 788，`wall_ms=2901`），随后进程 abort
  （`memory allocation of 200720 bytes failed`，console 日志）。事发日 Run A 已把
  `D:` 写到 0 字节；**系统托管页面文件无法在满盘上增长**，于是 commit 在名义 limit
  之下就失败（记录峰值 22.84 GB / 名义 30.97 GB ≈ 73%）。
- **含义**：①`min(80%×limit, limit−4 GiB)` 在本机给出约 25.4 GB，**高于**观测到的
  失败点，单个失控工具树仍可能把宿主推到失败区；②两条事故并非完全独立——**盘轴**
  是共同驱动，内存死亡是盘满的下游。设计 §4.7 只登记了「Job 管不了磁盘」，**反向
  耦合（盘满 → commit 失败）没有登记**。
- **建议**：在设计 §4.7/§9 补登记该耦合；把盘轴（S2 的 reserve + 回收阶梯）明确为
  承重件，F 视为次级防线；考虑更严的 commit 取值，或让准入门改看「物理可用 +
  页面文件可增长余量」而非名义 limit。

### F-5（中）门只探测会话 `cwd` 的卷，设计的「目标卷」未实现

- **事实**：`Call_tool_inner` 传的是 `&self.cwd`；命令自身的 `cd X:`、`--target-dir Y:`、
  `-C` 全不参与判据。真实语料里已出现 `--target-dir D:\CLI\.gsa\cargo-target`
  与 `cd D:\CLI\orz; …` 形态（本机同卷，故无损）。
- **影响**：单卷工作区无影响；跨卷（模型 `cd C:\…` 后构建）时会以错卷读数放行，
  与 §2「重活在动作发起卷原地完成」的观测意图不符。
- **建议**：S2 增补目标卷提取（`cd` / `Set-Location` / `--target-dir` / `-C`），
  或在设计与审计里显式登记「只按会话卷判定」。

### F-6（低—中）`tier_for` 把「读数不可得」折叠成 `hard`

- **事实**：`HostCapacitySnapshot::unavailable()` 的 `volume_free_bytes = 0` →
  `tier_for` 返回 `Hard`。轻活路径会把 `tier=hard` 放进 Allow 信封；`evaluate` 的
  拒绝路径虽用 `tier: None` 规避了这一点，但**值本身仍具误导性**。
- **风险**：S2 若以 `tier_for` 驱动「hard 档树杀 / reclaim-direct 直删」，探针故障
  会被当成 hard 档。
- **建议**：S2 前改为 `Option<ResourceTier>` 或新增 `Unknown` 变体，机械上与
  `source_quality` 绑定。

### F-7（低）分档器对 here-string / heredoc 体内的 shell 元字符无感知

- **事实**：PowerShell `@'…'@` 与 bash `<<EOF` 体不进 `tokenize` 的引号态，
  体内的 `>`（如 Python 的 `x = 1 > 0`）被当成文件重定向 → 判重档。回放中
  56 条重档里有 16 条无重程序命中，其中一部分即此形态；对照组
  `python -c "print(1 > 0)"` 正确判轻（体在引号内）。
- **性质**：方向 fail-closed（多拒不漏），代价是 soft 档下多发一次拒绝、多耗一轮。
- **建议**：登记为已知精度残余；S2 若要收敛，只需在分段前剥离 here-string /
  heredoc 体（不必做完整 shell 解析）。

### F-8（低）命名、计数、注释三处与实现不同口

1. `ResourceTier::as_str()` 产出 `reclaim_direct`，设计与记录写 `reclaim-direct`
   → S2 冻结事件/fixture 词汇表前应统一（否则法官镜像会撞字符串）。
2. 计数偏差：实施记录 §2 表写 `resource_gate.rs`「17 条单测」，实为 **15**；
   `resource_job.rs` 写「6 条」，实为 **7**（含 1 条 `not(windows)` 条件编译，
   Windows 上跑 6 条）；`encoding.rs` 写「10 条新单测」，实为 **11**。
   （§5 证据表的 15 / 20 / 121 与实测一致。）
3. `read_file/mod.rs` 的门注释仍把 GB18030 列为链上一步，与实现收窄相反
   （`encoding.rs` 的 `sniff_text_bytes` 文档是对的）。

### F-9（低—中）缺「工具调用 → 终端 spawn → 子进程确实在带限 job 内」的端到端测试

- **事实**：`resource_job.rs` 的单测**手工** attach 一个自己 spawn 的进程；
  `orz-host` 的装配测试只断言 run job 的内核读回。真正决定事故面的是
  `terminal.rs` 的 spawn 路径（`CREATE_BREAKAWAY_FROM_JOB` → 失败则回退无 breakaway
  → `attach` 失败**只记 debug 日志**），这条链没有任何断言覆盖。
- **本机实测**：`spawn_breakaway = OK`（当前启动上下文里 breakaway 可用，故两条
  spawn 路径都不会静默降级）；但 ACP 宿主/服务形态下 `ERROR_ACCESS_DENIED` 回退是
  代码承认存在的路径，一旦走它，F 的强制与树回收**同时静默失效**且无机械可见面。
- **建议**：S2/B 加端到端测试（cargo 或 python 子进程 + job 读回 + 真实咬合），
  并把 attach/breakaway 失败提升为计数或事件（不做事件也应至少 WARN）。

### F-10（登记）观测面尚未接线

`ResourceGate::last_snapshot()` / `resource_gate()` 目前只有测试调用；§4.5 的
session 面读数（余量 / 档位 / 在跑重活计数）与 §5 的 `host_resource_snapshot`
均未落 —— 属 S2 范围，此处只登记「判据 13 在 S1 态不成立」。

## 6. 复核边界

- 只读复核：未改任何实现、登记或索引；未执行 manifest 重算等写操作。
- 未复跑：`orz-bin --bins`（41/0）、clippy（本批文件零告警）、0v/其他批次回归；
  这些结论本次**未独立验证**，沿用实施记录。
- 复跑形态：全部为 `--release`（复用既有产物以控磁盘）；`--release` 与 `--debug`
  在本批代码路径上无 cfg 差异，但 debug-only 断言未覆盖。
- 探针为一次性自建程序（复核后清理），原始输出见 §3.1；分类器回放用的是**从源码
  逐行切出**的同一函数体，未重写、未改写。

## 7. 建议处置（S2 起手清单增补）

1. **先裁决 F-1 与 F-3**（形态与取值），再动 S2 的 B 子项——这两条决定 Job 层怎么搭。
2. **补 F-2**：`run_tests` 接门 + 接 ceilings，并加「重档工具 ↔ 门/上限」机械护栏。
3. **把 S2 的回收阶梯写成动作顺序**（依 F-4/F-5）：`reclaim-direct` 与 hard 档必须有
   明确次序，避免删到**在跑重活的 target 目录**（设计 §4.6 只写「本工作区构建缓存」
   在候选序列里，未排除在跑动作的产物面）；同时补齐 reserve/树杀/回收与盘轴的耦合登记。
4. **F-6 前置**：`ResourceTier` 增 `Unknown`/改 `Option`，别让 S2 的树杀挂在
   「探针故障 = hard」上。
5. **F-9 补测**：端到端 spawn → job 读回 → 咬合；attach/breakaway 失败升可见度。
6. **F-8 收口**：`reclaim_direct` vs `reclaim-direct`、三处计数、`read_file` 注释。
7. **S4 必测**：F-3 的进程数上限（真实 workspace 构建）与 F-4 的盘满注入——两者都在
   本轮事故的同一动作面上。

## 8. 处置回执（2026-09-12，S1.1 批次；用户授权工程裁决后逐项落地）

| 发现 | 裁决 | 落点 | 验证 |
|---|---|---|---|
| **F-1** 两级形态 | **保留设计原形并恢复**：run 级 `RunResourceJob` 持限项（内核句柄活到进程生命周期）+ 调用级 `ProcessGroup` 持 `KILL_ON_JOB_CLOSE`；`attach_pid` **先根后子**两次指派 | `xai-tty-utils/src/resource_job.rs`（`RunResourceJob` 真持 job + `assign_process_handle` / `contains_process`）、`ProcessGroup::new_with_limits` / `attach_pid` / `readback_limits` / `readback_call_job_limits`；设计 §4.2 第 1 条补注 + §4.7.1 裁决 1 | `per_call_group_nests_under_the_run_job_and_reports_its_ceilings`、`run_job_ceiling_bounds_a_child_of_a_per_call_group`、orz-host `tool_spawn_is_really_bounded_by_the_run_job`（全部绿） |
| **F-2** `run_tests` 双漏 | **入门 + 挂 Job**：同一分档器/探针/拒绝口径；spawn 改经 `ProcessGroup`（run 级限项 + 调用级树杀），超时先 job 杀再 TaskKill | `orz-host/src/lib.rs::run_tests` | `run_tests_is_gated_like_any_heavy_action`（拒绝且"Nothing was started"） |
| **F-3** 进程数上限 | 由"核数"改 **`2 × 核数 + 8`（下限 16）** | `resource_gate::run_active_process_limit` + `default_job_limits`；设计 §11 裁决 4 该臂修订（§4.7.1 裁决 2） | `job_limits_use_the_eighty_percent_or_reserve_rule`（含 `> 核数` 断言） |
| **F-4** commit 上限与轴间耦合 | 取值改 `min(上限帽, 装配期 commit 余量 − 1 GiB)`、下限 2 GiB；**耦合登记** | `default_job_limits`；设计 §4.7.1 / §9 | 同一条单测（余量条款三例：忙碌 21 GiB / 空闲 80 GiB / 濒临耗尽落 2 GiB 地板） |
| **F-5** 目标卷 | 静态写入目标提取（cwd + 重定向 + `--target-dir`/`--out-dir` + `cd`/`Set-Location`/`pushd` + `CARGO_TARGET_DIR` 类），**全卷全过**；探针按最近存在祖先取卷 | `resource_gate::write_targets` / `evaluate_for_volumes` / `probe_volume_nearest`；拒绝信封新增 `write_targets[]`；设计 §4.7.1 裁决 5 | `write_targets_follow_the_real_corpus_shapes`（含 Run A 真实形态）、`write_targets_ignore_reads_and_null_redirection`、`write_targets_capture_redirection_and_build_cache_env`、`any_short_write_target_volume_refuses_the_heavy_action` |
| **F-6** `hard` 假档 | 新增 `ResourceTier::Unknown`；`GateDecision::Refuse.tier` 由 `Option` 改实值 | `resource_gate::tier_for` / `GateDecision`；设计 §4.1 机器键说明 + §4.7.1 裁决 6 | `tiers_follow_the_ladder`（unavailable → unknown）、`unavailable_readings_fail_closed_for_heavy_only` |
| **F-7** here-string / heredoc 误判 | **剥体留头**（保留首行；找不到终止符则原样保留，避免吞掉后续命令） | `resource_gate::strip_payload_bodies` + `payload_start` / `heredoc_delimiter` / `find_terminator`；设计 §4.7.1 裁决 7 | `here_string_bodies_are_not_command_syntax`、`heredoc_bodies_are_not_command_syntax`、`unterminated_payload_markers_do_not_swallow_commands` |
| **F-8** 命名/计数/注释 | ①机器键统一 snake_case（`reclaim_direct` 等），设计 §4.1 显式登记；②S1 实施审计三处计数更正；③`read_file` 门注释改为收窄后的真实链 | 设计 §4.1；`0Z_S1_IMPLEMENTATION_AUDIT` 表内更正 + §3.3 更正块；`read_file/mod.rs` 注释 | 文本级核对 + `cargo fmt --check` |
| **F-9** 端到端与可见度 | ①新增真机端到端（轻档调用 → 终端 spawn → 子进程被 run 级 commit 上限拒绝）；②attach 失败**计数化**（`record_attach_failure` / `attach_failure_count`）+ 终端路径日志由 debug 升 warn | `xai-tty-utils`（计数器 + attach 路径）、`orz-tools/computer/local/terminal.rs`、orz-host 端到端测试 | `tool_spawn_is_really_bounded_by_the_run_job`（绿） |
| **F-10** 观测面未接线 | 维持 **S2** 归属（事件族与 `blackboard_read section=session` 读数同批）；本批只把 run 句柄与读回值开放给该面 | 设计 §5 / BACKLOG 0z S2 行 | — |
| 回收次序（§5 F-4/F-5 外溢项） | **登记为 S2 承重**：回收不得删在跑重活的产物面；在跑判定 → 不在用 cache / 先树杀后回收 | 设计 §4.7.1 末段 + §11 裁决 14 | — |

**一手来源（本次裁决直接引用）**：

1. Job Objects（learn.microsoft.com/windows/win32/procthread/job-objects）：
   "A process can be associated with more than one job in a hierarchy of nested jobs"；
   "The ability to nest jobs was added in Windows 8 and Windows Server 2012"；
   "If the job object is nested, accounting information for each child job is aggregated in its parent job"。
2. Nested Jobs（…/procthread/nested-jobs）："To ensure that the job hierarchy is valid, first assign
   all processes to the job at the root of the hierarchy, then assign a subset of processes to the
   immediate child job object, and so on"。
3. `JOBOBJECT_BASIC_LIMIT_INFORMATION`（…/api/winnt/ns-winnt-jobobject_basic_limit_information）：
   `JOB_OBJECT_LIMIT_JOB_MEMORY` — "Causes all processes associated with the job to limit the
   job-wide sum of their committed memory"；`JOB_OBJECT_LIMIT_ACTIVE_PROCESS` — "Establishes a
   maximum number of simultaneously active processes associated with the job"。
