# 295 批：0dh 通道选型对证（Grok 启用 vs Claude Code 借鉴）＋N3 撤回/中途补充外部调研（2026-10-17）

> 性质：**记录批**（只读调研＋设计基调登记；用户两问：①「那是启用Grok的即可还是借鉴claude code的？」
> ②「消息撤回/中途补充的设计我们也做外部调研吧……额外注意不打断模型当前动作这点，我们不做即时打断的那种补充」）。
> 零源码、零 orz 改动、零跑批；计数不变 64。

## §1 0dh 通道选型对证

**同构对照**（两套机制表层几乎同构＝「壳工具上一个显式参数＋读/杀伴随工具」）：

| 面 | Grok（在树，T1.3 封闭中） | Claude Code（业界成熟形态） |
|---|---|---|
| 声明参数 | `is_background`（schema 隐藏＋显式输入拒绝） | `run_in_background`（布尔，公开） |
| 读出 | `get_task_output`（整读 output_file，尾部截断缓冲）／`read_file` 同文件 | `BashOutput`/`TaskOutput`（**增量拉取**，自上次读点续） |
| 杀 | `kill_task`（requires_expr 强制与读出同台） | `KillShell`/`TaskStop` |
| 完成提醒 | `surface_bg_completion_reminders`（下一个工具结果顶部 system-reminder） | 无主动提醒（下次拉取时见） |
| 教学 | 描述模板一句话（「lifetime owned by the model via kill_task」） | 描述一句话＋参数自解释 |
| 生命周期 | 会话属主（现被两层收割覆盖） | 会话内跨轮、不跨会话 |

**选型建议（待用户拍板）**：**基座＝启用 Grok（`hide_background_input=false` 逃生阀），借鉴 Claude Code 三处，不另造新表层**。理由：两套同构、无实质形态差异；Grok 链已在树且与 orz 的 task_id/output_file/完成 reminder/双兜底管线咬合（零接线成本、零工具面新增）。

- **借鉴 a（教学面）**：Claude Code 的「描述一句话＋参数自解释」为教学上限——0dh 教学面三件照此收敛（本批 §3 同款原则）。
- **借鉴 b（增量读出）**：Claude Code 读出＝增量；orz 现役＝整读 output_file（尾部截断）。S1 权衡：增量游标（上次读点续读）vs 现状整读＋tail——长输出训练日志下增量显著省 token，倾向借鉴。
- **借鉴 c（生命周期边界显式化）**：Claude Code＝会话内跨轮、不跨会话；orz 按用户裁决走更远（跨 run 不收割＋30h）——S1 设计稿把这条边界写成显式声明，不沿别人口径收缩。

**不做**（两轮已定，无变化）：新工具、进程类型集、即时打断式通道。

## §2 N3 外部调研——中途补充（排队式）

- **Claude Code（形态标杆，与用户裁决完全同形）**：agent 运行中输入即**排队**，默认**不打断**当前 turn，在下一次 LLM 调用边界 flush 投递（GitHub #50246 确认 deferred 语义＝「appended and flushed at the next LLM call」）；可叠多条；Esc＝硬打断是**另一条独立通道**；社区指南术语对照＝steering（注入活跃 turn）vs interrupt（中止重开）vs queue（排队等边界）。开放 issue 另有人要求「priority mid-turn steering」（即时转向）与「Send-Now 键」（立即打断发送）——**均属用户明确不要的即时打断形态**。
- **Codex CLI**：Tab 队列＝消息等 agent 完成当前 turn 才发（同 queue 形态）。
- **Cursor**：反面教材——论坛实测「排队消息被处理时 agent 仍切过去丢下手头活」，被抱怨「只有打断没有 steering」。
- **对齐结论**：用户裁决「自动接在当前思考/动作结束后、不中途打断」＝**Claude Code/Codex 的 queue＋边界 flush 形态**；明确不做＝Esc/Send-Now/priority steering/Cursor 切走式。
- **orz 现状与设计稿要点**：web `submitPrompt` 在 `state.running` 时直接拒绝（无排队）；ACP `session/prompt` 单飞。S1 勘定项＝排队归属（web 前端 vs 桥 vs agent loop）＋flush 点定义（当前工具批结束/下一 LLM 调用前）＋ACP 并发 prompt 服务端语义＋排队中消息的可见面（web 状态行）。

## §3 N3 外部调研——消息撤回（recall）

- **Claude Code `/rewind`（checkpointing，业界最完整公开实现）**：Esc Esc 或 `/rewind` 开菜单；checkpoint 在每次 Claude 编辑前**自动捕获、绑定到触发它的用户消息**；可恢复 code／conversation／both；**恢复会话＝原 prompt 回输入框供编辑重发**——即业界主流「撤回」形态＝**编辑式重发（分支截断）**，与 ChatGPT/Claude.ai 网页「编辑历史消息→重发」同构。
- **诚实边界（官方文档承认）**：checkpoint 只追踪 Claude 的文件编辑，**bash 命令副作用不可回退**，git 才是真兜底。
- **对 orz 的映射（供设计裁决）**：①会话面＝`conversations/*.json` 天然 checkpoint 载体，截断至目标消息前即完成「撤回」；②上下文面＝messages 截断，同源同刻（与压缩折叠/存档三键机制对账）；③工作区文件面＝orz 无 checkpoint 系统，bash 副作用同样不可回退——沿 Claude Code 诚实边界「撤回只回退会话/上下文，文件面 git 兜底」，还是做文件 checkpoint，**留用户裁决**；④与 0dd（会话生命周期用户主权）衔接——撤回是用户主权面的一部分。

## §4 处置

- **0dh**：本批 §1 为 S1 设计基调补强（选型建议待用户拍板后进设计稿）；BACKLOG 0dh 节与索引 0dh 条目补 S0-2 指针。
- **N3（撤回/中途补充）**：调研落档维持 `candidate`（OBS-WEB-DOGFOOD 第三件），设计裁决待用户——两项独立裁决点：①补充通道排队归属与 flush 点；②撤回范围（仅会话/上下文 vs 连带文件 checkpoint）。
- 零源码、零跑批；本批未发起任何 orz 会话；计数不变 64。
