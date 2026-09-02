# Windows 精选案例库

本目录接收从事故台账晋级的脱敏案例。首批按 ADR-0010 §11.7 裁决：

- `ORZ-WIN-PROC-001` — tool_timeout 进程树终结（候选）
- `ORZ-WIN-PROC-002` — task_cancel 进程树终结（候选）
- `ORZ-WIN-PROC-003` — parent_exit 孤儿树终结（候选）
- `ORZ-WIN-PS-001` — PowerShell 5.1 管理面摩擦（提权/编码/Hyper-V，候选；
  2026-09-01 P0-0l-② S4 载体准备晋级）
- `ORZ-WIN-SBX-001` — Session 0 服务窗口工作站/桌面 ACL 导致跨用户 spawn 0xC0000142
  （候选；2026-09-02 win-s4 三臂排障晋级）
- `ORZ-WIN-SBX-002` — CreateProcess 家族选型矩阵（CPTW 拒扩展信息 87 / CPAU 需
  SeAssignPrimaryTokenPrivilege 1314 / 环境块 0x400，候选）
- `ORZ-WIN-SBX-003` — AppContainer 子进程兼容面（CLR/ctypes/whoami 不可用、管道句柄、
  TEMP 重写、Low 完整性写限制、包目录归属，候选）
- `ORZ-WIN-ACL-001` — 受保护系统根 ACL 修改（icacls exit 5 → .NET；Registry provider；
  规则构造参数顺序，候选）
- `ORZ-WIN-PS-002` — 加固后 guest 管理通道（CIM/DCOM 被出站墙阻断 → schtasks 等原生
  命令行；退出码 Int32 溢出，候选）
- `ORZ-WIN-CTYPES-001` — ctypes P/Invoke 坑清单（函数名/DLL 归属/argtypes/结构数组/
  句柄包装/指针生命周期/Path 语义，候选）

**状态口径**：全部条目均为 `candidate`（晋级自
`GROK_WINDOWS_CHILD_TREE_PROBE_2026-07-23.md` 的三场景，fixture/verifier/digest
齐全，以及 2026-09-01/09-02 S4 载体搭建与三臂排障的实机复现），但未宣称产品级闭环——
探针是 fake-only fixture 证据，不构成通用进程安全或科学正确性证明。

待晋级/不晋级清单（ADR-0010 §11.7）：
- Observer 子进程泄漏保留在事故路由（`docs/incidents/windows/`），待 raw artifact
  identity 与结构化 verification 补齐。
- Windows Native Sandbox raw TCP 残留为 open limitation（`WIN-LIM-001`），不晋级。
- Credential hardening 为 offline implementation evidence，不晋级为 Credential
  Manager 实机兼容案例，直至真实 credential-read 路径完成脱敏验证。
