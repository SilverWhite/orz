# ORZ 框架当前生效实际设计清单（2026-08-29 源码盘点）

> 性质：盘点记录（只读，以源码为准；不产生新设计裁决）。背景：
> 2026-08-29 用户指示——「机制侧东西已不少，很多内容都分在外部机械层」，
> 要求彻底清理一遍，看当前生效的实际设计都有什么。本文列出当前
> 源码中实际生效的机制（含休眠/退役但代码保留的形态），并标注与
> 设计文档的状态差异；后续清理方向以此清单为基础。
> 基线：orz 子模块 HEAD 5b3fe27（feat/fusion-architecture），工作树
> 含 S5-2 未提交改动；调研记录见
> [`DEEPSEEK_CAPABILITY_AND_SUBTRACTION_DIRECTION_RESEARCH_2026-08-29.md`](DEEPSEEK_CAPABILITY_AND_SUBTRACTION_DIRECTION_RESEARCH_2026-08-29.md)。

## 0. 总览：模型可见面 vs 机械层

| 层 | 数量级 | 说明 |
|---|---|---|
| 模型可见面 | 系统提示 **0 字符** + 工具 8 面（framework_fallback 模式） | prompt 全空（V2 §9.1），工具 = 6 GrokBuild + blackboard_read + submit |
| 运行注入块 | **10 类**仍注册在 `is_injected_block_text` | 反例门/plan 反例/orientation/编辑推送/压缩标记/白名单/状态行/budget/breaker/机械审计/动作台账/DC/checkpoint refill |
| 生成期守卫 | 复读（L=400/W=800/门槛 20/标点块 0.50/802 线）+ 3-gram（0.70/门槛 15）+ 空响应链（low 封顶）+ DEGENERATION_LIMIT=3 | transport.rs 常量 |
| 机械门/权限 | ACAF fail-closed（TB 强制）+ IPG + 权限桥 + 候选计数 + 写锚点 + read 粗门 + 检索模式门 | 运行期常驻 |
| 执行侧助理层 | 操作台（console/direct 双模式）+ 实体注册表 + 失败诊断 + 黑板动作栏 | console_default_enabled=true（生产路径） |
| 审计/事件面 | journal 链 + 事件 Schema v0.2 + verifier + 机械审计层（≤128 键）+ action ledger + epoch 归档 | 全链路 |
| 检索子代理 | 内部/外部双 lane，relay 纯函数路由 | web 族调用实际由外部子代理执行 |
| 压缩/折叠 | fold 桥 + 外挂台账 + 压缩标记 + whitelist | fold 触发 128K token |

一句话：**模型看到的是近零提示 + 8 工具；框架承载的是上面整张表**。
这正是设计 P6「对模型极简 ≠ 对框架极简」的现状——机械层并不薄。

## 1. 模型可见面（当前生效）

### 1.1 系统提示

- `BASE_SYSTEM_PROMPT = ""`（prompt.rs，2026-08-29 W4-R4 置空）。
- `build_system_prompt` 仅返回探针块（tool-availability 注入）。
- 契约全部下沉：read_file 信封/offset、search_replace 锚点、
  blackboard_read 分区简定义、submit 两阶段、写锚点校验拒绝信封。

### 1.2 工具面（framework_fallback 模式 = TB 实际模式）

主代理声明（源码确认）：

| 工具 | 来源 | 备注 |
|---|---|---|
| run_terminal_cmd | GrokBuild 保留集 | 两档默认超时（普通 300s/程序脚本 600s）、自动后台化、max 900s |
| read_file | GrokBuild 保留集 | 粗门 8–32KiB 可配、offset 续读 |
| grep | GrokBuild 保留集 | 结构化输出 |
| search_replace | GrokBuild 保留集 | 内容锚点、写后校验 |
| web_search | GrokBuild 保留集 | **relay 派发到外部检索子代理**；客户端 120s 总超时（S5-1） |
| web_fetch | GrokBuild 保留集 | **relay 派发到外部检索子代理**；候选计数域 |
| blackboard_read | controller 注入 | 分区：plan/exec/actions/entities/session/gate_log |
| submit | controller 注入（console_default_enabled=true） | 两阶段状态展示，无 plan 放行（V2 §9.3） |
| browser_read | 条件声明（local_browser 就绪时） | TB 下 local_browser probe 失败→framework_fallback，不声明 |

实际 TB 主面 = **8 工具**（6 + blackboard_read + submit；browser_read 缺席）。
web 族虽在工具面，但主代理不执行——dispatch 到外部检索子代理。

已封存（R1_SEALED_MAIN_TOOLS，接口拒绝 `sealed_tool_denied`）：todo_write、
update_goal、compaction_whitelist_add、list_dir、run_tests、search_tool、
project_doc_index、pdf_read、retrieval_disposition、retrieve_project_docs。
plan_write 不在此表（休眠 plan_first 路径启用时才声明；生产不启用）。

### 1.3 注入块（运行期仍会出现，但均从持久会话过滤）

`is_injected_block_text` 注册的 9 类前缀（P2-11 DC 清理 2026-09-01：
plan 反例变体 / `[DIAGNOSTIC_COVERAGE …]` / `[CHECKPOINT_REFILL …]`
随 DC 机制一并删除；`[任务状态]` 为 resident 状态行，不经由本过滤器，
此处不列）：

1. `[COUNTEREXAMPLE_GATE v0.1]` — 终答前一次反例自查
2. `[ORIENTATION v0.4]` — 软门三问（2026-08-29，阈值 50 轮）
3. `[本轮编辑 …]` — 文件编辑轮推送
4. `[前文上下文已压缩 …]` — 压缩标记
5. `[压缩白名单 …]` — 白名单 resident 块
6. `[TOOL_ROUND_BUDGET …]` — 工具轮预算
7. `[TOOL_POLICY_BREAKER]` — 策略违规提醒
8. `[MECHANICAL_AUDIT v0.1]` — 执行事实报告（≤128 键）
9. `[动作台账 v0.1]` — 折叠视图动作台账

模型面极简不等于零注入：**长任务里这些块仍会周期性出现**（反例门 1 次/
run、orientation 阈值 50、审计报告终答前）。

### 1.4 effort 与模型

- `MAIN_AGENT_MODEL = deepseek-v4-flash`（0731 检查点，API 默认）。
- thinking 默认档 **max**（2026-08-27 R1 起；官方 82.7% 基线）。
- 空响应链：max/high → 重试 ≤2 → low → 重试 ≤2 → 明确失败（V2 R1；
  `EMPTY_RESPONSE_MAX_RETRIES=2`，disabled 仅手动 A/B）。

## 2. 生成期守卫（transport.rs 实际常量）

| 守卫 | 常量 | 现值 |
|---|---|---|
| 复读滚动哈希 | L=400 / W=800 / HIT_LIMIT=20 | V2 R1 定案 |
| 标点块二级确认 | ratio ≥0.50 | 保留（防代码引用误杀） |
| 同字符连串触发线 | 802 = 2×400+2 | 保留（用户裁决） |
| 3-gram 兜底 | ratio 0.70 / NGRAM_HIT_LIMIT=15 | V2 R1（3→15） |
| 触发动作 | 显式拦截、**不降档**、不重试 | V2 R1（移除 session_thinking 单调降级） |
| run 级累计 | DEGENERATION_LIMIT=3 → run_invalidated | 保留 |
| 空响应重试 | 2 次/档 + backoff 0.5–10s + jitter | 保留 |
| 流中断重试 | CHUNKED_MIDSTREAM_MAX_RETRIES=1 | 保留 |

已物理删除/退役：reasoning-stall（2026-08-28）、跨轮 Runtime Stagnation
Guard（2026-08-22）、序列内容门（DNA/RNA/蛋白，V2 R1 全删）。

## 3. 执行侧机械层

### 3.1 操作台（console/direct 双模式）

- `console_default_enabled=true`（main.rs 生产路径；控制器默认 false，CLI 翻转）。
- 动作栏/结果栏/服务注册表（console.rs）；执行经 `run_host_tool` 权限/ACAF 链。
- direct 降级：3 连败（`DEFAULT_DIRECT_FALLBACK_THRESHOLD=3`）→ 询问轮
  switch/stay（最多 2 次重填，默认 stay）。

### 3.2 半助理层（V2 R2，2026-08-28 已实施）

- `diagnostics.diagnose`：11 个签名词典（terminal/file/process/environment），
  极简记录 ≤2KB（tail ≤12 行 / key_fields ≤8），执行失败自动派发。
- `entities` 注册表：process/file/environment 三域，容量 128/渲染 24，
  file 锚点 size/sha256/mtime/encoding、process exit_code/timed_out/status。
- target=实体级：ActionSpec.target_policy（None/File/Process/Environment/
  AnyEntity），target↔data 二选一 + 双写一致性 + 域前缀校验。
- 黑板 entities 分区（live-only，不进 epoch 快照）。

### 3.3 ACAF 控制面

- 独立签发器 orz-signer + 文件 keystore + K_session HKDF + 七项验票。
- TB 评测：`ORZ_ACAF_FAIL_CLOSED=1` 强制（用户裁决，不接受影子模式）。
- 票据：file_write_v1 / command_exec_v1 / network_v1 / control events。
- 控制事件持票：orientation/DC/反例门等（Blocked gate 跳过 fire）。
- 生产默认：main.rs 未设 env 时 fail-closed 仍生效（unset = enforced）。

### 3.4 权限/安全门

- IPG（instruction provenance gate）：用户 prompt 注入扫描 + workspace trust。
- 权限桥：ReadOnly/AutoApprove/Ask 分族；读写分离。
- 写锚点校验：search_replace 内容锚点 + 写后 verify + 拒绝信封。
- read_file 粗门：8–32KiB（env/TOML 口子），FileTooLarge 兜底。
- 候选计数：web_fetch/browser_read 共享 per-activation 域，cap=8
  （`DEFAULT_WEB_FETCH_CANDIDATE_CAP`）。
- 来源加权：白名单 1.1/默认 1.0/劣质 0.7 + `ORZ_SOURCE_WEIGHTING_CONFIG`。
- 候选预筛：URL canonical/host 去重/已知失败形态/相关性粗筛。
- 检索模式门：local_browser / framework_fallback / off 显式，禁止隐式切换；
  工具面跟随模式（模式 A 自动定档 + 机械降级记 transition）。

### 3.5 检索子代理

- 双 lane：内部（工作区文档，只读族）/ 外部（web 族 + browser_read）。
- relay 纯函数路由：web_search/web_fetch → ExternalRetrieval；其余 → Host。
- web_search 全局并发 1（主代理与外部 lane 共享信号量）。
- 结果通道：blackboard（默认指针摘要）| inline（旧全文，A/B 可切）。

### 3.6 压缩/折叠

- fold 触发 128K token（`DEFAULT_FOLD_TRIGGER_TOKENS`）、tail 8K
  （`DEFAULT_FOLD_TAIL_TOKENS`）、每次 max 注入 50K
  （`DEFAULT_MAX_INJECT_TOKENS_PER_ROUND`）。
- 桥内保留纯文本 assistant 消息的 reasoning_content（S5-1 修复 A，
  声明消息仍剥）；外挂台账文件 + 折叠指针 + whitelist（cap 16K）。
- 每 loop 折叠失败上限 3（`FOLD_WRITE_FAILURE_LIMIT`）。

## 4. 轮次/预算/终止

- `REQUEST_MAX_TOKENS = 256_000`（max effort 档）。
- `MAX_TOOL_ROUNDS = 120`（编译期默认；TB adapter 传 999）。
- 每轮注入 token 上限 50K；TEXT_DELTA_PACING 120ms。
- 墙钟：TB 经 `--max-wallclock`（harbor timeout − 余量），graceful
  run_invalidated 终止先于 harness 硬杀。
- submit：两阶段 requested→confirmed；无 plan 会话纯状态展示；
  机械审计 + 终答前反例自查承接交付核对。

## 5. 事件面/审计

- journal 哈希链（run-event v0.2）+ verifier（Python 236 断言级）。
- 机械审计层：执行事实对象（≤128 键）、仅结构化字段消费（P5，
  「400」哈希串零误报已回归）、最终报告随终答前注入。
- action ledger：近 2 轮尾部 + 折叠外挂文件 + `tool_running` v0.2 事件。
- epoch 归档：黑板按 plan epoch 轮换，`.gsa/blackboard` 存档。
- TUI 投影 + ACP server（交互路径；TB 用无头 `-p`）。

## 6. TB 评测实际配置（D:\tb-eval\tb_agents\orz.py）

| 项 | 值 |
|---|---|
| 命令 | `orz -p <instruction> --real --allow-write --allow-shell --max-tool-rounds 999` |
| 网络 | `--allow-network`（PUBLIC 任务）+ `--retrieval-mode local_browser`（probe 失败→framework_fallback） |
| ACAF | manifest/keystore/signer 注入容器 + `ORZ_ACAF_FAIL_CLOSED=1` |
| 模型 | `ORZ_MAIN_AGENT_MODEL=deepseek-v4-flash`（=0731） |
| 透传 env | ORZ_TOOL_TIMEOUT_SECS / ORZ_STALL_TIMEOUT / ORZ_ORIENTATION_THRESHOLD / ORZ_THINKING_MODE / ORZ_RETRIEVAL_RESULT_CHANNEL（仅宿主已设时） |
| 日志 | gsa volume 挂载（每 trial 子目录，host 实时可见） |
| 墙钟 | `--max-wallclock`（harbor timeout − 余量；900s 任务 → 840s） |

## 7. 与设计文档的状态差异（需清理的登记点）

### 7.1 事实性差异（调研发现）

- THIN-HARNESS-REDESIGN §1「82.7% = 4 工具 bare profile」→ 实际官方
  minimal mode 2 工具 / Ante 复现默认工具面。已落调研记录 §3，待修订
  设计文档（见调研 §5）。

### 7.2 实施状态（TODO P0-0j）

| 项 | 状态 |
|---|---|
| W1-R1 复读后置化 | S1/S2/S3 完成；**S4 复验未闭合** |
| W2-R2 半助理层 | S1/S2/审查处理完成；**W3-R3 余项未做**（实体 id 形态/分区命名/注册表形态、process/environment 探针扩展） |
| W3-R3 | A/B 小样本跑分未做；清理登记未做 |
| W4-R4 prompt 全空 + orientation 软门 + submit 门 | S1 完成；**S4 复验已跑（8/31）但条目未勾选闭合**；S5-1 修复完成 |
| W4-R4 S5-2 终端分层超时 + 中间回报 | S1/S2/审查处理完成；**S3 重建未做；acaf_e2e 7 项失败待排查**（疑似 S5-1 引入遗留回归，独立轮次定位） |

### 7.3 设计文档中已定案但当前源码状态待核

- 强制模板轮（ORIENTATION / DC）：已随 P2-11 DC 清理（2026-09-01）整体
  删除（`force_template_round` 休眠参数亦于 2026-09-01 复审收口）；
  orientation 仅保留软门 ✓。
- plan_first：main.rs 仅注释保留，`with_plan` 在 plan-gate 路径（已不
  是默认路径）；生产路径不启用 ✓。
- 序列内容门：设计文档 V2 已删；源码常量已删 ✓（文档标记 withdrawn 待 W3-R3 清理登记）。

## 8. 清理方向提示（供后续讨论，非裁决）

1. **机械层不是薄层**：清单 §0–§5 显示外部机械层有 10+ 类注入块、5 组
   守卫、8 个安全门、2 个检索 lane、压缩/折叠/审计/事件面。若继续减法，
   对象应是「模型可见面」而不是这些执行件；Kinsley 数据（49.4% ↔ 71.9%）
   已证明执行侧粗糙会直接掉分。
2. **冗余候选**（可评估后清理）：12 类注入块中 plan 反例变体是否仍有
   fire 路径；`TOOL_ROUND_BUDGET`/`TOOL_POLICY_BREAKER` 与工具轮上限/
   sealed 拒绝的重复面；WORK_TOOLS 23 项探针全集 vs 实际 8 工具面的
   死重（probe 仍探测 list_dir/run_tests 等已封存工具）。
3. **待闭合链**：W1-R1 S4、W3-R3、W4-R4 S4 勾选、S5-2 S3/S4 +
   acaf_e2e 7 项回归定位——先于任何新机制。
