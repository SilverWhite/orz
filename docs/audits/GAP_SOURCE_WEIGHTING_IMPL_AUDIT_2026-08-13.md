# GAP-SOURCE-WEIGHTING-IMPL 实施审计（2026-08-13）

- 范围：ADR-0010 §3.7 条 12（FUS-SOURCE-WEIGHTING，v1.7）——机械来源梯队判定器、种子名单配置加载、web_search 第二层原文核验提示词、子代理模型加权标注、local_browser 直接分级加权、结构化结果 weight/tier 字段、二存一模式门控；GAP-SOURCE-WEIGHTING-IMPL（`pending` → `implemented`）
- 设计入口：[`RETRIEVAL_SOURCE_WEIGHTING_DESIGN_2026-08-12.md`](../RETRIEVAL_SOURCE_WEIGHTING_DESIGN_2026-08-12.md) / [`SOURCE_QUALITY_SEED_LISTS_2026-08-12.md`](../SOURCE_QUALITY_SEED_LISTS_2026-08-12.md)
- 迁移纪律（§5.3）：Schema/fixture/verifier 先行 → producer → 捕获重放 → 全量验证

## 1. 已变更（核心）

### 1.1 Schema/fixture/verifier（S1，先行）

- `runtime/retrieval-result-event-payload-v0.2.schema.json`：
  - `source_entry` 新增可选字段 `tier`（authoritative/default/low_quality）、
    `mechanical_weight`（0.7/1.0/1.1）、`weight_reason`、
    `model_weight`、`model_weight_reason`、`annotation_status`（adopted/annotated）；
  - `allOf` 条件约束：`web_page` 必带 tier/mechanical_weight/weight_reason；
    tier↔weight 固定配对（authoritative=1.1、default=1.0、low_quality=0.7）；
    `model_weight` 出现时必带 reason + status；
  - `organized_response` 新增可选 `source_annotations`（source_id/weight/reason/status）；
    （数组 `uniqueItems`，同一 source_id 的重复标注由 verifier/producer 双重拒绝）
  - `filter_entry.reason` 枚举 +`annotation_invalid`。
- `assurance/run_event_journal_validation.py`：新规则
  `_verify_v02_source_weighting`（注册进 v0.2 管线）——web_page 机械字段必填、
  tier/weight 固定配对、模型标注字段 all-or-none、annotated=0.7 / adopted≥1.0、
  机械 low_quality 不得 adopted、low_quality 被引用必须 annotated、
  source_annotations 与合并后的 ledger 字段机械一致。
- `scripts/generate_run_event_fixtures.py`：good fixture 补齐加权字段与
  source_annotations；新增负例 `retrieval-result.tier-weight-mismatch.constraint.invalid.json`
  （authoritative + 1.0）；`check_repository.py` 同步登记该负例。
- 真实 journal `local-browser-read.jsonl` 按新 producer 重捕（web_page 证据带
  tier=default/1.0）。

### 1.2 机械判定器 + 配置（S2）

- `orz-assurance/src/source_weighting.rs`（新模块）：
  - `SourceTier`/`SourceWeight`/`SourceWeightConfig`；固定档位常量
    1.1/1.0/0.7；
  - `classify()`：URL 形态规则 → edu.cn 个人主页标记 → 劣质平台域 →
    独立媒体域 → 白名单后缀 → 白名单显式域 → 默认；scheme 缺失自动按
    https 重试；
  - 配置加载：`embedded()`（include_str 编译默认）、`from_json_str`、
    `from_json_file`、`from_env_or_default()`（`ORZ_SOURCE_WEIGHTING_CONFIG`
    覆盖，损坏时 warn + 回退 embedded，显式不静默）。
- `runtime/source-quality-seed-lists-v0.1.json`（新）：机器可读种子名单——
  白名单后缀/显式域、劣质平台域、独立媒体域、URL 形态、个人主页标记；
  名单本体在 JSON，不在 Rust 字面量。

### 1.3 producer 接线（S3）

- `orz-loop/src/controller.rs`：
  - `AgentLoopController` 新增 `source_weighting` 字段 + 
    `with_source_weighting_config` builder（默认 from_env_or_default）；
  - `build_structured_result` 新增 weight_config 参数：web_page 证据与 URL 形态
    pdf_document 证据写 tier/mechanical_weight/weight_reason；外部 `[SOURCE]`
    声明行走同一判定器；project_doc/local_file/web_search_result 摘要条目不写
    加权字段；
  - `source_annotations` 校验与合并：合法标注写入 ledger 的 model_* 字段；
    非法标注（未知 source_id/非法 weight/缺 reason/非法 status）DROP 并写入
    filtering_log（reason=`annotation_invalid`），不降级 organized_response；
    审查修复批补强（§6）：合并规则与 Python verifier 完全一致——
    annotated 必须 0.7、adopted 必须 ≥1.0、机械 low_quality 不得 adopted、
    同一 source_id 重复标注丢弃；已合并标注回写
    `organized_response.source_annotations`；被引用的 low_quality 来源缺少合法
    annotated 标注时，整个结构化结果显式降级（visibility_degraded=true，清空
    sections/claims/model 字段），保证 committed journal 恒过 verifier。
- `orz-loop/src/agent_loop.rs` + `prompt.rs`：
  - `SystemPromptKind::Retrieval` 携带检索模式；`LoopProfile::retrieval` 接
    mode；
  - `build_retrieval_system_prompt` 增加 §3.7 条 12 合同：加权语义
    （低质量可用但必须 annotated）、framework_fallback 第二层（web_fetch
    候选 ≤5、禁 browser_read）、local_browser 直接读取（禁 web_fetch/
    web_search）、`source_annotations` 输出形状。

## 2. 决策登记

| ID | 决策 | 依据 |
|---|---|---|
| D-1 | weight/tier 字段进 source_entry + source_annotations 进 organized_response（先 Schema/fixture/verifier 再 producer） | 设计 §6.3 纪律；§3.7 条 12 结构化结果输出 |
| D-2 | `web_page` 必带机械字段；`web_search_result` 摘要条目不硬造权重 | web_search 引用 URL 当前不流入 loop（见边界 B-1）；output_text 只作线索不作证据 |
| D-3 | 种子名单落地为 `runtime/source-quality-seed-lists-v0.1.json`，二进制 include 为默认；env 覆盖=替换语义 | 设计 §6.1 “不硬编码”+“可增删”；机器合约与源码分离 |
| D-4 | 第二层候选上限 ≤5 为提示词合同，不做工具级硬门 | 设计 §6.2 “按工具轮预算校准”；120 轮预算即机械背板 |
| D-5 | 非法 source_annotation 丢弃并记 filtering_log，不降级 organized_response | v0 标注不拦截；失败必显式（非静默忽略） |
| D-6 | 模型标注字段 all-or-none；status 仅 adopted/annotated（无 “none” 第三态） | 语义最小化；absence=未标注 |
| D-7 | 二存一模式门控沿用既有 mode authority（§3.7 条 1），本切片补提示词合同与测试 | 设计 §2；不重复造门禁 |
| D-8 | 合并规则与 Python verifier 一致（annotated=0.7、adopted≥1.0、low_quality 不得 adopted、重复标注丢弃）；已合并标注回写 organized_response | 审查修复批 P5/P6：producer 不得产出 verifier 判 invalid 的 journal |
| D-9 | 被引用的 low_quality 来源无合法 annotated 标注 → 整块显式降级（清空 model 字段与 sections/claims） | 审查修复批 P7：与现有"验证失败显式降级"语义一致，不静默改写模型输出 |

## 3. 边界（明确未做，登记给后续切片）

- **B-1 web_search 引用 URL 不流入 loop**：`WebSearchOutput.citations` 存在于
  orz-tools，但 ToolResult 只携带渲染文本（query + content），controller 拿不到
  引用 URL 列表；`web_search_result` 条目因此不带 tier。后续切片把 citations
  结构化透传到 evidence（或按引用 URL 展开 ledger 条目）后补齐。
- **B-2 local_browser 主车道读取无 ledger**：`browser_read` 目前是 Host 路由的
  主车道工具，不经过检索子代理证据收集；本切片覆盖“证据进入结构化结果”的路径
  （web_fetch/声明行），browser_read 在子代理 lane 内收集时才加权。提示词已带
  local_browser 通道合同。
- **B-3 候选核验上限为软约束**：≤5 只写进提示词，不设机械拒绝；120 轮工具预算
  是硬背板。
- **B-4 配置替换语义**：env 指向的文件整体替换 embedded 名单（不是并集）；后续
  如需增量合并另设 merge 语义。
- **B-5 raw_source_refs 不新增 weight 投影**：保持既有五字段形状，weight 只在
  source_entry 上。
- **B-6 b23.tv 短链不展开**：机械层"零额外请求"约束下，`b23.tv` 整域直接 0.7
  （保守降权）；种子文档 §2.1/确认 6 已登记，展开后判定留待后续切片。

## 4. 验证

- `cargo test -p orz-assurance`：**139 passed / 0 failed**（+11 判定器单测：
  白名单后缀/显式域、劣质平台/媒体域、URL 形态、edu.cn 个人主页、默认档、
  scheme-less 重试、自定义配置、文件/env 加载）
- `cargo test -p orz-loop`：**203 passed / 0 failed / 3 ignored**（+3：
  web_page 机械加权 e2e、source_annotations 合并 e2e、检索提示词模式合同；
  审查修复批另 +2 e2e：一致性/重复丢弃 + low_quality 缺标注显式降级）
- `cargo test -p orz-host`：**199 passed / 0 failed / 4 ignored**
- `cargo test -p orz-bin`：全绿（conformance 捕获 suite 13 ignored 不在此列）；
  `capture_local_browser_read` 单独重捕通过，journal 已换新
- Python `runtime/tests/test_run_event_journal_validation.py` +
  `test_run_event_conformance.py`：**128 passed / 0 failed**（+9 加权规则测试，
  含审查修复批的重复 source_annotation 检测）
- Python `assurance/tests` 全量：**1606 passed / 14 skipped**
- `scripts/check_repository.py`：**valid / error_count 0**（schemas 240、
  v0.2 payload 正例 14 / 负例 12）
- `cargo fmt --all` 已跑；`git diff --check` 见登记命令

## 5. 三面审查闭环（2026-08-13）

实施过程自查发现并修复：

- **P1（设计闭环）web_search_result 权重来源不可得**：设计输入是引用 URL，
  但 ToolResult 不携带 citations——调整为 web_page 必带机械字段、摘要条目不
  硬造权重，并登记 B-1。
- **P2（Schema 缺陷）重复 `if` 键互相覆盖**：多条件 if/then 写成并列键，
  JSON 解析后只剩最后一个——改为 `allOf` 组合，负例 fixture 从“意外通过”
  变为精确 1 个约束错误。
- **P3（fixture 一致性）旧真实 journal 缺新字段**：`local-browser-read.jsonl`
  为旧 producer 产物，按新代码重捕替换；check_repository 由 invalid 转 valid。
- **P4（登记）环境覆盖失败语义**：`ORZ_SOURCE_WEIGHTING_CONFIG` 损坏时
  warn + 回退 embedded，显式不静默（质量层不阻断运行）。

## 6. 审查修复批（2026-08-13）

对 GAP-SOURCE-WEIGHTING-IMPL 未提交内容的全面审查发现并修复：

- **P5（producer/verifier 一致性）**：合并校验原先窄于 verifier——annotated=0.7、
  adopted≥1.0、low_quality 不得 adopted 只存在于 Python 规则；现 producer 同规则
  校验，非法标注 DROP 并记 `annotation_invalid`，新增 e2e
  `source_annotations_consistent_status_and_duplicates_enforced`。
- **P6（payload 缺口）**：producer 原先合并 model 字段到 ledger 但不回写
  `organized_response.source_annotations`，任何模型标注都会产出自验不过的 journal；
  现已合并即回写，原 e2e 同步断言回写内容。
- **P7（low_quality 引用缺标注）**：被引用 low_quality 来源无合法 annotated 标注时，
  整块按验证失败显式降级（sections/claims/model 字段清空、visibility_degraded=true），
  新增 e2e `used_low_quality_without_annotation_degrades`。
- **P8（重复标注）**：Schema `source_annotations` 数组 `uniqueItems`（精确重复），
  Python verifier 增加按 source_id 的重复检测（`duplicate source_annotation`），
  producer 丢弃重复标注；新增 Python 测试 1 条。
- **P9（b23.tv 登记）**：实现为整域 0.7、不展开短链，与种子文档原"展开后判定"表述
  不一致；已在种子文档确认 6 登记为 v0 保守降权（零额外请求约束）。
- **P10（状态语义文档化）**：adopted/annotated 的机械不变量补进设计文档 §3 第三层，
  不再只存在于 verifier/提示词。

## 7. 仓库边界与提交顺序

- 本切片横跨两个 git 仓库：父仓库 `D:\CLI`（文档/Schema/夹具/verifier/种子名单）
  与嵌套仓库 `D:\CLI\orz`（Rust 实现，父仓库 `.gitignore` 忽略 `orz/`）。
- 种子名单 `runtime/source-quality-seed-lists-v0.1.json` 位于父仓库，由
  `orz-assurance/src/source_weighting.rs` 在编译期 `include_str!` 引用
  （`CARGO_MANIFEST_DIR/../../../runtime/...`）；单独检出 orz 仓库无法编译。
- 提交顺序：**先提交父仓库（含 runtime 种子名单与审计），再提交 orz 仓库**；
  若 orz 仓库需独立可复现构建，后续可把种子名单 vendor 进 orz 或声明外部依赖
  （当前以注释形式在 `source_weighting.rs::embedded()` 处声明）。
