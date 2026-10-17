# 0df S1 设计稿：任务级计费机械记录点——`run_usage_metered` 事件族（PER-TASK-METERING，2026-10-17）

> 性质：**S1 设计勘定批**（284 批立项档 §4 五项待勘定在本稿全部定案；零源码、零跑批）。
> 设计权威入 ADR-0010 §14.86（v1.89，随本批转录）。**S2 落码与 S1 同批执行**（用户令
> 「请先进行0df吧」；沿 0cl／0ct 先例），落码面见设计稿 §5 与 [`287 批档`](audits/287_0DF_S1_S2_RUN_USAGE_METERED_2026-10-17.md)。
> 用户令（2026-10-16）：「我打算给下一个版本的 orz 配个任务级计费，跟着会话一起存档。」
> ＋「请将这一内容记为待办吧，作为新的机械记录点。」＋（2026-10-17 追加）「把配置侧的
> reasoning effort 档位写进任务行——与观测到的 reasoning token 分列」。

## §1 定位与动因（284 批档 §3 摘录）

- 283 批的单任务成功成本全部是**事后只读复算**（逐题余额差分＋10 分钟粒度余额序列
  对齐）；TB 2.1 半边因**缺机械记录**金额面与 token 面双双不可得（`result.json` usage
  逐件 null＋该窗无余额台账）。本项把「事后对齐」变成「记录自带」：每个 run 的
  token 消耗在 run 终态由机械层写一条计量行，随会话存档同走，单任务归因不再依赖
  人手把台账落到题目粒度。

## §2 事实面勘察（2026-10-17 只读源码勘察，行号为当日 orz `b94f9fb7`）

1. **模型调用统一收口**：主车道与检索子代理走同一 `run_agent_loop`
   （`retrieval/dispatch.rs:454` 传 `self.writer`——子代理轮写**主 run journal**）；
   `RoundAgent` trait（`agents/main.rs:84`、`agents/retrieval.rs:463`）统一转发
   gateway。⇒ **一个 run 一本 journal 覆盖全部车道**的模型轮。
2. **usage 读数面**：`ModelResponse`（`gateway/model.rs:236-266`）携带五项——
   `prompt_tokens`／`cache_hit_tokens`／`cache_miss_tokens`／`completion_tokens`／
   `reasoning_tokens`（前四 `Option<u64>`、completion/reasoning `Option<u32>`）。
   journal 的 `model_output` 事件（`agent_loop.rs:3464-3497`）只落四项、**缺
   `prompt_tokens`**（A6 压缩触发内存消费）；流中断臂（3398-3416）全 null ＋
   `incomplete: true`。⇒ 折算必须在**内存 ModelResponse 到达点**做，不能事后从
   journal 重放（会丢 prompt 口径）。
3. **cost 缺位**：DeepSeek transport 全文无 cost 解析（`gateway/transport.rs` 零
   命中）；fork 的 `CompletionUsage` 丢弃未知字段（provider 即使多传 cost 也收不到）。
   `cost_usd_ticks` 管线（`orz-chat-state/src/usage.rs`）属**另一产品面**且
   `RecordModelCallUsage` 在 orz-workspace 内**零发送方**（只定义未接线）。
4. **effort 配置面**：`ThinkingMode {EnabledHigh, EnabledLow, EnabledMax(默认),
   Disabled}`（`gateway/model.rs:86`）→ `ModelConfig.thinking`（:119）→ env
   `ORZ_THINKING_MODE`（`transport.rs:1042`，词汇 max|high|low|disabled）；
   `apply_thinking` 双写 wire `thinking`＋`reasoning_effort`（:1127）；请求头指纹含
   thinking 档（:1087）。**空响应链降级是 per-call 局部变量**（:2186 `thinking =
   next`，不改 `self.config`）⇒ run 尾读 config 即设计值；降级事实已由
   `request_header_change` 指纹面承载。
5. **run 终态臂**：`controller.rs`——completed（:4347 起）／Cancelled／Degeneration/
   Failed（:3436-3470）／journal 降级卷 RunTerminated（:4926 附近），统一
   `record_terminal`（:4429）。grill 模式 `EventWriter{journal: None}` 全事件不落链
   （:4697-4702）——计量经同一 writer 自动随 grill 丢弃，无需专门处理。
6. **存档打包**：`acp_server.rs` `collect_session_footprint`（:1245）按
   `collect_run_dir`（:1233）**整目录**收集 `.gsa/runs/`（RUN-／RST-／RUN-CLI-）⇒
   journal 事件天然随 0dd 手动存档／0cy 收卷同走，**存档面零新增**。
7. **事件注册同改面（face_fingerprint 先例，orz `7a6fe91e`）**：EventType 变体
   （`orz-assurance/src/journal/event.rs`）→ 父仓 `runtime/run-event-v0.2.schema.json`
   enum → `runtime/run-event-payload-registry-v0.1.json` 两 tracks → 新 payload
   schema 文件 → Python 冻结镜像 `_verify_v02_*`（`assurance/run_event_journal_
   validation.py`，registry 视图自动派生不许手改）→ 法官族＋ALL_FAMILIES →
   envelope/payload fixtures → TUI bridge 降级 Unknown（`bridge.rs:505` 先例姿态）。
   web 投影 `default` 臂（`projection.js:302`）优雅跳过未识别事件 ⇒ **UI 零改动**。
8. **journal 不可改写**：journal 哈希链（seq/prev_hash 连续，EventWriter :4717-4722）
   ⇒ 事后回填**不得**改写历史 journal 行——金额归因只能走派生台账。
9. **grandfather 锚先例**：0q 失败管线在 `run_started` payload 加 optional
   `failure_pipeline: "funnel-v1"`，法官族仅对带锚 run 执法——旧刊零回归。

## §3 定案（D1–D8）

### D1 记录载体＝独立 EventType `run_usage_metered`（否 mechanical_audit kind 扩档）

284 批档 §4-1 预设「mechanical_audit 新 kind（沿 0ae/0bg）」；勘察后**改判**独立
事件族，三理由：(a) mechanical_audit 语义是**轮级对象键覆盖写**（key/round/summary/
anomaly），本记录是 **run 级一次性终态行**——round 语义不合；(b) schema 明文
「主车道专属：检索子代理车道不产生本事件」，而计量必须折入子代理消耗，扩 kind 要
改写该约束；(c) mechanical_audit 行在 web/TUI 渲染 `[机械审查]` 卡（285 批 F3 幽灵行
前科），数字行混入审查卡是渲染负担。同改面沿 **face_fingerprint 先例**全量执行；
TUI 沿 `WriteControlReview`/`FaceFingerprint` 姿态降级 Unknown、web 走 default 臂，
两端零改动。

### D2 粒度＝一 run 一行（284 §4-3 勘定）

计量行在 **run 终态前**落一条（本 run journal 内）。单位＝run；SCB 形态题＝run 恰
等，TB 形态题↔run 映射由评测侧 harness 承担（机械行自带信封 run 身份＋session 关
联，映射所需键齐备）。**不做**题级聚合行——聚合是派生消费，不是记录点职责。子代
理（internal/external 检索车道）轮在 D6 折算点统一折入**同一行**（同 journal）。

### D3 payload v1 字段（`run-usage-metered-event-payload-v0.2.schema.json`）

```text
model              string   配置模型 id（整个 run 单一网关配置；子代理共用同网关）
model_calls        integer  折算轮数（ModelResponse 到达数；transport 内部 D-6 重试
                            属单轮内部细节，不折算计——重试面由 transport_retry 承载）
prompt_tokens      integer  逐轮饱和求和（直测总量；hit+miss 的上位口径）
cache_hit_tokens   integer  逐轮饱和求和
cache_miss_tokens  integer  逐轮饱和求和
completion_tokens  integer  逐轮饱和求和
reasoning_tokens   integer  逐轮饱和求和（观测值；与 effort_tier 分列——用户令）
effort_tier        enum     配置侧设计值：max|high|low|disabled|unknown
                            （gateway 配置单读；unknown=无配置暴露的 fake/网关）
usage_incomplete   boolean  任一折算轮五项有缺（流中断臂等）⇒ true；
                            非 completed 终态臂由控制器保守置 true
```

`additionalProperties: false`、全字段 required（数字 ≥0）。`prompt_tokens` 计入
（billing 口径直测值；283 分账的 hit/miss 上位总量）。

### D4 金额面＝journal 不记金额；回填走派生台账；折算值不入机械记录（284 §4-2 勘定）

- **in-band 无自足来源**：provider 不报 cost（§2-3）；余额差分（GET /user/balance
  前后分）**并发不安全**（重叠 run 互相污染差分）＋ TB 2.1 实证端点/台账不可用场景
  ＋每 run 增加计费 API 调用；provider 账单侧无 API 面。
- **journal 哈希链不可改写**（§2-8）⇒ 「回填金额进历史行」结构性不成立。金额归因
  ＝**run 行 token × 金额来源在派生台账配对**（283 的余额差分/账单导出方法学机械
  化；配对键＝信封 run 身份）。派生台账工具不入本项批序（另行立项）。
- **折算（rate card）不入机械记录**：价目是外部漂移知识，机械层记录陈旧折算值即
  「以折算冒称实付」的温床（283 纪律：实付与折算两栏分列）。未来 provider 若在
  usage 上报 cost，走 payload v0.3 加 optional 字段（本 schema 的演进路径），届时
  与实付同栏核证。

### D5 effort 档位设计值＝gateway 配置单读（2026-10-17 用户令）

`ModelGateway` trait 新增 `thinking_tier_label(&self) -> &str`（trait default 返回
`"unknown"`；DeepSeek transport 返回 `self.config.thinking` 标签 max/high/low/
disabled；测试 fake 可覆写）。run 尾读值即设计值（降级链是 per-call 局部量，§2-4，
不改 config）；降级事实由 `request_header_change` 指纹面承载，计量行**不重复**记
降级。**与观测 `reasoning_tokens` 分列**——r4 实证档位恒定 ≠ 推理量恒定（全轮一档
下推理占 completion 76%、逐题 28%–90%），两值并存各有口径。〔288 批勘注：trait
形态实落为 `usage_meter_labels(&self) -> (String, String)`（model id＋tier 一次
单读；default `("unknown", "unknown")`），语义同本节，见 §4 勘注①〕

### D6 折算点＝`run_agent_loop` 的 ModelResponse 到达点；宿主 `SharedLoopServices`

- 折算器 `RunUsageMeter`（五项求和＋`model_calls`＋`incomplete` 旗）挂
  `SharedLoopServices` 新字段 `usage_meter: &'a Mutex<RunUsageMeter>`（非
  Option——两车道都要折；`retrieval_calls` Option 先例是车道特有语义，计量不是）。
- 折算在 ModelOutput 记录点旁（`agent_loop.rs:3464` 前后）：响应到达即折
  （五项 Option 逐项 fold、有 None 置 incomplete）；流中断臂（:3398）置 incomplete
  不折数字。
- **per-run 隔离**：controller 在 `run_turn` 起点新建 meter（沿
  `for_new_run()` 每 run 换新的隔离先例）；dispatch 构造子代理 svc 时透传同一
  引用（`dispatch.rs:439/:562` 两处构造点）⇒ 主＋双子代理轮折入同行。
- 机械模板压缩零模型调用（`CompactDecision::Executed` 机械模式），无 usage 可折、
  不入计量；模型参与压缩在主循环轮内，自然折入。

### D7 写点＝五终态臂 `record_terminal` 前恰一条

controller 五终态臂（completed／cancelled／invalidated(degeneration)／failed／
terminated）在 `record_terminal` 之前经统一 helper 落一条
`run_usage_metered`（grill 模式随 `journal: None` 自动丢弃）。completed 臂按 meter
实况；四个非 completed 臂**保守置 `usage_incomplete=true`**（终态前在途轮的消耗
不可知——如实登记不装完整）。事件序列钉值校准：controller
`run_turn_full_gate_sequence` 等期望序列补计量行（face_fingerprint 先例同操作）。

### D8 执法＝`usage_metering` 法官族＋`run_started` grandfather 锚

- `run_started` payload 增 optional 锚 `usage_metering: "run-v1"`（0q
  `failure_pipeline` 先例同构；仅新载体写入）⇒ 法官族只对带锚 run 执法，历史刊零
  回归。
- Rust 法官族 `usage_metering`（families.rs＋ALL_FAMILIES）：带锚 run 恰一条
  `run_usage_metered`、位于终态事件之前、字段形状/枚举/非负核对；
  Python 冻结镜像 `_verify_v02_run_usage_metered`＋`_verify_v02_run_started` 锚容忍
  同步（Rust↔Python 对拍表注册）。
- fixtures：envelope valid＋payload minimal.valid／constraint.invalid；conformance
  场景按 face_fingerprint 先例增补（corpus 钉值 +1）。〔288 批勘注：287 实落 +2＝
  `usage_metering_ok`／`_violations`（259→261）比本稿更全；288 处置再 +1＝
  `usage_metering_non_string_anchor`（261→262），见 §4 勘注②〕

## §4 边界与不取项

1. **零模型面改动**：机械记录只记不发——无注入、无新工具、无文案；web/TUI 零渲染
   改动（未知事件优雅降级为既证事实）。若未来需要模型可见的用量提醒，另行立项。
2. **mechanical_audit kind 不扩**（D1 三理由）；`ORZ_THINKING_MODE` 配置面不动；
   不做 rate-card/价格配置面（D4）。
3. **不做 in-band 余额查询**（D4）；**不做题级/会话级聚合行**（D2）——聚合与金额
   配对都是派生消费（283 方学名机械化时另行立项）。
4. 计量行不进审查报告块、不进黑板、不进任何模型面投影；`model_calls` 是 loop 轮
   数而非 HTTP 尝试数（transport 内部重试由 `transport_retry` 事件承载）。
5. 历史会话/旧载体的 run 无计量行（锚不存在 ⇒ 法官族不执法）——**如实缺口**，不
   追补。

> **勘注（2026-10-17，288 处置批）**：0df S1+S2 基线（287 批）全面审查处置六件——
> ① **D5 trait 形态**：实落 `usage_meter_labels(&self) -> (String, String)`
> （model id＋effort 档一次单读；default `("unknown", "unknown")`；DeepSeek 返回
> `config.model_id`＋thinking 档设计值）——本稿 D5 原拟 `thinking_tier_label->&str`
> 为形状草案，语义不变，原文保留于上。
> ② **D8 corpus 钉值**：287 实落 +2（259→261）比本稿「+1」更全；288 处置再 +1
> （`usage_metering_non_string_anchor`，261→262）。
> ③ **崩溃刊判定语义（如实登记，两侧同形）**：带锚 run 无终态事件（进程崩溃）——
> 崩溃点在计量行前 ⇒ 「恰一条」违例如实报（`got 0`）；崩溃点在计量行后、终态前 ⇒
> 无终态即无「终态前」义务，排序检查空转、恰一条满足 ⇒ 静默通过。
> ④ **降级刊区分文案**：锚已落盘后中途 journal 降级 ⇒ 计量行被降级写入器丢弃 ⇒
> `got 0` 属盘面如实；法官族对该形态附区分性注（不豁免执法；0q 锚定族同行为类；
> Rust↔Python 同形落盘）。
> ⑤ **恰一条生产侧封死**：`RunUsageMeter` 增 `journal_line_written` 幂等旗（成功
> record 后置位；record 失败不置位，Err 臂仍可重试）——completed 臂落行后
> `record_terminal` 罕见硬错上抛进 Err 臂不再补写第二条（「got 2」理论破口关闭）。
> ⑥ **非字符串锚对齐**：锚键存在但值非字符串（损坏形态）Rust 侧同 Python 走
> 「unknown version」错误（此前经 `str_of` 静默跳过）；缺键／显式 null 仍容忍。
> 判据读数：orz-loop 883/0（+1 幂等钉）＋orz-assurance 303/0（+2 直调钉；parity
> 262 场景×族全绿）＋tui 172/9（同集）＋bin 15/0＋web 47/0＋fmt 净＋clippy 触碰
> 面零新增＋门禁 valid: true。

## §5 S2 落码清单与判据（随后批执行）

| 面 | 落点 | 内容 |
|---|---|---|
| 事件族 | `orz-assurance/src/journal/event.rs` | `RunUsageMetered` 变体＋payload 契约注释 |
| 折算器 | `orz-loop/src/agent_loop.rs`（或新 `usage_meter.rs`） | `RunUsageMeter`＋fold 点＋svc 字段＋四构造点透传 |
| 档位单读 | `gateway/model.rs`＋`transport.rs` | trait `thinking_tier_label`＋DeepSeek 实现＋fake 覆写 |
| 写点 | `orz-loop/src/controller.rs` | 五终态臂统一 helper＋锚写入＋序列钉校准 |
| 法官族 | `orz-assurance/src/journal/families.rs` | `usage_metering` 族＋ALL_FAMILIES＋corpus |
| runtime 合约 | 父仓 `runtime/` | 信封 enum＋registry 两 tracks＋payload schema＋run_started 锚容忍＋fixtures |
| Python 镜像 | `assurance/run_event_journal_validation.py` | `_verify_v02_run_usage_metered`＋run_started 锚容忍＋对拍表 |
| 钉子 | 触碰 crate 测试 | fold 精度（五项/None→incomplete）、双车道折入同行、五终态臂恰一条、grill 零落链、锚 grandfather、schema↔常量互证 |

判据：orz-loop／orz-assurance／orz-tui lib 全绿（tui 同一 ACAF 预存集合例外）＋
orz-bin bins 绿＋Rust↔Python parity 绿＋conformance fixtures 过＋clippy/fmt 触碰面
零新增＋门禁 `valid: true`。S3 载体重建（字节判据＝`run_usage_metered` 字面＋锚
键）与 S4 真机读数（一行一 run、五项与 provider 账面可对、子代理折入）随后批。

## §6 台账

- 本稿；ADR-0010 §14.86（v1.89）随批转录；BACKLOG `0df` 节勘注（S1 达成＋五项
  定案指针）；287 批档；索引/TODO 治理行随批同步。
- 零源码、零 `orz/` 子树改动；门禁 `python scripts/check_repository.py` ⇒ 目标
  `valid: true`。

## §7 关键词

0df S1、任务级计费、机械记录点、run_usage_metered、事件族、face_fingerprint 先例、
折算器、SharedLoopServices 宿主、双车道统一折算、prompt_tokens 补账、effort_tier
设计值、reasoning_tokens 分列、金额不入 journal、哈希链不可改写、派生台账回填、
grandfather 锚 run-v1、usage_metering 法官族、grill 丢弃、287 批。
