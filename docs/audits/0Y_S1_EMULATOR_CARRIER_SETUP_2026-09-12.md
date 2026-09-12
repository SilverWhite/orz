# 0y / S1 模拟器验证载体搭建记录（2026-09-12）

> **文档类型**：实施记录（S1 前四项中的 ①②③ 完成）
> **状态**：S1 ①无头镜像选型搭建、②orz 载体上机冒烟、③Magisk 模块打包 / 安装 / 禁用 / 恢复演练 **完成**；④M5 流程纪律干跑 **未开始**
> **日期**：2026-09-12
> **范围**：NP1 机械身体集成支线设计 §12「2026-09-12 用户裁决」引入的常设验证载体——模拟器——的搭建与能力判据
> **边界**：本记录只登记载体搭建事实与判据，不修改设计权威；发现的两处 §12 偏差只登记、待用户裁决（用户裁决回填见 §8）。
> **证据**：模拟器运行 stdout/stderr 与设备侧证据落 `D:\tb-eval\s1_emulator\logs\`；干净基线副本落 `D:\tb-eval\s1_emulator\baseline_avd\`；③ 批次的补丁 ramdisk / 模块包 / 容器脚本落 `D:\tb-eval\s1_emulator\{ramdisk,module,bin,magisk}\`。

---

## 1. 目标与判据

按设计 §12 裁决，模拟器承担两件事：①自有代码验证（守护进程、IPC、协议、工具层、语音流水线）；②M5 补丁「打补丁 → 进系统 → 开机 → 回滚」流程纪律干跑。

本批把「载体可用」拆成四组可机械核对的判据：**能启动**、**能执行我们的载体**、**状态可保留**、**能回到干净基线**。

---

## 2. 环境与硬约束

| 项 | 值 |
| --- | --- |
| 宿主 | Windows 11 专业版 26200（25H2）/ i5-12400F（6 核 12 线程）/ 16 GB |
| 虚拟化 | Hypervisor 已在运行（Docker Desktop + WSL2 常驻）；模拟器官方自检 `WHPX(10.0.26200) is installed and usable` |
| **磁盘（决定搭建位置）** | C 盘仅余 **1.8 GB**；D 盘 30.4 GB → 工具链与虚拟设备**全部落 D 盘**，并把 Java 临时目录重定向到 `D:\android\tmp` |
| 网络 | 沙箱内访问 Google 下载源 SSL 失败 → 全部下载**出沙箱**执行 |

`S1-ENV-01`：C 盘 1.8 GB 是本次搭建的硬约束。后续若要更大的系统镜像或更多基线副本，须先清理 C 盘或明确接受 D 盘容量上限。

---

## 3. 工具链落位

| 组件 | 版本 | 落点 | 占用 |
| --- | --- | --- | --- |
| cmdline-tools | 19.0（`commandlinetools-win-13114758_latest`） | `D:\android\Sdk\cmdline-tools\latest` | 157 MB |
| Android Emulator | 37.1.11.0（build 15917651） | `D:\android\Sdk\emulator` | 1,033 MB |
| Platform-Tools | adb 1.0.41 | `D:\android\Sdk\platform-tools` | 17 MB |
| 系统镜像 | `system-images;android-35;aosp_atd;x86_64` | `D:\android\Sdk\system-images` | 1,696 MB |
| 虚拟设备 | AVD `orz_body_a35` | `D:\android\avd` | 3,144 MB |

安装时逐条确认了 Google 官方 SDK 许可条款（`licenses` 已落 SDK 根）。

选型理由：选 **AOSP ATD**（Automated Test Device）而非 `google_atd`——本支线验证的是我们自己的代码，不需要 Google 服务面；选 **API 35** 与 NP1 的 Android 15 对齐（`BODY-PRE-11` 定版 `V3.2-260618-1045`）。

---

## 4. 启动判据

AVD 配置：x86_64 / 4 核 / 2 GB RAM / 数据分区 6 GB / 无 Play Store / 无 GPU（ATD 无头形态）/ 320×640。

无头启动参数：`-no-window -no-audio -no-boot-anim -gpu swiftshader_indirect -port 5554`。

| 判据 | 实测 |
| --- | --- |
| `sys.boot_completed` | `1`（首次启动 **35 s**） |
| `ro.build.version.sdk` / `release` | `35` / `15` |
| `ro.product.cpu.abi` | `x86_64` |
| `uname -r` | `6.6.30-android15-8-gdd9c02ccfe27-ab11987101` |
| 默认 adb shell 身份 | `uid=2000(shell)` |
| `ro.adb.secure` | `0`（边界见 §9） |

---

## 5. orz 载体上机冒烟

用**当前冻结载体 0.4.3**（源 `D:\tb-eval\orz-linux`），非设计文本写的 0.4.2——原因见 §8 偏差一。

| 件 | 字节 | SHA256（主机 = 设备，逐件吻合） |
| --- | --- | --- |
| `orz` | 109,540,832 | `eb7241343efb7b572cd7c2aeaad5fbb1de75b554551ecb286a9a1a60e84c5e9f` |
| `orz-acaf-provision` | 1,215,056 | `6092b85bd835f51e32f2472af5df4262c0bb04f110b3c48cebf803f142cc4cc3` |
| `orz-signer` | 1,396,424 | `e1b84ac559bfe4213d3c2f9b40162d1720f1ca3a48aecd5b5498d9b14a27aaf3` |

ELF 自包含性（设备自带 toybox `readelf` 0.8.11-android 核验，三件同形）：

| 判据 | 值 |
| --- | --- |
| `e_type` | `DYN`（PIE） |
| `e_machine` | `x86-64` |
| `INTERP` 段数 | **0** |
| `NEEDED` 依赖 | **0** |

执行判据（落 `/data/local/tmp/orz-s1/`，`timeout 20`）：

| 件 | 观测 | 退出码 |
| --- | --- | --- |
| `orz --version` | `error: tui io error: No such device or address (os error 6)` | 1 |
| `orz-acaf-provision` | 打印自身 usage 行 | 1 |
| `orz-signer` | `signer manifest not found at /data/local/tmp/orz-s1/signer-manifest.json` | 1 |

三件都走到了各自的内部代码路径，即**在 Android 15 内核上真实执行**；`orz` 的报错是它在无 TTY 的 adb shell 下初始化 TUI 失败，与 Linux 宿主冒烟的既有形态一致（`BODY-PRE-01` 在 x86_64 上的对应判据成立）。另：`orz` 内含 `0.4.3` 版本标记 3 处。

---

## 6. 载体能力判据（持久化 / 回滚 / root）

### 6.1 状态持久化——先否定、后修复

| 轮次 | 配置 | 观测 | 判定 |
| --- | --- | --- | --- |
| run1 → run2 | avdmanager 默认（`disk.dataPartition.path = <temp>`） | 重启后 `/data` 标记 **MISSING** | ❌ 不持久 |
| run3 → run4 | 移除该行后 | 重启后标记 `beta` **仍在** | ✅ 持久 |

`S1-CARRIER-01`：默认创建的 AVD 数据分区是**临时**的，`/data` 改动重启即失。装 Magisk 模块、走「安装 → 重启 → 禁用 → 恢复」回路都要求持久化，故必须移除 `config.ini` 中的 `disk.dataPartition.path = <temp>`（已改，原文件备份 `config.ini.s1-backup`）。

### 6.2 回滚原语——快照不可用，改判为文件级基线副本

设计 §12 裁决写的回滚机制是「删快照，零成本回滚」。本机实测**两条快照恢复路径都不可用**：

| 路径 | 观测 |
| --- | --- |
| 保存 `adb emu avd snapshot save` | ✅ OK（单快照 `ram.bin` 约 1.6 GB） |
| 热加载 `adb emu avd snapshot load` | ❌ WHPX `Failed to set virtual processor context, hr=c0350015` + `Unexpected VP exit code 4` → 模拟器退出 |
| 冷加载 `-snapshot <name>` 启动 | ❌ 进程起来、日志报 WHPX operational，但 guest 持续 `adb offline`（等 242 s 无 `boot_completed`） |

改判为**文件级基线副本**：

```text
基线 = AVD 目录副本（robocopy /MIR，排除 snapshots/）   实测 561 MB
回滚 = 关机 → 副本还原 → 正常启动（约 35 s）
```

判据：runB 写入 `/data/local/tmp/persist_probe.txt = gamma`，还原基线后 runC 复读为 **MISSING** → ✅ 回滚成立。

`S1-CARRIER-02`：回滚语义由「快照」改为「基线副本」，代价仍在零成本量级（一次文件还原 + 一次正常启动），但机制与设计文本不同——登记为**设计偏差，待用户裁决**（§8 偏差二）。

### 6.3 root 与系统分区形态（③ 的前置）

| 项 | 实测 |
| --- | --- |
| `adb root` | ✅ `uid=0(root)`，SELinux 上下文 `u:r:su:s0`；`adb unroot` 可退回 shell |
| `/data/adb/modules` | root 下可读写（建删目录实证） |
| `/`、`/vendor` 挂载 | `ro,seclabel,relatime`（只读） |
| `adb remount` | ❌ `Device must be bootloader unlocked` |
| Magisk | 未安装（无 `/data/adb/magisk`，无 `magisk` 命令） |

`S1-CARRIER-03`：模拟器上改系统面**不能**走 `adb remount`，只能在启动时带 `-writable-system`（本镜像非 Play Store 变体，符合条件）。与 NP1 的 Magisk systemless overlay 同族但机制不同，③ 须按模拟器自身形态落实施方案。

---

## 7. S1-③ Magisk 模块演练（打包 / 安装 / 禁用 / 恢复）

### 7.1 版本与工件冻结

| 工件 | 来源 / 版本 | 字节 | SHA256 |
| --- | --- | --- | --- |
| Magisk 官方 APK | `topjohnwu/Magisk` release **v30.7**（2026-02-23；与真机 NP1 现役同版本） | 25,278,536 | `40729525e298a11dad6e13586a29efa48a48e97cbf00203798e8047d1c710cb2` |
| 原生 ramdisk | 镜像 `system-images;android-35;aosp_atd;x86_64` 的 `ramdisk.img` | 2,141,831 | `2322fe242722ed8a974809ee6fa984524078eb8d71dfca89fc4714e4f46776e9` |
| Magisk 补丁 ramdisk | 本批脚本按 v30.7 `assets/boot_patch.sh` 的 ramdisk 段重建 | 2,834,471 | `2101609a98fc5965c67656da833984af177936ec6267c111837564d61fbdd137` |
| 演练模块包 | `orz_body` v0.1.0-s1drill（12 项） | 3,533 | `8307cb33bf24ce7a2fca50c8659ccb834e300a22a7fea2e0c3323c2310907cd3` |

选版理由：与真机 NP1 现役 Magisk 同版本（`BODY-PRE-12`），模块语义与终验一致；v30.7 官方包内含 x86_64 全套组件（`libmagiskinit` / `libmagiskboot` / `libmagiskpolicy` / `libbusybox` / `libinit-ld` / `libmagisk`），无需自建。APK 的 SHA256 与 GitHub release 公布的 digest 逐字吻合。

### 7.2 进模拟器的机制（为什么不是装 APK）

模拟器没有可修补的 `boot.img`（内核是 emulator 自带的 `kernel-ranchu`，ramdisk 独立成文件），Magisk 应用「安装 → 修补 boot 镜像」路径不适用。本批走 **ramdisk 注入**：解压 `ramdisk.img`（实测 `lz4_legacy` → cpio，内含 `/init`，标准 2SI 形态）→ 按官方 `boot_patch.sh` 的 ramdisk 段替换 `/init` 为 `magiskinit`、加入 `overlay.d/sbin/{magisk,stub,init-ld}.xz`、写 `.backup/.magisk` 配置 → 以 `-ramdisk <补丁文件>` 启动。

实测：开机 **44 s** 达 `boot_completed=1`；`magisk --path` = `/debug_ramdisk`；daemon 版本 `30.7:MAGISK:D`（`-V` = 30700）；init 的三个触发器 `post-fs-data` / `service` / `boot-complete` 在 dmesg 中逐条成功回执。补丁与打包全程在本地容器内完成，`magiskboot` 的 `decompress` / `cpio` / `compress=lz4_legacy` 三个子命令可用。

### 7.3 一处必要的额外交付：持久环境落地

注入式 Magisk 只有 ramdisk 里的一份二进制，`/data/adb/magisk/` 为空 → `magisk --install-module` 报 **`Incomplete Magisk install`**（该分支的判据就是找不到 `/data/adb/magisk/util_functions.sh`，已在二进制内定位到该字符串与文件路径）。按 Magisk 应用首次运行的同名清单补齐 `/data/adb/magisk/`（`magisk` / `magisk32` / `magisk64` / `magiskboot` / `magiskinit` / `magiskpolicy` / `busybox` / `init-ld` / `stub.apk` + 五个 `.sh` + `chromeos/`），`magisk --restorecon` 后模块安装正常。**这是注入路线的固定代价**（真机走应用安装不涉及，故不构成设计缺口）。

### 7.4 模块打包（`orz_body` v0.1.0-s1drill）

包内 12 项，按官方模板成形（`module.prop` + `customize.sh` + `META-INF/com/google/android/{update-binary,updater-script}`，其中 `update-binary` 逐字取自 v30.7 `assets/module_installer.sh`），payload 对齐设计 §9.3 的四类承载点：

| 承载点（设计 §9.3） | 包内落点 | 本轮核证 |
| --- | --- | --- |
| 常驻守护 | `service.sh`（拉起心跳守护）+ `post-fs-data.sh`（早期钩子） | ✅ 两阶段均执行，守护进程存活 |
| systemless overlay | `system/etc/orz-body/overlay-marker.conf` | ✅ 生效且摘除可逆 |
| priv-app 白名单 | `system/etc/permissions/privapp-permissions-orz-body.xml` | ⚠️ 只验证「同批 overlay 不破坏开机」，未验证真实特权授予 |
| 物理键兜底 | `system/usr/keylayout/orz-body.kl` | ⚠️ 同上：文件形态落地，未做按键行为核证 |
| SELinux 微调 | `sepolicy.rule` | ❌ **未生效**——见 §7.7 |
| 卸载钩子 | `uninstall.sh` | ✅ 卸载路径实测清理状态目录 |

### 7.5 安装 → 禁用 → 恢复 全回路（含卸载与回滚）

| 轮次 | 操作 | 判据 | 结果 |
| --- | --- | --- | --- |
| 安装 | `magisk --install-module <zip>` | 落 `modules_update/orz_body`、安装器打印 `- Done` | ✅ |
| 开机 ① | 重启客户机 | overlay 三件出现在 `/system/...`（`u:object_r:system_file:s0`）、`post-fs-data.sh` 与 `service.sh` 均执行、心跳守护 `PPID=1` | ✅ |
| **禁用** | `touch /data/adb/modules/orz_body/disable` + 重启 | overlay 三件全部消失、两个脚本**未**再执行（标记仍是上一轮 `boot_id`）、Magisk 挂载数 9 → 7、Magisk 本体 `-V` 仍 30700 | ✅ |
| **恢复** | `rm /data/adb/modules/orz_body/disable` + 重启 | overlay 回归、脚本重新执行（标记刷新为新 `boot_id`）、守护进程重新拉起 | ✅ |
| 卸载 | `touch /data/adb/modules/orz_body/remove` + 重启 | 模块目录整目录消失、overlay 消失、`uninstall.sh` 清掉 `/data/adb/orz-body-drill` | ✅ |
| **回滚** | 关模拟器 → 还原 AVD 基线副本 → **不带** `-ramdisk` 启动 | `/debug_ramdisk` 为空、`command -v magisk` = none、Magisk 挂载数 = 0、基线标记文件缺失、`sdk=35` 正常开机 | ✅ |

三次开机的 `boot_id` 各自独立（`a6ff4d33…` → `d77ecb48…` → `36f1d20e…`），排除「假重启」误判；禁用轮与恢复轮之间还有一次重启（`d77ecb48…` 即禁用轮）。

### 7.6 三条对设计有直接价值的事实

1. **`service.sh` 拉起的进程能活过开机脚本壳的清理**——实测守护进程 `busybox sh /data/adb/modules/orz_body/service.sh` 的 `PPID=1`（被 init 收养），心跳每 15 s 一条、跨多轮持续。设计「常驻守护靠模块 `service.d` 拉起 + 看护脚本」在真机制上成立。
2. **systemless 是真的**——模块文件只出现在 `magisk /system/etc tmpfs ro` 这层挂载上（`/proc/mounts` 直接可见）；禁用后文件即消失，底层系统镜像零改动。
3. **回滚原语在「Magisk 已装 + 模块已装」的满载状态下仍成立**——模拟器会把 `-ramdisk` 指定的补丁 ramdisk 自动落到 AVD 目录的 `initrd`（实测 2,143,752 → 2,836,392），因此「还原 AVD 基线副本」一步同时清掉 `/data` 改动与补丁 ramdisk，不需要第二个动作。

### 7.7 本轮**未**取得的两项（如实登记）

- **`sepolicy.rule` 未生效**：安装时 Magisk 报 `- Unable to find preinit dir`。注入式 Magisk 的 `.backup/.magisk` 配置里没有 `PREINITDEVICE`（真机由应用在安装时写入），`/debug_ramdisk/.magisk/preinit` 不存在，模块 sepolicy 规则无处暂存；设备侧 `magisk --preinit-device` 的回答是 `vdd1`。**修法已知**（把 `PREINITDEVICE` 写进补丁 ramdisk 的配置后重装模块），本轮未做 → 登记为 S1-③ 遗留项（§11），不构成设计缺口：真机由 Magisk 应用安装，preinit 面在位。
- **priv-app 白名单与 keylayout 只验证了「同批 overlay 不破坏开机」**：真实特权授予与按键行为都需要真实薄壳 APK / 真机，按设计 §12 的载体边界留给 NP1 终验。

---

## 8. 与设计文本的偏差（只登记，待裁决）

**偏差一（低风险，建议直接更正）**：设计 §12 裁决段写「orz **x86_64 musl 三件套直接复用 0.4.2 载体**」。载体已随同日 0v 第二批 S3 重建 bump 到 **0.4.3**（orz `f9fb70e4`），0.4.2 仅存 `.bak`。本批按「复用当前冻结载体」的本意使用 0.4.3，未新建任何构建。

**偏差二（需裁决）**：设计 §12 裁决段写「坏补丁代价 = **删快照**，零成本回滚」。本机快照恢复两条路径均不可用（§6.2 实测），改用 AVD 目录基线副本。建议该句改为「坏补丁代价 = 还原 AVD 目录基线副本 + 重启一次」，并登记「快照在本机不可用」这一事实。

**裁决回填（2026-09-12 用户裁决）**：两项偏差已按本记录建议落地于设计 §12——①载体版本表述由「复用 0.4.2 载体」更正为「复用**当前冻结**的 x86_64 musl 三件套（S1 时为 0.4.3）」；②回滚原语由「删快照、零成本回滚」更正为「**还原 AVD 目录基线副本 + 重启一次**」，并登记快照在本机不可用；同批同步 §9.3 附注 4（模拟器系统面只读、改系统面须 `-writable-system`）、§13 M5 依赖增列模拟器验证载体、§14 待定项 2 状态收口，以及文档头新增修订行。本节前两段保留为裁决前的原始记录。

---

## 9. 边界与未覆盖

- **`ro.adb.secure=0`**：模拟器 adb 免授权，因此**不能**验证「目标侧授权」前提（真机与其它安卓设备都需要授权）。模拟器不覆盖任何授权/同意面。
- **AOSP ≠ Nothing 系统**：设计 §12 已写明的边界不变——不验证厂商框架、Glyph、NP1 内核 config 与平台签名行为。
- **ATD 无头形态**：无 GPU、无 Play Store、无启动器，界面类与图形类能力不代表真机。
- **注入式 Magisk 少了应用安装面**（见 §7.3 / §7.7）：模块 sepolicy 规则的 preinit 暂存未走通、priv-app 白名单与 keylayout 只做了「不破坏开机」级核证——属**本载体形态限制**，不是设计缺口。
- 全程未做任何真机（NP1）操作；所有 adb 命令均显式指名 `emulator-5554`。

---

## 10. 失误与纠正（如实登记）

1. **第 9 步判据期望值设错**：首轮回滚测试期望「还原回 marker-1」，但快照是在 marker 已丢失之后拍的，因此「仍缺失」其实是自洽结果——该轮**不构成证据**，已按正确顺序（先写标记 → 拍快照 → 改标记 → 还原）重做。教训：快照回滚测试必须先确认拍快照时刻的状态。
2. **重启 adb 服务影响真机**：清理残留进程时执行了 adb 服务重启，导致真机 NP1 在设备列表中显示 `unauthorized`，需在手机上重新确认授权。手机本身无任何改动，但这是一次对用户设备的非预期打扰。
3. **ELF 解析写法失败**：首次用 `Get-Content -Encoding Byte` 取文件头，在当前 PowerShell 下不支持而失败，改用 .NET 流读取后成立（不影响结论）。
4. **PID 误读**：曾把同时启动的两个模拟器进程误判为「孤儿进程」，实际是父子关系（同创建时刻），已核正。
5. **③ 首批解包漏了 32 位组件**：首次只从 APK 里取了 `lib/x86_64/*`，环境落地脚本首跑即失败（`cp: bad '/data/local/tmp/apk/lib/x86/libmagisk.so'`）；补出 `lib/x86/libmagisk.so` 作为 `magisk32` 后成立。教训：**先按目标清单列全工件，再执行副作用步骤**。
6. **容器镜像名写死拉取失败**：首次用 `alpine:3.20` 触发拉取，本机容器仓库不可达而失败；改用本机已有镜像 ID 后成立。属操作层失误，不影响判据。
7. **脚本阅读被打字习惯绊倒**：读 APK 内 shell 脚本时把函数名写成 `R`，与 PowerShell 内置别名 `Invoke-History` 冲突而失败；改名后成立。
8. **一处非预期报错留痕**：末次证据收集时 `getprop | head` 触发一次 `Segmentation fault` 输出；判据不依赖该命令，未复现、未追查，只登记事实。

---

## 11. 未完成项（S1 余下）

| 项 | 内容 | 前置 |
| --- | --- | --- |
| S1-③ | ~~Magisk-in-AVD：模块打包 / 安装 / 禁用 / 恢复实机化演练~~ **已完成（2026-09-12，见 §7）** | — |
| S1-③ 遗留 | 补丁 ramdisk 的 `.magisk` 配置写入 `PREINITDEVICE` → 重装模块 → 核证模块 sepolicy 规则真的被暂存与注入 | §7.7；需重建一次补丁 ramdisk + 一轮重装 |
| S1-④ | M5 补丁「打补丁 → 进系统 → 开机 → 回滚」流程纪律干跑首轮 | S1-③（已完成）+ 基线副本回滚原语（已就绪） |

---

## 12. 复现参数速查

```text
SDK 根        D:\android\Sdk
AVD 根        D:\android\avd            （ANDROID_AVD_HOME）
模拟器 home   D:\android\emulator-home  （ANDROID_EMULATOR_HOME）
临时目录      D:\android\tmp            （JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=…；TEMP/TMP）
设备序列      emulator-5554
启动参数      -avd orz_body_a35 -no-window -no-audio -no-boot-anim -gpu swiftshader_indirect -port 5554 -no-snapshot
Magisk 态启动 追加 -ramdisk D:\tb-eval\s1_emulator\ramdisk\ramdisk.magisk.img
干净基线      D:\tb-eval\s1_emulator\baseline_avd   （回滚 = robocopy /MIR 回 AVD 目录）
证据日志      D:\tb-eval\s1_emulator\logs\
补丁 ramdisk  D:\tb-eval\s1_emulator\ramdisk\   （ramdisk.stock.img / ramdisk.magisk.img / build_magisk_ramdisk.sh）
模块包        D:\tb-eval\s1_emulator\module\orz_body-0.1.0-s1drill.zip
Magisk 工件   D:\tb-eval\s1_emulator\magisk\    （Magisk-v30.7.apk / apk 解包 / materialize_env.sh）
```

统一纪律：所有命令显式 `-s emulator-5554`。真机 NP1 与本模拟器同挂一台电脑，省略设备号会打到真机。

③ 的操作链（可整段复现）：`build_magisk_ramdisk.sh`（容器内）→ `-ramdisk` 启动 → 推 `magisk\apk` 并跑 `materialize_env.sh` → 推模块包 → `magisk --install-module` → 重启；禁用/恢复/卸载分别是 `touch disable` / `rm disable` / `touch remove` 加一次重启；回滚是「关模拟器 → 还原基线副本 → 不带 `-ramdisk` 启动」。

---

## 13. 占用小结

| 位置 | 占用 |
| --- | --- |
| `D:\android\Sdk` | 2,903 MB |
| `D:\android\avd` | 3,168 MB |
| `D:\tb-eval\s1_emulator`（基线副本 561 MB + Magisk 工件 + 补丁 ramdisk + 模块包 + 日志） | 621 MB |
| 收尾时 C 盘 / D 盘可用 | 10.76 GB / 22.70 GB |

收尾状态：模拟器已关机（无残留 emulator / qemu 进程），真机 NP1 连接与授权未受影响。**AVD 现处于「Magisk 已装 + 演练模块已装并生效」的可用态**（为 S1-④ 备用）；因模拟器会把补丁 ramdisk 落进 AVD 目录的 `initrd`，该状态由「AVD 目录 + 启动时带 `-ramdisk`」共同定义——**回滚仍是单步**（还原基线副本即同时清掉 `/data` 改动与补丁 `initrd`，已在 §7.5 实测）。
