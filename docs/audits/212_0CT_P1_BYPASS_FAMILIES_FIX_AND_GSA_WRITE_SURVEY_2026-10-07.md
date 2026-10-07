# 212 批：0ct P1 两族旁路收口（原 P1 恢复执行）＋`.gsa` 写因调查要点＋备份区考虑项登记（2026-10-07）

> **用户令**：先经一轮调查（「模型不是只有一次写入，这一问题比较多发，关键是模型为什么写入，
> 他是否需要写入？我们先进一轮调查再做决策吧」）→ 裁决「**原 P1 可以正常做**，需要进一步考虑
> 的就是如何处理模型想要备份这一问题」。
> **性质**：修复批（exec_policy 单文件）＋调查要点入档；**计数不变 63**；未提交、未推送；
> **S3 载体前置门解除**（P1 已收口）。
> **P1 处置对象**：209 批挂起的 `read_direction_exempt_positions` 复制族两族写向旁路
> （209 档 §0）——修复面＝单函数内两处收集逻辑＋一张新闭表；真实语料零实踩（见 §1），
> 修复为不变量卫生性质。

## §1 `.gsa` 写因调查要点（2026-10-07，本批前置调查入档）

**口径**：磁盘全量任务工件＝41 任务树（官方 0cr 36 题＋0cj 早期轮）、303 份 journal、234 个
checkpoint `.gsa` 卷影快照；journal 穷举（660 条触及 `.gsa` 事件、3,110 条写控审查）＋卷影
逐文件归类。边界＝journal 不带命令原文（仅哈希＋长度）、模型文本流零提及 `.gsa`
（thinking=none），写因按目标形状＋block 前后事件＋卷影证据重建。提取底稿
`.tmp-p1-survey/gsa_events.jsonl`。

**核心读数**：

- **写向尝试（被拦）全 corpus 7 起**：官方轮 5 起（13.9% 题发生率）＝D2 dynamic_config
  seq188（多目标 rm 含 rollback）／D4 l2m seq173（写自己 run 的 journal 本体）／D4 meshctl
  seq67（读向 cp＝0ct 触发例）／**D4 metric_transform_lang seq194（写 rollback 备份槽——
  SCB §7.1 表漏记此条，9 例实为 10 例；D4 日账 372 次审查与 allow 315＋warn 50＋block 2
  加总 367 差 5，此条疑在缺口内，如实登记不回改批档）**／D6 test_translator seq177（自建
  .bak 入 rollback）；0cj 早期 recli 0238 两起（同 run 两次＝全 corpus 唯一 run 内重复）。
- **读向交互是主体**：官方轮 21/36 题（58%）动过 `.gsa`，87 条工具事件中 82 条为读/非写；
  且读几乎全走 shell 通道（工具级两段门信封官方轮仅 2 起）——shell 直读旁路教育门是常态
  （与 0ct 读全开放裁决方向一致）。
- **零成功写入、零旁路实踩**：234 个卷影逐文件归类，`.gsa` 内全部文件皆为框架命名
  （rollback 4,562 个文件全部吻合 harness `*callXX.bak(.meta)` 模式）；allow 级 `.gsa` 写控
  事件为零。**P1 两族旁路在全部真实语料中从未被模型踩中**——修复是不变量卫生，非止损。
- **block 后行为**：7 起全部 1 轮改道、同 run 零 `.gsa` 重试、零题因写控失败；meshctl 被拦
  后下一手为 blackboard_write（框架正规替代物的自发使用）。

**写因分类（官方 5 起）**：A 备份槽误用 2 起（mtl/ttr——把 harness 逐调用备份区
`.gsa/rollback/<hash>/<ts>-callXX.bak` 当自己的备份库写入；harness prompt/环境配置从未
文档化该区，模型经验发现后归纳误用）；B 备份区清理 1 起（dcfg rm）；C journal 本体写入
1 起（l2m——唯一真系统面，意图不可确证，有界假设＝重定向/原位改）；D 读向误拦 1 起
（meshctl，已由 0ct 修）。**"是否需要写"判定＝不需要**：无一为任务所必需（改道即成），
唯一持续真需求是读（已由 0ct 满足）。

## §2 P1 修复（exec_policy.rs 单文件）

- **族 a（nullish 目标位竞选落点）**：复制族段的位置词元收集改为——重定向符号（`>`/`>>`）
  的**后随词元不问 nullish 一律排除**（原实现只排非 nullish，`/dev/null` 留在收集里竞选成
  "末位落点"，真落点位 `.gsa` 被豁免成源位）；**裸数字词元一并排除**——tokenizer 落地行为
  为 `2>` 分词产出 `2`＋`>` 两词元、`2>&1` 的 `&` 是段分隔符，故 `cp a .gsa/b >/dev/null
  2>&1` 的旁路实为裸数字 `2` 竞选成"末位落点"（非 `/dev/null` 本身）。排除出的词元由主扫描
  恒覆盖，绝不因解析放行。
- **族 b（旗值落点）**：新增闭表 `COPY_DEST_VALUE_FLAGS`（4 项＝`-t`/`--target-directory`/
  `-destination`/`--destination`）——复制族段内出现任一即**整段不豁免**（保守全扫＝0ct 前
  行为；沿"目标位不可辨即不豁免"既纪律）。`=` 形（`--target-directory=<gsa>`）不经本表，
  由主扫描 `path_candidates` 的 kv 拆值直接覆盖（本批钉锁）。**误伤面（既登记代价）**＝经
  `-t` 从 `.gsa` 读向复制的罕见形状被拒（保守方向：宁可多扫）。
- 文档同步：module 头 0ct 段补 212 修复记载；函数 doc 重写（旁路机理＋分词落地行为入注）。

## §3 钉（先红后绿）

- **新钉 2 条**：`gsa_nullish_redirect_target_and_fd_shards_never_dest_candidates`
  （修复前 Allow 的 2 形状→Block；读向 2 形状→Allow 回归）＋
  `gsa_dest_value_flags_disable_source_exemption`（4 旁路形状→Block；`=` 形→Block；
  `-t` 读向误伤面备查钉）；防膨胀钉补 `COPY_DEST_VALUE_FLAGS.len() == 4`。
- **旧钉全绿**：`gsa_` 组 19/19（原 17＋新 2）；触发例 8 钉、写向对照 8 钉、两段门/
  逃逸/凭据各组零回归。

## §4 验证

- **orz-tools lib 全量 3023/0**（3021＋恰 2 新钉；6 ignored 口径同 209）。
- clippy（lib＋tests）26 条全存量（与 209 时点同数、逐条落存量行，本批零新增）。
- fmt：`cargo fmt -p orz-tools` 净；hunk 全部 confined（header/豁免函数/规则 5 注/钉组/
  防膨胀）。**如实记**：本批先用了裸 `rustfmt --edition 2021`（本仓实际 2024 let-chains），
  报错未落盘、改用 `cargo fmt` 后净——209 批 blackboard.rs 的 1299 卷入即同源（当时裸
  rustfmt 以错误 edition 重排后手工回退），两条经验并记。
- 门禁 `check_repository.py`：见 §6（本批面全绿；隔壁 0cv 在途项沿 209 口径）。

## §5 备份区考虑项登记（用户「需要进一步考虑」，不计入开放项）

调查实证：模型在**受挫时刻**有自备份冲动（2/5 官方写向＝自建 .bak 入 rollback；block 前一拍
均为工具报错），但**并非"没有备份区可用"**——`/tmp` 与工作区备份从未被尝试、blackboard 可
记状态、0ct 后 harness 备份区读向完全开放（meshctl 的真需求＝读旧版比对，已满足）。模型选
`.gsa/rollback` 纯因"那里看起来就是备份区"。**候选方向（留用户裁决，本批零实现）**：
(i) 维持现状＋0ct 确认信封加一句 rollback 语义教学（掐误归纳诱因，零新机制）；
(ii) 提供模型可写的正式备份/暂存面（新机制，需立项评估与 8 工具面/分区纪律对照）；
(iii) 纯观察（数据上成本已极低）。登记于本档与 TODO `P1-0ct` 节，不占计数。

## §6 门禁

`python -m scripts.check_repository` ⇒ **`valid: false`／error_count 1＝「orz submodule
working tree is dirty」**（204/209/212 三批未提交的直接结果、沿 204 同型如实记）；其余面
全绿——台账帽内（指针行 1175／计数行 1132／TODO P1 路由行 1190／P1 总览行 1169）＋一致性
（计数 63、P1 token 三面一致）＋schema/fixtures；隔壁 0cv 在途项已由其自行清零。

## §7 台账

- 本档：`docs/audits/212_0CT_P1_BYPASS_FAMILIES_FIX_AND_GSA_WRITE_SURVEY_2026-10-07.md`（编号说明：206–211 被并行窗 SCB 系/0cv/本批改号竞用占用，本批终号 212，沿 208/205 先例如实记）。
- TODO：头部计数行（63 不变、本批指针）＋`P1-0ct` 节（P1 挂起项勾销→212 修复；S3 前置门
  解除；备份区考虑项登记）＋P1 路由行。
- BACKLOG：本批记录指针＋计数行（63 不变）＋P1 总览行＋`0ct` 节处置＋锚点行。
- 索引：头行 → v4.184（并行窗顺延）。第二卷 §1.159。
