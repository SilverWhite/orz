# ORZ-WIN-SBX-002 — CreateProcess 家族选型矩阵（CPTW/CPAU/环境块，案例候选）

- **状态**：`candidate`（2026-09-02 晋级候选；未宣称产品级闭环）
- **晋级裁决**：2026-09-02 `win-s4` 三臂排障中，run-user spawn 通道连续踩中
  CreateProcessWithTokenW 与 CreateProcessAsUserW 的行为差异（87/1314），经二分复现
  逐项隔离（扩展启动信息、令牌类型、环境块、cwd），得出可复用的选型矩阵，晋级为案例候选。
- **来源证据**：`scripts/s4_vm_repro_cptw2.ps1`（CPTW 组合）、`s4_vm_repro_v5.ps1`
  （EXTENDED 二分）、`s4_vm_repro_v6.ps1` / `s4_vm_repro_v9.ps1`（CPAU 1314 vs SYSTEM
  成功）、`s4_vm_repro_v4.ps1`（环境块矩阵）；`assurance/windows_sandbox.py` spawn 块。
- **能力**：在"以其他用户令牌创建带 AppContainer/Job 属性的进程"场景下正确选型：
  CPAU（SYSTEM 令牌）或 CPTW（纯 STARTUPINFO）+ 后置 Job 分配；正确构造环境块。

## 观察记录

1. **CreateProcessWithTokenW 拒绝 STARTUPINFOEX / EXTENDED_STARTUPINFO_PRESENT**。
   - 复现：CPTW + 合法初始化的 ProcThreadAttributeList + 0x80000 标志 → `87`
     (ERROR_INVALID_PARAMETER)；去掉 EXTENDED（纯 STARTUPINFOW）→ 成功。
   - 二分结论：管道句柄、令牌虚拟化、LOW IL、Job 限制均不是 87 来源；EXTENDED 属性列表
     是唯一因素（`s4_vm_repro_v5.ps1`：A/B/C/E/G 全 87，D_NO_EXT 成功）。
2. **CreateProcessAsUserW 需要 SeAssignPrimaryTokenPrivilege**。
   - 复现：管理员用户计划任务令牌（HL）调用 CPAU + LogonUser 主令牌 → `1314`
     (ERROR_PRIVILEGE_NOT_HELD)；同代码在 SYSTEM 任务令牌下成功。
   - 要点：LogonUser 返回**主令牌**，可直接给 CPAU（无需 DuplicateTokenEx）；文档中
     "令牌不可指派时才需要 SeAssignPrimaryTokenPrivilege"在该环境下并不成立——实测必需。
3. **环境块**：UTF-16 块必须带 `CREATE_UNICODE_ENVIRONMENT (0x400)`；ANSI 块不带。
   - 复现：UTF-16 块无 0x400 → 87；带 0x400 → 成功；`create_unicode_buffer` 与
     `c_char_p(bytes)` 两种指针构造均可（注意保持对象存活）。
   - 曾误判"环境块导致 87"：真凶是 cwd（`C:\s4\...` 对目标用户不可写/不可遍历）；
     cwd 换成目标用户可写路径（`C:\workspace`）后所有环境块组合成功。
4. **可靠路径**：需要 AppContainer/Job 创建期属性 → CPAU + SYSTEM 任务令牌
   （`schtasks /ru SYSTEM`）；CPTW 只接纯 STARTUPINFO，Job 改创建后
   `AssignProcessToJobObject`；cwd 必须是目标用户可写/可遍历路径。

## 回归入口

- `assurance/windows_sandbox.py` spawn 块（CPAU 优先 + CPTW 纯 SI 回退 +
  `_enable_process_privilege("SeIncreaseQuotaPrivilege")`）。
- `scripts/s4_vm_run_elev.ps1`（持久任务强制 `/ru SYSTEM`）。

## 边界

- 复现平台：Windows 11 25H2 build 26200；API 行为在不同版本可能有差异。
- SeAssignPrimaryTokenPrivilege 通常只出现在 SYSTEM/服务令牌；管理员用户令牌没有。
- 命令行长于 1024 字符（CPTW 文档限制）未在本次覆盖。
