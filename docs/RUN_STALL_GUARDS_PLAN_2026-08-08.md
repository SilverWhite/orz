# Run Stall Guards（挂死守卫）实施计划（2026-08-08）

**自包含交接文档**——新窗口按此实施，勿重开讨论。背景证据链见 §1，归因结论 §2，待办 §3（P0×2 / P1×1 / P2×1），明确不做 §4，前置 §5。

## 1. 背景与证据

**TB hard 稳定性验证（2026-08-08，3+1 题）**：
- cancel-async-tasks ✅ PASS（109 事件，inquiry 1/15 轮，白名单链完整）
- custom-memory-heap-crash ✅ PASS（228 事件，inquiry 4 次，`run_invalidated` 终局但 verifier 1.0——停滞守卫正常兜底）
- dna-assembly：16:02 首 PASS（reward 1.0）→ 17:02 正常完成（0.0，模型 clamp 细节错误）

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
