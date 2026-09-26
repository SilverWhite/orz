# 0BS 大狗粮轮执行报告（2026-09-26）

- **轮次**：会话 `RUN-CLI-6ab6b76d`（2026-09-26）；基线 HEAD `a441ff1c4066`（run 起始 worktree 已含未提交改动）。
- **口径**：0bt 全轮并入 0bs（编号保留、执行随 0bs、**闭合随 0bs、计数 56 不变**）；**全轮不做载体重建**（用户令）；本件落树面＋报告，**不提交／不推送／不重建**。
- **本轮执行面**：⑮／⑯／0bt④ 落码＋钉子；auto-mode 半程核验；0bt①②③、⑧-⑫、⑬ 未竟（§6）。
- **相关件**：前轮报告 [`0BS_FRICTION_REMEDIATION_AND_CARRYOVER_2026-09-25.md`](0BS_FRICTION_REMEDIATION_AND_CARRYOVER_2026-09-25.md)（摩擦 m1–m15／同载五件 S1 勘定／移交面）；本件承接其 §4/§6。

## §0 交付摘要

| 件 | 状态 | 锚点 |
|---|---|---|
| ⑮ 审批弹窗否认选项去 Grok（“No, and tell the AI what to do differently”） | ✅ 落码＋钉子 | `orz-workspace/src/permission/prompter.rs` 单源常量（`:19` 一带）＋五构造点一致性测试 |
| ⑯-① transport 错误文案去 Grok（“The AI …”两臂） | ✅ 落码＋钉子 | `orz-sampling-types/src/error.rs` 502–504／520–524 两臂 |
| ⑯-② 模型面身份标签 | 撤出（不改；按 BACKLOG 第二卷 §1.32 定案） | — |
| ⑯-③ MCP OAuth 客户端名（`MCP_OAUTH_CLIENT_NAME` → “AI”） | ✅ 落码 | `orz-mcp/src/oauth.rs:25` 一带 |
| 0bt④ 权限判定来源落账（观测面） | ✅ 落码＋钉子 P1–P5 | orz-loop `host.rs`／orz-host `permission.rs`・`lib.rs`／orz-loop `tool_run.rs`＋父仓 runtime schema |
| auto-mode 半程核验（2026-09-26 用户令落码面） | ✅ 测试面核验 | `orz-host/src/permission.rs` `initial_yolo`；`ask_auto_approves_under_default_yolo` |
| 0bt①/②/③（长单行取用／版本核验旁路／宿主 shell 通道纪律） | 未竟 → §6 | 承 0bb／承 0bj①／承 0bj③＋m9 |
| ⑧-⑫ 检索线五件（S1 已定稿） | 未竟 → §6 | 前轮报告 §4／§6（批序 ⑪→⑩/⑨→⑧→⑫） |
| ⑬ 信任×ACAF（S1 已定稿） | 未竟 → §6 | 批八线条目（BR Web 工作台） |
| 本报告 | ✅ 本件 | `docs/audits/0BS_EXECUTION_AND_FRICTION_2026-09-26.md` |

## §1 依据与范围

- **用户令（并件条款）**：0bt→0bs 全轮并入——“四件（①-④）＋落码五件（⑧-⑫）全部随 0bs 执行；编号保留、闭合随 0bs、计数不变”（沿 0bj→0bp→0bq 并轮先例）；检索面执行口径＝“除 S3 载体重建与 S4 真机复验外全部做完”（S1 定稿＋S2 落码＋钉子一次做完）。
- **本轮（本会话）用户令**：处理 0bs 大项；完成后不必提交／推送／重建，按项目惯例落一份报告文档即可；任务过程中的摩擦项一并记入报告。
- **边界**：全轮不做载体重建（用户令“重建不做”）；S3/S4 待令；本报告只落树面证据与移交面，不推进任何休眠/未启用面。
- **承接**：前轮报告 §4 五件 S1 勘定（⑧-⑫）与 §6 移交面（S4 复验五点、批次序、⑬/⑭ 口径）原样有效；本轮在其上完成 ⑮/⑯/0bt④ 三处 S2 落码与 auto-mode 核验。

## §2 逐件处置

### 2.1 ⑮ 审批弹窗否认选项去 Grok（已落码＋钉子）

- **用户口径**：审批弹窗否认选项“告诉 Grok 该做什么”的映射不合适，换为通用 “AI”。
- **落点＝单一源常量**：`orz/crates/codegen/orz-workspace/src/permission/prompter.rs` 的 `REJECT_ONCE_LABEL`，由**五处构造点**共用（edit／bash-TUI／generic-bash-Web／WebFetch／MCP）；泛 bash 的 `reject-always`（“No, and don’t run bash commands”）不含该名、不动。一处改，Web 与 TUI 同面（ACP `options[].name`，前端无独立副本）。
- **改动**：常量文案 → `No, and tell the AI what to do differently`（附注释记用户令与单一源口径）。
- **钉子**：`reject_once_label_uniform_and_de_groked_across_five_construction_points`——五个构造点逐一断言：选项渲染含常量原文、不含 “Grok”；常量全文唯一（防未来回退重建点名）。
- **证据**：`cargo test -p orz-workspace --lib -- permission::prompter` → **29/29 过**。

### 2.2 ⑯-① transport 用户可见文案去 Grok（已落码＋钉子）

- **落点**：`orz/crates/codegen/orz-sampling-types/src/error.rs` 的 `status_user_message` 两臂：
  - `502..=504` → `The AI is temporarily unavailable. Please try again in a moment. (HTTP {code}).`
  - `520..=524` → `Connection to the AI timed out or was interrupted. Please try again. (HTTP {code}).`
- **钉子**：`status_user_messages_are_de_groked_and_keep_http_code`——两臂逐码断言：含 “AI”、不含 “Grok”、保留 `(HTTP {code})` 后缀（保留既有对外契约形状）。
- **证据**：`cargo test -p orz-sampling-types --lib` → **285/0 过**。
- **边界**：不动 HTTP 码分档与任何重试语义；只改两处文案。

### 2.3 ⑯-② / ⑯-③

- **⑯-②（撤出）**：模型面身份标签（模板/配置层零消费）不改——按 BACKLOG 第二卷 §1.32 定案撤出，本记录留痕。
- **⑯-③（已落码）**：`orz/crates/codegen/orz-mcp/src/oauth.rs` 的 `MCP_OAUTH_CLIENT_NAME` → `"AI"`；注释载明**对端或需重新注册**的已知风险（见 §5 m5）。不动模型 id/name、`GrokBuild:*` 前缀、`GROK_HOME`/`~/.grok`、`xai-*` 等冻结面。

### 2.4 0bt④ 权限判定来源落账（已落码＋钉子 P1–P5；原 0bu 并件）

- **性质＝只加观测面**：不改任何判定结果、阈值、拒绝序与 fail-closed 语义；不启用休眠面；旧 journal 回放零新增报错（字段可选）。
- **封闭集**（`orz-loop/src/host.rs`，`PermitSource`，wire 字符串）：
  `policy`／`scope`／`yolo`／`user`／`timeout`／`fail_closed`／`classifier`（保留：auto-mode 分类器，现休眠，生产不产出）。
- **映射表（orz-host `permission.rs::request_with_source`）**：

  | 判定路径 | source |
  |---|---|
  | 桥策略门短路（ReadOnly 拒；Benchmark 四险类判定） | `policy` |
  | 桥读面 scope 拒绝（窗口幽灵形态守卫；镜像让路后生产难达） | `scope` |
  | 提示等待超时（300s → fail-closed） | `timeout` |
  | manager 快路自动放行（yolo ON） | `yolo` |
  | manager 裁决（yolo OFF）：Allow＋ReadOnly 险类 → 读面自动放行 | `policy` |
  | manager 裁决（yolo OFF）：Allow 其他（客户端放行） | `user` |
  | `Decision::Ask`（无客户端应答） | `fail_closed` |
  | `Decision::FollowupMessage`／`Reject`（客户端答复） | `user` |
  | `Decision::PolicyDeny`（manager 策略臂；现休眠） | `policy` |
  | `Decision::Cancelled`（交互未答复终止） | `fail_closed` |
  | 无桥宿主（OrzHost 默认） | `fail_closed` |

- **落点清单（5＋1）**：
  1. `orz-loop/src/host.rs`：`PermitSource`＋`as_str()`；新 trait 方法 `request_permission_with_source`（默认包装旧方法、source=None——所有既有宿主零改动兼容）。
  2. `orz-host/src/permission.rs`：`request()` 变包装；新增 `request_with_source()` 承载上表映射。
  3. `orz-host/src/lib.rs`：`SessionHost` 覆盖（有桥→`Some(source)`；无桥→`(Deny, Some(fail_closed))`）。
  4. `orz-loop/src/host_exec/tool_run.rs`（`~:570`）：`permission_decision` payload 建为 `{tool, decision}`＋宿主自报时增可选 `"source"`。
  5. 父仓 `runtime/permission-decision-event-payload-v0.1.schema.json`：新增可选 `source`（enum 七值＋description；`additionalProperties:false` 不变、不入 `required`）。
  6. 附注：`permission_decision` 仅登记 v01 轨（v0.2 无该键）；v0.2 卷的 payload 校验经回退用 v01 payload 表（Python 与 Rust 双判官同源）——故单文件改动即覆盖两条轨的校验。
- **钉子（全过）**：
  - **P1** `ask_auto_approves_under_default_yolo` → `(AllowOnce, yolo)`（同址扩钉）。
  - **P2** `read_only_policy_denies_non_read_before_manager`（三险类循环＋MCP 名臂）→ `(Deny, policy)`。
  - **P3** `host_without_bridge_fails_closed_on_permission` → `(Deny, Some(fail_closed))`。
  - **P4** journal 落账：`denied_tool_round_replays_tool_message` → payload `source="fail_closed"`（正例）；`tool_call_round_trips_through_dispatcher` → 未自报宿主保持旧 `{tool, decision}` 形状（零新增字段）。
  - **P5** schema 封闭集：`permission_decision_source_schema_rejects_unknown_values` → 七值逐一放行／未知值（`grok_said_no`）拒绝／旧形状放行。

### 2.5 auto-mode 半程核验（记录；伴随账）

- **事实**：`orz-host/src/permission.rs` spawn 面 `initial_yolo=true`（工作台默认自动审批；手动弹窗退役为非默认路径）；来源＝2026-09-26 用户令“直接开 auto mode 就行”；树面文件（mtime 约 01:10–01:21）**未随任何治理行记账**（§5 m3）。
- **核验**：`ask_auto_approves_under_default_yolo` 过（Ask 类不再等弹窗/超时）；`permission::` 21/21；orz-host 全量 lib 318 过／6 负载敏感败（单跑全过，见 §5 m2）。
- **影响记录**：默认配置下 `source=user/timeout` 臂不可达（观测面局限，S4 对账注意；§5 m8）；`fail_closed` 无桥默认不变（P3 钉）。

## §3 验证证据

| # | 命令 | 结果 |
|---|---|---|
| 1 | `cargo test -p orz-workspace --lib -- permission::prompter` | **29/29 过**（含 ⑮ 新钉） |
| 2 | `cargo test -p orz-sampling-types --lib` | **285/0 过**（含 ⑯-① 新钉） |
| 3 | `cargo check -p orz-loop -p orz-host` | Finished（**57.79s**，0bt④ 全链编译过） |
| 4 | `cargo test -p orz-host --lib` | 318 过／6 败＝负载敏感族（见 §5 m2）；**单跑 6/6 全过** |
| 5 | `cargo test -p orz-host --lib -- permission::` | **21/21 过**（含 P1/P2） |
| 6 | `cargo test -p orz-host --lib -- host_without_bridge` | **1/1 过**（P3） |
| 7 | `cargo test -p orz-assurance --lib` | **276/276 过**（含 P5） |
| 8 | `cargo test -p orz-loop --lib -- tool_run::` | **40/40 过**（含 P4 两钉） |
| 9 | 盘面 | D: 一度 0.001 GiB（os error 112 中断编译）→ 清理后 7.24 GiB→测试后 ~5.17 GiB（§5 m1） |

（第 4 条为 0bt④ 落码前的基线全量；0bt④ 落码后受影响面以 5–8 的定向套件覆盖。）

## §4 工作区状态与锚点（截至报告时）

- **orz 仓**：21 文件 modified＋1 untracked 目录 `crates/codegen/orz-workspace/tests/`（内含 `real_store_probe.rs`，707 B，9/25 22:56 本地；**本会话未动**，处置见 §5 m4）。本会话自编辑 **8 文件**：`prompter.rs`／`error.rs`／`oauth.rs`（⑮⑯）、`host.rs`＋`permission.rs`＋`lib.rs`＋`tool_run.rs`（0bt④）、`conformance.rs`（P5）。其余在途改动（`bin/main.rs`、`acp_server.rs`、`codex_app.rs`、`session.rs`、`orz-web/*` 9 文件）为前序面。
- **父仓**：8 文件 modified（含本会话 `runtime/permission-decision-event-payload-v0.1.schema.json`）＋本报告新增；其余（`CLI_PROJECT_INDEX.md`／`TODO.md`／BACKLOG 两卷／`0BR_S3_…` 残改／存稿 index）为前批未提交面。
- **口径**：不提交／不推送／不重建；闭合时建议 orz 与父仓**成对提交**（schema 与产出代码互为约束，跨仓一体）。
- **留档**：本会话本地全量留档（journal＝`RUN-CLI-6ab6b76d`；压缩档与黑板 notes 载关键状态）。

## §5 摩擦台账（本轮 f1–f8；前轮 m1–m15 见前件 §5）

- **f1 磁盘写满（os error 112）**——现象：orz-loop 测试编译中断（`failed to build archive …libaws_sdk_s3… 磁盘空间不足`；`rustc-LLVM ERROR: IO failure on output stream: no space on device`；D: 余量 ≈0.001 GiB）。处置：清理 `orz/target/debug/incremental`（7.87 GiB）＋`deps/rmeta*` 中断残留临目录 → 余量 7.24 GiB → 重跑通过（测试后 ~5.17 GiB）。残：D: 盘面长期紧张（target 38.4 GiB＝debug 32.25／release 6.15）；建议后续轮次轮前检查构建余量（本报告未动 release 物）。
- **f2 orz-host 六测试负载敏感并联败**——名单：`call_tool_timeout_kills_process_tree`／`tool_spawn_is_really_bounded_by_the_run_job`／`session_volume_symlink_windows_end_to_end`／`resource_hint_reads_once_per_call_tool`／`unavailable_readings_hint_nothing_and_never_block`／`call_tool_with_timeout_override_is_honored`；全量并联 6 败、单跑 6/6 全过（资源竞争型）。处置：单跑复核全过；属既有 flaky 族（TODO P3“orz-host flaky”已登记）。残：维持单跑复核惯例。
- **f3 auto-mode 落码未记账**——现象：`initial_yolo=true` 等在途改动在树面、无治理行。处置：§2.5 记录核验；建议补一行记账（同批达成增量入账惯例）。残：无技术残。
- **f4 real_store_probe.rs 探针未处置**——`orz-workspace/tests/real_store_probe.rs`（707 B，9/25 22:56）untracked，原文标注“诊断后删除”。处置：本会话未动（避免越权）。残：建议闭合前确认删除或转夹具入库，防 untracked 面混入后续提交。
- **f5 ⑯-③ 对端重注册风险（设计风险）**——MCP OAuth 客户端名改 `AI` 后，对端若按名白名单或需重新注册授权；代码注释已载、本报告记录。残：观察项。
- **f6 PowerShell 包装噪音与 GBK 回显面（工具面）**——cargo stderr 被包装成 NativeCommandError 噪音（exit 仍正确）；中文档直读偶发 GBK 乱码（需 `[Console]::OutputEncoding=UTF8`）。处置：命令前统一设编码、噪音忽略。残：既有工具面摩擦（前轮 m 族同源）。
- **f7 长单行/大文件读取需分页（承 0bb 族复证）**——本会话回读 MB 级 .log 需 read_file 分页；大单行仍无专门取用窗口。残：即 0bt① 未竟依据（§6）。
- **f8 观测面局限（0bt④ 自识别）**——auto-mode 默认下 `user`/`timeout` 来源臂不可达；`scope` 臂因镜像让路生产难达；`Cancelled`（交互终止）归 `fail_closed` 为口径合并。处置：映射表如实标注（§2.4）。残：S4 对账时据此校准。

## §6 未竟与移交

| 项 | 状态 | 指针 |
|---|---|---|
| 0bt① 长单行/大单列取用面 | 未动 | 承 0bb；本轮 f7 复证 |
| 0bt② 版本核验旁路＋入口纪律 | 未动 | 承 0bj① |
| 0bt③ 宿主 shell 通道纪律 | 未动 | 承 0bj③＋m9 |
| ⑧-⑫ 检索线五件（S1 定稿已毕） | S2 未落码 | 前轮报告 §4/§6；批序 ⑪→⑩/⑨→⑧→⑫；“除重建与实机外全部做完”口径未达——建议下轮载体化 |
| ⑬ 信任×ACAF（S1 定稿） | S2 未落码 | 批八线（BR Web 工作台） |
| S3 载体重建／S4 真机复验 | 待令 | 全轮不重建口径 |
| auto-mode 记账行 | 待补 | §2.5／f3 |
| real_store_probe.rs 处置 | 待定 | f4 |

## §7 附录

### A. 本会话自编辑文件（9 件）
- orz（8）：`crates/codegen/orz-workspace/src/permission/prompter.rs`（⑮）／`crates/codegen/orz-sampling-types/src/error.rs`（⑯-①）／`crates/codegen/orz-mcp/src/oauth.rs`（⑯-③）／`crates/orz-loop/src/host.rs`・`crates/orz-loop/src/host_exec/tool_run.rs`・`crates/orz-host/src/permission.rs`・`crates/orz-host/src/lib.rs`（0bt④）／`crates/orz-assurance/src/journal/conformance.rs`（P5）。
- 父仓（1）：`runtime/permission-decision-event-payload-v0.1.schema.json`（0bt④ 可选 `source`）。
- 另：本报告新增（`docs/audits/0BS_EXECUTION_AND_FRICTION_2026-09-26.md`）。

### B. 验证命令（原文见 §3）
`cargo test -p orz-workspace --lib -- permission::prompter`／`cargo test -p orz-sampling-types --lib`／`cargo check -p orz-loop -p orz-host`／`cargo test -p orz-host --lib`（并联）＋两条单跑滤径／`cargo test -p orz-host --lib -- permission::`／`cargo test -p orz-host --lib -- host_without_bridge`／`cargo test -p orz-assurance --lib`／`cargo test -p orz-loop --lib -- tool_run::`。

### C. 会话属性
- run＝`RUN-CLI-6ab6b76d`；基线 HEAD＝`a441ff1c4066`（run 起始 worktree 含未提交改动）；本地单对话全量留档；压缩档与黑板 notes（“0bs 处理进展”）载关键状态。
- 载体口径：**不提交／不推送／不重建**；`permission_decision` schema 与 orz 代码跨仓成对（闭合提交时须一体）。
- 报告落点：`docs/audits/0BS_EXECUTION_AND_FRICTION_2026-09-26.md`。