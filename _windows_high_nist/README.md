# Windows 加固脚本 + enforcement-probe（P0-0l ②）

> 状态：**S1 实施 + S2 测试 + 全面审查处理完成（2026-09-01）；S3 重建
> （Windows x86_64 三件套，orz f0eeb524）完成 + S4 本机冒烟闭环
> （2026-09-01：三件套启动/守卫符号核验、sandbox control 臂端到端
> compliant、冒烟修复 3 项——ProcThreadAttributeList 查询大小误判、
> 管道 drain 的 c_void_p 句柄取值、CLI --command REMAINDER；35 passed）；
> **硬化 VM 三臂 enforcement-probe 实机闭环（2026-09-02：control /
> non-admin / high-nist 全 PASS（non-admin 10/10、high-nist 19/19、
> sandbox observation compliant）+ 修复链 6 项 + 案例库 6 篇 ORZ-WIN-*，
> 进度见 [S4_PROGRESS_2026-09-02.md](S4_PROGRESS_2026-09-02.md)）**。
> 正式三臂序列（模板切换）+ 任务集 + 跑批 + 缺口登记待续。对应
> [WINDOWS-HIGH-NIST-MAX-FRICTION 设计 §10.2](../docs/WINDOWS_HIGH_NIST_MAX_FRICTION_DESIGN_2026-09-01.md)；
> Linux arm 方法学干跑见 [../_linux_arm_dryrun/README.md](../_linux_arm_dryrun/README.md)。

## 目标（设计 §10.2）

1. **加固脚本（PS/Win32）落模板**：把设计 §3 的 N×F×P 格映射为 VM 模板级
   机器/账户策略（ACL deny-write、注册表只读、UAC 收紧、账户、防火墙出站
   默认 block + 每任务 allowlist、AppLocker 白名单）。
2. **enforcement-probe**：GAK-SBX-001 的 7 项检查扩展为每轴断言集，每臂
   启动前无 agent 先验墙；任一断言失败 → 该臂不作数（fail-closed）。
3. **`assurance/windows_sandbox.py` 由探针扩展为运行环境**：受限 token +
   LOW IL + AppContainer + Job Object 下 spawn 任意命令树（含 orz），
   spawn 期事实写入 run observation（新 schema + 独立 verifier）。

## 策略轴 → 落点

| 轴 | BoundaryBench 语义 | 本 ② 落点 |
|---|---|---|
| Privilege·non-admin | 普通用户、禁提权 | UAC 收紧（EnableLUA=1、ConsentPromptBehaviorAdmin=2）+ RunUser 标准账户 + 受限 token（`windows_sandbox.py`：Administrators 禁用/deny-only、6 特权移除、TokenVirtualizationAllowed=0） |
| Privilege·no-escalation | no_new_privs + 能力剥离 | LOW integrity（`windows_sandbox.py`）+ UIPI + AppLocker 白名单（`-EnableAppLocker`） |
| Privilege·capability drop | 能力剥离 | AppContainer 空 capability（沿用 GAK-SBX-001 机制） |
| Filesystem·readonly-os | 只读 OS、仅工作区可写 | deny-write ACL：C:\Windows / Program Files / Program Files (x86) / ProgramData / 盘符根；HKLM、HKCR 只读 |
| Filesystem·freeze-home | 冻结 home | %USERPROFILE%/%APPDATA%/%LOCALAPPDATA% deny-write + HKCU 冻结 + `%TEMP%` 重定向到 `{workspace}\.tmp`（spawn 期） |
| Network·restrict-egress | 每任务白名单、挡元数据/私网 | 防火墙出站默认 block + 元数据/link-local/RFC1918/组播 block 规则 + 每任务 allowlist 规则；DeepSeek 端点恒放行 |

## 目录

- `hardening/apply_hardening.ps1`：VM 模板加固脚本（三臂模式、幂等、`-Revert`
  可撤销、日志落 `hardening/logs/`）。
- `policy/enforcement_probe.ps1`：每轴断言探针（PASS/FAIL 行 + 可选 JSON）。
- `run/run_enforcement_probe.ps1`：runner（`-Native` 当前会话直跑；
  默认 `-Sandbox` 经 `run_windows_native_sandbox_command.py` 在墙内拉起）。
- 运行环境：`assurance/windows_sandbox.py::run_windows_native_sandbox` +
  `assurance/windows-native-sandbox-run-v0.1.schema.json` +
  `assurance/sandbox_verifier.py::verify_windows_native_run_observation` +
  `scripts/run_windows_native_sandbox_command.py`。

## 运行步骤（在硬化 VM 模板上，管理员 PowerShell）

```powershell
# 1) 加固模板（模板一次性；-Arm non-admin|high-nist）
.\hardening\apply_hardening.ps1 -Arm high-nist -RunUser AgentUser `
    -WorkspaceRoot D:\workspace -EnableAppLocker -AllowlistIp 1.1.1.1

# 2) enforcement-probe 先验墙（每臂一个独立会话/沙箱，无 agent）
.\run\run_enforcement_probe.ps1 -Arm control
.\run\run_enforcement_probe.ps1 -Arm non-admin
.\run\run_enforcement_probe.ps1 -Arm high-nist

# 3) 撤销（如需回滚模板变更）
.\hardening\apply_hardening.ps1 -Arm high-nist -Revert
```

## 断言集（7 项检查 → 每轴）

- **control**：`workspace_writable`、`os_writable`。
- **non-admin**：`workspace_writable`、`non_admin`、
  `token_virtualization_disabled`、`system32_write_blocked`、
  `program_files_write_blocked`、`hklm_write_blocked`、`runas_blocked`、
  `privileges_removed`、`temp_write_succeeded`、`home_write_succeeded`。
- **high-nist**：non-admin 全部 + `programdata_write_blocked`、
  `drive_root_write_blocked`、`hkcufrozen`、`home_frozen`、
  `appdata_frozen`、`temp_redirected`、`low_integrity`、
  `appcontainer_token`、`job_object_assigned`、`network_blocked`、
  `metadata_blocked`、`probe_file_cleaned`；提供 `-AllowlistIp` 时另加
  `allowlist_reachable`（runner 自动把首个 allowlist IP 传给探针）。

`network_blocked` 的探针 IP 会自动避开 allowlist（若用户显式放行 1.1.1.1，
则改用 8.8.8.8 断言阻断），避免自相矛盾。

run observation 的 `checks` 只含 spawn 期事实（token/IL/AppContainer/Job/
TEMP 重定向 + harness 侧 workspace 写探针）；行为级墙断言由
`enforcement_probe.ps1` 在墙内执行，两者互补。

## 已知边界与坑（S4 需在硬化 VM 上逐项核对）

1. **AppContainer 不挡原始 TCP 出站**（GAK-SBX-001 登记残余）：网络隔离
   由宿主防火墙兜底（出站默认 block + allowlist），不得把"AppContainer
   配置正确"写成"网络隔离完成"。
2. **防火墙语义**：block 规则优先于 allow 规则 → 加固脚本把出站默认策略设
   为 block，运行环境只加 allow 规则（`_create_egress_allow_rules`）。
3. **受限 token spawn 需管理员**：`CreateProcessAsUserW` 要求宿主具备
   `SeAssignPrimaryToken/SeIncreaseQuota`；非提权宿主降级为当前 token spawn，
   run observation 自动 noncompliant（fail-closed）。
4. **AppLocker 仅对非管理员生效**：RunUser 必须是标准用户；模板需 Pro/
   Enterprise 且组件可用，否则 `-EnableAppLocker` 记 FAIL（不影响其他墙）。
5. **PowerShell exit code 分裂**（设计 §5 #1/#6）：探针/agent 脚本的"成功"
   必须以断言结果为准，不能只看 `$LASTEXITCODE`；enforcement-probe 的
   exit code 与 PASS/FAIL 行都进入记账。
6. **TokenVirtualizationAllowed=0 是 per-token 构造**：由运行环境在
   `CreateRestrictedToken` 后设置；探针以行为断言（Program Files 写失败 +
   VirtualStore 无残留）验证，而非读注册表。
7. **control 臂基线必须在未加固环境采集**：模板加固后机器级策略
   （UAC/ACL/防火墙默认 block）不会因 `-Arm control` 消失，control 的
   行为基线会失真。请在独立 VM 或加固前快照上跑 control。
8. **RunUser 身份与 harness 的关系**：若 harness 以 RunUser 运行，其
   profile 被冻结时自身写面也受限；AppContainer profile 目录
   `AppData\Local\Packages` 已由加固脚本显式 grant（豁免），其余 profile
   面保持冻结。RunUser 需至少登录过一次（NTUSER.DAT 存在）才能完成 HKCU
   hive 级冻结；未登录时脚本 WARN 并依赖文件层 deny 兜底，建议登录后重跑。
9. **DeepSeek 恒放行有前提**：加固时需能解析 `api.deepseek.com`，或显式
   传 `-DeepSeekIp`；两者都不可用时加固脚本 FAIL（设计不变量不满足），
   不会静默进入评测。`-Revert` 会按规则清单删除防火墙规则并恢复 hosts。
10. **AppLocker 组件缺失时 `-EnableAppLocker` 记 FAIL 并使整脚本退出 1**
    （fail-closed：模板视为未完成），其他墙已应用；README 早期表述
    “不影响其他墙”指墙本身仍会落盘，但退出码不可忽略。
