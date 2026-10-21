# 327 批：恢复并继续 P0 修复（tool_calls 配对／归档直恢复／标记栏 turn，0.9.7 换装，2026-10-21）

> **用户报告**：「恢复并继续还是存在bug的，必须存档再回档后才能成功恢复，并且恢复后标记栏错误，
> 新的消息输入后报 `An assistant message with 'tool_calls' must be followed by tool messages
> responding to each 'tool_call_id'`」。
> **本批**＝按真机 `C:\Users\1\OneDrive\Desktop\新建文件夹\.gsa` 的 `c23e0aaa` 会话与
> `b41b096d`／`e6508b76` 归档包逐件核证后修复；**只读探针，未改写用户活数据**。
> **0.9.7** 已构建并热替换 Windows 装机位/载体位；ACAF 已重 provision；旧 `orz web` 进程
> 仍持有 0.9.6 映射，需重启后新 UI/后端才生效。

## §0 结论速览

| 面 | 修复前事实 | 修复后 |
|---|---|---|
| 归档会话恢复 | 活区无侧车 ⇒ `session/load` 直接 `session not found`；只能先手动「回档」 | `load` 按回档语义自动补缺件（只补缺、不覆盖），随后 attach/修复 |
| 重构 run 顺序 | `ids.sort()` 字符串序：`-10` 排在 `-2` 前（真机 19 run 会话乱序） | 按 run 末尾序号**数值**排序；CLI 单 run 稳定置后 |
| 工具配对 | 重构保留 `model_output.tool_calls`，journal 的 `tool_completed` **不携带结果正文** ⇒ 199 个孤立声明、0 条 tool 消息 | 重构不再重放工具声明；装载时通用配对修复（全缺省剥离、部分缺省补真实语义合成结果、孤儿/重复结果删除） |
| 恢复后 provider 400 | 坏侧车直接把孤立 `tool_calls` 送进请求 | 旧侧车在 attach 与 prompt 前双重修复；请求只含 provider-valid 历史 |
| 成功率语义 | 重构全量拼接失败/取消 run（重复首问＋中间输出） | 只收 `run_finished` 的 run；最后一个未完成 run 仅保留用户输入 |
| 上下文水位 | 重构丢失 `context_scale_notified`，恢复后 192k/256k 重发（真机 `RUN-c23e0aaa-18`） | 从**成功 run** 的 `mechanical_audit_update` 恢复水位键（含 `first_block`/`320k` latch） |
| Web 标记栏 | 恢复后 `turnCounter` 从 0 重排，`RUN-…-18` 标成 T1 | `prompt_submitted` 取 run 后缀 `n+1` 作绝对 turn |
| 归档视图恢复按钮 | `resumeTarget` 只认 `conversation://`，归档视图按钮不可点 | `archive://` 同权；归档详情带 `session_id` 即可「恢复并继续」 |
| 计数 | 63 | 63（本批为缺陷修复，不新增待办） |
| Windows 构建 | 0.9.6 | `cargo build -p orz-bin --release -j 2` **exit 0（最终构建 1m58s）**；`orz.exe` 58,832,896 `29f2d765…`／signer 6,740,480 `ad147ea1…`／provision 6,640,128 `71c455bb…` |
| 换装 | 装机位/载体位 0.9.6 | 三件 MATCH 3/3；两侧 `.0.9.6-bak`；运行中旧文件 `.swap-old` 仍为 0.9.6；`--build-info` 双点 `0.9.7` |
| ACAF | manifest `0bf55ac8…` | 备份 `signer-manifest.json.bak-20261021-327`／`-327b`；re-provision exit 0；`binary_sha256=ad147ea1…` ↔ signer |
| 冒烟 | — | `RUN-CLI-6ad8879b` **rc=0**，11 events；`run_finished` `terminal_source=short_text`／nudge 0／selfcheck false |

## §1 真机证据（只读）

- 活侧车 `.gsa/conversations/c23e0aaa.json`：`schema=0.2.0-draft`、`reconstructed_from_journal=true`；
  **256 条消息＝18 user＋238 assistant**，其中 **199 条 assistant 带 tool_calls、0 条 tool 消息**。
- 同会话 run 0–18：19 个 `RUN-c23e0aaa-*` 目录；run 0 与 run 1 重复首问，run 1 无
  `run_finished`；run 18 无 `run_finished`（用户报告 400 的那次）。
- 归档包：`b41b096d.json.gz`／`e6508b76.json.gz` 的 `conversation` 均为重构产物
  （159/133 assistant 工具轮、0 tool 消息）；`2b1f85d2.json.gz` 为正常侧车（181 tool 消息）。
- 新重构只读探针（同代码路径）结果：**66 条消息＝17 user＋49 assistant、0 带 tool_calls**；
  `context_scale_notified=["first_block","192k","256k","320k"]`；最后一条 user 为 run 18 的
  未完成请求（保留用户输入，不保留失败中间输出）。

## §2 修复面（代码）

| 文件 | 改动 |
|---|---|
| `orz/crates/orz-host/src/conversation_repair.rs`（新增） | `repair_tool_pairing(&mut Vec<Message>)`：全缺省声明剥离（无正文则删消息）、部分缺省逐 call 补 truthful 合成结果、孤儿/重复 tool 结果删除；幂等；6 条单测 |
| `orz/crates/orz-host/src/acp_server.rs` | `StoredConversation.reconstructed_from_journal`；重构数值排序/成功 run 过滤/最后失败 run 用户输入保留/去 tool_calls/恢复水位键；`journal_covers_session` 防 retention 缺口覆盖；attach 重放＋配对修复并回写；prompt 前防御修复；`session/load` 自动回档；`journal_key_facts` 数值排序 |
| `orz/crates/orz-web/assets/app/projection.js` | `prompt_submitted` 的 turn 取 `RUN-{s8}-{n}` 后缀 `n+1`；无后缀（`RUN-CLI-…`）回退本地累加 |
| `orz/crates/orz-web/assets/app/state.js` | `syncToolbar` 允许 `archive://` 视图启用「恢复并继续」 |
| `orz/crates/orz-web/assets/app/main.js` | `openArchive` 从归档详情写 `resumeTarget`；`viewedConversation8` 识别 `archive://`；归档浏览文案给恢复提示 |
| `orz/crates/orz-web/tests/frontend_smoke.mjs` | 新增「恢复后 turn 号按 run 序号」与「归档视图直接恢复」两条钉子 |

## §3 行为口径

- **全缺省 vs 部分缺省**：assistant 声明的调用**全部**无结果时，剥离声明（有正文保留、无正文删消息）；
  **部分**有结果时保留声明，逐缺失 call 插入
  `工具调用 … 的结果在会话恢复时缺失：journal/侧车未保留工具输出。…请勿把它当作成功；如仍需要该事实，请重新调用。`
- **成功 run 过滤**：只有 `run_finished`（status=completed）的 run 进入重构对话；最后一个未完成 run
  只保留 `prompt_submitted` 用户消息（若与上一条用户消息完全重复则跳过）。
- **重放覆盖门**：`journal_covers_session` 要求 `RUN-{s8}-{n}` 序号从 0 连续无缺口；
  当重建面 prompt 数少于侧车时，还要求侧车确有重复 prompt（失败 run 污染证据）且侧车每个
  prompt 都能在重建面找到（不丢独有输入）。任一不满足即不覆盖侧车，只做就地配对修复
  （保数据优先于排序修复）。
- **旧侧车自动迁移**：`reconstructed_from_journal=true` 的装载先按新规则重放 journal；journal
  已随归档删除/有缺口时退化为配对修复并回写。
- **自动回档**：`session/load` 在侧车缺失且 `{s8}.json.gz` 存在时调用
  `unarchive_session_on_demand`（只补缺、不覆盖，随后删包），再走同一 attach 路径。

## §4 验证

| 入口 | 结果 |
|---|---|
| `cargo test -p orz-host --lib conversation_repair` | **6/6** |
| `cargo test -p orz-host --lib archive_` | **13/13**（含数值排序/成功 run/最后失败 prompt） |
| `cargo test -p orz-host --lib session_load` | **2/2**（含归档自动回档＋配对修复） |
| `cargo test -p orz-host --lib new_server_resumes_conversation_from_sidecar` | 1/1 |
| `cargo test -p orz-host --lib dangling_tool_calls_from_sidecar_never_reach_the_provider` | 1/1（请求面断言无孤立声明） |
| `cargo test -p orz-host --lib` | **405 tests：393 passed / 6 failed / 6 ignored**；6 failed 中 5 条为既有环境失败（`call_tool_timeout_kills_process_tree`、`run_tests_timeout_kills_process_tree`、`call_tool_with_timeout_override_is_honored`、`run_terminal_cmd_truncation_carries_output_object`、`session_volume_symlink_windows_end_to_end`），另 1 条 `run_tests_records_workspace_delta` 在并发全跑下偶发、**单独复跑 1/1 通过** |
| `cargo test -p orz-web --lib` | **52/52** |
| `node crates/orz-web/tests/frontend_smoke.mjs` | **全部通过**（含新增 2 条） |
| `cargo build -p orz-bin --release -j 2` | **exit 0（最终构建 1m58s；首轮 2m58s）**；唯一警告＝`orz-host/permission.rs` unused imports（预存） |
| Windows 装机位/载体位 | 三件 MATCH 3/3；`--build-info` 双点 `0.9.7 os=windows arch=x86_64`；两侧 `.0.9.6-bak` 在位；`carrier-manifest.json` 刷新为 0.9.7（3 entries，启动校验无告警） |
| ACAF | 备份后 re-provision exit 0；manifest `ad147ea1…` ↔ carrier signer 逐位一致 |
| 冒烟 | `RUN-CLI-6ad8879b` rc0，11 events，`run_finished` 四字段正常 |
| 真机只读探针 | 见 §1；未写入用户 `.gsa` |

## §5 边界与未决

- **旧重构侧车的乱序不可逆**：若 journal 已被 retention 清出缺口，装载只能修配对、无法恢复原时间序；
  本批以「不覆盖、不丢数据」优先，日志警告 `journal rebuild covers fewer prompts than the sidecar`。
- **黑板不可由 journal 重构**：`tool_completed` 不带 `blackboard_write` 正文；重构侧车仍 `blackboard=None`，
  水位显示 `【0.0M/10M】` 是真实空板，不是本次 400 的原因。恢复黑板需另立设计（不在本批扩面）。
- **8 位 session_id**：重构包内 `session_id` 是 8 位前缀，直接恢复会以 8 位 id 绑定；数据面仍按 8 位
  前缀寻址，功能可用但丢失完整 UUID 形。正常侧车包不受影响。
- **本批未涉及**：Linux 载体重建、GitHub Release 发行。Windows 0.9.7 已换装，但报告时
  `orz web --real`（PID 28644，0.9.6 映射）仍在运行；**重启该进程后新 UI/后端才生效**。
  旧文件以 `.swap-old` 保留（与 `.0.9.6-bak` 同哈希），进程退出后可删除。

## §6 入口

- 代码：`orz/crates/orz-host/src/conversation_repair.rs` / `orz/crates/orz-host/src/acp_server.rs`
- 前端：`orz/crates/orz-web/assets/app/{projection,state,main}.js`
- 测试：`orz/crates/orz-web/tests/frontend_smoke.mjs` / orz-host 单测
- 真机证据：`C:\Users\1\OneDrive\Desktop\新建文件夹\.gsa\conversations\c23e0aaa.json`（只读探针）
