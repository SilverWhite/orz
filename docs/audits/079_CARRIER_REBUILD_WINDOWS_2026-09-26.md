# 0.6.17 Windows 载体重建与换装（2026-09-26，同日第二重建）

> **本批＝用户令「再进一轮重建吧，依旧只重建windows」**——0bs 后半轮树面落码（0bt①②③／⑪ 输入拟真 v1／f4，[`0BS 后半部分处理报告`](0BS_PROGRESS_2026-09-26b.md)，未提交）随本批进 Windows 在役载体。
> **版本 bump**（0.6.16 → 0.6.17，树面两文件两行）② Windows 三件套增量重建＋换装＋字面量核证＋ACAF 与装配复核＋载体级 Web 探针＋`--build-info` 首读。
> **未提交／未推送／未发行；Linux musl 腿待补（§7，沿「依旧只重建 windows」令）。**

## 0. 结论速览

| 项 | 读数 |
|---|---|
| 版本 bump | `crates/orz-bin/Cargo.toml` ＋ `Cargo.lock` 两文件两行：0.6.16 → **0.6.17**（`cargo metadata --locked` exit 0）；**树面未提交**（0.6.15→16→17 三级 bump 同停树面） |
| Windows 三件套 | **增量重建**（D: 余 20 GiB，无需腾挪）`Finished release` **4m12s** exit 0 |
| 字面量核证 | 0bt①②③／⑪ 新面**全部进体**（§5；滚动 CDP 方法名为字串池尾合并形态，非缺面）；0.6.16 面（⑮）保持、边界保留字逐格不动 |
| 换装与 ACAF | 三件换装（`.0.6.16-bak` 链）逐件 **MATCH=True 3/3**；ACAF 重 provision exit 0（keystore 前后逐位未动、manifest `binary_sha256` ↔ 换装位 signer 逐位一致）＋冒烟 2/2；**`orz --build-info` 在役首读＝`version=0.6.17 os=windows arch=x86_64 profile=release`**（0bt② 面世即验证，`carrier=unknown` 时代终结） |
| Web 探针 | 全过：`/` 200／无令牌 **401**／伪 Host **401**／带令牌 **200**；归档 **26 件**（078 首读 25 → +1＝后半轮 `RUN-CLI-6ab6ce44`）；**零残留进程** |
| 提交推送 | **无**（沿未提交口径）；**发行状态＝0.6.17 未发行**（发行面仍停 `v0.6.13`） |

## 1. 前置基线（0.6.16 在役件，换装前实取）

| 文件 | 尺寸 (B) | SHA256 |
|---|---:|---|
| `orz.exe` | 56,555,520 | `c3b0c72ca6ce3b523071ddf1d7bb8a7d9daf948d1b915a800407970c7049c868` |
| `orz-signer.exe` | 6,742,528 | `1689e3886fde7482899b800eee4be6538c4cc41bf5fe8c9b7cc6578489c50d86` |
| `orz-acaf-provision.exe` | 6,642,176 | `bad15a9db45441bade77ecb130e87bf0abc8510d230b141f3f25b7bb8a855ad6` |

换装前逐件复制为备份链 **`.0.6.16-bak`**（077 链 `.0.6.14-bak`／078 链 `.0.6.15-bak` 原样保留）。

## 2. 版本 bump

- `crates/orz-bin/Cargo.toml` `version` 与 `Cargo.lock` `orz-bin` 条目：**0.6.16 → 0.6.17**（两文件两行）；`cargo metadata --locked` **exit 0**。
- 树面 bump 链：0.6.15 → 0.6.16（078）→ 0.6.17（本批）**均未提交**，随 0bs 两轮落码同停树面。

## 3. 预检

默认口径**不预检**（用户 2026-09-22 口径延续）。

## 4. Windows 三件套（宿主 release，增量）

- 构建入口＝`scripts/build_orz.ps1 -Release`（绝对路径起跑，078 §8-① cwd 漂移摩擦不复现）。
- 磁盘：起跑时 D: 余 20 GiB（078 清 debug 后后半轮回弹 `target/debug` 9 GiB 属正常测试产物）；增量构建新增产物 ~1 GiB 级，**无需清理、不挪盘**。
- 构建：`Finished release profile [optimized]` **4m12s**，**exit 0**；级联面＝后半轮改动 crate（orz-tools〔read_file/bash〕／orz-host〔local_browser/input_sim〕／orz-bin〔--build-info〕等）→ orz-bin 链接。

| 文件 | 尺寸 (B) | 与 0.6.16 差异 | SHA256（换装位＝构建产物，MATCH=True） |
|---|---:|---:|---|
| `orz.exe` | 56,726,016 | **+170,496 B** | `ddb6e95603c80caa57a78b35752d97f70b7c7e1f60c3fbf441247d8a0a151479` |
| `orz-signer.exe` | 6,742,528 | ±0 | `9e2db123472455117f6976aeba3e0825a85477b3b99394ff70da2e4bdb06be01` |
| `orz-acaf-provision.exe` | 6,642,176 | ±0 | `989e341ef8868f1d3e76d4528974d23f7c9f6960ff626378697d06644c746b0e` |

> `orz.exe` +170,496 B 与 0bt①（行窗面＋73 处调用点适配）②（--build-info）③（三模板改写）＋⑪（`input_sim.rs` 全新模块＋CDP 白名单放开＋类型留档）代码量级相称；signer／provision 无源码改动，哈希漂移属重编译非确定性（同族先例 068–078）。

## 5. 字面量核证（Windows 二进制，字节级；对照列＝换装前 0.6.16 在役件）

| 字节模式 | 0.6.16（役） | 0.6.17（新） | 判定 |
|---|---:|---:|---|
| `orz-build-info: version=` | 0 | **1** | 0bt② 旁路进体 ✅ |
| `continue with line=` | 0 | **2** | 0bt① 续读指针进体（描述＋标记格式两处）✅ |
| `chars total; showing ` | 0 | **1** | 0bt① 行窗标记进体 ✅ |
| `rewrite the command instead of retrying it` | 0 | **3** | 0bt③ 三模板纪律进体 ✅ |
| `Input.dispatchKeyEvent` | 0 | **1** | ⑪ CDP 白名单放开进体 ✅ |
| `Input.insertText` ／ `Input.dispatchMouseEvent` | 0 | 各 **1** | 同上 ✅ |
| `Input.synthesizeScrollGesture`（全串） | 0 | 0 | **字串池尾合并形态，非缺面**（见下） |
| `No, and tell the AI what to do differently`（⑮ 保持面） | 1 | 1 | 零回退 ✅ |
| `GrokBuild:` ／ `GROK_HOME`（边界保留） | 26／7 | 26／7 | 逐格不动 ✅ |

- **`Input.synthesizeScrollGesture` 观察项**：全串不连续，但 `Input.synthesize`（1）与 `ScrollGesture`（1）两段**邻接命中**——MSVC 链接器字串池尾合并（/OPT:ICF）把重叠后缀串共享存储所致（池内可见 `…Input.dispatchKe|izeScrollGesture|Input.synthesize|useEvent…` 邻接排布）；源码三处（白名单＋执行两处）在体。**字面被池化剖开 ≠ 缺面**，与 077「版本串不落字节面」同属「字节面判据的形态边界」记录项。
- ⑪ 输入拟真参数（200ms±40% 等）为数值常量，不设字节级判据（证据面＝local_browser 93/0/4 ignored）。

## 6. 换装、ACAF 与装配复核

- 换装：构建产物三件 → `D:\tb-eval\orz-windows\`，逐件 **MATCH=True 3/3**；备份链 `.0.6.16-bak` 三件齐备。
- ACAF 重 provision（**exit 0**）：keystore 前后逐位未动（两件哈希一致）；回显 `binary_sha256=9e2db123…` ↔ 换装位 signer **逐位一致**；旧 manifest 留 `signer-manifest.json.bak-20260926-079`。
- 冒烟：provision 无参 **exit 1**（usage）；signer 缺 manifest **exit 1**（fatal）。
- **`--build-info` 在役首读**：`orz-build-info: version=0.6.17 os=windows arch=x86_64 profile=release`——0bt② 旁路在换装位即验即用；`dogfood_launch.ps1` 版本段自此显示真实载体版本（`carrier=unknown` 摩擦 m15/0bt② 收口面）。
- **载体级 Web 探针（`.tmp-b079-win-web-probe.ps1`，端口 21559）**：全过（守卫 401·401·200、boot 键 `cwd,agent,trusted_workspaces`、归档 **26 件**、`leftover_orz_procs=0`）。启动文案 GBK 呈现为 077 §7.1 同款已知观察面（探针捕获路径）。

## 7. 未做项

1. **Linux musl 三件套未重建**：用户令「依旧只重建windows」；`D:\tb-eval\orz-linux` 三件仍为 **0.6.15**（连续第三批待补：078/079 两批增量均未进 Linux 载体），补跑随用户令。
2. **本地打包未做／未发行**：发行面仍停在 `v0.6.13`。
3. **提交推送未做**：沿未提交口径（0bs 两轮落码＋三级 bump 同停树面）。
4. **S4 真机复验未跑**：0bs 两轮报告的移交面（含 S4 复验五点、⑪ 唯二门禁接线、⑧⑨⑩⑫⑬、⑬-S2）各行其账。
5. **0bs 残余未动**：按用户令「模型自己裁决的…不做干预」，分轮形态保持。

## 8. 本批摩擦（仅记录，不另立项）

1. 无新增。078 §8 三条（cwd 漂移／D: 余量临界／探针 GBK 呈现）中：cwd 漂移已以绝对路径起跑规避；余量本轮余 20 GiB 未触临界；GBK 呈现复现（同款已知观察面）。

## 9. 记账面

- orz 子模块 HEAD：**未动** ⇒ `orz_source_manifest.sha256` 按设计不变（清单哈希取子模块 HEAD blob；078 §9 同口径）。
- 父仓（全部树面，未提交）：本档 ＋ TODO／BACKLOG 0bs 载体注记延展（0.6.17）＋ 第二卷 §1.34 ＋ 索引头行 → **v4.52**（v4.51 头行滚入存档卷，101 → **102 行**）。
- **计数不变**（**未闭合 56**）：本批为重建与换装，不新增／不闭合开放项。

## 10. 关联与关键词

[`0BS 后半部分处理报告`](0BS_PROGRESS_2026-09-26b.md) ／
[`0BS 执行报告（前轮）`](0BS_EXECUTION_AND_FRICTION_2026-09-26.md) ／
[`078 重建档`](078_CARRIER_REBUILD_WINDOWS_2026-09-26.md) ／
[`077 重建档`](077_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-25.md) ／
[`ADR-0010`](../../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)

关键词：0.6.17、Windows 载体重建、同日第二重建、0bt①②③ 进载体、⑪ 输入拟真 v1 进载体、
`--build-info` 在役首读、字面量核证、字串池尾合并、CDP 白名单放开、ACAF 重 provision、
载体级 Web 探针、`.0.6.16-bak` 备份链、**未发行载体**、Linux 腿三批待补。
