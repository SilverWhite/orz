# 摩擦台账（FRICTION LEDGER）

> 状态：`current`；建立：2026-09-13（orz 制作 orz 狗粮线，用户裁决）。本文件是**活页台账**，不是审计快照：每轮 orz 真机 run 产生的摩擦项在此累积，逐条对账后要么立案（转 BACKLOG/GAP）、要么登记为观察、要么标记已修。
>
> **写入协议**（2026-09-13 用户裁决）：
> 1. **orz 侧**：每轮 run 收尾由 orz 中的模型**直接追加**条目到本文件（run 末摩擦自报，三约束：run 末一次性不逐轮注入；只报事实不给建议；无摩擦明说"无"不凑数）。条目必须带 `run_id`。
> 2. **主会话侧**：`tb21_friction_scan.py` 机械扫描 + journal 对账；自报只是线索，**journal 是权威**（归因纪律 / S-16 证据基线）；对账后标注处置（立案/观察/已修/否定）。
> 3. 条目格式：`F-###`（日期 | run_id | 归因类：装置侧/设计内门/模型习惯 | 现象与一手证据 | 代价 | 处置）。
> 4. 立案的条目同步进 BACKLOG；已修的保留记录并标 `fixed`。

## 条目

### F-001 | 2026-09-13 | RUN-CLI-6aa6a868 | 装置侧（文档/契约）| fixed
**ACAF signer env 契约错位**：宿主侧读 `ORZ_ACAF_MANIFEST`，signer 子进程读 `ORZ_SIGNER_MANIFEST`（未设时回退 exe 同目录），`README.md` 启动环境只列三个变量、缺 signer 侧两个 ⇒ 手工部署极易配错且报错信息不指向根因。证据：`orz-signer fatal: signer manifest not found at …\orz-windows\signer-manifest.json`（env 已设 `ORZ_ACAF_MANIFEST` 仍找不到）。处置：本轮以「同时设两套变量」绕过（host 透传逻辑本身正确，`orz-loop/src/acaf.rs:175`）；**候选动作**：README 补全两组 env 或宿主 spawn 时自动注入 signer 侧变量——待立案。

### F-002 | 2026-09-13 | RUN-CLI-6aa6a868 | 装置侧（发布流水线）| fixed
**发布包 manifest 哈希过期**：`acaf/signer-manifest.json` 的 `binary_sha256` 与包内实际 `orz-signer.exe` 不符（包刷新时未重算）⇒ signer 每次启动即自哈希失败退出（fail-closed 设计行为正确），host 自愈重试每次再死。**代价**：RUN-CLI-6aa6a868 全程 229 次 `control_ticket_rejected`，模型被锁死在只读形态跑完 128 工具轮。处置：`orz-acaf-provision` 重算 manifest（旧件备份 `.bak-20260913`）；**候选动作**：发布清单加「二进制更新 ⇒ manifest 重算」核对步骤——待立案。

### F-003 | 2026-09-13 | RUN-CLI-6aa6a868 | 设计内门（0ac 正在修的病）| open → 0ac S3 验收样本
**确定性不可达无 cause**：229 次拒绝的失败载荷只有 `reject_code=signer_unreachable` + `io error: 管道正在被关闭 (os error 232)`，没有一条说明"manifest 哈希不匹配"或"env 缺失"——模型与排障者都只能盲猜，模型为此空转了全部只读轮次。这正是 0ac「机械层即时回报 + 单事件自描述」要治的形态；**处置**：作为 0ac S3 `cause` 字段的活体验收样本（修好后同形故障应一步报因）。

### F-004 | 2026-09-13 | RUN-CLI-6aa6a868 | 模型习惯（正面样本，登记不立案）
**模型行为正确的对照**：在"只能读、不能执行"的牢笼里，模型没有伪造进度，终稿如实声明「未落码、未跑门禁、未写入仓内文件」，并把可复核的设计 diff 全部列在会话里。与 F-003 合并看：机械层报因缺失时模型守住了诚实底线——归因纪律的正面证据。

### F-005 | 2026-09-13 | RUN-CLI-6aa6a868 / RUN-CLI-6aa6ac42 | 装置侧（观察）| open
**S2 产出核证全绿但有两处待复查**：① run3 仅 2 次 `content_anchor_mismatch` 编辑锚点重试（自恢复，代价小，暂不立案）；② S2 声称的"7 个 schema 文件 JSON 解析全通过"与"fixture 13+3 条"数目与实际提交（schema ×5、fixture 16）口径不一致——产物本身核验为真（门禁 `valid: true`），但自报计数不精确，**下轮自报要求附机械核对命令输出原文**。

### F-006 | 2026-09-13 | 传话流程 | 装置侧（流程）| fixed
**摩擦自报模板未随任务下发**：首轮传话按用户指示"仅告知"任务原话，run 末自报未发生，摩擦只能由主会话从 journal 反向提取。处置：本文件写入协议第 1 条已立；**下轮起传话模板固定附带自报指令**（见协议）。

## 统计

| 日期 | run | 摩擦条目 | 立案候选 | 已修 | 观察 |
|---|---|---|---|---|---|
| 2026-09-13 | RUN-CLI-6aa6a868 / 6aa6ac42 | F-001…F-006 | F-001②/F-002②（待立案） | F-001①/F-002①/F-006 | F-003（并入 0ac）/F-004/F-005 |
