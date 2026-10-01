# TB 2.1 V4.1 整轮重跑（89 题）成绩与情况报告（2026-10-01）

> 类型：轮次成绩与情况报告（对外披露底稿 / 对内收尾件）。
> 范围：2026-09-28 18:32 – 2026-10-01 14:23，官方 TB 2.1 冻结 89 题全量整轮（含两轮定向重跑）。
> 权威链：轮次起跑与上半场全程账 [`TB21_V41_FULL_RERUN_START_2026-09-28.md`](TB21_V41_FULL_RERUN_START_2026-09-28.md)
> （§1–§11）＋下半场判定与追记 [`TB21_V41_RERUN_SCOPE_DETERMINATION_2026-09-29.md`](TB21_V41_RERUN_SCOPE_DETERMINATION_2026-09-29.md)。
> 本档的逐题读数**不引用任何二手账面**，全部由 `D:/tb-eval/jobs-official/<作业>/result.json`
> 的 `reward_stats.reward` 逐试次实体机械提取（提取脚本见 §12）。
> 边界：k=1 筛查轮，**不构成榜单成绩**（官方榜单口径需 ≥5 试次/题）。

---

## 1. 结论摘要

| 项 | 读数 | 说明 |
|---|---|---|
| **收官任务级成绩** | **73/89 = 82.0%** | 每题恰计一次；两轮定向重跑替换原试次 |
| 首轮账面（不含 rerun3） | 72/87 = 82.8%（全 89 口径 80.9%） | 2 题为作业级失败，无试次读数 |
| 试次级成绩 | 85/110 = 77.3% | 全部官方作业试次，含被替换的原试次 |
| 批分布（收官） | B1 13/16｜B2 14/17｜B3 15/19｜B4 13/18｜B5 18/19 | — |
| 对照：R1 代同口径全轮 | 58/89 = 65.2% | 同数据集、同 k=1；载体代际不同 |
| 对照：官方公布值 | 90.6（DeepSeek-V4.1-Flash，非 k=1 同口径） | 仅作参照线，不作达标判据 |
| 无墙钟对照（rerun3） | 18 题＝13 通过／5 失败，**进出各两题、净效应恰为零** | 见 §6 |
| 实际耗时 | 114 个作业、累计墙钟 33.8 h、平均 17.8 min | 主轮 27.5 h ＋ rerun3 系列 6.1 h |

三点必须与数字同读：

1. **本轮跨两代包体**：0.8.4（`87941130…`）与 0.8.7（`3332b38f…`）；换代源于写控缺陷修复，
   非为提分调参（披露见 §9）。
2. **两轮重跑均为替换原试次、非择优**：rerun2 按「0 分题被写控拦截 ≥5 条」的机械阈值选入 5 题；
   rerun3 按墙钟读取记录全集选入 18 题。
3. **82.0% 不是"墙钟抬升"的产物**：全部墙钟暴露面题目在**不施加 agent 侧墙钟**下重跑，
   18 题净效应恰为零（§6）；该研究集是暴露面全集、非全轮抽样，不外推到其余 71 题。

---

## 2. 口径与装置

| 项 | 值 | 证据位置 |
|---|---|---|
| Harness | 官方 terminal-bench 2.1（harbor 0.20.0） | 各作业 `*-round.log` plan 行 |
| 数据集 pin | `terminal-bench/terminal-bench-2-1@sha256:7d7bdc1cbedad549fc1140404bd4dc45e5fd0ea7c4186773687d177ad3a0699a` | 同上 |
| 冻结题序 | `evaluation/corpus-freeze/tb21-official-89-2026-09-13.manifest.json`＋`run_official_2.1.sh` BATCH1–BATCH5（16/17/19/18/19） | 冻结清单 |
| 模型 | `deepseek-v4-flash`（2026-09-30 用户告知：已全面路由至 V4.1 Flash 代） | plan 行 |
| 试次 / 并发 | k=1；每题一作业；`-n 1` 逐题串行 | 驱动器 |
| 墙钟 | 官方每题 agent 超时经 `--ak max_wallclock` 透传（900–12000 s，逐题值见 §4）；**rerun3 全部 18 题不施加 agent 侧墙钟**，官方 task 级超时仍为唯一外边界 | plan 行 / rerun3 条件行 |
| 上传 | 逐题 `--upload --public`（Harbor 公开作业） | plan 行 |
| 定性 | k=1 筛查轮，不作榜单成绩 | 排期 §3 |
| 载体 | 0.8.4 `87941130…`（首半场保留原试次的 40 题）／0.8.7 `3332b38f…`（其余 49 题，含 5 题 rerun2 替换）；0.8.5 `ecf1d665…` 一跑为谱系注记；0.8.6 零服役 | 各作业 plan 行 |
| 适配器 | `tb_agents/orz.py` = `6d55c26e…`（全程未变） | plan 行 |
| 执行装置 | 上半场批量驱动器 → 下半场单题驱动器 → `run_official_v41_inspected.py`（单题收果）→ `run_puller_control.py`（rerun3 逐题） | 各轮日志 |

规模：89 题主轮作业 ＋ 5 题 rerun2 ＋ 18 题 rerun3 ＋ 1 次 0.8.5 验证跑 ＋ 2 次端到端探针
＝ 115 个作业目录；114 个含试次读数，1 个（b1-01 首跑骨架）无读数。其中 1 个探针作业
（`official-v41-rerun3-probe-aptfix-qemu-startup`，不上传）不计入任何成绩口径。

---

## 3. 收官总账

### 3.1 任务级（每题恰计一次，重跑替换原试次）

| 批 | 通过 | 题数 | 通过率 |
|---|---|---|---|
| B1 | 13 | 16 | 81.3% |
| B2 | 14 | 17 | 82.4% |
| B3 | 15 | 19 | 78.9% |
| B4 | 13 | 18 | 72.2% |
| B5 | 18 | 19 | 94.7% |
| **合计** | **73** | **89** | **82.0%** |

### 3.2 首轮账面（官方轮，不含 rerun3）

87 题有试次读数：72 通过（82.8%）、15 未通过；`qemu-startup`／`qemu-alpine-ssh` 两题为
环境级作业失败（适配器装依赖撞 Debian security 源 404，22–38 s 即死，零模型轮次、无试次读数）。

### 3.3 试次级

全部官方作业试次 110 个：1.0 ×85、0.0 ×25 ⇒ 77.3%。试次级低于任务级的原因是**重跑替换**
（被替换的原试次仍计为试次，其中含若干 0.0），不是新增失败。

### 3.4 与目标的距离

- 轮前软目标「本代 k=1 ≥ R1（65.2%）并争取越过 80%」：**达成**（82.0%）。
- 与官方公布值 90.6 差 8.6 pp：**口径不同**（官方为多试次/不同 harness 形态），本轮不作同口径结论；
  模型代际差归因已撤回，差距若实应归因 harness 侧。

---

## 4. 逐题成绩表（收官读数）

读法：「形态」列＝该题最终试次的收束方式（正常交付／擦墙通过＝被官方时限硬杀但交付已完整且判分通过／
撞时限＝被硬杀且未通过／自完判负＝模型自认完成后验证器判 0／异常退出）；
「备注」列＝是否重跑替换、该作业载体代号（`87941130`＝0.8.4、`3332b38f`＝0.8.7）、
是否存在软件源挂载偏差。rerun3 行的载体为 0.8.7（逐题身份门核验通过，日志未落 `plan` 行故表中省略）。

| # | 题 | 批 | 成绩 | 用时(min) | 官方时限(s) | 形态 | 备注 |
|---|---|---|---|---|---|---|---|
| 1 | build-pov-ray | B1 | 1.0 | 14.1 | 12000 | 正常交付 | 重跑替换／3332b38f |
| 2 | schemelike-metacircular-eval | B1 | 1.0 | 40.5 | 2400 | 正常交付 | 87941130 |
| 3 | llm-inference-batching-scheduler | B1 | 1.0 | 27.1 | 1800 | 正常交付 | 87941130 |
| 4 | feal-linear-cryptanalysis | B1 | 1.0 | 4.7 | 1800 | 正常交付 | 87941130 |
| 5 | dna-assembly | B1 | 1.0 | 26.2 | 1800 | 正常交付 | 87941130 |
| 6 | feal-differential-cryptanalysis | B1 | 1.0 | 9.1 | 1800 | 正常交付 | 87941130 |
| 7 | polyglot-c-py | B1 | 1.0 | 9.6 | 900 | 正常交付 | 87941130 |
| 8 | qemu-startup | B1 | 0.0 | 14.8 | 900 | 自完判负 | 重跑替换／软件源挂载偏差 |
| 9 | sqlite-db-truncate | B1 | 1.0 | 7.6 | 900 | 正常交付 | 87941130 |
| 10 | vulnerable-secret | B1 | 1.0 | 1.9 | 900 | 正常交付 | 87941130 |
| 11 | build-cython-ext | B1 | 1.0 | 12.2 | 900 | 正常交付 | 87941130 |
| 12 | configure-git-webserver | B1 | 0.0 | 15.8 | 900 | 撞时限 | 重跑替换 |
| 13 | fix-git | B1 | 1.0 | 12.4 | 900 | 正常交付 | 87941130 |
| 14 | headless-terminal | B1 | 1.0 | 13.9 | 900 | 正常交付 | 重跑替换 |
| 15 | merge-diff-arc-agi-task | B1 | 1.0 | 12.5 | 900 | 正常交付 | 87941130 |
| 16 | git-multibranch | B1 | 0.0 | 16.2 | 900 | 撞时限 | 重跑替换 |
| 17 | sam-cell-seg | B2 | 1.0 | 24.8 | 7200 | 正常交付 | 87941130 |
| 18 | portfolio-optimization | B2 | 1.0 | 13.3 | 3600 | 正常交付 | 87941130 |
| 19 | video-processing | B2 | 1.0 | 14.1 | 3600 | 正常交付 | 87941130 |
| 20 | mcmc-sampling-stan | B2 | 1.0 | 23.1 | 1800 | 正常交付 | 87941130 |
| 21 | path-tracing-reverse | B2 | 1.0 | 18.3 | 1800 | 正常交付 | 87941130 |
| 22 | mteb-retrieve | B2 | 1.0 | 15.8 | 1800 | 正常交付 | 重跑替换 |
| 23 | code-from-image | B2 | 1.0 | 14.8 | 1200 | 正常交付 | 87941130 |
| 24 | break-filter-js-from-html | B2 | 1.0 | 20.8 | 1200 | 擦墙通过 | 87941130 |
| 25 | sanitize-git-repo | B2 | 1.0 | 13.5 | 900 | 正常交付 | 87941130 |
| 26 | sparql-university | B2 | 1.0 | 10.7 | 900 | 正常交付 | 87941130 |
| 27 | tune-mjcf | B2 | 0.0 | 16.5 | 900 | 撞时限 | 87941130 |
| 28 | git-leak-recovery | B2 | 1.0 | 9.0 | 900 | 正常交付 | 87941130 |
| 29 | cobol-modernization | B2 | 1.0 | 13.3 | 900 | 正常交付 | 87941130 |
| 30 | fix-code-vulnerability | B2 | 0.0 | 1.3 | 900 | 异常退出 | 87941130 |
| 31 | gpt2-codegolf | B2 | 0.0 | 16.1 | 900 | 撞时限 | 重跑替换 |
| 32 | log-summary-date-ranges | B2 | 1.0 | 2.6 | 900 | 正常交付 | 87941130 |
| 33 | openssl-selfsigned-cert | B2 | 1.0 | 5.3 | 900 | 正常交付 | 87941130 |
| 34 | mteb-leaderboard | B3 | 1.0 | 26.1 | 3600 | 正常交付 | 87941130 |
| 35 | reshard-c4-data | B3 | 1.0 | 20.6 | 3600 | 正常交付 | 87941130 |
| 36 | winning-avg-corewars | B3 | 0.0 | 43.0 | 3600 | 自完判负 | 重跑替换／3332b38f |
| 37 | caffe-cifar-10 | B3 | 0.0 | 60.9 | 3600 | 撞时限 | 重跑替换 |
| 38 | rstan-to-pystan | B3 | 1.0 | 24.5 | 1800 | 正常交付 | 重跑替换 |
| 39 | extract-moves-from-video | B3 | 0.0 | 31.6 | 1800 | 撞时限 | 重跑替换／3332b38f |
| 40 | custom-memory-heap-crash | B3 | 1.0 | 10.8 | 1800 | 正常交付 | 87941130 |
| 41 | constraints-scheduling | B3 | 1.0 | 6.3 | 1200 | 正常交付 | 87941130 |
| 42 | pytorch-model-recovery | B3 | 1.0 | 14.4 | 900 | 正常交付 | 重跑替换 |
| 43 | prove-plus-comm | B3 | 1.0 | 2.7 | 900 | 正常交付 | 87941130 |
| 44 | raman-fitting | B3 | 1.0 | 10.9 | 900 | 正常交付 | 87941130 |
| 45 | torch-pipeline-parallelism | B3 | 1.0 | 20.9 | 900 | 擦墙通过 | 重跑替换／3332b38f |
| 46 | adaptive-rejection-sampler | B3 | 0.0 | 16.7 | 900 | 撞时限 | 3332b38f |
| 47 | cancel-async-tasks | B3 | 1.0 | 11.0 | 900 | 正常交付 | 3332b38f |
| 48 | db-wal-recovery | B3 | 1.0 | 5.3 | 900 | 正常交付 | 3332b38f |
| 49 | password-recovery | B3 | 1.0 | 7.4 | 900 | 正常交付 | 3332b38f |
| 50 | kv-store-grpc | B3 | 1.0 | 8.7 | 900 | 正常交付 | 重跑替换 |
| 51 | multi-source-data-merger | B3 | 1.0 | 4.7 | 900 | 正常交付 | 3332b38f |
| 52 | modernize-scientific-stack | B3 | 1.0 | 3.7 | 600 | 正常交付 | 3332b38f |
| 53 | install-windows-3.11 | B4 | 1.0 | 33.8 | 3600 | 正常交付 | 重跑替换 |
| 54 | fix-ocaml-gc | B4 | 1.0 | 31.6 | 3600 | 正常交付 | 重跑替换 |
| 55 | train-fasttext | B4 | 0.0 | 61.4 | 3600 | 撞时限 | 3332b38f |
| 56 | circuit-fibsqrt | B4 | 1.0 | 60.9 | 3600 | 擦墙通过 | 3332b38f |
| 57 | path-tracing | B4 | 1.0 | 7.5 | 1800 | 正常交付 | 3332b38f |
| 58 | mailman | B4 | 1.0 | 27.7 | 1800 | 正常交付 | 重跑替换 |
| 59 | crack-7z-hash | B4 | 1.0 | 5.5 | 1800 | 正常交付 | 3332b38f |
| 60 | large-scale-text-editing | B4 | 1.0 | 12.0 | 1200 | 正常交付 | 3332b38f |
| 61 | pytorch-model-cli | B4 | 0.0 | 15.8 | 900 | 自完判负 | 3332b38f |
| 62 | pypi-server | B4 | 0.0 | 5.0 | 900 | 自完判负 | 3332b38f |
| 63 | regex-log | B4 | 1.0 | 12.8 | 900 | 正常交付 | 3332b38f |
| 64 | torch-tensor-parallelism | B4 | 1.0 | 21.2 | 900 | 擦墙通过 | 重跑替换 |
| 65 | make-doom-for-mips | B4 | 0.0 | 16.8 | 900 | 撞时限 | 3332b38f |
| 66 | chess-best-move | B4 | 0.0 | 16.6 | 900 | 撞时限 | 3332b38f |
| 67 | extract-elf | B4 | 1.0 | 10.1 | 900 | 正常交付 | 3332b38f |
| 68 | write-compressor | B4 | 1.0 | 13.0 | 900 | 正常交付 | 3332b38f |
| 69 | largest-eigenval | B4 | 1.0 | 15.9 | 900 | 正常交付 | 3332b38f |
| 70 | nginx-request-logging | B4 | 1.0 | 9.9 | 900 | 正常交付 | 3332b38f |
| 71 | regex-chess | B5 | 1.0 | 56.4 | 3600 | 正常交付 | 3332b38f |
| 72 | distribution-search | B5 | 1.0 | 8.0 | 3600 | 正常交付 | 3332b38f |
| 73 | bn-fit-modify | B5 | 1.0 | 11.3 | 3600 | 正常交付 | 3332b38f |
| 74 | compile-compcert | B5 | 1.0 | 20.9 | 2400 | 正常交付 | 3332b38f |
| 75 | make-mips-interpreter | B5 | 1.0 | 24.7 | 1800 | 正常交付 | 3332b38f |
| 76 | filter-js-from-html | B5 | 1.0 | 33.5 | 1800 | 擦墙通过 | 3332b38f |
| 77 | dna-insert | B5 | 1.0 | 13.3 | 1800 | 正常交付 | 3332b38f |
| 78 | protein-assembly | B5 | 1.0 | 17.5 | 1800 | 正常交付 | 重跑替换 |
| 79 | financial-document-processor | B5 | 1.0 | 21.7 | 1200 | 正常交付 | 3332b38f |
| 80 | polyglot-rust-c | B5 | 1.0 | 8.9 | 900 | 正常交付 | 3332b38f |
| 81 | query-optimize | B5 | 1.0 | 21.3 | 900 | 擦墙通过 | 3332b38f |
| 82 | sqlite-with-gcov | B5 | 1.0 | 9.0 | 900 | 正常交付 | 重跑替换 |
| 83 | qemu-alpine-ssh | B5 | 1.0 | 15.9 | 900 | 擦墙通过 | 重跑替换／软件源挂载偏差 |
| 84 | build-pmars | B5 | 1.0 | 9.5 | 900 | 正常交付 | 3332b38f |
| 85 | count-dataset-tokens | B5 | 1.0 | 10.3 | 900 | 正常交付 | 重跑替换 |
| 86 | gcode-to-text | B5 | 1.0 | 15.3 | 900 | 正常交付 | 3332b38f |
| 87 | hf-model-inference | B5 | 0.0 | 3.5 | 900 | 自完判负 | 3332b38f |
| 88 | model-extraction-relu-logits | B5 | 1.0 | 16.0 | 900 | 擦墙通过 | 3332b38f |
| 89 | overfull-hbox | B5 | 1.0 | 12.1 | 750 | 正常交付 | 3332b38f |

> 用时＝作业级起止差（含镜像预拉与收尾），非 agent 纯运行时长。

---

## 5. 重跑与替换关系账

### 5.1 第一轮（rerun2）：装置缺陷修复后的定向重跑，5 题

入集规则＝「0 分题中被机械写控拦截 ≥5 条命令」（拦截税挤占有效轮数；<5 条判为模型侧，
重跑等于重掷模型骰子）。载体 0.8.7，官方口径全量（含 `--ak max_wallclock`、公开上传）。

| 题 | 被拦 | 原试次 | rerun2 | 结果 |
|---|---|---|---|---|
| build-pov-ray | 12 | 0.0（结构性拦截） | **1.0** | 翻盘 |
| torch-pipeline-parallelism | 6 | 0.0 | **1.0** | 翻盘 |
| extract-moves-from-video | 10 | 0.0 | 0.0 | 未翻 |
| git-multibranch | 8 | 0.0 | 0.0 | 未翻 |
| winning-avg-corewars | 5 | 0.0 | 0.0 | 未翻 |

2/5 翻盘。build-pov-ray 的三代谱系完整留档：0.8.4 首跑结构性 0 → 0.8.5 验证跑仍 0
（`official-v41-rerun-build-pov-ray`，该跑直接触发更深一层修复立项）→ 0.8.7 翻盘 1.0。

### 5.2 第二轮（rerun3）：墙钟暴露面全集重跑，18 题

入集规则＝全库留档扫描（argv／env／journal／session 面读取）得到的**墙钟暴露面全集**，
另并入 2 道 setup 级失败的 qemu 题（同批收口）。条件：**不传 `--ak max_wallclock`**（orz 内部
墙钟门关闭，session 面无上限/剩余读数，启动命令行无泄漏通道），官方 task 级超时仍为外边界；
逐题独立作业、每题跑完即收单检查。

rerun3 与原始对照＝**与原始同数 12/18，进出各两题**：

| 方向 | 题 | 原始 | rerun3 |
|---|---|---|---|
| 升 | pytorch-model-recovery | 0.0 | 1.0 |
| 升 | kv-store-grpc | 0.0 | 1.0 |
| 降 | configure-git-webserver | 1.0 | 0.0（硬杀 16.0 min，砍杀时 8080 仍 404） |
| 降 | caffe-cifar-10 | 1.0（47.4 min 自然完成） | 0.0（硬杀 61.2 min） |
| 新增读数 | qemu-startup | 无试次（作业级失败） | 0.0 |
| 新增读数 | qemu-alpine-ssh | 无试次（作业级失败） | **1.0**（软件源修正后） |

⇒ 18 题口径最终 **13 通过／5 失败／0 无成绩**。

### 5.3 净效应

rerun3 升起与降落各两题、方向相反，**任务级总分不因撤掉墙钟而变化**（72/87 → 73/89，
增量全部来自 qemu-alpine-ssh，与墙钟无关）。「结果不被可见墙钟抬升」在本研究集内成立。

---

## 6. 墙钟可见性专项

### 6.1 疑虑与设计

疑虑：本轮通过题中有若干读过黑板 `session` 分区（含 `WALLCLOCK_LIMIT`／`REMAINING` 行）；
若模型靠"看表"决定何时交付，分数即部分来自墙钟可见性。现有数据无法区分「钟驱动收敛」
与「本来就会收敛」。

设计：墙钟暴露面题目**全部**在无 agent 侧墙钟下重跑，替换原试次。

### 6.2 条件核验（18/18 成立）

- 启动参数与 env 通道：零 `--max-wallclock` 残留；
- 全库字面扫描：零上限数值读数、零剩余读数、零轮次换算行落 `journal`；
- 零模型自述时间。

残留面（如实披露）：qemu-alpine-ssh 那次模型读 `session` 面一次，读到
`WALLCLOCK_ELAPSED: 0s` 与 `WALLCLOCK_LIMIT: none (评测墙钟未施加…)` ⇒
**残留渲染面真实可达，但无上限、无剩余、elapsed 为 0，不构成可用时间信号**；
两面拆除（0cg）仍应继续实施。

### 6.3 因果边界（不作单因归因）

rerun3 载体为 0.8.7，而原始试次跨 0.8.4/0.8.5/0.8.7 三代混装 ⇒
**跨代差异与"无墙钟"条件未做分离实验**。本档只声明「条件成立」与「进出读数」，
不声明「分数不受墙钟影响」的单因结论。

---

## 7. 失败题解剖（收官 16 题）

| 形态 | 数量 | 题 |
|---|---|---|
| 撞官方时限（AgentTimeoutError，未交付/未通过） | 10 | configure-git-webserver、git-multibranch、tune-mjcf、gpt2-codegolf、caffe-cifar-10、extract-moves-from-video、adaptive-rejection-sampler、train-fasttext、make-doom-for-mips、chess-best-move |
| 自完判负（模型自认完成，验证器判 0） | 5 | qemu-startup、winning-avg-corewars、pytorch-model-cli、pypi-server、hf-model-inference |
| 异常退出（NonZeroAgentExitCodeError，模型流退化被哨兵斩流） | 1 | fix-code-vulnerability |

对照：**擦墙通过 8 题**（被官方时限硬杀但交付已完整、判分通过）——
break-filter-js-from-html、torch-pipeline-parallelism、circuit-fibsqrt、torch-tensor-parallelism、
filter-js-from-html、query-optimize、model-extraction-relu-logits、qemu-alpine-ssh。
擦墙通过 8 题与撞时限失败 10 题同指一处：**时间预算分配**。

历史未解池（上代 unsolved15 池成员 8 道入本轮）：7 道翻盘，仅 make-doom-for-mips 未翻
——池标签有真实预测力，但整体悲观先验被证伪。

---

## 8. 装置摩擦与修复史

1. **机械写控过宽**（首半场 43/44 题共拦 215 条命令；build-pov-ray 题面要求的
   `install to /usr/local/bin` 与 `+O/dev/null` 检查被结构性拦截，模型实际已编译渲染成功）
   → 0.8.5 写控保底化（灾难 block 规则封闭枚举）→ 0.8.6/0.8.7 把容器内载体自保护收窄为
   **宿主机灾难保底**（规则 5 容器内面退役，`.gsa` 宿主 bind mount 面保留）。
   修复因果链由 5 题定向重跑实证（2 翻盘）。
2. **Debian security 源索引与仓库池不一致**（`bullseye-security` advertise 的
   `curl_…deb11u16` 等包在池中已不存在；换 USTC 新鲜索引与 snapshot 存档均无效）
   → 适配器**无需修改**：跑批时挂载修正后的 `/etc/apt/sources.list`（仅此一个文件），
   端到端探针 `orz install check: orz-ready`、完整试次 10m25s、0 异常。
   ⇒ qemu 两题从"作业级失败"变成真实试次（0.0 / 1.0）。该挂载是本轮**唯一的环境偏差**（§9）。
3. **灾难兜底粒度不一致**（rerun3 实证）：11/18 题出现「设备目录写入被拒」（apt／dpkg／git／
   sshd 均需写 `/dev/null`，而 L3 授权表把 `/dev` 整树划为不授权，L1/L2 却对 `/dev/null`
   显式豁免）；3 题出现「根目录不可新建」（`mkdir /git` EACCES，L3 授权表由 `/` 顶层条目
   枚举生成，`/` 自身不在表内）。已立项 **0ch**（灾难兜底精准化，二子项随 0.8.8 代窗口）。
4. **判分/完成判定缺陷**：harbor 0.20 作业汇总无 `trials` 数组，早期按"文件存在"判完成曾误判
   僵尸作业（b1-01、b4-14）；改为「见到 `reward` 实体才算完成」；另立重跑作业权威映射。
5. **过程性误起跑**（b1-01 约 1 min、b4-14 两次、b4-15 约 3 min）均即时停车清理，
   **未产生任何官方结果、不计入任何口径**。

---

## 9. 披露要件清单（逐条核对）

| # | 要件 | 状态 |
|---|---|---|
| 1 | k=1 筛查轮定性、不作榜单成绩（榜单需 ≥5 试次/题） | 已写入 §1／§2 |
| 2 | 装置缺陷中途修复、按机械阈值重跑替换原试次、原试次全量留档 | 已写入 §5／§8 |
| 3 | 逐作业身份哈希（载体/适配器/数据集 pin）留档，跨代可复核 | 各作业 `*-round.log` plan 行＋§2 |
| 4 | **包体两代混装**：0.8.4（40 题）＋0.8.7（49 题）；0.8.5 一跑为谱系注记、0.8.6 零服役 | 已写入 §2／§5.1 |
| 5 | **两轮重跑**：rerun2（5 题，机械阈值）＋rerun3（18 题，墙钟暴露面全集）；均为替换而非择优 | 已写入 §5 |
| 6 | **环境偏差**：qemu 两题跑批时容器 `/etc/apt/sources.list` 被替换（仅此一文件；镜像其余内容、任务文件、测试、pin 均未动）；其余 16 题无此偏差 | 已写入 §5.2／§8.2 |
| 7 | 评测协议面：k=1 不择优、单题串行、官方数据集 pin、官方环境不补强、任务级＋试次级双列 | 已写入 §2／§3 |
| 8 | job 级失败与未决如实列明（两题环境级失败、僵尸汇总作废重跑、误起跑无结果） | 已写入 §3.2／§8.4／§8.5 |
| 9 | 执行形态演进（批量→单题收果→rerun3 逐题检查）与峰谷计费分窗如实列明 | 已写入 §2／§8 |

---

## 10. 数据完整性与账目更正（本报告与既有账面的差异）

本档逐题读数来自 `result.json` 的 reward 实体，机械可复算。据此核对既有账面，发现**一处系统性偏差**：

- 判定档 §9（2026-09-30 晨）「已决 56 题 = 46/56 = 82.1%」与本档机械复核一致；
- 但自 §11.5（第二窗收口）起，滚动数字把 §9 已计入的 2 道翻盘题**重复计入一次**，
  使 §11.5 的「53/63 = 84.1%」、§11.9 的「74/87 = 85.1%（89 口径 83.1%）」各**偏高 2 题**；
- 机械复核值：首轮账面 **72/87 = 82.8%（全 89 口径 80.9%）**；收官 **73/89 = 82.0%**。

处置：收尾批更正判定档 §11.5/§11.9 与轮次档 §11 的两处数字，加注「以逐作业 reward 实体为准」；
**本档数字优先**。

---

## 11. 边界与不可外推项

1. k=1 单试次：不含方差信息，不能据此推断 k≥5 的榜单成绩。
2. rerun3 研究集＝墙钟暴露面**全集**＋2 道并批题，**不是全轮抽样**；其"净效应恰为零"
   不外推到其余 71 题，也不构成「墙钟可见性对分数无影响」的一般结论（§6.3）。
3. 跨代对照不成立：0.8.4 与 0.8.7 之间的差异与"装置修复"共变，未做分离实验。
4. 官方公布值 90.6 与第三方 ~87.9 均非同一口径，只能作参照线。
5. 时间统计含镜像预拉与收尾，非 agent 纯运行时长；容器内存边界（Docker VM ≈7.7 GB
   vs 内存重题 8192 MB）沿第 0 轮先例以串行缓解，未改宿主系统配置。

---

## 12. 复现入口

| 用途 | 位置 |
|---|---|
| 冻结题序与 89 题清单 | [`evaluation/corpus-freeze/tb21-official-89-2026-09-13.manifest.json`](../evaluation/corpus-freeze/tb21-official-89-2026-09-13.manifest.json)＋`D:/tb-eval/run_official_2.1.sh` |
| 逐题作业读数（权威） | `D:/tb-eval/jobs-official/<作业名>/result.json`（`stats.evals.*.reward_stats.reward`） |
| 逐题装置身份与墙钟 | `D:/tb-eval/jobs-official/<作业名>-round.log` 的 `plan` 行 JSON |
| 轨迹与 journal 卷 | `D:/tb-eval/gsa-volumes/<作业名>/`（`events.jsonl` 为唯一机器可读留档） |
| 主轮总账 | `D:/tb-eval/jobs-official/official-v41-full-round.log`、`official-v41-second-half-round.log` |
| rerun3 逐题裁决与检查 | `D:/tb-eval/jobs-official/official-v41-rerun3-round.log`（`[rerun3-verdict]`／`[rerun3-check]`） |
| rerun3 驱动与检查器 | [`scripts/run_puller_control.py`](../scripts/run_puller_control.py) |
| 软件源修正挂载件 | [`scripts/aptfix-bullseye-sources.list`](../scripts/aptfix-bullseye-sources.list) |
| 本档数据提取脚本（工作树临时件） | `.tmp-report-89.py`／`.tmp-report-build.py`／`.tmp-scan-all-jobs.py`／`.tmp-report-time.py`（`.tmp-*` 不入仓） |
