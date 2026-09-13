# P2-15 S1/S2：TB 2.1 官方 89 题语料冻结与阈值校准干跑（2026-09-13）

> 类型：实施与校准记录（P2-15 EVALUATION-CORPUS-FREEZE S1 语料冻结 +
> S2 阈值校准干跑）。触发：TB 2.1 V4.1 代际新一轮跑批排期第 0 步
> （[`TB21_V41_GENERATION_ROUND_SCHEDULE_2026-09-13`](../TB21_V41_GENERATION_ROUND_SCHEDULE_2026-09-13.md) §3）。
> 结论一句话：官方 89 题语料身份已按**注册表（registry）digest**冻结并与既有
> 账面逐题对齐（89/89 全等）；同时查出两处必须登记的事实——本地 checkout 的
> 台账对 1 题过期、内部 evaluation runner 的产出与其注册 schema 不兼容（76 处）。
> 本记录**不含任何跑批结果**，也不做任何 V4 / V4.1 分差比较。

## 0. 为什么这轮必须先做语料冻结

旧账面（R1–r4b）是**混代际**数据，不能直接当新后端基线；重跑目标是"单代际、
语料冻结、载体版本唯一"的新基线。若语料身份不可复现，新数据同样不可复现——
这一冻结就是本轮数据的**可复现锚点**：跑批时 harness 解析的是哪一版题目内容、
跑完后如何证明 89 题全部来自同一份被冻结的语料。

## 1. S1 语料冻结（89 题）

### 1.1 冻结对象与三个独立身份来源

| 来源 | 内容 | 本次核证结果 |
|---|---|---|
| 数据集 pin | `terminal-bench/terminal-bench-2-1@sha256:7d7bdc1c…`（取自数据集仓 `leaderboard/src/leaderboard/core/hub.py::DATASET_REF`，与跑批脚本 `run_official_2.1.sh::DATASET` 逐字一致） | 一致 |
| 权威注册表 digest | `harbor` 注册表对上述 pin 的**逐题 sha256**（元数据查询，不下内容）：89 题 | **与账面 89/89 全等** |
| 本地 checkout | 提交 `7131e437`（工作树干净）、89 个题目目录、1,035 文件 / 58,447,021 B | 88/89 与注册表内容等价（见 §1.3） |

权威 digest 快照：`evaluation/corpus-freeze/tb21-registry-task-digests-2026-09-13.json`
（`scripts/fetch_tb21_registry_digests.py` 生成；复现命令见 §4）。

### 1.2 批次与规模（冻结清单）

- 分批沿用跑批脚本自身定义的 `BATCH1…BATCH5`（由脚本解析，不手抄）：
  **16 + 17 + 19 + 18 + 19 = 89**（c1–c9 形态即 R1 的分块形态）。
- 集合核证：批次并集 = 本地题目目录 = 注册表清单 = 89；**无重复、无缺题、无多余项**。
- 难度分布：easy 4 / medium 55 / hard 30；16 个 category（software-engineering 26、
  system-administration 9、scientific-computing 8、security 8、data-science 8 …）。
- 逐题记录：tree digest + 三组分面 digest（`prompt` / `environment` / `oracle`，
  其中 `oracle` = `tests/` + `solution/`，与模型可见面物理分开）+ task.toml 元数据
  （category / difficulty / tags / agent 与 verifier 超时 / 镜像 / cpus / memory）。

清单文件：`evaluation/corpus-freeze/tb21-official-89-2026-09-13.manifest.json`
（243 KB；89 条逐题条目 + 批次表 + 一致性块 + 装置件 digest）。
语料**本体不入仓**（上游 Harbor 数据集，靠 pin + 注册表 digest 复现；本地 checkout
仅作阅读副本，按提交号 + 逐题 digest 登记）——这是 S1"存放形态"的定案。

### 1.3 一个必须登记的差异：`sanitize-git-repo`（本地台账过期，非跑批问题）

1. 本地 `tasks/dataset.toml` 对 `sanitize-git-repo` 记 `sha256:73c94a21…`，
   而**注册表**与**账面**都记 `sha256:6e862977…`。
2. 为定性，把 pin 对应内容整包下载到临时目录逐题逐文件比对（EOL 归一化 +
   忽略 `.gitignore`）：**88/89 完全一致**，唯一差异是
   `sanitize-git-repo/tests/test_outputs.py`。
3. 差异内容是**语义等价**的写法改动：本地把假密钥拆成
   `"AKIA" + "1234567890123456"`，发布版为整串字面量 `"AKIA1234567890123456"`
   （其余 5 条同样）。即本地副本是"拆串"版，pin 对应的发布版是"整串"版。
4. 结论：**账面 89 题跑的都是 pin 对应发布版内容**（注册表 digest 与账面
   lock / result 逐题全等）；本地台账（`dataset.toml`，由 `harbor sync` 维护）
   在该题上过期。影响面：只有"读本地副本"的场合会被误导；本轮跑批走 pin，
   不受影响。
5. 另一条设备性事实：本地 checkout 为 `core.autocrlf=true`（文本 CRLF）且不含
   `.gitignore`，与注册表内容**不是逐字节相同**——因此冻结的权威身份只能是
   注册表 digest，本地 digest 只作证据面。这条写进清单的
   `published_copy_comparison` 与 `dataset_registry.role` 字段。

### 1.4 装置件（执行契约）冻结

| 件 | 字节 | sha256（前 16） |
|---|---|---|
| `tb_agents/orz.py`（官方口径透传） | 36,828 | `2737cfadc5c43603` |
| `tb_agents/orz_strict.py` | 1,983 | `8f48256d091edfde` |
| `orz-linux/orz`（0.5.0） | 109,982,680 | `393eee34dd0357ca` |
| `orz-linux/orz-signer` | 1,397,360 | `3d5c8155ef2b93e5` |
| `orz-linux/orz-acaf-provision` | 1,216,288 | `73fcaeda51eeb3f8` |
| `orz-windows/orz.exe` | 52,742,144 | `a45b60e54b080b65` |
| `orz-windows/orz-signer.exe` | 6,742,528 | `d19a1394bf16149e` |
| `orz-windows/orz-acaf-provision.exe` | 6,642,176 | `34a4a91a9a31de27` |
| `scripts/setup_harbor_proxy.ps1`（装置侧代理入口） | 1,613 | `ed65cf11ce850b9a` |
| `scripts/orz_acaf_run.ps1`（ACAF 透传入口） | 1,671 | `4bd18379f80b24a9` |

载体五项与索引 v2.98 登记的 0.5.0 哈希逐一吻合（`a45b60e5…` / `393eee34…` /
`d19a1394…` / `34a4a91a…`）。`D:\tb-eval\.env` 只登记存在与大小（273 B），
**故意不哈希**：密钥轮换不应让语料冻结失效。

## 2. 阈值校准干跑

### 2.1 本轮判据阈值在既有账面上的工作点（S2 第一部分）

判据来源：排期文档 §6.5「零 400 + 哨兵 ≤3 + 命中率 ≥90%」，逐题抽查面见
`evaluation/corpus-freeze/threshold-calibration-2026-09-13.json`。
口径：**只看阈值的工作点，不比较分数**；reward 仅作上下文携带。

**样本**：账面 269 份 `result.json`，其中官方 TB 2.1 批次 **245 试次**
（TB 4.0 / smoke / dry-run 不计）；含 journal 的 116 试次。

**① 命中率（journal 口径，provider cache）**：

| n | min | p10 | p50 | p90 | max | mean |
|---|---|---|---|---|---|---|
| 116 | 63.93% | 91.29% | 94.98% | 97.64% | 99.46% | 94.26% |

- 达标（≥90%）**106/116 = 91.4%**：阈值落在历史分布的 p10 附近，**有区分度**，
  是一条会真被踩到的线，不是形同虚设的高线。
- 10 条未达标（最低 largest-eigenval 63.93%）：r1-c5 ×3（constraints-scheduling、
  prove-plus-comm、cancel-async-tasks）、r1-c8 ×3（polyglot-rust-c、distribution-search、
  dna-insert）、dna-insert ×2（另两批）、multi-source-data-merger 90.0% 临界。
- 建议：报告里沿用"逐试次命中率 + 达标计数"二元口径，并把**样本数与分母**
  一并写出（避免只报均值）。

**② 输出健康哨兵（每 run 触发次数 ≤3）**：可观测面 = `agent/orz.txt` 的
`output-health guard trip detail=degeneration_detected:…` 行
（一次触发落 3 行日志，故计数以 guard trip 行为准）。

| 口径 | 值 |
|---|---|
| 触发总数 | 12 |
| 单 run 最大 | **2** |
| 达标（≤3） | **245/245** |
| 家族 | `reasoning_stall` 8 试次、`reasoning_repetition` 1 试次 |

- 结论：既有官方批次**没有超过预算的反例**（历史最大 2 次），阈值 ≤3 目前
  **未经受压力**；本轮若出现 >3 或 `run_invalidated`，才是该阈值的首个判别样本。
- 边界：哨兵**不落 journal 事件**（设计上哨兵中断不产生 `transport_retry`），
  只在 agent 日志面可测。因此该判据的可核对性依赖"日志面保留"这一前提。

**③ 零 HTTP 400**：三个面全零——116 份 journal 的 payload（0 命中）、
245 份 agent 日志、274 份 harness `trial.log`，marker 集合共 11 个写法
（`HTTP 400` / `status_code=400` / `"status": 400` / `BadRequestError` …）。

- 边界：这是"**已检面为零**"而不是"无关面为零"。本轮报告须**连 marker 集合
  一起登记**，否则"零 400"是未检零。

**④ 上下文（不设阈值）**：终端态 `run_finished` 113 / `run_invalidated` 3；
3 条失效全在 `official-r2-failures-900s`，payload 均为 `{"status":"wallclock"}`
（墙钟杀死，与 0v-C 同族）。0z 的资源族事件（`host_resource_snapshot` /
`host_resource_denied` / `resource_exhausted` / `process_tree_reaped` /
`reclaim_performed` / `resource_limit_hit`）在旧账面**全为 0**——它们是 0.5.0
的新增面，本轮才是首次落地观测。

### 2.2 内部 evaluation/holdout 阈值层：干跑结果（S2 第二部分）

P2-15 原文的 S2 是"小样本干跑校准 **evaluation/holdout 判定阈值**"。
干跑记录：`evaluation/corpus-freeze/evaluation-threshold-dryrun-2026-09-13.json`
（`scripts/p215_evaluation_threshold_dryrun.py`）。

**做了什么**：取冻结内部语料 `regression/cases-v0.1.yaml` 的 development 分区
小样本 6 题（FEP-REG-001…006）→ 走真实导出链（匿名化 + leak scan）→ 单独构造
oracle bundle → 以占位（全 `unassessed`）响应驱动 `EvaluationRunner` 全流程 →
按注册 schema 校验产出。**不执行任何模型**，因此不产生任何能力结论。

**结果**：

| 项 | 值 |
|---|---|
| 导出 / leak scan | 通过（clean） |
| oracle 隔离 | 通过（scenario 面仅 `case_id`/`title`/`task`/`visible_facts`） |
| run status | `descriptive_only`（runner 明确封顶） |
| acceptance 块 | `{"status": "requires_review", "limiting_factors": []}` |
| 与注册 schema 的校验错误 | **76 处**，跨 10 个顶层字段 |

错误分布（条数）：`red_lines` 24 / `system_profile` 11 / `counts` 8 / `metrics` 8 /
`adjudication` 7 / `protocol_ref` 6 / `corpus_ref` 4 / **`acceptance` 4** /
`integrity` 3 / `artifacts` 1。

**结论（两句话）**：

1. **阈值层现在不可设**：runner 把状态封顶在 `descriptive_only`，不产出
   `threshold_set`；按 `SCORING_PROTOCOL_v0.1` §9，在有密封 evaluation/holdout
   分区 + 双人 blind baseline 之前，acceptance 必须停在 `not_calibrated`。
   干跑把这条**从"协议自述"变成"实测行为"**。
2. **还查出一处硬缺口**：`evaluation/evaluation-result-v0.1.schema.json` 要求
   acceptance 为 `{status: not_calibrated|pass|fail, threshold_set, reasons}`，
   而 runner 实际输出 `{status: "requires_review", limiting_factors: []}` ——
   `requires_review` 不在枚举内、`threshold_set`/`reasons` 缺失。即**产出与其
   自身注册 schema 不兼容**，阈值无从挂靠。

**处置建议（不本批实施，待裁决）**：登记 **GAP-EVAL-RESULT-SCHEMA-DRIFT**
（runner 产出 vs 注册 schema 的机械对齐 + schema 校验进测试面），作为 P2-15 S3
（首轮真实 evaluation 跑批）的前置项；未对齐前 S3 不具备可核对产出。

## 3. 冻结机制怎么用（本轮起跑的机械动作）

1. **跑前**：确认 pin 未变 —— `scripts/freeze_tb21_corpus.py --registry-digests
   … --check --out …`，期望 `OK: corpus matches frozen manifest`（不一致即 DRIFT）。
2. **跑中**：harness 每个 job 落 `lock.json`（内含逐题 `task.digest`），
   试次 `result.json` 内含 `task_id.ref` 与 `task_checksum`——这是"实际跑了
   哪一版内容"的一手记录。
3. **跑后**：把 89 题的 `lock.json` digest 与清单的 `registry_digest` 逐题对表；
   任一处不等即说明轮内语料漂移（或解析到了别的版本），须按 §2 代际纪律记录。
4. **报告**：语料身份 = 数据集 pin + 注册表逐题 digest + 载体哈希；
   报告为**单代际基线**，不与 V4 Flash 做分差比较。

## 4. 复现命令

```text
# 1) 权威注册表逐题 digest 快照（需评测 venv：harbor + leaderboard 包）
D:\tb-eval\venv\Scripts\python.exe scripts\fetch_tb21_registry_digests.py ^
    --out evaluation/corpus-freeze/tb21-registry-task-digests-2026-09-13.json

# 2) 冻结清单（含 89 题 digest、批次表、装置件、一致性块）
python scripts\freeze_tb21_corpus.py ^
    --registry-digests evaluation/corpus-freeze/tb21-registry-task-digests-2026-09-13.json ^
    --env-file D:/tb-eval/.env ^
    --artifact D:/tb-eval/tb_agents/orz.py --artifact D:/tb-eval/tb_agents/orz_strict.py ^
    --artifact D:/tb-eval/orz-linux/orz --artifact D:/tb-eval/orz-linux/orz-signer ^
    --artifact D:/tb-eval/orz-linux/orz-acaf-provision ^
    --artifact D:/tb-eval/orz-windows/orz.exe --artifact D:/tb-eval/orz-windows/orz-signer.exe ^
    --artifact D:/tb-eval/orz-windows/orz-acaf-provision.exe ^
    --artifact scripts/setup_harbor_proxy.ps1 --artifact scripts/orz_acaf_run.ps1 ^
    --out evaluation/corpus-freeze/tb21-official-89-2026-09-13.manifest.json

# 3) 跑前检查（drift 检查）
python scripts\freeze_tb21_corpus.py --check ^
    --out evaluation/corpus-freeze/tb21-official-89-2026-09-13.manifest.json

# 4) 阈值校准（既有账面）
python scripts\tb21_threshold_calibration.py ^
    --out evaluation/corpus-freeze/threshold-calibration-2026-09-13.json

# 5) evaluation/holdout 阈值干跑（内部语料小样本）
python scripts\p215_evaluation_threshold_dryrun.py ^
    --out evaluation/corpus-freeze/evaluation-threshold-dryrun-2026-09-13.json

# 6) 与 pin 对应内容逐题比对（需先 harbor download 到临时目录）
harbor download terminal-bench/terminal-bench-2-1@sha256:7d7bdc1c… -o <tmp> --export
python scripts\freeze_tb21_corpus.py --verify-against <tmp>/terminal-bench-2-1 ^
    --ignore-name .gitignore ^
    --out evaluation/corpus-freeze/tb21-official-89-2026-09-13.manifest.json
```

## 5. 边界与未做项

- 不产生、不引用任何跑批分数；reward 只在校准文件里作上下文，未做任何分差比较。
- 语料本体不入仓；注册表查询为元数据（不下内容）。内容级比对用的整包下载落在
  系统临时目录，比对完删除，`D:\tb-eval` 与语料仓均未被写入。
- `D:\AGI` 全程未触碰；本批写入面仅 `D:\CLI` 内的 `scripts/`、`evaluation/`、
  `docs/` 与台账文件。
- 未做项（待裁决）：① `GAP-EVAL-RESULT-SCHEMA-DRIFT` 的立案与修复（P2-15 S3
  前置）；② 密封 evaluation/holdout 分区的建立与双人 blind baseline（S2 的真
  阈值前置，属人力项）；③ 命中率未达标 10 条的逐条归因（本记录只做阈值工作点）。

## 6. 证据清单

- 冻结清单：`evaluation/corpus-freeze/tb21-official-89-2026-09-13.manifest.json`
- 注册表 digest 快照：`evaluation/corpus-freeze/tb21-registry-task-digests-2026-09-13.json`
- 阈值校准：`evaluation/corpus-freeze/threshold-calibration-2026-09-13.json`
- evaluation 阈值干跑：`evaluation/corpus-freeze/evaluation-threshold-dryrun-2026-09-13.json`
- 工具：`scripts/freeze_tb21_corpus.py` / `scripts/fetch_tb21_registry_digests.py` /
  `scripts/tb21_threshold_calibration.py` / `scripts/p215_evaluation_threshold_dryrun.py`
- 一手 pin 来源：`D:\tb-eval\terminal-bench-2-1\leaderboard\src\leaderboard\core\hub.py`
  （`DATASET` / `DATASET_REF`）；跑批脚本 `D:\tb-eval\run_official_2.1.sh`
- 账面锚点：`D:\tb-eval\jobs-official\**\lock.json` 与 `**\result.json`
- 关联：排期 [`TB21_V41_GENERATION_ROUND_SCHEDULE_2026-09-13`](../TB21_V41_GENERATION_ROUND_SCHEDULE_2026-09-13.md) /
  内部测评协议 [`SCORING_PROTOCOL_v0.1`](../../evaluation/SCORING_PROTOCOL_v0.1.md) /
  分区与 oracle 隔离 [`PARTITION_AND_ORACLE_ISOLATION_v0.1`](../../evaluation/PARTITION_AND_ORACLE_ISOLATION_v0.1.md) /
  [`BACKLOG P2-15`](../BACKLOG_AND_PRIORITIES.md) / [`TODO P2-15`](../../TODO.md)
