# 0.6.9 双平台载体重建与换装（2026-09-22）

> 用户令：「现在的话进一轮重建吧，把新的代码应用上」。本批三段：① **版本 bump**
> （0.6.8 → 0.6.9）② **双平台重建** ③ **换装＋ACAF 复核＋进件核证**；**打包与发行不在
> 本批范围**（0.6.7 不补发、0.6.8 中间过渡版不发行的既有裁决沿用）。
> **源冻结线**：orz **`88b6dea1`**（`feat/fusion-architecture`）＝ 0bd 摩擦大杂项轮
> `56d328ee`（六件）＋ bump 两文件两行。上一载体：双平台 0.6.8（`f76e5e32` 冻结，
> **不发行**）。本批**免预检**（默认口径沿用 0be §8.1 收窄结论）。

## 1. 前置基线（0.6.8 在役件，换装前实取）

| 平台 | 文件 | 尺寸 (B) | SHA256 |
|---|---|---:|---|
| Windows | `orz.exe` | 53,603,328 | `4015380419B966C1863942B52B801CC68CDAC3ED888D388BFEA4493E572CDE61` |
| Windows | `orz-signer.exe` | 6,729,728 | `DA6EE484C1826FB2CF206ABB3F7CEC8EE4BFA9DD366C1765C85F76425C343571` |
| Windows | `orz-acaf-provision.exe` | 6,639,104 | `CA5FE56F0EEBC3B378347E3C01B4A3BD78B2494B8303E14CF681AEBEF6DC1A37` |
| Linux | `orz` | 112,158,432 | `aa71ff8634cdd306f3c46390d9939456b3fbff1a9b52ce7ece34bf01e978898c` |
| Linux | `orz-signer` | 1,401,456 | `3a6cf2bf0ec8c9e17687f2d324bd1cf1316fbebd7a3fa865c481041a90ee04ab` |
| Linux | `orz-acaf-provision` | 1,220,328 | `3cef82a6295a8f3f2b86216bab142e99c3b5a1d8cb9fc48cab2de232ea0bd5d8` |

六件与 [`068 档 §4/§5`](068_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-22.md) 记录**逐位一致**
（在役件未被外力改动）。换装前逐件复制为备份链 **`.0.6.8-bak`**（Windows
`D:\tb-eval\orz-windows\`；Linux `D:\tb-eval\orz-linux\`）——备份件与该表**逐位一致**。

## 2. 版本 bump 与源冻结

- `crates/orz-bin/Cargo.toml` ＋ `Cargo.lock` **两文件两行**：`0.6.8 → 0.6.9`
  （orz 提交 **`88b6dea1`**；**本轮未推送**——用户令只含重建，推送留后续批）。
- 版本号只在显式 bump 时递增；本批源冻结线＝`88b6dea1`；**0bd 六件首次进入在役载体**
  （`xai-tty-utils` 再导出／`orz-tools` 工具描述与测试面／`orz-loop` 测试分支／
  `orz-host` 夹具／新 example 探针）。

## 3. 预检：本批按默认口径免除

- 用户 2026-09-22 口径：**默认不预检**，仅当出现只能在另一平台验证的条件编译面、
  或用户明确点名时才做。本批为 shell 描述文案＋测试面＋再导出（＋ bump），
  **不做**双平台预检。

## 4. Windows 三件套（宿主 release，clean 全量）

- 构建入口＝**0bd ① 新脚本** `scripts/build_orz.ps1 -Release -Clean`（本批即该脚本的
  首次真机使用）：自检读数 **12 核／物理 15.8 GiB（余 5.9）／commit 23.5 GiB
  （余 4.0，使用 82.9%）** ⇒ auto 档**判 `-j 2`**（与 0bc/0be 惯例档一致）。
- `cargo clean`：**29,186 文件／21.2 GiB**；构建 **exit 0，1705.4 s（28m26s）**，
  日志 `.tmp-b069-windows-build.log`。**警告 0 条**（068 轮尚存的 `xai-tty-utils`
  dead_code 与 `orz-tools` unused variable 两条已由 0bd ④a/④b 清零）。
- **墙钟对照（读数，非缺陷）**：068 轮同口径 clean 全量为 984 s（16m24s）且**显式
  `-j 4`**；本批 auto 档判 **2** ⇒ **28m26s**。auto 档的收益是规避宿主 commit 上限
  导致的 OOM（0am FR2），代价是 commit 吃紧时墙钟显著拉长——两档均为**预期行为**。

| 文件 | 尺寸 (B) | 与 0.6.8 差异（尺寸） | SHA256（换装位＝构建产物，MATCH=True） |
|---|---:|---:|---|
| `orz.exe` | 53,603,328 | ±0 B | `0C4598B0515D033C1155FF0CD9B3B2CD15CED8F5261178ACFD54FB79AE39C110` |
| `orz-signer.exe` | 6,729,728 | ±0 B | `76D8D0F4770A844BD5B47A91D18542EDDE6D3719899190281A81690CE5665434` |
| `orz-acaf-provision.exe` | 6,639,104 | ±0 B | `BE622ACAEBD8633C2BDB28AA9CE9F6038B0EE40DB42F8BB0766E7ABBF8881EC9` |

> 三件尺寸与 0.6.8 全同、哈希全异：本批改动为**文案＋测试面＋再导出**（无新增/删除
> 大段代码），尺寸不变属预期；哈希差异亦含重编译非确定性（signer／provision 未改码）。

## 5. Linux musl 三件套（Docker，直写换装位）

- **实包预检**：`apt-get update -qq && apt-get install -y -qq musl-tools` →
  **APT_EXIT=0** ⇒ 按 [`Docker 代理配方`](../../scripts/DOCKER_PROXY_RECIPE.md) §0
  **不切代理、不重启**（日志 `.tmp-b069-linux-apt-pre.log`）。
- 构建：`rust:1.97-slim` ＋ `build_orz_aliyun_trixie.sh`（ORZ-BUILD-MOUNT-001 契约；
  脚本与仓库件同源），挂载 `-v D:/CLI:/orz`（父仓原地）＋`-v D:/tb-eval/orz-target:/target`
  （暖缓存）＋`-v D:/tb-eval/orz-linux:/out`（**直写换装位**，换装前已留 `.0.6.8-bak`）；
  **exit 0，578.2 s（9m38s）**，日志 `.tmp-b069-linux-build.log`。
- **容器未在源树留痕**：构建后 orz 仓 `git status` 为空。

| 文件 | 尺寸 (B) | 与 0.6.8 差异（尺寸） | SHA256 |
|---|---:|---:|---|
| `orz` | 112,158,136 | −296 B | `d17a607f8a67f12559186bbf4dd92747566c46004cde6e875017994d90c15589` |
| `orz-signer` | 1,401,320 | −136 B | `6626f6c83fe2bdc868669811246bf84b062cbc934c20eaedeb9085d679d37864` |
| `orz-acaf-provision` | 1,220,328 | ±0 B | `8544e8c9f63ed4ff94661c33db7fd1d8f72cc771ce4c3d0daef2c27da7dfbdf4` |

- ELF 静态核验（`.tmp-b066-elfcheck.py`，逐件）：三件均 `e_type=3`（ET_DYN static-pie）＋
  `e_machine=62`（x86-64）＋ **PT_INTERP=0**。
- 双向加载冒烟：`debian:bookworm-slim` 与 `alpine:3.20` 两侧、三件均 **exit 1**
  （orz 无 TTY `tui io error`／signer manifest 缺失／provision usage——与 052–068 同形态），
  **6/6**。
- **警告面（平台条件面基线）**：`xai-tty-utils` **1 → 0**（0bd ④a 在 Linux 侧同样生效）；
  `orz-config` 1／`orz-assurance` 4／`orz-host` 2 **与 0.6.8 同数**（Unix 侧既有项，
  非本批引入）。

## 6. 字面量核证（双方二进制，字节级出现次数；对照列＝在役 0.6.8 备份件）

| 字面量 | Win 0.6.8 | Win 0.6.9 | Lin 0.6.8 | Lin 0.6.9 | 判读 |
|---|---:|---:|---:|---:|---|
| `Windows PowerShell notes:` | 0 | **1** | 0 | **1** | 0bd③b shell 工具描述 PowerShell 注**进件** |
| `swallows a bare` | 0 | **1** | 0 | **1** | 0bd③b `&` 吞裸 `--` 说明**进件** |
| `[Console]::OutputEncoding` | 0 | **1** | 0 | **1** | 0bd③b CJK 输出编码处方**进件** |
| `powershell -File <script.ps1> <args>` | 0 | **1** | 0 | **1** | 0bd③b 推荐调用式**进件** |
| `process_alive` | 0 | 0 | 0 | 0 | 0bd④a 为**再导出**（符号面），二进制字面量不增——非回退 |
| `RLI 未启用`／`RLI on`／`转移倾向: [`／`近提醒: ` | 1／1／1／1 | 1／1／1／1 | 1／1／1／1 | 1／1／1／1 | 0bf① ② 面保持 |
| `域迁移确认: `／`持续越线: `／`（失配概率 `／`RLI提醒:` | 1／1／1／1 | 1／1／1／1 | 1／1／1／1 | 1／1／1／1 | 0bf③ 两触发与投递面保持 |
| `本会话负担总值 `／`（网格补点 ` | 3／1 | 3／1 | 3／1 | 3／1 | 0bf④ 用户面复合总值与网格补点保持 |
| `λ̂=`／`pred1_prog`／`p1(1T̂)` | 1／10／6 | 1／10／6 | 1／6／6 | 1／6／6 | 0be①② 读数列保持（Win/Lin 计数差异为既有形态） |
| `建议适时换新对话`／`越过本会话自校准 q85=` | 1／0 | 1／0 | 1／0 | 1／0 | 0be④ 档位文案保持（旧 q85 档按设计退役） |
| `ORZ_LIF_RLI_SHADOW`／`slow_prog`／`fast_prog`／`rli.history` | 7／12／12／2 | 7／12／12／2 | 7／8／8／2 | 7／8／8／2 | 0am 开关／模态对／域级定位面保持 |
| `ORZ_JOB_CPU_RATE_PERCENT`／`ORZ_JOB_ACTIVE_PROCESS_LIMIT`／`资源软提示` | 1／1／2 | 1／1／2 | 1／1／2 | 1／1／2 | 0bc 资源面保持 |
| `utf-8-lossy:` | 1 | 1 | 1 | 1 | 0as 降级读数保持 |

> **核证边界**：字面量核证只回答「面是否进件」，**不构成语义等价证明**；功能级读数
> 由真机狗粮轮收取（本批未跑）。

## 7. 换装、ACAF 与装配复核

- Windows（`D:\tb-eval\orz-windows`）：三件逐件 SHA256 与构建产物 **MATCH=True**；旧件留
  **`.0.6.8-bak`**；旧 manifest 留 `signer-manifest.json.bak-20260922-069`。
- ACAF 重 provision（root＝`D:\tb-eval\orz-windows\acaf`，**exit 0**）：keystore **逐位未动**
  （外层 `installation-key.dpapi` `2408F596…`／`installation-key.json` `C491FC95…`；
  `keystore/` 内两件 `F37556AB…`／`2AA80CB8…`——**前后四值全一致**）；
  回显 `binary_sha256=76d8d0f4770a844bd5b47a91d18542edde6d3719899190281a81690ce5665434`
  ↔ 换装位 `orz-signer.exe` **逐位一致**（manifest 169 B 同步更新）。
- 冒烟（换装位）：provision usage **exit 1**；signer fatal（manifest 缺失）**exit 1**。
- 宿主装配复核：`scripts/dogfood_launch.ps1 -TaskFile .tmp-0bd-task.txt -DryRun` 断言全过
  （cwd＝`D:\CLI` 断言通过、载体哈希回显 `0C4598B0515D…`、ACAF fail-closed=True、
  `PROTOC`／`GROK_HOME` 就位、**`rli=on（缺省常开）`＋`ORZ_LIF_RLI_SHADOW=1` 回显**＝
  0bd ⑤⑧ 生效、flags `--real --allow-write --allow-shell --allow-network`；
  题面 `.tmp-0bd-task.txt` 回显 **116 字符・utf-8 BOM・读取＝显式 UTF-8**＝0bd ⑭ 生效）。
  版本列显示 `unknown` 系 Rust 载体无 VersionInfo 的**既有口径**，非本批缺陷。
- Linux：构建脚本直写 `/out`（＝`D:\tb-eval\orz-linux`），在役 0.6.8 已预留 `.0.6.8-bak`；
  评测容器运行期自行 provision，不涉 Windows 侧 manifest。
- **入口脚本功能性复核（PS 5.1 真机，非 dry-run）**：`powershell -File scripts\run_orz_tests.ps1
  metadata -q --nocapture` ⇒ 脚本**解析通过**（BOM 修复生效）＋打印
  「检测到被吞的前导 `--`：已在测试参数前补回分隔符（0bd ③）」＋cargo 侧报
  `unexpected argument '--nocapture'`（＝分隔符确实抵达 cargo，0bd ③a 在 5.1 下实证）。

## 8. 本轮摩擦与处置（重建面新增两条）

1. **无 BOM 的 UTF-8 `.ps1` 在 PS 5.1 下解析失败（已修，⑭ 同族）**：`scripts/build_orz.ps1`
   经 `powershell -File` 调用首轮即 8 处解析错误（ANSI 误读中文串，引号配对被打乱）；
   同批 `dogfood_launch.ps1`（2 处）、`run_orz_tests.ps1`（3 处）解析面同样不通过
   （0bd 轮内由 pwsh 7 调用故未暴露）。处置＝**三脚本补 UTF-8 BOM**，
   PS 5.1 解析 **0 错**（三脚本复测）＋**构建与测试入口两脚本均经 5.1 实跑通过**
   （本批 Windows 全量构建即经 `powershell -File scripts\build_orz.ps1`，
   `run_orz_tests.ps1` 的 `--` 补回面见 §7 末条）；与 0bd §6-a 的「写入端强制 BOM」
   同族，写入端强制 BOM 的**题面**面仍留 0bg。
2. **构建并行档读数（观察，非缺陷）**：0bd ① 的 auto 档在多任务并存时判 `-j 2`
   （commit 使用 82.9%）⇒ 墙钟 28m26s，对 `-j 4` 的 16m24s 近乎翻倍。两档均按设计
   行为（auto 档优先防 OOM）；后续若需缩短墙钟，可显式 `-Jobs 4` 或收敛宿主 commit 占用。

## 9. 记账面（pin、清单、索引）

- orz：`56d328ee`（0bd 批）→ **`88b6dea1`**（bump，**本地未推送**）。
- 父仓：本档 ＋ 子模块 pin ＋ `orz_source_manifest.sha256` 重算 **1463 条**（差异恰
  **2 行**＝bump 两文件）＋ 0bd 台账「进载体」标注 ＋ 索引 v4.24 → **v4.25** ＋
  三脚本 BOM 修复；门禁 `valid: true`（error_count 0）。

## 10. 本批未做（边界）

1. **不打包、不发行**：0.6.7 不补发（既有裁决）；0.6.9 同属 RLI 中间过渡版，
   按同一裁决**亦不发行**。若后续需发行，沿用 `.tmp-b066-package.ps1` 形态。
2. **未跑真机狗粮轮**：0bd 六件的功能级效果（工具描述对模型命令形态的影响、
   `--` 吞参回补、测试入口清 env 等）留待下一轮真机；0bg 九件的真机轮同此。
3. **未推送**：orz bump 与父仓本批改动均留本地（用户令只含重建）。

## 11. 证据留档（`.tmp-*` 门禁豁免）

`.tmp-b069-windows-build.log`／`.tmp-b069-linux-build.log`／`.tmp-b069-linux-apt-pre.log`／
`.tmp-b069-literals.py`／`.tmp-orz-commit-msg-bump069.txt`；两平台 `.0.6.8-bak` 备份链、
`signer-manifest.json.bak-20260922-069` 与 `.tmp-b069-*` 日志保留。

## 12. 关联

[`068 重建档`](068_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-22.md) ／
[`0bd 报告`](0BD_FRICTION_MISC_2026-09-22.md) ／
[`BACKLOG`](../BACKLOG_AND_PRIORITIES.md) ／ [`TODO`](../../TODO.md) ／
[`Docker 代理配方`](../../scripts/DOCKER_PROXY_RECIPE.md)。
关键词：0.6.9、双平台、载体重建、中间过渡版、不发行、免预检、clean 全量、暖缓存、
ACAF 重 provision、Linux musl static-pie、字面量核证、0bd 六件进件、
无 BOM 脚本、PS 5.1 解析、自动降并行、未推送。
