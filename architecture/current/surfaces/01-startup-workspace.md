# S1 启动与工作区（surface-startup）

> 状态：`current-design`（2026-10-21 330 批建立）。设计权威 [ADR-0010](../../../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)。
> 代码：`orz/crates/orz-web/src/{lib.rs,server.rs}`、`orz/crates/orz-bin/src/main.rs`（`web` 分派）、`orz/crates/orz-web/assets/app/main.js`（boot）。
> 审计：[330 批档](../../../docs/audits/330_SURFACE_CONTRACT_BASELINE_2026-10-21.md)；归属项：0br、0dk。

## 1. 状态机

```text
orz web → parse_args → bind(127.0.0.1:21487) → 生成 per-boot token
  → GET / (静态 UI) → GET /api/boot (token/root/版本) → WS /ws/acp?token=…
workspace switch: POST /api/workspace/switch → 重绑 root → 重新 boot
```

## 2. 不变量

1. **环回硬门**：只允许 `127.0.0.1`/`::1`；不提供 LAN/公网直连（远程走 SSH 隧道）。
2. **令牌**：每个 boot 生成 256-bit 令牌；所有 `/api/*` 与 `/ws/*` 必须携带，缺失/错误 401。
3. **Origin 检查**：WS 校验 Origin，防跨站直连。
4. **工作区单一**：一个桥进程只服务一个 root；切换即重绑，不并行两个 root。
5. **静态面自足**：UI 不依赖外网 CDN/构建链，`assets.rs` 内嵌。

## 3. 失败模式

| 失败 | 业界事实 | 本地面 |
|---|---|---|
| 端口占用 | CC/Codex 由 CLI 报错并给替代端口 | 由 `orz web` 报错；无自动换端口 |
| 浏览器未打开 | 产品通常打印 URL | 打印 URL，需手动打开 |
| 令牌泄漏 | opencode 本地 server 同样用 token | 令牌进 URL/WS query（本机环回，接受） |
| 工作区切换失败 | — | 返回错误，前端保留旧 root |

## 4. 明确不做

- 远程直连、多用户、内置 TLS、自动更新服务。

## 5. 现状与差距

| 项 | 现状 | 证据 | 等级 |
|---|---|---|---|
| 环回/令牌 | 已实现 | `server.rs:846` 路由、`acp.js:60` | 接受 |
| boot 投影 | `/api/boot` 返回 token/root/版本 | `server.rs:867` | 接受 |
| 端口冲突 | 无自动换端口/无友好提示 | `orz-web` parse_args | 可延 |
| 诊断 | 无 `orz doctor`；启动失败只看 stderr | 无实现 | 应修 |
| 工作区切换 | 有路由，无「切换前在跑 run」保护 | `server.rs:869` | 应修 |

## 6. 0.9.8 判决

- **必闭**：切换工作区时若有在跑 run，必须显式拒绝或先取消（不能静默丢）。
- **可延**：端口自动回退。
- **不做**：远程直连。

## 7. 测试映射

| 不变量 | 现有测试 | 缺口 |
|---|---|---|
| 1 环回硬门 | `only_loopback_binds_are_allowed` | — |
| 2 令牌 | `token_is_hex_and_distinct_per_call` | 401 面仅 handler 测 |
| 3 Origin | `ws_origin_must_name_loopback_or_be_absent` | — |
| 4 工作区单一 | `visited_workspace_survives_switch_and_resolves_old_root` | 切换时在跑 run 无测试 |
| 5 静态面自足 | smoke §8 | — |
