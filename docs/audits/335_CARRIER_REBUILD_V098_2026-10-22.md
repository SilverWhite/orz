# 335 批：0.9.8 双平台载体重建＋Windows 换装（2026-10-22）

> **用户令**：「请进行重建吧」，随后「请先进行推送吧，安装包先不发行」。**本批**＝0.9.7→0.9.8 代窗口双平台重建（沿 320/327 形态：版本 bump→Windows release 构建→Linux musl 容器构建→ACAF 重 provision→双平台换装/清单→冒烟与 334 新面活体核证→落账→推送）。**零源码语义改动**（仅版本号两行）；**已推送父仓 `main`＋orz `cli/feat/fusion-architecture`；未建 tag、未发 GitHub Release、未跑 0dn S2 真机十条**；计数 64 不变。

## §0 结论速览

| 面 | 结果 |
|---|---|
| 版本 bump | `orz/crates/orz-bin/Cargo.toml`＋`Cargo.lock` **恰两行 0.9.7→0.9.8**；`cargo metadata --locked` exit 0；orz 提交 **`c3faec70`** |
| Windows 构建 | `cargo build -p orz-bin --release -j 2` **exit 0（3m29s，暖缓存）**；**0 warning**；`orz.exe` **59,261,952 B `e3df4308…`**／signer **6,740,480 B `49ea0702…`**／provision **6,640,128 B `10917f3b…`** |
| Windows 换装 | 载体位＋装机位 **MATCH 3/3**；两侧 `.0.9.7-bak` 预建（前件 `ae8b12d0…`／`d02bb021…`／`3a3a6ae9…`）；`--build-info` 双点 `0.9.8 os=windows arch=x86_64 profile=release` |
| ACAF | 载体 `acaf` 重 provision **exit 0**；旧 manifest 备份 `signer-manifest.json.bak-20261022-335`；新 `binary_sha256=49ea0702…` ↔ signer 逐位一致 |
| Windows 清单 | `D:/tb-eval/orz-windows/carrier-manifest.json` **v0.9.8 / 3 entries**；carrier 位 `orz doctor` **ok=12 warn=0 fail=0**（载体 3 件复验通过） |
| Linux 构建 | docker `rust:1.97-slim`＋trixie 脚本 **exit 0**（cargo `Finished … 7m14s`；容器墙钟约 10 分）；`orz` **118,384,560 B `8f61631f…`**／signer **1,397,896 B `e649aa6c…`**／provision **1,216,344 B `83e72a9a…`** |
| Linux 形态 | 三件直写 `/out`；ELF `e_type=3 (DYN)`、`e_machine=62`、**无 PT_INTERP**（static-pie）；alpine 3.20／bookworm-slim 双冒烟 `0.9.8 os=linux` rc0 |
| Linux 清单 | `D:/tb-eval/orz-linux/carrier-manifest.json` **v0.9.8 / 3 entries**；bookworm `orz doctor` **ok=11 warn=1 fail=0**（唯一 warn＝容器无 Chrome，属预期） |
| 334 新面活体 | 真实 WS：`session/new→created`、`session_close→closed/already=true`；关闭后 prompt **-32010＋四要素 data**；`session_delete→deleted`；版本偏斜 **bridge 0.9.8 vs 磁盘 0.9.9 → version_skew=true** |
| 冒烟 | WIN `RUN-CLI-6ad9eb66` rc0（11 events，`run_finished/completed`，swept 前轮＋归档 941 B）；LIN alpine `6ad9ed53→6ad9ed5b`（swept＋归档 3323 B）＋bookworm `6ad9ed64` 均 rc0 |
| 磁盘 | 构建前 D: **6.13 GB free**；清 `target/debug/incremental` **10.12 GB**；构建后清 `target/debug` 缓存 **55.11 GB**；终态 **61.84 GB free** |
| 门禁 | `check_repository.py` 提交后 **valid:true / error_count:0 / warning_count:4**；`orz_source_manifest.sha256` **1494 条** `--check` valid |
| 推送 | 父仓 `main` `827ae53e..baf7260f`、orz `cli/feat/fusion-architecture` `4bf16094..c3faec70` **推送成功**；远端 ref 回读一致；**无 `v0.9.8` tag、无 Release** |

## §1 版本与源冻结

- `orz-bin` 包版本 `0.9.7→0.9.8`；`Cargo.lock` 同名包版本同步。仅此两行，`cargo metadata --locked --offline` exit 0。
- orz 提交 **`c3faec70`**（`chore(release): bump orz 0.9.8`）；父仓 pin 与源清单在 §7 提交中更新。
- 0.9.8 进体面＝334 修复批 `cae76d3e`（CLI 删除门／版本偏斜／桥 id/审批重放／状态机／跨区根／doctor／上下文等）＋版本 bump。

## §2 Windows 重建、换装与 ACAF

**构建**

- `cmd /c "cargo build -p orz-bin --release -j 2 > D:\tb-eval\buildlogs\win_098_build.log 2>&1"` → **exit 0，3m29s，0 warning**。
- 三件尺寸/hash：

| 文件 | 0.9.7（bak） | 0.9.8 | 变化 |
|---|---|---|---|
| `orz.exe` | 58,833,408 `ae8b12d0…` | **59,261,952 `e3df4308…`** | +428,544 B |
| `orz-signer.exe` | 6,740,480 `d02bb021…` | **6,740,480 `49ea0702…`** | Δ0 B（版本字面变） |
| `orz-acaf-provision.exe` | 6,640,128 `3a3a6ae9…` | **6,640,128 `10917f3b…`** | Δ0 B（版本字面变） |

**换装**

- 载体位 `D:/tb-eval/orz-windows` 与装机位 `C:/Users/1/AppData/Local/orz/bin` 均先建 `.0.9.7-bak`，再以 release 件覆盖；两侧 **MATCH 3/3**。
- 装机位 `--build-info`＝`version=0.9.8 os=windows arch=x86_64 profile=release` rc0；`orz --version`＝`orz 0.9.8 (windows/x86_64/release)` rc0。
- 换装前无 `orz*` 进程在跑（无锁文件/`.swap-old` 新增）。

**ACAF**

- 旧 manifest 备份 `D:/tb-eval/orz-windows/acaf/signer-manifest.json.bak-20261022-335`。
- `orz-acaf-provision.exe <acaf>/keystore <acaf>/signer-manifest.json` → exit 0；`binary_sha256=49ea0702…` 与新 signer 逐位一致；keystore 复用未覆盖。
- 载体位 `orz doctor`：`[ok] carrier: 载体清单复验通过（3 件）`，`summary: ok=12 warn=0 fail=0`。

## §3 Linux 构建与形态

- docker：`rust:1.97-slim`，mount 源 `/orz`、宿主 target `/target`、输出 `/out`；脚本 `scripts/build_orz_aliyun_trixie.sh`（与 `D:/tb-eval/build_orz_aliyun_trixie.sh` 同 hash）。
- **exit 0**；cargo `Finished release … 7m14s`；日志 `D:/tb-eval/buildlogs/lin_098_build.log`。
- 三件：

| 文件 | 0.9.7（bak） | 0.9.8 |
|---|---|---|
| `orz` | 117,752,808 `7416d397…` | **118,384,560 `8f61631f…`** |
| `orz-signer` | 1,397,952 `f6321d1b…` | **1,397,896 `e649aa6c…`** |
| `orz-acaf-provision` | 1,216,400 `015b493c…` | **1,216,344 `83e72a9a…`** |

- ELF 头解析：`e_type=3`（DYN）、`e_machine=62`（x86_64）、**无 PT_INTERP** ⇒ static-pie。
- 双镜像 `--version`／`--build-info` 均 `0.9.8 os=linux` rc0；bookworm `doctor`：`ok=11 warn=1 fail=0`（warn＝容器无 Chrome）。
- Linux 不重 provision（沿 320 口径；冒烟在容器内 `/tmp/acaf` 临时 provision）。

## §4 冒烟与 334 新面活体

**Windows（`D:/tb-eval/smoke-098`，`--fake-provider -p`）**

- 首跑未带 ACAF env → fail-closed 拒绝（预期启动门）；带 `ORZ_ACAF_*` 后 `RUN-CLI-6ad9eb66` **rc0**，10 sequence＋run_finished 共 **11 events**；`run_finished` status=completed、`terminal_source=short_text`；启动 sweep 收卷前轮 `6ad9eb61`，`archives/6ad9eb61.json.gz` 941 B。
- 0.9.8 新面：
  - `orz --version` 单行 0.9.8（旧 0.9.7 无此命令，会落 TUI——已实测并避免记录为失败）。
  - `orz doctor` 11 节（version/cwd/gsa/sessions/runs/conversations/compaction/carrier/retrieval/web/token/cdp）`ok=11 warn=1 fail=0`。
  - `delete-session` 无令牌 **exit 2**；16 字符令牌→「无数据可删除」**exit 1**（鉴权门先于存在性）。
- **版本偏斜真实触发**：复制 0.9.8 为 `skew098/orz.exe` 启动 `orz web`，运行中把磁盘同名文件替换为打印 `orz 0.9.9` 的探针；`/api/boot` 返回：
  `bridge_version=0.9.8`、`child_version=0.9.9`、`child_probe=ok`、`version_skew=true`。证明 334 §3.1 有生产触发路径（旧实现恒 false）。
- **真实 WS 生命周期**（Python `websockets` 15.0.1，token 门）：
  - attach 前置帧 `_orz/bridge_attached`（session_id null、cwd 正确）；
  - `initialize` 1.0；`session/new` 得 `d5a61906…`；`session_state`=`created`；`session_close`→`closed/already=false`；再查 `closed/already=true`；
  - 另建会话关闭后 `session/prompt`→**-32010**，`data={code:session_not_found,class:session,hint:…,evidence:session=…}`；
  - `_orz/session_delete`→`deleted/already=false`；再查 state=`deleted/already=true`。

**Linux（alpine 3.20／bookworm-slim）**

- alpine：`6ad9ed53` 首轮 rc0 → `6ad9ed5b`（swept `6ad9ed53`，归档 3323 B）rc0；journal 11 events。
- bookworm：`6ad9ed64` rc0；journal 11 events。两镜像均无 carrier-integrity 警告（清单刷新后复跑）。
- 容器内临时 provision `binary_sha256=e649aa6c…` ↔ Linux signer。

## §5 清单与核证

- Windows carrier：`carrier-manifest.json` v0.9.8，3 entries（三件套）；`generate_carrier_manifest.py`。
- Linux carrier：v0.9.8，3 entries；同脚本。
- 装机位无 carrier-manifest（沿 321 口径：装机位非载体目录，启动不校验；日常启动由载体位清单覆盖）。
- 载体内 `doctor` 对 Windows carrier 3 件、Linux carrier 3 件均复验通过。
- 冒烟目录为 `D:/tb-eval/smoke-098`、`smoke-098-lin`、`smoke-098-lin-bookworm`，不入仓。

## §6 磁盘与构建告警

- 构建前 D: free **6.13 GB**；先删 `orz/target/debug/incremental` **10.12 GB**（free 15.46 GB）。
- 构建后 Linux 容器缓存增长，free 降至 5.15 GB；删除 `orz/target/debug` 缓存 **55.11 GB**，终态 **61.84 GB free**（`target/release` 11.47 GB 与 `D:/tb-eval/orz-target` 10.24 GB 保留）。
- Windows 0 warning；Linux 8 条代码告警（orz-config 1／orz-assurance 4／orz-host 2＋1 汇总行）＋2 条 rustup/ripgrep 工具提示——均为既有 musl cfg 门控/未使用变量，非本批引入；日志在案。

## §7 台账与路由

- 本档：`docs/audits/335_CARRIER_REBUILD_V098_2026-10-22.md`。
- orz：bump 提交 `c3faec70`（父仓 pin 随本档提交）。
- 父仓：本档＋索引 v4.306＋TODO/BACKLOG 指针＋`orz_source_manifest.sha256`（1494 条）＋`orz` 子模块指针。
- **已推送**：父仓 `main` → `baf7260f`；orz `cli/feat/fusion-architecture` → `c3faec70`；远端 ref 回读一致。
- **未建 tag／未发 Release**（用户令「安装包先不发行」）；README 发布面保持 v0.9.7 对应说明（发布批另做）。
- 计数 64 不变；334 修复批仍 `AUTH-334-FIX-328-333`，本批新增 `AUTH-335-REBUILD-V098`（`reference`）。

## §8 边界与如实记

1. **已推送但未发行**：父仓 `main` 与 orz `cli` 分支已推送；GitHub Release／tag／安装包打包未做（按用户令「安装包先不发行」）。
2. **0dn S2 未跑**：真机十条场景仍待 0.9.8 载体上执行；本批只证「可构建、可换装、可启动、334 新面在体」。
3. **Linux 未换装到评测宿主机位**：产物在 `D:/tb-eval/orz-linux` 与 `/target` 缓存；评测容器/宿主换装另批。
4. **版本偏斜探针的旧载体边界**：0.9.7 无 `--version`，3 s 超时后 `child_version=null / version_skew=null`（不妄判）；0.9.9 假探针用于证明 true 分支。
5. **ACAF**：Windows 载体 keystore 复用、manifest 刷新；装机位无 acaf 面（沿 321 口径）。Linux 仅容器内临时 provision，未动载体 acaf 面。
6. **构建缓存清理**：删除 `target/debug` 是缓存操作，不触源码/载体；后续 debug 测试需重编。
