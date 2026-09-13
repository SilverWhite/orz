# 0y NP1 S1 验证载体定案与入账（模拟器常设载体，2026-09-13）

> **类型**：S1 定案记录 + 盘面证据入账（补记——盘面工作已于 2026-09-12 完成，账本此前只登记为「S1 载体搭建待放行」，本批把形态、证据、哈希与未完成项一次落账）。
> **范围**：0y NP1 的 §14.2「模拟器常设验证载体」S1 搭建面（载体形态 / root 与模块承载 / M5 补丁流程纪律干跑）。
> **口径来源（不回改）**：设计 [`NP1_ORZ_BODY_INTEGRATION_DESIGN_2026-09-11`](../NP1_ORZ_BODY_INTEGRATION_DESIGN_2026-09-11.md) §9.3（全模块化承载）/ §12（验证策略 + §14.2 裁决段）/ §13（里程碑 M0–M8）。
> **入口**：BACKLOG **0y** / TODO **P0-0y** / 索引 `AUTH-NP1-BODY-INTEGRATION` + `0Y-S1-EMULATOR-CARRIER`。
> **证据根**：`D:\tb-eval\s1_emulator\`（本地，691 MB / 59 份日志 12.3 MB / 16 次引导会话；不入仓、不入 manifest、不进 Release）。
> **边界**：本批**零 orz 源码改动、零 ADR 改动**；只做「形态定案 + 证据入账 + 缺口登记」。模拟器验证的是**我们自己的代码**，不覆盖厂商框架、Glyph 硬件、NP1 内核 config 与平台签名（设计 §12）。

## 1. 定案：S1 载体形态

### 1.1 设备面（AVD）

| 项 | 定案值 |
|---|---|
| AVD 名 | `orz_body_a35`（注册件 `D:\android\avd\orz_body_a35.ini`，`target=android-35`，活体目录 `D:\android\avd\orz_body_a35.avd`） |
| 系统镜像 | `system-images\android-35\aosp_atd\x86_64`（**ATD 自动化测试镜像**，即设计 §12 技术形态「无头 x86_64 + ATD 类」） |
| 冻结副本 | `D:\tb-eval\s1_emulator\baseline_avd\`（`config.ini` sha256 `541a6303…`，含 `config.ini.s1-backup` 对照） |
| 启动 argv | `emulator.exe -avd orz_body_a35 -no-window -no-audio -no-boot-anim -gpu swiftshader_indirect -port 5554 -no-snapshot -wipe-data`（实录见 `baseline_avd\emu-launch-params.txt`） |
| 模拟器版本 | `37.1.11.0 (build 15917651)`，graphics backend `gfxstream` |
| 资源形态 | `hw.ramSize=2G`（启动日志自动抬至 2560 MB）、`hw.cpu.ncore=4`、`data` 分区 6 GiB、`abi.type=x86_64` |
| 机内事实 | `ro.product.device=emu64x`、`release=15`、`sdk=35`、内核 `6.6.30-android15-8-gdd9c02ccfe27`（x86_64，Toybox）（`logs\s1c_01_boot_env.txt`） |

- **无头 + 端口固定 5554**：跑批/干跑期间不抢前台，所有交互走 `adb`；`-no-snapshot` 保证每次引导都是干净冷启（判据可复现的前提）。
- **载体为常设**：AVD 保留在 `D:\android\avd\`，冻结副本供「回到 baseline」使用（坏补丁代价 = 删/回滚快照，见 §2.4）。

### 1.2 Root 与持久面（Magisk 进 AVD 的方法）

| 步 | 做法 | 制品 |
|---|---|---|
| 1 | **ramdisk 直注**（模拟器直接 `-ramdisk` 消费，无需 boot 镜像封装）：`magiskboot decompress` → 注入 `init`(magiskinit) + `overlay.d/sbin/{magisk,stub,init-ld}.xz` + `.backup/.magisk`（`patch` + `backup` 指令）→ `compress=lz4_legacy` | `ramdisk\build_magisk_ramdisk.sh` |
| 2 | **持久环境落盘**（ramdisk-only 注入不做这步，等价于 Magisk app 首次运行）：apk 内 `lib/x86_64|lib/x86` 与 `assets/*` 拷到 `/data/adb/magisk`，执 `--restorecon` | `magisk\materialize_env.sh` |
| 3 | 版本核证 | `magisk --version` = `30.7:MAGISK:D`（versionCode 30700），apk `Magisk-v30.7.apk` |

- 无 `PREINITDEVICE` 的 ramdisk 上装模块会报 `Unable to find preinit dir`（sepolicy 规则无法暂存）；**定案用带 `PREINITDEVICE=vdd1` 的版本**（见 §2.3）。

### 1.3 模块承载面（设计 §9.3 五面全命中）

| 设计元素 | S1 载体形态 | 证据 |
|---|---|---|
| systemless overlay | `system/etc/orz-body/overlay-marker.conf`、`system/etc/permissions/privapp-permissions-orz-body.xml`、`system/usr/keylayout/orz-body.kl` 落 `/system`，底层为 magisk tmpfs（`ro`） | `s1c_03_overlay.txt` / `s1c_06` / `s1c_07_final_state.txt` |
| `service.d` 常驻拉起 | `post-fs-data.sh`（早阶段 marker）+ `service.sh`（late_start 起心跳守护） | `s1c_04_boot_markers.txt`（两阶段 marker 齐；`heartbeat n=1..8`，**`ppid=1`** ⇒ 进程脱离引导壳存活） |
| priv-app + 白名单同批 | 白名单 XML（占位包 `net.orz.body.shell`——镜像内无此包，本件只验「白名单与薄壳同批打包」这条承载路径） | `s1c_06_privapp_and_keylayout.txt` |
| keylayout 覆盖 | `orz-body.kl`：`key 212 ASSIST` / `key 213 VOICE_ASSIST`（物理键兜底载体） | 同上 |
| `sepolicy.rule` | 注入探针规则 `allow shell magisk process { getsched }`；安装时写入 `/metadata/watchdog/magisk/sepolicy.rule`（context `u:object_r:watchdog_metadata_file:s0`） | `s1c_08_preinit_state.txt` |

- 模块 `orz_body`（`name=ORZ Body Layer (S1 drill)`），版本序列 `0.1.0 → 0.1.1 → 0.2.0 → 0.2.1 → 0.2.2 → 0.2.3 → 0.3.0`（模块件清单见 §3）。
- 判据：`post-fs-data`/`service` 两阶段 marker 在同一 `boot_id` 下都落，且守护进程 `ppid=1` ⇒ **"模块承载的常驻进程"这条 BODY-HOST 依赖在模拟器上成立**。

### 1.4 M5 补丁流程纪律干跑（流水线形态）

**流水线（可复现入口 `pipeline\build.ps1`，六步）**：

1. 冻结原厂 jar → 抽 `classes.dex`（`extract_dex.py`）；
2. `baksmali` 反汇编；
3. **加性补丁**——在既有类 `android/hidl/base/V1_0/DebugInfo$Architecture` 上追加 `.field public static final ORZ_BODY_M5_MARKER` + `.method orzBodyM5Marker()`（只加不改，最小破坏面）；
4. `smali` 重组（`-a 29`）；
5. 编译**反射探针** `ProbeMain.java`（`javac --release 11` + `d8 --min-api 29`）；
6. 重打包（`repack.py`）：条目 4 字节对齐（extra id `0xd935`，等价 zipalign）、`META-INF/MANIFEST.MF` 字节等同、`testzip` 通过 ⇒ 再经 `mkmodule.py` 打成**确定性模块 zip**（固定 `date_time`、显式权限位）。

- **目标选择有据**：先做 jar 装载映射（`s1d_02_jar_map.txt`）——`services.jar` **只被 `system_server` 装载**；`android.hidl.base-V1.0-java.jar` 被 `zygote64` / `system_server` / `webview_zygote` 装载。故**坏补丁选前者**（影响面最大、最真）、**好补丁选后者**（可读回、代价小）。
- 干跑序列与结局见 §2.4；`bad` 模式 = 破坏 `classes.dex` `0x40` 起 512 字节（`repack.py` / `repack_bad.py`）。

## 2. 盘面证据（实测）

### 2.1 引导与 root 基线

- Magisk `30.7`（`30.7:MAGISK:D`）；`/debug_ramdisk` 为 magisk tmpfs；`/data/adb/modules/orz_body` 就位（`s1c_02_module_state.txt`）。
- overlay 生效形态：`magisk on /system/etc type tmpfs (ro)`、`magisk on /system/usr/keylayout type tmpfs (ro)` 等（`s1c_07_final_state.txt`）。
- 守护进程：`daemon_start … pid=540`、`heartbeat n=1/2`、`547 1 busybox sh /data/adb/modules/orz_body/service.sh`（`ppid=1`）。

### 2.2 模块禁用 / 恢复语义

- `disable` 标志置位 + 重启 ⇒ overlay 挂载消失、模块脚本不执行（`s1d_12_rescue_state.txt`）；清标志 + 重装 ⇒ 恢复注入态（`preinit_ab_summary.txt` 第 6 行）。
- **登记发现 A（见 §4）**：禁用模块**不撤销**已进内核的 sepolicy 规则。

### 2.3 PREINITDEVICE → 模块 sepolicy 规则入内核（A/B，6 次引导）

判据工具：`magiskpolicy --print-rules` 全量规则对比（各 42,4xx 行）+ `sha256sum /sys/fs/selinux/policy`；探针规则特意选「对照组必然不存在」的形态（`shell→magisk` 在对照组只有一条 `unix_stream_socket`）。

| # | boot_id（前 8） | 模块 | 暂存文件 | 策略哈希（前 8） | 规则在否 | 含义 |
|---|---|---|---|---|---|---|
| 1 | `205b8a47` | 启用 | 不存在 | `8242a06d` | 否 | 对照组：旧 ramdisk（无 `PREINITDEVICE`） |
| 2 | `65963a19` | 启用 | 安装时写入 | `8242a06d` | 否 | 新 ramdisk：preinit 面就位，本轮内装模块 |
| 3 | `babf1da8` | 启用 | 存在 | `d2365b61` | **是** | 重启后规则进入内核策略 |
| 4 | `153e915c` | 禁用 | 仍在 | `d2365b61` | 是 | **发现 A**：禁用只摘 overlay，规则不撤销 |
| 5 | `7c86f1ff` | 禁用 | 已删除 | `8242a06d` | 否 | 清暂存后重启 ⇒ 策略精确回到对照组 |
| 6 | `1f530b87` | 启用 | 重装写入 | `d2365b61` | 是 | 重装 ⇒ 恢复注入态（收尾态） |

- 两组规则**全量差异恰好一条**（即模块 `sepolicy.rule` 的那句）⇒ 归因干净，无夹带。

### 2.4 M5 补丁「打好 → 进系统 → 生效 / 坏补丁失败 → 自救 → 回滚」

| # | 步 | 实测结局 | 证据 |
|---|---|---|---|
| 1 | 原厂基线 | 模块 `0.1.1`，overlay 就位，策略 `d2365b61` | `s1d_01_state.txt` |
| 2 | 好补丁 v1（模块 `0.2.0`，jar `3e0fa9cd…`） | overlay 挂到 `/dev/block/dm-43`；需**杀掉并重启装载进程**才生效（观测 `Killed` + odex 重建） | `s1d_04` / `s1d_05` |
| 3 | 好补丁 v2（模块 `0.2.2`，jar `1e8a33ab…`） | **反射探针读回补丁**：`patched_field=patched_method=ORZ-BODY-M5-DRYRUN-MARKER-9F31`、`classloader=PathClassLoader[… /system/framework/android.hidl.base-V1.0-java.jar]`、`exit=0`、`warn_lines=0` | `s1d_07` / `s1d_08` |
| 4 | 坏补丁（模块 `0.2.1`/`0.2.3`，jar `061009d8…`） | 安装成功、无异常 | `s1d_09` |
| 5 | 坏 services 补丁（模块 `0.3.0`，`services.jar` `05a0fa2f…`） | **开不了机**：`zygote64` 装载 dex 时 `SIGABRT`，`system_server` 起不来，`boot_completed` 缺席；只剩 `servicemanager`/`hwservicemanager`/`zygote64` | `s1d_11_badservices_failure.txt` |
| 6 | 自救 | 置 `disable` → overlay 空、live jar **回原厂件**（`services.jar 1a921dbd…` / hidl `e60b7e33…`）、`boot_completed=1`；**策略仍 `d2365b61`** | `s1d_12_rescue_state.txt` |
| 7 | 策略回退 | 清 `/metadata/watchdog/magisk/sepolicy.rule` 后重启 ⇒ 策略 `8242a06d`；终态（无 Magisk、无模块残迹、jar 全原厂）策略 `d1749c5d` | `s1d_13` / `s1d_15` |

- 结论：**M5 的三段纪律都拿到了真机证据**——补丁能真进系统（第 3 行，机械读回而非自报）、坏补丁确实炸在开机链上（第 5 行）、**回滚后回原厂件**（第 6 行）。这正是设计 §12 让模拟器承担的第二件事。

## 3. 工件清单与哈希（sha256，供复现对照）

| 工件 | 字节 | sha256（前 16） |
|---|---|---|
| `ramdisk\ramdisk.stock.img`（原厂） | 2,141,831 | `2322fe242722ed8a` |
| `ramdisk\ramdisk.magisk.img`（无 PREINITDEVICE） | 2,834,471 | `2101609a98fc5965` |
| `ramdisk\ramdisk.magisk.preinit.img`（`PREINITDEVICE=vdd1`，定案件） | 2,834,491 | `ac0c532f5734c5cb` |
| `magisk\Magisk-v30.7.apk` | 25,278,536 | `40729525e298a11d` |
| `pipeline\stock\services.jar`（原厂，= live 回滚值） | 21,446,365 | `1a921dbd3fe1751a` |
| `pipeline\stock\android.hidl.base-V1.0-java.jar`（原厂） | 12,890 | `e60b7e330d59c331` |
| `pipeline\out\orc-m5-good-a.jar` / `-good-b.jar`（好补丁 **v2** 形态，两轮同件） | 16,073 | `1e8a33ab6f1649d2` |
| 好补丁 **v1** 形态的 overlay jar（`3e0fa9cd…`，见 `s1d_04`）——已被后续构建覆盖，不在 `out\` 现存件内，如实留痕 | — | `3e0fa9cd…`（历史观测值） |
| `pipeline\out\orc-m5-bad.jar`（坏补丁） | 16,073 | `061009d883deb41f` |
| `pipeline\out\orc-m5-badservices.jar` | 21,445,455 | `05a0fa2f32f7baa9` |
| `pipeline\out\reflection-probe.jar` | 22 | `8739c76e681f9009` |
| `module\orz_body-0.1.0-s1drill.zip` | 3,533 | `8307cb33bf24ce7a` |
| `module\orz_body-0.1.1-s1drill.zip` | 3,653 | `3f5ace575df1f136` |
| `module\orz_body-0.2.0-s1drill.zip` | 11,442 | `55bd4fe67b5d6902` |
| `module\orz_body-0.2.1-baddrill.zip` | 11,460 | `3f1ffd9faa02c477` |
| `module\orz_body-0.2.2-s1drill.zip` | 11,685 | `7ca9a539fc2afc56` |
| `module\orz_body-0.2.3-baddrill.zip` | 11,703 | `d292a2805219ae11` |
| `module\orz_body-0.3.0-badservices.zip` | 9,119,260 | `c8b679420b5ec4ea` |
| `baseline_avd\config.ini` | 3,809 | `541a6303517a6499` |

## 4. 本批登记的发现（需裁决 / 后续）

**发现 A（载体级，影响回滚语义）**：「禁用模块 + 重启」**不撤销**已注入内核的 SELinux 规则——暂存件只在模块**安装/更新**时重写。彻底撤销需重写暂存（安装/更新任一模块触发）或直接清 `/metadata/watchdog/magisk/sepolicy.rule` 后重启。

- 影响面：残留是**加性 allow 规则**，不改变 overlay/脚本的禁用回滚效果，但**常驻内核策略**；设计 §10.3 层 1「回滚 = 禁用模块 + 重启」对这一条不完整。
- 候选处置（待裁决）：把「清暂存件」写进回滚清单 / 由 orz 机械层代管「模块回滚 = overlay + 暂存件 + 重启」三件套。

**发现 B（账本更正）**：SELinux 策略哈希是**三态**而非两态——`8242a06d…` = 对照组（Magisk 在、模块规则未暂存）；`d2365b61…` = 注入态（模块规则已入内核）；`d1749c5d…` = **无 Magisk 干净态**。BACKLOG 0y 此前写的「回滚后回到注入前 `d1749c5d…`」把后两者混为一谈，本记录更正为「回原厂件 + 按状态区分哈希」。原值不删，作历史留痕。

**发现 C（M5 生效判据）**：新装载的 jar **需要装载进程重启**才生效（v1 需杀 `system_server`）；对 `zygote` 直接装载的 jar，在 v2 形态下可在同一引导内读回。⇒ M5「补丁生效」判据必须带**装载时机**字段（同一引导内可读回 / 需重启进程 / 需重启设备），该字段与 §14.1 接口定义同批确定。

## 5. 未完成（本批不做，留 S1 收尾 / M1 / M0）

1. **orz x86_64 musl 三件套上机冒烟——经本批核证确认未做**：S1 日志全量检索无 orz 二进制痕迹，模块常驻进程实际是 `busybox sh service.sh` 桩守护。这是 S1 判据里唯一缺的实机项（静态 ELF 在安卓内核直接执行 = `BODY-PRE-01` 同形前提）。
2. 感知总线 / 动作分级 / 语音域 / Glyph（属 M1–M4，依赖 §14.1 接口定义）。
3. **M1 前置：接口定义**（事件 schema / 动作契约 / 策略注册表形态，设计 §14.1）——未开始。
4. **M0 定版**（用户侧，唯一分区写入批）：[首次尝试中止记录](0Y_M0_ABORTED_FIRST_ATTEMPT_2026-09-12.md) 的复用路线仍有效（全量备份 + 260618 官方镜像 + 预打补丁 boot 均已保留）。
5. 载体版本同源：本批用件为 0.5.0 世代；`GAP-ORZ-HOST-RESOURCE-SNAPSHOT-DROP`（orz `ea777918`）与 0ac `GAP-MECH-IMMEDIATE-FEEDBACK` 尚未进载体——**安卓支线用件应在下一次载体重建时同批重出**，避免第二条版本分叉（`ORZ-VERDICT-EPOCH-001` 同族纪律）。

## 6. 判据（本批可机械核对）

1. AVD `orz_body_a35` 见于 `D:\android\avd\`，镜像为 `aosp_atd\x86_64`，启动 argv 与 `baseline_avd\emu-launch-params.txt` 逐字一致。**成立**（§1.1）。
2. `magisk --version` 报 `30.7:MAGISK:D`，模块 `orz_body` 见于 `/data/adb/modules/`。**成立**（§2.1）。
3. 模块五面承载各有落点证据（overlay / service.d / privapp / keylayout / sepolicy.rule）。**成立**（§1.3）。
4. 两阶段 marker 同 `boot_id`，守护进程 `ppid=1`。**成立**（§1.3 / §2.1）。
5. PREINITDEVICE A/B 六行齐、两组规则差异恰好一条、策略哈希按态可复现。**成立**（§2.3）。
6. M5 干跑五态齐（原厂 / 好补丁生效读回 / 坏补丁安装 / 坏 services 失败 / 自救回原厂）+ 策略回退。**成立**（§2.4）。
7. 工件哈希可对照（§3）。**成立**。
8. orz 三件套上机冒烟。**未成立**（§5 第 1 条）——S1 收尾项。

## 7. 复现入口

- 建 Magisk ramdisk：`ramdisk\build_magisk_ramdisk.sh [PREINITDEVICE] [OUT]`（容器内挂 `bin`/`ramdisk`/`magisk.apk` 三面；定案调用 `vdd1`）。
- 起模拟器：见 §1.1 argv（`-ramdisk` 指向定案件）。
- 落持久面：`adb push Magisk-v30.7.apk` 解包后执 `magisk\materialize_env.sh`。
- 打补丁与打包：`pipeline\build.ps1 -Mode good|bad` + `pipeline\repack_bad.py` + `pipeline\mkmodule.py`。
- 判定工具：`magiskpolicy --print-rules` + `sha256sum /sys/fs/selinux/policy` + `dumpsys`/`boot_completed` 读数。
- 边界：干跑当时的**驱动命令**未落成单一脚本（逐条命令的历史见 `logs\*`）；把 S1 干跑串成一键复现脚本属后续项，不在本批范围。
