# 0di 设计稿：消息撤回＋中途补充——Claude Code 形态移植（MESSAGE-RECALL-AND-QUEUED-SUPPLEMENT）

> 版本：v1.1（2026-10-18，305 批 0dj 审查处置；v1.0＝S1 勘定＋S2 落码同批）。
> 立项：296 批（2026-10-17，64 → 65）；调研底稿＝295 批档 §2–§3；真机动因＝293 批 N3
> （web `submitPrompt` 运行中直接拒绝，无排队；无任何撤回入口）。
> 用户已裁决（296 批）：①撤回与中途补充**完全借鉴 Claude Code**；②撤回**不连带工作区文件**，
> 文件面 git 兜底（不建文件 checkpoint，沿 CC「bash 副作用不可回退」诚实边界）。
> 本稿是 0di 的设计权威；ADR-0010 §14.88（v1.91）为本稿转录。

## §1 勘定结论（S1 读码定案，六项）

| # | 待勘项 | 定案 |
|---|---|---|
| 1 | 排队归属 | **agent 侧会话级队列**（`AcpServer.StoredSession.supplement_queue: Arc<SupplementQueue>`，`build_host` 注入 `OrzHost`——0dh S2-R terminal backend／browser `SharedBrowser` 同先例，跨 host 每-prompt 重建存活）。web 前端/TUI 只是投递通道；桥（orz-web pump）是透明 1:1 中继，零改动 |
| 2 | flush 点 | `run_agent_loop` **主车道 loop-top**（压缩/检查点块之后、`request_messages` 装配之前）drain，FIFO 追加为 `Role::User` 消息——恰是「下一次 LLM 调用边界」（CC #50246 deferred 语义）。检索/grill 车道不 drain |
| 3 | ACP 并发 prompt 语义 | 现状＝客户端各自 running 守卫（web `state.running`／TUI `app.running`）＋服务端 `runs` 表 overwrite 语义；补充**不走 `session/prompt`**（零并发 run 风险），走新 ext 方法。`ext_method` 请求在 agent 侧与在途 run 并发处理（acp 连接每请求独立 task），运行中投递可达 |
| 4 | 排队可见面 | 投递回执：queue_append 响应带 `depth`；投递事实：`supplement_delivered` journal 事件→web/TUI 投影渲染用户卡（用户可见「补充已生效」）；客户端发送时系统行提示 |
| 5 | 撤回入口 web+TUI | web＝`/rewind` 命令→对话框（`_orz/recall_preview` 列表→选择→确认→原文回输入框）；TUI＝`/rewind` 命令→Snapshots 式 modal（列表+确认→原文回 textarea）。不做消息卡逐卡按钮（journal 卡↔侧车 ordinal 映射脆弱，菜单制=CC 同形） |
| 6 | 0dd 衔接 | 撤回＝0dd 会话生命周期用户主权的**内容面**操作；run／restore／recall 在单进程会话内共用 `runs` 表互斥；`close_session` 语义零改动（close×recall 归档竞态登记为正常 UI 拓扑不可达的 P3 边界） |

## §2 设计裁决（D1–D8）

### D1 中途补充——通道与队列（子件①）

- **通道**＝ACP 扩展方法 **`_orz/queue_append`**（client→agent 请求；线上 `method:"_orz/queue_append"`，
  acp 库剥 `_` 前缀路由为 `ExtRequest{method:"orz/queue_append"}`；params `{session_id, text}`；
  响应 `{queued:true, depth:n}`）。**零 8 工具面触碰**（ext_method 是 client↔agent 协议扩展面，
  非模型工具面；acp 库 `ExtRequest/ExtResponse/ExtNotification` 逃生阀现役）。
- **队列语义**：Prompt 运行中或空闲可投（运行中＝本 run 可用的下一次模型调用边界投递；空闲＝**跨 run 存活**，
  下一 run 首 loop-top 投递）；cancel **不清队**；上限＝每条 ≤256 KiB、深度 ≤16，超出
  fail-closed `InvalidRequest`（不静默截断）。多条按 FIFO 保序。restore／recall 在途时入队与其
  共用 `runs` 锁原子拒绝，避免 push-after-clear 落入撤回后的分支。
- **不做**（295/296 两轮已定）：Esc 硬打断／Send-Now／mid-turn steering／Cursor 切走式——
  零打断通道，取消仍走现役 `session/cancel`（Ctrl+Z）。

### D2 flush 点与投递语义

- drain 点＝主车道 loop-top 安全间隙（绝不在工具批内；压缩/检查点算术之后追加，补充永不进入
  压缩区间）；每条投递**先记 journal 事件再入 messages**（事件先于当轮 `model_request`）。
- **失败/取消 run**：已投递补充随 run 消息面一并丢弃（SUCCESS-ONLY 写回语义——与 run 内模型
  输出/工具产物同命运；`supplement_delivered` 事件留证于该 run journal）；web/TUI 在非成功终态
  明确提示「本次运行已投递的 N 条补充未写回会话，请按需重新发送」。
- **投递可见面**：journal 事件 → web `projection.js`／TUI `projection.rs` 各加一 case 渲染用户卡。

### D3 `supplement_delivered` 事件族

- 新 `EventType::SupplementDelivered`（非终态；v0.2 track）。payload 契约：
  `{supplement_seq:u32≥1（run 内从 1 严格递增）, character_count:>0（=text chars）, text:非空}`。
- **姿态＝prompt_submitted 同族**（用户输入事实，非机制事件）：registry v02 条目＋payload
  schema 文件＋fixtures 三件（minimal.valid／constraint.invalid／envelope.valid）＋v02 enum；
  **法官族/Python 镜像新族/锚不加**（不变量由 loop 单元钉承载；conformance 的 registry 解析
  为硬门，事件必须注册——Python 判官按 registry 派生视图，未注册即 `unknown event_type`）。
- 历史刊零回归：事件只在有补充时出现，无锚、无 grandfather 需求；既有序列钉不受影响
  （无补充的 run 不产出该事件）。

### D4 撤回——截断语义与边界（子件②）

- **语义**＝CC `/rewind` 编辑式重发：恢复会话到目标用户消息**之前**（该条含其自身及之后全部
  移除）＋原文回输入框供编辑重发。目标＝**真实用户消息 ordinal**（会话续接包 `messages` 中
  `Role::User` 且排除已登记机械注入块与 `[任务状态` 尾随块后的 1-based 序数；侧车是唯一真相面——
  web/TUI 的 journal 卡只作历史证据，不作截断索引）。
- **门**＝idle-only：`RestoreInflightGuard` 同锁互斥（run／restore／recall 三方互斥；recall 期间
  prompt/restore 一律拒绝，反之亦然）。
- **落点**：`StoredSession.continuation` 截断候选（`None` 时沿 next-prompt take-out 同逻辑
  `load_conversation_sidecar` 回退）→ `persist_conversation_sidecar` **成功后**清补充队列并写回
  `session.continuation`；落盘失败返回错误并原样归还会话与队列，禁止假成功和崩溃后尾巴复活。
- **排队残余**：recall 时未投递补充**一并丢弃**（`dropped_supplements` 计数返回＋客户端提示）
  ——撤回重开的分支不得注入陈旧意图。
- **不回滚面（诚实边界，登记）**：①工作区文件（用户裁决，git 兜底）；②LIF 会话轴／黑板
  live 视图／负担与水位档位（**机械观测面单调不回退**——黑板全量记录纪律 P2-13＋轮计数
  已被 journal 锁定；撤回后模型仍可见撤回段的黑板内容，S4 真机观察体感，若混淆再议黑板
  epoch 标记）；③run 历史/journal（evidence chain 不可变；web 视图旧卡保持为历史证据，
  撤回以系统行提示）。
- **压缩/存档对账**：撤回段内的压缩／清零／白名单 marker 随消息一并丢弃；留在目标之前的
  restore-retained marker 保持原位（菜单已过滤，不会回填为用户原文）；`.gsa/archives` 归档包与三键互标**不动**（归档是历史工件，
  指针仍有效）。
- **侧车审计**：`StoredConversation.recall_log: Vec<RecallRecord>`（serde default 零迁移）＝
  `{recalled_at, ordinal, removed_messages, remaining_messages, dropped_supplements}`——审计随工件
  （会话卷是 local session artifact、非 run evidence chain，无 run journal 可落；restore 先例
  有 RST run 因其触工作区，recall 只触侧车，不建 run）。

### D5 撤回通道与入口

- 通道＝`_orz/recall_preview`（响应 `{messages:[{ordinal, character_count, preview≤120 chars}]}`，
  服务端按旧→新返回；TUI 默认首条、web 默认末条，均可显式选择）＋ `_orz/recall`（params `{session_id, ordinal}`；响应
  `{removed_messages, recall_text, remaining_messages, dropped_supplements, recalled:true}`）。
- 入口：web `/rewind`（`runSlash` 分派→dialogs 新对话框：列表/选择/确认→`recall_text` 填入
  输入框并 focus＋系统行）；TUI `/rewind`（commands.rs 注册→modals.rs 新 `Modal::Rewind`
  （Snapshots 同构：列表+confirm 两段）→app.rs confirm 臂→文本回 textarea）。运行中两入口
  均被服务端 idle 门拒绝并如实回显；web 另有运行态前端守卫，服务端仍为最终权威。

### D6 ext_method 接线

- agent 侧：`StdioAgentHandler::handle_request` 增 `ClientRequest::ExtMethodRequest` 臂 →
  `AcpServer` 三 handler（`handle_queue_append`／`handle_recall_preview`／`handle_recall`）→
  `AgentResponse::ExtMethodResponse`；未知 ext 方法＝method_not_found（现役语义）。
- 客户端：TUI `InProcessClient` 三方法（`ClientSideConnection::ext_method` 出站）；web
  `acp.js` 三函数（raw JSON-RPC，`_orz/*`）。
- 会话校验统一：未知 session＝`SessionNotFound`（现役错误面）。

### D7 契约与兼容

- 零模型面工具改动、零 8 工具面触碰、零 schema 破坏性变更；
  `StoredConversation`+`recall_log`（serde default 向后兼容）；`SupplementQueue` 新类型
  零外部契约；journal 事件族 +1；`LoopHost`+`drain_user_supplements()`（default 空——
  一次性车道/测试宿主零行为变化）。
- 队列注入：`OrzHost` 增 `supplement_queue: Option<Arc<SupplementQueue>>` ＋ setter
  （不扩 `too_many_arguments` 构造链）；`build_host` 从 `StoredSession` 查表注入。

### D8 批序与 S4 判据

- 批序：~~S1 设计稿~~→~~S2 落码＋钉子~~→~~0dj 审查处置（305 批）~~→S3 载体重建
  （**与 0dh S3 同车**，字节判据随 S2 落码批定）→S4 真机。
- S3 字节判据（随 S2 定）：`_orz/queue_append`/`_orz/recall_preview`/`_orz/recall` 字面进体；
  `supplement_delivered` 进 v02 enum 与 registry；`recall_log`/`SupplementDelivered`/
  `drain_user_supplements` 契约面字面。
- S4 真机判据：①补充＝运行中发送不打断当前轮、下一轮模型请求含补充（卡片＋journal 事件
  先于当轮 model_request 可证）；②撤回＝截断后下一 prompt 上下文不含被撤回段（模型侧＋
  侧车一致）、原文回输入框可编辑重发；③撤回不回滚文件面/机械面为预期行为确认；
  ④ext 三方法 web+TUI 双面可用；⑤双入口并发与 preview idle 门；⑥失败/取消 run 的补充回滚提示；
  ⑦撤回×排队竞态无陈旧意图残留；⑧菜单不出现机械块；⑨满队极端量下失败提示与恢复动作明确。

## §3 钉子清单（S2）

1. **orz-host**：queue_append 深度/FIFO/单条字节帽/上限 fail-closed；ext 未知方法 method_not_found；
   recall 截断＋侧车落盘＋recall_text；recall×run 互斥（运行中拒绝）；recall×restore 互斥；
   queue_append×restore/recall 互斥；失败 ordinal 不清队；落盘失败会话与队列回滚；机械块过滤；
   recall 丢排队残余计数；recall_log 落盘；continuation None 回退侧车臂。
2. **orz-loop**：loop-top drain 主车道专属（检索车道不 drain）；投递序＝事件先于当轮
   model_request；多条 FIFO 保序；跨 run 残余下一 run 首投递；supplement_seq 每 run 从 1 重计。
3. **orz-assurance**：schema/registry/fixtures 三件对拍（payload valid/invalid＋envelope）。
4. **orz-tui**：`/rewind` modal 两段确认；运行中 `/rewind`/投递拒绝回显；queue_append
   运行中接线；SupplementDelivered 投影用户卡。
5. **orz-web**：frontend_smoke 新节——running 时 submitPrompt 走 queue_append（不发
   session/prompt）；/rewind 对话框接线与原文回填；supplement_delivered 卡片渲染。

## §4 边界与不取项

1. 不做文件 checkpoint（用户裁决）；不做消息卡逐卡撤回按钮（菜单制）；不做打断式补充。
2. 桥（orz-web Rust 面）零改动；8 工具面零改动；模型面提示词零改动（补充是普通 User
   消息，模型无感知投递机制）。
3. stdio 外部客户端（非 orz 前端）直接调 `_orz/*` 属协议面允许，无额外承诺。
4. grill 车道不 drain 队列；grill 运行不登记 `runs`，因此 raw ext recall 并发态只在单进程内部存在、
   正常 TUI 内联输入拓扑不可达；TUI 分派顺序固定先认 grill 再认 running，防未来异步化误排队。
5. 队列跨 run 存活是**会话域**语义：close_session／进程退出即灭（与 terminal backend
   会话域锚点一致）；不承诺跨进程重启。
6. 每条 256 KiB／深度 16 是**传输容量上限，不是上下文预算保证**；补充在压缩算术后投递以保持
   loop-top/前缀缓存纪律，故不在本批移动 drain 或二次压缩。极端满队可能令提供方拒绝本轮；
   `supplement_delivered` journal＋双前端失败提示为恢复面，S4 用极端量样本核证后再决定是否收窄容量。
7. stdio ext 错误继续沿现役 `internal_error + data` 线格式；`recall_text` 覆盖编辑框草稿沿 Claude Code
   同形保留；close×recall 归档竞态和跨进程同会话互斥均属正常产品拓扑不可达边界，不新增机制。
