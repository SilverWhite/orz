# P2-12 S1/S2 全面审查处理（2026-09-02）

> 性质：审查处理记录（2026-09-02 对 P2-12 当前实现的全面检查——设计合理性 /
> 实现合理性 / 设计与实现符合性——所发现问题的处置落地）。不产生新设计裁决；
> 溢出补全段实现口径沿既有用户边界（「指针须可回查、不能误导模型」）落为
> 「被挤出聚合行随压缩摘要存档保存」。设计权威仍为讨论稿
> `COMPRESSION_LINGUISTIC_FORMAL_LAYER_DISCUSSION_2026-09-02.md` §3/§6 与
> `CONTEXT_COMPACTION_DESIGN_2026-08-14.md` §4.4.4。
> 范围：S1 代码修正 + S2 测试补充（S1/S2 实施记录见
> `P2-12_COMPRESSION_LINGUISTIC_FORMAL_LAYER_S1S2_IMPL_2026-09-02.md`）；
> S3 重建 / S4 复验 / ADR-0010 转录待续（未闭合、计数不变）。

## 处理清单（审查问题 → 处置）

1. **溢出指针不可回查（中）——代码修复**：`failure_agg` 不是
   `blackboard_read` 查询分区（PULL 面零新增），注意事项槽 3K 溢出时被挤出
   的聚合行经普通回查入口取不到，指针「其余 N 条见 … 摘要存档」不可兑现。
   修复：`summary::render_facts_notes` 现返回 `NotesFacts { text,
   hidden_failure_rows }`（被挤出且未进槽的聚合行以完整行文本带出）；
   `summary_archive_markdown` 在溢出时把补全段「失败目标聚合（注意事项槽 3K
   溢出补全）」追加进压缩摘要存档；marker 与槽位 ≤3K 不变。接线：
   `agent_loop::run_template_compact` 把 `hidden_failure_rows` 传入存档。
   测试：新增 `archive_annex_carries_overflowed_failure_rows` + 溢出测试扩展
   （hidden 行全部以完整行缺席槽位、全部出现在存档补全段；纯计划面溢出时
   补全段为空）。
2. **host 工具错误码不可由事件载荷直接复核（中低）——登记口径**：
   `tool_completed.error` 是自由文本，`tool_timeout`/`tool_not_found`/
   `execution_failed` 由 ToolErrorKind 结构化映射产生但事件载荷无结构化 code
   （锚点/候选拒单路径的 `error` 字段即结构化 code）。处置：讨论稿 §3/§5 与
   `host_exec::host_error_code` 注释登记——§5 离线 verifier 须复刻同一映射
   或经独立 schema-first 切片补事件结构化 code；不属本切片改事件面。
3. **跨 run 时间轴口径（低）——登记边界**：聚合行内 t/轮号为 run 作用域
   （run-relative LIF 原点）；同一 plan epoch 跨 run/会话恢复存活时新旧行
   刻度不可比。当前接线（CLI 新 plan_id 轮换清空、ACP 每 prompt 重建黑板且
   不挂 epoch 归档）规避；登记于讨论稿 §3/§5、`failure_agg.rs` 模块头与
   CONTEXT §4.4.4；恢复同 epoch 继续累计属未来切片。
4. **§4.4.4 回查入口措辞（低）——文档修正**：exec 原文不进摘要存档（本次
   改动所致），回查入口更正为 `blackboard_read` exec 分区（live/epoch）+
   journal；聚合行溢出回查 = 摘要存档补全段。
5. **§4.4.1 残留现行措辞（低）——文档修正**：§4.4.1 标题下加 P2-12 修订
   标注（第 2 数据源与排序已被 §4.4.4 取代，正文保留为历史原文）。
6. **段轮区间语义近似（信息）——登记边界**：段轮区间 = 该目标在该域的失败
   事件首末轮（非域整体驻留区间）；同域隔空合并保留时间轴近似。登记于讨论稿
   §3、`failure_agg.rs` 模块头、CONTEXT §4.4.4。

## 验证

- `cargo test -p orz-loop --lib`：654 passed / 0 failed / 3 ignored
  （审查处理前 653，净增 1：`archive_annex_carries_overflowed_failure_rows`）。
- `cargo test -p orz-assurance --lib`：199 passed / 0 failed。
- `cargo fmt --check` 干净；clippy 无新增告警（改动均在既有警告集之外）。

## 待续

- S3 Linux musl 重建 / S4 实机复验 / ADR-0010 转录：不变，随 P2-12 放行纪律
  走（未入账、计数 28 不变）。
