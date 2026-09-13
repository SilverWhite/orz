# ORZ 待办项记录（TODO）

> 状态：living（实施勾选清单）；建立：2026-08-14。
> 定位：面向后续实施与改动的操作型清单，派生自 [`docs/BACKLOG_AND_PRIORITIES.md`](docs/BACKLOG_AND_PRIORITIES.md) 的未闭合项；优先级、决策门与状态权威仍是 BACKLOG，设计权威是 ADR-0010 / ADR-0011，召回路由是 [`CLI_PROJECT_INDEX.md`](CLI_PROJECT_INDEX.md)。本文件只登记指针与勾选状态，不新增设计裁决，不重复维护设计细节。
> 维护纪律：完成一项勾选一项，并同步 BACKLOG 与索引状态；新增或调整优先级只改 BACKLOG；删除或归档条目前先回查索引入口；每次改动保持本文件与 BACKLOG 的入口一致性。
> 入口：索引 AUTH-TODO（2026-08-14 登记）。
> 2026-08-31 清理轮：已闭合项压缩为单行核对条目（实施细节以 BACKLOG 变更记录与审计文档为准）；历次计数流水不再在快照重复；勾选状态以 BACKLOG 为权威，本次仅对 BACKLOG/索引已声明闭合的滞后项补勾，未闭合项原样保留。备份：`%TEMP%\TODO.md.bak-20260831`。
> 2026-09-09 整理轮：P0-GOV 00/00a、P0-0p、P1-0q 等已闭合区再次压缩为单行核对（明细见全量快照）；开放项路由同步至 BACKLOG 当前口径。全量快照：[`存档/todo/TODO_FULL_2026-09-09.md`](存档/todo/TODO_FULL_2026-09-09.md)。
> 2026-09-10 滞后入账轮：0o T0（2026-09-07）已达成的 S3 型闭合——0b 验证② / P2-14 S3 / P2-11×3 S3——按 BACKLOG 各条目小节入账后同步补勾与状态注记；勾选状态以 BACKLOG 为权威。
> 2026-09-12 审查入档轮：全项目只读深审报告入档（[`docs/audits/FULL_PROJECT_DEEP_REVIEW_2026-09-12.md`](docs/audits/FULL_PROJECT_DEEP_REVIEW_2026-09-12.md)；不立项、不动计数——0v-C 两项 P0 已被同日 orz `ba934af8` 修复闭合覆盖且触发源实锤与报告独立判断（URL 无痕改写）吻合，账本三处同步断裂已回补，其余 P1/P2 与体系面发现留用户裁决，BACKLOG 治理注记为登记权威）。

## 使用说明

- `[ ]` = 待办；`[x]` = 已完成（保留单行供核对，不计入开放项）。
- 每项标注 canonical ID / 优先级 / 关键内容 / 入口；同一概念只出现一次，不复制 BACKLOG 的决策记录。
- 已闭合项单行核对格式：`[x] <ID>：<一句话>（闭合日期；入口：<文档/审计>）`。

## 开放项路由（2026-09-09 同步；勾选与计数权威在 BACKLOG）

- 未闭合总数：**26 项**（BACKLOG 计数口径；逐次计数流水见 [`docs/BACKLOG_AND_PRIORITIES.md`](docs/BACKLOG_AND_PRIORITIES.md) 未闭合计数，TODO 不重复维护）。TODO `[ ]` 明细含父/子项，计数以 BACKLOG 为准。
- P0：0b 验证③④（⑤ 89 题独立用户门）；0d 后续 3/4/5 S4 复验；0j W1-R1 S4 复验 + W3-R3 余项 + W4-R4 S5-2 总项；0l ⑥⑦（⑧ TER 见 [`TODO2.md`](TODO2.md)）；0m S3/S4；0n GAP-APPROVAL-PROMPTER（延期，S1 设计定稿前置）；0o T3 批次 W2 chunk3 + T4 批次 L + T5 批次 O + T6 统一收口；0t S4 实机复验待续；0u R4 15 题官方复跑（0t S4 载体，2026-09-10 放行）；0v 检索引擎 SERP + browser_control 分类修正（**2026-09-12 用户裁决闭合入账：S1–S4 全部转 `implemented`，计数 26 → 25；0v-C 修复落码并**闭合**（`ba934af8`：回传落盘 `event_sha256` + 四处调用方线程化 + 全链重放钉子；根因实锤 = 漏斗无痕改写，Run A 4 处断链前序行全为含 URL 的 `model_output`））**；0w TB 4.0 单题摩擦探针（第三跑成立并跑完（2026-09-11））；0y NP1 机械身体集成支线（2026-09-11 用户裁决立项，实施未开始；§14.2 验证载体已裁决引入模拟器（2026-09-12），S1 载体搭建待放行，M0 定版为首个无依赖里程碑）；0z 真机资源安全边界与崩溃收尾（2026-09-12 用户裁决立项，设计完成待放行实施；S1–S4，**不与 0v-C 搭车**）。（0x 初始轮中立问询已于 2026-09-11 S1–S4 全部闭合转 `implemented`，计数 27 → 26；**0v 闭合 26 → 25、0z 立项 25 → 26**；ADR-0010 §14.66 / v1.67。）
- P1：FUS-COMPONENT-REGISTER 组件审计；GAP-WINDOWS-EVIDENCE 三项；IMPL-DEEPSEEK-TRANSPORT live 晋级证据；ORZ-SESSION-CONTEXT-MONITOR 四项。
- P2：IMPL-CONTROL-FABRIC Slice 3/4；OPS-PROTOCOL 裁剪与接线裁决；P2-11 余项（PULL 自描述 S3/S4、retryable 分类位、依赖图 S3/S4）；P2-12 S4 复验；P2-13 B4；P2-14 S3/S4。
- P3：EVIDENCE-LOCAL-BROWSER、GATE-CHAIN、observed-scope 枚举、V11-IMPL-003、V11-IMPL-007、orz-host flaky。
- 审计登记边界（条件触发，不占当前优先级）：orz-host 可选后端、headless 计划信号、23 工具分区 journals、B-1 后续、ORZ-RECOVERY-TOOL-OUTCOME、ORZ-STAGNATION-TOOL-SIGNAL。
- 已闭合分组（单行核对见下）：P0-GOV 00/00a、P0-E、0c、0d 主项与后续 1/2/6/7/8、0e、0f、0g、0h、0i、0k、0p、P0-B、P0-C、P0-C2、P0-D、P1-0q、P1 已闭合项、P2-10 全部闭合。

## P0 — 当前工作集

### P0-GOV 全项目宏观架构对齐与门禁修复（最优先阻断项，2026-09-04 登记；**2026-09-06 全部闭合**）

> 入口：[首轮审查报告](docs/audits/GLOBAL_ARCHITECTURE_AND_INTEGRITY_AUDIT_2026-09-04.md) / [深层审查报告](docs/audits/GLOBAL_ARCHITECTURE_DEEP_AUDIT_2026-09-04.md)；BACKLOG 00；AUTH-GLOBAL-ARCHITECTURE-AUDIT。完整勾选明细见全量快照 [`TODO_FULL_2026-09-09`](存档/todo/TODO_FULL_2026-09-09.md)。

- [x] Phase 1 门禁与编译紧急修复（2026-09-04 闭合）：Markdown 断链修复 / run-event v0.2 payload 夹具映射补齐 / `orz_source_manifest.sha256` 重算 / Rust 告警清零 / 门禁 Exit 0。
- [x] Phase 2 仓库卫生清理与 Git 规范化（2026-09-04 闭合）：根目录 31 个临时调试目录与遗留调试文件清理 + `.gitignore` 收拢本地测试输出。
- [x] Phase 3 权威与产品对齐（第一批，2026-09-04 闭合）：ADR-0010 导言与主 README 过时描述重写 + `architecture/current/` 产品面架构投影扩充 + `cargo check --workspace` 64 members 零告警零错误。
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

### P0-F FUS-BENCHMARK-FULL-EXEC（`pending`=实施完成待验证；P0，2026-08-18 用户裁决实施）

> 入口：[设计](docs/BENCHMARK_FULL_EXEC_DESIGN_2026-08-18.md)；ADR-0010 §14.24；BACKLOG 0b。orz 子模块 3f43478 / 4e9e61b。

- [x] 实施完成（2026-08-18）：权限层 Benchmark 两轴参数化 + 探针 BenchmarkFull + console `workspace.run_terminal` 注册 + CLI `--allow-shell`/`--allow-network` + `tb_agents/orz.py` 透传 + 审查收口；验证① 全量测试通过（含修复既有测试漂移 c4772fc）；折叠 400 根因修复 S1-S5 闭合（处理文档 LEDGER_FOLD_MARKER_INDEX_FIX_HANDLING）。
- [x] 验证②：Linux musl 重建（ORZ-BUILD-MOUNT-001 契约，输出 `D:/tb-eval/orz-linux`）——2026-09-07 随 0o T0 闭合（暂缓由 0o 集中批放行解除；产物后经 0t S3 / 0.4.0 轮翻新）。（2026-09-10 补勾；入口：BACKLOG 0b/0o）
- [ ] 验证③（2026-08-18 修复后复验一次，机制断言全过、reward 项未达标，待加预算重跑）：单题 make-doom-for-mips——0 异常、无 400、6 笔订单→5 组票据零拒绝、压缩后会话继续；reward 0=墙钟内未产出可运行 ELF（非机制回归）。
- [ ] 验证④（用户指示暂缓）：2–3 题交叉（compile-compcert、hf-model-inference 等 build/run 与网络类）。
- [ ] 验证⑤（用户指示暂缓）：`run_official_2.1.sh` 89 题 5 批。
- [ ] 闭合：验证全过 → BACKLOG/TODO/索引状态同步，未闭合 27 → 26。

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

### P0-0k RETRIEVAL-ORCHESTRATION-MECHANICAL（P0；2026-08-30 定稿；第一批 + 第二批实施 + S4 实机复验闭环 2026-08-31）

- [x] 全部闭合（2026-08-31）：双模式定案（local_browser 可用仅 browser_read / 不可用仅 web 族）+ 引擎 SERP Google 主序 + 原生兜底 + 第一批五项（S1/S2 + S3 重建 + S4 实机复验）+ Google 门禁观察实验（多轮实机）+ 主面封存 browser_read + 第二批（project_doc_index v2 / 会话级 tab 池 + 同轮多页并行 + DNS 缓存 / 委托契约复杂度分档）S1/S2 + 方向 C（删除 [RESULT_JSON] 组织块契约）S4 实机复验闭环（未闭合 32 → 30）。入口：BACKLOG 0k / ADR-0010 §14.45/§14.46 / S4 复验记录。

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

### P0-0m GSA-SESSION-VOLUME-BOTTOM-LAYER（P0；2026-09-06 设计定稿，同日用户裁决放行，排期实施）

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
- [ ] S3 接线复验：补读链/run_tests 窗口端到端（`.gsa` symlink 会话卷
  实机构造）+ GAP-GSA-SYMLINK-STALE-TEST 演进注记。
- [ ] S4 收口：索引/BACKLOG/TODO 同步 + 门禁 Exit 0。

入口：[设计](docs/GSA_SESSION_VOLUME_BOTTOM_LAYER_DESIGN_2026-09-06.md) /
[ADR-0010 §14.56](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) /
[BACKLOG 0m](docs/BACKLOG_AND_PRIORITIES.md)。

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
- [x] **本轮定性 = k=1 筛查轮（2026-09-13 用户裁决）**：与项目此前 k=1 89 题批次同形——严格遵守官方口径设置，但**仅 k=1**；`K` 参数默认 5（榜单口径），本轮以 `K=1` 起跑。
- [x] **本轮正文 = orz 自校验 + 摩擦项发现（2026-09-13 用户裁决，分数只作参考）**：优先序 ① 0z 新增面（预检门/回收阶梯/Job 硬上限/七族事件）真机整轮实测与前几轮死机形态是否复现；② 摩擦项逐项归因（orz 侧/模型侧/装置侧）→ 优化候选；③ 成绩只作参考。
- [ ] **参照线（软，不卡死）**：k=1 成绩达官方 DeepSeek-V4.1-Flash 的 TB 2.1 公布值 **90.6**（`api-docs.deepseek.com/updates/` 2026-09-10 条目）才值得补 k=5 取正式成绩；**未达不阻断本轮**。读数件：`scripts/tb21_round_gate.py`（R1 同口径回测 58/89 = 65.17%）。
- [ ] **摩擦清单产出（静默旁路）**：`scripts/tb21_friction_scan.py` 把整轮 journal/日志折叠成摩擦清单并归因；**R1 基线已扫描入档**（33 试次真工具失败 / 15 试次模型侧工具名错 / 42 框架信号 / 23 角色拒绝 / 11 权限拒绝 / 9 哨兵 / 0 资源事件），本轮结果与之对照。
- [ ] **两件读数件共同契约（2026-09-13 用户指示）**：**静默旁路、不阻断跑分流程**——只读、容忍半程数据、`--quiet` 零输出、**退出码恒 0**、不得接入跑批链作为阻断步；只作跑批中/后的快速初步分析。

### P0-0p 模型自信息面补强与 `.gsa` 两段门（2026-09-07 设计定稿同日排期；**T0/S1/S2/T2/S4/S5 全部闭合 2026-09-08**；BACKLOG 0p 转 `implemented`）

> 设计：[`BLACKBOARD_SELF_HISTORY_AND_GSA_TWO_STAGE_GATE_DESIGN_2026-09-07`](docs/BLACKBOARD_SELF_HISTORY_AND_GSA_TWO_STAGE_GATE_DESIGN_2026-09-07.md) / ADR-0010 §14.61；索引：AUTH-BLACKBOARD-SELF-HISTORY-GSA-GATE。完整勾选与实施流水见全量快照 [`TODO_FULL_2026-09-09`](存档/todo/TODO_FULL_2026-09-09.md)。

- [x] T0：orz 版本 bump 0.3.2（`7b00bbc9`）。
- [x] S1 黑板补强：`failures_only` 聚合面 + `search` 字面检索（≤20 行）+ 工具描述教学 + turn_count 真实计数（orz `928dceb3` + 复审处理 `fd46d4f9`；F-C 治本转 P1-0q）。（2026-09-07 闭合）
- [x] S2 两段门：内部区首读通知信封 → 二读放行 + 状态会话卷级持久化 + key 拦截五漏斗全卷零 sk- + 结构化 policy_denial（orz `7d7d89e7` + 复审处理 `542c35d5`）。入口：[复审处理审计](docs/audits/0P_S2_REVIEW_HANDLING_2026-09-07.md)。（2026-09-07 闭合）
- [x] T2 双平台重建：Windows 三件套 0.3.2 哈希锁定 + Linux musl 翻新 + VM 换装 + DryRun 全对 + enforcement-probe high-nist 19/19。入口：[T2 审计](docs/audits/0P_T2_DUAL_PLATFORM_REBUILD_2026-09-08.md)。（2026-09-08 闭合）
- [x] S4 重跑 train-fasttext（tf-selfhistory-032）：判据表全项通过（两段门审计对首次生产落账 / 全卷零 sk- / 命中率 96.37%）；任务未过 = 环境缺 fasttext（→ 0r）。入口：[S4 分析](docs/audits/0P_S4_TRAIN_FASTTEXT_TF_SELFHISTORY_032_ANALYSIS_2026-09-08.md)。（2026-09-08 闭合）
- [x] S5 收口：BACKLOG/TODO/索引 v2.63/ADR-0010 §14.62 同步 + manifest 重算 + 门禁 Exit 0。（2026-09-08 闭合）

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

### P0-0v 检索引擎 SERP 接入与 `browser_control` 车道分类修正（2026-09-10 立项；**第二批（软备忘 + 0v-A 取证面合批）S1–S4 全部执行完毕（S4 判据 1/2/6/7/11/12 成立、8 成立、3/4/5 部分成立、9/10 未取得）；0v 闭合与判据遗留/0v-C 缺口立项留用户裁决**）

> 裁决：navigate 被拒应满足模型需求 + 坐实 DeepSeek 后端慢（调查已闭合）+ 按既有 SERP 设计补实现（候选 Bing）；区域固定 `en-US`；Bing 无登录态污染专项优化；低质量域名加权复用既有 `SourceWeightConfig`（标注 + 稳定排序，不硬过滤，不承担恶意域识别）。
> 设计：[`RETRIEVAL_ENGINE_SERP_SEARCH_ACTION_DESIGN_2026-09-10`](docs/RETRIEVAL_ENGINE_SERP_SEARCH_ACTION_DESIGN_2026-09-10.md)；BACKLOG 0v。

- [x] S1 代码（2026-09-10 完成）：`risk_class` browser_control ReadOnly 豁免 + `search` 动作（引擎链/失败备忘/SERP 有机结果提取/URL 解码/pacing/信封）+ 低质量域名加权接线（`SourceWeightConfig` 注入 search 编排）。
- [x] S2 测试（2026-09-10 完成）：纯函数单测（引擎表/失败备忘/可用引擎选择/pacing 上下限/解析与 CAPTCHA/URL 解码/加权稳定排序/字段上限/固定表达式 Bing 广告排除）+ 动作信封与参数校验 + 搜索上限失败态 + 车道门测试（`write_gate None`、ReadOnly、action_category=read）。S2 发现并修复首次 search 误等 5s 冷却基值的实际缺陷。
- [x] 复审第 1 轮（2026-09-10，设计 §5.3）：P1-1 Bing 跳转解码（`a1<base64url>` + URL 校验 + 原始链接回落）、P2-1 `engine_attempts` 引擎标注（成功/失败/跳过都显式）、P2-2 取消整包 8KiB 截断改字段上限（url 2048 取代整包截断）、P2-3 反污染下沉（启动层 `--window-size` + 会话层尽力而为、白名单收窄）、P2-4 车道 SERP 预算方案 A。
- [x] 复审第 2 轮（2026-09-10，设计 §5.4，P2-4 专项）：P1-1 拒绝信封字段补 schema 登记 + 2 个 fixture 进 `check_repository`；P2-1 会话上限改"检查点式（最坏 40+2）"措辞；P2-2 用量挂激活（`serp_navigations_used` 随 sidecar／continue／每条路径回写）；P2-3 会话检索底线额度（floor 16，宿主报事实、loop 施加策略）；P2-6 票据分支补预留回滚；P3-1/P3-2/P3-3 注释与边界登记。回归：loop lib 757/0/3、host lib 286/0/5、workspace check 无告警、run-event conformance 15 passed。
- [x] S3 双平台重建（2026-09-11 完成，**随 0x S3 同批**）：0v S1–S2 代码随本批版本 bump（0.4.0 → 0.4.1）一并进载体；三件套重建 + 冒烟绿 + 符号核证（`browser_control` 21→75 / 25→59、`retrieval_enabled` 5/8、车道标注 2/2）见 [0X S3 重建记录](docs/audits/0X_S3_DUAL_PLATFORM_REBUILD_2026-09-11.md) §3。
- [ ] S4 实机复验（2026-09-11 首轮**未通过、受阻**）：与 0x S4 同场同题。**F1（框架缺陷）**：3 次 `browser_control {action: search}`（seq 24/264/384）全部 `permission_decision=deny`（无 `tool_started`，日志三次 `prompter ... channel closed`）——0v S1 只改控制器侧 `orz-loop/src/tool.rs::risk_class`（→ ReadOnly），未同步宿主侧 `orz-host/src/permission.rs::access_kind`（480 行起），`browser_control` 落 `AccessKind::Edit` 兜底 → 无头确定性拒；同车道 `browser_read` 5/5 `allow_once`（有 `Read(None)` 分支，543 行）即对照。**F2（装置）**：Chromium 引导 600s 超时（143MB/246MB）+ PATH 命中 snap 桩 → `browser_launch_result` 5/5 failure。判定：判据 1/5/6/7 未观察、2 与 4 部分成立。**修复方向（已按用户裁决处理，见下两条）**：`access_kind` 增 `browser_control` 映射（按 action 分档）+ headless fail-closed 测试；装置侧提高引导超时或预置浏览器。入口：[0X/0V S4 实机复验记录](docs/audits/0X_0V_S4_LIVE_VERIFICATION_2026-09-11.md)。
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


### P0-0y NP1 机械身体集成支线（2026-09-11 用户裁决立项；设计定稿 + 全模块化承载确认；实施未开始）

> 设计权威：[`NP1_ORZ_BODY_INTEGRATION_DESIGN_2026-09-11`](docs/NP1_ORZ_BODY_INTEGRATION_DESIGN_2026-09-11.md)（§9.3 全模块化承载确认 + 附录 D 登记前实机复核）；BACKLOG 0y；索引 `AUTH-NP1-BODY-INTEGRATION`。orz 之外扩展面：不修改 ADR-0010 与模型工具面。

- [ ] M0 定版（无依赖；本支线唯一分区写入批，前置全量分区备份）：升级官方最终版 `V3.2-260618-1045` → 全量备份 → 重新 root（Magisk）→ 重建去预装（`nothing_debloat` 模块）→ **Magisk 安全模式演练**（收尾验收，设计 §9.3）。**首次执行尝试已中止（2026-09-12，用户接管）**：全量备份（86 分区双侧哈希吻合）与 260618 官方镜像/预打补丁 boot 等产物已就位并保留；装置零改动回退；关键结论（unlock_critical 策略拒绝 / update_engine_client headers 失效 / sideload 瞬败待查）与复用路线见 [M0 中止记录](docs/audits/0Y_M0_ABORTED_FIRST_ATTEMPT_2026-09-12.md)。
- [x] **aarch64 载体重建（M1 前置）完成（2026-09-13，按 0.5.0 同源重建）**：源冻结 orz `1f13e5ec`（与 x86_64 S3 同源）→ x86 容器 + zig cc + rust-lld 交叉编译（12m11s，`BUILD_EXIT=0`）→ `orz` 73,007,720 B / `orz-signer` 1,757,984 B / `orz-acaf-provision` 1,595,128 B（AArch64 静态 ET_EXEC + `PT_INTERP=0`）；bookworm/alpine 双向加载冒烟绿 + 0z 七族符号全命中；旧 0.2.0 三件就地备份（`*-0.2.0.bak`）。复现入口 `scripts/build_orz_aarch64_musl_cross.sh`（LIFECYCLE `active`）。入口：[记录](docs/audits/0Y_AARCH64_REBUILD_2026-09-13.md)。
- [ ] M1 前置：接口定义（事件 schema / 动作契约 / 策略注册表形态）与 orz 机械层共同确定（设计 §14.1）。
- [x] §14.2 验证载体决策（**2026-09-12 用户裁决**）：引入模拟器为常设验证载体——载体集 = 模拟器（新增、常设）+ NP2（既有，流程纪律）+ NP1（端到端终验）；模拟器承担自有代码验证 + M5 补丁流程纪律干跑（坏补丁代价 = 删快照）；不验证厂商框架与 NP1 专属内核面（设计 §12 裁决段）。
- [ ] S1 验证载体搭建（待放行）：无头 x86_64 安卓镜像选型与搭建（候选 ATD 类自动化镜像）；orz x86_64 musl 三件套直接上机冒烟（静态 ELF 在安卓内核执行，与 BODY-PRE-01 同理，无需新构建）；Magisk-in-AVD 模块打包/安装/禁用/恢复实机化演练；M5 补丁流程纪律干跑首轮。**2026-09-13 盘面注记**：`D:\tb-eval\s1_emulator\` 已有 baseline AVD、Magisk 模块、M5 补丁流水线，并完成「好补丁→进系统→坏补丁（service 失败）→自愈→回滚」干跑（回滚后 `services.jar` 与 SELinux 策略哈希回到注入前）——S1 定案与入账待补，账本状态同步见 BACKLOG 0y。


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
- [ ] S4 实机复验（判据 1–13）：真机长任务复跑（含重活路径）+ 满盘注入 + abort 注入 + 编码样本；**0v-C 已另行闭合（`ba934af8`），本轮不搭车**；载体 0.5.0；S2 复审登记残段（F-BE-4/EV-7 producer 读回、F-BE-7 残段、F-C-5/6/11）随取证核验。
- [x] 裁决封闭（2026-09-12）：阈值（8 GiB / 25% / 16-8-**5**-2 GiB 阶梯）、硬上限（commit 80% 类 + CPU 80% + 并发=核数）、Job 限项（启用 commit/并发/CPU/KILL_ON_JOB_CLOSE；不启用每进程内存与 working set）、hard 档树杀（允许，四条限制）、孤儿扫除（三条件 + 三硬化）、soft 档柔性降级（默认关）、回收（轮数窗口 2 轮/上限 3、reclaim-direct 5 GiB、超预算拒绝不询问、**回收站取消**）、0z 与 0v-C 不合批。**设计无开放裁决项**；详见设计 §11 裁决记录 / §4.8。
- [x] **S1.1 裁决修订（2026-09-12，用户授权工程裁决；设计 §4.7.1 + §11 裁决 11–14）**：**两级 Job 保留**（先根后子；修订 S1 的降级结论）、**`ACTIVE_PROCESS = 2 × 核数 + 8`（下限 16；修订原"核数"臂）**、**commit 上限 = `min(min(80%×limit, limit−4 GiB), 装配期余量 − 1 GiB)`（下限 2 GiB）**、`run_tests` 入门、目标卷按静态写入目标判定、新增 `unknown` 档、here-string/heredoc 剥体留头；**盘—内存轴间耦合登记**；**回收与在跑重活的次序**登记为 S2 承重项。

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

### P1-0q 统一失败事件管线（F4 盖章治本；2026-09-07 登记；BACKLOG 0q；**S1–S4 全部闭合 2026-09-08，转 `implemented`**）

- [x] S1 设计定稿（ADR-0010 §14.63）：写入侧边界单一漏斗（host_exec 完成装配点）+ receipt 补 `action_target` 第五族 + Rust 法官唯一执法 + Python 冻结对照 + 行集纯增量零迁移；四点裁决权由用户授予主代理。入口：[设计定稿](docs/0Q_FAILURE_EVENT_PIPELINE_DESIGN_2026-09-08.md)。
- [x] S2+S3 实施（orz `4dfb3d77`）：`stamp_failure` 收口四散布写点（退役逐点对拍）+ `failure_agg_absent` 标记（与 failure_target XOR）+ console 订单漏斗 + grandfather 锚 `failure_pipeline: "funnel-v1"` + 法官新族 `failure_agg_coverage`（31 族）+ Python 镜像同步 + schema 三处 + 覆盖面矩阵/e2e/正反两测/六场景对拍全绿（orz-loop 751 / orz-assurance 210 / orz-tools 2844）。（2026-09-08 闭合）
- [x] S4 收口：BACKLOG/TODO/索引/ADR §14.64 闭合转录 + manifest 重算 1441 条 + 门禁 Exit 0；基线 0.3.2 不 bump；计数 28 → 27。（2026-09-08 闭合）
- 完整勾选与调研明细（六家对标）见全量快照 [`TODO_FULL_2026-09-09`](存档/todo/TODO_FULL_2026-09-09.md)。

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

### ORZ-CACHE-CONTEXT-COST（`implemented`；2026-08-15 三项全部闭合）

- [x] 全部闭合：`request_header_change` 请求头留痕 + 探针准确性审计（翻转↔header 交叉核对）+ 单轮工具结果注入预算（默认 50K、超限拒批 + offset 续读）与策略化读取；orz-loop 337 / Python 1888+14 skipped / 仓库门禁 valid。入口：ADR-0010 §14.9 / [探针设计](docs/TOOL_AVAILABILITY_PROBE_DESIGN_2026-08-13.md) / [审计](docs/audits/GAP_CACHE_CONTEXT_COST_IMPL_AUDIT_2026-08-15.md)。

### ORZ-ORIENTATION-FORCED-TEMPLATE（`implemented`；2026-08-15 闭合；2026-09-01 退役）

- [x] 全部闭合：ADR-0010 §4.2 正文修订 + 强制模板轮实现（无工具 checkpoint 轮、模板校验、一次重填 + 降级兜底、pending 单槽、主车道）+ 缓解必做（progress_evidence 交叉校验 + 缺失面）+ v0.2 `checkpoint_response` 事件（Schema/verifier/fixtures）+ 二次审查修复；orz-loop 333 / Python runtime 264。入口：[设计](docs/ORIENTATION_FORCED_TEMPLATE_DESIGN_2026-08-14.md) / ADR-0010 §14.13 / [审计](docs/audits/GAP_ORIENTATION_FORCED_TEMPLATE_IMPL_AUDIT_2026-08-15.md) / BACKLOG 6c。

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

### ORZ-BLACKBOARD-PLAN-EPOCH（`implemented`；S1-S7 全部闭合，2026-08-14/15；2026-09-03 退役标注：生产语义被会话作用域黑板取代，`--plan` 诊断保留）

- [x] 全部闭合：plan epoch 身份与批准事件 / 原子轮换与归档（.gsa/blackboard）/ 压缩解耦 / 跨 epoch 回查 / 测试审计 + S6/S7 复查补强（epoch 时间戳单调、归档写盘原子化、epoch_archive_write_failed 事件）。入口：[设计](docs/BLACKBOARD_PLAN_EPOCH_DESIGN_2026-08-14.md) / ADR-0010 §14.15 / [审计](docs/audits/GAP_BLACKBOARD_PLAN_EPOCH_IMPL_AUDIT_2026-08-14.md) / BACKLOG 6e。

### ORZ-LARGE-FILE-READ-CONTRACT（`implemented`；2026-08-17 设计定案，同日实施闭合）

- [x] 全部闭合：读取句柄信封（粗门默认 16KB、可配 8–32KB + 有界预览 ≤4KB + offset 续读指针）+ 模型面契约提示（grep/结构优先、大文件分段、空结果语义）+ 黑板/结果栏只放指针 + 全面检查修复（信封空窗口/越界语义、toolset 配置端到端、描述同步）；orz-tools read_file 201 / orz-loop 440 / orz-host e2e 5。入口：[设计 §11](docs/CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md) / ADR-0010 §14.22 / [审计](docs/audits/GAP_LARGE_FILE_READ_CONTRACT_IMPL_AUDIT_2026-08-17.md) / BACKLOG 6f。

### FUS-LEDGER-FOLD-STATE（`implemented`；2026-08-18 设计定案，同日实施闭合）

- [x] 全部闭合：fold 三态 + 有状态请求视图 + loop-top 推进触发（128K）+ 压缩联动/摘要同源/恢复 + 参数接线（192K/256K）+ 二次全面审查收口（冻结台账进摘要存档、v0.2 `ledger_fold_advance` 事件、完整性回退、死代码删除）；orz-loop 452 / conformance 15 + journal validation 214。入口：ADR-0010 §14.26 / BACKLOG 6g / [审计](docs/audits/GAP_LEDGER_FOLD_STATE_IMPL_AUDIT_2026-08-18.md)。

### 0aa 历史卷 journal 全量 verifier 复扫（P1；2026-09-13 立项，深审附带建议①）

- [ ] 卷清单先行（工作区 `.gsa/` + `jobs-official` 等评测批次 + 存档卷，覆盖率判据 100%）→ `journal-conformance` 全量复扫 → 断链分布审计落档（按批次/时间窗/事件族分桶，区分脱敏改写断链 vs 墙钟杀死接缝断链）→ `recover_torn_journal` 适用性只读评估。边界：只读、不改写历史卷、不重跑 run。入口：[深审 §2 P0-2](docs/audits/FULL_PROJECT_DEEP_REVIEW_2026-09-12.md) / [BACKLOG 0aa](docs/BACKLOG_AND_PRIORITIES.md)。

### 0ab 账本一致性与瘦身机械化（P1；2026-09-13 立项，深审附带建议②）

- [ ] `check_repository.py` 增计数一致性四点交叉核对（BACKLOG 总数 ↔ 各节开放项 ↔ TODO 勾选 ↔ 索引状态速查）+ 账本瘦身检查（台账行长限/行龄）；负例钉子（人为不一致可检出）；首批账本瘦身随 S1 做。入口：[深审 §3](docs/audits/FULL_PROJECT_DEEP_REVIEW_2026-09-12.md) / [BACKLOG 0ab](docs/BACKLOG_AND_PRIORITIES.md)。

### 重文件拆分勘察（2026-09-13 落档；拆分立项留用户裁决，未计数）

> 只读扫描 + 车道归属判定，全文与数据见 [`HEAVY_FILE_SPLIT_SURVEY`](docs/audits/HEAVY_FILE_SPLIT_SURVEY_2026-09-13.md)；索引 `AUTH-HEAVY-FILE-SPLIT-SURVEY`。

- [ ] （候选 1，第一优先）`orz-loop/src/host_exec.rs`（9,184 行）沿 CONTROLLER-SPLIT 先例拆分：S1 切分图 → S2 pub(crate) 机械搬移（行为不变）→ S3 回归核验；验收 = 单文件 ≤10,000 + 职责域单一。
- [ ] （候选 2）`orz-loop/src/gateway/transport.rs`（5,846 行）流式/非流式拆分，与深审 P2-7 重试链统一同批闭合。
- [ ] （候选 3）`orz-assurance/src/journal/families.rs`（6,491 行）按 35 事件族分模块，纯机械搬移；可与下一个事件族批同车。
- [x] 休眠/血统车道六超重件（handle.rs / conversation.rs / textarea.rs / manager.rs / servers.rs / queue.rs）判定**不拆**——退役/冻结裁决时一并处置（OBS-PERMISSION-DUAL-IMPL 终局治理视野）。

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

### MECHANICAL-LAYER-MATH-CALCULUS（`implemented`；阶段 0-3 全部闭合 2026-08-31，BACKLOG P2-10）

- [x] 全部闭合：阶段 0 决策（D1-D7）/ 阶段 1 设计定稿（F1-F6 + ADR-0010 §14.47 转录）/ 阶段 2 实施切片（I1 T̂+LIF 计算器、I2 失败目标身份、I3 temporal 分区、I4 域 spike 侧车、I5 类型化信封、I6 pipe 归约）/ 阶段 2 全面审查处理（R1-R9 + F10-F14 全部收口）/ 阶段 3 验证（V1 FakeProvider 面 8 项 + F11 receipt↔事件链同构核对、V2 离线 102 runs 四对照门 + 聚类对照 + 零误干预、V3 S3 重建 + S4 实机冒烟 1/1 与 temporal 四查询面端到端一致）。入口：[正式设计](docs/MECHANICAL_LAYER_MATH_CALCULUS_DESIGN_2026-08-30.md) / [讨论稿](docs/MECHANICAL_LAYER_MATH_CALCULUS_DISCUSSION_2026-08-30.md) / [ADR-0010 §14.47](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [阶段 3 验证记录](docs/audits/MECHANICAL_LAYER_MATH_CALCULUS_PHASE3_VERIFICATION_AUDIT_2026-08-31.md) / BACKLOG P2-10。

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
