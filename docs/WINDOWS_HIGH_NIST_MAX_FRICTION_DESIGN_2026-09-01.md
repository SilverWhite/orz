# Windows 原生 HIGH-NIST 最大摩擦评测设计（BoundaryBench 模式，路线 B）

- 状态：`partial`（2026-09-01 设计定稿；② S1/S2 完成 + 全面审查处理完成，
  S3/S4 实机复验待放行）
- 关联：[`BoundaryBench`](https://github.com/boundary-bench/boundary-bench)（论文
  arXiv:2608.02670）；[`FUS-BENCHMARK-FULL-EXEC`](BENCHMARK_FULL_EXEC_DESIGN_2026-08-18.md)；
  [`GAK-SBX-001`](GAK_SBX_001_WINDOWS_NATIVE_SANDBOX_AUDIT_2026-07-27.md)；
  [`MECHANICAL_LAYER_MATH_CALCULUS_DESIGN`](MECHANICAL_LAYER_MATH_CALCULUS_DESIGN_2026-08-30.md)；
  [`ADR-0010 §6/§11.7`](../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)
- 实施路由：设计轮登记 [BACKLOG 0l](BACKLOG_AND_PRIORITIES.md) / [TODO P0-0l](../TODO.md)
  （设计轮不动计数）

## 1. 背景与问题

TB2.1 官方跑分与失败复验（2026-08-31，official-r2-failures-recheck-10t，
10/10 完成）呈现单一失败画像：7/10 墙钟超时、0 次 policy_denial、检索降级每
run 恰 1 次、全部门拒绝"按设计"、模型持续推进至墙钟。机械层（LIF 的
err/deny/stall/slow 通道、T̂、stuck）全绿——但这不构成"机械层无缺漏"的证据：
机械层是被事件流喂养的，而当前评测环境的反馈分布太窄（bash exit code + 超时），
通道在平凡值上饱和，"健康"与"失明"在数据上不可区分。

BoundaryBench（2026-08 发布）给出放大摩擦的现成模型：把同一 Terminal-Bench
任务集放到 OS 级强制加固策略下（N×F×P 格：Network × Filesystem × Privilege），
拒绝以原生 OS 错误形态出现（EROFS/EPERM/连接被拒，无 shim），测量能力损失并
归因。论文核心发现对 orz 高度相关：策略挡路时 agent 不早停——59% 失败为超时、
37% 为错误解，即"磨时间"正是主流失败形态。

2026-09-01 用户裁决（路线 B）：orz 是 Windows 原生框架，Linux musl 容器跑分
只覆盖移植面；应按 BoundaryBench 模式设计 **Windows 原生 HIGH-NIST 最大摩擦
评测**，让原生代码路径（CredReadW、ConPTY、DPAPI/ACAF、Program Files 浏览器
发现、Job Object、PowerShell 语义）与最严策略同时受压；Linux arm 值得做，
但作为方法学干跑，不是主摩擦面。

## 2. 目标与不变量

- **摩擦放大**：把反馈分布从"bash 错误 + 超时"扩展为全谱（拒绝/锁/延迟/静默写/
  假成功/路径语义/脚本执行），使机械层缺漏第一次可观测。
- **原生强制、无 shim**：策略全部由 Windows 原生机制执行（ACL/完整性级别/
  AppContainer/Job Object/WFP/Firewall）；不新增 orz 内测试专用权限 shim。
  orz 内部权限门（Benchmark 两轴）保持原语义放行，OS 层拒绝成为新的反馈类别。
- **BoundaryBench 结构不变**：control / non-admin / high-nist 三臂格；每臂同
  harness、同模型、同任务；不可用任务记账（policy applicability）；阻塞分析
  （policy blockage）与缺口判据可证伪。
- **与现有评测面分离**：本设计不改变 TB2.1 官方口径；Windows arm 使用独立
  任务载体与 verifier，产出为机械层缺口清单，不是 leaderboard 数字。

## 3. 总体设计：N×F×P 格 → Win32 原语映射

| 轴 | BoundaryBench 语义 | Windows 原生强制原语 | 直接受压的 orz 组件 |
|---|---|---|---|
| Privilege·non-admin | 普通用户、禁 sudo | 受限 token（去 Administrators、deny-only SID）+ `EnableLUA` 收紧 + 禁 runas/COM 提权 + 移除 SeDebug/SeBackup/SeRestore/SeTakeOwnership/SeLoadDriver/SeCreateSymbolicLinkPrivilege + **TokenVirtualizationAllowed=0**（防静默写重定向） | `permission.rs` Benchmark 两轴放行后的全部命令；ACAF（DPAPI）签名链 |
| Privilege·no-escalation | no_new_privs + 能力剥离 | 低完整性级别（LOW IL）+ UIPI + WDAC/AppLocker（白名单二进制）+ HKLM\SYSTEM 只读（禁服务/驱动/计划任务/BITS） | 终端后端、web 工具、local_browser |
| Privilege·capability drop | 能力剥离 | AppContainer（沿用 GAK-SBX-001 profile/探针模式） | 全部外部面 |
| Filesystem·readonly-os | 只读 OS、仅工作区可写 | ACL deny-write C:\Windows / Program Files / Program Files (x86) / ProgramData / 盘符根；HKLM、HKCR 只读；可选 Controlled Folder Access | 写放置门、run_tests、search_replace、编码门 |
| Filesystem·freeze-home | 冻结 home | %USERPROFILE%/%APPDATA%/%LOCALAPPDATA% deny-write；HKCU 冻结；**%TEMP% 重定向到 {workspace}\.tmp**（否则 orz 自身先崩） | 凭据读取（CredReadW 读不受影响）、pip --user、浏览器缓存、pytest 缓存 |
| Network·restrict-egress | 每任务白名单、挡元数据/私网 | Windows Firewall 出站规则（按任务钉 IP）+ 阻断 169.254.169.254 / link-local / RFC1918 / 组播；WFP DNS 过滤或 hosts 钉 IP；**DeepSeek 模型端点恒放行** | web_search/web_fetch、检索机械降级链、DNS 缓存 |

继承的已知残余：GAK-SBX-001 已登记 **AppContainer 不挡原始 TCP 出站**
（`network_connect_blocked=false`），必须由 host firewall/WFP 在沙箱边界兜底；
不得把"AppContainer 配置正确"写成"网络隔离完成"。

## 4. 承载方式与任务载体

TB2.1 的 89 题语义是 Linux（apt/gcc/ELF/bash），Windows HIGH-NIST 不能直接复用。
三个承载选项：

1. **Windows 容器（servercore）**：最接近容器化评测，但 Harbor 任务镜像是
   Linux、TB 语义不符，Windows 容器绑宿主内核版本。不推荐做主载。
2. **硬化 Windows VM（推荐主载）**：一次性模板 + 加固脚本（PowerShell/Win32，
   等价 BoundaryBench 的 daytona scripts）+ 快照回滚。任务集 = 摩擦探针 +
   少量真实 Windows 任务。orz 全原生路径（CredReadW、ConPTY、DPAPI、Program
   Files 浏览器发现、Job Object）在真实 Windows 上受压——即"最摩擦"面。
3. **Linux arm 干跑**：照 BoundaryBench 原样移植（adapted verifier、
   not_applicable 记账、blockage 分析），验证分析管线正确性，再搬到 Windows。

定案：**选项 2 为主载，选项 3 为方法学干跑**。选项 3 的产出是管线正确性，
不是分数；选项 2 的产出才是机械层缺口清单。

## 5. 最大摩擦点清单（Windows 特有、Linux 测不到）

| # | 摩擦源 | 表现形态 | 机械层应捕获的证据 | 预期暴露的缺口 |
|---|---|---|---|---|
| 1 | PowerShell 原生命令失败但 exit code=0 / 错误文本与 exit 码分裂 | 命令"成功"但没做事 | err 通道漏报 | 错误分类把 OS 层失败当成功 |
| 2 | UAC 文件/注册表虚拟化静默写（若 TokenVirtualizationAllowed=1） | 写"成功"但落 VirtualStore，verifier 看不到 | 假成功零反馈 | 机械审计抓不到写入路径与结果不一致 |
| 3 | 文件锁/共享冲突（ERROR_SHARING_VIOLATION 32、LOCK_VIOLATION 33） | 构建工具持文件，删除/改名失败 | slow/err、同 failure_target 重试计数 | 重试无上限、连打同一失败目标 |
| 4 | SeCreateSymbolicLinkPrivilege 缺失 | ln -s 类操作失败 | err（OS 错误不会走 deny 通道） | deny 计数恒 0 下的压力盲区 |
| 5 | Defender 首扫/EDR 延迟、写后读失败、SmartScreen/MotW 阻断 | 新二进制墙钟暴涨、下载后无法执行 | slow 通道（wall_ms>60s）、T̂ 失真 | slow 被误判成 stall → LIF 误 fire 或错失 |
| 6 | PowerShell 执行策略/脚本签名、ConPTY 输出 drain | .ps1 被 Restricted/AllSigned 拦 | 终端反馈面 | 分层超时/中间回报在 PS 后端的接线 |
| 7 | MAX_PATH、大小写不敏感冲突、UNC/junction、CRLF | 长路径/改名失败、路径歧义 | err10/succ10 | 路径规范化缺环（编码门之外的新面） |
| 8 | CredReadW 在受限 token/AppContainer 下、DPAPI profile 锁 | 凭据读失败 → fail-closed | 启动期错误 | Windows 凭据通道在受限面下的回归 |
| 9 | Chromium 在 LOW IL + 能力剥离下 sandbox 起不来 | local_browser launch 失败 | 降级 transition | 强制触发浏览器→framework_fallback 链，且触发点与 Linux 不同 |
| 10 | WinHTTP/WinINET/raw socket 差异、系统代理、DNS 缓存 | web_fetch/web_search 行为漂移 | 检索事件面 | 网络层错误文本分类缺失 |
| 11 | Job Object 嵌套（宿主 CI 已在 job 内 → AssignProcess 失败）、后台任务/取消 | 进程树 containment 失败 | fail-closed 报告 | `JobObjectSupervisor` 已登记边界的实机验证 |

注：#9 的 local_browser profile 已收进工作区（`.gsa/chrome-profile-*`，
ADR-0009），冻结 home 不会先杀浏览器；真实摩擦是"浏览器存在但起不来"，
比 Linux 容器"没有浏览器"的降级路径更接近真实故障。

## 6. 任务集与探针

- **摩擦探针集**（每轴 3–5 个原子任务，参照 GAK-SBX-001 7 项检查 +
  BoundaryBench enforcement-probe）：写 HKLM、写 Program Files、建 symlink、
  pip --user、克隆非白名单主机、写 %TEMP%、启动服务、跑未签名 .ps1、下载并
  执行 exe（MotW）、长构建（Defender 延迟）。每个探针有"策略下必须失败 /
  必须成功"的断言。
- **真实任务子集**：移植少量 OS 无关的 TB 题逻辑（regex-log、
  log-summary-date-ranges 等纯文件/文本任务），或从内部任务池抽 Windows 原生题。
- **三臂**：control / non-admin / high-nist，同 harness、同模型、k=1 起步，
  与 BoundaryBench 格同构，delta 可归因。
- **硬化验证**：每臂启动前跑 enforcement-probe（等效 GAK-SBX-001 的 7 项检查
  扩展），确认墙真实生效（root 写不进 HKLM/System32、外网连接被拒、setuid/
  提权失效），再放行 agent。

## 7. 缺口判据（可证伪断言，证据=journal 事件 + LIF 读数）

1. **deny/err 分类口径**：OS 级 ACCESS_DENIED 一族（0x5、0x80070005、
   0x80004005）应归 deny 或 err 的哪个通道须有确定口径；同一错误类别在事件面
   分裂或漏喂 = taxonomy 缺口。
2. **err 升压**：拒绝风暴下 u_err 应快速升压（近 3 分钟窗 ≥2 错误）；若
   10 连拒 u_err 仍低 = 通道接线缺口。
3. **slow/stall 区分**：Defender 延迟（wall_ms>60s）应走 slow 通道；若 300s+
   命令无中间回报 = 分层超时缺口（对照 foreground_block_budget 设计）。
4. **LIF fire**：应在"重复同一失败目标"模式上触发；检查 failure_target 相同
   且无方法变更的轮数。
5. **降级链**：浏览器→framework_fallback 应在 Windows 路径真实触发、恰一次、
   无循环。
6. **假成功检测**：虚拟化静默写 → verifier 结果与事件面写入路径不一致 →
   机械审计应能抓到。

## 8. 预期缺口假设（按概率排序）

1. PowerShell exit code 语义导致 err 漏报（原生命令失败 exit 0）。
2. 虚拟化静默写 → 假成功零反馈（最阴，机械审计盲区）。
3. ACCESS_DENIED 文本未进 deny 分类 → u_err 升压但 deny 计数恒 0。
4. Defender 延迟让 slow 误判为 stall → LIF 误 fire 或 T̂ 失真。
5. 文件锁重试无上限 → 同 failure_target 连打。
6. MotW/SmartScreen 阻断 → "文件不存在/无法运行"类模糊错误。
7. AppContainer 网络残余（已知限制）→ 白名单外连接仍通。
8. Windows 浏览器降级触发链与 Linux 不同 → transition 次数或时机异常。

## 9. 与现有资产衔接

- `assurance/windows_sandbox.py`（GAK-SBX-001）：AppContainer + Job Object +
  受限 token 探针、profile/observation schema、独立 verifier——从"探针"扩展为
  "运行环境"（加固态 spawn orz 及其命令树）。
- `orz-assurance/src/sandbox/job_object.rs`：kill-on-close Job Object 监督器，
  AssignProcess 失败 fail-closed 边界。
- `orz-host/src/permission.rs`：Benchmark 两轴放行语义保持不变（OS 层拒绝成为
  新反馈类别）。
- `orz-host/src/local_browser/discovery.rs`：Windows Program Files 候选 +
  工作区 profile（ADR-0009）。
- `orz-loop/src/gateway/credentials.rs`：Windows CredReadW 通道在受限面下的回归。
- `MECHANICAL_LAYER_MATH_CALCULUS_DESIGN`：err/deny/stall/slow 通道口径与
  §7 判据逐项对接。
- ADR-0010 §6/§11.7：Windows 观察面清单（Job Object/ConPTY、PS 5.1/7、CMD、
  Git Bash/MSYS、盘符/UNC/长路径/Unicode/CRLF/文件占用、Credential Manager、
  ACL/AppContainer/网络隔离、EDR、Docker Desktop/WSL）。

## 10. 验证计划（放行后执行）

1. **Linux arm 干跑**：BoundaryBench 模式移植到现有 Harbor 管线，跑通
   policy blockage / adapted verifier / not_applicable 记账（方法学验证）。
   **已闭环（2026-09-01）**：三臂 control/non-root/high-nist 12/12
   reward=1.0、0 异常；enforcement-probe 三臂先验墙全过；OS 错误通道记账
   non-root epErm×1 / high-nist eroFS×1（策略墙以原生 OS 错误形态真实呈现，
   合法工作区路径零误伤、真实任务三臂同分）；干跑期修复 6 项（Harbor
   docker_image 忽略 Dockerfile、指令引用不存在文件、non-root 缺 agent 用户、
   OrzStrict stdout None、cap_drop 下 apt/ACAF chown 需依赖预装 + CHOWN 桥接、
   分离 verifier workdir=/tests 对齐），详见 `_linux_arm_dryrun/README.md`。
   结论：管线正确性成立；egress allowlist 本机不可用（记 reachable 基线，
   restrict-egress 留 Windows 主载）；Windows 原生摩擦仍按 §7 判据在硬化 VM
   上逐项核对。
2. **Windows 加固脚本 + enforcement-probe**：加固脚本（PS/Win32）落模板；
   `windows_sandbox.py` 扩展为运行环境；7 项检查扩展为每轴断言。
   **S1/S2 完成（2026-09-01）**：加固脚本落 `_windows_high_nist/hardening/
   apply_hardening.ps1`（control/non-admin/high-nist 三臂、幂等、-Revert
   可撤销、日志）；enforcement-probe 落 `_windows_high_nist/policy/
   enforcement_probe.ps1`（GAK-SBX-001 7 项检查扩展为每轴断言集，PASS/FAIL
   行 + JSON，fail-closed）；`windows_sandbox.py` 新增
   `run_windows_native_sandbox()` 运行环境（受限 token：Administrators 禁用/
   deny-only + 6 特权移除 + TokenVirtualizationAllowed=0；high-nist 另加
   LOW IL + AppContainer 空能力 + Job kill-on-close + `%TEMP%` 重定向到
   `{workspace}\.tmp` + egress wall），输出
   `windows-native-sandbox-run-v0.1.schema.json` + 独立 verifier +
   `scripts/run_windows_native_sandbox_command.py` CLI；S2 全绿
   （assurance 测试 46 passed 含既有）。S3/S4（硬化 VM 模板实机加固 + 三臂
   enforcement-probe 实跑）待放行。**S1/S2 全面审查处理（2026-09-01）**：
   egress allowlist 规则逐条校验 netsh 返回码 + verifier 收紧（allowlist
   意图不再等于墙已建）；runner 对 run observation noncompliant 改硬失败
   （fail-closed）；HKCU 冻结改为加载 RunUser NTUSER.DAT 设置 hive ACL
   （原实现误加到管理员 hive）；profile 目录预建 + `AppData\Local\Packages`
   显式豁免（AppContainer profile 运行期写面）；Job Object 补
   active-process limit（兑现 profile pids_limit）；AppContainer profile
   删除结果如实记录；deny-write ACL 补 AD（建子目录）；`-Revert` 按规则
   清单清理防火墙并恢复 hosts；DeepSeek 恒放行改为显式 `-DeepSeekIp` 或
   加固时有网解析，否则 FAIL（删除“任务启动时补钉”承诺）；探针特权枚举
   改 P/Invoke（locale 无关）并自动避开 allowlist 探针 IP；受限 token
   LUID 数组压缩。**S3 重建 + S4 本机冒烟闭环（2026-09-01）**：Windows
   x86_64 三件套重建（orz f0eeb524，12m15s；orz.exe 50,291,712 B /
   orz-signer.exe 6,739,456 B / orz-acaf-provision.exe 6,642,176 B）；
   三件套启动行为与守卫符号（dep_graph / blackboard_read /
   工具→实体变更: / deps total=reads:）核验通过；sandbox control 臂
   端到端冒烟 compliant；**冒烟修复 3 项**——
   `InitializeProcThreadAttributeList` 查询大小误判（Win32 查询期预期
   返回 FALSE+122，原实现当致命错误，导致全部臂无法 spawn）、管道 drain
   对 `c_void_p` 句柄误用 `int()`（句柄值被当 bytes 解析）、CLI
   `--command` 用 `nargs='+'` 拒绝 `-` 前缀子命令参数（改 REMAINDER 且
   须置于末位，runner 同步调整参数顺序）；S2 测试 35 passed（+2 回归）、
   assurance 全量 1624 passed、check_repository valid。边界：control
   臂基线须在未加固环境采集；RunUser 需至少
   登录一次（NTUSER.DAT）方可 hive 级冻结 HKCU。
3. **摩擦探针集 control 臂基线**：k=1，确认探针在无策略下可达、verifier 断言
   成立。
4. **high-nist 小批**：按 §7 判据逐项核对，产出首批缺口清单；对照 §8 假设排序。
5. **全量 + 缺口登记**：全任务跑完后按 BACKLOG 纪律登记缺口（GAP-* 或
   TODO 项），并回写本设计的判据口径。

## 11. 登记

- CLI_PROJECT_INDEX：AUTH-WINDOWS-HIGH-NIST-MAX-FRICTION（`partial`；
  2026-09-01）；BACKLOG 0l；TODO P0-0l。
- 设计轮不动计数；实施放行后按既有 S1-S4 纪律推进。
