# 任务 D S3/S4 Python 退役边界与归档范围深挖（2026-09-06）

> **性质**：用户指派深挖轮——「Python 侧退役边界与 S4 归档范围的切分还得
> 再详细看看」。只读盘点 + 边界方案登记，**不实施**；S3 开工仍待用户放行。
> **本批同时登记两项用户裁决（2026-09-06）**：
> ① S3 门禁改接形态 = **独立 CLI**（orz-bin 薄封装 Rust 法官，非 cargo
> test 复用）；② 门禁期刊校验 = **v0.1 纳入**（深挖证实 v0.1 六期刊本已
> 在门禁校验内，本裁决实义 = Rust CLI 承接后双轨 18 期刊校验面不减）。
>
> **2026-09-06 复审处理批勘误/状态更新**（[复审处理审计](TASK_D_S3S4_REVIEW_HANDLING_2026-09-06.md)）：
> ① 本文为 `1e5e086` 时点快照——其后 D-1/D-2/D-3 已由用户裁为**方案 α**
> 并随 S3 放行实施（父仓库 `20fd762` / orz `5053bc7f`），§4 状态表「待
> 裁决/待放行」各行以此为准；② 本文全部行号为翻转前快照，S3 翻转后
> `run_event_journal_validation.py` 3,627 → 3,464 行（两 dict 定义
> `:49`/`:95` 已不存在，由 `_derive_payload_registry_views()` 派生取代）、
> `validate_journal_text :3551→:3388`、`validate_journal_file :3621→:3458`、
> 机械族区间 `:457-3526→:294-3454`、`PRODUCER_SCHEMAS :272→:109`；
> `check_repository.py` v0.1 循环 `:2364→:2422`、v0.2 循环 `:2404→:2462`、
> registry 同步段 `:2420-2454→:2490` 区段、v0.1 夹具实例校验
> `:1948-2010→:2005-2068`；orz 侧 `conformance.rs`/`tool_probe.rs` 引用
> 未漂移；③ 本文与 batch-1 审计中的「契约 §8 变更流程」应为 **§9**
> （§8 = Rust 镜像同步纪律），就地更正。
> **关联**：[S2d 收口审计](TASK_D_S2D_CLOSURE_2026-09-06.md)（本批对其 §2
> 作一处数字勘误）/ [batch-1 治理审计](P0_GOV_TASK_D_DUAL_IMPL_GOVERNANCE_2026-09-04.md)。

## 1. 模块全息盘点（`assurance/run_event_journal_validation.py`，3,627 行）

退役对象不是一个函数，而是同一模块上叠着的**四层职责**，归宿各不相同：

| 层 | 内容（行号） | 职责 | S3/S4 归宿（推荐） |
|---|---|---|---|
| **A 注册表** | `PAYLOAD_SCHEMA_BY_EVENT_TYPE`（v01=34，`:49`）/ `_V02`（v02=27，`:95`） | event_type→payload schema 映射（当前权威源） | **S3 翻转**：registry JSON 升唯一权威，dict 改为模块加载时从 JSON 派生的兼容视图（下游零改动）；导出脚本（`scripts/export_run_event_payload_registry.py`）随权威方向反转而失效，S4 退役登记 |
| **B schema 级法官** | `validate_journal_file/text`（`:3621`/`:3551`）+ `_load_events` / `_envelope_errors` / `_payload_errors` / `_resolve_payload_schema` / `_verify_chain` / 链哈希 helpers / schema 缓存 | envelope + 按轨 payload + raw-JSON 哈希链（Rust `conformance.rs` 已镜像并对拍 0 差） | **S3 摘除执法权**：门禁 18 期刊校验改接 Rust CLI，模块不再被门禁调用；**S4 转冻结 reference**（保留可导入，见 §3 D-1） |
| **C 机械规则族** | 30 × `_verify_v02_*` + 常量群（`_WORK_TOOLS` / `_ACAF_TICKETED_TOOLS` / `_RETRIEVAL_MODE_GATED_TOOLS` / `_JUDGMENT_WORD_TOKENS` 等，`:457-3526`） | 跨层机械不变量（Rust families.rs/families_s2c.rs 已镜像，spec 表 6990 格 + 对拍 7530 格 0 差） | 随 B 同步：执法权归 Rust 判官；**S4 随模块转冻结 reference**（对拍对照面仍需其活着，见 §3 D-1） |
| **D reference 轨受理** | `PRODUCER_SCHEMAS`（`:272`，~20 producer：orientation 后缀形 / canonical-cli 去后缀形 / cli-session-lifecycle / envelope-only 两生产者）+ `GSA_PREFLIGHT_FRAGMENT`（`:310`） | run-event 双轨之外 payload_schema 串的解析受理——**Python 独有能力** | **保留在 Python 侧、不入退役面**：Rust 法官对非双轨 payload_schema 直接报 unknown（`conformance.rs:522-541` fail-closed）。门禁两轨期刊均在 run-event 轨，改接不受影响；batch-1 审计 §2 已裁「reference 轨自校验、不入 Rust 法官范围」，本批确认维持 |

## 2. 消费点全图（代码级，全量核过）

| # | 消费者 | 用什么 | S3 翻转后动作 | S4 动作 |
|---|---|---|---|---|
| 1 | `scripts/check_repository.py:2364`（v0.1 六期刊）+ `:2404`（v0.2 十二期刊） | `validate_journal_file` 期刊校验 | **改接 Rust CLI**（18 期刊双轨全量，用户裁决②） | — |
| 2 | `scripts/check_repository.py:1955/:2082/:2434` | 两个 dict（payload 正负夹具映射 + registry 同步比对） | registry 转正后 dict 变 JSON 派生视图，**零改动继续工作**；同步比对语义自动反转为「视图↔JSON 守卫」 | 登记口径 |
| 3 | `scripts/export_run_event_payload_registry.py` | dict → JSON 导出 | 权威反转后失去意义 | **退役登记**（视图派生已内建于模块） |
| 4 | `runtime/tests/test_run_event_journal_validation.py`（**258 tests**） | 判官全模块（B+C 层逐函数负/正测） | 不动（测的是 Python 侧自身） | 随模块转 reference **保留**（见 §3 D-2），健康度证据 |
| 5 | `runtime/tests/test_run_event_conformance.py`（**15 tests**） | 两个 dict（fixture 命名/schema 一致性守卫） | dict 变视图，零改动 | 不动 |
| 6 | `assurance/probe_accuracy_audit.py` + `assurance/tests/test_probe_accuracy_audit.py`（**8 tests**） | `_WORK_TOOLS` 常量导入（注释明言 single source of truth） | 不动 | 随模块保留导入（见 §3 D-3）；若未来模块物理删除则常量须先搬迁 |
| 7 | orz `families.rs` / `families_s2c.rs` crosscheck（subprocess 整模块喂 30 族） | **整模块作为 parity 对照面** | 不动（S3 期间对拍继续全绿） | **取决于 §3 D-1 裁决**——这是退役边界最重的一刀 |
| 8 | orz `conformance.rs:5` / `tool_probe.rs:38` | 文档注释提及 | 不动 | 措辞随退役标注同步 |
| 9 | `scripts/generate_run_event_fixtures.py` | 仅注释级提及（fixture 与判官语义对齐的约定出处） | 不动 | 注释措辞随退役标注同步 |
| 10 | 契约文档 `architecture/PYTHON_REFERENCE_SPEC_CONTRACT_v0.1.md`（§3 判官地位 / §7 reference 轨 / §9 变更流程） | 治理表述 | §9 变更流程先行登记（或随 S4 一并） | **S4 必改**：schema 演进第一执法 = registry JSON + Rust 法官，Python reference 同步降为第二动作 |
| 11 | `runtime/run-event-payload-registry-v0.1.json` `source` 字段 | 自证「源 = Python dict」 | S3 步 1 改自指（JSON 升权威） | — |
| 12 | 索引 `IMPL-PYTHON-REFERENCE` / BACKLOG 00 / 历史审计文档引用 | 召回路由 | 不动 | S4 收口时更新状态表述 |

## 3. 退役边界的三个真裁决点（本批深挖结论）

「退役」在本项目语境 = **摘除执法权 + 状态转 `reference` 冻结**，不是物
理删除。理由是模块上有两处**离开 Python 侧即失去的能力/证据**，物理删除
前必须显式裁决其归宿：

### D-1（最重）：Rust↔Python 对拍对照面的归宿

S2b/S2c/S2d 建立的 parity 体系（spec 表 6990 格 + 对拍 7530 格）的结构是
**Rust 侧 verdict ↔ Python 侧 verdict 逐格相等**——Python 模块就是 parity
的另一极。两个选项：

- **方案 α（推荐）**：Python 模块整体转 `reference` 冻结保留，crosscheck
  继续作为长期回归证据。Rust 法官成为唯一执法者，Python 侧降为「语义镜
  像参照物」——未来任何 schema/规则演进，两侧同步改动仍被对拍钉住，漂移
  无法静默。成本：3,627 行 Python 留在仓内（不参与生产、门禁零调用）。
- **方案 β（彻底退役）**：模块与 crosscheck 一并退役归档，Rust spec 表
  + fixture 测试成为唯一执法与唯一语义记录。代价：**对拍证据链终止**——
  「Rust 法官 = Python 语义的忠实镜像」这一命题失去持续核对机制，此后
  Python 侧历史语义只剩审计文档转述；且 probe_accuracy_audit 的
  `_WORK_TOOLS` 单源导入断裂（须先搬迁常量）。

深挖意见：**α**。任务 D 的目标是「解除对 Python 法官的单点执法依赖」，
不是消灭 Python 参照；β 省下的 3,627 行冻结代码换来永久失去 parity 回归，
与 S2 系列建立的证据体系自相矛盾。

### D-2：258 个 Python 判官测试的归宿

- 随模块转 reference **保留（推荐）**：0.68s 跑完、零维护负担，是 Python
  参照物自身健康度与对拍对照面有效性的直接证据；
- 随 β 才需要退役归档。

### D-3：`_WORK_TOOLS` 评测面单源导入

`probe_accuracy_audit.py`（8 tests）按「single source of truth」注释从判
官模块导入 `_WORK_TOOLS`。方案 α 下零改动；若采 β，须先把常量搬迁到独立
常量归宿（或直接读 Rust 侧 `WORK_TOOLS`——但评测面是 Python，会引入新的
跨语言单源问题）。方案 α 下此项自动消解。

## 4. 已登记的用户裁决（2026-09-06）与既定序列的合并视图

| 裁决点 | 状态 |
|---|---|
| S3 接线形态 = 独立 CLI（orz-bin 薄封装 `validate_journal_file(journal_path, repo_root)`） | **用户已裁**（2026-09-06） |
| 门禁期刊校验含 v0.1（Rust CLI 承接双轨 18 期刊，校验面不减） | **用户已裁**（2026-09-06）；深挖证实 v0.1 六期刊本已在门禁（S2d 收口审计 §2.1-4 勘误） |
| D-1 parity 对照面归宿（α 冻结 reference 推荐 / β 彻底退役） | **用户已裁 = α**（2026-09-06，随 S3/S4 翻转批落地） |
| D-2 258 测试面归宿 | **用户已裁**（随 α：保留） |
| D-3 `_WORK_TOOLS` 单源 | **用户已裁**（随 α：导入自动消解，零改动） |
| S3 开工放行 | **用户已放行并实施**（2026-09-06，父仓库 `20fd762` / orz `5053bc7f`） |

S3 实施序列更新（合并用户裁决①②，取代 S2d 收口审计 §2.4 步 2 的形态
备选）：步 1 registry 转正不变 → 步 2 = orz-bin 增 `journal-conformance`
独立 CLI + 门禁 18 期刊循环（`:2364` + `:2404`）改接（双轨一次承接，负测
= 篡改 fixture 副本验证门禁报错后还原）→ 步 3 Python 门禁调用摘除 +
`validate_journal_file` 退役标注。S4 = D-1/D-2/D-3 落地 + 契约 §9 改写 +
导出脚本退役登记 + 索引/BACKLOG 收口。（**2026-09-06 复审处理批注**：本
序列已按上文勘误注完整执行——步 1–3 与 S4 见
[翻转实施审计](TASK_D_S3_S4_FLIP_IMPL_AUDIT_2026-09-06.md)；契约「§8
变更流程」系节号笔误，实为 §9，已就地更正。）

## 5. 本批变更

- 新增本文档；S2d 收口审计 §2.1-4 数字勘误（门禁期刊校验 12 → 18 双轨
  全量）。**零代码改动、零门禁改动**；S3 实施待放行。
