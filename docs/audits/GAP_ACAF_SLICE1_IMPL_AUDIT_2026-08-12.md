# GAP-ACAF-SLICE1 实施审计（2026-08-12）

> 状态：**Slice 1 已实施并验证**（ADR-0011 §7 Slice 1——签发器 v1 + 控制事件票据，无前置依赖）。
> 范围：`docs/audits/` 惯例——实施事实、决策登记、验证证据、边界。设计权威回查
> [`ADR-0011`](../../adr/ADR-0011-authenticated-control-and-action-fabric.md) 与
> [`AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md`](../../docs/AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md)。

## 1. 摘要

ACAF Slice 1 实施完成：独立签发器进程（`orz-signer`）+ host 验票客户端 + 四类控制事件
（Orientation / parent disposition / close / goal revision）持一次性 HMAC-SHA256 票据，
全部影子模式（失败仅记录 `control_ticket_rejected`，不阻断控制事件）。新事件类型
`control_ticket_issued / _consumed / _rejected` 经"先 Schema 后 producer"纪律落地，
Python 双轨 verifier 机械配对校验全绿。E2E 用真实签发器进程 + 真实 controller 路径验证。

## 2. 交付清单

| 层 | 文件 | 内容 |
|---|---|---|
| Schema | `runtime/control-ticket-issued-event-payload-v0.2.schema.json`（新） | 票据签发事件 payload（绑定字段，**不含 HMAC**；kind→capability_scope allOf 映射；orientation 强制 template+无 activation） |
| Schema | `runtime/control-ticket-consumed-event-payload-v0.2.schema.json`（新） | 消费事件（outcome=accepted const） |
| Schema | `runtime/control-ticket-rejected-event-payload-v0.2.schema.json`（新） | 拒绝事件（8 个 reject_code 枚举） |
| Schema | `runtime/run-event-v0.2.schema.json` | event_type 枚举 39→42 |
| Verifier | `assurance/run_event_journal_validation.py` | 3 事件注册 + `_verify_v02_control_tickets` 机械规则（issued 必须先行、kind 一致、one-shot 终态互斥） |
| Fixture | `scripts/generate_run_event_fixtures.py` + 生成物 | 3 事件 minimal.valid/constraint.invalid + envelope（v0.2 42 事件全覆盖） |
| 测试 | `runtime/tests/test_run_event_conformance.py` | 枚举计数 39→42 |
| 核心 | `orz-assurance/src/acaf/mod.rs`（新） | ControlTicket/TicketKind/IssueContext/VerifyContext/七项检查/TicketLedger/K_session HKDF 派生/纯函数单测 11 |
| 签发器 | `orz-bin/src/bin/orz-signer.rs`（新） | 独立进程：manifest 自校验启动（fail-closed）、DPAPI K_install（非 Windows fail-closed）、枚举化 stdio JSON-lines 接口、orientation 模板签发器持有、单测 6 |
| 客户端 | `orz-loop/src/acaf.rs`（新） | spawn/stdio JSON-RPC/ensure_initialized（goal 变化→新 K_session）/sign_ticket/verify_and_consume（七项+ledger）/事件 payload 构造/单测 3 |
| 接线 | `orz-loop/src/controller.rs` | `acaf: Option<Arc<Mutex<AcafClient>>>` + `with_acaf` + `acaf_control_event` 统一 helper + 四接线点 |
| 接线 | `orz-bin/src/main.rs` | `build_acaf_client`（`ORZ_ACAF_MANIFEST`+`ORZ_ACAF_KEYSTORE` env 门控） |
| 事件 | `orz-assurance/src/journal/event.rs` | EventType +3 变体 |
| TUI | `orz-tui/src/{bridge,events,projection}.rs` | 三事件投影（kind/scope/reject_code，无 HMAC/摘要） |
| 密钥暴露 | `orz-host/src/keystore.rs` | `secret_bytes()` 公开（签发器取 K_install 原始字节；调用方必须 zeroize） |
| 注册 | `orz-assurance/src/lib.rs` | acaf 模块注册 |
| E2E | `orz-bin/tests/acaf_e2e.rs`（新，Windows） | 真实进程全链 4 测试 |

## 3. 设计决策与登记（v1 简化）

| # | 决策 | 依据 / 登记 |
|---|---|---|
| D1 | **IPC = stdio JSON-lines 单请求单响应**（设计文档 §4.4 写"命名管道 + ACL"） | 进程边界即 ACL（子进程 stdin/stdout 仅启动者可达）；跨平台（Windows 本机 + Linux musl 评测容器）；复用 acp_server stdio 先例。命名管道升级登记为 v2 |
| D2 | **启动验证 = host 提供 manifest（JSON）+ 签发器自校验二进制哈希**（设计文档要求"签名 manifest + 独立发布密钥"） | 签名 manifest 链需独立发布密钥基础设施——v2 升级路径。威胁模型 §2.2 边界：更新链被控已是严重事件，哈希校验防二进制替换 |
| D3 | **影子模式全程**（§2 决策 11） | Slice 1 验收入口"拒绝路径写安全事件"——`signer_unreachable` 是核心拒绝路径（签发器不可达 fail-closed 的记录面）；动作票据 fail-closed 切换在 Slice 2 |
| D4 | **goal_digest = sha256(canonical {goal: prompt})**，run 开头固定；goal_version 恒 0；policy_revision 恒 0 | controller 无 goal 版本计数器；GAP-DENIAL-POLICY-REVISION 接线面 = Slice 2。K_session 的 policy_digest 输入用 8 字节 LE policy_revision 占位（登记：未来切换即全量重派生） |
| D5 | **goal revision 票绑定旧 goal 上下文** | continue 的 requirement_delta 修订 next_goal；K_session 重派生（新 goal）登记为 Slice 2 接线（controller 的 goal_digest 缓存不更新） |
| D6 | **拒绝判定（rejected_stale/conflicting）与 replay 幂等不签票** | 纯机械记录非状态移动；真实状态移动（accepted close/continue、close record、orientation fire、goal revision）持票 |
| D7 | **pre-handoff orientation（audit-only 不注入）不签票** | 非模型控制通道；真实注入的 fire 才持票 |
| D8 | **未配置签发器（env 缺失）→ 零票据事件** | 零行为变化；配置后才产生票据事件 |
| D9 | **模板文本双处持有**（prompt.rs 注入块 + orz-signer 常量） | 单一 `template_sha256` 常量在 orz-assurance（acaf 模块）机械固定两端；验票检查 2 用签发器下发的权威值（非票据自报） |
| D10 | **签发器依赖 orz-bin crate（与 orz 同包）** | 复用 keystore/DPAPI 零迁移；"独立进程"语义由进程边界保证（进程不共享内存/文件句柄）；musl 下 orz-signer 因无 DPAPI fail-closed——评测容器签发器不可用正是设计行为 |

## 4. 验票七项检查实现映射（§4.2）

| 检查 | 实现 |
|---|---|
| 1 签名有效 | `verify_ticket` canonical_body（去 hmac 字段）→ `signer.verify`（K_session） |
| 2 模板版本 | `TicketKind::requires_template()` 时比对签发器下发 `template_sha256`（客户端 session 字段，非票据自报） |
| 3 单调+一次性 | 核心：sequence 上限合理性；消费：`TicketLedger`（session 级 nonce 集 + sequence 单调） |
| 4 上下文绑定 | session/agent/activation/goal(v+digest)/policy 逐项比对 |
| 5 真实对象摘要 | `canonical_arguments_digest(kind, args)` 调用点用**实时参数**重算比对（TOCTOU 面） |
| 6 未过期 | issued_at/expires_at 与 now 比对（5s TTL） |
| 7 链一致性 | 进程内 ledger（check 3 承担）；跨进程链=journal `previous_event_sha256`（票据事件链由 verifier 配对规则承担） |

## 5. 验证证据

- **Rust**：orz-assurance 106/0（+11 acaf）、orz-loop 184/0/3 ignored（+3 acaf 客户端）、
  orz-bin 6+6+3+2+1（含 3 个 e2e、stdio_e2e、real_flag）、orz-tui 177/0；
  clippy 四 crate 0 warning；-j2 低并行（E0463 并行竞态教训复现规避）
- **E2E（真实进程，Windows DPAPI）**：
  - `signer_process_full_lifecycle`：manifest 自校验 + 临时 DPAPI keystore + 签发→消费；
    **重放同一票 → replay_detected**；**live 参数与票不一致 → target_mismatch**
  - `controller_control_events_carry_tickets`：scripted 检索 run（accepted close）——
    journal 中 **2 张票（disposition + close）issued→consumed 配对**、ticket_kind 正确、
    **sequence 单调**、goal_digest 64hex 绑定
  - `signer_unreachable_shadow_records_rejection_and_proceeds`：杀签发器后 run——
    **control_ticket_rejected(signer_unreachable) 记录且 control event 照常发生**（影子语义）
- **Python**：runtime 189 passed（基线 183 + ControlTicketPairingTests 6 个正/反例；
  3 新事件并入既有 contract 测试的 subTest，pytest 计数不含 fixture 文件增量——
  v0.2 payloads 22 + envelope 58 为文件数非测试数，审查 C1-2 修正）、
  assurance 1606 passed/14 skipped、conformance 109 passed（含配对规则 6 测试，
  审查 C1-1 补齐——此前该规则零自动化覆盖）、
  check_repository **valid**、capture 13/13（acaf 默认关闭零影响）、fixtures 重生成一致
- **git**：diff --check 待确认（提交前）

## 6. 边界登记

- 签发器在 Linux（musl）下因无 DPAPI fail-closed——评测容器内签发器不可用属设计行为；
- manifest 无独立发布密钥签名（v2）；IPC 无命名管道（v1 登记 D1）；
- goal 修订不触发 K_session 重派生（D5）；policy_revision 恒 0（Slice 2 接线）；
- 控制事件票 TTL 5s——事件消费点在签发后毫秒级，慢路径（journal 写阻塞）可能超期 → 影子记录；
- `signer_revision`/`signer_measurement` 客户端持有待 Slice 2 审计面使用；
- 子代理 lane 的 orientation fire 以 `agent_role` 作为 canonical args 入票（activation 为空）——
  disposition/close/goal_revision 持 activation 绑定，orientation 是 session 级（schema allOf 一致）；
- 模板文本双处持有（D9）：prompt.rs 与 orz-signer 需同步修改（登记为已知漂移面，两端
  由同一 template_sha256 机械校验——模板改而不同步时 orientation 票被拒 → 影子记录可发现）；
- E2E 为 Windows-only（DPAPI 依赖）；Linux 侧进程级验证由签发器单测（stdio 管道）覆盖。

## 7. 下一步

- Slice 2 前置已全部就位（RT-001~003 闭合 2026-08-11 + policy_revision 接线面 = 本切片的
  K_session 派生输入）；
- 可选：conformance capture 新增票据场景（当前 capture 无 acaf 配置，零事件，未重捕）。

## 8. 三面审查闭环（2026-08-12，用户发起"全面检查"）

设计/实现/符合性三独立代理审查——**无 D1/P0/C1 级未修项**；修复批 16 项已应用并复验：

**必须修**：
- **D1-1（设计）模板幻影**：签发器持有的 orientation 模板与真实注入文本不一致（一行旧稿
  vs 多行 `ORIENTATION_BLOCK`）——检查 2 模板绑定空洞。修复：签发器常量改为**直接引用
  `orz_assurance::orientation::checkpoint::ORIENTATION_BLOCK`**（单一来源，零复制）；
  `template_sha256_matches_the_injected_orientation_block` 单测逐字节钉死两端
- **C1-1（符合）配对规则零测试覆盖**：`_verify_v02_control_tickets` 在测试套件中是死代码。
  修复：`ControlTicketPairingTests` 6 测试（正例 issued→consumed / 未知 ticket /
  consume 先于 issued / kind 不一致 / one-shot 终态互斥 / rejected 合法）
- **C1-2（符合）审计假增量**："+80 新测试"实为 fixture 文件数、"含新配对规则单测"不实。
  修复：措辞改为真实（189 = 183 + 6；配对规则测试列名）
- **C2-4（符合）schema 漏约束**：非 orientation 的 activation null / template 非 null 通过
  schema（Rust issue 拒之）。修复：allOf 改 if/then/else 镜像（orientation → template 非空+
  activation null；其余 → template null + activation 非空）；fixture 坏例保持单违反

**应修（P1/D2，互证）**：
- **P1-1（实现）协议失步**：call 超时/错位后响应流永久错位。修复：`call_raw` + 失败时
  `respawn`（kill+spawn+重放确定性 initialize 恢复 K_session）；Signer 错误（健康往返）不
  respawn；e2e 新增 `signer_crash_respawns_and_recovers` 验证自愈 + 序列单调延续
- **D2-1/P1-2（设计+实现互证）ledger epoch 陈旧**：goal/policy 重派生后 signer 序列归零、
  host ledger 高水位不重置 → 新票全拒 replay。修复：`TicketLedger::reset(session_id)` +
  `ensure_initialized` 上下文变化时 reset + 单测 `ledger_reset_clears_epoch_high_water_mark`
- **D2-2/P2-1（设计+实现互证）activation 自指**：验票上下文用票面 activation 自比恒等。
  修复：`verify_and_consume` 增 `live_activation_id` 参数（消费点真实上下文，controller
  传 `act.activation_id`）

**P2（建议修）**：
- P2-2 错误类型滥用：base64url_decode → 专用 `InvalidSignature` 变体
- P2-3 非 ASCII session id 字节切片 panic：两处改 `chars().take(8)`
- P2-4 模块文档 check 7 措辞：诚实标注落地方式（journal 链 + verifier 配对规则；票级
  receipt 链 = v2 升级路径）
- P2-5 e2e 断言加强：consumed 全部配对 issued + 交错顺序断言
- P2-6 未知 ticket_kind 占位 orientation_v1 → `Protocol` 错误
- P2-7 `canonical_arguments_digest` unwrap_or_default 吞错 → expect（canonical_json 对
  serde_json::Value 不可失败）

**C2/C3 轻修**：删 verifier 死代码重复 `return errors`；引用精度（ADR-0011 §4.2/§4.6
→ 设计文档 §4.2/§4.6，机制层归属；ADR-0011 §2.1 为票据绑定）；测试名 39→42；索引
"最近整理"日期更新。

**D3/C3 登记项（不修，回填）**：
- 验收证据拆分：四接线点代码级 4/4 + e2e 路径级 2/4（orientation 需 7 轮、goal revision
  需 continue——代码存在 + 单测覆盖签发，controller 路径 e2e 留后续）
- 票结构省略三字段（resolved_target_sha256 / previous_receipt_sha256 /
  user_confirmation_sha256）：Slice 1 控制类由 journal 链兜底可辩护；**Slice 2 动作票据
  必须补 resolved_target_sha256**
- goal revision 票绑 run-goal 上下文（非被修订的 next_goal）；K_session 重派生留 Slice 2
- 影子差异台账：生产配置 run 的 capture 重捕为可选未做
- 错误映射精度：客户端错误统一 signer_unreachable（审计事件 reject_code 失真，当前路径
  不可达）；未知 kind 已修（P2-6）
- 签发器 stderr 丢弃（Stdio::null）——故障诊断仅 "signer exited"，留 trace 转发后续
- e2e MANIFEST 常量非 64hex（测试脚手架产物；e2e journal 不参与 envelope schema 校验，
  配对规则验证走合成 journal）
- chain_mismatch 枚举值无 Rust 产出路径（schema 超集，保持）
- HKDF info 字节布局需登记为权威（Python 权威侧若实现须逐字节一致）
- AcafClient 无 Drop kill——依赖 stdin EOF；父进程硬杀会孤儿 signer
- hex_decode 未校验 32 字节键长（ring 接受任意长度键）
- §5.6 子代理 activation/assessment 持票属 Slice 2+ 接线面

**复验（修复后全量）**：orz-assurance 107/0（+1 ledger reset）、orz-loop 184/0/3、
orz-bin 6+6+4+2+1（**e2e 4 测试**：全生命周期 / controller 全链 / shadow / 自愈）、
orz-tui 177/0、clippy 四 crate 0 warning、pytest runtime **189** + assurance 1606/14、
conformance 109、check_repository valid、capture 13/13、fixtures 重生成一致、
diff --check 干净。
