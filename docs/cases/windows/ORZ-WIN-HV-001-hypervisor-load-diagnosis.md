# ORZ-WIN-HV-001 — Hyper-V hypervisor 加载状态鉴别：固件读数怪癖 vs 真故障（案例候选）

- **状态**：`candidate`（2026-09-07 晋级候选；未宣称产品级闭环）
- **晋级裁决**：2026-09-07 S3/S4 集中实机验证批 T0 阻塞排查（docker/WSL2/Hyper-V
  VM 全阻）与 2026-09-01 S4 VM 载体准备期的读数误判两轮合并——同机上"固件读数
  False"一次是怪癖、一次是真故障，形态几乎相同而结论相反，鉴别路径完整可复用。
- **来源证据**：`Microsoft-Windows-Hyper-V-VMMS-Admin` 事件 15130/20148/33540
  （2026-09-06/07 实测）；codex 会话记录 2026-09-01（VM 创建期诊断与物理机核实）；
  `scripts/s4_hv_diag.ps1` 本轮诊断实跑输出。
- **能力**：在物理机（铭瑄/Maxsun MS-TZZ H610ITX + i5-12400F，Win 11 25H2 内核）
  上，用**事件日志 + 物理环境关键查询**区分三类状态——(a) 固件读数假阴但虚拟化
  完全可用；(b) 本 boot hypervisor 未加载的真故障；(c) BIOS 级虚拟化被关。

## 观察记录（两轮同形异诊）

**Round 1（2026-09-01，读数怪癖）**：`systeminfo` 报 "A hypervisor has been
detected" + `VirtualizationFirmwareEnabled=False` → 一度误判"本机本身就是一台
虚拟机"。修正两条：① "hypervisor detected" 在物理机启用 Hyper-V 时同样出现
（Windows 自身作为管理分区跑在自家 hypervisor 上），不构成 guest 证据；② 物理
身份以固件表为准（`Win32_BIOS`/`Win32_BaseBoard`：Maxsun MS-TZZ H610ITX，
物理机）。该板 `VirtualizationFirmwareEnabled` **持续读 False**，但功能面全绿
（vmms/vmcompute 运行、Docker Linux 引擎 29.6.2、`New-VM` 建 win-s4 成功并跑通
三臂批次）——读数假阴、虚拟化可用。

**Round 2（2026-09-07，真故障）**：同样的读数 False，叠加 Docker 引擎 npipe 500、
WSL2 报"请启用虚拟机平台并确保在 BIOS 中启用虚拟化"。表面像"BIOS 被关"，但配置
层全对（`hypervisorlaunchtype=Auto`、VirtualMachinePlatform=Enabled、vmms/
vmcompute 均 Running）→ 排除配置回归。决定性链：`Start-VM` **无报错但 VM 状态
12s 后仍 Off**（异步静默失败）→ 事件日志 `Hyper-V-VMMS-Admin` **20148「无法启动
虚拟机 win-s4，因为虚拟机监控程序未运行」**（伴随 15130），且该 ID 在全日志中
**无更早发生记录** → 本 boot（LastBootUpTime 09-06 10:22）hypervisor 从未加载、
上一 boot 正常 → 判定为"本 boot hypervisor 未随系统启动"（快速启动/休眠恢复为
最常见诱因；boot 时另有 33540「主机处理器统计信息可能无法使用」相印证）。处置
阶梯 = 普通重启 → 彻底断电冷启动（事件文本自带：BIOS 级设置仅 reset 不够）→
BIOS 重存 VT-x。

## 关键查询速查（Win 环境：事件日志 + 物理环境）

| 层 | 查询 | 回答什么 | 本例实测 |
|---|---|---|---|
| L3 权威 | `Get-WinEvent -LogName 'Microsoft-Windows-Hyper-V-VMMS-Admin'`，过滤 **20148/15130** | hypervisor 是否真的没加载（20148=「虚拟机监控程序未运行」） | 20148+15130 出现于每次 Start-VM；全日志无更早先例 → 本 boot 首次真故障 |
| L2 功能 | `Get-VM` + `Start-VM`（等 12s 再查 State） | 决定性探针：VM 起得来 = hypervisor 在 | 静默失败、State 恒 Off |
| L2 配置 | `bcdedit /enum '{current}'` | hypervisor 是否配置为自动启动 | Auto |
| L2 配置 | `Get-WindowsOptionalFeature -Online -FeatureName X`（取 `.State`，locale 无关） | 虚拟化功能是否被卸 | VirtualMachinePlatform / Hyper-V-All = Enabled |
| L2 服务 | `Get-Service vmms,vmcompute` | 管理栈状态——**Running ≠ hypervisor 已加载**（两层） | 均 Running 但 hypervisor 未加载 |
| L2 读数 | CIM `HypervisorPresent` / `VirtualizationFirmwareEnabled` | 仅作参考：本板已知**持续假阴** | False / False |
| L2 物理 | `Win32_BIOS` / `Win32_BaseBoard` | 物理机 vs guest 身份 | Maxsun MS-TZZ H610ITX（物理机） |
| L2 历史 | `LastBootUpTime` + boot 时刻系统日志 | 快速启动/休眠恢复嫌疑 | 09-06 10:22（本 boot 无 hypervisor） |

## 教训与归因纪律

1. **单 API 读数不作 BIOS 结论**——本板 `VirtualizationFirmwareEnabled` 假阴是
   已知怪癖；"读不到"与"没开启"必须用功能面与事件日志区分。
2. **服务 Running ≠ hypervisor 加载**——vmms/vmcompute 是管理栈，hypervisor 是
   内核层，两层可独立失效。
3. **WSL2 的"请在 BIOS 中启用虚拟化"报错是泛化提示**，不构成 BIOS 被关的证据；
   hypervisor 未加载的任何原因都会触发同一文案。
4. **同形异诊要靠时间轴**：同 ID 事件（20148）在全日志的首次出现时间 + 上一 boot
   的正常基线，把"怪癖一直如此"与"本 boot 新故障"分开。
5. 附带互证：诊断脚本初版因无 BOM UTF-8 中文被 PS 5.1 按 GBK 解析炸（正是
   ORZ-WIN-PS-001 教训 1 现场复演），改 locale 无关 API（`Get-WindowsOptionalFeature
   .State`）与全 ASCII 后修复——两案例结论互相印证。

## 回归入口

- `scripts/s4_hv_diag.ps1`（全 ASCII；L1 配置 → L2 运行时/物理身份 → 功能探针
  （`-StartProbe` 选通）→ L3 事件日志 → VERDICT 判定输出）。

## 边界

- 单板单机观察（H610ITX + i5-12400F）；读数怪癖是否他板普遍未验证。
- `HypervisorPlatform` 可选功能 Disabled 与本例无关（另一特性，Hyper-V 不依赖它）。
- `s4_hv_diag.ps1` 的 20148 消息正文在部分时点输出为空（小瑕疵）；结论由事件 ID
  表覆盖，不影响判定。
- 处置阶梯的"冷启动"分支本轮尚未走到（普通重启前的登记），若重启后 20148 复现
  再补记。
