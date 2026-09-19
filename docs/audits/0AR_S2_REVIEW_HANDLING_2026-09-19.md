# 0ar 过夜批审查处置回执（2026-09-19）

> 状态：**处置件全部落于工作树（未提交、未推送、未重建）**；日期 2026-09-19。
> 指令口径（用户）：「请对审查出的全部问题进行处理，处理完成后依旧暂时不提交、推送、重建」。
> 依据：[`审查报告`](0AR_S2_RS_MECHANICAL_0M_S4_OVERNIGHT_REVIEW_2026-09-19.md)（有条件通过；P1×2／P2×2／P3×5）。
> 关联：[设计稿 v1.1](../RETRIEVAL_BATCH_HANDOFF_AND_ROUND_SEAT_DESIGN_2026-09-19.md) / [ADR-0010 §14.73](../../adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [过夜批报告](0AR_S2_RS_MECHANICAL_0M_S4_OVERNIGHT_2026-09-19.md) / [BACKLOG 0ar](../BACKLOG_AND_PRIORITIES.md) / [TODO P0-0ar](../../TODO.md) / 索引 v3.80。

---

## 1. 一句话结论

审查报告全部发现**同日处置闭合**：P1-a/P1-b 采**改文路线**（设计稿 v1.0 → **v1.1** 勘误＋ADR §14.73＋S1 schema description＋TODO/BACKLOG 口径行四方对齐，**零行为变更**——勘误依据坐实 relevance 限定词在机械计数点不可得且 ledger 层恒 direct 为空操作、逐 query 收尾语义与护栏 10 数学不相容）；P2-a 生成器 README 模板修复并重生成；P2-b 去噪两项登记并入 S3 前处置范围；P3 五条逐条处置（README 333／溢出文案澄清／载体口径对齐／指针轻校验不立项／归因粒度本已登记）。**门禁执法一次**：处置中途链接检查器如实拦截两处指向本回执的断链（ORZ-GATE-ASYM-001 活例证），回执落盘后复跑全绿。读数（处置后）：orz-loop **805/0/3**、runtime **366 OK**、门禁 `error_count=1`（唯一＝orz 脏树预期态）、生成器重跑件数稳定、compileall exit 0、两仓 `git diff --check` 干净。

## 2. 处置逐项（发现 → 处置 → 落点）

### P1-a｜「阈值逐 query 计数」未实现 → **改文：批级合计＋逐 query 可核披露**

- **裁决理由**：①逐 query 收尾语义与用户定案护栏 10 **数学不相容**——逐 query 5×3=15 > 10，合并激活将恒触护栏、阈值形同虚设，照字面实现反而摧毁「二取一余地」；②覆盖保障可由「逐 query 披露（判据 6 本就只要求可核）＋主代理续派」承担——与设计根本哲学（裁决权归主代理）一致；③改码需重新裁决护栏参数（逐 query 阈值＋逐 query 护栏），无真机读数支撑，**留数据驱动路径**：S3 若显示合并批覆盖受损（query B 长期 0），凭读数再提逐 query 参数组另行裁决。
- **落点（四方对齐）**：设计稿 §5.4（v1.1 勘误段）；ADR-0010 §14.73(4)（同句改写并注明勘误）；TODO `P0-0ar` 判据口径行（「收尾阈值按批级合计、逐 query 计数为可核披露」）；代码文案 `dispatch.rs` 合并契约行「（阈值逐 query 计数）」→「（收尾判定按批级可用计数）」。

### P1-b｜宽口径缺 `relevance = direct` → **改文：口径收窄为 visibility**

- **裁决理由（勘误依据坐实）**：①机械计数点（工具证据 `EvidenceRecord`）**无 relevance 字段**，过滤不可实施；②本文一手读数的引用数据源（机械 ledger）relevance **恒硬编码 `direct`**（`evidence.rs` 三处构造点）——该过滤在设计的引用事实上**为空操作**，§3.3 七批次读数（12／10／21／11／21／14／13）即 visibility 口径读数 ⇒ 改文＝与实现及读数**零行为差异**的纯文本对齐；③tangential 语义由 ledger 披露与 prefilter 候选池承载，不入机械计数。
- **落点（四方对齐）**：设计稿 §3.3（v1.1 勘误段＋同批改警示）；S1 契约 `information-sufficiency-assessment-event-payload-v0.2.schema.json` 顶层与 `usable_source_count` 两处 description（约束零改动）；TODO `P0-0ar`／BACKLOG 0ar 计数口径行；生成器 README 模板（随 P2-a）。

### P2-a｜生成器 README 模板文本损坏 → **修复＋重生成**

- `scripts/generate_run_event_fixtures.py`：恢复被 S2 批插入吞掉的「Assessment side: optional `usable_source_count` …」句头（收窄后口径），0ar S2-D3 条目（design v1.0 → **v1.1** 引用）移至其后成完整列表项；重生成后 `runtime/fixtures/run-event-v0.2/README.md` 句读完整（第 149–156 行实态核证），件数稳定（payloads 68＋envelope 48＋canonical_cli 7＋v0.2 payloads 123＋v0.2 envelope 81）。

### P2-b｜设计 §10-2 去噪三项未实现且未登记 → **登记并入 S3 前处置范围**

- 设计稿 §10-2 注记（v1.1）：批间契约不重置由既有 M4 continue 语义天然满足；**空批不唤醒**与**重复 query 回踩**未随 S2 实装，登记并入 S3 前处置范围。BACKLOG 0ar 处置行与 TODO S3 行同步挂指针。**不新立项、不动计数**。

### P3 五条

| 条 | 处置 | 落点 |
|---|---|---|
| P3-1 README 串行读数 332 落地即过时 | **已修** | orz README Development 节 332 → **333**（RS-07 后实态） |
| P3-2 倒数行载体 journal→会话侧车 可追溯性差异 | **改文对齐** | 设计稿 §3.6（v1.1）：载体明示「模型面消息——会话侧车承载、journal `ToolCompleted` payload 不含该行」 |
| P3-3 提前交付指针只验非空 | **不立项**（fail-open 哲学＋`early_delivery_missing_pointer`/`early_delivery_streak` anomaly 已覆盖滥用面）；登记于 BACKLOG 0ar 处置行 | — |
| P3-4 多 query 归因粒度（tool_used/result_count/派生证据不归因） | **无需动作**（schema description 与 `per_query_usable_counts` 注释已如实登记为边界） | — |
| P3-5 溢出文案「已派发 1 次」歧义 | **已修** | `agent_loop.rs` 模板改「本轮已派发 **1 个检索激活**」；设计稿 §5.5 模板同步 |

## 3. 处置后验证读数（主会话独立复跑）

| 项 | 命令 | 读数 |
|---|---|---|
| orz-loop lib | `cargo test -p orz-loop --lib`（PROTOC 已设） | **805 / 0 / 3**（与处置前持平——两处代码改动均为字面文案，阈值/合并/溢出/提前交付/超时五集成测试全绿） |
| runtime 全套 | `python -m unittest discover -s runtime/tests` | **Ran 366 tests — OK**（schema description 改动后全绿） |
| 门禁 | `python scripts/check_repository.py` | 处置中途 **error_count=3**（两处＝指向本回执的断链——链接检查器如实执法；回执落盘复跑后）`error_count=1`，唯一＝`orz submodule working tree is dirty`（预期态） |
| 生成器 | `python scripts/generate_run_event_fixtures.py` | exit 0、件数稳定、README 句读完整；`git status` 无新增改动面 |
| schema 语法 | jsonschema `check_schema`（assessment） | valid |
| 编译 | `python -m compileall -q scripts assurance` | exit 0 |
| 空白 | `git diff --check`（父仓＋orz） | 两仓干净 |

未重跑项与口径：orz-host 串行 333/0/5／orz-assurance 246/0／orz-tui 178/0／pytest 收集 2080 系本日处置前同树读数——处置改动面（orz-loop 两处字面文案＋orz README 一处数字＋父仓文档/schema description/生成器模板）不触及该四 suite 的任何被测行为，读数沿用并如实注明；Python assurance 全套未跑（同 S1/S2 口径：改动面零涉、`test_doctor_full_repository_check` 在未提交批下必红）。

## 4. 边界与未做项

- **不提交、不推送、不重建**（用户硬约束；全部处置件留工作树）。
- **P1-a 数据驱动路径**：逐 query 参数组（逐 query 阈值＋逐 query 护栏）不预先实施——无真机读数支撑；S3 收取后凭读数提请裁决（设计稿 §5.4 勘误段已登记该路径）。
- **P2-b 去噪两项**：登记并入 S3 前处置范围，实施随 S3 前批次（未随本批实施——本批只闭合「未登记」状态）。
- 账本计数**不变 36**（处置不立项不动计数）；索引 v3.80 头行随批；审查报告补处置关联行。

## 5. 改动面清单

- **orz**（3 件）：`crates/orz-loop/src/retrieval/dispatch.rs`（合并契约行文案）、`crates/orz-loop/src/agent_loop.rs`（溢出模板文案）、`README.md`（333）。
- **父仓**（9 件）：`docs/RETRIEVAL_BATCH_HANDOFF_AND_ROUND_SEAT_DESIGN_2026-09-19.md`（v1.1：§3.3/§3.6/§5.4/§5.5/§10-2/§13＋头部）、`adr/ADR-0010-fusion-runtime-and-agent-architecture.md`（§14.73(4)）、`runtime/information-sufficiency-assessment-event-payload-v0.2.schema.json`（description 两处）、`scripts/generate_run_event_fixtures.py`（README 模板修复）、`runtime/fixtures/run-event-v0.2/README.md`（重生成）、`docs/BACKLOG_AND_PRIORITIES.md`（0ar 节头 v1.1＋口径行＋处置行）、`TODO.md`（口径行＋S3 行）、`CLI_PROJECT_INDEX.md`（v3.80 头行）、本回执＋审查报告关联行。

## 6. 复现

```
cd orz && export PROTOC="$PWD/bin/protoc.exe"
cargo test -p orz-loop --lib                          # 805/0/3
python -m unittest discover -s runtime/tests          # 366 OK
python scripts\generate_run_event_fixtures.py         # 件数稳定、README 句读完整
python scripts\check_repository.py                    # error_count=1（唯一＝orz 脏树，预期态）
```

---

## 7. 复核批补遗（主会话，2026-09-19）

> 指令口径（用户）：「请处理掉『RS-10 有残留杂物：8 个日志已从索引删除（暂存 `D`），但文件仍在磁盘且没进 .gitignore，所以它们以 `??` 形态挂在状态里』这点吧，补忽略规则」「0m 闭合即可」。
> §1–§6 为过夜批处置原文，本节只追加复核批的收口与确认，**不改动上文结论**；全程仍未提交、未推送、未重建。

### 7.1 RS-10 残件收口（`??` × 8 归零）

- **问题实态**（审查报告 §7 RS-10 行如实登记）：8 件日志 `git rm --cached` 摘除跟踪后，**文件按 LIFECYCLE 纪律保留磁盘**，但当时**无忽略规则** ⇒ 8 件以 `??` 形挂在状态里（`_linux_arm_dryrun/build/` 2 件、`_windows_high_nist/` 2 件、`存档/root-artifacts-2026-09-12/` 4 件）。风险面＝提交批若用 `git add -A` 类操作会把它们重新扫回索引，抵销 RS-10 本身。
- **处置**：`.gitignore` 补三式作用域规则＋脚注（依审查报告 §RS-10 口径「日志不入库应为默认」）：`/_linux_arm_dryrun/build/*.log`、`/_windows_high_nist/*.log`、`/存档/root-artifacts-*/**/*.log`。**只加忽略、不删文件**，本地存档保留。
- **核证**：`git check-ignore -v` 8/8 命中（三条规则逐条对上）；`git status --porcelain` 中 8 件 `??` 归零（`存档/root-artifacts-2026-09-12/` 整体不再冒头；`_linux_arm_dryrun/build/` 内 4 件已跟踪脚本不受影响）；暂存删除态（`D` × 8）保持不动、随下批提交生效；`git ls-files | grep '\.log$'` 仍归零。
- **边界**：其它目录的日志若确需入库，用 `git add -f` 显式声明（规则脚注已写明）。

### 7.2 0m 闭合确认（维持 37 → 36 已生效态）

- 用户复核确认「0m 闭合即可」——S4 判据即「索引/BACKLOG/TODO 同步＋门禁 Exit 0」，该判据在本批实态达成（三方账本同步＋门禁 `error_count=1` 唯一＝orz 脏树预期态）；S3 实机判据 2026-09-07 经 0o T1 达成（已在 BACKLOG/TODO 回填注记）。**结论：0m 闭合维持，不回调、不复议**。
- 计数现态 **37**：0m 闭合在 36 态生效、0as 立项回补至 37；BACKLOG 未闭合总数行／P0 总览行／P0 开放项行／TODO 路由行／索引头行五处锚点一致（门禁计数一致性检查通过）。

### 7.3 账本同步（复核批追加）

- **TODO 0aq 勾选态对齐 BACKLOG**：过夜批只更新了 BACKLOG 勾选态，TODO 的 RS 清单滞后。本批复核批同步 9 件为 `[x]`（RS-03／RS-08／RS-10／RS-11／RS-12／RS-13b／RS-13c／RS-14／RS-16），并对部分闭合三件（RS-04 部分／RS-05 大头／RS-07 主项）就地注明余项——**纯勾选态对齐、不动计数、不改明细归属**（明细权威仍在 BACKLOG／审查报告）。
- **索引 v3.82 头行**本批追加；审查报告 §7 RS-10 行与 §8 账本一致性行补复核批指针。

### 7.4 复核批读数

| 项 | 读数 |
|---|---|
| 门禁 | `error_count=1`，唯一＝`orz submodule working tree is dirty`（预期态；计数一致性检查通过） |
| 忽略实证 | `git check-ignore -v` 8/8 命中；`git status --porcelain` 中 `??` × 8 归零 |
| tracked 日志 | `git ls-files` 匹配 `*.log` 计数 **0** |
| 空白 | 父仓／orz `git diff --check` 均干净 |
| 计数 | **37**（本批零立项、零闭合变更） |

### 7.5 复核批改动面

- **父仓 5 件**：`.gitignore`（三式规则＋脚注）、`TODO.md`（0aq RS 勾选态对齐）、`docs/BACKLOG_AND_PRIORITIES.md`（RS-10 行残件收口）、`CLI_PROJECT_INDEX.md`（v3.82 头行）、本回执 §7＋审查报告两处指针。
- **零代码、零子仓改动**：orz 工作树本批复核批未触碰（既有 0ar S2／0am 影批脏树维持原态）。
