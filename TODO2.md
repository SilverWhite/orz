# TODO 卷 2 — 已闭合核对与历史段（TODO 第二部分）

> 用途：承接主 [TODO.md](TODO.md) 的**已闭合项核对节、近期已闭合单行核对与历史整理轮记录**——
> 主 TODO 只留开放项勾选节＋开放项路由（形态对齐 [BACKLOG 第二卷](docs/BACKLOG_AND_PRIORITIES_2.md)，
> 即「主文件＋第二部分」双件形态）。
> 起因：2026-10-02 用户令「todo2改一下形态，直接变成第二部分，给todo腾地方用，就和index的第二部分一样」
> ——本文件原为 TER 专项实施树（2026-09-03 建立），自本批起改形态为 TODO 第二部分，
> TER 树原样收编为 §4（标题降一级、正文零改）。
> **权威边界**：计数与勾选权威仍在主 TODO 计数行/路由行与 BACKLOG——门禁 `check_repository.py`
> 只解析主 TODO（计数行/路由行/开放勾选节），**不解析本卷**；本卷是闭合面与历史段的**叙述留档**
> （不删一字、按原序重排）。新闭合随批先记主 TODO 条目，沉淀后移入本卷。

## 1. 主 TODO 头部历史整理轮记录（2026-08-31—2026-09-19；原样移入）

> 2026-08-31 清理轮：已闭合项压缩为单行核对条目（实施细节以 BACKLOG 变更记录与审计文档为准）；历次计数流水不再在快照重复；勾选状态以 BACKLOG 为权威，本次仅对 BACKLOG/索引已声明闭合的滞后项补勾，未闭合项原样保留。备份：`%TEMP%\TODO.md.bak-20260831`。
> 2026-09-09 整理轮：P0-GOV 00/00a、P0-0p、P1-0q 等已闭合区再次压缩为单行核对（明细见全量快照）；开放项路由同步至 BACKLOG 当前口径。全量快照：[`存档/todo/TODO_FULL_2026-09-09.md`](存档/todo/TODO_FULL_2026-09-09.md)。
> 2026-09-10 滞后入账轮：0o T0（2026-09-07）已达成的 S3 型闭合——0b 验证② / P2-14 S3 / P2-11×3 S3——按 BACKLOG 各条目小节入账后同步补勾与状态注记；勾选状态以 BACKLOG 为权威。
> 2026-09-12 审查入档轮：全项目只读深审报告入档（[`docs/audits/FULL_PROJECT_DEEP_REVIEW_2026-09-12.md`](docs/audits/FULL_PROJECT_DEEP_REVIEW_2026-09-12.md)；不立项、不动计数——0v-C 两项 P0 已被同日 orz `ba934af8` 修复闭合覆盖且触发源实锤与报告独立判断（URL 无痕改写）吻合，账本三处同步断裂已回补，其余 P1/P2 与体系面发现留用户裁决，BACKLOG 治理注记为登记权威）。
> 2026-09-13 跑批待办入账轮：TB 2.1 V4.1 第 0 轮 + 全框架时间预算语义审计 + 即时结果回报/流式检索设计产生的全部开放项一次入账——新增 **0ac `GAP-MECH-IMMEDIATE-FEEDBACK`（P0，设计定稿待放行实施）**、即时结果回报与流式检索设计 **S1–S4**、时间预算语义审计修正批次 **R1–R10 / E1–E9**、`GAP-ORZ-HOST-RESOURCE-SNAPSHOT-DROP` **载体重建待放行**，并更正 **C6 撤回（本轮第 0 轮全部重跑）**。勾选与计数权威仍是 BACKLOG（本轮 **29 → 30**）。

> 2026-09-15 检索补强裁决轮：本地分段检索补强设计稿定稿（v1.0）并落裁决——用户裁决**代理不做引擎白名单**（真机开代理即生效）、其余四点授权工程裁决（G1 三段预算 10 s/5 s/10 s·页、G2 闸门默认开 + 25% + 词集封顶 12、G3 解包 6 worker/6 s、落码顺序 G2 → G1 → G3 → G4）；新增 **S3①-a 补强**开放项（服务 ①-a 检索侧，非 ①-b 投递侧）与索引条目 `DESIGN-RETRIEVAL-LOCAL-SEGMENTED-HARDENING`；**0ac 子切片 ⇒ 不动计数**（未闭合总数维持 **30 项**，勾选与计数权威仍是 BACKLOG）。
> 2026-09-15 账本机械化轮（0ab S1）：`check_repository.py` 新增计数一致性与账本瘦身两组机械检查并常驻门禁（钉子 assurance/tests/test_ledger_consistency_nails.py）；本文件与 BACKLOG/索引的计数行、开放项路由行、`###` 勾选节自此受机械对账约束（结构化锚点核对，报错由人处置，门禁不自动改写）；首批瘦身随批执行——P0 路由撤已闭合 0v/0x、P2 路由补（P2-7）/（P2-8）小节记号、`### P0-B` 重复标题修复、0v 残留勾选按 2026-09-12 闭合入账补勾。勾选与计数权威仍是 BACKLOG（**35 项**，2026-09-16 收口轮后）。
> 2026-09-16 裁决轮：**0ab 闭合入账**（邻线 GLM 完成，判据①②③满足，33 → 32，提交 `d190a17a`）＋ **0ai 重文件拆分立项**（`orz-loop/src/host_exec.rs` 优先，32 → 33）并定为**本轮狗粮线修复考核测试任务**；同轮 **0ah 批序 ② 改为 0ai 狗粮考核测试**（用户口径：工程化任务、不做研究式 A/B 采样），**净 33 项不变**。
> 2026-09-16 收口轮（0.5.2 重建＋考核后）：**0ai 闭合入账**（run `RUN-CLI-6aa999d6` 产出经独立复核合回 orz `b682a67f`，33 → 32）＋ **0aj/0ak/0al 三项立项**（0ai 考核测出：黑板写权限层 deny〔旧摩擦残余〕／增量归档 `-p` 不可达／门禁克隆树漂移，32 → 35）。

> 2026-09-19 排期轮：**检索批次回送与轮级单席位立项 0ar（36 → 37）**——用户令「请将 `RETRIEVAL_BATCH_HANDOFF_AND_ROUND_SEAT_DESIGN_2026-09-19.md` 注册进 todo 并排期」；设计稿 v1.0 定稿（判定权归子代理满 5 条回送／护栏 10 条／可见倒数／提前交付／单批 300 s 未达标交回／合并优先＋溢出拆轮）挂 P0 当前工作集，本文件新增 `### P0-0ar` 勾选节与 P0 路由行；批序 **S1 契约面 → S2 实施（D1→D2→D3）→ S3 同三题 k=1 真机复验**，各步独立放行、**不得跳步合批**，**ADR-0010 转录随 S2**；实施启动以用户实施令为准（本批零代码、零子仓改动）。


## 2. 近期已闭合（供核对，不计入开放项；2026-10-02 自主 TODO 移入）

- [x] P1-0cm 对外英文翻译线（EN-TRANSLATION-LINE，Reddit 征求建议英文门面件）：**2026-10-02 用户裁决定案闭合（60 → 59，160 批）**＝D2 英文 README 用户过目定案（第一轮审阅无新发现）＋D1 术语对照表冻结 `v1.0`＋`docs/en/` 五件（D1–D5）入仓推送 `origin/main`；发帖动作与发帖后纠错回灌不入仓（charter §1/§3）（入口：[`157 批档 §14/§15`](docs/audits/157_0CM_S1_S4_FIRST_TRANSLATION_ROUND_2026-10-02.md)）。

- [x] P1-0aj 黑板写权限层放行：**2026-10-07 用户令「已达成的内容可以进行闭合了」集中闭合（63 → 54，216 批）**＝判据面满足〔`RUN-CLI-6aaad7c8`：`blackboard_write` ×8 全 `allow_once`→`plan_write` ×8 一一对应→`blackboard_read(section=plan)` exit 0〕（入口：[`216 批档`](docs/audits/216_ACHIEVED_ITEMS_CLOSURE_2026-10-07.md)）。
- [x] P1-0am LIF 动力学升级线：**2026-10-07 集中闭合（63 → 54，216 批）**＝S1–S4＋P8 全达成、预测段真机读数〔188〕；翻转裁决以闭合承载＝1D／RLI 并存维持（入口：[`216 批档`](docs/audits/216_ACHIEVED_ITEMS_CLOSURE_2026-10-07.md)）。
- [x] P1-0bc 资源层收口与可失败分配：**2026-10-07 集中闭合（63 → 54，216 批）**＝S4 判据①–⑥逐条对账全落＋S3 轮 `RUN-CLI-6ab00c8a`＋读数面齐〔09-22＋188〕（入口：[`216 批档`](docs/audits/216_ACHIEVED_ITEMS_CLOSURE_2026-10-07.md)）。
- [x] P1-0bk 压缩区间解析偏差修正：**2026-10-07 集中闭合（63 → 54，216 批）**＝S1/S2 落码〔随 0.6.12 进体〕＋S3 真机读数〔188：5 次压缩削减 44–63%〕（入口：[`216 批档`](docs/audits/216_ACHIEVED_ITEMS_CLOSURE_2026-10-07.md)）。
- [x] P1-0bz 上下文脸面瞬态分叉与指纹观测件：**2026-10-07 集中闭合（63 → 54，216 批）**＝S1/S2 指纹件＋S3/S3′ 修码＋S3 载体＋S4 真机读数〔205〕全达成（旧第 2 针归零）（入口：[`216 批档`](docs/audits/216_ACHIEVED_ITEMS_CLOSURE_2026-10-07.md)）。
- [x] P1-0cb 写入管控保底化修订：**2026-10-07 集中闭合（63 → 54，216 批）**＝S1–S4 全达成（S4 读数〔123〕＝拦截 215→个位数、2/5 翻盘、零误拦）（入口：[`216 批档`](docs/audits/216_ACHIEVED_ITEMS_CLOSURE_2026-10-07.md)）。
- [x] P1-0cq 写控误拦两族：**2026-10-07 集中闭合（63 → 54，216 批）**＝S1–S3＋零误拦真机复核〔188，83 审查 0 block〕全达成（入口：[`216 批档`](docs/audits/216_ACHIEVED_ITEMS_CLOSURE_2026-10-07.md)）。
- [x] P1-0cr SlopCodeBench 官方轮记分卡：**2026-10-07 集中闭合（63 → 54，216 批）**＝S1–S3 全达成（134/196、Harbor 36/36 发布、报告档〔207〕＋§13〔210〕＋§14〔215〕）（入口：[`216 批档`](docs/audits/216_ACHIEVED_ITEMS_CLOSURE_2026-10-07.md)）。
- [x] P2-0cs 工具名近似提示：**2026-10-07 集中闭合（63 → 54，216 批）**＝S1〔191〕＋S2〔194〕＋S3 真机读数〔198 首样本，n=1 边界在案〕（入口：[`216 批档`](docs/audits/216_ACHIEVED_ITEMS_CLOSURE_2026-10-07.md)）。

## 3. 已闭合项核对节（2026-10-02 自主 TODO 移入；按优先级分组、原序原样）

> 来源：主 TODO「## P0 — 当前工作集」闭合子节

### P0-GOV 全项目宏观架构对齐与门禁修复（最优先阻断项，2026-09-04 登记；**2026-09-06 全部闭合**）

> 入口：[首轮审查报告](docs/audits/GLOBAL_ARCHITECTURE_AND_INTEGRITY_AUDIT_2026-09-04.md) / [深层审查报告](docs/audits/GLOBAL_ARCHITECTURE_DEEP_AUDIT_2026-09-04.md)；BACKLOG 00；AUTH-GLOBAL-ARCHITECTURE-AUDIT。完整勾选明细见全量快照 [`TODO_FULL_2026-09-09`](存档/todo/TODO_FULL_2026-09-09.md)。

- [x] Phase 1 门禁与编译紧急修复（2026-09-04 闭合）：Markdown 断链修复 / run-event v0.2 payload 夹具映射补齐 / `orz_source_manifest.sha256` 重算 / Rust 告警清零 / 门禁 Exit 0。
- [x] Phase 2 仓库卫生清理与 Git 规范化（2026-09-04 闭合）：根目录 31 个临时调试目录与遗留调试文件清理 + `.gitignore` 收拢本地测试输出。
- [x] Phase 3 权威与产品对齐（第一批，2026-09-04 闭合）：ADR-0010 导言与主 README 过时描述重写 + `architecture/current/` 产品面架构投影扩充 + `cargo check --workspace` 64 members 零告警零错误。
- [x] Phase 3 权威与产品对齐（第二批，2026-09-18 闭合）：主 README 与 `architecture/current/` 投影过时描述对齐——冻结 10 工具面、十工具地位平等（用户裁决，ADR §14.72 第 11 条/v1.74）、`--retrieval-enabled` 启用门＋双车道（§14.65）、v8 上下文窗口（§14.69）、黑板模型写入面与水位读数、0ak 无头会话归档、0x 首轮问询、发布面 v0.6.2 GitHub Releases；零代码零计数。
- [x] 任务 A 解耦寄生：`render_fold.rs` 从 `epoch.rs` 剥离生产折叠渲染，切断 epoch 状态依赖（2026-09-04 闭合）。
- [x] 任务 B 底座瘦身：剔除 15 个无头僵尸 crate，workspace members 64 → 49（2026-09-04 闭合）。
- [x] 任务 C 路径沙箱与 ACAF 下沉：读工具 CWD canonical 越界硬拦截 + ACAF fail-closed 默认强校验下沉 `AgentLoopController`（2026-09-04 闭合）。入口：[P0-GOV 收口审计](docs/audits/P0_GOV_UNCOMMITTED_REVIEW_HANDLING_2026-09-04.md)。
- [x] 任务 D 双实现终局治理（方案 α：Rust 单一执法 `journal-conformance` CLI + Python 冻结 reference 对拍对照面）——batch-1 + S2a/S2b/S2c/S2d + S3/S4 翻转全部闭合（2026-09-06）。入口：[Task D 批次 1 审计](docs/audits/P0_GOV_TASK_D_DUAL_IMPL_GOVERNANCE_2026-09-04.md) / [S3/S4 翻转实施审计](docs/audits/TASK_D_S3_S4_FLIP_IMPL_AUDIT_2026-09-06.md)。


### P0-GOV GLM 外部只读审查处置（2026-09-06 用户裁决；**同日全部闭合**）

- [x] F1 skills 豁免收窄为注册技能根白名单（orz `67b51eb1`）；R-1 353 个已跟踪本地运行产物转本地件（另 1 个误中夹具恢复）；R-2 manifest 生成器显式 LF 重算；R-3 九个根目录一次性产物归档 `存档/root-artifacts-2026-09-06/`（gsa.py 门禁 required 保留）。（2026-09-06 闭合）
- [x] F2 approval prompter 存根登记 GAP-APPROVAL-PROMPTER（排期后同日延期，见 P0-0n）；观察项 (c) 权限判定分散登记 OBS-PERMISSION-DUAL-IMPL；复核修正批 + GAP-GSA-SYMLINK-STALE-TEST 登记并同日用户裁决收口（对齐 Task C，orz `a29f7377`）。
- 入口：[GLM 登记审计](docs/audits/GLM_EXTERNAL_REVIEW_REGISTRATION_2026-09-04.md) / [处置 + S2 排期审计](docs/audits/P0_GOV_GLM_DISPOSITION_AND_TASK_D_S2_SCHEDULE_2026-09-06.md)。


### P0-E 评测冒烟暴露问题（2026-08-17 登记；2026-08-18 全部闭合）

- [x] 全部闭合：ACAF 容器内供应 / console 工具名下划线 / plan_write 校验消息形状 / actions 形状探针锁定 / 计划视图步骤 ID / 订单发放前拒绝入事件面 / grep 搜索范围契约（结构化信封 + 结局三型 + hidden/no_ignore + 静态 rg）/ list_dir 范围计数 / grep files_searched 全结局探针。入口：BACKLOG 0a / ADR-0010 §14.21/§14.23 / 对应实施审计（GAP_ACAF_HARNESS_PASSTHROUGH、GAP_CACHE_CONTEXT_COST 等）。


### P0-0c LEDGER-FOLD-EXTERNAL-FILE（S1-S4 全部闭合 2026-08-19，计数 29 → 28）

- [x] 全部闭合：外挂台账文件 + 固定指针消息 + 写失败降级（ledger_fold_write_failed 事件）+ B 定案（机械压缩零模型调用）+ D1=(c) HA 结构化事实聚合 + 黑板读取缓存成本（receipt_id 按需点读）+ 折叠桥接截断；S4 复验 provider 口径命中率 95.33% ≥90%、零 400、重付/截断达标。入口：[设计](docs/LEDGER_FOLD_EXTERNAL_FILE_DESIGN_2026-08-18.md) / ADR-0010 §14.28–§14.32 / BACKLOG 0c。


### P0-0d OUTPUT-DEGENERATION-GUARD（S1-S4 全部闭合 2026-08-20）

- [x] 全部闭合：make-doom 退化复读失败防护——8K 全统一 + 补读闭环 + 实时检测 + 32K；S4 复验 95.28% 命中率、哨兵零误杀。入口：BACKLOG 0d / ADR-0010 §14.33。


### P0-0d 后续：STREAM-RETRY-RHYTHM（2026-08-25 取代归档）

- [x] 原定案被后续 3（180s 窗口）+ OUTPUT-BUDGET（idle 50s→30s）取代，归档不实施。入口：[STREAM_RETRY_RHYTHM_DESIGN](docs/STREAM_RETRY_RHYTHM_DESIGN_2026-08-20.md) / BACKLOG 0d。


### P0-0d 后续：OUTPUT-BUDGET-RESTORE-AND-STALL-GUARD（S1-S4 全部闭合 2026-08-20，29 → 28）

- [x] 全部闭合：`REQUEST_MAX_TOKENS` 32K→256K（回落 128K）+ D-6 空流链官方化收窄 + 输出健康哨兵（content/reasoning/tool、reasoning 复读、stall 600s/64K、idle 30s）+ DEGENERATION_LIMIT=3 三族共享；S4 复验命中率 ≥90%、零误杀。入口：[设计](docs/DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md) / ADR-0010 §14.35 / BACKLOG 0d。


### P0-0d 后续 2：THINKING-DEFAULT-HIGH-LADDER（S3/S4 换题复验闭环 2026-08-20，29 → 28）

- [x] 全部闭合：默认 high + low 中间档降级梯（high→low→disabled→失败，max 保留显式档）；S4 换题 make-doom-for-mips 复验（零异常、零 400、命中率 92.18%、哨兵/stall 全零触发、每轮更快成本更低）。入口：设计 §3.6/§4.4–§4.7 / ADR-0010 §14.35 第 5–8 项 / BACKLOG 0d。


### P0-0d 后续 6：REPETITION-DETECTOR-ROLLING-HASH（S1-S4 全部闭合 2026-08-23，计数 29 → 28）

- [x] 全部闭合：路径①替换为滑动窗口滚动哈希任意偏移检测（144 字符缓冲、任意周期命中、DNA 正常序列免疫、O(1)/字符）+ 3-gram 路径②兜底；S3 重建 + S4 dna-assembly 复验闭环（低熵误杀消除、真复读仍触发、零 400）。入口：设计 §3.3/§4.8 / ADR-0010 §14.35 第 13 项 / BACKLOG 0d。


### P0-0d 后续 7：AGENT-DELIVERY-FLOW（S4 复验闭环 2026-08-23，计数 29 → 28）

- [x] 全部闭合：计划无空转 + 末步机械递交（submit 双阶段 requested→confirmed）+ 引用修正一次/二次阻断 + 订单反馈 receipt 点读链；S4 复用 NGRAM S4 实机复验（8/8 完成试次走 submit 双阶段、零 400、命中率全 ≥90%）。入口：ADR-0010 §14.35 第 19 项 / BACKLOG 0d 后续 7。


### P0-0d 后续 8：NGRAM-GUARD-CALIBRATION（S4 复验闭环 2026-08-23，计数 30 → 29）

- [x] 全部闭合：3-gram 门槛 3→15 校准 + 复读滚动哈希边界对齐；S4 实机复验闭环。入口：BACKLOG 0d 后续 8 / 设计 §4.11。


### P0-0e CONTEXT-SCAFFOLDING-PULL-REDESIGN（S1-S4 验证闭环 2026-08-21，29 → 28）

- [x] 全部闭合：预算块 PUSH→PULL + 工具输出汇总消息退役（方案 C 维持 256K 暂不收紧，用户裁决）；命中率 94.45%、零哨兵触发、输入增长放缓。入口：ADR-0010 §14.33 / BACKLOG 0e。


### P0-0f FUS-READ-ANCHOR-WRITE-GUARD（S4 复验闭环 2026-08-23，计数 28 → 27）

- [x] 全部闭合：read_file 内容锚点下传（sha256/size/mtime）+ search_replace 写前机械核证（content_anchor_mismatch 拒绝、重读后重试）；S4 复用 NGRAM S4 实机复验（10 试次零误拒、锚点实机可见、命中率 94.11%–98.55% 全 ≥90%、零 400）。入口：ADR-0010 §14.38 / BACKLOG 0f。


### P0-0g MECHANICAL-AUDIT-LAYER（S4 复验闭环 2026-08-25，计数 31 → 30）

- [x] 全部闭合：首轮 plan 门保留 + direct 执行面 + 半助理层 + 静默机械审查层（每对象仅最后一轮结果覆盖写、不给建议、报告随最终答案前中立问询轮注入）+ 检索恢复 + 引用校验器删除 + 读范围放开。入口：ADR-0010 §14.39 / BACKLOG 0g。


### P0-0h RETRIEVAL-SUBAGENT-WIRING（S4 复验闭环 2026-08-25，计数 30 → 29）

- [x] 全部闭合：外部子代理模式 A 自动定档 + 内部子代理结构化检索外包（retrieve_project_docs 触发面）+ prompt tips + harness 传参；S4 单道检索题 mteb-leaderboard k=1 reward 1.00、事件链完整性 100%、零 400。入口：ADR-0010 §14.46 相关 / BACKLOG 0h。


### P0-0i FINAL-SMOKE-2026-08-25 对拍暴露问题（2026-08-25 登记；2026-08-26 全部闭合）

- [x] GAP-EVENT-SCHEMA-DRIFT：三类 Schema 漂移修复完成并复验（retrieval_mode_transition 枚举补全、ledger_fold_advance view_estimate_after、control_ticket_issued activation_id 放开；事件链复验 5 run 非终止错误 0）。入口：BACKLOG 0i / 实施审计。
- [x] GAP-REPETITION-DETECTOR-DNA-FALSE-POSITIVE（S1-S4 全部闭合 2026-08-26，30 → 29）：序列内容门（L=400 维持 + 无切分点 sequence_kind 判定 + 序列/蛋白双族独立阈值 + 命中 3→5）；S4 复验闭环（EGFP 式合法引用 1–4/5 命中零误杀仅审计、真复读 5/5 仍触发降 EnabledLow、零真实 400、命中率 82.36% 持平）。观察项：dna 82.36% / feal 88.21% 命中率 <90% 与 web 检索注入相关，成本观察不阻塞（并入对拍审计记录）。入口：BACKLOG 0i / 序列内容门设计 / ADR-0010 §14.41。


### P0-0k RETRIEVAL-ORCHESTRATION-MECHANICAL（P0；2026-08-30 定稿；第一批 + 第二批实施 + S4 实机复验闭环 2026-08-31）

- [x] 全部闭合（2026-08-31）：双模式定案（local_browser 可用仅 browser_read / 不可用仅 web 族）+ 引擎 SERP Google 主序 + 原生兜底 + 第一批五项（S1/S2 + S3 重建 + S4 实机复验）+ Google 门禁观察实验（多轮实机）+ 主面封存 browser_read + 第二批（project_doc_index v2 / 会话级 tab 池 + 同轮多页并行 + DNS 缓存 / 委托契约复杂度分档）S1/S2 + 方向 C（删除 [RESULT_JSON] 组织块契约）S4 实机复验闭环（未闭合 32 → 30）。入口：BACKLOG 0k / ADR-0010 §14.45/§14.46 / S4 复验记录。


### P0-0m GSA-SESSION-VOLUME-BOTTOM-LAYER（P0；2026-09-06 设计定稿，同日用户裁决放行，排期实施；**2026-09-19 S4 收口闭合，37 → 36**）

用户裁决：`.gsa` 为 LIF 科学性组件，保留并下沉为底层部件；权限层不裁撤
（后续「助理层拦截系统核心路径、仅删除保护」另行立项），放开压到最窄。
排期（2026-09-06 放行）：S1 代码先行，S1–S4 独立审计 + 独立提交；S3 复验
吸收 GAP-GSA-SYMLINK-STALE-TEST 连带观察（`.gsa` terminal-log 白名单会话卷
形态豁免）。

- [x] S1 代码：SessionVolume 资源 + host 装配 canonical 单源注入 + 沙箱
  三分判定 + 窗口契约下沉（terminal-log / run_tests 只读窗口）+
  gitignore 绕过 + permission.rs `.gsa` 段退役标注。——**2026-09-07
  完成**（orz-tools `SessionVolumeRoot` 资源 + `is_path_allowed_for_read`
  三分单点 + D3 窗口契约双窗口；SessionContext 加 `session_volume_root`
  字段，host `build_toolset` 装配期一次 symlink-aware canonical 注入，
  上游 agent/workspace 传 None fail-closed；read_file 域内 deny 文案
  区分 + D4 gitignore 绕过；permission.rs `.gsa` 段 RETIRED-IN-PLACE
  注记保留不演进。workspace check 零警告、clippy 新增零告警、fmt 净）。
- [x] S2 测试：11 项测试矩阵 + 全量回归。——**2026-09-07 完成**（纯函数
  8 + 工具级 5 新测试，矩阵 11 项双层覆盖 + run_tests 名字 symlink 顶替
  附加负测；orz-tools 2829 / orz-loop 729 / orz-agent 573 / orz-workspace
  22 全绿；orz-host 214 passed / 34 failed 与 stash 基线失败集逐项 diff
  完全一致（ACAF signer 存量失败族 + 1 挂死均存量，登记观察）；Python
  assurance 无 `.gsa` 判定不受影响。入口：
  docs/audits/P0_0M_GSA_SESSION_VOLUME_S1_S2_IMPL_AUDIT_2026-09-07.md）。
  同日 S1 三路复审 + 全部问题处理收口（P2×2 + P3×6，orz-tools 2832 全绿；
  入口：docs/audits/P0_0M_S1_S2_REVIEW_HANDLING_2026-09-07.md）。复审遗留
  两项既有观察同日处理：ACAF 默认翻转下游测试面族闭合（orz-host 249/0/4
  无跳过 + acaf_e2e 23/23，入口：docs/audits/ACAF_TEST_DEFAULT_FLIP_INFRA_
  FIX_2026-09-07.md）；OBS 权限双实现摸底 + 同日用户裁决定稿方向 = α
  （判定面单图 + 休眠面冻结标注，纯文档批；β/γ 否决——自研面不膨胀、
  薄层哲学、桥接为特意形态），五项推荐确认后 α 同日实施完成（单图
  docs/PERMISSION_JUDGMENT_SURFACE_MAP.md + 标注 14 文件 + ADR §14.60 +
  索引 v2.61，零行为变更，OBS 终态已管控；入口：
  docs/PERMISSION_DUAL_IMPL_CONVERGENCE_DESIGN_2026-09-07.md）。
- [x] S3 接线复验：补读链/run_tests 窗口端到端（`.gsa` symlink 会话卷
  实机构造）+ GAP-GSA-SYMLINK-STALE-TEST 演进注记。——**2026-09-07 完成**
  （经 P0-0o T1/批次 0 实机复验达成；BACKLOG 0m S3 行同日注记）。
- [x] S4 收口：索引/BACKLOG/TODO 同步 + 门禁 Exit 0。——**2026-09-19 完成**
  （过夜批随 0ar S2／0aq 处置同批落账，计数 37 → 36；报告
  [`0AR_S2_RS_MECHANICAL_0M_S4_OVERNIGHT_2026-09-19`](docs/audits/0AR_S2_RS_MECHANICAL_0M_S4_OVERNIGHT_2026-09-19.md)）。

入口：[设计](docs/GSA_SESSION_VOLUME_BOTTOM_LAYER_DESIGN_2026-09-06.md) /
[ADR-0010 §14.56](adr/ADR-0010-vol-14-addenda-index.md) /
[BACKLOG 0m](docs/BACKLOG_AND_PRIORITIES.md)。


### P0-0p 模型自信息面补强与 `.gsa` 两段门（2026-09-07 设计定稿同日排期；**T0/S1/S2/T2/S4/S5 全部闭合 2026-09-08**；BACKLOG 0p 转 `implemented`）

> 设计：[`BLACKBOARD_SELF_HISTORY_AND_GSA_TWO_STAGE_GATE_DESIGN_2026-09-07`](docs/BLACKBOARD_SELF_HISTORY_AND_GSA_TWO_STAGE_GATE_DESIGN_2026-09-07.md) / ADR-0010 §14.61；索引：AUTH-BLACKBOARD-SELF-HISTORY-GSA-GATE。完整勾选与实施流水见全量快照 [`TODO_FULL_2026-09-09`](存档/todo/TODO_FULL_2026-09-09.md)。

- [x] T0：orz 版本 bump 0.3.2（`7b00bbc9`）。
- [x] S1 黑板补强：`failures_only` 聚合面 + `search` 字面检索（≤20 行）+ 工具描述教学 + turn_count 真实计数（orz `928dceb3` + 复审处理 `fd46d4f9`；F-C 治本转 P1-0q）。（2026-09-07 闭合）
- [x] S2 两段门：内部区首读通知信封 → 二读放行 + 状态会话卷级持久化 + key 拦截五漏斗全卷零 sk- + 结构化 policy_denial（orz `7d7d89e7` + 复审处理 `542c35d5`）。入口：[复审处理审计](docs/audits/0P_S2_REVIEW_HANDLING_2026-09-07.md)。（2026-09-07 闭合）
- [x] T2 双平台重建：Windows 三件套 0.3.2 哈希锁定 + Linux musl 翻新 + VM 换装 + DryRun 全对 + enforcement-probe high-nist 19/19。入口：[T2 审计](docs/audits/0P_T2_DUAL_PLATFORM_REBUILD_2026-09-08.md)。（2026-09-08 闭合）
- [x] S4 重跑 train-fasttext（tf-selfhistory-032）：判据表全项通过（两段门审计对首次生产落账 / 全卷零 sk- / 命中率 96.37%）；任务未过 = 环境缺 fasttext（→ 0r）。入口：[S4 分析](docs/audits/0P_S4_TRAIN_FASTTEXT_TF_SELFHISTORY_032_ANALYSIS_2026-09-08.md)。（2026-09-08 闭合）
- [x] S5 收口：BACKLOG/TODO/索引 v2.63/ADR-0010 §14.62 同步 + manifest 重算 + 门禁 Exit 0。（2026-09-08 闭合）


### P0-0v 检索引擎 SERP 接入与 `browser_control` 车道分类修正（2026-09-10 立项；**第二批（软备忘 + 0v-A 取证面合批）S1–S4 全部执行完毕（S4 判据 1/2/6/7/11/12 成立、8 成立、3/4/5 部分成立、9/10 未取得）；0v 闭合与判据遗留/0v-C 缺口立项留用户裁决**）

> 裁决：navigate 被拒应满足模型需求 + 坐实 DeepSeek 后端慢（调查已闭合）+ 按既有 SERP 设计补实现（候选 Bing）；区域固定 `en-US`；Bing 无登录态污染专项优化；低质量域名加权复用既有 `SourceWeightConfig`（标注 + 稳定排序，不硬过滤，不承担恶意域识别）。
> 设计：[`RETRIEVAL_ENGINE_SERP_SEARCH_ACTION_DESIGN_2026-09-10`](docs/RETRIEVAL_ENGINE_SERP_SEARCH_ACTION_DESIGN_2026-09-10.md)；BACKLOG 0v。

- [x] S1 代码（2026-09-10 完成）：`risk_class` browser_control ReadOnly 豁免 + `search` 动作（引擎链/失败备忘/SERP 有机结果提取/URL 解码/pacing/信封）+ 低质量域名加权接线（`SourceWeightConfig` 注入 search 编排）。
- [x] S2 测试（2026-09-10 完成）：纯函数单测（引擎表/失败备忘/可用引擎选择/pacing 上下限/解析与 CAPTCHA/URL 解码/加权稳定排序/字段上限/固定表达式 Bing 广告排除）+ 动作信封与参数校验 + 搜索上限失败态 + 车道门测试（`write_gate None`、ReadOnly、action_category=read）。S2 发现并修复首次 search 误等 5s 冷却基值的实际缺陷。
- [x] 复审第 1 轮（2026-09-10，设计 §5.3）：P1-1 Bing 跳转解码（`a1<base64url>` + URL 校验 + 原始链接回落）、P2-1 `engine_attempts` 引擎标注（成功/失败/跳过都显式）、P2-2 取消整包 8KiB 截断改字段上限（url 2048 取代整包截断）、P2-3 反污染下沉（启动层 `--window-size` + 会话层尽力而为、白名单收窄）、P2-4 车道 SERP 预算方案 A。
- [x] 复审第 2 轮（2026-09-10，设计 §5.4，P2-4 专项）：P1-1 拒绝信封字段补 schema 登记 + 2 个 fixture 进 `check_repository`；P2-1 会话上限改"检查点式（最坏 40+2）"措辞；P2-2 用量挂激活（`serp_navigations_used` 随 sidecar／continue／每条路径回写）；P2-3 会话检索底线额度（floor 16，宿主报事实、loop 施加策略）；P2-6 票据分支补预留回滚；P3-1/P3-2/P3-3 注释与边界登记。回归：loop lib 757/0/3、host lib 286/0/5、workspace check 无告警、run-event conformance 15 passed。
- [x] S3 双平台重建（2026-09-11 完成，**随 0x S3 同批**）：0v S1–S2 代码随本批版本 bump（0.4.0 → 0.4.1）一并进载体；三件套重建 + 冒烟绿 + 符号核证（`browser_control` 21→75 / 25→59、`retrieval_enabled` 5/8、车道标注 2/2）见 [0X S3 重建记录](docs/audits/0X_S3_DUAL_PLATFORM_REBUILD_2026-09-11.md) §3。
- [x] S4 实机复验（2026-09-11 首轮**未通过、受阻**）：与 0x S4 同场同题。**F1（框架缺陷）**：3 次 `browser_control {action: search}`（seq 24/264/384）全部 `permission_decision=deny`（无 `tool_started`，日志三次 `prompter ... channel closed`）——0v S1 只改控制器侧 `orz-loop/src/tool.rs::risk_class`（→ ReadOnly），未同步宿主侧 `orz-host/src/permission.rs::access_kind`（480 行起），`browser_control` 落 `AccessKind::Edit` 兜底 → 无头确定性拒；同车道 `browser_read` 5/5 `allow_once`（有 `Read(None)` 分支，543 行）即对照。**F2（装置）**：Chromium 引导 600s 超时（143MB/246MB）+ PATH 命中 snap 桩 → `browser_launch_result` 5/5 failure。判定：判据 1/5/6/7 未观察、2 与 4 部分成立。**修复方向（已按用户裁决处理，见下两条）**：`access_kind` 增 `browser_control` 映射（按 action 分档）+ headless fail-closed 测试；装置侧提高引导超时或预置浏览器。入口：[0X/0V S4 实机复验记录](docs/audits/0X_0V_S4_LIVE_VERIFICATION_2026-09-11.md)。（**2026-09-12 用户裁决闭合入账 26 → 25**：判据 9/10 与 CAPTCHA 样本不再追——本框系 0v 闭合后残留未勾选项，随 0ab S1 对账补勾，无新增实施。）
- [x] **F1 修复落码（2026-09-11 用户裁决放行，orz `340fe4a7`）**：`access_kind` 增 `browser_control` 分支按动作分档（七种现行动作 → `Read(None)`，未知/Phase 2 交互动作 → `Edit` fail-closed）；同刀补跨表护栏测试 `read_only_tools_never_fall_into_the_edit_bucket`（11 组样本，把「双面修一面」变成机械可查）。验证：orz-host lib 287/0/5（单线程）、permission 20 全绿、fmt 干净、新增代码 clippy 零告警。
- [x] **F2 装置侧改造（2026-09-11 用户裁决：改用容器内真实 chromium）**：宿主侧经代理一次性取官方 Chromium 快照（rev `1696156`、246,549,653 B）解压到 `D:\tb-eval\browser\chrome-linux\`，跑批时只读挂到 `/opt/chrome-linux`（命中装置既有判定即复用，不再每次现下）；容器内实测 `Chromium 155.0.8053.0` 启动正常。
- [x] **S3′ 载体重建（2026-09-11 完成）**：F1 修复落在 0.4.1 基线之后 → 先 bump **0.4.1 → 0.4.2**（orz `b81c90ac`）冻结源基线，再双平台重建——Windows 宿主 release（`CARGO_EXIT=0`，38.68s）+ Linux musl（官方源变体，`BUILD_EXIT=0`，28m43s）三件套；载体刷新为 0.4.2、ELF 三件 PT_INTERP=0、双向加载冒烟绿、接线符号双平台全命中、manifest 1441 条 + 门禁 `valid: true`。**F1 已进载体；S4 复跑待放行。** 入口：[0v S4 复跑重建记录](docs/audits/0V_S4_REFRESH_REBUILD_2026-09-11.md)。
- [x] **S4 前置连通性预检（2026-09-11 完成，不消耗跑批额度）**：①**F2 成立**——评测镜像 Ubuntu 24.04 本体缺 25 个共享库（只挂 chrome 目录起不来），但装置依赖安装位于预挂载判断**之前**，按序实测装完依赖后 `missing=0`、`Chromium 155.0.8053.0` 正常。②**引擎链前提成立**——Google 120s 超时 + DOM 0 字节（DNS 污染 `2001::1`）、DDG 不可达、**Bing 返回 164 KB 真实有机 SERP**。③`ol#b_results > li.b_algo` 选择器命中（8 条），无 `b_ad`/challenge/consent，链接为**直链**（`ck/a`=0、`u=a1`=0 → P1-1 解码路径自然不可达，透传正确）。④判据可及性：**1/6/7 有样本**、2/4 可续证、**5 不定**、**9/10 需定向探针**（需单次激活 ≥9 次引擎导航）。入口：[S4 连通性预检](docs/audits/0V_S4_CONNECTIVITY_PRECHECK_2026-09-11.md)。
- [x] **S4 复跑（2026-09-12 完成）**：载体 0.4.2、同题同口径。①**F1/F2 实机确认修复**（`browser_control` 3 调用 0 执行 → **8 调用 8 执行**、零拒绝；`browser_launch_result=success`、`origin=Env`）。②**0x 再次通过**（initial_round 恰好 1 次 / seq=22 / 63 条审查不含三问）。③0v 本体未取得：判据 6 ✓、判据 7 机制面 ✓（`tier`/`mechanical_weight`/`weight_reason` 已在生产）、判据 2 改善至 1、判据 4 命中率 94.36% ✓，**判据 1 仅部分、5/9/10 无样本**。④**F3**：抽取竞态已排除（load 时点 Bing SERP 已有 `li.b_algo`×10）、通用出口正常（`web_fetch` 取回 PLOS 86KB）、Google/DDG 无提交成功导航记录；形态与**会话级失败备忘**一致（首搜 44.7s 真导航，其后 3 次 0.9–3.9s 零导航）。⑤**缺口**：`engine_attempts`/`error_class`/`low_quality` **无持久化面** → 判据 1/5/7 不可事后取证（**已登记为 0v-A**）。入口：[0v S4 复跑记录](docs/audits/0V_S4_RERUN_2026-09-12.md)。
- [x] **设计定稿 + 实施放行 + 排期（2026-09-12 用户裁决）**：软备忘（设计 §6 备选「只调整顺序」）+ **0v-A 引擎级取证面** **合批实施**，**0v-B 定向探针挂 S4**。合批理由：同一片 `local_browser` 代码面 + 同一次载体重建 + 同一次实机复跑。§6 的「完全去备忘」被否（本环境 Google 被 URL gate 确定性拦，代价为**每次** +30s）。排期与判据见 [设计 §8.7](docs/RETRIEVAL_ENGINE_SERP_SEARCH_ACTION_DESIGN_2026-09-10.md)。
- [x] **S1 落码（三步，2026-09-12 完成）**：①**取证面形态定案 = 会话卷落盘** `runs/<run>/serp-attempts/<round>.json`（loop 层 `host_exec.rs::persist_serp_attempts` 在 P2-4 预算结算点旁路写入、orz-secrets 脱敏漏斗、同轮顺延后缀；`wall_ms` 定案 = 信封 `engine_attempts[]` 加性字段，取证文件与模型所见逐字同源；派发前拒绝/宿主错误无引擎事实不落文件）。②**软备忘语义**：`available_engines()` → `ordered_engines()`（头/尾两段各保链序、全员置尾退回链序）；失败记录降级纯排序依据（**澄清：成功不清除备忘**，置尾持续全会话）；`skipped` → `not_attempted`；cdp.rs 链循环「按序尝试直至成功」、成功时未触及引擎显式 `not_attempted`；`all_engines_failed` 收紧（构造保证三引擎均真实尝试）。③**取证面落码**（旁路，失败只 WARN）。改写 1 条既有单测（`ordered_engines_demotes_failures_to_the_tail_without_removal`）。回归：orz-host lib 287/0/5（单线程）、orz-loop lib 763/0/3、fmt 干净、新增代码 clippy 零告警。**同日三面复审处理（§9）：O-1 用户裁决不做增量（Empty 置尾罕见且自限、重试由上限/冷却/预算兜底，登记闭合、S4 凭证据可重开）；O-2 用户裁决实施（cap_exceeded 信封补全量三引擎 `not_attempted` 表 + caller-supplied reason，预算结算读数不变、取证面自动覆盖上限路径），287/0/5 + 763/0/3 复绿。** 入口：[设计 §8.7 S1 实施记录 + §9 复审处理](docs/RETRIEVAL_ENGINE_SERP_SEARCH_ACTION_DESIGN_2026-09-10.md)。
- [x] **S2 测试与合约（2026-09-12 完成，orz `ee4ef617`）**：改写 1 条既有单测（`session_state_memoizes_failures_and_caps_navigations` → 断言「备忘不删除 + 上限仍在」）+ 新增 5 条测试函数覆盖 4 项软备忘语义（置尾后再失败保持队尾 / 成功时后续引擎记 `not_attempted`——同刀把成功路径标注循环抽为行为等价纯函数 `mark_unreached_as_not_attempted` / 「三引擎全失败才 `all_engines_failed`」与「同会话先前失败引擎仍被真实重试」合落于一条离线确定性链测试：DNS 缓存预热 + 浏览器 WS 拒连固定失败点，导航计数 3→6 证真实重试）+ 取证面 2 条（文件↔调用↔journal 三面对应含同轮 `-2` 后缀与逐字同源信封、机械读数逐字段；落盘失败只 WARN 不影响工具结果）；合约面核查 `skipped`/`engine_attempts` 在 schema/fixtures 零命中 → 无同步项、不新增事件族。回归：orz-host lib 290/0/5（单线程）、orz-loop lib 765/0/3、fmt 干净、新增代码 clippy 零告警、`check_repository` `valid: true`（manifest 重算 1441 条）。登记观察（不动码）：`begin_search` pacing 在冷却过期后仍无条件叠加 0–2.5s jitter，是否设计意图留用户裁决。**同日三面复审（两子代理 + 本批裁决）pass、P1×2/P2 与可处理 P3 全部收口（脱敏漏斗钉字 + lane=null + fixture 文案，orz `b1e9ac65`，orz-loop lib 767/0/3）**，剩余 P3（成功路径接线/抽取等价性/常量硬复制）登记随 S4 或永久登记。入口：[设计 §8.7 S2 实施记录 + 复审处理](docs/RETRIEVAL_ENGINE_SERP_SEARCH_ACTION_DESIGN_2026-09-10.md)。
- [x] **S3 双平台重建（2026-09-12 完成，orz `f9fb70e4` bump + 载体刷新）**：版本 bump **0.4.2 → 0.4.3**（两文件两行）冻结源基线；Windows 宿主 release（10m51s，`CARGO_EXIT=0`，orz.exe +164,864 B 即软备忘+取证面增量）+ Linux musl（`BUILD_EXIT=0`，20m39s，orz +153,032 B）三件套；staging/SHA256SUMS + 载体 `D:\tb-eval\orz-windows` / `orz-linux` 刷新且哈希逐对吻合（0.4.2 Linux 三件已 `.0.4.2.bak` 备份）；ELF 三件 `PT_INTERP=0`（static-pie OK）；**双向加载冒烟绿（bookworm glibc + alpine musl，三件预期 exit 1 形态一致）**；接线符号双平台全命中（`not_attempted`/链成功固定 reason/ceiling reason/收紧后 all_engines_failed 文案/`serp-attempts` 取证面串 ×48/15、`0.4.3`、`low_quality` 11→23、`engine_attempts` 2→14；`ordered_engines` 仅 Linux debuginfo 段命中 1、`initial_round_inquiry` 双平台 0 均与上批同形非缺陷）；manifest 重算 1441 条（差异面恰 bump 两行）+ 门禁 `valid: true`。操作沉淀：Git Bash 下 docker `-w /orz/orz` 须 `MSYS_NO_PATHCONV=1`（MSYS 路径转换致首次瞬败）。入口：[S3 重建记录](docs/audits/0V_S3_BATCH2_DUAL_PLATFORM_REBUILD_2026-09-12.md)。
- [x] **S4 实机复验（含 0v-B 定向探针，2026-09-12 完成，载体 0.4.3）**：**(a)** dna-assembly 复跑（33m12s / 334 事件，与 0v S4 复跑逐条对照）+ **(b)** 0v-B 定向探针（`D:/tb-eval/probe-0vb` 固定 12 查询，61m41s / 1348 事件 / 27 份取证文件；首次因 harbor 本地任务缺 environment/ 目录瞬败，补目录 + `--disable-verification` 后成立）。**判据结果：1/2/6/7/11/12 成立（1/7/11/12 决定性——探针 26 次 search 成功路径 google 恒 `not_attempted` 且真实尝试恒 1 次；`not_attempted` 53 条/`skipped` 0 条；low_quality 43 条零排序违例；两 run 被墙钟杀死而 `serp-attempts/*.json` 完整可复核）、8 成立（0.4.2 journal 经 0.4.3 法官 OK）、3/4/5 部分成立（命中率 81.66%/87.71% <90%、CAPTCHA 无样本、chrome-error 归类 blocked 偏差——三项观察登记）、9/10 未取得（模型拆多 activation 未打穿 16、主车道无调用）**。0x 搭车两场再通过且周期/初始问询互不影响获双会话样本。新缺口候选 0v-C（墙钟杀死 run 的 journal 链断——external 接缝 digest×2 + sha×5/×2 + terminal 缺失；正常结束历史卷全部链完整）与 0v-B 打穿策略留用户裁决。入口：[S4 复验记录](docs/audits/0V_S4_BATCH2_LIVE_VERIFICATION_2026-09-12.md)。


### P0-0x 初始轮中立问询（2026-09-11 用户裁决立项；**S1–S4 全部闭合 2026-09-11，转 `implemented`，计数 27 → 26**）

> 来源：TB 4.0 探针审计 O1（agent 4h53m 优化了与验收口径不同的代理指标，方向性错误直到收尾才暴露；与「过度自制」同源）。
> 设计权威：[`INITIAL_ORIENTATION_DECLARATION_DESIGN_2026-09-11`](docs/INITIAL_ORIENTATION_DECLARATION_DESIGN_2026-09-11.md)；ADR-0010 §14.66 / v1.67；BACKLOG 0x。

- [x] 设计定稿（2026-09-11，含同日两轮复核，**无待裁决项**）：复用中立问询软门/票据/事件面/pending 闸，在**首轮动作批次结束**（`post_tool_batch_gap`）一次性机械注入三问（交付物与判定口径 / 大方向与阶段 / 做法优劣与任务评估）；**不携带机械审查报告**（审查依旧只在结尾）；**不在周期问询里加问**（动作中只回看与确定、不质疑）；不落黑板、不做消费审计（「只要让模型想了那就足够」）、复用 `OrientationV1` 票据；无工具首轮顺延边界已认可。
- [x] S1 实施（2026-09-11 完成）：`INITIAL_ROUND_INQUIRY_BLOCK` 常量 + 注入块前缀登记（`is_injected_block_text`）+ 会话一次性触发状态（`initial_round_fired`，`#[serde(default)]` 兼容旧侧车）+ 控制器分派（`maybe_fire_orientation` 统一出口；触发判定落点相对设计的更正已登记）+ 测试矩阵最小集（一次性/顺延/不互扰/软门不禁工具/纯文本消费/审查报告与反例门不变且不含三问）；受影响既有测试同步（orz-loop 2 例、orz-host 8 例）。入口：[S1 实施记录](docs/audits/0X_S1_INITIAL_ROUND_INQUIRY_IMPL_2026-09-11.md)。（同刀顺带修复：`orz-bin` 测试目标 0v S2 遗留编译缺口 + 三条守卫测试的 ACAF 环境隔离。）
- [x] S2 实施（2026-09-11 完成）：①payload 正例 `orientation-checkpoint.initial-round.valid`（负例沿用 `constraint.invalid`；生成器条目 + `check_repository` 登记齐全）；②第 **34** 族法官 `initial_round_inquiry`（Rust 执法 + Python 冻结镜像，跨执法面逐格零差；5 个合成场景 = 1 正 4 负）；③`orientation-fire-run.jsonl` 期刊重捕（两条 fire：initial_round + completed_turns_interval）与两侧期望序列/断言同步；④**签名侧第二模板摘要**——signer 持两块内置模板 + `template_sha256_initial_round`，`sign_orientation_v1` 用可选 `trigger` 选模板、`check 2` 用同一 trigger 比对（kind + 摘要匹配）；⑤门禁 `valid: true`。入口：[S2 实施记录](docs/audits/0X_S2_EVENT_FACE_AND_SIGNER_2026-09-11.md)。（同刀修复：`family_stage_tamper_detected_end_to_end` 预存在失败改锁新族端到端；生成器补登 0v 两条会被重跑静默删除的 fixture 条目；capture 的 ACAF 环境说明。）
- [x] S1/S2 全面复审处理（2026-09-11 完成，三面复审结论：整体成立、符合度高）：2 处文档一致性更正（`TODO.md` 路由行状态；设计 §5-6 / ADR §14.66 第 4 项⑥「Python 镜像」指针改为 `run_event_journal_validation.py`，v0.1 `orientation_runtime_guard.py` 保持冻结）+ 2 处测试补强（设计 §5-7「中断后恢复重触发」端到端钉子 `initial_round_refires_next_run_when_the_fire_was_not_consumed`；签名侧跨 trigger 负例 `template_mismatch`）；**无代码语义改动**。入口：[S2 复审处理](docs/audits/0X_S2_REVIEW_HANDLING_2026-09-11.md)（子模块 `dcf4a774`）。
- [x] S3 双平台重建（2026-09-11 完成）：**版本 bump 0.4.0 → 0.4.1**（orz `a6f902ef`）冻结本批源基线 → Windows 宿主 release（1m38s）+ Linux musl（rust:1.97-slim 官方源变体，冷缓存全量 41m04s）三件套；`SHA256SUMS` 两份 + staging→载体同步哈希逐对吻合；ELF 三件 PT_INTERP=0（musl static-pie）；双向加载冒烟全过；接线符号命中（`[INITIAL_ROUND_INQUIRY v0.1]` 各 1、`post_tool_batch_gap` 7/7、`template_sha256_initial_round` 1/1）；manifest 重算 1441 条 + 门禁 `valid: true`。入口：[S3 重建记录](docs/audits/0X_S3_DUAL_PLATFORM_REBUILD_2026-09-11.md)。
- [x] S4 实机复验（2026-09-11 完成，**判据通过**）：载体 orz 0.4.1，单题 `dna-assembly`、k=1、官方墙钟（31m35s / 50 模型轮 / 407 事件）。`trigger=initial_round` 恰好 1 次（seq=12）、`post_tool_batch_gap`、在首个动作批次（seq=9）之后、块前缀与三问逐字命中、50 轮内无复发、41 条机械审查不含三问、无黑板锚点。边界：主车道仅 2 轮 → 周期问询未触发，「互不影响」由 S1/S2 测试覆盖，本轮为单会话样本。入口：[0X/0V S4 实机复验记录](docs/audits/0X_0V_S4_LIVE_VERIFICATION_2026-09-11.md)。
- [x] **闭合入账（2026-09-11 用户裁决）**：S1–S4 全部闭合、S4 实机判据通过 → 转 `implemented`，未闭合 **27 → 26**。



### P0-0ac GAP-MECH-IMMEDIATE-FEEDBACK 机械层即时回报与流式检索（2026-09-13 用户裁决登记；S1 探针 + S2 机器合约完成 2026-09-13，S3 实现待放行）

> 需求口径（用户 2026-09-13）：机械层对**每个**模型请求都要**即时且有信息量**地回报——确定性不可达必须立刻返回、检索/网络 10 s 拿不到首个结果就立刻明确回报网络问题、等待必须可见（日志 + 事件）、超时必须有稳定码与原因。**10 s = 请求发出后等首个结果的上限，不是检索任务总时限**（总预算另计）。入口：设计稿 [`IMMEDIATE_RESULT_DELIVERY_AND_STREAMING_RETRIEVAL_DESIGN_2026-09-13`](docs/IMMEDIATE_RESULT_DELIVERY_AND_STREAMING_RETRIEVAL_DESIGN_2026-09-13.md) / 审计 [`FRAMEWORK_TIME_BUDGET_SEMANTICS_AUDIT_2026-09-13`](docs/audits/FRAMEWORK_TIME_BUDGET_SEMANTICS_AUDIT_2026-09-13.md) / [`第 0 轮起跑记录 §6.13`](docs/audits/TB21_V41_ROUND0_MEMORY_HEAVY_START_2026-09-13.md) / BACKLOG 0ac / 索引 `GAP-MECH-IMMEDIATE-FEEDBACK`。计数：立项 **29 → 30**（2026-09-13）。

- [x] ① 探针 `probe_scope` 扩 `retrieval_family`（浏览器可执行 / 搜索引擎端点 / web 通道三类硬设施在位读数），run 起始一次并写 journal——每 run ≥1 条含检索族结论的 `tool_availability_check`。（2026-09-15 随 S3①-a `4c892951` 落码；勾选补记 2026-09-27）
- [x] ② 检索/网络请求统一**截止时间**（`ORZ_RETRIEVAL_DEADLINE_MS` 默认 10 000 ms），到点**立刻**返结构化错误，不等引擎自身 120 s。（2026-09-15 随 S3①-a＋补强 G1 三段账落码；勾选补记 2026-09-27）
- [x] ③ 稳定码 `capability_unreachable`（确定性不可达）/ `network_no_response`（到点无响应）——返回面与 journal 双写、单事件自描述（`tool_completed` 失败载荷补 cause，不再只给壳码）。（2026-09-15 随 S3①-a 落码；勾选补记 2026-09-27）
- [x] ④ 框架契约「机械层对任何模型请求都必须及时且有信息量」+ 回归钉子：任何等待型调用在截止后必须产出带稳定码的结果，无「到点前零事件」的等待路径。（2026-09-15 随 S3①-a 法官面＋钉子落码；勾选补记 2026-09-27）
- [x] ⑤ 投递策略 I1–I3 + D7 机制 M1–M3（后台完成 / 生成中结果即时回报；常规=最早安全边界、极端=思维链句号边界分段续写；不打断当前思维链）——设计 §2/§4。（2026-09-15 S3①-b 收尾批 `1deeba75` 落码；勾选补记 2026-09-27）
- [x] ⑥ 检索子代理提前收口（确定不可达 / 连续确定失败 / 结果已形成 → close activation 并立即回传；墙钟只作最后兜底）。（2026-09-15 S3①-b 收尾批落码〔`ORZ_RETRIEVAL_EARLY_CLOSE_FAILURES`〕；勾选补记 2026-09-27）
- [x] ⑦ `web_search` 信号量 acquire 独立短截止 + 排队即时回报（不再被 900 s 外层包住）。（独立截止 2026-09-15 勘误确认随 `4c892951`；排队可见性随 S3①-b M3 tick；勾选补记 2026-09-27）
- [x] ⑧ S1 探针 → S2 机器合约 → S3 实现（带开关 + A/B）→ S4 实机复验 + 整轮重跑 89 题（与设计 S1–S4 同轨）。（**S4 三题实机复验 2026-09-27 达成**：判据①②③全过＋FR-D02 达成＋reward 2/3 仅记录 ⇒ **0ac 闭合 55 → 54**；**整轮重跑 89 题按用户修正令「不直接跑89题」本轮不发起**、维持 `0b` 验证⑤ / C6 E9-R10 登记，启动器已备 `D:/tb-eval/run_official_21_k1_browser.sh`；[`098 S4 收口档`](docs/audits/098_0AC_S4_CLOSURE_2026-09-27.md)）
- [x] **S1 探针完成（2026-09-13，用户放行真实 API 调用）**：① 流式 `/responses`+`web_search` 实测 SSE 全序列——`response.web_search_call.in_progress/searching/completed` 实测存在、逐事件带时间戳（`deepseek-v4-pro`：TTFB 9.3 s + 首检索进度 10.5 s + 首检索完成 11.1 s，全程 19.6 s，无 `[DONE]` 哨兵、终态=`response.completed`+EOF）⇒ **流式路线确认、分段检索后备不启用**；判活锚点修正为 SSE 通道首字节（TTFB 主导）。② 分段续写 3/3 通过（`deepseek-flash`/`deepseek-v4-flash`/`deepseek-v4-pro` 均接受「部分 assistant+reasoning_content+注入事实」、从句号边界继续、无重复、无配对破损；token 预算须为重 reasoning 留量）。③ **重大运行面发现**：`deepseek-v4-flash`/`deepseek-flash` 上 web_search 工具**确定性不绑定**（4/4 对照零 `web_search_call`，模型 reasoning 自述"没有工具"后编造来源）——而同日早些时候第 0 轮跑批同模型名有 116 条真实检索 ⇒ 服务端兼容路由行为当日变化或间歇；`web_search_call` 存在性必须进 `retrieval_family` 探针读数（flash 静默失绑 = `capability_unreachable` 的真实形态）。入口：[`0AC_S1_PROBE_RECORD_2026-09-13`](docs/audits/0AC_S1_PROBE_RECORD_2026-09-13.md)；探针件 `D:\tb-eval\probe-0ac-s1\`（不入仓）。**下一步 S2 机器合约（待放行）**。
- 验收：检索类**首个结果** `wall_ms` p99 ≤ 10 s；`subagent_wallclock_timeout_mid_tool` = 0；等待路径零事件为 0。风险：10 s 截止会砍检索长尾 ⇒ 保留放宽开关 + 「10 s vs 现状」A/B 记录。
- 边界：**不改 FP-2**（能力级不可达如实汇报 + 有结果即发回本在 FP-2 语义内）；**不新增容器内浏览器**；官方口径不变（流式化只改 agent 侧）。

- 审记（2026-09-14）：S3①② 部分落地（orz `4c892951` + 法官面 `ac5d6375`）后全面审记——设计面 0 问题级缺陷（2 口径注记）；实现面 **G1** 本地检索核心解析器 panic（P0）/ **G2** `fmt` 门未过（P1）/ **G3** 投递侧未落（符合性 P1）⇒ 0ac 仍 open，⑤⑥⑦ 维持未落，裁决点三枚待用户。入口：[`0AC_S3_IMPLEMENTATION_AUDIT_2026-09-14`](docs/audits/0AC_S3_IMPLEMENTATION_AUDIT_2026-09-14.md) / 摩擦台账 F-014/F-015。
- 修复（2026-09-14，run `RUN-CLI-6aa6d379`）：**G1/G2 已修并机械核证**（orz `96d2b263`：`local_segmented` 去字节切片 + 命名实体 + 钉子测试；`fmt --check` 16 处 → 0、新文件 clippy 2 处 → 0；`orz-tools --lib` **2819/49 → 2869/0**）；**F-012 同批修**（`blackboard` 会话面测试改与产品同源解析 ⇒ 常驻 `ORZ_MAX_WALLCLOCK=3600` 下 `orz-loop --lib` **771/0/3**）；**F-016 已修**（PATH 上 `rg` 首解为悬空 WinGet 垫片 ⇒ 46 例 spawn 面红 + `grep` 工具 5 次空返；绕行修后同批转绿，复现实验 47 红）；**G3 维持 open**（三事件仍零产品码写点 ⇒ 摩擦 F-017 立案候选，拆 `S3①-a/①-b` 待裁决）；审记 §2.3/§6-③ 开关口径勘误（F-018，开关实为**已移除**）+ 设计稿 §10.5 回写。**未做**：推送、载体重建、S4 实机复验（用户边界）。入口：[`修复报告`](docs/audits/0AC_S3_FIX_REPORT_2026-09-14.md)。
- **拆分裁决（2026-09-14，用户采纳 F-017 建议）**：**S3① 拆为 ①-a 检索侧 / ①-b 投递侧**——
  - **S3①-a 检索侧（已落**，orz `4c892951` + `96d2b263`**）**：本地分段检索前端（Bing 直连主引擎）、双钟截止（每引擎 10 s + 整体 30 s 兜底，env 可配）、`cause` 自描述、检索族探针 run 起始一次、法官规则 + Python 镜像、F-007(a) 宽口径执法（对应上列 ①②③④ 大部）。
  - **S3①-b 投递侧（未落，下一实现批次）**：三事件 `EventType` 变体 + 族注册 + 产品码写点（`retrieval_progress` / `retrieval_result_segment` / `result_delivered`）、**⑤** 投递策略 I1–I3 + M1–M3（M2 合法边界投递先行、M1 句号边界分段续写带开关 + A/B——S1 探针②已证续写可行 3/3）、**⑥** 子代理提前收口、**⑦** semaphore acquire 独立截止。实施顺序：机械件 + M2 → M1/M3。
  - 0ac S3 出口条件 = ①-b 落码 + ③ 跨 run 时序钉子 + 门禁/镜像全绿；之后载体重建（0.5.1 冻结基线）与 S4 实机复验另行放行。
> 勘误（2026-09-15，交接件 §7 摩擦 A）：⑦ 的 acquire 独立截止已随 `4c892951` 落码（`orz-host` `retrieval_lane_wait_budget`，`ORZ_RETRIEVAL_SEMAPHORE_WAIT_MS` 默认 10 000 ms、`0`=禁用＋有界 acquire `retrieval_lane_busy` cause）——实际只差「排队即时回报」可见性，随 ①-b 收尾批落。
- **[x] S3①-b 收尾批（2026-09-15 过夜批，orz `1deeba75`）——⑤⑥⑦ 全部落码，S3 出口达成**：⑤ M2 B1 边界投递（宿侧 `drain_completed_tasks` + per-run `DeliveryQueue` + 中性事实消息 + `result_delivered{suppressed=false}`）与 M1 收尾注入（子开关 `ORZ_IMMEDIATE_RESULT_DELIVERY_M1`、每 run 一次、boundary=B2）；M3 tick（`ORZ_RETRIEVAL_PROGRESS_TICK_MS` 默认 10s，⑦ 排队可见性就此闭合）；⑥ 提前收口（`capability_unreachable` 即时 / 连续 3 次阈值〔`ORZ_RETRIEVAL_EARLY_CLOSE_FAILURES`〕；`RetrievalSubagentEarlyClose` → `subagent_failed` + cause 自描述，契约不扩枚举）；③ 跨 run 时序钉子（fresh-run 重新入队 / 投递↔payload 一一对应 unsuppressed / 逾期降级 class=I3 / close-drop 不跨 run 泄漏）；交接件 §5-A/C/D/E 四核对全过；摩擦 A 勘误随批落地（acp 期望 9→11 事件，基线 worktree 实证三件合法已提交行为）。门禁全绿（orz-loop 788 / orz-host 324 单线程 / orz-tools 2882 / assurance 226 / fmt / clippy 新代码零告警）。**S4 实机复验 + 整轮重跑 89 题 + 载体重建待放行**。

- [x] **S3①-a 补强（检索侧补强稿 `DESIGN-RETRIEVAL-LOCAL-SEGMENTED-HARDENING`；2026-09-15 设计定稿 + 裁决，未实施）**：设计稿 [`RETRIEVAL_LOCAL_SEGMENTED_HARDENING_DESIGN_2026-09-15`](docs/RETRIEVAL_LOCAL_SEGMENTED_HARDENING_DESIGN_2026-09-15.md)（v1.0；服务 **①-a 检索侧**，不是 ①-b 投递侧）——五处缺口 + 裁决：**G1 计时语义**（三段账 `T_acquire` 5 s ⊂ `T_first` 10 s ⊂ `T_overall` 30 s、`T_segment` 10 s·页；`ORZ_RETRIEVAL_ACQUIRE_MS`/`ORZ_RETRIEVAL_SEGMENT_MS` 可配，`ORZ_RETRIEVAL_DEADLINE_MS` 语义收窄为 `T_first`）；**G2 降级页相关性闸门**（HTTP 200 + 整页无关 `b_algo` 现被判成功 ⇒ 闸门**默认开**：前 3 条 ∩ 查询词集，阈值 25% + 词集**封顶 12**；判负复用 `empty_result` + detail 并继续引擎链、**不新增稳定码**；开关 `ORZ_RETRIEVAL_RELEVANCE_GATE`）；**G3 引擎面收尾**（跳转包装并发解包 **6 worker / 单条 6 s** + 页抓取最终 URL 回填；`ORZ_RETRIEVAL_UNWRAP_WORKERS`/`_MS`）；**G4 代理管道**（只加在分段检索专用客户端 `local_http`；读取序 `ORZ_RETRIEVAL_PROXY` → `HTTPS_PROXY` → `HTTP_PROXY`，`none` 显式关；**不设引擎白名单**）；**G5** 无头/有头仅登记（待容器内复验）。**落码顺序 G2 → G1 → G3 → G4**；每项按「改动 + fixture + 法官镜像 + A/B 读数」走（设计稿 §8），**不合批进 ①-b**（两件代码面/验收面互不重叠）；落码后需载体重建，建议与本批 ①-b 或 0z S4 共用一次三件套。判据沿用：检索类首个结果 `wall_ms` p99 ≤ 10 s、`subagent_wallclock_timeout_mid_tool` = 0。未决/复验条件见设计稿 §10.7。入口：设计稿 / 索引 `DESIGN-RETRIEVAL-LOCAL-SEGMENTED-HARDENING` / BACKLOG 0ac「检索侧补强设计定稿与裁决」（子切片，**不动计数**）。
- **[x] S3①-a 补强落地（2026-09-15 过夜批，orz `7e151ed1`）——G2→G1→G3→G4 全落**：相关性闸门默认开（词集封顶 12、判负复用 `empty_result` + 分数 detail、全链取最高分、词集空放行）/ 三段账（`T_acquire` 5s connect_timeout · `T_first` 10s 每引擎钟只包 SERP · `T_segment` 10s/页 慢页不吞命中 · `T_overall` 30s 从属）/ 跳转解包 6worker·6s + 页抓取最终 URL 回填取值序 / 代理管道（`local_http` 专用、读取序 + `none` 显式关、无引擎白名单、有代理默认链四引擎 + 探针 `proxy=on|off`+脱敏端点）。23 单测绿。**A/B 实机读数与 G5 复验留 S4**。


### P0-0ar 检索批次回送与轮级单席位（P0；2026-09-19 用户令纳入排期；设计稿 v1.1；**S1 契约面已完成并验收通过（`28e855b1`）＋S2 已提交（`3d7d7a74`）＋S3 前去噪已提交（`bd253ee7`）＋载体 0.6.3 双平台已重建（`ac17a521`）＋S3 同三题真机复验已完成（2026-09-20，读数见下）**；**2026-09-20 用户裁决：闭合（未闭合 36 → 35）**）

> 入口：设计稿 [`RETRIEVAL_BATCH_HANDOFF_AND_ROUND_SEAT_DESIGN_2026-09-19`](docs/RETRIEVAL_BATCH_HANDOFF_AND_ROUND_SEAT_DESIGN_2026-09-19.md)（v1.0） / S1 实施报告 [`0AR_S1_CONTRACT_SURFACE_2026-09-19`](docs/audits/0AR_S1_CONTRACT_SURFACE_2026-09-19.md) / 上游分析件 [`TB21_RETRIEVAL_VOLUME_CONVERSION_DEEP_ANALYSIS_2026-09-19`](docs/audits/TB21_RETRIEVAL_VOLUME_CONVERSION_DEEP_ANALYSIS_2026-09-19.md) / 验证轮记录 [`TB21_V41_TIMEOUT3_VERIFY_2026-09-18` §10](docs/audits/TB21_V41_TIMEOUT3_VERIFY_2026-09-18.md) / BACKLOG 0ar / 索引 `RETRIEVAL-BATCH-HANDOFF-ROUND-SEAT`。计数：立项 **36 → 37**（2026-09-19；S1 完成不动计数）。批序**各步独立放行、不得跳步合批**；**S1 实施令已于 2026-09-19 下并执行完毕**，**S2／S3 仍以逐步放行为准**。

- [x] **S1 契约面（2026-09-19 完成并验收通过；工作树未提交、未推送、未重建）**：`runtime/retrieval-close-record-event-payload-v0.2.schema.json` 的 `terminal_reason` 闭枚举 **9 → 12**（`evidence_threshold_met`／`dispatch_wallclock_bound`／`subagent_early_delivery`；`normal_close` 既有分支未动、新值不受其约束；`dispatch_wallclock_bound` 另挂「assessment 链必带 `assessment_id`＋`result_digest`」`allOf`）＋三件正例＋两件约束反例 fixture（生成器产出）＋`runtime/tests/test_retrieval_close_reason_enum.py` 的 `ALL_REASONS`=12 同步＋`information_sufficiency_assessment` 增可选 `usable_source_count`／`sufficiency_gap`（`gap ⇒ count` 配对）。**验收读数**：靶向契约单测 9 passed／runtime 全套 **366 tests OK**（292.1s）／门禁 `error_count=1`（唯一＝orz 影批脏树，预期态）＋v0.2 payload 正/负 **71/51**／`compileall` exit 0／六件 fixture 独立 jsonschema 复核（正例 0 错、负例各恰 1 错）／生成器重跑 346 件逐文件 SHA256 全等。报告：`docs/audits/0AR_S1_CONTRACT_SURFACE_2026-09-19.md`。
- **本轮狗粮实测（新狗粮轮，2026-09-19）**：run `RUN-CLI-6aad9497`（无墙钟、exit 0、30m29s、141 次工具调用、`run_finished` 正常收尾）承接前序 run `RUN-CLI-6aad91f0` 的 WIP 完成 S1；**框架摩擦 8 项（F1–F8）**登记于报告 §7。其中 **F1 已随本批修复（携带项，非 0ar 语义）**：`scripts/dogfood_launch.ps1` 原在 `$ErrorActionPreference='Stop'` 下汇流原生 stderr，PowerShell 5.1 把首条 transport WARN 升级为终止错误 `NativeCommandError` ⇒ 脚本中止、管道被拆、进程树被收、Tee 日志不落盘（`RUN-CLI-6aad91f0` 第 10 分钟／72 轮／97 次调用死于此，journal 停 seq 776 且无终态事件）；修法＝发起处局部降 `Continue`（WARN 照常落日志）。
- [x] **S2-D1 阈值回送（2026-09-19 提交批 `3d7d7a74`）**：宽口径可用计数（`retrieval/batch_close.rs::usable_source_count`——visibility∈{full,partial} 按 content_sha256 去重）满 **5** 即 β 收尾（post-batch 间隙武装＋下一轮工具面机械收空＋唯一收尾回合，`LoopOutcome.retrieval_close` 携带成因 → close `evidence_threshold_met`）；护栏 **10** 强制收尾（`mechanical_cap_force_close` 码）；可见倒数行随每批检索工具结果机械追加（条目/调用分开报）；提前交付＝`[EARLY_DELIVERY]` 标记＋证据指针（缺指针 fail-open＋anomaly；连续 ≥3 次 streak anomaly）；新增正常收尾臂、不复用失败臂。测试：batch_close 6 单测＋阈值／提前交付正反集成 3 件。
- [x] **S2-D2 未达标交回（2026-09-19 提交批 `3d7d7a74`）**：墙钟到点不再走 `RetrievalSubagentTimeout` 失败臂——合成 `WallclockBound` outcome 走共用收尾路径（部分证据报告 exit 0 正常交回、assessment 带 `usable_source_count`＋`sufficiency_gap`、close `dispatch_wallclock_bound` 必带 assessment 链＝判据 1）；档位表 240/600/900 → **180/300/450**；`ORZ_RETRIEVAL_SUBAGENT_TIMEOUT_SECS` 优先级不变。测试：超时集成测试重写为确定形态（子代理第二请求挂起）。
- [x] **S2-D3 合并优先＋溢出拆轮（2026-09-19 提交批 `3d7d7a74`）**：派发前预扫描——前 **3** 个检索调用合并为单激活多 query（goal 并 `[合并查询 n]` 行；`query_summary` 逐 query 一条＋逐 query 可用计数，契约 `query_entry` 增可选 `usable_source_count`）；被合并调用以合并回执交回（ToolStarted/Completed 成对）；溢出调用无 `ToolStarted` 拒绝（`cause=retrieval_dispatch_deferred_one_per_round`＋stamp_failure 漏斗）＋下一轮一次性重述。测试：合并／溢出集成 2 件。
- [x] **ADR-0010 转录（随 S2 同批，2026-09-19）**：**§14.73 / v1.75**——恢复的是**裁决权而非仪式**（不要求主代理调用 `retrieval_disposition`）；依据＝THIN_HARNESS §4.4 自述该偏差「ADR 修订留待 R3 验证通过后实施」，验证轮 r1–r3 已完成 ⇒ 现处裁决窗口。
- [x] **提交与隔离验证（2026-09-19）**：0am hunk 级分离并 stash 后，提交树隔离复核——orz-loop **800/0/3**／orz-assurance **231/0**／orz-host 串行 **333/0/5**／orz-tui **178/0**／orz-tools lib **2890+2**（LSP 负载敏感，串行 16/0）／orz-hooks lib **192/0**／orz-bin bin **12/0**＋stdio_e2e **1/0**／`cargo fmt --all --check` 0。
- [x] **S3 前去噪（2026-09-19，`bd253ee7`）**：空批不唤醒（`should_arm_close(0)=None`）＋重复 query 指针回踩（检索车道跨轮/主车道同轮去重，指针回执交回原 call_id）；orz-loop **804/0/3**，六个既有主车道新激活测试保持绿。
- [x] **载体 0.6.3 双平台重建（2026-09-19，源冻结 `ac17a521`）**：Windows `cargo clean`＋release 全量 15m44s＋ACAF 重 provision（manifest↔signer 一致）；Linux musl 静态三件套（PT_INTERP=0、bookworm/alpine 冒烟 exit 1）；字面量核证 0ar S2／去噪／0as 进件、**0am 面全零**。见 [`063 重建审计`](docs/audits/063_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-19.md)。
- [x] **S3 同三题 k=1 真机复验（2026-09-20 完成；用户令「请直接进 S3 同 3 题的复验吧」）**：作业 `official-verify-timeout3-s3`（55m06s、exit 0、`--upload --public`），载体 **0.6.3**（`ac3fb7ba…`）＋适配器 `fdd161d4…`（F1 隔离修复首验），口径与 r1–r3 逐项一致。**判据**：1 达成（7/7 已闭合激活 `evidence_threshold_met`）、2 达成（批墙钟 157.8／78.2／232.1／57.4／97.6／155.8／155.8 s，全在 300 s 门内；D2 到点臂未触发）、3 达成（首批 commit 后主回合 54／29／17，旧形态 4/6 试次为 0）、6 部分达成（多 query 激活 4 例条目数与逐 query 计数在场，2 例逐 query 欠归因）、4／5／7 未触发（语料无 >3 检索、无计数不一致、模型未用提前交付；倒数行无落盘面 ⇒ 判据 7 后段真机不可核）。**读数**：torch 首次 `run_finished{completed}`＋判分物产出＋verifier 2/4 通过（余 2 项数值不匹配 max diff 0.0137）；gpt2 首次出现检索（2 批）但仍未落判分物；extract-elf 3 批达标回送但未落判分物（相对 r3 的 1.0 系泄漏路径，见审计 §7-N5）。报告：[`0AR_S3_THREE_TASK_VERIFY_2026-09-20`](docs/audits/0AR_S3_THREE_TASK_VERIFY_2026-09-20.md)。**待裁决**：0ar 是否闭合／N2 run 尾部派发是否转 A2 立项／N3 倒数行落盘面是否补。
- 计数口径与判据口径：阈值／护栏／可见倒数共用**宽口径**一把尺（`visibility ∈ {full_text, partial}`，按 `content_sha256` 去重；2026-09-19 审查处置勘误——relevance 在机械计数点不可得且 ledger 层恒 direct 为空操作，限定词收窄，设计稿 v1.1 §3.3）；软规则与机械护栏不一致时落 anomaly、不静默取其一；收尾阈值按**批级合计**、逐 query 计数为可核披露（逐 query 收尾语义与护栏 10 不相容，设计稿 v1.1 §5.4）。
- [x] **0ar 收口裁决（2026-09-20 完成）**：判据 1/2/3 达成、6 部分达成，4/5/7 因语料未触发（机制面 S2 锁定）经用户裁决接受为充分 ⇒ **0ar 闭合（未闭合 36 → 35）**，索引条目转 `current-design`。附：N2（run 尾部派发）／N3（倒数行落盘面）等六项摩擦随同日「N1–N6 深挖」**登记观察、不立项**（处置候选汇总见深挖报告 §9）：[`0AR_S3_FRICTION_DEEP_DIVE_N1_N6_2026-09-20`](docs/audits/0AR_S3_FRICTION_DEEP_DIVE_N1_N6_2026-09-20.md)。
- 边界：不改任务镜像／verifier／数据集 pin；**不在 orz 做容器内环境提前补强**（0ac r3 环境裁决沿用）；不针对单一 benchmark 调参——D1/D2/D3 参数须对日常检索与调研场景同时成立；不恢复 disposition 仪式、不改 FP-2、不新增工具面。


### 0ag 契约面机械对账（P2；2026-09-15 立项并当日闭合，orz `8512fc71`；ID 冲突更正：0af 归邻线资源门项〔先占〕，本项改名 0ag，计数 32 净不变）

- [x] schema 闭枚举 ↔ 实现常量测试期逐字互证（`immediate_delivery` 钉子 `schema_closed_enums_verbatim_match_implementation_constants`：常量 ⊆ 闭枚举 + 抑制码/五稳定码全等闭集 + plan_write `section` 枚举 = `ModelNoteSection`）；交接件摩擦 C 治本，§5-A 人工核对点退役。入口：[BACKLOG 0ag](docs/BACKLOG_AND_PRIORITIES.md) / orz `8512fc71`。


### 0af 资源门拒绝文案明确化（P1；2026-09-15 立案，深审摩擦 B 注册；**2026-09-16 闭合入账，36 → 35**）

- [x] `orz-host` resource_gate 拦截信封原因文案改为「宿主机内存/储存资源即将耗尽，无法新增派发，请寻找其他方案」（按实际耗尽轴标注内存/储存＋readings 随附）；同步涉及文案断言的测试/fixture。边界：不改 fail-closed 判定逻辑与阈值。（2026-09-16 闭合：定案句按轴标注＋**Unknown 档变体句防不实陈述**＋**headroom 一位小数向下取整＋字节直读**（根除「25% < 25% required」字面自相矛盾）＋**连带完成 F-BE-12 残留**（拒绝臂第二次 evaluate 改用唯一一次判定）；钉子 4 条，orz `b6ed78d9`，orz-host 串行 332/0/5、fmt 干净、clippy 与 HEAD 基线零新增。入口：[`0AF_0AH_CLEANUP_2026-09-16`](docs/audits/0AF_0AH_CLEANUP_2026-09-16.md) / [BACKLOG 0af](docs/BACKLOG_AND_PRIORITIES.md)）


### P0-B FUS-RETRIEVAL-MECH（`implemented`；批次 1-6 全部闭合 2026-08-14，保留供核对）

- [x] 全部闭合：B-1 citations 结构化透传 / 步骤 2 web_fetch 候选计数门禁（cap=8）/ 步骤 3 机械预筛（canonical 去重 + 失败形态剔除 + tier/weight）/ 步骤 4 browser_read 模式扩展 + 计数域复用 / 步骤 5 输出级引用校验器 / 步骤 6 提示词缩短。入口：[检索机械控制设计](docs/RETRIEVAL_MECHANICAL_CONTROLS_DESIGN_2026-08-13.md) / 各步骤实施审计 / BACKLOG 0B。
- 注：DC 剩余两信号（`same_module_no_evidence` / `key_surface_unexamined`）已转 P3 遗留小项，2026-08-31 随 P2-11 DC 清理退役。


### P0-C CLASSICAL-EXEC-ASSISTANT（已转正式组件；小样 1/2/3 + S1-S4 全部闭合 2026-08-16）

- [x] 全部闭合（2026-08-16 用户裁决转正式组件，不达标即撤条款未触发）：小样 1/2/3（控制台路由 / 编辑执行器 / 机械组合脚本）+ orz 内嵌集成 S1-S4（操作台核心 + 黑板动作栏 / 模型面投影 + 轮末机械发放 / 结构化策略拒绝 + assistant.trace + run_script + Profile/Bundle / 端到端 + 单步超时 + 脚本预算 + 二次审查收口）。入口：[设计](docs/CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md) / [POC](prototype/classical_console/README.md) / 各实施审计 / BACKLOG。


### P0-C2 PLAN-FIRST-BLACKBOARD（阶段 A/B/C 全部闭合 2026-08-16；生产默认路径不启用）

- [x] 全部闭合：阶段 A（模板去人格 + AGENTS.md 机械包裹 + 首轮计划轮硬门 + plan_write）/ 阶段 B（注册板块=探针投影）/ 阶段 C（console 默认 + direct 受控降级双模式 + 步骤门 + 事件契约）。入口：[设计](docs/PLAN_FIRST_BLACKBOARD_DESIGN_2026-08-15.md) / ADR-0010 §14.17 / 各实施审计。


### P0-D ORZ-COMPACTION-REDESIGN（`implemented`；S1-S6 全部闭合 2026-08-14）

- [x] 全部闭合：S1 恢复预检截断 + marker/白名单保留 / S2 动作台账机械坍缩 / S3 五段模板摘要 + 事件面 + 存档 / S4 审计同步 / S5 审查修复（守卫重试、session_end、冷却、超时）/ S6 二次复查。入口：[压缩设计](docs/CONTEXT_COMPACTION_DESIGN_2026-08-14.md) / ADR-0010 §14.10/§14.14 / [审计](docs/audits/GAP_COMPACTION_REDESIGN_IMPL_AUDIT_2026-08-14.md) / BACKLOG 3b。

## P1 — 可并行审计 / 证据


> 来源：主 TODO「## P1 — 可并行审计 / 证据」闭合子节

### P1-0q 统一失败事件管线（F4 盖章治本；2026-09-07 登记；BACKLOG 0q；**S1–S4 全部闭合 2026-09-08，转 `implemented`**）

- [x] S1 设计定稿（ADR-0010 §14.63）：写入侧边界单一漏斗（host_exec 完成装配点）+ receipt 补 `action_target` 第五族 + Rust 法官唯一执法 + Python 冻结对照 + 行集纯增量零迁移；四点裁决权由用户授予主代理。入口：[设计定稿](docs/0Q_FAILURE_EVENT_PIPELINE_DESIGN_2026-09-08.md)。
- [x] S2+S3 实施（orz `4dfb3d77`）：`stamp_failure` 收口四散布写点（退役逐点对拍）+ `failure_agg_absent` 标记（与 failure_target XOR）+ console 订单漏斗 + grandfather 锚 `failure_pipeline: "funnel-v1"` + 法官新族 `failure_agg_coverage`（31 族）+ Python 镜像同步 + schema 三处 + 覆盖面矩阵/e2e/正反两测/六场景对拍全绿（orz-loop 751 / orz-assurance 210 / orz-tools 2844）。（2026-09-08 闭合）
- [x] S4 收口：BACKLOG/TODO/索引/ADR §14.64 闭合转录 + manifest 重算 1441 条 + 门禁 Exit 0；基线 0.3.2 不 bump；计数 28 → 27。（2026-09-08 闭合）
- 完整勾选与调研明细（六家对标）见全量快照 [`TODO_FULL_2026-09-09`](存档/todo/TODO_FULL_2026-09-09.md)。


### ORZ-CACHE-CONTEXT-COST（`implemented`；2026-08-15 三项全部闭合）

- [x] 全部闭合：`request_header_change` 请求头留痕 + 探针准确性审计（翻转↔header 交叉核对）+ 单轮工具结果注入预算（默认 50K、超限拒批 + offset 续读）与策略化读取；orz-loop 337 / Python 1888+14 skipped / 仓库门禁 valid。入口：ADR-0010 §14.9 / [探针设计](docs/TOOL_AVAILABILITY_PROBE_DESIGN_2026-08-13.md) / [审计](docs/audits/GAP_CACHE_CONTEXT_COST_IMPL_AUDIT_2026-08-15.md)。


### ORZ-ORIENTATION-FORCED-TEMPLATE（`implemented`；2026-08-15 闭合；2026-09-01 退役）

- [x] 全部闭合：ADR-0010 §4.2 正文修订 + 强制模板轮实现（无工具 checkpoint 轮、模板校验、一次重填 + 降级兜底、pending 单槽、主车道）+ 缓解必做（progress_evidence 交叉校验 + 缺失面）+ v0.2 `checkpoint_response` 事件（Schema/verifier/fixtures）+ 二次审查修复；orz-loop 333 / Python runtime 264。入口：[设计](docs/ORIENTATION_FORCED_TEMPLATE_DESIGN_2026-08-14.md) / ADR-0010 §14.13 / [审计](docs/audits/GAP_ORIENTATION_FORCED_TEMPLATE_IMPL_AUDIT_2026-08-15.md) / BACKLOG 6c。


### ORZ-BLACKBOARD-PLAN-EPOCH（`implemented`；S1-S7 全部闭合，2026-08-14/15；2026-09-03 退役标注：生产语义被会话作用域黑板取代，`--plan` 诊断保留）

- [x] 全部闭合：plan epoch 身份与批准事件 / 原子轮换与归档（.gsa/blackboard）/ 压缩解耦 / 跨 epoch 回查 / 测试审计 + S6/S7 复查补强（epoch 时间戳单调、归档写盘原子化、epoch_archive_write_failed 事件）。入口：[设计](docs/BLACKBOARD_PLAN_EPOCH_DESIGN_2026-08-14.md) / ADR-0010 §14.15 / [审计](docs/audits/GAP_BLACKBOARD_PLAN_EPOCH_IMPL_AUDIT_2026-08-14.md) / BACKLOG 6e。


### ORZ-LARGE-FILE-READ-CONTRACT（`implemented`；2026-08-17 设计定案，同日实施闭合）

- [x] 全部闭合：读取句柄信封（粗门默认 16KB、可配 8–32KB + 有界预览 ≤4KB + offset 续读指针）+ 模型面契约提示（grep/结构优先、大文件分段、空结果语义）+ 黑板/结果栏只放指针 + 全面检查修复（信封空窗口/越界语义、toolset 配置端到端、描述同步）；orz-tools read_file 201 / orz-loop 440 / orz-host e2e 5。入口：[设计 §11](docs/CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md) / ADR-0010 §14.22 / [审计](docs/audits/GAP_LARGE_FILE_READ_CONTRACT_IMPL_AUDIT_2026-08-17.md) / BACKLOG 6f。


### FUS-LEDGER-FOLD-STATE（`implemented`；2026-08-18 设计定案，同日实施闭合）

- [x] 全部闭合：fold 三态 + 有状态请求视图 + loop-top 推进触发（128K）+ 压缩联动/摘要同源/恢复 + 参数接线（192K/256K）+ 二次全面审查收口（冻结台账进摘要存档、v0.2 `ledger_fold_advance` 事件、完整性回退、死代码删除）；orz-loop 452 / conformance 15 + journal validation 214。入口：ADR-0010 §14.26 / BACKLOG 6g / [审计](docs/audits/GAP_LEDGER_FOLD_STATE_IMPL_AUDIT_2026-08-18.md)。


### 0ab 账本一致性与瘦身机械化（P1；2026-09-13 立项，深审附带建议②）

- [x] `check_repository.py` 增计数一致性四点交叉核对（BACKLOG 总数 ↔ 各节开放项 ↔ TODO 勾选 ↔ 索引状态速查）+ 账本瘦身检查（台账行长限/行龄）；负例钉子（人为不一致可检出）；首批账本瘦身随 S1 做。入口：[深审 §3](docs/audits/FULL_PROJECT_DEEP_REVIEW_2026-09-12.md) / [BACKLOG 0ab](docs/BACKLOG_AND_PRIORITIES.md)。
- [x] S1 完成（2026-09-15）：两组检查进 `check_repository.py` 常驻门禁（计数一致性交叉核对＋状态词封闭集＋标题漂移检查；头部台账行 ≤1200 字符/行龄 ≤21 天提示归档）＋钉子 `assurance/tests/test_ledger_consistency_nails.py`（13 合成负例＋真实仓库常驻零错 2 钉）＋首批瘦身随批执行（索引 54 行滚出归档、BACKLOG 计数流水行收缩入档、P0/P1 清单对账修复、0v 残留勾选补勾、P2 重复标题修复）；门禁 `valid: true`，不动计数。


### 0ai 重文件拆分（`host_exec.rs` 优先；**本轮狗粮线修复考核测试任务**；P1；2026-09-16 立项；**2026-09-16 闭合：产出已合回主仓 orz `b682a67f`**）

- [x] S1 切分图：`host_exec.rs`（拆分基线实读 9,710 行）按职责域拆 7 模块（mod 24 / tool_run 6,784 / failure 917 / facts 855 / serp 817 / dep_graph 269 / candidate 186）；产出 run 报告 `0AI_HOST_EXEC_SPLIT_REPORT_2026-09-16.md`（**工作区件，随产出合回后入库**；读数全文见下方收口文档）。
- [x] S2 机械搬移：代码实体逐行搬迁零逻辑改写；行多重集等价（MISSING 9/INVENTED 118 全为胶水）＋差分自检非空转；`pub(crate) use` 保 `crate::host_exec::*` 旧路径。
- [x] S3 回归核验：orz-loop **818/0/3**（主会话独立复跑同值）、**clippy 52→52 零新增**（主会话补跑，agent 沙箱无 protoc）、orz-host 串行 **325/0/5**（并行 8 失败复跑归因抖动）、fmt 干净；agent 报告 §6 读数齐。
- [x] 狗粮考核测试：run **`RUN-CLI-6aa999d6`**（46m47s、234 轮、2097 事件；无墙钟）——命中率 **96.70%**（≥90% ✓）、5 窗驱逐（折叠后 view 21–29K）、500K 纯提醒如实、`read_file` 38 全文读零 offset；**存档三键 N/A**（archives 接线在 ACP 车道，`-p` 不可达）；**考出接线缺口三条**（blackboard_write 未进 `-p` 声明面／增量归档 `-p` 不可达／门禁冻结克隆 F2）＋agent 摩擦 F1–F8。读数全文：[`0AI_DOGFOOD_ASSESSMENT_CLOSURE_2026-09-16`](docs/audits/0AI_DOGFOOD_ASSESSMENT_CLOSURE_2026-09-16.md)。单轮如实标注、不作架构结论。
- [x] 前置：**载体重建**——已完成 **0.5.2**（2026-09-16，源冻结基线 orz `a580eb08`＝`61982a56`＋bump＋orz-tui 断裂修复；双平台三件套＋冒烟＋换装＋manifest 1450 条＋门禁 `valid: true`；`GAP-ORZ-HOST-RESOURCE-SNAPSHOT-DROP` 随批闭合）。入口：[`052 重建与放行记录`](docs/audits/052_CARRIER_REBUILD_AND_0AI_RELEASE_2026-09-16.md)。
- [x] **产出合回主仓**（2026-09-16 用户裁决）：orz **`b682a67f`**（7 文件，rename 71%；合回前逐文件哈希 MATCH、合回后主仓复跑 818/0/3＋fmt 干净）；agent 报告入库；manifest 1450 → 1456 条。**0ai 闭合（33 → 32）**；同批考出三条摩擦立项 0aj/0ak/0al（32 → 35）。


### 0ak 增量归档/三键存档 `-p` 车道不可达（P2；2026-09-16 立项，0ai 考核测出；**2026-09-18 判据达成闭合入账，36 → 35**）

- [x] **取证完成（2026-09-16）**：打包内容＝**对话侧车原文**（`package_session_archive`），而 `-p` 一次性 run 不写对话侧车（GAP-CONVERSATION-RESTORE）⇒ 该车道**无归档源**（非"漏接一个调用"）。**选项 A**＝登记「归档面 ACP-only」＋0ah S1 存档三键判据口径改挂 ACP 车道（零代码）；**选项 B**＝为一次性 run 引入对话持久化（会话身份＋侧车落盘＋里程碑归档）。
- [x] **用户裁决（2026-09-16，零代码登记）：采 B**。理由（用户口径）：「UI 部分估计还要相当一段时间才能进行适配」⇒ 归档能力不押 ACP/UI 车道。入口：[`0ak 裁决 §4.3`](docs/audits/0AJ_0AL_FRICTION_FIX_AND_053_CARRIER_2026-09-16.md)。
- [x] **实施落码（2026-09-16 用户放行「请先进行 0ak 的剩余部分吧」）**：①会话身份 `{ts}-cli`（ts = run id 同秒后缀，session8 = ts 与 `RUN-CLI-{ts}` journal 目录互认；跨调用恢复不开启）＋空 `Vec` 对话随 run 线程携带（成功后取回；附带行为＝长 run 收尾会话末机械压缩同源生效，ADR 登记）；②新增 `orz_host::acp_server::headless_session_archive`——`StoredConversation::full` 同源装配＋`incremental_archive_due` 判定＋`package_session_archive` 打包＋ARC `session_archive{incremental:true}` 审计，全复用 ACP 原语（打包链新增 `explicit_runs` 显式注入 run id：`RUN-CLI-{ts}` 不匹配 `RUN-{session8}-` 前缀扫描，ACP 传空零变化）；③侧车仅归档到期时落盘（阈值下零产物）；④ADR-0010 **§14.68 / v1.69** 转录。**钉子 3 例**（端到端三键＋显式 run 注入断言／阈值下零产物／同里程碑幂等）。读数：orz-loop 821/0/3、orz-host 串行 328/0/5、orz-assurance 229、fmt 干净、clippy 与基线持平。入口：[`0AK_HEADLESS_ARCHIVE_IMPL_2026-09-16`](docs/audits/0AK_HEADLESS_ARCHIVE_IMPL_2026-09-16.md)。
- [x] 判据：无头长 run（≥500K）产出 `.gsa/archives/<session8>.json.gz` 且 `archive_keys` 三键齐备。**2026-09-17 读数（run `RUN-CLI-6aaad7c8`）＝未触发不可判**：最高模型面估算 324,739 ＜ 500K ⇒ `archives/` 未产生（「阈值下零产物」钉子的生产实证）。**2026-09-18 判据达成（run `RUN-CLI-6aac0af5`，摩擦处理狗粮批）**：128 工具轮、峰值模型面估算 291,410，收尾产出 `.gsa/archives/6aac0af5.json.gz`＋`6aac0af5.milestones.json`（archived_tokens=**505,560**≥500K）；主会话解包核证三键齐备（lif 轮跨度 121→128／ledger seq 1–7691／journal run seq 0–1275）＋conversation＋schema ⇒ **0ak 闭合 36 → 35**。入口：[BACKLOG 0ak](docs/BACKLOG_AND_PRIORITIES.md) / [`处理批报告 §4`](docs/audits/FRICTION_INVENTORY_TREATMENT_2026-09-17.md)。


### 0an 多字节路径包含性判定进程崩溃修复（FR-N01；P1；2026-09-17 狗粮 run 发现并**当日修复**；**已闭合 36 → 35（2026-09-18）**）

- [x] **发现（2026-09-17，run `RUN-CLI-6aabf5eb`）**：t+11m27s 对 `read_file("存档/docs/README.md")` 触发**进程 panic**（无 `run_finished`、journal 断链、全场产物丢失）——`orz-tools` 路径包含性判定 Windows 分支按字节切多字节路径名，`end byte index 11 is not a char boundary`，崩点 `crates/codegen/orz-tools/src/types/resources.rs:509`；可达面＝读工作区内任何含中文名目录。证据：[`处理狗粮报告 §2/§3`](docs/audits/FRICTION_INVENTORY_TREATMENT_DOGFOOD_2026-09-17.md) ／ [`摩擦盘点 §7b`](docs/audits/FRICTION_INVENTORY_2026-09-17.md)。
- [x] **修复（2026-09-17 当日）**：`candidate_is_under_raw` 改字节级 `eq_ignore_ascii_case` 比较（禁字符串字节切片，语义等价）＋钉子 2 条（多字节不 panic／中文目录端到端）；读数 orz-tools lib **2886/0/6**、`fmt` 干净、clippy 零新增。
- [x] **载体 0.6.1**：clean 全量重建 9m48s＋冒烟＋字面量 11 项与 0.6.0 基线一致；换装 `D:\tb-eval\orz-windows` 逐对 MATCH（旧件 `.0.6.0-bak`）；signer 哈希变更 ⇒ ACAF manifest 按 052 先例重 provision；**实证**＝0.6.1 读取 `存档/docs/README.md` 正常返回。
- [x] **代码提交（2026-09-17，用户令「提交并推送」）**：orz `26dcce1b`（FR-C04）／`05db4b3d`（本修复）／`1b047158`（bump 0.6.1）；父仓 pin → `1b047158` ＋ `orz_source_manifest.sha256` 重算；两仓已推送（真机直连）。
- [x] **Linux 载体重建（2026-09-18 完成，本项最后一项动作）**：Linux musl 三件套已随 **0.6.2** 重建换装（2026-09-18 用户令「请进行重建吧，双平台」；`D:\tb-eval\orz-linux`，旧件 `.0.6.0-bak`；PT_INTERP=0、bookworm/alpine 双向冒烟绿；首轮 apt 代理 5 连败 ⇒ 切代理后一次通过＋字节级还原；详见 [`062 载体重建`](docs/audits/062_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-18.md)）。
- [x] **闭合入账（2026-09-18，随 0.6.2 提交批：36 → 35）**：orz `08ab194c`（bump 0.6.2 载体重建源冻结）随提交批入库；索引 `GAP-BYTE-BOUNDARY-PANIC` 载体句同步「双平台补平」。入口：[BACKLOG 0an](docs/BACKLOG_AND_PRIORITIES.md) ／ [索引 `GAP-BYTE-BOUNDARY-PANIC`](CLI_PROJECT_INDEX.md)。


### 0ao 重名面全库收敛（P1；2026-09-18 用户令纳入排期；处理批报告 §1.3 注册；**已闭合 37 → 36（2026-09-18，orz `ad8c0da3`）**）

- [x] **剩余副本复核与替换（2026-09-18 过夜批落码，工作树未提交）**：单一源上移 `orz_assurance::tool_names`（依赖合法唯一公共层；`context_compress` 自诞生同源）——`blackboard.rs` 改零字面再导出；生产判等面/文案面全收敛（tool.rs／tool_probe.rs／tool_run.rs／agent_loop 工作台文案／context_scale 文案／orz-host permission 臂／orz-assurance families 判官表；projection.rs 经复核为纯测试面）；机械扫描钉子 2 枚（全库遍历＋标识符级匹配＋恰一定义处；具名豁免表当前为空）。（判据达成待提交批闭合）
- [x] **闭合入账（2026-09-18，随 0.6.2 提交批：37 → 36）**：orz `ad8c0da3` 入库（判据四项——扫描钉子/隔离工作树 791/231/332 全绿/clippy 逐位持平/fmt 干净——全过，读数见回执 §5/§8）。入口：[BACKLOG 0ao](docs/BACKLOG_AND_PRIORITIES.md) ／ [`实施回执 §1`](docs/audits/0AP_0AO_COMPRESSION_INTERACTION_AND_TOOLNAME_SINGLE_SOURCE_2026-09-18.md)。


### P1-0bq 进程收口兜底（run 收尾扫净绕出 Job 的游离进程）（P1；2026-09-24 用户令立项，BACKLOG 0bq；来源＝2026-09-24 真机会话 `RUN-aa18ef7e-0`〔模型用 `WMI Win32_Process.Create` 以 detached 方式拉起 Chrome 绕开子 Job 回收：PID 23916、进程数 14→27〕＋用户令「模型既然绕出去了那就是有需求，加个收口用来兜底吧，orz 发出的进程在主进程结束后将一并被关闭，最后收干净就行」；**同轮执行＝0bm**）

> **口径澄清（2026-09-24）**：orz 本身跑在真机、无 OS 沙箱（`orz-sandbox` 未接线生产面）；run 根 Job 的 `KILL_ON_JOB_CLOSE`＋上限与每次调用的子 Job 只约束 **orz 派生出去的进程树**。本条只补「绕出 Job 的游离进程」这一残余面，不改沙箱语义（无沙箱）。

- [x] **形态**：**出身登记＋收尾扫净**——orz 派生的进程一律留痕（`.gsa/process_trees/` 登记面，沿 [`FUS-HOST-RESOURCE-SAFETY`](CLI_PROJECT_INDEX.md) B 面既有设计：父链死／指纹匹配／时间窗三条件），run 收尾与 orz 主进程退出时扫净**登记面内仍存活且可归因本 run** 的进程；detached 绕出者按出身登记＋时间窗追认；收尾失败如实记事件、不静默；判据＝「主进程结束而派生进程仍存活」零复现。
- [x] **同轮接口**：与 0bm ③ spawn sink per-dispatch 化共用登记面与归属面（登记留全局可见、归因 per-dispatch），避免两套账。（两件随 0bm 轮落码入账；闭合见下）
- **翻账 `pending` → `partial`（2026-09-27，REV-083-16；0bv 结转批）**：证据＝`orz-host/src/process_tree.rs` 孤儿清扫已在树；索引条目与 §8 桶同批翻账；**S4 真机读数仍待**；本批不动计数（0bq 仍开放，勾选保持未闭合）。
- **闭合（2026-09-27 尾巴批；计数 58 → 57，状态 `partial` → `implemented`）**：S4 真机判据达成（0BV S4 档 §1.3：出身登记在册＋收尾扫净 20 目标＝19 `not_running`＋1 `fingerprint_unknown` fail-closed 拒杀不误杀、机面核查零孤儿零残留）⇒ 判据「主进程结束而派生进程仍存活」零复现。入口档：[`097 尾巴闭合档`](docs/audits/097_TAIL_CLOSURE_AND_FLIPS_2026-09-27.md)。
- 入口：BACKLOG 0bq / [`HOST_RESOURCE_SAFETY_DESIGN`](docs/HOST_RESOURCE_SAFETY_DESIGN_2026-09-12.md) / [`TODO P1-0bo`](#p1-0bo-合并狗粮长轮0bm-审查裁决落地批0bn-必定压缩复审补口并为一轮p12026-09-24-用户令并轮立项backlog-0bo来源0bm0bn-两个候选狗粮轮用户令将其合并为一执行形态狗粮长轮不直接处理)。


### P1-0bv 0bs c 轮残余承接与检索真网复验杂项轮（可处理五件）（P1；2026-09-26 用户令立项，BACKLOG 0bv；56 → 57；来源＝[`0BS 后半部分处理报告（第二轮）`](docs/audits/0BS_PROGRESS_2026-09-26c.md) §二＋§三＋[`真网在线验证`](docs/audits/0BS_RETRIEVAL_ONLINE_PROBE_2026-09-26.md)；**执行形态＝杂项狗粮轮**）

- [x] **可处理五件（S1 勘定→S2 落码＋钉子→S3 载体→S4 真机）**：① **浏览器 SERP 链首 resource 接缝**（⑧ 链首；`registry/types.rs` `BrowserSerp` 能力类型＋宿主装配点＋按 ⑧ 注记语义并入）**＋预算单账本并账**（并入 `SerpSearchBudget`）。② **⑬ 余项**＝工作区切换（B 形态）＋单击切换/权限总览——**2026-09-26 用户裁决：留在 0bv 就地按 B 形态执行**（B 形态＝服务随启动而立＋信任清单全局共享＋服务内点击切换，A 单例否决；切换语义与门禁沿 0bs ⑬ S1 要点 ③④⑤；S1 已定稿 ⇒ 不再等形态裁决）。③ **f13** 编辑歧义失败回执给候选行号。④ **f14** 替换后行长剧变提示。⑤ **f15** 压缩窗口回执给摘要块落点示例骨架。（S2 五件全落＋S3 载体 0.7.2→0.8.0 达成；S4 主体达成〔0BV S4 档 2026-09-27：①让渡语义／②运行中禁切〕，**余矩阵 ③④ UI 级重走＋链首承接采样随尾巴批**；勾选补记 2026-09-27）
- **观察·记录不入**：f16／f17。
- [x] **S3 载体（2026-09-26 达成：0.7.0 双平台，[`080 重建档`](docs/audits/080_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-26.md)）**＝c 轮树面 ⑨⑩/⑧/⑫/⑬ 已进双平台载体（Win `.0.6.17-bak` 链 MATCH 3/3＋ACAF 重 provision＋探针全过〔归档 27 件〕；Linux `.0.6.15-bak` 链 0.6.15→0.7.0 直跨＋ELF static-pie `PT_INTERP=0`＋双向冒烟 6/6；字面量保持面零回退＋c 轮新面全部进体）；**S4 真机（未跑）**：真网补读＝DDG 代理腿／百度 link 壳展开／指纹 off 对照／arXiv 回落腿 UA。
- [x] **提交推送与发行（2026-09-26 达成：081 批，[`081 提交推送与发行档`](docs/audits/081_SUBMIT_PUSH_AND_RELEASE_2026-09-26.md)）**＝orz 两笔（`abba6886` 落码／`c5245558` bump 0.7.0）推 `cli`＋源清单 1498→**1500 条**＋pin；双平台打包 `rel-081-stage`（zip 28,556,923 B `d099d9f3…`／tar.gz 37,008,587 B `78cec631…`；解包回读 **6/6 MATCH**＋容器 `sha256sum -c` 全 OK）；**GitHub Release `v0.7.0` 发布**（相对已发布 0.6.13 的首个正式增量版；中间载体 0.6.14–0.6.17 不单独发行）＋README 发布面对齐；**计数 57 不变**。
- [x] **同载并件（2026-09-26 用户令「083审查中的待优化与处理项全部进0bv」；编号 REV-083-* 保留、执行随 0bv、闭合随 0bv、不新增计数；清单＝[`083 审查档 §8`](docs/audits/083_FULL_PROJECT_REVIEW_2026-09-26.md)，明细逐条见 BACKLOG `### 0bv.` 并件块）**：REV-083-02 已升级 0bw、-17 已闭合〔083 批〕、-05 拆分面已随 0bs ⑭（本件增量＝orz-loop 集成测试目录），其余 **18 项**随本件执行——01 权限语义收敛／03 审批叙事收口／04 README 工具面口径批＋ADR 状态行 frozen→evolving／06 journal 口径二选一／07 compute_event_hash 守护测试／08 signer respawn 旧票据补偿／09 ACAF 收口批次／10 journal writer 移出阻塞线程／11 SSRF 解析 pin 连接／12 panic 契约／13 参数表三列化入 ADR／14 判据钉纪律 fixture 先红后绿／15 assurance 冻结宣言落账／16 GAP-SPAWN-ORPHAN-RECLAIM 状态 pending→partial／18 P3 小修集合／19 流程（小步提交＋双会话 status 双确认）／20 LIF 观察判据＋复裁日期落账。（01/03/04/06/07/08/10/12/13/14/15/16/18a–h/19/20/21/22 全落〔092／094 批〕；**余 REV-083-09＝独立验票执行器排期＋shadow pre-signing 拒绝落账，随尾巴批执行**；勾选补记 2026-09-27）
- [x] **同载并件二（2026-09-26 用户令「未竟的后续批直接进0bv吧」；编号随 0bw 史、执行随 0bv、闭合随 0bv、不新增计数；来源＝[`0BW 报告 §6`](docs/audits/0BW_WRITE_CONTROL_IMPLEMENTATION_AND_FRICTION_2026-09-26.md)，轮 `RUN-CLI-6ab7b7eb` completed·109 轮·v1 全绿）**：① Linux Landlock 落码／② 运行时载体完整性自检／③ 专用 journal 事件族（block/warn 结构化）／④ L4 git 检查点·journal undo／⑤ CFA enforce 翻转裁决（待 1124 读数，runbook 已备）／⑥ S3 载体·S4 真机；狗粮观察项（warn 噪音／提示行渲染／block 姿态）随 0bv S4 收读。**F7 处置＝采 B（读取侧转码梯）并已收口**（journal 实证主链已在役；补 bash 提示词兜底臂／索引预览／标题抽取三处梯＋钉两枚；PS 5.1 内部误读＝捕获不可达边界，详 [`0BW 报告 §9`](docs/audits/0BW_WRITE_CONTROL_IMPLEMENTATION_AND_FRICTION_2026-09-26.md)）。（①②③④ 已落码并随 0.8.0 进在役〔094 批；Landlock ABI=3 真机读数／首个 carrier-manifest／`write_control_review` 全链／`orz rollback` 回环〕；⑤ 经两轮 1124 读数空裁决**不翻**；⑥ S3 载体达成＋S4 真机读数随 0BV S4 档达成〔零拦截零误拦／L2 零痕迹〕；勾选补记 2026-09-27）
- [x] **同载并件三（2026-09-26 用户令「F7进0bv，跟着REV-083 18项一起」；编号随 0BV 轮摩擦史、执行随 0bv、闭合随 0bv、不新增计数）**：**读取侧转码梯覆盖面扩张**（0BV 轮 `RUN-CLI-6ab7d8b7` 摩擦 F7）——冒烟日志经 GBK 控制台读出乱码；journal 实证两轮执行均走 `run_terminal_cmd`（seq 1871 `output_encoding=gb18030`／1880 `utf-8`，后端编码门已识别）⇒ 缺口在下游消费臂，086 梯三处（bash 提示词兜底臂／索引预览／标题抽取）未覆盖该臂。**执行纪律＝先勘定后落码**：复现勘定（重跑 node 冒烟、核模型面实收字节）分两支——① 梯臂真缺 ⇒ `decode_text` 接入该臂＋钉子（GBK 字节经梯可读＋标签如实）；② PS 5.1 管道写侧已重编码（信息已丢）⇒ 机械面不可救，落点＝既有纪律面（bash 工具描述已有「CJK 乱码先设 `[Console]::OutputEncoding=UTF8`」提示语；0bw F7-B 已登记「PS 5.1 内部误读＝捕获不可达边界」）＋模型未执行纪律的事实记档。归属＝0bw F7 摩擦史延续（同族）。（2026-09-27 尾巴批执行：勘定分支①实证——截断臂 `from_utf8_lossy` 预摧毁 GBK，红测复现；`maybe_truncate` 改走 `decode_text` 梯＋`truncation_decode_label` 标签留档＋钉子先红后绿；orz-tools **2973/0**）
- [x] **F4 预置红已修（2026-09-26 用户令「F4直接修吧」，主会话批）**：orz-loop 探针测试两臂断言同步 0bs c 轮缺省引擎链（`retrieval/projection.rs` 直连 `360search,baidu`／代理 `360search,baidu,duckduckgo`＋两行注释；`immediate_delivery.rs` 的 `bing_cn` 为自建解析器夹具、非缺省链断言，不动）＋顺带收编本批两文件 fmt（`agent_loop.rs:1333`／`host_exec/serp.rs:51`，沿 0bs ⑤「fmt 仅整理本批改动文件」口径）；读数＝定向 2✓、orz-loop 全量 **852✓/0✗**（预置红清零）、`cargo fmt -p orz-loop --check` 净。随 0bv 落账批提交。
- **S2 五件落码＋S3 载体达成（2026-09-26／27）**：轮 `RUN-CLI-6ab7d8b7` 五件全落（① 浏览器 SERP 链首接缝＋预算单账本并账／② 工作区切换 B 形态全栈＋单击切换／③ f13 候选行号／④ f14 版式剧变提示／⑤ f15 落点骨架单一源；裁决 D-c…D-g 见 [`0BV 轮报告`](docs/audits/0BV_SERP_LANE_AND_WORKSPACE_SWITCH_2026-09-26.md)）＋**F4 预置红修复**（`retrieval/projection.rs` 两臂断言同步，orz-loop 全量 852/0）；**S3 载体＝0.7.2 双平台已换装进在役**（[`089 重建档`](docs/audits/089_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-27.md)）＋090 批落账推送（orz `6018b540`／`71af0ee3`）。**S4 真机未跑**（SERP 链首真网读数／切换矩阵四点／0bw 三读数）；并件 24 项（REV-083 18＋0bw 六）**本轮未开工**，顺序建议见 0BV 报告 §6-R2。
- **结转批落账（092 批，2026-09-27）**：REV-083 18 项处置＝07／08／10 三小修全落＋18a–h 八个拆项全落＋01 代码面＋09 告警面；文档面（01/03/04/06/13/15/20）已落字（ADR-0010 v1.82 §14.79–§14.81）；余 05／11／12／14／16／19 与 0bw①–④ 随轮；F-1 案例化（[`ORZ-PS1-BULK-REWRITE-001`](docs/incidents/ORZ-PS1-BULK-REWRITE-001.md)）；载体 0.7.3 已双平台换装（[`091 重建档`](docs/audits/091_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-27.md)）。**S4 真机仍未跑。**
- [x] **S4 真机复验达成＋摩擦并件四（2026-09-27 用户令「请进行0bv S4吧」→「请将本轮遇到的摩擦项并进REV-083 余项中吧」；编号 REV-083-21／22、执行随 0bv、闭合随 0bv、不新增计数；读数档＝[`0BV S4 真机复验`](docs/audits/0BV_S4_LIVE_VERIFICATION_2026-09-27.md)，读数明细见 BACKLOG `### 0bv.` S4 块）**：**REV-083-21（P1，服务端）**＝orz-web 切换信任门与 `resolve_root` 在 Windows 恒 403——`std::fs::canonicalize` 的 `\\?\` verbatim 前缀 × `path_eq`（`orz-web/src/server.rs:108-116`）不剥 ⇒ 比较恒 false（连当前已信任工作区自身也拒；`?root=` 面同病；单测未覆盖真实 canonicalize 形态＝「符号在位≠接线」族第三例）；修复＝剥 verbatim（含 `\\?\UNC\`）＋补形态单测。**REV-083-22（P1，前端）**＝`orz-web/assets/app/api.js:74` `switchWorkspace` 引用未定义符号 `sendJsonBody`（仅 `sendJson` 在册）⇒ UI 切换 ReferenceError 从未发出请求；修复＝补函数或扩展 `sendJson`＋冒烟补真实 fetch 断言。两缺陷独立且串联同链 ⇒ 修复同批＋重建载体后重走切换矩阵补 ③④ 面与链首就绪承接采样。S4 其余读数＝② 运行中禁切✅／0bq 零复现（具备闭合讨论条件，翻转待裁）／0bw 零误拦＋CFA 1124 零行 ⇒ ⑤ 维持不翻。
- [x] **过夜批＝并件余项与 REV-083-21/22 修复落码（2026-09-27 晚用户令；档＝[`0BV 并件余项与 S4 修复批`](docs/audits/0BV_REMAINING_ITEMS_AND_S4_FIXES_2026-09-27.md)）**：REV-083-21/22 修复＋11 SSRF pin＋05 集成测试目录＋12 退出码文档＋14 执行实例＋0bw① Landlock＋0bw② 载体自检＋0bw③ 事件族全链＋0bw④ undo 面全部落码（明细见 BACKLOG 0bv 过夜批行）。**同日复审与修复✅**（用户令「由你裁决并处理全部问题」；P0×1/P1×2/P2×11/P3×9 全处置；D-9 restore 目标域收敛＋第四消费点＋原子写／D-10 symlink 不授权／timeout 臂补收／schema 封闭集＋第 7 件反例／空命令防线／配对双侧判官＋对拍 258／D-12/13 装挂永不 fail spawn＋架构门／D-14 dunce 去剥除／carrier 键组件判定＋未列文件／CLI 严格化／D-9…D-15 在档；明细见档 §12 与 BACKLOG 0bv 复审行）。**余**＝落账批（093，小步提交＋双会话 status 双确认）→载体重建批（0.7.4：manifest 生成＋换装＋矩阵 ③④ 重走＋`orz rollback` 真机＋Landlock 真机读数）→0bw⑤ CFA 翻转（待 1124 读数）→0bq 翻转（待裁）。
- [x] **落账与载体回执（094 批，2026-09-27）**：orz `ee417b80`（29 文件 `+3067/−238`）＋`c25e459a`（bump 0.8.0）推 `cli`；父仓 pin＋源清单 **1501 → 1506 条**；载体重建已按 **0.8.0** 执行（093 批，非登记的 0.7.4）——首个 carrier-manifest、Landlock exec 后真机读数 ABI=3、`rollback` 真机回环、切换矩阵 ③④ API 级；余＝矩阵 ③④ **UI 级重走**＋链首承接采样＋0bw④ git 半边裁决＋0bq／0bw 状态翻转待裁。入口：[`094 落账档`](docs/audits/094_SUBMIT_PUSH_2026-09-27.md)。
- [x] **发行回执（095 批，2026-09-27）**：用户令「请进行发行吧」——双平台 0.8.0 载体打包（`rel-095-stage`：zip `db5877d1…` 28,671,984 B／tar.gz `1c1afd1b…` 37,160,321 B／顶层 `SHA256SUMS`；包内六件 6/6 MATCH＋容器 `sha256sum -c` zip 4/4・tar 4/4・顶层 2/2 全 OK＋双平台 `--build-info` 读 0.8.0）＋**GitHub Release `v0.8.0`**（相对已发布 v0.7.0；三资产 digest 与回下载逐位一致）；余＝矩阵 ③④ UI 级重走＋链首承接采样＋0bw④ git 半边裁决＋0bq／0bw 状态翻转待裁。入口：[`095 发行档`](docs/audits/095_RELEASE_2026-09-27.md)。
- **尾巴批执行与闭合（2026-09-27 用户令「清尾巴」；计数 56 → 55，状态 `pending` → `implemented`）**：并件三落码（见上）＋REV-083-09⑤ shadow pre-signing 拒绝照常落账（`fail_closed_refusal` 双模化＋e2e 钉 24/24）＋② 独立验票执行器登记 `candidate` 排期项（触发器＝下次 ACAF 面改动批并入，不占计数）＋矩阵 ③④ UI 级补走达成（旧会话只读回放不清场／B 区新会话文件面双确认；② 回归保持；① UI 面结构性不可点、403 维持 093 API 级）＋链首就绪承接采样两跑未复现变体（v2/v3 就绪后仍让渡 `browser_unavailable`；接缝＝就绪信号跨激活不传播 ⇒ 登记 OBS candidate 不占计数，留下次检索面批勘定）＋0bw④ git 半边裁决不实施；f13/f14/f15 维持 S2 读数、自然采样随后续轮。入口档：[`097 尾巴闭合档`](docs/audits/097_TAIL_CLOSURE_AND_FLIPS_2026-09-27.md)。
- 入口：BACKLOG 0bv / [`0BS 后半部分处理报告（第二轮）`](docs/audits/0BS_PROGRESS_2026-09-26c.md) / [`真网在线验证`](docs/audits/0BS_RETRIEVAL_ONLINE_PROBE_2026-09-26.md) / [`0BW 报告`](docs/audits/0BW_WRITE_CONTROL_IMPLEMENTATION_AND_FRICTION_2026-09-26.md)。


### P1-0bw 写入管控：机械层收窄锁死＋命令审查留痕＋补偿自检（P1；2026-09-26 用户令立项「请将本轮内容同步进index等相关文档吧，并将写面的完善内容直接立项成为正式待办」，BACKLOG 0bw；57 → 58；来源＝[`083 全面审查档 §10.2–§10.6`](docs/audits/083_FULL_PROJECT_REVIEW_2026-09-26.md)（四轮收敛定案：业界对照＋L0–L4 映射＋安全定位）；**执行形态＝S1 设计档先行＋Windows CFA audit 探针，非狗粮轮**）

- [x] **S1 设计档**：收窄锁死面 deny 单一源表（Win／Linux 系统核心与组件路径初版）＋三落地点勘定（工具面写入路径／`run_terminal_cmd` 命令面／进程面）＋CFA 探针方案＋与 0z（真机资源安全边界）边界分工表。（2026-09-26 随 v1 达成：[`WRITE_CONTROL_MECHANICAL_DESIGN`](docs/WRITE_CONTROL_MECHANICAL_DESIGN_2026-09-26.md)；勾选补记 2026-09-27）
- [x] **CFA 探针（先行，沿 ACAF「先影子后翻转」惯例）**：Windows 受控文件夹访问开 audit 模式（只记事件 1124 不拦截）→ 狗粮轮实测 → 读 1124 测系统目录覆盖与误伤率 → 据实裁决 enforce 翻转；失败回落候选 B＝AppContainer/受限令牌。（非提权读数两轮＋0BV S4 真机轮 1123/1124 轮前轮后均零行 ⇒ **⑤ 裁决维持不翻**〔判据输入空〕；提权 enable＋读数批留用户排期，runbook `.tmp-0bw-cfa-audit.ps1` 已备；勾选补记 2026-09-27）
- [x] **S2 落码＋钉子**：① 锁死面三落地（deny 表为机械层单一源；工具面写路径 canonical 化复用读面 `resources.rs` 同族实现；**载体自保护**〔orz 安装目录／`.gsa`／journal／存档／本管控配置自身〕并入表，配置完整性纳入 manifest 校验——无 allowlist 形态下的承重墙约束）；② L2 命令面机械审查（借 Codex execpolicy 形态：规则库＋fixture 钉＋block/warn/allow 三分类；deny 覆盖「翻转安全机制」类命令〔Defender cmdlet 等〕；定位＝best-effort 风险闸＋全程留痕）；③ L3 Linux＝Landlock（spawn 时顶层目录枚举 allow 写权限、核心集除外、读不设限；seccomp 网络过滤先例同型接线）；④ L4 补偿（git 自动检查点或 journal undo〔content anchor 前像复用〕＋载体完整性自检〔`orz_source_manifest.sha256` 复用〕）；⑤ 安全口径三档措辞改写（README 安全节＋`orz/SECURITY.md`：保证／阻力／审计）。（v1 2026-09-26〔`1682699e`〕＋①–④ 过夜批 2026-09-27〔`ee417b80`，随 0.8.0 进在役〕；git 自动检查点半边裁决随尾巴批；勾选补记 2026-09-27）
- [x] **S3 载体／S4 真机**：随重建与真机轮实测（锁死面拦截读数／L2 留痕读数／Landlock 生效面）。（S3＝0.7.1→0.8.0 历批载体全进体；S4＝0BV S4 真机轮读数：零违规零误拦／L2 零 block-warn 痕迹〔与实现一致〕／Landlock ABI=3 exec 后真机读数〔093 §8〕；勾选补记 2026-09-27）
- [x] **v1 达成（2026-09-26 狗粮轮 `RUN-CLI-6ab7b7eb`·completed·109 轮）**：S1 设计档（[`WRITE_CONTROL_MECHANICAL_DESIGN`](docs/WRITE_CONTROL_MECHANICAL_DESIGN_2026-09-26.md)）＋S2 v1 落码（工具面/命令面/载体自保护＋install_dir ⊆ cwd 文件级降级；write_control 12／exec_policy 5／search_replace 136／bash 233 全绿）＋CFA 探针非提权读数＋SECURITY/README 三档措辞；裁决 D1–D6/X1–X7、摩擦 F1–F10 留痕（报告 [`0BW`](docs/audits/0BW_WRITE_CONTROL_IMPLEMENTATION_AND_FRICTION_2026-09-26.md)）。**未竟六项经用户令并件 `P1-0bv`（执行随 0bv、闭合随 0bv）**；本条目闭合仍随自身史（落账批）。**0.7.1 双平台载体已进体（087 批）＝并件⑥ S3 载体达成**（Windows 3m57s／Linux musl 10m42s，全核证过，[`087 重建档`](docs/audits/087_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-26.md)）；S4 真机随 0bv 轮。
- 边界：**allowlist 不做**（2026-09-26 用户裁决——读层沙箱动机＝先读后写保障＋隐私，写层只有「安全」一义，不做限制性可写根）；**安全定位＝高阻力＋强审计、非绝对保证**，对外不做绝对安全性声明（083 档 §10.6）。
- **落账达成（2026-09-26 088 批）**：orz `1682699e`（0bw v1＋086 F7-B 收口）＋`42a14d16`（bump 0.7.1）推 `cli`；父仓 pin＋源清单 1500 → 1502 条；门禁 `valid: true`（[`088 落账档`](docs/audits/088_SUBMIT_PUSH_2026-09-26.md)）。**状态翻转与计数待裁**：L3 Linux Landlock／运行时载体自检／专用 journal 事件族／L4 未落码，直接记 `implemented` 与状态词表冲突（候选 `partial`）；**本批计数 58 不变**。
- [x] **0bw④ git 半边裁决（2026-09-27 尾巴批裁决）**：git 自动检查点**不实施**——undo 面（`orz rollback`）＋编辑面回退窗口＋载体完整性自检已交付 L4 补偿目标；git 自动检查点必须写用户仓库状态（refs/stash/对象库），与「orz 不动用户 git 状态」边界冲突且容器／触发／回收无干净解；剩余风险归 L1 锁死面与 L2 审查职责域。登记设计留档（[`097 尾巴闭合档 §1.5`](docs/audits/097_TAIL_CLOSURE_AND_FLIPS_2026-09-27.md)）。
- **闭合（2026-09-27 尾巴批；计数 57 → 56，状态 `pending` → `implemented`）**：S1/S2/S3/S4 全达成＋⑤ CFA 维持不翻＋⑥ 载体与真机读数达成＋④ git 半边裁决落定（上条）⇒ 0bw 全量闭合，索引 §8 同批翻转。
- **结转批处置（2026-09-27）**：⑤ CFA enforce＝不翻（1124 空、无读数）／⑥ S3 载体达成（0.7.2／0.7.3）、S4 真机仍待；①–④ 结转下一批。**闭合与计数 58 → 57 仍待用户裁决。**
- 入口：BACKLOG 0bw / [`083 全面审查档`](docs/audits/083_FULL_PROJECT_REVIEW_2026-09-26.md)。


### P1-0bx 浏览器会话就绪判定（hand-off 形态）修复（P1；2026-09-27 用户令立项并同批处理，BACKLOG 0bx；54 → 55；来源＝097 批观察项 `OBS-SERP-READY-HANDOFF-CROSS-ACTIVATION` 真机复核改判）

> 机理：`--user-data-dir` 为会话固定 profile，已有实例在跑时第二次 `launch` 把请求交付（hand-off）给既有实例后立即退出 ⇒ 只认「子进程存活」的 `is_alive()`／`ready()` 恒否 ⇒ 宿主逐调用重复启动、`web_search` 链首恒以 `browser_unavailable` 让渡（097 采样 `RUN-CLI-6ab91021` 5/5）。修复＝端点兜底（profile 端口一致＋回环应答，TTL 1000 ms）＋`shutdown` 关停位压制。

- [x] **S1 勘定**：097 采样 journal 复算（5/5 浏览器调用各落启动事实＋同调用可读页面）＋代码路径勘定（`cdp.rs` 启动／`is_alive`／`ensure_browser_launched` 与 `browser_serp.rs` 链首）。勘察结论＝机理改判（非「跨激活不传播」）。
- [x] **S2 落码＋钉子**：`endpoint_liveness` TTL 缓存＋`profile_names_port`＋`endpoint_listening`＋`shut_down` 位；三枚钉子＝真机 hand-off 复现钉（**先红后绿**）／端点三态钉／关停钉；读数 cdp 组 37/0。
- [x] **S3 载体**：0.8.1 双平台重建换装并进包（100／101 批；MATCH 3/3 ×2＋ACAF 重 provision＋包内清单活体两态）＋发行 `v0.8.1`。
- [x] **S4 真机**：agent 级采样实证 `web_search` 链首**承接**（不再让渡），或如实让渡于真不可用；同时作为 89 题重跑前的就绪性闸。**2026-09-27 达成（102 批）**：容器形态检索题 `RUN-CLI-6ab93312` 链首承接（`serp-attempts` google ok 9 命中／7 814 ms＋启动事实 1/15＋零让渡）；桌面 hand-off 预置条件 `RUN-CLI-6ab9363d` 启动事实 1/10＋`browser_read` 9/9，对同条件 0.8.0 对照 `RUN-CLI-6ab93831`／`RUN-CLI-6ab93aaa` 的 32/32・39/39。
- [x] **闭合**：S3／S4 达成后 `partial` → `implemented`，计数 55 → 54（闭合判据＝真机链首承接读数）。**2026-09-27 落账（102 批）**：判据达成，状态翻 `implemented`，计数 55 → 54。
- 入口：[`102 闭合档`](docs/audits/102_BROWSER_READY_HANDOFF_S4_CLOSE_2026-09-27.md) / [`099 就绪判定档`](docs/audits/099_BROWSER_READY_HANDOFF_2026-09-27.md) / [`097 尾巴闭合档`](docs/audits/097_TAIL_CLOSURE_AND_FLIPS_2026-09-27.md) / BACKLOG `0bx` / `orz-host/src/local_browser/cdp.rs`。


### P1-0cc 写控宿主机灾难保底收窄（0cb 规则 5 载体自保护退役）（P1；2026-09-29 晚用户裁决立项，BACKLOG 0cc；57 → 58；来源＝build-pov-ray 0.8.5 重跑结构性 0——7 拦全封闭枚举内 `carrier-write`：`/usr/local/bin` 安装 ×4〔含 `/tmp/lnk` 软链绕道识破〕／`.gsa` ×2／`_bgprobe` ×1；模型构建渲染 3/3 自证、结束自述 `reason=blocked`；run `RUN-CLI-6abbb013`）

> 用户裁决：「应该继续收窄，把灾难保底纯粹变成宿主机灾难保底吧，毕竟只要不重建，orz实际上不会被即时破坏」。方向＝保底只保护「会话结束后仍存在的宿主状态」：容器化评测面内载体文件（orz 安装目录／三件套／`grok-home/`）为**易弃状态**不设位；`.gsa` 会话卷（宿主 bind mount）**保留**；规则 1–4 定位不变；Windows 宿主原生面目标集重述＝S1 裁量点。爆炸半径（49 题测试面已扫）＝未跑面仅 b5-14 `build-pmars` 同构。批序＝S1 设计稿（契约面 schema v0.3 规则类别＋legacy 回放面，判官/Python 镜像/fixture 同批）→ S2 落码＋审查 → S3 双平台重建 **0.8.6**＋身份门换装 → S4＝TB21 5 题重跑（build-pov-ray 第一＝翻盘实锤）＋b3-13 断点续跑（**重跑线自 09-29 晚暂停至 0.8.6**）。

- [x] **S1 设计稿**（2026-09-29 晚完成：0cb 设计档升 **v3.0**——规则 5 缩为宿主状态两窄目标〔`.gsa` 会话卷＋ACAF keystore 根〕；安装目录／三件套／`grok-home`／⊆cwd 降级**双面全退役**〔§2 条 5 四点论证：守卫自保循环论证／恢复成本分类／Windows OS 文件锁／keystore 信任锚例外〕；**契约面零变化＝schema v0.3 枚举不动**〔`carrier-write` 沿用、描述不绑定目标集〕）。
- [x] **S2 落码＋审查**（2026-09-29 晚完成，120 批，用户令放行；orz `397ba7cc`）：`write_control.rs` 载体集重定义＝C1＋C2′ keystore 根/signer manifest（`HostStateTargets` 装配 env 解析）＋C2/C3 退役（`CARRIER_BINARY_NAMES`/`CARRIER_PROTECTED_SUBDIRS`/`current_install_dir`/`install_dir_hit` 删表；新 `carrier:keystore-root`/`carrier:signer-manifest` 文案带规则 id）；`exec_policy.rs` 规则 5 同步（退役面放行钉＝`/usr/local/bin` 安装/软链指入/三件套/`grok-home`/`_bgprobe`；负向集增 keystore 根＋manifest 两族＋Linux 容器形态）；**L3 Landlock 排除集收窄＝`LINUX_DISASTER_KERNEL_FACES` 恰 4 项（/boot /dev /proc /sys），载体面系统树放行〔S4 成败项闭合〕**；`search_replace`/`rollback_maintenance` 消费面同步。读数＝orz-tools **2986/0/6**（+3）・assurance **278/0**・runtime conformance **378/0**〔契约面零 diff 实证〕・tui **178/0**・loop 859/0/3 与 host 353/0/6（各 1/2 存量环境敏感，stash 基线对拍同红）・clippy 13/99 基线持平・触碰面 fmt 零 diff；反馈面核证达成＝L2 block 文案带规则 id＋工具面文案带来源标识＋L3 为 shell EPERM 既有形状，不新增机制。
- [x] **S3 双平台重建 0.8.6＋换装＋身份门更新**（2026-09-29 晚完成，121 批；源冻结 orz **`f95e1831`**＝`397ba7cc` S2＋bump 0.8.5→0.8.6）：Windows 4m17s 换装 MATCH 3/3（`cbe39aff…`/`4638fbaf…`/`363d05f4…`）＋ACAF 重 provision 绑 `4638fbaf…`（keystore 两件逐位未动）＋进体判据＝新规则面四值进体＋退役面零残留＋版本串 0.8.6；Linux musl 22m23s 直写 MATCH 3/3（`979a38fa…`/`c01095eb…`/`16bcf6fb…`）＋static-pie×3/INTERP=0＋alpine/bookworm 双冒烟；`run_r0_heavy_official.py` 身份门换装（载体 → `979a38fa…`，适配器 `6d55c26e…` 未动）；打包 `rel-121-stage` 两态全绿（容器 4/4・4/4・2/2、活体两态 0/1 条）；未推送未发行。
- [x] **S4 读数**（build-pov-ray 重跑翻盘＝收窄实锤；0cb S4 余项合并执行）。**首读达成（2026-09-30，0.8.7 在役）**：`official-v41-rerun2-build-pov-ray`〔run `RUN-CLI-6abbf664`，14m17s／12000s，exit 0〕**reward＝1.0**＝翻盘实锤（3 测试全过，Harbor 公开上传）；拦截恰 1 条＝**v3.1 祖先链臂实战首拦**（`/etc` 写侧＝keystore 根 `/etc/orz-acaf/keystore` 祖先），`/usr/local/bin` 安装面全放行、结束自述 `reason=completed`；止损门未触发。**重跑系列 5/5 完毕（2026-09-30 晨，123 批）**：＋torch-pipeline-parallelism **1.0 翻盘**（21.2 min）；extract-moves-from-video 0.0（31.8 min）／git-multibranch 0.0（16.5 min）／winning-avg-corewars 0.0（43.6 min）仍 0 属模型侧余量、不再追——**2/5 翻盘，全部 exit 0、公开上传、一次成功**。**下半场第一窗 12/44 完毕（04:27–08:57，official-v41-second-half 驱动器，08:50 门收口）**：B3 批 7 题收官 5 过 2 不过、B4 推进 5/18＝4 过 1 不过，0 作业失败；轮累计任务级 **46/56＝82.1%**。**同日用户裁决：b3-13 起未跑面（44 题）直接重跑、不续跑**——载体换版（0.8.4→0.8.7）后同一道轮跨包体版本续跑不合适，`run_official_v41_full.py` 断点续跑形态退役（判定档 §10）；**余 32 题（b4-06…b4-18＋B5×19）下窗照判定档 §7 状态卡续跑**。**收官（2026-10-01 用户裁决「0cc 已经做完了，89 题已经完成」）**＝未跑面并入 89 题整轮直接重跑完成，整轮成绩报告落档〔142 批〕＝任务级 **73/89＝82.0%**（官方轮读数以报告口径为准）——S4 达成。
- [x] **闭合**：S1–S4 ⇒ `implemented`，**62 → 61**（2026-10-01，145 批，用户裁决；立项时目标 58 → 57 的差额为后续 0cd/0ce/0cf/0cg/0ch 立项与 0cb 并件的计数演变，闭合时点以当前计数为准）。
- 入口：BACKLOG `0cc` / [`0cb 设计档 v3.1`](docs/WRITE_CONTROL_BACKSTOP_REVISION_DESIGN_2026-09-29.md) / [`TB21 判定文档 §10 追记`](docs/TB21_V41_RERUN_SCOPE_DETERMINATION_2026-09-29.md)。


### P2-0cd 死代码清退——COMPACT_SYSTEM_PROMPT 常量与 accessor（P2；2026-09-30 用户裁决立项，BACKLOG 0cd；58 → 59 立项、同批闭合 59 → 58，净 58 不变；来源＝桌面调查「DeepSeek 官方跑分配置开源状态」对照自身实证——`COMPACT_SYSTEM_PROMPT`（orz-agent template.rs 两句版）与 `Agent::compact_system_prompt()` 全仓零调用方、TB 无头路径系统提示词＝空串在役）

> 用户裁决：「请给清理死代码部分立项吧，并直接开始实施」。背景＝THIN-HARNESS-REDESIGN-V2 §9.1「Prompt 全空」（2026-08-29 用户裁决）在役实证：`orz-loop/src/prompt.rs:26` `BASE_SYSTEM_PROMPT = ""`（测试锁死）、`agent_loop.rs:2922` 主代理臂 `build_system_prompt(None)` ⇒ 空串；本轮 46/56＝82.1% 即零提示词姿态读数。TB 提示词姿态维持零句（用户令「tb维持无提示词即可」；是否加一句「请彻底完成这一任务」留 0.8.8 代裁决，本轮身份门钉死不动）。

- [x] **S1 落码**（2026-09-30，124 批，orz worktree 态）：template.rs 常量本体＋doc 注释删除／`test_compact_prompt_matches_expected` 删除／`test_mid_session_switch_concise_to_full` 瘦身更名 `test_full_prompt_has_tool_sections`（保留 render_base 工具节覆盖）／agent.rs accessor 删除；2 文件（agent.rs −7／template.rs +1/−21）；**源冻结 orz `20e4c574`**（`79a3e8e5` → `20e4c574`，子模块内提交）。
- [x] **S2 验证**（同批）：全仓 grep 零残留；orz-agent 测试 **573→572/0**（stash 基线对拍恰 −1）；orz-workspace `cargo check` 过；clippy orz-agent 本体 **0**（依赖闭包 16 条＝存量基线）；触碰面 fmt 零 diff；载体 0.8.7 在役二进制零重建、TB 轮身份门（载体 `3332b38f…`／适配器 `6d55c26e…`）不受影响；源清单重生成 1506 条、门禁 `valid: true`；子模块已内提交、父仓未提交未推送（待令）。
- [x] **闭合**：`pending` → `implemented`（2026-09-30，124 批，立项即实施即闭合；计数 59 → 58）。边界另案＝orz-agent 模板族（product 面遗留、去人格已做、不进 eval 路径）与 `orz-subagent-resolution` 孤儿 crate。
- 入口：BACKLOG `0cd` / [`THIN-HARNESS-REDESIGN-V2 §9.1`](docs/THIN_HARNESS_REDESIGN_V2_DESIGN_2026-08-28.md) / 0bs 三模板转写清退先例（BACKLOG 第二卷 §1.32）。


### P2-0ce 死代码面勘定清退——orz-agent 模板族与 orz-subagent-resolution 孤儿 crate（P2；2026-09-30 用户裁决立项，BACKLOG 0ce；58 → 59；**登记暂缓实施**）

> 用户令：「剩下的无消费死内容也明确立项吧，但暂不实施」。范围两面（0cd 同日调查边界另案）：①orz-agent 模板族（`templates/prompt.md` 51 行 base／CODEX apply-patch 加密模板／SUBAGENT 加密模板／`ORCHESTRATOR_PROMPT_BODY`／TemplateOverride 渲染路径与 `Agent` 组装）——**在役消费面实证＝orz-bin→orz-host→orz-workspace→orz-agent 仅 `plugins`＋`prompt::skills` 两面**（`list_skills`/`SkillsConfig`/`plugins::discovery`/`plugins::trust`），模板/Agent/config 机制零服务消费、不在 eval 二进制路径、去人格已做（2026-08-15/16）；②orz-subagent-resolution 整 crate（3,018 行/7 文件；无工作区成员资格、无消费方、root `[workspace.dependencies]` 注册行悬空）。

- [x] **S1 勘定**（2026-10-01，146 批）：per-module 全仓消费表钉——外部消费实证**六边**（`plugins`／`prompt::skills`／`prompt::agents_md`／`repo::RepoDirChain`／`discovery::project_agent_dirs_in`／`config::workspace_grok_build_toolset`；原「仅 plugins＋skills 两面」为低估，六边全保留）；`apply_patch_template_source()` 产品出口已不存在（全仓零调用）随族退役。
- [x] **S2 落码清退**（146 批，orz `be4f90ff`）：删除面 15 文件（templates/ 3＋encrypt_templates.py＋prompt 4 件＋agent/builder/compaction/system_reminder/error）＋孤儿 crate 整删（3,018 行）＋root 注册行＋config.rs/discovery.rs 收敛最小件＋skills.rs 三个新死注入函数随批退役；加密模板连 plaintext 源一并清退、零转写留存。
- [x] **S3 验证＋影响面**（146 批）：workspace check 绿＋orz-agent lib 302/0＋orz-workspace 1,500/0＋orz-host lib 350/0＋clippy 触碰面零新增（104↔104 逐文件一致）＋fmt＋源清单重生成 1,485 条（1506−21 恰合删除面）＋门禁 valid；**连带修复＝serde_json `preserve_order` 统一链断裂**（原经 orz-agent→orz-sampling-types 隐式开启；链断致 orz-workspace 权限金样键序翻转 3 败 ⇒ orz-workspace/orz-host serde_json 边显式声明）。
- [x] **S4 台账＋闭合**：**61 → 60**（2026-10-01，146 批）；`pending` → `implemented`。0.8.8 在役载体零重建，源态随 0.8.9 代重建进体。
- 入口：BACKLOG `0ce` / 0cd 同族先例（P2-0cd，124 批闭合） / [`146 批档`](docs/audits/146_0CE_0CF_0CG_DEADCODE_GUIDE_WALLCLOCK_2026-10-01.md)。


### P2-0cf blackboard_read 工具描述补黑板说明书分区简注（P2；2026-09-30 用户裁决立项，BACKLOG 0cf；59 → 60；**登记随 0.8.8 代窗口实施——2026-10-01 144 批注记：0.8.8 窗口已随 0ch 发射，本项落点顺延 0.8.9 代窗口**）

> 用户令：「框架使用说明书在黑板上但目前这件事没有交代，后续在工具栏的 blackboard_read 后面再加一句简短的注解进行标注」。背景＝0bh ⑭⑮ guide 分区已实施在役（`orz-loop` controller.rs 1867＋tool_run.rs 2592）但模型面零交代——与本窗 section=session 拉取 0 次同构（判定档 §11.4）；范围恰一点＝blackboard_read 工具描述末尾一句简注（说明书在 guide 分区、`section=guide` 可读）。

- [x] **S1 措辞勘定＋落码**（2026-10-01，146 批，orz `be4f90ff`）：描述分区段末尾补一句（`0cf (2026-10-01): the framework usage manual lives on the blackboard too — section=guide reads it (mechanism-only, live-only, zero badges).`）＋section enum 增 `guide`（tool_run.rs 验证面本就放行，enum 缺位为文档缺口）。
- [x] **S2 文案钉/快照测试**（146 批）：blackboard F-012 测试双钉（enum 含 guide＋描述含简注）。
- [x] **S3 随 0.8.9 代重建进体**（2026-10-01，147 批）：Windows/Linux musl MATCH 3/3、build-info 0.8.9；字节判据＝简注 0→5、`section=guide` 0→5（两平台一致）。
- [x] **S4 闭合**：**60 → 59**（147 批）；`pending` → `implemented`。
- 入口：BACKLOG `0cf` / [`0bh ⑭⑮ 设计档`](docs/BLACKBOARD_GUIDE_AND_POINTER_DESIGN_2026-09-22.md) / 判定档 §11.4。


### P2-0cg 黑板 session 面墙钟提示拆除（P2；2026-09-30 用户裁决立项，BACKLOG 0cg；60 → 61；**登记随 0.8.8 代窗口实施——2026-10-01 144 批注记：0.8.8 窗口已随 0ch 发射，本项落点顺延 0.8.9 代窗口**）

> 用户令：「黑板的墙钟提示部分要拆下去才行，新立项这一部分吧」。背景＝130 批「透传墙钟不是标准做法」裁决＋全轮实测：session 面（`WALLCLOCK_ELAPSED/LIMIT/REMAINING`＋T̂ 剩余轮数行）仅 8 次/89 作业拉取，7 拉取作业 4 过 3 不过、**b3-09 看表仍撞墙**——拉取与收敛无正相关性。

- [x] **S1 落码**（2026-10-01，146 批，orz `be4f90ff`）：两渲染面拆除（session_face_block_with_wallclock 退役、session_face_block 收敛三参；build_status_line 去 rounds_line 参；T̂ 换算函数族全退役；controller wallclock_rounds_line 方法退役；`run_elapsed_wallclock_secs` 保留＝F6 push 仍消费）＋orz-bin argv/env 收口（argv→env 桥退役；`MAX_WALLCLOCK_INPUT` OnceLock 启动期一次解析＋`remove_var`＋Linux `argv_scrub` /proc/self/mem 原位零化〔best-effort 四钉；非 Linux＝已知边界〕）；`TOOL_ROUND_*` 行与 status 行保留；F6 push（默认 off）与 retrieval 保留量机械面不动；到期硬门 `run_invalidated{status: wallclock}` 语义不变。
- [x] **S2 测试**（146 批）：prompt.rs 五件 0am Part A 钉子退役＋session 面负向钉（无 `WALLCLOCK_`）＋controller/blackboard session 面断言改造＋argv_scrub 四钉。
- [x] **S3 随 0.8.9 代重建进体**（2026-10-01，147 批）：字节判据＝`WALLCLOCK_ELAPSED/LIMIT/REMAINING_ROUNDS` 1/2/1→0/0/0（两平台）、保留面 F6_BUDGET_CUE 2→2；**argv_scrub 实弹修复＋探针双绿**＝初建载体 `/proc/self/mem` 通道实弹失败（ps 仍见完整 argv）⇒ 直接裸指针写重设计（orz `e87b0630`），复测 ps cmdline 旗标＋值消失、`/proc/<pid>/environ` 读数 0。~~S3 前置＝无墙钟对照实验读数~~（已由 137 批核查与 rerun3 承载解除）。
- [x] **S4 闭合**：**59 → 58**（147 批）；`pending` → `implemented`。
- 入口：BACKLOG `0cg` / 判定档 §11.4。


### P1-0ca ADR-0010 分卷拆分（按大章节物理分卷；语义零增删；狗粮轮题面任务）（P1；2026-09-28 用户令立项，BACKLOG 0ca；56 → 57；来源＝0bm 未竟移交 ①）

> 用户令：「请给ADR-0010立单独项吧，拆解方式的话，按照大的章节拆即可」。对象＝`adr/ADR-0010-fusion-runtime-and-agent-architecture.md`（512 KB／5,989 行；§1–§13 ≈1,120 行，**§14 裁决索引独占 ≈4,869 行**）。执行形态＝**下一轮真机狗粮的题面任务**（上下文自然过压缩水位 ⇒ 0bz S2 真机单轮采集同轮承载；0bs S4 复验并跑与否留轮时定）。分卷＝物理拆分、**语义零增删零改写**；卷首保留权威声明＋分卷总目录；索引 §1 入口与全仓引用面同一变更内修正。（**2026-09-28 执行更正（115 批）**：用户令「直接做完这一项」⇒ 主会话直接执行完成——未经狗粮轮、0bz S2 采集未同轮承载；其余约束不变。）

- [x] **S1 分卷方案勘定**：章→卷映射表＋§14 处理（**裁定＝单卷不分层**：内部小节为追加序、非数值序，再分层无益）＋文件命名（`ADR-0010-vol-NN-<slug>.md`）＋主文件形态（**保留为入口页＋分卷总目录**）＋全仓引用面清单（142 处/68 文件）。**2026-09-28 达成**（115 批）。
- [x] **S2 执行**：机械分卷落盘 14 卷＋主文件改造；**字节级往返重组 sha256 与原文一致**（`973438d4…`）＝语义零增删零改写。**2026-09-28 达成**（115 批）。
- [x] **S3 引用面修正**：索引 §1 AUTH-ADR-0010 入口加注分卷＋非存档 **82 处**单章引用改指对应分卷（21 文件）＋全仓引用面清单与台账同步（索引头/§3/§8、BACKLOG、第二卷）。**2026-09-28 达成**（115 批）。
- [x] **S4 门禁与回读**：`check_repository.py` `valid: true`＋零断链＋分卷可读性回读达成。**2026-09-28 达成**（115 批）。
- [x] **闭合**：S1–S4 达成 ⇒ `pending` → `implemented`，计数 57 → 56。**2026-09-28 落账（115 批）**；详见 [`115 执行档`](docs/audits/115_ADR0010_VOLUME_SPLIT_EXECUTION_AND_FRICTION_2026-09-28.md)。
- 入口：BACKLOG `0ca` / [`115 执行档`](docs/audits/115_ADR0010_VOLUME_SPLIT_EXECUTION_AND_FRICTION_2026-09-28.md) / [`114 立项档`](docs/audits/114_ADR0010_VOLUME_SPLIT_REGISTRATION_2026-09-28.md) / [`0bm 报告 §1`](docs/audits/0BM_EDIT_FACE_IMPLEMENTATION_AND_FRICTION_2026-09-25.md)。


### P1-0bl 本轮全仓审查修复批（P1；2026-09-24 审查修复批立项，BACKLOG 0bl；来源＝本轮全仓审查发现＋0bi 承接；**十件当日全绿落码并同日闭合 2026-09-24：50 → 49**）

> 十件已落码并全绿收口（2026-09-24）；收口见审计档 `docs/audits/072_FULL_REVIEW_REMEDIATION_2026-09-24.md`。

- [x] **落码十件**：① `search_replace` 锚点行窗末空行修复＋钉子测试；② hashline 编辑面接入 BOM／emoji 单点；③ 编辑入口 UTF-16 fail-closed；④ emoji 告知行语义澄清；⑤ 压缩守卫 after 公式修正＋性质测试；⑥ 工具执行取消臂（落地为 select 专用取消臂＋`CancellationToken` 贯通，优于原设想的心跳臂检查）；⑦ 超时杀点定向化（per-call JobObject `TerminateJobObject`，句柄不可得回退全局杀）；⑧ 三处滞后注释更正；⑨ 两处 unwrap→expect；⑩ verify_content_anchor sha 口径与工具层一致。
- **待裁决/后续批（八件，登记不占计数；裁决后各自立项顺延编号）**：ADR-0010 §14 合并回正文＋分卷冻结机制／run_agent_loop 状态体拆分（巨型函数 ~3258 行）／RLI 影子通道转正·退役判据／spawn sink 全局单槽竞争／编辑面大文件上限阈值（需阈值裁决）／CRLF 归一化副作用声明／写前核证按工具名字符串特判的横切逻辑下沉为公共层／设计档模板增补「回滚路径」必填小节。
- 入口：审计档 `docs/audits/072_FULL_REVIEW_REMEDIATION_2026-09-24.md`（已闭合；BACKLOG 0bl 条目照录第二卷 §1.10）。



### P1-0au 检索尾部派发预算（P1；2026-09-20 立项，BACKLOG 0au；来源＝S3 摩擦 N2；**2026-10-02 用户裁决随 0cg 尾巴退役闭合 58 → 57，`withdrawn`〔148 批〕**）

> 入口：[`深挖档 §3`](docs/audits/0AR_S3_FRICTION_DEEP_DIVE_N1_N6_2026-09-20.md) / BACKLOG 0au。批序待定（建议 S1 落码 → S2 语料回放 → S3 随下一轮官方跑批收取）。

- [x] **S1 落码**（2026-09-20 过夜批）：`batch_close.rs` 三常数＋纯函数（`CLOSE_ROUND_MARGIN_SECS=60`／`RUN_TAIL_RESERVE_SECS=120`／`WALLCLOCK_RESERVED_CAUSE`；`wallclock_reserved` 两支任一即保留，无上限恒不保留）＋`agent_loop.rs` 预扫描余量判定（上限解析序＝env＞controller 测试 seam `run_wallclock_limit_secs`＞None；批墙钟＝首个可派发调用档位默认；保留批全位拒绝）＋保留拒绝臂（无 `ToolStarted`、`stamp_failure(Refused)`、post-batch cause 自述一次性重述、不喂 deny 断路器、无重派邀请）；钉子＝纯函数三件（S3/r1/r2 逐例回放＋边界方向）＋集成一件（seam 10 s ⇒ 无 ToolStarted＋cause＋重述）。
- [x] **S2 语料回放**（2026-09-20 过夜批）：四轮横向表 14 例回放——摩擦例 **7/7 全拦**（26–191 s trailing＋117 s 近失）、健康批 **7/7 不受扰**、边界 1 例保守改变（S3 extract 224 s；其「达标」系合成凑阈，与 0ax 修正同向）。判据 ①③ 达成、② 由集成钉断言。
- [x] **S3 真机复验**：~~随下一轮官方跑批收取~~ **未实机即退役**（2026-10-02，148 批）——三维度审查查明 0cg ②③ env 收口把本项唯一读源（env `ORZ_MAX_WALLCLOCK`）断供＝0.8.9 起死配置；用户裁决「F6/保留量不涉及其他部分的话，就进行记录，一起退役即可」；实现面（batch_close 判定族＋agent_loop 预扫描/保留臂/重述）与供流线随批清退（orz `e1c373ec`）；S1 机制曾随 0.8.8/0.8.9 载体在役（官方评测形态下为活机制）——历史如实记。见 [`148 批档`](docs/audits/148_0CG_TAIL_F6_RESERVE_RETIREMENT_2026-10-02.md)。


### P1-0ay 检索合成判定面落盘与可核复算（P1；2026-09-20 立项、**同日用户裁决闭合（未闭合 41 → 40）**，BACKLOG 0ay；来源＝0ax S1 独立审计 F-2；S1/S2/S3 落码与语料复算收口＋S4 同三题真机达成＋载体 0.6.4 重建发布）

> 入口：[`独立审计与裁决 §2`](docs/audits/0AT_0AU_0AV_0AW_0AX_S1_INDEPENDENT_AUDIT_AND_ADJUDICATION_2026-09-20.md) / [`S1/S2/S3 实施报告`](docs/audits/0AY_S1_S2_S3_IMPLEMENTATION_2026-09-20.md) / [`S4 真机复验`](docs/audits/0AY_S4_THREE_TASK_VERIFY_2026-09-20.md) / [`064 重建审计`](docs/audits/064_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-20.md) / 索引 `GAP-RETRIEVAL-SYNTHETIC-JUDGEMENT-AUDITABILITY` / BACKLOG 0ay。

- [x] **S1 契约面**（2026-09-20 完成）：`web_search_result` ledger 条目增可选 `citation_url_count`（**正整数**，原引用池非空才落——**勘误**：原写「非负整数／合成条目落 0」与 F-3 裁决正文及判据 ② 冲突，按裁决正文执行＝合成条目字段缺席）——schema＋fixture＋Python 镜像＋Rust verifier 四件同批；`merged-multi-query` fixture 重写（合成条目改查询串形态＋有引用池正例 3 = 1 + 2）。
- [x] **S2 实现**（2026-09-20 完成）：`evidence.rs` 装配面单源落原池条数（同一 helper `batch_close::citation_url_count`，与 `is_synthetic_answer` 判定同源）；字段条件落盘（不扰动单 query 批与无引用池批）。
- [x] **S3 语料复算**（2026-09-20 完成）：S3 三 run 七批归档重算逐格 **7/7 吻合**（旧 37／收窄 **16**／合成 21）；旧 journal 回放零新增报错（字段可选＋生成代际门）；混合批正例经真归档验证。驱动：`D:\tb-eval\_0ay_s3_recompute.py`。
- [x] **S4 真机复验**（2026-09-20 完成）：载体重建 0.6.4（orz `a47e9185`）＋作业 `official-verify-timeout3-s4`（45m44s、exit 0）——三批**无 URL 占比 12/12＝100 %、可引用来源 0、收窄可用 18**；3/3 批声明面（`query_summary[0].usable_source_count`＝批级、`synthetic_answer_count`）与 ledger-only 复算**逐值一致**、schema／判官 **0 错**。**边界**＝纯合成车道下无带池条目 ⇒ 0az ① 代际门与 S2 恒等式无可检样本；新观察＝gpt2 无 terminal（硬杀）、torch 收尾 1000.3 s。驱动：`D:\tb-eval\_0ay_s4_readings.py`。**闭合裁决待用户**（闭合则 41 → 40）。
- [x] **0ay 闭合裁决（2026-09-20 用户令「0ay 可闭合」）**：S4 读数与边界并入 [`S4 真机复验`](docs/audits/0AY_S4_THREE_TASK_VERIFY_2026-09-20.md) 后裁决接受 ⇒ 本项转 `implemented`、三方计数 **41 → 40**；同批载体 0.6.4 提交推送＋双平台包发布。余项（不阻塞）＝带池样本面随 `ORZ_WEB_SEARCH_LOCAL` 放行（0ax S2）收取。
- 判据：① journal 侧可独立重算（与实现同源同值）；② 单 query 批与无引用池批 payload 逐字节不变；③ 不动阈值 5/10、FP-2、官方口径（`task.toml`／镜像／verifier／数据集 pin）。
- 边界：与 0av（读数面）同族不同面——0ay 落判定**输入**；0ax S1 行为语义不变。


### P1-0az 检索合成判定面收口（P1；2026-09-20 立项、同日实施批闭合 42 → 41，BACKLOG 0az；来源＝0ay S1/S2 独立审查批 F-1…F-7；**七项缺口合成单总项排期**）

> 入口：[`独立审查`](docs/audits/0AY_S1_S2_INDEPENDENT_REVIEW_2026-09-20.md) / 索引 `GAP-RETRIEVAL-SYNTHETIC-JUDGEMENT-AUDIT-CLOSURE` / BACKLOG 0az / [`0ay 实施报告`](docs/audits/0AY_S1_S2_S3_IMPLEMENTATION_2026-09-20.md) / TODO P1-0ay。计数：立项 **41 → 42**（2026-09-20）；**同日实施批闭合 42 → 41**（报告 [`0az 实施与验证`](docs/audits/0AZ_SYNTHETIC_JUDGEMENT_AUDIT_CLOSURE_2026-09-20.md)）。

- [x] **① 判官口径收口（F-1＋F-2，优先）**（2026-09-20 实施批）：代际门改「**有任一条目带 `citation_url_count` 才校验**」＋判官改**类内去重**（与生产者同尺）＋schema 代际门描述同步；**并收口审查 §5 指出的声明面**——单 query 逐值对拍／多 query `unattributed_usable_count` 恒等式对拍，均在门内（避免拿 pre-0ax 宽口径声明撞窄口径复算）。Python 冻结镜像与 Rust 离线判官同形。
- [x] **② 契约面闭合（F-3／F-4／F-5）**（2026-09-20 实施批）：`allOf`（`citation_url_count` present ⇒ `source_type` const `web_search_result`）＋`prefilter_log` 进 `required`＋缩进回正；两条反例经 schema 直测各 1 错（此前 0 错）⇒ schema 接受集＝判官接受集。fixture 预期变化 3 件（minimal／v0.2 信封／tier-weight-mismatch 派生件）＋生成器同批，重跑 330/330 逐字节一致。
- [x] **③ 权威面同步（F-6）**（2026-09-20 实施批）：`usable_source_count` 描述对齐窄口径＋**ADR-0010 §14.74（v1.76）**转录（本条第 1 项修订 §14.73 第 2 条宽口径）＋0ar 设计稿 v1.2 §3.3 勘误注（零行为变更）。
- [x] **④ 钉子（含 F-7 覆盖缺口）**（2026-09-20 实施批）：Python 契约钉 +5（跨类同 digest／完全净化形态 raw 3→retained 0／0ax 时代混合批回放／声明可用面两条）＋schema 钉 +1＋Rust 对拍语料 +3（253 → 256，两侧逐族逐格对拍）。
- 判据：① 0ax 时代「同批有池＋无池」形态不再误报（构造例＋回放例）；② 同去重键跨类时判官重算与声明逐值相等；③ schema 接受集 ＝ 判官接受集；④ 阈值 5／10、FP-2、官方口径零改动；⑤ 既有读数不回归。
- 边界：只动判官与契约／文档面，**不动** 0ay S1/S2 生产语义（S2 语义等价重构保持）；S4 真机与 0ay 闭合裁决不受本项阻塞。


### P1-0bf RLI 生产化与域建模试验（含繁杂度阈值校准）（P1；2026-09-21 用户令立项，BACKLOG 0bf；来源＝[`0be 报告 §7`](docs/audits/0BE_RLI_ONLINE_ADAPTIVE_2026-09-21.md)＋用户 2026-09-21 裁决）

> 用户令：RLI 转正替换 LIF 按两步走执行（先常开承接时间轴／域定位，预测与繁杂度留参考面）；预测转域建模；繁杂度与水路疲劳加权合成总值＋持续性判据。**禁动作建议、禁外挂决策模型**——模型用 RLI 特征自主判断。

- [x] **S1 勘定**：① RLI 常开面（默认启用、影子退役）与 LIF 承接清单（时间轴／事件追溯／渲染面）② 域建模形态与边界（只给特征与域状态）③ 繁杂度与提醒参数（**无长度门**；两触发＝域迁移完成／持续性越线 k；与水路加权口径；失配概率随报）④ 采样补点（10 s 网格，空档才补）⑤ 开销读数面（侧车体积、压缩窗口取值、≤1 KiB）。（2026-09-22 闭合；入口：[`0bf 报告 §1/§2`](docs/audits/0BF_RLI_PRODUCTION_AND_DOMAIN_MODELING_2026-09-22.md)）
- **S1 预注册口径（2026-09-21 用户裁决，落定即生效）**：① **采样**＝事件级（决策轮＋每个工具事件）＋**空档按 10 s 网格补点**（不继续细分到亚事件层；0be 实测空档 >30 s 共 13 次／最长 183 s，重的正是长工具执行段）；② **模型面提醒只有两个触发器**——**域迁移完成**（等震荡结束后确认迁移，只报一次）与**持续性越线**（连续 k 次，用户令「持续越线次数增加」以补偿取消长度门，起始值建议 k=5 待 S1 标定）；**不加会话长度门**（用户令「模型需要获得数据」）；③ **预测失配只记录、不反馈**：不引入惩罚、不改变 RLI 内部状态与域判断（用户令「引入惩罚会让这个组件膨胀且难控制，还扰乱本身的域判断」），仅在发出提醒时**一并报出失配概率**，由模型自行裁决是否需要额外决策；④ **模型面与用户面分开**：用户面＝繁杂度与水路疲劳**加权合成总值**（每档一次、仅用户面、不注入模型；长任务语义由水位疲劳天然承担），模型面＝上两条触发器；⑤ 其余内部量由 RLI 自行消化、不外报。
- [x] **S2 落码**：四项实施＋钉子（常开开关与 LIF 退位、域特征输出面、阈值与加权、开销计数各留可核入口）。（2026-09-22 闭合：五文件落码＋新增钉子 6 项全绿；入口：[`0bf 报告 §3`](docs/audits/0BF_RLI_PRODUCTION_AND_DOMAIN_MODELING_2026-09-22.md)）
- [x] **S3 长会话真机轮**：默认开启下的长会话跑真机——同轮收 0bc 三项资源面读数与 0be 投递链／侧车覆盖；新摩擦随轮记录。（2026-09-22 以本会话观测轮＋离线探针回放（`rli-forecast-contrast-0bf.json`）**暂代**；**未新开真机长会话轮**——用户同日裁决转部分达成 ⇒ 本项**仍挂**；运行值缺口与 kill switch 缺省路径如实登记——[`0bf 报告 §4.4`](docs/audits/0BF_RLI_PRODUCTION_AND_DOMAIN_MODELING_2026-09-22.md)）（**2026-09-22 达成**：随 0bd 轮执行——run `RUN-CLI-6ab17325`，40m56s，exit 0；RLI 两触发真机首现、kill switch 缺省常开路径得证；0bc 三项资源面与 0be 投递链同轮收取，读数见 [`0bd 报告 §3`](docs/audits/0BD_FRICTION_MISC_2026-09-22.md)）
- **2026-09-22 用户令（长会话轮的任务源）**：「长会话真机狗粮轮依靠实际任务，用 0bd 作为任务」⇒ 本项 **S3 长会话轮与 0bd 真机狗粮轮同轮执行**（0bd 十四件即该轮实际任务；同轮并收 0bc 三项资源面读数与 0be 投递链／侧车覆盖）；前置＝0bf 码进载体（本批重建批 0.6.8）。
- [x] **S4 收口**：总结档＋判据对账；ADR 转录与计数同步随提交批。（2026-09-22 报告已落（含判据对账、摩擦 a–g 与 §9 裁决节）；**本项未真正收束**——S3 未完、ADR 转录与计数同步未做，故随用户裁决转部分达成后仍挂；本批**不提交／不推送／不重建**——用户令「不必进行提交/推送/重建，按照项目惯例落一份报告文档即可」）（**2026-09-22 闭合**：与 0be 合并转录 ADR-0010 §14.75／v1.77；余项移交 0bg）
- **2026-09-22 用户裁决**：「0bf 转为部分达成吧」⇒ S1／S2 勾选保留，**S3／S4 不勾**；未闭合计数维持 46（自轮内自记的 45 回调）；同批把**题面编码摩擦**并入 0bd ⑭（来源＝运行方观测：启动器 PS 5.1 按 ANSI 读无 BOM 题面 ⇒ 载体收乱码；0be 轮实测、0bf 轮以 BOM 验证）。
- **2026-09-22 闭合（与 0be 合并转录；用户令「0be/0bf的合并转录可进行」）**：S3＝**0bd 轮**
长会话真机达成（RLI 两触发真机首现／kill switch 缺省常开路径得证／`commit_notification` 首现／
0bc 三项资源面与 0be 投递链同轮收取）；S4＝ADR-0010 **§14.75／v1.77** 一条转录覆盖 0be／0bf
（不为中间过渡语义重复转录）⇒ **本项闭合（46 → 44 之一）**；余项（压缩窗口取值开销口径／
双迁移通知去留／权重与锚标定／侧车落盘核／读数面 1 KiB 截断）**移交 0bg**。

### P1-0be RLI 在线自适应与预测面收口（含繁杂度）（P1；2026-09-21 用户令立项，BACKLOG 0be；来源＝[`RLI 前推对拍`](docs/audits/RLI_FORECAST_CONTRAST_2026-09-21.md) §6／§7）

> 用户裁决：三点放行＋繁杂度并入本批。作用域＝**本会话**；口径＝**自适应 ≠ 优化器**（估计式更新，非损失下降）；ω／ζ 不动、禁拟合不变。

- [x] **S1 勘定**：λ̂ 定义与钳制（估计式，非损失式）／分通道 horizon 表与短视锚点形态／自适应轨迹侧车字段／繁杂度指标与分位数档位。（2026-09-21 随 S2 落码闭合；入口：[`0be 报告 §2`](docs/audits/0BE_RLI_ONLINE_ADAPTIVE_2026-09-21.md)）
- [x] **S2 落码**：四项实施＋钉子（λ̂ 上下界与退化、短视锚点、侧车轨迹、繁杂度档与用户面提醒各留可核入口）。（2026-09-21 闭合：六件落码＋钉子；orz **`0b7b89dd`**——`lif/rli.rs`／`lif/mod.rs`／`host/acp_server.rs`／`loop/controller.rs`／`loop/lib.rs`＋新增 `loop/complexity.rs`，入口：[`0be 报告 §3`](docs/audits/0BE_RLI_ONLINE_ADAPTIVE_2026-09-21.md)）
- [x] **S3 真机狗粮轮**：**逐会话**读数（消费率／转向相关／域一致性＋新读数面＋繁杂度触发），同轮记录新摩擦。（2026-09-21 达成：真机两轮＋离线重放；rli 面**消费率 0/3**；**覆盖缺口**＝繁杂度投递链与侧车真机覆盖、探针收编、冻结语料纪律、host 两条既存红——**已移交** 0bf S3 与 0bd ⑩⑪⑫⑬，非本项独有开放面；入口：[`0be 报告 §4`](docs/audits/0BE_RLI_ONLINE_ADAPTIVE_2026-09-21.md)）
- [x] **S4 收口**：总结档＋判据对账；ADR 转录与计数同步随提交批。（**2026-09-22 口径更新**：报告＋判据对账已落（[`0be 报告 §5/§7`](docs/audits/0BE_RLI_ONLINE_ADAPTIVE_2026-09-21.md)）；**唯一余项＝ADR-0010 转录**——0be 与 0bf 同属 RLI 主线，转录**合并为一条**随 **0bf S4** 落地（不为中间过渡语义重复转录），届时 0be 与 0bf 一并闭合）（**2026-09-22 闭合**：唯一余项＝ADR-0010 转录，已与 0bf 合并为一条（§14.75／v1.77）⇒ 本项闭合（46 → 44 之一）；覆盖缺口随该条移交 0bg）
- **2026-09-22 状态更新（用户令「更新 0be 情况」）**：本项台账由「四项全未勾」更正为 **S1–S3 达成、S4 待合批**——机制码已提交（orz `0b7b89dd`）并**已进在役载体**（0.6.7＝orz `6f23bbbf`；本批 0.6.8 重建批随源冻结 `f76e5e32`）；**未闭合计数维持 46**（0be 仍开放，余项仅 S4）。
- 入口：BACKLOG 0be / [`对拍件`](docs/audits/RLI_FORECAST_CONTRAST_2026-09-21.md)。


### P1-0as 编码 lossy 兜底细化（P1；2026-09-19 用户令「值得做」，BACKLOG 0as；由 0ar S1 狗粮轮 F4 回查引出；**实施批落码并于提交批闭合 2026-09-19（37 → 36；orz `3d7d7a74`＋`e897dce2`）**）

> 入口：BACKLOG 0as / 索引 `GAP-ENCODING-LOSSY-REFINEMENT` / F4 记录 [`0AR_S1_CONTRACT_SURFACE_2026-09-19` §7](docs/audits/0AR_S1_CONTRACT_SURFACE_2026-09-19.md) / 编码门审计 [`GAP_ENCODING_GATE_IMPL_AUDIT_2026-08-13`](docs/audits/GAP_ENCODING_GATE_IMPL_AUDIT_2026-08-13.md) / 实施回执 [`0AS_ENCODING_LOSSY_REFINEMENT_2026-09-19`](docs/audits/0AS_ENCODING_LOSSY_REFINEMENT_2026-09-19.md) / `orz/crates/codegen/orz-tools/src/util/encoding.rs`。计数：立项 **36 → 37**（2026-09-19）。目标口径：可读部分保持可读、不可解码字节显式可见且噪声最小（现状＝四级梯最后一级整段 `�`；run `RUN-CLI-6aad9497` 实测 `utf-8-lossy` 1 次）。

- [x] 口径定稿（2026-09-19 实施批）：分段粒度＝**行**（`split_inclusive`，UTF-8/GB18030 均不含 0x0A 跨行不可能）；择优＝**降级单元数最少、平手按梯序取 UTF-8**；占位形态＝`⟨0x8F⟩`／多字节 `⟨0xF0 0x9E 0x81⟩`（maximal-subpart 粒度与旧基线一一对应）；降级读数＝`utf-8-lossy:<p>%`（单元数÷BOM 剥离后输入字节×100，两位小数）。
- [x] 落码（2026-09-19 实施批）：`util/encoding.rs` 行粒度分段＋段级梯＋最小替换择优（**前缀三级判定顺序与命中语义不变**；写入侧不动；无解码链路径维持 `None`）；`decode_text` 签名 `(String,&'static str)`→`(String,String)`，六处 label 消费调用点适配；Cargo.lock 零改动。
- [x] 契约面评估（2026-09-19 实施批）：取值形状变更＝标签后缀化——两份 schema（v0.1/v0.2）`output_encoding` description 补注形态与比例定义（字段保持自由字符串、非闭枚举）；Python 冻结参照 `assurance/ops_executor.py::decode_text` 同算法移植，六案 Rust/Python 逐字节对齐；`encoding_lossy` 诊断签名 `contains("lossy")` 兼容零改动；fixtures 生成器重跑 346 件零差异。
- [x] 钉子（2026-09-19 实施批，六件）：① 混合样本（`lossy_mixed_sample_keeps_readable_parts_intact`——可读文本零损伤＋GB 段保持可读＋比例 2.94% 可复算）；② 纯编码零回归由既有四钉逐字节断言保持；③ 降级读数可核（`lossy_label_ratio_excludes_stripped_bom`＋journal 透传链零改动）；另 GB 胜出／平手梯序／多字节占位形态三钉＋既有两例 lossy 测试改钉。读数：orz-tools lib 2889+2（LSP e2e 两例负载敏感串行 18/0 绿，RS-08 同族）、orz-host 串行 **333/0/5**、orz-loop **805/0/3**、orz-hooks **192/0**、orz-bin **11+1/0**、runtime **366 OK**、fmt 触碰面全净、clippy 本批零新增。
- [x] F4 并线（2026-09-19 实施批落码）：`orz-bin/main.rs` 启动时（Windows）`SetConsoleOutputCP(65001)`——零依赖 FFI（沿库内 kernel32 先例，Cargo.lock 零改动）；宿主控制台与子进程随 CP 65001 走 UTF-8，TUI/ACP 车道不受扰；载体重建后生效；绕行纪律（关键内容 `read_file` 核证）保留。见[裁决文档 §4](docs/audits/RS_RESIDUAL_AND_FRICTION_F1_F8_RULING_2026-09-19.md)＋[实施回执 §5](docs/audits/0AS_ENCODING_LOSSY_REFINEMENT_2026-09-19.md)。
- [x] 审查处置（2026-09-19 主会话全面审查通过后随批）：P2×2＋P3×2 全部处置——① Python 冻结参照 `decode_text` 零测试覆盖 → 新增常驻钉 `assurance/tests/test_ops_executor_decode_text.py` **9/9 绿**（六案 golden＋严格级回归＋结构不变式＋表边界；CI 同款 discovery 可收集）；② GB18030 **全空间差分核证 1,611,796 例**（1 字节＋2 字节＋四字节全空间）→ 四类分叉双侧钉死：清洁度唯一分叉＝裸 0x80（唯一 label 分叉）、二字节 20 对＋四字节 1 槽位 PUA↔正式字符映射分叉、越界四字节替换粒度 1 vs 2 U+FFFD 可翻转择优；裁决＝不做语义对齐（需自研解码器），双侧钉死＋文档声明范围（`encoding.rs` module 头＋`ops_executor.py` docstring）；③ 实施回执 §9 勘误：读数漏记 6 ignored 实为自洽（审查发现更正）＋"gb18030 同粒度"限定六案范围。读数：orz-tools lib **2891 passed＋1 failed（LSP e2e 负载敏感，串行绿）＋6 ignored＝2898**（=2897＋1 新钉）、compileall 0。入口：[`处置回执`](docs/audits/0AS_REVIEW_HANDLING_2026-09-19.md)。
- [x] 自研解码器评估与裁决（2026-09-19，零代码零计数）：用户问询后落调查评估档——解码方向 1–2 人日、成本在表版本维护责任；encoding_rs 已与 GB18030-2022 一致（`A3A0` web-compat 例外＝差分实测值）、CPython 未跟进、Rust 无独立 crate；三路线评估后**用户裁决「那就先维持现状即可」**（不立项不动码），重开条件三条入档、若重开推荐 Python 侧 WHATWG 移植（约 200 行）。入口：[`评估档`](docs/audits/GB18030_DECODER_SELF_BUILD_ASSESSMENT_2026-09-19.md)。
- 边界：不改容器内环境、不动 `GAP-ENCODING-GATE` 既有四级梯语义；载体重建随用户放行。
- [x] 闭合入账（2026-09-19 提交批）：判据①②③随批落账闭合 37 → 36；orz `3d7d7a74`＋`e897dce2`，父仓 pin/账本/契约随批。
- 残留：已知梯序平手例（同行 GB 正文＋坏字节且单元数相等 → `⟨0xD6⟩`＋错配合法对，实施回执 §6）——沿注册候选「平手按梯序」执行，S3/狗粮读数可感时再提占优度加权裁决。

## P2 — 生产化决策门


> 来源：主 TODO「## P2 — 生产化决策门」闭合子节

### MECHANICAL-LAYER-MATH-CALCULUS（`implemented`；阶段 0-3 全部闭合 2026-08-31，BACKLOG P2-10）

- [x] 全部闭合：阶段 0 决策（D1-D7）/ 阶段 1 设计定稿（F1-F6 + ADR-0010 §14.47 转录）/ 阶段 2 实施切片（I1 T̂+LIF 计算器、I2 失败目标身份、I3 temporal 分区、I4 域 spike 侧车、I5 类型化信封、I6 pipe 归约）/ 阶段 2 全面审查处理（R1-R9 + F10-F14 全部收口）/ 阶段 3 验证（V1 FakeProvider 面 8 项 + F11 receipt↔事件链同构核对、V2 离线 102 runs 四对照门 + 聚类对照 + 零误干预、V3 S3 重建 + S4 实机冒烟 1/1 与 temporal 四查询面端到端一致）。入口：[正式设计](docs/MECHANICAL_LAYER_MATH_CALCULUS_DESIGN_2026-08-30.md) / [讨论稿](docs/MECHANICAL_LAYER_MATH_CALCULUS_DISCUSSION_2026-08-30.md) / [ADR-0010 §14.47](adr/ADR-0010-vol-14-addenda-index.md) / [阶段 3 验证记录](docs/audits/MECHANICAL_LAYER_MATH_CALCULUS_PHASE3_VERIFICATION_AUDIT_2026-08-31.md) / BACKLOG P2-10。


## 4. TER 工具执行层改革（TER）实施步骤（本卷原生内容，2026-09-03 建立）

> 原 TODO2 本体（`# TODO2 — 工具执行层改革（TER）实施步骤`）；主 TODO 路由行「⑧ TER 见 TODO2.md」指针仍指向本卷。正文与导语零改、标题降一级。

> 用途：TER（Tool Execution Layer Reform）专用分步实施勾选树；主
> [TODO.md](TODO.md) 只保留指针，本文为 TER 明细权威。
> 上级：docs/BACKLOG2.md（TER 开放项）/ 设计稿
> [TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md](docs/TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md)。
> 分步原则：每一步都是最小可验收单元（改代码 + 单测 + 验收），可随时
> 停；里程碑 M0–M3 各有一个放行门；步骤间按依赖排序，无依赖的步可单独
> 抽做但须过各自验收。
>
> **证据基线注记（2026-09-13，全项目深审 S-16 处置）**：M0–M2 已勾选项的
> 验收证据引用 0.3.x 载体时点，现载体已演进至 0.4.x（0t/0v/0x/0z 多轮
> bump），**历史勾选不自动等价当前行为**。M3（T3.1–T3.5）回归复验必须
> 以执行时点的当前 0.4.x 载体为基线重取证据（登记载体版本号），不得沿用
> 0.3.x 时点证据作为放行依据；M1/M2 若被后续批次触碰同一代码面，随批按
> 同纪律重验。

### M0 设计门（合约先行；只产出核对表与 schema，不写业务代码）

- [x] T0.1 默认值收敛核对表：BashParams struct default / 工具 schema
  默认 / 后端 15s（GROK env）三处来源逐项核对，产出“现状→目标”表
  （auto_background_on_timeout / foreground_block_budget_ms /
  hide_background_input / timeout 分层）。验收：核对表覆盖 §3.1 参数表
  全行，标注单一生效源方案。（2026-09-03 完成：核对表见
  [`TER_T0_1_DEFAULT_CONVERGENCE_CHECKLIST_2026-09-03.md`](docs/audits/TER_T0_1_DEFAULT_CONVERGENCE_CHECKLIST_2026-09-03.md)，
  覆盖 §3.1 全 6 行并给出单一生效源方案。）
- [x] T0.2 schema/verifier/fixtures 先行：新事件 `budget_cue_injected`；
  `tool_completed` 增 `output_truncated` / `total_bytes` /
  `output_object_id`（可选）；idle-kill 形态（status=idle_killed +
  reason）；黑板 `processes` / `env` live 分区渲染契约与越权边界。
  验收：orz-assurance 校验分支 + fixtures 先行，schema 版本号 bump。
  （2026-09-03 完成：契约与落地清单见
  [`TER_T0_2_SCHEMA_VERIFIER_FIXTURES_CONTRACT_2026-09-03.md`](docs/audits/TER_T0_2_SCHEMA_VERIFIER_FIXTURES_CONTRACT_2026-09-03.md)；
  run-event 枚举 53→54、tool_completed payload v0.1→v0.2、tool_running
  idle-kill 形态；校验/夹具/测试全绿 273 passed。）
- [x] T0.3 ADR 候选登记：TER 设计稿转 ADR-0010 §14.xx 候选项（含
  取代/衔接关系：S5-2 常驻化、PUSH→PULL 的 push 例外）。验收：ADR
  候选条目落盘，索引无冲突。
  （2026-09-03 完成：ADR-0010 §14.53 候选项 3 条落盘（TER 总登记 /
  S5-2 常驻化 / PUSH→PULL push 例外），CLI_PROJECT_INDEX v2.48 登记
  AUTH-TOOL-EXECUTION-REFORM（`pending`），设计稿状态行与 PUSH→PULL
  锚点同步；验收与 §0.5 检查见
  [`TER_T0_3_ADR_CANDIDATE_REGISTRATION_2026-09-03.md`](docs/audits/TER_T0_3_ADR_CANDIDATE_REGISTRATION_2026-09-03.md)。）
- [x] T0.4 放行签名：S0 核对表 + schema diff + ADR 候选齐备后签放行，
  进入 M1。验收：签名记录写回本文。
  （2026-09-03 完成：齐备条件逐项核过——T0.1 默认值收敛核对表 /
  T0.2 schema diff + 273 passed / T0.3 ADR-0010 §14.53 候选项 /
  生成器 v0.1 方向 A 落地（v0.1 树重建 0 差异）；用户确认放行，签名与
  M1 起点见
  [`TER_T0_4_M0_RELEASE_2026-09-03.md`](docs/audits/TER_T0_4_M0_RELEASE_2026-09-03.md)；
  “生成器 v0.2 表全面对齐”已单列专项
  [`BACKLOG2 TER-0.1`](docs/BACKLOG2.md)，不阻塞本门。M0 四步全闭，
  下一实施步为 T1.1。）

### M1 orz 主线（Linux 单测/构建闭环；每步都可独立停）

- [x] T1.1 S5-2 默认开启：`auto_background_on_timeout` struct 默认
  false→true；单测更新（含“关闭态仍可配”用例）。验收：默认构造即
  true，既有显式 false 用例不回归。
  （2026-09-03 完成：BashParams `Default` + serde 缺省均改
  `default_true`；orz-host 删除该字段与 `enabled_background` 的冗余
  显式注入（单一生效源，生效值不变）；新增 default_enables_auto_bg /
  explicit_false_disables_auto_bg，registry 与 bash 关闭态用例补显式
  false。验证：orz-tools bash 224 passed、grok_build::bash 156
  passed、orz-host 参数测试 1 passed、fmt --check 净、git diff
  --check exit 0。审计见
  [`TER_T1_1_S5_2_RESIDENT_DEFAULT_2026-09-03.md`](docs/audits/TER_T1_1_S5_2_RESIDENT_DEFAULT_2026-09-03.md)。）
- [x] T1.2 首报/后台化预算默认 180s：`foreground_block_budget_ms`
  生效默认 180_000（schema 与 struct 单一来源，消除 15s 后端默认）。
  验收：不传参数时前台命令 180s 触发 auto-bg；单测覆盖。
  （2026-09-03 完成：BashParams `Default` + serde 缺省 + 显式 `null`
  回退统一收敛 `DEFAULT_FOREGROUND_BLOCK_BUDGET_MS=180_000`；orz-host
  删除 300_000 显式注入（缺省经 serde 解析即 180_000）；终端后端兜底
  常量 15s→180s；schema timeout 描述与工具描述「中间回报点」改由生效
  预算渲染（不再硬编码 after 300s）。验证：orz-tools `--lib bash` 226
  passed、`foreground_block_budget` 18 passed（含 request 捕获用例与
  后端常量守卫）、orz-host 参数测试 1 passed、fmt --check 净、git
  diff --check exit 0。审计见
  [`TER_T1_2_FG_BUDGET_180S_DEFAULT_2026-09-03.md`](docs/audits/TER_T1_2_FG_BUDGET_180S_DEFAULT_2026-09-03.md)。）
  （2026-09-04 复审处理：F1 终端 actor 注释 “Default is 15s” 更正为
  180s 退役口径；F2 `effective_auto_bg_wait_ms` 改名
  `effective_fg_wait_ms` 并澄清语义 = FG wait deadline（仅当预算 <
  解析超时时才是 auto-bg 点，默认 120s<180s 时为 kill 点）；
  F3 `MAX_FOREGROUND_BLOCK` 注释更新为 “request 预算 + 后端兜底”
  口径。验证：`foreground_block_budget` 18 passed、`--lib bash`
  226 passed、fmt --check 净、git diff --check exit 0。见
  [`TER_T1_2_REVIEW_HANDLING_2026-09-04.md`](docs/audits/TER_T1_2_REVIEW_HANDLING_2026-09-04.md)。）
- [x] T1.3 模型面封闭：`hide_background_input` 默认 true；显式
  `&`/is_background 拒绝用例保持。验收：单测 + 工具 schema 无
  is_background 暴露。
  （2026-09-04 完成：BashParams `Default` + serde 缺省均改
  `default_true`（resident 封闭默认）；orz-host 删除
  `hide_background_input=true` 冗余显式注入（单一生效源，生效值不变）；
  显式 `false` 逃生阀保留（可见后台面 opt-in，registry 级默认封闭
  测试 `bash_definition_closed_by_default`）。同步：既有可见面用例
  夹具补显式 false；顺手校正 T1.2 遗留的 orz-workspace auto-bg 接线
  用例（短预算 500ms + timeout 300s 驱动，T1.2 预算语义下原 timeout
  300ms 属 kill-on-timeout）。验证：orz-tools `--lib bash` 229 passed
  （含新增 default_closes_is_background_surface /
  serde_omission_and_explicit_false_for_hide_background_input）、
  orz-host 参数测试 1 passed、orz-workspace 后台接线 7 passed、
  fmt --check 净、git diff --check exit 0。审计见
  [`TER_T1_3_MODEL_FACE_CLOSURE_2026-09-04.md`](docs/audits/TER_T1_3_MODEL_FACE_CLOSURE_2026-09-04.md)。）
- [x] T1.4 去硬杀语义：timeout 分层（普通/程序/模型上限）不再作为杀
  进程点，改为 auto-bg deadline 引用；原 timed_out 杀进程测试改为
  “auto-bg 或 idle-kill”断言。验收：无任何“满 timeout 杀活跃命令”
  代码路径（评测墙钟除外）。
  （2026-09-04 完成：前台解析超时与 180s 预算先到者即 auto-bg deadline
  （终端按 `min` 判定，bash 层取消“timeout ≤ 预算 → kill-on-timeout”
  逐调用门）；auto-bg/用户后台化后原解析超时退役，统一收敛 10h 绝对兜底
  （0b 清扫只撞绝对上限；显式后台任务正向模型超时 kill-backstop 保留）；
  kill-on-timeout 仅剩显式 `auto_background_on_timeout=false` 逃生阀；
  模型面文案删除 “Timeout enforcement … kills” 与 “300s ordinary /
  600s program” 静态分层宣示，改由 `min(默认超时, 预算)` 单源渲染。
  验证：orz-tools bash 230 passed、终端 actor 40 passed、orz-host 参数
  1 passed、orz-workspace 接线 1 passed、fmt --check 净、git diff
  --check exit 0。审计见
  [`TER_T1_4_NO_HARD_TIMEOUT_KILL_2026-09-04.md`](docs/audits/TER_T1_4_NO_HARD_TIMEOUT_KILL_2026-09-04.md)。）
- [x] T1.5 idle+CPU 兜底：进程监视采样器（输出字节增长 + CPU 时间）；
  连续 300s 无活跃 → kill + 提醒文本 + idle_killed 事件；阈值参数化。
  验收：模拟“无输出计算”与“真 idle”两类用例，不误杀前者。
  （2026-09-04 完成：`ActivitySampler` 每 tick 比输出字节、1s 节流读
  进程树 CPU（Windows Job 记账 / Linux /proc pgrp 汇总；其它平台 CPU
  未知不判 idle）；连续无两者增长满 `idle_kill_timeout`（默认 300s，
  actor/env `GROK_IDLE_KILL_TIMEOUT_MS` 参数化，0=禁用）→ SIGTERM+
  `signal=idle_killed`；完成提醒渲染 `idle-killed (no output growth or
  CPU activity for 300s)`（T0.2 同口径）；`KillReason` 增 idle_killed。
  验证：终端 actor 47 passed（含 5 采样器用例 + sleep idle-kill 实机 +
  无输出忙循环不误杀实机）、bash 231 passed、task_completion 49
  passed、fmt --check 净、git diff --check exit 0。审计见
  [`TER_T1_5_IDLE_CPU_BACKSTOP_2026-09-04.md`](docs/audits/TER_T1_5_IDLE_CPU_BACKSTOP_2026-09-04.md)。）
- [x] T1.6 黑板 processes live 分区：读取时现算快照（task_id/命令/
  elapsed/状态/活跃度/字节/可 kill）；状态跃迁落事件；kill 动作模型面
  可达且不暴露 `&`。验收：live 渲染单测 + 越权边界用例。
  （2026-09-04 完成：终端 `TaskLiveSnapshot` + `list_live_tasks`（读取
  时现算，status=running/idle/completed/killed）；LoopHost
  `terminal_live_processes`（fail-closed 默认空）；`section=processes`
  渲染（命令摘要 ≤80B、≤8KiB 预算截断、空态「（无）」）；live-only
  越权守卫（epoch/receipt_id 显式报错 exit 1）；tool schema/错误文案
  同步；kill 形态核对 = 进程行 pid + 既有 run_terminal_cmd PID 中断
  （不暴露 `&`/is_background）。验证：终端 48 passed、orz-loop processes
  6 passed（渲染 4 + 回达 + 越权）、orz-host 映射 1 passed、check 净、
  fmt/diff check 净。审计见
  [`TER_T1_6_PROCESSES_LIVE_PARTITION_2026-09-04.md`](docs/audits/TER_T1_6_PROCESSES_LIVE_PARTITION_2026-09-04.md)。）
- [x] T1.7 轮预算默认无限制：max_tool_rounds 默认移除（“120 per turn”
  静态文案动态化/移除）；保留可配上限逃生阀与 budget_insufficient。
  验收：默认运行无 120 拦截；显式配置上限时原机制仍生效。
  （2026-09-04 完成：`MAX_TOOL_ROUNDS` 120 → 0（0=unlimited 默认）；
  轮数闸加 `>0` 守卫（默认不挂 tool_rounds_limit/exhaustion）；
  session 面 budget=0 渲染 unlimited（显式上限仍按生效值渲染数字档与
  remaining）；`budget_insufficient` 预检仅显式非零上限时生效；检索
  子代理与主车道取 min 的组合在“主车道无上限”下正确落到检索档位。
  验证：orz-loop 全量 699 passed / 3 ignored、fmt --check 净、git diff
  --check exit 0。审计见
  [`TER_T1_7_UNLIMITED_ROUND_BUDGET_DEFAULT_2026-09-04.md`](docs/audits/TER_T1_7_UNLIMITED_ROUND_BUDGET_DEFAULT_2026-09-04.md)。）
- [x] T1.8 F6 pull 面：session（或 processes）补 wallclock
  （elapsed/limit/remaining）；blackboard_read 返回含时间轴。验收：
  渲染单测 + 越权边界。
  （2026-09-04 完成：session 面 `session_face_block_with_wallclock` 增
  `WALLCLOCK_ELAPSED`（LIF run-relative 只读换算）/ `LIMIT` +
  `REMAINING`（`ORZ_MAX_WALLCLOCK` >0 生效；未施加渲染 limit none，
  不虚构 remaining）；旧 wrapper 输出逐字节不变；epoch/receipt_id 越权
  守卫沿用。验证：orz-loop 全量 700 passed / 3 ignored、fmt --check 净、
  git diff --check exit 0。审计见
  [`TER_T1_8_F6_PULL_WALLCLOCK_2026-09-04.md`](docs/audits/TER_T1_8_F6_PULL_WALLCLOCK_2026-09-04.md)。）
- [x] T1.9 F6 push 档：阈值 <600/300/120s 注入中性事实 ≤4 次/run +
  `budget_cue_injected` 事件；开关 env/config 默认 off。验收：开关
  off 零注入（回归 PUSH→PULL）；on 时次数上限与事件可审。
  （2026-09-04 完成：`EventType` 增 BudgetCueInjected（v0.2 轨，
  snake_case 与 run-event schema 一致）；`ORZ_F6_PUSH` 默认 off +
  `ORZ_MAX_WALLCLOCK` 上限配置时，主车道每轮请求前按剩余 <600/300/120
  逐档注入一次中性事实（`[F6_BUDGET_CUE …]`，只报剩余/上限/已用轮、
  无建议，注册进 injected-block filter 不持久化）并记
  `budget_cue_injected`（remaining_seconds/rounds_used/threshold_seconds）；
  每 run ≤3 次（T0.2 verifier ≤4 兼容）；off 零注入/零事件/零文本。
  验证：orz-loop 全量 703 passed / 3 ignored、orz-assurance 201
  passed、fmt --check 净、git diff --check exit 0。审计见
  [`TER_T1_9_F6_PUSH_BUDGET_CUE_2026-09-04.md`](docs/audits/TER_T1_9_F6_PUSH_BUDGET_CUE_2026-09-04.md)。）
- [x] T1.10 W-F13a read_file 64KB 档：限制链核对（粗门 clamp/行 limit/
  50K 注入预算）后放宽至 64KB 级。验收：vm.js 级文件 ≤2 次读完；
  单次注入不触发截断；大文件仍可结构化分段。
  （2026-09-04 完成：coarse gate 默认/上限 16/32K → **64K**（下限 8K
  逃生阀保留）；限制链核对：64K ASCII ≈ ≤16K token < 25K 读档/50K 单轮
  预算、行档 1000 行语义保留；>64K 仍回有界信封（preview ≤4K + offset
  结构化分段）；orz-host 配置口子 clamp 同步 8–64K；full/concise/
  handle 文档与旧 16K 测试档位同步。验证：read_file 桶 208 passed、
  orz-host clamp 1 passed、fmt --check 净、git diff --check exit 0。
  审计见
  [`TER_T1_10_W_F13A_READ_FILE_64K_2026-09-04.md`](docs/audits/TER_T1_10_W_F13A_READ_FILE_64K_2026-09-04.md)。）
- [x] T1.11 W-F13b 输出检索对象：长输出落盘对象 + pattern/行区间/尾部
  N 行检索；ToolCompleted 截断标记 + 对象指针（schema 已 T0.2 定稿）。
  验收：检索语义单测；模型无需 .gsa 即可补读自身输出。
  （2026-09-04 完成：orz-tools `computer::output_object` 公共检索 API
  （pattern 大小写不敏感/上限、1-based 行区间闭区间+越界 clamp、尾部
  N 行，统一固定解码链）；对象 id = 落盘 log 路径（read_file/grep 直接
  消费，无需 .gsa 摸黑）；host 映射截断输出为 TerminalOutputObject，
  ToolCompleted（v0.2 轨）落 output_truncated/total_bytes/output_object_id
  （配对规则照 T0.2）；console 重建路径透传。验证：output_object 4
  passed、journal 三字段 1 passed、host 实机 30K 截断映射 1 passed、
  orz-loop 全量 704 passed / 3 ignored、fmt/diff check 净。审计见
  [`TER_T1_11_W_F13B_OUTPUT_RETRIEVAL_OBJECT_2026-09-04.md`](docs/audits/TER_T1_11_W_F13B_OUTPUT_RETRIEVAL_OBJECT_2026-09-04.md)。）
- [x] T1.12 W-F11 环境快照：probe 扩展至代码工具环境（工具/语言/包/
  版本/连通判定，Linux 先本地快速判定）；落黑板 `section=env`（PULL）。
  验收：快照 ≤5s；env 分区 PULL 渲染 + 越权边界。
  （2026-09-04 完成：orz-host `env_snapshot`（固定注册表 PATH 存在性 +
  `--version` 并发探测单项 1s 超时、输入在场布尔、无 allowlist/任务结论
  ——实测 ≈1.6s < 5s）；`LoopHost::env_snapshot_facts`；`section=env`
  渲染（kind 白名单登记 tool/language/package/input/connectivity，越权
  kind 渲染层拒绝、8KiB 预算、空态（无））；epoch/receipt_id live-only
  显式报错；连通性行由 W-F12（M2 T2.2）闭环后接入（本步不伪造）。
  验证：orz-loop 全量 709 passed / 3 ignored、env 相关 13 passed、
  orz-host env_snapshot 2 passed（≤5s）、fmt/diff check 净。审计见
  [`TER_T1_12_W_F11_ENV_SNAPSHOT_2026-09-04.md`](docs/audits/TER_T1_12_W_F11_ENV_SNAPSHOT_2026-09-04.md)。）
- [x] T1.13 M1 验收：orz-loop/orz-assurance 全量测试绿 + clippy/fmt 净
  + Linux 三件套构建成功 + 事件链 verifier 过新 schema。放行进入 M2。
  （2026-09-04 完成：orz-loop --lib 709 passed / 3 ignored、orz-assurance
  201 passed；clippy 四 crate 零 error（存量告警登记）、fmt 净；Linux
  release 构建成功（WSL Ubuntu 24.04 x86_64，rustc 1.98.1：
  orz-tools/orz-loop/orz-host + orz-bin CLI 闭包 Finished release，
  Linux 编译修复 tty-utils 借用 + orz-tui EventType arm 并 Windows
  复检绿）；事件链 verifier 273 passed（T0.2 schema/fixtures）+
  Rust 新载荷单测。**M1 放行进入 M2**。审计见
  [`TER_T1_13_M1_GATE_2026-09-04.md`](docs/audits/TER_T1_13_M1_GATE_2026-09-04.md)。
  2026-09-04 全面审查：门审计 §1.3 “tool_running idle_killed 由单测锁定”
  属过度声明（当时只有 schema/fixtures/verifier）；生产者已由审查处理批
  补入（orz `35db6741`，见
  [`TER_REVIEW_HANDLING_2026-09-04.md`](docs/audits/TER_REVIEW_HANDLING_2026-09-04.md)
  与 ADR-0010 §14.55 条目 5）。）

### M2 Windows runner / VM（依赖 M1 新 build）

- [x] T2.1 墙钟单一化：删 runner `--max-wallclock` 与 60s 余量；
  sandbox `--timeout`=官方 agent_timeout_seconds 为唯一评测墙钟；F6
  push 读源以 runner 施加值为准。验收：DryRun 确认无 840 参数、逐题
  timeout 生效。
  （2026-09-04 完成：`run_agent_arm.ps1` 删除 `perTask-60` 派生与
  `--max-wallclock`（840 余量移除）；sandbox `--timeout` = task.json
  官方 agent_timeout_seconds 为唯一评测墙钟；官方值经
  `ORZ_MAX_WALLCLOCK` env 透传 orz（F6 pull/push 读源以 runner 施加值
  为准，T1.8/T1.9 消费）；DryRun 新增 wallclock 计划行。验证：DryRun
  两题 900s/3600s 计划输出无 --max-wallclock/840、逐题 --timeout 与
  env 生效、AGENT_ERRORS=0。审计见
  [`TER_T2_1_WALLCLOCK_SINGLE_SOURCE_2026-09-04.md`](docs/audits/TER_T2_1_WALLCLOCK_SINGLE_SOURCE_2026-09-04.md)。）
- [x] T2.2 W-F12 本地透明层：先测基准（当前墙外 Test-NetConnection /
  curl / Invoke-WebRequest / python requests 各自失败耗时表）→ 实现
  DNS/TCP 本地拒答 → 复测 ≤2s/目标；allowlist 内连通不变。验收：mteb
  HF 探测 ≤2s、连通扫描总成本 ≤10s。
  （2026-09-04 完成：根因定案——AppContainer（空能力）+ allowlist 不兼容
  （allow 规则被 AC compartment 吞，allowlist_reachable 必 FAIL），生产墙
  为 `--no-appcontainer` + allowlist（2026-09-03 裁决）；enforcement 探针
  增 `-ExpectAppcontainer` 与 WF12 stdout 时延行、runner 增
  `-NoAppcontainer`（no-AC 原位执行探针，修复 Errno 13 读文件）、sandbox
  工作区 grant 加 `/T` + 子进程 stderr/out spill、新增受控 op
  `vm-wf12-probe`（AC 基线 + no-AC 生产墙 + egress 前/复测 + DNS 拒答可逆
  部署）。验证（win-s4 实机）：no-AC 墙 allowlist_reachable PASS
  （15–47ms，连通不变）+ network/metadata_blocked 0–16ms；egress 复测
  （DNS 拒答 127.0.0.1:53，NXDOMAIN 7ms，测后 DNS 恢复）总成本 920ms
  （≤10s）、最差行 735ms（≤2s）；AC 基线 allowlist FAIL 作对照登记。
  审计见
  [`TER_T2_2_WF12_LOCAL_TRANSPARENT_LAYER_2026-09-04.md`](docs/audits/TER_T2_2_WF12_LOCAL_TRANSPARENT_LAYER_2026-09-04.md)。）
- [x] T2.3 Windows 后台任务存活验证（2026-09-07 完成，S3/S4 集中实机
  验证批 T2/批次 W1）：fake 场景三判据 ALL_PASS——① auto-bg 中报事件
  @180s + 跨调用存活（R2 采样 ≥95 tick 且增长至收尾）；② bg 交接 + 任务
  在子进程存活期间自然跑满；③ `tool_running(status=idle_killed)` journal
  事件。零 API（`ORZ_FAKE_SCENARIO` 驱动、无 `--real`）。产物
  `_windows_high_nist/job-w1-fake-batch.ps1` + `evidence-w1-20260907/`
  （3 journal + bg 心跳日志 + sandbox observations + 断言 JSON）。
  （边界：完成提醒文本仅模型会话面注入、不落持久面，fake harness 下
  不可直接断言——提醒可达性判据归 W2 真实模型面 §7 C 组。）
- [x] T2.4 同步与接线（2026-09-07 完成，S3/S4 集中实机验证批 T0c）：
  M1 新 Windows 三件套 0.3.1 进 VM（copy_to_vm 15/15 哈希核验 + Program
  Files 换装备份 + C:\s4\tools）+ keystore/signer 复检（0.3.1 provision
  重刷 C:\workspace\acaf manifest，signer sha256 自校验通过）+ runner 配置
  收敛（不再补开 S5-2）。验收过：vm-agent DryRun 全对（AGENT_RUN_OK=True /
  ERRORS=0）+ enforcement-probe 三臂墙内全绿（control exit 0 / non-admin
  9/9 / high-nist 19/19，SYSTEM 提权作业通道）。证据
  `_windows_high_nist/evidence-t0-restore-20260907/`。

### 审查处理（2026-09-04）

- [x] TER M1/M2 全面审查处理：P1-1 idle-kill journal 生产者补入＋单测
  （orz `35db6741`）；P1-2 ADR-0010 §14.55 正式裁决（转正/取代清单）；
  P2 × 5 与 P3 ~24 逐项处理/登记（含 no-AC 生产墙、10h 例外、push 次数
  口径、completed elapsed 冻结、live-capable 标注、命中行 4K 钳制、DNS
  fail-fast、fake loader fail-closed、文案残留清理）。验证：orz-tools
  output_object 5 / idle 2 / description 97、orz-loop idle-kill 1、
  orz-host mapping 1、orz-bin loader 1 + cargo check 全绿、dns_refusal
  selftest/fail-fast OK。审计：
  [`TER_M1_M2_COMPREHENSIVE_REVIEW_2026-09-04.md`](docs/audits/TER_M1_M2_COMPREHENSIVE_REVIEW_2026-09-04.md)
  + [`TER_REVIEW_HANDLING_2026-09-04.md`](docs/audits/TER_REVIEW_HANDLING_2026-09-04.md)。

### M3 回归复验

- [ ] T3.1 gcode 复跑：300s 渲染 180s 收中间状态并后台化；无输出则
  300s 处 idle-kill+提醒；事件链含 tool_running。验收：journal 证据。
- [ ] T3.2 make-doom 复跑：无 840 硬杀；连通探测 ≤2s/目标收敛；进程区
  可见运行中探测并可 kill。验收：journal + 观察记录。
- [ ] T3.3 mteb 复跑 + F6 push 档：HF 探测 ≤2s 判定失败；push ≤4
  次/run + `budget_cue_injected` 可审。验收：journal 证据。
- [ ] T3.4 W-F13 复验：vm.js 阅读轮数 20+→≤2；长输出补读不再触 .gsa。
  验收：轮数统计对比。
- [ ] T3.5 §7 判据 1–6 复验登记 + ADR/BACKLOG2/TODO2 收口。验收：
  S4_PROGRESS 登记 + 勾选树闭合 + 设计稿状态更新。

### 首轮明确不做（防膨胀）

- 15min 长档（视 180s 实跑数据再定）。
- 待审圈系统审计（web/检索/浏览器/console 600s/其它默认面）。
- fold 后任务状态摘要（折叠重建，单列候选）。
