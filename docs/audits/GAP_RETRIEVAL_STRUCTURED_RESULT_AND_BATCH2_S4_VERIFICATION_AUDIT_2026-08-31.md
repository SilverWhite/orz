# 方向 C + 0k 第二批 S4 实机复验记录（2026-08-31）

> 日期：2026-08-31；范围：GAP-RETRIEVAL-STRUCTURED-RESULT（方向 C）与
> RETRIEVAL-ORCHESTRATION-MECHANICAL 0k 第二批（project_doc_index v2 /
> 会话级 tab 池 + 同轮多页并行 / 委托契约复杂度分档）的 S4 实机复验；
> 二进制：orz `f4f96eb8`（Linux musl 三件套，2026-08-31 重建，
> orz 106,905,752 B / signer 1,390,072 B / provision 1,207,936 B）。
> 状态：**S4 实机复验闭环**——方向 C 主体验证通过 + 3 项复验发现修复
> 后重跑全绿；0k 第二批行为实机可见。

## 1. 复验方法与任务

- 基础设施：harbor + `tb_agents.orz:Orz`（debian:bookworm-slim 任务容器，
  deepseek-v4-flash，`eval_browser=true`，容器内 apt Chromium 注入）。
- 任务集（k=1，真实二进制 + 真实容器）：
  - `count-dataset-tokens`（外部 web_search → 外部子代理 browser_read →
    evidence ledger → [SOURCE] 行 → retrieval_result_committed）；
  - `mteb-retrieve`（本地 project_doc 检索 + web 检索混合场景）。
- 三轮运行：首轮 0.2.0 发布二进制（s4-2026-08-31）暴露 2 项发现；
  b 轮修复后二进制（s4-2026-08-31b）暴露 1 项既有 schema 漂移；
  c 轮全部修复后二进制（s4-2026-08-31c）复验闭环。

## 2. 方向 C 主体验证（c 轮，count-dataset-tokens，reward 1.0）

- `retrieval_result_committed` payload 为机械四段（query_summary /
  source_ledger / filtering_log / raw_source_refs），**无
  organized_response / model_weight / annotation_status**（[RESULT_JSON]
  契约已删除，方向 C 生效）。
- `visibility_degraded=false`（full_text_observed 7 + partial 5，非空
  ledger）；source_counts.total=15；`[SOURCE]` 声明行 URL 规范化干净
  （URL 尾巴 0、空 source_title 0）。
- `retrieval_close_record` 带 `effort=extended`（0k 第二批档位实机登记
  生效）；terminal_reason=auto_close。
- 12 次 browser_read 实机执行（tab 池场景充分；含同轮多页读取）。
- 事件链 verifier：**0 错误**。

## 3. 0k 第二批行为实机观察

- **tab 池 + 同轮多页并行**：count-dataset-tokens 12 次 browser_read，
  多批并行（同轮 ≥2 读类调用并发，`parallel_read_batch` 路径）；
  二进制含 `ORZ_BROWSER_TAB_POOL_SIZE` 符号。
- **effort 档位**：close record `effort=extended` 实机落盘（档位
  映射 + schema/verifier 白名单生效）。
- **project_doc_index v2**：主面 R1 封存（`retrieve_project_docs` /
  `project_doc_index` 不可达），实机仅能验证二进制符号存在
  （`classify_retrieval_effort` / `ORZ_BROWSER_TAB_POOL_SIZE` /
  `split_source_declaration` 均在二进制内）；写后失效 / 驻留 /
  增量语义由 S2 git 夹具测试覆盖（20 项），维持既有边界登记。

## 4. 复验发现与修复（3 项）

| # | 发现 | 根因 | 修复 | 验证 |
|---|---|---|---|---|
| 1 | F11 违例：count-dataset-tokens 事件链同一 call_id 两条 tool_completed（候选门拒绝 `browser_read_candidate_cap_exceeded` + 预算拒绝 `round_inject_budget_exceeded`） | 并行批次提交阶段预算超限对已真实执行/已被 gate 拒绝并留痕的调用又补写完成事件 | `refuse_inject_budget` 增 `write_completed` 参数；并行批次传 false（只注入消息面 + deny），串行预检保持 true | orz-loop 648 passed（新增 `inject_budget_parallel_refusal_does_not_duplicate_completed_event` + 更新 P2-3 批次断言）；c 轮事件链 0 错误 |
| 2 | [SOURCE] 声明行 URL 尾巴：`source_url_or_ref` 含 `%EF%BC%88full`（全角括号说明被 percent-encode 进 URL） | `split_source_declaration` 空格分支未在「（」提前截断（URL（说明）无空格形态） | 空格候选内按全角左括号提前截断（半角 '(' 保留——维基等合法 URL 可含）；无空格分支分隔符集合补「（」 | `source_declaration_url_is_normalized` 新增用例；c 轮 URL 尾巴 0 |
| 3 | 既有 schema 漂移（b 轮暴露）：tool_completed 带 `epoch` 字段未声明；`source_title` 空串违例 | ① blackboard_read 完成事件回显 epoch，tool-completed schema 未允许；② 非 URL 声明行 title 回退空串，retrieval-result schema `nonempty` 违例 | ① `tool-completed-event-payload-v0.1.schema.json` 补可选 `epoch`（integer≥1，与工具 schema 对齐）；② `split_source_declaration` 非 URL/解析失败行 title 回退整行原文 | Python verifier 260 passed；c 轮两个 journal 均 0 错误 |

## 5. 最终复验证据（c 轮，s4-2026-08-31c）

| 项 | 结果 |
|---|---|
| 任务完成 | 2/2 reward 1.0、0 异常（count-dataset-tokens 1.0、mteb-retrieve 1.0） |
| 事件链校验 | 两 journal 均 0 错误（含 F11 receipt↔事件链同构、failure_target、policy_denial、retrieval 全量交叉） |
| 方向 C payload | 无 organized_response；visibility_degraded=false；URL 尾巴 0；空 title 0 |
| effort 档位 | close record effort=extended（实机落盘） |
| tab 池并行 | 12 次 browser_read（含同轮并行），二进制含池化符号 |
| 静态门 | orz-loop 648 passed / 0 failed；Python verifier 260 passed；clippy 无本次修改相关告警 |

## 6. 状态同步

- orz 子模块：`03072b69`（F11 + 全角括号）+ `f4f96eb8`（source_title
  回退），feat/fusion-architecture；
- GAP-RETRIEVAL-STRUCTURED-RESULT：`partial` → `implemented`（方向 C
  S4 闭环）；
- 0k 第二批三项：实机验证闭环（S3 重建 = 2026-08-31 Linux musl
  三件套；S4 实机复验 = 本记录）；
- 计数：BACKLOG 0k「S4 闭环 -1 待闭」与「实机验证待续」闭合；
  未闭合总数按 TODO 扫描快照纪律登记。
