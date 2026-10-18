# 长程进程显式通道设计稿（0dh S1）——TER 封闭面受控重开＋收割语义修订＋30h 兜底

> 版本：v1.0（2026-10-18，297 批）。
> 上游：294 批立项（S0 机制盘点＋业界调研）→ 295 批 S0-2 选型对证（Grok↔Claude Code 同构）→
> **296 批用户定案＝启用 Grok（`hide_background_input=false` 逃生阀）＋借鉴 Claude Code 三处**。
> 本稿为 S1 设计稿：把选型定案落成可实施细则，并勘定五项待勘（收割豁除×0bq 并存／per-call
> Job 归属／`is_background` 重开×工具面对账／30h 落点／ADR 承载节）。ADR-0010 **§14.87（v1.90）**
> 随批转录。
> 用户既定裁决（294 批，本稿不再重开）：**跨 run 存活、不做收割**；**绝对兜底 10h→30h**；
> idle-kill 300s「无输出增长且 CPU 时间不增」CPU 感知判据原样保留；**教学面极常用工具不得
> 长备注常驻注入**。

---

## §1 目标与非目标

**目标**：orz 会话内的模型能够声明并管理**长程进程**（过夜训练、长跑服务）——声明即立即
后台化、跨 run 存活至完成、增量读出、可杀、完成有提醒；两层收割语义按用户裁决修订；
全部机制复用现役管线，不另造表层。

**非目标**（沿 294/295 批，不变）：
- 不做新工具、不做进程类型集、不做即时打断式通道；
- 不承诺跨 orz 进程重启存活（生命周期边界见 D2⑥——宿主进程死亡即会话域终止，残留树由
  下会话装配 sweep 与 Job 句柄闭合兜底；S4 判据「跨 run 过夜训练存活至完成」亦在会话内）；
- 不改 8 工具面冻结纪律本身（本线是继 blackboard_write 之后的**第二个用户主导显式例外**，
  296 批裁决授权，随 §14.87 登记）。

## §2 现役机制勘定（S1 读码结论）

读码范围：`orz/crates/codegen/orz-tools/src/implementations/grok_build/bash/mod.rs`（6,241 行）、
`orz/crates/codegen/orz-tools/src/computer/local/terminal.rs`（6,467 行）、
`orz/crates/orz-host/src/lib.rs`（DispatchGuard／spawn sink／finalize sweep）、
`orz/crates/orz-host/src/process_tree.rs`（登记与 sweep 判定矩阵）、
`orz/crates/codegen/xai-tty-utils/src/lib.rs`（ProcessGroup／Job 挂接）、
`orz/crates/codegen/orz-tools/src/implementations/task_output/tool.rs`（读出截断）。

### §2.1 三种后台化路径同归 actor 任务域

`LocalTerminalActor` 三种后台化殊途同归：
①**显式** `is_background=true`（`handle_run_background`，`BackgroundReason::Explicit`）；
②**auto-bg**（前台 180s 预算到，`transition_to_background`，`ForegroundTimeout`，TER T1.2）；
③**用户信号**（Ctrl+G，`UserSignal`）。三者均把 `ProcessState`（含 per-call `ProcessGroup`）
保留在 actor 进程表内——**per-call Job（`KILL_ON_JOB_CLOSE`）原句柄由 actor 持有直到任务终**，
派发结束只关闭宿主侧复制的句柄 dup（`DispatchGuard::drop`），不杀活任务。

**勘定结论一（Job 归属）**：294 批待勘②「长程宿主不得落前台调用域 job」在**后台路径已结构性
成立**——后台任务的 shell 落在 actor 持有的独立 per-call Job，树拆点＝kill_task／任务完成／
绝对兜底／会话关闭四处，均符合预期。ep1 事故（前台调用内 `Start-Process` 脱离挂后台）属
**前台域 by-design 拆树**：前台 shell 被收殓时其 per-call Job 句柄闭合，脱离出去仍留在 job 内的
孙进程整树回收。正解是**教学**（长程必须走显式声明通道），不是重挂 Job。

### §2.2 跨 run 死因唯一＝`run_shutdown` finalize sweep

`OrzHost::finalize_process_trees`（lib.rs:580）在 run 尾以
`SweepMode::Finalize{own_run_id, own_pid}` 调 `plan_sweep`：凡登记行
`run_id == own_run_id` 且三条件（pid 存活／镜像指纹匹配／创建时刻匹配）成立者
`kill_pid` 整树收割。所有派发期 spawn 经 ambient spawn sink 写
`.gsa/process_trees/<call_id>-<pid>.json`（`run_id=own_run_id`，`job_name="call"`）——
**受管后台任务 shell 亦不例外**。昨晚 ep96（auto-bg 训练进程 21:33 起跑、21:36 run 尾消失）
即此机理；TER T2.3「后台存活」判据只在单 run 内成立的原因即 sweep 在 run 边界接管。
另**装配期 Start sweep**（`SweepMode::Start`，仅宿主装配时一次）扫「宿主已死的孤儿行」：
会话内跨 run 场景下宿主进程存活 → 登记行 parent_chain 含存活 orz pid → `ParentAlive`
拒绝，天然不杀；跨会话（新进程装配）时旧会话孤儿行被扫净＝0bq 兜底初衷，**保留不动**。

**勘定结论二（豁除面）**：跨 run 存活的修订点是**唯一**的——finalize sweep 对受管后台任务
开豁除口；Start sweep、前台域拆树、0bq 孤儿兜底全部不动。

### §2.3 30h 兜底执法点已在，且显式后台并非真无界

绝对兜底唯一执法点＝actor poll tick：
`p.start_time.elapsed() > p.timeout.min(BACKGROUND_MAX_RUNTIME)`（terminal.rs:1713），
`BACKGROUND_MAX_RUNTIME = 36_000s`（terminal.rs:50）。auto-bg／Ctrl+G 两路径在后台化瞬间
把 `process.timeout` 收敛为 `BACKGROUND_MAX_RUNTIME`；**显式后台**经
`resolve_effective_timeout`：正向 `timeout` 钳 `ABSOLUTE_MAX_TIMEOUT_MS`（bash/mod.rs:601，
36_000_000ms）、`0`/`None` → `Duration::MAX`——但 poll tick 的 `min(BACKGROUND_MAX_RUNTIME)`
**实际把 `Duration::MAX` 也钳在 10h 兜底内**。

**勘定结论三（295 批勘正）**：显式后台「timeout: 0 = 无界、寿命归 kill_task」的现表述与
实际不符——无模型超时的显式后台任务同样撞 10h 绝对兜底（signal=`max_runtime`）。
295 批「0/None → unbounded」表述随本批勘正；且这使 **30h 放宽＝单常量改一处执法点**，
无语义新增。

### §2.4 读出与完成提醒现役形态

- **读出**：`get_task_output` 返回 actor 内存尾窗（`output_byte_limit` 默认
  `DEFAULT_TOOL_OUTPUT_BYTES`=40K，front/tail 截断＋preview＋`read_file` 指针句）——
  **每次调用重复回传同窗尾**，无游标。长训练日志反复轮询下 token 冗余随日志长度增长
  （借鉴 b 的动因）。全量历史走 `read_file(output_file)`（磁盘全量，64MiB 保留上限）。
- **完成提醒**：`surface_bg_completion_reminders=true`（默认），`TaskCompletionReminder`
  在**每次工具调用**时查 `list_tasks`，把新完成任务以 `<system-reminder>` 附于工具结果顶部；
  `ReportedTaskCompletions` 去重。跨 run 送达（完成发生在两 run 之间→下一 run 首个工具调用
  附带）链路在树，列入 S2 验证点（D7）。
- **杀**：`kill_task`（`KillTaskAction` kind）在树，`requires_expr` 与读出同台（§2.5）。
- **任务快照**：actor 完成 tombstone 上限 100 条；输出文件 5GiB 硬帽（`GROK_MAX_OUTPUT_FILE_BYTES`）。

### §2.5 显式面封闭与伴随工具耦合

TER T1.3 封闭＝`hide_background_input` struct/serde 缺省 `true`：`is_background` 不进模型
schema、显式输入被拒（bash/mod.rs:2208）。`requires_expr`（bash/mod.rs:1850）在
`enabled_background ∧ ¬hide_background_input` 时**强制** `BackgroundTaskAction`
（get_task_output）＋`KillTaskAction`（kill_task）kind 同台。生产装配
`RETAINED_GROK_BUILD_TOOLS`（orz-host tools.rs:413）＝6 名单不含这两件 → 现役封闭态下
伴随工具不同台，可观察性由 180s 中间回报（PID＋落盘输出）＋完成提醒＋终止命令承担。
**逃生阀重开＝宿主装配一处注入 `hide_background_input:false` ＋保留面名单 +2**，零新表层。

## §3 设计定案（D1–D8）

### D1 通道形态＝Grok 显式后台面受控重开（选型定案落细则）

1. 宿主装配 `run_terminal_cmd_tool_params()` 增注 `"hide_background_input": false`
   （与现役 `allow_background_operator:false` 同一注入点，orz-host tools.rs:174）；
   BashParams struct/serde 缺省 `true` **不动**（库层缺省保持封闭，封闭态仍是其他宿主
   的安全缺省；orz 生产以显式注入重开——与 T1.3「宿主装配默认」边界同构反转）。
2. `is_background` 进模型 schema（描述已自解释在树：bash/mod.rs:348「Set to true for
   long-running commands…returns a task id immediately…notified on completion, so do
   not poll or sleep-wait」）。`enabled_background=true` 维持；`&` 封闭
   （`allow_background_operator=false`）**不动**——模型侧「一次调用＝一个结果」纪律保持，
   显式后台走参数不走 shell 操作符。
3. 声明即立即后台化：`is_background=true` 直接走 `handle_run_background` 路径，**不进
   前台、不烧 180s 预算**（在树行为，零改动）。
4. 伴随工具同台（requires_expr 强制）：`RETAINED_GROK_BUILD_TOOLS` +2＝
   `get_task_output`、`kill_task`。**工具面对账（五项待勘③）**：模型面 8 工具面纪律
   （MODEL-RESIDUAL-PRESSURE-INVENTORY 登记冻结；blackboard_write 为第一显式例外）
   本线为**第二显式例外，+2**（一裁决两件，不可拆——requires_expr 封闭耦合，只开参数
   不同台读/杀属残废面，业界无此形态）。随 §14.87 登记；8 工具面纪律文本本身不改。

### D2 收割豁除面（五项待勘①）＝登记行寿命标记＋finalize 豁除，0bq 并存

1. `ProcessTreeRecord` 增 `lifetime` 字段：`"call"`（缺省，serde default 向后兼容旧行）／
   `"session"`（受管后台任务）。写点＝宿主 spawn sink（lib.rs:1214）：sink 从
   `SpawnObservation` 读 lifetime；工具层在三种后台化 spawn 点（`handle_run_background`／
   `transition_to_background` 前的 spawn 不适用——它 spawn 时是前台，标记在**转换时刻
   改写**该任务登记行）把「本 spawn 属会话域」传递进 observation（xai-tty-utils
   `register_spawn` 载荷 +1 字段；不改 sink 作用域契约）。
2. **finalize sweep 豁除**：`plan_sweep(Finalize)` 对 `lifetime=="session"` 行统一
   `Refuse(record, SessionOwned)`（新 `SweepRefusal` 变体，审计行可见、不杀）；
   `"call"` 行判定矩阵原样。豁除记账走现役 sweep fact 通道，**零新 journal 事件族**；
   refusal 键 `session_owned` 若入 frozen fixture 面随 S2 三处同改（schema／Python 镜像／
   fixtures，沿 0ae 纪律）。
3. **Start sweep 不动**：装配期孤儿清扫对 session 行照扫（宿主已死＝会话域终止，残留树
   与行由下会话装配按三条件收净）——0bq「run 尾/装配扫净游离进程」兜底初衷完整保留；
   豁除只作用于「本 run 收尾」这一层，并行实例保护（F-BE-1）语义不变。
4. **统一豁除口径＝三种后台化路径一律 `session`**（显式 is_background＋auto-bg 180s＋
   Ctrl+G）。理由：①用户实际事故 ep96 正是 auto-bg 被 sweep 收割——只豁显式则事故原地
   重演；②auto-bg 定义上已属「长于 180s」的长程任务；③与 Claude Code「后台 shell 会话内
   跨轮存活」单一语义对齐，教学一句话可说清（D5）；④30h 兜底（D4）已统一封顶，无泄漏面。
5. 前台域拆树不动：前台调用结束其 per-call Job 句柄闭合整树回收（ep1 机理保留）——
   「一次调用＝一个结果」的拆树纪律是特性不是缺陷，长程意图必须显式声明（教学点，D5）。
6. **生命周期边界显式化（借鉴 c）**：受管后台任务寿命＝**会话域**——宿主进程存活期间跨
   run 存活（finalize 不杀、装配 sweep 因 ParentAlive 不杀）；宿主进程死亡＝会话域终止
   （actor 灭→Job 句柄闭合→直接树灭；breakaway 残余由下会话装配 sweep 收净）。本条写为
   显式声明，不沿 Claude Code「会话内跨轮」口径收缩、也不承诺跨进程重启。

### D3 Job 归属层级（五项待勘②）＝零新增层，现役 actor 任务域写成设计条款

不新增 Job 层、不重挂 per-call Job。§2.1 勘定结论一升格为设计语义：**后台化瞬间，
per-call Job 的所有权从派发域转为任务域**（原句柄由 actor `ProcessState` 持有至任务终；
派发侧 dup 句柄照旧关闭，无杀效）。树拆边界四处：`kill_task`（模型显式）／任务自然完成
（actor 收殓）／30h 绝对兜底（D4）／会话关闭（actor 关闭→句柄闭合）。训练进程在任务域
Job 内继承整树管理；主动 daemonize/breakaway 脱树的残余不受 Job 管辖，由 0bq sweep 在
会话边界兜底（与现役游离进程口径一致，不另设追踪）。

### D4 30h 兜底落点（五项待勘④）＝单常量改执法点＋env 口子＋文案两处

1. `BACKGROUND_MAX_RUNTIME`（terminal.rs:50）`36_000s → 108_000s`（30h）。唯一执法点
   （§2.3）自动覆盖三种后台化路径；kill 报 `signal=max_runtime` 不变。
2. `ABSOLUTE_MAX_TIMEOUT_MS`（bash/mod.rs:601）`36_000_000 → 108_000_000`（模型正向
   `timeout` 钳制同步放宽，语义「模型可选更短 cap」不变）。
3. env 口子（沿 T1.5 `GROK_IDLE_KILL_TIMEOUT_MS` 先例）：`GROK_BG_MAX_RUNTIME_MS`，
   缺省 108_000_000；**钳位下限 3_600_000（1h）**——绝对兜底不可配没（0/非法值回落
   缺省，不回落为「无兜底」）。
4. `monitor/types.rs` 的 36_000_000（DEFAULT/MAX_TIMEOUT_MS）**不动**——monitor 工具不在
   ORZ 保留面，零改动零升级。
5. 文案两处随批改写（教学面，见 D5）：schema `timeout` 描述与后台描述模板中
   「`timeout: 0` … disables the wrapper timeout entirely」按勘定结论三改写为 30h 封顶语义。
6. idle-kill 300s CPU 感知判据**原样保留**（用户裁决；对静默计算型训练友好——CPU 记账
   未知不判 idle 的宁可不杀方向不动）。

### D5 教学面（借鉴 a＋用户简化令）＝三件套，零常驻注入

1. **参数自解释**：`is_background` schema 描述一句话微调——现句保留，句尾补会话域边界
   一短句（「survives across runs until the session ends or the 30h safety cap」级），
   不超过一句。
2. **描述模板一句话**：现役 lifetime/kill_task 句保留（「the child's lifetime is owned by
   the model via kill_task」）；`timeout: 0` 句按 D4⑤ 改写。模板其余不动。
3. **结果文案 just-in-time**：`BackgroundTaskStarted` summary（现役「Background task
   {task_id} started」＋retrieval_hint）补一行边界句（跨 run 存活＋30h 封顶＋增量读出
   指引），只在起任务时花 token；`get_task_output` 返回文案带增量读数（D6）。**禁长备注
   常驻注入**：零新增常驻 token、零新触发规则、零 prompt 常驻段（用户令，P9 同族纪律）。
4. **完成 reminder 现役保留**（`surface_bg_completion_reminders=true` 链路零改动）。

### D6 增量读出（借鉴 b）＝服务端每任务字节游标

1. 定案：`get_task_output` 对**运行中任务**采用服务端游标——会话域 per-task
   `已回传字节 offset`（actor 侧 `ProcessState` 字段承载，随任务灭）；每次调用返回
   `[offset, now)` 新增段（沿用现役解码梯与 40K 单次钳制），并照报 `total_bytes`；
   返回文案一行「N new bytes since last read; M total」。
2. 首次调用游标从 0 起（返回已有全部尾窗内增量，含启动即产出的历史段——语义＝
   「自上次读出以来的新增」，与 PULL 自描述增量头同族）；**终态任务**（完成/被杀）的
   末次调用返回余段后游标封口，重复查询同现役尾窗（tombstone 无游标，信息不丢）。
3. 全量历史仍走 `read_file(output_file)`（现役指针句不变）；单次返回钳制、截断 preview、
   软换行现役语义不动。
4. 契约面：`TaskOutputResult` 增 optional `new_bytes: Option<u64>`（None＝旧语义/终态
   重复查询）；schema 与 Python 冻结镜像三处同改（沿 0ae/0bg 纪律）。TUI/web 无新渲染
   （字段进既有结果文本）。

### D7 契约面变更清单与跨 run 完成提醒验证点

| # | 面 | 变更 | 兼容 |
|---|---|---|---|
| 1 | `ProcessTreeRecord`（orz-host，`.gsa/process_trees/*.json`） | +`lifetime`（serde default `"call"`） | 旧行零迁移 |
| 2 | `SpawnObservation`（xai-tty-utils） | +`lifetime` 载荷字段 | 缺省 `"call"` |
| 3 | `SweepRefusal` | +`SessionOwned` 变体（refusal 键 `session_owned`） | frozen 面（如有）三处同改 |
| 4 | `TaskOutputResult`（xai_tool_types） | +`new_bytes: Option<u64>` | None＝旧语义 |
| 5 | schema 文案（is_background／timeout／后台模板） | 三句改写 | 无契约键变化 |
| 6 | journal 事件族 | **零新增**（sweep fact／tool_running／tool_completed 现役承载） | — |

跨 run 完成提醒验证点：完成发生在 run 间空窗时，`ReportedTaskCompletions` 去重集不吞
下一 run 首个工具调用的提醒（S2 钉＋S4 真机核）。

### D8 批序与判据（S2–S4 预登记）

- **S2 落码＋钉子**触点：orz-host `tools.rs`（注入+保留面+2）／`lib.rs`（sink lifetime
  传递＋Finalize 豁除接线）／`process_tree.rs`（lifetime 字段＋SessionOwned＋env）；
  xai-tty-utils（SpawnObservation）；orz-tools `terminal.rs`（30h 常量＋env＋转换/显式两
  路径标记）＋`bash/mod.rs`（ABSOLUTE_MAX_TIMEOUT_MS＋描述模板＋is_background 边界句＋
  summary 边界行）＋`task_output`（游标＋new_bytes）。钉子面：①显式声明即后台化不烧
  180s；②session 行 finalize 豁除（refusal=`session_owned`）×call 行照杀对照臂；③Start
  sweep 对 session 行照扫（宿主死）；④30h 生效路径（tick 越线→`max_runtime`）＋env 钳位；
  ⑤游标增量语义（首读/续读/终态封口）；⑥schema 可见性（is_background 在面＋伴随同台＋
  封闭态回归）；⑦教学三件字面钉；⑧跨 run 提醒不吞。
- **S3 载体重建**：字节判据随 S2 定（预期＝`is_background` 字面进 schema、`get_task_output`/
  `kill_task` 名单、`session_owned` 键、30h 常量字面），双平台重建进体。
- **S4 真机判据（294 批预登记不变）**：①一次跨 run 过夜训练在 orz 会话内存活至完成
  （对照 ep96）；②run 尾 sweep 后存活（journal：finalize 臂 `session_owned` refusal 行）；
  ③30h 生效路径可回放（env 或短钳演示）；④完成提醒跨 run 送达；⑤增量读出真机读数
  （轮询 token 降幅对照现役整读）。

## §4 与既有设计的关系

- **ADR-0010 §14.55（TER）修订三点**，由 **§14.87 新节**承载（五项待勘⑤）：T1.3
  「模型面封闭」在 ORZ 生产装配转「显式受控重开」（库层缺省不变）；T1.5「10h 绝对兜底」
  →30h；T2.3 单 run 存活判据→会话域跨 run 存活（本稿 D2⑥）。§14.55 其余条款
  （去硬杀、180s、idle-kill、W-F13 等）不动。
- **0bq/0z（进程树登记与双段 sweep）**：机制全保留；唯一变更＝finalize 臂豁除 session 行
  （D2）。0z S2 §4.2 的「no record, no kill」硬ening 方向不变。
- **0aq RS 面**：无涉（零安全边界放宽——豁除面仅受管登记任务，游离进程兜底原样）。
- **AUTH-PULL-SELF-DESCRIPTION**：D6 增量语义与其「分区版本计数＋未读徽章」同族异层
  （任务域 vs 黑板域），不共用游标。
- **0dd（会话生命周期）**：会话域终止边界（D2⑥）与 0dd 用户主权语义衔接——任务随宿主
  进程灭，不由归档动作触发。

## §5 风险与接受项

1. **豁除＝泄漏面扩大**：受管后台任务跨 run 存活意味着被遗忘的任务至多存活 30h／会话域
   ——用户显式裁决接受；输出 5GiB 帽与 idle-kill 300s 抑制失控面。
2. **auto-bg 统一豁除的语义扩张**：普通长命令 auto-bg 后不再随 run 收殓。接受（②③④
   理由，D2④）；真机轮如现「僵尸服务堆积」摩擦，S4 后可复议收窄为显式+Ctrl+G 两路径。
3. **游标与会话存档**：游标是 actor 内存态不入档——存档回放不依赖游标（journal 与
   output_file 为真相源），零对账面。
4. **伴随工具 +2 的模型面压力**：两工具描述均为一句话级（在树模板），常驻成本 ≈ 两条
   schema 行；已由 296 裁决接受。
