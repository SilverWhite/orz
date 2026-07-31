# 会话持久化与布局压缩 v0.1

**状态**: 设计冻结（2026-08-01 修正：session 持久化所有权归 Grok）
**日期**: 2026-07-31
**范围**: 布局压缩（Address/Find 折叠至 Toolbar）、会话列表 UI（只读视图，数据源为 Grok session 目录）
**所有权裁决**: Grok 拥有 session 持久化和恢复；TUI 只提供只读会话列表视图（扫描 Grok 产出目录），不维护独立 session store。
**依赖**: `CLI_UI_INTERACTION_MODEL_v0.1.md`、`CLI_UI_SIMPLIFICATION_SUPPLEMENT_v0.1.md`、`CONTENT_PANE_CONVERSATION_RENDERING_v0.1.md`

---

## 1. Address 与 Find 折叠至 Toolbar

### 1.1 当前布局

```
MenuBar      (行 1)
Toolbar      (行 2)
AddressBar   (行 3)
FindBar      (行 4)
ContentPane  (行 5+)
StatusBar    (末行)
```

### 1.2 目标布局

```
MenuBar      (行 1)
Toolbar      (行 2)  ← Address 和 Find 入口折叠至此
ContentPane  (行 3+) ← 多获得两行
StatusBar    (末行)
```

### 1.3 Toolbar 改造

当前：

```
后退  前进  刷新  停止  打开  验证  属性
```

改造后：

```
后退  前进  刷新  停止  打开  验证  属性  命令...  查找...
```

规则：
- `命令...` — 激活打开 AddressDialog 弹窗（当前已实现，Ctrl+L）
- `查找...` — 激活打开 FindDialog 弹窗（当前已实现，Ctrl+F）
- 两个按钮文本右对齐，与左侧操作按钮形成视觉分组
- AddressDialog 和 FindDialog 的键盘快捷键（Ctrl+L / Ctrl+F）保持不变
- 只有当某个侧栏或对话框处于活跃状态时，对应的 Toolbar 按钮才高亮

### 1.4 弹窗行为

AddressDialog 和 FindDialog 是已实现的 modal overlay。折叠到 Toolbar 后行为不变：
- `Esc` 关闭弹窗
- `Enter` 确认输入
- 弹窗居中显示，半透明背景保留主窗口上下文感

---

## 2. 会话持久化（Grok-owned）

**所有权裁决 (2026-08-01)**: session 持久化与恢复是 Grok 的通用 agent runtime 能力。本仓库**不维护独立 session store**。TUI 的会话列表是只读视图——扫描 Grok 的 run root 目录展示已有会话。恢复操作使用 `grok session resume`。

### 2.1 参考产品

Claude Code 的 `/resume`：显示最近会话列表（标题 + 摘要），用户选择后加载历史上下文，恢复对话。

### 2.2 数据存储（Grok 产出，TUI 只读）

目录结构：

```
.gsa/runs/                   ← Grok ACP session 产出目录（由 bridge 管理）
  <session_id>/
    events.jsonl             ← 已有（Grok ACP 产出 — normalized events）
    acp_transcript.jsonl     ← 已有（Grok ACP 产出 — raw transcript）
    session-verification.json ← 已有（post-run verifier）
    session.json             ← 薄标记（bridge 写入，供 TUI 列表发现）
```

`session.json`（TUI 发现用薄标记，非独立 store）：

```json
{
  "session_id": "S-20260731-a1b2c3d4",
  "created_at": "2026-07-31T14:32:05Z",
  "first_prompt": "帮我看看 gsa.py 的入口文件",
  "turn_count": 5
}
```

### 2.3 Session ID 格式

```
S-YYYYMMDD-<8-char-hex>
```

示例：`S-20260731-a1b2c3d4`

### 2.4 会话列表交互（TUI 只读视图）

进入会话列表：
- **双击 Esc**（两次 Esc 在 500ms 内）→ ExplorerPane 从"来源树模式"切换为"会话列表模式"
- 再次双击 Esc 或选中会话后自动切回"来源树模式"

会话列表在 ExplorerPane 中的展示（复用树形结构）：

```
▾ 2026-07-31
  ▸ 帮我看看 gsa.py 的入口文件 (5 turns)
  ▸ 审查一下 assurance 层的 Gate... (12 turns)
▾ 2026-07-30
  ▸ 修复 Job Object 竞态窗口 (3 turns)
```

规则：
- 按日期分组，日期倒序
- 每个条目显示：`first_prompt` 截断至 28 字 + turn 数 + session ID 后 8 位
- 选中 `Enter` → 显示 session ID（完整恢复用 `grok session resume`）

### 2.5 恢复流程（Grok 原生）

会话恢复使用 Grok 原生命令：

```
grok session resume <session_id>
```

本仓库不实现自有恢复逻辑。

---

## 3. 完整布局（设计冻结）

```
┌─ GSA ───────────────────────────────────────────────────────────────────────────────┐
│ 文件  事件  标记  编辑  模型  来源  运行  验证  帮助                                   │
├──────────────────────────────────────────────────────────────────────────────────────┤
│ 后退  前进  刷新  停止  打开  验证  属性                        命令...  查找...      │
├────────────────────┬────────────────────────────────────────────────┬─────────────────┤
│ ▾ INDEX            │                                                │  Markers        │
│   ▾ MAP_MEM.md     │  ┌ [模型] · turn 1 ─────────────────────────┐ │  ───────        │
│   ▾ LIF_CURRENT_IN │  │ 让我先看看项目结构...                     │ │  (none)         │
│ ▾ MAP              │  └───────────────────────────────────────────┘ │                 │
│ ▾ R Series         │                                                │                 │
│ ▸ Artifacts        │    [read_file]   cli.py · 120 行   ▸ [展开]   │                 │
│ ▸ Adapters         │    [search_content]   "main" · 3 matches  ▸    │                 │
│ ▸ Verifier         │                                                │                 │
│ ──────────         │  ┌ [模型] · turn 2 ─────────────────────────┐ │                 │
│ Events             │  │ gsa.py 是顶层入口，只做了一件事...        │ │                 │
│   ▾ Run (5)        │  └───────────────────────────────────────────┘ │                 │
│   ▾ Decisions (1)  │                                                │                 │
│   ▸ Errors (0)     │                                                │                 │
├────────────────────┴────────────────────────────────────────────────┴─────────────────┤
│ 守护 | 网络关闭 | 沙箱严格 | 来源 3/4 | DeepSeek | 空闲                                  │
└──────────────────────────────────────────────────────────────────────────────────────┘
```

关键变化：
- AddressBar 和 FindBar 两行**消失**，节省 2 行
- Toolbar 右侧新增 `命令...`（激活 AddressDialog）和 `查找...`（激活 FindDialog）
- 快捷键 Ctrl+L / Ctrl+F 不变
- ContentPane 获得额外 2 行可用高度
- 双击 Esc → ExplorerPane 切换为会话列表模式（覆盖左侧来源树区域）

---

## 4. 反规则

- ❌ 会话恢复时不复制旧 Grok 进程——始终创建新 ACP session
- ❌ 会话恢复时不复用旧 permit/trust receipt——每次都重新验证
- ❌ 归档不删除文件，只标记 status
- ❌ Address/Find 折叠后不改变其原有键盘快捷键
