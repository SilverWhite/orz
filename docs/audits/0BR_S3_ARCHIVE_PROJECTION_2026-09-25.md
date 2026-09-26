> 状态：`current`；批次＝**0br S3 新增面落码（会话归档投影面）**（BACKLOG/TODO P1-0br；承接 [`0BR_S2_REVIEW_HANDLING_2026-09-25.md`](0BR_S2_REVIEW_HANDLING_2026-09-25.md) §8 走查反馈批六登记的用户令 2026-09-25「会话的归档和查看归档会话都要做的，请将其归入S3项」；2026-09-25 用户令「请先做完S3的新增面吧」）；性质＝实施回执。**未提交／未推送／未重建载体**（沿 S2 口径，记账随提交批办理）。

# 0br S3 新增面回执（会话归档投影面：桥只读归档 API ＋ 探索器归档标记 ＋ archive:// 只读浏览）

## 1. 交付物清单

### 1.1 桥 crate `orz-web`

| 文件 | 职责 |
|---|---|
| `src/archives.rs`（新增，约 420 行含测试） | `.gsa/archives` 只读投影：清单（仅 `{s8}.json.gz` 过 id 门者、里程碑事实 best-effort、新→旧、cap 200）＋ 详情（gzip 解包 → 信封容忍解析 → 会话事实＋三键段透传＋有界转写） |
| `src/security.rs` | 增 `session_id_ok`（归档/侧车 `{s8}` 路径段与 run id 同规则：`[A-Za-z0-9_-]`、无点无分隔符、Windows 保留名拒） |
| `src/server.rs` | 增路由 `GET /api/archives` 与 `GET /api/archives/{session8}`（token＋Host 门沿既有 `gate`）；错误映射 1:1 类型化（400 id／404 缺失／413 超限／500 损坏）；`error_response` 帮助函数 |
| `src/lib.rs` | 挂载 `archives` 模块 |
| `Cargo.toml` | 增 `flate2 = { workspace = true }`（workspace 既有 1.1.5 zlib-rs，Cargo.lock 零新增外部包） |

**读数契约（有界投影，沿大文件读取契约 §14.22 姿态）**：压缩输入上限 64 MiB；解压流式上限 128 MiB（`take(cap+1)` 区分「恰在上限」与「超限」，gzip 炸弹不进内存）；每条转写消息 4,000 字符截断（保留 `content_chars` 原长计数＋`truncated` 标记）；转写总量 1 MiB 字符（超限停装＋`transcript_truncated`＋`messages_shown/total`）；工具参数 JSON 每调用 2,000 字符。

**信封容忍解析**：与 agent 侧写入方 `orz-host/acp_server.rs::decode_archive_package` 同口径——v0.2 信封（`schema`＋`conversation`［＋`archive_keys`］）与旧裸包（gzip 内容直接是 sidecar）双形态接受；其余形状判损坏（fail-closed）。

### 1.2 前端（零新增文件，全部在既有模块内）

| 文件 | 职责 |
|---|---|
| `app/api.js` | `fetchArchives`／`fetchArchive`（令牌经 `?token=`） |
| `app/state.js` | `state.archives`；`vm.addArchivedMessage`（静态只读装填：无锚点、无去重——转写是归档的整体投影，不是流式事实通道） |
| `app/main.js` | `archive://` URI 路由进 `openArchive`（清场→取详情→事实行→转写装填→收尾提示）；`renderArchiveTranscript`（用户/模型卡＋`tool_calls` 工具行，工具结果按 `call_id` 并回调用行明细，对不上即如实丢弃）；`archiveFactsLine`（归档时间／tokens／消息数／LIF 轮跨度［三键段 `archive_keys.lif`］／台账 seq 跨度／运行数）；探索器刷新拉归档清单（失败保留旧值） |
| `app/widgets.js` | 会话组行尾部「◆归档」跳转（`stopPropagation`，点击开 `archive://{s8}`；组行点击仍是会话合并视图）；**仅存归档（侧车已清扫）的会话同样入树**；组行文本补「· 已归档」 |
| `app.css` | `.archive-jump` 样式（右浮、主色深蓝、悬停下划线；行悬停反白） |

**每区块标注来源件**：本面为 S3 新增自研面（综合稿 §7 无对应上游件——归档浏览是 orz `.gsa/archives` 三键包特有对象；登记于本档，R-7 同级），交互语义沿 R-4 对象 URI 路由与 R-5 投影移植先例，视觉沿 98.css 照搬件。**UI 零执行事实不变**：读取、截断判定、错误分类全在桥侧，前端只渲染。

## 2. 验证读数（本机 Windows，2026-09-25）

- **Rust**：`cargo test -p orz-web`＝**38 通过 / 0 失败**（S2 处置后 31＋新增 7 钉：session id 门、清单只认合法 `.json.gz`＋里程碑事实、信封解析计数与 4K 截断＋原长计数、旧裸包容忍、类型化错误矩阵［id 门先于文件系统/缺失/非 gzip/非 JSON/未知形状］、转写总量上限停装并置标、API 门矩阵［无令牌 401／伪造 Host 401／遍历·保留名·带点 id 400／缺失 404／happy path 三键透传］）；`cargo clippy -p orz-web --all-targets`＝**0 警告**；`cargo fmt -- --check`＝净；`cargo build -p orz-bin`（debug，`PROTOC=orz/bin/protoc.exe`）通过。
- **前端冒烟门**：`node tests/frontend_smoke.mjs`＝全绿（新增两钉：归档转写装填无锚点、探索器归档标记＋仅归档会话入树）；`node --check` 10/10 净。
- **实机 HTTP 探针**（`orz web` 真进程 @ 真实 9 归档工作区）：`/api/archives` 无令牌＝**401**、伪造 Host＝**401**、带令牌＝9 件清单（里程碑 `archived_at`/`archived_tokens` 在位）；详情（6aac0af5）＝消息 307（用户 6／助手 129／工具结果 172）、LIF/台账/归档事实全在位；`NUL`＝**400**；不存在＝**404**；`a%2Fb`＝**400**。
- **最大包性能**（6ab3dbe5，4.0 MiB gz／约 4.2M tokens 会话）：详情 200，响应 1.57 MB、**0.28 s**；截断读数＝821 条装 677 条、`transcript_truncated: true`、1,046,457 字符（1 MiB 上限生效）。
- **浏览器级走查**（真实浏览器，`orz web` @ 1280×720）：探索器 20 会话组中 9 组带「◆归档」；点击 6ab3dbe5 的 ◆归档 → 地址栏切 `archive://6ab3dbe5`、后退钮点亮、事实行完整（归档于 2026-09-23T17:03:56Z · 约 4200K tokens · 消息 821 条 · **LIF 轮 151→252（normal）** · 台账 seq 1→10128 · 运行 1 个）、转写中文卡与工具行渲染正常、截断收尾行「转写超上限已截断（677/821 条）」；「后退」→ `workspace://live` 实时重灌正常。截图留档会话 artifacts。

## 3. 实机走查发现与处置

| 级 | 发现 | 处置 |
|---|---|---|
| P2 | 事实行 LIF 段缺失——首版读 `conversation.lif`（sidecar 原生键 `entry_round`/`round`）却按三键段键名（`round_start`/`round_end`）取值 | `archiveFactsLine` 改读 `archive_keys.lif`（三键段）；两把尺的键名差异在代码注释锁定 |
| 观察项（不落码） | 旧归档（6aac0af5，2026-09-17）首条用户消息在数据面即为乱码（UTF-8 字节按 GBK 解码的历史形态）；同一包内 `archive_keys.axis_note` 中文完好、新归档（6ab3dbe5）全文完好——**根因在归档数据本身**（当次无头 run 的输入编码历史问题，与 `GAP-READ-FILE-TEXT-ENCODING`/编码门工作线同族），归档「纯打包零内容变换」不变量下投影层如实呈现 | 投影无责不改；本档登记为数据面历史观察项，不作修复对象 |

## 4. 边界与批序

- 归档浏览＝**有界摘要＋有界转写**，不是全文归档回放（全文经「已截断」标记指引归档本体）；黑板/时间轴等其余包内成员未投影（三键段原样透传在详情 JSON，前端展示面按需扩展）。
- 工具结果并入调用行依赖 `call_id` 匹配，匹配不上即丢弃（不造孤行）——如实呈现包内对应关系。
- 快照选择器、会话标题增强（侧车首条消息作组头可读名）仍留 S3 批；本批未动。
- 本批**未提交／未推送／未重建载体**；`orz web` 进件仍以载体重建为前置（S3 真机首读时办理）。
- 探针进程已清理（服务与 `--stdio` 子进程零残留；一处前批残留 `orz web` 冒烟进程顺带清点）。

## 5. 入口

- 账面：[`0BR_S1 勘定`](0BR_S1_WEB_FORM_SURVEY_2026-09-24.md) / [`0BR_S2 回执`](0BR_S2_IMPLEMENTATION_2026-09-24.md) / [`0BR_S2 审查处置`](0BR_S2_REVIEW_HANDLING_2026-09-25.md) / [`综合稿`](../UI_FORM_CONSOLIDATED_DESIGN_2026-09-24.md)
- 实现：`orz/crates/orz-web/src/archives.rs`（桥投影）＋ `assets/app/main.js`（`archive://` 视图）＋ `assets/app/widgets.js`（归档标记）
- agent 侧归档写入方：`orz/crates/orz-host/src/acp_server.rs`（`package_session_archive`／`build_archive_keys`／`.milestones.json`）
- 待办：BACKLOG/TODO `P1-0br`（S3 真机首读待放行）

关键词：0br S3 新增面、会话归档投影、`/api/archives`、信封容忍解析、三键段透传、有界转写、4K 截断、1 MiB 上限、gzip 双上限、◆归档标记、archive://、仅归档会话入树、LIF 轮跨度、数据面历史乱码观察项。

---

# 批二（同日 2026-09-25 第二用户令）：探索器三组重构 ＋ 归档动作 ＋ 状态语义 ＋ 错误路径

> 用户令六点：①已归档内容不再在会话栏混杂展示，探索器改「工作区／活跃会话／归档会话」三组；②工作区组分两级子级「当前工作区＋全部已信任工作区」，总栏补加粗；③活跃会话保留「归档」键但功能改为**归档活跃会话**；④「进行中」只针对真正在动作/流式输出的实时窗口；⑤修归档内容读取失败；⑥修未归档活跃会话打不开。

## 6. 交付物

| 面 | 内容 |
|---|---|
| 探索器三组 | `renderExplorer` 重写：**工作区**（`explorer-group` 加粗组头；子级＝`当前: {cwd}` 行［点击回实时］＋可折叠「已信任工作区（N）」子清单）、**活跃会话（N）**（有运行/侧车者；行尾 `◆归档` ＝归档动作键）、**归档会话（N）**（仅存归档包者；行＝`{s8} · 归档时间`，点击开 `archive://`）。归档包存在但仍有活跃面的会话**留在活跃组**（其归档面不再混杂展示） |
| 信任工作区 | `orz-workspace` `TrustStore` 新增只读 `decisions()` 访问器；桥新模块 `trust.rs`（列授予、跳过拒绝、排序）经 `orz-workspace` workspace 件读**与 agent 门同一**用户信任存储（`user_grok_home()/trusted_folders.toml`，redirect 后即 `{install}/grok-home`），零新增外部包；`/api/boot` 增 `trusted_workspaces`；前端 `boot()` 先行拉取（不等首次会话创建）。**禁第二套实现**：信任语义单源在 TrustStore |
| 归档动作 | orz-host 新 pub 入口 `archive_session_on_demand(base_dir, s8)`——**复用关闭归档同一原语** `archive_session_package`（gzip 信封＋三键段＋里程碑水位＋ARC journal 事件一次到位，无 incremental 标记）；orz-bin 新 `archive <session8>` 子命令（cwd 工作区，stdout 带结果，退出码表成败）；桥 `POST /api/archives/{session8}`（token＋id 门→spawn 子进程→120 s 超时→成败与输出如实回传，kill_on_drop）。UI 递单、agent 执行——零执行事实边界不变（§14.78 条 5）。运行中的实时窗口禁用按钮 |
| 状态语义 | `state.liveRunId`（实时尾通道跟随的 run）＋`state.running` 构成「进行中」**唯一判据**；journal 无终态事件的旧 run（中断收场）如实标「**未完成**」 |
| 错误路径 | `startTail`／`openRunReplay`／`openConversation` 全部 try/catch——失败显式系统行报错并渲染，不再留空白＋未处理拒绝（bug ⑥ 的真实缺陷面） |

## 7. 两 bug 根因考证（⑤⑥）

- 当前构建下**全量扫描通过**：20 个会话行＋9 个归档跳转逐一点击，全部正常渲染（含 4 个「未完成」会话与最大 4 MiB 归档）。
- 复现条件＝**浏览器标签页指向已被终止的旧服务端口**（本批重建/重启服务所致的残留页）：归档路径有 catch 显式报「读取失败」，会话路径无 catch 静默留白——两条路径的失败表现与用户所见一致。
- 修复后语义：桥不可达时任何打开动作都给出可见答复（横幅＋内容区系统行），不再静默。

## 8. 验证读数（本机 Windows，2026-09-25）

- **Rust**：`cargo test -p orz-web`＝**41/41**（＋3：归档动作门矩阵［401/400/500 spawn 失败如实回传］、信任清单授予列/拒绝跳过、空存储零条）；orz-workspace 全绿（含 `decisions()` 新钉）；orz-host 按需归档钉子通过（临时工作区：包落盘＋`ARC-{s8}-1` journal 事件无 incremental 标记＋水位落地＋幂等重打包＋无侧车显式报错）；clippy **0 新增**（orz-host 11 条均为依赖 crate 既有基线，逐条核对不在本批代码）；fmt 净。
- **POST 双路径 e2e**（临时工作区＋真实二进制）：有侧车会话 `POST → ok:true`（包＋ARC journal＋里程碑三件套产物核对在位，message 含 digest 与 tokens）；无侧车会话 `POST → 500`「无侧车……无可归档内容」逐字回传。
- **浏览器走查**（临时工作区 1280×720 截图留档）：三组渲染（工作区加粗＋当前行＋已信任（0）折叠子级／活跃会话行带「· 未完成」＋◆归档动作键／归档会话行带归档时间）；boot cwd 正确。
- **信任清单读数＝0 条为本机权威事实**：本地构建 folder-trust 惰性从未写授权，redirect 后 orz 自有 grok-home 存储为空；`~/.grok/trusted_folders.toml` 既有条目属上游 xai 工具残留，非 orz 门面决策，不读（观察项登记）。
- 冒烟门全绿（三组结构／未完成／进行中唯一判据／归档动作键＋有归档包仍留活跃组等断言）；`node --check` 10/10。
- 探针进程与临时工作区已清理，零残留。

## 9. 批二边界与登记

- 归档动作对**当前连接中的活跃会话**同样可用（即时快照打包；侧车随 prompt 持续落盘，关闭归档随后再覆盖写）——运行中窗口禁用按钮为唯一护栏。
- `orz archive` 子命令走 `TrustPolicy::Enforce`（与生产会话同姿态；ARC journal bootstrap 失败不阻断包落盘——best-effort 语义沿原语不变）。
- 本批改动进 orz-host/orz-bin/orz-workspace/orz-web 四 crate；**未提交／未推送／未重建载体**（沿 S2 口径）。

关键词：批二、探索器三组、工作区两级子级、TrustStore decisions、同一信任存储、归档动作、`orz archive` 子命令、archive_session_on_demand、POST 归档、进行中唯一判据、未完成、错误路径收口、旧端口残留页根因。

---

# 批三（同日 2026-09-25 第三用户令）：归档全部会话 ＋ 分组移组修复 ＋ journal 重构归档

> 用户令：「请尝试归档当前全部会话」，并报两问题——①存在归档失败；②归档成功但「归档会话」组无显示。另：D 盘写满致构建链接失败（LLVM no space on device），用户令「请清理D盘，不要使用其他盘」。

## 10. 两问题根因与处置

| 问题 | 根因 | 处置 |
|---|---|---|
| ②归档成功不入「归档会话」组 | 分组规则缺陷：旧规则＝「仅存归档包（无活跃面）者入归档组」，而归档成功的会话都还有运行/侧车 ⇒ 永远留在活跃组 | 分组规则改为**归档判定**：会话存在归档包**且**包 mtime ≥ 该会话最新运行 journal mtime ⇒ 入归档组；归档后又继续（新 run 更新 journal）则如实回活跃组（待下次归档再移入）。归档组行＝`{s8} · 归档时间`，点击开 `archive://` 只读浏览 |
| ①归档失败（9 个会话） | 9 个无头 `-p` 会话（6ab14384/6aafe5d8/6aafe51d/6aafdaa5/6aafc998/6aad91f0/6aac0a28/6aabfe74/6aabf5eb）无侧车——§14.68 设计边界：未跨 500K 里程碑的无头 run **不落侧车**，而归档原语只打包侧车 ⇒「无侧车——无可归档内容」 | **journal 重构归档**（新回退路径）：侧车缺失时从 run journal 机械重构对话——`prompt_submitted` → 用户消息、`model_output`（正文或工具调用）→ 助手消息；包内注入 `reconstructed_from_journal: true` 标记（如实标注非零变换侧车拷贝）；三键段 journal 键显式纳入无头 run（`RUN-CLI-{s8}` 精确 ＋ `RUN-{s8}-*` 前缀，与 0ak 注入同口径）；黑板/LIF 该类会话本就没有（zero loss），ARC journal 事件与里程碑水位照常 |

## 11. 实现面（批三）

- orz-host：`archive_session_package` 拆为「读侧车薄壳 ＋ `archive_raw_session_package`（任意原始字节共享包装：打包→水位→ARC 事件）」，同步面改 `package_archive_raw`（吃字节不吃路径）——磁盘侧车与重构字节同一打包入口，禁两套实现；新 `reconstruct_conversation_from_journal`（run 面＝前缀∪精确、按 run id 排序、run 内 sequence 序；started_at 取首事件 RFC3339；`has_success` 取 run_finished）；`archive_session_on_demand` 回退重构并在结果消息带「journal 重构（N 次运行）」。
- 前端 `widgets.js`：归档判定分组（mtime 比对）＋归档组行带时间。
- 钉子：orz-host 重构归档测试（journal-only 会话成包、标记在位、三键段含无头 run、零事实报错「无可重构」）；前端冒烟新增移组/回组断言（归档包新 ⇒ 归档组；归档包旧 ⇒ 回活跃组带 ◆归档）。

## 12. 归档全部会话执行读数（真实工作区 D:\CLI）

- **19/20 成功**（11 侧车会话打包／覆盖 ＋ 8 个无侧车会话 journal 重构成包；aa18ef7e 为重复归档＝覆盖写幂等）；全部 ARC journal＋水位落盘。
- **1 个如实失败**：`6aac0a28`——其 run（658 B）仅 run_preflight＋tool_availability_check，**从未发出 prompt 即早夭**，零对话事实 ⇒「无侧车且 journal 无可重构对话事实」。该会话保留在活跃组（未完成），如实呈现。
- 浏览器验证：活跃会话（1）／归档会话（19）分组正确；重构归档 6aabf5eb 打开可读（事实行含「约 18K tokens · 消息 75 条」，用户/工具行/模型卡齐全）。截图留档。

## 13. D 盘清理（用户令「请清理D盘，不要使用其他盘」）

- 根因：D 盘 0 可用（276.6 GB 满）→ `orz-bin` 链接失败（LLVM no space on device）。
- 处置：删除 `D:\CLI\orz\target\debug\incremental`（**17 GB**，纯增量编译缓存，完全可再生）→ 余量 **15.2 GB**。其余大目录（tb-eval／swebench-eval／VMs／DockerData 等）属用户数据未动；`.gsa`（235 MB）与项目文档面未动。

## 14. 批三读数与边界

- orz-web 41/41＋冒烟全绿＋`node --check` 10/10；orz-host 归档面 11/11（含重构钉子）；clippy 新增代码 0 告警；fmt 净。
- 重构包语义边界：重构对话仅含 journal 中用户/助手正文与工具调用事实，**不含**黑板/LIF/推理链/工具结果全文（该类会话从未落过这些面）；包内标记保证读者可区分。此语义为本批新增扩展（§14.68 未预见按需归档面），如需收窄/调整留用户裁决。
- 本批仍未提交／未推送／未重建载体。

---

# 批四（同日 2026-09-25 第四用户令）：归档浏览的标记栏可用

> 用户令：「归档会话中能也使标记栏可用吗？」——归档浏览此前清空标记栏（转写走无锚静态装填）。

- **改动**（state.js＋main.js，零新文件）：`addArchivedMessage` 用户输入逐条产「▸ T{n} 输入」锚（与实时视图同语义）；`openArchive` 转写装填后调 `anchorFinalOutput()` 定「◆ 最终输出」锚（中间轮不产锚，沿实时视图同规则）；点击跳转/展开折叠卡经既有 `scrollToItem` 原样可用。
- **读数**：冒烟门新增锚点断言全绿（用户锚＋终态回溯输出锚＋指向校验）；实机验证（归档 6aac0af5）：标记栏＝6 个输入锚＋1 个最终输出锚（与该包 6 条用户消息吻合），点击 ▸ T3 输入 scrollTop 3727→1050 跳转生效。
- **列表对齐补笔（同日第五用户令「归档会话的列表能做到完全对齐吗」）**：会话行改双 span 结构（`session-id`／`session-when`，Consolas 等宽 12px）——比例字体下 hex 宽窄不一导致参差；ID 恒 8 字符＋时间恒定长格式 ⇒ 两列严格成列，活跃会话行 ID 前缀同款。冒烟门 DOM stub 补 `createTextNode`；截图验证 19 行两列全对齐。
- **状态栏去「沙箱严格」段（同日第六用户令）**：生产面无 OS 沙箱（`orz-sandbox` 未接线生产面，真机 run Job/子 Job 只约束派生进程树——`GAP-SPAWN-ORPHAN-RECLAIM` 口径澄清同源），保留即失真假面 ⇒ StatusBar 冻结六段改五段（守护/网络关闭/工具 n/m/模型/运行态），注释锁定去除断代由与依据；**orz-tui 母本的同步留待 TUI 后补批**（投影双实现同步纪律登记面）。实机验证五段在位。
- 本批仍未提交／未推送／未重建载体。

关键词：批四、归档标记栏、▸ 输入锚、◆ 最终输出锚、scrollToItem。

---

# 批五（同日 2026-09-25 第七用户令）：归档收尾流程三件——ARC 水位分组修复 ＋ 工具栏「新建会话」＋ 实时尾随随 run 切换

> 用户令：「当前归档实质不可用，刷新后也没有将活跃会话归档，而且需要一个明确的新建会话按键才可以，放在"后退"的前面吧」

## 15. 根因（「归档实质不可用」＝两件叠加）

- **归档后不进归档组（刷新依旧活跃）**：批三分组规则「包 mtime ≥ 最新运行 mtime ⇒ 归档组」把 **ARC 审计 journal** 计入活动水位——归档动作自身的落盘序＝包 → 里程碑水位 → ARC journal（`archive_raw_session_package` 内 bootstrap_session 晚于打包），ARC journal 恒晚于包 1–3 秒 ⇒ 每次新归档 `isArchived` 恒 false。真实工作区实证：`6ab6275c`/`6ab6570c`（0bm/0bs 轮会话，包 mtime 1790342264/1790342302，含 ARC 的最新运行 1790342267/1790342303）→ 双双误判活跃。批三验证时「归档全部」把 19 个包 mtime 整体重刷、掩盖了该结构缺陷。
- **流程无法收尾**：桥为单 ACP 会话绑定（`ensureSession` 终身复用首个会话），无任何换会话入口——即便分组正确，归档后下一条 prompt 立即产生新 run（mtime ＞ 包）⇒ 会话弹回活跃组，对活跃会话而言归档动作等于无效。这就是用户令「需要一个明确的新建会话按键才可以」的语义。

## 16. 实现面（批五）

- **分组修复**（widgets.js）：活动水位计算排除 `ARC-` 前缀 run（归档审计记录，非对话活动）；归档后真实新 run 仍如实回活跃组，批三口径其余不变。
- **工具栏「新建会话」**（state.js／widgets.js／main.js）：`{id:'new', label:'新建会话'}` 居首位（用户令指定「放在后退的前面」）；`ui.newSession`＝运行中拒绝（flash 提示）→ 清内容/标记/轮次、导航栈归零（「后退」不得回到旧会话回放）、解绑旧 ACP 会话（`acpSessionId=null`；ACP 无 `session/close`，旧会话对象就地闲置、事实已留 `.gsa`，探索器可回看）→ 复位状态栏「空闲」→ 立即 `ensureSession()` 建新会话并在内容区显示新会话 id。
- **实时尾随随 run 切换**（main.js `syncLiveTail`，既有缺陷一并修复）：每轮 prompt 一个新 run 目录（`RUN-{s8}-{n}`），而尾随此前只在 boot 挂到当时 `runs[0]` 且永不切换——新 run 的事实（工具行/门控/终态）永不流入实时窗。现口径：已建会话只跟**本会话**最新 run（新建会话后不再灌旧会话内容）；无会话时跟全局最新（boot 连续性保留）；切换从 offset 0 重灌（journal_tail 自文件头补发，事实不丢）。挂点＝5s 探索器刷新＋运行期 1s 跟随轮（`submitPrompt` 内自停 interval）＋boot/returnToLive。
- **状态栏残留复位**（main.js）：`submitPrompt` catch 与 `newSession` 就地回「空闲」——走查实证引导期失败的 run 只有 `run_preflight` 无终态事件，标签残留「预检/完成」。

## 17. 验证读数（批五）

- 冒烟门 `node tests/frontend_smoke.mjs` 全绿（**＋5 钉**：工具栏顺序〔新建会话首位、后退次位〕／ARC journal 不得把新归档顶回活跃组／真实新 run 仍回活跃组／newSession 复位导航清后退栈／桥不可达不残留旧会话绑定）；orz-web Rust **41/41**；`cargo build -p orz-bin` 过（前端资产 `include_bytes!` 内嵌，随构建进位）。
- **真机走查**（`orz web` 合成工作区：bug 场景 `6aab1234`〔RUN mtime −1h、包 −30min、ARC −30s〕／仅活跃 `6aab5678`／仅归档 `6aabdead`）：分组正确（活跃（1）＝6aab5678 带 ◆归档；归档（2）＝6aabdead＋**6aab1234**——修复前该场景恒判活跃）；「新建会话」点击＝内容清空＋「ACP 会话已创建（uuid）」＋横幅＋导航/状态栏复位；归档浏览 6aab1234 事实行＋转写可读、「后退」回实时＝**空实时窗**（当前会话无 run 的正确形态，不再灌旧会话内容）；新会话发 prompt（临时目录未信任 ⇒ 信任门 fail-closed 拒绝，符合预期）⇒ 探索器秒级出现新 run「69422736（1 次运行 · 未完成）」＋journal 尾自动切换回灌 `run_preflight`——**尾随切换与跟随轮实证**。
- **走查方法教训（登记）**：`goto` 仅变 URL hash（新令牌）不重载页面——陈旧页面（旧构建 JS＋已死 WS＋旧令牌）会伪装成回归（flash 可见但消息/复位全不生效）；以 `about:blank` → 目标 URL 强制真实重载后全量复验通过。

## 18. 批五边界

- 本批仍未提交／未推送／未重建载体；索引无召回路由变化、不 bump（§0.5）。
- 历史误判会话（6ab6275c/6ab6570c 等）无需数据迁移——前端纯投影修复，刷新即自行归位归档组。
- 「进行中/未完成」语义、信任门 fail-closed 拒绝面均为既有口径如实呈现，本批不改。

关键词：批五、ARC 审计 journal、活动水位、新建会话、实时尾随随 run 切换、journal_tail offset 0 重灌、goto hash 不重载。

---

# 批六（同日 2026-09-25 第八用户令）：旧归档"消失"诊断 ＋ 归档 UI 逻辑重构（组级功能键/选中/回档/删除）＋ orz 免全路径

> 用户令：「原本旧的归档消失了／话说归档的会话需要能够被删除/还原才可以，我想改一下UI逻辑，"归档"键和"活跃会话"这一栏同行，想要归档会话需要选中会话再点击"活跃会话"这一栏中的总"归档"功能键，然后弹弹窗确认／同理，将"回档"和"删除"放在"归档会话"的这一行，先选中目标对话再点击功能键，随后再弹弹窗确认／话说现在的orz web指令需要使用全路径才行，能不能绑成最起码当前安装文件夹内部全局的，直接"orz web"就可以？」

## 19. 「旧的归档消失了」诊断＝扫错工作区（非数据丢失）

- 用户从 orz 仓根（`D:\CLI\orz`）启动 `orz web`，桥按**启动时 cwd** 定 `.gsa` 根——该目录的 `.gsa` 只有 1 个运行、**零归档包**，探索器于是显示「归档会话（0）」。真实工作区（`D:\CLI`）的 21 个归档包逐包复算批五新口径全部 `归档OK`，**无一丢失**。
- 处置：`orz web` 启动台面新增**工作区行**（`工作区: <cwd>（探索器/运行/归档均按此目录的 .gsa 投影；不符请用 --cwd 指向工作区根）`），错目录启动一眼可辨。

## 20. 归档 UI 逻辑重构（组级功能键＋选中模型）

- **选中模型**（state.js `selected: {group:'active'|'archived', s8}`）：会话行**单击＝选中**（再次单击取消；深蓝底白字高亮），**双击＝打开**（会话合并视图/归档只读浏览；Enter 同义）。
- **组级功能键**（widgets.js `groupHeader` 扩展 actions）：「归档」与「活跃会话」组头同行、「回档」「删除」与「归档会话」组头同行；**未选中时可见但禁用**（title 提示操作流程），选中本组会话才点亮；组头本体折叠开关语义不变（功能键 stopPropagation）。行内「◆归档」动作键退役（CSS 同步清除）。
- **三动作**（main.js，确认弹窗均为原生 confirm，文案如实告知后果）：
  - 归档（既有）：打包＋ARC 审计 journal；运行中的实时会话拒绝归档（守卫移入动作内）。
  - **回档**＝`DELETE /api/archives/{s8}` → 桥 spawn **`orz unarchive <s8>`**（新子命令）→ orz-host `unarchive_session_on_demand`：移除归档包＋里程碑水位，会话回活跃组；运行/侧车数据全保留。
  - **删除**＝`DELETE /api/sessions/{s8}` → 桥 spawn **`orz delete-session <s8>`**（新子命令）→ orz-host `delete_session_on_demand`：彻底移除归档包＋水位＋对话侧车＋会话名下全部运行 journal（`RUN-{s8}-*` ∪ `ARC-{s8}-*` ∪ `RUN-CLI-{s8}` 精确），**不可恢复**。
  - 动作完成清选中并刷新探索器；桥侧三路由共享同一 spawn 助手（`run_agent_session_tool`：单子进程形态/单超时/单错误映射，禁每路由第二套）。

## 21. 数据安全面

- **会话段门**：回档/删除取**最严口径**——严格 8 位小写十六进制（会话 id＝UUID 前 8 字符），其余形态拒绝且不触文件系统。
- **前缀误吞缺陷被钉子逮住**：`RUN-CLI-{s8}` 无尾分隔符，前缀匹配会误删近似会话（`RUN-CLI-6ab7de01` 吞 `RUN-CLI-6ab7de011`）——钉子实证后改为精确匹配（与重构归档同口径）；`RUN-{s8}-`/`ARC-{s8}-` 前缀天然带分隔符无碰撞。
- 边界登记：删除为本地 `.gsa` 梳理动作，不落 journal 审计事件（确认弹窗即用户授权面）；对正在运行进程持有的会话不设跨进程锁（UI 只对归档组会话提供删除，活跃会话不可达此动作）。

## 22. orz 免全路径（用户令③）

- 本机无独立安装目录（"安装"＝本仓构建/载体），取**用户级 PATH 绑定**：`%LOCALAPPDATA%\orz\bin\orz.exe`（当前 debug 构建拷贝）＋ 该目录写入用户 PATH（注册表安全写法，无 setx 截断风险）——**新开终端任意目录 `orz web` 即用**。刷新方式＝构建后重拷；发行载体落地后可整目录替换。
- 配套可见性：启动台面工作区行（§19）。

## 23. 验证读数（批六）

- 冒烟门全绿（组级功能键/选中模型 **＋6 断言**：行内 ◆归档 退役、三键与组头同行、未选中全禁、选中点亮对应组、再点取消选中）；orz-web Rust **42/42**（＋1：DELETE 双路由 401/400/子进程失败透传钉）；orz-host 归档面 **12/12**（＋1：回档/删除作用域与门钉——**逮住并修复 RUN-CLI 前缀误吞缺陷**）；clippy 本批文件零新增（余量告警均为既有 local_browser 面）；单跑负载敏感 5 件全过（0aq 已登记类）。
- **真机走查全链**（合成工作区，confirm 以页内桩自动接受并捕获文案）：选中高亮生效；「归档」→ 弹窗文案正确 → 会话移归档组；「回档」→ 回活跃组、包/水位移除、侧车/运行保留（磁盘核对）；「删除」→「已彻底删除会话 6aab1234（共 5 件）」横幅、磁盘五件全清、邻近近似前缀会话无恙；双击打开＝会话合并视图。走查中两次"失败"均为 fixture 侧车不合 `StoredConversation` schema（schema_version 数字≠字符串、Message 必填 tool_call_id/tool_calls/reasoning_content）——管线如实报错，行为正确。
- 走查附带修复：横幅 flash 过 `sanitizeText`（子进程 stderr 透传的 ANSI 转义码不再直入横幅）。
- 本批仍未提交／未推送／未重建载体；索引无召回路由变化不 bump。

关键词：批六、回档、删除、组级功能键、选中模型、unarchive、delete-session、会话段门、RUN-CLI 精确匹配、LOCALAPPDATA PATH、工作区行。

---

# 批七（同日 2026-09-25 第九用户令）：prompt「Internal error」四层根因修复 ＋ 信任窗 Web 化 ＋ `--real` ＋ 大栈 ＋ 单击即开修正

> 用户令：「在使用中存在问题，输入问题并提交后显示[错误] Internal error」＋「是否考虑反向查看，看信任窗口中起进程的过程会注入什么？」＋「现在点击其他对话，主窗口不会切换了」

## 24. 诊断（四层叠加根因，逐层实证）

- **① ACAF 签名器 env 未配置**：`orz --stdio` 子进程 fail-closed 拒跑（`assurance invariant: ACAF fail-closed is enabled but no signer client is configured (ORZ_ACAF_MANIFEST + ORZ_ACAF_KEYSTORE)`）。ACAF 三 env 历来逐次启动内联设置（052 审计命令形态），桥/子进程无继承源。stdio 直接复现实锚。
- **② 工作区未授信＋Web 链无信任窗**（用户反查提示命中）：orz-bin main 的 L1 写入位置重定向（2026-08-08）把 `$GROK_HOME` 指到安装目录沙盒（`target\debug\grok-home\`，空存储）；`D:\CLI` 带仓库配置（`.grok/roles`）→ `decide()` 非交互（stdio 子进程 `is_interactive=false`）fail-closed `Untrusted`。TUI 车道同一输入因**交互式信任窗**（`persist_trust`→`set_trusted`）授信后通过——Web 链缺这个窗，授信失败只剩裸「Internal error」。
- **③ 假模型**：桥 spawn `orz --stdio` 未带 `ORZ_REAL`，真实 prompt 拿到 `(fake) 已收到请求`（与 TUI `--real` 显式约定一致，但 Web 面从未接线——S3 真机首读未开工的实测暴露）。
- **④ 栈溢出崩溃**：真实网关下 `run_agent_loop` 巨型 future（0bt 未竟拆分项②）首 poll 打穿 Windows 主线程默认栈（实锚 `thread 'main' has overflowed its stack`，0xC00000FD，进程静默消失、无终态 journal 事件）。
- 诊断基建缺口：agent 把真实原因放在 JSON-RPC `error.data`，前端只读 `error.message` ⇒ 一切失败都显示成「Internal error」。

## 25. 修复面（批七）

- **错误细节透出**（acp.js `errorMessage`）：`error.data` 并入错误消息，用户可见真实原因（本批即靠它在浏览器里露出②③）。
- **ACAF 启动检查**（lib.rs）：`orz web` 启动台面检测三 env 缺失并印警告行（提示从已 provision 终端启动 / `orz-acaf-provision` 回显）。
- **信任窗 Web 化**：`POST /api/trust` → 桥 spawn **`orz trust <cwd>`**（新子命令）→ orz-host `grant_workspace_trust(_with)`：授信写入与 prompt 子进程同一 GROK_HOME 下的信任存储（redirect 链由 orz-bin main 统一注入、子进程继承同值——同链同存储）；前端 `submitPrompt` 捕获 `workspace not trusted` → 确认弹窗 → 授信 → **自动重发一次**；授信为用户决定（弹窗在先），桥自带工作区路径（浏览器零路径输入）。
- **`orz web --real`**（lib.rs）：置位时桥进程设 `ORZ_REAL=1`，子进程 env 继承走真实 DeepSeek 通道（与 TUI `--real` 同一决策点 `build_gateway`）；启动台面新增「模型传输：真实（--real）/真实（继承）/⚠ fake（测试替身）」三态行——fake 回应不再可能被误读为成功。
- **主线程大栈**（orz-bin main）：整个 main 体搬上 **64 MiB** 显式栈线程，join 透传退出码——④ 的栈溢出实测消除（真实轮「收到」回轮全通过）；长期治本仍＝0bt ② run_agent_loop 拆分。
- **单击即开修正**（widgets.js，用户报告「点击其他对话，主窗口不会切换了」）：会话行**单击＝打开主窗口＋同时选中**（高亮保持、组级功能键照常可用）——撤销批六「单击仅选中、双击打开」的中间态；Enter 同语义。

## 26. 验证读数（批七）

- 冒烟门全绿（＋api 客户端断言＋单击即开/选中钉，撤「再次单击取消选中」旧钉）；orz-web **42/42**；orz-host session 模块 **7/7**（＋授信翻转/路径门钉）；clippy 本批文件零新增。
- **真机全链**（真实工作区 D:\CLI，`orz web --real`）：浏览器 prompt → 信任确认弹窗（文案如实）→ 授信落沙盒存储 → **自动重发 → 真实模型回轮「收到」＋运行完成**；探索器「已信任工作区（1）」沙盒授信可见；工具栏/组级功能键/单击即开全链回归通过。
- 观察项（不修，留观）：真实轮偶发长延迟（首实测 >3 min 后自行完成，复测秒级）——模型排队/推理时长形态，非挂死（journal 事件持续推进可证）；如复现加密观测。
- 走查方法教训（追加）：goto 仅变 hash 不重载页面（批五已登记）与本批再踩一次——换 token 测试必须换全新标签页。

## 27. 批七边界

- 本批仍未提交／未推送／未重建载体；索引无召回路由变化不 bump。
- ACAF 三 env 已写入**用户级环境变量**（配合批六 `%LOCALAPPDATA%\orz\bin`＋用户 PATH）——新终端 `orz web --real` 即完整可用；删除该三枚 env 即回退逐次内联形态。
- `D:\CLI\.gsa` 新增本批验证运行 journal（RUN-CLI-6ab69637 等）属真实狗粮痕迹，如实保留。

关键词：批七、Internal error、error.data 透出、ACAF 启动警告、信任窗 Web 化、orz trust、--real、ORZ_REAL、64 MiB 主线程栈、单击即开、模型传输三态行。
