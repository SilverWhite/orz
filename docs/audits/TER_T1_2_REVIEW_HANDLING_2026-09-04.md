# TER T1.2 复审处理（2026-09-04）

> 上级：TODO2 T1.2（已完成）的全面复审处理。复审结论见
> [`TER_T1_2_FG_BUDGET_180S_DEFAULT_2026-09-03.md`](TER_T1_2_FG_BUDGET_180S_DEFAULT_2026-09-03.md)
> 对应的全面检查（设计合理性 / 实现合理性 / 设计与实现符合性）：总体
> 符合，无功能性偏差；发现 3 项低优先（P3）注释/文档遗留，本记录处理
> 全部 3 项。处理不改变 T1.2 行为语义（180s 单源、request 携带预算、
> 15s 后端默认退役均保持不变）。

## 1. 问题清单与处理

| # | 位置 | 问题 | 处理 |
|---|---|---|---|
| F1 | `computer/local/terminal.rs` 前台预算 tick 注释 | 仍写 “Default is 15s”，与 180s 兜底常量矛盾，15s 退役不彻底（审计 §3 只声明 mod.rs 注释干净） | 更正为 “Backend default is 180s（TER T1.2，15s 独立默认退役）”，并说明 BashParams 经 request 自带预算、后端常量只兜底直连请求 |
| F2 | `grok_build/bash/mod.rs` `effective_auto_bg_wait_ms` | 名称/文档暗示 “auto-bg wait”，但默认参数下返回值 120s——此时逐请求 auto-bg 门关闭，120s 是 kill-on-timeout 点而非 auto-bg 点；实际是 FG wait deadline | 改名 `effective_fg_wait_ms`；文档改写为 “FG wait deadline：min(默认超时, 预算)；仅当预算 < 解析超时时才是 auto-bg 点，否则为 kill 点”；5 处测试引用与注释同步；保留 2026-09-04 改名历史注记 |
| F3 | `grok_build/bash/mod.rs` `MAX_FOREGROUND_BLOCK` 文档 | “Backgroundable commands use the terminal's FOREGROUND_BLOCK_BUDGET” 措辞过时——T1.2 后 BashTool 前台请求恒携带 request 预算，终端常量只兜底直连请求 | 注释更新为 “request 预算（resident 默认 180s）+ 后端兜底” 口径 |

## 2. 验证证据

- `cargo test -p orz-tools --lib foreground_block_budget` → **18 passed /
  0 failed**（含改名后的 helper 用例与终端预算机制/常量守卫）；
- `cargo test -p orz-tools --lib bash` → **226 passed / 0 failed**
  （完整 bash 桶无回归）；
- `cargo fmt -p orz-tools -p orz-host -- --check` → 净（exit 0）；
- orz 仓库 `git diff --check` → exit 0（仅 `registry/types.rs` 既有
  CRLF 换行警告，T1.1 遗留未提交改动，非本次引入）。
- 说明：本次只改注释与一个 pub(crate) dead-code helper 的命名/文档，
  不触碰生效逻辑；orz-host 无代码改动，clippy 与全量回归仍属 T1.13。

## 3. 边界

- 不改 T1.2 既有行为：`DEFAULT_FOREGROUND_BLOCK_BUDGET_MS=180_000`
  单源、显式 `null` 回退、request 恒携带预算、schema/描述同源渲染均
  保持。
- 不改 T1.2 已登记边界：纯 BashParams 默认（120s < 180s 为
  kill-on-timeout）与 schema “300s ordinary / 600s program” 分层文案
  归 T1.4；hide_background_input 默认翻转归 T1.3。
- 旧名 `effective_auto_bg_wait_ms` 仅在改名注记中保留为历史引用，
  无代码引用残留。
