# ADR-0010 分卷 03：§3 主 Agent 与检索子代理同构

> 本卷为 [`ADR-0010：ORZ 融合运行时、同构 Agent 与设计权威重整`](ADR-0010-fusion-runtime-and-agent-architecture.md)（AUTH-ADR-0010，ORZ 当前自然语言设计的唯一权威基线）的物理分卷 03（§3 主 Agent 与检索子代理同构）；规范正文以所指分卷章节为准，分卷总目录见主文件。分卷为物理拆分、**语义零增删零改写**——本行以下正文与分卷前原文逐字一致；卷题与本声明为分卷批新增（分卷批：0ca，2026-09-28）。

## 3. 主 Agent 与检索子代理同构

### 3.1 同构原则

默认保留一个主 Agent 与两个检索子代理：

- 项目文档/工作区检索子代理；
- 外部来源检索子代理。

三个 Agent 复用同一套：

- agent loop 与模型请求/流式响应架构；
- 模型配置、transport、retry、timeout、cancel、stall 和 wallclock 语义；
- tool registry、tool availability、permission、IPG 和 capability 评估机制；
- context、compaction、blackboard、journal、event、snapshot、archive 和恢复机制；
- orientation、信息充分性、停滞守卫和其他适用的 assurance 机制；
- 错误、terminal、budget 与审计语义。

两个检索子代理允许同时处于 `active`，形成最多双并发：项目文档/工作区检索一席、外部来源检索一席；
每个角色只保留一个 active instance，不动态复制同角色子代理。全局 `web_search` 同时只执行一个，
主 Agent 与外部检索子代理共享同一 semaphore；内部与外部检索 session 仍可并行推进。

三个 Agent 使用完全相同的模型、thinking、transport、具体工具 registry、context、compaction 和预算
默认值；各 session 独立记账，不共享递减余额，也不通过削减能力建立“轻量子代理”。工具自身的安全
timeout、网络限流和全局 `web_search = 1` 属资源所有权约束，不构成 Agent 配置降级。

不得再为检索子代理建立零工具、无 session、无 journal、自由文本假 ledger 或一次性模型调用的
特殊 runtime。子代理不是弱化版模型调用，而是**同构 Agent 在检索任务合同下运行**。

### 3.2 允许的差异

主 Agent 与子代理只允许因职责产生以下差异：

- role/system contract；
- task contract、scope boundary 与交付格式；
- blackboard 写入分区和返回对象；
- parent/child session identity；
- 文件写入权限的 deny-only 检索角色约束。

“仅执行检索任务”由任务合同、scope gate 和结构化结果验收保证，不通过硬编码 `tools: []`、绕过
主运行时或删除基础权限机制实现。新增第三类子代理、常驻多模型调度器或不同 runtime 仍需新 ADR。

子代理继续看到并调用与主 Agent 相同的工具，但所有写动作必须经过同一 permission/capability 路径，
且只允许写入以下逻辑域：自身 blackboard 检索分区、当前任务的检索文档、检索记录存档。session
bootstrap 必须把三个逻辑域解析为绝对路径/对象范围并写入 capability receipt；源代码、普通设计文档、
配置、测试和其他工作区文件默认拒绝。shell、MCP、脚本、软链接、junction 或路径重解析不得绕过该约束。

### 3.3 生命周期与结果

检索子代理至少具有以下状态：

```text
created -> active -> result_ready -> assessing -> awaiting_parent_disposition
                 ^                                      | close -> closing -> closed_resumable
                 |                                      \ continue(requirement_delta)
                 \----------- contract_revision + 1 ----------------/

active -> failed/cancelled -> closing -> closed_resumable
```

要求：

1. 主 Agent 通过显式任务合同创建/唤醒子代理；
2. 一个检索任务内 session 持久存在，允许多轮模型/工具动作；
3. 检索结果先形成并验证结构化 `query_summary`、`source_ledger`、`filtering_log`
   与 `raw_source_refs`（机械四段；v1.45 修订，2026-08-30：`organized_response`
   / `[RESULT_JSON]` 组织块删除——机械 ledger 单轨，见 §14.45）；
4. 机械 assessment 发生在结果形成之后，不能与首次检索模型调用合并，也不新增子代理模型轮；主 Agent
   随后必须给出结构化 parent disposition：`close` 或 `continue(requirement_delta)`；
5. 只有经过验证的 `close` disposition 才提交 close record 并清空 activation 的 live
   assessment/trigger/dedupe state，但不清零 journal、检索文档或归档；满足
   parent identity 与新任务合同后可以用新的 `activation_id` 恢复；
6. 关闭、失败、取消、预算耗尽和 scope 完成必须产生不同 terminal/receipt 语义；
7. 默认工具轮预算为 120；到限后提供一个无工具最终轮报告部分结果与明确终止原因。

现有 `retrieval-result-v0.1.schema.json` 与 `retrieval-session-close-receipt-v0.1.schema.json` 作为
迁移输入；新 writer 必须使用本 ADR 对应的新版本，旧版本只用于 replay。

### 3.4 统一模型、thinking、transport 与凭据

1. 默认 provider family 冻结为 **DeepSeek 系列**，当前 release line 为 **V4**。本次实现审计观察到的
   current model ID 是 `deepseek-v4-flash`；具体 SKU 通过单一 model registry/config 选择，不在三个
   Agent 构造器中分别硬编码。DeepSeek family 之外的默认 provider 变更需要新 ADR；V4 系列内 SKU
   变更必须留下兼容性、质量、延迟和工具调用回归证据，但不必为每次 SKU 更新新建 ADR。
2. 当前默认模型配置由同一 `ModelConfig` 构造并注入三个 Agent：thinking enabled、
   `reasoning_effort = max`、`max_tokens = 160000`。不得在子代理构造器中复制一份较弱默认值。
3. 所有生产模型轮走 streaming Chat Completions transport；连接、流空闲、总时长、取消和 partial
   output 语义统一。当前 retry/timeout 继承 ADR-0007：非流式 transient request 最多 10 次且 32 秒
   窗口封顶；流式请求不在已见输出后重发；20 秒 idle warning、90 秒 idle timeout、30 分钟流总预算，
   取消与超时严格区分。
4. reasoning replay 只在 provider adapter 单点实现；合法工具轮的空 `reasoning_content` 保持字节语义，
   最终输出为空时按“同请求一次重试 → thinking disabled 一次降级 → 明确失败”处理，不静默吞掉。
5. Credential 继续使用 ADR-0006 的 Windows Credential Manager 注册表：主 Agent
   `orz-deepseek/agent`、外部检索 `orz-deepseek/1`、内部检索 `orz-deepseek/2`。凭据目标不同只用于
   identity/audit，不改变模型能力和配置一致性。
6. Agent 级工具轮预算均为 120、独立计数、模型可见；deny 轮仍计入。每轮剩余量通过尾部机械消息
   提供，不写入随轮变化的 system prompt，以保持前缀缓存稳定。预算是 anti-runaway backstop，不是
   对正常复杂任务工作量的估计。预算按 session（activation 生命周期）连续记账：主 Agent 每 run
   独立起算；检索子代理经 `continue(requirement_delta)` 重入后保持同一 session——已耗工具轮跨
   dispatch 累计，不得因重入重置；只有 activation 关闭（close/失败/取消/预算耗尽）后，下一个新
   activation 才从 0 起算（v1.2 补写，2026-08-10）。

### 3.5 Tool availability、permission 与失败反馈

1. **单一探针面（v1.8 修订 2026-08-13，取代 v1.5「registry 全量 + 零可用性承诺」）**：session
   bootstrap 生成一次 **registry 能力目录**（会话声明集）；每个模型请求构造前对全部主 Agent 工作
   工具重算机械链路快照，本轮模型可见 = 机械链路完整 ∩ 会话声明集，**仅工具名、不标注状态**；
   链路不完整者确定不可提议。探针是“探针时刻”的确定判定，不是调用成功承诺——调用时机械门禁仍是
   最终兜底：每次工具调用由 permission gate 逐次判定（AllowOnce/Deny）并返回明确结构化结果。
   `tool_availability_check` 事件只在状态翻转时发出（run-start 空映射 → 当前快照为首翻，先于
   run_started）。tool registry 相同表示三个 Agent 具有同一能力目录，不表示每个参数组合都被授权。
   检索车道工具（web_search/web_fetch/browser_read/pdf_read/project_doc_index）由子代理确定性
   留痕与既有声明门治理，不参与主探针矩阵；§3.7.1 检索 mode 门禁 off 投影不受影响。
2. **单一探针面取代名级过滤与三面分类（v1.8 修订 2026-08-13）**：v1.5“统一声明完整 registry
   目录”与 v0.1 三面分类（恒声明/探针过滤/固定列表）废止——全部主 Agent 工作工具统一先探后列：
   本轮模型可见 = 机械链路完整 ∩ 会话声明集，仅工具名、无状态标注；不完整即移除（确定不可提议）。
   Interactive/ReadOnly/Benchmark 由探针快照驱动各自可见集（ReadOnly 下写探针不完整即移除）；
   ReadOnly/Grill 只读保证由执行层 gate 承担（ReadOnly policy 拒非读），不由可见性承担；写探针为
   metadata-grade，不构成写权限承诺。拒绝消息只陈述本次调用事实，主车道兜底消息与探针 reason 使用
   中性陈述（不使用 available/unavailable 等判定词；事件 error 码、机器 reason 与明确事实性内容
   不受此限）。MCP 名称（`{server}__{tool}`）prefix-spoof 防御保留（slice #16 D2-1），执行层同款
   deny 纵深兜底；`use_tool` 机械链路 = MCP/能力注册存在且会话作用域内。动机（v1.5 保留）：声明层
   与执行层不一致的“假 available”对 DeepSeek 行为不可预测（2026-08-11 TB 复盘）；polyglot 烧轮
   教训由调用时明确拒绝 + 本条 4 连续拒绝熔断承担。子代理 deny-only 写策略不受影响（lane 门禁在
   执行层，GAP-SUBAGENT-RUNTIME）；检索车道与主车道非工作工具保持各自既有声明规则。
3. 所有 tool path 必须返回非空 success/error/deny/timeout/cancel 结果；不得让模型从空字符串猜测状态。
4. 保留**同类拒绝连续三轮**的机械熔断，删除“每 run 累计拒绝 10 次”的总量机制。计数单位是已完成的
   tool-call round，不是同一 assistant response 中并列的每个 tool call；只有连续三轮都没有成功工具，
   且归一化 `(tool_name, denial_reason_code, policy_revision)` 相同，才注入一次换策略提示。成功工具、
   denial key 变化或 permission policy revision 变化都会重置连续计数。用户取消、timeout、tool error 与
   permission deny 分开记账。该机制只修正 tool-belief/availability，不触发 Orientation，也不承担总预算
   职责；全局 anti-runaway 已由 120 工具轮预算覆盖。
5. 子代理文件写域继续以 §3.2 为唯一 allowlist，并服从 ADR-0009：blackboard、检索文档与检索记录
   都是 workspace-related A 类写点，默认位于工作区 `.gsa/` 或任务合同显式指定的工作区路径；不得
   落入 B 类本体目录。B 类 runtime 配置/session/memory 仍走安装目录 `grok-home` 与既定降级链。
6. **缓存代价与请求 header 留痕（v1.9，2026-08-14）**：保持 v1.8 的声明面翻转语义——探针每
   模型请求前重算，列表只在真实状态变化时翻转；工具集变化会打穿 DeepSeek 前缀缓存（工具块由
   服务端模板前置渲染，变化即全量 miss）属接受代价，第二轮同前缀请求自动恢复命中。禁止为缓存
   牺牲机械化保障：per-window 探测、预热轮、工具层后置渲染、工具层压缩均否决。每个模型请求的
   header（system+tools 摘要 + config + 变化原因）变化时在 journal 留痕，用于审计翻转、归属
   miss 与核对探针准确性。（v1.19，2026-08-17：console 默认面下注册板块/工具栏为黑板数据、
   经 `blackboard_read` 取回，不参与 tools 摘要，工具栏刷新不构成前缀 miss 源；header 变化
   仅剩 console↔direct 模式切换与只读工具探针翻转等真实状态变化，见 §14.19。）
7. **探针准确性与稳定性优先（v1.9，2026-08-14）**：探针目标是“真实变化才翻转、误判最少”；
   翻转事件与 header 留痕配合，可事后核对每次翻转是否真实合理；可选后端接线时须同步补翻转
   测试（既有边界）；不为缓存调整探针语义或降低 fail-closed 程度。（v1.58，2026-09-06：
   翻转的记账与交叉核对只作用于当前可见声明面工具，封存工具的探针翻转不进事件面，见
   §14.58；分区形状子句随之收窄，见 §14.59。）

### 3.6 Blackboard、Mechanical Relay 与 context

Blackboard 是共享的结构化状态视图，采用单写者分区：

| 分区 | 写入方 | 内容 |
|---|---|---|
| plan/workboard | controller/批准后的 plan bridge | goal、步骤、状态、当前步骤和软约束 |
| edit actions | controller | 成功编辑的文件、行变化和时间戳 |
| tool actions | controller | read/edit/terminal/retrieval 等分类动作与时间戳 |
| internal retrieval | 内部检索子代理 | 项目来源、ledger、结构化结果 |
| external retrieval | 外部检索子代理 | 外部来源、ledger、结构化结果 |
| gate log | assurance/controller | gate decision、orientation/check identity |
| temporal | LIF/机械层 | 时间特征域记录（域标签 + 连续电位；PULL 查询面、零常驻 token、无注入；v1.47，2026-08-30，见 §14.47） |

所有 Agent 可按合同读取，只有指定 writer 可以修改对应分区。不得增加自由随记区或让模型直接写入
controller/gate 分区。Mechanical Relay 只根据结构化 event/function identity 做确定性路由，不调用模型、
不从自然语言猜测事件类型。

Blackboard 对模型采用三层外化：plan 存在时提供跨轮字节稳定的极简状态行；每个工具轮只增量推送本轮
动作/结果摘要；历史通过 `blackboard_read` 按分区和时间范围取用。禁止每轮把完整 blackboard 注入
system prompt。

黑板是控制面状态视图，不是内容缓冲：任何大内容（如文件全文）不得写入黑板；黑板/结果栏只放
指针（path/document_id/size/digest/offset），内容本体留在盘上或内容寻址证据区，避免 epoch
快照膨胀与过期副本（v1.22，2026-08-17，见 §14.22）。

Context compaction 对三个 Agent 使用同一策略（v1.10，2026-08-14）：有效窗口按 DeepSeek V4 检索
质量塌陷最低值定为 384K；实测 prompt tokens ≥160K 触发模板摘要（目标=五段模板 ≤17K 字符 +
白名单 + 最近尾），≥200K 为无视冷却的兜底触发（384K − 160K completion 预算 − 24K 余量）。工具调用记录
每轮机械坍缩为动作台账行（工具、目标、结果指针/digest，零模型调用、无冷却），完整记录保留在
journal/侧车审计面；模板摘要采用五段固化结构（目的/计划/变动文件路径/注意事项/后续衔接，字符
上限 3/3/5/3/3K=17K），目的/计划/路径由黑板机械填充，注意事项/后续衔接由模型生成并标
derived_unverified；摘要冷却 ≥2 模型轮（v1.14 审查修复，防长动作累积），摘要链只进审计存档、
不继承旧语义，滚动单 marker 附回查清单。v1.14 审查修复（2026-08-14）追加：缩减守卫不满足时
跨触发轮重试 ≤3 次、仍失败则强制执行一轮压缩并以 `guard_failed` 显式报告机制失败（不使用原始
机械截断）；每次成功会话（主车道与检索子代理一致）在结束时对全量消息估算超过
`session_end_trigger_tokens`（默认 160K）的会话自动执行一轮压缩，把摘要 marker
固定进 sidecar 一起存档（恢复治本，D2-2 恢复预检保留为旧会话兜底）；路径槽按 Top-40+5K 字符
封顶且为「本 plan epoch 增量」，溢出指针指向当前 epoch 快照；压缩不再清空/滚动黑板
（v1.15，2026-08-14：黑板生命周期按 plan epoch 轮换，与压缩解耦，见 §14.15；v1.15⑧，2026-08-15：
plan_epoch 为时间戳单调编号且身份一一对应强制、retention 保留最高编号快照，见 §14.15 ⑧）；退化守卫改为 ORZ 自定
300 等效字符（CJK 一字折算 2 等效字符，替代 orz-compaction 英文向 500 字符门）；摘要存档写失败
显式重试 ≤3 次并在事件/marker 中报告；摘要调用设 120s 专用超时。首个工具批次可以通过
`compaction_whitelist_add` 写入最多 16K 字符的客观任务背景；白名单常驻 preamble、跳过压缩，并随
session journal/retention 记录。详见 §14.10/§14.14 与 `docs/CONTEXT_COMPACTION_DESIGN_2026-08-14.md`。

每个模型轮的新注入工具结果设置 token 预算（v1.9，2026-08-14）：默认 50K、可经 env
`ORZ_MAX_INJECT_TOKENS_PER_ROUND` 调整，按模型轮累计（并行批内多结果累加），超限拒绝本批
后续调用并显式提示用 offset 续读；提示词层采用策略化读取（grep/结构提取优先、证据关键文件
才全文、大文件 offset 分段），与压缩阈值共同控制上下文增长与单轮缓存 miss 量。

读取工具契约升级为有界返回（v1.22，2026-08-17）：超过粗门的文件返回读取句柄信封
（path、size、encoding、content_sha256、可用范围、有界预览 ≤2–4KB、truncated、
offset 续读指针），不返回全文；小文件保持全文一次返回（一次往返）。粗门默认 16KB、
可配 8–32KB；精门=50K 注入预算为最终兜底。语义适配留在模型，助理层只提供机械原语
（引用 + 范围读），提示词策略化读取（grep/结构提取优先、证据关键文件才全文、大文件
offset 分段）落成工具契约；与 pdf_read 的 `document_id` + `page_range` 先例对齐
（文本文件用 path + offset）。详见 §14.22。

### 3.7 检索证据与外部浏览器边界

1. 检索模式是 session/task-contract 中的显式枚举：`local_browser`、`framework_fallback`、`off`。session
   未授权检索时从 `off` 开始；启用检索时优先选择 `local_browser`。`framework_fallback` 只有在用户或
   parent task contract 明确选择时才能进入，不能因 browser timeout、登录失败、CAPTCHA 或结果不足而
   自动切换。任何 mode transition 都必须产生带旧值、新值、authority 和 reason code 的机械事件。
   该条为冻结后补写（v1.65，2026-09-09，0t 检索子代理双车道定稿，见 §14.65）：检索模式三值状态
   （`off`/`local_browser`/`framework_fallback`）**整体退役**——不再作为激活状态，mode transition
   机械事件族不再产出；原 `off` 的「未授权默认无可检索工具」语义改由**独立检索启用门**承接（如
   `retrieval_enabled=false`，未启用会话主面与外部 lane 均无检索工具、派发仍 fail-closed 拒绝）；
   检索启用会话 external 子代理工具面**双族恒在**（browser_read + web_search/web_fetch，
   `retrieval_mode_requires_framework_fallback` 拒绝族退役），浏览器可用性改为纯事件事实
   （`browser_launch_result`），浏览器不可用按普通失败回传（§3.7.2 显式状态语义继续适用），
   换道由模型依静态标注自主完成、机械层不再降级。
2. `local_browser` 保留旧 LBR 设计的规范性状态与异常结果，但不冻结旧文档的单一线性步骤、60 秒
   timeout 或具体 tab 数：搜索、打开、读取、PDF 发现/下载/验证/索引、来源验证、清理均为显式状态；
   `LOGIN_REQUIRED`、`CAPTCHA_REQUIRED`、`SOURCE_UNAVAILABLE`、`INVALID_PDF`、`NO_TEXT_LAYER`、
   `PAGE_BLOCKED`、`POLICY_BLOCKED`、`TIMEOUT`、`PARTIAL_EVIDENCE` 等失败不得静默降级为成功。
3. URL policy 在初始导航和每次 redirect 后重新检查；默认拒绝 `file://`、browser internal、localhost、
   private IP、cloud metadata 与未移交的账户/支付/密码管理面。网页内容永远是 evidence，不是 instruction，
   不能扩权、请求无关 tab 或触发本地文件访问。默认只允许预定义读取函数；任意 JavaScript 不进入当前
   MVP，未来如引入必须独立记录、限时限量、仅作用于 Agent-owned tab，且默认禁止表单提交和非 GET 写入。
4. Retrieval result 除 query/filter/organized result 外，每个关键来源必须记录稳定 identity、source type、
   access time、visibility、observed scope、missing scope、最高允许 claim、limitation 和内容 digest（可得时）。
5. `full_text_observed`、`partial_text_observed`、`metadata_only`、`unavailable` 是不同证据等级。未读全文
   不得生成全文级归因，不得用摘要、搜索片段、二手转述或模型记忆关闭反例缺口。
6. 外部检索使用受约束的 browser/web 工具时，只控制自己创建或用户显式移交的 tab；**该条 tab
   生命周期措辞为冻结后补写（v1.46，2026-08-30）：tab target 可机械池化复用（会话级有界池，
   默认 4），一次调用独占一个 target 的控制权、归还即重置、模型永不接触 tab 句柄、内容/控制权
   不跨调用共享；「one tab lives exactly for this call」语义从 target 生灭调整为一次调用独占
   控制权（详见 §14.46）**；cookies、
   password field、auth header、local storage 和无关 tab 永不返回模型。论文、标准和 PDF-first 来源优先
   下载为内容寻址的本地证据，页文本、metadata 与 extraction record 可重建。
7. 全局 `web_search = 1`、browser tab ownership、站点速率限制和下载大小限制属于检索工具合同；它们
   不削减子代理 session 的模型、thinking、transport 或 120 轮预算。
8. 主 Agent 可以直接使用同一 `web_search`，但不得绕过 source ledger/visibility 规则；通过子代理检索
   的主要价值是隔离上下文、保留原始来源和降低主对话污染，不是把无来源摘要包装成事实。
9. 保留 FIX_PLAN D-1 的“claim-bearing 内容在使用处绑定可定位来源”原则，废止把固定
   `[来源: 路径:行号]` 字符串和 grep 命中当成充分验证。writer 使用稳定 `source_id` 绑定 ledger；本地
   代码可记录 observation-time `path:line`，内部文档优先使用文档 ID + section/anchor，外部来源使用
   URL/document identity + observed scope。renderer 可以显示 `[来源: source_id]` 或展开后的可读定位；
   verifier 必须检查 source identity 存在、可见性等级和 claim 上限，而不只检查标记文本存在。
10. 检索执行器（`web_search`）必须使用当前接入的 provider 的服务端搜索能力——DeepSeek 服务端
    web search（Responses API `/v1/responses`，与主 transport 同一把 key、同一供应商）；**禁止引入
    独立检索 API 供应商**（第二供应商、第二 key、独立计费）。该条为冻结后补写（v1.3，2026-08-11）：
    曾提议 xAI Grok 搜索后端独立 key（`orz-grok/search`），被用户裁决否决——检索是模型 API 的
    组成部分，不依赖外部检索 API；来源：web_search 执行器实施审计（2026-08-11），索引见 §14.3。
11. PDF 下载是**双通道路由**：可配置域名白名单（env `ORZ_PDF_BROWSER_DOMAINS`，逗号分隔、`*` 通配
    一个子域 label——`*.cnki.net` 匹配 `kns.cnki.net` 但不匹配 apex `cnki.net`；未配置=全部走直连）
    决定通道——**命中走浏览器**（CDP 下载，利用操作者经隔离 profile 手动登录的文献库会话），
    **未命中走直连**（web_fetch 同源 HTTP 通道）。白名单内的浏览器下载失败（未登录/付费墙/超时/
    取消）是**显式失败**，**不得自动回退直连**（§3.7.2 显式状态；登录后重试）。两种通道产出的 PDF
    都进入同一内容寻址证据管线（§3.7.6）。该条为冻结后补写（v1.4，2026-08-11）：用户裁决
    「每个学校买的文献库不一样，需要留白名单进行范围确定——白名单的走白名单（登录态），不在
    白名单的自动走直连」；来源：PDF 证据管线实施审计（2026-08-11），索引见 §14.4。
12. 检索来源质量：`web_search`（framework_fallback）使用**三层结构**，`local_browser`
    直接**分级加权**（v1.6，2026-08-12 用户裁决；v1.7 修订，2026-08-13：公众号主体级
    白名单已撤回、微博全站入劣质源、三层仅限 web_search；`framework_fallback` 与
    `local_browser` 二存一，禁止混用/隐式切换）：① 机械来源梯队——白名单（政府与
    机关单位，命中即满足权威性、子代理可直接采纳）weight 1.1、白名单外默认 1.0、
    劣质源 0.7（初始含 CSDN、知乎、百家号、哔哩哔哩个人专栏、微博、独立新闻媒体、
    自媒体新闻号与"XX财经"类自媒体号、小型个人站点；黑名单/营销域名与个人新闻号/
    非认证号，账号级由模型层判定；weight 为相对排序乘数，允许 >1，不作 0-1 置信度
    解释），域名级白名单不硬编码；② 选择性原文核验——**仅 web_search 使用**：引用经
    机械预筛+模型初选后，仅对高价值/结论依赖候选用 `web_fetch` 抓原文，禁止全量抓取；
    `local_browser` 不套第二层（`browser_read` 页面读取即原文）；③ 子代理模型加权标注
    ——账号认证状态判断、weight+理由输出，v0 为标注+排序不硬拦截（web_search 与
    local_browser 均使用）。抓到的原文走同一套加权并进 evidence ledger/visibility；
    `output_text` 只作线索不作证据。该机制属结果质量层，不改变 ACAF 授权边界
    （ADR-0011 D-12/D-13）。来源：检索来源加权设计文档（2026-08-12），索引见 §14.6。
    该条为冻结后补写（v1.9，2026-08-30）：主面在 `local_browser` 下可声明
    `web_search` 单一**派发入口**——模型调用即派发外部检索子代理，执行在
    子代理面完成；入口与执行解耦不构成模式混用。执行面仍二存一：外部 lane
    在 `local_browser` 下仅 `browser_read`（引擎 SERP）为检索通道，原生
    `web_search` 兜底是机械路径（`retrieval_mode_transition`，
    authority=mechanical_probe），不是子代理模型的自由选择；`web_fetch`
    族与 `web_search_*` 变体不进入 local_browser 主面（单入口语义，最小
    模型面变化）。来源：门禁观察四轮结构性零检索结论与用户裁决「先恢复
    外部」（2026-08-30），索引见 §14.44。
    该条为冻结后补写（v1.10，2026-08-30，GAP-RETRIEVAL-STRUCTURED-RESULT
    方向 C 用户裁决，索引见 §14.45）：③ 子代理模型加权标注**退役**——
    其唯一载体 `[RESULT_JSON].source_annotations` 依赖运行中不可见的
    后置分配 source_ids（机械 ledger 在子代理跑完后才签发），生产中从未
    生效（每次真实块 organized_response 恒空、
    structured_result_validation_failed 误触发）；`[RESULT_JSON]` 组织块
    契约整体删除，检索结果回归 `[DOC]`/`[SOURCE]` 声明行 + 机械 ledger
    单轨；`[SOURCE]` 声明行 URL 走 ACAF 网络目标规范化（SRC-002 表象
    修复）；`visibility_degraded` 重定义为「机械 ledger 无文本级证据
    （full_text_observed/partial_text_observed 均 0）」；机械来源梯队 ①
    与选择性原文核验 ② 语义不变。
    该条为冻结后补写（v1.65，2026-09-09，0t 检索子代理双车道定稿，见 §14.65）：v1.9 补写中
    「执行面仍二存一……原生 web_search 兜底是机械路径（`retrieval_mode_transition`，
    authority=mechanical_probe），不是子代理模型的自由选择」随三值检索模式 γ 退役——检索启用
    会话外部 lane 双族并存、换道为模型自由选择（机械层只做事实记录与正常回传）；「主面可声明
    web_search 单一**派发入口**（纯机械侧标注）」保留为检索启用会话主面投影的既有形态，不再
    限定于 local_browser 模式，模式差异不再进入工具面；web_fetch 族与 web_search_* 变体不进
    主面的单入口语义不变。本条第 ① 机械来源梯队、② 选择性原文核验与分级加权（local_browser
    直接分级）语义不变。

### 3.8 受控 `run_tests` / hidden-test 反馈环

保留 FIX_PLAN D-9 的单 run 测试反馈能力，但重裁其安全和控制语义：

1. `run_tests` 只在 harness/session contract 提供固定 command、cwd、timeout、environment/mount policy 时
   出现在工具声明中；Agent 不能改写命令或读取 hidden test 文件。
2. `run_tests` 是**受控代码执行**，不是 read-only 工具。即使它没有编辑 API，测试进程仍可能写文件、
   访问网络或启动子进程，因此必须经过 execution permission、sandbox/Job Object、timeout、输出上限和
   workspace delta/journal 记录。
3. 模型只接收 exit status、结构化 summary、脱敏且截断的 stdout/stderr tail 和完整输出 artifact identity；
   hidden test source、secret、host path 和无关环境信息不得通过失败输出泄露。完整输出保存在受控任务
   artifact 中，可按 permission 读取。
4. 每次 `run_tests` 调用按 §4.2 计一个 tool-call round。测试通过是实现证据，不自动证明设计符合；测试
   失败允许模型继续修复，但不自动扩大权限或暴露测试源码。
5. 不冻结“harness 自动再跑第二轮”为产品默认。额外迭代必须由 Agent 显式再次调用或由外部 harness
   contract 设置独立、有界的 attempt 数，并在不同 run/attempt identity 下记录。

