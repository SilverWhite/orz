# ORZ 待办项记录（TODO）

> 状态：living（实施勾选清单）；建立：2026-08-14。
> 定位：面向后续实施与改动的操作型清单，派生自 [`docs/BACKLOG_AND_PRIORITIES.md`](docs/BACKLOG_AND_PRIORITIES.md) 的未闭合项；优先级、决策门与状态权威仍是 BACKLOG，设计权威是 ADR-0010 / ADR-0011，召回路由是 [`CLI_PROJECT_INDEX.md`](CLI_PROJECT_INDEX.md)。本文件只登记指针与勾选状态，不新增设计裁决，不重复维护设计细节。
> 维护纪律：完成一项勾选一项，并同步 BACKLOG 与索引状态；新增或调整优先级只改 BACKLOG；删除或归档条目前先回查索引入口；每次改动保持本文件与 BACKLOG 的入口一致性。
> 入口：索引 AUTH-TODO（2026-08-14 登记）。
> 分卷口径（2026-10-02 用户令「todo2改一下形态，直接变成第二部分，给todo腾地方用，就和index的第二部分一样」）：[TODO2.md](TODO2.md) 自本批起为**本清单第二部分**——承接已闭合项核对节、近期已闭合单行核对与历史整理轮段（形态对齐 [BACKLOG 第二卷](docs/BACKLOG_AND_PRIORITIES_2.md)）；主 TODO 只留**开放项勾选节＋开放项路由＋本批要点**，闭合随批先记本文件条目、沉淀后移入第二部分。
> 历史整理轮记录（2026-08-31—2026-09-19）原样移入 [TODO2.md](TODO2.md) §1；近期已闭合单行核对移入其 §2。

## 使用说明

- `[ ]` = 待办；`[x]` = 已完成（保留单行供核对，不计入开放项）。
- 每项标注 canonical ID / 优先级 / 关键内容 / 入口；同一概念只出现一次，不复制 BACKLOG 的决策记录。
- 已闭合项单行核对格式：`[x] <ID>：<一句话>（闭合日期；入口：<文档/审计>）`。

## 开放项路由（2026-09-13 同步；勾选与计数权威在 BACKLOG）

> **分卷口径（2026-09-25 用户令）**：逐批流水与记录文档自**下一批**起**直接**记入 [BACKLOG 第二卷](docs/BACKLOG_AND_PRIORITIES_2.md)（取代先前「先落治理行、沉淀后再移入第二卷」）；本文件与主 BACKLOG 只留「当前计数＋本批一句要点＋指针链接」。

- 未闭合总数：**59 项**（BACKLOG 计数口径；**本批＝171 批 recli 二跑真机核证＝0cp S4 达成＋0cn S4 达成（计数不变 59；闭合待用户裁决 59 → 57）**——0.8.11 载体；8/8 checkpoint 单会话全程 ≈53min exit 0；0cp §5 七判据逐条核收〔动作样 320 全源 ✓／看门狗真机未触发按测试钉口径／k=3 两 fire 均 ×3 ✓／直投 37 fire＝37 journal 行同刻＋delivered_total=37 恰等 ✓／零动作周期零注入 ✓／D6 行宽 186–228B≤240 ✓／D7 spike 13/13 全提醒 ✓〕；0cn `预算`/kind=budget 首跑 136→0 ✓；同题同源（ckpt prompt 逐字节等）；[`171 批档`](docs/audits/171_0CP_S4_0CN_S4_RECLI_SECOND_RUN_2026-10-03.md)。前批＝**170 批 0.8.11 双平台载体重建进体＝0cp S3＋0cn S3 达成**（源冻结 orz `6493fdae`；已提交推送）；[`170 批档`](docs/audits/170_CARRIER_REBUILD_V0811_0CP_S3_0CN_S3_2026-10-03.md)。前批＝169 批 0cp S2 审查处置／168 批 0cp S2 落码／167 批 D7 补订／166 批 D6 补订／165 批 recli 二遍分析／164 批 0cn S2 落码。）TODO `[ ]` 明细含父/子项，计数以 BACKLOG 为准。
- P0：0d 后续 3/4/5 S4 复验；0j W1-R1 S4 复验 + W3-R3 余项 + W4-R4 S5-2 总项；0l ⑥⑦（⑧ TER 见 [`TODO2.md`](TODO2.md)）；0n GAP-APPROVAL-PROMPTER（延期，S1 设计定稿前置）；0o T3 批次 W2 chunk3 + T4 批次 L + T5 批次 O + T6 统一收口；0t S4 实机复验待续；0u R4 15 题官方复跑（0t S4 载体，2026-09-10 放行）；0w TB 4.0 单题摩擦探针（第三跑成立并跑完（2026-09-11））；0y NP1 机械身体集成支线（2026-09-11 用户裁决立项，实施未开始；§14.2 验证载体已裁决引入模拟器（2026-09-12），S1 载体搭建待放行，M0 定版为首个无依赖里程碑）；0z 真机资源安全边界与崩溃收尾（2026-09-12 用户裁决立项，设计完成待放行实施；S1–S4，**不与 0v-C 搭车**）。`GAP-ORZ-HOST-RESOURCE-SNAPSHOT-DROP` 已随 0.5.1 载体重建闭合（2026-09-14）。**即时结果回报与流式检索线（GAP-MECH-IMMEDIATE-FEEDBACK）已于 2026-09-27 S4 三题实机复验通过后闭合**（55 → 54；判据①②③全过＋FR-D02 达成；整轮 89 题重跑按用户修正令不发起；原验证⑤登记已随 152 批退役废止）：[`098 S4 收口档`](docs/audits/098_0AC_S4_CLOSURE_2026-09-27.md)。**2026-09-20 闭合**：检索批次回送与轮级单席位线（S1 契约面／S2 实施／S3 同三题真机复验全部完成，ADR-0010 §14.73/v1.75 转录随 S2；未闭合 36 → 35）＋同日 N1–N6 摩擦深挖登记观察不立项。
- P1：0bz 上下文脸面瞬态分叉（S1 已落码；S2–S4 待续）；0by ACAF 签名器不可达（S1＋S2 已修，S3/S4 待续）；0cb 写控保底化（S1–S3；S4 读数已随 123 批达成，闭合待裁决）；组件审计；GAP-WINDOWS-EVIDENCE 三项；IMPL-DEEPSEEK-TRANSPORT live 晋级证据；ORZ-SESSION-CONTEXT-MONITOR 四项；0aa 历史卷 journal 全量 verifier 复扫；0aj 黑板写权限层放行（`blackboard_write` ReadOnly deny，旧摩擦残余）；0al 门禁冻结克隆树漂移；0ae 上下文软门与模型参与压缩；0ah 滑块上下文（v8；实现更正批＋只读审查处置＋收口清理批已落码，五项真机读数待实测，登记待放行）；0am LIF 动力学升级线（挂起，O2 为前置门）；0aq 严格审查处置线（RS-01…RS-19 待修；明细第二卷 §2.2）；0ba 运行身份唯一性（run id 秒级撞名）；0bc 资源层收口与可失败分配（随轮观测 RLI）；0bd／0bg／0bh／0bi／0bj／0bk 摩擦大杂项轮（明细第二卷）；**0bm 狗粮长轮**（首次真机轮达成 09-25：编辑面簇四件＋0bj②⑤⑥ 落码 `b7dd241e`；同轮并入 0bj／0bp）；**0bn 必定压缩复审补口**（单独进行、不进轮；S2 已随 0.6.12 进载体）；**0bp 检索形态改指浏览器车道**（B 路径；执行面自 09-25 起随 0bs 同载）；**0br Web 形态 UI**（Web 先行、TUI 形式后补；ADR §14.78）；**0bs／0bt 大狗粮轮**（0bt 全轮并件入 0bs；七件＋0bt 四件＋⑧-⑯ 已随 **0.7.0** 提交推送与发行；S3 载体达成、S4 真机待跑）；S3 摩擦处置 0aw／0ax；**0ci Frontier-Bench 试点线**（2026-10-02 立项）；**0cl** 黑板瘦身；**0cn** budget 处置（**S1 定稿＝撤除**〔161 批补记〕；**S2 落码达成〔164 批〕**；**S3 达成〔170 批〕**；**S4 达成〔171 批〕**、闭合待裁决）；**0cp** RLI 动作采样与附注直投（163 批立项；S1 v1.2 达成；**S2 落码达成〔168 批〕**、**169 批审查处置修正**〔看门狗时间基转轴／D7 回归勘误〕、**S3 达成〔170 批〕**、**S4 达成〔171 批〕**、闭合待裁决）。已闭合流水沉淀第二卷（RLI 线三件收口〔163/161 批〕、灾难兜底精准化〔151 批〕、英文翻译线〔160 批〕等）。
- P2：IMPL-CONTROL-FABRIC Slice 3/4（P2-7）；OPS-PROTOCOL 裁剪与接线裁决（P2-8）；P2-11 余项（PULL 自描述 S3/S4、retryable 分类位、依赖图 S3/S4）；P2-12 S4 复验；P2-13 B4；P2-14 S3/S4；P2-15 EVALUATION-CORPUS-FREEZE（S1/S2 完成；附属 `GAP-EVAL-RESULT-SCHEMA-DRIFT` 已修复）；0ap FR-A06 压缩交互设计批（首步设计评估稿，取舍随稿裁决）；0bb 命令行引号与长行取用摩擦（入口见 BACKLOG）；S3 摩擦处置 0at（逐 query 归因）／0av（批次数读数落盘面）；
- P3：EVIDENCE-LOCAL-BROWSER、GATE-CHAIN、observed-scope 枚举、V11-IMPL-003、V11-IMPL-007、orz-host flaky。
- 审计登记边界（条件触发，不占当前优先级）：orz-host 可选后端、headless 计划信号、23 工具分区 journals、B-1 后续、ORZ-RECOVERY-TOOL-OUTCOME、ORZ-STAGNATION-TOOL-SIGNAL。
- 已闭合分组（单行核对与闭合核对节见 [TODO2.md](TODO2.md) 第二部分）：P0-GOV 00/00a、P0-E、0c、0d 主项与后续 1/2/6/7/8、0e、0f、0g、0h、0i、0k、0p、P0-B、P0-C、P0-C2、P0-D、P1-0q、P1 已闭合项、P2-10 全部闭合、0ak（2026-09-18）。

## P0 — 当前工作集
### P0-F FUS-BENCHMARK-FULL-EXEC（P0，2026-08-18 用户裁决实施；**2026-10-02 用户裁决退役闭合 58 → 57〔152 批〕＝「0b 本身就不做」；`pending` → `withdrawn`**）

> **退役闭合（2026-10-02，152 批第二轮）**＝验证③④⑤随裁决退役不再执行；机制面（Benchmark 轴＋探针＋console 注册＋CLI 旗标）在役并经 TB 2.1 全轮实战＝验证线退役、机制面不退役。入口：[`152 批档`](docs/audits/152_FRONTIER_BENCH_SLOPCODEBENCH_ADJUDICATION_2026-10-02.md) §6/§9。

> 入口：[设计](docs/BENCHMARK_FULL_EXEC_DESIGN_2026-08-18.md)；ADR-0010 §14.24；BACKLOG 0b。orz 子模块 3f43478 / 4e9e61b。

- [x] 实施完成（2026-08-18）：权限层 Benchmark 两轴参数化 + 探针 BenchmarkFull + console `workspace.run_terminal` 注册 + CLI `--allow-shell`/`--allow-network` + `tb_agents/orz.py` 透传 + 审查收口；验证① 全量测试通过（含修复既有测试漂移 c4772fc）；折叠 400 根因修复 S1-S5 闭合（处理文档 LEDGER_FOLD_MARKER_INDEX_FIX_HANDLING）。
- [x] 验证②：Linux musl 重建（ORZ-BUILD-MOUNT-001 契约，输出 `D:/tb-eval/orz-linux`）——2026-09-07 随 0o T0 闭合（暂缓由 0o 集中批放行解除；产物后经 0t S3 / 0.4.0 轮翻新）。（2026-09-10 补勾；入口：BACKLOG 0b/0o）
- [x] 验证③（2026-08-18 修复后复验一次，机制断言全过、reward 项未达标，待加预算重跑）：单题 make-doom-for-mips——0 异常、无 400、6 笔订单→5 组票据零拒绝、压缩后会话继续；reward 0=墙钟内未产出可运行 ELF（非机制回归）。
- [x] 验证④（用户指示暂缓）：2–3 题交叉（compile-compcert、hf-model-inference 等 build/run 与网络类）。
- [x] 验证⑤（用户指示暂缓）：`run_official_2.1.sh` 89 题 5 批。
- [x] 闭合：验证全过 → BACKLOG/TODO/索引状态同步，未闭合 27 → 26。


### P0-0d 后续 3：ZERO-CHUNK-RETRY-WINDOW-180S（P0 派生；2026-08-21 定稿）

> 入口：[设计修订](docs/STREAM_RETRY_RHYTHM_DESIGN_2026-08-20.md) / ADR-0010 §14.36 / BACKLOG 0d。
> 定案：`request_retry_window` 50s → **180s**（`request_max_retries` 10 不变，双上限先到者止）；非流式 create 退避窗口同步放宽；retry 参数参与请求头指纹。

- [x] S1/S2 实施闭合（2026-08-21）：窗口/退避/指纹落地 + 测试。
- [x] S3 重建——**2026-08-25 核证闭合**：随 0g/0h/0.1.0 发布构建轮合入 Linux musl 三件套（a96faab，merge-base 祖先核证）。
- [ ] S4 复验（无 400、命中率 ≥90%、断连窗口内可骑过节点抖动）。


### P0-0d 后续 4：STALL-DEGENERATION-FAILFAST（P0 派生；2026-08-21 定稿；S1/S2 闭合 2026-08-21、S3 闭合 2026-08-25、S4 待续）

> 入口：[设计](docs/STALL_DEGENERATION_FAILFAST_DESIGN_2026-08-21.md) / ADR-0010 §14.37 第 1 项 / BACKLOG 0d。

- [x] S0 证据门通过 + S1/S2 实施 + 全面审查处理闭合（2026-08-21）：会话级 thinking 档位（哨兵后不回 high）+ 哨兵计数单调（成功不清零、达 3 run_invalidated）+ disabled 档即终止 + `ModelGateway::for_new_run()` 每 run 隔离 + v0.2 `transport_retry` 事件面；orz-loop 544 / conformance 230 通过。
- [x] S3 重建——**2026-08-25 核证闭合**：与窗口 180s + 解码兜底批次合并（a0d85f8，随 0g/0h/0.1.0 构建轮覆盖）。
- [ ] S4 复验（单 run 哨兵预算有界 ≤3 次触发 × 单次预算、显式终止可观测、命中率 ≥90%、零 400）。


### P0-0d 后续 5：MIDSTREAM-DECODE-RETRY（P0 派生；2026-08-21 定稿；S1/S2 闭合 2026-08-21、S3 闭合 2026-08-25、S4 待续）

> 入口：[设计](docs/MIDSTREAM_DECODE_RETRY_DESIGN_2026-08-21.md) / ADR-0010 §14.37 第 2 项 / ADR-0007 修订注记 / BACKLOG 0d。

- [x] S1/S2 实施闭合（2026-08-21）：重试判定「无完整 tool_calls」（`wrap_no_tool_side_effects` + `has_complete_tool_call` 双保险）+ 中段有界 1 次（`CHUNKED_MIDSTREAM_MAX_RETRIES`）+ `StreamInterrupted.saw_chunk` + `transport_retry` 事件面（run-event enum 53→54、schema/fixtures/conformance/TUI 同步）。
- [x] S3 重建——**2026-08-25 核证闭合**：与窗口 180s + fail-fast 批次合并（同后续 3 S3 注）。
- [ ] S4 复验（dna 类场景不再因解码错误杀 run、零 400、命中率 ≥90%）。


### P0-0j THIN-HARNESS-REDESIGN-V2（P0；2026-08-28 设计定稿，实施进行中）

> 入口：[设计](docs/THIN_HARNESS_REDESIGN_V2_DESIGN_2026-08-28.md) / [HA 调研](docs/HA_SERVICE_MODEL_RESEARCH_2026-08-28.md)；BACKLOG 0j。
> 定案摘要：复读门槛统一 20 + 序列门全删 + 3-gram 15 + 802 保留 + 空响应链 low 封顶 + 触发显式拦截不降档 + 审计结构化字段 + 半助理层加厚（诊断/实体/黑板）+ HA 服务模型；2026-08-29 S4 归因后补定案：prompt 全空 + orientation 软门 + submit 无 plan 放行/降级（W4-R4）+ S5-1（fold 桥 reasoning + web_search 120s）+ S5-2（终端分层超时 + 中间回报）。

- [x] W1-R1 S1 代码 + S2 测试 + S3 重建（2026-08-28 完成）：复读/3-gram/空流链/审计结构化全量落地（orz-loop 567、容器冒烟三件套 orz 6cc8586）。
- [ ] W1-R1 S4 复验：EGFP / sam-cell-seg 真实 span 回放静默、构造真循环触发、零真实 400、命中率 ≥90%。
- [x] W2-R2 失败诊断 + 实体登记 + 服务调用形态收敛 + 黑板接线 + 全面审查处理（2026-08-28 完成）：diagnostics 签名词典 ≤2KB、entities 三域注册表、target 实体级、entities 分区；orz-loop 608/0/3、pytest 216。
- [ ] W3-R3 HA 目标架构落地余项：实体 id 形态、分区命名、注册表 Rust 形态（含实体 id 相对/绝对/大小写归一——本轮仅落地无状态部分：分隔符与 `./` 前缀；process/environment 状态探针扩展入账 R3）。
- [ ] W3-R3 A/B 验证：小样本跑分（含 THIN-HARNESS v0.4 R2b 按需读取观察；R2c `parse_retrieval_result_json` 兜底回收判定已随 2026-08-30 方向 C 裁决物理删除，不再观察——见 ADR-0010 §14.45）。
- [ ] W3-R3 清理与登记：旧序列门文档标记 withdrawn、ADR-0010 减法修订、CLI_PROJECT_INDEX 登记（含 BACKLOG/TODO 计数入账）。
- [x] W4-R4 prompt 全空 + orientation 软门 + submit 门修复（2026-08-29 完成）：BASE_SYSTEM_PROMPT 置空、orientation 软门（阈值 50、触发轮不禁工具）、submit 无 plan 降级状态展示。
- [x] W4-R4 S2 测试 + S3 重建 + S4 复验——**2026-08-31 按最低口径判定闭合**：S2/S3 完成 2026-08-29（orz-loop 611/0/3、三件套 5b3fe27）；S4=31 题已解 9（08-29 复验 8 + 10 题小批 rstan-to-pystan 1），未复验题不新增计数、全量成绩不再外推；8 工具面冻结不再删除、只做通用修正不为跑分特化（用户裁决）。
- [x] W4-R4 S5 修复 + S5-1（2026-08-29 完成，orz ad5f9ee）：A=fold 桥保留纯文本 assistant 消息 reasoning_content；B=orientation 触发轮放行工具（DC 强制模板轮仍禁工具）；web_search 客户端总超时 120s + connect 10s + 结构化 Timeout。
- [ ] W4-R4 S5-2 终端分层超时 + 中间回报（2026-08-29 用户裁决，独立批）——普通命令默认 300s / 程序脚本类 600s（模型可传 timeout 覆盖、上限 900s）；运行满 300s 未完成 → 机械插入一次「运行 + 工具自身情况」中间状态（单次仅一次），回报后默认继续、模型可主动中断；后台路径=终端 actor 自动后台化（满 300s 且解析超时 >300s 才后台化，后台截止=原解析超时）；事件面=`tool_running`（v0.2）+ ToolCompleted `running: true`。S1 代码 + S2 测试 + 全面审查处理已闭合（2026-08-29，见下）。
- [x] W4-R4 S5-2 S1 代码 + S2 测试 + 全面审查处理 + 审查处理补充（2026-08-29 完成）：宿主分类注入 + 自动后台化报告 + actor 后台截止 + `tool_running` 事件面（schema/verifier/fixtures）+ console 订单面同步（移除 is_background、timeout 上限 900s、默认 600s）；orz-tools/host/loop/tui/assurance 全绿、pytest 236。
- [x] W4-R4 S5-2 验证期发现（2026-08-29 登记）——**2026-09-07 闭合**：
  acaf_e2e 原登记 7 项失败，4 项已被中间批次修复；现存 3 项 shadow 场景
  失败实测根因 = Task C（2026-09-04）fail-closed 默认翻转后下游测试面未
  显式声明（journal 形态比对证实为 Blocked 形态，非 ad5f9ee 回归残留），
  与 orz-host 34 失败 + 1 挂死完全同族；修复 = shadow 场景显式
  `with_acaf_fail_closed(false)`（acaf_e2e 11 处 + orz-host 52 处）+
  大文件夹具对齐 TER T1.10 64K 门；orz-host 全量 249/0/4 EXIT=0（无跳过）、
  acaf_e2e 23/23。入口：
  docs/audits/ACAF_TEST_DEFAULT_FLIP_INFRA_FIX_2026-09-07.md。
- [x] CONTROLLER-SPLIT 二轮（2026-08-30 全部闭合）：N1-N5 全部分批完成（controller.rs 29,091 → 4,142 行，测试区 202 项按主题归位，验收 ≤10,000 行达成；每批独立提交 + 全量回归）。


### P0-0l WINDOWS-HIGH-NIST-MAX-FRICTION（P0；2026-09-01 设计定稿，实施待放行）

> 入口：[设计](docs/WINDOWS_HIGH_NIST_MAX_FRICTION_DESIGN_2026-09-01.md)；
> BACKLOG 0l；索引 AUTH-WINDOWS-HIGH-NIST-MAX-FRICTION。

- [x] 设计定稿落盘（2026-09-01）：N×F×P 三轴格 → Win32 原语映射 + 11 类
  Windows 特有摩擦点 + 承载方案（硬化 Windows VM 主载 / Linux arm 干跑）+
  摩擦探针与真实任务子集三臂 + 6 项可证伪缺口判据 + 8 项预期缺口假设。
  设计轮登记不动计数。
- [x] ① Linux arm 干跑（2026-09-01 闭环）：BoundaryBench 模式移植到现有
  Harbor 管线方法学验证——三臂 control/non-root/high-nist 12/12 reward=1.0、
  0 异常；enforcement-probe 三臂先验墙全过；OS 通道记账 non-root epErm×1 /
  high-nist eroFS×1；真实任务三臂同分。干跑记录见
  `_linux_arm_dryrun/PREP_RECORD_2026-09-01.md`。
  - [x] ② Windows 加固脚本 + enforcement-probe（S1/S2 + 全面审查处理完成
    2026-09-01；**S3 重建 + S4 本机冒烟闭环 2026-09-01；硬化 VM 三臂
    enforcement-probe 实机闭环 2026-09-02**）：`_windows_high_nist/`
    加固脚本（三臂模板、-Revert、日志）+
    enforcement-probe（每轴断言集）+ `windows_sandbox.py` 运行环境扩展
    （受限 token / LOW IL / AppContainer / Job / TEMP 重定向下 spawn orz
    命令树）+ run observation schema/verifier + CLI 入口；S2 全绿
    （assurance 新增 30 测试，46 passed 含既有）；S3=Windows x86_64
    三件套重建（orz f0eeb524，12m15s）+ 守卫符号核验；S4 本机冒烟=
    sandbox control 臂端到端 compliant + 冒烟修复 3 项
    （ProcThreadAttributeList 查询大小误判 / 管道 drain c_void_p 句柄 /
    CLI --command REMAINDER）；35 passed + 全量 1624 passed；
    **2026-09-02 硬化 VM 三臂实机闭环**=control/non-admin/high-nist 全
    PASS（non-admin 10/10、high-nist 19/19、sandbox observation
    compliant），修复链 6 项（SYSTEM 持久任务提权 / CPAU 选型与 CPTW
    回退 / session0 桌面 ACL / AppContainer TEMP+LOW+包目录 / run-user
    NTUSER.DAT 冻结 / 探针宿主 Python 化），案例库新增 6 篇 ORZ-WIN-*
    （进度见 `_windows_high_nist/S4_PROGRESS_2026-09-02.md`）。
- [x] ③ 三臂正式序列固化（2026-09-02 闭环：control 基线快照 2/2 /
  non-admin 10/10 / high-nist 19/19（AppLocker 恢复）全 PASS；修复驱动
  输出流误判、apply_hardening Get-ProtectedPaths SYSTEM profile 根、
  sandbox LoadUserProfileW 缺 UnloadUserProfileW；证据
  `_windows_high_nist/formal-2026-09-02/`，详见
  S4_PROGRESS_2026-09-02.md §10）。
- [x] ④ 任务集（首批 2 摩擦探针 + 1 真实任务 log-summary-date-ranges
  + verifier）→ control 臂基线（k=1）——2026-09-02 闭环：3/3
  attempt=success、observation=compliant、verifier 全 PASS，证据
  `_windows_high_nist/formal-2026-09-02/evidence-task-control/`；
  模型侧 agent k=1 依赖 ⑦ 网络/凭据后与 ⑤ 合并。详见
  S4_PROGRESS_2026-09-02.md §11。
- [x] ⑤ high-nist 小批（2026-09-02 机器侧闭环：驱动新增 tasknonadmin /
  taskhighnist 自包含 stage——基线恢复 + 模板加固 + 墙探针先验 + 任务批
  按臂落盘；high-nist 臂 3/3：墙探针 19/19、两个写探针 attempt=denied
  （WinError 5）、真实任务 success、verifier 全 PASS，证据
  `evidence-task-high-nist/`）。§7 判据 1/6 探针层核对完成；事件面判据
  （err 升压 / slow-stall / LIF / 降级链 / 假成功-事件面）依赖 agent
  k=1，并入 ⑦ 网络/凭据后与模型侧合并执行。详见
  S4_PROGRESS_2026-09-02.md §12。
- [ ] ⑥ 全量 + 记账（2026-09-02 口径：任务执行=control + high-nist
  双臂、不做 non-admin 任务消融——用户裁决，见 BACKLOG 0l / 设计
  §6/§10；主体=§6 剩余摩擦探针/真实任务移植 + high-nist 主载跑批 k=1 +
  记账；**⑥.1 机器侧 batch-2 已闭环（2026-09-02）**——control 9/9
  success、high-nist 9/9（denied×4 / blocked×2 / success×3），证据与
  缺口见 S4_PROGRESS §14；网络/Defender 长构建轴记账归 ⑦（Clash 网络 +
  模型侧 slow/stall），agent 侧 §7 事件面判据随 ⑦ 执行；整体勾选待 ⑦
  合并收口）。
  - [x] ⑥.1 batch-2 任务集移植 + verifier/runner/驱动扩展 + 双臂机器侧
    跑批（2026-09-02 闭环：6 新任务——probe-temp-write /
    probe-symlink-create / probe-service-create /
    probe-pip-user-install / probe-unsigned-ps1-run / regex-log，
    manifest 累计 9、batch `P0-0l-batch2`；control + high-nist 全
    PASS，见 S4_PROGRESS §14）。
- [ ] ⑦ 收尾：AppLocker 恢复复验、DeepSeek 凭据 CredRead 验证、临时
  任务/累积 ACE/旧目录清理、Clash 网络导入；补模型侧 §7 事件面判据
  （agent k=1，依赖网络/凭据重录）与 LoadUserProfileW 5023 候选修复
  （apply 后 hive 释放窗口，归 ⑦）。
  - [x] ⑦ 网络/凭据/收尾子项（2026-09-02 深夜闭环）：Clash 7897
    本地端口与经代理出站连通验证通过（netcheck）；AgentUser 凭据
    CredReadW non-admin 可读 / high-nist AppContainer WinError 5
    （摩擦点 #8，登记为 agent 轮凭据注入前置缺口）；驱动
    `-CheckpointName`（wrapup/网络轮默认 `S4-BASE-NET-2026-09-02`）；
    AppLocker 复验 Enabled + 4 规则、enforcement-probe 19/19 ×2；
    5023 候选修复（apply hive settle + sandbox 重试 + SYSTEM 就绪门 +
    `UnloadUserProfileW`→`UnloadUserProfile` 符号修复）并登记残余
    in-sandbox 5023 缺口；清理 10 个残留 AppContainer 包目录/工作区，
    无残留任务与进程。证据 `evidence-netcheck/` + `evidence-wrapup/`
    （S4_PROGRESS §15）。
  - [ ] ⑦ 模型侧 §7 事件面判据（agent k=1；2026-09-03 前置收敛：
    Clash 常驻/自启退役——用户裁决 VM 内不上网，DeepSeek 直连恒放行，
    NET 快照仅作维护用；AppContainer 凭据注入已闭环——orz Windows env
    通道 + 沙箱 `--env-file` + 实机链路证据
    `evidence-cred-inject/`，S4_PROGRESS §16。**2026-09-03：Windows
    orz.exe 已重建并同步（CF5662...CFAC4D6，旧二进制备份于
    `_windows_high_nist/backup/`）；agent stage 接线完成——新客机
    runner `_windows_high_nist/run/run_agent_arm.ps1` + 驱动
    `agentcontrol`/`agenthighnist` stage，bootstrap 取凭据 +
    `--env-file` 注入 + allowlist 221.204.163.76 已并入（未跑批，
    S4_PROGRESS §16.7）；**2026-09-03 执行前必做项闭环**——桥 op
    `vm-agent` live 执行路径（host `scripts/s4_vm_agent_run.ps1`）、
    worker restart、sync 全绿、VM 内 DryRun chunk1 通过（3 题/逐题
    timeout/env-file/allowlist 全对）；TB2.1 最新错题集 9 题资产树
    `_windows_high_nist/agent-tasks-tb2.1/` + 3 题/批口径（chunk1-3，
    S4_PROGRESS §16.8）；runner 修 1 项变量遮蔽（大小写不敏感参数覆盖）。
    真跑待放行**。**2026-09-03 §7 事件面分析（chunk1-f4 三 journal +
    LIF 离线重放）已完成并登记 S4_PROGRESS §16.15**：六判据结论=
    deny/err 通道零 fire、slow 1（mteb 300s 工具）、stall 0、判据 5
    本任务集 N/A、无事件面假写；新增候选缺口 F6–F10（run 墙钟模型
    不可见 / 长工具无中间回报 / file-write×junction 不兼容 /
    workspace 跨批残留泄漏 / permission deny 不入 deny 通道）。判据
    1–6 需 0.3.0 journal 复验后闭合；0.3.0 Windows 三件套待同步 VM。
- [ ] ⑦ 模型侧 §7 判据 0.3.0 复验（2026-09-03 已推进：0.3.0 三件套
  同步 + keystore 重建 + signer 复检完成；DeepSeek key 经用户轮换后
  改走 key 文件覆盖通道（ORZ_AGENT_KEY_FILE，VM AgentUser 凭据库写
    路径不可靠已登记）；chunk1-0303 3/3 ran（make-doom/gcode 仍死于
    840s 墙钟，mteb 348s 完成），§7 判据 1–6 复验结论与 F6–F10 状态
   更新见 S4_PROGRESS §16.16；F9 驱动修复已落地）。下一步：⑥ 全量
    主体 chunk2（path-tracing / train-fasttext /
    adaptive-rejection-sampler）3 题/批续跑。
- [ ] ⑧ 工具执行层改革（TER）：明细移至 [TODO2.md](TODO2.md)（M0 设计
  门 → M1 orz 主线 → M2 Windows runner/VM → M3 回归复验），开放项见
  [BACKLOG2.md](docs/BACKLOG2.md)；本行仅作主 TODO 指针，勾选以 TODO2
  为准。


### P0-0n GAP-APPROVAL-PROMPTER（**延期**；2026-09-06 排期登记，同日用户裁决延期）

延期裁决（2026-09-06 用户）：当前无具体设计文档的项均非急切或必需内容——
S1 设计定稿完成前不排期实施、不占当前工作集；本节保留作排期登记档案，
S1 定稿后按下列批次恢复推进。

GLM F2 处置转排期（2026-09-06 用户裁决）：`orz-host/src/approval.rs` 全文件
注释 + TODO 存根、`lib.rs` 标注 approval path still a stub——交互审批器补齐。
边界：审批器只承担交互审批呈现、决策回传与持久化，不收敛权限判定双实现
（OBS-PERMISSION-DUAL-IMPL 另案）；缺省 fail-closed 不变。

- [ ] S1 设计定稿：审批触发面（Interactive 权限门）+ 决策词汇（allow / deny /
  持久化语义，含 `approval_allow_persists_for_identical_bash` 既有语义收编）+
  permission 判定层接口 + TUI/ACP 两车道呈现 + 缺省 fail-closed；产出设计
  文档（涉及 ADR-0010 时按 §14.x 转录）。
- [ ] S2 实施：`approval.rs` 实装 + `lib.rs` approval stub 摘除 + 决策持久化 +
  契约/事件面登记（如涉及）。
- [ ] S3 测试与复验：单测矩阵 + orz-host 既有 flaky
  `approval_allow_persists_for_identical_bash` 复核收编 + Interactive 实机复验。
- [ ] S4 收口：GAP-APPROVAL-PROMPTER 状态翻转 + 索引/BACKLOG/TODO 同步 +
  门禁 Exit 0。

入口：[approval.rs](orz/crates/orz-host/src/approval.rs) /
[GLM 登记审计](docs/audits/GLM_EXTERNAL_REVIEW_REGISTRATION_2026-09-04.md) /
[处置 + S2 排期审计](docs/audits/P0_GOV_GLM_DISPOSITION_AND_TASK_D_S2_SCHEDULE_2026-09-06.md) /
[BACKLOG 0n](docs/BACKLOG_AND_PRIORITIES.md)。


### P0-0o S3/S4 集中实机验证批（2026-09-07 排期，同日用户裁决放行）

> 入口：[排期文档](docs/LIVE_VERIFICATION_BATCH_SCHEDULE_2026-09-07.md) /
> [BACKLOG 0o](docs/BACKLOG_AND_PRIORITIES.md)。
> 裁决：版本 0.3.1（过程验证版本，非修完版本）；放行 = 事实可严格放行即
> 闭合，不逐项流程放行；一份 journal 喂多个判据、统一分析一次性收口。
> orz 源冻结基线 `a1b73aeb`，批次期间仅版本 bump 一次提交。

- [x] T0 双平台重建（2026-09-07 完成）：orz 版本 bump 0.3.1（`d21b883e`）→
  Linux musl 三件套（BUILD_EXIT=0、static-pie 零 ld-linux、bookworm 容器
  冒烟三件执行、接线符号 dep_graph/session_volume_canonical_root/render_
  命中 → 闭合 0b ② / P2-14 S3 / P2-11×3 S3，P2-12/P2-13 S3 翻新注记）→
  Windows 三件套（staging-0.3.1，sha256 锁定）+ VM 同步（copy_to_vm 15/15
  哈希核验 + Program Files 0.3.1 换装备份）+ DryRun 全对（AGENT_RUN_OK=
  True / ERRORS=0）+ enforcement-probe 三臂全绿（control exit 0 / non-admin
  9/9 / high-nist 19/19，SYSTEM 提权作业通道、ResultPath 墙内落盘）→ TER
  T2.4 验收事实达成。证据 `_windows_high_nist/evidence-t0-restore-20260907/`；
  操作沉淀：构建脚本 apt 源 HTTPS 化；copy_to_vm 探针路径漂移（C:\s4\run\
  vs 规范 C:\s4\_windows_high_nist\run\）登记 T6 修正。
- [x] T1 批次 0（本地实机）（2026-09-07 完成）：0m S3 接线复验——
  `.gsa` symlink 会话卷端到端五段断言（评测容器挂载形态构造、终端 >8K
  截断补读链窗口、read_file 窗口、卷内非窗口 agent-invisible、run_tests
  输出窗口），orz-host 250 passed；orz `19585b88` + 父仓库 `ed145dc`
  （manifest 重算、门禁 Exit 0）。
- [x] T2 批次 W1（VM 确定性短批）（2026-09-07 完成）：TER T2.3 三判据
  全过（ALL_PASS）——① auto-bg 中报事件 @180s + 跨调用存活（R2 采样
  ≥95 tick 且增长至收尾）；② bg 交接 + 任务在子进程存活期间自然跑满；
  ③ `tool_running(status=idle_killed)` journal 事件。零 API（fake 场景
  驱动、无 --real）。产物 `_windows_high_nist/job-w1-fake-batch.ps1` +
  `evidence-w1-20260907/`。边界：完成提醒文本仅模型会话面注入、不落
  持久面，可达性判据归 W2 真实模型面（§7 C 组）。操作沉淀：`--real`
  抢占 fake provider；`-p` 工具集 shell 工具 = run_terminal_cmd
  （description 必填）；换新二进制须重刷 C:\workspace\acaf manifest
  （机器级 ORZ_ACAF_* 启动链 + signer sha256 自校验）；no-AC 墙形态必须
  --allowlist-ip（否则 egress 规则零创建、fail-closed 拒证）。
- [ ] T3 批次 W2（VM 任务批）：chunk1 复跑 + chunk2/chunk3 续跑（TB2.1
  错题集 9 题 3 题/批，`ORZ_F6_PUSH=on` + 官方墙钟唯一）——0l ⑥ agent
  主载 + 记账、0l ⑦ 模型侧 §7 判据、TER T3.1–T3.5。
  （2026-09-07 进度：chunk1（w2-chunk1-031）3/3 ran/compliant/journal 齐
  收——make-doom run_finished @862s 越过 840s 旧硬杀线（840 硬杀除名
  决定性实证）、gcode run_invalidated(wallclock) @900s 官方墙钟唯一、
  mteb 409s 完成（答案 Qwen/Qwen3-Embedding-8B，≠0303 基线，入 ⑥ 记账）；
  T3.3 F6 push cue 3+3 条 payload 可审（mteb 0 条=remaining 始终 >600s
  设计自洽）、T3.4 vm.js read_file 0 次（0303 为 20+ 轮）；通用零 400 +
  零哨兵 + 命中率 96.82%；P2-11 dep_graph 事件字段随 ToolCompleted 落
  journal（read 58 / write 4 边）。边界：T3.1 真实面无 ≥180s 前台命令
  （tool_running 未发生，机制已由 W1 fake 覆盖）；HF 探测 12.5s 判定失败
  （≤2s 理想受 DROP 模式限制，零泄漏）。首跑作废件（GBK 注释吞换行
  $workdir 空值 + elev 退出码假绿）双修复 `173ade6`。分析
  [`W2_CHUNK1_031_S4_ANALYSIS`](docs/audits/W2_CHUNK1_031_S4_ANALYSIS_2026-09-07.md)；
  chunk2（w2-chunk2-031）3/3 ran/compliant——train-fasttext run_finished
  @3443s（submit 两阶段确认，2471 事件/283 工具调用）、adaptive-rejection
  run_finished @889s 压线（ars.R+样本文件交付，工件已拉回宿主）、
  path-tracing run_invalidated(wallclock) @1800s（同 gcode 模型面耗时
  形态）；cue 3/2/3 全 ≤4；零 400/哨兵，命中率 96.81%；chunk2 journal
  零 API 自调 → v4-pro 4 次全归属 chunk1 mteb（对账闭环）。**新发现并
  修复：Reset-AppJunction 穿透删除**——任务切换时经 junction 递归清空
  前一任务工作区（F2 2026-09-03 引入的工件证据毁灭缺陷；journal 面
  无损），修复为 ReparsePoint 只删链接。chunk2 后按用户指示暂停
  （chunk3 待指示）。分析
  [`W2_CHUNK2_031_S4_ANALYSIS`](docs/audits/W2_CHUNK2_031_S4_ANALYSIS_2026-09-07.md)。）
- [ ] T4 批次 L（Linux/TB2 容器批）：make-doom（0b ③ + P2-14 S4 /
  P2-13 B4 / P2-12 S4 长会话遥测）、compile-compcert +
  hf-model-inference（0b ④ + web 通道 A/B）、dna-assembly（0d 后续 5）；
  全批通用统计：零 400 + 命中率 ≥90%（0d 后续 3/4、0j W1-R1）+ 哨兵 ≤3
  + P2-11 三项 S4（增量头/temporal/deps/retryable）。
- [ ] T5 批次 O（离线）：0j W1-R1 EGFP/sam-cell-seg span 回放 + 构造真
  循环触发；判据分析脚本化统一出表。
- [ ] T6 统一收口：逐项审计/复验记录（docs/audits/）+ BACKLOG/TODO/
  TODO2/BACKLOG2/索引同步 + 计数入账 + manifest 重算 + 门禁 Exit 0。
- [ ] 本轮载体（2026-09-13 用户裁决方向）：0b ⑤ 与 T4/T5 并入 **TB 2.1
  V4.1 代际新一轮跑批**（载体 0.5.0）；起跑前置已处置（盘余量 D: 7.30 →
  26.76 GB；语料/阈值冻结 P2-15 S1/S2 为第 0 步）。排期与代际记录纪律见
  [`排期记录`](docs/TB21_V41_GENERATION_ROUND_SCHEDULE_2026-09-13.md)。
- [x] **本轮定性 = k=1 筛查轮（2026-09-13 用户裁决）**：与项目此前 k=1 89 题批次同形——严格遵守官方口径设置，但**仅 k=1**；`K` 参数默认 5（榜单口径），本轮以 `K=1` 起跑（口径归属经用户裁决确认：**本轮计入 89 题**——第 0 轮 8 题 + 后续 1–5 批 81 题 = 89，每题恰 1 次；冻结清单批次划分与冻结 runner 不改，分批偏离登记在案）。
- [x] **本轮正文 = orz 自校验 + 摩擦项发现（2026-09-13 用户裁决，分数只作参考）**：优先序 ① 0z 新增面（预检门/回收阶梯/Job 硬上限/七族事件）真机整轮实测与前几轮死机形态是否复现；② 摩擦项逐项归因（orz 侧/模型侧/装置侧）→ 优化候选；③ 成绩只作参考。
- [ ] **参照线（软，不卡死）**：k=1 成绩达官方 DeepSeek-V4.1-Flash 的 TB 2.1 公布值 **90.6**（`api-docs.deepseek.com/updates/` 2026-09-10 条目）才值得补 k=5 取正式成绩；**未达不阻断本轮**。读数件：`scripts/tb21_round_gate.py`（R1 同口径回测 58/89 = 65.17%）。
- [ ] **摩擦清单产出（静默旁路）**：`scripts/tb21_friction_scan.py` 把整轮 journal/日志折叠成摩擦清单并归因；**R1 基线已扫描入档**（33 试次真工具失败 / 15 试次模型侧工具名错 / 42 框架信号 / 23 角色拒绝 / 11 权限拒绝 / 9 哨兵 / 0 资源事件），本轮结果与之对照。
- [ ] **两件读数件共同契约（2026-09-13 用户指示）**：**静默旁路、不阻断跑分流程**——只读、容忍半程数据、`--quiet` 零输出、**退出码恒 0**、不得接入跑批链作为阻断步；只作跑批中/后的快速初步分析。
- [x] **`.gsa` 放开按既有裁决回查收口（2026-09-13）**：ADR-0010 §14.56（09-06）→ **§14.61（09-07 用户裁决）修订为"内部区两段式有界开放"**（首读通知信封 → 二读放行 `open_after_notice`；shell 直读=已裁决接受的旁路）；容器实测 `cargo test -p orz-tools --lib two_stage` **7/7 通过**，0.5.0 双载体含三件符号。此前基于 R1（§14.61 之前）得出的"两车道不一致=缺口"结论**已作废并改写为历史基线**。
- [x] **GAP-ORZ-TEST-TARGET-UNIX-BUILD 立案并同日修复（2026-09-13，orz `23e7a378`）**：`cargo test -p orz-tools --lib` 在 Linux 编不过（5 处：`detect_windows_shell`/`WindowsShell` 未加 cfg 门，涉及 `terminal.rs::require_git_bash_backend` 与 `bash/mod.rs` 测试）；生产目标不受影响；修法=unix 放行/非 unix 维持；全量 orz-tools lib 2923 通过 / 17 失败（`lsp::tests::*`×16 与 `opencode::glob::gitignore_respected`×1，形态为容器缺件）。
- [x] **本轮经验泛化沉淀入案例库（2026-09-13 用户裁决）**：新增两条 incident —— [`ORZ-PLATFORM-TARGET-001`](docs/incidents/ORZ-PLATFORM-TARGET-001.md)（平台目标覆盖：**Windows 面全绿 ≠ 非 Windows 目标可编**，同窗口三批 9 处；生产目标与测试目标互不覆盖、只在容器 Linux 实测/载体重建窗口暴露）与 [`ORZ-VERDICT-EPOCH-001`](docs/incidents/ORZ-VERDICT-EPOCH-001.md)（结论代际纪律：**数据正确 ≠ 结论当前有效**）——并各晋级一条 `candidate` 案例（[`harness_environment/`](docs/cases/harness_environment/)）；「缺件伪装成失败」不新立案例，补入 [`ORZ-TOOL-BINARY-COMPAT-001`](docs/cases/harness_environment/ORZ-TOOL-BINARY-COMPAT-001-bundled-rg-glibc.md) 验证记录（缺 protoc / ripgrep / python3 与产品缺陷同形，`lsp::tests::*`×16 + `opencode::glob::gitignore_respected`×1 登记为环境形态不逐项归因）。计分语料（`regression/cases-v0.1.yaml`）与 `corpus_revision` 不动。
- [x] **TB 2.1 跑批容器侧排查 + 起跑口径裁决落档（2026-09-13）**：容器侧主体链路实测可用（0.5.0 载体在任务镜像内可执行 / 适配器真实命令行被接受 / ACAF fail-closed 生效 / 容器内出网 `api.deepseek.com` 401 应答 / Harbor 已登录且 dataset pin 一致 / 无残留容器）；登记两条边界——**容器内磁盘读数是盘内视角**（`volume 937.17 GiB free of 1006.85 GiB`，0z 磁盘轴在本轮容器形态不可取证）与 **Docker VM 内存 7.677 GiB**（8 题请求 8192 MB）；四项裁决落地：**不预拉镜像 + 清空全部镜像已执行**（回收 18.27 GB，镜像 36 → 0，宿主 `D:` 可用值不变——VHDX 稀疏、≈18 GB 转为盘内归还）、**第 0 轮 = 内存重题 8 题前置**（`-k 1` + **`-n 1`** + 窗口内不做其他内容；此前口头报 6 题系截断表之误，实测 8 题）、**契约漂移（`--max-tool-rounds` 被 0.5.0 静默忽略）仅记录不处理**、**第 0 轮计入 89 题并按 81+8 划分**（不改冻结 runner，分批偏离台账登记）。入口：[`容器侧排查`](docs/audits/TB21_V41_EVAL_CONTAINER_AUDIT_2026-09-13.md) / [`排期 §3.2`](docs/TB21_V41_GENERATION_ROUND_SCHEDULE_2026-09-13.md)。
- [ ] **第 0 轮起跑（内存重题前置轮）——主作业已中止；补跑作业已起（2026-09-13）**：`official-r0-heavy`（8 题：mcmc-sampling-stan / gpt2-codegolf / mteb-leaderboard / caffe-cifar-10 / torch-pipeline-parallelism / rstan-to-pystan / torch-tensor-parallelism / filter-js-from-html），`-k 1 -n 1 --upload --public`；镜像按需拉取（压缩 ≈2.06 GB）；跑完删该轮镜像 + 落资源遥测（五类）与摩擦读数（`tb21_round_gate.py` / `tb21_friction_scan.py` 静默旁路，退出码恒 0）；**期间不做其他内容**（起跑前清空态成立：Docker 镜像 0 / 容器 0、宿主 `D:` 可用 33.25 GiB）。入口：[`第 0 轮起跑记录`](docs/audits/TB21_V41_ROUND0_MEMORY_HEAVY_START_2026-09-13.md) / [`排期 §3.3`](docs/TB21_V41_GENERATION_ROUND_SCHEDULE_2026-09-13.md)。
- [ ] **第 0 轮中止与补跑（2026-09-13 用户裁决）**：第 8 题 `rstan-to-pystan` 被外部影响（本代理操作事故，记录 §6.6）⇒ **杀掉、主作业 `official-r0-heavy` 中止不上传**；补跑作业 `official-r0-netretry` **18:32 起跑**（5 题 = 4 个网络因素题 `caffe-cifar-10`/`filter-js-from-html`/`mcmc-sampling-stan`/`torch-pipeline-parallelism` ＋ 被中止的 `rstan-to-pystan`）。**首次执行「预拉镜像」裁决**：逐题 `docker pull` + 4 次重试 + digest 入档，**不全绿即中止**，实测 5/5 全绿；执行器新增 `--tasks`/`--job-name`/`--pull-only`/`--no-pre-pull`。**89 题口径更正**：有效试次 = 补跑 5 题 ＋ 本地保留 3 题（torch-tensor-parallelism / mteb-leaderboard / gpt2-codegolf）。
- [ ] **本轮两条硬发现（2026-09-13，待处置）**：① **模型动作面**——检索类调用占工具调用 **38%**（R1 全局基线 4.2%），四个检索型试次 **`web_search` 独占官方 agent 预算 54–91%** ⇒ 超时主因是检索而非模型慢；**容器内无浏览器**致 `browser_control` 13 / `browser_read` 5 / launch 14 全败（R1 同形 ⇒ 非新回归，但 V4.1 更常去撞）；失败面按**装置侧 20 / 设计内门 37 / 模型习惯 4** 三归因；第二条契约漂移 `--retrieval-mode local_browser` 已弃用且被忽略。② **0z 资源面在生产车道漏接**——`host_resource_snapshot` 未进 `orz-loop/src/host_exec.rs:154` 判定表 ⇒ 6 run **13 次 `audit-face loss` WARN、journal 0 事件**；设计要求（§4.5 F-EV-7）＋发射端（`orz-host/src/lib.rs:463`/`:1284`）＋事件类型（`orz-assurance …/event.rs:45`）三处齐备 ⇒ **生产车道实现漏接（非设计内）**；放大器 = **符号在位 ≠ 端到端接线**。**已按用户裁决处置（2026-09-13 追加）**：**停跑先修**——补跑作业 `official-r0-netretry` 18:37 停止；修复 `GAP-ORZ-HOST-RESOURCE-SNAPSHOT-DROP`（orz `ea777918`：映射表提取为可测 const + 补 `host_resource_snapshot` 项 + 覆盖性钉子，反向对照会红；`orz-loop --lib` 770/0/3、fmt 干净、父仓 manifest 重算 1446 条）。**影响面按用户裁定不止取证面**（loop 侧唯一消费者=journal 面+TUI 桥；缺 run_start/跨档读数即无法判定任务是否在资源压力下运行）。**修复进载体须双平台重建（待放行）**——**2026-09-14 已随 0ac S3 载体重建（版本 0.5.0 → 0.5.1，源冻结 orz `dbb42b1d`）进载体并发布 GitHub Release v0.5.1**（入口：[`重建记录`](docs/audits/0AC_S3_DUAL_PLATFORM_REBUILD_2026-09-14.md)）。
- [ ] **`GAP-ORZ-ADAPTER-FLAG-DRIFT`（candidate，仅记录）**：适配器旗标契约漂移两条——`--max-tool-rounds 999`（0.5.0 无此旗标、静默忽略）与 `--retrieval-mode local_browser`（0.5.0 已弃用后忽略，0t γ / ADR-0010 §14.65）；候选动作 = `run_official_2.1.sh` 起跑前「适配器旗标 ⊆ 载体接受表」机械对账（并把 `eval_browser` 开关一并纳入）。
- [ ] **深挖 A：agent 超时后 orz 不停（2026-09-13 定案，待裁决处置）**：`torch-tensor-parallelism` 实证——agent 阶段 07:46:01 被掐断后该 run 仍有 **272 条事件**（34 次模型往返 / 56 次工具调用），写到 08:00:49 容器删除才停；机制 = 适配器**后台子壳**起 orz + harbor **只取消自身等待、不杀容器内进程**；代价 = 空烧 API 14.5 min ＋ 与 verifier 抢容器 ＋ **一次已通过（verifier 13/13、reward=1）没被记账**（该题 verifier 阶段整 900 s 未返回，而其余三个超时题 verifier 17 s/104 s/9 min 正常返回）。处置候选：**传 `--ak max_wallclock=<超时−余量>`**（agent 侧自预算、**不改 harness 墙钟**）／装置侧收尾显式清理 orz／**定向复现**（带 vs 不带）闭环。
- [ ] **深挖 B：浏览器车道反馈（2026-09-13 定案，修法待裁决）**：**原因文案不缺**（失败带真实 cause：`no browser executable found (ORZ_BROWSER_PATH unset; searched: chrome, …, msedge)`；模型已逐字读懂并切 web 族）；**缺的是"别再来一次"**——惰性启动按 dispatch 重复、`tool_availability_check` 只探主工作面不探检索族、S1 设计明确拒绝 capability precheck ⇒ 13 次尝试属结构性（代价落在模型轮次与子代理预算，5 次 `subagent_wallclock_timeout_mid_tool`；单次尝试仅 3–13 ms）；取证面 `tool_completed` 失败载荷**不含 cause**。修法候选：run 级车道粘性／探针扩到检索族／收窄 S1 为"首次如实报因、终态失败不重复尝试"／cause 并入失败载荷——**不加浏览器**（用户已裁决）。
- [x] **墙钟预算必传（2026-09-13 用户裁决；回看 P0-2 既有修复）**：`--max-wallclock` 属 P0-2（2026-08-08）既有修复，2026-09-02 已收口为「官方 agent 超时值即唯一墙钟」。执行器改为**每题官方超时经 `--ak max_wallclock=<sec>` 透传**（不改 harness 墙钟/题目/verifier），并按超时**分组作业**（`-t<sec>`；5 题补跑集 = `t3600`/`t1800`/`t900`），`--no-wallclock` 留作对照；**偏离登记** = 由「每批一作业名」改为「按超时分组的同名族作业」。
- [ ] **`GAP-MECH-IMMEDIATE-FEEDBACK`（新需求，设计定稿待放行实施）**：机械层对**每个**模型请求都要**即时且有信息量**地回报——① 探针 `probe_scope` 扩 `retrieval_family`（浏览器/搜索引擎/web 通道三类硬设施在位读数，run 起始一次入 journal）；② 检索/网络请求统一**截止时间默认 10 000 ms**（`ORZ_RETRIEVAL_DEADLINE_MS` 可配），到点**立刻**返结构化错误，不等引擎自身 120 s；③ 新增稳定码 `capability_unreachable`（确定性不可达）与 `network_no_response`（到点无响应），返回面与 journal 双写、单事件自描述；④ 写进框架契约并加回归钉子。验收：检索类 `wall_ms` p99 ≤ 10 s、`subagent_wallclock_timeout_mid_tool` = 0。风险：会砍检索长尾 ⇒ 保留放宽开关 + 「10 s vs 现状」A/B 记录。
- [x] **两处提问核实（2026-09-13）**：① `torch-tensor-parallelism` 未记账**不是「没 submit」**，是**用满官方墙钟**（`submit` 0 次、`run_finished` 0 次），题目实际已解出（verifier 13/13、`reward.txt`=1），未记账的次因是 **verifier 阶段同样吃满 900 s**（R1 同题当时记了 `reward=1.0`）⇒ 新观察项：**agent 超时后 orz 未随之终止**；② 浏览器车道全败 = **`eval_browser` 注入开关未传**（浏览器由适配器按开关注入，默认关；本轮与 R1 都没传），`browser_control`/`browser_read` 因必须先启动浏览器而连带失败，`web_search`/`web_fetch` 走纯 HTTP 不受影响；**更正**：相对 R1 非回归，但相对 R3/R4/R4b 是能力回退。
- [ ] **`C6` 撤回 / 本轮第 0 轮全部重跑（2026-09-13 用户裁决）**：时间预算语义审计 §4 C6 **撤回**——本轮**全部重新跑**（含已跑 6 个 run）；第 0 轮账面**只作摩擦证据**，不再修计数、不再局部补跑，也不作成绩/基线。故上面两行「中止与补跑 / 有效试次 = 补跑 5 + 本地保留 3」的记账**仅作过程史**，不构成本轮 89 题的有效试次口径；大改（E1–E8）完成后按新载体整轮重跑 89 题（E9/R10）。入口：[`时间预算语义审计 §4/§5`](docs/audits/FRAMEWORK_TIME_BUDGET_SEMANTICS_AUDIT_2026-09-13.md)。
- [ ] **即时结果回报与流式检索设计 S1–S4（2026-09-13 落档，`current-design`；未实施）**：设计稿 [`IMMEDIATE_RESULT_DELIVERY_AND_STREAMING_RETRIEVAL_DESIGN_2026-09-13`](docs/IMMEDIATE_RESULT_DELIVERY_AND_STREAMING_RETRIEVAL_DESIGN_2026-09-13.md)（索引 `DESIGN-IMMEDIATE-RESULT-STREAMING-RETRIEVAL`）。**S1 探针（待用户放行；要发真实 API 调用）** = ①流式 `/responses` + `web_search` 记录 SSE 事件序列/时间戳，确认 10 s 内有无可用首事件；②分段续写探针（部分 assistant + reasoning + 注入事实，验证继续生成、无重复、无配对破损）——该实验决定走流式还是退分段。**S2** 机器合约先行（schema/verifier/fixtures：`retrieval_progress`/`retrieval_result`、`result_delivered`、`tool_completed` 失败载荷补 cause、探针扩检索族、五稳定码）。**S3** 实现（流式检索客户端 + 投递策略 I1–I3 + D7 机制 M1–M3 + 子代理提前收口 + semaphore acquire 截止 + 检索族探针 + cause 自描述，全部带开关 + A/B）。**S4** 小任务实机复验（不评正式分）通过后整轮重跑 89 题。
- [ ] **时间预算语义审计修正批次 R1–R10 / E1–E9（仅登记，待放行）**：审计 [`FRAMEWORK_TIME_BUDGET_SEMANTICS_AUDIT_2026-09-13`](docs/audits/FRAMEWORK_TIME_BUDGET_SEMANTICS_AUDIT_2026-09-13.md)（索引 `AUDIT-TIME-BUDGET-SEMANTICS`，`reference`）逐部件判定 7 处等待化/延迟形态——**D1** 检索子代理墙钟 600 s 到期才回报（3 次 ≈1800 s）/ **D2** `web_search` 非流式整包（n=116、p50 29.9 s、96.6% ≥10 s）/ **D3** 信号量 acquire 无独立截止（被 900 s 外层包住）/ **D4** 浏览器能力级不可达无 run 级记忆 + 探针不含检索族 + 失败载荷不含 cause / **D5** agent 超时后 orz 孤儿 14.5 min / **D6** verifier 通道吃满 900 s / **D7** 后台完成按「下一次工具边界」带回（用户裁决改掉、新增机制）。修正口径 = **四本时限分账**（`first_result_deadline` ≤10 s / `operation_deadline` / `total_budget` / `run_wallclock`）+ 通用契约「及时且有信息量」。**FP-2 不改、不是例外**（见审计附录 C）。
- [x] **载体侧重建（`GAP-ORZ-HOST-RESOURCE-SNAPSHOT-DROP` 进载体，2026-09-14 完成并发布）**：orz `ea777918` 修复已落码；进载体须**双平台重建**（Windows + Linux musl，约 40–60 min），重建后产生 0.5.x 新载体哈希 ⇒ 代际记录 / 适配器锁定值 / 冻结清单 `harness_artifacts` 同批更新；与 0ac 大改批次合并或单独排期，**待用户放行**——**已随 0ac S3 载体重建完成并发布 GitHub Release v0.5.1（2026-09-14，run `RUN-CLI-6aa77e19`；源冻结基线 orz `dbb42b1d`，版本 0.5.0 → 0.5.1；双平台三件套 + 冒烟 + 符号核证 + 载体换装；「代际记录 / 适配器锁定值 / 冻结清单」三项核查与处置见审计 §6）**。入口：[`0AC_S3_DUAL_PLATFORM_REBUILD_2026-09-14`](docs/audits/0AC_S3_DUAL_PLATFORM_REBUILD_2026-09-14.md)。


### P0-0t 检索子代理双车道并行标注面与 R3 摩擦处置（2026-09-09 用户裁决立项，同日两轮复核 + v1.3 复核收口（R1–R5 并入）；**S1 定稿转录 + S2 实施完成 2026-09-09**）

> 用户裁决（立项）：Clash 已关、本地镜像下 Google 检索与 Chrome 实质不可用
> 但无法降级原生检索；原生检索与本地浏览器检索都开放给检索子代理，像主
> agent 工具栏一样打标注，推荐先用本地浏览器检索；R3 新摩擦（FP-1～FP-9）
> 随本项统一处置。第一轮复核（v1.1）：FP-2 不教学不阻拦仅正常回传错误
> （根因=浏览器可拉起但实质不可用，可用性声明冲突同族）；不做标注健康度，
> 仅静态标注让模型自主选择；mteb 镜像转核查；framework_fallback 待议；
> FP-3 节点用户自理；FP-4 不干预；FP-5 拦截+正常反馈即要求（R3 已验证
> 满足）；FP-6 非问题。第二轮复核（v1.2）：浏览器问题=注入＋网络环境，
> 用户真机环境测试处理，orz 保证真机日常使用真实浏览器正常（锁版本等
> 不通用方案不做，跑分不重要）——FP-1 解除立项、活体探针取消；
> framework_fallback 采纳 γ 方案（三值检索模式退役、浏览器可用性纯事件
> 事实化，修改成本不作考量）。v1.3 复核收口（R1–R5 并入，用户确认）：
> 宿主机真机验证执行主体=执行代理（用户只提供环境/网络条件与裁决）；
> 授权门与模式退役分离（独立启用门承接 fail-closed）；schema 生产者侧
> 退役（枚举/verifier 只读回放）；S2 触点清单与替代 conformance 义务
> 补全；静态标注措辞与 token 口径定稿。证据：
> [0S 细节分析](docs/audits/0S_DETAIL_ANALYSIS_2026-09-09.md) /
> [mteb 核查](docs/audits/0T_MTEB_IMAGE_CHECK_2026-09-09.md)。
> 入口：[设计 v1.3](docs/RETRIEVAL_SUBAGENT_DUAL_LANE_DESIGN_2026-09-09.md) /
> [BACKLOG 0t](docs/BACKLOG_AND_PRIORITIES.md)。

- [x] S1 设计定稿（2026-09-09 完成）：v1.3 确认 + ADR-0010 §14.65 转录
  + §3.7 条 1/12 与 §14.40/§14.43–44 退役标注 + FUS-RETRIEVAL-MODE
  退役（索引转 `withdrawn`）+ FUS-RETRIEVAL-DUAL-LANE 登记 +
  BACKLOG/TODO/索引同步。
- [x] S2 实施（2026-09-09 完成）：orz 双车道（
  `retrieval_mode_requires_framework_fallback` 拒绝族退役、静态工具栏
  标注）+ γ 模式退役（`retrieval_mode`/`retrieval_mode_transition`/
  `framework_fallback`/机械降级语义退役；`browser_launch_result` 事实
  事件，schema/verifier/fixtures 先行 + 旧 journal 只读兼容；无
  `model_lane_switch`——ToolCompleted 即事实）+ 检索失败正常回传（FP-2：
  真实错误类别，无教学/无阻拦）。
  - Task 1（schema 先行 + 生产改写 + `browser_launch_result` 生产闭环）
    已实施；2026-09-09 三线全面审查收口（无 P0；P1×2 + P2×4 + P3），
    审查与修复批排期见
    [审查处理](docs/audits/0T_S2_TASK1_REVIEW_HANDLING_2026-09-09.md)。
- [x] S2-R 正式行动阶段（2026-09-09 排期并全部执行完成）：P1 设计定稿轮
  （P1-2b = browser_control Phase 1 导航级动作集
  navigate/back/forward/refresh/wait_load/snapshot + 每动作日志特征回传，
  价值裁决已放行；P1-2a 启动事实口径；P2-2 web_fetch 声明）→ P2 正确性批
  （P1-1 跨 prompt 浏览器生命周期 + P1-2a）→ P3 语义/声明批（P2-1 懒启动
  并发竞态 + P2-2 web_fetch 声明恢复 + P2-3 registry 声明语义 + P1-2b
  实现）→ P4 卫生批（P3 全项）→ P5 conformance 正反例 → P6 capture
  重写 → P7 S2 收口（本条目下方各阶段注记）。P8 S3 重建（已完成，
  2026-09-09，见下）→ P9 S4 实机复验
  + 宿主机可用性（R-D1 TUI 载体起手裁决）待续。记录不行动：多内核接口
  扩展、click/type/任意 JS eval（Phase 2 交互）。明细见
  [审查处理](docs/audits/0T_S2_TASK1_REVIEW_HANDLING_2026-09-09.md) §5/§3.6。
  - （2026-09-09 P1–P3 实现复核处理批 X1–X4 已执行：envelope 键名对齐
    action_status / ToolStarted→fact→ToolCompleted 全序断言 / 启动单次
    seam 计数测试 / P1 设计状态头与 §2.3 注记；ADR §14.65 转录 P1 新增项
    登记为 P7 必含项。证据：
    [0T_S2R_P1P3_IMPL_REVIEW](docs/audits/0T_S2R_P1P3_IMPL_REVIEW_2026-09-09.md)。）
  - （2026-09-09 P5 conformance 正反例补全完成：retrieval_enable_gate /
    browser_launch_result 两族 +11 场景（含 S1/S2/S3/S4、gate 反例与
    browser_control 声明面）；Rust↔Python 双侧法官工具表纳入
    browser_control，launch 义务覆盖 browser_read + browser_control；
    语料计数 233→244 钉死，Rust↔Python 对拍绿。证据：
    [0T_S2R_P1P3_IMPL_REVIEW](docs/audits/0T_S2R_P1P3_IMPL_REVIEW_2026-09-09.md)。）
  - （2026-09-09 P4 卫生批（S2-R3，P3 全项）执行完成：§3.7.1 条文号引用
    清理、env 非法值显式 warn、fact 全序断言、families 注释 31→33、过时
    注释与未用 import/dead code 清理；cargo check/clippy 零新增告警、fmt
    净。证据：[0T_S2R_P1P3_IMPL_REVIEW](docs/audits/0T_S2R_P1P3_IMPL_REVIEW_2026-09-09.md)
    §8 R3 建议批与 P1–P3 处理批 X1–X4。）
  - （2026-09-09 P6 三个 conformance capture 重写完成：capture_mode_off_refusal
    → retrieval_not_enabled 拒绝语义、capture_local_browser_capability_error
    → S1 启动失败 browser_launch_result 语义、capture_local_browser_read →
    S4 就绪成功读语义；13/25/24 事件各自重放绿。证据：同上 §12。）
  - （2026-09-09 P7 S2-T4 收口完成：ADR §14.65 v1.66 P1 新增项转录 + S2
    实施完成登记（本条）；BACKLOG/TODO/CLI_PROJECT_INDEX 同步；PDF 下载
    修复（profile 偏好种子层级 Default/Preferences，真机 e2e 绿）；重捕
    小批 10 fixture 换新 + Rust/Python 期望同步（12 capture + Python
    273 全绿）。证据：同上 §13/§14。）
- [x] S3 重建（双平台三件套 + manifest，2026-09-09 完成）：Windows
  release 三件套（`cargo build --release -p orz-bin`，CARGO_EXIT=0）+
  Linux musl 三件套（Docker `rust:1.97-slim`，BUILD_EXIT=0）构建冒烟绿；
  SHA256 锁定 + 0t 接线符号命中（`browser_control` / `browser_launch_result`
  / 双族静态标注）+ bookworm 冒烟三件加载执行全过；orz `92875fd5`、
  版本 0.3.2 不 bump。证据见
  [0T S3 重建记录](docs/audits/0T_S3_DUAL_PLATFORM_REBUILD_2026-09-09.md)。
- [ ] S4 实机复验：双车道存在性 / 静态标注存在性 / γ 退役兼容（旧回放过
  verifier、新生产不产出模式面）/ 失败回传形态 / 通用统计（判据见设计
  §5）；对照集 = R3 检索主导 4 题或搭 0o 批次 L。
- [ ] 真机日常浏览器可用性验证（**执行代理操作**，可与 S2/S3 并行；口径
  =日常使用——代理在宿主机环境以真实浏览器完成拉起→导航→交付，用户提供
  环境/网络条件与裁决、不担任操作者；orz 侧按暴露问题做通用性修复，不做
  锁版本等特化）。


### P0-0u 官方 R4 未通过 15 题复跑（0t S4 实机复验载体；2026-09-10 用户裁决放行）

> 裁决：机场波动不开代理，无代理直连 + 本地预拉镜像（15/15 在位）跑 R3
> 未通过 15 题——0t 双车道目标真实环境。口径与 R3 一致（k=1、一题一作业、
> 官方数据集 pin、官方墙钟唯一、eval_browser=true、deepseek-v4-flash）。
> 载体：orz 0.4.0 发布三件套（`D:/tb-eval/orz-linux/orz`，SHA256 `0797610e…`
> 与 [0.4.0 发布审计](docs/audits/0.4.0_RELEASE_2026-09-09.md) 锁定值一致；
> orz `a467d0f9` = 0t S3 `92875fd5` + 版本 bump，双车道代码同一）。
> 明细与判据预登记见 [BACKLOG 0u](docs/BACKLOG_AND_PRIORITIES.md)。

- [x] 跑批（2026-09-10 完成）：15/15 reward 面有效；**真实试次 13/15**——DeepSeek 余额 06:25 耗尽致 protein/train-fasttext 未运行、path-reverse 中断、filter-js pass-2 未运行（verifier-only 不计机制口径）；审计 §3。
- [x] journal 分析（2026-09-10 完成）：**0t S4 判据 1–5 全部过**（全卷零 γ 模式面字符串、web 族零拒绝、launch 事实事件带真实原因、失败回传无教学句、零 transport_retry）；四分类再归因——检索主导 4 题死亡形态质变（浏览器注入健康、双车道行为实证）但无代理 web 慢通道仍是预算杀手；轮次延迟/长命令结构性复现；model-extraction 靠 verifier 运气通过。入口：[R4 复跑审计](docs/audits/OFFICIAL_R4_UNSOLVED15_RERUN_2026-09-10.md)。
- [x] 余额受影响 4 题补跑（2026-09-10 完成，r4b 批次当日收尾）：4/4 真实试次，账面 64/89 → **65/89**（唯一翻案 path-tracing-reverse）；protein-assembly / train-fasttext / filter-js pass-2 为官方墙钟耗尽 0.0。入口：[R4 复跑审计 §2A](docs/audits/OFFICIAL_R4_UNSOLVED15_RERUN_2026-09-10.md)。
- [x] 模型代际补注（2026-09-12 完成，用户提问触发的事实补注、非新跑批、不改账面数值）：官方账面通过题 V4 Flash / V4.1 Flash 代际划分——2026-09-10 正午切换后，V4.1 Flash（兼容路由）通过题有且只有 r4b 的 path-tracing-reverse 一题（14:11–14:27），其余全部通过题均为真 V4 Flash（四种口径 65/74/76/77 下不敏感）；账面口径分歧（窄口径 65 vs 盘面 77）经用户裁决保留、不并轨（目前成绩并不可靠）。入口：[代际补注](docs/audits/OFFICIAL_LEDGER_MODEL_GENERATION_ANNOTATION_2026-09-12.md) / [R4 复跑审计 §2](docs/audits/OFFICIAL_R4_UNSOLVED15_RERUN_2026-09-10.md)。
- [ ] 收口余项：0t S4 判据 6 宿主机日常可用性另线；计数按 0t 小节闭合时入账。


### P0-0w TB 4.0 单题摩擦探针（2026-09-10 用户指示立项；**第三跑成立并完整跑完（2026-09-11）；开放项 = 审计 O1–O7**）

> 指示：找一个最适合摩擦的题做单测，关键是看 orz 的水平。
> 排期与判据（执行权威）：[`TB40_CTR_OPTIMIZATION_FRICTION_PROBE_SCHEDULE_2026-09-10`](docs/TB40_CTR_OPTIMIZATION_FRICTION_PROBE_SCHEDULE_2026-09-10.md)；执行器 [`run_tb40_ctr_probe.py`](scripts/run_tb40_ctr_probe.py)；BACKLOG 0w。

- [x] 预登记（2026-09-10）：选型 `terminal-bench/ctr-optimization`（长驻无输出进程 ≈4.8h + 本地 HTTP localhost:5000 + 大输出 + 硬时序 + 多服务/sidecar 采集，2C8G/gpus=0 本机可跑）；载体固定 orz 0.4.0（`0797610e…`，不叠 0v 未重建代码）；9 项摩擦点清单 F1–F9 + 10 条判据 + 中止/降级条件已落文档。
- [x] 前置核验（2026-09-10）：载体 SHA256 与 0.4.0 发布锁定值一致；三枚镜像（environment/sidecar/verifier）按 digest 预拉成功；仓库门禁仅剩预期 orz dirty。**数据集级解析故障未解除**（`terminal-bench/terminal-bench@4.0.0` 四次复测失败：服务端 statement timeout / HTTP/2 ConnectionState.CLOSED），改用单题解析路径 `harbor run -t terminal-bench/ctr-optimization`（实测可用；不落数据集级 digest 钉）。
- [x] 口径更正（2026-09-10）：初稿「非多容器」判断错误（实为 main + api sidecar + verifier 三镜像多服务题）；「52 道纯单容器题」计数受同一扫描缺陷影响，需重算。
- [x] 首跑（2026-09-10 19:03–19:17，用户放行）：**未成立、零有效试次**——pass 1 真实试次 9 分 27 秒（285 事件 / 32 工具调用）死于模型流中断（`transport_retry` zero_chunk ×10 耗尽 → `run_failed`，官方 `NonZeroAgentExitCodeError` / `reward 0.0`）；pass 2/3 未进入试次（harbor 鉴权换票失败，同时段 hub TLS 握手不通）。归 §5 环境顺延，不计失败批次。
- [x] 第二跑（2026-09-11 00:22 收尾）：**未成立、零试次、零模型 token**——三轮全部 `AgentSetupTimeoutError`（6m28s / 6m23s / 6m25s，整批 24m43s）：harbor 准备超时默认 360s，本步要在容器内装 Chromium，坏线路下超阈值。验收门与 `-r 0` 均被证实有效。
- [x] 执行器修订（累计 6 处 + 自测）：首跑后 4 处——每轮独立控制台日志；前置鉴权预检门（不通过即顺延、零轮次消耗，新增退出码 2）；`job_complete` 排除出错试次；harbor `-r 3` → `-r 0`。第二跑后 2 处——前置类失败签名扩围至准备阶段（`AgentSetupTimeoutError` / `EnvironmentStartTimeoutError`）；`--agent-setup-timeout-multiplier 4`（360s → 24min）。口径已同步排期文档 §2/§4/§5/§6/§7.6/§7.8。
- [x] 重跑（2026-09-11 00:31 起）：**批次成立并完整跑完**——4h53m00s / 774 工具轮 / 4356 万 tokens；`DONE exit=0`；reward 0.0（唯一失败项为 CTR 阈值 0.4363% vs 2.2%），框架侧判据全过。准备阶段约 15 分钟，**依赖新增的 ×4 准备超时余量才得以通过**。
- [x] 跑后分析（2026-09-11）：F1–F9 逐项对照 + 判据 1–10 判定 + 失败根因深度分析，落档 [`docs/audits/TB40_CTR_OPTIMIZATION_FRICTION_PROBE_2026-09-11.md`](docs/audits/TB40_CTR_OPTIMIZATION_FRICTION_PROBE_2026-09-11.md)。
- [x] 框架级开放项裁决（2026-09-11 用户，详见审计 §10）：**O1 进入设计**（判定非常有价值且属方向问题——与过度自制同源，提前处理可避免方向偏移）；**O2 缓议**；**O3 关闭**（查清后裁决不处理——机制按设计维持现状，机制在位且默认开，零触发因 agent 轮询并显式消费产物）；**O4 关闭**（机械层已拦住）；工具面不用 web 面**关闭**（归模型自身风格）。O5/O6/O7 为正面证据与观察，无待办。
- [x] O1 设计定稿（2026-09-11，含同日两轮复核，**无待裁决项**）：**初始轮中立问询**——[`docs/INITIAL_ORIENTATION_DECLARATION_DESIGN_2026-09-11.md`](docs/INITIAL_ORIENTATION_DECLARATION_DESIGN_2026-09-11.md)。形态按用户裁决：**首轮动作批次结束**时一次性机械注入三问（交付物与判定口径 / 大方向与阶段 / 做法优劣与任务评估）；**不携带机械审查报告**（审查依旧只在结尾）；**不在周期问询里加问**（动作中只回看与确定、不质疑）；不落黑板、不做消费审计（「只要让模型想了那就足够」）、复用现有票据类型；无工具首轮顺延边界已认可。含实施触点 7 项、S1–S4 排期建议。
- [ ] O1 实施（待放行）：S1 常量+会话状态+触发接线 → S2 事件面/schema/fixtures/Python 镜像同步 → S3 双平台重建 → S4 实机复验。
- 边界：非成绩批次——TB 4.0 榜单无任何 DeepSeek 型号，无同模型参照，不产出水平结论；本题不触碰检索车道，不验证 0t/0v。


### P0-0y NP1 机械身体集成支线（2026-09-11 用户裁决立项；设计定稿 + 全模块化承载确认；S1 模拟器验证载体已定案入账（2026-09-13）；M0 定版与 M1 接口定义未开始）

> 设计权威：[`NP1_ORZ_BODY_INTEGRATION_DESIGN_2026-09-11`](docs/NP1_ORZ_BODY_INTEGRATION_DESIGN_2026-09-11.md)（§9.3 全模块化承载确认 + 附录 D 登记前实机复核）；BACKLOG 0y；索引 `AUTH-NP1-BODY-INTEGRATION`。orz 之外扩展面：不修改 ADR-0010 与模型工具面。

- [ ] M0 定版（无依赖；本支线唯一分区写入批，前置全量分区备份）：升级官方最终版 `V3.2-260618-1045` → 全量备份 → 重新 root（Magisk）→ 重建去预装（`nothing_debloat` 模块）→ **Magisk 安全模式演练**（收尾验收，设计 §9.3）。**首次执行尝试已中止（2026-09-12，用户接管）**：全量备份（86 分区双侧哈希吻合）与 260618 官方镜像/预打补丁 boot 等产物已就位并保留；装置零改动回退；关键结论（unlock_critical 策略拒绝 / update_engine_client headers 失效 / sideload 瞬败待查）与复用路线见 [M0 中止记录](docs/audits/0Y_M0_ABORTED_FIRST_ATTEMPT_2026-09-12.md)。
- [x] **aarch64 载体重建（M1 前置）完成（2026-09-13，按 0.5.0 同源重建）**：源冻结 orz `1f13e5ec`（与 x86_64 S3 同源）→ x86 容器 + zig cc + rust-lld 交叉编译（12m11s，`BUILD_EXIT=0`）→ `orz` 73,007,720 B / `orz-signer` 1,757,984 B / `orz-acaf-provision` 1,595,128 B（AArch64 静态 ET_EXEC + `PT_INTERP=0`）；bookworm/alpine 双向加载冒烟绿 + 0z 七族符号全命中；旧 0.2.0 三件就地备份（`*-0.2.0.bak`）。复现入口 `scripts/build_orz_aarch64_musl_cross.sh`（LIFECYCLE `active`）。入口：[记录](docs/audits/0Y_AARCH64_REBUILD_2026-09-13.md)。
- [ ] M1 前置：接口定义（事件 schema / 动作契约 / 策略注册表形态）与 orz 机械层共同确定（设计 §14.1）。
- [x] §14.2 验证载体决策（**2026-09-12 用户裁决**）：引入模拟器为常设验证载体——载体集 = 模拟器（新增、常设）+ NP2（既有，流程纪律）+ NP1（端到端终验）；模拟器承担自有代码验证 + M5 补丁流程纪律干跑（坏补丁代价 = 删快照）；不验证厂商框架与 NP1 专属内核面（设计 §12 裁决段）。
- [x] **S1 验证载体搭建 + 定案与入账完成（2026-09-13 入账；盘面实做 2026-09-12）**：形态定案 = AVD `orz_body_a35`（android-35 `aosp_atd` x86_64、无头 `swiftshader_indirect`、`-no-snapshot`、port 5554）+ Magisk 30.7（ramdisk 直注 `PREINITDEVICE=vdd1` + `/data/adb/magisk` 持久面）+ `orz_body` 模块承载五面 + M5 补丁流水线（冻结 jar → baksmali 加性补丁 → smali 重组 → 反射探针 → 4 字节对齐重打包 → 确定性模块）。判据 1–7 成立：五面承载落点齐、两阶段 marker 同 `boot_id` 且守护 `ppid=1`、PREINITDEVICE A/B 六次引导（两组规则差异恰好一条）、M5 五态干跑（好补丁机械读回 MARKER / 坏补丁安装 / 坏 services 补丁 `zygote64` SIGABRT 开不了机 / `disable` 自救回原厂件 / 策略回退）。**同批三条发现**：A「禁用模块 + 重启」不撤销已入内核的 SELinux 规则（设计 §10.3 层 1 对该条不完整，候选处置待裁决）；B 策略哈希三态更正（`8242a06d` 对照组 / `d2365b61` 注入态 / `d1749c5d` 无 Magisk 干净态）；C 补丁生效判据需带「装载时机」字段（与 §14.1 同批定）。入口：[S1 定案记录](docs/audits/0Y_S1_EMULATOR_CARRIER_2026-09-13.md)。
- [ ] **S1 收尾：orz x86_64 musl 三件套上机冒烟**（判据 8，未成立）——真 orz 替换 busybox 桩守护，验静态 ELF 在安卓内核直接执行（与 `BODY-PRE-01` 同理、无需新构建）；同批把干跑串成一键复现脚本（当前只有制品 + 逐条命令历史）。



### P0-0z 真机资源安全边界与崩溃收尾（2026-09-12 用户裁决立项；**设计完成、S1（A + D + F）与 S1.1 复核收口均已落码**；不换盘不换卷）

> 设计权威：[`HOST_RESOURCE_SAFETY_DESIGN_2026-09-12`](docs/HOST_RESOURCE_SAFETY_DESIGN_2026-09-12.md)；BACKLOG 0z；索引 `FUS-HOST-RESOURCE-SAFETY` + `GAP-READ-FILE-TEXT-ENCODING`。
> 触发：本轮真机自举两轮非正常终止（Run `RUN-CLI-6aa4f384` 盘满致命退出 / Run `RUN-CLI-6aa50fdf` commit 耗尽 abort）。
> 追加裁决（2026-09-12）：**不挂沙箱**——以「机械硬门 + 回收兜底 + 必在收尾」替代（设计 §3.4）；借镜调研走一手来源： [`HOST_RESOURCE_MANAGEMENT_BORROW_RESEARCH_2026-09-12`](docs/HOST_RESOURCE_MANAGEMENT_BORROW_RESEARCH_2026-09-12.md)。
> 追加裁决（2026-09-12 二轮）：**机械层优先 + 信息返回**（沿用 `OPS-PROTOCOL`"判断下沉机械层"正典）；**资源硬上限替代动态并发限流**（Job Object 内核强制：commit / 并发进程数 / CPU 速率）。承接：`OPS-PROTOCOL`（删除安全 → E）、`GAP-ENCODING-GATE`（编码门 → D 复用，不新造字段）。
> 追加裁决（2026-09-12 三轮，终裁）：回收保留策略由"容量溢出"改**轮数窗口（默认 2 轮、最多扩至 3）**，**超预算缩减或拒绝、不向模型二次确认**；**回收站整体取消**（同卷不释放空间、可恢复价值被窗口覆盖；容量口径与 `trash` 态仅作历史回查记录）；`unknown` 类改 **fail-closed 拒绝**；**主撤销面 = git 兜底**（tracked 内容；既有 `snapshot_created`，本轮实测 25 次）+ `cache` 可再生 + 证据面拒绝；hard 档跳过窗口直删 `cache`（设计 §4.6.1）。

- [x] **S1 代码面（缺口 A + D + F）落码完成并提交（2026-09-12，orz `73a8f25c`）**：派发前资源预检门（动作分档 + 余量读数准入 + 四档阶梯计算 + `HostCapacitySnapshot` 装配期注入；`orz-host/src/resource_gate.rs`）；编码门复用（`read_file` 文本族 decode-first，复用既有 `output_encoding`，**不新造字段**）；资源硬上限（run 级 ceilings 决策 + 每个工具调用 Job 承载 `JOB_OBJECT_LIMIT_JOB_MEMORY` / `ACTIVE_PROCESS` / CPU hard cap / `KILL_ON_JOB_CLOSE` + 内核读回；`xai-tty-utils/src/resource_job.rs`）。manifest 重算 1444 条 + 门禁 `valid: true`；证据与三处偏离：[`S1 实施审计`](docs/audits/0Z_S1_IMPLEMENTATION_AUDIT_2026-09-12.md)。
- [x] **S1.1 复核收口落码完成并提交（2026-09-12，orz `ea794f90`，6 文件 +1234/−178）**：独立复核 10 项全部处置（[`复核`](docs/audits/0Z_S1_INDEPENDENT_REVIEW_2026-09-12.md) / [`收口审计`](docs/audits/0Z_S1_REVIEW_CLOSURE_2026-09-12.md)）——**F-1 两级 Job 恢复**（run 级 job 持限项 + 调用级 job 持 `KILL_ON_JOB_CLOSE`，**先根后子**指派；一手来源 Nested Jobs + 本机探针；并发聚合残余消除）、**F-2 `run_tests` 入门 + 挂 Job**、**F-3 `ACTIVE_PROCESS = 2×核数+8`（下限 16）**、**F-4 commit 上限改 `min(帽, 装配期余量−1 GiB)`（下限 2 GiB）+ 盘—内存轴间耦合登记**、**F-5 目标卷静态写入判定 + 逐卷读数**、**F-6 新增 `unknown` 档**、**F-7 here-string/heredoc 剥体留头（语料误判 56→49、零漏判）**、**F-8 命名/计数/注释收口**、**F-9 端到端钉子 + attach 失败计数**。回归 xai-tty-utils 27/0、orz-host lib 317/0/5 单线程、orz-tools encoding 20/0 与 read_file 121/0、fmt 干净；manifest 重算 1444 条 + 门禁 `valid: true`。设计同批更新（§4.2 补注 / §4.7.1 裁决表 / §9 耦合与代价 / §11 裁决 11–14）。
- [x] **S2 机制与合约收口完成并提交（2026-09-12，orz `b3479716`，18 文件 +3422/−30）**：**C**（ENOSPC 分类→三退→Degraded 骨架-only + 8KiB 界 + degraded 摘要注入 + DegradedDropped 不推进链 + `run_terminated` 新终止事件 + TERMINAL.json 侧车 + reserve 64MiB + conformance `degraded_complete` 分类不混 invalid——判据 6/7）；**B**（`RunResourceJob::kill` 树杀面 + SpawnSink 落 attach_pid 唯一汇点 + `.gsa/process_trees/` 登记 + 三条件扫除矩阵（指纹=规范化镜像哈希，pid 复用防护）+ 三硬化 + sweep-log 审计行 + 装配期/收尾双 sweep + 真实进程正负例 6/6——判据 5）；**E**（分类三态 + 轮数窗口 2/3 + 超预算拒绝不询问 + §4.7.1 第 14 条在跑产物面保护 + 审计先行 + 60s 冷却 + 门后机械触发 + reclaim-direct/hard 跳窗直删）；**事件族**（7 新事件类型 + payload schema ×7 + 注册表 28→35 + fixtures 21 件 + Rust 法官七族 + Python 镜像逐格零差 + 对拍场景 +2；终端四族 schema 加可选 degraded；`host_resource_denied` producer 落拒绝臂；hard 档 §4.8 全路径 planned→树杀→executed）。回归 assurance 218/0、loop 769/0/3、host 324/0/5 单线程、fmt 干净、新增代码 clippy 零告警（dunce 替代禁用 canonicalize）；manifest 重算 1446 条 + 门禁 `valid: true`。边界：`resource_limit_hit` 判官/fixture 就位、producer 归 S4（判据 12 violation 读回）；hard 档 run 终态由 loop §4.3 端承担。
- [x] **S2 全面复审 + 返工收口（2026-09-13）**：三路独立并行审查（C / B+E / 事件面）判原批次未达出口（5 P0 / 11 P1 / 14 P2 / 10 P3）——**五项 P0 全部返工**（F-C-1 repair 4KiB 尾窗整卷清零→ 64KiB 倒序扫描 + 撕裂注入缝钉子；F-C-2 BufWriter 残段拼接→错误路径弃缓冲；F-BE-1 finalize 跨 run 误杀→登记 run_id + `SweepMode::Finalize` 只候选本 run 行；F-BE-2 回收在跑保护恒空 + hard 先回收后杀→ in_flight_targets 登记面 + 先杀后回收 + expiry 在跑复核 + 门单 evaluate；F-EV-1 `resource_exhausted` 缺必填 call_ids→登记表枚举）；**P1 返工**（F-C-3 两侧终态清单/豁免同步、F-C-4 RunRecorder DegradedDropped 收口、F-EV-2 Python 七函数接 `validate_journal_text` 入口、F-EV-3 终态前 finalize+三路 drain（`LoopHost::finalize_process_trees` 新 trait 面）、F-BE-5 创建时间守卫（5s 容差）+ NotRunning 行清理、F-BE-6 嵌套候选发现（深度 2）、F-BE-7 阶梯排序）；**裁决落定（2026-09-13 用户裁决 15/16，已实施）**：F-BE-3=per-call-job 杀面（选项 a，已实施 orz `a42fa0c3`——SpawnObservation 携带复制 call job 句柄 + live_call_jobs 登记面 + terminate_heavy_call_jobs 只杀重档调用树 + 内核级爆半径测试）、F-C-8=fail-closed 维持（盘满不起新 run，设计 §4.3 第 8 条登记）；**随批 P2**：F-C-7/9、F-BE-8/11/12/14、F-EV-5/8/9/10/11（含 run 总量上限 3× 预算、窗口轮=工具调用轮、审计行 reap 标签、source_quality 词表修正 + unavailable 正例）；**F-EV-4 裁决已定**：facts-drain 通道非 0q 违反（ADR 转录按实登记）；登记 S3/S4：F-BE-4/EV-7 残段（run_terminated{resource_exhausted} + resource_limit_hit producer）、F-BE-7 残段（更早 run scratch 层）、F-C-5/6/11 等。回归 assurance 219/0、loop 769/0/3、host 324/0/5 单线程、recorder 13/13（含撕裂钉子）、process_tree 6/6、对拍 7/7；审计：[`0Z_S2_COMPREHENSIVE_REVIEW_HANDLING_2026-09-13`](docs/audits/0Z_S2_COMPREHENSIVE_REVIEW_HANDLING_2026-09-13.md)。
- [x] **S3 载体重建完成并发布 GitHub Release v0.5.0（2026-09-13，用户指示直接打 0.5.0 并发布、跳过 0.4.4）**：版本 bump **0.4.3 → 0.5.0**（orz `187cb9d7`，两文件两行）；构建中发现 **第三处 Linux 构建断裂**并修复（orz `1f13e5ec`——orz-host lib.rs 无条件读取 `SpawnObservation.job_handle_dup`（字段 `#[cfg(windows)]`，S2R 引入；非 Windows 登记 `job_handle=0` 既有语义 + 关句柄臂门控；P1-4 批只修 xai-tty-utils 两处、本处漏网，musl 重建实测暴露），Windows 侧按新 HEAD 作废重建；**源冻结基线 = orz `1f13e5ec`**。双平台三件套（Windows release 52,742,144 B `a45b60e5…` / Linux musl orz 109,982,680 B `393eee34…` 等）+ ELF static-pie 核验 OK + bookworm/alpine 双向加载冒烟绿 + 0z 接线符号双平台全命中（七族事件 + 门/回收/进程树/降级标记）+ 载体 `D:\tb-eval\orz-windows` / `orz-linux` 刷新（0.4.3 六件哈希重建前录 `orz-0.4.3-carrier-hashes-before-rebuild.txt` 备查）+ manifest 1446 条 + 门禁 `valid: true`；**GitHub Release v0.5.0 已发布**（`SilverWhite/CLI`，tar.gz/zip 双资产 + notes，包内容清单与资产 size 核验）。证据：[`0Z_S3_DUAL_PLATFORM_REBUILD_2026-09-13`](docs/audits/0Z_S3_DUAL_PLATFORM_REBUILD_2026-09-13.md)。
- [ ] S4 实机复验（判据 1–13）：真机长任务复跑（含重活路径）+ 满盘注入 + abort 注入 + 编码样本；**0v-C 已另行闭合（`ba934af8`），本轮不搭车**；载体 **0.5.1**（2026-09-14 载体重建刷新；原记 0.5.0）；S2 复审登记残段（F-BE-4/EV-7 producer 读回、F-BE-7 残段、F-C-5/6/11）随取证核验。
- [x] 裁决封闭（2026-09-12）：阈值（8 GiB / 25% / 16-8-**5**-2 GiB 阶梯）、硬上限（commit 80% 类 + CPU 80% + 并发=核数）、Job 限项（启用 commit/并发/CPU/KILL_ON_JOB_CLOSE；不启用每进程内存与 working set）、hard 档树杀（允许，四条限制）、孤儿扫除（三条件 + 三硬化）、soft 档柔性降级（默认关）、回收（轮数窗口 2 轮/上限 3、reclaim-direct 5 GiB、超预算拒绝不询问、**回收站取消**）、0z 与 0v-C 不合批。**设计无开放裁决项**；详见设计 §11 裁决记录 / §4.8。
- [x] **S1.1 裁决修订（2026-09-12，用户授权工程裁决；设计 §4.7.1 + §11 裁决 11–14）**：**两级 Job 保留**（先根后子；修订 S1 的降级结论）、**`ACTIVE_PROCESS = 2 × 核数 + 8`（下限 16；修订原"核数"臂）**、**commit 上限 = `min(min(80%×limit, limit−4 GiB), 装配期余量 − 1 GiB)`（下限 2 GiB）**、`run_tests` 入门、目标卷按静态写入目标判定、新增 `unknown` 档、here-string/heredoc 剥体留头；**盘—内存轴间耦合登记**；**回收与在跑重活的次序**登记为 S2 承重项。


### 0ae 上下文软门与模型参与压缩（P1；2026-09-15 设计定稿并实施落码（orz `f0040557`），同日审查修复批（`183fbb08`）；A/B 判据留 S4 实机）

- [x] **D0–D4 全部落码（2026-09-15 过夜批，orz `f0040557`）**——D0 黑板写入面：`blackboard_write(section∈{plan,notes}, ≤8K)` ＋ 水位状态标【x.xM/10M】恒挂 live 读响应头（`plan_write` 事件族 + schema `section`/`content_chars` 增量；notes 分区随 epoch 快照；ReadOnly 类无条件声明；8 工具面显式例外 +1）。
- [x] D1 首轮 plan 问询＋补救规则——initial-round 间隙追加一问（0x 模板与 signer 摘要面不动）+ N=20 一次性提醒（`model_note_count()==0` 判定）。
- [x] D2 注意力阶梯——新模块 `attention_ladder.rs`：128K 打断式 / 160K 提醒式 / 300K・500K・600K・700K 软提醒 / 800K 截断式硬提醒；每级一次、920K 后 `rearm()`；`ORZ_LADDER_*_K` 全 env 可配；fire 经 `mechanical_audit_update` 落账。
- [x] D3 920K 压缩轮——`PendingCheckpoint::ModelCompression` ≤3 轮无工具窗口；窗口结束机械模板压缩兜底（`attention_920k_window`）+ `model_participated` 如实落账；**按标注分区选择性折叠 v1 留 S4 精化**。
- [x] D4 折叠桥增补——`run_context_block`（基线 `capture_run_baseline` + 自编辑清单 + 最近 5 指纹）随推进冻结，前缀字节稳定保持。
- [ ] S4 型 A/B 判据（设计 §8 八项）——实机长 run 验证，待放行。
- [x] **审查修复批（2026-09-15，orz `183fbb08`；[审计件](docs/audits/QUAD_BATCH_DEEP_REVIEW_2026-09-15.md)）**：D3 压缩窗口参与修复（窗口轮仅 `blackboard_write` 面＋派发＋延迟收口，审查 P0 闭合）；`mechanical_audit_update` 契约合规（schema kind 枚举 3→6＋Python 镜像＋常量单一源＋逐字钉子）；plan_write `validation` 形状；D4 非 git 降级；阶梯水位；压缩 NoOp 不 rearm；plan-gate 轮面补 `blackboard_write`。
- **死面披露（审查 0AE-C4）**：阶梯量尺＝折叠后实测 prompt，机械 fold@128K 未动 ⇒ D2≥160K 各级与 D3 920K 窗口默认配置下常态不可达。**用户裁决（2026-09-15）：0ae 暂不动、后续还要整体修改，死面维持登记不激活；slider 由邻线补全中**（细节见审计件 §10/§11）。
- 排期：独立批，前置＝0ac ①-b 收尾批（不与 ①-b 合批）。入口：[设计稿](docs/CONTEXT_SOFT_GATE_MODEL_PARTICIPATED_COMPRESSION_DESIGN_2026-09-15.md) / [深审](docs/audits/0AC_S3B_RUN_DEEP_REVIEW_2026-09-15.md) / [BACKLOG 0ae](docs/BACKLOG_AND_PRIORITIES.md)。


### 0ah 滑块上下文（v8 模型自控注意力窗口；P1；2026-09-15 立项；**2026-09-16 勘误**——v7「常驻滑窗／机械驱逐」系记录错误、S1 五连批依错误记录落码，**实现更正批已落码（未提交）＋只读审查处置同日闭合；登记待放行**）

- [x] **必定压缩补足批（2026-09-24 用户裁决；ADR-0010 §14.77／v1.79；设计稿 §14 补足；落码未提交）**：T1（500K 估算 ≈385K 真实）由机械硬截断改为**必定压缩三步升级**——首问强制压缩窗（≤3 轮，缺省＝主滑块外全部已闭合分块）→ 窗口未产出 ⇒ 升级再询问（明示质量严重衰减＋最后机会）→ 仍未产出 ⇒ 机械截断兜底（仅留主滑块＋告知「需要前置上下文时回查存档」）；700K 守卫与指针化路径不动；H1 文案随新语义改写；升级计数在压缩达成／截断执行后复位。orz-loop 串行 **845/0**（含三步升级钉）；S3 载体随下批放行，真机观察（压缩次数／实得削减对照 0bi 基线）随下一狗粮轮。入口：[BACKLOG 0ah](docs/BACKLOG_AND_PRIORITIES.md) / ADR-0010 §14.77。

- **2026-09-16 勘误与 v8 待办**（用户裁定＋放行登记）：v8＝**模型自控注意力窗口**——模型面＝主滑块 x（160K 估算≈123K 真实）＋主滑块以外 y=32K 分块（仅分块、不流出模型面）＋机械摘要行；减少模型面只有模型压缩与 T1 硬截断；阶梯＝软提醒 192/224/256/288K → 320K 硬打断 → 500K 硬截断（950K 取消），依据＝1M 上下文模型普遍注意力水平；机械轨／语义轨并存（均作用于模型面）；模型压缩不覆盖本地面（全量留档＋按块回放）；**机械轨边界（同日追加裁定）**：机械轨照常按其内容策略工作，但**不再承担总窗口压缩**——退役 rhythm 192K（H＋缓冲，视图尺）触发与视图兜底 256K（`compact_messages` 截断至 160K），模型面总量只由模型自压与 H1/T1 管。**待办流水**：① 实现更正（模型面投影层／块表／新阶梯／H1·T1／按块回放／本地面零覆盖）→ ② 判据读数（单轮如实标注、不作架构结论）→ ③ v7→v8 收口。入口：[`v8 设计稿`](docs/CONTEXT_SLIDER_V8_DESIGN_2026-09-16.md) / [`成本重算`](docs/CONTEXT_SLIDER_V8_COST_RECOMPUTATION_2026-09-16.md) / ADR-0010 §14.69（v1.70）。

- [x] **S1 五连批提交收尾与门禁回绿**（2026-09-15 用户确认放行「第 ① 步提交收尾」）：orz 子模块单笔 `61982a56`（13 文件 +4588/−850：`context_scale.rs` 新增、`attention_ladder.rs` 删除）＋父仓账本/契约单笔 `524518fe`（12 文件）；`orz_source_manifest.sha256` 重算 1450 条（差异面 12 行）；**门禁 `valid: true`（`error_count: 0`）**。入口：[索引 v3.40 头行](CLI_PROJECT_INDEX.md) / [任务书 §10.5](docs/CONTEXT_DYNAMIC_SLIDER_S1_IMPLEMENTATION_TASK_2026-09-15.md)。

- [x] **950K 截留二次改判（2026-09-15 用户三条裁定并当日落码；回执＝[`S1 任务书 §10.10`](docs/CONTEXT_DYNAMIC_SLIDER_S1_IMPLEMENTATION_TASK_2026-09-15.md)）**：① **窗口内溢出可机械消化**——`action_ledger::pointerize_oversized_tool_results`：截留后仍在线之上时，把超过 **8K** 估计（`OVERSIZED_TOOL_RESULT_CAP_TOKENS`）的 `Role::Tool` 正文换成「**原文头部 ≤400 字符 ＋ 回读指针**」（指向 run journal `events.jsonl`，按 `call_id=` 检索；声明「读取当时的快照、编辑/决策前新鲜读取」），**大者优先**直到降线；**`role`/`tool_call_id` 不动（配对不破坏）／幂等／不动非工具消息**（先例＝OUTPUT-DEGENERATION-GUARD／ADR-0010 §14.33）；② **模型面措辞统一「当前上下文窗口」**——500K／900K 提醒、首次驱逐固化提醒、窗口任务块、窗口降级块、压缩失败告知块、S1 台账固定指针一律去「滑块／驻留带」（内部注释与设计稿保留分区术语）；③ **截留告知不报容量读数**（本地存量无上限；500K/900K 提醒仍按 A6 带读数），告知改为逐项如实：窗口外 N 轮已移出／窗口内 M 个超大结果已指针化（含 `call_id` 指引）／两者皆无即「机械层到此为止」。**落账**：anomaly 三值 `hard_context_compaction_failed_truncated`／`_result_pointerized`（新）／`_slider_bound`，summary 增 `pointerized_results=`／`freed_tokens=`。**读数**：orz-loop **818/0/3**（+2 钉）、orz-host **325/0/5**（串行）、orz-assurance 229＋fixtures 全绿、fmt 干净、clippy 新增零告警、门禁 `valid: true`（`error_count: 0`；2026-09-15 提交收尾后实测）。入口：[`设计稿 §3.4.1／§5／§8`](docs/CONTEXT_DYNAMIC_SLIDER_DESIGN_2026-09-15.md)。

- [x] **950K 失败处置改判（2026-09-15 用户裁定并当日落码；回执＝[`S1 任务书 §10.9`](docs/CONTEXT_DYNAMIC_SLIDER_S1_IMPLEMENTATION_TASK_2026-09-15.md)）**：口径「**压不动的话不进 NoOp 了，强硬只保留当前滑块，将其他的丢弃，并明确返回『上一轮上下文压缩失败，已机械截留』，让模型自己决定下一步，这样的话任务还能继续**」——① **硬截留**：机械层强制压一次（`reason=context_scale`，切点＝驻留带 L 换算）⇒ 视图只剩「前置＋固定指针＋当前滑块」，行入台账、逐字原文留 journal；**删 `hard_context_stopped`**（不永久停手）、不进 NoOp、**950K 档不再强制开窗**（`last_resort_block` 退役、生产零调用）；② **明确告知**：`context_scale::TRUNCATION_FAILURE_HEADLINE`＋`compaction_failed_truncation_block`（有截留报「滑块之外的 N 轮已移出」＋回读指引；零截留如实报「滑块之外已无可截留内容——溢出位于滑块内」；末句给低成本选项＋「任务无需中止」）；③ **落账**：新 key `context_scale:hard_950k_intercepted`、anomaly `hard_context_compaction_failed_truncated`／`_slider_bound`；告知每 run 一次、冷却 4 loop 迭代（冷却后新累积的滑块外轮次仍可再截留＝不是逐轮重压）。**读数**：orz-loop **816/0/3**（+2 钉）、orz-host 325/0/5、orz-assurance 229＋fixtures 全绿、fmt 干净、clippy 新增零告警、门禁 `valid: true`（`error_count: 0`；2026-09-15 提交收尾后实测）。入口：[`设计稿 §3.4.1／§5／§8`](docs/CONTEXT_DYNAMIC_SLIDER_DESIGN_2026-09-15.md)。

- [x] **审查修正批（2026-09-15 用户令「请对审查出的全部问题进行处理」放行并当日落码；回执＝[`S1 任务书 §10.8`](docs/CONTEXT_DYNAMIC_SLIDER_S1_IMPLEMENTATION_TASK_2026-09-15.md)）**：① **P1 终止轮语义摘要消费缺口**——`pending_semantic` 此前只有 loop-top 一个消费点（终答候选／预算耗尽／IPG 截停三处 break 静默丢弃，无事件无 marker）⇒ 补 **run 尾安全网**同形语义压缩（压得动落 `mode=model_summary`＋移出被压区＋存档；压不动如实 NoOp），钉子 `final_answer_semantic_summary_is_consumed_at_the_run_tail`（临时停用该段 ⇒ 断言 0 vs 1 失败，回归检出已实证）；② **P2 压缩窗口上传上限守卫（fail-soft）**——窗口轮 `messages` 全量上传无上限（越 provider 窗口即硬失败）⇒ 新增 `window_upload_cap_tokens`（默认 **1.10M**；env `ORZ_CONTEXT_SCALE_WINDOW_CAP_TOKENS`）＋越线**不开窗**（降级块＋`anomaly=window_upload_over_cap`＋同迭代机械强制压缩 `reason=context_scale`；950K 档的强制开窗同受约束），钉子 `window_over_upload_cap_degrades_instead_of_opening_a_window`；③ **P1 恢复面改稿（按实现保留）**——marker 实为 **restore-retained** ⇒ 语义摘要随侧车跨恢复留存（原表述写反）；④ **P2 定位指针载体口径收窄**——逐字原文权威载体＝journal（被压区 drain 后不在 messages／sidecar；存档只存摘要与指针）；⑤ 其余：增量归档「同尺」口径澄清、收尾／检索 journal 跨度注释更正、950K 停手措辞对齐实现、语义轨 `retained_rounds` 按驻留带如实报、台账 `[seq]` 并发边界注明。**读数**：orz-loop **814/0/3**、orz-host **325/0/5**（串行）、orz-assurance **229**＋fixtures 全绿、fmt 干净、clippy 新增零告警、`runtime/tests` 361/1（既有无关红灯）、门禁 `valid: true`（`error_count: 0`；2026-09-15 提交收尾后实测）。入口：[`设计稿 §3.4.1／§3.5.1`](docs/CONTEXT_DYNAMIC_SLIDER_DESIGN_2026-09-15.md)。

- [x] **S1 修订批（v7 裁决，2026-09-15 用户令「开始进行批次 A S1 部分」放行并当日落码；清单与验收线见 [`S1 任务书 §10.6`](docs/CONTEXT_DYNAMIC_SLIDER_S1_IMPLEMENTATION_TASK_2026-09-15.md)，执行回执见 **§10.7**）**：① **压缩分工落地**（机械＝结构化轨**照常 `drain` 工具／命令类内容并作用于上下文**——留台账摘要行＋指针＋compaction 存档＋事件；模型＝语义轨：`[SEMANTIC_SUMMARY]` 摘要块替换被压区，`mode=model_summary`；对象边界＝滑块之外，未折叠降级改以驻留带 L 为界）；② **500K 降为纯提醒**（不打断/不开窗/可延后）、**900K 改强制压缩窗口**（窗口轮绕过折叠装载「滑块之外的携带内容」＋滑块，由模型产出**语义摘要**）；③ `context_compressed.mode` 增 **`model_summary`**、`reason` 增 **`model_selected`**（schema／Python 镜像／Rust 法官三处同步）；④ 950K 兜底加 **once＋冷却＋停手**（压得动不停手；压不动分「语义层残留 ⇒ anomaly＋强制开窗」与「无可压内容 ⇒ anomaly 停手」）；⑤ 提醒水位改**会话级**（`StoredConversation.context_scale_notified`，与黑板同族、新会话独立）；⑥ 压缩 marker 补**四项原文定位指针**（compaction 路径＋digest／台账 `[seq]` 区间／journal run+sequence／sidecar 路径；机械与语义 marker 共用）；⑦ 成本判据改「**不高于 08-19 前形态**」（×1.6 降参考线）。附带修 `[模型参与压缩…]` 注入前缀注册（0AE 遗留缺口）。零新增工具面。**读数**：orz-loop **811/0/3**、orz-host **325/0/5**（串行；并行时进程树类钉子 F-1 抖动）、orz-assurance 全绿（含 Rust↔Python parity）、fmt/clippy 干净（新增零告警）、`runtime/tests` 361/1（唯一失败＝既有无关红灯 `test_v02_all_51_event_types_covered`）、门禁 `valid: true`（`error_count: 0`；2026-09-15 提交收尾后实测）。入口：[`设计稿 v7.1 §3.4.1`](docs/CONTEXT_DYNAMIC_SLIDER_DESIGN_2026-09-15.md) / [`任务书 §10.7`](docs/CONTEXT_DYNAMIC_SLIDER_S1_IMPLEMENTATION_TASK_2026-09-15.md)。

- [x] **S1 实施批**（2026-09-15 落码；按 [`S1 任务书`](docs/CONTEXT_DYNAMIC_SLIDER_S1_IMPLEMENTATION_TASK_2026-09-15.md) A1–A8）：常驻滑窗内核（`compact.rs` H=160K／L=64K 两参数、驱逐深度 H−L、段边界整轮对齐、指针字节级固定）／台账续写照旧（零改动）／首次驱逐一次性固化提醒（`context_scale.rs`，**三量尺已对齐并逐处写明**）／D2 注意力阶梯整体下线（`attention_ladder.rs` 整档退役＋`ORZ_LADDER_*` 五 env 随删）／实际上下文 500K・900K 两级提醒（带实际读数、每级一次/run）／D3 开窗量尺改挂实际上下文＋模型自选（reason 更名 `context_scale_window`）／机械压缩兜底上调（rhythm＝H＋缓冲；硬兜底＝实际上下文 ≥950K 强制一次）＋同步存档三键与归档包信封扩展（`acp_server.rs`）。**读数**：orz-loop 802/0/3、orz-host acp_server 46/0、clippy 新代码零告警、fmt 干净、门禁 `valid: true`（`error_count: 0`；2026-09-15 提交收尾后实测）。**回执与未核项**见[任务书 §10](docs/CONTEXT_DYNAMIC_SLIDER_S1_IMPLEMENTATION_TASK_2026-09-15.md)。入口：[设计稿 §3](docs/CONTEXT_DYNAMIC_SLIDER_DESIGN_2026-09-15.md) / [BACKLOG 0ah](docs/BACKLOG_AND_PRIORITIES.md)。
- [x] **狗粮考核测试（原 A/B；2026-09-16 用户裁决改写：工程化任务不做严格 A/B 采样）**（题＝`0ai` 重文件拆分）：**已完成并收口（2026-09-16）**——run **`RUN-CLI-6aa999d6`**（46m47s、234 轮、2097 事件、无墙钟、0.5.2 载体、ACAF 票据 188/188）；读数：命中率 **96.70%** ✓、5 窗驱逐（折叠后 view 21–29K）、500K 纯提醒如实、零 offset 续读、**存档三键 N/A**（→ 0ak）；单轮如实标注、不作架构结论。读数全文：[`0AI_DOGFOOD_ASSESSMENT_CLOSURE_2026-09-16`](docs/audits/0AI_DOGFOOD_ASSESSMENT_CLOSURE_2026-09-16.md)。
- [ ] **S2 裁决**（gate 后置）：逐字分页档案三形态（① 复用落盘＋回读指针＋`read_file` 分页，零新工具，**优先**／② 新工具 `ledger_read`，8 工具面第二例外／③ 不做）；判读使用率前须先核「档案确有逐字内容」前提，否则记不可判读。入口：设计稿 §4。
- [ ] **尾批＝块轴**（独立批，排最后）：`LedgerFoldState` 轴表（5 ⇒ 6 字段）＋ `DomainSpike.round` ＋ 台账行扩列（LIF 轮/域/双时间）＋ 段表同轴；不新增工具，沿用既有域段书签＋`domain`/`round_from`/`round_to` 词汇；契约触碰（08-18 行格式部分翻转、侧车 schema 扩展）随本批登记。入口：[设计稿 §7](docs/CONTEXT_DYNAMIC_SLIDER_DESIGN_2026-09-15.md)。
- [x] **登记与翻转落账**（随 S1 落码批，2026-09-15）：已落 ①＋②——① `FUS-LEDGER-FOLD-STATE` 验收线按四条新口径改写（旧两条 `ORZ_FOLD_*` env 退役）；② 0ae D2 下线（`AUTH-CONTEXT-SOFT-GATE` 条目改写＋kind 保留仅供历史回放）。③（若采 S2 即重开外挂件 §3 相关面）随 ③ S2 裁决决定。入口：[S1 任务书 §8](docs/CONTEXT_DYNAMIC_SLIDER_S1_IMPLEMENTATION_TASK_2026-09-15.md)。
- [x] **审查 R-12 余项处置批**（2026-09-16，随 v8 更正批，**未提交**）：① **阶梯批量触发已修**——单轮暴涨**一轮内只注入最高档**（低档水位／事件照记，`form=suppressed_superseded_by_higher_tier`／`deferred_to_truncation_notice`）；T1 同轮只发截断告知块并**自带截断后读数**；新钉子 2 条。② **`.gsa` 回放窗口裁定不扩白名单**（内部区走两段门：通知后二读放行、`access_state.json` 卷级持久化 ⇒ 后续读直接放行）；③ **0.77 换算与五项真机读数待真机**（实施回执 §9 给离线取数配方，不拍数）。读数：orz-loop 807/0/3、orz-host 串行 328/0/5、fmt 干净、clippy 50（持平）、契约面零改动。入口：[`实施回执 §8/§9`](docs/audits/0AH_V8_IMPLEMENTATION_2026-09-16.md) / [`只读审查 §3.7/§9`](docs/audits/0AH_V8_IMPLEMENTATION_REVIEW_2026-09-16.md)。


## P1 — 可并行审计 / 证据
### FUS-COMPONENT-REGISTER（`partial`）

- [ ] 逐 crate/component 采用审计——65 组件全 `audit_required`；从当前代码可达性与 local diff 出发，不得由 crate 名/编译推断采用档位（V11-IMPL-008）。
  - 成熟复用评估（2026-08-16，只读）：部分——审计对象即 65 个 Grok Build/Rust 生态成熟组件，产出即复用裁决（直接复用/薄适配/fork）。
- 入口：[register yaml](upstream/fusion-component-register-v0.1.yaml) / [register schema](upstream/fusion-component-register-v0.1.schema.json)。


### GAP-WINDOWS-EVIDENCE（`partial`）

- [ ] ORZ-WIN-PROC-001/002/003 案例晋级——真实 provenance、脱敏、分类与回归门槛（FUS-DOC-001）。
  - 成熟复用评估（2026-08-16，只读）：明确——可复用 Grok child-tree probe 证据与 GAK AppContainer/Job Object 审计。
- [ ] 建立 Windows 平台兼容性设计文档（ADR-0010 §6/§11.7 目标：`architecture/WINDOWS_PLATFORM_COMPATIBILITY_DESIGN_v0.1.md`）。
  - 成熟复用评估（2026-08-16，只读）：明确——设计输入含既有 Windows 审计/事故材料与 Codex/Grok Windows 行为对照。
- [ ] 建立 `regression/windows/` 与 `.observed-runs/windows/` 路由（自动回归/人工复核入口与 git-ignored 原始运行目录）。
  - 成熟复用评估（2026-08-16，只读）：明确——路由可承接既有 observed runs/探针矩阵先例（Grok probe）。
- 入口：[incidents](docs/incidents/windows/README.md) / [cases](docs/cases/windows/README.md)。


### IMPL-DEEPSEEK-TRANSPORT + SEC-CREDENTIALS（`partial`）

- [x] transport/retry/thinking 主/子代理同构复核（2026-08-16 闭合）：三实例共享单一 `DeepSeekTransport`（ModelConfig/RetryPolicy/ThinkingMode::EnabledMax 单一来源）、`REQUEST_MAX_TOKENS=160_000` 单一常量、请求级 thinking 覆盖仅 `-p` 预检轮（F-07 文档化例外）；边界=压缩摘要/预检轮为 loop 外辅助请求（非同构范畴）、契约 §2.1 旧别名拒绝与 /models 预检未实现（另行跟踪）。
- [ ] DeepSeek live 通道与 Windows 实机晋级证据（ADR-0010 §11.7；当前仍为 offline / 构建时 evidence）。
  - 成熟复用评估（2026-08-16，只读）：部分——DeepSeek 官方 API/文档为成熟参照；主要工作是证据收集而非实现复用。
- 入口：[ADR-0007](adr/ADR-0007-transport-retry-policy.md) / [DEEPSEEK_ADAPTER_CONTRACT](architecture/DEEPSEEK_ADAPTER_CONTRACT_v0.1.md)。


### ORZ-SESSION-CONTEXT-MONITOR（`approved`；2026-08-16 度量重定）

- [x] 压缩恢复预检估算校准（chars/2 中文低估）：2026-08-16 用户裁决度量改次数制后废止，不再依赖 token 估算口径。
- [ ] 度量接线：监听 `context_compressed` 事件（reason=rhythm/fallback）累计会话内压缩次数；`session_end` 不计；同一次压缩只计一次。
  - 成熟复用评估（2026-08-16，只读）：有——复用既有 `context_compressed` v0.2 事件面，无新 Schema。
- [ ] 阈值配置（env/TOML，默认 2 次提醒 / 3 次总结推荐）；同一阈值只触发一次。
  - 成熟复用评估（2026-08-16，只读）：部分——2/3 为旧 384K/500K 语义对应，默认待校准。
- [ ] 最简实现：阈值到达的最后一轮模型输出末尾机械附言（附压缩次数）；headless/自动化仅写日志；3 次附五段模板 + 新窗口开场提示骨架。
  - 成熟复用评估（2026-08-16，只读）：部分——3 次推荐复用压缩五段模板（Grok compaction 血统）。
- [ ] 测试（到达/未到达、session_end 不计、一次一计、headless 分支、幂等）+ 实施审计 + BACKLOG/TODO/索引状态同步。
  - 成熟复用评估（2026-08-16，只读）：无——自有测试/审计工作。
- 入口：[设计](docs/SESSION_CONTEXT_MONITOR_DESIGN_2026-08-14.md) / ADR-0010 §14.13/§14.18 / BACKLOG 6d。


### 0aa 历史卷 journal 全量 verifier 复扫（P1；2026-09-13 立项，深审附带建议①）

- [ ] 卷清单先行（工作区 `.gsa/` + `jobs-official` 等评测批次 + 存档卷，覆盖率判据 100%）→ `journal-conformance` 全量复扫 → 断链分布审计落档（按批次/时间窗/事件族分桶，区分脱敏改写断链 vs 墙钟杀死接缝断链）→ `recover_torn_journal` 适用性只读评估。边界：只读、不改写历史卷、不重跑 run。入口：[深审 §2 P0-2](docs/audits/FULL_PROJECT_DEEP_REVIEW_2026-09-12.md) / [BACKLOG 0aa](docs/BACKLOG_AND_PRIORITIES.md)。


### 0aj 黑板写权限层放行（`blackboard_write` ReadOnly deny；P1；2026-09-16 立项，0ai 考核测出，**旧摩擦残余**）

- [ ] 权限桥对 `blackboard_write`（ReadOnly 类）按设计自动放行（各策略面核对；现状＝每次调用 `permission_decision{deny}`，journal 实证 run `RUN-CLI-6aa999d6`）；探针注册面补声明（现与请求面脱同步）；`plan_write` 事件族端到端钉子。
- [x] **判据面满足（2026-09-17，run `RUN-CLI-6aaad7c8`）**：`blackboard_write` ×8 全 `allow_once`（全 run 205/205 零拒绝）→ `plan_write` ×8 一一对应 → 写后 `blackboard_read(section=plan)` exit 0；**闭合入账待用户裁决**。入口：[BACKLOG 0aj](docs/BACKLOG_AND_PRIORITIES.md) / [`0AM_DOGFOOD_CLOSURE` §3](docs/audits/0AM_DOGFOOD_CLOSURE_2026-09-17.md)。
- [x] **落码（2026-09-16，orz `12396e6a`）**：权限桥 `access_kind` 补 `blackboard_write` 内存类 arm（`Read(None)` 自动放行）＋探针面 `WORK_TOOLS` 23 → 24 三处同批（orz-loop `tool_probe`／orz-assurance `families`／Python `_WORK_TOOLS`，判据 `probe_storage`）＋三钉（表级断言＝修复前实跑红／跨表护栏样本补齐＝漏网直接原因／端到端链：调用 → `tool_completed{exit_code 0}` → `plan_write` 出账 → `blackboard_read` 读回一致）。读数 orz-loop 819/0/3、orz-host 串行 325/0/5、orz-assurance 229、fmt 干净、clippy 52→52 零新增。**判据行维持未勾**（要求无头 run 内实证；用户 2026-09-16 指示暂不开始新狗粮线）。入口：[`0aj/0al 修复与 0.5.3 载体` §1](docs/audits/0AJ_0AL_FRICTION_FIX_AND_053_CARRIER_2026-09-16.md)。
- [x] **复核处理（2026-09-16，用户指示「全面检查设计/实现/符合性」＋「处理全部问题」）**：跨表护栏改**单一源 ＋ 遍历式**——控制器侧新增 `ToolDispatcher::READ_ONLY_EXEMPT_TOOLS` 并驱动 `risk_class`，宿主护栏遍历该表且断言「代表参数表恰好覆盖单一源」（漏补参数同样报红）；新增**声明面分类护栏**（声明面工具必须 ∈ 工作工具 ∪ 规则式非工作族；含 `RUN-CLI-6aa999d6` 7 件声明面冻结样本）；**归因补正**（漏网两因＝样本表漏列 ＋ 0ae 深审 §4.1「orz-host 无需改动」误判，原报告已就地更正）；例次口径统一为**同形第四例**；`families.rs` 过期计数改派生式；fixture 生成器两处 payload ＋ 3 件生成物补 `blackboard_write`（逐件 SHA256 与生成器新输出全部 MATCH）。读数：orz-loop 821/0/3、orz-host `permission::tests` 20/20。**判据行仍维持未勾**。**同日追补（O1 处置）**：0z 资源族＋0ac 检索投递族 10 事件表项＋8 个追加正负例并入生成器、28 件手工 fixture 排版归一化，**重跑生成器零差异**（347 件 SHA256 全等）。入口：[`0aj/0al 独立复核与问题处理`](docs/audits/0AJ_0AL_REVIEW_HANDLING_2026-09-16.md)。


### 0al 门禁冻结克隆树漂移（`check_repository` 导入原始树；P1；2026-09-16 立项，0ai 考核测出）

- [ ] 门禁 ROOT 按脚本位置显式锚定（禁依赖 site-packages 可编辑安装；现状＝克隆内按文档口径调用会崩或静默校验错误的树）；补「克隆内校验克隆树」钉子。
- [ ] 判据：**冻结克隆＝整树复制（保留未入库工作件；git 派生克隆不适用——门禁链接检查依赖未入库工作件，必然红）**，克隆内 `python scripts/check_repository.py` 校验克隆自身且与 `-m` 形态读数一致。入口：[BACKLOG 0al](docs/BACKLOG_AND_PRIORITIES.md) / [`0AI_DOGFOOD_ASSESSMENT_CLOSURE` §3.3](docs/audits/0AI_DOGFOOD_ASSESSMENT_CLOSURE_2026-09-16.md)。
- [x] **落码（2026-09-16，父仓）**：`sys.path` 首位锚定脚本推导 `ROOT`（禁可编辑安装遮蔽）＋ `_check_reference_root_anchor` fail-closed（来源树 ≠ 本树即 `valid: false`）＋钉子 `assurance/tests/test_gate_root_anchor_nails.py` 4 例（本树锚定／reference 归属／外来模块负例／克隆形态自证）。实测对照：修复前模拟克隆内 `reference_module_root = D:\CLI`（错树）→ 修复后＝克隆自身；门禁 `valid: true`、新钉 4/4。**判据行维持未勾**（真克隆内复验待日后 run）。入口：[`0aj/0al 修复与 0.5.3 载体` §2](docs/audits/0AJ_0AL_FRICTION_FIX_AND_053_CARRIER_2026-09-16.md)。
- [x] **复核处理（2026-09-16，用户指示「处理全部问题」）**：锚定判据 **fail-fast**（错误树立即收口返回，杜绝 `main()` 异常路径吞掉 `gate would validate the wrong tree` 诊断）＋锚定按**解析后路径**比较（`-m` 形态不再重复插入同树条目）＋非仓库内容排除前缀补 `.tmp`（此前 `.tmp*` 草稿目录被当仓库内容，实测可把门禁打成 `valid: false`）；钉子 4 → **7 例**（+B2 诊断保留／+B3 草稿排除／+B4 锚定幂等）。**真实克隆形态 A/B 取证**：修复前脚本在真克隆内逐字同形崩（`ValueError: … is not in the subpath of '<克隆根>'`），修复后不崩、无错误树，余 8 条错全为克隆既有事项（TER 失效链接 ×7＝既有摩擦 F6 ＋ 复刻剔除 `.git` 致 orz 子模块清单不可列 1 条）。判据行仍维持未勾（真克隆整链读数待日后 run）。入口：[`0aj/0al 独立复核与问题处理`](docs/audits/0AJ_0AL_REVIEW_HANDLING_2026-09-16.md)。


### 0am LIF 动力学升级线（P1 轮次预算换算先行＋RLI 谐振漏积分基座影子并行与观测判据预注册；P1；2026-09-16 立项）

- [ ] S1 Part A：T̂→墙钟↔轮次换算面（resident 任务状态行＋SESSION 面渲染；1-2-5 阶梯保守取整、永不高于真值、不阻断、fail-soft；零契约面）；A1 方向安全／A2 桶保真钉子。
- [ ] S2 RLI 影子并行（**2026-09-20 二次裁定后口径**）：二阶欠阻尼谐振通道族旁路影子（u/v 闭式精确更新、解析包络、节律计数＋分位数自校准阈值；生产 1D 不动；env 门控；状态入侧车）＋**改造四项**：① **域判定接上**（LIF 域状态机职能平移到 RLI——「LIF 变体」名分，RLI 自判动作域）／② **预测步长改 10**（闭式自由演化解对 Δt=10·T̂ 前推，不引入迭代）／③ **零注入 PULL 参考面**（模型可自行拉取；否则可用性无从观测）／④ **评估件重写**（`rli_shadow_replay.rs` 标签回路退役）。不动面＝核／env 门控／侧车／生产 1D。
- [ ] S3 回放对照（**2026-09-20 二次裁定后口径**）：**标签类与锚点类口径整体退役**（原 Q1 分离 AUC≥0.70／Q2 增量 ΔAUC≥0.05／Q3 及时 lead≥3 轮作历史读数保留、不再作转正判据）；判据＝观测**实际可用性**，三条**零标签**读数——① **消费率**（模型是否真的拉取 RLI 信号）／② **转向相关**（转向与该信号是否相关）／③ **域一致性**（RLI 自判动作域与框架实际动作结果是否同面一致）；C1–C5 证伪门保留。（口径注：`jobs-official` 为滚动集——2026-09-17 实测 134 runs、2026-09-20 实测 137 runs。）
- [ ] S4 真实任务摩擦探针一轮（0ai 先例：单轮如实标注、不作架构结论）；翻转裁决（替换或维持 1D，含参数纪律 ADR 修订）留用户。
- [x] **载体 0.6.6 双平台重建换装与发行（2026-09-21 用户令「请先重建，随后做提交与推送吧，再次重建以后版本是不是就到 0.6.6 了，请一并发布双平台包吧」；见 [`0.6.6 重建与发行档`](docs/audits/066_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-21.md)）**：① **预检重建**（未提交树、不换装）——Windows 增量 269 s／Linux Docker 717 s 双平台 exit 0，先验后提交（0bc §8 点名的 Linux cfg 面）；② orz **`1ecbcaa3`**（0bc 批 17 文件）→ **`6efd192f`**（bump 0.6.5→0.6.6）推 `cli`；③ Windows clean 全量 **15m02s**（clean 37,286 文件／26.2 GiB）／Linux musl 暖缓存 **5m53s**、三件 PT_INTERP=0、bookworm·alpine 双向冒烟 exit 1；换装六件（`.0.6.5-bak` 链）＋ACAF 重 provision（keystore 逐位未动、manifest↔signer 逐位一致）＋`dogfood_launch -DryRun` 断言过；④ **字面量核证**：0bc 面首次进件（`ORZ_JOB_CPU_RATE_PERCENT`／`ORZ_JOB_ACTIVE_PROCESS_LIMIT` 0→1、`commit_notification` 0→2/0→3、`资源软提示` 1→2、FR-3 文案 0→1、`NFC/NFD` 0→1），0am 面与既有面保持（Windows `context_compress`／`blackboard_write` −2＝FR-3 删窗口收窄提示行的预期；`slow_prog`／`fast_prog` 7→12＝FR-5 单源物化）；⑤ 父仓 pin＋`orz_source_manifest.sha256` **1461** 条（差异恰 38 行）；⑥ **GitHub Release `v0.6.6`** 双平台包（zip 26,842,565／tar.gz 35,840,358）＋`SHA256SUMS`，容器内 `sha256sum -c` 全 OK、解包回读 6/6 MATCH。**计数不变（45；0bc S3 资源面读数待下一轮真机收取）。**
- [x] **载体 0.6.5 双平台重建与换装（2026-09-21 用户令「请进行提交与推送，并重建吧」；见 [`0.6.5 重建档`](docs/audits/065_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-21.md)）**：**0am 全线首次进件载体**。orz `b8789259` → **`e3bf357c`**（bump 0.6.4→0.6.5）推 `cli`；**构建根＝原地**（源树已干净，免 064 的 worktree 镜像）：Windows clean 全量 **14m40s**／exit 0（`cargo clean` 21,812 文件／11.8 GiB）、Linux musl Docker **12m24s**／exit 0（三件 PT_INTERP=0、bookworm·alpine 双向冒烟 exit 1 形态）；换装六件（`.0.6.4-bak` 链）＋ACAF 重 provision（keystore 逐位未动、manifest↔signer 逐位一致）＋`dogfood_launch -DryRun` 断言过；**字面量核证**：`ORZ_LIF_RLI_SHADOW` 0→**7/7**、`slow_prog`／`fast_prog` 0→7/8、`rli.history` 0→2/2、`WALLCLOCK_REMAINING_ROUNDS` 0→1/1，既有面保持；`[EARLY_DELIVERY]` Windows 1→0 判为跨批物化摆动（不改码）；父仓 pin＋manifest **1461** 条（差异恰 6 行）。**未发 Release**。**同批裁决：RLI 维持影子组件（默认关），下一步裁决待 0bc 实测后。**
- [ ] **0am 模态分离批（2026-09-20 用户裁决；报告 [`0AM_RLI_MODE_SPLIT`](docs/audits/0AM_RLI_MODE_SPLIT_2026-09-20.md)；已随 0.6.5 批提交推送）**：裁决＝**读数层拆分做／形态层补振荡不做**（`E ≡ u` 的机制已在报告 §1 证为恒等式）＋**追加裁决＝采样粒度改事件级／`slow_prog`·`fast_prog` 保留／`env_prog` 撤名**＋**域级定位面（用户令「LIF 组件的作用是外挂时间轴…RLI 得配上域级判断部分来方便模型进行进度定位和回看」）**。已落码——锚点面十名 → **十一名**（＋`slow_prog`／`fast_prog`（实极点分支专用、复极点返回 `NAN`），−`env_prog`（与 `u_prog` 恒等））／采样点＝**决策轮 ＋ 每个工具事件**（判决粒度不变：自判域仍只在决策轮记录）／`blackboard_read` 的 `name` enum＋描述串同步／回放件 `prog_predicate_contrast` 段（两判据核读＋模态间隔统计的轮／事件两栏）／**域级定位面**（`RliDomainSpike` 带 `round`（`Option`，旧侧车 `None`＝FR7 口径）＋`RliDomainMachine::segments()` 推导 `from→to`／驻留／recovery＋`rli.history` 改段表、**全 run 无窗口**；用户裁决「RLI 与 LIF 域语义**不必同格**」——只借信息形态、不借读数）。**读数**：① 两判据**完全等价**（一致率 0.8221、主张 599、**翻转 0 次**）⇒ **换量无判据增量，判据维持 `u_prog`**；② 第二轴分辨力＝采样粒度的函数——轮粒度 `|c_slow−u|` 均值 0.00427／>0.01 占 12.5% ⇄ 事件粒度均值 **0.02316**／>0.01 占 **85.1%**／>0.05 占 11.1%（快模态在注入当刻最大）⇒ 形态层「不做」维持、但理由改准（瓶颈是采样粒度，分辨力已由事件级采样解决）。**回归自检**与 v3 一致（0.9557／0.8221／0.8060）。**待办**＝**影子启用面裁决**（默认关，`ORZ_LIF_RLI_SHADOW=1`；**已裁：维持影子，下一步裁决待 0bc 实测后**）。**计数不变（0am 线内延续）**。
- [ ] **0am 改造批落码（已完成 2026-09-20，狗粮轮 `RUN-CLI-6aafe5d8`、载体 0.6.4、exit 0；随提交批入账）**：五项全部落码——配极改（`prog` 实极点 ζ=2.0＋过阻尼双曲闭式分支）／判断标准改写（压力轴＝水平阈 ∨ `v`/`E` 原生覆盖、`stuck` 不补通道、**撤销与 LIF 现域对照**）／C3 分离（衰减列＋频率列）／`feature` 序列面（`blackboard_read section=rli selector=feature`）／FR4（白名单并入 `context_compress`，保 10 工具面）。读数：压力轴 95.6%（保守欠报 26 主张 vs 265 实际）／进度轴 82.2%／joint 80.6%；C3 衰减为主效应（0.247 vs 频率 0.03–0.06）；`prog` run 均 u 0.519→0.734。**FR-8 主会话已处置**（C3 补 `prog` 实极点列，恒等点全 0；新观察＝过阻尼分支 `E ≡ u`、a3 不独立，待复核）。入口：[`0AM_RLI_RETROFIT_REMAINDER`](docs/audits/0AM_RLI_RETROFIT_REMAINDER_2026-09-20.md)。
- [ ] **下一批待办与裁决登记（2026-09-20 用户令；四项已裁、可开工）**：① **配极改＝批准**（数学形式不整体换——按设计 §11 极点配置表**按通道语义分配极点**：速率／压力类保留复极点（预期＋节律），水平／新鲜度类走实极点（持久／适应））／② **判断标准改写＝批准**（**不补** `stuck` 通道——按设计 §10.3 `v`／`E` 锚点原生覆盖 stuck 语义＋§9 谓词重推导）；**RLI 不再与 LIF 对照**：LIF 维持现役组件，RLI 无实际作用性则用已跑通的 LIF、有作用再替换／③ **C3 直接明确分开**（固定 ω_d 只扫 ζ，把衰减与频率两效应拆开；现口径降格标注为「参数非惰性检验」）／④ **`feature` 序列面要补**（PULL 面增锚点**序列**面）／⑤ **FR4 形态已定**＝压缩白名单**挂在 `context_compress` 下**（模型处理压缩时可额外保存白名单文件；**不做独立工具**、保 10 工具面）／⑥ **`prog` 先取活体读数再定**（随 0bc）／⑦ **FR3 更新 F14 状态**＝「条件性可构建」／⑧ **FR5 留观察**。
- **0am 线整体挂起（2026-09-17 用户指示「LIF 组件暂时不做，留给狗粮线」）**：上述未勾各项与随挂起的 B 类六条摩擦（FR-B01–B06）**一并挂起，以 O2 裁决为前置门**；本批零实施，orz 工作树在场的 0am 批保持未提交，合回口径留裁决。入口：[`摩擦盘点 §3/§7`](docs/audits/FRICTION_INVENTORY_2026-09-17.md)。
- **方向二次裁定＋执行顺序（2026-09-20 用户令）**：① **标签类与锚点类口径整体退役**——LIF 提取特征本就不用标签；RLI 是 **LIF 变体**（LIF 能做的它本身也能做，含**自行判断动作域**），**无需外部标签、亦无需人工设定提取规则**；② 预测／引导参考仅为**尝试方向**，判据面＝观测**实际可用性**——原 Q1–Q3（分离 AUC≥0.70／增量 ΔAUC≥0.05／及时 lead≥3 轮）与标签锚点一并**退役**（S3 行判据按本注重定）；③ 预测时间步先定 **10**；④ **顺序＝先改好 RLI → 载体重建 → 进 0bc**（0bc 本身即 RLI 效果观测任务）。入口：[`0am 报告裁定补注`](docs/audits/0AM_DECODER_FORM_REPLAY_2026-09-20.md)。
- 入口：[重文件拆分勘察](docs/audits/HEAVY_FILE_SPLIT_SURVEY_2026-09-13.md) / [BACKLOG 0ai](docs/BACKLOG_AND_PRIORITIES.md) / [索引 `GAP-HEAVY-FILE-SPLIT`](CLI_PROJECT_INDEX.md)。
- 入口：[重文件拆分勘察](docs/audits/HEAVY_FILE_SPLIT_SURVEY_2026-09-13.md) / [BACKLOG 0ai](docs/BACKLOG_AND_PRIORITIES.md) / [索引 `GAP-HEAVY-FILE-SPLIT`](CLI_PROJECT_INDEX.md)。


### 摩擦盘点 全仓未处理摩擦项（2026-09-17；**不计入未闭合总数**）

- [ ] **盘点落档（已落）**：36 条未处理摩擦/挂账（A 14／B 6／C 8／D 4／E 4）＋注记级 1（`blackboard_write` 字面双副本）＋F 类 7／G 类 4 单列引用；全文见 [`FRICTION_INVENTORY_2026-09-17`](docs/audits/FRICTION_INVENTORY_2026-09-17.md)，路由见 [BACKLOG 摩擦盘点](docs/BACKLOG_AND_PRIORITIES.md) 与索引 `OBS-FRICTION-INVENTORY`。
- [x] **判定批注（2026-09-17 用户）**：不处理 4（FR-A01/A02/A05/A09）／FR-C05 核实销项闭合／FR-C04 采②分 run 标注**已落码并提交**（orz `26dcce1b`：`EditRecord.run` 章＋D4 分组渲染＋3 钉）。
- [x] **判需处理 30 条＋注记级 1**（2026-09-18 用户放行「摩擦处理批次并作为狗粮线」后**已部分执行**，run `RUN-CLI-6aac0af5`，orz `07405e61`＋父仓同批）：**落地闭合** FR-A07（修复＋钉）／FR-A08（归档＋链接改写）／注记级（常量去重）／FR-A12／FR-A13／FR-A14（启动器脚本）；**落地** FR-D01／FR-D03（清单/配方）；**销项候选** FR-E01/E02/C08、**降级候选** FR-A11（根因＝预置 `GROK_HOME`）、**口径** FR-A10 串行固定、FR-A03 取证收口不修码、FR-A04 留复核；**维持登记** B 类 6（O2 门）／C01–C03/C06/C07／D02/D04／E03/E04；**新摩擦 FR-N02/N03/N04** 登记（orz 内 agent 自主发现），N02/N04 入案例库。逐项批注见[盘点档 §12](docs/audits/FRICTION_INVENTORY_2026-09-17.md)；报告 [`FRICTION_INVENTORY_TREATMENT_2026-09-17`](docs/audits/FRICTION_INVENTORY_TREATMENT_2026-09-17.md)。
- [x] **边界**：非 F-001 时代摩擦自报台账（`docs/FRICTION_LEDGER.md`，2026-09-14 删除）的复辟——不新增摩擦项记录文档、不重启自报管线；逐项处置后在对应行补批注，全部处置完毕整档转 `historical` 并入 `存档/`。


### 0ap FR-A06 压缩交互设计批（P2；2026-09-18 用户令纳入排期；处理批报告 §1.7 注册）

- [x] **设计评估稿（2026-09-18 落档）**：[`COMPRESSION_INTERACTION_NINTH_TOOL_DESIGN_2026-09-18`](docs/COMPRESSION_INTERACTION_NINTH_TOOL_DESIGN_2026-09-18.md)——主案＝常驻第九工具 `context_compress`（发起 D3 压缩＋响应自带滑块读数；摘要通道照旧 `[SEMANTIC_SUMMARY]`，禁第二套格式）；九点位注册清单（0aj 教训硬防）＋契约面零改动＋S0 回查五项＋S1 四钉判据。用户裁决三点已录（常驻 8→9／计数口径／同步面）；尾徽标已否决。
- [x] **用户审稿通过（2026-09-18，设计定稿转 current-design；工具名终版 `context_compress` 用户定名，其余无异议）**。
- [x] **S0 回查＋S1 四钉＋机制落码（2026-09-18 过夜批，工作树未提交；用户放行「0ap 全部含工具注册，不重建」）**：九点位全落＋D3 开窗接线（请求位/在程位原子＋loop-top 消费锁存顺延＋防抖三态）＋滑块读数表（读时现算）＋blackboard_read 头增段＋归因分流（工具窗口 ⇒ `model_selected`）＋子代理面剔除（main-lane only）＋S0 发现四项（Python 镜像表格数据 +1＝probe-partition 校验必需、规则零改动等，见回执 §2）；ADR-0010 §14.72 / v1.73 随批转录。读数：orz-loop **794/0/3**（基线 791＋钉 3）、orz-assurance **246/0**、orz-host 串行 **332/0/5**、clippy 逐位持平、fmt 净。
- [x] **S2 载体重建换装＋字面量核证（2026-09-18 用户令「请进行重建吧，双平台」＋「请提交并推送吧，双平台安装包也发布上去」）**：0.6.2 双平台三件套（Windows clean 全量重建 10m36s／Linux musl static-pie 18m51s）先备份后换装逐对 MATCH＋ACAF 按 052/061 先例重 provision＋字面量核证双平台同表；随提交批入库（orz `08ab194c`）并发布 GitHub **Release v0.6.2** 双平台包。详见 [`062 载体重建`](docs/audits/062_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-18.md)。
- [ ] **S3 狗粮实证（余项）**：单轮如实标注——读数表**括号内总数**与 journal `blocks=N` 一致〔主数与 `blocks=N` 不同量——主数＝可压缩子集 `closed ∧ Live`，只作 advisory〕／模型经新工具完成至少一次 model_selected 压缩／常驻成本读数。入口：[BACKLOG 0ap](docs/BACKLOG_AND_PRIORITIES.md) ／ [设计稿](docs/COMPRESSION_INTERACTION_NINTH_TOOL_DESIGN_2026-09-18.md) ／ [`实施回执`](docs/audits/0AP_0AO_COMPRESSION_INTERACTION_AND_TOOLNAME_SINGLE_SOURCE_2026-09-18.md)。
- [x] **主会话只读复核＋全部问题处置（2026-09-18）**：P1 并行批读数假值（只读会话视图＋真值钉，修复前实跑红）／P2 归因固定点＋设计稿三态口径＋Python 镜像表格措辞＋fixture 样本同步（生成器重跑仅 3 件按预期变化）／P3 读数对账口径收紧＋import 归位＋`.tmp-*` 入 `.gitignore`；读数 orz-loop 796/0/3、orz-assurance 246/0、orz-host 332/0/5、clippy 逐位持平。入口：[`实施回执 §8`](docs/audits/0AP_0AO_COMPRESSION_INTERACTION_AND_TOOLNAME_SINGLE_SOURCE_2026-09-18.md)。


### 0aq 全项目全面严格审查处置线（P1；2026-09-18 用户令「对本项目进行一轮全面严格审查」；报告 [`FULL_PROJECT_STRICT_REVIEW`](docs/audits/FULL_PROJECT_STRICT_REVIEW_2026-09-18.md)）

- [x] **RS-01（P0）父仓 CI 断流修复（2026-09-18 闭合，run 35338101577 全绿）**：`S4_PROGRESS_2026-09-02.md:1128` 绝对路径链接改相对＋`check_repository.py` `_check_markdown_links` 平台对称化；修后确证 CI 全绿＋CI 状态纳入门禁口径。
- [x] **RS-02（P0→P1）CI 补 Rust 测试轨（2026-09-18 闭合：用户裁决「直接处理」生效；run 35338101577 rust 三连全绿）**：最小 orz-loop／orz-assurance `--lib`＋orz-host 串行；提级与否待裁决。
- [x] **RS-03（P1）920K 模型面残留文案**（2026-09-19 过夜批闭合，明细见 BACKLOG RS-03）：两处按现行 500K 硬截断口径改写＋同串连续空格顺修；`checkpoint.rs:42`／`agent_loop.rs` 注释余留「920K」为历史注记（描述已退役阶梯、非模型面，不动）。
- [ ] **RS-04（P1）测试封闭性**（2026-09-19 过夜批**部分闭合**：①orz-tui 9 例 ignorable 化完成、②orz-host 串行口径文档化完成；③assurance slow-e2e 分层维持开放）。（CI 实证载敏三例已随批修复，明细见 BACKLOG RS-04）
- [ ] **RS-05（P1）锁中毒级联治理＋生产路径 unwrap Top 10**（2026-09-19 过夜批**大头完成**：五热点 144 处去中毒化＋Top-10 六处；余 4 处结构性 unwrap 留渐进，明细见 BACKLOG RS-05／报告 §2-RS-05）。
- [ ] **RS-06（P1）0am 批提交前钉**：libm `=0.2.15`＋example fmt＋P3 五小件（随 O2 裁决批）。
- [ ] **RS-07（P2）高危模块测试补强**（2026-09-19 过夜批**主项闭合**：权限桥跨声明面 25 工具全遍历集成测试完成；余 acaf_e2e 非 Windows 缺口处置决策待裁决）。
- [x] **RS-08（P2）巨石文件拆分补线**（2026-09-19 过夜批闭合）：agent_loop.rs 8,017／controller.rs 6,274 纳入 HEAVY_FILE_SPLIT_SURVEY 候选，登记不立项（拆分留裁决）。
- [x] **RS-09（P2）仓库历史瘦身**（2026-09-19 裁决批闭合＝仅记录、不额外处理）：侦查推翻"滞留 ref"定性——`feat/fusion-architecture` 实为 orz 子仓上游备份分支（单仓库双仓承载）、`codex/np1-body` 为 0y 活跃线；删 ref＝毁 orz 唯一云端备份 ⇒ 现状维持。拓扑修正见[裁决文档 §1](docs/audits/RS_RESIDUAL_AND_FRICTION_F1_F8_RULING_2026-09-19.md)。
- [x] **RS-10（P2）入库日志清理**（2026-09-19 过夜批摘跟踪＋复核批补忽略规则收口）：8 件 `git rm --cached`（文件保留磁盘作本地存档；删除态已暂存随下批生效）；`.gitignore` 补三式作用域规则使磁盘件不再以 `??` 冒头（`git check-ignore` 8/8 命中）；tracked log 归零。
- [x] **RS-11（P2）.gsa/ 磁盘卫生**（2026-09-19 过夜批闭合）：`.gsa/cargo-target`（614MB）迁出至 `D:\tb-eval\orz-cache\cargo-target`；`installation-key.json` 秘密面确认通过（元数据＋DPAPI blob 摘要，非可移植秘密）。
- [x] **RS-12（P2）README 源码构建前置补 protoc**（2026-09-19 过夜批闭合）：父仓＋orz README 双处补 `PROTOC` 前置与「缺件约 20 分钟后才失败」警示。
- [x] **RS-13a（P3，本批当改）BACKLOG P1 总览行 0am 挂起标注**：本批已补。
- [x] **RS-13b（P3）索引 §3.1 GAP-TOOL-BUDGET 120→0 注记**（2026-09-19 过夜批闭合）：索引条目补「TER T1.7 起主车道 0=unlimited、与检索档位取 min」现行为注记。
- [x] **RS-13c（P3）releases/ 0.3.1 断档与命名统一**（2026-09-19 过夜批闭合）：补 `releases/orz-0.3.1-linux-x86_64/README.md` 占位使断档点有意化；后缀两段式登记为历史约定不改。
- [x] **RS-14（P3）.gitignore/.gitattributes 补**（2026-09-19 过夜批闭合）：.gitignore 增 `/.pytest_cache/`、`/.agents/`；.gitattributes 增 `*.exe`／`*.png`／`*.ico` binary 标注。
- [x] **RS-15（P3）一次性产物清理批**（2026-09-19 裁决批闭合＝仅记录、不额外处理）：侦查分层——s4_vm_* 中 ~70 件 consumed 有案例在位引用（LIFECYCLE 明文勿移动）、VM 生命周期件 retained（0l ⑥⑦ 未到）；用户裁决全部维持现状。见[裁决文档 §2](docs/audits/RS_RESIDUAL_AND_FRICTION_F1_F8_RULING_2026-09-19.md)。
- [x] **RS-16（P3）Python 测试面卫生**（2026-09-19 过夜批闭合）：pyproject 死配置删除＋pytest.ini `norecursedirs`（2080 项零错误收集）＋evaluation/regression 占位 README 去误导；顶层裸 `test_*.py` 实测已为零。
- [ ] **RS-17（P3）TODO2 M3 复验启动决策**（T3.1–T3.5 未勾选，明细落后两个载体版本）。**2026-09-19 用户裁决：不急**——维持开放、不排期。
- [ ] **RS-18（P3）巨型文档增长观察**（挂观察，超限随瘦身机制处置）。**2026-09-19 用户裁决：不急**——观察态维持。
- [ ] **RS-19（P2）门禁不含 Python 合约测试轨**（2026-09-28 111 批随批勘误时发现，112 批补登记；非 09-18 审查原 18 项）：`check_repository.py` 门不运行 `pytest runtime/tests/test_run_event_conformance.py`（16 例，0.43s）——合约钉值漂移（094 批 0bw③ 加 `write_control_review` 时 `test_v02_all_event_types_covered` 钉值漏校准 65 vs 枚举 66）自 2026-09-27 起在 HEAD 上红 4 批、穿 v0.7.1–0.8.2 三次发行未拦截；处置候选＝套件纳入门禁或定期核，实施随处置批。
入口：[BACKLOG 0aq](docs/BACKLOG_AND_PRIORITIES.md) ／ [审查报告](docs/audits/FULL_PROJECT_STRICT_REVIEW_2026-09-18.md)。


### 重文件拆分勘察（2026-09-13 落档；候选 1 已立项 0ai（2026-09-16），候选 2/3 未计数）

> 只读扫描 + 车道归属判定，全文与数据见 [`HEAVY_FILE_SPLIT_SURVEY`](docs/audits/HEAVY_FILE_SPLIT_SURVEY_2026-09-13.md)；索引 `AUTH-HEAVY-FILE-SPLIT-SURVEY`。

- [ ] （候选 1，第一优先）`orz-loop/src/host_exec.rs`（9,184 行）沿 CONTROLLER-SPLIT 先例拆分：S1 切分图 → S2 pub(crate) 机械搬移（行为不变）→ S3 回归核验；验收 = 单文件 ≤10,000 + 职责域单一。
- [ ] （候选 2）`orz-loop/src/gateway/transport.rs`（5,846 行）流式/非流式拆分，与深审 P2-7 重试链统一同批闭合。
- [ ] （候选 3）`orz-assurance/src/journal/families.rs`（6,491 行）按 35 事件族分模块，纯机械搬移；可与下一个事件族批同车。
- [x] 休眠/血统车道六超重件（handle.rs / conversation.rs / textarea.rs / manager.rs / servers.rs / queue.rs）判定**不拆**——退役/冻结裁决时一并处置（OBS-PERMISSION-DUAL-IMPL 终局治理视野）。


### P1-0bk 压缩区间解析偏差修正（P1；2026-09-24 用户令立项，BACKLOG 0bk；来源＝[`0bi 报告 §10-⑤`](docs/audits/0BI_FRICTION_CARRYOVER_2026-09-23.md)）

> 用户令「解析偏差的修正摘出来额外做，单独立项」；并裁「**压缩下限这个设计就不加也不留**」（「模型做的是对的但实现是错的」）与「**机械层的压缩是明确设计**……不用改」。⇒ **只修实现面偏差**：不加压缩下限、不加连号抑制、不做工作点标定，也不动机械层压缩产物。

- [x] **S1 勘定**（2026-09-24 同日）：偏差本体与修法定案（`extract_block_selection` **放宽解析**：接受行内注解／括号说明，非法段仍拒；**解析失败如实回报**并给出当前可压区间，禁静默兜底；缺省语义（未指定 ⇒ 最旧一块）维持但如实回报）；**待核已结**＝seq 1007 声明 `1-3` 而实得第 8 块（块号为位置编号，同迭代先截断/压缩再消费摘要会重编 ⇒ 回执对账行如实呈现缓解，机制不改）。
- [x] **S2 落码**（2026-09-24）：实施＋钉子（① 带注解区间解析钉〔`压缩块: 1-62（说明…）` ⇒ `[1..62]`，含全角冒号〕；② 失败回报钉〔未识别 ⇒ 回执写明＋给可压区间＋审计落账，不得静默退化为最旧单块〕；③ 缺省语义钉〔未指定 ⇒ 最旧一块＋marker「区间说明」如实标注〕）。orz-loop 串行 845/0。
- [ ] **S3 真机**：随下次狗粮轮观察压缩次数与实得削减（对照本 run：12 次压缩、6 次实得 ≤16%；载体重建随下批放行）。
- **口径**：**输出面（completion／reasoning）必需、不处理**——成本治理只在输入侧（压缩行为／缓存）；本项不得引入输出侧配额。
- 入口：BACKLOG 0bk / [`0bi 报告 §10`](docs/audits/0BI_FRICTION_CARRYOVER_2026-09-23.md)。


### P1-0bm 狗粮长轮（审查裁决落地批七件＋RLI 观察件，同轮并入 0bj 摩擦承接二轮／0bp 检索形态 B／0bq 进程收口）（P1；2026-09-24 用户令立项并加项，BACKLOG 0bm；来源＝0bl 待裁决八件·用户逐件裁决（主代理全数同意）＋2026-09-24 用户令「0bo 和 0bq 进 0bm」「0bj 融进 0bm」；**执行形态＝狗粮长轮（不直接处理）**；裁决记录＝审计档 `docs/audits/072_FULL_REVIEW_REMEDIATION_2026-09-24.md` §7）

> **加项裁决（2026-09-24）**：用户令「0bn 是单独进行的内容，不进狗粮轮，是 0bo 和 0bq 进 0bm」——本项即狗粮长轮本体（保留 0bm 号），**0bn（必定压缩复审补口）单独进行、不进轮**；**同日再令「0bj 融进 0bm」＝0bj 全件并轮同载**；**2026-09-25 用户令再并入一件**＝`orz web` 启动文案在 GBK 控制台显示乱码（「这个问题也并入 0bm 吧」，归 0bj ④ 输出编码链一族，随轮处理、不新设计数）。**本轮任务＝七件＋RLI 观察件＋0bj（摩擦承接二轮六件）＋0bp（检索形态改指浏览器车道）＋0bq（进程收口兜底）**。前置建议（下发时定）＝重建载体纳入**全部未提交落码批**（0bi 四件＋0bl 十件＋0bk S2＋必定压缩补足批），使轮内编辑面／取消面／压缩面改动自举可用。

- [ ] **0bm 七件（S1 勘定→S2 落码→S4 收口）**：① ADR-0010 分卷拆分（三份＝卷一正文基线冻结／卷二 §14 裁决流水 v1.1–v1.78 冻结／卷三现行口径操作投影；新开卷四承接 v1.79 起新裁决；**原文件保留为入口锚**，不破 ~200 入站链接）；② run_agent_loop 状态体拆分（两步走：先状态聚合 struct＋18 参 context 对象，后按 pub 消费面清点下沉子 crate）；③ spawn sink per-dispatch 化（**七件中优先**；**实施约束〔复审补记〕**：0bl ⑥⑦ 两处新杀点依赖 `global_process_scope().kill_active()` 全局登记可见性——须保留全局可见杀伤面或同步接入 per-dispatch scope 并加回归钉，否则取消响应静默回退）；④ 编辑面大文件上限（16–32MiB 实施时定档＋`ORZ_…` env 逃生＋明确报错）；⑤ 编辑面 CRLF 逐行行尾保真（钉子覆盖混排行尾且不回退 0bl ① 四形状）；⑥ 写前核证下沉机械层（核证＋编码保真＋emoji 剥离收敛为写路径强制公共层，去 `search_replace` 名字特判）；⑦ 编辑面回退窗口＋告知携带（硬编辑预存回退窗口、告知行携带回退指针；连带设计档模板增补「回滚路径」必填小节〔仅约束新增，存量不回填〕）。
- [ ] **0bm 观察件（不占计数）**：RLI 影子通道——2026-09-24 用户令「还在测试中，暂时不进行处理但留作观察项，没价值的话就回到域判定状态，将其退化」；退化判据＝后续 release 全量档／狗粮轮无决策面真实消费 RLI 输出，则退化回域判定状态、冻结谐振扩展。
- [ ] **本轮同载三件（各有独立小节，同轮执行）**：**0bj**＝0bi 轮摩擦与承接二轮（① 版本核验旁路＋入口纪律／② 块表说明行口径／③ PS 5.1 `cd /d` 告知／④ GBK 乱码编码链评估＋⑤ 硬打断 320K 柔和化·硬形上移 500K＋⑥ 中性终态与结束自述实施，及承接 ⑧⑨ 首读；见 `P1-0bj`）；**0bp**＝检索形态改指浏览器车道（`web_search` → 本地浏览器 SERP，B 路径；四子件由主代理定，见 `P1-0bp`）；**0bq**＝进程收口兜底（出身登记＋run 收尾扫净绕出 Job 的游离进程，见 `P1-0bq`）。
- [ ] **并入件·①（2026-09-25 用户令）**：`orz web` 启动文案在 GBK 控制台显示乱码——证据＝[`075 §10.5`](docs/audits/075_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-25.md) 载体级 Web 探针日志（`orz web: Web 宸ヤ綔鍙板凡鍚姩…`）；处理面＝0bj ④ **输出编码链评估**（能力面/工具面输出编码收口；`orz web` 启动行随其统一，不单独立项）。**S3 真机随轮首读。**
- [ ] **S3 真机**：随本轮一并观察（编辑面／取消面／压缩新面〔0bk 回执／0bn 随批〕／检索新形态／进程收口新面首读；新摩擦随轮记录）。
- [x] **首次真机轮达成增量（2026-09-25，轮 `RUN-CLI-6ab6275c`；载体 0.6.13；源冻结 `b7dd241e`）**：**编辑面簇四件落码并全绿**——⑤ CRLF 逐行行尾保真（`LineEnding`／`line_endings`／`prevailing_ending`／`push_lf_text_rendered`／`splice_with_line_endings`＋锚点/经典两路径＋7 钉，旧钉按新语义改写）／④ 编辑面大小上限（默认 16 MiB＋`ORZ_EDIT_MAX_FILE_BYTES` 逃生＋含实际大小的拒绝文案）／⑥ 写路径收敛去名字特判（新增 `util/write_face.rs` 单点＝`decode_for_edit`＋`attach_notice_line`；loop 侧三处改「`expected_anchor` 在场」**结构触发**；删 `utf16_rejected_output` 死码）／⑦ 回退窗口（`.gsa/rollback/<hash8>/<millis>-<call8>.bak` 原始字节快照、每文件保留 5、成功面告知行带指针、失败随行如实携带原因）。**0bj 侧**：② 块表说明行口径改（指针仅「已压缩/已截断」块具备）／⑤ 320K 柔和化（去「现在就压」祈使、500K 强制形保留）已落码（**钉子待补**）；⑥ 勘定更正＝中性终态**早已实施**（0bh `5e153de1`），原立项描述「未实施」为误判。读数：`orz-tools --lib` **2939/0/6**、`search_replace` 119/0、`orz-loop anchor` 20/0、fmt 净；**主会话独立复核** `orz-tools --lib` 2938/1（唯一失败＝lsp e2e 并行 flaky，单跑过）。明细 [`0BM 轮报告`](docs/audits/0BM_EDIT_FACE_IMPLEMENTATION_AND_FRICTION_2026-09-25.md)；**记账批入账 `b7dd241e`（未重建载体）**。
- [x] **载体重建与换装（2026-09-25，载体 0.6.14；源冻结 orz `e22c0dba`）**：用户令「随后再重建一轮，把新的修改应用进 orz 包体」——版本 bump **0.6.13 → 0.6.14**（两文件两行，已推 `cli`）＋双平台重建（Windows `-Release -Clean`：`cargo clean` 35,708 文件／18.9 GiB ⇒ **28m30s** exit 0、**警告 0／错误 0**；Linux musl 暖缓存 **11m40s** exit 0、三件 ET_DYN／x86-64／PT_INTERP=0、bookworm·alpine 双向冒烟 **6/6**）＋**六件换装 MATCH=True**＋ACAF 重 provision exit 0（keystore 四值逐位未动、manifest↔signer 逐位一致）＋**载体级 Web 探针全过**（无令牌 401／伪 Host 401／带令牌 200、零残留进程）＋**字面量核证新面 7 项 0 → ≥1／保持面 43 项零回退**（本轮新发现：载体版本串不落字节面，登记备查）＋本地打包暂存 `D:\tb-eval\rel-076-stage\`（**0.6.14＝中间过渡版、不发行**〔用户令 2026-09-25〕）。⇒ 0bm 增量（编辑面簇四件＋0bj②⑤）**首次进入在役载体**。明细 [`076 重建档`](docs/audits/076_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-25.md)。
- [ ] **未竟移交（勘定结论已入报告 §5）**：① ADR-0010 分卷拆分／② `run_agent_loop` 状态体拆分（**执行面自 2026-09-26 起随 `P1-0bs` 同载**，2026-09-26 用户令「也合并进0bs这个大狗粮轮」；本条目闭合仍随 0bm 史）／③ spawn sink per-dispatch 化（落点已清点，方案未定稿）／**0bq** 进程收口（与 ③ 共登记面）／**0bp** 四子件（规格已录）／**0bj 残余**＝②⑤ 钉子、③ `cd /d` 告知落点、④ 设计档模板位置未决。
- [ ] **S3 重建后验证清单（报告 §6）**：块表说明行新文案／320K 软文案／`[RUN_END]` 自述与 `run_finished` 字段面／回退指针行随编辑成功面／大小上限拒绝文案（可用 `ORZ_EDIT_MAX_FILE_BYTES` 小值实测）。
- **归属更正（2026-09-25 记账批，本轮回查实证）**：0bj ④「输出编码链」**范围收宽**——非仅 `orz web` 启动文案，PS 5.1 管道／Tee 路径下 orz 中文 stdout **全谱**乱码（本轮启动器日志正文＋最终汇报即实例）；该件**执行面**随下节 `P1-0bs` 同载落地，**0bj 条目的闭合仍随 0bm 收口**（沿 0bp／0bq 同轮接口先例，不设两套账）。
- 入口：审计档 `docs/audits/072_FULL_REVIEW_REMEDIATION_2026-09-24.md` §7 / TODO P1-0bm / BACKLOG 0bm / 第二卷 §1.11。


### P1-0bp 检索形态改指浏览器车道（`web_search` → 本地浏览器 SERP · B 路径）（P1；2026-09-24 用户令立项，BACKLOG 0bp；来源＝2026-09-24 ACP 真机会话 `RUN-aa18ef7e-0` 观察〔检索门关闭 ⇒ 模型自建 Python 脚本抓取，Bing 脚本返回垃圾结果、Google 不可用〕＋用户令「把 web_search 重新指向本地浏览器车道」「web_search 这一部分做 B 路径即可」「四个子件由你来确定」；**同轮执行＝0bm**）

> **形态（用户裁决 2026-09-24）＝B 路径**：外部检索子代理调用 `web_search` 时，执行体先走本地浏览器车道（引擎 SERP），失败按真实类别回退脚本道；**主面 `web_search` 单一入口语义不变**，模型面标注面不动（不触碰 ADR-0010 §14.65 第 3 项②「静态标注、零动态字段」）。

- [ ] **四子件（主代理定，2026-09-24）**：① **工具关系**——`browser_control{action:"search"}` 保留，与 `web_search` **共用同一份 per-activation SERP 预算**（单账本、不双计）；`web_search` 为检索默认执行入口，`browser_control search` 留给导航／标签页型流程；② **降级序**——浏览器 SERP（首选）→ 本地分段 HTTP（`ORZ_WEB_SEARCH_LOCAL=1` 时）→ provider 合成（`/responses`）；全不可达按 FP-2 如实回传真实 cause；`ORZ_WEB_SEARCH_LOCAL` 语义由「整条 `web_search` 短路」改为「允许 HTTP 兜底」（撤 `orz-tools …/web_search/client.rs` 整段短路，改分级执行）；③ **时限统一**——收敛到既有裁决「首个结果 ≤10 s」：引擎 per-engine 10 s、`T_segment` 每页 10 s、整体 30 s；浏览器拉起等待不吃首个结果预算（拉起失败立即 `capability_unreachable`）；④ **批序**——S1 设计定稿（含 ADR-0010 §14.65 脚注：`web_search` 静态标注语义改述为「检索入口（浏览器优先、脚本兜底）」，双车道条文与 γ 退役不动）→ S2 落码＋钉子（接线钉／降级钉／预算共账钉／撤整段短路钉）→ S3 重建载体（随 0bo 轮前置）→ S4 真机复验。
- [ ] **S4 判据（预登记）**：带 URL 来源占比、证据阈值达成、引擎链读数（`chain_detail`）、失败类别如实回传、零真实 400；顺带收 0ax「无 URL 合成」主路径消解读数。
- **2026-09-25 用户令**：**执行面随 `P1-0bs` 同载**（本条目闭合仍随其自身史）——S1 设计定稿→S2 落码＋钉子→S3 载体重建→S4 真机复验随 0bs 跑。
- 入口：BACKLOG 0bp / [`ADR-0010 §14.65`](adr/ADR-0010-vol-14-addenda-index.md) / [`检索补强设计`](docs/RETRIEVAL_LOCAL_SEGMENTED_HARDENING_DESIGN_2026-09-15.md) / [`0v 实施设计`](docs/RETRIEVAL_ENGINE_SERP_SEARCH_ACTION_DESIGN_2026-09-10.md)。


### P1-0bs 0bm 轮摩擦与承接大杂项三轮（可处理七件；同载 0bj ④ 输出编码链执行面）（P1；2026-09-25 用户令立项，BACKLOG 0bs；来源＝[`0bm 轮报告 §4 摩擦台账`](docs/audits/0BM_EDIT_FACE_IMPLEMENTATION_AND_FRICTION_2026-09-25.md) F1–F13＋主会话 2026-09-25 回查；**执行形态＝杂项狗粮轮（不直接处理）**）

> 用户令「随后将除了启动器摩擦以外其他的应该处理的摩擦项立项为新的杂项狗粮轮任务吧」。三分类沿 0bd／0bg／0bh／0bi／0bj：**只收可处理项**；**启动器摩擦（`PSModulePath` 遮蔽致 `Get-FileHash` 失败）仅记录、不入轮**（见下）。**同载**＝0bj ④ 输出编码链**执行面**（范围收宽实证随本轮落地）；0bj 条目闭合仍随 0bm。

- [ ] **可处理七件（S1 勘定→S2 落码＋钉子→S3 真机首读→S4 收口）**：
  ① **F8 结束自述通道告知面缺位**——机制在位（`model_stop.rs` 解析＋controller 落账＋钉子 5084）但**模型面零常驻告知**：`[RUN_END]` 语法行只出现在 `render_board_guide()`（＝pull 面 `blackboard_read section=guide`，`controller.rs:1817`），实证＝0bm 轮 137 工具轮**未自述**、`run_finished` 仍三键（`status`／`turn_count`／`tool_rounds`）；处置面＝告知面收口（常驻或收束轮专用告知）＋钉，承接 0bj ⑥ 的「实施核心」争议。
  ② **F6 输出编码链全谱**（同载 0bj ④）——PS 5.1 管道／Tee 路径下 orz 中文 stdout **全谱**乱码（`orz web` 启动文案为其中一例，本轮日志正文亦同）；处置面＝输出编码链收口（能力面/工具面编码自报或入口纪律）＋钉。
  ③ **F3 lsp e2e 并行 flaky**——`implementations::lsp::tests::e2e_restart_replay_requeues_pending_diagnostics`：轮内两态、主会话独立复核 2938/1、**单跑恒过**（1.48 s）；判据＝全量并行连跑零失败，或根因隔离（端口／临时目录／共享状态）＋钉。
  ④ **F5 分块表说明行同源重复**——说明行在**表尾**与**模型面通知**两处重复出现；判据＝单源一次（与 0bj ② 不重叠：② 管「指针可见性口径」，本件管「同文重复」）。
  ⑤ **F11 rustfmt 版本噪声**——本机 rustfmt 与仓库档案版本差 ⇒ `cargo fmt` 产出两处无关噪声（`read_file/mod.rs` `panic!` 折行、`emoji_strip_ranges.rs` 尾空行），轮内已 `git checkout --` 复原；处置面＝工具链版本钉或口径落文档＋fmt 检查门不误伤。
  ⑥ **F12 压缩窗口摘要消费时机不可观测**——模型两度投递摘要未见即折、第三度明示区间方折；判据＝窗口与摘要的握手时点对模型可见（回执／告知面时点收口）。
  ⑦ **F13 报告类产物被机械 emoji 剥离**——本轮报告初稿状态符号被剥离（告知行已给计数与行号）；与 `FUS-WRITE-EMOJI-STRIP` 同族。**2026-09-25 用户令定案＝开「状态符号白名单窗口」**：只放行状态符号（含 VS16 变体，整体放行、不拆序列、不拦截、不计数告知），其余照旧拦截；窗口集合与判据由本项 S1 定稿（**不另立计数**）。
- [ ] **同载两件（2026-09-25 用户令「将 0bp 检索形态 B 和 0ax S3 部分加进 0bs 项吧」；不新增计数——两件早已在 55 内）**：⑧ **0bp 检索形态 B 执行面**（四子件＝共 per-activation SERP 预算／降级序 browser→本地分段 HTTP→provider／时限「首个结果 ≤10 s」／S1 设计定稿含 ADR-0010 §14.65 脚注；批序 S1→S2 落码＋钉子→S3 载体重建→S4 真机复验；**现状＝`ORZ_WEB_SEARCH_LOCAL` 仍整条开关语义**）；⑨ **0ax S3 落码面**（fetch 侧兜底 `rquest`／`wreq` 或 reader 服务＋反爬站点复测；S4 真机仍留官方跑批面）。
- [ ] **检索线三件（2026-09-25 用户令「检索部分的处理也并进 0bs 好了」；不新增计数）**：⑩ **HTTP 检索路线（兜底定位）**——**2026-09-25 用户令：「http 本身终究还是兜底」** ⇒ 只服务「无浏览器／无 sidecar 时的最低可用性」；**清洗件不自写**＝SearXNG 单仓即现成清洗面（`360search` 114 行〔`li.res-list`＋`h3.res-title>a[data-mdurl]`＋`p.res-desc`〕／`baidu` 219／`sogou` 141／`quark` 382／`bing` 189，上游持续维护；独立 360 清洗项目在 GitHub 几无可用者——唯一命中＝2016 年 0★ `cl3403/360SerpScraper`）；聚合层＝自托管 SearXNG（JSON API 单入口；引擎集裁到直连可达者；**其网络层即 curl_cffi、`DEFAULT_IMPERSONATE="chrome"`**）；传输层＝指纹伪装只在**无 sidecar 兜底与 `web_fetch` 抓页**时需要（Rust `wreq`／sidecar `curl_cffi`）。**本机实测实证必需**＝同 URL 同版本客户端，无伪装时百度 1,438 B 验证页／搜狗 5,413 B 拦截页，切 Chrome 指纹后分别 905,164 B／119 结果与 480,463 B／11 结果；**夸克指纹无效**（阿里 `tmd/punish`，除非走浏览器车道）；Bing CN 可达但长技术查询**降级**（纳入相关性闸门复测口径）；**不做**＝自写引擎解析与反爬。⑪ **本地浏览器车道（主战场）**——用户令「实际上主要还是优化本地浏览器车道，有真机浏览器的话就安全和方便不少，**关键是要让模型能够正常使用浏览器**」＋「反检测浏览器这步值得做，加给本地检索路径」：真机浏览器接入（CDP 接入真机已装浏览器／profile 复制保留登录态／**有头优先**〔技能侧实测有头 4/4 相关 vs headless 0/5〕）＋反检测底座（`camoufox`／`DrissionPage`／`nodriver`／`patchright` 择一，与 0bp B 路径共用选型）；覆盖百度/搜狗风控、夸克 `tmd` 挑战、Bing 降级等 HTTP 面拿不到的读数。**判据＝模型能正常使用浏览器**：①工具面可发现可用（静态标注＋`browser_launch_result` 事实事件）；②导航／搜索／读页／交互四类动作可用；③并发 1、超时与断线可恢复；④失败可归因（`capability_unreachable`／`network_no_response`／`empty_result`／`no_progress`）；⑤安全边界不回退（回环／令牌／审批／零执行事实）；⑥沙箱面显式设计（技能侧实测：沙箱内同调用返回无关降级页、沙箱外返回真 SERP）；⑦S4 真机读数＝同题对比（浏览器 vs HTTP）。⑫ **检索源补充**（用户令「检索源补充值得加」）——直连友好高质量垂直源（GitHub API／Stack Exchange／arXiv／OpenAlex／Crossref／PyPI／npm）纳入检索源面，技术类质量高于百度、零反爬。**2026-09-25 第二令（定位修正，取代本件 ⑩ 的「自托管 SearXNG 聚合层」措辞与 ⑪ 的「反检测底座」措辞）**：用户令「浏览器车道肯定走真机环境啊，orz 自己都不在沙箱里面了」「orz 跟着上游更新也不是个事，360 既然是怎么都可达且可用，那我们就只做伪装，http 层留 DDG/bing（非 cn bing）/百度/360，这样就不用跟着上游更新了，子代理好歹也是和主代理同级的模型，对结果的判断就交给他了」「剩下的就是本地浏览器的指纹伪装＋我们自己做的检索动作伪装」⇒ ① **⑩ HTTP 面＝自建四引擎小集**（`duckduckgo`／`bing_global`／`baidu`／`360search`）＋**只做 TLS/HTTP2 指纹伪装**；**不引入 SearXNG 运行时依赖、不跟上游更新**（其 `360search.py` 114 行等仅作**移植参考**，留来源标注）；**结果质量判断交检索子代理**（同级模型），机械层只做取回与如实回传、不设强相关性门。② **⑪ 浏览器车道＝真机（无沙箱）**：**指纹面天然为真**（真机已装浏览器＋真实 profile ⇒ 不需要 camoufox 类反检测底座），接入＝CDP 接真机浏览器／复制登录态 profile 避免锁／有头优先／避免自动化痕迹（不启用 `--enable-automation`）；**自研检索动作伪装**＝现状已落 pacing＋jitter（`local_browser/serp.rs` session pacing ≥5 s＋0–50% jitter），设计已定人化输入（逐字符键入＋提交前停顿＋Enter）但**当前不可达**——`local_browser` CDP 白名单禁 `Input.*`／`Runtime.*` ⇒ 需 S1 设计门放开最小输入动作集（键入／回车／滚动／点击）＋节奏伪装＋URL 门禁与审批边界。③ **本机直连补测（2026-09-25）**：百度 无伪装 1,438 B／验证页 → Chrome 指纹 **745,746 B／118 结果**；360 两态均 ~358 KB／7 结果（无风控）；**Bing 家族（`bing_cn` 与 `bing_global`）两态都降级**（同返「Rust 程序设计语言／菜鸟教程」泛化标题 ⇒ 换非 cn 不解决）；DDG 直连不可达（需代理）；夸克指纹无效、不进集。**2026-09-25 第三令（形态再修正）**：用户令「那 http 层的 bing 也不留了」「CDP 直接直连用户真机浏览器，直接启动即可，不复制，当前浏览器占用的话，模型自己看浏览器控制栏进行判断，如果用户在明确使用浏览器，那就启动无头；如果只是进程挂着，那就直接用当前进程」「模型对浏览器的使用是完全的，和用户一样，在具体使用上不做限制」「输入动作方面，常驻，由机械层直接机械做，模型下发指令后机械层自动做对应的输入动作拟真，避免拦截和风控」⇒ ① **⑩ HTTP 集缩为三件**＝`duckduckgo`／`baidu`／`360search`（**Bing 家族移除**，依据＝两态皆降级的实读；DDG 直连不可达，定位为有代理时的质量源）。② **⑪ 接入＝CDP 直连用户真机浏览器**（orz 直接启动，**不复制 profile**）。③ **占用判定交模型**：机械层出浏览器状态事实（控制栏／占用态），模型判——用户明确在用 ⇒ 起**无头**实例；仅进程挂着 ⇒ **用现有进程**。④ **动作面＝用户等价、使用上不做限制**（取消现 CDP 白名单对 `Input.*`／`Runtime.*` 的限制；标签页／下载／脚本等一并不设动作级限制）。⑤ **输入动作伪装＝常驻、机械层自动施加**：模型下发指令后机械层自动做对应输入拟真（键入／停顿／滚动／点击节奏），模型无感，目的＝避免拦截与风控。⑥ **S1 须一并定**：动作面放开后的安全与审计面口径（URL 门禁／审批／「UI 零执行事实」的重新表述）、占用判定的事实形状、无头实例与真机实例的会话隔离、拟真参数单一源。**2026-09-25 第四令（S1 四项口径结清）**：用户令「动作放开就行，orz 自己都进真机环境了，浏览器操控放开没什么问题，下载和脚本执行的话一律弹验证，需要用户明确批准才能执行」「无头和用户实例两端不混用，单次子代理进程中只使用一类，每次使用时对所使用的浏览器类型进行记录留档即可」「占用判定的话，我们做的本地浏览器使用本身就是将浏览器的控制面开放给模型的啊，不是单纯做的使用接口的链接，模型直接查看当前浏览器页面的占用和活跃情况以及界面内容/操控情况就好了」「拟真参数的话，就统一定一套固定标准即可」⇒ ① **动作面＝全面放开**（用户等价）；**唯二门禁**＝**下载与脚本执行一律弹验证、需用户明确批准**。② **无头实例与用户实例不混用**：单次子代理进程只用一类；**每次使用记录所用浏览器类型并留档**。③ **占用判定不需要额外机械事实面**：控制面本身对模型开放 ⇒ 模型直接查看当前页面的占用／活跃／界面内容与操控情况自行判断。④ **输入拟真＝统一固定标准（单一源）**，取值来源＝公开人类操作数据（Aalto 136M 击键：168,000 人／平均 52 WPM／数据集可下载回算；Balabit Mouse Dynamics Challenge：鼠标时序与坐标；WindMouse 类人类化轨迹件；CMU Killourhy-Maxion 击键基准；Fitts 律），**不下载数据集、不做回算**——直接以**论文公开数据区间自行取值**并登记来源（用户令「数据集有点大啊，我们不回算了，就用论文中的数据区间并直接自行取值吧，这也不是什么要求一定严谨的内容」）。读数与候选清单见 [`0BS 检索线路线调研`](docs/audits/0BS_RETRIEVAL_ROUTE_SURVEY_2026-09-25.md)。
- [ ] **同载两件（2026-09-26 用户令「这一部分的话我打算将其合并进0bs，还有“0bt 已登记的未竟拆分项②”，也合并进0bs这个大狗粮轮」；不新增计数，闭合随各自史）**：⑬ **信任按钮绑 ACAF＋B 形态整机全局＋点击切换＋会话清单随切换刷新**（0br Web 工作台批八线，承接批七「信任窗 Web 化」）——用户令「‘信任’就是要orz能够使用这一工作区，当然要绑ACAF，这一信任动作本身也是在对应工作区下打开orz后才能发生的，到直接使用的场景中后，orz不能是一个必需依赖其他的AI来帮助认证的半成品，如果不绑ACAF的话orz根本没办法在未信任工作区被拉起啊」「我打算做“B 服务随启动而立＋信任清单全局共享＋服务内点击切换”」「切换时正打开的对话留下即可，不清场，以免丢失内容」。**S1 设计要点（落码前定稿）**＝① **信任＝可用性一体动作**：授信从「仅 TrustStore 写入」升级为「TrustStore 授信＋ACAF 绑定核验/provision」一体面——桥侧核 ACAF 三件（manifest／keystore／签名器可达），缺失触发既有 `orz-acaf-provision` 或如实回显缺口，失败 fail-closed 沿批七 error.data 透出口径；**ACAF 架构不动**（签发器独立进程窄 IPC、ADR-0011 语义不变）——绑定的是「工作区授信 ⇒ ACAF 就绪」组合动作，不是新票据类型；② **按钮面**＝探索器「当前: {cwd}」行尾「信任」功能键（与「归档」「回档」「删除」组级功能键同款；已受信且 ACAF 就绪 ⇒ 呈已信任/禁用态；`POST /api/trust` 既有路由扩展，浏览器零路径输入不变、cwd 取桥 state）；③ **B 形态**＝服务随启动而立（**不做**单例／固定端口／跨进程切换——A 形态否决）；信任清单本就用户级全局共享（TrustStore 单源、`/api/boot` 已下发）；服务内点击「已信任工作区」清单项 → 机械切换；④ **切换语义（机械层机械切换、模型零感知）**＝桥 `state.cwd` 由启动固定改可切换（内部锁），切换＝重解析 `.gsa` 根 → runs／conversations／archives 清单即时刷新＋新会话在新 cwd spawn（桥持「工作区 → agent 子进程」惰性缓存，旧对话子进程不杀）；**同会话不迁移（对话只认初始工作区）**；**切换时正打开的对话留下、不清场**（与 `newSession` 清场语义显式区分）；跨工作区投影＝已打开旧会话的实时尾／回放按其**归属工作区根**解析（防切区后尾断／404）；⑤ **门禁衔接**＝点击未信任工作区 fail-closed 拒绝并引导「信任」按钮（授信后重试）；运行中（活跃 run）禁止切换（沿运行中拒绝守卫）；⑥ **批序**＝S1 设计定稿→S2 落码＋钉子→随 0bs 下一轮载体重建→S4 真机（与 0bs 复验五点并跑）。⑭ **run_agent_loop 状态体拆分（执行面）**（**0bm 未竟②** 并入；两步走方案沿 0bm 原登记＝先状态聚合 struct＋18 参 context 对象〔机械搬移，orz-loop 测试兜底〕、后按 pub 消费面清点下沉子 crate；0bm 条目闭合仍随自身史；**勘误**＝[`0BR_S3 归档投影审计` 批七](docs/audits/0BR_S3_ARCHIVE_PROJECTION_2026-09-25.md) 收尾语「治本归 0bt②」系**误标**——该拆分正规登记位为 0bm 未竟②，0bt ② 为版本核验旁路，本批就地更正批七收尾语）。**2026-09-26 追加（状态）**＝**S1 定稿完成**（会话内定稿；六缺口 D1-D9 全关：授信＋ACAF 绑定收口 `orz trust` 执行体、provision 原语单源下沉 orz-host〔禁两套实现〕、绑定结构化回传＋桥缓存注入 spawn、`POST /api/workspace/switch`＋cwd RwLock 化、切换机械刷新＋停尾、**不清场＝旧对话保渲染标「旧工作区会话（回看）」**、不引入会话归属映射的简化归属、双端守卫）；**执行＝S2＋钉子交接 orz 狗粮处理**（用户令「实际查验缺口直接给orz进行处理」；**不落任务书文件、主会话不代启动**——交接内容由用户掌握）；**载体重建本轮不做**（用户令「重建不做」）——S2 落码面停在树面，随下一轮载体令进载体。
- **0bt 全轮并入（2026-09-26 用户令「0bt进0bs」；编号保留、执行随本轮、闭合随 0bs、计数不变——沿 0bj／0bp／0bq 并轮先例）**：**0bt ①-④**（长单行／大单列取用面〔承接 0bb〕／版本核验旁路＋入口纪律〔承接 0bj①〕／宿主 shell 通道纪律〔承接 0bj③＋m9〕／权限判定来源落账〔原 0bu 并件〕）随本轮执行，S3／S4 即本轮下一轮载体与真机；落码五件（⑧-⑫）已列上节，不再两套批序。**0br 残余暂不并入**（用户令「0br暂时不」——快照选择器／会话标题增强＋0br S3 真机首读与 S4 收口仍归 0br 条目）。
- [ ] **同载一件（2026-09-26 第三令；不新增计数，闭合随 0bs）**：⑮ **审批弹窗否认选项去 Grok 化**——用户令「目前审批弹窗下否认选项问的还是告诉Grok应该做什么，这个映射不合适，将其换为通用的‘AI’」。**落点＝单一源常量**：`orz/crates/codegen/orz-workspace/src/permission/prompter.rs:19` 的 `REJECT_ONCE_LABEL = "No, and tell Grok what to do differently"`，由**五处构造点**共用（edit／bash-TUI／generic-bash-Web／MCP／fallback；泛化 bash 的 `reject-always`「No, and don't run bash commands」不含该名、不动）⇒ **一处改即 Web 与 TUI 同面**（Web 弹窗按钮文案取 ACP `options[].name`，前端无独立副本、无中文译名副本）。**目标文案**＝`No, and tell the AI what to do differently`（把 `Grok` 换为通用 `AI`；若取逐字替换口径则为 `No, and tell AI what to do differently`，由 S1 定名）。**批序**＝S1 定名→S2 落码＋钉子（五处构造点文案一致性断言）→随 0bs 下一轮载体→S4 真机实读。**同族 `Grok` 字样＝已随 ⑯ 并入执行（第四令，见下）**：transport 错误文案两处（`orz/crates/codegen/orz-sampling-types/src/error.rs`：`Grok is temporarily unavailable…`／`Connection to Grok timed out…`）／模型面身份标签（`orz/crates/codegen/orz-agent/src/prompt/context.rs` `DEFAULT_SYSTEM_PROMPT_LABEL = "Grok"`）／MCP OAuth 客户端名（`orz/crates/codegen/orz-mcp/src/oauth.rs` `MCP_OAUTH_CLIENT_NAME`）——是否一并去 Grok 化、是否含模型面，留用户裁决。
- [ ] **同载一件（2026-09-26 第四令；不新增计数，闭合随 0bs）**：⑯ **同族 `Grok` 字样一并通用化（三处）**——用户令「一并通用化吧，都加进0bs项」「0bs项中不做重建」。① **transport 错误文案**（用户可见）：`orz/crates/codegen/orz-sampling-types/src/error.rs` 两处——`Grok is temporarily unavailable. Please try again in a moment. (HTTP {code}).`（502..=504 档）／`Connection to Grok timed out or was interrupted. Please try again. (HTTP {code}).`（520..=524 档）⇒ 主语改通用 `AI`（S1 定名；同函数其余档不含该名、不动）。② **⑯-②（模型面身份标签）＝撤出本件**——用户 2026-09-26 口径「**用户层不出现 Grok 即可；内部不消费的不用动，毕竟是 grok 的代码资产**」：该标签在**模板层零消费**（阶段 A 去人格后三份在役模板 0 命中）、在**配置解析层亦零消费**（`DefaultModelEntry` 只反序列化 `model`，同块 `name`／`description`／`system_prompt_label` 一律被忽略）⇒ 属内部/上游资产字面，**不改**（原「字面替换」口径作废）。；③ **MCP OAuth 客户端名**（对端可见）：`MCP_OAUTH_CLIENT_NAME = "Grok"`（`orz/crates/codegen/orz-mcp/src/oauth.rs:25`）经 RFC 7591 动态注册暴露为第三方 OAuth 同意页应用名 ⇒ 改通用 `AI`；**风险**＝对端或按名留档、改名后或需重新注册（S1 记档）。**边界（不在本件；改则破协议或破兼容）**＝模型 id／name（`grok-4.5`／`Grok 4.5`，传输标识）／工具 id 前缀 `GrokBuild:*`（协议面、工具面冻结）／`GROK_HOME` 环境变量与安装目录、`~/.grok` 信任存储（兼容面）／`xai-*` crate 名与 `GrokClientHandler` 等内部符号。**文案与口径定案（用户 2026-09-26）**＝① **⑮ 文案**「就按照你推荐的就行」⇒ 定案 `No, and tell the AI what to do differently`（不再留 S1 定名）；② **⑯-② 再修正＝撤出本件**（同日用户令「用户层不出现Grok即可，内部的不消费就不用动」：模板层＋配置解析层双零消费 ⇒ 内部资产字面不动；原「只改标签文字」口径作废）；PLAN-FIRST 模板去人格纪律不放松（`PERSONALITY_KEYWORDS` 仍锁 `released by xAI`／`Grok Build subagent`）；佐证＝**PLAN-FIRST 阶段 A 模板去人格（2026-08-15／16）已清零身份宣告**（主/子代理/apply-patch 模板与 `ORCHESTRATOR_PROMPT_BODY` 删身份宣告与语气内容、`<persona>` 段退役、XOR 加密模板重生成、渲染测试锁人格关键词）；实测在役三模板对 `system_prompt_label` **零消费** ⇒ ⑯-② 仅为代码默认＋模型配置值的字面替换、对模型面零注入；模板内唯一 `Grok` 为上游文档指针行（provenance 指针、非身份句）；**三份解密转写（BASE／CODEX／SUBAGENT）与解密脚本无留存价值，按用户令清退**（内容已零化并移出工作区根至 `.tmp-purge-staging/`，待删文件——本机命令策略拦 `Remove-Item`）。**批序**＝S1 定名与风险记档→S2 落码＋钉子（两处断言）→**本轮不做载体重建**⇒落码停树面、S3／S4 待后续令。**用户层口径（2026-09-26 用户令「用户层不出现Grok即可，内部的不消费就不用动，毕竟是grok的代码资产」）**＝**只清用户可见面**（判据＝该字面是否出现在用户或对端可见文本）；内部面（不消费字面／协议标识／上游代码资产）不动。**⑯-④（观察·待裁）**＝用户可见告警里的 `$GROK_HOME` 字面（`orz-bin/src/main.rs:111`／`orz-tui/src/runner.rs:173` 的「install dir and cwd/.gsa both unwritable — $GROK_HOME stays on the user directory」）——该名为**真环境变量**，改显示会失真，建议保留原样、留裁。
- [ ] **S2 落码（2026-09-25 达成；2026-09-26 两轮增量＝⑮⑯／0bt④＋auto-mode 核验〔a 轮，见 [`0BS 执行报告`](docs/audits/0BS_EXECUTION_AND_FRICTION_2026-09-26.md)〕＋0bt①②③／⑪ v1／f4〔b 轮，见 [`0BS 后半部分处理报告`](docs/audits/0BS_PROGRESS_2026-09-26b.md)〕）／S3 载体（2026-09-25 达成：0.6.15；2026-09-26 二度重建 0.6.16〔078〕／**三度重建 0.6.17 Windows 侧**＝0bt①②③／⑪ v1 进在役＋`--build-info` 在役首读，[`078 重建档`](docs/audits/078_CARRIER_REBUILD_WINDOWS_2026-09-26.md)／[`079 重建档`](docs/audits/079_CARRIER_REBUILD_WINDOWS_2026-09-26.md)；Linux musl 腿三批待补）／S4 真机（未跑）**：七件于真机轮 `RUN-CLI-6ab6570c` 内落码＋钉子（orz `1ec7b729`，已推 `cli`），并随 **0.6.15** 双平台重建换装（源冻结 orz `a8430054`；[`077 重建档`](docs/audits/077_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-25.md)）；**S4 复验五点**（① 结束自述常驻尾行实读／② launcher 编码实读／③ lsp 全量复跑／④ ⑦ 剥离计数不复发／⑤ ⑥ 未落地回执构造实读）留下一轮真机；新摩擦随轮记录。
- **仅记录（不入范围）**：F1 写路径告知行 vs concise 全串断言（轮内改 `starts_with`＋`contains`）／F2 快照名序陷阱（轮内改递增毫秒位避让）／F4 320K 模型面实读 322,223tk（喂 0bh ④／0bj ⑤）／F7 资源软提醒（设计内软档、无阻断）／F9 Rust 2024 env 测试写法（`unsafe` 块＋进程级锁）／F10 同形文段歧义往返（编辑面天然摩擦）。
- **启动器摩擦（用户令排除、仅记录待裁决）**：`scripts/dogfood_launch.ps1` 由外部进程拉起时 `Get-FileHash` 解析失败——子进程继承 codex 运行时 `PSModulePath`，其中 PS7 版 `Microsoft.PowerShell.Utility` 在搜索序上**遮蔽** Windows PowerShell 5.1 同名模块 ⇒ 首启中止（PID 13316）；绕行＝净化 `PSModulePath` 后重起通过；与 `orz_acp_launch.ps1` 注释已记录的同族摩擦一致（该脚本早用 .NET 兜底，狗粮启动器未跟上）；证据＝`.tmp-dogfood-0bm-launch2.*`／`.tmp-diag-psmod*`。
- **⑯-④ 裁决收口（2026-09-26 用户令「变量名必须保留原样」）**：上条末句「观察·待裁」项结清——用户可见告警里的 `$GROK_HOME` 取**真环境变量名不改显示**，本件**不改码**；不新增计数、闭合随 0bs。
- 入口：BACKLOG 0bs / [`0bm 轮报告`](docs/audits/0BM_EDIT_FACE_IMPLEMENTATION_AND_FRICTION_2026-09-25.md) / [`中性终态设计档`](docs/NEUTRAL_TERMINATION_AND_MODEL_STOP_DESIGN_2026-09-22.md) / TODO P1-0bj。


### P1-0bt 0bs 轮摩擦承接与检索线落码大杂项轮（可处理四件＋落码五件）（P1；2026-09-25 用户令立项，BACKLOG 0bt；来源＝[`0bs 轮报告 §5 摩擦台账`](docs/audits/0BS_FRICTION_REMEDIATION_AND_CARRYOVER_2026-09-25.md) m1–m15＋§4 同载五件 S1 勘定；**执行形态＝杂项狗粮轮**）

> **2026-09-26 用户令「0bt进0bs」＝全轮并入 `P1-0bs` 大狗粮轮**——四件（①-④）＋落码五件（⑧-⑫）全部随 0bs 执行（批序／载体／真机不再单列：S3／S4＝0bs 下一轮载体与真机，**载体重建本轮不做**待令）；**编号保留、闭合随 0bs、计数不变**（沿 0bj／0bp／0bq 并轮先例）；本节保留作范围清单与勾选面。

> 用户令「本轮新的摩擦项，除观察项内容以外，和本轮勘定内容的落码部分一同落成新的杂项狗粮轮」。三分类沿 0bd／0bg／0bh／0bi／0bj／0bs：**只收可处理项**；**观察·记录项不入范围**；**启动器摩擦按用户令排除**。批序＝S1 勘定→S2 落码＋钉子→S3 载体重建→S4 真机复验。**2026-09-25 用户令（范围裁决）**：「当前 0bt 内部的有关检索部分的任务内容……直接确定为除了重建和实机部分以外，全部做完吧」⇒ **检索部分（⑧-⑫）的执行面＝除 S3 载体重建与 S4 真机复验外全部做完**（S1 设计定稿＋S2 落码＋钉子**一次做完**，不再停在「S1 勘定」或分小批交付）；**四件（①-④）口径不变**（S3／S4 仍随下一轮载体与真机）。

- [ ] **可处理四件（新证据；均承接已立项件、不新增计数）**：① **长单行／大单列取用面**（m1／m2／m5／m13）——承接 **0bb**（`GAP-TOOL-LONGLINE-QUOTING`，P2）：`read_file` 对超长单行返回截断（`CLI_PROJECT_INDEX.md`／`TODO.md` 本轮双双中招）、`.gsa/conversations/*.json` 巨型单行不可直读、跨 PowerShell 层级 `$` 展开被吞；轮内拐棍＝`.tmp-lineview.py`；落码＝工具面稳定取行面（行窗／切片）＋账本条目切行。② **版本核验旁路＋入口纪律**（m15）——承接 **0bj①**：`dogfood_launch -DryRun` 回显 `carrier=unknown`（Rust 载体无 VersionInfo）⇒ 版本改读**打包面**／`--build-info` 类旁路。③ **宿主 shell 通道纪律**（m3／m9）——承接 **0bj③**（PS 5.1 `cd /d` 告知落点）＋本轮**新证据** m9：宿主 shell 无 `grep`／`head`／`sed`（工具面已提供 grep 工具、命令侧须改写）⇒ 工具描述／文档备注单点收口。④ **权限判定来源落账**（观测缺口；同日并件）——`permission_decision` 仅落 `{tool, decision}`（真机轮 `RUN-CLI-6ab6570c` 230/230 全 `allow_once`）⇒ 判定来源（rule-match Ask／fail-closed Ask／分类器〔现休眠〕／提示超时 fail-closed／Deny 来源）**不进 journal**，「为什么放行或拒绝」不可机械复核；方案＝**只加观测面**（封闭集来源字段＋schema 可选字段＋钉子＋S4 逐条对账）；边界＝不改判定语义、不启用休眠面、不新增判定能力；判定面变更须同步判定面单图。
- [ ] **落码五件（本轮勘定内容的落码部分；不新增计数；执行面＝除 S3／S4 外全做完，见上令）**：⑧ **0bp 检索形态 B 执行面**四子件（共 per-activation SERP 预算／降级序 browser→本地分段 HTTP→provider／时限「首个结果 ≤10 s」／S1 设计定稿含 ADR-0010 §14.65 脚注）／⑨ **0ax S3 fetch 侧指纹兜底**（与 ⑩ 共传输层）／⑩ **HTTP 三引擎**（`duckduckgo`／`baidu`／`360search`，Bing 出集）＋TLS/HTTP2 指纹伪装／⑪ **本地浏览器车道＝真机 CDP 直连**（不复制 profile；无头与用户实例不混用＋类型留档；动作面用户等价、唯二门禁＝下载与脚本执行弹验证；输入拟真常驻固定标准 v1）／⑫ **检索源补充**（垂直源：GitHub API／Stack Exchange／arXiv／OpenAlex／Crossref／PyPI／npm）。**建议批序**（沿 0bs §6）＝⑪ → ⑩/⑨ → ⑧ → ⑫（**批序只是推进顺序，不是交付边界**——一次做完）。
- [ ] **S3 载体重建＋S4 真机复验（本项对外两段）**：检索部分（⑧-⑫）落在这一层之外——S3／S4 待下一轮载体与真机令；四件（①-④）同随其各自批序（0bs 七件的 S4 复验五点并跑）。
- **观察／记录（不入范围）**：m4 供给侧 GBK（0bs ② 已修消费链）／m6 320K 硬窗口实读（设计内）／m7 回退窗口正常／m8 ⑦ 剥离亲历（0bs ⑦ 证据、S4 复验点）／m10 摘要投递时机（⑥ 面动力）／m11 同轮不可见（低优先观察）／m12 CRLF 进件留意（S3）／m14 消费侧 ANSI（② 反身实证）／启动器摩擦（用户令排除）。
- 入口：BACKLOG 0bt / [`0bs 轮报告`](docs/audits/0BS_FRICTION_REMEDIATION_AND_CARRYOVER_2026-09-25.md) / [`0BS 检索线路线调研`](docs/audits/0BS_RETRIEVAL_ROUTE_SURVEY_2026-09-25.md) / TODO P1-0bs。


### P1-0by ACAF 签名器不可达（桌面形态；根因未明）正式立项（P1；2026-09-28 用户裁决立项，BACKLOG 0by；54 → 55；来源＝102 批观察项 `OBS-ACAF-SIGNER-UNREACHABLE-DESKTOP-080` 升级）

> 现象：桌面形态下凡需控制票据的宿主工具（`browser_read`／`web_fetch`）被 `control_ticket_rejected: signer_unreachable` 拒（detail＝`io error: 管道正在被关闭。 (os error 232)`）——0.8.0 桌面两轮 27／23 条（工具面全灭）、同条件 0.8.1 轮 0 条、容器形态两载体均 0 条；本批秒级探针（三配置 `initialize_session` 全应答）排除签名器件与密钥库 ⇒ 差异在桌面 orz↔签名器交互面且无已知代码变更解释；**新版未复现≠已修**，按未修问题立项。

- [x] **S1 勘定（2026-09-28 完成，106 批）**：**根因已定**＝[`dogfood_launch.ps1`](scripts/dogfood_launch.ps1) 密钥库根错配（`ORZ_ACAF_KEYSTORE` 导出 `<AcafRoot>\keystore`，provision 建在 `<AcafRoot>`）⇒ 新 bin 目录下签名器 fail-closed 启动即退（`installation key root is not a directory`）；**秒级复现**＝[`probe_acaf_keystore_root.py`](scripts/probe_acaf_keystore_root.py) `--emulate-launcher`；**变量隔离**＝六格矩阵（三载体×两根）排除**载体版本轴**，「0.8.1 未复现」＝长期目录恰有遗留 `acaf\keystore`（负载轴不适用：首调用即全灭）；**观测缺口已补**＝`ORZ_ACAF_SIGNER_STDERR_LOG` 可开闭旁路（`Stdio::null()` 退役为缺省臂）＋钉 `signer_stderr_bypass_is_opt_in_and_fails_loud`（orz-loop lib 15/0）；**整机级单工具最小题面 A/B**＝红 `RUN-CLI-6ab949a4`（0 `tool_started`、6 拒、旁路 7 行 fatal）vs 绿 `RUN-CLI-6ab949c2`（票据 2 issued→2 consumed、命令照跑）。详见 [`106 S1 勘定档`](docs/audits/106_0BY_S1_DIAGNOSIS_2026-09-28.md)。
- [x] **S2 落码＋钉子（2026-09-28 完成，107 批）**：**修复＝provision 落点**——[`dogfood_launch.ps1`](scripts/dogfood_launch.ps1) 的 `& $provision $AcafRoot $manifest` 改为 `& $provision $keystore $manifest`（落点＝导出根＝`<AcafRoot>\keystore`；与容器侧 `/etc/orz-acaf/keystore`、重建批 provision 落点、历年实跑命令一致——**导出值本来就是对的那个**）＋[`orz_acp_launch.ps1`](scripts/orz_acp_launch.ps1) **同形同病同修**＋**装配门**（provision 后断言根是目录／`installation-key.json` 在册／有受保护密钥文件，不满足即在**装配点**抛出，实测「清单在册、密钥库缺件」⇒ exit 1 且不启动）＋装配清单回显 `acaf = keystore=… / manifest=…`。**先红后绿**：钉子 [`probe_acaf_keystore_root.py`](scripts/probe_acaf_keystore_root.py) `--check-launcher`（对 [`.tmp-b107-launcher-prefix.ps1`](.tmp-b107-launcher-prefix.ps1) ＝prov 落点≠导出根红＋签名器 `installation key root is not a directory`；对两枚现脚本＝各 6 项全绿＋签名器应答）；整机级（**发布 0.8.1 载体**）红 `RUN-CLI-6ab94e86`（38 事件／0 执行／6 拒）→ 绿新目录 `RUN-CLI-6ab9513b`（32 事件／2 issued→2 consumed）＋在役目录 `RUN-CLI-6ab95149`（43 事件／2 issued→2 consumed）。**同批方向更正**＝首次误改导出值（`$keystore = $AcafRoot`）后复核口径改正，中间读数 `RUN-CLI-6ab94f1e`／`-6ab94fae` **作废**。详见 [`107 S2 修复档`](docs/audits/107_0BY_S2_FIX_2026-09-28.md)。
- [x] **S3 载体（2026-09-28 完成，109 批）**：修复随载体重建进体——双平台 **0.8.2**（Windows release 增量 3m06s／Linux musl 6m27s；三件换装 MATCH 3/3）；**签名器 stderr 旁路装进二进制**（字面量核证：`ORZ_ACAF_SIGNER_STDERR_LOG` 两平台 0.8.2＝3 处／0.8.1＝0 处；缺省关、行为不变）＋两启动器 provision 落点修正随父仓 `a3298152` 生效；Windows 在役目录 ACAF 重 provision 后 manifest 绑新签名器；包内 `--build-info` 双平台读 `version=0.8.2`。详见 [`109 重建发行档`](docs/audits/109_CARRIER_REBUILD_AND_RELEASE_V082_DUAL_PLATFORM_2026-09-28.md)。
- [ ] **S4 真机**：桌面形态零拒绝（票据类工具全通过）＋容器形态回归零拒绝。
- [ ] **闭合**：S1／S2／S3／S4 达成后 `pending` → `implemented`，计数 55 → 54。
- 入口：[`107 S2 修复档`](docs/audits/107_0BY_S2_FIX_2026-09-28.md) / [`106 S1 勘定档`](docs/audits/106_0BY_S1_DIAGNOSIS_2026-09-28.md) / [`104 立项档`](docs/audits/104_ACAF_SIGNER_UNREACHABLE_REGISTRATION_2026-09-28.md) / [`probe_acaf_keystore_root.py`](scripts/probe_acaf_keystore_root.py) / BACKLOG `0by` / `scripts/dogfood_launch.ps1`。


### P1-0bz 上下文脸面瞬态分叉与指纹观测件（前缀缓存第二针／空跑 compress 分叉／自发塌陷；压缩触发策略不动）正式立项（P1；2026-09-28 用户令立项，BACKLOG 0bz；55 → 56；来源＝六轮狗粮存档只读逐轮复算（2026-09-28）＋[`0bm 报告 §8 缓存面实测`](docs/audits/0BM_EDIT_FACE_IMPLEMENTATION_AND_FRICTION_2026-09-25.md)＋[`0bi §10-④`](docs/audits/0BI_FRICTION_CARRYOVER_2026-09-23.md)「逐轮模型面前缀指纹」候选升级）

> 现象与读数：六轮狗粮长轮存档（09-25～09-27；`RUN-CLI-6ab6275c`〔0bm〕／`6ab6ce44`／`6ab6dc02`／`6ab7b7eb`〔0bw〕／`6ab7d8b7`／`6ab7f32f`；1,041 模型轮／input 146,947,652 tk）只读逐轮复算（口径与 0bm §8 账面逐位吻合）＝整体命中率 90.9%–95.5%；miss 10,301,636 tk 分账＝压缩后第 1 针 26%（2,650,263）／**第 2 针 24%（2,426,381）**／空跑 `context_compress` 瞬态分叉 12%（1,225,754）／自发塌陷 7%（737,873）／常态尾巴 30%／冷启动 1%。**三源客户端证据**＝① 恢复指纹：空跑 compress（防抖三态 no-op、无 `context_compressed` 事件）与自发塌陷（无任何特殊事件）均塌一轮即恢复命中原前缀（`6ab7b7eb` seq857→863→871＝214,144→7,296→217,344）⇒ provider 单元一直在盘、当轮请求是客户端脸面变体；② 脏 +2 例 +2 hit 反低于 +1（六例，极端 31,488→9,344）＝纯落盘时序不可能（同前缀至少保底），只能 +1→+2 间脸面又被改一次；③ 干净 +2 三例（comp@596 等）＝单往返落盘足够 ⇒ 第 2 针全部额外成本＝脸面重渲。**边界（2026-09-28 用户裁决）**＝「不压第一针，如果强制少压深压，模型的注意力质量和动作连续性就要受到影响，目前已经不能再割舍了」⇒ **压缩触发策略不动**（频率／时机／窗口大小／模型自选压缩面全部不动），第一针不作为消除目标（0bm §8.5 候选③出局）；**修复面＝渲染稳定化，规格边界写死＝只动位置／时点、不动内容**（瞬态行移尾／固定位置／开窗期间延续旧脸渲染、摘要落地方才换脸；禁删行删信息）。

- [x] **S1 指纹件落码（插桩，模型零感知）**（2026-09-28 完成，111 批）：**实现勘定＝指纹单位取「逐消息」**（v8 视图＝消息列表投影，消息即装配自然单元——比设计稿的「分段」更细、定位更直接）；`face_fingerprint()` 逐消息对 **wire 同字段投影**（role/content/tool_call_id/tool_calls/reasoning_content；`round` 不落 wire 故不入哈希——防内部轮章变化造成假分歧）做 sha256 取 12 hex＋`first_divergence()`（LCP 首分歧；kind=mutated/inserted/removed）＋digest（16 hex）＋head 诊断头（120 chars）；生产者＝主车道每请求 emit（`model_round` 严格 +1；事件紧邻 `model_output` 之前 ⇒ 「分歧消息 ↔ miss」逐轮对账）；合约面＝`EventType::FaceFingerprint`＋`verify_face_fingerprint` 法官族（跨事件 LCP 重算对账＋两侧哈希核证）＋ALL_FAMILIES（47 → 48；112 批勘误——111 批此处原误记 66 → 67，该数实为信封 wire 枚举数）＋对拍场景（corpus 258 → 259）＋TUI 降级 Unknown。读数：orz-assurance **278/0**（Rust↔Python parity 绿）／orz-loop **858/0**（`user_cancel_closes` 偶发 flaky 与本批无关——干净树可复现）／orz-tui **178/0**；fmt 噪声与 HEAD 同面；clippy 零新增。
- [ ] **S2 钉子＋单轮读数**：先红后绿钉（fixture 两版脸面判定首分歧段）＋真机单轮采集，把三源（第 2 针／空跑分叉／自发塌陷）定位到具体段落（0bi §10-④ 的「把推断变为可对账读数」落地）。〔钉子半已随 S1 落码（111 批）：model_face 三钉（同 wire 同哈希／round 不入哈希／分歧三形）＋families 对拍场景；载体前置已达成（113 批＝0.8.3 双平台重建进体，`face_fingerprint` 事件随 run 落 journal）；**余项＝真机单轮采集**（随下一狗粮轮）。〔**部分读数已获（115 批，`RUN-CLI-6ab99969`）**：自发塌陷族指纹定位＝index 1 折叠桥/机械摘要行 mutated（r40，塌一轮即恢复）；第 1 针 ×2 确认（分歧段＝压缩 marker 本体）；+2 双 clean（本轮无第 2 针样本）；空跑族无样本——第 2 针／空跑定位仍待长轮。〕
- [x] **S3 渲染稳定化修码**（2026-09-28 完成，116 批，orz `b956b3ba`）：指纹定位把「开窗瞬态／自发塌陷」机理解剖为「指针＋D4 拖到首个分块形成才插入 face 前部」⇒ **修码①**＝常驻头自首个轮次起即在（注入点＝`round_ranges[0].0`＝开窗后 `preamble_end`，开窗转换只剩尾部追加，前缀零重排）；**修码②**＝审计确认现树 marker 单写手、D4 重渲同拍落 +1（115 批 0/2 第 2 针）⇒ 落**两连压稳定性钉**锁死（marker1 字节/位置与指针位在第二次落地后不变）；＋2 钉；既有协议钉 2 处按新不变量更新（3→5 条）；loop **860/0**／assurance **278/0**／tui **178/0**，clippy/fmt 触碰面零新增；**随 0.8.4 双平台载体重建进体**（pin `9f12ecc2`；Windows 3m42s／Linux 冷 40m59s；打包与活体两态全绿；未推送未发行）。机械判据（+2 针归零＋空跑零分叉）随 S4 真机。
- [ ] **S4 真机对账**：下一狗粮轮读数＝① 第 2 针归零 ② 空跑分叉归零 ③ 自发塌陷读数（如仍现，指纹定位）④ 命中率观察读数如实记录（不作硬判据，单轮纪律）。
- [ ] **闭合**：S1／S2 指纹件（有钉）＋S3 修码（先红后绿：指纹事件证明瞬态段消失）＋S3 载体＋S4 真机读数 ⇒ `pending` → `implemented`，计数 56 → 55。
- 入口：[`110 立项档`](docs/audits/110_CONTEXT_FACE_TRANSIENT_FORK_REGISTRATION_2026-09-28.md) / [`0bm 报告 §8`](docs/audits/0BM_EDIT_FACE_IMPLEMENTATION_AND_FRICTION_2026-09-25.md) / [`0bi §10-④`](docs/audits/0BI_FRICTION_CARRYOVER_2026-09-23.md) / BACKLOG `0bz` / `orz/crates/orz-loop/src/model_face.rs` ＋ `host_exec/tool_run.rs`（`context_compress` 防抖三态）＋ `agent_loop.rs`（loop-top 开窗）。


### P1-0cb 写入管控保底化修订（0bw v2；灾难硬边界保底＋一般写动作归还审批组件）（P1；2026-09-29 用户裁决立项，BACKLOG 0cb；56 → 57；来源＝TB 2.1 V4.1 官方轮错题解剖〔写控 43/44 题拦 215 条命令，build-pov-ray 题面要求装 `/usr/local/bin` 且自带 `+O/dev/null` ⇒ 结构性无解〕＋用户裁决「自研面锁的应该是 bios 等根本性文件夹……拦截外部根目录下危险的递归删除并返回命令已被拦截，需精准删除的硬边界」「其余的审批交给 codex 审批组件」）

> 设计权威＝[`WRITE_CONTROL_BACKSTOP_REVISION_DESIGN_2026-09-29`](docs/WRITE_CONTROL_BACKSTOP_REVISION_DESIGN_2026-09-29.md)（v2.0；修订 [`0bw v1 设计档`](docs/WRITE_CONTROL_MECHANICAL_DESIGN_2026-09-26.md) deny 表范围与规则分类；单一源／三落地点／留痕／补偿架构不变）。定案＝五条 block 规则封闭枚举（`catastrophic-recursive-delete`／`raw-device-write`〔`/dev/null` 豁免〕／`boot-firmware-flip`／`registry-hive-delete`／`carrier-write`）；整树位置锁、重定向即写、`/dev//proc//sys` 前缀词扫退役；一般写动作交回 Codex 血统审批组件；warn 全集留痕不阻断。

- [x] **S1 设计档 v2.0**（2026-09-29 完成）。
- [x] **S2 落码＋防膨胀钉**（2026-09-29 完成；未提交）：`write_control.rs` 收窄＝载体集门（C1/C2/C3）＋根本树根常量（`WINDOWS_DISASTER_TREE_ROOT_FALLBACKS`／`LINUX_DISASTER_TREE_ROOTS` 12 项，仅作规则 1 递归删除目标；v1 表 B `LINUX_SYSTEM_CORE` 原样保留给 L3 排除集）＋`path_equals_root`（恰为本体比对）＋`is_volume_root`；`exec_policy.rs` 五规则重写（`BLOCK_RULES` 恰 5 条封闭集＋`WARN_RULES` 2 条）＋`disaster_target_forms`（POSIX 根形态不拼 cwd）＋block 文案带「已越过保底硬边界」与规则 1「精准删除」指引；防膨胀钉＝恰 5 条断言＋触发表行数钉＋正向放行集 fixture（设计 §3 十例）＋五规则负向集（每规则 ≥2）＋退役面回归（系统树一般写不再 block、递归删仅 warn）；契约面同步＝schema 枚举/描述＋Rust 判官封闭集/配对表＋Python 镜像（fixture `carrier-write` 不变）；消费面＝`search_replace`（门注释＋退役回归测试）、undo（注释）、bash（规则 id 断言）；读数＝orz-tools lib **2981/0/6**（fmt 后 types 609/0 复验）、orz-assurance **278/0**（含 Rust↔Python 对拍绿）、runtime conformance **378/0**、orz-tui **178/0**、clippy 与基线逐位持平（tools 13／assurance 6／host 99）、触碰面 fmt 干净。**既有红（stash A/B 判归因，与本批无关）**＝orz-loop `user_cancel_closes_pending_activations_before_run_cancelled`（时序敏感，干净树同红；全量 859/1/3）＋orz-host `second_prompt_continues_hash_chain`／`session_prompt_produces_valid_journal_chain`（干净树同红；串行 353/2/6）＋doctor 门「orz submodule working tree is dirty」（未提交落码预期态，提交批消除）。
- [x] **S2 审查处理（v2.1，2026-09-29 同日）**：主会话全面审查（设计/实现/符合性三面；P1×1＋P2×3＋P3×5）＋用户三项裁决（① schema 升 v0.3 ② 过严臂收窄 ③ 全部处置）同日闭合——**① P1 契约面**＝`write-control-review-event-payload` schema 升 **v0.3**（7 现行值＋2 legacy 回放专值 `safety-mechanism-flip`/`system-core-write`；v0.2 文件冻结在盘作历史契约）＋registry 单一权威改指 v0.3＋判官/Python 镜像 `WRITE_CONTROL_LEGACY_RULE_CATEGORIES`（按原 block 分类配对核证；**只读豁免非生产集**——生产侧被恰 5 条封闭集钉死永不产出，回放豁免≠配对豁免）＋legacy 正例 fixture 一件＋对拍 corpus legacy 臂（双判官同语料）⇒ **0.8.4- 历史 journal 回放不再判红**；**② P2 过严臂收窄**＝规则 2 PhysicalDrive 词元扫收窄到写侧目标位（`of=` 值/`mkfs*` 目标词——读侧 `dd if=\\.\PhysicalDrive0` 备份取证与查询类提及放行）＋`mkfs*` 目标位判定（镜像文件构建放行）＋`/dev/vd`/`/dev/mapper` 登记进块设备形态集＋规则 1 动词集补 `ri`（`ri C:\Windows -Recurse` 落 block）＋Windows 盘符相对根形态（`\Windows`）按 `%SystemDrive%` 补全比对（绕过面闭合）＋规则 4 蜂巢判定收窄到目标位词元（数据值提及不误拦）；**③ P3**＝行数钉补全至全部封闭表（`DESTRUCTIVE_VERBS` 等 5 表＋载体面 3 表）＋sysctl 写 allow 登记为已接受后果（规则 3 候选留档）＋错题解剖引用修正（解剖数字在设计档头部，TB21 §8 为窗口总账）＋设计档升 **v2.1**。读数＝orz-tools lib **2983/0/6**（＋2 收窄负测）、orz-assurance **278/0**（含 Rust↔Python 对拍 legacy 臂绿）、runtime conformance **378/0**（schema 文件名约定测试带 v0.3 显式覆盖表）、check_repository 除 doctor「orz submodule dirty」预期态零错误（error_count=1）、clippy tools 13/assurance 6/host 99 与基线逐位持平、触碰面 fmt 零 diff。
- [x] **S3 载体**（2026-09-29 完成，117 批，用户令放行；源冻结 orz **`15bc1bfb`**＝`a8478ff4` S2＋审查处理＋`15bc1bfb` bump 0.8.4→0.8.5，`cargo metadata --locked` exit 0）：0.8.5 双平台重建换装——Windows `build_orz.ps1 -Release -Jobs 2` **exit 0（3m38s）**，换装 MATCH 3/3（`orz.exe 88848628…`／`orz-signer.exe 79d9cc3c…`／`orz-acaf-provision.exe 53ce64bc…`），`--build-info`＝0.8.5，回滚点更新 `.0.8.4-bak`；ACAF 重 provision exit 0、manifest `binary_sha256=79d9cc3c…` ↔ 换装位逐位一致、keystore 两件（`f37556ab…`/`2aa80cb8…`）逐位未动；Linux musl（APT_OK 零代理切换）直写换装位 MATCH 3/3（`orz ecf1d665…`／`orz-signer 9f0b925d…`／`orz-acaf-provision 28d690bc…`）、static-pie×3＋INTERP=0、alpine/bookworm 双冒烟 0.8.5；墙钟 **34m00s**（orz-tools 基座变更 ⇒ 近全树重编，如实记）；进体判据＝新规则 id 四值进体（1/2/2/1）＋「已越过保底硬边界」文案＋1＋版本串滚动（v1 旧规则 id `system-core-write`/`safety-mechanism-flip` 退出在役二进制——legacy 回放豁免集在判官 crate 非发行件，如实记）；打包 `rel-117-stage` 两侧各 5 entries、包内 6/6 MATCH、容器核证 zip 4/4・tar 4/4、包内 build-info＝0.8.5、清单活体两态（干净 0 finding／README+1B 恰 1 条）；源清单重生成 1506 条 valid；**未推送未发行**。
- [x] **S4 读数**（**2026-09-30 达成，123 批**；0.8.7 在役）：基准实测判据达成——拦截从 215 量级跌到个位数（build-pov-ray 0.8.7 试次全场恰 1 拦＝v3.1 祖先链臂按设计开火非误拦；`/usr/local/bin` 安装面全放行）；**重跑系列 5/5 完毕＝2 翻盘**（build-pov-ray 1.0、torch-pipeline-parallelism 1.0；extract-moves-from-video/git-multibranch/winning-avg-corewars 仍 0.0 属模型侧余量），全部 exit 0、Harbor 公开上传；下半场第一窗 12 题 0 作业失败＝保底零误拦、一般写动作归还审批组件在役。**〔口径史＝09-29 一裁全量重跑 → 二裁外科重跑 → 09-29 晚三裁收窄立项 0cc → 09-30 用户令重跑系列＋下半场第一窗执行完毕；权威＝[`TB21_V41_RERUN_SCOPE_DETERMINATION_2026-09-29`](docs/TB21_V41_RERUN_SCOPE_DETERMINATION_2026-09-29.md) §8–§11〕**
- [ ] **闭合**：S2–S4 达成 ⇒ `pending` → `implemented`，57 → 56（**闭合随读数用户裁决**——读数已齐）。
- 入口：BACKLOG `0cb` / [`v2 设计档 v2.1`](docs/WRITE_CONTROL_BACKSTOP_REVISION_DESIGN_2026-09-29.md)（错题解剖数字见其头部触发段）/ [`0bw v1 设计档`](docs/WRITE_CONTROL_MECHANICAL_DESIGN_2026-09-26.md) / [`TB21 V4.1 轮次档 §8`](docs/TB21_V41_FULL_RERUN_START_2026-09-28.md)（窗口总账）。


### P1-0ch 灾难兜底精准化（防删除破坏口径；L3 粒度对齐 L1/L2＋设备面文件级放行＋根级新条目放行）（P1；2026-10-01 用户令立项，BACKLOG 0ch；61 → 62；**2026-10-02 用户裁决闭合 57 → 56〔151 批〕**；来源＝140 批 rerun3 十八题逐题复算——11/18 题出现「设备目录写入被拒」、3 题叠加「根目录不可新建」）

> 用户令原话：「灾难兜底还存在摩擦啊，考虑进一步放大权限？能不能将灾难兜底做的更加精准一些？毕竟主要是防删除破坏」「两条框架摩擦跟着立项成灾难兜底改良的子项吧」。口径＝灾难兜底防的是**删除／毁写／引导与内核机制翻转**，不是一般性写入；L3 粒度必须与 L1/L2 表内规则对齐。

- [x] **子项 ① 设备目录写入被拒（`/dev`）**：L3 `LINUX_DISASTER_KERNEL_FACES` 把 `/dev` 整棵子树列为不授权 ⇒ 写族（含 `>/dev/null`）内核层 EACCES，而 L1/L2 规则 2 对 `/dev/null` **显式豁免**（两层粒度不一致）。候选＝设备面**文件级**放行安全节点（`/dev/null` `/dev/zero` `/dev/full` `/dev/tty` `/dev/random` `/dev/urandom` `/dev/ptmx`；块设备与 `/dev/mem` 类仍不授权）。
- [x] **子项 ② 根目录不可新建（`/`）**：L3 授权表由 `/` 顶层条目枚举生成、`/` 自身不在表内 ⇒ make 族在 `/` 上不成立 ⇒ `mkdir /git` 恒 EACCES。候选＝`/` 本体授予 make 族（顶层新条目放行），`/boot` `/proc` `/sys` 与 `/dev` 未授权面仍不可入内创建。
- [x] **S1 设计勘定**：两子项逐条定档＋`/proc` `/sys` `/boot` 写需求复核＋「枚举期快照」边界判（spawn 后新增顶层目录仍无授权）。
- [x] **S2 落码＋钉子**：L3 授权面与 L1/L2 粒度一致性测试；题面级 e2e 两探针＝`>/dev/null` 可行、`mkdir /git` 可行，且 `/dev/sd*` 类仍不可写。
- **S1＋S2 已达成（2026-10-01，143 批）**：设计档 v4.0 §7 定档＋落码（`DEVICE_SAFE_NODES` 恰 7＋`ROOT_MAKE_GRANT` 恰 5 位 make 子集＋三段规则＋单一 helper）＋权限集钉×2/表钉/一致性钉/e2e 探针集；读数＝Windows tools 2990/0/6・Linux 全套件绿＋e2e ABI v3 真机全过（明细第二卷 §1.95）。**同批审查处理（2026-10-01，设计/实现/符合性三面通过、读数独立复现）**：P2×1＝README 五处未提交改动未入落点→§1.95/索引头行补账；P3×4＝terminal.rs doc 拼接归位・「23+3」记法勘正（Windows 实为 23/0，三钉 Linux-gated）・e2e 补表外块设备写拒探针（上项「`/dev/sd*` 类仍不可写」条款的字面闭环：root 门＋块设备形态门、仅守卫臂——对照臂真写盘面＝灾难不设；容器无块设备节点＝skip）・恢复臂断言收敛 root 分支防假红；补证＝Linux `cargo check -p orz-tools --lib` exit 0（cfg(linux) helper 编译证据）。
- [x] **S3 重建进体**（2026-10-01，144 批）：**0.8.8 双平台重建进体**——源冻结 orz **`2da7dba0`**（`60b21a37` S2＋审查处理＋bump；0cd 随行进体）；Windows `build_orz.ps1 -Release` exit 0（2m49s）换装 `D:\tb-eval\orz-windows\` MATCH 3/3＋回滚点 `.0.8.7-bak`＋ACAF 重 provision exit 0 绑 `f651f7ba…`（keystore 两件逐位未动）；Linux docker `rust:1.97-slim`（ORZ-BUILD-MOUNT-001）直写换装位 MATCH 3/3（`orz 874df6ca…`）static-pie×3＋INTERP=0＋alpine/bookworm 双冒烟 0.8.8；进体判据＝Linux 件 `DEVICE_SAFE_NODES` 全表 7 项可达（Python 逐字节核证：zero/full/ptmx 0→1、urandom 1→2、`/dev/` 族 +6；Windows 件 cfg(linux) 死引用消除如实记）＋Windows 版本串滚动＋五规则 id 与保底文案零变化；身份门 `EXPECTED_CARRIER_SHA256` `3332b38f…`→`874df6ca…`（适配器未动）；rel-144-stage 双包 6/6 MATCH＋容器核证 zip 4/4・tar 4/4・顶层 2/2＋包内 build-info 0.8.8＋清单活体两态（干净 0 finding／README+1B 恰 1 条）；未推送未发行。注＝0ce 暂缓、0cf/0cg 未实施无代码随行，三者落点顺延 0.8.9；明细 [`144 批档`](docs/audits/144_CARRIER_REBUILD_V088_0CH_S3_2026-10-01.md)。
- [x] **S4 重跑复验＋闭合**（2026-10-02 用户裁决「证据足够了那0ch就可以进行闭合了」闭合，57 → 56，151 批）：`configure-git-webserver`／`caffe-cifar-10`／`git-multibranch` 三题同为 `/dev` 摩擦重灾面；闭合 **57 → 56**。**首题已跑（2026-10-01，145 批，用户令「找一道目前指令被拒绝，摩擦最大的题目来跑就行」）＝configure-git-webserver**（作业 `official-v41-rerun3-configure-git-webserver-0chs4`，`--job-suffix` 保 rerun3 留档；0.8.8 身份门 `874df6ca…` 过）：reward **0.0**（官方 900s 硬杀，check PASS 零墙钟泄漏）——**但 0ch 两目标摩擦实证消除**＝零改系统二进制（rerun3 同题 sed 4 处：apt-key/sshd/nginx/ucf）＋`/dev` 写拒 0＋`mkdir -p /git` 与写/删轮内亲测全过。**新露第三摩擦＝用户裁决落 0ch 已接受边界注记〔已知代价〕不立项不放行**：apt 装 git/openssh 时 `libcryptsetup12` 解包报 `unable to install new version of './lib': Permission denied`（镜像 `/lib -> usr/lib` 根级符号链接替换需 `/` REMOVE/rename 权＝`ROOT_MAKE_GRANT` 刻意排除面、规则 1 威胁模型本体；差分定因＝无守卫同镜像 rc=0 即框架侧；非 0ch 回归）；模型绕行面（瘦装 git nginx 避开 systemd 链）未尝试、转检索兔子洞烧尽余时；`/zlink_at_root` 查明为模型自身 rename 探针残留。明细 [`145 批档`](docs/audits/145_0CH_S4_FIRST_RERUN_V088_2026-10-01.md)。**余＝`caffe-cifar-10`／`git-multibranch` 两题**。**余两题已跑（2026-10-02，150 批，用户令「请进行0ch剩余两题重跑吧」）＝双 1.0 翻盘（0.8.9 在役，身份门 `4e35f410…` 过）**：caffe-cifar-10（`official-v41-rerun3-caffe-cifar-10-0chs4`）reward **1.0 self-completed**（29.3 min，run `RUN-CLI-6abe9839`；rerun3 同题 0.0 硬杀）＋git-multibranch（`official-v41-rerun3-git-multibranch-0chs4`）reward **1.0 self-completed**（15.1 min，run `RUN-CLI-6abe9f15`；rerun2 同题 0.0）——两题 `[rerun3-check]` PASS 零墙钟泄漏；`/dev` 零摩擦＋根级 make 实战（`/git` 建仓）＋零改系统二进制；保底保留实证（caffe 写控 block 3＝.gsa×2＋keystore 祖先臂×1，绕行后收官）；残余三点不立项（dpkg usrmerge 已接受边界／pty 拒非 0ch 回归／hook cp 未定谳）。明细 [`150 批档`](docs/audits/150_0CH_S4_REMAINING_TWO_RERUNS_V089_2026-10-02.md)。**S4 三题全过，闭合（57 → 56）已随 151 批用户裁决执行**（[`151 批档`](docs/audits/151_0CH_CLOSURE_2026-10-02.md)）。
- 不做面：规则 1 根级递归删除与规则 5 宿主状态两目标仍由 L1/L2 承载；不做「全放开再靠事后追责」。不动面：L1/L2 五条 block 枚举、schema v0.3、判官／Python 镜像、身份门与评测口径零变更。
- 入口：BACKLOG `0ch` / 第二卷 §1.92 / 代码入口 `orz-tools::write_control::LINUX_DISASTER_KERNEL_FACES`＋`orz-sandbox::child_write_guard`＋`orz-tools::types::exec_policy`；上游 0cc／0cb。


### P1-0ci Frontier-Bench 试点线（Harbor 同管线第二证据轴）（P1；2026-10-02 用户裁决立项，BACKLOG 0ci；56 → 57；来源＝[`152 批档`](docs/audits/152_FRONTIER_BENCH_SLOPCODEBENCH_ADJUDICATION_2026-10-02.md)＋用户裁决「Frontier-Bench 顺位跑」；TB 2.1 成绩线随同批裁决收官）

> 定位＝与 TB 2.1 同 Harbor 管线的第二证据轴（74 环境确定性 oracle；k=1 筛查口径）。诚实边界＝`frontier-bench` 数据集仓库 pin 未钉死（152 批档 §7）——S0 未过不得出冻结清单。

- [ ] **S0 勘定**：数据集仓库与版本 pin 钉死＋许可与任务清单核证＋与 TB 2.1 装置差异清单。
- [ ] **S1 冻结**：试点题集 manifest（先 10–15 题子集；全 74 整轮另行裁决；P2-15 冻结纪律同形）。
- [ ] **S2 适配核证**：`tb_agents/orz.py` 透传核证＋驱动脚本平移（`run_official_2.1.sh` 形态）。
- [ ] **S3 冒烟**：1–2 题真机冒烟＋读数收取面核证。
- [ ] **S4 试点轮**：k=1 串行，逐题 `result.json` 机械提取，报告档落档后回查闭合门。
- 追加裁决（152 批第二轮）：读数解读以错题解剖与缺陷提取为主、分数仅作评判参考（「用题目找bug和不足才重要」）。
- 入口：BACKLOG `0ci` / [`152 批档`](docs/audits/152_FRONTIER_BENCH_SLOPCODEBENCH_ADJUDICATION_2026-10-02.md)。

### P1-0cj SlopCodeBench 挑题摩擦探针线（RLI 观测场）（P1；2026-10-02 用户裁决立项，BACKLOG 0cj；57 → 58；**2026-10-02 用户裁决收口闭合（60 → 59，161 批补记）＝负结果如实闭合**；来源＝[`152 批档`](docs/audits/152_FRONTIER_BENCH_SLOPCODEBENCH_ADJUDICATION_2026-10-02.md)＋用户裁决「先挑题并且作为摩擦任务来跑」「可以用来观察 RLI」）

> 定位＝摩擦任务线（`0w` 同形：挑题、非整轮、不追分）；长程迭代任务＝RLI 观测场；兼作 RLI 模型可见性验证场。只观测不改造（RLI 数学归 `0am`，挂起）。

- [x] **S0 勘定**（158 批达成）：仓库/DOI/判分器核证＋agent 接入契约核证＋挑题准则定稿。**三件全核证**：①双仓 pin（harness `31ceea3a`／题目 `38d627ec`）＋DOI＋36 题/196 检查点吻合＋pytest 确定性判分（LLM judge 仅质量参考）；②接入契约定形＝容器内常驻 `orz --stdio` ACP 多轮（`-p` 新会话不适用），RLI 会话级跨 run 累积＝观测面成立；③准则 C1–C5＋Hard 池 12 题排序（首推 recli 备选 sith）——首题圈定随 S1 冻结留用户裁决（[`158 批档`](docs/audits/158_0CJ_S0_SURVEY_2026-10-02.md)）。
- [x] **S1 挑题冻结（首题先行）**：**recli**（用户令「就按照你的判定，跑recli吧」＝Hard 池最长跑道×综合面×仅 pyyaml）；**冻结口径＝1 题**，3–5 题全 manifest 留首跑读数后一并冻结（偏差如实登记，159 批）。
- [x] **S2 适配器**：orz 无头模式包装 SCBench agent 契约（新写，非 Harbor 系）——**达成（159 批）**：rig clone 落 `agents/orz/`（Agent ABC 六方法＋ACP 驱动）＋四配置件；形态＝容器内常驻 `orz --stdio --real` 多轮（同会话跨 checkpoint＝RLI 全题长程观测面）；watchdog cancel 兜底（0.8.10 无 ORZ_MAX_WALLCLOCK）；ACAF 容器内 provision；harness Windows 缺陷修正四处（run id 共享会话前缀 `65f2858a`＝同会话实证）；ACP 冒烟全绿。
- [x] **S3 摩擦探针轮**：逐题真机＋RLI 读数随轮收取（消费率／阈值触发／读数-转向相关）＋摩擦台账。**首题探针轮达成（159 批终局读数）＝recli 8/8 checkpoint 单会话跑完**（86 min、155 轮模型、LIF 域迁移累计 109 次、上下文压缩 10 次）；**strict pass 4/8**（ckpt1/2/3/8 CORE 全过），轨迹完整呈现扩展→侵蚀（80%→20%→0%）→反弹（40%→86%）〔2026-10-02 更正：官方口径 strict＝3/8（ckpt8 6/7 非全过）、「反弹」废止——累计回归面单调恶化至 61%，见 0CJ 报告 §8〕；lint 错误 135→1992 单调膨胀；**RLI 消费率＝0**（注解单臂未抬升＝判据①首笔真机数据；0ck S4 注解可见性核证仍待做）；阈值推面真阴性（全 audit 流扫描无 rli 痕迹，对照 0bf S3 两触发＝本轮未越阈；**0co 更正 162 批：全程并非零触发——ckpt_8 slow StreakCrossed×3〔未投递〕，侵蚀窗内零触发成立**）；**时间元认知维持者判读＝budget/temporal resident 行族而非 RLI**（budget「已用 N/999 轮」×136 每批刷新）。**模型身份勘误＝名义 v4-flash 后端自动路由 v4.1（用户澄清），全部读数属 v4.1**；报告档＝[`0CJ_RECLI_FIRST_PROBE_REPORT_2026-10-02.md`](docs/audits/0CJ_RECLI_FIRST_PROBE_REPORT_2026-10-02.md)。**全 manifest（3–5 题）扩展随收口不执行**（161 批补记；如需另立）。
- [x] **S4 报告＋RLI 可见性判据**（2026-10-02 收口闭合，用户令「0cj可收口」，161 批补记 §7.4，60 → 59）：判据①「消费率 >0」未达成（消费率 0；0ck S4 可见性核证补齐后两层判定闭合＝**可见但不用**——失败模式＝无体感需求，非注解丢失）；判据②「读数在场轮转向变化复算」自然空转（消费 0 ⇒ 无样本，如实登记不补做）；**线定位（RLI 观测场）达成**＝观测基底移交 0co/0am（快照 rli×87／u_err×20／domain×385／spike×4＋10 run journal 域迁移 109）；第三层注入式维持出局（152 批档 §5）。
- 追加裁决（152 批第二轮，定案）：注解先行（已单独立项 0ck，措辞定稿并落码〔153 批〕）；注入式不再增加（第三层出局——注入必牵引模型对整体时间与进程的注意力；0bd ⑦ 在役面维持不扩展）。
- 入口：BACKLOG `0cj` / [`152 批档`](docs/audits/152_FRONTIER_BENCH_SLOPCODEBENCH_ADJUDICATION_2026-10-02.md) §5。

### P1-0ck RLI 模型面注解（`blackboard_read` rli 域一句使用指引）（P1；2026-10-02 用户令单独立项＋落码，BACKLOG 0ck；57 → 58；**2026-10-02 用户裁决闭合（61 → 60，161 批补记）＝`pending` → `implemented`**；来源＝[`153 批档`](docs/audits/153_RLI_TOOL_ANNOTATION_2026-10-02.md)＋152 批 §9.2＋用户令「先把RLI注解做了，然后再进入新一轮验证，请进行单独立项」；**执行形态＝单独立项单独批〔0bn 同形〕，不并线 0cj**）

> 定位＝0cf 同构（面在役、模型不知晓 ⇒ 0bf S3 消费率 0/3）；措辞约束＝一句、精简、简单明了（152 批 §9.2）；注入式不加（第三层出局）。

- [x] **S1 落码**（2026-10-02，153 批）：定案措辞（用户令修订后定稿，无批次号考古）＝`如需时间/动作连续性与节奏压力走向等信息，请查看 RLI 参考面（section=rli）。`——插于 rli 分区机制括注之后、0cf guide 句之前；单来源＝`orz-loop` controller.rs `blackboard_read` description（全仓唯一、无测试钉原文）。
- [x] **S2 测试面**（2026-10-02，153 批）：触碰面 fmt 零 diff（controller.rs rustfmt --check 干净；全仓 fmt 漂移在案不拒批）＋ `cargo test -p orz-loop --lib` 终态 847 过／0 挂全绿（首跑 1 挂＝`user_cancel_closes_pending_activations_before_run_cancelled` 既存 **flaky**〔连挂 3 次后整套复跑自愈〕，stash 差分定因干净 HEAD 同挂＝与 0ck 无因果，登记留裁决不扩批）；clippy stash 前后 73/73 行＝新增 0。
- [x] **S3 重建进体**（2026-10-02，155 批）：0.8.10 双平台重建换装（源冻结 orz `55d61c47`）；字节判据＝RLI_ANNO 0→5 两平台（**初建件判据拦截 0cl 瘦身丢 0ck 注解句＝`55d61c47` 修复重build**）＋Linux MATCH 3/3＋static-pie×3＋alpine/bookworm 双冒烟 0.8.10＋ACAF 重 provision `b4b5d87b…`＋身份门 `4e35f410…`→`4ed531e8…`；Windows fake-provider 整轮冒烟 rc=0。明细 [`155 批档`](docs/audits/155_CARRIER_REBUILD_V0810_0CK_0CL_S3_2026-10-02.md)。
- [x] **S4 真机核证**（2026-10-02，161 批）：注解在模型面可见＝活体探针逐字引出通过（注解原句＋section 枚举 rli 条目＋selector 注记与源码一致）；消费率读数随 0cj 收取＝0（对照基线 0/3 同值）。**两层判定闭合＝可见但不用（无体感需求，非注解丢失）**。**0ck S1–S4 全达成、闭合待用户裁决**（[`161 批档`](docs/audits/161_0CK_S4_VISIBILITY_AND_0CN_0CO_ADJUDICATION_2026-10-02.md) §2）。
- [x] **闭合**（2026-10-02 用户裁决「0ck可闭合」，161 批补记 §7.2，61 → 60）：`pending` → `implemented`；「可见但不用」两层判定存档；S2 既存 flaky 观察归测试面另案不随 0ck。
- 入口：BACKLOG `0ck` / [`153 批档`](docs/audits/153_RLI_TOOL_ANNOTATION_2026-10-02.md) / [`152 批档`](docs/audits/152_FRONTIER_BENCH_SLOPCODEBENCH_ADJUDICATION_2026-10-02.md) §5/§9。

### P1-0cl 黑板模型面瘦身（`blackboard_read` 描述＋guide 说明书简明化）（P1；2026-10-02 用户令立项＋落码，BACKLOG 0cl；58 → 59；来源＝[`154 批档`](docs/audits/154_BLACKBOARD_FACE_SLIMMING_2026-10-02.md)＋153 批 §4＋用户令「请先进行黑板瘦身吧，说明书也要简明扼要，可以做成框架内部名词解释和组件关系交代」；与 0ck 同族、单独立项单独批）

> 定位＝blackboard_read 描述 5,593 字符＋10＋ 处批次考古引用（153 批 §4 实锤）；说明书按用户定形态重写＝名词解释＋组件关系。

- [x] **S1 落码**（2026-10-02，154 批）：主描述 5,593→**2,492** 字符、批次引用→0；十条参数描述去考古；enum 补 `journal`/`notes`＋补声明 `anchor`（模型面与派发器一致性修复）；guide 重写【名词】＋【组件关系】**621 字符**；测试钉同步（blackboard.rs）。
- [x] **S2 测试面**（2026-10-02，154 批）：触碰面 fmt 零 diff（controller.rs＋blackboard.rs）＋ `cargo test -p orz-loop --lib` **847 过／0 挂**全绿。
- [x] **S3 重建进体**（2026-10-02，155 批）：0.8.10 双平台重建换装（与 0ck 同窗）；字节判据＝GUIDE_NOUNS/ANCHOR_PARAM 0→5＋OCF_OLD_GONE（0cf 英文旧句）5→0＋enum（journal/notes）/anchor 进体＋保留面零回归（`/dev/null` WIN 3→3・LIN 46→46）；Linux MATCH 3/3＋static-pie×3＋双容器冒烟 0.8.10。明细 [`155 批档`](docs/audits/155_CARRIER_REBUILD_V0810_0CK_0CL_S3_2026-10-02.md)。
- [ ] **S4 真机核证**：guide 可读、journal/notes/anchor 可用。
- 入口：BACKLOG `0cl` / [`154 批档`](docs/audits/154_BLACKBOARD_FACE_SLIMMING_2026-10-02.md)。

### P1-0cn budget 注入处置（BUDGET-INJECTION-DISPOSAL）（P1；2026-10-02 用户裁决立项，BACKLOG 0cn；59 → 60；来源＝0CJ 报告 §3.1 判读＋用户令「预算注入不应该留啊，999也只是一个占位值而已，这个要处理掉，请立项吧」）

> 定位＝`mechanical_audit_update kind=budget`「已用 N/999 轮」注入行处置——recli 轮实测 136 条（每工具批一刷新）＝当前模型轮次感的**事实主来源**（0CJ 报告 §3.1）。

- [x] **S1 设计定稿**（2026-10-02 用户裁决「0cn的话，就是单纯不传这个轮数记次了，这个注入撤销」，161 批补记 §7.3）：三选一定为**撤除**——不真值化（999 占位值不接真实预算语义）、不并入 temporal 面重构；**不设替代物、接受时间感回归**（张力解除——轮次感改由 temporal／域迁移行族承载）。
- [x] **S2 落码**（2026-10-02，164 批；ADR-0010 §14.82/v1.83 转录同批）：①`agent_loop` 逐批 `kind=budget` 事件写入删除（同块 `lif_domain` 连带记录保留）；②`mechanical_audit.rs` `record_budget()` 删除＋报告块预算行退役（墙钟行保留）＋`KIND_BUDGET` 标退役（枚举/键保留仅供历史 journal 校验）；③orz-bin 三处 e2e 期望双条→单条；④schema/Python 镜像 docstring 标注退役（枚举/键集零变化、fixture 保持合法）；⑤budget 零写入钉子。**边界**：session 面轮预算 PULL（真值）与 `budget_insufficient` 硬门不动。**验证边界**：本会话 `protoc` 缺失＋用户令「暂时不重建」⇒ 编译/测试未跑、S3 进体暂缓；改动区 fmt 自查干净；构建/测试随 S3 进体批补跑。
- [x] **S3 进体**（2026-10-03，170 批 0.8.11 双平台载体重建；源冻结 orz `6493fdae`）：Windows/Linux release 重建零告警 rc=0〔WIN `build_orz.ps1 -Release -Jobs 2` 25m07s／LIN musl cargo 42m08s〕＝**164 批登记的构建验证补跑随批闭合**；字节判据 `预算：`2→0〔报告块预算行退役落件〕＋`墙钟约`1→1 保留＋回归面零漂移；换装 MATCH 3/3×2＋`.0.8.10-bak` 链；冒烟 WIN 整轮＋LIN 双绿；[`170 批档`](docs/audits/170_CARRIER_REBUILD_V0811_0CP_S3_0CN_S3_2026-10-03.md)。
- [x] **S4 真机核证**（2026-10-03，171 批 recli 二跑）：`kind=budget`＝0＋`预算`字串＝0（9 卷 journal 覆盖式扫描；首跑 136→0）；轮次感改由 lif_domain 98＋rli_notice 37＋context_scale 8 承载；`墙钟约` 保留面＝报告块 face（bench 形态两轮均不渲染，保留性由 170 批字节判据覆盖，非回归）；同题同源（ckpt prompt 逐字节等）。[`171 批档 §4`](docs/audits/171_0CP_S4_0CN_S4_RECLI_SECOND_RUN_2026-10-03.md)。**0cn S1–S4 全达成、闭合待用户裁决**。
- 入口：BACKLOG `0cn` / [`0CJ 报告`](docs/audits/0CJ_RECLI_FIRST_PROBE_REPORT_2026-10-02.md) §3.1 / [`161 批档`](docs/audits/161_0CK_S4_VISIBILITY_AND_0CN_0CO_ADJUDICATION_2026-10-02.md) §3。
- [ ] **闭合**：S1–S4 全达成 ⇒ `pending` → `implemented`（与 0cp 同批用户裁决，两项合计 59 → 57）。

### P1-0co RLI 触发阈值数据评估（RLI-TRIGGER-THRESHOLD-EVAL）（P1；2026-10-02 用户裁决立项，BACKLOG 0co；60 → 61；**2026-10-02 用户裁决 S3 闭合（59 → 58，163 批）＝`pending` → `implemented`**；来源＝用户令「没越界就不推是设计明确的……先用本轮数据算算RLI组件的预测情况再说，或许应该调整触发阈值，一切看具体数据，这一部分也请额外立项」）

> 定位＝用 recli 首跑数据离线复算 RLI 组件预测表现，据数据评估 0bd ⑦ 触发阈值；只评估与建议，不动在役面（改动随裁决另行立项）；RLI 数学归 0am（挂起）。

- [x] **S0 数据提取与复算管线**（2026-10-02，162 批）：数据源在案并全部解析＝checkpoint_8 活体会话档 `conversations/65f2858a.json` 的 `lif.rli_shadow`（终态权威）＋归档冻结副本（ckpt6→7 边界）＋逐 checkpoint 8 时点演化序列＋8 run journal（2,381 事件）；刺激语义提取（slow＞60s／stall＞90s／k=5／10s 网格补点占采样 49.6%）。逐 checkpoint 演化表见 [`162 批档 §2`](docs/audits/162_0CO_S0_S2_RLI_THRESHOLD_DATA_EVAL_2026-10-02.md)。
- [x] **S1 预测表现报告**（2026-10-02，162 批）：**阈值推面并非零触发**——侧车队列 StreakCrossed×3（全 slow 通道、ckpt_8 恢复期、全部 delivered=false）；**侵蚀窗（ckpt 4–6）四压力通道全静默＝真阴性成立**（归档副本独立证实）；3 连发全部为单工具段内网格补点累计（73–99s 成功 build/test）＝对质量无判别力；更正 0CJ 报告 §3.2「0 触发」判读（journal 扫描对提醒面结构性盲区）。明细 [`162 批档 §3`](docs/audits/162_0CO_S0_S2_RLI_THRESHOLD_DATA_EVAL_2026-10-02.md)。
- [x] **S2 阈值调整建议稿**（2026-10-02，162 批）：提案 A＝streak 单工具段免疫（网格补点不重复累计）／B＝k→8（次选）／C＝θ 衰减地板（无实证样本、低优先）／deny 评测口径豁免；不建议调整 θ 初值/η/q/周期（无归因样本）；「侵蚀不可见」属刺激面语义缺口移交 0am 转正裁决。明细 [`162 批档 §4`](docs/audits/162_0CO_S0_S2_RLI_THRESHOLD_DATA_EVAL_2026-10-02.md)。
- [x] **S3 用户裁决**（2026-10-02，163 批，59 → 58）：提案 A 升格根本化（采样层整体动作触发式、网格退役）／新增卡死看门狗时间样 181s（TER 180s＋1s）／**k 5→3**（异常累积值语义不变；上轮调研明确 5 过高）／**附注式直投采纳**（fire 当刻注入行）——整体落地为 **0cp**（设计稿 v1.0）；残差观察两项（θ 地板／deny 口径豁免）未裁决留档不占计数。[`163 批档 §1`](docs/audits/163_0CO_S3_CLOSURE_AND_0CP_PROJECT_2026-10-02.md)。
- 入口：BACKLOG `0co` / [`0CJ 报告`](docs/audits/0CJ_RECLI_FIRST_PROBE_REPORT_2026-10-02.md) §3.2 / [`161 批档`](docs/audits/161_0CK_S4_VISIBILITY_AND_0CN_0CO_ADJUDICATION_2026-10-02.md) §4。

### P1-0cp RLI 动作触发式采样与提醒附注式直投（RLI-ACTION-SAMPLING-NOTICE-PUSH）（P1；2026-10-02 用户三裁决立项，BACKLOG 0cp；58 → 59；来源＝162 批 0co 数据＋用户令「干脆根本性调整，变成动作触发式采样呢？每个动作是一个采样，而每个动作天然携带不同特征」「k……下调至3，并且改为附注直投」「无动作样且模型流式传输停滞后3分01秒后触发一个时间采样」；单独立项单独批，与 0cn S2 相邻但独立不并批）

> 设计权威＝[`RLI_ACTION_SAMPLING_NOTICE_PUSH_DESIGN`](docs/RLI_ACTION_SAMPLING_NOTICE_PUSH_DESIGN.md) v1.2（163 批 v1.0；166 批 D6 注解格式；167 批 D7＝spike 即提醒＋域稳定切换提醒二合一〔统一域事件提醒族：spike 进入端即提醒、回归不提醒，行含 T̂＋动作特征＋域趋势；实证＝稳定确认 0 次/27 spike 全滤，提醒面事实死亡〕）；五通道数学/θ 自校准/繁杂度/λ̂/域机器/T̂ 全不动（0am 线）。

- [x] **S1 设计定稿**（2026-10-02，163 批；v1.1 补订 D6〔166 批〕；v1.2 补订 D7〔167 批，用户裁定「单独使用域稳定切换再提醒看来还是太理想了……不能假设现实任务的实际处理情况能够被分出域特征」〕）：**D1** 采样层动作化（streak/锚点采样只绑决策轮＋工具完成事件；`fill_grid_gaps`／`RLI_GRID_SECS`／`RLI_GRID_FILL_MAX` 退役）；**D2** 卡死看门狗（`RLI_WATCHDOG_SAMPLE_SECS=181.0`＝TER 首报后台化 180s＋1s 避让；无动作样∧流式停滞→恰一个时间采样，动作样重武装）；**D3** `RLI_STREAK_K` 5→3；**D4** fire 当刻附注式直投（mechanical_audit 注入行、journal 自动留痕＝0co 盲区闭环；队列转事件历史）；**D5** 152 批 §5.2 局部取代声明（fire 附注放行、周期读数注入维持出局；ADR 转录随 S2）。
- [x] **S2 落码**（2026-10-03，168 批；本会话用户令「请进行0cp S2吧」放行）：rli.rs（D1 网格退役＋动作样两源、D2 看门狗 `RLI_WATCHDOG_SAMPLE_SECS=181.0` 三同时恰一样后休眠、D3 k=3、D4 `take_pending_for_push` 队列转事件历史、D6 `RLI_T_HAT_ANNOTATION`＋趋势段、D7 `DomainSpikeEntry` 回归不提醒＋spike 计数随快照）＋controller（lif Arc 化＋`lif_shared`／`take_pending_rli_notices`＋pull-delta 头提醒段退役＋rli.now 面头同步）＋agent_loop（D4 主车道轮末附注直投「RLI提醒: …」＋`mechanical_audit_update{kind:rli_notice}` journal 行；D2 `spawn_rli_watchdog` 5s 级任务随 run 守卫）＋契约面（schema/Python 镜像 `rli_notice` 第 10 kind＋键形 `rli.notice.<kind>`＋互证钉）＋ADR-0010 §14.83（v1.84，D5 局部取代声明＋§14.81 关系）落卷。验证＝orz-assurance 279/0＋orz-loop 847/0＋orz-bin 60/0＋clippy 差分 −1＋fmt 触碰面零新增；§5 七判据钉映射见 [`168 批档 §2`](docs/audits/168_0CP_S2_ACTION_SAMPLING_DIRECT_PUSH_IMPL_2026-10-03.md)。
- [x] **S2 审查处置修正**（2026-10-03，169 批；用户令「请对审查出的全部问题进行处理」，对象＝168 批三维度全面检查）：**P1** 看门狗时间基错位修正——`spawn_rli_watchdog` 曾把 epoch 秒直传影子（窗口①恒真＋通道湮灭＋影子永久冻结＋冻结态反噬误导性域提醒），改 `LifEngine::on_rli_watchdog_tick` 唯一宿主转轴入口（epoch→run 相对轴）＋接缝轴判别钉；**P2** D7 回归分类勘误——回归端＝凡异常域→Normal（Pressure→Normal 不再误报「spike进入」），新钉 `domain_pressure_to_normal_counts_return_not_entry`；**P3** `take_pending_for_push` 谓词直取化（消除队列序不变量依赖）＋两投递会计方法冻结面注记；ADR §14.83 第 7 项就地勘误＋主文件 v1.85。验证＝orz-assurance 281/0＋orz-loop 848/0＋clippy 106 持平；[`169 批档`](docs/audits/169_0CP_S2_REVIEW_FIXES_2026-10-03.md)。
- [x] **S3 进体**（2026-10-03，170 批 0.8.11 双平台载体重建：源冻结 orz `6493fdae`＝164＋168＋169 三落码批；Windows `build_orz.ps1 -Release -Jobs 2` 25m07s 零告警 rc=0、Linux musl `rust:1.97-slim` cargo 42m08s rc=0；换装 MATCH 3/3×2 平台＋`.0.8.10-bak` 链；字节判据＝0cp 新面全落位〔`域事件: 稳定确认`/`spike进入`/T̂ 注解 0→1、`rli.notice.` 0→7〕＋`域迁移确认`1→0＋`RLI提醒`2→0＋0cn `预算：`2→0＋回归面零漂移＋残留 0.8.10=0；ACAF 重 provision `9f1e2889…` 绑新 signer；身份门 `4ed531e8…`→`c6a0812c…`；ELF static-pie×3；冒烟 WIN 整轮 `RUN-CLI-6ac00bd6`＋LIN alpine/bookworm 双绿；164 批登记的构建验证补跑随批闭合；[`170 批档`](docs/audits/170_CARRIER_REBUILD_V0811_0CP_S3_0CN_S3_2026-10-03.md)）。
- [x] **S4 真机核证**（2026-10-03，171 批 recli 二跑；判据 2 看门狗经评估对话定测试钉口径——181s 真卡死面正常任务不可达〔TER 180s 后台化精确避让〕，设计原注「真机可选」）：①动作样累计 ✓（320 采样全动作源、网格退役零残留、唯一 >60s 命令段内零采样、并行批边界样本深度全 0.0s）；②看门狗＝真机未触发（无卡死、0 时间样＝预期，测试钉覆盖）；③k=3 ✓（2 次 fire 均 ×3）；④直投 ✓（37 fire＝37 journal 行同刻装配〔tool_result 后 5–14ms〕＋`notice_delivered_total=37` 恰等＋fire 全集 journal 可复算）；⑤回归 ✓（零动作周期零注入、五通道在算）；⑥D6 ✓（37 行 186–228B≤240 带参数含义＋趋势）；⑦D7 ✓（spike 13/13 全提醒、回归端零专用提醒＝提醒面复活）。[`171 批档 §3`](docs/audits/171_0CP_S4_0CN_S4_RECLI_SECOND_RUN_2026-10-03.md)。**0cp S1–S4 全达成、闭合待用户裁决**。
- 入口：BACKLOG `0cp` / [`163 批档`](docs/audits/163_0CO_S3_CLOSURE_AND_0CP_PROJECT_2026-10-02.md) / [`162 批档`](docs/audits/162_0CO_S0_S2_RLI_THRESHOLD_DATA_EVAL_2026-10-02.md)。
- [ ] **闭合**：S1–S4 全达成 ⇒ `pending` → `implemented`（与 0cn 同批用户裁决，两项合计 59 → 57）。

### P1-0cm 对外英文翻译线（Reddit 征求建议用英文门面件）（P1；2026-10-02 用户裁决立项，BACKLOG 0cm；59 → 60；来源＝2026-10-02 会话三步收敛＋用户令「当前新增0cm」；charter＝[`EN_TRANSLATION_PROJECT_CHARTER_2026-10-02.md`](docs/EN_TRANSLATION_PROJECT_CHARTER_2026-10-02.md)）

> 定位＝为 Reddit 发帖征求建议与纠正产出最小英文门面集（五件落 `docs/en/`）；质量靠对照表注入＋表格数字照抄＋档头声明，不依赖人工逐句审查。只出稿，发帖动作与最终措辞留用户。

- [x] **S1 对照表落盘（v0.1）**：charter 附录 A 种子扩充至 52 行 63 词条＋EN↔CN 文档映射，落 `docs/en/TRANSLATION_GLOSSARY.md`（术语权威）；用户增删定稿（v0.1→v1.0）待续（157 批；**同日复审勘正 62→63**）。
- [x] **S2 README 英译出稿**：`docs/en/README.en.md` 全译落盘（v0.8.10 发布面钉版，链接改指 `../../`，中文内部档标注 Chinese）；**用户过目待做**（157 批）。
- [x] **S3 设计当前态蒸馏英译**：`WRITE_CONTROL_CURRENT_EN.md`（v4 规则面＋v1 存续架构）＋`ACAF_CURRENT_EN.md`（含 0.8.2/0.8.7 在役变更）落盘（表格照抄＋声明前置，无人工读数）（157 批）。
- [x] **S4 跑分报告英译**：`TB21_V41_89_FULL_ROUND_REPORT_EN.md` 全译落盘（89 行表逐格照抄，仅翻表头与形态/备注两列）；可选发帖草稿未产出（留用户）（157 批）。
- [x] ~~**D6 总纲整合件**~~（同日增补→同日并入 D2＝用户令「把英文版总纲和英文版readme合起来，英文版readme细致些」＋「中文版恢复原样」：英文 README 扩展版吸收总纲＋安全章，`CURRENT_DESIGN_EN.md` 删除退役，五件口径恢复；中文 README 恢复原框架介绍）（157 批 §8/§10）。
- [x] **收尾批（0cm 闭合门）**：D2 过目反馈已应用（六处修订＋双版评测移除＋README 精简→再裁决反转〔中文恢复原样；英文扩展版＝总纲并入 README 细致化〕，157 批档 §6/§9/§10）；D1 增删定稿（v0.1→v1.0）＋D2 后续轮次＋可选发帖草稿＋发帖后纠错回灌待续；**2026-10-02 用户裁决定案（60 → 59）**＝D2 过目定案＋D1 冻结 v1.0＋`docs/en/` 五件推送 `origin/main`（157 批档 §15）。
- [x] **中文 README 增补真机安全设计章**（同日再裁决）：`README.md` 在「开发者入口」前新增 `### 真机安全设计`（四层＋已知边界，链接指中文权威档）；同日再同步＝中文侧删「Bing 家族已出集」句，中英内容互为镜像、唯一差别＝英文版总纲（157 批 §12/§13）。
- 边界：不翻译 docs/ 全量与台账/审计；不动 `orz/README.md`；不承诺逐句等价；不新造英文品牌名。
- [x] **英文五件末轮全量审查**（同日用户令「请最后对全部的英文版文件做最后一轮审查」）：五件逐件复审＝链接 0 断／D3 数值零漂移（89 题表六列＋哈希计数）／D4·D5 常量与事实回查源码；英文侧 5 处措辞勘正＋中文权威侧 `rmtree` 漏列登记留中文批；**发帖渠道定案 Reddit**（linux.do／V2EX／NodeSeek 排除）（157 批档 §14）。
- 入口：BACKLOG `0cm` / [`charter`](docs/EN_TRANSLATION_PROJECT_CHARTER_2026-10-02.md) / [`156 批档`](docs/audits/156_SUBMIT_PUSH_AND_RELEASE_V0810_2026-10-02.md) §5。

### P1-0bn 必定压缩复审补口（单独进行，不进狗粮轮）（P1；2026-09-24 0bm 复审批立项，BACKLOG 0bn；来源＝[`073 复审`](docs/audits/073_UNCOMMITTED_REVIEW_AND_REMEDIATION_2026-09-24.md) R1/R2；设计权威＝ADR §14.77 复审补记＋v8 §15；**执行形态＝单独批次（2026-09-24 用户令「0bn 是单独进行的内容，不进狗粮轮」）**）

> 073 复审两处 P2 单独立项；用户裁决采纳主代理「就地小修」标准解，被否备选（从上下文预算豁免／常驻化——破零常驻注入纪律、不解决停摆本体、牵连「仅留主滑块」语义）留档。

- [x] **S1 设计补充**（2026-09-24 本批完成）：两边缘定案——① **升级链停摆**：强制窗收口且压缩未真正落地（摘要已产出但区间未识别／无可压块／台账失败）⇒ 同计为「未产出」，计入升级计数并复位 T1 闩（「摘要产出 ≠ 压缩达成」）；② **强制窗块自嵌模板**：摘要格式模板（含 ≥2 小节要求）内嵌块本体，不依赖早前注入。
- [x] **S2 落码＋钉子**（2026-09-24 本批）：实施＋钉子（① **未落地升级钉** `mandatory_window_summary_that_fails_to_land_still_escalates_and_does_not_stall`——强制窗产出摘要但区间未识别 ⇒ 未落地同计「未产出」：`t1_window_failures` 推进到 1＋复位 T1 闩 ⇒ 第二次询问照常出现 ⇒ 第三次机械截断收敛（700K 守卫抬到不可达，证非兜底代打）；未落地事实落账 `context_scale:mandatory_summary_not_landed` 不静默；② **自嵌模板钉** `mandatory_window_block_embeds_the_summary_template_self_contained`——两级询问块含模板全形与「至少命中 2 个小节」识别门槛，`（见上）`指称退役；③ 既有三步全流程钉与 700K 守卫钉随全量不回退）。落码点＝loop-top 消费点 else 臂（四路未落地同返 `None`：区间未识别／无可压块／区间无命中／台账写失败；run 尾消费点不升级——loop 已终结链无从推进）；`summary_block_guide` 单一来源增「≥2 小节」门槛句（软／硬／窗三块同享）。orz-loop 串行 **847/0/3**（＋2 钉）；**载体批次＝2026-09-24 随 0.6.12 双平台重建进件（不进 0bm 轮）**。
- [ ] **S3 真机**：随各批真机观察（升级链停摆／模板可见性）。
- 入口：BACKLOG 0bn / [`073 §3`](docs/audits/073_UNCOMMITTED_REVIEW_AND_REMEDIATION_2026-09-24.md) / [`v8 §15`](docs/CONTEXT_SLIDER_V8_DESIGN_2026-09-16.md)。


### P1-0br Web 形态 UI（三稿形态 · 照搬复用优先 · TUI 后补）（P1；2026-09-24 用户令立项，BACKLOG 0br；设计权威＝三份 UI 设计稿（2026-09-24 解冻）＋综合稿 [`UI_FORM_CONSOLIDATED_DESIGN_2026-09-24.md`](docs/UI_FORM_CONSOLIDATED_DESIGN_2026-09-24.md)；定位权威＝ADR-0010 §2.4 条 8／§2.6／§14.78（v1.80）；**执行形态＝Web 优先、TUI 形式后补**）

> 用户裁决（2026-09-24）：「UI 的具体形态设计就用那三份 UI 设计稿即可」「那三个稿子要从冻结状态里拽回来」「能复用的当然直接复用……甚至做成 web 的都可以」「能照搬的就照搬，我们遵守开源协议」「先做 web 后补 TUI 形式的 UI」。

- [x] **S1 勘定（只读）**：pager 族依赖闭包与缺件表 ＋ `xai-grok-X → orz-X` 改名映射 ＋ 同代性核对 ＋ 三稿区块 × 复用件 × 照搬/自研 映射表定稿 ＋ Web 桥（ACP-over-WebSocket）与前端工程选型 ＋ 分发面（回环监听／端口／启动方式）与安全边界。（2026-09-24 完成；[`0BR_S1 勘定`](docs/audits/0BR_S1_WEB_FORM_SURVEY_2026-09-24.md)）
- [x] **S2 落码**：桥 ＋ 前端壳（外观照搬 `98.css`／`XP.css`）＋ 投影接线（会话列表／工具卡片／机器动作行／审批弹窗／黑板·块表·指南指针／RLI 提醒／压缩回执／收尾自述／状态栏与 Markers）；**每区块标注来源件**（照搬优先的机械判据）。（2026-09-24 落码，未提交；[`0BR_S2 回执`](docs/audits/0BR_S2_IMPLEMENTATION_2026-09-24.md)）
- [x] **S2 全面审查与处置（2026-09-25，用户令「对当前实现的0br进行全面检查，包括设计合理性、实现合理性、设计与实现的符合性」）**：设计/实现/符合性三路并行只读深查＋主会话亲核；**P0×2 已修**（main.js 严格模式未声明赋值整体死加载；桥打印令牌 fragment `#<hex>` 与前端 `#token=` 解析不匹配——前端从未经浏览器级验证的实证）＋ P1/P2 修复批（md.js 净空收口与代码块双重转义、权限弹窗 Esc/并发应答结算、回放后尾流失复、assets Host 门、journal WS Origin 门、WS 帧上限与行内存上限、会话槽 RAII、渲染内存上限、键盘补全 Tab/Alt+字母、六 scheme 中性路由、`orz web --stdio` 分发序、常量时间令牌比较、端口段校验、Windows 保留名、探针上限与终态窗 256K）＋ 登记面（ADR v1.81「父仓→分发仓」勘误、综合稿 §7 R-7 与形态选择登记、§8 三稿豁免清单与风险补笔、THIRD-PARTY-NOTICES marked 版权行补括注）＋ **前端最低冒烟门**（`orz-web/tests/frontend_smoke.mjs`，零依赖 node 直跑）。读数：Rust 31/31、clippy 0、fmt 净、前端冒烟全绿、实机探针全过。明细 [`0BR_S2_REVIEW_HANDLING`](docs/audits/0BR_S2_REVIEW_HANDLING_2026-09-25.md)；**本批即 S3 放行前置门**。
- [x] **S3 新增面（会话归档投影面）落码（2026-09-25，用户令「请先做完S3的新增面吧」；登记源＝上款用户令"会话的归档和查看归档会话都要做的，请将其归入S3项"）**：桥 `orz-web/src/archives.rs`（清单 `GET /api/archives`＋详情 `GET /api/archives/{session8}`：信封容忍解析与写入方 `decode_archive_package` 同口径、有界转写＝每消息 4K 截断＋总量 1 MiB 停装置标、gzip 双上限 64 MiB/128 MiB、错误 400/404/413/500 类型化、`session_id_ok` 路径门、`flate2` workspace 件零新增外部包）＋前端（探索器会话组行「◆归档」跳转、仅存归档的会话入树、`archive://{s8}` 只读浏览视图＝事实行含 LIF 轮/台账跨度＋转写卡＋工具行按 `call_id` 并线）。读数：Rust **38/38**（＋7 钉）、clippy 0、fmt 净、前端冒烟全绿（＋2 钉）、实机探针（401/400/404 拒绝面＋9 件清单＋最大包 4.0 MiB gz 0.28 s、截断 677/821 生效）＋**浏览器级走查通过**（归档标记／中文转写／截断提示／后退回实时）。实机走查发现：事实行 LIF 段首版取错对象（`conversation.lif` 原生键 ≠ 三键段 `archive_keys.lif` 键）已修；旧归档（2026-09-17）首条用户消息乱码为**归档数据本身**的历史输入编码问题（编码门工作线同族），投影如实呈现不修复（观察项）。明细 [`0BR_S3_ARCHIVE_PROJECTION`](docs/audits/0BR_S3_ARCHIVE_PROJECTION_2026-09-25.md)。
- [x] **S3 新增面·批二（2026-09-25 第二用户令）**：①探索器三组重构＝工作区（加粗组头；两级子级「当前工作区＋已信任工作区」）／活跃会话／归档会话——归档内容不再混入活跃组，仅存归档者独入归档组；②信任清单＝orz 自有信任存储只读投影（orz-workspace `TrustStore` 新增 `decisions()` 访问器，与 agent 门同一存储禁第二套实现；`/api/boot` 增 `trusted_workspaces`；本机 0 条为权威事实——`~/.grok` 条目属上游 xai 工具残留不读）；③◆归档改**归档动作**＝`POST /api/archives/{s8}` → 桥 spawn `orz archive <s8>`（orz-bin 新子命令）→ orz-host `archive_session_on_demand`（复用关闭归档同一原语：包＋三键段＋水位＋ARC journal），UI 递单 agent 执行零执行事实不变，运行中窗口禁用；④「进行中」唯一判据＝实时窗口（liveRunId＋running），journal 无终态旧 run 标「未完成」；⑤⑥两 bug 根因＝指向已终止旧服务端口的残留标签页（当前构建全量扫描 20 会话＋9 归档全通），暴露真实缺陷＝打开路径无错误处理留空白——startTail/openRunReplay/openConversation 全部 try/catch 收口。读数：Rust 41/41（＋3 钉）＋orz-host 按需归档钉子＋clippy 0 新增、POST 双路径 e2e（有侧车 ok:true 产物三件套／无侧车 500 逐字回传）、浏览器走查三组渲染＋截图留档、冒烟门全绿。明细 [`0BR_S3_ARCHIVE_PROJECTION` 批二](docs/audits/0BR_S3_ARCHIVE_PROJECTION_2026-09-25.md)。
- [x] **S3 新增面·批三（2026-09-25 第三用户令「请尝试归档当前全部会话」＋两问题报告）**：①「归档成功不入归档组」＝分组规则缺陷已修——归档判定＝有归档包**且**包 mtime ≥ 最新运行 mtime ⇒ 归档组，归档后继续（新 run）如实回活跃组；②「归档失败」根因＝9 个无头会话按 §14.68 未跨里程碑无侧车，归档原语只打包侧车——补 **journal 重构归档**回退（orz-host `reconstruct_conversation_from_journal`：prompt/model_output 事实机械成包＋`reconstructed_from_journal` 标记＋三键段显式纳入无头 run；打包拆 `archive_raw_session_package` 共享包装禁两套实现）；③**归档全部执行：19/20 成功**（8 个重构成包，aa18ef7e 重复归档幂等覆盖），唯一失败 `6aac0a28`＝早夭 run 零对话事实（如实保留活跃组）；④D 盘满致构建失败——清 `target/debug/incremental` **17 GB**（纯增量缓存可再生），余量 15.2 GB。读数：orz-web 41/41＋冒烟全绿、orz-host 归档面 11/11（重构钉子）、浏览器验证分组与重构归档可读＋截图。明细 [`0BR_S3_ARCHIVE_PROJECTION` 批三](docs/audits/0BR_S3_ARCHIVE_PROJECTION_2026-09-25.md)。**批四（同日第四用户令）**：归档浏览标记栏可用——归档转写用户输入产「▸ 输入」锚＋`anchorFinalOutput` 定「◆ 最终输出」锚（与实时视图同语义），点击跳转/展开照常；实机验证 6 输入锚＋1 输出锚、scrollTop 跳转生效。明细同档批四节。**批四补两笔（同日第五/第六用户令）**：会话列表等宽双列对齐（session-id/session-when，Consolas 12px）；状态栏去「沙箱严格」失真段（生产面无 OS 沙箱，六段→五段；orz-tui 母本同步留 TUI 后补批）。
- [x] **提交推送批（2026-09-25，用户令「请进行提交推送与重建吧，0.6.13 也作为第一个 UI 版本推上 github」）**：orz 落码 `459f8d85`（`orz-web` 桥＋前端＋S2 审查处置＋S3 新增面四批；`.gitattributes` 增前端 LF 稳定与 vendor `-text` 字节保真）＋ 父仓记账（pin＋四份 0br 审计档＋ADR v1.81 勘误＋综合稿 §7／§8＋组件册四件＋索引 v4.40）＋ 载体冻结线 orz `5998d4b1`（bump 0.6.12 → 0.6.13）。读数：orz-web 41/0、orz-host 归档面 11/0、orz-workspace 全绿、冒烟门全绿、clippy 新增 0。
- [x] **0.6.13 重建与发行批（2026-09-25，用户令「请进行提交推送与重建吧，0.6.13 也作为第一个 UI 版本推上 github」）**：源冻结 orz `5998d4b1`；Windows clean 全量 **29m06s**（警告 0／错误 0）、Linux musl 暖缓存 **6m10s**、六件换装 MATCH=True＋ACAF 重 provision（keystore 四值未动）＋ELF 3/3 PT_INTERP=0＋双向冒烟 6/6；**0br 首次进件＝首个 UI 载体**（`orz.exe` +1.53 MB／Linux `orz` +2.24 MB）；字面量新面 8 项 0→≥1、保持面 35 项零回退；**载体级 Web 探针**（`orz web` 换装位真机：`/api/boot` 无令牌 401／伪 Host 401／带令牌 200 含 `trusted_workspaces`、`/api/archives` 200 → 19 件、零残留）；双包 → **GitHub Release `v0.6.13`**（解包回读 6/6 MATCH、容器内 `sha256sum -c` 4/4＋4/4 OK）；README／索引 v4.41／BACKLOG 记账；门禁 `valid: true`；**计数不变（54）**。摩擦＝前批 dev server 残留进程占位致 clean 拒绝访问（仅记录）。明细 [`075`](docs/audits/075_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-25.md)。
- [x] **S3 新增面·批五（2026-09-25 第七用户令「当前归档实质不可用，刷新后也没有将活跃会话归档，而且需要一个明确的新建会话按键才可以，放在"后退"的前面吧」；走查对象＝v0.6.13 载体）**：①**归档分组根因修复**——批三「包 mtime ≥ 最新运行 mtime」规则把 **ARC 审计 journal** 计入活动水位，而 ARC journal 由归档动作自身写入（包 → 水位 → ARC，恒晚于包 1–3 秒）⇒ 每次新归档恒判活跃（真实工作区实证 6ab6275c/6ab6570c；批三全量重打包掩盖了结构缺陷）——活动水位排除 `ARC-` 前缀 run；②**工具栏「新建会话」居「后退」之前**（单 ACP 会话绑定无换会话入口＝归档流程无法收尾的根因）：`ui.newSession`＝运行中拒绝＋清内容/标记/导航＋解绑旧会话（ACP 无 session/close，事实留 `.gsa` 可回看）＋立即建新会话显示 id＋状态栏复位「空闲」；③**实时尾随随 run 切换**（既有缺陷一并修）：每轮 prompt 一个新 run 目录而尾随恒挂 boot 时 runs[0]——`syncLiveTail` 已建会话只跟本会话最新 run（新建会话后不灌旧内容）、无会话跟全局最新（boot 连续性）、切换 offset 0 重灌（journal_tail 文件头补发），挂点＝5s 刷新＋运行期 1s 跟随轮＋boot/returnToLive；④状态栏残留复位（引导期失败 run 无终态事件，标签回「空闲」）。读数：冒烟全绿 **＋5 钉**、orz-web Rust 41/41、真机走查（合成 bug 场景分组正确／新建会话全链／归档浏览＋后退＝空实时窗／新会话 prompt 秒级出现新 run「未完成」＋尾随切换回灌实证；信任门 fail-closed 拒绝符合预期）。明细 [`0BR_S3_ARCHIVE_PROJECTION` 批五](docs/audits/0BR_S3_ARCHIVE_PROJECTION_2026-09-25.md)。未提交／未重建载体；索引无路由变化不 bump。
- [x] **S3 新增面·批六（2026-09-25 第八用户令：旧归档"消失"＋归档/回档/删除 UI 逻辑＋orz 免全路径）**：①**旧归档"消失"诊断＝扫错工作区**——用户从 orz 仓根启动 `orz web`，桥按启动 cwd 定 `.gsa` 根（该处零归档包）；真实工作区 21 包逐包复算批五口径全 `归档OK` 无一丢失；处置＝启动台面新增工作区行（错目录一眼可辨）；②**归档 UI 逻辑重构**——会话行单击＝选中（再点取消、深蓝高亮）/双击＝打开；「归档」与「活跃会话」组头同行、「回档」「删除」与「归档会话」组头同行，未选中可见但禁用，动作均弹确认窗；行内「◆归档」退役；③**回档**＝`DELETE /api/archives/{s8}` → 新子命令 `orz unarchive <s8>`（移除归档包＋水位，会话回活跃组，数据保留）；**删除**＝`DELETE /api/sessions/{s8}` → 新子命令 `orz delete-session <s8>`（彻底移除归档包/水位/侧车/运行 journal，不可恢复）；桥侧三路由共享单 spawn 助手；会话段门取最严 8 位小写十六进制；**钉子逮住 RUN-CLI 前缀误吞缺陷并改精确匹配**；④**orz 免全路径**＝`%LOCALAPPDATA%\orz\bin\orz.exe`＋用户 PATH 注册（新终端任意目录 `orz web` 即用；刷新＝构建后重拷）；⑤附带＝横幅 flash 过 sanitize（stderr 透传 ANSI 不再直入横幅）。读数：冒烟＋6 断言全绿、orz-web 42/42、orz-host 归档面 12/12、真机走查全链（归档移组/回档回组/删除五件全清/近似会话无恙/双击打开）。明细 [`0BR_S3_ARCHIVE_PROJECTION` 批六](docs/audits/0BR_S3_ARCHIVE_PROJECTION_2026-09-25.md)。未提交／未重建载体；索引无路由变化不 bump。
- [x] **S3 新增面·批七（2026-09-25 第九用户令：prompt「Internal error」四层根因＋信任窗 Web 化＋--real＋大栈＋单击即开修正）**：①**ACAF env 未配置 fail-closed 拒跑**（stdio 复现实锚；三 env 历来逐次内联、桥无继承源）→ `orz web` 启动台面缺失警告＋三 env 已写用户级；②**工作区未授信＋Web 链无信任窗**（用户反查提示命中：GROK_HOME 重定向→安装目录沙盒空存储，`D:\CLI` 带仓库配置 → 非交互 fail-closed；TUI 靠交互式信任窗授信，Web 缺此窗）→ 信任窗 Web 化＝`POST /api/trust` → 新子命令 `orz trust <cwd>` → `grant_workspace_trust(_with)` 写同一 GROK_HOME 沙盒存储，前端确认弹窗→授信→自动重发；③**假模型**（桥 spawn 无 ORZ_REAL）→ `orz web --real`（与 TUI `--real` 同一 build_gateway 决策点）＋启动台面模型传输三态行；④**栈溢出**（run_agent_loop 巨型 future／**0bm 未竟②** 真实网关首 poll 打穿主线程栈 0xC00000FD）→ main 体搬 64 MiB 显式栈线程（治本仍归 0bm 未竟②；**2026-09-26 勘误＝本条原记「0bt②」系误标**——0bt ② 实为版本核验旁路，该拆分正规登记位为 0bm 未竟②，执行面已随 `P1-0bs` 同载）；⑤**错误细节透出**（error.data 并入消息，替代裸 Internal error）；⑥**单击即开修正**（用户报告「点击其他对话主窗口不切换」——会话行单击＝打开＋选中，撤批六双击中间态）。读数：冒烟全绿、orz-web 42/42、session 7/7、真机全链（信任弹窗→授信→自动重发→真实模型「收到」回轮；已信任工作区(1) 沙盒可见；单击即开回归）。明细 [`0BR_S3_ARCHIVE_PROJECTION` 批七](docs/audits/0BR_S3_ARCHIVE_PROJECTION_2026-09-25.md)。观察项：真实轮偶发长延迟（自行完成，非挂死）留观。未提交／未重建载体；索引无路由变化不 bump。
- [ ] **S3 真机首读**：Web 端真机会话（含审批、长任务、并行）＋ 三稿区块逐项对照（存在性／语义／默认中文／键盘可控）。**S4 收口**：判据入账＋TUI 形式后补的排期与复用件盘点。
- **判据（S1 定稿①–⑦）＋ S3 增补（2026-09-25 处置批登记）**：① 三稿区块齐备；② 每区块照搬来源可核；③ 默认中文＋Help 分页；④ 核心操作全键盘可达；⑤ UI 零执行事实（核查方式：前端 grep 无权限判定/会话归属决策分支＋选项透传核对）；⑥ 协议面齐备（`NOTICE`／许可文本／组件登记；落点＝orz 分发仓，ADR v1.81）；⑦ 分发面零外部可达（回环＋令牌＋Host/Origin 实测拒绝）。**S3 增补**：IME 组词（Enter 提交 vs 确认、Esc 取消组词）、刷新/断线提示实测、长会话渲染读数（内存上限与 rAF 节流生效）、md.js 净空浏览器级复验（javascript: 链接剥除、外链图片剥离）、探索器首屏 5s 滞后消除复验。**S3 新增面（用户令 2026-09-25"会话的归档和查看归档会话都要做的"）**：**会话归档投影面**——桥只读归档 API（列 `.gsa/archives` 清单＋gzip 内摘要）＋会话组行"归档"标记＋归档内容只读浏览面板（与快照选择器、会话标题增强同批）；agent 侧归档已实现且在工作（`orz-host/acp_server.rs` 增量判定＋单包 gzip＋milestones＋`session_archive` 事件），本面只补 Web 只读入口，零执行事实不变。**已落码（2026-09-25）**，回执 [`0BR_S3_ARCHIVE_PROJECTION`](docs/audits/0BR_S3_ARCHIVE_PROJECTION_2026-09-25.md)；快照选择器与会话标题增强仍留 S3。
- 入口：[`综合稿`](docs/UI_FORM_CONSOLIDATED_DESIGN_2026-09-24.md) / [`三稿`](architecture/CLI_UI_INTERACTION_MODEL_v0.1.md) / ADR-0010 §14.78 / BACKLOG 0br。


### P1-0as 编码 lossy 兜底细化

### P1-0aw 宿主资源面：OS 委派执行（P1；2026-09-20 立项、同日重定案＋设计定稿；BACKLOG 0aw；来源＝S3 摩擦 N4）

> 设计权威：[`OS 委派执行设计`](docs/HOST_RESOURCE_OS_DELEGATION_DESIGN_2026-09-20.md) / 入口：[`N4 重定案`](docs/audits/N4_OS_SCHEDULING_DELEGATION_2026-09-20.md) / [`N4 首版调研`](docs/audits/N4_RESOURCE_GATE_INDUSTRY_RESEARCH_2026-09-20.md) / [`深挖档 §5`](docs/audits/0AR_S3_FRICTION_DEEP_DIVE_N1_N6_2026-09-20.md) / BACKLOG 0aw。
> 口径（用户 2026-09-20 澄清）：orz 只发进程、只收结果；资源与运行交给操作系统；linux 侧需顾及**真机**（不只容器）。原「基线校准／分级降速／压力优先」方案作废。

- [x] **S1 拆准入拒绝（小）**（2026-09-20 过夜批）：25 % commit headroom 预检拒绝／`GateDecision`/`Refuse` 分支/`host_resource_denied` 注入/run_tests 预检门全部删除；`CommitLimit`／`Committed_AS` 降为观测字段（注释明示）；判据 5 历史兼容实测（S3 三 run journal 经保留 verifier 回放零错误）。
- [x] **S2 分类器整体删除（小）**（2026-09-20 过夜批）：分类器 15 件＋拒绝文案族整体删除；`resource_gate.rs` → **`resource_hint.rs`**（保留快照/探针/写目标/tier 观测标签/job 限额推导，新增 `ResourceHint` 软提示＋每卷去重）；`lib.rs` 拆两臂（`terminate_heavy_call_jobs`＋`resource_exhausted` 生产端＝裁决 ④-1；`run_reclaim_pass` 调用与 `reclaim_performed` 生产端＝裁决 ④-2；`reclaim.rs` 保留不接线）、`live_call_jobs` 登记表保留只删 `action_class`（`LiveCallJob`＋`ProcessTreeRecord` 两处）；`families.rs`/schema/fixtures 未动。
- [x] **S3 内核强制面校正（中）**（2026-09-20 过夜批）：裁决点 B 落码＝`default_job_limits` 推导**原样保留、只作上限**（注释补 Run B 因果依据）；`active_process`（2×cores+8≥16）与 `cpu_rate` 80 % 维持；既有 Windows e2e 钉（300 MB 预算子进程 MemoryError）与装配读回钉原样绿。
- [x] **S4 卷余量轴：观测＋预派发软提示（中）**（2026-09-20 过夜批）：`VOLUME_HINT_FREE_BYTES = 4 GiB`；汇点 `write_targets` → 一次 `read_for_volumes` → 跨档快照（保留）→ 软提示附结果头部（中文短句＋英文读数，0af 混排；不阻断、每 run 每卷一次 `HashSet` 去重）；读数不可得 ⇒ 无提示、照常派发；钉子＝触发/去重/边界（恰 4 GiB 不提示）/双卷/单调用单探针五件。
- [ ] **S5 真机复验（中）**：同语料回放 **heavy 类拒绝数 = 0**；无 OOM 杀；run 墙钟不因等待恶化；每次调用仍留可核观测（tier／readings／`enforced`）；只改 orz 侧，不动 `task.toml`／镜像／verifier／数据集 pin。

- [x] **S0 裁决门（已裁决 2026-09-20）**：两条 0z S2 动作臂**一并退役**——删 `terminate_heavy_call_jobs` 与 `resource_exhausted` 生产端 ＋ 删回收触发与 `reclaim_performed` 生产端；三条事件 schema／verifier 保留供历史 journal；`live_call_jobs` 登记表保留、只删 `action_class`（详见 [`设计 §10.1`](docs/HOST_RESOURCE_OS_DELEGATION_DESIGN_2026-09-20.md)）。

**裁决点（待用户）**：**A** 卷轴归属（全交 OS／观测＋软提示〔推荐〕／维持现状）；**B** run 级 Job commit 预算（默认不设／固定预算如 4 GiB／保留推导但只作上限〔推荐〕）；**C** 分类器（保留为观测标签〔推荐〕／随门删除）。

### P1-0ax 检索形态 URL 完整性（P1；2026-09-20 立项，BACKLOG 0ax；来源＝S3 摩擦 N5）

> 入口：[`调研档 §2`](docs/audits/N1_N5_ITEM_REGISTRATION_AND_N4_N5_RESEARCH_2026-09-20.md) / BACKLOG 0ax。

- [x] **S1 口径面**（2026-09-20 过夜批）：`is_synthetic_answer`（web_search_result 且引用池空）＋`synthetic_answer_count` 单列；`usable_source_count` 剔除合成（阈值 5/护栏 10/倒数共用尺一致性不变；FP-2/阈值数值/工具面不动）；倒数行 `synthetic>0` 附可引用性披露（=0 逐字节不变）；payload 顶层可选 `synthetic_answer_count`（>0 才落）；契约面按 0ar S1 先例（schema＋fixture＋check_repository）。
- [x] **S2 形态面（评估件）**（2026-09-20 过夜批）：评估落报告 §7.1——开关 `ORZ_WEB_SEARCH_LOCAL` 已在位（默认 off、off 即零行为变化）；可比性论证＝通道消融（任务/verifier/pin/墙钟不动）＋契约不变＋G1–G3 补强为网络形态预设；启用＝装置侧 env 一行、不动 `task.toml`、不启用 `eval_browser`；判据预登记（带 URL 占比 ≥90 %、synthetic 恒 0、fetch 数不低于 r3）。**2026-09-20 用户放行**——启用进下一轮官方跑批（装置侧适配器 `tb_agents/orz.py` env 白名单透传 `ORZ_WEB_SEARCH_LOCAL`；0.6.4 载体已含该车道，**无需重建**）。
- [ ] **S3 fetch 侧兜底**：Rust 指纹伪装（`wreq`／`netty`〔原 `rquest`，2026-09-25 核实已改名〕）或 reader 服务，先覆盖判据要求的公共源；复测 grep.app 类反爬站点。（2026-09-20 过夜批：路线评估落报告 §7.2——推荐先 S2 本地车道、S3 只兜残余面；依赖新增需网络实测核证与本批「不重建」边界冲突，落码待放行。）
- [ ] **S4 真机复验**：同三题口径下「无 URL 结果占比／可引用来源数」可机械读数并进入审计。（2026-09-20 0ay S4 真机已取读数：provider 合成车道下**无 URL 占比 12/12＝100 %、可引用来源 0、收窄可用 18**，见 [`0ay S4 真机复验`](docs/audits/0AY_S4_THREE_TASK_VERIFY_2026-09-20.md)；本地分段车道形态判据〔带 URL ≥90 %、synthetic 恒 0〕随本项 S2 放行后再核。）
- **2026-09-25 用户令**：**S3 落码面随 `P1-0bs` 同载**（S4 真机仍留官方跑批面；本条目闭合仍随其自身史）。



### P1-0ba 运行身份唯一性（run id 秒级撞名）（P1；2026-09-20 用户令立项〔「F13/15均立项」〕，BACKLOG 0ba；来源＝0am 狗粮轮 `RUN-CLI-6aafc998` 摩擦 F13）

> 入口：[`0am 报告 §5-F13`](docs/audits/0AM_DECODER_FORM_REPLAY_2026-09-20.md) / [`main.rs`](orz/crates/orz-bin/src/main.rs:1439) / BACKLOG 0ba。

- [ ] **S1 铸号唯一化**：`RUN-CLI-{ts}` 的 ts 为秒级十六进制（无 pid／计数）⇒ 同批并发同秒撞名（语料 7 组）；铸号加 pid＋毫秒或进程内计数，保留 `RUN-CLI-*` 前缀。
- [ ] **S2 钉子与兼容注**：两钉（同秒两次铸号互异／同 cwd 并发 journal 目录互异）＋语料与工具侧「路径＋run id」复合键口径注；既有 `.gsa/runs`／`session_archive` 扫描面回归核证。


### P1-0bc 资源层收口与可失败分配（**复合狗粮任务**）（P1；2026-09-20 用户令立项，BACKLOG 0bc；来源＝资源三层复核轮；设计裁决＝[`OS 委派执行设计 §11`](docs/HOST_RESOURCE_OS_DELEGATION_DESIGN_2026-09-20.md)）

> 裁决五条：③ CPU 上限**去掉**／① run 级 Job commit 上限改**通知式**（`JOB_OBJECT_LIMIT_JOB_MEMORY_LOW`＋作业完成端口）／② 活动进程上限**保留＋env 覆盖**／④ **可失败分配＋降级（双平台）**／⑤ 外部 supervisor **仅记录不实施**。落点预判：[`resource_hint.rs`](orz/crates/orz-host/src/resource_hint.rs)／[`lib.rs`](orz/crates/orz-host/src/lib.rs)／[`resource_job.rs`](orz/crates/codegen/xai-tty-utils/src/resource_job.rs)＋orz-loop 大缓冲与 journal 装配路径。

> **0bc 复合狗粮轮收口（2026-09-21，run `RUN-CLI-6ab00c8a`）**：S1/S2 与长杂轮（FR-3/5/6/7）、杂项（FR1/FR2）全部落码并本地验证全绿（loop 822/0/3、host 322/0/5、assurance 259/0、xai-tty-utils 32/32、orz-tools search_replace 113/0、fmt 干净；工作树未提交、不推送、不重建）。S3 RLI 观测已收（可用性裁决：实际可用、任务中消费率低、维持影子——总结档 §6）；资源面新特性读数与 Linux 编译面待下批。S4 判据①–⑥对账入账（ADR 转录随批）。详见 [`0BC_COMPOSITE_DOGFOOD_2026-09-21`](docs/audits/0BC_COMPOSITE_DOGFOOD_2026-09-21.md)。

- [ ] **S1 勘定与契约面**：可失败分配路径清单（journal 装配／模型缓冲／ledger）＋env 名（活动进程上限覆盖）＋临限通知事件形状（不新增事件族）。
- [ ] **S2 落码**：三面改造（去 CPU 顶／commit 通知式／进程上限 env 覆盖）＋可失败分配与降级链（Windows／Linux 双平台）。
- [ ] **S3 复合狗粮轮**：真机长 run——CPU 去顶后墙钟、临限通知事件、无 orz abort、进程上限读数；**同轮观测 0am 的 RLI 实际表现**（**2026-09-20 顺序裁决：0am RLI 改造与载体重建先行**——先改好 RLI → 重建 → 进 0bc）。
- [ ] **S4 收口**：判据 ①–⑥ 逐条对账入账；ADR 转录随批。
- [ ] **0bc 长杂轮（2026-09-20 用户令「FR-3/5/6/7 全部进行处理，纳入 0bc 长杂轮」）**：① **FR-3 硬提醒层锁工具面**＝**需处理**（硬提醒**仅打断式提醒、不锁工具面**；撤掉「窗口内仅 `blackboard_write`／`context_compress`」式收窄）／② **FR-5 `name` enum 参数面／校验面双源**／③ **FR-6 配极不进快照**／④ **FR-7 旧侧车行 `v`/`E`=0 真伪不分**；**FR-4（先读后改）不处理**（正确实现）。**定位声明**：0bc **主要仍是完成任务**，RLI 读数参考＝**任务完成后再看实际运行中 RLI 的状况**。
- [ ] **0bc 杂项（2026-09-20 用户令「FR1/2 均可立项，并入 0bc 杂项中」）**：① **FR1** 组合字符精确匹配（`search_replace` 对 `T̂`＝`T`+U+0302 等报「string not found」，自 `read_file` 输出复制即成功；疑 NFC/NFD）⇒ 落「匹配失败回显最近似行的码点／规范化提示」；② **FR2** dogfood 会话继承 `ORZ_ACAF_FAIL_CLOSED=1` 等四键 ⇒ 「起 run」类测试全 panic（本批 19+30 例）⇒ 落「测试入口 script化 unset 或测试内显式 ACAF override」，不动 fail-closed 纪律。



### P1-0bd 摩擦大杂项轮（FR 批处置 · 繁杂狗粮轮）（P1；2026-09-21 用户令立项，BACKLOG 0bd；来源＝0bc §7 摩擦九条＋启动器 RLI 开关缺口）

> 用户令「我同意你对摩擦项的判定和处理裁决，请像0bc一样处理吧，将目前待处理的摩擦项并成一个大杂项任务并进入排期，继续当成狗粮轮」。三分类＝可处理／仅记录／不做；本项只含可处理项，按 0bc 同形真机狗粮轮执行（落总结档即可，不必提交/推送/重建）。

- [x] **S1 勘定**：**十二件**落点与改法定案（五件＝构建自检／测试入口 env／`--` 吞参／警告清零／启动器 RLI 开关〔原第六件＝设计 §11② 勘误，已于 2026-09-21 随 0bc 提交批落地〕＋**追加七件**＝⑦ RLI 数值判定式提醒｜⑧ 启动器影子开关（⑤ 升级）｜⑨ env 假红改夹具自清（② 升级）｜⑩ 探针收编｜⑪ 冻结语料纪律｜⑫ 宿主两红定位｜⑬ 繁杂度投递／侧车覆盖〔与 0bf 长轮同源〕）。
- [x] **S2 落码**：十二件实施＋钉子（env 清理／`--` 吞参／判定式提醒阈值各留一处可核入口）。
- [x] **S3 真机狗粮轮**：杂项合并轮跑真机（同轮观测 RLI 面；新摩擦随轮记录）。（**2026-09-22 用户令「长会话真机狗粮轮依靠实际任务，用 0bd 作为任务」**⇒ 本项即 0bf S3 长会话轮，同轮并收 0bc S3 三项资源面与 0be 投递链／侧车覆盖；前置＝0bf 码进载体，已随 **0.6.8** 重建完成——[`068 重建档`](docs/audits/068_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-22.md)）（**2026-09-22 达成**：run `RUN-CLI-6ab17325`，40m56s，exit 0；十四件中十三件落码（⑥ 早随 0bc 落地）；同轮收 0bc S3 三项资源面与 0be 投递链覆盖；摩擦 a–n 见 [`0bd 报告 §6`](docs/audits/0BD_FRICTION_MISC_2026-09-22.md)；**未提交／未推送／未重建**）
- [ ] **S4 收口**：总结档＋判据对账；计数同步随提交批（设计 §11② 勘误已随 0bc 提交批落地）。（**2026-09-22 轮达成**：报告已落 [`0bd`](docs/audits/0BD_FRICTION_MISC_2026-09-22.md)、十三件落码（轮内未提交／未推送／未重建）；**2026-09-22 提交推送**：orz `56d328ee` 六件＋父仓本批（见 [`0bd 报告 §9`](docs/audits/0BD_FRICTION_MISC_2026-09-22.md)）；**计数同步已随提交批落地**）（**2026-09-22 进载体**：六件已随 **0.6.9** 双平台重建进在役载体，源冻结 orz `88b6dea1`；字面量核证 0bd③b 四条首次进件、既有面零回退；[`069 重建档`](docs/audits/069_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-22.md)）
- 入口：BACKLOG 0bd / [`0bc 总结档 §7`](docs/audits/0BC_COMPOSITE_DOGFOOD_2026-09-21.md)。


### P1-0bg RLI 收尾与摩擦大杂项轮（FR 批处置 · 繁杂狗粮轮）（P1；2026-09-22 用户令立项，BACKLOG 0bg；来源＝[`0bd 轮报告 §6`](docs/audits/0BD_FRICTION_MISC_2026-09-22.md) 摩擦 a–n＋§7 遗留）

> 用户令「本轮的摩擦项中需要处理的可直接立项，并于0bd同形态，落成一份大的杂项用作狗粮轮任务」。三分类沿 0bd（可处理／仅记录／不做），只含可处理项；**按 0bd 同形真机狗粮轮执行**（落总结档即可，不必提交/推送/重建）。

- [ ] **S1 勘定**：九件可处理项的落点与改法定案（宿主并行负载敏感红的处置〔wrapper 文档化串行档为红判据入口＋`call_tool_timeout` 续跑调用显式覆盖＋两条竞态单独设计，**不放宽断言强度**〕／测试层 `reclaim.rs:478` 警告清零／RLI 读数面 1 KiB 截断的可见性策略／测试入口 auto 降并行档／侧车 drain 落盘核／题面写入端强制 BOM／其他 `&` 调 cargo 脚本的 `--` 容错／RLI 压缩窗口取值开销口径／权重与锚标定〔探针 skill 表作输入〕）。
- **2026-09-22 重建轮先落一件（069）**：`.ps1` 三脚本（`build_orz.ps1`／`dogfood_launch.ps1`／`run_orz_tests.ps1`）**已补 UTF-8 BOM**（PS 5.1 解析 0 错；0bd 轮内由 pwsh 7 调用故未暴露）——本项「写入端强制 BOM」只剩**题面**面待做。
- [ ] **S2 落码**：实施＋钉子（各件留一处可核入口）。
- [ ] **S3 真机狗粮轮**：杂项合并轮跑真机（同轮观测 RLI 面；新摩擦随轮记录）。
- [ ] **S4 收口**：总结档＋判据对账；计数同步随提交批。（**2026-09-22 提交推送**：orz `7da1a9fe` 八件＋父仓本批，见 [`0bg 报告 §10`](docs/audits/0BG_RLI_FINALIZE_AND_FRICTION_2026-09-22.md)；计数同步已随提交批落地＝**维持 46**）（**2026-09-22 进载体**：八件已随 **0.6.10** 双平台重建进在役载体，源冻结 orz `f36dee7c`；字面量核证新面双侧进件、模型面 `域迁移+` 徽章 1→0；宿主全量串行 322/0/5；[`070 重建档`](docs/audits/070_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-22.md)）
- **第三 kind 定案（2026-09-22 用户裁决）**：**采「掩盖缺口」`CoverageGap`**（分母＝域枚举 `normal／pressure／low_progress／stuck` 四值、`Start` 不计；`g = 未访问域占比`；就绪门＝已完成段数 ≥ 3〔冷启动久些可接受〕；越线门＝当前段驻留 ≥ 该域已完段中位；触发沿一次、域切换重武装；**随报**＝`g`＋未覆盖域列表＋驻留比 `r`＋域模型累积失配）；**域模型累积失配降为随报字段**（不独立触发）；驻留比 `r` 不另立 kind，只作随报量。
- **提醒注解定案（2026-09-22 用户裁决：随报）**：每条提醒**都**随报注解（不采首现一次；理由是压缩后模型可能再也看不到释义）；固定模板＋自含（术语释义／基准／非阻断声明）、与 `name` 描述单一源（FR-5）、本体＋注解 ≤240 B/条。**覆盖（2026-09-22 用户裁决「该加的注解都加上」⇒ 全采纳）**＝RLI 三 kind（掩盖缺口／域迁移确认／持续越线）＋失配两把尺（通道级／域级）＋RLI 面缩写族（面头符号表，与摩擦 g 同批）＋机械码随事件告知块给固定释义；已有自带解释者（会话提示两族、资源软提示、检索倒数行）只对齐口径不重做。
- **双迁移通知定案（2026-09-22 用户裁决：模型面只报确认式）**：① **LIF 本轮不退役**（暂不退役；RLI 稳定后仍可退役**或降为影子组件**，按读数裁决）；② LIF `域迁移+n` 即时计数徽章**不再报告给模型**——仅自行累计＋**机械层连带记录**（**2026-09-22 用户裁决「连带记录」**：能低成本直供更多信息即更好；落 `mechanical_audit_update`，key＝`lif.domain_migration`（审查表为**每键一条覆盖写**）、summary＝`{from}→{to}@r{n}；累计 m 次`、anomaly＝null（不进报告块）；**只记不发模型**、零 payload 形状变更、逐次历史由 journal 事件流可离线复算）；**实现面先决（勘定实测项）**＝本族 kind 为**闭枚举**，新增须**同批同步 schema 与钉子**（沿用 0AE-C2 纪律）——拟扩一档 `lif_domain`（schema enum＋Python `_verify_v02_mechanical_audit`＋Rust `families_s2c::verify_mechanical_audit`＋本模块契约钉子）；静态读码见 **Rust 侧仍只认三值**（tool_result／plan_gate／budget）而 schema 与 Python 侧**已八值**（含 retrieval_batch／context_scale 等），**须实测核实是否构成对拍空洞**；③ **模型面只留 RLI 确认式**「域迁移确认」（稳定 3 轮、一次性）。读数面（`temporal`／`rli.history`，模型主动拉取）照旧。落点＝0bg S2（删徽章段＋挂机械记录＋四条钉子）。
- 入口：BACKLOG 0bg / [`0bd 报告 §6/§7`](docs/audits/0BD_FRICTION_MISC_2026-09-22.md)。

### P1-0bh 交互面收口二轮：告知面与提醒面（P1；2026-09-22 用户令立项，BACKLOG 0bh；来源＝[`0bg 轮报告 §5`](docs/audits/0BG_RLI_FINALIZE_AND_FRICTION_2026-09-22.md)＋主会话回查）

> 用户令「以上值得修的内容可以全部进行立项，请再次将这些内容合并成为一个大的杂项吧，作为下一轮狗粮任务」。**按 0bd／0bg 同形真机狗粮轮执行**（落总结档即可，不必提交/推送/重建）。**执行前置＝0.6.10 重建**（0bg 码需先进载体）。

- [ ] **S1 勘定**：**十六件全覆盖**的落点与改法定案（① 提醒投递预算的让位策略／② 拆树告知面的文案与触发点／③ 压缩回执字段／④ 压缩带宽取值／⑤ 两条竞态设计／⑥ 构建争用面的降并行与排队口径／⑦ 报告自核纪律的验收写法／⑧ 0bg 遗留承接口径／⑨ 编辑回抄的锚点形态／⑩ 乱码降级告知文案／⑪–⑬ 三项减负候选取舍／⑭⑮ 说明书与定位符的落码面〔以 [`设计档 §6`](docs/BLACKBOARD_GUIDE_AND_POINTER_DESIGN_2026-09-22.md) 为准〕／⑯ 中性终态与结束自述的落点〔以 [`设计档`](docs/NEUTRAL_TERMINATION_AND_MODEL_STOP_DESIGN_2026-09-22.md) 为准〕）。
- [ ] **S2 落码**：实施＋钉子（各件留一处可核入口）。**提醒投递面钉子＝头段余量读数＋投递率计数（`delivered` 计数化）**；任务面钉子＝徽章让位后提醒仍可投递（构造长头段用例，断言提醒**不**被静默吞掉）。
- [ ] **S3 真机狗粮轮**：合并轮跑真机（同轮收 0bg 新面首现读数与宿主串行判据入口；新摩擦随轮记录）。
- [ ] **S4 收口**：总结档＋判据对账；**报告数字须与收官 journal 对齐**（0bg 复盘教训）；计数同步随提交批。
- **2026-09-22 前置达成（070 重建批）**：**0.6.10 已重建换装**（源冻结 orz `f36dee7c`；档 [`070`](docs/audits/070_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-22.md)）——0bg 八件进件；**宿主全量串行测试 322/0/5 已随批收取**（⑧ 首项达成）。另按主会话核对补齐三处编排面：**S1 覆盖 ①–⑯**／索引 §8 `pending` 桶补 `FUS-BLACKBOARD-GUIDE`·`FUS-CONTEXT-POINTER`／0bg 轮内已处置四条（b/c/d/f）入档。
- **2026-09-22 追加裁决与新增（用户令＋主会话回查）**：**压缩面**＝取消 224K 软提醒、软梯改 **192／256（64K 步距）**、320 硬／500 截断不动（**承重件推论**随 S1 文档化：本轮 7 次压缩全为 `model_selected`，机械 RHYTHM 线 0 次）；**压缩回执带机械摘要＋压缩边界与域时间线对齐**判「值得做」（并入 ③）；**新增两件**＝⑨ 编辑回抄成本（`search_replace` 108.1 KB／66 次；仓内 `grok_build_hashline` 锚点形态可引）⑩ 乱码纯损失（14.7 KB GBK 乱码 → 编码降级告知）；**2026-09-22 用户令「有助于模型降低压力的全都可以做」**⇒ 三项候选**入范围（⑪–⑬）**＝grep 命中预算／审计档按节定位／重复读增量窗口面；压缩面 **192／256 双档经用户确认定稿**（「我同意保留192/256k双档」）；**件数＝十五件**；**0bg 报告勘误**（软/硬档位误标、轮中快照计数、§5 g 机理、§8 归属）随此项登记。
- **两门已裁决（2026-09-22）**：① **门一＝选 A**——不做显式脱离面；告知面升级为「**主进程＋逐条子进程都标注**，并在有子进程时附注释说明子进程随本调用一并关闭」，模型需维持子进程则**自行开受管后台**；② **门二＝选 A**——只做**目标档建议**＋**较早内容仅留「域位置＋轮号」导航形式**，不加折叠下限、不动机械 `max_reduction_ratio`。
- **2026-09-22 追加两件（用户提仪：⑭⑮）**：⑭ **框架说明书（简要、常驻黑板）**＝常驻黑板分区（`section=guide`／selector 折叠面）承载机制说明书，单源＋版本/digest，**只释机制不给动作建议**，pull 面零常驻注入，与 ① 头段预算分工；⑮ **机械定位符（跳转回查）**＝每轮每项自动生成短指针（例 `r112·b7·s1841`＝轮／块号／journal seq），在压缩回执／域时间线／块表三处随报；基建已有（journal `seq`＝行号、黑板 `epoch` 归档读、`read_file` offset/limit＋sha256 锚点、块表块号）；指针预算单列。**设计档已落**：[`BLACKBOARD_GUIDE_AND_POINTER_DESIGN`](docs/BLACKBOARD_GUIDE_AND_POINTER_DESIGN_2026-09-22.md)（v1.0；索引条目 `FUS-BLACKBOARD-GUIDE`／`FUS-CONTEXT-POINTER`）。**件数＝十六件**（本行覆盖同日上文的「十五件」计数）。
- **2026-09-22 追加一件并入（用户提仪：⑯）**：⑯ **中性终态与模型主动停止（结束自述）**——用户令「这个也直接并进0bh就行，做狗粮轮」＋「这一部分是新设计，要新开设计文档并同步进index等文档中」。**设计档已落**：[`NEUTRAL_TERMINATION_AND_MODEL_STOP_DESIGN`](docs/NEUTRAL_TERMINATION_AND_MODEL_STOP_DESIGN_2026-09-22.md)（v1.0 定稿、未实施；索引条目 `FUS-NEUTRAL-TERMINAL`）：**中性终态**（「完成」降为与其它原因平级的**一种**原因，不再是唯一正常终态；旧值保留可读，v0.1 冻结回放面按旧值解释）＋**结束原因类别**＋**结束摘要常态化**（每次结束都写，不审查不罚、不设质量门；不得因摘要质量区分终态待遇）＋**无应答者时暂停与结束同形**（暂停＝结束的一个**意向取值**；收束为「待回应」、不死锁、续跑走外部入口）；**四原则**＝判断归模型（不设证据门槛、不要求举证）／机械层**只记录与传话，不判定不建议不驳回**／回应归外部（不回加框架、不推回模型）／无应答者不剥夺出口；**跑分口径**＝挂起形态**显式关闭**（无人应答）＋起跑前断言，中性终态与结束摘要**保留**（判分取自产物与 verifier，不由终态字段决定）；**子项**＝模型面「**替模型做判断**」的文案清理——**判定＝去判断而非去建议**（处置优先级 **删 ＞ 保留 ＞ 改**）：用户最初口径为「能换事实陈述就换／不加后果」，经 2026-09-22 复核**大幅收窄**（用户批「对本轮判定我没有异议了，就按照这个落档吧」）——① 用户面提示不在模型面（疲劳水位＝`orz-bin/main.rs` run 末 stderr 附言、注释明写不注入模型）；② 通知天然带祈使（「请按下方指针回读」「需要更早内容时按上表回放」），去祈使即损害可读性；③ **不做机械禁词钉**（铺开会误杀通知；`prompt.rs` F6 push 单面禁词钉仅作先例）。**净范围＝只删一处**（`context_scale.rs` 的 T1 截断告知块与 700K 上限守卫块之「任务无需中止／继续即可」——同句的「工作现场与残段逐字未动」已是状况陈述）；**其余六处保留现状**（320K「现在就压」＝建议且带「不压缩也可以」授权句、为压缩模型自撰的承重件；窗口轮两条＝协议通知；门二＝只给目标档告知；② 工具描述＝通道信息；用户面水位＝不进模型）。**判定原则**＝报告保留原样／推动先问必要性／**替模型下判断＝删**；**既有「带意见」文案归属**经用户澄清＝**外部设计者的注解与判定**、非机械层自行发声（与原则不冲突）。**件数＝十六件**；**计数维持 46**（并入 0bh，不新增未闭合项）。
- **2026-09-23 进载体（★）**：**0.6.11 已双平台重建换装**（源冻结 orz `917fadfb`；档 [`071 重建档`](docs/audits/071_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-23.md)）——**落码十三件进件并经字面量核证**（⑯ 声明块以「前缀＋闭合」两件核证，全形为字节级方法边界）、**退役面 `任务无需中止` 2→0**、**保持面零回退**；⑤ 设计定稿随批留档；⑨⑬ 未落码 ⇒ 承接 0bi S2。⇒ **0bi S3 真机首读的前置（载体带新面）已达成**。**不发行**（中间过渡版，沿既有裁决）。
- 入口：BACKLOG 0bh / [`0bg 报告 §5`](docs/audits/0BG_RLI_FINALIZE_AND_FRICTION_2026-09-22.md) / [`071 重建档`](docs/audits/071_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-23.md)。


### P1-0bi 0bh 轮摩擦与承接大杂项（P1；2026-09-23 用户令立项，BACKLOG 0bi；来源＝[`0bh 轮报告 §5/§7`](docs/audits/0BH_INTERFACE_CLOSEOUT_R2_2026-09-22.md)＋主会话回查）

> 用户令「本轮摩擦项中值得立项的都落进下一轮大杂项中即可，依旧按照狗粮杂项的例子落」。**按 0bd／0bg／0bh 同形真机狗粮轮执行**（落总结档即可，不必提交/推送/重建）。**执行前置＝下一次双平台重建**（⑧ S3 新面首读需新载体）。

- [ ] **S1 勘定**：十一件落点与改法定案（① 编辑面编码保真的写入侧形态〔保 BOM／按原编码回写〕／② 批量注入纪律的自检口径／③ 测试构造 helper 收敛面／④ 报告自核的收官口径与链接可达自检／⑤ `search_replace` 锚点模式／⑥ 重复读增量窗口／⑦ 调用终态机械告知／⑧ S3 新面首读清单〔grep 尾注·`outline`·`guide`·anchor 三态·`run_finished` 新字段·块表指针列·投递计数〕／⑨ 外部客户端草稿/终稿重复的处置形态〔ACP 车道门控缓冲 or 替换语义；用户口径＝先接着用〕／⑩ 反例门触发条件收窄的落码面〔`agent_loop.rs` 终答候选处兼读 `tool_rounds` 与黑板 plan 状态；设计转录 ADR-0010 §14.76〕／⑪ 写入面 emoji 剥离的咽喉点与逃逸开关〔以设计档 §5 为准〕）。
- [ ] **S2 落码**：实施＋钉子（① BOM 保持用例＋`.ps1` 解析 0 错；② 插入计数自检＋幂等折叠；⑤ 锚点编辑模式；④ 报告自核体例；⑩ 反例门触发条件＋钉子〔有执行事实或未完成 plan 才触发〕；⑪ emoji 剥离＋钉子〔ZWJ 残肢／非 emoji 保留／逃逸反例〕）。
- [ ] **S3 真机狗粮轮**：合并轮跑真机（同轮做 0bh 新面首读；新摩擦随轮记录）。
- [ ] **S4 收口**：总结档＋判据对账；**报告数字须与收官 journal 对齐**（0bh 复盘教训：3213 快照 vs 3374 收官）；计数同步随提交批。
- **2026-09-23 追加（并入 ④／S2 纪律；来源＝[`071 重建档 §8`](docs/audits/071_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-23.md)）**：0bh 轮**只跑过滤档**（`cargo check --tests`＋具名过滤器）⇒ **0ao 扫描红**（⑭ guide 文案字面）与 **orz-loop lib 三红**（④／⑯ 改行为后旧断言未同步）漏网；0.6.11 重建批的**全量串行复跑**抓出并**当场修复**（orz `a46c7c02`；行为零变化，含一处**生产源码形态面**改动〔`controller.rs` guide 文案改按常量插值、豁免表维持空表；模型面输出逐字等价〕＋ 三处测试断言改钉状况陈述）。⇒ **④ 增一条纪律**：落码轮**红判据入口＝全量档** `run_orz_tests.ps1 test --release -p orz-assurance -p orz-loop --lib -- --test-threads=1`（过滤档只作补充）。
- **2026-09-23 前置达成（071 重建批）**：**0.6.11 已重建换装**（源冻结 orz `917fadfb`；档 [`071`](docs/audits/071_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-23.md)）——0bh 十三件进件＋字面量核证＋宿主装配断言全过 ⇒ **0bi S3 真机首读的前置已备齐**；**不发行**（中间过渡版）。
- **2026-09-24 轮达成 ＋ release 档**：真机 run `RUN-CLI-6ab3dbe5`（载体 0.6.11／181.7 min／exit 0／工具轮 252）——落码四件＝① BOM 保真／⑤ anchor 行窗／⑩ 反例门收窄／⑪ emoji 剥离（各带钉子）；裁定七件＝② 归模型侧纪律、③⑥⑦ 延后、⑧ S3 首读读数、⑨ 记录、④ 报告自核；报告 [`0BI_FRICTION_CARRYOVER`](docs/audits/0BI_FRICTION_CARRYOVER_2026-09-23.md)；**release 全量串行档已于 2026-09-24 补跑**（exit 0／警告 0：orz-assurance 275/0、orz-loop 839/0/3，`Finished release` 7m01s）；**未提交／未推送／未重建**；⑧ 三项未读与 ⑪ 告知行首读转 0bj S3（**2026-09-24 晚补：四批落码已提交推送 `aeffea2b` 并随 0.6.12 进载体与发行**）。

- [x] **载体 0.6.12 双平台重建换装与发行（2026-09-24 用户令「请进行重建吧，本次重建的双平台包发布上去」；见 [`074 重建与发行档`](docs/audits/074_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-24.md)）**：① 版本 bump 0.6.11 → 0.6.12（orz `d69dab47`，两文件两行，推 `cli`）；② **四批落码首次进在役载体**＝0bi 四件＋0bl 十件＋0bk S2＋0bn S2；③ Windows clean 全量 **25m56s**（clean 63,779 文件／37.0 GiB，警告 0 错误 0）／Linux musl 暖缓存 **10m00s**（APT 预检 OK、未切代理、警告面与 0.6.11 同数）、三件 PT_INTERP=0＋双向冒烟 6/6；④ 换装六件 MATCH=True（`.0.6.11-bak` 链）＋ACAF 重 provision（keystore 四值未动、manifest↔signer 逐位一致）＋`dogfood_launch -DryRun` 断言全过；⑤ **字面量核证**新面 9 项 0→≥1、保持面 26 项零回退（`（见上）` 1→1 记方法边界）；⑥ **发行 GitHub Release `v0.6.12`**（zip 27,815,042 `1a48f33c…`／tar.gz 36,042,724 `5bd2af6e…`，包内 `sha256sum -c` 4/4＋4/4、解包回读 6/6 MATCH）；⑦ 记账面：manifest 1466 条、索引 v4.39、README 发布面 v0.6.12、门禁 `valid: true`。**计数不变（54）**；0bm 前置（四批进件）已备齐，0bk／0bn S3 真机随下一轮。
- 入口：BACKLOG 0bi / [`0bh 轮报告 §5/§7`](docs/audits/0BH_INTERFACE_CLOSEOUT_R2_2026-09-22.md) / [`071 重建档`](docs/audits/071_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-23.md)。

### P1-0bj 0bi 轮摩擦与承接大杂项二轮（**同轮执行＝0bm**）（P1；2026-09-24 用户令立项，BACKLOG 0bj；来源＝[`0bi 轮报告 §5/§7`](docs/audits/0BI_FRICTION_CARRYOVER_2026-09-23.md)＋主会话 2026-09-24 回查）

> 用户令「摩擦项a/b/d/e可立项进新一轮大杂项任务」。三分类沿 0bd／0bg／0bh／0bi，只收可处理项；**同轮执行＝0bm 狗粮长轮（2026-09-24 用户令「0bj 融进 0bm」；原「按 0bd 同形单独轮」取消，S4 总结档随 0bm 轮）**。**执行前置＝0bi 码进载体**。

- [ ] **S1 勘定**：六件落点与改法定案（① 版本核验旁路形态〔`--build-info` vs 读 `Cargo.toml`〕＋「勿裸跑 `orz.exe`」纪律入口／② 块表说明行口径〔指针仅已压缩·已截断块具备〕／③ `cd /d` 的告知落点〔工具描述 or 文档备注〕／④ 输出编码链〔机械层补手 vs 纪律告知〕／⑤ **320K 柔和化文案**〔状况陈述＋「请判断当前情况是否需要压缩」〕＋**500K 底线「真硬打断＋必压缩」块形态**〔现状 500K 无模型面提醒、只有截断后告知；320K 的 ≤3 轮窗口语义去留随此定〕／⑥ 中性终态与结束自述实施的契约面形状〔结束原因类别取值、结束摘要落 journal 形态、跑分关闭开关与起跑前断言〕）。
- [ ] **S2 落码**：实施＋钉子（① 非交互版本入口冒烟钉；② 块表文案钉〔未压缩块无指针列则不得宣称「指针见上」〕；⑤ 硬提醒文案钉〔320K 无「现在就压」类祈使、保留授权句与目标档；500K 强制形态有钉〕；⑥ 中性终态钉〔原因类别可核／摘要常态化／无应答者不死锁／跑分开关关闭时行为〕＋ADR-0010 转录）。
- **2026-09-24 更正（用户裁决：⑩ 撤销 ⇒ 转 0bk）**：⑩「压缩行为面再裁决」**撤销**——浅压缩＝**实现面解析偏差**（模型意图正确：`压缩块: 1-62（全部已闭合块；工作现场保留）` 被静默丢弃 ⇒ 退化为「只压最旧一块」），用户裁「**压缩下限这个设计就不加也不留**」「**模型做的是对的但实现是错的**」；机械层压缩属**明确设计**、成本不单算 ⇒ **不改**。⇒ 不加下限／不做连号抑制与工作点标定的新机制；**解析修正单独立项 `### P1-0bk`**。量级留档＝窗口改写事件 21 次（12 压缩＋9 T1 截断）、其后 3 轮 miss 占 64%。
- [ ] **S3 真机**：随 0bm 合并轮跑真机（同轮做承接 ⑧／⑨ 首读；新摩擦随轮记录）。
- [ ] **S4 收口**：总结档＋判据对账（数字与收官 journal 对齐）；计数同步随提交批。
- **承接登记**：⑦ release 全量串行档＝**2026-09-24 已补跑**（exit 0／警告 0：orz-assurance 275/0、orz-loop 839/0/3）；⑧⑨ 首读见 S3。
- **仅记录（不入范围）**：c 编辑锚点重读往返成本／f 压缩截断频仍／i CRLF→LF worktree 行尾面（用户 2026-09-24「先观察」）。
- **案例库（不入计数）**：h＝`ORZ-PS1-BOM-001` 同族追加 ⑨「BOM 断言必须落字节面」（已随批落案例档与 README）。
- **2026-09-25 记账批入账（0bm 真机轮 `RUN-CLI-6ab6275c`）**：② 块表说明行口径已落码（「指针列与原文回放路径仅『已压缩/已截断』块具备」；**钉子待补**）／⑤ 320K 柔和化已落码（去「现在就压：」祈使，改「是否现在压缩、压缩哪些块由你判断」＋目标档建议；500K 强制窗「请你现在压缩」硬形保留；**钉子待补**）／**⑥ 勘定更正**＝中性终态**早已实施**（0bh `5e153de1`：`[RUN_END]` 解析＋`run_finished` 新字段＋schema＋钉子 5084），0bj ⑥ 原立项描述「实施核心未实施」为**误判**；**但 0bm 轮实证该通道静默失效**（137 工具轮未自述、`run_finished` 三键）⇒ 根因＝告知面只挂 pull 面 `blackboard_read section=guide`，**处置面随 `P1-0bs` ① 承接**。**④ 输出编码链**：本轮实证为**全谱**面（非仅 `orz web`）⇒ **执行面随 `P1-0bs` ② 同载**，0bj 闭合仍随 0bm。**残余**＝②⑤ 钉子、③ `cd /d` 告知落点、④ 设计档模板位置（docs 无 TEMPLATE 档，待查）。
- **仅记录（本轮随轮实录）**：PS 5.1 控制台回显乱码（F6，同 ④ 族）／压缩窗口摘要时点不可观测（F12，转 `P1-0bs` ⑥）／报告初稿被机械 emoji 剥离（F13，转 `P1-0bs` ⑦）。
- 入口：BACKLOG 0bj / [`0bi 轮报告`](docs/audits/0BI_FRICTION_CARRYOVER_2026-09-23.md) / [`中性终态设计档`](docs/NEUTRAL_TERMINATION_AND_MODEL_STOP_DESIGN_2026-09-22.md)。



## P2 — 生产化决策门
### 15. EVALUATION-CORPUS-FREEZE（P2；2026-09-13 立项，深审 S-13 注册）

- [x] **S1 考卷语料冻结（2026-09-13 完成，TB 2.1 官方 89 题线）**：pin `terminal-bench/terminal-bench-2-1@sha256:7d7bdc1c…` + 注册表逐题 sha256（89 题，与账面 lock/result 逐题全等）+ 本地 checkout `7131e437` + 批次 16/17/19/18/19 + 装置件 digest；存放形态 = 清单入仓、语料本体不入仓；发现本地 `dataset.toml` 对 `sanitize-git-repo` 过期（逐文件比对证明 88/89 等价、唯一差异为该题 `tests/test_outputs.py` 假密钥拆串写法，语义等价）。入口：[P2-15 记录](docs/audits/P2-15_CORPUS_FREEZE_AND_THRESHOLD_CALIBRATION_2026-09-13.md) / 清单 `evaluation/corpus-freeze/tb21-official-89-2026-09-13.manifest.json`。
- [x] **S2 阈值校准（2026-09-13 完成）**：① 官方三阈值工作点——命中率 ≥90% 在 116 试次 p10 91.29%/达标 106 条（91.4%，有区分度）；哨兵 ≤3 历史最大 2 次、245/245（尚未受压）；零 400 在 journal/agent 日志/trial.log 三面为零（须连 marker 集合登记）；② evaluation/holdout 阈值层干跑（6 题 development 小样本，oracle 隔离通过）实测封顶 `descriptive_only`、不产出 `threshold_set`，并查出 runner 产出与注册 schema **76 处不兼容**（acceptance 块 `requires_review` 不在枚举内 + 缺 `threshold_set`/`reasons`）。入口：[校准件](docs/audits/P2-15_CORPUS_FREEZE_AND_THRESHOLD_CALIBRATION_2026-09-13.md) / `evaluation/corpus-freeze/threshold-calibration-2026-09-13.json` / `evaluation/corpus-freeze/evaluation-threshold-dryrun-2026-09-13.json`。
- [x] **S3 前置——GAP-EVAL-RESULT-SCHEMA-DRIFT 已立案并同日修复（2026-09-13）**：实现对齐合约（系统档/`protocol_ref` 实算摘要/`{valid,checks}` integrity/counts 七键/§6 七维指标/多对多簇/带对照类型配对/逐案 `kind` red line/adjudication/`acceptance` 恒 `not_calibrated`/带真实摘要 artifacts/案例 id 与 revision fail-closed/`gsa eval` CLI 通路）；**schema 校验进测试面**（`ResultContractTests` 7 项钉子 + 主流程逐条断言），`test_evaluation_runner.py` 26/26，干跑 schema 0 错误；全量 assurance 1631/0（1 项既有不相关失败见下）。入口：[修复记录](docs/audits/GAP_EVAL_RESULT_SCHEMA_DRIFT_FIX_2026-09-13.md) / [结果 schema](evaluation/evaluation-result-v0.1.schema.json) / [测试](assurance/tests/test_evaluation_runner.py)。
- [x] **S3 路线按现状收敛（2026-09-13 用户说明：本项目无评审人）**：内部语料线的「密封 evaluation/holdout 分区 + 双人盲审 baseline」不具备条件 → 该线**停在 `not_calibrated`**（GAP 修复后机械保证：状态封顶 `descriptive_only`、acceptance 恒 `not_calibrated`、无 `threshold_set`），S3 降级为可选描述性记录、不作为阈值门；**本轮官方跑批不依赖它**（官方 verifier 即 oracle）。
- [ ] **试次数口径待确认**：用户指示 k=1，但官方榜单 CI 要求每题 ≥5 试次（`static_analysis.py::MIN_TRIALS_PER_TASK=5`）→ k=1 可上传 Harbor 但不可上榜单；入口已参数化 `K="${K:-5}"`。
- [ ] **另立候选（待裁决）**：GAP-DOC-INDEX-RANKING-TEST-FRAGILITY——既有脆弱用例 `test_search_p3_action_authorization`（文档索引前 10 位硬阈值；已核实与本批无关），建议改为可解释口径。
- [ ] S3 首轮真实 evaluation 跑批（runner 全链 + oracle isolation 生效）；S4 报告与闭合裁决。边界：排期在 0z S3/S4 之后。入口：[深审 §2 S-13](docs/audits/FULL_PROJECT_DEEP_REVIEW_2026-09-12.md) / [BACKLOG P2-15](docs/BACKLOG_AND_PRIORITIES.md) / `assurance/evaluation_runner.py`。


### IMPL-CONTROL-FABRIC（`partial`）

- [x] fail-closed 生产启用（2026-08-16 闭合）——默认翻转（未设置即强制，显式 `0|false|no|off` 影子，非法值 exit 2）；CLI run / ACP stdio / TUI 三入口接线；核查清单 ⑦⑨⑩⑪ 收口；`orz-acaf-provision` 供应工具 + 启动链。入口：[ADR-0011](adr/ADR-0011-authenticated-control-and-action-fabric.md) / [ACAF 设计](docs/AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md) / [fail-closed 审计](docs/audits/GAP_ACAF_SLICE2_FAILCLOSED_IMPL_AUDIT_2026-08-13.md)。
- [ ] Slice 3：ModeChangeTicket → `bump_policy_revision` 首个生产递增来源 + policy_digest 真摘要切换 + 会话级计数器 gate（D3-1）。
  - 成熟复用评估（2026-08-16，只读）：部分——HKDF-SHA256/HMAC 与单调计数器为成熟标准原语；ACP 模式切换为成熟先例。
- [ ] Slice 4：Windows Sandbox backend（D-11）。
  - 成熟复用评估（2026-08-16，只读）：明确——Windows Sandbox（Hyper-V）、AppContainer、Job Object 为成熟 OS 能力；GAK/P2 审计可直接支撑。
- [ ] 可选：检索车道 web_fetch activation 绑定接线；conformance capture 票据场景；normalize_lexical 单源化；ACP 会话路径接 ACAF。
  - 成熟复用评估（2026-08-16，只读）：部分——ACP 规范/xai-acp-lib、orz-paths、既有 D-13 机制与 fixture 体系可复用。


### OPS-PROTOCOL（`pending`；裁剪方向已定）

- [ ] 产出裁剪设计：删除安全保留为 host-owned 工具、跨环境桥接内部化、双执行器收敛单一参考（生产走 Rust 工具面）。
  - 成熟复用评估（2026-08-16，只读）：部分——Windows 回收站（BitBucket）与 XDG trash 为成熟 OS 约定；既有执行器作参考。
- [ ] 生产接线裁决（先验票，再由协议执行器执行）。
  - 成熟复用评估（2026-08-16，只读）：无——用户决策门。
- 入口：[协议](protocol/structured-operation-protocol-v0.1.md) / [Schema](protocol/structured-operation-protocol-v0.1.schema.json)。


### MODEL-RESIDUAL-PRESSURE-FOLLOWUP（P2；2026-08-31 二次讨论裁决登记，BACKLOG P2-11）

> 排期：设计轮（PULL 自描述 / retryable 分类位 / 依赖图）→ 设计定稿 → 实施放行；
> DC 清理已裁决可直接实施（P3 清理类，放行时入账）。入口：
> [讨论稿](docs/MODEL_RESIDUAL_PRESSURE_DISCUSSION_2026-08-31.md)（§8 裁决）/
> [BACKLOG P2-11](docs/BACKLOG_AND_PRIORITIES.md)。

- [x] PULL 自描述（2026-08-31 设计定稿 + S1/S2 + 审查修复完成，`partial`；S3 重建已随 0o T0 闭合（2026-09-07，后续 0p T2 / 0t S3 轮翻新），S4 实机复验待放行）：`blackboard_read` 增量头 + temporal 一次返回（零注入、8 工具面冻结）。入口：设计 `docs/PULL_SELF_DESCRIPTION_DESIGN_2026-08-31.md` / ADR §14.48 / [审查修复](docs/audits/P2-11_PULL_SELF_DESCRIPTION_S1_REVIEW_AUDIT_2026-08-31.md)。
- [x] DC 强制模板轮清理（2026-09-01 实施完成，闭合）：DC 机制全删 + plan 反例变体注册 + P3「DC 硬信号 4/6」退役 + schema/verifier/fixtures/测试收口。入口：[实施审计](docs/audits/P2-11_DC_FORCED_TEMPLATE_CLEANUP_IMPL_AUDIT_2026-09-01.md) / ADR-0010 §14.49。
- [ ] retryable 机械分类位：`Fail` 信封增 `retryable: bool`（确定性失败 false / 暂时性 true，错误码事实推导）；**S1 实施 + S2 测试 + 审查处理 O1–O4 已完成（2026-09-01，ADR-0010 §14.50）**；S3 重建已随 0o T0 闭合（2026-09-07，后续 0p T2 / 0t S3 轮翻新），S4 实机复验待放行。
- [ ] 依赖图实施：文件锚点链最小范围 + `blackboard_read section=deps` PULL 面 + 模型零改动 + F11 顺带闭合；**2026-09-01 设计定稿 + S1 实施（含全面审查处理收口）+ S2 测试完成（`partial`，orz-loop 642 lib 全绿）**；入口：设计 `docs/DEPENDENCY_GRAPH_MAINLINE_DESIGN_2026-09-01.md` / ADR-0010 §14.51 / [实施审计](docs/audits/P2-11_DEPENDENCY_GRAPH_IMPL_AUDIT_2026-09-01.md) / [审查处理记录](docs/audits/P2-11_DEPENDENCY_GRAPH_S1_REVIEW_AUDIT_2026-09-01.md)；S3 重建已随 0o T0 闭合（2026-09-07，后续 0p T2 / 0t S3 轮翻新），S4 实机复验待放行。
- [x] 工具名幻觉登记边界（不改名/不别名；fail-loud 自回正）——2026-08-31 裁决。
- [x] search_replace 锚点 / submit 两阶段维持现状（优化收益不足）——2026-08-31 裁决。


### COMPRESSION-LINGUISTIC-FORMAL-LAYER（P2；2026-09-02 讨论稿登记，BACKLOG P2-12）

> 排期：讨论稿（reference 路由）→ 转正式设计（域标注 + F4 失败目标聚合，更新
> CONTEXT_COMPACTION_DESIGN 注意事项槽渲染语义）→ 用户裁决 → 实施放行。入口：
> [讨论稿](docs/COMPRESSION_LINGUISTIC_FORMAL_LAYER_DISCUSSION_2026-09-02.md)
> （§3/§6 收口）/ [BACKLOG P2-12](docs/BACKLOG_AND_PRIORITIES.md) /
> [S1/S2 实施记录](docs/audits/P2-12_COMPRESSION_LINGUISTIC_FORMAL_LAYER_S1S2_IMPL_2026-09-02.md)。

- [ ] 域作为压缩参考（方案 A，2026-09-02 定案口径）：聚合行键 = F4 身份 (kind, id) + epoch 内累计 + 跨 marker 去重；域降级为行内序列标注；错误码行内集合；相对 run 起点墙钟首末时间；聚合状态归黑板；域标注 = 写时盖章 + 域段书签归并。**设计轮前置条件 + S1/S2 实施 + 全面审查处理（2026-09-02，orz-loop 654 全绿）+ S3 重建（2026-09-02 musl 三件套符号命中与冒烟通过）均已闭合**，入口：[审查处理记录](docs/audits/P2-12_COMPRESSION_LINGUISTIC_FORMAL_LAYER_REVIEW_HANDLING_2026-09-02.md)；S4 实机复验与 ADR-0010 转录待续。
- [ ] 失败目标聚合进注意事项槽：F4 聚合行渲染替换「最近 5 条截断错误」窗口语义；**S1/S2 渲染改造已随上条一并实施（2026-09-02）**，设计投影见 `CONTEXT_COMPACTION_DESIGN_2026-08-14.md` §4.4.4；ADR-0010 转录待实施闭合时登记。
- [ ] §5 离线验证切片（随 P2-12 放行后单独排期，未排期）：虚拟压缩点回放覆盖率 + 「概括」正确性（digest 计数 / 首末时间 / 错误码集合对账）；开放锚点覆盖率待 Centering 方向实施后再并入。见 [BACKLOG P2-12](docs/BACKLOG_AND_PRIORITIES.md)。


### BLACKBOARD-CONVERSATION-SCOPE-FOLD（P2；2026-09-03 设计定稿 + ADR-0010 §14.52 转录；B1–B3 已完成，B4 待放行，BACKLOG P2-13）

> 入口：[设计稿（v0.8 定稿）](docs/BLACKBOARD_CONVERSATION_SCOPE_FOLD_DESIGN_2026-09-03.md)
> / [BACKLOG P2-13](docs/BACKLOG_AND_PRIORITIES.md)。设计裁决链（v0.5 开放问题 R1–R6 收口 / 方案 B 主线 / 参数定档 v0.4–v0.8 / 评估复核）与 B1–B3 完整实施流水见全量快照 [`TODO_FULL_2026-09-09`](存档/todo/TODO_FULL_2026-09-09.md)。

- [x] **B1 会话化基础**（S1/S2 完成 2026-09-03）：(round, domain) 写时盖章 + 会话级 live 黑板续载 + conversation-relative 轴迁移。入口：[B1 实施审计](docs/audits/P2-13_B1_CONVERSATION_BASE_IMPL_AUDIT_2026-09-03.md)。
- [x] **B2 渲染折叠**（S1/S2 完成 2026-09-03）：折叠态渲染 + domain/round_from/round_to 展开参数 + 分区渲染 cap。入口：[B2 实施审计](docs/audits/P2-13_B2_RENDER_FOLD_IMPL_AUDIT_2026-09-03.md)。
- [x] **B2 复审处理**（2026-09-03）：receipt_id 守卫旁路修复 + 未达阈值展开 = 普通读取 + 展开目标 cap 保护 + 单域单段语义裁定。入口：[B2 复审处理](docs/audits/P2-13_B2_REVIEW_HANDLING_2026-09-03.md)。
- [x] **B3 契约与收尾**（2026-09-03）：空槽「（无）」统一 + 用户侧疲劳提醒（W=10MiB 三档）+ 存档单包 gzip + `session_archive` v0.2 事件 + plan-epoch 生产面退役（`--plan` 保留）。入口：[B3 实施审计](docs/audits/P2-13_B3_IMPL_AUDIT_2026-09-03.md)。
- [x] **B3 复审处理**（2026-09-03）：疲劳档位状态机收口（压缩轮数门槛移除）+ close-with-active-run 存档推迟补触发 + 存档 IO 移 blocking 池。入口：[B3 复审处理](docs/audits/P2-13_B3_REVIEW_HANDLING_2026-09-03.md)。
- [ ] **B4 验证**：S3 重建完成（2026-09-03：Linux musl 三件套 orz
  d4a37fdb，静态/符号/冒烟核证通过，见
  [B4 S3 重建记录](docs/audits/P2-13_B4_S3_BUILD_2026-09-03.md)）；
  S4 复验（web 通道 A/B、折叠态读取与展开、恢复、长会话遥测）待续。


### P2-14 CONTEXT-COMPACTION-FOLD-SNAPSHOT

### P2-0at 逐 query 归因口径与谱系（P2；2026-09-20 立项，BACKLOG 0at；来源＝S3 摩擦 N1）

> 入口：[`深挖档 §2`](docs/audits/0AR_S3_FRICTION_DEEP_DIVE_N1_N6_2026-09-20.md) / BACKLOG 0at。

- [x] **S1（B 面，小）**（2026-09-20 过夜批）：`unattributed_usable_count`＝批级可用 − Σ 逐 query 可用；**多 query 批限定落盘**（单 query 批 payload 逐字节不变，判据 ③）；契约面＝schema 可选字段＋`merged-multi-query` fixture 扩展＋check_repository 登记＋Python schema 校验同面（S3 归档回放 gap=4/6/3/6/0/0/0 与深挖档 §2.1 逐批一致）。
- [x] **S2（A 面，中）**（2026-09-20 过夜批）：`origin_query_assignments` 五规则（单 query 全 None／逐字／归一化／候选池回溯／leader 谱系）＋`query_ids_for` 单源（`query_summary` 与谱系同 id 域）；多 query 批工具证据 ledger 条目落可选 `origin_query_id`；判据＝恒等式钉＋覆盖率由 leader 兜底构造 100 %（S3 归档无证据级 search_query，真机复核随 S3 收取）＋端到端钉（合并派发 ledger origin=query_summary[0]）。
- [ ] **S3 口径同步**：设计稿 §5.4 与 ADR 转录同步（归因修好前不得据逐 query 披露做续派依据）。


### P2-0av 检索批次数读数落盘面（P2；2026-09-20 立项，BACKLOG 0av；来源＝S3 摩擦 N3）

> 入口：[`深挖档 §4`](docs/audits/0AR_S3_FRICTION_DEEP_DIVE_N1_N6_2026-09-20.md) / BACKLOG 0av。

- [x] **S1 落码**（2026-09-20 过夜批）：`dispatch.rs` 批收尾写 `mechanical_audit_update{kind:"retrieval_batch"}`（五值与 batch_close 单源 helper 同值；Err 臂不落）；kind 枚举 8 值＋schema **按 kind 条件分支**（五键批读数／均一四键）＋契约钉改写＋Python 校验器分支（Rust↔Python 同步，历史 journal 回放零错误）；新正例 fixture＋check_repository 登记。
- [ ] **S2 可核判据**：真机 run 中「倒数行读数 ＝ 该批 `usable_source_count`」可机械重算（0ar 判据 7 后段转可核）。（机制面已落：倒数/assessment/journal 读数三面共用 `usable_source_count` 单源；真机重算演示随下一轮官方跑批收取。）
- [ ] **S3（可选）**：headless run 尾无条件落侧车（或评测开关）——使整条模型面消息可审计。



### P2-0bb 命令行引号与长行取用摩擦（P2；2026-09-20 用户令立项〔「F13/15均立项」〕，BACKLOG 0bb；来源＝0am 狗粮轮 `RUN-CLI-6aafc998` 摩擦 F15）

> 入口：[`0am 报告 §5-F15`](docs/audits/0AM_DECODER_FORM_REPLAY_2026-09-20.md) / BACKLOG 0bb。

- [ ] **工具面**：给出稳定取行面（按行号直读的结构化路径／超长行截断标记类），不做 shell 语义适配、不新增工具；判据＝同类动作不再连续 ≥3 轮试错。
- [ ] **文档面**：账本超长条目切行（随既有瘦身机制，不新立机制）。


### P2-14 CONTEXT-COMPACTION-FOLD-SNAPSHOT（P2；2026-09-04 设计定稿；S1–S3 已收口（S3 2026-09-10 滞后入账），S4 待续）

> 入口：[设计稿](docs/CONTEXT_COMPACTION_BLACKBOARD_FOLD_DESIGN_2026-09-04.md)
> / [BACKLOG P2-14](docs/BACKLOG_AND_PRIORITIES.md) / ADR-0010 §14.54。S1 落地明细（第三条路轮章 / 快照入口 / v0.3 A–E 装配 / run_template_compact 接线）见全量快照 [`TODO_FULL_2026-09-09`](存档/todo/TODO_FULL_2026-09-09.md)。

- [x] 设计定稿与裁决收口（2026-09-04：R1–R5 按推荐定案、总量 20K 定档；ADR-0010 §14.54 转录；索引登记 AUTH-COMPACTION-FOLD-SNAPSHOT）。
- [x] **S1 实施 + 全面复审处理收口**（2026-09-04）：r_keep 接线验证 + render_fold 快照入口纯函数 + v0.3 A–E marker 装配与 run_template_compact 接线 + §7 单测矩阵 1–7；orz-loop lib 720 passed。入口：[P2-14 S1 复审处理](docs/audits/P2-14_S1_REVIEW_HANDLING_2026-09-04.md)。
- [x] **S2 压缩 e2e**（2026-09-04 收口）：rhythm / fallback / session_end / 恢复预检全串行绿——滚动单 v0.3 marker 逐请求与收尾断言，恢复预检后 marker 逐字节原样（§7 矩阵第 8 项复验）；orz-loop lib 722 / orz-host ACP 43。入口：[P2-14 S2 e2e 审计](docs/audits/P2-14_S2_E2E_2026-09-04.md)。
- [x] **S3 Linux musl 重建**（沿用 ORZ-BUILD-MOUNT-001 契约）——2026-09-07 随 0o T0 闭合（0p T2 / 0t S3 轮翻新）。（2026-09-10 补勾；入口：BACKLOG 0o 滞后入账 / 14 小节）
- [ ] **S4 实机复验 + 遥测**：marker 实际字符分布、块 B/C 溢出频率、压缩后 blackboard_read 跟随调用频率、restore 后 marker 可用性。

## P3 — 收尾 / 清理

- [ ] EVIDENCE-LOCAL-BROWSER：Python 路径（retrieval_workflow / evidence_store / pdf_evidence）按 ADR-0010 重新符合性审查或退役。
  - 成熟复用评估（2026-08-16，只读）：明确——生产 Rust 已复用 CDP（Chrome）与 pdf_oxide；Python 路径按此审查/退役。
- [ ] GATE-CHAIN：分层 gate 链与融合 runtime 的最终接线随切片审计复核。
  - 成熟复用评估（2026-08-16，只读）：无——复核既有 assurance 链接线。
- [ ] （可选）提示词补列 observed scope 合法枚举（`full_text_observed` / `partial_text_observed` / `metadata_only`）——P0-B 步骤 6 复核观察登记，verifier 已机械兜底，暂不实施。
  - 成熟复用评估（2026-08-16，只读）：无——提示词枚举补列。
- [ ] V11-IMPL-003：Global Review receipt 与真正审查结论严格分离——复核并登记闭合或转 gap。
  - 成熟复用评估（2026-08-16，只读）：无——复核登记。
- [ ] V11-IMPL-007：Toolbar/run-history 数据源统一到 ORZ session ownership、旧路径残留——复核并登记闭合或转 gap。
  - 成熟复用评估（2026-08-16，只读）：部分——orz 即 Grok Build fork，原 toolbar/session 代码在仓库内；Codex app-server 投影为成熟参考。
- [ ] orz-host 既有 flaky（`approval_allow_persists_for_identical_bash`）——复核并登记闭合或转 gap。
  - 成熟复用评估（2026-08-16，只读）：无——测试修复。
- [x] DC 硬信号 4/6：`same_module_no_evidence` / `key_surface_unexamined` 接线（原建议并入 P0-B，批次已闭合，独立待办）——**2026-09-01 随 P2-11 DC 强制模板轮清理一并退役**（信号与机制随删除，不再单独接线；2026-08-31 裁决登记）。

## 审计登记边界（条件触发，不占当前优先级）

- [x] GLM-2026-09-04 外部只读审查候选处置（2026-09-06 用户裁决：F1 /
  R-1 / R-2 / R-3 / 观察 (c) 完成，F2 登记 GAP-APPROVAL-PROMPTER，同日
  排期 0n 后延期）
  ——入口：[登记审计](docs/audits/GLM_EXTERNAL_REVIEW_REGISTRATION_2026-09-04.md)
  / [处置审计](docs/audits/P0_GOV_GLM_DISPOSITION_AND_TASK_D_S2_SCHEDULE_2026-09-06.md)。
- [x] orz read_file `.gsa` 符号链接旧回归测试与 Task C canonical 沙箱语义
  冲突（GAP-GSA-SYMLINK-STALE-TEST，2026-09-06 复核登记）——已按用户裁决
  对齐 Task C：旧测试改写为拒读安全回归测试（orz `a29f7377`）；连带观察
  （permission.rs `.gsa` 白名单对 read_file 不可达）登记于 BACKLOG 00a 待
  裁决。入口：[处置审计 §5](docs/audits/P0_GOV_GLM_DISPOSITION_AND_TASK_D_S2_SCHEDULE_2026-09-06.md)。
- [ ] orz-host 可选后端接线（lsp / memory / 图像 / 视频 / MCP）：接线时翻转能力访问器并补翻转测试（FUS-TOOL-PROBE 边界）。
  - 成熟复用评估（2026-08-16，只读）：明确——LSP/MCP 为成熟开放标准，仓库内已有 orz-mcp；图像/视频走成熟服务 API。
- [ ] headless 计划模式能力信号（plan 模式探针当前以交互用户信号代理，未来 headless 计划模式需独立信号）。
  - 成熟复用评估（2026-08-16，只读）：部分——Codex headless/plan 模式可参考。
- [ ] 下一次真实运行捕获自然携带 23 工具分区 journals（当前 12 个为已提交 fixtures 重建）。
  - 成熟复用评估（2026-08-16，只读）：无——纯运行收集。
- [ ] B-1 后续：canonical URL/host 级去重留待预筛步骤 3；若 host/loop 拆为跨进程边界，补 `ToolResult.structured` 序列化契约。
  - 成熟复用评估（2026-08-16，只读）：部分——成熟 url 库/PSL 与 serde/JSON Schema。
- [ ] ORZ-RECOVERY-TOOL-OUTCOME：崩溃恢复工具结果词汇（`TOOL_NOT_STARTED` / `TOOL_OUTCOME_UNKNOWN` 合成 Tool 消息 + 只重试只读/幂等指引）——出现恢复面 400 或副作用未知证据时实施（单点修复）。
  - 成熟复用评估（2026-08-16，只读）：部分——Codex/Grok 工具生命周期与句柄化控制可参考。
- [ ] ORZ-STAGNATION-TOOL-SIGNAL：停滞守卫「同工具同参数」信号——出现「同参循环且输出持续变化」证据时在 stagnation guard 内加最小计数信号。
  - 成熟复用评估（2026-08-16，只读）：明确——仓库内已有 Grok 血统 stagnation 模块（orz-assurance/orientation/stagnation.rs）可直接扩展。

## 近期已闭合（供核对，不计入开放项）

- [x] FUS-TOOL-PROBE：P0-A 步骤 1-7 与 P0-A-2 全部闭合（ADR-0010 v1.8，23 个工作工具单一探针面）。
- [x] FUS-SOURCE-WEIGHTING-IMPL：来源加权实现闭合（机械三档 + 机器可读种子名单 + 模型加权标注）。
- [x] GAP-ENCODING-GATE：机械编码门控闭合。
- [x] GAP-ACAF-SLICE1 / SLICE2A / SLICE2B / FAILCLOSED 与 GAP-DENIAL-POLICY-REVISION：ACAF 实施切片闭合（fail-closed 生产启用已随 P2 IMPL-CONTROL-FABRIC 闭合）。
- [x] OPS-PROTOCOL 审查判定登记（裁剪方向定案；裁剪设计待产出）。

> **2026-09-13 晚补记（P0-0ac 路径改定）**：用户裁决**全面转向本地检索**（服务端 web_search 被 DeepSeek 下架，官方文档明载"内置工具忽略"；flash 路由静默失绑 4/4；v4-pro 残余通道随 2026-09-14 12:00 路由切换预计关闭）。**偏离登记：检索后端 服务端 web_search → 本地分段检索**（agent 侧能力，TB 2.1 官方口径四要素不动，不构成违反；可比性注记入台账）。**S1′ 本地检索探针完成**：DDG html 纯 HTTP 可用（12/12 解析、命中 9.8 条、TTFB 冷 11 s/热 5.8 s）；Bing HTML 有结果块、提取器需修（非 bot 墙）、RSS 相关性差不作主通道；Google 无浏览器弃用；串行流水线首 SERP p50 21 s（Bing 先验白等）不可行 ⇒ **10 s 截止落法 = 多引擎并行竞速 + 预热，默认值待容器内复验后定**；页面抓取成功率 83%。入口：[`0AC_S1_PROBE_RECORD_2026-09-13` §6](docs/audits/0AC_S1_PROBE_RECORD_2026-09-13.md) / 设计稿 §9 修订。**下一步 S2 机器合约（双路径：本地分段为主、流式为恢复预留），待放行**。
> **2026-09-13 晚二次更正（S1′ 代理假象）**：用户指出 DDG 早已排除 ⇒ 复核证实首测走系统代理（127.0.0.1:7890），**直连（= 容器形态）下 Bing HTML TTFB 0.4 s、DDG/Google 不可达**——0v R4「duckduckgo 本地不可达」实证成立。更正：默认引擎集 = **Bing HTML 直连单引擎**（提取器需重写）；**截止按引擎单独计时 + 整体兜底 30 s**（用户裁决）。探针记录 §7 / 设计稿 §9.4 已更正。
> **2026-09-13 晚三补（用户裁决：引擎选路与工具面）**：① cn.bing.com = 无代理默认引擎；② 有代理时引擎交模型自选（接 0v §6 留存的引擎自选方案）；③ **工具面保留 `web_search` 名称**，实现改指本地检索（8 工具面冻结不破），模型可见描述如实标注本地来源。已入设计稿 §9.6。

> **2026-09-13 狗粮线与摩擦台账**：用户裁决开跑「orz 制作 orz」线（本会话只作传话与设计，orz 真机直跑本工作区）。首轮两 run：`RUN-CLI-6aa6a868` 被装置侧锁死为只读（ACAF signer env 契约错位 + 发布包 manifest 哈希过期，229 次拒绝无 cause；模型诚实未伪造进度——正面样本）；修复后 `RUN-CLI-6aa6ac42` 由 orz **独立完成 0ac S2 机器合约**（28 文件、fixture 含负例、门禁 `valid: true`、S3 待放行）。新建活页台账 docs/FRICTION_LEDGER.md（AUTH-FRICTION-LEDGER；2026-09-14 删档），F-001…F-006 入账，F-003 转 0ac S3 `cause` 活体验收样本；**下轮传话固定附带 run 末摩擦自报指令（直写该台账）**。S3 待放行。

## P3 — 收尾 / 清理
## 审计登记边界（条件触发，不占当前优先级）

- [x] GLM-2026-09-04 外部只读审查候选处置（2026-09-06 用户裁决：F1 /
  R-1 / R-2 / R-3 / 观察 (c) 完成，F2 登记 GAP-APPROVAL-PROMPTER，同日
  排期 0n 后延期）
  ——入口：[登记审计](docs/audits/GLM_EXTERNAL_REVIEW_REGISTRATION_2026-09-04.md)
  / [处置审计](docs/audits/P0_GOV_GLM_DISPOSITION_AND_TASK_D_S2_SCHEDULE_2026-09-06.md)。
- [x] orz read_file `.gsa` 符号链接旧回归测试与 Task C canonical 沙箱语义
  冲突（GAP-GSA-SYMLINK-STALE-TEST，2026-09-06 复核登记）——已按用户裁决
  对齐 Task C：旧测试改写为拒读安全回归测试（orz `a29f7377`）；连带观察
  （permission.rs `.gsa` 白名单对 read_file 不可达）登记于 BACKLOG 00a 待
  裁决。入口：[处置审计 §5](docs/audits/P0_GOV_GLM_DISPOSITION_AND_TASK_D_S2_SCHEDULE_2026-09-06.md)。
- [ ] orz-host 可选后端接线（lsp / memory / 图像 / 视频 / MCP）：接线时翻转能力访问器并补翻转测试（FUS-TOOL-PROBE 边界）。
  - 成熟复用评估（2026-08-16，只读）：明确——LSP/MCP 为成熟开放标准，仓库内已有 orz-mcp；图像/视频走成熟服务 API。
- [ ] headless 计划模式能力信号（plan 模式探针当前以交互用户信号代理，未来 headless 计划模式需独立信号）。
  - 成熟复用评估（2026-08-16，只读）：部分——Codex headless/plan 模式可参考。
- [ ] 下一次真实运行捕获自然携带 23 工具分区 journals（当前 12 个为已提交 fixtures 重建）。
  - 成熟复用评估（2026-08-16，只读）：无——纯运行收集。
- [ ] B-1 后续：canonical URL/host 级去重留待预筛步骤 3；若 host/loop 拆为跨进程边界，补 `ToolResult.structured` 序列化契约。
  - 成熟复用评估（2026-08-16，只读）：部分——成熟 url 库/PSL 与 serde/JSON Schema。
- [ ] ORZ-RECOVERY-TOOL-OUTCOME：崩溃恢复工具结果词汇（`TOOL_NOT_STARTED` / `TOOL_OUTCOME_UNKNOWN` 合成 Tool 消息 + 只重试只读/幂等指引）——出现恢复面 400 或副作用未知证据时实施（单点修复）。
  - 成熟复用评估（2026-08-16，只读）：部分——Codex/Grok 工具生命周期与句柄化控制可参考。
- [ ] ORZ-STAGNATION-TOOL-SIGNAL：停滞守卫「同工具同参数」信号——出现「同参循环且输出持续变化」证据时在 stagnation guard 内加最小计数信号。
  - 成熟复用评估（2026-08-16，只读）：明确——仓库内已有 Grok 血统 stagnation 模块（orz-assurance/orientation/stagnation.rs）可直接扩展。

