# TER T0.3 ADR 候选登记（2026-09-03）

> 上级：TODO2 T0.3（M0 设计门）；设计稿
> `docs/TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md`；开放项
> `docs/BACKLOG2.md` TER-0。
> 状态：T0.3 完成（TODO2.md 已勾选）；T0.4 放行签名待续（M1 前最后一道门）。

## 1. 交付物（本次落盘）

- ADR-0010 追加 **§14.53 v1.53 补写裁决索引**，候选项 3 条：① TER
  工具执行层改革总候选登记；② 取代/衔接候选——S5-2 常驻化；③ 衔接
  候选——PUSH→PULL 的 push 例外。头部「冻结版本」链同步登记 v1.53。
- CLI_PROJECT_INDEX **v2.47 → v2.48**：§1 新增 canonical entry
  AUTH-TOOL-EXECUTION-REFORM（`pending`），§8 状态速查 `pending`
  列表同步，头部「最近整理」登记 v2.48。
- 设计稿状态行更新（「设计讨论稿（未实施、未入 ADR）」→「设计稿
  （未实施；已登记 ADR-0010 §14.53 候选项）」）；PUSH→PULL 锚点
  修正（§4/§8：原「ADR §14.40 第 10 项」系笔误 → §14.35
  第 10–12 项）。
- TODO2 T0.3 勾选 + 完成注记（含本审计链接）。

## 2. ADR 候选项要点

登记语义：候选项**不即时取代既有设计语义**；候选状态维持至 M1–M3
实施批次放行，各批次登记正式裁决时再逐项转正。

- 条目 1：TER 总登记（P0；2026-09-03 用户逐条裁决）——常驻能力自身
  默认全开（auto-background=true / 首报后台化 180s / 模型面封闭）、
  去自身硬超时（仅留官方评测墙钟；300s 输出/CPU 活跃兜底 idle-kill）、
  黑板 processes live 视图、轮预算撤默认 120 硬限（保留可配逃生阀）、
  F6 三档默认 off、W-F11 env PULL / W-F12 快速确定性失败 / W-F13
  阅读面（64KB + 输出检索对象 + 截断标记）；首轮不做：15min 长档 /
  待审圈系统审计 / fold 状态摘要；T0.1/T0.2 产物作 M0 证据入条目。
- 条目 2：S5-2 常驻化——候选取代 THIN_HARNESS_REDESIGN_V2 §9.7 的
  「依赖 harness 注入开启（默认 false）+ 原解析超时满即树杀」接线
  口径（T1.1–T1.4 锚点）；`tool_running`/journal 链规则沿用，idle-kill
  形态为兜底扩展（T1.5）。
- 条目 3：PUSH→PULL push 例外——§14.35 第 10–12 项零常驻注入纪律
  保持；F6 push 档为可开关显式例外（budget_cue_injected v0.2 事件，
  T1.8 锚点）；进程/环境面仍 PULL。

## 3. 验收二「索引无冲突」与 CLI_PROJECT_INDEX §0.5 检查

| 检查项 | 结果与证据 |
|---|---|
| prior-existence scan | 索引此前无 TER / 工具执行层改革 / AUTH-TOOL-EXECUTION-REFORM 条目；主 TODO/BACKLOG 指针（TODO.md / BACKLOG_AND_PRIORITIES.md）已存在，TER 细节经 BACKLOG2 / TODO2 路由。本次新增 canonical entry 无重复概念。 |
| 稳定 ID 去重 | AUTH-TOOL-EXECUTION-REFORM 全文唯一（3 处出现 = 头部登记注记 / §1 条目 / §8 状态速查，同一 entry）。 |
| 状态合法性 | `pending` ∈ §0.2 允许状态；条目内注明「候选；M1–M3 放行后转正式裁决」边界，不冒充 current-design。 |
| 入口存在性 | ADR-0010（§14.53）、设计稿、BACKLOG2、TODO2、本审计文件均存在（Test-Path 全 True）。 |
| 旧路径残留 | 未删除/重命名任何文件；无旧路径引用残留。 |
| git diff --check | 5 个改动文件作用域内 exit 0（无空白错误）。 |

## 4. T0.2 残留核查与处理（2026-09-03，用户确认方向 A）

- **方向 A 已执行（v0.1 树）**：`tool_running` 是 v0.2-only 事件
  （2026-08-29 S5-2；v0.1 schema/夹具树均无），已从生成器 `EVENT_TYPES`
  移除并留注释；随后 v0.1 树完整重建成功，与 git 索引 **0 差异**
  （含 canonical_cli，7 文件一致）——方向 A 验收达成。
- **为让生成器跑通补齐的表缺口**：`SLUGS_V02` 补
  `budget_cue_injected` slug（v0.2-only 无回退键）；`PAYLOAD_GOOD_V02`
  /`PAYLOAD_BAD_V02` 补 `transport_retry` 定义（规范形态以已提交 payload
  样例为准）；`V02_ENVELOPE_TIMESTAMP_OVERRIDES` 补 `transport_retry`
  （保留 2026-08-21 批次日期）。生成器现可完整跑通（GEN_EXIT=0）。
- **新发现开放项：v0.2 表与夹具树全面不同步（专项对齐，建议单列
  BACKLOG/TODO）**：P2-11/P2-13 起的夹具增量多为手工维护、未同步进
  生成器表——`session_archive` 事件与 payloads、`tool_completed`
  dep-graph / failure-target extras、`retrieval_close_record` extras、
  `tool_running` / `mechanical_audit_update` envelope 时间戳与身份
  override、`retrieval_close_record.minimal` 演进、FIXTURES_README_V02
  文本均落后于已验收夹具树。全树确定性重建需一次专项对齐，本次**不**
  在 T0.4 前强推。
- **试运行副作用（已还原）**：`main()` 先 rmtree 再重建三棵夹具树；
  试运行曾清空/改写夹具文件。已用 `git checkout-index` 把
  runtime/fixtures 与 assurance/fixtures/canonical_cli 的**全部已跟踪
  文件**恢复到索引态（0 差异）；8 个 T0.2 未跟踪新增夹具保留
  （budget-cue-injected 信封按生成器表归一为规范形态）。
- **测试基线复跑**：`pytest runtime/tests/test_run_event_conformance.py
  runtime/tests/test_run_event_journal_validation.py -q` → **273 passed**
  （warning 仅 `.pytest_cache` 写权限，不影响结论）。

## 5. 状态与后续

- M0 进度：T0.1 ✓ / T0.2 ✓ / T0.3 ✓；剩余 T0.4 放行签名（需用户确认
  进入 M1）。
- T0.4 前置：S0 核对表（TER_T0_1）+ schema diff（TER_T0_2 / 273
  passed）+ ADR 候选（§14.53）；生成器 v0.2 表全面对齐（§4）作为
  独立专项建议，不阻塞 T0.4。
