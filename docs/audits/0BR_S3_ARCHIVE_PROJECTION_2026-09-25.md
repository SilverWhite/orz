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
