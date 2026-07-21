# Grok disposable fixture checkpoint/delta spike（2026-07-21）

状态：fixture-only、hash-only、non-restoring。未接入 Grok launcher，未触碰真实用户 workspace，未改变任何
provider/protocol 语义。

## 目标与边界

本 spike 为后续“完整记录进程与文件变化”建立最小文件系统证据层：运行前生成 checkpoint，运行后生成 delta，
并让每个结论可由相对路径、bytes、SHA-256 与 aggregate 重算。考虑当前磁盘预算，v0.1 不保存正文或 blob，
因此能证明状态差异但不能恢复文件。

硬门槛：

- workspace 根必须是普通目录，并含 schema `0.1.0` 的 `.lif-disposable-workspace.json`；
- marker 必须声明 `marker_kind=lif-disposable-workspace`、唯一 `FIXTURE-<32 hex>` ID 和 `disposable=true`；
- checkpoint、delta 与 checkpoint input 均须位于 workspace 外；
- reparse point/symlink、非普通文件、大小/数量上限、case-insensitive 路径碰撞一律 fail-closed；
- 每次 capture 做两个连续完整 hash pass；任一文件的 size/mtime/file identity 或两次 file list/hash 不同即失败；
- delta 重新验证 checkpoint schema、receipt validity、file ordering/uniqueness、counts/bytes/aggregate 与 marker entry；
- 输出使用 create-new 语义，拒绝覆盖旧 receipt。

## Receipt 语义

checkpoint 登记完整文件清单与 aggregate；delta 登记 created/modified/deleted 的 before/after metadata、unchanged
计数、当前 aggregate 与 change aggregate。两者都只登记 metadata，不包含内容片段。

这些类别只表示两个观察时刻的文件系统差异，不能单凭 receipt 推断：

- 变化由 Grok、tool、用户还是其他进程造成；
- 文件内容在语义上正确；
- 两次 pass 之间以外不存在瞬时变化；
- 已具备 rollback/restore 能力。

## Mechanical validation

`fep-script-validation` 的 observation-first task board 能解析 Python，但把这个通用文件证据工具按
`data_generator` profile 识别时报告 generator API/seed/domain 不匹配；这是 validator domain mismatch，不是本
工具需要通过增加伪 seed 或神经动力学字段来“清除”的问题。

专用 unittest 覆盖：

- checkpoint/delta schema validation；
- created/modified/deleted/unchanged 分类；
- content marker 不进入 receipt；
- missing/false disposable marker；
- output overwrite 与 workspace 内输出拒绝；
- marker 变化拒绝；
- checkpoint aggregate 篡改拒绝；
- 主机允许创建 symlink 时的 reparse 拒绝。

Windows 本地结果：5 tests passed；与 event bridge regression 合并为 9 tests passed。

## 下一门槛

在接入任何 Grok tool run 前，需要新增独立 verifier，根据 checkpoint + delta 重建 current file metadata 并重算
aggregate；随后才评估内容寻址、加密、容量上限明确的 blob layer 与 conflict-aware restore plan。restore plan
必须默认只报告冲突，不能自动覆盖真实用户文件。
