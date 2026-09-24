> **UI 形态权威（2026-09-24 用户裁决解冻）**：本文件经用户令「那三个稿子要从冻结状态里拽回来」**解除 pre-ADR-0010 冻结（2026-09-24 批就地冻结的反向操作）**，与综合稿 [`UI_FORM_CONSOLIDATED_DESIGN_2026-09-24.md`](../docs/UI_FORM_CONSOLIDATED_DESIGN_2026-09-24.md) 共同构成 orz UI 的**形态设计权威**（执行形态＝Web 优先、TUI 形式后补；复用原则＝能照搬就照搬、遵守开源协议）；定位与冲突裁决仍以 [`ADR-0010 §2.4／§2.6／§14.78`](../adr/ADR-0010-fusion-runtime-and-agent-architecture.md) 为准。正文不回改（细节属早期基线、保留原样），仅解除冻结标注。

# CLI UI Simplification Supplement v0.1

状态：UI direction supplement；补充 `CLI_UI_INTERACTION_MODEL_v0.1.md`，用于指导下一轮 TUI 精简和中文化。

## 1. 背景

现有 `assurance/tui/` 已实现第一版 TUI 原型：菜单栏、工具栏、地址栏、查找栏、左侧 Explorer、中间内容区、右侧 Markers、底部状态栏、权限弹窗、属性弹窗、命令面板和 prompt_toolkit 交互 demo。

该原型证明了终端内多区域 UI 可行，但默认画面仍有过多常驻区域。个人单屏 AI 开发使用中，最大的阅读和输入区域比完整导航外壳更重要。本补充文档冻结下一轮优化方向：借鉴成熟桌面软件和文字聊天室的信息组织，不复刻老 IE 或 Win98 视觉外观。

## 2. 设计原则

- 默认中文：UI 文案、菜单和帮助默认使用中文。
- 英文保留：命令名、URI、schema、日志字段和英文别名保留，主要放在 Help 中。
- 大主窗优先：简单任务下应能完全隐藏非强制侧栏，形成单一大主输出/输入窗口。
- 两层顶部语义：菜单/模式入口与即时操作入口不合并，只做压缩和重排。
- 弹窗承载长输入和说明：Help、完整 address/command 输入、高级搜索等不挤占主布局。
- 键盘全可控：所有核心操作必须能通过键盘完成，鼠标只作为便利补充。
- 运行中展开、结束后折叠：工具调用、计划、审批和事件在任务进行时显示关键状态，任务结束后默认折叠。

## 3. 默认中文与 Help

UI 默认中文。Help 中保留英文选项、英文命令和快捷键别名，满足双语可发现性。

建议：

- 顶部菜单使用中文。
- 弹窗标题、按钮和状态栏使用中文。
- 内部对象 URI 保持英文，例如 `command://run/current-task`。
- slash commands 保持英文，例如 `/help`、`/plan`、`/models`，但显示中文名和中文说明。

Help 不应只是顶栏下拉菜单，也不应切换到新窗口。Help 应是 modal overlay，可分页、可关闭、可键盘导航。

Help 入口：

- 顶部 `帮助`。
- `Alt+H`。
- `/help`。

Help 关闭：

- `Esc` 关闭当前 Help 弹窗。

推荐 Help tabs：

- 快捷键
- 命令
- 模型
- 审批
- 终端
- 来源

Help 内容应中文优先，并可在右侧或括号中保留英文命令/术语。

## 4. 顶部区域

当前四行顶部结构为：

```text
MenuBar
Toolbar
AddressBar
FindBar
```

下一轮不做完全二合一，而是压缩为两层：

第一行：全局菜单、显示开关、模式入口。

候选：

```text
文件  事件  标记  编辑模式  模型  来源  运行  验证  帮助
```

第二行：即时操作、短命令入口、当前状态摘要。

候选：

```text
后退  前进  刷新  停止  打开  命令...       Plan/Manual | CPU/MEM | 当前模型
```

原则：

- 不把全局菜单和即时操作彻底混在一行。
- 可以缩短按钮文字，但不牺牲语义分区。
- `Open`、`Verify`、`Properties` 等高频入口可从旧 toolbar 迁移或重排。
- 第二行保留短命令/位置入口，不直接承载长路径或长命令文本。

## 5. Address 与 Command 输入

完整 address/path/command 输入应从常驻 address row 移入弹窗，避免长路径、长 URI 或多行输入导致顶部布局变形。

常驻入口只显示短摘要或单词级入口：

```text
命令...
位置...
当前: run/current-task
```

激活后打开弹窗：

- 可输入完整 `command://...`、`workspace://...`、`source://...`。
- 支持历史记录。
- 支持 slash command autocomplete。
- 支持确认、取消和详情预览。

关闭规则：

- `Esc` 关闭输入弹窗，不执行命令。
- `Enter` 确认。
- `Shift+Enter` 可用于多行命令内容，前提是弹窗明确支持多行。

## 6. Find

Find 保留，但主界面只显示短输入或摘要。长搜索、多行搜索、正则、范围组合和高级过滤走弹窗。

原则：

- Find 必须做多行输入适配，以防长 query 或复杂检索条件。
- 主界面不因 Find 内容过长而换行挤压主体。
- 高级搜索弹窗中可显示 scope、case sensitivity、regex、source visibility 过滤等选项。

## 7. 大主窗模式

左侧 Explorer、右侧 Events、右侧 Markers 都不是简单任务的强制需求。下一轮 UI 必须支持完全大主窗模式。

显示/隐藏入口：

- `文件`：控制左侧文件/对象 Explorer 的显示或打开文件相关弹窗。
- `事件`：控制事件栏显示/隐藏。
- `标记`：控制 markers 栏显示/隐藏。

所有侧栏都隐藏时，主体区域应成为完整宽度的大主窗口。

默认策略待定：

- 可默认显示三栏，以保持可发现性。
- 可提供大主窗默认配置，供个人高频使用。

无论默认如何，切换必须轻量、可键盘完成、可在 Help 中发现。

## 8. 菜单项调整

`Sources` 保留，因为来源可见性、证据链和 source gate 是本项目核心能力。

`View` 不再作为顶层必需菜单。其职责被拆分：

- 侧栏显示/隐藏交给 `文件`、`事件`、`标记`。
- 密度或布局配置可进入 Help 或设置。
- 折叠策略进入事件/主窗相关设置。

`Agent` 改为 `模型` / `Models`。

`模型` 用于：

- provider 切换。
- model 切换。
- thinking intensity / reasoning effort 切换。
- Plan/Manual/Auto 等运行策略入口的邻近展示。
- 当前 adapter 状态查看。

## 9. 编辑模式

传统 `Edit` 的 Cut/Copy/Paste 语义不适合本 CLI 作为顶层菜单中心。`Edit` 应转为 `编辑模式`。

候选内容：

- 插入/覆盖。
- 光标模式。
- 半角/全角。
- 粘贴模式。
- 自动换行。
- 多行输入行为。
- 输入法/终端兼容提示。

这使 `编辑模式` 成为输入行为选择器，而不是文档编辑菜单。

## 10. 键盘焦点

采用成熟桌面习惯：

```text
F6              切换大区域
Tab             当前区域内切换控件
Shift+Tab       当前区域内反向切换控件
方向键          在树、菜单、列表、tab 内移动
Enter / Space   激活当前项
Esc             关闭弹窗/菜单；运行中按既定策略取消当前动作
Ctrl+L          打开命令/位置输入
Alt+字母        打开对应顶层菜单
```

当前实现已使用 `F6` 切换 major panes，`Tab`/`Shift+Tab` 在 pane 内移动，`Esc` 关闭弹窗/菜单/输入缓冲，运行中可取消 run。Help 必须把这套规则写清楚。

## 11. 主体区域：聊天室式可折叠任务流

主页面应借鉴成熟文字聊天室 UI 的优点：

- 时间顺序清晰。
- 角色清楚。
- 大文本阅读舒适。
- 易回看。
- 不依赖复杂图形。

主体不应只是 source visibility 表格。下一轮主页面应以 AI 任务输出流为中心：

- 用户输入。
- 模型公开输出。
- Plan mode 四部分。
- 工具调用摘要。
- 权限请求和结果。
- 来源检查。
- 运行资源摘要。
- 验证结果。

折叠策略：

- 未完成任务：关键推理摘要、工具调用、审批和事件默认展开或半展开。
- 当前进程结束：工具调用、事件、来源检查和资源采样默认折叠。
- 用户可展开查看详情。

重要边界：

- 不显示 provider-private 原始思维链。
- 可显示公开推理摘要、计划依据、工具调用、观察结果和验证状态。
- 折叠不能隐藏阻塞错误、未审批动作或 verifier failure。

## 12. Help 弹窗规格

Help overlay 可以复用 `PropertiesSheet` 的 tab 形式，也可以实现专门 `HelpOverlay`。

最低要求：

- modal overlay，不切换新窗口。
- `Esc` 关闭。
- `Tab`/`Shift+Tab` 可移动焦点。
- `←`/`→` 可切换 tab。
- 内容中文默认。
- 命令和快捷键可复制或至少可清晰阅读。
- 不遮挡后仍保留背景上下文感。

Help tabs 初版内容：

- 快捷键：焦点、菜单、弹窗、运行取消。
- 命令：slash commands、命令 URI、常用操作。
- 模型：provider/model/thinking intensity 说明。
- 审批：Plan approval、manual/auto action approval。
- 终端：系统终端、VS Code terminal、CPU/MEM title 刷新。
- 来源：source visibility、retrieve/search、evidence 等级。

## 13. 当前冻结

冻结：

- 默认中文 UI。
- Help 做 modal overlay，而不是新窗口或普通下拉。
- `Esc` 是关闭弹窗/菜单的主路径。
- 顶部保留两层语义，不完全二合一。
- 完全大主窗模式是目标能力。
- `Sources` 保留。
- `View` 不再保留为必要顶层菜单。
- `Agent` 改为 `模型` / `Models`。
- `Edit` 转为 `编辑模式`。
- Address 完整输入弹窗化。
- Find 需要多行/高级弹窗适配。
- 主体区域转向聊天室式可折叠任务流。

延期：

- 默认是否开启大主窗。
- 顶部中文菜单最终词汇。
- HelpOverlay 是复用 PropertiesSheet 还是新 widget。
- 各侧栏隐藏状态的持久化配置。
- 鼠标点击区域与键盘焦点状态的统一模型。
