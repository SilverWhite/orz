# P1 会话身份与归档删除生命周期审计（2026-07-28，2026-07-31 修订：接口/分类/行数同步）

## 裁决

新增 P1 独立审计文档，对会话身份、安全信封、敏感操作许可、归档删除控制器及归档日志四个核心模块进行完整审计。P1 是 GSA 框架中负责会话身份 (conversation identity) 和归档删除生命周期 (archive-deletion lifecycle) 的基础合约层——位于 P0 语义基础之上，为 P2-P5 的沙箱执行、指令授权和审计恢复提供命名空间和生命周期控制。

本轮为**审计补漏**（此前 P1 是唯一没有独立审计文档的 P 级合约，仅由源码与 schema 约束），不新增代码功能。全部模块遵循 offline/development baseline 边界，不调用真实模型、不联网。

## 问题背景

Agent 在运行时产生大量临时数据：模型输出、工具结果、快照、receipt、日志。若无统一的会话命名空间和归档删除控制器：

1. **无归属数据累积**：会话结束后遗留文件无法识别归属，也无法判断哪些应删除、哪些应保留
2. **跨会话信息泄漏**：新会话可能意外读取旧会话的残留文件
3. **敏感操作无审计追踪**：用户批准的敏感操作（如凭据使用、网络访问、文件删除）缺少密码学绑定的许可生命周期
4. **删除不完整**：归档后原始文件可能未被完整清除，残留文件可被重建

P1 的核心设计目标：
- **Conversation Namespace**：以 `conversation_id` 为命名空间的隔离文件系统容器，所有 artifact 按保留类别 (retention category) 分类存储
- **Security Envelope**：HMAC 签名的安全信封，绑定会话允许/拒绝的能力集合、父信封链接和过期时间
- **Sensitive Action Permit**：用户批准的一次性操作许可——issue → HMAC 签名 → consume → fail-closed claim，防止重放
- **Archive Controller**：按 retention policy 执行分类删除（16 类→删除，3 类→保留），生成 terminal deletion receipt
- **Archive Journal**：带文件锁的 JSONL 归档日志，每个 entry 有 hash chain 和独立 verifier

## 组件描述

### 1. Conversation Namespace (`assurance/conversation.py`, 257 行)

核心类 `ConversationNamespace`：
- 一个显式的文件系统命名空间，对应一个活跃会话
- 目录结构：
  ```
  {conversation_id}/
    .assurance-conversation.json    # marker
    conversation-state.json          # state (active → archiving → archived/failed)
    artifacts/
      active_security_envelope/      # kernel-owned
      redacted_conversation/         # PERSIST
      user_pinned_snapshot/          # PERSIST
      terminal_deletion_receipt/     # PERSIST
      {delete_category}/             # 16 categories, deleted on archive
    receipts/
  ```

静态工厂方法 `ConversationNamespace.create`：
- 生成唯一 `conversation_id`（`CONV-{UUID}` 格式）
- 调用 `create_security_envelope` 创建安全信封（绑定 allowed/denied capabilities、parent_envelope_id、resumed_from_conversation_id）
- 原子写入 marker、state、envelope
- 创建 receipts 子目录

关键方法：
- `state()`：加载并 schema-验证会话状态（`active` / `archiving` / `archived` / `failed`）
- `load_active_envelope()`：加载当前活跃安全信封（state 必须为 `active`）
- `write_artifact(category, relative_path, value)`：按 retention category 写入 artifact（仅 active 状态允许）
- `claim_artifact(category, relative_path, value)`：fail-closed 非覆盖写入（用于 permit consumption claim）
- `read_artifact(requested_conversation_id, category, relative_path)`：默认拒绝跨会话读取
- `import_untrusted_snapshot(...)`：从外部会话导入快照，标记 `imported_untrusted`

安全约束：
- 所有路径操作通过 `require_within` 和 `require_no_linked_ancestors` 防止路径遍历和 symlink 攻击
- `read_artifact` 强制检查 `requested_conversation_id == self.conversation_id`，默认拒绝跨会话访问
- `active_security_envelope` 类别为 kernel-owned，禁止外部 artifact 写入
- 仅 `active` 状态允许写入 artifact

### 2. 安全信封 (`assurance/envelope.py`)

函数 `create_security_envelope`：
- 绑定 `conversation_id`、`allowed_capabilities`、`denied_capabilities`
- 记录 `frozen_context`（固化的外部上下文 fingerprint）
- 设置 `expires_at`（包络生命周期上限）
- 可选 `parent_envelope_id`（用于 resume 或 child agent）
- 可选 `resumed_from_conversation_id`（用于会话恢复溯源）
- 由 `InstallationKeyStore` 做 HMAC-SHA256 签名
- `capabilities` 采用 `deny_by_default` 策略——不在 allowed 列表中的能力默认拒绝

函数 `verify_security_envelope`：
- 独立验证 envelope schema、key_id、HMAC 签名、过期时间
- 恢复场景下验证 parent envelope 链

### 3. 敏感操作许可 (`assurance/permit.py`, 311 行)

两层生命周期：

`issue_sensitive_action_permit`：
- 输入：`confirmation_sha256`（用户批准内容的摘要）、`action_sha256`、`target_sha256`、`impact_scope_sha256`、`attempt`（重试计数）
- 生成唯一 `permit_id`（`PERMIT-{UUID}`）
- 创建 `sensitive_action_permit` receipt（状态 `issued`）
- `authority: "local-interactive-user"` —— 明确标识批准来源
- `raw_confirmation_recorded: false` —— 不保留原始用户批准文本
- `raw_action_recorded: false` —— 不保留原始动作文本
- TTL 受 envelope `expires_at` 上限约束
- HMAC 签名后写入 `one_shot_permit/{permit_id}.issued.json`

`consume_sensitive_action_permit`：
- 验证 issued permit 的签名、时效性和 binding 匹配
- 通过 fail-closed claim（`exclusive_create_bytes`）防止重复消费
- 创建 `consumed` 状态 receipt，链接回 issued permit 的 SHA256
- 双重验证：issued receipt 自身验证 + consumed receipt 验证

`verify_sensitive_action_permit`：
- 递归验证：issued permit 独立验证；consumed permit 还会验证其引用的 issued receipt
- 检查项：key_id、HMAC 签名、conversation/envelope 匹配、时效性、binding 一致性、issued_receipt_sha256

### 4. 归档控制器 (`assurance/archive.py`, 622 行)

类 `ArchiveController`（支持 crash recovery 与可插拔 storage backend）：

构造函数参数：
- `key_store`：HMAC 签名密钥
- `storage`：可插拔 `StorageAdapter`（默认 `LocalStorageAdapter`）
- `delete_file`：向后兼容的 `DeleteFile` callback
- `max_retries`：删除重试次数（默认 3，指数退避）
- `retry_base_delay_seconds`：重试基础延迟（默认 0.1s）
- `lock_timeout_seconds`：advisory lock 超时（默认 30s）

方法 `.archive(namespace)`：
- 全新归档：验证 active envelope → 写 lifecycle receipt → 状态转移 `active` → `archiving`
- Crash recovery：若已在 `archiving` 状态 → `replay_archive_journal()` 重放 journal → 跳过已删除文件继续
- 若 journal 已有 terminal event → 直接从磁盘 receipt 重建结果（`_rebuild_result_from_disk()`），不重新删除
- 通过 `exclusive_archive_lock()` 获取 advisory file lock 后执行
- 按 `retention-policy-v0.1.json` 定义的 `delete_on_archive` 16 个类别执行删除：
  - `raw_provider_payload`、`private_reasoning`、`raw_tool_result`、`full_stdout_stderr`
  - `network_body`、`credential_lease`、`confirmation_token`、`one_shot_permit`
  - `unpinned_snapshot`、`temporary_checkpoint`、`temporary_profile`、`sandbox_ephemeral_storage`
  - `session_recall_index`、`active_security_envelope`、`temporary_lifecycle_receipt`、`temporary_import_receipt`
- 逐文件带重试删除（指数退避）；失败记录到 `error_pairs`，不静默
- 通过 `ArchiveJournalWriter` 写 append-only JSONL journal（每个 `file_deleted`/`file_failed`/`category_completed` 事件含 hash chain）
- 保留 3 个类别（`redacted_conversation`、`user_pinned_snapshot`、`terminal_deletion_receipt`）跨越 archive boundary
- `unsafe_entries` 追踪：symlink/reparse point/非文件实体标记为 unsafe，不遍历但记录
- 生成双 receipt：`archive-deletion.json`（HMAC 签名 `archive-deletion-receipt`）+ `lifecycle-terminal.json`（`session-lifecycle-receipt`）
- 最终状态转移至 `archived`（无残留且无错误）或 `failed`

函数 `resume_archived_conversation()`（同模块）：
- 调用 `verify_archive()` 确认 archive 完整性
- 仅在 verified complete archive 上创建新 `ConversationNamespace`
- 将旧 deletion receipt 的 `envelope_id` 作为新 namespace 的 `parent_envelope_id`

独立验证函数 `verify_archive()`（`assurance/archive_verifier.py`）：
- 独立重读 `archive-deletion.json` + `lifecycle-terminal.json`，验证 schema 合规
- 验证 HMAC 签名、conversation/envelope ID 匹配、状态一致性
- **独立重新扫描文件系统**：对每个 delete 类别调用 `os.walk()` 重新计数，与 receipt 中的 `remaining_count` 比对
- 重新计算 cleanup projection 与 receipt 中的值比对
- 验证 `archive_complete=true` 时无 errors 且无残留；`archive_complete=false` 时有 unresolved categories

### 5. 归档日志 (`assurance/archive_journal.py`, 632 行)

类 `ArchiveJournalWriter`（context manager）：
- `.append_event(event_type, payload)`：追加带 hash chain 的 journal entry
- 每个 entry 含 `entry_id`（`AE-{UUID}`）、`entry_sha256`、`previous_entry_sha256`、`sequence`
- 退出 context 时自动 flush
- Journal 旋转策略：单文件最大 1 MiB，达到则创建新段

Context manager `exclusive_archive_lock(journal_path, timeout_seconds)`：
- Windows：`msvcrt.LK_NBLCK`；POSIX：`fcntl.LOCK_EX | LOCK_NB`
- 每秒轮询一次 lock，直到超时

函数 `replay_archive_journal(journal_path)`：
- 在 crash recovery 时重放 journal，返回 `{valid, events, deleted_files, terminal_event, last_sequence, last_event_sha256}`
- 验证 hash chain 完整性
- 使 `ArchiveController` 可跳过已删除文件继续归档

函数 `recover_archive_journal(journal_path)`：
- 修复截断/损坏的 journal tail
- 支持五种恢复路径：clean_interrupted / torn_journal / corrupt_journal / stale_lock / archiving_no_journal

函数 `detect_stale_archive_lock()` / `cleanup_stale_archive_lock()`：
- 检测并清理崩溃后残留的孤儿 advisory lock

函数 `inspect_archive_journal(journal_path)`：
- 人类可读的 journal 内容检查入口

## 与 P 级合约的关系

| 合约 | 关系 |
|---|---|
| P0 | P1 消费 P0 的基础 Schema 定义（`contracts.py`、`envelope.py` 的 HMAC 格式） |
| P2 | 沙箱探针的 disposable workspace marker 由 P1 的 conversation namespace 管理 |
| P2.5 | Guarded execution 的 execution receipt 写入 P1 namespace 的 `temporary_lifecycle_receipt` 类别 |
| P3 | Instruction gate 消费 P1 envelope 的 capability 约束做子 agent capability 继承 |
| P4 | Audit ledger 依赖 P1 的 conversation_id 命名空间做事件归属；archive verifier 消费 terminal deletion receipt |
| P4.5 | Workspace trust 在 P1 的 conversation namespace 内验证 workspace marker |

## Schema 体系

### 核心 Schema
| Schema | 用途 |
|---|---|
| `conversation-state-v0.1.schema.json` | 会话状态：state 四态枚举、envelope_id 绑定、revision 计数 |
| `conversation-marker-v0.1.schema.json` | 命名空间标记：conversation_id、marker_kind |
| `effective-security-envelope-v0.1.schema.json` | 安全信封：capability 列表、expires_at、HMAC 完整性 |
| `sensitive-action-permit-v0.1.schema.json` | 许可 receipt：state (issued/consumed)、binding、HMAC 签名 |
| `retention-policy-v0.1.schema.json` | 保留策略：delete_on_archive 16 类 + persist 3 类 |
| `terminal-deletion-receipt-v0.1.schema.json` | 终端删除 receipt：category_counts、residual_entries、HMAC |
| `conversation-archive-receipt-v0.1.schema.json` | 归档 receipt：state 转移记录、删除统计、verification |
| `archive-journal-entry-v0.1.schema.json` | 归档日志 entry：hash chain、payload 绑定 |

### 测试夹具 Schema
| Schema | 用途 |
|---|---|
| `session-lifecycle-receipt-v0.1.schema.json` | 会话生命周期 receipt（positive fixture） |
| `retention-policy-*.schema.json` | 保留策略的正/反验证 |

## 文件清单

### P1 核心模块
| 文件 | 行数 | 用途 |
|---|---|---|
| `assurance/conversation.py` | 257 | ConversationNamespace：隔离文件系统容器、artifact 读写、跨会话拒绝 |
| `assurance/envelope.py` | 268 | Security Envelope：HMAC 签名、capability 绑定、parent/child 链接、key migration |
| `assurance/permit.py` | 290 | Sensitive Action Permit：issue → sign → consume 完整生命周期 |
| `assurance/archive.py` | 622 | ArchiveController：状态转移、16 类删除、crash recovery（journal replay + retry）、advisory lock |
| `assurance/archive_journal.py` | 632 | ArchiveJournalWriter + crash recovery：advisory lock、JSONL hash chain、journal 旋转、replay/recover/inspect |
| `assurance/keystore.py` | 305 | InstallationKeyStore：256-bit random key + Windows DPAPI / macOS Keychain |
| `assurance/key_lifecycle.py` | 483 | Key lifecycle：rotation、revocation、crash-safe journal、envelope migration |

### 关联 Schema（8 个核心 + 3 个 fixture）
已在上方 Schema 体系表中列出。

### 测试文件
| 文件 | 覆盖范围 |
|---|---|
| `assurance/tests/test_conversation.py` | ConversationNamespace 创建/状态/artifact/跨会话拒绝/导入 |
| `assurance/tests/test_permit.py` | Permit issue/consume/verify/expiry/binding mismatch/replay |
| `assurance/tests/test_envelope.py` | Envelope create/verify/expiry/parent chain/tamper detection |
| `assurance/tests/test_p1_archive.py` | Archive 状态转移/16 类删除/保留类别/unsafe entries |
| `assurance/tests/test_archive_journal.py` | Journal 写入/验证/hash chain/lock 并发/旋转 |
| `assurance/tests/test_keystore.py` | DPAPI 往返/密钥持久化/签名验证/tamper 失败 |

## 测试覆盖

### Conversation Namespace
- 创建合法 namespace（marker + state + envelope 一致性）
- 拒绝非目录/符号链接 root
- 拒绝 marker 缺失或格式错误
- 拒绝跨会话读取（`requested_conversation_id != self.conversation_id`）
- 非 active 状态拒绝写入
- kernel-owned 类别拒绝外部写入
- `import_untrusted_snapshot` 标记 `imported_untrusted`
- 路径遍历和 symlink 攻击拒绝

### Sensitive Action Permit
- issued permit 的 HMAC 签名和验证
- consumed permit 的 binding 匹配验证
- 过期 permit 拒绝消费
- binding 不匹配拒绝消费
- 重复消费拒绝（fail-closed claim）
- consumed permit 的 issued_receipt_sha256 链接验证
- TTL 受 envelope expires_at 约束
- 递归验证（consumed → issued）

### Archive Controller
- active → archiving → archived 合法状态转移
- 16 类删除完整执行
- 保留类别（3 类）跨越 archive boundary
- terminal deletion receipt 的 HMAC 签名
- unsafe entries（symlink/reparse point）记录但不遍历
- 状态验证（非 active 不允许启动归档）

### Archive Journal
- advisory lock 获取/释放（Windows msvcrt + POSIX fcntl）
- lock 竞争拒绝（LK_NBLCK / LOCK_NB）
- hash chain 完整性（previous_entry_sha256 → entry_sha256）
- journal 旋转（>1 MiB 自动分片）
- entry payload schema 验证
- 独立 verifier 重建验证

## 边界与限制

- **Conversation namespace 是文件系统级隔离**：不提供内存隔离或进程级隔离。恶意代码在同一用户权限下仍可访问 namespace 外的文件
- **Installation key 的 DPAPI 保护是用户级**：同一 Windows 用户的其他进程可解密 key。key rotation/revocation 尚未生产化（GAK-ID-001）
- **Cross-conversation read 默认拒绝**：显式 `import_untrusted_snapshot` 是唯一合法跨会话路径——但调用方需自行验证 snapshot 内容
- **Permit 不保留原始用户批准文本**：`raw_confirmation_recorded: false` 和 `raw_action_recorded: false`——事后无法从 permit 自身还原用户批准的原始内容，仅保留 SHA256
- **Archive 的 delete 是尽力而为的**：`unsafe_entries` 和 `residual_entries` 记录在 terminal deletion receipt 中——不静默失败，但也不保证物理销毁
- **Journal lock 是 advisory**：恶意或 buggy 进程可忽略 lock。lock 最多等待 30 秒（每秒轮询一次）
- **当前为 development/conformance baseline**：P1 已完成 contract + fixture 层 + ArchiveController crash recovery（journal replay + retry + exponential backoff + stale lock cleanup）；key rotation/revocation 第一切片已完成（GAK-ID-001）；真实 runtime adapter 接入（GAK-SESSION-001）与 retention/deletion 生产化 stress test（GAK-RET-001）仍待推进

## 下一步

后续适合接入：
- Key rotation/revocation 的完整生命周期（GAK-ID-001）
- Conversation namespace 接入真实 runtime adapter（GAK-SESSION-001）
- Retention/deletion controller 的生产化 stress test 与多进程并发恢复（GAK-RET-001；crash recovery / retry / journal replay 第一切片已在 `ArchiveController` 中实现）
- Archive journal 的直接追加模式（替代当前的全量重写）
- Permit 的 approval ledger 集成（`INTERACTIVE_APPROVAL_LEDGER_CONTRACT`）
- Cross-conversation import 的安全策略（当前仅标记 untrusted，不做内容验证）
