# 0az 检索合成判定面收口：实施与验证（2026-09-20）

> 状态：**实施完成（四子项全部落码），判据 ①–⑤ 全部达成**。用户令「请开始处理 0az 吧，先明确计划随后处理掉
> 0az 内部的待办项」。对象＝`GAP-RETRIEVAL-SYNTHETIC-JUDGEMENT-AUDIT-CLOSURE`（0az）的四个子项：判官口径收口
> （F-1／F-2）／契约面闭合（F-3／F-4／F-5）／权威面同步（F-6）／钉子（含 F-7）。
> 本批**未提交、不推送、不重建**；`orz` pin（`3a1e34c5`）与 `orz_source_manifest.sha256` 未动；0am 影批工作树
> **未触碰**（本批 orz 侧只碰 `journal/families.rs`、`journal/families_s2c.rs` 两个判官文件）。计数：立项 41 → 42
> （上一批），**本批闭合 42 → 41**（口径以 [`BACKLOG`](../BACKLOG_AND_PRIORITIES.md) 为准）。
> 入口：[`0ay S1/S2 独立审查`](0AY_S1_S2_INDEPENDENT_REVIEW_2026-09-20.md)（F-1…F-7）／
> [`0ay S1/S2/S3 实施报告`](0AY_S1_S2_S3_IMPLEMENTATION_2026-09-20.md)／[`ADR-0010 §14.74`](../../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)／
> BACKLOG 0az／TODO P1-0az。

## 1. 计划与执行

BACKLOG 0az 的方案为「单批三步、各步独立放行」。本轮用户实施令覆盖为**一次性交付**，执行时仍保持逐步自证
（每步先改、后跑该步的钉子）：

| 步 | 内容 | 落点 |
|---|---|---|
| ① | 判官口径收口：F-1 代际门／F-2 类内去重／审查 §5 声明面收口 | `assurance/run_event_journal_validation.py`、`orz/crates/orz-assurance/src/journal/families_s2c.rs` |
| ② | 契约面闭合：F-3 `allOf`／F-4 `required`／F-5 缩进 | `runtime/retrieval-result-event-payload-v0.2.schema.json`＋fixture 两件＋生成器 |
| ③ | 权威面同步：F-6 | 同 schema 描述、ADR-0010 §14.74（v1.76）、0ar 设计稿 §3.3 勘误注（v1.2） |
| ④ | 钉子（含 F-7 覆盖缺口） | `runtime/tests/test_run_event_journal_validation.py`、`runtime/tests/test_run_event_conformance.py`、`journal/families.rs` 对拍语料 |

**边界遵守**：只动判官与契约／文档面，**未动** 0ay S1/S2 的生产语义（`citation_url_count` helper／装配点零改动）；
阈值 5／护栏 10／FP-2／官方口径（`task.toml`／镜像／verifier／数据集 pin）零改动。

## 2. ① 判官口径收口（F-1／F-2／审查 §5）

**F-1 生成代际门**（P1，潜在误报）：门由「声明了 `synthetic_answer_count` **或**任一条目带 `citation_url_count`」
改为**只认「有任一条目带 `citation_url_count`」**——这是 0ay 生产者唯一能写、0ax 生产者写不出的标记。两侧判官
（Python 冻结镜像／Rust 离线判官）同形修改。全空池批**有意不检查**：此时每条 `web_search_result` 按构造皆为
无池条目，声明值不含复算之外的信息。

**F-2 类内去重**（P2，二把尺）：复算与生产者同尺——合成类与可用类**各自**按去重键（`content_sha256`，回退
`identity:`）去重，同一键可合法在两个类各计一次（生产者正是「先分类后去重」）。此前判官「先去重后分类、首见定类」，
同键跨类时丢计一次。

**审查 §5 判据 ① 后段收口**：收窄后的**声明可用面**此前只被算出、无人对拍。本批补两条（均在代际门内，避免拿
pre-0ax 的宽口径声明去撞窄口径复算）：

- 单 query 激活：`query_summary[0].usable_source_count` 按构造即批级窄口径值 ⇒ **逐值对拍**；
- 多 query 激活：`unattributed_usable_count` 是 `批级 − Σ 逐 query` 的**饱和差**，声明的三元组必须复现它
  （披露恒等式本身只在逐 query 桶按 digest 互斥时成立；重叠使缺口饱和到 0，这是生产者归因的性质，不是本对拍的条件）。

## 3. ② 契约面闭合（F-3／F-4／F-5）

- **F-3** `source_entry` 增 `allOf` 子句：`citation_url_count` 出现 ⇒ `source_type` const `web_search_result`。
  实测（bootstrap 同源校验）：`web_page` 带该字段 → 1 条 schema 错（`'web_search_result' was expected`），此前 0 错。
- **F-4** 顶层 `required` 增 `prefilter_log`（新恒等式 `citation_url_count == retained ＋ removals` 的前置条件；
  生产者恒写该字段）。实测：删整段 → 1 条 schema 错（`'prefilter_log' is a required property`），此前 0 错。
- **F-5** `candidate_pool.items.$ref` 缩进由 5 空格回正 6 空格（本文件 1 空格递进风格）。
- **fixture 随批（预期变化 3 件）**：`retrieval-result.minimal.valid.json` 与 `retrieval-result.valid.json`（v0.2 信封）
  补 `"prefilter_log": []`；`retrieval-result.tier-weight-mismatch.constraint.invalid.json` 由同一正例派生、同步带上该字段。
  `retrieval-result.merged-multi-query.valid.json` 逐字节不变。生成器（`scripts/generate_run_event_fixtures.py`
  的 `PAYLOAD_GOOD_V02`）同批补字段，重跑后**生成器 ↔ fixtures 330/330 逐字节一致（0 差异）**。
- **F-3 附带（注释勘误）**：`test_only_web_search_entries_may_carry_the_count` 原注释称「由 schema 的 exclusivity 面拒收」
  与实测不符，本批更正为「schema + 族规则两侧都拒收」，并把族规则改为直调（跨层检查在 schema 非法输入上按设计不跑）。

## 4. ③ 权威面同步（F-6）

- schema `query_entry.usable_source_count` 描述由**宽口径**改为**窄口径**（文本级 ∧ 排除无 URL 合成答案 ∧ 类内去重），
  与同文件 `synthetic_answer_count` 描述、倒数行、阈值／护栏同尺；同一契约文件内不再两说。
- **ADR-0010 §14.74（v1.76）**新增：窄口径定案（0ax S1）／判定输入落 journal（0ay S1/S2）／判定面收口（0az ①/②）／
  声明面可核（0az ①）／不变量／边界与余项六条，并在头部「冻结版本补记」登记 v1.76 摘要。**本条第 1 项修订
  §14.73 第 2 条的宽口径**。
- **0ar 设计稿 v1.2 勘误注**：[`RETRIEVAL_BATCH_HANDOFF_AND_ROUND_SEAT_DESIGN_2026-09-19`](../RETRIEVAL_BATCH_HANDOFF_AND_ROUND_SEAT_DESIGN_2026-09-19.md)
  §3.3 增勘误注（窄口径指向 ADR §14.74，其余内容与一手读数照旧有效）＋§13 增 v1.2 行＋头注说明；**零行为变更**。

## 5. ④ 钉子（含 F-7 覆盖缺口）

**Python 契约钉（`test_run_event_journal_validation.py`，+5）**

| 钉 | 覆盖 |
|---|---|
| `test_0ax_era_mixed_batch_replays_without_false_positive` | F-1：0ax 时代「同批有池＋无池」形态回放 **clean**（并断言旧形态复算确实读出 2 ⇒ 门必须关） |
| `test_same_dedup_key_in_both_classes_counts_each_class` | F-2：同 digest 跨类（有池在前）⇒ 声明 1 ＝ 复算 1、可用面仍计该 digest |
| `test_fully_purified_pool_is_usable_not_synthetic` | F-7：**完全净化形态**（原池 3、保留 0、3 条移除）落计数 ⇒ 归可用、不计合成、`citation_url_total`=3 |
| `test_declared_single_query_usable_must_match_the_ledger` | §5 收口：单 query 声明 9 ↔ 复算 3 ⇒ 报错 |
| `test_multi_query_unattributed_usable_identity` | §5 收口：Σ 2 ＋缺口 1 ＝ 复算 3 通过；缺口改 0 ⇒ 报错 |

**Schema 钉（`test_run_event_conformance.py`，+1）**：`test_v02_retrieval_citation_clauses_are_machine_enforced`
——两条反例（`web_page` 带字段／删 `prefilter_log`）由 schema 本体拒收，钉住「schema 接受集 ＝ 判官接受集」。

**Rust 钉（`journal/families.rs` 对拍语料 ＋3，场景 253 → 256）**：`result_consistency_citation_audit_0ax_era_mixed`（clean）、
`result_consistency_citation_audit_cross_class_digest`（clean）、`result_consistency_usable_declared_drift`（违规）。
三个场景同时进入 **Rust↔Python 逐族对拍**（每场景 × 全族逐格断言），故两侧判官任意一侧走偏都会被抓住。

## 6. 验证读数（本批实取）

| 面 | 读数 | 说明 |
|---|---|---|
| `runtime/tests` 全量 | **378 passed / 0 failed** | 含契约面两件 288/0（本批 +6 钉） |
| orz-assurance lib | **246 passed / 0 failed** | 含 `s2b_family_verdicts_match_python`（**256 场景**×全族逐格）、`family_verdicts_match_spec_table` |
| orz-loop lib | **819 passed / 0 failed / 3 ignored** | 与基线同值，零回归（含 `evidence.rs` 的 0ay 混合批单测） |
| `cargo fmt --all --check` | exit 0 | 本批唯一 fmt 漂移已在批内修正 |
| `cargo clippy -p orz-assurance --all-targets` | 本批**零新增** | 本批新增的唯一告警（`collapsible_if`）已在批内重构消除；其余告警均为既有 |
| 生成器 ↔ fixtures | **330/330 逐字节一致** | 重跑后 0 差异；本批 fixture 变化恰为预期 3 件 |
| S3 语料复算（`D:\tb-eval\_0ay_s3_recompute.py`） | **ALL CELLS MATCH** | 七批逐格：旧 37／收窄 16／合成 21；混合批控制 clean（见下） |
| 0az 控制驱动（`D:\tb-eval\_0az_judgement_face.py`） | **ALL CONTROLS AS EXPECTED** | 见下三行 |
| ├ F-1 控制（真实语料注入有池条目、无字段） | 旧判官 **误报** `4 != 5` ⇒ 0az 判官 **clean** | 在真实归档上复现「门键选错」的误报面 |
| ├ F-2 控制（同 digest 跨类、有池在前） | 旧判官 **误报** `4 != 3` ⇒ 0az 判官 **clean** | 在真实归档上复现「二把尺」的误报面 |
| └ 155 件归档扫描 | 0az 新规则命中 **0** | 门／声明可用对拍／F-3 排他／F-4 必填四条在现存语料上零新增报错 |
| 155 件归档有池条目 | **0 件** | 复核上一批读数：F-1 今天零影响，`ORZ_WEB_SEARCH_LOCAL` 启用后即成真 |

**S3 混合批控制的一处随批修正**：0ay 批的 `mixed_batch_from` 只修了计数与 digest，未修**声明的单 query 可用面**；
0az ① 的新对拍因此在该控制 payload 上报出一次 `usable_source_count 5 != recompute 3`。该 payload 不是任何生产者
能写出的形态（0ay 生产者的单 query 声明恒等于窄口径批级值），故驱动随批补上「声明面同修」，并在文件内注明；
S3 七批读数与 0ay 报告口径零变化（旧 37／收窄 16／合成 21）。

## 7. 判据逐条核对（BACKLOG 0az 判据 ①–⑤）

| 判据 | 结论 | 证据 |
|---|---|---|
| ① 0ax 时代「同批有池＋无池」不再误报（构造例＋回放例） | **达成** | 构造例＝Python 钉；回放例＝0az 控制驱动 F-1（真实归档） |
| ② 同去重键跨类时判官重算与声明逐值相等（两侧钉） | **达成** | Python 钉 + Rust 对拍场景（跨类同 digest，clean） |
| ③ schema 接受集 ＝ 判官接受集（F-3／F-4 反例可被 schema 拒收） | **达成** | schema 直测两条反例各 1 错；conformance 钉 |
| ④ 阈值 5／10、FP-2、官方口径零改动 | **达成** | 本批改动面清单＝判官两文件＋schema＋fixture 3 件＋生成器＋测试＋文档，无阈值／FP-2／pin 改动 |
| ⑤ 既有读数不回归 | **达成** | orz-loop 819/0/3、orz-assurance 246/0、runtime 378/0、S3 七批 MATCH、155 件扫描零新增 |

## 8. 边界、未做项与余项

- **多 query 逐 query 桶的值不对拍**（有意）：归因键 `search_query` 精确匹配（逐 query 计数的来源）与 A 面
  `origin_query_id` 谱系是两套键，判官只对拍 **Σ 逐 query 与缺口**，不复算逐 query 值。已写入 ADR §14.74 第 6 项，
  属**边界**而非未闭合缺口。
- **未做**：0ay S4 同三题真机读数（待放行；0az 不阻塞 0ay 闭合裁决）；不动 0ay 生产语义；不引入新的合成判定阈值。
- **未复跑**：orz-bin 六目标、orz-host 串行（负载敏感族）——本批未触碰其输入面，沿用既有口径，不就其下结论。
- **环境**：clippy 需显式 `PROTOC=D:\tb-eval\.tools\protoc-25.3\bin\protoc.exe`；全程 `-j 1`（`ORZ-DEV-LINKER-CRASH-001` 同族），本轮无 `rust-lld` 崩溃。

## 9. 登记

- **0az 闭合：未闭合 42 → 41**（口径以 BACKLOG 为准）；TODO `P1-0az` 四子项全部勾选。
- 三方同步：BACKLOG（未闭合计数行＋当日流水＋优先级总览 P1 行＋P1 开放项清单＋`### 0az.` 小节）/ TODO（未闭合总数
  ＋P1 路由行＋`### P1-0az`）/ 索引（头行 v4.03＋§3 条目状态 `pending` → `implemented`＋§8 桶迁移）。
- 权威面：ADR-0010 §14.74／v1.76、0ar 设计稿 v1.2 勘误注、schema 描述对齐。

---

> 落款：0az 检索合成判定面收口（2026-09-20，主会话）。读数为本批实取；未提交、不推送、不重建。
