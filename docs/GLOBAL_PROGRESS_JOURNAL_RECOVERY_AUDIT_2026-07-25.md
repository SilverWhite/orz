# Global Progress journal recovery 审计（2026-07-25）

状态：no-model、explicit repair、disposable integration fixture。

## 恢复资格

恢复工具默认只读，并与所有已知 writer 使用同一 `<journal>.lock`。只有下列字节状态可恢复：

1. 完整且链一致的最后事件缺少最终换行；
2. 无换行尾部不是有效事件，但此前非空前缀通过 schema、run、manifest、sequence、previous digest、
   payload digest、event digest 与 terminal-order 重放。

以换行结束但含坏行、历史中段损坏、空前缀残片以及错误来源绑定均为 `unrecoverable`。工具不会尝试猜测、跳过坏行
或寻找更早的“看起来有效”截断点。

## 显式修复与证据

修复必须指定 `--apply` 与新 receipt 路径。被丢弃的尾部必须先逐字节写入不覆盖的 quarantine，并在恢复事件及收据中
记录 SHA-256、offset 和长度。journal 随后通过锁内原子替换写入：

- 非终态前缀追加一个 `artifact_registered` recovery event，使链内可见发生过修复；
- 终态前缀不允许追加任何事件，恢复只能由 quarantine 与外部 receipt 记录；
- 替换后立即执行完整 replay；若失败，工具尝试恢复原始损坏字节并 fail closed。

恢复后的 journal 可以继续由正常 `append_event` 扩展。再次对有效 journal 执行恢复会被拒绝，receipt 和 quarantine
也从不覆盖。

## 已验证边界

专用测试覆盖八项：torn tail 隔离与恢复、完整事件缺换行、中段损坏拒绝、终态前缀不追加、首事件残片拒绝、只读检查
服从共享锁、非有限值损坏不会绕过 hash 重建，以及 CLI inspect→apply 入口。全量回归为 181/181（prototype 58、Grok integration 44、
assurance 48、runtime 31）；仓库机械检查覆盖 100 个 schema 并返回 0 errors。

## 限制

本工具恢复的是可机械证明的 JSONL 尾部写入中断，不证明事件语义正确。`fsync` 与原子替换降低进程崩溃窗口，但本轮
没有证明磁盘控制器断电、Windows/POSIX 目录项持久化、网络文件系统或多主恢复语义。终态链无法加入 recovery event；
若 journal 替换后、外部 receipt 写入前再次崩溃，只能从 quarantine 与最终 journal 状态重建审计。
