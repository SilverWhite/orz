# 0ay S1/S2/S3 实施报告：检索合成判定面落盘与可核复算（2026-09-20）

> 用户令：「请直接开始 0ay S1/S2/S3 部分吧」（0ay＝`GAP-RETRIEVAL-SYNTHETIC-JUDGEMENT-AUDITABILITY`，
> P1，来源＝0ax S1 独立审计 F-2/F-3）。范围＝**S1 契约面 → S2 实现 → S3 语料复算**，
> 按立项批序逐步落账、不跳步；**S4 真机留待**（承接 0ax S4）。
> 本批**未提交、不推送、不重建**；未闭合计数**不变（41 项）**；`orz` 子仓 0am 影批（RLI）
> 未触碰（hunk 级分离保持）。

## 1. 一句话结论

判定输入已落盘并可独立复算：`web_search_result` ledger 条目在**原引用池非空**时携带
`citation_url_count`（原池条数），与保留池 ＋ `prefilter_log` 移除数构成机械恒等式
`citation_url_count == len(candidate_urls) ＋ 该源移除条数`；判官（Rust 家族 ＋ Python 冻结镜像）
由 ledger **单源重算** `synthetic_answer_count` 并与自报值对拍，重算口径与生产者同一 helper
（`batch_close::citation_url_count`，禁二把尺）。S3 七批归档逐格复算 **7/7 吻合**（旧 37／收窄 16／合成 21），
旧 journal 回放**零新增报错**（字段可选＋生成代际门），混合批正例在真归档上验证「有引用池 ⇒ 计可用、非合成」。

## 2. S1 契约面（四件同批）

| 件 | 落点 | 内容 |
|---|---|---|
| schema | `runtime/retrieval-result-event-payload-v0.2.schema.json` | `source_entry` 新增可选 `citation_url_count`（integer，minimum 1）；根描述与 `synthetic_answer_count` 描述补 0ay 段（含生成代际门说明） |
| fixture | `scripts/generate_run_event_fixtures.py` → `runtime/fixtures/run-event-v0.2/payloads/retrieval-result.merged-multi-query.valid.json` | 审计 F-3 勘误＋可核性正例（见 §2.1）；生成器重跑 diff **仅此 1 件** |
| Python 镜像 | `assurance/run_event_journal_validation.py` | 新增模块级 `recompute_retrieval_batch_counts()`；`_verify_v02_result_consistency` 增四条判定（形状／独占／子集／分区恒等式）＋自报对拍（含代际门） |
| Rust verifier | `orz/crates/orz-assurance/src/journal/families_s2c.rs` | 新增 `RetrievalBatchCounts` ＋ `pub fn recompute_retrieval_batch_counts()`；`verify_result_consistency` 同规则执法；`families.rs` 增两场景（ok／mismatch）并把合成场景计数 251 → **253** 登记 |

### 2.1 fixture 重写（F-3）与一处登记面勘误

- **SRC-0004**（合成条目）原为 URL 形态（`source_url_or_ref="https://search.example/synth"`）——生产端
  **不可产出**（`web_search_result` 条目 identity 恒取检索 query 串）：改写为查询串形态
  （`rust channel docs official reference`，且不与任何派发 query 逐字相等 ⇒ 谱系按 leader 兜底规则归 `QRY-0001`，
  与既有 `origin_query_id` 一致）。
- **SRC-0005**（新增，有引用池正例）：`citation_url_count=3` ∧ `candidate_urls`＝1（docs.rs tokio select）∧
  `candidate_pool`＝1 ∧ `prefilter_log` 2 条移除 ⇒ 恒等式 **3 = 1 + 2**；该条目为可引用证据 ⇒ 计可用、
  不计合成。连带同步：`source_counts.total 4→5`／`partial 3→4`、`QRY-0001-2` 逐 query 可用 1→2
  （批级可用 4 ＝ Σ逐 query 1＋2 ＋ 未归因 1，恒等式保持）、`raw_source_refs` 增该条镜像、`prefilter_log` 由空转 2 条。
- **登记面勘误（就地）**：BACKLOG／TODO 的 0ay S1 行曾把合成条目写成「查询串 identity ＋ `citation_url_count: 0`」，
  与 F-3 裁决正文（「合成条目改查询串形态 ＋ 新增 `citation_url_count` 示例；同时给出『有引用池』正例」）
  及判据 ②（无引用池批 payload 逐字节不变）**互相冲突**（落 0 会改变无池批形态）。本批按**裁决正文＋判据**执行：
  合成条目＝查询串形态 ＋ **字段缺席**（缺席即空池，判官据此判合成），计数示例落在「有引用池」正例
  （`citation_url_count ≥ 1`）。BACKLOG／TODO 该行已补勘误指针。

## 3. S2 实现（装配面单源）

- `orz/crates/orz-loop/src/retrieval/batch_close.rs`：新增 `pub(crate) fn citation_url_count(ev) -> Option<usize>`
  （原池为空 ⇒ `None`；非空 ⇒ 原池条数）；`is_synthetic_answer` 改为经**同一 helper** 判定
  （`web_search_result ∧ citation_url_count(ev).is_none()`）。
- `orz/crates/orz-loop/src/retrieval/evidence.rs`：ledger 装配点在同一 helper 上落 `citation_url_count`（原池非空才落；
  紧邻既有 `candidate_urls`/`candidate_pool` 装配块）⇒ 「判定输入 == 落盘字段」，无第二把尺。
- 恒等式由构造保证：`prefilter()` 把输入池**划分**为 `retained + removed`（`candidate_prefilter.rs:426-474`），
  落盘面即 `candidate_urls = retained`、`prefilter_log = removed`、`citation_url_count = len(raw)`.

## 4. 判据对照（TODO P1-0ay）

| 判据 | 结果 | 证据 |
|---|---|---|
| ① journal 侧可独立重算 `synthetic_answer_count` 与收窄 `usable_source_count`（与实现同源同值，含混合批） | **达成** | Rust 单测 `citation_url_count_and_judges_agree_on_mixed_batch`（生产 helper ↔ 判官重算逐值相等：合成 1／可用 1／条目 2／原池合计 3）；判官族规则 + Python 镜像重算同值；S3 七批复算见 §5 |
| ② 单 query 批与无引用池批 payload 逐字节不变（可选字段缺席即旧形态） | **达成（按「缺席即旧形态」口径）** | 无池条目**不落**该字段（`ledger[1].get("citation_url_count").is_none()` 钉），故无引用池批与旧形态逐字节一致；`citation_url_count≥1` 由 schema `minimum` 与判官双锁。**口径说明**：单 query 批**带引用池**时该条目新增字段（digest 随内容重算）——否则单 query 批不可核，而 S3 语料中三批（gpt2-01／extract-00／extract-01）正是单 query 批，0ay 的核心目的即覆盖它们；本批按此执行并在 §2.1 登记面勘误中留痕 |
| ③ 不动批阈值 5／10、不动 FP-2、不动官方口径（`task.toml`／镜像／verifier／数据集 pin） | **达成** | 本批零阈值／零 FP-2 改动；父仓仅 schema/fixture/镜像/账本，`orz` 仅两个 retrieval 文件 |

## 5. S3 语料复算（`official-verify-timeout3-s3` 七批归档）

驱动件（仓外件，导入**仓内**镜像函数，逻辑单源）：`D:\tb-eval\_0ay_s3_recompute.py`

| 批 | 旧宽口径 | 收窄可用 | 合成 | 过夜批报告 | schema | 判官族 |
|---|---:|---:|---:|---|---:|---:|
| torch-00 | 5 | 1 | 4 | 5/1/4 | 0 | 0 |
| torch-01 | 6 | 4 | 2 | 6/4/2 | 0 | 0 |
| gpt2-00 | 5 | 0 | 5 | 5/0/5 | 0 | 0 |
| gpt2-01 | 5 | 4 | 1 | 5/4/1 | 0 | 0 |
| extract-00 | 5 | 2 | 3 | 5/2/3 | 0 | 2（**已登记历史单件**） |
| extract-01 | 5 | 0 | 5 | 5/0/5 | 0 | 0 |
| extract-02 | 6 | 5 | 1 | 6/5/1 | 0 | 0 |
| **合计** | **37** | **16** | **21** | 37/16/21 | **0** | 2 |

- **逐格吻合 7/7**（收窄列合计 **16**＝37−21，与过夜批报告「37 条可用中 21 条为无 URL 合成」同一读数）。
- **旧 journal 回放零新增报错**：七件归档过更新后 schema **0 错**；判官族仅 extract-00 报 2 条，
  即已登记的历史单件 stale digest（旧二进制产出，见 §5.1 勘误）——0ay 不新增任何报错。
- **混合批正例**（真归档 extract-00 ＋ 注入 1 条有引用池条目，计数与 digest 就近修复）：收窄可用 2 → **3**、
  合成**不变**（3）、`citation_entries=4`、`citation_url_total=2`，判官族**全绿** ⇒ 「有引用池 ⇒ 计可用、非合成」在真 payload 形态上成立。

### 5.1 登记面勘误（历史单件文件标注）

`0AT_0AU_0AV_0AW_0AX_S1_REVIEW_FIXES_2026-09-20.md` §3 末行把 stale-digest 单件标为
「extract-02 批（…-02-r0-51）」；本批按三处独立读数（独立归档文件 digest 重算、run 3 journal 事件 **167**
（activation `…-00`）复算、七件逐件判官）确认**该单件实为 `extract-00`（…-00-r0-8d）**；
extract-02（…-02-r0-51）与其余五件 digest 一致。原行读数（2 条）与定性（旧二进制真实违约、不再深挖）不变，
仅文件标注更正（已在该档补勘误指针）。

## 6. 测试读数（本批实取）

| 面 | 读数 | 备注 |
|---|---|---|
| orz-loop lib | **819 passed / 0 failed / 3 ignored** | 基线 818/0/3 → ＋1（混合批单源钉） |
| orz-assurance lib | **246 passed / 0 failed** | 含 Rust↔Python **逐族判词对拍**（新增两场景随批）与 fixture 判官测试 |
| orz-assurance 集成 | fixture_journal_conformance 9/0、journal_conformance_cli 4/0、v1_fake_provider 9/0、另 1/0 | 全绿 |
| orz-bin 全目标 | main 12/0、signer 15/0、provision 2/0、acaf_e2e 23/0、real_flag 2/0、stdio_e2e 1/0 | 与上批一致 |
| orz-host lib（串行） | 316–318 passed / 5 ignored、**2–7 件真进程例失败** | 注册的**负载敏感**族（`call_tool_timeout_*`／`run_tests_timeout_*`／`run_terminal_cmd_truncation_*`／`process_tree::sweep_*`／`resource_hint_*`）——本轮四/两件**隔离单跑全绿**；本批未触 `orz-host` 任何文件（`acp_server.rs` 脏面属 0am 影批既有） |
| pytest | 契约面两文件 **283 passed / 0 failed** | 基线 277/0 → ＋6（0ay 六钉：正例／分区恒等式／缺保留池／非检索条目／自报漂移／pre-0ay 回放） |
| fmt / clippy | `cargo fmt --all --check` 全净；clippy 相对 HEAD **净零新增** | 首跑 own `unnecessary_lazy_evaluations` 1 条已就地改 `then_some`；环境注记见 §7 |
| 门禁 `check_repository.py` | `error_count=1`（唯一＝「orz submodule working tree is dirty」） | 不提交批预期态（前两批同形） |
| fixture 生成器 | 重跑 diff **仅限 `retrieval-result.merged-multi-query.valid.json` 1 件** | 生成器与既有 fixtures 同步 |
| S3 复算驱动 | `python -X utf8 D:\tb-eval\_0ay_s3_recompute.py` → **ALL CELLS MATCH**（exit 0） | 七批逐格＋混合批控制＋旧回放 |

## 7. 环境注记（按「开发机环境」惯例登记，不入计数）

- **`PROTOC` 门**：`cargo clippy -p orz-loop`（及 `--all-targets`）会触 `orz-tools-api` 构建脚本，
  本机需 `PROTOC=D:\tb-eval\.tools\protoc-25.3\bin\protoc.exe`；未设时报
  「`protoc` not found; likely it is missing in docker image」并 101 退出——**环境面**（构建脚本依赖），
  非本批引入的代码缺陷；设 `PROTOC` 后 clippy 正常（净零新增）。`cargo check`/`cargo test` 因缓存未触发该脚本，不受影响。
- 与 `ORZ-DEV-LINKER-CRASH-001` 同族：均为「取证前先核环境面」的先例（本批读数全程 `-j 1`，无 `rust-lld` 崩溃复现）。

## 8. 边界与留待

- **本批未动**：批阈值 5／10、FP-2、`ORZ_WEB_SEARCH_LOCAL` 开关、官方口径（`task.toml`／镜像／verifier／数据集 pin）、
  0am 影批工作树（hunk 级分离保持）、`orz` pin（`3a1e34c5`）、`orz_source_manifest.sha256`（清单对**已提交** blob 计算，
  未提交批无需重算；门禁实测仅剩脏树一条）。
- **留待**：0ay **S4 真机**（同三题「无 URL 占比／可引用来源数」可机械读并入审计，承接 0ax S4）；
  0ay 闭合需 S4 读数后由用户裁决（本批不闭合、计数不变）。
- **未提交、不推送、不重建**：按用户既有口径，提交／推送／载体重建待明示。

## 9. 文件清单

| 仓 | 文件 | 变更 |
|---|---|---|
| 父仓 | `runtime/retrieval-result-event-payload-v0.2.schema.json` | ＋`citation_url_count`（可选，minimum 1）＋描述 |
| 父仓 | `runtime/fixtures/run-event-v0.2/payloads/retrieval-result.merged-multi-query.valid.json` | 生成器重跑（F-3 勘误＋有池正例） |
| 父仓 | `scripts/generate_run_event_fixtures.py` | 同源 fixture 定义更新 |
| 父仓 | `assurance/run_event_journal_validation.py` | ＋重算函数＋判官四条判定＋对拍（代际门） |
| 父仓 | `runtime/tests/test_run_event_journal_validation.py` | ＋0ay 六钉 |
| 父仓 | `docs/BACKLOG_AND_PRIORITIES.md`／`TODO.md`／`CLI_PROJECT_INDEX.md` | 0ay 进度与勘误指针（计数不变） |
| 父仓 | `docs/audits/0AT_0AU_0AV_0AW_0AX_S1_REVIEW_FIXES_2026-09-20.md` | §3 末行补文件标注勘误指针 |
| 父仓（仓外件） | `D:\tb-eval\_0ay_s3_recompute.py` | S3 复算驱动（导入仓内镜像） |
| orz 子仓 | `crates/orz-loop/src/retrieval/batch_close.rs` | ＋`citation_url_count` helper；`is_synthetic_answer` 改单源 |
| orz 子仓 | `crates/orz-loop/src/retrieval/evidence.rs` | ledger 装配落字段＋两处测试（恒等式／混合批） |
| orz 子仓 | `crates/orz-assurance/src/journal/families_s2c.rs` | ＋重算函数＋判官规则 |
| orz 子仓 | `crates/orz-assurance/src/journal/families.rs` | ＋两场景（ok／mismatch）＋场景计数 253 |

---

> 落款：0ay S1/S2/S3 实施（2026-09-20，主会话）。判据与读数以本文为登记面；S4 真机读数另行登记。
