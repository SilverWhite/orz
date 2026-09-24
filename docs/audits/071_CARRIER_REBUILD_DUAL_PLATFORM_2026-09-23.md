# 0.6.11 双平台载体重建与换装（2026-09-23）

> 用户令：「请先进行重建吧」。本批三段：① **版本 bump**（0.6.10 → 0.6.11）
> ② **双平台重建** ③ **换装＋ACAF 复核＋进件核证**；另附 **§8 独立复跑抓出的四条
> 落码面缺口与其当场修复**（本批实质增量）。**打包与发行不在本批范围**（0.6.7–0.6.10
> 均为中间过渡版、不发行的既有裁决沿用；0.6.11 同属中间过渡版，**待用户裁决是否发行**）。
> **源冻结线**：orz **`917fadfb`**（`feat/fusion-architecture`）＝ 0bh 交互面收口二轮
> `5e153de1`（十六件落码十三件 ＋ ⑤ 设计定稿）＋ bump 两文件两行；**载体二进制由冻结线构建**。
> 上一载体：双平台 0.6.10（`f36dee7c` 冻结，不发行）。本批**免预检**（0be §8.1 收窄口径沿用）。

## 1. 前置基线（0.6.10 在役件，换装前实取）

| 平台 | 文件 | 尺寸 (B) | SHA256 |
|---|---|---:|---|
| Windows | `orz.exe` | 54,653,952 | `8F67381FEF2834142DB5E8F952374695042C4BDE5D7A43125FCD7477CAB8F0F0` |
| Windows | `orz-signer.exe` | 6,742,528 | `75E8C98BAE61C036FB2D6A5E90830ED74C1227498EF65FFF4D5D7F96542E8670` |
| Windows | `orz-acaf-provision.exe` | 6,642,176 | `791229E4B7DF37016E566AF829A5AB24AE4EF329514248E1DA9AF2C06AEEF2AD` |
| Linux | `orz` | 112,186,200 | `91BE5F9E7654EB786089AEB844EF908DB486431EA85DC5B48423740162E8C7F4` |
| Linux | `orz-signer` | 1,401,800 | `28E1BAFFA90EF4FD4E8D9EC4987F400B00D2683EE058F934B72BFB2B7FAFD319` |
| Linux | `orz-acaf-provision` | 1,220,536 | `7BA681AB802718D9B25BC5BFA78A4CD4189A1AC1EC18FABF275A2D60EF6DFF6B` |

六件与 [`070 档 §4/§5`](070_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-22.md) 记录**逐位一致**
（在役件未被外力改动）。换装前逐件复制为备份链 **`.0.6.10-bak`**（Windows
`D:\tb-eval\orz-windows\`；Linux `D:\tb-eval\orz-linux\`）——逐件 `backup_ok=True`。

## 2. 版本 bump 与源冻结

- `crates/orz-bin/Cargo.toml` ＋ `Cargo.lock` **两文件两行**：`0.6.10 → 0.6.11`
  （orz 提交 **`917fadfb`**；**本轮未推送**——用户令只含重建，推送留后续批）。
- 版本号只在显式 bump 时递增；本批源冻结线＝`917fadfb`；**0bh 十三件首次进入在役载体**
  （① 提醒投递预算独立化／② 调用级拆树告知面／③ 压缩回执带机械摘要＋定位符／
  ④ 压缩带宽 192／256 与目标档建议／⑥ 构建争用面 `-WaitForHeadroom`／⑦⑧ 体例与承接／
  ⑩ 乱码降级告知／⑪ grep 命中预算尾注／⑫ 审计档按节定位 `outline`／⑭ 黑板 guide 分区／
  ⑮ 机械定位符与 journal anchor 回查／⑯ 中性终态与结束自述）。⑤ 为设计定稿（无代码）；
  ⑨⑬ 未落码（S1 定案留档，承接 0bi S2）。
- 父仓侧本批起始时**带前批未提交改动**（索引 v4.31 头行 ＋ BACKLOG 条件触发节；见 §8 摩擦 4）。

## 3. 预检：本批按默认口径免除

- 用户 2026-09-22 口径：**默认不预检**，仅当出现只能在另一平台验证的条件编译面、
  或用户明确点名时才做。本批为交互面／文案面＋测试面（＋ bump），**不做**双平台预检。

## 4. Windows 三件套（宿主 release，clean 全量）

- 构建入口＝`scripts/build_orz.ps1 -Release -Clean`（auto 档自检读数 **12 核／物理 15.8 GiB
  （余 7.2）／commit 23.2 GiB（余 4.8，使用 79.1%）** ⇒ 判 **`-j 2`**，并触发 0bh ⑥ 的
  **余量软提示**（「判定归你，脚本不代办」原文回显——⑥ 新面在宿主脚本面首次实跑）。
- `cargo clean`：**41,799 文件／25.6 GiB**。构建 **exit 0**，`Finished release` **26m30s**
  （墙钟 26.8m）；**警告 0 条、错误 0 条**（`protoc` 前置按 0.6.10 批修复直接生效，无 070 §8 ① 复现）。

| 文件 | 尺寸 (B) | 与 0.6.10 差异 | SHA256（换装位＝构建产物，MATCH=True） |
|---|---:|---:|---|
| `orz.exe` | 54,768,640 | +114,688 B | `75EAA6E26A2CC37EC8EDADB865B65FE4E49F87652ED2B378DE553DE4FB44B5D7` |
| `orz-signer.exe` | 6,742,528 | ±0 | `20F8B0D3581FC3F8842273C73CEEE6C47A26548C635A606B30B1A511FBCEB2BA` |
| `orz-acaf-provision.exe` | 6,642,176 | ±0 | `A802517844A5429A523A7907B3E6BF46D7F2FE41F8B6EFA98CBC4460D1567FD0` |

> `orz.exe` +114 KB 与 0bh 进件量级相称（新设计档面 ＋ 结束自述解析 ＋ 定位符 ＋ 折叠面）；
> `signer`／`provision` **无源码改动**（版本串 `0.6.10→0.6.11` 为唯一差异来源），尺寸不变、
> 哈希变动属重编译非确定性——与 068–070「同码不同哈希」同族，非回退。

## 5. Linux musl 三件套（Docker，直写换装位）

- **实包预检**：`apt-get update -qq && apt-get install -y -qq musl-tools` → **APT_EXIT=0**
  ⇒ 按 [`Docker 代理配方`](../../scripts/DOCKER_PROXY_RECIPE.md) §0 **不切代理、不重启**
  （日志 `.tmp-b071-linux-apt-pre.log`）。
- 构建：`rust:1.97-slim` ＋ `bash /orz/scripts/build_orz_aliyun_trixie.sh`（ORZ-BUILD-MOUNT-001
  契约；脚本与仓库件同源），挂载 `-v D:/CLI:/orz`（父仓原地）＋`-v D:/tb-eval/orz-target:/target`
  （暖缓存）＋`-v D:/tb-eval/orz-linux:/out`（**直写换装位**，换装前已留 `.0.6.10-bak`）；
  **exit 0**：容器内 `Finished release` **8m34s**、含工具链与静态 `rg` 安装全流程 **10.8m**
  （日志 `.tmp-b071-linux-build.log`）。
- **容器未在源树留痕**：构建后 orz 仓 `git status` 为空。

| 文件 | 尺寸 (B) | 与 0.6.10 差异 | SHA256 |
|---|---:|---:|---|
| `orz` | 112,300,264 | +114,064 B | `7ba2e5ce44509a481b018436001dc6e211e0f891d6579225e1ab4ddd2ec50480` |
| `orz-signer` | 1,401,880 | +80 B | `73413f27053d26e60ca9d6c0d0954d9e16589e3a3357f8b89c21b248e0651eaf` |
| `orz-acaf-provision` | 1,220,568 | +32 B | `eef4ae2c50d857675b11c23878396491109aa7104daddd8d3a72979e135dac87` |

- ELF 静态核验（逐件）：三件均 `e_type=3`（ET_DYN static-pie）＋ `e_machine=62`（x86-64）
  ＋ **PT_INTERP=0**。
- 双向加载冒烟：`debian:bookworm-slim` 与 `alpine:3.20` 两侧、三件均 **exit 1**
  （orz 无 TTY `tui io error`／signer manifest 缺失／provision usage——与 052–070 同形态），
  **6/6**。
- **警告面（平台条件面基线）**：`orz-config` 1／`orz-assurance` 4／`orz-host` 2
  **与 0.6.10 同数**（xai-tty-utils 0，与 070 同）。

## 6. 字面量核证（双方二进制，字节级出现次数；对照列＝在役 0.6.10 备份件）

脚本 `.tmp-b071-literals.py`（可复跑）。**新面（0 → ≥1）**：

| 字面量 | Win 0.6.10 | Win 0.6.11 | Lin 0.6.10 | Lin 0.6.11 | 判读 |
|---|---:|---:|---:|---:|---|
| `Call-scope lifecycle` | 0 | **3** | 0 | **3** | ② 三份 bash 描述模板**进件** |
| `[编码降级告知]` | 0 | **3** | 0 | **3** | ⑩ 乱码降级告知**进件** |
| `目标档：≤` | 0 | **1** | 0 | **1** | ④ 压缩目标档建议**进件** |
| `[黑板说明书 guide v1 · digest sha256:` | 0 | **1** | 0 | **1** | ⑭ guide 分区面头**进件** |
| `r<轮>·b<块>·s<seq>` | 0 | **27** | 0 | **27** | ③⑮ 定位符形态（三生成点＋工具描述）**进件** |
| `…(+` | 0 | **1** | 0 | **1** | ① 徽章折叠标记**进件** |
| `条暂存` | 0 | **1** | 0 | **1** | ① 未挂足提醒的短告知**进件** |
| `[RUN_END` ／ `[/RUN_END]` | 0／0 | **1／1** | 0／0 | **1／1** | ⑯ 结束声明前缀／闭合**进件**（见下注） |
| `await_channel` | 0 | **5** | 0 | **5** | ⑯ 暂停通道标注**进件** |
| `unrecognized` | 27 | **32** | 55 | **60** | ⑯ 未识别声明原样受理（+5）**进件** |
| `lines truncated; narrow the pattern` | 0 | **1** | 0 | **1** | ⑪ grep 命中预算尾注**进件** |
| `Text files only: when true, return a section outline` | 0 | **1** | 0 | **1** | ⑫ `outline` 工具描述**进件** |
| `No section headings found in this ` | 0 | **1** | 0 | **1** | ⑫ 无标题如实告知**进件** |
| `more headings omitted` | 0 | **1** | 0 | **1** | ⑫ 标题省略告知**进件** |

**退役面**：`任务无需中止` **2 → 0**（⑯ 子项「替模型下判断的文案」删除；双侧一致）。

**保持面（零回退）**：`RLI提醒:` 1／`持续越线` 1／`域迁移确认` 1／`掩盖缺口` 1／`CoverageGap` 1／
`lif.domain_migration` 7／`分通道明细 selector=channels` 1／`符号: u=水平 v=速率 pred=闭式前推` 1／
`本会话负担总值 ` 3／`资源软提示` 2／`utf-8-lossy` 1。

> **注（字节级方法的第三条已知边界，本批新增）**：**`[RUN_END]` 全形在 .rodata 零命中**——
> 源码为 `MODEL_STOP_PREFIX = "[RUN_END"` ＋ 运行时 `format!` 接 `]`（`model_stop_syntax_line`），
> 两段分列 ⇒ 连续字节扫描不可见。核证按**前缀＋闭合两件**（各 0→1）。
> 与 [`070 §6 注①`](070_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-22.md)「短常量立即数装配／
> 仅测试可达」同族 ⇒ **字节级扫描是进件核证的下界，不是唯一判据**（已登记 BACKLOG 0bh 节）。

## 7. 换装、ACAF 与装配复核

- Windows（`D:\tb-eval\orz-windows`）：三件逐件 SHA256 与构建产物 **MATCH=True**；旧件留
  **`.0.6.10-bak`**；旧 manifest 留 `signer-manifest.json.bak-20260923-071`。
- ACAF 重 provision（root＝`D:\tb-eval\orz-windows\acaf`，**exit 0**）：keystore **逐位未动**
  （外层 `installation-key.dpapi` `2408F596…`／`installation-key.json` `C491FC95…`；
  `keystore/` 内两件 `F37556AB…`／`2AA80CB8…`——**前后四值全一致**）；
  回显 `binary_sha256=20f8b0d3…` ↔ 换装位 `orz-signer.exe` **逐位一致**（manifest 169 B 更新）。
- 冒烟（换装位）：provision usage **exit 1**；signer fatal（manifest 缺失）**exit 1**。
- 宿主装配复核（**PS 5.1 真机**）：`powershell -File scripts\dogfood_launch.ps1
  -TaskFile .tmp-0bi-task.txt -DryRun` 断言全过（cwd＝`D:\CLI`、载体哈希回显 `75EAA6E26A2C…`、
  ACAF fail-closed=True、`PROTOC`／`GROK_HOME` 就位、`rli=on（缺省常开）`）；
  **题面写入端同批实证**：以**无 BOM** 题面（289 B）投喂 ⇒ 启动器**原地重写为带 BOM**、
  回显 `113 字符・utf-8 BOM（启动器写入端已补）・读取=显式 UTF-8`（0bg ⑥ 修复在重建批首验）。
- **入口脚本解析面**：三脚本（`build_orz`／`run_orz_tests`／`dogfood_launch`）本轮实跑无解析错误
  （070 §8 ② 的 BOM 回归未复现）。

### 7.1 独立复跑（release，串行档）

| crate | 修前 | 修后 | 判读 |
|---|---|---|---|
| `orz-assurance --lib` | 274 passed／**1 failed** | **275 passed／0 failed** | 0ao 机械扫描红（§8 摩擦 1）已消 |
| `orz-loop --lib` | 835 passed／**3 failed**／3 ignored | **838 passed／0 failed**／3 ignored | ⑯／④ 的测试面断言已同步（§8 摩擦 2） |

（命令：`run_orz_tests.ps1 test --release -p orz-assurance -p orz-loop --lib -- --test-threads=1`；
**并行档与串行档同红**，故三红非并发竞态——与 0bh ⑤ 的两条竞态家族不同族。）

## 8. 本轮缺口、摩擦与处置

1. **【本批抓出并当场修复】0ao 机械扫描红：0bh ⑭ guide 文案落了工具名字面**
   （`orz-loop/src/controller.rs:1903` 的 guide 正文含 `blackboard_write`，非注释、非测试、
   未在 `LITERAL_EXEMPTS` 具名登记）⇒ `tool_names::tests::…` 报红。
   **处置**：按仓内既有形态（其余生产文案一律 `{BLACKBOARD_WRITE_TOOL_NAME}` 插值）把
   guide 正文由 `const` 改为 `board_guide_body()`（`format!` 插值常量），
   **`LITERAL_EXEMPTS` 维持空表**（不新增豁免，保 0ao 的单源口径）；
   **文案逐字等价经对拍实证**（690 B 逐字节一致）⇒ **digest 不变**。
2. **【本批抓出并当场修复】orz-loop lib 三红：0bh ④／⑯ 改行为后旧断言未同步**
   ——`a_single_round_surge_injects_only_the_highest_tier`（梯已缩为两个软档，断言仍按四档
   校验水位面与事件数）／`t1_truncation_moves_blocks_out_of_the_model_face_and_keeps_the_local_face`
   与 `model_face_guard_forces_truncation_above_the_safety_line`（⑯ 子项删掉「任务无需中止」句后，
   旧断言仍要求该句存在，与新加的「**不得出现**」钉（`context_scale.rs`）互相矛盾）。
   **处置**：断言随新行为同步——水位面／事件面改两个软档；两处告知块判据改**状况陈述**
   （`工作现场`＋`逐字未动`／`上限守卫`＋`逐字未动`），与 ⑯「只留事实」一致。
3. **【方法边界，新增一例】** `[RUN_END]` 全形字节级零命中（见 §6 注）。
4. **【观察】重建批起始时父仓带前批未提交改动**（索引 v4.31 头行 ＋ BACKLOG 条件触发节）：
   本批**在其上叠加** v4.32；两批改动同留工作树，提交面留待用户令。记此以备提交批对账。
5. **【读数】`cargo clean` 规模 41,799 文件／25.6 GiB**（068–070 同级读数：28.6k 文件／14.3 GiB）：
   target 目录体量随批次增长，重建前卷余量复核仍宽（构建全程 exit 0、无 ENOSPC／无 OOM）。
6. **【教训，建议并入 0bi ④ 报告自核】** 0bh 轮的**验证面**只跑过滤档
   （`cargo check --tests` ＋ 若干具名过滤器），故上述 1／2 两条**漏网**；
   重建批的**全量串行复跑**才把它们抓出。⇒ 落码轮宜以
   **`-p orz-assurance -p orz-loop --lib -- --test-threads=1` 全量档**为红判据入口（已入 0bi ④）。

### 8.1 修复提交（**行为零变化；但含一处生产源码形态面改动**）

- orz **`a46c7c02`**（`fix(0bh 落码面同步)`）：仅 `crates/orz-loop/src/controller.rs` 与
  `crates/orz-loop/src/agent_loop.rs` 两文件、`+31/−28`。**两面性质不同，须分开记**：
  1. **`agent_loop.rs`＝测试面**（四处 hunk 的上下文均为 `mod tests`）——断言随 ④／⑯ 新行为同步，
     不参与产物。
  2. **`controller.rs`＝生产源码形态面**（`impl AgentLoopController` 内）——guide 正文由
     `const` 改为按 `{BLACKBOARD_WRITE_TOOL_NAME}` 插值（保 0ao 单源口径）。
     **模型面输出逐字等价**（690 B 对拍一致，digest 不变）⇒ **行为零变化**；
     但**生产源码确已变动**：若由新 HEAD 重出载体，**二进制与现役件非同字节**（行为等价，
     rodata 里那句改由「常量＋`{}`」两段承担）。
- **与载体的关系（口径明示）**：**现役六件由冻结线 `917fadfb` 构建**，本档 §4／§5 的六件哈希与
  §6 字面量核证**对现役件成立**、换装位即该二进制；`a46c7c02` 是**源码面**跟进，
  不回溯改写已换装的产物。下次重建（0bi 前置）冻结线取新 HEAD（届时 §6 的
  `blackboard_write` 相关字节形态按新源复核）。
- 单独判据：`git diff 917fadfb..a46c7c02 --stat` ＝ 两文件 31 插入 28 删除；
  逐行审查＝`agent_loop.rs` 全在 `mod tests`、`controller.rs` 只在 guide 正文与
  `render_board_guide`（可复核）。

## 9. 记账面（pin、清单、索引、计数）

- orz：`917fadfb`（bump，**本地未推送**）→ **`a46c7c02`**（§8.1 修复，**本地未推送**）。
- 父仓：本档 ＋ 子模块 pin ＋ `orz_source_manifest.sha256` 重算 **1464 条**
  （差异恰 **2 行**＝§8.1 两文件）＋ TODO／BACKLOG 的 **0bh「进载体」标注**（十三件进件、
  退役面、保持面、方法边界）＋ 索引 v4.31 → **v4.32**；门禁 `valid: true`（error_count 0）。
- **计数不变**（**未闭合 47**；本批两侧修复均在批内闭合，不新增未闭合项）。

## 10. 本批未做（边界）

1. **不打包、不发行**：0.6.7 不补发（既有裁决）；0.6.11 同属 RLI 中间过渡版，
   **是否发行待用户裁决**（若发行沿用 `.tmp-b066-package.ps1` 形态）。
2. **未推送**：orz 两提交（bump ＋ 修复）与父仓本批改动均留本地（用户令只含重建）。
3. **未跑真机狗粮轮**：0bh 新面（提醒投递预算／拆树告知／定位符／guide／结束自述）的
   **首次真机读数**属 0bi S3；本批只做到「面进件＋字面量可核＋宿主装配可核」。
   本批已把 **0bi S3 的前置（载体带新面）** 备齐。
4. **未提交／未推送父仓**：含前批 v4.31 未提交改动（§8 摩擦 4），提交批由用户令发起。

## 11. 证据留档（`.tmp-*` 门禁豁免）

`.tmp-b071-windows-build.log`／`.tmp-b071-linux-build.log`／`.tmp-b071-linux-apt-pre.log`／
`.tmp-b071-literals.py`／`.tmp-b071-release-tests.log`（首轮）／`.tmp-b071-release-tests2.log`（并行档）／
`.tmp-b071-release-tests3.log`（orz-loop 串行）／`.tmp-b071-release-tests4.log`（修后串行）／
`.tmp-b071-guide-eq.rs`（文案逐字对拍件）／`.tmp-b071-manifest-pre.sha256`／`.tmp-b071-manifest-pre2.sha256`／
`.tmp-orz-commit-msg-bump071.txt`／`.tmp-orz-commit-msg-fix071.txt`；
两平台 `.0.6.10-bak` 备份链与 `signer-manifest.json.bak-20260923-071` 保留。

## 12. 关联

[`070 重建档`](070_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-22.md) ／
[`0bh 报告`](0BH_INTERFACE_CLOSEOUT_R2_2026-09-22.md) ／
[`0bi 立项（TODO）`](../../TODO.md) ／ [`BACKLOG`](../BACKLOG_AND_PRIORITIES.md) ／
[`索引`](../../CLI_PROJECT_INDEX.md) ／ [`Docker 代理配方`](../../scripts/DOCKER_PROXY_RECIPE.md)。
关键词：0.6.11、双平台、载体重建、中间过渡版、免预检、clean 全量、暖缓存、ACAF 重 provision、
Linux musl static-pie、字面量核证、0bh 十三件进件、独立复跑、0ao 扫描红、测试面断言同步、
`[RUN_END]` 方法边界、未推送。
