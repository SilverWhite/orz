# 0t S2-R P1–P3 实现全面复核（2026-09-09）

> **范围**：0t 检索子代理双车道 S2-R 已实施部分——P1（设计定稿轮：
> browser_control 工具面与特征回传 / P1-2a 启动事实口径 / P2-2 web_fetch
> 恢复）→ P2（正确性批：P1-1 生命周期 + P1-2a 接缝）→ P3（语义/声明批：
> P2-1 并发懒启动 + P2-2 web_fetch 声明 + P2-3 registry 声明语义 +
> P1-2b browser_control 实现）——对照 P1 设计定稿 / v1.3 / ADR-0010
> §14.65 的三面复核（设计合理性 / 实现合理性 / 设计与实现符合性）。
> **证据基线**：审查当日工作树（orz 子模块 + 父仓均未提交；P1–P4 改动
> 混存；P4 卫生批不在本复核对象内，仅作为当前树状态参与证据读取）。
> **方法**：本复核按用户指示尝试三路子代理并行深审，但本会话子代理
> 消息通道故障（三次派发/唤醒，任务正文均未送达子代理，子代理回复
> 「无任务内容」），故由父级在同一轮内按三线分工完成——设计文档
> 对照、实现代码逐项核验、逐条符合性矩阵、定点测试佐证。父级既有的
> S2 Task 1 三路子代理结论（`0T_S2_TASK1_REVIEW_HANDLING_2026-09-09.md`）
> 作为前序基线引用，不重复核验其已闭合内容。
> **结论**：无 P0/P1。发现 P2×1（envelope 键名与设计/描述不符）+
> P3×3（P2 验收全序断言缺口、P2-1 单次启动可观测断言缺口、P1 设计文档
> 状态头过时）+ P3 注记×2（§2.3 串行化措辞待同步、ADR §14.65 待 P7
> 转录 P1 设计新增项）。
> **本轮未改任何实现文件**；除本审查文档外未动其它文档（TODO/BACKLOG
> 排期登记待用户放行后按纪律执行）。

## 1. 复核基线文档

- P1 设计定稿：
  [`RETRIEVAL_BROWSER_LANE_P1_DESIGN_2026-09-09`](../RETRIEVAL_BROWSER_LANE_P1_DESIGN_2026-09-09.md)
  （设计 A/B/C、§6 实施归属与验收）
- 设计权威 v1.3：
  [`RETRIEVAL_SUBAGENT_DUAL_LANE_DESIGN_2026-09-09`](../RETRIEVAL_SUBAGENT_DUAL_LANE_DESIGN_2026-09-09.md)
  （§3.1–3.5 / §4 / §5）
- ADR-0010 §14.65（S1 转录原文，第 4837 行起）
- 排期/验收：`0T_S2_TASK1_REVIEW_HANDLING_2026-09-09.md` §5 阶段表

## 2. 总体判定

P1/P2/P3 三阶段主链路与设计定稿一致、可交付性成立：

- P1-1 跨 prompt 生命周期修复到位（run 前不再预存 unavailable 句柄、
  run 收尾 `fold_back_browser` 仅 ready 回写、`close_session` 持真实句柄
  触发 shutdown），并有对应 ACP 单测覆盖；
- P1-2a 接缝（`BrowserStepFailed{reason, launch_fact}` + 三臂装配 +
  Err 臂先落 fact 再 ToolCompleted + 稳定码映射）实现正确，S1/S2/S3/S4
  路径均有测试；
- P1-2b browser_control 全链（ToolDef / 六动作执行 / 会话级控制 tab /
  有界日志特征回传 / 同 URL 免重复导航 / 声明与投影 / fail-closed）整体
  与设计相符，单元与 stub 测试全绿，live e2e 此前已在真实 Chrome 通过；
- P2-1 双重检查锁定结构正确；P2-2/P2-3 声明面与 registry 语义已对齐 0t。

发现集中在**契约一致性与验收断言完整性**，无生产路径阻断。

## 3. 发现总表

| 编号 | 严重度 | 主题 | 证据（审查当日工作树） | 处置建议 |
|---|---|---|---|---|
| F1 | P2 | browser_control 每动作信封键名 = `status`，与设计 §2.2 / 工具描述 `action_status` 不符 | 设计 §2.2 字段表；orz-host local_browser/mod.rs:642（描述）、:800（`json!({"status": …})`）、:1517（测试注释自述 action_status）；测试 :1542/:1574 断言 `parsed["status"]` | 改实现：输出键 `action_status` + 测试同步（低风险，模型面契约） |
| F2 | P3 | P1 设计 §6 P2 验收「ToolStarted→fact→ToolCompleted 全序断言」未闭合：host_exec 三个 launch 事实测试只断言 fact < ToolCompleted | host_exec.rs:6489/6549/6631；断言仅 `fact_index < completed_index`（:6539/:6608/:6696），无 started_index 断言 | 三个测试补 ToolStarted 早于 fact 断言 |
| F3 | P3 | P2-1「同一 profile 只拉起一次」无可观测断言——现有并发测试为无死锁烟雾 + ready 并发成功；注释自认依赖 probe seam 未落地 | lib.rs:1589–1593 `concurrent_browser_read_lazy_launch_does_not_deadlock` | 下批补启动计数 seam 单测，或按注记收口为「结构正确性由串行锁+二次检查保证」并在 P9 实机覆盖 |
| F4 | P3 | P1 设计文档状态头仍为 `pending-design-release`，与实际已放行并实施 P2–P4 不符 | P1 设计文档行 4–5 | 更新状态头或并入 P7 收口登记 |
| F5 | P3 注记 | 设计 §2.3 字面「browser_read / browser_control 由 host 侧会话级动作队列串行（互斥执行）」与实现（control 在 CDP 会话层单控制 tab 串行；browser_read 池读可并发；同 URL 控制 tab 读取为尽力 try_lock）及用户已接受的取舍不一致 | P1 设计 §2.3；cdp.rs:389（`control` Mutex）、:500–540（read_page/try_control_tab_read） | 设计注记同步（无代码改动，除非用户改裁决为 host 侧全互斥） |
| F6 | P3 注记 | ADR §14.65 尚未转录 P1 设计新增项（browser_control 加入外部 lane、web_fetch 恢复、P1-2a BrowserStepFailed/S3 语义） | ADR-0010 §14.65 全文（仅覆盖基础双车道） | 归 P7 ADR 注记同步（既有排期项），本批登记确保不遗漏 |

## 4. 详细发现

### F1（P2）browser_control 信封键名与设计/描述不一致

设计 §2.2 字段表与 `browser_control` 工具描述均为 `action_status`
（`ok`/`error`）；`BrowserControlOutcome` 结构字段也是 `action_status`
（mod.rs:97），但输出装配 `json!({…, "status": outcome.action_status, …})`
（mod.rs:800）实际发给模型的键是 `status`。测试注释自称「字段齐全
（action_status/…）」（:1517），断言却是 `parsed["status"]`（:1542/:1574），
说明测试固化了实现形态而非设计契约。

影响：该输出是纯文本 JSON 信封，无结构化/机械消费方，因此不破坏
journal/ledger；但模型按工具描述与设计预期读 `action_status` 会拿不到
字段，属于「工具契约描述↔实际返回」偏差，正是本设计要消除的低信息量
摩擦。修复方向 = 改实现键名为 `action_status`（描述与设计已正确，测试
同步），改动小、无下游。

### F2（P3）P2 验收「ToolStarted→fact→ToolCompleted 全序断言」未闭合

`host_exec.rs` 三个事实测试（:6489 success、:6549 failure、:6631
step-failed）都只计算 `fact_index < completed_index`，未断言 ToolStarted
早于 fact。事件装配点（Ok 臂 :3168、Err 臂 :3456 在 ToolStarted 记录之后）
实际顺序正确，但测试不防回归——若未来 fact 落在 ToolStarted 之前，本组
测试仍绿。P1 设计 §6 明确 P2 验收含「ToolStarted→fact→ToolCompleted
全序断言」，属验收缺口。

### F3（P3）P2-1 单次启动无可观测断言

`ensure_browser_launched` 双重检查锁定结构正确（lib.rs:277–305：锁内
`!ready()` 才 probe_launch、成功后换入、等待者锁后二次检查直接跳过），
但 `concurrent_browser_read_lazy_launch_does_not_deadlock`
（lib.rs:1593）只验证「双双失败（找不到浏览器）/ ready 后并发成功」；
注释（:1589）自认「同一 profile 只拉起一次」断言依赖 probe seam，未
落地。属覆盖缺口，非生产缺陷。

### F4（P3）P1 设计文档状态头过时

P1 设计定稿文档头部仍写 `pending-design-release`（待用户复核放行后才
进入 P2/P3 实施），但该设计已获放行且 P2/P3/P4 已实施。doc-conformance
偏差，建议状态头更新为 `released`/`implemented`（或注明放行时间与阶段
落地），可随 P7 收口一并处理。

### F5（P3 注记）串行化措辞与已接受取舍不一致

设计 §2.3 写「同一会话的 browser_read / browser_control 动作由 host 侧
会话级动作队列串行（互斥执行）」；实现把 browser_control 动作串行化在
CDP 会话层单个控制 tab（`control: tokio::sync::Mutex<Option<ControlTab>>`，
动作全程持锁），browser_read 池化读可与控制动作并发，同 URL 控制 tab
复用读取以 `try_lock` 尽力而为（不等待、不互斥）。该取舍在 P3 交付时已
向用户说明并被接受。为维持「文档权威 = 实现口径」，建议在 P1 设计 §2.3
补注记：host 侧动作队列语义 = 控制动作串行 + 控制/读取尽力不互斥
（或按用户未来裁决改 host 侧全互斥）。

### F6（P3 注记）ADR §14.65 转录待补

ADR §14.65 目前只转录了基础双车道（browser_read + web_search/web_fetch、
静态标注、γ 退役、启用门、browser_launch_result 义务）。P1 设计新增的
`browser_control`（外部 lane 第三工具）、P1-2a `BrowserStepFailed`/S3
口径、P2-2 web_fetch 声明恢复尚未转录进 ADR。该同步在 S2-R 阶段表 P7
（ADR 注记同步）已排期，本复核登记为 P7 必含项，防止遗漏。

## 5. 符合性矩阵（要求 → 判定）

| P1 设计要求 | 实现证据 | 判定 |
|---|---|---|
| §2.1 动作集 = navigate/back/forward/refresh/wait_load/snapshot；无正文读取动作 | mod.rs `BrowserControlAction`（76–92）、tool_def enum（652–667）；read_content 不存在 | 符合 |
| §2.1 browser_read 同 URL 免重复导航、url 仍必填、透明 | cdp.rs read_page（510–522）/ try_control_tab_read（530–540）；mod.rs url 校验 | 符合 |
| §2.2 每动作统一信封五字段 + 有界日志 | mod.rs:794–812（键名偏差见 F1）；bounded_browser_log（839–872）≤12 行/≤2KiB + `[browser_log]` + 脱敏 | 部分符合（F1） |
| §2.2 日志只进工具结果，不进事件/黑板/主面 | `structured: None`；journal 面无新字段 | 符合 |
| §2.3 控制动作串行、懒启动前置、导航失败普通回传 | cdp.rs:389 control Mutex 全程持锁；lib.rs ensure_browser_launched；FP-2 错误文案 | 部分符合（措辞见 F5；行为用户已接受） |
| §2.4 启用会话恒声明 browser_read+browser_control；主面/内部 lane 不声明；未启用全无；relay=Host + gate + 不计候选 | tools.rs 431–505；projection.rs R1_SEALED 33–47 / 194–215 / 测试 562–700；relay.rs 70–110；`is_candidate_counted_tool` 不含 browser_control | 符合 |
| §2.4 静态标注两族固定文本、只落工具描述+一次性提示、无新常驻 token | projection.rs 207–216 标注注入；prompt.rs 404–430 既有 ≤1 句推荐序未新增 browser_control 句 | 符合 |
| §3.1–3.2 S1–S4 事实口径与解耦 | host.rs 169–215；lib.rs 559–580；call_tool 635–645；host_exec 3445–3463 | 符合 |
| §3.3 接缝/错误码/Display/0q | host_exec 126–135（BrowserStepFailed → CODE_EXECUTION_FAILED）、3544–3565（tool_error_kind=ExecutionFailed、Display）；families 规则不变 | 符合 |
| §3.3 事件顺序 ToolStarted→fact→ToolCompleted | Ok 臂 3168、Err 臂 3456 装配点正确 | 符合（测试全序断言缺口见 F2） |
| §4 web_fetch 恢复 + 标注 + 计数不变 | projection.rs 194–215；测试 562–600 断言 web_fetch 在 lane 且带 `[车道:原生检索]` | 符合 |
| §5 无新事件类型/payload；回退口径保留 | 事件面无改动；工具描述含降级回只读面可执行 | 符合 |
| §6 P2/P3 验收对应测试 | 见 §7 验证证据 | 部分符合（F2/F3） |
| 文档状态头与进度一致 | P1 设计头部 | 不符合（F4） |

## 6. 已确认符合/实现正确项（避免误伤）

- 授权门 fail-closed：`browser_declared` 位默认 false；未启用会话外部
  lane 全无检索工具（投影测试断言仅 `read_file` 残留）；启用门置位后
  browser_read/browser_control 恒声明，句柄注入不翻转声明位
  （lib.rs:1473–1510 测试）。
- 双车道声明面：external lane = browser_read + browser_control +
  web_search + web_fetch（projection 测试全量成员断言）；内部 lane 仅读
  族（browser_control/browser_read/web 均剔除）；主面封存
  browser_read/browser_control。
- 标注：`[车道:本地浏览器检索|推荐首选]`（browser_read/browser_control）
  与 `[车道:原生检索]`（web_search/web_fetch）逐字与 v1.3 §3.2 一致，
  只落工具描述 + 一次性提示。
- P1-1：`prompt_does_not_seed_unavailable_browser_handle` /
  `fold_back_browser_only_overwrites_with_ready_handle` /
  `browser_handle_survives_prompt_boundary_and_close_shuts_down_once`
  三测试覆盖 run 前不预存、ready 才回写、跨 prompt 同 Arc、close 才
  shutdown。
- P1-2a：`finish_browser_call_attaches_launch_fact_on_success_or_step_
  failure` 覆盖 S2/S3/S4 三臂；host_exec failure 测试断言稳定码
  `browser_launch_failed` + failure cause；step-failed 测试断言
  ExecutionFailed 系 + failure_target 盖章 + fact 早于 ToolCompleted。
- P2-2/P2-3 registry 语义：测试 `browser_read_declaration_follows_
  enable_gate_bit`、`browser_session_injection_does_not_flip_declaration`
  与 0t「enabled 恒声明、readiness 不裁剪」一致。
- P1-2b 参数校验严格（缺 action/url、非 navigate 带 url、非法 timeout、
  未知参数均显式拒绝，无静默容错）；动作集与 live e2e 一致；CDP 方法
  面保持只读/机械固定（allowed_method_set 测试、无任意 JS eval）。
- 事实事件面未新增事件类型/payload；浏览器日志特征不进 journal。

## 7. 验证证据（审查当日定点实测，均绿）

| 命令（D:\CLI\orz 下） | 结果 |
|---|---|
| `cargo test -p orz-host browser_handle_survives_prompt_boundary_and_close_shuts_down_once` | 1 ok |
| `cargo test -p orz-loop browser_launch` | 2 ok |
| `cargo test -p orz-loop browser_step_failed` | 1 ok |
| `cargo test -p orz-host finish_browser_call` | 1 ok |
| `cargo test -p orz-loop --lib projection` | 15 ok |
| `cargo test -p orz-host browser` | 74 ok + 4 ignored（live e2e，env-gated） |

live e2e（`browser_control_live_e2e` / `local_browser_e2e`）此前已在真实
Chrome 通过（P3 交付记录），本轮不重跑（env-gated）。

## 8. 建议处置批次（待用户放行，不预支）

> 原则：先裁决 F1 修复方向（建议改实现键名）；F2/F3 为测试补强；F4/F5/F6
> 为文档/登记批次。处置后可衔接 P5（conformance 正反例）与 P6/P7 原排期。
> （本节为处置前建议，保留作审查记录；执行结果与验证见 §10。）

| 批 | 内容 | 出口 |
|---|---|---|
| X1 | F1：mod.rs 输出键 `action_status` + 三处测试断言同步 + browser_control 测试绿 | browser_control 单测组绿 |
| X2 | F2：host_exec 三测试补 ToolStarted 早于 fact 断言 | browser_launch/browser_step_failed 组绿 |
| X3 | F3：补启动计数 seam 单测（或按注记收口、P9 实机覆盖，裁决后定） | 并发懒启动测试绿 |
| X4 | F4/F5/F6：P1 设计状态头更新；§2.3 串行化措辞注记；ADR §14.65 转录登记（可与 P7 合并） | 文档一致 |

建议 F1/F2/F3 作为一个小修复批先做（1 轮），F4/F5/F6 随 P7 收口；P5 场景
编写时同时固化「action_status 键名」与「ToolStarted→fact→ToolCompleted
全序」两条不变量，防止再漂移。

## 9. 收口注记

- 本轮除本审查文档外未改任何实现/文档文件；未做 git 写操作。
- 子代理消息通道故障已在 §方法 登记；如需恢复三线并行，可在后续轮次
  重试派发（本次故障与任务内容大小无关，短消息同样未送达）。
- P4 卫生批不在本复核对象；其完成度沿用 P4 交付记录（fmt/check/测试
  绿），本复核读取当前工作树时未发现 P4 改动影响 F1–F6 结论。

## 10. 处置记录（X1–X4；2026-09-09 用户放行后执行）

> 本轮处置只改审查发现的 X1–X4 对应面；未做 git 写操作/commit；工作树
> 仍为 P1–P4 + X1–X4 混存未提交。

| 处置 | 内容 | 证据/验证 |
|---|---|---|
| X1（F1） | `browser_control` 输出键 `status` → `action_status`，三处测试断言同步 | orz-host local_browser/mod.rs 输出装配与 :1542/:1574；lib.rs :1704；browser_control 单测绿 |
| X2（F2） | host_exec 三测试补 ToolStarted 早于 fact 断言 | host_exec.rs :6531 起三处 `started_index < fact_index < completed_index`；browser_launch 2 + browser_step_failed 1 绿 |
| X3（F3） | 新增 `ensure_browser_launched_with` 启动 seam + `concurrent_first_launch_invokes_probe_once` 计数断言 | lib.rs（并发首调计数 = 1、ready 并发 = 0）；单测绿 |
| X4（F4/F5/F6） | P1 设计状态头更新为 `released` + §2.3 串行化注记同步；ADR §14.65 转录 P1 新增项登记为 P7 必含项（TODO P0-0t 注记） | 本文档 F4/F5/F6 行 |

验证：`cargo fmt --all --check` exit 0；`cargo check -p orz-host -p orz-loop
--tests` 零警告；`cargo test -p orz-host browser` 74 ok + 4 ignored
（live e2e，env-gated）；`concurrent_first_launch_invokes_probe_once` ok；
`cargo test -p orz-loop browser_launch` 2 ok；`browser_step_failed` 1 ok。

复核结论：F1–F6 全部按建议处置闭合；X4 中 ADR §14.65 补充转录本身仍属
P7 收口排期项（本次只完成登记），不提前展开。

## 11. P5 处置记录（S2-T2 conformance 正反例；2026-09-09 用户放行后执行）

> 入口：P1 设计 §6 P5 / 审查处理 §5 P5 阶段表。出口 = Rust↔Python 对拍绿。

**语义补全（双侧对拍前提）**：0t S2-R P3 / P1-2b 新增的 `browser_control`
此前未进入两侧 conformance 法官的工具表——启用门族（`retrieval_enable_gate`
声明面）与 launch 义务（`browser_launch_result`）判定不到 host 车道的
`browser_control`。本批补入：

- Rust `families.rs` toolsets：`RETRIEVAL_MODE_GATED_TOOLS` /
  `HOST_LANE_RETRIEVAL_TOOLS` 增 `browser_control`；
  `verify_browser_launch_result` 由硬编码 `browser_read` 改为按
  `HOST_LANE_RETRIEVAL_TOOLS` 判定。
- Python `run_event_journal_validation.py`：`_HOST_LANE_RETRIEVAL_TOOLS` /
  `_RETRIEVAL_MODE_GATED_TOOLS` 增 `browser_control`；
  `_verify_v02_browser_launch_result` 同步按 host-lane 集合判定。

**场景补全（families.rs 共享语料）**：新增 11 个场景，语料计数 233 → 244
（钉死断言同步更新）：

| 场景 | 覆盖 | 期望违反族 |
|---|---|---|
| enable_gate_declared_dual_lane_ok | 声明面正例：header 声明双车道全族，web/browser_read/browser_control 调用合法（含 S2 success fact） | 无 |
| enable_gate_disabled_host_dispatch | gate 反例（host-lane）：未声明检索的会话出现 browser_control ToolStarted | retrieval_enable_gate |
| enable_gate_disabled_web_dispatch | gate 反例（dispatch-lane）：未声明检索的会话出现 web_search external ToolStarted | retrieval_enable_gate |
| browser_launch_s1_failure_fact_ok / _control_failure_fact_ok | S1 正例（browser_read / browser_control 均有前置 failure fact） | 无 |
| browser_launch_s1_missing_fact / _control_missing_fact / _late_fact | S1 反例：缺 fact / fact 晚于 ToolCompleted | browser_launch_result |
| browser_launch_s2_success_ok | S2 正例：success fact + ok 完成 | 无 |
| browser_launch_s3_step_failed_ok | S3 正例：success fact + 页面级失败（真实类别，不进义务） | 无 |
| browser_launch_s4_ready_no_fact_ok | S4 正例：已就绪调用无 fact | 无 |

**验证**：`family_verdicts_match_spec_table` 绿（244 场景 × 33 族逐格）；
`s2b_family_verdicts_match_python` 绿（Rust↔Python 对拍 0 差）；
orz-assurance lib 210 绿；Python run-event conformance 15 绿；
`cargo fmt --all --check` exit 0。未做 git 写操作。

P5 出口达成；P6（三个 `#[ignore]` conformance capture 重写）见 §12。

## 12. P6 处置记录（S2-T3 三个 conformance capture 重写；2026-09-09
用户放行后执行）

> 入口：审查处理 §5 P6 阶段表 / P1 设计 §3。出口 = 三个 capture 重放绿。
> 改动面：orz-bin main.rs conformance_capture 模块测试 8/9/10（场景名与
> fixture 名保留）；未做 git 写操作。

| 测试（场景名） | 原断言（退役语义） | 改写后断言（当前语义） |
|---|---|---|
| capture_mode_off_refusal | error `retrieval_mode_off`、target internal_retrieval | `retrieval_not_enabled` 拒绝：ToolCompleted(error) alone、无 ToolStarted、无 transition、无 policy_denial 信封；事件序补 `request_header_change` + 双 `mechanical_audit_update`（当前 v0.2 轨） |
| capture_local_browser_capability_error | bootstrap `retrieval_mode_transition` + `retrieval_capability_unavailable` 派发拒绝 | S1 启动失败：启用会话 web_search（web 族）与 browser_read（浏览器族）双 ToolStarted；`ToolStarted → browser_launch_result(failure, ORZ_BROWSER_PATH 真实 cause) → ToolCompleted(error browser_launch_failed)`；无 capability 预检/无 transition（hermetic：ORZ_BROWSER_PATH 指缺失路径，环境量 Drop 还原） |
| capture_local_browser_read | `retrieval_mode_transition` + `capability_status=available` | S4 已就绪成功读：双族 ToolStarted；ready 会话无启动尝试故无 `browser_launch_result`（S4 无 fact）；commit 证据断言保留（web_page full_text_observed）；全刊无 transition/capability_status |

**验证**：三个 capture 各自重放绿（13/25/24 事件，replay valid +
run_finished）；`cargo check -p orz-bin --tests` 零警告；`cargo fmt --all
--check` exit 0。

**发现（范围外，记录待裁决）**：全量 `--ignored conformance_capture` 套件
13 项中 5 项绿（本批 3 + plan/restore），其余 7 项（plain / cancelled /
failed / tool-snapshot / orientation-fire / real-doc-retrieval /
cross-prompt-restore）因 `request_header_change`（及工具轮后的
`mechanical_audit_update`）事件面加入而未随本批重捕——属 S2 Task 1 前
既有漂移，非退役语义改写范围；建议作为独立重捕小批（P7 收口前或并入
P7 排期裁决）。三个改写场景的已提交 fixture（
`runtime/fixtures/run-event-v0.2/journals/{mode-off-refusal,
local-browser-capability,local-browser-read}.jsonl`）仍为旧 capture 字节，
建议随上述重捕批一并换新。

**实机预检注记（2026-09-09，用户指示提前拉真实浏览器；沙箱外
`GSA_RUN_LIVE_BROWSER_TESTS=1`）**：真实 Chrome 153.0.8010.36 e2e 4 项中
3 项通过（拉起→读取 example.com / 导航控制 / 元数据 URL gate）；1 项失败：
`local_browser_e2e_pdf_download` 稳定返回 `EmptyContent`——w3.org dummy.pdf
从宿主机 curl 可达（HTTP 200、13,264 B），即排除网络；事件循环收到
`loadEventFired` 且无任何 `downloadWillBegin`（含 Browser.* 域），判定为
Chrome 153 对该 PDF URL 走内嵌查看器而非下载通道（无 download 事件产生）
--与 R3「可拉起但实质不可用」同族的下载通道候选缺陷。处置待用户裁决：
登记后随 P7 修复批次处理，或记入 P9 S4 判据（pdf 白名单域实测）后处置。

## 13. P7 PDF 下载修复处置记录（2026-09-09 用户放行后执行）

> 入口：用户「PDF 这点随 P7 一起修」，并询问可否捕捉查看器内部下载键或
> 直接关闭 PDF 查看器。出口 = `local_browser_e2e_pdf_download` 真机绿 +
> browser 模块全量回归绿。改动面：orz-host `local_browser/cdp.rs`
> `seed_pdf_download_preference`（路径层级）+ 两个单测；未做 git 写操作。

**取证结论（回答用户提问）**

1. Chrome 153（153.0.8010.36）顶层 `application/pdf` 导航必然被内嵌
   viewer 接管；viewer 是 OOPIF 架构——顶层 frame 停在 PDF URL（
   `document.contentType=application/pdf`、body 空），UI 在
   `chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/index.html`
   iframe target 的 shadow DOM（`pdf-viewer#viewer` →
   `VIEWER-TOOLBAR#toolbar`…），PDF 字节在另一 iframe 的
   `<embed type="application/x-google-chrome-pdf" src=
   "chrome-extension://…/<uuid>">` 扩展 blob 中。
2. 「捕捉查看器内部下载键」不可行/不采纳：需跨 OOPIF target（新 CDP
   attach 面）+ shadow DOM 深处合成交互，超出 orz 固定 EXPR 三件套的
   机械面；且 viewer DOM 为组件扩展内部结构，跨版本脆弱。与既有裁决
   （无 Input.*/任意 JS 面）一致，不扩展。
3. 「直接关闭 PDF 查看器」的正解是让导航不进 viewer。官方机制有二：
   a) profile 偏好 `plugins.always_open_pdf_externally=true`
   （chrome://settings/content/pdfDocuments 的开关）；
   b) 企业策略 `AlwaysOpenPdfExternally`（注册表，账户级副作用）。
   策略经 HKCU 短时写入实测有效（临时 profile 导航产生
   `Page.downloadWillBegin` + `downloadProgress`，文件落盘；测毕删除恢复
   原状态），但因属系统级副作用不作为 orz 默认路径。

**根因（写错层级）**：早期实现把偏好种子写入
`<profile_dir>/Preferences`（`--user-data-dir` 顶层）。实测 Chrome 从不
读取该文件——首启后顶层 `Preferences` 根本不存在（profile 真身在
`<profile_dir>/Default/Preferences`），种子「存活」只是因为 Chrome 从未
触碰。prefs 键与值本身在 Chrome 153 依然有效。

**修复**：`seed_pdf_download_preference` 改种子
`<profile_dir>/Default/Preferences`（create_dir_all `Default/`；merge +
idempotent 保留；launch 前冷启动预写）。CDP 面零新增（无
Input.*/Network.*读/Storage 写），`Browser.setDownloadBehavior` +
下载事件设计原样保留。

**验证（2026-09-09 真机，沙箱外）**

- probe：headed/headless 各配置在 Default/Preferences 种子下均触发
  `Page.downloadWillBegin` + `downloadProgress` + `dummy.pdf` 落盘；
  无种子对照仍 viewer 接管（EmptyContent 语义）。
- 单测：`pdf_preference_seed_writes_merges_and_is_idempotent`、
  `pdf_preference_seed_creates_fresh_profile` 绿。
- e2e：`local_browser_e2e_pdf_download` 绿；连同
  `local_browser_e2e` / `local_browser_e2e_gate_blocks_metadata_url` /
  `browser_control_live_e2e` 四个 live e2e 全绿。
- 回归：`cargo test -p orz-host local_browser::` 非忽略 63 项全绿；
  `cargo fmt --all --check` exit 0；`git diff --check` 干净。

**注记**：多内核/版本差异（Edge 148 同内核不同实现）留作后续观测；如
将来某 Chrome 版本连 profile 偏好也停用，可再评估策略通道（须先过设计
裁决：注册表写入属系统级副作用，建议仅显式配置开启或由管理员预置）。

## 14. P7 重捕小批处置记录（2026-09-09 用户放行后执行）

> 入口：§12 登记的重捕小批（7 项漂移 capture 重捕 + 3 个 P6 改写场景
> fixture 换新），用户「请进行P7剩余的重捕小批」放行。出口 = 12 个
> conformance capture 全绿 + Python run-event 对拍全绿。改动面：orz-bin
> main.rs（conformance_capture 7 个场景 expected/脚本）、
> `runtime/fixtures/run-event-v0.2/journals/` 10 个 fixture 换新、
> `runtime/tests/test_run_event_journal_validation.py`（EXPECTED_SEQUENCES_V02
> 同步 + assessment 断言更新）；未做 git 写操作。

**漂移形态（2026-09-09 取证）**：7 个场景因 S2 Task 1 起的事件面新增而
漂移——① 每次模型请求前落 `request_header_change`（plain / cancelled /
failed / tool-snapshot / real-doc / cross-prompt / orientation-fire 均受影响）；
② 工具轮后落 `mechanical_audit_update`（普通工具轮双条；disposition 工具
单条）；③ cross-prompt-restore 原 expected 的「disposition 后
tool_availability_check」被审计更新取代；④ orientation-fire 因轮次节奏
变化脚本耗尽（`fake script exhausted`），补足终答文本轮后以 run_finished
收束。

**改动明细**

| 场景 | Rust capture 改动 | 新事件数 | fixture |
|---|---|---|---|
| plain-run | expected + request_header_change | 9 | 换新 |
| cancelled-run | expected + request_header_change | 6 | 换新 |
| failed-run | expected + request_header_change | 6 | 换新 |
| tool-snapshot-run | expected + request_header_change + 双 audit | 17 | 换新 |
| real-doc-retrieval | expected + 双 request_header_change + 双 audit | 24 | 换新 |
| cross-prompt-restore | expected 同左（tool_availability_check → audit） | 16 | 换新 |
| orientation-fire-run | expected 循环化 + 脚本 +1 终答轮 | 81 | 换新 |
| mode-off-refusal | （P6 已改写） | 13 | 换新 |
| local-browser-capability | （P6 已改写） | 25 | 换新 |
| local-browser-read | （P6 已改写） | 24 | 换新 |

orientation-fire 的 expected 从手写 90 行改为与 Rust capture 同构的循环
构造（每轮模板 + 第 7 轮 checkpoint 夹双 audit 之间 + 尾部回答/反例门/
终答）；Python 侧 `EXPECTED_SEQUENCES_V02` 同步同构构造，避免两处巨型
手抄表再次漂移。Python `test_information_sufficiency_assessment_is_mechanical`
由 5 assessment（revision 0..4，旧 M4 disposition 语义）改为 7（R1
auto_close：revision 恒 0）。

**验证（2026-09-09）**

- `cargo test -p orz-bin -- --ignored conformance_capture`：12/12 绿
  （13/25/24/81 等事件数、replay valid + 正确 terminal）。
- `python -m pytest runtime/tests/test_run_event_journal_validation.py
  runtime/tests/test_run_event_conformance.py`：273 passed（哈希链 +
  schema + 序列 staleness + v0.2 生命周期全对拍）。
- `cargo fmt --all --check` exit 0；`git diff --check` 干净；fixture 行尾
  LF 与仓库规范一致。
- 旧 fixture 字节备份：`orz/target/conformance-journals/fixture-backup-
  20260909/`（10 个，gitignored，可恢复）。
