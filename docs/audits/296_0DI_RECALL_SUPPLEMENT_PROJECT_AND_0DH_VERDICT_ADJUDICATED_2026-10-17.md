# 296 批：0di 立项（消息撤回＋中途补充，Claude Code 形态移植）＋0dh 选型裁决落账（2026-10-17）

> 性质：**裁决落账＋立项批**（用户三裁决）。零源码、零 orz 改动、零跑批；计数 64 → 65（0di）。

## §1 用户三裁决（原话要点）

1. 「撤回不连带工作区文件，文件面用git兜底即可」；
2. 「那就按照你裁决的启用grok+三处借鉴来裁决0dh」；
3. 「撤回和中途补充就完全借鉴claude code，撤回和中途补充部分单独立一个项吧」。

## §2 裁决落账

### 0dh 选型定案（S0-2 拍板）

**基座＝启用 Grok（`hide_background_input=false` 逃生阀重开显式后台面），借鉴 Claude Code 三处**——
①教学面一句话＋参数自解释；②增量读出（`get_task_output` 现役整读 output_file 尾部截断 → S1 采增量游标）；
③生命周期边界显式化（会话属主跨 run 不收割＋30h，超出 Claude Code「会话内跨轮」口径，按用户 294 裁决走）。
**S1 设计稿自此可启动**（五项待勘不变：收割豁除×0bq 并存／per-call Job 归属／参数措辞×8 工具面对账／30h 落点／ADR §14.87）。

### 0di 立项：消息撤回＋中途补充（Claude Code 形态移植；MESSAGE-RECALL-AND-QUEUED-SUPPLEMENT）

**完全借鉴 Claude Code**，两个子件：

- **中途补充（queued supplement）**＝运行中输入进**队列**，**不打断**当前思考/工具批，在**下一次 LLM 调用边界
  flush** 投递（CC #50246 deferred 语义）；可叠多条；**不做**即时打断形态（Esc 硬打断／Send-Now／mid-turn
  steering／Cursor 切走式）。S1 勘定：排队归属（web 前端 vs 桥 vs agent loop）＋flush 点定义＋ACP 并发
  `session/prompt` 服务端语义＋排队可见面＋orz-tui 同族。
- **撤回（recall）**＝CC `/rewind` 的**编辑式重发**形态——恢复会话到目标消息之前（截断）＋原 prompt 回输入框
  供编辑重发；**用户已裁决：不连带工作区文件，文件面 git 兜底**（沿 CC「bash 副作用不可回退」诚实边界，
  不建文件 checkpoint）。orz 落点：`conversations/*.json` 截断＋上下文 messages 截断（与压缩折叠/存档三键
  机制对账）＋web/TUI 撤回入口交互面＋与 0dd（会话生命周期用户主权）衔接。
- **退役**：candidate `OBS-WEB-DOGFOOD-2026-10-17` 第三件（N3）随本立项升级，该 candidate 三件全部出清。
- **批序**：S1 设计稿（两子件面勘定）→S2 落码＋钉子→S3 载体重建→S4 真机（判据随 S1 定：补充＝运行中
  发送不拆断动作且边界后生效；撤回＝截断后上下文与会话存档一致、原 prompt 可编辑重发）。

## §3 处置

- 0di 立项（64 → 65）；0dh S0-2 裁决落账（状态待拍板 → 选型已定案＝S1 可启动）。
- 登记面：BACKLOG（计数行/指针行/P1 总览行/P1 锚点行/0dh 节勘正/新增 0di 节）＋TODO（计数行/P1 路由行/
  新增 P1-0di 节/P1-0dh 勘正）＋索引（头行 v4.269/OBS 条目出清注记/0dh 条目勘正）＋第二卷 §1.236。
- 零源码、零跑批；本批未发起任何 orz 会话。
