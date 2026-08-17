# Benchmark 完全体执行面设计（终端/网络两轴放开）

- 状态：`implementing`（2026-08-18 用户裁决；2026-08-18 实施完成、验证暂缓
  ——用户指示「完成后暂时不进行测试」；待验证闭环后转 `implemented`）
- 关联：[`ADR-0010 §14.24`](../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)（v1.24）；
  [`CLASSICAL-EXEC-ASSISTANT §13`](CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md)；
  [`PLAN_FIRST_BLACKBOARD §4`](PLAN_FIRST_BLACKBOARD_DESIGN_2026-08-15.md)
- 实施路由：实施前登记 BACKLOG / TODO（本设计轮不动未闭合计数）

## 1. 背景与问题

TB2 冒烟（`D:\tb-eval\jobs\2026-08-17__23-29-44`，make-doom-for-mips）reward 0 的
根因是评测配置 shell-less：`--allow-write` 触发 Benchmark 策略，而 Benchmark 对
shell 与网络 fail-closed；make-doom 需 LLVM 交叉编译 + `node vm.js`，无 shell 不可解。

进一步排查发现 shell 在 console 默认面下被**三层**同时关闭，缺一不可：

1. **权限层**：`orz-host/src/permission.rs` Benchmark 分支按名级排除 shell 工具
   （`run_terminal_cmd`/`bash`/`cmd`/`powershell`/`pwsh`），NetworkCall 一律拒绝；
2. **探针层**：`orz-loop/src/tool_probe.rs` `policy_allows_exec` 仅 Interactive 通过，
   Benchmark 下 `run_terminal_cmd` 探针判定「终端链路不完整」，动作栏按钮不渲染；
3. **控制台动作注册表**（关键缺口）：`console.rs default_service_registry` 没有
   `run_terminal_cmd` 动作，`is_console_surface_tool` 也不含该工具——console 默认面
   模型看不到任何执行工具，执行只能经动作栏订单下发。即使权限与探针放开，
   模型依然无法下单执行 shell。

## 2. 目标与不变量

- **模型面纪律不变**：shell 不开放为模型直接工具；模型只下单
  （`blackboard_action_write` + 动作名/参数），助理层执行——与
  `workspace.run_tests` 同构。
- **助理层获得执行资格**：permission gate、探针投影、console 注册表三层使能
  `run_terminal_cmd`；网络按任务合规镜像。
- **门禁不变**：ACAF fail-closed 票据（command_exec_v1 / network_v1）、订单门
  （registry/contract/target/ACAF/policy/mode）、step_id 绑定、预算/墙钟/停滞
  守卫、事件审计链全部保留。
- **TB 合规**：任务容器本就是 agent 全 shell 评测沙箱；harbor 在环境层 enforce
  任务网络；orz 内放开是对齐任务声明，不构成边界扩大。
- 默认 fail-closed：`Benchmark{allow_shell:false, allow_network:false}` 与现状
  完全一致；Interactive/ReadOnly 语义不动。

## 3. 权限策略：Benchmark 两轴参数化

`PermissionPolicy::Benchmark` 参数化为 `Benchmark { allow_shell: bool,
allow_network: bool }`（默认 false/false，保持旧语义与旧测试）。

Benchmark 分支决策表（`permission.rs`）：

| RiskClass | 条件 | 结果 |
|---|---|---|
| ReadOnly | 恒 | 走 manager（auto-allow + access_in_scope 不变） |
| LocalMutation | MCP（含 `__`） | Deny（不变） |
| LocalMutation | shell 工具且 !allow_shell | Deny（旧行为） |
| LocalMutation | 其余（search_replace/run_tests…） | AllowOnce（不变） |
| SandboxEscape | bash/sh/cmd/pwsh 且 allow_shell | AllowOnce（新增） |
| SandboxEscape | !allow_shell | Deny（旧行为） |
| NetworkCall | allow_network | AllowOnce（web_fetch/web_search 直调面；新增） |
| NetworkCall | !allow_network | Deny（旧行为） |

MCP 恒 deny、读限定（工作区外拒绝）不变；Interactive/ReadOnly 分支不动。

## 4. 循环侧投影：ToolPolicy::BenchmarkFull

- `orz-loop/src/host.rs` `ToolPolicy` 新增变体 `BenchmarkFull`（语义=读+写+终端；
  网络不参与名级投影——web 工具本就不在工作工具/操作台面，仅权限层把关）。
- `orz-host/src/lib.rs` `tool_policy()`：`Benchmark{allow_shell:true,..} →
  ToolPolicy::BenchmarkFull`；其余 Benchmark → `ToolPolicy::Benchmark`。
- `console.rs` `ActionBundle::allows()` 加臂 `BenchmarkFull => self.benchmark`
  （复用 benchmark 档；动作栏仍由探针收敛，旧 Benchmark 下终端探针不完整→按钮
  不出现，无需新档位字段）。
- `tool.rs` `policy_refuses` 不变（仅保留 MCP 名级防御）。

## 5. 探针：policy_allows_exec

- `tool_probe.rs`：`policy_allows_exec(policy) = Interactive | BenchmarkFull`。
- `probe_terminal` 逻辑不变（policy && terminal_available；CLI host 恒接线
  LocalTerminalBackend → terminal_available=true）。

## 6. 控制台动作注册表：workspace.run_terminal

新增 ActionSpec（`console.rs default_service_registry`）：

- 名称：`workspace.run_terminal`（沿用 `workspace.*` 命名）；`target_tool:
  run_terminal_cmd`；`kind: ActionKind::Host`。
- bundle：`ActionBundle::READ_WRITE`（standard + benchmark 档；ReadOnly 不加载）。
  交互式 console 动作栏同步出现该按钮（经订单 + Interactive 权限询问），属
  「完全体/正常工作」方向；如实施时只想限跑分，可改单档常量（一行差异，登记
  实施差异）。
- input_schema 镜像 BashToolInput：
  - `command`（string，必填）
  - `description`（string，必填——一句话说明用途）
  - `timeout`（integer，1–300000 ms，可选；默认 120000）
  - `is_background`（boolean，可选，默认 false）
  - `additionalProperties: false`
  - 不暴露 env/cwd：环境与工作目录由 host 决定（ACAF command_exec 目标摘要
    基于 host 侧 cwd/env，模型不可注入）。
- response_schema：既有 text_output 信封（`{"output": string}`）。
- 投影自动生效：`registrations_for` 对 Host 动作要求目标工具探针 Complete；
  BenchmarkFull + 探针完整 → 动作栏出现；旧 Benchmark/ReadOnly → 不出现。

## 7. CLI 旗标与 env

- 新增 `--allow-shell`、`--allow-network`（headless benchmark 专用）→ 映射
  `ORZ_ALLOW_SHELL` / `ORZ_ALLOW_NETWORK`（沿用 `--allow-write`→`ORZ_ALLOW_WRITE`
  先例）。
- 校验：未带 `--allow-write` 时 exit 2（fail-closed，防静默无效）。
- `build_cli_host`：ORZ_ALLOW_WRITE 时构造
  `Benchmark { allow_shell: ORZ_ALLOW_SHELL.is_ok(), allow_network: ORZ_ALLOW_NETWORK.is_ok() }`。
- `--help` 文本同步。

## 8. 适配器与任务合规

- `tb_agents/orz.py` `run()`：`allow_shell = True`（TB 本质是 shell 评测）；
  `allow_network = environment.network_policy.network_mode == NetworkMode.PUBLIC`
  （**实施注记（2026-08-18）**：取 `environment.network_policy`（trial 按
  agent 阶段策略 set 的有效策略，base.py `set_network_policy` / trial.py
  接线）而非 `task_env_config.network_mode` 基线——对 89 题全 PUBLIC 结果
  一致，且 NO_NETWORK/ALLOWLIST 任务按容器实际生效策略 fail-closed，严格不
  比设计更宽；`allow_internet` 已被 harbor 清除，活字段为 network_mode）。
- env 增 `ORZ_ALLOW_SHELL=1`、`ORZ_ALLOW_NETWORK=1`（按任务计算）；运行脚本
  belt-and-braces 同传 `--allow-shell`/`--allow-network`。
- 数据集现状：89 题（terminal-bench-2-1@sha256:7d7bdc…）全部 PUBLIC → 实际每次
  全开；机制仍按任务合规（未来 NO_NETWORK 任务自动关闭 orz 内网络面，与 harbor
  环境层一致）。

## 9. 安全面与边界

- 「放开」= 策略允许面：permission gate 从 deny 改 auto-allow，审计面不减——
  PermissionRequested/PermissionDecision、ACAF issued/consumed、ToolStarted/
  ToolCompleted、console_order_written/rejected 全部保留。
- ACAF fail-closed 票据是最终授权兜底：命令 argv/cwd/env 与 URL 规范化目标
  摘要仍逐调用签发+验票；缺票/错票拒绝且不执行。
- 容器层：harbor 按任务 network_mode 建网；NO_NETWORK 任务容器无网，orz 内
  即使配置放开也无法外联（纵深一致）。
- 不新增平行执行层：终端仍走 run_host_tool 既有门链与事件链。

## 10. 验证计划（实施阶段执行）

> 2026-08-18 用户指示：本实施轮**暂不进行测试**——以下验证项在后续窗口
> 执行（TODO P0-F 验证①–⑤未勾选）。

1. orz 子模块 cargo 全量测试（新增/更新用例先行）：权限决策表、探针映射、
   console 注册表投影、订单→run_host_tool→ACAF 票据路径；clippy 无新增告警。
2. Linux musl 重建（ORZ-BUILD-MOUNT-001 契约，输出 `D:/tb-eval/orz-linux`）。
3. 单题复验 make-doom-for-mips（同参数）：reward > 0、journal 出现
   `workspace.run_terminal` 订单→run_host_tool→ACAF `command_exec` issued/
   consumed、机制门不回归（无 400、step_id 绑定、无异常 policy_denied）。
4. 挑 2–3 题交叉验证（build/run 类如 compile-compcert、网络类如
   hf-model-inference），确认无框架缺口后进分批。
5. `run_official_2.1.sh` 分批（89 题 5 批）。

## 11. 登记

- ADR-0010 §14.24（v1.24）；CLASSICAL-EXEC-ASSISTANT §13；PLAN_FIRST_BLACKBOARD
  §4；CLI_PROJECT_INDEX（FUS-BENCHMARK-FULL-EXEC，`current-design`）。
