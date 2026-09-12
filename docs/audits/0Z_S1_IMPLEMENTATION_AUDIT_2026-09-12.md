# 0z S1 实施审计（真机资源安全边界——A 派发前预检门 + D 编码门复用 + F 资源硬上限）

> 日期：2026-09-12；批次：0z S1（代码面）；设计权威：
> [`HOST_RESOURCE_SAFETY_DESIGN_2026-09-12`](../HOST_RESOURCE_SAFETY_DESIGN_2026-09-12.md)
> §4.1 / §4.4 / §4.7、§8 分段表 S1 行。
> 出口：**落码 + 单测绿**（S1 出口定义）；事件族/fixture/法官与门禁 `valid: true` 属 S2。

## 1. 范围

**本批落地（S1）**

| 子项 | 内容 | 落点 |
|---|---|---|
| **A** | 派发前资源预检门：读数探针 + 静态动作分档 + 准入门 + 阶梯档位（watch / soft / reclaim-direct / hard）计算 | `orz-host/src/resource_gate.rs`（新增）+ `OrzHost::call_tool_inner` 汇点 |
| **D** | 编码门复用：`read_file` 对文本族扩展名走 decode-first（UTF-16 BOM / 无 BOM 奇偶启发式 / 无 NUL 的 UTF-8），命中后复用既有 `output_encoding` 字段 | `orz-tools/src/util/encoding.rs` + `implementations/grok_build/read_file/mod.rs` |
| **F** | 资源硬上限：run 级 ceilings 决策 + 每次工具调用 Job 承载内核硬限（commit / 并发进程数 / CPU 速率 + `KILL_ON_JOB_CLOSE`），装配期安装 | `xai-tty-utils/src/resource_job.rs`（新增）+ `ProcessGroup` + 生产装配两处 |

**本批不做（按设计归 S2）**：事件族（`host_resource_snapshot` / `host_resource_denied` /
`resource_exhausted` / `reclaim_performed` / `resource_limit_hit`）、Rust 法官 + Python 冻结镜像 +
fixture、`stamp_failure` 单漏斗接线、C（ENOSPC 降级 / 终止形态）、B（`.gsa/process_trees/` 登记 +
孤儿扫除）、E（回收机制 + 轮数窗口）、`manifest` 重算与门禁 `valid: true`、orz 子模块提交。

## 2. 变更清单（orz 子模块 `73a8f25c`）

| 文件 | 增量 | 内容 |
|---|---|---|
| `crates/orz-host/src/resource_gate.rs` | 新增 | 动作分档器（白名单轻 / 黑名单重 + wrapper 穿透 + 管道分段 + 文件重定向识别）、`HostCapacitySnapshot` 读数结构、`CapacityProbe` 探针接缝（Windows `GetDiskFreeSpaceExW` + `GlobalMemoryStatusEx`；Linux `statvfs` + `/proc/meminfo`）、阶梯档位、`ResourceGate::evaluate`、`default_job_limits`；15 条单测（**计数更正 2026-09-12**：原记 17 条） |
| `crates/orz-host/src/lib.rs` | +226 | `OrzHost` 新增 `resource_gate` 字段与 `with_host_resource_safety`（装配期：安装 run ceilings + 注入真实探针）、`with_resource_gate`（接缝）、`call_tool_inner` 汇点拒绝；3 条测试（汇点拒绝/放行、读数不可得 fail-closed、生产装配面真机读数 + 内核限项） |
| `crates/orz-host/Cargo.toml` | +14/−2 | 依赖 `xai-tty-utils`；Windows features `Win32_Storage_FileSystem` + `Win32_System_SystemInformation`；unix `libc` |
| `crates/orz-host/src/acp_server.rs` | +3 | ACP/stdio 生产装配点接线 |
| `crates/orz-bin/src/main.rs` | +4 | `-p` 生产装配点接线 |
| `crates/codegen/xai-tty-utils/src/resource_job.rs` | 新增 | `JobLimits` / `JobReadback` / `RunResourceJob`（内核读回）+ `global_run_job` / `install_global_run_job` + Windows Job 限项实现；7 条单测（其中 `non_windows_*` 在 Windows 上不编译 → Windows 实跑 6 条，含 commit 上限真实咬合测试；**计数更正 2026-09-12**：原记 6 条） |
| `crates/codegen/xai-tty-utils/src/lib.rs` | +69/−28 | `ProcessGroup::new_with_limits`（承载 run ceilings）、`readback_limits`（内核读回） |
| `crates/codegen/orz-tools/src/util/encoding.rs` | +303 | `TEXT_FAMILY_EXTENSIONS` / `is_text_family_extension` / `sniff_text_bytes`（decode-first 门）+ UTF-16 严格解码 + 无 BOM 奇偶启发式；11 条新单测（**计数更正 2026-09-12**：原记 10 条；模块总数 20 与 §5 一致） |
| `crates/codegen/orz-tools/src/implementations/grok_build/read_file/mod.rs` | +121 | decode-first 门接线（文本族先解码、失败才判二进制；非文本族顺序不变）；4 条新单测 |
| `crates/orz-loop/src/host.rs` | +4/−2 | `ToolResult.output_encoding` 文档补 `utf-16le` / `utf-16be`（字段与语义不变） |
| `Cargo.lock` | +2 | 新增依赖边 `orz-host → xai-tty-utils` |
| 父仓库 `runtime/tool-completed-event-payload-v0.{1,2}.schema.json` | 描述行 | `output_encoding` 描述补两个新标签（**枚举未变**：该字段是 `["string","null"]`，仅描述列标签） |

合计：9 个已跟踪文件 + 2 个新文件，`+718/−28`（不含父仓库两行 schema 描述）。

## 3. 实现口径（含与设计的三处偏离，均为实施中发现）

### 3.1 A：门判据与阶梯

- **准入（重活）**：`目标卷 free ≥ 8 GiB` **且** `commit 余量 ≥ 25% × commit limit`；两条同时
  满足才放行；读数不可得 → `source_quality=unavailable` → 按不足处理（fail-closed）。
- **阶梯**（`tier_for`）：`watch`（free < 16 GiB 或 commit 已用 > 70%）/ `soft`（free < 8 GiB 或
  > 85%）/ `reclaim-direct`（free < 5 GiB）/ `hard`（free < 2 GiB 或 > 95%）。S1 **只算档 + 报档**：
  soft 及以上的动作面（拒新重活）由准入门承担；`reclaim-direct` 的直删与 `hard` 的树杀是 S2 的
  §4.6 / §4.8 动作面。档位随拒绝信封一并回传，S2 可据此分类。
- **分档器**（静态、机械可核）：工具名优先（`run_tests` 恒重）；`run_terminal_cmd` 解析命令串——
  管道/`&&`/`;` 分段取各段首程序；穿透 `cmd /c`、`powershell -Command`、`bash -lc`、`sudo env …`
  等 wrapper；命中重程序表（cargo/rustc/make/cmake/docker/npm/pnpm/yarn/7z/tar/dotnet/go/… 共 100+
  项）即重；`python`/`node`/`java` 等条件程序仅在出现 `install`/`build`/`sdist`/`wheel` 等词时判重；
  输出重定向到**真实文件**判重（`> NUL` / `/dev/null` / `$null` / `2>&1` 不判重）。
- **拒绝形态**：`ToolResult{exit_code: Some(1), output: <读数 + 缺口 + "Nothing was started">,
  structured:{error:"resource_insufficient", phase:"pre_issue", action_class, tier, readings}}`。
  拒绝发生在**任何进程启动之前**（汇点位于工具集调用之前）。落 journal 的最终形态（
  `host_resource_denied` 族 / 单漏斗接线）按设计 §5 属 S2，本批不新造字段、不新增事件。
- **装配**：`with_host_resource_safety()` 只挂生产路径（`-p` 与 ACP）；`OrzHost::new` 保持
  `resource_gate: None` → 测试与嵌入式宿主行为与本批之前逐字一致。
- **S1 边界（登记）**：`HEAVY_TOOLS` 含 `run_tests`，但 `run_tests` 走的是 host-owned 固定命令
  路径（`LoopHost::run_tests`），**不经过 `call_tool` 汇点**，故 S1 未门控该路径；其拒绝形态要接
  `TestRunResult` 与本批之外的失败信封，归 S2 与事件族同批处理（Run A 的 54 条 cargo 命令全部由
  `run_terminal_cmd` 发出，本次门已覆盖）。

### 3.2 D：decode-first 门（**比设计 §4.4 收窄一处**）

- 设计 §4.4 的链为「UTF-8 BOM → UTF-16LE/BE BOM → 无 BOM 奇偶启发式 → GB18030 试解码」。
  实施中实测：把 **GB18030 拉进二进制判定之前**会让「文本扩展名 + 真二进制内容」从**拒读**变成
  **乱码文本**（首版实现对 PNG 头字节给出 `Some(("塒NG…", "gb18030"))`）——即该门会**削弱**它本该
  互补的二进制判定。
- 终态：门只承接「框架当前判错」的那一类——**宽字符文本**（UTF-16 BOM / 无 BOM 奇偶启发式），
  外加「无 NUL 的 UTF-8 / UTF-8 BOM」；**GB18030 留在历史路径**（`decode_text` 在 `is_binary` 之后），
  NUL-free 的 GB18030 文本照旧读回 `gb18030`，真二进制照旧拒读且文案不变。
- 第二条实测：UTF-16LE 的 ASCII 字节**是**合法 UTF-8（U+0000 合法），故门的 UTF-8 阶段必须
  先拒绝「含 NUL 的 UTF-8」，否则宽字符文本会被 `utf-8` 短路成乱码。二者都已在单测钉住。
- 摩擦可见性沿用既有面：`output_encoding` 已被机械审查（`diagnostics.rs` 的 `encoding_lossy`
  签名 + 关键字段）消费，本批不新造 `encoding_detected` 字段（设计 §11 裁决 9）。

### 3.3 F：Job 硬上限（**与设计 §4.7 的两级形态不同**）

- **实测事实**：`AssignProcessToJobObject(outer_job, inner_job_handle)` 在本机返回
  `ERROR_INVALID_HANDLE (0x80070006)`（空 job 亦复现）——即「显式 job 套 job」不可用；Windows 的
  job 嵌套是**隐式**的（由已在父 job 内的进程创建子 job 才会形成层级）。把 `orz.exe` 自己放进
  带限项的 job 会让 commit 上限与 `KILL_ON_JOB_CLOSE` 直接作用在 agent 本体上（比它要防的事更坏，
  与 §4.8 拒绝每进程限项同源理由）。
- **终态**：run 级**一次决策**（`install_global_run_job(JobLimits)`：commit =
  `min(80% × limit, limit − 4 GiB)`、CPU 80%、并发进程数 = 核数），**每个工具调用 Job 承载**这些
  限项 → 内核对该调用的整棵进程树强制。两次 2026-09-12 事故都是**单个重活**打穿，因此该形态对
  事故面等价；残余 = N 个并发调用各持一份上限而非共享聚合上限，**登记给 S2**（进程树生命周期
  子项 B 拥有层级问题）。
- 内核读回是权威（§6 判据 12 的机械面）：Windows 会把 commit 上限**向下取整到页/提交粒度**
  （实测 `1,500,000,000 → 1,499,996,160`）；reserve 规则在 limit ≤ 4 GiB 时把上限压到 0 → 视为
  **不设 commit 上限**（0 会让 job 连 spawn 都过不去）。二者都在单测与断言里钉住。
- 非 Windows：`RunResourceJob` 记录意图并自报 `kernel_enforced=false`（不假装强制）；Linux 侧
  的真正等价物需要 cgroup，而本设计明确不引入容器化前提。

**§3.3 更正（2026-09-12，独立复核 F-1；已由 S1.1 落码收口）**：本段据以降级为一级形态的
实验**不成立**——`AssignProcessToJobObject(outer_job, inner_job_handle)` 是把 **job 句柄**
放进 **process 参数位**，返回 `ERROR_INVALID_HANDLE` 与"嵌套能力"无关；Windows 的嵌套是
**进程指派**属性（Win8+），规则是"先指派到层级根部 job，再指派子集到子 job"。独立复核以
自建探针实测：同一进程先入普通 job、再入带 300 MiB commit 上限的 job，两次指派均成功且
1 GiB 提交仍被拒（对照组无 job 时提交成功）。因此 **S1.1 恢复设计 §4.2 第 1 条的两级形态**
（run 级 `RunResourceJob` 持限项 + 每次调用 `ProcessGroup` 持 `KILL_ON_JOB_CLOSE`，先根后子），
"N 并发调用各持一份上限"的残余**已消除**（改为 run 级汇总；一手来源：`JOB_OBJECT_LIMIT_JOB_MEMORY`
= *job-wide sum of their committed memory*）。证据与复现方式见
[`0Z_S1_INDEPENDENT_REVIEW_2026-09-12`](0Z_S1_INDEPENDENT_REVIEW_2026-09-12.md) §3.1。

## 4. 实施中发现（登记，含一条与本批同形的真机事故）

1. **盘满复现（与 Run A 同形，第一手）**：本批 `cargo test` 全量 debug 编译期间 `D:` 被写满
   （`FreeGB=0.00`），编译器以 `磁盘空间不足 (os error 112)` 失败——与 Run A 的
   `journal os error 112` 同一错误面。回收到 `orz/target/debug/incremental`（**cache 类、可再生**，
   14.76 GB；未触碰任何 run 内容/证据/产物面），随后以 `CARGO_INCREMENTAL=0` 续跑。
   **设计含义**：预检门在 13.4 GiB free 时会**放行**这次重活（≥ 8 GiB），而它实际需要 20+ GB——
   即「预检看不见写入量」。这直接支持设计 §3.4 的残余风险条目与 S2 的回收阶梯 / 在跑采样
   （watch→soft 的**过程中**响应）是**承重件**而非可选件，S4 满盘注入应覆盖该形态。
   批末复现第二次读数：`cargo clippy --all-targets` 等收尾验证后 D: free 降到 **3.64 GiB**
    （低于用户裁决的 reclaim-direct 5 GiB 线）→ 按该口径**直删 cache 类** `orz/target/debug`
    （**28.13 GB**，dev/test 产物，可再生；`target/release` 载体 6 GB 与全部 run 内容/证据未动）
    → D: free 回到 **30.70 GB**。这是设计 §4.6.1 阶梯语义的**操作者手执行**版本（框架机制在 S2 落地）。
2. **Job 不能跨层级聚合**（见 §3.3），已相应调整 F 的形态并登记残余。
3. **commit 上限被内核取整**、**reserve 压零**（见 §3.3）——读回权威，测试容差一个页。
4. **GB18030 前移会削弱二进制判定**（见 §3.2），已收窄并固化为单测。
5. **含 NUL 的「合法 UTF-8」会短路宽字符判定**（见 §3.2），已固化为单测。
6. **既有测试面摩擦（非本批引入）**：`orz-host` lib 全量并行跑时
   `call_tool_timeout_kills_process_tree` 会偶发失败（python 冷启动 + 并行争用），**单线程**
   （`--test-threads=1`）全绿；与项目既有记录口径（单线程）一致。

## 5. 验证证据

| 命令 | 结果 |
|---|---|
| `cargo test -p xai-tty-utils --all-targets` | **25 passed / 0 failed**（含 `resource_job` 6 条：三条限项内核读回、空限项自报未强制、per-call job 继承 run ceilings、**commit 上限真实咬合**——256 MiB job 上限下子进程 1 GiB 提交被内核拒绝） |
| `cargo test -p orz-host --lib -- --test-threads=1` | **308 passed / 0 failed / 5 ignored**（含 `resource_gate` 模块 15 条 + 汇点接线 2 条 + 生产装配面 1 条） |
| `cargo test -p orz-tools --lib util::encoding` | **20 passed / 0 failed** |
| `cargo test -p orz-tools --lib implementations::grok_build::read_file` | **121 passed / 0 failed** |
| `cargo test -p orz-bin --bins` | **41 passed / 0 failed**（24 + 2 + 15，三 bin 目标） |
| `cargo check -p orz-loop` | 通过（仅 `host.rs` 文档行变更） |
| `cargo fmt --all -- --check` | 本批文件干净；唯一差异为 `crates/orz-bin/tests/acaf_e2e.rs:324`（**本批未触碰**的既有差异） |
| `cargo clippy -p xai-tty-utils -p orz-host --all-targets` | 本批新增/修改文件**零告警**（残余告警位置均在未触碰文件） |
| `python scripts/generate_orz_source_manifest.py` | 重算 **1444** 条（原 1441；+2 本批新文件 +1 0v-C 遗留） |
| `python scripts/check_repository.py` | **`valid: true` / `error_count: 0`**（247 schemas 等全过） |

构建环境注记：`PROTOC=D:\CLI\orz\bin\protoc.exe`（`orz-tools-api` build script 需要）；
`CARGO_INCREMENTAL=0`（见 §4 发现 1）。

## 6. 与既有条目/设计的关系

- **设计 §4.1 / §4.4 / §4.7**：口径照落；偏离三处均在本记录 §3 显式登记（GB18030 收窄、
  job 层级形态、reserve 压零语义）。
- **`GAP-ENCODING-GATE`（implemented）**：D 不新造字段，复用 `output_encoding` + 既有解码链。
- **`OPS-PROTOCOL`（pending）**：本批只承接其「判断下沉机械层」正典；删除/回收面归 S2。
- **`AUTH-GSA-SESSION-VOLUME`**：探针与 ceilings 按「装配期注入」同族形态（`with_host_resource_safety`）。
- **`AUTH-TOOL-EXECUTION-REFORM`（TER）**：时间轴归 TER，本批为空间轴；`hard` 档树杀复用 TER 既有
  kill 面（S2 接线）。
- **0v-C（`GAP-JOURNAL-CHAIN-DOUBLE-SEAL`）**：不合批，不搭车。

## 7. 交接（S2 起手清单）

1. **事件面**：`host_resource_denied`（或按 §5 表格命名）落 `stamp_failure` 单漏斗 pre-issue 族
   + Rust 法官 + Python 冻结镜像 + fixture 正负例；决定 `policy_denial.source` 是否需要新枚举值
   （若需要，四处镜像同批：Rust judge / Python reference / 两处测试清单）。
2. **F 残余**：并发调用共享聚合上限（若需要）的层级方案；`resource_limit_hit` 的识别与标注；
   `RunResourceJob::kill` 与 hard 档树杀的接线。
3. **A 的档位动作面**：`reclaim-direct` 直删 `cache`、`hard` 树杀 + `resource_exhausted` + 收尾；
   在跑重活的 5 s 采样与跨档事件。
4. **B / C / E**：`.gsa/process_trees/` 登记 + 孤儿扫除三条件三硬化；ENOSPC 降级 + `run_terminated`
   + `TERMINAL.json` + `degraded_complete` 分类；回收机制（轮数窗口 2 轮 / reclaim-direct / 审计先行 /
   git 兜底）。
5. ~~**收口**：`orz` 子模块提交 + `orz_source_manifest.sha256` 重算 + `check_repository valid: true`~~
   —— **已于 2026-09-12 完成**：提交 `73a8f25c`（10 改 + 2 新增）、manifest 重算 **1444 条**、
   `check_repository` **`valid: true` / `error_count: 0`**。
