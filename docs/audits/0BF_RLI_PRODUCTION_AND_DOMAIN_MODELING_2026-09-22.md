# 0bf 报告：RLI 生产化与域建模试验（2026-09-22）

> 范围：RLI 四项改造——① **常开**（kill switch；LIF 回退面保留）；② **预测转域
> 建模**（域转移倾向＝特征面；只给特征与域状态）；③ **繁杂度与提醒口径校准**（模型
> 面两触发＋失配概率随报；用户面＝水位×繁杂度**复合总值**）；④ **开销读数与采样补
> 点**（10 s 网格；计数面）——机制落码、测试钉子、离线读数、判据对账与摩擦登记。
> 纪律：本批**不提交／不推送／不重建载体**（用户令原文）；**实验性自主轮**——过程
> 中可自行调整、可自行停止（用户授权）。本会话 `RUN-CLI-6ab15969` 即本批观测轮
> （env `ORZ_LIF_RLI_SHADOW=1` 由平台预设）。

---

## 1. 立项背景与判据

立项：用户令「RLI 转正替换 LIF 按两步走执行（先常开承接时间轴／域定位，预测与繁杂
度留参考面）；预测转域建模；繁杂度与水路疲劳加权合成总值＋持续性判据」。硬边界：
**禁动作建议、禁外挂决策模型**——模型用 RLI 特征自主判断（特征外报、决策归模型）。
落档：TODO `P1-0bf` / BACKLOG `0bf` / 来源＝0be 报告 §7。

四项判据（S1 预注册，2026-09-21 用户裁决，落定生效）：
1. **常开**＝`ORZ_LIF_RLI_SHADOW` 语义反转（缺省 ON；显式 `0/off/false/no` 关＝kill
   switch，LIF 回退面照常）；LIF 退役与否留待转正后读数；
2. **域转移倾向**＝本会话 spike 历史**纯经验统计**（完成段数／累计轮／去向分布／逐
   轮离开率／中位驻留；零拟合；只给特征与域状态；legacy 缺轮号如实跳过——FR-7）；
3. **提醒**：模型面**两触发**（域迁移确认＝新域稳定 ≥3 轮后一次性；持续越线＝
   `err/stall/slow/deny` 四压力通道 `u≥θ` 连续 **k=5** 采样点；`prog` 不参与）＋
   **失配概率随报**（近 ≤16 个 ρ 样本中 ρ>1 占比；只记录不反馈）；用户面＝
   `min(100, 水位% + 0.3×cplx%)`，档 **50/70/90** 每档一次（侧车单键
   `burden_tiers_notified`，替代 0be 两键）；
4. **采样与开销**：10 s 网格补点（空档 >10 s 按 10 s 网格**结算采样点**；不
   `check()`、不改 θ/fires/T̂ 语义；>512 点＝跨会话空闲整体跳过）＋计数面
   （`grid_samples`/`sample_points`）与渲染 ≤1 KiB 核证。

---

## 2. S1 设计决定（本批自行裁决，落黑板 plan；此处收口版）

### 2.1 常开关与开关语义
- `rli_shadow_enabled_override()`：`trim + lowercase` 后**只有**显式
  `0/off/false/no` 关；其余（含未设）＝开——缺省常开是本批的语义反转（0am–0be
  为缺省关）。
- 引擎侧等效钩 `LifEngine::disable_rli_shadow()`（kill switch 回退＋测试面）。
- 渲染面：off 文案改为「RLI 未启用（kill switch…LIF 回退面照常）」；面名保持 RLI。

### 2.2 域转移倾向（条件统计口径）
- `RliTransitionTendency{domain, current_dwell, completed_episodes,
  completed_rounds, directions, per_round_rate, median_dwell}`：以**当前域**为条件
  的**已完段**统计（「这个域通常驻留多久、离开后去哪」）；legacy spike 缺轮号＝该
  段跳过（不落假 0）。
- 只给特征：无动作建议、无外挂决策模型（硬边界照旧）；读数进 `rli.now` 行。

### 2.3 提醒两触发与投递
- 域迁移**确认**：新域稳定 ≥ `RLI_MIGRATION_SETTLE_ROUNDS=3` 轮（含切换轮）后报一
  次——震荡不报；与 0am 的迁移 spike 面并存（双面过渡，见 §6-c）。
- 持续越线：恰在第 k=5 个连续越线采样点触发一次（触发后继续计数不重复；归零后再
  达 k 可再触发）；文案含 `持续越线: err×5（u=…≥θ=…；失配概率 …）`。
- 投递：**pull-delta 头**（仿 `域迁移+n`）：挂头 ≤2 条、256 B 预算内逐条校验（不截
  半条）；挂上即标记已投递（游标锁释放后单独取 lif 锁，不倒锁序）；`rli.now` 面照
  旧可回看（近提醒 ≤2 条）。
- 失配概率：`RliComplexity::mismatch_rate()`（各通道近 ≤16 ρ 合并、ρ>1 占比；未就绪
  「—」）；**只记录不反馈**（不惩罚、不改内部状态——用户令）。

### 2.4 用户面复合总值与网格补点
- `burden = min(100, fatigue_percent + 0.3 × cplx_percent)`；`cplx_percent` 按
  θ85/95/99 锚 60/80/100 分段线性（c≤1→0；未就绪→0＝退化为纯水位——legacy 语义完
  全兼容）；单次只投最高未投递档、跳跃联档；SUCCESS-ONLY（失败 run 不更新档位）。
- 网格：`fill_grid_gaps(t)` 在每次采样前结算（决策轮／工具事件两入口），网格点只做
  `advance` ＋采样结算（含越线观察）——**不** `check()`、**不**改 θ/fires/T̂ 间隔样
  本（0bf ④ 的「不改语义」承诺）；超长空档（>512 点）＝跨会话空闲，整体跳过。

---

## 3. S2 落码清单与测试钉子

### 3.1 落码清单

| 位置 | 内容 |
|---|---|
| `orz-assurance/src/lif/rli.rs` | 常量 6 个（`RLI_GRID_SECS=10`／`RLI_GRID_FILL_MAX=512`／`RLI_STREAK_K=5`／`RLI_MIGRATION_SETTLE_ROUNDS=3`／`RLI_NOTICE_CAP=16`／`RLI_MISMATCH_WINDOW=16`）；通道 `streak`＋`observe_streak()`；`RliNoticeKind`／`RliNotice`／`RliPendingMigration`／`RliMigrationConfirmed`／`RliTransitionTendency`；影子 `note_sample_point()`／`fill_grid_gaps()`／`push_notice()`／`mismatch_probability_text()`／投递与计数访问器；域机 pending/confirmed 与确认钩、`transition_tendency()`；`RliComplexity::mismatch_rate()`；快照扩展（通道 streak／域 pending/confirmed／影子四字段，restore cap 与有限性过滤） |
| `orz-assurance/src/lif/mod.rs` | `LifEngine::disable_rli_shadow()` |
| `orz-loop/src/controller.rs` | kill switch 判定（缺省开）；off 文案；`rli.now` 头（`[RLI on \| 步数 \| 采样（网格补点）\| T̂ \| 自判域]`）＋转移倾向行＋近提醒行；`attach_pull_delta` 一次性投递（≤2 条、预算内、游标锁外标记已投递） |
| `orz-loop/src/complexity.rs` | 0bf 复合口径改写（`BURDEN_CPLX_WEIGHT=0.3`；私有锚 60/80/100；档 50/70/90；`cplx_percent`／`burden_total`／`pending_burden_notice`；旧 `pending_complexity_notice` 链退役） |
| `orz-host/src/acp_server.rs` | 侧车单键 `burden_tiers_notified`（替代两旧键）；成功路径＝水位＋繁杂度→复合判定→单条 `user_notice`（不进模型上下文） |

### 3.2 测试钉子（新增 6 项；口径摘录）

- `grid_fill_counts_points_and_skips_oversized_gaps`：≤10 s 空档不补；100 s 空档→补
  9 点（`sample_points` 照增）；超长空档（>512 点）整体跳过（不计数）。
- `streak_crossing_pushes_single_notice_and_marks_delivered`：连续错误事件（7 个余
  量——θ 随命中自适应上抬）→恰第 5 个连续越线采样点产一条；投递标记可消费且不重
  复。
- `migration_confirmation_and_transition_tendency`：pressure 稳定 3 轮→一条「域迁移
  确认…稳定 3 轮」；倾向＝当前域已完段条件统计（首段 0；`normal` 段后再入
  `pressure`→ ≥1、去向含 `normal`、中位/离开率可就绪）。
- `notices_grid_and_anchor_ride_the_shadow_snapshot`：提醒队列／采样锚／两计数随快
  照往返；legacy（新字段清空）＝空/0。
- `rli_reference_face_renders_shadow_signal`（改造）：先 `disable_rli_shadow()`→off
  文案；enable 后 `now` 面全量断言（分档 horizon＋p1＋λ̂＋繁杂度行），且不含「LIF」。
- `rli_notices_ride_pull_delta_header_once`：头一次性投递、不重发；kill switch 后零
  投递。

### 3.3 编译与测试核

- `cargo check -p orz-assurance -p orz-loop -p orz-host`：全绿（过程中修 2 处编译错：
  `complexity.rs` 比较引用层级、`acp_server.rs` 残留旧字段）。
- `orz-assurance --lib`：**272 全绿**（其中 `lif::rli` **37**）；`orz-loop --lib`：
  **830 全绿**；`orz-host --lib`：316 绿 / 6 红（见 §6-a，环境耦合；与 0bf 路径无
  关）。
- 测试入口纪律：跑 loop 全量须清 `ORZ_ACAF_*` 会话 env（否则「起 run」类测试因
  ACAF fail-closed 无 signer 假红——0bc FR2 同源）；官方入口
  `scripts/run_orz_tests.ps1`。

---

## 4. S3 读数

### 4.1 离线对拍（探针，0bf 口径）

- 探针 `D:\tb-eval\rli-forecast-probe` 对工作区新 API 重编（path 依赖）后运行：

  ```powershell
  cd D:\tb-eval\rli-forecast-probe
  $env:RLI_PROBE_OUT="D:\tb-eval\analysis\rli-forecast-contrast-0bf.json"
  cargo run --release -- D:\CLI\.gsa\runs
  ```

- 产物：`D:\tb-eval\analysis\rli-forecast-contrast-0bf.json`（逐 run sessions[]＋逐
  通道×horizon 对照）；stdout 摘录（n≈4367 决策行/通道）：

| 通道 | h=1 | h=2 | h=5 | h=10 | h=30 |
|---|---|---|---|---|---|
| Err skill（vs 持久性） | +0.240 | +0.350 | +0.505 | **+0.623** | +0.542 |
| Deny skill | +0.311 | +0.408 | +0.551 | **+0.638** | +0.571 |
| Stall skill | +0.303 | +0.369 | +0.326 | +0.213 | +0.089 |
| Slow skill | +0.222 | +0.295 | +0.317 | +0.247 | **−0.005** |
| Prog skill | +0.144 | +0.105 | +0.049 | +0.050 | +0.085（阈值命中率 **1.000**）|

→ 与 0be 收口口径一致（0bf **未改**预测／繁杂度内核；Err/Deny 高 horizon 峰、
Slow h=30 近零、Prog 全档转正后小幅正——如实呈现）。回放含本 run
（`RUN-CLI-6ab15969`，实时 journal；每轮 run 内前向匹配口径同 0be）。

### 4.2 会话级读数（沿用 0be，本批不重测）

λ̂/c/θ/锁存属预测与繁杂度内核（0bf 未改），沿用 0be 本 run 读数：λ̂(prog)=0.1106、
c=1.4007、θ=[1.536, 1.702, 1.659]、latched=[true,true,true]、合成样本 94、T̂ 均值
5.10 s、决策轮 204。

### 4.3 0bf 新增面读数（口径与证据）

0bf 四项为**机制/口径面**（开关、特征行、提醒投递、网格），无预测型指标；读数以
**钉子核读＋代码路径核**为主：

| 项 | 证据 | 读数/结论 |
|---|---|---|
| ① 常开（kill switch） | `rli_reference_face_renders_shadow_signal`（off 文案＋enable 面）＋`rli_notices_ride_pull_delta_header_once`（disable 零投递） | 语义反转可用；LIF 回退面照常 |
| ② 转移倾向 | `migration_confirmation_and_transition_tendency`（条件统计：首段 0→normal 段后 ≥1、去向含 normal） | 特征面只读、可用 |
| ③ 两触发＋复合投递 | `streak_crossing…`（触发沿恰一次、投递标记）＋`migration…`（稳定 3 轮确认）＋`rli_notices_ride…`（头一次性）＋complexity 7 测 | 触发/去重/投递路径全绿 |
| ④ 网格补点 | `grid_fill_counts_points_and_skips_oversized_gaps`（补点计数、超长跳过） | 语义不扰动核（不 check）成立 |

### 4.4 覆盖缺口（如实）

- 本批**未新开真机狗粮轮**（本会话即轮，实验性授权允许自停）；网格/提醒计数的
  **真机运行值**未单独抽取——以钉子核读＋代码路径核背书（转正后首轮读数补）。
- kill switch 的「缺省缺 env」真机路径未走（本会话 env `ORZ_LIF_RLI_SHADOW=1` 由
  平台预设）——仅测试覆盖 off/enable 两面。
- 「压缩窗口内取值开销」口径未落实测（以 `sample_points`/`grid_samples` 计数面代
  替），留待转正批。

---

## 5. 判据对账

| 判据 | 目标 | 证据 | 结论 |
|---|---|---|---|
| ① 常开＋LIF 回退 | 缺省 ON、显式关＝kill switch；回退面不退化 | `rli_shadow_enabled_override()` 反转；off 文案；`disable_rli_shadow()`；两钉子 | **达成** |
| ② 预测转域建模 | 只给特征与域状态；零拟合、禁动作建议 | `RliTransitionTendency` 条件统计＋钉子；render 行（转移倾向） | **达成** |
| ③ 提醒口径校准 | 两触发＋失配随报；用户面复合总值每档一次；不注入模型 | 实现＋3 钉子＋complexity 7 测；`user_notice` 单条；SUCCESS-ONLY 侧车键 | **达成** |
| ④ 开销读数＋补点 | 10 s 网格不改语义；计数可核；面 ≤1 KiB | `fill_grid_gaps`（不 check）＋钉子；头行计数；渲染测试（≤1024） | **达成** |
| 纪律 | 不提交/不推送/不重建；报告落 doc | 工作树未提交、未推送；载体未动；本档 | **达成** |

---

## 6. 摩擦登记

**a. 测试环境耦合（本机会话 env 预设）**：`ORZ_ACAF_*`（fail-closed 无 signer）令
`delivery::submit…` 假红（清 env 后过）；`GROK_HOME` 已设令 `grok_home::tests` ×4
红；`call_tool_timeout_kills_process_tree` / `run_tests_timeout_kills_process_tree`
并行 flaky（单跑过）。host 316 绿 / 6 红均与 0bf 路径无关。建议：入口脚本继续
unset（`scripts/run_orz_tests.ps1`），或测试内显式 override（0bc FR2 主项）。

**b. 侧车键迁移过渡**：0be 的 `fatigue_tiers_notified`/`complexity_tiers_notified`
两键 → 0bf 单键 `burden_tiers_notified`；旧侧车缺该键 = 空（serde default），
**legacy 会话可能重发一次档位提醒**——如实登记，不做迁移填充（FR-7）。

**c. 双迁移通知并存（过渡态）**：LIF 侧 `域迁移+n` 头（temporal.migration_count）
与 RLI 迁移**确认**提醒并存——前者即时、后者稳定 3 轮后；语义不同但同向，留待转
正批按读数裁决是否去一（本批不动 LIF 面）。

**d. 网格补点对既有测试期望的漂移面＝零**：`lif::rli` 33→37 全过、loop 全量 830
全过——补点路径（`advance`＋结算、不 `check`）未触碰既有断言。

**e. 预注册初值待标定**：复合权重 0.3、cplx 锚 60/80/100、k=5、稳定 3 轮——均
S1 预注册值，未做在线标定；转正后首轮读数再调（实验性轮的既有纪律）。

**f. 探针在外部路径**：`D:\tb-eval\rli-forecast-probe` 非仓库件，工作区 API 变更
后须重编（path 依赖）；本批已重编跑通（§4.1）。建议收编为 example（0be §6-4 已
登记，仍未做）。

**g. 压缩窗口取值开销未测**：0bf ④ 的「开销读数」以计数面（`sample_points`/
`grid_samples`＋渲染 ≤1 KiB）承担；压缩窗口内的取值开销未落实测（口径待定）。

**h. 题面编码（运行方观测，2026-09-22；用户令并入 0bd ⑭）**：狗粮启动器在 Windows
PowerShell 5.1 下以 **ANSI（本机 GB2312）** 读取题面文件，**无 BOM 的 UTF-8 题面被读成
乱码**后送进载体——回查 0be 轮 journal：`run_started.payload.prompt` 实为
`璇峰厛鏌ョ湅CLI_PROJECT_INDEX.md…`（该轮仍靠上下文猜对意图，属侥幸）；本批改用
**带 BOM 的 UTF-8** 题面，回查 `RUN-CLI-6ab15969` 的 `prompt` 字段确认**整段正确**。
处置面＝读取端显式 UTF-8（或写入端强制 BOM），属机械层修复、不涉模型命令改写。

---

## 7. 结论

- 四项机制（常开 kill switch／域转移倾向／两触发提醒＋复合总值／10 s 网格）全部
  落地；`cargo check` 三 crate 全绿；assurance 272、loop 830 全过（新增钉子 6
  项）；离线探针重编跑通并给出可核读数（§4.1）。
- 与硬边界一致：特征只读外报、无动作建议、无外挂决策模型；模型面提醒一次性投递
  不注入、用户面复合总值不进模型上下文。
- 遗留：真机长会话轮（网格/提醒计数的运行值）；kill switch 缺省路径真机验证；
  「压缩窗口取值开销」口径；双迁移通知去留；权重/锚标定；探针收编。均登记于
  §4.4/§6，留待转正批或后续轮。

---

## 8. 提交批

**不适用**——本批用户令「不必进行提交/推送/重建，按照项目惯例落一份报告文档即
可」；工作区改动留在本地（未提交、未推送、载体未动）。

---

## 9. 2026-09-22 用户裁决与追加登记（报告落盘后）

- **裁决：「0bf 转为部分达成吧」**——本档 §3／§7 的落地结论**不变**（S1／S2 已落地、
  钉子与读数齐备），但**本项不计入已闭合**：未闭合计数自轮内自记的 45 **回到 46**；
  S3（真机长会话轮）与 S4（真正收束：ADR 转录与计数同步随提交批）**仍挂**，§4.4
  三项缺口照旧。台账同步见 BACKLOG／TODO 的 0bf 节。
- **新增摩擦 h（用户令「可立项，请直接并进 0bd 中」）**：内容见 §6 h，已并入
  BACKLOG／TODO 的 **0bd ⑭**（与 ⑧ 同文件同批）。

---

## 10. 提交批（2026-09-22 用户放行）

| 项 | 结果 |
|---|---|
| orz 提交 | **`560e4387`**（0bf 五件：`lif/rli.rs`／`lif/mod.rs`／`host/acp_server.rs`／`loop/controller.rs`／`loop/complexity.rs`），推 `cli`（`6f23bbbf..560e4387`） |
| 父仓提交 | 本档（新增）＋ 台账第二卷 [`BACKLOG_AND_PRIORITIES_2.md`](../BACKLOG_AND_PRIORITIES_2.md)（新增）＋ 0bf 改判「部分达成」／0bd ⑭ 并项 ＋ 子模块 pin ＋ `orz_source_manifest.sha256` 重算，推 `origin main` |
| 载体 | **不重建**——在役 0.6.7 仍为 `6f23bbbf` 冻结源；本批源码**尚未进载体**（下次重建才带上） |
| 预检 | **免**（默认口径：纯逻辑面且不换装载体） |
| 门禁 | 提交前唯一 `error_count=1`＝「orz submodule working tree is dirty」（预期态）；提交后 `valid: true` |

**§8 与本节的边界**：§8「不适用」是**轮内**结论（任务令原文「不必进行提交/推送/重建」）；
提交与推送由用户 2026-09-22 **单独放行**，故不构成 §8 的破例——「载体未重建」一条两节一致。

---

关键词：0bf、RLI 生产化、域建模、域转移倾向、kill switch、常开、持续越线、
域迁移确认、失配概率、复合总值、`burden_tiers_notified`、10 s 网格补点、
`sample_points`、实验性自主轮、不提交不推送不重建。
