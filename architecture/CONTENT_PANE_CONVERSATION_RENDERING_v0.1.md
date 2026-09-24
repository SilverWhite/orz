> **UI 形态权威（2026-09-24 用户裁决解冻）**：本文件经用户令「那三个稿子要从冻结状态里拽回来」**解除 pre-ADR-0010 冻结（2026-09-24 批就地冻结的反向操作）**，与综合稿 [`UI_FORM_CONSOLIDATED_DESIGN_2026-09-24.md`](../docs/UI_FORM_CONSOLIDATED_DESIGN_2026-09-24.md) 共同构成 orz UI 的**形态设计权威**（执行形态＝Web 优先、TUI 形式后补；复用原则＝能照搬就照搬、遵守开源协议）；定位与冲突裁决仍以 [`ADR-0010 §2.4／§2.6／§14.78`](../adr/ADR-0010-fusion-runtime-and-agent-architecture.md) 为准。正文不回改（细节属早期基线、保留原样），仅解除冻结标注。

# ContentPane 对话渲染设计 v0.1

**状态**: 设计冻结
**日期**: 2026-07-31
**范围**: 主窗口内容区（ContentPane）的对话流、用户/模型消息、工具调用的渲染与交互设计
**不涉及**: 外部框架（MenuBar/Toolbar/AddressBar/FindBar/ExplorerPane/ContentMarker/StatusBar）——这些保持 `CLI_UI_INTERACTION_MODEL_v0.1.md` 和 `CLI_UI_SIMPLIFICATION_SUPPLEMENT_v0.1.md` 的现有设计不变

---

## 1. 设计参考

- **Microsoft Comic Chat (1996)**：每条消息是独立的视觉单元，非连续文本流。不同角色用不同视觉样式区分，消息之间有明确的视觉边界。2026-07 以 MIT 协议开源。
- **mIRC (1995–)**：经典三区布局。角色标签（`<nickname>`）区分说话者，颜色编码。事件（joins/parts）与对话内容在视觉上分离。
- **现代终端 chat TUI**：box-drawing 字符构建轻量卡片，`╭╮╰╯` 圆角或单线 `┌┐└┘`。工具调用显示为行内单行条目，可展开。

---

## 2. 核心视觉规则

**一条分界线**：

```
有边框 = 自然语言对话（用户输入 / 模型输出）
无边框 = 进程动作（工具调用行 / 展开清单）
```

这使对话内容与系统动作在视觉上天然隔离——不需要色彩或 emoji 就能一眼区分"人在说话"和"机器在做"。

---

## 3. 三种渲染形态

### 3.1 用户/模型消息（极简边框）

统一的轻量圆角框，不区分用户和模型的边框样式。角色通过标签区分。

```
┌ [用户] ──────────────────────────────────────────────────────────────────┐
│ 帮我看看 gsa.py 这个入口文件                                               │
└───────────────────────────────────────────────────────────────────────────┘

┌ [模型] · turn 1 ─────────────────────────────────────────────────────────┐
│ 好的，让我先看看项目结构...                                                │
│                                                                           │
│ gsa.py 是顶层入口，非常简单：                                              │
│                                                                           │
│   from assurance.cli import main                                          │
│   raise SystemExit(main())                                                │
└───────────────────────────────────────────────────────────────────────────┘
```

规则：
- 边框字符：`┌─┐│└┘`
- 标题行：`┌ [角色] · 附加信息 ───...───┐`，填充 `─` 到右侧
- 内容区：`│ 文本...                                 │`
- 内容不截断，完整展示
- 消息之间空一行
- Markdown 渲染（标题/粗体/列表/代码块/语法高亮）在边框内展开

### 3.2 工具行 · 折叠态（无边框，单行，滚动）

```
  [read_file]   cli.py · 120 行   ▸ [展开]
  [search_content]   "main" · 3 matches   ▸ [展开]
```

规则：
- `[工具名]` 方括号标签，固定不变
- 两空格后接 detail：当前操作的文件名/搜索词/目标 + 结果摘要
- 同一工具多次调用时，工具名不变，detail 滚动更新到最新操作
- 不同工具各自独立一行，按首次出现时间排列
- `▸ [展开]` 右对齐
- 不显示状态文字（运行中看滚动即知，完成则停住）

### 3.3 工具行 · 展开态（无边框，缩进清单，pin-to-top，全展开）

```
  [read_file] ▸ [关闭]
    1  cli.py · 120 行
    2  gsa.py · 8 行
    3  grok.py · 6 行
```

规则：
- 展开区域固定在 ContentPane 视口顶部，不随对话流滚动
- `[工具名] ▸ [关闭]` 作为标题行，点击 `[关闭]` 收起
- 缩进两个空格列出全部操作条目，按时间顺序编号
- 每个条目：序号 + 文件名 + 结果摘要（行数/匹配数等）
- **不展示文件内容**——展开清单是调用留痕/审计轨迹，不是内容浏览器
- 无最大高度限制，全展开
- 超出 ContentPane 可见范围时通过 ContentPane 整体滚动查看

---

## 4. 工具行数据模型

```python
@dataclass
class ToolEntry:
    """工具调用清单中的单条操作记录"""
    target: str       # 操作目标: "cli.py", '"main"'
    detail: str       # 结果摘要: "120 行", "3 matches"
    seq: int          # 时间顺序编号 (1-based)


@dataclass
class ToolTraceLine:
    """单个工具的调用轨迹（折叠态=单行，展开态=清单）"""
    tool_name: str              # 工具名: "read_file", "search_content"
    entries: list[ToolEntry]    # 操作列表，按时间追加
    expanded: bool = False      # 展开/折叠
```

渲染逻辑：
- **折叠态渲染**：取 `entries[-1]`（最新条目）构建 detail 文本
- **展开态渲染**：遍历 `entries` 构建完整清单
- **同工具追加**：`entries.append(ToolEntry(...))`，折叠态自动滚到最新

---

## 5. 交互规则

| 操作 | 行为 |
|------|------|
| 同工具多次调用 | 同一 `ToolTraceLine` 追加 `ToolEntry`，折叠态 detail 滚动 |
| 不同工具首次调用 | 创建新 `ToolTraceLine`，排在已有工具行之后 |
| `▸ [展开]` | 该工具行展开为 pin-to-top 清单，按钮变 `[关闭]` |
| `▸ [关闭]` | 收起为折叠态，按钮变 `[展开]`，pin 释放 |
| 展开后查看 | 展开区域固定在 ContentPane 顶部，对话流在下方正常滚动 |
| 全展开 | 无高度限制，超出视口部分通过 ContentPane 整体滚动 |

---

## 6. 流式显示

模型输出逐 token 追加到当前模型消息卡片底部，而非一次性替换。

Grok ACP 的 `assistant_message` 增量通知 → adapter 新增 `text_delta` 事件 → projector 追加到当前模型消息卡片的 content 末尾 → ContentPane 重渲染。

消息卡片在内容追加过程中实时重绘，用户看到逐字出现的效果。

---

## 7. 折叠策略

来自 `CLI_UI_SIMPLIFICATION_SUPPLEMENT_v0.1.md` §11：

- **运行中**：工具调用默认展开或半展开
- **结束后**：工具调用默认折叠
- **阻塞错误/未审批动作/verifier failure**：不折叠

实现上，`RUN_FINISHED` 事件到达时调用 `ContentPane.collapse_non_warnings()`，将所有非 warning 的工具行设为折叠态。

---

## 8. 反规则

- ❌ 不使用 emoji——宽度不一致，与 retro desktop 设计语言冲突
- ❌ 工具展开不展示文件内容——那是 Properties/ExplorerPane 的职责
- ❌ 展开区域不加边框——只有自然语言对话才有边框
- ❌ 不做展开最大高度限制——全展开
- ❌ 不在折叠态显示"已完成/失败"等状态文字——滚动即运行，停住即完成
- ❌ 用户和模型消息不使用不同边框样式——统一极简框

---

## 9. 与外部框架的关系

ContentPane 是主窗口中间的内容区，其外部框架（MenuBar、Toolbar、AddressBar、FindBar、ExplorerPane、ContentMarker、StatusBar）保持 `CLI_UI_INTERACTION_MODEL_v0.1.md` 和 `CLI_UI_SIMPLIFICATION_SUPPLEMENT_v0.1.md` 的现有设计。ContentPane 对话渲染的改动只影响 `widgets.py` 的 `ContentPane._render_messages()` 和 `projector.py` 的相关 event handler，不触及外部框架的布局和渲染逻辑。

外部框架的渲染问题（中文宽度计算偏差、分隔线对齐等）另行处理，不阻塞 ContentPane 对话渲染。
