# 通用 Assurance Kernel 安全与可审计迁移缺口登记 v0.1

状态：2026-07-23 目标架构与缺口登记。本文记录已接受的迁移方向，但**不表示所列能力已经实现**。
Grok Build 仍是当前唯一通用 Agent runtime；本文只把运行时外侧可复用的保障能力拆成通用内核，
并把 LIF 科学保障保留为第一个高保障 profile，不建设第二套 model/tool/session runtime。

## 1. 结论

当前项目下一阶段不应继续增加零散提示词或只面向 LIF 的特例，而应形成：

```text
Grok / future ACP-compatible runtime
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
  - lif-research
```

这里的“通用”指保障不变量可迁移，不指支持任意 runtime、任意云服务或面向公众运营。
LIF profile 继续增加 SourceRouter、EvidenceKernel、ValidatorBridge、claim promotion、
countercase、blind-first review 和科学自查；它不再承载本可被其他任务复用的沙箱、身份、留存和反注入逻辑。

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
| [`WINDOWS_RUNTIME_CONTRACT_v0.1.md`](WINDOWS_RUNTIME_CONTRACT_v0.1.md) | `aa261c7033048d0228ad41b965f0258b30bbc4b7b6e7dd94b09f3587f13c7e7a` | Job Object、no-shell、输出与取消边界 |
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
- **user-supplied / 已部分核实**：用户已安装 Docker；本轮没有执行 Docker 命令，因此只作为待实现环境入口。
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

## 9. 主要缺口登记

| ID | 缺口 | 当前证据/状态 | 严重度 | 下一产物与通过条件 |
|---|---|---|---|---|
| GAK-ARCH-001 | 通用内核与 LIF profile 尚未形成正式 schema/ADR | 现有文档仍以 LIF 专项保障层表述；本文仅接受目标方向 | 高 | 新增 Effective Security Envelope schema、profile registry 和 ownership ADR；不得复制 Grok runtime |
| GAK-SBX-001 | 默认物理 sandbox 未闭环 | Job Object/防火墙有 observed evidence；不是文件/身份 sandbox | 阻断 | disposable workspace 中证明越界读写、子进程逃逸、网络外发均失败 |
| GAK-DOCKER-001 | 无 Docker 一键启动与 digest/mount receipt | 用户称已安装，本轮未执行核验 | 高 | 固定镜像、无 socket/home mount、network-off、resource limits；正反 fixture |
| GAK-TRUST-001 | trust receipt 尚未统一到所有入口 | Grok launcher 已有双 trust receipt；其他 future adapters 未统一 | 高 | discovery 前统一 receipt；文件变化使 grant 失效 |
| GAK-ID-001 | 无安装密钥/会话 envelope | 当前主要靠 session ID/文件 digest；无签名身份链 | 高 | OS keystore 随机密钥、conversation envelope、resume/rotation/revocation fixture |
| GAK-SESSION-001 | 记录未被机械限制在单 conversation namespace | 有 Grok session 和 prototype artifacts，但无统一 access controller | 高 | cross-session read 默认拒绝；显式 import 标 `imported_untrusted` |
| GAK-RET-001 | 无归档 retention/deletion controller | D 有“全部摘要成功后删 logs”；当前仓库无统一生命周期 | 阻断 | active→archiving→archived/failed 状态机、删除清单、negative fixture 和独立 receipt verifier |
| GAK-INJ-001 | 无通用 instruction provenance/anti-injection gate | D 主要是 prompt/JSON/关键词；当前 trust 只覆盖部分项目面 | 阻断 | 来源类型、不可提权规则、恶意 project/web/tool/memory fixture 全部 fail closed |
| GAK-CHILD-001 | 子 Agent/子进程 capability 传递未统一证明 | Windows child-tree containment 已测；权限继承仍依赖 runtime | 高 | child envelope 必须是父 envelope 子集；越权 spawn/remote MCP fixture 拒绝 |
| GAK-NET-001 | network permit 尚未覆盖所有 tool/runtime 路径 | DeepSeek one-shot 与 broker 已有窄证明 | 高 | endpoint/body/attempt/retry-bound permit 覆盖 web、MCP、Git、provider；redirect/proxy 负例 |
| GAK-CRED-001 | 凭据仍存在不可控内存/内核副本 | Credential Manager 短租约和 WER NOHEAP 已实现；文档明确非绝对零化 | 高 | 保持“不绝对”声明；扩展 child env、dump、artifact、container mount 负例 |
| GAK-EVT-001 | 审计投影 completeness 仍为 partial | ACP/session/provider/supervisor 可对账，但跨来源排序和密封未完整 | 中 | typed IDs、sequence/hash、source completeness、terminal exactly-once 独立重建 |
| GAK-CMP-001 | compaction/摘要与归档删除尚未统一 | manual compaction provenance 已测；automatic threshold 未观测 | 中 | retained/discarded/unknown 明示；摘要永远 `derived_unverified` |
| GAK-REC-001 | 只有 hash-only checkpoint，无正式恢复系统 | 现有文档明确不能恢复 | 中 | 独立 shadow Git/等价 store；恢复单独批准；不删除审计事件 |
| GAK-WIN-001 | Windows start→Job assignment race | 已记录真实 limitation | 高 | suspended creation 或 creation-time job attribute；逃逸负例 |
| GAK-XPLAT-001 | 其他 OS 尚无 observed 支持矩阵 | 只有 Windows-first 证据 | 中 | Linux/macOS backend 各自正反 fixture；未测平台明确 unsupported |
| GAK-UX-001 | 不同水平用户的安全 UX 未验证 | 尚无外部测试 | 中 | 先用合成仓库和假凭据完成新手/熟练用户 task test；不得先接真实项目 |

## 10. 实施顺序

### P0：冻结语义

1. 写 Effective Security Envelope、session lifecycle、retention policy 和 sandbox receipt schema；
2. 写“通用内核 / runtime adapter / profile”ownership ADR；
3. 给每个字段定义 `observed/derived/user_asserted/unknown`，未知值不得默认补全。

### P1：会话身份与删除生命周期

1. 随机安装密钥 + OS keystore adapter；
2. conversation namespace 和 resume 新 envelope；
3. active→archiving→archived/failed 状态机；
4. fake payload/快照/日志的删除正反例与 terminal receipt。

优先做这一阶段，是因为它决定后续所有记录的生命周期；若晚做，测试本身会继续制造无统一归属的遗留数据。

### P2：本机严格 sandbox 与 Docker 快捷入口

1. 先实现 backend probe/selection receipt，不执行真实模型；
2. 完成 Docker 固定镜像和最小 mount/network/resource profile；
3. 补 Windows native strict backend，明确 Job Object 只是其中一层；
4. requested backend 不可用时 fail closed。

### P3：反注入与 capability 继承

1. instruction provenance 类型和 policy-independent action parser；
2. project/web/tool/memory 四类注入 fixture；
3. 子进程、子 Agent、remote MCP 的 capability subset verifier；
4. sensitive read/external side effect 的 digest-bound permit。

### P4：审计、compaction、恢复

1. 统一 ACP/session/provider/supervisor 的 metadata-only audit projection；
2. compaction 与 archive retention 共用来源/digest/unknown 语义；
3. shadow recovery 与授权分离；
4. 证明归档删除 raw process/tool payload 后，最小 receipt 仍可独立验证。

### P5：用户测试

只有 P0–P4 的 blocking fixture 通过后，才邀请朋友在 disposable repo、fake credential、network-off
环境测试。测试记录只保留 consented、脱敏的任务级问题，不收集硬件号，也不默认上传完整对话或命令历史。

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
| 参数性质是否透明 | 不适用 | 本文未设置实验超参 | Docker/resource 默认值待实现 |
| 新实现是否先定义 claim/反例 | 已核实 | §9–§10 每项含负例/通过条件 | 尚未实现 fixture |
| 环境是否一致 | 已核实 | Windows-first；Docker 仅 user-supplied | 其他设备/OS 未核 |
| 结论强度是否匹配证据 | 已修正 | 全文区分目标、partial、未实现 | 真实安全性仍未知 |
| 事实—指标桥是否显式 | 已核实 | §3.3 与缺口表 | 无运行指标 |
| 是否错误宣称普遍适用 | 已修正 | “通用”仅指不变量可迁移 | 跨 runtime/OS 尚未证明 |
| 是否有干预/对照支持安全主张 | 仍未核实风险 | 现有部分 fake/observed fixture | 新 hard gate 均待 negative fixture |
| 是否编造历史事实 | 已核实 | 三件套原件、D Drive 源码、仓库文档 | 用户 Docker 状态未执行验证 |
| 是否独立于用户预期 | 已核实 | 主板号方案被否决；保留留存/审计冲突 | 最终 UX 偏好待实测 |
| 是否继承旧模型置信度 | 已核实 | 旧 D 只作为 design input，prompt-only 被拒绝 | 无外部模型复核 |
| prior-existence/撤回扫描是否完成 | 已核实 | §3.1 搜索词与未命中边界 | INDEX 后续可能新增 |
| 表达、方向和不确定性是否清楚 | 已修正 | “不表示已经实现”、fail-closed、unchecked | 实现后需再版 |

