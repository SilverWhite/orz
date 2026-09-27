# 102 0bx 闭合——S4 agent 级真机承接读数（检索题实测＋hand-off 条件 A/B；2026-09-27）

> **日期**：2026-09-27；**用户令**：「0bx的闭合，跑一道检索题目吧」。
> **本批＝0bx 的 S4 真机承接读数与状态闭合**；**计数 55 → 54**（`GAP-BROWSER-READY-HANDOFF`，`partial` → `implemented`）。
> **形态**：父仓账本批 ＋ 两枚脚本文档件（单题执行器／读数提取器）；**未提交／未推送**；**orz 子模块零改动**（本批不动源码）。
> **载体**：0.8.1（容器 `D:/tb-eval/orz-linux/orz`＝`version=0.8.1 os=linux`；桌面 `D:/tb-eval/orz-windows/orz.exe`＝`version=0.8.1 os=windows`，sha256 `898820D6…`）。

## §0 结论速览

| 面 | 结果 |
|---|---|
| 判据（TODO `P1-0bx` 原文） | 「agent 级采样实证 `web_search` 链首**承接**（不再让渡），或如实让渡于真不可用」 |
| 容器形态检索题（0.8.1，官方 harness 单题） | **✅ 承接**——`serp-attempts` google 链条 status=ok（9 命中，首结果 7 814 ms）；启动事实 **1 条 / 15 次浏览器调用**；`browser_unavailable` **0**；reward 1.0 |
| 桌面形态・**hand-off 条件预置**（0.8.1） | **✅ 承接**——启动事实 **1 条 / 10 次浏览器调用**（含 `browser_read` 9/9 全成功）；`serp-attempts` 首轮即 google ok（6 命中，1 814 ms）；让渡痕迹 **0** |
| 同条件对照（0.8.0 载体，两轮） | **❌ 修复前形态**——启动事实 **32/32** 与 **39/39**（逐调用重复启动＝`ready()` 恒否；097 批 5/5 同一形态） |
| 判定 | **S4 达成 ⇒ 0bx 闭合（55 → 54，`partial` → `implemented`）** |

## §1 判据、口径与条件预置

### 1.1 机械读数面（先定面后读数）

1. `runs/RUN-CLI-*/serp-attempts/*.json` —— 引擎级车道归属（`lane`／`envelope.engine`／`envelope.engine_attempts[]`／`browser_type`）：链首承接＝该文件由浏览器车道产出且 `status=ok`。
2. journal `browser_launch_result` 条数 ÷ 浏览器调用条数 —— **这是 0bx 的判别式**：`ready()` 被判假时宿主逐调用重复启动（097 批 5/5 实证）；`ready()` 保持为真则仅首次启动。
3. 让渡痕迹计数（`browser_unavailable`／`local_http`）——判据要求 0，或如实让渡于真不可用。

### 1.2 两条采样线

| 线 | 执行器 | 口径 |
|---|---|---|
| 容器（官方形态） | [`run_0bx_s4_wsearch1.py`](../../scripts/run_0bx_s4_wsearch1.py)（本批新增） | 单题 `terminal-bench/count-dataset-tokens`；`-k 1 -n 1`；官方数据集 pin；`eval_browser=true`；**官方墙钟**（不设 `max_wallclock`）；**不上传**（不评正式分，沿 098 S4 先例） |
| 桌面（触发条件线） | [`dogfood_launch.ps1`](../../scripts/dogfood_launch.ps1) | `ORZ_RETRIEVAL_ENABLED=1`＋`ORZ_BROWSER_HEADLESS=1`；题面 [`.tmp-0bx-handoff-task.txt`](../../.tmp-0bx-handoff-task.txt)（检索通道措辞，沿 097 §1.4 方法学注记）；ACAF fail-closed＝默认开 |

读数提取器＝[`analyze_0bx_wsearch_readings.py`](../../scripts/analyze_0bx_wsearch_readings.py)（本批新增；引擎级＋journal 面一次成表）。

### 1.3 hand-off 条件预置（本批的关键设计）

- **动机（如实记）**：容器形态**不会进入 hand-off 条件**——当轮浏览器首启子进程存活，`is_alive()` 为真 ⇒ 0.8.0 容器轮也是「1 条启动事实 / 17 次调用」。故单看容器题**不足以区分 0.8.0 与 0.8.1**；要把读数压到修复点，必须复刻 097 的触发条件。
- **预置动作**：在 CLI profile 路径 `D:\CLI\.gsa\chrome-profile-RUN-CLI-`（`profile_dir_for(workspace, session_id)`＝`{cwd}/.gsa/chrome-profile-<session8>`，CLI 会话恒为 `RUN-CLI-`）**预置一个 headless Chromium 活实例**（`--remote-debugging-port=0 --user-data-dir=<该 profile>`，与载体自身的启动参数同形）。此后 orz 的每次 `launch` 都是**交付形态**：新进程把请求交给既有实例后立即退出。
- **机械确认（两轮一致）**：全程 `DevToolsActivePort` 内容恒为 **`64882` ＋ `/devtools/browser/315bb04f-…`**（未被重写）⇒ 本轮所有浏览器调用**都由预置的交付形态实例服务**，条件成立且有留档。（实例为探针自建，读数取毕即关；`chrome-profile-RUN-CLI-` 下已无该 profile 的持有进程。）

## §2 读数明细

### 2.1 容器形态・0.8.1（官方形态；`RUN-CLI-6ab93312`）

| 项 | 读数 |
|---|---|
| 事件／墙钟 | 617 事件；`23:12:08 → 23:25:56`（≈13m48s）；`run_finished:completed`；job reward **1.0**（1 trial／0 error） |
| 启动事实 | **1 条**（`BLAUNCH-CLI-6ab93312-0000`，`status=success`） |
| 浏览器调用 | **15 / 15 成功**；`web_search` **13 / 13 配对** |
| 链首归属 | `serp-attempts/0005.json`：`lane=external`、`engine=google`、`browser_type=headless`、`action_status=ok`、9 命中、首结果 **7 814 ms** |
| 让渡痕迹 | `browser_unavailable` **0**；`local_http` 1 处（**模型自述**的原生检索链回退，非就绪缺位——见 §4.4） |
| 同族基线（0.8.0 容器，098 批 `RUN-CLI-6ab91639`） | 启动事实 **1 / 17** 次调用；`serp-attempts` 4 条（google 全 ok，522–2 192 ms）⇒ 容器形态两载体同形（**条件未触发**，照录） |

### 2.2 桌面形态・hand-off 条件・0.8.1（`RUN-CLI-6ab9363d`）

| 项 | 读数 |
|---|---|
| 事件／终态 | 214 事件；`run_finished:completed` |
| **启动事实** | **1 条**（seq 18，`status=success`）——其后所有浏览器调用**不再重复启动** |
| 浏览器调用 | **10 / 10 成功**＝`browser_control` 1/1 ＋ **`browser_read` 9/9** |
| 链首归属 | `serp-attempts/0000.json`：`lane=external`、`engine=google`、`browser_type=headless`、`ok`、6 命中、首结果 **1 814 ms**（第 0 轮即承接） |
| 让渡痕迹 | `browser_unavailable` **0**／`local_http` **0** |
| 其它失败 | 1 处 `run_terminal_cmd exit=1`（题面外的普通命令非零退出，与检索面无关） |

### 2.3 同条件对照・0.8.0 载体（两轮）

| 轮 | 事件 | **启动事实 / 浏览器调用** | 链首归属 | 失败面（ACAF 面，见 §4.2） |
|---|---:|---|---|---|
| `RUN-CLI-6ab93831` | 451 | **32 / 32（1.00）** | `serp-attempts` 16 条（google 全 ok，654–1 343 ms） | `browser_read` 14 条 ＋ `web_fetch` 3 条＝`control_ticket_rejected:signer_unreachable`（`control_ticket_rejected` 共 27） |
| `RUN-CLI-6ab93aaa`（ACAF 件与 0.8.1 同源后再跑） | 429 | **39 / 39（1.00）** | `serp-attempts` 16 条 | `browser_read` 11 条 ＋ `web_fetch` 3 条（`control_ticket_rejected` 共 23） |

**判读**：对照轮的启动事实与 `browser_control` 调用**严格 1:1**（32/32、39/39），即宿主每次浏览器调用都重走启动路径 ⇒ `ready()` 恒否 ⇒ 这正是 097 批（5/5）与 0bx 立项所记的失效形态；而同一预置条件下 0.8.1 为 **1 / 10**。

### 2.4 差分表（四格齐备）

| 载体 | hand-off 条件 | 启动事实 / 浏览器调用 | 承接读数 |
|---|---|---|---|
| 0.8.0 | 无（容器） | 1 / 17 | 承接（条件未触发） |
| 0.8.1 | 无（容器） | 1 / 15 | **承接**（`serp-attempts` ok ＋ reward 1.0） |
| 0.8.0 | **有**（桌面预置） | **32 / 32**、**39 / 39** | 修复前形态（逐调用重复启动） |
| 0.8.1 | **有**（桌面预置） | **1 / 10** | **承接**（首轮 google ok ＋ 零让渡） |

## §3 判定

- **链首承接达成**：容器形态（官方形态）与桌面 hand-off 条件两种形态下，`web_search` 链首均由浏览器车道交付（`serp-attempts` 由浏览器车道产出且 `ok`），且**零 `browser_unavailable` 让渡**。
- **修复面命中**：在 097／0bx 的原始触发条件（同 profile 存在活实例＝交付形态）下，0.8.0 逐调用重复启动（32/32、39/39），0.8.1 仅首次启动（1/10）⇒ 就绪判定改认 CDP 端点的修复在 agent 级真机生效。
- **S4 达成 ⇒ 0bx 闭合**（`partial` → `implemented`，计数 **55 → 54**）；该闸同时放行 89 题整轮重跑（余额仍待充）。

## §4 边界与观察（如实记）

1. **容器形态不足以判别**：单跑容器检索题只能证「承接」，不能证「修复」——两载体在容器形态同为 1/17、1/15；判别力来自桌面 hand-off 条件 A/B（§1.3／§2.4）。
2. **对照轮的 ACAF 面差异（登记为观察项，仅记录、不占计数）**：对照两轮在**桌面**上，凡需控制票据的工具（`browser_read`／`web_fetch`）均被 `control_ticket_rejected: signer_unreachable` 拒绝（detail＝`io error: 管道正在被关闭。 (os error 232)`）；0.8.1 桌面轮**零拒绝**（18 张票据全消费）。换用 **0.8.1 签名器＋重 provision** 后仍复现（`RUN-CLI-6ab93aaa`）⇒ 与签名器件本身无关，疑在 **0.8.0 载体的 ACAF 客户端面**。登记 **`OBS-ACAF-SIGNER-UNREACHABLE-DESKTOP-080`（观察项，不占计数；触发器＝下次 ACAF 面改动批并入勘定）**，**留用户裁决是否立项**。
3. **该 ACAF 面不影响本批读数定性**：拒绝只落在票据类工具上、**从不产生启动事实**；重复启动与 `browser_control` 调用 1:1（32/32、39/39），而 `browser_control` 自身全成功（32/32、39/39）⇒ 判别式（启动事实 ÷ 浏览器调用）不受该面干扰。**对照轮的读取/抓取失败计数按边界排除，不作为 0bx 证据。**
4. **让渡注记的留档位置**：`[browser_serp] fell back to local_http: cause=…` 随**工具结果文本**回传，**不进 journal**；本批「让渡痕迹＝0」以 journal 字符串面计（0.8.1 两轮全 0），对照轮各 1 处为**模型自述**引用（非 journal 字段）。容器 0.8.1 轮 `local_http` 那 1 处同属模型自述（原生检索链回退），不构成就绪缺位。
5. **样本量**：容器 1 轮；桌面 0.8.1 1 轮、0.8.0 2 轮；条件为**预置**（非自然偶发）——预置胜在可复现与可判别，代价是「自然发生率」不在本读数内。
6. **未做**：89 题整轮重跑（余额 71.14 CNY 不足，按 098 档登记待充值放行）；双平台预检；打包／发行（本批无载体改动，0.8.1 已在役并已发行）。

## §5 台账同步

- **计数**：BACKLOG／TODO 计数行 **55 → 54**；BACKLOG P1 开放项清单与优先级总览表撤 `0bx`（入已闭合枚举）；TODO P1 路由行撤 `0bx`。
- **BACKLOG**：`### 0bx.` 节加 S4 达成与闭章节，状态行 `partial` → **`implemented`**；本批记录指针 → 第二卷 §1.54。
- **TODO**：`P1-0bx` 的 **S4 真机**与**闭合**两枚勾选翻 `[x]`（附读数摘要与批号）；计数行同步。
- **索引**：§6 `GAP-BROWSER-READY-HANDOFF` 条目 `partial` → **`implemented`**＋闭合法据；§8 桶由 `partial` 移入 `implemented`；头行 v4.69 → **v4.70**（v4.69 滚入 [`存档卷`](../../存档/index/CLI_PROJECT_INDEX_FULL_2026-09-24.md)，119 → **120 行**）。
- **第二卷**：§1.54（本批）。**门禁**：`python scripts/check_repository.py` ⇒ 目标 `valid: true`。
- **证据留档（门禁豁免 `.tmp-*` 与卷面）**：容器卷 `D:\tb-eval\gsa-volumes\s4-0bx-wsearch1-count-dataset-tokens\`＋作业目录 `D:\tb-eval\jobs-official\s4-0bx-wsearch1-count-dataset-tokens\`（含 `agent/trajectory.json`／`verifier/reward.txt`）；桌面卷 `D:\CLI\.gsa\runs\RUN-CLI-6ab9363d`／`6ab93831`／`6ab93aaa`；桌面日志 `.tmp-dogfood-20260927-232900.log(.live)`／`-233721`／`-234754`；对照载体目录 `D:\tb-eval\orz-windows-080`／`-080c`（0.8.0 orz.exe＋同源 ACAF 件）。

## §6 关联与关键词

[`099 就绪判定档`](099_BROWSER_READY_HANDOFF_2026-09-27.md)／[`097 尾巴闭合档`](097_TAIL_CLOSURE_AND_FLIPS_2026-09-27.md)（§1.4 采样口径＋§6 勘误）／[`098 S4 收口档`](098_0AC_S4_CLOSURE_2026-09-27.md)／[`101 发行档`](101_SUBMIT_PUSH_AND_RELEASE_2026-09-27.md)／[`cdp.rs`](../../orz/crates/orz-host/src/local_browser/cdp.rs)／[`browser_serp.rs`](../../orz/crates/orz-host/src/browser_serp.rs)／BACKLOG `0bx`／TODO `P1-0bx`。

关键词：0bx 闭合、S4 真机承接、检索题实测、hand-off 条件预置、同 profile 活实例、`DevToolsActivePort` 64882、启动事实 1/10、对照 32/32・39/39、`serp-attempts` google ok、零让渡、容器形态不判别、`OBS-ACAF-SIGNER-UNREACHABLE-DESKTOP-080`、计数 54。
