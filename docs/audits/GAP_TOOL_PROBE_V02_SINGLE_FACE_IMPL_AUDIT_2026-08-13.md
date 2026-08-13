# FUS-TOOL-PROBE v0.2 单一探针面实施审计（P0-A-2，2026-08-13）

> 审计范围：P0-A-2——把 v0.1 三面语义（面 A 恒声明 / 面 B 探针过滤 / 面 C 固定列表）
> 收敛为 ADR-0010 §3.5 v1.8 的 v0.2 单一探针面；实施、合约与测试证据边界。
> 裁决来源：2026-08-13 用户 A+C→B 定档；设计文档
> `docs/TOOL_AVAILABILITY_PROBE_DESIGN_2026-08-13.md` §2.0/§3/§4/§5；
> ADR-0010 v1.8 §3.5 条 1/2 与 §14.8。

## 1. 实施内容

### 1.1 探针模块（orz-loop `tool_probe.rs`）

- 面 A/B/C 常量与判定函数删除，改为单一 `WORK_TOOLS`（23 个主 Agent 工作工具，
  规范顺序与 Python verifier 的 `_WORK_TOOLS` 镜像）。
- 每个工具按设计 §2.0 表实现机械链路判定：读链（`read_file`/`list_dir`/`grep`/
  `search_tool`）、写链（`search_replace`，策略 + metadata-grade 可写）、runner 链
  （`run_tests`）、交互链（`ask_user_question`）、存储链（`blackboard_read` /
  `compaction_whitelist_add`）、goal 写链（`todo_write`/`update_goal`，工作区可写 +
  goal 上下文）、plan 模式链（`enter_plan_mode`/`exit_plan_mode`，交互用户）、检索激活链
  （`retrieval_disposition`）、终端链（`run_terminal_cmd`，Interactive 策略 + 宿主终端）、
  lsp 配置链、memory opt-in 链、图像后端链、视频后端链、MCP 注册链（`use_tool`）。
- 全部判定为两态中性（`Complete` / `Incomplete(中性 reason)`），未知工具 fail-closed；
  `MinimalProbeMap` 覆盖 23 个工具，`mark_incomplete` 回写限工作工具（调用即探针）。

### 1.2 信号接线

- `LoopHost` 新增六项 fail-closed 能力访问器：`terminal_available` / `lsp_configured` /
  `memory_enabled` / `image_backend_configured` / `video_backend_configured` /
  `mcp_registry_available`。
- orz-host：终端恒 true（`build_toolset` 恒建 `LocalTerminalBackend`）；lsp/memory/
  图像/视频/MCP 当前均 false（`build_toolset` 固定禁用可选后端）——探针据此移除这些
  工具的可见性（符合设计：调用时本就无法完成）。
- controller 新增 `goal_context_present()`（goal 绑定存在）与 `has_live_activation()`
  （存在携带未决 pending assessment 的激活；2026-08-13 审查复核收紧）探针信号；
  `ProbeContext` 在 run-start 与每模型轮前构造时携带全部信号。

### 1.3 投影与事件

- `project_main_agent_tool_defs` 改为 `探针完整集 ∩ 会话声明集 + 非工作工具`（仅名称）；
  `run_tests` 保持 host-owned 条件声明（探针完整才声明）。
- `tool_availability_check` payload 形状不变，`complete`/`incomplete` 覆盖全部 23 个
  工作工具；翻转事件与 run-start 首翻语义不变；调用即探针回写仍限主/grill 车道。

### 1.4 合约与 fixtures

- Python verifier `_FACE_B_TOOLS` → `_WORK_TOOLS`（23 个），交叉校验错误信息更新。
- `runtime/tool-availability-check-event-payload-v0.2.schema.json` 描述更新（形状不变）。
- fixture 生成器 `PAYLOAD_GOOD_V02` / `PAYLOAD_BAD_V02` 探针 payload 扩为 23 工具分区，
  README 文案同步；payload/envelope fixtures 重生成。
- 12 个含探针事件的 v0.2 journal fixtures 重建为统一 23 工具分区（9 complete +
  14 incomplete，中性 reason），并按 Rust parity 哈希链重算 `payload_sha256` /
  `previous_event_sha256` / `event_sha256`。

## 2. 行为变化

- 原先面 A/C 固定声明的工具现在由探针决定可见性：无检索激活时 `retrieval_disposition`
  不再可见；Benchmark/ReadOnly 下 `run_terminal_cmd` 不再可见；后端未配置时
  lsp/memory/图像/视频/use_tool 不再可见；headless 下 plan 模式工具不再可见。
- 检索车道（web_search/web_fetch/browser_read/pdf_read/project_doc_index）与主车道
  非工作工具（bash 等）保持各自既有声明规则，不参与探针面。

## 3. 验证证据

- orz-loop：236 通过 / 0 失败 / 3 ignored（探针单测、投影、翻转、车道隔离、跨 run 重置）。
- orz-host：199 通过 / 0 失败 / 4 ignored；orz-tui：178 通过 / 0 失败。
- Python：verifier 与相关 assurance 测试 395 通过（含 12 个 v0.2 journal 全链校验）。
- 仓库门禁 `check_repository.py`：valid、0 错误；`cargo fmt --all` 已执行；
  `git diff --check` 干净。

## 4. 边界与后续

- orz-host 可选后端（lsp/memory/图像/视频/MCP）未接线：对应探针恒为不完整，工具不可见。
  接线任一时须翻转 orz-host 能力访问器并补翻转测试（登记于 BACKLOG 边界）。
- 本次 journals 为已提交 fixtures 重建（非新捕获运行）；下一次真实运行捕获将自然携带
  23 工具分区。
- 检索车道激活生命周期会在主车道产生合法的 `retrieval_disposition` 翻转事件
  （incomplete→complete）——已登记并锁定测试语义，非污染。
- 2026-08-13 审查复核：`retrieval_disposition` 探针以未决 pending assessment 为完整
  条件（Active 无 pending / continue 已决后均不完整）；plan 模式探针以交互用户代理，
  未来 headless 计划模式需独立能力信号。

## 5. 入口

- 实现：`orz/crates/orz-loop/src/tool_probe.rs`、`agent_loop.rs`、`controller.rs`、
  `host.rs`；`orz/crates/orz-host/src/lib.rs`。
- 合约：`assurance/run_event_journal_validation.py`；
  `runtime/tool-availability-check-event-payload-v0.2.schema.json`；
  `runtime/fixtures/run-event-v0.2/`。
- 设计/裁决：`docs/TOOL_AVAILABILITY_PROBE_DESIGN_2026-08-13.md`；
  `adr/ADR-0010-fusion-runtime-and-agent-architecture.md` §14.8。
