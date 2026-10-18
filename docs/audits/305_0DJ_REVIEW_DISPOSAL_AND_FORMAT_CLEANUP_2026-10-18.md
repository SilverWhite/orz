# 305 批：0dj 审查发现全数处置＋历史格式漂移清理（2026-10-18）

> 性质：**修复＋勘正批**。用户令「请开始处理 0dj」，并追加「一并处理之前的格式漂移」。
> 被处置清单＝304 批 R1–R20；源码提交＝orz `5fd4c42d`（24 文件，+504/−135，未推送）。
> 计数维持 **66**：0dj 已达到实现处置态，正式闭合仍待用户裁决；0di/0dh 的 S3 仍按既定计划同车重建。

## §1 结果

0dj 的 P1×1、P2×7、P3×12 已逐项处置：需要改代码、测试或文档的项目全部落地；经核验属于
既有架构边界、现役错误面或 Claude Code 同形交互的低风险项明确登记不取。没有遗留的 P1/P2
修复项。格式漂移经 `cargo fmt --all` 一次性清理，之后 `cargo fmt --all -- --check` 与
`git diff --check` 均为净。

## §2 R1–R8（P1/P2）处置

| 项 | 处置 | 结果 |
|---|---|---|
| R1 | 修复 | recall 先校验 ordinal、构造候选并成功持久化，再清空补充队列和写回 continuation；越界不再丢队列。 |
| R2 | 修复 | `queue_append` 在 runs 同锁临界区内判定并入队；Prompt 运行可追加，Restore（含 recall）在途拒绝，封死 push-after-clear。 |
| R3 | 修复 | `persist_conversation_sidecar` 返回 `Result`；recall 落盘失败时恢复原 continuation、保留队列并返回 host error。新增确定性落盘失败回滚钉。 |
| R4 | 修复 | `recall_preview` 增加服务端 idle 门；web `/rewind` 增加运行中前端守卫。多客户端在 preview→confirm 间的自然陈旧窗口仍由 confirm 端 idle/ordinal 校验 fail-closed。 |
| R5 | 修复 | preview/recall 共享 `recallable_user_message`，排除 injected blocks 与 `[任务状态` 行；ordinal 只覆盖真实用户消息。 |
| R6 | 修复 | TUI/web 跟踪本 run 已投递补充数；failed/cancelled/invalidated/terminated 时显示「未写回会话、请按需重发」警告，成功结束清零。 |
| R7 | 登记边界 | 256 KiB×16 是传输上限，不是上下文预算；本批不移动 drain、不二次运行压缩算术，避免改变已冻结 loop-top/缓存前缀语义；极端满队纳入 S4。 |
| R8 | 修复 | TUI 的每个 `supplement_delivered` 都生成独立用户卡，不再按文本去重；同文 prompt＋两条同文补充钉为三卡。 |

## §3 R9–R20（P3）处置

| 项 | 处置 | 结果 |
|---|---|---|
| R9 | 登记不取 | close×recall 竞态在正常产品拓扑不可达；不为不可达支路扩张 close 状态机。 |
| R10 | 登记不取 | stdio ext 继续沿现役 `internal_error + data` 错误面，双前端可见原因文本；不单独扩协议错误码。 |
| R11 | 修复 | TUI runner 守卫顺序改为 restoring → grill → running，消除未来 grill 异步化时误入 supplement 队列的隐患。 |
| R12 | 修复 | 双前端回执改为「将在可用的下一次模型调用边界投递」，覆盖末轮、取消和跨 run 滞留。 |
| R13 | 修复 | web 省略号判断改用 `Array.from(preview).length`，与服务端 Unicode scalar/字符计数口径对齐。 |
| R14 | 登记不取 | recall_text 覆盖草稿是 Claude Code 编辑式重发同形行为；保留现状。 |
| R15 | 修复 | `supplement-delivered.minimal.valid.json` 的 `character_count` 由 12 勘正为 13。 |
| R16 | 勘正 | 304 的「16/17 枚」口径明确为 16 个列举子项、17 个测试函数；本批不再混写。 |
| R17 | 修复 | 跨 run 测试新增第二 run 的 `supplement_seq == 1` 断言并删死变量；host 新增每消息字节帽测试。 |
| R18 | 勘正 | 设计稿升 v1.1：字段名、双端默认顺序、marker 风险、实际 guard 顺序和失败提示全部对齐。 |
| R19 | 登记＋补钉 | 设计稿明确互斥为单进程域、grill 不登记 runs 但不 drain；queue_append×Restore、preview×Prompt 与失败回滚钉覆盖关键可达竞态。 |
| R20 | 勘正 | S4 增补双端/跨入口、失败/取消可见面、撤回×排队、机械块过滤、Unicode 与极端队列判据。 |

## §4 格式漂移清理

执行全仓 `cargo fmt --all`，除 0dj 功能文件外，吸收此前遗留在 host/web/codegen 的纯格式漂移，
包括 `browser_serp.rs`、`codex_app.rs`、`codex_permission.rs`、`local_browser/*`、`permission.rs`、
`project_doc_index.rs`、`reclaim.rs`、`session.rs`、`tools.rs`、`orz-web/src/{acp_pump,lib}.rs` 与
`orz-sampling-types/src/error.rs`。这些改动只做 rustfmt 规范化，不改变行为；与功能改动同提交是用户
本轮明确追加的范围。

## §5 验证

| 验证面 | 结果 |
|---|---|
| `cargo fmt --all -- --check` | 通过 |
| orz 子仓 `git diff --check` | 通过 |
| `orz-host` | 384 passed / 0 failed / 6 ignored（串行全库） |
| `orz-loop` | 886 passed / 0 failed / 3 ignored（串行全库） |
| `orz-assurance` | 303 passed / 0 failed |
| `orz-web` | 47 passed / 0 failed |
| web frontend smoke | 全部通过（含 0dj 新钉） |
| TUI 定向新钉 | 1 passed / 0 failed |
| TUI 全库 | 176 passed / 9 failed；9 项均为既有 ACAF signer 环境基线，零新增失败 |
| `generate_orz_source_manifest.py --check` | valid；1,488 entries |
| `check_repository.py` | `valid: true` / `error_count: 0` |

## §6 台账与下一步

- 设计稿 v1.1、ADR-0010 §14.88/v1.92、runtime fixture 与本档同步。
- 索引 v4.277、BACKLOG、TODO、BACKLOG 第二卷 §1.244 同步；0dj 标为「处置达成、闭合待用户裁决」，计数维持 66。
- 父仓已随子仓提交重生成 `orz_source_manifest.sha256`（1,488 entries），repository check 全绿。
- 下一步仍是 **0dh＋0di S3 同车，以 0.9.3 双平台载体重建进体**；之后按增补后的 0di S4 判据做真机核证。

## §7 关键词

305 批、0dj 处置、recall 原子性、persist rollback、queue_append 互斥、真实用户 ordinal、失败补充
提示、同文补充卡、Unicode scalar、格式漂移、cargo fmt、0.9.3 S3、闭合待裁决。
