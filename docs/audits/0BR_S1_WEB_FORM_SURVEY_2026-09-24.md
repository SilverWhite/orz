> 状态：`current`；批次＝**0br S1 勘定（只读）**（BACKLOG/TODO P1-0br，2026-09-24 用户令「先进0br，请开始」）；设计权威＝综合稿 [`UI_FORM_CONSOLIDATED_DESIGN_2026-09-24.md`](../UI_FORM_CONSOLIDATED_DESIGN_2026-09-24.md) §4/§6 ＋ 三稿（2026-09-24 解冻）＋ [`ADR-0010 §14.78`](../../adr/ADR-0010-vol-14-addenda-index.md)；性质＝只读勘定与选型定稿建议，零代码改动。

# 0br S1 Web 形态勘定（pager 族闭包 · 改名映射 · 同代性 · 复用映射定稿 · Web 桥与前端选型 · 分发面与安全边界）

## 1. 调查范围与一手证据

| 勘定点 | 一手来源 | 结论位置 |
|---|---|---|
| pager 族在树状态 | `orz/crates/` 目录清单、workspace `members`/`[workspace.dependencies]`（orz `Cargo.toml`） | §2 |
| 上游身份 | `upstream/grok-build.lock.json`（source_rev `6372e41d…`）、`orz/SOURCE_REV`（逐位一致）、上游 `xai-org/grok-build` crates/codegen 目录实测（84 目录，2026-09-24） | §2.3 |
| 改名映射先例 | 存档 `FORK_IMPLEMENTATION_DESIGN_v0.1.md` §1.1–§1.3 ＋ 在树事实（xai-grok-markdown→orz-markdown 等） | §3 |
| 复用映射现状 | 综合稿 §4 ＋ orz-tui 全源（`acp_client.rs`／`projection.rs`／`view_model.rs`／`widgets.rs`／`app.rs`／`modals.rs`／`dialogs.rs`／`journal_tail.rs`） | §4 |
| Web 桥依赖面 | workspace 已有 `axum 0.8（macros+ws）`、`tokio-tungstenite 0.27`、`agent-client-protocol 0.10.4（unstable）`；`orz-host::acp_server::AcpServer` 公开面；`orz --stdio` 入口（orz-bin `main.rs:187`） | §5 |
| 协议面 | `orz/THIRD-PARTY-NOTICES`（PART I 逐包＋PART II 许可文本）、父仓 `NOTICE`、`upstream/fusion-component-register-v0.1.yaml`（65 件） | §4.3／§7 |
| 外观照搬件版本与许可 | npm registry 实测（2026-09-24）：`98.css 0.1.21 MIT`、`xp.css 0.2.6 MIT`（98.css 扩展，dist 含双主题）、`marked 18.0.14 MIT` | §2.2／§4 |

## 2. pager 族依赖闭包与缺件表

### 2.1 在树件（同代、可用）

| 件（orz 名） | 在树路径 | 状态 | Web 阶段用途 |
|---|---|---|---|
| `orz-markdown`（＝上游 `xai-grok-markdown`，`authors=["xAI"]`，994 KB） | `orz/crates/codegen/orz-markdown` | 在树**休眠**——在 `[workspace.dependencies]`（`Cargo.toml:270`）但无任何成员引用、不在 members ⇒ 工作区构建不编译 | **不用**（ratatui 渲染面，见 §4 登记 R-1） |
| `orz-markdown-core`（45 KB） | `orz/crates/codegen/orz-markdown-core` | 同上休眠 | **不用**（同 R-1） |
| `xai-ratatui-textarea`（564 KB） | `orz/crates/codegen/xai-ratatui-textarea` | 在役（orz-tui 已接线） | **不用**（TUI 件；Web 用原生 textarea，登记 R-2） |
| `xai-acp-lib`（109 KB） | `orz/crates/codegen/xai-acp-lib` | 在役（orz-tui `acp_client.rs` 经 `acp_gateway` 走 in-process duplex） | **桥侧复用**（stdio 行帧泵语义，§5） |
| `orz-tui`（516 KB） | `orz/crates/orz-tui` | 在役 | **交互语义母本**：`projection.rs`（RunEvent/TuiEvent→视图变更）、`view_model.rs`（`ContentItem = Message | ToolTrace`——恰为三稿 ContentPane「有边框对话／无边框机器动作」数据模型）、`app.rs`（MenuBar/Toolbar/ContentPane/Marker/StatusBar/输入行、`nav_back/nav_forward` 对象历史、`show_explorer/show_marker` 大主窗开关、权限队列、模态 Help/Find/Properties/Snapshots）、`widgets.rs`（两行顶栏布局：Row0 菜单、Row1 工具栏＋右对齐「命令.../查找...」） | 
| `agent-client-protocol 0.10.4（unstable）` | workspace 依赖 | 在役 | ACP 语义权威（桥与前端共用） |

### 2.2 缺件表（未入树，全部为 **TUI 后补**阶段件；Web 阶段零阻塞）

| 缺件（上游名） | 按改名规则 | 上游存在性（实测） | 规模 | 入树阶段 | 备注 |
|---|---|---|---|---|---|
| `xai-ratatui-inline` | `xai-ratatui-inline`（非 grok 前缀，**不改名**） | 上游在（crates/codegen） | — | TUI 后补 | 终端 inline 渲染底座（wrapping/osc8/clipboard/prompt_images） |
| `xai-grok-pager-render` | `orz-pager-render` | 上游在 | — | TUI 后补 | TUI 渲染层 |
| `xai-grok-status-line` | `orz-status-line` | 上游在 | ≈39 KB | TUI 后补 | 状态行组件；**Web 侧不用**（StatusBar 自研，§4） |
| `xai-grok-pager`（`views/`、`scrollback/`、`app/`） | `orz-pager` | 上游在（另有 `pager-diff`／`pager-minimal`／`pager-bin`／`pty-harness` 关联件，fork 稿 §1.1 已裁 `pty-harness` 不需要） | ≈15.4 MB | TUI 后补（S4 盘点） | 最大块照搬件 |
| `xai-grok-mermaid` | `orz-mermaid` | 上游在（fork 稿 §1.1 曾裁「不需要」） | — | 后置 | Web 侧如需 mermaid 图渲染走 `mermaid.js`（前端）；TUI 侧后补再议 |
| **98.css / XP.css / marked** | —（npm 件，不改名） | `98.css 0.1.21`／`xp.css 0.2.6`／`marked 18.0.14`，均 MIT | CSS 数十 KB／JS 数十 KB | **Web S2 引入**（vendored） | 唯一 Web 阶段新增件，见 §4 |

### 2.3 同代性核对

- **代际锚**：`orz/SOURCE_REV` ＝ `6372e41d828b8a6ee82c29e01a69e27ec895cca9` ＝ `upstream/grok-build.lock.json.source_rev`（observed_at 2026-07-30，grok-build 0.2.112，`build_repo_commit 500129c7`）——**逐位一致**。
- 在树 xai 血统件（含休眠 orz-markdown 两件）全部属该代导入；`orz-markdown` 声明 `authors=["xAI"]`（沿 §14.78 条 4 单 crate 来源标注纪律），其依赖（ratatui 0.29／pulldown-cmark 0.13／syntect 5.3 等）全部解析到现行 workspace 依赖表 ⇒ 后补接线时无版本代沟。
- 上游 `main` 今日为 84 个 codegen 目录，UI 族件名实测在列（§2.2）；**TUI 后补阶段照搬缺件时按锁定代 `6372e41d` 取件，不取上游 main**（结论代际纪律，ORZ-VERDICT-EPOCH-001 同族）。
- Web 阶段**零新增 Rust 血统件**——不存在代际风险面；三件 npm 件为首次采用（当前版本 pin，随 S2 登记）。

## 3. 改名映射表（`xai-grok-X → orz-X`）

**通名规则（在树先例归纳）**：上游 `xai-grok-X` → `orz-X`；非 grok 前缀的 `xai-X`（`xai-acp-lib`／`xai-ratatui-textarea`／`xai-tty-utils`／`xai-ratatui-inline`）**保持原名**。

| 上游（grok-build @6372e41d） | orz 现名 | 状态 |
|---|---|---|
| xai-grok-markdown / xai-grok-markdown-core | orz-markdown / orz-markdown-core | 在树休眠 |
| xai-grok-config / -config-types / -env / -paths / -http / -models / -mcp / -hooks / -secrets / -sandbox / -workspace（-client/-types）/ -tools / -tools-api / -version / -shared / -agent / -telemetry / -shell | orz-config / -config-types / -env / -paths / -http / -models / -mcp / -hooks / -secrets / -sandbox / -workspace（-client/-types）/ -tools / -tools-api / -version / -shared / -agent / -telemetry / -shell（部分按 fork 稿合并/改组） | 在树 |
| xai-acp-lib / xai-ratatui-textarea / xai-tty-utils | 同名保留 | 在树在役 |
| xai-grok-pager | orz-pager | **缺件（TUI 后补）** |
| xai-grok-pager-render | orz-pager-render | **缺件（TUI 后补）** |
| xai-grok-status-line | orz-status-line | **缺件（TUI 后补）** |
| xai-ratatui-inline | xai-ratatui-inline（不改名） | **缺件（TUI 后补）** |
| xai-grok-mermaid | orz-mermaid | **缺件（后置）** |

## 4. 三稿区块 × 复用件 × 照搬/自研 映射表（Web 形态细化定稿）

> 综合稿 §4 映射表按「Web／TUI 两形态」细化；**处理判据不变**：每区块标注来源件，有来源件而不照搬须在此登记理由（§4.2）。TUI 列＝后补阶段适用；本表 Web 列为 S2 施工面。

### 4.1 映射表

| 三稿区块 | Web 复用件（来源） | 处理 | TUI 侧对应件（后补用） |
|---|---|---|---|
| 窗体框架／标题栏／窗钮／按钮／输入框／下拉／选项卡 | `98.css 0.1.21`（MIT，vendored；`XP.css 0.2.6` 作可选主题） | **照搬** | pager-render/appearance |
| MenuBar | 98.css 菜单原语＋自研结构（`orz-tui/widgets.rs` Row0 语义移植） | 照搬原语＋自研 | 同左 |
| 两行顶栏（行1＝全局菜单/显示开关/模式入口；行2＝即时操作/短命令/状态摘要） | 自研（`orz-tui/widgets.rs render_toolbar`＋`app.rs` view toggles 语义移植） | **自研**（§4.2 R-3） | 同左 |
| Toolbar | 自研（同上） | 自研（R-3） | 同左 |
| AddressBar（对象 URI＋`command://`＋Back/Forward 对象历史） | 自研（`orz-tui/app.rs nav_back/nav_forward` 语义移植） | 自研（R-4） | 同左 |
| FindBar（Find＋Scope） | 自研（`orz-tui Modal::Find` 移植） | 自研（R-3） | 同左 |
| ExplorerPane（会话/运行/对象树） | 自研（`orz-tui/explorer.rs` 语义移植）；数据源＝桥只读 API（§5.2） | 自研（R-5） | 同左 |
| ContentPane 对话渲染（有边框卡片＝对话；无边框工具行＝机器动作；pin-to-top 展开；流式追加） | 自研（三稿《CONTENT_PANE_CONVERSATION_RENDERING_v0.1》规则＋`orz-tui/view_model.rs ContentItem` 模型移植） | 自研（R-5） | orz-markdown（框内 Markdown 渲染） |
| ContentMarker | 自研（`orz-tui ContentMarker` 移植） | 自研（R-5） | 同左 |
| StatusBar | 自研（`orz-tui StatusBar` 中文状态项移植） | 自研（R-3） | `orz-status-line`（缺件，后补时评估） |
| 输入行／多行输入 | 自研（原生 `<textarea>`＋IME；bracketed-paste 等 TUI 关注点 Web 不适用） | 自研（R-2） | `xai-ratatui-textarea`（在役） |
| 权限弹窗（含倒计时、队列） | 98.css window 原语＋自研（对接 ACP `session/request_permission`；语义照 `orz-tui PendingPermission` 队列/倒计时/DialogAction 三值） | 自研（R-6） | 同左 |
| Help overlay（分页 modal，六 tabs：快捷键/命令/模型/审批/终端/来源） | 98.css tabs 原语＋自研（默认中文） | 自研（R-3） | 同左 |
| Properties／命令面板／快照选择 | 自研（`orz-tui Modal::{Properties, Snapshots}` 移植） | 自研（R-3） | 同左 |
| Markdown 渲染 | `marked 18.0.14`（MIT，vendored）；代码高亮／LaTeX／mermaid **后置**（不进首版判据） | **照搬**（R-1 登记：代换 orz-markdown） | orz-markdown / orz-markdown-core（在树休眠） |
| ACP 连接与事件流 | 桥＝自研 `orz-web`（Rust，axum）；帧泵复用 `xai-acp-lib` 行帧语义；前端＝自研最小 ACP client（JSON-RPC over WS，客户端角色同 `orz-tui::acp_client`） | 照搬语义＋自研传输 | `orz-tui/acp_client.rs`＋`xai-acp-lib` |
| journal 事实投影（运行事件流） | 桥侧复用 `orz_assurance::journal` RunEvent（JSONL 原样转发）＋前端 projection（移植 `orz-tui/bridge.rs`＋`projection.rs` 映射语义） | 照搬＋适配 | 同左 |
| 会话/运行历史只读投影 | 桥只读 API（扫 `.gsa/runs/`、`.gsa/conversations/`；语义照 `orz-tui/snapshot(s).rs`） | 照搬＋适配 | 同左 |

### 4.2 自研理由登记（承接综合稿 §7，新增 R-1/R-2）

- **R-1（Markdown 代换）**：综合稿 §4 的 `orz-markdown` 是 **ratatui 单元渲染器**（终端 cell 面），不产出 DOM——Web 侧不可用。Web 换 `marked`（MIT，vendored）；`orz-markdown` 保持 TUI 后补阶段的照搬件，两形态各自照搬、不冲突。
- **R-2（输入编辑器代换）**：`xai-ratatui-textarea` 是 TUI 件（bracketed paste／终端鼠标），Web 原生 textarea 已含 IME／粘贴／撤销；照搬无对象。
- **R-3**（两行顶栏／工具栏／查找／状态栏／Help／属性等布局件）：三稿特有形态，上游无可复用件（综合稿 §7 原判，Web 侧交互语义照 orz-tui 移植，视觉走 98.css）。
- **R-4**（AddressBar `command://` 语义＋对象历史栈）：三稿特有语义，无上游对应物。
- **R-5**（Explorer／ContentPane／Marker 的 orz 对象映射）：数据源是 orz session/journal/黑板/证据账，须自研投影映射（消费语义复用 `orz-tui/projection`）。
- **R-6**（审批弹窗对接 `session/request_permission`）：协议语义已有（`xai-acp-lib`／`orz-host`），呈现层自研。

### 4.3 协议合规清单（S2 随搬随办，判据 ⑥ 的执行面）

1. `orz/THIRD-PARTY-NOTICES` 增三件 PART I 条目：`98.css 0.1.21`（MIT，jdan/98.css）、`XP.css 0.2.6`（MIT，botoxparty/XP.css）、`marked 18.0.14`（MIT，MarkedJS）——MIT 版权文本入 PART II。
2. vendored 文件头部保留来源与版本注释；`marked` 保留其 LICENSE 头。
3. `upstream/fusion-component-register-v0.1.yaml` 新增条目（随搬随登记）：`orz-web`（桥 crate）、`98.css`／`xp.css`／`marked`（vendored 前端件）；`orz-markdown`／`orz-markdown-core` 保持既有条目不因休眠变动。

## 5. Web 桥与前端工程选型（定稿建议）

### 5.1 桥形态：`orz-web` crate ＋ `orz web` 子命令

- **新 crate `orz/web`：`orz/crates/orz-web`**（workspace member），`orz-bin` 增 `web` 子命令分发（现分发面：`--stdio`（main.rs:187）／`-p`／裸跑 TUI；`web` 槽位同层）。
- **agent 侧不改动**：桥 spawn `<current_exe> --stdio` 子进程（标准 ACP agent），对 orz 零侵入——与外部 ACP 客户端同构，`§14.78` 条 5「不得建立第二套产品 runtime」天然满足。
- **依赖零新增**：服务器用既有 workspace 依赖 `axum 0.8（macros+ws）`（orz-mcp/computer-hub 七件已在用）；ACP 帧泵复用 `xai-acp-lib`；静态资源嵌入用 `include_dir`（或 `rust-embed`，S2 定，均 MIT/Apache 二选一随登记）。
- **数据通道（两 WS＋只读 REST）**：
  - `GET /` ——静态前端；
  - `GET /ws/acp?token=…` ——一连接一 `orz --stdio` 子进程；JSON-RPC 文本帧 1:1 双向泵（ACP stdio 为行分隔 JSON ⇒ 一帧一 WS message）；**单会话策略**：已有活动连接时新连接明确拒绝（本地单用户产品面；连接关闭 ⇒ 桥杀子进程——桥是直接父进程，0bq 关切面在桥内闭环）；
  - `GET /ws/journal/{run_id}?token=…` ——`events.jsonl` 活动尾（50 ms 轮询语义照 `orz-tui/journal_tail.rs`），行原样转发；
  - `GET /api/runs`／`GET /api/run/{id}/events?from=`／`GET /api/conversations` ——只读扫描 `.gsa/runs/`、`.gsa/conversations/`（历史恢复与 Explorer 数据源；无任意文件读 API）。
- **S2 待确认设计点（不阻塞 S2 开工）**：① journal-run 归属关联——stdio 车道 run id 为 `RUN-{session8}-{n}`（`acp_server.rs:1720`），桥可按目录扫描＋前端按会话 id 前缀订阅最新 run（精确逐 prompt 关联在 S2 定案）；② 黑板/块表/指南指针投影的具体事件面（沿 journal `mechanical_audit`/`context_compressed` 族，前端映射表 S2 定）。

### 5.2 前端工程选型：零构建 vanilla + vendored 三件

- **vanilla ES Modules，零工具链**（无 npm build/打包器/框架）：`index.html ＋ app/*.js 模块`；投影层以纯 JS 类移植 `orz-tui projection.rs` 语义（TuiEvent→视图变更、`ContentItem = Message | ToolTrace`、折叠策略＝`RUN_FINISHED` 时 `collapse_non_warnings`）。
- **vendored**：`vendor/98.css`（0.1.21）＋`vendor/XP.css`（0.2.6，可选主题）＋`vendor/marked`（18.0.14）。运行时零 CDN、离线可用。
- **理由**：① 照搬纪律下无框架来源件，vanilla 使供应链面最小（三件全部可 pin＋登记）；② 98.css 是纯 CSS 无 JS，与 vanilla 天然同构；③ 零构建使「每区块标注来源件」的审查面为纯文本可查。

## 6. 分发面与安全边界（综合稿 §8 风险项的 S1 定稿建议）

1. **回环-only**：默认 bind `127.0.0.1`；`--addr` 只接受回环地址，非回环＝启动报错（不留「0.0.0.0 需显式」的弱口子——v1 一律回环，外部可达性为零）。
2. **端口**：默认 `21487`（S2 确认；workspace 无冲突默认值），占用则自动改配短暂端口；实际 URL 打印到控制台。
3. **每启动令牌**：`orz web` 启动生成随机 token，WS 路径与 API 均要求 `?token=`；控制台打印带令牌完整 URL——防本机其他用户/进程探测连接。
4. **Origin/Host 校验**：WS 升级校验 `Origin`（拒绝非本机来源网页的跨站驱动——DNS rebinding/CSRF 防线；令牌为第二防线）。
5. **无 TLS、无账号**（回环单用户）；**读面限定**：只读 API 仅暴露 `.gsa` 投影面（runs 清单／journal 切片／conversations 清单），无任意路径读。
6. **子进程生命周期**：WS 关闭/桥退出 ⇒ 杀 `--stdio` 子进程（桥为直接父）；桥崩溃残留由既有「stdin EOF ⇒ 退出」契约兜底（`FUS-ACP-STDIO-PARENT-BINDING` 实测印证该契约健康）。
7. **VS Code webview 宿主**：可选宿主后置（S4 盘点）；webview 同样走回环＋令牌，边界不变。

## 7. 判据清单（S1 定稿，承接综合稿 §6 草案）

1. **三稿区块齐备**：§4.1 映射表逐项在 Web 端存在（S3 逐项对照）。
2. **来源件可核**：每区块可指到具体件（vendored 文件＋版本＋sha256；自研件指到本表 R-编号与 orz-tui 移植源）。
3. **默认中文＋Help 分页**：UI 文案中文；Help 为 modal overlay、六 tabs、`Esc` 关闭。
4. **核心操作全键盘可达**：`F6` 切大区、`Tab/Shift+Tab` 区内移动、`Enter/Space` 激活、`Esc` 关弹窗、`Ctrl+L` 命令/位置、`Alt+字母` 开菜单（补充稿 §10）。
5. **UI 零执行事实**：前端无权限判定、无 session 归属/持久化/restore 决策（restore 仅透传 host 既有 ACP 语义）；不建第二套 runtime。
6. **协议面齐备**：§4.3 三项办结（NOTICES 条目、vendored 来源注释、组件登记）。
7. **分发面零外部可达**：回环-only＋令牌＋Origin 校验实测通过（外部探针连接被拒）。
- **S3 真机判据**（沿 TODO）：Web 端真机会话含**审批、长任务、并行**三形态；逐区块对照三稿（存在性/语义/默认中文/键盘可控）；流式追加与折叠策略（`RUN_FINISHED` 收折）实测。

## 8. 收敛估算与批序

| 项 | 估算 |
|---|---|
| `orz-web` 桥 crate（axum router＋WS 泵×2＋journal 尾＋3 只读 API＋资源嵌入＋`orz web` 子命令接线） | ≈600–900 行 Rust ＋ 钉子（回环-only／令牌拒绝／Origin 拒绝／连接-子进程生命周期） |
| 前端壳（index＋menubar/toolbar/explorer/content/marker/statusbar/input＋modals×4＋keymap＋acp client＋projection） | ≈2,500–4,000 行 vanilla JS＋CSS（vendored 三件另计） |
| 协议面 | NOTICES＋登记表＋来源注释（§4.3） |
| 批序 | S2 落码（桥→壳→投影接线）→ S3 真机首读（真机会话＋逐区块对照；**前置＝载体重建使 `orz web` 进件**）→ S4 收口（判据入账＋TUI 后补盘点） |

## 9. 边界与风险（S1 部分）

- **终端内体验让位**（综合稿 §8）：Web 先行期间 orz-tui 维持现状；TUI 后补时复用本表 TUI 列，不另起设计。
- **过渡面不变**：VS Code `formulahendry.acp-client` 仍为过渡面；`orz web` 为产品面候选，`FUS-UI-BOUNDARY` 定位（产品面＝orz-tui→本 Web＋TUI）随 0br 闭合时在索引对齐。
- **三稿正文年代**：三稿中的历史实现决策（`prompt_toolkit`／Python `widgets.py` 等）**不约束 Web 形态**——解冻的是形态效力（§14.78 条 1），实现载体由综合稿 §3 裁决（Web 优先）。
- **`orz-markdown` 休眠面**：本批不接线、不删除（TUI 后补阶段的照搬件）；同代性已核（§2.3），后补接线前补一次编译核验即可。

## 10. 入口

- 设计权威：[`UI_FORM_CONSOLIDATED_DESIGN_2026-09-24.md`](../UI_FORM_CONSOLIDATED_DESIGN_2026-09-24.md) / 三稿（`architecture/CLI_UI_INTERACTION_MODEL_v0.1.md`、`CLI_UI_SIMPLIFICATION_SUPPLEMENT_v0.1.md`、`CONTENT_PANE_CONVERSATION_RENDERING_v0.1.md`）/ [`ADR-0010 §14.78`](../../adr/ADR-0010-vol-14-addenda-index.md)
- 交互语义母本：`orz/crates/orz-tui/src/`（projection／view_model／app／widgets／modals）
- 上游身份：`upstream/grok-build.lock.json` ＋ `orz/SOURCE_REV`（`6372e41d…`）
- 待办：BACKLOG/TODO `P1-0br`；索引 `DESIGN-UI-FORM-CONSOLIDATED`

关键词：0br S1、pager 族缺件、6372e41d 同代、改名映射、orz-web、ACP-over-WebSocket、98.css、XP.css、marked、回环令牌、ContentItem、投影移植。
