# 302 批：0di 消息撤回＋中途补充——Claude Code 形态移植（S1 勘定＋S2 落码同批）（2026-10-18）

> 性质：**落码批**（S1 设计稿＋S2 落码＋钉子同批，0df/0cl 先例；用户令「请先进行0di吧，
> 把当前应该做的做完了再进重建，这样省事」——0di 落码完成后下一批进 0dh S3 载体重建，
> 0dg＋0di 两批改动同车进体）。计数 65 不变（0di 两子件落码达成＝闭合待用户裁决）。
> 设计权威：[`0DI_RECALL_SUPPLEMENT_DESIGN_2026-10-18`](../0DI_RECALL_SUPPLEMENT_DESIGN_2026-10-18.md)
> （D1–D8）；ADR-0010 **§14.88（v1.91）** 转录随批落卷。orz `01386744`
> （24 文件 +1861/−9，feat/fusion-architecture，**不推送**）。

## §1 S1 勘定（六项定案，读码实证）

| # | 待勘项 | 定案（代码实证） |
|---|---|---|
| 1 | 排队归属 | **agent 侧会话级队列**——桥（orz-web pump）是透明 1:1 中继（acp_pump.rs），前端无法注入在途 run；flush 点在 loop 内部 ⇒ 队列必须住 agent。落点＝`StoredSession.supplement_queue: Arc<SupplementQueue>`，`build_host` 注入（0dh S2-R terminal backend／browser `SharedBrowser` 会话域同先例，跨 host 每-prompt 重建存活） |
| 2 | flush 点 | `run_agent_loop` 主车道 loop-top（压缩/检查点算术之后、`sync_status_line_message`/模型面装配之前）drain——恰是「下一次 LLM 调用边界」（CC #50246 deferred 语义；绝不在工具批内打断） |
| 3 | ACP 并发 prompt 语义 | 现状＝客户端各自 running 守卫（web `state.running`／TUI `app.running`）＋服务端 `runs` 表 overwrite；ext_method 请求与在途 run 并发处理（acp 连接每请求独立 task）⇒ 补充走新 ext 方法、**不走 session/prompt**，零并发 run 风险 |
| 4 | 排队可见面 | 投递回执＝queue_append 响应带 `depth`；投递事实＝`supplement_delivered` journal 事件 → web/TUI 投影渲染用户卡（投递即可见确认）；客户端发送时系统行提示 |
| 5 | 撤回入口 web+TUI | web＝`/rewind` 命令→对话框；TUI＝`/rewind` 命令→Snapshots 式 modal。**菜单制**（journal 卡↔侧车 ordinal 映射脆弱；CC /rewind 亦为菜单制）；不做逐卡按钮 |
| 6 | 0dd 衔接 | 撤回＝0dd 用户主权的**内容面**操作；idle 门与 close/archive 票机制共用 `runs` 表互斥天然串行；`close_session` 零改动 |

**勘定新发现（S1 读码即勘正设计）**：`LoopProfile::grill` 的 `role` 也是 `AgentRole::Main`——
「主车道」判别不能走 role，否则补充会被 grill 轮 drain。落码改为 `LoopProfile` 新显式槽
`drain_user_supplements`（主 run 车道 true／grill、检索车道 false），钉子
`grill_turn_does_not_drain_supplements` 锁死。

## §2 S2 落码（orz `01386744`，24 文件 +1861/−9）

### 子件①中途补充（queued supplement）

- **ext 通道**：`_orz/queue_append`（client→agent；ACP ext_method `_` 前缀线格式，
  acp 库剥前缀路由 `ExtRequest`→`ExtResponse`）。stdio.rs 增 `ClientRequest::ExtMethodRequest`
  臂（未知 `orz/*`＝method_not_found，现役语义）；**零 8 工具面触碰**。
- **队列**：`SupplementQueue`（orz-host lib.rs）＝`Mutex<VecDeque<String>>`；FIFO；每条
  ≤256 KiB、深度 ≤16 fail-closed（`Result<(), &'static str>` 带原因拒绝，不静默截断）；
  **跨 run 存活**（cancel 不清队；空闲期投递落下一 run 首 loop-top）；close_session/进程
  退出即灭（会话域锚点与 terminal backend 一致）。注入＝`OrzHost::with_supplement_queue`
  （builder setter，不扩 too_many_arguments 构造链）。
- **loop 投递**：`LoopHost::drain_user_supplements()`（default 空＝一次性车道/测试宿主零
  行为）；loop-top drain（压缩算术后）→ 每条**先落 `supplement_delivered` 事件再入
  messages** → 当轮 `model_request` 即含补充。
- **事件族**：`EventType::SupplementDelivered`（非终态）。payload＝`supplement_seq`（run 内
  从 1 严格递增）＋`character_count`＋`text`。**prompt_submitted 同姿态**（用户输入事实、
  非机制事件）：registry v02 条目＋`supplement-delivered-event-payload-v0.2.schema.json`＋
  fixtures 三件＋v02 enum；**法官族/Python 镜像新族/锚不加**（不变量由 loop 单元钉承载；
  conformance 的 registry 解析为硬门——Rust `registry.resolve` 与 Python 判官
  `_derive_payload_registry_views` 双侧均按注册表强制，事件未注册即校验失败，注册已落）。
  历史刊零回归（无锚；无补充的 run 零产出）。

### 子件②撤回（recall）

- **ext 通道**：`_orz/recall_preview`（用户消息清单 `{ordinal, character_count, preview≤120}`，
  侧车唯一真相面）＋`_orz/recall`（`{session_id, ordinal}`）。
- **语义**：CC `/rewind` 编辑式重发——idle-only（`RestoreInflightGuard` 同锁互斥：run/
  restore/recall 三方）；截断＝ordinal 用户消息**含自身**及之后全部移除；响应带
  `recall_text`（原文回输入框）；未投递补充**一并丢弃**（`dropped_supplements` 计数回显）；
  越界 ordinal fail-closed 且失败不清会话（continuation 原样放回）。
- **落点**：`continuation` take-out（None 时沿 next-prompt 同逻辑回退侧车）→ 截断 →
  `persist_conversation_sidecar` 立即落盘 → 写回。**空会话落盘臂**＝persist 的空跳过守卫
  补 `recall_log` 非空例外（撤回到空必须落盘，否则重启复活已撤回尾巴）。
- **审计**：`StoredConversation.recall_log: Vec<RecallRecord>`（serde default 零迁移）＝
  `{recalled_at, ordinal, removed_messages, remaining_messages, dropped_supplements}`；
  run 写回臂原样带回（`restored_recall_log` 转移——否则下次成功 run 清零撤回历史）。
  recall 不建 RST run（会话卷是 local session artifact 非 run evidence chain；restore 有
  run 因其触工作区）。
- **不回滚面（设计稿 D4 诚实边界）**：文件（用户裁决 git 兜底）；LIF 会话轴/黑板 live
  视图/水位档位（机械观测面单调不回退——黑板全量记录纪律 §14.52）；run journal 历史
  （web 旧卡保持为历史证据，撤回以系统行提示）。

### 双前端

- **TUI**：`ClientMsg::SupplementQueued/RewindPreview/RecallDone`＋`spawn_queue_append/
  spawn_rewind_preview/spawn_rewind`（`ClientSideConnection::ext_method` 出站）；runner
  Prompt 臂＝运行中排队（恢复中仍拒——restore 与 prompt 互斥不变）；`Modal::Rewind`
  （`RewindSelector`，SnapshotSelector 同构两段确认）；`/rewind` 命令注册（帮助命令清单
  11→12 随批校准）；`SupplementDelivered` 投影用户卡（bridge 非 Unknown 降级）。
- **web**：acp.js 三 ext 函数（`_orz/*` raw JSON-RPC）；main.js submitPrompt **排队臂**
  （`flash('当前有运行在进行…')` 拒绝退役）＋`/rewind` 命令＋`ui.rewindPreview/rewindRecall/
  fillInput` 动作；dialogs `openRewind`（列表/点选/撤回/原文回输入框/回执 flash——丢弃
  补充计数回显）；projection `supplement_delivered` 用户卡。

## §3 钉子（16 枚）

- **orz-host 8**：队列 FIFO＋深度（`queue_append_enqueues_fifo_with_depth_and_drains`）；
  空文本/深度帽 fail-closed；撤回截断＋侧车落盘＋recall_text＋越界不清会话；撤回丢排队
  残余计数；run 在途拒绝（`RestoreInflightGuard` 互斥）；撤回空会话落盘＋recall_log；
  ext 路由（queue_append 经 ext 面）＋未知方法 method_not_found。
- **orz-loop 3**：loop-top 投递序（事件在 prompt_submitted 后、模型请求面 FaceFingerprint
  前；RequestHeaderChange 只指纹 system+tools 与消息面无关、先于投递无语义影响）；队列
  跨 run 存活＋残余下一 run 首投递＋supplement_seq 随 run 复位；grill 轮不 drain。
- **orz-tui 3**：两段确认武装 ordinal＋modal 关闭焦点回归；运行中/恢复中拒绝；`/rewind`
  命令运行中守卫＋空闲武装 preview。
- **web smoke §13 三节**：supplement_delivered 投递卡（用户面渲染＋turn 计数）；ext 线
  格式与排队/撤回接线字面（`_orz/*` 三方法＋排队臂取代拒绝臂＋/rewind 接线）；撤回对话
  框行为（mock 动作面：列表渲染/确认 ordinal/原文回填/回执 flash＋丢弃计数回显）。

## §4 读数

- `cargo test -p orz-host --lib --test-threads=1` ⇒ **382/0**（基线 374＋8 钉；并行 5 失败
  ＝已知负载敏感集，串行全过——293 批严格审查 RS 登记先例）。
- `cargo test -p orz-loop --lib --test-threads=1` ⇒ **886/0**（基线 883＋3 钉）。
- `cargo test -p orz-assurance` ⇒ **303/0**＋集成 9/0＋4/0；Python 判官注册表派生视图
  实证接受 `supplement_delivered`（schema 路径解析正确）。
- orz-tools 3050/0＋orz-workspace 1500/0＋xai-tty-utils 35/0＋xai-tool-types 100/0＋
  orz-bin 57/0（基线恰等）＋orz-web 47/0（基线恰等）＋orz-tui **175 过＋9 失败**＝已知
  ACAF 环境基线集（172/9 先例，零新增失败；帮助命令计数钉 11→12 随批校准）。
- `node frontend_smoke.mjs` ⇒ 全部通过（含新 §13 三节）。
- fmt：触碰件 19 个 Rust 文件 rustfmt 逐文件净（**过程勘误**：rustfmt 沿 lib.rs 递归子
  模块把非触碰预存漂移件一并格式化，同批 checkout 回滚——非触碰区照旧纪律维持）；
  预存漂移（sampling-types/browser_serp/codex_app/local_browser 族/orz-web lib.rs）非触
  碰区照旧。
- clippy：基线对比法（stash 前后逐位）＝orz-host **114=114**／orz-loop **97=97**／
  orz-tui **106=106**／orz-bin 告警位置逐位同；本批新告警三条随批清（`result_unit_err`
  ＝SupplementQueue::push 改 `Result<(), &'static str>` 带原因／`.err().expect()` 改
  `expect_err`／无用 `.into()` 删除）。

## §5 边界与不取项

1. 零模型面工具改动、零 8 工具面触碰、零提示词改动；桥（orz-web Rust 面）零改动。
2. 打断式补充不做（Esc 硬打断/Send-Now/mid-turn steering/Cursor 切走式——295/296 两轮
   已定）；取消运行仍走现役 session/cancel。
3. 机械观测面（LIF/黑板/水位档位）不回滚——S4 真机观察体感，混淆再议黑板 epoch 标记
   （设计稿 §2 D4 登记）。
4. grill 车道不接队列/撤回；stdio 外部客户端直调 `_orz/*` 属协议面允许、无额外承诺。
5. **环境处置（不入账）**：D: 盘满（100%，os error 112/1455）——清理
   `orz/target/debug/incremental` 37G 再生缓存后复跑全绿；增量编译缓存可再生，零源码
   面影响。

## §6 台账

- 本档；索引头行 v4.274 → **v4.275**（头行＋AUTH-MESSAGE-RECALL-AND-QUEUED-SUPPLEMENT
  新条目）；BACKLOG 指针行/计数行（65 不变）/P1 路由行/`0di` 专节（S1 勘定＋S2 落码
  达成注记＋S3 字节判据冻结）；TODO 计数行/P1 路由行/`P1-0di` 勾选＋闭合待裁决框；
  [`BACKLOG 第二卷 §1.242`](../BACKLOG_AND_PRIORITIES_2.md)；orz 指针 `ba77aacb`→
  `01386744`＋orz_source_manifest 重算 1,488 entries（恰 24 行变化＝24 触碰文件）。
- **计数 65 不变**（两子件落码达成＝闭合待用户裁决）。**不推送**。
- **S3 字节判据（随本批冻结，0dh S3 重建同车核验）**：`_orz/queue_append`／
  `_orz/recall_preview`／`_orz/recall` 字面进体；`supplement_delivered` 进 v02 enum 与
  registry；`recall_log`/`SupplementDelivered`/`drain_user_supplements`/
  `drain_user_supplements` 契约面字面；`SupplementQueue` 类型面。
- **S4 真机判据**（设计稿 §2 D8）：①补充＝运行中发送不打断当前轮、边界后生效；②撤回＝
  截断后上下文与会话存档一致、原 prompt 可编辑重发；③不回滚面为预期确认；④ext 三方法
  web+TUI 双面可用。

## §7 关键词

302 批、0di、MESSAGE-RECALL-AND-QUEUED-SUPPLEMENT、中途补充、queued supplement、
队列、queue_append、ext_method、`_orz/*`、SupplementQueue、drain_user_supplements、
LoopProfile.drain_user_supplements、supplement_delivered、编辑式重发、/rewind、recall、
recall_preview、recall_log、RestoreInflightGuard、Modal::Rewind、openRewind、
会话级 terminal backend 先例、下一批＝0dh S3 载体重建（0dg＋0di 同车）。
