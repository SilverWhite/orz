# 0BS 后半部分处理报告（第二轮：⑪ 收尾＋⑨⑩ 三引擎/指纹＋⑧ 降级序＋⑫ 垂直源＋⑬ 信任面；⑧ 浏览器接缝与 ⑬ 余项留档）

- 日期：2026-09-26（载体 0.6.17 已重建换装，079 档）
- 基线：orz HEAD `a8430054`（工作树含前批未提交改动；本轮**未提交/未推送/未重建**，遵嘱）
- 前档：`docs/audits/0BS_PROGRESS_2026-09-26b.md`（未竟清单）、`0BS_EXECUTION_AND_FRICTION_2026-09-26.md`、`0BS_RETRIEVAL_ROUTE_SURVEY_2026-09-25.md`
- 裁决原文：黑板 notes（本会话）＋ 本档 §五

## 一、本轮完成项

### 1. ⑪ 唯二门禁（下载/脚本逐次批准）——落码完成并验证
机制（ApprovalAlways，早期裁决）：`PermissionCommand::Request` 增 `force_prompt: bool`。

- `orz-workspace/src/permission/types.rs`：`Request` 增字段。
- `orz-workspace/src/permission/manager.rs`：`PermissionHandle::request_force_prompt()`（新）＋ `request_full(..., force_prompt)`；actor 解构该字段；`permission_mode` force⇒Ask；`reasons::APPROVAL_REQUIRED`；force 前导块（policy-deny 之后、yolo 之前）置 `auto_prompt_reason`；五处 `&& !force_prompt` 守卫（yolo 快路／会话授予／auto policy-allow／auto 分类器块／sandbox bash）；`policy_decision` force 时滤掉 `Allow`；`pre_decision` 中和 `(auto_forced_prompt || force_prompt) && auto_prompt_blocks_allow`；测试构造点补 `force_prompt: false`。
- `orz-host/src/permission.rs`：`force_prompt = tool == "browser_control" && browser_action_requires_approval(args)`（action ∈ {download, script}）；`request_fut` 分派；`yolo = is_yolo_mode() && !force_prompt`；force 的 Allow 来源记 `PermitSource::User`。
- `orz-host/src/local_browser/mod.rs`：`BrowserControlAction` 增 `Download { url }` / `Script { code }`；解析臂（download 要 url；script 非空且 ≤ 8000 字符）；caps `CONTROL_SCRIPT_MAX_CHARS=8000` / `CONTROL_SCRIPT_MAX_OUTPUT_CHARS=4000`；`BrowserControlOutcome` 增 `download: Option<BrowserDownloadInfo{path,bytes,final_url}>` / `script_output: Option<String>`；`browser_control_tool_def()` schema 重写为 **13 动作**（读/导航 7＋输入 4＋门禁 2）＋全属性与上限；信封渲染；`bounded_script_output()`（显式截断标记）。
- `orz-host/src/local_browser/cdp.rs`：dispatch 两臂；`control_download`（URL 门→`downloads_root()/uuid8`→`download_or_read`→Pdf⇒ok＋download 字段；Page⇒**状态化失败**"no download started…"；Err/超时映射）；`downloads_root()`＝`.gsa/browser-downloads-<session8>`；`control_script`（`Runtime.evaluate` returnByValue/awaitPromise/userGesture，有界；`exceptionDetails`⇒状态化失败"script threw: …"）。
- `orz-host/src/retention.rs`：`PruneReport.removed_browser_download_dirs`＋`browser-downloads-` 前缀清扫＋测试（与 `pdf-downloads-*` 同纪律）。
- 验证：`local_browser` 95 passed；`retention` 18 passed；`orz-workspace permission` 516 passed。

### 2. ⑨⑩ 三引擎＋解析器＋指纹（HTTP 车道）——落码完成并验证
- 引擎集（裁决）：`ENGINE_REGISTRY=[360search, baidu, duckduckgo, bing_cn, bing_global, google]`；**Bing 家族出默认链**（保留注册，显式链可用）。
- 默认链：直连 `DEFAULT_DIRECT_CHAIN=[360search, baidu]`；有代理 `PROXY_DEFAULT_CHAIN=[360search, baidu, duckduckgo]`；`Default` 由直连链构造。
- 解析器（按 SearXNG 对应引擎**移植**并留来源注释）：`parse_360_serp`（`li.res-list`／`h3.res-title>a`，**`data-mdurl` 优先**、`href` 回退／`p.res-desc`）；`parse_baidu_serp`（`data-tools` JSON 通道→`div.result` 锚点回退）；`parse_duckduckgo_serp`（标题锚点＋其后 4096 字符窗口取摘要——首版把可选摘要并进标题正则，leftmost-first 下"窗口空即成功"**静默跳过摘要**，fixture 当场抓到，已改两步式）；`normalize_ddg_url`（`uddg=` 离线解包）；`parse_engine_serp(id, body)` 分派。
- 指纹（裁决：sidecar 优先、如实回落）：新增 `fingerprint.rs`——curl_cffi 子进程（内嵌脚本、`impersonate="chrome"`、stdout＝`<status>\t<final_url>`、**非 2xx→None**、超时 kill、临时文件传 body）；`ENV_FINGERPRINT`（缺省 auto=开）、`ENV_PYTHON`；`python_bin()` 探测缓存；`fetch()`。
- 门控：`LocalSegmentedConfig.fingerprint: bool`（**Default=false**：单测/离线面确定性；`from_env_with` 按 env 置位）；`fetch_serp(http, engine, url, fingerprint)`。
- 测试：`config_defaults_and_env_seam`／`proxy_default_chain_leads_with_three_engines`／`proxy_switches_default_chain_and_probe_detail` 更新；新增 3 个解析器 fixture；结果 **52 passed / 0 failed**。

### 3. ⑧ 降级序＋开关语义（HTTP 车道分级）——落码
- `client.rs`：`search()` 与 `search_with_titles()` 由**整条短路**改为**链中一段**——本地 HTTP 成功即交付；失败 ⇒ `local_note`（cause/engine/waited_ms/detail）并**让渡 provider**；provider 成功则在内容前缀注记，provider 失败则 `with_details({"local_segmented_fallback": …})`。
- `ORZ_WEB_SEARCH_LOCAL` 语义：由"独占短路"改为**允许本地 HTTP 车道进入链路**（浏览器不可达时由它兜底，provider 仍为末段）。

### 4. ⑫ 垂直源引擎族——落码
- `VERTICAL_REGISTRY=[github, stackexchange, arxiv, openalex, crossref, npm]`（原生结构化 API）；`engine_template()` 统一查两张注册表（显式 `ORZ_RETRIEVAL_ENGINES` 可直接启用，**不塞默认链**——垂直源是特化检索，通用查询上会稀释命中）。
- `parse_vertical_serp()`：JSON 原生字段映射（github/npm/stackexchange/openalex/crossref）＋ arXiv Atom 正则；解析失败回空命中（如实 `empty_result` 降级，不造伪命中）。fixture 测试随 ⑨⑩ 批（52 passed）。

### 5. ⑬ 信任面（ACAF）——清单刷新补齐
- 现状核对：批七已落 `POST /api/trust`（桥 spawn `<agent trust <cwd>>`；工作区路径取桥 `state.cwd`，浏览器零路径输入）＋失败触发式授信流程（`workspace not trusted` ⇒ 确认 ⇒ 授信 ⇒ 自动重发）。
- 本轮补：授信成功后**信任清单就地刷新**（`fetchBoot()` → `state.trustedWorkspaces` → `renderExplorer()`），消除"授信后 UI 停在旧值"的空窗；boot 不可达时刷新失败不阻断重发。

### 6. f3 auto-mode 记账行
- 见 §五 裁决末行（报告记账，不改码）。

## 二、未竟（原因与下一步）

1. **浏览器 SERP 车道的 resource 接缝未装配**（⑧ 链首留白）：现状＝本地 HTTP 车道为链首、provider 末段兜底；裁决要求的"浏览器 SERP（若可用）→本地 HTTP→provider"三段的**链首**需要 `registry` 的 session-scoped resource 把宿主浏览器 SERP 能力注入工具侧，跨 crate 类型与装配点未落。下一步＝在 `orz-tools/src/registry/types.rs` 的资源缝上定义 `BrowserSerp` 能力类型 + `orz-loop`/宿主装配点注册；接入后按本档 ⑧ 的注记语义并入内容/`details`。
2. **预算单账本（跨层）**：现状账本（loop `host_exec/serp.rs` 的 `SerpSearchBudget`）已覆盖 `browser_control search` 导航；本轮 ⑨⑩ 的本地 HTTP 车道**不导航浏览器** ⇒ 无需并账（如实记账）；待浏览器 SERP 车道装配后，其导航计数应并入同一账本（与本档 1 同批）。
3. **⑬ 余项**：本轮补"授信后清单刷新"；S1 清单里的**工作区切换（B 形态）**与**单击切换/权限总览**仍待定——需要 UI 形态裁决（本次未改，因与在途 orz-web 改动重叠风险）。
4. **载体未重建**（遵嘱）：本轮改动（⑪ 已并入 0.6.17；⑨⑩/⑧/⑫/⑬ 本轮新增）**尚未**进入 Windows 载体，待下一批重建换装。
5. **推荐后续验证**（真网）：`ORZ_WEB_SEARCH_LOCAL=on` + `ORZ_RETRIEVAL_ENGINES=360search,baidu`（无 python 环境时观察指纹回落路径的 `cause` 如实性）；`ORZ_RETRIEVAL_ENGINES=github,arxiv` 走垂直源；`ORZ_RETRIEVAL_FINGERPRINT=off` 对照 A/B。

## 三、摩擦项（本轮记录，f13–f17）

- **f13 编辑面多点歧义**：`search_replace` 在"同形文本多处出现"（如测试构造点 `response`/`respond_to` 重复）时报歧义；须带更大上下文重试。建议：编辑工具在失败回执中给出**候选行号**（本会话靠回读人工定位）。
- **f14 编辑折行副作用**：一次替换把 `pub fn browser_type(...) { if … }` 折成一行（行内换行被吃掉）——以整行为锚的替换后**必须回读核版式**；已在同轮修复。建议：工具侧对"替换后行长剧烈变化"给出提示。
- **f15 压缩摘要的落点**：模型参与压缩要求把 `[SEMANTIC_SUMMARY]` 块写在**回复文本**里（写黑板不触发折叠）；本会话前两次投递落空、第三轮起才生效。建议：窗口回执中直接给出"请把摘要块放在回复文本"的示例骨架。
- **f16 大文件读取**：`cdp.rs`（4.2k 行）用**分页 read_file** 足够，未触发 read-handle 信封（0bt① 之后的分页面表现良好）。
- **f17 验证闭环**：代码先行批以 `cargo check -p <crate> --lib`（+定向 `cargo test`）为闭环判据：本轮 ⑪（95/18/516）、⑨⑩/⑫（52）全绿；**注意 PowerShell 会把 cargo 的 stderr 记成 `NativeCommandError`**，判据取 `Finished` 行而非退出码。

## 四、验证汇总（本轮跑过的全量定向测试）

| 目标 | 命令 | 结果 |
| --- | --- | --- |
| orz-host 本地浏览器（⑪ 动作面/信封/schema/截断/解析） | `cargo test -p orz-host --lib local_browser::` | **95 passed / 0 failed** |
| orz-host 保留期清扫（`browser-downloads-*`） | `cargo test -p orz-host --lib retention::` | **18 passed / 0 failed** |
| orz-workspace 权限（force_prompt 机制） | `cargo test -p orz-workspace --lib permission::` | **516 passed / 0 failed** |
| orz-tools web_search（⑨⑩ 引擎/解析器/指纹 + ⑧ 降级 + ⑫ 垂直源） | `cargo test -p orz-tools --lib web_search::` | **52 passed / 0 failed** |
| 编译面 | `cargo check -p orz-host --lib` / `-p orz-workspace --lib` / `-p orz-tools --lib` | 全部 Finished |

## 五、裁决记录（未定论部分，按建议与判断裁决）

1. **⑪ 门禁机制＝ApprovalAlways（`force_prompt`）**：`PermissionCommand::Request` 加字段；manager 跳过 yolo／会话授予／auto 分类器／policy-allow／sandbox 短路，**直落交互提示**；policy-deny 仍优先（deny > ask）；无客户端/无交互面时 fail-closed。判据＝"下载与脚本是唯二会**落地/执行**的浏览器动作"，其余读/导航/输入仍走既有风险分级。
2. **⑨⑩ 引擎集＝360search／baidu／duckduckgo（Bing 家族出默认链、保留注册）**；解析器**按 SearXNG 移植**（来源注释保留）；指纹＝**curl_cffi sidecar 优先**（本机 0.16.3 实证），不可用/非 2xx **如实回落 reqwest**；Rust `wreq` 记为目标形态（后续批），本轮不新增依赖、不动重建面。
3. **⑧ 降级序＝浏览器 SERP（可用时）→ 本地 HTTP → provider 合成**；`ORZ_WEB_SEARCH_LOCAL` 由"整条短路"改**"允许本地 HTTP 兜底"**（本轮已落其后两段与注记；浏览器链首的 resource 接缝留档 §二-1）；预算＝**单账本**（现状无需并账，见 §二-2）。
4. **⑫ 垂直源＝HTTP 车道垂直源引擎族**（github/stackexchange/arxiv/openalex/crossref/npm），**不塞默认链**（显式 `ORZ_RETRIEVAL_ENGINES` 启用；避免稀释通用查询命中）。
5. **⑬＝按 S1 要点**：核批七现状（`POST /api/trust` 已落）→ 本轮补"授信后清单刷新"；工作区切换（B 形态）与权限总览留 §二-3 待 UI 形态裁决。
6. **f3（auto-mode 记账行）＝报告记账**：auto 分类器与 force 提示的交互序已由 ⑪ 的守卫顺序固化（force 前导块先于 auto 块、`pre_decision` 中和 `auto_prompt_blocks_allow`），本轮**不改码**，仅在此记账（复核面：`manager.rs` actor 守卫序 + `gate_preflight.rs`）。

## 六、仓库状态（未提交，遵嘱）

- orz：工作树含本轮全部改动（`orz-host/src/{permission.rs,local_browser/{mod.rs,cdp.rs},retention.rs}`、`orz-workspace/src/permission/{types.rs,manager.rs}`、`orz-tools/src/implementations/web_search/{local_segmented.rs,fingerprint.rs,client.rs,mod.rs}`、`orz-web/assets/app/main.js`），**未 commit／未 push／未重建载体**。
- CLI 侧：本档 `docs/audits/0BS_PROGRESS_2026-09-26c.md`（新）。

