# ORZ-RUN-SEPARATION-001 — 跨 run 产物归属隔离：每 run 的产物归不同 run，不得混为一谈（案例候选）

- **状态**：`candidate`（2026-09-19 用户裁决沉淀；处置终点＝案例登记，不立工程项）
- **来源证据**：[`0AR_S1_CONTRACT_SURFACE_2026-09-19` §7-F2](../../audits/0AR_S1_CONTRACT_SURFACE_2026-09-19.md)
  （狗粮 run `RUN-CLI-6aad91f0` 被启动器误杀 → WIP 由 `RUN-CLI-6aad9497` 承接的交接现场）；
  用户裁决口径（2026-09-19）：**「每不同 run 的产物明确归不同 run，关键是不能混为一谈」**，
  且「额外能做的也没有什么了」——案例沉淀即本摩擦处置终点。
- **摩擦面（F2，严重级）**：被误杀 run 遗留的 WIP 中，`scripts/check_repository.py` 处于
  **语法级破损**（`UnboundLocalError`：负例登记写在负例映射建立之前）——「跑不通」的半成品
  若无人接手，收尾批会被直接卡死。交接得以成立依赖承接轮恰好按 run id 认领 WIP；
  归属一旦含混，破损件、断链 journal（停在 seq 776、无终态事件）与健康 run 的读数将互相污染。
- **能力（纪律，三字段可机械核对）**：
  1. **产物归属 run**：WIP 快照、journal、`.tmp-dogfood-<stamp>.log`、心跳侧车
     （`.log.live`）、测试读数——落盘时即绑定 run id / session8 / 启动时间戳，
     文件名可反查归属；不得写共用路径让两个 run 的产物覆盖混叠。
  2. **交接声明来源**：承接 run 开工前声明「承接自哪个 run、认领哪些 WIP、遗弃哪些」；
     接手后的修复与读数记在本 run 名下，不回填前身 run 的账。
  3. **读数按 run 切分**：任何「run 的 journal/日志/产物」读数结论只对该 run id 成立；
     跨 run 汇总前先逐 run 归属核对，禁止以目录名或时段推断归属。
- **同族先例（只引用、不复制机制）**：
  - TB21 适配器跨 run 隔离缺口（[`TB21_V41_TIMEOUT3_VERIFY` §10.7](../../audits/TB21_V41_TIMEOUT3_VERIFY_2026-09-18.md)）：
    容器启动前把兄弟试次移出挂载根至 `gsa-volumes/.quarantine/<job>/`——装置侧的
    run 间串染隔离，与本案例同族不同层（装置 vs 流程）。
  - [`ORZ-VERDICT-EPOCH-001`](ORZ-VERDICT-EPOCH-001-verdict-epoch-discipline.md)：相邻但不同——
    那条管「结论的代际时效」，本条管「产物的归属边界」；混 run 产物正是混代际结论的常见来源之一。
  - [`ORZ-ENV-POLLUTION-001`](ORZ-ENV-POLLUTION-001-env-pollution-fake-red.md)：同属
    「读数结论产出前先核来源」族——那条核 env 污染，本条核 run 归属。
- **回归入口**：journal `run_id`（`RUN-CLI-*`）↔ `.gsa/conversations/<session8>.json` ↔
  日志文件名 stamp ↔ WIP 快照路径，四者可交叉核对；断链 run（无 `run_finished`）的产物
  必须**显式标记孤儿态**后再交接，不得与健康 run 产物并列直读。
- **边界**：只约束流程纪律与读数口径，**不新增阻断门**（机械层不参与产物归属判定）；
  不改动 `.gsa` 落盘布局与 journal 单 writer 纪律；用户已定性「额外能做的也没有什么了」——
  本案例即处置终点，后续若出现归属混叠事故按本案例字段复盘。
- **关键词**：跨 run 产物归属、WIP 交接、孤儿 run、断链 journal、读数切分、交接声明、F2、0ar S1。
