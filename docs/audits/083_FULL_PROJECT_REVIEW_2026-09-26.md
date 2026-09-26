# 全面项目审查：设计·实现·符合性（2026-09-26）

> **本档＝用户令「请对当前项目进行全面详细审查，包括设计合理性，实现合理性，设计与实现的符合性
> （如有需要请派出子代理）」**。审查由 ZCode 主代理统筹，**5 路并行子代理**分头深审：
> ①设计（ADR-0010/ADR-0011/架构投影/索引）②核心运行时（orz-loop／orz-assurance／common／codegen 定性）
> ③执行面（orz-host／orz-bin／orz-tui／orz-web）④22 项设计声明逐条符合性核证 ⑤外仓账目与流程。
> 审查基线＝081 批后树面（orz `c5245558`＝v0.7.0 已发行；父仓账目面读到过 082 批 `3b44016f` 前后两个快照，
> §6 已分别标注）。本档另录**同日用户对审查结果的逐条裁决（第二轮）与审计员评判**（§7），
> 以及供台账承接的**整改清单 REV-083-\***（§8）。

## 0. 结论速览

| 维度 | 结论 | 一句话依据 |
|---|---|---|
| 设计合理性 | **良（下限）** | 单点机制（ACAF／journal／滑块 v8／黑板折叠）方向正确、威胁模型诚实；设计治理失灵：`frozen` 名下补写 81 次、机制反复造废（v7 整体作废、plan-epoch 建废、LIF 悬置）、安全叙事超卖（审批腿为空壳、journal"防篡改"口径越界） |
| 实现合理性（核心运行时） | **良＋** | 机制实现正确、失败路径测试深度罕见、决策可追溯注释逐条带 ADR 编号；组织性债务集中在 `run_agent_loop` 约 3,400 行巨型函数与数处结构性风险（async 内阻塞 fsync、票据账本 respawn 重置、hash 投影无字段守护） |
| 实现合理性（执行面） | **良** | 读沙箱／SSRF 门禁／凭据纪律／orz-web 防线一流；写入面无沙箱＋默认 yolo（0bs a 轮用户令）使"默认拒绝"叙事与实际行为脱节 |
| 设计↔实现符合性 | **高符合** | 22 条声明逐常量/逐分支核证：**19 MATCH／3 PARTIAL／0 MISMATCH／0 NOT_FOUND**——无虚报、无缺失，偏差均为文档滞后 |
| 账目与流程 | **账实相符、纪律真实生效** | 计数（57 项）三方一致、稳定 ID 抽查 4/5 全对、发行口径全对得上；最大结构性风险是批间"树面裸奔"窗口（审查期间实际目击一次） |

**规模基线**：自研核心 Rust **179,235 行**（orz-loop 88,776／orz-assurance 33,341／orz-host 33,618／orz-bin 7,905／orz-tui 12,581／orz-web 3,014）；vendored 移植 **525,368 行**（codegen 478,223＝Grok CLI 整体移植＋common 47,145＝xAI 栈组件）；全仓 .rs 约 70.8 万行、658 个文件含 `#[cfg(test)]`。

## 1. 方法与基线

- 五路子代理均为只读评审，结论全部要求 file:line 证据；行为类声明核对常量与默认值，不凭注释下结论。
- 基线快照：审查开始于 081 批落账前后（外仓树面曾见 17 条未落账），081 批（`044250df`＋`338dd0f1`，发行 v0.7.0）与 082 批（`3b44016f`，0bv 两项裁决）在审查期间先后落账；**本档事实面以终态树复核为准**（README.md:71/72/97/113、serp.rs:67-68 等关键证据均于终态重验）。
- 符合性核证的声明源＝README.md（081 批后版）＋ADR-0010＋architecture/current/README.md；实现源＝orz 子模块 `c5245558` 工作树。

## 2. 设计合理性（详目）

### 2.1 问题清单

- **[P1] "冻结 10 工具面"叙事与实际多层动态投影不符，冻结口径一日两改**
  ADR-0010 §14.72（v1.74，2026-09-18）"主代理常驻工具面自此冻结为 10 工具"，同日 v1.73→v1.74 九/十工具两改；architecture/current/README.md §2.1 同步宣称"十工具地位平等"，而检索关时 web 族整族剔除＝8 工具、检索开时 web_fetch 退出主面＝9 工具（`orz-loop/src/relay.rs:58-60`、`retrieval/projection.rs:71-91`）。"冻结"用在高频变化物上失去信息量。**建议**：改口径为"常驻主面＝N（数据）＋机制性投影＝规则"，不再以"冻结"命名工具面（见 §7 裁决①）。
- **[P1] Normal 模式人工审批腿：未设计、未实现，而叙事声称"用户逐项审批语义保留"**
  `orz-host/src/approval.rs` 全文件仅注释＋`// TODO: Implement approval prompter (Phase 1)`；索引 GAP-APPROVAL-PROMPTER 登记 partial 后延期；ACAF 设计 §5.2 的 `user_confirmation_sha256` 无真实来源。**建议**：给审批器最小设计定稿，或删除审批叙事（§7 裁决②取后者）。
- **[P1] journal"防篡改、防伪造"对外口径超出机制能力**
  architecture/current/README.md §2.4 宣称"防篡改、防伪造"；`orz-assurance/src/journal/chain.rs` 为无密钥 SHA-256（ACAF 设计 §1.3 自认"只能发现损坏"）；票级 receipt 链停在 v2 计划（ADR-0011 状态行）。**建议**：README 降口径为"完整性/损坏检测"，或立项 receipt 链（§7 裁决③，二选一）。
- **[P1] 设计记录→实现无机械对账点（v7 事故）**
  §14.69：v7 把滑块量尺记错对象（对本地全量 501,845 估算 vs 模型实际 73,006 真值），实现忠实按错误记录落码、固化，穿过"设计稿→ADR→五连批→测试全绿"全程才勘误。**建议**：量尺/边界/参数类机制定稿时同步产出可执行判据钉（fixture 先红后绿）；勘误强制回溯作废读数（§7 裁决④）。
- **[P1] LIF 过度设计且负结果后无限期悬置**
  一套时间/进度观测职能现存四套机制（LIF temporal／RLI／疲劳度／繁杂度）；LIF 134-run 回放 AUC 0.532＜0.70 后改角色"领航者"未实现，`orz-assurance/src/lif/` 6,184 行无消费者悬置。**建议**：给观察定判据与复裁日期、冻结新增面、标注试验性（§7 裁决⑤）。
- **[P2] 休眠/退役机制滞留生产代码与 ADR 正文**：console 三件与 plan-epoch 工程裁决仍在正文与代码，现行法与废止法不可分辨。
- **[P2] "两大块＋两小块"与代码分层在模块级不对应**：controller.rs 7,585 行／agent_loop.rs 9,218 行单体；"静默机械审查层"的注入曾被真机证伪进入用户可见面（0bi ⑨，§14.76 未闭合）。
- **[P2] 签发器 v1 实际独立性有限**：orz-signer 头注自认信任锚＝启动链（与宿主同信任级）；K_session 仍下发宿主验票；IPC 为 stdio。对外叙事应按 v1 实锚描述。
- **[P2] 反失控默认整体拆除**：去硬超时＋`MAX_TOOL_ROUNDS=0`＋F6 预算可见性默认 off；残余兜底仅 stall 360s／idle-kill／10h 背景／700K 守卫。属有意取舍，但至少 F6 `pull` 档建议默认开。
- **[P2] 参数面魔法数锚在第三方聚合站快照**：x=160K/y=32K/软梯/H1 320K/T1 500K/0.77 换算/700K 守卫，自注"不用 DeepSeek 口径"；`context_scale.rs` 注释引用 ADR 中不存在的"0bh④"裁决——裁决入代码不入权威，破坏自家 §7.1 权威层级。
- **[P2] journal 缺跨 run 全局账**：每 run 独立链＋归档单 gzip，归档件被替换无对账基准。
- **[P2] 账目体量成为最大维护面**：TODO 358KB／BACKLOG 两卷约 549KB／audits 248 件；小修复也走全量 S1–S4 仪式；项目已需发明"结论代际""跨 run 归属"等机制给账目防腐——体量失控的自证。
- **[P2] 缺失 ADR 级决策**：多平台形态矩阵、多会话并发共享面、升级迁移、0bi ⑨ 机械审查呈现边界契约。

### 2.2 值得肯定

ACAF 威胁模型诚实（显式"不防御"清单、检索内容默认有毒公理、影子先行、D-14/D-15 硬拒绝）；先 Schema/fixture 后 producer＋Python 镜像对拍（实际抓到过 GAP-EVENT-SCHEMA-DRIFT）；负结果与成本诚实入档（v8 更贵、RLI 消费率低、LIF AUC 0.532）；摩擦/案例/事故账目纪律在同类项目罕见。

## 3. 实现合理性——核心运行时（orz-loop／orz-assurance）

**vendored 定性**：`crates/codegen`（478,223 行）＝Grok CLI 整体移植（orz-workspace lib.rs 顶整排 allow，典型整仓搬运），为 orz-host 提供工具与工作区基座；`crates/common`（47,145 行）＝xAI 栈组件移植（xai-tool-runtime 等，`#![forbid(unsafe_code)]`，质量良好）；`orz-compaction` 等未接线 crate 不在主环路。**风险**：安全相关逻辑（终端进程树 kill 等）藏在实际无人逐行审查的 vendored 区。

### 3.1 问题清单

- **[P1] `run_agent_loop` 单函数约 3,400 行**（`agent_loop.rs:2065-5478`），30+ 正交状态变量耦合，已是回归热点；orz-loop 无独立集成测试目录。**建议**：按阶段拆出（先提上下文管理段为独立 async fn）。
- **[P2] journal 写者在 tokio 任务内每事件阻塞 `sync_all`**（`recorder.rs:578-580`）：慢盘占死 runtime worker；应移阻塞线程（语义不变）。
- **[P2] ACAF 票据账本 signer 崩溃重启后整体重置**（`acaf.rs:289-291`）：过期窗口内已消费票据存在理论二次消费面，无"拒绝 `issued_at`＜respawn 时刻"补偿。
- **[P2] `compute_event_hash` 手工枚举 12 字段无守护**（`chain.rs:69-82`）：未来 schema 加字段忘更新投影→新字段不进完整性哈希，恰是 hash-chain 承诺失效点。**建议**：字段集守护测试或"序列化后剔除 event_sha256"式实现。
- **[P2] `ORZ_ACAF_FAIL_CLOSED=0` 合法值静默进 shadow 无告警**（`main.rs:510-527`、`controller.rs:269-297`），且 shadow 下 D-14/D-15 类 pre-signing 拒绝不落 journal（`acaf_flow.rs:242-244`）——环境变量注入即整体降级且零审计痕迹。
- **[P3] 集合**：守卫默认值注释 1.10M vs 实际 700K（`compact.rs:131` vs `:287`）；锁中毒策略两写法并存；黑板水位标签每次全量序列化（`controller.rs:2873-2881`）；stall 看门狗仅 `-p` 路径装载且 permission 等待可被误杀（`main.rs:1046-1052`）；orz 仓根 `build.bat.before-*` 等入库。

### 3.2 核实为真的关键机制（抽样证据）

submit 两阶段（`tool_run.rs:3188-3287`，`bump_plan` 不动 epoch 使 pending 天然失效）；复读检测多级确认（滚动哈希任意相位＋字符级比对＋标点块≥0.50＋3-gram＞0.70＋流内 15 次＋`DEGENERATION_LIMIT=3`，含 DNA/EGFP 真实低熵样本测试）；空响应重试链（≤2 快重试→降档→显式失败）；压缩三承诺逐字断言测试；journal canonical JSON 与 Python 逐字节对齐＋torn-tail 64KiB 反向扫描＋ENOSPC 阶梯降级；ACAF 真进程边界＋签名覆盖 canonical(ticket_kind＋全量 args)＋resolved target digest＋消费点重解析实对象。

### 3.3 测试质量

明显高于平均且偏向失败路径：orz-loop 内嵌 854 测试、orz-assurance 300；orz-bin `tests/acaf_e2e.rs` 2,800 行驱动真实 signer 进程（含 crash→respawn／重放／fail-closed 分支）；journal-conformance 二进制级对拍。短板：orz-loop 无独立集成目录；vendored 区测试有效性未审。

## 4. 实现合理性——执行面（orz-host／orz-bin／orz-tui／orz-web）

### 4.1 问题清单

- **[P1] 默认 yolo 使 `--allow-*` 的 fail-closed 叙事与行为脱节**
  `orz-host/src/permission.rs:191` `initial_yolo=true`（注释"用户令 2026-09-26"；081 档 §1.1"0bs a 轮…会话初始 yolo；无客户端不再等审批 300 s"确认为最近有意变更）；Interactive 下 Bash/网络/写盘自动批准（有测试钉死）；而 `orz-bin/src/main.rs:315-318` 帮助文本仍写"bash/network fail closed unless opened"；"默认拒绝矩阵"所在的 Benchmark 策略**无任何 CLI 旗标可达**（仅 codex app-server 通道）。**建议**：README/帮助文本如实化＋`-p` 启动台面打印当前权限模式（§8 REV-083-01）。
- **[P1] search_replace 写入面无工作区沙箱，读写不对称**
  写前仅查 `.gsa` 写保护＋gitignore＋路径长度（`codegen/orz-tools/.../search_replace/mod.rs:232-247`），工具描述明示绝对路径合法；对照读面 canonical 级三类恒拒（`types/resources.rs:591-640`）。叠加默认 yolo＝模型可自动编辑机器上任意用户可写文件。**建议**：写面补与读面同形 canonical 沙箱（默认开、显式关），或过渡方案：cwd 外写入在 journal edit-action 面单列标记。
- **[P2] web_fetch SSRF 检查与实际连接间 DNS TOCTOU**（`web_fetch/client.rs:100-104` 两次独立解析；逐跳复核只封 redirect 链）。**建议**：解析结果 pin 到连接。
- **[P2] `.gsa` 两段门可经 run_terminal_cmd 直通 shell 旁路**：工具层门在默认策略下是"对模型的教育边界"而非强制边界；强制边界实为 OS＋权限轴——文档未明示剩余边界的真实位置。
- **[P2] panic=abort 使 `join→exit(101)` 契约不可达**（根 Cargo.toml `panic="abort"` vs `main.rs:83-93`），且无 panic hook／无 crash 落 journal。
- **[P3]**：orz-web 静态面缺 CSP/Referrer-Policy（server.rs:149-158；XSS 双层防线本身扎实）；权限桥 RETIRED 镜像段滞留（permission.rs:397-469）；env 解析风格不一（`ORZ_ACAF_FAIL_CLOSED` 畸形即 exit 2 vs `ORZ_RETRIEVAL_SEMAPHORE_WAIT_MS` 静默取默认）。

### 4.2 值得肯定

读面沙箱工程质量（Windows 大小写＋字节级比较防多字节 panic、符号链接外指恒拒、canonical 消失竞态分支）；SSRF 覆盖面罕见地全（CGNAT/NAT64/6to4/IPv4-mapped，曾漏 `::127.0.0.1` 已修；local 模式配置不经环境变量防模型翻面）；凭据纪律（CredReadW 原地清零、journal 落盘前脱敏、唯一出口 `redacted()` 有防泄漏测试钉）；权限为每调用检查且判定来源全程观测入 journal；orz-web 回环＋Host 防 rebinding＋256-bit token 常数时间比较＋run_id 白名单含 Windows 保留设备名。

## 5. 设计↔实现符合性核证（22 条）

| # | 声明摘要 | 判定 | 证据（file:line） | 备注 |
|---|---|---|---|---|
| 1 | 冻结 10 工具面 | **PARTIAL** | `orz-host/src/tools.rs:377-384`＋`controller.rs:3732/3900/3937/4105` | 注册层恰 10；但投影：检索关 8／检索开 9，web_fetch 恒不在主面。README:71/97 未同步（终态复核仍在） |
| 2 | submit 两阶段＋终答前机械审计＋反例自查 | MATCH | `tool_run.rs:3188-3300`；`agent_loop.rs:3631-3668` | |
| 3 | ACAF fail-closed | **PARTIAL** | `main.rs:510-527`；`controller.rs:3408-3417` | ①拒的是 run 启动非进程启动；②合法 `=0` 进 shadow 无告警 |
| 4 | `--allow-write`；shell/network 必须配对否则 exit 2 | MATCH | `main.rs:23-41,181-187` | 解析层严格；但默认 yolo 使 Interactive 路径实际不拒（§4.1 P1，语义层偏差） |
| 5 | `--max-wallclock`→`run_invalidated` 不硬杀 | MATCH | `main.rs:864-873,1119-1126` | |
| 6 | stall 360s／工具超时 300s | MATCH | `main.rs:1057`；`orz-host/src/lib.rs:129` | |
| 7 | 前台 180s 自动后台化＋idle-kill | MATCH | `bash/mod.rs:606`；`orz-host/src/tools.rs:117-143` | |
| 8 | journal hash-chain／schema v0.2／verifier／--replay 只读 | MATCH | `chain.rs:62-94`；`event.rs:281-295`；`orz-tui/runner.rs:146-153` | |
| 9 | 软提醒→320K 硬打断→500K 硬截断；不覆盖本地面 | MATCH | `context_scale.rs:127-142`（320_000/500_000 常量核对）；`compact.rs:30,767-818` | 实际软梯 4 档（192/256K），README 概括为"软提醒" |
| 10 | blackboard_write ≤8K；(domain,round) 盖章；水位【x.xM/10M】 | MATCH | `blackboard.rs:642`（8192）；`controller.rs:3267-3279,6966` | |
| 11 | web_search 并发 1；检索默认关；`--retrieval-mode` 废弃兼容 | MATCH | `orz-host/src/lib.rs:336`；`main.rs:204-228` | |
| 12 | 浏览器车道 SERP／人化输入／`browser_launch_result` | **PARTIAL** | `serp.rs:67-68`＝[Google, Bing, **DuckDuckGo**]；`input_sim.rs` | README 漏第三顺位；另 0bs c 轮 HTTP 三引擎线（360search/baidu/duckduckgo 指纹）README 亦未登记 |
| 13 | Web 工作台回环＋令牌＋内嵌＋ACP-over-WS | MATCH | `security.rs:43-45`；`server.rs:549`；`assets.rs:22-27` | |
| 14 | 跨进程恢复／-p 落持久化／里程碑归档／session_archive 单 gzip | MATCH | `acp_server.rs:608,1930-1957,694-695,2887-2894` | |
| 15 | 系统提示近零；开局三问一次；每 50 轮一次；软门 | MATCH | `prompt.rs:26`（BASE_SYSTEM_PROMPT=""）；`orientation.rs:34,50-54` | |
| 16 | 复读检测＋空响应重试链 | MATCH | `transport.rs:82-83,294-321,365-377` | |
| 17 | 轮预算默认无限 `MAX_TOOL_ROUNDS=0` | MATCH | `controller.rs:67,895` | |
| 18 | 内容锚点核证；实体三域登记；错误信封 | MATCH | `tool_run.rs:320-336,7406-7456`；`entities.rs:169,215,252`；`tool_envelope.rs:115-125` | |
| 19 | 凭据注册脱敏；URL 门禁与来源加权；候选计数 | MATCH | `orz-host/src/lib.rs:286`；`credential/mod.rs:442,493`；`source_weighting.rs:30-57` | |
| 20 | IPG 实现存在 | MATCH | `gates/ipg.rs:1-52`；`agent_loop.rs:3762-3790` | 非死代码，Block 即终止相位 |
| 21 | LIF 四特征／spike／fires 不注入／四 selector | MATCH | `lif/temporal.rs:51-54,238-257`；`controller.rs:2349-2456` | 机制在、消费者缺（§2.1 P1） |
| 22 | --plan 8 态状态机；plan_first 休眠 | MATCH | `plan/state_machine.rs:17,57-91`；`controller.rs:978` | |

**偏差汇总**：3 条 PARTIAL 均为文档滞后或措辞粒度差异，无实现缺失、无反向虚报。最严重＝条目 1（工具面口径）。

## 6. 账目与流程

### 6.1 一致性核证结果（抽查全过）

| 项 | 结果 |
|---|---|
| 未闭合计数 | 三方一致（57 项；082 批后不变） |
| 发行口径 | README 只指 v0.7.0（GitHub 确认 2026-09-26T10:03Z）；0.6.14–0.6.17"不单独发行"口径未回潮 |
| 稳定 ID 抽查 ×5 | 4/5 完全一致；1 条状态滞后（GAP-SPAWN-ORPHAN-RECLAIM 标 pending，但 `orz-host/src/process_tree.rs` 已落码孤儿清扫——"已落码待验证"被记成"未动手"） |
| audits 078-081 vs git log | 逐环吻合 |
| 仓库卫生（git 侧） | 干净：481 个 `.tmp-*`（约 71.5MB）＋8 个 `tmp*/` 全部忽略，`git ls-files` 零二进制 |
| orz 子模块 | 已收口（081 后 pin=`c5245558`） |

### 6.2 问题清单

- **[P1] 批间未落账窗口是最大结构性风险（审查期间实际目击）**：审查开始时树面 17 条未落账（＋279/−69 行账本＋7 篇审计档未跟踪），数小时后由 081 批一次收口。账本权威只存在于工作树的窗口内，磁盘故障即丢整批；且本审查自己的基线读数也因窗口过时了一轮（双操作者共树，见 §7 裁决⑥）。**建议**：账本小步提交与批次解耦，或双会话并行时"落账前 git status 双确认"。
- **[P2] assurance/ Python 镜像（192 文件，约 9.7 万行）半冻结态**：与 runtime/ 重叠仅 4 schema，真实同步点在 Rust 判官＋CI 对拍；对新功能是纯税。**建议**：显式宣言"冻结、仅 P0–P5 契约变更双写、禁新增；对拍连续 N 月零差异后转只读归档"（§7 裁决⑦）。
- **[P2] 账面引用的磁盘证据无托管清单**：`.tmp-b075-dl-*` 两发行包（36.8MB＋27.7MB，075 档明令保留）＋`.tmp-b071-guide-eq.pdb`（疑关联 ORZ-DEV-LINKER-CRASH-001）为无 sha256 清单的裸文件，一次清理即断证据链。**建议**：处置顺序＝先在引用审计档加"证据处置注记"→再删件；**pdb 例外**：GitHub 上没有，删前须结案或移 `存档/evidence/`（§7 裁决⑧）。
- **[P3] 索引头部单条批叙述约 700＋字逼近可读边界**；保持每 10 版滚出存档的节奏。

**一句话**：账实相符、纪律真实生效，风险不在记错账而在账只在批末才记。

## 7. 用户裁决记录与审计评判（2026-09-26 第二轮）

> 用户对 §2–§6 逐条回复；本节录裁决原文要点＋审计员评判。**评判分歧处如实保留**。

### 裁决① 「取消冻结」（对 §2.1 P1 冻结名存实亡）

**裁决要点**：设计不再冻结；错的内容就是要改、不能留下。
**评判：接受方向，附三项承接条件。**
(a) 取消冻结解决的是"名实相符"，不解决"现行法不可读"——506KB 单文件正文＋81 条补写混编，无论冻结与否读者都无法直接回答"现在是什么"。真正的替代品是**版本化现行法**：正文只留现行条款、退役条款移附录/存档、每次变更＝编号修订件（v1.x 机制保留并升格为唯一纪律：每个修订件三行头——改了什么/为什么/取代哪条；不要回到无痕原地改写）。ADR-0010 头部与 README:113 的 `accepted / frozen` 状态行需同步改（如 `accepted / evolving v1.8x`）。
(b) 冻结有一个隐性功能要另行承接：它是下游契约（事件 schema v0.2、assurance 冻结镜像、工具面声明）的稳定锚。建议拆开声明——**设计层演进自由，契约层变更须走修订件＋下游同步**，否则"取消冻结"可能被读成"什么都可以随手改"。
(c) "错就是要改"与"留痕"不冲突：v1.81 机制的真正价值恰是每次变更留了取代关系与证据链——改错可以，抹改不行。

### 裁决② 「去掉 ACAF 中人工审批相关内容」（对 §2.1 P1 审批腿空壳）

**裁决要点**：现在确实没有人工审批，相关内容删掉。
**评判：接受，删除虚假声明是对的；同步做三件事。**
(a) ACAF 威胁模型要随之改写：审批腿曾是"策略（机械）＋人（裁量）"双层门的上层，去掉后 ACAF 防的是**未授权工具调用／票据重放／参数事后篡改**，不防"模型做出格式正确但错误的决定"——README/ACAF 设计的安全叙事应如实降级（"人审＋票"→"策略＋票＋审计"）。
(b) 与默认 yolo（0bs a 轮）叠加后的现状＝交互面**零人工检查点**，安全边界＝OS＋工具层读门＋journal 审计。这可以是合法的产品决定（真机直通哲学），但必须在 README/ADR 明示"当前无人工审批、默认自动放行"，不能靠读者自己发现 approval.rs 是空壳。
(c) 权限桥 `PermitSource` 枚举已为将来恢复审批留了观测位——删"承诺"时保留一句"审批面为未来可选扩展、当前未实现"，成本为零、叙事诚实。

### 裁决③ 「ACAF 提上日程」（对 §2.1 P1-3 票据门未做完）

**裁决要点**：ACAF 一直在待办，应提上日程。
**评判：接受；"提上日程"需换成一张可验收清单，否则就是又一个无限期 pending。** 从审查证据出发的 ACAF 收口清单：①票级 receipt 链或归档 digest 外部锚（"防篡改"口径的现实基础）；②验票移独立执行器（ACAF §10 开放问题→排期项）；③`compute_event_hash` 字段集守护测试；④signer respawn 后旧票据补偿（拒绝 `issued_at`＜respawn_at）；⑤shadow 下 pre-signing 拒绝落 journal；⑥`FAIL_CLOSED=0` 合法值启动告警。**注意方向要逐条定**：①是"实现向描述看齐"（贵：涉及密钥管理）还是"描述向实现看齐"（把 README 口径降到"损坏检测"）——二选一，不要两头都停在半路；审查发现的正是"叙事在前、机制在后"的半程态。②-⑥ 便宜，直接做。

### 裁决④ 「更详细的记录和留档」（对 §2.1 P1-4 v7 事故）

**裁决要点**：只能后面更详细地记录留档。
**评判：部分接受，须指出"更详细的记录"不是 v7 事故的解药。** v7 的教训恰恰是：记录很详细、且是错的，错误还能穿过五连批实现＋测试全绿。解药不是更多过程文字，而是**机械对账点**：
(a) 量尺/边界/参数类机制，设计定稿时**同步产出可执行判据钉**（fixture 钉死"量尺对什么对象、何时翻转"），先红后绿——项目在事件面已用"先 Schema 后 producer＋对拍"证明此法有效（抓到过 GAP-EVENT-SCHEMA-DRIFT），推广到机制参数面即可；
(b) 重大机制勘误强制回溯"哪些既有读数作废"（v8 已做，升格为纪律）；
(c) **裁决先入 ADR 再允许进代码注释**（反例：`context_scale.rs` 注释引用了权威里不存在的"0bh④"）。
"更详细记录"只在"记录裁决来源与失效条件（代际标签）"这个意义上接受。

### 裁决⑤ 「LIF 暂不删除、继续观察」（对 §2.1 P1-5）

**裁决要点**：有试验性，保留观察。
**评判：接受保留；但"观察"需要观察什么、到什么时候，否则就是审查已发现的"负结果入档后无限期悬置"的延续。** 建议三件最小事：(a) 给观察定**可测判据与复裁日期**（§14.75 已有两条退役路径，缺的是触发条件与日期；例如"RLI 稳定判据达成日，或某日期前 temporal 分区仍无消费者则按无消费者退役流程处理"）；(b) 冻结 LIF 新增面（6,184 行存量维持、不再扩面）；(c) 索引/README 标注"试验性、非承重"，避免后续实现批次把它当在役机制。另建议补一张四机制职责边界表（LIF temporal／RLI／疲劳度／繁杂度）——没有对照物，"观察"无从观察。

### 裁决⑥ 内在矛盾清单两点回复（黑板／DeepSeek）

**黑板**：用户指出黑板须承担更多责任以对抗上下文压缩的信息失真，同时认可瘦身。
**评判：理由成立，两点兼容。** 黑板 PULL 面＋水位读数确实是对压缩失真的正确对冲；审查指出的问题不是"黑板太有用"，而是**分区无准入**（processes live／env 快照／RLI history 逐批叠加、无写者/消费者声明）。瘦身＝治理而非削职能：分区登记表化（每分区强制声明唯一写者＋消费者）、无消费者分区退役；登记表落进 ADR（现在散在 §3.6＋§14 多处）。
**DeepSeek**：用户立场＝框架不特化某一模型，但须适配在用模型（现为 DeepSeek），做多模型兼容时再做各自参数体系。
**评判：立场成立，且与审查原意一致。** 审查不是反对适配 DeepSeek，而是指出参数锚在**第三方聚合站快照**而非在用模型本体、且无代际标签与失效条件（模型已从 v4 flash 换 4.1，历史成绩失效）。建议参数表加三列 {适用模型代际、来源、失效/复校条件}——这与"将来多模型各自参数体系"是同一条路的两个站点，不冲突。

### 裁决⑦ 票据门补强＋"其他的内容改修的就全修"（对 §3/§4）

**裁决要点**：票据门与安全部分补强至描述和设计的水平；其余 P2/P3 全修。
**评判：接受；§8 REV-083 清单即 checklist。** 两点提醒：(a) "补强至描述水平"与"描述向实现看齐"要逐条二选一（见裁决③），防止半程态复现；(b) 全修清单中 `run_agent_loop` 拆分（REV-083-11）是最大工作量项，建议单独立项不与其他混批。

### 裁决⑧ 模型权限"再想一想"（对 §4.1 P1 写面无沙箱）

**裁决要点**：orz 要直接进真机、不受沙箱不全面环境限制；没给写配沙箱是想让机械层拦截"不适合写入"，只是机械层安全拦截还没做。
**评判：哲学自洽，但四点须严格对待。**
(a) **"真机直通"接受为产品定位**——"部分沙箱给虚假安全感"是真论点，README 一贯声明。
(b) **但现状是三无叠加**：无写沙箱（绝对路径可写任意用户可写文件）＋机械层拦截未做＋默认 yolo 自动放行。三者叠加＝当前写入面**没有任何有效门**。在"以后做机械层拦截"落地前是一段真实裸奔期：自己狗粮可接受，但过渡期至少做两件便宜事——帮助文本/README 如实化（"bash/network fail closed"文案已与行为脱节）、**cwd 外写入在 journal 单列留痕**（不做边界声明、只保审计底线）。
(c) **概念澄清（关键）**："不给写配沙箱、要机械层拦截不适合写入"——机械层拦截"不适合写入"本身就是一张策略表，与"沙箱"的差别只在**拦截发生在哪一层**（机械层 vs 工具层），不在"有没有边界"。所以真正的决定不是"沙箱 vs 无沙箱"，而是"边界规则放哪层、规则面是什么"（.gsa 保护已在；工作区内默认允许；工作区外默认？；哪些路径永禁）。建议把这张规则表做成小设计档＋fixture 钉（与裁决④判据钉纪律同构），"还没做"就升级为"按档施工"。
(d) **读写不对称是现状对"无沙箱"立场的内部反例**：读面已经在工具层有一层高质量 canonical 沙箱（三类恒拒）。要么承认读面那层就是边界的一部分、给写面对称补齐；要么说明为何读写适用不同哲学。当前"读有门、写无门"难以用"真机直通"一句话自洽。

### 裁决⑨ 账目三条回复（对 §6）

**批间窗口**：用户说明"刚是我在同时使用 codex 工作"。
**评判：说明接受，但它恰是风险的佐证而非解除**——双操作者共树时，批末一次落账的窗口内第二操作者的改动会互相踩（本审查基线读数就因此过时一轮）。081/082 两批均已干净收口、无遗留损失，此条降级为流程建议（REV-083-19）。
**assurance Python 冻结**：接受。冻结宣言按可判据形式落账（§6.2 口径）。
**磁盘证据本地不存、GitHub 上有**：**大部分接受**（两个发行包 GitHub 可复得，删本地不断链），**两处例外先处置**：(a) `.tmp-b071-guide-eq.pdb` GitHub 上没有——删前结案 ORZ-DEV-LINKER-CRASH-001 或移 `存档/evidence/` 附 sha256；(b) 075 档"保留作证据"明令要先加处置注记再删件——**顺序：先改账、后删件**，否则账面指向悬空。

## 8. 整改清单（供台账承接；编号 REV-083-\*）

| 编号 | 级 | 事项 | 证据锚 |
|---|---|---|---|
| REV-083-01 | P1 | 权限语义收敛：README/帮助文本与 yolo 默认改齐；`-p` 启动台面打印当前权限模式 | permission.rs:191；main.rs:315-318 |
| REV-083-02 | P1 | search_replace 写面补 canonical 沙箱（与读面同形），或过渡：cwd 外写入 journal 单列留痕 | search_replace/mod.rs:232-247；resources.rs:591-640 |
| REV-083-03 | P1 | 审批叙事收口：ACAF 设计/README 删"逐项审批语义保留"；approval.rs 空壳处置（裁决②） | approval.rs；ACAF §5.2 |
| REV-083-04 | P1 | README 工具面口径批：L71/L97 改"启用门＋单入口"投影现口径；SERP 链补 DuckDuckGo 与 HTTP 三引擎线；"拒绝启动"→"拒绝启动 run"；ADR-0010 状态行 frozen→evolving（裁决①⑤联动） | README.md:71/72/97/113 |
| REV-083-05 | P1 | run_agent_loop 拆分＋orz-loop 集成测试目录（建议单独立项） | agent_loop.rs:2065-5478 |
| REV-083-06 | P2 | journal 口径二选一：README 降"完整性/损坏检测"，或立项 receipt 链（裁决③） | chain.rs；README 投影 §2.4 |
| REV-083-07 | P2 | compute_event_hash 字段集守护测试 | chain.rs:69-82 |
| REV-083-08 | P2 | signer respawn 旧票据补偿 | acaf.rs:289-291 |
| REV-083-09 | P2 | ACAF 收口批次：独立验票执行器排期＋shadow pre-signing 拒绝落账＋FAIL_CLOSED=0 启动告警 | acaf_flow.rs:242-244；main.rs:510-527 |
| REV-083-10 | P2 | journal writer 移阻塞线程 | recorder.rs:578-580 |
| REV-083-11 | P2 | web_fetch SSRF 解析结果 pin 到连接（封 DNS rebinding TOCTOU） | web_fetch/client.rs:100-104 |
| REV-083-12 | P2 | panic 契约：统一 panic hook（尽力落 run_invalidated{crash}）或修订退出码文档 | Cargo.toml；main.rs:83-93 |
| REV-083-13 | P2 | 机制参数表三列化（代际/来源/失效条件）入 ADR；裁决先入 ADR 再进代码注释 | context_scale.rs（"0bh④"案） |
| REV-083-14 | P2 | 判据钉纪律：量尺/边界/参数机制定稿同步产出 fixture（先红后绿） | §14.69 教训 |
| REV-083-15 | P2 | assurance 冻结宣言落账（冻结、仅契约双写、禁新增；转归档判据） | §6.2 |
| REV-083-16 | P2 | GAP-SPAWN-ORPHAN-RECLAIM 状态 pending→partial | process_tree.rs |
| REV-083-17 | P2 | 磁盘证据处置：先注记后删件；pdb 先结案或入 存档/evidence/ | §7 裁决⑨ |
| REV-083-18 | P3 | 小修集合：守卫默认值注释 1.10M→700K；锁中毒统一；水位标签缓存；stall 看门狗 permission 等待豁免；env 解析统一畸形即报错；orz-web CSP；RETIRED 镜像段删除；orz 仓根 before-* 清理 | §3.1/§4.1 各锚 |
| REV-083-19 | P3 | 流程：账本小步提交与批次解耦／双会话并行"落账前 status 双确认" | §6.2 |
| REV-083-20 | P3 | LIF 观察判据＋复裁日期落账；冻结新增面；标注试验性；四机制职责边界表 | §14.71/§14.75 |

## 9. 落账接口

本档为新增审计件，**不代改共享账本**（README／ADR-0010／索引／TODO／BACKLOG），原因：审查期间存在并行会话共树（081/082 批相继落账），避免踩批。REV-083 清单应随下一批入账：其中 REV-083-01/03/04/06 对应的裁决已在本档 §7 记录在案，落账时可直接引用本档节号。

**落账回执（2026-09-26，084 批）**：用户令「083审查中的待优化与处理项全部进0bv」——REV-083 清单（§8，01…20）整体并件 [`GAP-FRICTION-AND-CARRYOVER-BATCH-R5`](../../docs/BACKLOG_AND_PRIORITIES.md)（0bv）：**编号保留、执行随 0bv、闭合随 0bv、不新增计数**。三项例外＝REV-083-02 已按本档 §10.2 升级 0bw（写入管控立项）、REV-083-17 已于本档 §10.1 闭合（083 批）、REV-083-05 的 run_agent_loop 拆分执行面已随 0bs ⑭（0bm 未竟②、S1 已定稿；0bv 承接增量＝orz-loop 集成测试目录）；其余 18 项逐条清单见 BACKLOG `### 0bv.` 并件块。

## 10. 第三轮裁决定案（2026-09-26，用户对 §7 评判的回复）

**用户令「我同意你的评判和全部建议，请将这些记录下来吧」**——§7 各裁决的审计员评判与承接条件**全部采纳为定案**，承接摘要：

| 裁决 | 定案承接 |
|---|---|
| ① 取消冻结 | ADR-0010 转**版本化现行法**：正文只留现行条款、退役条款移附录/存档、每次变更＝编号修订件（三行头：改了什么/为什么/取代哪条）；契约层变更（schema/工具面/账目 ID）须修订件＋下游同步；状态行 `frozen` → `evolving`（随 REV-083-04 落） |
| ②③ 审批删除＋ACAF 提日程 | 删审批叙事＋威胁模型改写（"人审＋票"→"策略＋票＋审计"）；ACAF 按六项收口清单立项（REV-083-09），receipt 链口径二选一在立项时裁 |
| ④ 记录→判据钉 | 判据钉纪律替代"更多记录"（REV-083-14）：量尺/边界/参数机制定稿同步产 fixture（先红后绿）；裁决先入 ADR 再进代码注释 |
| ⑤ LIF 保留观察 | 定可测判据＋复裁日期＋冻结新增面＋标注试验性＋四机制职责边界表（REV-083-20） |
| ⑥ 黑板/DeepSeek | 分区登记表化（每分区声明唯一写者＋消费者，无消费者退役）；参数表三列化（REV-083-13） |
| ⑦ 全修 | REV-083 清单即 checklist；REV-083-05（run_agent_loop 拆分）单独立项 |
| ⑧ 写入管控 | **升级为机械层写入管控立项**（见下，新裁决 B） |
| ⑨ 账目 | 账本小步提交（REV-083-19）；assurance 冻结宣言（REV-083-15）；磁盘证据按下述执行 |

### 10.1 新裁决 A：磁盘证据处置（已执行）

- `存档/evidence/` 成立。`.tmp-b071-guide-eq.pdb`（1,241,088 B，sha256 `2b51508ce2e9b325c7c6c42fcfe3fd90754ccd4e21e1a6be6e9e4270cde5fb44`）经复制→双端 sha256 比对一致→删原件，现存
  [`存档/evidence/ORZ-DEV-LINKER-CRASH-001-guide-eq.pdb`](../../存档/evidence/ORZ-DEV-LINKER-CRASH-001-guide-eq.pdb)。**该件 GitHub 无对应物，本地永久留存**；ORZ-DEV-LINKER-CRASH-001 结案时可引用本路径。
- `.tmp-b075-dl-*` 四件（v0.6.13 双平台发行包 36.8MB/27.7MB＋`SHA256SUMS` 两小件）：**先注记后删**——075 批 `2cab472c`"保留作证据"令自本档起解除，处置理由＝GitHub Release `v0.6.13` 在档、可随时回下载复得。删除动作于本档落定后同批执行（回执见本节末尾补记）。
- **REV-083-17 就此闭合**（pdb 留存＋两包删除＋注记本节）；其余约 480 个无账面引用的过程件清理另批处理，不在本裁决范围。

### 10.2 新裁决 B：写入管控进机械层（REV-083-02 升级，本轮最重要立项意向）

**用户原文要点**：「现在最紧迫的应该就是读和写的对齐，我想把写入管控加在机械层部分，重点管控并机械审查具有风险的命令，直接适应性锁死一切直达系统盘核心的删除与修改命令，最起码要确保模型无法破坏当前环境载体。」

定案：REV-083-02 由"search_replace 补 canonical 沙箱或留痕"**升级**为**机械层写入管控立项**（建议下一批顺延立项；S1 设计档以 §10.3 业界对照为输入）。设计须满足的三条用户底线：①读写对齐；②危险命令机械审查＋系统盘核心删除/修改锁死；③**模型无法破坏当前环境载体（硬底线）**。

### 10.3 写入管控设计输入：业界对照（2026-09 调研）

| 路线 | 代表实现 | 机制 | 对 orz 的可借鉴点 |
|---|---|---|---|
| **可写根 allowlist** | Codex CLI `workspace-write`；Claude Code `additionalDirectories` | 默认仅工作区（＋temp/显式声明目录）可写；读写分离（读不设限）；workspace-write 默认断网 | 行业收敛默认＝"工作区可写、其余拒绝"——与真机直通不冲突：收紧面只在**工作区之外**；将来人工审批（裁决②保留位）恰好挂在外域写入分支 |
| **OS 进程级强制** | Codex：macOS Seatbelt／Linux Landlock＋seccomp／**Windows 实验性 RestrictedToken＋AppContainer**（沙箱后端约 1.7 万行）；Claude Code sandboxed bash（bwrap/Seatbelt）；Gemini CLI（Docker/Seatbelt） | 每个 spawn 的 shell 挂 OS 级规则，越界写由内核拒绝——字符串层审查可绕过，进程级才是保证 | Linux 直接借 **Landlock**（无特权、按线程、deny-by-default；orz 已有 seccomp 网络过滤先例，同型接线）；Windows 无 Landlock 对等物，业界答案＝AppContainer/受限令牌（工程重）或补偿式（下两行） |
| **命令策略引擎** | Codex `execpolicy`（`codex-rs/execpolicy/`：Starlark 规则＋模式匹配，allow/prompt/deny 三分类） | 命令字符串解析→规则库分类→放行/提示/拒绝 | 正是"机械审查风险命令"的成例，形态可直接借（规则库＋fixture 钉＋三分类）。**警示**：其 2026-02 已暴露 Unicode confusable 字符绕过案例——此类审查只能定位为 best-effort 风险闸＋审计留痕，不承诺保证 |
| **检查点/回滚** | Claude Code `/rewind`、Cursor checkpoints、Aider 自动 commit | git 快照补偿，破坏后可恢复 | orz 已有 content anchor 前像＋全量归档，journal 驱动 undo 是同构延伸；git 自动检查点为最便宜兜底 |
| **容器/VM 换环境** | Gemini CLI Docker、Devin/云代理 per-session VM、gVisor/Firecracker | 隔离靠换环境而非设边界 | 违背"真机直通"主哲学——不作主防线，可作高危任务 opt-in 隔离模式 |

**机械层映射建议（L0–L4，供 S1 设计档作骨架）**：

- **L0 目标规范化**：所有写目标经读面同款 canonical 化（`resources.rs` 既有代码复用），不可规范化即 fail-closed（拒绝 `\\?\`、UNC、8.3、subst、连接点等歧义形态按读面已验证的规则）。
- **L1 写路径策略表（工具面硬门）**：可写根 allowlist（默认＝工作区＋`.gsa` 会话卷＋显式声明目录）＋系统盘核心 deny 常量表（Windows：`C:\Windows`、`Program Files*`、EFI/引导、注册表蜂巢文件等；Linux：`/boot`、`/etc`、`/usr`、`/lib`、`/bin`、`/sbin`、`/dev`、`/proc`、`/sys` 等）＋**载体自保护**（orz 安装目录、`.gsa`、journal/存档恒拒写）。→ 直接满足底线②的工具面半边与底线③。
- **L2 命令面机械审查（`run_terminal_cmd`）**：借 execpolicy 形态（规则库＋fixture＋block/warn/allow 三分类），解析出的目标路径过同一 L1 策略表；诚实边界入档＝可被混淆绕过，定位为风险闸＋审计。
- **L3 进程级强制（保证层，per-OS 适配器）**：Linux＝Landlock（每 spawn shell 挂 ruleset，可写根之外全拒）；Windows 分两步——v1 只做 L1＋L2＋L4，AppContainer/受限令牌列为 v2 排期项。
- **L4 补偿与自检**：git 自动检查点或 journal undo（复用 content anchor 前像）；**载体完整性自检**可复用 `orz_source_manifest.sha256` 机制在启动/定期校验在役载体——使底线③"可验证达标"而非仅靠拦截。
- **与现有哲学的接口**：这不是沙箱化——可写根默认＝整个工作区，真机直通保持；收紧的只有"工作区之外＋系统核心"两块，与 Codex/Claude 的行业默认同构。读写不对称（§4.1 P1 反例）由 L1 对写面补齐而消解。

### 10.4 删除回执（补记）

2026-09-26 执行：`.tmp-b075-dl-SHA256SUMS`／`.tmp-b075-dl-SHA256SUMS-api`／`.tmp-b075-dl-orz-0.6.13-linux-x86_64.tar.gz`／`.tmp-b075-dl-orz-0.6.13-windows-x86_64.zip` 四件已删，`ls .tmp-b075-dl-*` 确认无残留；`存档/evidence/` 现存一件（ORZ-DEV-LINKER-CRASH-001-guide-eq.pdb）。复得路径＝GitHub Release `v0.6.13`（zip 27,713,322 B／tar.gz 36,814,163 B，075 档曾回下载逐位核证）。

### 10.5 第四轮设计收敛（2026-09-26）：写入管控形态定稿方向

**用户裁决要点**：L1 的 allowlist 半边**否决**——读层沙箱的动机是"先读后写"明确保障＋用户隐私，写层只有"安全"一义，按真机直通哲学不做限制性 allowlist；定案形态＝**收窄式硬锁定（deny 半边）＋L2 借 execpolicy 进机械层（审查＋留痕）＋L4 补偿**；L3 值得做——Linux 侧简单，Windows 侧提议接 **Windows 自带安全中心**做系统威胁行为拦截（"orz 进真机环境的优点就是可以直接使用真机环境中的全部资源"）。

**形态定案（对 §10.3 L0–L4 的修订）**：

1. **收窄锁死面（＝L1 的 deny 半边，allowlist 半边作废）**：机械层单一源 deny 常量表——Win/Linux 底层系统核心与组件路径；**三处同一表落地**＝工具面写入路径（search_replace 等，机械层中介天然可加）＋L2 命令面＋L3 进程面。工具面落地不是 allowlist、与真机直通哲学不冲突，且成本极低（canonical 化复用读面代码）。
2. **承重墙警示（入档为设计约束）**：在无 allowlist 形态下，"模型无法破坏当前环境载体"的硬底线完全压在锁死面＋L4 上——**载体自保护必须并入 deny 表**（orz 安装目录、`.gsa`、journal/存档、**本管控配置自身**），且管控配置完整性纳入 manifest 校验。模型若能改写 deny 配置或 orz 二进制即自败；这是本形态唯一不可让步的构造点。
3. **L2**：借 Codex execpolicy 形态（规则库＋fixture 钉＋block/warn/allow 三分类）进机械层审查 `run_terminal_cmd`，全程留痕；定位＝best-effort 风险闸＋审计（Unicode confusable 绕过案例入档为边界证据）。deny 规则须覆盖"翻转安全机制"类命令（如 Defender cmdlet，见下④）。
4. **L3**：
   - **Linux（定案方向）**：Landlock。实现注记：Landlock **无 deny 规则**（默认拒＋显式 allow），故实现为"spawn 时枚举 `/` 顶层目录、核心集之外逐项 allow 写权限，读不设限"——效果等价收窄锁死，不构成限制面；orph 已有 seccomp 网络过滤先例，同型接线。
   - **Windows（候选待探针）**：**候选 A＝受控文件夹访问（CFA）**，用户提议，调研支持其对口性：内核 minfilter 强制、按 exe 允许表拦截对受保护文件夹的写入（非病毒判断、确定性路径拦截）、事件 **1123（拦截）/1124（审计）** 落 Windows Defender Operational 日志——机械层可读取事件入 journal 成事实事件（合"静默机械审查层"记录事实的设计）。调研显示默认保护面已含 `C:\Windows`、`C:\Program Files` 系目录，且管理员进程不在允许表同样被拦。**四代价入档**：①全机作用域，不区分 orz 子进程与用户其他进程（保护表仅放系统核心时误伤可控，但用户安装器偶发被拦为已知 CFA 摩擦）；②需 admin 配置＝系统状态改动，须用户显式同意＋README 披露；GPO 管控企业机本地不可改；③粒度粗：exe 允许表全局作用于全部受保护夹，无 per-process×per-folder；④同用户 admin 子进程可 `Set-MpPreference` 翻转（防篡改对 CFA 的覆盖有限）→ L2 deny 规则覆盖 Defender cmdlet，且 L4 自检兜底。**候选 B＝AppContainer/受限令牌**（Codex 实验路线）：per-process 作用域干净但工程重，备选。
   - **探针任务（先行，沿 ACAF 影子惯例"先影子后翻转"）**：CFA 开 **audit 模式**（只记 1124 不拦截）→ 狗粮轮实测 → 读 1124 事件测系统目录覆盖与误伤率 → 据实测定是否翻转 enforce。探针结论回填本节。
5. **L4**：git 自动检查点或 journal undo（content anchor 前像复用）＋载体完整性自检（`orz_source_manifest.sha256` 机制复用）。

**REV-083-02 二次改写**：设计范围由"补 canonical 沙箱或留痕"（第一版）→"机械层写入管控立项"（第二版）→ 本版定稿＝收窄锁死面（三落地）＋L2＋L4＋L3（Linux Landlock／Windows CFA-or-AppContainer 待探针），allowlist 不做。

### 10.6 安全定位定案（2026-09-26，第四轮收口）

**用户裁决要点**：按当前写入面裸奔至今的体验与多轮狗粮结果，模型自身出现写入错误的情况不多，机械层也会稳定拦截写错的指令；「"高阻力＋强审计"而非绝对保证」**可接受**——对外直接写明即可，**我们不做绝对安全性声明**。

**定案记录**：

1. **写入管控的安全定位＝高阻力＋强审计，非绝对保证**。实证依据（用户口径，入档）＝写入面裸奔期间的多轮狗粮：模型自发写入错误率低、机械层对错指令稳定拦截。本档 §10.5 四代价（CFA 可翻转、L2 可混淆绕过等）自本定案起不作为方案缺陷、而作为**已知并声明的边界**。
2. **对外口径原则（总口径，适用全部安全面）**：安全声明一律不得超出机制能力——这正是本档 §2.1 两起 P1（审批叙事越界、journal"防篡改、防伪造"越界）的同源病根，自此有总则可依：**不做绝对安全性声明；每条声明标注证据档位**。建议措辞三档：**保证**（机制可证：ACAF 票据 fail-closed、journal 完整性/损坏检测、锁死面工具面硬拒）／**阻力**（高成本低收益：L2 命令审查、CFA 拦截）／**审计**（事后可查：journal 留痕、1123/1124 事件）。落点＝README 安全节＋`orz/SECURITY.md`＋写入管控设计档；REV-083-01/04/06 的改写按本三档措辞执行。
3. 本定案与 §10.5 合并构成写入管控立项的完整口径输入；立项时（S1 设计档）直接引用 §10.2–§10.6。
