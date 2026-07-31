# VS Code 适配、显式 Shell 窗口与终端标题更新 v0.1

**状态**: 设计冻结（待实施）
**日期**: 2026-08-01
**范围**: VS Code 薄桥（ACP-based）、`terminal_visibility` 显式 shell 窗口策略、terminal tab 标题 OSC 更新
**参考产品**: Claude Code（VS Code 扩展 200万+ 安装、terminal 标题更新）、Codex CLI（官方 VS Code 扩展、sandbox 终端窗口）、Grok Build（ACP 协议、`run_terminal_command`）、acp-agent-hub（多平台 ACP client）

---

## 1. VS Code 适配（ACP 薄桥）

### 1.1 设计原则

- **不做完整 VS Code 扩展**。不重复 Grok 的对话渲染、tool dispatch、session 管理。
- **只做 assurance 状态面板 + 薄命令桥**。
- 通信面使用 **Grok ACP**（`grok agent stdio`），与本仓库现有 `grok_runtime_adapter.py` 的 `GrokAcpSession` 直接复用。
- VS Code 侧的 child process 管理使用 `node-pty`（VS Code 内置），遵循 ACP JSON-RPC 规范。

### 1.2 架构

```
VS Code 扩展（TypeScript，≤5 个源文件）
  ├─ extension.ts           激活入口，注册 sidebar webview + 命令
  ├─ acpBridge.ts           管理 Grok child process（node-pty + ACP JSON-RPC）
  ├─ assurancePanel.ts      WebView 内容生成（HTML — assurance 状态卡片）
  └─ commands.ts            命令注册（gsa verify / gsa review global / etc.）

          │  ACP JSON-RPC over stdio
          ▼
  Grok agent stdio（成熟 runtime）
          │
          │  ACP events → normalized events
          ▼
  assurance/grok_runtime_adapter.py（已有，复用）
          │
          ▼
  assurance/tui/bridge.py → projector.py → UI state
```

### 1.3 VS Code 侧功能清单

| 功能 | 优先级 | 说明 |
|------|--------|------|
| Sidebar webview — assurance 状态卡片 | P0 | Gate decisions、source visibility、tool availability、session verification 状态，只读 |
| 命令 `gsa.verify` | P1 | 触发 assurance 验证，结果显示在 webview |
| 命令 `gsa.reviewGlobal` | P1 | 触发全局审查模式 |
| 命令 `gsa.runGrok` | P1 | 从 VS Code 启动 Grok ACP session（复用 `gsa run --runtime grok`） |
| 状态栏项 — 当前 assurance gate 摘要 | P2 | VS Code 底部状态栏显示 "GSA: 3/4 sources ✓" |
| Diff 视图 — gate decision 详情 | P2 | 点击状态栏展开详情 |

### 1.4 明确不做

- ❌ 不在 VS Code 扩展中渲染对话流（Grok TUI 负责）
- ❌ 不实现自有 model loop、tool dispatch、session manager
- ❌ 不做独立于 ACP 的通信协议
- ❌ 不绑定特定 VS Code 版本（使用标准 WebviewViewProvider API）

---

## 2. 显式 Shell 窗口（`terminal_visibility` 策略）

### 2.1 动机

用户偏好终端命令在**独立可见窗口**中执行，而非后台静默运行。这适用于：
- 长时间运行的命令（build、test suite、deploy）
- 需要用户交互的命令（SSH、数据库 CLI）
- 调试场景（需要看到完整输出流）

参考产品：Codex CLI 的 sandbox 模式在用户可见终端中执行命令；VS Code 的 `TerminalManager` 管理 PTY 进程并提供可视化。

### 2.2 策略定义

在 Grok adapter 层新增 `TerminalVisibility` 枚举：

```python
class TerminalVisibility(str, Enum):
    INLINE = "inline"      # Grok 默认 — 后台执行，输出写入 artifact
    POPOUT = "popout"      # 拉起独立 Windows Terminal 窗口
    VSCODE = "vscode"      # 复用 VS Code 内置终端（仅 VS Code 扩展模式下）
```

默认值：`POPOUT`。用户可通过 CLI 参数或配置覆盖。

### 2.3 Windows 实现路径

| 策略 | Windows 实现 |
|------|-------------|
| `INLINE` | 当前行为。`subprocess.Popen` → Job Object 包裹 → stdout/stderr 捕获 |
| `POPOUT` | `Start-Process wt.exe -ArgumentList "powershell -NoExit -Command <cmd>"` 或 `conhost.exe` |
| `VSCODE` | 通过 ACP extension 调用 `vscode.window.createTerminal()` |

### 2.4 Grok adapter 改动

在 `grok_runtime_adapter.py` 的 `GrokRunRequest` 中新增字段：

```python
terminal_visibility: TerminalVisibility = TerminalVisibility.POPOUT
```

在 `run_terminal_command` 的 tool handler 中，根据 `terminal_visibility` 选择执行路径：
- `POPOUT`：拦截 Grok 的默认 shell 执行，替换为 `Start-Process` 拉起窗口
- `INLINE`：透传 Grok 默认行为

### 2.5 CLI 接口

```
gsa run --runtime grok --terminal-visibility popout|inline|vscode
```

---

## 3. 终端标题更新（OSC 转义序列）

### 3.1 动机

Claude Code 在 agent 等待用户输入时更新 terminal tab 标题（如 `Claude Code (2)`），让 alt-tab 切换窗口时能看到 agent 状态。低实现成本，高体验收益。

### 3.2 实现

使用 OSC（Operating System Command）转义序列：

```python
# OSC 0 ; <title> BEL  — 同时设置 window title 和 icon name
# OSC 2 ; <title> BEL  — 仅设置 window title

def set_terminal_title(title: str) -> None:
    """Set the terminal window/tab title."""
    sys.stdout.write(f"\x1b]0;{title}\x07")
    sys.stdout.flush()
```

### 3.3 触发时机

| 状态 | 标题格式 | 触发点 |
|------|---------|--------|
| 空闲等待输入 | `GSA — 就绪` | `StatusBar: 空闲` |
| Grok 运行中 | `GSA — 运行中 (turn 3)` | `RunStarted` / `text_delta` event |
| 等待用户审批 | `GSA — 等待审批` | `PermissionRequested` event |
| Gate 阻断 | `GSA — ⚠ 来源不足` | `GateDecision(decision=block)` |
| 运行完成 | `GSA — 完成 ✓` | `RunFinished` event |
| 运行失败 | `GSA — 失败 ✗` | `RunFailed` event |

### 3.4 集成位置

在 `assurance/tui/projector.py` 的事件处理器中，每个 terminal 事件处理后调用 `set_terminal_title(...)`。或更优：由 `TuiPrototype` 的 status update 方法统一管理，避免散落。

### 3.5 约束

- 仅在 `prompt_toolkit` 全屏模式下生效（`full_screen=True` 时终端支持 OSC）
- 不依赖终端类型检测（几乎所有现代终端都支持 OSC 0/2）
- 退出 TUI 时恢复原始标题

---

## 4. 实施顺序

| 阶段 | 内容 | 预估 |
|------|------|------|
| P0 | 终端标题更新 | ~30行，`projector.py` + `app.py` |
| P1 | `terminal_visibility` 策略（`POPOUT` 路径） | ~80行，`grok_runtime_adapter.py` + CLI 参数 |
| P2 | VS Code 扩展骨架（sidebar webview + ACP bridge） | ~200行 TypeScript + ~100行 Python adapter |
| P3 | VS Code 扩展完善（命令、状态栏、diff 视图） | ~200行 |

---

## 5. 反规则

- ❌ VS Code 扩展不实现自有对话渲染
- ❌ `terminal_visibility` 不改变 Grok 的 tool 语义，只改变执行窗口
- ❌ 终端标题不显示敏感信息（不显示 prompt 内容、API key、文件路径）
- ❌ 不依赖 VS Code 专有 API，使用标准 WebviewViewProvider
