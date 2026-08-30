# THIN-HARNESS-REDESIGN V2 设计收口（2026-08-28：思维侧减法收尾 + 执行侧重设计）

> 状态：`设计定稿（v0.4，2026-08-28）`；未确定内容已全部收口，HA 架构调研
> 已启动（独立调研笔记见 [HA_SERVICE_MODEL_RESEARCH_2026-08-28](HA_SERVICE_MODEL_RESEARCH_2026-08-28.md)），
> 半助理层目标架构已补全（§4.5）。
> 性质：THIN-HARNESS-REDESIGN（2026-08-27 v0.4）的减法延续——思维侧收尾
> （复读守卫后置化）+ 执行侧重设计（半助理层加厚 / 工具稳定性 / 审计误报治理）。
> 权威关系：本文是减法修订草案的延续，**2026-08-28 用户裁决设计定稿**；
> ADR-0010 修订与 CLI_PROJECT_INDEX 登记按既有纪律留待 R3 验证通过后实施
> （同 THIN-HARNESS-REDESIGN v0.4 §6）；BACKLOG 0j / TODO P0-0j 已登记。
> 已定裁决（2026-08-28 用户）：①复读守卫统一命中门槛 **20**、序列内容门
> **删除**；②3-gram 门槛 3→**15**（随复读守卫大幅拉升）；③802 同字符连串
> 线**保留**；④空响应链 thinking 降档**最多降到 low、不关闭**，low 后再失败
> 即明确失败；⑤target 语义=**实体级**（无作用对象时省略 target）；⑥实体状态
> **并入黑板**（不新增只读工具）；⑦黑板定义**写进 blackboard_read 工具描述**
> （不进系统提示词）；⑧其余裁决项由主 Agent 定夺并收口（§3.3 / §4.2 / §8）。
> 历史：v0.1（2026-08-28）设计初稿，HA 架构待调研；v0.2（2026-08-28）
> 未确定内容收口；v0.3（2026-08-28）HA 结论确认 + 目标架构补全；
> v0.4（2026-08-28）设计定稿。
> **2026-08-28 全面审查处理（R1+R2 S1/S2 三路审查）**：R1 无需返工，仅
> 文档措辞勘误（§3.3 DEGENERATION_LIMIT 口径、§8 测试表述）；R2 两个
> P1 已按下述定案修复——①file 域签名匹配改消费 stat 探针信号（§4.2）；
> ②target 与 data 路径关系定案为"二选一 + 双写一致性校验 + 域前缀校验"
> （§4.5），grep 改 FileOptional。P2/P3 修复与余项入账见 §4.2/§4.5/§8。
> **2026-08-29 定案收口（S4 失败归因后，用户裁决）**：前 20 道错题 k=1
> 重跑实证 2/20 解出（model-extraction-relu-logits / protein-assembly，
> 均文件型 verifier、submit 实际被拒）；失败归因五类——submit 门死锁 /
> verifier 环境错误 / 真实交付质量 / 提前收束 / 超时（明细见 §9）。
> **提前收束根因**=R1 将 orientation 接入无头路径 + 阈值 50 + 注入块文本
> 残留旧"强制模板暂停"措辞（"只输出 JSON 模板、不要调用任何工具"），模型
> 纯文本回答被 loop 当终答 → 长任务 50 轮被掐断（旧二进制零 orientation
> 触发、无此现象）。定案：①BASE_SYSTEM_PROMPT **全空**（含"工具按需使用"
> 与 submit 引导行；契约全部由工具描述/信封/机械门承载）；②orientation
> 恢复**软门**（阈值 50、触发轮不禁工具、纯文本回答消费后续跑、终答只由
> 模型自发；强制模板轮设计保留在代码不启用）；③submit 门**无 plan 放行/
> 降级为状态展示** + submit 工具描述清 plan 措辞（与 prompt 清空同批
> 实施）。设计原则新增 P6（对模型极简 ≠ 对框架极简）。详见 §9。

## 1. 背景与动机

- THIN-HARNESS-REDESIGN §1 结论：同一模型 max effort + 官方极简 harness
  = 82.7%；我们的厚 harness（18 工具 / 约 2.3K 字符系统文本 / 多类注入块 /
  首轮 plan 硬门）= 65.2%。**模型能力不是瓶颈，harness 摩擦是**。
- 走偏记录（哨兵家族）：跨轮 Runtime Stagnation Guard 2026-08-22 全量退役
  （S4 实证"对正常长会话普遍误杀且从未拦截真实退化"）；reasoning-stall
  2026-08-28 物理删除（官方对 max 只等待不杀）；复读哨兵误杀史——DNA 低熵
  （→滚动哈希）、EGFP 合法序列引用 3/3 直降 disabled（→序列内容门）、
  sam-cell-seg 代码引用（→L=400 + 标点块二级确认）、3-gram 0.60 边界自引用
  （→0.70 + 累计命中）。**每一轮加机制都是在修复自己上一轮造成的误杀**。
- 本轮方向（用户裁决）：回归"降低模型压力"原路——**思维侧做减法（不控制
  模型怎么想，帮助模型更好地做出想法）**；**执行侧做加法（半助理层 /
  审计层 / 工具效率）**。
- 边界：**观察留流内、控制移结果侧**；新增机制一律视为债务，须证明必要性。

## 2. 设计原则（定稿）

- **P1 思维侧只减不加**：不控制模型怎么想。thinking 档位不再由生成期哨兵
  动态改变；只剩显式配置（`ORZ_THINKING_MODE`）与待裁决的空响应链两处可动点。
- **P2 观察与动作分离**：检测可以留在流内（被动、零动作、仅审计）；明确
  拦截只发生在触发点，动作显式、有原因、不重试、不降档。
- **P3 机械输出必须稳定**：工具结果/诊断/审计报告同构、锚点稳定、错误分类
  固定（step/code/message/trace_id）、无环境噪声。
- **P4 执行侧投资集中在工具效率与自解释契约**，不叠仪式；半助理层与审计层
  同样受"新增机制一律视为债务"约束。
- **P5 审计只消费结构化字段，不做子串扫描**（§6）。
- **P6 对模型极简 ≠ 对框架极简**：模型可见面（system prompt / 注入块）
  只减不加；执行侧（半助理层 / 审计 / 机械门）按需加厚，承重在框架不在
  提示词。官方极简 harness 的 82.7% 只说明模型侧可以极简，不意味着框架
  可以整体简化。

## 3. 复读守卫后置化（施工 R1）

### 3.1 现状核对（2026-08-28 源码 transport.rs）

| 常量/机制 | 现值 | 去向 |
|---|---|---|
| `REPETITION_MIN_RUN_CHARS` (L) | 400 | 维持 |
| `REPETITION_WINDOW_CHARS` (W) | 800 | 维持 |
| `REPETITION_HIT_LIMIT`（非序列） | 3 | **→ 20（统一）** |
| `REPETITION_SEQUENCE_HIT_LIMIT`（序列族） | 5 | **删除** |
| `REPETITION_SEQUENCE_LIKE_RATIO`（DNA/RNA） | 0.90 | **删除** |
| `REPETITION_PROTEIN_LIKE_RATIO` | 0.95 | **删除** |
| `sequence_kind` / `SequenceKind` / kind 审计 | 双族判定 | **删除** |
| 标点块二级确认（0.50 覆盖） | 有切分点路径 | 维持（防代码引用误杀） |
| 同字符连串触发线 | 802 | 维持（待裁决，§3.3） |
| 路径② 3-gram | 0.70 / `NGRAM_HIT_LIMIT=3` | **门槛 3 → 15（阈值 0.70 不变）** |
| 触发动作 | 中断 + thinking 降档（max→disabled 单调） | **改为显式拦截、不降档** |
| `DEGENERATION_LIMIT` | 3（run 级累计） | 语义保留（待确认） |

### 3.2 定案

1. **统一命中门槛 20**：`REPETITION_HIT_LIMIT = 20`，序列/非序列同门槛。
   成本账：20×400 = 8K 字符 ≈ 2–8K token，远低于 256K/600s；真循环连续
   复读会快速凑满，不会等到 256K（用户裁决：20 块成本可控）。
2. **删除序列内容门**：`REPETITION_SEQUENCE_LIKE_RATIO` /
   `REPETITION_PROTEIN_LIKE_RATIO` / `REPETITION_SEQUENCE_HIT_LIMIT` /
   `sequence_kind` / `SequenceKind` / 审计 kind 字段（sequence_gated /
   dna_rna / protein）全部移除。原误杀场景（EGFP 合法引用 3 次、蛋白序列
   引用）被统一门槛 20 自然覆盖——一个旋钮取代整套内容分类器。
3. **触发动作改为显式拦截**：流内累计命中 ≥20 → 终止该轮，detail 带
   `content_repetition` / `reasoning_repetition` + hits/limit + 明确原因；
   **不重试、不改变 thinking 档位**（移除复读触发的降级分支与
   `session_thinking` 单调下降路径）。run 级 `DEGENERATION_LIMIT` 累计
   语义保留，达限 → `run_invalidated{status: degeneration}`。
4. **观察留流内**：命中 1–19 次仅审计 WARN（span + 块统计聚合，按既有
   delta 聚合语义），不中断、不降档。
5. **已接受边界**：同一 400 字符块在单个流内被合法引用 ≥20 次会误触。
   概率极低（合法引用多为 1–4 次）；审计留痕可复查；若实机出现，再评估
   独立白名单（不预设）。

### 3.3 相邻项定案（2026-08-28 收口）

- 路径② 3-gram：`NGRAM_HIT_LIMIT` 3 → **15**（用户裁决；0.70 阈值不变，
  与复读守卫统一"只抓明确复读"口径；15 个 1K-token 高重复窗口才 trip）。
- 802 同字符连串线：**保留**（用户裁决；病理同字符流，成本极低，属明确
  复读，不引入误杀面）。
- 空响应快速重试链：**最多降到 low，不关闭**（用户裁决）——当前档
  （max/high）空响应快速重试 ≤2 次 → 降 low → low 档重试 ≤2 次 → 仍空则
  **明确失败**（不再降 disabled）。`ORZ_THINKING_MODE=disabled` 仅保留为
  显式手动 A/B 配置，自动链永不进入 disabled。
- `DEGENERATION_LIMIT=3`：**保留**（主 Agent 定案）——语义=run 级复读族
  trip 累计（content/reasoning 统一），成功轮**不清零**（单调），达 3 →
  `run_invalidated{status: degeneration}`。**2026-08-28 审查处理口径
  精确化**：`DEGENERATION_LIMIT` 计数只累计复读族 trip；空响应链耗尽
  是独立明确失败（`GatewayError::Model` → run_failed），**不参与该计数、
  不产生 `run_invalidated`**。

### 3.4 验证方案（S2/S4）

- 构造：同一 400 字符 span 原样回放 19 次 → 0 trip（仅审计）；第 20 次 →
  trip（显式拦截、不带降档、不重试）。
- 真实回放：EGFP 400 字符 span 3 次、sam-cell-seg 代码引用形态 3 遍 →
  静默（序列门删除后由门槛 20 覆盖）。
- 3-gram：14 次高重复窗口 → 0 trip；第 15 次 → trip（0.70 阈值不变）。
- 802 边界：801 不触发 / 802 触发（保留回归）。
- 空响应链：max/high 空响应 ×2 → 降 low → low ×2 → 明确失败（无
  disabled 档、无降级到 disabled 的事件）；low 档成功 → 恢复。
- 回归：poly-A 399/400/401 静默、路径② 3-gram、run 级达限
  `run_invalidated`、thinking 档位不变断言（trip 后无 `session_thinking`
  变化）。

## 4. 半助理层加厚（执行侧重设计，施工 R2）

### 4.1 定位

现状：模型无感执行器 + 硬安全围栏（注册表路由 → 契约校验 → 目标解析 →
ACAF 票据 → 执行 → verifier；失败信封 trace_id / step / code / message）。

目标：加厚为**环境问题预处理器 + 失败诊断器 + 工具自解释实现层**——模型
不需要自己摸索环境，也不需要为一次失败连跑多轮探测。

### 4.2 失败自动诊断（用户裁决方向）

- 触发：命令执行失败（非零退出 / 超时 / 目标缺失 / 工具不可用）。
- 动作（半助理层机械执行，无模型参与、确定性、有界）：
  1. **调用物状态**：目标/进程/服务是否存在（Windows：进程、可执行文件、
     路径解析、端口/服务状态）；
  2. **日志扫描**：捕获 stderr / 工具自身日志，按结构化字段抽关键词
     （错误码、路径、缺失符号/依赖）；
  3. **极简记录**：退出码 + 诊断要点（≤N 行，格式稳定）+ 全量日志指针
     （可 read_file 续读）。
- 约束：输出有界、确定性、无模型参与、不绕过安全围栏；关键词抽取同样
  遵守 P5（结构化字段，不做子串误报——见 §6）。
- 价值：失败命令从"原始报错"变为"发生了什么 + 去哪看"，一次往返拿到
  诊断；在 0.8–0.95 模型往返/动作之上进一步降摩擦。

**记录格式定案（2026-08-28，主 Agent 定案）**：`diagnostic` 信封 =
`{exit_code, matched_signature?, key_fields[], target_state[], tail[],
tail_is_raw, log_pointer, truncated}`；
关键词来源=**结构化签名词典优先**（各工具域声明已知错误签名：缺文件/目录、
命令未找到、退出码类别、超时、权限拒绝、网络拒绝等，每签名声明要抽取的
key_fields）+ **有界原始尾部兜底**（无签名命中时附 stderr 尾部 ≤N 行，
标记为 raw（`tail_is_raw=true`），不参与错误分类——与 P5 同纪律；有签名
命中时 tail 仍附有界原始输出作为补充上下文，`tail_is_raw=false`）；总上限
≤2KB 摘要 + 指针。

**签名词典明细（2026-08-28，R2 施工定案）**：签名匹配只消费结构化信号
（`exit_code` 精确/集合、`timed_out`、`output_encoding` lossy 标记、
`ToolError` 类别、`PolicyDenial{source,code}`、**stat 探针结果
（exists/kind/size，2026-08-28 全面审查处理补明：fs 探针结果属结构化
信号，签名可声明 stat 谓词）**），**禁止对输出文本做子串判定**（P5）；
无签名命中时降级为有界原始尾部（raw），不产生错误分类。
首批词典（按工具域分组，R2 S1 落地）：

| 域 | 签名 id | 结构化匹配信号 | key_fields |
|---|---|---|---|
| terminal | `tool_timeout` | `timed_out == true` | timed_out |
| terminal | `command_not_found` | exit_code ∈ {127 (POSIX), 9009 (Windows cmd)} 且非 timed_out | exit_code |
| terminal | `exit_nonzero` | exit_code ≠ 0 且无更具体签名 | exit_code |
| file | `target_missing` | exit_code ≠ 0 + stat exists=false | target_path, exists=false |
| file | `target_type_mismatch` | exit_code ≠ 0 + stat exists=true 且 kind ≠ 期望（期望按参数键推断：`target_directory`→directory、`target_file`/`file_path`→file、`path`→不限） | target_path, kind |
| file | `not_executable` | exit_code ∈ {126 (POSIX)} | exit_code |
| process | `process_timeout` | `timed_out == true` | timed_out |
| process | `exit_nonzero` | exit_code ≠ 0 且无更具体签名 | exit_code |
| environment | `tool_unavailable` | `PolicyDenial{source ∈ {permission,acaf,retrieval_mode}}` | source, code, reason |
| environment | `tool_not_found` | `ToolError::NotFound` | message |
| environment | `encoding_lossy` | `output_encoding` 含 `lossy` 标记 | output_encoding |

**2026-08-28 全面审查处理**：签名按"更确定信号在前"排序——`timed_out`
优先于 `command_not_found`（exit=127 + timed_out 归 tool_timeout）；
file 域 `target_missing` 只在实际 stat exists=false 时命中（目标存在的
失败——如权限拒绝/内容校验失败——不得误报 target_missing，无更具体签名
时落 raw 兜底），`target_type_mismatch`/`not_executable` 由此恢复可达
（原实现不消费 stat 导致二者成为死签名）。

诊断信封字段上限（确定性、有界）：`exit_code` 1 项；`key_fields` ≤8 项；
`target_state`（调用物状态：file 域 path/exists/kind/size；process 域
command/status——2026-08-28 审查处理补落地；environment 域由环境实体
摘要承载、诊断信封不重复）≤4 项；`tail`（有界原始尾部，≤12 行且 ≤2KB，
raw 语义见上）；`log_pointer` = trace_id（`assistant.trace` 续读；terminal
全量日志 `.gsa/session/terminal/<order_id>.log` 指针沿工具结果尾部既有
机制随行，不重复进诊断信封）；序列化总上限 ≤2KB，超出时按 tail →
key_fields → target_state 顺序截断并置 `truncated: true`。
文件锚点（size/sha256/mtime）由半助理层在实体登记时确定性计算（sha256
仅对**文件**（目录跳过）且 ≤16MB 计算，超限记 size/mtime 不记哈希，
避免大文件无谓开销；registry 层强制不变式，调用方误传超限哈希直接丢弃）。

### 4.3 Windows 环境预适配

- 证据基础：GAP-WINDOWS-EVIDENCE（`partial`）、ORZ-WIN-PROC-001/002/003、
  WIN-LIM-001（raw TCP）、WIN-INC-001（observer leak）、
  ORZ-TOOL-BINARY-COMPAT-001（glibc rg 误打包）、GAP-ENCODING-GATE。
- 预适配清单（初拟）：路径语义（盘符/UNC/分隔符）、编码（BOM/GB18030/
  UTF-8，复用 encoding 模块）、进程与超时（分层超时 900s + 60s 心跳打点）、
  shell 选择（PowerShell/cmd 语义）、工具可用性探测（静态链接守卫）、
  `.gsa` 可见性边界。
- 目标：模型不感知"这是 Windows"的多数细节；环境问题由半助理层提前消化。

### 4.4 HA 架构深化（调研已启动，结论已确认）

既有基线：CLASSICAL-EXEC-ASSISTANT v0.5——Home Assistant 服务模型
（domain.service + schema + 确定性执行 + 响应）+ hassil 文法为首选成熟
参考；操作台形态 = 注册表路由 + 契约校验 + 命令模式。

调研笔记（2026-08-28）：[HA_SERVICE_MODEL_RESEARCH_2026-08-28](HA_SERVICE_MODEL_RESEARCH_2026-08-28.md)。
初步映射结论：

1. 服务注册表直接沿用 `domain.service + target + data` 形态，域划分初拟
   `file / terminal / retrieval / diagnostics / delivery / blackboard`；
   关键借用=**target 语义**（动作作用对象定级、不设默认）。
2. 失败诊断 = `diagnostics.diagnose` 服务，执行失败时自动派发，返回极简
   记录（响应数据）；fail-closed 信封保留为失败通道（对应 HA 异常面）。
3. Windows 实体抽象（process / file / environment）消化环境细节，模型面
   只见稳定状态与稳定服务。
4. hassil 文法**不引入**（模型面是结构化 JSON 订单，无 NLU 需求）。
5. 注册表=接口层，ACAF=控制面，不建第二套审计链。

待用户确认项已全部确认（2026-08-28），目标架构小节见 §4.5。

原待深入问题（已由调研笔记逐条回答）：

1. 服务注册表粒度：工具/动作如何建模为 domain.service（文件、终端、
   检索、诊断各域）？注册表结构如何落 Rust 类型？
2. 失败诊断在 HA 模型中的位置：服务返回结构化结果，还是独立"诊断域"？
   HA 的 `response: optional/none/full` 语义如何映射"极简记录"？
3. Windows 实体适配：进程/文件/环境如何抽象为可控实体（entity + service
   模式）；HA 本身非 Windows 原生，借形态不借平台。
4. hassil 文法是否引入：我们的模型面是结构化 JSON 订单，文法层的价值需要
   论证（P4：不叠仪式）；初步倾向为不引入，只借服务注册表/契约形态。
5. 与现有 ACAF / 半助理层边界：注册表路由与现有实现如何收敛，不建第二套
   审计链（既有不变量 4）。

### 4.5 半助理层目标架构（v0.3，2026-08-28 用户确认）

- **调用形态**：`domain.service` + `target(entity_id)?` + `data`。target=**实体级**
  （用户裁决）：动作作用对象显式、必填、不设默认；无作用对象的动作（terminal.run、
  delivery.submit、blackboard.read/write_section、retrieval.web_search/web_fetch）
  **省略 target**，不硬造全局实体。
- **target 与 data 的关系（2026-08-28 全面审查处理定案，原为未定项）**：
  **二选一、双写必须一致（fail-closed）**——有作用对象的动作，target 实体
  id 与 data 中的路径参数任写其一（仅 target → 执行时把路径注入参数；
  仅参数路径 → 发放时自动生成 target），双写时按归一化路径机械比较、
  不一致拒单（`step=target` / `code=target_mismatch`）；target 域前缀与
  策略不符（如 File 动作带 `process:`）同样拒单。契约校验在 target 解析
  **之后**执行（注入/生成后的参数面才是执行面）。`workspace.grep` 因 path
  可选（无 path=工作区级搜索）使用 **FileOptional** 策略：有 target 时有
  作用对象，无 target 且无 path 时省略，不强制编造 target。
- **实体抽象（状态视图）**：`process` / `file` / `environment` 三域起步。实体状态
  **并入黑板**（用户裁决）：半助理层执行工具时登记/更新实体（文件锚点 hash/size、
  进程 pid/exit_code、环境可用性），模型经 `blackboard_read section=entities`
  按需点读；**不新增只读工具**。
- **服务-实体映射（初拟；2026-08-28 审查处理注明实际注册表名）**：
  - `file.read / file.grep / file.search_replace / file.write` → target=file
    实体（锚点即 content_anchor）；实际注册表名为 `workspace.read_file /
    workspace.grep / workspace.search_replace`（`search_replace` 兼任独立
    write 服务；`workspace.list_dir` 同属 file 域 File 策略）；
  - `process.status / process.wait / process.kill` → target=process 实体；
  - `diagnostics.diagnose` → target=失败对象实体（file / process / environment），
    由执行器在命令失败时自动派发（§4.2 极简记录）；
  - `terminal.run` / `retrieval.*` / `delivery.submit` / `blackboard.*` →
    全局（无 target；URL/section 等在 data，ACAF canonical 语义不变）。
- **黑板定义与解释**：写在 `blackboard_read` 工具描述里（用户裁决）——简定义
  （黑板=框架状态区，分区保存实体状态/检索结果/审计留痕）+ 分区清单由工具
  结果自解释；**不进系统提示词**。其余工具同纪律：契约下沉到工具描述与结果。
- **返回面纪律（大小）**：所有半助理层返回同构、有界、指针化——成功路径=
  摘要 + 上限 + 指针（全文留盘/黑板，模型按需续读）；失败路径=fail-closed
  信封 + 极简诊断（总上限 ≤2KB，含退出码/签名要点/有界原始尾部/指针）；
  检索与实体渲染均设条目上限与总上限（检索沿用 8K/32K 既有口径，实体渲染
  同理：摘要清单 + 单实体详情按需拉取，不预置全量）。
- **边界**：注册表=接口层；ACAF=控制面；不建第二套审计链。

输出：HA 调研笔记 + 半助理层目标架构（§4.5）。

## 5. 工具结果稳定性契约（跨 R1/R2）

- 总则（用户裁决）：**机械输出必须稳定**。
- 条款：同构信封（字段/顺序固定）；锚点稳定（offset/size/hash 可复核）；
  错误分类固定（step/code/message/trace_id）；无环境噪声；结果自解释
  （指针 / 续读指引 / 超时建议）。
- 适用范围：所有 host-owned 工具结果、失败诊断记录、审计报告、检索指针
  摘要。
- 验收：模型在近零提示词下能单轮正确消费任意工具结果（THIN-HARNESS
  §4.2 目标态）。

## 6. 机械审计误报治理（随 R1）

- 案例：S4 事件链校验/冒烟统计出现 8 处"400"，全部为哈希串内子串，真实
  HTTP 400 = 0；若按子串扫描即误报。
- 定案（P5）：审计只消费结构化字段（transport 事件 `status_code`、错误
  detail、reason code、envelope 字段）；**禁止对 payload / hash / 日志文本
  做子串判定**。
- 落实点：事件链校验器、机械审计层、smoke 检查脚本、失败诊断关键词抽取
  （§4.2 同纪律）。
- 验证：构造含 "400" 的哈希串/文本 fixture → 零误报；真实 400 事件 →
  精确报出。

## 7. 实施路由（建议拆轮）

- **R1（复读后置化 + 空响应链收口 + 审计误报治理）**：S1 代码 → S2 测试 →
  S3 重建 → S4 复验；范围=门槛 20 统一 + 序列门删除 + 3-gram 15 +
  802 保留回归 + 空响应链 low 封顶 + 审计结构化字段判定。
- **R2（半助理层失败诊断 + Windows 预适配第一批）**：先补 §4.2 细节设计
  （诊断格式/上限/关键词来源）→ 同路由。
- **R3（HA 架构深化调研 → 半助理层目标架构）**：调研已启动
  （[HA_SERVICE_MODEL_RESEARCH_2026-08-28](HA_SERVICE_MODEL_RESEARCH_2026-08-28.md)），
  从五个问题入手（§4.4），先调研后设计，不预设工期；与用户共同进行。
- 计数纪律：设计轮不动；实施入账/闭环按既有规则登记。

## 8. 待裁决 / 开放项

> 2026-08-28 收口：设计侧未确定内容已全部定案（§3.2/§3.3/§4.2），
> 无遗留待裁决项。

- 序列门删除范围定案：常量 / `sequence_kind` 判定 / `SequenceKind` /
  kind 审计字段**全量删除**；相关测试**改判新语义**（EGFP/蛋白真实 span
  保留为"统一门槛 20 下不触发"回归断言，2026-08-28 审查处理勘误原"全量
  删除"表述）；旧设计
  `REPETITION_DETECTOR_SEQUENCE_CONTENT_GATE_DESIGN_2026-08-25.md`
  标记 `withdrawn`（被本设计取代）按 TODO W3-R3 清理轮与 ADR-0010 修订、
  CLI_PROJECT_INDEX 登记一并实施（2026-08-28 审查处理对齐 §8 与 TODO
  措辞，避免"随施工登记"与 W3-R3 计划的张力）。
- 失败诊断记录格式与关键词来源定案：结构化签名词典 + 有界原始尾部兜底
  （§4.2），R2 施工轮只需补充各工具域签名词典明细。
- HA 架构五个待深入问题（§4.4）为下一阶段共同调研项，不属于设计裁决。

## 9. 2026-08-29 定案收口（S4 失败归因后）

> 实证：前 20 道错题 k=1 重跑（新二进制 6cc8586，job
> `official-r2-failures-c1/c2`）2/20 解出。失败归因五类：①submit 门死锁
> （7 题尝试 submit 全被 `no plan in force` 拒绝——旧二进制同样存在，R1
> 摘除 plan 门后由"可绕开"变"必死 + 烧轮调查"）；②verifier 环境错误
> （pytorch-model-cli libGL.so.1 缺失，收集阶段报错）；③真实交付质量
> （query-optimize 运行时长 0.986s>1.05×golden、extract-elf 0% 匹配参考、
> dna-insert 引物 Tm 差 7.09>5、filter-js-from-html XSS 与"原样保留"双挂）；
> ④提前收束 5 题（50 轮处 orientation 强制纯文本回答被当终答，交付物缺失
> ——chess-best-move / make-doom-for-mips / make-mips-interpreter /
> caffe-cifar-10 / gcode-to-text）；⑤超时 8 题（其中 5 题 web 研究过重：
> torch-pipeline-parallelism 21 / count-dataset-tokens 24 / gpt2-codegolf
> 14 / tune-mjcf 11 / raman-fitting 13 次 web 调用）。journal 全查零真实
> 400、零复读触发。

### 9.1 Prompt 全空（2026-08-29 用户裁决）

- BASE_SYSTEM_PROMPT 置空（含"工具按需使用，一次一个"行为行与"完成后用
  submit 提交"引导行）。
- 契约归属：read_file 信封/offset、search_replace 锚点、blackboard_read
  分区简定义均已落在对应工具描述；submit 两阶段在 submit 工具描述；机械门
  （写锚点校验 / 拒绝信封）兜底。注入块（反例门 / orientation / [本轮编辑]
  / 预算耗尽）走消息层，与 system prompt 无关。
- 测试：`base_system_prompt_is_near_zero_intermediate_text` 反转断言为空。
- 原则：P6（对模型极简 ≠ 对框架极简）——框架侧重机械层（HA 助理层 / 审计
  / 门禁）不变，减法只作用于模型可见面。

### 9.2 Orientation 软门（2026-08-29 用户裁决）

- 形态：阈值 50（`ORZ_ORIENTATION_THRESHOLD` 可配）触发；注入块改为简短
  方向检查文本（去除"只输出 JSON 模板 / 不要调用任何工具"）；触发轮不禁
  工具——模型可回答后继续，也可直接继续动作；纯文本回答被"消费"后 loop
  明确续跑（复用 pending-checkpoint 消费路径，去掉模板校验与工具禁令）；
  终答仍只由模型自发（非问询轮的纯文本响应）。
- 强制模板轮（2026-08-14 硬门）设计**保留在代码不启用**
  （`force_template_round` 休眠参数维持），后续需要时再启用。
- 风险与回退：软门重新开放 2026-08-14"拉回失败"窗口（path-tracing /
  make-doom 类循环触发多次不矫正）。本轮实证硬门在 50 轮直接掐断 run 是
  更严重回归（5 题提前终答），且 make-doom 本轮仍死于硬门而非被其救回。
  S4 复验专门盯 path-tracing / make-doom 形态；若复发优先执行侧（工具
  效率/诊断），不叠硬门。
- 延迟 commit 固有边界（2026-08-29 审查收口）：fire 与 pending 消费之间
  硬中断（传输错误/取消/panic）时，计数不提交、journal 留一条已 fire 未
  消费的孤儿事件；下次 run 在 loop-top 首轮重触发（到期方向检查不丢失，
  符合 recovery-resumes-counting 语义）。接受现状并注释于代码，不补机制。
- **S4 实证偏差（2026-08-29 复验发现；S5-1 已实施修复）**：触发轮工具面实际仍为空
  ——`agent_loop.rs` 的 `pending_checkpoint.is_some() → Vec::new()` 对
  orientation 与 DC 统一禁工具，未落实本款「触发轮不禁工具」；实机触发
  轮（make-doom / gcode-to-text / video-processing 均有）模型只能以纯
  文本输出 XML 工具调用，浪费一轮真实工作。定案修复（用户裁决 2026-08-29）：
  orientation pending 放行工具、DC 强制模板轮保留禁工具（拆两条路径）。
  **S5-1 已实施（2026-08-29，orz ad5f9ee + 审查处理批）**：
  `pending_keeps_tools` 拆两条路径——orientation pending 放行探针/工具栏/
  console 注册/工具派发（回归测试 `orientation_trigger_round_keeps_tool_face_projected`），
  DC 强制模板轮与 console 询问轮保持禁工具；pending 在工具轮亦提交消费，
  无重复注入。

### 9.3 Submit 门修复（并入下一实施批次）

- 无 plan 会话：submit 放行 / 降级为纯状态展示（设计本就定位"信息展示、
  非硬门"），不再 `no plan in force` 拒绝。
- submit 工具描述清掉 plan 措辞（"The final plan step is the fixed
  递交/完成 step…"）。
- 与 prompt 清空同批：base prompt 的 submit 引导行随清空消失。

### 9.4 实施批次（1-3 已于 2026-08-29 实施；4 复验重排；5 S5-1 已实施）

1. [x] prompt 全空 + near-zero 测试反转（2026-08-29 完成，S2 全绿）；
2. [x] orientation 软门（块文本 + 消费续跑路径；强制模板轮休眠不动；
   2026-08-29 完成）；
3. [x] submit 门与描述清理 + 回归测试（2026-08-29 完成，含审查处理：
   无 plan 确认文案不虚构机械最终回答流程）；
4. [ ] 后续 S4 复验（错题重跑回归：提交不再被拒、长任务不再 50 轮提前
   收束、命中率对比；S3 重建已于 2026-08-29 完成，musl 三件套对应源码
   orz 5b3fe27）——原四题补跑批次（official-r2-failures-s5-1）**作废
   重排**（2026-08-29 用户指示：不沿用旧批次，全部处理完成后重新安排）；
   ADR-0010 修订（§14.42 已补写）与 CLI_PROJECT_INDEX 登记按 R3 纪律
   一并处理。
5. [x] S5-1（问题 A/B + web_search 120s 超时 + 事件面 wall_ms/timed_out；
   2026-08-29 实施 + 全面审查处理完成，orz ad5f9ee，S3 musl 三件套已
   重建）；S4 复验批次作废重排，见 §9.6。

**S4 复验结果（2026-08-29 完成，31 题全量 k=1，harbor 实机）**：

- 错题重跑 8/31 解出（official-r1 此 31 题全 0）：query-optimize /
  pytorch-model-cli / largest-eigenval / caffe-cifar-10 / circuit-fibsqrt /
  mteb-retrieve / video-processing / build-pov-ray。c1/c2 同批对照旧 R2
  二进制 2/20 → 4/20（旧解 model-extraction-relu-logits / protein-assembly
  单样本回落 0，official-r1 亦为 0）。
- 三项修复验证通过：submit 门全链零 `no plan in force` 拒绝（7 解出题
  4 走两阶段 submit）；5 个硬门题全部越过 50 轮（chess 74 / make-doom
  103 / make-mips 75 / caffe 79 / gcode 52），caffe-cifar-10 因此解出；
  path-tracing 55 步无 orientation 循环、真实 HTTP 400 零。AgentTimeout
  均为 harbor 按官方 `task.toml` 的 `[agent] timeout_sec` 掐的墙钟超时，
  非 orz 检索超时。
- **新暴露问题 A（用户裁决 2026-08-29；S5-1 已实施修复）**：fold 桥剥
  `reasoning_content`——`build_bridge` 对桥内每条消息置
  `m.reasoning_content = None`（2026-08-19「思维链不进桥」设计），
  DeepSeek 对非空 reasoning 的 assistant 消息要求原样回传；make-doom
  （103 步）与 gcode-to-text（52 步）均死在「orientation 纯文本回答 →
  下一 loop-top 立即 fold → 回答落入桥内末条被剥 → 400」。同 run 第 50 轮
  纯工具轮 fold 与 video-processing（回答后继续工具轮、回答被折进台账）
  均不触发——机制边界已实证：桥内**纯文本** assistant 消息 + 非空
  reasoning 为必要条件。定案修复方向：桥内保留纯文本 assistant 消息的
  reasoning_content（或整体不剥），补「桥含纯文本+非空 reasoning」回归
  测试。**S5-1 已实施（2026-08-29，orz ad5f9ee）**：按前者落地（桥内
  纯文本 assistant 消息保留 reasoning_content、声明消息仍剥）+ 双形态
  回归测试（`bridge_keeps_plain_text_assistant_reasoning_but_strips_declarations`）；
  审查处理记录声明消息边界认知（声明 reasoning 恒空依赖实机证据）与
  ADR-0010 §14.42 修订。
- **新暴露问题 B（用户裁决 2026-08-29；S5-1 已实施修复）**：见 §9.2 偏差记录。
  （S5-1 已实施，见 §9.2。）

### 9.5 S4 失败画像（2026-08-29 深挖，23 个失败题分类）

- **A 框架错误终止（2）**：make-doom-for-mips（103 步）、gcode-to-text
  （52 步）——均为问题 A（fold 桥剥 reasoning_content）导致的
  NonZeroAgentExitCodeError；A/B 修复后可再跑。
- **B AgentTimeout 墙钟超时（18）**——占失败主体；官方 `task.toml`
  `[agent] timeout_sec` 900–3600s，harbor `asyncio.wait_for` 掐断，
  与 orz 检索超时无关。子类：
  - **B1 web 检索时间黑洞（12/18）**：单题最慢调用为 web 类工具
    （web_search 50–1365s / web_fetch 最高 677s）；`web_search`
    reqwest 客户端**未配置超时**（client.rs builder 无 `.timeout()`），
    DeepSeek `/responses` 生成式搜索单次可挂 22.7 分钟（mteb-leaderboard
    1365s；path-tracing-reverse 828s；rstan-to-pystan 885s；path-tracing
    653s；protein-assembly 648s；torch-pipeline 533s 等）。web_fetch
    默认 60s 超时，慢调用多为重试/下载处理。
  - **B2 run_terminal 慢命令（6/18）**：adaptive-rejection-sampler
    `apt-get install r-base-core r-base-dev` 458s（任务环境缺 R，R2 旧
    批次 pytorch-model-cli libGL 缺失同族）；train-fasttext 训练命令
    1000s；extract-moves-from-video 视频帧提取 616s；extract-elf 末尾
    `grep -rl ... /` 全盘搜索卡死 14.5 分钟（命令选择低效）；gpt2-codegolf
    53s；chess-best-move 11s。
- **C 自然结束交付失败（3）**——真实模型质量差距：model-extraction-
  relu-logits（19 步，恢复 20×10 矩阵 vs verifier 期望 30×10，隐藏层
  规模猜错）；dna-insert（23 步，primers.fasta 引物长度/插入序列校验
  失败，Tm 差 7.09>5）；filter-js-from-html（37 步，XSS 漏拦 2 例 +
  5/12 干净 HTML 被改写，双挂）。
- **结论**：A/B 修复直接救 2 题；B1 指向机械层（半助理层工具效率）——
  web_search 无超时是 12 个超时题的主要时间黑洞，属 R2「工具效率/自
  解释契约」方向待补（2026-08-29 已定案，见 §9.6）；B2 含环境依赖与
  命令选择问题（半助理层失败诊断可提示 apt/长命令形态）；C 为模型真实
  能力边界，暂不属框架缺陷。

### 9.6 超时机制定案（2026-08-29 用户裁决，登记先行、实施分批）

> 输入：调研笔记
> [`COMMAND_TIMEOUT_AND_WEB_SEARCH_TIMEOUT_RESEARCH_2026-08-29.md`](COMMAND_TIMEOUT_AND_WEB_SEARCH_TIMEOUT_RESEARCH_2026-08-29.md)；
> 背景：§9.5 B1/B2 时间黑洞——web_search 客户端无超时单次最高 1365s，
> run_terminal 慢命令 458s–14.5min。

- **终端命令分层默认超时（两档，用户裁决）**：普通命令默认 300s；
  程序/脚本类默认 600s。模型可传 `timeout` 覆盖，上限维持 900s（现
  `max_timeout_secs`）。「程序/脚本 vs 普通」的机械判定规则待 S5-2
  设计小节定案（命令形态启发式 + 模型显式标记，或先统一启发式）。
- **5 分钟中间回报（用户裁决）**：命令运行满 300s 未完成 → 机械插入一条
  「运行 + 工具自身情况」中间状态（运行时长/进程状态/输出活跃度/落盘
  指针），单次仅一次（不累积、不周期）；回报后**默认继续等待**，模型可
  主动中断（kill），由模型自行判断；命令完成/超时后正常返回终态。
  - 机制要点：当前 `run_terminal_cmd` 前台阻塞执行、`enabled_background`
    禁用；中间回报需要后台/部分结果注入路径（mid-run 模型可见消息 +
    中断 + 终态送达），属新机制，S5-2 先落机制定案再实施。
- **web_search 超时（用户裁决 120s）**：客户端总超时 120s + connect 10s
  （`web_search/client.rs` reqwest builder 补 `.timeout()`）；超时返回
  结构化错误（已用时长 + 建议重试/换查询/直读页面），不新增自动重试。
- **事件面**：`tool_completed` 补 wall_ms 与超时标记——**S5-1 已落**
  （2026-08-29：schema 增 wall_ms/timed_out；通用工具路径与 run_tests
  完成均带 wall_ms，超时落 `timed_out: true`；审查处理 P2-1 补 run_tests
  F-09 超时的结构化 `TestRunResult.timed_out` 端到端透传）；中间回报
  对应事件（如 `tool_running`）随 S5-2 机制定案一并落 Schema/verifier/
  fixtures。
- **实施批次（压力评估定案，分两批）**：
  - **S5-1（轻量防御性修复，同批）**：问题 A（fold 桥保留纯文本
    assistant reasoning_content）+ 问题 B（orientation 触发轮放行
    工具）+ web_search 120s 超时 + 事件面 wall_ms/timed_out（审查
    处理纳入）；S1 代码 → S2 测试 → S3 重建已完成（2026-08-29，
    orz ad5f9ee，musl 三件套）；**S4 复验批次作废重排**（2026-08-29
    用户指示：不再沿用原四题补跑批次，全部处理完成后重新安排）。
  - **S5-2（新机制，独立批）**：终端分层超时 + 中间回报；先补本小节
    的机制定案（分类规则 / 后台或部分结果注入路径 / 事件面），再
    S1–S4 走完整批次。

### 9.7 S5-2 机制定案（2026-08-29 设计小节，S1 代码 + S2 测试实施）

> 输入：§9.6 定案 + 调研笔记 §5；机制要点：复用既有终端 actor 的
> 自动后台化（auto-background）路径做「部分结果注入」，不新增唤醒/
> 续跑机制（新增机制一律视为债务）。模型工具面保持「一次调用 =
> 一个结果」：显式后台化（`is_background` / `&`）仍不开放。
> **实施状态（2026-08-29）**：S1 代码 + S2 测试已完成（宿主分类注入 /
> auto-bg 中间状态 / actor 原超时截止 / `tool_running` 事件面），
> S3 重建 / S4 复验待续。**全面审查处理（2026-08-29）**：console 订单
> 执行边界与机械审计层「仍在运行 ≠ 失败」补齐（P1-1/P1-2）；用户主动
> 后台化（Ctrl+G / 显式 is_background）恢复 10h 硬上限截止、自动后台化
> 保留原超时（P2-2）；run 结束未决后台任务语义定案（P2-1，见 §9.7.2）；
> 事件面校验器加固（direct 关联、running:true ⟹ exit_code=null、单完成
> 事件、同 run 限定，P2-3/P2-4/P3-4）+ 文档边界（P2-5/P3-5）+ console
> 订单面工具面封闭补齐（P1-3：`workspace.run_terminal` 移除 is_background、
> timeout 上限 900s / 默认 600s 对齐 host 工具面）。

#### 9.7.1 程序/脚本 vs 普通 分类规则（机械启发式，先统一启发式）

- 两档默认超时由**宿主（半助理层）按命令形态逐调用判定**并注入
  `timeout`（毫秒）：普通命令 300_000；程序/脚本类 600_000。
  模型显式传入 `timeout` 时以模型为准（上限维持 900s，
  `max_timeout_secs` 不变）。注入只发生在模型未传 timeout 时。
- **程序/脚本判定（纯函数，逐 token 归一化）**：命令首 token
  （去引号/去 `./` 前缀、小写）命中解释器/包管理/构建工具集合
  （python/python3/py/node/npm/npx/yarn/pnpm/ruby/perl/php/go/
  cargo/rustc/make/cmake/ninja/gradle/mvn/java/javac/gcc/g++/
  clang/apt/apt-get/pip/pip3/docker/bash/sh/zsh/pwsh/powershell/
  cmd/pytest），或任一 token 以脚本扩展名结尾（`.py/.sh/.js/.ts/
  .rb/.pl/.php/.ps1/.bat/.cmd`），或首 token 以 `./` 开头（工作区
  脚本/可执行），判为程序/脚本；否则判为普通命令。
- 边界：误判由模型显式 `timeout` 覆盖（可低可高）；schema 静态默认
  仍显示工具层缺省（600s），逐类默认以宿主注入为准，属已知文档边界。
  另两条启发式边界（审查处理 P2-5，先统一启发式、接受误判由模型覆盖）：
  ① compound 命令（`cd dir && make` / `cd dir && npm test`）首 token
  非集合成员且无脚本扩展名 token → 判普通 300s（扩展名规则只能兜底带
  脚本文件名的形态）；② Windows `.\script` 靠扩展名规则兜底，但
  `.\build`（无扩展名可执行）漏判为普通命令。

#### 9.7.2 后台路径（部分结果注入 + 默认继续 + 可中断 + 终态送达）

- **触发点**：命令运行满 300s 且其解析超时 **> 300s**（即允许继续
  运行：程序/脚本默认 600s 或模型覆盖 >300s）→ 终端 actor 将命令
  **自动后台化**（`transition_to_background`），一次调用在此返回
  一条「运行 + 工具自身情况」中间状态；普通命令（解析超时 ≤300s）
  按旧语义在超时点树杀（`timed_out`），不产生中间回报——与「任何
  命令满 300s 未完成才回报」口径一致（能满 300s 未完成必是允许跑
  更久的命令）。
- **中间状态内容（模型可见，工具侧 `pre_formatted` 生成）**：运行
  时长 / 进程状态（PID、running）/ 输出活跃度（部分字节/总字节）/
  落盘指针（完整输出文件路径）/ 默认继续 + 可中断提示（按 PID 终止）
  / 终态送达说明（最终结果随下一次工具结果返回）。
- **默认继续**：命令转入后台继续运行，actor 以其**原解析超时**为
  截止（后台任务在 `min(原 timeout, BACKGROUND_MAX_RUNTIME)` 处
  超时树杀，超时后完成提醒标注 `terminated by signal timeout`）——
  两层默认超时在后台化后仍然生效。**审查处理（P2-2）**：截止分型——
  仅自动后台化（ForegroundTimeout）保留原解析超时；用户主动后台化
  （显式 `is_background` / Ctrl+G）维持 S5-2 之前的 10h 硬上限语义，
  不因本次改动顺带缩水。
- **run 结束语义（审查处理 P2-1 定案）**：run 结束（终答/失败）时未决
  后台任务**保持运行至其截止**，由宿主生命周期收口——CLI 逐 run 进程
  模型下进程退出即 OS/actor 收口（actor 关停 SIGKILL 全部），TUI 退出
  走既有 `kill_all`；同一进程内多 run 复用宿主时，未决任务跨 run 存活
  至截止属显式边界（不新增 run 结束 kill 机制，遵守「新增机制一律视为
  债务」；如未来出现进程内多 run 长驻宿主，再按 owner 收口）。
- **可中断**：中间状态携带 PID，模型用普通命令终止（taskkill /
  kill）；actor 观察到进程退出后，完成提醒按实际退出状态返回终态。
- **终态送达**：复用既有 `TaskCompletionReminder`——后台任务完成
  后，下一次任意工具结果顶部 `<system-reminder>` 携带最终
  退出码/信号/时长/输出指针（既有机制，不新增唤醒）。**边界（审查
  处理 P3-5）**：若模型在中间回报后不再发起任何工具调用（直接终答/
  run 结束），终态提醒随无后续工具结果而不会送达——模型已被告知
  「最终结果随下一次工具结果返回」，且 run 结束语义见上（任务按截止
  收口），属可接受边界。
- **工具面封闭**：ORZ `run_terminal_cmd` 参数置
  `enabled_background=true`（actor 自动后台化需要）但
  `allow_background_operator=false` 且新增 `hide_background_input`
  （schema 移除 `is_background`、调用显式拒绝）——模型侧看不到
  后台工具面，只能看到一次调用返回的中间状态与后续终态。console 订单面
  （`workspace.run_terminal` 动作 schema）同口径封闭：不暴露
  `is_background`，timeout 上限 900s / 默认 600s 与 host 工具面一致
  （审查处理 P1-3）。

#### 9.7.3 事件面（`tool_running`，v0.2 track）

- 中间回报发生时，控制器在 `ToolStarted` 与 `ToolCompleted` 之间
  记一条 **`tool_running`**（v0.2 事件类型 + 独立 payload schema）：
  `{tool, call_id, wall_ms（运行时长）, pid, total_bytes（输出活跃度）,
  output_file（落盘指针）, task_id}`；随后该调用照常收 `ToolCompleted`
  （exit_code=null + `running: true` 标记，表示命令仍在后台运行）。
- 链规则（verifier 强制）：`tool_running` 必须位于同一 run 内
  相同 `tool`/`call_id` 的 `tool_started` 之后、`tool_completed` 之前；
  同一 `call_id` 至多一条（单次仅一次）。
- 边界：后台任务的**最终状态**经完成提醒送达模型（模型可见），
  其 journal 侧的终态落账留待后续批次（本批不引入第二条
  ToolCompleted 破坏既有「一次调用一条完成事件」链规则）。
