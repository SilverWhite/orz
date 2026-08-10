# local_browser（CDP 浏览器自动化）实施审计（2026-08-10）

- 范围：ADR-0010 §3.7.1（检索模式 authority 的 local_browser 档真实化）、§3.7.2（异常不静默）、§3.7.3（URL/JS/prompt-injection 边界）、§3.7.5（可见性分级）、§3.7.6（tab ownership/凭据边界）、§3.7.7（工具合同——下载大小限制）——轨道 A 后续切片（GAP-RETRIEVAL-TOOLS 审计 §4 边界第 1 项）
- 用户裁决（2026-08-10）：① MVP=纯网页读取（navigate + read_page + URL 门禁 + 内容提取 + 可见性分级；**不做** PDF/表单/任意 JS evaluate——§3.7.3 明确非 MVP）② 驱动=自动启动 headless（隔离 profile；启动失败 → Degraded 显式失败）
- 生产实现在 Rust（orz-host）；Python 先例（`assurance/browser_retrieval.py` 等）只作参考不照搬（IMPL-PYTHON-REFERENCE 边界）

## 1. 已变更（核心）

### 1.1 新模块 `orz-host/src/local_browser/`（四文件）

- **`mod.rs`**：`BrowserSession` trait（`read_page`/`ready`/`shutdown`——与 ModelGateway/LoopHost 同构的抽象，测试与 conformance 注入 fake）；`LocalBrowserManager`（tokio Mutex 持会话级 `CdpBrowserSession`——guard 跨 await 合法；`ready()` 用 try_lock 不阻塞 + **`is_alive()` 进程存活检查**（复审 M3 修复——浏览器被外部杀死后探针会重 launch，自愈覆盖进程死亡）；`UnavailableBrowserSession`（fail-closed 占位）；`browser_read_tool_def()`（单工具，参数仅 `url`）；`handle_browser_read()`（严格参数解析——**缺/空 url → `[browser_read_missing_url]`、未知参数 → `[browser_read_invalid_arguments]`（复审 M5 修复，此前缺 url 无稳定码、多余参数静默忽略）** → session → `ToolResult`，截断打机械页脚 `\n\n[browser_read content truncated: N chars, page text only]`，错误映射稳定错误码文本 `[browser_read_*]`）；`probe_launch()`（二进制发现 → launch → DevToolsActivePort 验证，失败细分 cause）；**profile 级 async 锁**（`profile_lock`——launch 与 shutdown 对同一 profile 串行化，复审 M4 修复 close_session 脱离式 teardown 与同 session 立即重开的竞态）；常量 `MAX_READ_CHARS=100_000`（对齐 Python 先例，§3.7.7 下载大小限制）；`#[ignore]` e2e ×2（`GSA_RUN_LIVE_BROWSER_TESTS=1` 门控）
- **`discovery.rs`**：`find_browser()`——`ORZ_BROWSER_PATH`（显式路径，缺失=响亮失败）→ Windows 标准路径（Chrome + Edge，PATH 不覆盖 Program Files）→ PATH 查找（手写 `find_on_path`，PATHEXT 扩展，零新依赖）；`browser_launch_args(profile_dir, headless)`（**有头默认——用户裁决 D-13（2026-08-10 复审后）：窗口可见，操作者可通过窗口手动登录（校园 SSO/期刊账号），cookie 在会话隔离 profile 内持久；`ORZ_BROWSER_HEADLESS=1` 强制 headless 供无显示环境（CI/容器）**；`--remote-debugging-port=0`（消除端口竞态）+ `--remote-allow-origins=*`（新版 Chrome ws 握手必须，仅绑 127.0.0.1）+ 隔离 flag 集）
- **`url_gate.rs`**：`check_navigation_url_sync`（纯函数：scheme 白名单 http/https、长度 ≤4096、userinfo 凭据拒绝、host 名黑名单快速路径（**四个精确串**：localhost/metadata/metadata.internal/metadata.google.internal——复审 L1 措辞修正，无 `metadata.*` 通配、无 169.254 前缀条目，域名形式的 169.254/内网由 DNS 层覆盖）、IP 字面量同步策略检查（复用 ssrf `is_blocked_for_host`）、单标签 host fail-closed）+ `check_navigation_url`（+ DNS 解析 fail-closed，复用 `check_ssrf`；**复审 H1 修复：已接线到浏览器路径**——预检门、每个顶层 `frameNavigated` 重检、`location.href` 终检三处全部走完整 DNS 门，域名形式的 SSRF（如 `169.254.169.254.nip.io`）在导航前被拒）；`UrlGateError` 七变体 → 稳定 tool error code（`browser_read_*`）
- **`cdp.rs`**：`CdpBrowserSession`——launch（spawn → `DevToolsActivePort` 轮询 30s → `/json/version` 验证）+ `read_page`（预检门（**完整 DNS 门**）→ browser ws `Target.createTarget` → page ws `Page.enable`/`Runtime.enable`/`Page.navigate` → 事件循环等 `Page.loadEventFired`（30s）+ **每个顶层 `frameNavigated` 重新过完整 URL 门**（§3.7.3 初始+每次 redirect；`about:blank` 初始帧豁免——复审 L8 修复，Chrome 可能在 Page.enable 后重放初始导航）→ **`location.href` 终检先行**（复审 L8 修复——门检通过后才提取，被拒页面不执行任何表达式）→ 三个 host 自有固定表达式（`document.title`/`document.body ? document.body.innerText : ''`/`window.location.href`——模型无法注入 JS；evaluate 异常 → 显式错误，复审 L8 修复）→ **`Target.closeTarget` 全路径执行**（复审 M6 修复——含超时/错误路径，固定 5s cap；tab 只活在单次调用内，§3.7.6）+ `shutdown`（taskkill /T /F 树杀 + profile 删除重试 2s best-effort，**profile 锁串行化**）；`ALLOWED_CDP_METHODS` 六方法 allowlist（send 边界 fail-closed + 测试锁定，无 Network/Input/Storage/Cookie 域）；`ws_reader_task`（响应→oneshot、事件→mpsc 并行路由；**通道 256 + `loadEventFired` 永不丢弃（await 投递）、其余事件满时 try_send 丢弃**——复审 L6 修复，事件洪泛不再阻塞响应路由）；`http_get_json`（TcpStream 手写 HTTP GET，**Content-Length 感知**读取——Chrome 调试端点 keep-alive 忽略 `Connection: close`，无界 read 会挂在第二次读（e2e 实测 2026-08-10）；ConnectionReset 当 EOF（Windows 10054）；5s 超时兜底；**状态行 200 校验 + 非法 Content-Length 显式报错**——复审 L8 修复）；`CdpConfig`（load_timeout/total_budget 可注入）；`is_alive()` 用 `try_wait`（tokio 缓存 reap 状态，`shutdown` 的 wait 仍返回缓存值）

### 1.2 接线（orz-host）

- `lib.rs`：`OrzHost.browser: SharedBrowser`（默认 `UnavailableBrowserSession` fail-closed）+ `with_browser_session`/`set_browser_session`（声明随 readiness——探针与声明同源）+ `browser_ready()` + `call_tool` 特判 `browser_read`
- `tools.rs`：`ToolsetRegistry.browser_ready` 标志 + `get/list` 条件声明（未 ready 时模型永不见该工具）
- `permission.rs`：`access_kind("browser_read") → Read(None)`（否则落入 Edit 分支被 scope 拒绝——project_doc_index 先例的同类缺陷）
- `acp_server.rs`：`probe_retrieval_capability` async 化 + host 注入——LocalBrowser 分支真实探测（已有句柄 → Available；无 → `probe_launch` 成功注入 Available / 失败 `Degraded("browser_launch_failed: <cause>")`）；`StoredSession.browser`（跨 run 存活，同 profile 无锁冲突）；`close_session` 触发 `browser.shutdown()`（detached）
- `retention.rs`：A5 清扫新增 `chrome-profile-*` 前缀目录（孤儿 profile 兜底），`PruneReport.removed_browser_profiles`
- `Cargo.toml`：+ tokio-tungstenite / url / futures（均已在 workspace lock 图，零新增传递依赖）

### 1.3 接线（orz-loop）

- `tool.rs`：`risk_class("browser_read") → ReadOnly`（纯读；模式门在 controller 管可达性）
- `relay.rs`：`is_retrieval_mode_gated_host_tool` += `browser_read`（off 投影隐藏 + dispatch 门禁覆盖）
- `controller.rs`：`build_evidence_record` 增 `browser_read` 分支（页脚 marker/长度 backstop → `web_page`+`partial_text_observed`，否则 `full_text_observed`；**source_type 复用 `web_page`**——传输差异在 evidence.tool，不加 schema 枚举值）；`run_host_tool` 防呆门——非 local_browser 模式拒绝 `retrieval_mode_requires_local_browser`（无 ToolStarted，与 off 门同形态）

### 1.4 schema/verifier

- **schema 改动 1 处（复审 L3 修复）**：`retrieval-mode-transition` payload 的非 off `then` 子句补 `"required": ["capability_status"]`——此前 allOf 只约束"存在时的值"、不要求存在，缺失时 schema 通过，verifier 是唯一防线（原"schema allOf 双保险"表述过强，已修正为"值约束由 schema、存在性由 schema+verifier 双保险"）；result 因 web_page 复用不动
- `assurance/run_event_journal_validation.py` `_verify_v02_retrieval_mode`：local_browser 分支按 transition payload 的 `capability_status` 分支——`available` → 成功 dispatch + committed result 合法；`unsupported`/`degraded` → 保持严格规则（必须显式失败、禁 committed result）；缺失/非法 → 新错误（schema allOf 值约束 + verifier 存在性双保险）
- **host 车道检查（复审 M2 修复）**：`browser_read` 的 tool 事件无 `target` 字段（D-2 host 车道），target 基规则看不到它——off 分支与非 available 分支新增按工具名的检查（`_HOST_LANE_RETRIEVAL_TOOLS={"browser_read"}`；web_fetch/web_search 走 external 车道带 target，已被既有规则覆盖）

### 1.5 conformance capture

- 新场景 `capture_local_browser_read`（main.rs）：`StubBrowserSession` fake 车道（**不触真实浏览器**）+ `with_retrieval_mode(LocalBrowser, Available, pending=true)` + FakeProvider 脚本（web_search → browser_read → [RESULT_JSON] → 完成×2）；断言 transition `capability_status=="available"`、committed `source_counts.full_text_observed==1`、`visibility_degraded==false`、ledger `source_type=="web_page"`/`visibility=="full_text_observed"`；22 事件序列锁
- `local-browser-capability`（unsupported）场景**保留为回归**（unsupported 仍是合法 capability 态）；13 个 journals 全量重捕 + dev-copy + 三处 README/期望序列表同步（fixtures README / `generate_run_event_fixtures.py` / `check_repository.py` 精确集合 +1 / pytest 期望序列表）

## 2. 已删除

- （无删除——本切片为增量；GAP-RETRIEVAL-TOOLS 的 `local_browser_automation_not_implemented` unsupported 路径被真实探测替换，`local-browser-capability` capture 保留为手工构造回归）

## 3. 决策登记

| ID | 决策 | 依据 |
|---|---|---|
| D-1 | 单工具 `browser_read`（不拆 navigate/read 状态化） | MVP 单调用内完成 create→navigate→read→close，模型永不持有 tab 句柄（§3.7.6 平凡满足）；状态化留给后续切片 |
| D-2 | `browser_read` 走 Host 车道（不进 external dispatch 车道） | 浏览器是会话级状态属 host；子代理工具投影保留 host 工具 → 子代理可用 + evidence 照常入账 |
| D-3 | 探针=真实 launch（session-bootstrap 一次性；已 ready 直接 Available；失败每次 prompt 重试） | 用户裁决"自动启动 headless"；自愈（浏览器/环境后可用）；off/framework_fallback 会话零成本 |
| D-4 | 启动失败 → `Degraded("browser_launch_failed: <细分 cause>")` | §3.7.2 显式失败无静默降级；细分 cause 可行动（browser_not_found 列搜索路径） |
| D-5 | `Runtime.evaluate` 仅 host 内置固定表达式 | §3.7.3 任意 JS 非 MVP；模型不可注入（Python broker allowlist 经验） |
| D-6 | source_type 复用 `web_page`（不加 `local_browser_page`） | 传输层差异由 evidence.tool="browser_read" 记录；新增值迫使 schema/verifier/fixtures 联动无机械收益 |
| D-7 | 读取上限 100_000 字符 + 机械页脚 | §3.7.7 下载大小限制；页脚触发 evidence partial 降级（web_fetch 先例同构） |
| D-8 | 空 innerText → `EmptyContent` 显式错误 | §3.7.2 无静默降级；MVP 不产出 metadata_only（保留给状态化切片） |
| D-9 | http_get_json Content-Length 感知读取 | Chrome 调试端点 keep-alive 忽略 Connection: close（e2e 实测）；无界 read 挂第二次读 |
| D-10 | profile 删除 best-effort + 2s 重试 | Windows 文件锁异步释放；孤儿目录由 A5 清扫兜底 |
| D-11 | `--remote-debugging-port=0` + DevToolsActivePort 轮询 | 消除端口竞态（Chrome 写端口+ws 路径文件） |
| D-12 | 不注册进 global_process_scope | 工具超时 kill_active 会误杀浏览器（会话级生命周期独立管理） |
| D-13 | **有头默认**（`ORZ_BROWSER_HEADLESS=1` 回退 headless） | 用户裁决（2026-08-10 复审后）：① 找论文场景需校园网认证——IP 认证与 headless 无关，但登录类站点（SSO/期刊账号）headless 下无法手动登录（表单非 MVP）② 路径上反爬设备对 headless TLS 指纹返回挑战页（e2e 实测 example.com），有头正常指纹可避 ③ 窗口可见=透明可监督，安全姿态与 headless 同构（隔离 profile、cookie 不透出、tab 单次调用） |

## 4. 边界（明确未做，登记给后续切片）

- PDF 下载/校验/索引管线（旧 LBR 文档 §10 布局；§3.7.2 保留 INVALID_PDF/NO_TEXT_LAYER 异常语义）
- 表单填写/点击/滚动/任意 JS evaluate（§3.7.3 明确非 MVP）；LOGIN_REQUIRED/CAPTCHA 分类（MVP 空文本显式报错兜底）
- **账户/支付/密码管理面拒绝（§3.7.3"未移交的账户/支付/密码管理面默认拒绝"）——复审 M1 登记**：MVP 无会话状态、无移交面，门禁未做 URL 关键词启发式（login/payment/account 类路径），预填字段值可能随 innerText 返回（password 字段为掩码不透出）。启发式误伤面（如公开文档页带 login 路径）与收益不匹配，留后续切片裁决；LOGIN_REQUIRED 语义由空文本显式报错兜底。**D-13 后有头窗口：操作者可手动登录（这属于用户侧交互，不是模型表单填写），登录 cookie 存于会话隔离 profile，模型仍不可见 cookie**
- **模型侧表单填写/点击/任意 JS evaluate 仍非 MVP**（§3.7.3；D-13 后手动登录不改变此边界——登录是操作者通过可见窗口完成的，不经工具）
- 状态化 tab（navigate/read 分离）、多 tab 并发、browser 句柄跨进程重启恢复（sidecar 不存浏览器状态）；有头窗口每次调用闪现 tab 的 UX 留状态化切片
- web_search=1 全局 semaphore（§3.7.7 工具合同另一项）、站点速率限制、凭据注册表接线——轨道 A 其他切片
- 探针失败后的手动重试路径（当前每 prompt 自动重试）
- 已知残余：orz 硬崩溃时孤儿 Chrome 进程（无 listener 兜底；profile 由 A5 清扫）；`ORZ_BROWSER_PATH` 显式缺失=Degraded（用户可修正后重试）

## 5. 验证

- `cargo test -p orz-tools`：全绿（web_fetch ssrf 含新 `blocks_ipv4_embedded_forms`；2708 基线 + 1）
- `cargo test -p orz-host`：**141 passed / 2 ignored**（136 基线 + 复审修复批新增 5：cdp final-URL 门先行 / about:blank 豁免 / http 状态行 / http 非法 CL / mod 未知参数拒绝；+2 ignored e2e）
- `cargo test -p orz-loop`：**160 passed / 3 ignored**（本切片零改动）
- `cargo test -p orz-assurance -p orz-tui -p orz-bin`：95 / 177 / 6+13 ignored 全绿
- conformance capture **13/13**（新增 local-browser-read 22 事件；12 旧场景序列零漂移）；journals 全量重捕 dev-copy，fixtures 精确集合 13
- Python：`test_run_event_journal_validation.py` **89 passed**（原 86 含 5 新规则测试 + 复审新增 3：degraded host 车道成功拒绝 / degraded host 车道显式失败合法 / off host 车道派发拒绝；**现有 unsupported 测试为 3 个**——审计初稿"4 个"计数失实已修正）、`test_run_event_conformance.py` 14 passed、assurance 全套 **1790 passed / 13 skipped**（1787 + 3）
- `check_repository.py` **valid**（schemas 235、v0.2 journals 13 精确集合）
- **e2e（真实浏览器，env 门控）**：`GSA_RUN_LIVE_BROWSER_TESTS=1 cargo test -p orz-host -- --ignored local_browser_e2e`——门禁 e2e **通过**（`169.254.169.254.nip.io` 域名形式在导航前被 DNS 层拒绝；判定放宽为 PrivateAddress|DnsFailure——两者都证明 async 门已接入，字面量 PrivateAddress 由单测确定性覆盖）；主 e2e（example.com "Example Domain"）**受阻于网络环境**：2026-08-10 午后该网络的挑战服务开始对 headless Chrome 指纹返回中文"安全验证"页（curl 同 UA 直连为干净内容；当日 09:59/14:00 两次通过）——环境性非代码回归，严格断言保留
- clippy：变更文件零新增（pre-existing 仅 orz-config 1 / xai-fast-worktree 2 / agents/main.rs too_many_arguments）
- git diff --check：无错误

## 6. 踩坑记录（e2e 调试链）

1. **Chrome 调试端点 keep-alive**：`Connection: close` 被忽略——无界 read 循环挂在第二次 read（响应单块到达后连接保持打开）；修=Content-Length 感知 + 无 CL 时单块即止。
2. **Windows ConnectionReset（10054）**：Chrome RST 短生命周期连接（响应后）——read 错误当 EOF 宽容处理。
3. **grep 管道缓冲吞调试输出**：cargo test | grep 缓冲导致挂起误判——直接跑测试二进制重定向文件定位。
4. **孤儿 Chrome 累积**：调试期强杀 orz_host 测试进程 → tokio child Drop 不执行 → 孤儿 Chrome 持有 profile 锁、进程数累积（18 个）→ 后续测试 profile 删除失败；清理后正常。生产路径（close_session → shutdown）无此问题。
5. **`cargo build` 不重建测试二进制**：改了 lib 必须 `cargo test --no-run` 再跑测试。

## 7. 三面审查修复批（2026-08-10 复审）

三独立代理 + 主线程复核后的修复批（本审计文档 §1 描述与代码差异均已同步修正）：

| ID | 级别 | 发现 | 修复 |
|---|---|---|---|
| H1 | 高 | DNS 层 SSRF 防护未接线——`check_navigation_url` 是死代码，浏览器路径三处检查点全用 sync 变体；`169.254.169.254.nip.io` 类域名可直穿门禁 | 预检门/frameNavigated 重检/location.href 终检全部改走完整 DNS 门；e2e 门禁测试 URL 改域名形式（`169.254.169.254.nip.io`）；单测 `test.invalid` fail-closed |
| M1 | 中 | §3.7.3"未移交的账户/支付/密码管理面"无实现无登记 | §4 边界登记（含预填字段评估与启发式误伤权衡） |
| M2 | 中 | verifier 盲区：`browser_read`（host 车道无 target）在 off/非 available 下成功漏检 | verifier 增按工具名检查（off 的 tool_started + 非 available 的 tool_completed）+ 3 个回归测试 |
| M3 | 中 | `ready()` 在浏览器进程死亡后返回陈旧 true——自愈只覆盖探针时启动失败 | `ready()` 补 `is_alive()`（try_wait，tokio 缓存 reap 状态） |
| M4 | 中 | close_session 脱离式 shutdown 与同 session 立即重开竞态（旧 teardown 删新 profile / SingletonLock 冲突） | profile 级 async 锁串行化 launch 与 shutdown |
| M5 | 中 | 缺/空 url 无稳定错误码；多余参数静默忽略 | `[browser_read_missing_url]` / `[browser_read_invalid_arguments]` + 测试 |
| M6 | 中 | 失败路径 tab 泄漏（total_budget 取消未来体后 closeTarget 不执行） | closeTarget 移到预算外全路径执行（固定 5s cap）+ 预算 deadline 化 |
| L2/L8 | 低 | 页脚文本与审计/web_fetch 先例不一致（无冒号无分隔）；终检在提取后；evaluate 异常静默转空串；http_get_json 不校验状态行/非法 CL；about:blank 初始帧脆点；事件洪泛阻塞响应路由；IPv4-compatible/6to4/NAT64 嵌入 IPv6 漏网 | 页脚对齐 `\n\n[browser_read content truncated: ...]`；终检先行；exceptionDetails → 显式 Command 错误；状态行 200 + 非法 CL 显式报错；about:blank 豁免；通道 256 + loadEventFired 不丢其余 try_send；ssrf `to_ipv4()`（覆盖 compatible）+ 6to4/NAT64 提取 |
| L3 | 低 | "schema allOf 双保险"过强——then 无 required，缺失时 schema 通过 | schema 非 off `then` 补 `required: ["capability_status"]`（含 framework_fallback 方向，全 13 journals 复验通过） |
| L4/L5 | 低 | 文档数字失实：unsupported 回归测试实为 3 个（非 4）；Rust 分档计数；`DiscoveryError::VersionCheck` 死变体（无版本检查实现）；mod.rs 单测注释被误报"取 8 位写错"——**复核后原注释正确**（"RUN12345678" 取 8 = "RUN12345"） | 文档修正；删除死变体 |

**修复批后续裁决（用户，2026-08-10）**：**D-13 有头默认**——`browser_launch_args` 增 `headless` 参数（调用点读 `ORZ_BROWSER_HEADLESS` env），默认去 `--headless=new`（窗口可见，操作者可通过窗口手动登录，cookie 会话内持久）；`ORZ_BROWSER_HEADLESS=1` 回退 headless 供无显示环境。双模式 e2e 均通过（有头 2/2 + 回退 2/2；之前 example.com 的"安全验证"挑战页在 headless 下偶发，有头指纹不触发）。

**登记不修**：6to4 的 2002:V4::/48 之外的其他嵌入形式（Teredo 等，现代系统不路由）；send_command 单命令无超时（deadline 化后错误码语义已收敛）；framework_fallback 仍声明 browser_read（防呆门兜底，设计选择）；profile 锁注册表条目不回收（每 session 一个 tiny Arc，session 表有界）。
