# web_search 执行器（DeepSeek 服务端搜索）+ C2-1 解禁 实施审计（2026-08-11，含方向修正）

- 审计对象：web_search 后端 = DeepSeek 服务端 web search（Responses API，同 key）+ C2-1 解禁（外部检索 lane 内 web 工具 Host 直执行）合并切片
- 日期：2026-08-11
- 范围：`orz/crates/orz-host`（credentials.rs 简化 / tools.rs config / lib.rs 字段与 seam + live 测试）+ `orz/crates/codegen/orz-tools`（web_search client 执行器）+ `orz/crates/orz-loop`（agent_loop 派发 / controller 三门）+ 主仓文档
- 用户裁决：① 合并窗口 ② lane 内自执行 web 工具豁免 per-call 权限门（授权链=显式 mode 门）③ **方向修正（2026-08-11 实施中）**：web_search 不依赖外部检索 API（xAI Grok 曾方案被否决）——检索必须来自当前接入的 provider（DeepSeek 服务端 web search，同一把 key）

## 1. 背景与缺口

| 缺口 | 登记出处 | 本切片状态 |
|---|---|---|
| D-7 seam：web_search key 注入留后续 | `GAP_RETRIEVAL_TOOLS_IMPL_AUDIT_2026-08-10.md` §4 / `WEB_SEARCH_SEMAPHORE_IMPL_AUDIT_2026-08-10.md` §4 | **闭合**（复用主 DeepSeek key，无新注册目标） |
| C2-1：web_search 无任何真实执行路径 | `WEB_SEARCH_SEMAPHORE_IMPL_AUDIT_2026-08-10.md` §4 L39 | **闭合** |
| `WebSearchConfig::redacted()` 未接生产路径 | SEC-CREDENTIALS 组件级复核发现 | **闭合**（唯一出口 = `OrzHost::web_search_config_redacted`；生产接线=构建时 `tracing::info!(redacted)`） |
| **xAI Grok 独立搜索后端（本切片初版）** | 初版把 web_search 接到 xAI Responses API（第二供应商 + 第二 key） | **撤回**——用户裁决：框架检索能力不是外挂，必须用当前接入的 provider |

## 2. 方向修正记录（初版 → 终版）

初版方案把 web_search 接到 xAI Grok 搜索 API（`orz-grok/search` 注册目标 + `ORZ_WEB_SEARCH_API_KEY` env + 独立凭据层）。用户裁决否决：其他成熟框架（Claude Code 等）的检索是模型 API 自带能力（Anthropic WebSearch / DeepSeek 原生 web search），依赖独立搜索 API = 框架不完整。

**协议实测证据（2026-08-11，真实 key）**：

| 端点 | 结果 |
|---|---|
| OpenAI 兼容层 `chat/completions` + `tools:[{"type":"web_search"}]` | **400 不支持**（`unknown variant 'web_search', expected 'function'`）；`web_search_options` 参数被静默忽略（无搜索发生） |
| Anthropic 兼容端点 | 报错泄露合法工具类型：`web_search_20250305` / `web_search_20260209`（`server_tool_use` 形态不接受） |
| **Responses 端点 `/v1/responses` + `tools:[{"type":"web_search"}]`** | **可用**——同一把 DeepSeek key；服务端自主多轮搜索（`web_search_call`：search queries / open_page url）+ 综合答案 `output_text`；无需客户端回放 |
| 响应字段差异 | DeepSeek 的 search action 只有 `queries`（无 `query`）——async-openai 类型化 `rs::Response` 解析**必然失败**，执行器必须 raw-JSON 解析 |

## 3. 实现清单

### 3.1 凭据（orz-host，简化复用）

- **`crates/orz-host/src/credentials.rs`（初版重写）**：删除 xAI 目标与平台 reader；只保留 `DeepSeekCredentialReader`（委托 `orz_loop::gateway::credentials::read_agent_api_key()`——Windows CM `orz-deepseek/agent` + 非 Windows `ORZ_DEEPSEEK_API_KEY`，blob 零化由该模块承担）+ `CredentialReader` trait（测试 seam，无 key → Disabled）
- **`crates/orz-host/src/tools.rs`**：`web_search_config(reader)`——Ok→Enabled（base_url 默认 `https://api.deepseek.com`、model 默认 `deepseek-v4-flash`；`ORZ_WEB_SEARCH_BASE_URL`/`ORZ_WEB_SEARCH_MODEL` env 覆盖保留）；Err→Disabled + warn
- **`crates/orz-host/src/lib.rs`**：`web_search_config` 字段（替换 bool，单点构造）+ `with_credential_reader` seam + `tracing::info!(redacted)` 生产接线 + `web_search_config_redacted()` 唯一序列化出口（DeepSeek key = 主凭据，纪律更重要）

### 3.2 执行器（codegen/orz-tools，后端替换）

- **`implementations/web_search/client.rs`**：
  - 请求侧不变：async-openai `CreateResponseArgs` + `WebSearchTool`（Responses 协议与 DeepSeek 同构；live 验证确认含 filters 的完整请求形态被接受）。实际请求 URL = `{base_url}/responses`（默认 base_url `https://api.deepseek.com` 与主 transport 同源；"/v1/responses" 是规范路径简称）
  - 响应侧改为 **raw-JSON 解析**（`response_content`/`extract_citations`/`extract_citation_pairs` 遍历 `output[]`）：`message` 项的 `output_text` → content；`url_citation` annotations（xAI 兼容路径，DeepSeek 返回空）+ `web_search_call`（status=completed 且 action=open_page → URL，剥离 `#ws_call_id=...` 片段）→ citations
  - 理由：DeepSeek 的 search action 无 `query` 字段，类型化 `rs::Response` 解析必失败（实测证据）
- **测试**：`test_extract_citations_deepseek_web_search_call`（纯单测：search 动作/失败页排除、ws_call_id 剥离、去重）+ `deepseek_search_parses_web_search_call_citations`（wiremock 端到端）+ **live 实机** `live_deepseek_web_search_roundtrip`（orz-host，`#[ignore]`+`ORZ_TEST_LIVE=1`，真实 key 32s 通过：content 非空 + citations 非空）

### 3.3 C2-1 解禁（orz-loop，保留不变）

- `relay.rs`：`is_web_retrieval_tool` pub(crate)；route() 纯映射器不改
- `agent_loop.rs`：`lane_self_execute = target==ExternalRetrieval && denies_nested_dispatch()` → effective=Host 直执行；嵌套门对 `retrieve_project_*` 防递归保留；lane 泛化
- `controller.rs` 三门：off 门扩展 web 族（target 字段分名）/ **local_browser 新门** `retrieval_mode_requires_framework_fallback`（无此门则 local_browser 下 web 工具真执行=跨 lane）/ 权限豁免 `permission_gated`（lane 内跳权限桥含事件）
- 完整执行链：主 Agent web_search → relay → mode 门（framework_fallback+Available）→ 外部子代理 → lane 自执行 → run_host_tool（三门）→ `OrzHost::call_tool`（semaphore=1）→ toolset → `WebSearchClient`（DeepSeek `/v1/responses`）→ 服务端搜索 → 结果回子代理 → evidence ledger → [RESULT_JSON]

## 4. 决策表

| 决策 | 裁决 | 依据 |
|---|---|---|
| D-1 后端选择 | **DeepSeek 服务端 web search**（非 xAI Grok） | 用户裁决 2026-08-11：检索必须来自当前接入的 provider，不依赖外部检索 API |
| D-2 凭据 | 复用 `orz-deepseek/agent`（无新注册目标、无新 env） | 同一把 key 同 provider；eval 容器零新增（`ORZ_DEEPSEEK_API_KEY` 已有） |
| D-3 协议端点 | Responses API `/v1/responses` | 实测：OpenAI 兼容层 400 不支持；Responses 可用且无需回放 |
| D-4 响应解析 | raw-JSON 遍历（非类型化 `rs::Response`） | 实测：DeepSeek search action 无 `query` 字段，类型化解析必失败 |
| D-5 权限豁免 | lane 内豁免 per-call 权限桥 | 用户裁决；授权链=mode 门（journaled transition）；不豁免则 Benchmark 策略（eval `--allow-write`）NetworkCall→Deny，C2-1 空转 |
| D-6 lane 泛化 | 内/外检索 lane 均自执行 web 族 | 机制单一；受 mode 门+semaphore+ledger 约束 |
| D-7 api_key_provider | 保持 None（构建期读） | 声明时点天然构建期；key 轮换留后续切片 |
| D-8 初版 xAI 方案 | **撤回**（工作区未提交，无历史污染） | 用户裁决；ADR-0006 表新增行同步撤回，改记否决记录 |

## 5. 边界（登记不修 / 后续切片）

- **C2-1 闭合声明**：闭合 `WEB_SEARCH_SEMAPHORE_IMPL_AUDIT_2026-08-10.md` §4 L39 缺口；历史审计不改写；13 个真实 journals 无 web_search 完成事件——**不重捕 fixtures**（无事件/schema 变更）
- **DeepSeek 服务端搜索责任边界**：单次请求服务端自主多轮搜索（实测 32s；可能 30-90s+）——semaphore 等待变长，P0-1 300s 兜底；`open_page` 失败率由服务端承担（实测 x.ai 等反爬页 failed），执行器只取 completed 项
- **xAI 兼容路径保留**：`url_citation` annotation 解析保留（后端无感知差异；DeepSeek 返回空 annotations）
- **`WebSearchConfig` derive(Debug) 含 key**（codegen types.rs:7）：继承 crate 不修；纪律=只经 `redacted()` 出口序列化
- **Benchmark 策略下主 Agent 通用 NetworkCall 仍 Deny**：豁免仅 lane 内
- **key 轮换 provider**：`api_key_provider` 接线留后续切片
- **Linux 交叉编译**：非 Windows 分支（env 通道）与 orz-loop 已验证实现逐行同构；eval 二进制构建时验证

## 6. 验证

| 项 | 结果 |
|---|---|
| `cargo test -p orz-host` | **158 passed / 0 failed / 3 ignored**（+live DeepSeek 实机测试；semaphore/路由/redacted 测试语义不变） |
| `cargo test -p orz-loop` | **170 passed / 0 failed / 3 ignored**（C2-1 三门 + 嵌套门拆分） |
| `cargo test -p orz-tools` | **2711 passed / 0 failed**（+2 DeepSeek 解析测试） |
| **live 实机** `live_deepseek_web_search_roundtrip` | ✅ 32s 通过（真实 key：请求形态被接受 + content/citations 提取成功） |
| clippy（三个变更 crate） | 零新增（pre-existing：orz-config 1 / xai-fast-worktree 2 / agents/main.rs too_many_arguments 1） |
| 断言要点 | 嵌套门守卫仍绿；lane web_search 到达 toolset 汇点（"missing required resource" 快失败）；无 nested 拒绝事件；豁免无 PermissionRequested；B2 门无 ToolStarted；redacted 不含真 key；DeepSeek 响应解析（web_search_call→citations、ws_call_id 剥离、failed 排除、去重） |

## 7. 三面审查闭环（2026-08-11，实施后）

三面审查（设计合理性 / 实现合理性 / 设计与实现符合性）交叉结论：**无 D1/P0/P1/C1 级问题**。设计方向修正（拒绝第二供应商）与 FUS-CORE/FUS-RETRIEVAL-MODE/§11.7 既有语义一致；实现 seam（凭据失败→Disabled 不静默）、raw-JSON 解析（协议实测依据）、三门行为（无 ToolStarted / 无 PermissionRequested / 无 nested 拒绝事件）均有测试锁定；审计 §3/§4/§5 声称与代码逐项核对一致（凭据委托、redacted 出口、lane 自执行、模式门镜像、semaphore 覆盖），"零事件/schema 变更"与 git 状态一致（assurance/、runtime/ 未动）。发现项：

- **P2-1（文档完整性）**：ADR-0006 新增文字引用 "§ext 2026-08-07 Linux container channel"——ADR-0006 无 §ext 章节（committed 与工作区均无）；该扩展目前只登记在 `TERMINAL_BENCH_2_EVAL_2026-08-08.md`（"ADR-0006 扩展（Linux 容器通道，注释已记录）"）。修复=将非 Windows env 通道（`credentials.rs` 非 Windows 分支 + eval 容器注入，生产事实）正式补登记为 ADR-0006 小节，使引用成立。
- **P3-1（验证声称漂移）**：§6 声称 orz-host "160 passed / 4 ignored"，实测 "158 passed / 3 ignored"（差 2 passed + 1 ignored）；orz-loop 170/3、orz-tools 2711 复跑一致；clippy 三 crate 零新增（pre-existing：orz-config 1 / xai-fast-worktree 2 / orz-loop 1）复跑一致。
- **P3-2（陈旧注释）**：orz-host `lib.rs` TESTS_ENV_LOCK 文档注释仍引用已删除的 `ORZ_WEB_SEARCH_API_KEY`（锁仍必要——BASE_URL/MODEL 覆盖测试；仅注释需更新）。
- **P3-3（措辞过强）**：`web_search_config_redacted()` 目前仅测试消费——索引 SEC-CREDENTIALS 与 ADR-0010 §11.7 的 "`redacted()` 生产单出口" 宜收窄为"redacted 唯一序列化出口（当前生产接线=构建时 `tracing::info!(redacted)`）"。
- **P3-4（补写索引不对称）**：v1.3 仅条 10（禁独立检索供应商）升级进 §14.3；C2-1 解禁（lane 直执行 + per-call 权限桥豁免，行为安全相关）只停留 header 登记层，建议 §14.3 补记该裁决（含 D-5 依据）。
- **P3-5（端点措辞）**："/v1/responses" 为规范路径简称；实际 URL = `{base_url}/responses`（默认 base_url `https://api.deepseek.com` 与主 transport 同源）；live 实测证明可用。

### 7.1 修复批（2026-08-11 应用，三面审查后）

- **P2-1 修复**：ADR-0006 新增正式小节 §2.3「扩展：Linux 容器通道（2026-08-07 登记）」（env 通道为 §2.2 统一 CM 裁决的显式例外，仅限无 CM 平台；来源=TB2 eval 文档"ADR-0006 扩展"）；§2.2 web_search bullet 的 "§ext 2026-08-07" 断链改为指向 §2.3。
- **P3-1 修复**：§6 验证表 orz-host 计数更正为 "158 passed / 0 failed / 3 ignored"（实测复跑；orz-loop 170/3、orz-tools 2711 复核一致）。
- **P3-2 修复**：orz-host `lib.rs` TESTS_ENV_LOCK 注释更新——引用现函数名 `web_search_config_follows_credential_source` 与现 env 覆盖（`ORZ_WEB_SEARCH_BASE_URL`/`_MODEL`），注明 `ORZ_WEB_SEARCH_API_KEY` 已随方向修正移除。
- **P3-3 修复**：索引 SEC-CREDENTIALS 与 ADR-0010 §11.7 的 "`redacted()` 生产单出口" 收窄为 "唯一序列化出口（当前生产接线=构建时 `tracing::info!(redacted)`）"；审计 §1 缺口表同步精确化。
- **P3-4 修复**：ADR-0010 §14.3 补记条 2「C2-1 解禁与 lane 内权限豁免（2026-08-11 登记裁决，未升级正文条款）」；header 登记补索引引用。
- **P3-5 修复（精确化）**：审计 §3.2 请求侧注明实际 URL 形态（`{base_url}/responses`，与主 transport 同源）。
- **P3-6 修复**：web_search/types.rs + client.rs 测试 fixture 的 x.ai URL/key 前缀统一替换为 DeepSeek 形态；`cargo test -p orz-tools web_search` 20 passed 复核通过。

## 8. 用户动作（手动）

1. **无需注册任何新凭据**（web_search 复用主 DeepSeek key）；可选实机验证：`ORZ_TEST_LIVE=1 cargo test -p orz-host -- --ignored live_deepseek_web_search_roundtrip`
2. eval `.env` **无需改动**（容器里 `ORZ_DEEPSEEK_API_KEY` 已注入，web_search 自动可用）
3. 推送双仓（用户手动惯例：主仓 docs / orz 分支）
