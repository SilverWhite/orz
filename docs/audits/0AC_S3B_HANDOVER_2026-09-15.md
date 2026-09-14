# 0ac S3①-b 实施交接报告（含框架内部摩擦记录）

> 状态：**交接件（未提交、未推送、未重建；工作区保持原样）**；日期 2026-09-15。
>
> 触发：协调者指令——0ac ①-b 实施阶段**中途停止**，落交接文档；本轮**不再**启动新的实现 / 测试 / 构建 / 修复动作，**不**提交、**不**推送、**不**重建、**不**清理工作区。
>
> 关联：TODO `P0-0ac`（2026-09-14 拆分裁决）/ 索引 `GAP-MECH-IMMEDIATE-FEEDBACK`（v3.21/v3.25 条目）/ 设计稿 [`IMMEDIATE_RESULT_DELIVERY_AND_STREAMING_RETRIEVAL_DESIGN_2026-09-13`](../IMMEDIATE_RESULT_DELIVERY_AND_STREAMING_RETRIEVAL_DESIGN_2026-09-13.md) §2/§4/§10.3 / 审记 [`0AC_S3_IMPLEMENTATION_AUDIT_2026-09-14`](0AC_S3_IMPLEMENTATION_AUDIT_2026-09-14.md) / 修复报告 [`0AC_S3_FIX_REPORT_2026-09-14`](0AC_S3_FIX_REPORT_2026-09-14.md)。
>
> 口径声明：本件只登记**事实与恢复点**，不改契约、不改预算值、不改设计稿；凡未核证者一律标注「未核」，不写成结论。

---

## 1. 一句话状态

0ac S3①-b **未完成**：到达面三事件写点已提交（orz `f03b2a4f`）；投递面的**库侧机械件**（M2 队列 / M3 读数 / M1 子开关）以 **362 行未提交改动**躺在工作区且**未经本阶段编译与测试**；⑤接线、⑥子代理提前收口、⑦排队即时回报、③跨 run 时序钉**均未落**。出口条件（①-b 落码 + ③时序钉 + 门禁/镜像全绿）未达成。

---

## 2. 被打断时正在做什么（精确到文件 / 函数 / 行）

**正在进行的一步**：为 **⑤ M2 投递策略的宿侧来源面**做勘察（读代码，写点在勘察之后）。

按时间倒序的现场读数：

| 序 | 位置 | 读到了什么 | 用途 |
|---|---|---|---|
| 1 | `crates/orz-host/src/lib.rs:158-251` | `OrzHost` 字段表（`terminal` 187、`idle_kill_reported` 190、`web_search_semaphore` 210、`browser` 216 等） | 判定宿侧来源件挂在哪 |
| 2 | `crates/orz-host/src/lib.rs:263-292` | `OrzHost::{new, with_permission}` 构造链 | 追加字段 / accessor 的落点 |
| 3 | `crates/orz-host/src/lib.rs:1732-1872` | `impl LoopHost for OrzHost` 区已有的 **drain 形态**：`drain_terminal_idle_kills` 1833、`drain_host_resource_facts` 1858、`drain_process_tree_reap_facts` 1866 | `drain_completed_tasks` 的同形样板 |
| 4 | `crates/orz-host/src/tools.rs:323-437 / 441-450` | `build_toolset`（`FinalizedToolset` 装配）、`ToolsetRegistry{ toolset: Arc<FinalizedToolset>, … }` | 判定 `ToolBridge` 可否从宿侧就地取到（**未定论**，见 §5-C） |
| 5 | `crates/codegen/orz-tools/src/bridge.rs:636-684` | `ToolBridge::{list_tasks, drain_between_turn_bash_completions(reserved_ids)}`——完成任务过滤（`completed && kind != Monitor`）、owner 会话过滤、`ReportedTaskCompletions::mark_reported` 去重 | M2 的**真实来源面**（与 `TaskCompletionReminder` 共用同一「已报告」记账） |
| 6 | `crates/codegen/orz-tools/src/reminders/task_completion.rs:500-580` | `While you were idle, {n} … completed:` 批式文本格式 + `consumed_completion_ids`（模型「自己读到」的身份抽取） | 投递文本单一源；「框架投递 vs 模型自读」分开记账的既有机制 |
| 7 | `crates/orz-loop/src/agent_loop.rs:1089-3120` | `run_agent_loop` 骨架：终答候选分支 2141-2206（counterexample gate 2154-2185、final assistant push 2192-2201、`break` 2205）；post-tool-batch 注入区 2920-3050（`pending_policy` 2950、`[本轮编辑]` 2969、orientation gap 2989-3001、console 发放 3010-3021）；轮尾 3099 | **B1/B2 投递与 M1 注入的落点** |
| 8 | `crates/orz-loop/src/host_exec.rs:3509-3624` | 到达面写点（已提交）：`dispatched` 3537-3547、段 / `result_delivered(suppressed=model_read_directly)` / `finished` 3571-3611、`failed` 3613-3624 | 已落地面的边界与既有口径 |
| 9 | `crates/orz-loop/src/retrieval/dispatch.rs`（结构扫描） | `run_retrieval_subagent` 37、墙钟收口 425-500、收口分支 776-800 / 847-866、`close_activation` 调用点 779/795/865、`subagent_timeout` 映射 862 | ⑥ 的落点与 reason 口径 |

**结论**：本阶段**未对任何生产文件做新增编辑**——工作区的 362 行是**更早一轮**留下的（见 §3.2）。

---

## 3. 已落地 / 未落地的精确清单

### 3.1 已提交（orz 分支 `feat/fusion-architecture`）

- `f03b2a4f` **feat(0ac S3-b): immediate-feedback arrival/delivery write points**——新增 `crates/orz-loop/src/immediate_delivery.rs`（366 行）、`host_exec.rs` 派发前 / 返回两处接线、`orz-assurance` `event.rs` 三 `EventType` 变体（`retrieval_progress` / `retrieval_result_segment` / `result_delivered`）。
  - 口径（沿用索引 v3.21 记载）：开关 `ORZ_IMMEDIATE_RESULT_DELIVERY` **默认关**（关时零事件、零行为变化）；5 单测绿 + 三 schema jsonschema PASS；G3「零产品码写点」就此解除。
  - 本轮**未复跑**该件的编译 / 测试（按协调指令）。
- 更早同批：`4c892951`（检索侧 ①-a：本地分段检索前端、双钟截止、cause 自描述、检索族探针）、`96d2b263`（审记 G1/G2 修复）、`ac5d6375`（法官面 `immediate_feedback` 规则）、`dbb42b1d`（0.5.1 源冻结基线）。

### 3.2 工作区未提交（**本批最重要的隐藏状态**）

`git diff --stat`（orz，本轮实测读数）：**2 files changed, 362 insertions(+), 0 deletions(-)**

| 文件 | 增量 | 内容 |
|---|---|---|
| `crates/orz-loop/src/host.rs` | +28 | 新增 `CompletedTaskFact{ task_id, report, exit_code }` + `LoopHost::drain_completed_tasks()`（async，**默认空实现** `Vec::new()`，子代理/测试宿主零成本） |
| `crates/orz-loop/src/immediate_delivery.rs` | +334 | **M2**：`DeliveryQueue`（`new/from_env/admit/due/close_drop`）、`fact_dedupe_key`、TTL / 轮数兜底降级 digest（`ttl_ms_from_env` / `max_rounds_from_env`）、边界常量 `BOUNDARY_B1/B2`、方式 `MODE_DIRECT/DIGEST`、分级 `CLASS_I1/I3`、来源 `SOURCE_BACKGROUND_TASK`；**M3**：`STAGE_PROGRESS` + `progress_tick_payload`；**M1**：`M1_SWITCH_ENV` + `m1_enabled()`（从属主开关）；真投递 payload `delivered_fact_payload`（`suppressed=false` 且不带 `suppressed_reason`）；文本拼接 `render_delivery_message`；**6 条单测**（队列去重 / 逾期降级 / 关闭即删 / 真投递形态 / M3 读数形态 / M1 开关从属） |

- **核证状态：未编译、未测试**（本阶段未跑 `cargo test|fmt|clippy`；`f03b2a4f` 及 96d2b263 的既有绿读数**不覆盖**这 362 行）。
- **纪律**：恢复实施时**先** `git diff` 认领现场；**不得**用 `checkout` / `stash` / `clean` 清掉它（这是本批唯一在手的实现资产）。

### 3.3 父仓工作区（同批快照）

- `M orz`（子模块指针未动）；`?? docs/RETRIEVAL_LOCAL_SEGMENTED_HARDENING_DESIGN_2026-09-15.md`（**另一批**的检索侧补强设计稿；2026-09-15 已定稿 v1.0 并完成登记——索引 `DESIGN-RETRIEVAL-LOCAL-SEGMENTED-HARDENING` / BACKLOG 0ac「检索侧补强设计定稿与裁决」/ TODO `P0-0ac` 补强项；**与本件 ①-b 无直接关系，勿与本批混读**）。

---

## 4. 剩余件与建议落点（④ 组）

> 出口条件（TODO 拆分裁决原文）：**①-b 落码 + ③ 跨 run 时序钉子 + 门禁/镜像全绿**；之后载体重建（0.5.1 冻结基线）与 S4 实机复验**另行放行**。

### ⑤-A M2 投递接线（最大件；建议先落）

1. **宿侧来源**：实现 `OrzHost::drain_completed_tasks()`（`crates/orz-host/src/lib.rs`，`impl LoopHost for OrzHost` 区 ~1732-1872，样板：`drain_terminal_idle_kills` 1833）→ 内部经 `orz-tools` 的 `ToolBridge::drain_between_turn_bash_completions`（`bridge.rs:648`）取值，文本用既有 `While you were idle…` 批式格式（`task_completion.rs:~495-512`，函数名**待核**）。
   - 必须**共用** `ReportedTaskCompletions` 记账（否则与 `TaskCompletionReminder` 双投或零投——TB40 §10.1「注入 0 次」的成因，设计 §4.2 明写）。
2. **loop 侧边界投递（B1）**：`agent_loop.rs` post-tool-batch 区（2920-3050，与 `pending_policy` / `[本轮编辑]` 同间隙、**必须排在全部 tool replies 之后**）——drain → `DeliveryQueue::admit` → `due(round, now_ms)` → 注入一条中性事实消息 + 落 `result_delivered{boundary:"B1_tool_result", suppressed:false, delivery_mode, delivery_class, latency_ms, dedupe_key}`。
3. **per-run 队列**：队列建在 `run_agent_loop` 内部（run 生命周期 = 队列生命周期），run 尾（~3099-3102）`close_drop()` 并留痕；**不跨 run 存活**（设计 §4.2）。
4. 注入位置纪律：不得插进 assistant 声明与 tool replies 之间（`agent_loop.rs:2920-2927` 注释的 2026-08-07 review P1/P2 协议纪律）。

### ⑤-B M1 收尾注入接线（带开关 + A/B）

- 落点：终答候选分支 `agent_loop.rs:2141-2206`。语义（库侧已定）：本轮已是纯文本终答候选而队列仍有未投递事实时，**注入一次**（先保留该轮 assistant 文本含 `reasoning_content`，再注入机械事实消息，`continue` 续跑），**每 run 至多一次**，用尽即按原路径 `break`（绝不挂死、绝不重复注入）。
- 开关：`ORZ_IMMEDIATE_RESULT_DELIVERY_M1`（从属主开关，默认关）——A/B 面。
- 上行依据：S1 探针②「部分 assistant + reasoning + 注入事实」续写 3/3 通过（`0AC_S1_PROBE_RECORD_2026-09-13`）。

### ⑤-C M3 中途回报接线（顺带吃下 ⑦ 的可见性）

- 库侧 `progress_tick_payload` 已备；落点 = 工具等待窗（`host_exec.rs:3550-3565` 的 `tokio::select!` + 60 s 心跳循环）——在检索族调用上按 tick 落 `retrieval_progress{stage:"progress", waited_ms}`，使「到点前零事件」不存在。
- **⑦ 现状更正（见 §7 摩擦 A）**：acquire 独立截止**已落**（`crates/orz-host/src/tools.rs:46-77` `retrieval_lane_wait_budget`；`crates/orz-host/src/lib.rs:1476-1532` 有界 acquire + `retrieval_lane_busy` 自描述 cause）。⑦ 只差「**排队即时回报**」一件——即上条 tick 覆盖的窗口。

### ⑥ 检索子代理提前收口

- 落点：`crates/orz-loop/src/retrieval/dispatch.rs` 子代理循环（收口分支 776-800 / 847-866；`close_activation` 779/795/865；墙钟 425-500）。
- 语义：确定性不可达（`capability_unreachable`）/ 连续确定失败 / 结果已形成 ⇒ **立即 close 并回传**；墙钟（240/600/900 s 档）只作**最后兜底**（设计 §7 风险 6）。
- 先决：新的收口 reason 是否受契约面约束（见 §5-D）。

### ③ 跨 run 时序钉子（回归钉子）

建议钉子集（对应设计 §10.3-2 与 §4.2）：投递队列**不跨 run**（`close_drop` 后 `due` 恒空）；同一 `dedupe_key` **每 run 唯一**（同一事实只投一次）；`result_delivered` 与**真实投递**一一对应（不许「投了但被抑制」混记）；首个结果 `wall_ms ≤ deadline_ms` 的判据族；`subagent_wallclock_timeout_mid_tool` 计数为 0。

---

## 5. 动手前**必须先核**的检查点（皆为只读核对，未核项不得当结论用）

- **A. schema 闭枚举逐字对齐**：`runtime/retrieval-progress-event-payload-v0.2.schema.json` 的 `stage` 是否**含** `progress`；`runtime/result-delivered-event-payload-v0.2.schema.json` 的 `boundary` / `delivery_mode` / `delivery_class` / `result_source` 枚举字面值是否与 `immediate_delivery.rs` 新增常量（`B1_tool_result` / `B2_turn_end` / `direct` / `digest` / `I1_immediate_material` / `I3_deferred_digest` / `background_task`）**逐字**一致。对不上时以 **schema（S2 已放行契约）为准**改常量，并在报告写明取舍。**风险等级：不核就可能落出一个被 schema 拒的事件。**
- **B. 注入间隙**：新增投递消息与既有注入块（`pending_policy` / `[本轮编辑]` / orientation / console 发放 / counterexample gate）的**顺序与去重**是否会互相顶掉（同一间隙多处注入的历史纪律见 `agent_loop.rs:2920-2947`）。
- **C. 宿侧取 `ToolBridge` 的路径**：`ToolsetRegistry.toolset` 为私有字段；`ToolBridge` 由 `Arc<FinalizedToolset>` + `terminal` 构造（`orz-tools/src/bridge.rs:690 for_test` 形态）。⇒ 要么给宿加 accessor，要么在 `OrzHost` 内持 bridge 字段；**未定论**，决定 drain 实现路径。
- **D. 子代理收口 reason 的契约面**：`retrieval_close_record` 的 reason 闭枚举 / verifier 是否允许新值（`dispatch.rs:862` 已有 `"subagent_timeout"` 映射先例）。新增 reason 需同步契约面；否则用既有值承载、语义交给自描述 cause。
- **E. 投递文本的单一源**：`task_completion.rs` 中批式格式化函数名与形态（约 495-512）**待核**；纪律是「来源侧格式化、loop 只拼接不重写」。

---

## 6. 边界与纪律（不得越线）

- 开关纪律：主开关 `ORZ_IMMEDIATE_RESULT_DELIVERY` 默认**关**（关时零事件、零行为变化）；M1 子开关从属主开关。
- 不猜码：失败面只在错误文本自带稳定码时落账（`stable_code_from_error`）；稳定码闭枚举 = `capability_unreachable` / `network_no_response` / `network_error` / `empty_result` / `no_progress`。
- 抑制语义：真投递 `suppressed=false` 且**不得**携带 `suppressed_reason`；「模型自己读到」与「框架投递」分开记账（设计 §4.2）。
- 不新增常驻块；投递一律事件驱动、可去重、可审计（§2.3）。
- 不改 **FP-2**、不改官方口径、不新增容器内浏览器、不重建、不重跑（设计 §8）。
- 判据：检索类**首个结果** `wall_ms` p99 ≤ 10 s；`subagent_wallclock_timeout_mid_tool` = 0。

---

## 7. 框架内部摩擦记录（本阶段观察，候选；编号待批）

> 口径：按 2026-09-14 v3.23 收窄后的台账口径，只记 **orz 框架内部**情况；环境侧不入账。

- **摩擦 A（记账面—代码事实漂移，等级：中）**：索引 v3.21 / TODO 将 ⑦ 记为「未落」，但 **acquire 独立截止已随 `4c892951` 落码**——`crates/orz-host/src/tools.rs:46-77`（`retrieval_lane_wait_budget`，`ORZ_RETRIEVAL_SEMAPHORE_WAIT_MS` 默认 10 000 ms、`0`=禁用）与 `crates/orz-host/src/lib.rs:1476-1532`（有界 acquire + 自描述 cause `retrieval_lane_busy` + details）。**实际只差「排队即时回报」**。⇒ 建议下一批把 ⑦ 状态行勘误为「部分落地（缺排队可见性）」。危害：按「未落」再实现一遍会造成重复劳动。
- **摩擦 B（未提交工作区 = 账本看不见的状态，等级：中高）**：362 行 M2/M1/M3 库侧件在工作区、无提交、无本阶段核证；账本口径一律「未落」。若下一批不先 `git diff` 认领现场，极易**重写覆盖**或**误判为已落**。⇒ 本条同时是交接的**首要提示**。
- **摩擦 C（常量先行 / schema 后核的静默风险，等级：待核）**：`immediate_delivery.rs` 新增的 stage/boundary/mode/class/source 常量与 S2 契约枚举的**逐字一致性尚未核**（§5-A）。这类「实现侧先写常量、契约枚举不参与编译期校验」的形态，本身是**事件面静默失配**的温床（事件写出去才被发现）。⇒ 建议：核名动作写进恢复点第一步。
- **摩擦 D（默认空的 trait 方法的「静默零投递」，等级：低—中）**：`LoopHost::drain_completed_tasks()` 默认返回空（设计上利于子代理/测试宿主零成本），但宿主**忘记实现**时的症状是「投递恒零且无任何事件提示」——与 P1-1「忘了接线不得静默放行」同形。⇒ 建议在 `OrzHost` 实现处配钉子测试，或在 run 起始探针面留一条只读事实（不新增常驻块）。
- **环境侧（按口径不入账，仅记一句）**：本会话 Windows shell 下 `rg` 通配（`crates/…/*.rs`）报 os error 123、`awk`/`grep`/`sed` 不可用，需改用 `--glob` 与 PowerShell 等价写法——属宿主工具形态，不进框架账。

---

## 8. 恢复点（下一位接手者的 first action）

1. `cd D:\CLI\orz && git diff` —— **认领 362 行未提交现场**（`host.rs` / `immediate_delivery.rs`），决定「续写」而非重写。
2. 核 §5-A / §5-C / §5-D / §5-E（只读）。
3. 按 §4 顺序落码：**⑤-A M2 接线 → ⑤-C M3 tick（吞下 ⑦ 可见性）→ ⑤-B M1 → ⑥ → ③ 钉子**。
4. 门禁与镜像：`cargo fmt --check`、`cargo clippy`、`cargo test`（重点 `orz-loop --lib` / `orz-host --lib` / `orz-tools --lib` / `orz-assurance --lib`）+ `python scripts/check_repository.py`（判据：`valid: true` / `error_count: 0`）。
5. 出口条件复核：①-b 落码 + ③ 跨 run 时序钉 + 门禁/镜像全绿 ⇒ 该批方可报 S3 出口；**载体重建与 S4 实机复验另行放行**。

---

## 9. 本件未做（按协调指令，勿视为遗漏）

- 未跑任何构建 / 测试 / 格式化 / clippy；未新增或修改任何生产代码；未提交、未推送、未重建、未清理工作区。
- 未同步索引 / BACKLOG / TODO 的状态行（无提交面）；上文 §7 摩擦 A 的**勘误建议**留待下一批随提交一起落。
