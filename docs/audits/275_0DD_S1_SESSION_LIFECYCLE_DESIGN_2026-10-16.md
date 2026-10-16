# 275 批：0dd S1 设计稿——会话生命周期用户主权（run 末自动归档退役触发面勘定＋D1–D8 定案）（2026-10-16）

> **用户令**：「请进行0dd S1吧」。
> **性质**：**设计稿批**——新增设计档一件；零源码、零 API、零跑批、零重建；
> **计数不变 61**；orz 子模块不动（HEAD `0ba00262`）。
> **结论先行**：肇事面勘定＝**ACP 车道 run 尾 500K 里程碑增量归档**（`handle_session_prompt`
> spawn 块，`incremental=true` 与 266 实证逐项吻合）经 web 分组投影（`isArchived` mtime 判定）
> 联动把存活会话挪入「归档会话」组；主裁决＝**该 spawn 块退役**（DP-11④ 的 ACP 半边），
> close 道／0ak 无头道／0cy 启动侧收卷三道原样；归档原语单一源不破。
> 设计档：[`SESSION_LIFECYCLE_USER_SOVEREIGNTY_DESIGN_2026-10-16.md`](../SESSION_LIFECYCLE_USER_SOVEREIGNTY_DESIGN_2026-10-16.md)。

## §1 勘定读数（S1 三件事逐项闭合）

1. **触发面勘定（D1，266 遗留「何者待勘」就此闭合）**：`session_archive` 生产汇点唯一
   （`archive_raw_session_package`），触发入口四道——① run 尾里程碑增量（`acp_server.rs:3593`
   spawn 块；判据 `incremental_archive_due`＝实际上下文 ≥500K ∧ ≥ 水位＋500K；`incremental=true`、
   不删源）② close 归档（`close_session`；生产调用点＝`codex_app.rs:622`＋`orz-signer`；
   web 桥不调用——ACP 无 session/close，web「新开对话」＝`acpSessionId` 置空后 `session/new`，
   旧会话就地闲置）③ 0ak 无头里程碑（`headless_session_archive`，ADR §14.68）④ 0cy 启动侧
   收卷（`session/new`＋orz-bin `run()` 起点；非存活∧非当前保留集排除存活会话）。
   **肇事链**：① 打包（run_finished 后 80ms）→ web `isArchived`（`widgets.js:250`：有包 ∧
   包 mtime ≥ 最新真实 run；ARC journal 排除在活动水位外）→ 会话入「归档会话」组 →
   用户被迫手动回档。④ 的旁证（e6508b76 被收卷）属 0cy 既定语义、非肇事。
2. **0cy 收窄口径（D5）**：**保留现状不改提示制**——④ 触发点挂用户显式动作链（新开对话／CLI
   新 run）＝主权相容；提示制违背 0cy 立项初衷；266 §5 明示 0cy 核心语义不受挑战；肇事面不在
   sweep。差量回写义务登记：0cy 设计稿 §2 表「500K 增量」行随 S2 加勘注（仅指无头道）。
3. **`ORZ_GSA_HYGIENE=0` 边界（D7，266 遗留「是否受其管辖待核」就此裁决）**：`hygiene_enabled()`
   只注入 sweep 两生产调用点；**不管辖**①②③与 on-demand——`=0` 只关「收卷删源闸」不关
   「自动打包闸」，用户以 `=0` 自救本不会生效（S2 后随①退役问题消失）。

## §2 交付物

- **设计档**：[`docs/SESSION_LIFECYCLE_USER_SOVEREIGNTY_DESIGN_2026-10-16.md`](../SESSION_LIFECYCLE_USER_SOVEREIGNTY_DESIGN_2026-10-16.md)
  ——§0 事实／§1 四道清单＋肇事链／§2 主裁决（删 spawn 块＋票字段 `incremental` 连带退役＋
  原语保留）／§3 联动面 D3–D6／§4 env 边界／§5 数据安全与残差（「不结束没存档」缺口交互车道
  重开＝用户主权显式接受）／§6 S2 触点清单（含三处 dated 档转录义务）／§7 钉子 N1–N5＋S3/S4
  判据／§9 D1–D8 浓缩表。
- **S2 钉子定案**：N1 run 尾零归档（≥500K 续作成功 run ⇒ 零 `session_archive`／零新包／水位
  不动）；N2 close 推迟票回归；N3 手动面恰等；N4 无头幂等不翻；N5 纯判定不翻。
  既有测试勘定：run 尾 spawn 块无集成测试（纯函数与无头道测试各 1 不受牵连）⇒ 只加零归档钉、
  无既有测试翻转。

## §3 边界

1. 零源码、零 API、零跑批；orz 子模块不动；web 前端零改动（D6 投影不动）。
2. 滑块设计稿 §3.5.1／0cy 设计稿 §2／ADR-0010 §14 三处转录**随 S2 落码批**同批落（S1 只登记）。
3. S3 字节判据改口径＝行为面（①退役无常量文案可作字节判据；N1 触发例 journal 断言离线可复验）
   ——TODO `P1-0dd` S3 行「字节判据随 S1 定」就此定为行为判据。
4. 未提交、未推送（提交随批尾落账；推送另行放行）。

## §4 关键词

275 批、0dd S1、会话生命周期用户主权、SESSION_LIFECYCLE_USER_SOVEREIGNTY_DESIGN_2026-10-16、
run 尾里程碑增量归档、handle_session_prompt spawn 块、incremental_archive_due、DP-11④ ACP 半边退役、
isArchived mtime 判定、widgets.js:250、close 推迟票 B3 P2-3、0ak 无头道不动、0cy 保留现状不改提示制、
ORZ_GSA_HYGIENE=0 只关收卷闸、D1–D8、N1–N5、计数 61 不变、索引 v4.248。
