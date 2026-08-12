# GAP-ACAF-SLICE2B 实施审计（2026-08-12）

> 状态：**Slice 2 完整闭合已实施并验证**（ADR-0011 §2 决策 12 / 设计文档 §7
> Slice 2——`command_exec_v1` 与 `network_v1` 两类动作票据补齐，四类动作票就位
> （file_write 全链、credential_read 机制先建零接线、command_exec/network
> 同型扩展）；**全程影子模式，fail-closed 未切换**）。
> 范围：[`GAP_ACAF_SLICE2A_IMPL_AUDIT`](./GAP_ACAF_SLICE2A_IMPL_AUDIT_2026-08-12.md)
> §7 下一步第一项（“command_exec（run_tests ExecutionPermit + shell argv 规范化）
> 与 network（web_fetch/browser_read URL 目标解析——check_ssrf 已存在）各作一步
> 复制 file_write 同型扩展”）。设计权威回查
> [`ADR-0011`](../../adr/ADR-0011-authenticated-control-and-action-fabric.md) 与
> [`AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md`](../../docs/AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md)。
> 本批为单代理实施 + 自审 + **三面独立代理审查闭环（2026-08-12，用户发起“全面
> 检查”）**：无 P0/D1 级未修项；审查修复批见 §8。

## 1. 摘要

Slice 2 剩余两类动作票据补齐：`TicketKind` 新增 `command_exec_v1` /
`network_v1`（枚举 6→8，`capability_scope` 6→8）；目标解析新增
`resolve_network_url` / `network_target_digest`（canonical URL 绑定，
`orz-assurance` 首次引入 `url` crate）；host 客户端新增
`command_exec_canonical_args` / `command_exec_target_digest` /
`command_env_sha256` / `network_canonical_args`；`action_kind_for_tool`
单点映射扩展（run_tests / run_terminal_cmd → command_exec；web_fetch /
browser_read → network；**web_search 有意不映射并登记**，§3 D2）；签发器新增
`sign_command_exec_v1` / `sign_network_v1` 两个枚举化 method；controller 新增
network / command 事件路径与共享票据生命周期 `run_action_ticket`，并在
`run_tests` 分支的 ToolStarted 之前插入 command_exec 票据。Schema 先行
（ticket_kind/capability_scope 枚举 +2 + issued allOf 映射/激活/目标约束扩展），
fixture README 同步，Python 与 Rust 测试、Windows 真实签发器 e2e 全部补齐。
零新事件类型、零 Schema 枚举计数变化（事件类型仍 42）。三面审查修复批追加：
network 分支无关 store 门禁删除（P1-1）、network/command live 重派生表述修正
（D1-1）、env 摘要分隔符改 NUL（P2-1）、e2e canonical 精确断言与两个影子路径
测试（P2-2/P2-3）、文档计数与措辞修正（C1-1/C2-1/C2-2/C2-3）。

## 2. 交付清单

| 层 | 文件 | 内容 |
|---|---|---|
| Schema | `runtime/control-ticket-issued-event-payload-v0.2.schema.json` | `ticket_kind`/`capability_scope` 枚举 6→8；allOf 增 command_exec/network 能力映射 ×2、动作类 activation null 枚举 +2、动作类 resolved_target required 枚举 +2 |
| Schema | `runtime/control-ticket-consumed/-rejected-event-payload-v0.2.schema.json` | `ticket_kind` 枚举 +2 |
| Fixture | `scripts/generate_run_event_fixtures.py` | README_V02 文字同步（Slice 2 四动作类）；重生成仅 README diff |
| 测试 | `runtime/tests/test_run_event_journal_validation.py` | `_issued_ticket` 能力映射 +2；ActionTicketSchemaTests +2 有效例；ControlTicketPairingTests +1 command_exec 配对；`test_unknown_kind_rejected` 改用 mode_change_v1（原 command_exec 不再未知） |
| 核心 | `orz-assurance/Cargo.toml` | +`url = { workspace = true }` |
| 核心 | `orz-assurance/src/acaf/mod.rs` | `TicketKind` +2 变体 + `as_str`/`capability_scope`/`requires_target`；`parse_kind` +2；测试 `all_six_kinds` → `all_eight_kinds` |
| 核心 | `orz-assurance/src/acaf/target.rs` | 新错误变体 `InvalidUrl`/`UnsupportedScheme`/`UrlCredentialsUnsupported`；`resolve_network_url`（trim、url 解析、http/https 限定、userinfo 拒绝、fragment 去除、默认端口去除）；`network_target_digest`；单测 +2 |
| 签发器 | `orz-bin/src/bin/orz-signer.rs` | method 分派 +2（`sign_command_exec_v1`/`sign_network_v1`，target 参数必取）；方法表注释；单测 +2 |
| 客户端 | `orz-loop/src/acaf.rs` | `sign_ticket` method map +2；`verify_and_consume` kind 解析 +2；`action_kind_for_tool` +4 映射（web_search 有意 None）；新 canonical 助手 4 个；单测 +2 |
| 接线 | `orz-loop/src/controller.rs` | `acaf_action_event` 新分支派发（network/command）；新 `acaf_network_event` / `acaf_command_event` / `acaf_command_exec_event` / 共享 `run_action_ticket`；`run_tests` 分支 ToolStarted 前插票；**`acaf.is_none()` 门禁前移至新分支派发前（§8 D1-1 修复）**；回归单测 +1 |
| E2E | `orz-bin/tests/acaf_e2e.rs`（Windows） | `network_ticket_full_chain`（browser_read 真实签发器全链）、`run_terminal_cmd_command_ticket_full_chain`、`run_tests_command_ticket_full_chain`、`invalid_network_url_shadow_records_rejection_and_proceeds`（null ticket_id + target_mismatch 且工具照常执行）；TestHost 增 test_runner/run_tests 实现 |

## 3. 设计决策与登记

| # | 决策 | 依据 / 登记 |
|---|---|---|
| D1 | `command_exec_v1` 范围 = `run_tests`（宿主固定命令，模型无 argv）+ `run_terminal_cmd`（模型 shell 命令） | Slice 2A §7 明确“run_tests ExecutionPermit + shell argv 规范化”；生产工具面核实：GrokBuild `run_terminal_cmd` 已启用（`enabled_background=false`），`bash` 是未启用的 OpenCode 命名空间 |
| D2 | `network_v1` 范围 = `web_fetch` / `browser_read`（URL 目标）；**`web_search` 有意不映射** | web_search 无 URL 目标（query 送配置自有 DeepSeek 服务端搜索端点），绑定面未定型；登记为 fail-closed 翻转前核查清单新增项（§6 ⑦）——不能静默消失 |
| D3 | command canonical 形态 = `{"tool", "argv", "cwd", "env_sha256"}`；target digest = 无 tool 包装的 argv/cwd/env 三元素对象摘要 | 设计 §3.3“命令绑定规范化 argv/cwd/env 摘要”——命令的“解析后真实对象”就是该三元素；与 file 的路径目标同构（canonical 参数含 tool，target 不含，两摘要不同但同源重派生） |
| D4 | `run_terminal_cmd` env 绑定 = 空列表摘要；`run_tests` env_sha256 只覆盖 `TestRunner::env`（平台 allowlist 排除） | 终端后端进程 env 宿主所有、模型不可控；run_tests 平台 allowlist 是 host 进程稳定面（`TEST_ENV_ALLOWLIST` 在 orz-host 私有，orz-loop 不重复实现）——登记边界，fail-closed 翻转前复核 |
| D5 | `run_tests` 的 cwd = `snapshot_store.worktree()` | 生产上 == host 会话 cwd（ACF 会话构造同一 cwd）；简化登记，与 file_write 单基一致 |
| D6 | URL 规范化规则：绝对 http(s)；scheme/host 由解析器小写；显式默认端口去除；fragment 去除；userinfo（凭据）拒绝 | URL gate（check_ssrf/check_navigation_url）已拒绝凭据与危险 scheme——票据侧不重复实现 SSRF，只绑“将被请求的 canonical URL”；凭证 URL 不可票化（与 `~user` 同 fail-closed 方向） |
| D7 | 检索车道 web_fetch 的 activation 绑定保持 null（沿用 Slice 2A D2） | 主车道 web_fetch 由 relay 派发至 external retrieval 子代理；子代理与主 agent 共享同一 controller/acaf（GAP-SUBAGENT-RUNTIME），动作票 activation 仍 null——子代理车道网络动作的 activation 绑定面登记（§6 ⑧，Slice 1 审计 §5.6 登记面延续）；e2e 用 host 车道 browser_read 覆盖 network 全链 |
| D8 | 共享生命周期 `run_action_ticket`：sign→issued→live 重派生→verify→consumed/rejected；file_write 既有路径零改动 | 新分支与既有 file 路径同语义；避免重构已验证路径引入回归 |
| D9 | 签发器保持纯签名预言机形态（沿用 Slice 2A D3） | command/network 同 file_write：客户端传 canonical + resolved digest，TOCTOU 全在验票端 |
| D10 | ExecutionPermit 术语映射登记：本批 `command_exec_v1` 是 run_tests/run_terminal_cmd 的**授权面票据**；设计文档 §4.5 的独立 `ExecutionPermit`（每次 CreateProcess 一次性票、执行 broker 强制）属 Slice 3 | 三面审查 D3-1（2026-08-12）：避免把 Slice 2 票误读为 broker 级执行强制；fail-closed 翻转裁决时确认定义映射 |

## 4. 验票七项检查映射（Slice 2 全量）

| 检查 | 实现（Slice 2 全量） |
|---|---|
| 1 签名有效 | 不变（resolved_target_sha256 在 canonical body 内） |
| 2 模板版本 | 不变（动作类恒 null） |
| 3 单调+一次性 | 不变（TicketLedger） |
| 4 上下文绑定 | 不变（动作类 activation 恒 null，D2/D7） |
| 5 真实对象摘要 | canonical_arguments 比对不变；**5b**：四类动作票三形态比对（票面 None / live None / 不等）+ 控制类带 target 拒绝，均 `TargetMismatch` |
| 6 未过期 | 不变（5s TTL） |
| 7 链一致性 | 不变（journal previous_event_sha256 + Python 配对规则） |

## 5. 验证证据

- **Rust**：orz-assurance **127/0**（124 基线 + URL 规范化/摘要 2 测试 +
  审查修复批 `all_action_kinds_reject_live_target_mismatch`；
  `all_eight_kinds` 覆盖 8 变体）；orz-loop **196/0/3**（195 基线 + 1
  network/command 未配置零事件回归单测）；orz-bin：main **6**、signer **11**
  （9+2）、e2e **13**（7+4+审查修复批 2）、real_flag **2**、stdio_e2e **1**；
  clippy orz-assurance/orz-loop/orz-bin **0 warning**
- **E2E（真实进程，Windows DPAPI）**：
  - `network_ticket_full_chain`：browser_read（local_browser 模式）——
    issued=1（network_v1/capability=network/activation=null/
    resolved_target_sha256 64hex）+ consumed=1 配对、rejected=0、
    票据事件先于 ToolStarted
  - `run_terminal_cmd_command_ticket_full_chain`：command_exec_v1 全链
    （argv/cwd/env 目标绑定、issued→consumed、先于 ToolStarted）
  - `run_tests_command_ticket_full_chain`：host 固定命令 `python -m pytest` +
    `PYTHONPATH=src` env——command_exec_v1 全链 + ToolStarted 仍携带
    `fixed_command`
  - `invalid_network_url_shadow_records_rejection_and_proceeds`：
    `not a url` → rejected(network_v1, target_mismatch, **ticket_id=null**)
    且工具照常执行（影子台账）
  - `run_terminal_cmd_empty_command_shadow_records_rejection_and_proceeds`：
    空命令 → rejected(command_exec_v1, target_mismatch, ticket_id=null)
    且工具照常执行（审查修复批 P2-3）
  - `missing_network_arg_silently_skips_with_configured_acaf`：缺 url 键 →
    零票据事件（影子模式登记行为锁定，审查修复批 P2-3）
  - 精确 canonical 断言：network 绑定
    `network_target_digest("http://example.com/docs/guide?q=1")`（非原始
    拼写），command/run_tests 绑定 argv/cwd/env 三元素摘要，run_tests
    consumed outcome=accepted（审查修复批 P2-2）
  - 既有 7 项全绿
- **Python**：runtime **199 passed**（196 基线 + 3 新测试）、assurance
  **1606 passed/14 skipped**（基线一致）、capture **8/8**（本机收集 8 项；
  先例审计“13/13”为场景计数口径，本批未改 capture 路径）、
  check_repository **valid**、fixtures 重生成一致（仅 README diff）
- **git**：主仓 **10 文件**（9 修改：索引/ADR/设计文档 + Schema 3 + fixture
  README + Python 测试 + 生成器；1 新增审计），orz 仓 8 文件（Cargo.lock 1 +
  assurance 3 + loop 2 + bin 2 含 e2e）；
  `git diff --check` 干净

## 6. 边界登记

- **影子模式是验收语义**：验票失败照常执行；fail-closed 切换 = 完整 Slice 2
  里程碑（ADR-0011 决策 11），用户观察台账后另行裁决；
- **fail-closed 翻转前置核查清单**（Slice 2A §6 ①-⑥ 全部延续 + 本批新增）：
  ① 工具集变更重审 `action_kind_for_tool` 覆盖（“应签未签”面）；② 工具自身
  校验拒绝仍记 consumed “accepted”；③ `file_path`/`url`/`command` 键缺失
  静默跳过——翻转时转硬拒绝；④ acaf 未配置 / snapshot_store 缺失 /
  goal_digest 缺失静默 skip——翻转时逐项转硬拒绝或显式告警；⑤ capture 无
  acaf 场景；⑥ rejected GoalRevisionV1 也照常翻转 goal 绑定——翻转时必须
  gate 在 consumed 之上；**⑦ `web_search` 未映射（网络效果但无 URL 目标，
  服务端端点配置自有；注意它是文档登记差异，不产生任何影子台账事件）——翻转前
  需用户裁决其票化形态或显式排除；**⑧ 检索车道
  web_fetch 经子代理执行但动作票 activation 恒 null——翻转前需裁决子代理
  网络动作的 activation 绑定；⑨ **host 侧执行参数绑定面**：除 env
  （run_terminal_cmd 空 env、run_tests 平台 allowlist 未入 env_sha）外，
  shell 后端、`BashParams.cmd_prefix`、`SessionEnv` 资源均未入绑定（host
  稳定面）——翻转前复核是否需补绑定；⑩ **执行面与票据绑定面错位**：票据绑
  canonical 对象（URL 规范化 / argv-cwd-env 三元组），工具执行仍用原始参数
  字符串——翻转前确认工具端 URL gate 的规范化与 `resolve_network_url`
  等价，或由执行 broker 使用 canonical 形态；⑪ **network 重定向不重新
  票据**：初始 URL 合法、重定向到被禁主机由 URL 门禁逐跳拒绝，但票据 ledger
  只记一次 consumed——翻转前探针矩阵必须覆盖该场景；
- **verify→execute 残余窗口**：file_write 的 symlink 换入窗口登记延续；
  network/command 的目标为纯规范对象（URL 字符串 / argv-cwd-env 三元组），
  无 FS 窗口；
- **web_fetch 主车道**：主 agent 调用 web_fetch 时 relay 派发至 external
  retrieval 子代理（web 工具车道），票据在子代理共享 controller 路径签发
  （activation null，D7）；browser_read 为 host 车道直接覆盖；
- **e2e Windows-only**（DPAPI）；Linux 侧由核心/签发器单测覆盖；
- capture 计数口径（本机 8 项 vs 先例 13/13 场景计数）登记，未重捕；
- `cargo fmt --check` 全仓 pre-existing 漂移（orz-agent/builder.rs 等）
  非本次改动，不修（沿用先例）；本批变更文件由 clippy 0 + 编译保证。

## 7. 下一步

- **Slice 2 完整闭合只剩 fail-closed 切换**：用户裁决（探针矩阵 + 影子台账
  零误阻断 + §6 核查清单 ①-⑪ + D10 术语映射确认）；
- 可选：检索车道 web_fetch activation 绑定接线；conformance capture 新增
  票据场景；normalize_lexical 剩余拷贝单源化；
- **Slice 3**：ModeChangeTicket 模式切换 → `bump_policy_revision` 首个生产
  调用方 + policy_digest 真摘要切换 + 会话级计数器 gate（D3-1）；
- **Slice 4**：Windows Sandbox backend（D-11）。

## 8. 自审与三面审查闭环（2026-08-12，用户发起“全面检查”）

设计/实现/符合性三独立代理审查——**无 P0/D1 级未修项**；修复批已应用并复验。

**必须修（D1-1 / P1-1）**：
- **D1-1（实现，自审）network/command 分支在 `acaf.is_none()` 检查之前派发**：
  未配置 ACAF 的 fabric 遇到非法 URL / 空命令会误记 `control_ticket_rejected`
  （破坏“未配置=零行为变化”保证，与 `search_replace` 门禁顺序不一致）。
  修复：门禁前移至新分支派发前 + 两个新方法内防御性检查 + 回归单测
  `network_and_command_with_acaf_disabled_zero_ticket_events`（合法 URL +
  非法 URL + run_terminal_cmd 三调用零票据事件）。
- **P1-1（实现，三面审查）`acaf_network_event` 误用 snapshot_store 门禁 +
  `_worktree` 死代码**：URL 目标不需要 worktree 相对解析；该门禁是 file 分支的
  错误复制，ACAF 已配置而 store 缺失时网络工具将无票执行且零事件——影子台账
  看不见的通道。修复：删除 network 分支的 store 门禁与死代码（command 分支
  绑定 cwd 需要 store，保留合理）。

**应修（D1-1 表述 / P2 / C1 / C2）**：
- **D1-1（表述，三面审查）network/command 的“live 重派生”是确定性重算**：闭包
  重派生用的是签发前捕获的同一份输入，不是重新读取外部状态——check 5b 在这两
  类票上是“不信任票面值的一致性重算”，与 file_write 的 FS 重读 TOCTOU 强度
  不同。修复：`run_action_ticket` / network / command 注释与审计表述修正；
  新增核心单测 `all_action_kinds_reject_live_target_mismatch`（证明验票端
  确实比对 live 值，四类动作票全拒 target_mismatch）。
- **P2-1（实现）`command_env_sha256` 的 `k=v`+`\n` 拼接非严格单射**（env 值含
  换行时会义）：两侧计算一致、无验票漏洞；分隔符改 NUL（env 键值不可能含
  NUL），消除歧义。
- **P2-2（测试）e2e 只断言 64hex 不断言精确值**：补 canonical 期望断言——
  network 绑定 `network_target_digest("http://example.com/docs/guide?q=1")`
  （非原始拼写），run_terminal_cmd / run_tests 绑定 argv/cwd/env 三元素摘要，
  run_tests consumed outcome=accepted。
- **P2-3（测试）空命令 / 缺参影子路径无覆盖**：新增 e2e
  `run_terminal_cmd_empty_command_shadow_records_rejection_and_proceeds` 与
  `missing_network_arg_silently_skips_with_configured_acaf`（锁定登记行为）。
- **C1-1（符合）主仓文件计数**：“主仓 8 文件” → **10 文件（9 修改 + 1 新增
  审计）**；orz 仓构成枚举补 Cargo.lock、e2e 归入 bin。
- **C2-1（符合）索引 §5 ADR 表未同步**：ADR-0011 行补 Slice 2 两阶段状态。
- **C2-2（符合）“四类动作票全接线”措辞过强**：改为“四类动作票就位
  （credential_read 机制先建零接线）”。
- **C2-3（符合）orz 仓构成枚举类别重叠**：e2e 属 bin 目录且漏 Cargo.lock——
  枚举修正。

**登记项（D3/D2，不修回填）**：ExecutionPermit 术语映射（D10）；检索车道
activation 绑定（D7/⑧）；web_search 票化形态且为文档登记差异、无影子事件
（D2/⑦）；host 侧执行参数绑定面扩面（D3/⑨）；执行面与票据绑定面错位（⑩）；
network 重定向不重新票据探针（⑪）；capture 计数口径。

**复验（修复后全量）**：orz-assurance **127/0**、orz-loop **196/0/3**、
orz-bin signer 11 + e2e **13** + main 6 + real_flag 2 + stdio_e2e 1、
clippy 三 crate 0 warning、pytest runtime 199 + assurance 1606/14、
check_repository valid、双仓 `git diff --check` 干净。
