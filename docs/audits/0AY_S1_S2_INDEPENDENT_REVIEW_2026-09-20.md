# 0ay S1/S2 独立审查：设计、实现与符合性（2026-09-20）

> 状态：**只读独立审查完成**；用户令「请对当前实现的 0ay S1/S2 部分进行全面检查，包括设计合理性、
> 实现合理性、设计与实现的符合性」＋「请落文档并按台账纪律进行登记吧，本轮全部问题合成一个总代办项进行排期」。
> 审查对象＝`GAP-RETRIEVAL-SYNTHETIC-JUDGEMENT-AUDITABILITY`（0ay）的 **S1 契约面 ＋ S2 实现**（S3 语料复算随查、
> S4 真机不在本轮）。**本轮零代码改动**：只读取证、复现读数、写本档、登记单总项 **0az**（未闭合 **41 → 42**）。
> 本批**未提交、不推送、不重建**；`orz` pin 与 `orz_source_manifest.sha256` 未动；0am 影批工作树未触碰。
> 全部发现**合成一个总代办项**排期（用户令）＝`GAP-RETRIEVAL-SYNTHETIC-JUDGEMENT-AUDIT-CLOSURE`（0az，P1）。
> 入口：[`0ay S1/S2/S3 实施报告`](0AY_S1_S2_S3_IMPLEMENTATION_2026-09-20.md) /
> [`独立审计与裁决 §2 F-2/F-3`](0AT_0AU_0AV_0AW_0AX_S1_INDEPENDENT_AUDIT_AND_ADJUDICATION_2026-09-20.md) /
> 索引 `GAP-RETRIEVAL-SYNTHETIC-JUDGEMENT-AUDITABILITY` / BACKLOG 0ay / TODO P1-0ay。

## 1. 一句话结论

0ay S1/S2 的**设计方向正确**（判定输入落 ledger、被 digest 覆盖、落计数而非布尔、可由两张独立清单交叉复算），
**S2 是语义等价重构**（零行为变更），**读数全部经独立复现**；但判官侧存在**两处缺口**——生成代际门的键选错
（对 0ax 时代「同批有池＋无池」归档会**误报**）与**判官去重顺序与生产者不同尺**（同 digest 跨类时误报）——
另有 schema 契约面三处不闭合、权威面描述漂移与一处覆盖缺口。判定：**有条件通过**；
S1/S2 现状**不必回滚**，但 **F-1／F-2 应在 S4 真机前处置**（各一处小改＋钉子）。

## 2. 独立复现的读数（本轮实取，非引用实施报告）

| 面 | 实施报告值 | 本轮复现值 | 结论 |
|---|---|---|---|
| orz-loop lib | 819/0/3 | **819 passed / 0 failed / 3 ignored** | 一致 |
| orz-assurance lib | 246/0 | **246 passed / 0 failed**（含 `s2b_family_verdicts_match_python`、`family_verdicts_match_spec_table`） | 一致 |
| orz-assurance 集成 | 9/0、4/0、9/0 | fixture_journal_conformance **9/0**、journal_conformance_cli **4/0**、v1_fake_provider **9/0**（另 journal-conformance bin 0/0） | 一致 |
| pytest 契约面两件 | 283/0 | **283 passed / 0 failed** | 一致 |
| `cargo fmt --all --check` | 全净 | **exit 0** | 一致 |
| 门禁 `check_repository.py` | error_count=1 | **error_count=1**，唯一＝「orz submodule working tree is dirty」（不提交批预期态） | 一致 |
| 生成器 ↔ fixtures | 重跑 diff 仅 1 件 | **330/330 逐字节一致**，无多余/缺失产物（`journals/*.jsonl` 为手维护、不在生成面） | 一致 |
| S3 语料复算驱动 | ALL CELLS MATCH 7/7 | **ALL CELLS MATCH**；旧 37／收窄 **16**／合成 **21** 逐格吻合；混合批控制 clean（usable 2→3、合成不变 3） | 一致 |
| F-3 归属勘误（stale-digest 单件） | 实为 extract-00（…-00-r0-8d）、2 条 | 驱动输出 extract-00＝2、其余六件＝0 | 勘误正确 |
| 0ay 改动面隔离 | orz 4 文件 | `rg -l citation_url_count` 全仓仅命中 `batch_close.rs`／`evidence.rs`／`families_s2c.rs`／`families.rs`，0am 影批未混入 | 成立 |
| 契约 fixture 合法性 | — | `retrieval-result.merged-multi-query.valid.json` 过更新后 schema **0 错** | 成立 |

未复跑（沿用实施报告口径，非本轮结论）：orz-bin 六目标、orz-host 串行（负载敏感族）、clippy「净零新增」。

## 3. 设计合理性

**成立的三点（本设计要害）**

1. **落点正确**：字段挂在 `source_ledger` 条目上，因此被 `result_digest`／`ledger_digest` 覆盖
   （[`evidence.rs`](../../orz/crates/orz-loop/src/retrieval/evidence.rs) 的四段 canonical digest 含 `source_ledger`）
   ——改字段必改 digest，不可静默篡改。
2. **落「计数」强于落「布尔」**：计数能与两张**独立清单**对拍（保留池 `candidate_urls` ＋ `prefilter_log` 移除数），
   布尔只能自证；且原池内容无损可重建（raw ＝ retained ∪ removed），无信息膨胀代价。
3. **登记面勘误处置正确**：BACKLOG/TODO 原稿「合成条目落 `citation_url_count: 0`」与 schema `minimum: 1`、
   与判据②（无引用池批 payload 逐字节不变）**同时不相容**；按 F-3 裁决正文执行「字段缺席」是唯一自洽解。

**设计层面需质疑的一点**：「缺席 ＝ 空池」的编码存在**代际歧义**——判官无法区分「0ay 生产的空池」与
「pre-0ay 生产的池未记录」。设计以「生成代际门」规避，方向正确，但**门的键选错**（见 F-1）。

## 4. 实现合理性

- **S2 是语义等价重构，可证**：`citation_url_count(ev).is_none()` ⇔ `ev.candidate_urls.is_empty()`
  （`batch_close::citation_url_count`），故 `is_synthetic_answer` 行为零变化，仅新增一个落盘字段
  ——符合「只补可核性、不动语义阈值」。
- **单源收敛**：判定实现单点（`is_synthetic_answer`），消费两处（可用计数／合成计数），装配一处
  （`evidence.rs`）——全仓无第二处重复实现该判定（`rg` 实测）。
- **口径同尺核对通过**：生产者 `usable_source_count`／`synthetic_answer_count` 与判官重算在
  **类内去重**语义上一致（除 F-2 的跨类分支）。

**两处实现瑕疵（不影响当前行为）**

1. **装配谓词与判定谓词不同形**：判定＝`source_type == "web_search_result" ∧ helper.is_none()`，
   装配＝`helper.is_some()`（**不含** `source_type`）。当前不可达分叉（`candidate_urls` 仅在
   `tool == "web_search"` 时填充），但 helper 未编码 `source_type` ⇒ 将来若有别的工具带池，
   生产者会写出自己的判官拒收的 payload。
2. **同一谓词写了两遍**：`if !ev.candidate_urls.is_empty()` 与紧随其后的 helper 条件等价（后者恒真），
   属冗余；把 helper 提到外层可消掉。

## 5. 设计与实现的符合性（逐判据）

| 判据 | 结论 | 说明 |
|---|---|---|
| ① journal 侧可独立重算 `synthetic_answer_count` 与收窄 `usable_source_count`（与实现同源同值，含混合批） | **达成但有一处不完整** | 判官对拍 `synthetic_answer_count`；`usable_source_count` **只被算出、无任何声明值与之对拍**（Python/Rust 都不读 `unattributed_usable_count`／逐 query 计数）。属既有（0ar/0at）执行面缺口，由本项一并收口 |
| ② 单 query 批与无引用池批 payload 逐字节不变（可选字段缺席即旧形态） | **达成** | 无池条目不落字段；「单 query 带池批新增字段」例外已登记且必要（S3 语料含三批单 query） |
| ③ 不动阈值 5/10、FP-2、官方口径 | **达成** | 本批零阈值／零 FP-2／零官方口径改动 |

## 6. 发现清单（F-1 … F-7；合成一个总项 0az）

### F-1（P1，潜在误报；窗口明确）判官生成代际门按「声明了 `synthetic_answer_count`」开门

- **位置**：[`assurance/run_event_journal_validation.py:1341`](../../assurance/run_event_journal_validation.py)
  ／`families_s2c.rs:353`（两侧同形）。
- **现象**：门＝`"synthetic_answer_count" in p or any(entry has citation_url_count)`。0ax 生产者**只要本批有合成答案**
  就会写该声明，于是 **0ax 时代（有声明、无新字段）**的 payload 也会开门。
- **复现**：取 `merged-multi-query` fixture，删去全部 `citation_url_count`（＝0ax 生产者形态；有池条目仍在、
  声明 synthetic＝1）⇒ 判官报
  `synthetic_answer_count 1 != mechanical recompute 2 over the ledger`（把有池旧条目也读作合成）。
- **触发条件**：pre-0ay 归档中存在「**同批既有池、又有无池**的 `web_search_result` 条目」。
- **实测边界**：本机全部 **155** 件卡口归档扫描——含池 `web_search_result` 条目 **0 件** ⇒ **今天影响为零**，
  实施报告「旧 journal 回放零新增报错」**不是假读数**，只是被语料单形态掩盖。
- **窗口**：`ORZ_WEB_SEARCH_LOCAL`（0ax S2，待放行）若先于 0ay 落载体，这类归档即会出现；
  届时 0aa「历史卷 journal 全量 verifier 复扫」会直接撞上误报面。
- **建议修法**：门只认「有任一条目带 `citation_url_count`」——这是 0ay 生产者唯一能写、0ax 写不出的标记；
  全空池批不检查也无漏（此时重算恒等于声明值）。同步改 schema `synthetic_answer_count` 的代际门描述。

### F-2（P2）判官与生产者去重顺序不同尺（dedup-first vs class-first）

- **位置**：判官 `_verify_v02_result_consistency`／`recompute_retrieval_batch_counts`（py:1186、rs:147）
  vs 生产者 `batch_close::usable_source_count`／`synthetic_answer_count`。
- **现象**：生产者**先分类后按类内去重**；判官**先去重、首见定类**。同一去重键同时以合成与可用形态出现时，
  两侧分叉。
- **复现**：令 fixture 中 SRC-0004（无池合成）与 SRC-0005（有池可用）**共享 `content_sha256`**，
  并把有池条目排在前面 ⇒ 判官重算 synthetic=**0**，而声明=1 ⇒ 误报
  `synthetic_answer_count 1 != mechanical recompute 0`；同时 `usable_source_count` 失真。
- **触发面**：同一 query 重复发起、同文本但引用池有无不同（SERP 车道启用后为常态；设计本含
  「重复 query 回踩指针」幂等纪律）。概率低但**结构性可达**，且正是本项「禁二把尺」要消灭的一类。
- **注意**：Rust 与 Python 判官**逐字同错**，故既有 `s2b_family_verdicts_match_python` 对拍**测不出**。
- **建议修法**：判官按类内去重（与生产者同尺），或改生产者跨类去重——二者取一，并补钉子。

### F-3（P3）schema 未机器化「仅 `web_search_result` 可带该字段」

- **位置**：`runtime/retrieval-result-event-payload-v0.2.schema.json` `source_entry`（`:286`）。
- **复现**：给 `web_page` 条目加 `citation_url_count: 2` ⇒ **schema 0 错误**、判官报错 ⇒
  「判官接受集 ⊊ schema 接受集」。
- **附带**：`runtime/tests/test_run_event_journal_validation.py` 中该用例注释称「schema 的 exclusivity 面拒收」，
  **与实测不符**（实际由判官族规则拒收），属注释勘误。
- **建议修法**：按同文件既有先例加 `allOf` if/then（`citation_url_count` present ⇒ `source_type` const
  `web_search_result`），使 schema 与判官同集。

### F-4（P3）schema `required` 不含 `prefilter_log`，而新恒等式依赖它

- **位置**：同 schema 顶层 `required`（`:8`）。
- **复现**：从 fixture 删去整段 `prefilter_log` ⇒ **schema 0 错误**、判官报
  `citation_url_count 3 != retained 1 + prefilter removals 0`。
- **说明**：生产者恒写该字段，故非当前故障；但新规则的前置条件未进契约面。
- **建议修法**：`prefilter_log` 进 `required`（或判官在「有计数而缺该字段」时显式报缺件）。

### F-5（P3）schema 缩进漂移一处

- 同 schema `:283`：`candidate_pool.items.$ref` 缩进由 6 空格漂为 5 空格（JSON 合法、语义不变，
  与本文件 1 空格递进风格不一致）。建议随该契约面下次触碰一并修正。

### F-6（P3）语义权威面未同步（描述漂移）

- `query_entry.usable_source_count`（同 schema `:189`）描述仍写「visibility full/partial、按
  `content_sha256` 去重」——**未提 0ax 起的合成排除**，而同文件的 `synthetic_answer_count`
  描述已含窄口径与重算规则 ⇒ 同一契约文件内两处描述自相矛盾。
- 自然语言权威面同样停在上位：`ADR-0010` 止于 **§14.73**（0ar），
  [`RETRIEVAL_BATCH_HANDOFF_AND_ROUND_SEAT_DESIGN_2026-09-19.md`](../RETRIEVAL_BATCH_HANDOFF_AND_ROUND_SEAT_DESIGN_2026-09-19.md)
  §3.3 仍定义 `usable = |{按 content_sha256 去重 : visibility ∈ {full,partial}}|`（宽口径）。
- **建议修法**：随 0az 补 ADR 转录（§14.74 级）＋设计稿勘误注，并把两处 schema 描述对齐。

### F-7（P3）覆盖缺口：完全净化形态与跨类同 digest 无钉

- `citation_url_count` 最有区分度的形态＝**原池非空但过滤后保留为空**（raw 3、retained 0）。
  本轮实测判官判定**正确且 clean**（归可用、不计合成），但仓内无钉把守（既有「fully purified pool」例
  只断言 `candidate_urls==[]`／`prefilter_log.len()==3`，未断言 `citation_url_count`）。
- 混合批单源钉 `citation_url_count_and_judges_agree_on_mixed_batch` 用的是**不同 digest**，
  故 F-2 的跨类同 digest 分支无覆盖。
- **建议修法**：随 F-1／F-2 的钉子一并补（两侧：Rust 单测＋Python 契约钉）。

## 7. 未核与边界

- 本轮**未改任何代码**；F-1 … F-7 的修法**仅登记不实施**（按用户令合成单总项 0az 排期）。
- **未复跑**：orz-bin 六目标、orz-host 串行（已登记负载敏感族）、clippy 相对 HEAD 净零新增——
  三项沿用实施报告口径，本档不据其下结论。
- **不改**：批阈值 5／10、FP-2、`ORZ_WEB_SEARCH_LOCAL` 开关、官方口径（`task.toml`／镜像／verifier／数据集 pin）、
  `orz` pin（`3a1e34c5`）与 `orz_source_manifest.sha256`。
- 环境注记（不入计数）：orz-loop clippy 面需显式 `PROTOC=D:\tb-eval\.tools\protoc-25.3\bin\protoc.exe`，
  与 `ORZ-DEV-LINKER-CRASH-001` 同族；本轮全程 `-j 1`，无 `rust-lld` 崩溃复现。

## 8. 登记与排期（单总项）

- **新立项 `0az`＝`GAP-RETRIEVAL-SYNTHETIC-JUDGEMENT-AUDIT-CLOSURE`**（P1；未闭合 **41 → 42**；
  口径以 [`BACKLOG`](../BACKLOG_AND_PRIORITIES.md) 为准）——把 F-1 … F-7 **合成一个总代办项**排期。
- **建议批序（单批可闭环，各步独立放行）**：① **判官口径收口**（F-1 门改「有字段才校验」＋F-2 类内去重；
  schema 描述同步；两侧钉子含跨类同 digest 与完全净化形态）；② **契约面闭合**（F-3 `allOf` ＋F-4 `required`
  ＋F-5 缩进）；③ **权威面同步**（F-6 ADR 转录＋设计稿勘误＋`usable_source_count` 描述对齐）。
- **判据**：① 0ax 时代「同批有池＋无池」形态不再误报（构造例＋回放例）；② 同一去重键跨类时
  判官重算与声明逐值相等（两侧钉）；③ schema 接受集 ＝ 判官接受集（F-3／F-4 的两条反例已可被 schema 拒收）；
  ④ 阈值 5／10、FP-2、官方口径零改动；⑤ 既有读数不回归。
- 三方同步：BACKLOG（未闭合计数行＋当日流水＋优先级总览 P1 行＋P1 开放项清单＋`### 0az.` 小节）／
  TODO（未闭合总数＋P1 路由行＋`### P1-0az` 小节）／索引（头行 v4.02＋§3 条目＋§8 `pending` 桶）。

---

> 落款：0ay S1/S2 只读独立审查（2026-09-20，主会话）。发现与判定以本文为登记面；处置实施与读数另行登记。
