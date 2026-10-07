# 208 批：0cv 立项——DeepSeek 接口名对齐＋提交路径复核（2026-10-07）

> **用户令**：「再度纠正一个问题，目前仅仅只是接口名为deepseek-v4-flash，实际deepseek后端
> 自动路由至deepseek v4.1 flash。接口名称确实也应该改了，目前官方使用的应该是deepseek-flash吧？
> 请查证，并且对接口修改这一步专门立个小项吧」＋追问「官方有成绩提交路径吧？」。
> **性质**：立项批（查证落账＋登记）——零源码、零子仓改动；计数 **62 → 63**（新立项 0cv）；
> 未提交、未推送。**完成序顺延说明**：本批（208）完成晚于隔壁窗口并行批（209，0ct/0cu 全民
> 审查处置），编号顺延沿 205 批先例。
> **结论先行**：①官方 pricing 页核证＝现行接口名两枚 **`deepseek-flash`**（底模
> DeepSeek-V4.1-Flash）／`deepseek-v4-pro`；**`deepseek-v4-flash` 降为 legacy 别名仍收单**、
> 底层由 V4.1-Flash 承接（Flash 价位）——用户纠正与官方文档吻合，**历史读数同底模可比、改名
> 是账面清晰化非模型切换**；②改面盘点五点（预检 legacy 臂／TB 驱动 6 件／SCBench 发射与收取件
> ／orz 生产零硬编码确认／dated 工件不改）；③**提交路径复核**＝官方无文档化提交流程、
> **事实通道＝Harbor 公开 run**（207 报告 §2/§6 与 BACKLOG 0cr 三处同步纠正）。

---

## §1 S0 核证（官方 pricing 页，2026-10-07）

| 项 | 读数 |
|---|---|
| 现行接口名 | **`deepseek-flash`**（底模 DeepSeek-V4.1-Flash）＋`deepseek-v4-pro`（底模 DeepSeek-V4-Pro-0813） |
| `deepseek-v4-flash` | **legacy 别名，仍收单**；底层已退役、请求由 DeepSeek-V4.1-Flash 承接、按 Flash 价位计费 |
| 视觉注记 | `deepseek-v4-flash-vision-exp` 同为 legacy（视觉能力不再单列；现行 flash 模型支持 vision，FIM 仅非思考态） |
| 同源注记 | TB2.1 142 批报告 §2「2026-09-30 用户告知：已全面路由至 V4.1 Flash 代」与本核证吻合 |

**读数口径结论**：历史读数（TB2.1 82.0%／0cr 134/196 等）虽用 legacy 接口名，**底模同为
V4.1-Flash、同价位**——可比性不受影响；改名是账面清晰化，非模型切换。

## §2 改面盘点（S1 冻结用初版；五点）

1. **在役预检**：`assurance/adapter_preflight.py`（现仅拦 deepseek-chat/reasoner 退役名）
   补 `deepseek-v4-flash` legacy 臂→建议 `deepseek-flash`；`assurance/tests/test_adapter_integration.py`
   模型清单随钉。
2. **TB 驱动 6 件**（`D:/tb-eval/`）：`run_official_2.1.sh`／`run_official_21_k1_browser.sh`／
   `run_sweep_5.sh`／`run_sweep_5.ps1`／`run_pro_gpt2.sh`／`probe_deepseek.sh`——TB 2.1 线已
   收官，随下次使用批改；**不回改 dated 跑批日志**。
3. **SCBench 侧**：0cr 发射模板（196 批 §4 命令形态 `deepseek/deepseek-v4-flash`，前缀写法随
   provider 勘定）＋`0cr_official/scan_0cr.py` OUT_ROOT（**outputs 路径按模型名开键**——换名后
   输出树路径变，收取件随名）＋未来 0ci Frontier 驱动。
4. **orz 侧＝零改动面**：生产代码零硬编码（web_search client `deepseek-v4-flash` 仅测试夹具），
   208 批实测确认——**本项不触发载体重建**。
5. **dated 工件不改**：`_s4_dna_2026-08-26_config.json`／`evaluation/corpus-freeze/threshold-
   calibration-2026-09-13.json`／历史台账档。

**批序**：~~S0 查证＋改面盘点~~ **达成（本批）** → S1 改面清单冻结（provider 路径 pin：TB 直连
`-m` vs SCBench 前缀形态）→ S2 改名落码/脚本＋钉子（预检别名臂先红后绿；驱动随下次使用批）→
S3 探针验证（`deepseek-flash` 通路 ping＋SCBench 驱动 dry-run；新输出树路径 `outputs/deepseek-flash`
收取件同步）→ S4 注记面（后续报告新名＋legacy 路由注记条款）。执行随用户放行（S2 起）。

## §3 提交路径复核（同批；用户追问）

- **无文档化成绩提交流程**：README／FAQ／docs/evaluation（architecture/configuration/reporting/
  troubleshooting）／docs/metrics 六件／metrics-reference／CONTRIBUTING／scbench.ai 首页/
  leaderboard/contributing 页**全查无**提交流程，亦无试次数门槛表述——206 批「无公开通道」
  表述据此**收窄为「无文档化流程」**。
- **事实上的入榜通道＝Harbor 公开 run**：榜单页机制语「Showing best version by % Checkpoints.
  Select both Model and Harness to view all versions」＝按 Model×Harness 聚合公开 run；题目
  本体即以 Harbor dataset 分发；Harbor 有 `run --upload --public` 公开作业机制（TB2.1 官方轮
  即逐题公开上传先例）。**silverwhite 转换任务已发布（205 批）——后续若以 Harbor runtime 跑
  orz 并公开即构成榜单可见条目**；属新决策，随用户裁决。
- **同步纠正三处**：207 报告 §2（模型接口名注记）／§6（提交路径更正段）／BACKLOG 0cr 专节
  206-bullet（「无公开通道」→「无文档化流程＋事实通道」勘误注记）。

## §4 台账

- 本档：`docs/audits/208_0CV_DEEPSEEK_IFACE_ALIGN_2026-10-07.md`。
- BACKLOG：新增 `0cv` 专节＋计数行（**63**）＋指针行＋P1 总览行锚点；`0cr` 专节 206-bullet
  提交路径勘误注记。
- TODO：计数行（63、本批＝208、209 顺延注记）＋P1 路由行 0cv 片段（三处压缩腾帽至 1184）＋
  新增 `P1-0cv` 节（S0 勾选达成＋S1–S4 勾选项）。
- 索引：头行 v4.181 → **v4.182**（本批＝208＋完成序顺延说明）＋§8 `pending` 桶 0cv 条目
  （早前随 0cv 落入）。
- 第二卷：§1.157。
- 报告：`docs/SCB_V1_36_FULL_ROUND_REPORT_2026-10-07.md` §2／§6 两处纠正（本批属主）。
- 门禁 `check_repository.py` ⇒ **error_count 1＝`orz submodule working tree is dirty`**（204/209
  两批未提交落码预期态）；过程回缠三处＝BACKLOG P1「开放项：」锚点行补 `0cv` token＋压缩详注
  （节清单/优先级总览表/TODO 路由三面对齐，沿锚点语法）；台账行长终读：索引头行 733／BACKLOG
  指针行 1081／计数行 992／TODO 头行 537／P1 路由行 1184，均 ≤1200。

## §5 边界

1. **零源码、零子仓改动**；改名执行（S2 起）随用户放行；本项不触发载体重建。
2. **未提交、未推送**；208/209 两批与 196–207 各批同样在工作树待提交。
3. 207 报告定档读数不动（本批仅 §2/§6 注记与更正）；TB2.1 142 批报告不改（其模型行已有
   2026-09-30 路由注记，dated evidence）。
4. 官方接口名/榜单机制读数以 2026-10-07 公开页为准，后续如有变更随批勘定。

## §6 关键词

208 批、0cv 立项、DEEPSEEK-MODEL-IFACE-ALIGN、deepseek-flash 现行名、deepseek-v4-flash legacy
别名、路由 V4.1-Flash、读数同底模可比、账面清晰化非模型切换、改面五点、orz 生产零硬编码、
无文档化提交流程、Harbor 公开 run 事实通道、报告 §2/§6 纠正、计数 63、完成序顺延。
