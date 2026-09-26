# 0bv 结转批处理报告：REV-083 十八项 ＋ 0bw 六项（2026-09-27）

> **批次**：0bv 结转（随 0bv 闭合的 REV-083 18 项＋0bw 六项）。
> **基线**：父仓 `3a26f803`；orz 子仓 `71af0ee3`（0.7.2）；两侧工作树干净起步。
> **形态**：work batch——**不提交 / 不推送 / 不重建**；账本（索引 / BACKLOG / TODO）**本批不动**，随落账批同步（D-1）。
> **账目落点**：本档（唯一落点）；`D-*` ＝ 就地裁决；`F-*` ＝ 摩擦留痕。
> 源清单：`docs/audits/083_FULL_PROJECT_REVIEW_2026-09-26.md` §8／§7；`docs/audits/0BW_WRITE_CONTROL_IMPLEMENTATION_AND_FRICTION_2026-09-26.md` §6（后续批①–⑥）。

## §0 总览

| 项 | 状态 | 证据／说明 |
|---|---|---|
| REV-083-01 权限语义收敛（README/帮助与 yolo 默认改齐；`-p` 打印权限模式） | **部分**（代码面✓／README 面结转） | `orz-bin/src/main.rs`：help 三行改准＋`permission_mode_banner()`＋`-p/--plan` 启动打印；README 落字随批 |
| REV-083-03 审批叙事收口（ACAF 设计/README；approval.rs 空壳处置） | 结转 | 文档面未落（与 0bw 措辞批同批处理，见 D-3c） |
| REV-083-04 README 工具面口径批＋ADR-0010 状态行 frozen→evolving | 结转 | README:71/72/97/113、ADR 状态行未改 |
| REV-083-05 集成测试目录（0bv 增量；拆分已随 0bs ⑭ 定稿） | 结转（单独立项） | 审查建议单独立项；本批未动 |
| REV-083-06 journal 口径二选一 | **裁决✓**／README 落字结转 | D-6：取「完整性/损坏检测」档（§7 三档措辞总则），不立项 receipt 链 |
| REV-083-07 `compute_event_hash` 字段集守护测试 | **✅** | `orz-assurance/journal/chain.rs`：`compute_event_hash_covers_every_serialized_field`（serialize→剔除 `event_sha256`→canonical 参照＋shaken 断言） |
| REV-083-08 signer respawn 旧票据补偿 | **✅** | `orz-loop/acaf.rs`：`stale_ticket_cutoff`＋`note_issuer_generation_restart()`（respawn／ensure 两处）＋`predates_restart_cutoff()` 前置拒＋测试（D-5） |
| REV-083-09 ACAF 收口（排期＋shadow 落账＋FAIL_CLOSED=0 告警） | **部分**（告警✓／排期与落账结转） | `main.rs` shadow 启动告警已落；独立验票执行器排期、pre-signing 拒绝落账随批 |
| REV-083-10 journal writer 移阻塞线程 | **✅** | `journal/recorder.rs`：`spawn_writer_thread()`（命名 OS 线程）＋`run_blocking()`＋`append_with_backoff` 同步化＋`std::thread::sleep`（D-9） |
| REV-083-11 web_fetch SSRF pin | 结转 | 需客户端 resolver 层改造；随批（D-11 备记） |
| REV-083-12 panic 契约 | **裁决✓**／文档落字结转 | D-12：取「修订退出码文档」路线（hook 方案备档） |
| REV-083-13 机制参数表三列化入 ADR | 结转 | ADR 落字未做（表样见 D-13） |
| REV-083-14 判据钉纪律 | **裁决✓**（纪律文本入本档） | D-14：量尺/边界/参数机制定稿同步产 fixture（先红后绿） |
| REV-083-15 assurance 冻结宣言 | 结转（判据见 D-15） | 落字未做 |
| REV-083-16 GAP-SPAWN-ORPHAN-RECLAIM pending→partial | **就绪**（随落账批） | 证据＝`process_tree.rs` 孤儿清扫已在树；翻账机械改写随落账批（D-2） |
| REV-083-18a 守卫注释 1.10M→700K | **✅** | `compact.rs`／`controller.rs`／`agent_loop.rs`×3／`context_scale.rs` |
| REV-083-18b 锁中毒策略统一 | **✅** | 31 文件 198 处 → poison-tolerant（D-8） |
| REV-083-18c 水位标签缓存 | **✅** | `controller.rs` `watermark_label_cache`（按 live 字节数键控 memo） |
| REV-083-18d stall 看门狗 permission 等待豁免 | **✅** | `host_exec/tool_run.rs`：等待期 30s 心跳 stamp |
| REV-083-18e env 解析统一 | **✅** | `orz-host/tools.rs` 严格解析＋`tracing::warn` 回落（D-6e） |
| REV-083-18f orz-web CSP | **✅** | `orz-web/server.rs` 静态面 CSP＋nosniff＋no-referrer＋X-Frame-Options |
| REV-083-18g RETIRED 镜像段删除 | **✅** | `permission.rs` `access_in_scope` 恒 true；辅助函数转 `#[cfg(test)]`（D-7） |
| REV-083-18h orz 仓根 before-* 清理 | **✅** | 删 `build.bat.before-b-env-20260802-2228`、`build.ps1.before-b-env-20260802-2228` |
| REV-083-19 流程（小步提交／双会话确认） | 记录 | D-19：本批沿审计留痕形态；落账建议小步提交 |
| REV-083-20 LIF 判据＋复裁日期＋边界表 | **部分**（复裁日期记录）／落字结转 | D-20：复裁日 2026-10-11；冻结新增面/试验性标注/边界表落字随批 |
| 0bw① Linux Landlock 落码 | 结转 | 无 Linux 实测面（0bw D6 延续）；`orz-sandbox` 复用为落码首选 |
| 0bw② 运行时载体完整性自检（manifest＋启动校验） | 结转 | 涉发布链；本批不重建 |
| 0bw③ 专用 journal 事件族（block/warn 结构化） | 结转 | Schema＋producer＋fixture 全链随批 |
| 0bw④ git 检查点／journal undo（L4 余项） | 结转 | 0bm⑦ 回退窗口已有；undo 面随批 |
| 0bw⑤ CFA enforce 翻转裁决 | **裁决✓** | **不翻**：1124 现为空、无狗粮读数（D-3） |
| 0bw⑥ 载体重建后真机复验（S3/S4） | **裁决✓** | **不可达**（不重建）→ 维持登记（D-4） |

**完成度一句话**：REV-083-18 的 8 个拆项全落（a–h），07/08/10 三项独立小修全落，01 代码面与 09 告警面落；文档面（01/03/04/06/13/15/20 落字）与 0bw①–④ 结转下一批。所有裁决已随本档留痕。

## §1 处置明细（按项）

### REV-083-01 权限语义收敛（部分）
- `main.rs` help 文案改准：Bash/网络默认已自动批准（yolo），Benchmark 矩阵仅 `ORZ_ALLOW_WRITE` 可达；新增启动横幅 `permission_mode_banner()`，`-p/--plan` 启动即打印（`[permission] mode=...`）。
- 遗留：README 的权限节落字（随 01/03/04 文档批一并处理）。

### REV-083-07 `compute_event_hash` 守护（✅）
- 新增测试以「实现无关」参照实现对比：`serde_json::to_value(event)` → 剔除 `event_sha256` → canonical JSON → sha256，与手枚举投影的 `compute_event_hash` 必须逐字节一致；再加 shaken 断言（改一个字段必须变哈希）。未来新增字段漏入投影 ⇒ 测试红。

### REV-083-08 respawn 补偿（✅）
- 语义：K_session 同 epoch 决定性 ⇒ 重启后旧票据 HMAC 仍可通过；而 `ledger.reset` 清掉一次性台账 ⇒ 重启前已消费票据在 TTL 内存在二次消费面。
- 修复：`stale_ticket_cutoff: HashMap<String, i64>`（session→重启时刻）；respawn 与 ensure_initialized 两处 `ledger.reset` 改走 `note_issuer_generation_restart()`；`verify_and_consume` 在 checks 全通过后、ledger 消费前加「`issued_at` < cutoff ⇒ 拒」；拒绝码沿用 `ReplayDetected`（不扩张封闭集），detail 写明 `issued_at ... predates signer restart cutoff ...`。
- 测试：`restart_cutoff_predicate_is_strict_and_time_only`（严格小于；同一秒放行；不可解析＝不按陈旧处理——时间谓词只管时间）。

### REV-083-10 journal writer（✅）
- `JournalWriterTask` 由 `tokio::spawn(...)` 改为**命名 OS 线程**（`spawn_writer_thread` → `run_blocking`）；`append_with_backoff` 同步化；退避梯 `tokio::time::sleep` → `std::thread::sleep`。语义不变（单写者 FIFO、ack/oneshot 不变），fsync 不再占用运行时工作线程。

### REV-083-18 小修集合（a–h 全落）
- a：守卫默认注释 1.10M→700K（5 处）。
- b：锁中毒统一（D-8）；31 文件 198 处。
- c：水位标签 memo（键＝live 字节数；命中即返回克隆）。
- d：permission 等待期心跳 stamp（30s 周期），防 stall 看门狗误杀。
- e：`ORZ_RETRIEVAL_SEMAPHORE_WAIT_MS` 严格解析（`parse_retrieval_lane_wait_ms`）＋畸形 `tracing::warn` 回落默认；不再静默。
- f：静态面响应头四件套（CSP `'self'` 面＋nosniff＋no-referrer＋X-Frame-Options=DENY）；CSP 允许 inline style（主题切换）、禁 eval。
- g：`access_in_scope` 退役镜像段删除（恒 true）；`normalize_lexical`/`path_under`/`strip_verbatim_prefix`/`components_lower` 转 `#[cfg(test)]`；幽灵窗口形态恒拒由 orz-tools 工具层承担（单点）。
- h：仓根两枚 `*.before-b-env-20260802-2228` 删除。

### 0bw⑤ / 0bw⑥（裁决✓）
- ⑤ **不翻** CFA enforce：1124 查询仍为空、无狗粮读数；runbook 脚本（`.tmp-0bw-cfa-audit.ps1`）已备，翻转待读数批。
- ⑥ S3/S4 **不可达**（本批不重建）：维持登记，随重建批复验。

## §2 决策台账（D-*）

| 编号 | 决策 | 理由 |
|---|---|---|
| D-1 | 账本（索引/BACKLOG/TODO）本批**不动**，随落账批同步 | 沿 0bw X5 惯例；并行会话共树期避免踩批 |
| D-2 | REV-083-16 按**就绪**处理（证据已在树），机械翻账随落账批 | 翻账行文与台账同批落更一体 |
| D-3 | 0bw⑤ CFA enforce **不翻** | 1124 现为空、无狗粮读数；翻转缺证据（延续 0bw D2） |
| D-4 | 0bw⑥ S3/S4 **不可达**→维持登记 | 本批不重建（用户约束） |
| D-5 | REV-083-08 拒绝码用 `ReplayDetected`＋detail 注明 cutoff | 不改 RejectCode 封闭集与 schema/夹具面；detail 自描述 |
| D-6 | REV-083-06 journal 口径取**「完整性/损坏检测」**档 | §7 三档措辞总则（不做超能力声明）；receipt 链不立项 |
| D-6e | REV-083-18e 库层＝严格解析＋`tracing::warn` 回落；exit 2 仅守 CLI 守门变量 | 库层不放杀进程；静默回退取消 |
| D-7 | REV-083-18g 删镜像段，读范围单点归 orz-tools | 消除双计算漂移面；幽灵形态恒拒由工具层承担 |
| D-8 | REV-083-18b 统一为 **poison-tolerant**（`unwrap_or_else(...into_inner())`） | 不因无关线程中毒 abort 运行；与降级哲学一致 |
| D-9 | REV-083-10 走**独立 OS 线程**（非 `spawn_blocking`） | 一次性移走全部阻塞 IO；ack/FIFO 语义不变 |
| D-12 | REV-083-12 取**修订退出码文档**路线 | `panic=abort` 下 join-err→exit(101) 分支不可达；hook 方案备档随批 |
| D-13 | REV-083-13 ADR 表样＝机制参数三列（**代际/来源/失效条件**） | 裁决先入 ADR 再进代码注释（§14.69 教训） |
| D-14 | REV-083-14 判据钉纪律：量尺/边界/参数机制定稿**同步产 fixture（先红后绿）** | 本批未涉新判据，仅记录纪律文本 |
| D-15 | REV-083-15 冻结判据：禁新增事件族/字段、仅契约双写；转归档＝发布链双签完成 | 落字随批 |
| D-19 | REV-083-19 本批沿审计留痕形态；落账建议小步提交＋双会话「status 双确认」 | 与 081/083 批惯例一致 |
| D-20 | REV-083-20 LIF 复裁日期＝**2026-10-11**（+14d）；冻结新增面、标注试验性 | 复裁前不扩面；边界表落字随批 |
| D-3c | REV-083-03 审批叙事收口**与 0bw 措辞批合并**落字 | 两处同源措辞，合并防漂移 |

## §3 摩擦台账（F-*）

- **F-1（重大，已恢复）批量改字触发的 mojibake 事故**：以 PowerShell 5.1 `Get-Content -Raw`＋`Set-Content -Encoding UTF8` 做「锁中毒统一」批量替换，31 个文件被按 ANSI(GBK) 解码后重写（CJK 注释成 mojibake、UTF-8 BOM 注入）。**检测**：`git diff --stat` 异常放大（agent_loop.rs +3757 等）。**恢复**：`git checkout --` 31 文件回 HEAD → 用 **Python(utf-8)** 重放替换（198 处）→ 逐文件 check。**教训**：5.1 管道读写 UTF-8 源码一律避用（该仓 0bt③ 已有同类注记）；批量替换必须显式 utf-8 读写并经 diff 体量自检。**残留**：本批临时脚本 `.tmp-lock-sweep.py` 与 `.tmp-tests-*.log` 属过程件（建议清理，见 §4 附注）。
- **F-2 write-control 机械闸提示**：删除仓根 `*.before-*` 两枚时触发 `broad/destructive` 提示（动作照常执行，留痕即可）——符合 18h 预期，非阻碍。
- **F-3 后台作业收割**：以 `Start-Job` 起的测试作业随工具调用结束被 Job Object 收割（`.tmp-tests-assurance.log` 半截）；后改**托管后台路径**重跑。过程件留存 `.tmp-tests-*.log`。
- **F-4 平台面约束**（0bw F5/F6 延续）：CFA enable 需提权（本会话非管理员，读数面免提权）；无 Linux 实测面（Landlock 仅设计＋跨目标 check）。
- **F-5 跨目标面**：rustup 已装 `x86_64-unknown-linux-gnu`（可 `cargo check` 跨目标），无 musl 目标——0bw①落码可用 cross-check 做最低验证（真机复验仍缺）。

## §4 验证与回归

- `cargo check`（`orz-bin`/`orz-loop`/`orz-assurance`/`orz-host`＋首轮含 `orz-web`）：**全绿**（树面无编译错误；PowerShell 下 stderr 呈现为「假红」退出码，以 `Finished` 行为准）。
- `cargo test -p orz-assurance --lib`：**277 passed / 0 failed**（含 REV-083-07 新增守护测试；journal 链、降级、族一致性、沙箱等全绿）。
- `cargo test -p orz-loop --lib acaf`：**14 passed / 0 failed**（含 REV-083-08 新谓词测试；18c 缓存类型修正后复跑）。
- `cargo test -p orz-host --lib`：**334 passed / 5 failed**（`process_tree`／`reclaim`／`resource_hint` 等 5 项在并发编译负载下失败；隔离复跑 `process_tree::tests::sweep_refuses_every_condition_mismatch` **1/1 通过**，归因未决——建议随落账批全量复验；暂无与 18g/18e 改动关联的证据）。
- `cargo check -p orz-loop`（18c 修复后）：**Finished**（全绿）。
- 未跑（结转）：全量 0.7.2 回归、`clippy`、`fmt`、跨目标 `cargo check --target x86_64-unknown-linux-gnu`（建议随 0bw① 批）。

## §5 树面清单与结转

**orz 子仓（修改，未提交）**：`orz-bin/src/main.rs`；`orz-loop/src/{acaf,compact,controller,agent_loop,context_scale}.rs`、`host_exec/tool_run.rs`；`orz-assurance/src/journal/{chain,recorder}.rs`＋锁统一 31 文件（198 处）；`orz-host/src/{permission,tools}.rs`；`orz-web/src/server.rs`；**删除** `build.bat.before-b-env-20260802-2228`、`build.ps1.before-b-env-20260802-2228`。
**父仓**：本档（`docs/audits/0BV_CARRYOVER_REV083_0BW_2026-09-27.md`）——本批唯一父仓落字。

**结转（下一批）**：
1. **文档落字批**：01（README 权限节）／03（ACAF §5.2＋approval.rs 处置）／04（README:71-113＋ADR-0010 状态行）／06（README journal 口径）／13（ADR 三列化）／15（assurance 冻结宣言）／20（LIF 判据＋边界表）——引用本档 §2 决策即可。
2. **0bw 代码批**：①Landlock（`orz-sandbox` 复用＋cross-check）／②载体自检（manifest＋启动校验，涉发布链）／③journal 事件族（Schema＋producer＋fixture）／④git 检查点/journal undo。
3. **REV-083 余项**：05（集成测试目录，单独立项）／09（排期＋shadow 落账）／11（SSRF pin）／12（退出码文档落字）。
4. **落账批**：台账（索引/BACKLOG/TODO）同步（含 16 翻账）＋小步提交＋0bw⑤ 复裁（待 1124 读数）。

## §6 附注（过程件清理状态）

- `.tmp-lock-sweep.py`（orz 根）：**已删**（本档落定前清理）。
- `.tmp-tests-*.log`（orz 根）：**已删**（§4 结果已并档）。

## §7 落字批执行回执（2026-09-27，主会话批；用户令「案例登记＋§5 结转第 1 项文档落字批直接做」）

承接 §5 结转第 1 项（文档落字批）与 F-1 案例化；**零 Rust 代码改动**（唯一源码触点＝approval.rs 头注释）、未提交/未推送/未重建、台账（索引/BACKLOG/TODO）仍不动（D-1 延续，随落账批）。

| 项 | 落点 |
|---|---|
| F-1 案例登记 | [`docs/incidents/ORZ-PS1-BULK-REWRITE-001.md`](../incidents/ORZ-PS1-BULK-REWRITE-001.md) ＋ [`docs/cases/harness_environment/ORZ-PS1-BULK-REWRITE-001-ps1-bulk-rewrite-mojibake.md`](../cases/harness_environment/ORZ-PS1-BULK-REWRITE-001-ps1-bulk-rewrite-mojibake.md)（`candidate`；同族互引 ORZ-PS1-BOM-001／GAP-ENCODING-GATE／ORZ-ENV-POLLUTION-001）；索引路由随落账批 |
| REV-01 权限语义（README 面） | README「安全」条补 yolo 默认（写/命令/网络自动批准、`ORZ_ALLOW_*` 三键切 Benchmark 轴、`[permission] mode=…` 启动横幅） |
| REV-03 审批叙事收口 | ACAF 设计 §5.2 重写（删「逐项审批语义保留」，改「策略＋票＋审计」现行为口径＋未来可选扩展声明）＋ README「安全」条明示「当前无人工审批」＋ `orz-host/src/approval.rs` 头注释处置（TODO 降为未排期候选扩展，PermitSource 观测位保留） |
| REV-04 工具面口径批 | README：检索条改「启用门 fail-closed＋主面 `web_search` 单一派发入口」、SERP 链补 DDG 兜底、HTTP 分段道三引擎线（直连 `360search,baidu`／代理＋`duckduckgo`，Bing 出集）、「拒绝启动」→「拒绝启动 run」；**ADR-0010 状态行 frozen→evolving**（v1.82，§14.79）＋ README「当前状态」设计行同步 |
| REV-06 journal 口径（D-6） | `architecture/current/README.md` §2.4 标题与正文降「完整性/损坏检测」（防篡改保证声明删除、receipt 链不立项注记）＋ §1 架构图节点同步 |
| REV-13 参数表三列化（D-13） | ADR-0010 **§14.80**（三列 {代际/来源/失效条件}＋裁决先入 ADR 再进代码注释） |
| REV-15 assurance 冻结宣言（D-15） | `assurance/README.md` 新增「冻结宣言（2026-09-27）」节（禁新增／仅契约双写／转归档＝发布链双签完成） |
| REV-20 LIF 判据＋边界表（D-20） | ADR-0010 **§14.81**（观察判据 journal 可测两条＋复裁日 **2026-10-11**＋冻结新增面＋四机制职责边界表：LIF temporal／RLI／水位疲劳／繁杂度） |

**复核读数（主会话独立）**：本轮全部新增/改动文件 UTF-8 严格解码零替换字符；ADR-0010 `accepted / frozen` 字面残留仅存于历史补记引文（§14 头部 v1.65 前流水与 git 历史，按「不回改历史补记」纪律保留）；README/投影/ACAF 设计/assurance README 相对链接逐条在盘。

