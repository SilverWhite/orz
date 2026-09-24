# 072 — 本轮全仓审查与修复批（2026-09-24）

> 工作区 `D:\CLI`；orz 子模块 HEAD `a46c7c02` / 父仓 HEAD `b5e084ed`；worktree 含 0bi/0bl 未提交改动（按令**不提交／不推送／不重建**）。
> 用户令：「请对当前项目进行全面详细审查，审查内容包括设计合理性，实现合理性，设计与实现的符合性，如审查压力大可派出子代理」「请对审查出的全部问题进行严格处理，如实现压力大，请派出子代理协助」。
> 执行形态：四审查子代理并行（设计面／核心运行时／工具与保障面／符合性）→ 主代理逐行复核两条最重代码发现 → 三修复子代理并行落码／修档 → 一断言同步子代理收口宿主测试面 → 主代理落本档并过门禁。

## §0 结论速览

- **审查结论**：无 P0。P1 四处（实现一处＋文档三处）；P2 八处；P3 若干（§2）。**符合性抽查全部通过**：四个 ADR 的参数与语义要么与现行条款一致、要么有 ADR-0010 显式修订链，无静默漂移；0bi/0bh/0bg 档所有「已落码」声明均有真实代码对应物，测试计数精确吻合（orz-loop 842＝839＋3 逐一吻合）。
- **处置结论**：**落码十件**（0bl ①–⑩，§3.1）＋**文档九项**（§3.2）＋**附带收口一件**（0bi ⑩ 宿主断言同步，§3.3）＋**待裁决八件**挂 0bl 不占计数（§3.4）。
- **红判据（本批口径）**：`cargo test -p orz-tools --lib` **2926 passed / 0 failed / 6 ignored**（含新增 7 钉子，连跑 4 次稳定）；`cargo test -p orz-loop --lib` **845/0/3**（基线 842 全绿＋3 新测试）；`cargo test -p orz-host --lib -- --test-threads=1` **322/0/5**（0bi ⑩ 断言同步后复绿，数字与 0BI 档口径一致）；`python scripts/check_repository.py` 唯一红＝orz 脏树预期态；台账钉子 15 tests OK。

## §1 范围与方法

- 审查面：`adr/`（11 份全读）＋`architecture/`（契约抽样）＋流程账目（INDEX／TODO／BACKLOG／cases）＋近期设计档抽深三份（中性终态／emoji 剥离／黑板指针）；代码面 `orz` 三大区（orz-loop 87k／orz-host 31k／orz-tools 139k＋orz-assurance 33k，抽样策略化＋红旗扫描）；符合性面双向核查（文档承诺 → 代码证据，含未提交 diff）。
- 复核纪律：两条最重代码发现（锚点末空行、压缩守卫公式）由主代理逐行亲核确认后才派修复；orz-host 8 红先经 stash 基线对照证明与本批改动无关再处置。

## §2 审查发现总账（处置状态随注）

### §2.1 设计面

| # | 级 | 发现 | 处置 |
|---|---|---|---|
| F1 | P1 | ADR-0001 状态悬空（Proposed），核心子系统 orz 零命中，实际路线为 ADR-0010 | ✅ 处置注记＋状态 superseded (partial) |
| F2 | P1 | ADR-0008 修订链过期（默认无限轮已由 §14.55 取代，终止性保证未重述） | ✅ 二次取代注记（2026-09-04） |
| F3 | P1 | ADR-0010 §4.5 守卫表「300 秒 kill_active」行已被 §14.55 取代但未就地注记（同文件反例门却有范本） | ✅ 就地注记（仿 :683-686 样式） |
| F4 | P2 | ADR-0010 体量 5852 行／§14 共 78 节，一次裁决三处同写 | ⏳ 待裁决 (a) 分卷冻结 |
| F5 | P3 | 阈值类 ADR 一次成型能力弱（ADR-0005 案例） | ⏳ 挂 (h) 模板增补回滚路径一并考虑 |
| F6 | P2 | architecture/ 根 29 份 pre-ADR-0010 文件无取代状态头（§7.2.6 移档未执行） | ✅ 29/29 就地冻结状态头（偏离登记见 §5-4） |
| F7 | P3 | 旧契约 → orz 落点缺总映射 | ⏳ 未立项（低频查阅面） |
| F8 | P1 | INDEX 头行 84 条全部标 `current`，语义失效（v4.34 行 1058 字符） | ✅ 84→1 滚档＋v4.34 上位 |
| F9 | P2 | 批次入账写点约 5 处／跨两仓，计数手工递增 | ⏳ 机制化单源待后续批（本批已按门禁完成一致性同步） |
| F10 | — | 案例库为流程体系性价比最高件（正面） | 维持 |

### §2.2 实现面（核心运行时＋工具与保障）

| # | 级 | 发现（证据） | 处置 |
|---|---|---|---|
| C1 | P1 | 锚点行窗重建吞文件末空行（`search_replace/mod.rs` 后缀分隔符换行与终结符不可区分；`"a\nb\n\n"`→`"X\nb\n"`） | ✅ 0bl ① |
| C2 | P2 | 压缩守卫 `after` 公式两段并回全量数组，非 force 恒 GuardBlocked（`agent_loop.rs:873-875`；生产调用恒 force=true 掩盖） | ✅ 0bl ⑤ |
| C3 | P2 | 取消不打断在途工具（工具执行 select 无取消臂，延迟可达工具超时上限） | ✅ 0bl ⑥ |
| C4 | P2 | 工具超时杀树进程级全局误杀（`orz-host/lib.rs` `kill_active`） | ✅ 0bl ⑦ |
| C5 | P2 | hashline 编辑面绕过 BOM/emoji 单点（裸写 UTF-8 字节） | ✅ 0bl ② |
| C6 | P2 | 双锚点机制并存且 sha 口径不一（loop 层 `!=` vs 工具层 `eq_ignore_ascii_case`） | ✅ 0bl ⑩＋工具层注释互指 |
| C7 | P3 | emoji 告知行号语义含混 | ✅ 0bl ④ |
| C8 | P3 | 编辑面无 UTF-16 门（可能静默乱码写回） | ✅ 0bl ③ fail-closed |
| C9 | P3 | spawn sink 全局单槽归因竞态（并发杀树测试互扰，stash 基线实证既有） | ⏳ 待裁决 (d) |
| C10 | P3 | 编辑面整读整写无大小上限 | ⏳ 待裁决 (e) |
| C11 | P3 | 锚点模式 CRLF 归一化对混排文件的副作用（继承行为） | ⏳ 待裁决 (f) |
| C12 | P2 | `run_agent_loop` 巨型函数 ~3258 行／40+ 可变局部；orz-loop pub 面过宽 | ⏳ 待裁决 (b) |
| C13 | P3 | RLI 影子通道 3845 行无决策消费者（自认纯观测） | ⏳ 待裁决 (c) |
| C14 | P3 | 三处滞后注释／两处脆弱 unwrap | ✅ 0bl ⑧⑨ |
| C15 | P3 | 写前核证按工具名字符串特判的横切耦合 | ⏳ 待裁决 (g) |
| — | — | 正面：分层无反向依赖；错误必落终端事件；进程回收三重防误杀；shell 面超时/杀树/编码链生产水准；非测试 unwrap 仅 3 处且不可失败；ACAF 门票与 journal verifier 实质到位；concise 面系薄壳无逻辑漂移 | — |

### §2.3 符合性

- **ADR 抽查（0007/0008/0009/0010）全部符合**：32s→180s、90s→30s、40→120→unlimited 均有显式修订链且代码注释引用出处；三入口写入放置与四态降级链逐条落码；权限单源＋遍历式护栏与 v1.68 一致。
- **批次账实相符**：0bi 四件落码、构造点 ×6、diff 统计（11 files +730/−25）逐字吻合；0bg「3 值→9 值」在 `families_s2c.rs` 在位；0bk 立项所依据的实现偏差属实。
- **P3 滞后两处已修**：三处注释（→0bl ⑧）；emoji 设计档状态行（→§3.2-4）。
- **老审计档漂移**：TOOL_AVAILABILITY_GATE_AUDIT 测试 26→24、Rust 双轨未记 → 勘误已补（§3.2-5）。

## §3 处置账

### §3.1 落码十件（0bl ①–⑩，均在 orz worktree，未提交）

1. **锚点行窗末空行修复＋钉子**：收尾检查改「后缀分支无条件补终结换行／非后缀分支维持原判」（`grok_build/search_replace/mod.rs:526-538`）；钉子 2 条（`anchor_window_edit_preserves_trailing_blank_line` 覆盖四形状、`…_crlf_…` 逐字节断言 CRLF＋末空行）。
2. **hashline 接入 BOM/emoji 单点**：读侧取 `had_bom`、既有文件写侧 `encode_text_preserving_bom`＋整体 `strip_emoji`、新文件写路径同剥；告知行随 EditsApplied 尾附；`EMOJI_ENV_LOCK` 上移 `util/emoji_strip.rs` 供两测试域共用；钉子 2 条。
3. **编辑入口 UTF-16 fail-closed**：`utf16_shaped_input`（BOM `FF FE`/`FE FF` 或前 8KB NUL 占比 >30%）双路径设门（锚点＋经典），UTF-8 BOM 不受影响；钉子 3 条。
4. **emoji 告知行语义**：`[emoji 剥离] N 处（写入内容 L1/L2 行）`，doc comment 写明坐标口径；断言同步。
5. **压缩守卫公式修正＋性质测试**：`after`＝头部 `[..first_round_start]`＋保留尾 `[kept_start..]`＋marker（`first_round_start` 计算上移，retain 后重算逻辑保留）；全仓调用点核查确认无 force=false 生产路径（纯潜伏修复）；性质测试 ×2（放行／拦截两态可分辨）。
6. **工具执行专用取消臂**：`run_host_tool_with_timeout`/`…_plan_gate` 增 `cancel: Option<&CancellationToken>` 尾参（select 第三臂，首触杀进程后关臂防电平空转）；生产调用点（并行批 `agent_loop.rs:3645`／串行 `:4380`）传真实 token；loop 侧杀进程用全局作用域兜底（host 私有 LiveCallJob 够不到，残余风险挂 (d)）；新增真实子进程取消钉子（999s 子进程 0.4s 收口，journal `run_cancelled` 终态有效）；取消四场景全过。
7. **超时杀点定向化**：超时臂先按 dispatch token 查 `live_call_jobs` 逐一 `TerminateJobObject`（新增 `terminate_job_handle`，只终止不关句柄），句柄不可得回退全局杀；Cargo.toml 增 `Win32_System_JobObjects` feature。
8. **三处滞后注释更正**（071 状况陈述语义随注）。
9. **两处 unwrap→expect**（不变量入讯）。
10. **verify_content_anchor sha 口径统一**：`!=` → `!eq_ignore_ascii_case`，与工具层一致，注释互指闭环。

### §3.2 文档九项

1. ADR-0001 状态 `superseded (partial) by ADR-0010`＋处置注记（零命中经全库扫描复核后才写入）；2. ADR-0008 二次取代注记（显式重述终止性＝熔断＋墙钟兜底）；3. ADR-0010 §4.5 守卫表就地注记→§14.55；4. emoji 设计档状态回写「已落码（0bi ⑪）」；5. TOOL_AVAILABILITY_GATE_AUDIT 文末勘误（24 测试＋Rust 双轨指针）；6. architecture 根部 29/29 份冻结状态头（仅顶部插入 58 行，零删改）；7. **索引头行收敛**：83 行滚入 `存档/index/CLI_PROJECT_INDEX_FULL_2026-09-24.md`，v4.34 上位后 v4.33 补滚同档（现 84 行），活索引仅留 1 条；8. 0bl 立项（49→50，四处计数一致性同步）；9. current README 补 §14.76 投影。

### §3.3 附带收口：0bi ⑩ 宿主断言同步

0bi ⑩（反例门收窄）落码时 orz-host 集成断言未随更，遗留 8 个确定性红（5 acp＋3 codex_app；单跑复现）。本批按「断言随落码更新」惯例同步为 9 事件／每 prompt 一轮语义（仅测试脚本与断言，零生产代码）：`acp_server.rs` 五处（11→9、脚本 4→2/2→1、`reqs.len()` 对应收缩、失败路径语义改写）、`codex_app.rs` 三处（脚本 2→1、失败轮空脚本耗尽）。串行档复绿 **322/0/5**，与 0BI 档口径一致。

### §3.4 待裁决/后续批八件（挂 0bl，登记不占计数）——〔2026-09-24 同日全数裁决，见 §7〕

(a) ADR-0010 §14 合并回正文＋分卷冻结机制；(b) run_agent_loop 状态体拆分；(c) RLI 影子通道转正/退役判据；(d) spawn sink 全局单槽竞争；(e) 编辑面大文件上限阈值；(f) CRLF 归一化副作用声明；(g) 写前核证横切逻辑下沉公共层；(h) 设计档模板增补「回滚路径」必填小节。

## §4 验证

- `cargo test -p orz-tools --lib`：**2926/0/6**（连跑 4 次；新增 7 钉子全绿）。
- `cargo test -p orz-loop --lib`：**845/0/3**（＋3 新测试；cancel 族 9 passed 两次复跑）。
- `cargo test -p orz-host --lib -- --test-threads=1`：**322/0/5**；`cargo fmt -p orz-host -- --check` clean。
- `python scripts/check_repository.py`：error_count＝1（唯一红＝orz 脏树预期态）；台账钉子 `assurance.tests.test_ledger_consistency_nails` 15 OK；v4.34 头行 1058 字符 ≤1200。
- 改动面：orz 三 crate（tools 3 文件／loop 5 文件／host 4 文件）＋主仓文档（adr×3、architecture×30、docs×3、账目×3、存档×3、INDEX）；全部留 worktree 未提交。

## §5 摩擦与如实记录

1. orz-tools 全量首跑出现 1 次未复现瞬时失败（环境敏感 PowerShell 钉子族），后续 4 次连跑全绿；如复发建议单独排查。
2. 并发模式下 orz-host 进程族 6 测试偶红系**既有** spawn-sink 全局归因竞态跨测试互扰（stash 基线对照实证与本批无关）；根治需 per-dispatch sink 传递，挂待裁决 (d)。
3. 0bl ⑥ 落地形态优于立项设想（select 专用取消臂 vs 心跳臂检查，取消响应不受 60s 心跳周期约束）；loop 侧杀进程仍走全局作用域兜底，已注释如实记录。
4. §7.2.6 移档规则的**偏离裁决**：architecture 29 份采用就地冻结状态头而非移入 `存档/architecture/pre-adr-0010/`（引用面约 200 份文档，移档需全量回缠）；状态头内已自述替代关系，正式移档留待后续批。
5. 既有 fmt 漂移两文件（`grok_build/read_file/mod.rs`、`util/emoji_strip_ranges.rs`）未触碰，不属本批。
6. 基线对照期间对 orz-host 两文件做过两次 stash push/pop，已恢复并核验 diff 完整。

## §6 入口

[`TODO P1-0bl`](../../TODO.md) / [`BACKLOG 0bl`](../BACKLOG_AND_PRIORITIES_2.md) §1.10 / [`INDEX`](../../CLI_PROJECT_INDEX.md) / orz worktree（orz-tools·orz-loop·orz-host 未提交改动）。下一批入口：八件已于同日裁决（§7）——七件立项 **0bm**（49 → 50），RLI 转观察件。

## §7 裁决批（2026-09-24：0bl §3.4 八件·用户逐件裁决 → 0bm 立项）

主代理对八件裁决**全数同意**；七件立项 0bm（49 → 50），RLI 转观察件（不占计数）。实施约束随 0bm 条目：

| 件 | 用户裁决 | 主代理裁定与实施约束 |
|---|---|---|
| (a) ADR-0010 | 该拆就拆：拆成三份，再直接开新的第四份 | 同意。约束：**原文件保留为入口锚**（不破 ~200 入站链接，`check_repository` 等脚本引用面先盘点）；拟卷一正文基线（冻结）／卷二 §14 裁决流水 v1.1–v1.78（冻结）／卷三现行有效口径操作投影／卷四新裁决活卷（v1.79 起，主文自此不再增长） |
| (b) run_agent_loop | 需要拆 | 同意。两步走：先状态聚合 struct＋18 参 context 对象（机械搬移，orz-loop 845 测试兜底），后按 pub 消费面清点下沉子 crate |
| (c) RLI | 还在测试中，暂不处理，留作观察项；没价值则回到域判定状态、将其退化 | 同意。登记为观察件（不占计数，candidate 惯例）；退化判据＝后续 release 全量档／狗粮轮无决策面真实消费 RLI 输出，则退化回域判定状态、冻结谐振扩展 |
| (d) spawn sink | 要做 | 同意。per-dispatch 传递改造 orz-tools `xai-tty-utils` API；七件中优先（并发杀树测试互扰已实证） |
| (e) 编辑面大文件上限 | 同意主代理判定，应该做 | 同意。16–32MiB 档实施时定档＋`ORZ_…` env 逃生＋明确报错，对齐读面粗门风格 |
| (f) CRLF 归一化副作用 | 应该做（真修） | 同意。逐行行尾保真（per-line preservation）；钉子覆盖混排行尾，不回退 0bl ① 四形状 |
| (g) 写前核证下沉 | 直接重构下沉机械层，不定为规则 | 同意——规则化会转嫁为模型多考虑一步，与「机械层强制、模型侧零负担」的项目哲学一致 |
| (h) 回滚路径 | 机械层硬编辑留回退供模型选择；告知时一并将回退窗口交给模型 | 同意。机制＝硬编辑预存回退窗口＋告知行携带回退指针（模型可选择性回退）；连带设计档模板「回滚路径」必填小节（仅约束新增，存量不回填） |

入账：BACKLOG 0bm 条目＋TODO P1-0bm＋卷二 §1.11＋INDEX v4.35（v4.34 滚档）。

补充裁决（同日）：**0bm 执行形态＝狗粮长轮**——用户令「0bm 也定为新的狗粮长轮，不直接进行处理」；主代理不直接落码，待下发真机轮执行（轮内 S1→S2→S4，轮达成后审计入账）；前置建议（下发时定）＝重建载体纳入 0bl 十件未提交落码，使轮内编辑面／取消面改动自举可用。
