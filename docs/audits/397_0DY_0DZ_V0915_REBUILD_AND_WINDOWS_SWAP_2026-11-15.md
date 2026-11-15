# 397 批：0dy／0dz 落码、0.9.15 双平台重建与 Windows 换装（2026-11-15）

> 用户令：先提交推送，再重建并直接换装；构建前将版本从预排的 `0.9.14-7` 改为 `0.9.15`。未要求发行。未闭合计数仍为 73：0dy／0dz 的源码与载体阶段达成，真机交互与长时运行判据仍待实测。

## 源码与版本

- 子仓 `orz`：`e93684e2` 落 0dy Web `/checkpoint`、`/btw` 路由与输入历史方向键修复，以及 0dz 后台墙钟策略；`ba05c3a3` 曾按预排升至 `0.9.14-7`；用户改版后 `f6fb9412` 升至 `0.9.15`。三提交均已推至 `cli/feat/fusion-architecture`。
- 父仓：钉源清单 1505 项校验 `valid`；`20d7f6da` 钉至 `f6fb9412`，已推 `origin/main`。推送前整合了远端 3 个 README 提交，无覆盖。
- 源码门：Web `frontend_smoke.mjs`、Rust 定向单测（超时解析 12/12、工具描述 8/8、自动后台回执、31h/72h 墙钟判据）、`cargo fmt --all -- --check`、`git diff --check` 通过；未跑全量测试。
- 父仓 `check_repository.py`：钉源有效、文档链接错误清零；仍报 22 条头部台账瘦身错误（TODO 2 条旧超长行＋TODO/BACKLOG 20 条旧行龄），均不在本批改动行。完整输出 `D:\tb-eval\buildlogs\repo_397_check.json`，本批不代为改写旧台账历史。

## 双平台载体

- Windows：`cargo build -p orz-bin --release -j 2` 成功，日志 `D:\tb-eval\buildlogs\win_0.9.15_build.log`；暂存 `D:\tb-eval\orz-windows-0.9.15`，构建产物与暂存三件 SHA-256 逐一一致；`--version`／`--build-info` 均为 `0.9.15 windows/x86_64/release`；载体清单 3 项。
- Linux musl：`rust:1.97-slim` Docker 构建。首轮因容器未挂父仓 `runtime`，`orz-assurance` 的两个 `include_str!` JSON 缺件而失败（保留 `lin_0.9.15_build.log`）；续跑加 `D:\CLI\runtime:/orz/runtime:ro` 后成功（`lin_0.9.15_build_retry.log`，约 4m59s）。暂存 `D:\tb-eval\orz-linux`，旧清单已重生成 0.9.15 的 3 项；容器内 `--version`／`--build-info` 均为 `0.9.15 linux/x86_64/release`；三件 ELF `e_type=3`、`PT_INTERP=0`。Linux 未换装。

| 平台 | 文件 | 字节 | SHA-256 |
|---|---|---:|---|
| Windows | `orz.exe` | 68,579,328 | `1f27251e55f2d8dc5f0137b8858f12a4a334418883751f9a743d8c1278e7d144` |
| Windows | `orz-signer.exe` | 6,740,480 | `c6351c588e30c81bef75b327b948672ab9898fa5369a610c6022b7bf01f8c699` |
| Windows | `orz-acaf-provision.exe` | 6,644,736 | `b28a9cccd2c2ba7f973ed413efb4eadc1dfe222b33a8ff0cf1ae40242b52cd69` |
| Linux | `orz` | 98,518,680 | `b9218f4f052fba894012b1322c7693d244b3a696bb99a9b64de1d2e876208c7d` |
| Linux | `orz-signer` | 1,401,248 | `2a4ef291e079ead1faff41b33fe10c43fde2a6b4e76a786aa846599af8572e7e` |
| Linux | `orz-acaf-provision` | 1,217,672 | `620e0e5331c5d4c310e7e9bb11886575c39bcf564d7d79b0fdeadbe3ddcfe807` |

## Windows 换装

- 装机位 `%LOCALAPPDATA%\orz\bin` 换装前为 `0.9.14-6`，无 orz 进程。旧三件备份后缀 `.0.9.14-6-pre-0.9.15-bak`，旧 signer manifest 备份为 `.bak-20261115-0.9.15`；备份可用于停栈后成组回滚与重 provision。
- 暂存三件复制到装机位，SHA-256 全部与暂存一致（3/3）；装机位 `orz.exe --version` 为 `0.9.15`。
- 装机位 `orz-acaf-provision.exe` 重配置成功，manifest `binary_sha256=c6351c58…` 与新 signer 全哈希一致；keystore 与用户级三环境变量指针未迁移。
- 装机位 doctor 输出 `D:\tb-eval\buildlogs\doc_0.9.15_install.txt`：exit 0，`ok=10 warn=2 fail=0`。两项 warn 为装机目录无 run journal 与无 carrier manifest，和 394 基线同形。核后 orz 进程数为 0。

## 后续真机判据

0dy：Web `/checkpoint` 切换回执、`/btw` 同时代排队、编辑草稿时方向键行为；0dz：未设 timeout 启动提醒、显式 72h 不被 30h 夹取、未设 timeout 过 30h 不被绝对兜底杀；与 0dx 时代活体、0dt 判据 4 合并在手动首启窗口核证。这里仅登记源码与静态／短时验证，不将长时判据写成已达成。
