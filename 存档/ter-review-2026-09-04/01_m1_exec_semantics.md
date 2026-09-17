# TER M1 执行语义切片独立审查（T1.1–T1.5）

> 审查人：子代理 ter_review_a1_exec_semantics（2026-09-04）
> 审查基线：orz `2169e39f`（T1.1–T1.5 最终态；主改动提交 77ec1c14 /
> 1835aa45 / 2169e39f），未混入工作区 0l 并行未提交改动；只读审查，
> 未跑 cargo 测试/构建（工作树被并发改动污染，跑测试不能代表审查基线）。
> 三轴：①设计合理性 ②实现合理性 ③设计与实现的符合性。

## 1. 总览表

| # | 严重度 | 维度 | 主题 | 证据 | 一句话结论 |
|---|---|---|---|---|---|
| S1 | P2 | 设计/符合性 | 10h 绝对兜底与“无自身硬超时”文字口径的张力 | terminal.rs:1495-1535（0b 清扫）；bash_mod.rs:1096 `BACKGROUND_TIMEOUT`；设计稿 §2 P2/P3、§3.1 | 实现自洽且已审计声明“保留”，但设计稿/验收文字未登记“活跃任务 10h 绝对兜底”例外，需随 ADR 转正收口 |
| S2a | P2 | 实现/健壮性 | idle 采样在子进程关闭管道后停止 | terminal.rs:2036-2054（`process_done` 守卫） | daemonize/关管道但存活的后台任务不再被 idle-kill，只剩 10h 兜底，与设计“无活跃兜底”覆盖面有差 |
| S2b | P2 | 实现/健壮性 | Linux CPU 记账仅限同 pgrp，非全树 | xai_tty_utils lib.rs:491-510 / 540-580 | setsid/自成组的忙碌后代不计 CPU；无输出且管道仍开时可能被误判 idle（T1.5“不误杀无输出计算”只在同 pgrp 内被测试覆盖） |
| S3 | P3 | 实现/信息保真 | idle-kill 提醒文案固定渲染“300s” | task_completion.rs:123-131 | 非默认阈值（env/actor 缩短）下提醒文本失真；T2.3 短阈值验收时文案仍显示 for 300s（审计 T1.5 §5 已声明，仍建议快照带实际 idle 时长） |
| S4 | P3 | 实现/文档 | 后端兜底 15s 注释残留 | computer/types.rs:101（“typically 15s”） | T1.2 退役 15s 后该 doc 未同步（terminal.rs/mod.rs 已修，types.rs 漏网） |
| S5 | P3 | 实现/模型面 | “默认超时”渲染用会话级 600s，逐调用普通命令实际 300s | bash_mod.rs:1666-1673 模板；tools.rs:234-247 inject | T1.4 删除分层文案后渲染源与注入源仍不同源；180s 预算遮蔽下无碍，预算=0 或逃生阀路径会向模型报错值 |
| S6 | P3 | 符合性 | “封闭默认”依赖 host 注入 allow_background_operator=false | bash_mod.rs:220-230 字段 doc；tools.rs:127-130 | 纯 BashParams（无注入）默认 `&` 面开着，与“模型面封闭默认”表述并存；T1.3 审计 §7 已声明，建议随 ADR 转正裁定 |
| S7 | 提示 | 实现 | env 旋钮与“单一生效源”的关系 | terminal.rs:54-67 / 1104-1114 | GROK_FOREGROUND_BLOCK_BUDGET_MS、GROK_MAX_FOREGROUND_BLOCK_MS、GROK_IDLE_KILL_TIMEOUT_MS 均为显式 override 非第二默认，语义可接受；建议 T0.1 核对表“其它源”列显式列全 |

## 2. 详细条目

### S1（P2）10h 绝对兜底与“无自身硬超时”的登记张力

实现：终端 0b 清扫对 `p.start_time.elapsed() > p.timeout.min(BACKGROUND_MAX_RUNTIME)`
的后台任务无条件 SIGTERM/SIGKILL（signal=`timeout`（显式后台正向超时 <10h）或
`max_runtime`（自动后台化/未设超时撞 10h）），即**活跃进程也会在 10h 被硬杀**。
transition_to_background 统一把 auto-bg/用户后台化的任务 timeout 退役为 10h。

问题：设计稿 §2 P2“自加硬杀墙钟全部删除”、P3“工具执行层无自身硬超时”、§3.1 第 4
步兜底只写“无输出活跃 300s 才 kill”、TODO2 T1.4 验收“无任何满 timeout 杀活跃命令
代码路径（评测墙钟除外）”均未登记“10h 绝对安全兜底杀活跃命令”这一例外。T1.4/T1.5
审计以“保留绝对兜底”带过，但仓库自身权威规则（索引 §0.1：实现偏离 ADR 须标 gap）
下该偏差缺正式登记。

建议：随 a2 已报的 ADR §14.53 转正（F1）一并写入“绝对安全兜底例外”（10h
max_runtime 只防失控泄漏，不参与评测时间窗），并把 TODO2 T1.4 验收句改为
“无任何分层 timeout 杀活跃命令（评测墙钟与 10h 绝对安全兜底除外）”。

### S2a（P2）idle 采样在管道关闭后停止

`poll_process` 的采样守卫：`!process_done && running && backgrounded && try_wait==None`，
其中 `process_done = stdout_eof && stderr_eof`。子进程 daemonize/nohup 关闭继承管道
但继续存活时，采样永久停止——该任务既不会被 idle-kill，也不会因无活跃被回收，只撞
10h 兜底或会话结束。与“后台任务 300s 无活跃 → kill”设计覆盖面不一致。

建议：登记该限制；如需完整覆盖，对管道已关的后台任务退化为按进程树 CPU/文件 mtime
采样，或明确“daemonize 形态由 10h 兜底 + session 清理负责”。

### S2b（P2）Linux CPU 记账粒度 = pgrp，非全树

Windows 用 Job Object 记账（覆盖 Job 内全部后代）；Linux 只扫 `/proc/*/stat` 汇总
`pgrp == pgid` 的 utime+stime。setsid/自成进程组的忙碌后代不计 CPU。组合场景：管道仍
开、无输出、CPU 消耗全在 pgrp 外的子进程 → CPU 读数零增长 → 300s 后误杀
（T1.5 验收“无输出忙循环不误杀”只测了同 pgrp 场景）。方向偏“可能误杀”，与
“宁可不杀也不误杀”的平台原则相反。

建议：补一个 Linux 跨 pgrp 忙子进程用例；或把“仅同 pgrp CPU 可判”视作部分未知并
在 CPU 面不确定时回退不判 idle（对齐其它平台口径）。

### S3（P3）idle-kill 完成提醒硬编码 300s

`format_bash_completion` 对 `signal=idle_killed` 固定调用
`idle_kill_reason(DEFAULT_IDLE_KILL_TIMEOUT)`。env/actor 缩短阈值（T2.3 场景必用）时
提醒仍写 “no output growth or CPU activity for 300s”。T1.5 审计 §5 声明该边界（精确
时长走 tracing/审计字段），但仍建议 TaskSnapshot 增加 idle 时长或实际阈值字段，
避免模型/验收脚本读到误导文案。

### S4（P3）types.rs 残留 “typically 15s”

`computer/types.rs:101` `foreground_block_budget: None` 的 doc 仍写 “use the terminal
backend default (typically 15s)”；T1.2 后后端兜底为 180s（terminal.rs:60）。T1.2 复审
处理只改了 terminal.rs 与 mod.rs。P3 文档残留。

### S5（P3）“默认超时”渲染源（600s）与逐调用分层注入（300s）不一致

模型面文案（内部 auto-bg 模板）渲染 `default_timeout_ms` =
`effective_default_timeout_ms(params)`：主线 BashParams `timeout_secs=600` → 600s。
而宿主 `inject_terminal_default_timeout` 对普通命令注入 300s、程序 600s（仅未显式
timeout 时）。正常主线性 180s 预算先到（渲染 “after 180s” 仍准）；但预算=0
（timeout-only auto-bg）时普通命令实际 300s 自动后台化，文案说 600s；逃生阀
（auto_bg=false）时普通命令实际 300s kill，文案“Default: 600s enforced”。T1.4 审计
未登记此渲染/注入源不一致（它删除的是“300s ordinary/600s program”静态句，未把
默认值渲染接到 terminal_tier_default_timeout_ms）。

建议：把 schema/模板的 default_timeout 渲染改为与 per-call 注入同源
（或在描述中说明分层默认），与 T0.1 §4 “timeout 分层数值收敛为
terminal_tier_default_timeout_ms 纯函数单一源”的方案对齐。

### S6（P3）“封闭默认”在纯 struct 层并不封闭 `&`

BashParams 默认 `allow_background_operator=true`；`&` 拒绝由该键控制。orz 独立
（纯 struct，无 host 注入）时 `&` 面开着（hide=true 只挡 `is_background` 参数）。
主线封闭由 `run_terminal_cmd_tool_params()` 显式注入 false 保证。T1.3 审计 §7 已声明
该边界。P1“独立可用=完整可用”与“模型面封闭默认”的表述建议随 ADR 转正统一：
要么把该键也收敛为 resident false（连同逃逸场景复核），要么明确“封闭=orz-host 装配
默认、codegen 库层默认保持兼容开放”。

### S7（提示）env override 与单一生效源

GROK_FOREGROUND_BLOCK_BUDGET_MS（终端兜底）、GROK_MAX_FOREGROUND_BLOCK_MS（不可后台
化前台上限）、GROK_IDLE_KILL_TIMEOUT_MS（idle 阈值）都是显式运维/测试 override，
不改默认源语义，设计上可接受；建议 T0.1 核对表“需要退役/同步的其它源”列把三者与
“override≠默认”的关系显式列出，避免后续审查误判为第二默认源。

## 3. 符合性清单

| 验收项（TODO2） | 证据（提交态 2169e39f） | 判定 |
|---|---|---|
| T1.1 默认构造即 true；显式 false 关闭态不回归 | BashParams `Default`+serde `default_true`（bash_mod.rs:196-260）；测试 default_enables_auto_bg:4946 / explicit_false_disables_auto_bg:4975；registry 关闭态用例保留 | 符合 |
| T1.1 主线生效值不变、冗余注入删除 | tools.rs:125-135 仅注入 allow_background_operator/timeout_secs/max_timeout_secs；参数测试 run_terminal_cmd_params_s5_2:757 断言两键不注入 | 符合 |
| T1.2 不传参数 budget=180_000、15s 后端退役、schema 描述同源 | DEFAULT_FOREGROUND_BLOCK_BUDGET_MS=180_000（bash_mod.rs:600）；effective_foreground_block_budget None→180s（:1455-1460）；terminal FOREGROUND_BLOCK_BUDGET=180s（terminal.rs:60）+ backstop 守卫测试:3525；default_budget_180s_materializes_auto_bg_foreground_request:3566 | 符合（除 S4 文档残留） |
| T1.3 hide 默认 true、schema 无 is_background、& 仍封 | BashParams 默认 true（bash_mod.rs:248-264）；exported_input_schema 移除属性与 required（:1568-1655）；测试 default_closes_is_background_surface:5074 / serde_omission…:5321；registry bash_definition_closed_by_default:3781；主线 allow_background_operator=false（tools.rs:127-130） | 符合（纯 struct `&` 面见 S6） |
| T1.4 分层 timeout 不再杀活跃命令；解析超时=auto-bg deadline；timed_out 仅逃生阀/评测墙钟 | poll_process 预算与超时分支均先 auto-bg（terminal.rs:1984-2012）；kill 分支仅 auto_bg=false 可达；transition_to_background 统一退役 10h（:2105-2135）；bash run() 取消 timeout≤budget 门（:2327-2370）；测试 timeout_within_budget_requests_auto_bg_not_kill:3609 / test_auto_backgrounded_task_survives_original_timeout_deadline:4237；模型面模板已删除分层杀文案（:1660-1720 守卫残留仅逃生阀/禁用面） | 符合（10h 例外见 S1，默认值渲染见 S5） |
| T1.5 300s 无活跃 kill+idle_killed+自描述提醒；不误杀无输出计算；阈值参数化 | ActivitySampler tick（terminal.rs:171-207）；DEFAULT_IDLE_KILL_TIMEOUT=300s（:82-88）；idle_kill_background_task（:1472-1492）；KillReason::IdleKilled（bash_mod.rs:406-448）；task_completion 文案:123-131；采样器 5 用例+实机 2 用例（:3827-3990）；env/actor 参数化+0=禁用 | 符合（提醒文案见 S3；覆盖面见 S2a/S2b） |

## 4. 审计文档真实性核对

- TER_T1_2_REVIEW_HANDLING 的 F1–F3 在最终态均已落实：terminal.rs 注释改为
  “Backend default is 180s…只兜底直连请求”；helper 已改名 `effective_fg_wait_ms`；
  MAX_FOREGROUND_BLOCK 注释为 “request 预算（resident 默认 180s）+ 后端兜底”口径。
- 审计声称的测试名均可在提交态源码找到（见上表证据列）。测试**数量**无法在只读环境
  复跑验证：终端文件含 69 处 `#[test]`/`#[tokio::test]`（47 passed/5 ignored 需按
  filter 与平台门控确认）；bash_mod.rs 单文件 172 处（231 桶计数含 registry/types.rs
  等多文件）；task_completion.rs 49 处与审计 49 passed 一致。全量数字请以 T1.13 门
  （a3 切片）在干净工作树的复跑为准。
- 审计声称“残留扫描无 300s for ordinary / 600s for program / Timeout enforcement
  kills 于默认面”属实：提交态 grep 仅命中模板中逃生阀/禁用面分支与注释/测试。
- 审计边界声明与代码一致：idle-kill 提醒固定 300s（§5）、journal 侧 idle_killed
  接线归 T1.13（本切片只见到 terminal 快照层，事件链生产者在 orz-loop/orz-assurance，
  请 a3/根代理在 T1.13 复核）、10h/逃生阀保留。

## 5. 无法验证项（需干净构建/实机）

1. 各步测试计数（224/226/229/230/231、40/47、49、orz-host 1、orz-workspace 7）的
   精确复跑——需干净工作树 + cargo。
2. ActivitySampler Windows Job 实机 CPU 读数与 Linux /proc pgrp 读数正确性
   （T1.5 声称 Windows 实机 sleep/busy 用例通过；本次只做静态核对）。
3. 无输出跨 pgrp 忙碌子进程在 Linux 上是否误杀（无现成用例；见 S2b）。
4. 10h 兜底实际触发路径（无人愿意跑 10h）。

## 6. 结论摘要

设计合理性与实现合理性总体良好：默认收敛单源化、去硬杀语义、idle+CPU 兜底在
**主线性（orz-host 装配态）**自洽，测试结构与审计声明一致，无 P0/P1 功能性缺陷。
主要缺口是治理登记（S1：10h 例外未入设计/ADR，与 a2 F1 同源）与三个覆盖面/信息保真
边界（S2a/S2b/S3）。建议处理方式：S1/S6 随 ADR 转正批登记；S2a/S2b 补文档限制或
实现补强；S3/S4/S5 小修入下一轮 review-handling。
