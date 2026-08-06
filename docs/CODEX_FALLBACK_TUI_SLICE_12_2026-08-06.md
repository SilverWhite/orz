# Codex Fallback TUI — Phase 3 Slice #12 审计记录

**日期**: 2026-08-06
**状态**: 完成并已提交（orz `1903ba3` → `cli/feat/fusion-architecture`；主仓库收尾 docs → `origin/main`，推送用户手动）
**范围**: 设计 §2.4 双 TUI 策略的兜底侧——orz-host 新增 Codex app-server JSON-RPC 面 + 新二进制 `orz-codex`（Codex 风格轻量 TUI）。
**测试**: orz-host 48→**71**（+23：codex_app 17 / codex_permission 6）、orz-codex 新 crate **27**；自研 390→**440**。clippy 零新增（基线对比：修掉存量 `unused Arc` 1 个，新增 0 个）。

## 1. 交付物

| 文件 | 内容 |
|---|---|
| `orz/crates/orz-host/src/codex_app.rs`（新，~1900 行含测试） | Codex app-server JSON-RPC 面：`JsonMessage` framing（JSONL，无 `jsonrpc` 字段——fixture parity）、`read/write_json_message`、`CodexAppServer`（组合 `Arc<AcpServer>`）+ `CodexServerParts`（server + 出站 rx + 网关 rx）、`run_codex_server` 连接入口、网关翻译任务（ACP SessionNotification → `item/started`/`item/delta`） |
| `orz/crates/orz-host/src/codex_permission.rs`（新，~400 行含测试） | `CodexPermissionBroker`（approval 请求注册表 + 出站 sink）+ `CodexPermissionTransport`（impl `PermissionHookTransport`：`approval/request` 发请求、300s 可注入超时 fail-closed、decision 映射） |
| `orz/crates/orz-host/src/permission.rs`（+15） | `spawn_with_hub`（原 81 行硬编码 `None` 的穿透；`spawn` 委托） |
| `orz/crates/orz-host/src/lib.rs`（+15） | `with_bridge_and_hub`（`with_bridge` 委托）；module 声明 |
| `orz/crates/orz-host/src/acp_server.rs`（+25） | `hub_permission` 字段（Mutex 包裹）+ `set_hub_permission` + `build_host` 透传 |
| `orz/crates/orz-codex/`（新 crate，7 文件 ~2900 行含测试） | 兜底 TUI：`client.rs`（app-server JSON-RPC 客户端，进程内 duplex）、`app.rs`（`CodexApp`：消息/输入/状态/审批对话框）、`widgets.rs`（三带渲染 + 对话框 overlay，无 assurance 面板）、`runner.rs`（select! 循环 + teardown 纪律）、`snapshot.rs`（TestBackend 渲染助手）、`main.rs`（`[[bin]] orz-codex` 入口） |
| `orz/Cargo.toml`（+2） | workspace member + 依赖条目；Cargo.lock 仅新增本 crate（零新外部依赖） |

## 2. 协议面（wire 形状全部由仓库既有资产钉死）

实现面（对照 `runtime/fixtures/codex-app-server-lifecycle-v0.1/` 与 `fake_codex_app_server.py`）：
- 客户端→服务器：`initialize`(id 0) → result `{userAgent: orz-codex-host/{version}, platformFamily, platformOs}`（fake parity）；`initialized`（notification，no-op）；`thread/start {ephemeral, sandbox}`（服务器生成 `thr-{n}` 或接受客户端未知 id；已知 id 重复 → `-32001`）；`turn/start {threadId, input:[{type:text,text}]}`（text 块拼接；立即响应 `result.turn {id: turn_{n}, status: inProgress, items: [], error: null}`，fixture 顺序：响应 → `turn/started` → user item → 流式）；`turn/interrupt {threadId, turnId}`（**响应非终局**——`interrupt-without-terminal` fixture 语义，任务稍后发 `turn/completed{interrupted}`；空闲 interrupt 良性 no-op，ACP 先例）；`thread/unsubscribe`（响应 `{"status":"unsubscribed"}` + `thread/closed` + `acp.close_session`）
- 服务器→客户端：`thread/started`（idle status 形状）、`turn/started`、`item/started`（流式首 chunk 前置）/`item/delta`/`item/completed`（成功 `agentMessage` 终局文本；`turn/completed {status: completed|interrupted|failed, error{message}}` 对映 journal 终局 `run_finished`/`run_cancelled`/`run_failed`）、`thread/closed`
- **错误码**：`-32700` 坏行（id null，连接存活）、`-32601` 未知方法、`-32602` 参数错、`-32001` 冲突（运行中 turn/start、重复线程）
- **EOF 永不被提升为终局**：断连的 turn 继续跑到 journal 完成（协议研究钉死；测试 `eof_mid_turn_keeps_turn_running_to_valid_journal` 锁定）
- **每线程单活跃 turn**；多线程并发各自独立 journal（`concurrent_turns_*` 测试）

## 3. 权限扩展（超出现有 capture 面的扩展，wire 规范）

`approval/request`（服务器→客户端**请求**，带 id）+ 客户端 `approval/response`（JSON-RPC 响应，同 id）：
- request params = hub 的 `build_permission_payload` 形状（`tool_call_id`/`tool_name`/`description`/`scope` + `bash_command`/`edit_file_paths` 上下文）
- response result = 简单决策 `{"decision": "allow_once" | "allow" | "deny"}`（host 拥有 hub 语义，客户端不感知 scope kind）
- 映射（`CodexPermissionTransport`）：`allow_once` → `{outcome: approve}`；`allow` → `{outcome: always_approve}` + bash 时带 `{scope: {kind: bash_command, value}}`（Edit 访问由 manager 自动派生 `AllowEditsForSession`）；`deny`/未知 → `{outcome: reject}`（fail-closed）
- **超时双保险**：hub 路径 manager 无内置超时（`request_permission_via_hub` 无界 await）→ transport 自持 `PERMISSION_PROMPT_TIMEOUT`（300s，可注入）；`PermissionBridge::request` 外层再包一次
- **允许语义记录**：`PermissionBridge::request` 在 LoopHost 边界把 `Decision::Allow` 一律映射 `PermitDecision::AllowOnce`（permission.rs 存量行为）——journal 上 `allow_always` 永不出现；"始终允许"的可观察语义由 manager 持久化 grant 承担（本次执行 + 未来相同调用自动放行，wire 上第二次不再弹窗——测试 `approval_allow_persists_for_identical_bash` 锁定：2× tool_started、2× allow_once、wire 零第二次 approval/request）

## 4. 与 orz 的功能一致性（用户裁决"底层完全一致、可完全正常使用"）

兜底驱动的是与 orz **完全相同**的 host 机制：同一 `AcpServer`（run-id 预留/取消 token/in-flight 互斥/journal shutdown 全路径，组合复用零复制）、同一 `AgentLoopController`（gate/定向/plan 静默运行）、同一 `OrzHost` 28 工具 + IP6 PermissionBridge（审批经 approval 通道）、同一文本流式链路（Slice #6 `on_text_delta` → 网关翻译）。差异仅在**协议门**（app-server JSON-RPC vs ACP）与**渲染面**（无 assurance 面板）。

## 5. 关键设计决策与边界（记录）

1. **sandbox fail-closed**：v1 仅接受 `workspace-write`；`read-only`（需权限 override）/`danger-full-access`（需 yolo）→ `-32602`，后续 slice。兜底 TUI 恒发 workspace-write，自产品可用性不受影响。
2. **`ephemeral` 仅信息性**：journal 照写——assurance 要求 journal=证据，与真实 Codex 的 ephemeral 线程语义不同，记录。
3. **`userMessage` item 类型是扩展**：repo 捕获只见过 `agentMessage`；客户端类型宽容（TUI 也渲染自己的提交），真实前端 drop-in parity 前需对 codex-rs `protocol_v1` 复核。
4. **delta-after-completed 竞态**：run 返回时已在网关通道的 delta 可能落于 `item/completed` 之后——客户端对已封存 item 丢弃 delta（完成文本权威）；与 controller.rs 记录的残余边界同精神。
5. **thread/closed 时序**：真实 Codex 等最后订阅者 + ~30min idle unload；本实现首个 unsubscribe 即关（单进程单视图结构上成立）。
6. **quit-during-run teardown（对 orz-tui 的改进）**：运行中 Ctrl+C 先 `turn/interrupt` 再限时（5s）排空至 `turn/completed{interrupted}`——journal 以有效 `run_cancelled` 终局而非中断写入；5s 超时兜底退出（记录）。
7. **`initialized` 顺序宽松**：不强制 initialize 先行（记录；真实客户端按协议顺序发）。
8. **服务器生成线程 id `thr-{n}`**：进程级计数器；客户端可自供未知 id（确定性测试路径）。
9. **`initialize` result 3 字段**：镜像 repo 自己的 fake；真实 0.145.0 可能含 `protocolVersion`——客户端忽略未知字段。
10. **无 `jsonrpc: "2.0"` 版本字段**：fixture 捕获不带，字节级 parity 优先。

## 5.5 提交前审查闭环（三独立代理，2026-08-06）

- **实现正确性**：**P1×2 发现并处理**。**P1-1（记录为接受边界）**——取消 token 在权限 await/长工具执行内不可达（协作式取消，Slice #7 明确记录"权限 await 内不设检查点"）：审批挂起场景由 D2 修复确定性解决（wire deny 解阻塞 → 检查点 → run_cancelled，回归测试锁定）；长工具执行受协作式语义限制，5s 排空超时兜底退出（drain 文档如实标注 best-effort——与 orz-tui 立即退出同类，工具本身永不被中途强杀）。**P1-2（已修复）**——确定性线程 id `thr-{n}` 复用 run 目录：两次 orz-codex 同目录运行（或同 id 线程重建）会 append 到旧 journal 链（append 模式）→ 哈希链损坏。修复：`thr-{uuid}` 随机 id（stdio UUID 先例）+ **退休线程墓碑**（unsubscribe 后同 id 重建 → `-32001` 拒绝）；+2 回归测试（`generated_thread_ids_are_unique_across_servers_in_same_cwd` / `retired_thread_id_recreation_is_refused`）。**P2-1（已修复）**——run() 早期 `?` 路径 raw-mode 泄漏（EnterAlternateScreen/Terminal::new 失败时）：raw/alt 状态追踪 + 全路径回卷。**P2-2（记录）**——终局通知与 active_turn 清除同任务原子，客户端处理窗口毫秒级；TUI 的 Running 守卫覆盖人尺度窗口；客户端在终局后中断为良性（下一 prompt 2s 窗口内预取消，可见"已中断"+重试）。**P3×3 修复**——started_turns 按 (thread, turn) 键控（turn id 每线程重复）、transport/client pending 清理全路径（send 失败/responder 丢弃/超时）、空 bash_command 不持久化空 grant；**P3×3 记录**——bootstrap 失败时 wire 报 failed 而 journal 无终局（存量 handle_session_prompt 行为，如实记录）、EOF 中途审批 300s 慢速 fail-closed（延迟接受）、工具执行进度不上 wire（host 机制级一致）。
- **设计-实现符合性**：无 D1——五项用户裁决 + §2.4 全部结构验证成立（无 assurance 面板渲染、assurance 静默运行于同一 AgentLoopController、orz-bin 零 diff、wire 形状逐字段对照 7 fixture + fake server）；**D2-1 修复**：`turn/started` 携带 fixture 未载的附加 `threadId` 字段（真实 protocol_v1 有此字段，保留 + 本记录）；**D2-2 修复**：审计文档去除幻影 wire 类型名（实现用 `json!` 值，计划承诺的类型化 struct 未落地——记录为计划偏差）；D3×7 记录。
- **设计合理性**：无 D1；**D2 修复**——审批挂起时退出：run 阻塞在权限 await 内、取消检查点不可达、5s 排空超时即 journal 残缺。修复：`drain_until_cancelled` 先 wire 上 `deny` 解阻塞再 interrupt（+回归测试 `quit_during_approval_answers_deny_and_lands_cancelled_terminal` 锁定 journal run_cancelled）；`do_interrupt` 同款（Ctrl+Z 带审批对话框）。D3×10 记录。
- **修复清单**（全部落地）：① quit-drain/do_interrupt 审批先 deny；② 对话框打开时 Ctrl+C/Ctrl+Z 保持可用（状态行承诺兑现）；③ 死字段 `initialized` AtomicBool 移除；④ `thr-{n}` 生成 id 与客户端自供 id 碰撞循环（防覆盖会话）；⑤ no-assurance 断言收窄（`文件` 从面板串列表移除——审批对话框合法渲染 `文件: {paths}`，改断言 orz-tui 冻结菜单栏全串不出现）；⑥ `started_turns` 无界注释。
- **记录（D3）**：turn/started 附加 threadId（D2-1 保留项）；unsubscribe 中途 + 同 id 线程重启会撞 journal（`RUN-{suffix}-0` 双写）——兜底 TUI 不可达（服务器生成 id + 退出即关），未来 stdio 门需防；工具执行进度不上 wire（仅审批弹窗 + 终局文本可见——"完全一致"是 host 机制级而非显示级，记录）；`platformFamily == platformOs`（fake parity 已知瑕疵）；delta-after-completed 极端情形（turn N 迟到 delta 被 turn N+1 归因——翻译任务按当前活跃 turn 归因，人类操作窗口不可达，网关侧加 turn 元数据为未来 slice）；计划偏差：`run_codex_server(parts, transport)` vs 计划 `(server, read, write)`、`connect_inprocess(parts)` vs `(server)`、类型化 wire struct → `json!` 值；测试拆分 app 10→11 / widgets 7→6（总量正确）。

## 6. 测试要点

- orz-host codex_app 17：framing 往返/分类、坏行 -32700 连接存活、未知方法/参数错、thread/start fixture 形状 + sandbox fail-closed + 重复线程 -32001、turn happy path（响应/通知顺序、user item、流式、完成、journal run_finished）、chunk 拼接=响应文本、interrupt（interrupted + run_cancelled + 同线程第二 turn 成功）、空闲 interrupt 良性、运行中 turn/start 冲突、failed 映射（script exhausted + run_failed）、unsubscribe（closed + 会话移除 + 后续拒绝）、审批 allow_once 执行（journal 无 "Tool not found"）、deny 不启动、allow 持久化、超时 deny、EOF 断连 journal 仍有效、双线程并发各自 journal
- orz-host codex_permission 6：decision 映射 6 分支、broker 注册/解析/迟到 no-op、transport 往返、allow 映射、超时 fail-closed + pending 清理、通道关闭 fail-closed
- orz-codex 28：client 5（初始化+流式+完成+journal、审批往返、interrupt、错误面、close_thread）、app 11（提交守卫/trim/delta 累积/完成替换+封存/无流创建/echo 忽略/终局态/对话框循环/取消提示/键路由/对话框内 Ctrl+C+Z）、widgets 6（空闲无 assurance 面板断言、角色+流式标记、完成态、审批 overlay、视口过小、对齐）、snapshot 1、runner 5（E2E 流式+journal、中断流、审批 allow/deny、二次提交守卫、**审批挂起退出 → run_cancelled**）

## 7. 冒烟

`cargo run -p orz-codex -- --fake-provider` 启动进入事件循环无崩溃（8s 内无错误输出）。交互验证（提示词 → 流式 → 审批对话框 → 中断 → 退出后 journal verify）需真实终端手动执行。

## 8. 遗留（更新）

- **reasoning_content 回放缺口**（Slice #11 遗留，不变）：V4 thinking 模型或要求；live 验证后打 async-openai fork 补丁
- 严格 Clippy/零死代码收尾（orz-telemetry/orz-announcements/orz-config-types 保留待裁）
- Python reference-spec
- conformance suite 验证（payload schema good/bad fixtures 补齐）
- Codex 面扩展路径（sandbox read-only、thread resume、approval scope 客户端直出）——后续 slice
