# P0-GOV 任务 D 双实现终局治理（批次 1：Rust schema 级离线法官 + 单源映射）

> **审计日期**：2026-09-04
> **审查性质**：对 P0-GOV 任务 D（双实现终局治理）的落地批次——差异梳理、
> Rust `orz-assurance` 补齐关键校验断言、单源映射落地与证据对拍。
> **范围**：父仓库 `runtime/`（schema 注册表单源 JSON + 门禁钩子）与 orz
> 子模块 `orz-assurance`（`journal/conformance.rs` 新离线法官 + fixture
> 对拍集成测试）。
> **方法**：只读实测 + 源码穿透；验证含 `cargo test -p orz-assurance`、
> `cargo check --workspace`、`cargo fmt --all --check`、Python conformance
> pytest、`check_repository.py` 门禁。
> **关联前序**：[`GLOBAL_ARCHITECTURE_DEEP_AUDIT_2026-09-04.md`](GLOBAL_ARCHITECTURE_DEEP_AUDIT_2026-09-04.md)
> （缺陷三 / 第四阶段）/ [`P0_GOV_UNCOMMITTED_REVIEW_HANDLING_2026-09-04.md`](P0_GOV_UNCOMMITTED_REVIEW_HANDLING_2026-09-04.md)
> （§6 遗留开放）/ BACKLOG 00 任务 D。

---

## 1. 任务定义与批次边界

任务 D 原文（BACKLOG 00 / TODO P0-GOV）：

> 在 Rust `orz-assurance` 补齐关键校验断言，逐步退役 Python `assurance`
> 双法官冗余。

深层审查第四阶段给出两步方法：

1. 全面梳理 Python `assurance/run_event_journal_validation.py` 与 Rust
   `orz-assurance` 的差异；
2. 在 Rust 侧补齐缺失的断言逻辑，彻底解除对外部 Python 法官的单点依赖。

本批次（batch-1）落地其中的 **schema 级法官**（envelope + 按轨 payload +
raw-JSON 哈希链）与 **event_type→payload schema 映射单源化**；31 个机械
规则族（`_verify_v02_*`）、门禁改接 Rust 法官、Python 双法官正式退役
登记为 S2–S4 保留开放（§6）。任务 D 整体因此保持开放，不虚报闭合。

## 2. 差异梳理（第四阶段步骤 1）

Python 验证器（`assurance/run_event_journal_validation.py`，3,700 行）
的校验面与 Rust 侧现状逐层对照：

| Python 验证器层 | Rust 侧现状 | 处置 |
|---|---|---|
| `PAYLOAD_SCHEMA_BY_EVENT_TYPE(_V02)` 注册表（34+27） | 无同构映射；payload 为自由 `Value` | batch-1：导出机器可读 registry JSON（门禁强同步），Rust 与 Python 共表 |
| envelope 校验（v0.1/v0.2 由 `payload_schema` 选轨；required/const/additionalProperties/pattern/类型） | typed `RunEvent` 解析覆盖 required/enum，但不覆盖 additionalProperties、schema_version const、pattern | batch-1：Rust conformance 用同一 schema 文件 + jsonschema（format off = Python no-op） |
| 轨同质性（v0.2 journal 全体 `schema_version`/`payload_schema` 一致） | 生产 `EventTrack` 单轨；回放无整刊校验 | batch-1：conformance 强制整刊单轨 |
| payload 按轨 schema 校验（34+27 文件） | 无（payload 自由 `Value`） | batch-1：registry 解析 + jsonschema 校验 |
| `_verify_chain`（raw JSON 重算链） | `chain.rs`/`replay_journal`（typed 重算） | batch-1：conformance 增加 raw-JSON 链（原因见 §3.2） |
| 31 个 `_verify_v02_*` 机械规则族 | 运行时各状态机执行 + 部分 Rust 测试 | S2：逐族证据化或 Rust 显式实现（保留开放） |
| `PRODUCER_SCHEMAS`（reference 轨：canonical-cli/orientation/normalizer/gsa） | Rust 不产出 | 不入 Rust 法官范围（reference 轨自校验，见契约 §7） |

## 3. batch-1 落地

### 3.1 单源映射：`runtime/run-event-payload-registry-v0.1.json`

- 导出脚本 [`scripts/export_run_event_payload_registry.py`](../../scripts/export_run_event_payload_registry.py)
  从 Python 注册表 dict 生成 JSON（v01=34 / v02=27，repo-root 相对路径）。
- `check_repository.py` 新增同构比对：JSON `tracks.v01/v02` 与
  `PAYLOAD_SCHEMA_BY_EVENT_TYPE(_V02)` 不一致即门禁报错（rerun 提示），
  并把 registry + 导出脚本列入 required 文件。
- 权威方向：本批 Python dict 仍是权威源，JSON 是其机器可读投影、门禁强同步；
  Python 法官退役时（S4）JSON 转正为唯一表。

### 3.2 Rust 离线法官：`orz-assurance/src/journal/conformance.rs`

新增 `validate_journal_file(journal_path, repo_root)`：

1. 逐行 JSON 解析（parse/blank 错误先返回——与 Python gating 一致）；
2. envelope 校验：按 `schema_version`/`payload_schema` 轨对选 v0.1/v0.2
   envelope schema，jsonschema draft 2020-12 校验（format 关闭 = Python
   `FormatChecker` date-time no-op 对齐），并强制整刊单轨同质；
3. payload 校验：registry 解析（v0.2 有独立 payload 轨先取 v0.2 表、否则
   回落 v0.1 表——镜像 Python `_resolve_payload_schema`）→ jsonschema；
4. **raw-JSON 哈希链重算**：seq 稠密/run_id/manifest 恒定/前链/载荷与事件
   digest/唯一终态在末尾。

#### 3.2.1 实测漂移面：`runtime_stagnation_guard` 的历史 v0.1 回放

对拍测试首发发现：v0.1 fixture `plain-run.jsonl` 含 `runtime_stagnation_guard`
事件（2026-08-06 真实捕获），而 Rust typed `EventType` 于 2026-08-22 退役
该类型后无法解析——typed `replay_journal` 对历史 v0.1 全库不可回放。这是
「schema 权威（v0.1 冻结面仍登记 34 事件）与 typed 生产模型（已删变体）」
漂移的实证。处置：**离线 conformance 的链校验按 raw JSON 重算**（与 Python
`_verify_chain` 一致），不依赖 typed 枚举；typed 生产回放保持严格不动。
该漂移面登记为 Task D 已消解的一项具体差异（后续若需 typed 回放历史 v0.1
全库，另立条目恢复 `RuntimeStagnationGuard` 回放变体）。

### 3.3 对拍证据测试：`orz-assurance/tests/fixture_journal_conformance.rs`

- 正向：18 个真实 Rust 捕获 fixture（v0.1×6 + v0.2×12）全部通过 Rust
  schema 级法官（与 Python 基线 0 messages 对齐）；
- 负向 6 类篡改均被拒：顶层多余字段、schema_version 轨对错误、payload
  非 object、v0.2 轨退役事件类型、混轨不齐、链 digest 篡改。

## 4. 变更清单

父仓库：

- `runtime/run-event-payload-registry-v0.1.json`（新增，34+27 条目）
- `scripts/export_run_event_payload_registry.py`（新增）
- `scripts/check_repository.py`（registry 同步门禁 + required 文件）

orz 子模块：

- `crates/orz-assurance/Cargo.toml`（+`jsonschema` workspace 依赖）
- `crates/orz-assurance/src/journal/conformance.rs`（新增离线法官）
- `crates/orz-assurance/src/journal/mod.rs`（模块 + 重导出）
- `crates/orz-assurance/tests/fixture_journal_conformance.rs`（新增对拍）
- `Cargo.lock`（jsonschema 入 orz-assurance 依赖边）

## 5. 验证证据

- `cargo test -p orz-assurance`：lib 202 passed（+1 track 单测）/ 0 failed、
  fixture conformance 7 passed、v1_fake_provider 9 passed、doctest 1 passed。
- `cargo check --workspace`：Exit 0，0 错误 0 告警。
- `cargo fmt --all --check`：Exit 0。
- `cargo clippy -p orz-assurance --all-targets`：新代码零告警（既有 2 项
  lif/reducer 告警不在本批范围）。
- Python conformance pytest：273 passed（registry/门禁改动不触及验证器行为）。
- `python scripts/check_repository.py`：error_count=1，唯一错误 =「orz
  submodule working tree is dirty」（本批 orz 改动未提交所致，预期）；
  `run_event_payload_registry` 计数=1、无 registry 漂移错误、schemas 246。
- orz 工作树提交后需重算 `orz_source_manifest.sha256`（新增 2 个源文件 →
  1436）并复跑门禁 Exit 0。

## 6. 残留开放（任务 D 保持开放）

- **进度记录（2026-09-04）**：按用户指示，S2 的 31 族盘点与排期暂缓；当前
  进度记录止于 batch-1 闭合，不新增 S2 明细。

- **S2 机械规则族**：将 `run_event_journal_validation.py` 的 31 个
  `_verify_v02_*` 族逐族盘点为「Rust 运行时已强制（附测试证据）/需 Rust
  显式实现」两档；先落地与证据面直接相关的核心族（控制票配对、生命周期、
  检索模式、ledger fold、policy denial/failure target）。
- **S3 门禁改接**：`check_repository.py` 对真实 fixture journal 的校验改由
  Rust 法官执行（cargo test 或独立 CLI，--repo-root 参数化已备），Python
  `validate_journal_file` 对 Rust 轨退役；registry dict 源翻转 JSON 转正。
- **S4 收口**：`run_event_journal_validation.py` 归档/退役登记、契约文档
  （`architecture/PYTHON_REFERENCE_SPEC_CONTRACT_v0.1.md` §8 变更流程）同步、
  BACKLOG/TODO/索引状态收口与全量回归。

## 7. 状态

任务 D：`open`（batch-1 已闭合，S2–S4 保留开放）；本批证据见 §5。
