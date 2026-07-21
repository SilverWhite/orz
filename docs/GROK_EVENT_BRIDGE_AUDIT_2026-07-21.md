# Grok local trace/export 与 event-completeness bridge 审计（2026-07-21）

状态：Windows fake-only post-run evidence spike；v4 mechanically valid、trace/export `available`、总体
completeness `partial`。没有真实 provider、credential、模型费用或科学结论。v1-v3 的 session-not-found
结论已被证伪，保留为 harness regression 记录。

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

锁定源码还区分了三条发现路径：

- [`storage/search.rs`](https://github.com/xai-org/grok-build/blob/a881e6703f46b01d8c7d4a5437683546df30449d/crates/codegen/xai-grok-shell/src/session/storage/search.rs)
  将 SQLite 定义为首次 search 时 bootstrap、保存后 debounce upsert 的派生全文索引；
- [`storage/jsonl/mod.rs`](https://github.com/xai-org/grok-build/blob/a881e6703f46b01d8c7d4a5437683546df30449d/crates/codegen/xai-grok-shell/src/session/storage/jsonl/mod.rs)
  的 list 路径直接扫描并反序列化 `summary.json`；
- [`export_cmd.rs`](https://github.com/xai-org/grok-build/blob/a881e6703f46b01d8c7d4a5437683546df30449d/crates/codegen/xai-grok-pager/src/export_cmd.rs)
  与 [`trace_cmd.rs`](https://github.com/xai-org/grok-build/blob/a881e6703f46b01d8c7d4a5437683546df30449d/crates/codegen/xai-grok-pager/src/trace_cmd.rs)
  按 session ID 扫描目录并重放 `updates.jsonl`，不查询 FTS row。

Windows 支持也应准确表述：官方发布 Windows 预编译 binary；但锁定 commit 的
[README](https://github.com/xai-org/grok-build/tree/a881e6703f46b01d8c7d4a5437683546df30449d)
把 macOS/Linux 列为 supported build hosts，并称从该源码树构建 Windows 为 best-effort、currently untested。
这表示 Windows 的源码构建与边缘集成置信度较低，不等于预编译 CLI 整体“完成度很低”。

## 2. Observation-first 失败链与纠正

1. v1 的 `trace --local` 探针在 20 秒边界 timeout；v2-v3 的 trace/export 立即返回 session not found。
2. 同期本地 session 目录真实存在：`events.jsonl` 17 行、`updates.jsonl` 6 行、`chat_history.jsonl`
   8 行，且 `summary.json.info.id/cwd/grok_home` 都与 source run 一致；SQLite 的 `session_docs` 与 FTS
   document table 为 `0` 行。
3. 锁定源码复核证明这些现象不能合并解释：search SQLite 是派生索引；`sessions list` 直接读取
   `summary.json`；export/trace 按 ID 直接扫描 session 目录，均不要求 search row。
4. 根因在本项目 post-run harness：它通过反射设置 `ProcessStartInfo` 私有 `environment` 字段。在当前
   Windows PowerShell 5.1/.NET Framework 中，`Process.Start()` 实际消费公开的
   `EnvironmentVariables`/`StringDictionary`；子进程因此继承宿主环境，没有看到隔离 `GROK_HOME`。
5. 显式环境的最小复现立即成功导出 297-byte Markdown。修复为清空并填充公开
   `EnvironmentVariables` 后，管理员 v4 在 all-network firewall block 下成功生成 trace 与 export，且规则残留为 0。

因此，v1-v3 不能证明 Grok headless session 不可发现，也不是 Windows session-store 缺陷。它们直接证明的是
harness 的环境隔离声明曾为假；相关历史 artifact 不删除、不覆盖，但不得继续用作 upstream 缺陷证据。空
search index 仍是独立观察：表示派生全文检索缓存尚无 document row，不表示 session 文件丢失。

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

source run：`FAKE-18c9c39d65174b9b9a5b37124a7d5b94`；纠正后的最终 bridge：
`BRIDGE-256300684193457f90d39b0dd57b2c0b`；本地目录
`.observed-runs/postrun-evidence-tool-v4/`。v1-v3 保留为 harness regression 历史，不覆盖。

- post-run result valid，全部 13 个 checks 为 true；临时 firewall rule 残留 `0`；
- trace state=`available`、exit `0`，artifact 5702 bytes；
- export state=`available`、exit `0`，artifact 297 bytes；
- bridge manifest/event/verification/result 四个专用 schema 均通过；34 个 event 的 sequence/hash-chain 由独立
  verifier 逐条重算通过，verification 全部 10 个 checks 为 true；
- source counts：workspace scan `3`、stdout `3`、session events `17`、session updates `6`、provider `2`、
  trace/export status 各 `1`、supervisor `1`；
- stdout tool lifecycle=`absent`；session events tool lifecycle=`observed`；session updates tool identity=`observed`；
  provider tool continuity=`observed`；
- missing observations：cross-source runtime order、sealed encryption、explicit stdout tool lifecycle；
- bridge JSONL 中 fake reasoning/tool/output/credential marker 命中 `0`。

artifact SHA-256：

| artifact | SHA-256 |
|---|---|
| post-run `result.json` | `a7e0996e01f1eacd2eeb694a716e813cc62cdf15d42c97b56396556639e7150b` |
| bridge `manifest.json` | `da6dd6ab5e478743e284f20a901d23a1244c95519a031e4296837d799e8e33c7` |
| bridge `events.bridge.jsonl` | `a445af3a4ced029db451fce9005d16e24ff9b3e2bc57853df7fb35cebd66b893` |
| independent verification | `fb7c304ec3a6a56c0feb8896017a884ecd6c7362119924c3b49d0c9acae25738` |
| post-run trust receipt | `608b022a72117f39bd5d299cbf82969408d36a68689a50c6450da6996d95a4ec` |
| trace archive | `a6ca4ad6f46d2d11a1746da80d6be402516cfd54b7a89a91f94e82e8baa74322` |
| export Markdown | `4ff350b20b3de6dca6e96f205933a97e6296a7eb6f841ab2bfc2dc8d856a11b8` |

## 5. 结论边界与下一步

本轮直接证明：锁定 build 的 local session-store metadata 能补足 stdout 缺失的 tool lifecycle 与 tool-call ID；
sidecar 可以在不复制 raw content 的情况下生成可重放 hash-chain bridge；在正确的隔离 `GROK_HOME` 下，fake
headless session 可被 list/export/trace 直接发现。空 search index 与这些直接查找路径解耦。

没有证明：跨来源全局时序、raw source 已加密、任意写工具、真实 DeepSeek 或科学正确性。下一
implementation spike 转入 disposable fixture workspace 的 checkpoint/delta。search index 不通过手工写 SQLite
“修复”；如需依赖全文搜索，应调用官方 search bootstrap 并单独记录结果。
