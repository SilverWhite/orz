# 0.7.0 双平台载体重建与换装（2026-09-26）

> **本批＝用户令「请进行重建吧，本次重建做双平台重建，序号0.7.0」**——0bs c 轮树面落码（⑨⑩ 三引擎/指纹／⑧ 降级序／⑫ 垂直源／⑬ 信任面／⑪ 门禁收尾，[`0BS_PROGRESS_2026-09-26c.md`](0BS_PROGRESS_2026-09-26c.md)，未提交）随本批进双平台在役载体（Linux 腿为 **078/079 两批以来首次补跑**，0.6.15 → 0.7.0 直跨）。
> **版本 bump**（0.6.17 → **0.7.0**，树面两文件两行）② Windows 增量重建＋Linux musl Docker 重建（直写换装位）③ 双侧换装＋字面量核证＋ELF 静态核验＋双向加载冒烟＋ACAF 与装配复核＋载体级 Web 探针。
> **未提交／未推送（沿未提交口径）／未发行；本地打包未做。**

> **后批补记（081 批，2026-09-26）**：本行三项**已结清**——用户令「请进行提交与推送吧，0.7.0也发布上去」⇒
> orz 两笔提交并推 `cli`（`abba6886`／`c5245558`）＋双平台打包（`rel-081-stage`，解包回读 6/6 MATCH）
> ＋**GitHub Release `v0.7.0`**＋README 发布面对齐；S4 真机与预检仍挂。
> 明细见 [`081 提交推送与发行档`](081_SUBMIT_PUSH_AND_RELEASE_2026-09-26.md)。

## 0. 结论速览

| 项 | 读数 |
|---|---|
| 版本 bump | 两文件两行：0.6.17 → **0.7.0**（`cargo metadata --locked` exit 0）；树面 bump 链 15→16→17→**0.7.0** 四级同停树面 |
| Windows 三件套 | 增量重建 `Finished release` **5m25s** exit 0；`orz.exe` +102,912 B；**`--build-info` 读数 `version=0.7.0`** |
| Linux musl 三件套 | Docker `rust:1.97-slim` **exit 0**（直写换装位）；`orz` 115,167,400 B（**+433,024**）；代码警告 **7 条**与基线同数（非 Windows 分支 unused 同族；另 2 条工具链噪音非代码） |
| 字面量核证（Win） | **保持面零回退**（0bt①②③／⑮ 全数原样）＋ **c 轮新面全部首进体**（§5）；边界保留字逐格不动 |
| ELF 静态核验 | 三件均 static-pie `ET_DYN`＋x86-64＋**`PT_INTERP=0`**（BuildID `507a1db3`/`eabe36a2`/`322a6292`） |
| 双向加载冒烟 | bookworm-slim／alpine 3.20 双侧三件 **6/6 exit 1**（orz tui io error／signer manifest 缺失／provision usage——与 077 同形态） |
| 换装与 ACAF | Win 三件换装 **MATCH 3/3**（`.0.6.17-bak` 链）＋ACAF 重 provision exit 0（keystore 逐位未动、manifest↔signer 逐位一致）；Linux **直写换装位**（`.0.6.15-bak` 三件链换装前已留） |
| Web 探针 | 全过：守卫 401·401·200／归档 **27 件**（+1＝c 轮 `RUN-CLI-6ab6dc02`）／**零残留进程** |
| 发行状态 | **0.7.0＝未发行**（发行面仍停 `v0.6.13`）；提交推送沿未提交口径待令 |

## 1. 前置基线（换装前实取）

- Windows 在役 0.6.17：`orz.exe` 56,726,016 B `ddb6e956…`／signer `fa24e9f8` 前身的 `9e2db123…`／provision `989e341e…`；备份链 `.0.6.17-bak`。
- Linux 在役 **0.6.15**（078/079 未补）：`orz` 114,734,376 B `42de55f6…`；备份链 `.0.6.15-bak` 三件（换装前实复制）。

## 2. 磁盘与起跑

- 起跑时 D: 余 6.1 GiB（c 轮狗粮测试产物回弹 `target/debug` 22.9 GiB）⇒ `cargo clean --profile dev` 清 **30,305 文件／22.9 GiB**（不挪盘），清后余 28 GiB。
- Docker Desktop 未运行 ⇒ 主会话拉起（引擎 **29.6.2**；`rust:1.97-slim` 镜像在位）。

## 3. Windows 三件套（宿主 release，增量）

`Finished release profile [optimized]` **5m25s**，exit 0；级联面＝c 轮改动（orz-tools web_search 簇／orz-host local_browser+permission+retention／orz-workspace permission／orz-web assets）→ orz-bin 链接。

| 文件 | 尺寸 (B) | 与 0.6.17 差异 | SHA256（换装位＝构建产物，MATCH=True） |
|---|---:|---:|---|
| `orz.exe` | 56,828,928 | **+102,912 B** | `3558df7943e8166a805947617e26ff94409fc0c7807d957ecf41159f859444f8` |
| `orz-signer.exe` | 6,740,480 | −2,048 B | `fa24e9f8cbd94c2c1bb3eab93e8a21da7fcc6e4e82a8a378df23a2103b02f81c` |
| `orz-acaf-provision.exe` | 6,640,128 | −2,048 B | `069bba61c0b9f9715906e3063ce4c3380fa43e72bb8712fcbe4919dcc8e7c088` |

> signer／provision 尺寸 −2 KB 属 c 轮 permission 面改动波及的重编译非确定性（无源码改动的随动面），与「同码不同哈希」家族同记。

## 4. Linux musl 三件套（Docker，直写换装位）

- 挂载契约 ORZ-BUILD-MOUNT-001（`-v D:/CLI:/orz`＋cargo config＋`/target` 暖缓存＋`/out` 直写）；**exit 0**。
- **本批摩擦（已解）**：首次起跑 exit 125——Git Bash MSYS 路径转换把 `-w /orz/orz` 改写为 `B:/Git/orz/orz`（宿主侧 Shell 环境，非载体/配方问题）；`MSYS_NO_PATHCONV=1` 重起即过。同族启示：077 起跑环境未触发，主会话 Git Bash 环境需带此前缀（登记为启动配方注记）。

| 文件 | 尺寸 (B) | 与 0.6.15 差异 | SHA256 |
|---|---:|---:|---|
| `orz` | 115,167,400 | **+433,024 B** | `dd831719ae201c5e81af2e0d2db2a83142ad87905093dbe47591af6388c80a85` |
| `orz-signer` | 1,401,744 | −48 B | `1d88ec37b63252450dd9dca3d09bef591483367a2420785461273722218c3760` |
| `orz-acaf-provision` | 1,220,496 | −48 B | `0a1efffcb0c863293a94d43939601fe081ef72570258c0019c4a5cc3eba61f45` |

- ELF 静态核验（alpine 3.20 ＋ binutils/file）：三件 `ELF 64-bit LSB pie executable, x86-64, static-pie linked`＋**`PT_INTERP=0`**。
- 双向加载冒烟：bookworm-slim／alpine 3.20 **6/6 exit 1**（orz `tui io error: No such device or address`／signer manifest 缺失／provision usage）。
- 警告面：代码警告 **7 条**与基线同数（`WindowsDpapiInstallationKeyStore`／`Mutex` unused import、`cmd`／`path`／`pid`×3 unused variable——非 Windows 分支 unused 同族；另 2 条为 rustup 工具链与 ripgrep 安装噪音，非代码）。探针侧注：alpine `file`/`readelf` 需 `apk add binutils file`（077 未记，本批补登）。

## 5. 字面量核证（Windows 二进制；对照列＝换装前 0.6.17）

| 字节模式 | 0.6.17 | 0.7.0 | 判定 |
|---|---:|---:|---|
| `orz-build-info: version=` | 1 | 1 | 保持面 ✅ |
| `continue with line=` | 2 | 2 | 保持面 ✅ |
| `rewrite the command instead of retrying it` | 3 | 3 | 保持面 ✅ |
| `No, and tell the AI what to do differently` | 1 | 1 | ⑮ 保持面 ✅ |
| `No, and tell Grok what to do differently` | 0 | 0 | 零回退 ✅ |
| `ORZ_RETRIEVAL_ENGINES` | 2 | 2 | 保持面 ✅ |
| `ORZ_RETRIEVAL_FINGERPRINT` | 0 | **1** | ⑩ 指纹门进体 ✅ |
| `api.github.com/search/repositories` | 0 | **1** | ⑫ 垂直源进体 ✅ |
| `local_segmented_fallback` | 0 | **2** | ⑧ 降级注记键进体 ✅ |
| `browser-downloads-` | 0 | **2** | ⑪ 下载 retention 前缀进体 ✅ |
| `impersonate=` | 0 | **1** | ⑩ 内嵌 sidecar 脚本进体 ✅ |
| `360search` | 0 | **3** | ⑨ 引擎注册表进体 ✅ |
| `GrokBuild:` ／ `GROK_HOME` | 26／7 | 26／7 | 边界保留 ✅ |

## 6. 换装、ACAF 与装配复核

- Windows：构建产物三件 → `D:\tb-eval\orz-windows\`，**MATCH 3/3**；ACAF 重 provision **exit 0**（keystore 两件哈希前后逐位未动；回显 `binary_sha256=fa24e9f8…` ↔ 换装位 signer 逐位一致；旧 manifest 留 `.bak-20260926-080`）；冒烟 2/2（exit 1 形态）；**`--build-info` 换装位读数 `version=0.7.0 os=windows arch=x86_64 profile=release`**。
- Linux：构建直写换装位（`.0.6.15-bak` 链换装前已留）；ELF＋冒烟见 §4。
- **载体级 Web 探针（`.tmp-b080-win-web-probe.ps1`，端口 21560）**：全过——`/` 200／无令牌 **401**／伪 Host **401**／带令牌 **200**；`boot_keys = cwd,agent,trusted_workspaces`；归档 **27 件**（079 读数 26 → +1＝c 轮会话归档）；`leftover_orz_procs=0`。启动文案 GBK 呈现为已知观察面（探针捕获路径）。

## 7. 未做项

> **后批补记（081 批，2026-09-26）**：下列第 1、2 项**已结清**——用户令「请进行提交与推送吧，
> 0.7.0也发布上去」⇒ orz 两笔提交推送（`abba6886`／`c5245558`）＋双平台打包
> （`D:\tb-eval\rel-081-stage\`；zip `d099d9f3…`／tar.gz `78cec631…`；解包回读 6/6 MATCH）
> ＋**GitHub Release `v0.7.0`**＋README 发布面对齐；第 3、4 项（S4 真机／预检）仍挂。
> 明细见 [`081 提交推送与发行档`](081_SUBMIT_PUSH_AND_RELEASE_2026-09-26.md)。

1. **本地打包未做**：本批只重建＋换装（打包/SHA256SUMS 仪式随发行令循 077 §8 配方）。
2. **未提交／未推送**：沿未提交口径（0bs a/b/c 三轮落码＋四级 bump 同停树面）；源冻结＝worktree 态（orz 子模块 HEAD 未动 ⇒ `orz_source_manifest.sha256` 按设计不动）。
3. **S4 真机复验未跑**：0bs 移交面与 0bv S4（DDG 代理腿／百度 link 壳展开／指纹 off 对照／arXiv 回落腿 UA）各行其账。
4. **预检未做**（默认口径）。

## 8. 本批摩擦（仅记录，不另立项）

1. **Docker `-w` 参数被宿主 Git Bash MSYS 路径转换改写**（exit 125）⇒ `MSYS_NO_PATHCONV=1` 重起即过；登记为 Linux 构建启动配方注记（077 起 pre-existing 环境差异，非回归）。
2. **D: 余量反复回弹**（c 轮测试产物 22.9 GiB）⇒ 常规 `cargo clean --profile dev` 腾挪；狗粮轮后重建前宜先盘点（连续第三批同族操作，惯例化）。
3. **冒烟脚本 `$f` 未转义**致 126 假象（宿主 shell 先行展开）——修正后 6/6 exit 1；探针侧方法学注记。
4. **alpine `file`/`readelf` 需现装**（`apk add binutils file`）——077 未记，本批补登。

## 9. 记账面

- orz 子模块 HEAD 未动 ⇒ 源清单按设计不变。
- 父仓（树面，未提交）：本档 ＋ TODO/BACKLOG 0bv「S3 载体达成（0.7.0 双平台含 c 轮面）」标注 ＋ 0bs 载体注记延展 ＋ 第二卷 §1.36 ＋ 索引头行 → **v4.54**（v4.53 头行滚入存档卷，103 → **104 行**）。
- **计数不变**（**未闭合 57**）：本批为重建与换装。

## 10. 关联与关键词

[`0BS_PROGRESS_2026-09-26c`](0BS_PROGRESS_2026-09-26c.md) ／ [`真网在线验证`](0BS_RETRIEVAL_ONLINE_PROBE_2026-09-26.md) ／
[`079 重建档`](079_CARRIER_REBUILD_WINDOWS_2026-09-26.md) ／ [`078 重建档`](078_CARRIER_REBUILD_WINDOWS_2026-09-26.md) ／
[`077 重建档`](077_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-25.md) ／ [`ADR-0010`](../../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)

关键词：0.7.0、双平台载体重建、Linux 腿三批补跑、⑨⑩/⑧/⑫/⑬/⑪ 进载体、字面量核证、
static-pie PT_INTERP=0、双向加载冒烟、MSYS 路径转换、ACAF 重 provision、载体级 Web 探针、
`.0.6.17-bak`／`.0.6.15-bak` 备份链、**未发行载体**。
