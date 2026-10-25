# 357 批：ORZ 0.9.14 双平台载体重建与发行（2026-10-25）

> **用户令**：「请进行重建吧」→「本次请做双平台重建，并发行双平台安装包」→「Linux和win同版本代码即可，请选最新的那个版本代号统一命名」。
> **范围**：将 353（回合原子性／终态声明／时间基准／RLI 退役，orz `eb805117`/`3ddbf947`/`d5b16793`）＋354（系统控制台三层＋T4b 心跳＋354a，orz `f623fbfb`/`5a6d1df0`）＋355（353/354 审查全量处置，orz `69cf5fb9`）＋356（登记项裁决落码，orz `cc3d74a1`）全量编入双平台载体；版本 bump 0.9.14（orz `95faff3a`）；父仓钉源（`22fd11af`）；Windows 备份、换装、ACAF 重 provision、载体清单、doctor 与活体核证；Linux musl 重建与双发行版冒烟；双平台安装包、顶层校验文件、GitHub Release 与下载回读核证。流程沿 346（双平台＋发行）＋352（Windows 换装）既定口径。
> **计数**：66 不变（0dq／0dr 保持 open 待用户裁决）。

## §0 结论

双平台载体统一升至 **0.9.14**（同版本代号、同源码 `95faff3a`）：Windows release 件、载体位 `D:\tb-eval\orz-windows` 与装机位 `%LOCALAPPDATA%\orz\bin` 三件 cmp 逐字节一致（MATCH 3/3）；ACAF 已按新 signer 重 provision（`binary_sha256=d959aa3b…` 逐位一致）；载体清单 v0.9.14 重生成；doctor 载体位 `ok=12 warn=0 fail=0`、装机位 `ok=11 warn=1 fail=0`（warn 环境性，优于 346/352 基线）；目标工作区 web 桥以缺省姿态在役，`/api/boot` 报 bridge/child 均 `0.9.14`、`version_skew=false`，五会话投影正确。Linux 三件套 musl static-PIE，Alpine 3.20 与 Debian bookworm 冒烟均 `0.9.14`。

发行面：WIN zip `5fc3c425…`／LIN tar.gz `f19e78ff…`／顶层 `SHA256SUMS`（193B 显式 LF，`f637bc51…`）已上 GitHub Release **v0.9.14**（tag 与名一致、Latest、immutable、非 draft）；服务端三资产 digest 与本地候选逐项一致；下载回读 BYTE-IDENTICAL 3/3、顶层复算实跑 2/2 OK。**无 346 §5 型 tag 事故**（`v0.9.14` 名首次使用）。

## §1 版本与源码门

- orz 子模块 `95faff3a`（bump：仅 `crates/orz-bin/Cargo.toml` 与 `Cargo.lock` 两行）＝`69cf5fb9`（355）→`cc3d74a1`（356）→`95faff3a`。父仓钉源提交 `22fd11af`（gitlink＋`orz_source_manifest.sha256` 重生成 1496 条）。
- 测试基线＝355/356 批读数（同一源码态：orz-loop lib 910/0/3＋run_turn_integration 4/4、orz-host 串行 452/0/6、orz-tools 3062/0、orz-config 165/0、orz-assurance 309/0、orz-web 79/0、orz-bin 六套件全绿）。bump 仅版本串，不重跑。
- 推送：orz `cli/feat/fusion-architecture` `f623fbfb..95faff3a`（六提交）；父仓 `origin main` `a340eadb..b147faf1`（连带 353 起在途 docs 提交一并上远端）。

## §2 Windows 重建、备份、换装与 ACAF

构建 `cargo build -p orz-bin --release -j 2` exit 0（日志 `D:\tb-eval\buildlogs\win_357_build.log`）。`--version` → `orz 0.9.14 (windows/x86_64/release)`；build-info 同。

| 产物 | 字节 | SHA-256 |
|---|---:|---|
| `orz.exe` | 59,526,656 | `d848bbef3aeaaf2a656209aab9493126c9fd9f4978763ab7aeb3dc2a8a0d3bf0` |
| `orz-signer.exe` | 6,740,480 | `d959aa3b828a3224bfa6343f7b37528fed839bc69e2b9c539f8e5a840d7fa25b` |
| `orz-acaf-provision.exe` | 6,644,736 | `a974f14f1577186ad7d343ed168dfb79ddc25c2243072cfcccb942c1292ae247` |

- 备份：0.9.13 三件在载体位与装机位分别备份 `*.0.9.13-357-bak`，哈希 `6f440ef8…`／`635c684f…`／`72a1396a…` 与 352 账面逐位一致（回滚路径同 352 §2：停栈→成组还原→重 provision）。
- 换装前核：`Get-Process` 无 orz 进程；换装 release→载体位→装机位后 **cmp 逐字节 MATCH 3/3**（`orz.exe`／`orz-signer.exe`／`orz-acaf-provision.exe` 各三位一致）。
- ACAF：旧 manifest 备份 `acaf/signer-manifest.json.bak-20261025-357`；重 provision exit 0，`binary_sha256=d959aa3b…` 与新 signer 逐位一致；launch env 三指针不变（keystore 幂等装载）。
- 载体清单：`generate_carrier_manifest.py D:\tb-eval\orz-windows --version 0.9.14` → 3 entries。
- doctor：载体位 `ok=12 warn=0 fail=0`（同 352）；装机位 `ok=11 warn=1 fail=0`（warn 环境性；较 352 的 10/2 净改善一项）。

## §3 Linux 重建与验证

Docker engine 29.6.2 就绪（Desktop 冷启动）；`rust:1.97-slim`＋`D:\tb-eval\build_orz_aliyun_trixie.sh` 重建 exit 0（日志 `D:\tb-eval\buildlogs\lin_357_build.log`；`MSYS_NO_PATHCONV=1` 沿 346 注记）。旧 0.9.12 三件先备份 `*.0.9.12-357-bak`（哈希 `203acbba…`／`1c588a9a…`／`12e28495…` 与 346 账面逐位一致）。

| 产物 | 字节 | SHA-256 |
|---|---:|---|
| `orz` | 117,404,232 | `3566b3de541225fca21e7b51def4c40f7d119bab0956262d7c316f2e7afc9417` |
| `orz-signer` | 1,397,752 | `39bb81a48f156a8dafcdd26717525452d06d3172d3825b16a5ed72dcb5bd5824` |
| `orz-acaf-provision` | 1,216,352 | `d1b000668151f235a9afad7a511c5aaca9f133861a15dbd39dd8c3b4b6fb01e0` |

- Alpine 3.20：`--version`／`--build-info` 均 `0.9.14`。
- bookworm：`--version` `0.9.14`；ELF `Type: DYN`＋**`PT_INTERP` 缺席（static PIE，readelf 实测 0 条 INTERP）**；doctor `ok=10 warn=2 fail=0`（两 warn 环境性：无 journal、无浏览器，与 341/346 基线同形）。
- Linux 载体清单 v0.9.14 重生成；Linux 侧不重 provision（沿 320/336 口径）。

## §4 活体核证

目标工作区 `C:\Users\1\OneDrive\Desktop\新建文件夹`，装机位 `orz.exe web --real`（日志 `D:\tb-eval\buildlogs\orz_357_web_stdout.log`，监听 `127.0.0.1:21487`）：

- 启动 banner：`[permission] mode=interactive-yolo (auto-approve, source=default; benchmark axes off)`——§14.91 缺省姿态逐字核收。
- `/api/boot`：bridge `0.9.14`／child `0.9.14`／`child_probe=ok`／`version_skew=false`／cwd 为目标工作区／agent 为装机位件。
- `/api/conversations`：五会话——`51da03ea`、`54d12bcc`、`d24eadd6` 活跃，`073b8892`、`c23e0aaa` 已关闭，`legacy_short_id` 全 false；与 352 后已知状态连续。
- 旧标签页 token 失效，须用本次启动新 URL（token `ea893f72…`；343 同款注记）。

## §5 安装包与发行

### 5.1 包与哈希

| 资产 | 字节 | SHA-256 |
|---|---:|---|
| `orz-0.9.14-windows-x86_64.zip` | 28,878,781 | `5fc3c425372c57df17010fffcfc9a429c5f379526814be9ea7f48f50e763d925` |
| `orz-0.9.14-linux-x86_64.tar.gz` | 38,318,588 | `f19e78ffe9db0b1fc9f2b8b798848a50b5a0c9eb1bf4ec81e87b2323df7a5324` |
| `SHA256SUMS` | 193 | `f637bc51705316bac8462225084a5797b29f91c4bb3fe209cd4ec40960a9db21` |

每包 6 顶层 entry（三件可执行＋`carrier-manifest.json`＋`README.md`＋包内 `SHA256SUMS`）；包内 sums **显式 LF**（346 §5 纪律，Python 二进制核验 `\r` 零命中）与顶层 sums（193B LF）。

本地目录 `D:\tb-eval\rel-357-stage`；解包核证 `D:\tb-eval\rel-357-verify`：双包解包后 `--version` 均 `0.9.14`；六件与载体原件 **BYTE-IDENTICAL 6/6**（cmp）；包内 README 与发行工作树 README **3/3 一致**（中英）；包内校验实跑 5/5＋5/5 OK。

### 5.2 提交、推送、tag、Release 与回读

- 发行提交：父仓 `b147faf1`（README 中英 0.9.14 同步〔安装示例／Release 链接／§14.91 过渡语收口／0.9.14 更新节取代 0.9.12 节〕／TODO／BACKLOG／索引 v4.323／AUTH-357／orz gitlink）；tag `v0.9.14` → `b147faf1`。
- GitHub Release：`v0.9.14`（URL `https://github.com/SilverWhite/orz/releases/tag/v0.9.14`），`isDraft=false`、`isPrerelease=false`、`isImmutable=true`、**Latest**；服务端三资产 size＋`sha256:` digest 与本地候选逐项一致。
- 下载回读 `D:\tb-eval\rel-357-readback`：三件 **BYTE-IDENTICAL 3/3**；顶层 `sha256sum -c` **实跑 2/2 OK**（§5 纪律：不能只做逐字节比对）。

## §6 残余与边界

1. README 中英姿态过渡语已收口（"在役 0.9.12 双轨过渡"类表述随 0.9.14 在役移除，历史缘由保留一句）。
2. 本版不新增 macOS 原生包；Linux 仍为构建产物（static PIE），未做专门体验适配。
3. 0.9.13（Windows-only、未发行）无独立安装包——0.9.12-1 之后的首个发行即 0.9.14；Linux 从 0.9.12 直接跳 0.9.14（版本偏横幅清零）。
4. 备份与旧 manifest 保留在盘（载体位／装机位 `*.0.9.13-357-bak` 与 `*.0.9.12-357-bak`、`signer-manifest.json.bak-20261025-357`；回滚路径同 346 §6.3）。
5. 353～356 批的残余登记（capture 重录、353 §5.3②③、终答轴窗口、0dr Envelope 边界）不随发行闭合；0dq／0dr 闭合仍待用户裁决。
6. 桥重启后旧标签页 token 失效（新 token `ea893f72…`）。

## §7 最终判定

- 换装完整性：**通过**（双平台三件同源逐位、备份可回滚、provision/清单/doctor/活体四门全绿）。
- 发行完整性：**通过**（服务端 digest 一致、回读逐位、顶层复算实跑、immutable＋Latest）。
- 交付边界如实：计数 66 不变；0dq／0dr 待用户裁决。
