# S10 交付与终态（surface-delivery）

> 状态：`current-design`（2026-10-21 330 批建立）。设计权威 [ADR-0010](../../../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)；设计稿 [0dm](../../../docs/TERMINAL_DECLARATION_AND_SELF_CHECK_PULL_DESIGN_2026-10-21.md)。
> 代码：`orz/crates/orz-loop/src/{model_stop.rs,prompt.rs}`、`orz/crates/orz-web/src/delivery.rs`、`assets/app/delivery.js`。
> 审计：[330 批档](../../../docs/audits/330_SURFACE_CONTRACT_BASELINE_2026-10-21.md)；归属项：0dm。

## 1. 状态机

```text
有执行事实的 turn：模型纯文本 → 无 [RUN_END] ⇒ [TERMINAL_NOTICE]（≤2 次）→ 仍无 ⇒ fail-open 记 text_without_terminal
有 [RUN_END] ⇒ 终答（剥离声明块）
short chat：豁免
self_check：模型主动调用（无参，三问）→ run_finished.self_check_called
交付状态：GET /api/delivery?session8= → 只读投影（侧车 plan/最近 run/recall）
```

## 2. 不变量

1. **声明才算终答**：有本 run 执行事实时，纯文本不是终态，必须 `[RUN_END]`。
2. **有界 nudge**：最多 2 次提示，之后 fail-open 并记录 `terminal_source`。
3. **不自欺**：机械层不判定「是否完成」、不驳回 `[RUN_END]`。
4. **只投影**：交付面板只读侧车与最近 run 事实，空闲态不重算（loop 运行时基线）。
5. **submit 退役**：交付状态不再由模型工具写入。

## 3. 失败模式

| 失败 | 业界事实 | 本地面 |
|---|---|---|
| 「完成 vs 停顿」 | 四家成熟产品**都没有**机械完成判定；靠模型自述＋提示符空闲 | 本地面自造 `[RUN_END]`，是超前而非落后 |
| 面板空值 | 无对应 | `delivery_status` 在 submit 退役后多为 null |
| 超长 journal | 无对应 | 256 KiB 尾窗可能低估 `self_check` 次数 |
| 恢复 run | 无对应 | 不识别 `RST-<s8>-*` |

## 4. 明确不做

- 机械判定任务完成/驳回终答。
- 把 `submit` 设为硬门。

## 5. 现状与差距

| 项 | 现状 | 证据 | 等级 |
|---|---|---|---|
| 终态声明 | 落码，S4 未真机 | `model_stop.rs:20-24` | 应修（验证） |
| 自查 PULL | 工具栏按钮＋TUI Ctrl+G（TUI 已 `frozen`，该键保留但不再扩展） | `state.js:29` | 接受 |
| 交付面板 | 7 单测；**333**：前端冒烟钉住空态与 `terminal_source` 原码＋中文标注 | `delivery.rs:57`／`frontend_smoke` 14.11 | 已落（真机待 S4） |
| 空态文案 | 已有「尚无可展示的交付事实」空态；333 冒烟钉住 | `delivery.js` | 已落 |
| RST 前缀 | 不识别 | 329 档 | 可延 |

## 6. 0.9.8 判决

- **必闭**：面板空态文案；前端至少一条交付/自查钉；`terminal_source` 在 UI 可见（333 已补冒烟；面板能力 329 已有）。
- **可延**：RST 前缀。
- **不做**：机械完成判定。

## 7. 测试映射

| 不变量 | 现有测试 | 缺口 |
|---|---|---|
| 1 声明才算终答 | `terminal` 系列 loop 单测（3 枚） | 真机三会话未跑（S4） |
| 2 有界 nudge | nudge episode 单测 | 2 次上限无钉 |
| 3 不自欺 | `self_check_tool_is_callable_and_recorded` | — |
| 4 只投影 | `delivery_summary_projects_sidecar_and_last_run` 等 7；333 smoke 14.11（空态＋`terminal_source`） | 无端到端（真机待 S4）；`delivery_status` 在 submit 退役后多为 null（面板主要价值是最近 run 终态/`terminal_source`/self_check） |
| 5 submit 退役 | `submit` handler 单测 | 旧 console 兼容无测试 |
