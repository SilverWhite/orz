# THIN-HARNESS-REDESIGN 设计稿（v0.4）

> 状态：`R1 完成（未编译）/ R2a 完成（未编译）`（2026-08-27 用户裁决"开始
> THIN-HARNESS-REDESIGN R2 部分"；R2a = §4.4 黑板拉取第一步——检索派发
> 返回指针摘要（不删 inline 通道）；编译按用户指示推迟到后续轮次全部完成
> 后再统一执行；ADR-0010 / CLI_PROJECT_INDEX 登记仍按设计 §6 留待 R3
> 验证通过后实施）
>
> R2a 施工记录（2026-08-27，源码工作区 D:\CLI\orz，分支 feat/fusion-architecture）：
> §4.4 PUSH → PULL 第一步落地——检索派发完成路径新增
> `ORZ_RETRIEVAL_RESULT_CHANNEL=blackboard|inline` 通道开关（默认
> blackboard：主代理工具结果只回指针摘要「已写入 blackboard internal_ret /
> external_ret（N 来源 / M 结论，条目上限 8K）；需要时 blackboard_read
> section=... 读取」，子代理全文保留在分区/journal/retrieval-results 存档；
> inline：保留旧全文回传作 A/B 与回退）；`blackboard_read` 新增 live 检索
> 分区 internal_ret / external_ret 渲染（response/entries/ledger 三块，条目
> 上限 8K 复用方案 B 点读口径，带 epoch = 显式报错 live-only、receipt_id =
> 显式报错仅 actions 点读）；系统提示近零中间态补第 4 行检索分区拉取契约；
> 压缩/恢复标记与 blackboard_read 工具定义的 partition 枚举口径同步；
> 测试新增 7 项（channel env 解析、指针摘要默认、inline 回退、内部/外部
> 分区渲染、8K 条目截断、检索分区带 epoch 显式报错、检索分区工具级回达）。
> R2b（观察主代理按需读取行为）/ R2c（两轮数据后判定
> parse_retrieval_result_json 兜底通道回收）未动，inline 保留。编译未执行。
>
> R2a 审查处理（2026-08-27，全面审查 P2/P3 六项全数落地）：P2-3 用户
> 裁决选 a——write_section 每次派发**全量覆盖**分区（response / entries /
> ledger 均替换，不再 extend 累积；分区永远代表最近一次激活，指针摘要
> N/M 计数与分区内容一一对应）；P2-2 分区 ledger = 结构化 ledger 投影
> （source_id + 标题/URL），internal / external 同口径（此前 external
> source_ledger 从未被填充，渲染恒显 (none)）；P2-1 测试 env 锁共享化
> （模块级 RETRIEVAL_CHANNEL_ENV_LOCK，读方也持锁，消除 inline 测试
> 跨 await 持 env 期间并行 flake）；P3-1 用户裁决——恢复时重建分区：
> 检索分区随 ACP 激活快照 sidecar 持久化（internal_ret / external_ret，
> serde default 向后兼容），prompt 起始经 controller
> with_retrieval_partitions 灌回、run 结束折回快照（覆盖语义，无半成品
> 覆盖）；P3-2 MECHANICAL_CONTINUATION_PLACEHOLDER 分区清单同步补检索
> 分区；P3-3 检索分区渲染新增总上限 RETRIEVAL_SECTION_TOTAL_MAX_CHARS
> =32K（8K 逐条上限之外，超限显式标注省略条数与存档去向）。测试 +5：
> write_section 覆盖语义、派发级覆盖、external ledger 断言、分区灌回、
> 快照 serde 往返 + 渲染总上限。用户裁决「改动全部普适、不做跑分特化」
> 后补两处对齐：① plan 门同步摘除 ACP 交互/stdio 入口（acp_server 默认
> 网关、acp_server/codex_app/orz-tui/stdio_e2e 测试脚本全部去 plan 轮，
> 无门事件计数 18→9；§4.5 范围边界据此闭环）；② 工具超时逃生舱
> ORZ_TOOL_TIMEOUT_SECS 从仅 CLI 扩展到 ACP build_host（缺失保持默认
> 300s，0=不限，非法值显式报错）。仍不编译（用户裁决统一编译）。
>
> §4.6 审查处理（2026-08-27，调研 + 用户裁定「按建议做」）：工具超时分层
> 问题——run_terminal_cmd 继承 bash 工具自身的分层上限（schema 缺省
> 120s / 模型可传上限 300s / 非后台化前台块 300s），外层
> ORZ_TOOL_TIMEOUT_SECS 只是兜底，终端命令实际到不了外层值（official-r1
> 实测长命令即死于 300s）。修复（随统一编译轮验证）：① build_toolset 给
> run_terminal_cmd 配置 timeout_secs=900 / max_timeout_secs=900
> （config_timeout 同时把非后台化前台块上限抬到 900s）；② 通用工具执行
> 路径加 60s 周期心跳打点（run_tests 先例）——stall 回落不误杀长命令；
> ③ D:/tb-eval/.env ORZ_STALL_TIMEOUT 900→360（依赖新二进制打点，编译
> 前用旧二进制跑长任务有风险，.env 已加注释）。调研对照（其他家设计）：
> Claude BASH_DEFAULT_TIMEOUT_MS=120s / BASH_MAX_TIMEOUT_MS=600s（同样
> 分层「缺省+模型上限」，上限 10 分钟）；Codex CLI 单 shell 调用默认 300s、
> 后台进程另设等待预算；OpenHands 超时自动转后台重试；stall 类看门狗业界
> 一致按「进度喂活」设计（AceClaw progress-gated auto-extension、
> AgentField per-event 输出喂 idle watchdog，避免看门狗杀合法长操作）。
> 后台化维持禁用（调度器族禁令 2026-08-09），长命令策略=放宽前台上限+
> 进度喂活，不引入后台任务面。新增 orz-host config_tests 1 项
> （run_terminal_cmd 分层超时配置断言）。
>
> 统一编译轮完成（2026-08-27/28）：Linux musl 三件套构建成功（orz
> 104,775,192 B / orz-signer 1,388,448 B / orz-acaf-provision 1,206,424 B，
> 容器内全量重编 29m49s，静态链接 + 关键符号验证通过），产物已替换
> D:/tb-eval/orz-linux。测试验证全绿：orz-assurance 133 / orz-bin 31 /
> orz-host 219 / orz-loop 578 / orz-tui 178（0 失败；ignored 均为 conformance
> 采集与 live e2e；Windows 盘符语义测试按平台 cfg 门控）。编译发现的
> R1/R2a 遗留测试缺口已修复：transport 默认档指纹（EnabledMax）、whitelist
> 四个旧行为测试改封存断言、run_tests 声明面移除断言、探针翻转/预算块/
> 计划门证据断言更新、检索覆盖/激活身份断言（P2-3 语义）、checkpoint 阈值
> 显式化、sealed 测试去 plan 面、codex_app 直连拒绝断言、run_tests 环境
> 测试放行 Python 3.13 自注入 LC_CTYPE。还修复两处编译期问题
> （transport.rs `Self::thinking_mode_from_env` 限定、tools.rs 辅助函数
> 作用域）。构造脚本因 rust:1.97-slim 基镜像换 trixie 修正为直连官方源
> （build_orz_aliyun_trixie.sh），测试脚本 test_orz_aliyun_trixie.sh 同步。
>
> R1 施工记录（2026-08-27，源码工作区 D:\CLI\orz，分支 feat/fusion-architecture）：
> 4.1 工具面收敛（host GrokBuild 保留集 6 工具 + browser_read；controller 删除
> compaction_whitelist_add / retrieval_disposition / retrieve_project_docs 声明；
> 投影层 R1_SEALED_MAIN_TOOLS 统一封存；sealed_tool_denied 窄门）；4.2 系统提示
> 近零中间态（4 行；R2 检索分区行留待 R2a）；4.3 注入块（orientation 强制模板轮
> 退役=fire-and-continue 简短注入；SESSION 预算块删除；COUNTEREXAMPLE_GATE /
> MECHANICAL_AUDIT / DC 保留）；4.5 plan 门摘除（生产路径不再启用 plan_first，
> 代码休眠保留，物理删除留待 R3 清理裁决）；4.6 thinking 默认 max +
> ORZ_THINKING_MODE、ORZ_ORIENTATION_THRESHOLD 默认 50 + 无头 `-p` 接线、
> D:/tb-eval/.env 补 ORZ_TOOL_TIMEOUT_SECS=900 / ORZ_STALL_TIMEOUT=900。
> 检索结果改为每次调用即闭环（auto_close close record、fresh activation per
> dispatch、去 [ASSESSMENT]/disposition 往返）。编译未执行。
>
> R1 审查处理（2026-08-27，三路并行审查 + 修复）：P1 超时 env 容器透传已修
> （D:/tb-eval/tb_agents/orz.py 按 ORZ_MAX_WALLCLOCK 模式补四变量透传，含
> ORZ_TOOL_TIMEOUT_SECS / ORZ_STALL_TIMEOUT / ORZ_ORIENTATION_THRESHOLD /
> ORZ_THINKING_MODE）；P2-1 裁决：ACP 交互会话面保留 plan 门（不在 R1 范围，
> R3 统一裁决，见 §4.5）；P2-2 收口：project_doc_index / pdf_read 纳入封存
> 执行窄门（run_host_tool_with_timeout sealed_tool_denied），retrieve_project_docs /
> retrieval_disposition 标注为「休眠但可调用」（跨 run 恢复延续的唯一入口），
> R3 裁决彻底封死/物理删除；P2-3：auto-close 与 ADR-0010 close/continue 契约的
> 草案期偏差在 §4.4 明示，ADR/索引登记仍留 R3；P3：§4.1 计数修正（19 → 8）、
> orientation 注释笔误、run_tests_tool_def 死代码删除。仍不编译。
> v0.4 修订（2026-08-27）：中立问询阈值定为 50 逻辑模型轮（用户裁决；第一轮实测 89 题中 ≥50 轮的任务 20 个 / 22%，中位数 26——78% 任务不触发、大任务触发约 1 次；与"注意力衰减低、只需大任务轮检查"匹配）。
> v0.3 修订（2026-08-27）：中立问询计数语义确认（逻辑模型轮，非工具轮）+ 跑分 CLI 路径接线缺口认定；反例自查确认第一轮实际触发（终答前一次，保留）；边界工具改为"调用接口封存、独立块保留"；黑板拉取独立施工（R2，分步）；thinking 默认改 max（复读守卫已就位）。
> v0.2 修订（2026-08-27）：中立问询与反例自查保留（简短注入、触发阈值调大可配）；retrieve_project_docs 删除（8 工具定稿）；工具面按"硬保留 / 删除 / 边界"重审一轮。
> 权威关系：本文是 ADR-0010 的减法修订草案；裁决前不修改 ADR 与 CLI_PROJECT_INDEX，施工/验证闭环后再按流程登记。
> 用途：第一轮跑分（58/89 = 65.2%）后的薄 harness 重设计；为多轮施工（R1 编译轮 / R2 检索通道 / R3 验证清理）提供单一设计基线。

## 1. 背景与问题

- 第一轮：58/89 通过（含 custom-memory-heap-crash 重跑计入 1 分）；32 失败 = 23 超时（72%）+ 8 验证失败 + 1 基础设施。
- 官方基准：DeepSeek V4 Flash 0731 在官方 DeepSeek Harness（minimal mode）报 82.7%；Ante 0.preview.71 独立复现 82.7%（368/445 trials，max effort，总成本 $68.41）。
- 差距定位：同一模型，max effort + 极简 harness（4 工具、短提示、no skills、bare profile）= 82.7%；我们的 harness（high effort、18 工具、约 2.3K 字符系统文本 + 多类注入块、首轮 plan 硬门）= 65.2%。
- 结论：模型能力不是瓶颈，harness 摩擦是。摩擦 = 注意力稀释（工具面/提示词/注入块）+ 仪式往返（plan 门每任务约 2 次模型往返）+ 机械超时误杀（300s 硬杀长命令）。
- 动作速度已非瓶颈（实测每动作 0.8–0.95 次模型往返）；下一步做减法，降注意力负担。

## 2. 设计原则

1. 模型自己会的，harness 不重复教：不加"你是谁/做什么/怎么规划"。
2. 机械事实优先做成工具结果自解释；提示词只保留结果无法自解释的契约。
3. 安全职责留在机械层（ACAF/权限/预算/候选计数/read-anchor/半助理层/机械审计层），不下放到提示词。
4. 确定性机械注入、模型能直接看懂的内容，不给模型解释。
5. 模型越强，harness 只减不增；新增机制一律视为债务，须证明必要性。

## 3. 偏差认定：黑板回归"交流平台"定位

原设计（2026-08-08 `INQUIRY_FIX_AND_BLACKBOARD_PARTITION` / ADR-0010 §3.6）：

- 黑板 = 共享结构化状态视图、多 Agent 交流平台，单写者分区；
- 三层外化：极简状态行 / 每工具轮增量推送 / 按需 `blackboard_read`；
- "全量黑板渲染（每轮 system prompt）不做——模型不调 blackboard_read 时零稀释"；
- "框架唯一目的是让模型负担尽可能小"。

叠加历史（实现未跑偏，是设计演进过度）：

- 2026-08-15 PLAN-FIRST：黑板化指挥（console 默认面 = 黑板读写 + 只读核查，执行/变更/shell/检索全经助理层订单）、首轮计划轮硬门、分步计划状态机、receipt 链。
- 2026-08-24 MECHANICAL-AUDIT-LAYER：首轮 plan 门保留 + direct 执行面 + 静默机械审查层。
- 结果：黑板从"交流平台"变成"指挥控制面"，plan/exec/gate 分区承载命令仪式；主对话仍被大块注入（检索子代理全文、机械审计报告、反例门、orientation 强制模板轮、DC 强制模板轮）。v0.2 定稿：反例门与中立问询以"简短注入"形态保留（阈值调大），删除的是大块全文注入与强制模板暂停形态。

本轮裁决：黑板回退为通信/状态平台。分区保留（plan / exec / internal_ret / external_ret / gate_log / session），模型自主决定是否 `blackboard_read`；框架不主动把分区内容注入主对话（机械审计最终报告等留痕注入除外）。

## 4. 具体设计

### 4.1 主代理工具面：19 → 8（重审后定稿）

> 计数修正（审查 P3，2026-08-27）：原稿写「18 → 8」且删除表标「删除（10）」，
> 与表格 8 删除项 + 8 保留 + 3 边界 = 19 不符。定稿口径：**framework_fallback
> 检索模式下主面 19 → 8**；local_browser 模式下 web 族与 browser_read 互换，
> 主面为 7（web_search/web_fetch 隐藏、browser_read 显示）。

硬保留（8）：

| 工具 | 依据 |
|---|---|
| run_terminal_cmd | 2512 次，不可替代 |
| read_file | 792 次 |
| grep | 168 次，结构化输出优于 bash 回退 |
| search_replace | 155 次，精确写入 |
| web_search / web_fetch | 149 / 149 次，外部检索派发 |
| blackboard_read | 子代理通信 + 按需分区 |
| submit | 递交门（两阶段） |

删除（8）：

| 工具 | 理由 |
|---|---|
| list_dir（143） | bash 可达 |
| run_tests（37） | bash 可达；官方验证独立于 agent |
| search_tool（13） | 与 grep 重叠 |
| project_doc_index（20）/ pdf_read | 内部化，主代理不声明 |
| plan_write（186） | plan 门摘除 |
| retrieval_disposition（14） | 激活生命周期仪式退役 |
| retrieve_project_docs（20） | 用户裁决删除：主代理自己 grep/read，不再派发内部检索子代理；内部子代理随之下线 |

边界项（调用接口封存，独立块保留，A/B 观察后按需恢复）：

| 工具 | 边界理由 | 观察口径 |
|---|---|---|
| todo_write（48） | 模型自发使用中等；LIF 类长探索任务可能有记账价值 | 接口封存：不向模型声明、调用返回结构化拒绝；代码保留为独立模块，可经配置恢复 |
| update_goal（12） | 同上，价值更低 | 同上 |
| compaction_whitelist_add（10） | 压缩白名单是上下文保真机制，非纯仪式；LIF 长探索中固定任务背景可能有用 | 同上；观察压缩后关键事实是否丢失，需要时经配置恢复 |

注：web_search / web_fetch 在运行时路由到外部检索子代理执行（候选计数、ledger 留痕不变）；内部检索子代理（retrieve_project_docs / project_doc_index）本轮退役，主代理对项目文档直接用 grep/read/search_replace。`browser_read` 不在 8 工具表内，按检索模式门控保留（framework_fallback 隐藏、local_browser 显示并隐藏 web 族），两种模式下主面工具数不超 8。封存执行层（审查 P2-2）：project_doc_index / pdf_read 的 host 执行特例一并纳入 sealed_tool_denied 窄门；retrieve_project_docs / retrieval_disposition 保留「休眠但可调用」（跨 run 恢复 AwaitingDisposition 激活的延续入口，R3 裁决彻底封死/物理删除）。

### 4.2 系统提示：近零

现状：SESSION 预算块 + BASE_SYSTEM_PROMPT（1752 字符）+ PLAN_FIRST_FRAMEWORK_BLOCK（480 字符），另加 18 个工具 schema，每轮固定开销大；BASE 内还重复枚举工具名单与 plan-first 规矩。

目标：

- 中间态（工具结果尚未全自解释时）——自然语言逻辑块，无版本号、无 XML 围栏：

```
工具按需使用，一次一个；不需要的信息不读。
大文件读取返回截断信封，需要后续内容时按 offset 续读。
写入用 search_replace，携带当前内容锚点；不匹配按报错修正。
子代理检索结果写入 blackboard 分区（internal_ret / external_ret），需要时用 blackboard_read 读取。
完成后用 submit 提交：第一次返回交付清单，核查后再次调用确认。
```

- 目标态（工具结果自解释后）：

```
直接完成任务。完成后用 submit 提交（第一次返回交付清单，核查后再次调用确认）。
```

- 工具结果自解释要求（把契约从提示词搬进结果）：read_file 信封自带 offset 指引；run_terminal_cmd 超时报错自带时长与"长任务拆步"提示；策略拒绝自带 code/reason（已具备）；检索派发返回指针摘要（见 4.4）。

### 4.3 注入块：删除清单 / 保留清单（v0.3 修订）

保留（简短注入，提示词不解释；这些对准确性有实质价值，用户裁决保留）：

- ORIENTATION 中立问询：保留简短注入形态。
  - 计数语义（已确认）：按**完成的逻辑模型轮（大轮）**计数——一次 assistant 生成 + 其工具结果回放 = 1；同一响应内多个工具调用不拆分；deny 轮、gate 回答轮均计 1；transport 重试不计。**不是工具轮**。
  - 接线缺口（第一轮实测 0 次触发的原因）：跑分走 `orz -p` 无头路径，而该路径 `run_turn_with_guards(..., None, None)` 明确不带 orientation 状态（代码注释：只有 ACP 服务器带）。**R1 必须补接线**：无头 `-p` 路径按 run 创建 `OrientationSessionState`（一次性 run 内存态即可，无需 sidecar）。
  - 触发阈值：`ORIENTATION_THRESHOLD`（硬编码 7）改为 `ORZ_ORIENTATION_THRESHOLD`，默认 **50**（用户裁决）。依据：第一轮实测 89 题模型轮分布 min 6 / 中位 26 / 平均 43 / max 248 / p90 94；≥50 轮的任务 20 个（22%）——阈值 50 时 78% 任务零触发，大任务（约前 1/5）触发约 1 次（248 轮极端长任务约 4 次），与"当前注意力窗口衰减低、只需大任务轮方向检查"匹配。pre_handoff 触发并入反例自查（终答前方向核查由 COUNTEREXAMPLE_GATE 承担，去重）。
- COUNTEREXAMPLE_GATE：保留，终答前一次简短反例自查注入（约 150 字符），不做强制模板暂停。第一轮实测：到达终答的 run 均触发（未到达终答的 run 因超时/中断自然无终答，符合预期）。
- DIAGNOSTIC_COVERAGE：保留为信号触发形态（第一轮 0 次触发，实际零成本；debug 场景对 LIF 探究有价值），不做主动删除；阈值维持或随上下文调大。
- MECHANICAL_AUDIT 最终执行事实报告：保留（机械留痕 + 终答前注入）。

删除：

- PLAN_FIRST_FRAMEWORK_BLOCK（480 字符，随 plan 门摘除）
- ORIENTATION 强制模板轮变体（2026-08-14 暂停填表形态退役；只留简短注入形态）
- SESSION 常驻轮次预算块（预算保留为静默硬门，仅在耗尽时提示一次）

半助理层 / ACAF / 权限的机械拒绝信息（结构化 code/reason）与事件面 / journal 留痕不变。

### 4.4 检索子代理结果：PUSH → PULL

现状（controller.rs 检索派发完成路径）：

- 子代理返回全文 → `parse_retrieval_text` 解析 [DOC]/[SOURCE] → `build_structured_result`（机械 ledger + [RESULT_JSON] 校验）→ `write_section` 写自己分区 → 主对话收到子代理全文 + ASSESSMENT 行。
- 主对话膨胀来源 = 全文注入；黑板分区其实已写入（`write_section` 已存在），只是主代理仍收到全文。

目标：

- 主代理工具结果只回指针摘要，例如：`web_search: 已写入 blackboard external_ret（5 来源 / 2 结论），条目上限 8K；按需 blackboard_read section=external_ret 读取`。
- 子代理输出全文保留在分区与 journal（留痕不变），不再进主对话。
- `parse_retrieval_result_json` / claim×visibility 校验：机械 ledger（工具调用证据）保留；[RESULT_JSON] 解析降级为兜底通道，用 `ORZ_RETRIEVAL_RESULT_CHANNEL=blackboard|inline` 切换（默认 blackboard）。inline 保留两个施工轮作 A/B 与回退；两轮后若零命中，物理删除 parse 路径及其测试。
- 删除主面 `retrieval_disposition` 激活生命周期仪式（子代理每次调用即闭环，不做 close/continue 往返）。

> 分期与 ADR 偏差标注（审查 P2-3，2026-08-27）：本节「激活生命周期」部分
> （auto-close、fresh activation per dispatch、去 [ASSESSMENT]/disposition 往返）
> 已在 R1 提前落地（§6 同步）；它与此前 ADR-0010 §3.3/§4.4 的结构化
> close/continue 契约存在**草案期偏差**——本文是减法修订草案，ADR-0010 修订
> 与 CLI_PROJECT_INDEX 登记按用户裁决留待 R3 验证通过后实施。指针摘要
> （PUSH → PULL）仍按 R2a 施工，inline 通道保留两个施工轮作 A/B 与回退。

### 4.5 plan 门摘除

- `plan_write` 从主代理工具面下线；PLAN_FIRST_FRAMEWORK_BLOCK 删除；首轮直接进入工作工具（与 round 2 direct 面一致）。
- plan 分区保留在黑板（历史/审计），模型可不读。
- 防"惯性推进"的职责由机械审计层最终事实核对 + 提交门承担，不靠结构化 plan 仪式。

> 范围边界（2026-08-27 用户裁决闭环：改动全部普适，不做跑分特化）：plan
> 门摘除覆盖 **全部生产入口**——CLI/-p（main.rs）与 ACP 交互/stdio 会话面
> （acp_server.rs）一致不再启用 plan_first；`plan_first` 仅作休眠开关保留
> （controller 测试/回退），console 默认面随生产路径保留（两入口一致）。
> 原审查 P2-1「ACP 保留、R3 再裁决」的范围边界据此撤销。

### 4.6 思考档与命令保护时限

- thinking：默认档改为 EnabledMax（用户裁决；复读退化已有 OUTPUT-REPETITION 滚动哈希 + 序列内容门兜底，max 输出退化风险已被机械层覆盖），同时新增 `ORZ_THINKING_MODE`（high|low|disabled|max）环境变量覆盖以便 A/B 与回退。需要 transport.rs 改动 + 对应默认档测试更新（随 R1 一次编译）。
- **reasoning-stall 守卫已物理删除（2026-08-28 用户裁决）**：transport 的
  reasoning-stall 预算兜底（600s / 64K reasoning token 无 content → 降级）先前已
  **默认关闭**，本轮直接**删除**（无保留价值：官方 deepseek-harness 对 max 只等待不杀
  ——官方无生成期退化防护，仅 idle 看门狗 5 分钟 + max_tokens 256K 上限；复读判定
  content/reasoning repetition（滚动哈希 + 3-gram + 序列内容门）已足够；本轮 900s 档
  实测 write-compressor 即死于 stall 误杀——312s/~64K 估算 token 被掐断降级后挂死）。
  删除内容：`STALL_FIRST_CONTENT_TIMEOUT` / `STALL_REASONING_BUDGET_TOKENS` /
  `REASONING_CHARS_PER_TOKEN` / `REASONING_STALL_DETAIL_PREFIX` 常量、
  `DegenerationDetector::check_stall` / `reasoning_est_tokens` / `mark_first_chunk`、
  `ORZ_REASONING_STALL_GUARD` env 开关与透传、usage 估算复核审计、相关测试（env
  解析、时间/token 预算触发、SSE 级 on/default-off 两测试）共 10 项。复读哨兵保持
  活跃，降级梯/会话档位/failfast 计数语义不变（触发源=复读）。详见
  `DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md` 头注与
  `STALL_DEGENERATION_FAILFAST_DESIGN_2026-08-21.md` 头注。
- 工具超时：`ORZ_TOOL_TIMEOUT_SECS=900`、`ORZ_STALL_TIMEOUT=900` 加入 D:/tb-eval/.env（零编译）。依据：caffe-cifar-10、extract-moves-from-video 等存在 `TIMED OUT (300s)` 杀进程记录。
- 中立问询：`ORZ_ORIENTATION_THRESHOLD`（默认 50，原硬编码 7）+ 无头 `-p` 路径接线（随 R1 编译）。

## 5. 安全围栏保留清单（不动）

- ACAF / 权限轴（fail-closed）、工具轮预算硬门、web 候选计数、read-anchor 写前核证、`.gsa` 不可见。
- 半助理层：命令运行 / 写执行 / 检索派发，host 拥有 cwd/env/超时。
- 机械审计层：每对象仅最后一轮结果覆盖写、不给建议、最终执行事实报告注入。
- 事件面留痕、journal/台账、retention、恢复链路。

## 6. 施工拆解（多轮）

- 施工 R1（编译轮）：4.1 + 4.2 + 4.3 + 4.5 + 4.6 + §4.4 激活生命周期提前项
  （工具收敛至 8 + 边界三项接口封存、提示词近零、注入块保留/删除定稿、plan 门
  摘除（CLI 面）、thinking 默认 max + env 开关、orientation 阈值 env + 无头路径
  接线、检索每次调用即闭环 auto-close）。产物：新 orz Linux musl 三件套
  （orz / orz-signer / orz-acaf-provision，若签名/授权链路需要）。§4.4 指针
  摘要与 inline 通道回收仍按 R2a/R2b/R2c 分步施工。
- 施工 R2（检索通道，独立且分步）：4.4 黑板拉取。拆分为：R2a 检索派发返回指针摘要（不删 inline 通道）；R2b 观察主代理按需读取行为；R2c 两轮数据后判定 `parse_retrieval_result_json` 兜底通道回收。不一次性做完。
- 施工 R3（验证与清理）：A/B 小样本（§8）；判定 parse 兜底通道回收（删除或保留）；跑分脚本 env 固化；ADR/索引登记。

## 7. 构建环境（全部 D/B 盘，不碰 C）

- `CARGO_TARGET_DIR=D:\orz-target`（新建，预计 16GB+，D 盘现有 108GB 可用）。
- `CARGO_HOME=B:\.cargo`、`RUSTUP_HOME=B:\.rustup`、`DOTSLASH_CACHE=B:\.cache\dotslash`（沿用 build.ps1 既有配置）。
- 目标：x86_64-unknown-linux-musl（.cargo/config 已有 musl 段 + 加固 flags）。
- 产物替换 `D:/tb-eval/orz-linux/orz`（104,796,752 B）；施工前先对账二进制对应源码（`SOURCE_REV`=6372e41d 与当前 HEAD 6b208fb 不一致，需确认）。
- target 缓存已删，全量重编预计 20–60 分钟；可后台运行。

## 8. A/B 验证方案

- 样本：3–5 道代表题（c4 超时题 + 验证失败题，如 make-doom-for-mips、gpt2-codegolf、mteb-leaderboard、caffe-cifar-10、extract-moves-from-video），同题新旧配置各跑；再上 10 题小样本。
- 指标：reward / 通过率、超时数、工具轮数、模型往返数、每任务成本、前缀缓存命中率。
- 对比维度：旧（high + 18 工具 + 全文注入）vs 新（max + 8 工具 + 近零提示 + 黑板拉取）vs 中间态（只改 env 超时）——可叠加归因。
- 合规设置全程不变（§9）。

## 9. 合规说明

- 改动全部为 agent 侧配置/行为：官方 timeout_multiplier 不动、数据集 sha256 钉死、ACAF fail-closed、--upload --public、每任务 ≥5 trials。
- `ORZ_TOOL_TIMEOUT_SECS` 是容器内 agent 工具执行超时，非官方 agent timeout；thinking 档是模型配置；工具面/提示词不触发静态检查。
- 若社区提交仍关闭、仅 maintainer 可提交，本跑分为官方方式的自有评估数据，可自行合并与展示，不进公开榜单。

## 10. 风险与回退

| 风险 | 对策 |
|---|---|
| 减薄导致防"惯性推进"失效 | 机械审计最终事实核对 + 提交门兜底；A/B 数据验证 |
| 黑板拉取导致主代理漏读子代理结果 | 检索派发返回指针摘要（分区/条目数/上限），提示显式 |
| 近零提示导致工具使用方式歧义 | 契约下沉到工具描述/结果消息；逐工具核对 schema 自解释性 |
| 重编译风险（目标缺失/依赖变化） | 先对账源码提交；独立 target 目录；产物校验（大小/守卫符号/冒烟） |
| 回退 | env 开关（ORZ_THINKING_MODE / ORZ_RETRIEVAL_RESULT_CHANNEL / 超时）可即时回退；代码改动在 git 分支，可整体 revert |

## 11. 待裁决项

1. 中立问询阈值默认 50（原 7）、计数按逻辑模型轮、pre_handoff 并入反例自查——确认？
2. 无头 `-p` 路径补 orientation 接线（否则跑分里中立问询仍然不触发）——确认随 R1 做？
3. 边界三项（todo_write / update_goal / compaction_whitelist_add）接口封存、独立块保留——确认？
4. 黑板拉取按 R2a/R2b/R2c 分步施工、不一次性做完——确认？
5. thinking 默认 max + env 覆盖（复读守卫兜底）——确认？
6. ADR-0010 修订 + CLI_PROJECT_INDEX 登记：建议 R3 验证通过后再做。
7. （审查 P2-1）ACP 交互会话面的 plan 门是否与跑分面统一摘除——R3 裁决。
8. （审查 P2-2）内部检索 lane / retrieval_disposition 的「休眠但可调用」路径
   （跨 run 恢复延续）是否彻底封死或物理删除——R3 裁决；幻觉调用成本已评估
   （内部 lane 一次子代理运行，权限/预算/候选计数硬门照常生效）。
9. （审查 P2-2）run_tests 休眠执行路径（host.run_tests 反馈环）是否物理删除
   ——R3 裁决（当前仅执行层保留，声明面已移除）。
