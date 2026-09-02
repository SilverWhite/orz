# ORZ-WIN-PS-002 — 加固后 guest 管理通道（CIM/DCOM 阻断 → 原生命令行，案例候选）

- **状态**：`candidate`（2026-09-02 晋级候选；未宣称产品级闭环）
- **晋级裁决**：2026-09-01 `win-s4` 加固后，guest 内 CIM/DCOM 管理通道全部失效，导致
  Register/Start-ScheduledTask、Get-CimInstance 不可用；改用原生 `schtasks.exe` 等命令行
  通道后恢复。该摩擦在"出站墙 + 严格策略"的 Windows 环境具普遍性，晋级为案例候选。
- **来源证据**：`scripts/s4_vm_diag_schtasks.ps1` / `s4_vm_run_elev.ps1`（持久任务 +
  `schtasks /run` + 日志轮询）；结果文件 `vm-diag-schtasks-result.txt`。
- **能力**：在 egress 硬化（blockoutbound + RFC1918 block）的 Windows guest 中，用原生
  RPC/命令行通道（schtasks/secedit/reg/net）完成提权执行与状态采集，并正确处理大退出码。

## 观察记录

1. **CIM/DCOM 出站被墙**：加固（防火墙默认出站 block + RFC1918/link-local block）后，
   `Register-ScheduledTask` / `Start-ScheduledTask` / `Get-CimInstance` 全部失败（本机
   DCOM 也被拦）；原生 `schtasks.exe`（RPC）可用。
   - 可靠路径：持久任务只创建一次（`schtasks /create /f ...`），每次 `schtasks /run` +
     轮询任务写出的日志文件；不再反复注册/注销。
2. **无符号退出码 Int32 溢出**：`0xC0000142 = 3221225794` 转 `[int]` 报"值对于 Int32 太大"；
   读观察 JSON 的退出码需用 `[int64]`。
3. **SYSTEM 任务令牌特权差异**：`schtasks /create /ru SYSTEM /rl highest` 的令牌含
   SeAssignPrimaryTokenPrivilege（管理员用户任务令牌没有），是 CPAU 通道的前提。

## 回归入口

- `scripts/s4_vm_run_elev.ps1`（SYSTEM 任务 + 原生 schtasks 通道）。
- `_windows_high_nist/run/run_enforcement_probe.ps1`（`[int64]` 退出码读取）。

## 边界

- 宿主→guest 的 Hyper-V PSSession（WinRM 入站）不受出站墙影响；仅 guest 内 CIM/DCOM
  出站受限。
- 交互式会话的 UAC 弹窗不属于本案例；无头环境用计划任务提权。
