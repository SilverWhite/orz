# 缓存与上下文成本实施审计（2026-08-15）

> 状态：闭合；范围：ORZ-CACHE-CONTEXT-COST（BACKLOG 6b / TODO 11-13）。
> 设计权威：ADR-0010 §3.5 条 6/7、§3.6（v1.9）/ §14.9；探针设计
> `docs/TOOL_AVAILABILITY_PROBE_DESIGN_2026-08-13.md` §11。

## 1. 结论

ORZ-CACHE-CONTEXT-COST 三项（请求 header 变化留痕、探针准确性与稳定性、单轮工具结果
注入预算 + 策略化读取）全部实施闭合。修复后证据：orz-loop 337 通过（原 333 + 新增 4）、
orz-assurance 151 通过、orz-tui 178 通过、orz-bin 全部通过（单测/e2e）、Python
runtime+assurance 1888 通过、14 skipped（新增 header/probe-accuracy 交叉用例与独立
审计模块用例）、`scripts/check_repository.py` valid、0 错误。
2026-08-15 二次全面审查后复核修复（P1 验证器多链 initial 误拒、P2 变化原因未留痕/
辅助请求覆盖/注入字段无机校验、P3 口径与健壮性）全部处理，复核后 orz-loop 338 通过、
Python runtime+assurance 1896 通过、14 skipped、仓库门禁 valid、0 错误；详见 §7。

## 2. 机制实现

### 2.1 请求 header 变化留痕（ADR-0010 §3.5 条 6）

- 新增 v0.2 事件 `request_header_change`：payload 含 `reason`（initial/change）、
  `header_sha256`、`system_sha256`、`tools_sha256`、`config_sha256`、`agent_role`
  （main/internal_retrieval/external_retrieval）、`tools`、`tool_count`，
  change 时附 `previous_header_sha256` 与 `change_kind`（system/tools/config/
  multiple——机械「变化原因」，2026-08-15 二次审查修复；Schema 在 reason=change
  时必填两者）。
- header 指纹 = SHA-256(canonical{system_sha256, tools_sha256, config_sha256})；
  system 摘要 = 组装后的系统提示词 SHA-256；tools 摘要 = 当前投影工具列表
  （name/description/parameters 按 name 排序）canonical JSON SHA-256；config 摘要
  来自 `ModelGateway::config_fingerprint()`（DeepSeekTransport 实现：provider/
  model_id/api_base/max_tokens/thinking/retry，**api_key 不入摘要**；Fake 默认
  `unknown-config` 稳定值）。
- 发射点：每个模型请求构造后、`run_round` 前；同一 `run_agent_loop` 调用内维护
  `last_request_header`，首请求记 initial、后续仅真实变化记 change；消息体不参与
  header（前缀缓存键只包含静态部分）。
- 三车道（主/内/外检索）各自独立；每车道可含多条链——每次 loop 调用首请求
  initial（子代理多 activation/主车道多 run 合法），initial 重置链起点，验证器按
  `agent_role` 分区核对（2026-08-15 二次审查修复）。
- 边界（2026-08-15 二次审查登记）：压缩摘要（`run_template_compact`）/预检等
  loop 外辅助模型请求不参与 header 留痕——其 header（固定 system + 空 tools +
  config）恒定，与探针翻转无交互；若未来接入 loop 内请求级覆盖
  （max_tokens/thinking），须同步扩展 `config_fingerprint`。

### 2.2 探针准确性与稳定性（ADR-0010 §3.5 条 7）

- 验证器新增 `_verify_v02_probe_accuracy`：`tool_availability_check` 完整集翻转后、
  下一 `model_output` 前必须有主车道 `request_header_change(reason=change)`——
  翻转即工具集真实变化，必须真实改变请求 header（翻转↔header 事后核对）。
- 新增独立审计模块 `assurance/probe_accuracy_audit.py`：对 v0.2 journal 输出
  `flips`、`false_complete_candidates`、`false_incomplete_candidates`、
  `flip_without_header_change` 观察；调用时门禁类 error code
  （permission_denied / candidate_cap_* / round_inject_budget_exceeded 等）明确
  不计为假完整（不变量 2：调用时机械门禁是最终兜底）；检索车道事件
  （带 `target`）不参与主车道判定。
- 兼容边界：2026-08-15 之前捕获的 v0.2 journal 无 header 事件，翻转↔header
  交叉核对自动跳过（不追溯判旧 journal 非法）。
- 可选后端接线的翻转测试边界保持不变（探针设计 §10）——接线时翻转 orz-host
  能力访问器并补翻转测试，header 留痕会自动体现工具集变化。

### 2.3 单轮工具结果注入预算 + 策略化读取（ADR-0010 §3.6）

- `ORZ_MAX_INJECT_TOKENS_PER_ROUND`：默认 50K，控制器构造时读取并解析
  （trim 正整数，0/非法回退默认），测试 seam `with_max_inject_tokens_per_round`。
- 累计口径：每模型轮（每个 `for tc in tool_calls` 批次）独立计数，对每个已执行
  结果按 `[tool] output` 的 chars/2 估算（复用压缩估算器 `estimate_message_tokens`）。
- 超限行为：累计 ≥ 预算后，本批后续调用无 ToolStarted 拒绝，`tool_completed`
  记 `error=round_inject_budget_exceeded` + `inject_tokens_used/inject_tokens_budget`
  （tool-completed v0.1 schema 扩展可选字段）；模型面收到显式提示（预算已满 +
  grep/结构优先 + read_file offset 分段续读）；拒绝键进连续拒绝熔断（同一归一化
  键三轮后触发换策略提示）。
- 计数口径（2026-08-15 二次审查注记）：计入本轮实际注入的 tool 消息（含权限/写
  门禁等无 ToolStarted 拒绝的合成消息——它们同样进入下一请求）；预算拒绝提示本身
  不计。`_verify_v02_inject_budget` 机械校验拒绝码⇄字段配对（2026-08-15 新增）。
- 提示词：`BASE_SYSTEM_PROMPT` 新增「读取纪律（缓存成本）」段落——grep/结构提取
  优先、证据关键文件才全文、大文件 offset 分段、50K 预算与超限提示。

## 3. 契约变更

- 新 v0.2 事件 `request_header_change`：`runtime/request-header-change-event-payload
  -v0.2.schema.json`（2026-08-15 二次审查增 `change_kind` 枚举 +
  allOf 条件：reason=change 必填 `change_kind`/`previous_header_sha256`）；
  `runtime/run-event-v0.2.schema.json` 枚举 46→47。
- `tool-completed-event-payload-v0.1.schema.json` 增可选
  `inject_tokens_used` / `inject_tokens_budget`（跨轨道共用）；验证器新增
  `_verify_v02_inject_budget`（拒绝码⇄字段配对与范围，2026-08-15 二次审查修复）。
- Python 验证器 `assurance/run_event_journal_validation.py`：注册表新增
  `request_header_change` + `_verify_v02_request_header`（每车道 initial→change
  链、每车道多链 initial 重置、previous 一致、header 必不同、change_kind 与摘要差
  一致、tools 唯一、tool_count 匹配）+ `_verify_v02_probe_accuracy`。
- fixture 生成器 `scripts/generate_run_event_fixtures.py` 增 good/bad 样例并重新
  生成（v0.2 payloads 41→43→45：新增 change 正例与 change-missing-kind 负例；
  v0.2 envelope 62→63；fixtures README 46→47）；conformance 计数 46→47。
- Rust `EventType::RequestHeaderChange`；`ModelGateway`/`RoundAgent` 增
  `config_fingerprint()`；TUI `TuiEvent::RequestHeaderChange`
  （events/bridge/projection）。
- 新增 `assurance/probe_accuracy_audit.py` + `assurance/tests/test_probe_accuracy
  _audit.py`（独立审计，非硬门禁）。

## 4. 测试

新增 orz-loop 用例（+4，337 通过）：
- `request_header_fingerprint_is_stable_and_component_sensitive`：同输入稳定、
  工具顺序无关、system/tools/config 任一变化即变化。
- `request_header_payload_shapes_initial_and_change`：reason/摘要/agent_role/
  previous 字段形状。
- `inject_budget_refuses_later_calls_of_batch_with_offset_hint`：首个调用执行、
  后续无 ToolStarted 拒批、journal 带 used/budget、模型面 offset 提示。
- `max_inject_tokens_per_round_parse_rules`：env 解析规则。

2026-08-15 二次审查修复新增（orz-loop 338 通过）：
- `header_change_kind_attributes_component_changes`：system/tools/config/multiple
  归因精确。

既有序列断言同步：`run_turn_full_gate_sequence` / `text_deltas_forwarded_in_order
_before_model_output`（+`RequestHeaderChange`）、orz-bin stdio e2e event_count
9→10。

Python runtime 新增：
- `RequestHeaderChangeTests`（initial→change 链、车道分区、previous/header 一致性、
  tool_count 校验）。
- `ProbeAccuracyCrossCheckTests`（翻转→header change 干净/缺失拒绝、子代理 header
  不清主车道翻转、旧 journal 兼容边界）。
- `test_probe_accuracy_audit.py`（假完整/假不完整候选、门禁码排除、车道隔离）。

2026-08-15 二次审查修复新增（Python 1896 通过、14 skipped）：
- `RequestHeaderChangeTests`：同车道第二次 initial 合法（新链重置）、change_kind
  必填且与摘要差一致、initial 禁带 change_kind。
- `InjectBudgetCrossCheckTests`：拒绝码必带字段、字段仅该码可带、成对与范围。
- `test_probe_accuracy_audit.py`：`control_ticket_rejected:*` 前缀计门禁码、
  round_inject_budget_exceeded 不计假完整、WORK_TOOLS 与验证器单源一致。

## 5. 边界与兼容

- 仅 DeepSeek OpenAI 兼容面；Anthropic cache_control / 多断点缓存不在当前范围。
- 旧 v0.2 journal（无 header 事件）通过验证器兼容边界；新 producer 的 journal
  必须含 header 事件（首请求 initial）。
- `config_fingerprint` 不含 api_key（密钥轮换不构成 header 变化）。
- 预算估算为 chars/2（与压缩同口径）；单个超预算结果本身仍注入（超限拒绝的是
  **后续**调用），与 ADR「超限拒绝本批后续调用」一致。
- 压缩摘要/预检等 loop 外辅助模型请求不参与 header 留痕（2026-08-15 二次审查登记）。
- `config_fingerprint` 不含请求级覆盖（max_tokens min-cap、thinking override）；
  当前 loop 内请求均传 config 等价值（2026-08-15 二次审查登记）。
- header/config 摘要的 JSON 序列化失败回退为固定错误串（理论不可达，2026-08-15
  二次审查登记）。

## 6. 文档同步

- ADR-0010 §14.9 增实施闭环条目 ②。
- BACKLOG 6b 增 2026-08-15 实施闭合；TODO ORZ-CACHE-CONTEXT-COST 三项勾选并更新
  未闭合扫描快照（37→34）。
- 索引：FUS-REQUEST-CACHE `pending`→`implemented`，§8 状态速查同步，头部登记行追加。
- 探针设计 §11 标注实施闭合（§11.1）。
- 2026-08-15 二次全面审查修复：ADR-0010 §14.9、BACKLOG 6b、索引头部、探针设计
  §11.1 同步 `change_kind`、多链 initial、注入预算验证规则与辅助请求边界。

## 7. 二次全面审查修复（2026-08-15）

基于 2026-08-15 全面审查（含合成 journal 实证），处理全部发现：

| 严重度 | 发现 | 处理 |
|---|---|---|
| P1 | 验证器 `_verify_v02_request_header` 拒绝同一车道第二次 `initial`，真实多 activation/multi-run journal 会被误拒 | 验证器改为每车道多链：`initial` 重置链起点（不报错），`change` 仍须接最近事件；删除并替换固化错误行为的 `test_initial_after_initial_rejected`，新增 `test_second_initial_starts_new_lane_chain` |
| P2-1 | ADR §3.5 条6「变化原因」未落事件面 | payload 增 `change_kind`（system/tools/config/multiple），Rust `header_change_kind` 机械归因；Schema allOf 条件必填；verifier 校验与相邻事件摘要差一致 |
| P2-2 | header 留痕未覆盖压缩摘要等 loop 外辅助模型请求，审计「每个模型请求」表述过度 | 登记为有意边界（header 恒定、与探针无交互），ADR/审计/探针设计同步措辞 |
| P2-3 | `inject_tokens_used/budget` 只有 Schema 描述、无 verifier 机械校验 | 新增 `_verify_v02_inject_budget`（拒绝码⇄字段配对、成对、范围）并接线验证链 |
| P3-1 | 预算计数含门禁拒绝合成消息，与审计「已执行结果」口径不符 | 口径注记=实际注入的 tool 消息（含无 ToolStarted 门禁拒绝合成消息；预算拒绝提示不计） |
| P3-2 | config 指纹不含请求级覆盖 | 代码注释 + 审计边界登记（当前 loop 内请求均为 config 等价值） |
| P3-3 | GATE_ONLY_CODES 手工清单不全；WORK_TOOLS 双处复制 | 增 `control_ticket_rejected:` 前缀排除（覆盖 fail-closed 全码面）；WORK_TOOLS 改从验证器单源导入 + 相等性测试 |
| P3-4 | 摘要失败回退固定错误串 | 注释登记（理论不可达） |
| P3-5 | probe 交叉核对 initial 不清 pending flip | 注释说明不可达性，不改逻辑 |

复核证据：orz-loop 338 passed（3 ignored）、Python runtime+assurance 1896
passed、14 skipped、`check_repository.py` valid、0 错误、`git diff --check` 干净。
