# 写入管控（0bw）S1 设计档：机械层收窄锁死＋命令审查留痕＋补偿自检

> **状态**：`design v1.0`（2026-09-26 定稿；S1 交付）。**权威链**：本档为 0bw 的设计权威；
> 上游口径＝[`083 全面审查档 §10.2–§10.6`](audits/083_FULL_PROJECT_REVIEW_2026-09-26.md)（四轮收敛定案）。
> **执行形态**：S1 设计档先行＋Windows CFA audit 探针；非狗粮轮；不提交/不推送/不重建（用户令 2026-09-26）。
> **本档落码面**（S2 v1，本轮）：deny 单一源表＋工具面落地（`search_replace`）＋L2 命令面（`run_terminal_cmd`）＋
> 载体自保护；**未落码面**：L3（Linux Landlock／Windows CFA enforce）与 L4 全量补偿（分别留待真机/Linux 批与后续批）。
>
> **三条用户底线（硬约束）**：① **读写对齐**（读面已有机械沙箱，写面补齐机械门）；② **危险命令机械审查＋系统盘核心删除/修改锁死**；
> ③ **模型无法破坏当前环境载体**（hard）。**边界裁决**：**allowlist 不做**（写层只有「安全」一义，不做限制性可写根——2026-09-26 用户裁决）；
> **安全定位＝高阻力＋强审计、非绝对保证**（对外不做绝对安全性声明；三档措辞：保证／阻力／审计）。

## §0 一句话设计

机械层持有一张**单一源 deny 常量表**（系统核心路径＋载体自保护集），在三处落地——工具面写入路径（硬拒）、
`run_terminal_cmd` 命令面（best-effort 规则闸＋留痕）、进程面（Linux Landlock／Windows CFA 探针，随批推进）；
写层**不做可写根 allowlist**，允许面保持真机直通，锁死面之外的行为与现状一致。

## §1 形态总纲（L0–L4 收窄版）

§10.5 定稿形态 = **L1 的 deny 半边（收窄式硬锁定）＋L2（命令审查）＋L3（进程级，分平台）＋L4（补偿）**；
allowlist 半边作废。本档沿用 L0–L4 编号并收窄：

| 层 | 内容 | 本轮状态 |
|---|---|---|
| **L0 目标规范化** | 写目标与命令内路径统一经 canonical/词法归一化后比对；歧义形态按 §2.2 处理 | 落码 |
| **L1 锁死面（deny 单一源）** | deny 常量表：Win/Linux 系统核心＋载体自保护（§2.1）；三处同一表落地（§3） | 落码（工具面＋命令面）；进程面随 L3 |
| **L2 命令面机械审查** | `run_terminal_cmd` 规则库：block／warn／allow 三分类；best-effort 风险闸＋留痕（§4） | 落码（v1 规则集＋fixture） |
| **L3 进程级强制** | Linux＝Landlock（spawn 时顶层枚举 allow、核心集除外、读不设限）；Windows＝CFA 探针先行｜候选 B＝AppContainer/受限令牌 | 设计＋探针（本机提权/狗粮依赖，见 §5） |
| **L4 补偿与自检** | 编辑面回退窗口（既有，0bm⑦）＋git 检查点/journal undo（设计）＋载体完整性自检（manifest 机制复用） | 部分落码（见 §6） |

**与真机直通哲学的关系**：不构成沙箱化——可写面默认仍是整个真机环境，收紧的只有「系统核心」与「载体」两块，
与读面沙箱（workspace 收窄）方向一致、边界更窄；读写不对称由本档在写入面上补齐（读有门、写也有门）。

## §2 deny 单一源表（v1）

**单一源**＝`orz-tools/src/types/write_control.rs` 内编译常量（代码即配置，见 §2.3）。所有落地点引用同一模块，
不得各自复制表项；表项含义经本档声明，改动须同步本档（判据钉见 §8）。

### 2.1 表内容（v1）

**A. 系统核心（Windows）**——写（创建/修改/删除）恒拒：

| # | 表项 | 说明 |
|---|---|---|
| W1 | `%SystemRoot%`（默认 `C:\Windows`） | 含 System32、注册表蜂巢文件（`System32\config`）等 |
| W2 | `%ProgramFiles%`（默认 `C:\Program Files`） | 含 (x86) 变体 W3 |
| W3 | `%ProgramFiles(x86)%`（默认 `C:\Program Files (x86)`） | |
| W4 | `%ProgramData%`（默认 `C:\ProgramData`） | 机器级组件数据面 |

**B. 系统核心（Linux）**——写恒拒：`/boot`、`/etc`、`/usr`、`/lib`、`/lib32`、`/lib64`、`/libx32`、
`/bin`、`/sbin`、`/dev`、`/proc`、`/sys`。未列入面（`/home`、`/root`、`/tmp`、`/var`、`/opt`、`/srv`、
`/mnt`、`/media`、`/run`）不在锁死面内（真机直通保持；扩展须走本档修订）。

**C. 载体自保护**——任何形态恒拒：

| # | 表项 | 说明 |
|---|---|---|
| C1 | 会话卷域 `{cwd}/.gsa`（含 journal、archive、rollback、快照、会话状态） | 沿既有 `resources.rs` 窗口契约语义（工具面已拒；命令面经 L2 覆盖） |
| C2 | orz 安装目录（`current_exe()` 父目录） | 见下方降级规则 |
| C3 | 载体三件套文件（`orz`／`orz-signer`／`orz-acaf-provision`，含 `.exe`）与 `grok-home/` 子树 | 降级场景下的文件级保护面 |
| C4 | 本管控配置自身 | v1＝编译内置（即载体二进制本体，由 C2/C3 覆盖）；未来若引入配置文件，必须并入本表并以 manifest 校验兜底（§2.3） |

**C2 降级规则（勘定发现）**：当 `install_dir` ⊆ `cwd`（工作区与安装目录重合，如源码树内 `target\release\` 部署）时，
对安装目录的**整树拒写**会封锁工作区——此时**降级为文件级**（C3：三件套＋`grok-home`），整树面保留给
`install_dir` 在 `cwd` 之外的部署（主流生产形态）。降级面覆盖 DLL 侧植等残余风险**不保证**（登记为已知边界）。

**与读面复核**：`.gsa` 窗口契约（D3 两窗口：`session/terminal/*.log`、`run_tests_output.txt`）**仅属读面**，
写面在 v1 即全域拒——读写语义差保持（写面更窄）。

### 2.2 匹配语义（L0）

- **归一化链**：模型给路径 → `resolve_model_path`（`~` 展开＋display-cwd 重写，读面同族）→ 目标存在则 `canonicalize`；
  不存在则**近祖先 canonical**（上溯至首个存在祖先做 canonical，其余段词法拼接）。
- **比对**：`candidate_is_under`（读面同族实现；Windows 含字节级 ASCII 大小写折叠，FR-N01 防切片 panic 语义）＋
  `orz_paths::normalize_lexically`（`..` 归一化）。
- **前缀剥除**：比对前剥除 `\\?\` 与 `\\.\` 设备前缀（防谓词前缀绕过）；UNC（`\\host\…`）不在 v1 表内、不拒（登记边界）。
- **歧义形态**：8.3 短名／subst／junction 的**存在面**由 canonical 兜住；**未存在面**的上述形态**不保证**拦截（已知边界，§9）。
- **fail-closed 方向**：命中即拒；表项解析失败（env 缺失）回退字面默认（`C:\Windows`／`/etc` 等）——**绝不因解析失败放行**。
- 例：写 `C:\Windows\Temp\x` → 拒（W1）；`\\?\C:\Windows\Temp\x` → 剥前缀后拒；`<cwd>\..\Windows\x`（存在）→ canonical 拒；
  写 `<cwd>\.gsa\x` → 拒（C1）；写 `<cwd>\..\proj2\x` → 允许（非锁死面）。

### 2.3 管控配置决策（裁决 D3，记录）

- v1＝**编译内置常量**（本模块）：**无运行时配置文件、无 escape hatch、无模型可达的关闭开关**。
- 理由：(a) 最小活动件——配置文件自身须并入保护面并解决完整性与不可篡改，v1 不引入该攻击面；
  (b) 「管控配置自身→ v1 ＝载体二进制本体」，由 C2/C3 与 §6 载体完整性自检覆盖；
  (c) 与「allowlist 不做」同向——不给可疑形态留外带通道。
- 变更路径：表项变更＝代码变更（评审＋本档同步＋§8 判据钉），模型面不可达。

## §3 三落地点（deny 单一源的三处消费）

### 3.1 工具面：`search_replace`（写工具主入口）

- **落点**：`orz-tools/src/implementations/grok_build/search_replace/mod.rs` 写前检查区（原 :232-249 `.gsa` 域拒）
  **收敛为单点调用** `types::write_control::check_write_target(...)`；`.gsa` 域判定**委派既有 `resources.rs` 窗口函数**
  （语义不重写、文案保持近似）。三写点（经典 :570／新建 :730／锚点 :1253）共用同一前置检查（现有结构不变）。
- **拒绝形态**：`SearchReplaceOutput::InvalidInput`，机械文案（含 rule 与目标路径），不写不建不触碰目标文件。
- **读对齐注记**：读面 workspace 沙箱（`is_path_allowed_for_read`）不动。读写对齐＝**机械面对称补齐**
  （读有沙箱、写有锁死面＋命令审查），不做同构；可写面仍广于可读面（真机直通哲学，设计声明承担）。
- **已知旁路**：`run_terminal_cmd` 直通 shell（如 `Set-Content C:\Windows\x`）不走工具面——由 3.2 命令面 best-effort 覆盖。

### 3.2 命令面：`run_terminal_cmd`（L2 命令审查＋留痕）

- **落点**：`orz-tools/src/implementations/grok_build/bash/mod.rs` run() 既有机械验证链之后（自匹配 pkill 拒之后、
  `--- Prefix ---` 之前）插入 `types::exec_policy::review_command(cwd, &input.command)`：
  - `Block` → `ToolError` 机械拒绝文案（**命令不执行**；kind＝PermissionDenied 形态，实测接线按实现批定）;
  - `Warn` → **执行照常**＋结果头部附 `[写入管控·提示]` 行（沿用资源软提示的头部行先例）；
  - `Allow` → 零改动。
- **为什么在工具内**：`run_terminal_cmd` 是命令执行唯一入口（工具面单一源）；工具内拦截对全部调用方生效；
  文案随 tool 结果自然进 journal。
- **留痕形态（裁决 D5）**：v1＝block 拒绝文案／warn 提示行随 tool 调用与结果入 journal（既有记录面，零新事件类型）。
  专用事件族（schema＋verifier＋fixture＋Python 镜像全链）列为后续扩展——**不做半程 schema 扩张**。

### 3.3 进程面：L3（平台分叉）

- **Linux＝Landlock（设计注记）**：形态＝spawn 时（pre_exec／同型接线）枚举 `/` 顶层目录，**表 B 核心集不授权**、
  其余逐项 allow 写；**读不设限**；与既有 seccomp 网络过滤同型接线（先例：`computer/local/terminal.rs:3549`、
  `orz-sandbox/src/child_net.rs`）。**本轮仅注记不落码**（本机无 Linux 实测面；避免不可验证的 unsafe 代码——先红后绿纪律），
  排期随 Linux 批；实施批决策点＝复用 `orz-sandbox`（nono 引擎，未接生产态保持不动）或直写 syscall。
  **2026-09-27 复审增补（落码定案）**：① **顶层 symlink 一律不授权**（`ln -s /etc /w` 两步旁路；merged-usr 的
  `/bin→/usr/bin` 等本就在表 B 内被名字排除；非核心 symlink 丢授权＝默认拒＝fail-closed）＋ `O_NOFOLLOW` 打开纵深；
  ② **装挂失败永不 fail spawn**（三 fail-open 分支：内核不支持／枚举失败 ⇒ 不装 pre_exec、warn 一次；子进程内装挂
  失败 ⇒ `write(2)` 直写 stderr 一行提示〔async-signal-safe〕后照常 exec）；③ **架构门**：统一 syscall 号仅
  x86_64／aarch64 成立，其余架构 prepare 恒 `None`（错号探测可能命中无关 syscall，不得尝试）；④ **ABI v1 内核**
  rename/link 不受约束（REFER v2 才有）——见 §9。
- **Windows＝CFA 探针先行（§5）**；候选 B＝AppContainer／受限令牌（v2 排期项，不入 v1）。
- **现状勘定**：`orz-sandbox`（nono/Landlock/Seatbelt 引擎、deny glob、hook_write_deny）存在但**未接线生产**
  （orz-host 依赖注明 intentionally not declared）——本批**不改其接线**，避免牵动未启用面。

### 3.4 undo 面消费点（2026-09-27 复审增补；deny 单一源第四消费）

`orz rollback restore`（0bw④ undo CLI）是**写路径**且模型可经命令面调用（命令文本不含 L2 动词表词汇，
命令面审查不可达），故目标域定死如下（实现在 `orz-host::rollback_maintenance`）：

- **目标必须归一化后仍在 cwd 之内**：反斜杠归一＋词法走组件；绝对路径、盘符形态（`C:`/`C:x`）、`..` 越界恒拒。
- **C1 在归一后判定**：`./.gsa/…`、`x/../.gsa/…`、`.GSA` 大小写变体（Windows 路径大小写不敏感）均不可绕。
- **复用 `types::write_control::check_write_target`**——回退写面与工具面同表（系统核心 A/B＋载体 C2/C3 命中即拒）。
- **写回原子化**：同目录临时文件＋rename，中断不留截断目标。
- CLI 参数面：未知旗标与多余位置参数显式报错（exit 2），不静默过滤。

## §4 L2 规则库 v1（命令面机械审查；best-effort 风险闸）

**形态**：规则＝（id／分类／匹配／依据）。匹配机制＝「短语包含」＋「词元精确」两法（先经大小写折叠、空白折叠、
`unicode_confusables::normalize_confusables` 文本归一——既有保守表，**仅作归一辅助，不承载安全语义**）；
**不解析完整 shell 语法**（best-effort，声明边界）。命令内路径 token 经 §2.2 归一化后过同一 deny 表（单一源）。

| id | 分类 | 覆盖（v1） |
|---|---|---|
| `safety-mechanism-flip` | **block** | Defender 偏好域：`Set-MpPreference`／`Add-MpPreference`／`Remove-MpPreference`／`Set-MpComputerStatus`；防火墙：`netsh advfirewall set`／`netsh firewall set`／`Set-NetFirewallProfile`；Defender 服务：`sc stop|config … windefend|mpssvc`／`net stop windefend|mpssvc`／`Stop-Service … WinDefend|mpssvc`；引导：`bcdedit`；执行策略：`Set-ExecutionPolicy`；审计清除：`wevtutil cl`／`Clear-EventLog`；卷/分区/备份破坏：`diskpart`、`format`（词元精确）、`mkfs*`、`vssadmin delete`、`wbadmin delete`、`fltmc unload`、`dd`＋`of=` 落 deny 根/设备 |
| `system-core-write` | **block** | 破坏/修改动词（`rm`／`del`／`erase`／`rmdir`／`rd`／`Remove-Item`／`mv`／`Move-Item`／`cp`／`Copy-Item`／`xcopy`／`robocopy`／`Set-Content`／`Add-Content`／`Out-File`／`New-Item`／`mkdir`／`icacls`／`takeown`／`attrib`／`reg delete|add|import`（HKLM/HKCR/HKU）／重定向 `>`）＋目标 token 命中表 A/B |
| `carrier-write` | **block** | 同上动词＋目标命中 C1/C2/C3（`.gsa` 相对形态按 cwd 解析） |
| `broad-destructive` | **warn** | 删除类动词＋根级/通配目标（`/`、`\`、`C:\`、`*`、`~`、`$HOME`）或非 cwd 内目标＋递归强制旗（`-r`／`-rf`／`-Recurse`／`/s`） |
| `elevation` | **warn** | `sudo`／`doas`／`runas`／`gsudo`／`-Verb RunAs` 出现 |

**默认＝allow**（表外不动、不加限制）。**钉**：§8 fixture 表（先红后绿），含正向放行集（dev 例行：`cargo`／`git`／
`rg --format`／`python scripts/*`／`Remove-Item .tmp-*`）与逐规则负向命中集。
**已知边界（不保证）**：变量拼接与间接执行（`Invoke-Expression`、`cmd /c %X%`、脚本内命令）、别名与自定义函数、
`.NET` 直调（`[IO.Directory]::Delete`）、here-string 内文本误扫、未存在面的 8.3/shortlink 混淆——best-effort（§9）。

## §5 Windows CFA audit 探针（先行；本机可达性裁决）

**目的**：以 Defender 受控文件夹访问（CFA）**audit 模式**（只记录不拦截；事件 1123/1124）收集「真实写入面」
覆盖率读数，据实裁决 enforce 模式是否作为 L3-Windows v1 及其白名单需求。

**步骤（runbook）**：
1. 现状读数（免提权）：`(Get-MpPreference).EnableControlledFolderAccess`／`Get-MpComputerStatus`／
   Defender Operational 日志可读性（本机读数见报告档）。
2. **enable（需管理员，单条）**：`Set-MpPreference -EnableControlledFolderAccess AuditMode`
3. 狗粮轮（≥1 个真实 run；写面照常）——不拦截、只积累事件。
4. 读 1124：`Get-WinEvent -FilterHashtable @{LogName='Microsoft-Windows-Windows Defender/Operational'; Id=1124}`。
5. 归类：目标落点分布（workspace／`.gsa`／安装目录／系统核心／其他）＋误拦面候选（构建缓存、包管理器、临时目录）。
6. 裁决：覆盖够且误拦可白名单化 ⇒ enforce；否则维持 audit 或只保留 L1/L2（记录理由）。
   **revert**：`Set-MpPreference -EnableControlledFolderAccess Disabled`。

**本轮裁决（D2 记录）**：enable 需交互式提权（本会话非管理员）且狗粮读数依赖后续真实运行——**不触发 UAC、不改系统状态**；
交付 runbook＋脚本（`.tmp-0bw-cfa-audit.ps1`）＋非提权可达读数；enforce 翻转待 1124 读数后由用户/后续批执行。
**CFA 与 deny 表的关系**：CFA 保护面＝用户库类目录（可 `Add-MpPreference -ControlledFolderAccessProtectedFolders`
追加自定义目录），**不覆盖系统核心路径**（Windows/Program Files 对普通进程天然不可写）——CFA 定位＝**L3 附加保险
（对载体/工作区树）**，与 L1 表互补而非替代；若引入，白名单＝实测误拦面。**候选 B（登记）**：AppContainer／受限令牌（v2）。

## §6 L4 补偿与自检

- **既有（0bm⑦，已交付）**：编辑面**回退窗口**——`search_replace` 写前原始字节快照入 `.gsa/rollback/<key>/`
  （每文件保留 5 条），成功面附回退指针告知行。写入管控 v1 沿用为「工具面补偿 leg」。
- **undo 面（2026-09-27 落码，§3.4）**：`orz rollback list/restore`——restore 目标域收敛＋原子写＋第四消费点。
- **设计（未落码，登记）**：git 自动检查点与 journal undo（content anchor 前像复用）——须先裁定容器/触发/回收
  （避免污染用户仓库历史），列 0bw 后续批。
- **载体完整性自检**：**repo 层＝已存在**（`scripts/generate_orz_source_manifest.py`＋`check_repository.py::_check_orz_source_manifest`，
  覆盖 orz 子树；父仓门禁面，本批在报告档记实测读数）；**运行时形态（载体发布物 manifest＋启动自检）＝设计登记、未落码**
  （涉发布/重建链，随载体重建批实施）。**注**：本批未落账的新源码文件将使 manifest 校验呈预期 drift——
  重生成随落账批执行（不提交/不重建令下不动 manifest）。
  **2026-09-27 复审增补（落码边界）**：① 清单键按「单个 Normal 组件」判定（`C:foo` 盘符相对形拒；
  `a..b.txt` 合法文件名不误杀）；② **未列文件检测**（顶层清单外常规文件 ⇒ 告警——已列文件篡改/缺失之外的新增面）；
  ③ **坏清单 ⇒ 静默不检**（解析失败归 NoManifest：宁可不检不可误报——篡改者可删清单消音，D-4 审计腿定位的
  如实退化面）；④ manifest 生成端显式 LF。

## §7 与 0z（真机资源安全边界）的分工表

| 面 | 0z（资源安全） | 0bw（写入管控） |
|---|---|---|
| 管什么 | 资源轴（内存/CPU/进程数/磁盘余量）＋进程树回收＋盘满状态链 | 写目标面（系统核心/载体锁死）＋命令审查＋写补偿自检 |
| 执行者 | OS 内核（Job/cgroup）；orz 如实转达（软提示不阻断） | orz 机械层（工具面/命令面）；L3 由 OS 强制面执行 |
| 失败姿态 | 软提示＋如实转达（不做准入） | block＝硬拒（锁死面）；warn＝留痕不阻断；非绝对保证 |
| 交叉点 | 磁盘写**失败**（ENOSPC）处理属 0z | 写**目标合法性**属 0bw；进程级写权限强制（Landlock/CFA）属 0bw |
| 共享纪律 | 不替模型预判、不加拒绝臂 | 同类（锁死面之外不新增行为；软提示/硬拒界限清晰） |

## §8 判据钉清单（先红后绿）

1. `write_control`：Win/Linux 表命中/放行 fixture（含 `\\?\` 剥除、近祖先 canonical、大小写折叠、env 回退）。
2. 载体集：`.gsa`（委派等价性——既有测试保持绿）、安装目录（整树＋降级两形态）、三件套文件。
3. `exec_policy`：三分类 fixture（每规则 ≥1 命中；正向放行集见 §4）。
4. 接线钉：`search_replace` 拒绝后目标零存在（未写不建）；`run_terminal_cmd` block 不执行（无副作用）、warn 头部行存在。
5. 文案钉：block/warn 文案含规则 id 与（适用时）目标路径。

## §9 已知边界与诚实声明（不做绝对安全性声明）

- **非保证**：工具面＝对 `search_replace` 硬拒；命令面＝best-effort 风险闸；旁路（脚本内命令、变量拼接、.NET 直调、
  别名、图形化工具）不保证覆盖。**对外的表述一律不得超出本档能力面**。
- **非沙箱**：可写面默认仍为真机（allowlist 不做）；锁死面之外不新增限制。
- **CFA 四代价**（083 §10.5 已知）：可被翻转／白名单需求／误拦面／依赖系统状态——作为声明边界，不作为方案缺陷。
- **Unicode 同形字**：仅既有保守表做归一辅助；同形字绕过登记为已知边界。
- **降级面**：install_dir ⊆ cwd 时文件级保护；DLL 侧植等残余风险不保证。**UNC/网络路径**不在 v1 表内。
- **三档措辞**（保证＝机制可证｜阻力＝高成本低收益｜审计＝事后可查）落点＝`orz/SECURITY.md`＋父 README 安全节。
- **2026-09-27 复审增补边界**：
  - **Landlock（L3-Linux）**：① ABI v1 内核 `rename/link` 不受 ruleset 约束（REFER 为 v2 能力——旧内核上
    可把文件移入核心集，内核明载缺口）；② 统一 syscall 号仅 x86_64/aarch64，其余架构 L3 静默不启用
    （L1/L2 仍硬拒）；③ 子进程内装挂失败＝该命令无 L3（write(2) 一行提示，照常 exec）；④ `warn 一次`
    ＝每 spawn 点一次（每进程至多 3 条）；⑤ per-spawn 重复枚举 `/`（毫秒级成本；换来晚建顶层目录的新鲜度）。
  - **事件族（0bw③）**：无覆盖率要求（D-7）；`warn` 留痕在命令超时臂已与 Ok/Err 同纪律收取（2026-09-27
    复审修复）——但队列写入本身 best-effort，审计面缺失 ≠ 审查失效；review↔rule 分类配对由判官核证
    （schema 无法表达跨字段映射）。
  - **载体自检（0bw②）**：flat v1 只覆盖顶层常规文件（grok-home/ 子树由写保护面承接）；坏清单 ⇒ 静默不检
    （宁可不检不可误报）；启动自检时点 tracing subscriber 尚未初始化，有效腿＝stderr 横幅。
  - **undo 面（0bw④）**：目标域收敛 cwd＋归一后 C1 判定＋第四消费点（§3.4）；被挤出保留窗口的快照不可恢复。

## §10 交付与实施记

- 本档（S1）＋代码面（S2 v1：`write_control.rs`／`exec_policy.rs`＋`search_replace`／`run_terminal_cmd` 接线）＋
  CFA 探针 runbook 与读数＋措辞面＋报告
  [`docs/audits/0BW_WRITE_CONTROL_IMPLEMENTATION_AND_FRICTION_2026-09-26.md`](audits/0BW_WRITE_CONTROL_IMPLEMENTATION_AND_FRICTION_2026-09-26.md)。
- **未竟**（登记）：Landlock 落码（Linux 批）、运行态载体自检、专用 journal 事件族、git 检查点/journal undo、
  CFA enforce 翻转裁决、载体重建后的真机复验（S3/S4）。
