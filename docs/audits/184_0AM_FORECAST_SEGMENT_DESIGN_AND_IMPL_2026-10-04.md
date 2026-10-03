# 184 批：0am 预测段设计卷＋实现批（2026-10-04）

> **用户令**：「请开始进行"0am 余项：预测段设计卷＋实现批"吧，做完以后请不要进行重建」＝182 批登记的 0am 余项**放行**（设计卷落卷＋实现批落码）；**边界＝不重建、不推送**（本批双仓本地提交）。
> **性质**＝设计卷＋落码批；零契约面（journal schema／`rli.notice.{kind}` key／侧车 schema v2／路由与喂入面全部不动）。计数不变 58。

---

## §0 结论速览

| 面 | 结果 |
|---|---|
| 设计卷 | [`LIF_RLI_FORECAST_SEGMENT_DESIGN_2026-10-04`](../LIF_RLI_FORECAST_SEGMENT_DESIGN_2026-10-04.md) **v1.0**——形态＝**仅前推预告段**（182 定案逐条落卷）：闭式前推值＋趋势（死区 0.05）＋段内越线判定（五值闭集）；**到达概率不进模型面**（S0 结构否定＋S0.5 经验否定双重复核的兑现面）；`prog` 期望注入修正不进段；**零新触发**（P9：只在既有 streak fire 渲染） |
| 实现 | orz 本批 `001c7768`（未 bump）：`RliForecast`/`RliForecastTrend`/`RliSegmentCrossing` 三类型＋`RliChannel::forecast`（纯自由演化）＋`scan_crossing`（固定网格 64 点机械扫描）＋`RliShadow::forecast_segment_text`；**单 fire 行附预测段**（多 fire 行保持机制成因，沿成因段宽度纪律先例）；horizon 沿用预注册分通道表（`rli_horizon_steps`，零新增参数） |
| 行宽两预算重订 | ① 直投行预算 `RLI_NOTICE_TEXT_BUDGET` **320 → 400B**（合成最坏单 fire 形态实测 **383B**／宽度钉驱动 **372B**；注解同步扩展「前推=闭式自由演化至h·T̂」）；② `rli.now` 面让位预算 `NOW_BUDGET` **830 → 960B**（830B 会把近提醒一并让位＝追溯面事实死亡；960B＝保底一条最坏提醒＋省略行 ≤ 下方 `BOARD_CAP` 1024B 硬钳制，两条最坏提醒时按让位次序退至一条） |
| 验证 | orz-assurance lib **301/0**（＋预测段钉 3：五值越线闭集逐值／趋势死区与非法入参／多 fire 不附预测段；宽度钉与预算钉随批更新）；orz-loop lib **849 过／3 忽略／1 失败＝先存偶发**（见 §3①）；clippy **17=17／71=71 零新增**；触碰面 fmt 干净；重放件回归 **run1 与 r178 基线逐位全等、run2 本批树 vs 冻结树逐位全等**（§3②） |
| 渲染样例（钉驱动实测） | `持续越线: deny×3（u=3.84≥θ=1.19；失配概率 —）；源：计划/车道拒绝×2＋门/护栏拒绝×1＋检索启用拒绝×1＋其他拒绝×1；前推(10·T̂): 3.84→-0.52 ↓，段内回落；T̂(…)：8.00〔…前推=闭式自由演化至h·T̂…〕` |
| 计数 | **不变 58**（0am 线内延续；无立项／无闭合） |

## §1 设计卷要点（落卷定案）

- **三要素**：① 闭式前推值 `u_h`（`free_evolution_at(h·T̂)` 的 u 分量——与 `advance` 同公式、半群性由构造保证）；② 趋势 ↑/↓/→（死区 `RLI_FORECAST_TREND_DEADBAND=0.05`，与 T̂ 趋势段死区同口径）；③ 段内越线五值闭集（θ 上：`段内θ上持续`/`段内回落`/`段内再越线`；θ 下：`段内不越线`〔S0 实证主态〕/`段内越线`〔机械升越，结构性近零、出现即如实渲染〕）。
- **纯自由演化**：`prog` 的 λ̂ 期望注入修正**不进预测段**（S0.5 到达面双重复核否定的兑现——到达概率及其估计量不进模型面）；streak 白名单五通道本就不含 `prog`，生产渲染路径永不触达该分支。
- **承载面收窄**：唯一承载＝StreakCrossed 单 fire 行；域事件提醒族不附（无域级前推读数依据，非目标登记）；`section=rli` PULL 面不扩（既有 `pred(hT̂)` 已在前推值，趋势/越线留消费读数再议）。
- **P9/P7/P3 合规**：零新触发（渲染时纯读计算、不改状态不推进不反馈）；零输出内容读取；新增常数三件全为构造/实测常数（网格 64／死区 0.05／预算 400），horizon 沿用预注册表零新增。

## §2 实现与代码面

- `orz-assurance/src/lif/rli.rs`：类型三件（`RliForecast`/`RliForecastTrend`/`RliSegmentCrossing`＋`as_str` 闭集单源）；`RliChannel::forecast(steps, t_hat) -> Option<RliForecast>`（T̂/steps 非法＝`None` 不渲染）＋`scan_crossing`（`(0, h·T̂]` 均匀 64 点、`≥θ` 与 streak 同口径、τ=0 恒不取）；`RliShadow::forecast_segment_text`（`前推(h·T̂): u₀→u_h ↑/↓/→，段内{五值}`）；`observe_streaks_and_fire` 单 fire 臂接线（成因段之后、T̂ 趋势段之前）；StreakCrossed 注解扩展；预算 400。
- `orz-loop/src/controller.rs`：`NOW_BUDGET` 830 → 960（注记含与 `BOARD_CAP=1024` 的连锁算术）。
- **不动面**：路由器／喂入点／通道参数／θ 自校准／streak k／域机／看门狗／繁杂度／直投装配点（`take_pending_rli_notices` 语义不变——notice text 加段而已）／侧车 schema／journal 契约。

## §3 验证明细与两项先存发现（如实登记）

1. **orz-loop 先存偶发**：`retrieval::dispatch::tests::user_cancel_closes_pending_activations_before_run_cancelled` 本批树上失败（30ms cancel 竞速窗，`close_reasons` 得 0）——**冻结树 `54717d06` 上同样失败**（本批 stash 实测），与 0aq 登记「orz-host 负载敏感」同族（时序敏感非逻辑缺陷）；本批不处置（零范围蔓延），留 0aq/flaky 面随批登记。
2. **重放回归的两层对拍**：run1 与 r178 基线（`s3_run1_p8a_regression.json`）**逐位全等**；run2 与 r178 基线有一处分类计数翻转（`RUN-72764d4e-2` neutral_zero 16→17／verify_pass_zero 6→5）——**冻结树重放同样 17/5**（本批 stash 实测）⇒ 翻转先存于本批（r178 基线 JSON 生成之后语料面有变动——语料根为 scbench 输出树活体，非本批代码差异）；**本批树 vs 冻结树两轮语料均逐位全等**＝本批零路由/喂入漂移的构造性核证。回归件：`D:/tb-eval/0am_s3/s3_run{1,2}_forecast_regression.json`。
3. **fmt 先存差异**：examples／credential 等多处 fmt 差异为冻结树先存（本仓 fmt 非全局门、纪律为触碰面干净）；本批触碰面（`rli.rs`／`controller.rs`）已清零。
4. 自检：`rli_shadow_replay --selftest`／`--s3-selftest` 全过。

## §4 边界与后续

- **不重建、不推送**（用户令）——本批零载体动作；真机读数留下一真机轮（设计卷 §4.4：0bc 同轮观测顺接，09-20 顺序裁决不变）。
- 0am 余项自此**全部清账**（预测段为最后一项）——线内后续仅余真机轮读数与 0bc 进场。
- 非目标（设计卷 §5）：域事件提醒族附预测段／PULL 面趋势越线扩展／`prog` 前推修正的模型面表达／到达概率段重开（Hawkes/爆发感知 hazard 须新依据）。

## §5 台账

- 本档：`docs/audits/184_0AM_FORECAST_SEGMENT_DESIGN_AND_IMPL_2026-10-04.md`。
- 设计卷：`docs/LIF_RLI_FORECAST_SEGMENT_DESIGN_2026-10-04.md` v1.0（随批落卷）。
- TODO：`P1-0am` 增预测段设计卷＋实现批行（勾选）；P1 路由行「设计卷与实现批待放行」改为已落；头部计数行指针（计数不变 58）。
- BACKLOG：`0am` 节批序增 184 条目；P1 总览行同步；开放项路由行同步。
- BACKLOG 第二卷：§1.137 本批流水。
- 索引：头行 v4.155 → **v4.156**；`AUTH-RLI-BASE-SHADOW` 余项尾更新＋设计卷入口；`AUTH-LIF-RLI-STIMULUS-TYPED-BUS` 批序尾增补。
- 源清单：`generate_orz_source_manifest.py` 随批再生成（差异恰 2 行＝两触碰文件；1486 entries）；机械门禁落账后复跑 `check_repository.py`＝**valid true**。
- 提交：orz 子树 `001c7768`（2 文件）＋父仓本批（**不推送、不重建**——用户令）。

## §6 关键词

184 批、预测段设计卷、前推预告段、闭式前推值、趋势死区、段内再越线、五值越线闭集、
网格 64 扫描、单 fire 渲染、到达概率不进模型面、prog 期望注入不进段、行宽预算 400、
NOW_BUDGET 960、BOARD_CAP 连锁、零契约面、零新触发 P9、先存偶发如实登记、
重放两层对拍、计数 58 不变。
