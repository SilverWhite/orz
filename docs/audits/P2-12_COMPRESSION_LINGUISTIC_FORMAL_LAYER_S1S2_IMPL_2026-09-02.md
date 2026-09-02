# P2-12 语言学形式层与域参考——S1/S2 实施记录（2026-09-02）

> 性质：实施记录（非设计裁决）；设计权威 = 讨论稿
> `COMPRESSION_LINGUISTIC_FORMAL_LAYER_DISCUSSION_2026-09-02.md` §3/§6（方案 A，
> 2026-09-02 用户逐项裁决）+ BACKLOG P2-12；渲染语义投影更新见
> `CONTEXT_COMPACTION_DESIGN_2026-08-14.md` §4.4.4。
> 范围：S1（聚合状态 + 渲染改造）+ S2（单测/e2e）。S3 重建完成（2026-09-02，
> 见下）；S4 实机复验 / ADR-0010 转录待续（实施未闭合，计数不变）。全面审查
> 处理（2026-09-02）见
> `P2-12_COMPRESSION_LINGUISTIC_FORMAL_LAYER_REVIEW_HANDLING_2026-09-02.md`
> （溢出补全段代码修复 + 错误码可复核口径/跨 run 边界登记 + 文档修正）。

## S1 实施内容

1. **聚合分区**（`orz/crates/orz-loop/src/failure_agg.rs`，新增）：F4 失败目标
   聚合分区 `FailureAgg`——行键 = (kind, id)；每行含 preview（≤80 B）、计数、
   错误码集合 `CodeCount`（全留，首次出现序）、首末相对 run 起点墙钟秒、行内
   域段书签 `DomainSegment`（同域并入 / 异域开段，from/to 决策轮）。
2. **黑板接线**（`blackboard.rs`）：`failure_agg` 为黑板直属分区，epoch 作用域
   ——随 `EpochSnapshot` 归档/恢复、随 plan epoch 轮换清空（与 exec/actions 同
   纪律）；旧归档缺字段经 serde default 兼容。非 `blackboard_read` 查询分区
   （PULL 面与模型可见面零新增）。
3. **写时盖章**（`host_exec.rs` 三处 F4 失败事件 + `lif/mod.rs` 访问器）：host
   工具错误（code = ToolErrorKind → `tool_timeout`/`tool_not_found`/
   `execution_failed`，新增 `CODE_TOOL_NOT_FOUND`）、content-anchor 拒单
   （`content_anchor_mismatch`）、候选门拒单（`{family}_candidate_*`）在写入
   时取当轮 LIF 域值 + 轮号盖章（`ensure_run_origin` + `run_relative_secs` 保证
   与 temporal `t` 同一 run-relative 时间轴）。
4. **渲染改造**（`summary.rs`）：注意事项槽第 2 数据源由「exec.errors 最近 5
   条截断」改为 F4 聚合行——`[失败目标 <kind>] <preview> ×N | codes=[…] | 首末
   Xs–Ys | 域 normal(r10–12)→pressure(r13)`；exec 分区原文不再复制进槽（压缩
   不携带日志级明细）；行键即跨 marker 去重键。排序 = 计划面失败/阻塞 → 失败
   目标聚合 → 动作失败 receipt。

## S2 测试

- `failure_agg::tests`：同目标合并计数/首末/码集、异目标分行、域段归并/开段、
  空码兜底、serde roundtrip。
- `summary::tests`：排序（计划 → 失败目标 → 动作）、合并渲染、3K 溢出指针
  （200 行）、空态、原 exec 窗口断言退役。
- `blackboard::tests::failure_agg_rotates_and_snapshots_with_epoch`：轮换清空、
  快照保留、恢复带回、旧归档 JSON 兼容解析。
- `agent_loop::tests::compaction_marker_and_archive_carry_facts_notes`（更新）：
  marker 与存档同源同序携带聚合行，exec 原文不进槽。
- `host_exec::tests::failure_target_aggregation_stamps_on_write`（新增 e2e）：
  同一 file_target 两次超时 → 事件面带 `failure_target`、聚合分区一行
  count=2 / codes=[tool_timeout×2] / 域段按轮盖章、exec 原文照旧、渲染正确。
- 全量：`cargo test -p orz-loop --lib` 654 passed / 0 failed / 3 ignored
  （653 + 审查处理新增 `archive_annex_carries_overflowed_failure_rows`）；
  `cargo test -p orz-assurance --lib` 199 passed；`cargo fmt --check` 干净；
  clippy 无新增警告（新增 `record` 的 too_many_arguments 已按既有惯例 allow）。

## S3 重建（2026-09-02）

- Linux musl 三件套（ORZ-BUILD-MOUNT-001 契约，`build_orz_aliyun_trixie.sh`，
  rust:1.97-slim，`-j1`）BUILD_EXIT=0；`D:\tb-eval\orz-target` 缓存此前缺失，
  本次为全量冷构建，编译 41m11s；构建日志 `D:\tb-eval\orz-linux\build-20260902.log`。
- 三件套 2026-09-02 21:13 HKT：orz 106,926,480 B（sha256 31b4b44c…0299）/
  orz-signer 1,390,376 B（08988bc8…5def85）/ orz-acaf-provision 1,208,232 B
  （bdc98e76…2269f）。
- 静态核验：EM=x86_64、无 PT_INTERP、无 `ld-linux-x86-64` 字符串（musl 静态）。
- 接线符号：P2-12 聚合分区/渲染在二进制内（`failure_agg` ×43、`failure_target`
  ×17、存档补全段「失败目标聚合」头命中）；既有守卫 `retired_tool_denied` ×12 /
  `content_anchor_mismatch` ×13（连续性）。
- 冒烟：debian:bookworm-slim 容器三件正常加载执行（provision usage、signer
  manifest 缺失 fatal、`orz --real` 缺 key 与 `--version` tty io error 均属
  预期加载后行为）。
- 对应源码：orz f0eeb524（P2-11 依赖图主线，父 4868df39）+ 工作树 P2-12
  S1/S2 与审查处理改动（未提交：8 文件 683 insertions / 83 deletions + 新增
  `failure_agg.rs` 243 行）——本二进制同时覆盖 P2-11 依赖图主线 S1/S2。

## 边界与后续

- 无 F4 身份的 exec 原文（如全工作区 grep、参数缺失）不进注意事项槽（仍在 exec
  分区可回查）——与「压缩不携带日志级明细」一致；若需保留可后续加残余窗口，未占
  计数。
- `failure_agg` 行数在 epoch 内随不同失败目标增长（与 exec 分区无界同纪律，
  已知边界）。
- 待续：S4 实机复验、ADR-0010 转录（P2-11 §14.48–§14.51 模式）、BACKLOG/TODO
  闭合入账。
