# 169 批：0cp S2 审查处置——看门狗时间基转轴（P1）＋D7 回归分类勘误（P2）＋直投取走直取化（P3）（2026-10-03）

> **日期**：2026-10-03；**用户令**：「请对审查出的全部问题进行处理」（对象＝168 批未推送内容的三维度全面检查报告：设计合理性／实现合理性／设计与实现符合性）。
> **形态**：主会话直接执行（子仓修正批 orz `e36dcacb`＋父仓设计稿勘误＋ADR 勘误＋台账）。
> **计数**：不变 **59**（0cp S2 内的实现缺陷修正批，无新立项、无闭合）。

---

## §1 审查发现与处置对照

| # | 维度 | 发现 | 处置 |
|---|---|---|---|
| P1 | 实现合理性＋符合性 | **看门狗时间基错位**：`spawn_rli_watchdog` 直传 `now_epoch_secs()`（UNIX epoch ≈1.76e9）进 `RliShadow::on_watchdog_tick`，而影子内部轴＝run 相对秒（动作样经 `LifEngine::rel()` 转换后喂入、轴原点 0）——窗口①（`now−t0≥181`）对 epoch 恒真；触发时以 epoch 量级 Δt 推进闭式解＝全通道湮灭＋锚点序列记入 ~1.7e9 垃圾点＋后续动作样 `dt=0` ⇒ **影子永久冻结**（冻结态 u_prog≈0 持续 LowProgress 标签，反噬误导性域提醒；D2「看门狗样成触发沿→D4 直投」生产不可达）。单测未拦＝宿主接缝零覆盖（单测自洽小轴直调 shadow、集成测试不等 181s） | `LifEngine` 新增 `on_rli_watchdog_tick(t_wall, idle)` **唯一宿主入口**（`rel()` 转轴后喂影子；kill switch 恒 `false`），agent_loop 看门狗任务改走包装——轴转换收敛一点、宿主不绕行；接缝轴判别钉 `rli_watchdog_tick_converts_wall_epoch_to_run_relative_axis`（epoch 时间基下：窗未满不触发〔旧实现此处恒真必误触发〕／窗满恰产一样／动作锚不被盖、保持相对轴值 0.0／后续动作样照常结算） |
| P2 | 设计＋实现＋符合性 | **D7 回归分类不完备**：`recovery` 只枚举 Stuck/LowProgress→Normal；Pressure→Normal（`label()` 四值两两可达——错误压力直接衰减回正常、无需途经 LowProgress）落入进入分支＝`spike_entries` 多计＋模型面发「spike进入 pressure→normal」（恢复被说成进入）。设计稿 §2 D7 与 ADR §14.83 第 7 项的回归端枚举同源不完备 | 回归端改**「凡异常域→Normal」**（bootstrap Start→首域仍不计数不提醒）；`DomainSpikeEntry` 文档与 D7 注释块同步勘误；设计稿 §2 D7 勘误＋ADR §14.83 第 7 项就地勘误（主文件冻结版本补记 v1.85）；新钉 `domain_pressure_to_normal_counts_return_not_entry`（进入当刻提醒→回归静默仅计数→全集无「pressure→normal」进入文本→稳定确认照常一次且行内「进1回1」） |
| P3 | 实现合理性 | `take_pending_for_push` 原写法「全量置位后取队列**末尾 N 条**」依赖「已投递前缀／未投递后缀」非局部序不变量（当前全部变更点均维持、行为正确；但 `mark_notices_delivered(n)` 语义上允许部分置位形态，未来调用可取错集） | 改**按未投递谓词直取**（克隆先于置位、返回态随置位改写保持「与队列一致＝已装配」、`notice_delivered_total` 同步累计）——不变量依赖消除 |
| P3 | 实现合理性（卫生） | pull 投递退役后 `mark_notices_delivered_at`／`record_notice_delivery_accounting` 成为无生产调用方方法 | 补「**冻结面**」文档注记（保留 pub 库面服务历史侧车/复算，不删除不扩张） |
| 观察 | 设计 | 心跳为进程级量尺：并发检索子车道流式会暂时掩盖主车道卡死（有界——子代理预算必终结）；三同时中「无动作样」实际被「无活动盖章」蕴含（动作样必伴随盖章），判据实效由 idle 主导 | 不处置（设计已接受口径，登记备查） |
| 观察 | 文档 | 设计稿 §6 S2 括注误写「ADR-0010 §14.55 关联转录」（实际转录落卷 §14.83；§14.55 仅为 180s 出处引用） | 随本批 §6 勘误一并修正 |

## §2 边界

- 五通道数学／θ 自校准／繁杂度／λ̂／域机器（`label()`／`record_round`）／T̂ 估计零改动（0am）；D2 三同时判定逻辑、恰一样休眠/重武装语义、D4 装配点与 journal 形状、契约面（schema／Python 镜像）零变化——**纯实现缺陷修正，无语义新增，无需新裁决**。
- D7 勘误为**枚举完备化**（回归端原意图即「恢复不提醒」，Pressure→Normal 显然属恢复），非行为语义变更；设计稿版本仍为 v1.2（勘误随批注记，不升版）。

## §3 验证读数（Windows 本机，PROTOC 显式＝仓内 `bin/protoc.exe`）

- `cargo test -p orz-assurance --lib`＝**281/0**（279＋接缝轴判别钉＋Pressure→Normal 静默回归钉）；`cargo test -p orz-loop --lib`＝**848/0**（3 ignored；首跑 1 败重跑自愈＝148 批登记的 30ms 取消时序负载敏感预存 flaky 先例形态）。
- clippy（orz-assurance＋orz-loop，--all-targets）＝**106** 持平 168 批账面（零新增）。
- fmt：触碰三文件（`lif/mod.rs`／`lif/rli.rs`／`agent_loop.rs`）零新增 diff；未触碰面预存漂移照 148/168 批口径另记不动（本次 `cargo fmt` 对预存漂移面产生的重排已全部回滚，触碰面收敛三件）。
- 测试计数勘误附记：168 批档所记 orz-loop「847/0」与本批实测 848/0 的 ±1 差异＝首跑 flaky 件的通过/失败计入口径（848＋3 ignored 与静态测试属性计数 851 严丝合缝），非本批引入。

## §4 台账

- 设计稿：§2 D7 回归端枚举勘误＋版本行勘误注记；§6 S2 括注勘误（§14.55→§14.83）＋批序行补本批。
- ADR-0010：vol-14 §14.83 第 7 项就地勘误（回归端＝凡异常域→Normal）；主文件冻结版本补记 **v1.85**。
- TODO：`P1-0cp` 节补本批修正行＋头部指针行；BACKLOG：本批指针＋P1 总览行同步；BACKLOG 第二卷 §1.122。
- 索引：头行 v4.138 → **v4.139**＋`AUTH-RLI-ACTION-SAMPLING-NOTICE-PUSH` 条目补 169 修正批注。
- 机械门禁：落账后复跑 `check_repository.py`（见 §6 补记）；orz 源清单随批再生。

## §5 关键词

169 批、0cp S2 审查处置、看门狗时间基、epoch→run 相对轴、on_rli_watchdog_tick、
接缝轴判别钉、Pressure→Normal 回归、D7 勘误、凡异常域→Normal、take 谓词直取、
冻结面注记、影子冻结缺陷、计数不变 59、ADR v1.85。

---

## §6 补记（落账后门禁）

- 机械门禁：落账后复跑 `check_repository.py`＝error_count **0**、`valid: true`（2026-10-03 实测）；父仓 orz 源清单随批再生（`generate_orz_source_manifest.py`，1,485 条）。
