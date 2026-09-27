# 097 尾巴闭合与状态翻转落账（0bv／0bw／0bq；2026-09-27）

> **日期**：2026-09-27；**用户令**：「请先更新落后登记，然后清尾巴并做0ac S4实机复验吧，收完再跑」。
> **本批＝尾巴执行与闭合落账**（096 批已做落后登记对齐）；**计数 58 → 55**（0bq／0bw／0bv 三项闭合）。
> **形态**：父仓账本批＋orz 树面两笔本地提交（`7dccb794`＝转码梯＋shadow 落账三文件语义改动；`d0e29615`＝rustfmt 版本噪声收编六文件〔F11/0bs⑤ 登记面，零语义〕；源清单重生成 1506 条 9 处哈希更新）；**未推送**（推送待令）；**未重建载体**（树面增量随下一载体令，0.8.0 在役不受影响）。
> 0ac S4 三题跑批随本批并行进行，读数与收口另落 [`098 S4 档`]（后续批）。

## §1 尾巴执行回执

### 1.1 并件三：读取侧转码梯覆盖面扩张（执行纪律＝先勘定后落码）

- **勘定（红测实证）**：缺口臂＝`computer/local/terminal.rs::maybe_truncate` 截断臂——长输出（超过 `output_byte_limit`）的字符计数与切片走 `String::from_utf8_lossy`，GBK 字节在进入 `decode_text` 梯**之前**即被摧毁（U+FFFD 预注入），`to_result` 的梯收到的是已损字节。0BV 轮 F7「冒烟日志经 GBK 控制台读出乱码、而 journal 后端编码门已识别 gb18030」由此得到机制解释：门在捕获侧、摧毁在截断侧。**分支①（梯臂真缺）成立**。
- **落码**：截断臂改走 `decode_text` 梯（解码后的文本上计数与切片、以 UTF-8 回存），新增 `truncation_decode_label` 留档截断前原始解码标签，`to_result` 合并标签（OPS-PROTOCOL §8 只记实际产出文本的阶段，不把 gb18030 来源失真标成纯 utf-8）。
- **钉子（REV-083-14 先红后绿）**：`truncated_gbk_output_decodes_via_capture_ladder_not_lossy`——长 GBK 夹具截断后：头/尾中文完好＋零 U+FFFD＋标签含 gb18030 段。修复前实跑**红**（lossy 摧毁复现）、修复后**绿**。
- **读数**：orz-tools 全量 **2973/0**（6 ignored 不计）；`cargo fmt -p orz-tools` 净。

### 1.2 REV-083-09：ACAF 收口批（余量两件）

- **⑤ shadow pre-signing 拒绝落 journal（落码）**：`orz-loop/src/acaf_flow.rs::fail_closed_refusal` 由「shadow 静默放行」改为「**shadow 亦照常落 `control_ticket_rejected`（null ticket_id＋实际 code/detail）后放行**；fail-closed 落账并阻断」——`ORZ_ACAF_FAIL_CLOSED=0` 注入不再是无审计痕迹的静默降级（083 审查 §8 立案本意）。上游**未配置 fabric** 的 shadow 路径维持注册静默（ACAF 从未激活、无降级可审）；D-14 空参数、D-15 缺快照库两条调用点注释随新边界改写。**钉子**：`orz-bin/tests/acaf_e2e.rs::pre_signing_refusal_records_rejection_in_shadow_mode`（shadow 下缺 `file_path` 的 search_replace ⇒ journal 恰 1 条 `control_ticket_rejected`、`reject_code=missing_target_argument`、null ticket_id）。
- **② 独立验票执行器（排期登记，不占计数）**：ACAF §10 开放问题「验票移独立执行器」登记为 **`candidate` 观察项**（触发器＝下次 ACAF 面改动批并入实施；不单独排期占用载体轮）。与 ⑤ 合计 REV-083-09 全量闭合（⑥ FAIL_CLOSED=0 启动告警已于 092 批落地）。
- **读数**：orz-bin acaf_e2e **24/24**（含新钉）；orz-loop 全量 **852 基线确认**（`-j 4` 与串行各现 2–3 个 `cancel_*` 时序敏感失败，**单独串行复跑全过**——与严格审查已登记的负载敏感族同形，非本批回归）；fmt 净。

### 1.3 矩阵 ③④ UI 级重走（0BV S4 被 F-1/F-2 阻断面的补验；载体 0.8.0 真机、`orz web --real`、回环 21630、ACAF 三 env、UI 实驱）

| 点 | 读数 |
|---|---|
| ③ 切后旧会话尾不断 | **✅ 达成**——D:\CLI 打开会话 6ab812c6 后切往 B（`.tmp-tail-ws-b`）：横幅「已切换工作区：D:\tb-eval\.tmp-tail-ws-b」；旧会话视图保留不清场，呈「（会话回放 6ab812c6 —— 1 次运行合并；只读；点『后退』返回实时）」；探索器活跃会话即时刷新为 B 的清单（0 条） |
| ④ 新区新会话 | **✅ 达成**——B 内「新建会话」＋发送连通消息：会话 `ed376c03` 建立、`run_finished(completed)`；**文件面双确认**＝`B:\.gsa\conversations\ed376c03.json`＋`B:\.gsa\runs\RUN-ed376c03-0` 在 B 落盘、D:\CLI 侧零泄漏（spawn cwd=B 实证） |
| ① 未信任拒绝（UI 面） | **结构性不可点**——B 形态探索器只列已信任工作区，未信任条目无 UI 入口；403 fail-closed 拒绝面维持 093 API 级读数；**F-1 修复的通过半（已信任工作区可切）本批 UI 级实证**（③④ 即经信任门成功） |
| ② 运行中禁切（回归） | **✅ 保持**——B 内 40s 运行中点击 D:\CLI 行 → flash「运行中禁切：等本轮结束（或 Ctrl+Z 取消）后再切换工作区」、当前根不变、零切换 |

### 1.4 链首就绪承接采样（0bv①「浏览器就绪后 web_search 由链首承接」变体）

- **采样轮两跑（v2＝`RUN-CLI-6ab90f62`／v3＝`RUN-CLI-6ab91021`，`ORZ_RETRIEVAL_ENABLED=1`，launcher 起跑，载体 0.8.0）**：
  - v2：检索子代理首次 browser_read → `browser_launch_result` **failure**（CDP discovery 连接被拒 os error 10061＝未启动，如实失败）；其后 browser_read 重试 **success**（浏览器就绪）；子代理报告车道记录原文「web_search 经 browser_serp 回退 local_http 后返回 0 结果」。
  - v3：browser_control navigate → `browser_launch_result` **success**（两次）＋页面主标题取回；**随后的 web_search 仍让渡 local_http（cause=browser_unavailable）**，v3 模型自证「未复现就绪后承接变体」并**复核 v2 journal 判其同向（亦未承接）**——本主会话初读对 v2 报告句的「承接成立」解读**据此撤回**，以事件面复核为准。
- **结论（如实）**：**「就绪后由链首承接交付」变体在两轮真机中均未生效**——浏览器就绪事实（`browser_launch_result` success）存在后，后续激活的 web_search 仍走让渡臂（`browser_unavailable` → local_http）。潜在接缝＝**链首就绪信号跨激活不传播**（0bs⑧ 定 `BrowserSerp` 为 session-scoped 能力，激活内 S2 单测/集成读数成立，跨激活传播无真机正样本）⇒ 登记 **`OBS-SERP-READY-HANDOFF-CROSS-ACTIVATION`（candidate，仅记录不占计数；触发器＝下次检索面改动批并入勘定）**；0bv① 的让渡半（0BV S4 实证）与 S2 测试面维持达成，本观察不阻断闭合、留候选勘定。
- **采样方法学观察（仅记录）**：主车道模型面对「用浏览器导航」类题面会**如实报告自身无浏览器工具并阻断**（RUN-CLI-6ab90e4d，工具面 9 个＝冻结面无泄漏）——正面证据；采样题面需以检索通道措辞发起。v3 任务两步均达成（标题取回＋note 落盘 `sha256=f09877ee…`）。

### 1.5 0bw④ git 半边裁决（D-6 遗留：容器／触发／回收三项）

- **裁决＝git 自动检查点不实施**（登记为设计留档）：① L4 补偿目标已由**编辑面回退窗口（0bm⑦）＋`orz rollback` undo 面（过夜批）＋载体完整性自检（0bw②）**交付，误写可回退性成立；② git 自动检查点必须写用户仓库状态（refs/stash/对象库），与「orz 不动用户 git 状态」的项目边界冲突（0bv 狗粮题边界亦一贯禁 commit/push），容器／触发／回收三面均无干净解；③ 剩余风险（undo 窗外的批量破坏）属 L1 锁死面与 L2 审查的职责域，不由 L4 重复兜底。0bw④ 以 **undo 面达成**闭合，git 半边留档不实施。

### 1.6 真网补读四点（0bv S3 行登记项）处置

DDG 代理腿／百度 link 壳展开／指纹 off 对照／arXiv 回落 UA——前两点与第四点已由[`真网在线验证 2026-09-26`](0BS_RETRIEVAL_ONLINE_PROBE_2026-09-26.md)探针读数（DDG 直连 202 挑战／百度双路可达＋真解析 8-9 命中／arXiv 默认 UA 406）与 0BV S4／本批采样轮真机在题实测覆盖；**指纹 off 对照未单独跑**，登记为后续自然狗粮轮观察项（不阻断闭合）。

## §2 状态翻转与闭合（计数 58 → 55）

| 项 | 翻转 | 闭合判据 |
|---|---|---|
| **0bq** GAP-SPAWN-ORPHAN-RECLAIM | `partial` → **`implemented`**（58 → **57**） | 出身登记在册（0BV S4 §1.3）＋收尾扫净 20 目标＝19 not_running＋1 fingerprint_unknown fail-closed 拒杀、零孤儿零残留 ⇒ 判据「主进程结束而派生进程仍存活」零复现达成 |
| **0bw** GAP-WRITE-GUARD-MECHANICAL | `pending` → **`implemented`**（57 → **56**） | S1 设计档／S2 v1＋①–④ 落码进载体（0.8.0 在役）／CFA 探针两轮读数空 ⇒ ⑤ 维持不翻（D-3）／⑥ S3 载体＋S4 真机读数（0BV S4：零拦截零误拦、L2 零 block-warn 痕迹、Landlock ABI=3）／④ git 半边本批裁决不实施（§1.5） |
| **0bv** GAP-FRICTION-AND-CARRYOVER-BATCH-R5 | `pending` → **`implemented`**（56 → **55**） | 五件 S2/S3/S4 达成（③④ 本批 UI 级补走；链首承接变体两跑未复现 ⇒ 登记 `OBS-SERP-READY-HANDOFF-CROSS-ACTIVATION` candidate 不占计数，§1.4）＋并件 24 项全量闭合（REV-083 18 项＋0bw 六项；09 含本批 §1.2）＋F4 预置红已修；f13/f14/f15 维持 S2 测试读数、自然触发采样随后续轮（0BV S4 §3 口径） |

## §3 台账同步清单

- BACKLOG：计数行 58 → **55**（本批指针→第二卷 §1.50）；P1 开放项清单与优先级总览表撤 0bq／0bv／0bw、入已闭合枚举；三节闭合行。
- TODO：计数行同步；P1 路由行撤三项；`P1-0bv` 并件三补勾＋闭合、`P1-0bw` git 半边勾选闭合、`P1-0bq` 闭合行。
- 索引：`GAP-SPAWN-ORPHAN-RECLAIM` §6 条目闭合注记；§8 `GAP-SPAWN-ORPHAN-RECLAIM`（partial→implemented）、`GAP-WRITE-GUARD-MECHANICAL` 与 `GAP-FRICTION-AND-CARRYOVER-BATCH-R5`（pending→implemented）；头行 v4.65 → **v4.66**（v4.65 滚入存档卷 115 → 116 行）。
- 第二卷：§1.50（本批）。
- 门禁：`check_repository.py` ⇒ `valid: true`（目标）。

## §4 运维记录（仅记录）

- **磁盘腾挪**（构建期 D 盘 100% 满，os error 112）：删探针/已被取代 gsa 卷（gate-google-*、s4-0x-0v-*、s4-2026-08-31*、official-r2-failures-recheck-10t，≈10G——官方 R3/R4/R4b/R0/verify 证据卷全数保留）＋已发行/已被取代 rel-* stage 九处（≈3.4G，GitHub Release 可复得，沿 084 批先例；rel-095 保留）＋ `cargo clean --profile dev`（23.0GiB，沿 093 惯例）⇒ D 盘余量 13M → **35G**。
- **构建摩擦（仅记录）**：冷全量重建两遇 rustc `STATUS_STACK_BUFFER_OVERRUN`（0xc0000409，与 093 OOM 同族内存压力形态），`-j 4` 降并行后过；`protoc` env 缺失复发（093 惯例 `PROTOC=D:\tb-eval\.tools\protoc-25.3\bin\protoc.exe`）。
- **采样摩擦（仅记录）**：`orz web` 桥不经用户级 env 继承 ACAF 三件（会话内显式 export 后正常）；web 走查临时信任条目与 `.tmp-tail-ws-b/-c` 测试工作区随批清理（B 内测试会话 ed376c03 为 §1.3 证据，留档至本批落账后清理）。

## §5 关联与关键词

[`096 落后登记对齐`]（第二卷 §1.49）／[`0BV S4 真机复验`](0BV_S4_LIVE_VERIFICATION_2026-09-27.md)／[`0BV 并件余项与 S4 修复批`](0BV_REMAINING_ITEMS_AND_S4_FIXES_2026-09-27.md)／[`真网在线验证`](0BS_RETRIEVAL_ONLINE_PROBE_2026-09-26.md)／[`095 发行档`](095_RELEASE_2026-09-27.md)／BACKLOG `0bv`·`0bw`·`0bq`／TODO `P1-0bv`·`P1-0bw`·`P1-0bq`。

关键词：尾巴闭合、转码梯截断臂、from_utf8_lossy 预摧毁、truncation_decode_label、shadow pre-signing 落账、fail_closed_refusal、独立验票执行器排期、矩阵③④UI 级、旧工作区会话回放、新区新会话落盘、链首承接、browser_serp 回退 local_http、git 自动检查点不实施、0bq 闭合、0bw 闭合、0bv 闭合、计数 55。
