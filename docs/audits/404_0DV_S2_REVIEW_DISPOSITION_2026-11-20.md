# 0dv S1/S2 深度审查处置批（404 批，2026-11-20）

> **性质**：审查处置批（计数 73 不变；零 bump 零重建零换装；不推送）。**来源**：用户令「请对当前实现的 0dv S1/S2 部分做全面深度审查」——三维对拍（设计合理性／实现合理性／设计与实现符合性）完成，发现 P2×1＋P3×4＋P4×1，用户令「请对审查出的全部问题进行处理」，本批全数处置。审查基线＝402 设计稿＋403 落码（orz `2b534ffb`）。子仓 orz 本批提交（6 文件，见 §3 读数）。前批审查处置先例＝399（0dw S1）/391（0dx）。

## §0 审查结论（存档）

总体通过：设计勘定扎实（drain 两调用点、单次消费共账核到行号级）；peek-then-drain 竞态守卫语义自洽（观察者恒不消费，消费只在唤醒 run 定案后）；三裁决方向正确；两处实现修订（peek 走持久化账、e2e 留 S4）有据且如实登记。符合性逐条对拍成立（S2 六件＋三裁决＋契约链全绿；「双法官零改动」声明核实成立）。发现均为打磨项：无阻断项。

## §1 处置清单（六项全数闭合）

1. **P3-1（seen 标记时序）→ 创建回执确认制**：窗满开火只登记 **pending**（在途账），`seen` 由创建回执落账——`Created`/`BusyRejected`（窗内用户 prompt 竞态臂，胜出 run 的 B1 收走 facts）⇒ seen 标记；`Failed`（bootstrap/模型/会话态等创建臂自身失败）⇒ 仅回滚 pending＋冷却置位（`IDLE_WAKE_FAILURE_RETRY_MS`=60s，防持续失败按去抖窗节奏空转重试），seen 不动 ⇒ 观察者可再次为这些任务开窗。403 形态的「非 busy 失败即永久不再唤醒（进程级）」静默降级消灭，零丢失语义增强。纯函数 `wake_creation_outcome`／`resolve_wake_creation` 供钉测。
2. **P2（观察者状态机零自动测试）→ 状态机拆纯函数＋四钉**：`idle_wake_window_step`（空 peek 清窗／开窗／续窗／窗满开火＋pending 登记／冷却臂）、`resolve_wake_creation`（三回执落账＋冷却后重开窗组合臂）、`wake_creation_outcome`（Busy vs 其他 Err 分类）、peek pending 过滤臂（既有钉扩臂）。账本重构为 `IdleWakeBooks`（windows/seen/pending/cooldown，`Arc<Mutex>` 短临界区，均不持锁跨 await）。watcher 全链 e2e 仍留 S4 真机窗（不变）。
3. **P3-2（persist 滞后残余竞态）→ 写盘确认**：`FinalizedToolset::persist_state`（fire-and-forget `save`）改 `persist_state_and_flush`（`save_and_flush`＝并合挂起快照＋写盘 ack 回执，writer 掉 ack 映射 BrokenPipe；失败仅告警不 panic——内存账已一致，后续存档自然补档）；`OrzHost::drain_completed_tasks` 改调。writer 停滞超过去抖窗时「下一 run toolset build 读旧账 → 重复投递」的窗闭合。round-trip 钉（返回即文件在盘可读）。
4. **P3-3（写回面两处不一致）→ 注入块登记**：`INJECTED_BLOCK_PREFIXES` 加 `IDLE_WAKE_TRIGGER`（`[idle_wake]`——唤醒投递消息头由既有 `[结果投递]` 前缀同族覆盖不重复登记）；`is_generated_injected_block` 两臂＝机械 prompt **精确同文**（`IDLE_WAKE_PROMPT` 编译期常量，用户引用前缀不命中）＋投递消息头（`IDLE_WAKE_DELIVERY_HEADER` 新常量，`render_idle_wake_message` 改由常量拼接）头后必接事实生成形态（`" completed (`）。写回侧车／恢复过滤／`/rewind` 隐藏三面同治，与 B1/B2 投递消息同族待遇；用户真打机械 prompt 精确同文 ⇒ `note_user_authored_input` 豁免表优先保留（318 P1-C5 纪律不受损）。
5. **P3-4（单 ticker 偏差漏登记）→ 403 档 §2 追记**：设计稿 §2.1「会话域观察者（与 StoredSession 同生命周期）」vs 落码「进程级单 ticker（windows/seen 按会话键控）」——可观察行为等价，随批补记为显式偏差（见 403 档 §2 第四条）。
6. **P4（每 tick 每会话同步读盘）→ 登记不修**：`load_reported_task_ids` 每 peek 同步 `std::fs::read_to_string`（async 上下文、LocalSet 单线程）——文件小＋2s 周期实际无感；处置＝本档登记，**重开触发器＝观察面扩大（会话数上限提升/多工作区高密度）或实测 tick 抖动**，届时加 mtime 缓存。不预优化。

## §2 行为差与设计语义影响（如实登记）

- **P3-1 新增 402 设计稿未定义的臂**：设计稿 §2.2 只定义「窗内用户 prompt 到达 ⇒ 取消唤醒」（busy 臂）；「唤醒创建自身失败」臂 404 定型＝回滚 pending＋60s 冷却重试。S4 九条判据不变（判据 ⑧ busy 臂语义逐位保持）。
- **P3-3 使唤醒 run 两类机械文本不再持久化**：机械 prompt 与唤醒投递消息自本批起不写回会话侧车（403 期以准用户消息形态保留）；恢复后的会话不含唤醒痕迹（journal 全链照记，机械源标记在案）。与「机械注入块绝不持久化」既有纪律对齐，B1/B2 投递消息同待遇先例在案。
- **P3-2 纯收紧**：drain 返回时持久化账已落盘（原为最终一致，窗口期依赖观察者 seen 兜底）；观察者 peek 注释随改（「写盘确认保持新鲜」）。

## §3 读数

| 套件 | 读数 | 备注 |
|---|---|---|
| orz-loop lib | **993/0/4**（与 403 基线持平） | 本批 loop 侧为扩既有钉（prompt 前缀/生成形态清单两处、controller persist 过滤钉四臂），零新增钉函数 |
| orz-tools lib | **3065/0/6**（＋1） | `persist_state_and_flush_round_trips_to_disk_on_return` |
| orz-host 串行 | **467/0/6**（＋3） | 状态机三钉；**必须 `--test-threads=1`**（并联 env 竞态 8 失败实证，家规跑法在案） |
| orz-assurance | **311/0＋11/0＋4/0** | 契约链零改动（corpus 267 不变） |
| orz-bin 六套件 | **24/2/15/24/2/1 全绿** | banner 面零改动 |
| clippy 四 crate | host lib test **43（33 dup）＝干净基线持平**（本批新警告 4 条 `unnecessary_get_then_check` 当批消灭）；余 crate 无本批新命中 | stash 对照法 |
| fmt / git diff --check | **净** | fmt 随批格式化 acp_server.rs 一处（纯空白） |

## §4 边界

- 零 bump 零重建零换装（403 翻转行为本就随下次重建进体，本批处置件同窗）；**不推送**。
- S4 九条判据随下次换装窗不变（可与 0dw S3/0dy S3/0dz S3/0dx 活体/0dt 判据 4 合并首启窗同车）。
- Python 契约面零改动（无 schema/corpus 变更，Python 套件免跑）。

## §5 入口

- 审查对象：[402 设计稿](../IDLE_WAKE_BACKGROUND_COMPLETION_DESIGN_2026-11-19.md)／[403 批档](403_0DV_S2_IDLE_WAKE_IMPL_2026-11-19.md)（§2 本批追记单 ticker 偏差）／立项档。
- 台账：TODO 404 批行／BACKLOG 本批记录指针／索引头 v4.364。
