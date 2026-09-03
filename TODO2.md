# TODO2 — 工具执行层改革（TER）实施步骤

> 用途：TER（Tool Execution Layer Reform）专用分步实施勾选树；主
> [TODO.md](TODO.md) 只保留指针，本文为 TER 明细权威。
> 上级：docs/BACKLOG2.md（TER 开放项）/ 设计稿
> [TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md](docs/TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md)。
> 分步原则：每一步都是最小可验收单元（改代码 + 单测 + 验收），可随时
> 停；里程碑 M0–M3 各有一个放行门；步骤间按依赖排序，无依赖的步可单独
> 抽做但须过各自验收。

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
- [ ] T1.6 黑板 processes live 分区：读取时现算快照（task_id/命令/
  elapsed/状态/活跃度/字节/可 kill）；状态跃迁落事件；kill 动作模型面
  可达且不暴露 `&`。验收：live 渲染单测 + 越权边界用例。
- [ ] T1.7 轮预算默认无限制：max_tool_rounds 默认移除（“120 per turn”
  静态文案动态化/移除）；保留可配上限逃生阀与 budget_insufficient。
  验收：默认运行无 120 拦截；显式配置上限时原机制仍生效。
- [ ] T1.8 F6 pull 面：session（或 processes）补 wallclock
  （elapsed/limit/remaining）；blackboard_read 返回含时间轴。验收：
  渲染单测 + 越权边界。
- [ ] T1.9 F6 push 档：阈值 <600/300/120s 注入中性事实 ≤4 次/run +
  `budget_cue_injected` 事件；开关 env/config 默认 off。验收：开关
  off 零注入（回归 PUSH→PULL）；on 时次数上限与事件可审。
- [ ] T1.10 W-F13a read_file 64KB 档：限制链核对（粗门 clamp/行 limit/
  50K 注入预算）后放宽至 64KB 级。验收：vm.js 级文件 ≤2 次读完；
  单次注入不触发截断；大文件仍可结构化分段。
- [ ] T1.11 W-F13b 输出检索对象：长输出落盘对象 + pattern/行区间/尾部
  N 行检索；ToolCompleted 截断标记 + 对象指针（schema 已 T0.2 定稿）。
  验收：检索语义单测；模型无需 .gsa 即可补读自身输出。
- [ ] T1.12 W-F11 环境快照：probe 扩展至代码工具环境（工具/语言/包/
  版本/连通判定，Linux 先本地快速判定）；落黑板 `section=env`（PULL）。
  验收：快照 ≤5s；env 分区 PULL 渲染 + 越权边界。
- [ ] T1.13 M1 验收：orz-loop/orz-assurance 全量测试绿 + clippy/fmt 净
  + Linux 三件套构建成功 + 事件链 verifier 过新 schema。放行进入 M2。

## M2 Windows runner / VM（依赖 M1 新 build）

- [ ] T2.1 墙钟单一化：删 runner `--max-wallclock` 与 60s 余量；
  sandbox `--timeout`=官方 agent_timeout_seconds 为唯一评测墙钟；F6
  push 读源以 runner 施加值为准。验收：DryRun 确认无 840 参数、逐题
  timeout 生效。
- [ ] T2.2 W-F12 本地透明层：先测基准（当前墙外 Test-NetConnection /
  curl / Invoke-WebRequest / python requests 各自失败耗时表）→ 实现
  DNS/TCP 本地拒答 → 复测 ≤2s/目标；allowlist 内连通不变。验收：mteb
  HF 探测 ≤2s、连通扫描总成本 ≤10s。
- [ ] T2.3 Windows 后台任务存活验证：auto-bg 进程在 Job/LOW IL 下跨
  调用存活、输出持续落盘、完成提醒与 idle-kill 可达（实机）。验收：
  单测式实机脚本证据。
- [ ] T2.4 同步与接线：M1 新 Windows 三件套进 VM（Program Files +
  C:\s4\tools + keystore/signer 复检）；runner 配置收敛（不再补开
  S5-2）。验收：vm-agent DryRun + enforcement-probe 墙内全绿。

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
