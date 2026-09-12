# 0y M0 离线 OTA 升级执行记录（2026-09-12）

> **类型**：实施执行记录（M0 主链完成，收尾验收待续）
> **范围**：NP1（Spacewar / A063 / P212BQ001470）从 `V3.2-250701-1737` 升级至官方最终版 `V3.2-260618-1045`
> **设计权威**：[`NP1_ORZ_BODY_INTEGRATION_DESIGN_2026-09-11`](../NP1_ORZ_BODY_INTEGRATION_DESIGN_2026-09-11.md) §9.3 / §13（M0 定版）
> **前序**：[M0 首次尝试中止记录](0Y_M0_ABORTED_FIRST_ATTEMPT_2026-09-12.md)（2026-09-12，用户接管）
> **边界**：不动 ADR-0010 与模型工具面；M0 里程碑在「Magisk 安全模式演练」完成前保持开放（BACKLOG 0y / TODO P0-0y）

## 1. 结论摘要

M0 **全部完成（含收尾验收）**：**升级 → 重新 root → 去预装验证 → 救援通道演练**四步已在实机达成，
装置状态为 `V3.2-260618-1045` / 槽 `_b` / Magisk `30.7:MAGISK:R` 生效 / 去预装模块在新版本上经
反证验证有效。收尾验收按**事实更正**：设计原写的「Magisk 安全模式（按音量上禁用全部模块）」在本机
不成立（Magisk 30.7 无按键检测代码），改以两条**已实测**的物理救援通道验收，见 §6。

## 2. 手段更正（相对设计 §9.3；登记）

设计 §9.3 记 M0 为「fastboot 刷官方最终版分区 + 重新 patch boot 上 Magisk」。**本次实测该
手段不可行**：本机 `fastboot flashing unlock_critical` 被固件策略拒绝，critical 分区
（xbl/abl/tz/modem 等）直刷返回 `Flashing is not allowed for Critical Partitions`，而
250701→260618 的每一级增量包**均触及全部 28 个分区（含全部 critical 固件）**，跳过 critical
的混合刷写不成立（前序中止记录 F1–F3；本次已独立复核，见 §4.2）。

**更正后手段 = 官方签名 OTA 链**（唯一可行路径），重新 root 仍按设计用 fastboot 刷预打补丁
boot。设计 §9.3 的实质结论不变（M0 仍是本支线唯一分区写入批、引导链零触碰、模块外仅 M0 与
Magisk 本体两件一次性基座）；变更的只是「升级」这一半的载体形态。**建议**：把该更正补进设计
§9.3 或 §13 的 M0 条目（本记录不改设计裁决，留项目方裁决）。

## 3. 关键发现：官方离线升级应用

前序中止记录的三条路线（原生系统更新器 + 代理 / recovery sideload / 预打补丁 boot）**均未
覆盖本机自带的官方本地升级载体**：

- 包名 `com.nothing.OfflineOTAUpgradeApp`（`/system_ext/priv-app/NothingOfflineOtaUpdate/`，
  privileged，versionName 1.0 / targetSdk 35）；
- 入口 = 拨号密令 `*#*#682#*#*`（`TelephonySecreteCodeReceiver`，authority `682`），或
  `am start -n com.nothing.OfflineOTAUpgradeApp/.ui.MainActivity`；
- 读取 `/data/media/0/ota/`（= `/sdcard/ota/`）内的包，**自身完成 payload 解析与
  `payload_properties.txt` 处理**后调用 `update_engine`；界面暴露
  `Directly apply OTA from select` / `Stop` / `Reset` / `RELOAD` / `Browse`。

该应用正是前序 `update_engine_client --headers` 失败（metadata 无效）所缺的那一环：本次
每一级均先由它报出 **`verify metadata file success`** 才进入应用阶段。

**Clash / 代理路线（用户 2026-09-12 提议）评估结论**：方向成立（critical 分区只能由签名 OTA
更新），但优先级不应置于本路径之前——本机当时**无任何网络链路**（WiFi 已启用但未关联、无 IP、
无 DNS）、**系统时钟停在构建日期**（TLS 必失败），且 250701→260618 需经代理约 **3.4GB** 流量
（full 3.00GB + 5 级增量 0.34GB）——而全部官方包已完整握在本地。实测本路径不需要网络与代理，
故代理路线保留为后备（若离线应用拒不接受本地包）。

## 4. 执行流水与证据

### 4.1 前置复核（离线，不改设备）

| 项 | 结果 |
| --- | --- |
| 全量分区备份 | 86 分区、7.82 GiB；**重算全部 86 个落盘镜像 SHA256，与 dump manifest 的 OK 行逐条一致**（`D:\tb-eval\np1_m0\backup\`） |
| 260618 官方镜像 | `C:\np1_m0_ex\` 28 个镜像与官方 hash 清单 **28 匹配 / 0 差异 / 0 缺失** |
| 升级链完整性 | 6 个包 `testzip` 全过；链序 250926 → 251219 → 251231 → 260206 → 260416 → 260618；`POST_OTA_VERSION` 终点 = `Spacewar_V3.2-260618-1045` |
| 分区覆盖面 | 自写解析器（`DeltaArchiveManifest.partitions` = protobuf 字段 **13**）复核：6 个包**均触及 28 个分区**（含 critical） |
| 装置回退态 | 昇级前实测 `V3.2-250701-1737` / 槽 `_b` / root 活着 / `oem_unlock_allowed=null` / `/data/ota_package` 空 / 271 包 |

> 注：前序中止记录引用的 `payload_partitions.py` 解析字段写的是 2，重跑输出为空表——
> 其结论（28 分区）本次已用修正解析器独立复核成立，但**该脚本作为证据不可复现**，需修正。

### 4.2 升级链（每级：推包 → 应用 → 引擎应用成功 → 重启 → 核对版本）

载体 = 官方 6 个签名包 + `com.nothing.OfflineOTAUpgradeApp`；全程 USB 供电、电池 100%。

| # | 包 | 目标版本 | 结果（`ro.build.display.id` / 槽） |
| --- | --- | --- | --- |
| 1 | `Spacewar_V3.2-250926-1631_full.zip`（3,003,328,530 B） | 2509261631 | ✓ `V3.2-250926-1631` / `_a` |
| 2 | `delta_251219.zip` | 2512191652 | ✓ `V3.2-251219-1652` / `_b` |
| 3 | `delta_251231.zip` | 2512310041 | ✓ `V3.2-251231-0041` / `_a` |
| 4 | `delta_260206.zip` | 2602061016 | ✓ `V3.2-260206-1016` / `_b` |
| 5 | `delta_260416.zip` | 2604161140 | ✓ `V3.2-260416-1140` / `_a`（首次尝试失误，见 §5） |
| 6 | `delta_260618.zip` | 2606181045 | ✓ `V3.2-260618-1045` / `_b` |

关键证据：

- 第 1 级：`verify metadata file success` → `delta_performer` 3280 operations → 100% →
  `PostinstallRunnerAction ... kSuccess` → `Update successfully applied, waiting to reboot` →
  `PayloadApplicationCompleted - errorCode=SUCCESS/0`（**与前序手工 CLI 在 metadata 处失败形成对照**）。
- 第 1 级推送完整性：设备侧 `sha256sum` = 主机侧 `a97e47cb…8abdaaa`，逐字节一致。
- 全链：每级均以应用内 `Update OTA Success` 对话框 + 版本核对双重确认后才承认成功。

### 4.3 重新 root（fastboot）

`fastboot flash boot_b boot_260618_magisk_patched.img`（100,663,296 B，官方 260618 boot 经设备侧
`boot_patch.sh` 产出）→ 重启后实测 `uid=0(root)`、`30.7:MAGISK:R`。boot 非 critical 分区，原厂
`boot.img`（SHA256 `57f19345…`，与官方清单吻合）在手可回退。

### 4.4 去预装验证（反证法）

去预装 `nothing_debloat` 模块在新版本上的有效性用**反证**确认，不做推断：

| 状态 | 可见包数 | `pm path com.google.android.youtube` |
| --- | --- | --- |
| 模块**禁用**（`disable` 标志 + 重启） | **301** | `package:/product/app/YouTube/YouTube.apk` |
| 模块**启用**（移除标志 + 重启） | **271** | （不可见） |

差额 30 = 模块屏蔽路径数 30（`/data/local/tmp/debloat_list.txt` 的 30 条：17 app + 12 product
priv-app + 1 system_ext priv-app），**逐条与实际差额吻合** → 模块仍精确生效，屏蔽清单对新版本
无需增改。该反证同时完成设计 §10.3 第 1 层「回滚 = 禁用模块 + 重启」的**回滚演练**。

## 5. 过程中的一次失误与纠正（诚实记录）

第 5 级（260416）首次尝试：脚本以 `uiautomator dump` 的**残留文件**为判据，把上一级的
`Update OTA Success` 文本误判为本级成功，随即点击「重启」并核对版本。**脚本的版本核对把它拦下**
——版本仍为 260206、判定 `VERSION MISMATCH` 并中止；装置未受影响、无半成品写入。

纠正：改为 ①每次导出前先删除 dump 目标文件；②要求先观测到 `updater=RUNNING`（引擎真的在跑）
才接受成功判定；③每次应用前校验界面上的「当前版本 + 所选包名」。重做后一次通过。

教训（可复用）：**界面文本导出必须在导出前清空目标文件，且成功判据必须与独立事实（版本）绑定**，
否则设备脚本会退化成「乐观自证」。

## 6. 收尾验收：救援通道演练（含一次机制更正）

### 6.1 更正：本机 Magisk 没有「按键禁用模块」机制

设计 §9.3 原写「最坏情况经 Magisk 安全模式（开机按音量上键禁用全部模块）恢复」。实测与静态核证
均不支持该表述：

- `magiskinit`（`/data/adb/magisk/magiskinit`）中 **volume / key / safe / button 相关字符串为 0 条**
  ——该版本不实现按键判定；
- `magisk` 二进制中的 safe mode 判据是 **`persist.sys.safemode` / `ro.sys.safemode`** 两个属性，
  本机 `/system/framework/services.jar` 同样用这两个属性 → Magisk 是**跟随 Android 安全模式**，
  而非自己识别按键；
- 三次物理尝试均未进入 Android 安全模式（见 6.2）。

### 6.2 三次尝试记录

| # | 操作 | 结果 |
| --- | --- | --- |
| 1 | 音量上（重启后按，未给精确时序） | 正常启动；271 包，模块照旧 |
| 2 | 音量下（屏幕刚黑即按住） | **进入 fastboot 模式**（本机 bootloader 组合键 = 音量下）；`fastboot getvar` 可读、`unlocked: yes`，随后 `fastboot reboot` 正常回到系统 |
| 3 | 音量下（出现开机标志后再按，到锁屏） | 正常启动；`persist.sys.safemode` 空、271 包，未进安全模式 |

每次失败后均核对：`V3.2-260618-1045` / 槽 `_b` / `root` 正常 / 271 包 / `magisk.db settings` 仅
`bootloop=0` —— 装置状态无任何破坏。

### 6.3 验收判据（按事实更正后，全部实测通过）

| 救援通道 | 可达性 | 实测证据 |
| --- | --- | --- |
| **音量下开机 → fastboot**（bootloader 级，**不依赖 Android 能否启动**） | ✅ | 尝试 2 实测进入；`fastboot devices` / `getvar current-slot=b` / `getvar unlocked=yes` 全部可读 → 可刷原厂或预打补丁 boot，即「Android 起不来时仍能用物理键把 Magisk 拆掉或换回」 |
| **模块 `disable` 标志 + 重启**（需系统或 adb 可用） | ✅ | §4.4 反证演练：禁用 → 应用回归（301 包）→ 移除 → 恢复（271 包） |
| Android 安全模式按键组合（属性链路两侧均在位） | ⚠️ 未取得 | 三次尝试未进入；**登记为厂商固件层面待查项，不阻塞 M0** |

**结论**：物理键救援能力已被**比原表述更强**的通道证明——fastboot 通道在系统完全起不来时依然
可达，而原表述依赖的按键禁用模块在本机根本不存在。M0 据此收尾，设计 §9.3 / §10.3 / §13 已同批更正。

## 7. 证据清单

| 证据 | 路径 |
| --- | --- |
| 升级前基线快照 | `D:\tb-eval\np1_m0\logs\m0_resume_baseline.txt` |
| 升级后状态快照 | `D:\tb-eval\np1_m0\logs\m0_post_upgrade_state.txt` |
| 新版本包清单（271） | `D:\tb-eval\np1_m0\snapshots\packages_V3.2-260618-1045.txt` |
| 升级前包清单（271） | `D:\tb-eval\np1_m0\snapshots\packages_V3.2-250701-1737_pre_ota.txt` |
| update_engine 持久日志 ×6 | `D:\tb-eval\np1_m0\logs\update_engine\` |
| 全量分区备份 + dump manifest | `D:\tb-eval\np1_m0\backup\` / `D:\tb-eval\np1_m0\logs\dump_manifest.txt` |
| 官方 OTA 链载体 | `D:\tb-eval\np1_m0\ota\`（full + `deltas\`） |
| 预打补丁 boot | `C:\np1_m0_ex\boot_260618_magisk_patched.img` |

## 8. 边界与遗留风险

- **F1 未独立复核**：本批未再尝试 `unlock_critical`；其「被固件策略拒绝」的结论沿用前序记录，
  仍属未取证断言（前序日志只留下 `xbl_b` 被拒的原文）。因签名 OTA 路径已打通，该点不影响 M0 闭环。
- **系统时钟仍为构建日期**：无网络授时（设计 `BODY-PRE-13` 已登记），联网类能力上机前必须补授时。
- **A/B 槽现状**：活动槽 `_b`（260618 + Magisk），另一槽为 260416 的合法镜像；如需回退可切槽。
- **critical 固件** 已由签名 OTA 更新至 260618，与系统版本一致，不存在混合版本。
- 本次未触碰 `/data/adb/modules` 之外的状态；`/sdcard/ota` 已清空；设备侧临时导出文件已清理。

## 9. 后续

1. **M0 闭合**（2026-09-12，用户裁决按事实收尾）：升级 / 全量备份 / 重新 root / 重建去预装 / 救援通道演练
   全部达成；设计 §9.3、§10.3、§13 已同批更正。0y 条目本身保持开放（M1–M8 未做），计数口径不变。
2. M1 前置：aarch64 musl 0.4.2 三件套重建（现机三件为早期源状态产物）+ 接口定义与 orz 机械层共同确定。
3. 模拟器验证载体 S1（用户 2026-09-12 放行，按「单项做」纪律排在 M0 之后）。
4. 登记待查（不阻塞）：本机 Android 安全模式的按键组合与时序。
