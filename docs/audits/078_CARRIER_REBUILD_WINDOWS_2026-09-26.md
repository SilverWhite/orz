# 0.6.16 Windows 载体重建与换装（2026-09-26）

> **本批＝用户令「既然模型自决转下轮，那就将0bs下半部分先留着，先进一轮重建，把新的修补纳入二进制先」**——
> 0bs ⑮⑯／0bt④ 树面落码（[`0BS 执行报告`](0BS_EXECUTION_AND_FRICTION_2026-09-26.md)，未提交）随本批进 Windows 在役载体。
> **版本 bump**（0.6.15 → 0.6.16，树面两文件两行）② Windows 三件套增量重建＋换装＋字面量核证＋ACAF 与装配复核＋载体级 Web 探针。
> **未提交／未推送（沿 0bs 轮用户令继续停树面）／未发行；Linux musl 腿待补（§7）。**

## 0. 结论速览

| 项 | 读数 |
|---|---|
| 版本 bump | `crates/orz-bin/Cargo.toml` ＋ `Cargo.lock` 两文件两行：0.6.15 → **0.6.16**（`cargo metadata --locked` exit 0）；**树面未提交** |
| Windows 三件套 | **增量重建**（非 clean；§4 口径偏离与缘由）`Finished release` **3m41s** exit 0；构建日志警告/错误面＝未见（§4） |
| 字面量核证 | ⑮ 进出确认（旧文案 1 → 0、新文案 0 → 1）；边界保留字逐格不动；**⑯-① transport 两句＝新旧二进制均 0（链接期剥离，§5）** |
| 换装与 ACAF | 三件换装（`.0.6.15-bak` 链）逐件 **MATCH=True 3/3**；ACAF 重 provision exit 0（keystore 前后逐位未动、manifest `binary_sha256` ↔ 换装位 signer 逐位一致）＋冒烟 2/2（exit 1 形态正确）＋载体级 Web 探针全过（token 64 位／守卫 401·401·200／归档 25 件／**零残留进程**） |
| 提交推送 | **无**（用户令未解除；orz 子模块 HEAD 不动 ⇒ `orz_source_manifest.sha256` 按设计不动，§9） |
| 发行状态 | **0.6.16＝未发行**（在役换装照做；发行面仍停在 `v0.6.13`）；本地打包未做（§7） |

## 1. 前置基线（0.6.15 在役件，换装前实取）

| 文件 | 尺寸 (B) | SHA256 |
|---|---:|---|
| `orz.exe` | 56,475,648 | `4bcae4ee641483b56c67b22dcff5af674f68b013d85654dfe9dc50275b8777c0` |
| `orz-signer.exe` | 6,742,528 | `a831bf2f4a80140b59f409ea3a8af2f365cdf915be62fe8252cc852b7388c39a` |
| `orz-acaf-provision.exe` | 6,642,176 | `c90a908be0714a17d7222ba17d81e5cdc5b3c688dd94855244d302f1b7e8b912` |

换装前逐件复制为备份链 **`.0.6.15-bak`**（Windows 三件；077 链 `.0.6.14-bak` 原样保留）。

## 2. 版本 bump

- `crates/orz-bin/Cargo.toml` `version` 与 `Cargo.lock` `orz-bin` 条目两处：**0.6.15 → 0.6.16**（两文件两行）。
- `cargo metadata --locked` **exit 0**（lock 一致性核验过）。
- **未提交**：与 0bs ⑮⑯／0bt④ 树面落码同停树面；子模块 HEAD 未动 ⇒ 源清单按设计不动（§9）。

## 3. 预检

按用户 2026-09-22 口径**默认不预检**；本批为 0bs 增量进件重建，未做双平台预检。

## 4. Windows 三件套（宿主 release，**增量**）

- 构建入口＝`scripts/build_orz.ps1 -Release`（**无 `-Clean`**）。
- **口径偏离与缘由（登记）**：076/077 惯例为 `cargo clean` 后 clean 全量（077 实测 clean 23.9 GiB／重建 25m26s）。本批起跑时 **D: 余量仅 5.2 GiB**（0bs 轮摩擦 f1 延续面），不支持 24 GiB 级 clean 重建；处置＝`cargo clean --profile dev` 清 debug 测试产物 **26,731 文件／26.5 GiB**（release 产物 5.6 GiB 原样保留作增量基座；**不挪盘**——用户令「不要挪盘，就在D盘即可」），清后 D: 余 30 GiB。
- 构建：`Finished release profile [optimized]` **3m41s**，**exit 0**；日志（`.tmp-b078-windows-build.log`）未见 warning/error 行。增量级联面＝本轮改动 crate（orz-assurance／orz-sampling-types／orz-agent／orz-mcp／orz-loop／orz-workspace）→ orz-host／orz-web／orz-bin 链接。
- 首次起跑摩擦：后台起跑时 shell cwd 漂移致 `-File scripts/build_orz.ps1` 相对路径未中（exit 127、未构建）⇒ 改绝对路径重起成功（§8-①）。

| 文件 | 尺寸 (B) | 与 0.6.15 差异 | SHA256（换装位＝构建产物，MATCH=True） |
|---|---:|---:|---|
| `orz.exe` | 56,555,520 | **+79,872 B** | `c3b0c72ca6ce3b523071ddf1d7bb8a7d9daf948d1b915a800407970c7049c868` |
| `orz-signer.exe` | 6,742,528 | ±0 | `1689e3886fde7482899b800eee4be6538c4cc41bf5fe8c9b7cc6578489c50d86` |
| `orz-acaf-provision.exe` | 6,642,176 | ±0 | `bad15a9db45441bade77ecb130e87bf0abc8510d230b141f3f25b7bb8a855ad6` |

> `orz.exe` +79,872 B 与 ⑮⑯／0bt④ 增量代码量级相称（0bs 执行报告 §2：观测面五处落点＋钉子＋prompter/文案三处）；`signer`／`provision` 无源码改动、尺寸不变、哈希变动属重编译非确定性（068–077「同码不同哈希」同族）。

## 5. 字面量核证（Windows 二进制，字节级出现次数；对照列＝换装前 0.6.15 在役件）

| 字节模式 | 0.6.15（役） | 0.6.16（新） | 判定 |
|---|---:|---:|---|
| `No, and tell Grok what to do differently` | 1 | **0** | ⑮ 旧文案出面 ✅ |
| `No, and tell the AI what to do differently` | 0 | **1** | ⑮ 新文案进面 ✅ |
| `Grok is temporarily unavailable` | 0 | 0 | **均不在字节面**（见下） |
| `Connection to Grok timed out` | 0 | 0 | **均不在字节面**（见下） |
| `GrokBuild:`（工具 id 前缀，边界保留） | 26 | 26 | 逐格不动 ✅ |
| `GROK_HOME`（env 兼容面，边界保留） | 7 | 7 | 逐格不动 ✅ |
| `grok-4`（模型 id，配置面注入、不编译进体） | 0 | 0 | 与 077 §6 同读 ✅ |

- **⑯-① transport 两句字节面为零（新旧同态）——观察项登记**：唯一下游链＝`user_facing_api_error_message`（`error.rs:414`）→ `status_user_message`（`:354`），该链在 orz.exe 链接图**当前无引用** ⇒ MSVC `/OPT:REF` 链接期剥离（函数符号与字面同净）。与 077 §6「载体版本串不落字节面」同族：**字节面不可核证 ≠ 改动未生效**——⑯-① 证据面＝源码 diff ＋ 单测（`status_user_messages_are_de_groked_and_keep_http_code`，sampling 285/0 过）；该文案面如后续被接线（引用进入链接图），字面将随编译自然落体。
- MCP OAuth 客户端名（`"Grok"` → `"AI"`）：常量过短、字节计数无判别力，不设字节级判据（证据面＝源码 diff ＋ RFC 7591 对端可见面注释记档）。

## 6. 换装、ACAF 与装配复核

- 换装：构建产物三件 → `D:\tb-eval\orz-windows\`，逐件 SHA256 复核 **MATCH=True 3/3**；换装前备份链 `.0.6.15-bak` 三件齐备。
- ACAF 重 provision（`orz-acaf-provision.exe acaf\keystore acaf\signer-manifest.json`，**exit 0**）：
  - keystore 前后逐位未动（`installation-key.dpapi`／`installation-key.json` 哈希前后一致）；
  - provision 回显 `binary_sha256=1689e388…` ↔ 换装位 `orz-signer.exe` **逐位一致**（manifest 同步更新；旧 manifest 留 `signer-manifest.json.bak-20260926-078`）。
- 冒烟（换装位）：provision 无参 **exit 1**（usage）；signer 缺 manifest **exit 1**（fatal）——与 077 §7 同形态。
- **载体级 Web 探针（脚本 `.tmp-b078-win-web-probe.ps1`，端口 21558）**：`orz.exe web --addr 127.0.0.1:21558` 拉起换装位二进制 → 启动打印 64 位令牌 URL；`/`（无 token）200／`/api/boot`（无 token）**401**／伪造 Host **401**／带 token **200**；`boot_keys = cwd,agent,trusted_workspaces`；归档面 **25 件**（本机真实归档数，077 首读 21 件 → +4，含今日 0bs 执行轮 `RUN-CLI-6ab6b76d`）；探针后 **`leftover_orz_procs=0`**。
- 探针捕获日志启动文案呈 GBK 乱码：077 §7.1 同款已知观察面（`Start-Process` 重定向捕获链，0bm F6 族残留观察；消费链修复面在 launcher 侧已落、本捕获路径不在其范围内）。

## 7. 未做项

1. **Linux musl 三件套未重建**：Docker daemon 未运行（`dockerDesktopLinuxEngine` 管道不存在）。在役换装面＝Windows；Linux 腿（`D:\tb-eval\orz-linux` 三件仍为 0.6.15）**待补**——随下一轮载体批或用户令补跑（`build_orz_aliyun_trixie.sh` 配方与 077 §5 同）。
2. **本地打包未做**：本批不发行，无打包面（077 §8 先例为本批可循配方）。
3. **提交推送未做**：沿 0bs 轮用户令「不必进行提交/推送/重建」中的前两项继续停树面（重建为本批新令解除）；提交/推送待用户令。
4. **S4 真机复验未跑**：0bs 执行报告 §6 移交面（含 S4 复验五点）与本批载体行为复验均留下一轮真机。
5. **0bs 下半部分未动**：0bt①②③、⑧-⑫、⑬⑭ 等按用户令「0bs下半部分先留着」原样留树面/账面。

## 8. 本批摩擦（仅记录，不另立项）

1. **后台起跑 cwd 漂移**：shell 工作目录残留上一切换点（`D:\tb-eval\orz-windows`）致 `-File scripts/build_orz.ps1` 相对路径未中（exit 127）；改绝对路径重起即过。与 0bm「M-1 cwd 断言」同族（该断言护狗粮启动器；构建起跑无同款断言）。
2. **D: 余量临界**：起跑时 5.2 GiB（99% 用量），为 0bs 轮 f1（os error 112）同族延续面；本轮以清 debug 产物 26.5 GiB 缓解（清后 29–30 GiB），**未挪盘**。后续 clean 全量重建前须先腾挪或清理。
3. **探针捕获日志 GBK 呈现**：见 §6 末条；已知观察面，不重复立项。

## 9. 记账面

- orz 子模块 HEAD：**未动**（沿未提交口径）⇒ `orz_source_manifest.sha256` 按「tracked submodule content at HEAD」设计**不变为正确行为**（清单哈希取子模块 HEAD blob、非工作树；树面改动进清单须随下次 orz 提交批）。
- 父仓（全部树面，未提交）：本档 ＋ TODO／BACKLOG 的 0bs「0.6.16 已进 Windows 载体」标注 ＋ 第二卷 §1.33 本批流水 ＋ 索引头行 → **v4.51**（v4.50 头行滚入 [`存档/index/CLI_PROJECT_INDEX_FULL_2026-09-24.md`](../../存档/index/CLI_PROJECT_INDEX_FULL_2026-09-24.md)，100 → 101 行）。
- **计数不变**（**未闭合 56**）：本批为重建与换装，不新增／不闭合开放项。

## 10. 关联与关键词

[`0BS 执行报告`](0BS_EXECUTION_AND_FRICTION_2026-09-26.md) ／
[`077 重建档`](077_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-25.md) ／
[`0BS 摩擦修复与承接勘定（前轮）`](0BS_FRICTION_REMEDIATION_AND_CARRYOVER_2026-09-25.md) ／
[`ADR-0010`](../../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)

关键词：0.6.16、Windows 载体重建、增量构建、debug 产物腾挪、⑮⑯ 进载体、0bt④ 观测面进载体、
字面量核证、transport 链接期剥离、ACAF 重 provision、载体级 Web 探针、`.0.6.15-bak` 备份链、
**未发行载体**、Linux 腿待补。
