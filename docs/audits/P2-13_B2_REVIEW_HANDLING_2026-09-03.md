# P2-13 B2 全面审查处理（2026-09-03）

> 性质：审查处理记录（2026-09-03 对 P2-13 B2 渲染折叠当前实现的全面
> 检查——设计合理性 / 实现合理性 / 设计与实现符合性——所发现问题的处置
> 落地）。B2 范围定义见
> `P2-13_B2_RENDER_FOLD_IMPL_AUDIT_2026-09-03.md`；设计权威不变
> （`BLACKBOARD_CONVERSATION_SCOPE_FOLD_DESIGN_2026-09-03.md` v0.8 +
> ADR-0010 §14.52）。结论：无 P0；P1 一项（语义口径裁定 + 登记 + 语义钉
> 测试）、P2 三项（代码修复 + 用例）、P3/INFO 若干（登记或小修）。

## 处置清单（审查发现 → 处置）

1. **单段/单域长驻留下折叠退化为 4K cap、无域段标注（P1）——语义裁定 +
   登记 + 语义钉测试**：设计 §9.2.1（用户确认口径“展开子集 = 当前域段 +
   最近 K 轮 + 最近 20% 行”）的当前域段 = 整段展开是权威语义；单域会话
   （单段）不存在“更早域段”，折叠视图不产生标注行，旧内容只受 4K 字符
   cap 约束并经 domain+round 盲展开回查——这是该口径的既定后果，非缺陷。
   §9.1/R1“单域折叠退化为纯轮数区间”按“无域分支、展开参数只按轮数走”
   解释；不在单域段内额外引入轮数切分（那会改变用户确认的展开子集口径）。
   落地：`render_fold.rs` 新增语义钉测试
   `single_domain_partition_expands_all_rows_without_annotations`；总览
   是否满足长会话需求交由设计 §13.3 遥测（单域长会话的折叠读取/展开频率、
   域段数）在数据到位后复核，届时如需段内轮数折叠再回本裁决。
2. **折叠态下 receipt_id + 可折叠分区守卫旁路（P2）——代码修复**：
   `render_blackboard_section_fold` 的折叠分支加 `receipt_id.is_none()`
   前置条件——带 receipt_id 的 exec/edits/tool_actions 不再被折叠分支
   静默吞掉参数，仍走 render_section 显式报错（receipt_id 仅 actions；
   render-error 形状 → 不挂头、不推进游标）。新增控制器单元测试
   `fold_state_receipt_id_on_foldable_section_still_errors_explicitly`
   与 host_exec e2e
   `blackboard_read_receipt_id_on_fold_triggered_exec_errors_not_fold`。
3. **未达阈值 + 显式展开强制折叠（P2）——代码修复**：`fold_state` 改为仅
   T/W 触发（去掉 `|| expand.is_some()`）——未达阈值时携带展开参数输出与
   普通读取逐字节一致（B2 审计口径 #5，不额外裁剪、不报错）。补充登记：
   普通视图在 4K–64K 字符带按既有 cap 截断时，展开参数不绕过 cap（该带
   旧行不可经分区 live 读回查属 B1 及之前既有能力边界，不是 B2 回归）。
   新增控制器单元测试 `expand_below_threshold_matches_ordinary_read`。
4. **显式展开目标行可能被 4K cap“最近优先”静默裁掉（审查后补充发现，
   P2 同纪律）——代码修复**：折叠视图超上限时先保留显式展开目标行
   （protected），其余内容再按最近优先填充；目标自身超上限时显式提示
   “缩小轮数范围分批展开”，绝不静默空回。cap_fold_view 增 protected
   参数；三段折叠渲染经共享 `fold_view_lines` 装配。新增单元测试
   `folded_exec_expand_target_survives_cap_overflow`。
5. **标注行固定压段首、段内部分展开时时间序错位（P3）——代码修复**：
   标注行改落在该段**首个折叠行**位置（展开前缀在前、标注随后、K 窗口
   行继续），并统一三段装配逻辑（消除重复）；pre-stamp 标注时间范围改取
   折叠子集（与折叠计数一致）。新增单元测试
   `folded_edits_partial_expand_places_annotation_at_first_folded_row`。
6. **W 计量无短路（P3）——代码修复**：T 已满足时不再序列化整板算 W
   （controller 折叠路由先判分区字符，未达 T 才取 board bytes）。
7. **读路径锁序未注明（P3）——代码注释登记**：折叠分支注释写明
   “黑板读锁 → LIF 锁”的读路径顺序与写路径非嵌套纪律。
8. **折叠视图 50 行 cap 豁免（P3）——决策登记（不改代码）**：折叠视图按
   B2 审计 #7 以 4K 字符为唯一总上限，不叠加 50 行 cap（标注/展开行紧凑，
   字符上限已保证有界）；设计 §9.2.1“既有渲染 cap 为硬上限”按“字符 cap”
   口径执行，豁免随本文件登记。
9. **host_exec 守卫重复代码 + since_filter 重复（P3）——决策登记（不改
   代码）**：沿用文件既有参数错误分支样式；统一错误出口重构留待后续独立
   批次，避免本批扩大改动面。
10. **“归档 epoch 读…逐字节不变”措辞不精确（P3）——登记**：edits/
    tool_actions 的 cap 补齐（B2 范围）同时作用于归档读（render_section
    共享），>50 行或 >4K 的归档读输出较 B1 有变；该表述应限定为“折叠不
    作用于归档读”。epoch.rs 模块头补注：plan-epoch 生产面退役清理属 B3。
11. **current_domain 参数未消费（P3）——登记**：回退语义已定（无行轮回退
    最晚段，不做域二次过滤）；如后续需要在回退时给模型加“取最近段”注记，
    属可选体验项，B4 遥测后再定。
12. **render_capped_rows 冗余 total 形参（P3）——代码清理**：签名去掉
    total，调用点同步。
13. **“展开无匹配提示只在折叠态出现”语义（P3）——登记**：未达阈值时
    显式展开 = 普通读取（无裁剪、不报错、无无匹配提示）；无匹配提示仅在
    折叠态输出（与处置 3 同口径）。

## 验证

- orz-loop lib 全量：**682 passed / 0 failed / 3 ignored**（B2 审计基线
  676 + 新增 6：render_fold 语义钉 1、epoch 2、controller 2、blackboard
  e2e 1）。
- `cargo fmt --all --check` 干净；clippy（lib）无新增告警（仅既有告警，
  均不在本批改动行）。

## 待续

- B3 契约与收尾 / B4 验证排期不变（见 B2 实施审计 §7）。
- 单域长会话总览充分性待 B4 遥测（设计 §13.3）复核（处置 1）。
