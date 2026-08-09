# run_tests 安全复核（V11-IMPL-005，2026-08-09）

> 依据：ADR-0010 §3.8「受控 run_tests / hidden-test 反馈环」。本文件是复核记录，
> 不修改实现；结论中的改动项排入后续实现切片。

## 1. 复核范围

对 `run_tests`（D-9 反馈环）按 ADR §3.8.2 的六个控制维度逐项核对：

1. execution permission
2. sandbox / Job Object
3. timeout
4. 输出上限与脱敏
5. workspace delta / journal 记录
6. hidden-output 泄露

## 2. 逐项结论

| 维度 | ADR §3.8 要求 | 当前实现 | 结论 |
|---|---|---|---|
| execution permission | 受控代码执行，必须经过 execution permission | `controller.rs` run_host_tool 对 `run_tests` **跳过 permission gate**（注释：固定命令 host 拥有、模型无 argv） | **差距**：`ToolDispatcher::is_file_edit("run_tests") == false` 不构成安全论证；测试进程可写文件、联网、派生进程，与 Interactive 下其他 LocalMutation 工具不一致 |
| sandbox / Job Object | 必须经过 sandbox/Job Object | Job Object ✅（kill-on-close 挂载 + TaskKill 树杀 + 30min timeout）；AppContainer/Docker sandbox ❌（测试进程运行于宿主会话上下文） | 部分达标：进程生命周期控制完整，无隔离沙箱 |
| timeout | 必须设置 timeout | ✅ 30min 默认（`RUN_TESTS_TIMEOUT`），`TestRunner.timeout` 可注入 | 达标 |
| 输出上限 | 模型只接收脱敏且截断的 tail + artifact identity | ✅ 1MB 采集上限（`RUN_TESTS_OUTPUT_CAP`）+ 32KB 上下文 tail（`RUN_TESTS_CONTEXT_CAP`）+ 完整输出落盘 `.gsa/run_tests_output.txt` | 达标（上限与 artifact 身份） |
| 环境脱敏 | secret、host path 不得通过失败输出泄露 | `tokio::process::Command` **继承宿主全部 env**（无 env_clear / allowlist），测试进程可见 `ORZ_TEST_API_KEY` 等；输出文本无脱敏扫描 | **差距**：env 注入与输出内容均未脱敏 |
| workspace delta / journal | 必须记录 workspace delta 与 journal | journal ✅（ToolStarted/ToolCompleted + fixed_command 摘要 + exit_code，2026-08-07 F-02 修复）；workspace delta（测试运行前后工作区变化记录）❌ | 部分达标：journal 完整，无 delta 记录 |

## 3. 差距清单

| ID | 差距 | 风险 | 建议处理 |
|---|---|---|---|
| RT-001 | 跳过 execution permission | 测试进程与宿主同权限上下文；模型可借 run_tests 间接执行任意代码（fixed command 虽 host-owned，但测试文件内容模型不可见、无法借道；主要风险是权限语义不一致） | 下一轮：run_tests 至少登记 `LocalMutation`-级风险并经过 permission 语义（Interactive 下弹窗或按策略拒/放，harness 用 Benchmark/ReadOnly 策略覆盖）；不得以 `is_file_edit` 反推安全 |
| RT-002 | env 全继承 + 无输出脱敏 | 测试进程可读取宿主凭据/路径；失败输出可能打印 secret、host path、hidden test 内容（SWE-bench 已记录窄泄漏先例：django 框架文件） | 下一轮：`Command::env_clear` + 显式 allowlist（PATH/HOME 等必需项）；输出做路径/凭据样式扫描替换后再入上下文；完整输出保持 artifact 原样 |
| RT-003 | 无 workspace delta 记录 | 测试运行产生的文件变化（测试自己写文件、缓存等）无审计痕迹 | 下一轮：运行前后 snapshot 对比，写入 journal 或 blackboard tool_actions |

## 4. 已验证的正面事实（不需改动）

- 固定命令模型不可改写：`TestRunner.command` host-owned，模型无 argv 注入点
- hidden test source 不可直接读：测试文件位于任务目录外隐藏树，模型 cwd-scoped 不可见（D-9 Aider 模式）
- 进程生命周期完整：30min timeout + Job Object kill-on-close + TaskKill 树杀（52min forth 挂死闭环）
- 输出有界：1MB 采集 + 32KB 上下文 tail + artifact 落盘，病态输出不爆上下文
- 工具声明有门：`test_runner()` 为 `None` 时 `run_tests` 不声明、调用返回 NotFound（controller.rs:946-953）
- 调用计 tool-call round：ADR §3.8.4 满足（走工具批循环，计入 120 轮预算）

## 5. 处理决定

- **本次不修改实现**：RT-001~003 的改动涉及评测关键路径（SWE-bench/polyglot 依赖 run_tests
  无门禁快速运行），且 RT-002 的 env allowlist 需要与 harness 注入项（PYTHONPATH、venv 激活、
  sitecustomize shim）联调，属于实现切片而非复核切片。
- 排期建议：随 GAP-SUBAGENT-RUNTIME 之后的 Phase C 收尾切片一起处理，或独立小切片
  （RT-002 env_clear 相对独立，可先做）。
- 复核依据代码：`orz-host/src/lib.rs` `run_tests`（env 继承、Job Object、树杀、落盘）、
  `orz-loop/src/controller.rs` run_host_tool `run_tests` 分支（permission 跳过、journal）、
  `orz-loop/src/host.rs` `TestRunner`/`RUN_TESTS_*` 常量。
