# Global Progress 原子 journal 与 reason-code 迁移审计（2026-07-25）

状态：no-model、disposable integration boundary。

## 责任边界

仓库已有 `run-event-v0.1`、hash chain、`append_event` 和 replay 骨架，但原实现没有共享跨进程锁；
`reason-codes-v0.1.yaml` 处于 `frozen_for_design_review`，也没有与六个 `GPS-*` runtime-local control code
语义等价的完整集合。因此本轮补的是项目扩展层，不将其描述为成熟 CLI 的既有原生能力，也不改写冻结协议。

## 原子追加边界

`prototype/fep_agent_proto/journal_lock.py` 提供 Windows `msvcrt.locking` 与 POSIX `flock` 的同一 sidecar
锁约定。既有 `append_event` 和 transition appender 现在共用 `<journal>.lock`，并把 replay、链头校验、append、
`fsync` 和追加后 replay 放在同一独占临界区。

已验证：

1. 两个真实子进程用同一链头并发追加时恰好一个成功，另一个在获得锁后因链头已推进而拒绝；
2. 锁被其他进程持有时，appender 在显式期限后 fail closed，且不创建 journal；
3. 持久 sidecar 在锁释放后可复用，不通过删除锁文件制造 inode 分裂；
4. transition candidate 仍须通过 event/receipt schema、payload/event digest、run/manifest/sequence/hash-chain
   与 terminal 约束。

## reason-code 迁移边界

`global-progress-reason-code-migration-v0.1.yaml` 绑定当前正式 registry 的 SHA-256，并精确覆盖 receipt schema
中的六个 control code：

- `GPS-TRANSITION-PASS` 不生成 protocol reason；
- transition/focus/completion 三个目前无等价正式语义的 code 仅登记为下一修订候选；
- checkpoint/holistic 两个聚合 code 必须从源 verifier 投影已经注册且被实际证成的细粒度 reason，不能默认
  发出候选集合。

仓库检查会拒绝迁移表缺项、重复 control code、源 registry digest 漂移、未知正式 reason 或已注册 code 被误标为
候选。迁移表状态为 `design_only_not_protocol`；其中候选 code 不是正式 reason code。

## 验证与限制

专用 transition 测试为 9/9；全量回归为 173/173（prototype 50、Grok integration 44、assurance 48、
runtime 31）。仓库机械检查覆盖 98 个 schema、6 个迁移项并返回 0 errors。

sidecar 是 advisory lock：它保护仓库内协作 writer，但不能约束绕过该约定的外部写入者；本轮也没有证明 SMB/NFS
等网络文件系统上的锁一致性或多主容错。锁层与迁移登记仍属于 disposable integration fixture，尚未注入成熟 CLI
核心或真实模型回合。

后续补全：进程中断导致的无换行尾部残片检测、quarantine 与显式恢复边界见
[`GLOBAL_PROGRESS_JOURNAL_RECOVERY_AUDIT_2026-07-25.md`](GLOBAL_PROGRESS_JOURNAL_RECOVERY_AUDIT_2026-07-25.md)。
