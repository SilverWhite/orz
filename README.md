# orz — Local Assurance-First CLI Agent

状态：2026-08-05。**融合架构 v0.2 已冻结**——从 Grok Build 取用成熟底座（tools/workspace/sandbox/mcp/chat-state），
自研 Agent Loop 引擎（orz-loop）与 Assurance 层（orz-assurance），采纳 Codex 式架构纪律（薄核心、单向依赖、零死代码）。

**实施基线**：`feat/fusion-architecture` 分支 — 70 crate workspace，`cargo test --workspace` 全绿（0 failed）、clippy 0 error（Windows 首跑通）。
Phase 1（journal + transport + 单 Agent loop）、Phase 2（单主 Agent + gates + ACP + Plan Mode）已完成；
Phase 3 Slice #1（OrzHost + IP6 PermissionBridge 接线）、Slice #2（§4.6 问询三机制）、Slice #3（session snapshot + sandbox/credential/permit）、Slice #4（IP5 snapshot 接线 + host keystore 注入 permit）、Slice #5（orz-tui 核心工作台 v1）、Slice #6（host 流式 text-delta 生产者）、Slice #7（/stop 真取消 + 权限对话框 300s 倒计时）已完成（自研 292 tests——Slice #7 新增 18：loop +4、host +5、tui +9）。
Slice #13 已本地提交（待推送——用户手动惯例）——**Codex 纪律收尾第一项：严格 Clippy 零警告 + 死代码裁剪**（设计 §Phase 3 item 4 闭合）：① 死代码裁剪——orz-telemetry 8 死模块删（appender/context/debug_log/hooks_log/prompt_timing/sampling_log/sentry/unified_log，live 闭包 15 模块可达性验证）+ 4 依赖裁（git2/sentry/obfstr/orz_env，Cargo.lock -331 行含 sentry 家族 ~24 传递包）；orz-config-types 3 死模块删（mcp/permission/pool）+ 3 依赖裁；orz-http 死依赖裁；orz-announcements 审计判定 LIVE 保留；16 个产品不可达 codegen crate 为设计文档 §4 kept-provider 清单明确保留（记录不删）。② **clippy 全仓 128 → 0 警告**（两轮 --fix + ~30 人工：canonicalize→dunce ×2 履行 clippy.toml 禁令、PermissionBridge.journal 死字段删、orz-assurance 16× 死 cfg_attr(serde) 删、平台 cfg 门控 ×4、Box<RunEvent>、let-chain/match→if-let/sort_by_key 等机械修复）。③ 验证：`cargo test --workspace -j4` **10171 passed / 0 failed**（并行 -j 默认下 rust-lld OOM——-j4 稳定；orz-tools 并行负载 flaky 一次 4 轮单跑干净非回归）；自研 444 tests（assurance 96 / loop 71 / host 73 / tui 174 / codex 28 / bin 2）；diff 86 文件 +423/-4441。Phase 3 收尾剩余：Python reference-spec、conformance suite 验证。
Slice #11 已提交（orz `24d332b` → `cli/feat/fusion-architecture`，推送用户手动）——**真实 transport SSE 流式接线**（闭合遗留清单首项）：`DeepSeekTransport` 桩 → async-openai fork 真实实现——`generate` 真实 HTTP + `generate_stream` SSE 帧聚合（截断流检测：EOF 无 finish_reason 报错不 journal 为 stop）+ 流内取消（P3-7：select! 即时唤醒 + drop 关连接 + 预取消零请求 + `GatewayError::Cancelled → RunCancelled` 终局链）+ 协议正确性（D2-1/P1：assistant tool_calls 声明回放 + **run_host_tool 补 Tool 结果消息回放**（存量缺陷） + system 提示词前置 wire）+ P2-2 in-flight 单锁化（`runs: RunInFlight{Prompt|Restore}` 双向原子互斥）。三独立代理审查闭环：D1-1 system 消息 / D1-2 单锁 / D1-1 截断流 / D1-2 取消活性 / D2-1 assistant 回放全部修复；记录：reasoning_content 待 fork 补丁（live 验证后）、流路径无 429 重试、ContentFilter/FunctionCall 塌缩、close_session 边角。orz-loop 60→**71**（mock TCP E2E 全链路）、orz-host 45→**48**；clippy 零新增（git blame 验证）；live 测试双重门控 ORZ_TEST_LIVE+key。
Slice #10 已提交（orz `e65bb30` → `cli/feat/fusion-architecture`，推送用户手动）——TUI 快照选择器（restore 触发入口，闭合剩余清单首项）：`/snapshots` + Alt+S 打开选择器（新 `snapshots.rs`：全量扫 `.gsa/runs/RUN-*/events.jsonl` 的 snapshot_created 事件——最小本地反序列化保持 bridge-only 规则、manifest 文件数/日期富化、数值 turn 排序 + hash 去重）；两阶 Enter 确认（确认行显"覆盖当前文件（无撤销）"）；runner `do_restore` 直调 `AcpServer::restore_snapshot(session_id, hash, None)`（scope=None 全量——v1 裁决；`restoring` 标志双挡 prompt 提交与 cancel——host 无 restore 取消 token（P3-7）且无 in-flight 标记（P2-2）；排水臂预渲染"恢复中"帧——RST- journal 永不被 TUI tail，返回值即完成信号；绝不触碰 prompt_count——run-id 独立性由 host 测试钉死）。三独立代理审查闭合：实现无 P1、符合性无 D1/D2、设计合理性无 D1——D2×4 修复（loop 冻结预渲染帧 + 陈旧权限队列确认时清除 + 无撤销警告 + **host 一行修复**：bootstrap 失败释放 cancel token——泄漏会使恢复被永久误拒为"运行中"）+ D2-2 页脚"列表为打开时快照"标注 + D2-5 跨会话排序接受记录（与 discover_sessions 一致）。orz-tui 147→**174**、orz-host 44→**45**；clippy 零新增（stash 基线验证 lib 10/test 19）。
Slice #7 已提交推送（orz `b356048` → `cli/feat/fusion-architecture`，+1473/-122；主仓库 `7f5f7cc` → `origin/main`，用户手动推送）——① 协作式取消（原 CancelNotification no-op）：`AgentLoopError::Cancelled` + `run_turn_with_cancel`（run_turn 签名不变）+ 检查点（循环顶先于 120ms pacing / 模型轮后 / 工具分发前及**每工具前**；权限 await 内不设——协作式）+ 终局 `RunCancelled{"reason":"user_cancelled"}`（零 schema 变更）；host per-session token map（token 注册提前到 bootstrap 前）+ pending_cancels 2s 时间戳窗口（bootstrap 竞态兜底，陈旧 cancel 过期）；stdio `Err(Cancelled)` → `PromptResponse(StopReason::Cancelled)` 成功响应（ACP 协议契约）；TUI `stop_pending` + `do_cancel` + seq 代际守卫（陈旧完成不 stop tail/不清 running/不 dismiss）。② 300s 倒计时：`PERMISSION_PROMPT_TIMEOUT` pub const 单一事实源、`PendingPermission{opened_at}` 展示时重打、50ms tick 渲染"剩余 Ns"行 + 超时 dismiss 恢复运行中。提交前审查闭合（无 P1）：P2×2（pending 标记毒害后续 prompt——加 2s 窗口 + 清理路径；多工具轮每工具检查点——新增锁定测试）+ P3×1 修复（队列项保留等待审批）+ 3 记录（流内取消/并发 prompt token 覆盖/倒计时偏移）。全仓 CARGO_EXIT=0、clippy 0 error 零新增。
Slice #6 已提交推送（orz `8c8edf3` → `cli/feat/fusion-architecture`，主仓库 `4e1798b` → `origin/main`，用户手动推送）——模型输出流式传输到 TUI：`ModelGateway::generate_stream`（回调式，默认委托 `generate`，DeepSeekTransport 桩不动）+ FakeProvider char 边界分块（chunk_size/chunk_delay）→ `LoopHost::on_text_delta`（同步默认 no-op）→ OrzHost `forward_fire_and_forget` 发 ACP `agent_message_chunk` 通知（实时、非 journal、无 schema/EventType 变更——Python live-only 先例）；controller 循环顶部 120ms 节流守卫（覆盖 TUI journal 双 50ms 阶段的最坏 ~100ms 对齐 + gate/工具轮两路径 + turn≥2 首轮——守卫置 0 时 TUI E2E 确定性失败 `["X+X","X"]`）；stdio 线自动序列化 session/update 通知帧（stdio e2e 新增断言）；`--fake-provider` 演示 250ms 分块可见流式；TUI 生产代码 +2 行（terminal 事件重置流式追加目标，审查 P3-1 修复）。提交前审查闭合（两独立代理，无 P1）：P2×2（守卫 60→120ms 双阶段对齐；docs 措辞/计数修正）+ P3×4 修复（跨轮 append 目标残留、turn≥2 首轮竞态、README 自相矛盾、headless sleep 注释）。附存修复：`find_git_bash` 增强（GitForWindows 注册表 InstallPath + PATH 含 git 条目父目录探测——非标准安装位置如 B 盘 Git 可发现）+ 5 个 POSIX 语义测试 Git Bash 后端门控（PowerShell 后端显式 skip，d48b724 平台感知先例；`GROK_SHELL=bash` 下真实执行验证通过）。全仓 0 failed、clippy 0 error 新代码零新增警告。
Slice #5 已提交推送（orz `3f116d0` → `cli/feat/fusion-architecture`，主仓库 `a04d1eb` → `origin/main`，用户手动推送）——orz-tui：进程内 ACP 双工工作台（冻结布局/ContentPane 消息卡+工具行/状态栏/简化 Marker）、50ms journal tail 全事件投影、权限对话框交互解锁（现有 gateway 路径，无 permission.rs 改动）、replay 模式链校验、裸 `orz`→TUI（+`--replay`/`--run-root`/`--fake-provider`）；提交前审查闭合（P1×3：矩形卡、run-id bootstrap 前预留、运行中守卫+精确去重；P2×6：ANSI 净化、EOF 退出、权限队列、兜底扫描、require_terminal、teardown 恒执行）；全仓 0 failed、clippy 触碰文件 0 warning。
Slice #4 已提交推送（orz `58df321` → `cli/feat/fusion-architecture`，主仓库 `ce7ec83` → `origin/main`）——IP5：mutation 工具执行前快照（`snapshot_created` 事件，run-event schema 30→31）；permit：DPAPI keystore（Python 同格式，交叉兼容实测）注入 host `issue_permit`；全仓 0 failed、check_repository valid。
Slice #3 已提交推送（orz `fc8e213` → `cli/feat/fusion-architecture`，主仓库 `3190d31` → `origin/main`）；存量修复：xai-fast-worktree 4 个 metadata-feature 测试 Windows 平台语义（stash 验证为既有问题，非切片回归）。
主仓库 CI 4 作业（ubuntu/windows × 3.11/3.12）全绿；workspace trust 扫描已剪枝（跳过 .git/嵌套仓库，本地 doctor exit 0）。

Python assurance spec（`assurance/`）保留为 conformance suite、schema authority 与离线验证路径。

## 产品定位

- **自研二进制**：orz 是独立 Rust 二进制，assurance 编译进二进制内部，不是外部 sidecar/wrapper。
- **Grok 底座**：Grok Build (xai-org/grok-build, commit `500129c7`) 提供 tools/workspace/sandbox/mcp/chat-state/hooks/acp-lib 等成熟组件——保留但不修改其核心逻辑。
- **Codex 纪律**：薄核心（orz-host ~8k）、单向依赖（Grok crate 不反向依赖自研 crate）、零死代码（telemetry/marketplace/announcements/voice/mermaid 已删）、严格 Clippy。
- **核心差异**：Assurance 层 (orz-assurance) — hash-chained JSONL journal、instruction provenance gate、orientation checkpoint、工具可用性门禁、workspace trust、shadow Git snapshot。
- **多源借鉴**：Grok（底座）、Codex CLI（架构纪律）、Gemini CLI（invariant checker）、OpenCode（snapshot）、Goose（SecurityFinding/PermissionDecision 分离）。

## 上游策略

- **上游策略**：Grok Build 锁定 `500129c7`，本地 fork 不可向上游提交——Grok 不接受社区 PR。三轨制（historical/current/selected）版本策略见 `UPSTREAM_VERSION_STRATEGY_v0.1.md`。
- **研究边界**：旧研究工作区的 INDEX/MAP/self-check 不迁入本仓库。

完整裁决见 ADR 与架构文档。

## 融合架构文档 (权威)

- [`architecture/INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.2.md`](architecture/INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.2.md)：**当前权威设计** — 融合架构（Codex 纪律 + Grok 能力）、crate 矩阵、orz-host 设计、4-Phase 实施计划。
- [`architecture/INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.1.md`](architecture/INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.1.md)：已被 v0.2 取代。保留 LoopHost trait 和注入点分类参考。
- [`architecture/FORK_ARCHITECTURE_AND_DESIGN_LANGUAGE_v0.3.md`](architecture/FORK_ARCHITECTURE_AND_DESIGN_LANGUAGE_v0.3.md)：五源融合设计语言（Codex/Gemini/OpenCode/Goose + Grok）。
- [`architecture/AGENT_LOOP_REDESIGN_v0.1.md`](architecture/AGENT_LOOP_REDESIGN_v0.1.md)：Pro/Flash 双 Agent + Blackboard + MechanicalRelay + 检索子代理设计。
- [`architecture/PHASE2_POST_PLANA_REVIEW_v0.1.md`](architecture/PHASE2_POST_PLANA_REVIEW_v0.1.md)：Plan A 编译修复后审查 — 4 个伪代码 bug 已全部修正。

## 实施 (Rust)

- **仓库**：`D:\CLI\orz`（本地 fork of Grok Build `500129c7`；2026-08-04 由 `B:\orz` 迁移，B 盘空间不足）
- **分支**：`feat/fusion-architecture`（70 crate workspace, `cargo check` 绿色）
- **设计符合性**：详见[审查报告](#) — v0.2 设计与实施高度一致
- **跟踪项**：`memory/fusion-phase-tracking.md` — Phase 1/2 完成，**Phase 3 Slice #1-13 完成**（自研 444 tests：assurance 96 / loop 71 / host 73 / tui 174 / codex 28 / bin 2；**clippy 全仓零警告**）；Slice #12 = **Codex 兜底 TUI**（§2.4 双 TUI 策略兜底侧：orz-host Codex app-server JSON-RPC 面 + 独立二进制 `orz-codex`——用户裁决：独立二进制名、默认不启用、互斥显示；审计记录 `docs/CODEX_FALLBACK_TUI_SLICE_12_2026-08-06.md`）；**Slice #13 = 严格 Clippy 零警告 + 死代码裁剪**（§Phase 3 item 4：orz-telemetry 8 死模块 + orz-config-types 3 死模块删、4+3+1 依赖裁、canonicalize→dunce、clippy 128→0；orz-announcements 审计 LIVE 保留；16 产品不可达 crate 为设计保留不删）；剩余：**Python reference-spec**、conformance suite 验证（含 payload schema good/bad fixtures 补齐）、reasoning_content 回放缺口（live 验证后 fork 补丁）、Codex 面扩展（未排期）

## 当前文件 (Python assurance spec + 架构 + 协议)

- [`adr/ADR-0001-evidence-constrained-local-agent-kernel.md`](adr/ADR-0001-evidence-constrained-local-agent-kernel.md)：产品边界、核心不变量、案例库和反捷径决策。
- [`adr/ADR-0003-runtime-neutral-assurance-kernel.md`](adr/ADR-0003-runtime-neutral-assurance-kernel.md)：冻结 runtime-neutral 所有权、capability-gated 选择与 Grok `reference_only` 边界。
- [`adr/ADR-0004-general-science-profile-layering.md`](adr/ADR-0004-general-science-profile-layering.md)：冻结 `general-science → lif-research` 的只增不减分层和通用测试不得使用 LIF 内部任务的隔离边界。
- [`assurance/README.md`](assurance/README.md)：P0–P5 runtime-neutral 合同与 development fixture，包括会话生命周期、workspace-first/Docker backend policy、guarded execution、指令来源、能力子集、metadata audit、恢复授权和合成用户任务预检。
- [`assurance/source-visibility-ledger-v0.1.schema.json`](assurance/source-visibility-ledger-v0.1.schema.json)：外部文献、帖子和网页引用后的全文可见性 ledger；配套 gate receipt 会按 claim 类型要求 metadata/partial/full-text 状态并输出 allow/defer/block。
- [`assurance/canonical_cli.py`](assurance/canonical_cli.py)：canonical guarded CLI 的 offline/fake 纵向路径；串联 run manifest、instruction provenance gate、tool availability gate、orientation checkpoint、source visibility gate、fake adapter boundary、answer packet、runtime journal 和独立 verifier。
- [`gsa.py`](gsa.py)：P0.5 顶层 CLI dispatcher；提供 `doctor`、`source gate`、`run` 和 `verify`。
- [`assurance/task-contract-v0.1.schema.json`](assurance/task-contract-v0.1.schema.json)：冻结单次 CLI run 的任务合同；记录原始问题、source ledger、权限、claim 上限和输出要求。
- [`assurance/instruction_provenance_gate.py`](assurance/instruction_provenance_gate.py)：入口级指令来源/反注入门禁（GAK-INJ-001 offline）；在模型调用前对多来源指令做 batch 分类与 block/defer/allow。
- [`assurance/tool_availability_gate.py`](assurance/tool_availability_gate.py)：工具可用性机械探测与 gate receipt；生成必须注入模型上下文的 AVAILABLE/UNAVAILABLE 列表，并检测工具信念错配/停滞。
- [`assurance/retrieval_subagent.py`](assurance/retrieval_subagent.py)：检索子代理 no-model fixture；冻结委托合同、结构化结果、来源可见性校验与关闭不清零协议。
- [`assurance/windows_sandbox.py`](assurance/windows_sandbox.py)：Windows native AppContainer + Job Object 探针与 candidate builder；live probe 通过前 selection 保持 noncompliant。
- [`docs/CANONICAL_CLI_QUICKSTART_2026-07-26.md`](docs/CANONICAL_CLI_QUICKSTART_2026-07-26.md)：当前 offline/fake CLI 入口的最短使用说明。
- [`docs/CANONICAL_GUARDED_CLI_P0_AUDIT_2026-07-26.md`](docs/CANONICAL_GUARDED_CLI_P0_AUDIT_2026-07-26.md)：首个 canonical guarded CLI offline 主路径审计；确认 gate-before-model、fake no-network adapter、answer packet 与 verifier 边界。
- [`docs/GAK_INJ_001_AUDIT_2026-07-27.md`](docs/GAK_INJ_001_AUDIT_2026-07-27.md)：instruction provenance gate offline 切片与 canonical CLI 接入状态。
- [`docs/TOOL_AVAILABILITY_GATE_AUDIT_2026-07-27.md`](docs/TOOL_AVAILABILITY_GATE_AUDIT_2026-07-27.md)：工具可用性门禁、belief mismatch/stagnation 与 orientation 注入审计。
- [`docs/RETRIEVAL_SUBAGENT_AUDIT_2026-07-27.md`](docs/RETRIEVAL_SUBAGENT_AUDIT_2026-07-27.md)：检索子代理合同/结果/关闭协议 no-model 审计。
- [`docs/GAK_SBX_001_WINDOWS_NATIVE_SANDBOX_AUDIT_2026-07-27.md`](docs/GAK_SBX_001_WINDOWS_NATIVE_SANDBOX_AUDIT_2026-07-27.md)：Windows native sandbox 合同与 verifier；live probe 仍为关闭条件。
- [`docs/GSA_CORE_READONLY_SLICE_AUDIT_2026-07-25.md`](docs/GSA_CORE_READONLY_SLICE_AUDIT_2026-07-25.md)：首个非 LIF 通用科学多文件只读闭环、claim 强度门禁、正反回归与保留缺口。
- [`docs/GSA_VALIDATOR_BRIDGE_AUDIT_2026-07-25.md`](docs/GSA_VALIDATOR_BRIDGE_AUDIT_2026-07-25.md)：版本化 validator registry、设计/动作 schema、artifact finite gate 与基础统计报告完整性边界。
- [`docs/GSA_ARTIFACT_SCHEMA_REGISTRATION_AUDIT_2026-07-25.md`](docs/GSA_ARTIFACT_SCHEMA_REGISTRATION_AUDIT_2026-07-25.md)：artifact schema registry、结果摘要绑定、不可弱化映射与首个数值/统计基础 schema。
- [`docs/GSA_CROSS_ARTIFACT_LINEAGE_COMPARABILITY_AUDIT_2026-07-25.md`](docs/GSA_CROSS_ARTIFACT_LINEAGE_COMPARABILITY_AUDIT_2026-07-25.md)：跨 artifact JSON Pointer、producer/source lineage、比较不变量和 permission-reversal 门禁。
- [`docs/GSA_DISPOSABLE_REPRODUCTION_AUDIT_2026-07-26.md`](docs/GSA_DISPOSABLE_REPRODUCTION_AUDIT_2026-07-26.md)：首个 no-model disposable reproduction manifest、固定 replay、receipt/verifier/run-proof/runtime projection、一次性 runtime journal write/replay、execution lock、no-model runner skeleton、runner journal lock/recovery repair receipt、lifecycle repair policy 与“不增加独立证据”边界。
- [`docs/SOURCE_FULLTEXT_VISIBILITY_RULE_2026-07-26.md`](docs/SOURCE_FULLTEXT_VISIBILITY_RULE_2026-07-26.md)：文献、帖子、网页和 thread 检索时必须回报全文可见性状态；未完整抓取并浏览全文时明确标注 partial/metadata-only/unavailable，禁止摘要或片段冒充全文证据。
- [`docs/GSA_SELF_QUESTION_COUNTEREXAMPLE_DESIGN_2026-07-26.md`](docs/GSA_SELF_QUESTION_COUNTEREXAMPLE_DESIGN_2026-07-26.md)：中性 orientation checkpoint、独立 counterexample queue，以及 DeepSeek/Grok 等 runtime 高重复输出截断与重启的设计和首个只读 fixture。
- [`assurance/orientation_runtime_guard.py`](assurance/orientation_runtime_guard.py)：生成中性 orientation checkpoint、验证其不产生反例/claim disposition，并用公开输出重复阈值生成 runtime stagnation guard receipt。
- [`assurance/orientation_runtime_integration.py`](assurance/orientation_runtime_integration.py)：no-model 接入前置夹具；从 fixture input 写出 orientation/stagnation 两个 artifact 和汇总 receipt，并由 verifier 独立重建。
- [`assurance/orientation_runtime_journal.py`](assurance/orientation_runtime_journal.py)：no-model runtime event/journal projection；生成 `orientation_checkpoint` 与 `runtime_stagnation_guard` 事件并验证 JSONL hash-chain。
- [`assurance/runner_public_output.py`](assurance/runner_public_output.py)：真实 runner 前的公开输出抽取夹具；只把 public assistant 输出送入 `public_outputs`，私有 reasoning/redacted metadata 只能保留 digest 且不得复制文本。
- [`assurance/deepseek_api_observation.py`](assurance/deepseek_api_observation.py)：direct DeepSeek API one-shot 观测投影；消费单次受控 API result，把公开 assistant 输出接入 extraction、orientation/stagnation 和 runtime journal，不保存 credential、raw response 或隐藏 reasoning 文本。
- [`assurance/deepseek_stream_observation.py`](assurance/deepseek_stream_observation.py)：DeepSeek-shaped streaming fixture；把 public `delta.content`、private reasoning digest 和 terminal metadata 投影到 runner output stream，并用重复 public delta 验证 restart projection。
- [`scripts/invoke_deepseek_public_output_observation.ps1`](scripts/invoke_deepseek_public_output_observation.ps1)：两阶段真实 DeepSeek API 观测 launcher；先生成离线 plan，执行时从 Windows Credential Manager `FEP-Agent/DeepSeek` 读取凭据并只发起一次固定 marker 请求。
- [`docs/P2_DOCKER_SANDBOX_AUDIT_2026-07-24.md`](docs/P2_DOCKER_SANDBOX_AUDIT_2026-07-24.md)：P2 Docker 实测、Windows native fail-closed 状态、限制与正反例收敛策略。
- [`docs/P2_5_GUARDED_EXECUTION_AUDIT_2026-07-24.md`](docs/P2_5_GUARDED_EXECUTION_AUDIT_2026-07-24.md)：P1 envelope、P2 selector、无模型 action、宿主/容器进程追踪、HMAC 回执与残留复核的端到端实测。
- [`docs/P3_INSTRUCTION_AUTHORITY_AUDIT_2026-07-25.md`](docs/P3_INSTRUCTION_AUTHORITY_AUDIT_2026-07-25.md)：P3 来源不可提权、内核动作授权、能力子集委派与 digest-bound 一次性许可的测试和限制。
- [`docs/P4_AUDIT_COMPACTION_RECOVERY_AUDIT_2026-07-25.md`](docs/P4_AUDIT_COMPACTION_RECOVERY_AUDIT_2026-07-25.md)：P4 runtime-neutral metadata ledger、归档后复核、compaction provenance 与恢复候选/授权分离。
- [`docs/P4_5_WORKSPACE_FIRST_INTEGRATION_AUDIT_2026-07-25.md`](docs/P4_5_WORKSPACE_FIRST_INTEGRATION_AUDIT_2026-07-25.md)：P4.5 标准模式 workspace-first 策略、P3→固定动作→P4→archive 实测与无 Docker/子进程残留复核。
- [`docs/P5_SYNTHETIC_USER_TASK_PREFLIGHT_2026-07-25.md`](docs/P5_SYNTHETIC_USER_TASK_PREFLIGHT_2026-07-25.md)：P5 两项合成任务、R211 复杂任务只读投影、LIF 源文件不变证明及人类可用性未评估边界。
- [`docs/GENERAL_SCIENTIFIC_ASSURANCE_GAP_REVIEW_2026-07-25.md`](docs/GENERAL_SCIENTIFIC_ASSURANCE_GAP_REVIEW_2026-07-25.md)：将本轮缺口登记为通用科学保障 backlog，并明确 LIF 内部任务不得作为通用复杂测试或 holdout。
- [`architecture/PRODUCT_POSITIONING_AND_REFERENCE_STRATEGY_v0.1.md`](architecture/PRODUCT_POSITIONING_AND_REFERENCE_STRATEGY_v0.1.md)：多源借鉴分工、通用科学保障与 LIF 领域增量分层，以及未来 LIF-informed Agent 想法的隔离边界。
- [`architecture/GROK_BUILD_ADAPTATION_v0.1.md`](architecture/GROK_BUILD_ADAPTATION_v0.1.md)：基于官方开源快照的 adopt/adapt/defer/reject 矩阵与项目专化层。
- [`architecture/UPSTREAM_FIRST_INTEGRATION_v0.1.md`](architecture/UPSTREAM_FIRST_INTEGRATION_v0.1.md)：Grok reference adapter 的 Windows 集成范围，以及现有 prototype 的降级分类。
- [`architecture/UPSTREAM_VERSION_STRATEGY_v0.1.md`](architecture/UPSTREAM_VERSION_STRATEGY_v0.1.md)：将可复现实测 baseline 与当前上游 candidate 分离，以 conformance gate 选择更优版本而非永久锁死。
- [`architecture/OBSERVABLE_EXECUTION_ENVELOPE_v0.1.md`](architecture/OBSERVABLE_EXECUTION_ENVELOPE_v0.1.md)：优先准确性的全程可观测 wrapper、记录分层、资源取舍与反补全约束。
- [`architecture/MATURE_AGENT_DESIGN_DECOMPOSITION_v0.2.md`](architecture/MATURE_AGENT_DESIGN_DECOMPOSITION_v0.2.md)：对 Grok ACP、Codex、Gemini CLI、OpenCode、Goose 与 Cline 的职责深拆；把通用 runtime 交给可替换的外部框架，只保留保障层与 shadow-Git 恢复边界。
- [`architecture/GENERAL_ASSURANCE_KERNEL_GAP_REGISTER_v0.1.md`](architecture/GENERAL_ASSURANCE_KERNEL_GAP_REGISTER_v0.1.md)：将能力拆为通用 Assurance Kernel、`general-science` 与 LIF 领域 profile，登记安全、审计和用户测试缺口。
- [`architecture/GLOBAL_PROGRESS_SENTINEL_CONTRACT_v0.1.md`](architecture/GLOBAL_PROGRESS_SENTINEL_CONTRACT_v0.1.md)：从 task contract、计划和 append-only journal 派生全局进度摘要，并以跨检查点方向预算、证据停滞、延期债务和整体完成门降低单方向过推进与局部完成误判风险。
- [`architecture/D_SALVAGE_MATRIX_v0.1.md`](architecture/D_SALVAGE_MATRIX_v0.1.md)：Google Drive 中 Project D 核心源码的 source ledger、采用/改造/拒绝裁决与安全发现。
- [`adr/ADR-0002-defer-cloud-runtime.md`](adr/ADR-0002-defer-cloud-runtime.md)：冻结云端执行/运维范围，保持 Windows 本地 runtime，并记录未来重启条件。
- [`integration/grok/grok-observed-plan-v0.1.schema.json`](integration/grok/grok-observed-plan-v0.1.schema.json)：不执行模型的 observed dry-run 计划格式；配套 PowerShell 生成器只冻结 provenance 与控制意图。
- [`integration/grok/grok-fake-provider-result-v0.1.schema.json`](integration/grok/grok-fake-provider-result-v0.1.schema.json)：Windows 临时防火墙、隔离环境、loopback 请求捕获与 credential-leak 检查的 conformance 结果格式。
- [`upstream/grok-build.lock.json`](upstream/grok-build.lock.json)：当前上游 build commit、`SOURCE_REV`、README digest 与职责所有权锁。
- [`integration/grok/deepseek-custom-model.example.toml`](integration/grok/deepseek-custom-model.example.toml)：不含凭据、不设默认模型的 DeepSeek discovery-only custom-model 模板。
- [`docs/GROK_WINDOWS_BINARY_AUDIT_2026-07-21.md`](docs/GROK_WINDOWS_BINARY_AUDIT_2026-07-21.md)：官方 stable Windows binary 的体积、hash、签名、离线能力和未证明项审计。
- [`docs/GROK_FAKE_PROVIDER_AUDIT_2026-07-21.md`](docs/GROK_FAKE_PROVIDER_AUDIT_2026-07-21.md)：loopback 请求、临时防火墙、debug credential 泄漏发现、修复与 post-fix artifact ledger。
- [`docs/GROK_TOOL_CONTINUITY_AUDIT_2026-07-21.md`](docs/GROK_TOOL_CONTINUITY_AUDIT_2026-07-21.md)：两轮 `read_file`、DeepSeek reasoning continuity、Job Object 与 streaming-json 工具事件缺口审计。
- [`docs/GROK_ACP_INITIALIZE_AUDIT_2026-07-21.md`](docs/GROK_ACP_INITIALIZE_AUDIT_2026-07-21.md)：锁定 Windows binary 的 ACP protocol v1/capability/扩展通知无模型实测、独立验证与下一步 fake-tool 边界。
- [`docs/GROK_ACP_FAKE_TOOL_SPIKE_2026-07-23.md`](docs/GROK_ACP_FAKE_TOOL_SPIKE_2026-07-23.md)：ACP fake tool allow/cancel 双场景的 Windows 实测、三路证据对账、取消竞态与独立 verifier。
- [`docs/GROK_UPSTREAM_CANDIDATE_AUDIT_2026-07-23.md`](docs/GROK_UPSTREAM_CANDIDATE_AUDIT_2026-07-23.md)：Grok `0.2.111` 的官方发现、本地身份、ACP/DeepSeek fake-only 对照与已完成的 promotion gate。
- [`docs/DEEPSEEK_REAL_DEVELOPMENT_PROBE_2026-07-23.md`](docs/DEEPSEEK_REAL_DEVELOPMENT_PROBE_2026-07-23.md)：一次性真实 DeepSeek probe 的两阶段入口、离线测试、首个 real transport attempt 与 fail-closed 结果。
- [`docs/DEEPSEEK_CREDENTIAL_HARDENING_2026-07-23.md`](docs/DEEPSEEK_CREDENTIAL_HARDENING_2026-07-23.md)：WER `NOHEAP`、短进程、固定 endpoint、无 proxy/redirect/debug、失败 artifact 与无真实 key 模式扫描的安全边界。
- [`docs/GROK_REAL_DEEPSEEK_LAUNCHER_2026-07-23.md`](docs/GROK_REAL_DEEPSEEK_LAUNCHER_2026-07-23.md)：Grok Build 两阶段真实 DeepSeek 薄 launcher、Credential Manager 内部注入、Job Object、双 trust receipt 与离线 fail-closed 证据。
- [`docs/GROK_UPSTREAM_PROMOTION_2026-07-23.md`](docs/GROK_UPSTREAM_PROMOTION_2026-07-23.md)：将默认 Grok 从 `0.2.106` 提升到 `0.2.111` 的独立裁决与回退锚点。
- [`docs/GROK_WINDOWS_CHILD_TREE_PROBE_2026-07-23.md`](docs/GROK_WINDOWS_CHILD_TREE_PROBE_2026-07-23.md)：`run_terminal_command` timeout、background task cancel 与 parent-exit 三场景的 Windows 子进程树门禁及已完成的管理员 observed 矩阵。
- [`docs/GROK_COMPACTION_PROVENANCE_2026-07-23.md`](docs/GROK_COMPACTION_PROVENANCE_2026-07-23.md)：Grok `0.2.111` 手工 compaction 的 `PreCompact`/`PostCompact`、source span/digest、checkpoint、`derived_unverified` 摘要边界与独立 verifier。
- [`architecture/DEEPSEEK_ADAPTER_CONTRACT_v0.1.md`](architecture/DEEPSEEK_ADAPTER_CONTRACT_v0.1.md)：基于 DeepSeek 官方 API 文档的专项 adapter、thinking/tool-call、兼容层和重试边界。
- [`architecture/WINDOWS_RUNTIME_CONTRACT_v0.1.md`](architecture/WINDOWS_RUNTIME_CONTRACT_v0.1.md)：Windows-first Job Object、进程树、取消、输出 drain 与 terminal-event 契约。
- [`architecture/ACTION_KERNEL_CONTRACT_v0.1.md`](architecture/ACTION_KERNEL_CONTRACT_v0.1.md)：将 manifest、journal、Windows runner、artifact 与 session 串联的 no-model action-kernel spike。
- [`architecture/MODEL_ADAPTER_LOOP_CONTRACT_v0.1.md`](architecture/MODEL_ADAPTER_LOOP_CONTRACT_v0.1.md)：provider-neutral scripted transport、DeepSeek streaming/tool loop 与 private reasoning 落盘边界。
- [`architecture/LOOPBACK_TRANSPORT_AND_PRIVATE_TRANSCRIPT_v0.1.md`](architecture/LOOPBACK_TRANSPORT_AND_PRIVATE_TRANSCRIPT_v0.1.md)：仅 127.0.0.1 的 HTTP/SSE transport、取消/timeout 与 Windows DPAPI transcript 恢复边界。
- [`architecture/DEEPSEEK_EXTERNAL_TRANSPORT_SAFETY_v0.1.md`](architecture/DEEPSEEK_EXTERNAL_TRANSPORT_SAFETY_v0.1.md)：固定 DeepSeek HTTPS endpoint、一次性请求许可、Windows Credential Manager 与离线 fake-TLS 验证边界。
- [`architecture/NETWORK_PERMIT_BROKER_CONTRACT_v0.1.md`](architecture/NETWORK_PERMIT_BROKER_CONTRACT_v0.1.md)：每轮/每次 retry 的独立 permit、脱敏确认摘要与三-attempt fake HTTPS 模型循环。
- [`architecture/INTERACTIVE_APPROVAL_LEDGER_CONTRACT_v0.1.md`](architecture/INTERACTIVE_APPROVAL_LEDGER_CONTRACT_v0.1.md)：fake-only 逐 attempt digest 确认、append-only hash-chain ledger 与跨文件 verifier。
- [`protocol/PROTOCOL_DRAFT_v0.1.md`](protocol/PROTOCOL_DRAFT_v0.1.md)：协议语义、状态机、事件和独立性流程。
- [`protocol/agent-protocol-v0.1.schema.json`](protocol/agent-protocol-v0.1.schema.json)：语言无关的初步 JSON Schema。
- [`protocol/reason-codes-v0.1.yaml`](protocol/reason-codes-v0.1.yaml)：原因码语义、默认决策、误报保护和补救动作注册表。
- [`protocol/gate-matrix-v0.1.yaml`](protocol/gate-matrix-v0.1.yaml)：discussion/guarded/strict 三档门禁和阶段依赖。
- [`regression/case-corpus-v0.1.schema.json`](regression/case-corpus-v0.1.schema.json)：案例结构、状态词表和 historical/synthetic 分区约束。
- [`regression/fixture-v0.1.schema.json`](regression/fixture-v0.1.schema.json)：oracle-free 机器可读 fixture manifest 契约。
- [`regression/historical-excerpt-provenance-v0.1.schema.json`](regression/historical-excerpt-provenance-v0.1.schema.json)：reviewer-only 历史摘录的源文件、行段、归一化和 digest 契约。
- [`regression/fixtures/`](regression/fixtures/)：5 份 scenario-visible synthetic evidence packs，以及 5 份 reviewer-only 历史来源摘录；后者含纠正结论，禁止进入被测 Agent 上下文。
- [`regression/cases-v0.1.yaml`](regression/cases-v0.1.yaml)：30 个历史回归案例与 10 个合成反惯性案例。
- [`regression/CURATION_PROTOCOL_v0.1.md`](regression/CURATION_PROTOCOL_v0.1.md)：自动发现、人工归类、oracle 隔离与反惯性评测协议。
- [`regression/coverage-matrix-v0.1.yaml`](regression/coverage-matrix-v0.1.yaml)：10 个错误簇、challenge 类型、覆盖缺口与 holdout 前优先级。
- [`regression/coverage-matrix-v0.1.schema.json`](regression/coverage-matrix-v0.1.schema.json)：覆盖矩阵的机器约束。
- [`evaluation/SCORING_PROTOCOL_v0.1.md`](evaluation/SCORING_PROTOCOL_v0.1.md)：cluster-macro、多维评分、红线、盲审和阈值校准协议。
- [`evaluation/evaluation-result-v0.1.schema.json`](evaluation/evaluation-result-v0.1.schema.json)：不含单一总分的评测结果格式。
- [`evaluation/example-evaluation-result-v0.1.json`](evaluation/example-evaluation-result-v0.1.json)：仅用于 schema 自证的非运行示例。
- [`evaluation/PARTITION_AND_ORACLE_ISOLATION_v0.1.md`](evaluation/PARTITION_AND_ORACLE_ISOLATION_v0.1.md)：evaluation/holdout 的物理隔离、角色分离、污染和生命周期协议。
- [`evaluation/partition-manifest-v0.1.schema.json`](evaluation/partition-manifest-v0.1.schema.json)：密封分区 digest、状态和暴露策略契约；当前没有真实实例。
- [`evaluation/HUMAN_BASELINE_PROTOCOL_v0.1.md`](evaluation/HUMAN_BASELINE_PROTOCOL_v0.1.md)：双人独立盲审、adjudication 和阈值校准协议。
- [`runtime/SCENARIO_EXPORT_AND_RUNNER_CONTRACT_v0.1.md`](runtime/SCENARIO_EXPORT_AND_RUNNER_CONTRACT_v0.1.md)：scenario 裁剪、匿名化、leak gate、model adapter、tool broker 与可重放 runner 契约。
- [`runtime/scenario-export-manifest-v0.1.schema.json`](runtime/scenario-export-manifest-v0.1.schema.json)：场景包、opaque token、文件 digest 和导出策略 schema。
- [`runtime/scenario-instance-v0.1.schema.json`](runtime/scenario-instance-v0.1.schema.json)：被测 Agent 可见的最小场景实例 schema；排除 oracle、内部 case ID 和来源路径。
- [`runtime/leak-scan-report-v0.1.schema.json`](runtime/leak-scan-report-v0.1.schema.json)：机械/语义泄漏检查、人工签发与 fail-closed 决策 schema。
- [`runtime/run-manifest-v0.1.schema.json`](runtime/run-manifest-v0.1.schema.json)：不可变运行前置条件 schema。
- [`runtime/run-event-v0.1.schema.json`](runtime/run-event-v0.1.schema.json)：append-only、hash-chained JSONL event schema。
- [`runtime/global-progress-review-v0.1.schema.json`](runtime/global-progress-review-v0.1.schema.json)：由 task contract、计划和 journal 机械派生的全局进度、方向覆盖、验收覆盖与三类 WARN。
- [`runtime/global-progress-disposition-v0.1.schema.json`](runtime/global-progress-disposition-v0.1.schema.json)：模型对每条全局 WARN 的显式处置，以及 `continue/pivot/defer/stop/replan` 决策边界。
- [`runtime/fixtures/global-progress-sentinel-v0.1/`](runtime/fixtures/global-progress-sentinel-v0.1/)：六方向 no-model 输入与 `reasoned_continue`/`replan` 两种合法处置 fixture。
- [`runtime/global-progress-history-input-v0.1.schema.json`](runtime/global-progress-history-input-v0.1.schema.json)：跨检查点 hash-chain 投影，记录已观测方向动作数、证据新颖性、关键延期和整体验收状态；不估算隐藏 token/努力。
- [`runtime/global-progress-holistic-review-v0.1.schema.json`](runtime/global-progress-holistic-review-v0.1.schema.json)：方向预算占比、同方向空转、关键延期债务和整体完成资格的确定性派生视图。
- [`runtime/global-progress-holistic-disposition-v0.1.schema.json`](runtime/global-progress-holistic-disposition-v0.1.schema.json)：集中推进的有界许可，要求关键路径、退出条件、追加动作上限、下一复查周期和受影响关键方向。
- [`runtime/fixtures/global-progress-holistic-v0.1/`](runtime/fixtures/global-progress-holistic-v0.1/)：四检查点 no-model 历史，覆盖四种整体性失效模式及有界聚焦 permission reversal。
- [`runtime/global-progress-checkpoint-policy-v0.1.schema.json`](runtime/global-progress-checkpoint-policy-v0.1.schema.json)：冻结 task/contract 绑定、critical direction 与 acceptance 映射，防止跨周期静默缩小整体范围。
- [`scripts/build_global_progress_checkpoint.py`](scripts/build_global_progress_checkpoint.py)：从 GPS input/review/disposition/verification、policy 和可选 focus artifact 链机械派生 checkpoint，并对动作上限/复查期限超限返回阻断状态。
- [`scripts/verify_global_progress_checkpoint.py`](scripts/verify_global_progress_checkpoint.py)：重新运行上游独立 verifier、重建 checkpoint、检查 hash/provenance/policy continuity 与 focus 消耗。
- [`scripts/build_global_progress_transition_event.py`](scripts/build_global_progress_transition_event.py)：将 checkpoint verification 投影为 hash-chained `gate_decision` run event；阻断时保持原状态并保留 receipt。
- [`scripts/append_global_progress_transition_event.py`](scripts/append_global_progress_transition_event.py)：在共享跨进程锁内，只把能精确延伸 run/manifest/sequence/hash chain 的 transition event 追加到 JSONL journal，并在追加后重放。
- [`scripts/recover_torn_journal.py`](scripts/recover_torn_journal.py)：默认只读检查 journal；仅在显式 `--apply`、有效前缀可独立重放且损坏限于无换行尾部时执行锁内恢复，并生成 quarantine 与恢复收据。
- [`scripts/run_global_progress_controller.py`](scripts/run_global_progress_controller.py)：no-model disposable controller；组合 transition builder、幂等锁内 append、journal 恢复分类与状态投影，但不自动修复 journal。
- [`scripts/verify_global_progress_controller_receipt.py`](scripts/verify_global_progress_controller_receipt.py)：只读重建 controller 候选结果，并交叉检查 source digests、当前 journal 快照、事件唯一性与状态投影。
- [`scripts/reduce_global_progress_state.py`](scripts/reduce_global_progress_state.py)：在共享锁内从完整 journal 归约 controller 状态；`run_started` 唯一锚定 `executing`，后续 transition 必须保持 task/state 连续，并输出最近一次 gate-accepted checkpoint binding。
- [`scripts/append_cli_session_lifecycle_event.py`](scripts/append_cli_session_lifecycle_event.py)：有限、厂商无关的 CLI lifecycle ingress；把已标准化且有序的 session/turn/terminal observation 原子映射为 canonical run event，并复用 GPS journal-derived 完成态。
- [`scripts/verify_cli_session_lifecycle_receipt.py`](scripts/verify_cli_session_lifecycle_receipt.py)：只读回放 observation、adapter receipt 与当前 journal，检查快照、事件唯一性和 lifecycle projection。
- [`runtime/cli-session-lifecycle-observation-v0.2.schema.json`](runtime/cli-session-lifecycle-observation-v0.2.schema.json)：区分 completed/interrupted/failed turn terminal，并把 turn terminal 与 session terminal 分离的标准化 observation。
- [`scripts/capture_codex_app_server_lifecycle.py`](scripts/capture_codex_app_server_lifecycle.py)：在 Windows Job Object/POSIX process group 内启动显式 app-server executable，只执行 initialize 与 ephemeral/read-only thread start，并原子输出双向 ordered capture、独立 stderr 和 digest-bound receipt。
- [`scripts/verify_codex_app_server_lifecycle_capture.py`](scripts/verify_codex_app_server_lifecycle_capture.py)：独立核对 capture/receipt/stderr、连续 sequence、thread 身份、containment 和 no-turn/no-input 边界。
- [`scripts/normalize_codex_app_server_lifecycle.py`](scripts/normalize_codex_app_server_lifecycle.py)：只读消费可信 supervisor 的单一双向有序 Codex app-server capture，用 turn/start request/response 证明 turn→thread 绑定，再把 thread/turn/close 通知投影为 lifecycle v0.2 observation；忽略正文类 item/delta，EOF 保持 partial。
- [`scripts/verify_codex_app_server_lifecycle.py`](scripts/verify_codex_app_server_lifecycle.py)：独立重放 capture，核对 source/observation digest、状态投影、内容省略和 receipt。
- [`runtime/fixtures/codex-app-server-lifecycle-v0.1/`](runtime/fixtures/codex-app-server-lifecycle-v0.1/)：happy、interrupted、failed、missing-terminal 与 sequence/thread/truncation 反例 capture。
- [`protocol/global-progress-reason-code-migration-v0.1.yaml`](protocol/global-progress-reason-code-migration-v0.1.yaml)：GPS runtime-local control code 到冻结 protocol registry 的设计期迁移登记；候选码不视为已注册 reason code。
- [`runtime/examples/`](runtime/examples/)：仅用于 schema 自证的零 digest 示例，不代表真实导出或运行。
- [`prototype/`](prototype/)：一次性 Python conformance fixtures；固定 LIF schema、DeepSeek thinking continuity、redaction 与 Windows 进程边界，不是待扩展的生产 runner。
- [`scripts/check_repository.py`](scripts/check_repository.py)：CI 使用的仓库级机械完整性检查；验证 schema、corpus、coverage、fixture digest、交叉引用和本地文档链接。
- [`scripts/invoke_grok_acp_fake_tool_probe.ps1`](scripts/invoke_grok_acp_fake_tool_probe.ps1)：管理员级 fake-only ACP tool/permission/cancel continuity launcher；synthetic/tamper tests 与 Windows live smoke 均已通过。
- [`scripts/invoke_grok_windows_child_tree_probe.ps1`](scripts/invoke_grok_windows_child_tree_probe.ps1)：管理员级 fake-only Windows root/child/grandchild containment launcher；显式接受 baseline/candidate release metadata，不自动提升默认版本。
- [`docs/REPOSITORY_AUDIT_2026-07-21.md`](docs/REPOSITORY_AUDIT_2026-07-21.md)：迁移后审计、初始提交边界和下一阶段 session-validator spike 范围。
- [`docs/GLOBAL_PROGRESS_SENTINEL_SPIKE_2026-07-21.md`](docs/GLOBAL_PROGRESS_SENTINEL_SPIKE_2026-07-21.md)：no-model 全局回看生成、独立重建、合法处置、去重与篡改失败的实测记录。
- [`docs/GLOBAL_PROGRESS_HOLISTIC_GATE_AUDIT_2026-07-25.md`](docs/GLOBAL_PROGRESS_HOLISTIC_GATE_AUDIT_2026-07-25.md)：跨检查点整体性门禁、正反处置、hash-chain 篡改与保留边界的实测记录。
- [`docs/GLOBAL_PROGRESS_CHECKPOINT_ADAPTER_AUDIT_2026-07-25.md`](docs/GLOBAL_PROGRESS_CHECKPOINT_ADAPTER_AUDIT_2026-07-25.md)：来源绑定 checkpoint adapter、上游 verifier 重算、policy continuity 与有界聚焦实际消费核算记录。
- [`docs/GLOBAL_PROGRESS_STATE_REDUCER_AUDIT_2026-07-25.md`](docs/GLOBAL_PROGRESS_STATE_REDUCER_AUDIT_2026-07-25.md)：journal-derived 状态权威、初态锚点、锁内连续性校验及伪造状态反例记录。
- [`docs/CLI_SESSION_LIFECYCLE_ADAPTER_AUDIT_2026-07-25.md`](docs/CLI_SESSION_LIFECYCLE_ADAPTER_AUDIT_2026-07-25.md)：有限成熟 CLI 接入面、生命周期映射、厂商 normalizer 边界及 receipt 独立复核记录。
- [`docs/CLI_LIFECYCLE_SOURCE_COMPARISON_AND_CODEX_DIRECTION_2026-07-25.md`](docs/CLI_LIFECYCLE_SOURCE_COMPARISON_AND_CODEX_DIRECTION_2026-07-25.md)：Codex、Gemini、Qwen 与 Copilot lifecycle surface 比较，以及首个 Codex app-server read-only normalizer 的冻结方向和离线实现。
- [`docs/CODEX_APP_SERVER_LIFECYCLE_NORMALIZER_AUDIT_2026-07-25.md`](docs/CODEX_APP_SERVER_LIFECYCLE_NORMALIZER_AUDIT_2026-07-25.md)：ordered capture、正文省略、partial/terminal 语义、独立 verifier 与 canonical adapter 端到端实测。

## 证据路由

迁移前的设计遵循旧研究工作区的优先顺序：

1. `LIF_CURRENT_INDEX.md`
2. `fep_env_research.md`
3. 当前 MAP
4. 对应 R/JSON/log/code
5. `self_check_protocol.md`

这些旧工作区入口不属于本独立仓库，也不需要迁入。普通 CLI 工程工作不得依赖它们；只有具体 LIF claim-bearing 任务才按需跨目录读取原文件，并在当次 source ledger 中记录绝对路径与内容 hash。本仓库中的设计摘要不能替代这些证据。本仓库是新设计提案，不是现有实验结论入口，也不替代 MAP、INDEX、R 文档或原始产物。

## 当前边界

- **orz 二进制**：自研 Rust 二进制，assurance 编译进内部。Grok Build 提供工具/workspace/沙箱/MCP/持久化/hooks 等成熟组件（~320k 行），但不保留其 agent loop（orz-shell 已删除）和 TUI（Grok TUI 已删除）。
- **Python 项目角色**：Python assurance spec（`assurance/`）保留为 conformance suite、schema authority、新 gate 快速原型，以及离线 journal 验证。不删除。
- **不决定全部使用 Rust**：只把 Rust 视为 Agent 二进制 + 保障组件的实现语言。Python 保留参考角色。
- 不把历史案例直接作为模型提示词答案。不自动修改 MAP/INDEX。
- D 的候选机制已完成定向源码 salvage。
- “让 LIF/FEP 原理参与 Agent 控制或规划”是单独的 deferred research concept。

## 当前设计冻结点

- 融合架构（v0.2）是当前权威设计：Grok 底座 ~320k 行 + 自研核心 ~20k 行（orz-host/orz-loop/orz-assurance/orz-tui）。
- Grok 提供者 crate（tools/workspace/sandbox/mcp/chat-state/hooks）不改代码——assurance 通过 orz-loop 的 ToolDispatcher 包装层 + IP6 权限注入实现。
- 删除清单已执行：orz-shell (364k)、Grok TUI (477k)、telemetry/marketplace/announcements/sampler/http/config-types (35k)。
- 自动分类器只能提出候选，不能自动写入 canonical label。机械错误可 block，证据不足通常 defer dependent claim。
- 用户授权可以解决权限问题，但不能把不独立、混淆或无来源的证据改标为合格。

## License

This project is licensed under the Apache License, Version 2.0. See [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE).
