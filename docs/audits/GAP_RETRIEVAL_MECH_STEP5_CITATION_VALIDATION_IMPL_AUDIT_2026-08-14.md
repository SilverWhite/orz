# GAP-RETRIEVAL-MECH 步骤 5 实施审计：输出级引用校验器与交付边界接线（2026-08-14）

## 1. 范围

本审计覆盖 P0-B FUS-RETRIEVAL-MECH 步骤 5 的完整实施与验证：

- 设计：`docs/RETRIEVAL_MECHANICAL_CONTROLS_DESIGN_2026-08-13.md` §3（引用纪律机械化）；
- 权威：ADR-0010 §3.7.9（来源绑定 + verifier 机械检查，V11-IMPL-004 缺口）；
- 边界：主 Agent 最终回答形成后、交付前，与 counterexample gate 同一交付边界；
- 实现仓库：`D:\CLI\orz`（Rust）与 `D:\CLI`（Schema/verifier/fixtures/文档）。

## 2. 设计要点回顾（§3.2/§3.3）

1. 扫描主 Agent 最终回答中的 `[来源: ...]` 标记（结构化解析，非 grep 文本）；
2. 标记必须解析到合法来源：ledger `source_id`、可定位 `path:line`、或
   URL/document identity + observed scope；
3. 来源身份必须真实存在于本次检索/读取证据，不得凭空；
4. 引用强度不得超过该来源可见性允许的 claim 上限（复用 §3.7.5 矩阵）；
5. 校验失败 → 显式降级/阻止交付，返回机械 reason code；
6. 校验位置 = 最终回答形成后、交付前（与 counterexample gate 同一交付边界）。

## 3. 实现内容

### 3.1 Schema/verifier/fixtures（先行）

- `runtime/run-event-v0.2.schema.json`：事件枚举 +1 `citation_validation`（42 → 43）；
- 新增 `runtime/citation-validation-event-payload-v0.2.schema.json`：
  - 必填 `schema_version / position=final_answer / decision / marker_count /
    reason_codes / degraded`；
  - `decision=block` 强制 `degraded=true`、reason_codes 非空、`message_block`
    与 `markers` 必填；`decision=pass` 强制 `degraded=false`、reason_codes 空；
  - marker 项：`index / raw / target / binding / status / reason_codes`，
    `binding ∈ {ledger_source_id, path_line, url_identity, document_identity, unresolved}`；
- `assurance/run_event_journal_validation.py`：v0.2 注册表 +1；
  新增 `_verify_v02_citation_validation` 交叉校验（block 必须降级、必须带 reason
  codes 与机械 message_block、marker_count 一致、failed marker 必须有 reason）；
- fixtures：envelope `citation-validation.valid.json`、payload
  `minimal.valid.json` / `constraint.invalid.json`；
- `runtime/tests/test_run_event_conformance.py`：v0.2 枚举覆盖数 42 → 43；
- `runtime/tests/test_run_event_journal_validation.py`：新增
  `CitationValidationTests`（6 项：合法 block、degraded 强制、reason 强制、
  机械块强制、marker_count 一致、passed marker 不带 reason）。

### 3.2 Rust 事件面与 TUI 投影

- `orz-assurance/src/journal/event.rs`：`EventType::CitationValidation`；
- `orz-tui`：`TuiEvent::CitationValidation { decision, reason_codes }` +
  bridge 映射 + label + 投影卡片（仅展示 decision/reason，marker 明细留 journal）。

### 3.3 校验器（`orz-loop/src/citation_validation.rs`，新模块）

- `parse_markers`：按 `[来源:` 前缀 + 下一个 `]` 结构化切分；未闭合标记记
  `marker_unparsable`；
- 目标分类与绑定：
  - `SRC-###` → 本次 run 已提交 ledger 的 `source_id`；不存在记
    `unknown_source_id`；`visibility=unavailable` 记 `source_unavailable`；
  - `http(s)://...` → ledger `source_url_or_ref` 或 `candidate_urls`
    （web_search_result 池）；不在证据记 `url_not_in_evidence`；裸 URL（无
    observed scope/强度 tag）记 `url_missing_observed_scope`；
  - `path:line` → 主车道本次读取证据（`main_evidence`，read_file 等）身份匹配
    记 `path_not_observed`；文件行界校验（>32 MiB / >100 万行跳过行界，身份仍
    必须已读取），行越界记 `path_line_out_of_bounds`，文件不可读记
    `path_not_found`；
  - 文档身份（`文档ID §节/锚点` 或 project_doc/local_file 身份）→ ledger 绑定；
  - 无法分类 → `unresolvable_citation`；
- claim × visibility 上限：marker 携带显式 tag（`observed/derived/synthesized`
  或 `full_text_observed/partial_text_observed/metadata_only`）时，按 §3.7.5
  矩阵与绑定条目 `highest_allowed_claim`/`visibility` 比对，超限记
  `claim_exceeds_visibility`；裸 `SRC-###` 只作写入侧绑定（不宣称强度）；
- 单测 23 项（解析、各绑定形态、上限、多 marker 汇总、全角冒号/代码块
  跳过、canonical URL、主车道 URL 证据、run 唯一 id）。

复核修复批次（2026-08-14，全面检查后处理）：

- 解析增强：全角冒号变体 `[来源：...]` 同样结构化解析（不得借冒号换形
  逃避校验）；围栏代码块（``` / ~~~）与行内反引号代码段先计算 code mask，
  其中的字面标记跳过（格式示例不阻断交付）；未闭合/空标记语义不变；
- URL 归一化复用 `orz-assurance::acaf::target::resolve_network_url`（scheme/
  host 小写、默认端口去除、fragment 丢弃、userinfo 拒绝），替换 v0 的
  朴素去尾斜杠 + 小写匹配；
- URL 绑定同时接受主车道自身 web 证据（web_fetch/browser_read/pdf_read 的
  URL identity），消除 `find_url_entry` 的未用参数与误导注释；
- 文档 `§` 锚点是定位符，不再作为 claim tag 传入上限检查（语义修正）；
- `SRC-###` 改为 **run 级唯一分配**（`next_source_seq` 每 commit 递增、
  run 起始清零）——同一 run 多次 committed ledger 不再重复 `SRC-001`，
  消除 verifier first-match 绑定歧义（详见 §5）。

### 3.4 交付边界接线

- `AgentLoopController` 新增 per-run 证据：
  - `main_evidence`（主车道自身工具调用证据，run 起始清空；`run_agent_loop`
    主车道 `SharedLoopServices.evidence` 由 `None` 改为 `Some(&self.main_evidence)`）；
  - `run_source_ledgers`（每次 `retrieval_result_committed` 时追加该次
    `source_ledger`，run 起始清空）；
  - `next_source_seq`（run 级唯一 source_id 分配；`build_structured_result`
    消费自增，锁不跨 await）；
- `LoopProfile.citation_validation`：主车道 true；grill / 检索子代理 false；
- 最终回答边界（counterexample gate 同一位置）：`response.tool_calls.is_empty()`
  且 gate 已处理（或跳过）后，调用
  `controller.validate_final_answer_citations(&response.text)`；
  失败时：
  - journal `citation_validation`（decision=block、reason_codes、markers、
    degraded、message_block）；
  - 交付文本替换为机械降级块 `[CITATION_VALIDATION_FAILED v0.1] ...`（含
    reason codes，注册进 `is_injected_block_text`，不进 stagnation 输入）；
  - 模型文本不进入对话（`messages` 不追加），返回值为降级块；
- 通过时零事件、行为不变（现有 13 份 conformance journals 重捕后字节级无漂移）。

## 4. 机械 reason codes

| code | 触发 |
|---|---|
| `marker_unparsable` | `[来源:` 未闭合 |
| `empty_marker` | 标记内容为空 |
| `unknown_source_id` | `SRC-###` 不在本次 run ledger |
| `source_unavailable` | 绑定来源 visibility=unavailable |
| `claim_exceeds_visibility` | 显式 tag 强度/可见性超过 §3.7.5 上限 |
| `url_not_in_evidence` | URL 不在 ledger 身份/候选池或主证据 |
| `url_missing_observed_scope` | 外部 URL 裸引用（缺 observed scope） |
| `path_not_observed` | path:line 路径不在本次主车道读取证据 |
| `path_not_found` | 路径已读取但文件当前不可读 |
| `path_line_out_of_bounds` | 行号超出文件行数 |
| `unresolvable_citation` | 目标无法绑定任何合法来源形态 |

## 5. 语义边界（登记）

- 裸 `SRC-###` 标记只做身份绑定与可用性检查；claim 上限在标记带显式
  strength/scope tag 时机械执行（结构化结果内的 claim 上限仍由既有
  `build_structured_result` 校验）；步骤 6 提示词缩短时如需“最终回答必带强度
  tag”可在此收紧；
- `SRC-###` 由 `build_structured_result` 按 run 级唯一计数器分配：单次
  committed ledger 内稳定（`SRC-001` 起），跨同 run 多次 commit 不重号。
  verifier 对历史/外部 journal 中已存在的重复 id 仍按首次出现绑定
  （确定性 first-match，不构成新产出语义）；
- 外部 URL 裸引用按提示词契约（URL/document identity + observed scope）失败；
- `path:line` 必须命中本次 run 主车道读取证据（observation-time 语义），
  行界校验对 >32 MiB / >100 万行文件跳过（身份仍须已读取）；
- 行界校验读取的是交付时文件状态（TOCTOU 边界登记）：身份绑定为
  observation-time，行号越界是 delivery-time sanity check；文件删除/变更
  只影响行界判定（`path_not_found`/越界），不影响身份真实性；彻底快照化
  需要把读取时行数/内容范围引入证据记录，超出本步骤范围；
- 内部文档裸 `文档ID`（无 `§` 锚点）仍可通过（ADR-0010 “优先使用
  section/anchor”为软约束）；`§` 锚点作为定位符不参与 claim 上限；
- 预算耗尽的部分输出走独立 break 路径，不在本校验边界（partial result 语义）；
- 跨 run restore 的 activation 只带 digest/archive 引用，不带 ledger 内容——
  恢复证据不在“本次证据”集合内，v0 不参与绑定；
- 校验通过不 journal（避免全量序列噪音；仅失败事件可审计）；
- grill 答案、检索子代理文本不是 run 正式最终回答，跳过校验；
- 实时 streaming 已先行展示候选文本时，最终交付值仍为降级块（CLI/返回面
  严格阻止；TUI 以系统卡片呈现校验失败）。

## 6. 验证证据

- orz-loop：275 passed / 3 ignored（citation_validation 模块 23 项 +
  controller 集成 2 项：未知 source 阻止、主车道 path:line 通过）；
- orz-assurance：151 passed + 1 doctest；orz-tui：178 passed；orz-bin：
  常规 42 passed / 14 ignored（conformance 捕获类；复核修复后重跑确认）；
- conformance 捕获：13 份既有 journals 重捕无漂移；新增第 14 个场景
  `citation-validation-block.jsonl`（最终回答 `[来源: SRC-999]` → 阻止 + 事件 +
  正常 run_finished）；复核修复后 `orientation-fire-run.jsonl` 因 run 级唯一
  source_id 重捕（5 次 commit → `SRC-001..SRC-005`），其余 13 份无漂移；
- Python：runtime/tests 239 passed；assurance/tests 1607 passed + 14 skipped
  （合计 1621）；
- `python scripts/check_repository.py`：valid、0 错误（v0.2 事件枚举 43、
  payload/fixture 全覆盖、14 份 v0.2 journals 全校验通过）。

## 7. 复核修复批次（2026-08-14）

在步骤 5 闭合后的全面检查（设计合理性 / 实现合理性 / 设计与实现符合性）
中识别并处理的问题：

1. **`SRC-###` 跨 ledger 冲突（P1）**——改为 run 级唯一分配（见 §3.4），
   `orientation-fire-run` 重捕验证（5 个 commit 各 1 条目 → `SRC-001..005`）；
2. **URL 归一化朴素匹配（P3）**——复用共享 canonical 化，默认端口/大小写/
   fragment 变体可绑定，新增单测；
3. **行界 TOCTOU（P3）**——登记为显式边界（§5）；彻底快照化超出本步骤；
4. **`find_url_entry` 未用参数/误导注释（P3）**——主车道 web 证据接入 URL
   绑定，参数真正消费；
5. **全角冒号盲区 + 代码块字面标记误拦截（P3）**——解析增强 + code mask，
   新增单测；
6. **文档 `§` 锚点误当 claim tag（P3）**——定位符与强度 tag 分离；
7. **fixture message_block 与 producer 不一致、审计 ignored 计数失准
   （P2）**——fixture/测试 helper 对齐 producer 格式，审计数字修正；
8. **运行验证补全**——`orz-bin` 常规套件整体重跑（42 passed / 14 ignored），
   全量 Python 与仓库门禁重跑。

## 8. 审计结论

步骤 5（输出级引用校验器与交付边界接线）**已闭合**：

- 设计与实现一致：标记结构化解析、ledger/path:line/URL/document 绑定、§3.7.5
  上限、显式降级 + 机械 reason code、counterexample gate 同一交付边界全部落地；
- Schema/verifier/fixtures 先行且同步；producer → Python verifier 经真实捕获
  journal 双向校验；
- 边界按 §5 显式登记，未宣称超出范围的语义。
