# 0ar S1 契约面实施报告：检索批次回送与轮级单席位（含框架内部摩擦记录）

> 状态：**实施件已落工作区（未提交、未推送、未重建镜像）**；日期 2026-09-19。
> 基线：父仓 `HEAD=e7f6aba3`（run 起始 worktree 即含本批的前置 WIP——由前序被误杀 run 遗留，本 run 在其上收尾）；orz 子仓 pin 未动（工作树脏＝10 件 +500/−12：`Cargo.lock/toml`、orz-assurance lif（含新 `rli.rs`／`rli_shadow_replay` 示例）、orz-host `acp_server`、orz-loop `blackboard/controller/prompt`——0am 影批语境，非本批）。
> 关联：[设计稿 v1.0](../RETRIEVAL_BATCH_HANDOFF_AND_ROUND_SEAT_DESIGN_2026-09-19.md) / [TODO `P0-0ar`](../../TODO.md) / [BACKLOG 0ar](../BACKLOG_AND_PRIORITIES.md) / 索引 `RETRIEVAL-BATCH-HANDOFF-ROUND-SEAT` / [上游分析件](TB21_RETRIEVAL_VOLUME_CONVERSION_DEEP_ANALYSIS_2026-09-19.md)。
> 口径声明：本件只登记**事实与读数**；未运行者标「未跑」，未能核证者标「未核」。台账（TODO / BACKLOG / 索引）登记与计数随收尾批，本批按指令**不提交**。

---

## 1. 一句话结论

设计稿 §7「契约面（S1）」四项已收尾落于工作区：`retrieval_close_record.terminal_reason` 闭枚举 **9 → 12**（新增 `evidence_threshold_met`／`dispatch_wallclock_bound`／`subagent_early_delivery`；仅 `dispatch_wallclock_bound` 落「assessment 链必带」约束，`normal_close` 既有约束不动、新值不受其约束）；`information_sufficiency_assessment` 增**可选**「可用计数＋缺口」二字段（`gap ⇒ count` 配对）；三件正例＋两件约束反例 fixture 由生成器产出（重跑逐文件 SHA256 全等，346 件）；`ALL_REASONS` 同步锁定、契约单测 9 条全绿；门禁＝1 条预期红（唯一＝orz 子仓影批脏树）；runtime 全套 **366 tests OK**；grok 集成 46 tests OK；assurance 全套 1668 tests：唯一失败＝`test_doctor_full_repository_check`（**树态断言**——要求全净树，本地未提交批＋影批脏态下与门禁同源必红）、15 skipped。全程**零行为变更**（既有 journal／payload 无一失效）。

## 2. 任务口径

- 范围＝设计稿 §7「契约面（S1）」＋§9 批序第 1 步（schema＋fixtures＋pytest，零行为变更、可独立验收）；**未启** S2（D1→D2→D3）与 S3（真机复验）——遵守「各步独立放行、不得跳步合批」。
- 指令口径（本 run）：实现 0ar；**不提交、不推送、不重建**；按项目惯例落最终报告；过程中摩擦项一并登记（见 §7）。
- D3「未派发走 `tool_completed` 既有 `cause` 面、不新增事件类型」——S1 无需改动，登记为「已核，零改动」（落点在 S2）。

## 3. 实施结果（事实表）

| 文件 | `git diff --stat` | 内容 |
|---|---|---|
| `runtime/retrieval-close-record-event-payload-v0.2.schema.json` | 23 | 枚举 +3；新增 `allOf` if/then：`dispatch_wallclock_bound ⇒ assessment_id（string, minLength 1）＋result_digest（sha256）必带`；`description` 扩展（三值语义与约束依据） |
| `runtime/information-sufficiency-assessment-event-payload-v0.2.schema.json` | 30 | +`usable_source_count`（integer ≥0，可选）；+`sufficiency_gap{target,missing 必带；retrieval_calls,note 可选}`（`additionalProperties:false`）；新增 `allOf` if/then：`sufficiency_gap 在场 ⇒ usable_source_count 必带`；`description` 扩展 |
| `runtime/fixtures/run-event-v0.2/payloads/`（新增 6 件） | —（未跟踪） | 正例 4：`retrieval-close-record.evidence-threshold-met / dispatch-wallclock-bound / subagent-early-delivery`＋`information-sufficiency-assessment.partial-report`；反例 2：`…wallclock-missing-assessment`／`…gap-without-count`（均单一约束违反，见 §4） |
| `runtime/fixtures/run-event-v0.2/README.md` | 21 | 生成物：0ar S1 节（三值语义、两字段与配对规则、fixture 清单） |
| `scripts/generate_run_event_fixtures.py` | 161 | 新 fixture 常量＋README 文本（生成器＝fixture 单一事实源） |
| `runtime/tests/test_retrieval_close_reason_enum.py` | 116 | `BATCH_HANDOFF_REASONS`／`REQUIRED_REASONS`／`ALL_REASONS`（=12）＋`POSITIVE_FIXTURES` +3；新测试类 4 条（合计 9 条） |
| `scripts/check_repository.py` | 31 | 6 件新 fixture 正/负登记；**本 run 修复前序遗留两处破损**：负例登记先于负例映射定义（`UnboundLocalError`）→ 移至定义后；`subagent_timeout` 笔误 → 恢复 `subagent-timeout` |
| （携带，非 0ar）`scripts/dogfood_launch.ps1` | 21 | 狗粮启动器误杀修复（局部 `$ErrorActionPreference='Continue'`；背景见 §5／§7-F1；由编排方在本 run 启动前落盘，本 run 未改动） |
| **合计** | **+393 / −10**（7 件 M；不含 6 件新 fixture） | 父仓 `git status --short`：`M×7 ＋ ??×6 ＋ m orz` |

## 4. 验证读数

| 项 | 命令 | 读数 |
|---|---|---|
| 生成器对拍 | `python scripts\generate_run_event_fixtures.py`（前/后逐文件 SHA256） | **346 件全等（ZERO-DIFF）**；打印：v0.2 payloads 122／v0.2 envelope 81／v0.1 payloads 68／envelope 48／canonical_cli 7 |
| 契约单测（靶向） | `python -m unittest … test_retrieval_close_reason_enum` | 9 passed（前序 run 03:42:57 读数；本 run 由全量套件复核全绿） |
| runtime 全套 | `python -m unittest discover -s runtime/tests` | **Ran 366 tests in 283.581s — OK**（exit 0） |
| assurance 全套 | `python -m unittest discover -s assurance/tests` | **Ran 1668 tests in 759.050s — FAILED（failures=1／skipped=15）**：唯一失败＝`test_cli_dispatcher.GsaCliDispatcherTests.test_doctor_full_repository_check`（`gsa doctor` 全仓检查断言 exit 0、实得 1）——**归因实测**：手动 `python gsa.py doctor --json` 同报 `error_count=1`＝唯一 `orz submodule working tree is dirty`（与门禁同源；该测试要求全净树）⇒ 本地树态预期红，非 0ar 回归；15 skips＝环境门控（浏览器族等） |
| grok 集成 | `python -m unittest discover -s integration/grok/tests` | **Ran 46 tests in 44.856s — OK**（exit 0） |
| 门禁 | `python scripts\check_repository.py` | `error_count=1`——唯一＝`orz submodule working tree is dirty`（0am 影批在场，合回前预期态）；v0.2 payload 正/负 **71/51**（本批 +4／+2）；无 fixture set diverges；`valid=false` 仅因该 1 条；退出码 1（同因） |
| 编译 | `python -m compileall -q assurance scripts` | exit 0 |
| PS 语法 | 123 件 `scripts/*.ps1` 逐件 `Parser::ParseFile` | 全过（含携带的 `dogfood_launch.ps1`） |
| 负例单约束 | 逐件 jsonschema `iter_errors` 计数（`.tmp-neg-single-check.py`） | `…wallclock-missing-assessment` → **1 错**（path=`assessment_id`："None is not of type 'string'"）；`…gap-without-count` → **1 错**（"'usable_source_count' is a required property"） |
| 对照（前序 run 全量） | pytest 口径（03:42:07 读数） | 362 tests、**361 passed／1 failed in 287.45s**——唯一失败＝本批目标文件校准前状态（其后已修并复跑 9 passed；本 run 全量复核取代之） |

## 5. 断点与交接（run 上下文，事实）

- 本批跨两个狗粮 run：**RUN-CLI-6aad91f0**（前序，2026-09-19 03:33–03:43：完成 schema、生成器、fixture、测试主体后，因启动器 `NativeCommandError` 误杀——详 §7-F1）与 **RUN-CLI-6aad9497**（本 run，03:44:23 启动，承接续跑）。
- 编排方在本 run 启动前（03:44:10／03:44:16）落盘：前序 WIP 快照 `.tmp-0ar-aborted-wip-20260919.patch`（28,118B）＋ `.tmp-0ar-aborted-wip-status-20260919.txt`；并修复 `scripts/dogfood_launch.ps1`。两者均为批内过程件（`.tmp-*` 忽略态，不入提交）。
- 本 run 完成项：修复 `check_repository.py` 两处破损；生成器重跑对拍；全量验证（§4）；本报告。
- 遗留：S1 的**验收**（用户／收尾批）；台账登记与计数随收尾批；S2／S3 未启动。

## 6. 与设计的对照（S1 核验）

1. 枚举逐值对照设计 §7 首条 ✓（`normal_close` 约束不动；新值不落该 `allOf`——测试 `test_new_reasons_are_not_bound_by_the_normal_close_allof` 锁定；`dispatch_wallclock_bound` 约束＝「已得计数＋缺口」经 assessment 链携带，即 §8 判据 1 的契约面投影）。
2. 三件正例＋`ALL_REASONS` 同步 ✓。
3. assessment 两字段（可选⇒旧 payload 全部保持有效；`gap ⇒ count` 配对）✓——按「新增字段先立契约变更」口径。
4. D3 事件面「不新增事件类型」✓（已核，零改动）。

## 7. 框架内部摩擦记录（本 run 实测）

- **F1 启动器误杀（严重；前序 run 致死因）**：`dogfood_launch.ps1` 在 `$ErrorActionPreference='Stop'` 下以 `*>&1` 汇流，PowerShell 5.1 把原生 stderr 的**首条 transport WARN（stream idle 5s）**包装升级为终止错误 `NativeCommandError` ⇒ 脚本中止、管道被拆、进程树被收，`Tee` 日志不落盘。实测：RUN-CLI-6aad91f0 跑到第 10 分钟／72 轮／97 次工具调用被打断，journal 停在 seq 776、**无任何终态事件**；WIP 由补丁快照幸存。已由编排方修复（局部 Continue，WARN 照常落日志）。本 run 承接 WIP 续跑完成。
- **F2 半成品交接的脆弱性（严重）**：被误杀 run 遗留的 WIP 中，`scripts/check_repository.py` 处于**语法级破损**（`UnboundLocalError`：负例登记写在负例映射建立之前；另有 `subagent_timeout` 笔误）——即「跑不通」的部分若无人接手，收尾批会被直接卡住。本 run 修复后全绿。教训：写入后应立即做最低自测（`py_compile`／靶向 pytest），勿把「未自测的编辑」留在死点之后。
- **F3 输出缓冲致运行态不可观测**：PowerShell 管道下，unittest 的圆点输出（无换行）数分钟不落盘（`.tmp` 文件 0B），「慢」与「死」不可分；负例消息落盘后圆点计数仍滞后（缓冲伪停滞）。本 run 以进程树＋CPU 采样自证存活。改法：长命令以 `python -u`／分段落盘／`Start-Process` 直写日志。
- **F4 控制台编码（GBK 代码页）**：中文经 `run_terminal_cmd` 回显为 mojibake（如 `git diff` 中文注释、`urllib` 中文错误文本「鐢变簬…」）；绕行＝关键内容一律以 `read_file` 读取核证，勿以控制台回显判正误。
- **F5 命令面习惯冲突**：cmd 习惯（`dir /b`）→ `Get-ChildItem: Cannot find path 'D:\b'`；`head` 等 Unix 工具不可用——统一以纯 PowerShell 语素绕行。
- **F6 PowerShell 文本面陷阱**：双引号内 `"$n: …"` 的 `$n:` 被解析为作用域限定 ⇒ `ParserError`（本 run 实测 1 次；`${n}` 绕行）。
- **F7（观测，非缺陷）本机套件长跑量级**：runtime/tests ≈ 4m45s／366 tests（含 global-progress 子进程族）；assurance/tests ＝ 1668 tests／12m39s（759.050s；等待型长跑——浏览器族以 skip 门控、PDF 解析与凭证清理走噪声输出）；grok 集成 ≈ 45s／46 tests。供后续 run 时间预算参考。
- **F8（读数口径警示）树态敏感测试**：`test_doctor_full_repository_check` 断言 `gsa doctor` 全仓检查 exit 0——**要求工作树全净**；本 run 的未提交批＋orz 影批脏树使其必红（与门禁 `error_count=1` 同源）。本地全量读数应作此注记，勿计为回归。

## 8. 复现与回放指针

```
python scripts\generate_run_event_fixtures.py          # 生成器重跑（346 件逐文件 SHA256 全等）
python -m compileall -q assurance scripts
python scripts\check_repository.py                     # error_count=1（唯一＝orz 子仓脏树，预期态）
python -m unittest discover -s runtime/tests           # Ran 366 tests — OK
python -m unittest discover -s assurance/tests         # 1668 tests：唯一失败＝doctor 全仓检查（脏树预期红，见 §4）
python -m unittest discover -s integration/grok/tests  # Ran 46 tests — OK
```

- 证据文件（忽略态过程件）：`.tmp-gen-s1-before.txt`／`.tmp-gen-s1-after.txt`（346 件 SHA256 快照）、`.tmp-check-s1.txt`（门禁输出）、`.tmp-unittest-runtime.txt`／`.tmp-unittest-assurance.txt`／`.tmp-unittest-grok.txt`、`.tmp-neg-single-check.py`、`.tmp-doctor-s1.json`／`.tmp-doctor-inspect.py`（doctor 归因复测）。
- WIP 快照：`.tmp-0ar-aborted-wip-20260919.patch`（28,118B）／`-status.txt`。
- journal：本 run `.gsa/runs/RUN-CLI-6aad9497/events.jsonl`；前序 `.gsa/runs/RUN-CLI-6aad91f0/events.jsonl`（停 seq 776）。
