# 0ac S3 双平台重建记录（2026-09-14）

> 入口：TODO **P0-0ac** / BACKLOG **0ac**——0ac S3①② 落地与审记修复（orz
> `4c892951` / `96d2b263`）之后的**载体重建批**（用户指示：双平台重建完成后
> 直接提交、推送，并把新的双平台安装包发行上 GitHub）。
> 基线：orz `dbb42b1d`（版本 bump **0.5.0 → 0.5.1**，两文件两行）＝**源冻结
> 基线**；父仓构建前 HEAD `28c05968`。
> 产物：Windows x86_64 三件套 + Linux x86_64 musl static-pie 三件套 +
> 双平台安装包 + **GitHub Release v0.5.1** + 载体换装（`D:/tb-eval`）。
> 证据目录：`D:\CLI\_windows_high_nist\evidence-0ac-s3-20260914\`（本地，
> gitignore 覆盖，不入库）；staging：`_windows_high_nist\staging-0ac-s3-20260914\`。

## 0. 版本 bump 与源冻结

- bump：orz `dbb42b1d`，两文件两行（`crates/orz-bin/Cargo.toml` +
  `Cargo.lock`；父仓 `orz_source_manifest.sha256` 同步 2 行）。
- 源冻结基线 = orz `dbb42b1d`，位于 `feat/fusion-architecture` 分支
  （0.5.0 发布后 9 提交：S3①-a 检索侧、S2 合约侧落地、`ea777918`
  宿主资源事件修复、审记 G1/G2 修复、S3① 拆分裁决批等）。
- 版本上下文：0z S3 已发布 0.5.0（源冻结 `1f13e5ec`）；本批为
  **TODO P0-0z「载体重建（0.5.1 冻结基线）」**的落地段（TODO L454 条目
  口径：代际记录 / 适配器锁定值 / 冻结清单核查见 §6）。

## 1. Windows 三件套（宿主 release）

- 构建：`cargo build --release -p orz-bin`（PROTOC=`orz\bin\protoc.exe`）。
  13:15:04 起，编译行 `Compiling orz-bin v0.5.1`，`Finished release
  profile [optimized] target(s) in 2m 33s`，`CARGO_EXIT=0 at
  2026-09-14T13:17:38+08:00`（`windows-build.log`；含既有 `dead_code`
  warning，无 error）。
- 产物（`orz\target\release` → staging → `D:\tb-eval\orz-windows` 同步，
  SHA256 逐对 OK）：

| 文件 | 大小 (B) | SHA256 |
|---|---|---|
| orz.exe | 52,837,376 | `0eff8ef8568ed21d201c45b34e1900afa8a4cd8f7ecbca5234c8d31f45206485` |
| orz-signer.exe | 6,742,528 | `19edf5bd4595b6f839a9a46d344069a235f19e9a3ae153b5f2b4a45116b4fea8` |
| orz-acaf-provision.exe | 6,642,176 | `9fbc26397012b4211269c8b1c062e21ffa4530e706d21fa5d8d2be7cc42ea558` |

- 与 0.5.0 差异：`orz.exe` 52,742,144 → **52,837,376 B**（+95,232，
  S3①②/法官面/资源事件/G1/G2 代码增量）；`orz-signer.exe` 大小不变、
  哈希 `d19a1394…` → `19edf5bd…`（重建）；`orz-acaf-provision.exe`
  `34a4a91a…` → `9fbc2639…`（重建）。
- 冒烟（`windows-smoke-0.5.1.txt`）：provision usage exit 1；signer
  manifest 缺失 fatal exit 1——与上批形态一致。`orz.exe` 无手起探针
  （0.4.x/0.5.0 同口径：无 `--version` 子命令）。

## 2. Linux musl 三件套（Docker）

- 构建：`rust:1.97-slim` + `D:\tb-eval\build_orz_aliyun_trixie.sh`
  （ORZ-BUILD-MOUNT-001 契约 + `MSYS_NO_PATHCONV=1`）容器内构建，编译行
  `Compiling orz-bin v0.5.1 (/orz/orz/crates/orz-bin)` → `Finished release
  profile [optimized] target(s) in 42m 05s`（`linux-build-container-full.log`
  L1787-1788）；`build-20260914-04a4.log.stderr` 空（无致命输出）。
- **操作记录**：13:30 首轮 `BUILD_EXIT=1`——docker API 连接失败
  （`failed to connect to the docker API at
  npipe:////./pipe/dockerDesktopLinuxEngine … daemon is not running`；
  `build-20260914-1330.log`，Docker Desktop 冷机，**0z S3 同形**）；启动
  Docker Desktop 后 13:40 重试进入容器（`build-20260914-1340.log`：apt
  装包 protobuf/musl/tools + rustup），构建终态 14:17 落位。详见 §8 摩擦
  F-022。
- 产物（`D:\tb-eval\orz-linux` 直出；0.4.2 遗留 bak 保留，0.5.0 三件
  哈希见 §4）：

| 文件 | 大小 (B) | SHA256 |
|---|---|---|
| orz | 110,099,784 | `149ab44658f36d4e8dc10877e0a3f45cf6054ebc3fcccd58cfedf8c5e73d4b8d` |
| orz-signer | 1,397,552 | `97978a526894f9aa7032b69f9e0776c2b97e2a90fc290faecda66ddf532d949c` |
| orz-acaf-provision | 1,216,344 | `9ee21fcc8ac7d5970e7c3a76eb09a700ca3b7bed69c0901792ba94004ad3d0c7` |

- 与 0.5.0 差异：`orz` 109,982,680 → **110,099,784 B**（+117,104）；
  `orz-signer` 1,397,360 → 1,397,552；`orz-acaf-provision` 1,216,288 →
  1,216,344。
- 静态核验（`linux-elf-static-check-0.5.1.txt`）：三件均 `e_type=0x3`
  （ET_DYN）+ `e_machine=0x3e`（x86-64）+ **PT_INTERP=0**（musl
  static-pie），`static-pie OK`。
- **双向加载冒烟全绿**（预期 exit 1 形态三件一致）：`debian:bookworm-slim`
  （glibc）与 `alpine:3.20`（musl）两侧 provision usage exit 1 /
  signer manifest 缺失 fatal exit 1 / `orz --version` 无 TTY
  `tui io error` exit 1（`linux-smoke-bookworm-0.5.1.txt` /
  `linux-smoke-alpine-0.5.1.txt`）。

## 3. 打包与资产

- 打包：`package-0.5.1.py`（13:46），双包内容 = orz 主程序 + orz-signer +
  orz-acaf-provision + README（两平台快速开始 + 0.5.1 更新说明，
  `README.md` sha256 `22f1e900fc67c76f8cee5a7231f6f4a3456532b8036036f1e33383954616b4d9`）。

| 资产 | 大小 (B) | SHA256 |
|---|---|---|
| orz-0.5.1-linux-x86_64.tar.gz | 34,987,981 | `49df6cebbce12375cb2969f2bed03dfdfc8f4b02f5b89764f6475fb05ce7cdf0` |
| orz-0.5.1-windows-x86_64.zip | 26,472,460 | `f7e8274c706c9b0cf202ccc126d220e4719097bf98847fe3478c6b368689432f` |

- 包成员核验（`archive-verify-0.5.1.txt`）：tar.gz 4 成员 + zip 4 成员
  逐对 sha256 与 `D:\tb-eval` 载体 / staging **`match=True` ×8**。
- 压缩复算（`compress-probe-0.5.1.txt`）：成员 gzip-9 复算合 34,985,505 B
  ≈ 实际 tar.gz 34,987,981 B（差异 2,476 B，打包容器头尾），`ratio≈3.22`；
  0.5.0 同项记录 26,470,950 B（压缩参数差异，非缺件——成员核验为准）。
- 接线符号/字面量核证（`literal-scan-0ac.txt`、`binary-symbols-*.txt`）：
  `local_segmented` **6 / 54**、`retrieval_family` **1 / 4**、
  `ORZ_RETRIEVAL_ENGINES` / `ORZ_WEB_SEARCH_LOCAL` 各 **2 / 2**、版本字面量
  `0.5.1` **24 / 17**；浏览器/检索/轮次接线标记（`browser_launch_result` /
  `browser_control` / `retrieval_enabled` / `wait_load` / `engine_attempts` /
  `low_quality` / `[INITIAL_ROUND_INQUIRY]`）双平台全命中；三事件名
  （`retrieval_progress` / `retrieval_result_segment` / `result_delivered`）
  与 `probe_scope` / `retrieval_family_probe` 均 **0 / 0**——与 ①-b
  投递侧未落一致（BACKLOG 0ac「G3 仍 open」）。
- manifest 重算：`python scripts/generate_orz_source_manifest.py` →
  `wrote 1448 entries`（差异面恰 2 行：`crates/orz-bin/Cargo.toml` +
  `Cargo.lock`）；`python scripts/check_repository.py` →
  `"error_count": 0` / `"valid": true`（EXIT=0）。

## 4. 载体换装（`D:/tb-eval`）

- **windows**（`windows-carrier-sync-0.5.1.txt`）：swap 方法 =
  `fresh-copy-to-tmp -> rename old to *.0.5.0-bak -> move tmp into place`
  （**运行中宿主进程持旧镜像，rename 可行**——摩擦 F-023）；post-swap
  三件与 staging 逐对 **`MATCH=True` ×3**。留档：`orz.exe.0.5.0-bak` /
  `orz-signer.exe.0.5.0-bak` 两件实物（0.5.0 哈希 `a45b60e5…` /
  `d19a1394…` 可对照）；`orz-acaf-provision.exe` 换装前即与本批同哈希
  `9fbc2639…`（13:34 `windows-hashes.txt` 已测，未产生 bak；0.5.0 值
  `34a4a91a…` 存于 0z 记录与 GitHub v0.5.0 资产——已下载对照
  `gh-v0.5.0-download\`）。
- **linux**：构建脚本直出（三件 14:17 落位）；0.5.0 三件哈希（`393eee34…` /
  `3d5c8155…` / `73fcaeda…`）见 [0z S3 审计](0Z_S3_DUAL_PLATFORM_REBUILD_2026-09-13.md)
  与 v0.5.0 资产；0.4.2 bak 三件为历史遗留（09-11）。
- 0.5.0 → 0.5.1 汇编对照已由 `archive-verify`（成员级）+ carrier-sync
  （文件级）双重核验闭合。

## 5. 提交、推送与发布

- **子模块**：`git -C orz push cli feat/fusion-architecture` →
  `96d2b263..dbb42b1d  feat/fusion-architecture -> feat/fusion-architecture`
  （9 提交一次推送；远端核验 `dbb42b1d…refs/heads/feat/fusion-architecture`）。
- **父仓**：C1 `c4491629`（2 files：子模块指针 0.5.0 段 → `dbb42b1d` +
  manifest 2 行）→ `git push origin main` → `28c05968..c4491629
  main -> main`。
- **GitHub Release v0.5.1**（`SilverWhite/CLI`）：
  `https://github.com/SilverWhite/CLI/releases/tag/v0.5.1`，
  `publishedAt 2026-09-14T07:32:43Z`，`targetCommitish=main`（= C1），
  双资产 `state=uploaded`；**服务端 digest 与本地逐字一致**：
  tar.gz `sha256:49df6ceb…cdf0` / zip `sha256:f7e8274c…432f`，
  size 34,987,981 / 26,472,460。
- **发布后下载核验**：`gh release download v0.5.1` →
  `evidence-0ac-s3-20260914\gh-v0.5.1-download\`，重哈希
  `49DF6CEB…CDF0` / `F7E8274C…432F` 与 staging 资产**三方一致**。
- **账本回写（C2）**：本审计 + 索引 v3.20 + TODO/BACKLOG +
  摩擦台账 F-021…F-023 同批提交推送（收尾段自我报告另追加
  **F-024**，见 §8）——**C2 = `8c7529ee`**（6 files
  +316/−5，含本审计与 `releases/orz-0.5.1-x86_64/README.md` 新建）；
  推送回执 `c4491629..8c7529ee main -> main`（`origin/main` 头 = C2）；
  门禁 `python scripts/check_repository.py` → `"error_count": 0` /
  `"valid": true`（EXIT=0，含本批全部文档改动）。

## 6. 配套项（TODO L454「代际记录 / 适配器锁定值 / 冻结清单 `harness_artifacts` 同批更新」核查）

- **代际记录**：本批产出＝本审计 + [releases 条目](../../releases/orz-0.5.1-x86_64/README.md)
  （0.5.1 六件哈希 + 源冻结基线 `dbb42b1d`）+ §4 换装留档。
- **适配器锁定值**：落点 `scripts/run_r0_heavy_official.py`
  （`EXPECTED_CARRIER_SHA256` = 0.5.0 载体、`EXPECTED_ADAPTER_SHA256` =
  适配器 `tb_agents\orz.py` `2737cfad…`）。**本轮未改动脚本**：该值为
  **第 0 轮起跑身份**；适配器文件本轮实测仍为 `2737cfad…`（未变）。脚本
  对载体重建后的身份 DRIFT 有内建告警与显式放行开关
  （`--allow-identity-drift`，见其 L362-372），其更新属下一跑批起跑准备。
- **冻结清单 `harness_artifacts`**：落点
  `evaluation/corpus-freeze/tb21-official-89-2026-09-13.manifest.json`
  （第 0 轮装备快照：适配器 + 0.5.0 载体哈希；`corpus_digest
  e6aae82d…`）。**冻结语义下为历史物证，本轮不改动**。

## 7. 边界与遗留

- 本批只做重建、加载冒烟与字节串核证；**实机行为另行放行**：0z S4
  资源门/回收/树杀实机复跑（判据 1–13）；0ac 检索/投递实机取证
  （①-b 落码后）。
- **0ac 维持 open**：①-b 投递侧（三事件 `EventType` 变体 + 族注册 +
  产品码写点、I1–I3、M1–M3、子代理提前收口、semaphore 独立截止）未落；
  G3 状态不变；⑤⑥⑦ 未落。
- TODO L652 S4 条目载体口径已更新为 0.5.1（见账本回写）。

## 8. 摩擦（台账同步：F-021…F-024）

- **F-021**（装置侧·外部网络面，观察）：`docker pull rust:1.97-slim`
  失败——**Docker Desktop 无 HTTPS 代理、直连 `registry-1.docker.io:443`
  超时**（`docker-pull-rust.log`："…because Docker Desktop has no HTTPS
  proxy…dial tcp 162.125.34.133:443…failed to respond"）；绕行事实生效：
  本地既有 `rust:1.97-slim`（5 周前拉取、1.27 GB）支撑全部后续构建，
  **未阻塞**（Windows `CARGO_EXIT=0` / 容器 `Finished … 42m 05s`）。
- **F-022**（装置侧·容器环境，fixed）：13:30 首轮容器构建
  `BUILD_EXIT=1`——docker daemon 未运行（`failed to connect to the
  docker API at npipe://…`；Docker Desktop 冷机）；启动后 13:40 重试
  成功。**2026-09-13 0z S3 同形**（其审计 §2 操作记录）。
- **F-023**（装置侧·文件占用，fixed/绕行）：载体目录内**运行中宿主进程
  持旧镜像**致原位替换不可行；swap 记录明示
  `rename old to *.0.5.0-bak -> move tmp into place`（rename 绕行成功，
  post-swap `MATCH=True` ×3）。
- **F-024**（模型习惯·自报，收尾段）：收尾核证开始时对运行时会话卷
  `D:\CLI\.gsa\ledger\current.md` 发起 1 次编辑，被工具拒绝（原文
  `inside the runtime-owned .gsa session volume, which is not
  model-writable`）；该调用零写入、无内容变更，**代价 1 次工具调用**。

> **本批机械核证留痕（RUN-CLI-6aa77e19，重建与发布段实做输出摘录）**
> - `docker pull rust:1.97-slim` → `Error response from daemon: failed to
>   resolve reference "docker.io/library/rust:1.97-slim" … dialing
>   registry-1.docker.io:443 container via direct connection because
>   Docker Desktop has no HTTPS proxy … host has failed to respond`
>   （`evidence\docker-pull-rust.log`）。
> - 容器首轮 → `failed to connect to the docker API at
>   npipe:////./pipe/dockerDesktopLinuxEngine … the daemon is not running`
>   / `BUILD_EXIT=1`（`D:\tb-eval\orz-linux\build-20260914-1330.log`）；
>   重试 → 进入容器（`…-1340.log`）。
> - Windows 构建 → `Compiling orz-bin v0.5.1` / `Finished release …
>   in 2m 33s` / `CARGO_EXIT=0 at 2026-09-14T13:17:38`。
> - Linux 构建 → `Finished release [optimized] target(s) in 42m 05s`
>   （`linux-build-container-full.log` L1788）。
> - ELF → `e_type=0x3 e_machine=0x3e PT_INTERP=0 PT_LOAD=4 … static-pie
>   OK`（三件，`linux-elf-static-check-0.5.1.txt`）。
> - 冒烟 → bookworm/alpine `EXIT_CODE=1` ×3 预期形态（两文件）；
>   windows provision/signer `exit=1`。
> - `archive-verify` → 8 成员 `match=True` ×8；`compress-probe` → gzip-9
>   复算 34,985,505 ≈ 实际 34,987,981。
> - `literal-scan` → `local_segmented` 6/54、`retrieval_family` 1/4、
>   `0.5.1` 24/17、三事件名 0/0。
> - carrier-sync → post-swap `MATCH=True` ×3；`swap method=fresh-copy-to-tmp
>   -> rename old to *.0.5.0-bak -> move tmp into place`。
> - `python scripts/generate_orz_source_manifest.py` → `wrote 1448
>   entries`；`python scripts/check_repository.py` → `"error_count": 0` /
>   `"valid": true`。
> - `git -C orz push cli feat/fusion-architecture` → `96d2b263..dbb42b1d`;
>   `git -C orz ls-remote cli refs/heads/feat/fusion-architecture` →
>   `dbb42b1d0ac5be56e68aa4e5ca307ff168f465a6`。
> - `git -C D:\CLI push origin main` → `28c05968..c4491629 main -> main`。
> - `gh release create v0.5.1 …` → `https://github.com/SilverWhite/CLI/releases/tag/v0.5.1`;
>   `gh release view v0.5.1` → `isDraft=false`、`targetCommitish=main`、
>   assets `sha256:49df6ceb…`（34,987,981）/ `sha256:f7e8274c…`（26,472,460）。
> - `gh release download v0.5.1` → 重哈希 `49DF6CEB…CDF0` /
>   `F7E8274C…432F`（与 staging 三方一致）。
