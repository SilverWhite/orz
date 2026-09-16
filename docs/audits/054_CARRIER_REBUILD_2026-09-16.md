# 0.5.4 载体重建、双平台换装与 GitHub Release（2026-09-16）

> 用户 2026-09-16 指示：「请进行重建吧」＋「这一份构建完以后请直接提交并推送吧，
> 双平台的包也一同发布」——本批＝**0.5.4 双平台重建 ＋ 换装 ＋ 提交推送 ＋
> GitHub Release**（0.5.2/0.5.3 批未做的 Linux 载体与 Release 随本指示闭合）。
> **源冻结基线：orz `9b822914`**（`feat/fusion-architecture`；＝ 0ak 实施
> `bafa02b8` ＋ 版本 bump `9b822914`）。上一基线 0.5.3 = `4049bdf0`，基线内容
> 差分＝0aj 复核批 `e7144d8`（护栏单一源化＋声明面分类护栏）＋ 0ak 实施批
> `bafa02b8` ＋ bump，共 3 提交。
> 审计衔接：0ak 实施＝[`0AK_HEADLESS_ARCHIVE_IMPL`](0AK_HEADLESS_ARCHIVE_IMPL_2026-09-16.md)；
> 0aj/0al 复核＝[`0AJ_0AL_REVIEW_HANDLING`](0AJ_0AL_REVIEW_HANDLING_2026-09-16.md)。

## 1. 版本 bump

orz `9b822914`：`crates/orz-bin/Cargo.toml` ＋ `Cargo.lock` 两文件两行，
**0.5.3 → 0.5.4**（`chore(release): … carries 0ak headless session archive +
milestone incremental archive`）。

## 2. Windows 三件套（宿主 release）

- 构建：`cargo build --release -p orz-bin`（`PROTOC=orz\bin\protoc.exe`；
  clean 后全量重建，8m 42s，`CARGO_EXIT=0`，`Compiling orz-bin v0.5.4`）；
  warning 仅既有 dead_code 两项（`xai-tty-utils process_alive`／
  `orz-host register_live_call_job`），无 error。

| 文件 | 大小 (B) | SHA256 |
|---|---|---|
| orz.exe | 53,687,296 | `944e257a5e343fb5b9f958a0fdf6e0fcbe9925383d34a51075b648d6825d332e` |
| orz-signer.exe | 6,738,432 | `80740446135a2a67f926a397a6e80f8a0766548eac748fe124061f0436d7e004` |
| orz-acaf-provision.exe | 6,642,176 | `f0aa43df73d9ee5fba97fe8a52cec6f8b1d2df4d4ece4e0306e179ef15c96929` |

- 与 0.5.3 差异：`orz.exe` 53,624,320 → **53,687,296 B**（+62,976，0ak＋0aj
  复核批代码增量）；signer/provision 大小不变。
- 冒烟（换装后原位复跑，同 0.5.2/0.5.3 形态）：`orz-acaf-provision` 无参
  usage **exit 1**；`orz-signer` 无 signer-manifest fatal **exit 1**。
- 字面量核证（`grep -a -c -o`，与 0.5.3 备份件逐项对照）：
  **`session archive: ` 0.5.3 = 0 → 0.5.4 = 3**（0ak 收尾归档 stderr 指引——
  修复确实进载体）；`blackboard_write` 95 = 95（0aj 标记保持）；
  `ORZ_SLIDER_WINDOW_TOKENS` 1、`hard_950k_intercepted` 7（滑块面保持）；
  退役标记 0——`ORZ_FOLD_TRIGGER_TOKENS` 0、`attention_ladder` 0。
- 换装（`D:\tb-eval\orz-windows`）：**先备份后覆盖**（三件 0.5.3 备份为
  `*.0.5.3-bak`，0.5.0/0.5.1/0.5.2 备份链保留）；换装后逐件 SHA256 与构建
  产物 **MATCH**（三件全对）。

## 3. Linux musl 三件套（Docker）

- 构建：`rust:1.97-slim` ＋ `D:\tb-eval\build_orz_aliyun_trixie.sh`
  （ORZ-BUILD-MOUNT-001 契约 ＋ `MSYS_NO_PATHCONV=1`；`/target` 沿用 0.5.2
  缓存），`LINUX_BUILD_EXIT=0`；构建日志
  `D:\tb-eval\orz-linux\build-20260916-0.5.4.log`。
- **网络摩擦与处置（同 0.5.2 形态，如实登记）**：首轮 apt 五连败——本机
  Docker Desktop `ProxyHTTPMode: manual` → 上游 Clash 时通时断，经代理节点
  到 `deb.debian.org` 间歇 502（脚本 https 强转本次未能幸免；且 apt
  `update` 半失败仍 exit 0，**单跑 update 的预检会假绿**，须以 install 实
  包为准）。处置＝按 0.5.2 既定先例：`settings-store.json` 字节级备份 →
  `ProxyHTTPMode` 临时切 `disabled` → 重启 Docker Desktop（就绪 ~10 s）→
  `apt-get install musl-tools` 实测通过后启动正式构建 → 构建完成后**字节级
  还原并再次重启**（还原后读数 `manual / http://127.0.0.1:7890` 与原值一致，
  备份件 `settings-store.json.052-bak` 留档）。失败首轮日志留档
  `build-20260916-0.5.4.proxy-fail.log`。

| 文件 | 大小 (B) | SHA256 |
|---|---|---|
| orz | 111,067,992 | `eea135858e1c1bfaf1344c5442016dac14e0519af2eb0dcfb4d6f8208826dbcb` |
| orz-signer | 1,397,848 | `a3da64603b993e2e2fc2a0a9ea1e12da2ef8e739a9589dc6717d431694be6093` |
| orz-acaf-provision | 1,216,664 | `9cfd2dd40bea79c676cfa705f1273457b9aac23c4dd2379b46f59a119798295f` |

- 与 0.5.2 差异：`orz` 110,986,272 → **111,067,992 B**（+81,720）；signer
  1,397,864 → 1,397,848；provision 1,216,664 不变。
- ELF 静态核验（Python 解析 ELF 头）：三件均 `e_type=3`（ET_DYN）＋
  `e_machine=62`（x86-64）＋ **PT_INTERP=0**（musl static-pie OK）。
- **双向加载冒烟全绿**（预期 exit 1 形态三件一致）：`debian:bookworm-slim`
  与 `alpine:3.20` 两侧 orz / orz-signer / orz-acaf-provision 均 exit 1。
- 字面量核证（对比 0.5.2 备份件）：**`session archive: ` 0.5.2 = 0 →
  0.5.4 = 1**（0ak 进件）；`blackboard_write` 46 = 46；
  `ORZ_SLIDER_WINDOW_TOKENS` 1；退役标记 0（`ORZ_FOLD_TRIGGER_TOKENS`／
  `attention_ladder`）。
- 换装：构建脚本直写 `/out`（＝ `D:\tb-eval\orz-linux`）——**0.5.3 备份已在
  构建前预留**（`*.0.5.3-bak`，内容实测与 0.5.2 审计哈希逐条一致，即现存
  载体确为 0.5.2；0.5.3 批未重建 Linux），0.5.1 备份链保留。

## 4. 双平台打包与 Release

- 双包（暂存 `D:\tb-eval\rel-054-stage\`，本地件不入库）：
  `orz-0.5.4-windows-x86_64.zip`（26,772,206 B，SHA256
  `0041e88ca723531b381f0627183932c2f9eced5b93402795b2f4399b99528e07`）与
  `orz-0.5.4-linux-x86_64.tar.gz`（35,636,900 B，SHA256
  `cac736718e5544c98b1633034b3ee3f57ceec2ca7045ae79383fef7cd96e3122`）。
  **包内容**＝三件二进制 ＋
  `README.md`（0.5.4 适配：0ak/0aj 更新说明＋快速开始）＋ `SHA256SUMS`
  （包内逐件校验清单；0.5.1 包未带、本批补齐使 README 校验指引真实可执行）。
- 包完整性：暂存目录逐件 `sha256sum -c` 全 OK（Windows/Linux 双侧）；包内
  二进制与换装载体同源同哈希。
- Release：父仓 tag **`v0.5.4`** ＋ `gh release create` 双资产，说明文本含
  源冻结基线、载体二进制哈希表、0.5.4 更新说明与验证摘要（沿 v0.5.1 形态）。

## 5. manifest 与门禁

- `python scripts/generate_orz_source_manifest.py` → **1456 条**；差异面恰
  **2 行**（`Cargo.lock`／`crates/orz-bin/Cargo.toml`）＝ bump 两文件，与
  冻结基线差分完全对应。
- `python scripts/check_repository.py` → **`valid: true` / `error_count: 0`**。

## 6. 边界与遗留

- 本批只做重建、核证、换装、打包与发行；**不改判据状态**——0aj/0ak 判据行
  （无头 run 实证）与 0al 判据（冻结克隆内复验）仍待下一轮狗粮 run 收取
  （用户 2026-09-16 指示暂不开始新狗粮线），0.5.4 即判据收取载体。
- 0.5.3 批遗留的「未知旗标进 TUI 挂住」观察项本批再次亲历
  （`orz.exe --version` 触发，进程已手工回收）；处置仍留后续批次评估
  「未知旗标 fail-fast」。
- 推送与 Release 为用户本批显式指示（0.5.1 先例同形：单项指示不沿用）。
