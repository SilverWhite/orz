# ORZ 待办项记录（TODO）

> 状态：living（实施勾选清单）；建立：2026-08-14。
> 定位：面向后续实施与改动的操作型清单，派生自 [`docs/BACKLOG_AND_PRIORITIES.md`](docs/BACKLOG_AND_PRIORITIES.md) 的未闭合项；优先级、决策门与状态权威仍是 BACKLOG，设计权威是 ADR-0010 / ADR-0011，召回路由是 [`CLI_PROJECT_INDEX.md`](CLI_PROJECT_INDEX.md)。本文件只登记指针与勾选状态，不新增设计裁决，不重复维护设计细节。
> 维护纪律：完成一项勾选一项，并同步 BACKLOG 与索引状态；新增或调整优先级只改 BACKLOG；删除或归档条目前先回查索引入口；每次改动保持本文件与 BACKLOG 的入口一致性。
> 入口：索引 AUTH-TODO（2026-08-14 登记）。
> 2026-08-31 清理轮：已闭合项压缩为单行核对条目（实施细节以 BACKLOG 变更记录与审计文档为准）；历次计数流水不再在快照重复；勾选状态以 BACKLOG 为权威，本次仅对 BACKLOG/索引已声明闭合的滞后项补勾，未闭合项原样保留。备份：`%TEMP%\TODO.md.bak-20260831`。

## 使用说明

- `[ ]` = 待办；`[x]` = 已完成（保留单行供核对，不计入开放项）。
- 每项标注 canonical ID / 优先级 / 关键内容 / 入口；同一概念只出现一次，不复制 BACKLOG 的决策记录。
- 已闭合项单行核对格式：`[x] <ID>：<一句话>（闭合日期；入口：<文档/审计>）`。

## 未闭合扫描快照（2026-08-31 清理轮）

- 未闭合总数：**28 项**（BACKLOG 计数口径，2026-08-31：阶段 3 验证闭环 38 → 32；0k S4 实机复验闭环 32 → 30；P2-11 设计轮登记不动计数；2026-09-01：P2-11 DC 强制模板轮清理闭合 30 → 29，P3「DC 硬信号 4/6」退役 29 → 28；2026-09-02：P2-12 讨论稿登记不动计数，28 不变）。TODO `[ ]` 明细含父/子项，计数以 BACKLOG 为准。
- P0：FUS-BENCHMARK-FULL-EXEC 验证②③④⑤ + 闭合（见 P0-F）；0d 后续 3/4/5 的 S4 复验（各 1，S3 已随合并批次核证闭合）；0j（W1-R1 S4 复验、W3-R3 余项×3、W4-R4 S5-2 总项 + 验证期发现）；0l（WINDOWS-HIGH-NIST-MAX-FRICTION 设计定稿、实施待放行，见 P0-0l）；0m（GSA-SESSION-VOLUME 2026-09-06 用户裁决放行，S1–S4 排期实施，见 P0-0m）。2026-09-06 补记：任务 D 已全部闭合（S2a–S2d + S3/S4 翻转，P0-GOV 00 收口）；GAP-APPROVAL-PROMPTER（0n）排期后同日延期（无具体设计文档项非急切/必需，S1 设计定稿前置，见 P0-0n）。
- P1：FUS-COMPONENT-REGISTER 组件审计；GAP-WINDOWS-EVIDENCE 三项；IMPL-DEEPSEEK-TRANSPORT DeepSeek live 晋级证据；ORZ-SESSION-CONTEXT-MONITOR 四项。
- P2：IMPL-CONTROL-FABRIC（Slice 3 / Slice 4 / 可选）；OPS-PROTOCOL（裁剪设计 + 生产接线裁决）；MODEL-RESIDUAL-PRESSURE-FOLLOWUP 四项（P2-11）；COMPRESSION-LINGUISTIC-FORMAL-LAYER（P2-12，2026-09-02 登记：域标注 + 失败目标聚合 + 建构暂缓，设计/实施待放行）。
- P3：EVIDENCE-LOCAL-BROWSER、GATE-CHAIN、observed-scope 枚举、V11-IMPL-003、V11-IMPL-007、orz-host flaky（DC 硬信号已随 P2-11 退役）。
- 审计登记边界（条件触发，不占当前优先级）：orz-host 可选后端、headless 计划信号、23 工具分区 journals、B-1 后续、ORZ-RECOVERY-TOOL-OUTCOME、ORZ-STAGNATION-TOOL-SIGNAL。
- 已闭合分组（单行核对见下）：P0-E、0c、0d 主项与后续 1/2/6/7/8、0e、0f、0g、0h、0i、0k、P0-B、P0-C、P0-C2、P0-D、P1 已闭合项、P2-10 全部闭合。

## P0 — 当前工作集

### P0-GOV 全项目宏观架构对齐与门禁修复（最优先阻断项，2026-09-04 登记）

> 入口：[首轮审查报告](docs/audits/GLOBAL_ARCHITECTURE_AND_INTEGRITY_AUDIT_2026-09-04.md) / [深层审查报告](docs/audits/GLOBAL_ARCHITECTURE_DEEP_AUDIT_2026-09-04.md)；BACKLOG 00；AUTH-GLOBAL-ARCHITECTURE-AUDIT。
> 来源：2026-09-04 本地实测、Grok 4.6 架构审查与深层源码穿透。在继续处理 TER M1/M2 审查暴露的问题之前，必须先将本项全量闭合。

- [x] 门禁断链修复：修正 `docs/CONTEXT_COMPACTION_BLACKBOARD_FOLD_DESIGN_2026-09-04.md:246` 与 `docs/audits/P2-13_B1_CONVERSATION_BASE_IMPL_AUDIT_2026-09-03.md:92` 两处相对路径死链。（2026-09-04 闭合）
- [x] 门禁夹具补齐：在 `scripts/check_repository.py` 登记 3 个未映射的 run-event v0.2 payload 夹具（`tool-completed.output-object.valid.json` 等）。（2026-09-04 闭合）
- [x] 源码指纹重算：运行 `python scripts/generate_orz_source_manifest.py`，更新 `orz_source_manifest.sha256`。（2026-09-04 闭合：1434 文件）
- [x] 消除 Rust 告警：修复 `host_exec.rs:187` `run_host_tool` dead_code 及 `local_browser/mod.rs:670` unused assignment。（2026-09-04 闭合：`cargo check -p orz-bin` 0 warnings）
- [x] 门禁验收全绿：运行 `python scripts/check_repository.py` 退出码验证为 0（PASS，error_count=0，valid=true）。（2026-09-04 闭合）
- [x] 根目录卫生治理：彻底删除根目录 31 个临时调试目录（`tmp*`）及遗留调试文件（`HTTP`, `%{http_code}`, `_review_lif_replay_check.json` 等）。（2026-09-04 闭合）
- [x] 仓库日志收敛：更新 `.gitignore` 过滤 `_windows_high_nist/**/job-*`、`vm-*-result*.txt`、`diag-*.txt`、`evidence-*` 与 `.t23tmp/` 等本地测试输出。（2026-09-04 闭合）
- [x] 权威与实现对齐：重写 ADR-0010 导言与主 README，消除过时的 120 轮及 plan-epoch 轮换描述，在 architecture/current/ 扩充真实产品面架构投影（8 工具直调、会话黑板单实例、无硬杀）。（2026-09-04 闭合）
- [x] 全工作区编译摸排：运行 `cargo check --workspace` 验证 64 个 workspace members 零告警零错误。（2026-09-04 闭合：耗时 1m 38s）
- [x] 任务 A（解耦寄生代码）：创建 `render_fold.rs`，将生产折叠渲染与黑板压缩快照从 `epoch.rs` 剥离，切断对 epoch 状态的依赖；`epoch.rs` 保留 legacy plan-epoch 归档/`--plan` 支持（现 1970 行）与兼容重导出，不参与会话黑板生产折叠主链。（2026-09-04 闭合：render_fold 净移入 1338 行，epoch 净减 1363 行；723→725 个单元测试 + workspace check + fmt 全绿）
- [x] 任务 B（清理僵尸 Crate）：在 `orz/Cargo.toml` 中剔除未被 `orz-bin` 引用的 15 个无头 Crate，加速构建并净化审计面。（2026-09-04 闭合：剔除 15 个无头 Crate，工作区成员 64 → 49，全工作区 cargo check 零警告零错误，门禁 Exit Code 0 全绿）
- [x] 任务 C（路径沙箱与 ACAF 下沉）：读工具（`read_file`/`grep`/`list_dir`）统一建立 CWD 工作区 canonical 级越界硬拦截——`..` 越级、绝对路径指向 cwd 外、以及工作区内符号链接/重解析点指向 cwd 外均拒绝（skills 文档豁免；目标不存在时回退词法判定）；ACAF fail-closed 默认强校验下沉至 `AgentLoopController`（ACP/TUI 默认 fail-closed、`ticket_flow` 无 signer 即拒、`ORZ_ACAF_FAIL_CLOSED` 解析单源化并可显式逃生）。（2026-09-04 闭合：orz 提交 + 12 项沙箱单测 + orz-loop 725 + orz-bin env 单测 + 全工作区 check/fmt 绿，见 [P0-GOV 收口审计](docs/audits/P0_GOV_UNCOMMITTED_REVIEW_HANDLING_2026-09-04.md)）
- [x] 任务 D（双实现终局治理）：在 Rust `orz-assurance` 补齐规则，逐步退役 Python `assurance` 双重法官。**2026-09-06 全部闭合**——终态 = Rust 单一执法（`journal/conformance.rs` + `journal-conformance` CLI）+ Python 冻结 reference（对拍对照面 + `_WORK_TOOLS` 单源，方案 α）+ 对拍长期回归。（batch-1 2026-09-04，S2a–S2d 2026-09-06，S3/S4 翻转 2026-09-06；入口：[Task D 批次 1 审计](docs/audits/P0_GOV_TASK_D_DUAL_IMPL_GOVERNANCE_2026-09-04.md) / [S3/S4 翻转实施审计](docs/audits/TASK_D_S3_S4_FLIP_IMPL_AUDIT_2026-09-06.md)）
  - [x] batch-1（2026-09-04）：Rust `journal/conformance.rs` schema 级离线法官
    （envelope + 按轨 payload + raw-JSON 哈希链 + 整刊单轨）+ 18 fixture
    全量对拍 + 6 类篡改负测；`runtime/run-event-payload-registry-v0.1.json`
    单源映射 + 导出脚本 + 门禁同步钩子；实测并消解 `runtime_stagnation_guard`
    历史 v0.1 回放漂移面（离线链按 raw JSON 重算）。
  - [x] S2：31 个 `_verify_v02_*` 机械规则族盘点为「Rust 已强制（附证据）/
    需 Rust 显式实现」两档并补齐——**2026-09-06 全部子批闭合**
    （S2a/S2b/S2c/S2d；S2d 批 1 后族数 31→30）。
    - [x] S2a（盘点先行，2026-09-06 排期起点）：31 族两档盘点表 + A 档证据
      引用（不写业务代码）——**2026-09-06 完成**：27 A + 4 B，B 族=
      receipt_event_isomorphism / probe_accuracy / console_order_written /
      console_order_rejected，盘点表见
      [S2a 盘点审计](docs/audits/TASK_D_S2A_INVENTORY_2026-09-06.md)；
      挂批路由见
      [GLM 处置 + S2 排期审计](docs/audits/P0_GOV_GLM_DISPOSITION_AND_TASK_D_S2_SCHEDULE_2026-09-06.md)。
      三路全面复审无 P1/P2、9 项 P3 全部收口（含 orz `f45a9e39` 注释修正），
      见 [S2a 复审处理](docs/audits/TASK_D_S2A_REVIEW_HANDLING_2026-09-06.md)。
    - [x] S2b：核心族 Rust conformance 显式实现（control_tickets /
      lifecycle / retrieval_mode / ledger_fold advance+write_failed /
      policy_denial / failure_target）+ fixture 正/负对拍——**2026-09-06
      完成**（orz `809cdb4e` + 复审处理批 P1×2 修复；对拍 0 差 546 裁决格
      78 语料；60 场景表驱动单测；orz-assurance 204 passed），见
      [S2b 实施审计](docs/audits/TASK_D_S2B_FAMILIES_IMPL_AUDIT_2026-09-06.md)
      / [S2b 复审处理](docs/audits/TASK_D_S2B_REVIEW_HANDLING_2026-09-06.md)。
    - [x] S2c：其余族按档位收口（检索族 / 上下文与压缩族 / 控制面族子批）
      ——2026-09-06 完成 + 同日三路复审收口（orz `195c71b8` + `a7981654`，
      families_s2c.rs 24 校验器；230 场景×31=7130 格 + 对拍 7626 格 0 差
      （语料计数勘误：真实 248，见批 1 实施审计 §2）+ S2c e2e 负测；
      入口：docs/audits/TASK_D_S2C_FAMILIES_IMPL_AUDIT_2026-09-06.md
      / docs/audits/TASK_D_S2C_REVIEW_HANDLING_2026-09-06.md）。
    - [x] S2d：全量 Python↔Rust 对拍 0 差 + registry 翻转准备——
      **2026-09-06 收口**（零代码改动：spec 表 233×30=6990 格（186 负例格
      30 族全覆盖）+ 对拍 251 语料×30 族=7530 格 0 差新鲜实测 + Python
      判官 18 fixture 直扫 0 错误/540 族格 0 firing；registry 翻转触点
      盘点 + S3 三步序列/回滚方案登记，翻转执行属 S3 待放行；入口：
      docs/audits/TASK_D_S2D_CLOSURE_2026-09-06.md）。
      翻转前裁决清单两项已定案（2026-09-06 用户裁定），落地
      批次见 [S2d 翻转裁决登记](docs/audits/TASK_D_S2D_FLIP_ADJUDICATION_2026-09-06.md)：
      - [x] 批 1：console_order_written 规则退役 + console_order_rejected
        去摩擦留形状——**2026-09-06 完成 + 同日三路全面复审收口**（实施
        orz `362b6071`：ADR-0010 §14.57 转录 + 判官两侧同步，31→30 族；
        复审处理 orz `141bd2cf`：P1×1 父仓库判官测试面同步修复 258 passed、
        P2×2 登记修正（rejected 发射点运行时休眠 / 复活边界
        content_anchor_mismatch 码表）、P3 采纳×5（+3 场景 + written 守卫
        显式断言 + 注释同步）；spec 表 232 场景×30 族=6960 格 + 对拍 250
        语料×30 族=7500 格 0 差；入口：docs/audits/TASK_D_S2D_BATCH1_IMPL_AUDIT_2026-09-06.md
        / docs/audits/TASK_D_S2D_BATCH1_REVIEW_HANDLING_2026-09-06.md）。
      - [x] 批 2：probe_accuracy 收窄至当前可见工具——**2026-09-06 完成**
        （tool_probe 记账口径收窄：narrow_to_declared + run-start/逐轮两
        装配点接入，封存工具不进 complete/incomplete，翻转留痕点封堵；
        ADR-0010 §14.58 转录；判官两侧字面不动、对拍 250 语料×30 族 0 差
        零回归；orz-loop 728 lib 全绿；orz `00b9a440`；入口：
        docs/audits/TASK_D_S2D_BATCH2_IMPL_AUDIT_2026-09-06.md）。
        同日三路复审 + 用户补裁决收口（P1×1 tool_availability_probe 族
        exact-partition 子句收窄为「分区 ⊆ WORK_TOOLS 且两集互斥」、
        §14.59 转录、两侧判官同步；P2 消费点五处枚举修正；P3 采纳×3
        登记×4；orz `1595303b`：spec 表 233×30=6990 格 + 对拍 251×30=
        7530 格 0 差 + 729 lib；入口：docs/audits/TASK_D_S2D_BATCH2_REVIEW_HANDLING_2026-09-06.md）。
        两批完成，S2d 翻转前裁决清单清空。
  - [x] S3：`check_repository.py` 真实 fixture journal 校验改接 Rust 法官，
    Python Rust 轨 `validate_journal_file` 退役，registry 源翻转 JSON 转正
    ——**2026-09-06 实施**（裁决：独立 CLI + v0.1 纳入 + D-1=α）：registry
    JSON 升唯一权威 + dict 改导入时派生视图（基线等值 IDENTICAL）+
    orz-assurance `journal-conformance` CLI（+4 集成测试）+ 门禁双轨
    18 期刊循环改接 + Python 门禁调用摘除。入口：
    docs/audits/TASK_D_S3_S4_FLIP_IMPL_AUDIT_2026-09-06.md。
  - [x] S4：Python 双法官退役/归档登记、契约变更流程同步、索引/BACKLOG
    收口——**2026-09-06 收口（方案 α）**：模块头部 RETIREMENT STATUS +
    258 测试保留 + 导出脚本退役删除 + 契约 §9 改写；Python 三套件
    281 passed、篡改负测 9 错/原刊 0 错、门禁 Exit 0。入口：
    docs/audits/TASK_D_S3_S4_FLIP_IMPL_AUDIT_2026-09-06.md。

### P0-GOV GLM 外部只读审查处置（2026-09-06 用户裁决）

- [x] F1 skills 豁免收窄为注册技能根白名单（orz `67b51eb1`；SkillRoots +
  registry finalize 派生 + read_file/grep/list_dir 接线 + 负测）。
- [x] R-1：353 个已跟踪本地运行产物转本地件（`git rm --cached`，磁盘保留；
  另 1 个误中夹具随 59423ac 恢复跟踪）。
- [x] R-2：manifest 生成器显式 LF 并重算。
- [x] R-3：9 个根目录一次性产物归档 `存档/root-artifacts-2026-09-06/`
  （gsa.py 保留，门禁 required 文件）。
- [x] F2：approval prompter 存根登记为 GAP-APPROVAL-PROMPTER（已登记；
  2026-09-06 排期（S1–S4 见 P0-0n）后同日延期，S1 设计定稿前置）。
- [x] 观察项 (c)：权限判定分散登记 OBS-PERMISSION-DUAL-IMPL（另立观察）。
- [x] 复核修正批（2026-09-06）：R-1 口径（353 + 夹具恢复）、BACKLOG 00a
  引用修正、任务 D S2–S4 层级修正、处置审计 §3 限制清单补录
  （GAP-GSA-SYMLINK-STALE-TEST 另立）。
- 入口：[GLM 登记审计](docs/audits/GLM_EXTERNAL_REVIEW_REGISTRATION_2026-09-04.md)
  / [处置 + S2 排期审计](docs/audits/P0_GOV_GLM_DISPOSITION_AND_TASK_D_S2_SCHEDULE_2026-09-06.md)。

### P0-E 评测冒烟暴露问题（2026-08-17 登记；2026-08-18 全部闭合）

- [x] 全部闭合：ACAF 容器内供应 / console 工具名下划线 / plan_write 校验消息形状 / actions 形状探针锁定 / 计划视图步骤 ID / 订单发放前拒绝入事件面 / grep 搜索范围契约（结构化信封 + 结局三型 + hidden/no_ignore + 静态 rg）/ list_dir 范围计数 / grep files_searched 全结局探针。入口：BACKLOG 0a / ADR-0010 §14.21/§14.23 / 对应实施审计（GAP_ACAF_HARNESS_PASSTHROUGH、GAP_CACHE_CONTEXT_COST 等）。

### P0-F FUS-BENCHMARK-FULL-EXEC（`pending`=实施完成待验证；P0，2026-08-18 用户裁决实施）

> 入口：[设计](docs/BENCHMARK_FULL_EXEC_DESIGN_2026-08-18.md)；ADR-0010 §14.24；BACKLOG 0b。orz 子模块 3f43478 / 4e9e61b。

- [x] 实施完成（2026-08-18）：权限层 Benchmark 两轴参数化 + 探针 BenchmarkFull + console `workspace.run_terminal` 注册 + CLI `--allow-shell`/`--allow-network` + `tb_agents/orz.py` 透传 + 审查收口；验证① 全量测试通过（含修复既有测试漂移 c4772fc）；折叠 400 根因修复 S1-S5 闭合（处理文档 LEDGER_FOLD_MARKER_INDEX_FIX_HANDLING）。
- [ ] 验证②（用户指示暂缓）：Linux musl 重建（ORZ-BUILD-MOUNT-001 契约，输出 `D:/tb-eval/orz-linux`）。
- [ ] 验证③（2026-08-18 修复后复验一次，机制断言全过、reward 项未达标，待加预算重跑）：单题 make-doom-for-mips——0 异常、无 400、6 笔订单→5 组票据零拒绝、压缩后会话继续；reward 0=墙钟内未产出可运行 ELF（非机制回归）。
- [ ] 验证④（用户指示暂缓）：2–3 题交叉（compile-compcert、hf-model-inference 等 build/run 与网络类）。
- [ ] 验证⑤（用户指示暂缓）：`run_official_2.1.sh` 89 题 5 批。
- [ ] 闭合：验证全过 → BACKLOG/TODO/索引状态同步，未闭合 27 → 26。

### P0-0c LEDGER-FOLD-EXTERNAL-FILE（S1-S4 全部闭合 2026-08-19，计数 29 → 28）

- [x] 全部闭合：外挂台账文件 + 固定指针消息 + 写失败降级（ledger_fold_write_failed 事件）+ B 定案（机械压缩零模型调用）+ D1=(c) HA 结构化事实聚合 + 黑板读取缓存成本（receipt_id 按需点读）+ 折叠桥接截断；S4 复验 provider 口径命中率 95.33% ≥90%、零 400、重付/截断达标。入口：[设计](docs/LEDGER_FOLD_EXTERNAL_FILE_DESIGN_2026-08-18.md) / ADR-0010 §14.28–§14.32 / BACKLOG 0c。

### P0-0d OUTPUT-DEGENERATION-GUARD（S1-S4 全部闭合 2026-08-20）

- [x] 全部闭合：make-doom 退化复读失败防护——8K 全统一 + 补读闭环 + 实时检测 + 32K；S4 复验 95.28% 命中率、哨兵零误杀。入口：BACKLOG 0d / ADR-0010 §14.33。

### P0-0d 后续：STREAM-RETRY-RHYTHM（2026-08-25 取代归档）

- [x] 原定案被后续 3（180s 窗口）+ OUTPUT-BUDGET（idle 50s→30s）取代，归档不实施。入口：[STREAM_RETRY_RHYTHM_DESIGN](docs/STREAM_RETRY_RHYTHM_DESIGN_2026-08-20.md) / BACKLOG 0d。

### P0-0d 后续：OUTPUT-BUDGET-RESTORE-AND-STALL-GUARD（S1-S4 全部闭合 2026-08-20，29 → 28）

- [x] 全部闭合：`REQUEST_MAX_TOKENS` 32K→256K（回落 128K）+ D-6 空流链官方化收窄 + 输出健康哨兵（content/reasoning/tool、reasoning 复读、stall 600s/64K、idle 30s）+ DEGENERATION_LIMIT=3 三族共享；S4 复验命中率 ≥90%、零误杀。入口：[设计](docs/DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md) / ADR-0010 §14.35 / BACKLOG 0d。

### P0-0d 后续 2：THINKING-DEFAULT-HIGH-LADDER（S3/S4 换题复验闭环 2026-08-20，29 → 28）

- [x] 全部闭合：默认 high + low 中间档降级梯（high→low→disabled→失败，max 保留显式档）；S4 换题 make-doom-for-mips 复验（零异常、零 400、命中率 92.18%、哨兵/stall 全零触发、每轮更快成本更低）。入口：设计 §3.6/§4.4–§4.7 / ADR-0010 §14.35 第 5–8 项 / BACKLOG 0d。

### P0-0d 后续 3：ZERO-CHUNK-RETRY-WINDOW-180S（P0 派生；2026-08-21 定稿）

> 入口：[设计修订](docs/STREAM_RETRY_RHYTHM_DESIGN_2026-08-20.md) / ADR-0010 §14.36 / BACKLOG 0d。
> 定案：`request_retry_window` 50s → **180s**（`request_max_retries` 10 不变，双上限先到者止）；非流式 create 退避窗口同步放宽；retry 参数参与请求头指纹。

- [x] S1/S2 实施闭合（2026-08-21）：窗口/退避/指纹落地 + 测试。
- [x] S3 重建——**2026-08-25 核证闭合**：随 0g/0h/0.1.0 发布构建轮合入 Linux musl 三件套（a96faab，merge-base 祖先核证）。
- [ ] S4 复验（无 400、命中率 ≥90%、断连窗口内可骑过节点抖动）。

### P0-0d 后续 4：STALL-DEGENERATION-FAILFAST（P0 派生；2026-08-21 定稿；S1/S2 闭合 2026-08-21、S3 闭合 2026-08-25、S4 待续）

> 入口：[设计](docs/STALL_DEGENERATION_FAILFAST_DESIGN_2026-08-21.md) / ADR-0010 §14.37 第 1 项 / BACKLOG 0d。

- [x] S0 证据门通过 + S1/S2 实施 + 全面审查处理闭合（2026-08-21）：会话级 thinking 档位（哨兵后不回 high）+ 哨兵计数单调（成功不清零、达 3 run_invalidated）+ disabled 档即终止 + `ModelGateway::for_new_run()` 每 run 隔离 + v0.2 `transport_retry` 事件面；orz-loop 544 / conformance 230 通过。
- [x] S3 重建——**2026-08-25 核证闭合**：与窗口 180s + 解码兜底批次合并（a0d85f8，随 0g/0h/0.1.0 构建轮覆盖）。
- [ ] S4 复验（单 run 哨兵预算有界 ≤3 次触发 × 单次预算、显式终止可观测、命中率 ≥90%、零 400）。

### P0-0d 后续 5：MIDSTREAM-DECODE-RETRY（P0 派生；2026-08-21 定稿；S1/S2 闭合 2026-08-21、S3 闭合 2026-08-25、S4 待续）

> 入口：[设计](docs/MIDSTREAM_DECODE_RETRY_DESIGN_2026-08-21.md) / ADR-0010 §14.37 第 2 项 / ADR-0007 修订注记 / BACKLOG 0d。

- [x] S1/S2 实施闭合（2026-08-21）：重试判定「无完整 tool_calls」（`wrap_no_tool_side_effects` + `has_complete_tool_call` 双保险）+ 中段有界 1 次（`CHUNKED_MIDSTREAM_MAX_RETRIES`）+ `StreamInterrupted.saw_chunk` + `transport_retry` 事件面（run-event enum 53→54、schema/fixtures/conformance/TUI 同步）。
- [x] S3 重建——**2026-08-25 核证闭合**：与窗口 180s + fail-fast 批次合并（同后续 3 S3 注）。
- [ ] S4 复验（dna 类场景不再因解码错误杀 run、零 400、命中率 ≥90%）。

### P0-0d 后续 6：REPETITION-DETECTOR-ROLLING-HASH（S1-S4 全部闭合 2026-08-23，计数 29 → 28）

- [x] 全部闭合：路径①替换为滑动窗口滚动哈希任意偏移检测（144 字符缓冲、任意周期命中、DNA 正常序列免疫、O(1)/字符）+ 3-gram 路径②兜底；S3 重建 + S4 dna-assembly 复验闭环（低熵误杀消除、真复读仍触发、零 400）。入口：设计 §3.3/§4.8 / ADR-0010 §14.35 第 13 项 / BACKLOG 0d。

### P0-0d 后续 7：AGENT-DELIVERY-FLOW（S4 复验闭环 2026-08-23，计数 29 → 28）

- [x] 全部闭合：计划无空转 + 末步机械递交（submit 双阶段 requested→confirmed）+ 引用修正一次/二次阻断 + 订单反馈 receipt 点读链；S4 复用 NGRAM S4 实机复验（8/8 完成试次走 submit 双阶段、零 400、命中率全 ≥90%）。入口：ADR-0010 §14.35 第 19 项 / BACKLOG 0d 后续 7。

### P0-0d 后续 8：NGRAM-GUARD-CALIBRATION（S4 复验闭环 2026-08-23，计数 30 → 29）

- [x] 全部闭合：3-gram 门槛 3→15 校准 + 复读滚动哈希边界对齐；S4 实机复验闭环。入口：BACKLOG 0d 后续 8 / 设计 §4.11。

### P0-0e CONTEXT-SCAFFOLDING-PULL-REDESIGN（S1-S4 验证闭环 2026-08-21，29 → 28）

- [x] 全部闭合：预算块 PUSH→PULL + 工具输出汇总消息退役（方案 C 维持 256K 暂不收紧，用户裁决）；命中率 94.45%、零哨兵触发、输入增长放缓。入口：ADR-0010 §14.33 / BACKLOG 0e。

### P0-0f FUS-READ-ANCHOR-WRITE-GUARD（S4 复验闭环 2026-08-23，计数 28 → 27）

- [x] 全部闭合：read_file 内容锚点下传（sha256/size/mtime）+ search_replace 写前机械核证（content_anchor_mismatch 拒绝、重读后重试）；S4 复用 NGRAM S4 实机复验（10 试次零误拒、锚点实机可见、命中率 94.11%–98.55% 全 ≥90%、零 400）。入口：ADR-0010 §14.38 / BACKLOG 0f。

### P0-0g MECHANICAL-AUDIT-LAYER（S4 复验闭环 2026-08-25，计数 31 → 30）

- [x] 全部闭合：首轮 plan 门保留 + direct 执行面 + 半助理层 + 静默机械审查层（每对象仅最后一轮结果覆盖写、不给建议、报告随最终答案前中立问询轮注入）+ 检索恢复 + 引用校验器删除 + 读范围放开。入口：ADR-0010 §14.39 / BACKLOG 0g。

### P0-0h RETRIEVAL-SUBAGENT-WIRING（S4 复验闭环 2026-08-25，计数 30 → 29）

- [x] 全部闭合：外部子代理模式 A 自动定档 + 内部子代理结构化检索外包（retrieve_project_docs 触发面）+ prompt tips + harness 传参；S4 单道检索题 mteb-leaderboard k=1 reward 1.00、事件链完整性 100%、零 400。入口：ADR-0010 §14.46 相关 / BACKLOG 0h。

### P0-0i FINAL-SMOKE-2026-08-25 对拍暴露问题（2026-08-25 登记；2026-08-26 全部闭合）

- [x] GAP-EVENT-SCHEMA-DRIFT：三类 Schema 漂移修复完成并复验（retrieval_mode_transition 枚举补全、ledger_fold_advance view_estimate_after、control_ticket_issued activation_id 放开；事件链复验 5 run 非终止错误 0）。入口：BACKLOG 0i / 实施审计。
- [x] GAP-REPETITION-DETECTOR-DNA-FALSE-POSITIVE（S1-S4 全部闭合 2026-08-26，30 → 29）：序列内容门（L=400 维持 + 无切分点 sequence_kind 判定 + 序列/蛋白双族独立阈值 + 命中 3→5）；S4 复验闭环（EGFP 式合法引用 1–4/5 命中零误杀仅审计、真复读 5/5 仍触发降 EnabledLow、零真实 400、命中率 82.36% 持平）。观察项：dna 82.36% / feal 88.21% 命中率 <90% 与 web 检索注入相关，成本观察不阻塞（并入对拍审计记录）。入口：BACKLOG 0i / 序列内容门设计 / ADR-0010 §14.41。

### P0-0j THIN-HARNESS-REDESIGN-V2（P0；2026-08-28 设计定稿，实施进行中）

> 入口：[设计](docs/THIN_HARNESS_REDESIGN_V2_DESIGN_2026-08-28.md) / [HA 调研](docs/HA_SERVICE_MODEL_RESEARCH_2026-08-28.md)；BACKLOG 0j。
> 定案摘要：复读门槛统一 20 + 序列门全删 + 3-gram 15 + 802 保留 + 空响应链 low 封顶 + 触发显式拦截不降档 + 审计结构化字段 + 半助理层加厚（诊断/实体/黑板）+ HA 服务模型；2026-08-29 S4 归因后补定案：prompt 全空 + orientation 软门 + submit 无 plan 放行/降级（W4-R4）+ S5-1（fold 桥 reasoning + web_search 120s）+ S5-2（终端分层超时 + 中间回报）。

- [x] W1-R1 S1 代码 + S2 测试 + S3 重建（2026-08-28 完成）：复读/3-gram/空流链/审计结构化全量落地（orz-loop 567、容器冒烟三件套 orz 6cc8586）。
- [ ] W1-R1 S4 复验：EGFP / sam-cell-seg 真实 span 回放静默、构造真循环触发、零真实 400、命中率 ≥90%。
- [x] W2-R2 失败诊断 + 实体登记 + 服务调用形态收敛 + 黑板接线 + 全面审查处理（2026-08-28 完成）：diagnostics 签名词典 ≤2KB、entities 三域注册表、target 实体级、entities 分区；orz-loop 608/0/3、pytest 216。
- [ ] W3-R3 HA 目标架构落地余项：实体 id 形态、分区命名、注册表 Rust 形态（含实体 id 相对/绝对/大小写归一——本轮仅落地无状态部分：分隔符与 `./` 前缀；process/environment 状态探针扩展入账 R3）。
- [ ] W3-R3 A/B 验证：小样本跑分（含 THIN-HARNESS v0.4 R2b 按需读取观察；R2c `parse_retrieval_result_json` 兜底回收判定已随 2026-08-30 方向 C 裁决物理删除，不再观察——见 ADR-0010 §14.45）。
- [ ] W3-R3 清理与登记：旧序列门文档标记 withdrawn、ADR-0010 减法修订、CLI_PROJECT_INDEX 登记（含 BACKLOG/TODO 计数入账）。
- [x] W4-R4 prompt 全空 + orientation 软门 + submit 门修复（2026-08-29 完成）：BASE_SYSTEM_PROMPT 置空、orientation 软门（阈值 50、触发轮不禁工具）、submit 无 plan 降级状态展示。
- [x] W4-R4 S2 测试 + S3 重建 + S4 复验——**2026-08-31 按最低口径判定闭合**：S2/S3 完成 2026-08-29（orz-loop 611/0/3、三件套 5b3fe27）；S4=31 题已解 9（08-29 复验 8 + 10 题小批 rstan-to-pystan 1），未复验题不新增计数、全量成绩不再外推；8 工具面冻结不再删除、只做通用修正不为跑分特化（用户裁决）。
- [x] W4-R4 S5 修复 + S5-1（2026-08-29 完成，orz ad5f9ee）：A=fold 桥保留纯文本 assistant 消息 reasoning_content；B=orientation 触发轮放行工具（DC 强制模板轮仍禁工具）；web_search 客户端总超时 120s + connect 10s + 结构化 Timeout。
- [ ] W4-R4 S5-2 终端分层超时 + 中间回报（2026-08-29 用户裁决，独立批）——普通命令默认 300s / 程序脚本类 600s（模型可传 timeout 覆盖、上限 900s）；运行满 300s 未完成 → 机械插入一次「运行 + 工具自身情况」中间状态（单次仅一次），回报后默认继续、模型可主动中断；后台路径=终端 actor 自动后台化（满 300s 且解析超时 >300s 才后台化，后台截止=原解析超时）；事件面=`tool_running`（v0.2）+ ToolCompleted `running: true`。S1 代码 + S2 测试 + 全面审查处理已闭合（2026-08-29，见下）。
- [x] W4-R4 S5-2 S1 代码 + S2 测试 + 全面审查处理 + 审查处理补充（2026-08-29 完成）：宿主分类注入 + 自动后台化报告 + actor 后台截止 + `tool_running` 事件面（schema/verifier/fixtures）+ console 订单面同步（移除 is_background、timeout 上限 900s、默认 600s）；orz-tools/host/loop/tui/assurance 全绿、pytest 236。
- [ ] W4-R4 S5-2 验证期发现（2026-08-29 登记，S5-1 遗留回归，独立排查）：orz-bin `acaf_e2e` 7 项失败（`controller_control_events_carry_tickets` / `fail_closed_continue_consumes_goal_revision_ticket` / `fail_closed_goal_revision_rejected_does_not_migrate` / `fail_closed_web_search_executes_unticketed_with_zero_ticket_events` / `goal_revision_continue_flow_re_derives_session_key` / `missing_browser_read_url_refuses_before_acaf_with_count_gate` / `signer_unreachable_shadow_records_rejection_and_proceeds`），16 通过。归因链：S5-1 提交 ad5f9ee 引入的 ACAF 控制事件/disposition/票据域遗留回归（与 S5-2 改动路径不相交），需独立轮次定位（建议先核对 disposition 处理链与 orientation 触发轮交互）后再进 S3/S4。
- [x] CONTROLLER-SPLIT 二轮（2026-08-30 全部闭合）：N1-N5 全部分批完成（controller.rs 29,091 → 4,142 行，测试区 202 项按主题归位，验收 ≤10,000 行达成；每批独立提交 + 全量回归）。

### P0-0k RETRIEVAL-ORCHESTRATION-MECHANICAL（P0；2026-08-30 定稿；第一批 + 第二批实施 + S4 实机复验闭环 2026-08-31）

- [x] 全部闭合（2026-08-31）：双模式定案（local_browser 可用仅 browser_read / 不可用仅 web 族）+ 引擎 SERP Google 主序 + 原生兜底 + 第一批五项（S1/S2 + S3 重建 + S4 实机复验）+ Google 门禁观察实验（多轮实机）+ 主面封存 browser_read + 第二批（project_doc_index v2 / 会话级 tab 池 + 同轮多页并行 + DNS 缓存 / 委托契约复杂度分档）S1/S2 + 方向 C（删除 [RESULT_JSON] 组织块契约）S4 实机复验闭环（未闭合 32 → 30）。入口：BACKLOG 0k / ADR-0010 §14.45/§14.46 / S4 复验记录。

### P0-0l WINDOWS-HIGH-NIST-MAX-FRICTION（P0；2026-09-01 设计定稿，实施待放行）

> 入口：[设计](docs/WINDOWS_HIGH_NIST_MAX_FRICTION_DESIGN_2026-09-01.md)；
> BACKLOG 0l；索引 AUTH-WINDOWS-HIGH-NIST-MAX-FRICTION。

- [x] 设计定稿落盘（2026-09-01）：N×F×P 三轴格 → Win32 原语映射 + 11 类
  Windows 特有摩擦点 + 承载方案（硬化 Windows VM 主载 / Linux arm 干跑）+
  摩擦探针与真实任务子集三臂 + 6 项可证伪缺口判据 + 8 项预期缺口假设。
  设计轮登记不动计数。
- [x] ① Linux arm 干跑（2026-09-01 闭环）：BoundaryBench 模式移植到现有
  Harbor 管线方法学验证——三臂 control/non-root/high-nist 12/12 reward=1.0、
  0 异常；enforcement-probe 三臂先验墙全过；OS 通道记账 non-root epErm×1 /
  high-nist eroFS×1；真实任务三臂同分。干跑记录见
  `_linux_arm_dryrun/PREP_RECORD_2026-09-01.md`。
  - [x] ② Windows 加固脚本 + enforcement-probe（S1/S2 + 全面审查处理完成
    2026-09-01；**S3 重建 + S4 本机冒烟闭环 2026-09-01；硬化 VM 三臂
    enforcement-probe 实机闭环 2026-09-02**）：`_windows_high_nist/`
    加固脚本（三臂模板、-Revert、日志）+
    enforcement-probe（每轴断言集）+ `windows_sandbox.py` 运行环境扩展
    （受限 token / LOW IL / AppContainer / Job / TEMP 重定向下 spawn orz
    命令树）+ run observation schema/verifier + CLI 入口；S2 全绿
    （assurance 新增 30 测试，46 passed 含既有）；S3=Windows x86_64
    三件套重建（orz f0eeb524，12m15s）+ 守卫符号核验；S4 本机冒烟=
    sandbox control 臂端到端 compliant + 冒烟修复 3 项
    （ProcThreadAttributeList 查询大小误判 / 管道 drain c_void_p 句柄 /
    CLI --command REMAINDER）；35 passed + 全量 1624 passed；
    **2026-09-02 硬化 VM 三臂实机闭环**=control/non-admin/high-nist 全
    PASS（non-admin 10/10、high-nist 19/19、sandbox observation
    compliant），修复链 6 项（SYSTEM 持久任务提权 / CPAU 选型与 CPTW
    回退 / session0 桌面 ACL / AppContainer TEMP+LOW+包目录 / run-user
    NTUSER.DAT 冻结 / 探针宿主 Python 化），案例库新增 6 篇 ORZ-WIN-*
    （进度见 `_windows_high_nist/S4_PROGRESS_2026-09-02.md`）。
- [x] ③ 三臂正式序列固化（2026-09-02 闭环：control 基线快照 2/2 /
  non-admin 10/10 / high-nist 19/19（AppLocker 恢复）全 PASS；修复驱动
  输出流误判、apply_hardening Get-ProtectedPaths SYSTEM profile 根、
  sandbox LoadUserProfileW 缺 UnloadUserProfileW；证据
  `_windows_high_nist/formal-2026-09-02/`，详见
  S4_PROGRESS_2026-09-02.md §10）。
- [x] ④ 任务集（首批 2 摩擦探针 + 1 真实任务 log-summary-date-ranges
  + verifier）→ control 臂基线（k=1）——2026-09-02 闭环：3/3
  attempt=success、observation=compliant、verifier 全 PASS，证据
  `_windows_high_nist/formal-2026-09-02/evidence-task-control/`；
  模型侧 agent k=1 依赖 ⑦ 网络/凭据后与 ⑤ 合并。详见
  S4_PROGRESS_2026-09-02.md §11。
- [x] ⑤ high-nist 小批（2026-09-02 机器侧闭环：驱动新增 tasknonadmin /
  taskhighnist 自包含 stage——基线恢复 + 模板加固 + 墙探针先验 + 任务批
  按臂落盘；high-nist 臂 3/3：墙探针 19/19、两个写探针 attempt=denied
  （WinError 5）、真实任务 success、verifier 全 PASS，证据
  `evidence-task-high-nist/`）。§7 判据 1/6 探针层核对完成；事件面判据
  （err 升压 / slow-stall / LIF / 降级链 / 假成功-事件面）依赖 agent
  k=1，并入 ⑦ 网络/凭据后与模型侧合并执行。详见
  S4_PROGRESS_2026-09-02.md §12。
- [ ] ⑥ 全量 + 记账（2026-09-02 口径：任务执行=control + high-nist
  双臂、不做 non-admin 任务消融——用户裁决，见 BACKLOG 0l / 设计
  §6/§10；主体=§6 剩余摩擦探针/真实任务移植 + high-nist 主载跑批 k=1 +
  记账；**⑥.1 机器侧 batch-2 已闭环（2026-09-02）**——control 9/9
  success、high-nist 9/9（denied×4 / blocked×2 / success×3），证据与
  缺口见 S4_PROGRESS §14；网络/Defender 长构建轴记账归 ⑦（Clash 网络 +
  模型侧 slow/stall），agent 侧 §7 事件面判据随 ⑦ 执行；整体勾选待 ⑦
  合并收口）。
  - [x] ⑥.1 batch-2 任务集移植 + verifier/runner/驱动扩展 + 双臂机器侧
    跑批（2026-09-02 闭环：6 新任务——probe-temp-write /
    probe-symlink-create / probe-service-create /
    probe-pip-user-install / probe-unsigned-ps1-run / regex-log，
    manifest 累计 9、batch `P0-0l-batch2`；control + high-nist 全
    PASS，见 S4_PROGRESS §14）。
- [ ] ⑦ 收尾：AppLocker 恢复复验、DeepSeek 凭据 CredRead 验证、临时
  任务/累积 ACE/旧目录清理、Clash 网络导入；补模型侧 §7 事件面判据
  （agent k=1，依赖网络/凭据重录）与 LoadUserProfileW 5023 候选修复
  （apply 后 hive 释放窗口，归 ⑦）。
  - [x] ⑦ 网络/凭据/收尾子项（2026-09-02 深夜闭环）：Clash 7897
    本地端口与经代理出站连通验证通过（netcheck）；AgentUser 凭据
    CredReadW non-admin 可读 / high-nist AppContainer WinError 5
    （摩擦点 #8，登记为 agent 轮凭据注入前置缺口）；驱动
    `-CheckpointName`（wrapup/网络轮默认 `S4-BASE-NET-2026-09-02`）；
    AppLocker 复验 Enabled + 4 规则、enforcement-probe 19/19 ×2；
    5023 候选修复（apply hive settle + sandbox 重试 + SYSTEM 就绪门 +
    `UnloadUserProfileW`→`UnloadUserProfile` 符号修复）并登记残余
    in-sandbox 5023 缺口；清理 10 个残留 AppContainer 包目录/工作区，
    无残留任务与进程。证据 `evidence-netcheck/` + `evidence-wrapup/`
    （S4_PROGRESS §15）。
  - [ ] ⑦ 模型侧 §7 事件面判据（agent k=1；2026-09-03 前置收敛：
    Clash 常驻/自启退役——用户裁决 VM 内不上网，DeepSeek 直连恒放行，
    NET 快照仅作维护用；AppContainer 凭据注入已闭环——orz Windows env
    通道 + 沙箱 `--env-file` + 实机链路证据
    `evidence-cred-inject/`，S4_PROGRESS §16。**2026-09-03：Windows
    orz.exe 已重建并同步（CF5662...CFAC4D6，旧二进制备份于
    `_windows_high_nist/backup/`）；agent stage 接线完成——新客机
    runner `_windows_high_nist/run/run_agent_arm.ps1` + 驱动
    `agentcontrol`/`agenthighnist` stage，bootstrap 取凭据 +
    `--env-file` 注入 + allowlist 221.204.163.76 已并入（未跑批，
    S4_PROGRESS §16.7）；**2026-09-03 执行前必做项闭环**——桥 op
    `vm-agent` live 执行路径（host `scripts/s4_vm_agent_run.ps1`）、
    worker restart、sync 全绿、VM 内 DryRun chunk1 通过（3 题/逐题
    timeout/env-file/allowlist 全对）；TB2.1 最新错题集 9 题资产树
    `_windows_high_nist/agent-tasks-tb2.1/` + 3 题/批口径（chunk1-3，
    S4_PROGRESS §16.8）；runner 修 1 项变量遮蔽（大小写不敏感参数覆盖）。
    真跑待放行**。**2026-09-03 §7 事件面分析（chunk1-f4 三 journal +
    LIF 离线重放）已完成并登记 S4_PROGRESS §16.15**：六判据结论=
    deny/err 通道零 fire、slow 1（mteb 300s 工具）、stall 0、判据 5
    本任务集 N/A、无事件面假写；新增候选缺口 F6–F10（run 墙钟模型
    不可见 / 长工具无中间回报 / file-write×junction 不兼容 /
    workspace 跨批残留泄漏 / permission deny 不入 deny 通道）。判据
    1–6 需 0.3.0 journal 复验后闭合；0.3.0 Windows 三件套待同步 VM。
- [ ] ⑦ 模型侧 §7 判据 0.3.0 复验（2026-09-03 已推进：0.3.0 三件套
  同步 + keystore 重建 + signer 复检完成；DeepSeek key 经用户轮换后
  改走 key 文件覆盖通道（ORZ_AGENT_KEY_FILE，VM AgentUser 凭据库写
    路径不可靠已登记）；chunk1-0303 3/3 ran（make-doom/gcode 仍死于
    840s 墙钟，mteb 348s 完成），§7 判据 1–6 复验结论与 F6–F10 状态
   更新见 S4_PROGRESS §16.16；F9 驱动修复已落地）。下一步：⑥ 全量
    主体 chunk2（path-tracing / train-fasttext /
    adaptive-rejection-sampler）3 题/批续跑。
- [ ] ⑧ 工具执行层改革（TER）：明细移至 [TODO2.md](TODO2.md)（M0 设计
  门 → M1 orz 主线 → M2 Windows runner/VM → M3 回归复验），开放项见
  [BACKLOG2.md](docs/BACKLOG2.md)；本行仅作主 TODO 指针，勾选以 TODO2
  为准。

### P0-0m GSA-SESSION-VOLUME-BOTTOM-LAYER（P0；2026-09-06 设计定稿，同日用户裁决放行，排期实施）

用户裁决：`.gsa` 为 LIF 科学性组件，保留并下沉为底层部件；权限层不裁撤
（后续「助理层拦截系统核心路径、仅删除保护」另行立项），放开压到最窄。
排期（2026-09-06 放行）：S1 代码先行，S1–S4 独立审计 + 独立提交；S3 复验
吸收 GAP-GSA-SYMLINK-STALE-TEST 连带观察（`.gsa` terminal-log 白名单会话卷
形态豁免）。

- [ ] S1 代码：SessionVolume 资源 + host 装配 canonical 单源注入 + 沙箱
  三分判定 + 窗口契约下沉（terminal-log / run_tests 只读窗口）+
  gitignore 绕过 + permission.rs `.gsa` 段退役标注。
- [ ] S2 测试：11 项测试矩阵 + 全量回归。
- [ ] S3 接线复验：补读链/run_tests 窗口端到端（`.gsa` symlink 会话卷
  实机构造）+ GAP-GSA-SYMLINK-STALE-TEST 演进注记。
- [ ] S4 收口：索引/BACKLOG/TODO 同步 + 门禁 Exit 0。

入口：[设计](docs/GSA_SESSION_VOLUME_BOTTOM_LAYER_DESIGN_2026-09-06.md) /
[ADR-0010 §14.56](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) /
[BACKLOG 0m](docs/BACKLOG_AND_PRIORITIES.md)。

### P0-0n GAP-APPROVAL-PROMPTER（**延期**；2026-09-06 排期登记，同日用户裁决延期）

延期裁决（2026-09-06 用户）：当前无具体设计文档的项均非急切或必需内容——
S1 设计定稿完成前不排期实施、不占当前工作集；本节保留作排期登记档案，
S1 定稿后按下列批次恢复推进。

GLM F2 处置转排期（2026-09-06 用户裁决）：`orz-host/src/approval.rs` 全文件
注释 + TODO 存根、`lib.rs` 标注 approval path still a stub——交互审批器补齐。
边界：审批器只承担交互审批呈现、决策回传与持久化，不收敛权限判定双实现
（OBS-PERMISSION-DUAL-IMPL 另案）；缺省 fail-closed 不变。

- [ ] S1 设计定稿：审批触发面（Interactive 权限门）+ 决策词汇（allow / deny /
  持久化语义，含 `approval_allow_persists_for_identical_bash` 既有语义收编）+
  permission 判定层接口 + TUI/ACP 两车道呈现 + 缺省 fail-closed；产出设计
  文档（涉及 ADR-0010 时按 §14.x 转录）。
- [ ] S2 实施：`approval.rs` 实装 + `lib.rs` approval stub 摘除 + 决策持久化 +
  契约/事件面登记（如涉及）。
- [ ] S3 测试与复验：单测矩阵 + orz-host 既有 flaky
  `approval_allow_persists_for_identical_bash` 复核收编 + Interactive 实机复验。
- [ ] S4 收口：GAP-APPROVAL-PROMPTER 状态翻转 + 索引/BACKLOG/TODO 同步 +
  门禁 Exit 0。

入口：[approval.rs](orz/crates/orz-host/src/approval.rs) /
[GLM 登记审计](docs/audits/GLM_EXTERNAL_REVIEW_REGISTRATION_2026-09-04.md) /
[处置 + S2 排期审计](docs/audits/P0_GOV_GLM_DISPOSITION_AND_TASK_D_S2_SCHEDULE_2026-09-06.md) /
[BACKLOG 0n](docs/BACKLOG_AND_PRIORITIES.md)。

### P0-B FUS-RETRIEVAL-MECH（`implemented`；批次 1-6 全部闭合 2026-08-14，保留供核对）

- [x] 全部闭合：B-1 citations 结构化透传 / 步骤 2 web_fetch 候选计数门禁（cap=8）/ 步骤 3 机械预筛（canonical 去重 + 失败形态剔除 + tier/weight）/ 步骤 4 browser_read 模式扩展 + 计数域复用 / 步骤 5 输出级引用校验器 / 步骤 6 提示词缩短。入口：[检索机械控制设计](docs/RETRIEVAL_MECHANICAL_CONTROLS_DESIGN_2026-08-13.md) / 各步骤实施审计 / BACKLOG 0B。
- 注：DC 剩余两信号（`same_module_no_evidence` / `key_surface_unexamined`）已转 P3 遗留小项，2026-08-31 随 P2-11 DC 清理退役。

### P0-C CLASSICAL-EXEC-ASSISTANT（已转正式组件；小样 1/2/3 + S1-S4 全部闭合 2026-08-16）

- [x] 全部闭合（2026-08-16 用户裁决转正式组件，不达标即撤条款未触发）：小样 1/2/3（控制台路由 / 编辑执行器 / 机械组合脚本）+ orz 内嵌集成 S1-S4（操作台核心 + 黑板动作栏 / 模型面投影 + 轮末机械发放 / 结构化策略拒绝 + assistant.trace + run_script + Profile/Bundle / 端到端 + 单步超时 + 脚本预算 + 二次审查收口）。入口：[设计](docs/CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md) / [POC](prototype/classical_console/README.md) / 各实施审计 / BACKLOG。

### P0-C2 PLAN-FIRST-BLACKBOARD（阶段 A/B/C 全部闭合 2026-08-16；生产默认路径不启用）

- [x] 全部闭合：阶段 A（模板去人格 + AGENTS.md 机械包裹 + 首轮计划轮硬门 + plan_write）/ 阶段 B（注册板块=探针投影）/ 阶段 C（console 默认 + direct 受控降级双模式 + 步骤门 + 事件契约）。入口：[设计](docs/PLAN_FIRST_BLACKBOARD_DESIGN_2026-08-15.md) / ADR-0010 §14.17 / 各实施审计。

### P0-D ORZ-COMPACTION-REDESIGN（`implemented`；S1-S6 全部闭合 2026-08-14）

- [x] 全部闭合：S1 恢复预检截断 + marker/白名单保留 / S2 动作台账机械坍缩 / S3 五段模板摘要 + 事件面 + 存档 / S4 审计同步 / S5 审查修复（守卫重试、session_end、冷却、超时）/ S6 二次复查。入口：[压缩设计](docs/CONTEXT_COMPACTION_DESIGN_2026-08-14.md) / ADR-0010 §14.10/§14.14 / [审计](docs/audits/GAP_COMPACTION_REDESIGN_IMPL_AUDIT_2026-08-14.md) / BACKLOG 3b。

## P1 — 可并行审计 / 证据

### FUS-COMPONENT-REGISTER（`partial`）

- [ ] 逐 crate/component 采用审计——65 组件全 `audit_required`；从当前代码可达性与 local diff 出发，不得由 crate 名/编译推断采用档位（V11-IMPL-008）。
  - 成熟复用评估（2026-08-16，只读）：部分——审计对象即 65 个 Grok Build/Rust 生态成熟组件，产出即复用裁决（直接复用/薄适配/fork）。
- 入口：[register yaml](upstream/fusion-component-register-v0.1.yaml) / [register schema](upstream/fusion-component-register-v0.1.schema.json)。

### GAP-WINDOWS-EVIDENCE（`partial`）

- [ ] ORZ-WIN-PROC-001/002/003 案例晋级——真实 provenance、脱敏、分类与回归门槛（FUS-DOC-001）。
  - 成熟复用评估（2026-08-16，只读）：明确——可复用 Grok child-tree probe 证据与 GAK AppContainer/Job Object 审计。
- [ ] 建立 Windows 平台兼容性设计文档（ADR-0010 §6/§11.7 目标：`architecture/WINDOWS_PLATFORM_COMPATIBILITY_DESIGN_v0.1.md`）。
  - 成熟复用评估（2026-08-16，只读）：明确——设计输入含既有 Windows 审计/事故材料与 Codex/Grok Windows 行为对照。
- [ ] 建立 `regression/windows/` 与 `.observed-runs/windows/` 路由（自动回归/人工复核入口与 git-ignored 原始运行目录）。
  - 成熟复用评估（2026-08-16，只读）：明确——路由可承接既有 observed runs/探针矩阵先例（Grok probe）。
- 入口：[incidents](docs/incidents/windows/README.md) / [cases](docs/cases/windows/README.md)。

### IMPL-DEEPSEEK-TRANSPORT + SEC-CREDENTIALS（`partial`）

- [x] transport/retry/thinking 主/子代理同构复核（2026-08-16 闭合）：三实例共享单一 `DeepSeekTransport`（ModelConfig/RetryPolicy/ThinkingMode::EnabledMax 单一来源）、`REQUEST_MAX_TOKENS=160_000` 单一常量、请求级 thinking 覆盖仅 `-p` 预检轮（F-07 文档化例外）；边界=压缩摘要/预检轮为 loop 外辅助请求（非同构范畴）、契约 §2.1 旧别名拒绝与 /models 预检未实现（另行跟踪）。
- [ ] DeepSeek live 通道与 Windows 实机晋级证据（ADR-0010 §11.7；当前仍为 offline / 构建时 evidence）。
  - 成熟复用评估（2026-08-16，只读）：部分——DeepSeek 官方 API/文档为成熟参照；主要工作是证据收集而非实现复用。
- 入口：[ADR-0007](adr/ADR-0007-transport-retry-policy.md) / [DEEPSEEK_ADAPTER_CONTRACT](architecture/DEEPSEEK_ADAPTER_CONTRACT_v0.1.md)。

### ORZ-CACHE-CONTEXT-COST（`implemented`；2026-08-15 三项全部闭合）

- [x] 全部闭合：`request_header_change` 请求头留痕 + 探针准确性审计（翻转↔header 交叉核对）+ 单轮工具结果注入预算（默认 50K、超限拒批 + offset 续读）与策略化读取；orz-loop 337 / Python 1888+14 skipped / 仓库门禁 valid。入口：ADR-0010 §14.9 / [探针设计](docs/TOOL_AVAILABILITY_PROBE_DESIGN_2026-08-13.md) / [审计](docs/audits/GAP_CACHE_CONTEXT_COST_IMPL_AUDIT_2026-08-15.md)。

### ORZ-ORIENTATION-FORCED-TEMPLATE（`implemented`；2026-08-15 闭合；2026-09-01 退役）

- [x] 全部闭合：ADR-0010 §4.2 正文修订 + 强制模板轮实现（无工具 checkpoint 轮、模板校验、一次重填 + 降级兜底、pending 单槽、主车道）+ 缓解必做（progress_evidence 交叉校验 + 缺失面）+ v0.2 `checkpoint_response` 事件（Schema/verifier/fixtures）+ 二次审查修复；orz-loop 333 / Python runtime 264。入口：[设计](docs/ORIENTATION_FORCED_TEMPLATE_DESIGN_2026-08-14.md) / ADR-0010 §14.13 / [审计](docs/audits/GAP_ORIENTATION_FORCED_TEMPLATE_IMPL_AUDIT_2026-08-15.md) / BACKLOG 6c。

### ORZ-SESSION-CONTEXT-MONITOR（`approved`；2026-08-16 度量重定）

- [x] 压缩恢复预检估算校准（chars/2 中文低估）：2026-08-16 用户裁决度量改次数制后废止，不再依赖 token 估算口径。
- [ ] 度量接线：监听 `context_compressed` 事件（reason=rhythm/fallback）累计会话内压缩次数；`session_end` 不计；同一次压缩只计一次。
  - 成熟复用评估（2026-08-16，只读）：有——复用既有 `context_compressed` v0.2 事件面，无新 Schema。
- [ ] 阈值配置（env/TOML，默认 2 次提醒 / 3 次总结推荐）；同一阈值只触发一次。
  - 成熟复用评估（2026-08-16，只读）：部分——2/3 为旧 384K/500K 语义对应，默认待校准。
- [ ] 最简实现：阈值到达的最后一轮模型输出末尾机械附言（附压缩次数）；headless/自动化仅写日志；3 次附五段模板 + 新窗口开场提示骨架。
  - 成熟复用评估（2026-08-16，只读）：部分——3 次推荐复用压缩五段模板（Grok compaction 血统）。
- [ ] 测试（到达/未到达、session_end 不计、一次一计、headless 分支、幂等）+ 实施审计 + BACKLOG/TODO/索引状态同步。
  - 成熟复用评估（2026-08-16，只读）：无——自有测试/审计工作。
- 入口：[设计](docs/SESSION_CONTEXT_MONITOR_DESIGN_2026-08-14.md) / ADR-0010 §14.13/§14.18 / BACKLOG 6d。

### ORZ-BLACKBOARD-PLAN-EPOCH（`implemented`；S1-S7 全部闭合，2026-08-14/15；2026-09-03 退役标注：生产语义被会话作用域黑板取代，`--plan` 诊断保留）

- [x] 全部闭合：plan epoch 身份与批准事件 / 原子轮换与归档（.gsa/blackboard）/ 压缩解耦 / 跨 epoch 回查 / 测试审计 + S6/S7 复查补强（epoch 时间戳单调、归档写盘原子化、epoch_archive_write_failed 事件）。入口：[设计](docs/BLACKBOARD_PLAN_EPOCH_DESIGN_2026-08-14.md) / ADR-0010 §14.15 / [审计](docs/audits/GAP_BLACKBOARD_PLAN_EPOCH_IMPL_AUDIT_2026-08-14.md) / BACKLOG 6e。

### ORZ-LARGE-FILE-READ-CONTRACT（`implemented`；2026-08-17 设计定案，同日实施闭合）

- [x] 全部闭合：读取句柄信封（粗门默认 16KB、可配 8–32KB + 有界预览 ≤4KB + offset 续读指针）+ 模型面契约提示（grep/结构优先、大文件分段、空结果语义）+ 黑板/结果栏只放指针 + 全面检查修复（信封空窗口/越界语义、toolset 配置端到端、描述同步）；orz-tools read_file 201 / orz-loop 440 / orz-host e2e 5。入口：[设计 §11](docs/CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md) / ADR-0010 §14.22 / [审计](docs/audits/GAP_LARGE_FILE_READ_CONTRACT_IMPL_AUDIT_2026-08-17.md) / BACKLOG 6f。

### FUS-LEDGER-FOLD-STATE（`implemented`；2026-08-18 设计定案，同日实施闭合）

- [x] 全部闭合：fold 三态 + 有状态请求视图 + loop-top 推进触发（128K）+ 压缩联动/摘要同源/恢复 + 参数接线（192K/256K）+ 二次全面审查收口（冻结台账进摘要存档、v0.2 `ledger_fold_advance` 事件、完整性回退、死代码删除）；orz-loop 452 / conformance 15 + journal validation 214。入口：ADR-0010 §14.26 / BACKLOG 6g / [审计](docs/audits/GAP_LEDGER_FOLD_STATE_IMPL_AUDIT_2026-08-18.md)。

## P2 — 生产化决策门

### IMPL-CONTROL-FABRIC（`partial`）

- [x] fail-closed 生产启用（2026-08-16 闭合）——默认翻转（未设置即强制，显式 `0|false|no|off` 影子，非法值 exit 2）；CLI run / ACP stdio / TUI 三入口接线；核查清单 ⑦⑨⑩⑪ 收口；`orz-acaf-provision` 供应工具 + 启动链。入口：[ADR-0011](adr/ADR-0011-authenticated-control-and-action-fabric.md) / [ACAF 设计](docs/AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md) / [fail-closed 审计](docs/audits/GAP_ACAF_SLICE2_FAILCLOSED_IMPL_AUDIT_2026-08-13.md)。
- [ ] Slice 3：ModeChangeTicket → `bump_policy_revision` 首个生产递增来源 + policy_digest 真摘要切换 + 会话级计数器 gate（D3-1）。
  - 成熟复用评估（2026-08-16，只读）：部分——HKDF-SHA256/HMAC 与单调计数器为成熟标准原语；ACP 模式切换为成熟先例。
- [ ] Slice 4：Windows Sandbox backend（D-11）。
  - 成熟复用评估（2026-08-16，只读）：明确——Windows Sandbox（Hyper-V）、AppContainer、Job Object 为成熟 OS 能力；GAK/P2 审计可直接支撑。
- [ ] 可选：检索车道 web_fetch activation 绑定接线；conformance capture 票据场景；normalize_lexical 单源化；ACP 会话路径接 ACAF。
  - 成熟复用评估（2026-08-16，只读）：部分——ACP 规范/xai-acp-lib、orz-paths、既有 D-13 机制与 fixture 体系可复用。

### OPS-PROTOCOL（`pending`；裁剪方向已定）

- [ ] 产出裁剪设计：删除安全保留为 host-owned 工具、跨环境桥接内部化、双执行器收敛单一参考（生产走 Rust 工具面）。
  - 成熟复用评估（2026-08-16，只读）：部分——Windows 回收站（BitBucket）与 XDG trash 为成熟 OS 约定；既有执行器作参考。
- [ ] 生产接线裁决（先验票，再由协议执行器执行）。
  - 成熟复用评估（2026-08-16，只读）：无——用户决策门。
- 入口：[协议](protocol/structured-operation-protocol-v0.1.md) / [Schema](protocol/structured-operation-protocol-v0.1.schema.json)。

### MECHANICAL-LAYER-MATH-CALCULUS（`implemented`；阶段 0-3 全部闭合 2026-08-31，BACKLOG P2-10）

- [x] 全部闭合：阶段 0 决策（D1-D7）/ 阶段 1 设计定稿（F1-F6 + ADR-0010 §14.47 转录）/ 阶段 2 实施切片（I1 T̂+LIF 计算器、I2 失败目标身份、I3 temporal 分区、I4 域 spike 侧车、I5 类型化信封、I6 pipe 归约）/ 阶段 2 全面审查处理（R1-R9 + F10-F14 全部收口）/ 阶段 3 验证（V1 FakeProvider 面 8 项 + F11 receipt↔事件链同构核对、V2 离线 102 runs 四对照门 + 聚类对照 + 零误干预、V3 S3 重建 + S4 实机冒烟 1/1 与 temporal 四查询面端到端一致）。入口：[正式设计](docs/MECHANICAL_LAYER_MATH_CALCULUS_DESIGN_2026-08-30.md) / [讨论稿](docs/MECHANICAL_LAYER_MATH_CALCULUS_DISCUSSION_2026-08-30.md) / [ADR-0010 §14.47](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [阶段 3 验证记录](docs/audits/MECHANICAL_LAYER_MATH_CALCULUS_PHASE3_VERIFICATION_AUDIT_2026-08-31.md) / BACKLOG P2-10。

### MODEL-RESIDUAL-PRESSURE-FOLLOWUP（P2；2026-08-31 二次讨论裁决登记，BACKLOG P2-11）

> 排期：设计轮（PULL 自描述 / retryable 分类位 / 依赖图）→ 设计定稿 → 实施放行；
> DC 清理已裁决可直接实施（P3 清理类，放行时入账）。入口：
> [讨论稿](docs/MODEL_RESIDUAL_PRESSURE_DISCUSSION_2026-08-31.md)（§8 裁决）/
> [BACKLOG P2-11](docs/BACKLOG_AND_PRIORITIES.md)。

- [x] PULL 自描述设计（2026-08-31 定稿 + S1/S2 + 审查修复完成，`partial`；
  S3 重建 + S4 实机复验待放行）：`blackboard_read` 增量头 + temporal 一次返回
  （零注入、8 工具面冻结）。入口：设计 `docs/PULL_SELF_DESCRIPTION_DESIGN_2026-08-31.md`
  / ADR §14.48 / [审查修复](docs/audits/P2-11_PULL_SELF_DESCRIPTION_S1_REVIEW_AUDIT_2026-08-31.md)。
- [x] DC 强制模板轮清理（2026-09-01 实施完成，闭合）：DC 机制全删
  （`diagnostic_coverage.rs` / 强制模板轮 / 信号消费）+ plan 反例变体注册
  （`COUNTEREXAMPLE_GATE_PLAN_BLOCK`）+ P3「DC 硬信号 4/6」退役；
  schema/verifier/fixtures/测试收口；checkpoint 共用件拆分（orientation
  软门与 console 询问轮保留）。入口：[实施审计](docs/audits/P2-11_DC_FORCED_TEMPLATE_CLEANUP_IMPL_AUDIT_2026-09-01.md)
  / ADR-0010 §14.49 / BACKLOG P2-11。
- [ ] retryable 机械分类位：`Fail` 信封增 `retryable: bool`（确定性失败 false：
  scheme/锚点/sealed/cap；暂时性 true：超时/网络），错误码事实推导、非建议；
  schema/verifier/fixtures 先行。**S1 实施 + S2 测试完成（2026-09-01）**：
  `orz-assurance/src/tool_envelope.rs` 增 `retryable_for_code` 构造期推导
  （未知码 fail-closed false），reducer/fake-provider fixture 同步；设计转录
  ADR-0010 §14.50 / 机械层设计 §2.1；orz-assurance 195 lib + 9 fake-provider
  测试全绿、orz-loop 编译通过、fmt/clippy 无新增；2026-09-01 审查处理 O1–O4
  收口（字段私有化/归约边界位归一化/优先级与边界测试，199 lib + 9 全绿）；
  S3 重建 + S4 实机复验待放行。
- [ ] 依赖图实施（下一轮主线）：文件锚点链最小范围（read→write 锚点边 + 工具→
  实体变更边；D3 命令/检索副作用不建图），PULL 查询面、模型零改动；顺带闭合
  F11 receipt↔事件链逐段同构核对。**2026-09-01 设计定稿 + S1 实施 + S2 测试
  完成（`partial`）**：`orz-loop/src/dep_graph.rs` 新模块（ReadFact/WriteFact/
  锚点边匹配/容量/revision/渲染）+ 黑板接入 + `blackboard_read section=deps`
  PULL 面 + 通用执行路径成功建图（事实随 ToolCompleted 写 `dep_graph`
  可选事件字段）+ schema/verifier/fixtures 先行 + F11 顺带闭合；设计
  `docs/DEPENDENCY_GRAPH_MAINLINE_DESIGN_2026-09-01.md` / ADR-0010 §14.51 /
  [实施审计](docs/audits/P2-11_DEPENDENCY_GRAPH_IMPL_AUDIT_2026-09-01.md)；
  orz-loop 642 lib（+7）/ orz-assurance 199+9 / Python 255 全绿；
  S1 全面审查处理收口（2026-09-01：渲染截断 footer 预算 + 单测、无效
  section 文案、verifier 措辞/死字段、设计措辞统一、边界测试补充、成本
  与序列化面登记，见
  [审查处理记录](docs/audits/P2-11_DEPENDENCY_GRAPH_S1_REVIEW_AUDIT_2026-09-01.md)）；
  S3 重建 + S4 实机复验待放行。
- [x] 工具名幻觉登记边界（不改名/不别名；fail-loud 自回正，收益上限 ≈15 轮/
  10 题）——2026-08-31 裁决。
- [x] search_replace 锚点 / submit 两阶段维持现状（0 拒单 / 8 次全通，优化收益
  不足）——2026-08-31 裁决。

### COMPRESSION-LINGUISTIC-FORMAL-LAYER（P2；2026-09-02 讨论稿登记，BACKLOG P2-12）

> 排期：讨论稿（reference 路由）→ 转正式设计（域标注 + F4 失败目标聚合，更新
> CONTEXT_COMPACTION_DESIGN 注意事项槽渲染语义）→ 用户裁决 → 实施放行。入口：
> [讨论稿](docs/COMPRESSION_LINGUISTIC_FORMAL_LAYER_DISCUSSION_2026-09-02.md)
> （§3/§6 收口）/ [BACKLOG P2-12](docs/BACKLOG_AND_PRIORITIES.md) /
> [S1/S2 实施记录](docs/audits/P2-12_COMPRESSION_LINGUISTIC_FORMAL_LAYER_S1S2_IMPL_2026-09-02.md)。

- [ ] 域作为压缩参考（方案 A，2026-09-02 定案口径）：聚合行键 = F4 身份 (kind, id)
  + epoch 内累计 + 跨 marker 去重顺带解决；域不参与行键、降级为行内序列标注（切换
  中间部分天然吸收）；错误码行内集合（全留/不留）；相对 run 起点墙钟首末时间；不
  携带日志级明细；聚合状态归黑板（epoch 作用域、随黑板轮换自然重置）；域标注 =
  写时盖章（按事件所属决策轮）+ 域段书签归并。设计轮前置条件已闭合（2026-09-02）；
  **S1/S2 已实施 + 全面审查处理（2026-09-02：`failure_agg` 黑板分区 +
  快照/恢复/轮换重置 + 三处 F4 失败写时盖章 + 注意事项槽渲染替换 exec 错误
  窗口；审查处理 = 溢出指针可回查修复（被 3K 槽挤出的聚合行随压缩摘要存档
  补全段保存）、错误码可复核口径与跨 run 时间轴边界登记、文档修正；orz-loop
  654 passed / 0 failed）**；S3 重建完成（2026-09-02：Linux musl 三件套 21:13
  HKT，orz 106,926,480 B / orz-signer 1,390,376 B / orz-acaf-provision
  1,208,232 B；musl 静态无 PT_INTERP、P2-12 接线符号命中（failure_agg ×43 /
  failure_target ×17）、bookworm 冒烟三件加载执行通过；对应源码 orz f0eeb524
  + 工作树 P2-12 未提交改动，日志 D:\tb-eval\orz-linux\build-20260902.log，
  见 BACKLOG P2-12）；S4 实机复验与 ADR-0010 转录待续。
  审查处理见
  [审查处理记录](docs/audits/P2-12_COMPRESSION_LINGUISTIC_FORMAL_LAYER_REVIEW_HANDLING_2026-09-02.md)。
- [ ] 失败目标聚合进注意事项槽：F4 聚合行渲染替换「最近 5 条截断错误」窗口语义；
  实施时提为独立设计条目走排期（设计定稿 + 用户裁决后实施）。**S1/S2 渲染改造已
  随上条一并实施（2026-09-02）**；设计投影更新见
  `CONTEXT_COMPACTION_DESIGN_2026-08-14.md` §4.4.4，ADR-0010 转录待实施闭合时登记。
- [ ] §5 离线验证切片（BACKLOG P2-12 第 5 项，随 P2-12 放行后单独排期，未排期）：
  先验已实施部分（F4 失败目标聚合）——虚拟压缩点回放覆盖率 + 「概括」正确性
  （digest 计数 / 首末时间 / 错误码集合对账；错误码对账首选验证器复刻
  ToolErrorKind→code 映射、次选 schema-first 补事件结构化 code）；开放锚点覆盖率
  待 Centering 方向实施后再并入。见 [BACKLOG P2-12](docs/BACKLOG_AND_PRIORITIES.md)。
- [x] S1/S2 全面审查处理（2026-09-02）：溢出指针回查修复（存档补全段）+
  host 错误码可复核口径登记 + 跨 run 时间轴边界登记 + §4.4.1/§4.4.4 措辞
  修正——完成并测试全绿（654 passed / 0 failed / 3 ignored）。

### BLACKBOARD-CONVERSATION-SCOPE-FOLD（P2；2026-09-03 设计定稿 + ADR-0010 §14.52 转录；实施待放行，BACKLOG P2-13）

> 排期：初版设计稿 → 开放问题裁决 → 设计定稿（已完成）→ 实施放行。入口：
> [设计稿（v0.8 定稿）](docs/BLACKBOARD_CONVERSATION_SCOPE_FOLD_DESIGN_2026-09-03.md)
> / [BACKLOG P2-13](docs/BACKLOG_AND_PRIORITIES.md)。

- [x] 开放问题裁决（2026-09-03 v0.5：R1–R6 全部收口，见设计稿 §12；
  会话锚点是否新建维持开放（默认不建，不阻断）；体验项 E11 不阻断）。
- [x] 方案 A/B 主线裁决（2026-09-03：B 定为主线，黑板彻底不做存储压缩，
  A 冻结为对比档案）。
- [x] 参数直觉定档（2026-09-03 v0.4：W=512K / T=64K / K=10 / 展开下限 20% /
  疲劳档 50/70/90%，可 env 覆盖）。
- [x] W 口径收口（2026-09-03 v0.6：只计域折叠记录分区；512K 字符 ≈
  256K token，大于 192K token 压缩窗口；70% 建议加会话压缩 ≥2 门槛）。
- [x] W 改存储字节口径（2026-09-03 v0.7：10 MiB 紧凑 JSON；疲劳 50/70/90%
  ≈ 5/7/9 MiB；T=64K 字符渲染口径分列；env
  ORZ_BLACKBOARD_LIVE_BUDGET_BYTES）。
- [x] 评估复核（2026-09-03 v0.8：复杂度/压缩关系为未证实直觉，不作 W
  论证依据；W 依存储成本 + 寿命软上限；E12 不做弱保软；遥测路径登记
  于设计稿 §13.3）。
- [x] 设计定稿 + ADR/索引登记（2026-09-03：ADR-0010 §14.52 转录；
  CLI_PROJECT_INDEX 新增 AUTH/FUS-BLACKBOARD-CONVERSATION-FOLD；
  plan-epoch 系列（AUTH/FUS-BLACKBOARD-PLAN-EPOCH、BACKLOG 6e、旧设计
  文档）标注退役，`--plan` 诊断保留）。

实施分四批（2026-09-03 用户确认：分开做、不细化）：

- [x] **B1 会话化基础**：记录写时 (round, domain) 结构化盖章（ExecEntry
  等）+ 会话级 live 黑板持久化 / conversation-relative 轴（ACP 每 prompt
  续载、轮号与域机器续接、temporal/failure_agg 原点迁移；CLI 单 run 不变）。
  **S1/S2 完成（2026-09-03）**：ExecSection 行结构化 `ExecEntry{text,
  round, domain, ts}`（旧字符串 serde 兼容）；edits / tool_actions /
  actions receipts / 检索分区写时盖 (round, domain) 章；会话侧车续接包
  = 对话 + 黑板 live 视图 + LIF 会话轴快照（round/域机器/spike 时间线），
  ACP 每 prompt 续载、失败 run 不写回；temporal `t` 以会话起始墙钟为轴
  （跨 prompt 单调）；依赖图保持 live-only 不落侧车；CLI 单 run 行为
  不变。S2 全串行绿：orz-loop 657 / orz-assurance 201+9 / orz-host 242
  / orz-bin 53（orz-host 并行仅预存 flake
  `call_tool_timeout_kills_process_tree`，串行通过）；fmt/clippy 无新增。
  实施记录见
  [B1 实施审计](docs/audits/P2-13_B1_CONVERSATION_BASE_IMPL_AUDIT_2026-09-03.md)。
- [x] **B2 渲染折叠**：blackboard_read 折叠态渲染（默认展开 = 当前域段 ∪
  最近 K 轮 ∪ 最近 20% 行）、展开参数 domain + round_from/round_to、
  分区渲染 cap（edits/tool_actions 补齐）、pre-stamp 段。
  **S1/S2 完成（2026-09-03）**：render_fold 纯函数核心（T/W/K/20% 参数
  定档 + env 覆盖、域段切分/展开子集/触发判定）+ exec/edits/tool_actions
  折叠视图组装（标注行/pre-stamp 独立段/显式展开合并）+ edits/tool_actions
  渲染 cap 补齐（50 行/200 字符/4K 字符）；host_exec 组合守卫 fail loud
  （all-or-none/非法值/与 receipt_id/since/epoch 互斥/非折叠分区拒绝）；
  工具声明增量；归档 epoch 读与未达阈值读取逐字节不变。S2：orz-loop lib
  676 全绿（+18）/ orz-host 241（仅既有 flake 单独复跑通过）/ fmt/clippy
  无新增。实施记录见
  [B2 实施审计](docs/audits/P2-13_B2_RENDER_FOLD_IMPL_AUDIT_2026-09-03.md)。
- [x] **B2 复审处理（2026-09-03，全面复审）**：折叠态 receipt_id 守卫旁路
  修复（receipt_id 仅 actions 显式报错）+ 未达阈值显式展开 = 普通读取 +
  展开目标行 4K cap 保护（绝不静默丢失）+ 标注行落在首个折叠行位置 +
  pre-stamp 标注时间范围取折叠子集 + W 计量短路 + 单域单段语义裁定登记
  （无段标注总览为既定后果，B4 遥测复核）；orz-loop lib 682 全绿（+6）。
  处置登记见
  [B2 复审处理](docs/audits/P2-13_B2_REVIEW_HANDLING_2026-09-03.md)。
- [x] **B3 契约与收尾（2026-09-03）**：空槽「（无）」统一（目的/计划/
  变动文件路径/注意事项空态 + 状态行目标默认；后续衔接槽固定占位不改）；
   用户侧疲劳提醒（W=10MiB 水位 50/70/90 档、单档越线一次、70 档换对话
   建议——压缩轮数门槛于 B3 复审撤销、ACP user_notice/侧车 meta/CLI
   stderr 降级）；存档单包
  gzip（`.gsa/archives/<session8>.json.gz` 纯打包原子写 + digest +
  `session_archive` v0.2 事件——schema/verifier/fixtures 先行，run-event
  枚举 52→53、run_id 前缀 +ARC，专用 ARC run journal）；plan-epoch 生产
  面退役清理（marker `plan_epoch` → 会话快照行、blackboard_read `epoch`
  参数仅归档目录配置时声明、epoch.rs 头注登记，`--plan` 诊断保留）。
  orz-loop lib 689 全绿（+5）/ orz-host 242（仅既有 flake 单独复跑通过）
  / orz-tui 178 / orz-bin 全绿 / Python conformance 15 + journal 242。
  实施记录见
  [B3 实施审计](docs/audits/P2-13_B3_IMPL_AUDIT_2026-09-03.md)。
- [x] **B3 复审处理（2026-09-03，全面复审）**：疲劳档位状态机收口——压缩
  轮数门槛移除（50/70/90 只按 W 水位）、巨幅跳跃只报最高未提醒档且已越线
  低档一并落档不滞留补发（`FatigueDecision.tiers_to_mark`）；close 时有
  in-flight run 则存档推迟到该 run 收尾补触发（`pending_archives` 票）；
  存档同步 IO 移 blocking 池；注释/口径清理（session8 命名、Windows 替换
  原子性、损坏 sidecar 边界登记）。orz-loop lib 691 / orz-host 243（仅
  既有 flake 单独复跑通过）/ orz-bin 全绿 / fmt-clippy 无新增。处置登记见
  [B3 复审处理](docs/audits/P2-13_B3_REVIEW_HANDLING_2026-09-03.md)。
- [ ] **B4 验证**：S3 重建完成（2026-09-03：Linux musl 三件套 orz
  d4a37fdb，静态/符号/冒烟核证通过，见
  [B4 S3 重建记录](docs/audits/P2-13_B4_S3_BUILD_2026-09-03.md)）；
  S4 复验（web 通道 A/B、折叠态读取与展开、恢复、长会话遥测）待续。

### P2-14 CONTEXT-COMPACTION-FOLD-SNAPSHOT（P2；2026-09-04 设计定稿；S1/S2 已收口，S3–S4 待续）

> 入口：[设计稿](docs/CONTEXT_COMPACTION_BLACKBOARD_FOLD_DESIGN_2026-09-04.md)
> / [BACKLOG P2-14](docs/BACKLOG_AND_PRIORITIES.md) / ADR-0010 §14.54。

- [x] 设计定稿与裁决收口（2026-09-04：R1–R5 按推荐定案、总量 20K 定档；
  ADR-0010 §14.54 转录；CLI_PROJECT_INDEX 登记
  AUTH-COMPACTION-FOLD-SNAPSHOT，`pending`）。
  - [x] **S1（2026-09-04 用户放行第三条路后开工，全面复审处理收口）**：
    r_keep 接线验证（消息轮/LIF 轮 cadence）+ render_fold 快照入口纯
    函数 + v0.3 A–E marker 装配与 run_template_compact 接线 + §7 单测
    矩阵 1–7。落地明细：
    - 第三条路 = 消息补轮章 + 主车道决策轮执行窗 pin：gateway
      `Message.round`（serde default/skip、transport 不上 wire）、声明
      消息只盖 Main 车道轮章（车道范围裁决）、共享折叠分区行 /
      dispatch mirror / DispatchStamp / handle_parent_disposition audit
      mirror 统一取执行窗主轮章（effective_blackboard_stamp）；
    - epoch.rs 快照入口：`render_*_snapshot`（r_keep 过滤 + pre-stamp
      强制折叠归 C）+ `select_annotations_closest_to_window`（最接近
      近窗 ≤N 条 + 溢出计数）+ `cap_fold_view` 复用（B 明细视图 cap）；
    - summary.rs v0.3 A–E 块装配（A 600 / B 8K / C 30 / D 3K / E 1K、
      总量 20K 定档 + env 覆盖）、D 溢出随存档 annex、`SUMMARY_MARKER_
      ESTIMATE_TOKENS` 重校准 11K；v0.2 五段路径保留为检索/grill 车道
      与旧会话（消息无轮章）回退；
    - run_template_compact 接线：drain/fallback 后取保留尾首条声明轮章
      作 r_keep，槽构建移到 drain 后；v0.3 主会话 / v0.2 回退双轨；
    - 测试：cadence 锁定 + DispatchStamp 同轴断言、快照 6 项、装配 4 项、
      e2e 主车道 v0.3 marker 断言；orz-loop lib 720 passed / 0 failed /
      3 ignored，fmt/diff 净。全面复审处置与审计证据见
      [P2-14 S1 复审处理](docs/audits/P2-14_S1_REVIEW_HANDLING_2026-09-04.md)。
  - [x] **S2 压缩 e2e（2026-09-04 收口）**：rhythm / fallback /
    session_end / 恢复预检全串行绿——同一主会话一次 run 真实触发
    rhythm → fallback（绕冷却 ×2）→ session_end，压缩零模型调用、滚动
    单 v0.3 marker（A–E 块、无 v0.2 五段槽）逐请求与收尾会话断言；恢复
    预检（§7 矩阵第 8 项复验）后 v0.3 marker 仍在、内容逐字节原样（属
    preamble 恒保留，截断 marker 追加其后，D3-1 write-back 均存活）。
    r_keep 边界在真实长会话的遥测前置（S4 实机复验执行）。证据：新增
    2 项 compact e2e（串行链 + 恢复保留），orz-loop lib 722 passed / 0
    failed / 3 ignored、orz-host ACP 43 passed、fmt/diff 净；审计见
    [P2-14 S2 e2e 审计](docs/audits/P2-14_S2_E2E_2026-09-04.md)。
  - [ ] **S3 Linux musl 重建**（沿用 ORZ-BUILD-MOUNT-001 契约）。
  - [ ] **S4 实机复验 + 遥测**：marker 实际字符分布、块 B/C 溢出频率、
    压缩后 blackboard_read 跟随调用频率、restore 后 marker 可用性。

## P3 — 收尾 / 清理

- [ ] EVIDENCE-LOCAL-BROWSER：Python 路径（retrieval_workflow / evidence_store / pdf_evidence）按 ADR-0010 重新符合性审查或退役。
  - 成熟复用评估（2026-08-16，只读）：明确——生产 Rust 已复用 CDP（Chrome）与 pdf_oxide；Python 路径按此审查/退役。
- [ ] GATE-CHAIN：分层 gate 链与融合 runtime 的最终接线随切片审计复核。
  - 成熟复用评估（2026-08-16，只读）：无——复核既有 assurance 链接线。
- [ ] （可选）提示词补列 observed scope 合法枚举（`full_text_observed` / `partial_text_observed` / `metadata_only`）——P0-B 步骤 6 复核观察登记，verifier 已机械兜底，暂不实施。
  - 成熟复用评估（2026-08-16，只读）：无——提示词枚举补列。
- [ ] V11-IMPL-003：Global Review receipt 与真正审查结论严格分离——复核并登记闭合或转 gap。
  - 成熟复用评估（2026-08-16，只读）：无——复核登记。
- [ ] V11-IMPL-007：Toolbar/run-history 数据源统一到 ORZ session ownership、旧路径残留——复核并登记闭合或转 gap。
  - 成熟复用评估（2026-08-16，只读）：部分——orz 即 Grok Build fork，原 toolbar/session 代码在仓库内；Codex app-server 投影为成熟参考。
- [ ] orz-host 既有 flaky（`approval_allow_persists_for_identical_bash`）——复核并登记闭合或转 gap。
  - 成熟复用评估（2026-08-16，只读）：无——测试修复。
- [x] DC 硬信号 4/6：`same_module_no_evidence` / `key_surface_unexamined` 接线（原建议并入 P0-B，批次已闭合，独立待办）——**2026-09-01 随 P2-11 DC 强制模板轮清理一并退役**（信号与机制随删除，不再单独接线；2026-08-31 裁决登记）。

## 审计登记边界（条件触发，不占当前优先级）

- [x] GLM-2026-09-04 外部只读审查候选处置（2026-09-06 用户裁决：F1 /
  R-1 / R-2 / R-3 / 观察 (c) 完成，F2 登记 GAP-APPROVAL-PROMPTER，同日
  排期 0n 后延期）
  ——入口：[登记审计](docs/audits/GLM_EXTERNAL_REVIEW_REGISTRATION_2026-09-04.md)
  / [处置审计](docs/audits/P0_GOV_GLM_DISPOSITION_AND_TASK_D_S2_SCHEDULE_2026-09-06.md)。
- [x] orz read_file `.gsa` 符号链接旧回归测试与 Task C canonical 沙箱语义
  冲突（GAP-GSA-SYMLINK-STALE-TEST，2026-09-06 复核登记）——已按用户裁决
  对齐 Task C：旧测试改写为拒读安全回归测试（orz `a29f7377`）；连带观察
  （permission.rs `.gsa` 白名单对 read_file 不可达）登记于 BACKLOG 00a 待
  裁决。入口：[处置审计 §5](docs/audits/P0_GOV_GLM_DISPOSITION_AND_TASK_D_S2_SCHEDULE_2026-09-06.md)。
- [ ] orz-host 可选后端接线（lsp / memory / 图像 / 视频 / MCP）：接线时翻转能力访问器并补翻转测试（FUS-TOOL-PROBE 边界）。
  - 成熟复用评估（2026-08-16，只读）：明确——LSP/MCP 为成熟开放标准，仓库内已有 orz-mcp；图像/视频走成熟服务 API。
- [ ] headless 计划模式能力信号（plan 模式探针当前以交互用户信号代理，未来 headless 计划模式需独立信号）。
  - 成熟复用评估（2026-08-16，只读）：部分——Codex headless/plan 模式可参考。
- [ ] 下一次真实运行捕获自然携带 23 工具分区 journals（当前 12 个为已提交 fixtures 重建）。
  - 成熟复用评估（2026-08-16，只读）：无——纯运行收集。
- [ ] B-1 后续：canonical URL/host 级去重留待预筛步骤 3；若 host/loop 拆为跨进程边界，补 `ToolResult.structured` 序列化契约。
  - 成熟复用评估（2026-08-16，只读）：部分——成熟 url 库/PSL 与 serde/JSON Schema。
- [ ] ORZ-RECOVERY-TOOL-OUTCOME：崩溃恢复工具结果词汇（`TOOL_NOT_STARTED` / `TOOL_OUTCOME_UNKNOWN` 合成 Tool 消息 + 只重试只读/幂等指引）——出现恢复面 400 或副作用未知证据时实施（单点修复）。
  - 成熟复用评估（2026-08-16，只读）：部分——Codex/Grok 工具生命周期与句柄化控制可参考。
- [ ] ORZ-STAGNATION-TOOL-SIGNAL：停滞守卫「同工具同参数」信号——出现「同参循环且输出持续变化」证据时在 stagnation guard 内加最小计数信号。
  - 成熟复用评估（2026-08-16，只读）：明确——仓库内已有 Grok 血统 stagnation 模块（orz-assurance/orientation/stagnation.rs）可直接扩展。

## 近期已闭合（供核对，不计入开放项）

- [x] FUS-TOOL-PROBE：P0-A 步骤 1-7 与 P0-A-2 全部闭合（ADR-0010 v1.8，23 个工作工具单一探针面）。
- [x] FUS-SOURCE-WEIGHTING-IMPL：来源加权实现闭合（机械三档 + 机器可读种子名单 + 模型加权标注）。
- [x] GAP-ENCODING-GATE：机械编码门控闭合。
- [x] GAP-ACAF-SLICE1 / SLICE2A / SLICE2B / FAILCLOSED 与 GAP-DENIAL-POLICY-REVISION：ACAF 实施切片闭合（fail-closed 生产启用已随 P2 IMPL-CONTROL-FABRIC 闭合）。
- [x] OPS-PROTOCOL 审查判定登记（裁剪方向定案；裁剪设计待产出）。
