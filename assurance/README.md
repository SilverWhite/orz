# General Assurance Kernel P0–P5 contracts and fixtures

本目录冻结 runtime-neutral 保障层的 P0 数据语义，提供 P1 会话身份与删除生命周期以及
P2 Docker strict sandbox、P2.5 guarded execution、P3 指令权限/能力继承、P4
metadata-only audit/恢复授权、P4.5 workspace-first 集成链和 P5 synthetic-user-task
mechanical preflight 的
development/conformance 实现。不实现 model/tool/session runtime，也不表示 identity、retention、
Windows native sandbox 或 anti-injection hard gate 已达到生产可用状态。

权威所有权裁决见
[`ADR-0003`](../adr/ADR-0003-runtime-neutral-assurance-kernel.md)。

## 当前合同

- `effective-security-envelope-v0.1.schema.json`：把 conversation、workspace、runtime adapter、
  capability、sandbox 与签名验证冻结在同一 envelope；关键安全事实必须带证据状态。
- `assurance-profile-registry-v0.1.schema.json`：profile 只要求能力和验收门禁；runtime reference
  不能提升为 required runtime。
- `sandbox-selection-receipt-v0.1.schema.json`：只有 observed compliant backend 可以产生 `allow`；
  没有合格 backend 时必须进入 fail-closed terminal state。
- `session-lifecycle-receipt-v0.1.schema.json`：冻结
  `active → archiving → archived/failed`，归档成功要求敏感残留计数为零。
- `retention-policy-v0.1.schema.json`：冻结归档后最小保留、强制删除、永不持久化与跨会话召回边界。
- `instruction-provenance-receipt-v0.1.schema.json`：记录来源类型、内容摘要、显式结构化动作与
  data-only/candidate/rejected 路由；内容本身永远不产生授权。
- `capability-delegation-receipt-v0.1.schema.json`：证明 child process/agent/remote MCP 的能力
  不超过父 envelope，并拒绝 remote MCP 获得宿主本地能力。
- `sensitive-action-permit-v0.1.schema.json`：把本地交互确认绑定到 action/target/impact/attempt/TTL，
  并以排他 claim 实现 fail-closed 的一次性消费。
- `action-authorization-receipt-v0.1.schema.json`：由内核根据 provenance、envelope 与 consumed
  permit 独立决定 allow/deny；模型声称“已批准”不构成权限。
- `execution-backend-policy-v0.1.schema.json`：标准模式固定为无 Docker 的
  `workspace_guarded`；strict 固定为显式、按需、observed Docker，禁止隐式启动、停止和静默降级。
- `integrated-assurance-run-receipt-v0.1.schema.json`：绑定 P3 provenance/action authorization、
  P4 audit seal、工作区前后摘要、archive 要求，以及无模型/网络/子进程/Docker 的执行事实。
- `synthetic-user-task-suite-v0.1.schema.json` 与
  `synthetic-user-task-evaluation-receipt-v0.1.schema.json`：冻结新手 standard 和熟练用户 strict
  unavailable 两项内部任务，机械通过不等于人类理解或普通用户安全。
- `readonly-task-projection-v0.1.schema.json` 与配套 receipt：只读选取 LIF R211 的路由、讨论、
  代码和小型结果元数据，在 disposable snapshot 中复用，并证明源文件内容前后摘要不变。

`profile-registry-v0.1.json` 将 Grok Build 登记为 `lif-research` 的 `reference_only` runtime。
它不是默认、强制或唯一 runtime。任何实际选择仍需 adapter-specific observed evidence。

`evidence_status` 用于 workspace/runtime/sandbox 等外部观测或派生安全事实。schema 常量、ID、
状态转换和 terminal count 属于签名 receipt 自身的规范字段，不把它们再包装成“对自身的观测”。

`fixtures/p0/` 同时包含合法实例和必须被 schema 拒绝的负例。仓库检查器与
`tests/test_p0_contracts.py` 分别验证它们；这些检查是合同自证，不是操作系统安全证明。
样例遵循“每个硬不变量至少一正一反”，组合空间交给程序化变异与 verifier 测试，不维护笛卡尔积 fixture。

## P1 development implementation

- `keystore.py`：生成随机 256-bit installation key；Windows adapter 使用 Current-User DPAPI
  保存 protected blob，metadata 不保存 secret。非 Windows 只有 `memory-test-only` adapter。
- `envelope.py`：使用 installation key 对 RFC 8785 canonical envelope 做 HMAC-SHA256；runtime、
  workspace、sandbox 和 capability digest 均绑定到 envelope。
- `conversation.py`：每个 active conversation 使用独立目录和 marker；跨 conversation read
  默认拒绝，显式导入标记为 `imported_untrusted`。
- `archive.py`：至少删除 retention policy 中强制的 16 类临时数据，并允许后续 profile 扩展类别；
  不接受任意删除根；执行
  `active → archiving → archived/failed`，生成签名 deletion receipt，并独立重建 totals、
  terminal state 和剩余项。
- 恢复已归档对话时创建新的 `conversation_id`、`envelope_id`、nonce 和 capability digest；
  只保留签名 receipt 中的 parent link，不复用旧 permit、trust 或 active envelope。

`fixtures/p1/` 提供 schema 级正反实例；`tests/test_p1_identity_and_retention.py` 在临时目录中执行
真实文件创建/删除、合成删除失败、receipt 篡改和恢复隔离。Windows 上还执行 DPAPI
create/reopen/sign/verify/tamper observed roundtrip。

### P1 尚未关闭的生产缺口

- HMAC installation key 尚无 rotation/revocation/key-history controller；
- DPAPI 缩小静态暴露面，但不能保证 Python、内核、pagefile、hibernation 或管理员调试面零残留；
- 删除 verifier 只覆盖声明的 conversation namespace，不能证明备份、provider 或物理介质副本已删除；
- 当前是本地 fixture API，没有用户级 CLI、长期迁移、并发锁或 crash recovery；
- strict sandbox 的 Docker 分支由 P2 提供；Windows native 仍未关闭，P3 仅完成 development
  纵向切片，尚未成为所有 runtime/tool adapter 的统一生产 hard gate。

## P2 Docker strict sandbox

- `docker-sandbox-profile-v0.1.json` 固定 Python 3.10 slim 镜像摘要，禁止在线拉取；以
  `65532:65532` 非 root 身份运行，根文件系统只读，网络关闭，drop `ALL` capabilities，
  启用 `no-new-privileges`，并限制内存、CPU、PID 与 wall time。
- `sandbox.py` 只接受带精确 disposable marker 的显式工作目录；只挂载该目录到 `/workspace`，
  `/tmp` 使用受限 tmpfs，不挂载宿主 home、凭据或 Docker socket。所有 Docker 调用均为参数数组，
  不经过 shell。
- 容器内探针验证非 root、根目录不可写、工作区与 tmpfs 可写、网络连接失败、Docker socket 和
  host-home 不存在；容器配置再由 `sandbox_verifier.py` 独立交叉核对并要求清理成功。
- backend selection 只允许 `available + compliant + observed`；指定 backend 不合规时直接
  `fail_closed`，禁止静默 fallback。backend/profile/category 名称允许扩展，不把当前实现列表冻结成产品边界。
- Windows native candidate 当前明确为 `noncompliant`：Job Object 只有进程收束证据，尚无 restricted
  token/AppContainer、文件系统/注册表与网络边界证明。

2026-07-24 的真实 Docker Desktop/WSL2 探针通过，详见
[`P2 Docker audit`](../docs/P2_DOCKER_SANDBOX_AUDIT_2026-07-24.md)。该结果只证明本机固定配置的一次
development observation，不证明 Docker daemon、Linux kernel/WSL2、其他主机或任意容器逃逸均安全。

### P2 尚未关闭的生产缺口

- Windows native strict backend 尚未实现；无 Docker 时 strict 请求仍 fail closed；
- P2.5 已接入一个固定无模型 action、conversation namespace 和 archive lifecycle；尚未接入通用
  runtime/tool broker、任意用户命令或磁盘配额控制；
- Docker daemon 本身具有高权限，daemon/WSL2/kernel 边界不在本 profile 的证明范围；
- 当前仅验证 network-off profile；受 Network Permit Broker 约束的按 endpoint 联网尚未接入；
- 镜像更新、跨架构/跨 OS conformance、长期清理恢复和并发运行仍需后续验证。

## P2.5 guarded execution 与进程追踪

`guarded_execution.py` 提供一条刻意狭窄的端到端路径，只执行固定
`fixed_workspace_roundtrip` action：

- 使用 P1 HMAC envelope 绑定 conversation、workspace path/content/policy、固定镜像、
  runtime capability 与 Docker observation；
- 重新验证 P2 selection，只有同一 conversation 的 observed-compliant Docker candidate 可执行；
- 逐次启动无 shell 的 Docker CLI 子进程，记录角色、宿主 PID、参数摘要、时间、退出码和
  stdout/stderr 的长度与摘要，不持久化原始输出；
- 在容器运行期间记录 Docker inspect host PID 与 `docker top` 的 PID/PPID/command；
- 等待固定 action 退出，核对 workspace 输出后立即删除输出、容器，并用第二次 inspect 证明容器 absent；
- 整份 execution receipt 使用 installation key 做 HMAC-SHA256，独立 verifier 重建 envelope、
  selection、observation、profile、command template 与 process sequence；
- receipt 在 active conversation 中按 `temporary_lifecycle_receipt` 保存，archive 时删除。

2026-07-24 的真实运行与事后 PID/container residue scan 均通过，详见
[`P2.5 audit`](../docs/P2_5_GUARDED_EXECUTION_AUDIT_2026-07-24.md)。

## P3 指令权限与 capability 继承

`instruction_gate.py` 和 `permit.py` 将“内容”“候选动作”“执行权限”拆成三层：

- platform/user/已观测可信项目可以提出显式结构化候选；未建立 trust 的项目文件、web/external
  content、tool output、recalled memory 和 derived summary 只能作为 data；
- 原始内容不被解析为权限，receipt 只保留摘要和长度；recalled memory 必须带来源 conversation，
  未观测的 `trusted_project` 自动降为 `untrusted_project`；
- 动作所需 capability 必须是当前 envelope 的子集；child process/agent 使用实际签名 child
  envelope，remote MCP 不得继承 credential/filesystem/host/process/sandbox/secret 本地能力；
- normal 候选由 envelope 授权；sensitive/external side effect 还必须消费精确绑定、短 TTL、
  指定 attempt 的一次性 permit；
- action authorization、delegation 和 permit receipt 均做 RFC 8785 canonicalization、
  HMAC-SHA256 签名及独立语义重建；相关临时产物随 conversation archive 删除。

测试保持 7 个组合场景，不维护正反例笛卡尔积。它们覆盖 project/web/tool/memory data-only、
内容措辞不提权、能力越界、remote MCP 本地能力拒绝、receipt 篡改、permit 错绑/过期/重放，以及
normal/sensitive 内核授权分离。详见
[`P3 audit`](../docs/P3_INSTRUCTION_AUTHORITY_AUDIT_2026-07-25.md)。

### P3 尚未关闭的生产缺口

- 当前 API 接收调用方已经规范化的结构化动作，不包含任意模型文本到动作的 production parser，
  也不接受 shell command string；实际 path/endpoint canonicalization 必须由未来 tool broker 完成；
- 尚未接入通用 runtime/tool/MCP adapter，不能证明所有真实入口都绕不开该 gate；
- project trust 目前作为已观测状态输入，尚未统一接入所有入口的 trust receipt 与变更失效机制；
- HMAC 是本地 development integrity，不是跨主机远程证明；key rotation/revocation 缺口仍继承 P1；
- 排他 consumption claim 在并发竞争下 fail closed，但 crash recovery、stale-claim 仲裁和长期迁移
  尚未实现；
- 这是权限流隔离，不是对提示注入内容的语义“检测器”；scanner、redaction 与权限裁决保持独立。

## P4 metadata audit、compaction 与恢复授权

`audit.py` 将 ACP/session/provider/supervisor 等来源投影到 runtime-neutral metadata ledger：

- 每个事件使用 typed source、source-local sequence、全局 sequence、前序 hash、payload digest/bytes
  和固定元数据事实；raw payload 只写入 archive 必删类别，不嵌入 ledger；
- seal 必须显式声明 ACP/session/provider/supervisor 的 `complete/partial/unknown`，未观测来源只能
  保持 `unknown`，并要求 terminal exactly-once 且为最后一个事件；
- compaction summary 固定为 `derived_unverified`，retained/discarded/unknown range 不得重叠；
- metadata ledger 保存到 `redacted_conversation`，HMAC seal 保存到 receipts；P1 archive 删除 raw
  provider/private/tool/process/network 数据后，verifier 仍可重建链、计数、来源状态、terminal 和 compaction。

`recovery.py` 只实现恢复候选与授权分离：

- 候选必须引用 verified-complete archive、有效 audit seal 和用户显式 pinned snapshot；
- snapshot 始终标记 `untrusted_recovery_candidate`，候选 receipt 的 authority effect 为 `none`；
- 目标 workspace、候选 digest、snapshot digest 和 attempt 必须绑定到 P3 one-shot permit；
- 授权 receipt 即使 `allow` 也固定 `restoration_performed=false`，不会复制文件或修改 workspace。

测试使用 3 个组合场景覆盖归档后复核、compaction 范围、terminal 约束、journal 篡改、候选篡改、
无 permit 拒绝、精确 permit 授权与重放拒绝。详见
[`P4 audit`](../docs/P4_AUDIT_COMPACTION_RECOVERY_AUDIT_2026-07-25.md)。

### P4 尚未关闭的生产缺口

- source completeness 是显式审计声明，不能证明 runtime 未遗漏从未上报的事件；
- 尚未把 Grok 或其他真实 adapter 的 ACP/session/provider/supervisor 流接入统一 ledger；
- automatic compaction threshold 仍未 observed，当前只冻结 summary/range 语义；
- 没有 shadow Git/对象存储、snapshot 创建器、diff preview 或恢复执行器；`allow` 仅表示许可通过；
- metadata-safe facts 使用窄类型/标识符约束，但生产接入仍需 adapter 级字段映射和敏感值扫描；
- archive 删除证明仍仅覆盖 conversation namespace，不覆盖 provider、备份、pagefile 或物理介质。

## P4.5 workspace-first 集成链

`execution-backend-policy-v0.1.json` 将日常标准模式固定为 `workspace_guarded`，Docker 仅作为用户显式
请求的 strict/on-demand 后端。保障内核不会自动启动或停止 Docker Desktop，也不会在 strict 不可用时
退回标准模式。由于本机已观察到 Docker Desktop 长时间空闲后异常占用 CPU，运维建议是非 strict 任务
保持 Docker 关闭，仅在确实需要物理隔离证据时手动启动并走 P2 observed profile；这是一项运行策略，
不是对 Docker 资源异常根因的修复。

`integrated_run.py` 和 `p45_cli.py` 接通一条不依赖 Grok、Docker、模型、网络或子进程的窄链：

1. 精确 workspace marker 授权一次固定、进程内的 write/read/verify/delete roundtrip；
2. P3 将 user content 仅作为来源记录，并由 signed envelope 对结构化 normal action 独立授权；
3. P4 ledger 记录 kernel authorization、runtime completion 与 exactly-once supervisor terminal；
4. 固定 probe 清理后要求工作区摘要与执行前完全一致；
5. P1 archive 删除 raw authorization/result/lifecycle payload，保留 metadata ledger 和签名摘要；
6. 第二次独立 verifier 重开 DPAPI key，重建 retained receipt、audit chain、archive completeness 与
   raw-payload absence。

`workspace_guarded` 只声明 `logical_workspace_boundary_only`：它限制本实现允许执行的动作和路径，
但不是 AppContainer、restricted token、VM 或容器级物理沙箱。任意用户命令、shell、模型、网络和
子进程均不在 P4.5 standard action 的能力面中。真实 observed run 与 3 个组合测试详见
[`P4.5 audit`](../docs/P4_5_WORKSPACE_FIRST_INTEGRATION_AUDIT_2026-07-25.md)。

### P4.5 尚未关闭的生产缺口

- 标准模式是应用层逻辑边界，不能抵御同一宿主进程、管理员、内核或依赖被攻陷；
- 当前只允许固定 roundtrip，不是通用工具 broker，也不能把任意 shell 命令包装成“安全”动作；
- “无子进程/网络/模型/Docker”由封闭实现、代码路径和签名收据共同证明，尚无系统调用级审计器；
- strict 仍依赖 Docker Desktop/WSL2，并继承其高权限、长时间 CPU 异常和宿主内核信任边界；
- retained P3 receipts 可在归档后验证签名与交叉摘要，但旧 active envelope 按 retention policy 删除；
  因此完整 P3 语义重建发生在归档前，归档后验证其签名结果、绑定和 P4 事件链。

## P5 synthetic user task mechanical preflight

P5 当前只完成内部 mechanical preflight，没有邀请外部参与者：

- `P5-TASK-NOVICE-STANDARD` 执行 P4.5 workspace-first 固定动作并验证 archive/audit；
- `P5-TASK-EXPERIENCED-STRICT` 在没有 observed Docker selection 时 fail closed，不创建 workspace、
  conversation 或 fallback；
- 两项都使用结构化 message/boundary/next-action code，但固定
  `human_comprehension_assessed=false` 和 `actual_human_response=null`；
- receipt 固定外部参与者为 0，禁止把机械成功写成 human usability、ordinary-user safety 或
  production readiness。

根据用户建议，`readonly_projection.py` 另从 LIF 项目只读复制 R211 的 8 个复杂任务来源：
INDEX、短环境入口、自查协议、MAP6、R211 讨论、分析脚本、run manifest 和小型 summary。实现只使用
read + destination-only atomic write；不执行训练/分析脚本，不接触大 CSV/checkpoint，不修改源项目，
并对 source-before/source-after/snapshot 三份聚合摘要做一致性验证。复制品明确标记为 untrusted
source material。2026-07-25 后续审查确认 LIF 的专用路由与长期历史会污染通用复杂测试，因此该复制品
只保留为投影机制的机械 fixture，不进入通用深度测试、阈值校准或 holdout。

Observed run、来源路由与限制见
[`P5 audit`](../docs/P5_SYNTHETIC_USER_TASK_PREFLIGHT_2026-07-25.md)。

### P5 尚未完成

- 尚无真实参与者，未测理解度、错误恢复、认知负担或不同水平用户的实际行为；
- R211 snapshot 只验证投影完整性；不会用作通用复杂 review、阈值校准或 holdout；
- 通用复杂任务需另行从独立合成科研包或许可清楚的非项目来源构造，并物理隔离 reviewer oracle；
- 没有真实凭据、真实项目写入、网络或付费模型测试；这些不是本轮缺失测试的“默认通过”；
- LIF 源内容摘要不变只能证明所选 8 个文件的内容未变化，不证明文件系统 metadata、备份或未选文件；
- 进入外部参与者测试前仍需 consent、脱敏记录、停止条件和任务后清理协议。
