# 检索编排机械层第二批设计（2026-08-30）

> 性质：独立设计轮文档（BACKLOG 0k / TODO P0-0k「第二批」三项）。范围与
> 顺序已在 2026-08-30 检索问题最终评判定案（ADR-0010 §14.43）：本批三项
> = `project_doc_index` v2 / 会话级 tab 池 + 同轮多页并行读取 / 委托契约
> 复杂度分档。本文产出设计定案与开放项，供用户裁决；设计轮不动计数，
> 实施放行另计（+1）。全部为机械层改动，模型可见工具面零变化
> （`retrieve_project_docs` / `project_doc_index` 维持 R1 封存不动）。

## 0. 结论摘要

1. **`project_doc_index` v2**：git HEAD 基线快照（`git ls-files` + blob 内容
   哈希）+ 工作树增量层（`git status --porcelain` + Blake3 内容哈希）+ 驻留
   索引（首建后 query 零全树 stat）+ 写后失效钩子（模型刚写的内容立即可搜）。
   非 git 工作区回退 v1 既有语义（行为不变）。
2. **会话级 tab 池 + 同轮多页并行读取**：有界 N 个 CDP target 常驻（默认
   4，`ORZ_BROWSER_TAB_POOL_SIZE`，0=关闭回退现状），每次读取**独占租约**、
   归还重置、LRU 回收；`LocalBrowserManager` 去掉「整读持锁」，同轮多个
   `browser_read` 真正并发。下载路径（`download_or_read`）不池化。顺带落
   DNS 预检会话级缓存（host + TTL）。
3. **委托契约复杂度分档**：纯函数机械映射 query/scope/max_results/lane →
   三档 effort（`standard` / `extended` / `deep`），只调 run 级预算（墙钟 +
   轮数）与候选/并行上限，**不扩大 [DOC] 回传与注入预算上限**（第一批
   16 行 / 8K 既定边界不动）；推荐在 `retrieval_close_record` 登记可选
   `effort`（schema 先行）。

开放项 3 条（见 §5）：①tab 池与 ADR-0010 §3.7.6「one tab lives exactly
for this call」措辞对齐（推荐 ADR 补写）；②close record 是否登记 effort；
③默认参数表确认。

## 1. 现状核对（2026-08-30 源码）

### 1.1 `project_doc_index`（orz-host/src/project_doc_index.rs）

- v1 diff 键 = `size + mtime(secs,nanos)`；**每 query 全树 metadata 步行**
  （stat only、零内容读），与跨 run 持久化快照比对
  （`.gsa/project-doc-index/cache.json`，schema 0.1.0-draft）。
- 已知限制（模块注释登记）：同 size+mtime 内容修改不可察（仅逃生阀
  `ORZ_PROJECT_DOC_INDEX_FORCE_RESCAN=1`）；索引不驻留（无写后失效面）。
- 工具面：主面 R1 封存（`R1_SEALED_MAIN_TOOLS` 含 `project_doc_index` 与
  `retrieve_project_docs`）；内部 lane 触发面（`retrieve_project_docs`）同样
  封存 → **生产主流程当前不可达**。v2 是内部检索未来解封 / restore-dormant
  路径的机械底子，本批不改封存状态。

### 1.2 浏览器读取（orz-host/src/local_browser/）

- `LocalBrowserManager.inner = tokio::sync::Mutex<Option<CdpBrowserSession>>`，
  guard **跨越整个读取**（`read_page` 需 `&mut self`）——第一批的同轮并行
  批次（`PARALLEL_READ_TOOLS` 已含 `browser_read`）在 manager 锁上被压平为
  串行。
- 单次读取（`cdp.rs` `read_page_inner`）：`Target.createTarget(about:blank)`
  → page ws 连接 → `Page.navigate` → 等待（full=loadEventFired；
  preview/keywords=text-ready + `Network.setBlockedURLs`）→ innerText →
  `Target.closeTarget`（teardown every path，5s cap）；总预算 60s。
- DNS 预检（`check_navigation_url`）每次调用独立、跨 host 无缓存。
- `ALLOWED_CDP_METHODS` 固定集合（含 createTarget/closeTarget/navigate/
  setBlockedURLs/Runtime.evaluate 等），测试锁定。

### 1.3 委托契约与预算（orz-loop/src/retrieval/）

- `build_retrieval_task_goal(args, prompt)`：query 必填（缺省回退 prompt）+
  可选 scope/max_results 机械并入 goal 文本（子代理只见 goal）。
- 第一批 run 级预算已落地：墙钟默认 600s（`ORZ_RETRIEVAL_SUBAGENT_TIMEOUT_SECS`，
  0 禁用）+ 轮数上限 60（`ORZ_RETRIEVAL_MAX_TOOL_ROUNDS`，与主车道取 min）；
  每次派发新建激活（R1 auto-close）。
- [DOC]/[SOURCE] 回传预算已落地：`RETRIEVAL_RESULT_LINE_CAP=16` +
  `RETRIEVAL_RESULT_BYTES_CAP=8K`（逐条装填，截断附结构化标注，标注不计
  入 8K）。

## 2. 设计一：`project_doc_index` v2

### 2.1 问题与目标

索引新鲜度是 agent 专用失效模式（调研 §3.1）：陈旧索引 → 模型搜不到自己
刚写的内容 → 空转烧 token。v1 两个缺口：①同 size+mtime 内容修改不可察
（仅逃生阀）；②每 query 全树 stat（索引不驻留）。v2 目标：git 仓库下
「一次扫描建基线、写后增量刷新、稳定窗口 query 零 stat」。

### 2.2 设计定案

1. **双基线结构**：
   - 基线层（git HEAD）：`git ls-files -z` 取受版本控制文件清单 +
     **Blake3 内容哈希**（基线/增量统一用同一内容身份键——调研点的
     `git cat-file` blob-hash 方案在 §9 折叠为 blake3，未跟踪文件同样
     覆盖、语义等价且更简）→ 基线条目
     `(relative_path, blake3, tracked, size, title, headings)`；构建时对
     每个基线 doc 文件提取 title/headings + blake3 一次。
   - 增量层（工作树）：`git status --porcelain -z --no-renames
     --untracked-files=all`（`--no-renames` 避免 rename 双路径解析歧义）
     取 modified/untracked/deleted → modified/untracked 重读并算 **Blake3
     内容哈希**（新增条目键 `(blake3, size, mtime)`）；deleted 从索引移除。
   - 跨 run 快照比对：mtime 只作 stat-clean 快速跳过，内容哈希提供稳定
     身份（NTFS 100ns 粒度、时钟调整、跨 run 复制不误判）。
2. **内容哈希选型**：Blake3 已是 workspace 依赖（orz-config / orz-memory
   在用，`blake3 = { workspace = true }`），无新依赖引入。`QueryHit` 对外
   契约保持 `content_sha256` 不动（evidence 管线契约零漂移），blake3 仅作
   索引内部 diff 键。
3. **索引驻留 + 写后失效**：
   - 首 query 构建；后续 query 不再全树 stat。
   - 失效通道三路：①写类工具完成（当前仅 search_replace /
     run_terminal_cmd 两个写类工具）→ host 置 dirty（run_terminal_cmd
     输出不可解析 → 整索引 dirty）；
     ②节流兜底（距上次刷新 ≥ `ORZ_PROJECT_DOC_INDEX_REFRESH_MS`，默认
     30000，query 前增量刷新一次，防外部编辑器改动）；③逃生阀
     `ORZ_PROJECT_DOC_INDEX_FORCE_RESCAN=1` 全量重建（保留）。
   - 「query 零 stat」目标：dirty 未置 + 节流窗口内 → 直接查内存（零 IO）；
     dirty/窗口到 → git status 增量（git status 内部仍会对全树做 stat，
     真正的零 IO 收益在驻留窗口；增量层只重读变更文件的内容，不重读
     未变文件——相对 v1 每次 query 必做全树 stat 的收益不变）。
4. **同 size+mtime 修改残余盲区（诚实登记）**：git status 自身对 stat-clean
   文件不重 hash（racy-clean 之外）。v2 将该盲区从「全仓库常驻盲区」收敛为
   「git status 与 stat 双 clean 才信任」；**模型自己刚写的内容由写后失效钩子
   保证可见**（这正是 v1 空转场景的闭环）。逃生阀保留作机械兜底。
5. **非 git / git 不可用回退**：无 `.git` 目录或 git 命令失败/超时 → v1
   全树 stat + size/mtime 增量语义原样保留（行为不变、可测）。
6. **参数**：`ORZ_PROJECT_DOC_INDEX_REFRESH_MS`（默认 30000；0=每次 query
   强制增量）；`ORZ_PROJECT_DOC_INDEX_FORCE_RESCAN=1` 保留；封存状态不变。

### 2.3 边界

- 符号级结构（outline/callers）不在本批（调研 §3.3-3，后续通用场景增强）。
- `retrieve_project_docs` / `project_doc_index` 主面封存不动；v2 不改变任何
  模型可见面。
- 索引是 workspace 属性（cwd 级持久化），跨 run 复用语义不变。

### 2.4 测试计划（S2）

- 基线构建：受版本控制文件入基线、tracked 标记与 blake3 内容身份正确；
- 增量层：untracked 可见、modified 重提取 + blake3 更新、deleted 消失；
- 驻留零 stat：dirty 未置 + 窗口内 query 不触发 git status（探针计数）；
- 写后失效：search_replace 后下一 query 立即可见（含同 size+mtime 构造）；
- 非 git 回退与 v1 行为一致；逃生阀全量重建；
- 快照损坏自愈 / 持久化复用（沿用既有测试）。

## 3. 设计二：会话级 tab 池 + 同轮多页并行读取

### 3.1 问题与目标

慢的根因拆解（调研 §4.1）：每调用建/关 tab（3+ CDP 往返）、单 tab 串行、
DNS 预检无缓存。第一批已解决「等待语义 + 资源拦截」；本批解决「生命周期
往返 + 同轮并行」——tab 池 + 并行 = 8 页串行 fetch 场景分钟级收敛。

### 3.2 设计定案

1. **有界 tab 池**（挂在 `CdpBrowserSession` 内）：
   - 参数 `ORZ_BROWSER_TAB_POOL_SIZE`，默认 4；`0` = 关闭池，回退现状
     （每调用 create/close）。
   - 池结构：`Vec<PooledTab { target_id, page_ws, busy, last_used }>`；
     **租约**=一次读取独占一个 tab（busy=true），归还=busy=false + 内容重置
     （导航 about:blank 或下次导航覆盖）；空闲超池按 LRU 回收。
   - 并发边界：池大小 = 最大并发读（信号量/条件变量）；池满且全 busy →
     有界等待（计入调用总预算 60s），不临时溢出创建（池有界）。
   - **隔离语义**：池复用的是 CDP target 生命周期，不共享内容/状态——每次
     租约仍是完整 navigate + 读 + 归还；模型永不接触 tab 句柄；同一 session
     单 profile 的 cookie/localStorage 跨调用共享是既有事实（操作者登录态
     复用恰是其价值，调研 §4.2），池化不引入新的状态泄漏维度。
2. **同轮多页并行**：`LocalBrowserManager` 去掉「整读持锁」——锁内租约、
   锁外读取（`inner` 改共享池 + 租约 Arc）；第一批 `PARALLEL_READ_TOOLS`
   已含 `browser_read`，并行上限 = min(池大小, 批次并行度)；同轮 8 个
   `browser_read` → N 并发 + 有界等待。
3. **下载路径不池化**：`download_or_read` 保持每调用新 target + staging 目录
   重置（下载行为状态不得跨调用复用）。
4. **DNS 预检会话级缓存（顺带项，§4.3 B 组 7）**：按 host + TTL（默认
   300s，`ORZ_BROWSER_DNS_TTL_SECS`）；**redirect 后仍重检**（保留 redirect
   重检门，ADR-0010 §3.7.3 不削弱）。
5. **CDP 方法面不变**：池化只复用既有方法（createTarget/closeTarget/
   navigate/setBlockedURLs/Runtime.evaluate），`ALLOWED_CDP_METHODS` 集合
   不新增（测试锁定不变）。

### 3.3 ADR 对齐（开放项 ①）

ADR-0010 §3.7.6 条 6「只控制自己创建或用户显式移交的 tab」与 cdp.rs 注释
「Tabs live only for the duration of one read_page call（created and closed
inside it）」。**推荐**：ADR-0010 补写 v1.46——tab **target 可机械池化复用**，
每次调用独占租约、归还即重置、模型不可见句柄、内容/控制权不跨调用共享；
「one tab lives exactly for this call」语义从「target 生灭」调整为「一次
调用独占一个 target 的控制权」。需用户裁决（ADR 变更纪律）。

### 3.4 测试计划（S2）

- 租约互斥：并发 2 读各自独占 tab（busy 计数）；
- 池上限：N=2 时 3 并发 → 第 3 有界等待；
- LRU 回收：池满归还后新调用复用最近最少使用；
- 同轮并行：≥2 browser_read 实际并发（manager 无整读锁，并发峰值 ≥2）；
- 隔离语义：归还后页面状态重置；下载路径不池化；
- `ALLOWED_CDP_METHODS` 集合断言不变；`ORZ_BROWSER_TAB_POOL_SIZE=0` 回退
  现状路径。

## 4. 设计三：委托契约复杂度分档

### 4.1 问题与目标

委托契约四要素（目标 + 输出格式 + 工具/来源 + 任务边界）已在
`build_retrieval_task_goal` + 子代理提示词中覆盖（调研 §3.1 Anthropic
对照）。缺的是 **effort 分档**：单一默认预算（600s / 60 轮）对简单查询过宽
（空转等待）、对复杂调研可能偏紧。目标：机械映射任务复杂度 → 档位 → 预算，
模型零改动。

### 4.2 设计定案

1. **三档**：`standard` / `extended` / `deep`（Anthropic 二档 + 中间档）。
2. **纯函数机械映射** `classify_retrieval_effort(query, scope, max_results,
   lane) -> EffortTier`（确定性、可解释、无模型参与、表驱动单测）：
   - query 长度（字符）：≤200 → +0；200–800 → +1；>800 → +2；
   - 广度/聚合词面（实现 `BREADTH_WORDS` 12 词：调研/调查/比较/对比/
     分析/总结/综述/全部/所有/多个/各方面/各种 命中）：每命中 +0.5
     （上限 +1；裸「各」太泛、易误命中「各自/各阶段」等，不采用）；
   - scope 形态：显式窄 scope → +0；无 scope 或含通配/多目录 → +1；
   - max_results：>10 → +1；5–10 → +0.5；<5/缺省 → +0（审查处理
     2026-08-30 裁决：模型未传 max_results 时档位默认 5/8/12 会机械并入
     契约，执行面恒有界，故「无界 → +1」不适用——设计正文与实现统一）；
   - lane：外部 web 检索 → +1；内部文档 → +0；
   - 阈值（连续边界，0.5 粒度分数同按此裁决）：≤1.0 → standard；
     ≤3.0 → extended；>3.0 → deep（1.5/3.5 分别归 extended/deep；
     审查处理 2026-08-30 将 §9 登记落回正文，消除正文/§9 矛盾）。
   - env 强制覆盖：`ORZ_RETRIEVAL_EFFORT=standard|extended|deep`
     （评测/对拍用；显式设置优先于档位默认）。
3. **档位 → 预算参数**（上限语义；既有 env 显式设置优先）：

   | 参数 | standard | extended | deep |
   |---|---|---|---|
   | 墙钟上限 | 240s | 600s（现状默认） | 900s |
   | 轮数上限 | 30 | 60（现状默认） | 90（仍受主车道 min 约束） |
   | max_results 默认（模型未传时） | 5 | 8 | 12 |
   | 同轮浏览器并行 | min(池,2) | min(池,4) | 池大小 |

   - 既有 `ORZ_RETRIEVAL_SUBAGENT_TIMEOUT_SECS` / `ORZ_RETRIEVAL_MAX_TOOL_ROUNDS`
     显式设置 > 档位默认（评测对拍不漂移）。
   - **不扩大 [DOC]/[SOURCE] 回传与注入预算**（16 行 / 8K 既定边界不动），
     档位只调执行预算，不调输出上限（边界纪律）。
4. **预算语义**：档位是上限；预算耗尽 = 终态关闭（`budget_exhausted`）语义
   不变；standard 提前截断 = 简单任务不空转；deep 90 轮仍低于 ADR-0010
   §3.7.7 的 120 轮子代理预算。
5. **事件面登记（开放项 ②）**：推荐 `retrieval_close_record` 增可选
   `effort` 字段（schema 先行 + verifier + fixtures + 防回归测试，遵循
   GAP-EVENT-SCHEMA-DRIFT 教训）；若选最小改动可不登记（effort 仅内存审计，
   以实际轮数/墙钟体现）。

### 4.3 测试计划（S2）

- `classify_retrieval_effort` 表驱动单测（长度/词面/scope/max_results/lane
  边界与阈值）；
- 档位 → 预算解析（env 覆盖、显式优先、0=禁用语义保持）；
- 集成：standard 简单查询按 240s/30 轮执行；deep 放行 90 轮（或按主车道
  min）；`budget_exhausted` 终态不受档位影响；
- 若采纳 close record `effort`：schema/verifier/fixture 一致 + 防回归。

## 5. 开放项（2026-08-30 已裁决：用户「按建议落实」，见 ADR-0010 §14.46）

1. **tab 池与 ADR 措辞**：是否按 §3.3 补写 ADR-0010 v1.46（推荐）；或维持
   ADR 原文、将池化登记为 §3.7.6 条 6 的机械实现注记（不叫补写）。
   → **已按推荐落实**：ADR §14.46 记录补写，正文 §3.7.6 条 6 加内嵌
   补写标记（审查处理 2026-08-30 收口）。
2. **close record 登记 effort**：按 §4.2-5 推荐登记（schema 先行）；或最小
   改动不登记。
   → **已按推荐落实**：schema 可选字段 + verifier 白名单 + fixtures +
   防回归（bad-effort 负例入 check_repository 映射）。
3. **默认参数表确认**：池大小 N=4、`ORZ_PROJECT_DOC_INDEX_REFRESH_MS=30000`、
   DNS TTL=300s、三档预算表（§3.2/§2.2/§4.2）与 env 命名。
   → **已确认**：与实现/ADR §14.46 参数表一致。

## 6. 不实施清单复核

- `retrieve_project_docs` / `project_doc_index` **维持封存**（主面
  `R1_SEALED_MAIN_TOOLS` 不动；v2 不改变封存状态）；
- 向量语义检索、web_search 并发 >1、浏览器 daemon 跨 run 常驻——维持暂缓；
- 符号级结构（outline/callers）——不在本批；
- tab 池不做跨调用内容共享、不做 clear-state CDP 扩展（不扩大
  `ALLOWED_CDP_METHODS`）；
- 档位不扩大 [DOC]/[SOURCE] 与注入预算上限。

## 7. 验证计划

- S1 实施 + S2 测试（上述各节测试计划；schema 项先行）；实施放行入账 +1；
- S3 重建（Linux musl 三件套，参照 `scripts/build_orz_aliyun_trixie.sh`）
  + 实机验证（检索密集题 k=1：tab 池并行场景多页读取、写后索引新鲜度、
  档位预算观察）；
- S4 闭环 -1 后按既有纪律登记。

## 8. 登记（裁决后执行）

- BACKLOG 0k（第二批设计轮状态 + S1/S2 实施入账 33 → 34）；
- TODO P0-0k（第二批三项勾选 + 实施条目）；
- ADR-0010 §14.46（裁决登记，含 tab 池措辞补写）；
- CLI_PROJECT_INDEX v2.33（FUS-RETRIEVAL-ENGINE-SERP /
  GAP-PROJECT-DOC-INDEX-CACHE 入口更新 + 头部流水）；
- 本设计文档入口。

## 9. 实施收口注记（S1 + S2，2026-08-30）

- **设计一实现细节**：git HEAD 基线以「首次构建全量扫描 + `git ls-files`
  标记 tracked 条目」表达；条目内容身份统一用 Blake3（workspace 既有依赖，
  零新增）——调研点的 blob-hash-via-cat-file 折叠为 blake3（未跟踪文件
  同样覆盖，语义等价且更简）；增量刷新对未跟踪条目做存在性核验（补 git
  status 对未跟踪删除的盲区）。
- **设计二实现细节**：LRU 语义 = 租约时刷新 `last_used`（最近使用的 tab
  最后被复用）；池有界（信号量 permits = 池大小 + 创建串行化）后无需主动
  回收，LRU 只决定空闲复用顺序；读取失败丢弃 page ws（下次租约重连）。
- **设计三实现细节**：档位阈值取 ≤1.0 → standard、≤3.0 → extended、
  >3.0 → deep（设计 §4.2 边界登记）；档位默认经「显式 env > controller
  字段（seam）> 档位默认」解析；close record `effort` 为可选字段（旧
  journal / 恢复激活不携带）。
- **验证**：orz-loop 629/0/3、orz-host 239/0/4（串行全绿；并行仅既有
  `call_tool_timeout_kills_process_tree` flake，单跑通过）、orz-bin
  12/0/12、runtime pytest 325、fmt/clippy 无新增告警；S3 重建 + 实机
  验证待续。

## 10. 审查处理注记（S1 全面审查 + 处理，2026-08-30）

第二批 S1 全面审查（设计/实现/符合性三维）处理结论如下；设计文本已在
§4.2/§2.2/§5 同步修正，代码修复已入 orz 后续提交。

**P1（规格/安全）**

- **max_results 缺省语义统一**：裁决「模型未传 max_results 时档位默认
  5/8/12 机械并入契约、执行面恒有界」，故「无界 → +1」不适用，正文改为
  `<5/缺省 → +0`（§4.2）；实现保持 None → +0 不变。
- **档位阈值连续边界落回正文**：§4.2 原「0–1/2–3/≥4」与 §9「≤1.0/≤3.0/
  >3.0」矛盾（1.5/3.5 未定义），正文已改为连续边界并注明归属。
- **D1-1 路径信任回归收口**：git 驻留快路径与增量未变条目原先直接信任
  磁盘缓存 `path`（include_content 可被缓存篡改引向工作区外任意路径）；
  已改为缓存加载时用 `cwd + relative_path` 重建 path（零 IO），并新增
  git 模式缓存篡改测试。

**P2（边界登记/闭环）**

- **HEAD 移动盲区闭环**：git status 对 clean 工作树的新提交文件不可见
  （HEAD 移动/分支切换）；git_incremental 刷新时重取 `git ls-files` 合并
  缺失 tracked 条目并校正 tracked 标记（index 只读、无工作树 stat）。
- **git 命令超时**：设计 §2.2.5「失败/超时 → 回退 v1」的「超时」原先未
  实现；统一 `run_git_cmd` 带 10s 超时（超时尽力 kill 子进程并回退 v1）。
- **子模块内容盲区登记**：git 模式下 `git ls-files` 不列子模块内部文件、
  git status 只报 gitlink 指针——子模块内文档变更不可察（存在性核验只对
  untracked 条目且不重算 blake3）；登记为已知边界，逃生阀兜底（D:\CLI
  自身含 orz 子模块，解封前留意）。
- **DNS TTL 内重绑定取舍登记**：同一 host 在 TTL 内从公开解析变为私有/
  metadata 地址时初始导航预检门被跳过（redirect 重检门保留）；收敛方案
  （缓存解析 IP 集合 + 变化重检）不在本批，登记为已知边界。
- **git_mode 进程内永久降级登记**：git 命令失败后本进程不再重探 git
  模式（v1 回退直到进程重启），属保守取舍。

**P3（小项）**

- 池化读取错误保真：lease_tab 真实错误（create_target/浏览器 ws 连接失败）
  不再统一映射为 TotalTimeout，仅创建超时映射之。
- effort env 解析抽纯函数 `parse_retrieval_effort`，单测不再依赖进程环境。
- 词面表与 `--no-renames` 命令形态、基线 blake3 描述已在 §2.2/§4.2 正文
  统一；`to_remove` 的 Present 非 doc 死分支已简化。
- 测试覆盖注记：loop 级「≥2 browser_read 真实 wall-clock 重叠」断言未补
  （需真实浏览器夹具，成本高）；档位参数映射 + 池级租约/信号量单测已覆盖
  机制，S3 实机验证计划含「同轮多页并行读取（并发峰值观察）」补足。
- 验证：审查处理后 orz-loop / orz-host 全量回归 + runtime pytest 325
  复跑全绿（详见提交信息与登记）。
