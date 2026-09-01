# 终端命令超时与 web 搜索超时设计调研（2026-08-29）

> 性质：调研笔记（S4 失败画像 §9.5 的后续输入）。S4 深挖显示 18 个
> AgentTimeout 题中 12 个最慢调用为 web 类工具（web_search 单次最高
> 1365s、客户端无超时）、6 个为 run_terminal 慢命令（apt 装 R 458s、
> 训练 1000s、全盘 grep 卡 14.5min 等）。本文对比官方 harness 与成熟
> 产品在终端命令超时与 web 搜索超时上的设计，评估 2026-08-29 用户
> 构想（程序/脚本默认 10 分钟 + 5 分钟中间回报一次），待用户裁决后
> 再形成设计。

## 1. 终端命令超时：各家设计对比

| 产品 | 默认超时 | 长任务机制 | 运行中反馈给模型 |
|---|---|---|---|
| OpenAI Codex CLI | `exec_timeout_ms` 默认 **10s**（普通命令） | **后台终端**（模型主动 spawn/status/read/kill；空窗口轮询上限 300s/5min） | 模型主动查 status，无机械主动回报 |
| Claude Code | bash 默认 **2min**；模型可传 timeout 参数 ≤ **10min**（`BASH_DEFAULT_TIMEOUT_MS` / `BASH_MAX_TIMEOUT_MS`） | `run_in_background` 后台任务 | 无模型侧进度回报（仅用户 UI spinner；#82741 请求中） |
| OpenHands | **soft timeout 10s**（无新输出触发）→ 模型可选 continue/interrupt；**hard timeout 默认 120s**（杀命令返回 HARD_TIMEOUT） | — | soft timeout 把中间状态返回给模型，模型确认继续或中断（可反复触发） |
| Aider | subprocess 默认 300s（`AIDER_TIMEOUT`） | — | 无 |
| terminal-bench 官方 | 任务级 `[agent] timeout_sec` 900–3600s；**无单命令超时规范**（agent 实现各自处理） | — | 无 |
| orz（现状 5b3fe27） | `run_terminal_cmd` 缺省/模型可传上限均 900s；前台另有 300s `MAX_FOREGROUND_BLOCK` 钳制；`enabled_background` 禁用 | 无后台终端 | 无（完成才返回） |

要点：

- **短前台 + 长任务后台**（Codex）：普通命令默认极短（10s），超时自动/显式转
  后台终端，模型用 status 轮询（窗口 5min）。适合交互式 CLI 场景，代价是
  工具面多一套后台终端管理。
- **分层超时 + 中间状态**（OpenHands）：soft timeout（无新输出 10s）先返回
  中间状态给模型，模型选 continue/interrupt；hard timeout（120s）兜底杀。
  与用户构想最接近——区别是 OpenHands 按"无新输出"触发且可反复，用户构想
  按"运行满 5 分钟"触发且**单次只报一次**（更克制，符合 P2 观察与动作分离
  与「新增机制一律视为债务」）。
- **默认 2min / 上限 10min**（Claude）：模型可显式要求长命令超时，机械层
  给上限。是"模型可传 timeout"的成熟先例。

## 2. web 搜索超时：后端延迟形态与业界建议

### 2.1 DeepSeek `/responses` web_search（本项目实际后端）

- DeepSeek 官方文档（create-response）：`web_search` 为**服务端执行的内置
  工具**（tools 必须包含否则 400），服务端做多轮搜索（search → open pages
  → re-search）。
- 社区实测（deepseek-web-search skill）：**单次调用 15–70s 属正常**，
  "不要把延迟当失败"，脚本建议允许 120s。
- 本项目现状：`web_search` reqwest 客户端**未配置任何超时**（client.rs
  `reqwest::Client::builder()` 无 `.timeout()`），S4 实测单次 50–1365s；
  1365s（mteb-leaderboard）属异常挂起，但 120s 内正常返回的搜索占比
  不低——超时定太短会频繁误杀正常搜索。

### 2.2 业界建议

| 后端/产品 | 典型延迟 | 建议客户端超时 |
|---|---|---|
| OpenAI web_search_preview | 查询增加 ~38s；社区实测 1.5–3min 常见 | 官方建议 timeout=30s；社区反馈默认 2–3min 仍会超时 |
| DeepSeek /responses web_search | 15–70s 正常（服务端多轮） | 社区建议 120s 上限 |
| 通用 metasearch | 300ms–15s | 15s，超时返回 TIMEOUT 无部分结果 |
| 本项目 orz | 无超时（实测单次最高 1365s） | **待定（建议 120–180s 上限 + connect 10s + 结构化超时错误）** |

要点：

- 搜索超时必须与后端延迟匹配：DeepSeek 服务端多轮搜索天然慢，30s 级
  （OpenAI 口径）会频繁误杀；120s 为社区实测共识下限。
- 超时后的行为比超时值更重要：应返回结构化错误（step/code/message +
  建议：重试/换查询/走 web_fetch 直读），而不是挂死或空结果。
- DeepSeek 官方 dsh-tool-web 的做法是**超时预算作为工具契约**
  （ToolDefinition.timeoutMs 机械声明，模型面不暴露超时控制）——与本项目
  「契约下沉工具面、不进 prompt」方向一致。

## 3. 用户构想评估（2026-08-29）

构想原文要点：程序/脚本运行默认 10 分钟；其他命令满 5 分钟向模型返回一次
「运行 + 工具自身情况」；单次运行只返回一次（不累积）；只让模型确认一次
运行正常；运行完成后再正常返回。

### 3.1 与业界对照

- 默认 10 分钟 ≈ Claude 的模型可要求上限（10min）；比 Codex 10s 前台宽松
  得多、比 OpenHands hard 120s 宽松。对 TB2 编译/训练/安装类任务（apt 装
  R 458s、训练 1000s）10 分钟是合理档位。
- 5 分钟中间回报 ≈ OpenHands soft timeout 的「模型确认继续」，但：
  - 触发条件不同：OpenHands 按「无新输出 10s」，用户构想按「满 5 分钟」；
  - 频次不同：OpenHands 可反复触发，用户构想单次只报一次（更省 token、
    不打扰模型主线）；
  - 确认方式待定：OpenHands 强制模型选 continue/interrupt；用户构想是
    「确认一次运行正常即可」——可设计为回报后模型显式继续（多一轮交互）
    或默认继续+可中断（少一轮交互）。
- 「运行 + 工具自身情况」= 机械自解释契约（P3/P4）：回报内容建议含运行
  时长、进程状态（运行/无输出）、最近输出活跃度、落盘指针，让模型能判断
  「正常在跑 vs 疑似挂死」。

### 3.2 建议方案（待裁决）

1. **分层默认超时**：普通命令默认 300s（与回报点一致）；程序/脚本类默认
   600s（用户 10 分钟）；模型可传 `timeout` 覆盖（上限 900s，维持现状
   `max_timeout_secs`）。"程序/脚本"的机械判定待定（可先按命令形态启发式
   + 模型显式标记，或先统一 600s 不做分级）。
2. **5 分钟中间回报**：任何命令运行满 300s 且未完成 → 机械插入一条
   `tool_running` 中间状态（运行时长/进程状态/输出活跃度/指针），单次仅
   一次；回报后默认继续等待，模型可显式中断（或显式确认继续，二选一待
   裁决）；命令完成/超时后正常返回终态。回报内容与格式走机械自解释契约，
   不进 prompt。
3. **web_search 超时**：客户端设总超时（建议 120s）+ connect 10s；超时
   返回结构化错误（含已用时长、建议重试/换查询），不挂死；重试策略沿用
   transport 既有纪律（不新增自动重试机制，除非设计论证必要性）。
4. 事件面：`tool_running` 中间事件（S5-2 落地）+ `tool_completed` 补
   wall_ms/超时标记——wall_ms 与 timed_out 已随 S5-1 落地（2026-08-29，
   schema + 控制器通用/run_tests 路径；run_tests F-09 超时经结构化
   `TestRunResult.timed_out` 端到端透传，审查处理 P2-1），供审计与失败
   画像。

## 4. 裁决点（2026-08-29 已全部拍板，见 §5）

- 普通命令默认超时值（300s？120s？）与是否做「程序/脚本 vs 普通」分级；
- 中间回报后：默认继续 + 可中断，还是强制模型显式确认继续；
- 中间回报的触发点（满 300s 一次性 vs 每 300s）——用户倾向一次性，确认；
- web_search 超时上限（120s vs 180s）与超时后行为（重试/换查询/直读）；
- 是否本轮一并实施（与 A/B 修复同批，还是 A/B 先行）。

## 5. 定案（2026-08-29 用户裁决，已登记设计 §9.6 / TODO W4-R4 S5-1/S5-2）

- **终端命令分层默认超时（两档）**：普通命令 300s；程序/脚本类 600s。
  模型可传 `timeout` 覆盖，上限维持 900s（现 `max_timeout_secs`）。
- **5 分钟中间回报**：任何命令运行满 300s 未完成 → 机械插入一次
  「运行 + 工具自身情况」中间状态（运行时长/进程状态/输出活跃度/落盘
  指针）；单次仅一次（不累积、不周期）；回报后**默认继续等待**，模型可
  主动中断（kill），由模型自行判断；命令完成/超时后正常返回终态。
- **web_search 超时**：客户端总超时 120s + connect 10s；超时返回结构化
  错误（已用时长 + 建议重试/换查询/直读页面），不新增自动重试。
- **实施批次（压力评估定案，分两批）**：
  - **S5-1（轻量防御性修复，同批）**：问题 A（fold 桥 reasoning 保留）+
    问题 B（orientation 触发轮放行工具）+ web_search 120s 超时；一次
    重建 + 复验。
  - **S5-2（新机制，独立批）**：终端分层超时 + 中间回报——当前
    `run_terminal_cmd` 前台阻塞执行、`enabled_background` 禁用，中间
    回报需要后台/部分结果注入路径（mid-run 模型可见消息 + 中断 + 终态
    送达），属新机制；实施前先补设计小节定案（程序/脚本分类规则 /
    后台路径 / 事件面）。

## 6. S5-2 机制定案补充（2026-08-29，设计 §9.7）

- **分类规则**：宿主按命令形态逐调用注入默认 `timeout`——程序/脚本类
  600s、普通命令 300s（首 token 解释器/构建工具集合 + 脚本扩展名 +
  `./` 前缀启发式；模型显式 timeout 覆盖，上限 900s）。
- **后台路径**：复用终端 actor 自动后台化——命令满 300s 且解析超时
  >300s 时转入后台并返回一次「运行 + 工具自身情况」中间状态（时长/
  PID/输出活跃度/落盘指针），默认继续、可按 PID 中断、终态随下一次
  工具结果经既有完成提醒送达；后台任务截止=原解析超时（
  `min(原 timeout, BACKGROUND_MAX_RUNTIME)`）。工具面封闭
  （`&` 拒绝 + `is_background` 隐藏/拒绝）。
- **事件面**：新增 v0.2 `tool_running`（ToolStarted 与 ToolCompleted
  之间、同一 call_id 至多一条），承载 wall_ms/pid/total_bytes/
  output_file/task_id；对应 ToolCompleted 落 exit_code=null +
  `running: true`。
