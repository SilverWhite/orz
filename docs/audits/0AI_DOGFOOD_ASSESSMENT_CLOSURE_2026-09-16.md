# 0ai 狗粮考核测试收口（RUN-CLI-6aa999d6，2026-09-16）

> 入口：TODO **P1-0ai** / BACKLOG **0ai** / 索引 `GAP-HEAVY-FILE-SPLIT`；
> 前置与启动见 [`052_CARRIER_REBUILD_AND_0AI_RELEASE_2026-09-16`](052_CARRIER_REBUILD_AND_0AI_RELEASE_2026-09-16.md)。
> 执行者＝orz 0.5.2 载体；题＝0ai 重文件拆分；`ORZ_MAX_WALLCLOCK=0` 无墙钟；
> 隔离工作区 `D:\tb-eval\dogfood-0ai-20260916\cli`（orz `a580eb08`）。
> **单轮/少样本如实标注；不作架构结论**（2026-09-16 用户裁决口径）。
> 产出（拆分实施件＋agent 报告）**在工作区未提交**——是否按正常批次合回主仓
> 留用户裁决。

## 0. run 概况

- run_id＝`RUN-CLI-6aa999d6`；窗口 2026-09-15T19:17:42 → 20:04:29 UTC
  （**46 min 47 s**）；`run_finished{status: completed}`，234 工具轮，
  2,097 事件（journal 1.8 MB）；`.gsa` 本地增长 21.0 MB。
- 产出：`host_exec.rs`（9,710 行）→ `host_exec/` 七模块（合计 9,852 行＝
  原文 9,710 ＋ 胶水 142）：`mod.rs` 24（`pub(crate) use` 保旧路径）/
  `tool_run.rs` 6,784 / `failure.rs` 917 / `facts.rs` 855 / `serp.rs` 817 /
  `dep_graph.rs` 269 / `candidate.rs` 186。改动面恰＝`D host_exec.rs` ＋
  `?? host_exec/`；agent 报告＝工作区
  `docs/audits/0AI_HOST_EXEC_SPLIT_REPORT_2026-09-16.md`（含摩擦 F1–F8）。

## 1. 0ai 验收（主会话独立复核，非 agent 自报）

| 判据 | 读数 | 结论 |
|---|---|---|
| 单文件 ≤10,000 行 | 最大 `tool_run.rs` 6,784 | ✓ |
| 职责域单一 | 7 模块按域（编排/失败/事实/SERP/候选/依赖图/入口） | ✓ |
| 行为不变 | orz-loop `--lib` **818/0/3**（主会话独立复跑，与拆分前基线同值）；**clippy 52 → 52**（基线 `a580eb08` 对照实测，零新增——agent 沙箱无 protoc 跑不了，此判据由主会话闭合）；orz-host 串行 **325/0/5**（主会话复跑；agent 并行读数中的 8 失败〔4 既有 `grok_home`＋4 存疑〕复跑归因为**并行/环境抖动，非拆分回归**） | ✓ |
| 事件序列/journal 链不动 | 拆分为 orz-loop 内部代码组织，不触 journal 面事件写点 | ✓ |
| fmt | agent 实测退出 0 | ✓ |
| 外部面 | `lib.rs mod host_exec;` 未动、`crate::host_exec::*` 旧路径经 re-export 保留 | ✓ |

两处如实登记的偏移（agent 报告 §5）：跨模块项 `pub(crate)` 放宽（沿
CONTROLLER-SPLIT 先例）；测试专用导入改 `#[cfg(test)] use`（净 +14 行胶水）。

## 2. 狗粮判据读数（0ah S1 常驻滑窗，单轮）

**四件套**：

1. **命中率（provider cache）**：hit 17,250,176 / miss 587,927 ＝ **96.70%**
   （判据 ≥90% ✓）。
2. **每窗重付**：5 窗，折叠前 view 160,294–173,143，折叠后 view
   **20,755–28,792**（重建视图文付量）。
3. **驱逐次数**：**5 次**/234 轮（轮跨度 28/74/99/140/192）；旧形态无同轮
   对照 run——**增幅判据（≈+23%）本单轮不可判**，不作结论。
4. **重读 offset**：`read_file` 38 次/234 轮，**全部全文读、零 offset 续读**
   （本任务无重读税）。

**提醒面与压缩**：`context_scale:first_fold` @轮 52（161,536 → 21,195，水位
【0.2M/10M】）按「首次驱逐一次性固化提醒」触发 ✓；**500K 纯提醒** @轮 130
（actual_context_tokens=501,845，`opens_window=false` ✓ 纯提醒语义如实）；
900K 未达未触发；`context_compressed` **0 次**（机械/语义轨均未开窗——峰值
实际上下文 ~500K）；950K 硬截留未触发。尾部无塌缩（末次折叠后正常收尾）。

**存档三键**：**N/A——`.gsa/archives/` 0 件**。定性：增量归档/三键存档接线
在 **ACP 会话车道**（`orz-host/acp_server.rs` `incremental_archive_due`，
「会话关闭＋500K 里程碑」两点），headless `-p` 车道（狗粮考核所用车道）不
经过该路径 ⇒ 本 run 跨 500K 里程碑而无归档产出。主仓 `D:\CLI\.gsa` 同样无
archives（历史一致，非本 run 特异）。

**附带实证**：`host_resource_snapshot` ×4 入 journal（0z 修复经 0.5.2 载体
端到端生效；TB21 R0 时为 13 WARN/0 事件）；ACAF fail-closed 票据
188/188；orientation 5 次（50 轮阈值口径 ✓）；`reclaim_performed` 3。

## 3. 考核测出的框架缺口（本 run 增量发现；编号留台账）

1. **`blackboard_write` 权限层 deny（旧摩擦残余；2026-09-16 用户指认旧账＋主会话逐帧实证）**：
   工具**已声明**（controller.rs:3100 无条件注入请求面，模型实际发起多次调用，
   journal 17 处痕迹）→ 每次均 `permission_requested{risk: ReadOnly}` →
   **`permission_decision{decision: deny}`**——设计口径「ReadOnly 类所有策略
   自动放行」未在权限桥落地 ⇒ 计划/笔记面恒空（agent 报告 F5 的根因；模型
   自己在 notes 里写下「blackboard_write 三次被门禁拒」）。两处伴生症状：
   探针面 `main_agent_work_tools` 仍缺该工具（探针注册面与请求面脱同步）；
   `plan_write` 事件族 0 条（调用未达执行层）。旧账脉络：上一代狗粮 run
   （`RUN-CLI-6aa7e0aa` 深审「模型面无黑板写工具」）催生 0ae D0 加工具＋
   QUAD 审查补窗口轮消费，但 `-p` 车道权限桥的放行缺失使能力仍未达模型——
   **「符号在位≠端到端接线」族第三例**。
2. **增量归档/三键存档 `-p` 车道不可达**（§2 存档三键 N/A 的根因）——
   「长单对话不结束就没有存档」的闭合只覆盖 ACP 车道。
3. **门禁冻结克隆失效面（agent F2，严重）**：可编辑安装指向原始树 ⇒
   `python scripts/check_repository.py` 在克隆内崩（或同形路径时**静默校验
   错误的树**）；绕行 `python -m scripts.check_repository`。
4. agent 报告 F1–F8 全文见工作区报告 §7（clippy protoc 阻塞／控制台 GBK／
   折叠台账无摘要／TER 旧文断链×7／终端工具面缺失／orz-host 并行抖动）。

## 4. 用户裁决与执行（2026-09-16）

- **产出合回主仓**：已执行——orz `b682a67f`（7 文件 +4603/−4461，git 识别
  `host_exec.rs → host_exec/tool_run.rs` rename 71%）；agent 报告随批入库；
  合回后主仓复跑 orz-loop 818/0/3＋fmt 干净。**0ai 闭合**（33 → 32）。
- **三项摩擦直接立项**（用户 2026-09-16 指示）：**0aj**（`blackboard_write`
  权限层 deny，旧摩擦残余，P1）／**0ak**（增量归档 `-p` 车道不可达，P2）／
  **0al**（门禁冻结克隆静默校验错误树，P1）；计数 32 → 35。
- orz-host `grok_home` 4 条既有失败：维持独立立项候选（未立项，留裁决）。
