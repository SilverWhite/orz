# Phase 1 技术债务登记 v0.1

状态：2026-08-02。记录 `B:\orz` 在 Phase 1（Scaffold + Build）完成后的遗留问题，
作为 Phase 2 的输入。

## 1. 已完成的工作

| 项目 | 状态 |
|------|------|
| 63 → 45 crates（删 18） | ✅ |
| 21 `xai-grok-*` → `orz-*` 重命名（目录 + Cargo.toml 包名） | ✅ |
| 4 Grok TUI crates 保留原名（兜底 UI） | ✅ |
| Workspace Cargo.toml（members + deps）更新 | ✅ |
| 全仓 Cargo.toml 交叉引用修复 | ✅ 零残留 |
| `orz-bin` / `orz-assurance` / `orz-tui` 骨架 | ✅ 编译通过 |
| `orz.exe` 产出 + 运行验证 | ✅ |
| `xai-proto-build` Windows/MinGW protoc 兼容 | ✅ |
| Rust GNU 工具链（B: 盘）+ MinGW + protoc | ✅ |

## 2. 源码引用断裂（.rs 文件）

Cargo.toml 层面已全部修复。断裂全部集中在 `.rs` 源码文件中——
原 `xai_grok_*` crate 名被批量替换为 `orz_*`，但 14 个已删除的 crate 不应被改名，
其引用需要移除或替换。

### 2.1 断裂统计

| 原 crate（已删除） | 被错误改名后的引用 | 断裂文件数 | 影响范围 |
|---|---|---|---|
| `xai-grok-telemetry` | `orz_telemetry` | 138 | orz-shell(大量), orz-http, orz-mcp, orz-memory, orz-config-types |
| `xai-grok-test-support` | `orz_test_support` | 49 | orz-shell tests, orz-tools |
| `xai-grok-sampler` | `orz_sampler` | 41 | orz-shell session 层, orz-http, orz-agent |
| `xai-grok-announcements` | `orz_announcements` | 21 | orz-shell, orz-config-types, xai-grok-pager |
| `xai-grok-auth` | `orz_auth` | 17 | orz-http, orz-shell, orz-memory, xai-file-utils |
| `xai-grok-voice` | `orz_voice` | 15 | xai-grok-pager (TUI 语音功能) |
| `xai-grok-update` | `orz_update` | 8 | orz-shell, orz-version, xai-grok-pager |
| `xai-grok-plugin-marketplace` | `orz_plugin_marketplace` | 8 | orz-shell, xai-grok-pager |
| `xai-grok-subagent-resolution` | `orz_subagent_resolution` | 7 | orz-shell subagent 层 |
| `xai-grok-mermaid` | `orz_mermaid` | 3 | xai-grok-pager (Mermaid 图表渲染) |
| `xai-grok-shell-base` | `orz_shell_base` | 2 | orz-shell lib.rs, util/mod.rs |
| `xai-grok-shell-session-support` | `orz_shell_session_support` | 1 | orz-shell session/managed_mcp.rs |
| `xai-grok-secrets` | `orz_secrets` | 0 | 无 .rs 引用（仅 Cargo.toml 已清理） |
| `xai-mixpanel` | — | 0 | 无残留 |
| `xai-sqlite-journal` | — | 0 | 无残留（was renamed orz_sqlite_journal, then removed from .rs） |
| `xai-codebase-graph` | `orz_codebase_graph` | 0 | 无 .rs 引用（仅 Cargo.toml 已清理） |
| `xai-chat-state` | `orz_chat_state` | 0 | 无 .rs 引用（仅 Cargo.toml 已清理） |

**合计：310 个文件存在断裂引用。**（一个文件可能引用多个已删除 crate，去重后约 248 个文件。）

### 2.2 按 crate 分组的影响

#### orz-shell（最严重，~200+ 文件）

`orz-shell` 是 Grok 的 agent runtime loop，几乎所有已删除 crate 都被它直接或间接依赖：

| 功能路径 | 依赖的已删除 crate | 处理策略 |
|----------|-------------------|---------|
| telemetry 打点（agent ops, session metrics, otel gate, relay, subagent） | `orz_telemetry` | 删除打点代码 |
| auth/credential（login, credential_provider, privacy, remote client） | `orz_auth` | 删除云端认证路径，本地 only |
| announcement/config | `orz_announcements` | 删除公告功能 |
| plugin marketplace | `orz_plugin_marketplace` | 删除插件市场 |
| subagent resolution | `orz_subagent_resolution` | 评估后决定 |
| sampler（session layer: goal, memory_dream, model_switch, prompt_build, recap, sampler_turn, spawn, tool_calls, types） | `orz_sampler` | 采样逻辑合并回 orz-shell 自身 |
| shell-base re-export（lib.rs, util/mod.rs） | `orz_shell_base` | 代码已合并，删除 re-export |
| shell-session-support（managed_mcp.rs） | `orz_shell_session_support` | 代码已合并，删除引用 |
| test support | `orz_test_support` | 测试代码暂时 skip |
| update channel（leader, config/version） | `orz_update` | 删除更新检查 |

#### xai-grok-pager / pager-bin / pager-render / pager-minimal（兜底 TUI）

| 功能路径 | 依赖的已删除 crate | 处理策略 |
|----------|-------------------|---------|
| voice（app_view, dispatch/router, dispatch/settings, dispatch/voice, event_loop, diagnostics, settings/defs, settings/registry, voice/*） | `orz_voice` | 删除语音 UI |
| update（app/effects, app/event_loop, app/mod, views/welcome） | `orz_update` | 删除更新提示 |
| announcements（acp/mod, acp_handler/settings, acp_handler/tests, agent_view/links, agent_view/mod, agent_view/session, app_view, dispatch/router, dispatch/tests, effects/mod, event_loop, views/announcements, views/welcome） | `orz_announcements` | 删除公告 UI |
| plugin marketplace（agent_view/cta, dispatch/cta, dispatch/tests, plugin_cmd） | `orz_plugin_marketplace` | 删除插件入口 |
| mermaid（app/mermaid_worker, scrollback/blocks, tests） | `orz_mermaid` | 删除 Mermaid 渲染 |
| test support | `orz_test_support` | 测试 skip |

#### 其余受影响的 crate

| Crate | 依赖的已删除 crate | 处理策略 |
|-------|-------------------|---------|
| `orz-http` | `orz_auth`, `orz_telemetry`, `orz_sampler` | 移除 auth middleware，移除 telemetry 集成 |
| `orz-mcp` | `orz_telemetry` | 移除 telemetry 打点 |
| `orz-memory` | `orz_auth`, `orz_telemetry` | 移除 auth，移除 telemetry |
| `orz-config-types` | `orz_announcements`, `orz_telemetry` | 移除对应类型定义 |
| `orz-agent` | `orz_sampler` | 移除 sampler 引用 |
| `orz-workspace` | `orz_auth`（upload/mod.rs） | 移除云端上传 |
| `orz-version` | `orz_update` | 移除版本检查 |
| `orz-tools` | `orz_test_support` | 测试 skip |
| `xai-file-utils` | `orz_auth`（gcs.rs, lib.rs, queue.rs, storage_client.rs） | 移除云端存储 |
| `xai-tool-runtime` | `orz_tools_api`（context.rs） | 已修复（bulk rename） |

## 3. 构建状态

### 3.1 可编译

```
cargo build -p orz-bin -p orz-assurance -p orz-tui
```
→ ✅ 成功。产出 `orz.exe`。

### 3.2 不可编译（全仓）

```
cargo check --workspace
```
→ ❌ 失败。310 个 .rs 文件中的断裂引用阻止了以下 crate 的编译：
- `orz-shell`（最核心，~200+ 断裂文件）
- `xai-grok-pager` / `xai-grok-pager-bin` / `xai-grok-pager-minimal` / `xai-grok-pager-render`
- `orz-http`, `orz-mcp`, `orz-memory`, `orz-agent`, `orz-workspace`, `orz-version`, `orz-tools`, `orz-config-types`
- `xai-file-utils`, `xai-tool-runtime`

### 3.3 Proto 编译

`xai-proto-build` 的 `emit_rerun_if_changed` 在 Windows 上被 `#[cfg(not(windows))]` 跳过。
这是临时 workaround。长期应修复 `--dependency_out=/dev/stdout` 在 Windows/MinGW 下的兼容性。

## 4. 新增 crate 状态

| Crate | 状态 | Phase 2 工作 |
|-------|------|-------------|
| `orz-bin` | ✅ 骨架编译 | 链接 orz-shell（Phase 2）、orz-tui（Phase 4） |
| `orz-assurance` | ✅ 骨架编译 | 从 Python `assurance/` 移植 gates, journal, orientation, session, sandbox, credential, permit（Phase 2-4） |
| `orz-tui` | ✅ 骨架编译 | 从 Python `assurance/tui/` 移植 TUI（Phase 4） |

## 5. 工具链

| 组件 | 位置 | 备注 |
|------|------|------|
| Rust GNU 1.97.1（pinned） | `B:\.rustup\toolchains\1.97.1-x86_64-pc-windows-gnu` | `rust-toolchain.toml`: `channel = "1.97.1-x86_64-pc-windows-gnu"` |
| Rust MSVC 1.97.1 | `B:\.rustup\toolchains\1.97.1-x86_64-pc-windows-msvc` | 已安装 llvm-tools（rust-lld），待 SDK 就绪后切换 |
| Rust stable GNU | `B:\.rustup\toolchains\stable-x86_64-pc-windows-gnu` | 备用（旧 default） |
| MinGW (WinLibs POSIX UCRT) | `%LOCALAPPDATA%\Microsoft\WinGet\Packages\BrechtSanders.WinLibs.POSIX.UCRT_*\mingw64` | GNU 链接器（ld.exe, dlltool.exe） |
| protoc 35.1 | `%LOCALAPPDATA%\Microsoft\WinGet\Packages\Google.Protobuf_*\bin` | Windows 原生二进制。COPY 到 `B:\orz\bin\protoc.exe` |

**已知工具链问题**：
- **MSVC 未就绪**：Windows SDK 安装介质在 `B:\SDK\` 但未完成安装（缺少 .lib 文件）。
  SDK 安装完成后，将 `rust-toolchain.toml` 改为 `1.97.1`（MSVC host），移除 MinGW PATH 依赖
- **GNU + protoc**：`xai-proto-build` 的 `emit_rerun_if_changed` 在 Windows 上跳过（`#[cfg(not(windows))]`），
  因其硬编码了 `/dev/stdout`。Phase 2 可修复为 tempfile 方案
- **rustup 版本+target 组合支持**：`1.97.1-x86_64-pc-windows-gnu` 在 toolchain.toml 中可正常解析（已验证）。
  此前失败是因为未预先 `rustup toolchain install` 该 toolchain

## 5.1 Phase 1 收尾修正记录（2026-08-02）

| 修正项 | 来源 | 状态 |
|--------|------|------|
| `rust-toolchain.toml` 版本固定 | 审查 §3.2-A | ✅ `1.97.1-x86_64-pc-windows-gnu` |
| `.gitignore` 添加辅助脚本 + protoc 二进制 | 审查 §3.2-C | ✅ |
| 删除 tracked helper 文件 | 同上 | ✅ |
| `orz-tui/Cargo.toml` 移除空 `[dependencies]` | 审查 §3.2-D | ✅ |
| `orz-bin/main.rs` 使用 `CARGO_PKG_VERSION` | 审查 §4.1 | ✅ |
| `orz-assurance/lib.rs` GateDecision 含 payload | 审查 §4.2 | ✅ |
| bin/protoc 从 git 中移除 | 审查 §3.2-C | ✅ 已 `git rm --cached` |

## 5.2 设计决策补充（2026-08-02，用户确认）

与审查报告 §2.2 中提出的设计问题对应：

| 设计问题 | 决策 |
|----------|------|
| Computer Hub crates 去留 | 删除。但需保留 **MCP 安装接口**（从 `xai-grok-mcp`/`orz-mcp` 中保留功能性部分） |
| `xai-fast-worktree` 的 sqlite-journal 依赖 | 需额外设计。JSONL journal 替代 SQLite journal 的方案待定 |
| `xai-file-utils` 的 auth 依赖 | 将云端 OAuth 替换为**本地身份验证**：验证本机 Microsoft 账号或本地环境特征码 |
| `orz-sampler` 替代方案 | 需额外设计。sampler 涉及 model selection / temperature / top_p / attribution 等 41 个文件，不能简单删除 |
| `orz-version` vs `orz-update` | 保留 `orz-version`，仅删除 `orz-update`（更新检查）。版本号管理和更新通知分离 |
| 工具链版本固定 | ✅ 已执行（1.97.1 GNU），MSVC 待 SDK 就绪后切换

## 6. Phase 2 优先级排序

按阻断程度排列：

### P0：阻断 orz-shell 编译（必须最先处理）

| 债务项 | 文件数 | 策略 |
|--------|--------|------|
| `orz_telemetry` 引用清理 | 138 | 删除所有 telemetry 打点代码。telemetry 数据不外发，本地日志用 `tracing` 替代 |
| `orz_test_support` 引用清理 | 49 | 测试代码用 `#[cfg(test)]` + mock 替代，非测试代码删除引用 |
| `orz_sampler` 引用清理 | 41 | 采样逻辑（model selection, temperature, top_p 等）内联到 orz-shell session 层 |

### P1：阻断 orz-shell 其余模块编译

| 债务项 | 文件数 | 策略 |
|--------|--------|------|
| `orz_auth` 引用清理 | 17 | 删除云端 OAuth/X.AI 登录，替换为**本地身份验证**（本机 Microsoft 账号或环境特征码）。保留 credential_provider 骨架，对接 Windows Credential Manager |
| `orz_announcements` 引用清理 | 21 | 删除公告系统 |
| `orz_plugin_marketplace` 引用清理 | 8 | 删除插件市场。**保留 MCP 安装接口**（从 orz-mcp 中提取功能性部分） |
| `orz_subagent_resolution` 引用清理 | 7 | 评估 subagent 功能是否需要。若不需，删除 |
| `orz_update` 引用清理 | 8 | 仅删除更新检查。**保留 `orz-version`**（版本号管理） |
| `orz_shell_base` re-export 修复 | 2 | lib.rs 和 util/mod.rs 中删除对已合并模块的 pub use |
| `orz_shell_session_support` 引用修复 | 1 | managed_mcp.rs 中修复 |

### P2：兜底 TUI（xai-grok-pager）编译

| 债务项 | 文件数 | 策略 |
|--------|--------|------|
| `orz_voice` 引用清理 | 15 | 删除语音 UI 路径 |
| `orz_announcements` 引用清理 | ~10 | 删除公告 UI 路径 |
| `orz_update` 引用清理 | ~5 | 删除更新提示 |
| `orz_plugin_marketplace` 引用清理 | ~5 | 删除插件入口 |
| `orz_mermaid` 引用清理 | 3 | 删除 Mermaid 渲染 |

### P3：其余受影响 crate

| 债务项 | 策略 |
|--------|------|
| `orz-http` | 移除 auth middleware、telemetry 集成 |
| `orz-mcp` | 移除 telemetry 打点。**保留 MCP 功能性接口**（工具扩展协议） |
| `orz-memory` | 移除 auth、telemetry。保留记忆系统代码（默认禁用） |
| `orz-config-types` | 移除 announcements、telemetry 类型 |
| `orz-agent` | 移除 sampler 引用 |
| `orz-workspace` | 移除云端上传 |
| `orz-version` | 移除更新检查引用，**保留版本号管理** |
| `orz-tools` | 测试 skip |
| `xai-file-utils` | 将云端存储 auth 替换为本地身份验证 |
| `xai-tool-runtime` | 修复 orz_tools_api 引用（bulk rename 可能未覆盖） |

### P4：需额外设计（Phase 2 启动前先出方案）

| 设计项 | 涉及范围 | 说明 |
|--------|---------|------|
| **orz-sampler 替代方案** | 41 文件，orz-shell session 层 | 采样逻辑（model selection、temperature/top_p、attribution、goal、memory_dream、prompt_build、recap、tool_calls）需整体设计。不能简单删除——是 agent 行为核心 |
| **xai-fast-worktree 的 journal 依赖** | orz-workspace, orz-shell | 原依赖 `xai-sqlite-journal`。需设计 JSONL journal 如何满足 worktree 的快速索引需求（SQLite → JSONL 替代方案） |
| **xai-file-utils 本地认证** | `xai-file-utils`, `orz-workspace` | 将云端 OAuth 替换为本机 Microsoft 账号或环境特征码验证。需调研 Windows Authentication API |
| **Computer Hub crates 去留 + MCP 接口保留** | `xai-computer-hub-*` (3 crates), `orz-mcp` | 删除云端 Computer Hub 功能，但从 `orz-mcp` 中保留 MCP 协议安装/通信接口作为本地工具扩展机制 |

## 7. 不可变约束（贯彻 Phase 2-4）

以下约束来自 `FORK_ARCHITECTURE_AND_DESIGN_LANGUAGE_v0.3.md`，Phase 2 开始逐步实现：

1. 每个 action 恰好一个 terminal journal event
2. Journal hash chain 连续不可断裂
3. `run_finished` 必须是最后一个 event
4. 凭据不写入 journal（只允许 SHA-256）
5. Hidden reasoning 不进入公开输出
6. Orientation checkpoint 不生成 counterexample_candidate
7. 用户授权可满足 permission，不能证明 provenance/coverage/independence
8. SecurityFinding 不自动产生 PermissionDecision
9. Snapshot restore 必须写入 journal event
10. Journal event IDs 不重复、turn_id 不孤儿、时间戳不非线性

## 8. 构建命令速查

```powershell
# 环境
$env:Path = "B:\.cargo\bin;<mingw64>\bin;<protoc>\bin;" + $env:Path
$env:CARGO_HOME = "B:\.cargo"
$env:RUSTUP_HOME = "B:\.rustup"

# Phase 1 可用的构建
cd B:\orz
cargo build -p orz-bin                               # 仅 orz.exe
cargo build -p orz-bin -p orz-assurance -p orz-tui    # 三个新 crate

# Phase 2 目标
cargo check --workspace                                # 全仓编译通过
```

## 9. 参考

- [`FORK_ARCHITECTURE_AND_DESIGN_LANGUAGE_v0.3.md`](FORK_ARCHITECTURE_AND_DESIGN_LANGUAGE_v0.3.md) — 最终架构设计
- [`FORK_IMPLEMENTATION_DESIGN_v0.1.md`](FORK_IMPLEMENTATION_DESIGN_v0.1.md) — 注入点精确设计
- [`IMPLEMENTATION_DEVIATION_AND_CORRECTION_v0.1.md`](IMPLEMENTATION_DEVIATION_AND_CORRECTION_v0.1.md) — 原始偏差分析
- `B:\orz` @ `4066dd3` — Phase 1 产物（独立 git repo）
