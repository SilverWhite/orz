# LIF/RLI 刺激面类型化总线设计(第一版)——框架内部信息标签总清单

> **版本**:v1.3;日期:2026-10-03(v1.0 同日首版;v1.1 增补 P8 触发投递成因说明;v1.2 P8 收窄为仅机械成因与机械标签、增补 P9 禁外部固定触发规则;v1.3 增补 P7/P9 边界澄清注,原则语义不变)。
> **changelog v1.3（2026-10-03 审查处置批）**:P7/P9 边界澄清注（原则语义不变）。
> **定位**:0am(LIF 动力学升级线,`AUTH-RLI-BASE-SHADOW`,挂起)刺激面改革的**设计输入档**——本档只做两件事:①把本线已定案的设计原则如实落卷;②给出框架内部**全部可用信息标签的总清单**。路由表(标签→通道)、参数与权重改动、验证批均**不在本档定案**。
> **用户裁决(2026-10-03,本档 v1.0 依据)**:「把采样的信息投影路径都分离」「非特殊情况不做加权,就加通道标签就好了」「加参数是下下策……加多了还会与物理意义解耦」「不对跑分测试本身做特化……单纯只看框架内部可用的信息标签究竟有多少」。
> **用户裁决(v1.1 增补)**:「触发后传给模型时需要带简单成因解释,目前是带了名词解释和趋势,成因现在看来也要带了,最起码给模型一些信息补充」。
> **用户裁决(v1.2 增补)**:「我同意,仅导出机械成因与机械标签信息」「不过绝对不可以外部立下并确定且固定的触发规则,避免变成隐式提醒,触发与否全靠 RLI/LIF 自身的数据情况和演进,看用的信息究竟够不够触发 spike/域切换,以及如何触发」。

---

## §1 背景与问题定案(简)

两轮真机轮(首跑 159 批 / 二跑 171 批)的三线证据共同指认**刺激面失真**,而非动力学数学缺陷:

1. **阈值推面线(0co)**:首跑侧车队列 StreakCrossed×3 全部来自单工具段内网格补点累计,对质量无判别力;
2. **语义对应线(171 批后判卷)**:二跑 2 次 streak fire 均为**单事件回声**(一次写控词元误报、一条 61s 成功测试命令的衰减尾迹跨 3 个密集采样点);13 次域 spike 与任务状态零对应;**侵蚀全程未检出**且域机器自报失明(coverage_gap「未访 pressure/stuck」×10);
3. **LIF/RLI 双面失真方向线**:两者共用同一 `classify_event_outcome` 刺激漏斗(`lif/channels.rs:174`),LIF 本体钉死 normal(u_prog≈0.975 悖论,165 批登记并复现)、RLI 影子在自己的信号缺口上抖动——同一盲输入喂出相反两病。

**根因形式化**:异质刺激混入固定参数的少数通道 = 低维投影上的混叠,成分信息在混合步即不可逆丢失(数据处理不等式);θ 分位数自校准对混合分布运行,会把混叠洗白成"统计合法"的越线。**扩参数无法恢复混合步已损失的信息**——出路在混合之前(路由),不在混合之后(拟合)。

## §2 设计原则(已定案)

| # | 原则 | 内容 |
|---|---|---|
| P1 | **分离投影路径** | 刺激面 = 带标签事件总线;每个标签路由到**语义独占**的通道;禁止多语义混入同一通道 |
| P2 | **路由扩维度,不扩参数** | 观测面扩展以"新增标签→新增/既有通道"的路由条目实现;通道动力学保持小参数 |
| P3 | **参数物理性三条件** | 参数只允许以「有单位 + 有现实指涉 + 由构造或预注册校准设定、永不对结局拟合」的方式变动(现有范例:`PROG_TAU_ROUNDS × T̂`、`SLOW_W_MAX` clamp、`θ_stuck = 1.5·T̂`) |
| P4 | **非特殊情况不加权** | 通道内幅度加权只作特例保留(既有 slow 时长加权为先例);新刺激一律定值注入;提分辨率优先靠加路由不靠加幅度 |
| P5 | **读出端禁线性混合** | 域判定维持阈值/结构化规则形态(如「prog 低 ∧ stall 高 ∧ err 零 = 卡死形态」);禁止 `Σaᵢ·uᵢ` 式读出权重 |
| P6 | **离线重放先验证** | 路由方案以重放变换形式在既有 run journal 上离线对拍(`rli_shadow_replay` 件现成),判据以**框架内可复核事实**为基准(标签间一致性、已知机制事件回放对拍),不引外部任务分数、不对跑分特化 |
| P7 | **本线不写模型面内容** | 刺激面只消费机械事件与状态(与零注入/模型无感边界一致);标签清单不含任何需要读输出内容的项（澄清 v1.3：禁读**输出内容**;P8 机械标签写入模型面属「通道看见什么→机械传话」面,不在本条禁区。） |
| P8 | **触发投递带简单成因(仅机械)** | 触发提醒行在名词解释+趋势之外，增列**成因段**——成因段**仅含机械成因与机械标签信息**:①机制成因(通道读数 vs 阈值 vs 计数窗);②机械标签(驱动标签名+计数,如「验证类动作 0 次」)。不含任何语义转述、行为评判或行动建议(机械层只记录与传话纪律不变)。机制成因现状即可机械导出;标签成因随 S2 路由表落地。本条修订 0cp D6 的「无成因」子句;行宽预算(现 ≤240B)随实现批重订 |
| P9 | **禁外部固定触发规则** | 触发与否**完全由 RLI/LIF 自身的数据情况与演进决定**——信息够不够触发 spike/域切换、如何触发,一律是动力学自身的判断;**禁止在动力学之外立下任何确定且固定的「当 X 出现即提醒」式触发规则**,避免提醒通道退化成隐式提醒面。改革只改**通道看见什么**(刺激标签),永不添加**何时叫**(外部触发条件);成因段(P8)只描述动力学已经触发的事实,不构成新触发条件。违例形态示例:「读数与模型可见信号分歧即 fire」「每 N 次验证失败提醒一次」——前者是外部规则,后者是固定规则,均禁止（澄清 v1.3：字面「永不添加何时叫」指**不新增触发规则形态**——「当 X 出现即 Y」式外部规则;通道扩充经由既有内生触发机制生效,客观上新增了既有机制的可触会面,如实登记。） |

## §3 框架内部信息标签总清单

> **清点口径**:全部来源为当日源码实读(`orz-assurance/src/journal/event.rs` EventType 枚举、`orz-loop/src/mechanical_audit.rs` KIND 常量、`orz-loop/src/blackboard.rs` 分区、会话侧车结构与 `.gsa` 状态件)+ 二跑 journal 实测发生谱(23/70 型实际发生)。**在役/退役状态以枚举注释与批次史标注;逐标签的产出条件在路由设计档(S2)使用前须逐一复核**。
> **总况**:journal 事件类型枚举 **70 变体**(在役可产 **≈62**;退役/零产 **8**,枚举保留仅供历史回放);判官规则族 **48**;mechanical_audit 注入面 kind **10**(退役 2);黑板 PULL 分区 **11**;会话侧车与系统状态面 **6 组**。journal 本身即已是「schema 化、类型化、全量落盘」的事件总线——**类型化总线不需要新建采集面,只需要读侧路由纪律**。

### §3.1 journal 事件类型(event_type,70 变体)

**生命周期(7)** ——每 run 必然成组出现,为任何通道提供 run 边界:

| 标签 | 载荷要点 | 产出条件 |
|---|---|---|
| `run_preflight` | 预检读数 | 每 run 一次 |
| `run_started` | run 身份/清单 | 每 run 一次 |
| `run_finished` | 终态/轮计数 | 正常收尾 |
| `run_failed` / `run_cancelled` / `run_invalidated` / `run_terminated` | 终态分类(资源耗尽/journal 降级有显式终态) | 条件终态 |

**宿主资源(6,0z 族)** ——低频、跨档才落:

| 标签 | 载荷要点 | 产出条件 |
|---|---|---|
| `host_resource_snapshot` | 分档档位迁移读数(normal/watch/soft/reclaim_direct/hard) | 跨档才落 |
| `host_resource_denied` | 重动作预派发拒绝(档位+动作类) | soft 档拦截 |
| `resource_exhausted` | hard 档 pre-kill/post-kill 审计行 | hard 触发 |
| `process_tree_reaped` | 孤儿收割 planned/reaped | sweep 动作 |
| `reclaim_performed` | 删除前审计(outcome∈pending_delete/permanent/rejected) | 回收动作 |
| `resource_limit_hit` | 内核 Job 顶格命中(带标签的失败) | 条件 |

**提示/模型(5)**:

| 标签 | 载荷要点 | 产出条件 |
|---|---|---|
| `prompt_submitted` | 用户提交 | 每任务轮 |
| `model_request` / `model_response_received` | 模型请求/响应事实 | 每模型轮 |
| `model_output` | 模型输出轮(round 计数面) | 每模型轮 |
| `request_header_change` | 请求面(system+tools+config)变化——前缀缓存可归因 | 面变化才落 |

**ACP(2)**:`acp_initialize` / `acp_session_created` ——会话载体事实。

**工具(7)**:

| 标签 | 载荷要点 | 产出条件 |
|---|---|---|
| `tool_proposal` | 工具提案 | 每调用 |
| `permission_requested` / `permission_decision` | 权限请求/判定(含 yolo source) | 每调用 |
| `tool_started` / `tool_completed` | 执行起止;completed 带 exit_code/wall_ms/status/error | 每调用 |
| `tool_running` | 常驻命令 300s 首报中途态(运行时长/输出活跃度) | TER 后台化时 |
| `budget_cue_injected` | F6 剩余墙钟阈值跨越(remaining/rounds_used) | **默认 off**(PUSH→PULL 例外可开) |

**保证/门(5)**:

| 标签 | 载荷要点 | 产出条件 |
|---|---|---|
| `orientation_checkpoint` | 中立问询触发/阻断/位置(含 initial_round_inquiry 触发) | 阈值/pre-handoff/首轮 |
| `counterexample_gate` | 结论前反例门 | 计划/结论写入前 |
| `tool_availability_check` | 探针翻转事件 | 翻转才落 |
| `tool_belief_stagnation` / `instruction_provenance_gate` / `gate_decision` | 门与停滞通道事实 | 条件(在役状态待 S2 复核) |

**检索子代理(7)**:——**检索启用会话条件性产出**,普通主车道会话为零:

| 标签 | 载荷要点 |
|---|---|
| `information_sufficiency_assessment` / `retrieval_parent_disposition` / `retrieval_close_record` | 充分性 close/continue 结构化判定、父处置 |
| `retrieval_result_committed` / `retrieval_progress` / `retrieval_result_segment` / `result_delivered` | 结果提交/通道判活/段到达/框架→模型投递审计(来源/分级/抑制/去重) |
| `browser_launch_result` | 浏览器启动/探活事实(success/failure+真实原因) |
| `retrieval_activation_restored` | 跨 run 激活恢复 |
| `retrieval_mode_transition` | **产侧退役**(0t,枚举保留回放) |

**机械审查层(1)**:`mechanical_audit_update` ——见 §3.2 kind 细分。

**ACAF(3)**:`control_ticket_issued` / `control_ticket_consumed` / `control_ticket_rejected` ——控制票据生命周期(签发/消费/拒绝;每破坏性动作成对)。

**上下文(4)**:

| 标签 | 载荷要点 | 产出条件 |
|---|---|---|
| `context_compressed` | 压缩 marker(mode/reason/原文定位指针) | 压缩发生 |
| `ledger_fold_advance` / `ledger_fold_write_failed` | 台账折叠推进(折叠轮数/视图估计=缓存归因面)/外联写失败(降级审计) | 折叠点推进才落 |
| `context_recovery_truncated` | 恢复预检截断 | **生产零写入**(0ah 退役) |
| `session_archive` | 会话归档单包(archive_id/digest/fatigue_pct) | 会话收尾 |

**快照/工件(3)**:`snapshot_created` / `snapshot_restored` (IP5 变更前快照/恢复) / `artifact_registered`。

**计划面(8)**:

| 标签 | 载荷要点 | 产出条件 |
|---|---|---|
| `plan_write` | 首轮计划门结果(计划身份/目标/步数/机械校验/回填) | 主车道首轮 |
| `plan_proposed` / `plan_approved` / `plan_rejected` / `action_approved` | 计划模式 slices | plan 模式条件性 |
| `console_mode_transition` / `console_order_rejected` | 双模式转换判定/订单预签发拒绝(code 结构化) | 条件 |
| `console_order_written` | **退役**(2026-09-06,S2d) | 仅历史回放 |

**台账折叠(2)**:见上 `ledger_fold_*`。

**传输(1)**:`transport_retry` ——零 chunk/中段重试计数(outcome recovered/exhausted + kind + reason),传输健康面。

**写入管控(1)**:`write_control_review` ——block/warn/allow 三分类逐命令(allow 也落账;rule/detail/command_sha256/command_len)。

**模型面前缀指纹(1)**:`face_fingerprint` ——逐轮投影视图逐消息 sha256(face_sha256/stable_prefix_messages/first_divergent),前缀缓存 miss 三源定位面。

**遗留回放(4,产侧已退役)**:`neutral_inquiry` / `retrieval_completion_check` / `diagnostic_coverage_checkpoint` / `checkpoint_response` ——v0.1 冻结面回放专用,v0.2 产者断言不写。

**补充字段级标签(非独立事件)**:`tool_completed` 上的 `dep_graph` 可选事件字段(read→write 锚点边;依赖图面);`tool_completed` 的失败信封族(`failure_target`/`failure_agg` 记账,0q 漏斗)。

### §3.2 mechanical_audit 注入面 kind(模型面已在场的 10 kind)

`tool_result`(工具结果摘要行)/ `lif_domain`(LIF 域状态逐批行)/ `rli_notice`(key 形 `rli.notice.{streak_crossed|domain_spike_entry|migration_confirmed|coverage_gap}`,0cp 直投)/ `context_scale`(水位行)/ `model_compression`(压缩 marker)/ `plan_write_guidance`(首轮三问)/ `plan_gate` / `retrieval_batch`(检索会话) / `budget`(**退役**,枚举保留历史校验) / `attention_ladder`(**退役**,仅历史回放)。

### §3.3 黑板 PULL 分区(11)

`plan` / `notes`(模型可写两域)/ `exec`(订单台账)/ `edits`(编辑台账)/ `actions`(动作结果板,receipt_id 点读)/ `session`(预算+状态行)/ `temporal`(now/recent/feature 三意图)/ `rli`(RLI 读数面)/ `guide`(框架说明书)/ `deps`(依赖图)/ `journal`(anchor 定位符回查)。黑板含逐分区**版本计数**(增量头未读徽章面)。

### §3.4 会话侧车与系统状态面(持续态,非事件)

1. **LIF 本体侧车**(`lif`):round / has_success / current_domain / entry_round / spikes(域迁移 spike 序列);
2. **RLI 影子侧车**(`lif.rli_shadow`,14 键):t_hat / steps / channels×5(u/v 状态)/ domain(现域+rows)/ complexity / adapt_trace(λ̂/c/ρ/θ 序列)/ notices 事件历史 / spike_entries+returns / notice_delivered_total / sample_points;
3. **会话卷**(`.gsa`):水位(fatigue_pct,W=10MiB 软上限)、archives 单包、terminal 日志窗口、snapshots(IP5 对象/清单)、rollback、orientation 状态、activations、ledger、compaction blocks;
4. **resources_state.json**:工具参数与状态(grok_build.* 形态);
5. **process_trees/**:进程树 sweep 台账;
6. **时间原语**:全事件统一 UTC 时间戳 + run 相对轴;T̂(自适应轮周期估计);决策轮间隔/工具间隔序列(estimator 内在量)。

### §3.5 判官规则族(48,框架自认重要语义的清单)

`ALL_FAMILIES` 逐条 = inquiry_kind / initial_round_inquiry / plan_write / console_mode_transition / console_order_rejected / ledger_fold_advance / ledger_fold_write_failed / lifecycle / tool_running / output_truncation / budget_cue_injected / retrieval_mode / retrieval_enable_gate / browser_launch_result / result_consistency / reason_codes / source_weighting / search_candidate_pool / candidate_prefilter / candidate_count / inject_budget / policy_denial / failure_target / failure_agg_coverage / receipt_event_isomorphism / dep_graph_events / mechanical_audit / recovery_truncation / context_compressed / activation_restore / control_tickets / tool_availability_probe / request_header / probe_accuracy / host_resource_snapshot / host_resource_denied / resource_exhausted / run_terminated / process_tree_reaped / reclaim_performed / resource_limit_hit / retrieval_dedupe / result_delivered_accounting / retrieval_family_probe / failure_cause_shape / first_result_deadline / write_control_review / face_fingerprint。

### §3.6 语义原语聚类(检索辅助,**不构成路由定案**)

| 语义原语 | 覆盖标签(例) |
|---|---|
| 意图/计划 | prompt_submitted、plan_write、plan_proposed/approved/rejected、console_mode_transition、model_output |
| 动作与结果 | tool_proposal/started/completed/running、output_truncation、transport_retry |
| 变更与验证 | dep_graph 字段、(search_replace/write 类 completed)、(test/build 类 completed)——**当前无显式标签,分类属路由层工作** |
| 纪律/安全 | permission_*、control_ticket_*、write_control_review、policy_denial、failure_target/agg、resource_limit_hit、host_resource_denied |
| 上下文/认知负载 | context_compressed、ledger_fold_advance、request_header_change、face_fingerprint、model_request/response、context_scale/model_compression(audit kind) |
| 方向/自省 | orientation_checkpoint、counterexample_gate、tool_availability_check、gate_decision、section=rli/guide 消费 |
| 检索 | retrieval_* 全族、browser_launch_result、result_delivered |
| 资源/宿主 | host_resource_* 六族、snapshot_*、session_archive、会话卷水位 |
| 节律/时间 | T̂、决策轮间隔、工具间隔、(run 相对轴全事件时间戳) |

## §4 边界与未定项(后续档序建议)

1. **路由表(S2 设计档)**:标签→通道的认领矩阵;产出前须逐标签复核产出条件与在役状态;须包含「无标签归属即不采集」的封闭性原则与死窗条件标签的处理规则;**标签成因段(P8 第二档)的可用标签集随本档定**;**S2 只定义通道看见什么,不得定义任何触发条件(P9)**;
2. **权重**:非特殊情况不加权(P4 已定案);slow 时长加权为保留先例,任何新加权须单独立项带物理界;
3. **验证批(S3)**:路由方案以重放变换在既有 run journal(两轮在手)上离线对拍,判据框架内预注册(P6);验证判据同样只看「通道数据质量」(可辨识性/信噪),不引入外部触发规则;
4. **P8 实现批序**:机制成因段(机械成因)不依赖路由表,可先行;标签成因段随 S2;两档均须重订提醒行宽预算(现 ≤240B 由 0cp D6/S4 设立,加成因段后必然越限),行宽新值随实现批裁决;**0cp D6「无成因」子句由本档 P8 正式取代**(0cp 设计稿相应勘误随实现批);
5. **登记**:本档为 0am 转正裁决输入,索引/BACKLOG 登记随下一批落账。

## §5 关键词

刺激面类型化总线、分离投影路径、路由扩维度不扩参数、参数物理性三条件、非特例不加权、
读出端禁线性混合、离线重放验证、信息标签总清单、journal 70 变体、判官族 48、
mechanical_audit 10 kind、黑板 11 分区、语义原语聚类、不对跑分特化、触发投递成因段、
机械成因/机械标签两档、取代 0cp D6 无成因子句、禁外部固定触发规则、触发内生性、
隐式提醒禁令、0am 输入档。
