# GAP-ACAF-SLICE2A 实施审计（2026-08-12）

> 状态：**Slice 2 第一阶段已实施并验证**（ADR-0011 §2 决策 12 / 设计文档 §7
> Slice 2 的子集——动作票据 file_write 全链贯通 + credential_read 机制先建 +
> 目标解析 TOCTOU + 影子模式台账；**全程影子模式，fail-closed 未切换**）。
> 范围：`docs/audits/` 惯例——实施事实、决策登记、验证证据、边界。设计权威回查
> [`ADR-0011`](../../adr/ADR-0011-authenticated-control-and-action-fabric.md) 与
> [`AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md`](../../docs/AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md)。
> 用户裁决（2026-08-12）：① 最小闭环——file_write 一类全链贯通，其余动作类各作一步
> 复制扩展；② credential_read 机制先建、登记不接线（无现役消费工具）；③ 影子台账
> 用 journal 事件承载（沿用 Slice 1 语义）。

## 1. 摘要

ACAF Slice 2 第一阶段实施完成：`TicketKind` 新增 `file_write_v1` / `credential_read_v1`
两个动作类变体；`ControlTicket`/`IssueContext`/`VerifyContext` 补
`resolved_target_sha256`（解析后真实目标摘要）；新目标解析模块
`orz-assurance/src/acaf/target.rs`（词法解析 + reparse 检查 + 摘要，语义镜像
`resolve_model_path`）；签发器新增 `sign_file_write_v1` / `sign_credential_read_v1`
两个枚举化 method；host 客户端与 controller 全链接线——`run_host_tool` 在
permission gate 与 IP5 快照之间插入动作票据生命周期（签发→issued→**重新解析重算**→
验票→consumed/rejected），**影子模式：验票失败照常执行并记 `control_ticket_rejected`**
（无法开票的解析失败 → null ticket_id + `target_mismatch`）。事件类型复用
`control_ticket_issued/consumed/rejected`（只扩 `ticket_kind`/`capability_scope`
枚举 + `resolved_target_sha256` 字段），无新事件类型、零 Schema 枚举计数变化。
Schema 先行（主仓）→ producer（orz 仓）纪律完整执行，含修 Slice 1 遗留的
`FIXTURES_README_V02` 漂移项。

## 2. 交付清单

| 层 | 文件 | 内容 |
|---|---|---|
| Schema | `runtime/control-ticket-issued-event-payload-v0.2.schema.json` | `ticket_kind`/`capability_scope` 枚举 4→6；新 `resolved_target_sha256`（oneOf 64hex/null）；allOf 重构（activation 逐 kind 显式分支 ×3 + capability const ×2 + 动作类 target required/pattern + 控制类"若有则 null"） |
| Schema | `runtime/control-ticket-consumed/-rejected-event-payload-v0.2.schema.json` | `ticket_kind` 枚举 +2 |
| Fixture | `scripts/generate_run_event_fixtures.py` | GOOD/BAD issued + `resolved_target_sha256: null`（坏例保持单违反）；**修 Slice 1 漏项** `FIXTURES_README_V02`（8→11 事件、39→42、control-ticket 说明 + Slice 2 kind 说明）；重生成（4 文件 diff） |
| 测试 | `runtime/tests/test_run_event_journal_validation.py` | `_issued_ticket` helper +2 kind + 可选 resolved_target；`ActionTicketSchemaTests` 6 例；`ControlTicketPairingTests` +1（kind 无关性） |
| 核心 | `orz-assurance/src/acaf/mod.rs` | `TicketKind` +2 变体 + `requires_target()`；三结构体 + `resolved_target_sha256`；`issue_ticket` 对偶校验（MissingTarget/UnexpectedTarget + validate）；`verify_ticket` check 5b（三形态 TargetMismatch）；`parse_kind` +2；`verification_context_from` 透传；单测 +4 |
| 核心 | `orz-assurance/src/acaf/target.rs`（新） | `resolve_action_path`（sanitize/tilde/rooted 原样/join/`..` 折叠/verbatim 拒绝）、`has_reparse_or_symlink_component`、`is_reparse_or_symlink`（0x400 位移植）、`canonicalize_if_exists`、`resolved_target_digest`（统一斜杠 + sha256）；单测 9 |
| 签发器 | `orz-bin/src/bin/orz-signer.rs` | method 分派 +2（`sign_file_write_v1`/`sign_credential_read_v1`，target 参数必取）、`handle_sign` +`target_param`、`Signer::sign_ticket` +参数、方法表注释；单测 +3 |
| 客户端 | `orz-loop/src/acaf.rs` | `sign_ticket` +`resolved_target_sha256` + method map +2；`verify_and_consume` +`live_resolved_target_sha256` + kind 解析 +2；`issued_payload` +1 字段；新 `file_write_canonical_args` / `action_kind_for_tool`；单测 +2 |
| 接线 | `orz-loop/src/controller.rs` | `ticket_flow` 抽取（控制/动作共享）+ `journal_ticket_outcome` + `shadow_action_rejection` + `acaf_action_event`（解析→reparse→digest→签发→issued→**重新解析重算**→验票→终态）；`run_host_tool` 插线（permission 后、IP5 前）；`acaf_control_event` 保持签名（4 控制调用点零改动）；单测 +1 |
| E2E | `orz-bin/tests/acaf_e2e.rs`（Windows） | `file_write_ticket_full_chain`（真实签发器 + 真实 controller + search_replace → issued/consumed 配对、capability/activation/resolved_target 断言、票先于 ToolStarted）、`file_write_shadow_on_signer_unreachable`（影子放行 + rejected）；既有调用点补 None 参数 |

## 3. 设计决策与登记（v1 简化，Slice 2 第一阶段）

| # | 决策 | 依据 / 登记 |
|---|---|---|
| D1 | `TicketKind` 只加 `file_write_v1` + `credential_read_v1`；command_exec/network 接线时再入枚举 | 后两者的参数形态（argv/cwd/env、URL/endpoint）与目标解析语义未定型；入枚举等于为未设计的 schema 面写正反例。扩展成本为零（照 file_write 复制一步） |
| D2 | 动作类 `requires_activation() = false`（activation 必须 null） | search_replace 只在主 lane 可达（检索 lane 被 `write_gate` 前置拒绝 `retrieval_role_write_denied`）；主 agent 无 activation。子代理持票仍属 Slice 1 审计 §5.6 登记面 |
| D3 | 签发器保持纯签名预言机形态：客户端传 `canonical_arguments_sha256` + `resolved_target_sha256`；设计文档 §4.4 `SignFileWriteV1(path_resolved, content_sha256, lease_id?)` 结构化形态**登记为偏差** | TOCTOU 安全属性全在验票端（check 5/5b 用 live 参数重算）——签发器无 FS 无 cwd，收结构化参数只能对客户端字面量重哈希，安全增益为零；结构化参数要求签发器复制 canonical 包装格式，制造双处漂移面。`lease_id` 无 SandboxLease 机制不实现 |
| D4 | 目标解析层 = `orz-assurance/src/acaf/target.rs`（纯函数 + IO 检查都放这） | orz-assurance 依赖含 dunce + windows（cfg），无依赖障碍；`resolve_model_path`（orz-tools）不可复用（orz-loop 禁止依赖 orz-tools）。**语义镜像 + 两个有意差异**：① verbatim `\\?\` 前缀拒绝（绕过词法规范化）；② `..` 组件词法折叠（摘要覆盖真实触及对象）。显示路径 display_cwd 简化为 worktree 单一基（生产上 display==cwd） |
| D5 | file_write canonical 参数 = `{"tool":"search_replace","file_path":"<解析后绝对路径>","operation":"create"|"modify","content_sha256":sha256(new_string)}` | 空 `old_string` = 工具的新建路径语义 → create；content 摘要绑进票面——篡改 `new_string` 即 target_mismatch。`action_kind_for_tool` 是唯一工具→kind 映射点（后续扩展只改一处） |
| D6 | 验票插入点：`run_host_tool` permission Deny/Defer 早退之后、IP5 快照之前 | 被拒调用不执行→无需票（双层门禁：permission 管"策略允许"、票据管"本次授权"）；快照是证据层放票后；ToolStarted 绝不先于验票 |
| D7 | 影子语义：解析失败/路径非法（无票可发）→ `control_ticket_rejected(ticket_id=null, target_mismatch, detail)`；签发/验票失败 → 现 8 code 复用；**全部照常执行** | "影子差异台账"= journal 事件本身（用户裁决③）；null ticket_id 沿用 `signer_unreachable` 先例（Python 配对规则 L691-692 对非 str ticket_id 跳过，verifier 兼容已核实）。fail-closed 切换 = 完整 Slice 2 里程碑（用户观察台账后另行裁决） |
| D8 | credential_read 机制全建（kind + 签发器 method + 验票路径 + schema + fixture），controller 零接线 | 无现役凭据读取工具（orz-tools implementations 确认）；验证靠核心/签发器/schema 测试；未来接线需目标形态裁决（ADR-0006 凭据注册表） |
| D9 | TUI 三处零改动 | 事件类型未变，`ticket_kind: String` 字段天然透传；`resolved_target_sha256` 不进投影（纪律：TUI 不含摘要/HMAC） |
| D10 | capture 不重捕不加场景 | capture 环境无 acaf 配置（env 门控）→ 零事件、13/13 不变 |
| D11 | schema 兼容：`resolved_target_sha256` 对控制类非 required（可缺省或 null），动作类 allOf 强制 required+非空 | Slice 1 历史 journal（无该字段）保持合法；新 producer 恒带该字段（控制类 null） |
| D12 | check 5b 语义：动作类票面 None / 验票上下文 None / 两者不等 → 均 `TargetMismatch`；控制类带 target → `TargetMismatch` | 验票端"live 重算"的诚实结构——`acaf_action_event` 签发与验票之间**重新解析 + 重查 reparse + 重算 digest**，窗口内 symlink 换入命中 target_mismatch（影子可见） |

## 4. 验票七项检查实现映射（Slice 2 扩展）

| 检查 | 实现（Slice 1 基础 + Slice 2 扩展） |
|---|---|
| 1 签名有效 | 不变（`resolved_target_sha256` 在 canonical body 内，篡改即 check 1 失败） |
| 2 模板版本 | 不变（动作类 `requires_template()=false` → 恒 null） |
| 3 单调+一次性 | 不变（TicketLedger） |
| 4 上下文绑定 | 不变（动作类 activation 恒 null，D2） |
| 5 真实对象摘要 | `canonical_arguments_sha256` 比对不变；**新增 5b**：动作类 resolved_target 三形态比对（票面 None / live None / 不等）+ 控制类带 target 拒绝，均 `TargetMismatch` |
| 6 未过期 | 不变（5s TTL） |
| 7 链一致性 | 不变（journal `previous_event_sha256` + Python 配对规则；null ticket_id 跳过） |

## 5. 验证证据

- **Rust**：orz-assurance **122/0**（107 基线 + 4 核心 + 9 target.rs +
  审查修复批 2：`checked_resolve_scans_unfolded_candidate`/`tilde_user_rejected`）、
  orz-loop **188/0/3**（184 + acaf.rs 3 + controller 1）、orz-bin：
  signer **9/0**（6+3）、e2e **6/0**（4+2，Windows DPAPI 真实进程）、main 6、
  real_flag **2**、stdio_e2e 1；orz-tui **177/0**（零改动回归）；
  clippy 四 crate **0 warning**（4 处 orz-loop 新警告 + 1 处 e2e 未用 import 已修）
- **E2E（真实进程，Windows DPAPI）**：
  - `file_write_ticket_full_chain`：scripted search_replace run —— journal 断言
    issued=1（file_write_v1/capability=file_write/activation=null/
    resolved_target_sha256 64hex）+ consumed=1 配对（同 ticket_id/kind/outcome=accepted）、
    rejected=0、ToolStarted/ToolCompleted 存在、**票据事件先于 ToolStarted**
  - `file_write_shadow_on_signer_unreachable`：杀签发器 + 删 manifest →
    rejected(signer_unreachable, ticket_kind=file_write_v1) 且工具照常执行（影子）
  - 既有 4 测试全绿（调用点补 None 参数）
- **核心单测**：动作票 roundtrip（all_six_kinds）、MissingTarget/UnexpectedTarget、
  check 5b 三形态（重签手法隔离 5b 与 check 1）、target.rs 解析全规则
  （相对 join/引号转义/`..` 折叠/tilde/`~user` 不展开/rooted 原样/verbatim 拒绝/
  reparse 组件检出/摘要稳定敏感）
- **Python**：runtime **196 passed**（189 基线 + ActionTicketSchemaTests 6 +
  配对扩展 1）、assurance **1606 passed/14 skipped**、conformance 不变、
  check_repository **valid**、capture **13/13**（acaf 默认关闭零影响）、
  fixtures 重生成一致（仅 issued 4 文件 +1 字段 + README 修订，diff 精确）
- **git**：主仓 12 文件（11 修改 + 审计文档新增）、orz 6 文件（5 修改 +
  target.rs 新增）；diff --check 干净

## 6. 边界登记

- **影子模式是验收语义**：验票失败照常执行写入；fail-closed 切换 = 完整 Slice 2
  里程碑（ADR-0011 决策 11），用户观察台账后另行裁决；
- **双实现漂移面（已单源化 2026-08-12）**：解析原语迁入 `orz-paths::resolve`
  （三消费方共享：orz-tools 薄壳 / orz-assurance 委托 / orz-host 直调）——
  有意差异保留在票据侧编排（verbatim 拒绝 / `..` 折叠 scan-before-fold /
  `~user` 拒绝 / 单基 worktree）；折叠委托 `normalize_lexically` 引入一处登记
  行为变化（驱动器相对 `C:..\x` 保留 `..`），详见
  [`GAP_ACAF_GOAL_POLICY_WIRING_IMPL_AUDIT`](./GAP_ACAF_GOAL_POLICY_WIRING_IMPL_AUDIT_2026-08-12.md)；
  漂移自检出保持（验票 target_mismatch → 影子记录可见）；
- **verify→execute 残余窗口**：验票与执行之间 symlink 换入不可察（关闭需执行器
  进程路径，设计文档 §10 v2）；
- **非 search_replace 的 LocalMutation**：本工具集只有 search_replace 是文件写工具
  （bash=SandboxEscape、run_terminal_cmd 未声明），"应签未签"差异面登记；
  command_exec/network 接线后续各作一步复制扩展（D1）；
- **policy_revision 恒 0（已接线 2026-08-12）**：live 值穿透 5 接线点 +
  `bump_policy_revision` 机制 + goal_version 真实接线（AcceptedContinue 消费
  GoalRevisionV1 票后重派生 K_session，旧票死），独立审计
  [`GAP_ACAF_GOAL_POLICY_WIRING_IMPL_AUDIT`](./GAP_ACAF_GOAL_POLICY_WIRING_IMPL_AUDIT_2026-08-12.md)；
  Slice 3 前无生产递增来源（登记）；
- TTL 5s + 解析 IO（每组件一次 symlink_metadata，毫秒级）；慢路径超期 → 影子拒绝
  （沿用 Slice 1 登记）；capture 不重捕（D10）；e2e Windows-only（Linux 侧由
  签发器单测覆盖，non-Windows 下 reparse 检查退化为仅 symlink）；
- **环境坑**：E0463 缓存损坏（orz-host rmeta）→ `cargo clean -p orz-host -p orz-bin`
  重建解决（+6.3GiB）；`-j2` 在 `--` 后传测试二进制报 Unrecognized option（先例）；
  `cargo fmt --check` 全仓报 orz-agent/builder.rs pre-existing 漂移（非本次改动，
  未修）；Slice 1 已登记项延续（签发器 stderr 丢弃、AcafClient 无 Drop kill、
  MANIFEST 常量非 64hex）；
- **审计措辞纪律**（C1-2 教训）："+N 测试"均列真实测试名；fixture 文件数 ≠ 测试数；
- **fail-closed 翻转前置核查清单**（review D2-1/P2-3/P2-4/P2-5 登记 + 2026-08-12
  goal/policy 接线审查 D3-2 增补，翻转里程碑必查）：
  ① 每次工具集变更重审 `action_kind_for_tool` 覆盖（"应签未签"面——未映射的
  LocalMutation 工具调用零事件，今日可达写工具集恰好全映射）；② 工具自身校验拒绝
  （gitignore 拒绝、old_string==new_string、new_string 缺失）仍记 consumed "accepted"
  ——审计语义噪声，翻转时决定是否补 rejected；③ `file_path` 键名硬编码（snapshot_targets
  认三键）+ 缺失时静默跳过——翻转时须转硬拒绝；④ acaf 未配置 / snapshot_store 缺失 /
  goal_digest 缺失的静默 skip 分支——翻转时逐项转硬拒绝或显式告警（P2-5）；
  ⑤ capture 无 acaf 场景（当前 13/13 零票据事件）；⑥ **goal/policy 接线（2026-08-12
  增补）**：影子模式下 rejected 的 GoalRevisionV1 也照常翻转 goal 绑定——翻转时必须把
  `update_goal`（及整个 AcceptedContinue 状态迁移）gate 在票 consumed 之上；
  跨 run 的 policy 身份变化不被 K_session 捕获（per-run 归零，TTL 5s 界）——Slice 3
  模式切换需会话级携带计数器或显式接受（新审计 D3-1/D3-2）。
- **fmt 漂移面**（review C2-3）：`cargo fmt --check` 全仓报 orz-agent/builder.rs 及
  六个变更文件的 pre-existing 漂移（controller 159 hunks 等，HEAD 即存在）——本次
  未全仓 fmt（避免无关 diff），变更文件由 clippy 0 + 编译保证；全仓 fmt 留待独立批次。

## 7. 下一步

- **Slice 2 完整闭合**：command_exec（run_tests ExecutionPermit + shell argv 规范化）
  与 network（web_fetch/browser_read URL 目标解析——check_ssrf 已存在）各作一步
  复制 file_write 同型扩展；之后**用户裁决 fail-closed 切换**（探针矩阵 + 影子台账
  零误阻断）；
- 可选：conformance capture 新增票据场景（当前 capture 无 acaf 配置，零事件）；
  目标解析与 `resolve_model_path` 单源化（把解析移入 orz-tools 或反向引用，需
  依赖方向裁决）。

## 8. 三面审查闭环（2026-08-12，用户发起"全面检查"）

设计/实现/符合性三独立代理审查——**无 P0/D1 级未修项**；修复批已应用并复验：

**必须修（D1/P1）**：
- **D1-1（设计）`..` 折叠先于 reparse 扫描——票据绑定对象 ≠ 实际写入对象（fail-open 方向）**：
  `<junction>\..\..\secret.txt` 拼写折叠后扫描看不到 junction，而 OS 逐组件解析先跟随
  链接——票据绑折叠路径、工具写链接目标，且两侧自洽 → 验票通过（影子台账唯一静默
  吞掉的偏差类）。修复：新 `resolve_action_path_checked`（解析+IO 检查一体）在
  **未折叠候选**上先做 reparse 扫描（parent() 链仍携带 link 组件）再折叠；
  controller 签发/验票两处改用（`target.rs` + `controller.rs` 两调用点）；
  新单测 `checked_resolve_scans_unfolded_candidate`（symlink 目录 + `link\..\..\secret.txt` →
  ReparseComponent 拒绝；词法解析仍折叠）
- **P1-1（实现）operation 分类与工具语义不符**：空 `old_string` 打在已存在非空文件上
  工具实际全量覆写（`handle_new_file_creation`，`empty_old_string_does_not_override=false`
  默认），票据却绑 `create`。修复：新 `file_write_operation`（empty old_string **且**
  目标缺失或空 → create，其余 modify）在签发/验票两处对称探测——窗口内文件创建 →
  operation 翻转 → digest 不同 → TargetMismatch（诚实 TOCTOU 信号）；
  单测 `file_write_operation_matches_tool_semantics`
- **P1-2（实现）`~user` 与 shellexpand 真实分歧**：Unix 上工具展开 `~user`、票据不展开
  → 假绑定（consumed 而非 rejection）。修复：`~user` 形态直接拒绝
  （`TargetResolveError::TildeUserUnsupported`，fail-closed——含 home 缺失时）；
  单测 `tilde_user_rejected`

**应修（P2/C1，互证）**：
- **P2-1/C2-2 组合**：verbatim 拒绝漏 `DeviceNS`（`\\.\C:\...`）→ `Prefix::DeviceNS(_)`
  补入 + 单测；审计状态行 "ADR-0011 §7" → "ADR-0011 §2 决策 12 / 设计文档 §7"
- **P2-2（实现）check 5b 控制类不对称**：live 上下文携带 target（调用方 bug）被静默
  接受 → `else if` 增 `vctx.resolved_target_sha256.is_some()` 对称拒绝
- **P2-6（实现）acaf Mutex 跨整个生命周期**（含 re-resolve fs 遍历 + signer respawn）→
  锁范围拆分：sign（锁作用域 1）与 verify（锁作用域 2）各短持锁，journal issued 与
  re-resolve 在锁外
- **P2-7（实现）HOME 回退缺 HOMEDRIVE+HOMEPATH**（dirs 有此后备）→ 补回退链
- **C1-1（符合）审计 §5 主仓文件数**："9 文件" → "12 文件（11 修改 + 审计新增）"
- **C2-1（符合）real_flag 计数**："1" → "2"（`real_and_fake_provider_*` 两测试）

**登记项（D2/D3/P2/C2，不修回填）**：
- **D2-1** "应签未签"面不可动态观察（未映射工具零事件）→ fail-closed 前置核查清单
  （§6）；**D2-2** `~user` 第三镜像差异——已由 P1-2 升级为拒绝（消除差异）
- **P2-3** 工具自身校验拒绝仍记 consumed（gitignore 拒绝/old==new/new 缺失）→ §6 清单②
- **P2-4/P2-5** `file_path` 键硬编码 + 四个静默 skip 分支 → §6 清单③④
- **P2-8** controller 级测试盲区（resolve 失败影子拒绝 / reparse TOCTOU / file_path 缺失
  静默）→ e2e create 场景未补（create 分支由 unit 测试承担；controller 级需真实
  AcafClient 无法 mock，留 e2e 后续）
- **C2-3** fmt 漂移面（六个变更文件 pre-existing + orz-agent）→ §6 登记
- **C2-4** file_path 缺失完全静默 → 与 P2-4 合并登记
- **D3-1/D3-2**（credential_read 形态预裁决 / check 5b 与 check 5 冗余关系）：确认可接受，
  不修

**复验（修复后全量）**：orz-assurance **122/0**（+2 新测试）、orz-loop **188/0/3**
（+1 新测试）、orz-bin signer 9 + e2e 6 + main 6 + real_flag 2 + stdio_e2e 1、
clippy 四 crate **0 warning**、capture **13/13**、pytest runtime **196** +
assurance 1606/14（本批未动 Python 层）、check_repository valid、diff --check 干净。
