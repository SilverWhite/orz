# ORZ 待办项记录（TODO）

> 状态：living（实施勾选清单）；建立：2026-08-14。
> 定位：面向后续实施与改动的操作型清单，派生自 [`docs/BACKLOG_AND_PRIORITIES.md`](docs/BACKLOG_AND_PRIORITIES.md) 的未闭合项；优先级、决策门与状态权威仍是 BACKLOG，设计权威是 ADR-0010 / ADR-0011，召回路由是 [`CLI_PROJECT_INDEX.md`](CLI_PROJECT_INDEX.md)。本文件只登记指针与勾选状态，不新增设计裁决，不重复维护设计细节。
> 维护纪律：完成一项勾选一项，并同步 BACKLOG 与索引状态；新增或调整优先级只改 BACKLOG；删除或归档条目前先回查索引入口；每次改动保持本文件与 BACKLOG 的入口一致性。
> 入口：索引 AUTH-TODO（2026-08-14 登记）。
> 2026-08-31 清理轮：已闭合项压缩为单行核对条目（实施细节以 BACKLOG 变更记录与审计文档为准）；历次计数流水不再在快照重复；勾选状态以 BACKLOG 为权威，本次仅对 BACKLOG/索引已声明闭合的滞后项补勾，未闭合项原样保留。备份：`%TEMP%\TODO.md.bak-20260831`。

## 使用说明

- `[ ]` = 待办；`[x]` = 已完成（保留单行供核对，不计入开放项）。
- 每项标注 canonical ID / 优先级 / 关键内容 / 入口；同一概念只出现一次，不复制 BACKLOG 的决策记录。
- 已闭合项单行核对格式：`[x] <ID>：<一句话>（闭合日期；入口：<文档/审计>）`。

## 未闭合扫描快照（2026-08-31 清理轮）

- 未闭合总数：**28 项**（BACKLOG 计数口径，2026-08-31：阶段 3 验证闭环 38 → 32；0k S4 实机复验闭环 32 → 30；P2-11 设计轮登记不动计数；2026-09-01：P2-11 DC 强制模板轮清理闭合 30 → 29，P3「DC 硬信号 4/6」退役 29 → 28）。TODO `[ ]` 明细含父/子项，计数以 BACKLOG 为准。
- P0：FUS-BENCHMARK-FULL-EXEC 验证②③④⑤ + 闭合（见 P0-F）；0d 后续 3/4/5 的 S4 复验（各 1，S3 已随合并批次核证闭合）；0j（W1-R1 S4 复验、W3-R3 余项×3、W4-R4 S5-2 总项 + 验证期发现）；0l（WINDOWS-HIGH-NIST-MAX-FRICTION 设计定稿、实施待放行，见 P0-0l）。
- P1：FUS-COMPONENT-REGISTER 组件审计；GAP-WINDOWS-EVIDENCE 三项；IMPL-DEEPSEEK-TRANSPORT DeepSeek live 晋级证据；ORZ-SESSION-CONTEXT-MONITOR 四项。
- P2：IMPL-CONTROL-FABRIC（Slice 3 / Slice 4 / 可选）；OPS-PROTOCOL（裁剪设计 + 生产接线裁决）；MODEL-RESIDUAL-PRESSURE-FOLLOWUP 四项（P2-11）。
- P3：EVIDENCE-LOCAL-BROWSER、GATE-CHAIN、observed-scope 枚举、V11-IMPL-003、V11-IMPL-007、orz-host flaky（DC 硬信号已随 P2-11 退役）。
- 审计登记边界（条件触发，不占当前优先级）：orz-host 可选后端、headless 计划信号、23 工具分区 journals、B-1 后续、ORZ-RECOVERY-TOOL-OUTCOME、ORZ-STAGNATION-TOOL-SIGNAL。
- 已闭合分组（单行核对见下）：P0-E、0c、0d 主项与后续 1/2/6/7/8、0e、0f、0g、0h、0i、0k、P0-B、P0-C、P0-C2、P0-D、P1 已闭合项、P2-10 全部闭合。

## P0 — 当前工作集

### P0-E 评测冒烟暴露问题（2026-08-17 登记；2026-08-18 全部闭合）

- [x] 全部闭合：ACAF 容器内供应 / console 工具名下划线 / plan_write 校验消息形状 / actions 形状探针锁定 / 计划视图步骤 ID / 订单发放前拒绝入事件面 / grep 搜索范围契约（结构化信封 + 结局三型 + hidden/no_ignore + 静态 rg）/ list_dir 范围计数 / grep files_searched 全结局探针。入口：BACKLOG 0a / ADR-0010 §14.21/§14.23 / 对应实施审计（GAP_ACAF_HARNESS_PASSTHROUGH、GAP_CACHE_CONTEXT_COST 等）。

### P0-F FUS-BENCHMARK-FULL-EXEC（`pending`=实施完成待验证；P0，2026-08-18 用户裁决实施）

> 入口：[设计](docs/BENCHMARK_FULL_EXEC_DESIGN_2026-08-18.md)；ADR-0010 §14.24；BACKLOG 0b。orz 子模块 3f43478 / 4e9e61b。

- [x] 实施完成（2026-08-18）：权限层 Benchmark 两轴参数化 + 探针 BenchmarkFull + console `workspace.run_terminal` 注册 + CLI `--allow-shell`/`--allow-network` + `tb_agents/orz.py` 透传 + 审查收口；验证① 全量测试通过（含修复既有测试漂移 c4772fc）；折叠 400 根因修复 S1-S5 闭合（处理文档 LEDGER_FOLD_MARKER_INDEX_FIX_HANDLING）。
- [ ] 验证②（用户指示暂缓）：Linux musl 重建（ORZ-BUILD-MOUNT-001 契约，输出 `D:/tb-eval/orz-linux`）。
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
- [ ] W4-R4 S5-2 验证期发现（2026-08-29 登记，S5-1 遗留回归，独立排查）：orz-bin `acaf_e2e` 7 项失败（`controller_control_events_carry_tickets` / `fail_closed_continue_consumes_goal_revision_ticket` / `fail_closed_goal_revision_rejected_does_not_migrate` / `fail_closed_web_search_executes_unticketed_with_zero_ticket_events` / `goal_revision_continue_flow_re_derives_session_key` / `missing_browser_read_url_refuses_before_acaf_with_count_gate` / `signer_unreachable_shadow_records_rejection_and_proceeds`），16 通过。归因链：S5-1 提交 ad5f9ee 引入的 ACAF 控制事件/disposition/票据域遗留回归（与 S5-2 改动路径不相交），需独立轮次定位（建议先核对 disposition 处理链与 orientation 触发轮交互）后再进 S3/S4。
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
- [ ] ③ 三臂正式序列固化（control 基线快照 → non-admin → high-nist
  模板切换重跑 enforcement-probe 先验墙；2026-09-02 已单轮全 PASS）。
- [ ] ④ 任务集（首批 2 摩擦探针 + 1 真实任务 log-summary-date-ranges
  + verifier）→ control 臂基线（k=1）。
- [ ] ⑤ high-nist 小批 → 设计 §7 判据逐项核对。
- [ ] ⑥ 全量 + 记账 + 缺口判定登记 BACKLOG。
- [ ] ⑦ 收尾：AppLocker 恢复复验、DeepSeek 凭据 CredRead 验证、临时
  任务/累积 ACE/旧目录清理、Clash 网络导入。

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

### ORZ-CACHE-CONTEXT-COST（`approved`；2026-08-15 三项全部闭合）

- [x] 全部闭合：`request_header_change` 请求头留痕 + 探针准确性审计（翻转↔header 交叉核对）+ 单轮工具结果注入预算（默认 50K、超限拒批 + offset 续读）与策略化读取；orz-loop 337 / Python 1888+14 skipped / 仓库门禁 valid。入口：ADR-0010 §14.9 / [探针设计](docs/TOOL_AVAILABILITY_PROBE_DESIGN_2026-08-13.md) / [审计](docs/audits/GAP_CACHE_CONTEXT_COST_IMPL_AUDIT_2026-08-15.md)。

### ORZ-ORIENTATION-FORCED-TEMPLATE（`implemented`；2026-08-15 闭合）

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

### ORZ-BLACKBOARD-PLAN-EPOCH（`implemented`；S1-S7 全部闭合，2026-08-14/15）

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

- [x] PULL 自描述设计（2026-08-31 定稿 + S1/S2 + 审查修复完成，`partial`；
  S3 重建 + S4 实机复验待放行）：`blackboard_read` 增量头 + temporal 一次返回
  （零注入、8 工具面冻结）。入口：设计 `docs/PULL_SELF_DESCRIPTION_DESIGN_2026-08-31.md`
  / ADR §14.48 / [审查修复](docs/audits/P2-11_PULL_SELF_DESCRIPTION_S1_REVIEW_AUDIT_2026-08-31.md)。
- [x] DC 强制模板轮清理（2026-09-01 实施完成，闭合）：DC 机制全删
  （`diagnostic_coverage.rs` / 强制模板轮 / 信号消费）+ plan 反例变体注册
  （`COUNTEREXAMPLE_GATE_PLAN_BLOCK`）+ P3「DC 硬信号 4/6」退役；
  schema/verifier/fixtures/测试收口；checkpoint 共用件拆分（orientation
  软门与 console 询问轮保留）。入口：[实施审计](docs/audits/P2-11_DC_FORCED_TEMPLATE_CLEANUP_IMPL_AUDIT_2026-09-01.md)
  / ADR-0010 §14.49 / BACKLOG P2-11。
- [ ] retryable 机械分类位：`Fail` 信封增 `retryable: bool`（确定性失败 false：
  scheme/锚点/sealed/cap；暂时性 true：超时/网络），错误码事实推导、非建议；
  schema/verifier/fixtures 先行。**S1 实施 + S2 测试完成（2026-09-01）**：
  `orz-assurance/src/tool_envelope.rs` 增 `retryable_for_code` 构造期推导
  （未知码 fail-closed false），reducer/fake-provider fixture 同步；设计转录
  ADR-0010 §14.50 / 机械层设计 §2.1；orz-assurance 195 lib + 9 fake-provider
  测试全绿、orz-loop 编译通过、fmt/clippy 无新增；2026-09-01 审查处理 O1–O4
  收口（字段私有化/归约边界位归一化/优先级与边界测试，199 lib + 9 全绿）；
  S3 重建 + S4 实机复验待放行。
- [ ] 依赖图实施（下一轮主线）：文件锚点链最小范围（read→write 锚点边 + 工具→
  实体变更边；D3 命令/检索副作用不建图），PULL 查询面、模型零改动；顺带闭合
  F11 receipt↔事件链逐段同构核对。**2026-09-01 设计定稿 + S1 实施 + S2 测试
  完成（`partial`）**：`orz-loop/src/dep_graph.rs` 新模块（ReadFact/WriteFact/
  锚点边匹配/容量/revision/渲染）+ 黑板接入 + `blackboard_read section=deps`
  PULL 面 + 通用执行路径成功建图（事实随 ToolCompleted 写 `dep_graph`
  可选事件字段）+ schema/verifier/fixtures 先行 + F11 顺带闭合；设计
  `docs/DEPENDENCY_GRAPH_MAINLINE_DESIGN_2026-09-01.md` / ADR-0010 §14.51 /
  [实施审计](docs/audits/P2-11_DEPENDENCY_GRAPH_IMPL_AUDIT_2026-09-01.md)；
  orz-loop 642 lib（+7）/ orz-assurance 199+9 / Python 255 全绿；
  S1 全面审查处理收口（2026-09-01：渲染截断 footer 预算 + 单测、无效
  section 文案、verifier 措辞/死字段、设计措辞统一、边界测试补充、成本
  与序列化面登记，见
  [审查处理记录](docs/audits/P2-11_DEPENDENCY_GRAPH_S1_REVIEW_AUDIT_2026-09-01.md)）；
  S3 重建 + S4 实机复验待放行。
- [x] 工具名幻觉登记边界（不改名/不别名；fail-loud 自回正，收益上限 ≈15 轮/
  10 题）——2026-08-31 裁决。
- [x] search_replace 锚点 / submit 两阶段维持现状（0 拒单 / 8 次全通，优化收益
  不足）——2026-08-31 裁决。

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
