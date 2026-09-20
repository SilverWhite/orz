# 过夜批实施报告：S3 摩擦五项 S1 落码——0aw ＋ 0au ＋ 0at ＋ 0av ＋ 0ax（2026-09-20）

> 状态：**实施件全部落于工作树（未提交、未推送、未重建）**；日期 2026-09-20。
> 指令口径（用户）：「现在开一轮过夜任务……需处理的内容为 0au+0aw+0ax+0at+0av，请完成这些内容。完成后不进行提交推送与重建，请按照项目惯例落报告文档即可」。
> 基线：父仓 `HEAD=10f93c75`（索引 v3.97）；orz 子仓 pin 未动（`ac17a521`，0.6.3 bump；工作树脏＝**0am RLI 影批（既有）＋本批**，hunk 级分离——本批改动不含 0am 的 lif/rli/prompt 语义面）。
> 设计权威：0aw＝[`OS 委派执行设计 v1.1`](../HOST_RESOURCE_OS_DELEGATION_DESIGN_2026-09-20.md)（含裁决 ①–④）；其余四项＝[`N1–N6 深挖档`](0AR_S3_FRICTION_DEEP_DIVE_N1_N6_2026-09-20.md) §2–§6＋[`N1–N5 立项调研档`](N1_N5_ITEM_REGISTRATION_AND_N4_N5_RESEARCH_2026-09-20.md)。
> 关联：[BACKLOG 0at/0au/0av/0aw/0ax](../BACKLOG_AND_PRIORITIES.md) / [TODO](../../TODO.md) / 索引 `GAP-HOST-RESOURCE-ADMISSION-CALIBRATION` 等。

---

## 1. 一句话结论

五项 S1（落码/口径面）全部落地，**零提交、零推送、零载体重建、零版本 bump、计数不动（未闭合 40）**：

- **0aw（P1）**：OS 委派执行按设计 v1.1 全量落码——准入拒绝／分类器／两条 0z S2 动作臂整体退役，`resource_gate.rs` → **`resource_hint.rs`**（观测 ＋ 卷软提示 4 GiB、每 run 每卷一次）；run 级 Job 上限保留推导、只作上限（裁决点 B）。
- **0au（P1）**：派发前 run 级墙钟余量判定＋尾部保留 120 s——`remaining < 批墙钟＋收尾回合 60 s`（或 `< 120 s`）即结构化保留（无 `ToolStarted`、`cause=retrieval_dispatch_wallclock_reserved`）；S3/r1/r2 语料回放 **7/7 摩擦例全拦、7 例健康批不受扰**。
- **0at（P2）**：B 面 `unattributed_usable_count`（批级归因缺口，恒等式判据）＋A 面 `origin_query_id` 派发谱系（逐字＞归一化＞候选池回溯＞leader 谱系五规则）；单 query 批 payload 逐字节不变。
- **0av（P2）**：`mechanical_audit_update{kind:"retrieval_batch"}` 批读数落盘（activation_id/usable/cap/retrieval_calls/terminal_reason，与 batch_close 单源同值）；模型面零改动、不新增事件类型；schema 按 kind 条件分支。
- **0ax（P1）**：S1 口径面落码——无 URL 合成答案**不入可用额度**（阈值 5/护栏 10 不再可被不可引用文本凑满）、单列 `synthetic_answer_count`＋倒数行可引用性披露；S2（本地 SERP 车道评估）/S3（fetch 指纹兜底）为评估件落报告（§7），S4 留真机。

各步 S3（真机复验/随官方跑批收取）按批序留待下一轮，与 0ar/0ah 等既有批序纪律一致。

读数：orz-loop lib **816/0/3**、orz-host lib 串行 **319/0/5**、orz-assurance **246/0**、orz-bin 全目标绿（main 12/0＋acaf_e2e 23/0＋stdio_e2e 1/0 等）、`cargo fmt --check` 全净、clippy 相对 HEAD **净零新增**、`check_repository` 门禁 `error_count=1`（唯一＝orz 子仓脏树——不提交批的预期态，0ar S2 过夜批同形）、fixture 生成器重跑 diff 仅限本批两件、pytest 契约面（journal/mechanical/retrieval 过滤）**100/0**＋计数一致性钉 **15/0**。

## 2. 0aw：宿主资源面 OS 委派执行（S1–S4 落码，S5 留真机）

### 2.1 落点与做法

| 件 | 落点 | 内容 |
|---|---|---|
| **模块重写** | `orz-host/src/resource_gate.rs` → **`resource_hint.rs`** | 删除：`ActionClass`／`HEAVY_TOOLS`（run_tests）／`HEAVY_PROGRAMS`（109 程序名）／`CONDITIONAL_PROGRAMS`／`CONDITIONAL_HEAVY_TOKENS`／`WRAPPER_PROGRAMS`／`COMMAND_FLAGS`／`classify_action`／`classify_command`／`segment_is_heavy`／`skip_wrapper`／`has_file_redirection`／`is_env_assignment`／`normalize_program`／`is_command_flag`／`GateDecision`／`ResourceGate.evaluate(_for_volumes)`／`HEAVY_RELEASE_COMMIT_HEADROOM_PERCENT`／`HEAVY_RELEASE_FREE_BYTES`／`CODE_RESOURCE_INSUFFICIENT`／四条 DENIAL 定案句／`headroom_percent_floor`（拒绝文案专用）。保留：`HostCapacitySnapshot`／`SourceQuality`／`CapacityProbe`／`SystemCapacityProbe`（Windows `GetDiskFreeSpaceExW`+`GlobalMemoryStatusEx`；Linux `statvfs`+`CommitLimit/Committed_AS`——注释明确**降为观测字段**）／`write_targets`（目标卷解析，含 here-string/heredoc payload 剥离链）／`tier_for`＋档梯常数（**只作观测标签**）／`default_job_limits`／`gib`。新增：`VOLUME_HINT_FREE_BYTES = 4 GiB`（用户裁决）＋`ResourceHint`（`read_for_volumes`＋`soft_hint`＋`HashSet` 每 run 每卷去重） |
| **接线改写** | `orz-host/src/lib.rs` | `install_resource_safety` 保留（探针＋`default_job_limits`＋`install_global_run_job`；警告文案改「OS 答复面为兜底」）；**删除** `evaluate`/`Refuse` 分支、`host_resource_denied` 注入、hard 档树杀臂（`terminate_heavy_call_jobs`＋`resource_exhausted` planned/executed 生产端，裁决 ④-1）、回收触发（门汇点与 hard 臂内 `run_reclaim_pass` 调用＋`reclaim_performed` 生产端，裁决 ④-2；`reclaim.rs` 模块**保留不接线**＝「只在真 ENOSPC 面用」候选）、`in_flight_targets` 登记与 `register_live_call_job`（既有死代码，同面清理）；**保留** `live_call_jobs` 登记表（`DispatchGuard::drop` 关 per-call job 句柄，drop 时 debug 记 call_id）与跨档快照（`host_resource_snapshot`，tier_change/run_start 两触发）与 `drain_host_resource_facts`。汇点新流程＝`write_targets` → 一次 `read_for_volumes` → 跨档快照 → 软提示（不阻断，附结果头部）；`run_tests` 预检门同步退役。访问器 `resource_gate()`/`with_resource_gate()` → `resource_hint()`/`with_resource_hint()`；`with_host_resource_safety(_limits)` 方法名保留（生产调用点 `orz-bin/main.rs`＋`acp_server.rs` 各一处不动） |
| **登记面** | `orz-host/src/process_tree.rs` | `ProcessTreeRecord` 删 `action_class` 字段（serde default 宽松反序列化——历史登记行多出的键自然忽略，扫除三条件不依赖）；测试同步 |
| **S3 内核面** | `resource_hint.rs::default_job_limits` | **推导原样保留、只作上限**（裁决点 B）——`min(min(80 % × limit, limit − 4 GiB), 装配期余量 − 1 GiB)` 下限 2 GiB、≤0 不设；`active_process` 2×cores+8≥16 与 `cpu_rate` 80 % 维持；Linux 无强制面照旧（无 commit limit 即不设上限）。注释补裁决依据（Run B 死因＝orz 在 Job 外，Job 上限是保命面不是准入面） |

### 2.2 判据对照（设计 §7）

| 判据 | 状态 | 承载 |
|---|---|---|
| 1 拒绝归零（`host_resource_denied`=0、无 `resource_insufficient`） | ✓（机制） | 拒绝臂物理删除；真机归零读数留 S5 |
| 2 软提示可核（注入 free<4 GiB ⇒ 照跑＋一次提示；同卷第二次不重复） | ✓ | `volume_soft_hint_attaches_at_the_call_tool_seam`（Fixed 探针 1 GiB：exit 0＋恰一条 `[资源软提示]`＋机械读数＋第二次无提示）＋`any_short_write_target_volume_hints_once_per_volume`（双卷去重按卷） |
| 3 内核上限在位（读回非空、e2e 钉保持） | ✓ | `host_resource_safety_installs_real_probe_and_ceilings`（真实探针＋`global_run_job` 读回 `is_kernel_enforced`/`readback().is_enforced()`）＋`tool_spawn_is_really_bounded_by_the_run_job`（300 MB 预算子进程 MemoryError，Windows e2e）原样绿 |
| 4 无 OOM 杀、墙钟不恶化 | 留 S5 | 真机对照面 |
| 5 观测面保留（跨档快照；历史 journal verifier 仍绿） | ✓ | 快照两触发保留；S3 三 run 历史 journal（denied=14/11/4、exhausted=0、reclaim=0）经 Python verifier 回放零错误（本批条件分支 schema 改动**向后兼容**实测）（**勘误 2026-09-20**：该「零错误」限于 mechanical-audit／资源族；整卷另有旧二进制真实违约 35＋2 条，见 [`审查修复批 §3`](0AT_0AU_0AV_0AW_0AX_S1_REVIEW_FIXES_2026-09-20.md)） |
| 6 动作臂归零（`resource_exhausted`=0 且 `reclaim_performed`=0 不回涨） | ✓（机制） | 两生产端删除；S3 基线本就 0/0/0（三 run 实测 0/0/0） |
| 7 边界不动（0z B/C/E/F 未触及部分；官方口径） | ✓ | `xai-tty-utils/resource_job.rs`、`orz-assurance/journal/families.rs`（三事件 verifier）、`journal_pending_host_resource_facts` 排空链、`task.toml`/镜像/verifier/数据集 pin 全部未动 |

### 2.3 新增/改写测试（orz-host）

删除 25 件门/分类器面测试＋1 件 `run_tests` 门测试；新增/改写 13 件 hint 面：软提示触发与去重（含双卷）、水位边界（恰 4 GiB 不提示）、读数不可得无提示不阻断、单调用单探针读（沿 F-BE-12 纪律）、tier 梯观测标签、`default_job_limits` 推导（含 0 ⇒ 不设）、系统探针真机读、写目标解析三件＋here-string/heredoc/payload 语义三件（语义与旧实现逐字一致，仅断言对象从「重档判定」换为「写目标集合」）。

## 3. 0au：检索尾部派发预算（S1 落码＋S2 语料回放，S3 留官方跑批）

### 3.1 落点与做法

| 件 | 落点 | 内容 |
|---|---|---|
| **共享常数与纯函数** | `orz-loop/src/retrieval/batch_close.rs` | `CLOSE_ROUND_MARGIN_SECS=60`（一次收尾回合）／`RUN_TAIL_RESERVE_SECS=120`（方案 90–120 s 取上沿）／`WALLCLOCK_RESERVED_CAUSE="retrieval_dispatch_wallclock_reserved"`；`wallclock_reserved(remaining, batch_wallclock)` 纯函数——`remaining=None`（无已知 run 上限）恒不保留；两支任一即保留：①`remaining < batch_wallclock+60`（S3 五次 trailing 的直接成因）②`remaining < 120`（尾部保留，落盘窗口） |
| **派发前余量判定** | `agent_loop.rs` 预扫描 | 在 D3 合并/溢出扫描之后追加：上限解析序＝**env（`ORZ_MAX_WALLCLOCK`，生产读源）＞ controller 测试 seam（`run_wallclock_limit_secs`，本批新增字段＋`with_run_wallclock_limit_secs`）＞ None**；`remaining = 上限 − run_started_at.elapsed()`；批墙钟＝**首个可派发调用**的档位默认（`classify_retrieval_effort`→`wallclock_default()`，180/300/450 单一来源；合并激活共享同一批预算）。保留 ⇒ 本批**全部可派发位**（合并 leader/被合并/溢出）一并拒绝 |
| **保留拒绝臂** | `agent_loop.rs` 工具循环 | 模板同 D3：无 `ToolStarted`、`stamp_failure(Refused)` 过漏斗、`ToolCompleted(error)` 携 `error=WALLCLOCK_RESERVED_CAUSE`；文案只报事实（run 剩余/批墙钟/收尾保留 60 s/落盘保留 120 s）；不喂 deny 断路器（保留不是失败）；post-batch 间隙一次性重述（`[上轮检索未派发]`＋cause 自描述＋「尾部保留给落盘」，**不做重派邀请**） |

**口径说明（两点）**：① 保留判定对主/检两车道同位生效（run 预算是 run 级的）；无上限（env 缺席/为 0）的 run 不判定——S3 取证口径摩擦例全部产生于墙钟到点的 run，与「自然收工无预算可判」一致。② 溢出位与保留位同轮并存时保留优先（先于 D3 合并回执臂判定）。

### 3.2 S2 语料回放（判据 ①）

对深挖档 §3.1 四轮横向表逐条套新规则（extended 300 s 档，need=360 s）：

| 例 | 余量 | 新决策 | 历史结局 |
|---|---:|---|---|
| r1 torch／r1 gpt2／r2 torch／r2 gpt2／r2 extract／S3 extract 26 s | 183/153/39/123/191/**26** | **RESERVED** | 五次 trailing（截断）全拦 |
| S3 torch 117 s | 117 | **RESERVED** | 近失（78 s 侥幸收口）——保守侧改变 |
| S3 extract 224 s | 224 | **RESERVED** | 历史正常收口，但其「达标」系合成凑阈（N5）——0ax 新口径下该批本不达标，两修正同向 |
| r3 torch 462/668、S3 torch 780、S3 gpt2 642/400、S3 extract 848/466 | ≥400 | dispatch（不误伤） | 健康批全保留原行为 |

**结论：摩擦例 7/7 全拦；健康批 7/7 不受扰；边界侧行为改变 1 例（保守方向，且与 0ax 修正同向）。判据 ①「回放中 `remaining < batch_wallclock` 的派发数＝0」达成；判据 ③「不误伤剩余充足续派」达成。判据 ② 模型面只报事实——集成钉断言。**

### 3.3 新增测试

- 纯函数三钉：无上限不保留／S3+r1+r2 摩擦例逐条回放全拦＋边界（239/240 s）方向钉／尾部保留支（119 s 即便 standard 档也拦）。
- 集成钉 `retrieval_dispatch_reserved_when_run_wallclock_is_short`：seam 注入 10 s ⇒ ① 无 `ToolStarted` 配对＋`ToolCompleted.error=cause`；② post-batch 一次性重述进入下一轮请求；③ run 本身正常完成（保留不是失败）。
- 回归：既有 `subagent_wallclock_timeout_returns_partial_evidence_normally` 等不受扰（全量 816/0/3）。

## 4. 0at：逐 query 归因口径与谱系（S1 B 面＋S2 A 面落码，S3 口径同步随批）

### 4.1 落点与做法

| 件 | 落点 | 内容 |
|---|---|---|
| **B 面（小）** | `batch_close.rs`＋`evidence.rs`＋契约 | `unattributed_usable_count(queries, evidence)`＝批级可用 − Σ 逐 query 可用（逐 query 归因键＝逐字 `search_query` 精确匹配，口径不变）；`build_structured_result` 在**多 query 批**落顶层可选字段（单 query 批不落——判据 ③「单 query 批 payload 逐字节不变」） |
| **A 面（中）** | `batch_close.rs`＋`evidence.rs`＋契约 | `origin_query_assignments(queries, evidence, query_ids)` 五规则有序短路：①单 query 批全 `None`（不落字段）②逐字匹配 ③归一化匹配（trim＋空白折叠＋小写）④派生证据（`search_query=None`）identity 命中某已归因 web_search 证据的候选池 ⇒ 跟随之（「派生证据归发起它的那次检索」）⑤其余 ⇒ **leader 谱系**（首个派发 query；「子代理自查归所属派发 query」）。多 query 批在工具证据 ledger 条目落可选 `origin_query_id`（条目与证据同序——source_ledger 前 evidence.len() 条即工具证据；[DOC]/[SOURCE] 声明行不落） |
| **id 单源化** | `evidence.rs::query_ids_for` | `QRY-{sha256(call_id)[..8]}`（＋多 query `-N` 后缀）生成收敛为单函数——`query_summary.query_id` 与 `origin_query_id` 共用同一 id 域（契约 pattern `^QRY-[A-Za-z0-9.-]+$` 逐字对应） |

**语义互补声明（重要）**：A 面谱系不留空（每条证据归属到某个派发 query——覆盖率由构造保证）；B 面缺口继续用**字面匹配**口径度量「不可按 query 字面归属的可用证据」，两者不是同一把尺：A 面回答「这条证据服务哪个派发 query」，B 面回答「主代理还能不能按 query 字面读覆盖」。设计稿 v1.1 §5.4「逐 query 披露＋主代理续派」在**字面匹配口径**下仍不得作为覆盖保障依据（S3 实测 4/4 失真的结论维持；0at S3 口径同步时一并转录）。

### 4.2 判据对照

| 判据 | 状态 | 承载 |
|---|---|---|
| ① 恒等式 Σ 逐 query ＋ unattributed ＝ 批级可用 | ✓（附边界） | `unattributed_count_completes_the_per_query_identity`＋`unattributed_count_saturates_when_content_overlaps_two_queries`（**勘误 2026-09-20 审查修复批**：恒等式在逐 query 桶 digest 互斥时按构造成立；同一内容被两个派发 query 检索到时 Σ 逐 query 可超批级、缺口饱和到 0——装配点以机械告警留痕，原「只在口径漂移时触发」表述不成立）；S3 归档回放（**旧宽口径**，与深挖档 §2.1 同尺）gap=4/6/3/6/0/0/0 逐批一致；**实现口径（0ax 收窄后）同语料多 query 批缺口为 1/4/0/—/5**（torch-00/01、gpt2-00、extract-02；单 query 批恒 0）——原登记值是旧口径数字，非新字段的真实产出口径，§10-3 同此更正 |
| ② 归一化后逐 query 覆盖率 ≥90 %（S3 语料回放） | ✓（机制＋推导） | leader 兜底使每条多 query 批证据必带 `origin_query_id`（覆盖率 100 %）；S3 归档无证据级 `search_query`，覆盖率由构造＋单测钉证明，真机复核归 S3 收取 |
| ③ 单 query 批 payload 逐字节不变 | ✓ | 三字段（unattributed/origin_query_id/synthetic）全部条件落盘；既有 evidence/dispatch 测试（含 payload 断言）零改动全绿 |

### 4.3 新增测试

`origin_query_assignment_follows_the_rule_ladder`（五规则逐条：逐字/归一化/池回溯/leader/单 query 全 None）＋`unattributed_count_completes_the_per_query_identity`＋端到端（§7 合并派发钉：多 query 批 ledger 携 origin_query_id 且等于 `query_summary[0].query_id`）。

## 5. 0av：检索批次数读数落盘面（S1 落码，S2/S3 留真机/可选件）

### 5.1 落点与做法

| 件 | 落点 | 内容 |
|---|---|---|
| **批读数事件** | `dispatch.rs` 批收尾点 | assessment 落账后即写 `mechanical_audit_update{kind:"retrieval_batch", payload:{activation_id, usable, cap, retrieval_calls, terminal_reason}}`——五值与 `batch_close` 单源 helper 同源（usable=宽口径可用、cap=MECHANICAL_CAP、terminal_reason=收尾成因映射）；**模型面零改动**（不进审查表/报告块——「不加说明性内容」纪律）、**不新增事件类型**（既有家族新增 kind）。Err 臂（subagent_failed）不落——该路径非正常收尾成因域 |
| **kind 枚举** | `mechanical_audit.rs` | `KIND_RETRIEVAL_BATCH="retrieval_batch"`＋契约钉改写：kind 枚举 8 值逐字互证；payload 校验改**按 kind 条件分支**（retrieval_batch 五键形状／其余 kind 均一形状） |
| **schema** | `runtime/mechanical-audit-update-event-payload-v0.2.schema.json` | kind 枚举＋`allOf` 两分支（if retrieval_batch → 五键 required；else → 均一四键）；description 补 0av 定案与动机（倒数行只入模型面＋headless 侧车 500K 里程碑门 ⇒ 结构性无落盘面，0ar 判据 7 后段转可核） |
| **Python 镜像** | `assurance/run_event_journal_validation.py::_verify_v02_mechanical_audit` | kind 集合＋retrieval_batch 五键/非负整数/非空串校验（Rust↔Python 双侧同步；历史 journal 回放零错误实测） |

### 5.2 判据对照

| 判据 | 状态 | 承载 |
|---|---|---|
| ① 倒数行读数＝该批 `usable_source_count` 可机械重算 | ✓（机制） | 批收尾 journal 读数与 assessment/倒数共用同一 `usable_source_count` 值（单源）；真机重算演示留 S2（随下一轮官方跑批） |
| ② 不新增事件类型 | ✓ | `mechanical_audit_update` 家族内新增 kind |
| ③ 模型面字节不变 | ✓ | 纯 journal 面；模型面消息零改动（无测试需要变） |

新正例 fixture `mechanical-audit-update.retrieval-batch.valid`（生成器单一事实源＋check_repository 登记）；schema 负例路径（budget 形状坏 payload）对条件分支的拒绝方向经 Python `jsonschema` 直测验证。

## 6. 0ax：检索形态 URL 完整性（S1 口径面落码；S2/S3 评估件见 §7；S4 留真机）

### 6.1 S1 落码

| 件 | 落点 | 内容 |
|---|---|---|
| **合成判定与单列** | `batch_close.rs` | `is_synthetic_answer(ev)`＝`web_search_result` 且 `candidate_urls` 空（机械形态：宿主结构化 seam 未带来任何 citation URL——DeepSeek 侧 `url_citation` 恒空即此形态）；`synthetic_answer_count(evidence)`（宽口径内、按去重键去重） |
| **可用口径收窄** | `batch_close.rs::usable_source_count` | 合成答案**不入可用额度**——阈值 5/机械护栏 10/可见倒数三处共用此尺（§3.3 一致性不变），「不可引用的文本」不再能凑满阈值（S3 act 01 的 5/5 全合成提前收尾即此类）；FP-2/批阈值数值/工具面全部不动 |
| **披露面** | `countdown_line`＋committed payload | 倒数行 `synthetic>0` 时附「另有 N 条为无 URL 合成文本，未计入可用额度（不可引用）」（=0 时整行与旧形态逐字节一致）；payload 顶层可选 `synthetic_answer_count`（>0 才落——不扰动单 query 批与既有形态）；判据 ② 的「如实标注全为合成」由该字段承担（new_usable=0 且 synthetic>0 即全合成批的可机械读形态） |

### 6.2 S2 语料回放（S3 三 run 七批归档）

对 `official-verify-timeout3-s3` 三 run 的 `retrieval-results` 归档逐批重算（旧口径=ledger 可见性去重计数；新口径=剔除 web_search_result 无 candidate_urls 条目）：

| 批（7 件） | 旧 usable | 新 usable | 合成单列 | 阈值结局 |
|---|---:|---:|---:|---|
| torch-00（…-00-r0-34） | 5 | 1 | 4 | MET → no |
| torch-01（…-01-r0-d6） | 6 | 4 | 2 | MET → no |
| gpt2-00（…-00-r0-bf） | 5 | **0** | **5** | MET → no |
| gpt2-01（…-01-r0-5b） | 5 | 4 | 1 | MET → no |
| extract-00（…-00-r0-8d） | 5 | 2 | 3 | MET → no |
| extract-01（…-01-r0-5e） | 5 | **0** | **5** | MET → no |
| extract-02（…-02-r0-51） | 6 | 5 | 1 | MET → MET |

**合计：旧口径 37 条「可用」中 21 条（56.8 %）为无 URL 合成文本；7 批中 6 批的阈值提前收尾结局被翻转（唯一保持 MET 的批次其 5 条全部可引用）。判据 ①「无 URL 结果占比／可引用来源数可机械读」达成。** 与深挖档 §6 的归因链互证：S3 extract 的「潜在解法退化」正是因为阈值被合成文本凑满、子代理按规则提前收尾——本批后该通路关闭。

（同批 0at B 面缺口回放：**旧宽口径**下 Σ 逐 query 与批级之差＝4/6/3/6/0/0/0，与深挖档 §2.1 表逐批一致；**勘误 2026-09-20 审查修复批**：实现口径（0ax 收窄后）同语料缺口为 1/4/0/—/5（多 query 批；单 query 批恒 0）——上值系旧口径数字，不作为新字段在真实语料上的产出读数，§4.2/§10-3 同此更正。）

## 7. 0ax S2/S3 评估件（按 TODO 批序为评估步；不落码，待放行）

### 7.1 S2 形态面：官方口径下启用本地 SERP 车道评估

- **开关已在位**：`ORZ_WEB_SEARCH_LOCAL`（`LocalSegmentedConfig::from_env()`，默认 off；`registry/types.rs` 装配期接线，off 即零行为变化）——本地分段检索车道（SERP 引擎链 → 逐页抓取 → 逐段抽取）自带 URL，是既有契约（2026-09-15 补强设计稿 G1–G4：三段预算 10/5/10 s·页、闸门默认开＋25 %＋词集封顶 12、解包 6 worker/6 s）。
- **可比性论证**：①任务/verifier/数据集 pin/官方墙钟全部不动，唯一变量是 `web_search` 的实现车道（provider 合成 vs 本地 SERP+fetch）——与 R3/R4 的「web 通道 A/B」同构，属**通道消融**而非口径变更；②检索批 `query_summary`/`usable_source_count`/阈值 5 的契约不变，新口径下本地车道每条 web_search 带引用池 ⇒ 天然计入可用，结果可与历史轮对表；③风险＝评测容器网络形态（无浏览器、直连受限）下 SERP 引擎可达性与 10 s 预算——G1/G3 补强即为此设计；④启用路径＝装置侧 env（适配器起 run 时注入 `ORZ_WEB_SEARCH_LOCAL=on`），**不动 `task.toml`、不启用 `eval_browser`**（既定裁决边界内）。
- **建议判据（随 S4 真机收取）**：同三题口径下 `web_search` 结果带 URL 占比 ≥90 %；`synthetic_answer_count` 恒 0；fetch 目标数（`web_fetch` 发起数）不低于 r3 基线；无 URL 车道对比批留作对照。
- **裁决点**：是否在下一轮官方跑批启用（装置侧 env 一行）——待用户放行，本批不动。

### 7.2 S3 fetch 侧兜底：指纹伪装 / reader 服务评估

- 结论：**方向成立、本批不落码**。两条路线：① Rust 侧 TLS/HTTP2 指纹伪装（`penumbra-x/rquest`（原 `wreq` 系）或 `lwthiker/curl-impersonate` 思路的 Rust 移植）——优点零外部服务、缺点新增重型依赖＋指纹库需随浏览器版本迭代＋Cargo.lock/双平台重建成本；② reader/正文提取服务（Jina Reader／Firecrawl／自托管 SearXNG+`crawl4ai`）——优点即时可用，缺点外部依赖与泄漏面（检索内容过第三方）需裁决。
- 推荐次序：先 S2 本地车道（零代码、自持 URL），S3 兜底只覆盖「本地车道也被反爬」的残余面；落码时以 feature-gated 依赖＋环境开关（沿 `ORZ_WEB_SEARCH_LOCAL` 先例）＋grep.app 复测为验收三件。**依赖新增需网络实测核证，与本批「不重建」边界冲突，故仅登记方向**（候选面与开源对照已在[`调研档 §2`](N1_N5_ITEM_REGISTRATION_AND_N4_N5_RESEARCH_2026-09-20.md)）。

## 8. 契约面汇总（本批全部增量）

| 契约 | 增量 | 性质 |
|---|---|---|
| `retrieval-result-event-payload-v0.2.schema.json` | 顶层可选 `unattributed_usable_count`（≥0；多 query 批）／顶层可选 `synthetic_answer_count`（≥1 才落）；`source_entry` 可选 `origin_query_id`（QRY pattern；多 query 批） | 可选字段（0ar S1 先例） |
| `mechanical-audit-update-event-payload-v0.2.schema.json` | kind 枚举 ＋ `retrieval_batch`；payload 按 kind 条件分支（五键批读数／均一四键） | 枚举扩展＋条件分支 |
| fixtures | `mechanical-audit-update.retrieval-batch.valid`（新）；`retrieval-result.merged-multi-query.valid` 扩展（+SRC-003/004、origin_query_id、两顶层字段） | 生成器单一事实源重跑，diff 仅限本批 |
| `check_repository.py` | 新 fixture 登记＋批注 | 同批 |
| `assurance/run_event_journal_validation.py` | `_verify_v02_mechanical_audit` 增 retrieval_batch 分支 | Rust↔Python 双侧同步 |

**向后兼容**：S3 三 run 历史 journal 经新 Python 校验器回放零错误（**勘误 2026-09-20**：「零错误」限 mechanical-audit／资源族；整卷另有旧二进制真实违约 35＋2 条——见 [`审查修复批 §3`](0AT_0AU_0AV_0AW_0AX_S1_REVIEW_FIXES_2026-09-20.md)）；三事件（`host_resource_denied`/`resource_exhausted`/`reclaim_performed`）schema/fixtures/`families.rs` verifier 全部未动（裁决 ④-3）。

## 9. 测试读数汇总

| 面 | 读数 | 备注 |
|---|---|---|
| orz-loop lib | **816 passed / 0 failed / 3 ignored** | 本批新增 0au 三钉＋集成一件、0at 两钉＋端到端扩展、0av 契约钉改写；HEAD 基线 803/1/3——那 1 失败＝契约先行的 schema 钉（schema 已先行改 8 kind），本批实现跟上后转绿 |
| orz-host lib（串行） | **319 passed / 0 failed / 5 ignored** | 删门/分类器面 26 件、新增 hint 面 13 件；并行全量下 4 件真进程测试负载抖动（单跑全绿，与 0AS「负载敏感串行绿」同族登记） |
| orz-assurance lib | **246/0** | families.rs 未动（历史 verifier 保留） |
| orz-bin 全目标 | main 12/0、signer 15/0、provision 2/0、acaf_e2e 23/0、real_flag 2/0、stdio_e2e 1/0 | stdio e2e 期望集含 `host_resource_snapshot`（保留面）不受扰 |
| fmt / clippy | `--check` 全净 / 相对 HEAD 净零新增 | clippy 77（HEAD）＝ 77（工作树），本批另修 question_mark 1 处 |
| pytest | 契约面过滤 100/0；计数一致性钉 15/0 | `run_event_journal_validation` 五向直测（valid/缺键/坏值/未知 kind/均一）全过 |
| 门禁 `check_repository.py` | `error_count=1`（唯一＝「orz submodule working tree is dirty」） | 不提交批预期态（0ar S2 过夜批同形）；其余全过 |
| fixture 生成器 | 重跑 diff 仅限本批两件 | 生成器与既有 fixtures 完全同步 |

## 10. 语料回放与历史兼容（S2 型判据收取汇总）

1. **0au**：四轮横向表 14 例回放——摩擦 7/7 全拦（含 S3 extract 26 s 最极端例）、健康 7/7 不扰、边界 1 例保守改变（224 s，与 0ax 修正同向）。
2. **0ax**：S3 七批归档——合成占比 21/37（56.8 %），6/7 批「合成凑阈提前收尾」结局翻转；gpt2-00/extract-01 两批新口径 usable=0（全合成，如实单列）。
3. **0at**：S3 归档 B 面缺口逐批 **4/6/3/6/0/0/0（旧宽口径，与深挖档 §2.1 一致）**；**勘误 2026-09-20 审查修复批：实现口径（0ax 收窄后）同语料多 query 批缺口为 1/4/0/—/5**——原句「B 面披露字段在真实语料上的值得到验证」按旧口径读数登记，实现口径读数以本更正为准；A 面覆盖率由构造 100 %＋单测钉证明。
4. **0aw**：S3 三 run 历史 journal（denied 14/11/4、exhausted/reclaim 0/0/0）经保留 verifier 回放零错误（**勘误 2026-09-20**：「零错误」限 mechanical-audit／资源族；整卷另有旧二进制真实违约 35＋2 条——见 [`审查修复批 §3`](0AT_0AU_0AV_0AW_0AX_S1_REVIEW_FIXES_2026-09-20.md)）——退役不损历史可校验性；S3 基线同时实证两条动作臂真机产出本就为 0（裁决 ④-5）。

## 11. 边界与留待

- **留 S3/真机（按批序，不阻断本批）**：0au S3（随下一轮官方跑批收取——真机派发保留行为与文案）；0aw S5（同语料真机回放拒绝数=0、无 OOM、墙钟对照）；0ax S4（真机「无 URL 占比／可引用来源数」入审计）；0av S2（真机 run 倒数行↔journal 读数机械重算演示）；0at S3（设计稿 §5.4 与 ADR-0010 转录口径同步——归因修好前「逐 query 披露不作覆盖保障依据」的降级表述随转录落）。
- **待用户放行/裁决**：0ax S2 启用 `ORZ_WEB_SEARCH_LOCAL` 进下一轮官方跑批（装置侧 env 一行）；0ax S3 fetch 兜底依赖路线（rquest 系 vs reader 服务）。
- **登记性偏差（无）**：本批全部落点与 TODO/BACKLOG 登记一致；`ResourceHint` 方法命名、`ResourceGate`→`ResourceHint` 改名面、`register_live_call_job` 死代码清理为同面卫生件。
- **本批未动**：`reclaim.rs`（能力保留、不接线）、`xai-tty-utils/resource_job.rs`、`families.rs`、0z B/C/E/F 未触及部分、官方环境口径（`task.toml`/镜像/verifier/数据集 pin）、FP-2、批阈值 5/10 数值、orz pin（`ac17a521`）、0am RLI 影批工作树（hunk 分离，未触碰）。

## 12. 文件清单（orz 子仓；父仓另见 §8 契约五件）

| 文件 | 变更 |
|---|---|
| `crates/orz-host/src/resource_gate.rs` | **删除**（语义迁 resource_hint） |
| `crates/orz-host/src/resource_hint.rs` | **新增**（观测＋软提示＋写目标解析＋job 限额推导） |
| `crates/orz-host/src/lib.rs` | 接线改写（0aw S1/S2/S4＋两臂退役＋run_tests 门退役＋汇点软提示） |
| `crates/orz-host/src/process_tree.rs` | `action_class` 字段退役 |
| `crates/orz-loop/src/retrieval/batch_close.rs` | 0au 三常数＋纯函数；0ax 合成判定/单列/可用口径；0at 谱系/缺口；倒数行披露 |
| `crates/orz-loop/src/agent_loop.rs` | 0au 预扫描余量判定＋保留拒绝臂＋一次性重述＋集成钉 |
| `crates/orz-loop/src/controller.rs` | 0au 测试 seam（`run_wallclock_limit_secs`＋builder） |
| `crates/orz-loop/src/retrieval/evidence.rs` | `query_ids_for` 单源＋三字段装配（条件落盘） |
| `crates/orz-loop/src/retrieval/dispatch.rs` | 0av 批读数事件＋合并派发端到端钉扩展 |
| `crates/orz-loop/src/mechanical_audit.rs` | 0av kind 常量＋契约钉改写 |

（父仓：`runtime/` 两 schema、`runtime/fixtures/` 两 fixture、`scripts/check_repository.py`、`scripts/generate_run_event_fixtures.py`、`assurance/run_event_journal_validation.py`、本报告；TODO/BACKLOG/索引三方同步同批。）

---

> 落款：过夜批（2026-09-20 深夜）；执行＝ZCode 主会话。判据对照、回放数据与读数以本文为登记面；提交/推送/重建待用户明示。
