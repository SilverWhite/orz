# 绝对一对一远程形态——需求转录、业界检索与可行性判定（调研底稿 v1.2）（2026-10-19；v1.1＝311 批第二轮裁决录；v1.2＝312 批 BitLocker 触发形态查证）

> **性质**：调研底稿（零源码零跑批；310 批）。**立项待用户裁决**——其中第 5 件
> 「勒索病毒式锁定机器」**调研结论为拒绝原案、给替代方案**（§4），需用户裁决后才可入设计。
> 来源＝用户 2026-10-19 设计意向陈述（「绝对一对一的远程……对疑心病很友好」＋「做一轮检索再做」）。

## §1 需求转录（七件）

1. **绝对一对一设备绑定**：主机侧绑定远程设备的机械信息（硬件码），鉴权未过的设备**机械层直接拦截**其消息。
2. **〔远程使用中〕标识**：远程设备的信息携带「远程使用中」标识；远程与本地窗口的**用户信息注入二存一**（同一时刻只允许一个注入源）。
3. **退出远程显式确认**：退出远程模式需在远程设备上显式确认。
4. **本地外设机械锁死**（用户问「能做吗」）：开启远程时自动锁定本地外接设备——鼠标键盘耳机、全部接口不可用。
5. **风险机制＝勒索式锁机**（用户原案）：远程模式中本地层突然出现新驱动设备/外部接入 ⇒ 触发风险机制，即刻锁死 orz 并显式提醒模型「存在第二使用者、设备不可信」，**用勒索病毒的方式锁定本地主机全部文件和权限**，留记录并提交推送当前进度作备份。
6. **逃生后门**：防远程设备不可逆损坏导致无法取消远程模式——主机侧 orz 进程维持存活，恢复使用需 orz 侧身份鉴权（密码于**安装时**采集，与 ACAF 鉴权一同保管）。
7. **先检索再做**：业界远程方案成熟度调研。

## §2 业界检索结论（2026-10-19）

- **应用层远程桌面成熟**：[RustDesk](https://rustdesk.com/blog/best-free-remote-desktop-software)（开源 TeamViewer 替代，自建 ID/relay 服务器、每连接 ID＋密码会话制）与 [MeshCentral](https://www.computingforgeeks.com/guacamole-vs-rustdesk-vs-meshcentral)（agent 常驻制设备管理平台、web 控制台＋账户/权限分层）为两大自托管代表；形态分野＝[临时支持型 vs 常驻管理型](https://sobrii.io/blog/open-source-remote-desktop)。微软 Quick Assist 属托管黑盒。
- **网络层设备身份成熟**：Tailscale/WireGuard 形态＝**注册制设备身份**（入网签发密钥、设备级 ACL），硬件指纹只作展示不作唯一凭据——裸硬件码（MAC/机器码）会漂移（换网卡、MAC 随机化、重装），业界一律不用作唯一鉴权。
- **Windows 输入锁死硬约束**：[`BlockInput`](https://learn.microsoft.com/en-us/windows/security/application-security/application-control/user-account-control/settings-and-configuration) 需管理员且**效力止于 UAC 安全桌面**（[Microsoft Q&A 实证](https://learn.microsoft.com/en-us/answers/questions/238459/uac-prompt-is-blocked-when-i-use-quick-assist)：锁输入时 UAC 提示不可交互）；物理可接触者可硬重启绕过；UIAccess/UIPI 滥用属[攻击技术](https://projectzero.google/2026/02/windows-administrator-protection.html)不可作为产品面。
- **设备到达检测成熟**：[`WM_DEVICECHANGE`/`DBT_DEVICEARRIVAL`](https://learn.microsoft.com/en-us/windows/win32/devio/detecting-media-insertion-or-removal)＋`RegisterDeviceNotification`（服务形态走 [`SERVICE_CONTROL_DEVICEEVENT`](https://forums.codeguru.com/showthread.php?311458-Please-Help-DBT_DEVNODES_CHANGED-and-RegisterDeviceNotification)；须按 GUID 注册否则只收到 `DBT_DEVNODES_CHANGED`，[SO 常见坑](https://stackoverflow.com/questions/28998625/c-win32-not-receiving-dbt-devicearrival-or-dbt-deviceremovecomplete-on-wm-devi)）；WMI [`Win32_DeviceChangeEvent`](https://timgolden.me.uk/python/win32_how_do_i/detect-device-insertion.html) 更简但噪。
- **勒索式锁机＝杀软必杀**：Defender 以**行为启发**（大批量加密/ACL 变更序列）检测勒索行为、[不区分意图](https://medium.com/@abandedt962/how-microsoft-defender-protects-against-ransomware-95182991c329)；[Controlled Folder Access](https://redriver.com/cybersecurity/windows-defender-ransomware-protection) 直接拦未授权进程的批量加密；[Defender XDR 猎杀查询](https://learn.microsoft.com/en-us/defender-xdr/advanced-hunting-find-ransomware)以 ACL 变更为主信号——自研「勒索式锁机」会被即刻拦截/标黑，发行即自杀。

## §3 可行性逐条判定（七件）

| 件 | 判定 | 依据与形态 |
|---|---|---|
| 1 设备绑定鉴权 | **可做，改形态** | 拒裸硬件码作唯一凭据（漂移误拒）；改**配对注册制**＝远程设备首次入网签发 Ed25519 密钥对、主机侧钉公钥（硬件指纹仅作配对展示位），未注册设备消息在桥入口 fail-closed 拒收（与 0di ext 通道、web 令牌面同族，机械层拦截天然在桥） |
| 2 〔远程使用中〕＋二存一 | **可做** | 桥级会话焦点模型：`remote_active` 状态位＋同一时刻单注入源（本地输入框 or 远程通道），事件面带标识；与 0di SupplementQueue/投递卡基建同族 |
| 3 退出显式确认 | **可做** | 远程端确认票状态机（两段确认，`Modal::Rewind` 同形）；确认不可达时的逃生走第 6 件后门 |
| 4 本地外设锁死 | **部分可行，改形态** | 进程级 `BlockInput` 硬锁不彻底（UAC 安全桌面免疫＋硬重启绕过＋本地逃生密码无处输入）；正确形态＝**锁屏/会话切换**（Workstation Lock 或 RDP 式接管：本地控制台天然失活）＋音频端点禁用仅作体验面非安全面；物理接触面（电源/reset 键）任何软件都锁不住——如实登记为设计边界 |
| 5 勒索式锁机 | **拒绝原案，替代见 §4** | 见 §4 三重否决理由＋替代方案五件 |
| 6 逃生后门 | **可做，现成可扩** | `orz-acaf-provision` 安装时已建 keystore——加 identity secret（Argon2id 派生）同库保管；解锁入口走**安全桌面**（解决「锁输入后无法输密码」矛盾）；须加 rate-limit 防爆破 |
| 7 业界检索 | **已收** | 见 §2 |

## §4 第 5 件：拒绝「勒索病毒式锁机」原案，替代方案（待用户裁决）

**三重否决**：① **杀软必杀**——行为启发式按「批量加密＋ACL 变更」检测、不区分意图
（§2 检索），orz 会被 Defender/EDR 当勒索软件拦截或标黑，发行即自杀；② **触发器高误
报**——USB/驱动到达是日常事件（插 U 盘、手机充电即触发），把全机锁死挂在它上面＝自
毁概率极高；③ **失败模式灾难**——锁机逻辑任何一个 bug＝您自己永久丢整盘数据，自研
锁定不具备勒索软件「-tested-on-thousands-of-victims」的可靠性。

**替代方案（同样「疑心病友好」，零数据毁灭风险）**——新设备到达且远程会话在役时：

1. **orz 机械层即刻 fail-closed**：远程会话立即终止、journal 冻结封口、拒收一切后续
   输入（本地与远程都拒），风险状态位落事件面；
2. **锁屏**（Workstation Lock，可选 logoff）——本地控制台与远程输入面同时失活；
3. **git 兜底自动执行**：工作区＋`.gsa` 存档 commit＋push（用户原案要的「留记录＋提交
   推送备份」原样保留）；
4. **显式提醒模型**：「检测到第二使用者/设备不可信」注入事件面（风险段如实披露）；
5. **解锁唯一路径＝第 6 件 ACAF 密码后门**（安全桌面输入，rate-limited）。

与原案的安全收益对账：原案防的是「第二使用者在锁机前碰文件」——替代案 ①② 在毫秒级
完成 fail-closed＋锁屏，实际窗口不大于原案；原案多出的只有「文件级锁定」这一层，而它
正是杀软必杀与数据毁灭风险的来源。BitLocker 静态加密本就由 OS 承载，不需要 orz 自研。

## §5 推荐架构形态（三选项，待用户裁决）

- **选项 A（推荐）＝orz 做鉴权守卫层，传输借成熟件**：传输/画面用 RustDesk（自建
  relay）或 Tailscale（设备身份＋ACL），orz 负责——设备配对注册、〔远程使用中〕标识
  与二存一注入、风险机制（§4 替代案）、审计与 ACAF 后门。orz 不自研远程协议（自研画面
  流协议＝数月级工程且安全性难追业界）。
- **选项 B＝轻量自研**：orz web 桥扩展为「远程只读＋指令面」形态（无画面流）——移动
  设备远程**用 orz**（对话/审批/状态），不远程**用整台电脑**；工程量最小，与现有
  0di/web 桥基建同构，先落地再演进。
- **选项 C＝全自研远程桌面**：含画面流/输入注入/设备锁——不推荐（§2/§4 依据）。

## §7 第二轮裁决（2026-10-19，用户；311 批录）

1. **路线定向＝优先借鉴成熟设计**（「我们自己踩坑和打磨太费劲」）——选项 A 定向
   （orz 做鉴权守卫层，传输/画面借成熟件）；选项 B（远程指令面）可作为 A 的首个增量
   先落地。选项 C 退役。
2. **「用 Win 安全中心做文件锁定」查证＝拆两半**：事件式锁机**不可行**——Defender
   篡改保护（Tamper Protection）拦截一切程序化修改（`Set-MpPreference` 含 Controlled
   Folder Access），管理员同样被拒、无纯 PowerShell 旁路（须交互 UI 或 Intune/GPO
   预配置）〔检索：[MS Learn Troubleshooting mode](https://learn.microsoft.com/en-us/defender-endpoint/troubleshooting-mode-scenarios)／
   [cloudbrothers 篡改保护边界](https://cloudbrothers.info/current-limits-defender-av-tamper-protection)／
   [Superuser 实证](https://superuser.com/questions/1858090/pausing-windows-tamper-protection-and-defender-not-possible-anymore)〕；
   **安装时基线加固可行且采纳**＝provision 时一次性配 CFA 保护文件夹＋核 BitLocker
   状态，文件层防护由 OS 常驻承载（BitLocker 即真·文件层静态锁定）。
3. **锁形态采纳「外部接口一视同仁全锁＋已知设备白名单」**——用户直觉「全锁比单独锁
   鼠标键盘省事」在 OS 原生机制上成立：**设备安装限制策略**（Device Installation
   Restrictions，注册表 `HKLM\SOFTWARE\Policies\Microsoft\Windows\DeviceInstall\Restrictions`，
   `DenyUnspecified`＋`AllowDeviceIDs`）＝「禁一切新设备安装、白名单例外」恰一开关一表；
   被拦安装落安全日志 **Event 6423**（天然审计面进风险事件）〔检索：[MS Learn Manage Device Installation](https://learn.microsoft.com/en-us/windows/client-management/client-tools/manage-device-installation-with-group-policy)／
   [Superuser 注册表控制](https://superuser.com/questions/1189380/is-there-any-way-to-control-device-installation-restrictions-via-the-registry)／
   [Petri 分层评估](https://petri.com/microsoft-improves-windows-10-device-installation-control-with-new-apply-layered-group-policy-setting)／
   [Event 6423](https://www.ultimatewindowssecurity.com/securitylog/encyclopedia/event.aspx?eventid=6423)〕。
   白名单用**设备实例 ID**（物理件唯一；硬件 ID 同型号共享）——与 ① 配对注册制合流，
   配对时登记用户键盘/鼠标实例 ID。**边界如实记四条**：①新装阻断≠已装设备失活（已插
   非白名单外设需 PnP 枚举 `Disable-PnpDevice` 一把扫，或用新版「apply to already
   installed」）；②核心类豁免（网络适配器锁了远程会话自断／系统盘／显示）——「一视同
   仁」在外部设备类范围内成立；③改策略需管理员；④笔记本内置键盘不可拔＝天然逃生面
   的一部分。锁耳机＝音频端点同表管理零额外工作量。
4. **鉴权根＝ACAF 绑定密码（否 Windows PIN）**——用户裁「PIN 不是隐私物、不足为凭」
   ＝同意：Hello PIN 系设备本地 TPM 绑定、设备属主可重置，不作此场景信任根；安装时
   采集独立密码经 Argon2id 派生入 ACAF keystore（与签名密钥同库同备份纪律），解锁
   远程锁／设备变更必须过它＋rate-limit。

## §8 BitLocker 风险触发形态查证（v1.2；2026-10-19 用户问询「风险时能否直接触发 BitLocker——反正恢复后正常进系统」）

- **OS 卷运行中不可锁（硬边界，先说死）**：BitLocker＝静态加密——系统运行时卷密钥已在
  内存、卷已挂载，运行期访问控制交给 NTFS ACL；**没有任何受支持 API 能重新封印运行中
  的 OS 卷**；`manage-bde -lock` 仅适用**非系统固定数据卷**（存在打开句柄即失败）。
  另：BitLocker 锁的前提是**早已加密**——风险时无法现场加密（全盘加密小时级），一切
  依赖装机基线（311 裁决 2 的扩展）。
- **软锁＝数据卷即时上锁（可行，orz 存活）**：`manage-bde -lock <数据卷>`——前提该卷
  已启用 BitLocker（装机基线核验）；远程输 ACAF 密码即可解锁恢复；失败面＝有打开句柄
  （如实记）。适合「敏感数据盘即时锁＋会话内继续」。
- **硬锁＝重启进固件 PIN 门（可行，核弹档）**：装机时预配 **TPM+PIN 启动保护器**
  （PIN＝ACAF 密码）＋恢复密钥入 keystore；风险触发执行重启 ⇒ 机器停在固件 BitLocker
  PIN 门——无 PIN 谁也进不了系统（断电/冷启动同在，比锁屏强一级）；输对 PIN 正常进
  系统＝用户预期「恢复后正常进系统」成立。**代价（与 ⑥ 后门直接冲突，须显式裁决才可
  作默认）**：重启即 orz 进程终止＋远程会话终止——恢复必须有人**物理在机前**输 PIN，
  远程用户同样被锁在外面。
- **结论（风险响应分两档）**：默认软档＝替代五件套（fail-closed＋锁屏＋git 推送＋提醒
  ＋后门解锁）＋数据卷软锁；硬档＝BitLocker 重启 PIN 门（配置期显式启用，接受 orz 终止
  与物理恢复代价）。PIN 在固件 pre-boot 界面由内置键盘输入（与设备白名单机制兼容）。

## §6 关键词

绝对一对一远程、设备绑定、配对注册制、硬件码漂移、〔远程使用中〕、二存一注入、退出
显式确认、BlockInput 约束、UAC 安全桌面、锁屏形态、WM_DEVICECHANGE、DBT_DEVICEARRIVAL、
勒索式锁机拒绝、行为启发检测、替代方案五件、ACAF 后门、Argon2id、RustDesk、MeshCentral、
Tailscale、选项 A/B/C、立项待裁决。
