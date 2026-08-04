# CLI_PROJECT_INDEX

**更新**: 2026-08-05 (**CI 全绿恢复 + 本地扫描性能修复**——① CI 4 个失败测试断言对齐 alpha fail-closed 语义（主仓库 `aad7b34`）：f86eb15 有意收紧——无/不完整 ACP permission probe → `unavailable (block)` 而非 degraded(warn)/defer，3 个陈旧断言更新（observer 两测试重命名 + decision/degraded→unavailable/gate_decision warn→block；cli_dispatcher observe-tools JSON block）；GAK-CRED-001 allowlist 行号漂移修复（canonical_cli.py 凭据读取 1090→1099，f86eb15 插入行移位，finally scrub 仍在 L1137）。② workspace trust 扫描剪枝（主仓库 `336a409`，生产代码）：`_iter_workspace_files` os.walk topdown 剪枝——跳过 `.git` 与含 `.git` 的嵌套仓库根（gitignored orz/ 19GB 不再哈希）；此前 canonical `gsa run`/`verify`/`doctor` 哈希 D:\CLI 全树本地卡死（CI 无 orz/ 未暴露）；establish/verify 共用同一 helper 保持 digest 一致；`check_repository` `NON_REPOSITORY_PARTS` 加 `orz`（嵌套仓库文档由其自身审查治理）。③ orz README 14 处失效链接修复（orz `c00147f` → `cli/feat/fusion-architecture`）：`bin/protoc`、xai-grok-pager docs、third_party Mermaid 栈——均为 fusion 清理删除路径；third_party/README.md 重写为 provenance stub（NOTICE 保留）。修复后：本地 `gsa doctor` exit 0、test_cli_dispatcher 39 tests 25s 全绿、主仓库 CI 4 作业全绿。此前: 2026-08-04 (融合架构 orz 工作区**全仓测试编译/运行问题清零**——`cargo test --workspace` 全绿约 6300 tests 0 failed、clippy 0 error；继承 crate 首次在 Windows 跑通。修复 46 文件 +481/-149，**已提交推送**：orz `d48b724` → `cli/feat/fusion-architecture`，主仓库 `d4ba987` → `origin/main`。分类：① 测试编译错误 ~44 个（7 crate）；② Windows 生产 bug：orz-config managed_text 目录句柄/文件索引身份（NTFS 目录 mtime 误报 ParentChanged）、orz-memory journal_mode 恢复 WAL pragma+busy_timeout、resolve_model_path `is_absolute()`→`has_root()`（`/etc/hosts` 权限防护落空）、exec_risk/shell_access 安全路径分类、envrc `/bin/bash` 硬编码 + WSL bash 劫持→Git Bash 全路径、xai-fast-worktree WorktreeDb `contains('/')`→双分隔符（工作树查找全 None）、daemonize PidFile 持锁句柄读写；③ 平台适配：POSIX/HOME 模拟测试 cfg(unix) 门控 ~30 个、python3 别名存根→python、LSP mock 脚本 CRLF 翻译修复、路径分隔符断言组件级；④ ripgrep 15.2.0 已装（winget，grep 工具运行时依赖）。此前: **Phase 3 Slice #2 完成并已提交推送——§4.6 问询触发三机制接线**（122 tests），已推送：orz `a7fca22` → `cli/feat/fusion-architecture`，主仓库 `0c87d5b`/`023b699` → `origin/main`。此前: **Phase 3 Slice #1 完成并已提交推送**——OrzHost + IP6 PermissionBridge 接线三入口 + 提交前审查全发现处理（109 tests），已推送：orz `c4d6876` → `cli/feat/fusion-architecture`，主仓库 `6310c8a` → `origin/main`。再此前: Phase 2 完成并提交推送（102 tests）——单主 Agent loop + 完整 gate 链 + ACP stdio + OrzHost 真实工具 + IP6 permission 桥 + Plan Mode；已推送：orz `5ab590f` → `cli/feat/fusion-architecture`，主仓库 `40f0650` → `origin/main`。架构修正：Pro/Flash 双模型常驻已暂时放弃实现（存档，§4.5），规划/执行分离由 plan mode 状态机承载。2026-08-01 全局审查 L1-L7 闭合状态保持)
**定位**: GSA (General Scientific Assurance) 项目主召回索引 / 组件路由。本文收录**项目架构、P 级合约、Gate 链路、审计文档、Schema 体系、运行时集成、运行时所有权和关键设计约束**的召回入口，目标是让后续开发与回查可便捷定位到正确的文档、源码或 Schema。
**本文不替代审计文档、架构文档、Schema 定义或源代码**；它只负责召回和路由，不负责完整证明。

使用目标:

1. 在新增模块、修改 P 级合约、引入新 Gate 或变更设计约束前，先做 **prior-existence scan**：用关键词、别名、模块名、Gate ID、P 级编号查本文，判断是否已有同义条目、冲突条目、撤回链或废弃条目。
2. 若命中已有条目，沿用其 ID、状态和入口，由入口回归原文/源码进行检查确定，再写结论。
3. 若未命中，可标记为新条目；新增模块/文档后回填本文的短索引项。

回查顺序:

1. 本文：找 ID、别名、状态和入口。
2. 对应审计文档 (`docs/`)：核对具体审计结论、参数、边界条件。
3. 对应架构文档 (`architecture/`)：核对设计意图、合约边界。
4. 对应 Schema 文件 (`assurance/**/*.schema.json`, `runtime/**/*.schema.json`)：核对字段定义与约束。
5. 对应源代码 (`assurance/*.py`)：核对实际实现。
6. ADR (`adr/`)：核对不可变的架构决策记录。

推荐条目格式:

```text
- **ID** (来源/日期, 可选状态): 一句话定义。别名/关键词: xxx, yyy。状态: 事实/设计约束/待实施/撤回/废弃。注意: 主要限制或常见误用。入口: 文件名 / schema 路径 / 源码位置。
```

字段要求:

- **ID**: 优先沿用既有 ID（如 P 级编号、Gate ID）；同一概念改名时保留旧称提示。
- **一句话定义**: 只写可召回核心，不写完整论证。
- **别名/关键词**: 必须覆盖常见换名、误名、英文名和容易混淆的旧词。
- **状态**: 明确区分事实、设计约束、待实施、撤回、废弃。
- **注意**: 写最容易被误用的边界。
- **入口**: 需要精准回指到审计文档、架构文档、Schema 文件或源码位置。

维护规则:

- 本文尽量覆盖全部关键组件和设计约束，但每条保持短索引形态；长解释、完整论证放入审计/架构文档。
- 新审计文档、新模块、重大撤回或术语改名时，更新对应入口即可。
- 本文可以密集，但必须可扫描：ID 加粗、状态清楚、入口明确。

---

## 主题路由 (Topic Routing)

> 按组件领域组织的完整索引。查概念时先从此处浏览或搜索关键词。
> 每条保持短索引形态（≤2行），提供 ID、状态、别名和精准入口。

### A0. 当前方向裁决 (Runtime-First Direction)

- **Runtime-First Graft Decision** (2026-07-30, 设计约束): 通用 agent runtime 不再由本仓库自建；Grok CLI / Grok Build 是第一生产底座候选，本仓库保留 assurance、evidence、UI、adapter、fixture 和审查模式。别名: agent 底座嫁接, runtime-first, ownership correction。入口: `architecture/INTEGRATED_AGENT_ASSURANCE_DESIGN_v0.1.md` / `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md`
- **Agent Base Component Adoption Matrix** (2026-07-30, 设计约束): 部件级采用裁决——Grok 生产采用 model loop/session/tool/permission/sandbox/ACP/workflow/subagent；Claude/Goose/Aider/Codex/Gemini/Qwen 只借鉴 scoped subagent、workflow、MCP、git loop、approval、provider layout 等成熟模式。入口: `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md`
- **Grok CLI Specialization Graft Map** (2026-07-30, 进行中/适配审计; 2026-08-01 ACP fixture path 可运行): Grok runtime adapter、event normalizer、TUI live source、workflow/subagent/profile、`0.2.112` promoted lock 的初步嫁接位置。CLI/TUI 已支持 locked binary doctor、`version-smoke`、显式 `acp-smoke`；无凭证演示路径使用 `--fake-provider` loopback DeepSeek-compatible fixture，不代表真实 DeepSeek 自动接入。入口: `docs/GROK_CLI_SPECIALIZATION_ADAPTATION_AUDIT_2026-07-30.md` / `upstream/grok-build.lock.json` / `assurance/grok_runtime_adapter.py` / `assurance/tui/bridge.py`
- **Explicit Retrieval Mode Constraint** (2026-07-30, 设计约束): 检索模式必须显式选择 `local_browser` / `framework_fallback` / `off`；框架自带检索只作本地浏览器不可用或用户允许时的兜底，不得隐式并存或切换。入口: `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md` / `architecture/LOCAL_BROWSER_RETRIEVAL_AND_PDF_EVIDENCE_v0.1.md`
- **Two-Subagent Default Constraint** (2026-07-30, 设计约束): 默认仅保留项目文档检索子代理和外部检索子代理；不得默认发展第三类或本地多代理调度器。入口: `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md` / `docs/RETRIEVAL_SUBAGENT_AUDIT_2026-07-27.md` / `assurance/retrieval_subagent.py`
- **Neutral vs Counterexample Trigger Split** (2026-07-30 设计约束, 2026-08-04 定稿): 反例询问只在 plan 写入前和最终结论写入前触发；中立询问发生在执行过程中，用于全局回看和方向辅助；检索子代理关闭前增加“是否已获得完成当前主任务所需内容”的中性完成确认。**2026-08-04 定稿（融合架构 §4.6）**: 正式答案输出前**不触发中立问询**（过犹不及，模型能力足够）；正式答案输出前的反例询问**仅触发一次且 message_block 显式告知模型**；plan 写入前反例询问保留；4 判定点（输出阈值/工具调用/动作/轮次）任一触发同一中立问询且**触发瞬间 4 计数全部清零**；子代理动作次数按**上层语义动作**计（一次 `run_retrieval`/每轮 = 1 动作，内部关闭工具级计数）。入口: `architecture/INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.2.md` §4.6 / `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md` / `docs/GSA_SELF_QUESTION_COUNTEREXAMPLE_DESIGN_2026-07-26.md` / `docs/ORIENTATION_RUNTIME_GUARD_AUDIT_2026-07-28.md`
- **Diagnostic Coverage Check** (2026-07-30, 设计约束): debug/问题处理中的递进中性拉回机制。硬信号达到阈值时询问“继续沿当前路线前，关键诊断面是否已经覆盖到足以选择下一步？”阈值按单个 bug 递进 `2 -> 3 -> 4 -> 5`，新证据降噪，用户“继续”不关闭触发。入口: `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md`
- **Global Review Mode** (2026-07-30, 第一切片已完成/审查约束): 显式全局审查模式；日常 review 仍做局部工程审查，明确进入审查模式时才检查原设计理念、当前进度判断、当前实现内容定位、关键设计保留和项目任务边界。已接 `gsa review global` 生成结构化 activation receipt；该 receipt 只激活和界定全局审查义务，不冒充最终审查结论，也不专门服务 CLI/Grok。入口: `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md` / `assurance/global_review_mode.py` / `assurance/global-review-mode-receipt-v0.1.schema.json`
- **2026-08-01 Global Review L1-L7 Closure** (2026-08-01, 审查闭合/索引回填): L1/L2 已先行完成；L3 TUI ContentPane、L4 session/layout、L5 P0-P5/Gate/Security 抽样、L6 schema/repository check、L7 测试策略已在同一审计文档闭合。注意: 该记录是全局审查路由，不替代具体 finding 证据；真实 Grok ACP transcript 仍需后续外部证据回填。入口: `docs/GSA_GLOBAL_REVIEW_RECORD_2026-08-01.md`
- **Adapter Job Object Containment** (2026-07-30, 第一切片已完成/P1, 2026-07-31 全面升级至 CREATE_SUSPENDED): adapter 侧 Kill-On-Close Job Object 进程树包裹——作为 Grok-owned cleanup 的替代性 containment 保证。`JobObjectSupervisor` 用 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` 包裹 Grok 根进程，adapter 关闭 Job handle 时 kernel 杀死全部子进程树。全部 Grok 进程启动路径（`contained_run()`、`run_grok_headless_once()`、`run_grok_acp_once()`、`run_with_job_object_containment()`）已于 2026-07-31 升级至 `CREATE_SUSPENDED` + `AssignProcessToJobObject` + `ResumeThread` 模式，闭合 post-creation 竞态窗口。Containment 裁定：此模式与 `PROC_THREAD_ATTRIBUTE_JOB_LIST` 在 containment 保证上等价；后者仅为代码质量优化（更少 syscall），非安全性必需。`gsa run --runtime grok` promotion gate `adapter_containment_provided` 路径可与 Grok-owned cleanup 互替满足 containment 条件。别名: Job Object containment, adapter containment, JobObjectSupervisor。入口: `assurance/job_object_supervisor.py` / `assurance/grok_runtime_adapter.py` / `assurance/grok_prompt_tool_gate.py` / `docs/JOB_OBJECT_CONTAINMENT_SUFFICIENCY_JUDGMENT_2026-07-31.md`
- **Job Object Containment Sufficiency Judgment** (2026-07-31, 架构裁决): `CREATE_SUSPENDED` + `AssignProcessToJobObject` 已闭合 post-creation 竞态窗口，有效 containment 保证与 GAK-WIN-001 `PROC_THREAD_ATTRIBUTE_JOB_LIST` 等价。后者仅是代码质量优化（更少 syscall、更干净 API），非安全性必需。`windows_child_tree_owned_cleanup` 的 `carried-limitation` 不再构成 prompt/tool 提升的 containment 阻断——adapter 侧 `JobObjectSupervisor` 已提供等效保证。入口: `docs/JOB_OBJECT_CONTAINMENT_SUFFICIENCY_JUDGMENT_2026-07-31.md` / `assurance/job_object_supervisor.py` / `assurance/grok_runtime_adapter.py` / `assurance/grok_prompt_tool_gate.py`
- **Design Sequence Deviation — Headless Before ACP** (2026-07-31, 已修正): `GROK_CLI_SPECIALIZATION_ADAPTATION_AUDIT` §7 明确建议 ACP 优先作为主可观测性路径、headless 仅用于 narrow smoke。修正已完成：(1) `run_grok_acp_once()` 已落地（bdcb2c2），实现 `grok agent stdio` + ACP JSON-RPC `session/prompt` 完整生命周期；(2) CLI `--grok-mode` 默认值已从 `prompt-smoke` 切换为 `acp-smoke`，恢复 ACP→Headless 主次关系。`prompt_smoke`（headless `--prompt-file`）降级为显式指定窄 smoke 路径。入口: `docs/GROK_CLI_SPECIALIZATION_ADAPTATION_AUDIT_2026-07-30.md` §7 / `assurance/grok_runtime_adapter.py` / `assurance/cli.py`

### A0.5. 融合架构与 Rust 实施 (Fusion Architecture & Rust Implementation)

- **Fusion Architecture v0.2** (2026-08-03 冻结, 2026-08-04 Phase 1 + Phase 2 完成, 设计约束/实施中): 融合架构——Codex 纪律 + Grok 能力。Rust workspace 继承 Grok Build `500129c7` ~320k 行提供者（tools/workspace/sandbox/mcp/chat-state/hooks），删除 ~780k 行架构债（shell/pager/telemetry/sampler 等）；自研 orz-host（ACP 薄核心 + LoopHost impl）+ orz-loop（agent loop）+ orz-assurance（journal/gates/orientation/plan）+ orz-bin（`-p`/`--plan`/`--stdio`）+ orz-tui（Phase 3）。Python `assurance/` 保留为 conformance suite + schema authority。**2026-08-04 架构修正**: 放弃 Pro/Flash 双模型常驻（已暂时放弃实现这一构想，存档，非待办——设计文档 §4.5），采用单主 Agent + 检索子代理 ×2——回归 Two-Subagent Default Constraint，规划/执行分离由 plan mode 状态机承载（Rust 已移植），Python 单 agent 先例可 conformance 对齐。**Phase 2 完成（102 tests）**: 单主 Agent loop（8 事件序列 + IPG 前置工具阶段）、gates（IPG/tool availability）、orientation（checkpoint/stagnation）、plan（4-section/8-state）、ACP stdio 真实化、OrzHost（28 工具 + trust fail-closed，接线 Phase 3）、IP6 PermissionBridge（Read 自动放行/Bash headless fail-closed，接线 Phase 3）、run-event schema 扩展 plan 事件。别名: fusion, orz, 融合架构。**2026-08-04 补充定稿（§4.6）**: 中立问询与反例询问触发设计——正式输出前中立问询不触发、反例询问仅一次且显式告知模型、4 判定点触发即清零（**显式取代 CN 轮次豁免**）、子代理动作语义分层（上层语义动作计数）。**2026-08-04 提交前审查修复（102 tests）**: P0 多 prompt 每轮独立 run journal（journal=单 run 单 terminal 完整性单元）、recorder terminal 后显式 `TerminalAppended` 错误（不再静默丢）、run_turn 返回 `(response, next_sequence, last_hash)` 支持同 journal 续链（--plan 路径）、错误路径写 RunFailed、终局按 stagnation 判定写 run_invalidated、LoopHost 默认 Deny 防 fail-open（OrzHost/PermissionBridge **实现完成/接线 Phase 3**）、Defer 拒绝执行、plan revise/reject 回写 approval 字段。已知偏差（Phase 2 接受清单）: journal payload 字段形状与 Python no-model fixture 不同（真实 agent 数据，conformance 豁免）。低风险项已全清：event_id 对齐 Python `{:03}`、tool_availability_check 对齐 Python 先例（先于 run_started）、LoopHost 默认 Deny、permission access_kind 参数映射补齐（list_dir `target_directory`/grep `path`）、blackboard/注释清理、orz-host Cargo.toml 死依赖移除、acp_server `close_session`、EventWriter 链状态写后推进。已推送: orz `5ab590f` → `cli/feat/fusion-architecture`；主仓库 `40f0650` → `origin/main`（含本索引与 §4.6 设计）。入口: `architecture/INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.2.md`（权威） / `D:\CLI\orz` / `README.md` / memory `fusion-phase-tracking.md`
- **继承 crate Windows 适配全仓清零** (2026-08-04, 完成并已提交推送/46 文件 +481/-149; orz `d48b724` → `cli/feat/fusion-architecture`, 主仓库 `d4ba987` → `origin/main`): 全仓测试首次在 Windows 跑通——`cargo test --workspace` 全绿（自研 122 + 继承 ~6200 tests，0 failed）、clippy 0 error（169 既有 warning 零新增）。**生产 bug 修复**：① orz-config managed_text——`ParentAnchor` 目录句柄 cfg 到 unix（`File::open` 不能开目录）+ `FileIdentity` Windows 改用 `GetFileInformationByHandle` 文件索引（NTFS 目录 mtime 随条目变化，锁文件创建曾致每次 apply 误报 ParentChanged）；② orz-memory journal_mode 恢复 `busy_timeout + PRAGMA journal_mode=WAL`（fork 裁剪枚举时丢失）；③ orz-tools resolve_model_path `is_absolute()`→`has_root()`（Windows 根相对路径 `/etc/hosts` 被 join 成 `C:\etc\hosts` 致权限防护落空）；④ orz-workspace exec_risk/shell_access 同款 has_root（安全扫描路径错位、保护编辑分类失效）+ envrc `/bin/bash` 硬编码、裸 `bash` 解析到 WSL 启动器（CreateProcess 优先 System32）→ `find_git_bash` 提 pub 用全路径 + 路径正斜杠化；⑤ xai-fast-worktree WorktreeDb::get `contains('/')`→双分隔符（Windows 路径全 `\`，工作树 label/touch/source-repo 塌缩全 None）；⑥ daemonize PidFile 增 `contents()`/`write_contents()`（Windows 独占锁拒其他句柄读写）。**测试门控/适配**：POSIX shell 语义 + HOME 模拟测试（dirs 6.0.0 home_dir 走 SHGetKnownFolderPath，env 无法模拟）cfg(unix) 门控约 30 个；python3→python（WindowsApp 别名存根）；LSP mock 脚本 `sys.stdout.reconfigure(newline="")`（python text 模式 CRLF 翻译致 `\r\r\n`）；`nul` 保留设备名改名；CRLF 归一化；分隔符断言改组件级。**环境**：ripgrep 15.2.0 已装（winget BurntSushi.ripgrep.MSVC——grep 工具运行时依赖）。入口: `D:\CLI\orz\crates/codegen/*` / memory `fusion-phase-tracking.md`
- **CI 全绿 + 本地扫描性能修复** (2026-08-05, 完成并已提交推送): ① CI 失败清零（主仓库 `aad7b34`）——f86eb15 alpha 语义（无/不完整 ACP probe → unavailable block，非 degraded warn/defer）与陈旧测试断言对齐：observer 两测试重命名（`test_blocks_static_observation_without_acp_probe`/`test_single_attached_acp_probe_blocks_projection`）、decision defer→block、probe degraded→unavailable、gate_decision warn→block；cli_dispatcher observe-tools JSON 同改 block；GAK-CRED-001 allowlist 行号漂移修复（canonical_cli.py 1090→1099，f86eb15 插入行移位，finally scrub 仍在 L1137）。② workspace trust 扫描剪枝（主仓库 `336a409`，生产代码）——`_iter_workspace_files` os.walk topdown 剪枝跳过 `.git` 与含 `.git` 的嵌套仓库根（gitignored orz/ 19GB 不再哈希）：修复本地 canonical `gsa run`/`verify`/`doctor` 卡死（CI 无 orz/ 未暴露）；establish/verify 共用同 helper 保持 digest 一致。③ `check_repository` `NON_REPOSITORY_PARTS` + `orz`——嵌套仓库文档由其自身审查治理，防 orz 文档漂移再染本地 doctor。④ orz README 14 处失效链接修复（orz `c00147f`）——`bin/protoc`→`bin/`（实际为 protoc.exe/.dotslash）、xai-grok-pager docs 死链去链（crate 已删）、xai-grok-tools THIRD_PARTY bullet 删除、orz-hooks examples 10-hooks.md 去链、third_party/README.md 重写为 provenance stub（Mermaid 栈随 pager 移除，NOTICE 保留）。验证：本地 `gsa doctor` exit 0、test_cli_dispatcher 39 tests 25s 全绿、主仓库 CI 4 作业全绿。注意: orz/README.md 整体仍是上游 Grok 文档（build 段 `cargo run -p xai-grok-pager-bin` 等 stale 散文未重写，仅链接已修，后续文档 pass）。入口: `assurance/workspace_trust.py`（`_iter_workspace_files`）/ `scripts/check_repository.py` / `assurance/tests/test_{grok_tool_permission_observer,cli_dispatcher,credential_scrub}.py` / orz `README.md` + `crates/codegen/orz-hooks/examples/README.md` + `third_party/README.md` / memory `local-workspace-trust-scan-latency.md`
- **orz Rust Workspace 位置** (2026-08-04, 事实): Rust 实施仓库从 `B:\orz` 迁移至 **`D:\CLI\orz`**（B 盘空间不足 4GB→19.4GB；迁移后 cargo check 绿色）。分支 `feat/fusion-architecture`（70 crate）；远端 `cli` → `SilverWhite/CLI.git`（同一远端承载 D:\CLI Python main + orz fusion 分支）。别名: B:\orz (旧路径, 已删)。入口: `D:\CLI\orz` / memory `disk-space-and-migration.md`
- **Phase 1 完成（Journal + Transport + 单 Agent Loop）** (2026-08-04, 事实/22 tests pass): orz-assurance::journal（hash-chained JSONL recorder/verifier/chain，含 Post-PlanA 4 bug 修正——阻塞 send、verifier 重算 event_sha256、无 orphan 死代码；13 tests）+ orz-loop（AgentLoopController/Blackboard/MechanicalRelay/LoopHost trait；5 tests）+ orz-host（SessionBootstrap + ACP server scaffold + JournalOnlyHost；4 tests）+ orz-bin `-p` 入口。验收达成: `orz -p "hello"` → 5 事件（preflight→started→prompt→output→finished）连续 hash chain `events.jsonl`。审查修复: session bootstrap/run_turn 链衔接、terminal 后拒绝追加、测试隔离。入口: `D:\CLI\orz\crates/{orz-assurance,orz-loop,orz-host,orz-bin}` / `D:\CLI\architecture/INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.2.md` §5
- **Phase 2 完成（单主 Agent + Gates + ACP + Plan Mode）** (2026-08-04, 事实/102 tests pass): S00-S10 全部闭合——① orz-assurance: gates（IPG 24 模式/四态 tool availability）+ orientation（checkpoint/stagnation）+ plan（4-section/8-state/两级 approval），EventType 对齐 run-event schema（新增 plan_proposed/plan_approved/plan_rejected/action_approved，Python authority 先行）；② orz-loop: 单主 Agent loop（8 事件序列 + IPG 前置工具阶段 + tool 循环 + 检索子代理调度）、ModelGateway trait + FakeProvider、relay 检索路由；③ orz-host: ACP stdio 真实化（AgentSideConnection + AcpAgentGatewaySender）、OrzHost（FinalizedToolset **28 工具** + trust fail-closed 显式传参）、PermissionBridge（IP6：Read 自动放行/Bash headless fail-closed）；④ orz-bin: `-p`/`--plan`/`--stdio` 三入口。E2E: 13 事件完整链、IPG block 无 tool_started、plan 批准后才执行。2026-08-04 提交前审查修复: P0 多 prompt 每轮独立 run journal、recorder terminal 后显式错误、错误路径 RunFailed、终局 run_invalidated、默认 Deny、Defer 拒绝、plan 回写、tool_availability 前置、event_id `{:03}`（+7 tests）。**2026-08-04 Phase 3 Slice #1（OrzHost/PermissionBridge 接线完成 + 提交前审查全发现处理，109 tests）**: 三入口全部切换至 OrzHost + IP6 PermissionBridge——`PermissionBridge::spawn(gateway: Option)`（None → 死网关 fail-closed）、`OrzHost::with_bridge()` 公共构造 + `request_permission` 覆写（无 bridge 显式 Deny 保留 P1-1）；headless 语义落地：read_file 自动放行真实执行（allow_once → tool_started/completed），bash Ask → Deny（无 tool_started）；orz-bin 三入口包 LocalSet + cwd 绝对化；E2E 演示改为 read_file+rust-toolchain.toml（Cargo.toml 会诚实触发 stagnation ngram → run_invalidated）。**审查发现处理（109 tests）**: P1 Read 越界闭合——桥层 `access_in_scope()`：相对路径 lexical 归一（`..` 逃逸防护）+ canonicalize + 必须位于会话 cwd 内且不在 `{cwd}/.gsa` 树内（provider Read 决策无条件 Allow 且 read_file 保留绝对路径）；Windows `\\?\` verbatim 前缀剥离（canonicalize 回归——真实文件全被误拒，已用存在的文件用例锁死）；Grep 同受约束。P2-1: `bash|sh|cmd|powershell|pwsh` → Bash 分类（不再落 Edit 桶）。P2-2: `handle.request` 300s 超时 → Deny（静默 live 客户端不挂死）。P2-4: toolset ~160ms 实测无需缓存（已评估）。P3: `dead_gateway` 提 pub(crate) 复用。无 profile 副作用实证（`~/.grok/sessions` 无写入、EventWriter noop、actor 无泄漏）。测试 orz-host 15→19→23，总 109（loop 54 + host 23 + assurance 32）。剩余 Phase 3: session snapshot、sandbox（job_object/credential/permit）、TUI、Clippy 全仓、Python reference-spec。入口: `D:\CLI\orz\crates/*` / `architecture/INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.2.md` §5 / memory `fusion-phase-tracking.md`
- **Phase 3 Slice #2（§4.6 问询触发三机制接线）** (2026-08-04, 完成并已提交推送/122 tests): ① 中立问询——`orz-loop/src/inquiry.rs`（InquiryCounters 4 判定点 + DEFAULT_THRESHOLDS + parse_completion_decision）+ controller 工具循环检索后 `maybe_fire_neutral_inquiry`（触发瞬间三实例全清零）；INFO_SUFFICIENCY block 以 User message 注入主 agent 下一轮。② 子代理关闭 completion check——`RETRIEVAL_COMPLETION_CHECK v0.1` Python 逐字移植，block 注入子代理请求（`run_retrieval` 新增 `completion_check_block` 参数），响应逐行解析 decision，事件带完整响应文本。③ 反例询问——正式答案输出前：无 tool_calls 分支拦截一次（草稿 journal 不入会话）；plan 写入前：`run_plan` submit_plan 前模型轮（plan 变体 block 无"仅一次"行，evidence-only）。三种新 run-event：`neutral_inquiry`/`counterexample_gate`/`retrieval_completion_check`（schema 27→30 + 3 payload schema，Python authority 先行）+ Rust EventType 同步。阈值定稿 ADR-0005（output>10/tool_calls>10/actions>10/rounds>8 严格大于）。模型回答无控制流后果（证据记录）。E2E：`-p` 10 事件链含 gate(final_answer)；`--plan` 13 事件链含 gate(plan_write+final_answer)。**提交前审查闭合**: P1 无；P2-1 plan 阶段错误路径补写 RunFailed 终态（抽取 `run_plan_phase` + `record_plan_failure`，+1 测试——journal 单 run 单 terminal 完整性）；P3×4 记录（block/摘要消息顺序观感、1-round 子代理下 completion check 每次检索触发的前瞻、gate 后继续工具调用无第二次 gate 的自洽语义、response 字段大小前瞻）。已推送: orz `a7fca22` → `cli/feat/fusion-architecture`；主仓库 `0c87d5b` + `023b699` → `origin/main`。入口: `D:\CLI\orz\crates/orz-loop/src/{inquiry.rs,controller.rs,prompt.rs,agents/retrieval.rs}` / `D:\CLI\runtime/run-event-v0.1.schema.json` / `D:\CLI\adr\ADR-0005-neutral-inquiry-thresholds-finalized.md` / `architecture/INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.2.md` §4.6

### A. P 级合约总览 (P-Level Contract Hierarchy)

- **P0** (2026-07-26, 事实/合约层): 数据语义层——基础 Schema 定义、信封、合约基类。这是所有上层合约的语义基础。入口: `assurance/contracts.py` / `assurance/envelope.py` / `docs/CANONICAL_GUARDED_CLI_P0_AUDIT_2026-07-26.md`
- **P1** (2026-07-25, 事实/合约层): 会话身份与归档删除生命周期——conversation namespace + permit + archive journal 的基础链路。2026-07-28: StorageAdapter 已接入 ArchiveController；archive_recovery.py（holistic 恢复编排器）；SessionGovernor（thin wrapper 确保所有 adapter artifact 通过 namespace 路由）；canonical CLI 真实 adapter 路径已接入 ConversationNamespace + enforce_adapter_call()。入口: `docs/P1_CONVERSATION_ARCHIVE_LIFECYCLE_AUDIT_2026-07-28.md` / `assurance/conversation.py` / `assurance/permit.py` / `assurance/archive.py` / `assurance/archive_journal.py` / `assurance/archive_recovery.py` / `assurance/archive_verifier.py` / `assurance/session_governor.py`
- **P2** (2026-07-24, 事实/合约层): Docker 严格沙箱——进程隔离、状态可观测性。Windows Native Sandbox (AppContainer + Job Object) 作为辅助沙箱选项。入口: `docs/P2_DOCKER_SANDBOX_AUDIT_2026-07-24.md` / `assurance/sandbox.py` / `assurance/windows_sandbox.py` / `docs/GAK_SBX_001_WINDOWS_NATIVE_SANDBOX_AUDIT_2026-07-27.md`
- **P2.5** (2026-07-24, 事实/合约层): 守卫执行 (Guarded Execution)——无模型动作、进程追踪、输出拦截。入口: `docs/P2_5_GUARDED_EXECUTION_AUDIT_2026-07-24.md` / `assurance/guarded_execution.py`
- **P3** (2026-07-25, 事实/合约层): 指令授权——instruction provenance、capability delegation、action authorization、child capability enforcement。入口: `docs/P3_INSTRUCTION_AUTHORITY_AUDIT_2026-07-25.md` / `assurance/instruction_gate.py` / `assurance/instruction_provenance_gate.py` / `assurance/child_capability_enforcer.py`
- **P4** (2026-07-25, 事实/合约层): 元数据审计 + 压缩 + 恢复授权——audit ledger, archive verifier, recovery candidate。入口: `docs/P4_AUDIT_COMPACTION_RECOVERY_AUDIT_2026-07-25.md` / `assurance/audit.py` / `assurance/archive_verifier.py` / `assurance/recovery.py`
- **P4.5** (2026-07-25, 事实/合约层): 工作区优先集成——workspace trust + integrated run + network permit gateway。入口: `docs/P4_5_WORKSPACE_FIRST_INTEGRATION_AUDIT_2026-07-25.md` / `assurance/integrated_run.py` / `assurance/workspace_trust.py` / `assurance/network_permit_gateway.py`
- **P5** (2026-07-25, 事实/合约层): 合成用户任务 + 机械预检 + 只读投影——synthetic user task evaluation, runtime preflight, readonly projection。入口: `docs/P5_SYNTHETIC_USER_TASK_PREFLIGHT_2026-07-25.md` / `assurance/user_task_evaluation.py` / `assurance/runtime_preflight.py` / `assurance/readonly_projection.py`
- **GSA-CORE** (2026-07-25, 事实/合约层): 领域中立的只读审查切片——多文件 artifact schema registry + validator bridge + general science review。独立于 P 级框架。入口: `docs/GSA_CORE_READONLY_SLICE_AUDIT_2026-07-25.md` / `assurance/artifact_registry.py` / `assurance/validator_bridge.py` / `assurance/general_science_review.py`

### B. Gate 链路 (Gate Chain)

> 规范守卫 CLI 的离线主路径 Gate 序列。Gate 之间为串行链：前一个通过后才进入下一个。

- **GAK-INJ-001 指令来源 Gate** (2026-07-27, 事实/2026-07-28 关闭): 入口级多源指令分类/反注入 Gate——batch gate 对多源指令进行 provenance 分类，拦截注入。2026-07-28: 新增 production content parser（parse_instruction_content + detect_obfuscated_injection）、canonicalizer 集成（路径遍历/SSRF 检测）、25 个注入模式、unified gate entry 验证；89 tests pass。别名: instruction provenance gate。入口: `docs/GAK_INJ_001_AUDIT_2026-07-27.md` / `assurance/instruction_provenance_gate.py` / `assurance/tests/test_injection_adversarial.py`
- **Tool Availability Gate** (2026-07-27, 事实/assurance-owned): 机械式工具可用性探测——在模型调用前验证工具声明与运行时实际可用性一致性，检测 belief mismatch/stagnation。Grok 负责实际 tool registry/dispatch，本 Gate 负责状态 receipt 和 UI/prompt 可见性；2026-07-30 已新增 Grok registry/permission observation receipt 与 `gsa grok observe-tools` 静态观察入口，ACP permission projection 需同时附加有效 `allow_once` 与 `cancel_permission` fake-tool verification 才从 degraded/block 转为 available/allow。入口: `docs/TOOL_AVAILABILITY_GATE_AUDIT_2026-07-27.md` / `assurance/tool_availability_gate.py` / `assurance/grok_tool_permission_observer.py`
- **Retrieval Subagents (2 only)** (2026-07-27, 事实/assurance-owned, 2026-07-30 重分类): 默认仅保留两个检索子代理：项目文档检索子代理和外部检索子代理。关闭子代理前执行中性 completion check：只问是否已获得当前主任务所需内容。后续优先映射到 Grok subagent/workflow/profile，不新增本地多代理调度器。入口: `docs/RETRIEVAL_SUBAGENT_AUDIT_2026-07-27.md` / `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md` / `assurance/retrieval_subagent.py` / `assurance/project_doc_index.py` / `assurance/tests/test_retrieval_subagent_real.py`
- **Orientation Runtime Guard / Neutral Inquiry** (2026-07-27, 事实/assurance-owned): 中性方向检查点 + 运行时停滞守卫——执行过程中辅助全局回看，检测 agent 是否陷入循环/停滞。2026-07-30 补充: debug 路线锁死时使用 Diagnostic Coverage Check，按递进阈值触发；Grok workflow/subagent lifecycle 可经 metadata-only projection 注入中性 Orientation context。入口: `docs/ORIENTATION_RUNTIME_GUARD_AUDIT_2026-07-28.md` / `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md` / `assurance/orientation_runtime_guard.py` / `assurance/grok_lifecycle_projection.py` / `assurance/orientation-checkpoint-v0.1.schema.json`
- **Workspace Trust Gate** (2026-07-28, 事实/2026-07-28 关闭 GAK-TRUST-001): 统一工作区信任——全部 Python 入口点（canonical CLI + 检索子代理 ×2）在 IPG 评估前建立 workspace trust receipt。`AdapterGateContext` 携带 trust_receipt + trust_status。别名: GAK-TRUST-001。入口: `assurance/workspace_trust.py` / `assurance/adapter_gate.py` / `assurance/canonical_cli.py` / `assurance/retrieval_subagent.py`
- **Source Visibility Gate** (2026-07-26, 事实): 全文可见性检查 Gate——验证引用源的完整文本可见性。入口: `docs/SOURCE_FULLTEXT_VISIBILITY_RULE_2026-07-26.md` / `assurance/source_visibility.py` / `assurance/source_visibility_cli.py`
- **Local Browser Retrieval and PDF Evidence** (2026-07-28 设计冻结, 2026-07-29 关闭/assurance-owned): 本地浏览器外部检索与论文 PDF 确定性证据链。2026-07-30 约束: 默认主路径为 `local_browser`；框架自带检索只作 `framework_fallback`，不得隐式并存或切换。别名/关键词: browser retrieval, local_browser, framework_fallback, PDF evidence, LBR-001, CDP, GSA Chrome profile。入口: `architecture/LOCAL_BROWSER_RETRIEVAL_AND_PDF_EVIDENCE_v0.1.md` / `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md` / `assurance/pdf_evidence.py` / `assurance/evidence_store.py` / `assurance/browser_retrieval.py` / `assurance/retrieval_workflow.py` / `assurance/tests/test_browser_retrieval_e2e.py`
- **Child Capability Gate** (2026-07-28, 事实/2026-07-28 关闭 GAK-CHILD-001): 子代理/子进程 capability 传递——`spawn_child_context()` 统一入口；检索子代理接受 parent_envelope；`execute_guarded_no_model_action()` 在 Docker create 前调用 `enforce_child_capabilities()`。别名: GAK-CHILD-001。入口: `assurance/child_capability_enforcer.py` / `assurance/instruction_gate.py` / `assurance/retrieval_subagent.py` / `assurance/guarded_execution.py`
- **Network Permit Gate** (2026-07-28, 事实/2026-08-01 复核通过): 网络许可统一覆盖——`call_deepseek_api()` 在 HTTP 请求前调用 `evaluate_network_permit()`；`AdapterGateContext` 携带 network_policy；`_resolve_and_setup_gates()` 构建 guarded 模式默认 policy。2026-08-01: endpoint allowlist 对 canonical endpoint 始终生效，新增 denylist；verifier 会按 policy 重放 category、endpoint 与 attempt 约束。别名: GAK-NET-001。入口: `assurance/network_permit_gateway.py` / `assurance/deepseek_adapter.py` / `assurance/adapter_gate.py` / `assurance/canonical_cli.py` / `docs/GSA_GLOBAL_REVIEW_RECORD_2026-08-01.md`
- **Adapter Gate** (2026-07-27, 事实/2026-08-01 复核通过): 适配器调用旁路执行 Gate——防止运行时绕过适配器直接调用模型。2026-08-01: `network_policy.require_permit_for_all` 会在 adapter call 前硬阻断未获 permit 的调用，enforcement verifier 也纳入 network permit 状态。别名: adapter gate enforcement, adapter preflight。入口: `docs/ADAPTER_GATE_AUDIT_2026-07-28.md` / `assurance/adapter_gate.py` / `assurance/adapter_preflight.py` / `assurance/adapter_output_validator.py` / `assurance/adapter_failure_classifier.py` / `assurance/adapter-gate-enforcement-receipt-v0.1.schema.json` / `docs/GSA_GLOBAL_REVIEW_RECORD_2026-08-01.md`
- **Canonical Guarded CLI Gate 序列** (2026-07-26, 事实/fixture+assurance path): 离线主路径 Gate 序列定义: instruction provenance gate → tool availability gate → orientation checkpoint → source visibility gate → fake DeepSeek adapter boundary → answer packet → runtime JSONL journal → independent verifier。2026-07-30 后不作为最终 production agent runtime；Grok 嫁接后保留为 conformance/assurance fixture 与回归路径。入口: `docs/CANONICAL_GUARDED_CLI_P0_AUDIT_2026-07-26.md` / `assurance/canonical_cli.py` / `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md`

### C. 运行时集成 (Runtime Integration)

- **Grok Build / Grok CLI** (2026-07-21~30, runtime-owned/第一生产底座; 2026-08-01 所有权复核): 当前优先生产 runtime 底座。Grok 应拥有 model loop、session、tool dispatch、permission 基础面、sandbox/进程基础、custom model config、ACP、workflow、subagent、MCP/plugins/skills、compaction/checkpoint。项目默认 lock 已提升至 `0.2.112 (9bbd559437)`；Windows child-tree timeout 已拆成 baseline-regression passed 与 owned-cleanup carried-limitation。2026-08-01: `gsa grok run --mode acp-smoke --fake-provider` 与 `gsa.py tui --runtime grok --fake-provider --run <prompt>` 已跑通 loopback ACP fixture；TUI session list 只读扫描 `.gsa/runs/*/session.json` 薄 marker，恢复委托 `grok session resume <session_id>`，本仓库不实现自有 session store/resume。入口: `architecture/GROK_BUILD_ADAPTATION_v0.1.md` / `architecture/SESSION_PERSISTENCE_AND_LAYOUT_COMPACTION_v0.1.md` / `docs/GROK_CLI_SPECIALIZATION_ADAPTATION_AUDIT_2026-07-30.md` / `docs/GSA_GLOBAL_REVIEW_RECORD_2026-08-01.md` / `upstream/grok-build.lock.json` / `integration/grok/`
- **DeepSeek Adapter** (2026-07-23, 事实/首个真实适配器): 首个真实模型适配器——通过 DeepSeek API 进行 one-shot/stream 观察。凭证硬化已完成。入口: `docs/DEEPSEEK_CREDENTIAL_HARDENING_2026-07-23.md` / `architecture/DEEPSEEK_ADAPTER_CONTRACT_v0.1.md` / `assurance/deepseek_adapter.py` / `assurance/deepseek_api_observation.py` / `assurance/deepseek_stream_observation.py`
- **Codex App Server / VS Code Internal Terminal Lifecycle** (2026-07-25~30, 事实/生命周期研究): Codex app server、VS Code 内部终端方向的初步会话生命周期捕获、标准化与验证。2026-07-30 裁决: 保留为本地 IDE/session 观测基础，不升级为完整 VS Code 插件或完整 IDE 前端适配层。入口: `docs/CODEX_APP_SERVER_LIFECYCLE_NORMALIZER_AUDIT_2026-07-25.md` / `docs/CLI_SESSION_LIFECYCLE_ADAPTER_AUDIT_2026-07-25.md` / `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md` / `runtime/codex-app-server-*.schema.json` / `scripts/capture_codex_app_server_lifecycle.py`

### D. Global Progress Sentinel (GPS)

> 全局进度哨兵系统——跨 agent 会话的进度追踪、检查点、状态归约与日志恢复。

- **GPS Holistic Gate** (2026-07-25, 事实): 全局进度整体门控——跨会话的 holistic 进度评估。入口: `docs/GLOBAL_PROGRESS_HOLISTIC_GATE_AUDIT_2026-07-25.md`
- **GPS Checkpoint Adapter** (2026-07-25, 事实): 检查点适配器——策略化检查点保存/恢复。入口: `docs/GLOBAL_PROGRESS_CHECKPOINT_ADAPTER_AUDIT_2026-07-25.md`
- **GPS Transition Gate** (2026-07-25, 事实): 进度转换门控——状态迁移的守卫逻辑。入口: `docs/GLOBAL_PROGRESS_TRANSITION_GATE_AUDIT_2026-07-25.md`
- **GPS Atomic Journal & Reason Migration** (2026-07-25, 事实): 原子日志 + reason code 迁移。入口: `docs/GLOBAL_PROGRESS_ATOMIC_JOURNAL_AND_REASON_MIGRATION_AUDIT_2026-07-25.md` / `protocol/reason-codes-v0.1.yaml`
- **GPS Journal Recovery** (2026-07-25, 事实): 日志恢复机制。入口: `docs/GLOBAL_PROGRESS_JOURNAL_RECOVERY_AUDIT_2026-07-25.md`
- **GPS Runtime Controller** (2026-07-25, 事实): 运行时控制器。入口: `docs/GLOBAL_PROGRESS_RUNTIME_CONTROLLER_AUDIT_2026-07-25.md`
- **GPS Controller Verifier** (2026-07-25, 事实): 控制器输出验证器。入口: `docs/GLOBAL_PROGRESS_CONTROLLER_VERIFIER_AUDIT_2026-07-25.md`
- **GPS State Reducer** (2026-07-25, 事实): 状态归约器——跨会话状态聚合。入口: `docs/GLOBAL_PROGRESS_STATE_REDUCER_AUDIT_2026-07-25.md`
- **GPS 合约设计** (2026-07-21, 设计约束): GPS 的架构合约定义。入口: `architecture/GLOBAL_PROGRESS_SENTINEL_CONTRACT_v0.1.md`
- **GPS Grok Lifecycle Projection** (2026-07-30, 第一切片已完成): Grok workflow/subagent lifecycle metadata 投影为 GPS journal events；只消费 metadata，不复制 prompt/tool/raw output，不把 workflow 决策提升为 GPS 决策。入口: `assurance/grok_lifecycle_projection.py` / `assurance/grok-lifecycle-projection-receipt-v0.1.schema.json`

### E. CLI 与执行 (CLI & Execution)

- **gsa CLI 入口** (2026-07, 事实): 顶级 CLI 分发器——`gsa doctor`, `gsa source gate`, `gsa run`, `gsa run --runtime grok`, `gsa verify`, `gsa tui`, `gsa grok doctor`, `gsa grok run --mode version-smoke`, `gsa grok observe-tools`, `gsa review global`。注意: `gsa run` 仍保持 canonical 默认路径；`gsa run --runtime grok` 只写 fail-closed prompt/tool promotion gate receipt，不执行 prompt/tool；`observe-tools` 只做 registry/permission 静态观察与 availability 投影。入口: `gsa.py` / `assurance/cli.py` / `assurance/grok_prompt_tool_gate.py`；TUI 设计: `architecture/CLI_UI_INTERACTION_MODEL_v0.1.md` / `architecture/CLI_UI_SIMPLIFICATION_SUPPLEMENT_v0.1.md`
- **Canonical Guarded CLI** (2026-07-26, 事实): 规范守卫 CLI 主路径——manifests, fake answer packets, verification。注意: TUI 将消费本模块的结构化事件流（gate_decision, run_state 等），通过 view model 投影渲染到终端；本模块保持事件发射职责，不耦合 UI 渲染。入口: `docs/CANONICAL_CLI_QUICKSTART_2026-07-26.md` / `assurance/canonical_cli.py` / `assurance/canonical_cli_main.py`；TUI 设计: `architecture/CLI_UI_INTERACTION_MODEL_v0.1.md`
- **Disposable Reproduction** (2026-07-26, 事实): 可处置复现——manifest, receipt, proof 的完整生命周期。入口: `docs/GSA_DISPOSABLE_REPRODUCTION_AUDIT_2026-07-26.md` / `assurance/disposable_reproduction.py` / `assurance/execution_lock.py`
- **Guarded Execution** (2026-07-24, 事实): P2.5 守卫执行——进程追踪与输出拦截。入口: `assurance/guarded_execution.py` / `assurance/child_capability_enforcer.py`
- **Runner (No-Model)** (2026-07, 事实): 无模型 runner 骨架——journal recovery/repair。入口: `assurance/runner.py` / `assurance/runner_public_output.py` / `assurance/runner_scoring_handoff.py`
- **Integrated Run** (2026-07-25, 事实): P4.5 工作区优先集成运行。入口: `assurance/integrated_run.py`
- **Task Contract** (2026-07, 事实): 从 ask 构建任务合约。入口: `assurance/task_contract.py`
- **Execution Lock** (2026-07, 事实): 可处置复现的执行锁。入口: `assurance/execution_lock.py`
- **Archive Journal Recovery** (2026-07, 事实): 归档日志的恢复与修复脚本。入口: `scripts/recover_archive_journal.py`

### F. 安全与沙箱 (Security & Sandbox)

- **Docker Sandbox** (2026-07-24, 事实): P2 Docker 严格沙箱——进程隔离、网络限制、状态可观测。入口: `docs/P2_DOCKER_SANDBOX_AUDIT_2026-07-24.md` / `assurance/sandbox.py` / `assurance/sandbox_verifier.py`
- **Windows Native Sandbox** (2026-07-27, 事实): AppContainer + Job Object + netsh firewall 的 Windows 原生沙箱。Elevated 路径 compliant（firewall rule 创建成功 → selection allow），non-elevated 路径 fail-closed（正确行为）。别名: GAK-SBX-001。入口: `docs/GAK_SBX_001_WINDOWS_NATIVE_SANDBOX_AUDIT_2026-07-27.md` / `assurance/windows_sandbox.py`
- **Network Permit Gateway** (2026-07-25, 事实): 网络许可评估——控制 agent 对外网络访问权限。入口: `assurance/network_permit_gateway.py` / `architecture/NETWORK_PERMIT_BROKER_CONTRACT_v0.1.md`
- **Workspace Trust** (2026-07-21, 事实): 工作区信任建立与验证——防止 agent 在不受信任的目录中执行。入口: `docs/GROK_WORKSPACE_TRUST_AUDIT_2026-07-21.md` / `assurance/workspace_trust.py`
- **Keystore / Key Lifecycle** (2026-07, 事实): 安装级密钥存储（内存 + Windows DPAPI）+ 密钥生命周期控制。入口: `assurance/keystore.py` / `assurance/key_lifecycle.py`
- **Envelope** (2026-07, 事实): 安全信封——数据完整性与来源验证。入口: `assurance/envelope.py`
- **Permit** (2026-07, 事实): 敏感操作许可生命周期。入口: `assurance/permit.py`
- **Endpoint Canonicalizer** (2026-07, 事实): 文件系统路径与网络端点的规范化——防止路径遍历与 SSRF。入口: `assurance/endpoint_canonicalizer.py`
- **Storage Adapter** (2026-07, 事实): 本地存储适配器——统一的文件系统访问接口。入口: `assurance/storage_adapter.py`

### G. 架构文档 (Architecture)

> 设计意图、合约边界与战略决策。每篇文档承担一个独立的设计主题。

- **产品定位与参考策略** (v0.1, 设计约束): 核心定位——运行时中立的通用科学保证内核，不绑定单一 LLM 运行时。入口: `architecture/PRODUCT_POSITIONING_AND_REFERENCE_STRATEGY_v0.1.md`
- **Integrated Agent Assurance Design** (v0.1, 设计约束/2026-07-30 修正): 保留全部关键设计，但明确成熟 runtime 拥有通用 agent 平台，本仓库拥有 assurance/evidence/UI/adapter/fixture。入口: `architecture/INTEGRATED_AGENT_ASSURANCE_DESIGN_v0.1.md`
- **Agent 底座嫁接价值中文评估** (2026-07-30, 设计约束/中文主入口): 中文确认保留内容、检索显式开关、两个子代理限制、中立/反例触发分离、全局审查模式、VS Code 内部终端保留、部件级采用裁决。入口: `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md`
- **Grok CLI Specialization Adaptation Audit** (2026-07-30, 进行中/嫁接审计): Grok runtime adapter、event normalizer、TUI live source、workflow/subagent/profile、`0.2.112` promoted gate 的实现路由；CLI/TUI 已接 first-slice version-smoke 与显式 ACP fake-provider smoke。入口: `docs/GROK_CLI_SPECIALIZATION_ADAPTATION_AUDIT_2026-07-30.md`
- **CLI UI Interaction Model** (v0.1, 事实/2026-08-01 复核通过): Windows-only 复古桌面/老 IE 风格终端 UI 交互模型。第一原型已完成（9 个模块 ~3900 行）：静态框架 + prompt_toolkit + 自定义 retro widget 层、中文斜杠命令 + CommandRegistry + CommandPalette 覆盖层、地址栏命令历史/多行输入/自动补全、Ctrl+Z 取消运行。2026-08-01: ContentPane `text_delta` 合并、code block ANSI 宽度、source/tool 区域高度补齐、session list 薄 marker 发现、address focus cycle 均已复核并测试通过；TUI bridge 可通过显式 `--fake-provider` 消费真实 Grok ACP loopback smoke 的 `text_delta` 与 normalized `model_output`。别名: retro TUI, Explorer-style UI, old IE UI, terminal TUI, ContentPane, session list。入口: `architecture/CLI_UI_INTERACTION_MODEL_v0.1.md` / `architecture/CONTENT_PANE_CONVERSATION_RENDERING_v0.1.md` / `architecture/SESSION_PERSISTENCE_AND_LAYOUT_COMPACTION_v0.1.md` / `docs/GSA_GLOBAL_REVIEW_RECORD_2026-08-01.md`；代码: `assurance/tui/`（`bridge.py` / `commands.py` / `widgets.py` / `app.py` / `pt_app.py` / `view_models.py` / `main.py` / `events.py` / `event_source.py` / `projector.py`）；Gap Register: GAK-UI-001
- **CLI UI Simplification Supplement** (v0.1, 设计约束/2026-07-29 R16 实施闭合): 下一轮 TUI 精简与中文化方向——R15 完成默认中文 UI（菜单/工具栏/状态栏/对话框全中文化）、HelpOverlay modal（6 tabs, Esc 关闭, ←/→ 导航）、FindDialog modal（Ctrl+F 多行/高级搜索）、大主窗模式（侧栏 toggle）、顶部两层布局保留、`Agent`→`模型`、`Edit`→`编辑模式`；R16 补齐 Address 完整输入弹窗化、聊天室式可折叠任务流与 Task Checklist 公告/展开联动。别名: UI 精简, 中文 TUI, Help 弹窗, 大主窗, AddressDialog, 可折叠任务流。入口: `architecture/CLI_UI_SIMPLIFICATION_SUPPLEMENT_v0.1.md` / `assurance/tui/app.py` / `assurance/tui/widgets.py`
- **Task Checklist Announcement Supplement** (v0.1, 设计约束/2026-07-29 R16 已实施): 主对话顶部公告式 checklist 与展开式工作清单页——Plan mode 批准后生成稳定 step ID 与 plan annotations；常态只显示步骤，展开显示软约束、运行记录和验收引用；Orientation Runtime Guard 只读取中性定位上下文，不询问是否偏移/正确/卡住。2026-07-29 R15 冻结架构文档（268 行）；R16 完成 7 个实施切片（checklist view model → plan mode bridge → announcement strip projection → expanded checklist page → orientation context bridge → journal/artifact registration → GPS mapping），25 checklist tests + 221 TUI tests + 80 plan mode tests pass。别名: checklist 公告, 工作清单, plan annotations, soft workboard, orientation checkpoint context。入口: `architecture/TASK_CHECKLIST_ANNOUNCEMENT_SUPPLEMENT_v0.1.md` / `assurance/task_checklist.py` / `assurance/tui/projector.py` / `assurance/tui/bridge.py`
- **ContentPane Conversation Rendering** (v0.1, 设计约束/2026-07-31 设计冻结): 主窗口内容区对话渲染设计——用户/模型消息使用极简统一边框（`┌─┐│└┘`），工具调用使用无边框 `[工具名]` 标签 + 单行滚动/缩进清单展开，一条视觉分界线隔离自然语言与进程动作。设计参考: Microsoft Comic Chat（消息是视觉单元）、mIRC（角色标签+颜色编码）、现代终端 chat TUI（box-drawing 轻量卡片）。alias: 对话渲染, ContentPane 设计, 聊天气泡, 工具调用行, ToolTraceLine。入口: `architecture/CONTENT_PANE_CONVERSATION_RENDERING_v0.1.md`
- **Session Persistence and Layout Compaction** (v0.1, 设计约束/2026-07-31 设计冻结, 2026-08-01 所有权修正): 布局压缩——Address/Find 折叠至 Toolbar（节省 2 行，按钮激活已有弹窗），双击 Esc 切换 ExplorerPane 为会话列表模式（只读视图，扫描 Grok 产出目录按日期分组）。Session 持久化与恢复由 Grok 拥有；TUI 只提供只读列表发现。alias: session 持久化, 会话恢复, 布局压缩, 双 Esc, Address 折叠。入口: `architecture/SESSION_PERSISTENCE_AND_LAYOUT_COMPACTION_v0.1.md`
- **VS Code 适配、显式 Shell 窗口与终端标题更新** (v0.1, 设计冻结/2026-08-01 待实施): 三大扩展方向——① VS Code 薄桥（ACP-based sidebar webview，只做 assurance 状态卡片 + 命令触发，不重复 Grok 对话渲染）；② `terminal_visibility` 策略（`inline`/`popout`/`vscode`，默认 `popout` 拉起 Windows Terminal 独立窗口）；③ OSC 转义序列终端标题更新（GSA — 就绪/运行中/等待审批/⚠来源不足/完成✓/失败✗）。参考: Claude Code VS Code 扩展（200万+ 安装）、Codex CLI 官方扩展、ACP 标准协议。alias: VS Code integration, ACP bridge, popout terminal, terminal title, OSC 标题。入口: `architecture/VSCODE_SHELL_AND_VISIBILITY_EXTENSIONS_v0.1.md`
- **Plan Mode and Process Usage Monitor** (v0.1, 设计约束/2026-07-29 R15 已实施): 稳定 Plan mode 与运行进程占用监控契约——计划固定四部分（前期调查/具体计划/具体设计/实施方案）、plan approval 与 action approval 分离、root PID 进程树 CPU/MEM/time 采样、系统终端即时显示、VS Code terminal title 2 秒刷新。R15 已交付：`plan_mode.py` (~700 行) — PlanArtifact（4-section structured plan）、PlanVerifier（7 check categories）、PlanStateMachine（8-state lifecycle）、ProcessUsageSampler（psutil 后台采样 + adaptive backoff）、VSCodeTitleUpdater、terminal status line；`plan-mode-result-v0.1.schema.json`；4 个新 TUI event 类型（plan_phase_entered/submitted/approval_decision/usage_sample）；80 tests pass。别名: plan mode, process usage monitor, CPU/MEM, VS Code terminal title。入口: `architecture/PLAN_MODE_AND_PROCESS_USAGE_MONITOR_v0.1.md` / `assurance/plan_mode.py` / `assurance/plan-mode-result-v0.1.schema.json` / `assurance/tests/test_plan_mode.py`
- **Grok Build 适配** (v0.1, 设计约束/2026-07-30 follow-up alignment): 以 Grok Build 为参考运行时的适配策略；旧 V0 中 headless-first、subagent 延后、TUI 延后的表述已按 Runtime-First 裁决修正为 ACP observability 优先、两个检索子代理映射到 Grok-compatible profile/workflow、TUI 消费 normalized runtime state。入口: `architecture/GROK_BUILD_ADAPTATION_v0.1.md` / `docs/RUNTIME_FIRST_FOLLOWUP_ALIGNMENT_2026-07-30.md`
- **Upstream First 集成** (v0.1, 设计约束): 上游优先集成策略——先锁定上游版本再进行适配。入口: `architecture/UPSTREAM_FIRST_INTEGRATION_v0.1.md`
- **Upstream 版本策略** (v0.1, 设计约束): 上游版本锁定、候选评估与升级流程。入口: `architecture/UPSTREAM_VERSION_STRATEGY_v0.1.md`
- **可观测执行信封** (v0.1, 设计约束): 可观测执行包装器设计——将 agent 执行包装为可审计的事件流。入口: `architecture/OBSERVABLE_EXECUTION_ENVELOPE_v0.1.md`
- **成熟 Agent 设计解构** (v0.2, 设计约束): Grok/Codex/Gemini/OpenCode/Goose/Cline 的深度设计分析。入口: `architecture/MATURE_AGENT_DESIGN_DECOMPOSITION_v0.2.md`
- **通用保证内核 Gap 登记** (v0.1, 设计约束): 当前内核与理想状态的功能差距登记。入口: `architecture/GENERAL_ASSURANCE_KERNEL_GAP_REGISTER_v0.1.md`
- **DeepSeek Adapter 合约** (v0.1, 设计约束): DeepSeek API 适配器的合约定义。入口: `architecture/DEEPSEEK_ADAPTER_CONTRACT_v0.1.md`
- **DeepSeek 外部传输安全** (v0.1, 设计约束): 外部 API 调用的传输安全策略。入口: `architecture/DEEPSEEK_EXTERNAL_TRANSPORT_SAFETY_v0.1.md`
- **Windows 运行时合约** (v0.1, 设计约束): Windows 平台的运行时约束与接口定义。入口: `architecture/WINDOWS_RUNTIME_CONTRACT_v0.1.md`
- **Action Kernel 合约** (v0.1, 设计约束): 无模型动作内核的 spike 设计。入口: `architecture/ACTION_KERNEL_CONTRACT_v0.1.md`
- **Model Adapter Loop 合约** (v0.1, 设计约束): 模型适配器循环的设计——适配器与 runner 之间的控制流。入口: `architecture/MODEL_ADAPTER_LOOP_CONTRACT_v0.1.md`
- **Network Permit Broker 合约** (v0.1, 设计约束): 网络许可代理的架构设计。入口: `architecture/NETWORK_PERMIT_BROKER_CONTRACT_v0.1.md`
- **Interactive Approval Ledger 合约** (v0.1, 设计约束): 交互式审批账本——用户审批的持久化与审计。入口: `architecture/INTERACTIVE_APPROVAL_LEDGER_CONTRACT_v0.1.md`
- **Loopback Transport & Private Transcript** (v0.1, 设计约束): 回环传输与私有转录——本地的安全通信通道。入口: `architecture/LOOPBACK_TRANSPORT_AND_PRIVATE_TRANSCRIPT_v0.1.md`
- **D Salvage Matrix** (v0.1, 设计约束): 项目 D 源码挽救矩阵——标识可从旧项目复用的组件。入口: `architecture/D_SALVAGE_MATRIX_v0.1.md`
- **Open Source Agent Gap Audit** (v0.1, 设计约束): 开源 agent 框架的功能差距审计。入口: `architecture/OPEN_SOURCE_AGENT_GAP_AUDIT_v0.1.md`
- **VS Code Extension (ACP-based)** (v0.1, 第一切片已完成/2026-08-01): VS Code 扩展骨架——sidebar webview assurance 状态面板（gates/sources/tools/session 四区卡片），`gsa.verify`/`gsa.reviewGlobal`/`gsa.runGrok` 三个命令，ACP bridge（node-pty + Grok `agent stdio`），Python `vscode_bridge.py` 格式化 assurance 快照。不实现对话渲染/自有 model loop。别名: VS Code 扩展, ACP bridge, sidebar webview。入口: `vscode/` / `assurance/vscode_bridge.py` / `architecture/VSCODE_SHELL_AND_VISIBILITY_EXTENSIONS_v0.1.md`
- **Terminal Title OSC Updates** (v0.1, 已实施/2026-08-01): 终端 tab 标题 OSC 转义序列——`GSA — 就绪`/`运行中 [turn N]`/`等待审批`/`⚠ gate`/`完成 ✓`/`失败 ✗`。`app.update_terminal_title()` + projector 事件触发 + `pt_app` 退出恢复。约30行。别名: terminal title, OSC 标题。入口: `assurance/tui/app.py` / `assurance/tui/pt_app.py` / `assurance/tui/projector.py`
- **Terminal Visibility Strategy** (v0.1, 已实施/2026-08-01): `TerminalVisibility` 枚举（`inline`/`popout`/`vscode`），默认 `popout` 拉起 Windows Terminal 独立窗口。`launch_popout_terminal()` 通过 `Start-Process wt.exe` 实现。CLI `--terminal-visibility` flag。别名: popout terminal, 显式 shell。入口: `assurance/grok_runtime_adapter.py` / `assurance/cli.py`
- **GPS 合约** (v0.1, 设计约束): Global Progress Sentinel 的架构合约。入口: `architecture/GLOBAL_PROGRESS_SENTINEL_CONTRACT_v0.1.md`

### H. ADR (Architecture Decision Records)

> 不可逆的架构决策记录。ADR 一旦记录即生效，修改需新 ADR。

- **ADR-0001** (2026-07, 事实/不可变): 证据约束的本地 agent 内核——产品边界与不变量定义。入口: `adr/ADR-0001-evidence-constrained-local-agent-kernel.md`
- **ADR-0002** (2026-07, 事实/不可变): 推迟云端运行时——显式将云端运行时排除在当前范围外。入口: `adr/ADR-0002-defer-cloud-runtime.md`
- **ADR-0003** (2026-07, 事实/不可变): 运行时中立的保证内核——保证内核不绑定任何特定 LLM 运行时。入口: `adr/ADR-0003-runtime-neutral-assurance-kernel.md`
- **ADR-0004** (2026-07, 事实/不可变): 通用科学 Profile 分层——从 general-science 到 lif-research 的 profile 层次结构。入口: `adr/ADR-0004-general-science-profile-layering.md`

### I. Schema 与合约体系 (Schema & Contract System)

- **assurance/ Schema 文件** (97 个 JSON Schema): 核心保证层的所有数据合约——receipt, manifest, event, envelope, gate receipt, sandbox profile, checkpoint, journal, retrieval, disposable reproduction 等。入口: `assurance/**/*.schema.json`
- **runtime/ Schema 文件** (48 个 JSON Schema): 运行时控制面的数据合约——run-manifest, run-event, scenario-export, leak-scan, CLI session lifecycle (v0.1/v0.2), Codex app server capture/normalization/verification/turn-probe, GPS review/disposition/checkpoint/holistic/transition/controller/state-reduction, journal recovery, deepseek adapter/development, network approval, action kernel, model loop, windows process。入口: `runtime/**/*.schema.json`
- **integration/grok/ Schema 文件** (25 个 JSON Schema): Grok 探针结果的数据合约——observed plan, fake provider, tool continuity, workspace discovery/trust, event bridge, ACP initialize/fake-tool, windows child tree, compaction provenance, real deepseek plan/result/failure。入口: `integration/grok/**/*.schema.json`
- **protocol/ 协议定义** (v0.1, 设计约束): 语言无关的协议草案——agent protocol schema, reason codes, gate matrix, global-progress reason code migration, 协议语义草案。入口: `protocol/agent-protocol-v0.1.schema.json` / `protocol/reason-codes-v0.1.yaml` / `protocol/gate-matrix-v0.1.yaml` / `protocol/PROTOCOL_DRAFT_v0.1.md` / `protocol/global-progress-reason-code-migration-v0.1.yaml`
- **regression/ Schema** (v0.1, 设计约束): 回归测试的完整 schema 体系——case corpus, coverage matrix, fixture, historical excerpt provenance。入口: `regression/case-corpus-v0.1.schema.json` / `regression/coverage-matrix-v0.1.schema.json` / `regression/fixture-v0.1.schema.json` / `regression/historical-excerpt-provenance-v0.1.schema.json` / `regression/cases-v0.1.yaml`
- **evaluation/ Schema** (v0.1, 设计约束): 评估结果与分区清单的 schema。入口: `evaluation/evaluation-result-v0.1.schema.json` / `evaluation/partition-manifest-v0.1.schema.json`
- **upstream/ Schema** (v0.1, 设计约束): 上游构建锁与候选版本的 schema。入口: `upstream/grok-build-lock-v0.1.schema.json` / `upstream/grok-build-candidate-v0.1.schema.json` / `upstream/grok-build.lock.json` / `upstream/grok-build.candidate.json`

### J. 评估与回归 (Evaluation & Regression)

- **Scoring Protocol** (v0.1, 设计约束): cluster-macro 评分、红线规则、盲审协议。入口: `evaluation/SCORING_PROTOCOL_v0.1.md`
- **Partition & Oracle Isolation** (v0.1, 设计约束): 评估分区与 oracle 物理隔离协议。入口: `evaluation/PARTITION_AND_ORACLE_ISOLATION_v0.1.md`
- **Human Baseline Protocol** (v0.1, 设计约束): 双盲人类基线的评估协议。入口: `evaluation/HUMAN_BASELINE_PROTOCOL_v0.1.md`
- **Regression Case Corpus** (v0.1, 事实): 30 个历史 + 10 个合成反惯性案例的回归语料库。入口: `regression/cases-v0.1.yaml`
- **Coverage Matrix** (v0.1, 事实): 10 个错误簇的覆盖矩阵。入口: `regression/coverage-matrix-v0.1.yaml`
- **Curation Protocol** (v0.1, 设计约束): 回归案例的策展协议。入口: `regression/CURATION_PROTOCOL_v0.1.md`
- **Self-Question Counterexample Design** (2026-07-26, 设计约束): 自问反例设计——GSA 的自检能力边界验证。入口: `docs/GSA_SELF_QUESTION_COUNTEREXAMPLE_DESIGN_2026-07-26.md`
- **General Scientific Assurance Gap Review** (2026-07-25, 事实): 通用科学保证的差距审查。入口: `docs/GENERAL_SCIENTIFIC_ASSURANCE_GAP_REVIEW_2026-07-25.md`
- **Cross-Artifact Lineage Comparability** (2026-07-25, 事实): 跨制品的 lineage 可比性审计。入口: `docs/GSA_CROSS_ARTIFACT_LINEAGE_COMPARABILITY_AUDIT_2026-07-25.md`

### K. Grok 探针系列 (Grok Probes)

> 对参考运行时 Grok Build 的系统性探针测试。每项探针验证 Grok 的一个具体行为维度。

- **Grok Windows Binary** (2026-07-21, 事实): Grok 在 Windows 上的二进制执行审计。入口: `docs/GROK_WINDOWS_BINARY_AUDIT_2026-07-21.md`
- **Grok ACP Initialize** (2026-07-21, 事实): ACP 协议初始化探针——验证 initialize 握手。入口: `docs/GROK_ACP_INITIALIZE_AUDIT_2026-07-21.md`
- **Grok ACP Fake Tool** (2026-07-23, 事实): ACP 假工具 spike——验证工具调用的协议层行为。入口: `docs/GROK_ACP_FAKE_TOOL_SPIKE_2026-07-23.md`
- **Grok Fake Provider** (2026-07-21, 事实): 假模型提供者探针——验证 provider 抽象边界。入口: `docs/GROK_FAKE_PROVIDER_AUDIT_2026-07-21.md`
- **Grok Event Bridge** (2026-07-21, 事实): 事件桥探针——验证 agent 事件流的完整性。入口: `docs/GROK_EVENT_BRIDGE_AUDIT_2026-07-21.md`
- **Grok Compaction Provenance** (2026-07-23, 事实): 压缩溯源——验证上下文压缩不丢失关键来源信息。入口: `docs/GROK_COMPACTION_PROVENANCE_2026-07-23.md`
- **Grok Workspace Trust** (2026-07-21, 事实): 工作区信任探针。入口: `docs/GROK_WORKSPACE_TRUST_AUDIT_2026-07-21.md`
- **Grok Workspace Discovery / Fixture Checkpoint Delta** (2026-07-21, 事实): 工作区发现机制 + 夹具检查点 delta spike。入口: `docs/GROK_FIXTURE_CHECKPOINT_DELTA_SPIKE_2026-07-21.md`
- **Grok Windows Child Tree** (2026-07-23, 事实): Windows 子进程树探针——验证进程树追踪。入口: `docs/GROK_WINDOWS_CHILD_TREE_PROBE_2026-07-23.md`
- **Grok Tool Continuity** (2026-07-21, 事实): 工具连续性探针——验证跨轮工具状态保持。入口: `docs/GROK_TOOL_CONTINUITY_AUDIT_2026-07-21.md`
- **Grok Real DeepSeek Launcher** (2026-07-23, 事实): 真实 DeepSeek 模型启动器——Grok 框架内调用真实模型。入口: `docs/GROK_REAL_DEEPSEEK_LAUNCHER_2026-07-23.md`
- **Grok Upstream Candidate** (2026-07-23, 事实): 上游候选版本评估。入口: `docs/GROK_UPSTREAM_CANDIDATE_AUDIT_2026-07-23.md`
- **Grok Upstream Promotion** (2026-07-23, 事实): 上游版本升级。入口: `docs/GROK_UPSTREAM_PROMOTION_2026-07-23.md`

### L. 原型代码 (Prototype)

> 旧原型子系统 `fep_agent_proto`——显式标记为 **不作为生产 runner**。保留用于设计参考。

- **FEP Agent Proto** (2026-06~07, 废弃/参考): 旧原型包——包含 action kernel, DeepSeek adapter/client, model transport, loopback mock server, network broker, approval ledger, journal (with lock/recovery), CLI session lifecycle, codex app server capture, global progress state 等模块。状态: 废弃/仅设计参考。入口: `prototype/fep_agent_proto/`
- **D Salvage Matrix** (v0.1, 设计约束): 标识可从此原型挽救到主项目的组件。入口: `architecture/D_SALVAGE_MATRIX_v0.1.md`

### M. 脚本与工具 (Scripts & Tools)

- **Repository Check** (2026-07, 事实): CI 仓库完整性检查脚本——schema/corpus/coverage/fixture 全面验证。入口: `scripts/check_repository.py`
- **GPS 脚本系列**: 构建/验证/追加/归约 GPS 状态的脚本。入口: `scripts/build_gps_*.py` / `scripts/verify_gps_*.py`
- **Grok 探针脚本**: Grok 调用的 PowerShell 启动器 (`invoke_grok_*.ps1`, 13 个) 与 Python 验证器 (`verify_grok_*.py`, 7 个)。入口: `scripts/`
- **DeepSeek 脚本**: DeepSeek public output observation 的 builder 与 launcher。入口: `scripts/build_deepseek_public_output_observation.py` / `scripts/build_deepseek_stream_observation_fixture.py` / `scripts/invoke_deepseek_public_output_observation.ps1`
- **Windows Sandbox Probe** (2026-07-27, 事实): Windows 原生沙箱的探针运行器。入口: `scripts/run_windows_native_sandbox_probe.py`
- **Archive Journal Recovery** (2026-07, 事实): 归档日志恢复脚本。入口: `scripts/recover_archive_journal.py`

### N. CI 与工程配置 (CI & Engineering)

- **CI Pipeline** (2026-07, 事实): GitHub Actions——Ubuntu + Windows, Python 3.11/3.12, 多步测试（repository check, compileall, PowerShell 语法检查, 单元测试）。入口: `.github/workflows/ci.yml`
- **pyproject.toml** (2026-07, 事实): 包构建配置——`gsa-assurance` 包, `gsa = "assurance.cli:main"` 入口点, jsonschema + rfc8785 依赖。入口: `pyproject.toml`
- **pytest.ini** (2026-07, 事实): Pytest 配置——`asyncio_mode = strict`, function-scoped fixtures。入口: `pytest.ini`
- **.claude/settings.local.json** (2026-07, 事实): Claude Code 本地权限配置。入口: `.claude/settings.local.json`

### O. 项目级文档 (Project-Level Documents)

- **README.md** (2026-07, 事实): 项目主 README——状态总结、P 级合约概览、Gate 链路描述、当前边界与已知限制。中文。入口: `README.md`
- **assurance/README.md** (2026-07, 事实): 保证内核包 README——P0-P5 合约摘要、权威所有权 ADR 引用、当前合同列表。入口: `assurance/README.md`
- **integration/grok/README.md** (2026-07, 事实): Grok 集成目录 README。入口: `integration/grok/README.md`

### P. 测试基础设施 (Test Infrastructure)

- **assurance/tests/** (67 个测试文件): 每个核心模块对应 1 个测试文件——P0-P5 合约、Gate 链路 (instruction provenance, tool availability, retrieval subagent)、orientation guard、DeepSeek adapter/observation、canonical CLI、Grok prompt/tool promotion gate、sandbox verifier、archive controller、adapter gate/integration、runner scoring、key lifecycle、trust/child/network、windows sandbox/race escape、endpoint canonicalizer、disposable reproduction、source visibility、general science review。入口: `assurance/tests/`
- **runtime/tests/** (8 个测试文件): 运行时控制面测试——GPS sentinel/holistic/checkpoint/transition、CLI session lifecycle adapter、Codex app server lifecycle normalizer/capture/turn probe。入口: `runtime/tests/`
- **integration/grok/tests/** (10 个测试文件): Grok 探针测试——workspace trust, event bridge, fixture workspace capture, ACP initialize/fake-tool, windows child tree, timeout gate split, fake provider, compaction provenance, real deepseek launcher。入口: `integration/grok/tests/`
- **assurance/fixtures/** (P0/P1/P5/general_science/source_visibility): 测试夹具数据——valid/invalid/semantic-invalid JSON 文件，覆盖各 P 级合约、GSA-CORE computational_decay 示例、source visibility ledger。入口: `assurance/fixtures/`
- **runtime/fixtures/** (GPS + Codex): GPS 输入与 disposition、Codex app server lifecycle 捕获 (happy/failed/interrupted/truncated/cross-thread/duplicate)。入口: `runtime/fixtures/`
- **regression/fixtures/** (10 个案例): 5 个合成反惯性 (FEP-SYN-006~010) + 5 个历史摘录 (FEP-REG-009/015/016/025/029)，各含 evidence/fixture/provenance。入口: `regression/fixtures/`

---

## 状态路由 (Status Routing)

> 按组件状态分类的 ID 速查列表。**Prior-existence scan 第一步**：先查此表确认某组件是否已有同义/相反/撤回/废弃条目。

### 已撤回 / 已废弃 (Withdrawn / Deprecated)

- **FEP Agent Proto** (废弃): 旧原型包 `prototype/fep_agent_proto/` 不作为生产 runner。仅保留用于设计参考。入口: `architecture/D_SALVAGE_MATRIX_v0.1.md`
- **旧 P2 CLI 入口** (废弃): 部分旧 CLI 入口点已被 canonical CLI 替代。所有新开发应使用 `assurance/canonical_cli.py` 的 Gate 序列。入口: `assurance/p2_cli.py` / `assurance/p45_cli.py` / `assurance/p5_cli.py`（仅作向后兼容保留）

### 保留但重分类 (Retained / Reclassified)

- **Canonical Guarded CLI** (保留为 fixture+assurance path): 不再作为最终 production agent runtime 叙事；后续 Grok 嫁接后用于 conformance、离线回归、Gate 行为证明。入口: `assurance/canonical_cli.py` / `docs/CANONICAL_GUARDED_CLI_P0_AUDIT_2026-07-26.md`
- **Local Browser Retrieval / PDF Evidence** (保留为 assurance-owned): 主检索证据链保留；必须通过显式 `local_browser` 模式启用，framework 检索只作 fallback。入口: `architecture/LOCAL_BROWSER_RETRIEVAL_AND_PDF_EVIDENCE_v0.1.md` / `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md`
- **Retrieval Subagents** (保留为两个固定子代理): 项目文档检索与外部检索保留；子代理关闭前做中性 completion check，确认是否已获得当前主任务所需内容；后续优先映射到 Grok profiles/workflows，不新增本地 scheduler。入口: `assurance/retrieval_subagent.py` / `docs/RETRIEVAL_SUBAGENT_AUDIT_2026-07-27.md`
- **Counterexample / Neutral Inquiry** (保留但触发分离): 反例询问只在 plan/conclusion 写入前；中立询问在执行中辅助全局回看。入口: `docs/GSA_SELF_QUESTION_COUNTEREXAMPLE_DESIGN_2026-07-26.md` / `docs/ORIENTATION_RUNTIME_GUARD_AUDIT_2026-07-28.md`
- **Diagnostic Coverage Check** (保留为中立询问子机制): 单个 bug/debug episode 内硬信号递进触发；每次触发后计数清零、阈值 +1，bug 解决后重置。用户继续不覆盖；明确新证据可降噪。入口: `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md`
- **Codex App Server / VS Code Internal Terminal Lifecycle** (保留为生命周期观测): 保留初步适配证据，不升级为完整 VS Code 插件。入口: `docs/CODEX_APP_SERVER_LIFECYCLE_NORMALIZER_AUDIT_2026-07-25.md`

### 待嫁接 / Runtime-First In Progress

当前待办顺序（2026-07-31 更新）:

1. **Grok `0.2.112` Candidate Gate** (已完成/已提升): PATH 已发现并登记 `0.2.112 (9bbd559437) [stable]`；candidate metadata、binary identity、static Grok integration tests、TUI baseline、repository regression、ACP initialize、fake tool allow/cancel、child-tree task_cancel/parent_exit、compaction provenance 均已通过；Windows child-tree timeout 已拆为 `windows_child_tree_baseline_regression`（passed）与 `windows_child_tree_owned_cleanup`（carried-limitation，2026-07-31 containment 裁定后不再构成 prompt/tool 阻断）。默认 lock 已更新为 `0.2.112`。入口: `docs/GROK_0_2_112_CANDIDATE_GATE_2026-07-30.md` / `docs/GROK_0_2_112_TIMEOUT_TRIAGE_2026-07-30.md` / `docs/GROK_CLI_SPECIALIZATION_ADAPTATION_AUDIT_2026-07-30.md` / `upstream/grok-build.candidate.json` / `upstream/grok-build.lock.json` / `scripts/verify_grok_timeout_gate_split.py`
2. **Grok `0.2.112` Upgrade Value Breakdown + Timeout Triage Decision** (已完成/已提升): `0.2.112` 更新集中在 background task lifecycle、terminal output capture、workflow/subagent、MCP inheritance、permission/search controls、Windows auth/provider helpers 和 session resume/fork；升级价值足够高，且 `tool_timeout` 不是新版本回归，因此按 carried limitation 提升。入口: `docs/GROK_0_2_112_UPGRADE_VALUE_BREAKDOWN_2026-07-30.md` / `docs/GROK_0_2_112_TIMEOUT_TRIAGE_2026-07-30.md`
3. **Grok Runtime Adapter + Event Normalizer** (第一切片已完成, 2026-07-31 全面升级; 2026-08-01 ACP receipt/schema 补齐): 已新增薄 adapter 与事件正规化层；覆盖锁定 Grok binary inspection、workspace trust 先行、隔离 profile/temp、`grok --version` headless smoke、ACP `session/prompt` JSON-RPC smoke（`run_grok_acp_once()` — 主可观测路径）、metadata-only stdout/stderr artifact、canonical runtime event JSONL、TUI-ready metadata projection，并强制记录 `no_residue_required` / `no_residue_observed`。2026-08-01: ACP smoke receipt 已补齐 schema 合规的 prompt / acp / containment / artifact 字段，runtime event schema 已登记 ACP lifecycle 与 prompt response 事件。全部 Grok 进程启动路径已升级至 `CREATE_SUSPENDED` containment（等价 GAK-WIN-001 `PROC_THREAD_ATTRIBUTE_JOB_LIST`）。该切片不实现自有 model loop/tool dispatcher/session runtime（属 Grok-owned）。入口: `docs/GROK_CLI_SPECIALIZATION_ADAPTATION_AUDIT_2026-07-30.md` / `assurance/grok_runtime_adapter.py` / `assurance/grok_event_normalizer.py` / `assurance/grok-runtime-receipt-v0.1.schema.json` / `runtime/run-event-v0.1.schema.json`
4. **CLI / TUI Grok Wiring** (第一切片已完成, 2026-07-31 ACP 接入; 2026-08-01 显式 ACP smoke CLI 入口): 已新增 `gsa grok doctor`、`gsa grok run --mode version-smoke`、`gsa grok run --mode acp-smoke --ask ...`、`gsa tui --runtime grok --run version-smoke`，以及 `gsa run --runtime grok` fail-closed prompt/tool promotion gate。默认 Grok smoke 使用 `run_root/workspace` 干净隔离目录；显式传入含控制面的项目根会被 restricted workspace trust 正确拒绝。2026-07-31 新增 `--grok-execute` + `--grok-mode`（默认 `acp-smoke`，ACP JSON-RPC 主路径；`prompt-smoke` 为窄 smoke 备选），promotion gate `decision: allow` 时可执行真实 Grok session。`gsa run` 默认仍为 canonical 路径；`--runtime grok` 执行需显式 `--grok-execute`；`gsa grok run --mode acp-smoke` 是显式 adapter smoke，不等同于默认产品提升。入口: `docs/GROK_CLI_SPECIALIZATION_ADAPTATION_AUDIT_2026-07-30.md` / `assurance/cli.py` / `assurance/grok_prompt_tool_gate.py` / `assurance/tui/main.py` / `assurance/tui/bridge.py`
5. **Global Review Mode** (第一切片已完成/2026-07-30 通用化修正): 已新增 `gsa review global` 和 `global_review_mode_receipt`，固定五个通用全局审查维度：原设计理念、当前进度判断、当前实现内容定位、关键设计保留、项目任务边界。默认可从 git status 收集范围，也可 `--no-git-status --path ...` 显式指定；输出只表示全局审查模式已激活并界定范围，不替代最终审查判断，不影响日常局部 engineering review，也不专门服务 CLI/Grok。入口: `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md` / `assurance/global_review_mode.py` / `assurance/global-review-mode-receipt-v0.1.schema.json` / `assurance/tests/test_global_review_mode.py`
6. **Runtime-First Follow-up Alignment** (对齐记录已完成/前六项后续切片已完成, 2026-07-31 全面更新): 已新增 follow-up alignment 记录，明确两个 retrieval subagent 只映射到 Grok-compatible profile/workflow、不新增本地 scheduler；retrieval mode 固定为 `local_browser` / `framework_fallback` / `off`。已新增恰好两个 Grok project agent profile drafts：`.grok/agents/gsa-project-doc-retrieval.md` 与 `.grok/agents/gsa-external-retrieval.md`；P2 workflow Rhai 语法已验证并编写 `.grok/workflows/gsa-retrieval.rhai`。已新增 `gsa grok observe-tools --include-tool-availability`、`grok_tool_permission_observation_receipt` 和 Grok availability projection：静态观察可发现 registry/permission/web-disable/tool-filter controls；P3 ACP permission dual verification（`allow_once` + `cancel_permission`）已闭合。已新增 `grok_lifecycle_projection`：支持 metadata-only Grok workflow/subagent lifecycle 事件投影为 GPS journal events 和中性 Orientation context。已新增 timeout split verifier：`windows_child_tree_baseline_regression` passed，`windows_child_tree_owned_cleanup` carried-limitation（containment 裁定后不再构成阻断）。已新增 `gsa run --runtime grok` fail-closed promotion gate：dual ACP + adapter containment → `decision: allow`；`--grok-execute` 可执行真实 session。旧 `prototype/` 文档入口改为 retired/reference。入口: `docs/RUNTIME_FIRST_FOLLOWUP_ALIGNMENT_2026-07-30.md` / `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md` / `architecture/INTEGRATED_AGENT_ASSURANCE_DESIGN_v0.1.md` / `architecture/GROK_BUILD_ADAPTATION_v0.1.md` / `.grok/agents/` / `assurance/grok_profile_drafts.py` / `assurance/grok_tool_permission_observer.py` / `assurance/grok_lifecycle_projection.py` / `assurance/grok_prompt_tool_gate.py` / `scripts/verify_grok_timeout_gate_split.py`
7. **Adapter Job Object Containment (P1)** (第一切片已完成/2026-07-30, 2026-07-31 CREATE_SUSPENDED 全面升级 + 裁定闭合): 已新增 `assurance/job_object_supervisor.py` — adapter 侧 Kill-On-Close Job Object 进程树包裹。`JobObjectSupervisor` 用 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` 包裹 Grok 根进程，adapter 关闭 Job handle 时 kernel 杀死全部子进程树。全部 Grok 进程启动路径（`contained_run()`、`run_grok_headless_once()`、`run_grok_acp_once()`、`run_with_job_object_containment()`）已升级至 `CREATE_SUSPENDED` + `AssignProcessToJobObject` + `ResumeThread` 模式，闭合 post-creation 竞态窗口。Containment 裁定：此模式与 GAK-WIN-001 `PROC_THREAD_ATTRIBUTE_JOB_LIST` 在 containment 保证上等价；后者仅为代码质量优化（更少 syscall），非安全性必需。`grok_prompt_tool_gate.py` 新增 `adapter_containment_provided` 路径 — 可与 `windows_child_tree_owned_cleanup` 互替满足 `containment_requirement_satisfied`。CLI 新增 `--grok-adapter-receipt` 参数。已更新 schema（grok-runtime-receipt containment 块 + grok-prompt-tool-promotion-gate-receipt adapter_containment 前置条件）。入口: `assurance/job_object_supervisor.py` / `assurance/grok_runtime_adapter.py` / `assurance/grok_prompt_tool_gate.py` / `assurance/tests/test_job_object_supervisor.py` / `docs/JOB_OBJECT_CONTAINMENT_SUFFICIENCY_JUDGMENT_2026-07-31.md`
8. **Grok Workflow Rhai Verification (P2)** (第一切片已完成/2026-07-30, 2026-07-31 P3 前置已闭合): Rhai 语法已从 Grok 0.2.112 bundled `create-workflow` skill 获取完整 reference——workflow 为 `.rhai` 文件，语法使用 `let meta = #{...}` 纯字面量 meta header + `agent()`/`parallel()`/`phase()`/`complete()` host API。已编写 `.grok/workflows/gsa-retrieval.rhai`（parallel project-doc + external retrieval workflow）。`grok_profile_drafts.py` 使用 `_validate_rhai_workflow_structure()` 结构检查（meta header + name/description + agent/parallel 调用）。`grok inspect --json` 确认两个 agent profile 可发现。前置依赖 P3 ACP permission dual verification 已闭合；workflow smoke-check 现可通过 `--grok-execute` 触发。入口: `.grok/workflows/gsa-retrieval.rhai` / `assurance/grok_profile_drafts.py` / `assurance/tests/test_grok_profile_drafts.py`
9. **ACP Permission Dual Verification (P3)** (第一切片已完成/2026-07-30, 2026-07-31 ACP 执行路径接入): ACP fake-tool 双重验证 end-to-end 闭合。`grok_tool_permission_observer.py` 已要求 `REQUIRED_ACP_PERMISSION_SCENARIOS = ("allow_once", "cancel_permission")` 两者同时 valid 才给出 `decision: "allow"` 和 `gate_decision: "allow"`。CLI 新增 `gsa grok observe-tools --output-gate-receipt <path>` 将 tool availability gate receipt 写入文件供 promotion gate 消费。Promotion gate 端到端测试 `test_full_promotion_chain_allow_with_dual_acp_and_containment` 证明：dual ACP + adapter containment → `decision: allow`（无 blocking reasons）。`gsa run --runtime grok` gate 满足时可通过 `--grok-execute` 执行真实 Grok session（默认 `acp-smoke`，ACP JSON-RPC 主路径）。入口: `assurance/grok_tool_permission_observer.py` / `assurance/grok_prompt_tool_gate.py` / `assurance/cli.py` / `assurance/tests/test_cli_dispatcher.py`
- **Subprocess Containment (2026-07-30, P1 补充)**: `JobObjectSupervisor` containment 已扩展到所有 Grok subprocess 调用路径。新增 `contained_run()` 便捷函数（drop-in `subprocess.run` 替代），在 `observe_grok_tool_permission_surfaces()`（3 次 Grok 调用）和 `inspect_grok_runtime()` / `_workspace_trust()`（PowerShell 脚本调用）中启用 Job Object 包裹。即使 subprocess timeout 未触发，Job handle close 确保 kernel 终止全部进程树，消除 Grok 子进程泄露风险。测试可通过 `CONTAINMENT_ENABLED = False` 禁用 containment。发现记录和根因分析见 observer subprocess leak audit。入口: `assurance/job_object_supervisor.py` / `assurance/grok_tool_permission_observer.py` / `assurance/grok_runtime_adapter.py` / `docs/GROK_OBSERVER_SUBPROCESS_LEAK_2026-07-30.md`

### 已关闭 (Closed)

- **GAK-UI-001 CLI UI Interaction Model** (2026-07-28 第一原型完成, 2026-07-29 Phase 2-4 全部完成): 9 个模块 ~3900 行。Phase 2：canonical CLI 事件流桥接（`bridge.py`、`LiveRunEventSource`、`JsonlFileSource`、3 个 typed gate event、`--run`/`--replay`）。Phase 3：真实 DeepSeek adapter 接入（`--real`、`build_live_run_fn(real_adapter=True)` 全链路贯通）。Phase 4 (R15)：默认中文 UI（菜单/工具栏/状态栏/对话框全中文化）+ HelpOverlay modal (6 tabs, Esc 关闭, ←/→ 导航) + FindDialog modal (Ctrl+F 多行/高级搜索) + 大主窗模式（侧栏 toggle）+ `Agent`→`模型` + `Edit`→`编辑模式`。221 TUI tests pass。入口: `architecture/CLI_UI_INTERACTION_MODEL_v0.1.md` / `assurance/tui/`
- **LBR-001 Local Browser Retrieval and PDF Evidence** (2026-07-29 关闭): Phase 1-4 全链路完成 + R9 闭合。Evidence Store (42 tests)、CDP 客户端 (27 tests)、检索工作流 (34 tests)、TUI 集成 (217 tests)。R9 新增: ① LiveBrowserTests 自启动 headless Chrome——不再需手动启动 Chrome，`setUpClass` 通过 `_ensure_browser()` 自动启停；skip 条件从 "Chrome 未运行" 改为 "Chrome 未安装"（4 tests pass, 0 skip）。② E2E 集成测试 `test_browser_retrieval_e2e.py` (6 tests)：Web 检索 5 (retrieve_urls 单页/多页/空列表/错误处理/progress callback) + 论文 PDF 检索 1 (run_retrieval arXiv → 下载 → 验证 → 证据库，验证 SHA-256、PDF header、metadata.json、pages.jsonl)。③ 修复 `classify_page` login 检测——"log in"/"login" 从关键词列表移除（arXiv 等网站导航栏中的 login 链接不再误触发 LOGIN_REQUIRED），改为仅匹配强信号 ("please log in to", "log in to continue", "authentication required" 等)。④ 测试可靠性加固——httpbin.org 替换为 example.com/example.org 消除间歇性 WebSocket 超时。全量 71 tests pass (0 skip)。已知限制: Chrome 主 profile 受企业安全策略拦截 CDP 连接，workaround 使用项目隔离 profile (`.gsa_chrome_profile/`)。入口: `architecture/LOCAL_BROWSER_RETRIEVAL_AND_PDF_EVIDENCE_v0.1.md` / `assurance/browser_retrieval.py` / `assurance/retrieval_workflow.py` / `assurance/tests/test_browser_retrieval.py` / `assurance/tests/test_retrieval_workflow.py` / `assurance/tests/test_browser_retrieval_e2e.py`
- **GAK-TRUST-001 Workspace Trust 统一入口** (2026-07-28 关闭): `establish_workspace_trust()` 已接入 canonical CLI 和检索子代理全部入口点。`AdapterGateContext` 携带 trust receipt；`_resolve_and_setup_gates()` 在 IPG 前建立 trust；`validate_all_entry_points_establish_trust()` AST 审计覆盖全部入口点。入口: `assurance/workspace_trust.py` / `assurance/adapter_gate.py` / `assurance/canonical_cli.py`
- **GAK-CHILD-001 Child Capability 统一传递** (2026-07-28 关闭): 新增 `spawn_child_context()` 统一入口（组合 predicate check + 签名 child envelope 创建）；检索子代理接受可选 `parent_envelope`，通过 `spawn_child_context()` 创建签名 child envelope 并传入 `ConversationNamespace.create(parent_envelope_id=...)`。入口: `assurance/child_capability_enforcer.py` / `assurance/retrieval_subagent.py`
- **GAK-NET-001 Network Permit 统一覆盖** (2026-07-28 关闭): `call_deepseek_api()` 在 HTTP 请求前调用 `evaluate_network_permit()`；`AdapterGateContext` 携带 network_policy/endpoint/category；`_resolve_and_setup_gates()` 构建 `guarded` 模式默认 policy。入口: `assurance/deepseek_adapter.py` / `assurance/adapter_gate.py` / `assurance/canonical_cli.py`
- **GAK-ID-001 Key Rotation/Revocation 产品化** (2026-07-28 关闭): `_FileLock` 文件锁保护所有 key mutation；crash-safe journal（pending → receipt → history → clear）；`recover_pending_rotations()` 恢复；`migrate_envelope()` 真实 envelope 重签名；`rotate()` 支持 `envelopes_to_migrate`。入口: `assurance/key_lifecycle.py` / `assurance/envelope.py`
- **GAK-SBX-001 Windows Native Sandbox** (2026-07-27 关闭): Development baseline 已达成——elevated 路径 compliant（netsh firewall + AppContainer + Job Object），non-elevated 路径 fail-closed。原始 TCP 残余记录为已知 Windows 平台限制，不阻塞 development 门禁。入口: `docs/GAK_SBX_001_WINDOWS_NATIVE_SANDBOX_AUDIT_2026-07-27.md`
- **GAK-WIN-001 Windows Job Object Race** (2026-07-28 关闭, 2026-07-31 补充裁决): `PROC_THREAD_ATTRIBUTE_JOB_LIST` 内核级 Job Object 原子绑定——进程创建时在内核中分配 Job，消除 `CreateProcess`→`AssignProcessToJobObject` 用户态竞态窗口。`CREATE_SUSPENDED` + `TokenIsAppContainer` + `IsProcessInJob` 在 `ResumeThread` 前全部验证。`PROC_THREAD_ATTRIBUTE_JOB_LIST` 不可用时优雅降级至 post-creation 分配。11 tests pass, 1 skipped（旧 OS 自动跳过）。入口: `assurance/windows_sandbox.py:856-983` / `assurance/tests/test_windows_race_escape.py:383-970`
- **GSA-PROFILE-001 Profile Registry 全量注册** (2026-07-28 关闭): 5 个 profile (general-code, restricted-review, headless-ci, general-science, lif-research) 已完成全量注册——全部 profile 包含 reference runtimes 与 evidence refs；新增 `verify_profile_registry_completeness()` 完整性校验（cross-reference extensions/capabilities 与实际 assurance 模块）；6 个新增测试。入口: `assurance/profile-registry-v0.1.json` / `assurance/profile_registry.py`
- **GAK-INJ-001 Instruction Provenance Gate 产品化** (2026-07-28 关闭): 新增 production content parser + obfuscation detector + canonicalizer 集成；25 个注入模式；adversarial injection 47 测试 + bypass hardening 8 测试。89 tests pass。入口: `assurance/instruction_provenance_gate.py` / `assurance/tests/test_injection_adversarial.py`
- **GAK-CMP-001 Automatic Compaction Observation** (2026-07-29 关闭): 新增 `compaction_observer.py` — `AutomaticCompactionSimulator`（自动阈值触发，boundary zone honest unknown）+ `ManualCompactionSimulator` + 6 个 invariant validator。36 tests pass。核心不变性：`classification_confidence < 1.0` 时必须有 unknown range；unknown 不可静默升级为 retained/discarded。`AUTOMATIC_THRESHOLD_NOT_OBSERVED` → `AUTOMATIC_THRESHOLD_OBSERVED`。入口: `assurance/compaction_observer.py` / `assurance/tests/test_compaction_observer.py`
- **GAK-REC-001 Shadow Recovery Store & Executor** (2026-07-29 关闭): 新增 `shadow_recovery.py` — `ShadowRecoveryStore`（SHA-256 内容寻址 store/retrieve/verify/list）+ `RecoveryDiffPreview`（元数据级 diff，不泄露内容）+ `RecoveryExecutor`（唯一执行恢复的组件，原子写入 + 签名 execution receipt，审计事件不删除）。23 tests pass。入口: `assurance/shadow_recovery.py` / `assurance/tests/test_shadow_recovery.py`
- **GAK-UX-001 UX Safety Validation Framework** (2026-07-29 关闭): 新增 `ux_safety.py` — `UXSafetyEvaluator` + 10 个 UX 安全场景（4 novice + 3 experienced + 3 shared），8 种 UXAssertionKind。签名 `ux_safety_evaluation_receipt`。20 tests pass。全部 8 个中等严重度 Gap 全部关闭。入口: `assurance/ux_safety.py` / `assurance/tests/test_ux_safety.py`
- **GAK-RET-001 Retention/Deletion Controller 产品化** (2026-07-28 关闭)
- **GAK-SESSION-001 Conversation Namespace → Real Runtime** (2026-07-28 关闭): `ConversationNamespace` 已接入 canonical CLI 真实 adapter 路径。新增 `SessionGovernor`；`run_canonical_guarded_cli_real()` 通过 `enforce_adapter_call()` 包装 DeepSeek API 调用；10 个 integration 测试。548 tests pass。入口: `assurance/session_governor.py` / `assurance/tests/test_session_namespace.py`: StorageAdapter 已接入 ArchiveController（替代裸 DeleteFile 回调）；archive_recovery.py 提供 holistic 恢复编排器（classify_archive_failure → recover_archive，覆盖 clean_interrupted/torn_journal/corrupt_journal/stale_lock/archiving_no_journal 五个恢复路径）；detect_stale_archive_lock + cleanup_stale_archive_lock 处理崩溃后孤儿锁；FaultInjectionStorageAdapter 支持故障注入测试；新增 18 个测试。484 tests pass。入口: `assurance/archive.py` / `assurance/archive_recovery.py` / `assurance/archive_journal.py` / `assurance/tests/test_archive_recovery.py` / `assurance/tests/storage_faults.py`

### 待实施 / 进行中 (Pending / In Progress)

> 2026-07-30 设计优点落实差距分析新增。以下条目来自《成熟 Agent 设计深拆》(`MATURE_AGENT_DESIGN_DECOMPOSITION_v0.2`) 和《Grok Build 适配矩阵》(`GROK_BUILD_ADAPTATION_v0.1`) 中已确定"应吸取/应实现"但尚未完全落实到自有代码中的设计优点。当前活跃待办以上方 **待嫁接 / Runtime-First In Progress** 顺序为准，以下为新增补充。

#### D1. 部分实现需补齐 (Partial → Complete)

10. **ACP 实时事件流 + 多 prompt 会话循环 + 工具结果可视化 + MCP 配置** (已关闭/2026-07-31): `GrokAcpSession` 类封装完整 ACP 生命周期（initialize→session/new→{send_prompt × N}→close），同一 `sessionId` 支持多次 `session/prompt`；`LiveRunEventSource.send_prompt()` 非阻塞通道 + TUI 地址栏非命令文本自动转发；`_handle_acp_notification` 扩展支持 `assistant_message`/`message`/`tool_call_update`/`tool_result`/`error`；projector 新增 ACP lifecycle handlers + 实时文本/工具/工具结果显示。`GrokRunRequest.mcp_servers` + `--mcp-config` CLI 参数传入 Grok `session/new`。入口: `assurance/grok_runtime_adapter.py` (GrokAcpSession class) / `assurance/tui/bridge.py` / `assurance/tui/event_source.py` / `assurance/tui/app.py`
11. **Shadow Git 改为实际 Git Repo** (已关闭, 2026-07-31): `shadow_recovery.py` 已从 SHA-256 CAS 文件系统完全重构为独立 Git repository（借鉴 Gemini CLI / OpenCode / Cline checkpointing）。`ShadowRecoveryStore` 每个 store 调用创建 Git commit（candidate.json + authorization.json + snapshot.bin 作为 blobs），commit_sha（40-char hex）是主键，tree-SHA 去重实现幂等。`ExecutionReceipt` 携带 `shadow_commit_sha`。Git 对象模型提供完整性验证（`verify_entry` 通过 `git cat-file -t` 校验 commit/tree/blob 链）。`RecoveryDiffPreview` 提供元数据级 diff。旧 SHA-256 CAS 格式已废弃（DeprecationWarning）。24 tests pass。入口: `MATURE_AGENT_DESIGN_DECOMPOSITION_v0.2` §4.1, §5 / `assurance/shadow_recovery.py`
12. **Post-Run Session File 交叉核验** (第一切片已完成/2026-07-31, 已关闭): 设计文档要求 runtime event ↔ session file 的 post-run 对账。`verify_acp_session()` 已落地（158f5e4）——7 项机械检查（acp_lifecycle_complete、session_id_consistent、tool_call_count_match、permission_count_match、permission_outcomes_match、transcript_has_expected_lifecycle、event_hash_chain_valid），验证 receipt ↔ events.jsonl ↔ acp_transcript.jsonl 三方数据源一致性。`run_grok_acp_once()` post-session 自动调用，写入 `session-verification.json`，非致命失败。14 tests pass。入口: `assurance/grok_session_verifier.py` / `assurance/grok-session-verification-receipt-v0.1.schema.json` / `assurance/tests/test_grok_session_verifier.py`
13. **Approval → Real Grok ACP Permission Bridge** (第一切片已完成/2026-07-31): 交互式 permission 双向通道已落地（b5a08ef）。`on_acp_event` 回调返回值控制 decision（`"allow_once"` → allow，其他非空字符串 → cancel，`None` → 自动决策）。`build_grok_acp_live_run_fn(interactive=True)` 返回 `(run_fn, permission_queue)`；ACP 收到 `permission_requested` 时通过 bridge 阻塞等待用户决策（5-min timeout），TUI 弹出 modal dialog（Allow Once/Cancel），用户 Enter/Esc → `respond_to_permission()` 回传决策至后台线程。`permission_decision` event 携带 `decision_source` 字段（`"user"` / `"adapter"`）。入口: `MATURE_AGENT_DESIGN_DECOMPOSITION_v0.2` §3 / `assurance/grok_runtime_adapter.py` / `assurance/tui/bridge.py` / `assurance/tui/event_source.py` / `assurance/tui/projector.py` / `assurance/tui/app.py`

#### D2. LIF 科学保障组件 (First Slice Implemented — Design from GROK_BUILD_ADAPTATION_v0.1 §4)

> `GROK_BUILD_ADAPTATION_v0.1` §4 列出了 11 项"不可省略的能力"。其中 6 项已于 2026-07-31 完成第一切片实现（15-20）；2 项（14 SourceRouter、21 ClaimRegistrySync）明确不实现——这些属于 LIF 项目通用纪律，区别于本项目需处理的通用科学性问题，且 CLI 需保持通用性。

14. **SourceRouter** (不实现, 2026-07-31 明确排除): 属于 LIF 项目通用纪律（LIF INDEX + 必读文档已覆盖），区别于本项目需处理的通用科学性问题。CLI 需保持通用性，过专化的来源路由组件会干扰其他任务类型。**本项目明确不实现。**入口: `architecture/GROK_BUILD_ADAPTATION_v0.1.md` §4 第 1 项（仅设计参考）
15. **EvidenceKernel** (第一切片已完成, 2026-07-31):: 维护 action / evidence / claim 三套独立状态，每层有各自的 reason-code gate；action 不能自动升级为 evidence，evidence 不能自动升级为 claim。当前 `runner_scoring_handoff.py` 有 action DAG 但不维护三层状态机。入口: `architecture/GROK_BUILD_ADAPTATION_v0.1.md` §4 第 3 项 / `assurance/runner_scoring_handoff.py`
16. **ClaimBoundary** (第一切片已完成, 2026-07-31):: 区分 measured / direct comparison / bridge hypothesis / promotion eligibility 四种 claim 类型，各自有不同的证据要求和 promotion 路径。当前只在 `test_runner_scoring.py` 中有 `ClaimBoundaryScoringTests` 测试类，无生产实现。入口: `architecture/GROK_BUILD_ADAPTATION_v0.1.md` §4 第 5 项 / `assurance/tests/test_runner_scoring.py`
17. **LeakScanner** (第一切片已完成, 2026-07-31):: 机械检查 + 独立语义审阅双通道；evaluation/holdout 场景 fail-closed；不依赖模型判断"是否泄漏"。当前 `credential_scrub` 覆盖凭据层面但不覆盖跨场景信息泄漏检测。入口: `architecture/GROK_BUILD_ADAPTATION_v0.1.md` §4 第 7 项
18. **ScenarioExporter** (第一切片已完成, 2026-07-31):: 从 regression corpus 生成只有可见事实的匿名场景包，供外部评测使用；不泄露原始数据、模型内部状态或未脱敏内容。入口: `architecture/GROK_BUILD_ADAPTATION_v0.1.md` §4 第 6 项 / `regression/cases-v0.1.yaml`
19. **EvaluationRunner** (第一切片已完成, 2026-07-31):: 冻结模型/prompt/工具/预算/digest；append-only journal；与 oracle 物理隔离。当前 `evaluation/` 目录有 scoring protocol 和 partition 文档，但无可执行 runner。入口: `architecture/GROK_BUILD_ADAPTATION_v0.1.md` §4 第 8 项 / `evaluation/SCORING_PROTOCOL_v0.1.md`
20. **CaseRetrievalGuard** (第一切片已完成, 2026-07-31):: blind-first 原则——历史案例检索必须在 precommitment（先独立形成判断）之后执行，防止锚定效应和 hindsight bias。入口: `architecture/GROK_BUILD_ADAPTATION_v0.1.md` §4 第 9 项
21. **ClaimRegistrySync** (不实现, 2026-07-31 明确排除): 属于 LIF 项目通用纪律（LIF INDEX + 必读文档已覆盖），区别于本项目需处理的通用科学性问题。CLI 需保持通用性，过专化的注册表同步组件会干扰其他任务类型。**本项目明确不实现。**入口: `architecture/GROK_BUILD_ADAPTATION_v0.1.md` §4 第 10 项（仅设计参考）

#### D3. 设计分离未强制 (Design Separation Not Enforced)

22. **Finding ↔ Permission 代码层强制分离** (已关闭/2026-07-31, 设计裁决: `docs/D3_22_FINDING_PERMISSION_SEPARATION_DECISION_2026-07-31.md`): 已落地薄架构不变量层。`Finding`（不可变/content-addressed/scanner 观察）+ `PermissionRecord`（独立决策记录/SHA-256 引用 findings/不嵌入内容）+ `FindingRegistry`（session-scoped 线程安全容器）。`run_grok_acp_once()` 每次 permission 决策自动记录 `PermissionRecord` → `permission-records.jsonl`。不变量：Finding 不含 permission 字段；PermissionRecord 不含 severity/evidence 字段。19 tests pass。入口: `assurance/finding_registry.py` / `assurance/finding-v0.1.schema.json` / `assurance/permission-record-v0.1.schema.json`
23. **Audit Checkpoint 引用 Shadow Commit/Tree ID** (已关闭, 2026-07-31): `AuditLedger.seal()` 已新增可选 `shadow_refs` 参数（commit_sha + tree_sha，均为 40-char Git SHA），写入 audit seal receipt 的 `shadow_refs` 块。`audit-seal-receipt-v0.1.schema.json` 已新增 `shadow_refs` 定义和 `git_sha` pattern。此条随 D1.11（shadow Git repo）一并关闭——audit receipt 现在可明确引用独立 shadow Git store 的 commit/tree ID，不再依赖模糊的 checkpoint 布尔值。入口: `MATURE_AGENT_DESIGN_DECOMPOSITION_v0.2` §4.1 / `assurance/shadow_recovery.py` / `assurance/audit.py`

#### 阻塞依赖

- **D1 全段（D1.10/11/12/13）已于 2026-07-31 全部关闭**。D1.10 ACP 实时事件流→TUI（本次 commit：`_handle_acp_notification` 扩展 + projector ACP handlers + `model_output` 实时文本流 + `maxTurns` 传递）、D1.11 Shadow Git repo（早前已关闭）、D1.12 Post-Run Session 交叉核验（158f5e4）、D1.13 交互式 ACP Permission Bridge（b5a08ef）。D1 段"部分实现需补齐"全部闭合。
- **D1.11 / D3.23** 已于 2026-07-31 同步关闭：shadow Git repo + audit shadow_refs 全链路闭合。
- **D2.15–20** 第一切片已完成（6/6 组件 + 测试）。**D2.14 / D2.21** 明确不实现（LIF 项目通用纪律，非本仓库科学性问题）。
- **D3.22** 已于 2026-07-31 关闭——薄架构不变量层（Finding + PermissionRecord + FindingRegistry），详见设计裁决 `docs/D3_22_FINDING_PERMISSION_SEPARATION_DECISION_2026-07-31.md`。
- **设计序列偏差** 已于 2026-07-31 修正闭合：`run_grok_acp_once()` 落地 + CLI `--grok-mode` 默认 `acp-smoke`，恢复 ACP→Headless 主次关系。`prompt_smoke` 降级为显式窄 smoke 路径。

---

## 术语速查 (Term Quick Reference)

| 术语 / 缩写 | 全称 / 定义 | 类型 |
|---|---|---|
| GSA | General Scientific Assurance——通用科学保证 | 项目名 |
| P0–P5 | P 级合约层——按数字递增的保证层级 | 合约 |
| GSA-CORE | 领域中立的只读审查切片（独立于 P 级框架） | 合约 |
| Gate | 守卫——在执行链路中对特定条件进行强制检查的组件 | 架构 |
| GAK | Gate Assurance Kernel——Gate 的唯一标识前缀（如 GAK-INJ-001） | 命名 |
| GPS | Global Progress Sentinel——全局进度哨兵系统 | 子系统 |
| ACP | Agent Communication Protocol——Grok 的 agent 通信协议 | 协议 |
| ADR | Architecture Decision Record——不可逆架构决策记录 | 流程 |
| Schema | JSON Schema 文件——定义数据合约的结构化约束 | 合约 |
| Envelope | 安全信封——数据的完整性与来源验证包装 | 安全 |
| Permit | 敏感操作许可——需用户审批的操作的权限生命周期 | 安全 |
| Manifest | 清单——记录执行计划和参数的 JSON 文件 | 数据 |
| Receipt | 收据——记录执行结果的 JSON 文件 | 数据 |
| Journal | 日志——JSONL 格式的运行时事件流 | 数据 |
| Canonical CLI | 规范守卫 CLI——经过完整 Gate 序列的离线主路径 | 执行 |
| Upstream | 上游——外部运行时（当前为 Grok Build）的版本锁定 | 集成 |
| AppContainer | Windows 原生应用容器——进程级隔离机制 | 沙箱 |
| Job Object | Windows 作业对象——进程组资源限制机制 | 沙箱 |
| DPAPI | Windows Data Protection API——密钥保护机制 | 安全 |
| No-Model | 无模型——不依赖 LLM 调用的纯机械验证步骤 | 架构 |
| Runner | 执行器——控制 agent 执行生命周期的骨架 | 执行 |
| TUI | Terminal User Interface——终端 UI，本项目指 retro 桌面风格（Explorer/老 IE 式）的字符单元交互界面 | UI |
| Task Checklist | 任务工作清单——顶部公告式软工作板，展开后显示 plan annotations、运行记录和验收引用 | UI/协作 |
| Graft | 嫁接——采用成熟 runtime 的部件作为生产所有者，本仓库只做适配/保障/UI/证据 | 集成 |
| runtime-owned | 由成熟 agent runtime 拥有的通用能力，如 model loop、session、tool dispatch | 所有权 |
| assurance-owned | 由本仓库拥有的保障能力，如 source visibility、PDF evidence、tool availability、反例 gate | 所有权 |
| fixture-only | 只作回归/探针/证明，不作为生产 runtime 路径 | 状态 |
| local_browser | 显式本地浏览器检索模式，默认主检索证据链 | 检索 |
| framework_fallback | 框架自带检索兜底模式，只在本地浏览器不可用或用户允许时使用 | 检索 |
| Global Review Mode | 全局审查模式——明确进入审查时检查原设计理念、当前进度判断、当前实现内容定位、关键设计保留和项目任务边界 | 审查 |
| Neutral Inquiry | 中立询问——执行过程中辅助全局回看，不等同于反例 gate | 保障 |
| Subagent Retrieval Completion Check | 子代理检索完成确认——子代理关闭前只问是否已获得当前主任务所需内容 | 保障 |
| Diagnostic Coverage Check | 诊断覆盖检查——debug 路线锁死时递进触发，检查关键诊断面是否足以选择下一步 | 保障 |
| Counterexample Gate | 反例询问门——仅在 plan 写入前和最终结论写入前触发 | 保障 |

---

## 附A: 审计文档速查

> 按日期倒序排列的审计文档入口。查具体审计结论时从此表定位。

- 2026-08-01: 全局审查 L1-L7 闭合记录 — `docs/GSA_GLOBAL_REVIEW_RECORD_2026-08-01.md`；覆盖 TUI ContentPane、session/layout 所有权重分类、P0-P5/Gate/Security 抽样、schema/repository check、全量测试策略。关键结果: `assurance/tests` 1524 passed / 14 skipped；`python gsa.py doctor --json` 与 `scripts/check_repository.py` 均 `valid: true`。注意: 真实 Grok ACP transcript 仍为后续外部证据待回填项。
- 2026-07-30: Runtime-first 方向校正与 agent 底座嫁接裁决 — 中文主入口 `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md`；Grok 适配审计 `docs/GROK_CLI_SPECIALIZATION_ADAPTATION_AUDIT_2026-07-30.md`；总设计补充 `architecture/INTEGRATED_AGENT_ASSURANCE_DESIGN_v0.1.md`；follow-up alignment `docs/RUNTIME_FIRST_FOLLOWUP_ALIGNMENT_2026-07-30.md`；observer subprocess leak audit `docs/GROK_OBSERVER_SUBPROCESS_LEAK_2026-07-30.md`；first slice 提交 `07c136d Land Grok runtime first slice`
- 2026-07-29-R16: Part A Task Checklist + Part B System Integration — `task_checklist.py` (~420 行, 25 tests)：ChecklistStatus/ChecklistItem/TaskChecklist/derive_checklist_from_plan + journal payload + GPS mapping；TUI AnnouncementStrip（L1/L2/L3 三层渐进式）+ ContentPane message mode（可折叠任务流）+ AddressDialog modal（Ctrl+L 弹窗）+ projector/bridge 全链路；orientation guard checklist_context 注入；12 files, 1687 行新增；326 tests pass；入口: `assurance/task_checklist.py` / `assurance/tui/widgets.py` / `assurance/tui/app.py`
- 2026-07-29-R15: Plan Mode + TUI Simplification 实施完成 + Task Checklist 设计冻结 — `plan_mode.py` (~700 行, 80 tests)：PlanArtifact + PlanVerifier + PlanStateMachine (8-state) + ProcessUsageSampler (psutil 后台采样 + adaptive backoff) + VSCodeTitleUpdater + terminal status line；TUI Phase 4 中文化（菜单/工具栏/状态栏/对话框全中文化）+ HelpOverlay modal (6 tabs) + FindDialog modal + 大主窗模式 (221 tests)；Task Checklist 架构文档冻结 (268 行)；入口: `assurance/plan_mode.py` / `architecture/CLI_UI_SIMPLIFICATION_SUPPLEMENT_v0.1.md` / `architecture/TASK_CHECKLIST_ANNOUNCEMENT_SUPPLEMENT_v0.1.md`
- 2026-07-29-R14: GAK-UX-001 — UXSafetyEvaluator + 10 UX 安全场景 + 8 种断言类型 + 签名 receipt；20 tests pass；全部中等严重度 Gap 全部关闭
- 2026-07-29-R13: GAK-REC-001 — ShadowRecoveryStore + RecoveryDiffPreview + RecoveryExecutor；23 tests pass；shadow recovery store 闭合
- 2026-07-29-R12: GAK-CMP-001 — AutomaticCompactionSimulator + ManualCompactionSimulator + 6 invariant validators；automatic threshold honest unknown 不变性证明；36 tests pass；`AUTOMATIC_THRESHOLD_NOT_OBSERVED` → `AUTOMATIC_THRESHOLD_OBSERVED`
- 2026-07-29-R11: GAK-EVT-001 — audit_integration 桥接层、`--run --real` 自动产出审计封印、provider partial/kernel complete/missing unknown 完整语义、11 audit tests；263 全量回归 pass
- 2026-07-29-R10: GAK-UI-001 Phase 3 — `--real` 标志接入真实 DeepSeek adapter、bridge wiring 测试 4 个、`build_live_run_fn(real_adapter=True)` 全链路贯通；221 TUI + 71 browser/retrieval tests pass
- 2026-07-29-R9: LBR-001 闭合 — LiveBrowserTests 自启动 headless Chrome (0 skip)、E2E Web + PDF 检索集成测试 6 个 (含真实 arXiv PDF 下载→验证→证据库)、login 关键词误判修复、httpbin→example.com 可靠性加固；71 tests pass
- 2026-07-29-R8: LBR-001 闭合推进 — TUI import isolation、Chrome CDP profile 限制文档化、tab cleanup 加固、retrieval_workflow 测试新增 34 个；275 TUI + browser + retrieval tests pass (4 skipped)
- 2026-07-29-R7: LBR-001 Phase 2-4 进行中 — 浏览器 CDP 客户端 + 检索工作流（论文 PDF + 通用搜索）+ TUI /search /retrieve 斜杠命令 + 权限对话框；Chrome 主 profile CDP 连接问题未解决；尚未闭合
- 2026-07-29-R6: LBR-001 Phase 1 PDF Evidence Store — PDF 验证/文本提取/页面索引 + SHA-256 内容寻址存储 + source record schema (A/B/C 证据等级) + version guessing；42 tests pass
- 2026-07-29-R5: GAK-UI-001 斜杠命令系统 — 15 个中文命令 + CommandRegistry + CommandPalette 覆盖层 + 地址栏历史/多行/自动补全 + Ctrl+Z 取消运行；137 TUI tests pass；989 total tests pass
- 2026-07-28-R5: 关闭 GAK-UI-001 (TUI 第一原型完成)、GAK-CRED-001 (凭据生命周期守卫)、GAK-WIN-001 (Job Object 竞态修复)；751 tests pass
- 2026-07-28: `ADAPTER_GATE_AUDIT`, `ORIENTATION_RUNTIME_GUARD_AUDIT`, `P1_CONVERSATION_ARCHIVE_LIFECYCLE_AUDIT`
- 2026-07-27: `GAK_INJ_001_AUDIT`, `TOOL_AVAILABILITY_GATE_AUDIT`, `RETRIEVAL_SUBAGENT_AUDIT`, `GAK_SBX_001_WINDOWS_NATIVE_SANDBOX_AUDIT`
- 2026-07-26: `CANONICAL_GUARDED_CLI_P0_AUDIT`, `CANONICAL_CLI_QUICKSTART`, `GSA_DISPOSABLE_REPRODUCTION_AUDIT`, `GSA_SELF_QUESTION_COUNTEREXAMPLE_DESIGN`, `SOURCE_FULLTEXT_VISIBILITY_RULE`
- 2026-07-25: `P3_INSTRUCTION_AUTHORITY_AUDIT`, `P4_AUDIT_COMPACTION_RECOVERY_AUDIT`, `P4_5_WORKSPACE_FIRST_INTEGRATION_AUDIT`, `P5_SYNTHETIC_USER_TASK_PREFLIGHT`, `GSA_CORE_READONLY_SLICE_AUDIT`, `GSA_ARTIFACT_SCHEMA_REGISTRATION_AUDIT`, `GSA_VALIDATOR_BRIDGE_AUDIT`, `GSA_CROSS_ARTIFACT_LINEAGE_COMPARABILITY_AUDIT`, `GENERAL_SCIENTIFIC_ASSURANCE_GAP_REVIEW`, GPS 系列 (7 篇), CLI/Codex 系列 (3 篇)
- 2026-07-24: `P2_DOCKER_SANDBOX_AUDIT`, `P2_5_GUARDED_EXECUTION_AUDIT`
- 2026-07-23: `GROK_ACP_FAKE_TOOL_SPIKE`, `GROK_UPSTREAM_CANDIDATE_AUDIT`, `GROK_WINDOWS_CHILD_TREE_PROBE`, `GROK_UPSTREAM_PROMOTION`, `GROK_COMPACTION_PROVENANCE`, `DEEPSEEK_CREDENTIAL_HARDENING`, `DEEPSEEK_REAL_DEVELOPMENT_PROBE`, `GROK_REAL_DEEPSEEK_LAUNCHER`
- 2026-07-21: Grok 系列 (7 篇) + `REPOSITORY_AUDIT` + `GLOBAL_PROGRESS_SENTINEL_SPIKE`

---

## 附B: 源码模块速查

> 按功能领域分组的 Python 源码入口。查具体实现时从此表定位到 `assurance/*.py`。

| 领域 | 核心模块 |
|---|---|
| CLI 入口 | `cli.py`, `canonical_cli.py`, `canonical_cli_main.py`, `gsa.py` (根) |
| Gate 链路 | `instruction_provenance_gate.py`, `tool_availability_gate.py`, `retrieval_subagent.py`, `source_visibility.py`, `source_visibility_cli.py`, `adapter_gate.py`, `adapter_preflight.py`, `adapter_output_validator.py`, `adapter_failure_classifier.py` |
| 方向/停滞守卫 | `orientation_runtime_guard.py`, `orientation_runtime_integration.py`, `orientation_runtime_journal.py` |
| 执行 | `guarded_execution.py`, `integrated_run.py`, `runner.py`, `execution_lock.py`, `disposable_reproduction.py` |
| 安全/沙箱 | `sandbox.py`, `sandbox_verifier.py`, `windows_sandbox.py`, `network_permit_gateway.py`, `workspace_trust.py`, `keystore.py`, `key_lifecycle.py`, `envelope.py`, `permit.py`, `endpoint_canonicalizer.py` |
| 审计/归档 | `audit.py`, `archive.py`, `archive_journal.py`, `archive_verifier.py`, `recovery.py`, `shadow_recovery.py`, `compaction_observer.py` |
| P3 授权 | `instruction_gate.py`, `child_capability_enforcer.py` |
| P5 预检 | `user_task_evaluation.py`, `runtime_preflight.py`, `readonly_projection.py`, `task_contract.py`, `ux_safety.py` |
| GSA-CORE | `artifact_registry.py`, `validator_bridge.py`, `general_science_review.py`, `general_science_cli.py` |
| DeepSeek 适配 | `deepseek_adapter.py`, `deepseek_api_observation.py`, `deepseek_stream_observation.py` |
| 数据/存储 | `storage_adapter.py`, `conversation.py`, `profile_registry.py` |
| PDF 证据 (LBR-001) | `pdf_evidence.py`, `evidence_store.py`, `evidence_store.schema.json`, `browser_retrieval.py`, `retrieval_workflow.py` |
| TUI (GAK-UI-001) | `tui/commands.py`, `tui/widgets.py`, `tui/app.py`, `tui/pt_app.py`, `tui/view_models.py`, `tui/main.py`, `tui/events.py`, `tui/event_source.py`, `tui/projector.py` |
| 全局审查模式 | `global_review_mode.py`, `global-review-mode-receipt-v0.1.schema.json` |
| 公共 | `__init__.py` (616 行公共 API 导出), `contracts.py`, `errors.py`, `utils.py`, `runner_public_output.py`, `runner_scoring_handoff.py` |

---

## 附C: 使用红线

- 本文只作路由摘要，**不作证据源**。不能作为事实、参数或审计结论的引用来源。
- 引用任何 P 级合约、Gate 行为、Schema 字段、参数或审计结论前，必须回查原始审计文档、架构文档、Schema 文件和源代码。
- 出现与本文冲突的新审计文档或架构文档时，以后续文档、Schema 和代码实现为准，并更新本文对应路由。
- 原型代码 (`prototype/`) 的结论不可直接用于主项目的保证论证。需要引用原型结论时必须标注"仅设计参考"。
- 修改 P 级合约或 Gate 链路时，必须在同一会话内更新本文对应条目和入口。
- 局部工程 review 不能替代全局审查。只有明确进入“审查模式/全局审查模式”时，才要求结合原设计理念、当前进度判断、当前实现内容定位、关键设计保留和项目任务边界进行判断。
- 不得把“已关闭/已实现”解释为“继续作为本仓库 production runtime 所有”。2026-07-30 后必须同时核对 runtime-owned / assurance-owned / fixture-only 分类。
- 检索不得隐式混用 `local_browser` 与 `framework_fallback`；子代理默认不得超过两个；子代理关闭前 completion check 只能中性询问是否获得所需内容；反例询问不得扩散到每个执行步骤。
- Diagnostic Coverage Check 不受用户“继续”关闭；只能由 bug 解决、阈值递进或明确新证据降噪影响触发，不得扩展成全局审查或大型重新分析。
