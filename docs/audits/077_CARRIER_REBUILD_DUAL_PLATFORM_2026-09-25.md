# 0.6.15 双平台载体重建与换装（2026-09-25）

> 用户令：「请进行提交推送并重建吧」。本批三段：① **提交推送**（orz `1ec7b729`＝0bs 七件
> 落码 ＋ 父仓 `6c129046`＝0bs 报告／0bt 立项／启动器编码钉）② **版本 bump**
> （0.6.14 → 0.6.15，orz `a8430054`）③ **双平台重建＋换装＋字面量核证＋ACAF 与装配
> 复核＋载体级 Web 探针＋本地打包**。
> **源冻结线**：orz **`a8430054`**（`feat/fusion-architecture`）＝0bs S2 增量 `1ec7b729`
> ＋ bump 两文件两行。**上一载体**：双平台 0.6.14（**中间过渡版、未发行**，
> 源冻结 orz `e22c0dba`）。本批**不发行**（用户令口径延续：「0.6.14 暂时不发，这个属于
> 中间版本」；**发行面仍停在已发布 `v0.6.13`**）——在役换装照做，先例＝0.6.7–0.6.11。
> 本批**免预检**（默认口径），但做了 Linux 侧 **apt 实包预检**（`APT_OK`，未切代理）。

## 0. 结论速览

| 项 | 结果 |
|---|---|
| 提交推送 | orz `e22c0dba..1ec7b729`（0bs 七件 8 文件 `+415/−87`）＋ `1ec7b729..a8430054`（bump）；父仓 `897b0071..6c129046`（10 文件 `+366/−20`） |
| 版本 bump | `crates/orz-bin/Cargo.toml` ＋ `Cargo.lock` 两文件两行：0.6.14 → **0.6.15**（`cargo metadata --locked` exit 0） |
| 0bs 进件 | 结束自述常驻尾行／分块表单源指向行／未落地回执＋审计键／状态符号窗口**首次进入在役载体** |
| Windows 三件套 | `cargo clean`（41,451 文件／23.9 GiB）后 clean 全量 **25m26s** exit 0；**警告 0／错误 0**；换装逐件 MATCH=True |
| Linux musl 三件套 | Docker `rust:1.97-slim`；实包预检 `APT_OK`；容器内 `Finished release` **9m16s**（墙钟 10.6 min）exit 0；三件 ET_DYN／x86-64／PT_INTERP=0；bookworm·alpine 双向冒烟 **6/6 exit 1** |
| 字面量核证 | **新面 4 项全部 0 → ≥1**（双侧）；**保持面 50 项零回退**（0.6.14 新面 7／0.6.13 面 8／0.6.12 面 9／0.6.11 面 14／更早面 12）；`failures=0` |
| 换装与 ACAF | 六件换装（`.0.6.14-bak` 链）＋ACAF 重 provision exit 0（keystore 四值逐位未动、manifest↔signer 逐位一致）＋载体级 Web 探针全过（21 件归档、零残留进程） |
| 打包 | 双平台包 ＋ 顶层 `SHA256SUMS` 暂存 `D:\tb-eval\rel-077-stage\`（解包回读 6/6 MATCH、容器内 `sha256sum -c` 全 OK）；**0.6.15＝未发行** |
| 摩擦（仅记录） | ① **PS 5.1 `-File` 读无 BOM 的 UTF-8 `.ps1`** ⇒ 解析失败／中文全乱（属 0bs ② 消费链同族又一实证）；② `orz web` 启动文案经 `powershell -File` 捕获仍 GBK 乱码（0bm／0bs ② 已立案面复现） |

## 1. 前置基线（0.6.14 在役件，换装前实取）

| 平台 | 文件 | 尺寸 (B) | SHA256 |
|---|---|---:|---|
| Windows | `orz.exe` | 56,440,832 | `B79F20DB60618B5F0CD02F919F36FEF272390F5728686D9DA0FD9CB7BBAD1CCB` |
| Windows | `orz-signer.exe` | 6,742,528 | `179A2E4048F57E19489D2DD662F8648036A24B03EF838C5E940014A9705871DD` |
| Windows | `orz-acaf-provision.exe` | 6,642,176 | `332A6EF8C925C19ABF7EA8F5860BFC28612383D03C81935797B7155AC9070CCE` |
| Linux | `orz` | 114,693,448 | `E1E0EEA75E2F4BB76D6315E4352ABFC16573D94C4F1CE99AC9626B7FFEA5C9AD` |
| Linux | `orz-signer` | 1,401,800 | `8DA16E7148D2774D97A4CD9A6782C71B029E63C3EC8CEADDD89BFD812462C63A` |
| Linux | `orz-acaf-provision` | 1,220,528 | `555124F5C80434E455186556B18AD70F02806F6249F9648373DEFA7809CB650C` |

六件与 [`076 档 §4／§5`](076_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-25.md) 记录**逐位一致**
（在役件未被外力改动）。换装前逐件复制为备份链 **`.0.6.14-bak`**（Windows
`D:\tb-eval\orz-windows\`；Linux `D:\tb-eval\orz-linux\`）——六件全部就位。

## 2. 提交推送与版本 bump

- **orz 两笔**：`1ec7b729`＝0bs 七件（8 文件 `+415/−87`：`model_stop.rs` 常驻尾行／
  `model_face.rs` 尾挂＋`BLOCK_TABLE_POINTER_LINE`／`agent_loop.rs` 调用点同步／
  `context_scale.rs` 未落地回执＋审计键／`lsp/manager.rs` 循环等待／`emoji_strip.rs`＋
  `emoji_strip_ranges.rs` 状态符号窗口／`rust-toolchain.toml` 口径注释）；
  `a8430054`＝bump `0.6.14 → 0.6.15`（两文件两行，`cargo metadata --locked` exit 0）。
  两笔均推 `cli` 远端（`feat/fusion-architecture`）。
- **父仓一笔**：`6c129046`（10 文件 `+366/−20`）＝0bs 轮报告（新档）＋0BM 报告 §8
  缓存面补记＋0bt 立项台账（索引 v4.46／TODO／BACKLOG 一卷二卷）＋
  `scripts/dogfood_launch.ps1` 编码链钉（`[Console]::OutputEncoding` ＋ `$OutputEncoding`
  ＝ UTF-8 ＋ 自检断言）＋子模块 pin `1ec7b729` ＋ `orz_source_manifest.sha256`
  重生成 **1,498 条**（差异 8 行＝子模块八文件）。推 `origin main`。
- 提交前门禁 `python scripts/check_repository.py`：唯一红＝「orz submodule working tree is
  dirty」（按 0bs 令未提交、**预期态**）；记账钉子
  `python -m unittest -q assurance.tests.test_ledger_consistency_nails`＝**15 tests OK**。
- **本批第二次生成 manifest**（随 `a8430054`）差异恰两行＝`Cargo.lock` 与
  `crates/orz-bin/Cargo.toml`。

## 3. 预检：本批按默认口径免除

- 用户 2026-09-22 口径：**默认不预检**。本批为 0bs 增量进载体＋打包，**不做**双平台预检。
- Linux 侧仍做**apt 实包预检**（`apt-get update -qq && apt-get install -y -qq musl-tools`
  → `APT_OK`，musl-tools 1.2.5-3.1~deb13u1）⇒ 按 [`Docker 代理配方`](../../scripts/DOCKER_PROXY_RECIPE.md)
  §0 **不切代理、不重启**。

## 4. Windows 三件套（宿主 release，clean 全量）

- 构建入口＝`scripts/build_orz.ps1 -Release -Clean`（auto 档自检读数 **12 核／物理
  15.8 GiB（余 6.5）／commit 25.8 GiB（余 8.4，使用 67.5%）** ⇒ 判 **`-j 2`**，
  并触发 0bh ⑥ 的余量软提示）。
- `cargo clean`：**41,451 文件／23.9 GiB**；构建 **exit 0**，`Finished release` **25m26s**
  （墙钟 25.7 min）；**警告 0 条、错误 0 条**（日志 791 行逐行复核）。

| 文件 | 尺寸 (B) | 与 0.6.14 差异 | SHA256（换装位＝构建产物，MATCH=True） |
|---|---:|---:|---|
| `orz.exe` | 56,475,648 | **+34,816 B** | `4bcae4ee641483b56c67b22dcff5af674f68b013d85654dfe9dc50275b8777c0` |
| `orz-signer.exe` | 6,742,528 | ±0 | `a831bf2f4a80140b59f409ea3a8af2f365cdf915be62fe8252cc852b7388c39a` |
| `orz-acaf-provision.exe` | 6,642,176 | ±0 | `c90a908be0714a17d7222ba17d81e5cdc5b3c688dd94855244d302f1b7e8b912` |

> `orz.exe` +34 KB 与 0bs 增量的代码量级相称（常驻尾行＋指针行常量＋回执函数＋
> 时钟/等待循环＋26 段窗口表）；`signer`／`provision` **无源码改动**，尺寸不变、
> 哈希变动属重编译非确定性——与 068–076「同码不同哈希」同族，非回退。

## 5. Linux musl 三件套（Docker，直写换装位）

- 构建：`rust:1.97-slim` ＋ `bash /orz/scripts/build_orz_aliyun_trixie.sh`
  （ORZ-BUILD-MOUNT-001 契约），挂载 `-v D:/CLI:/orz`（父仓原地）
  ＋`-v D:/tb-eval/cargo-config.toml:/root/.cargo/config.toml`
  ＋`-v D:/tb-eval/orz-target:/target`（暖缓存）＋`-v D:/tb-eval/orz-linux:/out`
  （**直写换装位**，换装前已留 `.0.6.14-bak`）；**exit 0**：容器内 `Finished release`
  **9m16s**、墙钟 **10.6 min**（日志 `.tmp-b077-linux-build.log`）。
- **容器未在源树留痕**：构建后 orz 仓 `git status` 全净。

| 文件 | 尺寸 (B) | 与 0.6.14 差异 | SHA256 |
|---|---:|---:|---|
| `orz` | 114,734,376 | **+40,928 B** | `42de55f69948aa497acce55aa7da9fdbf7ee5c4b233a9fea752bc97fa461ce2d` |
| `orz-signer` | 1,401,792 | −8 B | `36572be0864c0e0b94c92335def7b00d08869e2c490c7b25d9de452b1b7f69aa` |
| `orz-acaf-provision` | 1,220,544 | +16 B | `a4883778740726d65eb2e0065be1864115ce0b12fbe34aa4e7be2ffe0aa8b0ce` |

- ELF 静态核验（逐件，alpine:3.20 `file`＋`readelf`）：三件均 `ET_DYN`（static-pie）＋
  `x86-64` ＋ **PT_INTERP=0**；BuildID `d2526f90…`／`0d100adc…`／`b5c988fd…`。
- 双向加载冒烟：`debian:bookworm-slim` 与 `alpine:3.20` 两侧、三件均 **exit 1**
  （orz `tui io error: No such device or address`／signer `manifest not found`／
  provision `usage`——与 052–076 同形态），**6/6**。
- **警告面（平台条件面基线）**：`orz-config` 1／`orz-assurance` 4／`orz-host` 2
  **与 0.6.14 同数**（三处均为非 Windows 分支的 unused 项，既有基线）；**`orz-web` 0 警告**。

## 6. 字面量核证（双方二进制，字节级出现次数；对照列＝在役 0.6.14 备份件）

脚本 [`.tmp-b077-literals.py`](../../.tmp-b077-literals.py)（可复跑，本批 **exit 0／failures=0**）。
**新面（0 → ≥1，双侧）**：

| 字面量 | Win 0.6.14 | Win 0.6.15 | Lin 0.6.14 | Lin 0.6.15 | 判读 |
|---|---:|---:|---:|---:|---|
| `【结束自述通道】` | 0 | **1** | 0 | **1** | 0bs ① 常驻尾行**进件** |
| `分块表见**窗口尾部**（逐轮刷新；压缩/回放均按块号指定）。` | 0 | **1** | 0 | **1** | 0bs ④ 单源指向行**进件** |
| `已收到，但未压缩任何分块` | 0 | **1** | 0 | **1** | 0bs ⑥ 未落地回执**进件** |
| `context_scale:summary_not_landed` | 0 | **7** | 0 | **1** | 0bs ⑥ 审计键**进件**（两侧计数差＝内联副本数差异，非缺陷） |

**保持面 50 项零回退**（逐项双侧同值，不回退、不减损；其中 `当前可压区间` 1 → **2** 双侧
——0bs ⑥ 回执新增一处引用，属预期增量）：

- **0.6.14 新面 7 项**（0bm 编辑面簇）：`ORZ_EDIT_MAX_FILE_BYTES`／
  `exceeding the edit-face size limit of`／`[回退窗口]` 2／`编辑前内容已存`／
  `编辑前内容快照存储失败`／`.gsa/rollback/`／`是否现在压缩、压缩哪些块由你判断`。
- **0.6.13 新面 8 项**（0br Web 工作台）：`/api/archives` 6／`trusted_workspaces` 2（Win）・3（Lin）／
  `archive://` 6／`reconstructed_from_journal` 3／`用法: orz archive <session8>` 1／`归档于 ` 1／
  `转写超上限已截断（` 1／`已信任工作区` 5。
- **0.6.12 新面 9 项**：`[emoji 剥离]` 1／`ORZ_WRITE_KEEP_EMOJI` 1／`Optional line-window anchor edit`
  1／`appears to be UTF-16 encoded` 1／`当前可压区间` 1 → 2／
  `context_scale:block_selection_unrecognized` 7／`context_scale:mandatory_summary_not_landed` 7／
  `至少命中 2 个小节` 1／`superseded_by_mandatory_window` 7。
- **0.6.11 新面 14 项**：`Call-scope lifecycle` 3／`[编码降级告知]` 3／`目标档：≤` 1／
  `[黑板说明书 guide v1 · digest sha256:` 1／`r<轮>·b<块>·s<seq>` 27／`…(+` 1／`条暂存` 1／
  `[RUN_END` 1／`[/RUN_END]` 1／`await_channel` 5／`lines truncated; narrow the pattern` 1／
  `Text files only: when true, return a section outline` 1／`No section headings found in this ` 1／
  `more headings omitted` 1。
- **更早批次面 12 项**：`RLI提醒:` 1／`持续越线` 1／`域迁移确认` 1／`掩盖缺口` 1／`CoverageGap` 2／
  `lif.domain_migration` 7／`分通道明细 selector=channels` 1／`符号: u=水平 v=速率 pred=闭式前推` 1／
  `本会话负担总值 ` 3／`资源软提示` 2／`utf-8-lossy` 1／`反例` 34。

> **方法边界（读数记录，不判失败）**：① **0bs ⑦ 状态符号窗口不可由字面量核证**——窗口是
> **码点表**（`STATUS_SYMBOL_WINDOW` 26 段 `u32` 区间），常量名与符号字面量均**不落字节面**
> （`STATUS_SYMBOL_WINDOW` 0/0；`⛔`／`⏳` 0/0；`✅` 1/1 两侧同值——来自其他数据面，
> 与本题无关）；该件核证面＝**单测钉**（`status_symbol_window_passes_with_vs16_and_never_counts`
> 等，emoji 子集 17/0）＋S4 真机「报告类产物不再被剥离」实读。② **载体版本串不落字节面**
> （076 §6 已登记，本批复现：`0.6.15`／`0.6.14` 在四件二进制中全为 0）⇒ 版本核证以
> **打包面**（资产名／包 README／manifest）为准。③ 字节级扫描是进件核证的**下界**，
> 不能分块隔离、不能覆盖未编译进二进制的元数据。

## 7. 换装、ACAF 与装配复核

- Windows（`D:\tb-eval\orz-windows`）：三件逐件 SHA256 与构建产物 **MATCH=True**；旧件留
  **`.0.6.14-bak`**；旧 manifest 留 `signer-manifest.json.bak-20260925-077`。
- ACAF 重 provision（`orz-acaf-provision.exe acaf\keystore acaf\signer-manifest.json`，**exit 0**）：
  keystore **逐位未动**（`acaf\installation-key.dpapi` `2408F596…`／`installation-key.json`
  `C491FC95…`；`acaf\keystore\` 内两件 `F37556AB…`／`2AA80CB8…`——**与 076 档四值逐位一致**）；
  回显 `binary_sha256=a831bf2f…` ↔ 换装位 `orz-signer.exe` **逐位一致**（manifest 169 B 同步更新）。
- 冒烟（换装位）：provision usage **exit 1**；signer fatal（manifest 缺失）**exit 1**。
- **载体级 Web 探针（脚本 [`.tmp-b077-win-web-probe.ps1`](../../.tmp-b077-win-web-probe.ps1)，端口 21557）**：
  `orz.exe web --addr 127.0.0.1:21557` 拉起换装位二进制 → 启动打印 64 位令牌 URL；
  `/`（静态）**200**；`/api/boot` 无令牌 **401**；伪造 Host **401**；带令牌 **200**，
  boot 键＝`cwd,agent,trusted_workspaces`；`/api/archives` 带令牌 **200** → **21 件**
  （本机真实归档数，较 076 首读 20 件 +1）；探针后 **`leftover_orz_procs=0`**。
- Linux：`/out` 直写即换装位（换装前 `.0.6.14-bak` 六件齐备）；ELF／冒烟读数见 §5。

### 7.1 本批摩擦（仅记录，不另立项）

- **PS 5.1 `-File` 读无 BOM 的 UTF-8 `.ps1`**：本批打包脚本首跑经
  `powershell -NoProfile -File .tmp-b077-package.ps1` 调用 ⇒ **解析失败**
  （`MissingEndParenthesisInMethodCall`）；同一文件在当前宿主（pwsh 7.6.5）内
  `Parser::ParseFile` **0 错误**、`& .\.tmp-b077-package.ps1` 直跑全过。根因＝
  **编码链消费侧**：PS 5.1 对无 BOM 的 UTF-8 脚本按系统 ANSI（GBK）解码 ⇒ 中文串
  被解成非法字节序，进而破坏引号/括号配对。**属 0bs ②（输出编码链）同族第四条实证**
  （供给侧 `SetConsoleOutputCP`／消费侧 `[Console]::OutputEncoding`／管道 `$OutputEncoding`
  ／**脚本文件本体编码**），随 0bt 的 ② 承接面复议，**不新增计数**。
- **`orz web` 启动文案 GBK 乱码复现**：本批探针经 `powershell -File` 捕获输出，
  启动三行中文仍呈 GBK 乱码（`Web 宸ヤ綔鍙板凡鍚姩`…）——**0bm／0bs ② 已立案面**
  在 0.6.15 载体上的又一次实证（载体侧 `SetConsoleOutputCP(65001)` 不覆盖
  「子进程文本被父进程按 ANSI 解码」这一段）；**不新增计数**。
- 本批**零残留进程**、**零时间线意外**；Windows 构建仅触发既有余量软提示（不阻断）。

## 8. 打包与发行状态

- 暂存 `D:\tb-eval\rel-077-stage\`（本地件不入库），入口脚本
  [`.tmp-b077-package.ps1`](../../.tmp-b077-package.ps1)（沿 066／074／075／076 形态）；
  包 README 由同脚本从 0.6.14 包 README 增量改写（新增「0.6.15 更新」节＋首段摘要，
  **14,477 B**）。

| 资产 | 大小 (B) | SHA256 |
|---|---:|---|
| `orz-0.6.15-windows-x86_64.zip` | 28,402,465 | `fc4593324d3734ee31bf33b7870c783dd0de615a7152ef57ec882f6679cc7fc2` |
| `orz-0.6.15-linux-x86_64.tar.gz` | 36,834,523 | `21658f974c481612a682da120b6157cbda17368a869311fefad518e0bc65b498` |
| `SHA256SUMS`（顶层） | 193 | （两行 `hash *name`、LF 行尾，见 stage 目录） |

- 包内容＝三件二进制＋`README.md`（0.6.15 更新说明：告知面收口／压缩握手／状态符号窗口
  ＋0.6.14 及更早的面＋两平台快速开始＋完整性校验）＋`SHA256SUMS`。
- 包完整性：解包回读逐件与换装载体 **同源同哈希（6/6 MATCH）**；容器内
  `sha256sum -c SHA256SUMS` **全 OK**（zip 侧 4/4、tar 侧 4/4、顶层 2/2，`alpine:3.20`）。
- **不发布 GitHub Release**：本批只做 **重建＋换装＋本地打包**；**0.6.15＝未发行**
  （用户令口径：0.6.14 属中间过渡版不发，本批未获发行令）；README 发布面**保持指向
  已发布 `v0.6.13` 不变**。资产与哈希已冻结留档。

## 9. 记账面（pin、清单、索引、计数）

- orz：**`a8430054`**（0bs 七件 `1ec7b729` ＋ bump；两笔已推 `cli`）。
- 父仓：本档 ＋ `orz_source_manifest.sha256` 重生成（1,498 条，差异两行＝bump 两文件）
  ＋ 子模块 pin 指向 `a8430054` ＋ TODO／BACKLOG 的 0bs「七件已进载体 0.6.15」标注
  ＋ 第二卷 §1.27 本批流水 ＋ 索引头行 → **v4.47**（v4.46 头行滚入
  `存档/index/CLI_PROJECT_INDEX_FULL_2026-09-24.md`，96 → **97 行**）。
- **计数不变**（**未闭合 56**）：本批为提交推送＋重建与换装，不新增／不闭合开放项。
- 门禁 `python scripts/check_repository.py`：提交后复跑须 `valid: true`（见 §10）。

## 10. 边界与未做项

1. **0.6.15 未发行**：见 §8；**在役载体已是 0.6.15**，发行面仍停在 `v0.6.13`。
2. **未跑真机狗粮轮**：本批只做到「0bs 增量进件＋字面量可核＋载体级 Web 探针＋换装」；
   0bs 的 **S4 复验五点**（① 结束自述常驻尾行实读 ② launcher 编码实读 ③ lsp 全量复跑
   ④ ⑦ 剥离计数不复发 ⑤ ⑥ 未落地回执构造实读）与 0bt 的 S2–S4 各行其账。
3. **0bs 增量只做静态与字面量进件核证**：行为面由单元／集成测试覆盖（orz-loop 851/0/3、
   orz-tools 2941/0/6、emoji 子集 17/0、lsp 子集 3×19/0）；**载体面首次行为复验留给
   下一轮真机**。
4. **未做双平台预检**（默认口径；§3）；Linux 侧仅做 apt 实包预检。

## 11. 证据留档（`.tmp-*` 门禁豁免）

`.tmp-b077-windows-build.log`／`.tmp-b077-linux-build.log`／`.tmp-b077-literals.py`／
`.tmp-b077-package.ps1`／`.tmp-b077-package.log`／`.tmp-b077-win-web-probe.ps1`／
`.tmp-b077-win-web-probe.log`／`.tmp-b077-win-web-probe.out.log`；
两平台 `.0.6.14-bak` 备份链与 `signer-manifest.json.bak-20260925-077` 保留；
发布包暂存 `D:\tb-eval\rel-077-stage\`（含 `_verify_zip`／`_verify_tar` 回读目录）。

## 12. 关联与关键词

[`076 重建档`](076_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-25.md) ／
[`075 重建档`](075_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-25.md) ／
[`0bs 轮报告`](0BS_FRICTION_REMEDIATION_AND_CARRYOVER_2026-09-25.md) ／
[`0bm 轮报告`](0BM_EDIT_FACE_IMPLEMENTATION_AND_FRICTION_2026-09-25.md) ／
[`0BS 检索线路线调研`](0BS_RETRIEVAL_ROUTE_SURVEY_2026-09-25.md) ／
[`ADR-0010 §14.78`](../../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)

关键词：0.6.15、0bs 进件、结束自述常驻尾行、分块表单源指向行、未落地回执、
`context_scale:summary_not_landed`、状态符号白名单窗口、字面量核证、载体级 Web 探针、
ACAF 重 provision、PT_INTERP=0、PS 5.1 无 BOM 脚本解码、版本串不落字节面、
**未发行载体**、本地打包留档。
