# 217 批：0cv S1–S4——DeepSeek 接口名对齐执行（legacy `deepseek-v4-flash` → 现行官方 `deepseek-flash`）（2026-10-07）

> **用户令**：「请先进行0cv吧」（＝208 批边界「改名执行随用户放行（S2 起）」的放行）。
> **性质**：落码＋探针批—— assurance 落码（父仓）＋ orz 落码（子仓 `af6fa9a0`，不推送）＋
> SCBench 工具面（工作区）＋真机探针三步。**计数不变（54）**；闭合待用户裁决。
> **结论先行**：S1–S4 一次批内全达成。**S1 冻结期勘误 208 批盘点 ④——orz 生产面并非
> 零硬编码**：`gateway/transport.rs` `MAIN_AGENT_MODEL` 即生产主模型单一真源、
> `orz-host/tools.rs` `web_search_config` 回退默认为第二处，两处均随本批改至现行名；
> S3 真机读数＝`/models` **恰两枚现行名**（legacy 名已不在发现列表）＋新名最小补全
> HTTP 200 回显 `deepseek-flash` ＋ SCBench 驱动 dry-run 输出树新键解析成立。
> **在役载体 0.8.14 仍发 legacy 名（仍收单、同后端），进体随 0ct/0cu S3 同一重建批（0.8.15）**。

---

## §1 S1 改面清单冻结（含 208 ④ 勘误）

**provider 路径 pin（勘定）**：TB 直连 `-m deepseek-flash`（orz env `ORZ_MAIN_AGENT_MODEL` 同形）；
SCBench 前缀形态 **`deepseek/deepseek-flash`**（provider slug `deepseek`＋ModelCatalog 注册名，
注册名＝config 文件名＝outputs 树键）；orz 生产默认（无 env 时）＝`MAIN_AGENT_MODEL`。

**改面冻结清单（A＝本批改；B＝随下次使用批改；C＝dated 不改；D＝零改动确认）**：

- **A1 assurance（父仓）**：`adapter_preflight.py` legacy 建议臂＋GAK-07 消息勘正＋receipt
  `warnings[]`；schema 增可选 `warnings`／checks 增可选 `model_id_not_legacy_alias`；
  `test_adapter_integration.py` 清单换新名＋钉 6。
- **A2 orz（子仓，208 ④ 勘误后新进改面）**：`transport.rs:69 MAIN_AGENT_MODEL`（生产主模型
  单一真源，env 覆盖不动）／`orz-host/tools.rs:35 ORZ_WEB_SEARCH_MODEL` 回退默认＋:717 默认钉／
  `probe_thinking_max.rs:40` 探针常量（live 探针非生产）／`orz-host/lib.rs:3153` live 测试字面。
- **A3 SCBench 工具面**：新增 `configs/models/deepseek-flash.yaml`（alias
  `deepseek/deepseek-flash`、agent_specific.orz.model_name 新名、注释带 legacy 路由注记；
  legacy yaml 保留供 dated 轮复现）；`0cr_official/scan_0cr.py` OUT_ROOT 改 `SCB_OUT_ROOT`
  env 可覆盖（默认保持 0cr legacy 树，dated 轮复跑不破）＋docstring 注新名轮取值。
- **B（沿 208 口径不回改）**：TB 驱动 6 件（`D:/tb-eval/`，TB 2.1 线收官）；父仓 `scripts/*.py`
  9 件 dated run 驱动；未来 0ci 驱动（尚不存在）。
- **C（dated 不改）**：`_s4_dna_2026-08-26_config.json`／threshold-calibration json／
  `architecture/DEEPSEEK_ADAPTER_CONTRACT_v0.1.md`（pre-ADR-0010 冻结 fixture）／ADR 卷／
  审计档／TB21 报告等全部台账与设计档。
- **D（零改动确认）**：orz-tools `client.rs` mock fixture 三处与 `transport.rs` 测试构造字面
  （测传参忠实性、不测默认值）；物理 provider 配置与 key（`D:/tb-eval/.env`）。

**208 ④ 勘误记录**：208 批「orz 侧生产零硬编码＝零改动面」系误断——当时盘点仅勘到 web_search
client 的测试夹具，漏勘 `MAIN_AGENT_MODEL`（生产 `--real` 网关与 live 测试共享的单一真源）与
`web_search_config` 回退默认两处生产面；本批据实勘正并落码。由此 208 §5「本项不触发载体重建」
的依据同步失效——**进体需随下一重建批**（与 0ct/0cu S3「随下一重建批」自然合流，一次 0.8.15
重建进体三件）。

## §2 S2 落码

### §2.1 assurance（父仓）

- **GAK-10 legacy 建议臂**（`adapter_preflight.py`）：`_LEGACY_DEEPSEEK_ALIASES`
  （`deepseek-v4-flash`→`deepseek-flash`、`deepseek-v4-flash-vision-exp`→`deepseek-flash`，
  208 §1 视觉注记同源）——**建议不拦**（官方仍收单，硬拦将误伤合法在役用法）：preflight 通过、
  receipt 增 `warnings[]` 建议行、checks 增 `model_id_not_legacy_alias=False`。
- **GAK-07 消息勘正**：退役别名错误消息「use deepseek-v4-pro or deepseek-v4-flash instead」→
  「…or **deepseek-flash** instead」（被建议对象不再指向 legacy 名）。
- **schema**（`adapter-preflight-receipt-v0.1.schema.json`）：properties +`warnings`（可选）、
  checks.properties +`model_id_not_legacy_alias`（可选）——两者均不入 required，历史 receipts
  零回归；schema 无指纹台账、运行时加载，无连带面。
- **钉 6（先红后绿）**：legacy 名警告臂／vision 变体警告臂／新名零警告／legacy 大小写变体／
  退役消息建议新名／current-models 清单 v4-flash→deepseek-flash。**先红实读＝5 failed 1 passed**
  （vision 臂首跑因未传闸门参数误红，补参后重跑仍 5 failed＝正确红态）→ 落码后 **37/37 全绿**。

### §2.2 orz（子仓 `af6fa9a0`，4 文件 +12/−5，不推送）

`MAIN_AGENT_MODEL`／`web_search_config` 回退＋默认钉／探针常量／live 测试字面四处（明细见 §1 A2
与 orz 批提交消息）。

**验证**：
- `cargo test -p orz-loop --lib`：**858 过／1 挂**（1＝`user_cancel_closes…` 在案 30ms 竞速
  偶发，193 批同款）。
- `cargo test -p orz-host --lib`：失败族经 **stash 冻结树对拍**＝先前既有环境敏感面（冻结树
  7 失败、带改 5–6 逐轮漂移、失败集合为 timeout／进程树／symlink／scrub 族＝0aq RS 在案的
  orz-host 负载敏感记录；**本改零新增失败**）。
- **clippy 对拍**：`-p orz-host -p orz-loop --all-targets` 告警行集 stash 对拍 **41=41 恒等**
  （零新增）。
- **fmt**：触碰 hunks 净（`transport.rs`／`probe_thinking_max.rs`／`lib.rs` 全文件净；
  `tools.rs` 仅 :620 一处仓内既有漂移、非本批 hunks，全仓 fmt 漂移面为在库既有态，未触碰）。

### §2.3 SCBench 工具面（`D:/tb-eval/scbench/`，工作区件随冻结线）

新增 `deepseek-flash.yaml`＋`scan_0cr.py` env 可覆盖（明细见 §1 A3）。

## §3 S3 探针验证（真机三步，key 不落档）

| 步 | 读数 |
|---|---|
| `/models` 发现面 | **恰两枚**：`['deepseek-flash', 'deepseek-v4-pro']`；`deepseek-v4-flash` **已不在发现列表**（pricing 页口径「仍收单」仅指补全端点接受；发现面已只认现行名——GAK-09 探针对 legacy 名将返回 not-found，与本臂语义互补） |
| 最小补全 ping | `deepseek-flash`＋`"ping"`＋max_tokens 5 → **HTTP 200**、`model` 回显 **`deepseek-flash`**、usage 正常（completion 5＝reasoning 5，5-token 预算下 content 空属 thinking 形态；36 tokens 全额 miss 计费，成本可忽略） |
| SCBench 驱动 dry-run | harness 自身 `parse_model_override`：`deepseek/deepseek-flash` → registered name＝internal_name＝**`deepseek-flash`**、provider `deepseek` ⇒ outputs 树键 **`outputs/deepseek-flash`**；legacy `deepseek/deepseek-v4-flash` 解析保留（dated 轮复现不破） |

## §4 S4 注记面

- **注记条款（生效）**：后续报告与台账模型名一律 `deepseek-flash`；涉 legacy 读数处附「legacy
  别名仍收单、底模同代（V4.1-Flash）、同价，读数可比」路由注记。
- **207 报告核态（不动）**：§2 模型行已带 208 批注记（「现行官方接口名 `deepseek-flash`，改名随
  0cv」）——「改名随 0cv」在本批后仍为真陈述，定档读数不再触碰；§6 提交路径更正 208 已落。
- **在役面如实注记**：载体 0.8.14 的生产默认仍发 legacy 名（补全端点仍收单、同后端承接），
  **0.8.15 重建批进体后载体才发新名**——期间源码面与载体面接口名不同步，与 0ct/0cu 在途态同形。

## §5 台账

- 本档：`docs/audits/217_0CV_S1_S4_DEEPSEEK_IFACE_ALIGN_EXEC_2026-10-07.md`。
- BACKLOG：0cv 节批序四步勾达成＋边界更新（放行落地＋进体随重建批）＋入口补本档；计数行本批
  指针（**计数不变 54**）；优先级总览表 P1 行 0cv 片段；P1「开放项：」锚点行 0cv 片段。
- BACKLOG 第二卷：§1.164。
- TODO：头行（本批＝217）＋P1 路由行 0cv 片段＋`P1-0cv` 节 S1–S4 勾选。
- 索引：头行 v4.188 → **v4.189**（本批＝217）＋§8 `pending` 桶 DEEPSEEK-MODEL-IFACE-ALIGN 条目
  （S1–S4 达成〔217〕、进体随重建批、闭合待裁决）。
- orz 子仓：`af6fa9a0` 批提交（4 文件 +12/−5；**不推送**）。
- 源清单：`orz_source_manifest.sha256` 随批再生成＝**1,486 条、差异恰 4 行**（触碰四文件）。
- 门禁：`python -m scripts.check_repository` ⇒ **`valid: true`**（终读零 error；过程中两处中间态
  已随批收口＝ledger 一致性「TODO 全勾但 BACKLOG 仍记开放：0cv」→ TODO P1-0cv 补未勾「闭合」
  裁决门框〔沿 0cb 先例〕；orz source digest mismatch ×4 → 源清单随批再生成）。

## §6 边界

1. **载体未重建**：在役 0.8.14 仍发 legacy 名；进体＝0ct/0cu/0cv 三件合流的一次 0.8.15 重建批
  （随用户令放行）。零契约面（journal/schema/verifier 零改动）。
2. **TB 驱动 6 件与 `scripts/*.py` 9 件未回改**（随下次使用批改；dated 不改沿 208 口径）。
3. 0cv **闭合待用户裁决**（S1–S4 已达成；计数不变 54）。
4. 探针读数以 2026-10-07 实测为准（`/models` 发现面与 pricing 页「仍收单」并存的形态若再变，
   随批勘定 GAK-10 臂口径）。
5. 父仓全量 pytest **2454 过／9 失败**＝存量：8 项 `.tmp-0bvs4-work`（先前批 vendored 副本收集
  噪声）＋1 项 `test_doctor_full_repository_check`（`orz submodule working tree is dirty`＝
  未提交落码预期态，216 批档 §4 同款）；非本批引入。orz-host 失败族 stash 对拍零新增（§2.2）。

## §7 关键词

217 批、0cv、DEEPSEEK-MODEL-IFACE-ALIGN、deepseek-flash 现行名、deepseek-v4-flash legacy、
GAK-10 legacy 建议臂、warnings 字段、model_id_not_legacy_alias、MAIN_AGENT_MODEL、
ORZ_WEB_SEARCH_MODEL、208 ④ 勘误、SCB_OUT_ROOT、outputs/deepseek-flash、/models 恰两枚、
0.8.15 重建合流、af6fa9a0、计数不变 54。
