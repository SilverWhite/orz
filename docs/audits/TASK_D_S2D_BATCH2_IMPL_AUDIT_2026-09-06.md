# 任务 D S2d 批 2 实施审计：probe_accuracy 收窄至当前可见工具（2026-09-06）

> **性质**：任务 D S2d 翻转前裁决清单**批 2（裁决二）实施审计**——用户
> 放行后落地。裁决登记：
> [`TASK_D_S2D_FLIP_ADJUDICATION_2026-09-06`](TASK_D_S2D_FLIP_ADJUDICATION_2026-09-06.md) §2；
> 设计转录：ADR-0010 §14.58。orz 提交：`00b9a440`（feat/fusion-architecture）。

## 1. 实施内容

### 1.1 ADR-0010 §14.58 转录（裁决二设计权威化）

- 新增 `### 14.58 v1.58 补写裁决索引（2026-09-06）`：不变量收窄管辖范围
  （仅当前可见声明面工具）/ 生产侧唯一改动（记账口径收窄）/ 判官两侧
  零改动 / 工具面根本变动时重新议定的边界登记。
- 头部冻结版本行追加 v1.58。

### 1.2 生产侧唯一改动：记账口径收窄（orz `00b9a440`）

- `tool_probe.rs` 新增 `narrow_to_declared(snapshot, is_declared)`：按
  声明面过滤快照的 complete/incomplete——**封存工具（R1 主面封存表内的
  工作工具、registry 缺席的工作工具）不进两个集合**。
- 两处探针装配点统一接入（run-start `controller.rs` + 逐轮
  `agent_loop.rs`）：`probe_work_tools` 产出的全量快照在进入事件装配、
  翻转最小图（`probe_state_seed`/`probe_flip`）与可见面投影三处消费前
  统一收窄。
- **声明面判定 = registry 在场 ∧ 非 R1 主面封存**，以检索模式投影后的
  基表（`tool_defs`）为判定输入——与可见面投影
  （`project_main_agent_tool_defs`）同源，保证「记账集 = 投影面的工作
  工具部分」。
- 收窄的可见面效果为零：封存工具本就不进投影（R1 retain 前置剔除 /
  registry 缺席不发明），改变的只是**事件面记账**——原 S2a 注记的翻转
  留痕点（封存工具探针翻转被记入 `tool_availability_check`，而
  header 不可能变化）就此封堵，probe_accuracy 判官对生产刊不再产生
  幻影违规。
- **判官两侧零改动**：Python `_verify_v02_probe_accuracy` 与 Rust
  `verify_probe_accuracy` 字面不动；历史 fixture 期刊（全量分区事件）
  继续按原语义回放。

> **2026-09-06 批 2 三路复审勘误**（[复审处理](TASK_D_S2D_BATCH2_REVIEW_HANDLING_2026-09-06.md)）：
> ①上句「判官两侧零改动对真实刊零误报」仅对 probe_accuracy 族成立——
> `tool_availability_probe` 族的 exact-partition 形状子句与收窄后生产刊
> 冲突，经用户补裁决收窄该子句（ADR-0010 §14.59），两侧判官已同步；
> ②「可见面效果为零」不完整——收窄快照另有 console 注册板块与环境
> 实体清单两处消费，`workspace.run_tests` 动作随收窄从板块/实体面消失，
> 经用户确认按预期收敛登记（ADR §14.58 项 2 v1.59 修正）。本文其余
> 数字以复审处理 §3 为准（spec 表 233×30=6990 格、对拍 251 语料×30=
> 7530 格 0 差、orz-loop 729 lib）。

### 1.3 S2 单测矩阵（同批）

- `narrow_to_declared` 纯函数三单测：封存 + registry 缺席剔除（保序保
  reason）；封存翻转不成翻转 + 声明翻转仍是翻转（`MinimalProbeMap` 语义
  锁定）；全链上下文收窄后 R1 封存工作工具全部离开分区（声明面 =
  探针矩阵 − R1 封存交集）。
- 两 e2e 改收窄口径：`probe_flip_emits_second_availability_event_and_
  reprojects` 与 `probe_state_resets_across_runs` 的翻转载体由封存
  `run_tests` 改为声明面 `run_terminal_cmd`（原测试注释明言「探针事件
  仍记录翻转，但声明面不受探针影响」——正是裁决二封堵的漏洞语义）；并
  增封存静默断言（封存翻转不产生第二个事件 + 分区两个集合均无封存工具）
  与翻转重投影断言（第二请求不再声明翻转的声明工具）。宿主改用
  `SealedAndDeclaredRegistry`（封存 + 声明各一）与 `BenchmarkFull` 策略
  （exec 链路可翻转）。
- `host_exec` 的 run_tests 探针测试改封存静默口径（分区断言替换原来的
  incomplete-reason 断言）。

## 2. 验收

| 项 | 结果 |
|---|---|
| **probe_accuracy 族零回归**：判官两侧字面不动，Rust↔Python 对拍 250 语料 × 30 族 = 7500 格 0 差（`s2b_family_verdicts_match_python`），18 个历史 fixture 期刊校验 0 错误 | ✅ |
| orz-loop **728 lib + 3 ignored 全绿**（含新增/改写 6 项探针测试）；orz-assurance 204 + 9 + 9 + 1 全绿 | ✅ |
| `cargo check --workspace` 零警告零错误；`cargo fmt --check` 净；clippy 仅存量告警（lif/acaf，非本次文件） | ✅ |
| spec 表 232 场景 × 30 族 = 6960 格（批 1 口径，判官侧无变化） | ✅ |

## 3. 裁决影响面对账（对裁决登记 §2 逐项核对）

| 项 | 裁决要求 | 实施结果 |
|---|---|---|
| 不变量收窄管辖范围 | 仅当前可见（声明面）工具 | ✅ 记账集 = 投影面的工作工具部分 |
| 生产侧唯一改动 | tool_probe 记账口径收窄 | ✅ narrow_to_declared + 两装配点接入；其余生产面零改动 |
| 判官两侧 | 零改动 | ✅ 字面未动，对拍/fixture 零回归 |
| 合法翻转继续受查 | 检索模式 A 降级等真实声明面翻转仍在管辖内 | ✅ 声明工具翻转照常记事件 + 受查（e2e 断言） |
| 边界登记 | 工具面根本变动时重新议定 | ✅ §14.58 项 4 转录 |

## 4. 遗留与下一步

- **批 1 + 批 2 均已完成，S2d 翻转前裁决清单清空**。
- 下一步 **S2d**：30 族全量 Python↔Rust 对拍矩阵落盘 + registry 翻转
  准备（spec 表 232 场景 × 30 族、对拍 250 语料 × 30 族 0 差为基线）；
  随后 S3（门禁 `check_repository.py` 改接 Rust 法官）→ S4（Python 双
  法官退役/归档收口）。
