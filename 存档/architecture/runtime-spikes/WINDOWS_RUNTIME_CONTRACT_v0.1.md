# Windows-first Process Runtime 契约 v0.1

> Archive metadata: original_path=`architecture/WINDOWS_RUNTIME_CONTRACT_v0.1.md`; archived_at=`2026-08-09`; final_status=`runtime_spike/evidence_only`; superseded_by=`adr/ADR-0010-fusion-runtime-and-agent-architecture.md`; authority=`historical process contract only`.

状态：development-only implementation contract；目标是验证 Windows 进程、取消、输出和 terminal-event 语义，不是生产 sandbox。

> **2026-08-09 状态修正**：本文第 1–5 节保留为早期 runtime spike 的历史契约，不应据此声称 ORZ 已完成系统性的 Windows 原生适配。当前准确定位是 **Windows-first / Windows-only pre-beta**：基础跨平台能力主要继承自 Grok Build 与 Rust 生态，ORZ 已增加部分 Windows 专项加固，但尚未经过多机器、多 Windows 配置和多项目类型的 beta 验证。后续适配与案例库闭环见第 6 节。

## 1. 工程策略

本项目当前以 Windows 为首要运行平台。为避免完整 Grok Build Rust workspace 带来的磁盘、工具链和迁移成本，先用 Python 标准库与 Win32 Job Object 完成可丢弃 runtime spike。只有在这些语义通过回归后，才把稳定边界迁入最小 Rust 控制面。

普通 CLI 工程不依赖旧 INDEX/MAP/self-check；具体 LIF claim-bearing 工作仍按需跨目录读取来源。

## 2. 不变量

### WIN-PROC-001：无 shell 默认值

- executable 必须是已解析的绝对文件路径；
- arguments 使用 sequence 传给 `subprocess.Popen`；
- `shell=False` 固定，不让 cmd/PowerShell 再解释参数；
- command journal 默认保存 executable、参数数量和参数 digest，不保存可能含密钥的原始参数。

### WIN-PROC-002：进程树 containment 可见

- 每个 action 创建独立 Job Object；
- 设置 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`；
- root process 启动后立即通过 `AssignProcessToJobObject` 关联；
- Job Object 分配失败必须标为 `degraded`，strict/formal action 不得继续；
- 不设置 `CREATE_BREAKAWAY_FROM_JOB`，子进程默认继承 job chain；
- 当前 Popen 后关联存在一个很小的 start→assign race，V0 必须记录为 limitation；生产实现应使用 suspended creation 或 creation-time job attribute 消除。

Windows 8/Server 2012 起支持 nested jobs，因此在 CI/宿主进程本身位于 job 内时，可以在满足层级约束的前提下继续分配子 job。分配是否真正成功必须读 Win32 返回值，不能由平台版本推断。

### WIN-PROC-003：取消是分阶段动作

timeout 后依次尝试：

1. 对 `CREATE_NEW_PROCESS_GROUP` 进程发送 `CTRL_BREAK_EVENT`；
2. 在冻结的 grace period 内等待；
3. 若仍运行，关闭 kill-on-close Job Object，终止整个受控进程树；
4. 只有 job containment 不可用时才退化到 root `TerminateProcess`；
5. 继续 wait/drain，直至获得 exit code 或正式 `unknown`。

取消请求不等于取消成功；requested method、实际 escalation、exit code 和 terminal state 分开记录。

### WIN-PROC-004：输出有界但必须持续 drain

- stdout/stderr 使用独立 reader 持续排空，避免 pipe buffer deadlock；
- 全量字节进入 SHA-256 和 byte count；
- 内存只保留固定上限的 preview，默认 report 不写原文；
- 超出上限标记 `truncated=true`，不能把 preview 当作完整日志；
- 正式长跑后续应改为原子文件 sink + rolling digest，而不是无限内存缓冲。

### WIN-PROC-005：唯一 terminal event

每次已启动或启动失败的 action 生成且只生成一个 terminal event：

- exit code 0：`succeeded`；
- 非零 exit code：`failed`；
- timeout/cancel path：`cancelled`，即使进程自行响应 CTRL_BREAK 后退出；
- 无法获得 exit code：`unknown`；
- containment degraded 不改写实际 exit state，但使 strict/formal readiness 失败。

## 3. Spike 范围

`prototype/fep_agent_proto/windows_process.py` 实现：

- Win32 Job Object create/set/assign/close；
- fully-qualified executable、no-shell Popen；
- 有界双管道 drain 和完整 digest；
- timeout、CTRL_BREAK、job-close escalation；
- 单个结构化 terminal event；
- 无副作用的 Python child smoke。

不实现：token 权限降级、AppContainer、网络 sandbox、文件系统 virtualization、GPU 限额、Windows service、真实 model/tool execution 或科学产物验证。

## 4. 验收

- Windows success child：exit 0、两个输出 digest、一个 succeeded terminal；
- Windows failure child：非零 exit、一个 failed terminal；
- Windows timeout child：cancel requested、进程不遗留、一个 cancelled terminal；
- 带空格/Unicode argument 由 sequence 原样到达 child，不经过 shell；
- Job Object 分配状态和 limitation 始终出现在 report；
- Ubuntu CI 不伪装执行 Windows probe，只做 import/schema 与显式 skip。

## 5. 官方来源

- [Microsoft Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects)
- [Microsoft AssignProcessToJobObject](https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-assignprocesstojobobject)
- [Microsoft Nested Jobs](https://learn.microsoft.com/en-us/windows/win32/procthread/nested-jobs)
- [Microsoft JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_basic_limit_information)
- [Python subprocess documentation](https://docs.python.org/3/library/subprocess.html)
- [Python signal documentation](https://docs.python.org/3/library/signal.html)

## 6. 后续 Windows 适配与案例库闭环（2026-08-09 补充）

### 6.1 定位与声明边界

1. **Windows-first 是产品选择，不是兼容性完成声明。** 当前只保证在项目维护者的受控 Windows 环境中持续开发和验证；进入 beta 前不得写成“完整适配 Windows”或“广泛兼容 Windows 环境”。
2. **继承能力与 ORZ 新增能力分开记录。** Grok Build、Rust crate、PowerShell 或其他上游已经提供的能力标为 inherited；ORZ 新接线、加固或改变语义的部分标为 added/modified；仅在真实运行中观察到但尚未修复的现象标为 observed。
3. **事故不自动等于适配成果。** 只有问题已经形成可复现证据、根因边界、修复和回归验证，才能计入“已适配”；单次成功运行也不能推广为平台兼容结论。
4. **不以多 OS 覆盖为目标。** 本阶段不承诺 Linux/macOS 产品适配；Linux 仅可作为评测 harness、构建目标或必要的机械验证环境，不改变 Windows-first 定位。

### 6.2 后续适配观察面

Windows beta 期间至少覆盖以下观察面；它们是问题发现路由，不代表当前均有缺陷或均已实现：

- 进程与终端：Job Object、子进程继承/逃逸、取消、超时、ConPTY、输出 drain、长任务恢复；
- Shell 与参数：Windows PowerShell 5.1/PowerShell 7、CMD/Git Bash/MSYS 边界、引用与转义、退出码、环境变量；
- 文件系统：盘符与 UNC、长路径、Unicode/中文路径、大小写、CRLF、软链接、文件占用与原子替换；
- 凭据与落盘：Windows Credential Manager、`GROK_HOME`、工作区/本体/系统写点分层、权限降级与秘密脱敏；
- 工具链与项目类型：Git、MSVC、Python、Node、Rust、.NET/Visual Studio 项目的原生构建与测试路径；
- 隔离与安全：Job Object 只负责进程 containment 时的能力边界，以及 ACL、受限令牌、AppContainer、网络隔离的缺口；
- 环境差异：Windows 10/11、企业策略、杀毒/EDR、开发者模式、Docker Desktop/WSL、磁盘与临时目录资源；
- 发行体验：安装、升级、卸载、首次凭据配置、诊断命令、错误提示和可恢复性。

### 6.3 三层记录体系

Windows 适配证据采用三层结构，禁止用一份混杂文档同时承担全部职责：

1. **事故台账（incident ledger）**：追加式记录所有真实异常，包括尚未查清、不可复现或最终判定为非 Windows 特有的问题；不得为了整洁删除失败记录。
2. **精选案例库（case corpus）**：只收录根因边界相对稳定、可复现或有充分运行证据、对后续开发具有普遍价值的案例。案例必须区分 `windows_native`、`cross_platform_agent`、`harness_environment`，不能把 Docker 磁盘耗尽等环境事故直接宣传为 Windows 产品能力。
3. **回归测试（regression tests）**：将已稳定的案例转化为机械断言。能够自动化的案例应形成修复前失败/修复后通过的测试；暂时不能自动化的案例必须保留人工复核步骤和证据位置。

推荐流转关系：

```text
真实异常/观察
    -> 事故台账（允许 unknown）
    -> 精选案例（复现 + 根因/边界 + 脱敏）
    -> 回归测试（机械断言或明确人工复核）
    -> beta 兼容性证据
```

### 6.4 案例最小字段

后续案例 schema/YAML 至少应包含：

- 稳定 `case_id`、首次发现时间、最近复核时间与状态；
- 来源（探索跑分、正式跑分、beta 用户、日常任务、审计）；
- ORZ commit/二进制 digest、模型与模式、关键预算和配置；
- Windows 版本、Shell、CPU 架构、项目/工具链与相关环境事实；
- 用户可见症状、预期行为、最小复现步骤和原始证据引用；
- 根因或 `unknown`，以及 `windows_native` / `cross_platform_agent` / `harness_environment` 分类与分类依据；
- 修复内容、继承/新增/修改归属、已知剩余边界；
- 回归测试或人工复核入口，以及修复后复核结果；
- 脱敏状态。API key、用户名、绝对用户目录和私人项目内容不得直接进入可分发案例包。

### 6.5 晋级门槛

- **台账 -> 案例库**：具有稳定复现，或具有足够完整的日志/journal/二进制 provenance；根因或至少失败边界已收窄；内容已脱敏；结论没有超出证据。
- **案例库 -> 自动回归**：断言可重复、环境前提明确、失败与成功语义唯一；优先保留修复前失败/修复后通过证据。
- **案例库 -> beta 兼容性声明**：同类案例在不止一个任务或环境中得到验证，且没有未披露的高严重度反例。单一维护者机器上的通过只能标为 maintainer-observed。
- **跑分与平台适配分开统计**：SWE-bench-Live/Windows 和 Terminal-Bench 成绩用于衡量任务解决能力；案例闭环率、环境覆盖和复发率用于衡量 Windows 工程成熟度，二者不得互相替代。

### 6.6 当前候选种子（待逐项审计后入库）

以下仅是回查入口，不因列入本文自动获得“已闭环案例”状态：

- Job Object 树杀、工具超时与 `kill_all` 闩闭导致后续 spawn 失效；
- PowerShell/MSYS 路径转换、参数接线和未知 flag 静默忽略；
- stale binary 导致守卫或新参数未实际生效；
- Windows Credential Manager 三凭据读取与失败提示；
- `GROK_HOME` 本体写点、可写探针和降级链；
- Docker Desktop/WSL、宿主卷、C 盘资源与评测产物落盘问题。

候选种子应回查运行日志、审计文档、修复 commit 和现有测试，再决定分类、是否拆分及能否晋级。

### 6.7 beta 阶段要求

- pre-beta 先用维护者环境和 SWE-bench-Live/Windows 暴露高频故障，不将题集通过率包装成兼容性覆盖率；
- beta 开始后按 Windows/Shell/项目类型记录环境矩阵，优先修复阻断安装、凭据、执行、取消、恢复和数据安全的问题；
- 每次 Windows 专项修复必须先落事故台账，满足门槛后再进入精选案例与回归集；
- 对尚无证据的环境明确写 `unknown/not tested`，不得从代码可编译推断为可用；
- 正式发布前再冻结一版“已验证环境 + 已知限制 + 案例/测试入口”，作为 README 能力声明的证据源。
