# 0.6.10 双平台载体重建与换装（2026-09-22）

> 用户令：「请进行重建吧，同时请再确定一下该进0bh中的内容是不是都进去了」。本批三段：
> ① **版本 bump**（0.6.9 → 0.6.10）② **双平台重建** ③ **换装＋ACAF 复核＋进件核证**；
> 另附 **§9 0bh 完整性核对**（用户令第二问）。**打包与发行不在本批范围**
> （0.6.7 不补发、0.6.8／0.6.9 中间过渡版不发行的既有裁决沿用；0.6.10 同属中间过渡版，
> **待用户裁决是否发行**）。
> **源冻结线**：orz **`f36dee7c`**（`feat/fusion-architecture`）＝ 0bg 交互面与 RLI 收尾轮
> `7da1a9fe`（八件）＋ bump 两文件两行。上一载体：双平台 0.6.9（`88b6dea1` 冻结，不发行）。
> 本批**免预检**（默认口径沿用 0be §8.1 收窄结论）。

## 1. 前置基线（0.6.9 在役件，换装前实取）

| 平台 | 文件 | 尺寸 (B) | SHA256 |
|---|---|---:|---|
| Windows | `orz.exe` | 53,603,328 | `0C4598B0515D033C1155FF0CD9B3B2CD15CED8F5261178ACFD54FB79AE39C110` |
| Windows | `orz-signer.exe` | 6,729,728 | `76D8D0F4770A844BD5B47A91D18542EDDE6D3719899190281A81690CE5665434` |
| Windows | `orz-acaf-provision.exe` | 6,639,104 | `BE622ACAEBD8633C2BDB28AA9CE9F6038B0EE40DB42F8BB0766E7ABBF8881EC9` |
| Linux | `orz` | 112,158,136 | `d17a607f8a67f12559186bbf4dd92747566c46004cde6e875017994d90c15589` |
| Linux | `orz-signer` | 1,401,320 | `6626f6c83fe2bdc868669811246bf84b062cbc934c20eaedeb9085d679d37864` |
| Linux | `orz-acaf-provision` | 1,220,328 | `8544e8c9f63ed4ff94661c33db7fd1d8f72cc771ce4c3d0daef2c27da7dfbdf4` |

六件与 [`069 档 §4/§5`](069_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-22.md) 记录**逐位一致**
（在役件未被外力改动）。换装前逐件复制为备份链 **`.0.6.9-bak`**（Windows
`D:\tb-eval\orz-windows\`；Linux `D:\tb-eval\orz-linux\`）——备份件与该表**逐位一致**
（逐件 `backup_ok=True`）。

## 2. 版本 bump 与源冻结

- `crates/orz-bin/Cargo.toml` ＋ `Cargo.lock` **两文件两行**：`0.6.9 → 0.6.10`
  （orz 提交 **`f36dee7c`**；**本轮未推送**——用户令只含重建，推送留后续批）。
- 版本号只在显式 bump 时递增；本批源冻结线＝`f36dee7c`；**0bg 八件首次进入在役载体**
  （对拍空洞 9 值修复／CoverageGap／注解随报＋符号表／双迁移撤徽章转机械层连带记录／
  now 面重排＋`selector=channels`／horizon 标定／宿主续跑显式覆盖／dead_code 清零）。
- 父仓侧同时带入 **0.6.10 前置修复**（§8 ① ②）：`scripts/build_orz.ps1` 补 `PROTOC` 前置、
  `scripts/run_orz_tests.ps1` 同形补入、`scripts/dogfood_launch.ps1`／`run_orz_tests.ps1`
  恢复 UTF-8 BOM。**脚本不进载体**（不改二进制），故 §6 核证不受影响。

## 3. 预检：本批按默认口径免除

- 用户 2026-09-22 口径：**默认不预检**，仅当出现只能在另一平台验证的条件编译面、
  或用户明确点名时才做。本批为 RLI 逻辑面＋测试面（＋ bump），**不做**双平台预检。

## 4. Windows 三件套（宿主 release，clean 全量）

- 构建入口＝`scripts/build_orz.ps1 -Release -Clean`（auto 档自检读数 **12 核／物理 15.8 GiB
  （余 5.4）／commit 26.3 GiB（余 4.8，使用 81.8%）** ⇒ 判 **`-j 2`**）。
- `cargo clean`：**28,585 文件／14.3 GiB**。
- **首次尝试 exit 101（§8 ①）**：依赖阶段 `orz-tools-api` build.rs 报 `protoc not found`
  ——约 **5m40s** 后中断（日志 `.tmp-b070-windows-build-abort.log`）。补 `PROTOC` 前置后
  **续跑 exit 0，21m18s**（日志 `.tmp-b070-windows-build2.log`）；**警告 0 条**。

| 文件 | 尺寸 (B) | 与 0.6.9 差异 | SHA256（换装位＝构建产物，MATCH=True） |
|---|---:|---:|---|
| `orz.exe` | 54,653,952 | +1,050,624 B | `8F67381FEF2834142DB5E8F952374695042C4BDE5D7A43125FCD7477CAB8F0F0` |
| `orz-signer.exe` | 6,742,528 | +12,800 B | `75E8C98BAE61C036FB2D6A5E90830ED74C1227498EF65FFF4D5D7F96542E8670` |
| `orz-acaf-provision.exe` | 6,642,176 | +3,072 B | `791229E4B7DF37016E566AF829A5AB24AE4EF329514248E1DA9AF2C06AEEF2AD` |

> `orz.exe` +1.0 MB 与 0bg 进件量级相称（新面文案＋9 值分支＋折叠面）；`signer`／`provision`
> **无源码改动**（版本串 `0.6.9→0.6.10` 为唯一差异来源），其尺寸/哈希变动属重编译非确定性
> ——与 068／069「同码不同哈希」同族，非回退。

## 5. Linux musl 三件套（Docker，直写换装位）

- **实包预检**：`apt-get update -qq && apt-get install -y -qq musl-tools` → **APT_EXIT=0**
  ⇒ 按 [`Docker 代理配方`](../../scripts/DOCKER_PROXY_RECIPE.md) §0 **不切代理、不重启**
  （日志 `.tmp-b070-linux-apt-pre.log`）。
- 构建：`rust:1.97-slim` ＋ `bash /orz/scripts/build_orz_aliyun_trixie.sh`（ORZ-BUILD-MOUNT-001
  契约；脚本与仓库件同源），挂载 `-v D:/CLI:/orz`（父仓原地）＋`-v D:/tb-eval/orz-target:/target`
  （暖缓存）＋`-v D:/tb-eval/orz-linux:/out`（**直写换装位**，换装前已留 `.0.6.9-bak`）；
  **exit 0**：容器内 `Finished release` **8m34s**、含工具链与静态 `rg` 安装全流程 **10m30s**
  （日志 `.tmp-b070-linux-build.log`）。
- **容器未在源树留痕**：构建后 orz 仓 `git status` 为空。

| 文件 | 尺寸 (B) | 与 0.6.9 差异 | SHA256 |
|---|---:|---:|---|
| `orz` | 112,186,200 | +28,064 B | `91be5f9e7654eb786089aeb844ef908db486431ea85dc5b48423740162e8c7f4` |
| `orz-signer` | 1,401,800 | +480 B | `28e1baffa90ef4fd4e8d9ec4987f400b00d2683ee058f934b72bfb2b7fafd319` |
| `orz-acaf-provision` | 1,220,536 | +208 B | `7ba681ab802718d9b25bc5bfa78a4cd4189a1ac1ec18fabf275a2d60ef6dff6b` |

- ELF 静态核验（逐件）：三件均 `e_type=3`（ET_DYN static-pie）＋ `e_machine=62`（x86-64）
  ＋ **PT_INTERP=0**。
- 双向加载冒烟：`debian:bookworm-slim` 与 `alpine:3.20` 两侧、三件均 **exit 1**
  （orz 无 TTY `tui io error`／signer manifest 缺失／provision usage——与 052–069 同形态），
  **6/6**。
- **警告面（平台条件面基线）**：`orz-config` 1／`orz-assurance` 4／`orz-host` 2
  **与 0.6.9 同数**；`xai-tty-utils` **0**（0bd ④a 的在 Linux 侧收益保持）。

## 6. 字面量核证（双方二进制，字节级出现次数；对照列＝在役 0.6.9 备份件）

| 字面量 | Win 0.6.9 | Win 0.6.10 | Lin 0.6.9 | Lin 0.6.10 | 判读 |
|---|---:|---:|---:|---:|---|
| `符号: u=水平 v=速率 pred=闭式前推` | 0 | **1** | 0 | **1** | 0bg ③/§3.2 面头符号表**进件** |
| `〔g=未访域占比；r=驻留÷该域已完段中位；域失配=累计超期；仅读数非阻断〕` | 0 | **1** | 0 | **1** | §3.2 掩盖缺口注解**进件** |
| `掩盖缺口` | 0 | **1** | 0 | **1** | §3.1 第三 kind 文案**进件** |
| `；域失配 ` | 0 | **1** | 0 | **1** | 随报域失配字段**进件** |
| `CoverageGap` | 0 | **1** | 0 | **1** | §3.1 kind 标识**进件** |
| `coverage_gap_armed` | 0 | **2** | 0 | **2** | §3.1 触发沿＋重武装**进件** |
| `分通道明细 selector=channels` | 0 | **1** | 0 | **1** | ③ 省略行＋折叠面**进件** |
| `rli.channels → ` | 0 | **1** | 0 | **1** | ③ 折叠面头**进件** |
| `lif.domain_migration` | 0 | **7** | 0 | **7** | §3.3 机械层连带记录 key**进件** |
| `域迁移+` | 1 | **0** | 1 | **0** | §3.3 模型面徽章**退役**（零残留） |
| `lif_domain` | 0 | 0 | 0 | 0 | **字节级假阴性**（见下注）⇒ 实为**进件** |
| `retrieval_batch`／`plan_write_guidance` | 7／7 | 7／7 | 7／7 | 7／7 | 词表面保持 |
| `RLI on`／`近提醒: `／`域迁移确认`／`持续越线` | 1／1／1／1 | 2／1／1／1 | 1／1／1／1 | 2／1／1／1 | 0bf 模型面保持（`RLI on` 计数 +1＝③ 折叠面头引用） |
| `本会话负担总值 `／`资源软提示`／`utf-8-lossy:`／`λ̂=` | 3／2／1／1 | 3／2／1／2 | 3／2／1／1 | 3／2／1／2 | 0bf／0bc／0as／0be 面保持 |

> **注（字节级方法的两条已知边界，本批实测坐实；均为方法面，非进件缺失）**：
> ① **短常量以立即数装配**⇒ 文件里不出现连续字节：`lif_domain`（10 B）在 Win／Lin 双侧
> 均为 `mov rcx,'lif_doma'` ＋ `mov word […+8],'in'`（上下文实测可见，两平台各 7 处），
> 故扫描计数 0 而**实为进件**；同类现象在 0.6.9 即有（`attention_ladder` 16 B 旧新皆 0）。
> ② **仅测试/CI 可达的 Rust 对拍镜像不落地二进制**：`families_s2c::verify_mechanical_audit`
> 的唯一非测试调用点在 `verify_family` 分派，而分派在工作区内**仅被 `mod tests` 调用**
> ⇒ 0bg ⑤ 的 9 值修复**不产生二进制字面量**（`tool_result / plan_gate` 旧新皆 0）。该面
> 的进件由**源码＋契约钉测试**保证（0bg §6：`mechanical_audit` 9 值断言两处）。
> 本批顺带坐实：**Rust 对拍镜像是测试面，Python 镜像才是门禁面**（gate 计
> `run_event_journal_validation: 1`）——与 0bg ⑤「真 journal 记假错」的作用面表述一致。

## 7. 换装、ACAF 与装配复核

- Windows（`D:\tb-eval\orz-windows`）：三件逐件 SHA256 与构建产物 **MATCH=True**；旧件留
  **`.0.6.9-bak`**（逐位等于 §1 基线）；旧 manifest 留 `signer-manifest.json.bak-20260922-070`。
- ACAF 重 provision（root＝`D:\tb-eval\orz-windows\acaf`，**exit 0**）：keystore **逐位未动**
  （外层 `installation-key.dpapi` `2408F596…`／`installation-key.json` `C491FC95…`；
  `keystore/` 内两件 `F37556AB…`／`2AA80CB8…`——**前后四值全一致**）；
  回显 `binary_sha256=75e8c98bae61c036fb2d6a5e90830ed74c1227498ef65fff4d5d7f96542e8670`
  ↔ 换装位 `orz-signer.exe` **逐位一致**（manifest 169 B 同步更新）。
- 冒烟（换装位）：provision usage **exit 1**；signer fatal（manifest 缺失）**exit 1**。
- 宿主装配复核（**PS 5.1 真机**）：`powershell -File scripts\dogfood_launch.ps1
  -TaskFile .tmp-0bg-task.txt -DryRun` 断言全过（cwd＝`D:\CLI`、载体哈希回显 `8F67381FEF28…`、
  ACAF fail-closed=True、`PROTOC`／`GROK_HOME` 就位、`rli=on（缺省常开）`、题面
  `.tmp-0bg-task.txt` 回显 **113 字符・utf-8 BOM・读取＝显式 UTF-8**）。版本列 `unknown`
  系 Rust 载体无 VersionInfo 的既有口径。
- **入口脚本功能性复核（PS 5.1 解析面）**：三脚本 `Parser::ParseFile` **0 错误**
  （修前 `dogfood_launch` 3／`run_orz_tests` 1／`build_orz` 0——见 §8 ②）；
  `build_orz.ps1 -Release -DryRun` 清单新增 `protoc = D:\tb-eval\.tools\protoc-25.3\bin\protoc.exe`。

## 8. 本轮摩擦与处置（重建面新增四条）

1. **构建/测试前置缺 `PROTOC`（两次命中，两处已修）**：缺键时 `orz-tools-api` 的 build.rs
   在**依赖阶段**即失败（exit 101），此前只有狗粮启动器装配该键。① 构建入口
   `build_orz.ps1` 首次命中（Windows 全量首轮中断）；② **同一缺口在测试入口
   `run_orz_tests.ps1` 第二次命中**（宿主串行测试首轮 457 s 后 exit 101）。处置＝两处同形
   补前置（存在即设入、**只回显不覆盖**图上位；缺件时清单显式报「未设」，不静默）。
   归属＝0bd ①「构建前置自检」家族（0aq RS-11 同族），与 0bh ⑥「构建争用面」的
   「构建前软提示」同格。
2. **069 的 BOM 修复在 0bg 轮被覆盖（回归，已恢复）**：`scripts/dogfood_launch.ps1` 与
   `scripts/run_orz_tests.ps1` 的 UTF-8 BOM 在 0bg 轮编辑时丢失 ⇒ PS 5.1 解析实测
   **3 条／1 条**错误（`build_orz.ps1` 保持 0）；本批恢复 BOM ⇒ 三脚本 **0 条**。
   **教训**：机械层给文件打的形态标记（BOM）必须由**同一条写入路径**维持——069 §8.1 修了
   「脚本自身」，0bg ⑥ 只修了「题面写入端」，两条线同族但覆盖面不同 ⇒ 0bh 起草时宜把
   「写入端强制 BOM」表述为**脚本＋题面两端**（本批已按此恢复）。
3. **auto 档对 `--help` 类参数会把 `-j` 插到无位置处**（观察，非本批引入）：
   `run_orz_tests.ps1 --help` ⇒ cargo 报 `unexpected argument '--help-j 2'`。仅登记
   （0bh ⑥ 家族），本批不改（真实使用路径为 `test`／`build` 子命令）。
4. **`cargo clean` 后 debug 档全量重编**（读数，非缺陷）：宿主测试首轮含全量 debug 依赖
   编译（457 s 至 protoc 失败点），修复后续跑 **543 s**（其中测试本体 **55 s**）。
   ⇒ 0bh ⑥「构建争用面」的补充读数（release 21m18s ＋ debug ~9m，同日两档）。

## 9. 0bh 完整性核对（用户令第二问）

**结论：0bh 已应收尽收（十五件），未发现漏项**；核对中查出**三处编排面缺口**，随本批补齐。

- **来源追平**：0bg 报告 §5 摩擦 a–l ⇒ a→③④、e→⑥、**g→②**（机理按 §9 勘误更正为
  Windows 调用级 Job 连坐，非「缓冲」）、h／i→「仅记录」、j／k／l→0bg 轮内已修（⑤/②/③
  的实施面）＋⑧ 承接核证；主会话两条回查（提醒投递 17/4、拆树定性）→①、②；
  用户同日各条裁决（掩盖缺口、连带记录、192/256 双档、门一/门二、三项候选、⑭⑮、
  「范围纪律」不入常驻）**逐条可定位**。`范围纪律` 在台账正文中 **0 处**（确认已删）。
- **补齐 ①**：`TODO.md` 的 P1-0bh **S1 勘定行原先只列 ①–⑥ 的落点**（⑦–⑮ 未列）⇒ 改为
  **十五件全覆盖**（含 ⑦ 自核纪律／⑧ 0bg 遗留承接／⑨–⑬ 减负候选／⑭⑮ 新功能设计）。
- **补齐 ②**：索引 §8 `pending` 桶**补 `FUS-BLACKBOARD-GUIDE`／`FUS-CONTEXT-POINTER`**
  （前批只进了索引条目列表，未进状态速查桶，与 0bg 的 `GAP-RLI-CLOSEOUT-FRICTION-BATCH`
  处理不一致）。
- **补齐 ③**：0bg 轮内**已处置**的四条执行摩擦（b serde 双函数陷阱／c `notices()` API 形态／
  d 累计序数公式／f 日期笔误 28 处）此前未进 0bh 三分类 ⇒ 补入「**已处置（不另立实施项）**」
  一行，避免它们在核对表里看起来被静默丢弃。
- **顺带**：0bg §5-g 的「框架本身的问题」问句，其处置即 ②（告知面＋门一＝A）——**不**新增
  与 ② 并行的独立项（避免同一机制两条实施线）。

## 10. 记账面（pin、清单、索引、计数）

- orz：`7da1a9fe`（0bg 批）→ **`f36dee7c`**（bump，**本地未推送**）。
- 父仓：本档 ＋ 子模块 pin ＋ `orz_source_manifest.sha256` 重算 **1463 条**（差异恰
  **2 行**＝bump 两文件）＋ 0bg「进载体」标注 ＋ 索引 v4.27 → **v4.28** ＋ §8 ① ② 两处前置修复
  ＋ §9 三处 0bh 补齐；门禁 `valid: true`（error_count 0）。
- **宿主全量串行测试（0bg ① 判据入口／0bh ⑧ 首项）随本批收取**：
  `run_orz_tests.ps1 test -p orz-host --lib -- --test-threads=1` ⇒
  **322 passed / 0 failed / 5 ignored**（exit 0；测试本体 55 s，含编译 543 s）
  ——与 0bd 记录串行基线 322/0/5 **逐项一致**，0bg ① 的续跑显式覆盖生效。

## 11. 本批未做（边界）

1. **不打包、不发行**：0.6.7 不补发（既有裁决）；0.6.10 同属 RLI 中间过渡版，
   **是否发行待用户裁决**（若发行沿用 `.tmp-b066-package.ps1` 形态）。
2. **未推送**：orz bump 与父仓本批改动均留本地（用户令只含重建）。
3. **未跑真机狗粮轮**：0bg 新面（CoverageGap／注解随报／`channels`／`lif_domain`）的**首次真机
   读数**属 0bh S3（本批只做到「面进件＋字面量可核」）。

## 12. 证据留档（`.tmp-*` 门禁豁免）

`.tmp-b070-windows-build-abort.log`／`.tmp-b070-windows-build2.log`／`.tmp-b070-linux-build.log`／
`.tmp-b070-linux-apt-pre.log`／`.tmp-b070-host-serial-abort.log`／`.tmp-b070-host-serial2.log`／
`.tmp-b070-literals.py`／`.tmp-b070-prep.py`／`.tmp-b070-prep2.py`／`.tmp-orz-commit-msg-bump070.txt`；
两平台 `.0.6.9-bak` 备份链与 `signer-manifest.json.bak-20260922-070` 保留。

## 13. 关联

[`069 重建档`](069_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-22.md) ／
[`0bg 报告`](0BG_RLI_FINALIZE_AND_FRICTION_2026-09-22.md) ／
[`0bh 设计档`](../BLACKBOARD_GUIDE_AND_POINTER_DESIGN_2026-09-22.md) ／
[`BACKLOG`](../BACKLOG_AND_PRIORITIES.md) ／ [`TODO`](../../TODO.md) ／
[`Docker 代理配方`](../../scripts/DOCKER_PROXY_RECIPE.md)。
关键词：0.6.10、双平台、载体重建、中间过渡版、免预检、clean 全量、暖缓存、ACAF 重 provision、
Linux musl static-pie、字面量核证、0bg 八件进件、PROTOC 前置、BOM 回归、宿主串行 322/0/5、
0bh 完整性核对、未推送。
