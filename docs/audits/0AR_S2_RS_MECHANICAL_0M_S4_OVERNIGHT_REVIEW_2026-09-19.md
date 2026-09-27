# 过夜批全面审查报告：0ar S2 ＋ 0aq 机械可修项 ＋ 0m S4 收口（2026-09-19）

> 状态：**审查报告（只读；本件自身未提交）**；日期 2026-09-19。
> 指令口径（用户）：「请先查看CLI_PROJECT_INDEX.md路由，随后回查所需文档。请对当前未提交内容进行全面审查，审查内容包括设计合理性，实现合理性，自己设计与实现的符合性；审查完成后按照项目管理出审计报告即可，不提交不推送不重建」。
> 审查对象：索引 v3.79 头行所指过夜批全部未提交内容——父仓 M×14＋D×8（暂存）＋??×10；orz 子仓 27 件修改＋3 件新增（`batch_close.rs`／`lif/rli.rs`〔0am 影批〕／`rli_shadow_replay.rs`〔0am 影批〕）。
> 处置：全部发现已于同日处置闭合（P1×2 改文＋P2×2 修复/登记＋P3 逐条），回执见 [`0AR_S2_REVIEW_HANDLING_2026-09-19`](0AR_S2_REVIEW_HANDLING_2026-09-19.md)。
> 对照基准：[设计稿 v1.0](../RETRIEVAL_BATCH_HANDOFF_AND_ROUND_SEAT_DESIGN_2026-09-19.md)／[S1 报告](0AR_S1_CONTRACT_SURFACE_2026-09-19.md)／[ADR-0010 §14.73](../../adr/ADR-0010-vol-14-addenda-index.md)／[过夜批报告](0AR_S2_RS_MECHANICAL_0M_S4_OVERNIGHT_2026-09-19.md)／[严格审查 RS-01…RS-18](FULL_PROJECT_STRICT_REVIEW_2026-09-18.md)。
> 证据边界：主会话独立读码（全量 diff 逐 hunk）＋独立工具链复核（本机实跑，见 §3）；未跑 Python assurance 全套与 clippy 全量（改动面已由四个 Rust suite＋门禁＋runtime 覆盖，与过夜批报告 §6 同口径）；S3 真机判据不在本轮可证范围。

---

## 1. 一句话结论

**有条件通过**。工程面扎实：三线全部实锤落地、八个独立复核读数与账面逐一吻合、0aq 十二件处置逐项核实为真、0ar/0aq 与 0am 影批 hunk 级分离属实、账本三处同步（37 → 36）过门禁。但**设计-实现符合性存在两处口径级偏差（P1×2）须在 S3 放行前裁决对齐**：①设计 §5.4／ADR §14.73／TODO 三处写明「阈值逐 query 计数」，实现为批级总计数（且该语义与护栏 10 存在数学张力，需一并裁决）；②宽口径缺 `relevance = direct` 过滤——已提交的 S1 契约 schema 描述、设计 §3.3、TODO 判据口径行均含该条件，实现与 ADR 转录均无之（EvidenceRecord 结构无 relevance 字段，属「定稿口径未经机械计数源字段核对」）。另有 P2×2（生成器 README 模板文本损坏；设计 §10-2 去噪三项未实现且未登记）与 P3 注记五条。**未发现阻断性代码缺陷**；两处 P1 均为「改码或改文」的裁决题，不是事故。

## 2. 审查范围与方法

- **维度**：设计合理性（D1/D2/D3 参数与机制、0aq 处置取向）、实现合理性（正确性/边界/纪律）、设计-实现符合性（设计稿 §3–§7 落点与 §8 判据 1–7 逐条对照；ADR 转录忠实性）。
- **方法**：索引 v3.79 路由 → 过夜批报告 → 设计稿 v1.0 与 S1 报告回查 → orz 子仓 0ar 相关全量 diff 逐 hunk 读码（batch_close/agent_loop/dispatch/effort/evidence/controller/tool_run）＋0aq 面文件抽验 → 父仓 schema/生成器/门禁/账本 diff → 独立工具链复核。
- **批次归属甄别**：共享文件（controller.rs／prompt.rs／acp_server.rs 等）中 0am 影批 hunk（`rli_shadow_enabled_override`／`new_lif_engine`／`wallclock_rounds_line`／`lif/*`／Cargo libm）与 0ar/0aq hunk 可清晰辨识分离——过夜批报告「hunk 级分离」声明属实；本报告仅审 0ar/0aq/0m 面，0am 影批语义面留 O2 裁决批（RS-06 口径）。

## 3. 独立工具链复核（主会话本机实跑）

| 项 | 过夜批账面 | 本审查实测 | 判定 |
|---|---|---|---|
| orz-loop lib | 805/0/3 | **805 passed / 0 failed / 3 ignored** | ✓ |
| orz-host 串行 | 333/0/5 | **333 / 0 / 5**（`--test-threads=1`） | ✓ |
| orz-assurance lib | 246/0 | **246 / 0** | ✓ |
| orz-tui | 178/0 | **178 / 0**（9 例 ACAF 缺席跳过） | ✓ |
| fmt（orz-loop） | 本批 0 差异 | `cargo fmt -p orz-loop -- --check` 干净 | ✓ |
| runtime 全套 | 366 OK | **Ran 366 tests — OK**（261.6s） | ✓ |
| 门禁 | error_count=1（唯一脏树） | **error_count=1**，唯一＝`orz submodule working tree is dirty`；payload 正/负 **72/51** | ✓ |
| pytest 收集 | 2080 项零错误 | **2080 collected / 0 errors**（72s） | ✓ |
| 生成器 | 347 件零差异 | 重跑 exit 0、件数稳定、无新增改动面（`git status` 无新 M/??） | ✓ |
| git diff --check | — | 父仓/orz 两仓均干净 | ✓ |

## 4. 0ar S2 设计合理性（评审）

**总评：参数与机制的一手依据扎实，结构选择正确，设计质量高。**

- **参数有反证支撑**：宽口径口径改动以七批次严口径最大 7 的实测反证立论（§3.3）；300 s 对达标批 206 s 仅 1.45× 余量 ⇒ 「未达标也交回」与墙钟配对（§4.1）——参数不是拍脑袋，是读数驱动。
- **β 收尾形态合理**：工具面机械收空＋唯一收尾回合，既保留「结果深入筛选者」定位又机械保证收尾回合不烧检索；实现把 `close_round_armed` 置于工具面选择链**最顶端**（优先于压缩窗口轮）——收尾回合的工具面收空不可被其它分支覆盖，优先级正确。
- **可见倒数双数分开报**（条目额度／已发起调用数）直接钉死「上限误读为配额」的口径漂移（§3.6），并有单测锁定。
- **合并优先＋溢出拆轮互补结构成立**：合并消除 N 倍往返，溢出无 `ToolStarted` 拒绝＋一次性重述保住主代理回合；判据 4 的「无 ToolStarted」与 0q 漏斗（`stamp_failure(Refused)`）对账物齐备。
- **提前交付反灌水设计完整**：显式标记＋指针硬要求＋缺指针 fail-open＋达标如实归阈值＋连续 ≥3 次 streak anomaly（每 run 复位）——判据 5/7 全链有测试。

**设计自身张力（本审查新指出，待裁决）**：
1. **「阈值逐 query 计数」与护栏 10 数学不相容**——若逐 query 语义＝每 query 满 5 才收尾，3 query 合并需 15 条 > 护栏 10 ⇒ 合并激活将永远先触护栏、逐 query 阈值形同虚设。设计 §5.4 与护栏参数未做联调核对。
2. **定稿口径未经机械计数源字段核对**——§3.3 定稿含 `relevance = direct`，但机械计数发生在工具证据层（`EvidenceRecord`），该结构**无 relevance 字段**（relevance 只存在于 ledger/prefilter 投影层）；定稿时未核对「共用尺」在计数点上是否可计。
3. **§10-2 去噪三项**表述为 β 收尾的既定配套（「只做去噪三项」），但未列入 §7 实现面落点——范围归属天然模糊（见 §6-P2b）。

## 5. 0ar S2 实现合理性（评审）

**总评：实现纪律良好，关键放置决策有据，未发现阻断缺陷。**

- **`retrieval_calls` 计数器放 `SharedLoopServices`（dispatch 持有 AtomicU64）的理由成立**：墙钟到点路径 loop future 被丢弃、loop 局部量随 drop 消失而 dispatch 仍需读数——注释说理＋超时测试动态断言锁定。
- **D2 到点臂改造正确**：`act.conversation` 即 loop 消息缓冲（`&mut` 直传），future 丢弃后取最近非空 assistant 文本成立；`tool_rounds` 保留派发前已耗值（诚实读数）；in-flight 孤儿合成收口保留；合成 `Ok(LoopOutcome)` 走共用收尾路径 ⇒ 部分证据报告 exit 0、assessment 带 `usable_source_count`＋`sufficiency_gap`、close `dispatch_wallclock_bound` 必带 assessment 链——判据 1 全链有测试。
- **D3 事件面纪律完整**：被合并调用 ToolStarted/Completed 成对（真实被服务）；溢出拒绝无 ToolStarted＋exit 1＋`stamp_failure(Refused)` 过 0q 漏斗；`dispatch_bound` 为空时短路安全（`merged_positions.contains` 先短路，无越界索引）。
- **失败臂抑制有痕**：armed close 时吞 `RetrievalSubagentEarlyClose` 改 `tracing::warn!`——确定性失败事实已在链上工具事件留痕，不因之丢掉已达标证据；说理入注释，可辩护。
- **单 query 零漂移**：`build_structured_result` 改 `queries: &[String]` 后单 query 形态（query_text=任务契约全文、result_count=全 ledger 条数）与既有 payload 逐字节一致，有测试锁定；schema 增量全可选 ⇒ 既有 payload 全部保持有效。
- **阈值臂不可能误触发于压缩路径**：会话收尾压缩走 `run_template_compact`（非 run_agent_loop），阈值判定无从触及；主车道 `retrieval_calls=None` 双重隔离。
- **小瑕**（P3，见 §6）：orz README 串行读数落地即过时；提前交付指针只验非空；多 query 下 `tool_used`/`result_count` 归因粒度（已登记为边界）；溢出文案「已派发 1 次」在 3-merged 场景字面歧义（沿设计 §5.5 模板原文）。

## 6. 设计-实现符合性（核心发现）

### §6.1 判据对照（设计 §8）

| 判据 | 过夜批声明 | 本审查核证 |
|---|---|---|
| 1 wallclock 必带计数＋缺口 | ✓ 测试锁 | ✓ `subagent_wallclock_timeout_returns_partial_evidence_normally` 锁 assessment 链全字段；schema allOf 投影一致 |
| 2 单批墙钟 ≤300s＋收尾回合 | ✓（机制） | ✓ 机制面成立（档位表 180/300/450＋到点臂）；真机读数留 S3（如实标注） |
| 3 首批 commit 后主回合 ≥1 | 留 S3 | —（真机判据，不在本轮范围） |
| 4 溢出无 ToolStarted | ✓ 测试锁 | ✓ 溢出测试锁 cause＋无 ToolStarted |
| 5 软硬不一致落 anomaly | ✓ | ✓ `early_delivery_missing_pointer`／`early_delivery_streak`／`early_delivery_at_threshold` 三码齐 |
| 6 query_summary 条目数＝合并 query 数且逐 query 可核 | ✓ | ✓ 合并测试锁 2 条＋逐条 `usable_source_count` |
| 7 提前交付带指针；倒数与计数同口径 | ✓ | ✓ 指针硬校验（非空）；倒数/assessment 共用 `usable_source_count` 单源 helper |

### §6.2 偏差与发现清单

**P1-a｜「阈值逐 query 计数」未实现（实现＝批级总计数）**

- **声明面**：设计 §5.4「子代理对合并任务给结果总结，**阈值逐 query 计数**（判据 6），避免『query A 抓满就整体返回、query B 从未被回答』」；ADR-0010 §14.73(4) 同句转录「阈值逐 query 计数」；TODO `P0-0ar` 判据口径行「阈值须逐 query 计数」。
- **实现面**：`agent_loop.rs` post-batch 阈值判定用 `usable_source_count(&snapshot)`（**全批合计**）；`per_query_usable_counts` 仅用于 `query_summary` 逐条披露，不参与收尾判定。
- **后果**：合并 2–3 query 时，query A 满 5、B/C 零证据即触发 `evidence_threshold_met` 收尾——恰是设计要点名避免的形态；且判据 3（首批 commit 后主回合 ≥1）的收益在「B 未被回答即收尾」时打折。
- **裁决选项**：①改码（逐 query 达标才收尾；护栏同步逐 query 化，需联调 §6-张力 1 的 15>10 矛盾——如护栏改「逐 query 10」或「总 15」）；②改文（设计 §5.4／ADR §14.73／TODO 改为「批级阈值＋逐 query 可核披露」，明示放弃逐 query 收尾语义及理由）。**二者必居其一，不得留三方不一致进 S3**（S3 判据回收以文本为准）。

**P1-b｜宽口径缺 `relevance = direct` 过滤（四方不一致）**

- **含该条件的面**：设计 §3.3 定稿口径（`∧ relevance = direct`）；**已提交的 S1 契约** `information-sufficiency-assessment-event-payload-v0.2.schema.json` 的 `usable_source_count.description`（"visibility in {…} **AND relevance = direct**"）；TODO 判据口径行；fixtures README（S1 遗留句）。
- **无该条件的面**：实现 `batch_close::usable_source_count`（只查 visibility；`EvidenceRecord` 无 relevance 字段可查）；ADR-0010 §14.73(2)（转录时已无声删去——转录自称「设计定稿的 S2 实施转录」，忠实性存疑）。
- **方向评估**：实现比定稿口径**宽**（tangential 全文抓取也入数）⇒ 实际计数 ≥ 设计读数 ⇒ 阈值 5/护栏 10 更早触发。非危险方向，但「同一把尺」的四方文本只剩代码一把真尺。
- **裁决选项**：①改码——relevance 在计数点不可得，需把 prefilter 相关性结论下探进 `EvidenceRecord`（新增字段＋赋值链＋契约增量），或以 `candidate_urls`/`search_query` 代理判定（弱）；②改文——schema description／设计 §3.3／TODO／README 统一收窄为 visibility 口径并登记理由（推荐：一次文本批即可闭合，语义影响如实在 S3 读数中体现）。

**P2-a｜生成器 README 模板文本损坏（本批引入）**

- `scripts/generate_run_event_fixtures.py` 在 fixtures README 模板插入 0ar S1/S2-D3 条目时，把 assessment 描述句句头「Assessment side: optional `usable_source_count` (wide caliber — dedupe by」**整段替换**掉，生成物 `runtime/fixtures/run-event-v0.2/README.md:157-164` 现态为残句：「three normal-close reasons of the batch-handoff contract — \`content_sha256\`, visibility in {…} and relevance=direct) plus optional \`sufficiency_gap\`…」——悬挂右括号、三值枚举名丢失、语义断裂。
- 「生成器重跑零差异」只证确定性，不证语义完整；该损坏将随每次生成永续。随下一父仓批修复模板并重生成。

**P2-b｜设计 §10-2 去噪三项未实现且未登记**

- 全码检索无「空批不唤醒」（自然结束零证据仍以 `retrieval '…' returned no text` 唤醒主代理）、无「重复 query 回踩已有结果指针」（同 query 重发走完整重检索）；「批间不重置任务契约」一条由 M4 continue 语义天然满足。
- 设计 §10-2 将其表述为「不设硬节流 ⇒ 只做去噪三项」的既定配套；过夜批报告／BACKLOG 0ar／TODO 均未登记为遗留或豁免。**至少应登记**（S3 前补文本或明示并入 S3 范围）。

**P3 注记（不阻断）**

1. orz README（RS-04②）串行口径写死「332 passed / 0 failed / 5 ignored」——同批 RS-07 新测试使实态 333，**落地即过时**（本审查实测 333/0/5）；随下一批改 333 或去数字化。
2. 倒数行载体：设计 §3.6 写「随该次工具结果**入 journal**」，实现随模型面消息/会话侧车（ToolCompleted payload 不含追加行）——过夜批报告已登记「落点等价偏差」（tool_run.rs → agent_loop.rs 两执行点），但「journal → 会话侧车」的**可追溯性差异**未单列：倒数读数不能独立从 journal 复原，需读会话侧车。
3. 提前交付指针校验宽松：只验非空（`parse_early_delivery` 后 `!pointer.is_empty()`），不验指针形态/指向真实性——设计「必须带证据指针」实现为形态要求；fail-open 哲学下可接受，后续可按 `ledger`/`archive` 前缀轻校验。
4. 多 query 归因粒度：`tool_used` 取全批首条、派生证据（`search_query=None`）不归因、`result_count` 多 query 下按 search_query 精确匹配计数——schema description 与 `per_query_usable_counts` 注释已如实登记，属边界非缺陷。
5. 溢出拒绝文案「本轮已派发 1 次检索」在 3-query 合并场景字面歧义（1 个激活 vs 3 个调用）——沿设计 §5.5 模板原文，非实现自创；若裁决改文可顺手澄清。

## 7. 0aq 十二件与父仓杂项核证

| 项 | 声明 | 核证结果 |
|---|---|---|
| RS-03 | 920K 两处改写＋异常空格顺修 | ✓ `agent_loop.rs` 工作台提醒（500K 硬截断口径）＋`controller.rs` blackboard_write 描述（「durable memory…500K hard truncation (T1)」＋空格顺修）；checkpoint.rs/注释处历史 920K 如实标注不动 |
| RS-04①② | tui 9 例 ignorable 化＋串行口径文档化 | ✓ tui 178/0 实测；orz README Development 节增串行说明（**但数字 332 已过时，见 P3-1**）；③assurance 分层如实留开放 |
| RS-05 | 五热点 144 处＋Top-10 六处 | ✓ 逐文件计数 **53+25+19+20+27=144** 实测吻合；Top-10 六处（#3 codex_app 直用刚赋值／#4 recorder 降级日志／#6 state_machine expect 化／#7 transport just-pushed expect／#8 cdp prefs Result 化／#9 action_ledger matches! 合一）逐一在 diff 中确认；余 4 处结构性留渐进如实 |
| RS-07 | 跨声明面 deny 路径遍历测试 | ✓ `permission_bridge_decides_every_declared_work_tool`：WORK_TOOLS 25 工具全遍历＋ReadOnly 自动放行反向钉（0aj 族）＋两遍一致性；host 333/0/5 实测含本测试 |
| RS-08 | 两巨石入勘察候选 | ✓ 勘察档 §6 增补；行数 **8,017／6,274** 精确复测吻合；登记不立项口径清楚 |
| RS-10 | 8 件摘跟踪、文件保留磁盘 | ✓ D×8 已暂存（随下批生效）＋??×8 磁盘在；`tracked log 归零` 口径成立。**复核批已补忽略规则**：`??` 残留归零、`git check-ignore` 8/8（[处置回执 §7](0AR_S2_REVIEW_HANDLING_2026-09-19.md)） |
| RS-11 | cargo-target 迁移＋key 确认 | ✓ `.gsa/cargo-target` 迁 `D:\tb-eval\orz-cache\`（目录在）；`secret_material_persisted_in_metadata: false` 口径登记 |
| RS-12 | 双 README PROTOC 前置 | ✓ 父仓＋orz README 双处，含「约 20 分钟后才失败」警示 |
| RS-13b/c | 索引注记＋releases 占位 | ✓ GAP-TOOL-BUDGET 现行为注记；`releases/orz-0.3.1-linux-x86_64/README.md` 占位（断档点有意化＋命名约定登记） |
| RS-14 | gitignore/gitattributes | ✓ `/.pytest_cache/`、`/.agents/`；`*.exe/*.png/*.ico binary` |
| RS-16 | pytest 配置收敛 | ✓ pyproject 死配置删除；pytest.ini `norecursedirs`（含存档/prototype/evaluation/regression）；evaluation/regression 去误导 README；2080 零错误收集实证 |
| 未动五项 | RS-06/09/15/17/18 留裁决 | ✓ BACKLOG 勾选态如实（未勾） |

**0m S4 收口**：✓ S3 勾选（2026-09-07 经 0o T1 达成，注记回填）＋S4 勾选；BACKLOG 0m 节头闭合标记、P0 总览行与开放项清单摘除 0m、计数 37 → 36 三处同步（§8）。

**契约面（S2 增量）**：✓ `query_entry.usable_source_count`（integer ≥0 可选，description 完整记载归因口径与边界）；新正例 fixture `retrieval-result.merged-multi-query.valid` 生成器单一事实源＋门禁登记（72/51 实测）；ADR §14.73/v1.75 六条转录在档（忠实性两处存疑见 P1-a/P1-b）。

## 8. 账本一致性

- 计数 **37 → 36**：BACKLOG 未闭合总数行／P0 优先级总览行／P0 开放项行／TODO 路由行／索引 v3.79 头行五处同步；0m 移入已闭合清单（含日期注记）。
- 门禁 `error_count=1`（唯一＝orz 脏树预期态）含计数一致性机械检查通过；`git diff --check` 两仓干净。
- 索引/BACKLOG/TODO 对「0ar S2 未提交、0aq 十二件、0m 闭合」的叙述与实态一致；RS-10 暂存删除态「随下一提交批生效」已在报告与 BACKLOG 行登记——口径诚实。**复核批补遗**：RS-10 磁盘件 `??` 残留已由 `.gitignore` 三式规则收口（回执 §7.1）；TODO 0aq 勾选态已与 BACKLOG 对齐；0m 闭合经用户 2026-09-19 确认维持（计数三方锚点 37 一致，回执 §7.2）。
- 本审查报告自身按指令**不入账**（不更新索引/BACKLOG/TODO），登记留收尾批随批执行。

## 9. 结论与建议

**结论：有条件通过。** 三个维度分评：

- **设计合理性：良好**——参数读数驱动、机制结构正确；扣分项为 §4 所列三处设计自身张力（逐 query 阈值×护栏 10 不相容／定稿口径未对机械计数源字段核对／去噪三项范围归属模糊）。
- **实现合理性：良好**——关键放置决策有据、事件面与漏斗纪律完整、零漂移与诚实读数有测试锁定；无阻断缺陷。
- **设计-实现符合性：两处口径级偏差（P1×2）**——「逐 query 阈值计数」与「relevance=direct」均属多方文本声明 vs 单方实现落地的三方/四方不一致；按 §0.1 权威顺序，实现偏离已登记设计须标 `gap` 不得静默——当前状态即静默偏离，**须在 S3 放行前裁决对齐（改码或改文）**。

**建议的处置序列**（供裁决，不在本批执行）：

1. **P1-a／P1-b 裁决对齐**（可并入同一文本/代码小批）：推荐 P1-b 采改文（四方文本收窄为 visibility 口径，理由＝relevance 在机械计数点不可得＋方向偏宽非危险）；P1-a 二选一（改码则需联调护栏参数，改文则 S3 判据回收口径同步改写）。
2. **P2-a 生成器模板修复**随下一父仓批（改模板＋重生成＋门禁）。
3. **P2-b 去噪三项登记**：并入 0ar S3 范围或明示豁免，一处账本行即可。
4. **P3-1 README 333** 顺手改；P3-2/3/4/5 留注记不立项。
5. 上述闭合后 **0ar S3 真机复验放行**不受影响（判据 3 与五项验收读数本就留 S3 收取）。

——审查完毕。本件为只读审计产物：未提交、未推送、未重建、未动被审内容与账本。
