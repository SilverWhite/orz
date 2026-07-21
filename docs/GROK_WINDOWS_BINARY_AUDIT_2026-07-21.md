# Grok Windows binary 便携安装审计（2026-07-21）

状态：binary 已下载到 git-ignored 项目目录并完成离线检查；没有登录或模型调用。

## 安装边界

- 没有执行官方 `irm ... | iex` 安装器。
- 没有修改用户 PATH、`~/.grok`、PowerShell profile 或系统目录。
- binary 只存在于 `D:\CLI\.tools\grok\0.2.106\grok.exe`，不进入 Git。
- 配置解析检查使用隔离的 `GROK_HOME`，没有读取或写入用户级 Grok 配置。
- 没有读取 API key、创建 session、调用模型、执行工具或触发 update。

## 观测结果

| 项目 | 观测值 |
|---|---|
| stable channel | `0.2.106` |
| artifact | `https://x.ai/cli/grok-0.2.106-windows-x86_64.exe` |
| Content-Length | `130120520` bytes（124.1 MiB） |
| HTTP ETag / MD5 | `e91fa7c4ec65fbbe862526806cbb7ea7` |
| local SHA-256 | `a6a25d55daadca0c2458a5aceb4c1873eb7c76964ef307647d079e344c53969a` |
| PE header | `4D5A` |
| Authenticode | `Valid` |
| signer CN | `X.AI LLC` |
| `grok --version` | `grok 0.2.106 (bde89716f6)` |

MD5 这里只用于和服务器发布元数据逐字节对照；安全锁使用 SHA-256 与 Authenticode。可重复的本地检查入口是
[`../scripts/inspect_grok_install.ps1`](../scripts/inspect_grok_install.ps1)。

## 能力面检查

离线 `--help` 与子命令帮助确认该 binary 暴露：

- `--output-format streaming-json`、`--json-schema`、固定 `--session-id`；
- `--debug-file`、`trace --local`、`export`、`inspect --json`；
- `--sandbox`、`--allow`、`--deny`、`--tools`、`--max-turns`；
- `--no-memory`、`--no-subagents`、`--disable-web-search`；
- headless/TUI/stdio agent、sessions、plugins、MCP 与 worktree。

隔离 `GROK_HOME` 下的 `inspect --json` 成功读取 DeepSeek custom-model TOML。第一次调用也确认全局
`--cwd` 必须位于 `inspect` 子命令之前；随后正确调用退出码为 0。

## 未证明事项

- binary build ID `bde89716f6` 与开源仓库 commit / `SOURCE_REV` 的对应关系没有官方映射，保持
  `unverified`，不能因为签名有效就推断源码等价。
- `inspect` 成功只证明配置可解析，不证明 DeepSeek 最终 URL、thinking request shape、
  `reasoning_content` 连续性或 private-reasoning 脱敏。
- 尚未启动 fake provider conformance，更没有真实 DeepSeek development probe。

以上未证明项已写入 [`../upstream/grok-build.lock.json`](../upstream/grok-build.lock.json) 与下一阶段计划。

## 官方来源

- [Grok Build released-binary instructions](https://github.com/xai-org/grok-build#installing-the-released-binary)
- [Official Windows installer](https://x.ai/cli/install.ps1)
