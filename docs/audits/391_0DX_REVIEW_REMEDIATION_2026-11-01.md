# 391 批：0dx 全面审查处置——P2×7＋P3 批全数落码（CONTEXT-CHECKPOINT-ERA 审查批）（2026-11-01）

> 日期：2026-11-01（用户令「请直接对审查出的全部问题进行全面处理」）
> 性质：审查处置批——2026-10-31 三子代理分端审查（触发面／折叠投影面／事件账面）发现的 7 项 P2＋全部 P3 一次性处置；0dx 计数 **71 不变**（S3 进体随下次重建、S4 真机核证随其后不动）。
> orz 子模块：390 落码基线 `b8f9ebc4` 之上纯落码（本批新提交），零 bump、零重建、零换装。
> 父仓随批件：assurance Python 判官对齐（`_verify_v02_context_checkpoint_era` 缺键/负数两破口）。

## §0 结论

审查发现全数处置：**P2×7**（在途带参 /checkpoint 静默蒸发、词界 CJK 紧邻不设防且两表面分叉、reasoning_content 不入档、块尾组口径违 P0 禁令、折叠-失败非事务窗口、corpus 单调性负例名存实亡、判官同形三破口）＋ **P3 全部**（ext 参数离源、预算回退重折、ext check-then-act 空窗、跨清零边界块跳过、digest 时点、drain 中性拒绝无审计、时间戳双取、空 /btw 耗 run 序号、era_number_in 前缀计数、TUI 词界复刻、判官死代码/文案）。零设计语义变更（处置均在设计 §1 授权范围内；一处落账时序收敛见 §2-⑤）。家族性前缀污染面（清零/压缩 marker 同款暴露）**不扩批**，随 390 批登记继续在案。

## §1 处置清单（P2）

### ①在途 `/checkpoint <随行输入>` 静默蒸发（审查 P2-1）

enqueue 门原只拦裸命令（`Checkpoint(None)`），带参项入队后到 loop-top 被 `reject_checkpoint_inflight` 连文本丢弃、无用户面提示——设计 §1.1「不受理，返回提示」违约。处置：`handle_queue_append` 拒绝扩至 `Checkpoint(_)` 全形态（裸/带参/全角冒号拖尾判空形态同拒，带区分性文案）；loop 内 GateDecision 拒绝保留为兜底。钉：`queue_append_rejects_checkpoint_command_forms`（host，三拒绝形态＋机械位放行）。

### ②词界 CJK 紧邻不设防＋TUI/stdio 两表面分叉（审查 P2-2）

`strip_word` 词界只认空白：`/checkpoint修一下` 判 Plain（stdio/web＝意外换代且原句灌为新时代首条消息）、TUI 落未知命令——同输入两表面行为相反且均非用户意图。处置：词界扩围——后随**紧邻标点**（全角 `：，。；！？、`＋ASCII `:` `,`，剥除前导标点）或**任意非 ASCII 字符**（`/checkpoint修一下`＝带参切换）都算边界；依据＝代价不对称（漏判⇒整窗折叠，扩判只是标点进 rest；斜杠命令注册表无非 ASCII 后缀成员）。TUI `input.rs` 预过滤同规则改写（裸命令含拖尾标点判空形态归一化进命令层）。ASCII 字母数字紧随仍 Plain（`/btwxyz`、`/checkpoint2`）。钉：`input_intent_prefixes_are_word_bounded` 扩用例（era.rs）＋`era_prefix_boundary_matches_host_intent_parsing`（TUI，与 era.rs 用例清单镜像——任一侧改规则必须同步两侧）。

### ③`reasoning_content` 不入时代档案（审查 P2-3）

「无损整段折叠」保真缺口：归档只写 role/tool_calls/call_id/content。处置：`era_archive_markdown` 逐字补写 `[推理内容]` 节（空缺如实缺节）。钉：`era_archive_markdown_preserves_reasoning_and_tool_calls`。

### ④`post_fold_blocks` 尾组口径违 P0 禁令（审查 P2-4）

尾组写 `ranges.len()-1`（含未完整尾轮入块号分组），正是 `model_face.rs` 2026-09-16 P0 修复注释明令禁止的写法——台账定位行 seq 区间与读时分块表分叉。处置：照禁令记真正累计到的 `cur_last`。台账补挂同时改**逐轮**清零判定（见 §2-④）。

### ⑤折叠→run 失败＝非事务窗口（审查 P2-5，两名子代理独立汇聚）

四层收口（同一根因＝宿主 cell 初值快照与 controller 内部态双源漂移＋折叠面不即时持久）：
1. **统一账面**：新增 `era::EraRunState{usage, pending_era_event}`＋`SharedEraState`（单一 `Arc<Mutex>`）——宿主检查点 sink 闭包与 controller（drain 折叠清零换元、`run_started` 落账取走待命载荷）读写同一份；`with_era_usage` 注入类型随之更替，controller 内部 Mutex 降级为裸测试回退态。
2. **drain 折叠后即时 flush**：loop-top 折叠＋落账后立即调 `checkpoint_conversation_sidecar`——marker/清零后账面就此落盘，本 run 此后任何时点失败（含首个 0dq sink 写沿之前）侧车都已是折叠后状态；重折叠重复挂行与「era2 消息＋era1 账面」混合态窗口闭合。
3. **idle 带输入折叠后即时持久化**：折叠成功先写续接包再拉起 run（裸 `/checkpoint` 早返写回本就有；带输入路径为 390 缺口）——「侧车回退折叠前、marker 丢失而 journal 已有时代事件」窗口闭合。
4. **落账恰一次**：`journal_pending_era_boundary` 取走清的是共享态（sink 与 run 尾同步可见）；host 失败臂新增——携带待命载荷的 run 失败时按共享态**现值**持久化运行前会话面（已落账⇒待命位已清＝防下个 prompt 重复落账〔era 号会话内非递增/journal 重复事件〕；未落账〔journal 写失败即败〕⇒待命位保留＝下 run 重试，恰一次语义两头保持；快照仅载荷在途时付出拷贝代价；SUCCESS-ONLY 纪律不变）。

### ⑥corpus「六违例」名存实亡（审查 P2-6，本代理直接核证）

违例场景注释声称「era 不递增 3→2」，实际数据 era=0→2——严格递增规则仅因 Rust 对 era=0 双报而意外触达、boundary_round 不回退**全程零负例**；390 批档「六违例」表述失实（勘误随本档）。处置：场景重构为**三事件七违例**——事件 0＝era 0（≥1 破）＋trigger 越集＋digest 空串＋usage 缺键（四违例；<1 不进 prev）；事件 1＝正形锚（era 3/轮 5）；事件 2＝era 2（3→2 递增破）＋轮 4（5→4 回退破）＋boundary_at 缺失（三违例）。corpus 场景计数 264 不变（仍两 era 场景）。

### ⑦Rust↔Python 判官三处同形破口（审查 P2-7）

布尔对拍（`errors.is_empty()`）掩盖错误集形状漂移。处置：
- **Rust prev 守卫对齐 Python**：era/round<1（或不可解析）只报形状错、不进 prev 推进也不触发单调错（原 era=0 双报「must be >= 1」＋「strictly increasing (previous 0)」）；
- **Python 对齐 schema/Rust**：features 六数字键**缺键即错**（schema required 在案；null＝影子未启用/legacy 仍合法——不落假值与缺账自此可分）；usage 两键拒**负整数**（schema `minimum: 0`；bool 为 int 子类显式排除）；
- **对拍升级**：era 族加**错误计数对拍**（Python 脚本回传 `#era_count`，Rust 侧逐 corpus 场景断言两侧错误条数相等）——该族规则集自此逐条钉死，形状漂移不再可 invisible 通过。其余族布尔对拍维持（全族计数对拍会放大历史良性漂移，不扩批）。

## §2 处置清单（P3）与语义钉

1. **ext 参数离源**：`handle_session_checkpoint` 硬编码 160K/32K → `orz_loop::{production_slider_tokens, production_block_tokens}` 生产解析单源（env 覆盖优先、缺省同 `ContextCompactConfig::default`，与控制器构造同一解析；lib.rs re-export）。390 批档 §2 的如实登记自此消除。
2. **预算回退重折（一次排队输入双换代）**：`SupplementItem` 增 `era_folded` 位——drain 折叠成功后置位随项携带，延迟队列重试不再重折；复验回退路径同样保位。入队构造恒 false（对既有生产者零影响）。
3. **ext check-then-act 空窗**：`RunInFlight` 新增 `Checkpoint(u64)` 变体＋`CheckpointInflightGuard`（与 Restore 同构 RAII、id compare-and-remove）——查 runs 表＋占位原子化，折叠窗口内 prompt/restore/再次 checkpoint 一律 Busy；`handle_queue_append` 对 Checkpoint 占位同拒（与 Restore 同待遇）。钉：`session_checkpoint_ext_busy_marker_and_release`。
4. **跨清零边界块整块跳过补挂**：台账补挂改逐轮 `is_round_cleared` 判定（边界后段从未挂行的轮次照挂）；零行块不进台账定位清单（marker 无空区间）。
5. **digest 时点**：digest 改对**脱敏后**全文计算（`write_archive_retry` 落盘前统一脱敏，原 digest 对脱敏前原文＝与文件字节永久失配、复核不可为；脱敏幂等，写入侧二次脱敏零失真）。
6. **drain 中性拒绝无审计**：折叠拒绝臂增 `GateDecision{gate: era_supplement_intent, decision: fold_refused_same_era, reason}`（与空 /btw、在途 /checkpoint 审计对称）。性质＝恢复边界防御臂：常规 drain 序中折叠区间恒含本 run prompt（非 marker 尾），守卫不触达；连排无前缀项语义见下钉。
7. **时间戳单源**：`era_marker` 增 `boundary_at` 入参——marker「时代边界时间戳」行与事件载荷 `boundary_at` 同一瞬时（原两次 `chrono_utc_now()` 微秒级漂移消灭）。钉：`era_marker_and_payload_share_boundary_timestamp`。
8. **空 /btw 耗 run 序号**：判定移到 `prompt_count` 自增之前——机械回执零序号消耗（390 批档 §2 诚实登记的 RUN 空洞消灭，「不消耗」名实一致）。钉：`empty_btw_receipt_preserves_run_number`（含后续真实 prompt 仍取 0 号）。
9. **era_number_in 前缀计数**：改全形判定（前缀＋「时代边界」行可反解，`era::is_era_marker`）——仿造前缀文本不得膨胀时代号（同号档案覆盖风险）；防空折守卫同步全形。钉：`fold_refuses_empty_and_marker_tailed_conversations` 扩仿造前缀不拒案。家族性残留（用户构造**全形** marker 文本劫持投影终点/恢复保留）＝清零/压缩 marker 同款暴露，390 批已登记，不扩批。
10. **连排无前缀项语义钉**：`era_consecutive_plain_items_each_open_their_own_era`——每条无前缀排队项各开一个时代（设计 §1.1 字面语义：条目一开时代 N+1，条目二抵达时条目一已在窗内、随时代 N+2 折叠归档；era 号 1→2 严格递增、两条照投）。审查报告所引「第二条起被守卫拒」经实测证伪——守卫仅 marker 尾触发，drain 常规序不触达。
11. **判官装饰性清理**：死变量 `seen` 移除；era 族 format 串长空串清理。

## §3 测试面（全绿）

1. **era 执行核**（`era.rs`）：词界扩围 5 新案＋仿造前缀不拒＋归档保真（reasoning/工具调用逐字）＋时间戳同源；`6/0`（套件内）。
2. **drain 分流**（`controller.rs`）：连排无前缀项各开一时代（2 era 事件、era 1→2、全形 marker 恰二、两条照投）；既有 `era_queue_items_split_at_loop_top` 零回归。
3. **宿主**（`acp_server.rs`）：入队门三拒绝形态＋机械位放行；空 /btw 序号保持（0 号不被吞）；ext 占位 Busy＋释放恢复。既有 `checkpoint_survives_tool_round_when_run_fails_before_terminal_persist`／侧车 round-trip／连续流 /btw 显式化诸钉零回归。
4. **TUI**（`input.rs`）：词界镜像钉（直通/命令层/未知命令三分面）；ACAF 环境预存 9 败口径不变（382 §3.2）。
5. **判官对拍**（`families.rs` ↔ Python）：`s2b_family_verdicts_match_python` 全绿——布尔对拍＋**era 族计数对拍**双绿；`family_verdicts_match_spec_table` 全绿；corpus 计数锁 264 不变。Python assurance 套件 `pytest -k "journal or era or checkpoint"` **109/0**。

**读数**：orz-loop **982/0/3**（390 基线 979＋新钉 3：词界/保真/时戳在 era.rs＋连排钉在 controller——净增计 era.rs 3 新钉中 2 新增 1 扩充）；orz-host 串行 **459/0/6**（基线 456＋新钉 3）；orz-tui `--lib` **186/9**（基线 185/9：+1 词界镜像钉；9 败＝ACAF fail-closed 环境预存口径）；orz-assurance lib 全绿（含对拍）；clippy 四 crate 持平；fmt 五 crate `--check` 净。零重建零换装（S3 载体进体随下次重建批，0.9.14-5 在役容忍沿 385 停栈口径）。

## §4 边界与不做清单（本批）

- 0dx 开放项不动：S3 载体进体随下次重建批；S4 真机核证判据不动（逐时代协议触达数对历史衰减曲线，先收数后定阈；依赖 0dw 对账面读数）。
- 家族性 marker 前缀污染面（清零/压缩/时代三族同款的全形构造暴露）继续在案不扩批；era_number_in 全形判定已消 0dx 特有的时代号膨胀面。
- 其余族（非 era）布尔对拍维持——全族计数对拍会暴露历史良性漂移，属独立工程，不在本审查处置范围。
- Web 前端零改动（本批未触 bridge/协议面）。
- 失败 run 已计费用量不入时代账（SUCCESS-ONLY 同疲劳档位语义）维持 390 如实登记口径：run 级 `run_usage_metered` 仍在 journal 可对账。

## §5 入口

- 设计权威 [`CONTEXT_CHECKPOINT_ERA_DESIGN v1.1`](../CONTEXT_CHECKPOINT_ERA_DESIGN_2026-10-30.md)／立项档 [`0DX_*_REGISTRATION`](0DX_CONTEXT_CHECKPOINT_ERA_REGISTRATION_2026-10-30.md)／390 落码批档 [`390_*`](390_0DX_S2_ERA_IMPL_2026-10-31.md)。
- 实现：`orz/crates/orz-loop/src/era.rs`（执行核＋EraRunState＋词界）／`agent_loop.rs`（drain 分流＋era_folded 位＋折叠后 flush）／`controller.rs`（统一账面接线＋失败臂收口）／`model_face.rs`（era_marker 时戳入参）／`compact.rs`（生产参数单源）／`orz-host/src/acp_server.rs`（入队门/占位守卫/即时持久化/失败臂）／`orz-tui/src/input.rs`（词界同步）。
- 判官：`orz/crates/orz-assurance/src/journal/families.rs`（prev 守卫＋corpus 七违例＋计数对拍）↔ `assurance/run_event_journal_validation.py`（缺键/负数对齐）。
- 台账：TODO 391 行／BACKLOG 本批记录指针／索引头 v4.351。
