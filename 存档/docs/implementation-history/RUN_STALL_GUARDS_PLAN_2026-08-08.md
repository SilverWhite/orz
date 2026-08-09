# Run Stall Guards（挂死守卫）实施计划（2026-08-08）

> Archive metadata: original_path=`docs/RUN_STALL_GUARDS_PLAN_2026-08-08.md`; archived_at=`2026-08-09`; final_status=`implemented/transferred`; superseded_by=`adr/ADR-0010-fusion-runtime-and-agent-architecture.md`; authority=`implementation history only`.

**自包含交接文档**——新窗口按此实施，勿重开讨论。背景证据链见 §1，归因结论 §2，待办 §3（P0×2 / P1×1 / P2×1），明确不做 §4，前置 §5。

## 1. 背景与证据

**TB 混合稳定性/挂死机制诊断（2026-08-08，pre-beta 探索性；2 hard + 1 medium，含 dna 重跑）**：
- cancel-async-tasks ✅ PASS（109 事件，inquiry 1/15 轮，白名单链完整）
- custom-memory-heap-crash（官方难度 medium）✅ PASS（228 事件，inquiry 4 次，`run_invalidated` 终局但 verifier 1.0——停滞守卫正常兜底）
- dna-assembly：16:02 首次出现 PASS（reward 1.0，同时带 AgentTimeoutError）→ 17:02 正常完成（0.0，模型 clamp 细节错误）。该序列用于定位挂死与终局取证，不作为稳定通过证明。

成绩解释与 hard/medium 分类勘误见 `docs/TERMINAL_BENCH_2_EXPLORATORY_SCORE_AUDIT_2026-08-08.md`。

**dna-assembly 16:02 挂死（本计划触发事件）**：
- orz 完成工作（primers.fasta 正确，verifier ctrf 1/1 passed）但进程 30min 不退出 → harbor AgentTimeoutError 超时杀
- orz.txt **0 字节**（`-p` 路径无 tracing subscriber，warn/error 全静默丢失）
- **无 journal 拷贝**（`.gsa/runs` 不存在或拷贝失败——取证盲区）
- 17:02 tracing 版：正常退出（105 行 orz.txt + journal 拷贝成功）——**挂死未复现（偶发）**

**复现实验（排除压缩路径）**：
- 实验 1：239K 上下文（cache_hit 238,976）、7 模型轮 20 工具轮——正常退出（节奏压缩被 min_rounds=20 冷却挡住）
- 实验 2：**5 次 context_compressed 真实触发**（265K–304K）、正常退出——**A6/C.1/C.2 压缩路径无挂死**

**环境修复（已生效，本次会话）**：
- `C:\Users\1\.wslconfig`：`[experimental] autoProxy=false`（Docker WSL 引擎启动失败根因——NAT 模式 WSL 不支持镜像 localhost 代理 127.0.0.1:7890）
- `D:\tb-eval\build_orz_aliyun.sh`：aliyun Debian 镜像构建变体（USTC 网关 502/超时期间用）
- harbor 调用：`harbor.exe` + `PYTHONPATH=D:\tb-eval`（非 `python -m harbor`）

## 2. 归因结论（严格评判后定稿）

**挂死不发生在"工具反馈"**。orz 工具调用是同步 await：工具未返回 = 无判定发生；工具已返回 = 判定正确。挂死两个可能：

| 候选 | 现状 | 判定 |
|---|---|---|
| a) 卡在模型请求（API 挂起） | 10min 请求超时 + 90s idle watchdog + 30min 重试总预算已存在 | 30min 整挂死与"预算耗尽应报错退出"矛盾（0 字节无报错）——不能完全自洽 |
| **b) 卡在工具执行（真实缺口）** | **orz 除 run_tests（F-09 30min）外，其他工具执行无超时**——bash/read_file 等挂起则 run_host_tool 无限等待 | **与 16:02 现象完全吻合**（静默 + 30min 被杀） |

**"5min 无输出 → 改判上一个工具为否 + 通知模型"机制不采纳**（用户提出，严格评判后否决）：
1. 归因不可靠——API 层挂死时上一个工具判定**正确**，改判是错误惩罚（误导模型重试已成功操作）
2. 同步 await 下工具未返回 = 无判定可改
3. 污染 journal 证据链（已记录的工具结果不应被事后改写）

**正确语义**：执行超时 = 执行失败（一次性事件，判定从未发生）。

## 3. 待办清单（实施顺序）

### P0-1 工具执行级超时（对治挂死主源）
- **位置**：orz-loop controller `run_host_tool` / orz-host `call_tool` 包装层
- **语义**：工具执行 **5min 超时**（默认，可配）→ **kill 进程树**（TaskKill `/T /F` 树杀先例：先树后主——父死后孤儿重父化致 `/T` 找不到树）→ 返回**失败结果**（含 timeout 原因）→ 通知模型继续循环
- **先例扩展**：run_tests 30min 超时（F-09，2026-08-07）扩展到**所有**工具；权限等待 300s 已有（不重复）
- **测试**：fake 挂起工具 + 短超时 → 失败结果 + 模型继续 + journal 事件链完整（tool_completed 带 timeout 原因）
- **注意**：kill 后无残留子进程（孤儿重父化测试）

### P0-2 max-wallclock（进程内总时间预算）
- **位置**：orz-bin 新 flag `--max-wallclock <秒>`（env 透传 `ORZ_MAX_WALLCLOCK`——先例：`--allow-write`→`ORZ_ALLOW_WRITE`）
- **语义**：**模型不可见**（不进 prompt——时间压力诱导过早提交，premature-completion 反模式；对标 TB 官方 900s/SWE-bench 2h 全部框架层不可见）
- **与轮次预算对比**：轮次可见（动作规划语义，ADR-0008 每轮告知）；时间不可见（资源守卫语义，watchdog 同级）
- **行为**：到点 → **优雅终局** `run_invalidated{reason: wallclock}` + journal 完整 + 正常退出
- **与 harness 超时的关系**：预算 = harness 超时 − 余量；**替代不了**进程外硬杀（最后兜底），是前置补充——被杀（无终局无 journal）→ 优雅终局（有终局有 journal）
- **测试**：短预算 → 终局事件 + journal replay valid

### P1-1 全局停滞看门狗（进程级 heartbeat）
- **位置**：controller 层
- **语义**：任何 **5min 无活动**（工具执行/模型请求/重试链任一处静默）→ 优雅终局 `run_invalidated{reason: stall}`
- **与 max-wallclock 互补**：活动检测（挂死更早触发）vs 总时长（健康慢任务到点）
- **与 runtime_stagnation_guard 区分**：后者是**内容**停滞（ngram 重复输出检测——已实证工作：custom/dna 17:02）；看门狗是**活动**停滞（无任何输出）
- **测试**：挂起 fake → 短看门狗 → 终局 + journal valid

### P2-1 harness 挂载卷（消灭"无 journal"盲区）
- **位置**：`D:\tb-eval\tb_agents\orz.py`（populate 环节）或 harbor 环境配置
- **语义**：`.gsa` 挂载到容器外卷（`D:\tb-eval\jobs\...` 或独立目录）——**绕开拷贝环节**，容器内写盘即宿主可见
- **评估**：harbor 是否支持自定义挂载（docker-compose-mounts.json 注入点）；不支持则退化为 populate 时 `docker cp`（修复现有 cp 失败路径：确认 `.gsa` 实际位置 + 容器存活时序）
- **TB/SWE-bench harness 共用**（SWE-bench 侧 `D:\swebench-eval\run_swebench.py` 同款改造）

## 4. 明确不做

- **判定翻转机制**（改判上一个工具为否）：归因不可靠 / 无判定可改 / 污染证据链——§2 已述
- **反例询问绑定 journal checkpoint**：journal 已**每事件 fsync**（write_event `write_all+flush+sync_all`）——"开始保存"无增量；gate 触发点快照覆盖不了 gate 前挂死；被 P1-1 终局路径吸收（任何终局都保记录）
- **压缩路径改动**：复现实验排除（5 次压缩正常退出）

## 5. 前置 / 关联

1. **orz main.rs tracing init（`-p` 路径）未提交**——本次会话改动（cli_mode 分支加 tracing_subscriber，run_tui 同款，`try_init` 幂等）——**新窗口先提交**（与 P0 实施同一提交或独立提交均可，但必须带上——挂死取证依赖它）
2. 环境修复存档：`.wslconfig`（autoProxy=false）/ `build_orz_aliyun.sh`
3. 关联文档：`docs/TERMINAL_BENCH_2_EVAL_2026-08-08.md`（TB 适配+成绩）/ `docs/INQUIRY_FIX_AND_BLACKBOARD_PARTITION_2026-08-08.md`（A4-A6+C.1/C.2）
4. 验证基线：orz-loop 122 / orz-host 91 / assurance 95 / tui 174 / codex 33 / bin 4 全绿、clippy 零、conformance 7 + runtime 114 + check_repository valid
5. 下一步（原始计划）：TB medium 池补样本 → TB 全量 89（含 C.3 压缩数据闸门裁决）→ SWE-bench 剩余 14 题

## 6. 实施记录（2026-08-08 晚，本窗口完成 P0-P2）

**P0-1 工具执行级 5min 超时（已完成 + 审查修复）**：`ToolError::Timeout` 新变体 + `OrzHost::call_tool` 包装 `tokio::time::timeout(TOOL_CALL_TIMEOUT=300s, toolset.call)`——超时 → `global_process_scope().kill_active()`（Windows 每子进程 Job Object `TerminateJobObject` 树杀，含孙进程）→ 返回 `ToolError::Timeout`（原因含预算）；controller Err 分支特判输出"tool TIMED OUT and was killed"通知模型继续，journal `tool_completed{status:error,error}`。`with_tool_timeout` builder 可配；`-p` 路径 `ORZ_TOOL_TIMEOUT_SECS` env 逃生阀（0=禁用；交互路径文档化）。**2026-08-08 三代理审查修复（F1）**：初版用 `kill_all()`——它**闩闭**全局 scope（契约="进程退出时调一次"），一次工具超时后所有后续 spawn 注册即死（terminal.rs 忽略 register 返回值）→ 会话进程型能力报废。改 `ProcessScope::kill_active()`（杀当前存活组**不闩闭**，xai-tty-utils 新增），测试 follow-up 从 read_file（不 spawn，测不出闩闭）升级为 `run_terminal_cmd` 真进程调用。测试：orz-host `call_tool_timeout_kills_process_tree`（真挂起命令（python 载体——挂起实体非 bash，措辞修正）+ 孙进程 pidfile 验证 + **进程型**后续调用存活；预算 500ms→2s 防 python 冷启动 flaky）；orz-loop `tool_timeout_is_journaled_and_loop_continues`。

**P0-2 max-wallclock（已完成）**：`--max-wallclock <sec>` → `ORZ_MAX_WALLCLOCK`（`--allow-write` 先例）；模型不可见；`run()`/`run_plan()` 把整个 run future（plan 阶段 + 执行）包进 `tokio::time::timeout`；到点 → **丢弃 run future** → `record_guard_terminal("wallclock")`——flush（recorder 通道 FIFO，被丢 controller 已排队的写先落盘）→ 读文件恢复链位置（seq+hash）→ 追加 `run_invalidated{status:wallclock}` → 正常退出 0。schema status enum += `wallclock`。测试：parse fail-closed + 直接终局写 + 真丢 future 恢复链（flaky 修复：test_dir 同秒撞目录 + 100ms 预算在并行负载下先于首事件，改 400ms）。

**P1-1 全局停滞看门狗（已完成 + 审查修复）**：新 `ActivityClock`（单调源——`Instant` 基准 + AtomicU64 毫秒 elapsed，零锁；**2026-08-08 审查修复 F4**：初版 SystemTime 墙钟，休眠/NTP 前跳会误杀健康运行）——**打点源**：controller 每个 journal 事件（EventWriter.record）+ 工具执行前后 + **run_tests 执行期 60s 周期打点**（**2026-08-08 审查修复 F2**：初版仅前后打点，30min 合法长跑（F-09 上限）执行期内 idle 累计超窗口 → 看门狗误杀，注释声称的保护不存在）+ 网关流每帧（`generate_stream` 新 `heartbeat: Option<&ActivityClock>` 参数，transport 每 SSE 帧打点——**reasoning-only 增量也打**，controller 永远看不到的长时间 max-effort thinking 流只有 transport 能保活；fake 每 chunk 镜像）。`ORZ_STALL_TIMEOUT`（秒，默认 **360**，`0` 禁用；**审查修复 F3**——初版 300s 与 TOOL_CALL_TIMEOUT 相等，挂起工具上两守卫竞态且 stall 胜出时 kill 树从不执行；错开为 360s 后挂起工具**确定性**走 P0-1（树杀+继续），stall 只兜工具外静默）→ orz-bin `stall_fire`（500ms 轮询 idle ≥ 窗口）→ select 丢 run → `record_guard_terminal("stall")`。schema status enum += `stall`。测试：静默 gateway 短窗口 → run_invalidated{stall} + replay valid；活跃流（fake 每 chunk 打点）不误杀（**关键坑**：两轮文本必须不同——相同文本会触发内容停滞 guard 真终局 run_invalidated，测试错终局；窗口必须清 fsync 簇——每事件 sync_all，400ms/1.2s 都误杀，2.5s 稳）。

**P2-1 harness 挂载卷（已完成 + 审查修复）**：harbor 原生支持 `--mounts-json`（Docker Compose volume 格式，`config.environment.mounts` 亦可；注意 `--mounts-json` 实为 `--mounts` 的 deprecated alias——文档用规范旗标 `--mounts`）——bind host 卷 → 容器 `/orz-gsa`（固定路径；任务 workdir 因镜像而异不可依赖）。adapter `gsa_volume` kwarg（host 路径，__init__ mkdir + **best-effort 卷清理**（审查修复 D2-4：容器内 7 天 sweep 经 symlink 只扫自己 trial，host 卷无限累积——保留最新 5 个 trial 子目录））；run 脚本：`/orz-gsa` 存在可写时 `mkdir -p /orz-gsa/<trial-uuid> && ln -s /orz-gsa/<trial-uuid> $PWD/.gsa`（busybox 无 `-n`，先 `[ ! -e ]` 守卫不动已有 .gsa——注释与代码一致）——journal 实时落宿主卷，**harness 杀容器不再丢 journal**（dna 16:02 盲区根治）。`_find_journal` 优先卷内 `<trial-uuid>/runs`（精确，无跨 trial 排序歧义），回退旧拷贝。trajectory 补 run_invalidated/run_cancelled 终局 system step。**机制实测**：alpine 容器 bind+symlink 写 journal → 宿主数秒内可见。**P0-2 接线（审查修复 D2-2/P2-3）**：adapter 新增 `max_wallclock` CliFlag（`--max-wallclock` / `ORZ_MAX_WALLCLOCK`）——launch 传 `--ak max_wallclock=<任务超时−余量>`（如 harbor 900s → 840）即获得优雅终局；未设置则仍由 harness 硬杀兜底（P2-1 卷保住 journal 但无终局事件）。**SWE-bench 侧发现**：`run_swebench.py` 跑的是**宿主原生 orz.exe**（非容器）——worktree 即宿主 FS，`.gsa` 本就实时可见，`keep_journal` 仅归档——**无盲区，无需改造**（计划 §3 P2-1 的"同款改造"前提不成立，记录）。

**实施偏差记录**：P1-1 计划写"位置：controller 层"——活动打点确在 controller/gateway 层（按计划），但**看门狗定时器在 orz-bin**（controller 无法丢弃自己的 future；终局复用 P0-2 的文件恢复机制）。TUI/stdio 路径无此看门狗（有交互用户；acp_server 未接线，记录）。P0-1 kill 是**进程级全局**——并发后台命令也会被杀（记录取舍：挂死毒化会话，留孤儿更糟）；`kill_active` 不闩闭（F1）后误杀面从"毁掉会话剩余派生能力"降为"单次并发误杀"。**P0-1 对交互路径生效**（acp_server 构建 OrzHost 用默认 300s——TUI 用户跑 >5min 命令会被杀；`-p` 路径有 `ORZ_TOOL_TIMEOUT_SECS` 逃生阀（0=禁用），TUI/stdio 无逃生阀，记录）。guard 与 run 同刻完成的选择竞态由 `record_guard_terminal` 的"已终局跳过"吸收。P3 记录（审查）：record_guard_terminal 自身无超时（写盘卡死类停滞无解，磁盘挂则什么都写不了）；stall/wallclock 路径不 kill 工具树（挂起子进程残留至容器死/或 z 退出，Job Object KILL_ON_JOB_CLOSE 兜底）；守卫失败时 `?` 跳过 shutdown（数据已 flush，仅缺终局+exit 1）；极短 wallclock（< 首事件落盘）报错 exit 1 而非优雅终局（无数据可丢）；双 guard 并发写终局毫秒窗口（后写被 TerminalAppended 拒绝，journal 仍单终局有效）。
