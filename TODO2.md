# TODO2 — 工具执行层改革（TER）实施步骤

> 用途：TER（Tool Execution Layer Reform）专用分步实施勾选树；主
> [TODO.md](TODO.md) 只保留指针，本文为 TER 明细权威。
> 上级：docs/BACKLOG2.md（TER 开放项）/ 设计稿
> [TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md](docs/TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md)。
> 分步原则：每一步都是最小可验收单元（改代码 + 单测 + 验收），可随时
> 停；里程碑 M0–M3 各有一个放行门；步骤间按依赖排序，无依赖的步可单独
> 抽做但须过各自验收。
>
> **证据基线注记（2026-09-13，全项目深审 S-16 处置）**：M0–M2 已勾选项的
> 验收证据引用 0.3.x 载体时点，现载体已演进至 0.4.x（0t/0v/0x/0z 多轮
> bump），**历史勾选不自动等价当前行为**。M3（T3.1–T3.5）回归复验必须
> 以执行时点的当前 0.4.x 载体为基线重取证据（登记载体版本号），不得沿用
> 0.3.x 时点证据作为放行依据；M1/M2 若被后续批次触碰同一代码面，随批按
> 同纪律重验。

## M0 设计门（合约先行；只产出核对表与 schema，不写业务代码）

- [x] T0.1 默认值收敛核对表：BashParams struct default / 工具 schema
  默认 / 后端 15s（GROK env）三处来源逐项核对，产出“现状→目标”表
  （auto_background_on_timeout / foreground_block_budget_ms /
  hide_background_input / timeout 分层）。验收：核对表覆盖 §3.1 参数表
  全行，标注单一生效源方案。（2026-09-03 完成：核对表见
  [`TER_T0_1_DEFAULT_CONVERGENCE_CHECKLIST_2026-09-03.md`](docs/audits/TER_T0_1_DEFAULT_CONVERGENCE_CHECKLIST_2026-09-03.md)，
  覆盖 §3.1 全 6 行并给出单一生效源方案。）
- [x] T0.2 schema/verifier/fixtures 先行：新事件 `budget_cue_injected`；
  `tool_completed` 增 `output_truncated` / `total_bytes` /
  `output_object_id`（可选）；idle-kill 形态（status=idle_killed +
  reason）；黑板 `processes` / `env` live 分区渲染契约与越权边界。
  验收：orz-assurance 校验分支 + fixtures 先行，schema 版本号 bump。
  （2026-09-03 完成：契约与落地清单见
  [`TER_T0_2_SCHEMA_VERIFIER_FIXTURES_CONTRACT_2026-09-03.md`](docs/audits/TER_T0_2_SCHEMA_VERIFIER_FIXTURES_CONTRACT_2026-09-03.md)；
  run-event 枚举 53→54、tool_completed payload v0.1→v0.2、tool_running
  idle-kill 形态；校验/夹具/测试全绿 273 passed。）
- [x] T0.3 ADR 候选登记：TER 设计稿转 ADR-0010 §14.xx 候选项（含
  取代/衔接关系：S5-2 常驻化、PUSH→PULL 的 push 例外）。验收：ADR
  候选条目落盘，索引无冲突。
  （2026-09-03 完成：ADR-0010 §14.53 候选项 3 条落盘（TER 总登记 /
  S5-2 常驻化 / PUSH→PULL push 例外），CLI_PROJECT_INDEX v2.48 登记
  AUTH-TOOL-EXECUTION-REFORM（`pending`），设计稿状态行与 PUSH→PULL
  锚点同步；验收与 §0.5 检查见
  [`TER_T0_3_ADR_CANDIDATE_REGISTRATION_2026-09-03.md`](docs/audits/TER_T0_3_ADR_CANDIDATE_REGISTRATION_2026-09-03.md)。）
- [x] T0.4 放行签名：S0 核对表 + schema diff + ADR 候选齐备后签放行，
  进入 M1。验收：签名记录写回本文。
  （2026-09-03 完成：齐备条件逐项核过——T0.1 默认值收敛核对表 /
  T0.2 schema diff + 273 passed / T0.3 ADR-0010 §14.53 候选项 /
  生成器 v0.1 方向 A 落地（v0.1 树重建 0 差异）；用户确认放行，签名与
  M1 起点见
  [`TER_T0_4_M0_RELEASE_2026-09-03.md`](docs/audits/TER_T0_4_M0_RELEASE_2026-09-03.md)；
  “生成器 v0.2 表全面对齐”已单列专项
  [`BACKLOG2 TER-0.1`](docs/BACKLOG2.md)，不阻塞本门。M0 四步全闭，
  下一实施步为 T1.1。）

## M1 orz 主线（Linux 单测/构建闭环；每步都可独立停）

- [x] T1.1 S5-2 默认开启：`auto_background_on_timeout` struct 默认
  false→true；单测更新（含“关闭态仍可配”用例）。验收：默认构造即
  true，既有显式 false 用例不回归。
  （2026-09-03 完成：BashParams `Default` + serde 缺省均改
  `default_true`；orz-host 删除该字段与 `enabled_background` 的冗余
  显式注入（单一生效源，生效值不变）；新增 default_enables_auto_bg /
  explicit_false_disables_auto_bg，registry 与 bash 关闭态用例补显式
  false。验证：orz-tools bash 224 passed、grok_build::bash 156
  passed、orz-host 参数测试 1 passed、fmt --check 净、git diff
  --check exit 0。审计见
  [`TER_T1_1_S5_2_RESIDENT_DEFAULT_2026-09-03.md`](docs/audits/TER_T1_1_S5_2_RESIDENT_DEFAULT_2026-09-03.md)。）
- [x] T1.2 首报/后台化预算默认 180s：`foreground_block_budget_ms`
  生效默认 180_000（schema 与 struct 单一来源，消除 15s 后端默认）。
  验收：不传参数时前台命令 180s 触发 auto-bg；单测覆盖。
  （2026-09-03 完成：BashParams `Default` + serde 缺省 + 显式 `null`
  回退统一收敛 `DEFAULT_FOREGROUND_BLOCK_BUDGET_MS=180_000`；orz-host
  删除 300_000 显式注入（缺省经 serde 解析即 180_000）；终端后端兜底
  常量 15s→180s；schema timeout 描述与工具描述「中间回报点」改由生效
  预算渲染（不再硬编码 after 300s）。验证：orz-tools `--lib bash` 226
  passed、`foreground_block_budget` 18 passed（含 request 捕获用例与
  后端常量守卫）、orz-host 参数测试 1 passed、fmt --check 净、git
  diff --check exit 0。审计见
  [`TER_T1_2_FG_BUDGET_180S_DEFAULT_2026-09-03.md`](docs/audits/TER_T1_2_FG_BUDGET_180S_DEFAULT_2026-09-03.md)。）
  （2026-09-04 复审处理：F1 终端 actor 注释 “Default is 15s” 更正为
  180s 退役口径；F2 `effective_auto_bg_wait_ms` 改名
  `effective_fg_wait_ms` 并澄清语义 = FG wait deadline（仅当预算 <
  解析超时时才是 auto-bg 点，默认 120s<180s 时为 kill 点）；
  F3 `MAX_FOREGROUND_BLOCK` 注释更新为 “request 预算 + 后端兜底”
  口径。验证：`foreground_block_budget` 18 passed、`--lib bash`
  226 passed、fmt --check 净、git diff --check exit 0。见
  [`TER_T1_2_REVIEW_HANDLING_2026-09-04.md`](docs/audits/TER_T1_2_REVIEW_HANDLING_2026-09-04.md)。）
- [x] T1.3 模型面封闭：`hide_background_input` 默认 true；显式
  `&`/is_background 拒绝用例保持。验收：单测 + 工具 schema 无
  is_background 暴露。
  （2026-09-04 完成：BashParams `Default` + serde 缺省均改
  `default_true`（resident 封闭默认）；orz-host 删除
  `hide_background_input=true` 冗余显式注入（单一生效源，生效值不变）；
  显式 `false` 逃生阀保留（可见后台面 opt-in，registry 级默认封闭
  测试 `bash_definition_closed_by_default`）。同步：既有可见面用例
  夹具补显式 false；顺手校正 T1.2 遗留的 orz-workspace auto-bg 接线
  用例（短预算 500ms + timeout 300s 驱动，T1.2 预算语义下原 timeout
  300ms 属 kill-on-timeout）。验证：orz-tools `--lib bash` 229 passed
  （含新增 default_closes_is_background_surface /
  serde_omission_and_explicit_false_for_hide_background_input）、
  orz-host 参数测试 1 passed、orz-workspace 后台接线 7 passed、
  fmt --check 净、git diff --check exit 0。审计见
  [`TER_T1_3_MODEL_FACE_CLOSURE_2026-09-04.md`](docs/audits/TER_T1_3_MODEL_FACE_CLOSURE_2026-09-04.md)。）
- [x] T1.4 去硬杀语义：timeout 分层（普通/程序/模型上限）不再作为杀
  进程点，改为 auto-bg deadline 引用；原 timed_out 杀进程测试改为
  “auto-bg 或 idle-kill”断言。验收：无任何“满 timeout 杀活跃命令”
  代码路径（评测墙钟除外）。
  （2026-09-04 完成：前台解析超时与 180s 预算先到者即 auto-bg deadline
  （终端按 `min` 判定，bash 层取消“timeout ≤ 预算 → kill-on-timeout”
  逐调用门）；auto-bg/用户后台化后原解析超时退役，统一收敛 10h 绝对兜底
  （0b 清扫只撞绝对上限；显式后台任务正向模型超时 kill-backstop 保留）；
  kill-on-timeout 仅剩显式 `auto_background_on_timeout=false` 逃生阀；
  模型面文案删除 “Timeout enforcement … kills” 与 “300s ordinary /
  600s program” 静态分层宣示，改由 `min(默认超时, 预算)` 单源渲染。
  验证：orz-tools bash 230 passed、终端 actor 40 passed、orz-host 参数
  1 passed、orz-workspace 接线 1 passed、fmt --check 净、git diff
  --check exit 0。审计见
  [`TER_T1_4_NO_HARD_TIMEOUT_KILL_2026-09-04.md`](docs/audits/TER_T1_4_NO_HARD_TIMEOUT_KILL_2026-09-04.md)。）
- [x] T1.5 idle+CPU 兜底：进程监视采样器（输出字节增长 + CPU 时间）；
  连续 300s 无活跃 → kill + 提醒文本 + idle_killed 事件；阈值参数化。
  验收：模拟“无输出计算”与“真 idle”两类用例，不误杀前者。
  （2026-09-04 完成：`ActivitySampler` 每 tick 比输出字节、1s 节流读
  进程树 CPU（Windows Job 记账 / Linux /proc pgrp 汇总；其它平台 CPU
  未知不判 idle）；连续无两者增长满 `idle_kill_timeout`（默认 300s，
  actor/env `GROK_IDLE_KILL_TIMEOUT_MS` 参数化，0=禁用）→ SIGTERM+
  `signal=idle_killed`；完成提醒渲染 `idle-killed (no output growth or
  CPU activity for 300s)`（T0.2 同口径）；`KillReason` 增 idle_killed。
  验证：终端 actor 47 passed（含 5 采样器用例 + sleep idle-kill 实机 +
  无输出忙循环不误杀实机）、bash 231 passed、task_completion 49
  passed、fmt --check 净、git diff --check exit 0。审计见
  [`TER_T1_5_IDLE_CPU_BACKSTOP_2026-09-04.md`](docs/audits/TER_T1_5_IDLE_CPU_BACKSTOP_2026-09-04.md)。）
- [x] T1.6 黑板 processes live 分区：读取时现算快照（task_id/命令/
  elapsed/状态/活跃度/字节/可 kill）；状态跃迁落事件；kill 动作模型面
  可达且不暴露 `&`。验收：live 渲染单测 + 越权边界用例。
  （2026-09-04 完成：终端 `TaskLiveSnapshot` + `list_live_tasks`（读取
  时现算，status=running/idle/completed/killed）；LoopHost
  `terminal_live_processes`（fail-closed 默认空）；`section=processes`
  渲染（命令摘要 ≤80B、≤8KiB 预算截断、空态「（无）」）；live-only
  越权守卫（epoch/receipt_id 显式报错 exit 1）；tool schema/错误文案
  同步；kill 形态核对 = 进程行 pid + 既有 run_terminal_cmd PID 中断
  （不暴露 `&`/is_background）。验证：终端 48 passed、orz-loop processes
  6 passed（渲染 4 + 回达 + 越权）、orz-host 映射 1 passed、check 净、
  fmt/diff check 净。审计见
  [`TER_T1_6_PROCESSES_LIVE_PARTITION_2026-09-04.md`](docs/audits/TER_T1_6_PROCESSES_LIVE_PARTITION_2026-09-04.md)。）
- [x] T1.7 轮预算默认无限制：max_tool_rounds 默认移除（“120 per turn”
  静态文案动态化/移除）；保留可配上限逃生阀与 budget_insufficient。
  验收：默认运行无 120 拦截；显式配置上限时原机制仍生效。
  （2026-09-04 完成：`MAX_TOOL_ROUNDS` 120 → 0（0=unlimited 默认）；
  轮数闸加 `>0` 守卫（默认不挂 tool_rounds_limit/exhaustion）；
  session 面 budget=0 渲染 unlimited（显式上限仍按生效值渲染数字档与
  remaining）；`budget_insufficient` 预检仅显式非零上限时生效；检索
  子代理与主车道取 min 的组合在“主车道无上限”下正确落到检索档位。
  验证：orz-loop 全量 699 passed / 3 ignored、fmt --check 净、git diff
  --check exit 0。审计见
  [`TER_T1_7_UNLIMITED_ROUND_BUDGET_DEFAULT_2026-09-04.md`](docs/audits/TER_T1_7_UNLIMITED_ROUND_BUDGET_DEFAULT_2026-09-04.md)。）
- [x] T1.8 F6 pull 面：session（或 processes）补 wallclock
  （elapsed/limit/remaining）；blackboard_read 返回含时间轴。验收：
  渲染单测 + 越权边界。
  （2026-09-04 完成：session 面 `session_face_block_with_wallclock` 增
  `WALLCLOCK_ELAPSED`（LIF run-relative 只读换算）/ `LIMIT` +
  `REMAINING`（`ORZ_MAX_WALLCLOCK` >0 生效；未施加渲染 limit none，
  不虚构 remaining）；旧 wrapper 输出逐字节不变；epoch/receipt_id 越权
  守卫沿用。验证：orz-loop 全量 700 passed / 3 ignored、fmt --check 净、
  git diff --check exit 0。审计见
  [`TER_T1_8_F6_PULL_WALLCLOCK_2026-09-04.md`](docs/audits/TER_T1_8_F6_PULL_WALLCLOCK_2026-09-04.md)。）
- [x] T1.9 F6 push 档：阈值 <600/300/120s 注入中性事实 ≤4 次/run +
  `budget_cue_injected` 事件；开关 env/config 默认 off。验收：开关
  off 零注入（回归 PUSH→PULL）；on 时次数上限与事件可审。
  （2026-09-04 完成：`EventType` 增 BudgetCueInjected（v0.2 轨，
  snake_case 与 run-event schema 一致）；`ORZ_F6_PUSH` 默认 off +
  `ORZ_MAX_WALLCLOCK` 上限配置时，主车道每轮请求前按剩余 <600/300/120
  逐档注入一次中性事实（`[F6_BUDGET_CUE …]`，只报剩余/上限/已用轮、
  无建议，注册进 injected-block filter 不持久化）并记
  `budget_cue_injected`（remaining_seconds/rounds_used/threshold_seconds）；
  每 run ≤3 次（T0.2 verifier ≤4 兼容）；off 零注入/零事件/零文本。
  验证：orz-loop 全量 703 passed / 3 ignored、orz-assurance 201
  passed、fmt --check 净、git diff --check exit 0。审计见
  [`TER_T1_9_F6_PUSH_BUDGET_CUE_2026-09-04.md`](docs/audits/TER_T1_9_F6_PUSH_BUDGET_CUE_2026-09-04.md)。）
- [x] T1.10 W-F13a read_file 64KB 档：限制链核对（粗门 clamp/行 limit/
  50K 注入预算）后放宽至 64KB 级。验收：vm.js 级文件 ≤2 次读完；
  单次注入不触发截断；大文件仍可结构化分段。
  （2026-09-04 完成：coarse gate 默认/上限 16/32K → **64K**（下限 8K
  逃生阀保留）；限制链核对：64K ASCII ≈ ≤16K token < 25K 读档/50K 单轮
  预算、行档 1000 行语义保留；>64K 仍回有界信封（preview ≤4K + offset
  结构化分段）；orz-host 配置口子 clamp 同步 8–64K；full/concise/
  handle 文档与旧 16K 测试档位同步。验证：read_file 桶 208 passed、
  orz-host clamp 1 passed、fmt --check 净、git diff --check exit 0。
  审计见
  [`TER_T1_10_W_F13A_READ_FILE_64K_2026-09-04.md`](docs/audits/TER_T1_10_W_F13A_READ_FILE_64K_2026-09-04.md)。）
- [x] T1.11 W-F13b 输出检索对象：长输出落盘对象 + pattern/行区间/尾部
  N 行检索；ToolCompleted 截断标记 + 对象指针（schema 已 T0.2 定稿）。
  验收：检索语义单测；模型无需 .gsa 即可补读自身输出。
  （2026-09-04 完成：orz-tools `computer::output_object` 公共检索 API
  （pattern 大小写不敏感/上限、1-based 行区间闭区间+越界 clamp、尾部
  N 行，统一固定解码链）；对象 id = 落盘 log 路径（read_file/grep 直接
  消费，无需 .gsa 摸黑）；host 映射截断输出为 TerminalOutputObject，
  ToolCompleted（v0.2 轨）落 output_truncated/total_bytes/output_object_id
  （配对规则照 T0.2）；console 重建路径透传。验证：output_object 4
  passed、journal 三字段 1 passed、host 实机 30K 截断映射 1 passed、
  orz-loop 全量 704 passed / 3 ignored、fmt/diff check 净。审计见
  [`TER_T1_11_W_F13B_OUTPUT_RETRIEVAL_OBJECT_2026-09-04.md`](docs/audits/TER_T1_11_W_F13B_OUTPUT_RETRIEVAL_OBJECT_2026-09-04.md)。）
- [x] T1.12 W-F11 环境快照：probe 扩展至代码工具环境（工具/语言/包/
  版本/连通判定，Linux 先本地快速判定）；落黑板 `section=env`（PULL）。
  验收：快照 ≤5s；env 分区 PULL 渲染 + 越权边界。
  （2026-09-04 完成：orz-host `env_snapshot`（固定注册表 PATH 存在性 +
  `--version` 并发探测单项 1s 超时、输入在场布尔、无 allowlist/任务结论
  ——实测 ≈1.6s < 5s）；`LoopHost::env_snapshot_facts`；`section=env`
  渲染（kind 白名单登记 tool/language/package/input/connectivity，越权
  kind 渲染层拒绝、8KiB 预算、空态（无））；epoch/receipt_id live-only
  显式报错；连通性行由 W-F12（M2 T2.2）闭环后接入（本步不伪造）。
  验证：orz-loop 全量 709 passed / 3 ignored、env 相关 13 passed、
  orz-host env_snapshot 2 passed（≤5s）、fmt/diff check 净。审计见
  [`TER_T1_12_W_F11_ENV_SNAPSHOT_2026-09-04.md`](docs/audits/TER_T1_12_W_F11_ENV_SNAPSHOT_2026-09-04.md)。）
- [x] T1.13 M1 验收：orz-loop/orz-assurance 全量测试绿 + clippy/fmt 净
  + Linux 三件套构建成功 + 事件链 verifier 过新 schema。放行进入 M2。
  （2026-09-04 完成：orz-loop --lib 709 passed / 3 ignored、orz-assurance
  201 passed；clippy 四 crate 零 error（存量告警登记）、fmt 净；Linux
  release 构建成功（WSL Ubuntu 24.04 x86_64，rustc 1.98.1：
  orz-tools/orz-loop/orz-host + orz-bin CLI 闭包 Finished release，
  Linux 编译修复 tty-utils 借用 + orz-tui EventType arm 并 Windows
  复检绿）；事件链 verifier 273 passed（T0.2 schema/fixtures）+
  Rust 新载荷单测。**M1 放行进入 M2**。审计见
  [`TER_T1_13_M1_GATE_2026-09-04.md`](docs/audits/TER_T1_13_M1_GATE_2026-09-04.md)。
  2026-09-04 全面审查：门审计 §1.3 “tool_running idle_killed 由单测锁定”
  属过度声明（当时只有 schema/fixtures/verifier）；生产者已由审查处理批
  补入（orz `35db6741`，见
  [`TER_REVIEW_HANDLING_2026-09-04.md`](docs/audits/TER_REVIEW_HANDLING_2026-09-04.md)
  与 ADR-0010 §14.55 条目 5）。）

## M2 Windows runner / VM（依赖 M1 新 build）

- [x] T2.1 墙钟单一化：删 runner `--max-wallclock` 与 60s 余量；
  sandbox `--timeout`=官方 agent_timeout_seconds 为唯一评测墙钟；F6
  push 读源以 runner 施加值为准。验收：DryRun 确认无 840 参数、逐题
  timeout 生效。
  （2026-09-04 完成：`run_agent_arm.ps1` 删除 `perTask-60` 派生与
  `--max-wallclock`（840 余量移除）；sandbox `--timeout` = task.json
  官方 agent_timeout_seconds 为唯一评测墙钟；官方值经
  `ORZ_MAX_WALLCLOCK` env 透传 orz（F6 pull/push 读源以 runner 施加值
  为准，T1.8/T1.9 消费）；DryRun 新增 wallclock 计划行。验证：DryRun
  两题 900s/3600s 计划输出无 --max-wallclock/840、逐题 --timeout 与
  env 生效、AGENT_ERRORS=0。审计见
  [`TER_T2_1_WALLCLOCK_SINGLE_SOURCE_2026-09-04.md`](docs/audits/TER_T2_1_WALLCLOCK_SINGLE_SOURCE_2026-09-04.md)。）
- [x] T2.2 W-F12 本地透明层：先测基准（当前墙外 Test-NetConnection /
  curl / Invoke-WebRequest / python requests 各自失败耗时表）→ 实现
  DNS/TCP 本地拒答 → 复测 ≤2s/目标；allowlist 内连通不变。验收：mteb
  HF 探测 ≤2s、连通扫描总成本 ≤10s。
  （2026-09-04 完成：根因定案——AppContainer（空能力）+ allowlist 不兼容
  （allow 规则被 AC compartment 吞，allowlist_reachable 必 FAIL），生产墙
  为 `--no-appcontainer` + allowlist（2026-09-03 裁决）；enforcement 探针
  增 `-ExpectAppcontainer` 与 WF12 stdout 时延行、runner 增
  `-NoAppcontainer`（no-AC 原位执行探针，修复 Errno 13 读文件）、sandbox
  工作区 grant 加 `/T` + 子进程 stderr/out spill、新增受控 op
  `vm-wf12-probe`（AC 基线 + no-AC 生产墙 + egress 前/复测 + DNS 拒答可逆
  部署）。验证（win-s4 实机）：no-AC 墙 allowlist_reachable PASS
  （15–47ms，连通不变）+ network/metadata_blocked 0–16ms；egress 复测
  （DNS 拒答 127.0.0.1:53，NXDOMAIN 7ms，测后 DNS 恢复）总成本 920ms
  （≤10s）、最差行 735ms（≤2s）；AC 基线 allowlist FAIL 作对照登记。
  审计见
  [`TER_T2_2_WF12_LOCAL_TRANSPARENT_LAYER_2026-09-04.md`](docs/audits/TER_T2_2_WF12_LOCAL_TRANSPARENT_LAYER_2026-09-04.md)。）
- [x] T2.3 Windows 后台任务存活验证（2026-09-07 完成，S3/S4 集中实机
  验证批 T2/批次 W1）：fake 场景三判据 ALL_PASS——① auto-bg 中报事件
  @180s + 跨调用存活（R2 采样 ≥95 tick 且增长至收尾）；② bg 交接 + 任务
  在子进程存活期间自然跑满；③ `tool_running(status=idle_killed)` journal
  事件。零 API（`ORZ_FAKE_SCENARIO` 驱动、无 `--real`）。产物
  `_windows_high_nist/job-w1-fake-batch.ps1` + `evidence-w1-20260907/`
  （3 journal + bg 心跳日志 + sandbox observations + 断言 JSON）。
  （边界：完成提醒文本仅模型会话面注入、不落持久面，fake harness 下
  不可直接断言——提醒可达性判据归 W2 真实模型面 §7 C 组。）
- [x] T2.4 同步与接线（2026-09-07 完成，S3/S4 集中实机验证批 T0c）：
  M1 新 Windows 三件套 0.3.1 进 VM（copy_to_vm 15/15 哈希核验 + Program
  Files 换装备份 + C:\s4\tools）+ keystore/signer 复检（0.3.1 provision
  重刷 C:\workspace\acaf manifest，signer sha256 自校验通过）+ runner 配置
  收敛（不再补开 S5-2）。验收过：vm-agent DryRun 全对（AGENT_RUN_OK=True /
  ERRORS=0）+ enforcement-probe 三臂墙内全绿（control exit 0 / non-admin
  9/9 / high-nist 19/19，SYSTEM 提权作业通道）。证据
  `_windows_high_nist/evidence-t0-restore-20260907/`。

## 审查处理（2026-09-04）

- [x] TER M1/M2 全面审查处理：P1-1 idle-kill journal 生产者补入＋单测
  （orz `35db6741`）；P1-2 ADR-0010 §14.55 正式裁决（转正/取代清单）；
  P2 × 5 与 P3 ~24 逐项处理/登记（含 no-AC 生产墙、10h 例外、push 次数
  口径、completed elapsed 冻结、live-capable 标注、命中行 4K 钳制、DNS
  fail-fast、fake loader fail-closed、文案残留清理）。验证：orz-tools
  output_object 5 / idle 2 / description 97、orz-loop idle-kill 1、
  orz-host mapping 1、orz-bin loader 1 + cargo check 全绿、dns_refusal
  selftest/fail-fast OK。审计：
  [`TER_M1_M2_COMPREHENSIVE_REVIEW_2026-09-04.md`](docs/audits/TER_M1_M2_COMPREHENSIVE_REVIEW_2026-09-04.md)
  + [`TER_REVIEW_HANDLING_2026-09-04.md`](docs/audits/TER_REVIEW_HANDLING_2026-09-04.md)。

## M3 回归复验

- [ ] T3.1 gcode 复跑：300s 渲染 180s 收中间状态并后台化；无输出则
  300s 处 idle-kill+提醒；事件链含 tool_running。验收：journal 证据。
- [ ] T3.2 make-doom 复跑：无 840 硬杀；连通探测 ≤2s/目标收敛；进程区
  可见运行中探测并可 kill。验收：journal + 观察记录。
- [ ] T3.3 mteb 复跑 + F6 push 档：HF 探测 ≤2s 判定失败；push ≤4
  次/run + `budget_cue_injected` 可审。验收：journal 证据。
- [ ] T3.4 W-F13 复验：vm.js 阅读轮数 20+→≤2；长输出补读不再触 .gsa。
  验收：轮数统计对比。
- [ ] T3.5 §7 判据 1–6 复验登记 + ADR/BACKLOG2/TODO2 收口。验收：
  S4_PROGRESS 登记 + 勾选树闭合 + 设计稿状态更新。

## 首轮明确不做（防膨胀）

- 15min 长档（视 180s 实跑数据再定）。
- 待审圈系统审计（web/检索/浏览器/console 600s/其它默认面）。
- fold 后任务状态摘要（折叠重建，单列候选）。
