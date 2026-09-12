# 0y / S2 机械层接入面实测（一轮，2026-09-12）

> 立项：用户判断「当前安卓线的设计存在不足，深入融合意味着深度适配」→ 裁决
> **先做一轮实测，再改设计文本**。本轮只做实测与登记，**不改设计与代码**。
> 设计权威：[NP1 机械身体集成设计](../NP1_ORZ_BODY_INTEGRATION_DESIGN_2026-09-11.md)；
> 前置记录：[S1 模拟器载体搭建](0Y_S1_EMULATOR_CARRIER_SETUP_2026-09-12.md)。
> 结论落点：设计 §1.2 / §4.2 / §4.3 / §10.2 / §12 / §14.1 的修正批（**待用户裁决后落笔**）。

## 1. 目标与判据

设计与入口登记一直写着「身体层只做扩展面、**不修改 orz 现有机制与工具面**」，
而同一条线又要求§4.3 动作分级、§10.2「对动作与目标做规则匹配」、§4.2 感知总线
经 PULL 进模型。本轮要回答的唯一问题是：

> **现有机械层在安卓动作族上，哪些面能直接兜住、哪些面完全不存在、哪些面会误放？**

判据（机械可核验）：对同一批真实安卓命令，逐条记录现有判定面的输出；
「能兜住」= 判定面给出与该命令真实破坏性一致的分档；「会误放」= 承重墙级命令
得到与只读观测**同形**的判定；「需新开投影」= 判定面存在但语义不覆盖。

## 2. 方法与载体

| 项 | 内容 |
| --- | --- |
| 探针形态 | 源码内 `#[cfg(test)] mod body_probe` 三个（**不参与生产编译**，不改生产逻辑）：`orz-workspace/permission/manager.rs`（逐命令判定面）、`orz-host/permission.rs`（权限门 + 工具名映射）、`orz-tools/types/resources.rs`（读面沙箱） |
| 源码基线 | orz 工作副本 `f9fb70e4`（0.4.3，与 S1 载体同源） |
| 构建 | `cargo test -p orz-workspace -p orz-host -p orz-tools body_probe -- --nocapture`；`PROTOC=D:\CLI\orz\bin\protoc.exe`；首次全量编译 5m29s，修正版增量 3m01s |
| 设备侧载体 | 模拟器 `orz_body_a35`（Android 15 / AOSP ATD / x86_64 / userdebug）**干净基线态**（无 Magisk），`adb root` 与非 root 两档各跑一遍 |
| 设备侧脚本 | `.s2probe/device_probe.sh`（**只跑只读观测**；破坏性/不可逆动作一律不执行） |
| 原始证据 | `.s2probe\probe.out.log`（面 B/C + 面 A 第一版）、`.s2probe\probe2.out.log`（面 A 修正版）、`.s2probe\dev_shell\*.txt` / `.s2probe\dev_root\*.txt`（设备侧）、`.s2probe\build2.out.log` |

**纪律声明**：本轮设备侧不执行任何破坏性命令（`dd` 到块设备、`rm -rf`、`pm clear`、
`reboot`、`iptables -F` 等只做**判定面**核验，不在设备上真跑）。

## 3. 面 A：权限门与工具名映射（实测矩阵）

两列对照：**真身** = `gateway=None` 的无头桥（与身体 `-p` 部署同形，提示无客户端 →
Ask → Deny）；**替身** = `allow_all` 管理器（只用于暴露「策略短路面」）。
*（第一版只用替身，见 §9 失误登记。）*

身体部署形态 `Benchmark{allow_shell:true}`（`--allow-write --allow-shell`）：

| 动作 | 工具 | access_kind | 真身（无头） |
| --- | --- | --- | --- |
| 只读观测（`dumpsys`） | `bash` | `Bash(…)` | **AllowOnce** |
| 承重墙·分区写入（`dd of=/dev/block/by-name/boot`） | `bash` | `Bash(…)` | **AllowOnce** |
| 承重墙·同一命令经 `sh -c` 包装 | `bash` | `Bash(…)` | **AllowOnce** |
| 不可逆·清应用数据（`rm -rf /data/data/…`） | `bash` | `Bash(…)` | **AllowOnce** |
| 读系统私有文件 | `read_file` | `Read(Some(…))` | AllowOnce（工具层另拒，见 §5） |
| 假设新增：感知工具（未登记） | `body_perceive` | `Edit("body_perceive: …")` | **Deny** |
| 假设新增：动作工具（未登记） | `body_act` | `Edit("body_act: …")` | AllowOnce |
| 假设新增：感知 PULL（未登记） | `body_pull` | `Edit("body_pull: …")` | **Deny** |

对照组（其余策略，同一批请求）：`Benchmark{allow_shell:false}` 与 `ReadOnly` 下
**全部 shell 请求 Deny**（策略短路，非逐命令判定）；`Interactive` 无头下 shell 全
Deny、`read_file` 全 AllowOnce。可见**拒绝来自策略短路或无头提示失败，不来自动作语义**。

结论：

1. **身体部署形态下，权限门对承重墙命令零拦截力**——`allow_shell` 短路发生在
   `access_kind` 与逐命令分析面**之前**（`orz-host/src/permission.rs::request` 238–264），
   该形态唯一的兜底是 journal 与 ACAF 票据。
2. **未登记的新工具在无头下确定性拒**（`access_kind` else 分支落 `Edit` → 无头 Ask →
   Deny），与 0v S4 的 `browser_control` 缺陷同形（`permission.rs:552-572` 注释已自陈曾
   三次复发）。**一个身体能力要接入 = 至少两处登记（controller `risk_class` + host
   `access_kind`）+ 一处策略投影（`ToolPolicy`）**，否则无头部署下必拒。
3. 「不需要扩工具面」（`ACT-KEY-01`）在**能力**上成立（root shell 什么都能跑），
   但**不保证可判定性**：动作经 `bash` 走时，现有面要么全放行（身体形态）、要么全拒
   （其余无头形态），**没有中间档**。

## 4. 面 B：逐命令判定面（`evaluate_bash`，实测矩阵）

对 42 条真实命令逐条取 `writes_real_file` / `exec_risk` / `env_risk` / 段落分类 /
写路径清单 / 保护面判定。

| 设计分级 | 命令（摘） | writes_real_file | 段落分类 | 结论 |
| --- | --- | --- | --- | --- |
| 只读观测 | `dumpsys …` `logcat` `cmd appops get` `service call` `screencap` | false | **NeedsPrompts**（未知程序一律需提示） | 与破坏性命令**同形** |
| 只读观测 | `cat /proc/meminfo` `ls /data/data/…` | false | AutoAllow（在安全清单内） | 仅 `cat`/`ls` 这类通用命令受益 |
| 可逆动作 | `am start` `input tap` `pm grant` `am force-stop` `svc wifi` `settings put` `cmd netpolicy` `iptables -m owner` | false | NeedsPrompts | 与只读观测**同形**，无"可逆/需记账"标记 |
| 破坏性动作 | `dd if=… of=/dev/block/by-name/boot` | **true** | NeedsPrompts | 写盘面**捕获** |
| 破坏性动作 | `echo x > /dev/block/by-name/boot` | **true** | NeedsPrompts | 重定向面**捕获** |
| 破坏性动作 | `rm -f /system/framework/services.jar` / `rm -rf /metadata/…` | **true** | NeedsPrompts + `any_dangerous:true` | **捕获** |
| 破坏性动作 | `reboot` / `magisk --remove-modules` / `iptables -F` / `setprop persist.sys.safemode 1` / `mount -o rw,remount /system` / `pm clear` / `pm uninstall` | false | NeedsPrompts | **零标记**，与只读观测同形 |
| 不可逆动作 | `reboot bootloader` / `fastboot flashing unlock` / `rm -rf /data/adb` / `echo 0 > /sys/block/sda/device/delete` | 仅 `rm`/重定向捕获，其余 false | — | 一半零标记 |
| 绕过形态 | `sh -c 'dd … of=/dev/block/by-name/boot'`、`eval "dd …"`、`busybox dd …` | **false** | NeedsPrompts | **写盘面丢失** |
| 绕过形态 | `D=/dev/…; dd of=$D` | false | **Unparseable** + `env_risk=Unvetted` | 有兜底，但不识别为分区写入 |
| 绕过形态 | `echo … \| base64 -d \| sh` | false | NeedsPrompts ×3 | 无风险标记 |

关键路径面（`is_safe_write_sink` / `edit_target_protection`，18 条）：

| 类别 | 路径（摘） | 判定 |
| --- | --- | --- |
| 分区写入 / 引导链 | `/dev/block/by-name/boot`、`/dev/block/by-name/modem`、`/dev/block/sda1` | `sink_safe=false`、**`edit_protected=None`** |
| 系统完整性 | `/system/framework/services.jar`、`/system/usr/keylayout/gpio-keys.kl`、`/vendor/etc/selinux/precompiled_sepolicy` | **`None`** |
| 身份与账户 / 恢复通道 / 模型存活 | `/data/system/users/0/accounts.db`、`/metadata/watchdog/magisk/sepolicy.rule`、`/data/adb/modules/orz_body` | **`None`** |
| 对照（桌面语义） | `/etc/hosts` → `Some(Etc)`；`/home/user/.zshrc` → `Some(StartupFile)`；`/dev/null` → `sink_safe=true` | 保护表**只认桌面语义** |

结论：

1. **动作语义面不存在**：`reboot`、`pm clear`、`pm uninstall`、`magisk --remove-modules`、
   `iptables -F`、`setprop`、`mount -o rw,remount` 在现有面上**与只读观测完全同形**（0 标记）。
   §4.3 的四级分档**没有任何机械落点**。
2. **承重墙目标面不存在**：安卓承重墙路径在保护表上一个都不命中（桌面路径正常命中）；
   写盘白名单只认 `/dev/null|stdout|stderr`。§10.2「目标落在块设备/分区/引导分区即命中」
   的清单**当前无实现**。
3. **直接写法可捕获、包装写法丢失**：`dd of=`、`>`、`rm` 捕获；`sh -c`/`eval`/`busybox`
   包装后写盘面为 false。**拦截若只建在这一面上，可用一行包装绕过**。

## 5. 面 C：读面沙箱（`is_path_allowed_for_read`，实测）

工作区根取设备侧拟定落点 `/data/local/orz`，`session_volume=None`（纯工作区二元判定）：

| 设计域 | 路径 | allowed |
| --- | --- | --- |
| 内核态 | `/proc/meminfo` | **false** |
| 内核态 | `/sys/class/leds/aw210xx_led/brightness` | **false** |
| 系统服务 | `/data/system/packages.xml` | **false** |
| 应用与内容 | `/data/data/com.tencent.mm/databases/msg.db` | **false** |
| 外部存储 | `/sdcard/orz/shot.png` | **false** |
| 承重墙：分区/系统/存活 | `/dev/block/by-name/boot`、`/system/framework/services.jar`、`/data/adb/modules/orz_body/module.prop` | **false** |
| 对照：工作区内 | `/data/local/orz/notes.md` | true |
| 对照：会话卷窗口 | `/data/local/orz/.gsa/session/terminal/ord-1.log` | true |

结论：**身体要「看」的一切都在放行面之外**。`read_file`/`grep`/`list_dir` 三条读工具
在设备上对感知目标全拒（判据在 `orz-tools resources::is_path_allowed_for_read`：技能根
→ 会话卷窗口 → workspace → **其余拒**）。而权限门侧对同一路径是**放行**的（§3），
即**两层判定不一致**：读工具死路、shell 通路（唯一活路）。

## 6. 面 D：设备侧环境与能力面（模拟器实测）

### 6.1 环境与可执行边界

| 项 | shell 用户 | root |
| --- | --- | --- |
| `PATH` | `/product/bin:…:/system/bin:/system/xbin:/vendor/bin:…` | 同 |
| `HOME` | **`/`** | **`/`** |
| `TMPDIR` | `/data/local/tmp` | 同 |
| `id` | `uid=2000(shell) … context=u:r:shell:s0` | `uid=0(root) … context=u:r:su:s0` |
| 时间 | `Sat Sep 12 12:22:23 GMT 2026`（正确，无退时问题） | 同 |
| `/data/local/tmp` | 可写 + **可执行**（`EXEC_OK`） | 同 |
| `/sdcard` | 可写但 **noexec**（`can't execute: Permission denied`, exit 126） | 同 |
| SELinux | `Enforcing`，上下文可见（`u:object_r:*`） | 同 |
| 块设备 | `/dev/block/by-name/` 仅 `metadata super vbmeta vda vdd`——**无 `boot` 节点** | 同 |

**`HOME=/` 是一条被忽略的适配事实**：任何以 `HOME`/`~` 定位配置、凭据、缓存的机制
在设备上会落到根目录（rootfs，重启即失）；`/etc` 在安卓上也不是桌面语义的配置根。

### 6.2 命令可用性（`command -v`）

**有**：`toybox` 全家（`sed`/`grep`/`awk`/`find`/`sort`/`dd`/`xxd`/`base64`/`tar`/`gzip`/
`timeout`/`nproc`/`strace`/`nc`/`lsof`/`readelf`）、`getevent`、`dumpsys`、`am`、`pm`、
`cmd`、`input`、`screencap`、`iptables`、`svc`、`settings`、`logcat`、`uiautomator`、`su`。
**无**：`bash`、`python3`、`curl`、`wget`、`lz4`、`magisk`（干净基线）。

→ `bash` 缺失意味着「终端工具」在设备上实际执行面是 **mksh（`/system/bin/sh`）**；
现有分析面按 bash 词法解析（tree-sitter）在多数形态下仍可用，但这不是可忽视的差异。

### 6.3 感知面（设计 §4.2）逐条可用性

22 条只读观测命令**全部 exit=0**（`dumpsys activity|battery|meminfo|window|sensorservice|
connectivity|accessibility|display`、`service list`、`/proc/meminfo`、`/proc/stat`、
`logcat -d`、`pm list packages`、`settings get`、`getevent -lp`、`uiautomator dump`、
`screencap -p`、`service call` 等），单次体量：

| 观测 | 行数 | 字节 |
| --- | --- | --- |
| `dumpsys activity activities` | 266–269 | ~16 KB |
| `dumpsys meminfo` | 524 | ~25 KB |
| `dumpsys display` | 506 | ~25 KB |
| `dumpsys connectivity` | 169 | ~26 KB |
| `dumpsys sensorservice` | 120 | ~9.5 KB |
| `service list` | 282 | ~18 KB |
| `uiautomator dump`（UI 树） | 5 节点 | 1.8 KB |

**权限分档（shell vs root）**：绝大多数观测 **shell 即可**；仅
`/sys/class/leds`（`Permission denied`）、`/sys/class/thermal/*`（同）、
`iptables -L`（`you must be root`）需要 **root**。

**失败形态**：`cmd appops get <未安装包> RUN_ANY_IN_BACKGROUND` → `exit=255` +
`Error: No UID for <pkg> in user 0`——机械层必须把「命令失败/目标不存在/真无匹配」分型
（对照案例库 ORZ-TOOL-BINARY-COMPAT-001 的三型纪律），否则模型的观测面会静默失真。

**结构化界面**：shell 侧唯一通路是 `uiautomator dump`（**拉取式**，冷启动成本另计），
与设计 §4.2「AccessibilityNodeInfo **常驻订阅**」不是同一件东西——后者需要特权应用内
跑 AccessibilityService，属**新增执行落点**。

## 7. 三分类结论

**① 能兜住（可直接复用）：**

1. 写盘检测（`dd of=`/重定向/`rm` 危险标记）——覆盖「直接写法」的承重墙命令。
2. bash 词法解析面（tree-sitter）：安卓命令族**全部可解析**（除命令替换/变量间接 → `Unparseable` 兜底）。
3. 读面沙箱结构（技能根 → 会话卷窗口 → workspace → 拒）在设备上语义不变。
4. 设备侧只读观测面（`dumpsys`/`/proc`/`logcat`/`getevent`/`uiautomator`/`screencap`）全部可用。
5. 无头拒绝纪律自身（`Ask → Deny`）是现成的 fail-closed 行为。

**② 需新开投影（现有面存在、语义不覆盖）：**

1. **安卓动作语义表**：命令族 → 动作等级（只读/可逆/破坏性/不可逆）的机械映射。
2. **安卓承重墙目标集合**：分区/块设备/引导链/系统完整性/身份标识/模型存活的目标匹配。
3. **感知面通道归属**：`read_file` 死路已实测，感知要么走 terminal（文本、单次 16–26 KB），
   要么明确新开读通道。
4. **结构化界面的常驻订阅**：需特权应用内 AccessibilityService（新增落点）。
5. **新工具的登记面**：`risk_class` + `access_kind` + `ToolPolicy`，缺一即无头确定性拒。

**③ 会误放（承重墙级命令与只读观测同形）：**

1. **身体部署形态（`--allow-shell`）下 shell 一律放行**——§10.2 的拦截在该形态下无落点。
2. `reboot`／`pm clear`／`pm uninstall`／`magisk --remove-modules`／`iptables -F`／
   `setprop persist.sys.safemode 1`／`mount -o rw,remount /system`：**零标记**。
3. `sh -c`／`eval`／`busybox` 包装：**写盘面丢失**。
4. 安卓承重墙路径在保护表上 **全部不命中**。

## 8. 对设计文本的修正建议（待裁决）

| 设计位置 | 现状问题 | 建议措辞方向 |
| --- | --- | --- |
| §1.2 非目标行「不修改 orz 现有机制与工具面」 | 按字面读会否掉 §10.2 的实施前提；且与 §4.1「共用同一套身体层接口」冲突 | 拆两句：**模型可见工具契约冻结**（ADR-0010）；**机械层新增安卓投影**（动作语义表 + 承重墙目标表 + 策略注册表），属本支线交付物 |
| §4.3 动作分级 | 四档只有文字，无机械落点 | 补「落点」：分档必须落在安卓动作语义表上，且接在**执行前**的插入点 |
| §10.2 确定性拦截 | 「规则匹配」未写插入点与部署形态约束 | 补：拦截的插入点（权限门之前/之内）；并明确**身体不得以 `allow_shell` 全放行形态运行**，否则该节无判据 |
| §4.2 感知总线 | 未写通道归属与体量 | 补：感知经 terminal/PULL 域（`read_file` 已被沙箱拒，实测）；单次观测体量档（16–26 KB）；结构化界面「拉取 vs 订阅」二选一与代价 |
| §12 验证策略 | 无承重墙判据 | 补验收判据：**承重墙命令拒绝率**（N 条样例命令逐条拒/升级票据）+ 感知体量档 |
| §13 里程碑 | M1 前置只写「orz 接口定义」 | 明确该前置 = §14.1 三层面 + 本记录三分类的落地顺序 |
| §14.1 待定项 | 被列为「不阻塞当前」 | 升格为 **M1 前置交付物**，写清三层面与各自 owner（机械层 vs 身体守护进程） |
| 新增 | 无工具可用性差异登记 | 补一条「设备侧工具面差异清单」：无 bash/python3/curl/wget/lz4；`HOME=/`；`/sdcard` noexec；`by-name` 无 `boot` 节点；部分观测需 root |

## 9. 失误与纠正（如实登记）

1. **第一版权限门探针用了 `allow_all` 替身**，把「无头 → Ask → Deny」这一档掩盖了：
   结果是未登记工具在四个策略下**全部 AllowOnce**，与真实无头部署相反。修正为**双列
   对照**（真身 `gateway=None` + 替身），结论随之改写（§3 表）。**教训**：探针的替身
   若比真身宽松，会把"由提示面提供的拒绝"误读为"策略放行"。
2. **读面探针 `session_volume=None`**：对照行 `/data/local/orz/.gsa/journal/events.jsonl`
   显示 `allowed=true`，这只反映「卷未注入时退回 workspace 二元判定」，
   **不代表生产的窗口语义**；本记录已按此口径标注（§5）。
3. **路径保护面在 Windows 宿主上运行**：`edit_target_protection` 依赖 `is_absolute` 与
   符号链接解析，宿主语义与安卓不完全一致。本轮「安卓承重墙路径全部 `None`」的主要依据是
   **保护表条目全为桌面语义**（与平台无关），精确边界待设备侧复测。

## 10. 边界与未覆盖

- 真机 NP1 上的同名命令形态未测（厂商差异、`by-name` 命名、SELinux 域）。
- 模拟器为**干净基线**（无 Magisk），`/data/adb` 存在但无模块；模块面沿用 S1 结论。
- **ACAF 票据与 shell 动作的实际接线未查**（§3 结论 1 的兜底是否成立，需单独实测）。
- 未实测：M5 补丁件与权限面的交互；语音/凭据/Glyph 域；无障碍服务的常驻订阅。
- 探针源码**未提交进 orz**：原样归档在 `.s2probe\probe_sources\`（三份文件快照）+
  设备侧脚本 `.s2probe\device_probe.sh`；**实测后已把 orz 工作副本还原干净**
  （门禁 `valid:true`）。是否把三支探针升格为常驻回归面（orz 侧提交），
  属独立决策，未在本轮做。

## 11. 复现参数速查

```
构建     cd D:\CLI\.tools\np1-body\orz
         $env:PROTOC='D:\CLI\orz\bin\protoc.exe'
         cargo test -p orz-workspace -p orz-host -p orz-tools body_probe -- --nocapture
设备侧   adb -s emulator-5554 push .s2probe\device_probe.sh /data/local/tmp/
         adb -s emulator-5554 shell sh /data/local/tmp/s2_device_probe.sh shell|root
         结果在 /data/local/tmp/s2_probe_<tag>/（env/tools/paths/observe/exec）
载体     AVD orz_body_a35（干净基线态）；模拟器命令必须显式 -s emulator-5554
```
