# 0.6.8 双平台载体重建与换装（2026-09-22）

> 用户令：「长会话真机狗粮轮依靠实际任务，用 0bd 作为任务」「现在请进行重建吧，并更新
> 0be 情况」「0.6.7 不补发了，当前这几轮没有重要更新且都是 RLI 正在修改的中间过渡版本，
> 不发也没什么问题」。本批三段：① **版本 bump**（0.6.7 → 0.6.8）② **双平台重建**
> ③ **换装＋ACAF 复核＋进件核证**；**打包与发行不在本批范围**（用户同日裁决：中间过渡版
> 不发行，0.6.7 亦不补发）。
> **源冻结线**：orz **`f76e5e32`**（`feat/fusion-architecture`）＝ 0bf RLI 生产化与域建模批
> `560e4387`（五件）＋ bump 两文件两行。上一载体：双平台 0.6.7（`6f23bbbf` 冻结，**不发
> 行**）。本批**免预检**（默认口径沿用 0be §8.1 收窄结论）。

## 1. 前置基线（0.6.7 在役件，换装前实取）

| 平台 | 文件 | 尺寸 (B) | SHA256 |
|---|---|---:|---|
| Windows | `orz.exe` | 53,506,048 | `A0FA066F434DDCEE3D987194C790C63D2FDC0798BCC681979C12C7C2C7027A9C` |
| Windows | `orz-signer.exe` | 6,729,728 | `C7B1044251622655798BE2FA2C3605FD5A4AECAA4BBB9D0C118BF3410CF28179` |
| Windows | `orz-acaf-provision.exe` | 6,638,592 | `132AFF660AD560DA3B05E0C3545FA9D119691DB71FB25F8F73A1A4D5D676017C` |
| Linux | `orz` | 112,053,424 | `e9f477562bbcd15ea2d933d03ab02aa94b763edf745ad614fa434653e2643708` |
| Linux | `orz-signer` | 1,400,944 | `a866d379a80f510c5cfe227cc5db815ab55d6e734628a536e4b2f20e27ad9d96` |
| Linux | `orz-acaf-provision` | 1,219,808 | `bf613e3ddd87d384121753ebc3fc2a8571c2e24c4cd34ef33a0731dfa2e264bb` |

六件与 [`067 档 §1/§4/§5`](067_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-22.md) 记录**逐位一致**
（在役件未被外力改动）。换装前逐件复制为备份链 **`.0.6.7-bak`**（Windows
`D:\tb-eval\orz-windows\`；Linux `D:\tb-eval\orz-linux\`）——备份件与该表**逐位一致**。

## 2. 版本 bump 与源冻结

- `crates/orz-bin/Cargo.toml` ＋ `Cargo.lock` **两文件两行**：`0.6.7 → 0.6.8`
  （orz 提交 **`f76e5e32`**，推 `cli`：`560e4387..f76e5e32`）。
- 版本号只在显式 bump 时递增；本批源冻结线＝`f76e5e32`；**0bf 五件首次进入在役载体**
  （0be 六件已于 0.6.7 进件）。

## 3. 预检：本批按默认口径免除

- 用户 2026-09-22 口径（0be §8.1 收窄后）：**默认不预检**，仅当出现只能在另一平台验证的
  条件编译面、或用户明确点名时才做。本批为 RLI 面纯逻辑 diff ＋ bump，**不做**双平台预检。

## 4. Windows 三件套（宿主 release，clean 全量）

- `cargo clean`：15 s，D: 余量 **20.5 → 40.4 GiB**。
- 构建：`cargo build --release -p orz-bin -j 4`（`PROTOC=D:\tb-eval\.tools\protoc-25.3\bin\protoc.exe`、
  `CARGO_INCREMENTAL=0`）；**exit 0，984 s（16m24s）**，日志 `.tmp-b068-windows-build.log`
  （067＝890.6 s 同量级；`ORZ-DEV-LINKER-CRASH-001` 未复现）。
- 构建期告警沿用既存项：`xai-tty-utils` `process_alive` dead_code **1 条**（0bd ④ 待清零，
  非本批引入）。

| 文件 | 尺寸 (B) | 与 0.6.7 差异（尺寸） | SHA256（换装位＝构建产物，MATCH=True） |
|---|---:|---:|---|
| `orz.exe` | 53,603,328 | +97,280 B | `4015380419B966C1863942B52B801CC68CDAC3ED888D388BFEA4493E572CDE61` |
| `orz-signer.exe` | 6,729,728 | ±0 B | `DA6EE484C1826FB2CF206ABB3F7CEC8EE4BFA9DD366C1765C85F76425C343571` |
| `orz-acaf-provision.exe` | 6,639,104 | +512 B | `CA5FE56F0EEBC3B378347E3C01B4A3BD78B2494B8303E14CF681AEBEF6DC1A37` |

## 5. Linux musl 三件套（Docker，直写换装位）

- 构建：`rust:1.97-slim` ＋ `build_orz_aliyun_trixie.sh`（ORZ-BUILD-MOUNT-001 契约；挂载
  脚本与仓库件逐位同哈希 `3D47CA09…`）——挂载 `-v D:/CLI:/orz`（父仓原地）＋
  `-v D:/tb-eval/orz-target:/target`（暖缓存）＋`-v D:/tb-eval/orz-linux:/out`（**直写换装
  位**，换装前已留 `.0.6.7-bak`）；**exit 0，656 s（10m56s）**，日志 `.tmp-b068-linux-build.log`。
- **容器未在源树留痕**：构建后 orz 仓 `git status` 为空。

| 文件 | 尺寸 (B) | 与 0.6.7 差异（尺寸） | SHA256 |
|---|---:|---:|---|
| `orz` | 112,158,432 | +105,008 B | `aa71ff8634cdd306f3c46390d9939456b3fbff1a9b52ce7ece34bf01e978898c` |
| `orz-signer` | 1,401,456 | +512 B | `3a6cf2bf0ec8c9e17687f2d324bd1cf1316fbebd7a3fa865c481041a90ee04ab` |
| `orz-acaf-provision` | 1,220,328 | +520 B | `3cef82a6295a8f3f2b86216bab142e99c3b5a1d8cb9fc48cab2de232ea0bd5d8` |

- ELF 静态核验（`.tmp-b066-elfcheck.py`，逐件）：三件均 `e_type=3`（ET_DYN static-pie）＋
  `e_machine=62`（x86-64）＋ **PT_INTERP=0**。
- 双向加载冒烟：`debian:bookworm-slim` 与 `alpine:3.20` 两侧、三件均 **exit 1**
  （orz 无 TTY `tui io error`／signer manifest 缺失／provision usage——与 052–067 同形态），
  **6/6**。

## 6. 字面量核证（双方二进制，字节级出现次数；对照列＝在役 0.6.7 备份件）

| 字面量 | Win 0.6.7 | Win 0.6.8 | Lin 0.6.7 | Lin 0.6.8 | 判读 |
|---|---:|---:|---:|---:|---|
| `RLI 未启用` | 0 | **1** | 0 | **1** | 0bf① kill switch 关闭面进件 |
| `RLI on` | 0 | **1** | 0 | **1** | 0bf① 常开头行进件 |
| `影子 on` | 1 | **0** | 1 | **0** | 0bf① 旧影子头行**退役**（同一改动两面，见注一） |
| `转移倾向: [` | 0 | **1** | 0 | **1** | 0bf② 域转移倾向读数行进件 |
| `近提醒: ` | 0 | **1** | 0 | **1** | 0bf② 事件追溯近提醒行进件 |
| `域迁移确认: ` | 0 | **1** | 0 | **1** | 0bf③ 触发一（稳定后一次）进件 |
| `持续越线: ` | 0 | **1** | 0 | **1** | 0bf③ 触发二（k=5）进件 |
| `（失配概率 ` | 0 | **1** | 0 | **1** | 0bf③ 失配概率随报进件 |
| `RLI提醒:` | 0 | **1** | 0 | **1** | 0bf③ 模型面提醒投递前缀进件 |
| `本会话负担总值 ` | 0 | **3** | 0 | **3** | 0bf④ 用户面复合总值（50/70/90 三档）进件 |
| `（网格补点 ` | 0 | **1** | 0 | **1** | 0bf④ 10 s 网格补点计数面进件 |
| `越过本会话自校准 q85=` | 1 | **0** | 1 | **0** | 0bf③ 用户面改复合总值档 ⇒ 0be q85 档文案**退役**（见注二） |
| `建议适时换新对话` | 1 | 1 | 1 | 1 | 0be④ q95 档建议文案保持（并入档 70 文案） |
| `λ̂=` | 1 | 1 | 1 | 1 | 0be① λ̂ 读数列保持 |
| `pred1_prog` | 10 | 10 | 6 | 6 | 0be② 短视锚点 feature 名保持 |
| `p1(1T̂)` | 6 | 6 | 6 | 6 | 0be② 短视档渲染色标保持 |
| `ORZ_LIF_RLI_SHADOW` | 7 | 7 | 7 | 7 | 0am 影子开关 env 保持（语义已反转，见 0bf①） |
| `slow_prog`／`fast_prog` | 12／12 | 12／12 | 8／8 | 8／8 | 0am 模态对保持 |
| `rli.history` | 2 | 2 | 2 | 2 | 0am 域级定位面保持 |
| `ORZ_JOB_CPU_RATE_PERCENT` | 1 | 1 | 1 | 1 | 0bc CPU 去顶保持 |
| `ORZ_JOB_ACTIVE_PROCESS_LIMIT` | 1 | 1 | 1 | 1 | 0bc 进程上限 env 保持 |
| `资源软提示` | 2 | 2 | 2 | 2 | 0bc 软提示文案保持 |
| `utf-8-lossy:` | 1 | 1 | 1 | 1 | 0as 降级读数保持 |

> **注一（`影子 on` 1→0 非回退）**：0bf① 把 rli 面头行由「影子 on」改为「RLI on」
> （影子语义退役＝常开，见 0bf 报告 §2.1），旧字面量随改；同处新增 `RLI 未启用`
> （kill switch 关闭臂）与 `（网格补点 …）` 计数段。
> **注二（`越过本会话自校准 q85=` 1→0 非回退）**：0be④ 的用户面繁杂度提醒为 q85／q95
> 分位档自校准文案；0bf③ 按用户令改为**「水位＋0.3×繁杂度」复合总值**、档位固定
> **50/70/90**（侧车单键 `burden_tiers_notified` 取代 0be 两键，见 0bf 报告 §1 判据 3／§2.4），
> 故旧 q85 档文案退役——**同一改动两面**。
> **注三（核证边界）**：字面量核证只回答「面是否进件」，**不构成语义等价证明**；功能级读数
> 由真机狗粮轮收取（本批未跑）。

## 7. 换装、ACAF 与装配复核

- Windows（`D:\tb-eval\orz-windows`）：三件逐件 SHA256 与构建产物 **MATCH=True**；旧件留
  **`.0.6.7-bak`**；旧 manifest 留 `signer-manifest.json.bak-20260922-068`。
- ACAF 重 provision（root＝`D:\tb-eval\orz-windows\acaf`，**exit 0**）：keystore **逐位未动**
  （`installation-key.dpapi` `F37556AB…`／`installation-key.json` `2AA80CB8…` 前后一致）；
  回显 `binary_sha256=da6ee484c1826fb2cf206abb3f7cec8ee4bfa9dd366c1765c85f76425c343571`
  ↔ 换装位 `orz-signer.exe` **逐位一致**。
- 冒烟（换装位）：provision usage **exit 1**；signer fatal（manifest 缺失路径）**exit 1**。
- 宿主装配复核：`scripts/dogfood_launch.ps1 -TaskFile .tmp-0bf-task.txt -DryRun` 断言全过
  （cwd＝`D:\CLI` 断言通过、载体哈希回显 `4015380419B9…`、ACAF fail-closed=True、
  `PROTOC`／`GROK_HOME` 就位、flags `--real --allow-write --allow-shell --allow-network`）。
  版本列显示 `unknown` 系 Rust 载体无 VersionInfo 的**既有口径**（2026-09-18 已登记），
  非本批缺陷。
- Linux：构建脚本直写 `/out`（＝`D:\tb-eval\orz-linux`），在役 0.6.7 已预留 `.0.6.7-bak`；
  评测容器运行期自行 provision，不涉 Windows 侧 manifest。

## 8. 0be 台账更正（用户令「并更新 0be 情况」）

- 更正口径：0be **S1／S2／S3 达成**（机制码 orz `0b7b89dd` 已提交并**已进在役载体**；
  S3 真机两轮读数齐），**唯一余项＝S4 的 ADR-0010 转录**——与 0bf 同属 RLI 主线，
  **合并为一条随 0bf S4 落地**（不为中间过渡语义重复转录），届时 0be／0bf 一并闭合。
- 残留验证面（繁杂度投递链／侧车真机覆盖、探针收编、冻结语料纪律、host 两条既存红）
  **非 0be 独有开放面**，已分别挂在 0bf S3 与 0bd ⑩⑪⑫⑬。
- **未闭合计数维持 46**（0be 仍开放，余项仅 S4）；三件台账（BACKLOG 0be 节／TODO P1-0be／
  索引 §8 pending 桶）同步，BACKLOG P1 开放项行加注状态。

## 9. 长会话轮任务源（用户令）

- 用户令「长会话真机狗粮轮依靠实际任务，用 0bd 作为任务」⇒ **0bf S3 长会话轮与 0bd
  真机狗粮轮同轮执行**（0bd 十四件即该轮实际任务），同轮并收 **0bc S3 三项资源面读数**
  与 **0be 投递链／侧车覆盖**。前置＝0bf 码进载体（本批 0.6.8 已完成）。
- 本批**未跑**该轮（待放行）；0bd S1/S2 未开工。

## 10. 记账面（pin、清单、提交与推送）

- orz：`560e4387`（0bf 批）→ **`f76e5e32`**（bump），均推 `cli`。
- 父仓：本档 ＋ 子模块 pin ＋ `orz_source_manifest.sha256` 重算 ＋ 0be 台账更正 ／
  索引 v4.21 → **v4.22** ＋ 台账第二卷随批流水，推 `origin main`；门禁 `valid: true`。

## 11. 本批未做（边界）

1. **不打包、不发行**：0.6.7 **不补发**（用户令）；0.6.8 同属 RLI 中间过渡版，按同一裁决
   **亦不发行**。若后续需发行，沿用 `.tmp-b066-package.ps1` 形态（改 `$ver`／暂存目录）。
2. **未跑真机狗粮轮**：RLI 常开后的实际读数、0bc S3 三项资源面、0be 投递链／侧车真机覆盖、
   0bd 十四件效果——全部留待 §9 的同轮执行。
3. **LIF 退役裁决**：待 RLI 转正后首轮真机读数（0bf ①）。

## 12. 证据留档（`.tmp-*` 门禁豁免）

`.tmp-b068-windows-build.log`／`.tmp-b068-linux-build.log`／`.tmp-b068-lits-extract.py`／
`.tmp-b068-literals.py`／`.tmp-orz-commit-msg-bump068.txt`；两平台 `.0.6.7-bak` 备份链与
`signer-manifest.json.bak-20260922-068` 保留。

## 13. 关联

[`0bf 报告`](0BF_RLI_PRODUCTION_AND_DOMAIN_MODELING_2026-09-22.md) ／
[`0be 报告`](0BE_RLI_ONLINE_ADAPTIVE_2026-09-21.md) ／
[`067 重建档`](067_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-22.md) ／
[`BACKLOG`](../BACKLOG_AND_PRIORITIES.md) ／ [`TODO`](../../TODO.md)。
关键词：0.6.8、双平台、载体重建、中间过渡版、不发行、免预检、clean 全量、暖缓存、
ACAF 重 provision、Linux musl static-pie、字面量核证、RLI 常开、kill switch、域转移倾向、
两触发提醒、复合总值、10 s 网格、0be 台账更正。
