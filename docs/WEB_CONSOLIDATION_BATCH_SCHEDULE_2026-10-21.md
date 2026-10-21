# Web 收敛批次序（332–333 与 0.9.8 收口）（2026-10-21，331 补记）

> 状态：`current-design`（计划件；不新增设计语义，只定**批次顺序与每批范围**）。
> 依据：330 契约基线（12 面＋0.9.8 必闭十条，[`330 档`](audits/330_SURFACE_CONTRACT_BASELINE_2026-10-21.md)）；331 0dk S1（[`设计稿`](SESSION_LIFECYCLE_OWNERSHIP_AND_CLOSE_DESIGN_2026-10-21.md)）；用户裁决（2026-10-21）：**Web 唯一交互面、TUI `frozen`、ACP 不降级不追 v1、close 走私有扩展、底座可更换**。
> 约束：训练进程结束前**不重建**；源码/测试按 `-j 1 --offline` 小并发执行（329 先例）。

---

## §1 总序

```text
331（已完成）0dk S1 设计＋TUI frozen＋契约页口径
  → 332（已完成 2026-10-21）会话生命周期实现（close/有界/删除不复活/占用者提示）
  → 333（已完成 2026-10-21）Web 细节（必闭 3–10，拆 333a/333b）
  → 0.9.8 双平台重建＋Windows 换装＋ACAF 重 provision
  → 0dn S2 真机十条场景
  → S4 真机收口（0dk/0di/0dm）
```

依赖理由：332 定会话骨架，333 的 UI 都挂在它上面；重建必须等训练结束；真机场景必须跑在 0.9.8 载体上。

---

## §2 331（已完成，2026-10-21）

- 范围：0dk S1 设计稿＋TUI `frozen` 标记＋12 页契约里 TUI/ACP/close 口径。
- 产出：[`0dk S1`](SESSION_LIFECYCLE_OWNERSHIP_AND_CLOSE_DESIGN_2026-10-21.md)、S2/S3/S4/S10/S11 契约页、`architecture/current/README.md` §2.5、台账。
- 读数：提交 `c109217b`；门禁 `valid: true`；计数 64 不变。

---

## §3 332 会话生命周期实现（已完成 2026-10-21）

> 读数：[`332 档`](audits/332_SESSION_LIFECYCLE_IMPLEMENTATION_2026-10-21.md)——11 条单测全过、orz-host 421/0（串行）、orz-web 63/0、smoke 44 ok；未重建。

### 3.1 范围

| # | 项 | 落点 |
|---|---|---|
| 1 | `_orz/session_close`（先 cancel、10s、幂等、busy 不半关） | `orz-host/src/session_ops.rs`（新）＋`acp_server.rs` 薄适配器 |
| 2 | 会话表有界 32＋LRU 驱逐 idle（`ORZ_SESSION_CAP`） | `acp_server.rs` sessions map |
| 3 | 删除不复活：prompt 禁隐式物化＋`.deleted` tombstone | `acp_server.rs` prompt 查找路径、delete 原语 |
| 4 | 占用者提示：桥 409 带原因＋Web 文案 | `orz-web/src/acp_pump.rs`、`assets/app/{api.js,main.js}` |
| 5 | 状态通知 `_orz/session_state`（可延到 333） | `session_ops.rs` → 桥转发 |

### 3.2 验收（10 条单测，取自 0dk S1 §12）

`close_idle_releases_lease_and_removes_entry`／`close_active_cancels_then_closes`／`close_times_out_returns_busy_and_keeps_active`／`close_is_idempotent`／`prompt_after_delete_returns_not_found_without_materializing`／`delete_active_is_rejected`／`sessions_cap_evicts_lru_idle_only`／`second_process_gets_lease_conflict`／`session_state_notification_emitted_on_transition`／`unknown_private_method_returns_method_not_found`。

加：前端 smoke 一条（关闭入口存在且运行中禁用）。

### 3.3 非目标

跨进程恢复、force kill、ACP v1、TUI 改动。

---

## §4 333 Web 细节（0.9.8 必闭 3–10；已完成 2026-10-21）

> 读数：[`333 档`](audits/333_WEB_DETAILS_MUST_CLOSE_3_TO_10_2026-10-21.md)——orz-web 68/0、orz-host 421/0、orz-bin 19/0、smoke 47 ok；doctor `ok=7 warn=1 fail=0`；未重建。333a（1–4）与 333b（5–9）同批完成，未拆提交。

| # | 细节 | 面 | 参照 | 验收 |
|---|---|---|---|---|
| 1 | 保活窗口外不恢复的显式提示 | S3 | Codex app-server 重连语义 | smoke＋文案关键词 |
| 2 | 队列剩余额度/“不跨进程”说明 | S5 | CC queue | 文案＋深度显示 |
| 3 | 审批并发可见（上一请求被取消要提示） | S8 | CC blocked 态 | 并发弹窗 smoke |
| 4 | 上下文占用/最近一次压缩读数 | S9 | CC `/context`、Codex `/status`、Gemini footer | 读数非空 |
| 5 | 后台任务列表/状态，或明确“仅模型可见” | S7 | CC 后台任务面 | 面板或文案 |
| 6 | 交付面板空态＋`terminal_source` 可见 | S10 | 无（orz 自造） | 空态文案＋字段 |
| 7 | `orz doctor`（报告式，不自动修） | S11 | Codex `doctor` | CLI 退出码＋输出节 |
| 8 | `--version`＋桥/child 版本一致性提示 | S12 | CC 最低版本提示 | 两进程版本不一致时提示 |
| 9 | 错误面四要素（`code/class/hint/evidence`＋`copy.js`） | S4/S11 | CC errors 文档 | smoke 断言关键词 |

说明：333 若超出单批容量，按 333a（1–4）／333b（5–9）拆分，顺序不变。

---

## §5 0.9.8 重建与真机

1. 重建：双平台（WIN `build_orz.ps1 -Release`；LIN docker）＋ACAF 重 provision＋身份门。
2. 换装：`C:\Users\1\AppData\Local\orz\bin` 三件套＋`.bak` 链＋`--build-info` 核证。
3. 0dn S2 十条场景：刷新重连、双标签 409、撤回后重发、删除不复活、保活过期提示、交付面板、审批并发、上下文读数、后台可见、版本偏斜。
4. S4 收口：0dk（close/evict/删除）、0di（撤回重发）、0dm（交付/自查）；读数进审计后再翻 `implemented`。

---

## §6 风险与回退

| 风险 | 处置 |
|---|---|
| 332 碰会话路径，可能扰动 329 保活 | 单测＋`--fake-provider` 端到端；保活测试全量回归 |
| close 与 recall/restore 竞态 | `closing` 状态＋三方互斥（0di 既有 `RestoreInflightGuard`） |
| 333 UI 改动引入前端死加载 | `frontend_smoke.mjs` 全量（当前 47 ok） |
| 训练期内存紧张 | 只做增量 `cargo check/test -j 1`，不跑 release、不跑 docker |

---

## §7 不做（本序内）

- 跨进程会话恢复、多写者、ACP v1 升级、TUI 解冻、自动更新、文件级撤回。
