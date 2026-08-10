# CLI_PROJECT_INDEX

> 索引版本：v2.0；状态：`current`；最近整理：2026-08-09。
>
> 当前唯一自然语言设计权威是 [`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md)。本文件只负责召回和路由，不替代 ADR、Schema、审计结论、测试证据或源代码。
>
> 2026-08-09 整理前的完整索引已保存为 [`CLI_PROJECT_INDEX_FULL_2026-08-09.md`](存档/index/CLI_PROJECT_INDEX_FULL_2026-08-09.md)。历史实施流水只能从该快照回查，不得回填污染当前索引。

## 0. 固定写入格式与维护纪律

### 0.1 权威顺序

发生冲突时按内容类型分别裁决，不得用一种材料冒充另一种材料：

1. 自然语言设计：ADR-0010；新 ADR 只有在显式声明取代关系后才能改变它。
2. 机器合约：已登记 Schema、协议与 verifier；若不能表达 ADR-0010，登记为实现差距，不得反向削弱设计。
3. 实现事实：当前源代码和可复现测试；实现偏离 ADR 时标为 `gap`，不得写成新设计。
4. 审计状态：带日期、范围和证据边界的审计文档。
5. 历史材料：`存档/`；只作 provenance/evidence，不独立产生当前需求。

### 0.2 允许使用的状态

- `current-design`：ADR-0010 或其无新增语义的当前投影。
- `implemented`：有当前源码和验证入口支持，且未登记已知设计偏差。
- `partial`：已存在实现，但与当前设计仍有明确缺口。
- `pending`：当前设计已要求，尚无完整实现或审计闭环。
- `reference`：conformance、fixture、兼容层或历史先例，不拥有生产设计。
- `historical`：已归档，只用于追溯。
- `withdrawn`：明确撤回或已被取代；不得作为当前方案复活。

禁止使用“基本完成”“大致可用”“暂时一致”等不可机械核对的状态词。

### 0.3 标准条目格式

所有主题条目必须使用以下单段格式；同一概念只能有一个 canonical entry：

```text
- **<稳定 ID>** (`<允许状态>`; YYYY-MM-DD)：<一句话定义>。关键词：<别名/旧称/检索词>。入口：<权威文档> / <实现或验证入口>。
```

必要时可追加一个“差距：”或“边界：”句，但不得写实施流水。登记表可以使用表格，但只能映射 ID、状态和入口，不得在表中另写设计裁决。

### 0.4 长度与内容边界

- 每条只承载一个概念，保持一个项目符号和一个段落；正文建议不超过 220 个汉字，入口不超过 5 个。
- 禁止在文件开头追加“最新更新”、下一步、提交号、测试数量、跑分批次、终端日志或长篇修复过程；这些内容写入带日期的审计/评测/实施记录。
- 禁止复制 ADR 的完整参数表、状态机或论证；索引只保留足够检索的定义和精准入口。
- 设计与实现必须分条：设计项使用 `current-design`，不符合设计的实现使用 `partial` 或独立 `GAP-*`。
- 历史条目只保留一个归档路由；详细时间线不得同时出现在主题路由和状态路由。
- 路径必须真实存在；重命名、归档或删除文件时，必须在同一变更中修正入口。

### 0.5 更新检查

每次修改本索引必须同时完成：prior-existence scan、稳定 ID 去重、状态合法性检查、入口存在性检查、旧路径残留检查和 `git diff --check`。若只是一次实现进展且没有改变召回路由，不更新本文件。

---

## 1. 当前权威与治理入口

- **AUTH-ADR-0010** (`current-design`; 2026-08-09)：ORZ 融合 runtime 与 Agent 架构的唯一自然语言设计基线。关键词：融合架构、主 Agent、检索子代理、裁决、冻结。入口：[`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md)。
- **AUTH-CURRENT-PROJECTION** (`current-design`; 2026-08-09)：只接收由 ADR-0010 派生且不新增语义的当前设计投影。关键词：current architecture、派生状态机、接口清单。入口：[`architecture/current/README.md`](architecture/current/README.md)。
- **AUTH-V1.1-REVIEW** (`reference`; 2026-08-09)：记录 ADR-0010 v1.1 补写检查、裁决来源和未闭合工程项。关键词：supplement review、遗漏检查、设计复核。入口：[`ADR_0010_V1_1_SUPPLEMENT_REVIEW_2026-08-09.md`](docs/audits/ADR_0010_V1_1_SUPPLEMENT_REVIEW_2026-08-09.md)。
- **AUTH-FREEZE-AUDIT** (`reference`; 2026-08-09)：记录冻结范围、首批 17 份历史材料及冻结时实现差距。关键词：freeze audit、archive audit、FUS-IMPL。入口：[`ADR_0010_FREEZE_AND_ARCHIVE_AUDIT_2026-08-09.md`](docs/audits/ADR_0010_FREEZE_AND_ARCHIVE_AUDIT_2026-08-09.md)。
- **AUTH-ARCHIVE** (`historical`; 2026-08-09)：集中保存退出当前基线的架构、设计输入、实施记录和索引快照。关键词：存档、旧设计、provenance、historical evidence。入口：[`存档/README.md`](存档/README.md)。
- **AUTH-INDEX-SNAPSHOT** (`historical`; 2026-08-09)：保留索引 v2.0 整理前的完整主题、状态和实施时间线。关键词：旧索引、full index、progress history。入口：[`存档/index/README.md`](存档/index/README.md)。

## 2. 融合架构主题路由

- **FUS-CORE** (`current-design`; 2026-08-09)：采用成熟组件优先的融合架构；可复用成熟能力，但最终控制面和职责边界由 ADR-0010 裁决。关键词：ORZ、自研来源、成熟优先、fusion control plane。入口：[`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md)。
- **FUS-AGENT-TOPOLOGY** (`current-design`; 2026-08-09)：一个主 Agent、一个内部检索子代理和一个外部检索子代理复用同一 runtime 架构，模型、thinking、transport、工具、上下文、压缩和单会话预算默认一致。关键词：双子代理、同构 Agent、internal retrieval、external retrieval。入口：[`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md)。
- **FUS-CONCURRENCY** (`current-design`; 2026-08-09)：内外检索角色可双并发，各角色同时最多一个 active instance，全局 `web_search` concurrency 为 1。关键词：dual concurrency、web_search=1、子代理并发。入口：[`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md)。
- **FUS-RETRIEVAL-MODE** (`current-design`; 2026-08-09)：检索模式显式为 `local_browser`、`framework_fallback` 或 `off`，禁止失败后隐式切换。关键词：LBR、fallback、显式检索模式、source visibility。入口：[`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [`SOURCE_FULLTEXT_VISIBILITY_RULE`](存档/docs/design-inputs/SOURCE_FULLTEXT_VISIBILITY_RULE_2026-07-26.md)。
- **FUS-INFORMATION-SUFFICIENCY** (`current-design`; 2026-08-09)：信息充分性完全机械判定；主 Agent 必须提交结构化 `close` 或 `continue(requirement_delta)`，新需求保持同一子代理 activation active。关键词：insufficient、assessment、parent disposition、contract revision。入口：[`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md)。
- **FUS-ORIENTATION** (`current-design`; 2026-08-09)：Orientation 仅承担 session-level 中性方向检查，按 7 个逻辑模型轮和 pre-handoff 规则触发。关键词：中立问询、orientation checkpoint、7 轮。入口：[`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [`ORIENTATION_RUNTIME_GUARD_AUDIT`](docs/ORIENTATION_RUNTIME_GUARD_AUDIT_2026-07-28.md)。
- **FUS-DIAGNOSTIC-COVERAGE** (`current-design`; 2026-08-09)：单 bug episode 使用机械硬信号按 `2→3→4→5` 递进触发，解决后恢复 2。关键词：Diagnostic Coverage Check、debug coverage、证据身份。入口：[`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [`diagnostic_coverage.py`](assurance/diagnostic_coverage.py)。
- **FUS-COUNTEREXAMPLE** (`current-design`; 2026-08-09)：Counterexample 只在计划或正式结论写入前承担一次性反例检查，不介入普通执行方向。关键词：反例询问、counterexample gate、plan gate。入口：[`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md)。
- **FUS-STAGNATION** (`current-design`; 2026-08-09)：输出重复、无进展和运行时挂死只归 Runtime Stagnation Guard；不得交给 Orientation。关键词：重复输出、停滞守卫、wallclock、heartbeat、tool timeout。入口：[`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [`RUN_STALL_GUARDS_PLAN`](存档/docs/implementation-history/RUN_STALL_GUARDS_PLAN_2026-08-08.md)。
- **FUS-BUDGET** (`current-design`; 2026-08-09)：主 Agent 与两个检索子代理各自拥有 120 个工具调用轮预算；工具调用轮、问询轮和恢复后轮次均计数；子代理预算按 session 累计（continue 重入不重置，仅激活关闭后新起），主 Agent 每 run 独立起算（v1.2 补写）。关键词：tool round budget、120、session budget、session 累计。入口：[`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [`ADR-0008`](adr/ADR-0008-tool-round-budget.md)。
- **FUS-STATE-RECOVERY** (`current-design`; 2026-08-09)：journal、blackboard、snapshot、compaction、recovery 和子代理 disposition 共同组成可审计状态链。关键词：journal、snapshot、CAS close、compaction、recovery。入口：[`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md)。
- **FUS-WINDOWS-BOUNDARY** (`current-design`; 2026-08-09)：Windows process/runtime spike、产品事故记录和精选案例库分开治理；现阶段只保留证据边界，不提前宣称案例闭环。关键词：Job Object、Windows compatibility、incident、case library。入口：[`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [`docs/incidents/`](docs/incidents/) / [`docs/cases/`](docs/cases/)。
- **FUS-UI-BOUNDARY** (`current-design`; 2026-08-09)：保留 Toolbar 和只读 session/run-history 投影，产品边界不扩展为完整 IDE。关键词：TUI、Toolbar、readonly session projection、Codex app-server、VS Code lifecycle。入口：[`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md)。
- **FUS-COMPONENT-REGISTER** (`partial`; 2026-08-09)：register 文件框架已建（65 组件全 `audit_required`，不得从 crate 名/编译推断采用档位），逐 crate 审计未开始。关键词：component matrix、crate ownership、mature adoption、audit_required。入口：[`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [`fusion-component-register-v0.1.yaml`](upstream/fusion-component-register-v0.1.yaml) / [`AUTH-V1.1-REVIEW`](docs/audits/ADR_0010_V1_1_SUPPLEMENT_REVIEW_2026-08-09.md)。
- **FUS-CONTROL-FABRIC** (`current-design`; 2026-08-09)：跨信任边界控制事件与外部效果动作必须持一次性 HMAC 票据（ACAF）；签发器独立进程窄 IPC；三运行模式（正常审批/无运行自动/临时沙盒运行），不做完全授权。关键词：ACAF、ControlTicket、SandboxLease、PromotionPermit、签发器、三模式。入口：[`ADR-0011`](adr/ADR-0011-authenticated-control-and-action-fabric.md) / [`AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md`](docs/AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md)。

## 3. 当前实现与符合性路由

- **IMPL-RUST-RUNTIME** (`partial`; 2026-08-09)：`orz/` 是融合架构的 Rust 生产实现工作区，但冻结审计已确认其尚未完全符合 ADR-0010。关键词：orz-loop、orz-host、orz-assurance、orz-bin、orz-tui。入口：[`orz/`](orz/) / [`AUTH-FREEZE-AUDIT`](docs/audits/ADR_0010_FREEZE_AND_ARCHIVE_AUDIT_2026-08-09.md)。
- **IMPL-PYTHON-REFERENCE** (`reference`; 2026-08-09)：`assurance/` 主要承担 conformance、Schema authority、fixture、审计和窄兼容角色，不自动拥有融合 runtime。关键词：Python reference spec、conformance suite、assurance。入口：[`PYTHON_REFERENCE_SPEC_CONTRACT`](architecture/PYTHON_REFERENCE_SPEC_CONTRACT_v0.1.md) / [`assurance/`](assurance/)。
- **IMPL-RUN-EVENT-SCHEMA** (`implemented`; 2026-08-10)：v0.2 事件体系已建（8 个机制事件 + envelope + 双轨 verifier + §4.4 链校验 + mode/result/restore 跨层规则）；Rust producer 整轨写 v0.2，disposition/close/DC/result/restore/mode producer 已产，12 个真实 journals 捕获。关键词：run-event、event schema、receipt、replay、v0.2。入口：[`run-event-v0.1.schema.json`](runtime/run-event-v0.1.schema.json) / [`run-event-v0.2.schema.json`](runtime/run-event-v0.2.schema.json) / [`run_event_journal_validation.py`](assurance/run_event_journal_validation.py)。
- **IMPL-DEEPSEEK-TRANSPORT** (`partial`; 2026-08-09)：默认模型族为 DeepSeek，当前配置为 V4；transport、重试和 thinking 必须与主/子代理同构约束一起复核。关键词：DeepSeek V4、deepseek-v4-flash、transport retry、thinking。入口：[`DEEPSEEK_ADAPTER_CONTRACT`](architecture/DEEPSEEK_ADAPTER_CONTRACT_v0.1.md) / [`ADR-0007`](adr/ADR-0007-transport-retry-policy.md)。
- **IMPL-WRITE-PLACEMENT** (`implemented`; 2026-08-09)：工作区、本体状态和系统必要状态按 ADR-0009 分域，Grill 使用独立只读审计链。关键词：GROK_HOME、write placement、Grill、read-only。入口：[`ADR-0009`](adr/ADR-0009-write-placement-policy.md) / [`WRITE_PLACEMENT_AND_GRILL_DESIGN`](docs/WRITE_PLACEMENT_AND_GRILL_DESIGN_2026-08-08.md)。
- **IMPL-GLOBAL-REVIEW** (`implemented`; 2026-08-09)：显式 Global Review Mode 负责激活全局审查义务，其 receipt 不冒充最终审查结论。关键词：global review、activation receipt、L1-L7。入口：[`global_review_mode.py`](assurance/global_review_mode.py) / [`GSA_GLOBAL_REVIEW_RECORD`](docs/GSA_GLOBAL_REVIEW_RECORD_2026-08-01.md)。
- **IMPL-CONTROL-FABRIC** (`pending`; 2026-08-09)：ACAF 设计已定稿（ADR-0011 accepted），四切片实施独立于 Phase C；Slice 2 前置 GAP-RUN-TESTS RT-001~003 闭合与 policy_revision 接线。关键词：ACAF 切片、签发器、fail-closed、shadow mode。入口：[`ADR-0011`](adr/ADR-0011-authenticated-control-and-action-fabric.md) / [`AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md`](docs/AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md)。

### 3.1 已登记实现差距

- **GAP-TOOL-BUDGET** (`implemented`; 2026-08-09)：MAX_TOOL_ROUNDS 已改为 120（orz `c1513a5`），测试与 env 覆盖同步；ADR-0008 其余语义保留。关键词：MAX_TOOL_ROUNDS、ADR-0008、budget migration。入口：[`AUTH-FREEZE-AUDIT`](docs/audits/ADR_0010_FREEZE_AND_ARCHIVE_AUDIT_2026-08-09.md) / [`controller.rs`](orz/crates/orz-loop/src/controller.rs)。
- **GAP-INQUIRY-SPLIT** (`implemented`; 2026-08-10)：混合 inquiry 已拆为独立机制——Orientation（会话级 7 轮状态机+真实注入+v0.2 事件）、Information Sufficiency（机械 assessment，`indeterminate`）、Counterexample（保留）、Stagnation（去双重消费）；`inquiry.rs`/`neutral_inquiry`/`retrieval_completion_check` 已删（v0.1 replay-only）；Rust producer 整轨翻 v0.2。边界：internal/external 车道零投喂（接 GAP-SUBAGENT-RUNTIME）、Diagnostic Coverage 未接线。关键词：7 轮、orientation_checkpoint、information_sufficiency_assessment。入口：[`orz/crates/orz-loop/src/orientation.rs`](orz/crates/orz-loop/src/orientation.rs) / [`GAP-INQUIRY-SPLIT 实施审计`](docs/audits/GAP_INQUIRY_SPLIT_IMPL_AUDIT_2026-08-10.md)。
- **GAP-SUBAGENT-RUNTIME** (`implemented`; 2026-08-10)：子代理与主 agent 复用同一共享 AgentLoop（agent_loop.rs：LoopProfile/RoundAgent/SharedLoopServices），独立 120 轮预算、同一 journal 链、写域 deny-only 门禁（写/执行/Shell 三类结构化拒绝）、activation 注册表（D3-3 身份正规化）、assessment→disposition→close 链（retrieval_disposition 控制工具 + outcome 机械判定）、internal/external 车道 orientation 投喂、DC producer（§4.6 机制完整，硬信号 4/6 产出）。真实检索工具/结构化结果/跨 turn 持久化/mode authority 由 GAP-RETRIEVAL-TOOLS 闭合。关键词：shared runtime、disposition、close record、activation、DC。入口：[`agent_loop.rs`](orz/crates/orz-loop/src/agent_loop.rs) / [`GAP-SUBAGENT-RUNTIME 实施审计`](docs/audits/GAP_SUBAGENT_RUNTIME_IMPL_AUDIT_2026-08-10.md)。
- **GAP-RETRIEVAL-TOOLS** (`implemented`; 2026-08-10)：真实检索工具（project_doc_index 内部 + web_search/web_fetch 接 grok_build，local_browser 自动化已由 GAP-LOCAL-BROWSER 闭合）+ 结构化结果五字段 v0.2 schema（[RESULT_JSON] 模型块校验 + 机械 ledger + visibility 真实分级 + 显式降级）+ 跨 turn activation 侧车持久化（restore 事件合法化跨 run disposition）+ retrieval mode authority（三态 + transition 事件 + off 投影/门禁）+ DC 两信号 + pre_handoff trigger。关键词：retrieval_mode、result_committed、activation_restored、evidence、project_doc_index。入口：[`GAP-RETRIEVAL-TOOLS 实施审计`](docs/audits/GAP_RETRIEVAL_TOOLS_IMPL_AUDIT_2026-08-10.md) / [`project_doc_index.rs`](orz/crates/orz-host/src/project_doc_index.rs)。
- **GAP-LOCAL-BROWSER** (`implemented`; 2026-08-10)：local_browser 浏览器自动化（CDP）——host 自有工具 `browser_read`（单调用内 create→navigate→read→close tab，§3.7.6 tab ownership）+ 自动启动浏览器（**有头默认**（用户裁决 D-13：窗口可见可手动登录，cookie 会话隔离 profile 内持久）；`ORZ_BROWSER_HEADLESS=1` 回退 headless；隔离 profile、DevToolsActivePort 轮询、跨 run 存活、taskkill 树杀 + A5 清扫）+ URL 门禁（§3.7.3 初始+每次 redirect 重检；**DNS 层 SSRF 复用 check_ssrf** + scheme/凭据/黑名单/单标签）+ host 内置固定表达式（任意 JS 非 MVP）+ evidence web_page 分级 + 探针真实化（失败 Degraded 显式）。关键词：browser_read、CDP、local_browser 自动化、URL 门禁、有头/headless。入口：[`local_browser/`](orz/crates/orz-host/src/local_browser/) / [`GAP-LOCAL-BROWSER 实施审计`](docs/audits/LOCAL_BROWSER_IMPL_AUDIT_2026-08-10.md)。
- **GAP-SUFFICIENCY-SCHEMA** (`implemented`; 2026-08-10)：v0.2 Schema/fixture 已建，双轨 verifier 覆盖 §4.4 机械链校验（CAS/幂等/冲突 decision/迟到 close/close 后禁止）与 inquiry_kind 交叉校验；assessment/disposition/close producer 全部落地并捕获真实 journals（GAP-INQUIRY-SPLIT + GAP-SUBAGENT-RUNTIME）。关键词：awaiting_parent_disposition、close receipt、requirement_delta、v0.2。入口：[`run-event-v0.2.schema.json`](runtime/run-event-v0.2.schema.json) / [`run_event_journal_validation.py`](assurance/run_event_journal_validation.py) / [`AUTH-V1.1-REVIEW`](docs/audits/ADR_0010_V1_1_SUPPLEMENT_REVIEW_2026-08-09.md)。
- **GAP-WEB-SEARCH-SEMAPHORE** (`implemented`; 2026-08-10)：全局 `web_search` 并发=1 semaphore（§3.7.7 第 7 项 web_search=1 分量）——唯一汇点 `OrzHost::call_tool` 挂载 `Arc<Semaphore>(1)`（每 OrzHost 实例一个；§11.3 双并发形态的前瞻保护，当前 web_search 经 relay 派发+嵌套门无直执行路径——前序遗留缺口已登记审计 §4），acquire 在 P0-1 timeout 包裹内（等待计入 300s 预算、timeout drop 自动释放 permit、等待超时不杀树），`web_search_*` 变体同 gate（relay 同构防拼写绕过），web_fetch 不 gate；三面审查闭环（P3-1/P2-1/P3-4/F7 修复）。关键词：concurrency=1、web_search semaphore、资源所有权约束。入口：[`WEB_SEARCH_SEMAPHORE_IMPL_AUDIT`](docs/audits/WEB_SEARCH_SEMAPHORE_IMPL_AUDIT_2026-08-10.md) / [`lib.rs`](orz/crates/orz-host/src/lib.rs) / [`tools.rs`](orz/crates/orz-host/src/tools.rs)。
- **GAP-RUN-TESTS** (`partial`; 2026-08-09)：run_tests 三差距——RT-001 跳过 execution permission、RT-002 env 全继承无脱敏、RT-003 无 workspace delta 记录；Job Object/timeout/输出上限/声明门控已验证达标。关键词：run_tests、D-9、hidden test、Aider mode。入口：[`V11_IMPL_005_RUN_TESTS_SECURITY_REVIEW`](docs/audits/V11_IMPL_005_RUN_TESTS_SECURITY_REVIEW_2026-08-09.md)。
- **GAP-DENIAL-POLICY-REVISION** (`partial`; 2026-08-09)：DenialKey.policy_revision 当前恒 0 且无接线来源——ADR §3.5.4"policy revision 变化重置"结构上不可触发；未来引入 mid-session revision 机制时必须接入。关键词：denial breaker、policy_revision、V11-IMPL-012。入口：[`ADR-0010 §3.5.4`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [`controller.rs`](orz/crates/orz-loop/src/controller.rs)。
- **GAP-WINDOWS-EVIDENCE** (`partial`; 2026-08-09)：三个案例候选（ORZ-WIN-PROC-001/002/003，晋级自 child-tree 探针三场景）已登记并引用探针 digest，均标 `candidate` 未宣称闭环；事故路由保留 WIN-LIM-001（raw TCP）与 WIN-INC-001（observer leak）。关键词：Windows incident、case selection、compatibility evidence、WIN-LIM、ORZ-WIN-PROC。入口：[`docs/incidents/windows/`](docs/incidents/windows/) / [`docs/cases/windows/`](docs/cases/windows/)。

## 4. 保障、合约与安全路由

- **P0-DATA-CONTRACT** (`reference`; 2026-08-09)：Python assurance 的基础 Schema、信封和合约语义先例；是否进入融合 production path 需按组件登记表复核。关键词：P0、contracts、envelope、canonical guarded CLI。入口：[`CANONICAL_GUARDED_CLI_P0_AUDIT`](docs/CANONICAL_GUARDED_CLI_P0_AUDIT_2026-07-26.md) / [`contracts.py`](assurance/contracts.py)。
- **P1-SESSION-LIFECYCLE** (`reference`; 2026-08-09)：Python 会话身份、permit、archive journal 和恢复生命周期先例。关键词：P1、conversation namespace、archive recovery、session governor。入口：[`P1_CONVERSATION_ARCHIVE_LIFECYCLE_AUDIT`](docs/P1_CONVERSATION_ARCHIVE_LIFECYCLE_AUDIT_2026-07-28.md)。
- **P2-SANDBOX** (`reference`; 2026-08-09)：Docker 严格沙箱及 Windows Native Sandbox/Job Object 的既有审计与 conformance 输入。关键词：P2、Docker、AppContainer、Job Object。入口：[`P2_DOCKER_SANDBOX_AUDIT`](docs/P2_DOCKER_SANDBOX_AUDIT_2026-07-24.md) / [`GAK_SBX_001_WINDOWS_NATIVE_SANDBOX_AUDIT`](docs/GAK_SBX_001_WINDOWS_NATIVE_SANDBOX_AUDIT_2026-07-27.md)。
- **P2.5-GUARDED-EXECUTION** (`reference`; 2026-08-09)：无模型动作、进程追踪和输出拦截的 Python 守卫执行先例。关键词：P2.5、guarded execution、process tracking。入口：[`P2_5_GUARDED_EXECUTION_AUDIT`](docs/P2_5_GUARDED_EXECUTION_AUDIT_2026-07-24.md)。
- **P3-INSTRUCTION-AUTHORITY** (`reference`; 2026-08-09)：instruction provenance、capability delegation 和 child capability enforcement 的既有合约输入。关键词：P3、instruction gate、授权、子能力。入口：[`P3_INSTRUCTION_AUTHORITY_AUDIT`](docs/P3_INSTRUCTION_AUTHORITY_AUDIT_2026-07-25.md)。
- **P4-AUDIT-RECOVERY** (`reference`; 2026-08-09)：audit ledger、compaction、archive verifier 和恢复授权的 Python conformance 输入。关键词：P4、compaction、recovery、lineage。入口：[`P4_AUDIT_COMPACTION_RECOVERY_AUDIT`](docs/P4_AUDIT_COMPACTION_RECOVERY_AUDIT_2026-07-25.md)。
- **P4.5-WORKSPACE-FIRST** (`reference`; 2026-08-09)：workspace-first 集成与可恢复变更的既有审计输入。关键词：P4.5、workspace、mutation、snapshot。入口：[`P4_5_WORKSPACE_FIRST_INTEGRATION_AUDIT`](docs/P4_5_WORKSPACE_FIRST_INTEGRATION_AUDIT_2026-07-25.md)。
- **P5-TASK-PREFLIGHT** (`reference`; 2026-08-09)：任务合同、用户意图和 synthetic-user preflight 的 Python 先例。关键词：P5、task contract、preflight、readonly projection。入口：[`P5_SYNTHETIC_USER_TASK_PREFLIGHT`](docs/P5_SYNTHETIC_USER_TASK_PREFLIGHT_2026-07-25.md)。
- **GATE-CHAIN** (`partial`; 2026-08-09)：instruction、tool availability、adapter、source visibility、permission 和 runtime guards 构成分层 gate 链；与融合 runtime 的最终接线需随实现审计复核。关键词：Gate、IPG、permission bridge、source gate。入口：[`assurance/`](assurance/) / [`orz/crates/orz-assurance/`](orz/crates/orz-assurance/)。
- **SEC-CREDENTIALS** (`partial`; 2026-08-09)：凭据 registry、读取和脱敏已有实现与审计输入，但融合 production path 的最终注入与符合性仍需组件级复核。关键词：credential registry、keystore、scrub、GAK-CRED-001。入口：[`ADR-0006`](adr/ADR-0006-credential-target-registry.md) / [`DEEPSEEK_CREDENTIAL_HARDENING`](docs/DEEPSEEK_CREDENTIAL_HARDENING_2026-07-23.md)。
- **EVIDENCE-LOCAL-BROWSER** (`partial`; 2026-08-10)：Local Browser/PDF evidence 保留状态机、失败处理、URL/JS/prompt-injection 和全文可见性边界；**规范性裁决已转录 ADR-0010 §3.7**，网页读取生产实现在 Rust（GAP-LOCAL-BROWSER）；Python 实现（retrieval_workflow 等）待按 ADR-0010 重新符合性审查或退役，PDF 管线是后续切片。关键词：LBR-001、CDP、PDF evidence、prompt injection。入口：[`LOCAL_BROWSER_RETRIEVAL_AND_PDF_EVIDENCE`](存档/architecture/pre-adr-0010/LOCAL_BROWSER_RETRIEVAL_AND_PDF_EVIDENCE_v0.1.md) / [`retrieval_workflow.py`](assurance/retrieval_workflow.py)。

## 5. ADR 登记表

本表仅登记状态和入口；设计内容必须回到 ADR 原文。

| ADR | 当前状态 | 入口 |
|---|---|---|
| ADR-0001 | proposed | [`ADR-0001`](adr/ADR-0001-evidence-constrained-local-agent-kernel.md) |
| ADR-0002 | accepted / deferred | [`ADR-0002`](adr/ADR-0002-defer-cloud-runtime.md) |
| ADR-0003 | accepted / partially superseded by ADR-0010 | [`ADR-0003`](adr/ADR-0003-runtime-neutral-assurance-kernel.md) |
| ADR-0004 | accepted | [`ADR-0004`](adr/ADR-0004-general-science-profile-layering.md) |
| ADR-0005 | superseded by ADR-0010 | [`ADR-0005`](adr/ADR-0005-neutral-inquiry-thresholds-finalized.md) |
| ADR-0006 | accepted | [`ADR-0006`](adr/ADR-0006-credential-target-registry.md) |
| ADR-0007 | accepted | [`ADR-0007`](adr/ADR-0007-transport-retry-policy.md) |
| ADR-0008 | accepted / numeric value partially superseded | [`ADR-0008`](adr/ADR-0008-tool-round-budget.md) |
| ADR-0009 | accepted | [`ADR-0009`](adr/ADR-0009-write-placement-policy.md) |
| ADR-0010 | accepted / frozen / current authority | [`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) |
| ADR-0011 | accepted | [`ADR-0011`](adr/ADR-0011-authenticated-control-and-action-fabric.md) |

## 6. 评测与回归入口

- **EVAL-POLYGLOT** (`reference`; 2026-08-09)：多语言 benchmark 用于暴露 loop、transport、工具反馈和预算问题，不独立定义产品设计。关键词：polyglot benchmark、feedback loop、D-9。入口：[`POLYGLOT_BENCHMARK_FINDINGS`](docs/POLYGLOT_BENCHMARK_FINDINGS_2026-08-06.md)。
- **EVAL-TERMINAL-BENCH** (`reference`; 2026-08-09)：Terminal-Bench 2 记录任务级表现、挂死守卫和 harness 证据。关键词：TB2、wallclock、mounts、hard tasks。入口：[`TERMINAL_BENCH_2_EVAL`](docs/TERMINAL_BENCH_2_EVAL_2026-08-08.md) / [`TERMINAL_BENCH_2_EXPLORATORY_SCORE_AUDIT`](docs/TERMINAL_BENCH_2_EXPLORATORY_SCORE_AUDIT_2026-08-08.md)。
- **EVAL-SWE-BENCH** (`reference`; 2026-08-09)：SWE-bench Verified 记录软件修复任务的评测范围和结果边界。关键词：SWE-bench、verified、software repair。入口：[`SWE_BENCH_VERIFIED_EVAL`](docs/SWE_BENCH_VERIFIED_EVAL_2026-08-07.md)。

## 7. 源码与机器合约速查

| 领域 | 当前入口 |
|---|---|
| Rust Agent loop | [`orz/crates/orz-loop/`](orz/crates/orz-loop/) |
| Rust host / ACP | [`orz/crates/orz-host/`](orz/crates/orz-host/) |
| Rust assurance | [`orz/crates/orz-assurance/`](orz/crates/orz-assurance/) |
| Rust CLI / TUI | [`orz/crates/orz-bin/`](orz/crates/orz-bin/) / [`orz/crates/orz-tui/`](orz/crates/orz-tui/) |
| Python conformance / fixture | [`assurance/`](assurance/) |
| Runtime event Schema | [`runtime/`](runtime/) |
| Protocol | [`protocol/`](protocol/) |
| Repository checks | [`scripts/`](scripts/) |
| Current architecture projection | [`architecture/current/`](architecture/current/) |
| Current audits | [`docs/audits/`](docs/audits/) |
| Historical materials | [`存档/`](存档/) |

## 8. 状态速查

本节只列 canonical ID，不重复定义：

- `current-design`：AUTH-ADR-0010、AUTH-CURRENT-PROJECTION、FUS-CORE、FUS-AGENT-TOPOLOGY、FUS-CONCURRENCY、FUS-RETRIEVAL-MODE、FUS-INFORMATION-SUFFICIENCY、FUS-ORIENTATION、FUS-DIAGNOSTIC-COVERAGE、FUS-COUNTEREXAMPLE、FUS-STAGNATION、FUS-BUDGET、FUS-STATE-RECOVERY、FUS-WINDOWS-BOUNDARY、FUS-UI-BOUNDARY、FUS-CONTROL-FABRIC。
- `implemented`：IMPL-WRITE-PLACEMENT、IMPL-GLOBAL-REVIEW、IMPL-RUN-EVENT-SCHEMA、GAP-TOOL-BUDGET、GAP-INQUIRY-SPLIT、GAP-SUBAGENT-RUNTIME、GAP-SUFFICIENCY-SCHEMA、GAP-RETRIEVAL-TOOLS、GAP-LOCAL-BROWSER、GAP-WEB-SEARCH-SEMAPHORE。
- `partial`：IMPL-RUST-RUNTIME、IMPL-DEEPSEEK-TRANSPORT、GAP-WINDOWS-EVIDENCE、FUS-COMPONENT-REGISTER、GATE-CHAIN、SEC-CREDENTIALS、EVIDENCE-LOCAL-BROWSER。
- `pending`：IMPL-CONTROL-FABRIC。
- `reference`：AUTH-V1.1-REVIEW、AUTH-FREEZE-AUDIT、IMPL-PYTHON-REFERENCE、P0-DATA-CONTRACT、P1-SESSION-LIFECYCLE、P2-SANDBOX、P2.5-GUARDED-EXECUTION、P3-INSTRUCTION-AUTHORITY、P4-AUDIT-RECOVERY、P4.5-WORKSPACE-FIRST、P5-TASK-PREFLIGHT、EVAL-POLYGLOT、EVAL-TERMINAL-BENCH、EVAL-SWE-BENCH。
- `historical`：AUTH-ARCHIVE、AUTH-INDEX-SNAPSHOT。

## 9. 使用红线

- 本索引不是事实证据或设计权威；任何结论都必须沿入口回查。
- 不得从 `存档/` 直接生成当前实现要求；先确认该内容是否已转录进 ADR-0010。
- 不得因现有代码或测试锁定旧行为，就把实现现状反写成设计。
- 不得把 `reference`、fixture 或 Python conformance 路径描述为 production runtime owner。
- 不得把一次跑分、测试全绿、提交完成或阶段关闭写成架构符合性结论。
- 不得为同一概念创建第二条长说明；补充关键词和入口应修改原 canonical entry。
- 不得在状态速查、ADR 表或源码速查中复制主题定义。
