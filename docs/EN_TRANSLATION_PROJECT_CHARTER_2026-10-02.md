# orz 对外英文翻译线（0cm）立项档：Reddit 征求建议用英文门面件

> **状态**：`charter v1.0`（2026-10-02 用户令「就按照这么做吧，请进行立项」定档）。
> **权威链**：本档为该项目**范围与质量口径的权威**；S1 产物术语对照表为**术语权威**；实施批档届时落
> [`docs/audits/`](audits/)。台账登记（BACKLOG/TODO/TODO2/索引）**挂起待执行**，见 §6。
> **触发**：用户拟在 Reddit 发帖为 orz 征求建议与纠正，需要英文门面件；体量评估与本立项范围
> 经 2026-10-02 会话三步收敛（全量评估 → 关键档评估 → 最终范围定稿）。
> **用户裁决（2026-10-02，本档口径权威）**：
> ① 目标＝征求意见与纠正，**非严格审查**——质量标准为 gist 级可读，不承诺逐句等价；
> ② 范围＝**当前确定性设计内容与思路（蒸馏当前态，非逐字翻译档案）＋ 主仓 README ＋ 跑分报告**，三项即足够；
> ③ **需额外维护一份翻译对照表**（修正先前「不做术语表」的口令），直接使用指向同等事物的英文专有名词；
> ④ **人工逐句审查不可用**（用户自估审校预算≈小半月，且无第二审校人）——质量靠对照表注入、
> 表格数字程序化照抄、英文档头声明三类机制替代，不靠人读。

## §0 一句话定位

为 Reddit 征求建议产出**最小英文门面集**（README＋跑分报告＋写控/ACAF 当前态蒸馏＋术语对照表，
有效翻译量约 **1.5 万中文字 ≈ 1 万英文词**，占 docs/ 全量 109 万字的 **1.4%**），
LLM 出稿、对照表兜一致性、声明兜诚实性，用户个人投入收敛到 **1–2 天**（对照表定稿＋README 过目＋摘要抽查）。

## §1 交付物清单（五件；全部落 `docs/en/`）

| # | 交付物 | 来源 | 有效字数（中文口径） | 说明 |
|---|---|---|---|---|
| D1 | `docs/en/TRANSLATION_GLOSSARY.md` | 本档附录 A 种子 | — | **术语权威**。双语术语对照（约 30–40 条）＋中英文档映射（哪篇英文档对应哪篇中文权威档、钉到哪个版本）。版本化维护，触发时机＝某批修订了已翻译子系统：先改表、再一次 LLM 调用重出受影响英文档 |
| D2 | `docs/en/README.en.md` | 主仓 `README.md`（3,107 字） | ~3,100 | 对外唯一门面。**唯一需要用户亲自过目的翻译件**（约 2,000 英文词） |
| D3 | `docs/en/TB21_V41_89_FULL_ROUND_REPORT_EN.md` | [`TB21_V41_89_FULL_ROUND_REPORT_2026-10-01.md`](TB21_V41_89_FULL_ROUND_REPORT_2026-10-01.md)（3,366 字） | ~3,400 | 源档自标注「对外披露底稿」，结构现成；成绩表逐格照抄，只翻表头与说明列；保留「k=1 筛查轮、不构成榜单成绩」边界声明 |
| D4 | `docs/en/WRITE_CONTROL_CURRENT_EN.md` | 写控 **v4**（[`WRITE_CONTROL_BACKSTOP_REVISION_DESIGN_2026-09-29.md`](WRITE_CONTROL_BACKSTOP_REVISION_DESIGN_2026-09-29.md)，现行规则面权威）＋ **v1**（[`WRITE_CONTROL_MECHANICAL_DESIGN_2026-09-26.md`](WRITE_CONTROL_MECHANICAL_DESIGN_2026-09-26.md)，v4 声明「其余架构不变」的存续部分） | ~4,500（蒸馏后） | **蒸馏当前态，非逐字翻译**：只写「现在规则是什么、为什么这么定」；v4 头部约 40 行沿革压缩为每版一行；规则表（5 条 block 规则）命令/路径/目标形状本就是英文符号，逐格照抄 |
| D5 | `docs/en/ACAF_CURRENT_EN.md` | [`AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md`](AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md)（6,367 字，唯一专门档） | ~3,500（蒸馏后） | 蒸馏当前态；**必须并入 0.8.6/0.8.7 的 provision 变更**（keystore/signer-manifest 装配键、fail-closed 语义），以 0.8.7 双平台在役事实为准 |

帖子正文本身**不在仓内交付**（发帖时直接英文撰写，自包含）；D2–D5 作为帖内链接。

## §2 质量口径（无人工审查的机制化替代）

1. **对照表注入**：每次 LLM 翻译调用附 D1 全表，跨文档术语一致性由机制保证而非人工校对——
   这是审校预算为零时唯一可控制的一致性杠杆。
2. **表格与数字程序化照抄**：规则表、成绩表中的命令、路径、sha、百分比不进入翻译，
   逐格从源档复制；数字错位是此类文档最致命错法，结构性排除。
3. **诚实声明前置**：每篇英文档头部固定一行
   `LLM-assisted translation; the Chinese originals are authoritative. Corrections welcome.`
   ——既如实标注质量等级，又把纠错外包给评论区（与本立项「征求意见」目标同向：correction 即免费审校）。
4. **用户只读两样**：英文 README（D2）＋对照表定稿（D1）。D3–D5 不设人工读数。
5. **沿革块一律压缩**：嵌套沿革引文是速记体最密集、机器翻译错误率最高、对外部读者价值最低的三重
   最劣区——蒸馏形态直接绕开，单篇省 20–30% 篇幅。

## §3 批序（S1 → S4；各步独立放行）

- **S1 对照表定稿**：附录 A 种子交用户增删定稿 → D1 落盘。用户投入约半天。
- **S2 README 英译**：D1 注入 LLM 出稿 → 用户过目（约 30 分钟）→ D2 落盘。
- **S3 设计当前态蒸馏英译**：D4/D5 出稿（表格照抄＋声明前置）→ 落盘，无人工读数。
- **S4 跑分报告英译＋发帖包整理**：D3 出稿落盘；可选产出帖子英文草稿（`docs/en/REDDIT_POST_DRAFT.md`，
  纯草稿、发帖动作与最终措辞留用户）。发帖本身不入批。

## §4 明确不做（排除面）

- **不翻译 docs/ 全量**（486 篇 / 约 109 万中文字）：全量人工折算约 250–300 人日，机器＋校对约 15 全职日，均不可行且无必要。
- **不翻译台账与审计**：`BACKLOG_AND_PRIORITIES.md`（10.4 万字，全库最大文件）、第二卷、TODO、
  `CLI_PROJECT_INDEX.md`、`docs/audits/`（288 篇）全部排除——本质是内部工作记录，不对外。
- **不动 `orz/README.md`**：上游 Grok Build 英文件（0 中文字符），描述 fork 来源、不代表 orz 自身特性；
  orz 对外英文身份以 D2 为准。
- **不承诺逐句等价**：英文档定位 gist 级参考件，中文档为唯一权威（声明前置即此意）。
- **不新造英文品牌名**：专有名词取「指向同等事物的英文叫法」，以附录 A 定稿为准，不在翻译过程中改名。

## §5 风险登记

| # | 风险 | 等级 | 缓解 |
|---|---|---|---|
| R1 | 无审查预算下的误译（否定词/量词类压缩表述：「恰 N 项」「不触发」「双面全退役」） | 中 | 声明前置＋表格照抄（§2.2/2.3）；此类表述多集中于沿革块，蒸馏形态已绕开 |
| R2 | 术语跨文档漂移（无表状态下 carrier/安装目录 混叫，评论区被术语问题淹没） | 中 | D1 注入每次调用（§2.1）；D1 缺位视为未放行 |
| R3 | 台账并发冲突（隔壁窗口 143–155 批在途未提交，四处台账脏区活跃） | 高（本批已规避） | 本立项为**独立新文件**，零触碰台账；登记以补丁形态挂起（§6） |
| R4 | 源档漂移（写控/ACAF 后续批修订导致英文档过期） | 低频 | D1 的中英映射钉版本；修订批触发表更新＋受影响英文档重出（§1 D1 说明） |

## §6 台账登记（**挂起待执行**；本批零触碰）

隔壁窗口 143–155 批累积件在途未提交（最后提交 `1552e4d1`＝140–142 批），四处台账均处脏区。
按批序纪律，以下登记**待在途批提交后的下一台账批执行**，届时若 `0cm` 已被占用则顺延 `0cn` 并回改本档：

1. `docs/BACKLOG_AND_PRIORITIES.md`：P1 名册增一行「对外英文翻译线（0cm，2026-10-02 用户裁决立项＝Reddit 征求建议英文门面件；charter＝本档）」；计数 **59 → 60**；计数行批序要点改写。
2. `TODO.md`：开放项路由 P1 节增 `0cm` 勾选项（S1–S4）。
3. `TODO2.md` / `docs/BACKLOG_AND_PRIORITIES_2.md`：本立项批流水按第二卷形态记 §1.10x。
4. `CLI_PROJECT_INDEX.md`：本档登记入索引（类型 `charter`，关联 `0cm`）。
5. 批号：待分配（本档落盘时 156 空闲，但隔壁窗口下一批可能占用；以登记批实际号为准，本档不预先占用）。

## 附录 A 术语对照表种子（S1 起点草案；用户增删定稿）

| 中文 | 英文（定名） | 备注 |
|---|---|---|
| 载体 | carrier | 发布安装形态（目录＋三件套＋回滚点） |
| 三件套 | the three binaries | orz / orz-signer / orz-acaf-provision |
| 写控 / 机械写控 | write control / mechanical write control | |
| 灾难保底 / 硬边界 | catastrophic hard backstop / hard boundary | 「根本性保底＝防扬盘级灾难」 |
| 审批组件 | approval component | Codex 借鉴的 orz-workspace/permission 链 |
| 宿主态 | host state | `.gsa` 会话卷＋ACAF keystore 根两窄目标 |
| 会话卷 | session volume | `.gsa` |
| keystore 根 / signer manifest | keystore root / signer manifest | 已是英文，钉定冠词用法 |
| 规则面 / 封闭枚举 | rule surface / closed enumeration | 恰 5 条 block 规则 |
| 拦截 / 放行 | intercept / allow | 写控语境 |
| 装配 | wiring | 装配同源 env＝wiring-sourced env |
| 判官 | judge | 评测判官 |
| 试次 | attempt | 评测最小计分单位 |
| 题 / 任务 | task | TB 语境统一 task，不混用 problem |
| 整轮 / 收官 | full round / final round | |
| 定向重跑 | targeted rerun | |
| 翻盘 | turnaround | 失败题重跑转 1.0 |
| k=1 筛查轮 | k=1 screening round | 不构成榜单成绩 |
| 榜单成绩 | leaderboard score | 需 ≥5 试次/题 |
| 载体重建 / 换装 / 进体 | carrier rebuild / swap-in / landed in-carrier | |
| 源冻结 | source freeze | pin commit |
| 批 | batch | 台账批序单位 |
| 立项 | project initiation (charter) | 本档即形态样本 |
| 沿革 | revision history | 英文档压缩为一行/版 |
| 台账 | ledger | BACKLOG/TODO/INDEX |
| 狗粮轮 | dogfood run | |
| 摩擦 | friction | |
| 落码 / 复验 | code landed / re-verification | |
| 权威链 | authority chain | |
| 用户裁决 | user adjudication | |

## 附录 B 体量与估算依据（2026-10-02 只读实测）

- docs/ 全量：486 篇 md，约 7 MB，**1,091,881 中文字**（audits 288 篇/52.1 万；顶层约 150 篇/55 万；cases 29 篇/1.3 万；incidents 10 篇/0.37 万）。
- 主仓 `README.md` 3,107 字；`CLI_PROJECT_INDEX.md` 38,738 字；`TODO.md` 67,727 字；`orz/README.md` **0 字（上游英文件）**。
- 本立项集合：D2 3,107＋D3 3,366＋D4 蒸馏有效约 4,500（源 10,670）＋D5 蒸馏有效约 3,500（源 6,367）≈ **1.45 万字 ≈ 1 万英文词**，占全量 **1.4%**。
- 投入折算：LLM 出稿数小时；用户 S1 半天＋S2 过目 0.5 小时＋抽查 1–2 小时 ≈ **1–2 天**；对照全量审校路径（用户自估小半月）不可行——此对比即本立项形态成立的原因。
