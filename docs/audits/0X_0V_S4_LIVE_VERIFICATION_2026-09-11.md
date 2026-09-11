# 0x / 0v S4 实机复验记录（2026-09-11）

> 入口：BACKLOG **0x**（初始轮中立问询，判据＝「开局一次、动作中不复发」）/
> BACKLOG **0v**（检索引擎 SERP，判据＝
> [`RETRIEVAL_ENGINE_SERP_SEARCH_ACTION_DESIGN_2026-09-10`](../RETRIEVAL_ENGINE_SERP_SEARCH_ACTION_DESIGN_2026-09-10.md) §4 十条）。
> 载体：**orz 0.4.1**（0x S3 重建产物 `D:/tb-eval/orz-linux/orz`，109,383,984 B，
> SHA256 `4b83de75…` = [`0X S3 重建记录`](0X_S3_DUAL_PLATFORM_REBUILD_2026-09-11.md) 锁定值）。
> 口径：单题 `terminal-bench/dna-assembly`（R4 中 `retrieval_role_write_denied`
> 实测题）、k=1、`-r 0`、**官方墙钟唯一**、`deepseek-v4-flash`、
> `eval_browser=true`、Docker 容器；**无代理直连 + 本地预拉镜像**（沿 0u R4 纪律）。
> 执行器：[`run_0x_0v_s4_live_verify.py`](../../scripts/run_0x_0v_s4_live_verify.py)；
> 分析器：[`analyze_0x_0v_s4_journal.py`](../../scripts/analyze_0x_0v_s4_journal.py)。
> 证据目录：`D:\CLI\_windows_high_nist\evidence-0x-s4-20260911\`（本地件，不入库；
> 含 `events.jsonl` 原 journal、`agent-orz.log`、`journal-analysis.txt`、`result.json`）。

## 0. 环境预检（跑批前实测）

| 探测 | 结果 | 判读 |
|---|---|---|
| 容器 → `www.google.com` | FAIL | 0v 目标环境（Google 不可用） |
| 容器 → `www.bing.com` | OK | 0v 目标环境（Bing 可用） |
| 容器 → `duckduckgo.com` | FAIL | 与 R4 同形 |
| 容器 → `api.deepseek.com` | HTTP 401（可达） | 模型端点通 |
| 容器 → `hub.harborframework.com` | HTTP 200 | 装置端通 |
| 容器代理变量 | 无 | 无代理直连成立 |
| 残留容器 / harbor 鉴权 | 无残留 / 登录态有效 | 前置门通过 |

## 1. 批次结构

- 试次：2026-09-11 12:13:10 → 12:44:45 UTC（**31m35s**，官方墙钟 1800s 耗尽）；
  官方 `AgentTimeoutError` / `reward 0.0`——**非成绩批次**，与 R4 同形态。
- journal：407 事件 / 50 模型轮 / 67 工具轮；**主车道 2 轮，外部检索子代理 48 轮**
  （主代理一轮委派后，检索子代理承完全部后续动作）。
- 准备阶段 14m（11:59–12:13）：apt 依赖 + Chromium 引导（见 §3-F2）。

## 2. 0x 判据：开局一次、动作中不复发 —— **通过**

| 判据 | 实测 | 结论 |
|---|---|---|
| 触发类型 | `trigger=initial_round` | ✅ |
| 触发次数（全会话） | **恰好 1**（seq=12）；50 模型轮内无复发 | ✅ |
| 注入位置 | `injection_position=post_tool_batch_gap` | ✅ |
| 时机 | 首个 `tool_completed` seq=9 → fire seq=12（首轮动作批次之后） | ✅ |
| 注入块 | 前缀 `[INITIAL_ROUND_INQUIRY v0.1]` 命中、三问 3/3、`completed_turns_since_orientation=1` | ✅ |
| 不落审查报告 | 41 条 `mechanical_audit_update` 无一含三问 | ✅ |
| 不落黑板锚点 | 无三问内容进入黑板写面 | ✅ |
| 无关机制不受扰 | 周期问询 0 条（主车道仅 2 轮 < 阈值 50，符合设计）；反例门与终答前报告形态未变 | ✅ |

**边界（登记）**：本轮主车道只有 2 个模型轮，故「初始轮 fire 与周期问询互不影响、互不重置」
未被实机触发（无周期 fire 样本）；该不变量由 S1/S2 测试矩阵覆盖（`orz-loop` 763 全绿）。
即 0x S4 的实机取证是**单会话样本**，判据本身全部命中且无一处反例。

## 3. 0v 判据 —— **未通过（受阻，非「机制按设计工作」）**

### 3.1 逐条判定

| # | 判据 | 实测 | 判定 |
|---|---|---|---|
| 1 | `browser_control search` 返回 Bing 有机结果 | 模型**调用 3 次**（seq 24/264/384），全部止于权限门；容器内亦无可用浏览器 | ❌ 未观察到 |
| 2 | 检索车道 navigate 不再 `retrieval_role_write_denied` | `browser_control` **0 次**车道拒绝（3 次均未到车道）；外部车道 4 次拒绝全是 `run_terminal_cmd` | ◑ 部分（见 §3.4-O1） |
| 3 | 冷却 / 上限机械生效 | 未达额度，无样本 | — 未观察到 |
| 4 | 零 400 / 命中率 ≥90% / 延迟有界 | 零 HTTP 400；命中率 **86.17%**（检索注入摊薄，与 0i 先例同族）；search 无延迟样本 | ◑ 部分 |
| 5 | CAPTCHA / 空结果识别为失败 | 无浏览器，无样本 | — 未观察到 |
| 6 | `en-US`（`setmkt=en-US`）生效 | 无浏览器，Chrome History 无记录 | — 未观察到 |
| 7 | 低质量域名加权 | 无结果面，无样本 | — 未观察到 |
| 8 | 旧 journal 回放兼容 | 非本批（由 conformance/法官族覆盖） | — 不适用 |
| 9 | 车道预算拒绝（派发前） | 未达额度 | — 未观察到 |
| 10 | 会话底线保留 | 未达阈值 | — 未观察到 |

### 3.2 F1（**框架缺陷，阻断 0v 价值**）：权限门「双面修一面」

- **现象**：3/3 次 `browser_control` 调用记录
  `permission_requested{risk: ReadOnly}` → `permission_decision{decision: deny}`（seq 25/26、
  265/266、385/386），均无 `tool_started`；orz 侧日志三次报
  `orz_workspace::permission::prompter: failed to request permission e=... "unable to send
  'session/request_permission' request, channel closed"`。
- **根因**：0v S1 只改了控制器侧分类
  （`orz-loop/src/tool.rs::risk_class` → `browser_control` 归 `ReadOnly`），
  **未同步宿主侧权限映射**（`orz-host/src/permission.rs::access_kind`，函数在 480 行，
  `browser_read` 分支在 543 行）。`browser_control` 无分支 → 落入 `else`
  的 `AccessKind::Edit(...)` → 无头部署（gateway=None）按 Edit 类询问，
  而询问通道在 `-p` 无头模式下不可达 → **fail-closed 确定性拒绝**。
  这与代码注释里已记录过两次的同形缺陷完全一致（`project_doc_index`、`browser_read`
  都曾因缺该映射被 Edit 兜底拒；review P1-1）。
- **同车道对照**：同角色、同 `risk: ReadOnly` 的 `browser_read` 5/5 得 `allow_once`
  ——因为它在 `access_kind` 里有显式 `Read(None)` 分支。差异只来自这张映射表。
- **设计—实现差距**：设计 §1.1 记「连带动面：**权限门按 LocalMutation 询问/拒绝**」，
S1 登记口径为「一处修四面：检索写门/权限门/动作分区/快照面」；实测
**「权限门」这一面没有落地**——`risk_class` 只是这一面的控制器半边，宿主半边缺席。
- **0v 前对照**：R4（0.4.0）该调用**没有权限事件**，直接 `tool_started` →
  车道拒 `retrieval_role_write_denied`。即 0v 把失败点从车道门搬到了权限门，
  功能仍未可达；外部观察到的现象相似，成因完全不同。
- **修复方向（待放行）**：在 `access_kind` 为 `browser_control` 增映射（建议按
  `action` 分档：`search`/`navigate`/`history`/`reload` → `Read(None)`；
  未知与后续 Phase 2 交互动作 → `Edit`），并补一条 headless fail-closed
  测试（同 `browser_read` / `project_doc_index` 先例）。

### 3.3 F2（**装置/环境，非 orz 缺陷**）：容器内无可用浏览器

- 装置侧 `tb_agents/orz.py` 的 Chromium 引导用 `curl --max-time 600` 取
  `chromium-browser-snapshots`：本线速约 240 KB/s，**246,542,626 B 只取回
  143,410,950 B 即超时**（`SNAPSHOT_FAIL`，console 明文记录）。
- PATH 回落命中 Ubuntu **snap 桩** `/usr/bin/chromium-browser`（执行即报
  “requires the chromium snap”）→ `browser_launch_result` **5 次全部 failure**
  （`DevToolsActivePort ... not written within 30s`）→ `browser_read` 5/5
  `browser_launch_failed`（均值 30,090 ms）。
- 后果：即便 F1 修复放行，本环境仍无浏览器可用；0v 判据 1/5/6/7 需要
  **先修引导（提高超时/预置镜像/改用容器内真实 chromium）再复跑**。

### 3.4 观察项（不影响本节判定，登记备查）

- **O1 外部检索车道对只读 shell 的拒绝**：外部车道 `run_terminal_cmd`
  21 次放行 / 4 次 `retrieval_role_write_denied`——其中 1 次是
  `curl -o e1601.pdf`（写文件，拒绝正确），另 3 次是 `ls -la` / `find`（只读）。
  只读命令被拒与其「写门」命名不一致，属 0t 车道边界话题，本批不改动。
- **O2 委派形态**：主车道只出 2 轮、48 轮在外部子代理——周期问询因此未触发
  （阈值按主车道逻辑轮计），符合设计，非缺陷。
- **O3 命中率 86.17%**：检索注入摊薄上下文缓存，与 0i 先例同族（R4 亦有三题 <90%）。

## 4. 结论

- **0x S4：判据通过**（开局一次、动作中不复发，8 项逐条命中，单会话样本边界已登记）；
  **同日用户裁决闭合入账：S1–S4 全部闭合转 `implemented`，未闭合 27 → 26**。
- **0v S4：首轮未通过**——`browser_control` 在无头实机里被权限门确定性拒绝（F1，框架缺陷）；且本环境无可用浏览器（F2，装置侧）。
  两项已按用户裁决处理（见 §5）；0v 判据 1/5/6/7 需在**载体按批次重建后**复跑取证；
  判据 2 部分成立、判据 4 部分成立。
- 本批**不产出成绩结论**（官方墙钟耗尽、reward 0.0 与本项目判据无关）。

## 5. 本批处理（2026-09-11 用户裁决）

### 5.1 F1 修复（orz `340fe4a7`）

- 落点：`orz-host/src/permission.rs::access_kind` 增 `browser_control` 分支，按**动作**
  分档——`navigate`/`back`/`forward`/`refresh`/`wait_load`/`snapshot`/`search` 七种
  现行动作全部 → `Read(None)`（与 `browser_read` 同族：URL gate 在浏览器车道、
  权限层只判「读不是写」）；未知动作与后续 Phase 2 交互动作仍落 `Edit`（fail-closed）。
- **同刀补跨表护栏**：新增测试 `read_only_tools_never_fall_into_the_edit_bucket`——
  对 11 组「控制器侧 `ToolDispatcher::risk_class` 判 ReadOnly」的代表样本（含
  `browser_control` 的 `search`/`navigate`）断言宿主侧不得落 `Edit` 兜底。
  这把「双面修一面」从**靠人记住**变成**机械可查**（本项目已第三次踩此形：
  `project_doc_index` / `browser_read` / `browser_control`）。
- 验证：`permission` 模块 20 全绿；`orz-host` lib **287 passed / 0 failed / 5 ignored**
  （单线程；首跑 1 例 `codex_app::tests::approval_allow_persists_for_identical_bash`
  超时，为既知负载 flake——单测与复跑均绿）；`cargo fmt -p orz-host --check` 干净；
  `cargo clippy -p orz-host --lib --tests` 对本次改动**零新增告警**。

### 5.2 F2 装置侧改造（用户裁决：改用容器内真实 chromium）

- 宿主侧经本机代理一次性取官方 Chromium 快照：rev `1696156`、246,549,653 B、**10.7s**
  （容器内直连同 URL 只有 ~240 KB/s，正是原 600s 上限失败的根因）；
  解压至 `D:\tb-eval\browser\chrome-linux\`（`chrome` = ELF、523,032,168 B），
  版本记录 `D:\tb-eval\browser\snapshot-rev.txt`。
- 跑批时以**只读**方式挂到 `/opt/chrome-linux`：装置既有
  `if [ -x /opt/chrome-linux/chrome ]` 判定即直接复用，**不再每次现下**。
- 容器内实测（`alexgshaw/dna-assembly:20251031`，挂载 + 装置同款依赖）：
  `/opt/chrome-linux/chrome --version` → **`Chromium 155.0.8053.0`**；headless 启动
  正常（dbus 连接噪声无害）。`--no-sandbox` 由 orz 在 headless 下自带，无需装置补参。
- **CDP 启动信号实测（关键）**：按 orz 的实际启动参数（`--headless=new
  --remote-debugging-port=0 --remote-allow-origins=* --user-data-dir=… --no-sandbox
  --window-size=1280,800 --disable-dev-shm-usage --disable-gpu
  --blink-settings=imagesEnabled=false …`）拉起，15s 内 profile 目录写出
  **`DevToolsActivePort`（端口 40571 + `/devtools/browser/<id>`）** —— 这正是 orz
  等待的信号（原失败形态为该文件 30s 未写出）。即 F2 的失败面已消除。
- 执行器 `run_0x_0v_s4_live_verify.py` 已加入该挂载（目录缺失时自动跳过、回退装置
  原有引导，不阻断跑批）。
- 归因复核：本项为**装置/环境**改造，不改变 orz 代码与设计；`D:\tb-eval\browser\`
  为本地件，不入库。

### 5.3 后续（待放行）

- orz 源码已超前于 0.4.1 载体（F1 修复在 0.4.1 构建之后落地），0v S4 复跑前需先按
  批次做双平台重建（届时 bump 版本串），再执行判据 1/5/6/7 取证。
