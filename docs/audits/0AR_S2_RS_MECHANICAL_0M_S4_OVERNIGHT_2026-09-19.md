# 过夜批实施报告：0ar S2 ＋ 0aq 机械可修项 ＋ 0m S4 收口（2026-09-19）

> 状态：**实施件全部落于工作树（未提交、未推送、未重建）**；日期 2026-09-19。
> 指令口径（用户）：「现在开一轮过夜任务……请完成 0ar S2 部分＋0aq 审查处置线的全部机械可修项＋0m GSA 会话卷 S4 收口；完成后请按照项目惯例落任务完成文档；**需明确，完成后不可提交、推送、重建**」。
> 基线：父仓 `HEAD=28e855b1`（0ar S1 提交）；orz 子仓 pin 未动（工作树脏＝0am 影批＋本批，hunk 级分离——本批改动不含 0am 的 lif/prompt 影批语义面；共享文件 `controller.rs`／`acp_server.rs` 仅不同 hunk 叠加）。
> 关联：[设计稿 v1.0](../RETRIEVAL_BATCH_HANDOFF_AND_ROUND_SEAT_DESIGN_2026-09-19.md) / [S1 报告](0AR_S1_CONTRACT_SURFACE_2026-09-19.md) / [严格审查](FULL_PROJECT_STRICT_REVIEW_2026-09-18.md) / [BACKLOG 0ar·0aq·0m](../BACKLOG_AND_PRIORITIES.md) / [TODO](../../TODO.md) / 索引 `RETRIEVAL-BATCH-HANDOFF-ROUND-SEAT`。

---

## 1. 一句话结论

三线全落地：**0ar S2 三件＋ADR-0010 §14.73/v1.75 转录**（D1 阈值回送／D2 未达标交回／D3 合并优先＋溢出拆轮；契约增量＝`query_entry` 可选 `usable_source_count`；判据 1/2/4/5/6/7 契约面与测试面锁死、判据 3 留 S3 真机）；**0aq 机械可修项 12 件勾选**（RS-03/04 部分/05 大头/07 主项/08/10/11/12/13b/13c/14/16；RS-06/09/15/17/18 带裁决门未动）；**0m S4 收口闭合（37 → 36）**。全程**零提交、零推送、零载体重建、零版本 bump**。读数：orz-loop lib **805/0/3**、orz-host 串行 **333/0/5**（含 RS-07 新测试）、orz-assurance **246/0**、orz-tui **178/0**（9 例 ACAF 缺席显式跳过）、runtime tests **366 OK**、门禁 `error_count=1`（唯一＝orz 脏树，预期态）、fixture 生成器重跑零差异、`pytest --collect-only` **2080 项零错误**（修掉存档卷误收集）。

## 2. 0ar S2 实施面

### 2.1 落点与做法

| 件 | 落点 | 内容 |
|---|---|---|
| **共享机械模块**（新） | `orz-loop/src/retrieval/batch_close.rs` | 常数（`SUFFICIENCY_TARGET=5`／`MECHANICAL_CAP=10`／`MERGE_MAX_QUERIES=3`／`DEFERRED_CAUSE`／`EARLY_DELIVERY_MARKER`）＋纯函数（`usable_source_count` 宽口径去重计数／`per_query_usable_counts`／`countdown_line` 可见倒数／`close_round_block` β 注入块／`parse_early_delivery`／`append_countdown_to_tool_message`）＋6 单测（宽口径三钉/逐 query 归因/倒数行口径/标记解析/terminal_reason 映射） |
| **D1 阈值回送** | `agent_loop.rs`＋`dispatch.rs` | post-batch 间隙武装（`LoopOutcome.retrieval_close: Option<BatchCloseKind>` 携带成因）：宽口径满 5 → β 收尾（中性事实注入＋下一轮工具面机械收空＋唯一收尾回合后 D-8 同纪律退出）；满 10 → 机械护栏强制（同 β 形态）；**可见倒数**行在两处执行点（串行 Host 分支＋并行批提交段）按 `call_id` 追加到检索族工具结果尾部（`SharedLoopServices.retrieval_calls: AtomicU64` 计发起数——放 services 层因墙钟到点路径 loop future 丢弃后 dispatch 仍需读数）；**提前交付**＝自然结束路径解析 `[EARLY_DELIVERY]` 行（指针＋可用<5＋证据在场 ⇒ `subagent_early_delivery`；缺指针 fail-open `auto_close`＋`early_delivery_missing_pointer` anomaly；≥5 时如实归 `evidence_threshold_met`；连续 ≥3 次 `early_delivery_streak`，controller 每 run 复位） |
| **D2 未达标交回** | `dispatch.rs`＋`effort.rs` | 墙钟到点臂不再 `Err(RetrievalSubagentTimeout)`——合成 `WallclockBound` outcome（`last_text` 取会话内最近非空 assistant 文本供声明行解析、`tool_rounds` 保留派发前已耗值）走**共用正常收尾路径**：部分证据报告（可用 x/5＋已发起 M 次＋blackboard section／archive 指针，exit 0）；assessment 带 `usable_source_count`＋`sufficiency_gap{target,missing,retrieval_calls,note}`（判据 1「已得计数＋缺口」）；close=`dispatch_wallclock_bound` 必带 assessment 链（schema allOf 投影）；in-flight 孤儿合成收口保持不变；档位表 **240/600/900 → 180/300/450**（env 优先级不变） |
| **D3 合并优先＋溢出拆轮** | `agent_loop.rs` 串行循环派发前 | 预扫描本轮检索派发调用：前 **3** 个合并为单激活多 query（leader 携带 `merged_queries` → goal 并 `[合并查询 n]` 行；被合并调用以合并回执交回——ToolStarted/Completed 成对＋指针文本）；溢出调用**无 ToolStarted** 拒绝（`cause=retrieval_dispatch_deferred_one_per_round`＋`stamp_failure(Refused)` 漏斗，模板同族 `refuse_inject_budget`）＋post-batch 间隙一次性重述（`[上轮检索未派发]`，每轮至多一条） |
| **契约增量** | `runtime/retrieval-result-event-payload-v0.2.schema.json` | `query_entry` 增可选 `usable_source_count`（宽口径逐 query 可用计数；按 S1「新增字段先立契约变更」先例）；新增正例 fixture `retrieval-result.merged-multi-query.valid`（生成器单一事实源＋check_repository 登记） |
| **assessment 增量** | `dispatch.rs` | `usable_source_count` 恒在（宽口径）；`sufficiency_gap` 仅在本批以低于目标的终态（到点交回／有效提前交付）结束时携带 |
| **query_summary 多 query** | `evidence.rs` | `build_structured_result` 参 `task_goal: &str` → `queries: &[String]`；单 query 形态与既有 payload **逐字节一致**（query_text=任务契约全文、result_count=全 ledger 条数）；多 query 逐条产出＋按 `search_query` 归因（派生证据多 query 下不归因——边界登记于 `per_query_usable_counts`） |

**与设计 §7 的落点差异（一处，等价）**：设计写可见倒数落 `host_exec/tool_run.rs` 结果装配面——实际落 `agent_loop.rs` 两处执行点（runner 内推消息时无 dispatch 态可读；`tool_run.rs` 无法访问证据与计数）。模型面/journal 效果等价（倒数行随工具结果文本入会话侧车），登记为落点等价偏差。

### 2.2 ADR-0010 转录

**§14.73 / v1.75**（随 S2 同批）：恢复的是 **§3.3/§4.4 的裁决权而非仪式**——检索批次以「子代理阈值回送／到点正常交回」承担 close 语义，主代理以普通回合消费回送结果并自主续派，**不要求调用 `retrieval_disposition`**；依据＝THIN_HARNESS §4.4 自述「ADR 修订留待 R3 验证通过后实施」，验证轮 r1–r3 已完成 ⇒ 裁决窗口内转录。D1/D2/D3 定案参数与契约面全部入档。

### 2.3 判据对照（设计 §8）

| 判据 | 状态 | 承载 |
|---|---|---|
| 1 close reason ∈ 新三值且 wallclock 必带计数＋缺口 | ✓ | 超时集成测试锁 `dispatch_wallclock_bound`＋assessment_id＋result_digest＋usable_source_count＋gap |
| 2 单批墙钟 ≤ 300s＋收尾回合 | ✓（机制） | 档位表 180/300/450＋到点交回臂；真机读数留 S3 |
| 3 首批 commit 后主回合数 ≥1 | 留 S3 | D3 合并/拆轮为结构前提；真机语料回放收取 |
| 4 溢出 cause＋无 ToolStarted | ✓ | 溢出集成测试锁 |
| 5 软硬不一致落 anomaly | ✓ | `early_delivery_missing_pointer`／`early_delivery_streak`／`early_delivery_at_threshold` |
| 6 query_summary 条目数＝合并 query 数且逐 query 可核 | ✓ | 合并测试锁 2 条＋`usable_source_count` 逐条在场 |
| 7 提前交付必带指针；倒数与 source_counts 同口径 | ✓ | `parse_early_delivery` 指针硬校验；倒数与 assessment 共用 `usable_source_count` 单源 helper |

### 2.4 新增测试

`batch_close` 6 单测＋集成 5 件：`threshold_close_returns_summary_when_five_usable_sources`（β 收尾轮工具面为空断言，经 FakeProvider 请求捕获）／`early_delivery_marker_parses_to_dedicated_close_reason`（正反例）／`same_round_retrieval_calls_merge_into_one_activation`／`overflow_retrieval_calls_are_deferred_without_tool_started`／`subagent_wallclock_timeout_returns_partial_evidence_normally`（重写旧超时测试为确定形态：子代理第二请求挂起 ⇒ 到点必现，usable 断言动态化消时序敏感）。既有测试改动：`subagent_budget_is_fresh_per_dispatch` 等兼容无感（payload 单 query 形态逐字节不变）。

## 3. 0aq 机械可修项处置（12 件勾选，明细以 BACKLOG 0aq 节勾选行为准）

| 项 | 处置 | 摘要 |
|---|---|---|
| RS-03 | ✅ 闭合 | 920K 模型面残留两处按 500K/T1 现行口径改写＋同串异常空格顺修 |
| RS-04 | ◐ 部分 | ①orz-tui 9 例 ignorable 化（`skip_without_acaf_signer_env`，无签发器环境 178/0 全绿）；②串行要求入 orz README；③assurance slow/e2e 分层**维持开放**（属 CI 行为变更，留下一 CI 批） |
| RS-05 | ◐ 大头 | 五热点文件 **144 处**去中毒化（`unwrap_or_else(PoisonError::into_inner)`）＋Top-10 六处（#3/#4/#6/#7/#8/#9）；余 4 处结构性 unwrap（#1/#2/#5/#10）留渐进 |
| RS-07 | ✅ 主项 | `permission_bridge_decides_every_declared_work_tool`：25 声明工具全遍历——判定全总＋ReadOnly 全放行（0aj 反向钉）＋双遍一致；余 acaf_e2e 非 Windows 缺口**处置决策**留用户裁决 |
| RS-08 | ✅ | agent_loop.rs（7,664→8,017）/controller.rs（6,274）入勘察候选 §6；登记不立项 |
| RS-10 | ✅ | 8 件 `git rm --cached`（**文件保留磁盘**；删除态已暂存随下批生效）；tracked log 归零 |
| RS-11 | ✅ | `.gsa/cargo-target`（614MB）迁 `D:\tb-eval\orz-cache\`；`installation-key.json` 确认 `secret_material_persisted_in_metadata: false`＝非可移植秘密 |
| RS-12 | ✅ | 双 README 补 `PROTOC` 构建前置 |
| RS-13b | ✅ | 索引 GAP-TOOL-BUDGET 条目补现行为注记（TER T1.7 起 0=unlimited） |
| RS-13c | ✅ | 补 `releases/orz-0.3.1-linux-x86_64/README.md` 占位（断档点有意化）；后缀两段式登记为历史约定不改 |
| RS-14 | ✅ | .gitignore 增 `/.pytest_cache/`、`/.agents/`；.gitattributes 增 `*.exe`/`*.png`/`*.ico` binary |
| RS-16 | ✅ | pyproject 死 pytest 配置删除；pytest.ini 增 `norecursedirs`（修掉存档卷误收集）；顶层 `test_*.py` 实测为零；evaluation/regression 补命名去误导 README |

**未动**（带裁决门）：RS-06（随 O2）、RS-09（删 ref 不可逆）、RS-15（归档 vs 删除）、RS-17（启动决策）、RS-18（观察挂起）。**0aq 线整体维持开放**（不动计数）。

## 4. 0m S4 收口（37 → 36）

S3 接线复验实早于 2026-09-07 经 0o T1/批次 0 达成（BACKLOG 0m S3 行同日注记）；S4＝索引/BACKLOG/TODO 状态同步，随本批落账：0m 小节头补闭合标记、S3/S4 勾选、P0 总览行与开放项清单摘除 0m、计数 **37 → 36**。索引 `AUTH-GSA-SESSION-VOLUME`（`current-design`）为设计权威不受收口影响。

## 5. 验证读数

| 项 | 命令 | 读数 |
|---|---|---|
| orz-loop lib | `cargo test -p orz-loop --lib` | **805 passed / 0 failed / 3 ignored**（基线 798＋净增 7：batch_close 6＋净新增集成） |
| orz-host 串行 | `cargo test -p orz-host --lib -- --test-threads=1` | **333 / 0 / 5**（332 基线＋RS-07 新测试 1） |
| orz-assurance lib | `cargo test -p orz-assurance --lib` | **246 / 0** |
| orz-tui | `cargo test -p orz-tui` | **178 / 0**（9 例 ACAF 环境缺席显式跳过——RS-04①） |
| fmt | `cargo fmt -p orz-loop -- --check` | 本批文件 **0 差异**（全 orz 唯一 fmt 差异＝0am 影批 `rli_shadow_replay.rs` 既有违规＝RS-06 登记面，本批未触碰） |
| clippy | `cargo clippy -p orz-loop`（PROTOC 已设） | 本批代码区（batch_close/dispatch/effort/evidence/agent_loop 新段）**零新增**；全程修复本批引入的 2 处 collapsible-if |
| runtime tests | `python -m unittest discover -s runtime/tests` | **Ran 366 tests — OK**（schema 增量后全绿） |
| fixture 生成器 | `python scripts/generate_run_event_fixtures.py` 前后逐文件 SHA256 | **零差异**（346 → 347 件：+1 merged-multi-query） |
| 门禁 | `python scripts/check_repository.py` | `error_count=1`——唯一＝`orz submodule working tree is dirty`（0am 影批＋本批未提交态，预期态）；payload 正/负 **72/51** |
| pytest 收集 | `python -m pytest --collect-only -q` | **2080 collected / 0 errors**（修复前 1 收集错误＝存档卷遗留件） |
| compileall | `python -m compileall -q assurance scripts` | exit 0 |

## 6. 边界与未做项

- **不提交、不推送、不重建**（用户硬约束）：无 commit/push、无载体重建、无版本 bump；S2 产物与 0am 影批同处 orz 脏树（hunk 级分离），RS-10 的 8 件日志删除态已进暂存区（`git rm --cached` 的机械要求，随下一提交批生效——已在报告与 BACKLOG 行登记）。
- **0ar S3 真机复验待放行**（各步独立放行纪律）；判据 3（主回合数）与五项验收读数在 S3 收取。
- RS-04③（assurance slow/e2e 分层）与 RS-05 余 4 处结构性 unwrap 维持开放（前者属 CI 行为变更、后者需结构重构）。
- RS-07 余 acaf_e2e 非 Windows 缺口处置决策、RS-06/09/15/17/18 全部留用户裁决。
- Python assurance 全套（12 分钟级）本批未跑（改动面零涉 Python assurance；runtime/契约/门禁/生成器已覆盖改动面；树态敏感的 `test_doctor_full_repository_check` 在未提交批下必红＝S1 已登记的 F8 口径）。

## 7. 复现与回放指针

```
cd orz && export PROTOC="$PWD/bin/protoc.exe"
cargo test -p orz-loop --lib && cargo test -p orz-host --lib -- --test-threads=1
cargo test -p orz-assurance --lib && cargo test -p orz-tui
python scripts\generate_run_event_fixtures.py        # 347 件零差异
python -m unittest discover -s runtime/tests         # 366 OK
python scripts\check_repository.py                   # error_count=1（唯一＝orz 脏树，预期态）
```

- 本批父仓改动面：`git status --short`（M×14＋D×8〔暂存，文件保留磁盘〕＋??×10——含 4 件摘跟踪日志、本报告、evaluation/regression README、releases 0.3.1 占位、merged-multi-query fixture、存档卷目录）；orz 侧改动面见 `git -C orz status --short`（本批件与 0am 影批 hunk 级分离）。
- 过程件（忽略态）：`.tmp-s2-*.py`／`.tmp-ledger-*.py`。
