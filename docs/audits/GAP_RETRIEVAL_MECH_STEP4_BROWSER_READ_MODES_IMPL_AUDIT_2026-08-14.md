# GAP-RETRIEVAL-MECH 步骤 4 实施审计（2026-08-14）

- 范围：FUS-RETRIEVAL-MECH 实施序列第 4 步——`browser_read` 范围/模式参数
  扩展（全文/预览/关键词提取）与 local_browser 第二段计数域复用（web_fetch
  的 per-activation 语义：activation 累计、去重 URL 计数、continue 重入不
  重置、activation 关闭清零；随子代理循环传参并回写激活侧车）。
- 设计入口：
  [`RETRIEVAL_MECHANICAL_CONTROLS_DESIGN_2026-08-13.md`](../RETRIEVAL_MECHANICAL_CONTROLS_DESIGN_2026-08-13.md)
- 前置：B-1、步骤 2、步骤 3、步骤 5 已闭合；2026-08-14 用户裁决（主 Agent
  不执行检索任务、主车道投影移除 browser_read、子代理投影恢复、第二段计数域
  复用 web_fetch per-activation 语义）已登记在设计 §1.3 注。
- 迁移纪律：Schema/verifier 先行 → 接缝扩展 → producer → 测试/门禁全量验证。

## 1. 工具契约（定稿）

- `browser_read` 新增可选 `mode`：`full` / `preview` / `keywords`，默认
  `full`；`keywords` 为字符串数组（仅 mode=keywords 必需，1..16 个非空、
  去重、每词 ≤64 字符）。
- `full`：既有行为（≤100K 字符，截断打机械页脚）。
- `preview`：前 4_000 字符（`PREVIEW_READ_CHARS`）；短页未截断时返回完整
  内容（证据仍可按全文处理），截断时打页脚并 `truncated=true`。
- `keywords`：机械上下文摘录——纯 ASCII 文本/词大小写不敏感、其余精确子串；
  每词 ≤3 段（`KEYWORD_EXCERPTS_PER_TERM`）、半径 160 字符
  （`KEYWORD_EXCERPT_RADIUS`）、总输出 ≤12_000 字符
  （`KEYWORD_TOTAL_CHARS`）；恒 `truncated=true` 并打页脚；无命中返回中性
  陈述。证据恒为 `partial_text_observed`。
- 输出 JSON 携带 `mode`；keywords 额外携带 `keywords` 与页脚摘要
  （terms/excerpts 数），preview 额外携带 `preview_chars`。

## 2. 已变更（核心）

### 2.1 orz-host（工具面）

- `local_browser/mod.rs`：`browser_read_tool_def` 声明 mode 枚举与 keywords
  数组；`handle_browser_read` 严格解析新参数（未知键、非法 mode、keywords
  越界/重复/空项、mode 与 keywords 不匹配均返回稳定的
  `[browser_read_invalid_arguments]`）；新增 `keyword_excerpts` 机械提取
  与字符边界辅助；+5 测试（preview 截断/短页、keywords 摘录、无命中中性
  陈述、严格参数校验、工具定义声明 mode）。

### 2.2 orz-loop（计数域与门禁）

- `relay.rs`：新增 `is_candidate_counted_tool`（web_fetch 家族 +
  `browser_read`；`pdf_read` 不计数）；+1 边界测试。
- `controller.rs`：`WebFetchGateDecision` → `CandidateGateDecision`；
  `web_fetch_candidate_gate`/`refuse_web_fetch` → `candidate_gate`/
  `refuse_candidate`，按工具家族生成稳定拒绝码（`web_fetch_candidate_*` /
  `browser_read_candidate_*`）；计数域仍为 `ActivationState.web_fetch_candidates`
  （同一 Vec、同一 `ORZ_WEB_FETCH_CANDIDATE_CAP`，跨工具共享精确字符串
  去重）；`tool_completed` 的 `candidate_count`/`candidate_cap` 覆盖
  browser_read 成功与 host 错误路径。
- 证据映射：`build_evidence_record` 的 browser_read 分支按 `mode` 判定——
  keywords 恒 partial（observed_scope=keyword excerpts）；preview 截断时
  partial（first portion (preview)）、未截断（短页）保持 full；full 沿用
  页脚/长度兜底。
- 主车道 fail-closed 加固：主车道（无计数域）直呼 browser_read 现在由候选
  门禁以 `browser_read_candidate_count_unbound` 拒绝（早于 ACAF 票务），与
  “主 Agent 不执行检索任务”裁决一致。
- orz-loop +5 测试：browser_read 门禁计数/去重/满额拒绝、缺 url/无域
  fail-closed、外部 lane 计数回流、continue 跨派发累计、证据模式映射
  （preview 短/长、keywords）。

### 2.3 Schema/verifier（先行）

- `runtime/tool-completed-event-payload-v0.1.schema.json`：candidate 字段
  描述扩展为 web_fetch 家族 + browser_read。
- `assurance/run_event_journal_validation.py`：
  `_verify_v02_web_fetch_candidate_count` → `_verify_v02_candidate_count`，
  新增 `_is_candidate_counted_tool`；非 error 车道完成必须携带计数字段、
  字段仅限候选计数家族、cap-exceeded 拒绝（两种家族码）必须成对且
  count==cap。
- Python +6 测试（browser_read 成功携带字段、非 error 缺字段拒绝、
  cap-exceeded 合法边界、缺字段拒绝、count!=cap 拒绝、家族边界消息更新）。

### 2.4 测试与 fixture

- orz-host：205 passed（+5）；orz-loop：280 passed（+5）。
- conformance capture 14 场景全部重捕并 dev-copy 回
  `runtime/fixtures/run-event-v0.2/journals/`（`local-browser-read` 的
  browser_read `tool_completed` 现携带 candidate_count=1/candidate_cap=8；
  Rust 回放校验通过）。
- Python：runtime 244 passed；assurance 1607 passed / 14 skipped。
- orz-bin ACAF e2e 中 5 个 browser_read 场景从主车道直呼迁移到外部检索
  子代理车道（D-13 activation 绑定），并更新断言：缺 url 由候选门禁
  `browser_read_candidate_url_missing` 拒绝（取代旧 shadow 静默跳过，
  登记为更强保证）、verify RPC 失败仍单次 `signer_unreachable` 拒绝且
  browser_read 不 ToolStarted、invalid URL shadow 仍单次 target_mismatch
  且工具照常执行；orz-bin 42 passed。

## 3. 决策登记

| ID | 决策 | 依据 |
|---|---|---|
| D-1 | mode 契约：full/preview/keywords + 固定常量（4K/16/64/3/160/12K） | 设计 §1.3 只定方向未给具体参数；机械常量可审计、可测试，拒绝开放式长度 |
| D-2 | browser_read 与 web_fetch 共用同一 `web_fetch_candidates` 计数域与 cap（精确字符串去重） | 设计 §1.1/§1.3 “复用同一计数域”；两工具是同一候选核验预算的两种消费面 |
| D-3 | 拒绝码按家族独立（`browser_read_candidate_*`），verifier 覆盖两家族 | 审计面可区分工具；拒绝语义（无 ToolStarted、Denied 熔断同面）不变 |
| D-4 | 证据映射按 mode：keywords 恒 partial；preview 截断才 partial（短页全文可见）；full 沿用既有 | §3.7.5 可见性表以“模型实际观察内容”为准；机械判定不依赖模型自述 |
| D-5 | 主车道 browser_read 由候选门禁 fail-closed（count_unbound），早于 ACAF；ACAF e2e 迁移到子代理车道 | 用户裁决“主 Agent 不执行检索任务”；投影移除后的调用必须被机械拒绝 |
| D-6 | keywords 匹配：纯 ASCII 大小写不敏感，其余精确子串；无命中返回中性陈述而非错误 | 机械、确定性、跨 UTF-8 安全；空命中是合法结果（页面无该词） |

## 4. 边界（明确未做）

- 提示词相应缩短（计数/预筛/引用规则）与测试更新——步骤 6。
- `pdf_read` 不进候选计数域（PDF 证据管线，非候选核验）。
- canonical/host 级去重仍是步骤 3 预筛职责；计数域保持精确字符串去重。
- live 浏览器 e2e 未跑（`GSA_RUN_LIVE_BROWSER_TESTS` env 门控；本步为
  工具面与门禁机械改造，CDP 读取面未改）。

## 5. 验证汇总

- orz-host：205 passed / 0 failed / 4 ignored。
- orz-loop：280 passed / 0 failed / 3 ignored。
- orz-assurance：151 passed；orz-tui：178 passed；orz-bin：42 passed
  （含 acaf_e2e 21、orz-signer 12）。
- Python runtime：244 passed；assurance：1607 passed / 14 skipped。
- `scripts/check_repository.py`：valid / error_count 0（schemas 242）。
- `cargo fmt --all` 已跑；两个仓库 `git diff --check` 均 0。

## 6. 仓库边界与提交顺序

- 本步横跨父仓库 `D:\CLI`（Schema/verifier/设计/审计/待办/fixture）与
  嵌套仓库 `D:\CLI\orz`（Rust 实现与 e2e）。
- 提交顺序：先提交父仓库，再提交 orz 仓库（与既有步骤审计一致）。

## 7. 复核修复（2026-08-14，全面审查批次）

针对步骤 4 闭合后的全面审查（设计合理性/实现合理性/符合性），处理以下问题：

1. **候选计数消费时机（审查主项）**：候选门禁此前在权限/ACAF 票据门禁之前直接
   把 URL 写入计数域——被权限或票据拒绝的调用也会消耗候选，且拒绝事件不携带新
   计数。修复：`candidate_gate` 只做决策（保持“候选拒绝无 ToolStarted、不签发票据”
   的既有顺序），新增 `commit_candidate` 在权限/票据门禁通过后、ToolStarted 前提交
   消费；被后置门禁拒绝的调用不消耗预算、拒绝事件不携带计数字段。登记：ADR-0010
   §14.11、设计 §1.1/§1.3 注、步骤 2 审计 D-8 复核注。
2. **keywords 12K 严格上限**：分隔符/省略号此前不计入 `KEYWORD_TOTAL_CHARS`，
   输出可略超上限。修复：摘录正文按整段（分隔符+省略号+摘录）严格预算，超限时
   干净截断；新增主机侧预算断言测试。
3. **keywords 输入上限**：此前对整页原文全量扫描。修复：提取前按 `MAX_READ_CHARS`
   截断输入并打“input capped”页脚说明；新增超出上限的 no-match 测试。
4. **工具定义 Schema**：`keywords` 条目补 `maxLength=64`（与 `MAX_KEYWORD_CHARS`
   同源），工具定义测试同步断言。
5. **页脚 terms 语义**：改为“实际输出中代表的词数”（预算裁剪后），不再是匹配词数；
   无命中仍报请求词数。
6. **命名与事件**：`ActivationState.web_fetch_candidates` 改名为 `candidate_urls`
   （共享计数域）；`refuse_candidate` 仅在车道拒绝事件携带
   `target=external_retrieval`，`count_unbound`（无计数域的主/内车道兜底）不写
   target；Python verifier docstring 修正 Rust 函数名引用。
7. **测试更新**：orz-loop 新增门禁决策/提交拆分的去重与不消耗回归测试；orz-bin
   e2e `fail_closed_verify_rpc_failure_journals_once_and_blocks` 扩展为
   “票据拒绝不消耗 → 重试成功携带 candidate_count=1”的端到端断言。
8. **验证汇总（复核后）**：orz-host 207 passed / 4 ignored；orz-loop 281 passed /
   3 ignored；orz-bin 42 passed（acaf_e2e 21 项含扩展断言）；Python runtime +
   assurance 1851 passed / 14 skipped；仓库门禁 valid（schemas 242）；
   `git diff --check` 0。
