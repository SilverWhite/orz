# 286 批：检索启用门默认开——单源解析与四面接线（2026-10-17）

> 性质：**设计裁决落码批**（用户令 → ADR-0010 §14.85 → 落码＋钉子）。零跑批（本批未发起
> 任何 orz 会话）；orz 子树落码提交（**不推送**）；**未重建、未发行**——缺省开是载体面行为，
> 要在装机位生效须随下一版载体重建（装机位仍为 0.9.1）。

## §1 由来（用户令＋触发证据）

- **用户令（2026-10-17 逐字）**：「正常版本的 orz 检索门默认开即可，检索部分是很重要的
  信息获取，得常开」。
- **触发证据**（285 批档 §4，画图轮真机）：同一会话四个 run 的检索家族探针全为
  `retrieval_enabled=false`、`.gsa/activations/e689a7ea.json` 的 `activations` 为空 ⇒
  该轮检索既非本地浏览器车道、也非检索子代理 HTTP 兜底，而由**模型自驱终端原始 HTTP**
  完成（40 条网络命令；结果面 429×10／404×9／403×4；终端非零退出 13 次）。
- **旧口径**：0t（2026-09-09，ADR-0010 §14.65）把启用门定为 **fail-closed（缺省关）**，
  本批**局部取代**其缺省面（门的机制、拒绝码与 lane 语义退役结论不变）。

## §2 变更面（落码）

- **F1 单源解析**：新增 `orz-host/src/retrieval_gate.rs`——
  `resolve_retrieval_enabled(explicit, legacy)` 纯核＋`retrieval_enabled_from_env()` env 薄壳；
  优先级＝显式 `ORZ_RETRIEVAL_ENABLED`（`1`/`true`/`yes`/`on` ⇒ 开；其余显式值 ⇒ 关）>
  legacy `ORZ_RETRIEVAL_MODE`（非 `off` ⇒ 开，打印弃用提示）> **默认开**。
  `orz-bin` 原实现改为薄委托（`retrieval_enabled_from_env`），删去本地重复解析。
- **F2 四面接线**（此前各自硬编码 fail-closed 或缺失）：

| 面 | 落点 | 变更 |
|---|---|---|
| CLI/stdio/无头（含 `orz web` 桥 spawn 的 `orz --stdio`） | `orz-bin/src/main.rs`（`run_stdio_server` 入参与 `-p` 无头路径） | 随 F1 缺省开 |
| TUI 进程内面 | `orz-host/src/stdio.rs` `StdioAgentHandler::with_trust_policy` | 硬编码 `false` → 单源解析值 |
| 新会话侧车初值 | `orz-host/src/acp_server.rs` `StoredActivationSnapshot::for_session` | `false` → 单源解析值 |
| Codex app-server 集成面 | `orz-host/src/codex_app.rs`（`thread/start`） | 原经 `handle_session_new_with_policy`（内部硬编码 `false`）→ 改传单源解析值 |

- **F3 显式关保留**：`ORZ_RETRIEVAL_ENABLED=0`（推荐）或 legacy `--retrieval-mode off`；
  门机制（未启用 ⇒ 无检索工具、派发 `retrieval_not_enabled`）与 §14.65 其余结论不变。
  会话侧车 `retrieval_enabled` 字段降级为**存档读数**——每会话最终门值＝本次解析值
  （既有会话的旧侧车值一律被覆盖，故缺省开对老会话同样生效）。
- **钉子**：`retrieval_gate.rs` 三条（缺省开／显式关胜出＋legacy `off` 与显式并存优先级／
  显式开与 legacy 映射）；`orz-bin` 的 `retrieval_enabled_env_decisions` 改写（缺省断言翻面、
  补 `ORZ_RETRIEVAL_ENABLED=0` 与 `false` 两值显式关）。

## §3 不取项（如实登记）

1. **`orz-loop` 控制器结构体字面初值不动**（`controller.rs` 两处 `retrieval_enabled: false`）：
   该初值只服务「未设态」构造，生产四面一律显式注入（F2）；改动它会翻动 orz-loop 877 条
   测试里大量「禁用门路径」用例，非用户面收益，故不取。
2. **`scripts/dogfood_launch.ps1` 的 `ORZ_RETRIEVAL_ENABLED=1` 保留**：缺省开后该行为冗余
   显式开（语义无害）；不删以免动真机启动脚本面。后续若要清理，随真机脚本批一并做。

## §4 验证读数

- `cargo test -p orz-host --lib -- --test-threads=1` ⇒ **368 过／0 败**（基线 365＋本批 3 钉；
  并行跑会触发既有三例负载敏感红——017 批已记录的口径）。
- `cargo test -p orz-bin --bins` ⇒ **33 过／0 败**（含改写后的检索门解析钉；16＋2＋15）。
- `cargo test -p orz-tui --lib` ⇒ **172 过／9 败**（与 285 批同一 ACAF 预存失败集合，零新增）。
- `cargo test -p orz-web` ⇒ **47/47**；`node crates/orz-web/tests/frontend_smoke.mjs` ⇒ 全部通过。
- `cargo fmt --check`（触碰 crate）净；clippy 零新增（未触碰面既有告警照旧）。
- 门禁 `python scripts/check_repository.py` ⇒ 目标 `valid: true`。

## §5 生效路径（真机面）

- 缺省开属**载体面行为**：本批只改源码与文档，**未重建**——用户装机位（0.9.1）仍走旧
  fail-closed 缺省。下一轮真机要看到检索车道常开，须先重建新版本并换装（待用户令）。
- 生效后判据（下一真机轮）：新会话探针 `retrieval_enabled=true`／`tool_availability_check`
  的检索族三项不再列 `incomplete`；外部 lane 出现 `browser_read`／`web_search`／`web_fetch`
  调用；浏览器可用性仍以 `browser_launch_result` 事实行为准。

## §6 台账

- 本档；ADR-0010 [`§14.85`](../../adr/ADR-0010-vol-14-addenda-index.md)（v1.88）＋
  [`双车道设计 §3.1 勘注`](../RETRIEVAL_SUBAGENT_DUAL_LANE_DESIGN_2026-09-09.md)＋索引条目
  `FUS-RETRIEVAL-DUAL-LANE`（门默认口径与关键词）＋BACKLOG `0t` 勘注＋索引头行 v4.258 →
  **v4.259**＋BACKLOG 本批指针行/计数行＋TODO 计数行＋[`BACKLOG 第二卷 §1.226`](../BACKLOG_AND_PRIORITIES_2.md)。
- orz 子树落码提交（不推送）；`orz_source_manifest.sha256` 随 orz 提交重生成（orz-host 五件＋
  orz-bin 一件）。
- 零跑批；未重建、未发行；装机位保持 0.9.1；计数 62 不变。

## §7 关键词

286 批、检索启用门、缺省开、fail-closed 缺省退役、retrieval_gate、单源解析、
ORZ_RETRIEVAL_ENABLED、显式关、legacy retrieval-mode、TUI 进程内面、侧车初值、
codex app-server、画图轮证据、模型自驱 curl、收尾 0.9.2 待令、计数 62 不变、索引 v4.259。
