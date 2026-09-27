# 099 浏览器会话就绪判定（hand-off 形态）立项与同批修复（0bx；2026-09-27）

> **日期**：2026-09-27；**用户令**：「本轮中应该就只暴露了这一个可修问题吧？请将这一问题正式立项并进行处理」。
> **本批＝正式立项 `0bx` 并同批落码修复**；**计数 54 → 55**（`GAP-BROWSER-READY-HANDOFF`，`partial`）。
> **形态**：父仓账本批 ＋ orz 树面改动（`crates/orz-host/src/local_browser/cdp.rs` 修复＋三枚钉子；`rollback_maintenance.rs` 测试告警一处）；**未提交／未推送／未重建载体／未发行**。
> **判据来源**：097 批观察项 `OBS-SERP-READY-HANDOFF-CROSS-ACTIVATION`（candidate，不占计数）经本批真机复核**改判机理**并升级为正式开放项。

## §0 结论速览

| 面 | 结果 |
|---|---|
| 本轮可修问题盘点 | **功能性可修项＝1 个**（即本条）；另有两处非功能性小事：①`rollback_maintenance.rs` 测试代码 `unused_must_use` 告警（094 批遗留，本批顺手清掉）；②`orz-host` 全量测试的**负载敏感族**（既有 P3 `orz-host 既有 flaky` 登记成员扩充，不新增计数） |
| 机理判定 | **改判**——失效点是**就绪判定本身**（只认「子进程存活」），**不是** 097 登记的「就绪信号跨激活不传播」 |
| 红测（修复前） | `handoff_same_profile_second_session_reports_ready` 真机实跑 **红**：同一 profile 第二个会话可正常读页面，却报 not-ready |
| 绿测（修复后） | 同一钉子 **绿**；＋端点三态钉、关停钉各绿 |
| 修复读数 | cdp 组 **37/0**；全量 `--lib` **352/358**（3 项＝负载敏感族，单独串行复跑全过）；clippy 触碰面**新增 0**；fmt 触碰面净 |
| 状态 | `partial`——源码＋钉子已落；**S3 载体进体与 S4 agent 级真机承接读数待** |

## §1 取证与勘定（问题为什么是这一个、机理为什么改判）

### 1.1 现象读数（097 采样 `RUN-CLI-6ab91021`）

| 事件序 | 事件 | 读数 |
|---:|---|---|
| 337–339 | 子代理 `browser_control{navigate}` | `browser_launch_result=success`（`browser_type=headed`，`wall_ms=211`） |
| 345–347 | `browser_read{preview}` | **又落一条** `browser_launch_result=success`（`wall_ms=237`） |
| 379–381 | 下一激活 `browser_read` | **再落一条**启动事实（`wall_ms=216`） |
| 390–392 / 402–404 | 同激活后续 `browser_read` | **各再落一条**启动事实（603 ms／1442 ms） |
| 382–383 | 同批并发 `web_search`（子代理原生车道） | `wall_ms=19120`，随后模型面自述**回退 `local_http`（cause=`browser_unavailable`）** |

**判读**：一次 run 内 **5 次浏览器调用、5 条启动事实**——即每次调用都判定「未就绪」并重新走启动路径；而同时页面读取**每次都成功**。⇒ 「未就绪」是**判定失真**，不是事实。`browser_launch_result` 只在宿主确实尝试启动时落（`finish_browser_call(launch_attempted, …)`，`orz-host/src/lib.rs`），故此读数等价于「`ready()` 当时为假」。

### 1.2 代码路径勘定（槽位共享无缺陷，问题在谓词）

- 资源缝：`OrzHost::with_credential_reader` 里 `browser` 槽（`Arc<Mutex<SharedBrowser>>`）与注入工具侧 `HostBrowserSerp` 的句柄**同一个 Arc**；`set_browser_session`／`swap_browser_session` 就地写同一槽 —— **无论跨激活（ACP 每 prompt 重建 host 后 re-inject）还是同激活（懒启动换入）都无「两个槽」缺陷**。097 的「跨激活不传播」归因由此证否。
- 链首谓词：`HostBrowserSerp::search` 先看 `browser.ready()`，`ready()`＝`LocalBrowserManager::inner` 里 `Some(session)` 且 `session.is_alive()`（`local_browser/mod.rs`），而 `is_alive()`＝**被跟踪的子进程仍在跑**（`cdp.rs`）。
- 启动参数：`browser_launch_args` 用**固定** `--user-data-dir=<profile>`，而 profile 键＝会话 id 前 8 位（一次性 CLI 会话恒为 `RUN-CLI-`）——**同 profile 复用是常态**。
- 因此：同 profile 已有实例在跑时，第二次 `launch` 的新进程把请求**交付（hand-off）**给既有实例后**立即退出**；`poll_devtools_active_port` 读到既有实例的端口文件、`/json/version` 亦应答（故"启动成功"且耗时仅 200–600 ms），但 **`child` 已退出** ⇒ `is_alive()` 恒假 ⇒ 宿主逐调用重复启动、链首恒让渡。

### 1.3 复现钉（先红，真机态）

```text
test local_browser::cdp::tests::handoff_same_profile_second_session_reports_ready ...
thread '...' panicked at crates\orz-host\src\local_browser\cdp.rs:4417:9:
0bx: hand-off session must report ready — the CDP endpoint is alive even though
the spawned hand-off process already exited
test result: FAILED. 0 passed; 1 failed
```

（钉子内含**前置断言**：第二个会话先要能正常读 `https://example.com/` 并取到 `Example Domain`——该前置**通过**，失败点只在 `is_alive()`；即「可用但被判死」当场取证。）

## §2 修复（S2 落码）

`orz/crates/orz-host/src/local_browser/cdp.rs`：

1. **端点兜底**：`is_alive()` 先在关停位与子进程两条既有判据上判定；子进程已退出时改由 `endpoint_alive()` 兜底——两道机械检查：①`<profile>/DevToolsActivePort` 仍指向**本会话端口**（挡住端口被别的 profile／进程复用）；②对该回环端口一次**短超时（250 ms）**TCP 连接成功。读数经 **TTL 1000 ms** 缓存（`ready()` 在每次浏览器调用与链首判定都会读，热路径最多每秒一次建连）。
2. **关停位压制**：`shutdown()` 先落 `shut_down` 位并清缓存——交付形态下既有实例可能仍在应答该端口，关停后不得再报存活。
3. **语义保持**：进程真死（被杀／崩溃）⇒ 端口不再应答 ⇒ 仍走既有自愈重启（D-3 语义逐字保留）；`UnavailableBrowserSession`／桩会话路径不变。

## §3 读数（本批真跑）

| 项 | 读数 |
|---|---|
| `cargo test -p orz-host --lib local_browser::cdp::tests` | **37 passed / 0 failed**（含三枚新钉） |
| 真机钉 `handoff_same_profile_second_session_reports_ready` | 修复前 **红** → 修复后 **绿**（`GSA_RUN_LIVE_BROWSER_TESTS=1`，真实 Chrome ×2 同 profile） |
| 端点三态钉 `endpoint_fallback_requires_matching_profile_port_and_live_listener` | **绿**（匹配即在活／端口不再应答即死／文件指向别端口不采纳） |
| 关停钉 `shutdown_beats_endpoint_fallback` | **绿** |
| `cargo test -p orz-host --lib`（全量） | **352/358**；3 项失败＝负载敏感族（两跑失败集合**不同**：`sweep_reaps_a_matching_orphan_and_kills_it`＋`call_tool_timeout_kills_process_tree`＋`run_tests_output_scrubbed_of_secrets` vs `call_tool_timeout_kills_process_tree`＋`call_tool_with_timeout_override_is_honored`＋`run_tests_records_workspace_delta`；**单独串行复跑全过**）⇒ 与既有 P3 `orz-host 既有 flaky` 同族，本批不新增计数 |
| `cargo clippy -p orz-host --lib --tests` | 触碰面（`cdp.rs`／`rollback_maintenance.rs`）逐文件过滤：三处告警均为**未触碰面**（`cdp.rs:1224`／`cdp.rs:2616`／`rollback_maintenance.rs:169`）⇒ **新增 0** |
| `cargo fmt -p orz-host -- --check` | 触碰面**净**；树内既有 F11 版本噪声四处（`cdp.rs:981/1513/1719/1728` 与多文件）保持不动 |
| P3 小修 | `rollback_maintenance.rs` 测试代码 `unused_must_use` 告警清零（`let _ = std::fs::create_dir_all(...)`；094 批遗留） |

## §4 边界（如实记）

1. **交付形态下既有实例不被本会话关停**：`shutdown()` 仍只杀本会话派出的子进程（交付 stub 已退出故无事可杀），profile 目录删除维持 best-effort；`chrome-profile-*` 的 A5 保留清扫面照旧。**不在本项内扩权**（关停语义涉及跨会话所有权，另立再议）。
2. **端点探活是同步短超时**（250 ms）＋TTL 缓存：热路径有界，但极端下会引入最多 250 ms 的判定延迟。
3. **本批不重建载体**：修复停在树面，**0.8.0 在役载体不含本修复**；链首承接的 agent 级读数须待 S3 载体。
4. **未跑**：89 题重跑（余额 71.14 CNY 不足，按 098 档登记待充值放行）、双平台预检、打包与发行。

## §5 台账同步

- BACKLOG：新增 `### 0bx.` 节＋计数行 **54 → 55**＋P1 开放项清单与优先级总览表（`0bx`）。
- TODO：计数行 55＋P1 路由行 ＋ `### P1-0bx` 勾选块（S1／S2 已勾，S3／S4／闭合三枚待办）。
- 索引：§6 新条目 `GAP-BROWSER-READY-HANDOFF`（`partial`）＋§8 `partial` 桶＋0bv 闭合注记改判补记＋头行 v4.67 → **v4.68**（v4.67 滚入存档卷 117 → **118 行**）。
- 第二卷：§1.52。097 档：补 §6 勘误（机理改判＋candidate 撤）。
- 机械改写脚本（门禁豁免留档）：`.tmp-b099-ledger.py`（BACKLOG／TODO）＋`.tmp-b099-ledger2.py`（索引／存档卷／第二卷／勘误），逐处断言＋行宽复核，写文件统一 LF 无 BOM。
- 门禁：`python scripts/check_repository.py` ⇒ **唯一红＝`orz submodule working tree is dirty`（预期态；落账批随提交复绿）**。

## §6 关联与关键词

[`097 尾巴闭合档`](097_TAIL_CLOSURE_AND_FLIPS_2026-09-27.md)（§1.4 机理改判＋§6 勘误）／[`098 S4 收口档`](098_0AC_S4_CLOSURE_2026-09-27.md)／[`0BV S4 真机复验`](0BV_S4_LIVE_VERIFICATION_2026-09-27.md)／[`cdp.rs`](../../orz/crates/orz-host/src/local_browser/cdp.rs)／[`browser_serp.rs`](../../orz/crates/orz-host/src/browser_serp.rs)／BACKLOG `0bx`／TODO `P1-0bx`。

关键词：就绪判定、hand-off、交付形态、`--user-data-dir` 同 profile、子进程退出、`is_alive`、`ready`、链首让渡、`browser_unavailable`、端点兜底、TTL 缓存、关停位、先红后绿、0bx 立项、计数 55。
