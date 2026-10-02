# 164 批：0cn S2 落码——budget 轮数记次注入撤除＋ADR §14.82 转录（2026-10-02）

> **日期**：2026-10-02；**用户令**：「请先进行0cn吧」（本批开工令）、「暂时不重建」（构建/载体重建暂缓令，本次会话内 cargo 编译与测试不跑、S3 进体顺延）。
> **形态**：S2 落码批（子仓 orz 改动＋父仓契约面/文档）；主会话直接执行。
> **计数**：不变 **59**（0cn 仍 `pending`，S2 达成、S3–S4 待续）。

---

## §1 撤除范围与落点（S1 定稿＝撤除的落码）

处置对象＝`mechanical_audit_update kind=budget`「已用 N/999 轮」注入行（0CJ 报告 §3.1：recli 轮实测 136 条、每工具批一刷新＝当前模型轮次感的事实主来源，999 为占位值）。按 S1 用户裁决（161 批补记 §7.3「单纯不传这个轮数记次了，这个注入撤销」）整体撤除：

1. **逐批事件写入撤除**——`orz-loop/src/agent_loop.rs`：原「MECHANICAL-AUDIT-LAYER 预算键每轮末覆盖写」（`record_budget` ＋ `kind=budget` 事件）整块删除；同 `if profile.role == AgentRole::Main` 块内的 0bg `lif_domain` 域迁移连带记录**保留不动**；原位留退役注记（引 §14.82）。
2. **审查表面**——`orz-loop/src/mechanical_audit.rs`：`record_budget()` 方法删除；`report()` 报告块去 budget 行——「预算：{已用 N/999 轮}（，墙钟约 Xs）」与「预算：暂无」兜底文案退役，改为无墙钟不渲染、有墙钟单行「墙钟约 Xs」（墙钟行为 0cg 拆除范围之外的既有机械面，本批不动）；模块头对象键清单标注 budget 已退役。
3. **kind 常量**——`KIND_BUDGET` 标注**已退役（历史回放保留）**：生产零写入，枚举值保留只为历史 journal 仍可校验（删值＝让旧刊判 invalid）；`#[allow(dead_code)]` 同 `KIND_ATTENTION_LADDER` 先例。
4. **e2e 事件序列期望同步**——`orz-bin/src/main.rs` 三处 conformance capture（tool-snapshot-run／orientation-fire-run／mode-off-refusal）：每工具轮 `mechanical_audit_update` 期望双条（tool_result＋budget）→ 单条（tool_result）；checkpoint 相对位置注记随更。
5. **不受影响面（边界核对）**：`blackboard_read section=session` 轮预算 PULL 面（真值语义）不动；`budget_insufficient` 预检硬门与轮预算上限不动（deny 码词汇表 `channels.rs` 不涉）；`budget_cue_injected`（0au/148 批已退役）不涉；终审报告块注入时机（反例门同轮）与 injected-block filter 不动。

## §2 ADR 转录（§14.80 纪律：裁决先入 ADR 再进代码注释）

- **新增 §14.82「budget 轮数记次注入撤除（0cn）（2026-10-02，v1.83）」**（vol-14 卷尾）：撤除裁决（三选一定为撤除、不真值化、不并入 temporal、不设替代物接受时间感回归）；契约面保留（schema/镜像 kind 集合保留 `budget` 值、生产零写入由单测钉子钉住、session 面轮预算 PULL 不受影响）；报告块形状（执行事实／墙钟／异常事实三类）。
- 主文件入口页同步「冻结版本补记（2026-10-02 追加 v1.83）」条目。

## §3 契约面与测试

- **runtime schema**（`mechanical-audit-update-event-payload-v0.2.schema.json`）：kind 枚举与 allOf 分支**保留 `budget` 值不删**（历史 journal 回放纪律）；description 标注 budget 已退役（撤除裁决＋保留理由）。Rust 契约钉子（`mechanical_audit_update_schema_matches_kind_constants`）不需要改——枚举未动。
- **Python 冻结镜像**（`assurance/run_event_journal_validation.py` `_verify_v02_mechanical_audit`）：kind 集合保留 `budget`；docstring 标注退役（同 attention_ladder 注记式样）。
- **测试**——`mechanical_audit.rs`：报告测试更名 `report_contains_only_exec_wallclock_and_anomalies`（断言不含「预算」「已用」、无墙钟时不渲染墙钟行）；终答注入测试改为断言模型面无预算行＋**budget 零写入钉子**（本 run 不得出现 `kind=budget` 事件，引 §14.82）。
- **fixtures**：存量 fixture（含历史 `kind=budget` 事件）按历史刊口径**保持合法**，不重生成。

## §4 验证边界（如实登记）

- **本批未跑 cargo 编译与测试**：本会话环境 `protoc` 缺失（`orz-tools-api` build script 失败），且用户令「暂时不重建」——构建验证与 S3 载体重建一并顺延。改动区域静态自查：`cargo fmt --check` 下 `mechanical_audit.rs`／`orz-bin/main.rs` 无 diff；`agent_loop.rs` 的 fmt diff 均位于本批未触碰行（0bs F11 已登记的 rustfmt 版本噪声，本批不做全量格式化）。
- **补跑口径**：`cargo test -p orz-loop`（含 mechanical_audit／终答注入／零写入钉子）＋`cargo test -p orz-bin`（编译级）＋`check_repository.py` 随 **S3 进体批**一并执行后本节补记。
- schema/镜像改动为描述性文字与注释，枚举/键集零变化（历史回放语义不变）。

## §5 批序与后续

- 0cn：S1 设计定稿（达成，161 批）→ **S2 落码（本批达成）** → S3 进体（**暂缓**＝用户令「暂时不重建」；届时补跑本批验证）→ S4 真机核证。
- 0cp S2 落码仍待放行，与本批相邻但独立不并批（163 批口径不变）。

## §6 台账

- TODO：计数行本批指针＋P1 路由行 0cn 注记＋`P1-0cn` S2 勾选（含验证边界注）。
- BACKLOG：本批记录指针＋计数行＋P1 总览行 0cn 注记＋锚点行＋`0cn` 节 S2 达成行。
- BACKLOG 第二卷：§1.117（本批流水）。
- 索引：头行 v4.133 → **v4.134**＋§8 `pending` 桶 0cn 条目注记（S2 达成、S3 暂缓）。
- ADR-0010：vol-14 §14.82＋主文件 v1.83 补记（本批 §2）。
- 源码清单：`orz_source_manifest.sha256` 重生成 **1,485 条**（0cn S2 触碰三文件摘要同步，随提交批）。

## §7 关键词

164 批、0cn S2 落码、budget 注入撤除、999 占位值、已用 N/999 轮、轮数记次、
不设替代物、时间感回归、§14.82、v1.83、零写入钉子、墙钟行保留、S3 暂缓、计数 59。
