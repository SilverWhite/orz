# 框架时间预算语义审计（2026-09-13）

> 性质：只读调查 + 结论落档。本轮**不改代码、不改预算值、不改契约、不重建、不重跑**，也不触碰 `D:\AGI`。
>
> 修订 v1.1（2026-09-13，用户澄清）：**10 s = 检索请求发出后等"首个检索结果"的上限，不是检索任务的总时限**；总预算是另一本账（"总时间是总预算，10 s 与总预算不是同一个动作"）。C6 撤回（本轮全部重跑，已入档计数不再修）。D7 补充来源与机制；新增 §0.1、附录 C/D/E/F。
>
> 修订 v1.3（2026-09-13，用户裁决）：检索路径**选定流式优先**（分段为后备，通道判活+操作预算只作补充护栏）；**D7 改掉、新增机制**；**FP-2 不改、不是例外**；先完成设计、不动实现。设计稿：[`即时结果回报与流式检索设计`](../IMMEDIATE_RESULT_DELIVERY_AND_STREAMING_RETRIEVAL_DESIGN_2026-09-13.md)。
>
> 修订 v1.4（2026-09-13，账本同步）：本审计关联的 `GAP-MECH-IMMEDIATE-FEEDBACK` 登记位置由误引 `docs/BACKLOG_AND_PRIORITIES.md:448` 更正为 **BACKLOG `0ac` 节**（`:448` 实为 `TODO.md` 行号；BACKLOG 原无该条，已随本轮待办入账补入）；同时补入索引 v3.14 路由与 TODO `P0-0ac` 段。
>
> 触发（用户 2026-09-13 口径）：预算是**运行预算**，不是**等待预算**；结果一旦出现就是新的结果动作，机械层应立刻回报；不只是检索部分，其他部件也一样；查全框架各部分的预算，强制等待化的部分要全改掉；**先调查并写入文档，不直接行动**。
>
> 方法：`CLI_PROJECT_INDEX` 路由 → ADR/设计/实施审计回查 → 源码逐一核对 → 用 TB 2.1 第 0 轮 6 个 run 的 `.gsa` journal 实测校准。
>
> 关联：`GAP-MECH-IMMEDIATE-FEEDBACK`（BACKLOG **0ac**，2026-09-13 裁决登记，设计定稿待放行实施）。本审计是它的证据基座，同时**修正其两条验收判据与两条已入档计数口径**（见 §4 C5/C6、§6.5）。〔v1.4 更正：原引 `docs/BACKLOG_AND_PRIORITIES.md:448` 系误记，该行实为 `TODO.md:448`；BACKLOG 登记已于 2026-09-13 补入 0ac 节。〕

## 0. 直接回答

**既有设计没有把各部件的时间预算定位为"需强制等待"。** 权威口径三处都写的是"上限/兜底/及时返回"：

1. `adr/ADR-0010-fusion-runtime-and-agent-architecture.md:247-248`："预算是 **anti-runaway backstop**，不是对正常复杂任务工作量的估计。"
2. `docs/TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md` §2（2026-09-03 用户裁决）：**P2 只留官方评测要求的墙钟**（orz/runner 自加硬杀墙钟全删）；**P3 工具执行层无自身硬超时**——"超阈值自动后台化 + 一次提醒，不杀；……机械兜底=5 分钟无实际活跃（无输出增长）即杀并返回提醒"。
3. `docs/COMMAND_TIMEOUT_AND_WEB_SEARCH_TIMEOUT_RESEARCH_2026-08-29.md:62`："**超时后的行为比超时值更重要**：应返回结构化错误（step/code/message + 建议：重试/换查询/走 web_fetch 直读），而不是挂死或空结果。"

但**实现与装置层存在"等待化"漂移**，分三类：

| 类 | 形态 | 本轮判定 |
|---|---|---|
| T1 到期才回报 | 预算内没有"结果/确定不可达"的早期回报路径，只有到点返回 | **违规**：检索子代理墙钟、web_search 服务端静默等待、web_search 信号量排队 |
| T2 到期不终止 | 预算到期只取消了某一层的等待，动作本身继续跑 | **违规**：agent 阶段超时后 orz 孤儿；verifier 通道吃满 900 s |
| T3 结果出现但延后交付 | 结果已产生，机械层按下一次交互边界才带回 | **设计内延迟**：后台完成提醒；是否改"即时"需裁决 |

另有**两处新口径与既有已定案裁决的冲突**，不能静默改：

- **10 s 的准确语义（2026-09-13 用户澄清，v1.1 更正）**：10 s 是**检索请求发出后、对"首个检索结果"的等待上限**，不是检索这一任务的总时限；任务总时限是**另一本总预算**，10 s 与总预算不是同一个动作。10 s 内没有任何结果 ⇒ 基本可判定网络/通道不通，机械层应立即明确回报；一旦成功拿到结果，后续按总预算继续。**正确的工程含义 = 检索通道必须有 ≤10 s 内可观测的"首个结果"**。第 0 轮实测 web_search 单次**完成**耗时 p50 = 29.9 s、96.6% ≥10 s（§3.2、附录 A）⇒ 这只能证明"把 10 s 当成**完成**截止不可行"，不能反驳 10 s **首个结果**判据；真正的问题是当前 `web_search` 是非流式整包返回，10 s 内没有可观测的首个结果（详见 §3.2/D2、附录 E）。
- "探针扩到检索族 + 能力级不可达即时返回"与 0t v1.3 FP-2 的**关系（2026-09-13 用户定）**：**FP-2 不改**；这不是"例外"——能力级不可达的如实汇报本来就在 FP-2 的"正常回传真实错误类别"之内，唯一新增的是"**有结果就不再等待，立刻发回**"。只有在实现加入"不再重复尝试 / 失败计数反馈 / 移除车道"时才需要回头判断是否触碰 FP-2 的非目标（`docs/RETRIEVAL_SUBAGENT_DUAL_LANE_DESIGN_2026-09-09.md:126-128`、`:199-205`）。

### 0.1 用户澄清与新裁决（2026-09-13，v1.1 入档）

1. **10 s 是"等首个检索结果"的上限，不是检索任务总时限**；总预算另计。因此：
   - 10 s 内拿到首个结果 → 通道判活，检索继续按总预算走；
   - 10 s 内没有任何结果 → 判定网络/通道不通，机械层立即回报（不等引擎自身 120 s）；
   - 这要求检索通道具备**≤10 s 可观测的首个结果/首段进度**；当前 `web_search` 的整包返回不满足（见 D2、附录 E）。
2. **C6（第 0 轮计数更正）撤回**：用户裁决**本轮全部重新跑**（含已跑的 6 个 run），本轮账面只作摩擦证据，不再修计数、不再补跑局部。
3. **本轮暴露的问题严重，需要大改**：第 0 轮的结论不作为成绩或基线使用；改完再整轮重跑（见附录 E 的大改候选）。
4. **FP-2 不改、不叫例外**（用户 2026-09-13 定）：能力级确定不可达（无浏览器可执行、端点/凭据缺失、通道明确不通）按"**情况如实汇报 + 有结果就立刻发回、不再等待**"实现；0t 前置决策保持不变，除非新实现真的引入"不重复尝试/失败计数反馈/移除车道"（那才需要回查冲突）。FP-2 原文与来源见附录 C。
5. **D7 改掉，新增机制，缺口必补**（用户 2026-09-13 定）：唯一约束 = **不打断当前思维链**。可行形态：
   - 常规形态：在"当前轮结束 / 最早安全边界"即时回报；
   - 极端形态：**监控思维链文字**，后台结果到达后，在思维链出现的**第一个句号处立刻回报**（把信息/结果尽早交给模型，早知道了会省事）。
   来源链见附录 D；实现候选与协议边界见 E6。

### 0.2 即时回报 vs 打断：成本模型（2026-09-13 用户提问）

用户问：工具一返回就报给模型，对模型的压力和影响大不大？答案分三层：

1. **工具结果返回本身不算打断**：模型已结束该轮思考、发出 `tool_calls`，正停在合法边界等结果；每条结果立即回报是现有主循环默认行为（第 0 轮 622 次工具调用全部如此），**边际压力低**，失败/缺内容/空结果尤其应即时。
2. **真正贵的是常驻/周期注入**：`MODEL_RESIDUAL_PRESSURE_DISCUSSION_2026-08-31.md` §2.2 记 12 类注入块仍注册，"每次注入 = 一次注意力切换 + token 消耗 + 偏离主任务风险"；`THIN_HARNESS_REDESIGN` 结论=摩擦主要是注意力稀释（工具面/提示词/注入块）+ 仪式往返，模型能力不是瓶颈。框架的方向是**减少常驻**（PUSH→PULL、prompt 近零、LIF fires 不注入、F6 push ≤3 次/run、机械审计只报事实），不是禁止事件驱动的即时结果。
3. **生成中插入才是结构性打断**：流式 API 不允许在飞行中的 assistant 消息内插文本；可行实现=**句号边界分段续写**（停止当前流、保留部分 assistant+reasoning、带注入事实继续生成）。"不打断当前思维链"因此是优先偏好，不是绝对约束；结果价值足够高（确定不可达、内容缺失、空结果、前提被推翻、正在等的后台任务完成）时，允许句号边界即时注入；硬中断只留给高价值失效并单独 A/B。

决策规则：**即时注入当且仅当"避免的浪费（错误继续/空转/错误前提下的动作）> 打断成本（注意力切换+token+偏离+可能丢弃的半截生成）"**；缺内容/确定失败的避免浪费通常远大于成本，装饰性进度相反。完整设计见 [`即时结果回报与流式检索设计`](../IMMEDIATE_RESULT_DELIVERY_AND_STREAMING_RETRIEVAL_DESIGN_2026-09-13.md) §1.2/§2。

## 1. 回查口径（一手来源）

| 来源 | 口径要点 |
|---|---|
| `ADR-0010 §3.4.2`（:246-251） | 预算 = anti-runaway backstop，不是工作量估计；预算按 session 累计；只有 activation 关闭后才从 0 起算 |
| `TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md` §2/§3.1/§3.3/§3.8 | P2 只留官方墙钟；P3 无自身硬超时，超阈值自动后台化 + 一次提醒；F6 预算可见性 off/pull/push；撤默认 120 轮硬限（保留可配上限逃生阀） |
| `TER_T1_2_FG_BUDGET_180S_DEFAULT_2026-09-03.md` | 前台 block/首报点默认 180 s（单一生效源） |
| `TER_T1_4_NO_HARD_TIMEOUT_KILL_2026-09-04.md` | 分层 timeout（普通 300 / 程序 600 / 模型上限 900）不再作为杀进程点，只作 auto-bg deadline / 上限引用；后台化后只受 10 h 绝对兜底 |
| `TER_T1_5_IDLE_CPU_BACKSTOP_2026-09-04.md` | 后台任务连续 300 s 无输出增长且 CPU 不增 → idle_killed；计算密集无输出不误杀 |
| `TER_T1_7_UNLIMITED_ROUND_BUDGET_DEFAULT_2026-09-04.md` | 主车道 `max_tool_rounds` 默认 0 = unlimited；显式 >0 才恢复硬闸；评测以墙钟为准 |
| `TER_T1_9_F6_PUSH_BUDGET_CUE_2026-09-04.md` | F6 push 仅在 <600/<300/<120 s 注入中性剩余事实，≤3 次/run；只报剩余，不附建议 |
| `TER_T2_1_WALLCLOCK_SINGLE_SOURCE_2026-09-04.md` | 删 runner 自加 `--max-wallclock` 与 840 余量；sandbox `--timeout` = 官方 agent_timeout；`ORZ_MAX_WALLCLOCK` 透传同值 |
| `COMMAND_TIMEOUT_AND_WEB_SEARCH_TIMEOUT_RESEARCH_2026-08-29.md` §5/§6 | 终端分层默认超时 + 5 min 中间回报；web_search 客户端总超时 120 s + connect 10 s；超时返回结构化错误；不新增自动重试 |
| `THIN_HARNESS_REDESIGN_DESIGN_2026-08-27.md` §4.6 + R2a | `ORZ_TOOL_TIMEOUT_SECS` 是逃生舱（缺失 300 s；0 = 不限制；非法值显式报错）；工具执行期间 60 s 心跳保活，stall 只收模型侧静默 |
| `RETRIEVAL_SUBAGENT_DUAL_LANE_DESIGN_2026-09-09.md` v1.3 | 双车道恒在、静态标注、换道由模型自主；γ 机械降级退役；FP-2 = 检索失败正常回传、**不做勿重试教学/失败计数/机械降级提示** |
| `TOOL_AVAILABILITY_PROBE_DESIGN_2026-08-13.md` §2.1（:67） | 探针面为"主 Agent 工作工具"；**检索车道为显式例外**（web_search / web_fetch / browser_read / pdf_read / project_doc_index 无探针） |

## 2. 全框架预算清单（按层）

### 2.1 模型与运行层

| 部件 | 预算（默认） | 设计语义 | 当前实现行为 | 判定 | 证据 |
|---|---|---|---|---|---|
| 非流式模型请求 | `request_timeout` 18 min；`request_max_retries` 10；`request_retry_window` 180 s | 上限 + 有界重试 | 有结果立即返回；到点 `Timeout` | 合规（运行预算） | `orz/crates/orz-loop/src/gateway/model.rs:60-75`；`transport.rs:1151` |
| 流式模型请求 | idle warn 5 s / idle timeout 30 s / 总流 30 min | 死线看门狗 + 总兜底 | 有 chunk 即推进；30 s 完全静默才判死线 | 合规（看门狗） | `model.rs:47-57`；`transport.rs:1643-1710` |
| 空响应快速重试 | 2 次；退避 500 ms→10 s + 10% jitter | 无可见输出的安全重试 | 有界退避；不重复工具 | 可接受（失败重试等待） | `transport.rs:371-377`；`DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md` §3.2 |
| reasoning-stall 兜底 | 600 s / 64K reasoning token | 已于 2026-08-28 **物理删除** | 仅保留复读哨兵；不再有 stall 等待预算 | 无等待预算 | `DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md` 头注；`THIN_HARNESS_REDESIGN_DESIGN_2026-08-27.md` §4.6 |
| 工具轮预算 | 产品默认 0 = unlimited；评测适配器设 `ORZ_MAX_TOOL_ROUNDS=999` | anti-runaway 计数上限 | 默认无硬限；显式 >0 才拦 | 合规（计数上限） | `orz-loop/src/controller.rs:59-95`；`D:\tb-eval\tb_agents\orz.py` `_DEFAULT_MAX_TOOL_ROUNDS` / `run()` env |
| 主 run 墙钟 | 官方 `agent_timeout_sec`（900/1800/3600…），经 `--ak max_wallclock` → `ORZ_MAX_WALLCLOCK` | 运行上限；到期 `run_invalidated{status:wallclock}` 优雅收尾 | 到期 drop run future + 记终态；有结果则正常 `run_finished` | 设计合规；**实机复验待做** | `orz-bin/src/main.rs:113-127`、`:600-620`；`scripts/run_r0_heavy_official.py:247`；`TER_T2_1` |
| 模型侧 stall watchdog | 评测 `.env`：`ORZ_STALL_TIMEOUT=360` | 只收模型侧静默；工具执行期间 60 s 心跳保活 | 合法长工具不会被 stall 误杀 | 合规（兜底） | `host_exec.rs:3446-3462`；`D:\tb-eval\.env` |
| F6 预算可见性 | push 阈值 <600/<300/<120 s，≤3 次/run | 通知剩余，不要求等待 | 只注入中性事实 | 合规（通知） | `TER_T1_9`；`TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md` §3.3 |
| Orientation | 50 个逻辑轮 | 周期软门 | 计数触发，非时间预算 | 合规（计数） | `ADR-0010 §14.13/§14.16` 条目；`CLI_PROJECT_INDEX.md` FUS-ORIENTATION |
| 压缩 / fold / 注入预算 | 384K 窗口、160K 触发、17K 桥、50K 单轮注入等 | 上下文/成本上限 | 不构成时间等待 | 合规（token 上限） | `CONTEXT_COMPACTION_DESIGN_2026-08-14.md`；`ADR-0010 §3.6` |
| 检索候选 cap | 8/激活（web_fetch 家族与 browser_read 共享计数域） | 机械候选预算 | 失败 fetch 也消耗候选；每条结果带"候选 N/M" | 合规（计数上限） | `ADR-0010 §14.11/§14.12`；`RETRIEVAL_MECHANICAL_CONTROLS_DESIGN_2026-08-13.md` |

### 2.2 工具执行层

| 部件 | 预算（默认） | 设计语义 | 当前实现行为 | 判定 | 证据 |
|---|---|---|---|---|---|
| 宿主每工具外层 wall-clock（P0-1） | 代码默认 300 s；评测 `.env` `ORZ_TOOL_TIMEOUT_SECS=900`；0 = 不限制 | 机制性硬兜底 | 到期杀进程树 → `ToolError::Timeout`；模型继续 | 兜底合规，但有排队等待问题（见 D3） | `orz-host/src/lib.rs:147-152`、`:1507-1520`；`orz-bin/src/main.rs:1338-1352`；`D:\tb-eval\.env` |
| `run_terminal_cmd` 前台 | `foreground_block_budget_ms=180_000` | 到点自动后台化 + 一次中间状态 | 到期返回"仍在运行 + pid + 已耗时 + 输出活跃度"；**不杀** | **合规样板** | `orz-tools/src/computer/local/terminal.rs:60`；`TER_T1_2`；第 0 轮 7 次在 ~180.1 s 返回 |
| `run_terminal_cmd` 后台 | idle+CPU 300 s；绝对兜底 10 h | 无活跃才杀；绝对兜底防泄漏 | 无输出增长且 CPU 不增才 `idle_killed` | 合规（兜底） | `terminal.rs:50-85`；`TER_T1_5` |
| `run_terminal_cmd` 分层 timeout | 普通 300 / 程序 600 / 上限 900 | 仅作 auto-bg deadline / 上限引用 | 默认路径不再杀活跃命令；显式 `auto_background_on_timeout=false` 才保留 kill-on-timeout | 合规 | `TER_T1_4` |
| `run_tests` | 30 min（`RUN_TESTS_TIMEOUT`） | host-owned 测试运行上限 | 完成即返回；到点杀树返回 `timed_out` | 合规（运行预算） | `orz-loop/src/host.rs:445`；`orz-host/src/lib.rs:1893-1921` |
| `grep` | 20 s（WSL 60 s） | 到点返回**部分结果** + 截断标注 | 有结果立即返回；到点返回已缓冲 + `timed out` 信息 | **合规样板** | `orz-tools/src/implementations/grok_build/grep/mod.rs:189-210`、`:373-424` |
| 浏览器动作 | 无浏览器立即失败；launch 等 DevTools port ≤30 s；单页 total 60 s / load 30 s | 单次调用上限 | 无浏览器 0–13 ms 报错；有浏览器时到点结构化失败 | 单次合规；车道级见 D4 | `orz-host/src/local_browser/cdp.rs:147-160`、`:573`、`:674-728` |

### 2.3 检索 / 网络层

| 部件 | 预算（默认） | 设计语义 | 当前实现行为 | 判定 | 证据 |
|---|---|---|---|---|---|
| `web_search` | 总 120 s + connect 10 s | 服务端生成式检索上限 | 出结果立即返回；DNS/拒绝立即错误；**连上但服务端静默 → 等满 120 s** | **部分等待化**（见 D2） | `orz-tools/src/implementations/web_search/client.rs:68-76`；`COMMAND_TIMEOUT...2026-08-29.md` §5 |
| `web_search` 全局信号量 | 并发 = 1；acquire 无独立截止 | 防并发/配额保护 | acquire 被外层 `tool_timeout`（评测 900 s）包住；排队可等到 900 s 才返回 waiting timeout | **等待化**（见 D3） | `orz-host/src/lib.rs:1222-1240`、`:1476-1500`、`:1507-1520` |
| `web_fetch` | 总 60 s + connect 10 s + pool idle 30 s | 直连抓取上限 | 出结果立即返回 | 合规 | `orz-tools/.../grok_build/web_fetch/config.rs:26-27`、`:72-73`；`http.rs:57-63` |
| 检索子代理墙钟 | standard 240 s / extended 600 s / deep 900 s；env 可覆盖；0 = 禁用 | 单次派发**兜底**（包裹整个 subagent loop） | 到期 drop loop future、激活 `subagent_timeout` 收口；父侧只在到期时获知 | **T1 主违规**（见 D1） | `orz-loop/src/controller.rs:83-95`；`retrieval/effort.rs:36-42`；`retrieval/dispatch.rs:328-333`、`:425-460` |
| 检索子代理轮数 | standard 30 / extended 60 / deep 90 | 计数上限 | 与主车道显式上限取 min；主车道无限时不截断 | 合规（计数） | `retrieval/effort.rs:44-51`；`dispatch.rs:336-352` |
| 检索探针 | `probe_scope=main_agent_work_tools`（23 工具） | 主工作面探针；**检索车道为显式例外** | 检索族（浏览器 / 搜索引擎 / web 通道）无机械状态 | 反馈缺口（见 D4；与 FP-2 冲突） | `orz-loop/src/controller.rs:969`；`tool_probe.rs:44-70`；`TOOL_AVAILABILITY_PROBE_DESIGN_2026-08-13.md:67` |

### 2.4 装置层（适配器 + harbor）

| 部件 | 预算（默认） | 设计语义 | 当前实现行为 | 判定 | 证据 |
|---|---|---|---|---|---|
| agent 阶段 | 官方 agent timeout（如 900 s） | harness 等待上限 | `asyncio.wait_for(agent.run, timeout)` 只取消**自己的等待**，不杀容器内进程 | **T2 根因**（见 D5） | `D:\tb-eval\venv\Lib\site-packages\harbor\trial\trial.py:425-459` |
| adapter 启动脚本 | 无自身上限；靠 `--ak max_wallclock` | 让 orz 在 harness 硬杀前自收尾 | `{ orz …; echo $? > exit_file; } &` + `wait` + `while [ ! -f exit_file ]; do sleep 5; done`；agent 超时后无清理 | 与 D5 同根因 | `D:\tb-eval\tb_agents\orz.py` `_build_run_script` |
| verifier 阶段 | 官方 verifier timeout（900 s） | 测试执行上限 | 完成即返回；第 0 轮 `torch-tensor-parallelism` 出现脚本 51.51 s 完成、`reward.txt=1` 但通道整 900 s 未返回 | 需定向复现（见 D6） | `harbor/trial/single_step.py:87-112`；第 0 轮起跑记录 §6.10 |
| runner 预拉 | 4 次重试 + 15 s sleep | 拉镜像失败重试 | 非模型面，不进容器 | 可接受 | `scripts/run_r0_heavy_official.py:157-183` |

### 2.5 旁路（本轮评测路径之外）

| 部件 | 预算 | 判定 |
|---|---|---|
| permission | 评测 `Benchmark` policy auto-allow；headless `Ask` → `Deny` | 无等待预算，合规（`orz-host/src/permission.rs:42-90`） |
| hooks | 命令 / HTTP 各自 timeout（`orz-hooks/src/runner/command.rs`、`http.rs:121-122`） | 不在本轮 TB 评测路径；需单独审计，登记 |
| MCP | stdio shutdown grace、HTTP 重连退避（`orz-mcp/src/servers.rs`、`mcp_http_client.rs`） | 不在本轮评测路径；需单独审计，登记 |
| `ask_user_question` | 交互式等待用户；headless fail-closed | 设计上就是人机交互，非本口径对象 |

## 3. 判定：强制等待化清单（按严重度）

### D1（最重）检索子代理墙钟 600 s：父侧只在到期时获知

**第 0 轮一手证据（6 个 run 的 `.gsa` journal）**

- 9 个检索激活：`terminal_reason=auto_close` 6 个，`subagent_timeout` 3 个。
- 3 个超时激活的 `effort=extended`（档位墙钟 600 s）：
  - `mteb-leaderboard` 第一次：08:29:05.599 → 08:39:05.567，599.97 s；
  - `mteb-leaderboard` 第二次：08:39:08.629 → 08:49:08.620，599.99 s；
  - `gpt2-codegolf`：09:54:21.675 → 10:04:21.655，599.98 s；
  - 合计 ≈ **1800 s** 子代理墙钟，全部以到期错误收口。
- 每个超时只产生：一条父侧 `tool_completed{tool:web_search,target:external_retrieval,error:"retrieval subagent wallclock exceeded"}`；`mteb` 另有 2 条对 in-flight 调用的合成收口 `tool_completed{error:"subagent_wallclock_timeout_mid_tool"}`。
- 例：`mteb` 第二次激活在 08:39:03.170 启动最后一个 `web_search`，08:39:05.565 墙钟到点（2.4 s 后），整段激活被丢弃。
- 600 s 内**没有**向父侧/模型提供"车道正在失效 / 已确定不可达 / 已有部分结果"的中间回报；唯一结果就是到期错误。

**判定**：T1 典型违规——预算变成了父侧的等待时长，而不是运行上限。设计原意（`dispatch.rs:387-460`）是"墙钟包裹整个子代理 loop、到期兜底丢弃"，但实测里它成了这些激活**唯一**的终止路径。

**修法方向**（登记，不实施）：子代理在"确定不可达 / 连续确定失败 / 结果已形成"时**提前收口**并立即回传；至少给父侧/模型一条 mid-activation 事件；墙钟只做最后兜底。

### D2 `web_search` 120 s：服务端静默时等满；与 10 s 新口径冲突

**第 0 轮实测分布（n=116 条带 `wall_ms` 的 `web_search` 完成记录）**

| 指标 | 值 |
|---|---|
| min | 4,861 ms |
| p10 / p25 | 12,848 / 20,887 ms |
| p50 | **29,869 ms** |
| p75 / p90 | 32,564 / 34,759 ms |
| max | **37,246 ms** |
| `<10 s` | **4 条（3.4%）** |
| `≥10 s` | **112 条（96.6%）** |

（另有 127 次 `web_search` `tool_completed`，其中 5 次错误：3 次 `retrieval subagent wallclock exceeded` + 2 次 `subagent_wallclock_timeout_mid_tool`；116 条为带 `wall_ms` 的完成记录。）

**按 v1.1 澄清重新定性**：10 s 不是"检索任务总时限"，而是"**请求发出后等首个结果**"的上限。上表的 96.6% ≥10 s 说明的是：**把 10 s 当成"完成"截止不可行**（会误杀 96.6% 的正常完成）；它不构成对 10 s **首个结果**判据的反驳。

**真正的缺口**：`web_search` 当前是一次非流式 `response.bytes()`（`web_search/client.rs:222-232`），服务端 15–70 s 的多轮检索期间**没有任何可观测的首个结果/首段进度**。因此按"10 s 内无首个结果即判网络不通"的语义：

- 直接对当前实现加 10 s 截止 → 会把健康的服务端检索误判为网络不通；
- 要真正满足用户语义，必须让检索通道在 ≤10 s 内产出**可观测的首个结果/首段进度**，后续结果可以晚到并按总预算继续。

**建议口径（待裁决）**：

1. `first_result_deadline ≤10 s`：检索请求发出后，10 s 内必须出现"首个结果/首段进度/明确应答"；没有 → 立即返回稳定码（`network_no_response` / `capability_unreachable`），本 run 不再重复；
2. `total_budget`：**另一本账**；首个结果到达后，检索任务按总预算继续；每得到一个结果就是一个新的结果动作，机械层立即回报；
3. 实现路径二选一（见附录 E）：
   - **a. 流式/分段检索**：改用可流式读取的检索接口（或本地检索后端），首段结果 ≤10 s 到达，后续分段即时回报；
   - **b. 通道判活 + 操作预算**：10 s 内先确认通道判活（连接 + 应答/进度），完整结果仍允许 120 s，但必须有可见的进行中读数（不是静默等满）。

不建议把 120 s 直接改成 10 s（完成口径）；也不建议保留"10 s 内零观测"的现状。

### D3 `web_search` 信号量 acquire 无独立截止

**证据**：`orz-host/src/lib.rs:1476-1500` 的 `acquire_owned()` 没有 timeout；它被外面 `tokio::time::timeout(effective_timeout, fut)` 包住，而 `effective_timeout = min(timeout_override, tool_timeout)`，评测 `.env` 下 `tool_timeout=900 s`（`lib.rs:1507-1520`）。等待 permit 的调用 `started_exec=false`，超时**不杀进程**，但会等满外层预算才返回 waiting timeout。

**判定**：T1 等待化。修法方向：acquire 独立短截止（如 5–10 s）+ 排队即时报结构化 `engine_busy` / `queue_timeout`；不与执行预算混用。

### D4 浏览器"确定不可达"无 run 级记忆 + 探针不含检索族 + 失败载荷不自描述

**第 0 轮证据**

- 14 次 `browser_launch_result` 全 `failure`；13 次 `browser_control` + 5 次 `browser_read` 调用失败；单次 `wall_ms` 0–13 ms（**不是慢，是重复**）。
- `ensure_browser_launched_with`（`orz-host/src/lib.rs:827-847`）在启动失败时直接返回 `BrowserLaunchFailed`，句柄保持 not ready；每次 dispatch 重新 `probe_launch`，无 negative cache。
- `tool_availability_check` 只报主工作面：10 条事件全部 `probe_scope=main_agent_work_tools`，`complete=[read_file,grep,search_replace,blackboard_read,run_terminal_cmd]`，`incomplete=[]`；检索族无任何机械状态。
- `tool_completed` 失败载荷 `{tool,status:error,error:"browser_launch_failed"}` **不含 cause**；cause 只在相邻 `browser_launch_result.cause`（如 `browser_not_found: no browser executable found (ORZ_BROWSER_PATH unset; searched: chrome, …)`）。

**判定**：反馈/机制缺口，不是镜像缺陷，也不改变"不增加容器内浏览器"的既有裁决。但修法 1/3 与 FP-2 冲突：FP-2 明确不做"勿重试教学、失败计数反馈、机械降级提示"。建议把口径精确区分：

- **能力级确定不可达**（无浏览器可执行、web_search 凭据/端点缺失等）：首次即如实报因 + 本 run 记忆"该车道不再可用" + 不再重复尝试；
- **页面级失败**（导航超时、DNS 失败、防火墙拦截、可拉起但实质不可用）：保留 FP-2——正常回传真实类别、不教学、不计数、不诊断提示。

该区分需先修订 `ADR-0010 §14.65` / 0t v1.3 非目标措辞，不能由实现静默翻转。

### D5 agent 阶段超时后 orz 不停（预算到期不终止动作）

**第 0 轮一手证据**（`torch-tensor-parallelism`，起跑记录 §6.10）：agent 阶段 07:46:01 超时；该 run 随后仍写 **272 条事件**（34 `model_output`、56 次工具调用）直到 **08:00:49**（14.5 min），容器删除才停。

**机制**：harbor 的 agent 阶段超时是 `asyncio.wait_for` 取消**自己的等待**；adapter 把 orz 放在后台子壳（`{ orz …; echo $? > exit_file; } &` + `wait`），`docker exec` 结束不杀容器内进程 ⇒ orz 孤儿继续跑。适配器本有自救开关 `--ak max_wallclock`（orz 到期优雅 `run_invalidated`），第 0 轮**没有传**。

**已实现修法**：`scripts/run_r0_heavy_official.py` 已按每题官方 agent timeout 传 `--ak max_wallclock=<sec>`（commit `419b1c8f`），但**尚未实机复验**。注意：orz 墙钟起点略晚于 harbor agent 阶段起点（该试次 `agent_execution.started_at=07:31:01.053`，首个 orz 事件 `07:31:01.259`，相差约 0.2 s）⇒ 用官方值本身仍有毫秒级竞态；若定向复现仍出现孤儿，装置侧应在 agent 阶段结束后按 pid 文件显式 kill 容器内 orz 进程组。

### D6 verifier 通道吃满 900 s（"已通过未记账"）

**第 0 轮一手证据**：`torch-tensor-parallelism` 的 verifier 阶段 `07:46:01.160 → 08:01:01.239 = 900.08 s`；而该题 `tests/test.sh` 自身已跑完（`13 passed in 51.51 s`，`reward.txt=1`，脚本无收尾挂点）。对照其余三个同样被 agent 超时掐断的试次，verifier 阶段分别为 17 s / 104 s / 9 min 正常返回 ⇒ 唯一显著不同形态 = 该容器里还有在跑的 orz（与 D5 同族）。

**判定**：T2 代价；需随 D5 的"带 / 不带 `max_wallclock`"定向复现一起闭环。

### D7 后台完成"下一次工具结果顶部带回"

**来源（回查）**：这条不是 2026-09-03 新造的，而是 `THIN_HARNESS_REDESIGN_V2_DESIGN_2026-08-28.md` §9.7.2 的**复用既有机制**决定：

- 原文（`:591`）："终态送达说明（最终结果随下一次工具结果返回）"；
- 原文（`:612`）："复用既有 `TaskCompletionReminder`——后台任务完成后，下一次任意工具结果顶部 `<system-reminder>` 携带最终退出码/信号/时长/输出指针（**既有机制，不新增唤醒**）"；
- 边界（审查处理 P3-5）："若模型在中间回报后不再发起任何工具调用（直接终答/run 结束），终态提醒随无后续工具结果而不会送达——模型已被告知『最终结果随下一次工具结果返回』"；
- 2026-08-29 用户裁决（`COMMAND_TIMEOUT_AND_WEB_SEARCH_TIMEOUT_RESEARCH_2026-08-29.md` §5）承接为"运行满 N 秒机械插入一次中间状态；回报后默认继续等待、模型可主动中断"；2026-09-03 TER §3.1 条 3 只把触发点改为 180 s auto-bg，交付点未变。

**机制原因（为什么当时选"下一个工具结果"）**：

1. `TaskCompletionReminder` 是注册在 `FinalizedToolset` 上的横切 reminder（`orz-tools/src/reminders/task_completion.rs`），收集点就是**工具结果管线**；
2. 协议上，模型发出 `tool_calls` 后，下一轮请求必须带上这些 call 的 `tool` 结果；在 pending 工具未收口前没有合法的"下一轮模型请求"可以承载提醒；
3. 若要"当前轮结束后立即回报"，需要新增唤醒/打断通道（在 pending 工具未完成时起新一轮，或合成未完成工具结果），而 2026-08-28 的裁决明确选择"**不新增唤醒**"（"新增机制一律视为债务"）。

**实测边界（TB 4.0 摩擦探针，2026-09-11）**：该机制在**轮询型 agent** 上结构性不触发——模型每次都自己去读后台产物，读取即命中 `consumed_completion_ids`，提醒被抑制；该跑 21 次后台化 + 2 次击杀，`While you were idle…` 注入 **0 次**（`docs/audits/TB40_CTR_OPTIMIZATION_FRICTION_PROBE_2026-09-11.md` §10.1）。

**判定**：设计内延迟（T3），不是接线缺失；但用户质疑成立：

- 当前最早的**安全**注入点是"该批工具中最先返回的那个结果"（不要求等最慢工具），若模型只发起一个长工具，延迟就等于该工具时长；
- 若要"当前轮结束/最早边界即时回报"，必须要么新增唤醒机制，要么把长工具本身改成**中途回报**（terminal actor 180 s 中间态就是这种形态的样板）；
- 该点转入 C4 裁决，与"结果一旦出现即回报"的总契约合并设计。

### D8 非违规 / 已合规（对照，防止"一刀切"）

- `run_terminal_cmd` 180 s auto-bg：第 0 轮 7 次恰在 ~180.1 s 返回中间状态（**不是杀、不是干等**）；
- `grep` 20 s：返回部分结果 + 结构化超时；
- `web_fetch`：p50 0.55 s、max 10.0 s（2 次错误），无需改；
- 浏览器单次失败：0–13 ms 立即返回；
- permission：评测 `Benchmark` auto-allow / headless fail-closed，无等待；
- F6 push：只通知剩余，不要求等待；
- `max_tool_rounds` / 候选 cap：计数上限，非时间预算；
- 主 run 墙钟：到期优雅收尾，不是等待（但 D5 的装置层要做实）。

## 4. 需要重新裁决 / 修订的冲突点（本轮不决定）

| # | 冲突 | 建议口径 |
|---|---|---|
| C1 | 10 s 的语义 vs web_search 的非流式整包返回 | **10 s = 请求发出后等首个结果的上限，不是任务总时限**；总预算另计。**用户 2026-09-13 定：先做流式检索**（分段为后备，通道判活+操作预算只作补充护栏）；设计稿见 `IMMEDIATE_RESULT_DELIVERY_AND_STREAMING_RETRIEVAL_DESIGN_2026-09-13.md` |
| C2 | FP-2 非目标 vs "确定不可达即时回报" | **用户 2026-09-13 定：FP-2 不改**；如实汇报 + 有结果即发回本就在 FP-2 语义内，不是例外。只有引入"不重复尝试/失败计数/移除车道"才回查冲突 |
| C3 | 预算到期 vs 动作终止 | 墙钟到期必须终止容器内动作；只取消 harness 等待不够（D5/D6） |
| C4 | 后台完成回报时点 | **用户 2026-09-13 定：D7 改掉，新增机制，明确的缺口必须补**；约束 = 不打断当前思维链。候选：最早安全边界注入 / 思维链句号边界即时注入（E6） |
| C5 | `GAP-MECH-IMMEDIATE-FEEDBACK` 验收"检索类 `wall_ms` p99 ≤10 s；`subagent_wallclock_timeout_mid_tool=0`" | 按 v1.1 修正：10 s 是**首个结果**指标（不是完成时长）；操作总时长按总预算单独设；`mid_tool=0` 保留为目标，但依赖 D1 提前收口 |
| C6 | 第 0 轮已入档计数 | **撤回**（用户裁决：本轮全部重新跑，含已跑 6 个 run；账面只作摩擦证据，不再修计数、不再局部补跑） |
| C7 | 检索结果的"分步到达"能力 | 新增要求：首个结果 ≤10 s 可观测；后续每得到一个结果即刻作为新的结果动作回报；总预算另计（E1/E2） |

## 5. 建议的修正批次（仅登记，待用户放行；本轮不实施）

| # | 内容 | 验收判据（可机械核对） |
|---|---|---|
| R1 | 通用契约：机械层对任何模型请求"及时且有信息量"地返回；等待必须有可见日志/事件；确定性不可达走快速路径；预算只做上限 | 新增回归钉子：任何等待型调用在截止后必须产出带稳定码的结果；无"到点前零事件"的等待路径 |
| R2 | 四本时限分账：`first_result_deadline`（≤10 s，仅判"请求发出后有没有首个结果/首段进度"）/ `operation_deadline`（操作自身上限，按操作校准）/ `total_budget`（检索任务总预算，与 10 s 分开计）/ `run_wallclock`（官方值） | 每个预算点标注属于哪一本；四本不互相冒充；10 s 只作首个结果判据，不冒充任务总时限 |
| R3 | 检索子代理提前收口 + 中间回报：确定不可达 / 连续确定失败 / 结果已形成 → close activation 并立即回传 | 第 0 轮同类场景不再出现"整 600 s 只在到期返回"；`subagent_timeout` 只作最后兜底 |
| R4 | 探针扩 `retrieval_family`（浏览器 / 搜索引擎 / web 通道）+ 能力级如实汇报 | 每 run ≥1 条含检索族结论的 `tool_availability_check`；前置无（FP-2 不改）；如实现"run 级不重复尝试"再单独回查 FP-2 |
| R5 | web_search 信号量 acquire 独立截止 + 排队即时回报 | acquire 等待有稳定码与独立上限；不再被 900 s 外层包住 |
| R6 | 装置生命周期：`--ak max_wallclock` + agent 阶段结束后显式清理容器内 orz；verifier 900 s 定向复现 | "带 / 不带 wallclock"对照：带墙钟的次不再出现孤儿与 verifier 吃满 |
| R7 | 后台完成即时回报（C4/D7）：新增机制，不打断当前思维链 | 完成事件不依赖"模型下次自己读产物"；轮询型 agent 下也不被 `consumed_completion_ids` 抑制；常规=最早安全边界，极端=思维链句号边界 |
| R8 | **撤回**（C6 撤回：本轮全部重跑，不修第 0 轮计数） | 无 |
| R9 | 检索"首个结果"可观测 + 分步回报（C1/C7）：流式/分段检索，或"通道判活 + 操作预算"；每得到一个结果立即作为新结果动作回报 | 请求发出后 ≤10 s 内出现首个结果/首段进度；无 → 稳定码立即返回；后续结果不静默积压 |
| R10 | 大改完成后整轮重跑（用户 2026-09-13 裁决） | 含第 0 轮 8 题全部重跑；第 0 轮账只作摩擦证据，不作成绩/基线 |

## 6. 附录 A：第 0 轮实测数据（6 个 run）

### 6.1 工具墙钟分布（`tool_completed.wall_ms`）

| 工具 | n | p50 | p90 | max | ≥10 s |
|---|---|---|---|---|---|
| `web_search` | 116 | 29,869 ms | 34,759 ms | 37,246 ms | 112（96.6%） |
| `web_fetch` | 93 | 548 ms | 1,750 ms | 10,010 ms | 2（2.2%） |
| `run_terminal_cmd` | 244 | 114 ms | 30,798 ms | 180,112 ms | 40（16.4%） |
| `grep` | 40 | 36 ms | 105 ms | 1,173 ms | 0 |
| `read_file` | 33 | 9 ms | 30 ms | 293 ms | 0 |
| `browser_control` | 13 | 9 ms | 11 ms | 13 ms | 0 |

补充：`run_terminal_cmd` 中 7 次在 ~180 s（auto-bg 点）返回，170 次 <1 s；`web_search` 的 `<10 s` 仅 4 条（min 4,861 ms）。

### 6.2 检索子代理激活

| run | 激活 | effort | 起 | 止 | 时长 | 结口 |
|---|---|---|---|---|---|---|
| `mteb-leaderboard` | 第 1 次 | extended | 08:29:05.599 | 08:39:05.567 | 599.97 s | `subagent_timeout` + 1 条合成 mid-tool |
| `mteb-leaderboard` | 第 2 次 | extended | 08:39:08.629 | 08:49:08.620 | 599.99 s | `subagent_timeout` + 1 条合成 mid-tool |
| `gpt2-codegolf` | 1 次 | extended | 09:54:21.675 | 10:04:21.655 | 599.98 s | `subagent_timeout`（父侧 dispatch error） |

全部 9 个激活：`auto_close` 6、`subagent_timeout` 3。3 次超时合计 ≈1800 s。

### 6.3 浏览器车道

- `browser_launch_result`：14 次，全部 `failure`；cause 全为 `browser_not_found: no browser executable found (ORZ_BROWSER_PATH unset; searched: chrome, …)`；
- `browser_control` 失败 13 次、`browser_read` 失败 5 次；`wall_ms` 0–13 ms；
- `tool_completed` 失败载荷只有 `error:"browser_launch_failed"`，cause 需 join 相邻 `browser_launch_result`。

### 6.4 探针

- `tool_availability_check`：10 条（6 个 run）；
- 全部 `probe_scope=main_agent_work_tools`，`complete=[read_file,grep,search_replace,blackboard_read,run_terminal_cmd]`，`incomplete=[]`；
- 检索族（browser / search engine / web channel）零探针结论。

### 6.5 计数口径更正（相对已入档文字）

> 2026-09-13 用户裁决：**C6 撤回**——本轮全部重新跑，已入档计数不再修；下表仅保留为第 0 轮摩擦证据的读数，不作为后续引用口径。

| 已入档 | 本审计实测 | 说明 |
|---|---|---|
| "5 次 `subagent_wallclock_timeout_mid_tool`" | **3 次 `subagent_timeout` 激活**（2 mteb + 1 gpt2）+ **2 条合成 `subagent_wallclock_timeout_mid_tool`**（均 mteb） | 5 是"subagent 相关字符串出现次数"口径，混入了 2 条激活级 close；不是 5 条 mid-tool |
| "`web_search` 单次最高 26.8 s" | **max 37,246 ms；p50 29,869 ms；p90 34,759 ms** | 26.8 s 非本轮 6 个 run 的全局最大值 |
| "`web_search` 合计 807–977 s" | 本轮按 run 的口径不同；四题各自 `web_search` 总墙钟与工具数见 §6.1 | 该区间来自不同的按题/按 run 统计口径，需在引用时注明 |

## 7. 附录 B：源码 / 文档索引（便于逐条复核）

- 预算口径：`adr/ADR-0010-fusion-runtime-and-agent-architecture.md:246-251`；`docs/TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md` §2/§3.1/§3.3/§3.8；`docs/COMMAND_TIMEOUT_AND_WEB_SEARCH_TIMEOUT_RESEARCH_2026-08-29.md` §5/§6；`docs/TER_T1_2_FG_BUDGET_180S_DEFAULT_2026-09-03.md`；`docs/audits/TER_T1_4_NO_HARD_TIMEOUT_KILL_2026-09-04.md`；`docs/audits/TER_T1_5_IDLE_CPU_BACKSTOP_2026-09-04.md`；`docs/audits/TER_T1_7_UNLIMITED_ROUND_BUDGET_DEFAULT_2026-09-04.md`；`docs/audits/TER_T1_9_F6_PUSH_BUDGET_CUE_2026-09-04.md`；`docs/audits/TER_T2_1_WALLCLOCK_SINGLE_SOURCE_2026-09-04.md`。
- 检索/双车道：`docs/RETRIEVAL_SUBAGENT_DUAL_LANE_DESIGN_2026-09-09.md` §0/§2/§3.1/§3.4；`docs/TOOL_AVAILABILITY_PROBE_DESIGN_2026-08-13.md:63-67`。
- 模型/传输：`orz/crates/orz-loop/src/gateway/model.rs:30-75`；`orz/crates/orz-loop/src/gateway/transport.rs:371-377`、`:1151`、`:1581-1710`。
- 轮预算/墙钟/探针：`orz/crates/orz-loop/src/controller.rs:59-95`、`:969`；`orz/crates/orz-bin/src/main.rs:113-127`、`:600-620`；`scripts/run_r0_heavy_official.py:235-260`。
- 检索派发：`orz/crates/orz-loop/src/retrieval/dispatch.rs:328-333`、`:425-460`、`:862`；`orz/crates/orz-loop/src/retrieval/effort.rs:30-51`。
- 工具执行：`orz/crates/orz-host/src/lib.rs:147-152`、`:827-847`、`:1222-1240`、`:1476-1520`、`:1893-1921`；`orz/crates/codegen/orz-tools/src/computer/local/terminal.rs:44-123`；`.../grok_build/grep/mod.rs:189-210`、`:373-424`。
- 检索/网络：`orz/crates/codegen/orz-tools/src/implementations/web_search/client.rs:68-76`；`.../grok_build/web_fetch/config.rs:26-27`、`:72-73`；`.../web_fetch/http.rs:57-63`。
- 装置：`D:\tb-eval\tb_agents\orz.py`（`_build_run_script` / `run()` env 白名单 / `_DEFAULT_MAX_TOOL_ROUNDS`）；`D:\tb-eval\venv\Lib\site-packages\harbor\trial\trial.py:425-459`；`harbor/trial/single_step.py:69-112`；`D:\tb-eval\.env`。
- 第 0 轮一手账：`docs/audits/TB21_V41_ROUND0_MEMORY_HEAVY_START_2026-09-13.md` §6.10/§6.11/§6.12/§6.13；`D:\tb-eval\gsa-volumes\official-r0-heavy\*\runs\RUN-*\events.jsonl`；`D:\tb-eval\jobs-official\official-r0-heavy\*\result.json`。

## 附录 C：0t v1.3 FP-2 是什么、为什么、与本口径的边界

**来源**：`docs/audits/0S_DETAIL_ANALYSIS_2026-09-09.md` §4「FP-2 chrome-error 拒绝信封不透明 + 同 URL 重试无抑制」；处置裁决在 `docs/RETRIEVAL_SUBAGENT_DUAL_LANE_DESIGN_2026-09-09.md` §0 第 4 条 / §2 非目标 / §3.4。

**原始摩擦（R3 实证）**：

- `browser_read refused [browser_read_blocked_scheme]: unsupported URL scheme: chrome-error` 全批 **69 次**（mteb 52 / make-doom 6 / raman 5 / protein 4 / count-dataset 2 / filter-js 2）；
- 页面导航失败后 Chrome 停在 `chrome-error://` 内部页，工具按 scheme 门拒绝——机械上正确，但信封**不含导航失败原因**（DNS/断网/页面崩溃），也没有"勿重试同 URL"的指引；
- mteb 对 `huggingface.co/spaces/mteb/leaderboard` **重试 14 次**，另试 bing/ddg/wikipedia/r.jina.ai 代理均死，`browser_read` 错误墙 **865 s / 预算 2812 s**；失败读还计入候选计数（8 cap），加剧 activation 重建。

**用户 2026-09-09 裁决（FP-2 的目标）**：

- **不做**"勿重试"教学、额外阻拦、失败计数反馈或机械降级提示（v1.3 §2 非目标原文："不做任何形式的『勿重试』教学、失败计数反馈或机械降级提示"）；
- 仅**正常回传真实错误类别**（网络错误/超时/防火墙拦截等），同一 URL 是否重试由模型自主决定；
- 重试根因**定性**为"浏览器组件可拉起但实质不可用"，与"工具可用性声明与实际冲突"同族；结构性解 = **双车道并存 + 静态标注 + 失败正常回传**，不是给模型加教学句。

**因此 FP-2 的"目标"不是"掩盖失败"，而是：**

1. 不靠提示词教育模型、不靠机械"别重试"压住模型；
2. 让车道声明与真实能力一致（双车道恒在 + 标注），失败如实回传；
3. 把"页面级/导航级失败"和"能力级缺失"都保持在正常工具错误语义里，不加额外机制。

**与本轮口径的关系（2026-09-13 用户定：不改 FP-2、不是例外）**：

- FP-2 覆盖的是**页面级失败 / 可拉起但实质不可用**；它要求的是"正常回传真实错误类别、不做教学/阻拦/失败计数/机械降级提示"。
- 本轮遇到的是**能力级确定不可达**（容器内根本没有浏览器可执行：`browser_not_found: no browser executable found (ORZ_BROWSER_PATH unset; searched: chrome, …)`；14 次 launch 全 failure）。
- 该情况的"**如实汇报 + 有结果就立刻发回、不再等待**"本来就在 FP-2 的正常回传语义之内，因此**不需要修订 §14.65，也不构成例外**；FP-2 的前置决策保持不变。
- 唯一需要单独判断的情况：若实现进一步加入"**本 run 不再重复尝试**（run 级记忆）"或"从工具面移除该车道"，那属于 FP-2 非目标里的"失败计数反馈/机械降级提示/机械降级"族，需要另行回查裁决；本审计不把该动作当作既成结论。

## 附录 D：D7「下一次工具结果」一页纸（来源链）

| 时点 | 来源 | 决定了什么 |
|---|---|---|
| 2026-08-28 | `THIN_HARNESS_REDESIGN_V2_DESIGN_2026-08-28.md` §9.7.2 | 中间状态返回 + **终态复用既有 `TaskCompletionReminder`**；"最终结果随下一次工具结果返回"；明确"**不新增唤醒**"；边界：若模型不再发起任何工具调用，终态提醒不送达（审查处理 P3-5） |
| 2026-08-29 | `COMMAND_TIMEOUT_AND_WEB_SEARCH_TIMEOUT_RESEARCH_2026-08-29.md` §5 | 用户裁决：运行满 N 秒机械插入**一次**中间状态；回报后默认继续等待、模型可主动中断；完成/超时后正常返回终态 |
| 2026-09-03 | `TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md` §3.1 | 触发点改为 180 s auto-bg（不杀）；交付点不变——"完成 → 在下一个工具结果顶部带回" |
| 2026-09-11 | `docs/audits/TB40_CTR_OPTIMIZATION_FRICTION_PROBE_2026-09-11.md` §10.1 | 实测：轮询型 agent 下该机制**结构性不触发**（模型自己读产物 → `consumed_completion_ids` 命中 → 提醒被抑制；21 次后台化 + 2 次击杀，注入 0 次） |

**协议原因（为什么当时不是"当前轮结束立刻回报"）**：模型发出 `tool_calls` 后，下一轮请求必须携带这些 call 的结果；在 pending 工具未收口前，没有合法的下一轮模型请求可以承载提醒。要在此前轮结束后立即注入，必须新增唤醒/打断机制（或合成未完成工具结果），而 2026-08-28 明确选择"不新增唤醒"。**当前最早的安全注入点 = 该批工具中最先返回的那个结果**（不要求等最慢工具）；若模型只发一个长工具，延迟就等于该工具时长。用户 2026-09-13 的质疑成立，转入 C4/R7。

**2026-09-13 用户裁决（D7 改掉）**：新增机制，明确的缺口必须补；唯一约束 = **不打断当前思维链**。两种形态：

1. **常规**：在"当前轮结束 / 最早安全边界"即时回报（不再等"下一次工具结果"）；
2. **极端**：**监控思维链文字**——后台结果到达后，在思维链出现的**第一个句号处立刻回报**（"信息和结果部分，早知道了会省事"）。

**实现可行性（须在设计中验证）**：

- 当前流式 API 不能在**进行中的 assistant 消息**里插入新文本；"句号处注入"的可行实现是**分段续写（cut-and-continue）**：框架监控 reasoning/text delta，结果到达后置 `pending_injection`；在下一个句号边界**优雅停止当前流**，保存已生成的部分 assistant 内容（含 reasoning），在下一请求中带上该部分内容 + 注入的机械结果事实，要求模型**从句号后继续**。语义上思维链内容不被改写，但请求被切成多段。
- 需要验证的协议点：DeepSeek 对"部分 assistant 消息 + reasoning + 新注入事实"的续写是否接受、`reasoning_content` 回传规则、缓存命中、重复 token、事件/工具调用配对完整性。
- 另一类更简单的缺口：若结果到达时模型正在等一个长工具（已有 pending `tool_calls`），在工具结果未返回前没有合法续写点；此时"不打断当前思维链"只能做到"该工具结果一返回就立即带出"（现状的"下一次工具结果"），或者把长工具本身改成**中途回报**（terminal 180 s 中间态即样板）。这属于 E6 的设计取舍。

## 附录 E：大改候选（用户 2026-09-13：“需要大改”；本附录只登记，不实施）

### E1 选型对比：流式检索 vs 分段检索 vs 通道判活（2026-09-13 用户提问）

先把三个词分开：

| 方案 | 是什么 | 首个结果 | 后续结果 | 主要代价 |
|---|---|---|---|---|
| **流式检索** | 同一个检索请求，服务端用 SSE/分块**持续吐**中间事件与内容；框架逐段读取 | 首个事件/首段内容即可判活（可做到秒级） | 每个后续 chunk 立即转发 | 需要后端真的暴露中间事件；解析/重连/断流处理；DeepSeek `web_search` 的中间事件名需实测确认 |
| **分段检索** | 把一次大检索拆成多个**离散动作**（如 搜索候选 → 逐页抓取 → 逐段抽取），每段返回一个完整小结果 | 第一段（候选）结果即可判活并回传 | 每段完成即回传 | 需要框架自有检索后端/接口；多段编排、去重、质量与现有服务端生成式搜索不同 |
| **通道判活 + 操作预算（b）** | 不改检索返回形态，只在外部加一层"连接/应答"判活（connect/首字节/ack），判活后允许原操作跑其操作预算 | 只能证明"通道可达"，**不产生首个结果** | 仍是一次性整包返回 | 解决不了"结果已分批到达却要等整包"的问题；非流式调用可能 10 s 内既无结果也无首字节 |

**区别一句话**：

- **流式检索 = 改"结果怎么回来"**（一请求、多段到达；可分步即时回报）；
- **分段检索 = 改"检索怎么拆"**（多请求/多动作；每段独立结果与独立重试）；
- **b = 只改"怎么判断通道死了"**（不改结果形态；只做网络不可达的快速判定）。

**建议**：先做**流式检索**（改动最小、保持现有 DeepSeek `web_search` 的服务端生成式搜索质量；当前客户端恰好是"非流式整包"`response.bytes()`，切到流式后可拿到首个事件/首段结果）；如果实测 DeepSeek 流式响应不暴露可用的中间结果/进度，再退到**分段检索**（框架自有检索后端）。**b 只作补充护栏**（对不可流式化的操作做 10 s 通道判活），不能单独满足"每得到一个结果立刻回报"。

**前置实验（S1 探针）**：用同一 `deepseek-v4-flash` key 发一次 **stream=true** 的 `/responses` + `web_search`，记录 SSE 事件序列与时间戳，确认 10 s 内是否出现首个可用事件/首段结果；同时记录 `web_search_call` 的 `searching/completed`、`open_page` 事件形态。该实验决定走流式还是分段。

| # | 大改项 | 说明 | 前置 |
|---|---|---|---|
| E1 | 检索"首个结果"通道（**已定流式优先**） | 流式 `/responses` + `web_search`，首个进度/结果事件 ≤10 s；总预算另计；流式不可行时转分段检索（D2/C1；设计 §3） | 用户已裁决；S1 流式探针 |
| E2 | 结果分步即时回报 | 每得到一个检索结果/结果段就是一个新的结果动作，机械层立即回报；不静默积压到任务结束（设计 §3.3/§4） | E1 |
| E3 | 检索子代理提前收口 | 确定不可达 / 连续确定失败 / 结果已形成 → close activation 并立即回传；墙钟只作最后兜底（D1） | C2（能力级 vs 页面级） |
| E4 | 检索族探针 + 能力级如实汇报 | `tool_availability_check` 加 `retrieval_family`；能力级确定不可达如实汇报 + 有结果即发回（D4） | **无**（FP-2 不改）；若加"run 级不重复尝试/移除车道"再单独回查 FP-2 |
| E5 | 装置生命周期 | agent 阶段结束后显式清理容器内 orz；`--ak max_wallclock` 实机复验；verifier 900 s 定向复现（D5/D6） | 无（可直接做） |
| E6 | 后台完成即时回报（D7 改掉） | 新增机制：常规=当前轮结束/最早安全边界；极端=监控思维链文字，结果到达后首个句号处即时回报（cut-and-continue）；工具等待窗口内不打断 pending 工具，必要时把长工具做成中途回报。需先验证 DeepSeek 分段续写/`reasoning_content` 回传可接受 | 无（用户已裁决 "改掉、必补"）；协议探针先行 |
| E7 | 信号量 acquire 截止 | web_search 排队等待独立短截止 + 排队即时回报；不与执行预算混用（D3） | 无 |
| E8 | 四本时限分账 + 通用契约 | `first_result_deadline` / `operation_deadline` / `total_budget` / `run_wallclock`；等待必须可见、到点必须稳定码（R1/R2） | 设计定稿 |
| E9 | 大改后整轮重跑 | 含第 0 轮 8 题全部重跑；第 0 轮账只作摩擦证据，不作成绩/基线（用户 2026-09-13 裁决） | E1–E8 放行 |

## 附录 F：本轮已落审计文档覆盖图（回答"前面的内容没落文档吗"）

| 内容 | 落点 |
|---|---|
| 新一轮排期 / 批次计划 / 起跑前置四项裁决 | `docs/TB21_V41_GENERATION_ROUND_SCHEDULE_2026-09-13.md` |
| 跑批前容器侧排查（载体/适配器/ACAF/出网/镜像/pin/容器） | `docs/audits/TB21_V41_EVAL_CONTAINER_AUDIT_2026-09-13.md` |
| 第 0 轮起跑、8 题逐题结局、中途体检、硬发现 1（出网大文件不稳）、硬发现 2（0z 资源面被丢） | `docs/audits/TB21_V41_ROUND0_MEMORY_HEAVY_START_2026-09-13.md` §6.1 |
| 第 0 轮中止与补跑起跑（`official-r0-netretry`） | 同 §6.2 / §6.5 |
| 本轮模型动作实况（622 工具调用 / 379 model_output、检索占比、浏览器车道） | 同 §6.3 |
| 0z 资源面漏接修复（`orz ea777918`） | 同 §6.4 / §6.8；`docs/BACKLOG_AND_PRIORITIES.md` `GAP-ORZ-HOST-RESOURCE-SNAPSHOT-DROP` |
| 两处提问核实（Q1 未 submit 真相 / Q2 容器无浏览器）+ 适配器旗标漂移 | 同 §6.7 / §6.9；`GAP-ORZ-ADAPTER-FLAG-DRIFT` |
| agent 超时后 orz 不停（孤儿）+ verifier 900 s 代价 | 同 §6.10 |
| 浏览器车道工具反馈（原因文案/重复尝试/探针面/失败载荷） | 同 §6.11 |
| 墙钟预算按既有修复口径落进执行器 | 同 §6.12；`scripts/run_r0_heavy_official.py` |
| 新需求「机械层即时回报」（设计定稿） | 同 §6.13；`docs/BACKLOG_AND_PRIORITIES.md` **0ac** `GAP-MECH-IMMEDIATE-FEEDBACK`（原引 `:448` 误，实为 `TODO.md:448`，2026-09-13 更正） |
| P2-15 语料冻结 + 阈值校准 | `docs/audits/P2-15_CORPUS_FREEZE_AND_THRESHOLD_CALIBRATION_2026-09-13.md`；`evaluation/corpus-freeze/*` |
| GAP-EVAL-RESULT-SCHEMA-DRIFT 立案与修复 | `docs/audits/GAP_EVAL_RESULT_SCHEMA_DRIFT_FIX_2026-09-13.md` |
| R1 摩擦基线 / 轮次门回测 | `evaluation/round-v41-k1/friction-scan-r1-baseline-2026-09-13.json`；`evaluation/round-v41-k1/round-gate-r1-backtest-2026-09-13.json` |
| 本轮经验泛化：两事故 + 两 candidate 案例 | `docs/incidents/ORZ-PLATFORM-TARGET-001.md` / `docs/incidents/ORZ-VERDICT-EPOCH-001.md`；`docs/cases/harness_environment/*` |
| aarch64 按 0.5.0 同源重建 | `docs/audits/0Y_AARCH64_REBUILD_2026-09-13.md` |
| win-s4 VM 收敛（内部数据不保留，只留 high-nist 环境） | 排期 §3 / `scripts/s4_vm_checkpoint_slim.ps1` / `scripts/LIFECYCLE.md`（无独立审计文档） |
| 全框架时间预算语义 | **本文档**（本文件） |

**缺口说明**：第 0 轮的逐题结局与模型动作没有单独的 `ROUND0_OUTCOME_AUDIT`，它们合并在起跑记录 §6.1–§6.11；如果重跑前需要一份只讲 R0 结论/摩擦的独立审计，可从该记录的 §6.1–§6.11 机械抽取，不必重跑也不影响现有证据。

## 8. 本轮不做项

- 不改任何预算默认值（120 s / 600 s / 900 s / 180 s / 300 s 等）；
- 不改 `probe_scope`、不改错误码、不改 tool_completed 载荷；
- 不改 `ADR-0010`、不改 0t v1.3 裁决、不新立案例；
- 不重建、不重跑、不预拉镜像；
- 不触碰 `D:\AGI`。

下一步的入口是用户对 §4 六个冲突点（C1–C6）的裁决；其中 C1/C2 是两道"老裁决 vs 新口径"的门，必须先裁决再动实现。
