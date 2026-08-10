# project_doc_index 索引缓存/增量扫描 实施审计（2026-08-11）

- 审计对象：GAP-PROJECT-DOC-INDEX-CACHE — `project_doc_index` 内部检索工具的索引缓存 + 增量扫描切片
- 日期：2026-08-11
- 范围：`orz/crates/orz-host`（project_doc_index.rs 重写 / retention.rs 清扫集成 / lib.rs 字段注释）+ 主仓审计与索引登记
- 用户裁决：**进程内内存快照 + 跨 run 落盘持久化**（2026-08-11）

## 1. 背景与缺口

| 缺口 | 登记出处 | 本切片状态 |
|---|---|---|
| `project_doc_index` 每 query 全树全扫（无缓存/无增量） | `GAP_RETRIEVAL_TOOLS_IMPL_AUDIT_2026-08-10.md` §4 L81：「project_doc_index 索引缓存/增量扫描（每 query 全扫，正确性优先）」 | **闭合** |
| 模块头自带边界声明 "cached/mtime-diffed index is a later optimization" | `project_doc_index.rs` 旧 L12-13 | **闭合**（声明改写为已实现机制 + 新边界） |

ADR-0010 无缓存/增量/失效语义条款（全仓 grep 确认）——设计空间空白，本切片自建设计并登记。工具参数契约（`query`/`include_content`/`max_results`/`max_content_bytes`）、门禁归属（retrieval mode off 投影/拒绝、ReadOnly、`access_kind=Read(None)`）、证据语义（截断→partial、`content_sha256` 前 N 字节摘要）逐项保持。

## 2. 用户裁决

- **D-1 持久化范围**：进程内内存快照 + 跨 run 落盘持久化（非仅 per-run）——每个新 run 首查只 stat 不重读未变文件，大工作区每次 prompt 都受益
- **D-2 失效策略**：每次 query 仍做全树 stat 遍历（零内容读），与快照按 `size + mtime(secs, nanos)` 判等 diff——结构增删由 stat 遍历保证永不漏；未变化文件零重读
- **D-3 正确性优先**：任何缓存失败（损坏/版本不匹配/指纹不匹配/不可写）退化为全量重建——查询结果永不因缓存改变，最多变慢

## 3. 实现清单

### 3.1 数据结构（project_doc_index.rs）

- `CachedEntry` = DocEntry + `mtime_secs: u64` + `mtime_nanos: u32`（显式拆存避开 serde SystemTime 序列化漂移；nanos 保留 NTFS 100ns 粒度同秒修改判定）
- `CachedIndex`（落盘 envelope）：`schema_version="0.1.0-draft"`（sidecar 惯例）+ `cwd_fingerprint`（`dunce::canonicalize(cwd)`，dunce 已是主依赖）+ `built_at_secs`（信息性）+ `entries`
- `CachedSnapshot { entries }` 内存快照（Arc 交换共享，entries 恒按 relative_path 排序——结果顺序确定性）
- `ProjectDocIndex` 新字段：`fingerprint`（构造时算一次，构造零 IO）、`state: Mutex<Arc<CachedSnapshot>>`、`force_rescan: AtomicBool`

### 3.2 增量扫描（refresh）

- `stat_pass(cwd)`：全树 DFS 元数据遍历——排除目录/扩展名白名单/符号链接语义与旧 `discover()` 逐字一致；`metadata.modified()` 失败记 `(0,0)` → 每次 diff 视为变化 → 保守重提取（登记取舍）
- `refresh()`（锁内单临界区，无 await）：首建惰性 `load_cache`（逃生阀生效时跳过）→ stat 遍历 → diff（`size+mtime` 全等复用 / 变化 `entry_from_path` 重提取 / 缺失删除）→ 重排序 → 脏则 `persist_cache` → Arc 交换
- 并发：`Mutex<Arc<CachedSnapshot>>`——锁内 stat+diff+extract+persist（第二 lane 不重复扫描、首建竞争合并）；锁外匹配与 content 读取（16KB×100 磁盘读不阻塞另一 lane）；中毒恢复 `unwrap_or_else(|p| p.into_inner())`；跨进程无锁 last-writer-wins（登记边界）

### 3.3 持久化

- 路径 `{cwd}/.gsa/project-doc-index/cache.json`（cwd 级非 session8——索引是工作区属性；`.gsa` 在 EXCLUDED_DIRS 免自指污染）
- load：NotFound→None 静默；损坏/schema 不匹配/指纹不匹配→`tracing::warn!` + None（全量重建）；绝不 panic
- persist：diff 后立即 best-effort（稳态零写放大；崩溃丢失窗口=只丢本次 diff，磁盘文件是唯一事实源，零正确性损失）；非原子覆盖写（sidecar 先例，损坏自愈）
- 逃生阀：`ORZ_PROJECT_DOC_INDEX_FORCE_RESCAN=1`（跳过 load + 忽略旧快照全量重提取 + 照常 persist 覆盖）+ `set_force_rescan(bool)` 测试 seam（测试不碰 env 防并行互踩）

### 3.4 retention 集成（retention.rs）

- `PruneReport` 加 `removed_project_doc_caches: Vec<String>`；`prune_old_records` 追加 `prune_old_files(project-doc-index)` 清扫（7 天按 mtime，与 conversations 同构）
- 判据：缓存 100% 可重建 + best-effort 写 → 删除永远安全（与 keystore「永不扫」的例外互补——可重建才可扫）

### 3.5 query/discover 改造

- `query()`：仅 `for entry in self.discover()` → `for entry in &self.refresh().entries`；参数解析/匹配/截断/摘要/序列化逐字节不变
- `discover()` 保留公开 API：`refresh_forced()` 强制全量（跳过缓存与 diff）+ 结果 persist 为新基线

## 4. 决策表

| 决策 | 裁决 | 依据 |
|---|---|---|
| D-1 持久化范围 | 进程内 + 跨 run 落盘 | 用户裁决 2026-08-11；sidecar 先例（orientation/activation/conversation）可完整复用 |
| D-2 失效判定 | `size + mtime(secs,nanos)` 判等 | stat 遍历保证结构增删永不漏；mtime+size 双键覆盖内容变化；nanos 覆盖 Windows 同秒修改 |
| D-3 保存时机 | diff 后立即 best-effort persist | 崩溃丢失窗口只丢本次 diff（磁盘为唯一事实源）；稳态零写放大优于 run 结束存 |
| D-4 并发 | `Mutex<Arc<CachedSnapshot>>`，锁内 diff、锁外 content 读 | 双 lane 并发形态（ADR-0010 §11.3）；content 读是延迟大头不阻塞另一 lane；锁内时间 ≤ 现状串行全扫 |
| D-5 逃生阀形态 | env + AtomicBool seam，不加工具参数 | 参数契约冻结（决策 1）；运维/诊断通道不该进模型可见 schema |
| D-6 缓存路径 | `.gsa/project-doc-index/cache.json`（cwd 级） | 索引是工作区属性非 session 属性；session8 路径会丢失跨 run 共享意义 |
| D-7 retention | 纳入 7 天清扫 | 可重建故可扫；防 `.gsa` 永不过期 orphan 破坏 A5 有限目录树不变量 |
| D-8 指纹 | `dunce::canonicalize(cwd)`，失败回落原始路径 | 与 acp_server.rs:1513 既有先例一致；路径大小写变化触发一次冗余重建（无害登记） |

## 5. 验证

- `cargo test -p orz-host project_doc_index`：**13 passed**（3 旧 + 10 新）
- `cargo test -p orz-host`：**169 passed / 3 ignored**（158 → 169，+11；连续 3 次全绿，首次 168+1 failed 为并发测试偶发、重跑即过）
- `cargo test -p orz-loop`：**170 passed / 3 ignored**（门禁/证据记录零回归）
- `cargo test --workspace`（-j2）：**167 个测试目标全部 ok，0 failed**（orz-tools 2711/6、orz-workspace 1498/11 等）
- `cargo test -p orz-bin -j2 -- --ignored`：**13 passed**（conformance capture 端到端真实工具链路）
- `cargo clippy -p orz-host --all-targets`：**零新增**（4 处 let-chain/无用转换已修；其余 warning 全部 pre-existing 于其他 crate）
- `python scripts/check_repository.py`：**valid**
- pytest（从仓库根目录 `D:\CLI`）：**1607 passed / 13 skipped / 0 failed**（6m30s）。注：非根目录 cwd 下 `test_task_contract_records_absolute_source_ledger_path` 因相对路径解析失败（D3-7 登记「pytest 须根目录跑」）——运行环境问题，非本切片引入（本切片零 Python 改动）
- `git diff --check`：干净；orz 3 文件 +644/-38（lib.rs 注释 8 行 / project_doc_index.rs 重写 / retention.rs +40）

## 6. 登记边界（本切片不做）

1. 不碰 `lib.rs` call_tool 超时包装（project_doc_index 同步特例在 P0-1 之前返回——既有登记，不修）
2. 不做跨进程锁（多 session 同 cwd 并发写 cache = last-writer-wins + 自愈 diff）
3. 不引入文件系统事件监听（stat 遍历是强制基础；事件驱动属后续切片）
4. 不缓存 query 结果本身（缓存只覆盖索引元数据提取；每次 query 仍完整匹配 + 可选内容读取）
5. 不做原子写（temp+rename）——跟随 sidecar 非原子写先例，损坏自愈
6. 不改工具参数契约、门禁、权限、证据记录代码（§1 清单逐项核实保持）
7. 不做缓存 size 上限/截断（10 万文件级 mtime+size+headings 元数据量级可接受；压缩属后续）
8. 不改 `content_sha256` 语义（「前 max_content_bytes 字节摘要」逐字保留）
9. 已知盲区：同 size + 同 mtime 的内容修改不可察（逃生阀兜底）；Windows 路径大小写变化触发一次冗余重建（无害）；`metadata.modified()` 失败文件每次重读（保守正确）；`discover()` 强制全量每次 persist（罕见路径，写放大可接受）
