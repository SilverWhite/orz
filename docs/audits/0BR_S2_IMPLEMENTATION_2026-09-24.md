> 状态：`current`；批次＝**0br S2 落码**（BACKLOG/TODO P1-0br；承接 [`0BR_S1_WEB_FORM_SURVEY_2026-09-24.md`](0BR_S1_WEB_FORM_SURVEY_2026-09-24.md)；2026-09-24 用户令「请直接进入落码阶段吧，暂时不提交不重建」）；性质＝实施回执。**未提交／未推送／未重建载体**。
> **2026-09-25 勘误指针（不改原读数）**：本回执 §1.1「嵌入表 19 项」实为 20 条 `asset!`（唯一文件 19）；「静态无令牌但 Host 门」在原码不成立（`/assets` 无门、`/ws/journal` 无 Origin）；§3 冒烟读数真实但只覆盖协议层——**前端存在两个 P0（main.js 严格模式未声明赋值死加载；令牌 fragment 格式桥/前端不匹配），UI 从未经浏览器级验证**；§4 判据①③④表述过宽。全部发现与处置（含修复与补验）见 [`0BR_S2_REVIEW_HANDLING_2026-09-25.md`](0BR_S2_REVIEW_HANDLING_2026-09-25.md)。

# 0br S2 Web 形态落码回执（桥＋前端壳＋投影接线＋协议面）

## 1. 交付物清单

### 1.1 桥 crate `orz-web`（新增，orz workspace member）

| 文件 | 职责 |
|---|---|
| `orz/crates/orz-web/Cargo.toml` | 依赖＝axum（workspace 既有 0.8 macros+ws）＋tokio＋futures＋serde/serde_json＋tracing＋rand；dev-dep＝tower（oneshot 钉子）。**零新增外部依赖项** |
| `src/security.rs` | 令牌生成/校验（256-bit hex）、回环 bind 校验、run_id 路径安全、Host/Origin 校验（纯函数＋钉子） |
| `src/assets.rs` | 显式 `include_bytes!` 嵌入表（19 项）；来源可核（钉子断言 vendor 版本头） |
| `src/runs.rs` | `.gsa` 只读投影：run 清单（头/尾探针轻解析＋终态归并）、journal 字节切片（半行扣留＋`next_offset` 续读）、conversation 清单 |
| `src/journal_tail.rs` | 50 ms 轮询活动尾（`orz-tui/journal_tail.rs` 移植；截断归零、缺文件 60 s 退避、只发完整行） |
| `src/acp_pump.rs` | WS↔`orz --stdio` 子进程 1:1 帧泵；单会话槽（`SessionSlot`）；stderr 入 tracing 防堵管；`kill_on_drop`＋显式 start_kill |
| `src/server.rs` | axum 路由：静态（`/`、`/assets/{*rest}`，无令牌但 Host 门）／`/ws/acp`／`/ws/journal/{run_id}`／`/api/runs`／`/api/run/{run_id}/events`／`/api/conversations`／`/api/boot`（回 cwd 供前端建会话） |
| `src/lib.rs` | `run(WebConfig)`＋`parse_args`（`--addr`／`--cwd`，默认 `127.0.0.1:21487`） |

### 1.2 `orz-bin` 接线

- `main.rs`：`args[1]=="web"` 分发（先于裸跑 TUI 兜底；`--real` 等 env 旗标随进程 env 传给子进程）＋`run_web()`（stderr tracing 姿态与 `run_stdio` 同源）。
- `Cargo.toml`：`orz-web = { path = "../orz-web" }`；workspace members 增 `crates/orz-web`。

### 1.3 前端（零构建 vanilla，全部 `orz/crates/orz-web/assets/`）

| 文件 | 区块来源（照搬判据标注） |
|---|---|
| `index.html` | 文件头注释逐区块标注来源件；窗体/控件＝98.css 照搬件 |
| `app.css` | 自研布局（R-3/R-5）；「有边框＝对话卡片、无边框＝工具行」分界线（三稿 §2）；pin-to-top 工具展开区（§3.3） |
| `app/state.js` | 视图模型（移植 `orz-tui/view_model.rs`：`ContentItem = Message | ToolTrace`、StatusBar 冻结六段、MenuBar/Toolbar 冻结中文词表、`nav_back/forward` 对象历史） |
| `app/md.js` | marked 照搬件包装（R-1）；**先 HTML 逃逸再 parse**（模型输出 HTML 按字面呈现，XSS 由构造排除） |
| `app/api.js`／`app/acp.js`／`app/journal.js` | REST＋WS 客户端（令牌经 `?token=`；ACP 客户端角色＝`initialize`→`session/new`→`session/prompt`＋`session/cancel`；接收 `session/update`＋`session/request_permission`，未实现请求回 -32601 不静默） |
| `app/projection.js` | journal 事件投影（移植 `bridge.rs` 防御式取字段＋`projection.rs` 中文文案与状态迁移：运行态/工具行/权限/门控/机械审查/上下文压缩/快照/票据/终态收折；流式去重＝「卡片全文与 journal `model_output` 精确一致」规则） |
| `app/widgets.js`／`app/dialogs.js`／`app/keymap.js`／`app/main.js` | 渲染器＋模态面（权限弹窗/Help 六冻结页/查找/属性/命令）＋键盘模型（F6/Esc 双击/Ctrl+L/Ctrl+F/Ctrl+Z/Alt+H/F1/`/`）＋装配 |

### 1.4 vendored 件（S1 §2.2 唯一 Web 新增面）

`vendor/98.css 0.1.21`、`vendor/XP.css 0.2.6`（两主题＋4 字体）、`vendor/marked.umd.js 18.0.14`——均 MIT、未修改、上游版本头在位；`MANIFEST.sha256.txt` 记录 7 文件 digest（98.css `2201559a…`／XP.css `b2a9ff8c…`／marked `21568877…`）。

### 1.5 协议面（判据 ⑥）

- `orz/THIRD-PARTY-NOTICES` 新增「PART I (continued) — BUNDLED WEB FRONTEND ASSETS (orz-web)」三条目（98.css／XP.css／marked；MIT 版权声明逐字录入，字体随主题归并）。
- `upstream/fusion-component-register-v0.1.yaml` 65 → **69** 条：`98css`／`xpcss`／`marked`（`origin.kind: vendored`＋project/version/license 事实；审计档位按冻结纪律保持 `not_assessed`）＋`orz-web`（`origin.kind: local`，首方桥）。YAML 解析核验通过。

## 2. 与 S1 勘定的偏差与登记

1. **`/api/boot` 端点（S1 未列）**：`session/new` 的 `cwd` 为 ACP 必填参数而前端无从得知 ⇒ 桥补一条令牌门控的 boot 事实端点（回桥侧工作区 cwd）。属 S1 §5.2「S2 待确认设计点」的落地，不改分发面边界。
2. **权限弹窗倒计时未实现**：`orz-tui` 的审批倒计时源自其进程内 pending 结构；ACP 请求帧无超时字段 ⇒ Web v1 弹窗无倒计时（如实呈现选项）。已记 S3 观察。
3. **快照恢复只读**：`restore_snapshot` 是 `AcpServer` 进程内方法、非 ACP 协议面 ⇒ Web `/snapshots` v1 为提示性展示（「恢复未接线」如实标注），与 §14.78 条 5（UI 不拥有 restore）一致。
4. **菜单词表**：沿 `orz-tui` 冻结九项（文件/事件/标记/编辑/模型/来源/运行/验证/帮助），补充稿「编辑模式」等最终词汇属其§13 延期项。
5. **Markdown 高亮/LaTeX/mermaid**：按 S1 判据口径后置，不进首版。

## 3. 验证读数（本机 Windows，2026-09-24）

- **测试**：`cargo test -p orz-web` → **21 通过 / 0 失败**（安全门 6＋runs 投影 5＋assets 2＋slot 1＋参数 1＋其余单元；含令牌拒绝/伪造 Host 拒绝/遍历 run_id 拒绝/半行扣留续读/vendor 版本头断言）。
- **lint/format**：`cargo clippy -p orz-web --all-targets` **0 警告**；`cargo fmt -p orz-web`＋`-p orz-bin` `--check` 干净。`clippy -p orz-bin` 仅既有基线警告（`orz-signer.rs:593`／`main.rs:1330` 等非本批代码），orz-web/orz-bin 新增面零新增警告。
- **构建**：`cargo build -p orz-bin`（debug，protoc=`orz/bin/protoc.exe`）通过。
- **HTTP 冒烟（真实进程）**：`orz web` 起服务后——静态 `/`＝200、`/assets/vendor/98.css`＝200；`/api/boot` 无令牌＝**401**、带令牌＝正确 JSON；`/api/runs`／`/api/conversations`＝空清单 200；URL 编码遍历 run_id＝**400**；伪造 Host＝**401**；伪 Origin WS 升级＝**401**。
- **WS 端到端（Node 24 内建 WebSocket）**：① journal 尾通道——写入 2 行的 `events.jsonl` 后订阅，**2/2 行逐字到达**（首行含 `run_started`）；② ACP 泵通道——WS 发 `initialize` ⇒ 桥 spawn `orz --stdio` 子进程 ⇒ 回传 `{"id":1,"protocolVersion":1}` **真回路闭环**；连接关闭后子进程随之退出（无孤儿）。
- 冒烟进程已清理；临时目录不涉及仓库面。

## 4. 判据对照（S1 §7；③–⑦ 待 S3 真机）

| 判据 | 状态 |
|---|---|
| ① 三稿区块齐备 | 壳已齐（S3 逐项对照） |
| ② 来源件可核 | ✅ 每文件头标注＋登记表＋NOTICES＋嵌入表钉子 |
| ③ 默认中文＋Help 分页 | 已落码（六冻结页；S3 键盘复验） |
| ④ 核心操作全键盘可达 | 已落码（keymap；S3 复验） |
| ⑤ UI 零执行事实 | ✅ 前端只消费 ACP/journal 投影；权限判定、restore 均不在前端 |
| ⑥ 协议面齐备 | ✅ §1.5 |
| ⑦ 分发面零外部可达 | ✅ 回环-only＋令牌＋Host/Origin 门（HTTP 冒烟实测拒绝面） |

## 5. 批序与边界

- **S3 真机首读**（待放行；**前置＝载体重建使 `orz web` 进件**——用户已明示本批不重建）：Web 端真机会话（审批／长任务／并行）＋三稿区块逐项对照＋判据 ③④ 复验＋`model_output`/流式去重实测。
- **S4 收口**：判据入账＋TUI 后补盘点（pager 族缺件照搬按锁定代 `6372e41d`）。
- 本批**未提交／未推送**；父仓索引/BACKLOG/TODO 记账随提交批一并办理。
