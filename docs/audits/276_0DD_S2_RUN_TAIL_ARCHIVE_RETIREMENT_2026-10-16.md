# 276 批：0dd S2 落码＋钉子——交互车道 run 尾自动归档退役（orz `4749f6b1`）（2026-10-16）

> **用户令**：「请开始进S2吧」（275 批设计稿 D1–D8 定案后的落码放行）。
> **性质**：**落码批**——orz 子仓一件（`orz-host/src/acp_server.rs`，131+/35−）＋
> 父仓 dated 档转录三处＋README 一句收紧＋登记面同步；零跑批；**计数不变 61**。
> **结论先行**：`handle_session_prompt` run 尾 500K 里程碑增量归档 spawn 块删除
> （DP-11④ ACP 半边退役）；`PendingArchiveTicket.incremental` 旗标连带退役
> （ACP 票恒 close 道）；`incremental_archive_due`／水位／`archive_session_package
> (incremental)` 原语保留给无头道（0ak 单一源不破）；close 推迟票（B3 P2-3）不动。
> 钉 N1/N2 全绿；host lib 单线程 **365/0**（+2 钉）＋web **47/0**＋bin check 净＋
> clippy 触碰面零新增＋fmt 触碰区净。

## §1 落码面（orz `4749f6b1`，单文件）

| 改动 | 内容 |
|---|---|
| 删 spawn 块 | `handle_session_prompt` run 尾 `incremental_archive_due` 判定＋`spawn_archive_task(incremental:true)` 整块移除，原位留 0dd 退役注记（**不得 reintroduce**；归档仅由 close/0ak/0cy/on-demand 发起） |
| 票字段退役 | `PendingArchiveTicket.incremental` 撤除——spawn 恒传非增量、close 构造点连带收缩；`include_footprint` 注释改注「close 票恒 true」 |
| 注释同步 | `ARCHIVE_INCREMENT_TOKENS` 与 `incremental_archive_due` 文档改注「唯一消费者＝无头车道（0ak）」；`record_archived_tokens` 调用点「下次 run 尾重判」改为「无头道下次收尾重判」 |

## §2 钉子（N1/N2 新增；N3–N5 既有全绿）

> **勘误（2026-10-16，279 批）**：本节 N1 前提读数失真（原文保留于下）——「1.5M 字符长答」系当时估算；实测长答 **1,088,890 字符**（字符数定长）⇒ 侧车估算 ≈544,448、≥500K 余量 ~8.9%（**前提确定性成立**，钉子回归强度不变）；orz 测试注释已随 orz `1ecad57c` 同步勘正；提交信息（orz `4749f6b1`）同源失真按不改写历史纪律留档。

- **N1 `run_tail_performs_no_archive_even_across_milestone`**：1.5M 字符互异词元长答
  （大分块 200K 防流式碎 chunk）驱动成功 run ⇒ 断言零归档包／零里程碑水位／零 ARC
  journal／侧车照常落盘；**前提自证**＝回读侧车 `estimate_conversation_tokens ≥ 500K`
  （旧判据在彼刻必真——无水位 ⇒ due——钉子具回归强度）。〔前提读数已由 279 批勘误，见上勘注〕
- **N2 `deferred_close_ticket_still_archives_at_run_tail`**：向 `pending_archives` 注入
  close 推迟票 → 成功 run 尾消费 → 含足迹包落盘（B3 P2-3 语义不翻；close_session
  入队半边不改）。
- **实施勘误（如实记）**：两钉初版以 `});` 收尾漏 `.await`——clippy「unused
  implementer of Future」抓出空跑通过（0.00s 假绿）；补 `.await` 后 N1 暴露第二处
  勘误＝会话 8 前缀误计（`sess-n2de` → 实为 `sess-n2d`；归档机器实际已正确落包），
  断言文件名随勘。N1 经此成为双重断言（文件面＋ARC 计数面＋前提自证）。
- **N3–N5**：手动面 archive/unarchive/delete 既有测试、无头幂等
  （`headless_run_archive_is_idempotent_per_milestone`）、纯判定
  （`archive_keys_and_incremental_milestones_follow_the_design`）随全量套件全绿。

## §3 验证读数

- `cargo test -p orz-host --lib -- --test-threads=1`：**365 passed / 0 failed**（6 ignored；+2 钉）。
- `cargo test -p orz-web`：**47/0**（与 246 批基线一致）。
- `cargo check -p orz-bin`：净。
- clippy（`-p orz-host --all-targets`）：触碰面零新增（文件内余 3 条预存：775/1250/5390）。
- fmt：触碰区净；**`cargo fmt -p orz-host` 曾把 crate 内 12 文件预存折行漂移一并改写
  ——已按「预存漂移不动」纪律逐文件 `git checkout --` 还原**（漂移仍留 HEAD 基线；
  orz-sampling-types 预存漂移同样未动）。

## §4 dated 档转录（父仓，随批落）

1. **ADR-0010 卷 14 §14.84**（v1.87）：会话生命周期用户主权——交互车道 run 尾自动
   归档退役（来源／裁决／原语单一源／数据安全／契约零变更／落码与钉子五条）。
2. **滑块设计稿 §3.5.1**：追加「增量归档时点勘误（0dd，2026-10-16）」——里程碑追加
   半边 ACP 车道退役、无头半边保留、「不结束没存档」缺口重开＝显式接受。
3. **0cy 设计稿 §2**：追加勘注——「500K 增量」行自此仅指无头车道；收卷口径经 0dd S1
   裁决保留现状不改提示制。
4. **README**：「交互会话结束时由 session_archive 打包」收紧为归档包发起面口径
   （用户显式动作／会话关闭／新会话起点收卷／无头里程碑）＋0dd 括注。

## §5 边界

1. 零跑批、零重建（S3 载体重建进体待放行；版本号随批）；web 前端零改动（D6）。
2. orz 推送另行放行；父仓提交含 orz 指针 bump（`0ba00262` → `4749f6b1`）。
3. 计数不变 61（S2 非闭合批）。
4. orz-host crate 内 12 文件预存 fmt 折行漂移＝预存基线事实，本批如实登记、未收编。

## §6 关键词

276 批、0dd S2、run 尾自动归档退役、handle_session_prompt、PendingArchiveTicket.incremental
退役、incremental_archive_due 无头唯一消费者、N1 零归档钉、N2 close 推迟票回归、
前提自证估算≥500K、ADR §14.84 v1.87、滑块 §3.5.1 勘误、0cy §2 勘注、README 收紧、
host 365/0、web 47/0、orz 4749f6b1、计数 61 不变、索引 v4.249、fmt 预存漂移未收编。
