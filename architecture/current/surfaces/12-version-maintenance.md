# S12 版本与维护（surface-version）

> 状态：`current-design`（2026-10-21 330 批建立）。设计权威 [ADR-0010](../../../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)。
> 代码：`orz/crates/orz-bin/src/main.rs`（`--build-info`、维护子命令）、`orz/crates/orz-host/src/carrier_integrity.rs`。
> 审计：[330 批档](../../../docs/audits/330_SURFACE_CONTRACT_BASELINE_2026-10-21.md)；归属项：0dl。

## 1. 状态机

```text
启动 → verify_at_startup（carrier-manifest 比对）
  → 缺席：开发树形态，静默跳过
  → 失配/缺失：stderr 告警横幅（不拒绝启动）
维护 CLI：orz archive / unarchive / delete-session / trust / rollback restore
  → 需桥注入 ORZ_MAINTENANCE_TICKET（≥16 字符）
```

## 2. 不变量

1. **身份可读**：`orz --build-info` 打印 `version/os/arch/profile`，是唯一版本身份面。
2. **完整性审计腿**：清单失配只告警不拒绝（开发树形态合法）。
3. **维护命令受门**：无令牌 exit 2；长度不足 exit 2。
4. **回滚是文件级**：`rollback restore` 恢复 `.gsa/rollback/*.bak`，不是二进制回滚。

## 3. 失败模式

| 失败 | 业界事实 | 本地面 |
|---|---|---|
| 版本偏斜 | CC 组织策略最低版本硬报错；Codex `/status` 显示 server 版本；opencode 版本不符表现为 Connection Failed | **无运行时校验**（web 桥与 `orz --stdio` 可不一致） |
| 更新 | CC 自动更新＋`claude update`；Codex `codex update`；Gemini 自动更新；opencode `upgrade` | **无更新入口**（手工换装） |
| 回滚二进制 | opencode `upgrade v0.1.48` 可当回滚 | 只有文件级 rollback |
| 版本命令 | 四家都有 `--version`/等价 | **无 `--version`**（只有 `--build-info`） |

## 4. 明确不做

- 自动更新（发行由人工换装，含 ACAF 重 provision）。
- 二进制级回滚。

## 5. 现状与差距

| 项 | 现状 | 证据 | 等级 |
|---|---|---|---|
| `--build-info` | 有 | `main.rs:159` | 接受 |
| `--version` | **333 落码**：`orz --version`/`-V` 稳定单行（home 重定向前退出） | `orz-bin/src/main.rs` | 已落 |
| 版本偏斜 | **333 落码**：`/api/boot` 带 `bridge_version/child_version/version_skew`；偏斜横幅＋系统行 | `server.rs`／`main.js` | 已落（真机待 S4） |
| 更新/回滚 | 无 | — | 可延 |
| TTY 双保险 | 未实现 | 0dl | 应修 |

## 6. 0.9.8 判决

- **必闭**：`--version` 别名（333 已落）；web 启动时校验桥与 child 的版本一致并提示（333 已落）；`orz doctor` 输出载体完整性（333 已落清单复验，非逐文件哈希列表）。
- **可延**：更新入口、二进制回滚。
- **不做**：自动更新。

## 7. 测试映射

| 不变量 | 现有测试 | 缺口 |
|---|---|---|
| 1 身份可读 | `build_info_line`（bin） | 无 `--version` 钉 |
| 2 完整性审计腿 | `missing_manifest_reports_no_manifest`、`wrong_kind_manifest_is_ignored`、`vendored_bytes_match_manifest_sha256` | — |
| 3 维护命令受门 | `maintenance_ticket_requires_present_value_of_at_least_16_chars` | TTY 双保险无测试 |
| 4 回滚文件级 | `rollback` 单测 | 二进制回滚不存在 |
