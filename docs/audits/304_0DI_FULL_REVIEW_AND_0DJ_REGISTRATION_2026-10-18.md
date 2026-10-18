# 304 批：0di 全面审查——设计合理性／实现合理性／符合性三路深查＋全部问题立项 0dj（2026-10-18）

> 性质：**审查批**（零源码、零跑批；用户令「请对当前完成的0di进行全面检查，包括设计合理性、
> 实现合理性、设计与实现的符合性。如审查压力大请派出子代理」＋「请落一份审查文档先吧，并将
> 审查出来的全部问题立项成一个杂项，作为下一步待办」）。被审对象＝orz `01386744`（302 批，
> 24 文件 +1861/−9）。计数 **65 → 66**（新立项 0dj）。
> 方法＝三路并行只读深查（设计／实现／符合性各一子代理）＋**承重结论主会话逐条亲验代码**
>（0dh 299 批审查同法）。审查依据＝设计稿 [`0DI_RECALL_SUPPLEMENT_DESIGN_2026-10-18`](../0DI_RECALL_SUPPLEMENT_DESIGN_2026-10-18.md)
>（§1 六项勘定＋D1–D8）＋302 批档＋ADR-0010 §14.88。

## §1 总体裁决（三路汇合，主会话核定）

| 路 | 裁决 | 要点 |
|---|---|---|
| 设计合理性 | **基本合理，无结构性缺陷** | S1 六项勘定与 D1–D8 经代码实证成立（详见 §2）；问题集中于失败臂语义、用户可见面登记与边界声明 |
| 实现合理性 | **基本可靠；零 P0；P1×1＋P2×7** | 锁面干净（三锁短临界／无跨 await／无嵌套锁序／RAII）；主路径（take-out→截断→persist→写回、restored_recall_log 三路径带回、空会话落盘例外）全对；缺陷集中在失败臂与竞态窗口 |
| 符合性 | **零不实表述** | 批档 §4 读数四项全部精确复现；钉 16/17 枚全真实存在且断言力总体强；契约三件完备且与 prompt_submitted 真同形；零夹带、−9 删除行全部对位；偏差全为 P3 文档滑差 |

与 0dh 审查（299 批）对照：本轮**无 P0 结构性缺口**（0dh S2-R 会话级 terminal backend 恰为 0di
队列归属铺平道路）；遗留面集中在「失败臂/竞态窗口的收尾纪律」与「用户可见面的诚实登记」。

## §2 设计合理性（P2×3＋P3×4）

**成立面（代码实证）**：①ext_method 与在途 run 确为并发处理（acp 库每请求独立任务，queue_append
运行中可达）；②recall×restore×run 三方互斥在 runs 表**同一把锁内原子 check+insert**，单进程内
完备；③loop-top drain 位置确在压缩/检查点算术之后、模型面装配之前（`agent_loop.rs:3224`）——
「补充永不进入压缩区间」成立，尾追追加不破坏 I2/I6 前缀缓存，批档「RequestHeaderChange 只指纹
system+tools」评估正确；④契约姿态与 prompt_submitted 真同形；⑤grill 显式槽是正确机制（三构造子
main=true/grill=false/retrieval=false 全仓仅此三处）；⑥跨 run 存活链路成立（build_host 每 prompt
查表注入同一 Arc）。

- **[P2-R6] 失败/取消 run 吞掉已投递补充，可见面缺口未登记**：D2 登记了 SUCCESS-ONLY 机制语义，
  但用户面没登记——用户看到「补充已生效」卡→run 失败/被 Ctrl+Z→补充从上下文消失且无任何提示；
  且 cancel 不清队使「未投递残余」存活而「已投递者」蒸发，同批补充两种命运。S4 四条判据均不覆盖。
- **[P2-R7] 队列上限是传输面不是预算面**：每条 256KiB×深度 16＝满队 4MiB；本轮 token 估算与
  阶梯判定（`agent_loop.rs:2523` 一带）先于 drain（`:3224`）且投后不复算——极端时单轮注入可推过
  500K 硬截断/700K 保险线，提供方报错→run 失败→已投补充随 SUCCESS-ONLY 蒸发（与 R6 复合成
  「补充全部丢失」）。「永不进入压缩区间」≠「不进入上下文预算」，设计稿未登记此边界。
- **[P2-R5] recall 菜单的 ordinal 空间被机械 User 消息污染**：`[任务状态` 状态行按前缀缓存纪律
  以**尾随 User 消息**持久化（`controller.rs:1520`；未注册 `is_injected_block_text`，写回过滤
  `controller.rs:4506` 放行），压缩 marker/清零 marker/白名单块更被 `is_restore_retained_block`
  **有意保留**——而 `handle_recall_preview` 仅按 `role == Role::User` 过滤（`acp_server.rs:4129`）。
  ⇒ `/rewind` 菜单列出用户从未见为消息卡的条目，选中 marker 会把机械文本当原文回填输入框；
  「菜单制=CC 同形」的用户体验承诺落空（截断语义本身自洽、preview/recall 同一真相面无错位）。
- **[P3-R19a] 跨进程同会话并发不在互斥承诺内**：`session/new` 服务端强制 UUIDv4＋`session/load`
  未实现＋web 泵每连接独立子进程 ⇒ 正常拓扑不可达，但 §4 边界未登记进程域。
- **[P3-R19b] grill 轮不注册 runs 表**：recall 可在 grill 进行中截断主会话（grill 自有 history 面
  且不 drain，实害有限）；§4.4「grill 不接队列/撤回」未登记此并发态。
- **[P3-R18] 设计稿三处表述漂移**：D4 `recall_chars` vs 实现 `remaining_messages`（批档与代码
  一致，设计稿漏更新）；D5「新→旧排序由客户端倒排」两端均未做且 TUI 默认选最旧（`selected:0`）/
  web 默认选最新（`entries.length-1`）互相不一致；D4「生产无 marker 侧风险」括注与 restore-retained
  marker 有意持久化的事实不符。
- **[P3-R20] S4 判据覆盖面不足**：双端/跨入口并发（R4 的 TOCTOU 场景）、失败 run 补充可见面（R6）、
  撤回×排队竞态、菜单机械块（R5）均无判据——设计承诺的「投递可见面」「撤回后上下文一致性」在
  真机判据里只有单客户端正常路径。

## §3 实现合理性（P1×1＋P2×4＋P3×8）

**成立面（亲验）**：锁面（runs 表互斥同锁 check+insert＋RAII 全路径释放；prompt 仅拒 Restore、
prompt-vs-prompt 保持 overwrite 语义＝与设计一致）；loop-top 投递序（事件 record 失败 `?` 早退＝
事件→消息原子；supplement_seq 函数作用域随 run 复位；drain 在压缩算术后/`sync_status_line_message`
前/请求视图装配前）；队列生命周期（StoredSession 唯一构造点持 Arc、build_host 注入同一实例；
256KiB 计字节、character_count 计 chars）；recall_log 存续（成功写回带回/失败侧车兜底/重启读盘）；
ext 线格式（acp 0.10.4 实源 `format!("_{}")` 出站＋`strip_prefix('_')` 入站，TUI/web 双面正确，
桥透传无阻）；契约面（event.rs snake_case→registry v02 条目/schema/fixtures 三件/v02 enum/
Python 判官 registry 派生零改动生效）。**零夹带**（24 文件逐一过 diff）；−9 删除行五组全部为
特性接线有替代取代（空跳过守卫→recall_log 例外；runner 拒绝臂→恢复拒＋运行排队；web flash 拒绝→
排队臂；帮助计数 11→12；纯注释两行）。

- **[P1-R1] recall 失败臂静默销毁已排队补充**（设计、实现两路独立命中，主会话亲验）：
  `acp_server.rs:4190` 的 `queue.clear()` 执行于 ordinal 定位（`:4203` miss 臂）**之前**——越界时
  continuation 确实原样放回（符合批档「失败不清会话」），但队列已被清空且不归还：丢弃数不进错误
  消息、不写 recall_log，客户端无从得知。批档自述「fail-closed」，fail-closed 在会话维度兑现、
  在队列维度破例；用户刚收到「已排队（第 N 条）」回执的输入一次失败撤回中无痕蒸发。触发窗口窄
  （对话框只提供有效 ordinal；主要靠 raw ext 调用或并发 recall 后的陈旧 ordinal），但违背本批
  自述契约＋不可逆数据丢失，裁 P1。修法一行＝`clear()` 挪到定位成功之后（与 persist 同序）。
- **[P2-R2] recall 与 queue_append 无互斥**（亲验）：`handle_queue_append`（`acp_server.rs:4086`）
  不查 runs 表——recall 执行中并发到达的 push 落在 clear 之后即存活，进入撤回重开分支且不计入
  `dropped_supplements`，正面违反 D4「撤回重开的分支不得注入陈旧意图」。多线程 runtime 下两任务
  真并行，微秒-毫秒窗口。修法＝queue_append 顺手查 runs 表、`RunInFlight::Restore` 在册即拒绝
  （一行改动，同时消掉「restore 期间可入队而 restore 不清队」的不对称）。
- **[P2-R3] recall 落盘失败被吞**（亲验）：`persist_conversation_sidecar` 返回 `()`，目录创建/
  序列化/`fs::write` 失败一律 `tracing::warn!` 后返回（`acp_server.rs:705-723`）；recall 随后照常
  写回内存并返回 `recalled:true`（`:4230` 调用点不感知）。落盘失败（本批就真实发生过 D 盘满，
  302 批档 §5）→重启复活已撤回尾巴——正是 D4「立即落盘防崩溃复活」要防的场景。warn-only 沿袭
  run 写回臂惯例，但 recall 的正确性声明单独依赖这次落盘。修法＝recall 臂感知 persist 失败
  （改返回 `Result` 或探测文件 mtime），失败时响应降级或回滚写回。
- **[P2-R4] recall_preview 无 idle 门且 doc 注释与实现不符**（亲验）：doc 写明「run/restore 在途时
  拒绝（撤回与提示同锁互斥）」（`:4114`）但函数体无任何 runs 表检查；web `/rewind` 无前端运行守卫
  （`main.js` runSlash 直通 `dialogs.openRewind()`），运行中可打开对话框看到**陈旧清单**（run 在途
  时 continuation 被 take-out，preview 回退侧车＝上一成功 run 快照），点撤回才被 recall 的 idle 门
  挡下——fail-closed 最终无害，但违背 D5「运行中两入口均被服务端 idle 门拒绝并如实回显」；TUI
  双侧有守卫（dispatch_command＋rewind_confirm 复查 running）。陈旧 preview 的 ordinal 按位置语义
  在尾部追加会话下仍指向同一消息文本（无错位截断），危害限于误导＋R4′ TOCTOU：preview 后会话若
  经并发 recall 增长，旧清单确认可能截错（窗口＝人工确认耗时与另一 recall 的赛跑，web 确认臂
  不比对 recall_text 与预览所见）。
- **[P2-R8] TUI 投影去重与自身注释矛盾**（亲验）：`projection.rs:94-115` 注释称「去重键含序数
  避免同文补充被折叠」，实现纯按 `m.content == text` 去重、`let _ = supplement_seq;` 显式丢弃
  ——同文补充（或补充恰同原 prompt）折叠进旧卡、marker 指向旧行；web 侧（`projection.js:53-58`）
  无去重逐次加卡，双面行为不一致。纯展示层，不改会话真相。
- **[P3-R9] close_session×recall 归档竞态**：recall 持 `Restore` 标记时 close 立即
  `spawn_archive_task`（`run_in_flight` 只认 Prompt，`acp_server.rs:4265-4271`），归档从磁盘读
  侧车与 recall 的 persist 赛跑（可能打包截断前内容）；close 无差别摘除 runs 标记（含 Restore）
  使 guard Drop 变 no-op——互斥破口因会话行已删不可达，仅归档内容竞态；restore 先例已有同形状。
- **[P3-R10] stdio 面 ext 错误码坍缩 internal_error**（-32603）：SessionNotFound/容量/busy/越界
  经 `into_internal_error` 全并入，原因文本落 data（双前端可见）；与现役 prompt 路径同构、符合
  「现役错误面」承诺，InvalidRequest 语义未上映射到线格式。
- **[P3-R11] TUI runner 守卫次序耦合**：`runner.rs:381-395` `running` 先于 `grill_active`——grill
  轮期 `app.running=true`，若可达则 grill 回答会被误排队为 supplement（违反 §4.4）；现不可达仅因
  grill 内联 await 阻塞按键循环＋`/grill` 入口运行守卫，两处任一改动（如 grill 异步化）即引爆。
  建议注释钉住次序依赖或调换判断。
- **[P3-R12] 排队回执文案过强**：「将在当前运行的下一次模型调用前投递」（`main.js:84-87`／
  `runner.rs:420-423` 同文）在 run 已末轮/随后 cancel 时失实（实际落下一 run 首 loop-top 或滞留）；
  跨 run 存活机制本身正确，仅文案过强。队列 pending 深度亦无常驻可见面。
- **[P3-R13] web 撤回行省略号判定**：`entry.character_count > entry.preview.length`——服务端
  chars vs JS UTF-16 码元，含增补平面字符（emoji）时漂移。纯外观。
- **[P3-R14] recall_text 无条件覆盖输入框草稿**（`runner.rs:445`／main.js `ui.fillInput`）：CC
  同形行为，可登记不取；建议至少 flash 提示。
- **[P3-R15] valid fixture 字符数失配**：`runtime/fixtures/run-event-v0.2/payloads/
  supplement-delivered.minimal.valid.json` `character_count:12` 而 text 实为 13 chars（亲验）；
  schema 无跨字段约束故校验通过、判官接受，但与契约「character_count=text chars」自相矛盾，
  误导后续对拍。
- **[P3-R17] 钉面弱项**：loop 钉 `supplement_queue_survives_run_and_drains_next_run` docstring 与
  批档声称「supplement_seq 随 run 复位」但测试体无 payload seq 断言（复位由局部变量结构性承载），
  另留死变量 `queue`/`let _ = &queue;`；单条 256KiB 字节帽拒绝已实现（`SUPPLEMENT_MAX_BYTES`）
  但无测试钉（批档只声称空文本/深度帽，无虚报）。

## §4 符合性（零不实；偏差全为 P3）

**读数复验（全部精确复现）**：

| 批档声称 | 实测 | 判定 |
|---|---|---|
| orz-host 串行 382/0（基线 374+8） | 382 passed; 0 failed | ✅ |
| orz-loop 串行 886/0（基线 883+3） | 886 passed; 0 failed | ✅ |
| orz-assurance 303/0＋集成 9/0＋4/0 | 同左（另 doctest 1/0） | ✅ |
| frontend_smoke 全过含 §13 三节 | 全过（`ok 0di ①②③`） | ✅ |
| 24 文件 +1861/−9 | `--stat` 逐位精确 | ✅ |

**钉子 16 枚逐枚核验**：全部真实存在，断言力总体强。关键易虚报处均为真断言——「越界不清会话」
有续接包长度复核（钉 4）；「第一段不武装」显式断言 `pending_rewind` 仍 None（钉 12）；「空会话
落盘」断言侧车 `messages=0`＋`recall_log=1`（钉 7）；「排队臂取代拒绝臂」断言旧拒绝文案退役
（smoke §13②）。弱项两处见 R17。
**契约面**：registry v02 条目与 prompt_submitted 真同形；schema 逐字符合（seq≥1/count≥1/text
minLength 1/additionalProperties:false）；fixtures 三件齐；v02 enum 含 `supplement_delivered`；
Python 判官 registry 派生视图零改动自动生效。D8 S3 字节判据全部字面进体。
**批档 §2 抽核**：cancel 不清队（全仓 `supplement_queue.clear()` 仅 recall 一处）；越界 fail-closed
原样放回；restored_recall_log 转移三路径无丢失；空会话落盘例外；三方互斥 guard 结构——均经代码
与测试双重证实。
**偏差（全 P3）**：[P3-R16]「钉 16 枚」计数口径打架——按 host 分号列举项计 7+3+3+3=16，按测试
函数计 host 8 枚共 17；读数基线（374+8/883+3）与实测一致，纯笔误；批档 §6 S3 字节判据
`drain_user_supplements` 重复写两次（同笔误级）。设计稿 §3 三子项（assurance 对拍钉、recall×restore
专钉、continuation None 回退钉）未落成专属测试、由既有机制承载——批档未虚报，属设计稿→批档钉
清单口径收窄未声明（并入 R19/R17 登记）。

## §5 问题清单汇总（0dj 立项面；编号全批统一）

**P1×1**：R1 recall 失败臂静默清空补充队列。
**P2×7**：R2 queue_append 与 recall/restore 无互斥；R3 recall 落盘失败被吞；R4 recall_preview 无
idle 门＋doc 失实＋web 无前端守卫；R5 菜单 ordinal 空间机械块污染；R6 失败 run 吞已投递补充
可见面缺口；R7 队列上限非预算面；R8 TUI 投影去重矛盾。
**P3×12**：R9 close×recall 归档竞态；R10 stdio 错误码坍缩；R11 runner 守卫次序耦合；R12 回执
文案过强；R13 省略号 UTF-16 漂移；R14 recall_text 覆盖草稿；R15 fixture 字符数失配；R16 钉计数
口径；R17 loop 钉 seq 断言弱＋字节帽无钉；R18 设计稿漂移三处；R19 进程域/grill 并发态未登记＋
recall×restore 无专钉；R20 S4 判据覆盖面不足。

**修复裁量建议（供 0dj 处置批参考，细则随裁决）**：
- **修码类四件**（R1＋R2＋R3＋R4）全在 `acp_server.rs` 同一区域（R2 含一行 runs 表检查），可
  合成一个小修复批；R8 展示面一行（去重键并入 supplement_seq）可随车。
- **登记类**（R5 处置二选一＝preview 侧过滤机械块或菜单行标注＋设计稿 D4 补登记；R6 失败收口
  补用户可见系统行＋S4 判据增补；R7 边界登记或 drain 后守卫复算；R19 进程域/grill 态登记）。
- **文档勘正类**（R15 fixture 改 13；R16 计数口径注记；R18 设计稿三处）。
- **P3 其余**（R9–R14）随裁决定去留，多数可登记不取。

## §6 台账

- 本档；索引头行 v4.275 → **v4.276**（头行＋AUTH-MESSAGE-RECALL 条目补审查达成注记＋新条目
  AUTH-0DI-COMPREHENSIVE-REVIEW）；BACKLOG 本批指针/计数行（**65 → 66**）/P1 总览行/P1 节
  开放项锚行补 `0dj`/`0dj` 专节＋`0di` 专节补审查达成行；TODO 计数行/P1 路由行/`P1-0dj` 新节；
  第二卷 §1.243。
- **门禁补正三项（随批如实记）**：①BACKLOG P1 节「开放项：」锚行补 `0dj`（初版漏改致 ledger
  consistency 两误报）；②四处台账行超长随批收缩（TODO:20/22＋BACKLOG:13/45，终态 1200/1197/
  931/1194 ≤ 1200）；③`orz_source_manifest.sha256` 重算 **1,488 entries**——勘正 **303 bump 批
  遗留漂移**（Cargo.lock＋crates/orz-bin/Cargo.toml 两文件 digest 失配；bump 批只动 orz 仓未随批
  重算，本批补账）。补正后门禁 **valid: true（error_count 0）**。
- orz 仓源码零改动（本批 manifest 重算只为吸收 bump 遗留，orz 指针维持 `01386744`）。
- **处置待用户裁决**：修复批（修码四件＋R8）是否下一批发起、登记类各项去留、S4 判据增补采纳。
- 0di 主线批序不变：S3 载体重建（与 0dh S3 同车，字节判据冻结不变）→ S4 真机。

## §7 关键词

304 批、0di 全面审查、三路并行只读深查、设计合理性、实现合理性、符合性、P1-R1 recall 失败臂
清队、queue.clear 越界、fail-closed 破例、R2 push-after-clear、R3 persist 吞错、R4 recall_preview
idle 门、R5 机械块污染菜单、任务状态尾随消息、restore-retained marker、R6 SUCCESS-ONLY 可见面、
R7 传输面非预算面、R8 投影去重、0dj、GAP-0DI-REVIEW-FIXES、65 → 66、索引 v4.276。
