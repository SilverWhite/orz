# ORZ-WIN-PS-001 — PowerShell 5.1 管理面摩擦（提权/编码/Hyper-V，案例候选）

- **状态**：`candidate`（2026-09-01 晋级候选；未宣称产品级闭环）
- **晋级裁决**：2026-09-01 P0-0l-② 正式 S4 载体准备（Hyper-V 专用 VM 创建）过程中
  连续踩到 5 类 PowerShell/Win32 管理面摩擦，均有明确复现命令、根因与可靠替代路径，
  晋级为精选案例候选（`windows` 类）。
- **来源证据**：2026-09-01 会话执行记录；`scripts/s4_create_vm.ps1` /
  `scripts/s4_check_tpm.ps1` / `scripts/s4_enable_tpm.ps1`（修复后脚本）；
  `D:\CLI\_windows_high_nist\vm-create.log`（初始失败日志）。
- **能力**：在 PowerShell 5.1 + Windows 10/11（25H2）管理面上，规避 5 类常见摩擦并
  给出可靠执行路径——无 BOM UTF-8 中文脚本的解析错误、`Start-Process -Verb RunAs`
  的 ArgumentList 拼接、UAC 过滤令牌与真管理员令牌区分、Hyper-V `New-VM` 参数名与
  虚拟 TPM 启用前置、`vmconnect` 权限。

## 观察记录（5 类摩擦，各含复现与可靠路径）

1. **无 BOM UTF-8 中文 PS1 在 PS 5.1 下按 ANSI(GBK) 解析 → 语法错误**。
   - 复现：`apply_patch` 生成含中文 `Write-Step "OK ..."` 的无 BOM UTF-8 脚本，
     提权执行报 `表达式或语句中包含意外的标记` / `字符串缺少终止符`，
     错误上下文把 UTF-8 字节按 GBK 显示成乱码，定位困难。
   - 根因：Windows PowerShell 5.1 对无 BOM 文件按系统 ANSI 代码页解码；
     中文多字节序列被拆成单字节，引号/花括号配对错乱。
   - 可靠路径：PS 脚本保持 UTF-8 **BOM**，或全 ASCII 内容；`apply_patch`
     生成脚本后先做非 ASCII 检测（`Select-String '[^\x00-\x7F]'`）。

2. **`Start-Process -Verb RunAs` 的 ArgumentList 拼接不可靠**。
   - 复现：`-ArgumentList @(...,'-Command',"& 'x.ps1' *> 'log'")` 中 `*>` 被当
     字面参数；含空格/引号/`$LASTEXITCODE` 的命令被拆分或转义错位，日志不落盘、
     ExitCode 误导（0 但未执行）。
   - 可靠路径：把整段命令 Base64 编码后用 `-EncodedCommand` 传递
     （`[Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($cmd))`），
     子进程内部自行重定向日志。

3. **UAC 过滤令牌 ≠ 真管理员**。
   - 复现：Codex 会话进程 `codexsandboxonline` 非管理员；`require_escalated`
     得到 `SWITCH\1` 但 Administrators 组 `deny-only`（IsAdmin=False）；
     Hyper-V/`Get-WindowsOptionalFeature` 仍被拒。
   - 可靠路径：`Start-Process -Verb RunAs` 弹 UAC 后子进程才是真管理员
     （实测 `SWITCH\1` IsAdmin=True、管理员组 Enabled）。

4. **Hyper-V 参数名与虚拟 TPM 前置**。
   - 复现：`New-VM -VHDSizeBytes` 报 `找不到与参数名称“VHDSizeBytes”匹配的参数`
     （正确参数为 `-NewVHDSizeBytes` + `-NewVHDPath`）；`Enable-VMTPM` 直接报
     `未配置有效密钥保护程序`（Win11 安装需 TPM 2.0，新 VM 默认无虚拟 TPM）。
   - 可靠路径：`New-VM -NewVHDPath <p> -NewVHDSizeBytes <n>`；
     TPM 前先 `Set-VMKeyProtector -VMName <vm> -NewLocalKeyProtector`，
     再 `Enable-VMTPM -VMName <vm>`（宿主有物理 TPM 时也可用宿主保护器）。

5. **`vmconnect.exe` 需管理员令牌**。
   - 复现：普通令牌启动 `vmconnect localhost win-s4` 报
     `你没有完成此任务所需的权限`。
   - 可靠路径：`Start-Process vmconnect.exe -ArgumentList @('localhost',$vm)
     -Verb RunAs`。

## 回归入口

- `scripts/s4_create_vm.ps1`（全 ASCII、EncodedCommand 执行、`-NewVHDSizeBytes`）。
- `scripts/s4_enable_tpm.ps1`（Key Protector + Enable-VMTPM 顺序）。
- `scripts/s4_check_tpm.ps1`（宿主 TPM 探测）。

## 边界

- 案例覆盖 PS 5.1 + 25H2 管理面；PS 7（pwsh）默认 UTF-8 行为不同，不在本案例范围。
- UAC 弹窗依赖交互式会话；无头/CI 环境需另走服务账户或计划任务提权。
- `-NewLocalKeyProtector` 是本地密钥保护（不绑定宿主 TPM），安全性弱于宿主保护器；
  生产模板建议宿主 TPM 保护器 + 快照回滚。
