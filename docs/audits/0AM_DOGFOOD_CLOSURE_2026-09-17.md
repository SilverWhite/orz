# 0am 狗粮 run 收口与 0ah/0ak/0aj 验收读数（主会话报告）

> 日期：2026-09-17；run＝**`RUN-CLI-6aaad7c8`**（2026-09-16T17:54:17Z 启动，18:27:25Z `run_finished`，历时 32m08s，1,586 事件，161 模型轮，205 工具调用，ACAF 票据 118/118，权限决策 205/205 零拒绝）。
> 载体＝**orz 0.6.0** Windows 三件套（`D:\tb-eval\orz-windows\orz.exe`，SHA256 `a642b8d1…`，与 [`060_CARRIER_REBUILD`](060_CARRIER_REBUILD_2026-09-16.md) 换装位逐对一致）。
> 题面＝用户 2026-09-17 指定原文逐字（存 `.tmp-0am-task.txt`，548 B）：0am 系列任务、略带实验性质、可自主调整、觉得需停即停、不提交不推送、留任务报告、摩擦一并登记。**无墙钟**（`ORZ_MAX_WALLCLOCK=0`）。
> 本报告为**主会话收口档**：与 agent 自落报告 [`0AM_DOGFOOD_2026-09-17`](0AM_DOGFOOD_2026-09-17.md) 互补——负责三项验收判据读数、启动配置复现面、误启动摩擦登记与 F11 归因更正。两仓工作树改动保持**未提交、未推送**。

---

## 0. 三项验收判据结果速览

| 项 | 判据 | 结果 | 关键读数 |
|---|---|---|---|
| **0aj** 黑板写权限放行 | 无头 run 中调用 → allow → `plan_write` 出账 → 读回一致 | **判据面满足**（闭合待裁决） | `blackboard_write` ×8 全放行（decision=`allow_once`）→ `plan_write` ×8（plan 2／notes 6）一一对应；写后 `blackboard_read(section=plan)` exit 0 读回 |
| **0ak** 无头三键存档 | 无头长 run（≥500K）产出 `.gsa/archives/<session8>.json.gz` 三键齐备 | **未触发（不可判）** | 全 run 最高模型面估算 **324,739** est，未跨 500K 里程碑；`archives/` 目录未产生；会话侧车 0 件（「阈值下零产物」钉子生产实证） |
| **0ah** v8 真机读数 | 0.77 换算复测＋五项真机读数（实施回执 §9 配方） | **读数已收取** | 阶梯全轨迹＋H1 ×3（越线重武装实活）＋T1 ×0＋压缩 ×5＋0.77 复测八配对（§2） |

---

## 1. 启动与复现配置

- **ACAF 重新 provision（换装后必须步）**：0.6.0 signer 哈希 `ade2db9a…` ≠ 旧 manifest 内 `5b363aca…`（0.5.2 时代值）。以 `orz-acaf-provision.exe <keystore> <manifest>` 现场重配后 manifest 绑定新哈希；旧 manifest 留档 `signer-manifest.json.bak-20260916-052`。
- **启动命令**（052 先例原样，cwd＝`D:\CLI`）：
  `cd /d/CLI && ORZ_MAX_WALLCLOCK=0 ORZ_ACAF_MANIFEST=…\signer-manifest.json ORZ_ACAF_KEYSTORE=…\keystore ORZ_ACAF_BINARY=…\orz-signer.exe D:/tb-eval/orz-windows/orz.exe -p "$(cat .tmp-0am-task.txt)" --real --allow-write --allow-shell --allow-network > .tmp-0am-dogfood-run.log 2>&1`
- **启动健康**（+2.5 min）：347 事件、ACAF 20/20、`orientation_checkpoint` 已发、`plan_write` 已出账——0aj 链路在 run 首几分钟即开始工作。
- **工作区形态**：按用户指示直接以 `D:\CLI` 为 cwd（非隔离复制）。信任条目已在册；`orz/target` 暖构建被 run 复用。

## 2. 0ah v8 真机读数（判据＝读数收集，非通过/失败门）

### 2.1 阶梯与水位轨迹（mechanical_audit `context_scale:*` 全量）

| 轮 r | 键 | 里程碑 | 模型面估算 | tier | 分块数 |
|---|---|---|---|---|---|
| 48 | `first_block` | — | 168,851 | standalone_block | 1 |
| 60 | `192k` | 192,000 | 192,584 | soft | 1 |
| 73 | `224k` | 224,000 | 226,218 | soft | 3 |
| 83 | `256k` | 256,000 | 257,747 | soft | 5 |
| 121 | `288k` | 288,000 | 289,619 | soft | 8 |
| 139 | `320k` | 320,000 | 320,237 | **hard_reminder (H1)** | 9 |
| 141 | `320k` | 320,000 | 324,739 | **hard_reminder (H1)** | 9 |
| 151 | `320k` | 320,000 | 321,384 | **hard_reminder (H1)** | 9 |

- 软提醒四级各恰好一次；**H1 320K 三次触发**＝「按越线重新武装」语义实活（触发→模型交摘要→水位仍越线→再武装）。**T1 500K 硬截断 ×0**（未达）。全 run 最高估算 **324,739**。
- 压缩面 `context_compressed` ×5：`model_summary/model_selected` ×3（retained_rounds 61/75/87）＋ `model_summary/context_scale_window` ×2（retained_rounds 142/152）——机械轨/语义轨并存、模型自选压缩与窗口压缩两种形态均获真机样本。观察（非缺陷登记）：五事件中 `model_participated`／`view_estimate_after` 等字段为 null（schema 可空），语义轨迹的读数载体以 `retained_rounds` 为主。

### 2.2 0.77 换算复测（配方：`context_scale:<档>` 行估算 ↔ 其后第一条 `model_output` 的 hit＋miss）

| 行 | est | real（hit+miss） | est/real |
|---|---|---|---|
| first_block | 168,851 | 128,127 | 1.318 |
| 192k | 192,584 | 143,585 | 1.341 |
| 224k | 226,218 | 158,042 | 1.431 |
| 256k | 257,747 | 176,619 | 1.459 |
| 288k | 289,619 | 198,288 | 1.461 |
| 320k (r139) | 320,237 | 216,310 | 1.480 |
| 320k (r141) | 324,739 | 220,336 | 1.474 |
| 320k (r151) | 321,384 | 224,722 | 1.430 |

- 浅水区（168–192K）比值 1.32–1.34 贴合现行换算 1/0.77≈1.30；**深水区（224–320K）漂至 1.43–1.48**（＝real/est ≈ 0.68–0.70）。方向上偏保守（守卫 700K est 按实测折算 ≈ 476–490K real，距 1M 窗口更远），不构成风险；数值移交 0ah 判据线，是否按深度分段修正换算留裁决。
- **H1 缓存成本观察**：三次 H1 触发中两次打破前缀缓存（r139 hit=0／r151 hit=3,072，单次全量 miss ≈ 216–222K real token）——硬提醒块的注入位置与 I6 前缀纪律的相互作用建议在 0ah 线复核（成本重算记录的补充数据点）。
- 全 run 成本：hit 20,723,840 ＋ miss 2,280,146 ≈ **23.0M real token**（hit 占比 90.1%）。

### 2.3 回放/指针面

零 offset 续读、零 compaction/archives 回读——本 run 未产生需要回放的强制需求（无 T1 截留、无指针化事件），回放面「零使用」为如实读数，不构成判据结论。

## 3. 0aj 黑板写全链（判据面满足）

| 链节 | 读数 |
|---|---|
| 调用 | `blackboard_write` ×8（plan 2／notes 6） |
| 权限 | 全 run 权限决策 205/205 零拒绝；8 次黑板写全 `allow_once`（ReadOnly 内存类自动放行） |
| 出账 | `plan_write` ×8 与调用一一对应（0ai run 的 `plan_write` ×0 恒空形态就此翻转） |
| 读回 | 写后 `blackboard_read(section=plan)` exit 0；模型后续行为（按黑板任务书推进 S1→S2→S3→S4）为消费旁证 |

边界：`blackboard_read` 响应正文不进 journal，「文本级一致」超出本证据面可判范围（上述行为旁证＋exit 0 为可得读数）。**0aj 可闭合入账（33→…按账本口径），待用户裁决。**

## 4. 0ak 无头三键存档（未触发，不可判）

- 全 run 最高估算 324,739 < 500,000 里程碑 ⇒ `incremental_archive_due` 两点判定均未满足；`.gsa/archives/` 未产生（目录不存在），`.gsa/conversations/` **0 件**——0ak 实施「阈值下零产物」钉子的生产形态实证。
- 与下发前预警一致：0am 系列 33 分钟收尾不足以自然推过 500K。0ak 判据**维持未勾**，待跨 500K 的长 run（建议下轮狗粮线选题时把「跨 500K」作为选题约束之一，或在题面显式给出足够工作量）。

## 5. 0am 本体读数（引 agent 报告＋主会话独立复核）

- agent 报告：[`0AM_DOGFOOD_2026-09-17`](0AM_DOGFOOD_2026-09-17.md)——S1 Part A 校核（修 2 处编译错）/ S2 RLI 影子落码（`lif/rli.rs` 新建、门控默认关、侧车随行）/ S3 134-run 回放**负结果如实**（Q1 AUC 0.532<0.70、Q2 ΔAUC −0.011 CI 含 0、Q3 弱；建议维持一阶基座、RLI 保持影子态）/ S4 摩擦 12 条（F1–F12）/ 未决 O1–O8。
- **主会话独立复核（本机实跑）**：`cargo test -p orz-loop --lib` → **787/0/3**；`cargo test -p orz-assurance --lib` → **244/0**——与 agent 报告读数逐位一致。
- 回放读数档：`D:\tb-eval\analysis\0am-{rli-shadow-replay,lif-1d-baseline}-20260917.json`（在盘核验）。
- 工作树改动（未提交）：`orz` 子模块 10 文件（`rli.rs`／`rli_shadow_replay.rs` 新建＋lif 族/loop 三文件/host 一文件/两 Cargo.toml）＋父仓 `docs/audits/0AM_DOGFOOD_2026-09-17.md` 新建。账本（BACKLOG/TODO/索引）零改动（O7 留裁决，符合指令）。

## 6. 摩擦登记（主会话侧，编号接 agent 报告 F1–F12 之后）

- **M-1（运行方操作摩擦，主因归我）误启动与 cwd 残留**：后台启动命令未显式 `cd`，执行 shell 的工作目录残留在前一命令的 `D:\CLI\orz` ⇒ 首次启动的 run（`RUN-CLI-6aaad682`，01:48:50）以 **orz 子模块目录为 cwd** 起跑——题面要求的 `CLI_PROJECT_INDEX.md` 与设计文档全在 `D:\CLI` 根，读工具越出 git 根受硬拦，run 必然瘸腿。+4 min 健康检查（journal 落点＋题面文件不可达）检出后 taskkill 终止、清残留、显式 cwd 重启。**连带后果（重要）**：该误启动 run 有写权限且活跃工作 4.5 分钟，在共享工作树留下未提交 WIP（prompt.rs＋controller.rs 约 294 行、含 2 处编译错）——即 agent 报告 §2/F11 所称「本 run 起始即含未提交 WIP」的真实来源。教训：**狗粮启动器脚本化**（0ai 收尾建议二次坐实）——env＋信任＋载体路径＋**显式 cwd 断言**一次装配；后台启动前 `pwd` 断言应入 checklist。
- **M-2（F11 归因更正）**：agent 报告称 WIP「疑为此前会话中断产物」——按时间线更正：主会话于启动前（~01:40）实测 orz 工作树**干净**（`git status --short` 零输出），01:48–01:53 间唯一写入者即误启动 run，故 WIP 作者＝误启动 run `RUN-CLI-6aaad682`（其 journal 已随清理删除，逐秒归因以时间线为证，如实登记残余不确定性）。S1 Part A 基线代码实为误启动 run 所写、真实 run 校核修复——代码质量已由 §5 独立复核背书，但**产出血缘含一次被终止的 run**，合回裁决时应知悉。
- **M-3（F3 佐证边界）**：serialize 失败模型面见 2 处、journal 可检索 1 处——工具结果序列化缺陷的计数口径在两侧不一致，排查时以两处并查为宜。
- **M-4（0ah 成本观察，非缺陷）**：见 §2.2 H1 缓存成本——两次 H1 各付出一次全量 miss（合计 ≈ 440K real），为滑块成本重算的新数据点。

## 7. 未决与移交

1. **0am 裁决**：O1（判据标签口径）＋O2（RLI 转正/维持影子）留用户；agent 建议＝维持一阶基座、影子默认关、补数据后复审（报告 §8 给翻转批顺序）。
2. **0aj**：判据面满足，闭合入账待裁决（连带 0al 的账本联动由账本批统一处理）。
3. **0ak**：判据维持未勾；下轮长 run 收取（选题约束建议含「自然跨 500K」）。
4. **0ah**：0.77 复测读数与 H1 缓存成本数据移交判据线；S2 三形态裁决与尾批（块轴）仍待放行；`context_compressed` 部分字段 null 的读数载体形态建议在 S2 裁决时一并看。
5. **本批产物合回**：两仓工作树改动未提交；0am 产出（S1/S2/S3）是否合回、以何批次合回，留用户裁决（注意 M-2 血缘说明）。
6. **卫生项**：`.tmp-0am-task.txt`／`.tmp-0am-dogfood-run.log` 留盘（`.tmp` 前缀在门禁排除面内），供取证；不需要时可直接删除。
