# Grok local trace/export 与 event-completeness bridge 审计（2026-07-21）

状态：Windows fake-only post-run evidence spike；mechanically valid、completeness `partial`。没有真实 provider、
credential、模型费用或科学结论。

## 1. 官方边界与本地命令面

锁定源码 commit `a881e6703f46b01d8c7d4a5437683546df30449d` 的
[session 文档](https://github.com/xai-org/grok-build/blob/a881e6703f46b01d8c7d4a5437683546df30449d/crates/codegen/xai-grok-pager/docs/user-guide/17-sessions.md)
说明：headless session 也会持久化；`GROK_HOME` 可覆盖 `~/.grok`；`summary.json` 是索引条目；
`updates.jsonl` 是恢复会话的权威对话日志。锁定 binary `0.2.106 (bde89716f6)` 的本地 help 进一步确认：

- `grok trace --local --json --output <path> <session-id>` 明确跳过 remote upload；
- `grok export <session-id> <path>` 导出 Markdown；
- 两个命令都暴露 `--debug-file`，本项目继续禁用该参数。

命令面 probe 先生成 restricted workspace receipt，再用 clean environment、隔离 profile/temp 和 fail-closed
proxy 运行 help；没有模型或 tool session。

## 2. Observation-first 失败链

1. 第一次 `trace --local` 探针在 20 秒边界 timeout；没有 trace artifact，进程被终止，export 未开始。
2. 分离运行 export，即使指定相同 cwd、HOME、USERPROFILE 和显式 `GROK_HOME`，仍立即返回 session not found。
3. `sessions list` 返回 `No sessions found`，同时 remote search timeout。只读 SQLite 检查显示
   `session_search.sqlite` 的 `session_docs` 与 FTS document table 均为 `0` 行。
4. 与此相对，本地 session 目录真实存在：`events.jsonl` 17 行、`updates.jsonl` 6 行、
   `chat_history.jsonl` 8 行，且 `summary.json.info.id/cwd/grok_home` 都与 source run 一致。
5. 最终管理员 post-run run 使用 all-network firewall block；trace/export 都在约 0.2 秒内明确返回 not found，
   没有网络等待，也没有伪造 archive/transcript。

这说明当前 fake headless session 的落盘状态与 CLI search/export 可发现性之间存在实际缺口。bridge 不修改
SQLite、不复制 session 目录冒充 trace，也不把 command failure 升级为 source run failure。

## 3. Bridge 实现

[`build_grok_event_bridge.py`](../scripts/build_grok_event_bridge.py) 只读取并哈希现有 source：

- streaming-json stdout；
- Grok session `events.jsonl`、`updates.jsonl`、summary/signals；
- 脱敏 provider result 与已有 private artifact digest；
- preflight/launch/postrun 三张 workspace receipt；
- trace/export command status 与 source run result。

bridge 不复制 prompt、reasoning、tool input/output 或 chat body。每条 JSONL 原文只产生整行 SHA-256；输出仅含
白名单 metadata。`updates.jsonl` 的 `toolCallId/status/kind` 可直接提取，但 content/rawInput/rawOutput 被省略。
每条 bridge event 记录 source、source sequence、raw digest、redaction/completeness state 和 hash-chain
predecessor。跨来源 runtime 顺序固定写为 `not_established`；bridge sequence 只表示确定性追加顺序。

[`invoke_grok_postrun_evidence_bridge.ps1`](../scripts/invoke_grok_postrun_evidence_bridge.ps1) 负责：

- 第三张 restricted receipt，并与 source run 的 aggregate/scan-policy digest 对比；
- 对已核验 Grok binary 建立临时 all-network outbound block；
- clean environment、无 credential、禁用 debug-file；
- trace/export 各自使用 kill-on-close Job Object 和 timeout；
- 无论命令成功、failed 或 timed out，都先记录 status，再构建 metadata bridge；
- `finally` 清理 firewall rule。

## 4. 最终实际观察

source run：`FAKE-18c9c39d65174b9b9a5b37124a7d5b94`；最终 bridge：
`BRIDGE-4d321211df974f06b8a1a48c921bfb8c`；本地目录
`.observed-runs/postrun-evidence-tool-v3/`。v2 在 v1 基础上增加 builder 内部 hash-chain/raw-field 重算；v3
再加入独立 [`verify_grok_event_bridge.py`](../scripts/verify_grok_event_bridge.py) replay verifier。旧 run 保留为
历史观察，不覆盖。

- post-run result valid，全部 12 个 checks 为 true；临时 firewall rule 残留 `0`；
- trace state=`failed`、exit `1`：session not found under `~/.grok/sessions`；
- export state=`failed`、exit `1`：session not found；
- bridge manifest/event/verification/result 四个专用 schema 均通过；34 个 event 的 sequence/hash-chain 由独立
  verifier 逐条重算通过；
- source counts：workspace scan `3`、stdout `3`、session events `17`、session updates `6`、provider `2`、
  trace/export status 各 `1`、supervisor `1`；
- stdout tool lifecycle=`absent`；session events tool lifecycle=`observed`；session updates tool identity=`observed`；
  provider tool continuity=`observed`；
- missing observations：cross-source runtime order、sealed encryption、explicit stdout tool lifecycle、trace archive、
  export transcript；
- bridge JSONL 中 fake reasoning/tool/output/credential marker 命中 `0`。

artifact SHA-256：

| artifact | SHA-256 |
|---|---|
| post-run `result.json` | `d71542d6f5993e558c08e01cae65dd0947ccb54a3e39d652ee29b77d1cc81a6a` |
| bridge `manifest.json` | `2e5d3cee5abd97f8782ca67be99f3fb39f239b47909c183b13a5e7ee9a861db5` |
| bridge `events.bridge.jsonl` | `d3d6867d6557d2a2a39b9af55f8f8dbe3c9126180cc82b0d2038bcf4e34d35de` |
| independent verification | `0f291a38c7b0dcd295847485ef27a72748ddeb253f8c5327eb35c08b26252663` |
| post-run trust receipt | `26d36dc07dfbb1a20d334e4e3963fc8a7485534b255ba065f1f9c2ccfff589f6` |
| trace stderr | `d8385dda63b967127874b5094a9c02d28972109387e27cd50cefa30e1c0c9bfb` |
| export stderr | `7c10c95a304f8bdb06b18030170f47ebbc2011c34c2e2002a27eec38899c809c` |

## 5. 结论边界与下一步

本轮直接证明：锁定 build 的 local session-store metadata 能补足 stdout 缺失的 tool lifecycle 与 tool-call ID；
sidecar 可以在不复制 raw content 的情况下生成可重放 hash-chain bridge。它同时直接证明当前 fake headless
session 无法由 trace/export 命令发现。

没有证明：trace archive/export transcript 可用、跨来源全局时序、raw source 已加密、任意写工具、真实
DeepSeek 或科学正确性。下一 implementation spike 转入 disposable fixture workspace 的 checkpoint/delta；
trace/export indexing 缺口保留为 upstream/local compatibility issue，不通过篡改索引绕过。
