# Codex 面扩展 — Phase 3 Slice #16 审计记录（sandbox read-only + thread resume 闭合 + approval scope 直出）

**日期**: 2026-08-06
**状态**: 完成（orz 本地提交待推送，主仓库 docs 收尾待推送——用户手动惯例）
**范围**: Slice #12 遗留三项 Codex 面扩展，用户 2026-08-06 裁决落地。
**测试**: orz-host 73→**78**（+5：permission 2 / acp_server 1 / codex_app 2 新增 + 1 改写）、orz-codex 28→**33**（+5：client 2 / runner 1 / widgets 2）；兄弟回归 orz-tui 174 / orz-bin 2 全绿；clippy 全零警告（Slice #13 纪律）；`cargo fmt --check` 零漂移（Slice #15 后门禁）。

## 1. 用户裁决回顾（2026-08-06，先于设计）

1. **thread resume → 文档闭合（零代码）**：探索证实真实 Codex 协议"线程恢复" = 同一 threadId 上再次 `turn/start`（`interrupted-then-second-turn` fixture 钉死），orz-host 已实现并有测试锁定（`interrupt_produces_interrupted_terminal_and_next_turn_succeeds` codex_app.rs）；全库 0 处 `thread/resume` 方法、0 fixture、0 schema 状态；新增 wire 方法违反 fixture parity 原则 → 不实现。
2. **sandbox read-only**：风险类 ReadOnly（read_file/grep/list_* 等）自动放行真实执行；Bash/Edit/MCPTool 等 mutation 一律 Deny 不弹窗（fail-closed，"写永不发生"）；客户端 CLI `--sandbox read-only|workspace-write`（默认 workspace-write，沿用"默认不启用"裁决）。
3. **approval scope 客户端直出**：widgets.rs 渲染 scope（"范围: 读/写"中文标签）；tool_call_id 不渲染（wire 关联 ID 无用户决策价值，params 已全量保留）。

## 2. 交付物

| 文件 | 内容 |
|---|---|
| `orz/crates/orz-host/src/permission.rs` | 新 `PermissionPolicy {Interactive(默认), ReadOnly}` + `PermissionBridge.policy` 字段 + `spawn_with_hub_and_policy`（`spawn`/`spawn_with_hub` 委托 Interactive 零破坏）；`request(risk, ...)` 启用 risk 参数——`ReadOnly && risk != ReadOnly` → `Ok(Deny)` 短路于 `handle.request` 之前 |
| `orz/crates/orz-host/src/lib.rs` | `OrzHost::with_bridge_and_hub_policy`（`with_bridge_and_hub` 委托 Interactive） |
| `orz/crates/orz-host/src/acp_server.rs` | `StoredSession.policy` 字段；`handle_session_new_with_policy`（原函数体迁移，`handle_session_new` 委托默认）；`handle_session_prompt` 读 policy → `build_host` 透传 |
| `orz/crates/orz-host/src/codex_app.rs` | `thread/start` sandbox 枚举匹配（`workspace-write`→Interactive / `read-only`→ReadOnly / 其他含 danger-full-access → -32602）；L423 resume 文案更新；头注释重写（sandbox 双值 + resume c1/c2 闭合） |
| `orz/crates/orz-host/src/codex_permission.rs` | 头注释补 read-only 短路说明 |
| `orz/crates/orz-codex/src/client.rs` | `initialize_and_start_thread(sandbox)` 参数化（5 处调用回归改传 workspace-write） |
| `orz/crates/orz-codex/src/runner.rs` | `TuiConfig.sandbox`（Default workspace-write）→ run_loop → do_prompt 透传 |
| `orz/crates/orz-codex/src/main.rs` | `--sandbox <read-only|workspace-write>`（非法值 → 提示 + help + **exit 2**） |
| `orz/crates/orz-codex/src/widgets.rs` | `render_approval_dialog` `描述:` 后 scope 行（read→读 / write→写 / 其他原样；缺 scope 容错不渲染）；tool_call_id 不渲染 |
| 主仓库 `docs/CODEX_FALLBACK_TUI_SLICE_16_2026-08-06.md`（本文件） | 三项裁决实现 + 语义记录 |

## 3. thread resume 闭合（c1 实现 / c2 拒绝）

- **c1（同线程恢复）— 已实现，文档闭合**：真实协议的"恢复"是同一 threadId 上的第二次 `turn/start`。orz-host 自 Slice #12 已实现并有测试锁定：`interrupt_produces_interrupted_terminal_and_next_turn_succeeds`（codex_app.rs:1301 区域）——interrupt 后同线程第二次 turn/start 成功，各自独立 journal 终局。本次仅更新拒绝文案与头注释：`"thread {tid} already exists — a second thread/start on a live thread is refused; turn/start resumes it"`。
- **c2（跨连接恢复）— 拒绝，理由记录**：① journal 为 append-mode 哈希链，跨连接恢复（新连接复用 threadId）会把后续 run append 到旧 journal 链上——链损坏（P1-2 同一约束：`thr-{uuid}` 随机 id + 退休墓碑即因此而生）；② 全库 0 处 `thread/resume` 方法证据（0 fixture、0 schema 状态、0 capture）——新增 wire 方法违反 fixture parity 原则。

## 4. sandbox read-only 语义

**策略存储**：session == 线程（codex 面 thread_id == ACP session_id）→ per-thread 策略落点 `StoredSession.policy`（per-session bridge 天然支持双沙箱线程共存；hub transport 是连接级单槽无线程上下文，不可行——设计探索结论）。

**判定链**（`PermissionBridge::request`）：
```
ReadOnly && risk != RiskClass::ReadOnly → Ok(Deny)   // handle.request 之前短路
ReadOnly && risk == ReadOnly → access_in_scope → manager 自动 AllowOnce
Interactive（默认）→ 原逻辑不变
```
- 短路位置在 `handle.request` **之前** → 不弹窗、无 approval wire、无 tool_started。
- **证据链**：controller 对每个工具调用无条件记录 `PermissionRequested` + `PermissionDecision{deny}`——"写永不发生"由 journal 证明（测试断言 deny + 恰 1 次 tool_started（只读工具）或 0 次）。
- **read-only 线程永无 SnapshotCreated**：mutation 从未过闸（IP5 snapshot 接线在权限放行后触发，Slice #4 顺序保持）。
- **restore/快照不受影响**：host 动作不走工具权限（acp_server.rs 记录）。
- **read-only 下 journal 照写**：assurance 静默完整运行（§2.4 不变）。

**NetworkCall 与 MCPTool 裁决**：
- NetworkCall（web_fetch/web_search）read-only 下 **Deny**：网络有真实外部副作用（fetch 可外发本地内容），"mutation 一律 Deny"按 fail-closed 解释为"除本地只读类之外一律 Deny"；`scope_for_access` 标 WebFetch 为 `"read"` 是 chat 审批 payload 提示语义，非沙箱安全边界。
- MCPTool read-only 下 **Deny**：读写性无法静态分类，纯读 MCP 是已知牺牲品（记录）。

**测试适配（终版，历经两次修正）**：NetworkCall 的 codex_app 层 E2E **架构上不可构造**。推理链：① 初版理由"ToolNotFound 先于权限层"**错误**（对 Host 类工具，权限门在分派之前）；② 设计审查补正"权限门先于分派、E2E 可行"——对 Host 类成立，但**对 web 工具不成立**：`route()`（orz-loop/relay.rs:26-29）把 web_search/web_fetch 归为 retrieval 形状，在 `run_host_tool` **之前**就路由到 ExternalRetrieval 子代理——权限桥对它们永不被触达（实测：脚本硬塞 `web_fetch` 在子代理路径失败，policy gate 从未参与）。终版：NetworkCall 拒绝由 **permission 单元测试钉死**（`read_only_policy_denies_non_read_before_manager` 含 web_fetch → Deny，直接驱动桥）+ tool.rs risk 分类测试（web_search → NetworkCall 锁定）。若未来把 web 工具改为 Host 类直通，E2E 可补。

**双沙箱共存**：同一服务器上 Interactive 与 ReadOnly 线程并行，各自 journal；`read_only_and_workspace_write_threads_coexist` 锁定——恰 1 次 approval 且属于 workspace-write 线程（按 bash_command 区分）。

## 5. approval scope 直出

- `render_approval_dialog` 在 `描述:` 之后渲染 `范围: 读|写`（hub `scope_for_access` 产出 `"read"`/`"write"`；未知值原样显示；缺 scope 容错不渲染）。
- **tool_call_id 不渲染**：wire 关联 ID 对用户决策无价值，params 已全量保留（tool_name/description/scope/bash_command/edit_file_paths 渲染面不变）。测试锁定 `!rendered.contains("tool_call_id")`。

## 6. --sandbox CLI

`orz-codex [--sandbox <read-only|workspace-write>]`（默认 workspace-write，显式调用才启用——"默认不启用"裁决延续）。非法值 → 错误消息 + help + **exit 2**。`--sandbox` 仅在线程创建时生效（首 prompt 的 `thread/start`），后续 prompt 复用活动线程（do_prompt 注释言明）。

## 7. 零破坏论证

- stdio/orz-tui 走 `handle_session_new` / `with_bridge_and_hub` / `spawn_with_hub` 原签名（默认 Interactive）——15 处测试调用点仅机械补参（helper 签名扩展，语义不变）。
- LoopHost trait、wire 协议、ThreadEntry/new_parts、run-event schema 零改动。
- 兄弟 crate 回归：orz-tui 174 / orz-bin 2 全绿。

## 8. 测试清单（落地）

- **permission.rs 2**：`read_only_policy_denies_non_read_before_manager`（allow_all handle 下 SandboxEscape/NetworkCall/LocalMutation 三风险类 → Deny 不触 manager；**MCP 名 `read_server__extract` 即使 ReadOnly 标签也 Deny**——D2-1 防御锁定；cwd 内 read → AllowOnce；cwd 外 read → Deny——P1 scope 仍生效）；`interactive_policy_is_default_and_unaffected`（默认 Interactive + allow_all 下 bash → AllowOnce，证明短路是策略门控非工具门控）。
- **acp_server.rs 1**：`session_prompt_respects_session_policy`——AlwaysAllowTransport hub + 双会话（ReadOnly 拒 bash 无 ToolStarted；Interactive 执行）——证明 StoredSession.policy 直连 prompt 路径。
- **codex_app.rs 改写 1 + 新增 2**：`thread_start_fixture_shapes_and_fail_closed_sandbox` 改写（read-only 接受 + idle 形状；danger-full-access/未知值仍 -32602）；`read_only_sandbox_allows_read_and_denies_mutation_without_prompt`（read 执行 + bash 无 approval 帧 + journal tool_started 恰 1 + deny + run_finished；drain 循环任何终局 break 并断言 completed——运行失败断言而非超时）；`read_only_and_workspace_write_threads_coexist`（交错脚本 [A,B,text×4]，恰 1 approval 属 ww 线程，双 journal 断言）。网络 E2E 架构不可构造（见 §4 终版理由），NetworkCall 由 permission 单元钉死。
- **orz-codex 5**：client `read_only_sandbox_denies_bash_without_approval` + `read_only_sandbox_executes_read_tools`；runner `read_only_sandbox_prompt_denies_bash_without_dialog`（无 WaitingApproval、无对话框、deny journal）；widgets `approval_dialog_overlay_renders_with_countdown`（补 scope: write 断言 + tool_call_id 不渲染）+ `approval_dialog_read_scope_renders_chinese_label` + `approval_dialog_without_scope_renders_tolerantly`。
- **main 手动**：`--sandbox bogus` → exit 2；`--sandbox read-only --fake-provider` 冒烟需真实终端（runner E2E 已覆盖无对话框路径）。

## 9. 冒烟

`cargo run -p orz-codex -- --sandbox read-only --fake-provider`：bash 静默拒绝无弹窗（demo 脚本 read_file 执行 + run_terminal_cmd 被拒），交互验证需真实终端手动执行（用户惯例）。`--sandbox bogus` 已自动验证 exit 2。

## 9.5 提交前审查闭环（三独立代理 + 实现正确性自查，2026-08-06）

- **实现正确性**（代理因 API 余额 402 中途终止 → **改为自查**）：权限门先于工具分派的断言链复查（controller PermissionRequested → request_permission → Deny 短路跳过 call_tool）；StoredSession.policy 与 prompt_count 同一临界区读取（sessions 锁内，无 check-then-act）；sandbox 枚举三态 fail-closed；client/runner/main 透传链 5 处调用点 grep 全量核对；drain 循环改进（任何终局 break + 断言 status，失败不再伪装为超时）；零破坏路径（stdio.rs:76 等 25 处 `handle_session_new` 原签名调用点未动，orz-tui/orz-bin 回归全绿）。
- **设计-实现符合性**：**无 D1**；D2 唯一一项（网络 E2E 未按设计清单落地）处置合格——删除理由经设计合理性代理质疑、自查修正后以**终版理由**（路由截获）落定，见 §4；D3×3 记录（行号位移、validate_sandbox 内联化、widgets 断言补强而非快照更新）。
- **设计合理性**：**无 D1**；**D2×4 修复/处理**——① **D2-1 修复**：MCP 工具名（`{server}__{tool}`）前缀可被归为 ReadOnly 绕过短路（risk_class 全名前缀匹配）→ 短路条件加 `tool.contains("__")` 恒拒 MCP + 回归测试锁定（当前 toolset 无 MCP 配置不可触发，防御性一行）；② **D2-2 处理（两轮）**：审查称"权限门先于分派、网络 E2E 可行"——对 Host 类成立但**对 web 工具不成立**（`route()` 在 run_host_tool 前把 web 工具路由到 ExternalRetrieval 子代理，权限桥永不触达）；终版裁决 = E2E 架构不可构造 + permission 单元钉死（§4 推理链完整记录，含初版理由的错误与修正）；③ **D2-3 处理**：`--sandbox` 线程创建后不可切换——与真实协议一致（sandbox 是 thread/start 参数），help 文案补"仅线程创建时生效，切换需重启"；④ **D2-4 处理**：permission 单元测试 risk 标签与真实链路不一致（run_terminal_cmd 真实归类 LocalMutation 非 SandboxEscape）→ 测试改用 `bash` 别名 + 注释言明；D3×3 记录。
- **D3 记录**：① **resume c2 盲区**——live/retired 双 -32001 仅覆盖进程存活期；服务器重启后同 threadId 的 `thread/start` 静默接受、bootstrap 不检查 run 目录存在 → 链损坏。当前不可达（orz-codex 客户端不传 threadId、服务器生成 `thr-{uuid}`、app-server 进程内嵌无外部 wire 入口）；未来接入真实外部 Codex 客户端时需在 thread/start 加磁盘 run 目录存在性检查。② **read-only 牺牲品清单**（"除本地只读类之外一律 Deny"的必然结果，除 MCPTool 外）：`ask_user_question`（read-only 下模型无法向用户澄清）、`get_task_output`（后台任务输出读取）、`scheduler_*`/`task`/`todo_write`、`kill_task`、`enter_plan_mode`/`exit_plan_mode`、`lsp` 等 GrokBuild 语义工具。③ **NetworkCall Deny vs scope "read" 标签差异**已认可记录（chat 审批提示语义非沙箱安全边界）。

## 10. 遗留（更新）

- **conformance suite 验证（#7——Rust↔Python 交叉验证）——Phase 3 正式收尾仅剩此项**
- reasoning_content 回放缺口（live 验证后 fork 补丁，不变）
- 后续（未排期）：web 工具配置启用时补 NetworkCall E2E；`--sandbox` 帮助文案进 README（推送后惯例）
