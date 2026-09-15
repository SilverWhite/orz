# 过夜四联批全面审查（0ac ①-b 收尾 / 检索侧补强 G1–G4 / 0ae 上下文软门 / 0af·0ag 与账本对账）

> 状态：`reference`（只读深度审查；零代码改动、零账本外文件改动；四批均未动，处置全部留用户裁决）；日期 2026-09-15。
>
> 审查对象：orz 子模块 `feat/fusion-architecture` @ `8512fc71` 上 2026-09-15 过夜四联批四笔提交——① 0ac S3①-b 收尾批 `1deeba75`、② 检索侧补强 G1–G4 `7e151ed1`、③ 0ae 上下文软门与模型参与压缩 `f0040557`、④ 0af（后改名 0ag）契约面机械对账钉子 `8512fc71`——外加 0af（GAP-RESOURCE-GATE-DENIAL-COPY，pending 未实施）设计定案与三账本对账面。设计权威：[`IMMEDIATE_RESULT_DELIVERY_AND_STREAMING_RETRIEVAL_DESIGN_2026-09-13`](../IMMEDIATE_RESULT_DELIVERY_AND_STREAMING_RETRIEVAL_DESIGN_2026-09-13.md) / [`RETRIEVAL_LOCAL_SEGMENTED_HARDENING_DESIGN_2026-09-15`](../RETRIEVAL_LOCAL_SEGMENTED_HARDENING_DESIGN_2026-09-15.md) / [`CONTEXT_SOFT_GATE_MODEL_PARTICIPATED_COMPRESSION_DESIGN_2026-09-15`](../CONTEXT_SOFT_GATE_MODEL_PARTICIPATED_COMPRESSION_DESIGN_2026-09-15.md)。
>
> 方法：四路并行子代理深审（每路＝设计稿逐条 → `git show` 批次 diff → 当前文件逐点对账 → 可红性/测试面核对），主会话对五处承重发现**亲手逐点抽查坐实**（M1 接线、D3 窗口工具面、b_algo 解析器复用、`mechanical_audit_update` 枚举、`admit` 唯一调用点），另有门禁复跑。审查基准＝项目自有纪律：fail-closed、稳定码闭枚举、journal 单 writer、开关默认值、账本主张须与代码一致（「符号在位≠接线」教训）。
>
> 评级：P0＝阻塞性缺陷；P1＝必须修；P2＝应修；P3＝注记。类别三轴：设计合理性 / 实现合理性 / 符合性（设计与实现、账本与代码）。

---

## 0. 一句话结论

四批的机械件与账本纪律总体扎实（M2 投递、G2/G1/G3 落码、D0/D1/D4、0ag 钉子、先占改名治理链均经核实为真；门禁复跑 `valid: true / 1450 条 / error_count: 0`），**但「符号在位≠接线」本项目自有教训在本夜复发三处，其中一处构成 P0**：单测绿＋门禁绿盖不住接线级缺陷——0ac M1 收尾注入永不可触发、0ae D3「模型参与压缩」窗口结构上不可能成功（`model_participated` 恒 false）、检索探针 proxy 读数未接线；同夜 0ag 钉子落地的时刻，新契约漂移在钉子未覆盖的枚举上复发（`mechanical_audit_update` schema kind 越界，首次阶梯触发即产生 schema-invalid journal）。账本两处主张须附加限定：「0ac S3 出口条件达成」（M1 实为不可达代码）、「0ae 全部代码面完成」（D3 实为窗口结构在、参与机制断）。

发现计数：**P0×1 ＋ P1×7 ＋ P2×8 ＋ P3×25**，全部附 file:line 证据，见 §2–§5。

## 1. 主会话亲手坐实的五处承重发现（证据等级最高）

1. **M1 永不可触发**：全仓唯一 `delivery_queue.admit` 调用点在 B1 块内（`agent_loop.rs:3273`），其后同块内立即 `due()`——`due` 用 `mem::take` 把整个队列全量取出（含未到期者降级 I3 digest），`immediate_delivery.rs:459-478` ⇒ 每 B1 间隙结束队列恒空 ⇒ 终答分支 `!delivery_queue.is_empty()`（`agent_loop.rs:2392`）恒假。
2. **D3 窗口自我否定**：`pending_keeps_tools` 只匹配 `Orientation`（`agent_loop.rs:1639-1641`）⇒ ModelCompression 窗口轮 `current_tool_defs = Vec::new()`（:1684-1691），模型看不到 blackboard_write 声明；消费分支只保留 text、tool_calls 无条件丢弃不派发（:2199-2255）⇒ `participated = writes_now > window_start_writes` 恒 false。
3. **代理默认链前两引擎恒空**：`fetch_serp` 对所有引擎统一 `parse_bing_serp`（`local_segmented.rs:959`），BLOCK_RE 只匹配 `li.b_algo`（:391-396，Bing 专有类名）⇒ DDG/Google HTML 恒 0 命中。
4. **schema 枚举越界**：`runtime/mechanical-audit-update-event-payload-v0.2.schema.json` kind 闭枚举＝`{tool_result, plan_gate, budget}`，而 0ae 新写 `attention_ladder`×2 / `model_compression` / `plan_write_guidance`×2（`agent_loop.rs:1304/1333/2243/3357/3387`）——已排除 1906/1955 两处（属 `TransportRetry` 事件族，非本事件）。
5. **探针接线缺失**：`engine_chain_detail` / `proxy_display` 在定义文件之外全仓零消费者（grep 实证）；`retrieval_family` 探针在 `retrieval/projection.rs:138-163` 另行重读 env，无 proxy 字段。

---

## 2. 0ac S3①-b 收尾批（orz `1deeba75`，9 文件 +680/−6）

**总评**：M2 宿侧投递、⑥ 提前收口、③ 跨 run 钉子质量扎实；核心缺陷是 M1 收尾注入接线断裂（不可达代码），且全仓无任何测试会因此变红——交接件摩擦 D 预言的「静默零投递」形态的实际变体。

### 2.1 已核实健全的点

- **开关纪律**：主开关 `ORZ_IMMEDIATE_RESULT_DELIVERY` 未设即关、关时 B1/到达面/tick 全部零行为（`immediate_delivery.rs:53-66`；单测 `switch_truthy_contract_and_default_off` 钉住）。
- **M2 双通道 exactly-once 为真**：`OrzHost::drain_completed_tasks`（`orz/crates/orz-host/src/lib.rs:1869-1891`）经 `ToolBridge::from_parts`（`bridge.rs:699-706`）取 `drain_between_turn_bash_completions`（:648-684），内部 `state.mark_reported` 与 `TaskCompletionReminder` 的 `reported.insert` 共用同一 `ReportedTaskCompletions` 集 ⇒ 无双记/漏记；owner 会话过滤同源；文本单一源 `format_bash_completion` 成立（交接件 §5-E 核对过）。
- **B1 真投递形态**：`delivered_fact_payload` 恒 `suppressed=false` 且不带 `suppressed_reason`（`immediate_delivery.rs:498-510`），与 result-delivered schema `allOf` else 分支吻合；注入位置在全部 tool replies 之后、不插 assistant 声明与 tool replies 之间。
- **⑦ 非设计偏离**：`semaphore acquire 独立截止`已随更早批次 `4c892951` 落码（`orz-host/src/tools.rs:46-59` `ORZ_RETRIEVAL_SEMAPHORE_WAIT_MS` 默认 10s、`0`=禁用；`orz-host/src/lib.rs:1478-1519` 有界 acquire＋`retrieval_lane_busy` 自描述 cause）——本批 M3 tick 只补「排队可见性」残项，交接件摩擦 A 勘误成立；排队最长 10s，不会重回「被 900s 外层包住」的老问题。
- **⑥ 触发条件与账本一致**：`capability_unreachable` 一次即收口且当轮立即 return；连续失败阈值 `ORZ_RETRIEVAL_EARLY_CLOSE_FAILURES` 默认 3、`0`=禁用（`immediate_delivery.rs:360-365`）；检索工具成功（exit 0）清零 streak；角色门仅 Main＋两条检索车道（`orientation.rs:67-71`）；收口用既有闭枚举 `subagent_failed`（`retrieval/dispatch.rs:866`），cause 随错误文本自描述（交接件 §5-D 过）。
- **③ 跨 run 时序钉子三枚在位**：新 run 队列不继承去重状态 / 每条 Delivery 恰一 unsuppressed payload 且键逐字一致 / close_drop 清空且 due 恒空（`immediate_delivery.rs:644-693`），实跑绿。
- **acp 期望 9→11 属实**：三件事件（检索族探针 / `request_header_change` / `host_resource_snapshot`）均为先前已提交批次的合法行为，旧期望 9（2026-08-27 落笔）属过期计数；两 acp 测试在 HEAD 实跑通过。
- **定向测试实跑**：`cargo test -p orz-loop --lib immediate` **14/14 绿**（温热增量，未触发重编译）。

### 2.2 问题清单

| # | 级别 | 类别 | 发现与证据 |
|---|---|---|---|
| 0AC-A1 | **P1** | 实现合理性 | **M1 收尾注入永不可触发**（§1-1）。admit 与 due 背靠背发生在同一 B1 块（`agent_loop.rs:3273→3286`），终答候选分支的队列非空条件恒假（:2392）⇒ `BOUNDARY_B2` 投递、M1 路径 `MODE_DIGEST`/`CLASS_I3`、TTL 120s/轮数 3 降级在接线中**全部不可达**（`overdue_facts_degrade_to_digest` 仅类型级单测，接线永不演练）。功能缺口＝终答（无工具轮）期间完成的后台任务本 run 零投递。缺失半边正是交接件 §4-⑤-B「本轮已是纯文本终答候选而队列仍有未投递事实」所要求的终答处 drain。账本「M1 已接线、每 run 至多一次、boundary=B2」与代码现实不符（`m1_used` 局部变量因不可触发而空洞）。建议：终答候选分支补 drain+admit，或如实降级账本口径 |
| 0AC-A2 | P2 | 符合性/设计 | **⑥ 把 `empty_result` 计入「连续确定失败」**。设计 §3.4 定义空结果＝通道判活的合法 I1 信息；本地分段实现里引擎链合法空转即产出该码 ⇒ 2×`network_no_response`＋1×`empty_result` 也达阈值收口，难查询（合法空×3）被过早杀成 `subagent_failed`。建议 streak 只认 `network_no_response`（或独立计），cause 文案区分「查无可得」与「通道失联」 |
| 0AC-A3 | P2 | 符合性（测试面） | **接线级零测试**：本批 agent_loop/controller/host_exec 0 个 `#[test]`；全 crates 无 `ResultDelivered`/`EarlyClose`/`drain_completed_tasks` 接线断言。B1 块整段删除、M1 永不触发、宿主忘实现 drain 三者均无测试变红（A1 即其变体）。建议补三枚接线钉：fake host 的 B1 投递+journal 断言 / M1 触发一次端到端 / `drain_completed_tasks` 正测 |
| 0AC-A4 | P3 | 设计合理性 | M3 tick 默认 10s 与 acquire 截止同刻，常态排队（<10s）零 tick，「排队可见性」实际打折；且 tick 只落 journal 非模型可见中途回报——设计稿 §4.1 M3 原义被收窄为观测面，交接件 §5-C 按窄口径登记但设计稿未回写（文档滞后） |
| 0AC-A5 | P3 | 符合性 | 交接件给⑥列的第三触发「结果已形成」既未落码也未挂账为未竟项——静默缩围（账本主张只写前两触发，与代码一致、不虚报） |
| 0AC-A6 | P3 | 实现合理性 | run 尾 `close_drop`+warn 只在 Ok 路径；Err 路径（含⑥收口 return）队列靠 Rust drop 静默消亡、无 journal 留痕，与库侧注释「调用方据此落 journal 留痕」不对齐（接线现实下队列恒空故无实际影响；修 A1 后此面变大） |
| 0AC-A7 | P3 | 注记 | 「三件合法事件」对应 9→11 只 +2，commit message 算术与措辞不完全对账；in-code 11 件序列实测为权威 |

---

## 3. 检索侧补强 G1–G4（orz `7e151ed1`，2 文件 +824/−83）

**总评**：G2/G1/G3 落码忠实且带 wiremock 行为级测试（23 单测实跑绿），G2 阈值语义实现精确；但 G4 三项主张一项未接线、两项在传输/默认链层面失效，且默认链激活了一个先前休眠的解析器缺口。

### 3.1 已核实健全的点

- **G2**：默认开（缺省 true，仅显式 `0/false/off/no` 关，`local_segmented.rs:215-222`）；判负复用既有闭枚举 `empty_result` 并 `continue` 引擎链（:794-821）、不新增稳定码；阈值语义精确＝**词集命中率** `need = ceil(0.25 × min(|Q|,12))`（:635），score＝前 3 条 title+snippet 中出现的不同查询词数（:636-642），3 词查询 need=1 非虚设、词集空（纯符号/纯数字）直接放行（:632-634，有测试）；闸门位于页抓取/解包启动之前——判负引擎不浪费页级预算。
- **G1**：`tokio::time::timeout` 只包 `fetch_serp`（:738-743）——页抓取挪出，`ORZ_RETRIEVAL_DEADLINE_MS` 语义收窄属实；页级预算＝`min(segment, remaining)`（:863-869）；失败码映射正确：`e.is_connect()` ⇒ `capability_unreachable`（:925-931）、每引擎钟到点 ⇒ `network_no_response`（:745-761）、页级到点不影响已解析命中交付（wiremock 慢页测试 :1527-1565）；`T_overall` 30s 经逐引擎/逐页 `min(remaining)` 从属执行。
- **G3**：6 worker＝`Semaphore` + `tokio::spawn` 波次并发（:841-858）、单条 6s 请求级超时（:661）——墙钟＝⌈n/6⌉×6s，无串行放大；回填取值序「页抓取回填 > 解包 > 包装原样」由 `backfilled` 集合实现（:861-883），两条 wiremock 回环测试分别钉住（:1452-1486 / :1488-1523）。
- **G4（解析层）**：读取序 `ORZ_RETRIEVAL_PROXY → HTTPS_PROXY → HTTP_PROXY`、命中（含 empty/none）即返回不读后续（:669-687，四分支测试 :1569-1603）；显式 `ORZ_RETRIEVAL_ENGINES` 永远优先（:240-258）；有代理默认链四引擎（:259-272）；代理只加 `local_http` 客户端（:1019-1021），页抓取与解包共用同一客户端；脱敏 `proxy_display` 剥 scheme/路径/凭据、保留 host:port，含凭据内 `@` 的正确处理（:690-697）。
- **测试实跑**：`cargo test -p orz-tools --lib local_segmented` **23 passed / 0 failed**（本批新增 13：gate×5、symbol_only、unwrap 判定、解包回填、回填优先、慢页、proxy×3）；降级页/解包/回填/慢页均为 wiremock 真实 HTTP 回环行为测试，非仅配置解析。

### 3.2 问题清单

| # | 级别 | 类别 | 发现与证据 |
|---|---|---|---|
| RET-B1 | **P1** | 符合性＋实现合理性 | **有代理默认链 `duckduckgo,google,bing_cn,bing_global` 前两引擎永远解析不出任何结果**（§1-3）。`parse_bing_serp`（b_algo 正则）对 DDG html 版与 Google SERP 恒 0 命中 ⇒ 最坏各烧满 T_first（2×10s 死重），bing_cn 命中后页级剩余预算归零（交付无正文段）；G3 解包机制（只由 google/baidu 包装触发）在该默认链下整体休眠。解析器缺口系先前批次既有，但**本批首次把这两引擎放进默认链**、使缺口在默认配置激活；设计稿排序依据「DDG 走代理 1.0s 且相关」无法兑现为交付。建议：补 DDG/Google 解析器，或解析器就位前把代理默认链退回 bing_cn 系并同步账本 |
| RET-B2 | **P1** | 符合性 | **`ORZ_RETRIEVAL_PROXY=none` 未在传输层实现显式关**：resolve 返回 None 时 `build_http_client` 既不加显式代理也不调 `.no_proxy()`（:1015-1023）⇒ reqwest 0.12 默认 `auto_sys_proxy` 追加系统代理匹配器、读取 HTTPS_PROXY/HTTP_PROXY/**ALL_PROXY**（含小写）⇒ env 有代理时 `none` 与不设同态走代理。设计 §6.1「none 供评测基线保证无代理形态」落空；A/B「代理 vs 直连」对照读数被污染。读取序本身也缺 ALL_PROXY 一项。建议：resolve=None 时对 local_http 加 `.no_proxy()`（env 全无时行为不变） |
| RET-B3 | **P1** | 符合性 | **「探针读数带 proxy=on|off＋脱敏端点」未接线**（§1-5）：`engine_chain_detail`/`proxy_display` 零外部消费者；探针侧 `retrieval/projection.rs:138-163` 另行重读 env，detail 形如 `web_search engines=builtin local_segmented=true`——无 proxy 字段，且代理切换的默认四链被报成 `builtin`；探针侧重复读 env 亦构成漂移面。commit message 与索引 v3.29 ② 该项主张不实。建议探针消费 `WebSearchClient` 的 detail（或等价注入） |
| RET-B4 | P2 | 符合性 | G3 解包段未按设计 §5.3「整段取 min(剩余整体预算)」钳制：单条 6s 超时但 await-all 不受 remaining 约束（:876-883），页循环耗尽 30s 后交付仍可再等 ⌈n/6⌉×6s——T_overall 可被突破（当前因 RET-B1 休眠、触发面小）。建议 await 前按 remaining 截断 |
| RET-B5 | P3 | 设计合理性 | `T_acquire`「不算检索预算」未字面实现：每引擎钟在建连前起跳，建连 4.9s ⇒ 真正检索窗口仅剩 5.1s。设计 §3.3「时间上相邻、先到者归因」部分认可此形态，机器判据（wall_ms 自请求发出起算）同口径——注记非违约 |
| RET-B6 | P3 | 设计合理性 | 词集封顶截断序：先 ASCII 词后 CJK bigram 直接 `take(12)`（:594-624）⇒ ≥12 个 ASCII 词的混排长查询把 CJK 词全部裁掉；跨词 bigram（「编程入门」中「程入」）是永不命中的死重，抬高 need；CJK 单字查询词集空恒放行。S4 调阈值时宜一并登记 |
| RET-B7 | P3 | 注记 | 回填/解包得到的最终 URL 未经清洗（utm/跟踪参数、开放重定向落点原样进交付与 citations，:870-874/:660-664）。设计未要求，与「不改写」纪律一致，已知取舍 |
| RET-B8 | P3 | 符合性 | 设计稿 §8.1 现文仍停留 v3.26 时点门禁读数（`valid:false` / 5 错），「v3.29 随 manifest 重算复归 valid:true」的就地注记未回写（复归主张只在索引头部行）；文档状态面可读性欠佳 |
| RET-B9 | P3 | 注记 | 服务端 `/responses` 客户端与其余 reqwest 客户端同样受 env auto_sys_proxy 影响（`client.rs:75-91`，既有行为非本批引入）：设计 §6.1「不动服务端客户端以保凭证边界」的实际约束力弱于表述——env 有代理时带 key 客户端本就经代理转发。备查 |
| RET-B10 | P3 | 注记 | unwrap 触发形态按设计表登记为 `google.com/goto` / `baidu.com/link`（:654-656）；真实 Google SERP 常见包装为 `/url?q=` 形态，未离线验证。当前因 RET-B1 休眠；入集前需一手 SERP 样本核对 |

---

## 4. 0ae 上下文软门与模型参与压缩（orz `f0040557`，10 文件 +1237/−46）

**总评**：D0/D1/D4 符合度高、接线真实；但 D3 是结构性 P0，事件契约面两处破损，D2 大半在默认配置下是死面且账本未披露。该批未动 orz-host / orz-tools / orz-assurance / prompt.rs / compact.rs / render_fold.rs——凡主张涉及这些面的，实际落点均查明（见各条）。

### 4.1 已核实健全的点

- **D0 工具注册与约束**：`blackboard_write` 在 `controller.rs:3036-3060` 无条件注册（不随 plan_first 门）；`section` 双闭集——ToolDef JSON `enum:["plan","notes"]` ＋ `blackboard.rs` `ModelNoteSection::parse` 拒绝式解析；≤8K 为**拒绝非截断**（`host_exec.rs:1850-1857`，`MODEL_NOTE_MAX_CHARS=8192`）；写入走 (round,domain)+ts 盖章。
- **D0 ReadOnly 判定真实**（非 0v F1 的未知工具 fail-closed Edit 桶）：orz-loop `tool.rs:84-89` `risk_class` 收录 `blackboard_write → ReadOnly`，权限门 `host_exec.rs:1119-1132` 消费的正是同一 `ToolDispatcher::risk_class` ⇒ 全策略自动放行；orz-host permission.rs 无需改动——落点澄清，无「漏改 orz-host」问题。
- **notes 随 epoch 快照**：`epoch_snapshot` 携带 notes、`restore_epoch_snapshot` 恢复并推计数、`#[serde(default)]` 兼容旧档；机械单写者分区未开放写入。
- **水位状态标**：`blackboard_watermark_label` 恒挂于每个 live `blackboard_read` 响应头（`controller.rs:2275-2320`，含零增量时）；blackboard_write 成功回执亦带；分母随 `ORZ_BLACKBOARD_LIVE_BUDGET_BYTES`（`render_fold.rs:31/39`，默认 10MiB，复用 fatigue 单一预算源）。
- **D1 落点与顺序**：落点在 `agent_loop.rs:3345-3360` 而非 prompt.rs（prompt.rs 确未动）；0x 模板块先推（`controller.rs:3755-3762`）、guidance 后推——「追加一问」顺序正确、0x 模板零改动；N=20 一次性补救＝`plan_reminder_done` 旗标＋`model_note_count()==0` 判据＋MechanicalAuditUpdate 留痕。
- **D4 冻结语义**：`run_context_block` 存入 `LedgerFoldState`、随推进刷新、推进之间字节稳定（`action_ledger.rs:1055-1058`）；基线 `git rev-parse --short=12 HEAD` + `status --porcelain` 零模型调用（:81-100）；指纹源＝黑板 edits 分区（机械单写者、模型不可伪造）。
- **D2 数据面**：8 阈值与设计逐一对应，env 名 `ORZ_LADDER_{INTERRUPT,REMIND,SOFT,HARD,COMPRESS}_K` 与账本一致，0=显式禁用、软级列表解析容错、去重键=级别+K、每级一次、rearm 全量重置（`attention_ladder.rs:19-214`）；4 单测实跑绿。
- **plan_write schema 顶层增量已补**：父仓 `10297bf2`（f0040557 后 7 分钟）为 `runtime/plan-write-event-payload-v0.2.schema.json` 增 `section`（enum plan|notes）/`content_chars` 可选字段；0ag 钉子机械核对了 section 枚举＝`ModelNoteSection`。工具面 8+1 显式例外登记一致（projection.rs 测试期望清单同步）。

### 4.2 问题清单

| # | 级别 | 类别 | 发现与证据 |
|---|---|---|---|
| 0AE-C1 | **P0** | 实现合理性＋符合性 | **D3 压缩窗口自我否定**（§1-2）。窗口轮 `current_tool_defs = Vec::new()`（`agent_loop.rs:1684-1691`——`pending_keeps_tools` 仅含 Orientation，:1639-1641），模型连 blackboard_write 的声明都看不到；消费分支无条件只保留 text、tool_calls 丢弃不派发（:2199-2255）⇒ `participated = writes_now > window_start_writes` **恒 false**：3 个 920K 级文本轮纯空转后必然 `model_participated: false`。窗口提示语还在要求模型「固化必要内容并标注可弃范围」——用它调不动的工具。设计 §6 主路径「机械按模型标注执行折叠」零实现（标注只存在于被丢弃文本中，折叠/compact 均不消费），实际只有机械兜底在跑；该消费分支**零测试覆盖**。commit message「model solidifies via blackboard_write」不可成立。建议：窗口轮保留 blackboard_write 单面（其余不派发＝打断动作），或改设计为「文本标注＋机械解析」；补窗口行为测试 |
| 0AE-C2 | **P1** | 符合性 | **`mechanical_audit_update` schema 未随 0ae 扩展**（§1-4）：schema kind 闭枚举 `{tool_result,plan_gate,budget}`＋payload required `{key,round,summary,anomaly}`＋`additionalProperties:false`，0ae 新写三种越界 kind 与另一形状 payload ⇒ conformance 判定器（`orz-assurance/src/journal/conformance.rs:464` 逐事件 payload schema 校验）将在**首次 128K 阶梯触发时判 journal invalid**。父仓 10297bf2 只补了 plan-write schema；0ag 钉子未覆盖此枚举——「实现常量先行、枚举不对账」在钉子落地的同夜换位复发。建议：schema v0.3 扩 kind＋按 kind oneOf 放宽 payload，并把 mechanical_audit_update 纳入契约钉子 |
| 0AE-C3 | **P1** | 符合性 | **plan_write 事件 `validation` 子对象违规**：实现写 `validation:{valid,section,content_chars}`（`host_exec.rs:1917-1925`）；schema `$defs/validation` required `[valid,errors,ignored_fields]`＋`additionalProperties:false` ⇒ 缺 2 必填＋多 2 禁用属性，conformance 必挂。家族规则 `verify_plan_write` 只读 `valid` 布尔所以未报——两层校验口径分叉掩盖违规。建议 validation 内只留 `valid:true`，section/content_chars 只放顶层（顶层两字段本身合法） |
| 0AE-C4 | **P1** | 符合性＋设计合理性 | **D2≥160K 各级＋D3 920K 默认配置下常态不可达（死面），账本未披露**：阶梯量尺＝折叠后视图的实测 prompt（`last_prompt_tokens`），而机械 fold@128K 未动（`compact.rs:273`），每次把视图压回 ~9.6K ⇒ 160K–920K 全部不可达；连 128K 级也与 fold 线是两把尺（131072 实测 vs 128000 chars/2 估算），可达性随语种漂移。设计 §2「现行 128K/160K/200K 三点**转软门**」未发生——机械三点全保留（`agent_loop.rs:1346-1350`），实为「叠加」。0ae 账本与 commit message 把 D2/D3 当可运行主张，A/B 判据「920K 压缩轮 model_participated 占比」默认参数下永远采不到数（且参与率恒 0，见 C1）。建议：账本补死面登记；欲激活阶梯须同批裁决 fold 触发线上调/退役 |
| 0AE-C5 | P2 | 实现合理性 | 非 git 工作区 D4 整块缺席：`run_baseline.as_deref().map(...)`（`agent_loop.rs:1492-1497`）使 baseline=None 时自编辑清单＋编辑指纹（与 git 无关的两段）一并消失，与 `action_ledger.rs:78-80` 注释「不渲染基线段，其余两段照常」直接矛盾——render 函数签名专门收 `Option<&str>` 就为 None 降级，调用点却耦死。建议 run_context_block 无条件渲染、baseline 作内部段 |
| 0AE-C6 | P2 | 符合性 | 阶梯提醒文本不带水位：设计 §3 明定「D2 各阶梯提醒文本携带当时水位」，实现各 block 文案无【x.xM/10M】，fire 点亦未附加（`attention_ladder.rs:216-254`；fire 审计 payload 无水位字段） |
| 0AE-C7 | P2 | 实现合理性 | rearm-on-NoOp 潜在窗口循环：`CompactDecision::NoOp` 也 rearm（`agent_loop.rs:1409-1413`）——若压缩在 920K 处返回 NoOp，下一 loop-top 重新开窗→3 轮→NoOp→rearm 循环，仅靠轮预算封顶，每循环烧 3 个 920K 级请求。建议 NoOp 不 rearm |
| 0AE-C8 | P3 | 实现合理性 | 无效 section 错误文案 `{{section_raw:?}}` 转义花括号不插值（`host_exec.rs:1739-1741`），模型看到字面 `{section_raw:?}`；行尾 `\` 产生字面反斜杠——明显笔误（plan|notes 限制信息仍有传达） |
| 0AE-C9 | P3 | 实现合理性 | `partition_revisions()` 缺 notes 项（`blackboard.rs:898-910`）：`revisions.notes` 计数器存在、写时自增，但 PULL 增量头永不显示 notes 徽章——模型写笔记的可见性信号缺失 |
| 0AE-C10 | P3 | 设计合理性 | plan-gate 轮工具面只暴露 plan_write+blackboard_read（`agent_loop.rs:1692-1701`），`blackboard_write` 不在内——「无条件声明」在 plan-first 首轮不成立，D1 指引引导的写入口与该轮可用面错位 |
| 0AE-C11 | P3 | 符合性 | D4「**本 run** 自编辑文件」标签不准：edits 是会话级累积（跨 run 存活、epoch 轮换才清）——多 run 会话中该段含历史 run 编辑，恰在要解决的「出处失忆」场景制造新的归属歧义 |
| 0AE-C12 | P3 | 设计合理性 | 首次实测读数直接 >800K（恢复会话/长中断）时 7 个 block 一次性齐发＋同轮开窗（`attention_ladder.rs:167-194`）——「每级一次」的节奏设计在跳变下退化为噪声突发 |
| 0AE-C13 | P3 | 注记 | 多处多行注入文案含成串空格（字面量未用 `\` 续行），cosmetic |

### 4.3 slider 新设计稿关系评估

[`CONTEXT_DYNAMIC_SLIDER_DESIGN_2026-09-15`](../CONTEXT_DYNAMIC_SLIDER_DESIGN_2026-09-15.md)（父仓未跟踪件）与已实施 0ae 是**演进取代关系**（固定锯齿折叠 → 常驻滑窗），非正交补充：

- **存活面**：D0/D1/D4 完整存活且地位上升（黑板固化是唯一不随滑窗滚动的内容；D4 机械段成为 S2 manifest 宿主候选）——与代码事实一致。
- **归位面**：D2/D3 在现行 H=128K 下登记为死面（「保留兜底＋登记死面」），与本审查 C4 的独立代码结论**相互印证**；量尺不一致（阶梯/压缩梯吃实测 prompt、滑窗触发吃 chars/2 估算）被其 §3.2/§3.4 显式列为实施批前置项——这是 slider 对 0ae 的最重要增量发现。
- **冲突面**：env 名不冲突（新增 `ORZ_SLIDER_WINDOW_TOKENS`，`ORZ_LADDER_*` 保留）；但阶梯阈值若采其 §3.4①「按 H 派生 0.5H/0.75H/0.95H」将改写 attention_ladder.rs 常量面与「每级一次」语义（滑窗周期内会反复越线），须与档位同批裁决——slider 自身亦如此要求。
- **成熟度与边界**：v3 已含两轮勘误、用户参数裁定、两处显式翻转登记，并经两轮真机评审（`tmp_slider_review/` 有 RUN-CLI-6aa8d2c6 / 6aa8d2a3 痕迹）——draft 品级里属高成熟；DP-1…DP-9 未裁决、自律边界清晰。**遗留风险**：其成本模型为解析推导非实测；且 slider 对「D3 已落码」的核对停在符号级（只核 `DEFAULT_COMPRESS_K=920`、`COMPRESSION_WINDOW_ROUNDS=3` 在场），**未发现本审查 C1 的 P0**——「保留兜底」实际保留的是一个永远不会返回 true 的兜底。

---

## 5. 0af（资源门文案）/ 0ag（契约钉子）与账本对账（orz `8512fc71`）

**总评**：账本面干净。0af 未落地与 pending 状态一致；0ag 钉子与三份 schema 逐字吻合、静态可红；先占改名治理链完整；门禁复跑成立。

### 5.1 0af（GAP-RESOURCE-GATE-DENIAL-COPY，设计审查——该项未实施）

- **状态一致**：定案文案关键词（「无法新增派发/即将耗尽/寻找其他方案」）在 orz、assurance、runtime、protocol 全零命中；现行信封仍是英文旧文案（`orz/crates/orz-host/src/resource_gate.rs:1236-1249`，"heavy action refused before dispatch — resource headroom insufficient…Nothing was started."）——与 BACKLOG:958-963 / TODO:700-702 / 索引 §8 `pending` 三面一致。
- **摩擦 B 证据闭环（诊断半边）**：深审 run journal 恰 2 条 `host_resource_denied`（EVT-20729/20786），payload 含轴级缺口＋七字段机械读数——「模型随即正确诊断」成立；「拒绝后该干什么」空白＝0af 立项范围，归因准确。
- **边界防误扩已写明**：BACKLOG/TODO 双写「不改 fail-closed 判定逻辑与阈值，只改文案与轴标注」。
- **实施前应补的三个定义（均 P3）**：① **Unknown 档变体**——`resource_gate.rs:1180-1191` fail-closed 分支（读数不可得）若套「即将耗尽」为不实陈述（什么都没测到），需要变体文案（如「无法核实宿主机资源余量」），防止实施时两分支一刀切；② **百分比取整**——真机首拒 reason 实文「commit headroom 25% < 25% required」字面自相矛盾（实值 7.28/29.85=24.4% 被 100−75 显示），改文案时建议同步一位小数或字节直读；③ **中英混排**——新文案中文、信封其余（`error: resource_insufficient` 键、readings、英文读数行）保持英文，实施时应明示「reason 中文＋机械读数英文」为有意选择；「储存」vs「存储」系用户逐字定案，尊重不改、仅注记。

### 5.2 0ag（契约面机械对账钉子，立项即闭合）

- **与 schema 逐字吻合**：`runtime/result-delivered-event-payload-v0.2.schema.json`（boundary 含 `B3_mid_generation` 预留、`suppressed_reason` 恰 3 值）、`retrieval-progress-event-payload-v0.2.schema.json`（`stable_code` 恰 5 值、stage 6 值）、`plan-write-event-payload-v0.2.schema.json`（section=[plan,notes]）与钉子断言（`immediate_delivery.rs` 8512fc71 +109 行）逐一比对全部一致；引用的 `BOUNDARY_B1/B2` 等确为生产常量；`ModelNoteSection` = {Plan,Notes} 与 section 枚举同构。
- **可红（静态判断）**：常量值改一位 → `contains` 断言红；闭集任一值变 → `assert_eq!` 红；schema 文件缺失 → `panic!("runtime schema readable")` 红。
- **可移植性/CI**：`CARGO_MANIFEST_DIR/../../../runtime` 与 orz-assurance 三处既有同布局解析一致（`candidate_prefilter.rs:154-161` 等，ORZ-BUILD-MOUNT-001 背书）；orz 唯一工作流只跑 rustfmt、测试不在 CI——零 CI 影响。
- **覆盖边界（P3 注记）**：六枚举 ⊆ 族为单向包含（schema 侧新增不红，代码已注释为预留取舍）；实现侧新增常量若忘记加进钉子则不红；五稳定码在生产侧仍有三份字面副本（orz-tools `CAUSE_*`、orz-assurance `RETRIEVAL_STABLE_CODES`）未被钉绑定（跨 crate 依赖方向所限，运行期由 journal 校验兜底）；plan_write section 断言用字面量而非 `ModelNoteSection::as_str()`——账本「逐字互证」表述对后两点略有拔高。
- **改名痕迹（P3，不改史）**：`8512fc71` 提交主题/正文仍写 "feat(0af)"，活代码注释仍写「── 0af 契约面机械对账 ──」；更正批 `b9b93f2c` 与索引 v3.29 已把改名登记清楚，历史提交不重写——建议下次触碰该文件时把注释改 0ag。

### 5.3 账本对账

| # | 级别 | 发现 |
|---|---|---|
| LED-D1 | P2 | **索引 §6/§8 自洽**：§6:338 GAP-MECH-IMMEDIATE-FEEDBACK 状态 `in_progress` 且叙述止于 2026-09-14 修复报告；§8:365 同一 ID 列于 `pending`——两处互相矛盾（§8 即状态清单）。现行事实（BACKLOG/TODO）＝0ac 开放、S3 出口达成、S4 待放行；建议下批统一并补 ①-b/补强两笔历史（索引已有复合状态先例：「implemented（代码）；载体重建待放行」） |
| LED-D2 | P2 | **slider 设计稿账本不可见**：`docs/CONTEXT_DYNAMIC_SLIDER_DESIGN_2026-09-15.md` 未跟踪，索引/TODO/BACKLOG「slider/动态」零命中（门禁 manifest 亦不含未跟踪件）。建议下一账本批登记入库（先例：`1ff0c0d9` 邻线代提）或显式标注归邻线处置 |
| LED-D3 | P3 | GAP-RESOURCE-GATE-DENIAL-COPY 缺 §6 路由条目（同族 GAP 均有；仅见 §8 pending＋v3.28 头部叙述）——补一条短路由即可 |
| LED-D4 | P3 | BACKLOG 0ac 节头仍写「G3 仍 open」（BACKLOG:919），节体已含 ①-b/补强完成——节体权威且新，节头未随批更新 |
| LED-D5 | P3 | 交接件 §5-A 原文无「已由 0ag 机械化退役」就地批注（退役指针在 BACKLOG/TODO 0ag＋索引 v3.29＋代码注释，双向引用成立；冻结件不加批注符合以账本为更正面的惯例——仅注记） |
| LED-D6 | P3 | 临时目录处置建议（仅登记不执行）：`tmp_s3b/`（6 件）与 `tmp_slider_review/`（12 件，含 slider 设计稿备份与 0ae 评审 run 产物）均 gitignore、账本无登记；按 S-14 先例建议下一卫生批归档 `存档/` 或声明弃置 |
| — | — | **计数与三账本一致性**：0af 先占改名治理链（c18a2e07 → 10297bf2 → b9b93f2c）完整、计数 32 无误、0aa–0af/0ag 节全在位、0ad 撤案留痕；0ae/0af 三账本状态表述一致。**门禁复跑**：`python scripts/check_repository.py` → `"error_count": 0, "valid": true`，`orz_source_manifest_files: 1450`——账本主张核实成立 |

---

## 6. 交叉结论（三个复发性教训）

1. **「符号在位≠接线」同夜复发三处**（M1 不可达 / D3 恒 false / 探针 proxy 未接线），且全是单测绿＋门禁绿盖不住的**接线级**缺陷。现有测试策略缺的是「接线钉」（带 fake host 的端到端断言）。建议把「每个新机制的触发条件在接线中至少可达一次」列为批次出口的机械核对项；0ag 的机械对账模式可推广，但须钉接线而不只钉枚举。
2. **契约漂移在 0ag 落地同夜换位复发**：0ag 钉住 immediate_delivery 三份 schema 的同时，0ae 同批向第四份 schema（mechanical_audit_update）写了越界枚举。建议把「新增 MechanicalAuditUpdate kind 必须同步 schema＋入钉」补进契约纪律，schema v0.3 扩枚举或按 kind oneOf。
3. **账本主张面三处虚/缩**：M1「已接线」（实为不可达）、探针 proxy 读数（未接线）、M3「模型可见」收窄未回写设计稿、D2/D3 死面未披露——「0ac S3 出口达成」与「0ae 代码面完成」两主张按 §0 附加限定修正。S4 放行前必须先修 0AE-C1（P0）与 0AE-C2/C3（首次触发即 invalid journal，S4 一开阶梯就会踩）；M1（0AC-A1）至少降级账本口径或补接线，否则 S4 的 M1 A/B 判据无意义；RET-B2（none 失效）会污染检索 A/B 对照读数。

## 7. 建议处置清单（全部留用户裁决，本审查未动任何代码与账本实体）

- **S4 放行前必须**：0AE-C1（D3 窗口工具面/消费分支）、0AE-C2＋0AE-C3（schema 对账）、0AC-A1（M1 接线或账本降级）、RET-B2（`no_proxy`）。
- **下批修**：RET-B1（默认链解析器或退链）、RET-B3（探针接线）、RET-B4、0AC-A2（empty_result streak）、0AC-A3（接线钉×3）、0AE-C4（死面披露与 fold 线裁决）、0AE-C5/C6/C7。
- **账本修正**：LED-D1（§6/§8 统一）、LED-D2（slider 登记）、LED-D3/D4、§0 两处主张限定、0ae 账本补死面。
- **注记级**：0AC-A4–A7、RET-B5–B10、0AE-C8–C13、0af 三定义、0ag 覆盖边界与改名痕迹、LED-D5/D6。

## 8. 方法与未覆盖范围（诚实声明）

- **已做**：四路子代理并行深审（合计约 200 次工具操作）；主会话五处承重发现亲手抽查（§1）；定向测试实跑（orz-loop `immediate` 14/14、orz-tools `local_segmented` 23/23、orz-host acp 两测试 11 期望绿、orz-loop `attention_ladder` 4 绿，均温热增量）；门禁复跑 valid:true/1450。
- **未做/不可检**：全量测试套件未跑（避免长构建与写面）；fmt/clippy 门未复跑；1deeba75 时点门禁读数（orz-loop 788 等）未在不 checkout 前提下精确复验（HEAD 实测 orz-loop 792 / orz-host 329 量级吻合）；DDG/Google 真机 HTML 与包装形态未一手复测（RET-B1 基于 b_algo 为 Bing 专有类名的强先验）；代理传输端到端无行为测试（RET-B2 由 reqwest/hyper-util 源码证实、非运行时复现）；Python 侧 validator 未读（schema-invalid 结论以 Rust conformance.rs 自述镜像为准）；`.gsa/runs` 下 0ae 后是否有真机 run 已触发 ladder/plan_write 未排查（若有，0AE-C2/C3 应已在门禁显形——可复核的预言）；模型侧 `reasoning_content` 不可检（沿深审口径）。

## 9. 关键证据速查

- M1 断点：`orz/crates/orz-loop/src/agent_loop.rs:3273`（admit）→ `:3286`（due）→ `:2392`（M1 条件）；`immediate_delivery.rs:459-478`（due 全量 take）。
- D3 断点：`agent_loop.rs:1639-1641`（pending_keeps_tools）→ `:1684-1691`（Vec::new）→ `:2199-2255`（消费分支＋恒 false participated）。
- 枚举越界：`runtime/mechanical-audit-update-event-payload-v0.2.schema.json`（kind 3 值）vs `agent_loop.rs:1304/1333/2243/3357/3387`。
- b_algo 复用：`orz/crates/codegen/orz-tools/src/implementations/web_search/local_segmented.rs:959`、`:391-396`。
- 探针未接线：`local_segmented.rs:287-298`（detail 定义）vs `orz-loop/src/retrieval/projection.rs:138-163`（另行重读 env）。
- 门禁：`check_repository.py` → `valid: true / error_count: 0 / 1450`（本审查落档前复跑）。

---

## 10. 处置批注（2026-09-15 修复批，orz `183fbb08`，父仓随批账本 v3.31）

按用户指示「能修复的问题先修完」，机械可修集已全部落码（修复实施＝三路并行代理按文件所有权分组＋主会话收口；契约面 schema 与 Python 镜像随批同步）：

**已修（13 项）**：

| 发现 | 修复要点 |
|---|---|
| 0AE-C1（P0） | 窗口轮仅暴露 `blackboard_write` 工具面；消费分支过滤派发（其余动作丢弃＋机械提示）；延迟收口使 `model_participated` 基于派发后真实写数；端到端钉子×3（参与/混合声明丢弃/纯文本耗尽） |
| 0AC-A1（P1） | 终答候选分支补 B2 drain+admit（gate=`m1_enabled()`＋`!m1_used`，避免只进不投）——M1「已接线」主张自此为真 |
| RET-B1（P1） | 有代理默认链改序不改集：`bing_cn,bing_global,duckduckgo,google`（止损序；**偏离 v3.26 记录序**——DDG/Google 无解析器恒空转；守卫钉防回退；完整解析器留 S4） |
| RET-B2（P1） | resolve=None 时 `local_http` 加 `.no_proxy()`——`none` 显式关在传输层生效，A/B 对照不再被 ALL_PROXY/小写 env 污染 |
| RET-B3（P1) | `from_env()` 装配期写 OnceLock 快照＋`assembled_local_segmented()` accessor；探针 `search_engine` 分支消费单一源（`proxy=on|off`＋脱敏端点入读数，代理四链不再误报 `builtin`） |
| 0AE-C2（P1） | runtime schema kind 枚举 3→6＋Python 镜像同步；六 kind 常量单一源（`mechanical_audit.rs`）；五处写入收敛为 `{key,round,summary,anomaly}`；逐字对账钉子（schema 枚举↔常量全等＋payload required） |
| 0AE-C3（P1） | plan_write `validation` 改为 `{valid,errors:[],ignored_fields:[]}`（section/content_chars 仅顶层） |
| 0AC-A2（P2） | ⑥ 确定失败 streak 仅认 `network_no_response`（`empty_result`=合法 I1 信息退出计数） |
| 0AE-C5（P2） | D4 无条件渲染（非 git 工作区仅基线段缺席，清单/指纹照常） |
| 0AE-C6（P2） | 阶梯提醒与窗口块注入带水位行＋审计 summary 带 `watermark=` |
| 0AE-C7（P2） | 压缩 NoOp 不再 rearm（rhythm 路径逐轮重试机械压缩，不停摆） |
| 0AE-C8/C9/C10（P3） | section 错误文案转义修正；`partition_revisions` 补 notes 徽章；plan-gate 轮面补 `blackboard_write` |
| 顺手项 | C13 其余成串空格（首轮引导/N 轮提醒/窗口块文案）；`immediate_delivery.rs` 注释 0af→0ag 更正；controller 分区序文档补 notes |
| 窄边沿追加（同日修复二批，orz `1f303cf4`；用户裁定「触发面窄也得修」） | §10 原登记观察「窗口轮落穿后同迭代恰遇 budget 耗尽收尾 break（D-8）或 IPG block 时收口状态随循环终止丢弃」——收口逻辑抽为统一出口 `finalize_model_compression_close`（`take` 幂等），接入四点：下一轮 loop-top / budget 收尾 break 前 / IPG block break 前 / run 尾安全网；e2e 钉 `compression_window_close_survives_budget_exhaustion_break`（break 前落地＋`model_not_participated`＋`writes_now=0`）。`?` Err 传播路径仍丢弃＝0AC-A6 同类，维持挂账 |

**修复批门禁汇总**：首批（`183fbb08`）orz-loop 796/0/3；追加批（`1f303cf4`）orz-loop **797**/0/3（含新钉）、fmt 干净、agent_loop.rs clippy 零告警（全 crate 65 条均在未触碰区域、既有）；manifest 随批重算。

**门禁（修复批合并态）**：orz-loop 796/0/3、orz-tools 2884/0/6（首跑 1 例 flaky 未留名，全量复跑×2 绿＋local_segmented 定向 5 连绿）、orz-host 324/0/5 单线程、orz-assurance 226/0、Python 镜像（mechanical）11 通过、fmt 干净、clippy 零新增。

**留裁决/未修**（责任面与理由）：
- 0AC-A3（接线钉扩展）：C1 端到端钉×3 已随批落地；B1 投递/宿主 drain 两枚接线钉仍缺（需 fake host 基建扩展，建议随下批）。
- 0AC-A5（⑥ 第三触发「结果已形成」）：未落码，静默缩围转显式挂账（本件 §2.2 维持登记）。
- 0AC-A6（Err 路径 close_drop 留痕）：loop 终止路径多点 `?` 传播，单一收口点不存在，涉及结构改造，留裁决。
- 0AE-C4（fold 线 vs 阶梯死面）：需用户裁决 fold 触发线上调/退役；死面已披露进 BACKLOG/TODO 0ae。**用户裁决（2026-09-15）：暂缓**——0ae 后续还要整体修改，死面维持登记、不激活；slider 由邻线补全中（见 §11 口径补记）。
- 0AE-C11/C12（编辑清单跨 run 归属标签、跳变齐发）：行为语义裁决项，未动。
- RET-B1 完整解析器（DDG/Google）、RET-B5–B10：S4 面（真机样本前置）。**用户裁决（2026-09-15）：解析要补充**——架构口径见 §11 补记：解析器属纯 HTTP 后备车道（承前裁决 §9.1「CDP 依赖已剥离」），与浏览器车道正交；**Google 纯 HTTP 直取（含代理）为 consent 页**（设计稿 §一手证据表），其可行读取路径是真浏览器车道——实施批待排、须带一手 SERP 样本（含 RET-B10 `/url?q=` 形态核对）。
- LED-D2（slider 设计稿入库）：归邻线处置，未代提交（**用户确认 2026-09-15：slider 尚在补全**）。
- 0af（资源门文案）实施：独立 pending 项按其立项排期，未并入本批；其实施注意三条已补进 §6 路由条目。
- 已登记新观察（→ **已修复**，见上表窄边沿行）：窗口轮落穿后若同轮恰命中 budget-exhausted/IPG 终止路径，`model_compression_close` 随 loop 终止丢弃、收口审计事件不落账——修复批追加（orz `1f303cf4`）以统一收口出口闭合；残余面仅剩 `?` Err 传播路径（0AC-A6 同类挂账）。「blackboard_write」名字在 controller 注册处与新过滤函数仍为两份字面副本（已注释互指）。

---

## 11. 架构口径补记（2026-09-15，用户提问驱动）：DDG/Google 解析器 vs 深度浏览器操控

**问**：orz 侧是深度浏览器操控，检索经浏览器直接读取即可，本身还需要 SERP 解析器吗？

**答：两条车道，正交关系——解析器只属于纯 HTTP 后备车道，浏览器车道永远用不到它。**

1. **浏览器车道**（`browser_control`/`browser_read`，Chromium/CDP，FUS-RETRIEVAL-DUAL-LANE external lane）：搜索引擎页由浏览器渲染，模型直接读渲染后 DOM——**不需要任何 SERP 解析器**。其可用性前提是真有浏览器：真机可装（`ORZ_BROWSER_PATH`）；eval 容器默认没有（第 0 轮 14 次 launch 全 failure，用户已裁决不新增容器内浏览器），须适配器注入 Chromium 快照（0v F2 形态）。
2. **纯 HTTP 后备车道**（`web_search` 的 `local_segmented` 分段实现）：**设计承前裁决 §9.1 明写「分段检索保持纯 HTTP（CDP 依赖已剥离）」**——这条车道的立意就是「没有浏览器、或浏览器车道不可达/太慢时，纯 HTTP 也要能在 10s 预算内拿到结果」。它拿到的只有原始 HTML，因此必须自带解析器；现状只写了对 Bing 的 `parse_bing_serp`（b_algo 正则），DDG/Google 恒 0 命中（审查 RET-B1）。
3. **补充解析器的现实边界（一手证据，设计稿 §证据表）**：
   - **DDG**：`html.duckduckgo.com` 纯 HTTP 可达且结构稳定（结果链接为 `uddg` 跳转参数——正好是 G3 解包机制的既定输入形态），**值得补解析器**；
   - **Google**：设计稿一手证据明载「代理下 Google 直取 = 200 / 92 KB（**纯 HTTP 为 consent 页**；真浏览器去掉 `--enable-automation` 后为正常 SERP）」——纯 HTTP 车道里 Google 解析器大概率解析的是 consent 页而非 SERP，**Google 的可行读取路径就是浏览器车道**（双车道设计本就允许模型按静态标注自主换道）。
4. **结论口径**：补 DDG 解析器＝补齐纯 HTTP 后备车道的引擎覆盖（尤其 G4 代理形态）；Google 不走解析器路线，其可用性由浏览器车道承接。实施批（待排）须带一手 SERP 样本核对（含 RET-B10 的 `/url?q=` 包装形态），不得凭结构记忆写正则。

---

## 12. 无浏览器（纯 HTTP）模式实施批设计输入（2026-09-15，用户口径）

> 用户口径：「无浏览器模式还要考虑过滤，甚至是专门调整一下时间预算。」本节把两项落成实施批（DDG 解析器补充，待排）的输入清单——只登记，不定案。

### 12.1 过滤面（现状 vs DDG 加入后的清单）

**已有**：G2 相关性闸门默认开（前 3 条 ∩ 词集、need=⌈25%×min(|Q|,12)⌉、判负复用 `empty_result` 继续引擎链，作用于**所有引擎**的解析结果且在页抓取之前）；Bing 解析天然排广告（只认 `b_algo`）。

**DDG 解析器落地时须同批考虑**：
1. **DDG html 非结果区排除**：广告块与「Related searches」等不得进结果集（对应 Bing `b_ad` 的天然排除位）；
2. **uddg 跳转**：DDG 结果链接为 `uddg` 跳转参数——G3 解包机制（6 worker/单条 6s/剩余预算钳制）即既定承接件，解析器须产出可被 `needs_network_unwrap` 识别的包装形态；
3. **词集闸门已知偏差同批修**（审查 RET-B6）：封顶 `take(12)` 先 ASCII 后 CJK ⇒ 混排长查询裁光 CJK 词；跨词 bigram（「程入」类）为永不命中死重、抬高 need；CJK 单字查询词集空恒放行——修截断序/加权重需一并定；
4. **低质域名过滤移植与否**：0v 浏览器车道 search 动作已有 Bing 反污染＋低质域名加权，纯 HTTP 车道要不要对齐（两车道口径问题，留裁决）；
5. **交付 URL 清洗**（utm/跟踪参数，RET-B7 现为已知取舍）：DDG `uddg` 解包后的落点 URL 是否顺手清洗。

### 12.2 时间预算面（引擎链全活后的新形态）

**现状三段账**：`T_acquire` 5s（connect_timeout）⊂ `T_first` 10s/引擎 ⊂ `T_overall` 30s；页级 10s/页取 `min(remaining)`；解包段已按剩余预算钳制（修复批）。判据保护：首引擎立即起跑 ⇒ 「首个结果 ≤10s p99」不受链位影响。

**死引擎时代结束后（bing_cn,bing_global,duckduckgo 全活）的新形态与候选调整**：
1. **级联耗尽（主要风险）**：慢失败级联 3×10s 即吃光 T_overall ⇒ 页抓取 remaining=0 ⇒ 命中无正文交付（死引擎时代已实测过同形态）。候选：给页抓取阶段**预留保底预算**（如 `segment_pages×3s` 或 T_overall 的 1/3），引擎链只在「remaining−预留」内接力；
2. **递减引擎预算**：首引擎保 10s（首结果判据），后续引擎 `min(T_first, 剩余×α)` 或固定降档——失败链不再均摊吃满；
3. **模式化默认值**：直连（容器）与代理（真机）网络成本不同，`ORZ_RETRIEVAL_{DEADLINE,SEGMENT,ACQUIRE}_MS` 已全 env 可配——是否按模式给不同默认值（代理下 T_acquire 放宽/直连收紧）留实施批 A/B 定；
4. **硬边界**：任何调整不得破坏「10s = 首个结果上限」判据与「整体 30s 兜底」承前裁决；判据类 wall_ms 口径（自请求发出起算）不变。
