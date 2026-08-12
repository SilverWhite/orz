# GAP-ACAF-GOAL-POLICY-WIRING 实施审计（2026-08-12）

> 状态：**两挂起项已实施并验证**。范围：① 目标解析单源化（`orz-paths` 共享家，
> 用户裁决）② goal_version/policy_revision 接线（GAP-DENIAL-POLICY-REVISION
> 消费面，用户裁决保持 HKDF LE 占位、只接机制）。设计权威回查
> [`ADR-0011`](../../adr/ADR-0011-authenticated-control-and-action-fabric.md) 与
> [`AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md`](../../docs/AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md)。
> 用户裁决（2026-08-12）：① 依赖方向 = orz-paths 共享家（唯一无环候选——
> orz-shared→orz-tools 边排除 orz-shared，orz-assurance 必须保持 tool-free）；
> ② policy 输入保持 8 字节 LE `policy_revision` 占位（不切换真 policy_digest），
> 只接机制——goal_version 真实接线 + policy_revision live 穿透 + bump 机制，
> 首个生产递增来源 = Slice 3 ModeChangeTicket（登记）。

## 1. 摘要

**挂起项 1（单源化）**：解析原语（`sanitize_model_path_arg` / `tilde_expand` /
`tilde_expand_strict` / `resolve_lexical` / `is_reparse_or_symlink`）迁入
`orz-paths::resolve`（新模块，shellexpand 3.1 同 spec 依赖）；`orz-tools`
`resolve_model_path` 薄壳化（行为逐字节不变，~95 测试为安全网）；`orz-assurance`
`acaf/target.rs` 公开 API 与错误枚举零改动、私有原语委托（`~user` 拒绝映射、
折叠改用 `orz_paths::normalize_lexically`、reparse 检查薄壳）；`orz-host`
lib.rs:482 本地 `is_reparse_or_symlink` 删除改调 orz-paths。范围外登记：
normalize_lexical 剩余 2 拷贝（permission.rs / orz-config）、
`resolved_target_digest` 留 orz-assurance。

**挂起项 2（goal/policy 接线）**：`GoalContext { digest, version }` 替换
`Mutex<Option<String>> goal_digest`（成对快照）；`policy_revision: AtomicU64`
live 值（per-run 重置 0，与 denial_state 同域）；`update_goal` / `bump_policy_revision`
/ `policy_revision()` 方法；5 个接线点改读 live 值（ticket_flow 与 acaf_action_event
的 ensure_initialized、controller 与 agent_loop 的 DenialKey ×2，DenialKey 字段
u32→u64）；**AcceptedContinue 在 GoalRevisionV1 票消费后更新 goal 绑定**
（digest 换新 + version 0→1）→ 下一票 ensure_initialized 重派生 K_session +
ledger 重置 → 旧未消费票死（ADR-0011 决策 5，Slice 1 审计 D5 边界闭合）；
熔断聚合抽取纯函数 `aggregate_denial_round`（行为不变，供测试直达）。

## 2. 交付清单

| 层 | 文件 | 内容 |
|---|---|---|
| 单源化 | `orz-paths/src/resolve.rs`（新） | 五原语 + `TildeExpandError`；crate 21 单测（含新模块 16 个 + lib.rs 既有，join 分隔符字节锁定、Windows/Unix cfg 分叉） |
| 单源化 | `orz-paths/Cargo.toml` | +shellexpand = "3.1"（同 orz-tools spec，lockfile 图内零新条目） |
| 单源化 | `orz-tools/src/types/resources.rs` | `resolve_model_path` → 三行薄壳；删本地 sanitize；40 个 `resolve_model_path_*` 测试（变更后 L1213-1606）原样全绿 |
| 单源化 | `orz-tools/Cargo.toml` | +orz-paths（workspace 依赖声明已有） |
| 单源化 | `orz-assurance/src/acaf/target.rs` | 模块 doc 重写（镜像→单源表述）；私有委托（sanitize/strict-tilde/fold/reparse 薄壳）；`reject_verbatim` 保留；+1 测试 `fold_drive_relative_preserves_parent_prefix` |
| 单源化 | `orz-assurance/Cargo.toml` | +orz-paths（首个内部依赖，无环） |
| 单源化 | `orz-host/src/lib.rs` | 删本地 `is_reparse_or_symlink`（L482-497），walk 改调 orz-paths（丢 `ft` 参数，`ft` 绑定保留供 is_dir/is_file） |
| 接线 | `orz-loop/src/controller.rs` | `GoalContext` 结构 + `goal_context` 字段；`policy_revision: AtomicU64`；`set_goal_digest` 重置 version / `goal_digest_of` / `update_goal` / `bump_policy_revision` / `policy_revision()`；ticket_flow 与 acaf_action_event 成对快照 + live 值；per-run 重置；DenialKey live 值；AcceptedContinue goal 更新插入（票消费后）；+2 测试 |
| 接线 | `orz-loop/src/agent_loop.rs` | `SharedLoopServices.policy_revision`；`role_gate_denied` +policy_revision 参数（3 调用点）；聚合块抽取 `aggregate_denial_round` 纯函数（行为不变）；+3 测试 |
| 接线 | `orz-bin/tests/acaf_e2e.rs` | +1 e2e `goal_revision_continue_flow_re_derives_session_key`（Windows，真实签发器） |
| 修复批 | `orz-assurance/src/acaf/target.rs` | **D2-1**（三面审查）：parse_target 镜像 "forgot leading slash" 恢复分支（`/{input}` 与 worktree 前缀比对 strip——幻影绑定类逃过自检出，工具侧同输入解析到真实路径）+ 单测 `forgot_leading_slash_recovery_binds_real_object`；**P3-1**：`Component` import 收窄 cfg(windows)（Linux 构建 unused 警告） |

## 3. 设计决策与登记

| # | 决策 | 依据 / 登记 |
|---|---|---|
| D1 | 单源化家 = orz-paths（codegen，仅 camino+serde+thiserror，零内部依赖）；orz-tools/orz-assurance/orz-host 三方依赖，无环 | orz-shared 被 orz-shared→orz-tools 边排除；orz-assurance 保持 tool-free（orz-loop→orz-assurance 不拖工具栈）；orz-paths 已被 orz-host/orz-workspace 使用，定位吻合 |
| D2 | 双入口保留：`tilde_expand`（宽松，shellexpand 语义）+ `tilde_expand_strict`（票据侧，`~user` 拒绝，**PathBuf join 语义而非字符串拼接**——字节级差异：Windows 下 `C:\Users\test\proj/a.rs` 混合分隔符，正是票据摘要覆盖的字节，测试锁定） | 工具侧宽松行为是 10 个调用点的既有契约（permission manager 2 处）；票据侧 fail-closed 是有意差异（P1-2） |
| D3 | 折叠委托 `orz_paths::normalize_lexically`——**行为变化登记**：驱动器相对 `C:..\x` 折叠结果由旧 `C:x`（弹前缀）变为 `C:..\x`（保留 `..`，前缀保护），更保守方向；有根路径两实现同结果（`Path::pop` 不弹根，实测 `C:\worktree\..\..\secret` 两侧均 `C:\secret`）；D1-1 未折叠候选 reparse 扫描在折叠前，无安全回归 | orz-paths 既有 Windows 测试（lib.rs L421-424）+ target.rs 新测试 `fold_drive_relative_preserves_parent_prefix` 锁定新语义 |
| D4 | `is_reparse_or_symlink` 单实现（symlink_metadata + 0x400 位）；orz-host 消费点丢 `ft` 参数（Unix 每目录项多一次 symlink_metadata——语义等价，性能毫秒级） | 两拷贝语义等价（read_dir entry type 与 symlink_metadata 同为不跟随），合并零行为面 |
| D5 | `GoalContext` 单 Mutex 成对快照（digest+version） | 读写点均需成对取值/写入（ticket 绑定一致性）；Mutex 先例 = denial_state |
| D6 | `policy_revision: AtomicU64`（先例 pacing_rounds AtomicU32）；per-run 重置 0（与 denial_state 同点）；DenialKey 字段 u32→u64 与 ACAF 对齐 | bump/read 是单值无状态操作，无需共享锁域；DenialKey 仅 PartialEq 无序列化，影响面 2 生产构造点（+1 测试辅助 `denial_key`） |
| D7 | goal 更新插入点 = AcceptedContinue 分支 `acaf_control_event(GoalRevisionV1).await?` **之后**（当前 L4130-4140：acaf_control_event 调用与 update_goal 之间） | 票生命周期（ensure_initialized→sign→issued→verify_and_consume）全用旧缓存——票授权转变本身，消费后才切换绑定；下一票才重派生。acaf 未配置时 update 无害照常执行 |
| D8 | continue 的 requirement_delta 非空由既有校验结构性保证；delta 成为新 run 级 goal 绑定（会话级 K_session 重派生，旧未消费票全死——影子模式 journal rejected，fail-closed 后真拒绝） | ADR-0011 决策 5 明文；Slice 1 D5 边界闭合 |
| D9 | `bump_policy_revision` 机制就位、**无生产调用方**（dead_code allow + 注释登记）——首个生产递增来源 = Slice 3 ModeChangeTicket（ADR-0011 决策 9） | GAP 定义是"结构上不可触发"——现机制可达（测试证明），来源随 Slice 3 交付；索引措辞如实 |
| D10 | 零 Schema/事件变更（goal_version/goal_digest/policy_revision 已在 issued payload；无新事件类型） | fixtures/verifier/capture 零影响 |
| D11 | e2e 断言 ledger epoch：重派生后 sequence 自 1 重启（`[1,2,1,2]`） | Slice 1 D2-1 语义（per K_session epoch），实测通过 |
| D12 | **D2-1（三面审查修复）**：parse_target 非 rooted 分支镜像 `resolve_lexical` 的 "forgot leading slash" 恢复（`/{input}` 与 worktree 前缀比对 strip）——此前相对拼写重复 worktree 组件时票据绑定**双倍幻影路径**，两侧自洽 → consumed，target_mismatch 自检出对该类永久失明（工具侧同输入解析到真实对象）；修复后票据绑定真实对象，与工具侧同源语义 | 预存在差异（旧 parse_target 同 body），本批"不再有双实现漂移面"结论因此过强——审查指出后修复；镜像恢复后与工具侧行为一致（除登记差异外） |

## 4. 验证证据

- **单源化回归网（行为锁定）**：orz-paths **21/0**（crate 总数，含新模块）；
  orz-tools **2728/0/6**（40 个 `resolve_model_path_*` 测试原样全绿——薄壳逐字节
  等价）；orz-assurance **124/0**（+2：fold 语义测试 + D2-1 恢复分支测试）；
  orz-host **198/0/4**（delta walk reparse 跳过回归）
- **接线**：orz-loop **193/0/3**（+5：aggregate 三测试 + goal_context 测试 +
  policy_revision bump/重置测试）；clippy 六 crate（orz-paths/orz-tools/
  orz-assurance/orz-host/orz-loop/orz-bin）**0 warning**
- **e2e（真实 DPAPI 签发器，Windows）**：`goal_revision_continue_flow_re_derives_session_key`
  —— 4 票（disposition-continue / goal_revision_v1 / disposition-close / close_v1）
  issued+consumed 配对、rejected 空；旧上下文票 goal_version==0 + run 起始 goal
  digest；GoalRevisionV1 票带 activation 绑定；重派生后票 goal_version==1 +
  continue delta digest；sequence `[1,2,1,2]`（epoch 重启）；consumed 均后于 issued
- **orz-bin 全量**：main 6、signer 9、e2e **7**（6→7）、real_flag 2、stdio_e2e 1、
  capture **13/13**（acaf env 门控零影响）
- **Python**：pytest runtime **196 passed** + assurance **1606 passed/14 skipped**
  （基线一致）；check_repository **valid**；git diff --check 干净

## 5. 边界登记

- **bump_policy_revision 无生产递增来源**（Slice 3 ModeChangeTicket 交付前）：
  §3.5.4 "policy revision 变化重置"路径现**结构上可触发**（bump 机制 + 单测证明），
  但生产端到端触发随 Slice 3；
- **normalize_lexical 剩余拷贝**：orz-host permission.rs:297、orz-config
  managed_text/source.rs:298（本次后 target.rs 拷贝消灭，剩 2 处 + orz-paths
  唯一权威）——单源化需行为等价测试先行（RootDir pop 保护、空串返回 "."、
  驱动器相对保留），留独立批次；
- **`resolved_target_digest` 留 orz-assurance**（票据专属统一斜杠 sha256，不引
  sha2 入 orz-paths）；
- **goal 重派生影响检索子代理会话**：continue 是子代理 activation 的转变，但更新
  的是主 run 级 goal 绑定（同一 session_id 的 K_session）——重派生作废该会话全部
  未消费票（含其他 activation 的），影子模式仅 journal rejected；设计意图（goal
  是会话级绑定），登记；
- **per-run 重置 vs 会话级绑定的跨 run 盲点（D3-1，Slice 3 gate）**：
  `policy_revision` per-run 归零与 K_session 的**会话级**绑定存在张力——同一 ACP
  会话跨 run 时，Slice 3 模式切换的 bump 会在下一 run 起点被抹平，决策 9 "作废
  全部未消费票据"跨 run 边界只剩 TTL 5s 兜底。对 denial breaker 消费者完全一致
  （熔断窗口本就是 run 级）。当前无实际后果（无生产 bump 来源；验票只发生在新签
  发票上；票签发后毫秒级消费；ledger 随 controller 重建）。**Slice 3 接线 gate**：
  计数器需会话级携带（sidecar，与 goal 重钉同模式）或显式接受 TTL 界失效；
- **fail-closed 翻转 gate（D3-2）**：影子模式下 rejected 的 GoalRevisionV1 也照常
  翻转 goal 绑定（与影子语义一致）；翻转时必须把 `update_goal`（及整个
  AcceptedContinue 状态迁移）gate 在票 consumed 之上——已补入 SLICE2A §6 核查清单
  第 ⑥ 项；
- **e2e Windows-only**（DPAPI）；Linux 侧由 orz-signer 单测
  `goal_revision_ticket_binds_old_context`（key 重派生）+ orz-loop 纯函数测试覆盖；
- **target.rs 与 resolve_model_path 残余差异**（有意保留）：verbatim 拒绝 /
  `..` 折叠（scan-before-fold）/ `~user` 拒绝 / 单基 worktree——现均为"同源原语 +
  票据侧编排差异"，不再有双实现漂移面；残差分歧仍由验票 target_mismatch 影子可见；
- **shellexpand 保留于 orz-tools**（另有 2 处使用：resources.rs:1564 测试、
  grok_build/task/types.rs:198 生产）；
- `cargo fmt --check` 全仓 pre-existing 漂移（orz-agent/builder.rs 等）非本次
  改动，不修（沿用先例）。

## 6. 下一步

- **Slice 2 完整闭合**（前置已全部就位——policy_revision 接线本批完成）：
  command_exec（run_tests ExecutionPermit + argv 规范化）与 network
  （web_fetch/browser_read URL 目标解析）各作一步复制 file_write 同型扩展；
  之后用户裁决 fail-closed 切换（探针矩阵 + 影子台账零误阻断 + §6 核查清单）；
- **Slice 3**：ModeChangeTicket 模式切换 → `bump_policy_revision` 首个生产调用方
  + policy_digest 真摘要切换（全量重派生一次性事件，登记边界）+ 会话级计数器
  gate（D3-1）；
- 可选：normalize_lexical 剩余拷贝单源化（行为等价测试先行）。

## 7. 三面审查闭环（2026-08-12，用户发起"全面检查"）

设计/实现/符合性三独立代理审查——**无 P0/P1/D1/C1 级未修项**；修复批已应用并复验：

**应修（D2-1）**：
- **D2-1（设计）`parse_target` 未镜像 "forgot leading slash" 恢复分支**：相对拼写
  重复 worktree 组件（`data/user/...` vs worktree `/data/user/...`）时工具侧解析到
  真实对象、票据侧 join 成双倍幻影——两侧自洽 → consumed，target_mismatch 自检出
  对该类永久失明（预存在差异，但本批"不再有双实现漂移面"结论过强，审查指出）。
  修复：parse_target 非 rooted 分支前镜像 `/{input}` 与 worktree 前缀比对 strip
  （`resolve_lexical` 同源语义）+ 单测
  `forgot_leading_slash_recovery_binds_real_object`（双平台组件级比较）
- **P3-1（实现）`Component` import 跨平台警告**：收窄 `#[cfg(windows)]`（Linux
  构建下仅 reject_verbatim 使用）

**C2（符合性，措辞/数字/引用精度）**：
- C2-1 "~95 测试" → "40 个 `resolve_model_path_*` 测试（变更后 L1213-1606）"
- C2-2 ADR-0011 头部 + resolve.rs 注释 "§4.5" → "设计文档 §4.5"（ADR-0011 无 §4.5）
- C2-3 索引 SLICE2A 条目 "词法解析语义镜像" → "解析原语同源 orz-paths（票据侧
  保留编排差异）"
- C2-4 审计 D7 行号 L4041-4042（HEAD）→ L4130-4140（当前）

**登记项（D3/C3/P3）**：D3-1 per-run 重置 vs 会话级绑定跨 run 盲点（Slice 3 gate，
§5 登记）；D3-2 fail-closed 翻转时 `update_goal` 须 gate 票 consumed（SLICE2A §6
核查清单第 ⑥ 项）；D3-3 模型可经 continue 触发会话级重派生（只毁票不授票，安全
方向正确）；D3-4 空串 delta 结构性拒绝（controller L3816-3833 核实）；P3-2 混合
key round 补测建议（分支已覆盖）；P3-3 e2e policy_revision>0 留 Slice 3（单测已
覆盖）；P3-4 GoalContext Clone derive 冗余（登记）；C3-1 DenialKey 影响面补
"+1 测试辅助"；C3-2 设计文档 §7 反引号路径相对化；C3-3 "21 单测"归属精化；
C3-4 IMPL-CONTROL-FABRIC 入口补新审计。

**复验（修复后）**：orz-assurance **124/0**（+1 D2-1 测试）、clippy orz-assurance
**0 warning**（含 cfg(windows) import 修复）、其余 crate 数字不变（本批未动）、
check_repository valid、diff --check 干净。
