# FUS-LEDGER-FOLD-STATE 实施审计（2026-08-18）

- 状态：implemented
- 范围：orz-loop（`action_ledger.rs` / `agent_loop.rs` / `controller.rs` /
  `prompt.rs` 格式化收口）
- 关联：ADR-0010 §14.26（v1.26）；`docs/LEDGER_FOLD_STATE_CACHE_DESIGN_2026-08-18.md`；
  CLI_PROJECT_INDEX（FUS-LEDGER-FOLD-STATE）；BACKLOG 6g / TODO P1
- orz 子模块：`a5bea77`（feat/fusion-architecture）；**2026-08-18 审查收口
  `5274b39`**（设计 §3.5 归档实现、`ledger_fold_advance` 事件、完整性加固，
  详见 §6）

## 1. 背景与根因（折叠滑动重写前缀）

DeepSeek 控制台命中率 ≈ 67.4%（8,845,056/4,279,939），TB2 复验 66.5%——
ba86910 状态行修复（§14.25）消除了 system 摘要变化，但
`action_ledger::build_collapsed_request` 仍在**每个模型请求前无状态重算**折叠
边界（保留最近 tail=2 轮原文）：每轮请求多一个完整轮次即滑动一次、早期数万
token 的完整工具记录被整体替换为台账行，请求前缀每轮被重写（复刻模拟首次
折叠重合率 1.5%、此后 50%→84% 爬升），与 v1.9 前缀缓存纪律（§3.5 条 6）
冲突——2026-08-14 折叠定稿时尚未把「请求视图字节级前缀稳定」纳入约束。

## 2. 实施落点（S1–S4）

### S1 折叠点状态化 + 有状态视图（action_ledger.rs）

1. 新增 `LedgerFoldState`（`fold_start`/`fold_cut`/`folded_ledger` 三态 +
   不变量：两索引同 Some 或同 None、台账非空当且仅当已折叠）。
2. 新增 `build_request_view(messages, fold)`：未折叠=原文（不再做每请求无状态
   坍缩）；已折叠=preamble（`[..fold_start]`）+ 冻结台账（user 消息）+
   `messages[fold_cut..]`。
3. 新增 `advance_fold`：复用 `collapsed_cut` 的整轮配对纪律推进（全局轮次号
   延续旧台账、旧行逐字节不变、`fold_cut` 前移、`fold_start` 首推进后不变）；
   无完整轮可折叠或 `kept_start <= 当前 fold_cut` 时返回 false（防空转）。
4. 新增 `rounds_before`（压缩 drain 区域的轮次计数）。

### S2 推进触发（agent_loop.rs）

loop-top（与压缩同纪律、checkpoint 轮优先）每请求前用
`estimate_messages_tokens` 估算当前折叠视图；≥ `fold_trigger_tokens`
（默认 **128K**，`ORZ_FOLD_TRIGGER_TOKENS` 可配）时机械推进一次（零模型
调用）。推进不改变 `messages` 本体（journal/sidecar 完整记录保留）。

### S3 压缩联动 + 摘要同源 + 恢复

- `run_template_compact` 增 `fold_state` 参数：摘要输入改与主请求同一有状态
  折叠视图（不再单独无状态重算）；drain 保留起点=已折叠时的 `fold_cut`
  （不重算 tail）；压缩执行（成功与 termination 两分支）后 fold 三态重置。
- 主/检索车道 session_end 压缩（controller.rs）取 `LoopOutcome.fold_state`
  传参；折叠状态为**每轮循环实例局部**，随 `LoopOutcome` 返回——同一
  controller 被主车道与嵌套检索子代理循环共用，控制器共享字段会被子代理
  调度污染（实现偏离设计字面「controller 会话级字段」；语义等价：每次循环
  起始 fold=None 重新累积，与设计「恢复后 fold=None 重新累积」一致，恢复
  首推进重写一次前缀为接受的低频成本）。

### S4 参数接线（controller.rs）

- `ContextCompactConfig` 增 `fold_trigger_tokens`（默认 128K，env
  `ORZ_FOLD_TRIGGER_TOKENS`，正整数值；测试 seam `with_fold_trigger_tokens`）。
- 压缩普通触发 `trigger_tokens` 160K→**192K**、兜底 `safety_tokens`
  200K→**256K**（2026-08-18 用户裁决定案）；`recent_tail_rounds`/恢复/
  session_end 阈值不变。
- 注释/文档同步（口径：折叠推进=请求前估算、压缩触发=上一请求实测，均以
  折叠视图为准）。

## 3. 测试证据

- action_ledger.rs 新增 5 项单测：未折叠视图=原文；首推进折叠+视图形状；
  无新轮/轮次不足防空转；推进轮次连续+旧行不变；折叠前缀跨追加字节稳定。
- orz-loop 循环级新增 2 项：`fold_state_advances_once_and_prefix_stays_stable`
  （触发前无台账、推进后按冻结台账版本锚定的纯追加断言）；
  `fold_state_resets_after_compaction_and_summary_uses_same_view`
  （折叠先于摘要、摘要输入含冻结台账、压缩后重置并重新累积）。
- orz-loop 452 通过 / 0 失败 / 3 ignored（455 项；审查收口后实测；审计首版
  记 450 系基线口径偏差）；orz-assurance 152 / orz-tui 178 / orz-bin（含
  acaf_e2e 23、real_flag）全量通过；clippy 无新增可归因告警；`cargo fmt
  --all` 收口（含 prompt.rs 上批遗留换行漂移）。
- manifest 重生成 1401 条目、仓库门禁 `check_repository.py` valid（0 错误）。

## 4. 预期收益与实机验证

- 预期把该类 run 命中率从 ~67% 拉回 90%+（设计 §5 估算 ≈95.5%、成本约为
  现状 1/4：128K 阈值 $89/窗口 ≈ $1.18/轮）。
- 实机验证=Linux musl 重建后单题复验（make-doom），核对
  `request_header_change` 次数趋零、命中率回升；本审计不含实机验证。

## 5. 边界

- 折叠推进=每窗口一次前缀重写（128K 量级，接受；窗口内其余请求命中补齐）。
- 折叠触发与压缩触发口径并存（估算 vs 实测），设计 §4 注明、代码注释标注。
- 未折叠即压缩的罕见情形（单轮超大、无完整轮可折叠）：摘要输入=完整原文
  视图（与主请求同源），成本同主请求量级——设计接受。
- fold 状态不持久化：恢复/跨 prompt/跨车道一律 None 重新累积（§3.6）。

## 6. 二次全面审查收口（2026-08-18；orz 5274b39）

用户指示对首版实施进行全面检查（设计合理性 / 实现合理性 / 设计—实现符合性）
并处理全部审查发现。处置如下：

1. **设计 §3.5 第 1 步归档缺口（首版漏实现）**：压缩成功分支此前只做
   `fold_state.reset()`，冻结台账文本被直接丢弃（`summary_archive_markdown`
   无台账段）。收口后成功分支把 `folded_ledger` 以「折叠台账（冻结快照）」
   段追加进摘要存档——drain 后存档成为台账唯一落盘快照。终止态（摘要重试
   失败）无归档文件、不落盘，登记为接受边界。测试
   `fold_state_resets_after_compaction_and_summary_uses_same_view` 改为断言
   两次压缩归档均含冻结台账。
2. **折叠推进无事件留痕（设计层欠账）**：新增 v0.2 事件 `ledger_fold_advance`
   （`fold_start`/`fold_cut`/`rounds_folded`/`view_estimate_tokens`/
   `agent_role`），loop-top 真实推进成功才发（防空转 no-op 不发）。Schema/
   verifier/fixtures/生成器/TUI 全链同步；verifier 窗口不变量：窗口内
   `fold_start` 恒定、`fold_cut` 严格递增、`rounds_folded` 不递减，
   `context_compressed` 重置开新窗。
3. **完整性加固（首版既有边界）**：`collapsed_cut` 回退只查边界轮，中途不
   完整轮会被折叠成 `no_result` 行（正常流程不可达、防御加固）。改为
   `ranges[..collapse_count].all(is_round_complete)`，新增
   `mid_history_incomplete_round_never_folds` 单测。
4. **死代码清理**：`collapsed_round_count` 已被 `rounds_before` 取代、无
   生产调用，删除。
5. **压缩联动测试修正（暴露既有测试缺陷）**：原测试脚本 prompt_tokens
   （2K）与真实视图量级（chars/2 ≈ 5–9K）不一致，缩减守卫必然拦截首个
   rhythm 触发，`summary_response()` 被普通轮次消费、摘要实际走终止态——
   原断言只查 marker 形状故未暴露。收口后脚本 prompt_tokens 提至 20K，两次
   rhythm 触发均走成功路径（两次归档均含台账）。
6. **口径/文档修正**：设计 §4 显式区分 session_end 终局清理（全量估算，
   既有语义）与 loop-top 两口径（折叠视图）；设计 §3.1 补 per-loop local
   实施注记；审计测试计数修正 450 → 452（455 项、3 ignored）。
7. **fixture 生成器脱节修复（既有隐患）**：`generate_run_event_fixtures.py`
   的 V02 列表缺 console 三事件与身份/时间戳覆盖，重生成会删 console
   fixtures——已回填列表、payload 与 `V02_ENVELOPE_IDENTITY_OVERRIDES`；
   重生成后除新事件外仅 console-mode-transition 数组格式规范化（语义不变）。

验证：orz-loop 452 / orz-tui 178 / orz-assurance 152 / 0 失败；Python
`test_run_event_conformance` 15 / `test_run_event_journal_validation` 214 通过；
clippy 无新增可归因告警；manifest 1401、仓库门禁 valid。
