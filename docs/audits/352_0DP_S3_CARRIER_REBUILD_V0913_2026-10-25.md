# 352 批：0dp S3——Windows 载体重建换装 0.9.13

> 日期：2026-10-25
> 用户令：请进行重建换装吧。
> 范围：将 349 批 0dp S2 落码（orz `adcac16f`）与 350 批 orz 侧两修复（`535470f2` 会话短 ID owner 碰撞 fail-closed＝F-02、`546baa48` ORZ_YOLO 编码拒绝＝F-10）编入 Windows 载体；版本 bump 0.9.13（`6a1828f4`，351 批钉源）；备份、换装、ACAF 重 provision、载体清单重生成、doctor 与活体核证。流程沿 343 先例（Windows-only：无 Linux 载体、不发行）＋346 口径（备份/换装/provision/清单/doctor/活体）。
> 计数：65 不变。0dp 主线 S1–S3 全达成，闭合待用户裁决。

## §0 结论

在役载体从 0.9.12 升至 **0.9.13**：release 三件、载体位 `D:\tb-eval\orz-windows` 与装机位 `%LOCALAPPDATA%\orz\bin` 逐字节一致（MATCH 3/3）；ACAF 已按新 signer 重 provision（`binary_sha256=635c684f…` 逐位一致）；载体清单重生成 v0.9.13（3 entries）；doctor 载体位 `ok=12 warn=0 fail=0`、装机位 `ok=10 warn=2 fail=0`（warn 环境性，同 341/346 基线）；目标工作区 web 桥以**缺省姿态**（`source=default` 自动审批）在役，`/api/boot` 报 bridge/child 均 `0.9.13`、`version_skew=false`，五会话投影正确。**0dp 设计面（缺省 YOLO＋`--yolo`/`--no-yolo` 旗＋`ORZ_YOLO` env 单源＋banner 源感知）全部进役实测通过。**

## §1 版本与源码门

- orz 子模块 `6a1828f4`（`heads/feat/fusion-architecture`）＝349 S2 `adcac16f` → `535470f2`（F-02 修复）→ `546baa48`（F-10 修复）→ `6a1828f4`（bump 0.9.13：仅 `crates/orz-bin/Cargo.toml` 与 `Cargo.lock` 两行）。已推 `cli/feat/fusion-architecture`；父仓 351 提交 `ab83c417`（钉源＋`orz_source_manifest.sha256` 重生成）已推 `main`。
- 版本实测：`orz.exe --version` → `orz 0.9.13 (windows/x86_64/release)`；`--build-info` → `version=0.9.13 os=windows arch=x86_64 profile=release`。
- 测试基线引 350 批档 §5（同一源码态工作区读数：`cargo test -p orz-host --lib -- --test-threads=1` 436 passed / 6 ignored / 0 failed、orz-bin 各套件全绿、frontend smoke 全过）。352 零源码改动，不重跑。
- 仓库门禁：`python scripts/check_repository.py` → `valid=true`、`errors=[]`（350 §6 预留的「提交 orz 后重生成清单再提交父仓」已由 351 闭合，本批为提交态首次核收）。

## §2 构建、备份与换装

构建 `cargo build -p orz-bin --release -j 2` 于 2026-10-25 14:32–14:33 完成（351 钉源同一工作会话；产物 mtime 为证，构建日志未留存——证据链为下表哈希、`--version`/`--build-info` 活体与部署件逐位一致）。本批会话完成门禁核收、provision、清单、doctor 与活体。

| 产物 | 字节 | SHA-256 |
|---|---:|---|
| `orz.exe` | 59,313,664 | `6f440ef87dd03c83c5c2b46dac20bb6edbed9d55d65fea665412c4ec755dae88` |
| `orz-signer.exe` | 6,740,480 | `635c684fe1c55daf411fbe68a27740c0ca4141c308c5dfd5a554e7d350a9c80b` |
| `orz-acaf-provision.exe` | 6,640,128 | `72a1396a26a3e3183e300c2e7821eb64dfcfb42e93f1ddb6fc9d7b2772ab9293` |

- 备份：0.9.12 三件在载体位与装机位分别备份为 `*.0.9.12-351-bak`，哈希 `46cdb8a0…`／`6dcdff1f…`／`2cd448c9…` 与 346 账面逐位一致（回滚路径同 346 §5：停栈→成组还原→重 provision）。
- 换装：release → 载体位 → 装机位，显式路径 SHA-256 复核六件 MATCH 3/3。
- 在役栈：本批核证时无 orz 进程在跑（`Get-Process` 0 个），无停栈事件。

## §3 ACAF 与载体清单

- ACAF：旧 manifest 备份 `acaf/signer-manifest.json.bak-20261025-352`；`orz-acaf-provision.exe <keystore> <manifest>` exit 0，新 `binary_sha256=635c684f…` 与新 signer 逐位一致；provision 回显 launch env 与用户级 env 三指针一致（`ORZ_ACAF_KEYSTORE=D:\tb-eval\orz-windows\acaf\keystore`、`ORZ_ACAF_MANIFEST=…\acaf\signer-manifest.json`、`ORZ_ACAF_BINARY=D:\tb-eval\orz-windows\orz-signer.exe`——生产 signer 取载体位）。keystore 幂等装载，未新建。
- `ORZ_YOLO` 机器级/用户级均未设置——0.9.13 缺省面（`default_initial_yolo=true`）即生效姿态。
- 载体清单：`python scripts/generate_carrier_manifest.py D:\tb-eval\orz-windows --version 0.9.13` → 3 entries（`generated_at=2026-10-25T06:44:09Z`），三件哈希/字节与部署件一致。

## §4 doctor

- 载体位（`D:\tb-eval\orz-windows`）：`ok=12 warn=0 fail=0`——含 carrier 清单复验 3 件、retrieval 缺省开、CDP Chrome 发现；优于 346 基线（11/1/0）。
- 装机位（`%LOCALAPPDATA%\orz\bin`）：`ok=10 warn=2 fail=0`——两 warn 环境性（该目录无 run journal；装机位本不放载体清单），与 341/346 基线逐项同形。

## §5 活体核证

目标工作区 `C:\Users\1\OneDrive\Desktop\新建文件夹`，装机位 `orz.exe web --real`（日志 `D:\tb-eval\buildlogs\orz_352_web_stdout.log`／`orz_352_web_stderr.log`，监听 `127.0.0.1:21487`）：

- `/api/boot`：bridge `0.9.13`／child `0.9.13`／`child_probe=ok`／`version_skew=false`／cwd 为目标工作区／agent 为装机位件。
- 会话投影：`/api/conversations` 5 会话——`51da03ea`、`54d12bcc`、`d24eadd6` 活跃（closed=false），`073b8892`、`c23e0aaa` 已关闭，`legacy_short_id` 全 false；与 343 后已知状态连续。
- **0dp 姿态面实测**（banner 经启动面 stderr 逐字核收，§14.91 条 2 源感知三形态＋旗面）：

| 输入 | banner 实测 |
|---|---|
| 缺省（无旗无 env） | `[permission] mode=interactive-yolo (auto-approve, source=default; benchmark axes off)` |
| `--yolo` | `[permission] mode=interactive-yolo (auto-approve, source=--yolo; benchmark axes off)` |
| `--no-yolo` | `[permission] mode=confirm-first (per-action approval, source=--no-yolo; benchmark axes off)` |
| `ORZ_YOLO=0`（env 无旗） | `[permission] mode=confirm-first (per-action approval, source=ORZ_YOLO; benchmark axes off)` |

- **负向面**（响亮失败 exit 2）：`ORZ_YOLO=garbage`（启动面报错并退出 2）；`--yolo=1`（无值精确形态报错）；`--yolo --no-yolo`（互斥报错）。`ORZ_YOLO=1 ＋ --version` 正常 exit 0（版本面不受校验影响）。
- 收尾：桥以缺省姿态重启在役（PID 20700），新 URL token `6004c5fa…`——**旧标签页 token 失效，须用新 URL**（343 同款注记）。

## §6 未执行项与交付边界

1. 无 Linux 载体、无 tag/Release/README 更新（沿 343 Windows-only 先例；Linux 件仍为 0.9.12，版本偏横幅在 Linux 位工作区属预期，下一自然节点补平）。
2. 本批档随 docs 批本地提交、不推送（推送待用户令）。
3. 过渡期急用桥（Benchmark 三旗过桥）随缺省 YOLO 进役失效归零（347 预告口径兑现），无需动作。
4. Web 工作台审批模式开关仍为条件触发候选（BACKLOG 条件触发区，348 裁决不变）。
5. 0dp 闭合待用户裁决（S1–S3 全达成；S4 无独立真机判据——姿态面已在本批活体逐字实测）。

## §7 最终判定

- 换装完整性：**通过**（三件同源逐位、备份可回滚、provision/清单/doctor/活体四门全绿）。
- 0dp 符合性：**通过（在役态）**——缺省矩阵、旗/env 单源、源感知 banner、响亮失败与 348 设计稿 §1.4/§2 逐字一致。
- 交付边界如实：无发行、无 Linux、未推送。
