# ORZ 待办项记录（TODO）

> 状态：living（实施勾选清单）；建立：2026-08-14。
> 定位：面向后续实施与改动的操作型清单，派生自 [`docs/BACKLOG_AND_PRIORITIES.md`](docs/BACKLOG_AND_PRIORITIES.md) 的未闭合项；优先级、决策门与状态权威仍是 BACKLOG，设计权威是 ADR-0010 / ADR-0011，召回路由是 [`CLI_PROJECT_INDEX.md`](CLI_PROJECT_INDEX.md)。本文件只登记指针与勾选状态，不新增设计裁决，不重复维护设计细节。
> 维护纪律：完成一项勾选一项，并同步 BACKLOG 与索引状态；新增或调整优先级只改 BACKLOG；删除或归档条目前先回查索引入口；每次改动保持本文件与 BACKLOG 的入口一致性。
> 入口：索引 AUTH-TODO（2026-08-14 登记）。
> 2026-08-31 清理轮：已闭合项压缩为单行核对条目（实施细节以 BACKLOG 变更记录与审计文档为准）；历次计数流水不再在快照重复；勾选状态以 BACKLOG 为权威，本次仅对 BACKLOG/索引已声明闭合的滞后项补勾，未闭合项原样保留。备份：`%TEMP%\TODO.md.bak-20260831`。
> 2026-09-09 整理轮：P0-GOV 00/00a、P0-0p、P1-0q 等已闭合区再次压缩为单行核对（明细见全量快照）；开放项路由同步至 BACKLOG 当前口径。全量快照：[`存档/todo/TODO_FULL_2026-09-09.md`](存档/todo/TODO_FULL_2026-09-09.md)。
> 2026-09-10 滞后入账轮：0o T0（2026-09-07）已达成的 S3 型闭合——0b 验证② / P2-14 S3 / P2-11×3 S3——按 BACKLOG 各条目小节入账后同步补勾与状态注记；勾选状态以 BACKLOG 为权威。

## 使用说明

- `[ ]` = 待办；`[x]` = 已完成（保留单行供核对，不计入开放项）。
- 每项标注 canonical ID / 优先级 / 关键内容 / 入口；同一概念只出现一次，不复制 BACKLOG 的决策记录。
- 已闭合项单行核对格式：`[x] <ID>：<一句话>（闭合日期；入口：<文档/审计>）`。

## 开放项路由（2026-09-09 同步；勾选与计数权威在 BACKLOG）

- 未闭合总数：**27 项**（BACKLOG 计数口径；逐次计数流水见 [`docs/BACKLOG_AND_PRIORITIES.md`](docs/BACKLOG_AND_PRIORITIES.md) 未闭合计数，TODO 不重复维护）。TODO `[ ]` 明细含父/子项，计数以 BACKLOG 为准。
- P0：0b 验证③④（⑤ 89 题独立用户门）；0d 后续 3/4/5 S4 复验；0j W1-R1 S4 复验 + W3-R3 余项 + W4-R4 S5-2 总项；0l ⑥⑦（⑧ TER 见 [`TODO2.md`](TODO2.md)）；0m S3/S4；0n GAP-APPROVAL-PROMPTER（延期，S1 设计定稿前置）；0o T3 批次 W2 chunk3 + T4 批次 L + T5 批次 O + T6 统一收口；0t S4 实机复验待续；0u R4 15 题官方复跑（0t S4 载体，2026-09-10 放行）；0v 检索引擎 SERP + browser_control 分类修正（S1–S2 完成，S3–S4 待续）；0w TB 4.0 单题摩擦探针（首跑未成立（环境）、执行器已修订，顺延待重跑）。
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
- [ ] 收口余项：0t S4 判据 6 宿主机日常可用性另线；计数按 0t 小节闭合时入账。

### P0-0v 检索引擎 SERP 接入与 `browser_control` 车道分类修正（2026-09-10 立项；**S1–S2 完成，S3–S4 待续**）

> 裁决：navigate 被拒应满足模型需求 + 坐实 DeepSeek 后端慢（调查已闭合）+ 按既有 SERP 设计补实现（候选 Bing）；区域固定 `en-US`；Bing 无登录态污染专项优化；低质量域名加权复用既有 `SourceWeightConfig`（标注 + 稳定排序，不硬过滤，不承担恶意域识别）。
> 设计：[`RETRIEVAL_ENGINE_SERP_SEARCH_ACTION_DESIGN_2026-09-10`](docs/RETRIEVAL_ENGINE_SERP_SEARCH_ACTION_DESIGN_2026-09-10.md)；BACKLOG 0v。

- [x] S1 代码（2026-09-10 完成）：`risk_class` browser_control ReadOnly 豁免 + `search` 动作（引擎链/失败备忘/SERP 有机结果提取/URL 解码/pacing/信封）+ 低质量域名加权接线（`SourceWeightConfig` 注入 search 编排）。
- [x] S2 测试（2026-09-10 完成）：纯函数单测（引擎表/失败备忘/可用引擎选择/pacing 上下限/解析与 CAPTCHA/URL 解码/加权稳定排序/字段上限/固定表达式 Bing 广告排除）+ 动作信封与参数校验 + 搜索上限失败态 + 车道门测试（`write_gate None`、ReadOnly、action_category=read）。S2 发现并修复首次 search 误等 5s 冷却基值的实际缺陷。
- [x] 复审第 1 轮（2026-09-10，设计 §5.3）：P1-1 Bing 跳转解码（`a1<base64url>` + URL 校验 + 原始链接回落）、P2-1 `engine_attempts` 引擎标注（成功/失败/跳过都显式）、P2-2 取消整包 8KiB 截断改字段上限（url 2048 取代整包截断）、P2-3 反污染下沉（启动层 `--window-size` + 会话层尽力而为、白名单收窄）、P2-4 车道 SERP 预算方案 A。
- [x] 复审第 2 轮（2026-09-10，设计 §5.4，P2-4 专项）：P1-1 拒绝信封字段补 schema 登记 + 2 个 fixture 进 `check_repository`；P2-1 会话上限改"检查点式（最坏 40+2）"措辞；P2-2 用量挂激活（`serp_navigations_used` 随 sidecar／continue／每条路径回写）；P2-3 会话检索底线额度（floor 16，宿主报事实、loop 施加策略）；P2-6 票据分支补预留回滚；P3-1/P3-2/P3-3 注释与边界登记。回归：loop lib 757/0/3、host lib 286/0/5、workspace check 无告警、run-event conformance 15 passed。
- [ ] S3 双平台重建。
- [ ] S4 实机（可搭 0t S4 判据 6 宿主机日常可用性同场）。

### P0-0w TB 4.0 单题摩擦探针（2026-09-10 用户指示立项；**两跑均未成立（环境）、执行器已修订 6 处，顺延待重跑**）

> 指示：找一个最适合摩擦的题做单测，关键是看 orz 的水平。
> 排期与判据（执行权威）：[`TB40_CTR_OPTIMIZATION_FRICTION_PROBE_SCHEDULE_2026-09-10`](docs/TB40_CTR_OPTIMIZATION_FRICTION_PROBE_SCHEDULE_2026-09-10.md)；执行器 [`run_tb40_ctr_probe.py`](scripts/run_tb40_ctr_probe.py)；BACKLOG 0w。

- [x] 预登记（2026-09-10）：选型 `terminal-bench/ctr-optimization`（长驻无输出进程 ≈4.8h + 本地 HTTP localhost:5000 + 大输出 + 硬时序 + 多服务/sidecar 采集，2C8G/gpus=0 本机可跑）；载体固定 orz 0.4.0（`0797610e…`，不叠 0v 未重建代码）；9 项摩擦点清单 F1–F9 + 10 条判据 + 中止/降级条件已落文档。
- [x] 前置核验（2026-09-10）：载体 SHA256 与 0.4.0 发布锁定值一致；三枚镜像（environment/sidecar/verifier）按 digest 预拉成功；仓库门禁仅剩预期 orz dirty。**数据集级解析故障未解除**（`terminal-bench/terminal-bench@4.0.0` 四次复测失败：服务端 statement timeout / HTTP/2 ConnectionState.CLOSED），改用单题解析路径 `harbor run -t terminal-bench/ctr-optimization`（实测可用；不落数据集级 digest 钉）。
- [x] 口径更正（2026-09-10）：初稿「非多容器」判断错误（实为 main + api sidecar + verifier 三镜像多服务题）；「52 道纯单容器题」计数受同一扫描缺陷影响，需重算。
- [x] 首跑（2026-09-10 19:03–19:17，用户放行）：**未成立、零有效试次**——pass 1 真实试次 9 分 27 秒（285 事件 / 32 工具调用）死于模型流中断（`transport_retry` zero_chunk ×10 耗尽 → `run_failed`，官方 `NonZeroAgentExitCodeError` / `reward 0.0`）；pass 2/3 未进入试次（harbor 鉴权换票失败，同时段 hub TLS 握手不通）。归 §5 环境顺延，不计失败批次。
- [x] 第二跑（2026-09-11 00:22 收尾）：**未成立、零试次、零模型 token**——三轮全部 `AgentSetupTimeoutError`（6m28s / 6m23s / 6m25s，整批 24m43s）：harbor 准备超时默认 360s，本步要在容器内装 Chromium，坏线路下超阈值。验收门与 `-r 0` 均被证实有效。
- [x] 执行器修订（累计 6 处 + 自测）：首跑后 4 处——每轮独立控制台日志；前置鉴权预检门（不通过即顺延、零轮次消耗，新增退出码 2）；`job_complete` 排除出错试次；harbor `-r 3` → `-r 0`。第二跑后 2 处——前置类失败签名扩围至准备阶段（`AgentSetupTimeoutError` / `EnvironmentStartTimeoutError`）；`--agent-setup-timeout-multiplier 4`（360s → 24min）。口径已同步排期文档 §2/§4/§5/§6/§7.6/§7.8。
- [ ] 重跑（待线路稳定；预检门绿了即起跑）：单题 / k=1 / 官方墙钟 8h（实际约 4.8h）；跑批期间不并行其他实机批次。
- [ ] 跑后分析：按 F1–F9 逐项填对照表 + 判据 1–10 判定；结果落档 `docs/audits/TB40_CTR_OPTIMIZATION_FRICTION_PROBE_<date>.md`。pass 1 已留下 F1/F6/F7/F9 的部分证据可并入。
- 边界：非成绩批次——TB 4.0 榜单无任何 DeepSeek 型号，无同模型参照，不产出水平结论；本题不触碰检索车道，不验证 0t/0v。


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

## P2 — 生产化决策门

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
