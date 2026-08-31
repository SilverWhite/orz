# P2-11 第 1 项 PULL 自描述设计 — S1 全面审查与修复（2026-08-31）

> 状态：`partial`（S1 代码 / S2 测试 + 审查修复完成；S3 重建 + S4 实机复验待放行）。
> 范围：`blackboard_read` 增量头 + temporal 一次返回（P2-11 第 1 项）。
> 方法：根代理三向审查（设计合理性 / 实现合理性 / 设计与实现符合性）+ 设计分片子代理
> 交叉复核；计数层与接线层以全量静态扫描补齐（写点覆盖、序列化边界、锁序、有界性）。
> 未修改任何非审查对象文件；本轮所有修复经 `cargo test` 与 `clippy` 验证。

## 1. 审查结论

**S1/S2 可放行**：设计整体自洽、实现干净、写点覆盖完整、测试充分；发现 2 项中等问题
（迁移基线刻度错配、参数误用路径推进游标）、1 项低（读 actions 自触发徽章）与若干
低/观察项，本轮全部修复。原审查误判 1 项（N2）经复核撤销（见 §3）。

## 2. 发现与修复

| # | 级 | 问题 | 修复 | 验证 |
|---|---|---|---|---|
| M1 | 中 | temporal「迁移基线」与「版本游标」混用同一把尺：`migration_count ≤ round` 恒成立，读过 temporal 后（游标=当时 round）新迁移段沉寂或 `n` 少报，违背「n=自上次读取以来的迁移总数」 | controller 新增独立迁移基线游标 `temporal_migration`（存上次读 temporal 时的 migration_count）；读 temporal 双基线同时推进，读其它分区只推该分区 | 新回归测试 `pull_delta_migration_baseline_tracks_new_migrations_after_temporal_read`（读 temporal → 新迁移 → 读 plan 显示 `temporal+2 域迁移+2`） |
| M2 | 中 | 渲染层失败形状（未知分区 / receipt_id 组合误用 / 点读未找到）走成功路径：挂增量头并推进游标，违反设计 §3「参数错误不推进游标」 | 新增 `is_blackboard_render_error` 判定，host 层对失败形状不挂头、不推进；事件面 exit_code 0 既有 O4 契约保持不变 | 单测 `is_blackboard_render_error_detects_fail_shapes_only` + e2e 断言（unknown section 响应无 `[黑板增量]` 头） |
| M3 | 低 | 读 actions 前 `sync_console_registrations` 无条件 `set_registration` → 每次读 actions 都带自触发 `actions+1` 徽章 | `set_registration` 改 compare-and-set：内容相同不计数（设计「可见内容变化才计 1」口径） | 更新 `action_board_revision_bumps_on_mutations`（同内容不 bump、变化才 bump） |
| L1 | 低 | 设计 §2 表「temporal 派生 = round + migration_count」与 §4 示例矛盾 | 设计文档随 M1 统一为双基线表述（round 徽章 / migration_count 迁移基线） | 文档核对 |
| L2 | 低 | 设计 §6「机器可读面同文本面一致」不实（human 完整、structured 面 cap 内截断） | 措辞收窄为「文本面完整优先；机器面 entries 在 cap 内尽力一致」 | 文档核对 |
| L3 | 低 | receipt 点读 8_000 字符口径在 CJK 下可达 24 KiB 字节，与 structured 8 KiB cap 错位；加增量头后必截尾 | 点读上限改字节口径 `RECEIPT_DETAIL_MAX_BYTES = 8 KiB − 增量头预算(256 B) = 7 936 B`，截断改 UTF-8 字节安全 | 4 项 `render_actions_receipt_point_read_*` 测试更新并全绿 |
| L4 | 低 | 设计 §5 now 趋势行在轮数 <2 时省略、未说明早期轮次呈现 | 设计补一句「轮数 <2 时省略趋势行（实现按实际长度标注近 N 轮）」 | 文档核对 |
| L5 | 低 | 设计 §7 边界机制描述不准（ToolFilter 描述 vs 实际投影剥除） | 改为「子代理工具投影面不含 blackboard_read」 | 文档核对 |
| N1 | 观察 | 增量头 ≤256 B 无独立截断测试 | 新增 `pull_delta_header_truncates_at_256_bytes`（全分区大徽章 + 迁移段触发截断，断言首行 ≤256 B、UTF-8 安全、正文保留） | 测试全绿 |
| N3 | 观察 | 既有时序敏感测试 `subagent_wallclock_timeout...` 断言把主车道错误（contains("wallclock")）与合成收口混计，150 ms 预算下在途工具未完成即计 2 → 并行 flake | 断言拆分：主车道错误精确匹配恰 1；合成收口（`subagent_wallclock_timeout_mid_tool`）≤1（0/1 均合法） | 单测与全量并行均绿 |

## 3. 复核撤销项

- **N2（撤销）**：原判定「feature 尾注 `{latest:.3}` 对 String 是空操作」系误读——
  `latest` 来自 `Vec<f64>.last()`（f64 而非 String），`.3` 是有效精度（0.2 → "0.200"）。
  已还原原实现并加注释，测试断言 `0.200` 保持。
- **N4（未动作，既有登记）**：`restore_spikes` 的 `at_round/dwell_rounds` 近似重建是
  P2-10 F2 已登记项；本轮确认 `migration_count` 重建计数本身正确，不在本项范围。

## 4. 通过项（审查确认无问题）

- 分区版本计数写点覆盖完整：全树残留 `.push` 直写均在 `#[cfg(test)]`，生产路径统一走
  push_*/bump_*；`mark_step_*` 幂等返回 bool 全部调用点正确接线；rotate/restore 计数
  与设计一致；`#[serde(skip)]` 生效（epoch 快照/存档不携带版本）；锁顺序无反向死锁。
- 增量头格式契约（固定分区序、只列 delta>0、零噪音、≤256 B）、temporal 整响应
  ≤1 KiB（render 预扣 marker + host 层二次 enforce_bound）、非 temporal structured
  entries ≤8 KiB、工具描述补写（selector/k/name 已在 schema，闭合上轮 i5/i6 观察①）、
  事件面零新增字段、run 级游标复位覆盖全部入口——均通过。
- 7 个被 cargo fmt 重排的文件（lif/mod.rs、lif/channels.rs、reducer.rs、tool_envelope.rs、
  v1_fake_provider.rs、lif_replay.rs、orz-loop/src/lib.rs）逐字核对为纯格式重排，
  无实质内容变化；用户裁决保留。

## 5. 验证证据（2026-08-31）

| 项 | 结果 |
|---|---|
| orz-loop lib 全量 | 658 passed / 3 ignored / 0 failed（含修复后新增 2 项测试） |
| orz-loop 集成 | 1 passed / 0 failed |
| orz-assurance lib + 集成 | 193 + 8 passed / 0 failed |
| clippy --all-targets（orz-loop + orz-assurance） | 无错误；告警全部落在既有代码（本轮新增行无告警） |
| 修复定向测试 | pull_delta 5 项 / M1 回归 / N1 截断 / M2 判定 / receipt 点读 4 项 / wallclock / action_board / unknown-section 全绿 |

## 6. 处置

- 全部修复落在 orz 工作树（未提交）；父仓库文档（设计/ADR/BACKLOG/TODO/本审计）同步更新。
- 状态保持 `partial`：S3 重建（Linux musl 三件套）+ S4 实机复验待用户放行；S4 建议加一道
  断言：跨轮读取非 temporal 分区能观察到「域迁移+n」段。
