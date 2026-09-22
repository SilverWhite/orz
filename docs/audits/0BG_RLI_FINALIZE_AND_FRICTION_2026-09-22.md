# 0BG — RLI 收尾与摩擦大杂项轮（2026-09-22）

> **轮次性质**：P1「RLI 收尾与摩擦大杂项轮」S1⇒S4 全批落地（用户令：**不提交/不推送/不重建**；摩擦随报告）。
> **真机轮**：本会话 `RUN-CLI-6ab274e2`（0.6.9 在役载体；本批改动待在下一个重建批随 0.6.10 生效——S3 读数为**旧二进制在役**下的观测）。
> **基线**：父仓 069 重建批已暂存未提交（索引 v4.25）；orz 子模块 `88b6dea1`（ahead 1、未推送、bump 0.6.9）；RLI 常开（env=1）。

## §0 结论速览

- **九件：全部落码**（①–⑨；①⑤②含处置/核证类，其余为落码类）。其中 **⑤实锤升级**：Rust `verify_mechanical_audit` 只认 3 值 vs schema/Python 8 值 ⇒ 真 journal 会被记假错（**对拍空洞坐实**），本批重写为 9 值＋按 kind 分支校验。
- **三项 RLI 裁决全采纳**：CoverageGap（第三 kind：就绪/越线/沿一次/域切换重武装/每轮≤1 域类提醒/随报 g＋未覆盖列表＋r＋域失配）；注解随报（三 kind＋面头符号表；**本体＋注解 ≤240 B/条**）；双迁移（删「域迁移+n」徽标＋机械记录 `lif_domain`，模型面只留 RLI「域迁移确认」）。
- **验证**：`orz-assurance` lib **275 通过**；`orz-loop` lib **830 通过**（3 ignored）；`orz-host --tests` `cargo check` 通过；controller 专项 41 通过。宿主全量**串行**测试未在本批执行（属①的判据入口，且按用户令不重建——留下一批）。
- **⑧⑨落定**：压缩窗口取值开销口径＝三面合读（见 §4.3）；标定＝Slow horizon 1→5、Deny 30→10，Err/Stall/Prog 与权重/锚保持（见 §4.4）。

## §1 范围与基线

| 项 | 值 |
|---|---|
| 批 | 0bg（RLI 收尾与摩擦大杂项轮） |
| 会话 | `RUN-CLI-6ab274e2`（起 2026-09-22 20:30:29+08；无墙钟、轮次无限） |
| 载体 | orz 0.6.9 在役（Windows `D:\tb-eval\orz-windows`；Linux `D:\tb-eval\orz-linux`） |
| 父仓 | 069 批已暂存未提交（索引 v4.25＋069 档＋三脚本 BOM＋子模块 pin＋manifest 1463 条） |
| 子模块 | `88b6dea1`（ahead 1、未推送） |
| 题面 | `.tmp-0bg-task.txt`（292 B、带 BOM；journal prompt 正确） |
| 环境 | `ORZ_LIF_RLI_SHADOW=1`（常开）、`ORZ_MAX_WALLCLOCK=0` |

## §2 九件对账（逐件）

### ① 宿主并行敏感红处置
- **现象**（0bd §5）：串行 322/0/5 全绿；并行 320/2/5（`call_tool_timeout_kills_process_tree` 与 `run_tests_output_scrubbed_of_secrets` 或 `run_terminal_cmd_truncation_carries_output_object`）。
- **处置**：a) `call_tool_timeout_kills_process_tree` 的**续跑调用改显式 30 s 覆盖**（`call_tool_with_timeout(…, Some(30s))`——语义保留：续跑仍被显式预算打断，但不再撞 2 s 默认（0bd 定位的竞态窗口）；b) `scripts/run_orz_tests.ps1` **头注文档化「串行档为 orz-host 红判据入口」**＋auto 降并行档＋提示；c) 两条竞态（scrubbed／truncation）**仅登记**（不放宽断言强度——留后续设计批）。
- **验证**：`cargo check -p orz-host --tests` 通过；宿主全量运行留待下一批（不重建）。

### ② `reclaim.rs:478` dead_code 清零
- `write_file`（L478-483）定义后零引用（实测 grep 坐实）⇒ **删除**。
- 验证：`cargo check -p orz-host --tests` 通过（警告清零）。

### ③ RLI 面 1 KiB 截断的可见性（摩擦 g）
- **现象**（本轮 S3 直证）：now 面在 434 步时截断落在 `slow` 行**中部**（`…p1(1T̂)=2…(truncated)`）——「繁杂度」行常落窗外。
- **处置**（用户裁决 (c) 同批）：now 面**重排**（头行→面头符号表→自判域→转移倾向→**繁杂度前移**→近提醒→上一迁移→分通道），**预算内装配**（`NOW_BUDGET=830` B）＋**显式省略行**（`…已省 n 行（分通道明细 selector=channels；域历史 selector=history）`），超预算从尾部让位（分通道→近提醒；核心行不剪）；新增 **`selector=channels` 折叠面**（分通道明细，渲染与 now 尾段单一来源）；`enforce_bound(content, 1024)` 尾截断保留为最终护栏。
- 验证：渲染测试（含 channels 面与 1 KiB 界）全绿；真机新形态待 0.6.10。

### ④ 测试入口 auto 降并行
- `scripts/run_orz_tests.ps1`：未显式 `-j` 时按 **commit 阈值**自动档（usedPct≥88% 或 free<4 GiB→1；≥72% 或 <12 GiB→2；否则 min(核数,8)）＋「orz-host 串行」提示（逻辑镜像 `build_orz.ps1` 的 auto）。

### ⑤ 侧车 drain 落盘核
- 0bd 会话 `6ab17325.json`：已落盘、含 `lif.rli_shadow` **11 字段**（`domain.current=low_progress`、`sample_points=499`、`grid_samples=122`）；顶层**无** `burden_tiers_notified`，与 journal `user_notice=0` 一致（预期口径）。
- **同批实锤（0bd 登记的「对拍空洞」）**：`families_s2c::verify_mechanical_audit`（Rust）只认 3 值，而 schema/Python 已 8 值 ⇒ 真 journal（0bd 实测 5 种 kind）会被 Rust 验证器**记假错**。本批重写为 **9 值＋`retrieval_batch` 五键分支**校验（与 Python 镜像同规）。

### ⑥ 题面写入端强制 BOM
- `scripts/dogfood_launch.ps1`：写入侧**强制 BOM**（缺则原地重写为 UTF-8 带 BOM）＋**U+FFFD 断言**（读取端既有显式 UTF-8 读＋BOM 探测）；根治「题面 BOM 靠人肉」的摩擦。

### ⑦ `&` cargo 脚本 `--` 容错
- 清单**实核**：仓库内 `& cargo` 调用仅 `build_orz.ps1`／`run_orz_tests.ps1` 两个 PS 脚本（`.sh` 三脚本无 PS 吞 `--` 问题）——两脚本均已具容错（滤前导 `--`／启发式补回）⇒ **收口无新增**。

### ⑧ RLI 压缩窗口取值开销口径（0bf §6-g 遗留）
- **口径（定稿）＝三面合读**：
  a) **侧车快照字节**：`lif.rli_shadow` 快照序列化字节（0bd 会话实测 **24 010 B**；`lif` 合计 24 663 B）；
  b) **压缩窗口内取值**：窗口开启期间的 rli 面读取（次数×字节；本 run 6 次 `context_compressed`、rli 面读取按 ≤1 KiB 帽计；journal 可机械复算）；
  c) **now 面预算与可见性**：`≤1 KiB` 帽＋本批预算装配/省略行（首屏不再静默截断）。
- **结论**：量级 tens of KB/session（可忽略）；口径可复算，此后批次按此三面报数。详见 §4.3。

### ⑨ 权重与锚标定
- **输入**＝0bf 探针 skill 表（Err h10 **+0.623**；Deny h10 **+0.638**（h30 +0.571）；Stall h2 +0.369；Slow h5 **+0.317**（h1 +0.222）；Prog 弱 +0.144）。
- **处置**：分通道 horizon 重标——**Slow 1→5、Deny 30→10**（各取峰档）；Err=10、Stall=2、Prog=10 **保持**；**权重（0.3）与繁杂度锚（60/80/100）保持**（无新证据触碰）。钉子测试同步（`horizon_table_and_short_anchor_pin`）。
- 详见 §4.4。

## §3 RLI 三项裁决落地

### 3.1 CoverageGap（第三 kind；用户令「直接做掩盖缺口吧」）
- **口径**（S1 勘定稿全采纳）：分母＝四值域枚举（`normal/pressure/low_progress/stuck`；`Start` 不计）；`g`＝未访问域占比；**就绪门**＝已完成段 ≥3；**越线门**＝当前段驻留 ≥ 该域已完段驻留中位（不足退**会话内全域中位**）；**触发沿一次**（报后关闭，**域切换重武装**）；**每轮最多一条域类提醒**（与域迁移确认同轮让位——沿不被吞，条件持续成立即下一轮报）。
- **随报字段**：`g`＋未覆盖域列表（≤2 名）＋驻留比 `r`＋**域模型累积失配**（`Σ max(0, 段驻留−该域已完段中位)` 轮，报 `域失配 x/y 轮`）。域失配**不独立触发**。
- **实现**：`rli.rs`——`RliNoticeKind::CoverageGap`；`RliCoverageStats`＋`RliDomainMachine::coverage_stats()`（判据单一来源）＋`RliShadow::coverage_stats()`；`coverage_gap_text()`＋`maybe_push_coverage_gap()`（可单测）；`RliShadow.coverage_gap_armed`（快照持久，legacy 缺字段=**true**）；`on_decision_round` 域切换检测重武装＋本轮已有迁移确认时让位。
- **测试**：`coverage_gap_fires_on_edge_and_respects_budget`（3 已完段/未访 `low_progress`/`g=0.25`/驻留 6>中位 3 ⇒ 触发；关闭不重发；重武装再发；文案断言 `域失配 0/6 轮`）。

### 3.2 注解随报（用户令「该加的注解都加上」）
- **形态**：固定模板、**自含**（术语释义／基准／非阻断声明）；**单一源** `rli_notice_annotation(kind)`；预算 **`RLI_NOTICE_TEXT_BUDGET=240 B/条`**（本体＋注解；`push_notice` 统一追加——三构造点不散落）。
- **覆盖**：三 kind 注解＋**面头固定符号表一行**（`RLI_SYMBOL_LEGEND`：u/v/pred/p1/E/r/θ/λ̂/ρ/c/T̂）＋机械码释义（不逐行注解，避免挤掉读数——用户裁决 (c)）。
- **测试**：`notice_texts_stay_within_budget`（三 kind 最坏形态 ≤240 B）＋`symbol_legend_names_the_abbreviation_family`（缩写族齐全）。

### 3.3 双迁移（模型面只留 RLI 确认＋机械层连带记录）
- **模型面**：`域迁移+n` 徽章**撤除**（`attach_pull_delta` 删段；迁移计数基线 `MIGRATION_CURSOR_KEY` 退役）；RLI「域迁移确认」（震荡稳定后一次）不变；**域类提醒每次投递 ≤1 条**（第二条留待下次读取，不重不漏）。
- **机械层**：`mechanical_audit_update{kind:"lif_domain"}`，`key="lif.domain_migration"`、`summary="{from}→{to}@r{n}；累计 m 次"`、`anomaly=null`（**只记不发模型**；审查表每键一条覆盖写、不进报告块；逐次历史由 journal 可离线复算）。
- **四条同步**：schema `mechanical-audit-update-event-payload-v0.2`（enum＋uniform 分支＋描述）／Python `_verify_v02_mechanical_audit`／Rust 契约钉（`mechanical_audit.rs`）／Rust 验证器（`families_s2c::verify_mechanical_audit` 9 值重写）。
- **消费点**：`AgentLoopController::take_new_lif_migrations()`（内部游标 `lif_migration_recorded`；**累计序数**＝`(total−history.len())+j+1`，有界 history 口径）＋`agent_loop` 逐轮（预算记录后）落 journal。
- **测试**：迁移两测试改断言「**无**徽章」＋消费计数断言（首取 3、后取 2、序数 4/5 准确）。

## §4 S3 真机读数（本会话 `RUN-CLI-6ab274e2`；0.6.9 在役）

### 4.1 journal
- 事件面 37 种；`mechanical_audit_update` 合计 300+ 条，kinds＝`{plan_write_guidance:1, budget:226, tool_result:107, context_scale:5}`——**全部落在 9 值词表内**（对照 0bd：`{plan_write_guidance:2, budget:161, tool_result:128, context_scale:6, model_compression:1}`；两 run 合计即坐实 Rust 验证器旧三值集合的假错面）。
- `user_notice`＝**0**；`context_compressed`＝**6**。
- 无 `retrieval_batch`／`lif_domain`（前者本 run 无检索批；后者 0.6.9 无此 kind——本批待 0.6.10）。

### 4.2 rli 面（now；读时快照）
```
rli.now → [RLI on | 步数 434 | 采样 495（网格补点 61）| T̂=4.3s | 自判域 normal]
自判域行: r210 [1512s | normal | 入域 r209 | 驻留 2 轮] | 输入 u_err=0.00 v_err=+0.000 E_err=0.00 u_prog=0.82
上一迁移: low_progress→normal@1504s (round 209)
转移倾向: [normal] 驻留 2 轮 · 完成段 14（累计 190 轮 · 去向 low_progress×14）· 离开率≈0.074/轮 · 段驻留中位 11 轮
近提醒: 域迁移确认: low_progress→normal@r202（稳定 3 轮；失配概率 0.00） | 持续越线: slow×5（u=2.70≥θ=1.40；失配概率 0.15）
  err: u=0.00 v=+0.000 pred(10T̂)=0.00 p1(1T̂)=0.00 E=0.00 r=1 θ=0.70 hits=15
  stall: u=0.35 v=-0.036 pred(2T̂)=0.08 p1(1T̂)=0.20 E=0.52 r=1 θ=0.67 hits=14
  slow: u=2.68 v=-0.006 pred(1T̂)=2.65 p1(1T̂)=2…(truncated)
```
- **③直证**：截断（`(truncated)`）落在 `slow` 行中部——1 KiB 帽吞掉其后两行通道＋繁杂度行（新形态：省略行点名 + `selector=channels` 折读）。
- **头部徽章**（同刻）：`rli+367 域迁移+20: low_progress→normal@r209`——旧徽章在役证据（双迁移改动生效前）。
- RLI 会话规模：步数 434／采样 495（网格补点 61）；迁移累计 20 次；完成段 14（累计 190 轮）。

### 4.3 ⑧ 压缩窗口取值开销（三面读数）
- a) **侧车快照字节**（0bd 会话 `6ab17325.json` 实测）：`lif.rli_shadow`＝**24 010 B**；`lif` 合计 24 663 B；侧车总 2 384 689 B（rli 影子占比 ≈1%）。
- b) **压缩窗口内取值**：本 run 开窗 **6** 次（`context_compressed`）；窗口期间 rli 面读取 ≤1 KiB/次（journal 可核）；另有 pull-delta 头的 rli 徽章（每读挂，字节数 ≈「rli+N」小数 B）。
- c) **now 面预算**：≤1 KiB 帽不变；本批增预算装配（830 B）＋省略行（首屏不静默截断）。
- **结论**：开销量级 tens of KB/session；口径机械可复算（侧车字节＝jq/脚本可测；读取字节＝journal；预算＝渲染测试）。

### 4.4 ⑨ 权重与锚标定（落码面）
- `rli_horizon_steps`：Slow **1→5**、Deny **30→10**（skill 峰档）；Err/Stall/Prog 保持 10/2/10。
- 权重 0.3、繁杂度锚 60/80/100、θ 三档自校准——**保持**（无新证据）。
- 钉子：`horizon_table_and_short_anchor_pin` 更新（含「Slow h=5 起与短视锚点分列」断言）；渲染面 `pred(5T̂)`/`pred(10T̂)` 断言更新。

## §5 摩擦项清单（本轮执行摩擦＋结构摩擦）

**执行摩擦（本会话）**
- a) **上下文压缩打断**：`[CONTEXT_SCALE]` 硬提醒 2 次（272 K／289 K 估算）＋6 次 `context_compressed`；压缩窗口与工具批注叠加，多轮节奏被打断（压缩机制本身按设计工作，仅登记为节奏摩擦）。
- b) **serde 双函数陷阱**：`default = "is_true"` 需**零参**函数、`skip_serializing_if = "is_true"` 需**借用参**函数——单函数两用编译错（E0061）；拆为 `is_true_default()`／`is_true(&bool)`。
- c) **API 形态差**：`RliShadow::notices()` 返回 `Vec<&RliNotice>`（非 `&VecDeque`）——测试 `.back()` 编译错；改 `.last().copied()`。
- d) **累计序数公式**：`take_new_lif_migrations` 首版把 history 下标当累计序数（漏 `offset=(total−len)`）——测试「左侧 5 ≠ 4」暴露；已修＋注释。
- e) **构建时长**：`orz-loop` 编译 ≈2–3.5 min/次（增量），迭代旋回偏慢；测试入口 auto-j 不涉编译并发。
- f) **日期笔误**：注释误写 2026-09-27（28 处）——批修为 2026-09-22（脚本核证 BOM 保持）。
- g) **后台测试日志重定向**：`Start-Process -RedirectStandardOutput` 下测试进行中日志为空（缓冲/句柄行为）——改前台跑（登记为工具摩擦）。
- h) **内存软提示**：编译期 commit 3.65 GiB／通知线 3.09 GiB（`limit_flags 0x200`）——本批未受影响。
- i) **在役二进制差**：S3 全读数出自 0.6.9（无本批改动）——「改动生效」的读数窗天然缺失（用户令不重建），报告显式标注。

**结构摩擦（代码库层面，已在本批修复/登记）**
- j) **对拍空洞**（0bd 登记、本批实锤修复）：Rust 验证器 3 值 vs schema/Python 8 值 ⇒ 真 journal 记假错。
- k) **双轨迁移**：模型面旧徽章＋`MIGRATION_CURSOR_KEY` 双基线长期并行——本批退役，模型面/机械层分工清晰。
- l) **now 面静默截断**：超预算内容无指针（找不回）——本批改显式省略行＋折叠面。

## §6 验证记录
| 命令 | 结果 |
|---|---|
| `cargo check -p orz-assurance` | ✅（is_true 修正后） |
| `cargo check -p orz-loop` | ✅ |
| `cargo check -p orz-host --tests` | ✅（reclaim 删码后） |
| `cargo test -p orz-assurance --lib` | ✅ **275 passed / 0 failed** |
| `cargo test -p orz-loop --lib` | ✅ **830 passed / 0 failed / 3 ignored** |
| `cargo test -p orz-loop --lib controller::` | ✅ 41 passed（含迁移/渲染专项） |
| 锚点资产 | schema/Python/Rust 三件 `lif_domain` 同步；契约钉两处 |

## §7 遗留与下一步
- **宿主全量串行测试**（①判据入口）：留待 0.6.10 重建批（本批不重建）。
- **两条竞态**（`run_tests_output_scrubbed_of_secrets`／`run_terminal_cmd_truncation_carries_output_object`）：仅登记，未设计。
- **新面真机数据**：CoverageGap／注解／`channels`／`lif_domain` 的首次真机读数待重建后狗粮。
- **提交/推送/重建**：按用户令**未做**（改动留工作树；父仓 069 暂存批原样）。
- 清理：`.tmp-fixdate.py`（辅助脚本）删除；`.tmp-0bg-task.txt` 保留（题面）。

## §8 改动文件清单
**父仓（D:\CLI）**
| 文件 | 改动 |
|---|---|
| `orz/crates/orz-host/src/reclaim.rs` | 删 `write_file` 死代码（②） |
| `orz/crates/orz-host/src/lib.rs` | 续跑调用显式 30 s 覆盖（①a） |
| `scripts/run_orz_tests.ps1` | 头注＋auto 降并行＋串行提示（①b/④） |
| `scripts/dogfood_launch.ps1` | 写侧强制 BOM＋U+FFFD 断言（⑥） |
| `runtime/mechanical-audit-update-event-payload-v0.2.schema.json` | `lif_domain`（enum＋uniform 分支＋描述）（§3.3） |
| `assurance/run_event_journal_validation.py` | `lif_domain`（§3.3） |
| `docs/audits/0BG_RLI_FINALIZE_AND_FRICTION_2026-09-22.md` | 本报告 |

**子模块 orz（`88b6dea1` 工作树）**
| 文件 | 改动 |
|---|---|
| `crates/orz-loop/src/mechanical_audit.rs` | `KIND_LIF_DOMAIN`＋契约钉两处（9 值） |
| `crates/orz-assurance/src/journal/families_s2c.rs` | `verify_mechanical_audit` 重写（9 值＋`retrieval_batch` 分支；⑤） |
| `crates/orz-assurance/src/lif/rli.rs` | CoverageGap 机制＋注解随报＋符号表＋预算＋域失配＋horizon 标定＋4 新测（§3.1/3.2/§4.4） |
| `crates/orz-assurance/src/lif/mod.rs` | re-export 补常量（3） |
| `crates/orz-loop/src/controller.rs` | now 面重排/预算/省略行/`channels` 面/`rli_channel_lines`；attach_pull_delta 删徽章＋域类≤1/次；`take_new_lif_migrations`＋`lif_migration_recorded`；描述与测试更新（§3.3/③） |
| `crates/orz-loop/src/agent_loop.rs` | 逐轮 `KIND_LIF_DOMAIN` 连带记录（§3.3） |

## §9 勘误（主会话 2026-09-22 回查；沿「补勘误指针、不改原读数」惯例）

1. **§5 a 档位误标**：原稿写「`[CONTEXT_SCALE]` 硬提醒 2 次（272 K／289 K 估算）」——实为**软档 256K／288K**（模型面估算 272 124／289 361，即原稿那两处读数），本轮**硬提醒 0 次**（320K 未触发）。软提醒实际 **4 次**：`first_block` r51／`192k` r58／`224k` r86／`256k` r112／`288k` r173（`conversation.context_scale_notified` 侧车逐键可核）。
2. **§4.1 计数为轮中快照**：收官 `mechanical_audit_update` 实为 **365** 条（`budget` 240／`tool_result` 119／`context_scale` 5／`plan_write_guidance` 1；原稿 339 为读时快照），`context_compressed` **6 → 7**。原稿论断不受影响（全部落在 9 值词表内）。
3. **§5 g 机理勘误**：不是「缓冲/句柄行为」。真机理＝Windows 每次工具调用挂自带 `KILL_ON_JOB_CLOSE` 的 Job（`resource_job.rs`「调用级拆树粒度」）＋`ProcessGroup::Drop → CloseHandle`；前台调用在下一个 poll tick 被回收即关句柄，**调用内拉起的后台进程被连坐杀死**。现场证据：该调用自身输出 9 B 正常、子进程 `RedirectStandardOutput` 文件 0 B、`stderr` 停在两条 `Compiling`；随后模型盲等 90+120 s。处置方向已在 0bh 立项（告知面／可选 breakaway，须裁决）。
4. **§8 表归属勘误**：`orz/crates/orz-host/src/reclaim.rs` 与 `lib.rs` 列在「父仓（D:\CLI）」表内，实为**子模块 orz** 文件（父仓侧改动＝4 文件＋本报告）。

## §10 提交批（2026-09-22 用户放行：「现在请先进行一轮提交并推送吧」）

| 项 | 结果 |
|---|---|
| orz 提交 | **`7da1a9fe`**（八件：`orz-assurance/src/journal/families_s2c.rs`／`lif/mod.rs`／`lif/rli.rs`；`orz-host/src/lib.rs`／`reclaim.rs`；`orz-loop/src/agent_loop.rs`／`controller.rs`／`mechanical_audit.rs`），推 `cli`（`88b6dea1..7da1a9fe`） |
| 父仓提交 | 本档（新增 §10）＋ 069 重建批暂存件（索引 v4.25 头行／069 档／三脚本 BOM／子模块 pin／manifest）＋ 0bh 立项台账三方同步 ＋ **索引条目补登 `GAP-INTERACTION-SURFACE-CLOSEOUT-BATCH`**（并入 §8 `pending` 桶）＋ 子模块 pin ＋ `orz_source_manifest.sha256` 重算 **1463** 条（差异 8 行＝子模块八件），推 `origin main` |
| 载体 | **不重建**——在役 0.6.9 仍为 `88b6dea1` 冻结源；本批源码随 **0.6.10** 进载体（0bh 执行前置即此） |
| 预检 | **免**（默认口径：纯逻辑面且不换装载体） |
| 门禁 | 提交前唯一 `error_count=1`＝「orz submodule working tree is dirty」（子模块待提交的**预期态**）；提交后 `valid: true` |
| 计数 | **维持 46**（0bh 立项已在前批落定；本批不动计数） |

**§7 与本节的边界**：§7「提交/推送/重建：按用户令**未做**」是**轮内纪律**（0bg 任务令原文「任务完成后请不必进行提交/推送/重建，按照项目惯例落一份报告文档即可」）；提交与推送由用户 2026-09-22 **单独放行**，故不构成 §7 的破例——「载体未重建」一条两节一致。

**提交前的两处顺手收口**（不属本轮摩擦，随批记录）：① `orz-host/src/lib.rs` 内 0bg ① 注释日期笔误 `2026-09-27 → 2026-09-22`；② 索引缺 0bh 的 canonical 条目（0bd／0bg 各有 `GAP-FRICTION-BATCH-DOGFOOD`／`GAP-RLI-CLOSEOUT-FRICTION-BATCH`），本批补登 `GAP-INTERACTION-SURFACE-CLOSEOUT-BATCH` 并入 §8 `pending` 桶——命名沿 0bg 同形，用户可随时改判。

**本批并推的 0.6.9 重建批**：`88b6dea1`（bump 0.6.8 → 0.6.9）随本批首次推 `cli`，档见 [`069 重建档`](069_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-22.md)；**不发行**（0.6.7 不补发、0.6.8／0.6.9 均属中间过渡版）。

关键词：0bg、提交批、`7da1a9fe`、9 值验证器、`CoverageGap`、注解随报、`lif_domain`、`selector=channels`、0bh 立项、`GAP-INTERACTION-SURFACE-CLOSEOUT-BATCH`、载体未重建、0.6.10 前置。

> 页脚：0bg｜S1⇒S4 落码完成（未提交/未推送/未重建）｜报告 2026-09-22｜摩擦见 §5。
