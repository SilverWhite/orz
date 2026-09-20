# 0am RLI 改造四项 · 落码与离线读数（2026-09-20）

> 本批 run：`RUN-CLI-6aafdaa5`（狗粮轮，orz 内自改）。**状态：工作树未提交**——
> 按用户令「不必提交／推送／重建」。本批性质为研究性改造（可变动、可按需停止），
> 停点＝四项落码完成＋离线读数取毕；**读数①消费率 / ②转向相关**需载体重建后的
> 活体 run（0bc），不在本批范围。
>
> 入口：TODO `P1-0am`（S2 口径）／BACKLOG `0am`／索引 v4.09（四项定稿）／
> 历史档 [`0AM_DECODER_FORM_REPLAY_2026-09-20.md`](0AM_DECODER_FORM_REPLAY_2026-09-20.md)
> ／基座设计 [`RLI_RESONANT_CHANNEL_BASE_DESIGN_2026-09-16.md`](../RLI_RESONANT_CHANNEL_BASE_DESIGN_2026-09-16.md)。

## 0. 本批口径（用户令摘要）

- 判据＝观测**实际可用性**，三条零标签读数：①消费率（模型是否真的拉取）／
  ②转向相关／③**域一致性**（RLI 自判动作域与框架实际动作结果同面一致）。
- 改造四项：①**域判定接上**（LIF 域状态机职能平移到 RLI）／②**预测步长改 10**／
  ③**零注入 PULL 参考面**／④**评估件重写**（标签回路与 Q1′–Q3′ 判据整体退役，
  历史读数保留）。
- 不动面：核（闭式解／五通道／分位自校准）· env 门控 · 侧车持久化 · 生产 1D；
  「研究性」授权＝允许适当变动与停点。

## 1. 落点清单（按文件）

| 文件 | 内容 |
| --- | --- |
| `orz/crates/orz-assurance/src/lif/rli.rs` | ② `RLI_PREDICTION_STEPS=10.0`；`RliChannel::prediction()` 改为闭式自由演化单点求值（`free_evolution_at()` 与 `advance()` 共用同一 φ/C/S 公式，返回 `(u₁,v₁,E₁)`），退役一阶短视 `u+v·T̂`；T̂ 非法时保守落回 `u`。① 新增 `RliDomainMachine` / `RliDomainRow` / `RliDomainSnapshot`（谓词与 `temporal.rs` 同形：`has_success` 门 + `low_progress(u_prog<0.5)` × `pressure(u_err≥2.0)` 四象；**`u_stuck` 轴随 v1 stuck 通道延后＝err 水平单轴**，显式登记、不拟合补轴）；`RliShadow` 增 `domain` 字段与 `on_decision_round(t, t_hat, lif_domain)` 同轮对照、`on_tool_event` 喂成功门；快照扩展 `domain` 字段（`serde(default)`，schema 仍 `rli-shadow-v1`，旧侧车＝fresh 域机器）。新增测试 4 个（闭式=advance@10·T̂、谓词四象、对照计数+spike、快照续接+legacy）。 |
| `orz/crates/orz-assurance/src/lif/mod.rs` | 喂入点：引擎逐决策轮把 `temporal.current_domain()` 传入影子（对照面）；公开面增出 `RLI_PREDICTION_STEPS` / `RliDomainMachine` / `RliDomainRow` / `RliDomainSnapshot` / `RLI_DOMAIN_RECENT_CAP`；新增引擎级钉 `rli_domain_consistency_recorded_per_decision_round`。 |
| `orz/crates/orz-loop/src/controller.rs` | ③ `render_rli_section(selector,k)`（now｜recent｜history；影子未启用＝中性说明；≤1 KiB UTF-8 安全截断）；blackboard_read 工具 schema：section enum + prose 增 `rli`，selector/k 描述补 rli 口径；`attach_pull_delta` 影子启用时挂 `rli+N` 徽章（读 rli 推进游标；未启用不挂=零成本）。新增测试 2 个（面渲染+边界+未启用；徽章/游标）。 |
| `orz/crates/orz-loop/src/host_exec/tool_run.rs` | ③ 新增 `section=="rli"` 分发分支（与 temporal 同形：epoch／receipt_id 组合显式报错、selector 透传、k∈1..=20 校验、错误装配走 `ToolCompleted{exit_code:1}`）；无效 section 文案补 rli；整响应 ≤1 KiB（`enforce_bound(1024)` + structured `board_cap` 1024）。 |
| `orz/crates/orz-loop/src/blackboard.rs` | 测试面：section enum 断言增 `rli`（与 temporal 同格）。 |
| `orz/crates/orz-assurance/examples/rli_shadow_replay.rs` | ④ **整件重写**（565 行，zero-label 观测件）：删标签回路／岭回归解码器／Q1′–Q3′；保留 C1（10 置换）与 C3（ζ=0）锚点读数＋通道级读数；新增**域一致性离线读数**（逐轮 RLI 域 vs LIF 域、一致率、混淆、Start 轴错位、迁移对照）；`--selftest` 接线自检；`retired` 段留历史读数指针。 |

## 2. 验证记录（本机，2026-09-20）

```
cargo check -p orz-assurance --lib                      ✓ 4.15s
cargo check -p orz-loop --lib                           ✓ 1m09s
cargo test  -p orz-assurance --lib lif::                ✓ 51 passed / 0 failed
cargo test  -p orz-loop --lib --no-run -j 2             ✓ 2m40s（见 §5-FR3）
  └ 直跑 exe + RUST_MIN_STACK=67108864 + --test-threads=1（清 4 个 ORZ_ACAF_* 后）
      controller::tests::  40 passed / 0 failed
      blackboard::tests::  43 passed / 0 failed
cargo run -p orz-assurance --example rli_shadow_replay -- --selftest   ✓
cargo run -p orz-assurance --example rli_shadow_replay -- \
      D:\tb-eval\jobs-official D:\tb-eval\analysis\0am-rli-domain-consistency-2026-09-20.json   ✓ 5.9s
cargo fmt -p orz-assurance -p orz-loop                  ✓
```

（首轮直跑 orz-loop 测试出现 19+30 个失败，全部为 `ORZ_ACAF_FAIL_CLOSED=1`
继承所致的 fail-closed 拒跑，非本批改动——见 FR2。）

## 3. 离线读数（读数③ 的离线面；语料 137 journals / 5889 决策轮）

**总表**（`D:\tb-eval\analysis\0am-rli-domain-consistency-2026-09-20.json`）：

| 量 | 值 |
| --- | --- |
| 对照轮数 / 一致轮数 | 5889 / 3899 = **66.2%** |
| per-run 一致率（137 run） | min 0.43 / **avg 0.733** / max 1.00 |
| Start 轴错位（RLI=start 而 LIF≠start） | **0** |
| 域迁移数 | RLI **2528** vs LIF **629**（≈4×） |

**域分布**：RLI＝normal 3421 / low_progress 2251 / start 196 / stuck 12 / pressure 9；
LIF＝normal 5339 / low_progress 305 / start 196 / pressure 48 / stuck 1。

**混淆矩阵（RLI|LIF，节选）**：`normal|normal` 3394 · `low_progress|normal` **1933** ·
`low_progress|low_progress` 305 · `start|start` 196 · `normal|pressure` 27 ·
`low_progress|pressure` 13 · `stuck|normal` 6 · `pressure|normal` 6 · `stuck|pressure` 5 ·
`pressure|pressure` 3 · `stuck|stuck` 1。

**主不一致机理初读**：1990 个不一致轮中 1933（97.2%）为
`low_progress|normal`——RLI 判 `low_progress` 而 LIF 判 `normal`。通道读数支持
「prog 通道语义差」的解释：RLI `prog` 通道 run 均 |u|≈**0.519**（恰在谓词阈 0.5
附近摆动——谐振器周期 8·T̂、包络衰减时间常数≈2.5 轮），而 LIF `u_prog` 是成功即
置 1 的漏积分器（run 均显著高于 0.5）。即：**谓词同形移植成功（无轴错位、无尺度
漂移），差异集中在 prog 通道自身的动力学语义**——RLI 侧 prog 是振荡量而非单调
"进度位"。RLI 域切换频繁（2528 次 vs 629）同源于此。

**C1 时间打乱**（10 置换）：135/137 run 全部置换都改变锚点；mean |Δu_err| = 0.060，
mean |Δu_prog| = 0.345 —— 锚点对时间结构显著敏感（证伪门 C1 保持「非平凡」）。
**C3 ζ=0 反事实**：mean |Δu_err| = 0.241，mean |ΔE_err| = 0.345 —— 阻尼对读数有
实质贡献（同前口径）。

**通道读数**（run 均之均）：err |u| 0.081 / prog 0.519 / stall 0.033 / slow 0.062 /
deny 0.028；末态 hits 合计：err 382 / prog 681 / deny 249 / slow 221 / stall 215。

## 4. 未完成 / 延后（如实登记）

1. **RLI 无 stuck 通道（v1 延后）**⇒ 自判域压力轴＝err 水平单轴（不含 LIF 的
   `u_stuck ≥ 1.5·T̂` 项）。本批如实收窄谓词，不以拟合补轴；若后续补 stuck 通道，
   谓词轴再回填。
2. **PULL 面无 feature 序列面**（selector 仅 now｜recent｜history；`feature`
   显式报错）——需要 anchor 序列存储面，留待按需。
3. **读数①消费率 / ②转向相关**：需载体重建 + 实体 run（0bc）；本批未取。采集口径
   （0bc 时）：journal 内 `blackboard_read` 调用的 `section=="rli"` 次数/时机/读取后
   N 轮的转向（工具选择/文案），与同 run 非读取段对照。
4. **索引/TODO/BACKLOG 未改**（按不提交/不推送指令；登记留给用户裁定的下一批）。
5. 语料回放仅取离线面；未做「同轮对照」之外的在线面（如把一致性读数并入 temporal
   渲染头）——如需要，可在 temporal 面尾部加一行（渲染成本数十字节级），本批不做。

## 5. 框架摩擦登记（本批实测）

- **FR1（编辑面 · 组合字符匹配）**：`search_replace` 对含组合字符的行（`T̂` =
  `T`+U+0302）做精确匹配时报「string not found」，同形文本自 `read_file` 输出
  复制后成功——疑 NFC/NFD 规范化差异。**绕行**：所有含音标组合字符的替换一律
  从 read_file 输出复制原文；**建议**：编辑工具面透出规范化提示或在匹配失败时
  回显「最近似行」的码点序列（本工具有 nearest-match 行，但未给码点差）。
- **FR2（测试面 · dogfood 会话 env 污染）**：在 orz 会话内直接跑 `orz-loop`
  测试二进制时，继承 `ORZ_ACAF_FAIL_CLOSED=1`（+ MANIFEST/KEYSTORE/BINARY），
  所有「起 run」类测试 panic：`ACAF fail-closed is enabled but no signer client
  is configured ... refusing to start the run`（本次 19+30 个失败全属此类）。
  **绕行**：跑测试前 `Remove-Item env:ORZ_ACAF_*`（4 个）后全绿。
  **建议**：测试入口提示/脚本化 unset，或测试内显式注入 ACAF override（不污染
  生产语义）。
- **FR3（编译面 · F14 状态更新）**：上一批记录「orz-loop 测试目标本机不可构建」
  （栈溢出/内存）。本批 `cargo test -p orz-loop --lib --no-run -j 2` **构建成功**
  （2m40s，244MB 级并行内存余量），运行时需 `RUST_MIN_STACK=67108864` +
  `--test-threads=1`。**F14 状态改为「条件性可构建」**（受并行与内存余量影响）。
- **FR4（上下文面 · 压缩与工作现场边界）**：本批读入的大文件（temporal.rs、
  设计稿、controller/tool_run 片段）在分块压缩时被留作「工作现场」，更早勘察
  折为摘要；读数未落盘前需从块档案回放（`.gsa/compaction/blocks/*.md`）。
  本次配合黑板 `plan` 面（进度 v2）无碍；作为研究性长批次的常规摩擦登记。
- **FR5（操作注记）**：语料根默认 `D:\tb-eval\jobs-official`（沿用旧件默认）；
  一次误把根写成 `D:\CLI\tb-eval\...` 导致计数命令空跑——非框架缺陷，记以免
  重犯（根路径面可考虑校验存在性并 fail-loud）。

## 6. 复现命令

```powershell
cd D:\CLI\orz
$env:PROTOC="D:\CLI\orz\bin\protoc.exe"

# 单元测试（assurance）
cargo test -p orz-assurance --lib lif::

# 重写件自检 + 语料回放（离线读数）
cargo run -p orz-assurance --example rli_shadow_replay -- --selftest
cargo run -p orz-assurance --example rli_shadow_replay -- `
    D:\tb-eval\jobs-official D:\tb-eval\analysis\0am-rli-domain-consistency-2026-09-20.json

# orz-loop 面（条件性可构建；运行前清 ACAF env）
cargo test -p orz-loop --lib --no-run -j 2
Remove-Item env:ORZ_ACAF_*     # 4 个变量
$env:RUST_MIN_STACK="67108864"
& target\debug\deps\orz_loop-*.exe controller::tests:: --test-threads=1
& target\debug\deps\orz_loop-*.exe blackboard::tests:: --test-threads=1
```

## 7. 建议的下一批（供裁定）

1. **载体重建 + 0bc**（顺序已定：先改好 RLI → 重建 → 0bc）——0bc 观测读数①②；
   本批已把消费面（`blackboard_read section=rli`）与判据口径备好。
2. **一致性读数在线化**（可选、廉价）：temporal/rli 面尾行加「域一致 a/b (x%)」，
   让模型与读数同面；或维持离线。
3. **RLI stuck 通道**（若 0bc 显示 stuck 轴确需回填）：谓词补 `u_stuck` 项并复跑
   本件离线读数对照。
4. **prog 通道语义复议**（若 0bc 显示 low_progress 判定过频）：三种可选口径
   （保持振荡量原样／给谓词加时间窗中值／把 prog 定义改成成功置位的漏积分器），
   均需先登记口径再取数，避免数据驱动调参。

## 8. 本轮裁决登记（用户 2026-09-20，落账批；不改本件前述读数）

- **`prog` 语义差**：用户裁——**先取活体读数再定**（随 0bc 收读数①②），本批不改口径；§7.4 的三种候选维持「供裁定」，守「先登记口径再取数」纪律。
- **RLI 无 `stuck` 通道**：用户裁向——**应变化的是判断标准而非 RLI 侧**。不补 `stuck` 通道：按设计 §10.3（`v`／`E` 锚点原生覆盖 stuck 语义）与 §9（**特征集换 RLI 锚点＋谓词重推导**）重做谓词；连带订正＝谓词重推导后一致性基准回到「**框架实际动作结果**」，不再以 LIF 现域为代理（本件 §3 用 LIF 现域系代理口径，已登记）。
- **数学形式**：用户问询「是否不应采用当前数学形式（但振荡形态应更适合判断）」——**待裁**；登记候选＝按设计 §11 极点配置表**按通道语义分配极点**（速率／压力类保留复极点：预期＋节律；水平／新鲜度类走实极点：持久／适应），而非整体换形式。
- **`feature` 序列面**：用户令**要补**——PULL 面增锚点**序列**面（本件 §4 该延后项转正为待办）。
- **C3（ζ=0）口径**：用户问询「阻尼对读数有实质贡献」的实际含义——**待裁**：升级为带判准的对照（分离衰减与频率两效应）或明确降格为「参数非惰性检验」标注；现读数不构成阻尼有效性证据（机理见对话回执与 §3）。

## 9. 追加裁决（用户 2026-09-20 四次令：四项定案，开工）

- **C3＝直接明确分开**：C3 从「ζ=0 反事实」改为**分离对照**——**固定 ω_d 只扫 ζ**，把「衰减快慢」与「振荡频率」两效应拆开各出一列读数；现口径的 0.241／0.345 **降格标注为「参数非惰性检验」**（§3 与 `0AM_DECODER_FORM_REPLAY` §0 的措辞按此读）。
- **数学形式＝批准改配极（不整体换形式）**：按设计 §11 极点配置表**按通道语义分配极点**——速率／压力类（err／stall／slow／deny）保留复极点（预期＋节律语义），水平／新鲜度类（prog，及 stuck 语义的承载面）走**实极点**（持久／适应语义）。族不变（一阶＝该族的强过阻尼极限，设计 §6），改的是配极与谓词。
- **判断标准＝改标准，且 RLI 不再与 LIF 对照**：`stuck` 轴按设计 §10.3（`v`／`E` 锚点原生覆盖）改写为 RLI 原生锚点上的谓词，**不补通道**；**撤销**本件 §3 的「RLI 域 vs LIF 现域」代理基准（两个状态机不同形，无对照意义）。**定位不变**：LIF 维持现役组件；RLI 无实际作用性则继续用已跑通的 LIF，有作用再替换 LIF——即判据只问 RLI 自身的可用性。
- **FR4 形态＝挂到既有压缩工具下**：压缩白名单**并入 `context_compress`**（模型处理压缩时可额外保存白名单文件），**不做独立工具**、保持 10 工具冻结面（ADR §14.72 v1.74 不变）；R1 封存的独立工具面维持封存，机制本体（白名单块／`DEFAULT_WHITELIST_CAP`／持久化）复用于工具内。

## 10. 本轮范围定案与实现关口（用户 2026-09-20 五次令「这一轮范围我同意」）

- **范围**：① **配极改**（按通道语义分配极点）＋② **判断标准改写**（RLI 原生锚点谓词；撤销 LIF 现域对照）＋③ **C3 分离对照**（固定 ω_d 只扫 ζ）＋④ **`feature` 序列面**（PULL 面补锚点序列）＋⑤ **FR4** 压缩白名单并入 `context_compress`。**不含**：FR1／FR2（已归 0bc 杂项）、提交／推送、载体重建、0bc 本体。
- **实现关口（轮内勘定后择定，理由随文档登记）**：ζ>1 的实极点需**双曲闭式解**（cosh／sinh）方能落——择定于 ① 给二阶解加**实极点分支**（族内一致、数学干净，倾向此项）或 ② `prog` **直接复用既有的一阶漏积分更新**（改动小但族内出现两种更新式）。
- **附：中止轮留档（用户 2026-09-20 六次令「可进行惯例留档」）**：本批曾误下发一轮并于 t+3min 中止（`RUN-CLI-6aafe51d`，133 事件、无 `run_finished`、**树态零改动**；`orz`／`orz-signer` 已退出）；留档件 `.tmp-0am-aborted-run-20260920.txt`（gitignore 前缀），journal 原样保留于 `.gsa/runs/RUN-CLI-6aafe51d/`。
- **摩擦处置**：**FR1／FR2 立项、并入 0bc 杂项**（按本件 §5 自报建议：FR1＝匹配失败回显最近似行的码点／规范化提示；FR2＝测试入口 script化 unset 或测试内显式 ACAF override，**不动** fail-closed 纪律）；**FR3 更新 F14 状态**为「条件性可构建」（见 [`0AM_DECODER_FORM_REPLAY`](0AM_DECODER_FORM_REPLAY_2026-09-20.md) §5-F14 状态更新）；**FR4** 压缩白名单机制＝用户倾向**挪回**（形态待裁：恢复独立工具＝破 10 工具冻结面、需 ADR 级修订／挂到既有工具＝保持 10 工具面不变；代码现状＝`compaction_whitelist_add` 系 THIN-HARNESS-REDESIGN R1 **封存**，机制本体（白名单块／`DEFAULT_WHITELIST_CAP`＝16 KiB／持久化）仍在）；**FR5 留观察**。
