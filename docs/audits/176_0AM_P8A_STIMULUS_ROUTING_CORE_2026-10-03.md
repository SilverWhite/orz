# 176 批：0am P8-a——刺激面路由核心进体（S2 路由表实现批第一节）（2026-10-03）

> **用户令**：「请继续」（接 P8 放行后的实现批）。
> **本批**＝P8 第一节落码：**路由核心**（标签器模块化＋喂入扩展＋双 bank 扩容＋生产接线）进体。成因段渲染＋行宽重订＋RS-06＋载体重建＋bus 站点补布线＝**P8 尾批**（§3）。计数不变 57（0am 线内批）。
> **orz**＝`663aad89`（7 文件，+944/−294；父仓 pin＋manifest 1486 条随批）。

## §0 结论速览

| 面 | 结果 |
|---|---|
| router 模块 | `orz-assurance/src/lif/router.rs`（新，555 行）：`ActionClass`（动作类闭集＋`of_tool` 标签器＋验证词表词边界匹配）、`DenyClass`（拒绝类六分组＋`of_code`）、`StimulusRouting`（喂入路由键：class/deny_class/non_zero_exit/write_control_block）、`stimulus_targets`（**纯分派**：deny 优先／验证通过零注入＋Slow 豁免／H2 填平／Prog 收窄变更类／**未路由＝旧四值语义回退**〔合成事件与旧调用面零破坏〕）、`BusStimulus`（Ctx/Infra 总线认领）。**零触发语义（P9）**——模块不携带任何阈值/触发规则 |
| ToolEvent 扩展 | ＋`policy_denied: bool`＋`routing: Option<StimulusRouting>`（保持 `Copy`；构造器默认 None＝legacy 回退） |
| 双 bank 5→8 | `ChannelKind`＋Verify/Ctx/Infra；RLI 侧周期 `8/32/64·T̂`（S2 §2 预注册初值）、三新通道**实极点分支**（与 prog 同族）、horizon 默认档、`index_of` 追加不移（既有五通道索引稳定＝旧侧车通道序兼容）；1D 侧 `VERIFY/CTX/INFRA_TAU_ROUNDS` 同物理依据、决策轮 τ 重导出、advance/decay 全接入；快照 schema **v1→v2**（通道族变更显式化；restore 通道数校验使旧 v1 侧车 fresh 重启＝文档化降级） |
| 锚点 feature 面 | 13→19（新三通道 u/v 最小可读集；pred/env/r 按需后续）；`known_feature_names()` 自动扩展 PULL selector 门（零 schema 变更）；13→19 钉更新 |
| 生产接线 | tool_run **完成臂**：路由键随事件入引擎（`ActionClass::of_tool(name, command)`＋非零 exit 位＋policy_denied 信封位）；**ToolError 臂**：写控兜底 block 判定（`exec_policy::BLOCK_MESSAGE_PREFIX` 冻结前缀单源）→ `DenyClass::WriteControl`——S3-J2(a) err 回声源的生产侧摘除。21 处 `feed_lif_deny` 不动（Deny 值语义 S2 不变；拒绝类标签维随 P8-b 成因段） |
| 总线 API | `LifEngine::on_bus_event`（Ctx/Infra 注入＋**snapshot 跨档过滤**〔v1.1：run 首测基线不计〕＋影子前转）；**生产 bus 站点布线＝P8-b 首项**（发射点普查已入档 §3） |
| 重放件同源 | 本地标签器/分派整段删除，切 `orz_assurance::lif::router`（S2 §5 同源复用兑现）；读数账目接口不变 |
| 回归读数微移 | 根因＝S3 重放件曾把验证通过事件改写为 Other（五通道引擎时代的重放近似）；P8-a 生产语义 outcome 保真、域机 Start 门正常释放。二跑：S2 馈 lp 主张 62→**81**／命中 53→**68**（83.9%）／S2 语义窗一致 106→**117**/169；首跑：55 主张 29 中（**53.6%**，基线 41.9%＝lift +11.7pp；175 版 47.4%→53.6%）／窗一致 57→**62**/136。**注入账目逐位不变**（legacy/s2 tool 通道 totals 两轮全等）——微移只在域机输入面，不在刺激面 |
| 验证 | orz-assurance lib **287/0**（基线 281＋router 矩阵 6 例：词表边界/标签表/分派矩阵/legacy 回退/总线认领/引擎 bank+bus 钉）＋orz-loop lib **848/0**＋`--selftest`/`--s3-selftest` 双绿＋clippy 触碰面零新增 |

## §1 S2 档勘误随批

§2 标题「通道集定案(5→7 kind)」系**算术笔误**——通道表实为既有 5＋新 3＝**8**；实现以 8 为准（`RLI_CHANNELS: [ChannelKind; 8]`），S2 档标题已勘误（v1.1 注）。此前 174/175 批档与索引沿引的「5→7」一并随本批台账更正表述。

## §2 边界与不变面

- **零触发（P9）兑现**：router/分派/总线全部只定义「通道看见什么」；streak k=3、θ 自校准、域机规则、看门狗 181s、提醒沿全部原样；成因段（P8-b）只描述已触发事实。
- **8 工具面/journal 契约/黑板 PULL schema 零变更**（feature 名集经 `known_feature_names()` 运行时门，非 schema enum）。
- **21 处 deny 调用面零触碰**；`feed_lif_deny` 保持 legacy 形态（行为不变）。
- 预存 fmt 漂移面（orz-loop 43 处/credential 等）**未触碰**（crate 级 fmt 曾误改 12 文件已还原重提，提交严格限于 7 文件触碰面）。

## §3 P8 尾批（下一批）待办清单

1. **bus 站点布线**（engine API 已 live，生产发射点接线）：`context_compressed` ×2（agent_loop 1115/1601）、`transport_retry` ×2（agent_loop 3217/3266，RliWatchdogGuard Drop 语境）、`tool_availability_check` ×4（agent_loop 2921＋controller 4079/4092；controller 5259 为测试）、`ledger_fold_advance/write_failed` 发射点（待普查定位）、宿主资源三族（经 `facts.rs` drain 映射）——逐点核 lif 可达性后接线；
2. **成因段渲染**（P8 两档：机制成因＋标签成因随 S2 §4.3 闭集）＋行宽重订（现 ≤240B 必越限）＋0cp D6「无成因」子句勘误；
3. **RS-06 随批**（libm `=0.2.15` 精确钉、P3 五小件；`rli_shadow_replay` fmt 已随本批干净）；
4. **载体重建＋字面量核证**（下一重建窗口；RLI 侧车 schema v2 的字节判据随批）；
5. FR-B02 设计 §8 边界注（文档级随批）。

## §4 台账

- 本档：`docs/audits/176_0AM_P8A_STIMULUS_ROUTING_CORE_2026-10-03.md`。
- S2 设计档：§2 标题算术勘误（5→8）。
- TODO：头部计数行指针、P1 行 0am 注记、`0am` 节 P8-a 勾选行。
- BACKLOG：本批指针（175 转前批）、计数行、P1 总览行、`0am` 节 P8-a 条目＋P8 尾批清单。
- BACKLOG 第二卷：§1.129。
- 索引：头行 v4.147 → **v4.148**；`AUTH-LIF-RLI-STIMULUS-TYPED-BUS` 条目 P8-a 状态。
- orz：`663aad89`；父仓 pin＋`orz_source_manifest.sha256`（1486 条）随批。
- 机械门禁：落账后复跑 `check_repository.py`。

## §5 关键词

176 批、0am P8-a、路由核心、router 模块、stimulus_targets、ToolEvent 扩展、ChannelKind 5→8、
双 bank、锚点 feature 19、快照 schema v2、写控块判定、on_bus_event、跨档过滤、同源复用、
重放回归微移、S3 近似修正、lp 主张 81/命中 83.9%、5→7 算术勘误、P8 尾批清单、
计数不变 57、索引 v4.148。
