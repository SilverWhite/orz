# 246 批 0cy S1+S2——GSA 会话卫生（新对话新台账＋上一轮整卷进存档）

- **批次**：246（前档＝[`245`](245_RELEASE_V0816_READBACK_VERIFICATION_2026-10-11.md)；同窗＝[`235`](235_0CW_CONTAMINATION_AUDIT_0CY_GSA_HYGIENE_2026-10-11.md)／[`0DA_S4…`](0DA_S4_REACHABLE_CARRIER_SMOKE_AND_FRICTION_2026-10-11.md)）
- **任务**：0cy＝GSA-SESSION-HYGIENE（0cw 审计三因＝ledger 跨会话滚动＋上一轮足迹留活区＋0ct 读全开）
- **范围**：**S1 设计稿＋S2 落码**（本批）；**不做**：提交/推送/重建（S3）与真机复跑（S4）——用户令「不必进行提交/推送/重建，按照项目惯例落一份报告文档即可」
- **日期**：2026-10-11；现场＝父仓 HEAD `dce03a034507`（含未提交改动）＋ orz 子仓 `316107f6` 之上 3 文件未提交改动

## 1. 背景与目标

0cw 污染审计（[`235`](235_0CW_CONTAMINATION_AUDIT_0CY_GSA_HYGIENE_2026-10-11.md)）认定评测「跨会话污染」三因，0cy 的目标形态：

1. **新对话＝新台账**：新会话起点活区从空开始（`.gsa/ledger/current.md` 不带上一会话动作行）；
2. **上一轮整卷进存档**：上一会话的全部足迹（对话侧车、台账、run journal、压缩档、回放块等）**先打包进 `archives/` 再删源**；
3. **与 0cz 上下文面零耦合**：不动压缩/清零管线与工具面，只动会话边界的存档与活区。

0DA 处理轮留下摩擦 F1（指针文案与实况错位）/F2（活区残留）——F2 由本批「启动侧扫描收卷」直接闭合；F1 的文案前提（活区已清、新会话语义成立）随本批落地重新成立（文案本体在 eval 侧 driver，不在 orz 面，未动）。

## 2. S1 设计（已落稿）

设计稿：[`../GSA_SESSION_HYGIENE_DESIGN_2026-10-11.md`](../GSA_SESSION_HYGIENE_DESIGN_2026-10-11.md)（146 行）。

**自裁决定（用户令「S1 中如遇需裁决项可直接进行衡量并裁决，将决定记录下来」——以下为本批衡量后的定案）：**

| 编号 | 决策 | 依据 |
|---|---|---|
| D1 | **收卷＝启动侧扫描的唯一删改路径**：新会话起点（ACP `session/new` 入口、CLI `run()` 起点）扫描活区；close/headless 收尾包只**升级档案**（带足迹），不删源——「先写包后删源」的删除只发生在下一次会话起点 | 删改集中一处可审计；运行中会话的文件不被半途删除（close 时 run 可能仍在飞） |
| D2 | 扫描口径＝**非存活 ∧ 非当前**：存活＝进程内 `sessions` 表；跨进程边界保守（无表⇒只保护 keep） | 同进程安全优先；跨进程（评测重启）由「整卷进存档」兜底 |
| D3 | **足迹清单**＝逐件原文进包；`ARC-*`/`PLAN-*` 目录＝回执/计划豁免**不删**（证据留在盘面） | 包即证据；回执不属「足迹」 |
| D4 | **包 schema 升 v0.3**：`{schema, conversation(可 null), archive_keys, footprint}`；合并写包（旧包成员不丢：runs→id、其余→path）；**先写包、后删源**；单件 32MiB／总量 128MiB 超限⇒`incomplete`⇒不删源 | 读取侧（web `split_envelope`）容忍「schema 在场＋conversation 成员」⇒v0.3 对旧读者透明；宁留不丢 |
| D5 | **零新事件类型／零 payload 新字段**：`session_archive` 事件复用（payload schema `additionalProperties:false`）；ARC run journal 承载事件 | 契约面零变化 |
| D6 | **env 口径**：`ORZ_GSA_ARCHIVE_ROOT`（归档根覆盖；绝对原样、相对对 base 解析；净室重跑可把归档移出工作区）＋`ORZ_GSA_HYGIENE=0\|off\|false`（扫描关断逃生舱；缺省开） | 排障/评测双场景 |
| D7 | **unarchive 升级为按需恢复**：读包→只补缺件（绝不覆盖在位文件；ledger 仅缺席时恢复）→删包＋水位；**包不可读⇒拒绝移除**（宁拒不删）；`delete` 扩面（＋RST/定向/激活/grill/压缩档/回放块；ARC 目录保留原口径） | 「回档」从「删包」变为「真能拿回数据」；删除面向「彻底」 |
| D8 | **ledger 路径不变**（`.gsa/ledger/current.md`）；共享件（ledger/终端日志）归**最近被收卷会话**（mtime 最新；平手取 id 序后者）；存在其他存活会话⇒共享件保留不动 | 工具/读取面零改动；共享件不跨会话继承 |

**实施细化（S2 执行中定案）**：① keep/live 归一为 **8 字符**会话语（磁盘命名口径；不归一则 ACP `sess-<uuid>` 类全长 id 在「同会话重启」时误扫自己）；② 侧车损坏（半写/截断）⇒视为缺席走 journal 重构；重构不成且足迹空⇒早退不删（fail-closed）；③ `run_dir_session` 排除 `ARC-`/`PLAN-`（收卷回执不被当作足迹再收卷）。

## 3. S2 落码（3 文件）

**`orz/crates/orz-host/src/acp_server.rs`（+1678/−67）**——新节「0cy 会话卫生」＋收尾包升级＋接线：

- 新节函数锚点：`resolve_archives_root`(L958)／`hygiene_enabled_from`(L976)／`collect_session_footprint`(L1243，侧车族 conversations/orientation/activations/grill＋runs＋compaction＋blocks＋共享 ledger/terminal)／`merge_footprint`(L1448)／`read_archive_document`(L1474，读包上限 64/128MiB)／`delete_session_sources`(L1510)／`rollup_session`(L1568，侧车→journal 重构→旧包 conversation 回填)／`sweep_session_leftovers`(L1678)／`sweep_session_leftovers_with`(L1694，注入变体)／`scan_session_footprints`(L1759，`BTreeMap<s8, mtime>`)／`restore_session_from_archive`(L1858，只补缺件)。
- 包面：`build_archive_envelope`→v0.3＋`footprint` 成员（schema 常量更名）；`build_archive_keys` 收 `Option`；`package_archive_raw`（`Option raw`＋footprint 参）；`archive_session_package`（＋`include_footprint`）；`archive_raw_session_package`（＋footprint）；headless/close 收尾包带足迹、增量包不带。
- 接线：`handle_session_new_with_options` 入口接 sweep（keep＝新会话，先扫后注册）；`PendingArchiveTicket`＋`include_footprint`（close＝true／增量＝false）；`unarchive_session_on_demand` 按 D7 升级；`delete_session_on_demand` 扩面；存档路径统一走 `archives_root`（env 口径）。

**`orz/crates/orz-bin/src/main.rs`（+17）**：`run()` 起点调 `sweep_session_leftovers(&cwd, None, &[])`（CLI 车道新会话起点；best-effort，stderr 一行汇总）。

**`orz/crates/orz-web/src/archives.rs`（+11）**：`archives_dir(cwd)` 读 `ORZ_GSA_ARCHIVE_ROOT`（与 orz-host 同口径；跨 crate 小重复、设计稿 §6 在案）。

## 4. 测试与读数

| 面 | 读数 | 备注 |
|---|---|---|
| `cargo check -p orz-host -p orz-bin -p orz-web` | 净 | 零新增 warning（唯一 warning＝`permission.rs` 预存 unused-import） |
| orz-host lib 单线程全量 | **362 过 / 1 败 / 6 ignored** | 唯一败＝permission 护栏样本（§5.4，基线固有、非本批） |
| 0cy 钉子 N1–N8 | **全绿** | 收卷逐字对账／存活保护＋close 后收卷／共享 winner＋幂等／env 纯函数／关断／merge 防丢／回档恢复＋拒绝损坏包／删除扩面 |
| orz-web | **47 过 / 0 败** | 读取侧兼容（v0.2 fixture 仍绿） |
| orz-bin | `cargo check --tests` 净 | 大链接规避（§5.2，环境资源限） |
| fmt | 本批 3 文件 `rustfmt --check` 0 差 | 误触 14 文件已回退（§5.1） |

首轮 `-j2` 并发曾 9 败，其中**本批面 3 项已修**：`close_with_active_run_defers_archive_until_run_completion`＋`new_server_resumes_conversation_from_sidecar`（同因＝keep 未归一 8 字符、全长 id 会话被误扫，修＝keep8/live8 归一）＋`unarchive_and_delete_session_on_demand_scope_and_gates`（旧 fixture `b"pkg"` 非 gzip；D7 起回档先读包⇒拒绝移除；fixture 改真 gzip v0.2 信封）。其余 5 项＝负载敏感族（单线程全绿、并发挂；先例在案）。另：winner 平手规则经钉子暴露同刻 mtime 平手⇒改「平手取 id 序后者」（确定性）。

## 5. 摩擦项记录（本批）

1. **rustfmt 版本差**：`cargo fmt -p orz-host/-bin/-web` 误触 17 文件（14 个非本批），`git checkout` 回退；本批仅 3 文件保留格式化（版本差导致的仓库面漂移已复现，供后续批注意）。
2. **环境资源限**（本 run 宿主）：链接期「页面文件太小（os error 1455）」／`LLVM ERROR: out of memory`／`STATUS_STACK_BUFFER_OVERRUN`——`-j2` 串行规避；磁盘余量 3.25GiB 时 `cargo` 报 `os error 112`（空间不足）致 orz-web 编译中断——删 `target/debug/incremental` 释 23.15GiB 后续跑（后续构建置 `CARGO_INCREMENTAL=0`）。
3. **负载敏感族**（既有）：`call_tool_timeout_kills_process_tree`／`call_tool_with_timeout_override_is_honored`／`run_tests_timeout_kills_process_tree`／`run_terminal_cmd_truncation_carries_output_object`／`session_volume_symlink_windows_end_to_end`——单线程全绿、并发挂（先例 [BACKLOG_AND_PRIORITIES_2.md](../BACKLOG_AND_PRIORITIES_2.md) ln1399／0aq 批在案）。本批未修。
4. **permission 护栏样本欠跟（基线固有，非本批）**：`permission::tests::read_only_tools_never_fall_into_the_edit_bucket` 断言样本表**恰好覆盖** `ToolDispatcher::READ_ONLY_EXEMPT_TOOLS`——`context_manage`（0cz 批工具）在表、样本缺 ⇒ HEAD 红。与 0cy diff 零交集（本批未触 permission/tools/loop 面）；登记为「0cz 工具面样本欠跟」，本批不修（避免跨批面冲突）。
5. **新语义上线盯两类既有测试**（教训）：①「同会话重启」类（保存恢复流——keep 归一）②「占位 fixture」类（旧包字节非 gzip——读取升级后拒绝）。已修 3 项；建议后续批对此二类先行排查。
6. **0DA F1/F2 回指**：F2（活区残留）＝本批直接闭合（启动侧扫描＋收卷清空）；F1（指针文案与实况错位）＝文案前提随本批落地重新成立，文案本体未动。

## 6. 交付物与边界

- **交付物**：`orz/crates/orz-host/src/acp_server.rs`（+1678/−67）；`orz/crates/orz-bin/src/main.rs`（+17）；`orz/crates/orz-web/src/archives.rs`（+11）；`docs/GSA_SESSION_HYGIENE_DESIGN_2026-10-11.md`（新，146 行）；本档。
- **边界（如实记）**：不提交/不推送/不重建（S3 重建与 S4 真机不在本批）；`BACKLOG`/`TODO` 台账未动（沿 0da 先例「报告即留痕」）；跨进程边界＝保守（无表时按盘面收卷，keep 保护缺省失效——评测重启用例）；`incomplete` 超限件不删源（重扫机会保留）；侧车损坏且重构不成且足迹空 ⇒ 早退不删（fail-closed 残留）；orz-loop／orz-assurance 测试本批未跑（零触面；读数沿用，复跑随 S3/S4）。
- **净室重跑用例（供 S4）**：`ORZ_GSA_ARCHIVE_ROOT=<工作区外目录>` 起跑——会话边界处自动收卷，工作区 `.gsa` 不累积前会话足迹；`ORZ_GSA_HYGIENE=0` 可关断对照。

## 7. 指针

- 设计稿：[`../GSA_SESSION_HYGIENE_DESIGN_2026-10-11.md`](../GSA_SESSION_HYGIENE_DESIGN_2026-10-11.md)
- 前档：[`235`](235_0CW_CONTAMINATION_AUDIT_0CY_GSA_HYGIENE_2026-10-11.md)（三因）／[`238`](238_0CZ_S2_CONTEXT_MANAGE_IMPL_2026-10-11.md)（0cz S2）／[`241`](241_0DA_S2_CONTEXT_MANAGE_R2_IMPL_2026-10-11.md)／[`245`](245_RELEASE_V0816_READBACK_VERIFICATION_2026-10-11.md)
- 代码锚点：`acp_server.rs` 新节（L958–L1877 区）＋`handle_session_new_with_options` 扫描块＋close/`PendingArchiveTicket`＋`unarchive`/`delete`＋测试尾（N1-N8）。
