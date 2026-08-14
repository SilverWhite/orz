# GAP-RETRIEVAL-MECH 步骤 6 实施审计：提示词相应缩短与测试更新（2026-08-14）

## 1. 范围

本审计覆盖 P0-B FUS-RETRIEVAL-MECH 步骤 6 的完整实施与验证：

- 设计：`docs/RETRIEVAL_MECHANICAL_CONTROLS_DESIGN_2026-08-13.md` §1（候选计数反馈）、
  §2（机械预筛）、§3.3（引用纪律文本缩减为"标记格式 + verifier 校验"）；
- 权威：ADR-0010 §3.7.9（引用纪律机械化）与 §3.7 条 12（来源加权）；
- 实现仓库：`D:\CLI\orz`（Rust 生产提示词）与 `D:\CLI`（设计/登记文档）。

## 2. 设计要点回顾

1. §1：把提示词里的"候选 ≤5"软约束升级为机械硬门（步骤 2 已落地，`ORZ_WEB_FETCH_
   CANDIDATE_CAP` 默认 8），步骤 6 负责把提示词中残留的软约束改为机械预算反馈契约；
2. §2.3：预筛结果（候选池 + 每候选 tier/weight/形态原因）进入结构化结果，可审计；
   候选池的模型面展示/提示词更新属步骤 6；
3. §3.3：校验器通过后，prompt 中的引用纪律文本可缩减为"标记格式 + 由 verifier 校验"。

## 3. 实现内容

### 3.1 主 Agent 系统提示词（`orz-loop/src/prompt.rs` `BASE_SYSTEM_PROMPT`）

- 引用纪律段按设计 §3.3 缩减：保留标记格式（ledger `source_id` / 本地 `路径:行号` /
  外部 URL+observed scope / 内部 文档ID §节/锚点）与"不得凭记忆声称『参考自某处』"，
  删除"标记是写入侧绑定，不构成验证""行号会漂移""metadata-only 不得生成全文级归因"
  等说明性冗余，收尾改为一句机械契约：
  "标记格式、来源身份、可见性等级与 claim 上限由 verifier 在交付前机械校验，
  不通过即阻止交付"。
- 语义不变：仍要求模型写入侧绑定；校验职责已由步骤 5 的 verifier 承担，提示词不再
  展开解释。

### 3.2 检索子代理系统提示词（`build_retrieval_system_prompt`）

- framework_fallback 通道合同：删除 "at most 5 candidates, never the full
  reference list" 软约束，改为机械预算反馈契约：
  "the candidate budget is mechanical (per-result feedback '候选 N/M，剩余 K')"；
  保留 "verify only high-value / conclusion-dependent candidates"（预筛/初选的选择
  纪律）与 "browser_read is FORBIDDEN in this mode"（二存一）。
- Source weighting 段去冗余：删除档位解释括号（authoritative 1.1 = 政府/机关、
  default 1.0、low_quality 0.7 = 平台/个人博客等），只保留档位枚举与行为契约
  （优先高权重、low_quality 必须 `source_annotations` 标注 `"annotated"`）。
- Citation rule 段去冗余：删除 "the ledger is the single binding authority /
  observation-time path references are notes only" 等解释，保留标记要求、
  记忆引用禁令、内部文档 §节/锚点、外部 URL+observed scope 与
  "metadata-only material never gets full-text attribution"。
- local_browser / off 通道文本不变。

### 3.3 测试更新（`prompt.rs` 单测）

- `base_system_prompt_carries_d1_citation_rule`：原"不构成验证"断言改为
  "由 verifier 在交付前机械校验" + "不通过即阻止交付"；
- `retrieval_prompt_carries_mode_specific_weighting_contract`：
  "at most 5 candidates" 断言改为断言机械反馈契约存在（"候选 N/M，剩余 K"）与
  旧软约束已移除（不含 "at most 5"）；
- 其余 7 项 prompt 单测不变，9/9 通过。

## 4. 边界（明确未做）

- 未新增模型可见的候选池渲染：`candidate_pool`/`prefilter_log` 仍走结构化结果
  （ledger/raw_source_refs），提示词只保留"只抓高价值/结论依赖候选"的选择纪律；
  候选池是否出现在 provider 文本面由上游响应决定，不在本步控制面；
- 未收紧"最终回答必带强度 tag"（步骤 5 审计 §5 的可选项，本次不采纳，verifier
  语义不变）；
- Python 参考实现 `assurance/retrieval_subagent.py::_build_retrieval_system_prompt`
  未改动——它是项目文档检索（project-doc）契约，不含"候选 ≤5"文本；生产提示词
  权威在 Rust；
- 无 Schema/verifier/fixtures 变更：提示词不进 journal，既有 conformance
  journals 无需重捕；
- 未处理 orz-host 既有 `unused_assignments` 警告（`local_browser/mod.rs:639`，
  步骤 4 遗留，与本步无关，不扩大范围）。

## 5. 验证证据

- orz-loop：281 passed / 0 failed / 3 ignored（prompt 9/9，含更新后的契约断言）；
- orz-assurance：151 passed + 1 doctest；
- orz-host：207 passed / 4 ignored；orz-tui：178 passed；
- orz-bin：42 passed / 14 ignored（6 + 12 + 21 + 2 + 1）；
- Python：runtime/tests 244 passed；assurance/tests 1607 passed + 14 skipped
  （合计 1851 passed + 14 skipped）；
- `python scripts/check_repository.py`：valid、0 错误。

## 6. 审计结论

步骤 6（提示词相应缩短与测试更新）**已闭合**，P0-B FUS-RETRIEVAL-MECH 批次
（B-1 + 步骤 2-6）全部闭合：

- 设计与实现一致：主提示词引用纪律缩减为"标记格式 + verifier 交付前机械校验"；
- 检索提示词软约束 "候选 ≤5" 已全部移除，改指机械预算反馈（候选 N/M，剩余 K）；
- 来源加权/引用规则段落去冗余且未改变行为契约；
- 测试与门禁全绿；未宣称超出步骤 6 范围的语义。
