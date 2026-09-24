> **pre-ADR-0010 冻结状态头（2026-09-24，审查修复批）**：本文件属 pre-ADR-0010 时期冻结 fixture／development-only spike——**非 `current`、仅历史基线**，不作为实现依据；当前设计权威＝[`ADR-0010`](../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)（本状态头为其 §7.2 归档规则的**就地冻结**轻量替代，不移档不改正文，2026-09-24 批登记）。

# 通用 Assurance Kernel 安全与可审计迁移缺口登记 v0.1

状态：2026-07-25 P0 合同已冻结，P1 已完成 development/conformance fixture，P2 Docker
strict sandbox 与 P2.5 无模型 guarded execution 纵向切片已实测通过；P3 指令权限与 capability
继承、P4 metadata audit/compaction/恢复授权，以及 P4.5 workspace-first 集成链 development
纵向切片已通过；P5 internal mechanical preflight 已通过，但真实参与者测试尚未开始。
2026-07-27 起：instruction provenance gate 与 tool availability gate 已接入 canonical CLI offline
主路径；Windows native sandbox 已具备 profile/observation/verifier 合同，live AppContainer
probe 已跑（elevated path compliant, non-elevated fail-closed）；retrieval sub-agent 仅 no-model fixture。Windows native strict backend、P3/P4
真实 adapter 与恢复执行接入，以及 P5 人类可用性仍未关闭。
本文记录已接受的迁移方向，但**不表示普通用户安全能力已经就绪**。
General Assurance Kernel 保持 runtime-neutral；Grok Build 是当前证据最完整的 reference runtime，
不是强制或唯一底座。项目不建设第二套 model/tool/session runtime。

## 1. 结论

当前项目下一阶段不应继续增加零散提示词或只面向 LIF 的特例，而应形成：

```text
capability-gated external runtime
        |
runtime adapter
        |
General Assurance Kernel
  - workspace trust
  - capability envelope
  - physical sandbox launcher
  - credential/network broker
  - instruction provenance / anti-injection
  - audit projection / verifier
  - session-scoped snapshot and retention
        |
profiles
  - general-code
  - restricted-review
  - headless-ci
  - general-science
      |
      +-- lif-research
```

这里的“通用”指保障不变量可迁移，不指支持任意 runtime、任意云服务或面向公众运营。
`general-science` 承载 SourceRouter、EvidenceKernel、ValidatorBridge、claim promotion、
countercase、blind-first review 和科学自查；LIF profile 只增加当前来源路由、prior-existence
和领域 validator，不再承载可被其他科研任务复用的通用科学或安全逻辑。

## 2. 本轮任务契约

### MUST

- 默认进入经过证明的物理隔离环境；依赖虚拟环境只能作为依赖隔离，不能冒充安全沙箱。
- 在读取项目规则、hook、skill、plugin、MCP 配置和外部内容前建立 workspace trust。
- 工具权限、网络、凭据、子进程、子 Agent 和恢复动作受同一个可验证 capability envelope 约束。
- 快照、工具记录和过程记录以 `conversation_id` 为命名空间，不允许跨会话静默召回。
- 归档后删除原始调用/过程记录，只持久保留对话、用户固定的重要快照和最小终结/删除凭据。
- 事实、直接比较、桥接假说、用户待核叙述、Agent 推断和未核风险必须分开记录。
- 每个 hard gate 必须有 schema、negative fixture、terminal state 和独立 verifier；提示词与 hook 不能替代。

### MUST NOT

- 不以主板序列号、MAC、磁盘序列号或其他稳定硬件指纹作为主要身份、授权或会话解密根。
- 不把 Python `venv`、Node 环境、Job Object、确认弹窗、关键词黑名单或输出清洗单独称为 sandbox。
- 不因“能够恢复文件”自动批准外发、删库、密钥访问或其他不可逆副作用。
- 不保存 raw reasoning、Bearer/API key、完整环境变量、原始网络 body、无界 stdout/stderr 或跨会话原始工具结果。
- 不在 Docker/严格 sandbox 请求失败时静默降级为 unrestricted。
- 不为明确关闭 hard gate、绕过可信工作区或手工破坏审计链的使用方式提供“软保护已足够”的错觉。

### SOURCE OF TRUTH

- 当前仓库的 runtime、transport、审计和 D salvage 文档，见 §3。
- LIF 三件套原文件只贡献查找顺序、任务契约、自证和事实分层方法；不证明本文工程能力已经实现。
- Google Drive 的 D 存档只作为历史设计输入；其 prompt-only 防护、未鉴权远程执行和长期记忆设计不是安全基线。

### ACCEPTANCE / SELF-PROOF

- 有一份可追踪的缺口表，逐项写明当前状态、下一产物和通过条件。
- 身份、会话恢复、归档删除、Docker/本机 sandbox 和反注入均有明确 fail-closed 语义。
- 后续实现必须用 disposable fixture 验证；在通过前不接朋友的真实项目、真实凭据或真实研究数据。
- README 能直接进入本文；仓库链接与机械检查通过。

## 3. 来源账本与证据分层

### 3.1 本地原件

| 来源 | SHA-256 | 本文用途 |
|---|---|---|
| `C:\Users\1\OneDrive\Desktop\新建文件夹\LIF_CURRENT_INDEX.md` | `c11b9fc55b7da35a1e5851cf5f1025c966ca23d4946296e164cea0722742fc8d` | prior-existence scan / 撤回与状态路由 |
| `C:\Users\1\OneDrive\Desktop\新建文件夹\fep_env_research.md` | `20f11033279e64b6a7c245f2de1528b44f36bd2d7ff16e89043873020620a17b` | Agent 任务契约与自证字段 |
| `C:\Users\1\OneDrive\Desktop\新建文件夹\self_check_protocol.md` | `4150339d8cdc4547f240b4de3cea132b74a87f28bb31c511b6729e789830f930` | 事实/推断分层与第二轮自查 |
| [`D_SALVAGE_MATRIX_v0.1.md`](D_SALVAGE_MATRIX_v0.1.md) | `e923ba2112ccf5f23e1c4706c4ffa42b7c629507039383ebb6b9683633c71bc1` | D 源码 source ledger 与 adopt/adapt/reject |
| [`OPEN_SOURCE_AGENT_GAP_AUDIT_v0.1.md`](OPEN_SOURCE_AGENT_GAP_AUDIT_v0.1.md) | `060b369646ccec3a969350f1d786268a05abcc0aafac6df0e6c3de8627721054` | trust、event、checkpoint、compaction 缺口 |
| [`MATURE_AGENT_DESIGN_DECOMPOSITION_v0.2.md`](MATURE_AGENT_DESIGN_DECOMPOSITION_v0.2.md) | `52d020e92fd03b6d8d6f0ae647e3daa72169192e2f25acd85b1d36141c2a7398` | runtime/保障层职责分离 |
| [`WINDOWS_RUNTIME_CONTRACT_v0.1.md`](../存档/architecture/runtime-spikes/WINDOWS_RUNTIME_CONTRACT_v0.1.md) | `aa261c7033048d0228ad41b965f0258b30bbc4b7b6e7dd94b09f3587f13c7e7a` | Job Object、no-shell、输出与取消边界 |
| [`DEEPSEEK_EXTERNAL_TRANSPORT_SAFETY_v0.1.md`](DEEPSEEK_EXTERNAL_TRANSPORT_SAFETY_v0.1.md) | `5669becc95d1449419e1becf5db4438aceefd712937cfbc261dc4040fa21a60e` | 凭据、网络许可、真实残留限制 |

对 `LIF_CURRENT_INDEX.md` 执行了 `agent/CLI/sandbox/trust/prompt injection/audit/session/snapshot/
identity/Docker/compaction` 及中英文别名、相反说法、撤回词扫描；没有发现与本文通用安全设计同义的既有
LIF claim。因此本文是新的 CLI 工程设计记录，不登记为 LIF 科学事实，也不修改 INDEX/MAP。

### 3.2 Google Drive D 存档复核

本轮重新读取了 [`D（存档）`](https://drive.google.com/drive/folders/1yxnaY_ErMyrQdmZxN6Cf2zw5ZWWHMaPg)
中的以下代码：

- `D（存档0）/二号存档/src/safety.py`：危险关键词、敏感路径匹配和阻塞式 `y/n` 二次确认；
- `D（存档0）/二号存档/src/brain.py`：system prompt、记忆注入和模型驱动工具循环；
- `D（存档0）/二号存档/src/tools.py`：固定工具映射，但仍有 `os.system`/`start` 字符串执行；
- `D（存档0）/二号存档/src/memory.py`：短期历史、ChromaDB 长期记忆和 JSON 包装；
- `D（存档1）/cloud-brain/src/core/mcp_remote.ts`：远程 Python 请求带 header，但源码注释明确服务端未实际鉴权；
- `D（存档1）/cloud-brain/src/services/memory.ts`：先摘要/向量化，再在全部成功后删除短期日志；
- `D（存档1）/cloud-brain/src/handlers/telegram.ts`：Telegram user ID 白名单和显式全量记忆清除；
- `D（存档1）/cloud-brain/src/services/llm.ts`：工具结果直接回注模型，所谓 security sanitization 只清理输出标签。

可复用的是设计意图：危险动作二次确认、运行面分离、入口身份白名单、归档成功后再删除和明确 wipe。
不可复用的是实现强度：关键词/路径子串、prompt/JSON 包装、queued-as-success、未鉴权 remote Python、
跨会话长期向量记忆和输出清洗。该裁决与现有 `D_SALVAGE_MATRIX_v0.1.md` 一致。

### 3.3 证据类别

- **source-grounded**：上述实际打开的文件、当前仓库已有 observed fixture 和文档中的明确限制。
- **observed**：本机 Docker Client/Engine `29.2.1`、Linux/amd64 server、builtin seccomp 与固定摘要
  Python 镜像已经由 P2 探针读取；一次 network-off disposable container probe 已通过并清理。
- **agent-inferred design**：Assurance Kernel 模块划分、身份 envelope 和留存 lifecycle。
- **unchecked**：朋友设备、其他 OS、Docker Desktop/Engine 版本、TPM/keystore 可用性和真实用户体验。

## 4. 威胁模型与不保障项

### 4.1 默认防护

- 不可信仓库通过规则、hook、skill、MCP、依赖安装或工具输出注入指令；
- 模型误调用删除、覆盖、网络外发、密钥读取、后台进程或跨会话记忆；
- 子进程/子 Agent 越过父 action 权限；
- 审计记录被静默截断、压缩摘要冒充原始事实或归档后留下原始敏感过程数据；
- 失败重试、redirect、proxy、shell 插值或宽目录挂载扩大已批准动作；
- 用户恢复旧对话时，把旧授权、旧 workspace digest 或旧工具结果错误复活。

### 4.2 明确不保障

- 管理员、内核级恶意软件、debugger、pagefile/hibernation、外部备份和 provider 自身日志；
- 用户主动把密钥写入 prompt、仓库或公开日志；
- 用户显式选择 unrestricted/bypass 后仍期待 hard gate 生效；
- 对外部副作用的通用回滚，例如已发送消息、已泄漏密钥或已触发付费 API；
- “审计通过”自动等于模型正确、代码正确或科学结论成立。

## 5. 默认执行环境：物理隔离优先

### 5.1 后端选择

统一 launcher 应生成 `sandbox_selection_receipt`，按 profile 选择：

1. **proven native strict sandbox**：无需用户安装额外软件时的默认入口；
2. **Docker sandbox**：已安装且 daemon 健康时的一键入口，可由用户/profile 明确选择；
3. **no compliant backend**：strict/untrusted 任务 fail closed，不自动回落 unrestricted。

“自动”只表示探测和选择，不表示静默安装 Docker、修改系统设置或提升管理员权限。选中的 backend、
版本、镜像 digest、mount、network、user、resource limit 和降级原因都必须进入 receipt。

### 5.2 Docker 快捷启动的冻结要求

未来 `safe-agent --sandbox docker` 或等价入口至少满足：

- 固定镜像 digest；非 root user；`no-new-privileges`；drop capabilities；
- read-only root filesystem，临时目录使用独立 ephemeral volume；
- 只把明确 workspace 挂为读写；宿主 home、Credential Manager、SSH、浏览器 profile 和 Docker socket 不挂载；
- 网络默认关闭；需要外网时经 Network Permit Broker，只允许冻结 endpoint/attempt；
- CPU、内存、PID、磁盘和 wall-clock 有界；
- session volume 绑定单一 `conversation_id`，归档后由 Retention Controller 清理；
- Docker 不可用、镜像 digest 不符或挂载超界时 fail closed；
- launcher 不把 Docker daemon 权限误写为普通应用级 sandbox 安全证明。

### 5.3 无 Docker 用户

无 Docker 不能等价于无保护。Windows-first 的最低目标是受限 token/AppContainer 或经实测的等价文件、
注册表、进程和网络边界，再叠加 Job Object。当前仓库已证明 Job Object 进程树收束和 no-shell 参数传递，
但未实现 token 降级、AppContainer、文件系统 virtualization 或完整网络 sandbox，因此当前状态是
**process containment partial，不是 native strict sandbox ready**。

Linux/macOS 后续分别采用 namespace/seccomp/bubblewrap 类能力和 Seatbelt/container 类能力，但只有
observed negative fixture 通过后才能标为 supported。Python `venv`/Conda/Node 环境始终只标为 dependency isolation。

## 6. 身份与每次读取的反向验证

### 6.1 不采用主板号

主板号不适合作为主要身份或授权根：

- 是稳定硬件指纹，增加隐私和跨项目关联风险；
- 在虚拟机、容器、主板更换、维修或权限受限环境中不稳定；
- 可被伪造，也不能证明当前交互者就是获授权用户；
- 难以轮换、撤销和恢复，会把普通迁移变成设备锁死。

如果设备提供 TPM/secure enclave，可用于保护**随机生成的安装密钥**，但不读取、保存或上传硬件序列号。

### 6.2 推荐身份 envelope

首次安装生成随机非识别性 `installation_id` 和签名密钥；私钥进入 OS keystore，能用硬件保护时只增强私钥保护。
每个会话再生成随机 `conversation_id`，并冻结：

```text
installation_key_id
conversation_id
workspace_canonical_path_digest
workspace_content/policy_digest
runtime_binary_digest
assurance_config_digest
sandbox_backend_digest
monotonic_sequence
nonce
created_at / expires_at
```

会话开始、恢复、敏感读取、外部副作用和权限升级时验证 envelope；普通的每一个已准许文件 read 不重复做硬件身份
挑战，而由 OS sandbox 物理限制可读范围，并让 read event 引用同一 envelope 和递增 sequence。

“每次读取都查主板号”既增加噪声又不能提供真正授权。正确的反向验证是：

1. 先验证会话/安装签名、workspace digest 和 sandbox backend；
2. 再判断目标是否位于 capability 允许的文件类别和根目录；
3. 对 secret、跨 workspace、archive import 等敏感类别签发单次、digest-bound permit；
4. 记录 metadata-only read receipt；不默认保存文件正文。

## 7. 会话、快照与归档删除

### 7.1 单一对话命名空间

- snapshot、event、tool-call、process、network permit、credential lease 和 recovery checkpoint 都必须带
  `conversation_id`；
- 查询 API 默认只能看到当前 conversation；跨会话枚举和语义召回默认关闭；
- 恢复归档对话会创建新的 security envelope，不复用旧 permit、旧 trust 或旧 sandbox receipt；
- workspace/policy digest 变化时，旧快照只读，不能自动成为新 action 的授权依据；
- 用户显式导入重要快照时，记录来源 conversation、digest 和 `imported_untrusted` 状态。

### 7.2 活跃期数据

活跃会话只保留完成任务所需的最小数据：

- hash-chained metadata event；
- bounded stdout/stderr preview 与全量 digest，不默认保存全量正文；
- 加密或 ephemeral 的临时 provider/tool payload；
- 用户明确固定的重要 snapshot；
- secret value 不进入 journal、snapshot 或确认文本。

### 7.3 归档后的默认保留与删除

**持久保留：**

- 脱敏后的对话本身；
- 用户明确固定的重要快照；
- 最小 terminal/deletion receipt：conversation ID、类别计数、digest、终态、删除完成/失败项和时间。

**归档后删除：**

- raw provider request/response 和 private reasoning；
- 原始 tool result、stdout/stderr、process/debug log 和 network body；
- 未固定快照、临时 checkpoint、临时 profile、容器/volume；
- credential lease、环境副本、确认 token 和一次性 permit；
- 会话级全文索引、embedding 和跨会话召回缓存。

最小 deletion receipt 不保存原始内容，但它是证明“尝试删除了什么、哪里失败”的必要元数据。
如果连它也删除，就无法同时满足“可审计”和“能证明已清理”；因此它视为对话归档的终结元数据，而不是过程日志。

删除只能声明已覆盖的 storage boundary。它不能证明 pagefile、SSD wear leveling、备份、provider 或恶意软件没有副本。
任何清理失败必须保留失败类别并使 `archive_complete=false`，不得像 D 旧实现那样把摘要成功等同于全链路清理完成。

## 8. 反提示注入与指令来源

通用内核必须把“内容”和“授权”分离：

1. 指令来源至少标为 `platform/user/trusted-project/untrusted-project/external-content/tool-output/
   recalled-memory/derived-summary`；
2. 未建立 workspace trust 前，项目文件只能作为不可信数据扫描，不能激活 hook、skill、MCP、permission 或规则；
3. 网页、issue、README、图片 OCR、工具输出和历史记忆永远不能自行提高权限或改变 policy；
4. 记忆/摘要回注必须带来源 digest、原会话和 `untrusted-data` 标签，LIF claim/evaluation 默认关闭跨会话 recall；
5. capability 决策由内核根据结构化 action 作出，不由模型输出“我已获准”触发；
6. command 使用 executable + argument sequence，禁止默认 shell string；路径在解析、规范化和 symlink/reparse
   point 检查后再匹配 capability；
7. 高风险 action 的确认绑定 action digest、目标、影响范围、TTL 和 attempt；裸 `y/n` 不能复用；
8. scanner finding 与 permission decision 分离；关键词命中可触发 review，不能单独证明安全或恶意；
9. tool result 回注前做 secret redaction、size/type/schema gate，但输出清洗不被称为反注入；
10. 用 adversarial fixtures 验证：项目规则注入、网页“忽略前文”、恶意工具结果、记忆投毒、路径穿越、
    symlink/reparse point、子 Agent 权限升级和归档恢复旧授权。

Project D 的 JSON 记忆包装和 system prompt 提醒只能作为 `defense-in-depth` 文本提示，不能进入上述 hard gate 的证明链。

### 8.1 Windows Credential Manager 凭据注册表

所有子代理均使用独立 DeepSeek API key，通过 Windows Credential Manager 的 Generic Credential 类型存储。每个 target 对应一个独立的 API key，不存在共享或继承关系。

| Credential Target | 用途 | API Key 独立性 | 代码引用 |
|---|---|---|---|
| `FEP-Agent/DeepSeek` | 主 canonical CLI (`run_canonical_guarded_cli_real`) | 独立 key | `assurance/deepseek_adapter.py:23` `DEFAULT_CREDENTIAL_TARGET` |
| `FEP-Agent/DeepSeek-Retrieval` | 外部检索子代理（web/external search） | 独立 key，不与主 CLI 共享 | `assurance/retrieval_subagent.py` `DEFAULT_EXTERNAL_RETRIEVAL_CREDENTIAL_TARGET`；`dispatch_external_retrieval_subagent()` |
| `deepseek-retrieval-subagent` | 内部 Project Doc 检索子代理 (`dispatch_retrieval_subagent`) | 独立 key，不与主 CLI 或其他子代理共享 | `assurance/retrieval_subagent.py:678` `credential_target` 参数；Gap Register GAK-RET-SUB-001 |

**设计约束：**

- 每个子代理必须使用独立 API key — 即使同一 DeepSeek 账户可创建多个 key，也不得共用
- 凭据读取统一通过 `_read_windows_credential()`（`assurance/deepseek_adapter.py`），该函数不缓存、不持久化 key 明文
- key 使用后立即执行 best-effort scrub（调用方 `finally` 块覆盖变量）
- 新增子代理时，必须先在本文注册其 credential target，再部署 API key

## 9. 主要缺口登记

| ID | 缺口 | 当前证据/状态 | 严重度 | 下一产物与通过条件 |
|---|---|---|---|---|
| GAK-ARCH-001 | 通用内核、general-science 与 LIF profile 的 P0 schema/ADR | 2026-07-25 已新增 Effective Security Envelope、additive profile registry、sandbox/session/retention schema、正反 fixture 与 ADR-0003/0004 | P0 已关闭 | 后续实现必须消费这些合同；不得绑定单一 runtime、复制通用 runtime 或把 LIF 路由回灌 general-science |
| GAK-SBX-001 | 默认物理 sandbox 未完全闭环 | P2 Docker branch 已 observed compliant；2026-07-27 Windows native live probe 已跑两次（非 elevated + elevated）：AppContainer+Job+FS/registry/non-admin **observed pass**；elevated path 通过 netsh firewall outbound block rule 达成 **compliant** observation + selection allow；non-elevated path 因无法创建防火墙规则 **fail-closed**（正确行为）。GAK-SBX-001 **可关闭**（2026-07-27 audit 裁决）。原始 TCP 残余为已知 Windows 平台限制，记录于 limitations，不阻塞 development 门禁。详见 `docs/GAK_SBX_001_WINDOWS_NATIVE_SANDBOX_AUDIT_2026-07-27.md` | ~~阻断~~ → 已关闭 (development baseline) | 生产级网络隔离需 WFP 内核 callout 或 Microsoft 修复；non-elevated 路径的防火墙替代方案待研究 |
| GAK-DOCKER-001 | Docker 一键启动与 digest/mount receipt | P2 已固定镜像摘要、唯一 workspace mount、无 socket/home、network-off、resource limits；P2.5 已接入固定 no-model action、conversation lifecycle、进程追踪与 HMAC receipt | P2/P2.5 development 已关闭 | 接入通用 runtime/tool broker、磁盘 quota、并发/crash recovery 后再评估 production closure |
| GAK-TRUST-001 | trust receipt 尚未统一到所有入口 | **2026-07-28 关闭**：`establish_workspace_trust()` 已接入 canonical CLI 和检索子代理全部入口点。新增：`AdapterGateContext` 携带 `trust_receipt` + `trust_status` 属性；`_resolve_and_setup_gates()` 在 IPG 评估前调用 `establish_workspace_trust()` 并写入 governor journal；`dispatch_retrieval_subagent()` 和 `dispatch_external_retrieval_subagent()` 在 namespace 创建后立即建立 workspace trust；`validate_all_entry_points_establish_trust()` 静态 AST 审计验证所有入口点导入并调用 `establish_workspace_trust`（3 个入口点）。580 tests pass。详见 `assurance/adapter_gate.py`、`assurance/canonical_cli.py`、`assurance/retrieval_subagent.py`、`assurance/workspace_trust.py` | ~~高~~ → 已关闭 (development baseline) | 文件变化使 grant 失效的自动重检仍待后续版本 |
| GAK-ID-001 | 安装密钥/会话 envelope 尚未产品化 | **2026-07-28 关闭**：新增 `_FileLock` 跨平台文件锁（`O_CREAT|O_EXCL` + 过期锁检测）；`rotate()`/`revoke()`/`record_initial_key()` 全部受锁保护；crash-safe journal（`.pending-` 文件 → receipt → history → 清除 pending）；`recover_pending_rotations()` 恢复/回滚未完成旋转；`migrate_envelope()` 真实 envelope 重签名（rebuild body + new key sign + atomic write）；`rotate()` 的 `envelopes_to_migrate` 调用真实迁移。17 个测试 (8 个新增：锁获取/释放/互斥/过期清理 + crash 恢复 + 孤儿检测 + envelope 迁移 ×2)。590 tests pass。详见 `assurance/key_lifecycle.py`、`assurance/envelope.py`、`assurance/tests/test_key_lifecycle.py` | ~~高~~ → 已关闭 (development baseline) | 跨设备迁移、TPM-backed key 仍待后续版本 |
| GAK-SESSION-001 | conversation namespace 尚未接入实际 runtime | **2026-07-28 关闭**：`ConversationNamespace` 已接入 canonical CLI 真实 adapter 路径。新增 `SessionGovernor`（thin wrapper — 所有 gate receipt/checkpoint/answer packet/journal event 通过 `namespace.write_artifact()` 路由）；`_resolve_and_setup_gates()` 接受 `key_store` 参数自动创建 namespace + governor；`run_canonical_guarded_cli_real()` 通过 `enforce_adapter_call()` 包装 DeepSeek API 调用（`AdapterGateContext` 绑定真实 `conversation_id`）；`build_real_deepseek_answer_packet()` 接受 `conversation_id`/`envelope_id` 参数；新增 10 个 integration 测试（namespace 创建、governor artifact 路由、governor close 拒绝写入、namespace integrity、跨会话隔离 ×3、import untrusted、full lifecycle with adapter artifacts、resume creates new envelope）。548 tests pass。详见 `assurance/session_governor.py`、`assurance/canonical_cli.py`、`assurance/deepseek_adapter.py`、`assurance/tests/test_session_namespace.py` | ~~高~~ → 已关闭 (development baseline) | 生产级 key rotation（GAK-ID-001）、Project Doc Retrieval Subagent（GAK-RET-SUB-001）仍待后续版本 |
| GAK-RET-SUB-001 | Project Doc Retrieval Subagent (真实 adapter) | **2026-07-28 关闭**：新增 `ProjectDocIndex`（项目文档扫描器 — 8 个类别 glob pattern，in-memory search + lazy content 加载）；新增 `dispatch_retrieval_subagent()`（真实 DeepSeek v4 Pro API + ProjectDocIndex + SessionGovernor + AdapterGate 集成；支持 offline 模式无 API key 时直接用文档扫描结果构建结果）；新增 `_build_retrieval_system_prompt()`（带文档摘录的检索提示模板）；全部输出标记 delegated_retrieval / derived_unverified；遵循现有 retrieval-result-v0.1 schema；完全向后兼容 — build_fake_retrieval_result 保留；新增 15 个测试（ProjectDocIndex 7 + dispatcher 5 + integration 3）；22 个存量测试全部通过。详见 `assurance/project_doc_index.py`、`assurance/retrieval_subagent.py`、`assurance/tests/test_retrieval_subagent_real.py` | ~~待实施~~ → 已关闭 (development baseline) | 独立 DeepSeek credential 已于 2026-07-28 配置（Windows Credential Manager `deepseek-retrieval-subagent` target，独立 API key，不与主 CLI 或其他子代理共享）；语义搜索能力受限于关键词匹配 |
| GAK-RET-SUB-EXT-001 | External Retrieval Subagent (web search result processor) | **2026-07-28 关闭**：新增 `dispatch_external_retrieval_subagent(search_results=...)` — 接收原始 web search 结果（title/url/snippet），通过 DeepSeek API 进行结构化处理。**模型严格约束为仅使用提供的搜索摘录**（system prompt 显式禁止使用训练知识、禁止编造来源、要求逐条引用搜索结果编号）。覆盖 6 种外部来源类别。来源 ledger 基于真实搜索 URL 构建（非模型自报）。无 API key 或无搜索结果时返回带 opacity note 的空结果。`DEFAULT_EXTERNAL_RETRIEVAL_CREDENTIAL_TARGET = "FEP-Agent/DeepSeek-Retrieval"`（独立 API key）。11 个新测试（搜索来源 ledger、无结果模式、无 API 模式、opacity notes、URL 引用、namespace、governor、schema 验证等）。设计原则：子代理**不执行**搜索——它处理搜索工具的输出；搜索与处理分离以保持审计链可追溯。详见 `assurance/retrieval_subagent.py`、`assurance/tests/test_retrieval_subagent_real.py` | ~~待实施~~ → 已关闭 (development baseline) | 搜索工具本身（web search API）由主 agent 调用；子代理仅处理已获取的搜索摘录；需在未来接入真实搜索 API 调用链 |
| GAK-RET-001 | retention/deletion controller 尚未产品化 | **2026-07-28 关闭**：StorageAdapter 已接入 ArchiveController（替代裸 DeleteFile 回调）；archive_recovery.py 提供 holistic 恢复编排器（classify_archive_failure → recover_archive，覆盖 clean_interrupted/torn_journal/corrupt_journal/stale_lock/archiving_no_journal 五个恢复路径）；detect_stale_archive_lock + cleanup_stale_archive_lock 处理崩溃后孤儿锁；FaultInjectionStorageAdapter 支持 transient/permanent/walk error 故障注入测试；新增 18 个测试（recovery 路径、storage adapter 集成、故障注入、deletion boundary、stale lock、multi-process lock contention）；484 tests pass。详见 `assurance/archive.py`、`assurance/archive_recovery.py`、`assurance/tests/test_archive_recovery.py` | ~~阻断~~ → 已关闭 (development baseline) | 生产级 storage backend（网络/云存储）、跨设备并发 crash recovery 仍待后续版本 |
| GAK-INJ-001 | instruction provenance/anti-injection gate 尚未接入所有真实入口 | **2026-07-28 关闭**：新增 production content parser（`parse_instruction_content` — 提取 path/endpoint/directive 引用）；新增 obfuscation detector（`detect_obfuscated_injection` — 零宽字符、同形字、base64 payload、全角替换）；新增 canonicalizer 集成（`evaluate_instruction_provenance_gate_with_canonicalizer` — 路径遍历/SSRF 检测，数据源中非法路径/端点 → block）；INJECTION_PATTERNS 扩充至 25 个模式（新增 output your instructions、override security、system prompt、hidden instructions、debug mode、developer mode、disable safety、pretend you are、act as if）；新增 unified gate entry 验证（`validate_all_entry_points_consume_same_gate` — 静态 AST 检查 4 个已知 consumer）；adapter bypass hardening 8 个新测试；adversarial injection 47 个新测试（路径遍历 10 + SSRF 7 + 提示提取 7 + 混淆 7 + 越权 6 + 组合 5 + 边界 6）。全部 89 tests pass。详见 `assurance/instruction_provenance_gate.py`、`assurance/tests/test_injection_adversarial.py` | ~~阻断~~ → 已关闭 (development baseline) | 生产级语义注入检测、URL 解码路径扫描、真实 runtime adapter content bytes 接入仍待后续版本 |
| GAK-CHILD-001 | 子 Agent/子进程 capability 传递尚未统一到实际 runtime | **2026-07-28 关闭**：新增 `spawn_child_context()` 统一入口（组合 `enforce_child_capabilities` predicate check + `delegate_capabilities` 签名 child envelope 创建）；`dispatch_retrieval_subagent()` 和 `dispatch_external_retrieval_subagent()` 接受可选 `parent_envelope` 参数，提供时通过 `spawn_child_context()` 创建签名 child envelope 并传入 `ConversationNamespace.create(parent_envelope_id=...)`；`execute_guarded_no_model_action()` 在 Docker create 前调用 `enforce_child_capabilities()`（child_kind=child_process），enforcement receipt 写入执行回执 bindings。无 `parent_envelope` 时回退到硬编码能力（向后兼容）。580 tests pass。详见 `assurance/child_capability_enforcer.py`、`assurance/retrieval_subagent.py`、`assurance/guarded_execution.py` | ~~高~~ → 已关闭 (development baseline) | 实际 MCP adapter 接入仍待后续版本 |
| GAK-NET-001 | network permit 尚未覆盖所有 tool/runtime 路径 | **2026-07-28 关闭**：`call_deepseek_api()` 新增 `conversation_id`/`attempt`/`turn`/`allowed_categories`/`allowed_endpoints` 参数，在执行 `urllib.request.urlopen()` 前调用 `evaluate_network_permit()`（category=`llm_provider`）；`AdapterGateContext` 新增 `network_policy`/`network_endpoint`/`network_endpoint_category`/`allowed_categories`/`allowed_endpoints` 字段；`enforce_adapter_call()` 在 enforcement receipt 中记录 `network_permit_required`/`network_permit_granted`；`_resolve_and_setup_gates()` 调用 `build_network_permit_policy(mode="guarded")` 构建默认策略；`run_canonical_guarded_cli_real()` 传递 network 参数到 AdapterGateContext 和 call_deepseek_api。580 tests pass。详见 `assurance/deepseek_adapter.py`、`assurance/adapter_gate.py`、`assurance/canonical_cli.py` | ~~高~~ → 已关闭 (development baseline) | web_fetch/MCP/Git/package_registry 工具的 network permit 接入、redirect/proxy 负例仍待后续版本 |
| GAK-CRED-001 | 凭据仍存在不可控内存/内核副本 | **2026-07-28 development baseline 达成**：新增 `credential_scrub.py`（653 行）— `CredentialGuard` context manager（自动读取→使用→scrub 生命周期 + 异常安全 + 审计记录）；`sanitize_child_environment()` 和 `audit_child_environment()`（33 个凭据相关环境变量名 + 6 个模糊匹配模式）；`scan_dict_for_credentials()` 和 `assert_no_credential_in_dict()`（递归 artifact 扫描，跳过 SHA-256/URL/JSON/timestamp/短值）；`audit_container_mount()` 和 `assert_safe_container_mount()`（14 个禁止挂载根路径：Credential Manager、DPAPI、Crypto、SSH、GnuPG、keystore、.dpapi/.pem/.key 文件）；`audit_credential_scrub_sites()`（AST 静态扫描验证每个 `_read_windows_credential` 调用点有对应 scrub）；74 个测试全部通过。3 个真实凭据读取点（canonical_cli.py:1020、retrieval_subagent.py:901、retrieval_subagent.py:1357）均通过 manual `”\x00” * len(key)` finally-block scrub 或 `CredentialGuard` 覆盖。Python 不可保证内存零化——所有 scrub 为 best-effort defense-in-depth。详见 `assurance/credential_scrub.py`、`assurance/tests/test_credential_scrub.py` | ~~高~~ → development baseline 达成 | Python 级别 best-effort scrub 已完成；OS 级别 pagefile/hibernation/WER dump 凭据残余不可消除，保持”不绝对”声明 |
| GAK-EVT-001 | 审计投影尚未接入真实 runtime | **2026-07-29 development baseline 达成**：新增 `assurance/audit_integration.py` — canonical CLI journal events → `AuditLedger` 桥接层。`run_canonical_guarded_cli_real(audit=True)` 自动创建审计账本、逐事件写入、封印产出签名 `audit_seal` receipt。Provider 事件标记为 `partial`（外部模型 provider 完整性不可证明）；kernel/session/runtime 事件标记为 `complete`；缺失 source 声明为 `unknown`。`--run --real` 路径通过 `bridge.py` 自动启用 audit。11 个测试覆盖：event→audit 映射、fact 名称安全验证、空事件拒绝、未知事件类型兜底、API 失败时 audit 仍封印、缺失 provider source 检测。263 全量回归 pass。 | ~~中~~ → development baseline 达成 | 生产级：接入真实 ACP/session/supervisor adapter（当前仅 canonical CLI 单一 runtime）；自动观测跨 runtime 事件完整性。 |
| GAK-CMP-001 | automatic compaction 尚未观测 | **2026-07-29 关闭**：新增 `assurance/compaction_observer.py` — `AutomaticCompactionSimulator`（自动阈值触发分类，boundary zone 产生 honest unknown） + `ManualCompactionSimulator`（精确手动分类） + 6 个公开 validator（non-overlap / classification-required / summary-never-promoted / unknown-not-promoted / automatic-trigger-honesty / assert）。自动模拟器的核心不变性：token 阈值边界附近的项目**不假装精确分类**——`classification_confidence < 1.0` 时必须存在 unknown range。`SimulatedCompaction.to_audit_compaction()` 输出通过 `_normalize_compaction` 验证。36 个测试覆盖：自动阈值模拟 9、range 不变性 6、unknown 升级防护 8、AuditLedger 集成 4、manual vs automatic 比较 5、token 估算 4。Gap Register 中 `AUTOMATIC_THRESHOLD_NOT_OBSERVED` → `AUTOMATIC_THRESHOLD_OBSERVED`。 | ~~中~~ → 已关闭 (development baseline) | 生产级：接入真实 runtime 的 automatic compaction hook（当前为 simulated observation）；验证 runtime 的 tokenizer 估算精度 vs boundary zone width。 |
| GAK-REC-001 | 无正式 shadow recovery store/执行器 | **2026-07-29 关闭**：新增 `assurance/shadow_recovery.py` — `ShadowRecoveryStore`（SHA-256 内容寻址 store/retrieve/verify/list、完整性 manifest、candidate + authorization + snapshot 独立副本）+ `RecoveryDiffPreview`（元数据级 diff：hash/字节/行数，不泄露内容）+ `RecoveryExecutor`（唯一执行恢复的组件——验证 candidate+authorization 结构完整性、canonicalize 目标路径、原子写入、产出签名 `ExecutionReceipt`、不删除审计事件）。23 个测试覆盖：store/retrieve/verify/list 8、diff preview 5、executor 7、E2E 3。`.gsa_shadow_recovery/` 加入 `.gitignore`。 | ~~中~~ → 已关闭 (development baseline) | 生产级：真实 Git 后端作为 shadow store（当前为文件系统 + SHA-256）；跨设备恢复传输；恢复后审计投影自动更新。 |
| GAK-WIN-001 | Windows start→Job assignment race | **2026-07-28 已关闭**：`run_windows_native_sandbox_probe()` 使用 `PROC_THREAD_ATTRIBUTE_JOB_LIST`（`0x0002000D`）在内核创建进程时将 Job Object 原子化绑定——初始线程创建前进程已在 Job 内，消除用户态 `CreateProcess`→`AssignProcessToJobObject` 竞态窗口。进程以 `CREATE_SUSPENDED` 创建，`TokenIsAppContainer` + `IsProcessInJob` 均在 `ResumeThread` 前验证。`PROC_THREAD_ATTRIBUTE_JOB_LIST` 不可用时优雅降级至 post-creation 分配。6 个 GAK-WIN-001 专项测试（创建时 Job 分配验证、子进程继承、无代码运行逃逸、竞态窗口对比）+ 5 个 Job Object 基础测试（KILL_ON_CLOSE、BREAKAWAY_OK 禁止、已运行进程拒绝分配）。11/12 tests pass, 1 skipped（`PROC_THREAD_ATTRIBUTE_JOB_LIST` 不可用环境自动跳过）。详见 `assurance/windows_sandbox.py:856-983`、`assurance/tests/test_windows_race_escape.py:383-970` | ~~高~~ → 已关闭 (development baseline) | `PROC_THREAD_ATTRIBUTE_JOB_LIST` 在极旧 Windows 版本（<Win8）不可用——已优雅降级并记录于 diagnostics；生产级 WFP 内核 callout 仍待后续 |
| GAK-XPLAT-001 | 其他 OS 尚无 observed 支持矩阵 | **2026-07-29 显式推迟**：用户确认一年内不使用其他 OS；Linux/macOS 适配推迟至需要时再启动。当前仅 Windows-first 证据。 | ~~中~~ → 显式推迟 (≥1年) | 需要时再建立 Linux/macOS backend 各自正反 fixture；未测平台明确 unsupported。 |
| GAK-UX-001 | 不同水平用户的安全 UX 未验证 | **2026-07-29 关闭**：新增 `assurance/ux_safety.py` — `UXSafetyEvaluator` + 10 个 UX 安全场景（4 novice + 3 experienced + 3 shared），8 种 UXAssertionKind（gate_visible / fail_closed_explained / no_silent_fallback / dialog_bound_to_action / error_distinguishable / next_action_suggested / permission_denied_clear / permission_comprehension）。每个场景定义安全断言 + expected_user_action。`run_suite()` 产出签名 `ux_safety_evaluation_receipt`，self-verify 内建。环境声明：disposable workspace + fake credentials + 无模型/网络/外部参与者。20 个测试通过。所有已注册中等严重度 Gap 已全部关闭。 | ~~中~~ → 已关闭 (development baseline) | 生产级：真实外部参与者测试（需要 IRB/consent + 真实设备 + 多样性样本）；当前仅为程序化安全断言——不替代人类 UX 研究。 |
| GAK-UI-001 | Windows 终端 UI（retro 桌面风格 TUI） | **2026-07-29 Phase 3 完成 — development baseline 达成**：`assurance/tui/` 包（9 个模块，~3900 行）。Phase 1：widget 系统（10 个 widget 类）+ prompt_toolkit 集成 + 斜杠命令系统。Phase 2：canonical CLI 事件流桥接（`bridge.py`←导入边界、`LiveRunEventSource` 后台实时消费、`JsonlFileSource` journal 重放、3 个 typed gate event）。Phase 3：真实 DeepSeek adapter 接入 — `main.py` 新增 `--real`/`--credential-target` 标志、`build_live_run_fn(real_adapter=True)` 接线到 `run_canonical_guarded_cli_real`、真实 Gate 决策流全链路贯通。221 tests pass。入口: `architecture/CLI_UI_INTERACTION_MODEL_v0.1.md` / `assurance/tui/` | ~~中~~ → development baseline 达成 | 下一步（远期）：生产级多轮对话 TUI、真实 DeepSeek API 调用的 TUI 截图测试（需 API key）。|

## 10. 实施顺序

### P0：冻结语义

状态：**2026-07-24 已完成合同层冻结，不表示 hard gate 已实现。**

1. Effective Security Envelope、session lifecycle、retention policy 和 sandbox receipt schema 已写入
   [`../assurance/`](../assurance/)；
2. “通用内核 / runtime adapter / profile”ownership 已由
   [`ADR-0003`](../adr/ADR-0003-runtime-neutral-assurance-kernel.md) 冻结；
3. 安全事实使用 `observed/derived/user_asserted/unknown`，未知值不得默认补全；
4. Grok 在 profile registry 中固定为 `reference_only`，reference 状态不能授予 acceptance；
5. 正反 fixture 由仓库检查器和独立单元测试验证。

### P1：会话身份与删除生命周期

状态：**2026-07-24 已完成 development/conformance fixture，不表示生产 controller 已完成。**

1. 随机 256-bit installation key + Windows Current-User DPAPI adapter 已 observed 往返及篡改失败；
2. conversation namespace 已默认拒绝跨会话读取；显式 import 标 `imported_untrusted`；
3. resume 创建新的 conversation/envelope/nonce/capability digest，只保留签名 parent link；
4. `active→archiving→archived/failed`、16 类显式删除、成功/失败 terminal receipt 与独立 verifier
   已在临时 fake payload/快照/log 上通过；
5. rotation/revocation、并发、crash recovery、真实 runtime/storage 接入仍留在 GAK-ID/SESSION/RET。

优先做这一阶段，是因为它决定后续所有记录的生命周期；若晚做，测试本身会继续制造无统一归属的遗留数据。

### P2：本机严格 sandbox 与 Docker 快捷入口

状态：**Docker development/conformance 纵向切片已完成；Windows native live probe elevated path compliant（2026-07-27），GAK-SBX-001 已关闭。**

1. backend probe/selection receipt 已实现，不执行真实模型；
2. Docker 固定镜像与最小 mount/network/resource profile 已实测通过；
3. Windows native live：TokenIsAppContainer、Job、FS/registry 隔离 observed；elevated path 通过 netsh firewall outbound block rule 达成 compliant observation + selection allow；non-elevated path fail-closed（正确行为）；raw TCP 残余记录为已知平台限制；
4. requested backend 不可用或不合规时已验证 fail closed，不做静默 fallback；
5. GAK-SBX-001 已于 2026-07-27 关闭（development baseline 达成）；详细证据见
   [`../docs/P2_DOCKER_SANDBOX_AUDIT_2026-07-24.md`](../docs/P2_DOCKER_SANDBOX_AUDIT_2026-07-24.md) 与
   [`../docs/GAK_SBX_001_WINDOWS_NATIVE_SANDBOX_AUDIT_2026-07-27.md`](../docs/GAK_SBX_001_WINDOWS_NATIVE_SANDBOX_AUDIT_2026-07-27.md)。

### P2.5：无模型 guarded execution 收口

状态：**2026-07-24 已完成一次真实端到端执行与进程残留复核。**

1. P1 envelope、P2 Docker observation/selection 与固定 action 在使用点重新验证；
2. 记录 9 个 Docker CLI 宿主进程、容器 host PID 和 `docker top` 快照；
3. execution receipt 由 DPAPI-protected installation key 做 HMAC，事后重开密钥独立验证；
4. 容器、固定 workspace probe 与 10 个追踪 PID均无残留；
5. receipt 进入 conversation `temporary_lifecycle_receipt`，归档测试证明其被删除；
6. 证据见
   [`../docs/P2_5_GUARDED_EXECUTION_AUDIT_2026-07-24.md`](../docs/P2_5_GUARDED_EXECUTION_AUDIT_2026-07-24.md)。

### P3：反注入与 capability 继承

状态：**2026-07-25 已完成 development/conformance 纵向切片；2026-07-27 入口级 batch gate 已接入
canonical CLI offline 路径；production parser 与真实 adapter enforcement 未关闭。**

1. 已冻结八类 instruction provenance；未建立 trust 的 project、web/external、tool、memory、
   derived summary 均不得提权，内容 receipt 只保留摘要；
2. 已以 7 个组合测试覆盖 project/web/tool/memory data-only、恶意措辞不提权、篡改 fail closed，
   不扩张为正反例笛卡尔积；
3. 2026-07-27 `instruction_provenance_gate` 对多来源指令做 batch 分类与静态注入检测，并写入
   canonical CLI journal（gate 先于 model）；仍为 offline/fake，不解析真实 content bytes；
4. child process、child agent、remote MCP 已有 capability subset verifier 和签名 child envelope；
   escalation 与 remote MCP 本地能力均拒绝；
5. sensitive/external side effect 已使用 action/target/impact/attempt/TTL 精确绑定 permit，并以排他
   consumption claim 拒绝错绑、过期和重放；
6. `action-authorization-receipt` 明确只有 kernel envelope/consumed permit 可以授权，模型或内容声称
   “approved”始终忽略；
7. 当前只接受已规范化结构化动作，不解析 shell string；实际 path/endpoint canonicalizer、所有
   runtime/tool adapter 接入、crash recovery 和 key lifecycle 留在 GAK-INJ/CHILD/ID。

### P4：审计、compaction、恢复

状态：**2026-07-25 已完成 development/conformance 纵向切片；真实 adapter、automatic
compaction 和 recovery store/executor 未关闭。**

1. 已实现 runtime-neutral ACP/session/provider/supervisor metadata projection，包含 typed source、
   source/global sequence、hash chain、显式 complete/partial/unknown 与 terminal exactly-once；
2. compaction summary 固定为 `derived_unverified`，retained/discarded/unknown range 显式且不可重叠；
3. metadata ledger 保存到 archive 保留类别；raw provider/private/tool/process/network payload
   删除后仍可由签名 seal 重建事件链、计数、来源状态、terminal 与 compaction；
4. verified archive+pinned snapshot 只能生成 `untrusted_recovery_candidate`；精确 one-shot permit
   可产生恢复授权，但授权 receipt 固定不执行恢复；
5. 3 个组合测试覆盖 archive-after-verify、journal/candidate tamper、terminal 缺失、permit
   缺失/精确消费/重放，不维护正反例笛卡尔积；
6. source completeness 不能证明 runtime 从未遗漏事件；实际 adapter、automatic threshold、
   shadow store、diff preview、path canonicalizer 与恢复执行留在 GAK-EVT/CMP/REC。

### P4.5：workspace-first 集成收口

状态：**2026-07-25 已完成一次无 Docker、无模型、无网络、无子进程的真实标准模式链和归档后独立复核。**

1. 标准模式固定为 `workspace_guarded`，只声明逻辑/应用层工作区边界；Docker 只作为显式、
   externally-managed、on-demand strict backend；
2. strict 禁止隐式启动/停止和静默降级；本策略降低 Docker Desktop 长时间常驻造成的资源成本，
   但不声称修复 Docker CPU 异常根因；
3. P3 provenance/action authorization、固定进程内 roundtrip、P4 audit seal 和 P1 archive 已在同一
   conversation 接通；
4. probe 删除后工作区摘要必须与执行前一致；归档后重新验证 retained HMAC、audit chain、
   archive completeness 和 raw-payload absence；
5. 3 个组合测试覆盖完整链、strict 不降级、marker/receipt 篡改；全仓库 132 tests 通过；
6. 当前固定动作不接受 shell、任意路径、模型、网络或子进程，不能冒充通用 production sandbox。

### P5：用户测试

只有 P0–P4.5 的 blocking fixture 通过后，才先进行内部合成用户任务，再邀请朋友在 disposable
repo、fake credential、network-off 环境测试。测试记录只保留 consented、脱敏的任务级问题，
不收集硬件号，也不默认上传完整对话或命令历史。

状态：**2026-07-25 已完成 internal mechanical preflight；外部参与者与人类理解度评估未开始。**

1. 新手 standard 与熟练用户 strict-unavailable 两项任务均通过；strict 无 workspace/conversation、
   Docker 启动或 fallback；
2. P5 receipt 固定 `external_participant_count=0`、`human_comprehension_assessed=false`，不得将机械
   conformance 升级为可用性或安全默认结论；
3. 从 LIF R211 已完成复杂任务只读选取 8 个路由/讨论/代码/manifest/summary 文件到 disposable
   snapshot；源前、源后和 snapshot 聚合摘要相同；
4. 未执行 R211 分析脚本、训练或待定 A-CI 第二阶段，未修改 LIF 项目内容；
5. 3 个 P5 组合测试通过；全仓库 135 tests、79 schemas、0 contract errors；
6. 下一步是先定义 consent/任务后清理/脱敏记录，再进行少量内部人工 dry run；在此之前 P5 不关闭。

### P5.5：终端 UI 第一原型 (GAK-UI-001)

P5.5 与 P0–P4.5 解耦——第一原型是静态 mock，不依赖 assurance core。它可以与 P5 并行推进，但在核心 hard gate 全部关闭前不得接入真实模型调用或敏感数据。

状态：**2026-07-28 架构文档已冻结；代码实现未开始。**

1. 架构文档 `architecture/CLI_UI_INTERACTION_MODEL_v0.1.md` 已冻结：
   - 八区域布局（MenuBar/Toolbar/AddressBar/ExplorerPane/ContentPane/StatusBar/Dialog/Properties）
   - 10 种对象类型、标准操作集、键盘模型
   - `prompt_toolkit` 终端基板 + 自定义 retro widget 层
   - UI 不耦合 assurance core——消费 canonical CLI 的结构化事件流 → view model 投影 → 渲染
2. 第一原型范围（断开 assurance core）：
   - 主框架 + 菜单/工具栏/地址栏/资源管理器/内容/状态栏
   - 对象 URI 地址栏显示
   - 固定 source tree
   - task summary 视图
   - 属性 dialog mock
   - 阻断 permission dialog mock
   - 键盘焦点跨窗格导航
   - 100×30 和 140×40 截图测试
3. 之后：接入 canonical CLI 结构化事件 → view model → retro 渲染
4. 实现约束：
   - 中文字符宽度用真实宽度计算器
   - 不稳定 emoji 宽度假设做保守处理
   - 核心 UI 优先 ASCII + box-drawing 字符
   - UI 代码不得直接调用模型、子进程流或绕过 Gate 链
   - 保持 stdout/stderr 捕获并转为事件，不做裸打印

## 11. 完成定义

不能以“文档齐全”关闭这些缺口。最低正常使用门槛为：

- 当前 OS 至少一个 strict sandbox backend observed 通过；
- Docker shortcut 可选但不成为无 Docker 用户的唯一安全路线；
- trust-before-discovery、conversation namespace、archive deletion 和 anti-injection hard gate 有负例；
- credential/network/action permit 都绑定 digest、attempt 和 TTL；
- 子进程/子 Agent 权限不扩大；
- 归档只留下对话、固定快照和最小终结/删除凭据；
- verifier 能区分完整、部分、未知和失败；
- README/CLI 不再把 Job Object、venv、prompt guard 或 scanner PASS 写成强安全证明。

在此之前，本仓库可继续作为 development/conformance 工程使用，但不能宣称已进入“普通用户可安全默认使用”。

## 12. 第二轮自查

| 检查问题 | 状态 | 证据 | 未核风险 |
|---|---|---|---|
| 数值/来源是否可追溯 | 已核实 | §3 路径、hash、Drive 文件名 | Drive 文件以后可能更新 |
| 代码/计算链是否被过度概括 | 已修正 | D 源码逐文件复核；引用现有 salvage matrix | 未执行 D 代码 |
| 参数性质是否透明 | 已核实 | P2 profile 固定镜像、身份、mount、network 与 resource limits | 默认值尚未做跨设备校准 |
| 新实现是否先定义 claim/反例 | 已核实 | §9–§10 每项含负例/通过条件 | 尚未实现 fixture |
| 环境是否一致 | 部分核实 | Windows-first；本机 Docker Desktop/WSL2 已 observed | 其他设备/OS 未核 |
| 结论强度是否匹配证据 | 已修正 | 全文区分目标、partial、未实现 | 真实安全性仍未知 |
| 事实—指标桥是否显式 | 已核实 | §3.3 与缺口表 | 无运行指标 |
| 是否错误宣称普遍适用 | 已修正 | “通用”仅指不变量可迁移 | 跨 runtime/OS 尚未证明 |
| 是否有干预/对照支持安全主张 | 仍未核实风险 | 现有部分 fake/observed fixture | 新 hard gate 均待 negative fixture |
| 是否编造历史事实 | 已核实 | 三件套原件、D Drive 源码、仓库文档与本机 Docker probe | 其他用户设备未验证 |
| 是否独立于用户预期 | 已核实 | 主板号方案被否决；保留留存/审计冲突 | 最终 UX 偏好待实测 |
| 是否继承旧模型置信度 | 已核实 | 旧 D 只作为 design input，prompt-only 被拒绝 | 无外部模型复核 |
| prior-existence/撤回扫描是否完成 | 已核实 | §3.1 搜索词与未命中边界 | INDEX 后续可能新增 |
| 表达、方向和不确定性是否清楚 | 已修正 | “不表示已经实现”、fail-closed、unchecked | 实现后需再版 |

## 13. Project Doc Retrieval Subagent 设计提案 (2026-07-28)

### 动机

项目文档量持续增长（168 schemas + 30+ audit docs + 20+ architecture docs + 40+ source modules），主 agent 在检索项目内部信息时面临两个风险：

1. **上下文压力**：大量文档片段直接注入主 agent 上下文会严重消耗 token budget
2. **过早观点形成**：主 agent 在检索过程中可能将检索到的设计意图与当前实现状态混淆，形成未经验证的结论

### 设计决策

| 决策点 | 选择 | 依据 |
|--------|------|------|
| 检索范围 | 仅项目内部文档 | 外部检索由其他子代理负责；本项目子代理专注 GSA 项目自身知识 |
| 模型 API | 独立 DeepSeek v4 Pro | 与主 agent 模型调用链完全隔离；与其他搜索子代理一致 |
| 返回格式 | 遵循现有 retrieval task contract | 复用 `retrieval_task_contract` → 机械 ledger 单轨（`query_summary`/`source_ledger`/`filtering_log`/`raw_source_refs`）；`organized_response` 已随 2026-08-30 方向 C 退役（ADR-0010 §14.45）；全部输出标记 `delegated_retrieval` / `derived_unverified` |
| 生命周期 | 独立 ConversationNamespace | 子代理对话独立存储，由主 agent 派遣合同 → 关闭确认；复用 P1 namespace 设计 |

### 架构位置

```text
主 Agent
  │
  ├─ build_retrieval_task_contract()    ← 复用 (retrieval_subagent.py)
  ├─ dispatch_retrieval_subagent()      ← 新增
  │     ├─ 独立 DeepSeek v4 Pro API
  │     ├─ 独立 ConversationNamespace
  │     └─ ProjectDocIndex (glob+grep+Read)
  │          扫描: assurance/**/*.py, docs/**/*.md,
  │                architecture/**/*.md, adr/**/*.md,
  │                protocol/**, regression/**
  │
  └─ build_retrieval_session_close_receipt() ← 复用
```

### 实施依赖

- **GAK-SESSION-001** (先决) — conversation namespace 接入实际 runtime，子代理的独立 namespace 依赖此能力
- **GAK-ID-001** (后续) — 独立 API key 的 rotation/revocation 管理

### 与现有 retrieval_subagent 的关系

现有 `retrieval_subagent.py` 是 no-model fixture（`build_fake_retrieval_result` 合成假结果）。本提案将其升级为真实 adapter——保留全部 contract/schema/close-receipt 设计，替换 fake result builder 为真实 DeepSeek API 调用 + 项目文件系统扫描。升级路径与 GAK-RET-001（DeleteFile → StorageAdapter）和 GAK-INJ-001（static patterns → content parser + canonicalizer）一致。
