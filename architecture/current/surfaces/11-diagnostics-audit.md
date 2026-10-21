# S11 诊断与审计（surface-diagnostics）

> 状态：`current-design`（2026-10-21 330 批建立）。设计权威 [ADR-0010](../../../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)。
> 代码：`orz/crates/orz-web/src/{runs.rs,server.rs}`、`assets/app/journal.js`、`orz/crates/orz-bin/src/main.rs`（`--replay`、`rollback`）。
> 审计：[330 批档](../../../docs/audits/330_SURFACE_CONTRACT_BASELINE_2026-10-21.md)；归属项：0dk、0dm。

## 1. 状态机

```text
run 运行 → .gsa/runs/<RUN-*>/events.jsonl 追加（hash 链）
前端: /api/runs（轻元数据）→ /api/run/{id}/events（切片）| /ws/journal/{id}（活动尾）
离线: orz --replay <journal>（完整性/损坏检测）
```

## 2. 不变量

1. **单一真相**：journal 是事实面；UI 投影不得反向写。
2. **路径受控**：只读校验过的 run id，不接受任意路径。
3. **轻元数据**：列表不整读 journal（512 KiB 事件窗、200 run、50 探针为既定边界）。
4. **尾部安全**：切片不返回半行；断点可续。
5. **完整性可检**：hash 链断/篡改可被 `--replay` 检出（不提供外部锚定）。

## 3. 失败模式

| 失败 | 业界事实 | 本地面 |
|---|---|---|
| 配置/环境坏了 | CC `/doctor` 可自动修；Codex `codex doctor` 出报告；Gemini/opencode 无 doctor | **无 doctor**，只能看 stderr |
| 日志在哪 | CC/opencode 有明确路径与 `--debug` | 无统一日志入口文档 |
| 事件过大 | 无对应 | 512 KiB 窗截断，UI 无「还有更多」提示 |
| run 数过多 | 无对应 | 200 run 上限后标 archived |

## 4. 明确不做

- 外部 digest 锚定/防篡改保证（083 裁决）。
- 远程日志上传。

## 5. 现状与差距

| 项 | 现状 | 证据 | 等级 |
|---|---|---|---|
| journal 投影 | runs/events/tail 三通道 | `server.rs:853` | 接受 |
| 路径校验 | run id 白名单 | `runs.rs:3-4` | 接受 |
| doctor | 无 | — | 阻塞 |
| 版本/配置诊断 | 无 CLI 入口（`orz-config` 是继承件，未接） | `orz-config` | 应修 |
| 截断提示 | 无「还有更多」 | `runs.rs` | 可延 |

## 6. 0.9.8 判决

- **必闭**：`orz doctor`（版本/载体哈希、配置分层、`.gsa` 健康、端口/令牌、retrieval/CDP 可用性、最近 run 终态）。
- **可延**：日志面板、截断提示。
- **不做**：外部锚定。

> 错误分类与四要素文案（`code/class/hint/evidence`）见 [`0dk S1 §9`](../../../docs/SESSION_LIFECYCLE_OWNERSHIP_AND_CLOSE_DESIGN_2026-10-21.md)。

## 7. 测试映射

| 不变量 | 现有测试 | 缺口 |
|---|---|---|
| 1 单一真相 | `list_runs_reads_light_metadata_and_terminal_status` | — |
| 2 路径受控 | `runs.rs` 白名单单测 | — |
| 3 轻元数据 | `runs_beyond_probe_cap_read_as_archived` | 512 KiB 窗无钉 |
| 4 尾部安全 | `slice_events_withholds_partial_tail_and_resumes` | — |
| 5 完整性可检 | `--replay` verifier（assurance） | doctor 无入口 |
