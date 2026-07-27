# Retrieval Sub-Agent audit（2026-07-27）

## 裁决

新增检索子代理（Retrieval Sub-Agent）的 no-model fixture 纵向切片，包括任务合同、结构化结果格式和对话关闭协议。
子代理与主代理完全隔离，只执行检索与前处理任务，返回格式透明可审计，对话生命周期复用现有 ConversationNamespace 设计。
本轮实现全部遵循 no-model/offline 边界，不调用真实模型、不联网、不读取 credential。

## 问题背景

LIF 等项目需要大量检索，而检索结果直接注入主代理上下文会严重污染上下文窗口，且模型可能将检索摘要与原始证据混淆。
解决方案：将检索任务委托给独立的子代理——独立上下文、独立来源 ledger，只回传结构化结果。

## 设计决策

### 彻底隔离
- 子代理仅获得检索相关能力子集（search, web_fetch, file_read, tool_registry）
- 子代理不得执行写入、执行、或超出合同范围的任何操作
- 子代理结果标记为 `delegated_retrieval`，主代理引用时必须保留来源链

### 主代理控制生命周期
- 子代理对话由主代理打开（派遣合同）和关闭（确认本轮完毕）
- 关闭不清零：journal 完整保留，子代理可被同一 parent session 重新唤醒
- 子代理对话与主代理对话按同等标准存储

### 结果透明
- 子代理回传不是模糊摘要，而是分层结构化结果：
  - `query_summary`：实际执行的每次检索动作
  - `source_ledger`：每项来源的全文可见性状态
  - `filtering_log`：每项被排除的内容及原因
  - `organized_response`：按合同要求分节的结构化回答
  - `raw_source_refs`：主代理可据此独立复核

## 新增组件

### 1. Retrieval Task Contract (`build_retrieval_task_contract`)

冻结的委托合同，包含：
- `retrieval_question`：主代理的具体信息需求
- `allowed_source_categories`：允许的来源类型（academic_paper, web_page, documentation, code_repository 等）
- `required_visibility`：要求的全文可见性等级
- `return_format.sections`：要求的返回小节
- `scope_boundary.forbidden_topics`：禁止触碰的主题
- `max_sources` / `context_budget_tokens`：资源预算

委托约束字段强制为 `subagent_must_not_expand_scope_beyond_contract`。

### 2. Retrieval Result (`build_fake_retrieval_result` + `validate_retrieval_result`)

回传格式 schema 包含两部分机械验证：

**来源完整性验证**（`_check_source_visibility_ledger`）：
- 所有 section 引用的 source_id 必须在 source_ledger 中注册
- 每项来源的 visibility 必须达到合同要求的等级
- filtering_log 必须记录每项排除

**范围合规验证**（`_check_scope_compliance`）：
- query_summary 中每次检索的 source_category 必须在合同允许范围内
- 检索文本不得触碰 forbidden_topics
- 来源数量不得超过 max_sources

### 3. Session Close Receipt (`build_retrieval_session_close_receipt`)

关闭回执确认：
- `conversation_zeroed: false` — 不清零
- `journal_preserved: true` — journal 完整保留
- `can_resume: true` — 可恢复
- `subagent_resumable: true` — 需 parent_session_id 匹配 + 新合同

### 4. No-model Fixture Pipeline

`run_retrieval_subagent_fixture` / `verify_retrieval_subagent_fixture`：
- 输入：合同、query_texts、source_entries、filter_entries、sections_content（全部 fake）
- 输出 5 个 artifact：
  - `retrieval-task-contract.json`
  - `retrieval-result.json`
  - `retrieval-result-validation-receipt.json`
  - `retrieval-session-close-receipt.json`
  - `retrieval-subagent-fixture-summary.json`
- 独立 verifier 重建 result/receipt 并检测篡改

## 文件清单

### 新增文件
| 文件 | 用途 |
|---|---|
| `assurance/retrieval_subagent.py` | 核心模块 |
| `assurance/retrieval-task-contract-v0.1.schema.json` | 检索任务合同 schema |
| `assurance/retrieval-result-v0.1.schema.json` | 检索结果 schema |
| `assurance/retrieval-session-close-receipt-v0.1.schema.json` | 对话关闭回执 schema |
| `assurance/tests/test_retrieval_subagent.py` | 22 个测试 |
| `docs/RETRIEVAL_SUBAGENT_AUDIT_2026-07-27.md` | 本审计文档 |

### 修改文件
| 文件 | 变更内容 |
|---|---|
| `assurance/__init__.py` | 新增 7 个导出符号 |

## 测试覆盖

- **Retrieval Task Contract**（7 个测试）：最小合同、自定义 sections/topics、无效 parent session、空问题、空 categories、无效 visibility、预算超范围
- **Retrieval Result Validation**（6 个测试）：合同一致验证、contract_id 不匹配、visibility 不足、scope 违规、缺失 section、filtering_log 保留
- **Session Close Receipt**（3 个测试）：不清零确认、四种 close trigger、非法 trigger 拒绝
- **Fixture Pipeline**（6 个测试）：写入+验证、非空 root 拒绝、result 篡改检测、close receipt 篡改检测、alias 函数、无来源结果验证

## 与现有架构的关系

| 层级 | 复用组件 |
|---|---|
| 会话身份 | ConversationNamespace（子代理派生自 parent session） |
| 能力委派 | capability_delegation_receipt（subject_kind: "child_agent"） |
| 来源完整性 | source_visibility_gate |
| 对话存储 | envelope / archive / journal（与主代理同等标准） |
| 自身保障 | orientation_checkpoint + stagnation_guard + tool_availability_gate |

## 边界与限制

- 当前为 no-model/fake fixture：不连接真实 ConversationNamespace，不发起真实检索或 API 请求
- 子代理的 ConversationNamespace 派生逻辑（parent-child 关系）在 fixture 中仅模拟
- 来源可见性验证基于子代理声明，主代理必须独立复核
- close 协议确认不清零，但实际归档路径未接入 archive.py

## 下一步

适合接入：
- 子代理 ConversationNamespace 的 parent-child 派生实现（复用 P1 envelope）
- 真实 retrieval tool adapter 接入（web search API, academic database 等）
- 子代理与主代理之间的 tool_availability_gate + orientation_checkpoint 共享保障
- 多轮检索委托（同一子代理 session 内多次派遣新合同）
