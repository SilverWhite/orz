# PDF 证据管线 实施审计（2026-08-11，含三面审查修复批）

- 审计对象：GAP-PDF-EVIDENCE — 内容寻址 PDF 证据管线切片（轨道 A 最后一切片）
- 日期：2026-08-11
- 范围：`orz/crates/codegen/orz-tools`（pdf_evidence.rs 证据核心 + web_fetch 直连内联）+ `orz/crates/orz-host`（白名单路由/浏览器下载通道/pdf_read/retention）+ `orz/crates/orz-loop`（relay 门禁 + evidence 记账）+ 主仓审计与索引登记 + ADR-0010 v1.4 补写（§3.7 条 11 + §14.4）
- 用户裁决：① **双通道路由**——env 通配域名白名单 `ORZ_PDF_BROWSER_DOMAINS`（`*.cnki.net`）命中走浏览器 CDP 下载（登录态访问学校文献库），未命中自动走 host 直连；未配置=全直连 ② **白名单内浏览器失败显式失败**，不自动回退直连 ③ **工具形态=内联+pdf_read**——web_fetch 检测 PDF 自动走管线返回前段文本+document_id；新增 host-owned `pdf_read(document_id, page_range)` 按页读已入库证据 ④ **pdf-evidence 纳入 7 天 retention 清扫**（rebuildable → sweepable）
- 三面审查（2026-08-11，用户发起"全面检查"）：设计合理性/实现合理性/符合性三独立代理——**无 P0**；P1×3 + D1×1 + C1×1 已修复（浏览器下载通道事件机制/落地 URL 归因/live 验证缺失 + 记账误判 + 文档引用）；修复批见 §7

## 1. 背景与缺口

| 缺口 | 登记出处 | 本切片状态 |
|---|---|---|
| PDF 证据管线未实施（`INVALID_PDF`/`NO_TEXT_LAYER` 等失败码无生产实现） | `CLI_PROJECT_INDEX.md` EVIDENCE-LOCAL-BROWSER（`partial`）：「PDF 管线是后续切片」 | **闭合** |
| local_browser MVP 声明 "NO PDF pipeline" | `local_browser/mod.rs` 旧 L5-9 | **闭合**（声明改写为已实现机制 + 新边界） |
| web_fetch 下载 PDF 只落盘返回短提示 → evidence 误判 `full_text_observed`（短提示非文档文本） | controller.rs `build_evidence_record`（现存 bug，2026-08-11 审查发现） | **闭合**（legacy 提示→`metadata_only` + PDF marker 分支） |

ADR-0010 §3.7 转录的规范性裁决（条 2：显式状态与失败码；条 6：PDF-first 来源优先下载为内容寻址本地证据，页文本/metadata/extraction record 可重建；条 7：下载大小限制）已落地；`version_guess`/`document_id` 等实现细节未转录（旧设计文档为 historical 参考），本切片自建设计并登记。

## 2. 用户裁决（2026-08-11）

- **D-1 下载通道**：双通道可配置路由——`ORZ_PDF_BROWSER_DOMAINS`（env，逗号分隔，`*` 通配子域；`*.cnki.net` 匹配 `kns.cnki.net` 但不匹配 apex `cnki.net`，测试锁定）命中 → 浏览器 CDP 下载（登录态文献库）；未命中 → host 直连（web_fetch 内联管线）。未配置 = 全直连（默认安全面）
- **D-2 白名单内失败**：浏览器下载失败（未登录/付费墙/超时/canceled）→ 显式 `[web_fetch_pdf_*]` 错误码，**不自动回退直连**（§3.7.2 显式状态）
- **D-3 工具形态**：web_fetch 检测 `application/pdf` → 完整管线（校验→sha256→pdf_oxide 逐页抽文本→证据入库→返回前段文本+document_id）；新增 host-owned `pdf_read(document_id, page_range)`（1-based、"3"/"1-5"/"5-"、≤20 页/调用、跨 turn/run 复用不重下载）
- **D-4 retention**：`pdf-evidence/{p2}/{full64}/` 按目录龄 7 天清扫（rebuildable → sweepable，project-doc-index 先例）；`pdf-downloads-*` staging 同步清扫
- **D-5 提取库**：pdf_oxide 0.3.43（workspace 已有，orz-tools read_file 生产使用）——**零新第三方依赖**（orz-tools 仅加 `sha2`，workspace 已 pin 0.10 force-soft）
- **D-6 schema**：零 run-event 变更——`EvidenceRecord.source_type` 自由 String（`"pdf_document"`）+ `content_sha256`/visibility 四档已有

## 3. 实现清单

### 3.1 证据核心（orz-tools `implementations/pdf_evidence.rs`，新建）

- **布局**：`{root}/{sha256[..2]}/{sha256_full}/{original.pdf, pages.jsonl, metadata.json}`；`document_id = "sha256:{64hex}"`；metadata.json **最后写 = 完成标记**（torn 写自愈：重 ingest 幂等覆盖）
- **envelope**：`PdfMetadata{schema_version="0.1.0-draft", document_id, source_url, downloaded_at(RFC3339), sha256, bytes, pages, has_text_layer, extraction_status("ok"|"no_text_layer"), parser, total_chars}`（侧车纪律：schema_version 首字段）
- **校验链**：`MAX_PDF_BYTES=50MB`（对齐 read_file/pdf.rs）→ `%PDF` magic（复用 `read_file::metadata::is_pdf_magic`）→ `PdfDocument::from_bytes` + `page_count()`（0 页拒绝）→ 逐页 `extract_text`（`PDF_PROCESS_TIMEOUT=60s` spawn_blocking + timeout）——**HTML 伪装 .pdf（magic 缺失）硬拒绝**
- **NO_TEXT_LAYER 语义**：全页空白文本 → **非硬失败**（Ok + `has_text_layer=false` + `extraction_status="no_text_layer"` + 页文本空行入库）——调用方显式表达 metadata_only，满足 §3.7.2 显式状态
- **读取**：`read_pages(root, document_id, page_indices)`（0-based 索引过滤，坏 JSONL → `Corrupt` 显式错误不自愈）；`read_metadata`（NotFound 显式）；`document_dir` 路径纯由 hex 派生（穿越免疫）；`parse_page_indices` 复用 read_file 语义（pub fn）
- **marker 输出契约**（跨 crate，双端测试锁定）：`PDF evidence: {N} pages, document_id=sha256:{hex}, text_layer={yes|no}` + `--- Page N ---` 页文本 + 截断 footer `[web_fetch pdf content truncated: {n} chars]`；截断上限 `MAX_RETURN_TEXT_CHARS=100_000`（对齐 browser_read）
- **错误码**：`web_fetch_pdf_invalid/too_large/timeout/invalid_document_id/not_found/invalid_page_range/corrupt/io_error`

### 3.2 直连内联（orz-tools web_fetch）

- `WebFetchParams` + `pdf_evidence_root: Option<PathBuf>`（`#[serde(default)]`；`deny_unknown_fields` 兼容旧配置）；orz-host `web_fetch_config_default(cwd)` 注入 `{cwd}/.gsa/pdf-evidence`；**其他 orz-tools host 不配 root → 保留 legacy save_pdf**（配置差异登记，非静默降级）
- `fetch_url` 字节上限判定**移到调用方**（content-type 已知后）：PDF → `MAX_PDF_BYTES(50MB)`，其余 → 配置 `max_content_length`（非 PDF 合同不变）；PDF 分支：root 配置时 `ingest_pdf_bytes` 内联（截断 preview **不进缓存**——同截断文本纪律），None 时 legacy `save_pdf`
- `WebFetchError::PdfEvidence(#[from])` 变体

### 3.3 浏览器下载通道（orz-host local_browser CDP）

- `ALLOWED_CDP_METHODS` **仅加** `Browser.setDownloadBehavior`（不传 browserContextId = 默认上下文；非 Network./Input./Storage. 域——下载行为不是网络拦截）；下载事件 `downloadWillBegin`/`downloadProgress` 是**接收事件**不进白名单（ws_reader_task 转发，best-effort；丢 completed → 显式超时 + 目录轮询自愈，不静默）
- `download_or_read(url, download_dir)`：门禁预检 → **staging 目录重置**（每次调用清空——"最新文件"语义精确，防旧残留误取）→ createTab → `Browser.setDownloadBehavior{behavior:"allow", downloadPath}`（幂等重发）→ navigate → `wait_download_or_load` 事件循环：
  - `downloadWillBegin` → pending_guid（此后 `loadEventFired` **不再是终态信号**——下载导航的空 frame 也 fire load）
  - `downloadProgress{completed}` → `wait_for_new_file` 轮询 staging ≤5s（Windows Chrome 异步落盘/文件锁重试）→ evaluate location.href 得 final_url → `Pdf{path, final_url}`
  - `{canceled}` → 显式 `DownloadCanceled`（不回退读页）
  - 无 pending 的 `loadEventFired` → 既有读路径（`extract_page` 抽取公共助手，门禁重检同 read 路径）→ `Page(outcome)`
  - 超时 → `LoadTimeout`；tab teardown 全路径 5s cap（read_page 同构）
- 白名单测试改**精确断言**（删 `Browser.` 通配 ban，`Browser.` 域仅允许精确一个方法；`Network./Input./Storage.` ban 保留；`disallowed_methods_fail_closed` 补 `Browser.getVersion`/`Browser.close` 反例）
- `BrowserSession` trait + `download_or_read` + `BrowserDownloadOutcome::{Pdf, Page}`；`UnavailableBrowserSession` 显式 Err；`StubBrowser` 扩展

### 3.4 host 路由（orz-host `pdf_evidence.rs`，新建）

- 白名单：`ORZ_PDF_BROWSER_DOMAINS` env 解析（逗号分隔、trim、尾点归一、大小写不敏感）；`domain_matches` 精确语义（`*.x` 不匹配 `x` 根；`*` 匹配一切——文档标注危险；IP 字面量仅 `*` 命中）；`set_domains_override` 测试 seam（set_force_rescan 先例，Atomic/静态 + TESTS_ENV_LOCK 防并行竞态）
- **路由汇点**：`call_tool` 拦截精确 `web_fetch` 名——参数 `url` 预检白名单命中 → `handle_browser_pdf`（不走 toolset）；未命中 → 落 toolset（直连内联管线）。拦截在 P0-1 timeout 包裹之前的分支区（browser_read 先例位置）；浏览器下载受 **CDP 自身 total_budget（60s）约束**（P0-1 300s 工具预算不覆盖拦截分支——审查 C3-5 措辞修正）
- `handle_browser_pdf`：`!browser.ready()` → `[web_fetch_pdf_browser_unavailable]`（**不回退**）→ `download_or_read`（每调用唯一 UUID staging 子目录——审查 P2-3）→ `Pdf{path}`：**先 `metadata().len()` 大小检查再整读**（审查 P2-2 防 OOM）→ `ingest_pdf_bytes`（同一直连核心）→ **全路径清理 staging**（成功/超限/ingest 失败统一——审查 P3-1）→ 输出 marker+文本；`Page` → browser_read 同构 JSON 输出（登录态网页浏览一体）
- `handle_pdf_read`：严格参数解析（未知 key 拒绝）→ `read_metadata` 存在性 + `has_text_layer` 检查 → `parse_page_indices`（None=全部页）→ `read_pages` → 拼接 `--- Page N ---` + 截断 footer `[pdf_read content truncated: ...]`；错误码 `[pdf_read_*]` 全套
- `pdf_read_tool_def`：恒声明（无 browser 依赖，project_doc_index 先例）；`relay::is_retrieval_mode_gated_host_tool` += `"pdf_read"`（mode=off 投影 + dispatch 门禁同盖）

### 3.5 evidence 记账修复（orz-loop controller.rs）

- web_fetch 分支：输出含 `document_id=sha256:` marker → **PDF 分支**——`text_layer=no` → `("pdf_document", "metadata_only", "metadata only (no text layer)", "page text")`；截断 → partial；否则 full + "extracted text layer"；**`content_sha256` 从 marker 解析文档 hex**（原始字节摘要，机械派生，非预览文本摘要）
- **legacy 修复**：`"PDF downloaded"` 短提示 → `("web_page", "metadata_only", "download metadata only", "page content")`（原误判 `full_text_observed`——下载提示不是文档文本，§3.7.5）
- `pdf_read` 分支：截断 → partial / 否则 full（"requested pages"）；identity 链加 `document_id` 参数

### 3.6 retention（orz-host retention.rs）

- `PruneReport` + `removed_pdf_evidence_dirs`（记录 full64 名）+ `removed_pdf_download_dirs`
- `prune_pdf_evidence`：两级布局最深优先（`{p2}/{full64}` 目录按龄 `entry_older_than`，删文档目录；`{p2}` shard 容器永不删——空 shard 保留，上界 256 个目录）；超龄即删不检查 metadata（审查 C3-4：fail-safe 仅存在于 stat 失败路径）
- `prune_old_dirs(gsa_root, cutoff, "pdf-downloads-")`：staging 崩溃残留兜底
- 判据：证据 100% 可重建（源 URL 可重下载）+ best-effort → 可扫；document_id 失效后 `[pdf_read_not_found]` 显式（与 conversations 超期同类语义）

## 4. 决策表

| 决策 | 裁决 | 依据 |
|---|---|---|
| D-1 下载通道 | 双通道白名单路由（env 通配域名） | 用户裁决 2026-08-11：「每个学校买的文献库不一样，需要留白名单进行范围确定，在白名单的走白名单（登录态），不在白名单的自动走直连」 |
| D-2 白名单内失败 | 显式失败不回退 | 用户裁决 2026-08-11；§3.7.2 失败不得静默降级 |
| D-3 工具形态 | 内联 + pdf_read | 用户裁决 2026-08-11；论文文本层超输出预算（10MB PDF ≈ 数百 KB），单次返回必然截断 → 分页读必需；旧设计 §5.4 read-by-page 先例 |
| D-4 retention | 纳入 7 天清扫 | 用户裁决 2026-08-11；rebuildable → sweepable（project-doc-index 先例） |
| D-5 提取库 | pdf_oxide（workspace 已有） | 零新第三方依赖；orz-tools read_file 已生产使用（99.5% PyMuPDF parity）；评估过 pdf-extract（91.5% pass rate）/lopdf（无内置提取）后不引入 |
| D-6 schema | 零 run-event 变更 | source_type 自由 String + visibility/content_sha256 已有 |
| D-7 证据核心位置 | orz-tools（非 orz-host） | 直连 fetch 执行在 codegen crate 内（call_tool → toolset → WebFetchClient），管线必须在 orz-tools 内运行；浏览器通道由 orz-host 调其公开 API，无循环依赖；orz-host 不新增 pdf_oxide 依赖 |
| D-8 上限 | 50MB（对齐 read_file） | 旧设计 100MB 过宽；10MB 内联预算与 50MB 下载上限的差距由"存证据、只回前段"消解（§3.7.7 下载大小限制落实） |
| D-9 NO_TEXT_LAYER | 非硬失败（显式 metadata） | 旧设计 §10.4 `{"valid_pdf": true, "has_text_layer": false, "status": "unreadable_without_ocr"}` 先例；OCR 为 non-goal |

## 5. 边界登记（已知限制，不修）

1. **下载完成事件 best-effort 投递**：丢 `downloadProgress{completed}` → 轮询兜底（pending 下载 + staging 文件 500ms 稳定采样两次即完成——审查 P2-4 修复后为真）；轮询也失效 → 显式 `[web_fetch_pdf_timeout]`，不静默成功
2. **Chrome 落盘异步性**：completed 事件后文件可能被 Windows 短暂锁定 → `wait_for_new_file` ≤5s 轮询重试；仍失败 → `[web_fetch_pdf_browser_failed]`
3. **suggested_filename 不使用**：下载文件名从不参与路径拼接（审查 C3-3——该字段连日志也未使用；取新文件用目录采样）
4. **staging 生命周期**：每调用唯一 UUID 子目录（审查 P2-3）；**全路径清理**（成功/超限/ingest 失败——审查 P3-1）；崩溃残留由 retention `pdf-downloads-` 按龄兜底
5. **`*.cnki.net` 不匹配 apex**（需显式列出）；`*` 危险标注；未配置=全直连为默认安全面
6. **pdf_oxide 提取限制**：单文本对象 >32767 字符被解析器截断（引擎行为，未留回归测试——审查 C3-2；截断场景测试用多页 30K 构造；真实论文逐页文本通常 <32K）
7. **marker 契约脆弱性**：orz-tools 输出 ↔ orz-loop hook 跨 crate 契约（B1/E1 双端测试锁定；格式变更必须双端同步——截断 footer 先例同级）；消费端对 marker 内 hex 做 64-hex 形状校验（审查 P3-8——页面文本含 `document_id=sha256:` 字样不能伪造 pdf_document 记录）
8. **并发边界**：同 cwd 双 session 并发 ingest 同 URL——内容寻址幂等；无跨进程锁（project-doc-index 先例）；torn 写由 metadata.json 最后写 + 重 ingest 自愈；pdf_read 坏 JSONL 显式 `Corrupt` 不自愈（重下载是恢复路径）；**同 session 并发白名单下载由唯一 UUID staging 子目录隔离**（审查 D3-2——每调用独立目录，无 reset 竞态）
9. **白名单路由仅精确 `web_fetch` 名**：`web_fetch_*` 变体不拦截（走 toolset，自动获直连管线）；浏览器通道独享精确名
10. **浏览器通道与 mode 的关系**：mode=off 时检索工具整体投影/拒绝（既有）；framework_fallback 下白名单命中 + 浏览器不可用 → 显式失败（D-2 覆盖）
11. **retention 7 天**：`pdf_read` 跨 run 复用仅窗口内有效；document_id 失效后显式 `[pdf_read_not_found]`
12. **其他 orz-tools host**（codex 等）不配 `pdf_evidence_root` → 保留 legacy save_pdf（配置差异登记，非静默降级）
13. **无 OCR**：扫描 PDF 显式 `[pdf_read_no_text_layer]`（旧设计 non-goal）
14. **有头浏览器手动登录的 cookie 会话隔离**：登录态只存在于隔离 profile 内（既有 D-13 机制），证据库不含任何凭据
15. **白名单拦截绕过 web_fetch 缓存**（审查 D3-5）：命中白名单的 URL 每次走浏览器重新下载，不读 web_fetch 缓存；浏览器通道门禁恒为 allow_local=false（比直连更严，`allow_local` 参数对拦截路径不生效）——路由裁决的必然结果，非缺陷

## 6. 验证（全绿）

| 项目 | 结果 |
|---|---|
| orz-tools pdf_evidence（A 组 13 新） | **13 passed**（text-layer/无文本层/垃圾字节/HTML 伪装/magic-ok-parse-fail/超限/截断/document_id 穿越/round-trip/NotFound/坏 JSONL/幂等重 ingest） |
| orz-tools web_fetch（B 组 4 新 + 既有 128） | **132 passed**（wiremock 直连全链路/伪 PDF 硬错/legacy 兼容/PDF 上限 50MB） |
| orz-host local_browser（G 组 6 新 + 白名单测试改） | **40 passed**（completed→Pdf/canceled 显式/load 无 pending→Page/downloadWillBegin 后 load 跳过/超时/新文件轮询；白名单精确断言 + 2 反例） |
| orz-host pdf_evidence（C 组 11 新） | **11 passed**（domain_matches 表/路由 env 解析/未配置直连/浏览器管线 ingest/不可用显式/HTML 页走读/取消显式/pdf_read 成功/跨 run/错误码全套） |
| orz-host call_tool 拦截（C7） | **1 passed**（白名单命中 → 浏览器通道；无 browser → `[web_fetch_pdf_browser_unavailable]` 无回退） |
| orz-host retention（F 组 2 新） | **14 passed**（证据两级按龄/shards 容器保留/staging 按龄） |
| orz-loop relay + evidence（D4/E 组） | relay 6 passed；evidence_visibility 1 passed（marker 三态/legacy 修复/hex 摘要/pdf_read 两态/**被拦截截断用例/伪 marker 拒识**） |
| **orz-host 全量** | **191 passed / 0 failed** / 3 ignored（158→191，+33）——修复批后局部复验见 §7 |
| **orz-loop 全量** | **170 passed / 0 failed** / 3 ignored（测试数 170 与前序基线一致；evidence_visibility 为既有测试扩展断言，新增用例不计数） |
| orz-tools 全量 | **2728 passed / 0 failed** / 6 ignored（-j2） |
| clippy / pytest / check_repository / git diff --check | 见 §8 验证命令（三面审查后复验；`-D warnings` 需先修 orz-config 前置警告——修复批 C2-1） |

## 7. 三面审查修复批（2026-08-11，全部已应用并复验）

三独立代理（设计合理性/实现合理性/符合性）审查未提交内容——**无 P0**。修复批：

| 级别 | 发现 | 修复 |
|---|---|---|
| **P1-1**（实现，高） | `Browser.setDownloadBehavior` 缺 `eventsEnabled:true`——Chromium 的 `download_events_enabled_` 默认 false，现代 Chrome 上下载事件完全静默（Page.* 事件已弃用）→ 通道必然超时；CI 无法发现（脚本化 ws 测试） | `eventsEnabled:true` + ws_reader_task 双事件组转发（`Page.*` + `Browser.*`）+ 轮询兜底（P2-4） |
| **P1-2**（实现，高） | 下载导航在渲染层取消 → `location.href` 恒 `about:blank` → `final_url` 失真写入 metadata `source_url`（破坏"源 URL 可重建"）；测试断言是脚本回放值（P2-5） | `final_url` 改用 `downloadWillBegin.params.url`（真实资源 URL）+ 缺省回退请求 URL + `gate_download_final_url` 策略检查（非空非 about:blank 过门禁，与 extract_page 同级——设计审查 P1-1）；测试断言改事件 URL + 新增 about:blank 回退用例 |
| **P1-3**（实现/符合） | 审计 §7 引用的 live 测试 `local_browser_e2e_pdf_download` 不存在——下载通道从未对真实 Chrome 验证 | 补建 env-gated live e2e（`GSA_RUN_LIVE_BROWSER_TESTS=1`，真实 Chrome 下载 w3.org 公共测试 PDF，断言落盘+%PDF magic+final_url≠about:blank） |
| **D1-1**（设计，必须修）=P2-1=C2-2 | 白名单命中 + HTML 页截断输出含 `[browser_read content truncated` footer，controller web_fetch 分支不识别 → 截断页误判 `full_text_observed`（§3.7.5 违规） | web_fetch 截断检测加 `[browser_read content truncated` + 被拦截形态测试用例（partial） |
| **P2-2** | `fs::read` 整读后才查 50MB 上限——超大下载（setDownloadBehavior 无字节上限）OOM | 先 `metadata().len()` 检查再读 |
| **P2-3** | staging 共享目录 + 重置失败 → 上一轮残留文件被当新下载（静默错配） | **每调用唯一 UUID 子目录**（"最新文件"语义精确；残留由 retention 兜底） |
| **P2-4** | 「丢 completed 事件目录轮询自愈」声明不实——事件循环从不轮询 | 事件循环内轮询兜底：pending 下载 + staging 文件 500ms 稳定采样两次 → 完成（与 P1-1 互补）；修复片段化等待误报 LoadTimeout 的 bug + 回归测试 |
| **P3-1** | ingest 失败路径不清理 staging（注释与行为不符） | 全路径清理（成功/超限/ingest 失败统一 remove_dir_all）+ `whitelist_hit_ingest_failure_cleans_staging` 测试 |
| **P3-7** | 下载路径 extract_page fallback 硬编码 "about:blank"（与 read 路径不一致） | fallback 传请求 URL |
| **P3-8** | pdf_hex 探测过宽（页面文本含 marker 字样可伪造 pdf_document 记录） | 64-hex 形状校验 + 伪 marker 拒识测试用例 |
| **C1-1** | 审计 §7 引用不存在测试（同 P1-3） | 补建 live e2e 后引用成立 |
| **C2-1** | `clippy -D warnings` 复验失败——orz-config 前置 `unused variable: metadata`（Windows 分支参数未用，切片外） | 修 orz-config 参数名 `_metadata`（unix 分支引用保留）后 `-D warnings` 通过 |
| **C2-3** | ADR-0010 §3.7 无 PDF 管线条目——D-1/D-2 是设计级新增裁决，按 §14.3 先例须转录 | **ADR-0010 v1.4 补写**：§3.7 条 11（双通道路由+白名单+不回退）+ §14.4 索引 + 头部版本标记 |
| **D2-1** | `PDF_OXIDE_VERSION` 常量 0.3.43 vs lock 实际 0.3.46（caret 区间漂移） | workspace pin 精确化 `=0.3.46` + 常量同步 + 注释（升版须双改） |
| **D3-1** | 同 P1-3 | 同 P1-3 |
| **D3-2** | staging 共享目录 reset 竞态未登记 | P2-3 唯一子目录已消除；登记随 §5 边界 8 更新 |
| **D3-3** | ingest 失败残留措辞不覆盖 | P3-1 已修；§5 边界 4 更新 |
| **D3-4** | pdf_read「≤20 页/调用」与默认读全部页矛盾 | 工具描述/索引措辞统一（"显式 range ≤20 页；默认全部页截断"） |
| **D3-5** | 白名单拦截绕过 web_fetch 缓存未登记 | §5 边界 15 补记 |
| **D3-6** | 同 P3-8 | 同 P3-8 |
| **D3-7**=C3-4 | retention 注释宣称 fail-safe（无 metadata 保留）与实现不符 | 注释修正（超龄即删，fail-safe 仅 stat 失败路径）+ 空 shard 保留登记 |
| **C3-1** | 「目录轮询自愈」措辞（修复前不实） | P2-4 修复后措辞为真（轮询兜底已实现） |
| **C3-2** | 32767 引擎截断边界无回归测试 | 审计标注（不补测试——引擎行为非本切片契约） |
| **C3-3** | suggested_filename「仅日志展示」——实际未使用 | §5 边界 3 措辞修正 |
| **C3-5** | 「P0-1 timeout 包裹」措辞——浏览器下载实际受 CDP total_budget(60s) 约束 | §3.4 措辞修正 |
| **C3-6** | orz-loop「167→170，+3」基数与前序审计矛盾 | §6 表措辞修正（测试数 170 不变，扩展断言不计数） |
| **C3-7** | 边界计数口径（14 vs 13） | §5 按 15 条编号（含修复批新增 1 条） |

**修复批后复验**：local_browser **43 passed**（+轮询兜底回归 + Browser.* 事件组 + 事件 URL 回退）、orz-host pdf_evidence **13 passed**（+ingest 失败清理）、orz-loop evidence_visibility **1 passed**（+被拦截截断/伪 marker 用例）、clippy `-D warnings` 通过、全量复核见 §8。

## 8. 验证命令

```powershell
cd D:\CLI\orz
cargo test -p orz-tools -j2 pdf_evidence; cargo test -p orz-tools -j2 web_fetch
cargo test -p orz-host -j2; cargo test -p orz-loop -j2
cargo clippy -p orz-tools -p orz-host -p orz-loop -j2 -- -D warnings
cd D:\CLI
python -m pytest assurance/tests -q -x; python scripts/check_repository.py; git diff --check
# 本地手工 live（不进 CI）：真实 Chrome 下载公共测试 PDF（P1-1/P1-2 实机验证面）
# cd D:\CLI\orz; $env:GSA_RUN_LIVE_BROWSER_TESTS="1"; cargo test -p orz-host -j2 -- --ignored local_browser_e2e_pdf_download
```
