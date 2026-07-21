# Windows-first Process Runtime 契约 v0.1

状态：development-only implementation contract；目标是验证 Windows 进程、取消、输出和 terminal-event 语义，不是生产 sandbox。

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
