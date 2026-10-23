# 344 批：全仓三面审查（设计／实现／符合性）与处置收口（2026-10-23）

> **用户令**：「请对当前一整个项目进行全面深入检查，包括设计合理性、实现合理性、设计与实现的符合性」；随后「请先落一份审计文档，随后对审查出的问题进行全面处理吧，处置方式就按照你的建议即可」。
> **数据纪律**：主会话通读 ADR-0010 入口页，五个并行审查子代理分区深查（①设计合理性 ②orz-loop 核心实现 ③平台与安全层 ④外围 crate ⑤符合性），主会话逐项核验关键证据（file:line 双侧对账）；本档为发现全量登记与处置证明。审查证据中与代码相关的关键项均已由主会话二次复核（projection 过滤器漏改、tool_names 双口径、restore 锁点、permission yolo 接线、exec_policy 表外恒 Allow、fixture 生成器漂移）。

## §0 结论

- **实现质量整体匹配设计野心**：orz-loop 产品码 unwrap 仅约 25 处、911 个测试约占一半代码量、审计链（343/342/341）逐条与代码和 git 历史对账属实；已登记设计条款的实现符合度 **8/11 精确命中**。
- **三处结构性风险**：①治理失真——0cz／0dm 两批产品级机制绕过 ADR 落码，「唯一自然语言设计权威」近两周事实上失真（P0）；②Windows 安全承诺名不副实——沙箱 enforce 仅 `cfg(unix)`、交互缺省全自动审批、写面仅剩宿主状态两窄目标门，三者叠加＝模型命令≈用户命令（P1）；③核心循环巨石化（P1 结构）。
- **本批处置**：ADR 收口（§14.89 追认 0cz/0dm＋失效标注清单；§14.90 安全加固三裁决）＋七项代码修复＋四同步面（schema/双判官/fixture）＋五个旧契约测试按新法改写＋仓库卫生（556 项无引用 tmp 清除、206 项审计证据保留）。全部受影响 crate 测试通过（§4）。

## §1 审查范围与方法

- 范围：父仓 D:\CLI（adr/ 11 份 ADR＋ADR-0010 十四卷、docs/ 设计稿、docs/audits/ 483 份审计、CLI_PROJECT_INDEX.md）；子仓 orz（Rust workspace 约 73 万行，其中 orz-loop 9.6 万行／67 文件）。
- 方法：设计面按 ADR 分卷逐卷对读＋治理模式评估；实现面按 crate 分区（机械统计＋核心路径精读＋抽查 bug 级证据）；符合性面从 ADR 提取 11 条机器可核查承诺逐条对码＋审计链抽查（343/342/341 声明 vs 代码 vs git log）。

## §2 发现清单

### 2.1 设计面（P0×1／P1×4／P2×4／P3×2）

- **P0 现行法多口径并存**：主车道工具面三口径（入口页「8 工具直接执行」／§14.72-11「冻结为 10 工具」／0dm 稿「生产装配 12 工具」）；vol-03 仍写「默认工具轮预算 120」无失效标注（§14.55-2 已取代为默认无限制）。
- **P1**：§14.79「正文只留现行条款」自立纪律未执行，退役裁决须通读 5184 行 vol-14 还原；LIF 复裁日（§14.81 设 2026-10-11）爽约无登记；vol-02 §2.4.8「主 UI 为 orz-tui」已被 §14.78/0dk 实质反向取代；入口页补记索引停于 v1.86（vol-14 实到 v1.92）。
- **P2**：vol-10 验收条件多为自查式陈述缺客观判据；仪式化机制真机即撤（反例门收窄／budget 注入撤除／run 尾归档退役／滑块迭代至 v8）属可预见过度工程；LIF 线性解码器自认「无消费者、效果无测」仍保留。
- **P3**：状态行已改 `accepted / evolving` 而正文通篇「冻结版本」词汇残留；0dk 十一行动词表与 BACKLOG 九动词口径摩擦。

### 2.2 实现面

- **P1 结构**：`run_agent_loop` 单函数约 3800 行（agent_loop.rs:2354–6158）；`AgentLoopController` 56 字段／约 214 方法／impl 散布 15 文件（controller.rs 8889 行）。
- **P1 并发**：session restore 面三处裸 `.lock().unwrap()`（controller.rs:3229/3290/3335），与 watchdog 路径的毒锁容忍式不一致——LifEngine 持锁 panic 后恢复路径二次 panic，恰是 343 事故面。
- **P2**：内容前缀当消息身份（compact.rs `upsert_whitelist_message` 以 `starts_with(WHITELIST_PREFIX)` 定位常驻消息）；`with_plan` 走 `.expect()` panic（controller.rs:1406，已有 `try_with_plan` 兜底未接 --plan 路径）；assurance `families.rs` 7700 行／`rli.rs` 5073 行／`orz-bin/main.rs` 4235 行巨石文件；fixture 生成器落后已提交 fixture（0dm 五字段，本次勘正）。
- 外围 crate 全部真实完成、无孤儿无脚手架、无恒真判官（`tool_names::` 单源被真实引用＋机械钉强制）。

### 2.3 安全面

- **P1 Windows 无 OS 级隔离**：orz-sandbox enforce 仅 `cfg(all(feature="enforce", unix))`（non-unix 显式 no-op）；Windows 侧仅 `KILL_ON_JOB_CLOSE` Job Object（job_object.rs:59-98）；子进程持完整用户令牌交 `pwsh -Command` 整串执行。
- **P1 权限门缺省全放行**：`permission.rs:202` `initial_yolo=true`（2026-09-26 用户令）；yolo 快路径对 Ask 类（含 bash/SandboxEscape）AllowOnce。
- **P1 写保护收窄过激**：文件写工具仅拒宿主状态两目标（write_control.rs:434 v3）；`search_replace` 接受任意绝对路径无 workspace 包含性判定；exec_policy「表外恒 Allow」——`powershell -enc <base64>` 词法零命中零留痕。
- **P2**：nono apply 失败 fail-open 残留（orz-sandbox lib.rs:192-235）；桥面 `access_in_scope` 恒 true（读越界判定单点外移 orz-tools workspace 门）。
- **可信面**：凭据链（DPAPI keystore＋全链 redact）、读工具路径门（symlink 逃逸恒拒）、`READ_ONLY_EXEMPT_TOOLS` 单源＋宿主遍历式护栏。

### 2.4 符合性面

- 11 条承诺对码：**8 条精确符合**（RLI 缺省常开闭集、181s 看门狗、k=3、繁杂度公式与档梯、plan_epoch 单调占号、budget 零写入钉、.gsa 读全开放、tool_names 单源＋扫描钉）。
- **偏离未登记 ×2（P1）**：0cz `context_manage` 第十一工具改 10 工具冻结面（ADR 全 15 卷零命中）；0dm 终态声明/自查 PULL 改道反例门（agent_loop.rs:4210 自述）。
- **实现缺陷 ×1（P2）**：`retrieval/projection.rs:312` 子代理投影过滤器只剔 `context_compress` 未随 0cz 承继剔 `context_manage`——main-lane only 语义漏网（本批修复＋钉）。
- 审计链抽查：343 三项修复落码属实（后于本日提交 `bd42e011` 并 bump 0.9.11）；342/341 发布与载体检证属实，无夸大。

## §3 处置

### 3.1 ADR 收口（父仓）

| 触点 | 内容 |
| --- | --- |
| `adr/ADR-0010-vol-14-addenda-index.md` | 新增 **§14.89（v1.93）**：0cz/0dm 治理追认（context_manage 第十一工具方案 A／终态声明与反例自查 PULL 化／投影过滤器随批勘正）＋**失效标注清单**（vol-03 预算 120 两处→§14.55-2；vol-02 §2.4.8 主 UI→§14.78/0dk；入口页 8 工具行就地标注）。新增 **§14.90（v1.94）**：安全加固三裁决（写面 workspace 包含性／exec_policy encoded-exec warn／Windows 交互缺省 yolo 收紧）＋附随勘误＋残余登记 |
| `adr/ADR-0010-fusion-runtime-and-agent-architecture.md` | 入口页补记 bullets 补齐 v1.87–v1.92（滞后一个月）＋新增 v1.93/v1.94；「生产真实面裁决校准」行 8 工具口径就地标注现行指针 |

### 3.2 代码触点（orz 子仓）

| # | 触点 | 内容 |
| --- | --- | --- |
| 1 | `orz-loop/src/retrieval/projection.rs` | 子代理投影过滤器补剔 `CONTEXT_MANAGE_TOOL_NAME`（§14.89 条 3 勘正）＋测试父集补名钉 |
| 2 | `orz-loop/src/controller.rs` | restore 面三处锁（`restore_temporal_spikes`／`rli_complexity_reading`／`restore_lif_session`）统一毒锁容忍 `unwrap_or_else(PoisonError::into_inner)` |
| 3 | `orz-tools/src/types/exec_policy.rs` | 新增 warn 规则 **`encoded-exec`**（WARN_RULES 封闭集 2→3）：⑧臂 (a) PowerShell 编码旗 `-enc/-ec/-encoded/-EncodedCommand`；(b) 管道直落 shell 解释器 `| sh/bash/dash/zsh/ksh/powershell/pwsh`（原串词法扫描，`|` 属 Sep 词元）。warn 留痕不阻断；block 化留待真机读数。三枚新单测（编码旗／管道臂／普通命令零误报） |
| 4 | `orz-tools/.../search_replace/mod.rs` | 宿主状态门后新增 **workspace 包含性判定**：canonical 前缀比对（沿用 `path_hits_root` 大小写折叠／verbatim 前缀），越界 `InvalidInput` 机械拒绝＋双臂回归测试（工作区内照常／外部绝对路径拒绝且不落盘） |
| 5 | `orz-host/src/permission.rs` | `initial_yolo` 改由 `default_initial_yolo(gateway_live, hub_live)` 单一源：**Windows 有活审批面（gateway/hub）缺省 confirm-first**；headless（皆 None）与非 Windows 维持 yolo；运行时 `set_yolo_mode` 与 Benchmark 面不变＋姿态单源钉测试 |
| 6 | `orz-host/src/acp_server.rs`／`codex_app.rs` | 五个旧「默认 yolo」契约测试按 §14.90 新法改写（见 3.3） |
| 7 | `orz/build/xai-proto-build/src/find_protoc.rs` | 第②臂补 `bin/protoc.exe`（Windows vendored 副本此前永不命中，裸 `cargo check` 必败）；NotFound 报错改可操作指引（vendor 路径＋build.ps1/build.bat 指针） |
| 8 | `orz-assurance/src/tool_names.rs` | 豁免清单「当前为空」过期注释勘误（其下 0cz S2／0da S2 两批已登记） |

### 3.3 四同步面（encoded-exec 契约扩展）

- Rust 生产者：`exec_policy::WARN_RULES`（恰 3 条钉更新）。
- Rust 判官：`orz-assurance/journal/families.rs` `WRITE_CONTROL_RULES`／`WRITE_CONTROL_RULE_CATEGORIES`（`("encoded-exec","warn")`）。
- Python 判官：`assurance/run_event_journal_validation.py` `_WRITE_CONTROL_RULES`／`_WRITE_CONTROL_RULE_CATEGORIES` 同形。
- Schema＋fixture：`runtime/write-control-review-event-payload-v0.3.schema.json` 枚举 7→8 现行值（v0.2 冻结代际不动）；生成器新增 `write-control-review.encoded-exec-warn.valid` 正例，重跑 **零差异**（顺带勘正生成器 v0.2 `run_finished` 落后 0dm 五字段的漂移——以 `V02_ENVELOPE_PAYLOAD_OVERRIDES` override 恢复，v0.1 冻结树不受影响）。

### 3.4 旧契约测试改写登记（语义翻转，非削弱）

| 原测试 | 新法处置 |
| --- | --- |
| `session_prompt_default_yolo_auto_approves_with_dead_gateway` | 改名 `session_prompt_mutation_posture_with_dead_gateway_follows_1490`：Windows 断言 deny＋无 ToolStarted（dead gateway＝装配审批面的交互车道，无应答 fail-closed）；非 Windows 断言维持 allow_once 契约 |
| `session_prompt_allowed_mutation_records_snapshot` | 放行来源由「初始 yolo」改 **Benchmark 策略**（LocalMutation 编辑类确定性 Policy 放行、双平台一致）；快照时序断言原样保留 |
| `default_yolo_executes_tool_and_journals_without_approval` | 改名 `workspace_write_order_executes_and_journals_following_1490`：新增共享应答助手 `answer_approvals_until_turn_completed`（代表真 codex 客户端答 allow_once）；Windows 断言审批 1 次、非 Windows 0 次；「工具真实执行＋落 journal」两臂皆断言 |
| `default_yolo_repeats_identical_orders_without_prompt` | 改名 `workspace_write_identical_orders_follow_1490_posture`：Windows 断言每单各审批一次（`remember_tool_approvals=false` 无持久授予可乘）；非 Windows 断言 0；双执行断言保留 |
| `read_only_and_workspace_write_threads_coexist` | 审批断言改平台分叉（Windows 1／非 Windows 0）；read-only 拒绝臂原样保留 |

### 3.5 仓库卫生（父仓）

- 根目录 `.tmp*`／`tmp*` 共 762 项：以全库文本引用扫描（docs/audits 等 94 份审计引用 `.tmp-` 路径作载体构建证据）分类——**删除未被引用的 557 项**（28MB 中大部分），**保留 206 项证据文件**；被审计引用的证据面零触碰。

## §4 验证

| 门 | 结果 |
| --- | --- |
| `cargo check -p orz-tools-api`（无 PROTOC 环境变量） | ✅ 通过（find_protoc 自发现 `bin/protoc.exe`，29s） |
| `cargo test -p orz-tools --lib` | ✅ 3045 passed / 0 failed |
| `cargo test -p orz-assurance --lib` | ✅ 306 passed / 0 failed |
| `cargo test -p orz-host --lib` | ✅ 431 passed / 0 failed / 6 ignored（串行门 39.7s；并联模式下 `tests::` 模块存在与本批无关的子进程时序雪崩，见 §5 残余 7） |
| `cargo test -p orz-loop --lib` | ✅ 908 passed / 0 failed / 3 ignored（11.8s） |
| 五个改写语义测试（串行） | ✅ 5 passed（4.55s，无 30s 停顿） |
| fixture 生成器重跑 | ✅ 零差异（仅新增 encoded-exec 正例） |
| `git diff --check` | ✅ 无空白错误 |

> **补记（2026-10-23 收口时）**：orz-loop 全量 908 passed / 0 failed / 3 ignored；orz-host 串行门 431 passed / 0 failed。首轮并联 9 失败＝3 个旧 yolo 契约测试（已按 §14.90 改写，改写后并联亦绿）＋ `tests::` 模块 5–6 个进程树/截断/超时测试的并联时序雪崩（30s 停顿放大暴露；串行全绿、与本批代码改动无关，见 §5 残余 7）。

## §5 残余与后续批

1. **LIF 复裁**：§14.81 设 2026-10-11 窗口已爽约，顺延待用户裁决（退役／降影子两路径保留原状）。
2. **Windows 真隔离**：AppContainer/受限令牌级沙箱（§14.90 条 5 登记为 backlog；本批仅完成缺省姿态收紧＋写面包含性＋命令面留痕三层）。
3. **巨石拆分**：`AgentLoopController` trait 化拆相、`run_agent_loop` 分相抽取、`families.rs`/`rli.rs`/`main.rs` 分文件——0do S2+ backlog（本批不动，避免发版前重入 9.6 万行主循环）。
4. **fail-open 残留**：nono apply 失败继续、Landlock 子写守卫三分支 fail-open（Unix 侧，非本批范围）。
5. **Linux 评估容器 0600 明文 key**（keystore.rs:58,143-155）：登记待裁。
6. **发版**：本批修复随下一发行批进体（0.9.11 已含 343；本批预期 0.9.12 载体重建时入包）。
7. **orz-host `tests::` 模块并联脆弱性（本批新登记，非本批引入）**：进程树击杀／超时覆盖／输出截断／会话卷 symlink 五测试在 `--test-threads>1` 全量并联下偶发「子进程即时退出代替挂起」（串行恒绿、39.7s）；首轮被三个 30s 提示停顿测试放大必现，语义测试改写后呈概率性。疑似 Windows 子进程树/管道捕获在并联 spawn 压力下的既有时序缺陷，登记待专项批。
