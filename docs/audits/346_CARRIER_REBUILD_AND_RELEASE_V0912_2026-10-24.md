# 346 批：ORZ 0.9.12 双平台载体重建与发行（2026-10-24）

> **用户令**：请进行重建吧，重建完成后可做提交与推送，并发布新版本安装包。
>
> **范围**：将 343（RLI NaN 侧车自愈）/344（安全加固三裁决等八触点）/345（审查发现全量处置）修复编入双平台载体；Windows 备份、换装、ACAF 重 provision、重启与活体核证；Linux static PIE 重建与双镜像冒烟；双平台安装包、顶层校验文件、轻量 tag 与 GitHub Release、下载回读核证。流程沿 341（重建）＋342（发行）既定口径。

## §0 结论

重建完成，版本从 `0.9.11`（Windows 位）/`0.9.10`（Linux 位）统一升至 `0.9.12`。Windows release 件、载体位 `D:\tb-eval\orz-windows` 与装机位 `%LOCALAPPDATA%\orz\bin` 三件逐字节一致；ACAF 已按新 signer 重 provision；目标工作区 ORZ Web 已用新装机件重启，活体 `/api/boot` 报告 bridge/child 均 `0.9.12`、`version_skew=false`，五会话投影正确。Linux 三件套由既有 trixie 构建脚本产出，主程序为无 `PT_INTERP` 的静态 PIE，通过 Alpine 3.20 与 Debian bookworm 冒烟。

发行面：WIN zip `5f4b30d6…`／LIN tar.gz `d293df93…`／顶层 `SHA256SUMS` `38412a9b…` 已上 GitHub Release **v0.9.12-1**（Latest／immutable）；下载回读 BYTE-IDENTICAL 3/3、顶层复算 2/2。**tag 为 `v0.9.12-1` 而非 `v0.9.12`**——首发行事故所致（§5），包体版本仍为 `0.9.12`。

## §1 版本与源码门

版本只改两处：`orz/crates/orz-bin/Cargo.toml` 与 `orz/Cargo.lock`（`0.9.11` → `0.9.12`），提交 `dbc6c96d`（bump）。载体承载 345 批修复 `144442d8`（包含门 canonical 补口、毒锁 warn 留痕与混式收口、orz-host 文档姿态勘误、测试 5 断言修复、exec_policy 残余登记、CI Linux 面）与 344 批 `ccaa2017`、343 批 `bd42e011`。orz 两提交已推 `cli/feat/fusion-architecture`（`73b95597..dbc6c96d`）。

## §2 Windows 重建、换装与 ACAF

构建 `cargo build -p orz-bin --release -j 2`（仓库内 protoc 自发现），exit 0，4m45s，零 warning。日志 `D:\tb-eval\buildlogs\win_346_build.log`。

| 产物 | 字节 | SHA-256 |
|---|---:|---|
| `orz.exe` | 59,296,256 | `46cdb8a03926912bc57bc7dd6743f91fb94eaf20d5a971e7f2c621ef317eca02` |
| `orz-signer.exe` | 6,740,480 | `6dcdff1fb02caebd95169e2762011c0110ee33ac3bdf9de1239d0a36b043cace` |
| `orz-acaf-provision.exe` | 6,640,128 | `2cd448c9b8eba3d7bb8d96bd91d1b2e7e2e96e530bbeb50cfda6d46f7ffe4b8d` |

- `orz.exe --version`：`orz 0.9.12 (windows/x86_64/release)`；build-info 同为 `0.9.12`。
- 在役栈（装机位 bridge `web --real` ＋ child `--stdio`、载体位 signer）确认路径后停止；换装后 release→载体位→装机位逐件 MATCH 3/3（脚本首比对误报 `$LOCALAPPDATA` 变量展开问题，显式路径复核 3/3 一致）。
- 旧 0.9.11 三件套在载体位与装机位分别备份为 `*.0.9.11-346-bak`，备份哈希 `c1c5faf5…`／`1bac63e4…`／`3c6aa9db…` 与 343 账面逐位一致。
- ACAF：旧 manifest 备份 `acaf/signer-manifest.json.bak-20261024-346`；重 provision exit 0，新 `binary_sha256=6dcdff1f…` 与 signer 逐位一致。
- 载体清单重生成（`scripts/generate_carrier_manifest.py`，3 entries，v0.9.12）。
- doctor：载体位 `ok=11 warn=1 fail=0`、装机位 `ok=10 warn=2 fail=0`——warn 均为环境性（doctor 运行目录无 run journal；装机位本不放载体清单），非换装引入。

## §3 Linux 重建与验证

Docker Desktop 冷启动 20s 就绪（engine 29.6.2）；`rust:1.97-slim`＋`D:\tb-eval\build_orz_aliyun_trixie.sh` 重建，exit 0（Cargo release 全链约 11 分钟）。日志 `D:\tb-eval\buildlogs\lin_346_build.log`。

**环境注记**：首次 docker run 因 Git Bash MSYS 路径转换把 `-w /orz/orz` 改写为 `B:/Git/orz/orz` 而 exit 125（载体未被触碰，产物仍为旧 0.9.10）；加 `MSYS_NO_PATHCONV=1` 重跑成功。旧 0.9.10 三件套已先备份为 `*.0.9.10-346-bak`（哈希与 341/342 账面一致）。

| 产物 | 字节 | SHA-256 |
|---|---:|---|
| `orz` | 118,424,344 | `203acbba24118780ac59a85d270193c52257ec57c6f10db2f6cb2182696e28cb` |
| `orz-signer` | 1,397,752 | `1c588a9a4d98a57e6cce1af03e5e1539b4032686a0ebd07fed43f009c36e9080` |
| `orz-acaf-provision` | 1,216,352 | `12e28495a854f4637635e5f04d9bfb6ad88c27bef937de0a97be7a7773930247` |

- Alpine 3.20：`--version`／`--build-info` 均 `0.9.12`，rc0。
- bookworm：`--version` `0.9.12`；doctor 首轮 `fail=1`＝载体清单仍为 0.9.10 旧账（预期中间态），重生成清单后 `ok=10 warn=2 fail=0`（两项 warn 均环境性：无 journal、无浏览器），与 341 基线一致。
- ELF：Type `DYN`、x86-64、**static-pie linked**（无 `PT_INTERP`）。
- Linux 侧不重 provision 载体 acaf（沿 320/336 口径）。

## §4 活体核证与发行

### 4.1 重启与活体

换装后从目标工作区 `C:\Users\1\OneDrive\Desktop\新建文件夹` 启动装机位 `orz.exe web --real`（日志 `D:\tb-eval\buildlogs\orz_346_web_stdout.log`，监听 `127.0.0.1:21487`）：

- `/api/boot`：bridge `0.9.12`／child `0.9.12`／`version_skew=false`／cwd 为目标工作区；
- `/api/conversations`：五会话投影正确（`51da03ea` 活跃、`c23e0aaa`/`073b8892` 已关闭、`54d12bcc`/`d24eadd6` 活跃；无 legacy 短 ID 残留）。
- 重启后旧标签页 token 失效，须用本次启动生成的新 URL（341 已登记行为，未改）。

### 4.2 安装包与哈希

| 资产 | 字节 | SHA-256 |
|---|---:|---|
| `orz-0.9.12-windows-x86_64.zip` | 28,785,082 | `5f4b30d6248a2bbae76081398b130fdaba058b58ce03197a8aeb7c777646c195` |
| `orz-0.9.12-linux-x86_64.tar.gz` | 38,214,983 | `d293df93d57e06522e0f2e53737b2ac417cc25dab9652ad59e896bee102b2b0e` |
| `SHA256SUMS` | 193 | `38412a9b18ae8215043df1c5bbc6f04334c20706be3974387eb94ccb65a3369c` |

每包 6 个顶层 entry（三件可执行＋`carrier-manifest.json`＋`README.md`＋`SHA256SUMS`），包内校验 5/5。本地目录 `D:\tb-eval\rel-346-stage`；解包核证 `D:\tb-eval\rel-346-verify`：双包解包后 `--version`／`--build-info` 均 `0.9.12`，三件与载体原件 BYTE-IDENTICAL 6/6，包内 README 与发行工作树 README BYTE-IDENTICAL 3/3（README 本体 23,651→更新后随包；中英发布面同步 0.9.12，含 §14.90 双轨权限姿态行取代过时的「权限 yolo 自动放行」）。

### 4.3 提交、推送、tag 与 Release、回读

- orz 子仓：`144442d8`（345 处置）＋`dbc6c96d`（bump）推 `cli/feat/fusion-architecture`。
- 父仓发行提交：`4d5661cb` `release(346): publish orz 0.9.12 packages`（README 中英／TODO／BACKLOG／索引 v4.314／orz gitlink）；已推 `origin main`。
- GitHub Release：`orz 0.9.12` 于 **tag `v0.9.12-1`**（→ `4d5661cb`，与远端 main 同点），`isDraft=false`、`isPrerelease=false`、`isImmutable=true`、Latest；URL `https://github.com/SilverWhite/orz/releases/tag/v0.9.12-1`。
- 服务端三资产 size 与 `sha256:` digest 与本地候选逐项一致；下载回读目录 `D:\tb-eval\rel-346-readback`，三件 BYTE-IDENTICAL 3/3，顶层 `SHA256SUMS` 复算 2/2 OK。

## §5 发行事故登记：tag `v0.9.12` 不可复用（本批新发现）

1. **缺陷（已修复于发行前）**：顶层 `SHA256SUMS` 由 Python 文本模式写出，Windows 上落成 **CRLF**（195 B），`sha256sum -c` 报 `\r` 读失败；包内两份（bash printf 生成）为 LF 不受影响。改为显式 LF 重写（193 B，`38412a9b…`）。
2. **事故链**：首发行已带 CRLF 版 sums 上传（Release tag `v0.9.12`，immutable）。删除该 Release 换正确资产时发现：**immutable release 删除后其 tag 名被 GitHub 永久保留**——`tag_name was used by an immutable release`＋ref 创建被拒（GH013 creations restricted；git refs API 同 422），远端 tag 重推同拒；对照探针证实封锁仅限该名字（`v0.9.12-probe` 建删自如）。删除的 release 本体 404 不可恢复、无 restore 端点、immutability 不可解除（PATCH 无效）。
3. **处置**：同提交（`4d5661cb`）改打轻量 tag **`v0.9.12-1`** 重建 Release（再发行后缀惯例，包体版本仍 0.9.12）；README 中英与索引链接随改并注明缘由。
4. **纪律沉淀**：①不可变发行一经创建即**不可删换资产**——上传前必须在本地完成全部校验（含 `sha256sum -c` 实际执行，不能只做逐字节比对）；②临时资产校验文件一律显式 LF（沿 carrier-manifest v0.3 显式 LF 先例）；③若未来再遇同类保留，沿本批后缀惯例处理，不改包体版本。

## §6 残余与边界

1. 本版不新增 macOS 原生包；Linux 仍为构建产物，未做专门体验适配。
2. `v0.9.12` tag 名与首发行 Release（已删）不可恢复；如需在该名下重发，只能待 GitHub 政策变化或联系支持——本批以 `-1` 后缀收口，不作为悬置事项登记。
3. 装机位 0.9.11 三件备份与 ACAF 旧 manifest 保留在盘（回滚路径与 341 §5 同款：停栈→成组还原→重 provision）。
4. 345 档残余清单（encoded-exec 收敛、orz-host Linux 观察模式转必阻断、毒锁助手全量统一等）不因本次发行自动闭合；`linux-tests` CI 面待远端首跑读数。
5. 用户点「恢复并继续」（`c23e0aaa`）在 0.9.11 已自愈过的侧车不受本批影响；活跃会话随桥重启 attach 正常。
