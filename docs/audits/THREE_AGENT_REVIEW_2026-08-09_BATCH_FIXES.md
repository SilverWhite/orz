# 三代理审查记录（2026-08-09 未推送批次）

> 审查范围：orz `ee6ce7f..d943596` + 主仓 `58f0965..bddfb72` + run_swebench.py。
> 三独立代理：设计合理性 / 实现合理性 / 符合性。本文件记录发现、修复与裁决。

## 1. 审查结论摘要

| 维度 | 结论 | 修复批 |
|---|---|---|
| 设计合理性 | 无设计背离；D1×1 + D2×5 + D3×5 | orz `424e62d` + 主仓本批 |
| 实现合理性 | 无 P1；P2×3 + P3×4 | 同上 |
| 符合性 | 无 C1；C2×6 + C3×9 | 同上 |

## 2. 已修复项（本轮）

| ID | 内容 | 修复 |
|---|---|---|
| D1-1/P2-2 | PHASE1_COMPLETION.md 归档引用 `architecture/archive/` 错路径 | → `../存档/architecture/pre-adr-0010/` |
| P2-1/C2-3 | breaker None 反馈把 timeout/error/whitelist 拒绝当成功重置 | PolicyFeedback 增 `Succeeded` 变体；None=中性（不重置不计数） |
| P3-1 | 成功重置测试在轮级语义下无区分度 | 重写为 3 轮差分（第 2 轮穿插成功），与纯 deny 3 轮对照 |
| C2-4 | counterexample 测试名未引用权威设计 | → `counterexample_blocks_match_adr0010_v1_1_s4_5` |
| D2-5 | D-1 prompt 缺外部来源绑定 | 主 Agent + retrieval 子代理补 URL/document identity + observed scope |
| C2-1/D2-2 | 索引 GAP-TOOL-BUDGET 状态失实 + §8 双 partial 列表 | 状态改 implemented、§8 合并纯 ID 列表 |
| P2-3 | register 缺 workspace 成员 cli-chat-proxy-types | 补条目（64→65 含 gateway 逻辑组件） |
| C2-6 | capability_group "inherited-support" 预设所有权 | → 中性名 `codegen-support`（schema 同步） |
| D2-4 | register 未覆盖 provider_client 能力群 | 新增 orz-loop-gateway 逻辑组件（code_path gateway/） |
| P3-3/C3-1 | 案例回归入口写 `regression/` 实际在 `scripts/` | 三份案例修正 |
| D2-3 | run_tests 三差距无索引召回入口 | 登记 GAP-RUN-TESTS |
| C2-2 | policy_revision 恒 0 重置路径不可触发 | 登记 GAP-DENIAL-POLICY-REVISION |

## 3. 记录裁决（不修复，登记说明）

### R-1 diagnostic_coverage schema 原地改 enum（D3-1）
`weight` enum [0, 0.5, 1.0] → [0, 1.0] 直接改在 v0.1 schema 上。
豁免依据：0.5 从未写入任何 fixture/journal（全仓 grep 无历史数据），不破坏 replay；
是 ADR-0010 §4.6.3 同日冻结裁决的执行。未来同类修正走版本化（v0.2）路径。

### R-2 轮内混合 deny key → 整轮重置（D3-3a）
一轮内多个工具同时被 deny 且 key 不同 → `all_same_key=false` → 重置（不误触发）。
ADR §3.5.4 未明说此情形；取保守语义，已注释。

### R-3 空轮（纯子代理轮）也重置 streak（P3-2）
`round_denials` 为空 → `all_same_key=false` → 重置。"连续 deny 轮"被非 deny 轮
打断而重置可辩护；旧 per-call 实现子代理轮不影响计数。语义偏差极小，注释已说明。

### R-4 check_repository 存档区链接跳过（C2-5）
147 条死链通过跳过 `存档/` 区归零而非修复。理由：改写归档原件链接会伪造
provenance（ADR §7.2）；归档快照指向冻结时路径是特性。已加注释说明。

### R-5 breaker 与 §5.3.1 迁移顺序（C3-2）
breaker 改动不新增/修改事件或 Schema（注入文本非事件），无 verifier 耦合，
"先 Schema/fixture 后 producer"不适用，豁免成立。

### R-6 旧 neutral_inquiry 生产残留（C3-3）
本次未触碰 inquiry 面；旧 INFO_SUFFICIENCY/RETRIEVAL_COMPLETION_CHECK 与
8 轮四计数仍存在，GAP-INQUIRY-SPLIT 索引如实标 partial，允许存在（Phase C 处理）。
**后续（2026-08-10）**：GAP-INQUIRY-SPLIT 已清除全部残留——`inquiry.rs`、
两个旧 block、`NeutralInquiry`/`RetrievalCompletionCheck` producer 删除
（v0.1 replay-only），索引标 implemented。见
`GAP_INQUIRY_SPLIT_IMPL_AUDIT_2026-08-10.md`。

### R-7 案例字段待补齐（D3-5）
ORZ-WIN-PROC-001~003 缺"复核结果"与"脱敏状态"字段，candidate 阶段可接受，
正式晋级前补齐。

### R-8 ee6ce7f 删除的 long_silent_tool_loop 回归点（D3-4）
删除的 output_repeats 全对话测量 bug 测试，新设计由 Runtime Stagnation Guard
（§4.5）承担；Phase C 重建测试时应保留该回归点。
**落点（2026-08-10，GAP-INQUIRY-SPLIT）**：`is_injected_block_text` 的
`[ORIENTATION` 前缀注册 + 注入块过滤测试（prompt.rs）承担该回归——注入的
orientation block 被排除在停滞测量之外，重复注入不污染 ngram 统计；per-round
输出重复测量随旧计数器删除（仅 turn 末尾守卫测量）。
