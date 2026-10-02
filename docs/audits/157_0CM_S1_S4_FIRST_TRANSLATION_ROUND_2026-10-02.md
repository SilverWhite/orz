# 157 批：0cm S1–S4 第一轮翻译出稿（2026-10-02）

> **日期**：2026-10-02；**用户令**：「翻译对应稿就先这些，请开始第一轮翻译吧」。
> **形态**：主会话直接执行（LLM 翻译线本批即产出）；**未提交、未推送、未发行**；计数不变 60。
> **范围**：0cm charter §1 五件交付物（D1–D5）一次性出稿落盘；对应 charter 批序 S1–S4。

## §1 交付物清单（五件，全部落 `docs/en/`）

| # | 文件 | 源档与钉 | 形态 |
|---|---|---|---|
| D1 | [`docs/en/TRANSLATION_GLOSSARY.md`](../en/TRANSLATION_GLOSSARY.md) | charter 附录 A 种子（30 条）扩充至 52 行（含别名并写共 63 个中文名词条，按 `/` 切分计）＋文档映射表（EN↔CN 权威档＋版本钉） | `v0.1`——**用户增删定稿待续**；定稿前为工作术语权威 |
| D2 | [`docs/en/README.en.md`](../en/README.en.md) | 主仓 `/README.md`（v0.8.10 发布面；137 行，§6 删评测节后 131 行） | **全文翻译**（非蒸馏）；相对链接改指 `../../`；指向中文内部档处标注 Chinese；写控节加英文蒸馏件交叉链接 |
| D3 | [`docs/en/TB21_V41_89_FULL_ROUND_REPORT_EN.md`](../en/TB21_V41_89_FULL_ROUND_REPORT_EN.md) | [`TB21_V41_89_FULL_ROUND_REPORT_2026-10-01.md`](../TB21_V41_89_FULL_ROUND_REPORT_2026-10-01.md) | **全文翻译**；89 行成绩表逐格照抄（任务名/批/分数/用时/时限/哈希零改动），仅翻译表头与「形态」「备注」两列文字；其余 8 张表全部保留数值原样（全档共存 9 张表，复审勘正） |
| D4 | [`docs/en/WRITE_CONTROL_CURRENT_EN.md`](../en/WRITE_CONTROL_CURRENT_EN.md) | 写控 **v4.0**（现行规则面权威）＋ **v1.0**（v4 声明不变的架构） | **蒸馏当前态**：五条 block 规则命令逐字保留；沿革 5 版各一行；§7.6 否决备选留名防复活 |
| D5 | [`docs/en/ACAF_CURRENT_EN.md`](../en/ACAF_CURRENT_EN.md) | ACAF 设计档（2026-08-09 定稿）＋在役变更（0.8.2 签发装配修复、0.8.7 keystore faces、0.8.10 现状） | **蒸馏当前态**：威胁模型/密钥体系/票据结构/七项验票全录；三模式标注部署实况（仅 Normal 在役、无人审腿）；D-1…D-16 压缩一段 |

## §2 方法与质量口径落实（charter §2 逐条）

1. **对照表先行**：D1 先落盘再译（本批顺序即机制）；52 行 63 词条在 D2–D5 内复用零漂移
   （载体=carrier、灾难保底=catastrophic hard backstop、试次=attempt、擦墙通过=at-the-wall pass、
   自完判负=self-completed judged fail、翻盘=turnaround 等）。
2. **表格与数字程序化照抄**：D3 成绩表、D4 规则表中的命令/路径/哈希/百分比全部逐格复制，
   翻译不触数；D3 表头注记明示该纪律。
3. **声明前置**：五件档头统一 `LLM-assisted translation; the Chinese originals are
   authoritative. Corrections welcome.`＋源档链接＋版本钉。
4. **沿革压缩**：D4/D5 沿革块各压成一行一版（v1→v4 共 5 行；ACAF 5 行），速记体最密集区全部绕开。
5. **用户读数面**：charter §2.4「用户只读 D2＋D1」——D2 已落盘待用户过目；D1 待增删定稿；
   D3–D5 无人工读数（声明兜底）。

## §3 待续项（不闭合 0cm）

- **D2 用户过目**（charter S2 收尾步；约 2,000 英文词）；
- **D1 用户增删定稿**（定稿后 `v0.1` → `v1.0`）；
- **可选发帖英文草稿**（charter S4 可选项）本批未产出——发帖动作与最终措辞留用户（charter §1 明示不入仓）；
- 后续源档修订触发 D1 映射表更新＋受影响英文档重出（D1 §4 维护纪律）。

## §4 台账

- TODO：`P1-0cm` 勾选节 S1–S4 四项勾选（附待续注记）；计数行本批指针改写（计数不变 60）。
- BACKLOG：计数行本批指针改写（计数不变 60）；`0cm` 专节补第一轮出稿状态行。
- BACKLOG 第二卷：§1.109（本批流水）。
- 索引：头行 v4.125 → **v4.126**；`AUTH-EN-TRANSLATION-LINE` 条目与 §8 桶补出稿状态（`pending` 维持——用户过目/定稿未竟）。
- 机械门禁：本批落盘后复跑 `check_repository.py`——**首跑 3 错误如实记并同批修复**（① 第二卷 §1.109 批档链接 `../audits/…` 应为 `audits/…`〔自 docs/ 解析〕；② TODO `P1-0cm` 四项全勾触发「TODO 全勾但 BACKLOG 仍记开放」——补第 5 项开放勾选「收尾批」＝D2 过目＋D1 定稿，与 BACKLOG 开放口径一致；③ TODO P1 路由行 1252>1200 超长——回退该行 0cm 项为原文，进度由计数行与专节承载）；**复跑 `valid: true`、error_count 0**。另 D3 成绩表逐格核验＝源/译两版各 89 行，任务名/批/分数/用时/时限五列逐格一致（脚本比对，非人工目测）。

## §5 关键词

157 批、0cm S1–S4、第一轮翻译、docs/en/、TRANSLATION_GLOSSARY v0.1、README.en、TB21 报告英译、
写控当前态蒸馏、ACAF 当前态蒸馏、计数 60 不变；157 批复审、D5 平台勘正（DPAPI／file-0600）、
清单签名 v2 边界、stdin/stdout JSON-lines、中英分叉声明、62→63 词条勘正。

## §6 D2 用户过目·第一轮（同日，工作树内直接应用）

用户逐行审阅 D2，五处修订全部应用：

1. 行 75：「directly on the real machine rather than in a sandbox」→「**but not** in a sandbox」（去除比较意味）；
2. 行 84：删「the Bing family has been removed」；默认引擎补目的说明「for networks without a VPN」（缺省直连 `360search,baidu` 的补充性短语）；
3. **codex 命令审查交代中英同补**：中文 README 行 70 按用户口述原文增「同时使用并借鉴了其命令审查部分」；英文对应「while also using and adapting its command review」——`README.md` 改动随本批入工作树；
4. 行 98：黑板「shared by the main agent and the mechanical layer」→「shared by **the agents** and the mechanical layer」；
5. **Evaluation 块整节移除**（用户口径＝评测不进 README——README 是总览介绍，只放框架本身的功能性与设计介绍；跑分披露留发帖正文）——英文版移除后，**用户随即裁决仓根中文版同步移除**：`README.md` 的 `## 评测` 节（原行 108 起，含 73/89＝82.0% 段与 k=1 边界段、报告链接）已删，两版对齐；TB21 报告的对外入口自此只剩 `docs/en/` 发帖包与本档引用。

修订后门禁复跑 `valid: true`、error_count 0。

## §7 措辞与充分性复查轮（同日，用户令「再检查一下翻译文件的整体措辞以及充分性」）

- **修复 4 处**：① D4 §3 条 3 反引号失衡（`` `/dev`//proc`//sys` `` → `/dev` / `/proc` / `/sys`）；② D2「the three keys `ORZ_ALLOW_WRITE` etc.」→「the three `ORZ_ALLOW_*` keys」；③ D2 idle-kill 因果挂反 →「backgrounded and kept output/CPU-alive as a backstop against the idle-kill」；④ D2 审计归档连读句 →「the mechanical audit fact report and the single-package session-blackboard archive」。
- **D1 增补 4 词**：载体集=carrier set／根本性树根=fundamental tree roots／宿主态祖先链臂=host-state ancestor-chain arm／信任锚=trust anchor；**程序化实测＝52 行（含别名并写共 63 个中文名词条，按 `/` 切分计）**，四处台账「51 条」计数口径同批更正为实测值；2026-10-02 复审勘正为 63（原记 62 系计数口径差 1，见 §11）。
- **充分性结论**：D2 与中文 README 结构逐节对齐（评测节双版已除、codex 命令审查交代双版同补）；D3 成绩表数值列脚本核验逐格一致；D4/D5 蒸馏覆盖现行规则面与机制面，关键处均保留中文权威档链接。未覆盖面（有意）：agent/机械层/黑板/LIF 的机制深述仅存在于 README 总览级——整合形态见用户同日提问的处置裁决。

## §8 D6 总纲整合件（同日用户裁决增补）

- 用户令「加一份总纲吧，D4/D5 两部分作为真机环境下的安全保障，额外详细介绍也可以」→ charter §1 增补 **D6**＝`docs/en/CURRENT_DESIGN_EN.md`（现行设计总纲：定位 → 架构总览 → Agent 层 → 机械层 → 黑板 → LIF → 载体与组件 → **§7 真机环境下的安全保障**〔四层合成＋加深介绍＝ACAF 溯源／权限桥与审批组件策略／灾难写控保底／审计与恢复，附诚实边界〕→ 设计权威与钉），英文首版实测 2,097 词（charter 预估 4,000 已按实测更正）。
- **D4/D5 保持独立深入件**（EN↔CN 一对一维护映射不破），总纲 §7/§8 下链；词典 §3 增 D6 映射行；charter §1 增 D6 行；台账改六件口径（TODO/BACKLOG 计数行、0cm 节、第二卷 §1.109、索引条目与 §8 桶）。
- **冗余收敛（同日用户令「总纲里面 D4/D5 就不用详细介绍了，正常就行」——前令「额外详细介绍也可以」仅指 D4/D5 本体可保持独立）**：§7 由五小节详细合成版收敛为正常篇幅（四层各一段＋边界一段＋深入件链接）；§8 重复链接行删；收敛后实测 **1,697 词**，charter/台账措辞与字数同步更正。
- 门禁复跑 `valid: true`、error_count 0。

## §9 README 精简轮（同日用户令「readme 中的篇幅和总纲是不是有大量重合啊，readme 要不要精简一下」）

- 裁决＝**精简，中英两版同步**（对齐纪律）。重合实况＝D6 总纲 §2–§6 与 README「框架介绍」块互为双叙事；分工定为 **README＝功能门面**（快速开始／配置／入口＋框架速览），**D6 总纲＝设计叙事**。
- CN `README.md`：删「### Agent 层」～「### 载体与组件」五小节＋运行路径段；框架介绍收敛为两段（架构速览含安全四层一句，保留写控设计权威与 `orz/SECURITY.md` 链接；载体与组件＋D6 总纲链接）。
- EN `README.en.md`：同步删 Agent layer ～ Carrier and components 五节＋run-path 段，收敛为同构两段＋ `CURRENT_DESIGN_EN.md` 链接。
- charter D2 行补精简裁决；词典 §3 D2 行 pin 改「slimmed face」。门禁复跑 `valid: true`、error_count 0。

## §10 精简反转与 D6 并入轮（同日用户令「中文版别同步啊，母语者又用不到总纲，中文版恢复原样；要不然就把英文版总纲和英文版readme合起来，英文版readme细致些」）

- **CN `README.md` 恢复原样**：框架介绍五小节（Agent 层／机械层／黑板／时间与动作域判断组件／载体与组件）＋运行路径段全数恢复，去除 D6 总纲链接，重回 0.8.10 发布面原样（本批前序保留项不变＝codex 命令审查交代在、评测节已除）。
- **英文 README 改扩展版并吸收 D6**：恢复 Agent layer ～ Carrier and components 五节＋run-path 段，新增「Safety on the real machine」章（总纲 §7 收敛版吸收＝四层＋边界＋D4/D5 链接）；档头注明 expanded edition 与中英分叉。
- **`docs/en/CURRENT_DESIGN_EN.md` 删除退役**（内容全数吸收：§0/§1→README 既有段、§2–§6→恢复节、§7→安全章、§8→既有权威链接）；**D6 编号保留记并入 D2，五件口径恢复**；charter D2/D6 行＋词典 §3 D2/D6 行同步。
- 中英两版 README 自此形态有意分叉（EN 独有安全章；CN＝发布面原样）。门禁复跑 `valid: true`、error_count 0。**（→ 同日 §12：中文版亦增补真机安全设计章，此处「EN 独有安全章」与「CN＝发布面原样」不再成立。）**

## §11 独立复审与勘正（同日，157 批审查处置，2026-10-02）

> 触发：用户令「审计一下 157 批的内容，判断其内容是否合理、措辞是否合适」。形态＝主会话只读复审（五件英文档＋中文源档＋相关源码回查）＋同批勘正；**计数不变 60、未提交未推送**。

**独立复核通过（批档自称判据，逐条复算）**：① D3 成绩表＝源/译两版各 89 行，编号/任务/批/分数/用时/时限六列**零差异**；载体哈希逐档计数一致（`87941130` 35/35、`3332b38f` 42/42）；两版均 9 表 23 标题，结构 1:1。② 声明句五件齐全且与 charter §2.3 逐字一致。③ D1 扩表基数可复现（charter 附录 A 种子 30 行 → 52 行）。④ 有嫌疑但正确：D2 上下文阈值 320K/500K 与现行源码一致（`orz-loop/src/context_scale.rs` H1/T1），「10 工具面」与项目口径自洽（8 冻结＋2 次用户主导例外）。

**勘正项（同批复审落账）**：

1. **D5 平台事实**（P2）：`K_install` 原写「DPAPI-protected」系 Windows 专有表述，Linux 载体实为 `file-0600-installation` 明文密钥库（0600 权限，面向评测容器）；已补平台注记。源＝`orz/crates/orz-bin/src/bin/orz-signer.rs` 启动契约＋`orz/crates/orz-host/src/keystore.rs`。
2. **D5 IPC 与清单校验事实**（P2，本轮最重）：原写「Named-pipe IPC」与「signed manifest＋独立发布密钥」，与实现不符——实际传输为 **stdin/stdout 上的 JSON-lines**（源码自注「the narrow-IPC v1 transport」），清单**签名校验尚未实现**（源码显标 v2 升级路径；当前信任锚＝启动链交付清单＋二进制自哈希，与宿主二进制同信任级）。已按实现事实改写，未落地能力改为显式标注（「registered v2 upgrade path」），避免对外文档高估在役信任链。
3. **中英分叉未完整声明**（P2）：英文 README 删去「Bing 家族已出集」而中文版保留，原 §10 只声明安全章一处有意分叉；已在 D2 档头与 D1 §3 映射行列为**第二处有意分叉**（尊重用户原删改裁决，不改回中文侧事实）。**（→ 同日 §12：安全章分叉撤销，仅余 Bing 句一处分叉。）**
4. **「untouched release face」措辞失实**（P2）：中文 README 本批实有改动（补 codex 命令审查句、删评测节），并非未动；D2 档头与 D1 映射行改为 release-shaped／unexpanded。
5. **D5 D-12 理由陈旧**（P3）：`provider-side search` 已被本地检索线取代（2026-09-13 后端转本地、0bp 指浏览器车道）；ticketless 结论仍成立，已加括注说明沿革。
6. **台账计数勘正**（P3）：D1 中文名词条数 62 → **63**（按 `/` 切分口径），四处台账同改；D3「其余 10 张表」→ **8**（全档 9 表）；D2「137 行」补注删评测节后 131 行。
7. **措辞四处**（P3）：D2 `A run roughly flows:` → `A run flows roughly like this:`；`but not in a sandbox` → `with no sandbox`（去比较意味）；`smaller debts to` → `smaller borrowings from`；`存档/readme/README.md` 链接加「Chinese archive directory」括注。

**受影响文件**：`docs/en/ACAF_CURRENT_EN.md`／`docs/en/README.en.md`／`docs/en/TRANSLATION_GLOSSARY.md`／本档；台账三处（`TODO.md`、`docs/BACKLOG_AND_PRIORITIES.md`、`docs/BACKLOG_AND_PRIORITIES_2.md`）计数口径同步。

**未改**：中文权威档（`README.md` 等）与 D3/D4 数值面零改动；未否定任何用户裁决。机械门禁复跑 `check_repository.py` ⇒ `valid: true`、error_count 0。

## §12 中文 README 增补真机安全设计章（同日，用户令，2026-10-02）

> 用户令「中文版 readme 里面也单独加上真机安全设计这一节吧，加在开发者节前面」。这是对 §10「EN 独有安全章」形态的**同日再裁决**，属 0cm 线内的事后收口；**计数不变 60、未提交未推送**。

**改动**：`README.md` 在「框架介绍」末尾、「## 开发者入口」之前新增 `### 真机安全设计`——内容为英文版 "Safety on the real machine" 章的中文对应件（四层＝ACAF 来源可信／权限桥与审批组件策略／写入管控灾难保底／审计与恢复，＋已知边界段）；链接指中文权威档（[`ACAF 设计档`](../AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md)／[`写控设计档`](../WRITE_CONTROL_BACKSTOP_REVISION_DESIGN_2026-09-29.md)／`orz/SECURITY.md`），不指 `docs/en/`。

**事实核验（写死前逐条回查源码，沿 §11 纪律）**：`orz rollback list|restore` 存在于 `orz-bin/src/main.rs`；载体完整性自检存在（`orz-host/src/carrier_integrity.rs`＋启动期 `verify_at_startup()`）；ACAF fail-closed、yolo 默认放行与人工审批未实现、五条封闭规则、Landlock 守卫均与既有权威档一致。ACAF 一段**未**复用 §11 第 2 项勘正掉的过度表述（不含「签名清单已校验」）。

**分叉口径更新**：中英两版自此**均含**真机安全设计章 ⇒ §10「EN 独有安全章」与 §11 第 3 项「第二处有意分叉」不再成立；**剩余唯一有意分叉＝英文版省略中文版「Bing 家族已出集」一句**（§11 第 3 项对该句的分叉声明继续有效）。同步面＝`docs/en/README.en.md` 档头、`docs/en/TRANSLATION_GLOSSARY.md` §3 映射行、BACKLOG `0cm` 节、BACKLOG 第二卷 §1.109、索引头行 157 批注记。**（→ 同日 §13：该句已由中文侧同步删除，分叉归零。）**

**未改**：英文版安全章正文（仍为英文读者入口，链接指英文蒸馏件）；D1 词条数 63；0cm 开放项与计数 60。门禁复跑 `check_repository.py` ⇒ `valid: true`、error_count 0。

## §13 Bing 句同步与「唯一差别＝总纲」口径（同日，用户令，2026-10-02）

> 用户令「中文那边，bing那一句也进行同步吧，两边的唯一差别就是总纲就好了，中文母语者看得懂设计文稿但英文使用者看不懂，就给英文使用者留一份总纲配着看」。**计数不变 60、未提交未推送**。

- **Bing 句同步**：中文 `README.md` 外部检索子代理条删除「；Bing 家族已出集」半句（英文侧已于 §6 删除），该处**不再构成中英分叉**；§11 第 3 项与 §12 所述「唯一剩余分叉」随之**归零**。
- **口径定案**：中英两版 README **内容互为镜像**；唯一有意差别＝**英文版带总纲**——退役 D6 总纲并入英文 README（英文 README 即「镜像＋总纲」扩展版）；中文母语者可直接阅读中文设计档（ADR-0010 与 `docs/` 设计权威档），故中文版不重复同一叙事。英文档头与 D1 §3 映射行按此改写。
- **事实边界与裁决收口（2026-10-02 用户裁决）**：真机安全设计章现两版皆有（§12），故此处「总纲」指**英文版的扩展叙事定位**，不是「英文版另有中文版没有的章节」。用户裁决＝**(a) 维持现状**、不拆独立总纲件；**理由＝不翻译具体设计文档**——本仓设计权威档全为中文、翻译麻烦，英文使用者改以英文 README 的总纲部分替代。该理由同时登记进 charter §4 排除面。
- **受影响文件**：`README.md`／`docs/en/README.en.md`／`docs/en/TRANSLATION_GLOSSARY.md`／本档；台账三处与索引头行 157 注记、`AUTH-EN-TRANSLATION-LINE` 条目同步。
- 门禁复跑 `check_repository.py` ⇒ `valid: true`、error_count 0。

## §14 末轮全量审查（同日，用户令「请最后对全部的英文版文件做最后一轮审查」，2026-10-02）

> 范围＝`docs/en/` 五件全部＋中英两版 README 结构对照＋英文档所引数值/常量回查源码；形态＝只读复算＋同批勘正。**计数不变 60、未提交未推送**。

**独立复算（全部通过）**：

1. **链接零断**：五件英文档全部相对链接（`../../` 指仓根/ADR/源码、`../` 指 `docs/`）逐个解析＝**0 断链**；`check_repository.py` 同判。
2. **D3 数值零漂移**：源/译两版各 98 行编号行（89 题成绩表＋9 行披露清单）；成绩表六列逐格比对（题名/批/分数/用时/时限）**0 差异**；载体哈希计数一致（`87941130` 35 次、`3332b38f` 42 次）；表分隔行 35＝9 表、标题 23，两版 1:1；全档数值词元对照仅余英文标题/日期合理增项。
3. **D2↔中文 README 结构镜像**：标题树逐条对照——中文 15 标题（无 H1），英文 16 标题（多出英文版 H1 自身），其余 15 条一一对应；「真机安全设计」章两版皆有；Bing 句两版皆无。
4. **D4 规则面常量回查源码**：`DEVICE_SAFE_NODES` 恰 7 项（源码 `[&str; 7]` 与表内 7 值逐项一致）、`ROOT_MAKE_GRANT` 恰 5 位、`ANCESTOR_SWEEP_VERBS` 恰 15（删除 8＋搬移 7）、Linux 树根恰 12／Windows 树根 4、`WRITE_FILE` 不上 `/` 的否决论证在源码注释在案；正向放行 fixture 抽验（`make install`／`dd if=x of=/dev/null`／`rm -rf /usr/local`）与 §6 所述一致。
5. **D5 事实面**：`K_install` 平台注记（Windows DPAPI／Linux `file-0600-installation` 0600）、signer 启动契约（清单自哈希、缺失或错配非零退出）、stdin/stdout JSON-lines 窄 IPC、`HKDF-SHA256(K_install, session_id‖goal_digest‖policy_digest‖signer_revision)`、票据结构字段、`control_ticket_issued/consumed/rejected`、六类票据名——均与源码/设计档一致。
6. **D2 事实面抽验**：`ORZ_ALLOW_WRITE/SHELL/NETWORK` 三键与 `[permission] mode=…` 启动面、`--allow-shell`/`--allow-network` 缺 `--allow-write` 即报错退出、`--retrieval-enabled`（旧 `--retrieval-mode` 兼容解析）、`ORZ_STALL_TIMEOUT` 360／`ORZ_TOOL_TIMEOUT_SECS` 300 默认值、`orz rollback list|restore`——全部与源码一致。

**本轮勘正（英文侧 5 处）**：

1. **D2 档头「扩展版」定性**（P2）：原句「唯一有意差别＝本版的总纲级设计叙事」已不成立——中文版经 §10 恢复五小节、§12 增补安全章后，两版**内容结构完全镜像**（标题树差额仅为英文 H1 本身）。改为按用户口径写实：两版内容互为镜像，英文版是**角色上的扩展**（覆盖不变）——兼作英文读者面对中文设计权威档的总纲替代件；D1 §3 映射行同步。
2. **D2 配置表第 3 步**（P3）：`unconfigured, it refuses to start a run (the process works, runs are refused)` 语义重复 → `unconfigured, it refuses to run (the process starts; runs are refused)`。
3. **D2 真机安全设计第 3 层**（P3）：悬垂分词 `…belong to the approval component; enforced at the tool surface…` 补主语（`It is enforced at …`）。
4. **D4 §2 文案段首**（P3）：`Copy on a block refusal:` 非自然英文 → `**Block-refusal copy**:`。
5. **D3 档头句**（P3）：`Every per-task reading … cites no secondhand accounts` 主语不搭（读数不「引用」账面）→ `No per-task reading in this report comes from a secondhand account: all were extracted …`。

**登记未改（中文权威侧缺陷，留中文批处理）**：

- **规则 1 删除动词封闭表漏列 `rmtree`**：中文设计档 §1 与 D4 均列 7 个动词（`rm`/`rmdir`/`rd`/`del /s`/`Remove-Item`/`erase`/`ri`），实现 `CATASTROPHIC_DELETE_VERBS`／`DELETE_VERBS` 各为 **8 项**（含 `rmtree`）。英文侧忠实照译中文权威，**不在英文侧单独改**（改则破坏「中文原文权威」口径），登记为中文设计档下一轮勘正项；`del /s` 系动词与递归旗并写，同批可一并规整。

**发帖渠道定案（用户同日裁决）**：**Reddit**——linux.do 密码重置/登录码邮件不可达（判定为投递链路问题而非封号）、V2EX 已转邀请码制、NodeSeek 广告多且热度低，三者均排除；与 charter／台账既有「Reddit 征求建议」口径一致，无需改档。

门禁复跑 `check_repository.py` ⇒ `valid: true`、error_count 0。

## §15 定案与上传（同日，用户令「英文readme我第一轮时已经审过，目前没什么新发现，可进行定案并同步上传至github页」，2026-10-02）

> **计数 60 → 59（0cm 闭合，160 批）**；本次提交并推送 `origin/main`。用户裁决＝D2 第一轮过目即定案、无追加修订。

**动作**：

1. **D2 定案**：`docs/en/README.en.md` 用户过目（157 批 §6 第一轮）无新发现 ⇒ 冻结为对外英文门面；本轮未改正文（§14 五处措辞勘正先于定案入档）。
2. **D1 定稿**：`docs/en/TRANSLATION_GLOSSARY.md` `v0.1 → v1.0`——用户未提词条增删，按现表冻结（档头「用户增删待续」移除；§4 维护触发保留：先改表、再重出受影响英文档）。
3. **台账闭合**：索引头行 v4.128 → **v4.129**（本批＝160 批 0cm 定案上传 60 → 59）＋`AUTH-EN-TRANSLATION-LINE` 条目 `pending` → `implemented` ＋ §8 状态桶由 `pending` 移入 `implemented`；BACKLOG 计数行 60 → 59＋P1 开放项清单与优先级总览表移出 `0cm`＋`0cm` 节补闭合行；TODO 计数行与 P1 路由行同步＋`P1-0cm` 收尾批勾选；第二卷 §1.112（本批流水）。
4. **入仓与上传**：本次提交含 157/158/159 三批累积件（五件英文档＋三份批档＋台账）与 0cm 闭合改动，推送 `origin/main`（远端 `SilverWhite/CLI`）；对外英文门面入口＝`docs/en/README.en.md`。
5. **不入仓项**：Reddit 帖子正文、发帖动作与发帖后纠错回灌（charter §1/§3）——如需回灌按新项立项。

**门禁**：闭合改动后复跑 `check_repository.py` ⇒ `valid: true`、error_count 0（台账一致性对拍覆盖 BACKLOG/TODO 计数、优先级总览表、开放项清单、TODO 勾选态、索引 §8 桶）。

**上传回执（同日补记）**：父仓 `main` 提交 **`c5f7a0fe`**（`a8360ace..c5f7a0fe`）已推送至远端仓库（GitHub 侧仓库已由 `SilverWhite/CLI` 改名 `SilverWhite/orz`，旧名 301 重定向；`docs/en/README.en.md` 线上可访问，raw／blob 均 200）；本地 `origin` 已改指新地址。

## §17 英文 README 与中文版再对齐（同日，用户令「现在的话，请按照远端中文版修改英文版吧」，2026-10-02）

> 计数不变 59；同批入仓。

**背景**：用户当日在 GitHub 网页连续修改中文版 README（`54fe4330`／`f27e3024`／`7a1cfa44`）——删除「从源码运行」整节（用户口径：有源码运行需求的开发者会自己查）、构建前置 `[!IMPORTANT]` 移至「开发者入口」、审批口径按实现实况更正、多节门面化精简。

**对齐动作**（英文 `docs/en/README.en.md` 逐节同步）：① 删「Building from source」整节及其 `[!IMPORTANT]`（该 important 依中文版新位置落在开发者区）；② 主 Agent 条补 DeepSeek 自动路由注记（接口名暂不变动）；③ 机械层「Security」与真机安全设计第 2 层换新审批口径（交互审批面已实现并接线、yolo 默认不逐次弹窗、强制审批类除外）；④ 写控条删版本沿革括注与「非普遍写审查」、定位句与「设计稿」措辞同步；⑤ 黑板删 plan-epoch 括注；⑥ 真机安全新增狗粮轮风险注记、段首与「已知边界」措辞同步；⑦ 开发者区删单点维护句；⑧ 当前状态「设计」「发布」两行精简（发布链接保留新仓名）。

**复核**：中英标题树 1:1（英文侧多 H1 与总纲档头）；英文档内部相对链接 0 断；`check_repository.py` ⇒ `valid: true`、error_count 0。

## §16 上传后收尾（同日，用户反馈「GitHub页里我没看到英文版readme啊？main链里发布的还是中文readme」「对外链接统一成新地址吧」，2026-10-02）

> 计数不变 59；提交 **`b916b966`**，推送 `origin/main`。

**成因**：GitHub 仓库落地页只渲染**仓根 README**（本仓为中文版）——英文门面在 `docs/en/README.en.md`，属既有布局但首页无入口，用户因而找不到。

**处置**：

1. **中英双向语言入口**：`README.md` 与 `docs/en/README.en.md` 首行各加一行语言切换（中文侧为「中文 ｜ English」、英文侧为「English ｜ 中文」，两端分别指向中文版与英文版；本档只用文字描述该行、不写 Markdown 链接语法，以免被当本档相对链接）——落地页一眼可点进英文版；两版仍互为镜像，仅该行方向相反。
2. **对外链接统一新地址**：`README.md`／`docs/en/README.en.md` 的 GitHub Releases 链接（4 处）＋`.gitmodules` 子模块 URL（`SilverWhite/CLI.git` → `SilverWhite/orz.git`，本地 `submodule.orz.url` 已随 `git submodule sync orz` 同步）统一改指新仓名 `SilverWhite/orz`。
3. **留档不改（登记）**：历史审计档（17 篇）、ADR-0010 §14 沿革记录、台账流水与发行包内快照中的旧名引用**按原样留档**（旧名 301 重定向可用、不构成断链）；如需全库统一可另批扫改。

**线上复核**：落地页 `github.com/SilverWhite/orz` 渲染 HTML 含英文入口链接；`releases/tag/v0.8.10`／`releases`／`blob/main/docs/en/README.en.md` 均 200。**门禁如实记**：本节首次入档（提交 `c0a3dc12`）时首跑红——门禁把本档内的语言入口示例当成本档相对链接，报 1 错误（`docs/en/README.en.md` 相对 `docs/audits/` 解析）；改为代码体写出同批复绿，`valid: true`、error_count 0。
