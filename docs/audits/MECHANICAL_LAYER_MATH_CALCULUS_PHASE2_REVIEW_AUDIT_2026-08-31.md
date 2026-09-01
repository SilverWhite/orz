# P2-10 阶段 2 I1–I6 全面审查记录（设计合理性 / 实现合理性 / 符合性）

> 日期：2026-08-31；范围：I1–I6（TODO P2-10 / BACKLOG 10）。
> 方式：三个并行子代理分片深查（I1+I2 / I3+I4 / I5+I6）+ 根代理跨切片核查
> 与 102-run 语料量化；本次为只读审查，未修改任何实现代码。
> 依据：正式设计
> [`MECHANICAL_LAYER_MATH_CALCULUS_DESIGN_2026-08-30.md`](../MECHANICAL_LAYER_MATH_CALCULUS_DESIGN_2026-08-30.md)
> （§1–§8）、ADR-0010 §14.47、实施记录
> [`MECHANICAL_LAYER_MATH_CALCULUS_I1_IMPL_AUDIT_2026-08-30.md`](MECHANICAL_LAYER_MATH_CALCULUS_I1_IMPL_AUDIT_2026-08-30.md)
> 与
> [`MECHANICAL_LAYER_MATH_CALCULUS_PHASE2_IMPL_AUDIT_2026-08-30.md`](MECHANICAL_LAYER_MATH_CALCULUS_PHASE2_IMPL_AUDIT_2026-08-30.md)。
> 处理项登记：TODO P2-10「阶段 2 全面审查处理（2026-08-31）」（R1–R9）与
> BACKLOG P2-10 指针；放行计数不变（未闭合仍 38，V1–V3 待续）。

## 1. 总体判定

三方面结论：

- **设计合理性：高，无阻断项**。固定墙钟（err/stall）与轮语义（stuck/prog，
  秒值经 T̂ 换算、非拟合）分层自洽；err 180s 绝对锚 + T̂ 钳制 [3,600] 防自指
  反馈；二阶闭式解避免右端点离散化伪迹；fires 仅内部留痕、模型无感边界清晰；
  ADR-0010 §14.47 转录与正式设计逐项一致。设计层仅一处措辞歧义（F14-a：
  §4.5「最近 16–32 个决策间隔」vs「≥8 采样启用」）。
- **实现合理性：中上**。核心计算正确（闭式解对 0.05s ODE 参考偏差 ≤0.006、
  右端点离散化 ≥2× 分离测试、expm1 零除防护、T̂ 截尾/钳制/启用阈值正确）；
  运行时接线、侧车零迁移、fail-loud 参数校验干净；测试全绿（见 §5）。
- **符合性：整体符合，但存在 2 项重大跨切片一致性缺口（F1/F2）与若干契约
  「半落地」问题（F4–F6）**。总体为**有条件放行**：S1/S2 可进入阶段 3，但
  F1/F2 须在 V2 前裁决，F3–F6 建议在 V1 前处理。

## 2. 重大发现

### F1 err 事件谓词三方冲突（重大｜I1×I3）

**离线复验锚点不代表生产行为。** 三处对「工具错误事件」的定义不一致：

- 复验 harness
  [`lif_replay.rs`](../../orz/crates/orz-assurance/examples/lif_replay.rs:69)
  `outcome_of` 把 `exit_code≠0 → Error`（且 `status=error` 优先判 Error）；
- 生产运行时
  [`host_exec.rs`](../../orz/crates/orz-loop/src/host_exec.rs:2509) 按 D2 值语义
  把 `exit≠0 → Other`（中性），只喂 `timed_out=Error` 与宿主级错误；
- 设计 §2.2④ 明言 exit≠0 是结构化**值**不是 Fail，但 §4.3 err 输入又写
  「工具错误事件」，未定义是否含非零退出与拒单。

102-run 语料量化（`D:\tb-eval\jobs-official`，4907 个 tool_completed）：

| 口径 | err 事件数 | err fires（同引擎动力学模拟） |
|---|---|---|
| replay 谓词（status=error ∨ exit≠0） | 513 | 24 fires / 22 runs（与审计一致） |
| 生产谓词（宿主错误 + 超时，拒单不喂） | 106 | 1 fire / 1 run |

407 个事件分类不同、波及 89/102 run。且锚点拒单、候选门拒绝等 status=error
事件在生产中**根本不喂 LIF**（早返回路径）。结论：「err 22 run/24 fires 精确
复现」只在 harness 自身谓词下成立；V2 四对照门之前必须裁定「工具错误事件」的
精确定义并统一设计措辞、replay 谓词、运行时喂入三方。

### F2 deny 通道无生产喂入（重大｜I1×I3）

[`channels.rs`](../../orz/crates/orz-assurance/src/lif/channels.rs:63) 实现 deny
常量与实例，但全树无 `deny.spike` 调用点；`ToolOutcome` 只有 Error/Success/Other
三态。门拒绝路径（policy_denial、锚点拒单、candidate-cap、sealed_tool_denied）
要么早返回不经 `on_tool_event`，要么被归为中性 Other。语料中 policy_denial 事件
为 0，故「deny 0/102 精确一致」是空对照。处理：接线拒绝路径（如新增
`ToolOutcome::Deny`）或在设计/审计中显式登记「本阶段 deny 未接线」，避免 V2
对它做空对照。

### F3 跨 prompt 恢复重建 has_success 缺失（重大｜I4）

[`restore_spikes`](../../orz/crates/orz-assurance/src/lif/temporal.rs:319) 只恢复
spike 列表与当前域，**未恢复 `has_success`**（可从「末 spike 非 Start」推断）。
生产路径 [`acp_server.rs`](../../orz/crates/orz-host/src/acp_server.rs:1125) 在
全新引擎上调用恢复，`has_success` 恒 false → 恢复后第一个决策轮 label 返回
Start，产生「Normal/Stuck → Start」虚假迁移与 spike，待下次成功再跳回。
单元测试
[`temporal.rs:445`](../../orz/crates/orz-assurance/src/lif/temporal.rs:445)
在 restore 前手动 `observe_tool_outcome(Success)` 预置 has_success，恰好掩盖此
缺陷，且无端到端恢复测试。恢复的迁移元数据（at_round=0、dwell_rounds=0、
recovery 恒 false）亦为近似值，与 §3.5「驻留轮数由相邻 spike 推导」在仅存 t
的数据结构下不成立。

### F4 temporal ≤1 KiB 截断路径溢出（重大｜I5）

[`controller.rs:1778`](../../orz/crates/orz-loop/src/controller.rs:1778) 截到
1024 字节后追加 `\n(truncated)`（12 B）不预扣 marker 长度，实际输出
1024+12=1036 B > 1 KiB。recent(20) 在长 run 必触发截断（20 行 ×~150 B ≈ 3 KB），
现有测试只覆盖未截断路径。修复 = 预扣 marker 长度 + 补一条触截断路径测试。

### F5 信封契约「半落地」：FailEnvelope 死代码 / cap 声明不实 / Fail 嗅探（重大｜I5×I6）

- `FailEnvelope`（[`tool_envelope.rs`](../../orz/crates/orz-assurance/src/tool_envelope.rs:102)）
  全树无构造点，所有失败路径仍走 legacy `{error}` + exit_code 形态——设计 §2.1
  Fail 半边契约未接线（审计以「核查并契约锁定」表述准确，但范围比 §7「8 工具
  落地」字面窄，应纳入 V1 验收项）；
- blackboard_read 对非 temporal 分区统一声明 `total_cap: 8192`
  （[`host_exec.rs:1465`](../../orz/crates/orz-loop/src/host_exec.rs:1465)），但
  plan/actions 等段并无 ≤8 KiB 总截断——声明上界 ≠ 实际保证；
- [`reducer.rs:188`](../../orz/crates/orz-assurance/src/reducer.rs:188) 用
  「含 step 且含 code」嗅探 Fail，而非判别式和类型，成功载荷恰含该两键会被
  静默误判并短路 pipe。

### F6 reducer 假组合与 grep→read 声明不符（重大｜I6）

[`splice_result`](../../orz/crates/orz-assurance/src/reducer.rs:88) 只拼接顶层
path/hash/size；grep 载荷是 `matches[]`，无「选哪个 match、span 如何转
offset/length」机制，`pipe(grep, read)` 静默退化为无组合。`pipe(terminal.run,
file.read)` 等不兼容组合通过 `validate_term`（只查 known_tools + 对象参数），
splice 找不到字段就原样调用——违背类型化组合 fail-closed 精神。V1 前需裁决：
补实现或删声明；不兼容组合应返回类型化错误（arg_validation）。

## 3. 次要发现

| # | 严重度 | 切片 | 位置 | 内容 |
|---|---|---|---|---|
| F7 | 中 | I3×I5 | [controller.rs](../../orz/crates/orz-loop/src/controller.rs:2215) | blackboard_read 工具声明缺 `selector`/`k`/`name` 参数（仅 section 描述文字提及）；Recent/History/Feature 查询面按 JSON schema 不可发现 |
| F8 | 中 | I2 | [schema](../../runtime/tool-completed-event-payload-v0.1.schema.json) / [verifier](../../assurance/run_event_journal_validation.py:2747) | `cmd_preview` 设计为 ≤80 字节，schema `maxLength` 与 verifier `len()` 按字符计；生产者按字节截断（更严），无实发违例但机械校验弱于契约 |
| F9 | 低 | I1/I2 审计记录 | 计数勘误 | lif 测试实为 23 项（`cargo test --list` 复测；审计写 30）；FailureTargetCrossCheckTests 实为 11 个函数（审计写 12 用例）；orz-loop lib 实测 644（审计写 641）；`cargo test` 编译测试面有 1 处 `unused variable: tau_stuck` 告警（`clippy --lib` 不覆盖测试 cfg） |
| F10 | 低 | I2 | [cmd fixture](../../runtime/fixtures/run-event-v0.2/payloads/tool-completed.failure-target-cmd.valid.json) | 正例 fixture 的 id 是 `sha256("test")`，与 cmd_preview 无对应关系（verifier 不做原文重算故不违例，但可复核性弱） |
| F11 | 低 | I2 | 设计 §5.4 | receipt↔事件链逐段同构核对未实现、未排期、未显式挂账——建议登记为未闭合项 |
| F12 | 低 | I5 | [tool_envelope.rs](../../orz/crates/orz-assurance/src/tool_envelope.rs:147) | `enforce_bound` 在 max_bytes < marker 长度时仍返回超界串；`to_value()` 用 `unwrap_or(Value::Null)` 吞序列化失败 |
| F13 | 低 | I6 | [reducer.rs](../../orz/crates/orz-assurance/src/reducer.rs:102) | 锚点形状漂移：设计 `Anchor={path,hash,size}`，实现产出 `expected_anchor={sha256,size}`（与运行时自洽、与设计类型名不一致，建议统一） |

另登记观察项（INFO，不阻塞）：a) §4.5「16–32 窗口」措辞歧义（实现为 8–32 起算
中位数，建议定稿统一措辞）；b) 每决策轮先 `set_tau/set_rhythm` 再 `advance`，
新 τ 对整段间隔生效（闭式解假设 τ 区间内恒定；err 固定 180s 无影响，
prog/stuck 在低精度容忍内，留档）；c) `TemporalState::recent` 返回旧→新而设计
写「时间倒序」，渲染层 `rev()` 后语义正确，库函数文档需对齐；d) fire 判定为
事件/决策轮步进离散，C2 对照应使用同一离散判定口径；e) `u_stuck` 非有限值
静默置 0（防御性，当前不可触发，建议留档）。

## 4. 逐切片结论

| 切片 | 结论 | 关键处理项 |
|---|---|---|
| I1 | 有条件通过 | F1/F2（V2 前置）、F9/F14-a |
| I2 | 通过 | F8/F10/F11 |
| I3 | 有条件通过 | F4（V1 前修）、F7 |
| I4 | 有条件通过 | F3（恢复重建，V2 聚类对照前决定是否补 entry_round） |
| I5 | 有条件通过 | F4/F5（纳入 V1 验收项）、F12 |
| I6 | 有条件通过 | F6（V1 前裁决）、F13 |

## 5. 实测证据（本次审查实跑）

| 项 | 结果 |
|---|---|
| orz-assurance lib | 178 passed（含 lif/envelope/reducer） |
| orz-loop lib | 644 测试（`--list` 实测；审计记录 641 为勘误项 F9） |
| orz-host 侧车定向 | 5 passed（roundtrip/legacy/空会话/retention） |
| Python journal 校验 | 236 passed；FailureTargetCrossCheckTests 11 passed |
| lif_replay 102 runs | 4157 决策点；err 22 run/24 fires（旧口径；R1 裁定生产口径后重跑为 1 run/1 fire，见 §7）；轮语义 k=3 → 3/3（轮⊂时间成立）；stuck 0/102、峰值 θ 比 0.991285（旧口径；新口径 0.6087）；slow 2；stall 8；deny 0；C1 17 变/5 不变——与 I1 审计表逐项一致 |
| 语料量化 | 4907 tool_completed；status=error 275（其中 169 同时 exit≠0）；exit≠0 无 error 状态 238；timed_out 0；policy_denial 0；replay vs 生产谓词分类差异 407 事件 / 89 run（F1 底座） |
| 静态门 | `git diff --check` 双仓库干净；check_repository 仅报 orz 子模块工作树 dirty（未提交期间固有） |

## 6. 处理项登记（指针）

R1–R9 明细与勾选状态统一维护于 TODO P2-10「阶段 2 全面审查处理（2026-08-31）」；
BACKLOG P2-10 仅保留指针。放行计数不变（未闭合仍 38）：R1–R9 为审查处理项，
非新放行切片，闭合时按既有纪律入账。

- R1（V2 前置）：err 事件谓词三方统一（设计 §4.3 定义 / lif_replay outcome_of /
  host_exec 喂入含拒单路径）——F1；
- R2（V2 前置）：deny 通道接线（`ToolOutcome::Deny` + 拒绝事件喂入）或显式登记
  休眠——F2；
- R3（V1 前）：temporal ≤1 KiB 截断 marker 预扣 + 触截断路径测试——F4；
- R4（V1 前）：blackboard_read 工具定义补 selector/k/name 参数声明——F7；
- R5（V1 前）：reducer 不兼容 pipe 类型化拒绝（arg_validation）/ grep→read 管线
  补实现或删声明——F6；
- R6（V1 前）：restore_spikes has_success 恢复 + recovery 重算 + 跨 prompt
  端到端恢复测试——F3；
- R7（V1 验收项）：FailEnvelope 接线与 Board cap 真实性收口（非 temporal 分区
  实际截断或改声明）——F5；
- R8（顺手修）：cmd_preview ≤80 字节口径（verifier 按 UTF-8 字节校验）——F8；
- R9（勘误）：审计计数（lif 30→23、FailureTarget 12→11、orz-loop 641→644）+
  channels 测试告警修复——F9。

## 7. 处理状态（2026-08-31 更新）

R1–R9 与 F10–F14 处理状态（R1/R2/R5 经 2026-08-31 用户裁决后执行；R2/R5
裁决为独立待办并已实施）：

| 项 | 状态 | 实测证据 |
|---|---|---|
| R1 | **已修** | 2026-08-31 用户裁定「维持生产口径」：err=超时+宿主级错误（status=error 无 exit_code 值），exit≠0=D2 值（Other）。lif_replay outcome_of 对齐生产谓词；102-run 重跑（`LIF_102RUNS_REPLAY_2026-08-31_R1.json`）：err 1 run/1 fire、stuck 峰值 θ 比 0.61（err 口径收窄后回落，贴近首轮 0.65）、决策点 4157 不变 |
| R2 | **已实施** | 2026-08-31 完成：orz-assurance `ToolOutcome::Deny` + 统一词汇表 `is_denial_code`/`classify_event_outcome`（生产/复验同一谓词）；host_exec（退役/封存/锚点/检索模式/权限/候选门/计划门/console）与 agent_loop（角色门/计划轮/注入预算）全部拒绝路径喂 deny + 端到端测试；102-run 复验（`LIF_102RUNS_REPLAY_2026-08-31_R2.json`）：deny 3 run/4 fires（语料拒绝事件 85：content_anchor_mismatch 15、role_write 57、candidate_cap 7、plan_round 4 等）——V2 deny 对照非空；err 1→0 run/0 fire 为候选门拒绝从 err 移入 deny 的语义修正 |
| R3 | **已修** | controller.rs 截断预扣 12 B marker（payload+marker ≤1024 B）；新增 `temporal_board_truncation_never_exceeds_cap` 测试 |
| R4 | **已修** | blackboard_read 工具定义补 selector/k/name 参数声明（与 host_exec 参数解析一致） |
| R5 | **已实施** | reducer pipe 兼容矩阵收口（read→grep / read→search_replace / grep→read 三条类型化透镜，其余在验证与归约两层均拒绝）；grep→read 按 `match_index`（缺省 0）确定性选 match，`span {start,end}` → read `offset`/`length` splice（显式参数优先），空 match/越界索引/坏 span 以 no_match / match_index_out_of_range / invalid_match_index / invalid_match_span 类型化关闭；不兼容 pipe 返回 arg_validation Fail（`pipe_incompatible`）且任一工具不执行（reducer 14 项测试全绿） |
| R6 | **已修** | temporal.rs restore_spikes 推断 has_success + 重算 recovery；单测去掩盖 + controller 级恢复测试 `restored_temporal_spikes_do_not_regress_to_start`（全新引擎=生产 ACP 恢复路径形态） |
| R7 | **已修（收口）** | Board cap 真实化（非 temporal 分区 entries 实际截断 ≤8 KiB）+ reducer `is_fail` 判别式（step 且无 summary，不再字段嗅探）+ F12 边界修复；纯子项错误 FailEnvelope 全量接线列入 V1 验收项 |
| R8 | **已修** | verifier 按 UTF-8 字节校验 cmd_preview + 多字节正/负例；schema 说明注明字节契约由 verifier 兜底 |
| R9 | **已修** | lif 测试计数更正为 **23**（`cargo test --list` 复测；本记录原写 24 一并更正）、FailureTarget 12→11、orz-loop 641→644；移除 channels 测试未使用 `tau_stuck` |
| F10 | **已修** | 正例 fixture cmd_preview 改 `"test"`，与 id=sha256("test") 对应可复核 |
| F11 | **已闭合（2026-09-01）** | 事件链侧核对 V1c 已随 V1 FakeProvider 验证面实施（2026-08-31，`_verify_v02_receipt_event_isomorphism`，verifier 245 passed）；P2-11 依赖图主线（2026-09-01）把依赖图事实纳入同一核对面（`dep_graph` 可选事件字段 + `_verify_v02_dep_graph_events`），设计 §5.4 状态行同步更新，正式闭合 |
| F12 | **已修** | `enforce_bound` 小 cap（<marker）返回 UTF-8 安全前缀恒 ≤max_bytes；Ok/Fail 信封 `to_value` 改 Result（fail-loud），调用点显式 map_err |
| F13 | **已修** | 设计 `Anchor` 类型对齐运行时 `{sha256, size}`（mtime 为工具 schema 可选附加字段） |
| F14 | **已修** | §4.5 措辞统一为「最近 8–32 个决策间隔」（采样 ≥8 启用、缓冲 ≤32）；estimator 注释同步 |

验证：orz-assurance lib 192 passed（reducer 14 项，含 R5 新增 7 项）；orz-loop lib 645 passed / 0 failed /
3 ignored；Python journal 校验 236 passed；lif_replay 102-run 重跑（R1 新
口径 + R2 deny 接线复验）；`git diff --check` 双仓库干净；变更文件 clippy
无新增告警
（orz-assurance 1 条 `THatMode` derive 建议与 orz-loop 28 条均为实施既有
基线，非本次处理新增）。

独立待办（2026-08-31 用户裁决，下一轮实施，不阻塞阶段 3 排期）：
- **R2（F2）**：**已实施**（2026-08-31，见上表 R2 行；语料 policy_denial=0，
  接线后 V2 deny 对照非空=3 run/4 fires）。
- **R5（F6）**：**已实施**（2026-08-31，见上表 R5 行；reducer 14 项测试 +
  orz-assurance lib 192 passed；兼容矩阵/类型化拒绝文档同步至设计 §2.3）。
