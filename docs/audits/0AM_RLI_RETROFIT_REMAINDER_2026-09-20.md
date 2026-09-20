# 0am RLI 改造 · 剩余部分（配极改／判断标准改写／C3 分离／feature 序列面／FR4）落码与离线读数（2026-09-20）

> 本批 run：`RUN-CLI-6aafe5d8`（狗粮轮，orz 内自改；上批 `RUN-CLI-6aafdaa5`）。
> **状态：工作树未提交**——按用户令「不提交／推送／重建」。本批性质为研究性
> （可适当变动、可按需停止）；停点＝五项落码完成＋离线读数取毕＋本档落成。
> **读数①消费率／②转向相关仍需活体 run（0bc）**，不在本批范围。
>
> 入口：上批 [`0AM_RLI_RETROFIT_2026-09-20.md`](0AM_RLI_RETROFIT_2026-09-20.md)
> ／设计 [`RLI_RESONANT_CHANNEL_BASE_DESIGN_2026-09-16.md`](../RLI_RESONANT_CHANNEL_BASE_DESIGN_2026-09-16.md)
> §10.3／§11／历史档 [`0AM_DECODER_FORM_REPLAY_2026-09-20.md`](0AM_DECODER_FORM_REPLAY_2026-09-20.md)。

## 0. 本批口径（用户令摘要）

- 范围＝上批「改造四项」**剩余的补充项**：①**配极改**（按通道语义分配极点）／
  ②**判断标准改写**（`stuck` 语义按 v／E 锚点原生覆盖；撤销 RLI vs LIF 对照；
  一致性基准改「框架实际动作结果」）／③**C3 分离对照**（固定 ω_d 扫 ζ ×
  固定 σ 扫 ω_d 两列，拆衰减与频率）／④**PULL 面 feature 锚点序列面**／
  ⑤**FR4 压缩白名单并入 `context_compress`**（不做独立工具、保 10 工具面）。
- 不动面：1D 生产基座 · LIF `temporal`（维持现役组件）· env 门控
  （`ORZ_LIF_RLI_SHADOW`）· 侧车 schema `rli-shadow-v1` 兼容 · 工具面计数。

## 1. 落点清单（按文件）

| 文件 | 内容 |
| --- | --- |
| `orz/crates/orz-assurance/src/lif/rli.rs` | ① `RLI_PROG_ZETA=2.0`：**prog 走实极点**（λ±=−ω(2∓√3)：慢模态 τ≈3.73/ω＝持久面、快模态 τ≈0.27/ω＝适应面），速率／压力类（err/stall/slow/deny）保持复极点 ζ=[`RLI_ZETA`]；新增 `rli_zeta_for(kind)`（配极表＝语义常数表）。二阶解族**过阻尼分支**双曲闭式（`free_evolution_at`：u/v 用 cosh/sinh；包络 E=φ(|u|cosh+\|B′\|sinh)；`refresh_envelope` 实极点=φ\|u\|；μ 下限 1e−12 保 ζ→1 邻域稳定）；`set_poles(ω,ζ)` + `set_zeta` 钳制扩到 `[0,4]`。② `RLI_DOMAIN_ERR_ENVELOPE_FLOOR=1.0`；谓词改 `pressure = u_err≥2.0 ∨ (v_err>0 ∧ env_err≥1.0)`（设计 §3 a2/a3 原生覆盖 stuck 语义；**不补通道**）；`RliDomainRow` 以 `v_err/env_err` 取代 `lif_domain/consistent`（`serde(default)`）；`RliDomainMachine` 去 `compares/agrees`，`record_round(t,u_err,u_prog,v_err,env_err)`；**去 LIF 对照**。④ `RLI_FEATURE_SERIES_CAP=20` + `RliAnchor`/`RLI_ANCHOR_FEATURE_NAMES`/`RLI_ANCHOR_FEATURE_TABLE`（err/prog × u/v/pred/env/rhythm）+ `RliShadow.anchor_series`（决策轮采样、live-only）+ `feature(name,k)`/`known_feature_names()`；`on_decision_round(t,t_hat)` 去 LIF 参数。测试：新/改写 7 条（过阻尼 RK4 对拍+包络单调、set_poles 两分支、v/E 谓词、序列表对齐、序列 cap/live-only、域机 spike、快照续接）。 |
| `orz/crates/orz-assurance/src/lif/mod.rs` | 喂入点改 `shadow.on_decision_round(t, t_hat)`（不再传 `temporal.current_domain()`）；公开面增出配极/阈/序列表；引擎级钉改 `rli_domain_rows_record_native_anchors_per_decision_round`（两轴同轮刻度 + 原生锚点 + 序列表采样）。 |
| `orz/crates/orz-loop/src/controller.rs` | ③ `render_rli_section(selector,k,name)` 重写：now/recent 显示原生锚点（`u_err/v_err/E_err/u_prog`）、history 去域一致；**新增 feature 序列面**（名/缺名/非法名显式报错；k≤20；紧凑序列＋当前值＋自判域）。`blackboard_read` schema：selector 描述补 `rli: now\|recent\|history\|feature`；`name` enum 补 10 个 rli 锚点名。⑤ `context_compress` 声明新增可选 `whitelist`（`array<string>`，maxItems 16）＋描述句。 |
| `orz/crates/orz-loop/src/host_exec/tool_run.rs` | ③ `section=="rli"` 分支透传 `name`。⑤ `context_compress` 分支新增 FR4 处理：逐条 trim→累计 cap（`whitelist_cap`，默认 16K）→落内存列表→`archive_whitelist_entry`（`.gsa/whitelist.jsonl` best-effort）→`upsert_whitelist_message`（常驻前言区，压缩跳过）；响应追加「白名单：保存 N 条（累计 X/16K 字符…）」＋超限/空条目跳过注记；**fail-soft**（三态语义与 exit 0 信封不变）。 |
| `orz/crates/orz-loop/src/compact.rs` | ⑤ 新增端到端测试 `context_compress_whitelist_entries_land_and_respect_cap`（落盘/存档/常驻消息/超限跳过/空条目跳过；cap=12 小值）。封存的 `compaction_whitelist_add` 相关测试原样保留（5 条，全绿）。 |
| `orz/crates/orz-assurance/examples/rli_shadow_replay.rs` | ②③④ 整件再重写（v3）：`RoundRow` 增**同轮动作结果窗**（该决策轮后、下一决策轮前完成的工具结果；`window_negative/success/outcomes`）+ `pressure_claim/low_progress_claim/pressure_actual/progress_actual`；域一致性改**对框架实际动作结果**两轴核读（confusion `claim|actual`）；`PoleTransform{Baseline,ZetaAtFixedWd,WdAtFixedSigma,LegacyZetaZero}`（只作用复极点类通道）；C3 两列（衰减列 ζ∈{0,0.25,0.5,0.75,0.95} 固定 ω_d；频率列 ×{0.5,0.75,1.0,1.5,2.0} 固定 σ；各含恒等点自检）；旧 ζ=0 同 ω 口径降格为 **legacy_non_inertia**（参数非惰性检验）；report schema `rli-shadow-replay-v3`；`--selftest` 改两轴接线自检。 |
| `orz/crates/orz-host/src/permission.rs` | ⑤ 注释面：说明并入的 whitelist 写仍属「内存 + .gsa 机械 best-effort 存档」类，**不新增访问类型**（两表登记纪律不变；无代码分支改动）。 |

## 2. 验证记录（本机，2026-09-20）

```
cargo check -p orz-assurance --examples                ✓ 0.73s
cargo check -p orz-loop --all-targets                  ✓ 1.60s
cargo test  -p orz-assurance --lib                     ✓ 255 passed / 0 failed
cargo test  -p orz-assurance --lib lif::               ✓  55 passed / 0 failed
cargo test  -p orz-loop --lib --no-run -j 1            ✓ 40.9s（见 FR2）
  └ 直跑（清 4 个 ORZ_ACAF_* + RUST_MIN_STACK=16777216 + --test-threads=1）：
      context_compress:: 3 ✓  whitelist:: 7 ✓  rli:: 4 ✓  compact:: 8 ✓
      controller:: 40 ✓  agent_loop:: 42 ✓  tool_run:: 42 ✓  blackboard:: 50 ✓
cargo run -p orz-assurance --example rli_shadow_replay -- --selftest      ✓
cargo run -p orz-assurance --example rli_shadow_replay -- \
    D:\tb-eval\jobs-official D:\tb-eval\analysis\0am-rli-shadow-replay-v3-2026-09-20.json   ✓
```

## 3. 离线读数（语料 137 runs / 5889 决策轮；报告见 `D:\tb-eval\analysis\0am-rli-shadow-replay-v3-2026-09-20.json`）

### 3.1 域一致性（补充项②口径：RLI 自判域 vs **框架实际动作结果**）

同轮动作结果窗＝该决策轮之后、下一决策轮之前完成的工具结果；空窗不计。窗数 5846。

| 轴 | 一致率 | 主张数 | 实际数 | 混淆（claim\|actual） |
| --- | --- | --- | --- | --- |
| 压力面（Error\|Deny） | **95.57%**（5587/5846） | 26 | **265** | false\|false 5571 · **false\|true 249** · true\|false 10 · true\|true 16 |
| 进度面（Success） | **82.21%**（4806/5846） | 599 | 587 | false\|false 4733 · false\|true 514 · true\|false 526 · true\|true 73 |
| 两轴同时 | **80.60%**（4712/5846） | — | — | per-run joint：min 0.30 / **avg 0.815** / max 1.00 |

RLI 域分布：normal 5065 · low_progress 602 · start 196 · pressure 21 · stuck 5
（迁移 1049 次）。**对读**：上批（口径＝vs LIF 现域）一致率 66.2%、low_progress
主张率 38.2%；本批低进度主张 607/5889＝**10.3%**，实际无进度轮 587/5846＝**10.0%**
——配极改（prog 实极点，run 均 u 由 0.519 → **0.734**）与谓词改写把进度轴的
**标度差基本消掉**；剩余误差为**对称噪声**（514 漏报 / 526 误报）。

**压力轴严重欠报**（主张 26 vs 实际 265；249 轮漏报）：u_err≥2.0 阈高，且
v/E 原生覆盖项要求「正在恶化 ∧ 未收敛」同刻成立（瞬时 episodes 多不满足）。
如实登记为**保守偏置**，是否校准留待裁定（见 §4-2）。

### 3.2 C3 分离对照（补充项③；只扫复极点类通道）

| 列 | 设置 | mean\|Δu_err\| | mean\|ΔE_err\| |
| --- | --- | --- | --- |
| 衰减列（固定 ω_d） | ζ=0 | **0.2467** | **0.3655** |
| | ζ=0.25 | 0.0352 | 0.0538 |
| | ζ=0.5（恒等点） | **0** | **0** ✓ |
| | ζ=0.75 | 0.0157 | 0.0358 |
| | ζ=0.95 | 0.0404 | 0.0958 |
| 频率列（固定 σ=ζω） | ×0.5 | 0.0587 | 0.0456 |
| | ×0.75 | 0.0284 | 0.0138 |
| | ×1.0（恒等点） | **0** | **0** ✓ |
| | ×1.5 | 0.0432 | 0.0124 |
| | ×2.0 | 0.0631 | 0.0162 |
| **legacy（同 ω 的 ζ=0）** | 参数非惰性检验 | **0.2408** | **0.3454** |

读法：**衰减是主效应**（ζ=0 处 Δ≈0.25/0.37），频率效应小一个量级（0.03–0.06）；
legacy 读数（0.241/0.345，与上批 0.241/0.345 一致）**几乎全由衰减贡献**——其
「同 ω」并未固定频率不变（ζ→0 时 ω_d 从 0.866ω 抬到 ω），故降格为**参数非惰性
检验**，不再充当阻尼有效性证据。

### 3.3 C1 时间打乱（证伪门）与通道读数

- C1（10 置换/run）：mean\|Δu_err\| **0.0601**、mean\|Δu_prog\| **0.1758**、
  mean\|Δrhythm_err\| 0.0195；平均 9.99/10 个置换改变锚点 —— 锚点对时间结构敏感，
  C1 保持「非平凡」。
- 通道（run 均之均 / 末态 hits 合计）：err u 0.081 v −0.002 pred −0.004 E 0.122
  r 0.113 / 382；prog u **0.734** v −0.022 pred 0.090 E 0.734 r 1.593 / 681；
  stall 0.033/−0.002/−0.002/0.054/0.109 / 215；slow 0.062/−0.000/0.034/0.072/0.030
  / 221；deny 0.028/−0.001/−0.003/0.044/0.078 / 249。

## 4. 未完成 / 延后（如实登记）

1. **读数①消费率 / ②转向相关**：需 env 门控开启的活体 run（0bc）；采集口径＝
   journal 内 `blackboard_read{section:"rli"}` 的次数/时机 + 其后 N 轮的转向
   （工具选择/文案），与同 run 非读取段对照。本批只交付了「面可拉取」的工程面
   （now/recent/history/**feature**）。
2. **0bc 复核登记项**（本批读数指向）：①压力轴欠报（26 vs 265）——若裁定校准，
   候选口径＝把「窗内 Error\|Deny 计数」引入压力轴（注意那是**直读标签化**，需
   用户裁决）；②v/E 阈 1.0（单次激励单位）随 0bc 复核；③一致性基准「同轮后窗」
   的对齐边界（决策轮后到下一决策轮前）为本次注册口径。
3. **实极点 ζ→1 邻域退化**：包络上界在 ζ→1 时数值退化（μ 下限兜底）；诊断扫描
   应避让 `[1−δ,1+δ]`（本批 C3 扫描面 ∈[0,0.95]）。
4. **feature 序列 live-only**：不随侧车持久化（跨 prompt 序列重置）——如需跨
   prompt，需 schema 升级（登记未做）。
5. **索引／TODO／BACKLOG 未改**（按不提交/不推送令；登记留给用户裁定的下一批）。
6. 未做「一致性读数并入 temporal/rli 渲染头」（上批同一延后项）。

## 5. 框架摩擦登记（本批实测）

- **FR1（测试面 · dogfood env 继承；复现，未闭环）**：orz 会话内直跑 `orz-loop`
  测试二进制时继承 `ORZ_ACAF_FAIL_CLOSED=1`＋MANIFEST/KEYSTORE/BINARY ⇒ 12 个
  「起 run」类测试 panic：`ACAF fail-closed is enabled but no signer client is
  configured ... refusing to start the run`（whitelist×7、context_compress×2、
  rli 过滤面×1、delivery×1 等）。**绕行**：跑前 `Remove-Item env:ORZ_ACAF_*`（4 个）
  ⇒ 全绿。与上批 FR2 同因；**建议**：测试入口脚本化 unset 或测试内显式注入
  ACAF override（不污染生产语义）。
- **FR2（编译面 · LLVM OOM；F14 状态再更新）**：`cargo test -p orz-loop --lib
  --no-run`（默认并行）→ `rustc-LLVM ERROR: out of memory / Allocation failed`
  （exit 0xc0000409）；`-j 1` 成功（40.9s）。**F14 状态＝条件性可构建**，建议
  `-j 1`（上批记 `-j 2`；本批未复测 -j 2）。
- **FR3（上下文面 · 压缩窗口吃动作预算）**：批中段模型面越 320K 硬提醒 ⇒ 机械层
  开窗，「窗口内仅 blackboard_write / context_compress 可执行」，**1 个工具动作
  被跳过**（回执行：本轮其余动作已跳过（1 个）），需产出语义摘要块后放行。关键
  结论已固化黑板 notes×2 规避回放成本；登记为长批次常规摩擦。
- **FR4（编辑面 · 记忆文本漂移）**：两次 `search_replace` 因「记忆中的文档注释
  文本」与文件实际不符而 `not found`（controller 测试 doc、`mod.rs` 导出行），
  新鲜读取后成功。工具已给 nearest-match 行，定位成本可接受；提示：跨批次改文件
  **先读再改**（本批的既定纪律，本次为漏读所致的自我摩擦）。
- **FR5（schema 面 · `name` 参数双面共享单 enum）**：`blackboard_read` 的 `name`
  是 temporal／rli 双面共享的单一 enum（本批补到 14 项），而实际校验在渲染侧
  （`render_*_section`）——**参数面与校验面双源**，新增通道面必须人工同步两处
  （漏同步时 enum 只影响提示性，不影响正确性）。
- **FR6（持久化面 · 配极不进快照）**：通道 ζ 是代码拥有量（旧契约，快照只带
  ω+动力学状态）。配极改后「家族/配极」**完全由代码版本决定**，诊断变换（C3）
  不能随侧车续接——研究批内可接受，登记为已知边界（若未来要持久化配极，需
  schema 升级）。
- **FR7（兼容面 · 旧行 v/E=0 真伪不分）**：`RliDomainRow` 新字段以
  `serde(default)` 落地 ⇒ 旧侧车行反序列化为 `0.0`，与「真实 0」不可区分
  （无法从行本身判断锚点是否可得）。如需区分应改 `Option`（登记未做）。
- **FR8（观测面 · C3 扫描面收窄）**：prog（实极点）不在 C3 扫描面——读数不得被
  读成「prog 阻尼无效」，而是「**未测**」（语义分配不在分离实验面）。

## 6. 复现命令

```powershell
cd D:\CLI\orz

# 编译面
cargo check -p orz-assurance --examples
cargo check -p orz-loop --all-targets

# 单元测试（assurance）
cargo test -p orz-assurance --lib lif::

# orz-loop 面（条件性可构建，见 FR2；运行前清 ACAF env，见 FR1）
cargo test -p orz-loop --lib --no-run -j 1
Remove-Item env:ORZ_ACAF_*          # 4 个变量
$env:RUST_MIN_STACK="16777216"
cargo test -p orz-loop --lib -- --test-threads=1 context_compress
cargo test -p orz-loop --lib -- --test-threads=1 whitelist
cargo test -p orz-loop --lib -- --test-threads=1 rli

# 重写件自检 + 语料回放（离线读数）
cargo run -p orz-assurance --example rli_shadow_replay -- --selftest
cargo run -p orz-assurance --example rli_shadow_replay -- `
    D:\tb-eval\jobs-official D:\tb-eval\analysis\0am-rli-shadow-replay-v3-2026-09-20.json
```

## 7. 建议的下一批（供裁定）

1. **0bc 活体读数**：载体重建后开 `ORZ_LIF_RLI_SHADOW`，取读数①②（消费率／转向
   相关），并复核 §4-2 的登记项（压力轴保守偏置、v/E 阈、后窗口径）。
2. **登记批**：把本批五项同步索引 v4.11→v4.12／TODO `P1-0am` 残余项／BACKLOG
   （含 FR1/FR2 的环境绕行写入 dev 文档或测试脚本）。
3. **（可选）压力轴校准**：若裁定引入「窗内 Error\|Deny 计数」直读，需明确其与
   「模型自判」的性质边界（直读＝观测，不是模型判据）。
4. **（可选）RLI vs LIF 并轨裁定**：本批已把 RLI 从「与 LIF 对照」改为「对框架
   实际动作结果」，LIF `temporal` 维持现役；若后续要退回单轨，需另裁。

## 8. 主会话追补（2026-09-20）：FR-8 处置与裁决登记

**FR-8 已处置（主会话直接落码，随提交批）**：`examples/rli_shadow_replay.rs` 的 C3 增**实极点列**（`PoleTransform::RealAtFixedOmega`）——`prog` 落在过阻尼分支、ω_d／σ 在该分支无定义，故单列「固定 ω 只扫 ζ>1」；`mean_anchor_deltas` 按通道参数化（复极点两列取 `Err`、实极点列取 `Prog`），三列覆盖两类通道，**无通道落在扫描面之外**。读数落 `D:\tb-eval\analysis\0am-rli-shadow-replay-v3b-fr8-2026-09-20.json`（**不覆盖**本批 v3 原始读数）；`cargo fmt` 净、示例重编译通过、`--selftest` 绿。

**FR-8 读数（137 runs 均值）**：

| 列 | 通道 | 扫描点 | 均值 \\|Δu\\| | 均值 \\|ΔE\\| |
|---|---|---|---|---|
| 衰减列（固定 ω_d） | err | ζ=0.0 | 0.2467 | 0.3655 |
| 衰减列（固定 ω_d） | err | ζ=0.95 | 0.0404 | — |
| 频率列（固定 σ） | err | ×0.5／×2.0 | 0.0587／0.0631 | — |
| **实极点列（固定 ω）** | **prog** | ζ=1.5 | 0.0385 | 0.0385 |
| **实极点列（固定 ω）** | **prog** | ζ=3.0／ζ=4.0 | 0.0464／0.0736 | 0.0736 |

恒等点三处（衰减列 ζ=0.5／频率列 ×1.0／实极点列 ζ=2.0）**全为 0** ⇒ 变换接线自检通过。判读：衰减项在 `err` 上仍是主效应（频率列小一个量级）；**`prog` 对自身配极的敏感度低（0.04–0.07）**，与其「水平稳定」的目标一致——原骑阈问题出在水平与阈的关系，不是 ζ 敏感度。

**观察（待复核，非结论）**：实极点列中 `prog` 的 ΔE 与 Δu **逐位相等**（0.0385／0.0385、0.0736／0.0736）⇒ 过阻尼分支下双曲包络式与状态式逐项同号（u₀ ≥ 0 且 v₀+ζω·u₀ ≥ 0 时 `E ≡ u`），即 **a3 包络锚点在 real-pole 通道上不独立**。列为观察项，供 0bc 长杂轮或后续设计复议；本批不作机制判决。

**本轮裁决登记（用户 2026-09-20）**：

- **FR-3（硬提醒层锁工具面）＝需处理**：硬提醒**仅是打断式提醒、不锁工具面**；本批实测的「窗口内仅 `blackboard_write`／`context_compress`」属不当实现。
- **FR-4（记忆文本漂移）＝不处理**：属先读后改机制的正确实现。
- **FR-5（`name` enum 参数面／校验面双源）／FR-6（配极不进快照）／FR-7（旧侧车行 v/E=0 真伪不分）＝并入 0bc 长杂轮**。
- **FR-1（ACAF env 继承致测试拒跑）** 与既有 0bc 杂项同项；**FR-2（默认并行 LLVM OOM、`-j 1` 方可构建）** 归 F14 状态更新（本轮再订正）。
- **0bc 定位声明**：0bc **主要仍是完成任务**；RLI 读数参考部分＝**任务完成后再看实际运行中 RLI 的状况**。
