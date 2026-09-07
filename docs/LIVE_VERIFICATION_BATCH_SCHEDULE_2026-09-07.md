# S3/S4 集中实机验证批排期（2026-09-07）

> **状态**：排期定稿，同日用户裁决放行（本批零代码改动，仅本排期文档 +
> BACKLOG/TODO 登记批；实施自阶段 0 起）。
> **上级**：[`BACKLOG 0o`](BACKLOG_AND_PRIORITIES.md) /
> [`TODO P0-0o`](../TODO.md)。
> **定位**：本文是本批执行排期、任务矩阵与判据对照的执行权威；不改变各
> 覆盖项的设计权威（ADR-0010 §14.x 与各设计文档），闭合登记仍写回各条目
> 自身的 BACKLOG/TODO 小节与审计文档。
> **源冻结基线**：orz `a1b73aeb`（父仓库 `fbfd601`，两侧工作区净）；本批
> orz 侧唯一预期改动 = 版本 bump 一次提交。

## 0. 用户裁决（2026-09-07，逐字口径登记）

1. **版本号 0.3.1 即可**——「毕竟这不是修完的版本，是过程中的验证版本」。
2. **放行纪律**：「只要事实可严格放行就可以，不必考虑流程严格逐项放行，
   毕竟这是工程类项目」——据此，P2-11×3 / P2-12 / P2-13 B4 / P2-14 各项
   S4 搭批次 journal 就地核验闭合，不再逐项单独要放行门；闭合仍以判据
   事实为准，未触达判据不硬闭合（见 §1 边界）。
3. **核心方法**（本批组织原则）：一次双平台重建（同一 orz 源）+ 三批次
   实机跑批 + 一份 journal 喂多个判据 + 统一分析一次性收口。

## 1. 覆盖范围与边界

**覆盖（预期闭合项）**：

- 0m GSA-SESSION-VOLUME：S3 接线复验 + S4 收口。
- 0b FUS-BENCHMARK-FULL-EXEC：验证②（musl 重建）③（make-doom reward）
  ④（交叉题）。
- 0d 后续 3 / 4 / 5 的 S4 复验。
- 0j THIN-HARNESS-V2：W1-R1 S4 复验；W3-R3 A/B 验证搭小样本成绩（实现
  余项另线不动）。
- 0l WINDOWS-HIGH-NIST：⑥ agent 主载跑批 + 记账、⑦ 模型侧 §7 事件面判据。
- TER：T2.3（后台存活实机）、T2.4（同步接线）、M3 T3.1–T3.5。
- P2-11×3（PULL 自描述 / retryable / 依赖图）S3 重建 + S4。
- P2-12：S4 复验（+ 闭合时 ADR-0010 转录登记）。
- P2-13 B4：S4 复验（S3 已建，本批翻新注记）。
- P2-14：S3 重建 + S4 实机复验 + 遥测。

**不混入（保持独立门/另线）**：0b ⑤（89 题 5 批大跑批，独立用户门）、
0j W3-R3 实现余项（实体 id 形态/分区命名/注册表 Rust 形态）、0n
（approval，S1 设计前置）、P1/P3 非实机项。

**边界**：

- 批次期间 orz 源冻结，不主动叠任何实现改动；跑批暴露需修复项时按既有
  纪律修复（独立提交登记于收口批）并局部重跑受影响判据。
- 0d 后续 3「骑过断连抖动」与 0d 后续 5「中段解码错误」依赖真实故障
  复现：批内未复现则登记「retry 面单测已绿 + 故障未复现」观察，顺延
  不硬闭合。
- VM 侧既有登记边界沿用（如 mteb 提交无 live 核验）。

## 2. 阶段 0：一次性双平台重建（T0）

### 2.1 版本 bump

- orz workspace 版本 0.3.0 → **0.3.1**，独立提交（本批唯一预期 orz 提交）。

### 2.2 Linux musl 三件套

- 契约：ORZ-BUILD-MOUNT-001（挂父仓库 `/orz` + 工作目录 `/orz/orz`、
  前置挂载守卫、构建日志留存）。
- 产物：`D:\tb-eval\orz-linux` 三件套（orz / orz-signer /
  orz-acaf-provision）；验收 = BUILD EXIT=0、musl 静态（无 PT_INTERP、
  无 ld-linux 字符串）、bookworm 容器冒烟三件加载执行。
- 直接闭合：**0b 验证②**；**P2-14 S3 重建**；**P2-11 三项 S3**
  （PULL 自描述 / retryable / 依赖图，翻新至当前源）。登记翻新注记：
  P2-12 / P2-13 的 S3 为旧源所建，本二进制顺带覆盖其后全部已提交批次。

### 2.3 Windows 三件套 + VM 同步（= TER T2.4）

- Windows x86_64 三件套重建 → VM 同步（Program Files + `C:\s4\tools`）
  + keystore/signer 复检 + runner 配置收敛（不再补开 S5-2）。
- 验收：vm-agent DryRun 全对 + enforcement-probe 墙内全绿 → 闭合
  **TER T2.4**，并为批次 W1/W2、0l ⑥⑦ 就绪新二进制。

## 3. 阶段一：批次 0 本地实机件（T1，可与构建并行）

- **0m S3 接线复验**：`.gsa` symlink 会话卷实机构造（评测容器会话卷挂载
  形态）+ 终端截断补读链（`session/terminal/*.log` 只读窗口）+
  run_tests 输出窗口端到端；GAP-GSA-SYMLINK-STALE-TEST 收口注记演进。
- 产物：0m S3 复验记录；S4 收口并入 §8 统一收口批。

## 4. 阶段二：Windows VM 批次（T2 / T3）

### 4.1 批次 W1：确定性短批（→ TER T2.3）

- 载体：`ORZ_FAKE_SCENARIO` fake 场景驱动（orz `dd5b1dac`）+ idle-kill
  journal 生产者（orz `35db6741`），均已就绪。
- 断言：① auto-bg 后台进程在 Job/LOW IL 下跨调用存活且输出持续落盘；
  ② 完成提醒可达；③ idle-kill 场景 `tool_running(status=idle_killed)`
  journal 事件断言。
- 产物：实机脚本 + journal 证据 → **TER T2.3**。

### 4.2 批次 W2：任务批（→ 0l ⑥⑦ + TER T3.1–T3.5）

- 任务：TB2.1 错题集 9 题 3 题/批——**chunk1 复跑**（gcode / make-doom /
  mteb；基线 = chunk1-0303 的 0.3.0 pre-TER journal）+ **chunk2 续跑**
  （path-tracing / train-fasttext / adaptive-rejection-sampler）+
  **chunk3**（余 3 题）；沿用 S4_PROGRESS §16.7/§16.8 已接线 runner
  （agenthighnist 主载、bootstrap 取凭据 + `--env-file` 注入）。
- 开关：`ORZ_F6_PUSH=on`（mteb 批判据，其余批顺带记账）+ runner 施加
  `ORZ_MAX_WALLCLOCK`（官方 agent_timeout_seconds 为唯一评测墙钟）。
- 判据映射见 §6 / §7（T3.1–T3.5、0l ⑥ 记账、0l ⑦ §7 判据 1–6）。

## 5. 阶段三：Linux/TB2 容器批（T4）

任务矩阵（容器管线沿用 D:\tb-eval 既有 harbor 链路，musl 三件套换新）：

| 任务 | 主判据 | 搭车判据 |
|---|---|---|
| make-doom-for-mips（官方墙钟全量） | 0b ③ reward | P2-14 S4（真实压缩 marker 遥测）；P2-13 B4（长会话遥测 + 折叠读取/展开/恢复）；P2-12 S4（failure_agg 渲染对账） |
| compile-compcert | 0b ④（build/run 类） | 通用统计 |
| hf-model-inference | 0b ④（网络类） | P2-13 B4 web 通道 A/B；0d 后续 3 断连窗口观察 |
| dna-assembly（备选 dna-insert） | 0d 后续 5（解码重试不杀 run） | — |
| 全批通用 | 零真实 400 + 命中率 ≥90%（0d 后续 3/4、0j W1-R1）；哨兵有界 ≤3（0d 后续 4）；P2-11 三项 S4 | P1 DeepSeek live 通道证据顺带登记 |

## 6. 证据 → 闭合对照表

| 实机证据 | 闭合项 | 关键判据（明细见 §7） |
|---|---|---|
| Linux musl 重建 + 冒烟 | 0b ②；P2-14 S3；P2-11×3 S3（+P2-12/P2-13 S3 翻新注记） | EXIT=0、musl 静态、接线符号命中、容器冒烟 |
| Windows 重建 + VM 同步 | TER T2.4 | DryRun 全对 + enforcement-probe 全绿 |
| 批次 0 本地件 | 0m S3（S4 并收口批） | 会话卷实机构造 + 双窗口端到端 |
| 批次 W1 | TER T2.3 | 跨调用存活 / 落盘 / 提醒 / idle-kill 事件 |
| 批次 W2 | TER T3.1–T3.5；0l ⑥；0l ⑦ | §7 C 组 |
| 批次 L（make-doom） | 0b ③；P2-14 S4；P2-13 B4；P2-12 S4 | §7 D1 |
| 批次 L（compcert / hf） | 0b ④；P2-13 B4 web A/B | 交叉题通过 |
| 批次 L（dna） | 0d 后续 5 | 解码错误不杀 run |
| 批次 L（通用统计） | 0d 后续 3/4；0j W1-R1（journal 半边）；P2-11×3 S4 | §7 D5 |
| 批次 O 离线 | 0j W1-R1（回放半边） | §7 E |

## 7. 判据明细（逐项，供分析脚本机械化核对）

### A. 重建闭合（免跑批）

- **0b ②**：musl 三件套重建完成 + 契约核证（时间戳、静态性、冒烟）。
- **P2-14 S3**：本二进制含 v0.3 装配/快照接线符号（render_*_snapshot /
  summary v0.3 A–E 块），冒烟通过。
- **P2-11×3 S3**：增量头 / `retryable_for_code` / `dep_graph` 接线符号
  在二进制内 + 冒烟（翻新登记，无新语义）。
- **TER T2.4**：VM 三件套同步 + keystore/signer 复检 + runner 收敛 +
  DryRun 全对 + enforcement-probe 全绿。

### B. 批次 0

- **0m S3**：`.gsa` symlink 会话卷实机构造下，终端截断补读链与
  run_tests 输出窗口端到端可达；GAP-GSA-SYMLINK-STALE-TEST 注记演进。

### C. 批次 W2（Windows VM 任务批）

- **T3.1（gcode）**：渲染类前台命令满 180s 预算点收 `tool_running`
  中间状态并后台化（或无输出 300s idle-kill + 提醒）；事件链含
  tool_running。
- **T3.2（make-doom）**：无 runner 自加 840 硬杀（`--timeout` 唯一官方
  墙钟）；连通探测 ≤2s/目标收敛；blackboard `section=processes` 可见
  运行中探测并可 kill（进程行 pid 中断语义）。
- **T3.3（mteb）**：HF 探测 ≤2s 判定失败；`ORZ_F6_PUSH=on` 下 push 提示
  ≤4 次/run（实现 ≤3、verifier ≤4 兼容）+ `budget_cue_injected` 逐条
  可审。
- **T3.4（W-F13）**：vm.js 阅读轮数 20+→≤2（与 chunk1-0303 基线对比
  统计）；长输出补读经 output_object_id / read_file 直读落盘 log，
  不再触 `.gsa`。
- **T3.5 + 0l ⑦**：§7 判据 1–6 新 journal 复验登记（err 升压 / slow-stall
  / LIF / 降级链 / 假成功-事件面 / deny 通道）+ F6–F10 缺口状态更新
  （F6→T1.8/T2.1 已修、F7→S5-2/T1.2 已修、F9 已驱动级修复，按新 journal
  复认）。
- **0l ⑥**：9 题 agent k=1 主载成绩 + observation compliant + 缺口记账
  核对表 → ⑥ 勾选（⑥.1 机器侧已闭）。

### D. 批次 L（Linux/TB2 容器批）

- **D1（make-doom → 0b ③ + P2 遥测）**：0 异常、无 400、订单→票据链
  正常、reward > 0（官方墙钟全量）；压缩真实触发下——P2-14：marker 实际
  字符分布（≤20K 档）、块 B/C 溢出频率、压缩后 blackboard_read 跟随调用
  频率、restore 后 marker 逐字节可用；P2-13 B4：折叠态读取与显式展开
  （domain/round 参数）、跨 prompt 恢复、长会话遥测；P2-12：注意事项槽
  failure_agg 聚合行的 digest 计数 / 首末时间 / 错误码集合与事件链
  failure_target 对账一致。
- **D2（compcert / hf → 0b ④）**：build/run 类与网络类交叉题通过
  （reward 口径按 TB2 verifier）。
- **D3（hf → P2-13 B4 web A/B）**：web 通道任务下黑板折叠/存档/疲劳
  遥测正常。
- **D4（dna → 0d 后续 5）**：dna 类场景中段解码错误不杀 run（重试 1 次
  继续）、零 400；未复现解码错误则按 §1 边界观察登记。
- **D5（通用统计）**：全批零真实 400；命中率（provider 口径）≥90%；
  单 run 哨兵触发有界 ≤3 且显式终止可观测（run_invalidated）；0d 后续 3
  的 180s retry 窗口以 transport_retry 事件 + 断连骑过为判据（未复现
  断连则观察登记）。
- **D6（P2-11×3 S4）**：blackboard_read 响应增量头（分区版本计数 +
  temporal 迁移摘要）与 temporal 一次返回 ≤1KiB 实机可见；Fail 信封
  retryable 位分布与错误码族一致（未知码 false）；`section=deps`
  live PULL ≤8KiB + 徽章、read→write 锚点边在真实 run 建图、
  dep_graph 事件字段随 ToolCompleted 落 journal。

### E. 批次 O（离线件）

- **0j W1-R1 S4**：EGFP / sam-cell-seg 真实 span 回放静默 + 构造真循环
  触发（离线可做）；journal 半边（零 400 + 命中率 ≥90%）由 D5 供给。
- 判据分析脚本化：journal → 指标表一次性出（400 计数 / 命中率 / 哨兵 /
  retry / tool_running / marker 尺寸与溢出 / blackboard_read 跟随率 /
  deps 与增量头抽样 / retryable 分布 / §7 通道）。

## 8. 阶段四：统一收口（T6，纯文档轮）

1. 逐项审计/复验记录落 `docs/audits/`（0m、0b、0d、0j、0l、TER、
   P2-11/12/13/14 各一份或按项合并，事实可严格闭合即闭合）。
2. BACKLOG（0b/0d/0j/0l/0m/§11/§12/§13/§14）、TODO、TODO2、BACKLOG2、
   CLI_PROJECT_INDEX 状态与入口同步；计数按各条目入账。
3. P2-12 闭合时 ADR-0010 转录登记；TER T3.5 含 ADR/BACKLOG2/TODO2 收口。
4. manifest 重算 + 门禁 Exit 0。

## 9. 前置条件与风险

- **DeepSeek key**：本地跑批与 VM `ORZ_AGENT_KEY_FILE` 通道均需确认有效
  （2026-09-03 用户轮换后走 key 文件覆盖）。
- **Windows VM 状态**：worker restart / sync / 磁盘例行核验（桥链路
  2026-09-03 已闭环）。
- **容器管线**：D:\tb-eval harbor/docker 链路可用性跑前确认。
- **不可控故障面**：断连抖动（0d 后续 3）与中段解码错误（0d 后续 5）
  批内未复现则观察登记、顺延不硬闭合（§1 边界）。
- **源冻结纪律**：批次期间不主动叠实现改动；修复类改动独立提交并局部
  重跑受影响判据。

## 10. 执行顺序

1. T0a orz 版本 bump 0.3.1 提交。
2. T0b Linux musl 三件套重建 + 冒烟（闭合 0b ② / P2-14 S3 / P2-11×3 S3）。
3. T0c Windows 三件套重建 + VM 同步 + DryRun + enforcement-probe
   （闭合 TER T2.4）。
4. T1 批次 0 本地实机件（可与 T0b/T0c 并行）。
5. T2 批次 W1 → T3 批次 W2（chunk1 → chunk2 → chunk3）。
6. T4 批次 L（make-doom → compcert → hf → dna，容器资源允许即串行）。
7. T5 批次 O 离线分析（journal 齐后统一出表）。
8. T6 统一收口（文档轮）。
