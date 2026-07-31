# P4 metadata audit, compaction, and recovery authorization — 2026-07-25（2026-07-31 修订："尚未关闭"同步）

## 结论

P4 已完成 runtime-neutral development/conformance 纵向切片：ACP、session、provider、supervisor
等来源可以投影到签名 metadata-only ledger；P1 archive 删除 raw payload 后，独立 verifier 仍能重建
顺序、hash chain、来源 completeness、terminal exactly-once 和 compaction provenance。

恢复部分刻意止步于“候选”和“授权”：verified archive 中的 pinned snapshot 仍视为不可信数据；
精确 one-shot permit 可以产生 allow receipt，但不会复制文件、覆盖 workspace 或执行恢复。

这不表示真实 runtime 已完整接入，也不表示已经存在可用的 shadow Git 恢复系统。

## 合同与实现

### Metadata audit

- `audit-event-v0.1.schema.json`
  - typed source：ACP、session、provider、supervisor、tool、runtime、kernel；
  - source-local sequence 与 ledger-global sequence；
  - payload 只保留 SHA-256、bytes 和 archive retention category，raw bytes 不嵌入事件；
  - facts 只允许窄 metadata 类型，并拒绝 content/input/output/prompt/reasoning/secret/token 等字段名；
  - hash chain 绑定每个事件，terminal outcome 与 final position 分离。
- `audit-seal-receipt-v0.1.schema.json`
  - 明示四个核心来源的 complete/partial/unknown；零事件来源不得提升为 complete；
  - 重建 source/type counts、journal digest/head、terminal exactly-once；
  - seal 绑定创建时 security envelope digest并使用 RFC 8785 + HMAC-SHA256。

raw source payload 写入 P1 `delete_on_archive` 类别；metadata journal 写入
`redacted_conversation`，因此归档后只保留 digest、有限元数据和签名 receipt。

### Compaction

compaction event 必须：

- 使用 `derived_unverified` summary status；
- 绑定 summary digest 与 source-span digest；
- 明示 retained、discarded、unknown ranges；
- 拒绝倒置或跨 disposition 重叠的 range。

当前 fixture 模拟 manual compaction。Automatic threshold 仍未 observed，不能从本轮结果推断其行为。

### Recovery authorization

- `recovery-candidate-receipt-v0.1.schema.json`
  - 只接受 verified-complete archive、有效 audit seal 与 user-pinned snapshot；
  - snapshot 分类固定为 `untrusted_recovery_candidate`；
  - authority effect 为 none，`action_authorized=false`，`restoration_performed=false`。
- `recovery-authorization-receipt-v0.1.schema.json`
  - one-shot permit 精确绑定 candidate/action、target workspace、snapshot 和 attempt；
  - 缺失、错绑、过期或重放均 deny；
  - 即使 allow，`restoration_performed` 仍固定为 false。

因此授权与执行是两个不同状态，当前代码没有恢复执行器。

## 测试结果

2026-07-25 本机回归：

| 套件 | 结果 |
|---|---:|
| prototype | 50 passed |
| Grok reference adapter | 44 passed |
| runtime | 6 passed |
| assurance（含 P4 3 项） | 29 passed |
| 合计 | 129 passed |

仓库检查验证 73 个 schema，`error_count=0`；Python compileall 与 `git diff --check` 通过。

P4 只增加 3 个组合测试：

1. 四来源 ledger + compaction + exactly-once terminal；归档删除 raw 后仍验证通过；
2. 缺失 terminal 拒绝 seal，journal 篡改 fail closed；
3. recovery candidate 默认无权限，缺失 permit 拒绝，精确 permit 允许但不执行，重放与候选篡改拒绝。

## 与 Grok CLI 的关系

旧 Grok event bridge/compaction probe 提供参考证据，但 P4 实现位于 `assurance/`，不依赖 Grok
binary、Grok schema 或 Grok session 路径。Grok 继续是 `reference_only` adapter。

## 尚未关闭（2026-07-31 更新）

以下条目为 2026-07-25 原始审计的”尚未关闭”列表。2026-07-31 审查确认部分已关闭：

已关闭：
- ~~没有 automatic compaction threshold observation~~ → GAK-CMP-001 已于 2026-07-29 关闭（`compaction_observer.py`，36 tests）
- ~~没有 shadow Git/对象存储~~ → GAK-REC-001 已于 2026-07-29 关闭（`shadow_recovery.py`，Git-based store，24 tests）；D1.11+D3.23 已于 2026-07-31 同步关闭
- ~~没有跨进程 writer lock、崩溃中间态 seal recovery~~ → `ArchiveController`（`archive.py`）已实现 advisory lock + journal replay crash recovery；`archive_journal.py` 已实现 `detect_stale_archive_lock()` / `cleanup_stale_archive_lock()` / `recover_archive_journal()`

仍开放：
- 没有实际 runtime adapter 写入统一 ledger；source completeness 是签名声明，不是”未遗漏”的证明
- 没有 production shadow Git recovery executor（当前 recovery authorization 固定 `restoration_performed=false`）
- HMAC 证明本地完整性，不是跨主机远程证明
- archive verification 不覆盖 provider retention、备份、pagefile、hibernation 或物理介质

合理的下一阶段不是直接让普通用户测试真实项目，而是先完成一个 P4.5 合成 adapter 纵向切片：
把 P2.5 的真实无模型 guarded execution、P3 action gate 和 P4 audit ledger 串成同一个
conversation，并在归档后独立复核。之后才能评估 P5 disposable-repo 用户测试。
