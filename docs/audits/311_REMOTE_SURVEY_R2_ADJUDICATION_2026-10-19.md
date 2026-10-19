# 311 批：绝对一对一远程形态第二轮裁决落账——调研底稿 v1.1（2026-10-19）

> **批次性质**：裁决落账批（零源码零跑批；**计数 60 不变**；候选
> `OBS-REMOTE-ONE-TO-ONE-2026-10-19` 维持 `candidate`，设计线立项待用户最终令）。
> **来源**：用户四点裁决（「优先借鉴成熟设计」「能用 win 安全中心做文件锁定吗」
> 「先锁定再放开已知 USB＝一视同仁全锁更省事」「PIN 不作凭据、要 ACAF 绑定密码」）。

## §1 四点裁决与查证

1. **路线定向＝A（借成熟底座）**——orz 做鉴权守卫层（配对/二存一/风险机制/审计/后
   门），传输/画面借 RustDesk 或 Tailscale；选项 B 可作为 A 的首个增量先落地；C 退役。
2. **Win 安全中心做文件锁定＝拆两半（查证落定）**：事件式锁机**不可行**——Tamper
   Protection 拦截一切程序化 Defender 修改（含 CFA 的 `Set-MpPreference`），管理员同拒、
   无纯 PowerShell 旁路；**安装时基线加固采纳**＝provision 时一次性配 CFA＋核 BitLocker，
   文件层防护由 OS 常驻承载。事件式响应维持替代五件套（310 §4）。
3. **锁形态＝外部接口一视同仁全锁＋已知设备白名单（用户直觉成立）**——OS 原生**设备
   安装限制策略**（`DeviceInstall\Restrictions` 的 `DenyUnspecified`＋`AllowDeviceIDs`）
   恰一开关一表；被拦安装落安全日志 Event 6423（天然审计面）；白名单用**设备实例 ID**
   （配对时登记用户键盘/鼠标——与 ① 配对注册制合流）。边界如实记四条：新装阻断≠已装
   失活（PnP 枚举禁用或新版 already-installed 选项）；核心类豁免（网络/系统盘/显示——
   锁网络＝远程会话自断）；改策略需管理员；内置键盘不可拔＝天然逃生面。
4. **鉴权根＝ACAF 绑定密码（否 Windows PIN）**——Hello PIN 系设备本地 TPM 绑定、设备
   属主可重置，不作信任根；安装时采集独立密码 Argon2id 派生入 ACAF keystore，解锁/
   设备变更必过＋rate-limit。

## §2 台账

- 调研底稿 v1.1（§7 第二轮裁决＋§5 A 定向注）：[`REMOTE_ONE_TO_ONE_DEVICE_BOUND_SESSION_SURVEY_2026-10-19`](../REMOTE_ONE_TO_ONE_DEVICE_BOUND_SESSION_SURVEY_2026-10-19.md)。
- 索引头行 v4.282 → **v4.283**＋候选条目裁决注；BACKLOG 指针/计数行；TODO 计数行；
  第二卷 §1.250。
- 门禁 `valid: true`；计数 60 不变；未推送。

## §3 关键词

311 批、第二轮裁决、路线 A 定向、Tamper Protection 查证、设备安装限制策略、
DenyUnspecified、AllowDeviceIDs、Event 6423、实例 ID 白名单、核心类豁免、ACAF 密码根、
Argon2id、计数 60 不变、v4.283。
