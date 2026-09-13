# 框架时间预算语义审计（2026-09-13）

> 性质：只读调查 + 结论落档。本轮**不改代码、不改预算值、不改契约、不重建、不重跑**，也不触碰 `D:\AGI`。
>
> 触发（用户 2026-09-13 口径）：预算是**运行预算**，不是**等待预算**；结果一旦出现就是新的结果动作，机械层应立刻回报；不只是检索部分，其他部件也一样；查全框架各部分的预算，强制等待化的部分要全改掉；**先调查并写入文档，不直接行动**。
>
> 方法：`CLI_PROJECT_INDEX` 路由 → ADR/设计/实施审计回查 → 源码逐一核对 → 用 TB 2.1 第 0 轮 6 个 run 的 `.gsa` journal 实测校准。
>
> 关联：`GAP-MECH-IMMEDIATE-FEEDBACK`（`docs/BACKLOG_AND_PRIORITIES.md:448`，设计定稿待放行）。本审计是它的证据基座，同时**修正其两条验收判据与两条已入档计数口径**（见 §4 C5/C6、§6.5）。

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

- 用户新口径"检索/网络 10 s 没结果就报网络问题"，与 **2026-08-29 已定案的 web_search 全程 120 s + connect 10 s** 冲突。DeepSeek `/responses` 的服务端生成式检索**单次 15–70 s 属正常**（`COMMAND_TIMEOUT_AND_WEB_SEARCH_TIMEOUT_RESEARCH_2026-08-29.md:42-62`）；第 0 轮实测 web_search **p50 = 29.9 s、96.6% 的调用 ≥10 s**（§3.2、附录 A）⇒ 10 s **总**截止会误杀绝大多数正常搜索。
- "探针扩到检索族 + 不可达不重复尝试 + 失败计数反馈"，与 **0t 双车道设计 v1.3 / `ADR-0010 §14.65` 的 FP-2 非目标**（"不做任何形式的『勿重试』教学、失败计数反馈或机械降级提示"）冲突（`docs/RETRIEVAL_SUBAGENT_DUAL_LANE_DESIGN_2026-09-09.md:126-128`、`:199-205`）⇒ 需先修订裁决，或把口径精确收窄为"**能力级确定不可达**"（与 FP-2 覆盖的"页面级导航失败 / 可拉起但实质不可用"区分）。

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

**与既有定案的冲突**：`COMMAND_TIMEOUT_AND_WEB_SEARCH_TIMEOUT_RESEARCH_2026-08-29.md:42-62` 已确认 DeepSeek `/responses` 的服务端生成式检索单次 15–70 s 属正常、120 s 是共识下限、30 s 级会频繁误杀。**10 s 总截止 = 误杀 96.6% 的正常调用**，不建议直接执行。

**建议口径**：拆成两段——

1. `reachability_deadline ≤10 s`：连接 / DNS / TCP / 进程在位 / 明确网络错误 → 立即返回稳定码（`capability_unreachable` / `network_unreachable`），不等待；
2. `operation_deadline = 120 s`：服务端生成式检索的既有校准值；出结果立即返回，到期返回结构化错误。

注意：当前 `web_search` 是整包 `response.bytes()`，服务端完成前没有可区分的"首字节"读数；要在 10 s 内区分"连不上"与"服务端正在多轮检索"，需要新增可观测的握手/进度面，不能只把 120 s 改成 10 s。

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

**证据**：`TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md` §3.1 条 3："完成 → 经 `surface_bg_completion_reminders` 在下一个工具结果顶部带回（一次延迟回报，模型感知'180s 提醒一次 → 结束带回'）"。

**判定**：T3 设计内延迟。结果不会丢，但确实不是"结果一出现就回报"。若按用户统一口径改为即时注入，会改变现有轮边界与注入面（可能打断当前模型轮），需单独裁决；当前至少是有界延迟（下一个工具边界）。

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
| C1 | 10 s 总截止 vs web_search 120 s 已定案 | 拆两段：`reachability_deadline ≤10 s` + `operation_deadline = 120 s`；10 s 总截止 = 96.6% 误杀 |
| C2 | FP-2 非目标 vs "确定不可达即时回报 / 不重复尝试" | 区分能力级与页面级；修订 `ADR-0010 §14.65` / 0t v1.3 非目标措辞 |
| C3 | 预算到期 vs 动作终止 | 墙钟到期必须终止容器内动作；只取消 harness 等待不够（D5/D6） |
| C4 | 后台完成回报时点 | 现状下一次工具边界；是否改即时需裁决（D7） |
| C5 | `GAP-MECH-IMMEDIATE-FEEDBACK` 验收"检索类 `wall_ms` p99 ≤10 s；`subagent_wallclock_timeout_mid_tool=0`" | 按本审计修正：web_search 用**可达性**指标；操作时长按服务端检索分布设；`mid_tool=0` 保留为目标，但依赖 D1 提前收口 |
| C6 | 第 0 轮已入档计数 | "5 次 `subagent_wallclock_timeout_mid_tool`"→ 实际 **3 次 `subagent_timeout` 激活 + 2 条合成 mid-tool 收口**；"web_search 单次最高 26.8 s"→ 实际 **max 37.2 s / p50 29.9 s**（附录 A） |

## 5. 建议的修正批次（仅登记，待用户放行；本轮不实施）

| # | 内容 | 验收判据（可机械核对） |
|---|---|---|
| R1 | 通用契约：机械层对任何模型请求"及时且有信息量"地返回；等待必须有可见日志/事件；确定性不可达走快速路径；预算只做上限 | 新增回归钉子：任何等待型调用在截止后必须产出带稳定码的结果；无"到点前零事件"的等待路径 |
| R2 | 三类时限命名与分层：`reachability_deadline`（≤10 s）/ `operation_deadline`（按操作校准，web_search 120 s）/ `run_wallclock`（官方值） | 每个预算点标注属于哪一类；三类不互相冒充 |
| R3 | 检索子代理提前收口 + 中间回报：确定不可达 / 连续确定失败 / 结果已形成 → close activation 并立即回传 | 第 0 轮同类场景不再出现"整 600 s 只在到期返回"；`subagent_timeout` 只作最后兜底 |
| R4 | 探针扩 `retrieval_family`（浏览器 / 搜索引擎 / web 通道）+ run 级车道状态 | 每 run ≥1 条含检索族结论的 `tool_availability_check`；**前置：C2 修订先落** |
| R5 | web_search 信号量 acquire 独立截止 + 排队即时回报 | acquire 等待有稳定码与独立上限；不再被 900 s 外层包住 |
| R6 | 装置生命周期：`--ak max_wallclock` + agent 阶段结束后显式清理容器内 orz；verifier 900 s 定向复现 | "带 / 不带 wallclock"对照：带墙钟的次不再出现孤儿与 verifier 吃满 |
| R7 | 后台完成即时回报（裁决点） | 若采纳：完成即产生机械事件/注入；若不采纳：保留 D7 现状并登记为"有界延迟" |
| R8 | 第 0 轮计数更正入档（C6） | 起跑记录与索引中的数字改为附录 A 口径 |

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

## 8. 本轮不做项

- 不改任何预算默认值（120 s / 600 s / 900 s / 180 s / 300 s 等）；
- 不改 `probe_scope`、不改错误码、不改 tool_completed 载荷；
- 不改 `ADR-0010`、不改 0t v1.3 裁决、不新立案例；
- 不重建、不重跑、不预拉镜像；
- 不触碰 `D:\AGI`。

下一步的入口是用户对 §4 六个冲突点（C1–C6）的裁决；其中 C1/C2 是两道"老裁决 vs 新口径"的门，必须先裁决再动实现。
