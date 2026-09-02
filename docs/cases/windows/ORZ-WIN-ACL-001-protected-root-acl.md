# ORZ-WIN-ACL-001 — 受保护系统根的 ACL 修改（icacls/.NET/Registry，案例候选）

- **状态**：`candidate`（2026-09-02 晋级候选；未宣称产品级闭环）
- **晋级裁决**：2026-09-01 至 09-02 `win-s4` 加固脚本开发中，对系统保护根目录/注册表做
  deny 授权时连续踩中三类平台摩擦（icacls 拒绝访问、Registry provider 参数、规则构造
  参数顺序），修复后 StepFailures=0，具备复现与可靠路径，晋级为案例候选。
- **来源证据**：`scripts/s4_vm_icacls_test.ps1` / `s4_vm_reg_acl_test.ps1`；
  `_windows_high_nist/hardening/apply_hardening.ps1`（Invoke-AclDeny/Invoke-RegistryDeny）；
  结果文件 `vm-icacls-test-result.txt` / `vm-reg-acl-test-result.txt`。
- **能力**：在 Windows 11 25H2 + PowerShell 5.1 下对系统保护根目录、注册表键可靠地应用
  deny-write 策略，并正确构造 .NET 访问规则。

## 观察记录

1. **icacls 对受保护系统根失败**：`icacls C:\WINDOWS /deny <user>:(W,AD,DE,DC)` 报
   "拒绝访问"（exit 5），交互式提权亦复现；.NET `Get-Acl/Set-Acl` 可用。
   - 可靠路径：改用 .NET `FileSystemAccessRule` + `Set-Acl`；对盘符根只加非传播 deny
     （不继承，避免误伤工作区）；只处理 NTFS 固定卷（排除 ISO/UDF 光驱与 FAT32/EFI）。
2. **Registry provider 不吃 `-LiteralPath`**：`Get-Acl -LiteralPath 'Registry::HKLM\...'`
   失败 → 用 `-Path`。
3. **RegistryAccessRule 参数顺序**：`(identity, rights, inheritance, propagation, type)`
   写反会把 AccessControlType 传给 inheritanceFlags → 按正确顺序构造。

## 回归入口

- `_windows_high_nist/hardening/apply_hardening.ps1`：`Invoke-AclDeny` / `Invoke-AclGrant` /
  `Invoke-RegistryDeny` / `Get-ProtectedPaths`。

## 边界

- 复现平台：Windows 11 25H2 + PowerShell 5.1；icacls 对普通目录仍可用。
- deny 优先于 allow：对同一主体先 grant 后 deny 时，deny 生效（本案例的冻结语义依赖此点）。
