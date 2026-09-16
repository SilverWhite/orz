# 0.6.0 载体重建、双平台换装与 GitHub Release（2026-09-16）

> 用户 2026-09-16 指示：「请进行重建吧。这一轮就定为0.6.0好了，请顺便将双平台包
> 发布到github上」——本批＝**0.6.0 双平台重建 ＋ 换装 ＋ GitHub Release**。
> **源冻结基线：orz `5041c3dc`**（`feat/fusion-architecture`；＝ 0ah v8 实现
> 更正＋审查处置＋R-12 余项 `4952bc8c` ＋ 0af 文案与 F-BE-12 残留完成
> `b6ed78d9` ＋ 0ah 收口清理 `501447c0` ＋ 版本 bump）。上一基线 0.5.4 =
> `9b822914`，基线内容差分＝上述 3 提交 ＋ bump 共 4 提交。
> 审计衔接：0ah v8＝[`0AH_V8_IMPLEMENTATION`](0AH_V8_IMPLEMENTATION_2026-09-16.md)；
> 0af/0ah 收口＝[`0AF_0AH_CLEANUP`](0AF_0AH_CLEANUP_2026-09-16.md)。
> 说明：0.5.4 载体（`9b822914`）不含 0ah v8（`4952bc8c` 落码在 0.5.4 冻结之后）
> ——0.6.0 才是 0ah 五项真机读数与 0.77 换算复测的收取载体。

## 1. 版本 bump

orz `5041c3dc`：`crates/orz-bin/Cargo.toml` ＋ `Cargo.lock` 两文件两行，
**0.5.4 → 0.6.0**（`chore(release): … carries 0ah v8 model-face projection +
review handling + cleanup, 0af resource-gate denial copy + F-BE-12 residual`）。

## 2. Windows 三件套（宿主 release）

- 构建：`cargo clean` 后全量重建 `cargo build --release -p orz-bin`
  （`PROTOC=orz\bin\protoc.exe`；10m 07s，`CARGO_EXIT=0`，
  `Compiling orz-bin v0.6.0`）；warning 仅 orz-host lib 1 项（既有
  dead_code 同族），无 error。

| 文件 | 大小 (B) | SHA256 |
|---|---|---|
| orz.exe | 53,875,712 | `a642b8d143369bf97dbc314889846ab6687a03da9494c1b147cd07dea767eca2` |
| orz-signer.exe | 6,742,528 | `ade2db9a7bca5fac5c288de67271b90e6d043a294477eab37491965d23fb7b49` |
| orz-acaf-provision.exe | 6,642,176 | `cc84b53ccc82cbaf7b1753d25b19f0dea65a47b0deb5b349c43860a2fb32309d` |

- 与 0.5.4 差异：`orz.exe` 53,687,296 → **53,875,712 B**（+188,416，0ah v8＋
  0af 代码增量−折叠族删除净效应）；signer +4,096；provision 大小不变。
- 冒烟（构建位与换装位各跑一遍，同 0.5.2–0.5.4 形态）：`orz-acaf-provision`
  无参 usage **exit 1**；`orz-signer` 无 signer-manifest fatal **exit 1**。
- 字面量核证（`grep -a -c -o`，与 0.5.4 备份件逐项对照，双平台同表）：

| 字面量 | 0.5.4 | 0.6.0 | 判读 |
|---|---|---|---|
| `session archive: ` | 3（Win）/1（Lin） | 3 / 1 | 0ak 保持 |
| `blackboard_write` | 95 / 46 | 94 / 45 | 0aj 保持（±1 为重构位移） |
| `ORZ_MODEL_FACE_SLIDER_TOKENS` | 0 | 1 | v8 主滑块 env 进件 |
| `context_scale:hard_truncate` | 0 | 7 | v8 T1 键进件 |
| `ORZ_SLIDER_WINDOW_TOKENS` | 1 / 1 | 0 | v7 H env 随 v8 退役 |
| `hard_950k_intercepted` | 7 / 7 | 0 | 950K 语义由 T1 500K 取代 |
| `宿主机内存/储存资源即将耗尽` | 0 | 1 | 0af 定案句进件 |
| `读数不可得` | 0 | 1 | 0af Unknown 变体句进件 |
| `（前略）` | 1 / 1 | 0 | 收口清理：桥截断族已退役 |
| `ORZ_FOLD_TRIGGER_TOKENS` / `attention_ladder` | 0 / 0 | 0 / 0 | 既有退役面保持 |

- 换装（`D:\tb-eval\orz-windows`）：**先备份后覆盖**（三件 0.5.4 备份为
  `*.0.5.4-bak`，换装前现载体哈希与 054 审计逐位一致核证＝944e257a／
  80740446／f0aa43df）；0.5.0–0.5.3 备份链保留；换装后逐件 SHA256 与构建
  产物 **MATCH**（三件全对）。

## 3. Linux musl 三件套（Docker）

- 构建：`rust:1.97-slim` ＋ `D:\tb-eval\build_orz_aliyun_trixie.sh`
  （ORZ-BUILD-MOUNT-001 契约 ＋ `MSYS_NO_PATHCONV=1`；`/target` 沿用缓存），
  `LINUX_BUILD_EXIT=0`（apt→rustup→静态 rg→`-j 1` 全链完成）；构建日志
  `D:\tb-eval\orz-linux\build-20260916-0.6.0.log`。
- **代理切换按 052/054 先例**：构建前 `settings-store.json` 字节级备份
  （`.060-bak`）→ `ProxyHTTPMode` 临时切 `disabled` → 重启 Docker Desktop →
  apt `update+install` 实包一次通过（无 502 重试）→ 构建完成后**字节级还原
  并再次重启**（还原后 `manual` 读数一致，备份件留档）。本轮未经历 052/054
  的 apt 五连败——disabled 切换后直连稳定。
- Linux 构建告警注记：`orz-host` (lib) **3 warnings**（Windows 同批 1）——
  差额为 Linux-only cfg 分支面既有告警，非本批新增（0.5.4 Linux 构建同源
  形态，日志留档可查），不影响产物。

| 文件 | 大小 (B) | SHA256 |
|---|---|---|
| orz | 111,256,888 | `f9b4b2ed83e94a5dbef4b8a17d73a555ef1774c188b95a50f11a2c446b939326` |
| orz-signer | 1,397,808 | `71082ac11945426dcd77188482ab6d96b2b3b7ac1214993b5bf85e8ab2a1cf90` |
| orz-acaf-provision | 1,216,608 | `439f6550eecdb466dd0f67f1e5a629303f6432db8bd9f1f8639e35fd7eff367a` |

- 与 0.5.4 差异：`orz` 111,067,992 → **111,256,888 B**（+188,896）；signer
  −40；provision −56。
- ELF 静态核验（Python 解析 ELF 头）：三件均 `e_type=3`（ET_DYN）＋
  `e_machine=62`（x86-64）＋ **PT_INTERP=0**（musl static-pie OK）。
- **双向加载冒烟全绿**（预期 exit 1 形态三件一致）：`debian:bookworm-slim`
  与 `alpine:3.20` 两侧 orz / orz-signer / orz-acaf-provision 均 exit 1。
- 字面量核证：见 §2 表（Linux 列与 Windows 同形态翻转）。
- 换装：构建脚本直写 `/out`（＝ `D:\tb-eval\orz-linux`）——0.5.4 备份在构建
  前预留（`*.0.5.4-bak`，`orz` 备份哈希 `eea13585…` 与 054 审计逐位一致核证）；
  0.5.1/0.5.3 备份链保留。

## 4. 双平台打包与 Release

- 双包（暂存 `D:\tb-eval\rel-060-stage\`，本地件不入库）：
  `orz-0.6.0-windows-x86_64.zip`（26,824,689 B，SHA256
  `01e06bd5728b01fbd7e2620a11c81947737f9eb368bfd25088b0199ef2fcb5ba`）与
  `orz-0.6.0-linux-x86_64.tar.gz`（35,742,096 B，SHA256
  `27631ef0b1aeaf8328fc4feedd0de857ea0bcce0ffb608e4dd0d0a9be47aaabe`）。
  **包内容**＝三件二进制 ＋ `README.md`（0.6.0 适配：0ah v8／0af／清理更新
  说明＋两平台快速开始）＋ `SHA256SUMS`（包内逐件校验清单）。
- 包完整性：暂存目录逐件 `sha256sum -c` 全 OK（Windows/Linux 双侧）；
  解包回读三件二进制与换装载体同源同哈希（zip 逐件读回＋tar 解包比对全
  MATCH）。
- Release：父仓 tag **`v0.6.0`** ＋ `gh release create` 双资产，说明文本含
  源冻结基线、载体二进制哈希表、0.6.0 更新说明与验证摘要（沿 v0.5.4 形态）。

## 5. manifest 与门禁

- `python scripts/generate_orz_source_manifest.py` → **1457 条**；差异面恰
  **2 行**（`Cargo.lock`／`crates/orz-bin/Cargo.toml`）＝ bump 两文件，与
  冻结基线差分对应（0ah v8/0af/收口三提交的源码行已随前批 manifest 入账）。
- `python scripts/check_repository.py` → **`valid: true` / `error_count: 0`**。

## 6. 边界与遗留

- 本批只做重建、核证、换装、打包与发行；**不改判据状态**——0aj/0ak 判据行
  （无头 run 实证）与 0al 判据（冻结克隆内复验）仍待下一轮狗粮 run 收取；
  **0.6.0 即判据收取载体**（0ah 五项真机读数＋0.77 换算复测同 run 按实施
  回执 §9 配方收取）。
- 「未知旗标进 TUI 挂住」观察项（054 遗留）本批未触碰（`--version` 未作为
  冒烟项）；处置仍留后续批次评估「未知旗标 fail-fast」。
- 推送与 Release 为用户本批显式指示（0.5.1 先例同形：单项指示不沿用）。
