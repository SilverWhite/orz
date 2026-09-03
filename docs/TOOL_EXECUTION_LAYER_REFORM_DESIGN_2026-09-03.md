# 工具执行层改革设计（去硬杀 + 常驻默认 + 环境可判定 + 阅读面大修）

> 日期：2026-09-03；状态：**设计稿（未实施；2026-09-03 经 TODO2 T0.3
> 登记为 ADR-0010 §14.53 候选项，M1–M3 放行后转正式裁决）**；范围：orz
> 主线工具执行层默认行为 + Windows high-nist 评测 runner 接线 + VM 环境侧
> 改造。上游裁决：2026-09-03 用户逐条确认（见 §2 与 §9 决策记录）。

## 1. 背景与证据

chunk1-0303（0.3.0，high-nist）三题 2/3 死于 840s 墙钟；chunk1-f4 同型复现；
官方 Linux recheck（OFFICIAL_R2_FAILURES_RECHECK_10T_2026-08-31）6 个超时题根因
为“总工作量超出官方墙钟”。逐轮重建（`_windows_high_nist/analysis/`，
probe/transcript/breakdown 产物）给出四类摩擦：

1. **隐形硬墙钟**：runner 自加 `--max-wallclock 840`（900-60 余量），模型全程
   不可见；make-doom/gcode 均在被掐前仍读文件/探测，810s+ 处无收尾。
2. **工具执行层硬杀**：auto-background 未开启时，普通命令 300s / 程序脚本
   600s / 模型上限 900s 满即杀；gcode 300s 纯 python 渲染被杀、零中间回报；
   make-doom 196s Test-NetConnection 盲等（低于 300s 首报点，无任何回报）。
3. **环境不可判定 + 信息黑洞**：防火墙 blockoutbound 静默丢包，墙外连接只见
   超时无“策略拒绝”；模型把超时当瞬时故障换目标重试（8 targets×4 ports
   =196s、pip install 21s、HF HEAD 12.5s）。
4. **阅读面摩擦**：vm.js 66KB 分 20+ 轮小块读（每轮 5–10s 模型延迟）；
   长命令输出截断后模型经 .gsa 自读补回（gcode 7–8 轮、涉及 4+ 条命令日志）；
   make-doom 在 fold 后读 ledger 重建工作记忆（另加编码困惑）。

## 2. 设计原则（用户裁决，2026-09-03）

- **P1 orz 独立可用**：orz 中已有的常驻能力必须自身默认开启，不得依赖外部
  harness 额外配置；orz 独立（无 harness）即应完整可用。
- **P2 只留官方评测要求的墙钟**：我们自加的硬杀墙钟（orz 内部
  `--max-wallclock`、runner 余量）全部删除；官方任务 agent_timeout_seconds
  作为评测墙钟保留（执行点在 runner/sandbox）。
- **P3 工具执行层无自身硬超时**：超阈值自动后台化 + 一次提醒，不杀；
  进程注册进黑板 live 视图，模型可自主等待/kill/忽略；机械兜底=5 分钟无
  实际活跃（无输出增长）即杀并返回提醒。
- **P4 压测信息最小化**：不把“须知环境”（allowlist、工具链缺失等）注入
  模型；环境侧改为快速确定性失败；通用环境快照以 PULL 查询面提供。
- **P5 预算可见性做成三档开关**（off/pull/push，默认 off），日常无感，
  评测/限时场景开启。
- **P6 阅读面大修**：放宽 read_file 单次限制以降低轮数；把“自己命令的完整
  输出”做成正式可检索对象；截断显式标记。折叠重建需求单列，不并入。

## 3. 逐项设计

### 3.1 F7：工具执行层去硬杀 + 自动后台化（orz 主线默认）

**行为状态机（单条 run_terminal_cmd 调用）**：

1. 前台执行，模型等待；到 `foreground_block_budget_ms`（默认 180s）仍运行 →
   机械层自动后台化，返回一次中间状态（“仍在运行 + PID + 已耗时 + 输出
   活跃度”），模型拿到返回后可继续其他工作。
2. 后台任务继续运行；状态进黑板 processes live 视图。
3. 完成 → 经 `surface_bg_completion_reminders` 在下一个工具结果顶部带回
   完整结果（一次延迟回报，模型感知“180s 提醒一次 → 结束带回”）。
4. 兜底：后台/前台进程连续 300s（5min）无输出活跃 → 机械层 kill 并返回
   “idle killed + 原因”提醒；无硬超时杀活跃进程。

**活跃判定（兜底口径）**：输出字节增长为主 + **CPU 活跃辅助**——5min 内
输出字节无增长且 CPU 时间基本不增才判 idle kill；计算密集但无输出（编译/
训练/渲染内核）不误杀。

**参数默认值（orz 独立默认，含评测 runner 不再额外开启）**：

| 参数 | 现状默认 | 新默认 | 说明 |
|---|---|---|---|
| `enabled_background` | true | true（不变） | 结构默认已 true |
| `auto_background_on_timeout` | false | **true** | 常驻 |
| `foreground_block_budget_ms` | None（后端 15s） | **180_000** | 首报/后台化点（用户：3min） |
| `hide_background_input` | false | **true** | 模型面封闭显式后台，一次调用一次结果 |
| `surface_bg_completion_reminders` | true | true（验证生效） | 后台完成带回 |
| `timeout_secs` / `max_timeout_secs` | 300/600/900 分层 | **去除硬杀语义** | 仅作后台化 deadline/上限引用 |

实现核对项：orz 工具 schema 侧与 BashParams 结构默认值存在不一致风险
（schema 展示 300s/struct 后端 15s），落地时以单一生效源收敛；确认 15s
短预算（GROK env）在 orz 面被 orz 默认覆盖为 180s。

**档位扩展（远期，不在首轮实施）**：若 180s（3min）对长运行任务过短，增加
15min 长档；形态=参数化分档（如 180s/15min 两档，按任务/会话配置选择），
默认仍 180s。

### 3.2 进程注册表：黑板 processes live 视图

- 新增黑板分区 `section=processes`（live-only，同 session 面先例）：读取时
  现算快照，内容含 task_id / 命令摘要 / elapsed / 状态（running | idle |
  waiting_input | completed | killed）/ 输出活跃度与累计字节 / 是否可 kill。
- 新鲜度保证 ≤1s（读取时现算，非每秒写事件）；状态跃迁（start /
  auto-background / complete / kill / 进入 idle 首现）才落 journal 事件。
- kill 语义：模型面经现有后台生命周期工具或等价动作（与 3.1 的模型面
  封闭并存，具体形态实现核对）发起；被墙模型面不暴露 `&`/`is_background`。
- 与 10MiB 黑板水位的关系：live 视图只占用渲染期临时空间，不累积事件流。

### 3.3 F6：预算可见性三档（默认 off）

- `off`（默认）：零注入，日常与现状一致。
- `pull`：blackboard `section=session`（或 processes）补 wallclock 面
  （elapsed / limit / remaining / rounds used），按需查询。
- `push`（评测/限时开）：剩余预算跨阈值（<600 / <300 / <120s）机械注入
  中性事实，上限 3–4 次/run，只报剩余不附建议。
- 评测墙钟来源 = 官方 task agent_timeout_seconds（900/3600…），由
  runner/sandbox 施加；orz 内部自减余量逻辑（840）删除。

### 3.8 轮预算撤除（120 tool rounds 默认硬限）

- 现状：max_tool_rounds=120，budget_insufficient 预检硬拒绝 + exhaustion
  块 + run_invalidated，系统提示声明“120 tool rounds per turn”。
- 用户意见：撤下去——评测墙钟已兜底，120 轮对“准确性优先”的长任务无
  必要。**评判（Codex）**：同意撤默认硬限；建议保留“可配置上限”逃生阀与
  budget_insufficient 机制（仅显式配置非零上限时生效），防止未来某次放行
  后出现无界重试循环时无机械闸。进程侧失控已由 5min idle+CPU 兜底覆盖，
  模型侧无进展循环仍由 orientation 软门/复读检测等软机制处理，不新增护栏。
- 定案：默认 max_tool_rounds 无限制（移除“120 per turn”静态文案，改动态/
  移除）；保留可配上限逃生阀；评测不额外设轮上限（墙钟为准）。

### 3.4 W-F11：机械层环境快照（通用查询面，非 brief 注入）

- 范围：工具/语言/包/版本的存在性与版本、关键数据/输入是否在场、连通性
  判定（通/不通，≤1–2s 每项）。
- 形态：复用/扩展既有 tool_probe 类探测为“代码工具环境快照”；结果落黑板
  `section=env` 白名单区（PULL，模型主动查）；快照生成 ≤5s。
- 明确不做：把“任务专属结论”（无 mips 工具链、只放行某 IP 等）文本注入
  首轮——压测语义保持“未知限制下决策”，此类结论只用于搭建侧任务可行性与
  装配，不喂模型。
- 快照“快”的前提是 3.5（W-F12）落地。

### 3.5 W-F12：环境侧快速确定性失败（动环境，不告知）

- 目标：allowlist 外目标 1–2s 内返回可判定失败（连接拒绝/DNS 拒答），
  不再黑洞式挂到超时；allowlist 内部连通不变。
- 实现候选（Windows）：本地 DNS 拒答（allowlist 外域名立即 NXDOMAIN）+
  TCP 出站快速拒答（防火墙无法表达 RST，需 WFP 自定义或本机透明策略层）；
  需覆盖任意客户端（Test-NetConnection / curl / Invoke-WebRequest /
  python requests）与协议族；以实测为准选择实现层。
- 语义边界：失败可判定不泄露 allowlist 内容（模型仍逐个试，但每次成本
  ≤1–2s，试错可收敛）；保留 drop 的信息面设计取舍（不对外暴露）。
- 验证点：mteb HF 探测应 ≤2s；make-doom 全盘连通扫描总成本应 ≤10s。

### 3.6 W-F13：阅读面大修

- **read_file 单次有效返回放宽**：目标 64KB 量级（vm.js 一次/两次读完），
  核对限制链=粗门 clamp（8–32KB）与单轮注入预算（50K token）的耦合，按
  “放宽后仍不爆注入预算”取档；行数 limit 语义保留（结构化跳读仍然成立）。
- **命令输出正式检索对象**：每条长输出落盘为对象，提供 pattern / 行区间 /
  尾部 N 行检索语义，替代模型逐段 read_file + .gsa 摸黑；输出可到 MB 级。
- **截断标记**：ToolCompleted 显式 output_truncated + total_bytes +
  指向检索对象。
- **单列项（不在 W-F13）**：fold 后任务状态摘要（解决 make-doom fold 后读
  ledger 重建记忆 + 编码困惑），并入既有 compaction/黑板议题。

### 3.7 常驻能力默认化审计（P1）

已核实的执行层清单：

| 能力 | 依赖 harness？ | 处置 |
|---|---|---|
| S5-2 auto-background / 中间回报 | 是（默认 false） | 默认 true（3.1） |
| 前台短预算 180s 首报 | 是（None→15s 后端） | orz 默认 180s |
| 后台完成带回（surface reminders） | 默认 true 但未验证生效 | 验证闭环（3.1） |
| 后台生命周期工具进 agent 工具面 | 否/视装配 | 与 3.1/3.2 一并核对 |

待系统审计的其它圈（本稿范围外，另列核查协议）：web/检索/浏览器配置默认、
read_file gate 与注入预算默认链、console 层命令默认超时（600s）、后台任务
Windows 沙箱（Job/LOW IL）跨调用存活与输出落盘。

## 4. 与既有机制衔接

- S5-2（THIN-HARNESS-REDESIGN-V2 §9.7）：本稿是 S5-2 的“常驻默认 + 去硬杀”
  演进，语义兼容；tool_running/journal 事件沿用。
- PUSH→PULL（ADR §14.35 第 10–12 项）：F6 push 为可开关例外；进程/环境面均
  PULL；零常驻注入纪律保持。
- F10/.gsa 口径：W-F13 把“读自己输出”正规化后，模型对 .gsa 的自读动机
  降低；不引入 .gsa 写保护（沿用 2026-08-31 裁决）。
- Windows 沙箱：评测墙钟由 sandbox `--timeout` 执行；`--max-wallclock`
  删除；AppContainer off 与 allowlist 语义不变。

## 5. 待实现核对项 / 风险

1. schema 默认（300s）与 BashParams struct 默认（false/None/15s 后端）来源
   收敛为单一生效源。
2. auto-background 后 Windows 后台进程在 Job 对象/LOW IL 下跨调用存活、
   输出持续落盘、完成提醒可达（需实机验证）。
3. 5min 无活跃判定口径：输出字节增长为主，CPU 为辅；防误杀“计算密集但无
   输出”命令（参数化 idle 判定，可加 CPU 阈值）。
4. F13 放宽后 read_file 大返回与 50K 注入预算的截断边界需压测（选 64KB 档
   后验证单次注入不触发截断）。
5. 评测 runner：删除 840 后，sandbox `--timeout`=官方值直接生效；F6 push
   只读剩余预算不读错源（以 runner 施加值为准）。

## 6. 验证计划（复验点）

1. gcode 复跑：300s 纯 python 渲染在 180s 收到中间状态并后台化；若其后
   无输出增长，300s 处（5min idle 兜底）被 kill 并返回提醒；事件链出现
   tool_running。
2. make-doom 复跑：无 840 硬杀；连通扫描因 W-F12 ≤2s/目标而收敛；进程区
   可见运行中探测并可 kill。
3. mteb 复跑：HF 探测 ≤2s 判定失败；模型不再盲试。
4. F6 push 档复验：剩余阈值注入 ≤4 次/run，事件类型
   budget_cue_injected 可审。
5. W-F13 复验：vm.js 阅读轮数从 20+ 降至 ≤2；长输出补读不再触 .gsa。

## 7. 待裁决参数

**已定案（2026-09-03 用户拍板）**：

| 参数 | 定案 |
|---|---|
| 首报/后台化点 | 180s 为 orz 主线默认；远期若不足，增加 15min 长档 |
| 5min idle 兜底 | 加 CPU 活跃辅助判定 |
| 轮预算 | 撤默认硬限（120→无限制），保留可配上限逃生阀 |
| W-F12 实现层 | 先做本地透明层 |
| F13 read_file 档位 | 暂定 64KB，按此实施后压测复核 |

## 8. 证据与入口

- 本轮分析：`_windows_high_nist/S4_PROGRESS_2026-09-02.md` §16.17/§16.18、
  `_windows_high_nist/analysis/`（probe/transcript/breakdown）。
- 官方对照：`docs/audits/OFFICIAL_R2_FAILURES_RECHECK_10T_2026-08-31.md`。
- 既有机制：`THIN_HARNESS_REDESIGN_V2_DESIGN_2026-08-28.md`（S5-2）、
  ADR-0010 §14.35 第 10–12 项（PUSH→PULL）。

## 9. 决策记录（2026-09-03 用户裁决）

1. F6 按三档（off/pull/push）做，默认 off，日常重准确性。
2. 包完整性：机制在代码内，判定为“接线缺失”，按 P1 默认化修复。
3. 仅留官方评测墙钟；orz/runner 自加硬杀墙钟全部删除。
4. 工具层无硬超时；5min 无活跃机械杀 + 提醒；进程注册表 live 视图同意；
   评测墙钟可留。
5. F11 不做 brief 注入，改为机械层环境快照（PULL 白名单区）治本。
6. F12 动环境侧：快速确定性失败，不给模型 allowlist 事实。
7. F13 直接大修（read_file 放宽 + 输出检索对象 + 截断标记）。
8. 常驻能力默认化（P1）作为本次横切原则。
9. （2026-09-03 拍板）应该常驻的都打开；180s 先用作默认，远期分档加
   15min；5min 兜底含 CPU 活跃辅助。
10. 轮预算：撤默认 120 硬限（采纳 Codex 折中：默认无限制 + 可配上限
    逃生阀）。
11. W-F12 先做本地透明层。
12. F13 先按 64KB 档实施。

## 10. 实施阶段设计（S0–S3）

> 放行纪律：动代码前先完成 S0（机器合约先行）与阶段门；每阶段验收全绿
> 才进下一阶段；S1 为 orz 主线（独立可用优先），S2 为 Windows
> runner/VM（依赖 S1 新 build），S3 为回归复验。

### S0 机器合约先行（实施前设计门）

- 事件面/schema 变更清单定稿：① 新事件 `budget_cue_injected`（F6 push，
  载荷=剩余秒/轮/触发档位）；② `tool_completed` 增可选字段
  `output_truncated` / `total_bytes` / `output_object_id`（F13）；③
  后台生命周期状态跃迁事件沿用 tool_running，补 idle-kill 形态
  （status=idle_killed + reason）；④ 黑板 `section=processes` /
  `section=env` live 渲染契约与越权边界（对齐 session 面先例）。
- schema/verifier/fixtures 先行（项目纪律：机器合约先于实现）；
  orz-assurance 对应校验分支与测试枚举先行。
- 默认值收敛核对：BashParams struct default / tool schema 默认 /
  后端 15s（GROK env）三处来源收敛为单一生效源（auto_background=true、
  budget=180s、hide_background_input=true）；产出核对表。
- ADR/BACKLOG/TODO 登记（本设计稿转 ADR 候选 §14.xx 项，BACKLOG 0l
  开放项，TODO P0-0l 勾选树）。
- 产出：S0 核对表 + schema diff + 放行签名（不写业务代码）。

### S1 orz 主线（独立可用优先；Linux 单测/构建闭环）

工作包（每包=代码 + 单测 + 验收）：

1. **S5-2 常驻默认**：bash params 默认收敛（auto_background_on_timeout
   =true、foreground_block_budget_ms=180_000、hide_background_input
   =true）；schema 展示与生效值一致。
2. **去硬杀语义**：run_terminal_cmd timeout 分层（普通/程序/模型上限）
   不再作为杀进程点；超限行为改为 auto-bg deadline 引用；timed_out
   形态仅保留于 idle-kill 与评测墙钟。
3. **idle+CPU 兜底**：进程监视采样（输出字节增长 + CPU 时间）；连续
   300s 无活跃 → kill + 提醒（idle_killed + reason 入事件）；阈值参数化。
4. **黑板 processes live 分区**：读取时现算（task_id/命令/elapsed/状态/
   活跃度/字节/可 kill）；跃迁落事件；kill 动作模型面可达且不暴露
   `&`/is_background。
5. **轮预算默认无限制**：max_tool_rounds 默认移除（“120 per turn”静态
   文案动态化/移除）；保留可配上限逃生阀与 budget_insufficient 机制。
6. **F6 三档**：session/processes 面补 wallclock（elapsed/limit/
   remaining）；push 档阈值 <600/300/120s 注入 ≤4 次/run +
   `budget_cue_injected` 事件；开关 env/config，默认 off。
7. **W-F13a read_file 64KB 档**：限制链核对（粗门 clamp / 行 limit /
   50K 注入预算）后放宽至 64KB 级；压测单次注入不触发截断。
8. **W-F13b 输出检索对象**：长输出落盘对象 + pattern/行区间/尾部 N 行
   检索语义；ToolCompleted 截断标记 + 对象指针（schema 已 S0 定稿）；
   模型不再需要摸 .gsa。
9. **W-F11 环境快照（机制先行）**：probe 扩展至代码工具环境（工具/
   语言/包/版本/连通判定）；落黑板 `section=env`（PULL）；Linux 端
   先以快速本地判定闭环，Windows 快速性依赖 S2 F12。

验收：orz-loop/orz-assurance 测试全绿（新增覆盖各包）；clippy/fmt 干净；
Linux 三件套构建成功；事件链 verifier 过新 schema。

### S2 Windows runner / VM（依赖 S1 新 build）

1. **墙钟单一化**：runner 删除 `--max-wallclock` 与 60s 余量；sandbox
   `--timeout`=官方 agent_timeout_seconds 为唯一评测墙钟；F6 push 读源
   以 runner 施加值为准。
2. **W-F12 本地透明层**：allowlist 外 DNS 拒答 + TCP 拒答实现（先做本地
   透明层，实测后定）；allowlist 内连通不变；验证 mteb HF 探测 ≤2s、
   连通扫描总成本 ≤10s。
3. **Windows 后台任务存活验证**：auto-bg 进程在 Job/LOW IL 下跨调用
   存活、输出持续落盘、完成提醒与 idle-kill 可达（实机）。
4. **同步与接线**：S1 新 Windows 三件套进 VM（Program Files + C:\s4\tools
   + keystore/签名复检）；runner 配置随新默认收敛（不再需要 harness
   补开 S5-2）。

验收：VM DryRun + enforcement-probe 墙内全绿；三题复跑条件齐备。

### S3 回归复验

1. gcode 复跑：300s 渲染在 180s 收中间状态并后台化；无输出则 300s 处
   idle-kill+提醒；事件链出现 tool_running。
2. make-doom 复跑：无 840 硬杀；连通探测因 F12 收敛（≤2s/目标）；进程区
   可见运行中探测并可 kill。
3. mteb 复跑：HF 探测 ≤2s 判定失败；答案仍无 live 核验（预期，登记）。
4. F6 push 档复验：≤4 次/run 注入 + 事件可审。
5. W-F13 复验：vm.js 阅读轮数 20+ → ≤2；长输出补读不再触 .gsa。
6. §7 判据 1–6 复验并登记；S4_PROGRESS/BACKLOG/TODO 收口；ADR §14.xx
   登记。

### 明确不做（首轮范围外）

- 15min 长档（远期，视 180s 数据）。
- 待审圈系统审计（web/检索/浏览器/console 600s/其它默认面）——独立
  审计步骤，不在 S1–S3。
- fold 后任务状态摘要（折叠重建）——单列候选。
