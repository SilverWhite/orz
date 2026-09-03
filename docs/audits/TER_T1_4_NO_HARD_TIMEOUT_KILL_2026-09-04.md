# TER T1.4 去硬杀语义实施审计（2026-09-04）

> 范围：TODO2 M1 步 T1.4——run_terminal_cmd timeout 分层（普通/程序/
> 模型上限）不再作为杀进程点，改为 auto-bg deadline 引用；原 timed_out
> 杀进程测试改为「auto-bg（超时点后台化）或 10h 绝对兜底」断言；验收 =
> 默认/分层路径无任何「满 timeout 杀活跃命令」代码路径（评测墙钟除外，
> 归 M2 T2.1）。
> 上级：[TODO2.md](../../TODO2.md) / 设计稿
> [TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md](../TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md)
> §3.1（行为状态机 + timeout 行）/ §10 S1-2（去硬杀语义）；前置步骤
> T1.1（auto-background 常驻默认 true）/ T1.2（180s 预算单源）/
> T1.3（模型面封闭）审计同目录。

## 1. 目标与验收

- 前台解析超时（模型 `timeout` / 会话 `timeout_secs` 缺省，经
  `max_timeout_secs` 封顶）与 `foreground_block_budget_ms`（180s）先到者
  即 **auto-bg deadline**：命中即自动后台化并返回一次中间状态，不杀。
  取消 T1.2 时代「解析超时 ≤ 预算 → 关闭逐调用 auto-bg、超时点
  kill-on-timeout」的门。
- auto-bg（含用户主动后台化）后，原解析超时**退役**：后台任务只受
  10h 绝对兜底（`BACKGROUND_MAX_RUNTIME`）约束；无活跃场景由 T1.5
  idle+CPU 兜底接管（本步先留注释锚点）。
- 显式 `auto_background_on_timeout=false`（逃生阀 / 不可后台化工具集）
  保留旧 bounded kill-on-timeout 语义（T1.1「关闭态仍可配」不回归）。
- 模型面文案删除「Timeout enforcement … kills」与「300s for ordinary /
  600s for program」静态分层杀宣示，改为生效 deadline 同源渲染。
- 验收点：bash/终端/接线测试全绿；默认与分层路径 grep 无
  kill-on-timeout；timed_out 杀进程用例改为 auto-bg/存活断言。

## 2. 代码改动

### 2.1 orz-tools 终端 actor（`computer/local/terminal.rs`）

- `poll_all_processes` 0b 清扫：注释改 TER T1.4 口径——只对「10h 绝对
  兜底」或「显式后台任务的正向模型超时 kill-backstop」生效；分层 timeout
  不再是杀活跃命令的点（无活跃场景归 T1.5 idle+CPU）。
- `poll_process` 前台预算/超时 tick：
  - 超时分支加 `!bg_status.is_backgrounded()` 守卫——后台化任务的原解析
    超时已退役，由 0b（10h）与 T1.5 接管；
  - `Foreground { auto_bg_on_timeout: true }` 命中超时 → auto-bg
    （解析超时 ≤ 预算时超时点即 auto-bg deadline）；
  - kill-on-timeout 分支只服务显式 `false` 逃生阀（注释与语义同步）。
- `transition_to_background`：删除「ForegroundTimeout 保留原解析超时」
  的分型逻辑，三种后台化原因统一 `process.timeout = BACKGROUND_MAX_RUNTIME`
  （后台化后分层 timeout 退役；10h 绝对兜底保留）。

### 2.2 orz-tools bash 工具层（`implementations/grok_build/bash/mod.rs`）

- `BashParams.timeout_secs` / `max_timeout_secs` /
  `auto_background_on_timeout` 字段文档补 TER T1.4 语义（deadline /
  上限引用；显式 false = 逃生阀）。
- `effective_fg_wait_ms` 文档从「kill-on-timeout 点」改为「auto-bg
  deadline = min(默认超时, 预算)」；移除 `#[allow(dead_code)]`（自本步起
  供模型面文案渲染）。
- `auto_bg_property_mid_run_sentence` / `auto_bg_mid_run_when_clause`：
  中间回报/后台化点由 `effective_fg_wait_ms` 单源渲染（不再硬编码
  budget 秒数），文案为「after Ns … automatically backgrounded instead
  of killed」。
- `exported_input_schema` hide+auto-bg 分支：删除「300s for ordinary /
  600s for program」静态分层文案，改为「default timeout applies;
  never killed for hitting a timeout; auto-backgrounding … returns one
  mid-run status」+ 生效 deadline 句子。
- 描述模板：
  - 内部 auto-bg 模板：默认超时行改为渲染 `default_timeout_ms` 值；
    `auto_background_on_timeout` 为真渲染「No timeout kill」，为假才渲染
    旧「Timeout enforcement … kills」；
  - 可见后台面（逃生阀）模板：杀文案同样以
    `auto_background_on_timeout` 守卫；
  - 后台禁用模板保持 bounded kill-on-timeout 文案（该面无后台路径）。
- `run()` 前台路径：删除逐调用 auto-bg 门（`timeout > budget` 才开
  auto-bg）——会话级开启即携带 auto-bg，终端按
  `min(解析超时, 预算)` 先到者后台化；`timeout ≤ budget` 不再
  kill-on-timeout。

### 2.3 orz-host / orz-workspace（注释口径）

- `orz-host/src/tools.rs`：`run_terminal_cmd_tool_params` /
  `terminal_tier_default_timeout_ms` / `inject_terminal_default_timeout`
  及参数测试文档补 TER T1.4——宿主逐调用注入的 300/600 与 900 上限只作
  auto-bg deadline / 上限引用；机制不变。
- `orz-workspace/src/hub.rs`：接线测试注释更新（模型 timeout < 预算同样
  auto-bg；短预算 + 300s 超时仅为测试提速）。

## 3. 测试同步（去硬杀断言改造）

| 位置 | 用例/夹具 | 处理 |
|---|---|---|
| terminal.rs | `test_backgrounded_task_keeps_original_timeout_deadline` | **改名** `test_auto_backgrounded_task_survives_original_timeout_deadline`：300ms 预算后台化 + 800ms 解析超时的任务越过 800ms 后**仍在运行**（原断言「~800ms 处 signal=timeout 树杀」翻转），随后 kill 清理 |
| terminal.rs | `test_timeout` / `test_timeout_uses_sigterm_then_sigkill` / `test_output_preserved_on_timeout` / `test_foreground_block_budget_skips_non_backgroundable` | 保持（显式 `auto_background_on_timeout=false` 逃生阀路径的 kill-on-timeout 覆盖，T1.1 关闭态不回归） |
| bash/mod.rs | `schema_hides_is_background_when_hide_background_input` | 补 `timeout_secs=600`（经 5min 前台上限收敛 300s → min=300s）；断言改无 300/600 分层文案、含 "never killed for hitting a timeout" 与 "after 300s" |
| bash/mod.rs | `internal_auto_bg_description_renders_without_background_surface` | 同上去分层文案断言；改含 "No timeout kill"、不含 "Timeout enforcement" |
| bash/mod.rs | `default_budget_180s_renders_in_model_facing_copy` | **改名** `default_deadline_renders_in_model_facing_copy`：纯默认 min(120s,180s)=120s → "after 120s"（T1.4：120s<180s 是 auto-bg 点）；主线性 `timeout_secs=600` → "after 180s" |
| bash/mod.rs | `default_enables_auto_bg` | 注释改 T1.4（deadline 即 auto-bg 点） |
| bash/mod.rs | `timeout_and_ampersand_text_branch_on_shell` | `render_flags` 增 auto_bg 参数：auto_bg 开不渲染 SIGTERM/Job Object 杀文案；auto_bg 关（逃生阀）与禁用面按 shell 渲染杀文案 |
| bash/mod.rs | 新 `timeout_within_budget_requests_auto_bg_not_kill` | request 捕获：纯默认 120s ≤ 180s 与显式 30s 模型 timeout 均携带 `auto_background_on_timeout=true`（旧门取消的核心验收） |

## 4. 验证证据

- `cargo test -p orz-tools --lib bash`：**230 passed / 0 failed**
  （229 → 230，新增 T1.4 request 捕获用例；全 bash/registry/reminder/
  output 桶无回归）。
- `cargo test -p orz-tools --lib computer::local::terminal::tests`：
  **40 passed / 0 failed**（5 ignored 为既有 flaky 标注；含
  `test_auto_backgrounded_task_survives_original_timeout_deadline` 与
  全部逃生阀 kill-on-timeout 用例）。
- `cargo test -p orz-host run_terminal_cmd_params_s5_2_layered_timeout_and_mid_run`：
  **1 passed**。
- `cargo test -p orz-workspace auto_background_on_timeout_increments_then_decrements_through_real_wiring`：
  **1 passed**。
- `cargo fmt -p orz-tools -p orz-host -p orz-workspace -- --check`：净
  （exit 0）。
- orz 仓库 `git diff --check`：exit 0。
- 残留扫描：src 树默认/分层路径不再出现 "300s for ordinary" /
  "600s for program/script" 生产文案；"Timeout enforcement … kills" 仅
  保留于显式 false 逃生阀与后台禁用模板分支；kill-on-timeout 终端分支
  仅 `auto_background_on_timeout=false` 可达。
- 说明：本 Windows 沙箱全量 `--lib` 的 grep/glob 外部命令失败属环境性
  （T1.3 已登记）；Linux 全量三件套 + orz-loop/orz-assurance + clippy
  归 M1 门 T1.13。

## 5. 边界声明

- 本步只去「分层 timeout 杀活跃命令」语义：显式模型正向超时
  kill-backstop（可见后台面 `is_background=true` + 正 timeout）与 10h
  绝对兜底保留；显式 `auto_background_on_timeout=false` 逃生阀保留
  bounded 前台 kill-on-timeout（不可后台化工具集需要）。
- `timed_out` 结果形态（signal=timeout）经 T1.4 后只可能来自逃生阀 /
  显式后台 backstop / M2 评测墙钟；auto-bg 路径不再产出。
- 5min idle+CPU 兜底与 `idle_killed` 事件形态归 T1.5；本步在终端注释与
  0b 文案中留锚点，未实现采样器。
- 后台化后任务截止统一 10h：既有 `test_user_backgrounded_task_keeps_max_runtime_deadline`
  语义不变（该用例保持通过）。
- 设计稿 §10 S1-2「timed_out 形态仅保留于 idle-kill 与评测墙钟」的
  idle-kill 侧待 T1.5 落地后复核；评测墙钟侧归 M2 T2.1。
