# 0y M0 首次尝试中止记录（2026-09-12）

> **类型**：实施尝试记录（已中止，未达成 M0）
> **范围**：NP1（Spacewar / A063 / P212BQ001470）定版升级至 Nothing OS `V3.2-260618-1045` 的执行尝试
> **结果**：装置零改动回退，用户接管后续实施；本文件沉淀排查结论供复用
> **边界**：不动 ADR/设计裁决；M0 里程碑本身保持开放（BACKLOG 0y / TODO P0-0y）

## 1. 已完成且保留价值的部分

- **全量分区备份**（M0-S1 达成）：86 个分区（除 userdata/raw 盘）dd 落盘，双侧 SHA256 逐对吻合，共 7.9GB，清单 `D:\tb-eval\np1_m0\logs\dump_manifest.txt`。**这是该装置现存唯一全量分区备份**。
- **系统状态快照**：getprop 全量、`pm list packages -f` 包清单（271 包，去预装重建基准）、去预装模块 tar 存档（`snapshots\nothing_debloat_250701.tgz`，含 magisk.db/service.d）。
- **260618 官方工厂镜像**：spike0en/nothing_archive release `Spacewar_V3.2-260618-1045`，28/28 镜像 SHA256 与官方清单吻合；解压于 `C:\np1_m0_ex\`。
- **预打补丁的 260618 boot**：`C:\np1_m0_ex\boot_260618_magisk_patched.img`（设备侧官方 `boot_patch.sh` 产出，stock boot SHA256 `57f19345…` 已验）——**复用时可直接 `fastboot flash boot_<active>` 免鸡生蛋恢复 root**。
- **官方 OTA 链载体**：full `250926`（3.0GB，CRC 全过）+ 5 级增量（`251219`→`260618`，尺寸与 Google OTA 服务器 HEAD 逐一核对），位于 `D:\tb-eval\np1_m0\ota\`。

## 2. 关键排查结论（复用价值）

| 编号 | 结论 | 证据 |
| --- | --- | --- |
| F1 | **`fastboot flashing unlock_critical` 被 Nothing 固件策略即时拒绝**（`Flashing Unlock is not allowed`，非交互确认超时）；Settings「OEM 解锁」开关灰置显示已解锁；`devinfo` 分区全零（非 Qualcomm 标准 devinfo 结构） | 多次复现；`settings put global oem_unlock_allowed 1` 置位无效 |
| F2 | **因此 bootloader 直刷 critical 固件（xbl/abl/modem/tz 等）不可行**；critical 分区只能由签名 OTA（recovery sideload / update_engine）更新 | `fastboot flash xbl_b` → `Flashing is not allowed for Critical Partitions` |
| F3 | **5 级增量全部触及全部 28 个分区（含全部 critical 固件）**——260618 终态固件 ≠ 250701 固件，跳过 critical 的混合刷写不成立，必须走完整签名链 | `payload_partitions.py` 解析五包 manifest，均列出 28 分区 |
| F4 | **`update_engine_client --headers` 在该 Nothing 构建上不生效**：引擎收到的 headers 全空（`metadata_size: 0` → `kDownloadInvalidMetadataSize (32)`）；设备侧脚本投递、`/data/ota_package/metadata` 默认路径两条路均复现。file:// 本地 OTA 路线不可用 | logcat `update_engine` 两次尝试同错误 |
| F5 | **recovery sideload 两次均瞬败**：`Total xfer: 0.00x` 即退出且设备自重启、active 槽不翻转；zip 本体 CRC 完好（`testzip` 全过）。根因未定位（候选：recovery 屏显错误未读取、USB 瞬断、包签名链被 recovery 拒） | 两次复现；`adb sideload` 管道 rc 不可靠（须直取） |
| F6 | **USB 枚举偶发掉线**：模式切换/长传输后 adb+fastboot 同时不可见，强制重启（长按电源 ~12s）恢复；恢复后传输正常 | 会话内两次复现 |

## 3. 回退清单（已全部执行并核验）

- 设备文件清理：`/data/local/tmp/{ota.zip,payload_props.txt,run_ota*.sh,ui*.xml,boot_*.img,new-boot.img}`、`/data/ota_package/{ota.zip,metadata}`、`/sdcard/{ui*.xml,scr.png}`、`/data/adb/magisk/new-boot.img`。
- 设置恢复：`settings delete global oem_unlock_allowed`（原值 null）；`svc power stayon false`。
- 去预装模块：移除 `disable` 标志并重启，magic-mount 重新生效（YouTube 包重归隐藏，`pm list packages` 计数 0）。
- 终态核验：`V3.2-250701-1737` / slot `_b` / root（Magisk 30.7）完好；分区内容全程零写入（F2/F4/F5 各失败路径均先于任何写入）。

## 4. 下次尝试的路线建议（按优先级）

1. **原生系统更新器 + 代理网络**（最官方）：给设备供网（WiFi 凭据或 gnirehtet 反向散射经 PC 代理），驱动设置 → 系统更新；EOL 告别更新应仍可服务，更新器自动处理多级链与重启。
2. **recovery sideload 链**：full `250926` + 5 级增量；重试前先解决 F5（侧载时人工盯 recovery 屏幕文字，确认失败原因；排除 USB 瞬断）。
3. **终态 root 恢复**：任一路线到达 `260618` 后，`fastboot flash boot_<active> boot_260618_magisk_patched.img`（§1 预打补丁产物）→ 重启即 root；随后重建去预装（快照包为基准 diff 新包清单）。
4. 每轮重启后先核 `current-slot`/版本再进下一步；安全模式演练（M0 收尾项）与 BODY-PRE 复测保留。
