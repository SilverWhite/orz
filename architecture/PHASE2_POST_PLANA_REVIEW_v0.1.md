# Phase 2 Plan A 执行后审查 v0.1

状态：2026-08-03。方案 A 执行完毕后（orz-shell 981→0 错误），对设计、实现和架构的全面审查。

## 1. 审查背景

方案 A 将 6 个 crate 从手写 stub 替换为 git `500129c` 的完整原始代码，解除了 Phase 2 编译阻塞。
但恢复过程中暴露了更深层的结构问题：**Grok 代码的模块间耦合远超设计预期**，直接修改 Grok 源码
存在显著的兼容性风险。

## 2. 核心发现

### 2.1 Crate 删除矩阵与现实的偏差（严重）

`FORK_IMPLEMENTATION_DESIGN_v0.1.md` §1.1 设计删除 18 个 crate，声称最终 ~35 crate。
实际状态：

| 类别 | 数量 | 详情 |
|------|------|------|
| 设计声称删除 | 18 | — |
| 实际已删除 | 8 | `update`, `voice`, `sqlite-journal`, `codebase-graph`, `pager-pty-harness`, `test-support`, `shell-base`, `mermaid` |
| 被迫恢复完整代码 | 9 | `telemetry`, `plugin-marketplace`, `subagent-resolution`, `announcements`, `secrets`, `sampler`, `chat-state`, `auth`, `shell-session-support` |
| 新建 stub | 1 | `xai-mixpanel` |
| **实际 workspace members** | **74** | 远非设计的 ~35 |

**根因**：设计基于"功能不需要"逻辑删除 crate，但未充分评估代码引用耦合度。
orz-shell 深度依赖这 10 个 crate 的类型定义、trait 实现和函数签名，删除它们导致 981 个编译错误。

### 2.2 Telemetry 实时初始化——设计意图违规（严重）

代理审查发现 `orz-shell/src/agent/init.rs:215` 调用 `orz_telemetry::client::init(...)`，
且此初始化在**共享 agent 初始化路径**中。这意味着：

- `orz` 二进制 ✅ 初始化真实遥测
- `grok` 兜底二进制 ✅ 同样初始化真实遥测

设计意图明确：telemetry → 删除，不外发。但当前仅靠配置开关
（`GROK_TELEMETRY_ENABLED`——**仍使用旧 `GROK_` 前缀**，与设计 §2.3 的 `ORZ_` 前缀矛盾）
做运行时门控，**没有机械强制禁用**。

对比 `xai-mixpanel` 的正确做法（硬 no-op stub），telemetry 需要同等处理。

### 2.3 实现设计伪代码的 4 个具体 Bug（高）

| # | 位置 | Bug | 影响 |
|---|------|-----|------|
| 1 | §4.6 SnapshotStore | `track()`/`patch()`/`restore()`/`revert()` 声明为 sync 方法但内部使用 `.await` — **无法编译** | 编译错误 |
| 2 | §4.5 JournalRecorder | `record()` 用 `try_send` 写入 bounded 256 channel — 满时**静默丢弃事件**，破坏 hash chain 连续性 | 数据完整性 |
| 3 | §4.5 JournalInvariants | `orphan_events` 维度永远无法填充（`RunEvent` 无 `turn_id` 字段）— 死代码 | 功能缺失 |
| 4 | §4.5 verifier | 仅检查链上 `previous_event_sha256` 匹配，**不重新计算**每个 event 的 `event_sha256` — payload 篡改无法检测 | 安全缺口 |

### 2.4 设计文档间矛盾

| 矛盾 | v0.2/v0.3 设计语言 | 实现设计 | 实际代码 |
|------|-------------------|---------|---------|
| Grok TUI 去留 | 删除，被 orz-tui 替换 | 保留为兜底（双二进制） | 保留但未验证编译 |
| Snapshot 所属 Phase | v0.3: Phase 2 | Phase 4 | 未实现 |
| IP1 锚定 | orz-models / sampler | 同左 | §1.1 删除 sampler，§7 锚定 sampler — 自相矛盾 |
| Crate 命名 | 设计文档保留 `xai-grok-*` 原名 | 实现全部改为 `orz-*` | `orz-*` |
| GateDecision 命名 | v0.1: `Allow/Warn/Defer/Block` | v0.2+: `Pass/Warn/Block/Defer/NotApplicable` | — |

### 2.5 注入点设计质量评估

| 注入点 | 锚定精度 | 状态 |
|--------|---------|------|
| IP1 (thinking:disabled) | **模糊** — 路径自相矛盾 | 需重新定位 |
| IP2a-c (prompt pipeline) | **精确** — `orz-shell/src/agent/mvp_agent/mod.rs` 已验证存在 | ✅ |
| IP3a-c (tool dispatch) | **精确** — `orz-tools/src/bridge.rs:198` 已验证存在 | ✅ |
| IP4a-c (session lifecycle) | **精确** — `acp_agent.rs` 已验证，但 session-close 边界定义不足 | ⚠️ |
| IP5 (snapshot) | **模糊** — 无具体文件/函数 | 需完整设计 |
| IP6 (hard-gate) | **模糊** — "编译时保证"无具体机制 | 需完整设计 |

### 2.6 设计文档版本混乱

| 文件 | 文件名版本 | 内容标题版本 |
|------|-----------|-------------|
| `FORK_IMPLEMENTATION_DESIGN_v0.1.md` | v0.1 | **v0.2** |
| `PHASE2_COMPILATION_STATUS` | 已修复为 v0.2 | 已修复 |
| `IMPLEMENTATION_DEVIATION_AND_CORRECTION_v0.1.md` | v0.1 | **v0.2** |

## 3. 架构风险评估

### 3.1 直接修改 Grok 代码的兼容性风险

通过 981 错误事件，识别出以下风险维度：

1. **耦合深度风险**：Grok 的 agent runtime（`orz-shell`）与 telemetry、auth、sampler、chat-state
   等模块形成深层耦合。修改任何一个都会引发连锁编译错误。

2. **幽灵引用风险**：批量重命名工具对已删除 crate 的引用也执行了重命名（如 `xai_grok_telemetry` →
   `orz_telemetry`），产生 310+ 个文件的"幽灵引用"——引用存在但目标 crate 不存在。

3. **API 签名精度风险**：Rust 类型系统要求每个方法签名、泛型约束、trait 实现与调用点完全匹配。
   手写 stub 策略失败的根本原因——~300+ 函数签名需要与原始完全一致。

4. **上游漂移风险**：Grok Build 上游持续更新。锁定 `500129c` 缓解了此风险，但随着时间推移，
   安全补丁和 bug 修复需要手动 backport。

### 3.2 各模块耦合度与价值评估

| 模块 | 代码量 | 耦合度 | 对 orz 的价值 | 风险等级 |
|------|--------|--------|-------------|---------|
| `orz-shell` | 最大 | 极高（依赖 12+ crate） | 核心（agent loop） | 🔴 极高 |
| `orz-telemetry` | ~13,600 行 | 极高（被 shell 深度引用） | 低（设计意图禁用） | 🔴 极高 |
| `orz-sampler` | ~10,000 行 | 高（shell session 层依赖） | 中（采样逻辑） | 🟡 高 |
| `orz-chat-state` | ~14,000 行 | 中 | 中（状态管理） | 🟡 中 |
| `orz-tools` | 大 | 中（相对独立） | 高（工具执行） | 🟢 低 |
| `orz-workspace` | 大 | 中（相对独立） | 高（文件系统/VCS） | 🟢 低 |
| `orz-sandbox` | 中 | 低（独立） | 高（安全沙箱） | 🟢 低 |
| `orz-mcp` | 中 | 中 | 高（协议支持） | 🟢 低 |
| `orz-config` | 中 | 中 | 高（配置管理） | 🟢 低 |
| `orz-http` | 小 | 低（独立） | 高（HTTP 客户端） | 🟢 低 |
| `orz-markdown` | 中 | 低（独立） | 中（终端渲染） | 🟢 低 |
| `orz-memory` | 中 | 中 | 低（默认关闭） | 🟡 中 |
| `orz-auth` | ~400 行 | 中 | 低（本地 only） | 🟡 中 |
| `orz-plugin-marketplace` | ~5,600 行 | 中 | 低（不需要） | 🟡 高 |
| `orz-subagent-resolution` | ~3,000 行 | 中 | 中（子代理功能） | 🟡 中 |
| `orz-announcements` | ~400 行 | 低 | 低（不需要） | 🟢 低 |
| `orz-secrets` | ~500 行 | 低（独立） | 高（凭据脱敏） | 🟢 低 |
| `xai-mixpanel` | ~46 行(stub) | 低（已 stub） | 无 | 🟢 无 |
| Grok TUI (pager*) | 大 | 中 | 兜底 UI | 🟡 中 |

## 4. 推荐方向

### 4.1 底线：必须立即修复

1. **Telemetry 硬禁用**：在 `orz-shell/src/agent/init.rs` 加编译时 feature gate 或启动时硬门控，
   确保 `orz_telemetry::client::init()` 永远不会在 orz 构建中发送数据。
   参考 `xai-mixpanel` 的 no-op stub 模式。

2. **JournalRecorder 设计修正**：`try_send` → 阻塞 `send` 或带背压的 bounded channel，
   防止事件静默丢弃导致 hash chain 断裂。

3. **伪代码 4 bug 修复**：更新 `FORK_IMPLEMENTATION_DESIGN` 中的代码草图。

### 4.2 建议：模块级重新评估

基于本次审查的耦合度分析，建议对 Grok 源码进行**第二轮拆解**：

- **高价值 + 低耦合** → 保留并适配（tools, workspace, sandbox, mcp, config, http, markdown, secrets）
- **高价值 + 高耦合** → 审慎重构，逐步解耦（shell, sampler, chat-state）
- **低价值 + 高耦合** → 替换为自主实现或成熟开源方案（telemetry, plugin-marketplace）
- **低价值 + 低耦合** → 直接删除或 stub（announcements, mixpanel）
- **兜底 UI** → 保留但裁剪（Grok TUI: 移除 voice/update/mermaid/announcements 引用）

## 5. 参考

- [`PHASE2_COMPILATION_STATUS_v0.2.md`](PHASE2_COMPILATION_STATUS_v0.2.md) — 编译修复完整记录
- [`IMPLEMENTATION_DEVIATION_AND_CORRECTION_v0.1.md`](IMPLEMENTATION_DEVIATION_AND_CORRECTION_v0.1.md) — 偏差分析
- [`FORK_IMPLEMENTATION_DESIGN_v0.1.md`](FORK_IMPLEMENTATION_DESIGN_v0.1.md) — 实现设计（feat/fork-architecture 分支）
- [`FORK_ARCHITECTURE_AND_DESIGN_LANGUAGE_v0.3.md`](FORK_ARCHITECTURE_AND_DESIGN_LANGUAGE_v0.3.md) — 最新设计语言（feat/fork-architecture 分支）
- `B:\orz` commit `a2a7868` — 方案 A 提交点
- 代理审查完整记录：`a9e7a4cf5d62cc286`（设计一致性）、`a0baff606859b9397`（crate 结构）、`ae51e0f2437807c37`（文档问题）
