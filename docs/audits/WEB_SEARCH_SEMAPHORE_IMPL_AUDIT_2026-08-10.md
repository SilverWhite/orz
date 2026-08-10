# GAP-WEB-SEARCH-SEMAPHORE 实施审计（2026-08-10）

- 审计对象：轨道 A 切片——全局 `web_search` 并发=1 semaphore（ADR-0010 §3.7.7 工具合同第 7 项 web_search=1 分量 + §11.3 + §3.2）
- 状态：✅ 实施完成 + 三面审查闭环（修复批已回填）
- 范围：Rust 生产实现（orz-host 2 文件）+ 测试；无 schema/事件/verifier 变更；无 capture 影响（事件序列零改动）
- 入口：`orz/crates/orz-host/src/lib.rs`（`OrzHost::call_tool` / `web_search_semaphore` 字段）、`orz/crates/orz-host/src/tools.rs`（`is_web_search_tool`）

## 1. 切片裁决

- 用户裁决（2026-08-10）：轨道 A 剩余 5 切片中本次先实施 **web_search=1 全局 semaphore**（其余：conversation 跨 prompt 恢复 / ADR-0006 凭据注册表接线 / project_doc_index 索引缓存 / PDF 证据管线留后续）。
- 前序边界：GAP-RETRIEVAL-TOOLS 审计 §4 登记"web_search=1 全局 semaphore（§3.7.7 工具合同项）"，D-10 记录"主/子派发串行、并发窗口极小"——本切片把合同显式化，为 §11.3 双并发检索 session 并行推进提供资源所有权约束。

## 2. 实施内容

### 2.1 匹配函数（tools.rs）

`pub fn is_web_search_tool(name: &str) -> bool`——精确 `web_search` + `web_search_` 前缀变体。与 `relay::is_web_retrieval_tool` 的 search 半边逐字符同构（防拼写绕过）；变体当前不存在于注册表（仅 `web_search`/`web_fetch` 两个工具 id），匹配为防御性前瞻（C3-3 登记）。`web_fetch` 明确排除（合同只限 `web_search`）。

### 2.2 Semaphore（lib.rs）

- 字段：`web_search_semaphore: Arc<tokio::sync::Semaphore>`（容量 1），构造默认 `Semaphore::new(1)`——**每 OrzHost 实例一个**（见 §4 边界）。
- 挂载点：`OrzHost::call_tool`——**所有消费方的唯一汇点**（F8 确认无旁路：orz-host 内仅有的两处 `toolset().call` 都在 call_tool；acp_server 的 JournalOnlyHost 空 registry；use_tool/computer-hub 未实例化；conformance capture/plan/grill/ACP 全走同一 host→run_host_tool→call_tool）。**注意（三面审查 C2-1/F1/P3-3 交叉确认）**：当前运行时 `web_search` 尚无真实执行路径——relay 把所有 `web_search*`/`web_fetch*` 名路由到 ExternalRetrieval，检索 lane 内 `denies_nested_dispatch` 拒绝其再次派发（`nested_subagent_dispatch_refused`，agent_loop.rs:774-808），主 Agent 直调亦经同一派发链；即 web_search 工具当前只作派发触发器，从未到达 call_tool。挂载点结论不变——**所有未来执行形态（解禁/直执行/双并发）都收敛于 call_tool**，semaphore 是 §11.3 前瞻保护（与 D-5 一致）。
- 执行形状：`is_web_search_tool(name)` 时 `acquire_owned()` 后执行 `toolset().call(...)`；permit 绑定在 async block 局部变量，future drop（含 P0-1 timeout 丢 future）即自动释放——持票者无法泄漏门。
- 观测：`tracing::debug`（acquire 前）；无 journal 事件（见 D-4）。

## 3. 决策登记

| ID | 决策 | 依据 |
|---|---|---|
| D-1 | 挂载点=OrzHost::call_tool，非 WebSearchTool::run 内部 | 继承 crate（codegen/orz-tools）不加逻辑（Slice #13 纪律：host 过滤是唯一表面）；host 是控制面；唯一汇点覆盖全部（当前与未来）消费路径。当前"主/子共享"为结构事实（共享实例），**非当前竞争路径**——见 §2.2 注 |
| D-2 | 匹配=精确名+`web_search_*` 变体；web_fetch 不 gate | §3.7.7 只限 web_search；变体调用同一 Grok search 后端；relay 路由同构防拼写绕过 |
| D-3 | 等待计入 P0-1 300s 预算（acquire 在 timeout 包裹之内） | 一次工具调用生命周期（含等待）有界；timeout 丢 future 即释放 permit（无法泄漏）；正常顺序执行零差异。**机制（三面审查 F2 修正）**：容量 1 下持票者必先于等待者获取且共享同一 `tool_timeout`——持票者截止时刻严格更早，其完成/超时都会先释放 permit，**等待者不可能在纯等待中被 timeout 误杀**；极端情形=持票者跑满 300s 后等待者带零头预算开始执行，随即在**自己的执行中**超时（此时杀树正当）。**审查修复（P3-1 升级）**：等待阶段超时不再调用 `kill_active`（等待者无自身进程树，杀全局纯属误伤——已实证：新 timeout 测试曾误杀并行测试的 python 子进程）；错误消息区分"执行超时杀树"与"等待超时未杀，可重试" |
| D-4 | 无新事件/schema/verifier；等待期间无 journal 事件 | §3.7.7 第 7 项明示 web_search=1 是资源所有权约束，"不削减子代理 session 的模型、thinking、transport 或 120 轮预算"（等待不计工具轮，tool_rounds 只在 batch 完成后 +1）；ToolStarted/ToolCompleted 括起整个调用使不 journal 可辩护。**审查补充（F3/F4）**：等待非瞬态（可达 ~300s）；默认安全由"tool_timeout(300s) < stall watchdog(360s)"不变量保证——**配置逃逸口可破坏**（`ORZ_TOOL_TIMEOUT_SECS`>360 或 `ORZ_STALL_TIMEOUT`<300 重开 2026-08-08 P2-1/D2-1 已消除的竞态），且 main.rs 帮助文本声称 `ORZ_TOOL_TIMEOUT_SECS=0` 为"0 disables/unbounded"而代码 `filter(|&s| s>0)` 把 0 当缺省回落 300s（pre-existing 文档/code 不一致，登记不修） |
| D-5 | 现实并发确认：agent_loop 工具 batch 为顺序 for 循环（非并行） | 三面审查补全（F1/C2-1）：真正的串行化来源是 relay 路由 + 嵌套派发门——web_search 当前无执行路径；semaphore 为 §11.3 双并发设计场景的前瞻保护，当前无行为差异 |
| D-6 | 测试组合=串行化机制测试 + 非搜索工具不受 gate 影响 + 匹配单测 + timeout-drop 泄漏测试（F7） | 真实 client 不可注入（client 由 env config 构造），机制级断言足够；无 key 快速失败路径天然构成"释放后完成"观测点 |

## 4. 边界（明确未做，登记给后续切片）

- **C2-1 前序切片遗留缺口（三面审查交叉确认，本切片不修）**：外部检索 lane 内 `web_search`/`web_fetch` 被嵌套派发门拒绝（`denies_nested_dispatch`，防跨 lane 递归的有意设计）——外部检索子代理**当前无法执行 web 检索工具**；13 个真实 journals 无任何 external_retrieval 目标或 web_search 工具完成事件。该缺口未登记在 GAP-SUBAGENT-RUNTIME/GAP-RETRIEVAL-TOOLS 审计 §4 边界；本切片只保证解禁后 semaphore 自动覆盖子代理路径。**登记待后续切片处理**（工具投影/直执行形态）。
- `web_fetch` 并发语义：不受限（合同只限 web_search）；如未来需要站点级限流，属检索工具合同扩展，另行切片。
- ADR-0006 凭据注册表接线：env 通道不变（D-7 seam 保留）。
- capture/conformance：无事件变更，13 个真实 journals 不需重捕。
- **跨 host 实例并发（三面审查 P3-2/F5 修正）**：semaphore 是**每 OrzHost 实例**一个（非进程内全局）——ACP server 单进程可承载多个并发 session，各自独立 semaphore，两个 session 的 web_search 可同时执行；跨实例/跨进程并发均契约范围外（§11.3 "全局"是 session 内语义：主+子代理共享实例）。

## 5. 验证

- `cargo test -p orz-host`：**145 passed / 2 ignored**（+4 新测试：`web_search_call_waits_for_global_permit` / `non_web_search_tools_ignore_the_gate` / `web_search_timeout_releases_the_permit` / `is_web_search_tool_matches_search_family_only`；修复批后连续 3 次全绿无 flake）
- 全量 `cargo test --workspace -j4`（167 测试目标）：**10348 passed / 0 failed / 96 ignored**。基线出处说明（C2-2）：10345 为修复批前本切片实施者自测数字（无已提交审计记录 workspace 总数；LOCAL-BROWSER 审计记录 orz-host 141 / orz-loop 160 分仓数字）；+3 与新增 3 测试一致，-p orz-host 实测 144/145 与自述吻合
- clippy：变更文件零新增（4 条 warning 全为 pre-existing：orz-config 1 / xai-fast-worktree 2 / agents/main.rs too_many_arguments 1）
- `git diff --check`：干净
- capture：conformance 13/13 全过（`--ignored conformance_capture`，第 13 场景 local-browser-read 源自 LOCAL-BROWSER 切片）；重捕 journals 与已提交 fixtures 结构对比——**事件序列/payload/事件数全部一致**，差异仅为 per-run 字段（timestamp 哈希链、随机 run_id、result_id digest 分量——P2-3/D2-2 已登记形态）；fixtures 不需重捕
- `check_repository.py`：valid（索引新增条目链接存在；schemas 235 / 13 journals）

## 6. 三面审查闭环（2026-08-10，修复批回填）

三独立代理审查（设计合理性/实现合理性/符合性）交叉结论：**无 D1/P0/P1/C1 级问题**；交叉确认一项前序遗留缺口（C2-1，登记 §4）。修复批：

- **P3-1 升级修复（实现 P3-1 + 设计 F2 + 符合交叉）**：等待阶段超时不再 `kill_active()`——等待者无自身进程树，全局杀树纯属误伤；新测试 `web_search_timeout_releases_the_permit` 曾实证误杀并行测试的 python 子进程（`call_tool_timeout_kills_process_tree` 失败 exit 1）。修复后错误消息区分执行超时（杀树）与等待超时（未杀、可重试），连续 3 次全量全绿。
- **P2-1 修复**：测试间 env 竞态——`web_search_call_waits_for_global_permit` 的 `remove_var` 与 tools.rs `web_search_config_follows_env_key` 的 `set_var`（且不恢复，向同 binary 泄漏 `ORZ_WEB_SEARCH_API_KEY=test-key`）构成双向竞态；修复=共享 `TESTS_ENV_LOCK` 静态 Mutex 串行化 + `EnvVarGuard` RAII 保存/恢复（tools.rs pre-existing 测试同步收敛）。
- **P3-4 修复**：semaphore Closed 分支错误链——`map_tool_error` 只透传 Display 文本，detail 内联 code 防模型看到裸 "semaphore closed"。
- **F7 修复（设计建议）**：timeout-drop 释放回归测试——`with_tool_timeout(100ms)` + 外部持票 → 断言 Timeout → 释放后再次调用立即可 acquire（持票者无法泄漏门锁定）。
- **登记项**：F3 配置逃逸口（tool_timeout ≥ stall window 破坏不变量）+ main.rs `0=unbounded` 文档/code 不一致（pre-existing）；F4 等待时长不可观测（acquire 成功后 debug 记录建议，未来双并发形态下模型重试前提）；F6 变体防御性匹配；P3-5 else 分支重复调用（风格可接受）；C2-2 基线出处（§5 已注明）；C3-1 §4/§5 journals 计数统一为 13；C3-2 索引措辞限定"web_search=1 分量闭合"。

## 7. 索引登记

- 新增 canonical 条目 **GAP-WEB-SEARCH-SEMAPHORE**（`implemented`; 2026-08-10）至索引 §3.1；§8 `implemented` 组同步。
- 条目措辞：挂载点=唯一汇点 + §11.3 前瞻保护（不声称"当前共享竞争"）；闭合范围限定"web_search=1 分量"（§3.7.7 第 7 项其余分量——tab ownership/站点限流/下载上限——分别由 GAP-LOCAL-BROWSER 与 client 内部限流覆盖）。
- GAP-RETRIEVAL-TOOLS 审计 §4 边界"web_search=1 全局 semaphore"由此切片闭合。
