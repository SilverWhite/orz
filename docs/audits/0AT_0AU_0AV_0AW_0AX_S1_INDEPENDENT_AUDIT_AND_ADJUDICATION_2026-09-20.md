# 独立审计与裁决：0AT/0AU/0AV/0AW/0AX S1 批（2026-09-20）

> 状态：**审计只读取证 ＋ 五项裁决全部落定**；本批随批即修 F-1／F-4／F-5，**F-2／F-3 立项 0ay**（计入未闭合 40 → 41），
> 环境项按「开发机环境」惯例落案例 `ORZ-DEV-LINKER-CRASH-001`（登记不入任务计数）。
> 日期 2026-09-20；**未提交、未推送、未重建、零版本 bump**。
> 指令口径（用户）：「请检查当前未提交内容」→「请先落一份审计报告出来吧，1/2/3/4/5 五项全部由你进行裁决即可，工程向的裁决我不如你，
> 只考虑最优修复即可，可直接进行登记并排期」＋「同时请按照『开发机环境』惯例登记一笔」。
> 审计基线：父仓 `HEAD=10f93c75`（索引 v3.99）＋子仓工作树（pin `ac17a521` 未动）。
> 审计对象：[`过夜批报告`](0AT_0AU_0AV_0AW_0AX_S1_IMPL_OVERNIGHT_2026-09-20.md)（五项 S1 落码）
> ＋ [`审查修复批报告`](0AT_0AU_0AV_0AW_0AX_S1_REVIEW_FIXES_2026-09-20.md)（F-1/F-2/P3 六项处置）。

---

## 1. 一句话结论

被审批的**代码面与绝大部分读数经独立复现成立**（五项落码可编译、可测、可回放，S3 语料数字逐格吻合）；
审计定出 **5 项缺陷**（1×P1 门禁红、1×P2 可核性缺口、3×P3 文档/契约面），
**F-1／F-4／F-5 本批即修**（零计数），**F-2 ＋ F-3 立项 `0ay`**（P1，40 → 41），**环境项落案例不入计数**。

## 2. 发现与裁决（F-1 … F-5）

### F-1（P1）门禁红：索引新头行超长 ＋ 常驻钉同红 —— **裁决：本批即修（缩行 ＋ 滚段入档）**

- **现象**：`python scripts/check_repository.py` → `valid: false`、`error_count=2`；第二条＝
  `ledger slimming: CLI_PROJECT_INDEX.md:1 头部台账/计数行超长（1254 > 1200 字符）`。
  常驻钉 `assurance/tests/test_ledger_consistency_nails.py::ResidentRepositoryLedgerGate::test_real_repository_header_lines_are_within_limits`
   同因变红（实测 14 passed / **1 failed**）。
- **取证**：索引 v3.99 头行 1254 字符（v3.98 1061／v3.97 1111 均在限内）；上限常量
  `LEDGER_LINE_CHAR_LIMIT = 1200`（`scripts/check_repository.py:922`），覆盖面含索引 `> 索引版本／近版摘要` 头行。
  两份报告登记的「`error_count=1`（唯一＝子仓脏树）」系 **v3.99 落笔前**的旧读数，落笔后未复核——读数与状态脱节本身也属登记纪律缺陷。
- **定性**：过程性缺陷（可机械检出、无功能影响），但**门禁红会阻塞本批任何提交**。
- **裁决**：①**收缩 v3.99 头行**至限内（明细回落到报告链接，不丢信息——报告与 BACKLOG 0aw 条目仍在）；
  ②同时把**最老一段头行（v3.40–v3.50，含尾部错位两行）滚入增量存档快照**
  [`存档/index/CLI_PROJECT_INDEX_FULL_2026-09-20.md`](../../存档/index/CLI_PROJECT_INDEX_FULL_2026-09-20.md)（沿 2026-09-15 增量件先例），
  复位头部余量，避免下一批再撞线；③本批新增头行 v4.00 自带字数约束（≤1200）。
- **归属**：本批即修；零计数。

> **F-1 附带（读数覆盖，本批随批补口径）**：未提交批状态下
> `assurance/tests/test_cli_dispatcher.py::GsaCliDispatcherTests::test_doctor_full_repository_check` **必然红**——
> `doctor` 的全仓检查内嵌本门禁，子仓脏树 ⇒ `valid=false` ⇒ 退出码 1（实测 55 passed / 1 failed，唯一失败即此件）。
> 属**不提交批的固有形态**（非缺陷，与门禁 `error_count=1` 同源），但两份报告的读数面都未列该件、也未声明
> 「契约面抽取口径」——故本批起读数口径显式写明：**契约/台账类常驻钉须单独列出，不得只报被过滤子集**。

### F-2（P2）0ax 判定面未落盘：`synthetic_answer_count` 不可由 journal 独立重算 —— **裁决：立项 0ay（P1）**

- **现象**：0ax S1 的核心判定 `is_synthetic_answer(ev) = web_search_result ∧ candidate_urls.is_empty()`
  （`orz/crates/orz-loop/src/retrieval/batch_close.rs:154`）所依赖的 **`candidate_urls` 不进 journal**：
  ledger 条目写 `source_url_or_ref = ev.identity`（`retrieval/evidence.rs:392`），而 identity 对 `web_search`
  恒取查询串（`retrieval/evidence.rs:83-88`：`path → url → query → document_id`）。
- **取证**：S3 torch journal 全文 **0 次**出现 `citation`／`candidate_url` 字样；`tool_completed`（web_search）载荷只有
  `call_id/exit_code/tool/wall_ms(/target)`。⇒ 事件里既有 `synthetic_answer_count`、收窄后的
  `usable_source_count`，但**没有可复算它们的输入面**。
- **定性**：与本项目「机械可核／单事件自描述」纪律及 **TODO P1-0ax S4 判据**（「无 URL 结果占比／可引用来源数可机械读数并进入审计」）
  直接冲突。附带勘误：过夜批报告 §6.2 的 S3 七批回放用的是**代理判据**（`source_url_or_ref` 非 URL）；
  S3 语料恰好全批引用池为空，两者数值才相等——**混合批会分叉**，该回放不构成实现口径的可核证明。
- **裁决**：**立项 `0ay`＝`GAP-RETRIEVAL-SYNTHETIC-JUDGEMENT-AUDITABILITY`**（P1，40 → 41），批序
  **S1 契约面 → S2 实现 → S3 语料回放复算 → S4 真机随官方跑批**，各步独立放行、不跳步：
  S1＝ledger 条目增机械字段（`citation_url_count`；`candidate_urls` 仅在非空时落计数，避免膨胀），
  schema／fixture／Python 镜像／Rust verifier 四件同批；S2＝装配面单源（`EvidenceRecord.candidate_urls.len()` 经同一 helper 落盘）；
  S3＝用新字段在 S3 三 run 语料上**重算** `synthetic_answer_count` 与收窄 `usable_source_count`，与实现判定逐格一致（含混合批正例）；
  S4＝真机读数入审计（承接 0ax S4）。
- **归属**：新立项 0ay（计数 40 → 41）；与 0av「读数落盘面」同族，但属**判定输入面**而非读数面，故独立成项。

### F-3（P3）契约 fixture 与机制相反 —— **裁决：并入 0ay S1（不单独立项）**

- **现象**：`runtime/fixtures/run-event-v0.2/payloads/retrieval-result.merged-multi-query.valid.json` 的 SRC-0004
  标题为「Synthesized answer (no URL)」，`source_url_or_ref` 却写成 `https://search.example/synth`。
- **定性**：生产端合成条目该字段＝**查询串**（identity 取 `query`），带 URL 的 `web_search_result` 条目**不可产出**
  ⇒ 该行既非可产出形态，又会被 §6.2 那套代理判据读反（被当成「有 URL ⇒ 非合成」）。
- **裁决**：并入 **0ay S1** 一并重写（合成条目改查询串形态 ＋ 新增 `citation_url_count` 示例；同时给出「有引用池」正例），
  使 fixture 能同时示范形态与可核性；不单独立项、不另计计数。

### F-4（P3）设计档 §4 勘误注打断表格 —— **裁决：本批即修（勘误移表后）**

- **现象**：`docs/HOST_RESOURCE_OS_DELEGATION_DESIGN_2026-09-20.md:55` 起的勘误引用块插在 §4 表格中段。
- **定性**：Markdown 表格被引用块终止 ⇒ 其后的 `tier 阶梯` 行脱离表体、按普通段落渲染（内容不错、结构坏）。
- **裁决**：勘误块移到 §4 表格**之后**，文字逐字保留（含「剥离链随 `write_targets` 保留」的定性）。
- **归属**：本批即修；零计数。

### F-5（P3）台账/报告口径残留：「回放零错误」旧表述无勘误指针 —— **裁决：本批即修（补指针）**

- **现象**：索引 v3.98 头行与过夜批报告 §2.2 判据 5 行仍写「S3 三 run 历史 journal 经保留 verifier 回放零错误」；
  修复批 F-2 已更正为「零错误**仅在 mechanical-audit／资源族内**成立；其余为旧二进制真实违约 **35＋2**」。
- **定性**：非事实错误（保留族确实零错误）但**读法歧义**——「零错误」易被读成整卷零错误，与 37 条已登记违约冲突。
- **裁决**：**在两处就地补勘误指针**（v3.98 行插入括注指向修复批 §3；报告 §2.2 判据 5 行同注），**不改写原读数**（留档纪律）。
- **归属**：本批即修；零计数。

## 3. 环境项（按「开发机环境」惯例登记）

### ORZ-DEV-LINKER-CRASH-001（`candidate`）并行 `rust-lld` 崩溃（`0xc000001d`）

- **现象**：本机 `cargo test -p orz-bin` 默认并行链接时，`rust-lld` 以 `exit code 0xc000001d`（NTSTATUS `STATUS_ILLEGAL_INSTRUCTION`）崩溃；
  崩溃残留**零长度 `.rmeta`** 与半成品 rlib，后续构建报 `crate <X> required to be available in rlib format`／`can't find crate`／`invalid metadata files for crate test`。
- **取证（本次实测）**：① 默认并行：多目标编译失败（含 `orz`／`orz-signer`／`acaf_e2e`）；② `cargo clean`（清 **35.7 GiB**）后重编**仍复现**；
  ③ 同一工作树 `<cargo test -p orz-bin -j 1>`：**全绿**（main 12/0＋12 ignored、provision 2/0、signer 15/0、acaf_e2e 23/0、real_flag 2/0、stdio_e2e 1/0）；
  ④ `cargo check --workspace --all-targets` 干净 ⇒ **非代码缺陷**，属开发机链接器并发崩溃＋target 缓存污染族。
- **纪律（能力）**：①**「测试红先核环境」**——报读数前先看是否为 lld/缓存形态（零长度产物、`rlib format`／`invalid metadata` 报错）；
  ②**重载目标串行**（`-j 1`）作为本机复现口径；③**清理面即恢复面**——`cargo clean` 是恢复手段而非证据，
  清理前后须分别记录读数（本次：清理前 1 失败形态、清理后串行全绿）。
- **同族先例**：`ORZ-DEV-TUNED-BOUND-001`（开发机调参上界与缓存掩盖）、`ORZ-ENV-POLLUTION-001`（读数前先核 env）——
  本条补「**构建/链接阶段的本机崩溃**」维；**登记不入任务计数**，不新增阻断门。
- **回归入口**：本机 orz-bin 全目标测试（默认并行 vs `-j 1` 对照）；案例文件
  [`docs/cases/harness_environment/ORZ-DEV-LINKER-CRASH-001-parallel-lld-illegal-instruction.md`](../cases/harness_environment/ORZ-DEV-LINKER-CRASH-001-parallel-lld-illegal-instruction.md)。

## 4. 独立复核读数（本次审计实取）

| 面 | 被审批登记值 | 本次独立实测 |
|---|---|---|
| orz-loop lib | 818/0/3 | ✅ 818/0/3 |
| orz-host lib（串行） | 320/0/5 | ✅ 320/0/5 |
| orz-assurance lib | 246/0 | ✅ 246/0 |
| orz-bin 全目标 | main 12/0、provision 2/0、signer 15/0、acaf_e2e 23/0、real_flag 2/0、stdio_e2e 1/0 | ✅ 同值（`-j 1`；默认并行撞 ORZ-DEV-LINKER-CRASH-001） |
| runtime 契约两文件 | 277/0 | ✅ 277/0（整目录 366/0） |
| 台账常驻钉 | （未登记） | ⛔ 14/1（F-1：头行超长）→ 本批修复后 15/0 |
| `doctor` 全仓检查钉 | （未登记） | ⛔ 55/1（`test_doctor_full_repository_check`：脏树态固有红，非缺陷；见 F-1 附带） |
| `check_repository` | error_count=1 | ⛔ error_count=2（F-1）→ 本批修复后 1（唯一＝子仓脏树） |
| workspace 全目标类型检查 | — | ✅ `cargo check --workspace --all-targets` 干净 |
| fmt / clippy / 生成器 / manifest | 净 / 净零新增 / 零漂移 / valid | ✅ 全部成立（clippy 需 `PROTOC`；新增面零 warning） |
| S3 三 run 事件计数 | 拒单 14/11/4、snapshot 3/1/1、exhausted/reclaim 0 | ✅ 逐值吻合 |
| S3 journal 校验 | 剩余 35＋2（旧二进制违约） | ✅ 15/13/9＝37；构成＝35×`failure_target requires status=error` ＋ 2×extract-02 digest |
| 0ax 七批回放 | old 5/6/5/5/5/5/6；合成 4/2/5/1/3/5/1；new 1/4/0/4/2/0/5 | ✅ 独立重算逐格一致（合成 21/37＝56.8 %）——**但属代理判据，见 F-2** |
| 0at 缺口 | 旧口径 4/6/3/6/0/0/0；实现口径 1/4/0/—/5 | ✅ 两组均复现一致 |
| 边界 | 未 bump／未推送／pin 未动／载体未动 | ✅ 确认；`D:\tb-eval\orz-windows\orz-signer.exe` sha256 仍 `AD05276F…`（063 档记录一致） |

## 5. 账本同步与计数

- **未闭合计数 40 → 41**：新增 **0ay**（P1）。F-1／F-4／F-5 为即修项、F-3 并入 0ay S1、环境案例按「登记不入任务计数」惯例 ⇒ 其余不动。
- 三方同步：BACKLOG（未闭合计数节＋优先级总览 P1 行＋P1 节「开放项：」＋`### 0ay.` 小节＋0aq 案例沉淀随注）／
  TODO（未闭合总数＋P1 路由＋`### P1-0ay` 小节）／索引（头行 v4.00＋v3.99 收缩＋v3.98 勘误指针＋§3.1 条目＋§8 `pending` 与 `reference` 桶）。
- 案例库：`docs/cases/README.md` 第六批 ＋ 同族追加⑧；索引 §8 `reference` 桶补 `ORZ-DEV-LINKER-CRASH-001`。
- 存档：`存档/index/CLI_PROJECT_INDEX_FULL_2026-09-20.md`（增量：v3.40–v3.50 头行原文）＋`存档/index/README.md` 表行。

## 6. 边界与留待

- **本批未动**：被审批的全部落码语义（F-1/F-4/F-5 只改文档/台账，F-3 只登记待 0ay S1 执行）；`orz` 子仓本次零改动；
  0am 影批仍按「hunk 级分离」挂起（`controller.rs` 同文件双批 hunk，提交时须分离）；官方口径（`task.toml`／镜像／verifier／数据集 pin）不动。
- **待用户放行**：0ay 实施（S1→S4）；0ax S2 启用 `ORZ_WEB_SEARCH_LOCAL`；0ax S3 fetch 兜底路线。
- **留待**：历史 journal 兼容仅按 S3 三 run 实测（全量复扫归既有 `0aa`）；本批提交／推送／载体重建待明示。
- **附带观察（不立项，未计数）**：索引与 BACKLOG 指向 `orz/crates/orz-loop/src/host_exec.rs` 的链接在 0ai 拆分后失效
  （文件已成目录 `host_exec/`）——纯链接卫生，建议随下次接触该条目时改指 `host_exec/mod.rs`；
  本次审计顺带发现的其余链接（存档快照内的原相对路径）按 2026-09-15 增量件先例保留原文，已在快照头部注明链接口径。

---

> 落款：独立审计（2026-09-20，主会话）；发现与裁决以本文为登记面。验证记录随 ORZ-DEV-LINKER-CRASH-001 案例档。
