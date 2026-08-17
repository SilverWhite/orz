# FUS-LEDGER-FOLD-STATE 实施审计（2026-08-18）

- 状态：implemented
- 范围：orz-loop（`action_ledger.rs` / `agent_loop.rs` / `controller.rs` /
  `prompt.rs` 格式化收口）
- 关联：ADR-0010 §14.26（v1.26）；`docs/LEDGER_FOLD_STATE_CACHE_DESIGN_2026-08-18.md`；
  CLI_PROJECT_INDEX（FUS-LEDGER-FOLD-STATE）；BACKLOG 6g / TODO P1
- orz 子模块：`a5bea77`（feat/fusion-architecture）

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
- orz-loop 450 通过 / 0 失败（基线 443 + 7 新增）；orz-assurance / orz-tui
  178 / orz-bin（含 acaf_e2e 23、real_flag）全量通过；clippy 无新增可归因
  告警；`cargo fmt --all` 收口（含 prompt.rs 上批遗留换行漂移）。
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
