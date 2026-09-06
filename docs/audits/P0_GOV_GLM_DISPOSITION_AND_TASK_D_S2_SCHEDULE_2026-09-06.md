# P0-GOV GLM 外部审查处置批 + 任务 D S2 排期登记（2026-09-06）

> **审计日期**：2026-09-06
> **性质**：GLM 外部只读审查候选（F1/F2/R-1/R-2/R-3 + 观察项 (c)）的用户裁决
> 处置落地；任务 D（双实现终局治理）剩余部分（S2–S4）排期登记。本批按
> 2026-09-04 登记口径「后续如用户裁决，将逐项转 BACKLOG 并附本登记为入口
> 证据」执行，不虚报未做项。
> **范围**：orz 子模块（F1 skills 豁免收窄）+ 父仓库卫生（R-1/R-2/R-3）+
> 差距/观察登记（F2/(c)）+ 任务 D S2–S4 排期（TODO/BACKLOG/索引）。
> **关联前序**：[GLM 外部审查登记](GLM_EXTERNAL_REVIEW_REGISTRATION_2026-09-04.md)
> / [P0-GOV 收口审计](P0_GOV_UNCOMMITTED_REVIEW_HANDLING_2026-09-04.md) /
> [任务 D batch-1](P0_GOV_TASK_D_DUAL_IMPL_GOVERNANCE_2026-09-04.md) /
> BACKLOG 00。

---

## 1. GLM 五项处置（用户裁决 2026-09-06）

| 编号 | 原级别 | 处置裁决 | 落地 | 证据/入口 |
|---|---|---|---|---|
| F1 | P2 | 收窄为注册表式技能根豁免（登记建议） | **已实施**：orz `67b51eb1` | 见 §1.1 |
| F2 | P2 | 补登记为 gap（索引 + BACKLOG） | **登记完成，实施待排期** | GAP-APPROVAL-PROMPTER（§1.2） |
| R-1 | P2 | `git rm --cached` 转本地件，或收窄 ignore | **转本地件**：353 个已入库本地运行产物移出索引（另 1 个误中夹具恢复跟踪；磁盘保留、历史可恢复） | §1.3 |
| R-2 | P3 | 生成脚本显式 LF 写出 | **已实施**：`generate_orz_source_manifest.py` 显式 `newline="\n"`；重新生成 manifest | §1.4 |
| R-3 | P3 | 归档至 `存档/` 或移出跟踪 | **归档 9 个产物**至 `存档/root-artifacts-2026-09-06/`；`gsa.py` 例外保留 | §1.5 |
| (c) | 观察 | 与任务 D 一并纳入终局治理视野，另立观察、不入 S2 盘点 | **登记观察项** OBS-PERMISSION-DUAL-IMPL | §1.6 |

### 1.1 F1：技能豁免收窄为注册技能根白名单

根因复核与登记一致：`resources::is_path_within_workspace` 对「文件名为
`SKILL.md` 或路径含 `skills` 目录组件」的任意 `.md` 先于沙箱放行，等价于可
构造的越界只读通道（模型可猜 `…/SKILL.md` / 任意 `skills/…` 绝对路径）。

实施（orz `67b51eb1`，5 文件 +275/−127）：

- 新增 `SkillRoots(Vec<PathBuf>)` 资源；`registry::finalize` 从
  `SessionContext.skills` 的每个 `SKILL.md` 派生技能包根（父目录），空列表 =
  无豁免（fail-closed）。orz-host `build_toolset` 现传 `skills: Vec::new()`，
  即生产面从「任意猜测路径豁免」收窄为「无豁免」。
- `read_file` / `grep` / `list_dir` 沙箱改传注册根；`read_file` 的整读/免截断
  carve-out 同步只对注册技能根内文档生效。
- 测试：resources 74 passed（含注册根放行/未注册拒绝/空根 fail-closed）；
  read_file 技能 carve-out 5 passed；新增工具级负测
  `read_file_rejects_unregistered_skills_outside_workspace`。
- `cargo check -p orz-tools --all-targets` Exit 0；`cargo fmt --check` 净。

### 1.2 F2：approval prompter 存根补登记

复核属实：`orz-host/src/approval.rs` 为注释 + TODO 存根，`lib.rs` 标注
「approval path (still a stub)」，索引/BACKLOG 无 gap 登记。处置=只登记、本批
不实施（防膨胀）：新增 `GAP-APPROVAL-PROMPTER`（索引 §3.1 + BACKLOG 00a），
排期与实现待用户裁决，不并入任务 D S2 盘点。

### 1.3 R-1：.gitignore 与已跟踪文件矛盾

处置=转本地件（`git rm --cached`，不删磁盘）：353 个被 2026-09-04 Phase 2
「本地测试输出不入库」策略覆盖的已跟踪产物（`_windows_high_nist` 下
`job-*` / `vm-*-result*.txt` / `vm-*-run*.txt` / `diag-*.txt` /
`evidence-*` 等）移出索引；同批另有 1 个集成夹具
`integration/grok/fixtures/workspace-control-surfaces/.claude/settings.json`
被当时未锚定的 `.claude/` 泛化规则一并命中移出，随 59423ac 锚定 `/.claude/`
后以原 blob（`38adf0e`）恢复跟踪，最终净移出 353 项（「354」= 当时的
tracked-but-ignored 命中总数，含该夹具；2026-09-06 复核修正口径）。核对：
`git ls-files -ci --exclude-standard` 354 → 0；工作树文件全部保留；历史提交
（如 2026-09-02 证据批）仍可恢复原文。忽略策略本身（Phase 2 已定）保持
不变，矛盾消除。

### 1.4 R-2：manifest 全 CRLF

处置=生成脚本显式 LF：`scripts/generate_orz_source_manifest.py` 以
`open(..., newline="\n")` 写出；重新生成 `orz_source_manifest.sha256` 并复核
无 `\r`（门禁 Exit 0）。

### 1.5 R-3：根目录一次性实验产物

归档 9 个产物至 `存档/root-artifacts-2026-09-06/`：`LIF_102RUNS_*`×5、
`_final_smoke_*`×3、`container-probe-2026-08-20.sh`。同步更新引用路径：
`MECHANICAL_LAYER_MATH_CALCULUS_I1_IMPL_AUDIT_2026-08-30`、
`…_PHASE2_REVIEW_AUDIT_2026-08-31`、`…_PHASE3_VERIFICATION_AUDIT_2026-08-31`
与 GLM 登记文档。

`gsa.py` **例外保留跟踪**：`scripts/check_repository.py` required 文件 +
旧 canonical CLI 顶层入口（`README`/quickstart/历史审计均有引用），不属于
一次性实验产物；已在 GLM 登记文档 R-3 行注明。

`存档/root-artifacts-2026-09-06/_final_smoke_2026-08-25_launcher.ps1` 保留
归档时原始内容，其内部 `D:\CLI\_final_smoke_2026-08-25_config.json` 为历史
绝对路径引用（2026-09-06 复核注记）：本目录定位为 provenance/evidence
存档，不再作为可复跑脚本面，引用保持原样不作改写。

### 1.6 观察项 (c)：权限判定分散

复核属实（`orz-workspace/permission/manager.rs` 8,756 行 vs
`orz-host/permission.rs` 1,331 行双处承担权限判定）。处置=另立观察
OBS-PERMISSION-DUAL-IMPL（BACKLOG 00a + 索引登记），**不入任务 D S2 盘点**；
终局治理视野（随任务 D/控制面收口）再排期。

## 2. 任务 D 剩余部分排期（S2–S4）

任务 D 总目标不变：在 Rust `orz-assurance` 补齐机械规则断言，逐步退役 Python
双法官。batch-1（schema 级离线法官 + payload registry 单源）已闭合
2026-09-04；本排期覆盖 S2–S4，全部以「最小可验收单元 + 独立审计 + 独立提交」
推进，可随时停。

### 2.1 批次计划

- **S2a 盘点与证据化（先行批，排期起点）**：将 `run_event_journal_validation.py`
  的 31 个 `_verify_v02_*` 族逐族盘点为两档：A=「Rust 运行时已强制（附测试
  证据）」；B=「需 Rust conformance 显式实现」。产出 31 行盘点表（族/行号/
  语义/档位/证据入口），并逐族补充 A 档测试证据引用。验收=31 行全档位落盘、
  每行至少一个入口、档位口径一致；本步不写业务代码。
- **S2b 核心六族 Rust 显式实现**：控制票配对（control_tickets）、生命周期
  （lifecycle）、检索模式（retrieval_mode）、ledger fold
  （ledger_fold_advance / ledger_fold_write_failed）、policy denial
  （policy_denial）、failure target（failure_target）——在
  `journal/conformance.rs` 补齐机械规则族校验，fixture 正/负对拍。
  验收=orz-assurance 测试绿 + 与 Python 同族对拍 0 差。
- **S2c 其余族按档位收口**：S2a 判 A 的族完成证据引用/对拍登记；判 B 的族
  分批转 Rust conformance 显式实现（子批按域划分：检索族 / 上下文与压缩族 /
  控制面族）。验收=逐子批测试绿 + Python↔Rust 对拍 0 差。
- **S2d 全量对拍与翻转准备**：31 族全量对拍 0 差；registry 源翻转 JSON 的
  读路径验证。验收=对拍矩阵落盘。
- **S3 门禁改接**：`check_repository.py` 对真实 fixture journal 的校验改由
  Rust 法官执行（cargo test 或独立 CLI，`--repo-root` 参数化已备）；Python
  Rust 轨 `validate_journal_file` 退役；registry dict 源翻转 JSON 转正，
  Python 改读 JSON。验收=门禁 Exit 0 + 无 Python Rust 轨调用。
- **S4 收口**：`run_event_journal_validation.py` 归档/退役登记；
  `architecture/PYTHON_REFERENCE_SPEC_CONTRACT_v0.1.md` §8 契约变更流程同步；
  BACKLOG/TODO/索引状态收口 + 全量回归。验收=§0.5 检查 + 门禁 Exit 0。

### 2.2 31 族挂批表（档位以 S2a 产出为准，本表只做批次路由）

| 批 | 族（`run_event_journal_validation.py` 行号） |
|---|---|
| S2b 核心 | control_tickets (2074)、lifecycle (2977)、retrieval_mode (1129)、ledger_fold_advance (915)、ledger_fold_write_failed (1033)、policy_denial (2437)、failure_target (2579) |
| S2c-1 检索族 | result_consistency (1257)、reason_codes (1437)、source_weighting (1466)、search_candidate_pool (1520)、candidate_prefilter (1702)、candidate_count (1818)、receipt_event_isomorphism (2697)、probe_accuracy (2925) |
| S2c-2 上下文与压缩族 | recovery_truncation (1956)、context_compressed (1986)、activation_restore (2054)、dep_graph_events (2806)、tool_running (3318)、output_truncation (3478)、budget_cue_injected (3513) |
| S2c-3 控制面族 | inquiry_kind (455)、plan_write (488)、console_mode_transition (571)、console_order_written (740)、console_order_rejected (821)、mechanical_audit (1905)、tool_availability_probe (2182)、request_header (2224)、inject_budget (2346) |

注：S2b 为 7 个函数入口（6 个族 + ledger fold 双事件）；core 清单与任务 D
原文「先落地与证据面直接相关的核心族（控制票配对、生命周期、检索模式、ledger
fold、policy denial/failure target）」一致。

## 3. 本批验证与限制

- orz F1：`cargo check -p orz-tools --all-targets` Exit 0、`cargo fmt --check`
  净；resources 74 passed；read_file 技能族定向 6 passed。
- 全量 `cargo test -p orz-tools --lib` 在本窗口 48 项失败——经 stash 基线复验
  均为环境/既有问题而非本批回归：①本沙箱拒绝启动 `rg.exe`（grep/glob/list_dir
  相关测试依赖 ripgrep 子进程）；②`bash_timeout_schema_defaults_to_120s` 为
  TER 转正后未同步的旧断言（HEAD 上同样 FAIL）；③
  `read_file_allows_gsa_symlink_outside_git_root_even_when_gitignored`
  （OUTPUT-DEGENERATION-GUARD 旧回归测试）与 Task C canonical 沙箱语义
  冲突，定向复跑确定性失败，`-S`/差异复核确认早于 F1（a348901a 同逻辑），
  非本批回归。三类已记录，不虚报绿；③ 另立 GAP-GSA-SYMLINK-STALE-TEST
  （见 §5）。
- 父仓库卫生核对：`git ls-files -ci --exclude-standard` = 0；归档引用已同步；
  manifest 重新生成后门禁 Exit 0（见 §1.4/§1.5）。

## 4. 状态

- GLM 批：F1/R-1/R-2/R-3/(c) 处置完成；F2 登记完成（实施待排期，任务 D
  之外的独立 gap）。
- 任务 D：`open`（batch-1 闭合；S2a 排期起点已登记，S2b–S4 待实施）。

## 5. 复核修正批（2026-09-06 审查发现处理）

对本批（F1/F2/R-1/R-2/R-3 + 观察项 (c)）的全面复核（设计合理性 / 实现
合理性 / 设计与实现符合性）发现 6 项低优先级问题，全部收口如下：

1. **R-1 计数口径**：改为 353 个本地运行产物 + 1 个被泛化规则误中的集成
   夹具（59423ac 原样恢复）；「354」仅指当时的 tracked-but-ignored 命中
   总数。同步：§1.3、GLM 登记 §3、BACKLOG 00a、TODO、索引 §1。
2. **BACKLOG 章节引用**：§1.2/§1.6 的「BACKLOG 00」修正为「BACKLOG 00a」
   （00 = 任务 D，00a = GLM 处置）。
3. **BACKLOG 层级**：00 小节任务 D 的 S2/S3/S4 恢复为任务 D 子项缩进，
   S2a–d 为其孙项。
4. **验证限制清单补录**：全量 orz-tools lib 的既有失败除 rg 沙箱拒绝与
   bash 旧断言外，另含 `.gsa` 符号链接旧回归测试与 Task C canonical 沙箱
   语义冲突（定向复跑确定性失败；`-S`/差异复核确认早于 F1，非本批回归）。
   处理=登记 `GAP-GSA-SYMLINK-STALE-TEST`（索引 §3.1 + BACKLOG 00a/§11 +
   TODO 审计登记边界）；涉及安全语义（拒读 vs 豁免），不擅自翻转既有回归
   预期，待独立裁决：更新测试预期对齐 Task C，或为会话重解析面显式登记
   豁免。（**2026-09-06 收口**：用户裁决=对齐 Task C，旧测试改写为拒读
   安全回归测试，orz `a29f7377`，orz-tools lib 2816 passed / 0 failed 全绿；
   环境清理批另修 bash timeout 旧断言对齐 §14.55、LSP drain 预算 3s→15s
   消除负载偶发。连带观察——permission.rs `.gsa` terminal-log 白名单对
   read_file 不可达——登记 BACKLOG 00a 待裁决。）
5. **R-3 归档可复跑性注记**：`_final_smoke_2026-08-25_launcher.ps1` 内部
   保留历史绝对路径引用；`存档/` 定位为 provenance/evidence，不改写归档
   原内容（见 §1.5）。
6. **F1 SkillRoots 接线期硬化注记**（无行为变更）：白名单为 registry
   finalize 快照，动态注册晚于 finalize 不会扩大豁免（fail-closed 方向）；
   `SkillRoots` 接受构造方提供的目录型根且不校验是否含 `SKILL.md`——当前
   两构造方（orz-host 空列表 / orz-agent 构建期发现）均安全，接线期建议
   追加根有效性校验或契约注释。

- 涉及文件：本审计（§1.2/§1.3/§1.5/§1.6/§3）、GLM 登记 §3、
  BACKLOG 00/00a/§11、TODO、CLI_PROJECT_INDEX（v2.54）。
- 验收：`git diff --check` 净；入口/路径存在性核过；门禁 Exit 0。
