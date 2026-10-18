# 299 批：0dh 全面审查处置——P0 生命周期缺口闭合＋九项落码（S2-R）（2026-10-18）

> 性质：**审查处置＋修复批**（用户令「请对当前完成的0dh进行全面检查，包括设计合理性、
> 实现合理性、设计与实现的符合性。如审查压力大请派出子代理」→ 处置裁决沿 0cc/0df 先例
> 由用户令「其他审查出的全部问题都需要处理，如待处理项较多，可拉起子代理帮助处理」授权；
> 修复方向用户裁定「把 shutdown_all 改为和 orz 主进程绑定……只要能 orz 进程本身结束后收掉
> 残余进程就可以」＝会话级 backend 使 `shutdown_all` 触发点收敛为会话关闭/进程退出）。
> orz 子仓本批提交 `7f85de3a`（15 文件，+701/−89）；不推送；父仓账本随批。计数不变 65。
> 设计权威：[`LONGRUNNING_PROCESS_EXPLICIT_CHANNEL_DESIGN_2026-10-18`](../LONGRUNNING_PROCESS_EXPLICIT_CHANNEL_DESIGN_2026-10-18.md)
> **v1.1（本批勘正）**＋ADR-0010 §14.87 第 7 条。

## §1 审查结论（三路并行只读深查＋主会话关键发现亲验）

- **符合性（设计↔实现）**：**符合**。D1–D8 逐条对账全部落码且可由代码证据追溯；钉八面全部
  真实非空壳；S2 批档 §1 五件与 §2 钉面声明与代码事实逐条相符、无不实表述（含「`session_owned`
  无冻结消费面」全仓核实为真）。低严重度偏差 6 项（渲染缺 m total／D5② 保留句被改写／D5①
  微扩／常量更名未声明／混合 sweep 审计盲区〔继承性〕／钉⑧ 无 run 边界测试）——本批全数处置。
- **实现质量**：**零 P0/P1**。豁除判定 fail-safe 方向、单 actor 游标串行化、Job 句柄生命周期
  三处高危面实证无误；生产码零新增 unwrap。P2×4（混合 sweep 审计缺口／首读 `new_output_bytes`
  语义漂移／封口臂断言缺失／30h 常量无一致性钉）＋P3×9——本批全数处置（「New output」首读
  措辞随 m total 一并解决；教学文案「~30h」硬编码经 eval 接受＝「~」缓冲，登记不修）。
- **设计合理性**：**P0×1**——**跨 run 存活被第二杀手结构性否决**（§2）；风险登记不足两处
  （§5.1「至多 30h」对 breakaway 残余不成立、CPU-busy 泄漏仅墙钟一道闸——设计稿 v1.1 勘正）；
  文档小不一致两处（BACKLOG P1 路由行 stale＝本批勘正；§14.55 第 7 条 10h 值波及＝指针注）。
  无问题确认清单：approval 门与写控 L2 对后台路径同等生效（permission 层无 `is_background`
  特判、`review_command` 先于后台化分支）、8 工具面第二例外形态与第一例外同构、多会话并发
  归属判定健全（run_id 先过滤＋标记集被过滤闸挡住）、env 命名先例一致、1h 下限合理、0bq
  兜底保留、库层缺省封闭/生产显式注入反转正确。
- **P0 证据链（主会话逐环亲验）**：①`build_host` 在 `handle_session_prompt`（每 prompt 一次）
  内调用，作者自注「host 每次 prompt 新建」（acp_server.rs:3277/:3603 一带）；②terminal
  backend 在 `OrzHost::new` 内私有构造、无会话级持有点（lib.rs:313；browser 有
  `SharedBrowser`+fold-back、terminal 无）；③backend 持 `cmd_tx`，host drop→通道闭→actor
  `None` 臂 `shutdown_all()`（terminal.rs:1224-1228）；④`shutdown_all` 对 `processes` 全量
  `send_sigkill_to_group` 无 lifetime 过滤（terminal.rs:2440-2455）。**用户方案的必要修正**：
  仅改 `shutdown_all` 跳过 session 任务不充分——actor 灭时 `ProcessState` drop 会闭合 per-call
  Job（KILL_ON_JOB_CLOSE）句柄照杀整树，且 actor 是唯一管理面（任务表/游标/提醒），灭则跨
  run 读杀提醒全失效；故「绑定 orz 主进程生命周期」的正解＝backend/actor 会话级持有，使
  `shutdown_all` 天然只在会话关闭/进程退出时触发（与用户目标等价）。

## §2 P0 修复＝会话级 terminal backend（orz `7f85de3a` 核心）

- `StoredSession` +`terminal_backend: Arc<LocalTerminalBackend>`（session_new 一次构造，
  一 actor 一会话；acp_server.rs）；`build_host` 查表注入（新构造变体
  `with_bridge_and_hub_policy_with_terminal`，`None`＝一次性/无头车道私有 backend 旧行为；
  `with_credential_reader_and_terminal` 内部穿引；grill/stdio/codex_app/TUI 经同一
  `build_host` 自动继承）；`close_session` 同步 `cancel()`→actor `shutdown_all` 收殓会话域
  任务（镜像 browser detached-shutdown 先例）。
- 跨进程残余兜底链不变：Job KILL_ON_JOB_CLOSE 随 orz 进程闭＋下会话装配 Start sweep 收
  breakaway 残余（0bq 不动）。in-flight run 场景：run 的 host 持自有 Arc，backend 存活至
  run 尾——cancel 只是让 actor 尽早退出，与 browser 语义同形。
- **e2e 钉 `session_terminal_backend_survives_host_rebuild_and_close_teardown`**（orz-host
  集成、真 spawn `sleep 6`）：prompt1 尾（host 已 drop）任务存活×prompt2 尾（第二个 host）
  仍存活×close_session 后收殓三断言。该场景 S2 八面单元钉无一覆盖——正是能拦住 P0 的那颗钉。
- **连带澄清**：S4 判据②（journal `session_owned` refusal 行）在修复前会「通过」而任务已
  被 teardown 杀死——refusal 行只证明 sweep 不杀；设计稿 v1.1 已在 S4 判据⑦登记该陷阱。

## §3 九项随批处置

| # | 项 | 处置 |
|---|---|---|
| 1 | P2-1 混合 sweep 的 SessionOwned 拒绝无审计行 | `execute_sweep` 重构：refusal 审计与 NotRunning 陈旧行清理（F-BE-5 同款）两分支统一执行，零收割臂逐字节兼容；新钉 `execute_sweep_audits_refused_rows_in_mixed_sweep`（合成决策，跨平台） |
| 2 | P2-2 首读 `new_output_bytes` 语义漂移 | 首读改报服务窗口 payload 长（与 `xai-tool-types` 契约「THIS response's payload」对齐）；渲染补总量 `({m} bytes total; …)`＝D6①「照报 total_bytes」兑现；内部 `TaskSnapshot`+`total_output_bytes` 穿引，**外部契约零新增字段**；顺带修 `raw_output_bytes` 增量 serve 下取段长（收缩、破坏 doom-loop 检测）为优先 producer 总量 |
| 3 | P2-3 封口臂断言缺失 | `incremental_cursor_first_delta_and_seal` 补终态余段断言（50ms×40 容差环：封口读含 THIRD-BATCH＋new_output_bytes 恰等＋只含余段） |
| 4 | P2-4 30h 常量无值钉/无一致性钉 | `ABSOLUTE_MAX_TIMEOUT_MS` 值钉 108_000_000＋跨常量钉（==`DEFAULT_BG_MAX_RUNTIME` 毫秒，转 pub(crate)） |
| 5 | P3 stale「10h」注释 | 六处勘正（bash/mod.rs 四处＋terminal.rs 四处命中后核对取舍：monitor 常量/历史裁决注记/外部 opt-in 三类有意保留） |
| 6 | P3 `SESSION_SPAWNS` 永不清理 | +`unmark_spawn_session`＋任务终态三面接线（tombstone 迁移/kill_task/shutdown_all；`transition_to_background` 进程仍活不删）＋往返单测 |
| 7 | P3 死导出 `session_spawn_started_at` | 全仓零调用证实后删除（定义+re-export） |
| 8 | P3 EnvGuard/SAFETY 注释不实 | 如实化（cargo test 默认并行、无害因无他测碰该变量＋生产并发读被 1h 下限/非法回落消解） |
| 9 | 三颗 sweep 钉 `#[cfg(windows)]` 空转 | 决策层（SessionOwned 豁除/容差/Start 不查标记）改合成行＋合成标记跨平台；执行层（真 spawn 真杀）保留 cfg(windows)；SessionOwned 判定先于存活检查的实际矩阵入钉 |

连带：orz-workspace `hub.rs` 测试 fixture 补 `total_output_bytes`（TaskSnapshot 加字段连带编译破损，实跑捕捉）。

## §4 读数

- orz-host **串行 374/0**（基线 372＋2 钉：混合审计钉＋本批 e2e 钉）；orz-tools **3031/0**
  （+3 钉：渲染总量/常量一致性/封口断言扩面）；orz-workspace **1500/0**；xai-tty-utils
  **35/0**（+1 unmark 往返）；xai-tool-types **100/0**；
- fmt 触碰件净（lib.rs/acp_server.rs/hub.rs＋两子代理触碰件；browser_serp/codex_app 等
  预存漂移非触碰区照旧——rustfmt 沿 mod 树展开，触碰件逐一单独核验）；
- clippy 对比法升级＝**告警位置列表 stash 前后逐位 diff**（较计数法更严）：orz-host
  114=114（新构造变体 `too_many_arguments` 沿仓内五处 `#[allow]` 先例）、orz-tools/
  xai-tty-utils/xai-tool-types 零新增、orz-workspace 22 持平；
- 下游 orz-bin/orz-tui/orz-web 全量 check 干净；全仓 check 受 `protoc` 缺失阻断＝环境预存
  （proxy 车道 orz-tools-api，0dh 未触碰，历批不跑全仓 check）。

## §5 边界登记（三项）

1. 同 run 内已死 session 行留盘至下次装配清扫（0bq 域——行是审计取证面，会话内删除会
   侵蚀 0bq 语义；不修，登记）。
2. idle-kill 事实跨 run 可能重复上报（per-host 去重集语义；会话级任务跨 run 后新 host
   去重集为空；信息面 journal fact、无控制语义；不修，登记）。
3. `session_new` 同 id 重创建＝新任务域（旧 backend drop 即收殓旧任务）；教学文案「~30h」
   为 env 可改兜底的近似表述（「~」缓冲，接受）。

## §6 处置

- orz 子仓提交 `7f85de3a`（feat/fusion-architecture；不推送）；版本 bump 随 S3 重建批
  （沿 289 先例）。
- 账本：设计稿 v1.1 勘正＋ADR §14.87 第 7 条＋§14.55 第 7 条指针注＋本批档＋索引
  v4.271→v4.272（头行＋AUTH-LONGRUNNING 条目勘正）＋BACKLOG 指针/计数行/路由行
  stale 勘正/0dh 节＋TODO 计数行/路由行/P1-0dh S2-R 勾选＋第二卷 §1.239。
- 下一批：**S3 载体重建进体**（字节判据随 298 批定，不变）＋S4 真机（294 预登记＋v1.1
  增补⑥⑦）。
