# ORZ-WIN-SBX-001 — Session 0 服务窗口工作站/桌面 ACL 导致跨用户 spawn 0xC0000142（案例候选）

- **状态**：`candidate`（2026-09-02 晋级候选；未宣称产品级闭环）
- **晋级裁决**：2026-09-02 正式 S4 Windows HIGH-NIST 硬化 VM（`win-s4`）三臂探针排障中，
  跨用户 spawn 全部以 `0xC0000142 (STATUS_DLL_INIT_FAILED)` 失败，经窗口工作站/桌面
  SDDL 读取 + 授予-复测-还原闭环确认根因；该现象极不直观（CreateProcess 成功、子进程
  启动即崩），复现命令与可靠路径齐备，晋级为精选案例候选（`windows` 类）。
- **来源证据**：2026-09-02 会话执行记录；`scripts/s4_vm_repro_v3.ps1` / `s4_vm_repro_v4.ps1`
  （SDDL 读取、授予-复测-还原）；`assurance/windows_sandbox.py` `_grant_session_desktop_access`；
  结果文件 `D:\CLI\_windows_high_nist\vm-repro-v3-result.txt` / `vm-repro-v4-result.txt`。
- **能力**：在 session 0 / 非交互服务上下文（计划任务、Windows 服务）中，以"非任务属主
  令牌"（受限管理员令牌、LogonUser 标准用户令牌）spawn 进程前，预判并修复
  `0xC0000142`：授予目标用户 SID（含 AppContainer SID）当前窗口工作站与桌面访问权。

## 观察记录

1. **症状**：`CreateProcessWithTokenW` / `CreateProcessAsUserW` 返回成功，子进程
   （cmd/powershell）随即以 `0xC0000142` 退出；沙箱观察里"进程创建成功但 DLL 初始化失败"。
   - 复现：计划任务（session 0）内以 AgentUser LogonUser 令牌 spawn `cmd.exe /c exit 42`。
   - 早期误归因：进度台账曾记"受限令牌（禁用/deny-only Administrators）无法 spawn 任何
     进程"——实为同一 ACL 问题的表象，而非受限令牌本身。
2. **根因**：session 0 的服务窗口工作站（`Service-0x0-<hex>$`）与桌面（`Default`）的 DACL
   只授予**任务属主用户 SID**（如 `...-1001`）与 `BUILTIN\Administrators`。子进程继承调用者
   的 winsta/desktop；当其令牌既非属主、又不含有效 Administrators（受限管理员令牌的
   Administrators 为 disable/deny-only，标准用户令牌根本没有）时，kernel32/user32 初始化
   因无法访问桌面对象而失败。
3. **可靠路径**：spawn 前在管理员/SYSTEM 上下文对 `GetProcessWindowStation()` 与
   `GetThreadDesktop(GetCurrentThreadId())` 读取 SDDL，为目标 SID 插入 `(A;;GA;;;<SID>)`
   ACE（幂等：已存在则跳过），再 `SetUserObjectSecurity`。服务 winsta 每次启动/每个任务
   进程独立，故需**逐次授予**，不能只做一次性加固。
   - 授予 AgentUser（-1002）GA 后：子进程 EXIT 42；还原 SDDL 后立即回到 0xC0000142——
     闭环证明 ACL 是唯一因素。

## 回归入口

- `assurance/windows_sandbox.py` → `_grant_session_desktop_access()`（含 extra_sids：
  high-nist 追加 AppContainer SID）。
- `scripts/s4_vm_repro_v3.ps1` / `s4_vm_repro_v4.ps1`（SDDL 获取/设置 + 授予-复测-还原）。

## 边界

- 仅覆盖 session 0 / 非交互服务上下文；交互式会话 `winsta0\Default` 的 ACL 语义不同。
- 修改窗口工作站/桌面 DACL 属系统级对象变更，仅在受控 VM/容器内使用；生产环境建议改用
  CreateProcessWithLogonW（交互桌面场景）或专用服务会话。
- 复现平台：Windows 11 企业评估版 25H2 build 26200 + PowerShell 5.1。
