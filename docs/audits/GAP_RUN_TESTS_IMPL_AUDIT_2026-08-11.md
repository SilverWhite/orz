# GAP-RUN-TESTS 实施审计（RT-001/002/003 闭合，2026-08-11）

> 前置：`V11_IMPL_005_RUN_TESTS_SECURITY_REVIEW_2026-08-09.md`（复核记录，三差距排入实现切片）。
> 依据：ADR-0010 §3.8「受控 run_tests / hidden-test 反馈环」条目 2（六控制维度枚举）+
> `V11_IMPL_005` 复核范围（六维度：execution permission / sandbox-Job Object / timeout / 输出上限与脱敏 /
> workspace delta-journal / hidden-output 泄露）。
> 范围：本切片只闭合 RT-001~003；已达标维度（Job Object/timeout/输出上限/计轮）机制本体零改动
> （声明过滤有增强，见 RT-001）。

## 1. 差距与修复

| ID | 差距（前序审计） | 修复 | 位置 |
|---|---|---|---|
| RT-001 | `run_tests` 跳过 execution permission——`run_host_tool` 中该分支在 permission gate 之前直接执行 | **run_tests 分支整体移到通用 permission gate 之后**：与任何 LocalMutation 工具同一门禁（PermissionRequested → `request_permission` → PermissionDecision → Deny/Defer 拒绝路径含 gate_log + 拒绝 tool message + 不产生 ToolStarted/ToolCompleted）；**声明处补 `policy_refuses` 过滤**（ReadOnly 只声明 read-class——此前 run_tests 绕过声明过滤，Grill/ReadOnly 下仍声明） | `orz-loop/src/controller.rs` |
| RT-002 | `tokio::process::Command` 继承宿主全部 env（测试进程可见 `ORZ_TEST_API_KEY` 等）；输出文本无脱敏扫描 | **env_clear + 最小 allowlist**：`Command::env_clear()` + 固定平台 allowlist（Windows: PATH/SystemRoot/PATHEXT/COMSPEC/TEMP/TMP/USERPROFILE；Unix: PATH/HOME/TMPDIR/LANG）+ 新 `TestRunner::env` 显式注入（harness 联调项：PYTHONPATH、venv、sitecustomize shim；`ORZ_TEST_RUNNER_ENV` JSON 注入点）；**上下文边界脱敏**：`compose_test_output_message` 注入前用 orz-secrets `redact_secrets` + `redact_user_paths`，**先脱敏后截断**（截断边界不能切开 secret 的半截），artifact 落盘保持原样 | `orz-host/src/lib.rs` / `orz-loop/src/host.rs` / `orz-bin/src/main.rs` / `orz-loop/src/controller.rs` |
| RT-003 | 测试运行产生的文件变化无审计痕迹 | **workspace delta**：运行前后 `workspace_delta_walk` 元数据遍历（零内容读、排除 .git/.gsa/node_modules/.venv/venv/target/**__pycache__/.pytest_cache/.mypy_cache/.ruff_cache/.tox**（Python 测试缓存面，审查 D2-1/P1 补入）、**跳过 symlink 与 Windows junction/reparse point**（审查 D2-2——junction 无环保护）+ `workspace_delta_diff`（added/modified/deleted，按 path 排序，上限 200 条 + `truncated` 标志）；写入 `TestRunResult` → **ToolCompleted 事件 payload**（`workspace_delta` + `workspace_delta_truncated`）；超时路径同样记录（被杀前可能已写文件） | `orz-host/src/lib.rs` / `orz-loop/src/host.rs` / `orz-loop/src/controller.rs` |

## 2. Schema 先行（ADR-0010 §5.3 纪律）

`runtime/tool-completed-event-payload-v0.1.schema.json` 先扩展：

- `workspace_delta`：`array of {path: string, kind: enum[added,modified,deleted]}`（additionalProperties false 的 item 结构）；
- `workspace_delta_truncated`：`boolean`（optional）。

信封 schema（run-event-v0.2）零改动；`payload` 为自由对象，无 per-event 约束。旧 journals 兼容（新字段 optional）。check_repository 校验 235 个 schema 全部合法。

## 3. 行为语义（按策略）

| 策略 | run_tests 声明 | permission |
|---|---|---|
| Interactive | ✅ 声明 | **弹窗确认**（LocalMutation 级；用户逐项审批语义） |
| Benchmark（harness，ORZ_ALLOW_WRITE） | ✅ 声明（LocalMutation 非 shell） | 自动 AllowOnce（host permission.rs 既有分支）——**harness 无门禁快速运行语义保持** |
| ReadOnly | **不声明**（`policy_refuses` 过滤） | —（不可达） |
| Grill | **不声明**（`grill.is_none()` 守卫，审查 P3-5 补入——与 compaction_whitelist_add/retrieval_disposition 先例一致；此前仅 ReadOnly 过滤，Benchmark+grill 组合仍声明） | —（不可达） |
| 检索 lane | — | 不达此点：写域 deny-only 门禁先拒（`retrieval_role_execution_denied`） |

## 4. 验证

- orz-host **198 passed / 4 ignored**（基线 195/4 → +3：`run_tests_env_is_isolated_from_host`、`run_tests_records_workspace_delta`、`workspace_delta_diff_caps_and_sorts`；timeout 树杀等既有测试零改动通过）
- orz-loop **173 passed / 3 ignored**（基线 170/3 → +3：`d9_run_tests_permission_gate_denies_under_interactive`（Deny 拒绝 + PermissionRequested/Decision 事件 + 无 ToolStarted/ToolCompleted + gate_log 记录）、`d9_run_tests_not_declared_under_readonly_policy`、`d9_run_tests_tool_completed_carries_workspace_delta`（含 truncated 标志 round-trip）；既有 D-9/DC 测试 host 改 AllowOnce 后全过）
- orz-bin 6 + 2 + 1 全绿、**capture 13/13 全过**（无 capture 场景含 run_tests——事件序列变化不影响既有 fixtures；顺带修复 orz-bin 测试 stub `StubBrowserSession` 缺 `download_or_read` 的 pre-existing 编译错，GAP-PDF-EVIDENCE 遗留）
- clippy `-D warnings` 三 crate 全绿（新代码 2 处 collapsible_if/match_result_ok 已修）
- pytest（assurance 目录收集，C2-3 惯例注明范围：**1607 passed / 13 skipped**；根目录全量 1836+13 含 assurance 之外的目录收集）+ check_repository **valid**（235 schemas，含扩展后的 tool-completed payload schema）
- `git diff --check` 干净

## 5. 登记边界（不修/留后续）

1. **脱敏范围**：只覆盖进入上下文的 tail（先脱敏后截断）；artifact 原样保留（有权限可读全量，Benchmark 下 read_file 自动放行可读原始 artifact——env 源已闭合，残余为宿主路径文本如 USERPROFILE/TEMP 值）——按审计建议原文。
2. **allowlist 固定集**：平台最小集 + `TestRunner::env` 显式注入；宿主任意 env 不继承。未来需新增平台变量时改 `TEST_ENV_ALLOWLIST` 常量并登记。Windows 侧未含 `NUMBER_OF_PROCESSORS`/`SYSTEMDRIVE` 等（有 `ORZ_TEST_RUNNER_ENV` 逃生阀）；`USERPROFILE` 指向宿主真实 profile——测试进程可借 HOME 在工作区外写缓存（Job Object-only 沙箱下能力未扩大，但 env 授权了工作区外落盘面）。
3. **delta 排除目录**：`.git/.gsa/node_modules/.venv/venv/target/__pycache__/.pytest_cache/.mypy_cache/.ruff_cache/.tox` 按名（大小写敏感）排除——Python 测试缓存面（每次运行必重写）不入 trace；`build`/`dist` 等构建产物目录未排除（200 条上限兜底）。100% 全覆盖需要内容哈希（成本高），当前 stat+mtime diff 是审计痕迹级（同 project_doc_index 的登记 trade-off：同 size 同 mtime 修改不可察；FAT/exFAT 2 秒 mtime 粒度下同尺寸快速重写也不可察）。
4. **delta 上限 200 条**：超限截断 + `workspace_delta_truncated` 标志，总量不可从事件恢复（只保证"有变化"且"变化被截断"两种事实）。
5. **Interactive 弹窗确认**是新增交互（此前零门禁）；用户可借 Deny 拒绝单次运行。Benchmark 语义不变（自动放行）。**行为变化登记**：Interactive + headless（`-p`/plan、dead gateway）下 run_tests 由"零门禁直跑"变为 fail-closed Deny（与 IP6 全工具语义统一，属 RT-001 设计意图）——评测路径均为 Benchmark 不受影响。**UX 局限**：Interactive 弹窗只显示泛化审批（run_tests 空参数），批准者看不到固定命令——fixed_command 进审批面留后续。
6. `ORZ_TEST_RUNNER_ENV` 解析失败（非 JSON 对象）→ `tracing::warn` + 空 env（env_clear 仍生效，fail-safe 方向正确；审查 P2 补告警，此前完全静默）。
7. **子代理调用 run_tests**：`retrieval_role_execution_denied` 前置拒绝保持不变（本切片零改动）。**lane-aware 声明**（pre-existing F6 形状）：检索子代理若宿主带 test runner 仍会看到 run_tests 被声明、随后被写域门拒绝，浪费一轮——声明过滤未来需 lane-aware，本切片不修。
8. orz-secrets 成为 orz-loop 新直接依赖（workspace codegen crate，零第三方新增）。
9. **delta 盲区**：symlink 与 junction 条目整体跳过（重定向不记录；文件被替换为 symlink 时报 Deleted 而非 Modified）；空目录创建不记录（walk 只记文件）；非 UTF-8 文件名经 `to_string_lossy` 可能碰撞为同一 key。均为登记，不修。
10. **grill 声明守卫**（审查 P3-5）：`grill.is_none()` 加入 run_tests 声明条件——grill 只读承诺与 compaction_whitelist_add/retrieval_disposition 先例一致。
11. **artifact 读取通道（pre-existing 偏差，2026-08-11 修复）**：F-09/ADR §3.8.3 语义是"完整输出可按 permission 读取"，但 `permission.rs access_in_scope` 显式排除 `.gsa` 树——模型 `read_file` 读 `{cwd}/.gsa/run_tests_output.txt` 实际被 permission 拒绝（e2e 实测发现）。**已修复**：access_in_scope 对 `.gsa` 树排除加**精确单文件白名单**——`{cwd}/.gsa/run_tests_output.txt`（规范化后逐字节相等，无通配）允许 Read/Grep；其余 `.gsa` 内容（journals/keystore/snapshots/session 状态）保持 agent-invisible。单测：白名单路径 AllowOnce + `.bak` 邻居路径仍 Deny（`read_scope_enforced_before_manager` 扩展）；e2e 实测模型 read_file 读到完整 artifact 并正确报告首行。安全面：白名单只放行一个固定文件（内容本就可经上下文 tail 看到大部分），无通配放大。**SWE-bench harness 适配验证**（2026-08-11，run_swebench.py `run_orz`）：`env["ORZ_TEST_RUNNER_ENV"] = json.dumps(env_extra)`——PYTHONPATH source-import 兜底与 django ORZ_SWE_APPS/ORZ_SWE_WORKTREE conftest 注册通道经显式注入恢复；e2e 冒烟实测注入项到达测试进程（`PROBE_PYTHONPATH`/`PROBE_SWE_APPS` 可见）且宿主 secret 不可见（env_clear 生效），双仓验证通过。

## 6. 三面审查与修复批回填（2026-08-11，用户发起"全面检查"）

设计合理性 / 实现合理性 / 符合性三独立代理审查：**无 D1/P0/C1**。修复批：

- **D2-1/P1/C2-1（互证）**：`DELTA_EXCLUDED_DIRS` 补 `__pycache__/.pytest_cache/.mypy_cache/.ruff_cache/.tox`——Python 测试缓存面每次运行必重写，此前会把 200 条上限耗尽使 delta 沦为缓存噪声（主部署场景 = hidden pytest 套件）；同步修审计 §5 #3 示例措辞（原写".pytest_cache 等不记录"与代码不符）。
- **D2-2**：walk 跳过 Windows junction/reparse point（`symlink_metadata` + `FILE_ATTRIBUTE_REPARSE_POINT` 位检测——junction 对 `is_symlink()` 报 false 且 `metadata()` 会跟随藏起该位）——junction 指向祖先可致 walk 无限遍历，且该 walk 在测试超时之外。
- **P2/D3-2**：`ORZ_TEST_RUNNER_ENV` 解析失败加 `tracing::warn`（此前静默——harness 拼错 JSON 时 PYTHONPATH 丢失表现为不可见的"module not found"）。
- **P3-5**：run_tests 声明加 `grill.is_none()` 守卫（Benchmark+grill 组合此前仍声明并执行；与 whitelist/disposition 先例一致）。
- **P3-6**：deny 测试补 gate_log 断言；delta payload 测试补 truncated=true round-trip（第二个 run_turn journal 目录笔误 j2→dir 顺带修正）。
- **C3-1~4**：审计措辞修订（§3.8.2→条目 2+前序六维度、pytest 数字补写并注明收集范围（assurance 目录 1607/13，C2-3 惯例）、"工具声明门零改动"→"机制本体零改动（声明过滤增强见 RT-001）"、错别字或z-secrets→orz-secrets）。

复验（修复后）：orz-host **198/4**、orz-loop **173/3**、orz-bin 6+2+1、clippy `-D warnings` 三 crate 全绿、`git diff --check` 干净。

## 7. 变更文件

- orz 仓（6 文件 + Cargo.lock）：`orz-loop/src/host.rs`（TestRunner.env / TestRunResult delta / WorkspaceDeltaEntry）、`orz-host/src/lib.rs`（run_tests env+delta / junction 保护 / 测试）、`orz-loop/src/controller.rs`（permission 接入 / 声明过滤+grill 守卫 / 脱敏 / ToolCompleted payload / 测试）、`orz-bin/src/main.rs`（ORZ_TEST_RUNNER_ENV / warn / stub 修复）、`orz-loop/Cargo.toml`（orz-secrets）
- 主仓（3 文件）：本审计 + `runtime/tool-completed-event-payload-v0.1.schema.json`、`CLI_PROJECT_INDEX.md`（GAP-RUN-TESTS 状态更新）
