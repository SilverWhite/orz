# TER M1 循环面切片审查报告（02：T1.6–T1.9 + T2.1）

> 审查人：根代理补跑 a2 切片（先前 a2 口头回报明细未落盘，本报告为
> 复核产出）。日期：2026-09-04。基线：orz HEAD 901cdcc3（T1.6 f2ed4d8e、
> T1.7 e6e8aee0、T1.8 95bf432c、T1.9 da2c23a1 均在历史内）；主仓库
> ac122e4（T2.1）。只读静态核对；未复跑 cargo（工作树被 0l 并行改动
> 污染，且审查纪律禁跑）。

## 1. 总评

T1.6–T1.9 与 T2.1 的代码与审计声明高度吻合：processes live 分区的
“读取时现算 + live-only 越权守卫 + ≤8KiB + 空态”齐备；轮预算 0=unlimited
语义干净（>0 才挂闸）；F6 pull 的 elapsed/limit/remaining 与未施加不虚构
一致；F6 push 默认 off、注入文本中性、每档一次 ≤3/run、事件载荷与 verifier
规则一致；T2.1 提交态无 840/--max-wallclock 派生、DryRun 计划行存在。

发现 **P1 × 1（cross-slice 缺口）**、**P3 × 4**、**待实机验证 × 2**。

## 2. 发现清单

### M1L-1（P1；符合性缺口，跨切片）——tool_running `status=idle_killed`
journal 事件无 Rust 生产者；T1.13 门审计存在过度声明

- 证据：T0.2 契约 §4（idle-kill 形态 + 链规则 + fixtures）与 T1.5 审计
  §5（“journal 侧接线…归 T1.13 门”）与 T1.13 审计 §1.3（“Rust 侧新生
  产者…`tool_running idle_killed`（T1.5）…已由单测锁定”）。
- 代码事实（orz HEAD 901cdcc3）：`EventType::ToolRunning` 仅 host_exec.rs
  :2982 一处生产（S5-2 auto-bg mid-run，payload 无 status 字段）；全仓库
  检索 `idle_killed` 仅命中 terminal.rs（IDLE_KILL_SIGNAL/快照）、
  bash/mod.rs（KillReason 渲染）、task_completion.rs（提醒文本）；**无任何
  代码构造 `{"status":"idle_killed","reason":…}` 的 tool_running 事件，
  也无对应单测**。Python verifier 273 只证明 fixture 合规，不证明生产
  者接线。
- 影响：真实 journal 在 idle-kill 场景只有提醒文本与 processes 快照
  status=killed；T0.2 §4 的生命周期事件与“每 call_id 至多一次/不引入
  第二个 tool_completed”链规则无法在实机验证；M3 T3.1 若要求
  idle_killed 事件证据会落空（当前 T3.1 只要求 tool_running，auto-bg
  mid-run 可满足，但 idle-kill 形态缺位）。
- 建议：在 host_exec 完成提醒/进程回收处补“idle-killed → 记
  tool_running(status=idle_killed, reason, 引用原 call_id/task_id)”
  生产者 + 单测；或显式降级 T0.2 契约为“快照 + 提醒文本”并修正 T1.13
  审计 §1.3 措辞；二选一需在审查门定案。

### M1L-2（P3；T2.1 边界）——runner 保留 `perTaskTimeoutSeconds < 60 →
取 60` floor，与“唯一墙钟=官方 agent_timeout_seconds”字面不完全一致

- 位置：run_agent_arm.ps1（ac122e4）L317–319 floor 保留。
- 说明：官方值（900/3600）远高于 60，floor 不产生实际偏差；但若官方值
  <60，sandbox --timeout 与 ORZ_MAX_WALLCLOCK 会与官方值不一致。建议
  注释说明 floor 仅防引导脚本误配，或在取官方值后不再 floor。

### M1L-3（P3；T1.6 契约单位）——命令摘要上限“≤80B”实为 ≤80 字符

- 位置：processes.rs `preview()` 用 `chars().count()` 截断 + 省略号；
  T0.2 §5.1 写 ≤80B。
- 说明：ASCII 下字符=字节无差别；含 CJK/emoji 的 display_command 会超
  80B。建议把契约改“≤80 字符”或按字节截断（UTF-8 边界安全）。

### M1L-4（P3；口径）——push 次数“3–4/≤4/≤3”三处表述不一

- 见设计层 D-3；实现侧证据：prompt.rs `f6_push_cue_for_remaining` 三档
  数组每档一次 ⇒ ≤3/run；controller 注释“T0.2 verifier 上限 4 兼容”。
- 建议统一文档口径（实现 ≤3、机器上限 ≤4）。

### M1L-5（P3；登记）——三个 `GROK_*` / `ORZ_*` env override 与单一
生效源的关系未入 T0.1 “其它源”列（同设计层 D-4、a1 S7）。

### M1L-6（P3；T1.8 语义边界）——`ORZ_MAX_WALLCLOCK=0` 时
`parse_main_wallclock_limit_secs` 返回 None（无上限），且 F6 push 要求
limit Some 才注入——语义正确，但 0 与“未配置”在渲染面上都显示
limit none，无法区分“显式禁用”与“未施加”；如需审计区分可加来源行
（提示级）。

### M1L-7（待实机；T2.1 边界）——ORZ_MAX_WALLCLOCK env 是否能穿过
sandbox 命令边界到达 orz 进程未实证

- DryRun 只证明计划行 env 值设置，不证明 Windows native sandbox 内 orz
  读到此 env。若 sandbox 重置环境，F6 pull/push 会在实机读到“无上限”
  （F6 push 将零注入）。T2.4/实机 DryRun 需在 orz journal/输出断言
  WALLCLOCK_LIMIT 或 budget_cue 出现。

## 3. 逐项符合性表

| 验收项（TODO2） | 判定 | 证据 |
|---|---|---|
| T1.6 processes live：现算快照/状态/pid/killable/fail-closed 空 | PASS | terminal.rs `live_task_snapshots()`（f2ed4d8e）；processes.rs 渲染 + 4 测试；blackboard.rs 2 回达测试；host 映射 1 测试 |
| T1.6 越权守卫（epoch/receipt_id → exit 1） | PASS | controller.rs `render_processes_section` Err 分支 + host_exec O4 处理 + 事件断言测试 |
| T1.6 kill 形态不暴露 &/is_background | PASS（语义） | blackboard_read 描述“kill 经既有 PID 中断语义”；模型以 taskkill/kill 命令终结 PID（bash_mod.rs:1672 文案）；不新增 & 面 |
| T1.7 默认无 120 拦截 | PASS | MAX_TOOL_ROUNDS=0；`>0` 守卫；host_exec 单测三工具轮放行且无 tool_rounds_limit/exhaustion |
| T1.7 显式上限逃生阀生效 | PASS | with_max_tool_rounds/ORZ_MAX_TOOL_ROUNDS 保留；session 面 capped 渲染测试 |
| T1.7 检索取 min 组合正确 | PASS | dispatch.rs `profile_rounds`（0/None 视为无上限再取 min） |
| T1.8 session 面 wallclock（elapsed/limit/remaining/未施加不虚构） | PASS | prompt.rs with_wallclock 渲染 + 单测；controller run_elapsed 经 LIF run_origin 只读 |
| T1.9 F6 push 默认 off 零注入 | PASS | f6_push_enabled_override() 白名单匹配；单测 off 零事件零文本 |
| T1.9 on 时中性事实 ≤3/run + 事件 | PASS | prompt f6_budget_cue_block 无建议文案单测；crossed[3] 逐档一次；事件载荷 remaining<threshold、rounds_used |
| T1.9 注入块不持久化 | PASS | is_injected_block_text 注册 F6_BUDGET_CUE_PREFIX |
| T2.1 无 --max-wallclock/840 | PASS | ac122e4 提交态 grep 零命中；DryRun 计划行存在；ORZ_MAX_WALLCLOCK=官方值 + 清理 |
| T2.1 唯一评测墙钟 | PASS（字面 60s floor 见 M1L-2） | sandbox --timeout=perTaskTimeoutSeconds（官方 agent_timeout_seconds） |
| 跨层：idle-kill journal 接线 | **FAIL（缺生产者）** | 见 M1L-1；T1.13 §1.3 声明无法在代码定位 |

## 4. 审计文档真实性核对

- TER_T1_6：声称的测试名与结构存在（processes.rs 4 + blackboard 2 +
  terminal 1 + host 1）；“命令摘要 ≤80B”按字符实现（M1L-3）。
- TER_T1_7/T1_8/T1_9：声称的测试名存在；push ≤3 语义与代码一致。
- TER_T2_1：DryRun 数值（900/3600、无 840）为已清理临时证据，无法复核；
  静态代码与声称一致。840 清理范围边界（历史 harness 不改）合理。
- T1.13 §1.3 “tool_running idle_killed 由单测锁定”无法在 orz HEAD 找到
  对应单测/生产者（M1L-1）——疑似将 Python fixture 测试误记为 Rust
  生产者测试。

## 5. 正面发现

- 0=unlimited 在轮数与 wallclock/检索组合处的处理（min 语义）比设计稿
  更严谨，测试覆盖了“默认无上限 + 检索档位仍生效”。
- F6 push 逐档记账用 Mutex<[bool;3]>，run 起始复位，天然满足每 run 上限，
  且一跳多档只取第一档的刷屏保护合理。
- processes/env live 越权处理与 session 面 O4 纪律完全同构（错误事件 +
  exit 1 + 文本回达），复用度高。

## 6. 结论

除 M1L-1（idle-kill journal 生产者缺位，P1）外，循环面无功能性缺陷。
M1L-1 需要审查门裁决（补生产者或显式降级契约），其余 P3 项可随
review-handling 批次合并处理。
