# 341 批：ORZ 0.9.10 双平台载体重建与 Windows 换装（2026-10-23）

> **用户令**：在 340 批完成 ORZ 会话恢复／切换审计与首轮修复后，进行重建。
>
> **范围**：将 340 批修复编入 Windows 与 Linux 载体；Windows 完成备份、换装、ACAF 重 provision、重启和活体核证；Linux 产出静态 PIE 载体并做跨镜像冒烟。本批不提交、不推送、不打 tag、不发 GitHub Release，也不替用户触发真实会话恢复。

## §0 结论

重建完成，版本从 `0.9.9` 升至 `0.9.10`。Windows release 件、载体位 `D:\tb-eval\orz-windows` 与装机位 `%LOCALAPPDATA%\orz\bin` 三件套逐字节一致；ACAF 已按新 signer 重 provision；目标工作区的 ORZ Web 已用新装机件重启。Linux 三件套由既有 trixie 构建脚本产出，主程序为无 `PT_INTERP` 的静态 PIE，并通过 Alpine 与 Debian 冒烟。

活体 `/api/boot` 已报告 bridge/child 均为 `0.9.10`、`version_skew=false`。`/api/conversations` 也已从新载体返回 340 批新增字段：`c23e0aaa` 被投影为 `closed=true`、`legacy_short_id=true`，并安全推断完整 UUID `c23e0aaa-e752-4215-9a4f-0e435ff09b6a`；它不再需要冒充活跃会话。

## §1 版本与源码门

版本只改两处：

| 文件 | 变化 |
|---|---|
| `orz/crates/orz-bin/Cargo.toml` | `0.9.9` → `0.9.10` |
| `orz/Cargo.lock` | `orz-bin` package `0.9.9` → `0.9.10` |

重建承载 340 批既有修复：关闭态投影／分组、受约束的 legacy 短 ID 迁移，以及 `load 新目标 → 切换绑定 → close(switch) 旧附着` 的真实切换顺序。源码回归门沿用 340 批结果：`orz-web` 78/0、host `session_load_` 4/0、frontend smoke 全绿、fmt 与 diff-check 通过。

## §2 Windows 重建、换装与 ACAF

构建命令使用仓库内 `protoc`：`cargo build -p orz-bin --release -j 2`，exit 0，用时 2m36s，编译器无 warning。release 产物如下：

| 产物 | 字节 | SHA-256 |
|---|---:|---|
| `orz.exe` | 59,284,992 | `30f736597ffac07bee1d000a61ccb000ddedbedb0bfe1d5947bbcba200076a1f` |
| `orz-signer.exe` | 6,740,480 | `d0e57032f630932f24c5038ff59c29f6acf6198998a85c97de84b0f7659b2913` |
| `orz-acaf-provision.exe` | 6,640,128 | `00c402e00605b8060770d97ab0fa8cb701cd23ab17239f3f2025eedfc8429def` |

- `orz.exe --version`：`orz 0.9.10 (windows/x86_64/release)`；build-info 同为 `0.9.10`。
- release → 载体位 → 装机位三件逐件 MATCH，合计 3/3。
- 旧 0.9.9 三件套已在载体位与装机位分别备份为 `.0.9.9-339-bak`，备份哈希与换装前原件一致。
- ACAF 原 manifest 备份为 `signer-manifest.json.bak-20261023-341`；重 provision 成功，新 `binary_sha256` 与 signer 的 `d0e57032…` 一致。
- Windows carrier manifest：552 B，SHA-256 `6e75c1f3364b8facfa0b2dd55d7d1c71401903dd00cbb3a6678e60fe4aee24a5`。
- 载体位 doctor：`ok=12 warn=0 fail=0`；装机位 doctor：`ok=11 warn=1 fail=0`，唯一 warning 是装机位本来不放 carrier manifest。

构建日志：`D:\tb-eval\buildlogs\win_341_build.log`。

## §3 Windows 重启与活体核证

换装前确认在役三进程均来自预期路径后停止旧栈；换装后从目标工作区 `C:\Users\1\OneDrive\Desktop\新建文件夹` 启动装机位 `orz.exe web --real`。本批核证时：

- Web 监听 `127.0.0.1:21487`；
- ACP WebSocket initialize 成功，`protocolVersion=1`；child 使用装机位 `orz.exe --stdio`，signer 使用载体位 `orz-signer.exe`；
- `/api/boot`：cwd 为目标工作区，bridge/child 都是 `0.9.10`，`version_skew=false`；
- `/api/conversations` 返回 4 个会话，其中 `51da03ea` 为 `closed=false, legacy_short_id=false`，`c23e0aaa` 为 `closed=true, legacy_short_id=true` 且返回受双伴随侧车支持的完整 UUID；另两项保持未关闭。

这里刻意没有调用 `session/load(c23e0aaa…)`：重建验证不代替用户作“恢复并继续”的状态变更。legacy conversation 信封仍会等到用户显式恢复、成功取得 lease 后才由 340 批逻辑规范化。

Windows 策略阻止了两次自动拉起浏览器，不影响服务、HTTP 或 WebSocket；旧标签页中的启动 token 会失效，应使用本次启动生成的新 URL。

运行日志：`D:\tb-eval\buildlogs\orz_341_web_stdout.log`、`D:\tb-eval\buildlogs\orz_341_web_stderr.log`。

## §4 Linux 重建与验证

首次容器命令在 Docker Desktop engine 未运行时立即失败，属于环境门，未进入源码构建。启动 Docker Desktop 后，以既有 `rust:1.97-slim` + `scripts/build_orz_aliyun_trixie.sh` 重跑，exit 0，总用时 7m20s（Cargo release 5m49s）；完成后已停止 Docker Desktop。

| 产物 | 字节 | SHA-256 |
|---|---:|---|
| `orz` | 118,415,448 | `4b944624fde7f15e592d5187628ada4b22942ec2ca150db9d66492e91724b0b8` |
| `orz-signer` | 1,397,896 | `758fe22b9941459c4b9918ce9feb72d1c64bcda3b3f416241fafad755e3bd616` |
| `orz-acaf-provision` | 1,216,352 | `de68a984bc697a7f02f5376296c82c823663ee6c4336197304412b3cb2fff67c` |

- Linux carrier manifest：541 B，SHA-256 `85babf8f7930474684ed9571b51e403c559aaeca15695f1281921580b3c4e746`。
- Alpine 3.20：version/build-info 均为 `0.9.10`，rc0。
- ELF：Type `DYN`、machine `x86-64`、无 `PT_INTERP`，即 static PIE。
- Debian bookworm doctor：`ok=10 warn=2 fail=0`；载体清单三件全部通过。两项 warning 是容器内无 journal 与浏览器，符合该冒烟环境。
- Linux 构建仍打印已知 cfg／平台条件 warning；没有把它误记为“零 warning”。

构建日志：`D:\tb-eval\buildlogs\lin_341_build.log`。

## §5 可回滚性与发布边界

1. Windows 载体位和装机位均保留经哈希核对的 0.9.9 三件套备份，ACAF 旧 manifest 另有独立备份；若需回退，应停当前 ORZ 栈后成组还原并重 provision，不应混用两个版本。
2. **342 批更新**：本批产物随后已进入 0.9.10 提交、推送与正式发行流程；发行资产与远端回读见 [`342 批档`](342_RELEASE_V0910_2026-10-23.md)。本档保留重建时点的构建事实。
3. `c23e0aaa` 的历史 `RUN-1` 无终态、ARC 计数口径，以及归档包不保存原 mtime，仍分别沿 340／338 批残余登记，不因重建自动消失。
4. 浏览器自动打开受本机策略阻止是启动便利性摩擦，不是 ORZ 服务失败；服务、API、ACP child 与 signer 均已独立核证。

