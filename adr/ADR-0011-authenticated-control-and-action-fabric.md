# ADR-0011：受信控制与动作授权面（Authenticated Control and Action Fabric，ACAF）

- 状态：**accepted**（2026-08-09；设计定稿。2026-08-12 Slice 1 已实施——签发器 v1 +
  控制事件票据，见 [`GAP_ACAF_SLICE1_IMPL_AUDIT`](../docs/audits/GAP_ACAF_SLICE1_IMPL_AUDIT_2026-08-12.md)；
  2026-08-12 Slice 2 已实施（两阶段）——四类动作票就位：file_write 全链贯通
  + credential_read 机制先建（第一阶段，见
  [`GAP_ACAF_SLICE2A_IMPL_AUDIT`](../docs/audits/GAP_ACAF_SLICE2A_IMPL_AUDIT_2026-08-12.md)），
  command_exec（run_tests + run_terminal_cmd）与 network（web_fetch +
  browser_read URL 目标解析）同型扩展（第二阶段，见
  [`GAP_ACAF_SLICE2B_IMPL_AUDIT`](../docs/audits/GAP_ACAF_SLICE2B_IMPL_AUDIT_2026-08-12.md)；
  **全程影子模式**：验票失败照常执行并记 `control_ticket_rejected`；
  web_search 无 URL 目标暂不映射，登记 fail-closed 翻转前核查）；
  2026-08-12 goal/policy 接线已实施——GoalRevisionV1 消费后 goal_version+1 重派生
  K_session（决策 5 "goal 变→旧票死"首次真实触发）；`bump_policy_revision` 机制
  就位（决策 9，Slice 3 ModeChangeTicket 为首个生产递增来源）；目标解析原语单源
  orz-paths（设计文档 §4.5 镜像），见
  [`GAP_ACAF_GOAL_POLICY_WIRING_IMPL_AUDIT`](../docs/audits/GAP_ACAF_GOAL_POLICY_WIRING_IMPL_AUDIT_2026-08-12.md)；
  **Slice 2 fail-closed 机制已实施**（2026-08-13）——D-12~D-16 落地：
  web_search 显式排除（D-12）、检索 lane 动作票 activation 绑定（D-13）、
  缺参/缺依赖硬拒绝（D-14/D-15，新拒绝码
  missing_target_argument / missing_snapshot_store / missing_goal_context
  入 Schema）、rejected GoalRevisionV1 不迁移（D-16）；`ORZ_ACAF_FAIL_CLOSED=1`
  显式翻转（默认影子）；**2026-08-16 生产启用翻转已实施**（用户 2026-08-15
  裁决放行）——fail-closed 改为**默认**（未设置即强制；`0|false|no|off` 显式
  影子；非法值 exit 2 fail-closed），CLI run / ACP stdio / TUI 三个生产入口
  全部接线（ACP/TUI 此前未挂签名器客户端，随翻转补齐），核查清单 ⑦⑨⑩⑪
  收口（⑦ web_search 显式排除——走 provider 原生搜索、无第三方 URL 目标，
  ADR-0010 §3.7 条 10；⑨ host 稳定面不补绑定；⑩ URL gate 与票据摘要规范化
  等价；⑪ 重定向逐跳 URL gate 覆盖，票据一次 consumed 语义登记），新增
  `orz-acaf-provision` 供应工具 + `scripts/orz_acaf_run.ps1` 启动链，见
  [`GAP_ACAF_SLICE2_FAILCLOSED_IMPL_AUDIT`](../docs/audits/GAP_ACAF_SLICE2_FAILCLOSED_IMPL_AUDIT_2026-08-13.md)
  与
  [`GAP_ACAF_FAILCLOSED_PRODUCTION_ENABLEMENT_IMPL_AUDIT`](../docs/audits/GAP_ACAF_FAILCLOSED_PRODUCTION_ENABLEMENT_IMPL_AUDIT_2026-08-16.md)；
  Slice 3/4 待实施）；**2026-08-16 全面审查处理登记**——P2-I1 grill 路径补
  D-15 启动期拒绝（与 `run_turn_with_guards` 同语义）；P2-I2 AcafClient 改
  per-session 缓存（多会话交错不再重置一次性账本）；respawn 活性修复（原
  e2e 从未真正杀签名器；respawn 后序列重启、账本随 epoch 重置，更正 Slice 1
  审计"序列单调延续"表述）；P3-I4 签发器请求行 1 MiB 上限。登记四项权威
  文本边界：① `policy_digest` 未实现前以 8 字节 LE `policy_revision` 占位
  （Slice 3 切换真摘要，用户 2026-08-12 裁决）；② `previous_receipt_sha256`
  由 journal 哈希链 + verifier 配对规则承担（v2 升级路径）；③ web_search
  例外（决策 1 网络持票的显式排除，D-12 / ADR-0010 §3.7 条 10）；④ 决策 11
  影子台账前置由用户 2026-08-15 裁决显式豁免，真实运行台账观察保留为
  运营项。详见 [`GAP_ACAF_FAILCLOSED_PRODUCTION_ENABLEMENT_IMPL_AUDIT`](../docs/audits/GAP_ACAF_FAILCLOSED_PRODUCTION_ENABLEMENT_IMPL_AUDIT_2026-08-16.md)
  §8。）
- 决策范围：跨信任边界控制事件与外部效果动作的票据授权面；独立签发器；三运行模式；
  威胁模型边界；实施切片的独立性
- 详细设计权威：[`AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md`](../docs/AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md)
- 派生/修订关系：
  - **派生自** ADR-0010 §2.4（permission hard gate）、§3.2/§3.8（子代理写域与受控 `run_tests`）、
    §4.1/§4.2（六机制分层与 7 轮机械计数）、§5.3/§5.4（Schema 迁移纪律与 journal/snapshot）、
    §11.3（子代理写域解析验证）；
  - **不改变** ADR-0010 任何既有条款；ADR-0010 的机制分层、机械判定与权威层级继续有效；
  - **衔接** ADR-0006（凭据目标与注入）、ADR-0008（工具轮预算）、ADR-0009（A/B/C 写点分域）；
  - 为 `GAP-DENIAL-POLICY-REVISION`（policy_revision 接线）提供必接消费面，为
    `GAP-RUN-TESTS` RT-001/002/003 提供执行器前置依赖关系。

## 1. 背景

提示词注入无法通过即时审查检索内容根除。中立问询等控制通道一旦被篡改并高频注入，等价于
攻击者反复追加"较新的用户指令"。安全边界因此必须移到动作执行侧：**默认检索内容可能有毒；
模型可能已被误导；未经独立授权的外部效果动作根本不发生**。该方向把注入的损害降为
"模型只能在既定策略范围内申请动作"，与 ADR-0010 "模型输出不等于状态事实、permission 是
执行事实 hard gate"的既有哲学同构，是本 ADR 的决策基础。

## 2. 核心决策

1. **票据面边界**：所有跨信任边界控制事件（Orientation、parent disposition、close、goal
   revision、模式切换）与外部效果动作（文件写、命令执行、网络、凭据读取）必须持一次性
   HMAC-SHA256 票据；**系统记账写入**（journal append、blackboard 单写者分区、snapshot、
   session 元数据、`.gsa` 结构目录）**不持票**，避免机制自锁。
2. **票据绑定**：`session + agent(+activation) + goal_version/goal_digest + policy_revision +
   action_kind + canonical_arguments_sha256 + resolved_target_sha256 + capability_scope +
   sequence + nonce + issued_at/expires_at + previous_receipt_sha256（+ user_confirmation_sha256）`，
   控制类票据另绑 `template_sha256`——**签名器只接受枚举化动作，不接受任意文本**。
3. **验票对象是解析后真实对象**：命令绑定规范化 argv/cwd/env 摘要，文件绑定解析后绝对路径；
   拒绝 symlink、junction、reparse point、路径穿越；防止 TOCTOU 与等价编码绕过。
4. **签发器独立（v1 级别：独立进程窄 IPC）**：主密钥不进入 Agent 进程；启动前由受信启动器
   验证签名 manifest 与二进制哈希；接口只接受枚举化动作；更新采用签名 manifest + 单调
   `signer_revision` + 回滚保护，**否决"永远固定单一哈希"**（会阻断安全升级）。
5. **密钥体系**：`K_install`（DPAPI 静态，现有 keystore）只存于签发器进程；
   `K_session = HKDF(K_install, session_id || goal_digest || policy_digest || signer_revision)`
   下发给 host 验票模块；goal 或 policy 变化 → 新 `K_session`，旧票立即失效。
6. **原子消费与恢复**：验票通过→执行→receipt 与结果一个 commit 落盘；崩溃恢复不重放；
   复用 ADR-0010 §4.4 的单写者串行与 CAS/幂等模式，不另造平行机制。
7. **三运行模式，全部手动开启**：`Normal`（默认，现状审批语义 + 票据接线）、
   `Auto-Staged No-Run`（长期自动迭代，只写暂存根；**无进程创建**；`SandboxLease` 租约内
   写入自动批准并产生 receipt；回写宿主必须 `PromotionPermit` 且重验 base_workspace_digest）、
   `Ephemeral Sandboxed Run`（暂存快照复制到一次性 VM/容器内运行进程与测试；只回传脱敏截断
   日志/退出码/摘要；每次结束销毁；默认无网络、无凭据、无宿主挂载）。**不做"完全授权"模式**；
   最高层级即临时沙盒运行。
8. **沙盒进程创建能力分开裁决**：No-Run 模式关闭；Ephemeral 模式开启但**强制 VM 级隔离**
   （v1 采用 Windows Sandbox，即 Hyper-V 轻量 VM；完整 Hyper-V VM 为强档，仅高风险任务启用），
   AppContainer/Job Object 组合在 raw TCP 残余与子进程继承未闭环前不得作为该模式隔离依据。
9. **模式切换**：必须 `ModeChangeTicket`（用户确认）；切换作废全部未消费票据与租约、
   递增 `policy_revision`（落实 GAP-DENIAL-POLICY-REVISION 接线）；模型不能切换模式、
   退出模式或扩大范围。
10. **威胁模型边界**：本设计防御**受提示词注入影响的 Agent（模型/子代理/插件）滥用工具**；
    不防御同用户恶意进程、管理员/内核、签发器本身被攻破与恶意模型服务（用户态机制够不着，
    见设计文档 §2.2）。
11. **影子模式先行**：每个实施切片先记录"应签未签/应拒放行"差异台账，确认零误阻断后再
    切换 fail-closed；验票失败一律拒绝并写安全事件。
12. **实施切片独立于 ADR-0010 §9 Phase C**：Slice 1（签发器 v1 + 控制事件票据，无前置依赖）、
    Slice 2（动作票据 fail-closed；前置 RT-001~003 闭合与 policy_revision 接线）、Slice 3
    （Auto-Staged No-Run）、Slice 4（Ephemeral Sandboxed Run）。共享 Phase C 的
    "先扩展 Schema/fixture 再修改 producer"迁移纪律，但不共享排期。

## 3. 后果

### 3.1 正面

- 注入损害从"可能执行"降为"只能在既定策略范围内申请"；
- 伪造控制标记、重放、参数替换复用旧许可、崩溃重放一次性写、插件冒充控制面均被机械拒绝；
- 长期 vibe coding 连续性与安全并存：自动迭代不打断用户，但任何越界都不落地；
- 两个现役缺口获得消费面：policy_revision 接线、RT-003 workspace delta（写入 receipt 覆盖）；
- 审计完整性补强：receipt 签名链补 journal 无密钥哈希链的"来源真实性"缺失。

### 3.2 代价与风险

- 独立签发器是新增二进制与 IPC 面：启动验证、更新链、密钥保护与故障语义需持续维护；
- 动作规范化与目标解析是主要工程量（非 HMAC 本身），存在长尾边界（编码等价、路径重解析）；
- 三模式依赖 workspace 级沙箱真实存在，而当前 Rust 侧只有 Job Object containment；
  沙箱 backend 需独立设计、实现与验证，硬件条件满足只是前提；
- fail-closed 引入新故障点：签发器不可达必须拒绝而非降级，需与既有守卫（tool timeout、
  wallclock、stall watchdog）协调，避免叠加误杀；
- 模式切换面扩大会话状态机与审计复杂度。

## 4. 验收条件

- 设计文档、本 ADR 与 ADR-0010 §11.8 登记一致；索引路由完整；
- 每个切片通过探针矩阵（伪造/重放/越权/逃逸/崩溃重放全拒）与影子模式台账（零误阻断）；
- 新事件类型（ticket issued/consumed/rejected、lease、promotion、mode change）经 v0.2
  Schema/fixture 双轨 verifier 校验，negative 覆盖 retired/冲突语义；
- 三代理审查惯例继续适用于每个实施切片；无 P1/D1 级背离；
- 本 ADR 不因实施延期而改写；实施进度与偏差按索引与审计文档记录。
