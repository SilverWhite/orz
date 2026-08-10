# project_doc_index 索引缓存/增量扫描 实施审计（2026-08-11，含三面审查修复批）

- 审计对象：GAP-PROJECT-DOC-INDEX-CACHE — `project_doc_index` 内部检索工具的索引缓存 + 增量扫描切片
- 日期：2026-08-11
- 范围：`orz/crates/orz-host`（project_doc_index.rs 重写 / retention.rs 清扫集成 / lib.rs 字段注释）+ 主仓审计与索引登记
- 用户裁决：① **进程内内存快照 + 跨 run 落盘持久化**（2026-08-11）② 三面审查（设计/实现/符合性，用户发起"全面检查"）修复批已回填本文件

## 1. 背景与缺口

| 缺口 | 登记出处 | 本切片状态 |
|---|---|---|
| `project_doc_index` 每 query 全树全扫（无缓存/无增量） | `GAP_RETRIEVAL_TOOLS_IMPL_AUDIT_2026-08-10.md` §4 L81：「project_doc_index 索引缓存/增量扫描（每 query 全扫，正确性优先）」 | **闭合** |
| 模块头自带边界声明 "cached/mtime-diffed index is a later optimization" | `project_doc_index.rs` 旧 L12-13 | **闭合**（声明改写为已实现机制 + 新边界） |

ADR-0010 无缓存/增量/失效语义条款（全仓 grep 确认）——设计空间空白，本切片自建设计并登记。工具参数契约（`query`/`include_content`/`max_results`/`max_content_bytes`）、门禁归属（retrieval mode off 投影/拒绝、ReadOnly、`access_kind=Read(None)`）、证据语义（截断→partial、`content_sha256` 前 N 字节摘要）逐项保持（GAP-RETRIEVAL-TOOLS 边界登记：工具参数契约随创建冻结）。

## 2. 用户裁决

- **D-1 持久化范围**：进程内内存快照 + 跨 run 落盘持久化（非仅 per-run）——每个新 run 首查只 stat 不重读未变文件，大工作区每次 prompt 都受益
- **D-2 失效策略**：每次 query 仍做全树 stat 遍历（零内容读），与快照按 `size + mtime(secs, nanos)` 判等 diff——结构增删由 stat 遍历保证永不漏；未变化文件零重读
- **D-3 正确性优先**：任何缓存失败（损坏/版本不匹配/指纹不匹配/不可写）退化为全量重建——查询结果永不因缓存改变，最多变慢

## 3. 实现清单

### 3.1 数据结构（project_doc_index.rs）

- `CachedEntry` = DocEntry + `mtime_secs: u64` + `mtime_nanos: u32`（显式拆存避开 serde SystemTime 序列化漂移；nanos 保留 NTFS 100ns 粒度同秒修改判定；内部类型，无公开面）
- `CachedIndex`（落盘 envelope）：`schema_version="0.1.0-draft"`（sidecar 惯例）+ `cwd_fingerprint`（`dunce::canonicalize(cwd)`，dunce 已是主依赖；构造时算一次——唯一的构造 IO，此后每 query 零 IO）+ `built_at_secs`（信息性）+ `entries`
- `CachedSnapshot { entries }` 内存快照（Arc 交换共享，entries 恒按 relative_path 排序——结果顺序确定性）
- `ProjectDocIndex` 新字段：`fingerprint`、`state: Mutex<Option<Arc<CachedSnapshot>>>`（Option 区分"未构建"与"空工作区"——空快照不重复 load/persist，审查 D2-3 修复）、`force_rescan: AtomicBool`

### 3.2 增量扫描（refresh）

- `stat_pass(cwd) -> (Vec<FileStat>, bool)`：全树 DFS 元数据遍历——排除目录/扩展名白名单/符号链接语义与旧 `discover()` 逐字一致；`metadata.modified()` 失败记 `(0,0)` → 每次 diff 视为变化 → 保守重提取（登记取舍）；返回元组第二项=根目录可读性（根不可读时空结果非权威——审查 P2-2 修复：调用方不删除条目不持久化）
- `refresh()`（锁内单临界区，无 await）：首建惰性 `load_cache`（逃生阀生效时跳过）→ stat 遍历（**每 query 按 D-2 执行**；并发第二调用者复用快照、跳过重提取与 persist，但仍付 stat 遍历——审查 D2-1/C2-2 措辞修正）→ diff（`size+mtime` 全等且 mtime 非 (0,0) 哨兵才复用——审查 P2-1 修复；复用条目 `path` 恒从 walk 重建——审查 D1-1 修复；变化 `entry_from_stat` 重提取；缺失删除）→ 重排序 → 脏则 `persist_cache` → Arc 交换
- 并发：`Mutex<Option<Arc<CachedSnapshot>>>`——锁内 stat+diff+extract+persist（首建竞争合并）；锁外匹配与 content 读取（16KB×100 磁盘读不阻塞另一 lane）；中毒恢复 `unwrap_or_else(|p| p.into_inner())`；跨进程无锁 last-writer-wins（登记边界）

### 3.3 持久化

- 路径 `{cwd}/.gsa/project-doc-index/cache.json`（cwd 级非 session8——索引是工作区属性；`.gsa` 在 EXCLUDED_DIRS 免自指污染）
- load：NotFound/读取失败→静默 None；解析失败/schema 不匹配/指纹不匹配→`tracing::warn!` + None（全量重建）；绝不 panic
- persist：diff 后立即 best-effort（稳态零写放大；崩溃丢失窗口=只丢本次 diff——磁盘是**安全 diff 基线**（stale 只导致多余重提取、永不致错），进程内存快照才是更新权威；非原子覆盖写（sidecar 先例，损坏自愈）
- 逃生阀：`ORZ_PROJECT_DOC_INDEX_FORCE_RESCAN=1`（跳过 load + 忽略旧快照全量重提取 + 照常 persist 覆盖）+ `set_force_rescan(bool)` 测试 seam（测试不碰 env 防并行互踩）

### 3.4 retention 集成（retention.rs）

- `PruneReport` 加 `removed_project_doc_caches: Vec<String>`；`prune_old_records` 追加 `prune_old_files(project-doc-index)` 清扫（7 天按 mtime，与 conversations 同构）
- 判据：缓存 100% 可重建 + best-effort 写 → 删除永远安全（与 keystore「永不扫」的例外互补——可重建才可扫；sidecar 先例引用属"结构同构"而非"失败处理同构"——orientation 对损坏 sidecar 是"warn 不丢弃"、缓存是"warn+重建覆盖"，由可重建性正当化的有意偏差，审查 D3-8 登记）

### 3.5 query/discover 改造

- `query()`：仅 `for entry in self.discover()` → `for entry in &self.refresh().entries`；参数解析/匹配/截断/摘要/序列化逐字节不变；锁外执行的匹配与 content 读取不受锁内范围变化影响
- `discover()` 保留公开 API：`refresh_forced()` 强制全量（跳过缓存与 diff）+ 结果 persist 为新基线

## 4. 决策表

| 决策 | 裁决 | 依据 |
|---|---|---|
| D-1 持久化范围 | 进程内 + 跨 run 落盘 | 用户裁决 2026-08-11；sidecar 结构先例（orientation/activation/conversation）可复用 |
| D-2 失效判定 | `size + mtime(secs,nanos)` 判等 | stat 遍历保证结构增删永不漏；mtime+size 双键覆盖内容变化；nanos 覆盖 Windows 同秒修改 |
| D-3 保存时机 | diff 后立即 best-effort persist | 崩溃丢失窗口只丢本次 diff（磁盘为安全基线）；稳态零写放大优于 run 结束存 |
| D-4 并发 | `Mutex<Option<Arc<CachedSnapshot>>>`，锁内 diff、锁外 content 读 | 双 lane 并发形态（ADR-0010 §11.3）；content 读是延迟大头不阻塞另一 lane；锁内时间 ≈ 现状串行全扫（严格说首次查询增序列化+写，量级可忽略，审查 D3-9 登记） |
| D-5 逃生阀形态 | env + AtomicBool seam，不加工具参数 | GAP-RETRIEVAL-TOOLS 边界登记：参数契约冻结；运维/诊断通道不该进模型可见 schema |
| D-6 缓存路径 | `.gsa/project-doc-index/cache.json`（cwd 级） | 索引是工作区属性非 session 属性；session8 路径会丢失跨 run 共享意义 |
| D-7 retention | 纳入 7 天清扫 | 可重建故可扫；防 `.gsa` 永不过期 orphan 破坏 A5 有限目录树不变量 |
| D-8 指纹 | `dunce::canonicalize(cwd)`，失败回落原始路径 | 与 acp_server.rs:1513 既有先例一致；路径大小写变化触发一次冗余重建（无害登记） |

## 5. 验证（修复批后复验）

- `cargo test -p orz-host project_doc_index`：**15 passed**（3 旧 + 12 新：N1-N9 + N10 unreadable_root + N11 tampered_cache_path）
- `cargo test -p orz-host`：**171 passed / 3 ignored**（158 → 171，+13；修复批前 169 曾连续 3 次全绿，首次 168+1 failed 为并发测试偶发、重跑即过——未修根因，登记后续排查）
- `cargo test -p orz-loop`：**170 passed / 3 ignored**（门禁/证据记录零回归）
- `cargo test --workspace`（-j2）：**167 个测试目标全部 ok，0 failed**（orz-tools 2711/6、orz-workspace 1498/11 等）
- `cargo test -p orz-bin -j2 -- --ignored`：**13 passed**（conformance capture 端到端真实工具链路）
- `cargo clippy -p orz-host --all-targets`：**零新增**（4 处 let-chain/无用转换已修；其余 warning 全部 pre-existing 于其他 crate——orz-config 1 / xai-fast-worktree 2 / orz-loop 1，与 P3-1 基线一致）
- `python -m pytest assurance -q`（cwd=仓库根 `D:\CLI`）：**1607 passed / 13 skipped / 0 failed**（6m30s；该数字为 assurance 目录收集量——根目录全量收集另含 runtime/integration 组，范围已标注）。注：非根目录 cwd 下 `test_task_contract_records_absolute_source_ledger_path` 因相对路径解析失败（FileNotFoundError——D3-7 登记「pytest 须根目录跑」同根因，症状表现为相对路径解析失败/scripts.* 导入失败两种形态）——运行环境问题，非本切片引入（本切片零 Python 改动）
- `python scripts/check_repository.py`：**valid**
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
9. 已知盲区：同 size + 同 mtime 的内容修改不可察（逃生阀兜底）；**该盲区的全树放大变体**（rsync -a / robocopy /MIR 等保留 mtime 的整体替换使陈旧从单文件放大到全树——逃生阀兜底，审查 D3-4 登记）；FAT32 2 秒 mtime 粒度下同秒内修改不可察（同盲区覆盖，审查 D3-5 登记）；Windows 路径大小写变化触发一次冗余重建（无害）；`metadata.modified()` 失败文件每次重读（保守正确）；`metadata()` 失败（文件中途消失/权限）文件整体不入索引——旧 discover 会以 size=0 收录，更保守的有意行为（审查 C3-4 登记）；`discover()` 强制全量每次 persist（罕见路径——仅测试调用，写放大可接受）；符号链接环路无检测（`is_dir()` 跟随，pre-existing 于旧 discover，审查 D3-6 登记）

## 7. 三面审查修复批（2026-08-11，用户发起"全面检查"）

三独立代理审查（设计合理性 / 实现合理性 / 符合性）：**无 D1/P1/C1 级正确性冲突**（D1-1 一处信任面缺陷已按 D1 级修复）。修复批 7 项已应用并复验：

| 编号 | 级别 | 发现 | 修复 |
|---|---|---|---|
| D1-1 | 必须修 | 缓存条目 `path` 被无条件信任——篡改 cache.json 可使 `include_content` 读工作区外任意文件（越权读）+ 伪造命中，破坏"host-owned 工具结构上只读工作区"不变量 | **reuse 分支 `path` 恒从 walk 重建**（缓存只贡献 title/headings）；模块头登记"缓存 path 永不信任"；新测试 `tampered_cache_path_is_not_read`（注入指向外部文件的篡改条目，断言 secret 内容不可见） |
| P2-1 | 必须修 | `modified()` 失败记 `(0,0)` 哨兵被 diff 复用——mtime 缺失平台首次构建后文件**永不重提取**，与登记的"每查询保守重提取"边界相反 | 复用条件加 `(st.mtime_secs != 0 \|\| st.mtime_nanos != 0)` 排除哨兵；注释登记 |
| P2-2 | 必须修 | 根目录瞬时不可读 → stat_pass 空 → 全部条目判 removed → 空快照持久化**覆盖完好缓存** | `stat_pass` 返回 `(Vec, root_ok)`；根不可读时沿用旧快照、不删除不 persist；新测试 `unreadable_root_keeps_snapshot_and_cache`（rename 工作区，断言快照照常服务且缓存 mtime 不变） |
| D2-1/C2-2 | 建议修 | 「第二 lane 不重复扫描」声明与实现不符（stat_pass 无条件执行）且与 D-2 自相矛盾 | 代码注释与审计 §3.2 改为「第二 lane 不重复重提取/不重复 persist（stat 遍历仍按 D-2 每 query 执行）」 |
| D2-2 | 建议修 | query() 注释锁内范围不完整（遗漏 changed-file re-extract 与 persist） | 注释改为「metadata walk + diff + changed-file re-extract + best-effort persist hold it」 |
| D2-3 | 建议修 | 空工作区快照恒空 → 「未构建」判定恒真 → 每 query 重复 load/persist（空 envelope 写盘） | 哨兵改 `Option<Arc<CachedSnapshot>>`（None=未构建），一次修复消除重复 load 与重复 persist；load 成功且无变更时不再回写磁盘 |
| C2-1 | 建议修 | 审计 D-5 依据「决策 1」悬空引用（全仓无可解析对象） | 改「GAP-RETRIEVAL-TOOLS 边界登记」 |
| C2-3 | 建议修 | pytest 数字 1607 未标注收集范围（为 assurance 目录量，非根目录全量 1836） | §5 标注确切命令 `python -m pytest assurance -q`（cwd=仓库根）与范围说明 |
| P3-1/D3-7 | 建议修 | `entry_from_stat` 的 size 来自第二次 stat（跨 TOCTOU 混入新旧三元组——方向恒保守） | `size: st.size` 单一来源 |
| P3-2 | 建议修 | `CachedEntry` 无公开 API 暴露却 pub | 去 pub（内部类型） |
| P3-3 | 记录 | 悬空符号链接带 doc 扩展名：旧 walk 索引 size=0 幽灵条目，stat_pass 跳过——行为更优但注释"mirror"不准确 | 模块头登记该有意差异 |
| P3-4 | 记录 | fingerprint 注释「构造零 IO」不实（canonicalize 是构造 IO） | 改「构造时算一次——唯一的构造 IO；每 query 零 IO」 |
| P3-5 | 记录 | N2 注释字节数错（9→10） | 修正 |
| P3-6 | 记录 | `removed` 变量实为「未复用」计数（仅驱动 persist 脏判定） | 改名 `not_reused` + 注释 |
| C3-2 | 记录 | load_cache 注释称 read 失败 warn——实为静默 None | 注释改「NotFound/读取失败→静默」 |
| D3-1 | 登记 | 崩溃窗口措辞「磁盘是唯一事实源」不精确 | 改「磁盘是安全 diff 基线」 |
| D3-2 | 登记 | 稳定活跃工作区每 7 天一次全量重建（retention 清缓存→首查重建→立即写回→周期闭合） | 本文件 §6 登记 |
| D3-3 | 登记 | removed 计数语义、锁内 persist 严格性（≈ 非 ≤）、并发偶发失败未修根因 | 本文件 §4/§5 登记 |

修复后复验：project_doc_index **15 passed**（+2 新测试）、orz-host **171 passed / 3 ignored**、clippy 零新增（orz-host 自身零 warning，其余 3 crate 与 P3-1 pre-existing 基线一致）、`git diff --check` 干净。修复仅涉生产代码行为（复用分支 path 重建 / 哨兵排除 / root_ok 保护）与文档措辞，工具参数契约与门禁零变化。
