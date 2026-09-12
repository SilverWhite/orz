# 0y / S1 模拟器验证载体搭建记录（2026-09-12）

> **文档类型**：实施记录（S1 四项全部完成）
> **状态**：S1 ①无头镜像选型搭建、②orz 载体上机冒烟、③Magisk 模块打包 / 安装 / 禁用 / 恢复演练（**含 ③ 遗留项：SELinux 规则注入闭环**）、**④M5 补丁「打补丁 → 进系统 → 开机 → 回滚」流程纪律干跑首轮（含坏补丁救援与策略回滚）全部完成**；载体已收尾回干净基线
> **日期**：2026-09-12
> **范围**：NP1 机械身体集成支线设计 §12「2026-09-12 用户裁决」引入的常设验证载体——模拟器——的搭建与能力判据
> **边界**：本记录只登记载体搭建事实与判据，不修改设计权威；两处 §12 偏差经用户裁决已回填（见 §9），SELinux 撤销边界经项目方裁决已落地设计（见 §8.6 与 [设计 §9.3 附注 5–7](../NP1_ORZ_BODY_INTEGRATION_DESIGN_2026-09-11.md)）。
> **证据**：模拟器运行 stdout/stderr 与设备侧证据落 `D:\tb-eval\s1_emulator\logs\`（③ 遗留项：策略全量 dump 与两组差异 `preinit_rules_{control,injected,diff}.txt`、观测序列 `preinit_ab_summary.txt`；④：`s1d_01…s1d_15` 序列与回滚后策略全量 `preinit_rules_after_rollback.txt`）；干净基线副本落 `D:\tb-eval\s1_emulator\baseline_avd\`；补丁 ramdisk / 模块包 / 容器脚本落 `D:\tb-eval\s1_emulator\{ramdisk,module,bin,magisk}\`；**M5 补丁管线（原厂件 + 产物 + 脚本 + 工具）落 `D:\tb-eval\s1_emulator\pipeline\`**。

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
| `ro.adb.secure` | `0`（边界见 §10） |

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

`S1-CARRIER-02`：回滚语义由「快照」改为「基线副本」，代价仍在零成本量级（一次文件还原 + 一次正常启动），但机制与设计文本不同——登记为**设计偏差，待用户裁决**（§9 偏差二）。

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

- **`sepolicy.rule` 未生效**：安装时 Magisk 报 `- Unable to find preinit dir`。注入式 Magisk 的 `.backup/.magisk` 配置里没有 `PREINITDEVICE`（真机由应用在安装时写入），`/debug_ramdisk/.magisk/preinit` 不存在，模块 sepolicy 规则无处暂存；设备侧 `magisk --preinit-device` 的回答是 `vdd1`。**修法已知**（把 `PREINITDEVICE` 写进补丁 ramdisk 的配置后重装模块）→ **同日第二轮已闭环，见 §7.8**。
- **priv-app 白名单与 keylayout 只验证了「同批 overlay 不破坏开机」**：真实特权授予与按键行为都需要真实薄壳 APK / 真机，按设计 §12 的载体边界留给 NP1 终验。

### 7.8 ③ 遗留项闭环：`PREINITDEVICE` 与 SELinux 规则真正生效（2026-09-12 同日第二轮）

**修法**：重建补丁 ramdisk 时把 preinit 分区写进配置（`build_magisk_ramdisk.sh` 参数化：`PREINITDEVICE=vdd1`，取值来自设备侧 `magisk --preinit-device`，即 `/dev/block/vdd1` → 挂载于 `/metadata`），重新启动后 Magisk 建出 `.magisk/preinit -> /metadata/watchdog/magisk`；此后安装模块不再报错，规则写入 `/metadata/watchdog/magisk/sepolicy.rule`（上下文 `u:object_r:watchdog_metadata_file:s0`），`magiskinit` 开机按该暂存文件注入内核策略。

**判据（可复核）**：以 `/data/adb/magisk/magiskpolicy --print-rules`（打印当前 live 策略全部规则）与 `/sys/fs/selinux/policy` 的 SHA256 为观测面，探针规则取 `allow shell magisk process { getsched }`（选它的理由：对照组里 shell→magisk 只有一条 `unix_stream_socket` 规则，该形态必然不存在，因而可归因）。

| # | 引导 `boot_id` | 模块 | 暂存文件 | 策略哈希（前 8） | 规则在否 | 说明 |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `205b8a47…` | 启用 | 不存在 | `8242a06d` | 否 | 对照组：旧 ramdisk（无 `PREINITDEVICE`），规则未暂存 |
| 2 | `65963a19…` | 启用 | 安装时写入 | `8242a06d` | 否 | 新 ramdisk：preinit 面就位（`.magisk/preinit` 软链），本轮内装模块 |
| 3 | `babf1da8…` | 启用 | 存在 | `d2365b61` | **是** | 重启后规则进入内核策略 |
| 4 | `153e915c…` | **禁用** | 仍在 | `d2365b61` | **是** | **【新发现】**禁用只摘 overlay，规则不撤销 |
| 5 | `7c86f1ff…` | 禁用 | 已删除 | `8242a06d` | 否 | 清掉暂存文件后重启，策略哈希**精确回到**对照组 |
| 6 | `1f530b87…` | 启用 | 重装写入 | `d2365b61` | 是 | 清 `disable` + 重装（重写暂存）→ 恢复注入态（收尾态） |

**最强判据**：对照组与注入组的**规则全量差异恰好一条**——`=> allow shell magisk process { getsched }`（两份 dump 各 42,4xx 行，落 `logs\preinit_rules_{control,injected,diff}.txt`）。

**新发现（登记待用户裁决）**：**「禁用模块 + 重启」不撤销已注入的 SELinux 规则**——暂存文件只在模块**安装/更新**时重写（`copy_preinit_files` 跳过带 `disable`/`remove`/`update` 标志的模块），开机注入读的是暂存文件，因此第 4 行实测「overlay 已摘、规则仍在、策略哈希不变」。彻底撤销需**重写暂存**（安装/更新任一模块触发）或**清掉暂存文件后重启**。残留物是加性 allow 规则，不改变 §10.3 层 1 对框架代码（overlay / 脚本）的回滚效果，但设计「回滚 = 禁用模块 + 重启」对本条不完整 → 已同步设计 §9.3 登记附注 5 与 §10.3 层 1 附注。

**工件（本批新增）**：

| 工件 | 字节 | SHA256 |
| --- | --- | --- |
| `ramdisk.magisk.preinit.img`（写入 `PREINITDEVICE=vdd1`） | 2,834,491 | `ac0c532f5734c5cba5fef6d1861b2899986de7a723d8b9b2e4ab2d551b59628f` |
| `orz_body-0.1.1-s1drill.zip`（探针规则，versionCode 2） | 3,653 | `3f5ace575df1f136f73be37bfe0cad0506e741c5d486c077fc55d6912e1110e1` |

观测序列另存 `logs\preinit_ab_summary.txt`；本轮共 6 次开机，每次 `boot_id` 互不相同。

---

## 8. S1-④ M5 补丁「打补丁 → 进系统 → 开机 → 回滚」流程纪律干跑首轮（2026-09-12）

**本轮定位**：把设计 §10.4 的「PC 侧补丁管线（反编译 → 改 → 重打包 → 重编 dex）」与 §10.3 层 1 / §9.3 附注 5 的回滚口径，在模拟器上**整条走一遍**，并把「坏补丁 → 起不来 → 救援」这一最坏档跑出实测形态。**边界**：本轮补丁是**结构最小、无语义**的加性补丁（既有类上新增常量与方法 + 一个探针类），只验证管线与纪律，不验证 M5 的框架语义（见 §10）。

### 8.1 判据与结果一览

| # | 判据 | 结果 |
| --- | --- | --- |
| 1 | 管线可产出**可复现**产物（两次独立构建哈希逐字相同） | ✅ |
| 2 | 补丁面**最小且可逐条字节核对**（smali 树差异恰好一处且全为新增行；重打包后除目标 dex 外逐条字节同一、MANIFEST 逐字节相同） | ✅ |
| 3 | 产物满足平台装载前提（ZIP 条目数据 4 字节对齐） | ✅（首版**不满足**，见 §8.3 / §11-12；修正后告警清零） |
| 4 | 补丁**落位真实**且 systemless（设备路径哈希 = 产物哈希；只读 overlay 挂载；系统分区零写入） | ✅ |
| 5 | 补丁**在运行期被采用**（补丁代码从 live 路径加载并执行；被改写既有类的新增成员可见可调） | ✅ |
| 6 | 坏补丁 → **起不来** → 救援 → 恢复 | ✅（小 jar 档**不成立**——系统照常开机，见 §8.4；换成系统服务自身依赖的 jar 后成立，见 §8.5–8.6） |
| 7 | 回滚完整（含内核策略面） | ✅（两件回原厂哈希、挂载消失、脚本停跑；策略哈希回对照 + 规则全量 0 行差异） |

### 8.2 补丁管线（工具 / 目标件 / 产物 / 最小化核对）

**工具链**（全部官方来源，落 `pipeline\tools\`）：反编译与重编 `baksmali`/`smali` **3.0.7**（Google 官方 Maven `dl.google.com/android/maven2`，依赖 guava 31.1-android / jcommander 1.64 / antlr 3.5.2）；dex 编译 **d8 8.9.27**（Android SDK cmdline-tools 自带，未另装 build-tools）；`javac` 用宿主 JDK 21（`--release 11`）。

**目标件**：`/system/framework/android.hidl.base-V1.0-java.jar`（**12,890 B**，原厂 SHA256 `e60b7e33…`，dex 版本 039、5 个类）——选它做"好补丁"是因为体量小、管线耗时可控且**在系统开机路径的映射清单内**；选它做"坏补丁一"是因为它**不是开机必需件**（见 §8.4）。

**补丁内容**（真实改写既有类 + 新增类）：`android.hidl.base.V1_0.DebugInfo$Architecture` 上**新增**一个静态常量与一个静态方法（既有方法体零改动）；另新增探针类 `orz.body.m5.ProbeMain`（新 dex 条目 `classes2.dex`）。

**产物**（`pipeline\out\`）：

| 产物 | 字节 | SHA256 |
| --- | --- | --- |
| 好补丁 `orc-m5-good-a.jar`（与 `-b` 两次独立构建**逐字相同**） | 16,073 | `1e8a33ab6f1649d27e3a44a307b7a8578892c2bfde67107f89581d22a3bb9054` |
| 坏补丁（小 jar 档）`orc-m5-bad.jar` | 16,073 | `061009d883deb41f9eae4c5eb5466c5e8751687a4142b5c6488b7db742f88988` |
| 坏补丁（系统服务档）`orc-m5-badservices.jar` | 21,445,455 | `05a0fa2f32f7baa9d12506f4ba1253f13f28564bb57b922ecf7d546989f0a1b7` |

**最小化核对**：`smali_stock` 与 `smali_patched` 全树比对 → **只有 1 个文件不同**（目标类），且该文件差异**全为新增行**；对补丁 dex 重新反编译做往返比对 → 与补丁后 smali 树**唯一差异是注释与空行**（dex 不承载注释）；重打包后 `classes.dex` 被替换、`META-INF/MANIFEST.MF` **逐字节相同**、`unzip -t` 通过。

**模块包**（`module\`）：好补丁 `orz_body-0.2.2-s1drill.zip`（11,685 B / `7ca9a539…`，overlay = 好补丁件）；坏补丁 `orz_body-0.2.3-baddrill.zip`（11,703 B / `d292a280…`）与 `orz_body-0.3.0-badservices.zip`（9,119,260 B / `c8b67942…`）。

### 8.3 好补丁：落位 → 开机 → 生效核证

装模块（`magisk --install-module`）→ 重启 → `boot_id=1fa2e2d3…`、`boot_completed=1`：

- **落位**：设备上该路径 SHA256 = `1e8a33ab…`（= 产物哈希）；`unzip -p <live 路径> classes2.dex | grep -c 标记` = 2。
- **systemless**：`mount` 显示 `/dev/block/dm-43 on /system/framework/android.hidl.base-V1.0-java.jar type ext4 (ro,seclabel,noatime,…,errors=panic)`——模块件以**只读挂载**覆盖该路径，**系统分区零写入**（与 §7.6 事实 2 一致）。
- **运行期采用**：`CLASSPATH=<live 路径> app_process64 … orz.body.m5.ProbeMain` 输出 `marker=ORZ-BODY-M5-DRYRUN-MARKER-9F31`、`patched_field=…`、`patched_method=…`，`exit=0` ⇒ 探针类**与被改写的既有类**都从 live 路径加载并执行、新增成员可见（未被启动镜像遮蔽）。
- **系统健康**：`system_server` / `zygote64` 在位；进程数 340、包数 168、`dumpsys activity` 正常输出。
- **对齐面**：首版产物 ZIP 条目数据未对齐，ART 报 `Can't mmap dex file … please zipalign to 4 bytes`（6 行）并退化为"解压装载"；加入 4 字节对齐后（条目数据偏移全部 `mod4=0`，与原厂件同对齐纪律）**告警清零**（`warn_lines=0`）。该缺陷与修正登记于 §11 第 12 条。

### 8.4 坏补丁（小 jar 档）：**没有**导致"起不来"——一条对 M5 有价值的反例

同一管线把目标件 `classes.dex` 打坏（其余条目逐字保留、仍 4 字节对齐）后装模块重启：**系统照常起完**（`boot_completed=1`，24.3 s，`boot_id=cc693a5d…`）。故障面取证：

- 显式加载该 live 路径：`Failure to verify dex file '/system/framework/android.hidl.base-V1.0-java.jar': Bad checksum (7111b0f5, expected 952435d1)` → 类找不到 → `SIGABRT`（进程级）。
- 开机路径：ART 对该件**记告警后丢弃**——`grep -c android.hidl.base /proc/<zygote64>/maps` = **0**（对照：好补丁轮该件在映射内），系统其余部分不受影响。

**结论（登记为 M5 纪律）**：坏框架补丁的后果**不必然是"起不来"**，也可能是**该件在运行期被静默丢弃**（只留一条 logcat 告警）。因此 M5 的验收**不能只核"文件落位"**，必须单列「补丁件在运行期真的被采用」的判据——本轮以「补丁代码可加载执行 + 被改写成员可见 + 与运行期映射面一致」三项收口。

### 8.5 坏补丁（系统服务档）：真正的"起不来"档

改用系统服务自身依赖的 `/system/framework/services.jar`（21,446,365 B，原厂 SHA256 `1a921dbd…`，159 个条目、3 个 dex），只把它的 `classes.dex` 打坏（其余条目逐字保留、4 字节对齐）→ 产物 `05a0fa2f…` → 装模块 `0.3.0-badservices` 后重启：

- **`boot_completed` 在 168.1 s 内始终为空**（`boot_id=7a824f56…`）。
- 故障形态：`zygote64` 启动 `system_server` 时在 `DexPathList.makeDexElements → DexFile.openDexFileNative` 处 `SIGABRT`（signal 6）；日志逐条为 `Zygote failed to write to system_server FD: Connection refused`、`Process … exited due to signal 6 (Aborted)`，`system_server` **反复重启**。
- 该状态下 **adb 仍然可用**（adbd 属早期启动面）：`adb shell` 得 `uid=2000(shell)`，`adb root` 后 `uid=0`，`/data/adb/modules/orz_body/` 可直接操作。**载体差异登记**：真机在系统起不来时未必有 adb，其等价通道是**音量下 → fastboot**（M0 已实测）。

### 8.6 救援 → 策略回滚 → 整体回滚（判据表）

| 步骤 | 操作 | 判据 | 结果 |
| --- | --- | --- | --- |
| 救援① | `touch /data/adb/modules/orz_body/disable` + 重启 | `boot_completed=1`（`boot_id=f7a0baa2…`）；两件均回原厂哈希（`services.jar`=`1a921dbd…`、hidl jar=`e60b7e33…`）；overlay 挂载消失；`service.marker` 未刷新（**模块脚本本轮不执行**） | ✅ |
| （顺带实证） | 同上（模块已禁用态） | `/sys/fs/selinux/policy` = `d2365b61…` **仍是注入态**、暂存文件仍在 ⇒ §7.8「禁用不撤销规则」在**故障救援场景二次成立** | ✅ |
| 策略回滚（面 B） | `rm -f /metadata/watchdog/magisk/sepolicy.rule` + 重启 | 策略哈希精确回到对照值 `8242a06d…`；`magiskpolicy --print-rules` 全量 **42,409 行**与此前对照 dump **0 行差异**（`logs\preinit_rules_after_rollback.txt`） | ✅ |
| 整体回滚 | 关模拟器 → 还原 AVD 基线副本 → **不带** `-ramdisk` 启动 | `boot_completed=1`（26.4 s，`boot_id=cb365959…`）；`command -v magisk` 空、`/debug_ramdisk` 与 `/data/adb` 均不存在、两件 = 原厂哈希、无 overlay 挂载 | ✅ |

形态差异说明：干净基线（无 Magisk）的策略哈希 `d1749c5d…` 与「Magisk 在位、无模块规则」对照值 `8242a06d…` **不同**——属正常形态差异（Magisk 本体也会调整部分策略面）；面 B 的对照口径固定用后者。

### 8.7 本轮产出的纪律（对设计与实施）

1. **管线三条硬要求**：产物冻结哈希且**两次构建一致**；改动面**可逐条字节核对**（smali 树 + ZIP 条目双面）；**ZIP 条目数据 4 字节对齐**（否则 ART 退化为解压装载，且属可避免的缺陷）。
2. **新增验收判据**：M5 补丁必须核「**运行期被采用**」——见 §8.4 反例；只核文件落位会把"静默丢弃"放过去。
3. **回滚两步**：面 A 框架代码 + 面 B 内核策略（承接 §7.8，设计侧已裁决落地）；策略面判据 = 哈希回对照值 + 规则全量 0 行差异。
4. **坏补丁代价口径照旧**：起不来 → **还原 AVD 目录基线副本 + 重启一次**（本轮实测 26.4 s 起完）；模拟器上另有一条更便宜的通道（禁用标志 + 重启），真机侧对应物理键通道。

---

## 9. 与设计文本的偏差（裁决前记录 + 裁决回填）

**偏差一（低风险，建议直接更正）**：设计 §12 裁决段写「orz **x86_64 musl 三件套直接复用 0.4.2 载体**」。载体已随同日 0v 第二批 S3 重建 bump 到 **0.4.3**（orz `f9fb70e4`），0.4.2 仅存 `.bak`。本批按「复用当前冻结载体」的本意使用 0.4.3，未新建任何构建。

**偏差二（需裁决）**：设计 §12 裁决段写「坏补丁代价 = **删快照**，零成本回滚」。本机快照恢复两条路径均不可用（§6.2 实测），改用 AVD 目录基线副本。建议该句改为「坏补丁代价 = 还原 AVD 目录基线副本 + 重启一次」，并登记「快照在本机不可用」这一事实。

**裁决回填（2026-09-12 用户裁决）**：两项偏差已按本记录建议落地于设计 §12——①载体版本表述由「复用 0.4.2 载体」更正为「复用**当前冻结**的 x86_64 musl 三件套（S1 时为 0.4.3）」；②回滚原语由「删快照、零成本回滚」更正为「**还原 AVD 目录基线副本 + 重启一次**」，并登记快照在本机不可用；同批同步 §9.3 附注 4（模拟器系统面只读、改系统面须 `-writable-system`）、§13 M5 依赖增列模拟器验证载体、§14 待定项 2 状态收口，以及文档头新增修订行。本节前两段保留为裁决前的原始记录。

---

## 10. 边界与未覆盖

- **`ro.adb.secure=0`**：模拟器 adb 免授权，因此**不能**验证「目标侧授权」前提（真机与其它安卓设备都需要授权）。模拟器不覆盖任何授权/同意面。
- **AOSP ≠ Nothing 系统**：设计 §12 已写明的边界不变——不验证厂商框架、Glyph、NP1 内核 config 与平台签名行为。
- **ATD 无头形态**：无 GPU、无 Play Store、无启动器，界面类与图形类能力不代表真机。
- **注入式 Magisk 少了应用安装面**（见 §7.3 / §7.7 / §7.8）：模块 sepolicy 规则的 preinit 暂存需手动补 `PREINITDEVICE`（已闭环）、priv-app 白名单与 keylayout 仍只做了「不破坏开机」级核证——属**本载体形态限制**，不是设计缺口。
- **SELinux 规则的撤销不完全跟随模块禁用**（§7.8 新发现）：设计侧「回滚 = 禁用模块 + 重启」对本条不完整——**已裁决收口**（2026-09-12）：回滚拆成面 A（框架代码）+ 面 B（内核策略），面 B 判据 = 策略哈希回对照值 + 规则全量 0 行差异；见设计 §9.3 附注 5–7 与 §10.3 层 1，S1-④ 二次实证见 §8.6。
- **S1-④ 的边界（本轮未覆盖面）**：①**补丁语义未验证**——本轮补丁是无语义的加性改动，只验证管线与纪律，不代表 M5 真实改动（如 `RTH-B-*` 上下文前摄）可用；②**M5 真实目标档未做正补丁**——`services.jar` 体量与耗时（3 个 dex、21 MB）不属首轮范围，本轮只把它用于"坏补丁"档；③**行为级证据限于探针类**（"补丁代码确实执行"），不构成对框架语义正确性的断言；④**故障态下的救援通道属载体形态**——模拟器上 adb 在系统起不来时仍可用，真机侧对应通道是音量下 → fastboot（M0 已验），不能由本轮推出"真机坏补丁也能靠 adb 救"。
- 全程未做任何真机（NP1）操作；所有 adb 命令均显式指名 `emulator-5554`。

---

## 11. 失误与纠正（如实登记）

1. **第 9 步判据期望值设错**：首轮回滚测试期望「还原回 marker-1」，但快照是在 marker 已丢失之后拍的，因此「仍缺失」其实是自洽结果——该轮**不构成证据**，已按正确顺序（先写标记 → 拍快照 → 改标记 → 还原）重做。教训：快照回滚测试必须先确认拍快照时刻的状态。
2. **重启 adb 服务影响真机**：清理残留进程时执行了 adb 服务重启，导致真机 NP1 在设备列表中显示 `unauthorized`，需在手机上重新确认授权。手机本身无任何改动，但这是一次对用户设备的非预期打扰。
3. **ELF 解析写法失败**：首次用 `Get-Content -Encoding Byte` 取文件头，在当前 PowerShell 下不支持而失败，改用 .NET 流读取后成立（不影响结论）。
4. **PID 误读**：曾把同时启动的两个模拟器进程误判为「孤儿进程」，实际是父子关系（同创建时刻），已核正。
5. **③ 首批解包漏了 32 位组件**：首次只从 APK 里取了 `lib/x86_64/*`，环境落地脚本首跑即失败（`cp: bad '/data/local/tmp/apk/lib/x86/libmagisk.so'`）；补出 `lib/x86/libmagisk.so` 作为 `magisk32` 后成立。教训：**先按目标清单列全工件，再执行副作用步骤**。
6. **容器镜像名写死拉取失败**：首次用 `alpine:3.20` 触发拉取，本机容器仓库不可达而失败；改用本机已有镜像 ID 后成立。属操作层失误，不影响判据。
7. **脚本阅读被打字习惯绊倒**：读 APK 内 shell 脚本时把函数名写成 `R`，与 PowerShell 内置别名 `Invoke-History` 冲突而失败；改名后成立。
8. **一处非预期报错留痕**：末次证据收集时 `getprop | head` 触发一次 `Segmentation fault` 输出；判据不依赖该命令，未复现、未追查，只登记事实。
9. **遗留项轮的一处预期被实测否定**：开工时的先验是「禁用模块 + 重启会把 SELinux 规则一并撤销」（设计 §10.3 层 1 的写法），实测被否定（§7.8 第 4 行：规则仍在、策略哈希不变）。教训：**加性策略规则的撤销要单独取证，不能从「模块禁用」推**；同时说明 §7.7 初版把该问题写成「不构成设计缺口」过早——现已在 §7.8 与设计附注 5 更正为「设计文本对本条不完整，待裁决」。
10. **容器命令引号层数没算清**：首次用一段带 `for ... done` 的嵌套引号命令查二进制字符串，经 PowerShell → docker → sh 三层后解析失败；改写成两条简单命令后成立。属操作层失误，不影响判据。
11. **一次日志落盘为空**：遗留项第二轮的模拟器 stdout 重定向文件为空（0 B），未追查原因；该轮判据全部来自设备侧取证（策略哈希、规则 dump、文件属性），不依赖该日志。登记为证据留痕缺口。
12. **重打包产物丢了 ZIP 对齐（S1-④）**：首版重打包按「逐条原样复制」写 ZIP，条目数据偏移变成 `mod4≠0`（原厂件是对齐的），ART 因此在加载时报 `Can't mmap dex file … please zipalign to 4 bytes` 并退化为"解压装载"。**这条缺陷是靠设备侧告警发现的，不是我们自己看出来的**——说明"产物冻结"必须把**平台装载前提（对齐）**纳入核对项。修正 = 写 extra 字段补齐对齐（全部 `mod4=0`），复测告警清零。教训：**打包脚本本身就会引入质量退化；产物核对不能只看内容哈希**。
13. **探针首版自身有空指针（S1-④）**：探针读取 `getProtectionDomain().getCodeSource()` 时未做空值保护，`app_process` 在第一行输出后即 `FATAL EXCEPTION`（`exit=137`），一度看起来像"系统杀掉了补丁进程"。按 logcat 的 `at orz.body.m5.ProbeMain.main(ProbeMain.java:15)` 定位后改为空值保护，复跑 `exit=0` 并取到全部判据。教训：**探针自身必须写成不会崩的形式**，否则会污染对系统行为的判断。
14. **模块包里的 overlay 落点写错（S1-④）**：首次打包把 overlay 落成**产物名**（`orc-m5-good-a.jar`），而 Magisk 按路径覆盖，等于没有命中 `/system/framework/android.hidl.base-V1.0-java.jar`。靠打包脚本自报的条目清单发现并修正（overlay 必须落成**目标原名**）。教训：**模块 overlay 落点要按目标路径逐条核对**，不能只看"打包成功"。
15. **d8 输出目录必须预先存在（S1-④）**：首调报 `Invalid output: …\dex2 / Output must be .zip/.jar or an existing directory`，建目录后成立。属操作层失误，不影响判据。
16. **「坏补丁 → 起不来」的先验被实测修正（S1-④）**：开工时的先验是"打坏任一个框架 jar 就等于系统起不来"，实测在小 jar 上**不成立**（ART 记告警后丢弃该件，系统照常起完，见 §8.4）；改用**开机必需件**（系统服务自身依赖的 `services.jar`）才复现出真正的"起不来"档（§8.5）。教训：**故障注入点要按依赖关系选（必需件 vs 非必需件），不能按"是不是框架件"选**。

---

## 12. S1 项目状态（四项全部完成）

| 项 | 内容 | 前置 |
| --- | --- | --- |
| S1-③ | ~~Magisk-in-AVD：模块打包 / 安装 / 禁用 / 恢复实机化演练~~ **已完成（2026-09-12，见 §7）** | — |
| S1-③ 遗留 | ~~补丁 ramdisk 的 `.magisk` 配置写入 `PREINITDEVICE` → 重装模块 → 核证模块 sepolicy 规则真的被暂存与注入~~ **已完成（2026-09-12 同日，见 §7.8；含新发现“禁用不撤销已注入规则”）** | — |
| S1-④ | ~~M5 补丁「打补丁 → 进系统 → 开机 → 回滚」流程纪律干跑首轮~~ **已完成（2026-09-12，见 §8：好补丁全链路 + 坏补丁两档 + 救援 + 策略回滚 + 整体回滚）** | — |

**S1 无余下项**。下一阶段（另批、需用户放行）：**M1 前置** = 设计 §14.1 接口定义（事件 schema / 动作契约 / 策略注册表形态）与 **aarch64 musl 三件套重建**；M5 侧则可按 §8.7 的三条管线纪律起正式补丁批。

---

## 13. 复现参数速查

```text
SDK 根        D:\android\Sdk
AVD 根        D:\android\avd            （ANDROID_AVD_HOME）
模拟器 home   D:\android\emulator-home  （ANDROID_EMULATOR_HOME）
临时目录      D:\android\tmp            （JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=…；TEMP/TMP）
设备序列      emulator-5554
启动参数      -avd orz_body_a35 -no-window -no-audio -no-boot-anim -gpu swiftshader_indirect -port 5554 -no-snapshot
Magisk 态启动 追加 -ramdisk D:\tb-eval\s1_emulator\ramdisk\ramdisk.magisk.preinit.img（含 PREINITDEVICE=vdd1，现行件；旧件 ramdisk.magisk.img 无该键，保留作对照）
干净基线      D:\tb-eval\s1_emulator\baseline_avd   （回滚 = robocopy /MIR 回 AVD 目录）
证据日志      D:\tb-eval\s1_emulator\logs\
补丁 ramdisk  D:\tb-eval\s1_emulator\ramdisk\   （ramdisk.stock.img / ramdisk.magisk.img / ramdisk.magisk.preinit.img / build_magisk_ramdisk.sh）
模块包        D:\tb-eval\s1_emulator\module\orz_body-0.1.1-s1drill.zip（探针规则版；0.1.0 保留作对照）
Magisk 工件   D:\tb-eval\s1_emulator\magisk\    （Magisk-v30.7.apk / apk 解包 / materialize_env.sh）
补丁管线      D:\tb-eval\s1_emulator\pipeline\ （stock\ 原厂件 · out\ 产物 · src\ 探针源码 · tools\ 反编译件 · build.ps1 / repack.py / repack_bad.py / mkmodule.py / extract_dex.py）
```

统一纪律：所有命令显式 `-s emulator-5554`。真机 NP1 与本模拟器同挂一台电脑，省略设备号会打到真机。

③ 的操作链（可整段复现）：`build_magisk_ramdisk.sh`（容器内）→ `-ramdisk` 启动 → 推 `magisk\apk` 并跑 `materialize_env.sh` → 推模块包 → `magisk --install-module` → 重启；禁用/恢复/卸载分别是 `touch disable` / `rm disable` / `touch remove` 加一次重启；回滚是「关模拟器 → 还原基线副本 → 不带 `-ramdisk` 启动」。

④ 的操作链（可整段复现）：`build.ps1 -Mode good`（反编译 → 改既有类 → 重编 dex → javac+d8 编探针 → 重打包，两次跑哈希应逐字相同）→ `mkmodule.py`（overlay 必须落成**目标原名**）→ 推模块包 → `magisk --install-module` → 重启 → 取证（路径哈希 / `mount` / `app_process64` 执行探针 / 系统健康）；坏补丁档分别是 `-Mode bad`（小 jar 档，实测**不起不来**）与 `repack_bad.py` + `mkmodule.py`（系统服务档，`services.jar` 只坏 `classes.dex`，实测起不来）；救援 = `touch disable` + 重启；**策略回滚 = `rm -f /metadata/watchdog/magisk/sepolicy.rule` + 重启**（判据：策略哈希回 `8242a06d…` 且规则全量与对照 dump 0 行差异）；整体回滚同 ③。**注意**：`adb root` 在每次重启后需重做，否则 `/data/adb` 与 `/sys/fs/selinux/policy` 读不到。

---

## 14. 占用小结

| 位置 | 占用 |
| --- | --- |
| `D:\android\Sdk` | 2,903 MB（未装 build-tools：d8 由 cmdline-tools 自带） |
| `D:\android\avd` | 3,136 MB |
| `D:\tb-eval\s1_emulator`（基线副本 561 MB + Magisk 工件 + 补丁 ramdisk ×3 + 模块包 ×4 + 日志含三份策略 dump + **M5 补丁管线 49 MB**） | 691 MB |
| 收尾时 C 盘 / D 盘可用 | 10.5 GB / 19.6 GB |

收尾状态：模拟器已关机（无残留 emulator / qemu 进程），真机 NP1 连接与授权未受影响。**S1-④ 收尾态 = 干净基线**（关模拟器 → 还原 AVD 基线副本 → 不带 `-ramdisk` 启动并核验：`magisk` 命令不存在、`/debug_ramdisk` 与 `/data/adb` 不存在、两件框架 jar = 原厂哈希、无 overlay 挂载）——即 ③ 建立的「Magisk 已装 + 模块已装 + 规则已注入」那套现成态**已被本轮整体回滚清掉**；若要复跑 ③/④ 的 Magisk 侧，按 §13 的 ③ 操作链从 `-ramdisk` 启动起重建（`materialize_env.sh` 一步不可省）。因模拟器会把补丁 ramdisk 落进 AVD 目录的 `initrd`，该状态由「AVD 目录 + 启动时带 `-ramdisk`」共同定义——**回滚仍是单步**（还原基线副本即同时清掉 `/data` 改动与补丁 `initrd`，§7.5 与 §8.6 两次实测）。
