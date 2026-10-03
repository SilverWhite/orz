# 178 批：0am 全面审查处置批——Ctx/Infra streak 显式排除＋deny_class 生产接线＋词表 v2＋S3 修正重放（2026-10-03）

> **用户令**：「做显式排除 Ctx/Infra 吧，做扩展没啥实际正向收益吧？请对审查出的全部所有问题进行处理，如施工面大，请派出子代理。」
> **前置**＝0am 线全面审查（同日，用户令「请对当前完成的0am部分进行全面审查……」）：四路并行子代理（设计合理性／orz-assurance 实现深查／orz-loop 布线符合性／验证证据复跑）＋主会话亲核（router/标签环/成因段/on_bus_event/快照 v2/完成臂 diff/θ 动力学量化）。**结论：无 P0；6 项 P1＋一批 P2**，本批全量处置。
> **orz**＝`181c9cc9`（14 文件，+723/−78；父仓 pin＋manifest 随批）。**计数不变 57**（0am 线内批）。

## §0 结论速览

| 面 | 结果 |
|---|---|
| **streak 观察域（审查要害）** | **Ctx/Infra 显式排除（用户裁决）**——`observe_streaks_and_fire` 改白名单 err/stall/slow/deny/Verify；审查实证：单次压缩于死窗 θ 收敛后（q=0.95/η=0.05/θ₀=1.0，数百采样点后 θ<0.7）经实极点慢模态高台（32·T̂，τ≈150s）几乎必然凑满 k=3 成「ctx×3」单事件回声——与 S2 §2 Ctx「防单事件尖峰」意图相反；S3 重放零总线馈入＝该形态从未被观察。Ctx/Infra 标签环照记（数据面），无 fire 渲染路径。新增**真差分钉**（旧口径下必红） |
| deny_class 生产接线 | `feed_lif_deny` 改收码串、21 站点带码单源 `DenyClass::of_code`；完成臂信封码喂入点解析；**plan_write_lane_denied 臂施工中新发现漏喂随批补喂＝第 22 站**（消生产-重放分歧：重放按 `is_denial_code` 判 Deny 而生产原不喂）。通道值语义零变化（Deny 1.0 照旧），仅标签环数据面增维 |
| run_tests 休眠臂 | Err/Ok 两臂预接臂（镜像完成臂形状；提前 return 不双喂）——R1 封存下不可达，解封复活不再有 Verify 死窗（审查 P1-6） |
| 验证词表 v2 | `cargo fmt`→`cargo fmt --check`（对齐 S2 设计原意）；＋`npm run test`/`nextest`/`pnpm test`/`yarn test`/`bun test`；词界计入下划线；`.exe` 归一；**首词守卫闭集 29 程序**（`&&`/`||`/`;`/`|` 分段剔除只读首词段——`grep -rn pytest .` exit 1 不再假判 Verify）；变更类扩 `apply_patch`/`write`/`edit`（适配器载体——非 grok_build 载体 Prog 防结构死窗，审查 P1-4）；`Submit` 标注保留类 |
| 行宽最坏形态 | 实测单 fire＋Deny 环 7 标签全渲染＝**349B>320B**→成因段机械截断（计数降序前 4，截断后 284B）＋钉（审查 P2） |
| 常数单源 | RLI 三周期引 1D 侧 `VERIFY/CTX/INFRA_TAU_ROUNDS`；1D 四构造器 T̂₀ 引 estimator `T_HAT_INIT_SECS`（RS-06 补齐 1D 半边）；`slow_weight` 引 `SLOW_W_MAX` 并 pub |
| example 预存破损（审查新发现） | `rli_forecast_probe.rs`/`lif_replay.rs` 缺 176 批 ToolEvent 新字段＝`cargo clippy --all-targets` **基线即红**（176 批「clippy 触碰面零新增」口径未覆盖 example 全体）——补 legacy 位修复 |
| S3 修正重放全量重收 | J1–J5 以 178 态全量重收（`s3_run1/2_20261003_r178.json` 仓外）：**注入账目与 175 基线逐位全等、J1 恰等闭合保持**；唯一改判＝首词守卫表实锤（二跑 `RUN-72764d4e-2` seq76 `cat > tests/test_….py` heredoc 假命中→中性，零注入总量不变）；J4/J5 与 176/177 修正态位级全等；**176 批档「53.6%」勘误＝29/55=52.7%（以 JSON 为准）**。审查 P1-3 的「S3 有偏读数未全量重收」就此闭合 |
| 文档与治理面 | S2 档 **v1.2**（bank 算术勘误收口 L44/L128/L142/L150、§2 streak 观察域勘误段、§4.1 词表 v2、**§4.4 已知边界与豁免登记**①–⑧、J5 处置注＝**域机重校准酌处结果＝不动**、§7 不变面如实化）；S0 档 **v1.3**（P7/P9 边界澄清注——不新增触发规则形态，通道扩充经既有内生机制生效）；0cp 设计稿三处 240B 残留清理；盘点档 §13 B 类批注（**B02 闭合**、B01/B03 随本批重放收取）；177 批档补记（「四构造点」勘误＝生产 3＋测试助手 1；fold_write_failed 同款零产出者复查项）；TODO/BACKLOG 随批 |
| 挂起线头去向 | 审查 P1-3 四线头：J5 重校准＝**酌处不动**（S2 §6 注）；FR-B01/B03＝本批重放收取（a4 容差与过阈率读数面见 §13 批注；若读数面缺项由台账批改判）；FR-B02＝闭合（S2 §4.4⑧）；fold_write_failed＝复查项登记（177 补记）；θ 衰减地板提案（162 批 C 案）与真机轮 Verify 读数预注册＝**登记入 0am 待办**（本批不实施——动力学面按 P3 须单独立项） |
| 验证 | orz-assurance lib **297/0**（288＋新增钉 9）＋orz-loop lib **848/0/3**＋`--selftest`/`--s3-selftest` 双绿＋clippy 触碰面零新增＋两轮全量测试多次复跑 |

## §1 审查发现→处置对照（P1 全收）

| 审查编号 | 发现 | 处置 |
|---|---|---|
| P1-1（三路合流要害） | streak 观察域随 bank 扩容静默 4→7、与 rli.rs 自身文档矛盾；单次压缩→ctx×3 回声量化可达；S3 未喂总线 | **Ctx/Infra 显式排除（用户裁决）**＋文档改写＋真差分钉；总线馈入的重放覆盖以引擎差分钉承担（语料无总线密集段，journal 重放不增信息） |
| P1-2 | `of_code` 生产零调用，四组拒绝类 live 不可达；重放/生产标签层不同源 | 21＋1 站点带码接线＋完成臂信封码解析（§0） |
| P1-3 | 四线头静默悬挂＋S3 有偏读数未重平 | J5 酌处不动（S2 §6 注）；B01/B03 随本批重放；B02 闭合；fold_write_failed 登记；S3 修正重放全量重收（§3） |
| P1-4 | 适配器变更工具落 Neutral＝Prog 结构死窗 | apply_patch/write/edit 入 Mutate＋S2 §4.1 对账（语料零事件＝零账目扰动） |
| P1-5 | Verify 死窗 θ 语义换轨未登记、有效性零实证 | S2 §4.4① 已知边界登记；真机轮 Verify streak 查准/转向相关预注册读数登记入 0am 待办；θ 地板提案纳入视野 |
| P1-6 | run_tests 休眠路径零喂入 | 两臂预接臂（§0） |
| 施工中新发现 | plan_write_lane_denied 臂漏喂（第 22 站）；两 example 预存编译红 | 随批补喂＋修复（§0） |

## §2 代码面明细（orz `181c9cc9`）

- **orz-assurance**（4 文件）：`lif/rli.rs`（streak 白名单＋文档如实化＋标签环/成因段 doc 两句＋行宽截断＋宽度钉＋RLI 周期引 1D 常量）；`lif/router.rs`（词表 26→31＋`cargo fmt --check`＋下划线词界＋`VERIFY_NON_ACTION_HEADS` 29 守卫＋分段匹配＋`.exe` 归一＋适配器三工具＋`slow_weight` pub 引 SLOW_W_MAX＋of_tool/头注 doc 勘误＋9 测试钉中 8）；`lif/channels.rs`（四构造器 T̂₀ 引 estimator）；`examples/rli_shadow_replay.rs`（tier 封闭枚举对账＋slow_weight 同源＋s3-selftest 增钉）。
- **orz-loop**（8 文件）：`host_exec/failure.rs`（feed_lif_deny 改签名单源解析）；`host_exec/tool_run.rs`（9 站点＋完成臂 deny_class＋run_tests 两臂＋plan_write_lane_denied 补喂）；`agent_loop.rs`（7 站点，4760 变量码站点按 `reason` 逐码判定——`write_gate` 码族 retrieval_role_*→RetrievalEnable／console_*·plan_write_lane_denied→PlanLane）；`controller.rs`（ACAF 票据码）；`delivery.rs`（console 码族）；`host_exec/serp.rs`（两站实测码 `browser_control_*` 落 Other——of_code 如实兜底）；`host_exec/candidate.rs`（candidate 码族→GateGuard）；`mechanical_audit.rs`（两处 240B 注释勘误）。
- 两 example 修复（`rli_forecast_probe.rs`/`lif_replay.rs`）：补 `policy_denied: false, routing: None`（legacy 语义位，行为零变化）。

## §3 S3 修正重放读数（r178，全量重收）

- **J1 恰等闭合保持**：首跑 230＝115 事件（118 标签）＋46 verify 零注入＋69 中性；二跑 172＝67 注入＋**34** verify 零注入＋**71** 中性（35/70→34/71＝唯一改判：首词守卫表剔 heredoc 假命中，两侧均零注入、注入账目逐位不变）。
- **J2 三回声全部保持**：写控 join 两例恰 1 次 Deny(写控)；61s unittest（wall=60815）双豁免；首跑 Slow 9 注入全验证类时长；err streak 驱动源不存在。
- **J3 死窗**：S2 slow 两轮 u≡0；Ctx 10/4、Infra 18/17、snapshot 跨档 0（seen 8）。
- **J4**：u_prog 首跑 0.7299/0.8425→0.5068/0.5775、二跑 0.7689/0.8480→0.4610/0.4387；165 悖论 legacy max 0.9756/ge975=2、S2 馈 ge975=1；窗一致 62/136、117/169；lp 查准首跑 S2 馈 29/55=52.7%（**176 批档 53.6% 系算术小误，随批勘误**）、二跑 81/68=83.9%。
- **J5**：一致率首跑 0.9338/0.8529、二跑 0.9704/0.8994（与基线位级同）。
- **总判**：词表修订未破坏对账封闭；r178 即 S3 修正读数正式版（175 批读数的历史账目由本节与本件取代）。

## §4 边界与不变面

- **P9 兑现**：本批零新增触发规则——streak 观察域为**收敛**（回 4 压力通道＋Verify）；deny_class/词表/守卫全为「通道看见什么/标签是什么」面；成因段截断为渲染规则。
- 通道注入值语义零变化（297/0＋848/0 全绿即证：deny 分派、Slow 权值、影子注入、直投钉逐位不变）。
- 载体重建＋字面量核证仍为 0am 余项（本批不重建；现网二进制仍 legacy 刺激语义——「未生效」非「已损坏」，侧车 v2 双向 fresh 降级）。
- W1 施工曾用 `git stash` 对拍基线（窗口期涉并行流 worktree）——主会话验证轮已逐 hunk 复核两流改动并存无重叠、测试全绿，无损失。
- 既有 flaky 观察：`retrieval::dispatch::tests::user_cancel_closes_pending_activations_before_run_cancelled`（Windows 本机偶发，干净 HEAD 可复现，30ms 取消时窗竞速）——非本批引入，未处置，登记为观察。

## §5 台账

- 本档：`docs/audits/178_0AM_REVIEW_DISPOSAL_2026-10-03.md`。
- S2 设计档 v1.1→**v1.2**；S0 设计档 v1.2→**v1.3**；0cp 设计稿残留清理；盘点档 §13 批注；177 批档补记。
- TODO：头部计数行指针、P1 行 0am 注记、`0am` 节 178 勾选行、摩擦盘点节批注更新。
- BACKLOG：本批指针（177 转前批）、计数行、P1 总览行、`0am` 节条目；第二卷 §1.131。
- 索引：头行 v4.149 → **v4.150**；`AUTH-LIF-RLI-STIMULUS-TYPED-BUS` 条目更新。
- orz：`181c9cc9`；父仓 pin＋`orz_source_manifest.sha256` 随批。
- 机械门禁：落账后复跑 `check_repository.py`。

## §6 关键词

178 批、0am 审查处置、Ctx/Infra streak 显式排除、单事件回声封堵、deny_class 生产接线、
22 站点、run_tests 预接臂、验证词表 v2、首词守卫、适配器变更类、行宽截断 top-4、
常数单源、example 预存破损、S3 修正重放 r178、J1 恰等保持、heredoc 假命中改判、
53.6% 勘误、S2 v1.2、S0 v1.3、J5 酌处不动、B02 闭合、计数不变 57、索引 v4.150。
