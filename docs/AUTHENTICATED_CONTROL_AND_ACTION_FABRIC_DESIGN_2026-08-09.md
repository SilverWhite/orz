# Authenticated Control and Action Fabric（受信控制与动作授权面，ACAF）设计

> 状态：**design finalized**（2026-08-09 定稿；**Slice 1 已实施** 2026-08-12——
> 实施记录见 [`GAP_ACAF_SLICE1_IMPL_AUDIT`](audits/GAP_ACAF_SLICE1_IMPL_AUDIT_2026-08-12.md)，
> v1 简化决策 D1~D10 登记在该审计 §3；**Slice 2 两阶段已实施** 2026-08-12——
> [`GAP_ACAF_SLICE2A_IMPL_AUDIT`](audits/GAP_ACAF_SLICE2A_IMPL_AUDIT_2026-08-12.md) 与
> [`GAP_ACAF_SLICE2B_IMPL_AUDIT`](audits/GAP_ACAF_SLICE2B_IMPL_AUDIT_2026-08-12.md)，
> 全程影子模式，fail-closed 未切换；**Slice 2 fail-closed 机制已实施**
> 2026-08-13——D-12~D-16 落地（缺参/缺依赖硬拒绝、rejected GoalRevisionV1
> 不迁移、检索 lane activation 绑定），`ORZ_ACAF_FAIL_CLOSED=1` 显式翻转，
> 默认仍影子；生产启用待用户裁决，见
> [`GAP_ACAF_SLICE2_FAILCLOSED_IMPL_AUDIT`](audits/GAP_ACAF_SLICE2_FAILCLOSED_IMPL_AUDIT_2026-08-13.md)）
> 权威关系：本文件是 ACAF 的**详细设计权威**；`adr/ADR-0011` 是其**决策权威登记**；两者均由
> ADR-0010 派生且不改变 ADR-0010 任何既有条款（登记见 ADR-0010 §11.8）。
> 实施排期：**独立于 ADR-0010 §9 Phase C**；与 Phase C 只共享若干缺口闭合的前置依赖（见 §7）。
> 自包含：新会话按本文件实施，不必重开设计讨论；与 ADR-0010 冲突时以 ADR-0010 为上位权威。

---

## 1. 背景与动机

### 1.1 事件起点

用户观察到提示词注入攻击的普遍性后，对项目中"中立问询"机制提出安全疑问：若上游攻击者
篡改问询词和询问次数，能否造成严重攻击。核查结论（2026-08-09 前序分析）：

- 问询文本是编译期常量（`orz-loop/src/prompt.rs`），旧阈值是编译期常量（`inquiry.rs`），
  普通网页/检索结果**不能直接修改**它们；
- 但触发后问询作为新的 `User` 消息注入对话（`controller.rs`），**若该通道被篡改并高频注入**，
  相当于攻击者反复追加"较新的用户指令"，可能压过原任务、引导工具滥用或造成 token 型 DoS；
- 真正能篡改该通道的攻击者已控制更新链、本地进程或模型通信端点之一——属于严重安全事件，
  威胁面不在"一个恶意网页"。

### 1.2 问题定位

**检索内容无法即时审查防御**：网页、PDF、检索结果是不可信数据，注入可能发生在第一轮就诱导
危险动作，中立问询只能帮助模型重新定向，不能作为硬安全边界。因此安全边界必须移到**动作执行**
一侧：**默认检索内容可能有毒；接受模型可能已被误导；把损害限制为"模型只能在既定策略范围内
申请动作，未经独立授权的动作根本不发生"**。

### 1.3 现有基础设施盘点（2026-08-09 核实）

| 组件 | 现状 | 复用方式 |
|---|---|---|
| `permit/mod.rs`（orz-assurance） | 一次性 HMAC-SHA256 敏感动作 permit：RFC8785 规范化、issued→consumed、防重放 | 票据原语直接复用/扩展 |
| `keystore.rs`（orz-host） | Windows DPAPI 保护的安装级签名密钥，`PermitSigner` trait | K_install 载体 |
| §4.4 链校验（v0.2 事件体系） | assessment→disposition→close 顺序、CAS、幂等重放、单写者串行 | 票据消费/崩溃恢复骨架 |
| `sandbox/mod.rs`（orz-assurance） | **仅** Job Object 进程树 containment | 仅覆盖进程树，不覆盖文件/网络/凭据/执行隔离 |
| `windows_sandbox.py`（assurance） | AppContainer 探针，**development baseline**；raw TCP 残余（`network_connect_blocked=false`），elevated 防火墙才 compliant | 验证材料，**不是生产沙箱** |
| `acp_server.rs` ReadOnly 策略 | 写工具在 permission bridge 前机械短路 Deny | 模式门禁先例 |
| `snapshot.rs` | pre-mutation snapshot + restore | PromotionPermit 的 base_workspace_digest 依据 |
| run_tests（GAP-RUN-TESTS RT-001/002/003） | 跳过 execution permission / env 全继承 / 无 workspace delta | **执行器前置依赖，必须先闭合** |
| policy_revision（GAP-DENIAL-POLICY-REVISION） | 恒 0、无接线来源 | **ACAF 的必接项，票据绑定它** |
| journal hash chain（`chain.rs`） | 无密钥 SHA-256，只能发现损坏 | 票据 receipt 补"来源真实性" |

---

## 2. 威胁模型与边界

### 2.1 防御对象（ACAF 覆盖）

| 攻击 | 是否防御 | 机制 |
|---|---|---|
| 恶意网页/PDF/检索结果注入模型，诱导越权动作 | ✅ 核心 | 动作必须有票；执行器验票后执行 |
| 伪造中立问询/控制标记 | ✅ | 控制事件持 ControlTicket，模板由签发器持有 |
| 篡改问询次数、重放旧问询 | ✅ | 7 轮计数机械化 + 单调 sequence + 一次性 nonce |
| 替换动作参数后复用旧许可 | ✅ | 验票对象 = 解析后真实对象（规范化 argv/绝对路径摘要） |
| 插件/子代理/日志文件冒充控制面 | ✅ | 票据签名 + session/agent/chain 绑定 |
| 崩溃恢复后重复执行一次性写 | ✅ | 原子消费 + receipt 链，恢复不重放 |
| 目标版本被替换后旧票继续生效 | ✅ | 票据绑 goal_version+goal_digest；目标修订 → 旧票立即失效 |

### 2.2 明确不防御（边界声明）

| 攻击 | 不防御原因 |
|---|---|
| 签发器本身被攻破 | 签发器是信任根；只能通过签名 manifest、回滚保护、最小接口降低概率 |
| 同用户恶意进程 / 调试注入 / 内存读取 | 用户态防线够不着；这是 OS 级问题（同用户进程可终止或注入任何同用户进程） |
| 管理员/内核级控制 | 任意修改，等同整机失守 |
| 恶意模型服务或中间代理 | 端点信任问题；"我准备发送什么"与"线上实际收到什么"的差距需出站请求证明（后续强化，见 §10） |
| 被允许的命令"语义上是否明智" | 票据证明来源真实 + 授权匹配，不证明语义正确；仍需最小权限与人工确认 |

### 2.3 威胁模型的一句话定义

> **ACAF 防御的是：受提示词注入影响的 Agent 进程（模型、子代理、插件）试图让任何跨信任边界
> 的控制事件或外部效果动作发生，而该动作没有获得独立签发器的机械授权。**
>
> 它不承诺防御"攻击者已经控制本机进程"的情形——那种情形下任何用户态机制都可以被绕过。

### 2.4 关键推论

1. 票据证明"这条动作确实由受信任调度器在正确上下文签发"，**不证明动作语义正确**；
2. **通过验签的动作不自动获得任何工具权限**——文件写入、命令执行、联网、凭据读取仍须经过
   各自动作绑定的 permit/人工批准（双层：permission 管"策略是否允许"，票据管"本次是否被授权"）；
3. 验票是机械代码义务，模型输出（包括注入后的输出）不参与验票判定。

---

## 3. 设计目标与不变量

1. **模型不可见、零负担**：票据与验票不进入模型上下文，无 token 与理解成本；
2. **动作必须有票**：所有跨信任边界控制事件 + 外部效果动作持票；系统记账写入不持票（见 §4.7）；
3. **验票对象是解析后真实对象**：命令绑定规范化 argv/cwd/env 摘要，文件绑定解析后绝对路径，
   拒绝 symlink/junction/reparse point/路径穿越——否则存在 TOCTOU 与等价编码绕过；
4. **原子消费 + 崩溃不重放**：票据消费与 receipt 落盘是一个 commit，恢复路径不重放已消费票据；
5. **签发器独立**：主密钥不进入 Agent 进程；签发器接口只接受枚举化动作，不接受任意文本；
6. **fail-closed**：验票失败一律拒绝并写安全事件；无票通道不存在；
7. **审计完整**：每张票的签发/消费/拒绝均有 receipt，进入 v0.2 事件体系与签名链；
8. **影子模式先行**：先记录"应签未签/应拒放行"差异并确认无误，再切换 fail-closed；
9. **模式显式**：任何模式都手动开启、手动选择；模型不能切换模式、不能退出模式、不能扩大范围。

---

## 4. 核心机制

### 4.1 票据类型总览

| 票据 | 用途 | 消费方 | 典型生命周期 |
|---|---|---|---|
| `ControlTicket` | 控制事件：Orientation/Disposition/Close/GoalRevision 等 | controller 注入点 | 签发→验证→消费→receipt |
| `SandboxLease` | 无运行自动模式的长租约：沙盒根内写入自动批准 | 沙盒写入面 | 模式进入时签发，模式退出/过期时作废 |
| `PromotionPermit` | 暂存差异回写宿主 | promotion 执行器 | 用户确认后一次性消费 |
| `ModeChangeTicket` | 模式切换（进入/退出/降级） | 模式状态机 | 用户显式触发 |
| `GoalRevisionTicket` | 目标修订（用户追加需求） | controller | 修订时签发，旧目标票立即失效 |
| `ExecutionPermit` | 沙盒内/沙盒外受控进程执行 | 执行 broker | 每次进程创建一次性票 |

> 票据是统一机制（HMAC + 绑定字段），类型差异只在 `ticket_kind` 与绑定字段集合。
> 检索子代理的检索文档写入（ADR-0010 §3.2 三域）属于**租约内写入**：进入子代理任务时签发
> 窄 `SandboxLease`（writable_root=检索三域解析路径），域内写入自动批准并产生 receipt，
> 不打扰用户、不创建无票通道。

### 4.2 ControlTicket 结构

```text
ControlTicket:
  schema_version
  ticket_kind              # 枚举：orientation_v1 / disposition_v1 / close_v1 /
                           #       goal_revision_v1 / file_write_v1 / command_exec_v1 /
                           #       network_v1 / credential_read_v1 / mode_change_v1 / ...
  ticket_id
  signer_revision + signer_measurement   # 签发器版本与二进制测量（manifest 链）
  session_id + agent_id + activation_id  # 子代理场景 activation 绑定
  goal_version + goal_digest             # 目标版本与摘要
  policy_revision                        # 机械策略版本（GAP-DENIAL-POLICY-REVISION 接线）
  action_kind + canonical_arguments_sha256
  resolved_target_sha256                 # 解析后真实对象的摘要
  capability_scope                       # 允许的能力范围（枚举）
  template_sha256                        # 控制事件文案模板摘要（仅控制类票据）
  sequence + nonce                       # 单调序号 + 一次性随机数
  issued_at + expires_at                 # 短 TTL
  previous_receipt_sha256                # 链式绑定前序 receipt
  user_confirmation_sha256               # 需要用户确认时绑定确认摘要
  hmac = HMAC-SHA256(K_session, canonical_payload)
```

> v1 实施登记（2026-08-16）：`previous_receipt_sha256` 字段在 Slice 1/2
> 实施中省略——链式语义由 journal 哈希链（previous_event_sha256）+ verifier
> 的 issued→consumed/rejected 配对规则承担；票级 receipt 链保留为 v2 升级
> 路径（ADR-0011 状态行登记）。

验票方检查（任一失败即拒绝 + 写安全事件 + 暂停高风险工具）：

1. 签名有效（用当前 `K_session`）；
2. `template_sha256` 等于内置模板版本（控制类）——**签名器不接受任意文本**；
3. `sequence` 单调递增，`nonce` 未被消费（一次性）；
4. session/agent/activation/goal/policy 绑定全部匹配当前上下文；
5. `resolved_target_sha256` 与"解析后真实对象"重算一致（TOCTOU）；
6. 未过期；`previous_receipt_sha256` 链一致；
7. 失败则不发送/不执行，写 `control_ticket_rejected` 事件。

### 4.3 密钥体系

```text
K_install = DPAPI 保护的主密钥（现有 keystore，安装级）
K_session = HKDF-SHA256(K_install, session_id || goal_digest || policy_digest || signer_revision)
```

> Slice 1/2 实施以 8 字节 LE `policy_revision` 占位 `policy_digest`（用户
> 2026-08-12 裁决，2026-08-16 全面审查登记）；Slice 3 ModeChangeTicket 提供
> 首个生产递增来源后，切换真 `policy_digest` 时全量重派生 K_session。

- `K_install` 只存在于**签发器进程**；Agent 进程不可读（签发器独立进程，见 §4.4）；
- `K_session` 由签发器派生，通过窄 IPC 下发给 host 的**验票模块**（机械代码）；验票用 `K_session`，
  伪造需要 `K_install`——Agent 进程拿不到；
- `goal_digest` 或 `policy_digest` 变化 → 新 `K_session` 派生，旧票立即失效（验票用当前 session key）；
- 升级路径（后续）：验票移到独立执行器进程，`K_session` 也不进入 Agent 进程（见 §10）。

### 4.4 签发器（独立进程、窄 IPC）——用户拍板级别

**进程模型**：独立进程，由受信启动器（host 启动链）在运行前验证**签名 manifest + 二进制哈希**
后拉起；哈希由独立发布密钥签名的 manifest 承载，不由签发器自证。

**接口（窄 IPC）**：命名管道 + ACL 限制到启动者身份；单请求单响应；请求超时与重试受限。
接口只接受**枚举化动作**：

```text
SignOrientationV1()                       # 模板由签发器持有，无参数
SignDispositionV1(activation, decision, delta_digest)
SignCloseV1(activation, disposition_id)
SignGoalRevisionV1(old_digest, new_digest, user_confirmation)
SignFileWriteV1(path_resolved, content_sha256, lease_id?)
SignCommandExecV1(argv_canonical, cwd, env_sha256)
SignNetworkV1(...) / SignCredentialReadV1(target, scope)
SignModeChangeV1(old_mode, new_mode, user_confirmation)
```

**更新链**：签名 manifest + 单调 `signer_revision` + 版本白名单 + 回滚保护。
**否决**："永远固定一个哈希"——那会导致签发器无法安全升级。审查固定的是**签名公钥与
manifest 链**，不是二进制哈希。

**密钥**：v1 用现有 DPAPI keystore（保护静态）；后续迁移 TPM 不可导出密钥（DPAPI 只保护
静态，同一用户上下文的恶意进程读内存仍是风险——但该威胁已在 §2.2 边界外）。

### 4.5 执行器 / 验证器

1. **验票层**：§4.2 七项检查的机械实现；只接受票据覆盖的枚举化动作；
2. **目标解析**：动作参数先解析（绝对路径展开、规范化 argv、环境变量摘要在执行前取定），
   再重算 `resolved_target_sha256` 比对；拒绝 symlink/junction/reparse point/路径穿越；
3. **原子消费**：验票通过 → 执行动作 → receipt（`control_ticket_consumed`）与动作结果
   一个 commit 落盘（复用 §4.4 链校验的单写者串行 + CAS 模式）；崩溃恢复不重放；
4. **双层门禁**：permission bridge 管"策略是否允许"（现状语义保留），验票管"本次是否被授权"；
   执行器同时检查两者，任一失败即拒绝；
5. **run_tests 是受控执行**：必须走 `ExecutionPermit`，并满足 ADR-0010 §3.8 全部语义
   （固定命令、sandbox/Job Object、timeout、输出上限、workspace delta 记录）；
   在 RT-001~003 闭合前，ACAF 执行器**不得放行** run_tests（见 §7）。

### 4.6 审计

- 每张票：签发（`control_ticket_issued`）、消费（`control_ticket_consumed`）、拒绝
  （`control_ticket_rejected`）三类事件进入 v0.2 事件体系；
- 事件类型扩展走 ADR-0010 §5.3 迁移纪律：**先扩 Schema/fixture（含 negative），再改 producer**；
- receipt 链绑定 `previous_receipt_sha256`，与 journal hash chain 并存：链保完整性，票保来源真实性；
- "写入形式与受签发情况"：无运行自动模式下每次暂存写入产生 receipt（路径、digest、ticket_id、
  sequence、before/after），即 RT-003 想要的 workspace delta 的签名版。

### 4.7 系统记账写入不持票（避免自锁）

以下写入是 host 系统记账，**不要求票据**：journal append、blackboard（controller 单写者）、
snapshot、session 元数据、TUI 状态、`.gsa` 结构目录。它们不是模型动作，不构成跨信任边界。
**模型动作写入**（文件写、命令、网络、凭据读取、检索文档写、目标修订）才持票。

---

## 5. 三模式设计（用户定稿：不做"完全授权"）

### 5.1 模式总览

| 模式 | 默认 | 进入方式 | 模型行为 | 进程创建 | 网络/凭据/宿主 |
|---|---|---|---|---|---|
| Normal（正常审批） | ✅ | — | 现状 + 动作票据逐步接线 | 经 permission+permit | 现状语义 |
| Auto-Staged No-Run（无运行自动） | 关 | 用户显式开启 | 长期自动迭代，只写暂存根 | **禁止** | 无网络/无凭据/无宿主 |
| Ephemeral Sandboxed Run（临时沙盒运行） | 关 | 用户显式开启 | 迭代 + 可运行进程 + 可测试 | **仅限一次性 VM/容器内** | 默认无网络/无凭据/无宿主挂载 |

- 三模式**手动开启、手动选择**；责任落在用户（与 1.md 讨论一致）；
- 模型不能自行切换模式、不能退出模式、不能扩大沙盒范围、不能改变目标；
- 模式切换必须经 `ModeChangeTicket`（用户确认）；切换时**作废全部未消费票据与租约**，
  并递增 `policy_revision`（顺带落实 GAP-DENIAL-POLICY-REVISION 的接线来源）。

### 5.2 Normal（默认）

> **2026-09-27 更正（083 审查裁决②，0bv 文档落字批）**：本节原「用户逐项审批语义保留，
> 审批确认摘要绑定进票据（`user_confirmation_sha256`）」与实现脱节——人工审批腿**从未实现**
> （`orz-host/src/approval.rs` 为空壳存根），且权限桥自 2026-09-26 起默认 yolo。原文保留于
> git 历史不删节，现行为口径如下。

**现行为口径**：Normal 模式当前**无人工审批**——权限桥默认 yolo 自动放行（写／命令／网络
默认批准；`ORZ_ALLOW_WRITE` 等三键切入 Benchmark 轴），交互面**零人工检查点**；动作票据接线
（§4）照旧，票据绑定的是**策略与目标摘要**，不存在 `user_confirmation_sha256` 人审摘要。
安全叙事相应降级：ACAF 防的是**未授权工具调用／票据重放／参数事后篡改**（「策略＋票＋审计」），
**不防**「模型做出格式正确但错误的决定」；叠加写入管控（0bw）后的安全边界＝机械层门禁＋
journal 审计。**审批面为未来可选扩展、当前未实现**（权限桥 `PermitSource` 枚举已为恢复审批
保留观测位）。

### 5.3 Auto-Staged No-Run

**允许**：在沙盒根内创建/修改/删除文件；写入自动经 `SandboxLease` 机械批准，产生 receipt；
可反复修改，保持长期迭代连续性。

**禁止**：写宿主工作区、`.git`、用户目录、凭据目录、系统目录；启动任何进程；运行脚本、
测试、编译器和 Git hook；网络；宿主环境变量；凭据；剪贴板；自行退出模式或扩大范围。

**"剥夺执行权限"不是机制，机制是"沙盒无进程创建能力"**（防间接加载）：

1. 沙盒根根本没有进程创建能力——不是"文件没有执行位"（恶意 Python/PS/JS 可被解释器当数据
   读取执行；DLL/`.pth`/Git hook/npm lifecycle 可在宿主以后运行工具时被间接加载）；
2. 工具注册表不声明 shell/测试/编译/package install 等执行类工具；
3. 执行 broker 拒绝所有没有独立 `ExecutionPermit` 的 `CreateProcess` 请求；
4. 宿主工具链（IDE 索引、git、node、文件监视器）不得自动打开/加载/扫描暂存产物；
5. 拒绝 junction/reparse point/symlink/hardlink/路径穿越逃逸出暂存根；
6. 产物回写宿主后仍保留隔离标记；可执行权限必须由另一张票单独解除（v1 不做，见 §10）。

**SandboxLease 结构**：

```text
SandboxLease:
  session_id + sandbox_id
  goal_version + goal_digest
  source_snapshot_digest
  writable_root（解析后绝对路径）
  allowed_operations = [create, modify, delete]
  process_execution = deny
  network = deny
  credentials = deny
  host_write = deny
  max_files + max_bytes
  expires_at + policy_revision + signer_measurement
  HMAC(K_session)
```

**回写宿主 = 单独安全动作（PromotionPermit）**：

```text
PromotionPermit:
  sandbox_id + base_workspace_digest
  final_diff_digest + affected_paths
  executable_content_present
  policy_revision + user_confirmation_digest + one_time_nonce
  HMAC(K_session)
```

执行前**重新验证宿主工作区仍等于 `base_workspace_digest`**：用户后台改过文件 → 拒绝直接应用，
必须重新审查或机械 rebase；票据原子消费，崩溃后不自动重放。

### 5.4 Ephemeral Sandboxed Run

**语义**：把当前暂存快照复制到一次性 VM/容器，在其中运行进程与测试；只向主 Agent 返回
截断脱敏后的日志、退出码与结果摘要；每次运行结束销毁，再次测试从新快照重新创建。

**硬性规则**：

1. 无宿主写挂载（宿主源码只读映射，或完全快照复制）；输出保存在沙盒虚拟磁盘，经受信 broker
   提取差异；
2. 无凭据、默认无网络；网络若用户显式开启，只允许任务必需且受限的端点（v1 保持关闭）；
3. 测试执行面复用 ADR-0010 §3.8 run_tests 语义：固定命令、脱敏截断输出、完整产物以受控
   artifact identity 落盘、每次调用计一个 tool-call round；
4. 测试生成的产物、二进制、缓存不自动带回暂存区；
5. 回写宿主仍须 `PromotionPermit`（与 §5.3 相同）。

**隔离级别**：本模式内"进程创建能力"开关开启，但**强制 VM 级隔离**：

- Windows Sandbox 本身是 Hyper-V 轻量虚拟机 → 满足"进程可运行 + VM 级隔离 + 一次性销毁"
  三重语义，是 v1 首选路径（宿主源码只读映射，可写映射禁止——微软明确警告可写映射目录
  可能让沙盒内容影响宿主）；
- 完整 Hyper-V VM 作为更强制冷档（更大内存/启动成本，仅高风险任务启用）；
- Docker（评测侧已验证链路）可作为已有 harness 侧的等价路径，不作为产品侧默认；
- 沙箱与宿主共享同一内核的路径（AppContainer/Job Object 组合）在 raw TCP 残余与子进程
  继承未闭环前**不得**作为本模式的隔离依据（GAK-SBX-001：非生产）。

**本机条件核查**（2026-08-09）：Windows 11 Pro（Windows Sandbox/Hyper-V 要求 Pro/Enterprise ✅）、
虚拟化已开启 ✅、内存需满足 Windows Sandbox 每实例 ~1-2GB 空闲（实施前实测）、Docker Desktop
已装（评测侧）。**硬件满足是前提，不等于产品有沙箱**——沙箱 backend 本身需要设计落地
+ 探针验证（§7 Slice 4）。

### 5.5 模式切换规则

```text
ModeChangeTicket:
  session_id + old_mode + new_mode
  user_confirmation_digest（必须人工确认）
  invalidation = all_unconsumed_tickets_and_leases
  policy_revision + 1（切换后递增）
  HMAC(K_session)
```

切换动作：作废旧租约 → 递增 policy_revision → 派生新 K_session → 生效新模式。
进入 No-Run/Ephemeral 必须先有 `source_snapshot_digest`（快照已就位）；退出时若存在未回写
差异，明确提示"存在待审查差异"，不自动回写。

### 5.6 与检索子代理的互动

子代理与主 Agent 同构（ADR-0010 §3.1），工具与权限路径相同：

- 子代理控制事件（activation/assessment/disposition/close）持 `ControlTicket`；
- 子代理写域（检索三域）持窄 `SandboxLease`（§4.1），域内自动批准 + receipt；
- 三模式下子代理同样受模式约束：No-Run 模式下子代理也不能启动进程、不能写宿主工作区。

---

## 6. 与现有架构的关系

### 6.1 派生条款（ADR-0010）

| ADR-0010 条款 | 派生内容 |
|---|---|
| §2.4.2 permission 是执行事实 hard gate | 双层门禁：permission 管策略，票据管授权 |
| §2.4.3 结构化接口优先于 prompt 纪律 | 验票是机械代码义务，模型输出不参与判定 |
| §3.2 子代理写域白名单 + 路径解析验证 | 检索三域租约 + 解析后真实对象验票 |
| §3.8 受控 run_tests | ExecutionPermit + RT-001~003 闭合前置 |
| §4.1/§4.2 六机制分层、7 轮机械计数 | Orientation 模板由签发器持有，注入持 ControlTicket |
| §5.3 先 Schema 后 producer | 新事件类型扩展纪律 |
| §5.4 journal/snapshot | receipt 链 + base_workspace_digest 复核 |
| §11.3 子代理写域 | 检索三域租约 |

### 6.2 消化的现役缺口

| 缺口 | 关系 |
|---|---|
| GAP-DENIAL-POLICY-REVISION（policy_revision 恒 0） | **必接项**：票据/租约绑定 policy_revision，模式切换与策略变更提供递增来源 |
| GAP-RUN-TESTS RT-001（跳过 execution permission） | 前置：执行器放行 run_tests 前必须闭合 |
| GAP-RUN-TESTS RT-002（env 全继承无脱敏） | 前置：票据绑定 env 摘要需要可取的 env 面 |
| GAP-RUN-TESTS RT-003（无 workspace delta） | 被暂存写入 receipt 机制覆盖（§4.6） |
| host envelope 真实性缺口（orz-host/lib.rs） | 票据 receipt 提供签名来源真实性 |

### 6.3 新增组件（相对于当前代码）

1. 签发器进程（独立二进制，窄 IPC 服务）；
2. host 内验票模块（K_session 持有 + 七项检查）；
3. 执行器验票接线（file/command/network/credential 路径插入）；
4. 模式状态机（三模式 + ModeChangeTicket + 租约管理）；
5. 暂存根/差异提取与 PromotionPermit 执行器；
6. 沙箱 backend（Slice 4 选型与实现）；
7. 新事件类型 Schema/fixture/verifier（v0.2 扩展路径）。

---

## 7. 实施切片（独立于 Phase C）

> 排期独立；依赖关系显式化。每个切片有独立验收入口，全部先影子模式后 fail-closed。

### Slice 1：签发器 v1 + 控制事件票据（无前置依赖）

- 签发器进程（签名 manifest 验证启动、枚举化接口、Orientation/Disposition/Close/GoalRevision）；
- K_session 派生与下发；host 验票模块 + 七项检查；
- Orientation（7 轮模板签发）、parent disposition、close、goal revision 四类控制事件持票；
- 新事件类型 Schema/fixture（issued/consumed/rejected）+ verifier（先 Schema 后 producer）；
- 验收入口：四类控制事件在真实 controller 路径带票；拒绝路径写安全事件；verifier 全绿。

### Slice 2：动作票据 + 执行器验票 fail-closed

- **前置：RT-001/002/003 闭合（Phase C 或独立完成）+ policy_revision 接线**（✅ 已满足，
  2026-08-12——goal/policy 接线 + 目标解析单源化，见
  `audits/GAP_ACAF_GOAL_POLICY_WIRING_IMPL_AUDIT_2026-08-12.md`）；
- file_write / command_exec / network / credential_read 四类动作票据接入执行器；
- 目标解析（规范化 argv/绝对路径）与 TOCTOU 验票；
- 影子模式：记录"应签未签/应拒放行"差异 → 确认无误后 fail-closed；
- 验收入口：模型注入探针（伪造参数/重放/越权）全部被拒；真实动作零误阻断（影子台账）。
- 用户裁决（2026-08-12）：fail-closed 翻转前置决策已登记，见 §11 D-12~D-16。
- **实施完成（2026-08-13）**：D-12 显式排除维持；D-13 activation 绑定、
  D-14 缺参硬拒绝（missing_target_argument）、D-15 缺依赖/未配置硬拒绝
  （missing_snapshot_store / missing_goal_context / 启动 fail-fast）、
  D-16 rejected GoalRevisionV1 不迁移均已落地；fail-closed 经
  `ORZ_ACAF_FAIL_CLOSED=1` 显式启用（默认影子）。实施审计：
  `audits/GAP_ACAF_SLICE2_FAILCLOSED_IMPL_AUDIT_2026-08-13.md`。

### Slice 3：Auto-Staged No-Run

- 暂存根布局 + 快照（source_snapshot_digest）+ 写入 receipt；
- SandboxLease 签发/消费；执行 broker 拒绝全部 CreateProcess；
- PromotionPermit（base_workspace_digest 复核、原子消费、崩溃不重放）；
- 模式状态机 + ModeChangeTicket + 租约作废；
- 验收入口：注入探针在暂存根内写文件成功但无进程/无网络/无凭据/无宿主写入；回写必须
  PromotionPermit 且用户后台修改被拒绝。

### Slice 4：Ephemeral Sandboxed Run

- 沙箱 backend 实施（**Windows Sandbox**（D-11 定稿）探针验证；完整 Hyper-V 为强档）；
- 暂存快照 → 一次性 VM 复制 → 受限 run_tests（§3.8 语义）→ 脱敏日志回传 → 销毁；
- 默认无网络/无凭据/无宿主挂载；产物不回带；
- 验收入口：沙箱内可运行进程与测试；宿主零写入；逃逸探针（junction/挂载/凭据）全拒。

> 依赖图：Slice 1 无依赖；Slice 2 依赖 RT 闭合 + policy_revision 接线；Slice 3 依赖 Slice 2 的
> 执行器接线；Slice 4 依赖 Slice 3 的暂存/快照。全部切片独立于 ADR-0010 §9 Phase C 的
> GAP-INQUIRY-SPLIT / GAP-SUBAGENT-RUNTIME 排期，但共享"先 Schema 后 producer"纪律。

---

## 8. 安全属性分析

### 能显著防止

- 检索注入诱导模型直接执行越权动作（损害降为"只能在既定策略范围内申请"）；
- 伪造中立问询或控制标记；篡改问询次数；重放旧问询；
- 替换动作参数后继续使用旧许可；
- 插件、子代理、日志文件冒充控制面；
- 崩溃恢复后重复执行一次性写操作；
- 无运行自动模式下：模型写出的恶意/错误内容**不能**在宿主运行、不能读秘密、不能联网外传、
  不能未经提升写回真实项目。

### 不能防止

- 签发器/受信启动器本身被攻破；同用户恶意进程；管理员/内核；恶意模型服务；
- 被允许命令在语义上不明智 → 仍需最小权限与人工确认；
- 高价值内容经模型"记忆"外泄（模型已见内容无法收回）——这是所有 agent 系统的固有边界。

---

## 9. 验收与回归

1. 每个切片：探针矩阵（伪造/重放/越权/逃逸/崩溃重放）全拒 + 真实动作零误阻断（影子台账）；
2. 全链路：三代理审查（设计合理性/实现合理性/符合性，项目惯例）无 P1/D1 级背离；
3. Schema 扩展走 §5.3 纪律，negative fixture 覆盖 retired 语义；
4. 性能预算：验票在工具轮延迟占比可忽略（HMAC 微秒级；目标解析为主要成本，接受影子模式
   测量数据）；
5. 故障语义：签发器不可达 → fail-closed（不降级为无票放行）；恢复链保留完整 hash chain；
6. 文档同步：每个切片实施后更新本文件实施记录与索引状态。

---

## 10. 开放问题与后续强化（不阻塞 v1）

1. **TPM/不可导出密钥**：签发器密钥迁 TPM（v2）；
2. **独立执行器进程**：验票移出 Agent 进程，K_session 也不进 Agent 进程（威胁升级时的路径）；
3. **WFP 内核 callout**：AppContainer 网络闭环（raw TCP 残余根治，当前靠 elevated 防火墙补偿）；
4. **出站请求摘要**："我准备发送什么"与"线上实际收到什么"的差距证明（防恶意模型服务/代理；
   属端点信任面，用户态不可完整解决）；
5. **可执行权限解除票**：暂存产物回写宿主后解除隔离标记的独立票据（v1 不做自动执行解除）；
6. **沙箱 backend 实证**：Windows Sandbox 每实例内存/启动实测；完整 Hyper-V VM 档的成本裁决。

---

## 11. 决策记录（2026-08-09 定稿）

| # | 决策 | 裁决 |
|---|---|---|
| D-1 | 新安全设计独立实施，不并入 Phase C | 采纳；依赖关系见 §7 |
| D-2 | 签发器级别 | **独立进程窄 IPC**（签名 manifest 启动、枚举化接口、DPAPI 静态密钥，TPM 后置） |
| D-3 | "完全授权"模式 | **不做**；最高层级 = 临时沙盒运行 |
| D-4 | 三模式 | Normal（默认）/ Auto-Staged No-Run / Ephemeral Sandboxed Run，均手动开启 |
| D-5 | 沙盒进程创建能力 | 分开算：No-Run 关闭；Ephemeral 开启但强制 VM 级隔离，默认无网络/无凭据/无宿主挂载 |
| D-6 | 威胁模型边界 | 防受注入影响的 Agent 滥用工具；不防同用户恶意进程/签发器被攻破/恶意模型服务 |
| D-7 | 密钥体系 | K_install（DPAPI）+ K_session=HKDF；K_install 不进 Agent 进程 |
| D-8 | 系统记账写入 | 不持票（journal/blackboard/snapshot/.gsa 结构）；模型动作写入持票 |
| D-9 | 影子模式先行 | 每个切片先记录差异台账，再 fail-closed |
| D-10 | 票据对模型不可见 | 不进入上下文，无 token 负担；receipt 全量入审计链 |
| D-11 | 沙箱 backend 选型 | **采用 Windows Sandbox**（Hyper-V 轻量 VM，满足进程可运行 + VM 级隔离 + 一次性销毁）；完整 Hyper-V VM 为强档，仅高风险任务启用；Docker 仅评测侧等价路径，不作为产品侧默认（2026-08-09 用户拍板） |
| D-12 | web_search 票据形态（用户裁决 2026-08-12） | 保持无映射：DeepSeek 原生服务端搜索、无模型可见 URL 目标，不发 network 票；显式排除并登记（不产生影子台账事件）。来源质量筛选属检索结果层，另行设计 |
| D-13 | 检索子代理动作票 activation 绑定（用户裁决 2026-08-12） | 必须绑定：子代理 lane 内 web_fetch 等动作票持真实 activation_id；接线为 fail-closed 翻转前置条件（当前仍 null，Slice 2B 核查⑧） |
| D-14 | 缺参静默跳过（用户裁决 2026-08-12） | fail-closed 时转硬拒绝：file_path/url/command 缺失或空 → `control_ticket_rejected`（missing_target_argument，ticket_id=null）+ 工具不执行 |
| D-15 | 未配置/缺依赖静默 skip（用户裁决 2026-08-12） | fail-closed 时禁止静默：acaf 未配置=启动期 fail-fast 或显式降级开关；snapshot_store 缺失 → file_write 硬拒绝（missing_snapshot_store）；goal_digest 缺失 → 动作/控制票硬拒绝（missing_goal_context） |
| D-16 | rejected GoalRevisionV1 状态迁移（用户裁决 2026-08-12） | fail-closed 时 consumed-only：票 rejected → update_goal 与 AcceptedContinue 状态迁移不执行、goal 绑定不动、写安全事件并向父 Agent 表面未授权 |

> **实施记录（2026-08-13）**：D-12 保持显式排除；D-13~D-16 全部实施——
> 动作票 activation 可选（Orientation 唯一禁止）、`sign_network_v1` 接受
> 可选 activation、controller 侧 `TicketGate` + fail-closed 开关
> （`ORZ_ACAF_FAIL_CLOSED`）、D-14/D-15 预签发拒绝码入 Schema/fixture、
> D-16 迁移门控。默认仍影子模式，生产翻转待用户裁决；详见
> [`GAP_ACAF_SLICE2_FAILCLOSED_IMPL_AUDIT`](audits/GAP_ACAF_SLICE2_FAILCLOSED_IMPL_AUDIT_2026-08-13.md)。
