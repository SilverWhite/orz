# 过夜批审查修复批：0AT/0AU/0AV/0AW/0AX S1 全面审查 → 问题处置（2026-09-20）

> 状态：**修复全部落于工作树（未提交、未推送、未重建、零版本 bump、计数不动）**；日期 2026-09-20。
> 上游：[`过夜批实施报告`](0AT_0AU_0AV_0AW_0AX_S1_IMPL_OVERNIGHT_2026-09-20.md)（本批对其做 F-1 三处口径更正）
> ＋ 主会话全面审查（设计合理性／实现合理性／符合性三面）。
> 范围：审查发现的 2 项 P2（F-1 报告口径、F-2 journal 漂移）＋ 6 项 P3 观察，全部处置；
> 处置中深挖出的三类被掩蔽跨层漂移一并修复或登记（§3）。

---

## 1. 一句话结论

审查发现的问题全部处置完毕：**F-1** 过夜批报告三处 0at 回放口径更正（附实现口径复算值）；
**F-2** 两条生产者−schema 活漂移回补修复（`retrieval_enabled`／锚点 `reason`）＋ 一条生产者契约
缺陷修复（命令级失败缺 `status`，F4 契约）＋ Python/Rust 双侧探针校验器 scope 缺陷修复（裁决一致）
＋ 一条单件历史 stale-digest 登记归档；**P3-1/2/3/4/5/6** 六项观察全部落码/落文档。
读数：orz-loop lib **818/0/3**（+2 钉）、orz-host lib 串行 **320/0/5**（+1 钉）、orz-assurance **246/0**、
fmt 净、clippy 三 crate 净（xai-tty-utils 两条为 HEAD 既有）、pytest 契约面 **277/0**、
生成器重跑 diff 仅限本批、`check_repository` `error_count=1`（唯一＝子仓脏树预期态）。

## 2. F-1：0at B 面回放口径更正（报告登记面）

**问题**：过夜批报告 §4.2/§6.2/§10-3 把 S3 归档 B 面缺口 **4/6/3/6/0/0/0**（旧宽口径，与深挖档
§2.1 同尺）登记为「B 面披露字段在真实语料上的值得到验证」。但本批 0ax 已把 `usable_source_count`
收窄（剔除无 URL 合成答案），实现口径下同语料的缺口是另一组数——旧口径数字不构成新字段的
真机验证值。另 §4.2「saturating_sub 只在口径漂移时才可能触发，恒等式按构造成立」表述不成立
（跨 query 重叠同样触发，见 §4 P3-2）。

**审查独立复算（实现口径，剔除合成后，多 query 批）**：

| 批 | 旧口径 gap | 实现口径 batch | 实现口径 Σ逐query | 实现口径 gap |
|---|---:|---:|---:|---:|
| torch-00 | 4 | 1 | 0 | **1** |
| torch-01 | 6 | 4 | 0 | **4** |
| gpt2-00 | 3 | 0 | 0 | **0** |
| gpt2-01／extract-00／extract-01 | 0（单 query） | 4/2/0 | — | 0 |
| extract-02 | 6 | 5 | 0 | **5** |

**处置**：报告三处原值保留＋就地勘误标注（旧口径数字＋实现口径数字并列）；
`batch_close::unattributed_usable_count` 文档补恒等式成立条件；装配点补机械告警（P3-2）。

## 3. F-2：journal 漂移——两类活缺陷修复＋被掩蔽漂移深挖

**起因**：审查发现 S3 三 run 历史 journal 全量校验有 2/1/1 条错误（「回放零错误」仅在
mechanical-audit/资源族内成立）。修复 schema 层后，校验器的跨层核查（原设计：payload schema
有违即跳过，`validate_journal_text` 的 `if not payload_errors` 门）不再被掩蔽，暴露出三类
更深漂移。逐类定性：

| 类 | 量级（3 run） | 定性 | 处置 |
|---|---|---|---|
| `tool_availability_check.retrieval_enabled` unexpected-key | 3 | **活缺陷**：0ac S3① 起检索族探针恒带该键（`projection.rs::retrieval_family_payload`），schema 未跟上——每个 run 都复现 | **schema 回补**：v0.2 增可选 boolean（v0.1 replay-only 不动）；fixture 同步（生成器单一事实源） |
| `tool_completed.reason`（锚点拒单明细）unexpected-key | 1 | **活缺陷**：0q 起锚点核证拒单把 `ToolError.upstream`（label/file_path/expected/actual 结构化明细）落 journal `reason`，schema 未跟上 | **schema 回补**：v0.2 增可选 `reason`（按明细形状类型化，`$defs/anchor_reading`，nullable 成员＝不可读面）；anchor fixture 同步并改注 v0.2 schema（reason 属 v0.2 时代字段，v0.1 登记面留其余三件 F4 fixture） |
| 命令级失败完成缺 `status` | **35** | **活缺陷（生产者）**：F4 契约（§5.3）「failure_target 只许挂失败完成（status=error）」，通用宿主路径命令级失败（exit≠0、无信封）经漏斗盖 `failure_target` 却不带 `status`（此前仅 policy_denial 臂设置）——Python/Rust 双侧校验器逐例报错 | **生产者修复**：`tool_run.rs` 通用路径在无信封且 exit≠0 时补 `status="error"`＋`error="exit_{n}"`（与失败聚合同族码；有信封时 error=信封码不覆盖；模型面零改动） |
| 探针分区/翻转规则误吃检索族探针 | 6（每 run 2） | **活缺陷（校验器）**：`_verify_v02_tool_availability_probe`／`verify_tool_availability_probe` 把 retrieval_family 探针的 complete（family 成员名）当工作面分区判 extra；`_verify_v02_probe_accuracy`／`verify_probe_accuracy` 把它计入工作面翻转链（0ac S3① 起每 run 一条探针事件 ⇒ 每 run 两误报） | **双侧同修**：按 `probe_scope` 分流——family 探针不进工作面分区/翻转判定，改受「成员 ⊆ {browser, search_engine, web_channel}」约束；Python 与 Rust（`families_s2c.rs`）逐字镜像，裁决一致 |
| `retrieval_result_committed` result/ledger_digest 与随行内容不符 | 2（run3 同一 commit） | **历史单件**：extract-02 批（…-02-r0-51）随行 digest 与其自身四段内容不匹配（同 journal 其余 6/7 commit 及全部 pytest 捕获语料均吻合；0.6.3 构建次序无 post-digest 突变；键集/数值/字节序逐项排除未命中） | **登记归档，不再深挖**：校验器行为正确（payload 确实自相矛盾），无现行生产者缺陷证据；留证（事件号 167、SRC-001..007、时间戳 16:38–16:39、候选字节形式穷举记录）供后续考古 |

**修复后语料读数**：pytest 捕获语料 **277/0**；S3 三 run 历史 journal 剩余 **35＋2** 条——
均为**旧二进制真实违约**（35 条 status＝本批生产者修复之前的产出；2 条 stale digest 同上），
校验器如实报错是正确行为，不做历史豁免（不弱化在版契约）；新二进制起的 journal 由生产者
修复保证清洁（端到端清洁读数归 S3/真机步随批收取）。

**受影响面**：`runtime/tool-availability-check-event-payload-v0.2.schema.json`、
`runtime/tool-completed-event-payload-v0.2.schema.json`（两 schema 回补）、
`runtime/fixtures/…/tool-availability-check.retrieval-family.valid.json`、
`runtime/fixtures/…/tool-completed.failure-target-anchor.valid.json`（生成器同步重跑）、
`scripts/check_repository.py`（anchor fixture 改注 v0.2）、
`assurance/run_event_journal_validation.py`（探针双侧 scope 分流）、
`orz/crates/orz-assurance/src/journal/families_s2c.rs`（Rust 孪生同修）、
`orz/crates/orz-loop/src/host_exec/tool_run.rs`（生产者 status 修复）。

## 4. P3 观察处置（六项全落）

| # | 观察 | 处置 | 落点 |
|---|---|---|---|
| P3-1 | 0aw 软提示去重键＝路径串，非卷（设计口径「每 run 每卷一次」） | **落码**：Windows 键改卷前缀（`Component::Prefix`，`D:`／UNC），相对路径回退路径串；Unix 无 statfs 不辨挂载，路径串为键（文档声明局限）；新增 Windows 钉 `same_volume_paths_hint_once_per_run`（同卷双路径单提示、兄弟路径静默） | `resource_hint.rs::volume_hint_key` |
| P3-2 | 0at 恒等式在跨 query 重叠（同 digest 落两桶）时静默破坏 | **落码＋落文档**：`unattributed_usable_count` 文档写明成立条件与饱和边界；装配点 `Σ逐query > 批级` 时 `tracing::warn!` 机械留痕（不阻断、不改 payload）；新增钉 `unattributed_count_saturates_when_content_overlaps_two_queries` | `batch_close.rs`＋`evidence.rs` |
| P3-3 | 0au 保留位认领 query 去重槽 → 同轮重复位指针回踩指向未派发回执（假指针） | **落码**（两半）：①保留臂摘除本调用认领的槽位（仅槽位仍指向本 call_id 时）；②保留臂**上移至重复回踩臂之前且条件扩及重复位**——保留判定整批同质，重复位同得 cause 回执而非假指针。机制注记：检索车道 `ToolFilter::Retrieval` 禁嵌套派发 ⇒ 车道内预扫描恒空，假指针的真实暴露面＝主车道同轮重复（跨轮在主车道本就新开表）。新增钉 `reserved_round_holds_duplicates_with_the_reserve_receipt` | `agent_loop.rs` 派发预扫描/工具循环 |
| P3-4 | merged-multi-query fixture 的合成条目 SRC-004 缺 `origin_query_id`，与实现「多 query 批每条工具证据必带谱系（leader 兜底）」不符 | **落契约**：生成器补 `origin_query_id: "QRY-0001"`（leader 谱系）并重跑，契约示例与实现行为一致 | `generate_run_event_fixtures.py` |
| P3-5 | mechanical_audit 契约钉不校验分支触发面（新增 kind 漏同步分支时 payload 形状静默失约束） | **落码**：钉子补 allOf/0 if-const＝retrieval_batch、allOf/1 if-enum＝其余 7 kind、两分支并集恰分枚举——三层互证 | `mechanical_audit.rs` 契约钉 |
| P3-6 | 0aw 设计 §4 把 `strip_payload_bodies` 列入删除清单，但其属保留件 `write_targets` 的解析链 | **落文档**：§4 增勘误注（剥离链随 `write_targets` 保留，仅分类件删除；实现批按此执行，本行补正文字） | `HOST_RESOURCE_OS_DELEGATION_DESIGN_2026-09-20.md` §4 |

## 5. 验证读数（修复批全量复跑）

| 面 | 读数 | 备注 |
|---|---|---|
| orz-loop lib | **818/0/3** | +2：0at 重叠饱和钉、0au 保留轮重复位钉；既有 816 全绿零改动（0au 保留臂上移不改任何既有形状） |
| orz-host lib（串行） | **320/0/5** | +1：同卷双路径单提示钉（Windows） |
| orz-assurance lib | **246/0** | families_s2c 探针双修后既有全绿 |
| fmt / clippy | 全净 / 三 touched crate 净 | xai-tty-utils 两条 warning（`process_alive` dead-code、可折叠 if）为 HEAD 既有、crate 属「不动」边界，不属本批；`--all-targets` 需 protoc（环境缺件，与本批无关） |
| pytest（run-event 契约两文件） | **277/0** | 探针分流与 schema 回补不扰动捕获语料 |
| 生成器重跑 | diff 仅限本批 fixture | 单一事实源同步 |
| `check_repository` | `error_count=1`（唯一＝子仓脏树） | 两 schema 回补/fixture 改注/登记迁移全部过闸 |
| S3 历史 journal 复放 | 15/13/9（全为旧二进制真实违约：35 status＋2 digest） | 修复前 17/15/11（含 6 条校验器误报）；误报清零，其余为如实报错，处置见 §3 |

## 6. 边界与留待

- **零提交、零推送、零重建、零 bump、计数不变**（未闭合 40）；过夜批工作树 hunk 分离面（0am RLI）未触碰。
- **留 S3/真机（随批序）**：①生产者 status 修复与探针分流后的真机 journal 清洁读数；②0at S3 口径
  同步时转录 §4.2 勘误（归因修好前「逐 query 披露不作覆盖保障依据」降级表述不变）。
- **登记归档（不再深挖）**：F-2c stale digest 单件（§3 末行留证）。
- **不动**：`xai-tty-utils`（含其两条 HEAD 既有 warning）、`reclaim.rs`（能力保留不接线）、0z B/C/E/F
  未触及部分、官方环境口径、FP-2、阈值 5/10、orz pin（`ac17a521`）。

## 7. 文件清单

| 仓 | 文件 | 变更 |
|---|---|---|
| 父仓 | `runtime/tool-availability-check-event-payload-v0.2.schema.json` | 增可选 `retrieval_enabled`（boolean） |
| 父仓 | `runtime/tool-completed-event-payload-v0.2.schema.json` | 增可选 `reason`（`$defs/anchor_reading` 类型化） |
| 父仓 | `runtime/fixtures/…/tool-availability-check.retrieval-family.valid.json` | +`retrieval_enabled: true`（生成器） |
| 父仓 | `runtime/fixtures/…/tool-completed.failure-target-anchor.valid.json` | +锚点拒单 `reason` 明细（生成器） |
| 父仓 | `runtime/fixtures/…/retrieval-result.merged-multi-query.valid.json` | SRC-004 补 `origin_query_id`（生成器） |
| 父仓 | `scripts/generate_run_event_fixtures.py` | 三 fixture 条目同步 |
| 父仓 | `scripts/check_repository.py` | anchor fixture 改注 v0.2 schema |
| 父仓 | `assurance/run_event_journal_validation.py` | 探针分区/翻转规则按 scope 分流 |
| 父仓 | `docs/audits/0AT_…_S1_IMPL_OVERNIGHT_2026-09-20.md` | F-1 三处勘误标注 |
| 父仓 | `docs/HOST_RESOURCE_OS_DELEGATION_DESIGN_2026-09-20.md` | §4 剥离链勘误注 |
| 父仓 | 本报告 ＋ `CLI_PROJECT_INDEX.md` v3.99 | 登记面 |
| orz | `crates/orz-loop/src/host_exec/tool_run.rs` | 命令级失败补 status/error（无信封臂） |
| orz | `crates/orz-assurance/src/journal/families_s2c.rs` | 探针分区/翻转 scope 分流（Rust 孪生） |
| orz | `crates/orz-host/src/resource_hint.rs` | 卷级去重键＋Windows 钉 |
| orz | `crates/orz-loop/src/retrieval/batch_close.rs` | 恒等式边界文档＋重叠饱和钉 |
| orz | `crates/orz-loop/src/retrieval/evidence.rs` | 跨 query 重叠机械告警 |
| orz | `crates/orz-loop/src/agent_loop.rs` | 保留臂上移＋条件扩重复位＋去重槽摘除＋钉 |
| orz | `crates/orz-loop/src/mechanical_audit.rs` | 契约钉分支触发面互证 |

---

> 落款：审查修复批（2026-09-20）；执行＝ZCode 主会话。F-1 复算值、F-2 定性与读数以本文为登记面；
> 提交/推送/重建待用户明示。
