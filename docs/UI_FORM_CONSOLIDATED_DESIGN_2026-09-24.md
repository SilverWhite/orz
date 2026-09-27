> 状态：`current-design`；立项＝ BACKLOG／TODO `0br`（2026-09-24 用户令）；裁决转录＝[`ADR-0010 §14.78`](../adr/ADR-0010-vol-14-addenda-index.md)（v1.80，2026-09-24）
> **本稿性质**：**执行形态与复用映射综合稿**——UI 形态本身由三份 UI 设计稿定义，本稿只指向、引用与落地，不改写其正文。

# UI 形态综合设计稿（Web 优先 · 复用照搬优先）

## 1. 权威链与本次裁决

- **形态权威（2026-09-24 解冻）**：三份 UI 设计稿——[`CLI_UI_INTERACTION_MODEL_v0.1`](../architecture/CLI_UI_INTERACTION_MODEL_v0.1.md)（工作台空间语言与全局区域）、[`CLI_UI_SIMPLIFICATION_SUPPLEMENT_v0.1`](../architecture/CLI_UI_SIMPLIFICATION_SUPPLEMENT_v0.1.md)（两行顶栏／大主窗优先／默认中文／Help 弹窗）、[`CONTENT_PANE_CONVERSATION_RENDERING_v0.1`](../architecture/CONTENT_PANE_CONVERSATION_RENDERING_v0.1.md)（ContentPane 对话与机器动作的渲染规则）。
- **定位权威**：[`ADR-0010 §2.4 条 8`](../adr/ADR-0010-vol-02-core-decisions.md)（UI 是投影层；主 UI 为 `orz-tui` assurance workbench；Codex TUI/app-server 可作 fallback；两者必须消费同一 host/loop/journal 事实，不建第二套产品 runtime）与 [`§2.6`](../adr/ADR-0010-vol-02-core-decisions.md)（Toolbar 与只读 session/run-history 投影为 presentation baseline；UI 不拥有执行事实、permission、session persistence 或 restore）。
- **复用权威**：`FUS-CORE`（成熟组件优先）＋ 本稿 §4 映射表（"能照搬就照搬"）。
- **本次用户裁决（2026-09-24；五句逐字转录以 [`ADR-0010 §14.78`](../adr/ADR-0010-vol-14-addenda-index.md) 为准）**：① UI 的具体形态就用那三份设计稿；② 三份稿子从冻结状态里拽回来；③ 搬 xai 并以 xai 作为基础就是为了复用成熟组件，能复用的当然直接复用，遵守开源协议；④ 先做 Web、后补 TUI 形式。

## 2. 形态摘要（引自三稿，细节以原文为准）

- **空间语言**：Explorer／早期 IE 式工作台——全局区域＝MenuBar、Toolbar、AddressBar、FindBar、ExplorerPane、ContentPane、ContentMarker、StatusBar，以及模态面（Help、Properties、权限、命令面板）。
- **简化与中文**：两行顶栏（第一行＝全局菜单／显示开关／模式入口，第二行＝即时操作／短命令／状态摘要）；大主窗优先（简单任务可全隐藏侧栏）；弹窗承载长输入与说明；键盘全可控、鼠标仅便利补充；**默认中文**，命令名／URI／schema／日志字段保留英文并进 Help；运行中展开、结束后折叠。
- **ContentPane 渲染**：**有边框＝自然语言对话**（`┌ [角色] · turn N ─────┐`，用户与模型同框异标签，Markdown 在框内展开，消息间空行、不截断）；**无边框＝机器动作**（工具调用行、展开清单）。这条"一条分界线"是对话与系统动作的视觉隔离规则。
- **地址栏语义**：地址栏表示 agent 原生对象（`workspace://`、`source://`、`run://`、`claim://`、`adapter://`、`artifact://`），Back／Forward 走**对象历史**；`command://…` 是绕过 AI 解释的直接命令入口；Address（改状态／导航）与 Find（只搜内容）＋ Scope（限定范围）三者语义分开。

## 3. 执行形态裁决：Web 先行、TUI 形式后补

- **Web 优先的理由**：三稿要求的旧时代桌面语言在 Web 侧有**现成照搬件**（`98.css` MIT 11.5k★、`XP.css` MIT 3.1k★）；三稿各区块在 Web 上是常规 DOM/CSS 工程；且不引入 Rust TUI 生态的源码搬运量。
- **runtime 不动**：orz 仍是标准 ACP agent（`orz --stdio`）。Web 侧新增 **ACP-over-WebSocket 桥**作为 transport，前端不接触执行事实。
- **宿主形态**：默认本地服务（**回环监听**）承载桥与静态前端；VS Code webview 作为可选宿主。分发面（端口、回环、启动方式）在 S1 定稿。
- **投影纪律不变**：前端只消费 host/loop/journal 投影（会话、事件、黑板、审批、RLI 提醒、压缩回执、收尾自述）；权限判定仍由 permission gate 与 ACP `session/request_permission` 决定，前端只呈现与回应（ADR-0010 §2.4 条 8／§2.6）。
- **TUI 形式后补**：形态同一套（同三稿），复用件共用（§4 同一映射表），避免两套设计分叉；后补顺序见 §6 批序末项。

## 4. 复用映射表（照搬优先）

> **机械判据（本稿强制）**：每个区块必须标注**来源件**；无来源件的区块必须在 §7 登记"自研理由"。凡有来源件而未照搬的，视为偏离本裁决，需在本稿登记理由。

| 三稿区块 | 可复用件（来源） | 处理 | 备注 |
|---|---|---|---|
| 终端渲染底座／滚动／换行／超链接／鼠标／剪贴板／图片 | `xai-ratatui-inline`、`xai-grok-pager-render`（wrapping／osc8／clipboard／prompt_images／appearance） | **照搬**（TUI 侧） | 缺件，纯照搬，无上游设计成本 |
| 多行输入编辑器（bracketed paste／鼠标） | `xai-ratatui-textarea` | **照搬（已在树）** | `orz-tui` 已在用 |
| Markdown 渲染（标题／列表／代码块／高亮／链接／LaTeX／mermaid） | `orz-markdown`（＝上游 `xai-grok-markdown`，`authors=["xAI"]`）、`orz-markdown-core`、`xai-grok-mermaid` | **照搬（已在树，未接线）** | 最大一块"已在手未用" |
| 主题／外观系统 | `orz-config` ＋ `orz-tui/theme.rs` ＋ pager-render/appearance | 照搬＋适配 | 主题值按三稿语义映射 |
| 状态行 | `xai-grok-status-line` | **照搬** | 缺件（39 KB） |
| 模态／设置／扩展面板／卡片视图 | `xai-grok-pager` 的 `views/`、`scrollback/`、`app/` | **照搬（TUI 侧）** | 最大块（15.4 MB），TUI 后补时启用 |
| ACP 连接与事件流、权限弹窗对接 | `orz-tui/acp_client`、`xai-acp-lib` | 照搬＋适配 | Web 桥复用同一 ACP 语义 |
| 会话／运行历史只读投影 | `orz-tui/projection`、`snapshots` | 照搬＋适配 | Explorer／会话列表数据源 |
| **旧时代桌面外观（Web）** | `98.css`（MIT）、`XP.css`（MIT） | **照搬** | 外观语言直接复用，不重画 |
| 两行顶栏／MenuBar／Toolbar／FindBar／Explorer／Markers／StatusBar 布局 | 无上游对应物 | **自研**（§7 登记） | 三稿特有形态 |
| AddressBar 的 `command://` 语义与对象历史 | 无上游对应物 | **自研**（§7 登记） | 含 Back/Forward 对象历史栈 |

## 5. 组件与协议合规（照搬的前置条件）

- **Apache-2.0（上游／xai 件）**：保留 `LICENSE`、`NOTICE`、`THIRD-PARTY-NOTICES`；单 crate 保留来源标注（沿 `orz-markdown` 的 `authors = ["xAI"]` 做法）；父仓 `THIRD-PARTY-NOTICES` 随搬件同步更新。
- **MIT（`98.css`／`XP.css`）**：保留版权声明与许可文本，纳入 `THIRD-PARTY-NOTICES`；前端产物保留来源注释。
- **组件登记**：新搬件随搬随登记进 [`fusion-component-register-v0.1.yaml`](../upstream/fusion-component-register-v0.1.yaml)（现状 65 件全 `audit_required`／`ownership: not_assessed`；"照搬"必须同步采用档位与来源，否则审查判为未登记采用）。

## 6. 批序与判据

- **S1 勘定（只读）**：pager 族依赖闭包与缺件表 ＋ 改名映射表（`xai-grok-X → orz-X`）＋ 同代性核对 ＋ **三稿区块 × 复用件 × 照搬/自研** 映射表定稿 ＋ Web 桥与前端工程选型 → 收敛估算并出判据清单。
- **S2 落码**：ACP-over-WebSocket 桥；前端壳（按 §4 照搬 `98.css`/`XP.css`）；投影接线（会话列表、工具卡片、机器动作行、审批弹窗、黑板／块表／指南指针、RLI 提醒、压缩回执、收尾自述、状态栏与 Markers）。
- **S3 真机首读**：Web 端真机会话（含审批、长任务、并行）；逐区块对照三稿（存在性／语义／默认中文／键盘可控）。
- **S4 收口**：判据入账；TUI 形式后补的排期与复用件盘点（若同批可做则并入，否则单独立项）。

**判据（草案，S1 定稿）**：① 三稿区块齐备；② 每区块的照搬来源可核（可指到具体件）；③ 默认中文＋Help 分页；④ 核心操作全键盘可达；⑤ UI 零执行事实（无第二套 runtime／权限判定／session 归属）；⑥ 协议面齐备（NOTICE／许可文本／组件登记）。

## 7. 自研理由登记（无复用件的区块）

> 2026-09-25 审查处置批增补 R-7 与两处形态选择登记（承接 S1 §4.2 R-1…R-6，编号续延）。

- **两行顶栏与全局区域布局**：三稿特有形态，上游 TUI 为自家布局，无可复用件。
- **AddressBar 的 `command://` 语义＋对象历史栈**：三稿特有语义，无上游对应物。
- **Explorer／Markers／StatusBar 与 orz 对象的映射**：数据源是 orz 的 session/journal／黑板／证据账，需自研投影映射（消费面复用 `orz-tui/projection`）。
- **审批弹窗与 ACP `session/request_permission` 的对接**：协议语义我方已有（`xai-acp-lib`／`orz-host`），呈现层自研。
- **R-7（语法高亮／LaTeX／mermaid 后置）**：ContentPane 稿 §3.1 将「语法高亮」列于框内渲染规则，Web v1 后置属对该形态权威条款的**显式豁免**（LaTeX／mermaid 系本稿行文自带、三稿原文未要求，不构成冲突）；S4 后补盘点时评估照搬件（如 highlight.js）。判据本身不能自我授权偏离形态权威，故在此登记。
- **形态选择·AddressBar／FindBar 弹窗化（Web v1）**：两栏无常驻区块，以行2 状态摘要＋「命令/位置」「查找」弹窗承载（补充稿 §5 弹窗化口径）；对象 URI 路由仅 `run://`／`workspace://live` 有投影，`conversation://` 给只读说明，`source://`／`claim://`／`adapter://`／`artifact://` 显式答复「v1 无投影面」不静默。S3 真机对照后如判形态不足再立补码批。
- **形态选择·快照选择器**：`restore_snapshot` 非 ACP 协议面（`AcpServer` 进程内方法），Web v1 为提示性横幅（「恢复未接线」），不建选择器面板；与 §14.78 条 5 一致，接线与否随 S3/S4 裁决。

## 8. 风险与边界

- **Web 分发面**：本地服务的监听范围（默认回环）、端口占用、启动方式、与 VS Code webview 的差异；S1 定稿并附安全边界（不得成为新的外部可达面）。
- **终端内体验让位**：Web 先行意味着终端内形态暂缓；TUI 后补时保持同一套形态与复用件，不另起设计。
- **与过渡面的关系**：外部 ACP 客户端（现 VS Code `formulahendry.acp-client`）仍是过渡面，本 Web 形态属**产品面**候选；与 `FUS-UI-BOUNDARY` 的边界在 S1 一并复核。
- **三稿正文年代**：三稿属早期基线，凡与本 ADR-0010 冲突处以 ADR 为准；本稿只解冻其**形态**效力。
- **三稿豁免清单（2026-09-25 处置批点名，替代 S1 §9 的「等」字兜底）**：① 补充稿 §1「不复刻老 IE 或 Win98 视觉外观」条款——与 §14.78 条 3 照搬 `98.css`/`XP.css` 的裁决冲突，**按 ADR 为准不适用**；② 交互模型稿 Windows-only/Windows Terminal 优先、100×30 最小视口、box-drawing 字符与中文宽度计算、alternate screen 等 TUI/终端约束——**不约束 Web 形态**；③ 补充稿 §13 冻结项「`Edit` 转为「编辑模式」」**已落码**（菜单词表用「编辑模式」），顶部中文菜单最终词汇仍在 §13 延期面，S3 词汇定稿收口；④ 历史实现决策（`prompt_toolkit`/Python `widgets.py`）沿用 S1 §9 豁免。
- **刷新/断线语义（v1 已知限制）**：桥为单会话且连接关闭即结束 `orz --stdio` 子进程 ⇒ 页面刷新＝活动 run 中断、ACP 会话上下文丢失（restore 非 ACP 面）；前端已有 `beforeunload` 确认守卫，重连/恢复路径留 S3/S4 裁决。
- **marked 供应链策略**：唯一解析不可信模型输出的 vendored 件（pin 18.0.14＋MANIFEST sha256 钉子＋嵌入表 digest 校验测试）；上游安全公告触发重 vendored 评估；前端净空（scheme 白名单/图片剥离）不依赖库默认行为。
- **投影双实现同步纪律**：`orz-tui/projection.rs`（TUI）与 `orz-web/assets/app/projection.js`（Web）为同一 journal 事件面的两份投影实现——journal 事件语义变更时**同批同步两处**，审查面＝双文件对照；长期单一语义源（如共享 schema 派生）留 S4 盘点。
- **渲染内存上限（形态豁免）**：ContentPane「不设展开高度限制」指卡片内容不截断；Web v1 另设条目数上限（3000 条/单工具清单 2000 条，超出裁最旧并计数提示）防长会话 DOM 无界——属性能边界而非渲染截断，完整事实恒在 journal（可 `run://` 回放）。

## 9. 入口

- 三稿：[`CLI_UI_INTERACTION_MODEL_v0.1`](../architecture/CLI_UI_INTERACTION_MODEL_v0.1.md) / [`CLI_UI_SIMPLIFICATION_SUPPLEMENT_v0.1`](../architecture/CLI_UI_SIMPLIFICATION_SUPPLEMENT_v0.1.md) / [`CONTENT_PANE_CONVERSATION_RENDERING_v0.1`](../architecture/CONTENT_PANE_CONVERSATION_RENDERING_v0.1.md)
- 裁决：[`ADR-0010 §2.4／§2.6／§14.78`](../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)
- 待办：[`BACKLOG 0br`](BACKLOG_AND_PRIORITIES.md) / [`TODO P1-0br`](../TODO.md)
- 索引：`DESIGN-UI-FORM-CONSOLIDATED`（§2 / §8）

关键词：UI 形态、三稿解冻、Web 优先、TUI 后补、照搬复用、98.css、XP.css、ACP-over-WebSocket、投影层、默认中文、ContentPane。
