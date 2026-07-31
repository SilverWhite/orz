# 会话持久化与布局压缩 v0.1

**状态**: 设计冻结
**日期**: 2026-07-31
**范围**: session index 格式、会话恢复流程、Address/Find 折叠至 Toolbar
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

## 2. 会话持久化

### 2.1 参考产品

Claude Code 的 `/resume`：显示最近会话列表（标题 + 摘要），用户选择后加载历史上下文，恢复对话。

### 2.2 Session Index 存储格式

目录结构：

```
.gsa/
  sessions/
    index.jsonl          ← 全局会话索引（追加写，用于快速列表）
    <session_id>/
      metadata.json      ← 会话元数据
      events.jsonl       ← 已有（Grok ACP 产出）
      acp_transcript.jsonl ← 已有
      session-verification.json ← 已有
```

`index.jsonl` 格式（一行一条，追加写入，与 journal 格式一致）：

```json
{"session_id": "S-20260731-a1b2c3d4", "created_at": "2026-07-31T14:32:05Z", "last_active_at": "2026-07-31T14:45:12Z", "first_prompt": "帮我看看 gsa.py 的入口文件", "prompt_preview": "帮我看看 gsa.py 的入口文件", "turn_count": 5, "run_root": "/path/to/run-root", "status": "active"}
```

`metadata.json` 格式：

```json
{
  "session_id": "S-20260731-a1b2c3d4",
  "created_at": "2026-07-31T14:32:05Z",
  "last_active_at": "2026-07-31T14:45:12Z",
  "first_prompt": "帮我看看 gsa.py 的入口文件",
  "turn_count": 5,
  "run_root": "/path/to/run-root",
  "workspace_path": "/path/to/workspace",
  "model_id": "lif-fake-deepseek",
  "status": "active",
  "artifacts": {
    "events_path": "events.jsonl",
    "acp_transcript_path": "acp_transcript.jsonl",
    "session_verification_path": "session-verification.json"
  }
}
```

### 2.3 Session ID 格式

```
S-YYYYMMDD-<8-char-hex>
```

示例：`S-20260731-a1b2c3d4`

### 2.4 恢复交互流程

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
  ▸ 跑 Grok session verifier... (1 turn)
```

规则：
- 按日期分组，日期倒序
- 每个条目显示：`first_prompt` 截断至 40 字 + turn 数
- 选中条目 → `Enter` → 恢复会话

恢复流程：
1. 读取 `metadata.json`，获取 `run_root` 路径
2. 验证 `events.jsonl` 和 `acp_transcript.jsonl` 完整性
3. 创建新的 Grok ACP session
4. 将历史 `acp_transcript.jsonl` 中的 `session/prompt` 和 assistant 消息作为上下文注入新 session
5. TUI ContentPane 渲染历史对话流（从 `events.jsonl` 重放）
6. ExplorerPane 切回来源树模式
7. 用户继续对话

### 2.5 CLI 命令

```
gsa session list              ← 列出所有会话（日期、首条 prompt、turn 数）
gsa session resume <id>       ← 恢复指定会话
gsa session archive <id>      ← 归档（标记 archived，不删除文件）
gsa session delete <id>       ← 删除（删除 run_root 目录，从 index 移除）
```

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
