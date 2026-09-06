# 任务 D S2a 盘点与证据化（2026-09-06）

> **审计日期**：2026-09-06
> **性质**：任务 D（双实现终局治理）S2a 先行批产出——把
> `assurance/run_event_journal_validation.py` 的 31 个 `_verify_v02_*`
> 规则族逐族盘点为两档：**A** = Rust 运行时已机械强制（附测试证据入口）；
> **B** = 需 Rust conformance 显式实现（`journal/conformance.rs` 或后续
> 扩展）。本步不写业务代码（S2a 验收约束）。
> **方法**：逐函数读取 Python 法官源码（行号区间起点 = def 行，终点为函数
> 体末行或下一族 def 前一行，以实际边界为准；S2a 复审处理批 2026-09-06
> 统一口径），在 orz Rust 工作区检索生产强制点与测试证据；档位口径见 §1。
> **关联前序**：[任务 D batch-1](P0_GOV_TASK_D_DUAL_IMPL_GOVERNANCE_2026-09-04.md)
> / [GLM 处置 + S2 排期](P0_GOV_GLM_DISPOSITION_AND_TASK_D_S2_SCHEDULE_2026-09-06.md)。

---

## 1. 档位口径与总体格局

- **A 档**：规则语义由 Rust 生产代码（orz 各 crate）在事件构造点机械强制，
  且至少一个测试证据入口可引用。个别子规则仅由构造保证、无独立断言的，
  仍判 A 并在缺口列注明（conformance 实现时补）。
- **B 档**：规则（或其关键子规则）仅存在于 Python 离线法官——Rust 未强制、
  仅部分强制、或生产面已退役导致规则不可满足——需要 Rust conformance
  显式实现。
- **基线事实**：batch-1 的 `orz-assurance/src/journal/conformance.rs` 只做
  envelope/payload schema、registry、链校验（schema 级）；31 族规则级校验
  在 Rust 侧均无对等物——A 档的含义是「生产侧已强制、conformance 只需
  复核」，B 档是「conformance 需要新建裁决逻辑」。
- **总体结果**：31 族 = **27 A + 4 B**。B 族：
  `receipt_event_isomorphism`、`probe_accuracy`（S2c-1 检索族）、
  `console_order_written`、`console_order_rejected`（S2c-3 控制面族）。

## 2. S2b 核心六族（7 个函数入口 = 7 行，全 A）

| 族 | Py 行号 | 规则语义要点 | 档位 | Rust 强制点 / 测试证据 |
|---|---|---|---|---|
| control_tickets | 2074–2136 | ①终态事件引用更早 issued 同名 ticket_id；②同一 ticket_id 的 kind 三事件一致；③一票至多一个终态（one-shot） | A | 强制：orz-loop `acaf_flow.rs:52-140`（单点流程）；orz-assurance `acaf/mod.rs:697-711`（nonce ledger）。测试：`ledger_rejects_replay_and_regresses`（acaf/mod.rs:1276）；acaf_e2e.rs:318/761/1302/2698。缺口：journal 侧 issued-before-terminal 复核不存在（流程构造保证） |
| lifecycle | 2977–3315 | ①disposition 引用更早 assessment 且 CAS revision 一致；②accepted 作用于当前 revision（陈旧→rejected_stale）；③每 assessment 一个 accepted；④revision 单调 +1；⑤normal_close 绑定 disposition/contract/digest；⑥close 后冻结；⑦重放 payload 逐字节一致 | A | 强制：orz-loop `retrieval/disposition.rs:232-285`（重放幂等）、`:328-355`（stale/conflict）、`:532`（+1）、`:676-754`（close）；activation.rs:373（restore 声明先于一切）。测试：acaf_e2e.rs:318/2181/2698；`restored_activation_journaled_and_disposable_across_runs`（activation.rs:637）。缺口：stale/conflict/replay-conflict 三分支无直接命名测试 |
| retrieval_mode | 1129–1242 | ①bootstrap transition 每 journal 至多一次；②old_mode 链式一致；③off 后无 dispatch/assessment/committed result；④local_browser transition 必带 capability_status，不可用必须显式失败 | A | 强制：orz-loop `controller.rs:3214-3252`（transition once + old_mode 持久化）；`host_exec.rs:508-568/633-680`（off 门）；`retrieval/dispatch.rs:110-147`（能力不可用显式失败）。测试：mode.rs:263/374/148/186/415/460；dispatch.rs:1692/1751。缺口：off 后无 committed result 为构造推论 |
| ledger_fold_advance | 915–1030 | ①fold 点合法（start<cut、rounds≥1、estimate≥0、role 枚举）；②estimate_after 严格 < estimate_tokens；③窗口内 start 恒定、cut 严格递增、rounds 不减；④压缩重置窗口 | A | 强制：orz-loop `action_ledger.rs:881-948`（anti-spin/递增/不可变）；agent_loop.rs:1091/1180-1195（单点发射）；compact 重置 agent_loop.rs:786。测试：`fold_state_advances_once_and_prefix_stays_stable`（compact.rs:1473，全形状断言）、compact.rs:1842；action_ledger.rs:1861/1904。缺口：②③为构造保证+测试断言，无生产 assert；agent_role 恒 main（比 Py 三车道窄） |
| ledger_fold_write_failed | 1033–1126 | ①完整审计形状；②attempt 逐次 +1、成功重置；③disabled==(attempt≥3)，耗尽后不再有 write failure | A | 强制：orz-loop `agent_loop.rs:1121-1160`（attempt/disabled=≥3，常量 :512）；`:1162`（成功重置）；`:1091`（耗尽后跳过折叠）。测试：`fold_append_failure_disables_fold_and_keeps_session_going`（compact.rs:1691）。缺口：attempt 递增/重置分支无独立命名用例 |
| policy_denial | 2437–2520 | ①source∈4 枚举、code 非空、reason 字符串；②携带 denial 的完成必是拒绝完成（error/exit≠0）；③source→工具族映射一致 | A | 强制：orz-loop `host.rs:73-76`（枚举）；逐源发射点 host_exec.rs:527/587/644/798、controller.rs:3697-3763、console_exec.rs:2554；透传 host_exec.rs:3062-3076。测试：denial.rs:59；mode.rs:148/415/460；acaf_e2e.rs:2305/2408/2483。缺口：②无统一校验器（逐点字面量）；taint 源无独立测试 |
| failure_target | 2579–2694 | ①只挂失败 completion；②kind∈4 枚举；③id=64 位小写 sha256；④kind 专属字段形状；⑤kind→工具族映射 | A | 强制：orz-loop `failure_target.rs:66-150`（单点生产者+映射）；host_exec.rs:465/3181/3434（仅错误路径挂载）。测试：failure_target.rs:174-245（7 项）。缺口：无 journal 侧 id hex/形状复核；anchor_hash 不验 hex |

## 3. S2c-1 检索族 8 项（6 A + 2 B）

| 族 | Py 行号 | 规则语义要点 | 档位 | Rust 强制点 / 测试证据 |
|---|---|---|---|---|
| result_consistency | 1257–1423 | ①source_counts=ledger 机械分布；②highest_allowed_claim=可见性投影；③organized_response 已退役；④degraded⇔无文本级证据；⑤digest 三件套规范 SHA-256；⑥后续 assessment 绑定同 digest/counts | A | 强制：orz-loop `retrieval/evidence.rs:552-571/355-360/496-509/573-590`（单写入方）；dispatch.rs:727-805（assessment 绑定）。测试：evidence.rs:1371/1451/1516。缺口：assessment ledger_digest/counts 相等无专项断言 |
| reason_codes | 1437–1463 | ①reason_codes 词表封闭（2 码）；②退役码永不得再现 | A | 强制：orz-loop `retrieval/dispatch.rs:781-784`（硬编码封闭）。测试：evidence.rs:1516（正向）/1451（退役码 absent） |
| source_weighting | 1466–1517 | ①tier+weight+weight_reason 成套；②固定乘数表（1.1/1.0/0.7）；③退役模型标注字段不得出现 | A | 强制：orz-assurance `source_weighting.rs:34-52/188`（固定表+单一分类器）；evidence.rs:390-398/487-492。测试：`web_page_evidence_carries_mechanical_tier_and_weight`（evidence.rs:1567）；tier_of 系列 source_weighting.rs:309-453 |
| search_candidate_pool | 1520–1689 | ①candidate_urls 仅 web_search_result；②非空唯一串列表；③空池须有移除留痕；④pool 与 urls 同长同序；⑤池条目元数据+固定 weight；⑥raw_source_refs 双向镜像 | A | 强制：orz-loop `retrieval/evidence.rs:91-94/405-441/542-547`；orz-assurance `candidate_prefilter.rs:79-90`。测试：evidence.rs:1100/1222/1317 |
| candidate_prefilter | 1702–1800 | ①有池必有 log；②log 引用带池条目且 (source,url,reason) 唯一；③reason 封闭 5 码 + action=removed；④非 duplicate 移除者不得留在保留池 | A | 强制：orz-assurance `candidate_prefilter.rs:62-79`（封闭枚举）+ prefilter() 主流程；evidence.rs:346-351/428-440。测试：candidate_prefilter.rs 11 项；evidence.rs:1222/1317 |
| candidate_count | 1818–1902 | ①count/cap 仅候选计数工具且成对；②0≤count≤cap、cap≥1；③lane 内完成必带、包装完成不带；④超限拒绝 error+count==cap | A | 强制：orz-loop `relay.rs:49`（家族边界）；`host_exec.rs:3342/3374-3404/3423-3447`（gate）；dispatch.rs:2174-2180（cap≥1）。测试：relay.rs:142；dispatch.rs:1969/2315/2086/2512 |
| receipt_event_isomorphism | 2697–2803 | ①gate 拒绝完成 start 先于完成；②非 gate 错误完成 1:1 映射 tool_started；③同 call 至多一条 completed；④run 级配对（非 run_invalidated 终止的 run 无开放 start） | **B** | 未发现任何 Rust 跨事件 start/completed 对账校验；生产仅构造性满足①②（host_exec.rs:933/1173）；③无强制无测试；④在串行子代理路径有 `in_flight_tools` 孤儿补事件构造性机制（agent_loop.rs:430/2456-2466，无验证器无测试，B 定档不变；S2c 实现时勿重复实现）。conformance 需显式实现对账器 |
| probe_accuracy | 2925–2974 | ①兼容边界：有 request_header_change 才启用；②complete 集翻转后必须随主车道 header change（下一 model_output 前） | **B** | header 留痕有（agent_loop.rs:1395-1427/216-234），但 flip⇒header 变化不变量不成立——R1 封存工具（如 run_tests）探针翻转不进声明面，可产出 Python 法官违规 journal（tool_probe.rs:983 记录翻转而工具面不变）。conformance 需显式实现（或先裁决封存工具翻转语义） |

## 4. S2c-2 上下文与压缩族 7 项（全 A）

| 族 | Py 行号 | 规则语义要点 | 档位 | Rust 强制点 / 测试证据 |
|---|---|---|---|---|
| recovery_truncation | 1956–1985 | ①先于首个 model_request；②rounds_dropped>0 | A | 强制：orz-loop `controller.rs:3354-3411`（结构性顺序）；`compact.rs:409-413`（0→noop 不产事件）。测试：controller.rs:4983/5020；compact.rs:2505 |
| context_compressed | 1986–2053 | ①mode∈2 枚举；②reason∈3 枚举；③guard_failed 不随 session_end；④mechanical 永不 incomplete；⑤incomplete⇔三 null；⑥archive_write_failed 只随 complete | A | 强制：orz-loop `agent_loop.rs:544-830`（单点生产）+ 调用点 996/1047、controller.rs:3494。测试：compact.rs:648/1385/2102/2198/2372。缺口：incomplete 分支生产者已死代码化、无测试（Python 保留验历史 journal） |
| activation_restore | 2054–2073 | 同一 activation_id 至多恢复一次 | A | 强制：orz-loop `retrieval/activation.rs:374-421`（mem::take drain-once）+180-186（seed 去重）。测试：activation.rs:637 |
| dep_graph_events | 2806–2924 | ①shape；②read⇒read_file 且 exit 0；③write⇒search_replace 且 exit 0、consumed_read 引用同 run 更早同路径匹配 read；④命令/检索不带 dep_graph；⑤事实可选 | A | 强制：orz-loop `host_exec.rs:100-131/2979-2993`；`dep_graph.rs:146-171`（sha256 权威匹配）。测试：dep_graph.rs:332/364/464；host_exec.rs:7110/7250。缺口：path 空串依赖 schema 兜底 |
| tool_running | 3318–3477 | mid-run：①started 后 completed 前；②每 call 至多一条；③completed 带 running:true 且 exit null；④恰一条 completed。idle-kill：⑤前置 mid-run+running completion；⑥晚于 completion；⑦每 call 至多一条；⑧reason 非空 | A | 强制：orz-loop `host_exec.rs:3030-3055/136-180`；orz-host `lib.rs:714-721/96-97`（exit null、去重）；controller.rs:3522-3524（收尾 drain）。测试：host_exec.rs:4412/4560；orz-host lib.rs:2579。缺口：started<running 顺序无显式断言（结构性事实） |
| output_truncation | 3478–3512 | ①truncated⇒带 total_bytes；②object_id⇒truncated+total_bytes 同在 | A | 强制：orz-host `lib.rs:722-731`（三字段单点绑定）；orz-loop `host_exec.rs:2999-3006`。测试：host_exec.rs:3906；orz-host lib.rs:2651 |
| budget_cue_injected | 3513–3590 | ①每 run ≤4 条；②仅 remaining 严格低于档位阈值（600/300/120）才发 | A | 强制：orz-loop `prompt.rs:275-284`（严格比较+每档一次）+ `controller.rs:1993-2030`。测试：prompt.rs:548-559；host_exec.rs:6843/6910。注：Rust 上限 ≤3/run 严于 Python ≤4 |

## 5. S2c-3 控制面族 9 项（7 A + 2 B）

| 族 | Py 行号 | 规则语义要点 | 档位 | Rust 强制点 / 测试证据 |
|---|---|---|---|---|
| inquiry_kind | 455–472 | orientation_checkpoint 的 payload.inquiry_kind == event_type | A | 强制：orz-loop `orientation.rs:180`（const）+ `controller.rs:3657`（单发点）。测试：orientation.rs:235/251；orz-bin main.rs:2298/2422 |
| plan_write | 488–568 | ①refill 仅 attempt=1 且后随第二次写且 invalid；②validation_failed_after_refill 仅 attempt=2；③机械降级 valid=true、validation 族 valid=false；④accepted⇒valid 且 attempt∈{1,2} | A | 强制：orz-loop `planning.rs:462-477`（decide_outcome 状态机）+ `:484-498`（valid=errors.empty）；agent_loop.rs:1853/2651、host_exec.rs:2366。测试：planning.rs:1026/1037/1467/1515/1635。缺口：rotate_failed payload 无专测 |
| console_mode_transition | 571–737 | ①c→d 固定字段；②d→c related 指向同 run 在先 c→d；③stay 固定；④每 run 一次 streak 询问；⑤transition_id 同 run 唯一；⑥direct 工具事件携带当前 transition_id | A | 强制：orz-loop `console_mode.rs:45-90/176-181/288-295`；`console_exec.rs:30-74/1123/1158-1195`；controller.rs:1633-1653 + host_exec.rs:316-319（盖章）。测试：console_mode.rs:310-378。缺口：事件级 payload/盖章关联无 journal 断言 |
| console_order_written | 740–818 | ①机械盖章齐备；②order_id 同 run 唯一；③同 run 在先同 call_id 的 blackboard_action_write 成功完成背书 | **B** | 写单路径已被 direct 执行面退役（host_exec.rs:321-368 窄门拒 `blackboard_action_write`，handler 休眠不可达）；活动面仅「零事件」（负向测试 console_exec.rs:2700/2970/3143）。规则语义现仅 Python 法官持有；conformance 需按退役语义显式实现 |
| console_order_rejected | 821–912 | ①phase/step/code 三元组封闭（pre_issue 3 码 / issue 4 步，policy 归一）；②同 run 在先同章 written；③每 order 至多一条 | **B** | 三元组由 `console_exec.rs:406-421` 统一构造器+五发射点（console_exec.rs:484/528/593/629/758）机械固定，测试充分（console_exec.rs:1478/1655/2543 等）；但子规则②的前置 written 在写单面退役后不可满足——活动面对残留/恢复订单仍可发 rejected，与 Python 规则存在真实分歧，需 conformance 裁决退役语义 |
| mechanical_audit | 1905–1953 | ①kind∈3 枚举；②payload 非空 key/非负 round/非空 summary/anomaly 可空；③key 形态前缀封闭 | A | 强制：orz-loop `mechanical_audit.rs:56-83/95-125/199-209/217+`；agent_loop.rs:2597/2800（plan_gate/budget 发射点）。测试：mechanical_audit.rs:402-603；console_exec.rs:2838。缺口：plan_gate/budget kind 的事件 payload 无专测 |
| tool_availability_probe | 2182–2221 | ①complete/incomplete 恰好划分全部工作工具各一次；②incomplete reason 中性固定、禁判断词 | A | 强制：orz-loop `tool_probe.rs:39/265-392/401-410`（常量+封闭归类）+ controller.rs:972-990/1018-1027。测试：tool_probe.rs:484/507/983/1106/1170/1224 |
| request_header | 2224–2319 | ①按 role 链：链首 initial 无 previous；②change 紧随 last、previous==last、kind=组件 digest 差异；③tools 唯一且 count 一致 | A | 强制：orz-loop `agent_loop.rs:219-234/240-264/1395-1432`（发射守卫，lane 局部链）。测试：agent_loop.rs:3127/3172/3193。缺口：tools 唯一性无显式断言 |
| inject_budget | 2346–2400 | ①超限错误完成必带 used+budget 两字段；②两字段成对且仅该 code 合法 | A | 强制：orz-loop `agent_loop.rs:3028-3093`（唯一发射点 :3053-3054）。测试：host_exec.rs:3687/3734；agent_loop.rs:3898。缺口：budget≥1 未机械断言（配置为 0 时与 Python 规则冲突） |

## 6. 对 S2b/S2c 后续批次的输入

> **S2b 已闭合（2026-09-06）**：核心六族（7 函数入口）Rust conformance
> 实现完成——orz `809cdb4e` + 复审处理批（P1×2 修复：permission 精确集合
> / V01 gating；对拍 0 差 546 裁决格），入口：
> [S2b 实施审计](TASK_D_S2B_FAMILIES_IMPL_AUDIT_2026-09-06.md) /
> [S2b 复审处理](TASK_D_S2B_REVIEW_HANDLING_2026-09-06.md)。

1. **B 族清单（4 个，S2c 显式实现对象）**：`receipt_event_isomorphism`、
   `probe_accuracy`（S2c-1 检索族）；`console_order_written`、
   `console_order_rejected`（S2c-3 控制面族）。后两族需先裁决「写单面退役
   后的规则语义」（Python 规则按活跃双写路径写就，与 direct 执行面退役后的
   事件面存在真实分歧）——建议 conformance 实现前由用户/设计裁决目标语义。
2. **A 档缺口子规则**（conformance 实现时顺带补测试）：lifecycle 三分支
   直接测试、policy_denial 统一形状校验、taint 源测试、failure_target id hex
   复核、context_compressed incomplete 退役分支、tool_running 顺序断言、
   plan_write rotate_failed、console_mode_transition 事件级断言、
   mechanical_audit plan_gate/budget 断言、request_header tools 唯一性、
   inject_budget budget≥1。
3. **Rust conformance 定位**：A 族实现为「独立 journal 复核器」（复算
   生产构造保证的跨事件/时序不变量），而非新语义；优先补生产代码无法
   自证的断言（control_tickets issued-before-terminal、lifecycle revision
   单调与 close 冻结、ledger 窗口复算、形状复验）。

## 7. 验收核对（S2a）

- 31 行全档位落盘：§2（6 族 / 7 函数入口）+ §3（8）+ §4（7）+ §5（9）= 31 ✔
- 每行至少一个入口：全部行含 Python 行号 + Rust 强制点/测试证据入口 ✔
- 档位口径一致：§1 定义，逐行适用 ✔
- 本步不写业务代码：本批无 Rust/Python 源码变更（仅文档登记）✔
- 盘点方法限制：规则语义与证据定位由四组并行只读盘点产生，行号/测试名为
  检索时刻快照；S2b/S2c 实现时应以现场代码为准回验关键断言。
