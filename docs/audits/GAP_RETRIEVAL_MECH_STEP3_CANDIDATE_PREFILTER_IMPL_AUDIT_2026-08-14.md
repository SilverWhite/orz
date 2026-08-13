# GAP-RETRIEVAL-MECH 步骤 3 实施审计（2026-08-14）

- 范围：FUS-RETRIEVAL-MECH 实施序列第 3 步——机械预筛模块（候选池净化 +
  排序，设计 §2：canonical/host 级去重、已知失败形态、相关性粗筛、
  tier/weight 排序标签进结构化结果）；本步不包含输出级引用校验器
  （步骤 5）、browser_read 范围/模式参数（步骤 4）与提示词缩短（步骤 6）。
- 设计入口：[`RETRIEVAL_MECHANICAL_CONTROLS_DESIGN_2026-08-13.md`](../RETRIEVAL_MECHANICAL_CONTROLS_DESIGN_2026-08-13.md)
- 前置：B-1（citations 结构化透传）与步骤 2（web_fetch 候选计数门禁）已闭合；
  本步直接消费 B-1 的 `candidate_urls` 候选池接缝。
- 迁移纪律：Schema/verifier 先行 → 接缝扩展 → producer → 测试/门禁全量验证。

## 1. 已变更（核心）

### 1.1 Schema/verifier（先行）

- `runtime/retrieval-result-event-payload-v0.2.schema.json`：
  - `source_entry` 与 `ref_entry` 新增可选字段 `candidate_pool`
    （`candidate_pool_entry` 数组：url/canonical_url/tier/mechanical_weight/
    weight_reason/relevance/form_reasons；tier×weight 一致性沿用 source_entry
    的 allOf 约束）；
  - payload 顶层新增可选字段 `prefilter_log`（`prefilter_entry` 数组：
    source_id/url/可选 canonical_url/reason/action/filtered_at；reason 枚举
    bad_url/login_wall/redirect_chain/duplicate_canonical/duplicate_host）；
  - 描述更新：`candidate_urls` 语义升级为“预筛后保留池”，与 candidate_pool
    成对镜像。
- `assurance/run_event_journal_validation.py`：
  - `_verify_v02_search_candidate_pool` 扩展——candidate_urls 与
    candidate_pool 必须成对出现；池长度/顺序/URL 逐项镜像；逐项校验
    canonical_url/tier/weight 一致性/weight_reason/relevance/form_reasons；
    raw_source_refs 同时镜像两字段；
  - 新增 `_verify_v02_candidate_prefilter`（注册进 cross-layer 管线）——
    `prefilter_log` 在存在候选池时必填（可为空）；每条绑定一个
    web_search_result 且携带候选池的 ledger 条目；`(source_id, url, reason)`
    去重；非 duplicate 理由移除的 URL 不得仍出现在保留池；duplicate 理由
    允许指向保留的首见副本（同一 URL 的重复副本被移除）。

### 1.2 配置种子（机器可读，不硬编码）

- 新增 `runtime/candidate-prefilter-config-v0.1.json`：tracking_query_params
  （utm_* + 常见点击 ID）、login_wall_url_markers（显式登录页标记）、
  redirect_query_keys（`url`/`next`/`goto` 等重定向器 query key）、
  redirect_url_patterns（google.com/url?、bit.ly 等重定向/短链服务）。
- 运行时覆盖通道 `ORZ_CANDIDATE_PREFILTER_CONFIG`（同来源加权的
  `ORZ_SOURCE_WEIGHTING_CONFIG` 语义：坏覆盖显式 warn 后回退内嵌种子）。

### 1.3 预筛模块（orz-assurance）

- 新增 `crates/orz-assurance/src/candidate_prefilter.rs`（lib.rs 导出）：
  - `canonicalize`：scheme/host 小写、剥 `www.`、去 fragment、剥 tracking
    参数、query 对排序重建、percent-hex 小写、根路径归一；非 http(s)/不可
    解析/空 host 返回 `None`（bad_url）。注：url crate 解析期已把显式默认
    端口归一，`port()` 对 `:443`(https) 返回 None，故默认端口不产生可观察
    原因码；
  - `detect_failure`：bad_url（解析失败/非法 scheme/空 host）、login_wall
    （种子标记）、redirect_chain（种子 query key + 重定向器 pattern）；
  - `prefilter`：失败形态移除 → canonical 去重 → host 首页去重 → 保留 →
    按 weight desc → relevance desc → 首见序排序；
  - `compute_relevance`：ASCII 词 token（len≥2）+ CJK 二元组对
    percent 解码后 URL 的词法重叠；0/1/≥2 → tangential/partial/direct；
    无关键词 → partial（中性，未知保留）。

### 1.4 orz-loop 接线

- `EvidenceRecord` 新增 `search_query: Option<String>`（仅 web_search 由
  ToolCall `query` 参数填充，供相关性粗筛使用）。
- `build_structured_result` 新增 `prefilter_config` 参数；web_search 条目
  形成 ledger 时执行预筛：
  - `candidate_urls` = 预筛后保留池（原 URL，保序）；
  - `candidate_pool` = 每候选元数据（canonical_url/tier/weight/weight_reason/
    relevance/form_reasons）；
  - `prefilter_log` = 移除记录（绑定 ledger source_id；producer 恒发，
    无移除时为空数组）；
  - `raw_source_refs` 同时镜像 `candidate_urls` 与 `candidate_pool`。
- `AgentLoopController` 新增 `candidate_prefilter` 配置字段（两个构造点 +
  `with_candidate_prefilter_config` 测试注入），真实调用点传入。

## 2. 决策登记

| ID | 决策 | 依据 |
|---|---|---|
| D-1 | 预筛在 ledger 形成时执行（`build_structured_result`），证据记录保留原始候选池 + query | B-1 接缝不变；预筛需要 query（相关性）与 weight config，且产出直接进结构化结果 |
| D-2 | `candidate_urls` 语义升级为预筛后保留池，新增 `candidate_pool`/`prefilter_log`（additive optional） | 设计 §2.3“预筛结果（候选池 + 每候选 tier/weight/形态原因）进入结构化结果，可审计”；旧 payload 仍 schema-valid |
| D-3 | canonical 化仅用于去重身份，绝不改写原 URL 展示 | 保留模型可见线索原貌；canonical_url 单独进元数据 |
| D-4 | host 级去重保守化：仅“同 host 首页”（canonical path=/）去重；同 host 不同页面保留 | 设计 §2.2“只移除明确差的”；跨页同 host 去重会误杀 |
| D-5 | 已知失败形态 v0 只实现 URL 可机械判定的三类：bad_url/login_wall/redirect_chain | 超大页与 robots 排除无法从裸 URL 离线判定；§2.3“未知保持保留” |
| D-6 | 相关性粗筛 = 词法重叠（ASCII token + CJK 二元组，percent 解码 URL），仅排序信号 | 设计 §2.2“机械词法，非语义判断”；标题在候选池不可得 |
| D-7 | 排序键 = weight desc → relevance desc → 首见序 | 来源档位是排序信号不是硬过滤（§2.2/§2.3） |
| D-8 | `prefilter_log` 放 payload 顶层（可选、producer 恒发），不进 five_fields result_digest | 与 source_counts/visibility_degraded 同级机械事实；不改变 §3.3.3 五段摘要契约 |
| D-9 | verifier 跨层规则：成对镜像、移除 URL 不得以非 duplicate 理由保留、duplicate 理由允许指向保留首见副本、`(source_id,url,reason)` 去重 | 保留“移除后可审计、不撒谎”；精确重复时首见副本保留、后见副本移除是同一 URL |

## 3. 边界（明确未做，登记给后续步骤）

- 超大页与 robots 排除未实现（离线不可判定），候选保留为 unknown；未来可
  由抓取结果/宿主信号回流后追加规则。
- web_fetch 候选计数域仍为精确字符串去重（步骤 2 硬门语义不变）；
  canonical/host 去重只作用于候选池净化，不改变抓取计数。
- 候选池的模型面展示/提示词更新属步骤 6；本步只产出结构化结果。
- CJK 相关性只覆盖 URL（含 percent 解码），不评估标题/语义（候选池无标题）。
- `prefilter_log` 不进 result_digest（five_fields 之外的 payload 机械事实，
  与 source_counts 同级）。
- DC 剩余两信号（`same_module_no_evidence` / `key_surface_unexamined`）
  仍未并入本步，建议保留到批次后续。
- browser_read 第二段计数域挂载面（步骤 4 前置裁决）不受本步影响。

## 4. 验证

- `cargo test -p orz-assurance`：**149 passed / 0 failed**（+10 预筛模块
  单测：canonical 化、去重、失败形态、相关性、排序、配置覆盖/环境覆盖、
  内嵌种子一致性）
- `cargo test -p orz-loop`：**246 passed / 0 failed / 3 ignored**（+1
  `mechanical_prefilter_shapes_candidate_pool_and_log`；既有 B-1 测试扩展
  断言 candidate_pool/prefilter_log 镜像）
- `cargo test -p orz-host`：**200 passed / 0 failed / 4 ignored**（未改动）
- `cargo test -p orz-tui`：**178 passed / 0 failed**；`cargo test -p orz-bin`
  全绿（含 orz-signer 12）
- Python `runtime/tests/test_run_event_journal_validation.py`：
  **137 passed / 0 failed**（+8 预筛契约测试）
- Python `assurance/tests` + `runtime/tests` 全量：**1838 passed /
  14 skipped**（步骤 2 基线 1828 + 10）
- `scripts/generate_run_event_fixtures.py`：重生成无 diff（可选字段不影响
  既有 fixture 形状）
- `scripts/check_repository.py`：**valid / error_count 0**（schemas 241）
- `cargo fmt --all` 与两个仓库 `git diff --check` 均 0

## 5. 审查发现（实施过程闭环）

- **P1（默认配置语义）**：初版 `CandidatePrefilterConfig` 由 derive 提供空
  `Default`，导致 `::default()`（测试路径）等于“无规则”；改为手动
  `Default = embedded()`，与 `SourceWeightConfig` 语义一致，测试暴露后修复。
- **P2（raw_source_refs 镜像遗漏）**：初版只镜像 `candidate_urls`，未镜像
  `candidate_pool`；控制器集成测试（refs[0]["candidate_pool"] == ledger 池）
  暴露，按 schema/verifier 成对镜像契约补齐。
- **P3（url crate 默认端口归一）**：`Url::port()` 对显式默认端口返回 None
  （解析期归一），`default_port_stripped` 原因码不可观察；删除该原因码并
  登记（canonical 输出不受影响）。

## 6. 仓库边界与提交顺序

- 本步横跨两个 git 仓库：父仓库 `D:\CLI`（Schema/verifier/配置种子/审计/待办）
  与嵌套仓库 `D:\CLI\orz`（Rust 实现）。
- 提交顺序：**先提交父仓库，再提交 orz 仓库**（与 B-1、步骤 2 审计一致）。
