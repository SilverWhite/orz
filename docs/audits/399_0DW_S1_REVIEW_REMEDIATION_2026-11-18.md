# 399 批：0dw S1 全面审查处置——P1 preflight schema 冲突＋P2 台账混尺＋P3×5 全数落码（2026-11-18）

> 日期：2026-11-18（用户令「请对审查出的全部问题进行处理」，对象＝398 批〔0dw S1 三件〕深度审查结论）
> 性质：审查处置批——398 批三件〔量尺换 tokenizer／逐调用 usage／preflight 载体章〕审查发现的 **P1×1＋P2×1＋P3×5** 一次性处置；0dw 计数 **73 不变**（S2 载体进体随下次重建、S3 真机核证待窗口不动）。
> orz 子模块：398 基线 `e5103563` 之上纯落码（本批新提交 `851d5341`），零 bump、零重建、零换装（P1 修复的 schema/registry/conformance 侧不进载体二进制）。
> 父仓随批件：`runtime/run-preflight-event-payload-v0.2.schema.json` 新立＋registry v02 条目＋fixtures 两件＋Python conformance 双测试＋payload schema 文案修正。

## §0 结论

审查发现全数处置：**P1×1**（398 批 preflight 扩键与已登记 payload schema 冲突——批档「registry/judge 两侧均无此族、扩键安全」论据证伪，潜伏期＝S2 载体进体后每次真机验收全卷校验必报 payload violation）＋ **P2×1**（台账指针化 freed 记账双尺相减——批档自称「台账指针化循环同尺换装」实际只换外环，恰是批档自谴的「保留第二把 chars/2 尺」）＋ **P3×5**（ruler 哈希字段歧义、命中路径 tool_calls args 仍分配、call_seq↔model_calls 合计对账无执法主体却按已执法口径表述、`model_summary_marker_estimate` 死代码残留 chars/2 尺、carrier exe 哈希 POSIX rename 盲区未注记）。零设计语义变更（处置均为 398 设计授权范围内补全与诚实化）。

## §1 P1：preflight 扩键与已登记 payload schema 冲突（闭合）

398 批在 `run_preflight` manifest 扩 `carrier_version`＋`binary_sha256` 两键，安全论据「envelope schema 对 preflight 载荷无形状约束（registry/judge 两侧均无此族）」。审查证伪：**registry v01 侧有严格 schema**（`run-preflight-event-payload-v0.1.schema.json`，`additionalProperties:false` 仅三键）且 Python 参考法官 `_resolve_payload_schema` 对 v0.2 轨事件按 Phase B 裁决「无自有 payload schema 则**回落 v0.1 文件**」解析（Rust conformance 同语义）——五键载荷必报 `Additional properties are not allowed`（审查实验实证）。消费方是真实验收流程：`validate_journal_file` 全卷校验（0AY S4 等先例）⇒ **0dw S2 进体后每次真机核证（含 S3 自身）会全卷报错**。398 批测试网全绿的原因＝Rust replay 只验链 mechanics 不做 registry 载荷验证＋对拍 corpus 只跑族级法官（`_payload_errors` 不在对拍集）——两套网恰好都兜不住这一格。

处置（沿本批 model_call_usage 先例）：
1. **自有 v0.2 schema**：`runtime/run-preflight-event-payload-v0.2.schema.json` 新立——三键原样＋两章键 properties（`additionalProperties:false` 保持）。**两键不进 required**：章前历史 v0.2 journal（三键，含 runtime/fixtures 全部 12 件既有期刊）合法存在（grandfather，与回落语义的历史兼容面一致）；新 journal 恒携带两键（bootstrap_session 生产侧单写）。
2. **诚实形态入 schema**：`binary_sha256` pattern `^([0-9a-f]{64}|unavailable)$`（sha256_hex 小写＋读失败哨兵）；`carrier_version` minLength 1（未注入哨兵 `"unknown"` 同合法）。
3. **registry v02 条目**＋**fixtures 两件**（五键正型／越键负型）。
4. **测试双面**：Python `TrackResolutionTests::test_v02_track_preflight_resolves_to_v02_schema`（解析钉）＋`PreflightCarrierIdentityPayloadTests` 六案（五键过／三键 grandfather 过／哨兵过／越键拒／空版本拒／虚构哈希拒）；Rust `fixture_journal_conformance.rs` 两钉（五键重封正型全链绿＋越第六键恰在 payload 阶段拒且重封后仅此一类违例）。
5. **不实论据修正**：session.rs 注释两处（v0.1 回落表述过时化＋登记证伪与闭合）；398 批档 §③ 勘误注记随批。

## §2 P2：台账指针化 freed 记账混尺（闭合）

398 批量尺换装只换了台账指针化循环的**外环**（`estimate` 初值与候选选择走 ruler），freed 记账 `after` 两处（`action_ledger.rs` pointerize_oversized／pointerize_replay）残留 `chars().count()/2` ⇒ `freed = before(ruler尺) − after(chars/2尺)` 双尺相减喂循环退出条件：CJK 高估 freed→提前退出（应指针化的没指针化）、ASCII 低估→过度指针化，方向随内容构成翻转。批档「边界与不做」自谴「保留第二把 chars/2 尺即保留下项谴责的双尺漂移」，此处恰是漏网且未入漂移注记。附带：指针化档案头文案「估算: ≈{before}tk token（chars/2）」自换尺起名不副实。

处置：两处 `after` 换 `ruler::text_tokens`（与 `before` 同尺）＋档案头标签改「provider tokenizer 量尺」＋`PointerizeStats::freed_tokens`／门槛常量 docstring 口径随批更新＋**回归钉** `freed_accounting_is_same_ruler_as_candidate_estimate`（CJK 30K 载荷：freed 必须＝`text_tokens(原文) − text_tokens(指针)` 同尺差；混尺修复前必失败）。既有 `oversized_tool_results_are_pointerized_idempotently`（ASCII 'B'×60K）实跑存活——tokenizer 对 B 串近 1 token/字符，`freed > 25_000` 判据在新尺下照常成立。

## §3 P3 处置清单

1. **ruler 哈希字段歧义**：`message_tokens` 哈希输入 content 与 reasoning_content 之间无边界——SipHash 流式拼接使 `("ab", Some("c"))` 与 `("abc", None)` 恒同键（`write(a)+write(b) == write(ab)`）⇒ 第二条消息拿到第一条缓存值且两者 miss 文本不同（确定性错值，非概率碰撞）。处置：补 `0xff` 字段边界字节（与 tool_calls 既有边界同款）。钉：`content_reasoning_boundary_prevents_key_aliasing`（先缓存拼接歧义形态、无 reasoning 形态必须等于自身文本冷计数——混尺键下必失配）。
2. **命中路径零分配真化**：398 批宣称「命中路径零分配」对含 tool_calls 消息不成立（命中路径每调用 `serde_json::to_string(&tc.arguments)` 分配）。处置：`hash_json` 就地规范哈希（判别式字节＋长度前缀＋对象键迭代序〔同一 Value 迭代序恒定〕；f64 经 `to_bits` 写 u64）——命中路径自此真零分配，宣称名实一致。
3. **call_seq↔model_calls 合计对账无执法主体**：payload schema description 按「已执法」口径表述「最大 call_seq == model_calls」，Rust 法官 docstring 称「合计对账由 usage_metering 族承载」——实际 usage_metering 族无此执法、model_call_usage 族明示不执法合计。处置：schema description＋法官 docstring 双改如实（「journal 单源可复算，S3 读数面核对，双族均不执法」）。
4. **死代码残留 chars/2 尺**：`summary.rs` `model_summary_marker_estimate`＋`MODEL_SUMMARY_MARKER_OVERHEAD_TOKENS`（语义轨替换文本估算）全仓零调用。处置：删除＋注记（语义轨缩减判定实际走 `SUMMARY_MARKER_ESTIMATE_TOKENS` 固定常数，量尺无关；不留第二把未接线的 chars/2 尺）。
5. **carrier exe 哈希 POSIX rename 盲区**：Windows（在役）文件锁下恒诚实；POSIX rename 换装（同路径覆盖 inode）时换装窗口内在跑旧进程 `fs::read(current_exe)` 读到新文件 ⇒ 旧进程盖新哈希。处置：carrier.rs 模块头注记平台边缘＋残余控制（换装停旧起新纪律＋同 run 版本/哈希并读互证）＋跨换装窗口归因以 `carrier_version` 为主锚（记入 S3 读数面）。

## §4 测试面（全绿）

| 套件 | 读数 |
|---|---|
| orz-loop lib | 988/0/4（398 基线 986/0/3 ＋新钉 2：freed 同尺／哈希边界；4 ignored＝既有活体探针×3＋ruler perf 探针，零新增） |
| orz-assurance lib | 311/0（对拍 corpus 计数钉 266 不变） |
| orz-assurance fixture conformance | 11/0（基线 9＋新钉 2）＋cli 4/0＋fake provider 9/0 |
| orz-host lib（串行，RUST_MIN_STACK=32M） | 462/0/6（与 398 基线逐位一致——host 改动均为注释级，零行为变化；carrier/preflight 面 12 钉在列） |
| Python runtime conformance | 282/0（275＋新 7：解析钉 1＋payload 六案） |
| Python assurance journal 族 | 52/52 |
| clippy | 五 crate：orz-host 120＝orz-tui 120 持平（「120=120」口径）；orz-loop 86／orz-bin 129／orz-assurance 6——本批改动行零警告命中 |
| fmt／`git diff --check` | 净 |

Grandfather 证据：全部 12 件 v0.2 既有 fixture 期刊（含三键 preflight）在新 registry v02 条目＋新 schema 下双侧全绿（Rust `all_fixture_journals_pass_rust_schema_conformance`＋Python real-journal 套件）。

## §5 边界与不做清单（本批）

- 两章键**不进 required**＝历史 v0.2 journal 三键形态永续合法——「章后新 journal 恒两键」是生产侧不变量（bootstrap_session 单写），不是 schema 执法面；若未来需要强制，属新裁决（会断历史兼容）。
- call_seq↔model_calls 合计对账**不补执法**（扩 usage_metering 族会动冻结镜像＋corpus，收益薄）；S3 真机读数时按 journal 复算核对，措辞已如实化。
- 跨车道并发序假设（「到达序＝写序」）**不加机械保证**——它是车道串行架构的涌现性质（审查 P3 已核实内联 await＋并行批缓冲重放），车道并行化批次出现时须回看法官单调性执法；本批仅维持 docstring 声明。
- 398 批档 §①②（量尺换装＋逐调用 usage 事件）审查结论「设计正确、读数复现」不动；本批只动 §③ 论据面与 §① 的两处宣称措辞（perf 探针读数复现实证在案）。
- 0dw 开放项不动：S2 载体进体随下次重建批（进体时请将本批 orz 提交一并纳入——P1 schema 修复使 S3 全卷校验可用）；S3 真机核证判据不动。
- payload fixtures（run-preflight 两件）沿 398 model_call_usage 先例作合约样本（无逐名消费测试），不接线。

## §6 入口

- 审查对象：398 批档 [`398_*`](398_0DW_S1_RULER_TOKENIZER_PERCALL_USAGE_PREFLIGHT_2026-11-17.md)（§③ 勘误注记随本批）／0dw 立项档 [`0DW_*_REGISTRATION`](0DW_CONTEXT_FACE_SCALE_RULER_ALIGNMENT_REGISTRATION_2026-10-30.md)。
- 实现：`orz/crates/orz-loop/src/ruler.rs`（哈希边界＋hash_json）／`action_ledger.rs`（freed 同尺×2＋标签）／`summary.rs`（死代码删除）；`orz/crates/orz-host/src/session.rs`（注释修正）／`carrier.rs`（平台边缘注记）；`orz/crates/orz-assurance/src/journal/families.rs`（docstring 修正）；`orz/crates/orz-assurance/tests/fixture_journal_conformance.rs`（两钉）。
- 父仓：`runtime/run-preflight-event-payload-v0.2.schema.json`／`run-event-payload-registry-v0.1.json`（v02 条目）／`runtime/fixtures/run-event-v0.2/payloads/run-preflight.{minimal.valid,constraint.invalid}.json`／`runtime/tests/test_run_event_journal_validation.py`（解析钉＋payload 六案）／`runtime/model-call-usage-event-payload-v0.2.schema.json`（call_seq 措辞）。
- 台账：TODO 399 行／BACKLOG 本批记录指针／索引头 v4.359。
