# 0BV 承接与检索真网复验杂项轮·处理报告（浏览器 SERP 链首接缝＋工作区切换 B 形态）

- **轮次**：`RUN-CLI-6ab7d8b7`（2026-09-26，狗粮轮）。
- **任务**：BACKLOG/TODO `P1-0bv`「0bs c 轮残余承接与检索真网复验杂项轮（可处理五件）」。
- **基线**：orz `42a14d16`（0.7.1；工作树干净起步）；父仓 `0a742bff`。
- **约束（用户令）**：不提交／不推送／不重建；完成落一份报告文档（本档）；摩擦项一并记录（§7）。
- **裁决**：过程中需裁决项就地衡量并裁决，决定随本档留痕（D-a…D-g，见各节）。
- **结果**：五件全落（① 全链落码＋定向测试；② 服务端＋前端落码＋测试；③④⑤ 落码＋钉子）；并件（REV-083 18 项＋0bw 六项）本轮**未开工**，如实记录＋裁决（§6）。

## §1 五件总览

| # | 事项 | 状态 | 读数 |
|---|------|------|------|
| ① | 浏览器 SERP 链首 resource 接缝＋预算单账本并账 | ✅ 落码 | 定向测试全过（§2） |
| ② | ⑬ 余项＝工作区切换（B 形态）＋单击切换／权限总览 | ✅ 落码（S2 口径） | orz-web lib 42✓；前端冒烟 13✓ |
| ③ | f13 编辑歧义失败回执给候选行号 | ✅ 落码 | search_replace＋registry 定向 125✓ |
| ④ | f14 替换后行长剧变提示 | ✅ 落码 | 新钉 2 枚过 |
| ⑤ | f15 压缩窗口回执给摘要块落点骨架 | ✅ 落码 | context_scale 13✓ |

## §2 ① 浏览器 SERP 链首接缝＋预算单账本并账

**语义**：`web_search` 检索链＝浏览器 SERP（若可用）→ 本地 HTTP 分段 → provider 合成（0bs ⑧ 裁决面）。浏览器车道与 `browser_control search` **同径同会话状态**（引擎链／软备忘／冷却／会话导航上限），只多一层能力注入缝。

**落码面（全链）**：
- `orz-tools/src/types/resources.rs`：新增 `BrowserSerpBackend`（`#[async_trait]`，`async fn search(&self,&str)->Result<BrowserSerpOutcome,BrowserSerpFailure>`）＋`BrowserSerpOutcome/Hit/Failure/Facts`（Hits 带 `tier/weight/reason`，与 `browser_control search` 信封同口径）。
- `orz-tools/src/types/output.rs`：`WebSearchOutput.browser_serp: Option<BrowserSerpFacts>`（skip-if-none；机械读数面）。
- `orz-tools/src/implementations/web_search/client.rs`：`search()` 委派新 `search_with_serp(query,allowed_domains,Option<&dyn BrowserSerpBackend>) -> (String,Vec<String>,Option<Facts>)`——链首成功即交付（`[browser_serp] engine=… hits=… navigations=… session=x/y` 逐条 URL 渲染＋引用池）；空／失败**让渡**且注记随链序并入（内容前缀或失败 `details.browser_serp_fallback`／`local_segmented_fallback`；不静默）。
- `…/grok_build/web_search/mod.rs`：`run()` 从资源缝取 `Arc<dyn BrowserSerpBackend>`（缺席＝链首不可用）并回填 facts。
- `orz-host/src/browser_serp.rs`（新）：`HostBrowserSerp{browser: Arc<Mutex<SharedBrowser>>}`——ready 检查→`browser_unavailable` 让渡；否则 `handle_browser_control({"action":"search"})` 信封解析（hits／navigations＝`engine_attempts` 中 ok|failed 计数、成功面至少 1／会话读数取 `serp_session_navigations()`）；cause 派生 `ceiling|all_engines_failed|empty|host_error`；`bounded(300)`。
- `orz-host/src/lib.rs`：装配期注入（`toolset.resources.try_lock()`；与字段**同一** browser 槽——之后 `swap_browser_session` 换入的真实会话对适配器同样可见）。
- `orz-host/src/tools.rs`：`structured_from_output` 扩 `browser_serp` 事实（guard＝引用非空 **或** 事实在场）。
- `orz-loop`：`SerpSearchBudget::merge_used(n)`（**无预留并账**；`tool_run.rs` 在既有 settle 块后对 `tc.name=="web_search" && !serp_reserved` 并账）＋`host_exec/serp.rs::browser_serp_navigations_from_structured`（读 `structured.browser_serp.navigations`）。

**裁决**：
- **D-c**：能力类型落 `types/resources.rs`（BACKLOG「`registry/types.rs` 资源缝」按**缝**理解——注册/读取经 registry 的 `update_resource`/`get`；类型随其余 resource 类型放置）。
- **D-d**：预算并账采 **settle 式**（真实导航数结进同一账本；**不做派发前拒绝**——链首是机会性车道，预算用尽只影响其后可用空间，搜索本身仍走本地 HTTP→provider；会话上限/冷却仍兜物理底）。
- **D-f**：适配器**不自动拉起浏览器**（use-if-ready；未就绪→`browser_unavailable` 让渡）。理由：a) 一次普通 `web_search` 不应以启动 Chromium 为副作用；b) 会话键 `session_id` 在宿主构造后才设置，此处拉起存在 profile 键分歧面。跟进项（未做）：共享会话键的自动拉起。

**定向读数**：`orz-host browser_serp` 3✓；`orz-tools web_search` 52✓；`orz-host structured_from_output` 1✓；`orz-loop` serp 定向 15✓。

## §3 ② 工作区切换（B 形态）＋单击切换／权限总览

**S1 要点落实对照**（要点③④⑤；S1 已定稿，本件不再等形态裁决）：

| S1 要点 | 落实 |
|---|---|
| B 形态（服务随启动而立；A 单例否决） | 不动进程架构；桥内切换 |
| 信任清单全局共享 | 沿用用户级 TrustStore 只读投影（`trust::list_trusted_workspaces`） |
| 服务内点击切换 | 探索器「已信任工作区」行可点击→`workspace://switch/<path>`→`POST /api/workspace/switch` |
| 桥 cwd 可切换（内部锁） | `ServerState.cwd: Arc<RwLock<PathBuf>>`＋`cwd()/set_cwd()` 访问器（短临界区） |
| 清单即时重解析 | 六读面全部经 `cwd()` 快照（ws_journal／runs／run events／conversations／archives／archive detail） |
| 新会话在新 cwd spawn | `ws_acp` 按**连接时点** cwd spawn；**每工作区一槽**（见 D-g）允许新区并行开新会话 |
| 旧对话子进程不杀 | 切换不触碰会话/子进程；槽按工作区隔离 |
| 同会话不迁移 | `ensureSession` 只认既有会话；前端记录 `state.sessionRoot`（会话初始根） |
| 切换不清场 | 前端不重置内容区；旧会话加「旧工作区会话（回看）」系统提示 |
| 旧会话按归属根解析 | 服务端 `?root=`（六读面，fail-closed 只收「当前∪已信任」canonical 根）；前端 `tailRun/replayRun/refreshRuns` 带 root |
| 未信任 fail-closed | 桥侧 403「未信任工作区：请先…点「信任」」；前端原样回显 |
| 运行中（活跃 run）禁切 | 前端 `state.running` 主判（桥侧无活跃 run 读数，不虚设判定——**D-e**） |

**服务端**：`ServerState` 换锁＋`resolve_root()`（fail-closed）＋`path_eq`（Windows 大小写/分隔符归一）；`TokenQuery/TailQuery/EventsQuery` 增 `root`；`ws_acp` 409 文案改「该工作区已有活动连接」；新增 `api_workspace_switch`（canonical 目录核→400；已信任门→403；`set_cwd`；回 `{ok,cwd,trusted_workspaces}`）＋路由。**acp_pump**：`SessionSlot` 由全局单槽改**每工作区一槽**（`HashSet<String>`，键＝连接时点 cwd；`try_acquire(key)/is_held(key)/any_held/release(key)`；`SlotGuard::new(slot,key)`＋`releaser` 带 key）。

**前端**：`api.js`（`switchWorkspace(path)`＋`sendJsonBody`；`fetchRuns/fetchRunEvents/fetchConversations/fetchArchives/fetchArchive` 可选 `root`）；`widgets.js`（已信任工作区行可点击＋「（当前）」标记＋点击标题说明；`ui.openUri('workspace://switch/…')`）；`main.js`（`pathNorm`／`switchWorkspaceTo()`（运行中禁切→回显不请求；成功→更新 cwd/信任清单＋`refreshExplorerData()`＋旧会话提示）／`openUri` 分支／`ensureSession` 记 `sessionRoot`／`syncLiveTail`＋`openRunReplay` 归属根）；`journal.js`（`refreshRuns(root)`／`replayRun(…,root)`／`tailRun(…,root)`）。

**裁决**：
- **D-g**：单会话槽→**每工作区一槽**（旧全局单槽与「切换不清场＋新区可开」直接冲突；B 形态语义所需）。
- **D-e**（沿前判）：运行中禁切＝前端主判＋桥信任门 fail-closed。
- **权限总览口径（本轮实现，如实记录）**：探索器「工作区」组即总览——「当前: <cwd>」行＋「已信任工作区（N）」清单（含信任时间与当前标记）。ACAF 状态位面未在 Web 侧新开（信任绑 ACAF 的按钮面属 0bs ⑬ 批七已落面，本件不重复）；若需要独立「权限总览」面板，留后续 UI 形态批。

**测试**：`orz-web --lib` 42✓；`frontend_smoke.mjs` 13 项✓（新增 `switchWorkspace` 断言）。
**未做（范围/约束）**：S3 载体（用户令不重建）、S4 真机；桥侧「活跃 run」读数（不存在，不虚设）。

## §4 ③④⑤

- **③ f13**：`search_replace` 歧义失败回执附候选行号（`(candidate lines: …)`；主点＋归一化点两处；cap 10＋`+N more`）。**落地修正**（本轮实测发现）：同一行多处命中会重复报行号→`format_candidate_lines` 改为**去重**（同一行只报一次；`+N more` 按去重后行数计）；受影响的 `registry/types.rs` 模板测试逐字断言同步。
- **④ f14**：替换后版式剧烈变化提示（`Formatting notice`）——两判据：多行折单行／单行行长至少翻倍且 ≥200 字符；成功面 default＋concise 附注，legacy 契约面不加（钉 2 枚）。
- **⑤ f15**：压缩窗口回执内嵌「落点＝回复文本＋可照抄骨架」；骨架单一源（`summary_block_skeleton()`）。context_scale 13✓。

## §5 回归读数与归因

| 套件 | 读数 | 归因 |
|---|---|---|
| orz-tools `--lib`（全量） | **2971✓ / 0✗**（修后重跑） | f13 连带 1 项已修（§4） |
| orz-loop `--lib`（全量） | 851✓ / **1✗** | **预置红**：`search_engine_probe_detail_…`（期望 `bing_cn`、实测 `360search,baidu`）——来自 `abba6886`（0bs c 轮）批次，**非 0bv 触碰面**；独立复跑确认，本轮未修（防越界） |
| orz-loop serp 定向 | 15✓ | 含 `merge_used` 新钉＋serp 取证面 |
| orz-host `--lib`（全量） | 335✓ / 4✗（并发） | 4 项均为进程/超时/符号链接测试；**单线程复跑 4/4✓**（并发负载下 flaky，非 0bv 面） |
| orz-web `--lib` | 42✓ | ② 服务端面 |
| 前端冒烟（node） | 13✓ | 含新增 `switchWorkspace` 断言；10 模块导入无求值期异常 |

## §6 并件处置（REV-083 18 项＋0bw 六项）＝本轮未开工（如实）

- **实况**：本轮预算集中于五件落码＋验证＋本报告；并件 24 项**零动**（不虚报）。
- **裁决 R1**：并件沿既有约定（「执行随 0bv、闭合随 0bv」）**继续挂账**于 `P1-0bv`，留后续轮次执行；本报告不复制清单正文（最小契约面：逐条见 `083 审查档 §8`／`0BW 报告 §6`）。
- **裁决 R2（优先级建议，供后续轮）**：快项先行＝REV-083-16（GAP-SPAWN-ORPHAN-RECLAIM `pending→partial` 翻账）／15（assurance 冻结宣言）／01（权限语义收敛）／03（审批叙事收口）→钉子面＝07（`compute_event_hash` 守护测试）／12（panic 契约）／14（判据钉纪律）；文档批＝04（README＋ADR 状态行）／13（参数表三列化）／19（流程）／20（LIF 判据＋复裁日期）；0bw 大项＝① Linux Landlock／④ L4 git 检查点·undo 优先，⑤ CFA enforce 翻转**待 1124 读数**（runbook 已备，无读数不翻）。

## §7 摩擦台账

- **F1** PowerShell 把 cargo stderr 记为 `NativeCommandError`（exit 1 噪声）——判据取 `test result:`/`Finished` 行；已沿 0bd ⑥ 族记档。
- **F2** `orz-tools` 首编 ≈4 min（180 s 自动转后台）——大 crate 编译预算需预期；用日志尾/进程面跟进。
- **F3** `orz-host` 4 项进程/超时测试在**并发全量**下 flaky（单线程复跑全过）——测试面纪律建议：进程树类测试建议独立分组/降并行（观察项）。
- **F4** orz-loop **预置红 1 项**（§5 归因）——属 0bs c 轮引入、未同步测试；本轮不越界修，建议随下轮处置。**〔批注 2026-09-26 用户令「F4直接修吧」〕已由主会话批修复**：`retrieval/projection.rs` 两臂断言同步 0bs c 轮缺省引擎链（直连 `360search,baidu`／代理 `360search,baidu,duckduckgo`）＋本批两文件 fmt 收编（`agent_loop.rs`／`host_exec/serp.rs`，0bs ⑤ 口径）；读数 orz-loop 全量 **852✓/0✗**，预置红清零（随 0bv 落账批提交）。
- **F5** f13 连带面：编辑面文案改动会打破**逐字相等**断言（`registry/types.rs`）——已同步；另发现候选行号重复项（去重修正）。
- **F6** 类型变更涟漪：`search_with_serp` 元组 2→3 一处 `return` 未同步（E0308）；`ServerState.cwd` 换锁引起 2 处测试构造——均为编译期可见，首编即暴露。
- **F7** 前端冒烟日志经 GBK 控制台读出乱码（exit 0 判据不受影响）——同 0bw F7 族（读取侧转码梯未覆盖该路径），记录不修。**〔批注 2026-09-26 用户令「F7进0bv，跟着REV-083 18项一起」〕并件 0bv（同载并件三）**：journal 实证两轮执行均走 `run_terminal_cmd`（seq 1871 `output_encoding=gb18030`／1880 `utf-8`，后端编码门已识别）⇒ 缺口在下游消费臂；执行纪律＝先勘定后落码（梯臂真缺 ⇒ `decode_text` 接入＋钉子；写侧已重编码 ⇒ 按 0bw F7-B「捕获不可达边界」落纪律面与事实记档）。

## §8 改动清单（orz 工作树：24 文件＝23 M＋1 新增；未提交）

- `orz-tools`（7）：`implementations/grok_build/search_replace/{helpers,mod}.rs`／`implementations/grok_build/web_search/mod.rs`／`implementations/web_search/client.rs`／`registry/types.rs`／`types/output.rs`／`types/resources.rs`
- `orz-host`（4）：`src/lib.rs`／`src/local_browser/mod.rs`／`src/tools.rs`／**`src/browser_serp.rs`（新）**
- `orz-loop`（5）：`src/agent_loop.rs`／`src/context_scale.rs`／`src/host_exec/{mod,serp,tool_run}.rs`
- `orz-web`（8）：`src/{acp_pump,lib,server}.rs`／`assets/app/{api,journal,main,widgets}.js`／`tests/frontend_smoke.mjs`

## §9 下一步建议

1. **S3 载体**（待令：0bv 五件＋② 前端改动需重建才可达真机）→ **S4 真机**（0bv 验证点：浏览器 SERP 链首真网读数／切换矩阵：未信任拒绝、运行中禁切、切后旧会话尾不断、新区新会话）。
2. 并件按 §6-R2 顺序推进；F4 预置红随下轮修（或另行立项）。**〔批注 2026-09-26〕F4 已修、F7 已并件 0bv（用户令），批注见 §7。**
3. 观察项（不动）：f16（大文件分页表现良好）／f17（验证闭环判据）。

—— 报告完。未提交／未推送／未重建（用户令）；全部改动在工作树内，可经本档 §8 清单复核。
