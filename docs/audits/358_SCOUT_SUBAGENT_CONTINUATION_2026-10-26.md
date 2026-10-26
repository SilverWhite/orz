# 358 接续文档：scout 只读子代理

> 日期：2026-10-26
> 用途：**跨会话接续**。黑板 `plan` 只跨压缩、不跨会话（用户 2026-10-26 纠正），
> 故落此文件；新 run 读本文件 + [`SUBAGENT_SCOUT_DESIGN_2026-10-26.md`](../SUBAGENT_SCOUT_DESIGN_2026-10-26.md) 即可接手。
> 基线：父仓 HEAD `08b448458e62`；orz HEAD `95faff3a`＋未提交改动（358+359 面）。

## §1 一句话状态

**358a／358b／358c 全部完成；审查修复（P0/P1/P2）已落码。** `cargo check -p orz-loop -p orz-host` 通过；`cargo fmt -p orz-loop -p orz-host --check` 净；`cargo test -p orz-loop --lib` **922 passed / 0 failed / 3 ignored**；`cargo test -p orz-host --lib -- --test-threads=1` **453 passed / 0 failed / 6 ignored**（并行全量有 7 个既有环境 flaky：进程树超时／symlink，单跑均过，与本批无关）。**未提交**。

## §2 已落码（文件级，便于核对）

| 文件 | 改动 |
|---|---|
| `relay.rs` | `DispatchTarget::Scout`；`route("spawn_scout")`；`spawn_scout_tool_def()`（loop 侧合成声明，含 task/scope/budget schema） |
| `agents/retrieval.rs` | `SubagentRole::Scout`（`as_str="scout"`、`section_name="scout_ret"`）；黑板写面对 Scout 为空实现 |
| `orientation.rs` | `AgentRole::Scout`；`state/state_mut` 暂与 `InternalRetrieval` 共用槽位 |
| `prompt.rs` | scout 车道契约文案（只读、findings 带 `path:line`、无子 scout） |
| `retrieval/dispatch.rs` | `(SubagentRole::Scout, "scout")` 臂；SERP 预算/候选计数恒 `None`；gate 域 `"scout"`；**检索启用门对 scout 放行** |
| `retrieval/projection.rs` | Scout **早退白名单**（`read_file`/`grep`/`project_doc_index`）＋负向测试 `scout_lane_projection_is_read_only_whitelist` |
| `agent_loop.rs` | `SubagentRole→AgentRole` 映射；dispatch 臂并入检索分支；主车道工具面追加 `spawn_scout` |
| `agents/scout.rs` | **358b**：`ScoutReport`/`ScoutTask`/解析/渲染/`count_files_read`＋`run_scout_subagent`（独立只读路径，不建 activation/evidence；≤2K 摘要；落 `scout-report/SC-xxxx.md`；`tokens_used` 用 meter 差值）。 |
| `retrieval/dispatch.rs` | **358b**：`role == Scout` 在检索启用门/activation 之前早退到 `run_scout_subagent`。 |
| `controller.rs` | `next_scout_seq`；`scout_semaphore`（默认 2，`ORZ_SCOUT_MAX_CONCURRENCY`）。 |
| `usage_meter.rs` | `token_totals()`（scout tokens 差值读取）。 |
| `prompt.rs` | **358b**：Scout 契约改 JSON（`summary`＋`findings[claim,evidence,confidence]`）。 |
| `tool.rs` / `permission.rs` | **358c**：`spawn_scout` 进 `READ_ONLY_EXEMPT_TOOLS`＋权限桥只读臂（两表同批）。 |
| `orz-host/src/tools.rs` | **358c**：capabilities curated 加 `spawn_scout`（`declared`/`curated`/schema=null）＋coverage 文案。 |
| `orz-host/src/inspect.rs` | **358c**：`run_timeline` 输出 `children[]`（scout `tool_completed` 携带 `child_run_id`/`parent_run_id`/报告 stats）。 |

## §3 完成记录（原顺序）

1. **结果成形** ✅ `agents/scout.rs`：`ScoutReport { report_id, status, summary, findings[], stats }`；全文落 `<journal_dir>/scout-report/SC-xxxx.md`；模型面 ≤2K。
2. **运行时冒烟** ✅ `cargo test -p orz-loop --lib scout`：7/7（含 `scout_dispatch_forms_bounded_report_and_journals_scout_role`：只读子代理读文件→JSON findings→≤2K→落盘→`agent_role=scout`）。
3. **`inspect` 父子视图** ✅ `run_timeline` 的 `children[]`；`run_timeline_lists_scout_children` 钉。
4. **`capabilities` 登记** ✅ `orz capabilities` 列 `spawn_scout`（curated/declared/readonly=true）。
5. **预算与并发** ✅ 并发：`scout_semaphore` 默认 2；预算：`max_wall_s` 硬超时，`max_files`/`max_tokens` 超限标记 `status=partial`（事后如实统计；中途硬停留待后续）。

## §4 关键决策与理由（**不要重新推导**）

- **只读 scout，不做 worker**：本仓库的正确性押在单写者不变式（owner/lease、write control review、journal 哈希链、黑板分区顺序）。写面并行需要 worktree 隔离、审批归属、冲突合并三件新机制，不在本批。
- **工具声明由 loop 侧合成**：与 `retrieval_disposition` 同路，避免动 `orz-tools` 与 host registry 的 14 工具面（工具面扩张是独立裁决）。
- **检索启用门对 scout 放行**：scout 是通用只读侦察，不依赖 web/doc 检索车道；门只约束 internal/external。
- **深度 1**：scout 的白名单里没有 `spawn_scout`，递归面在投影层就被切断。
- **scout 不写黑板**：子代理自身不写黑板分区（`scout_ret` 保留为空）；报告索引由主模型自行落 `findings`（若需要），全文在 `scout-report/`——避免子代理污染主写序。
- **scout 走独立路径**：不建 activation、不写 evidence/黑板；profile 仍是 `LoopProfile::retrieval(Scout, …)`，复用同一 loop 与工具投影。
- **返回契约 JSON**：`summary`＋`findings[claim,evidence(path:line),confidence]`；解析失败降级 `partial`（全文压成 ≤400 字摘要）。

## §5 已知陷阱（踩过或埋着）

- ~~`orientation.rs` 的 `state/state_mut` 让 Scout 与 InternalRetrieval 共用槽位~~ **已解**（审查 P2）：`OrientationSessionState` 增独立 `scout` 槽（`#[serde(default)]`，旧侧车兼容）。
- ~~`in_flight_tools: None` 超时孤儿~~ **已解**（审查 P2）：scout 传本地 `Mutex<Vec<(tool, call_id)>>`，超时补 `ToolCompleted(error=scout_timeout)`。
- ~~`dispatch_retrieval` 仍走 activation/evidence 记账~~ **已解**：`role == Scout` 在检索启用门之前早退到 `run_scout_subagent`，不建 activation、不写 evidence/黑板。
- `scout_ret` 分区名已存在但无人写（按设计不写）。
- `spawn_scout` 只在主车道工具面出现（`profile.role == AgentRole::Main`），子代理面不可见。
- 预算的 `max_files`/`max_tokens` 目前是**事后**判 `partial`（stats 如实），不是 loop 中途硬停；`max_wall_s` 是硬超时。
- `inspect run <child_run_id>` 目前不解析子 run 独立目录——`children[]` 挂在**父 run** 的 `inspect run <parent>` 输出里（子 run 与主 run 同 journal）。

## §6 验收判据（设计 §6 的可执行版）

1. ✅ `orz capabilities` 含 `spawn_scout`（`curated`/`declared`/`readonly=true`，并标 loop-synthesized）。
2. ✅ 冒烟：返回 ≤2K，每条 finding 带 `path:line` 与 confidence。
3. ✅ `inspect run <parent> --json` 的 `children[]` 可见 scout 的 `child_run_id`/报告 stats/终态/子工具序列（子 run 与主 run 同 journal）。
4. ✅ 负向：scout 工具面只有 `read_file`/`grep`/`project_doc_index`（`search_replace`/`run_terminal_cmd`/`spawn_scout` 均不可见）——`scout_lane_projection_is_read_only_whitelist` 钉。
5. ✅ 预算超限 → `status=partial` 且 `stats` 如实（`max_wall_s` 硬超时；`max_files`/`max_tokens` 事后标记）。
6. ✅ 主 run 上下文增量 ≤2K（`SCOUT_SUMMARY_MAX_CHARS`）。

## §7 边界

- 未提交、未推送、未重建；358+359 改动同树。
- 预算的 `max_files`/`max_tokens` 是事后 `partial`，不是 loop 中途硬停（后续批次可加 loop 钩子）；`max_wall_s` 到点若有已得输出降级 `partial`。
- `inspect run <child_run_id>` 不解析独立子目录；父子视图挂在**父 run** 的 `children[]`（含子工具序列）。
- 358 设计稿自记计数 66；现行账 65（0dq 闭合后）。索引未改（本批不新增召回路由）。
- 审查修复覆盖：P0 ToolStarted 配对；P1 轮数 min、timeout partial、summary 400、children tools、findings 索引行；P2 `scope.tools`/`files_read`/目录名/`in_flight_tools`/orientation 槽/`error.class`/失败路径。
