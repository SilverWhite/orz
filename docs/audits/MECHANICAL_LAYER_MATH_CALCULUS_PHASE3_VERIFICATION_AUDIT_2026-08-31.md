# P2-10 阶段 3 验证记录（V1 FakeProvider 面 / V2 离线 102 runs / V3 冒烟与状态）

> 日期：2026-08-31；验证项：V1–V3（TODO P2-10 / BACKLOG 10 / 设计 §7）；依赖：
> I1–I6 + R1–R9（阶段 2 已闭合）。状态：**V1–V3 全部完成（2026-08-31）**：
> V1/V2 先行闭合；V3 S3 重建 + bookworm 冒烟 + S4 实机冒烟（temporal 四查询面
> 渲染 + 回归）于 2026-08-31 补齐。

## V1 — FakeProvider 测试面验证（信封/组合/求值器语义 + §6.6 近零提示 + F11）

### V1a 信封/组合/求值器语义（I5/I6）

- 新增 `orz-assurance/tests/v1_fake_provider.rs`：脚本化 FakeProvider 工具面
  （8 工具契约返回类型化 Ok/Fail 信封），经 `reduce` 归约验证：
  - Apply 保留信封三件套（summary/cap/payload/pointer）与 FilePtr 指针；
  - read→grep（路径 splice）、read→search_replace（GetPut 锚点携带 + 效应
    计数 1）、grep→read（match 选择 + span→offset/length）三条类型化透镜；
  - 不兼容 pipe 在验证与归约两层拒绝（`pipe_incompatible`），任一工具不执行；
  - Fail 短路保留 trace_id（receipt↔事件链同构的机械层一侧，§5.4/F11）；
  - 效应工具经归约仍计入 effect_count（候选计数/预算硬门不被绕过）。
- 证据：`cargo test -p orz-assurance --test v1_fake_provider` **8 passed**。

### V1b §6.6 模型熟悉度近零提示

- 新增 `orz-loop/src/blackboard.rs` 测试
  `temporal_render_is_self_describing_for_near_zero_prompt`：四查询面
  （now/recent/history/feature）渲染自描述——字段标签（u_prog=/u_err=/
  u_stuck=/err10=/succ10=/T̂=）、域字符串、入域/驻留均内联，无需外部词汇表
  即可单轮正确消费；fires 在所有查询面永不渲染（§9.7）。
- 证据：orz-loop lib **647 passed / 0 failed / 3 ignored**（含本测试）。

### V1c F11 receipt↔事件链逐段同构核对

- `assurance/run_event_journal_validation.py` 新增并注册
  `_verify_v02_receipt_event_isomorphism`（§5.4）：
  - gate 段：policy_denial 标记或结构化拒绝码（R2 词汇表镜像
    `_is_denial_code_v02`）即 gate 证据；有 tool_started 时须先于完成；
  - execution 段：非 gate 失败完成须有同 (run, tool, call_id) 的先前
    tool_started（1:1 配对），完成先于启动即 receipt 顺序破坏；
  - 运行级配对：正常终止（run_finished/failed/cancelled）的 run 中每个
    tool_started 须有 tool_completed；run_invalidated（墙钟超时杀）的
    in-flight 调用按 S4 豁免不要求补终止；
  - 同一 call_id 至多一个 tool_completed（receipt 段不重复）。
- 测试：`runtime/tests/test_run_event_journal_validation.py` 新增
  `ReceiptEventIsomorphismCrossCheckTests`（9 用例）；verifier 全量
  **245 passed**（原 236 + 9）。102-run 语料复验：新检查 0 新增错误
  （语料 22 个无 start 的完成全为 gate 拒绝：content_anchor_mismatch 15 +
  web_fetch_candidate_cap_exceeded 7；6 个无完成的 start 全为
  run_invalidated 墙钟豁免）。

## V2 — 离线 102 runs 复验（四对照门 + §9.8 聚类对照 + 零误干预）

### V2a 四对照门（lif_replay 扩展）

- `orz-assurance/examples/lif_replay.rs` 新增：
  - C1 时间打乱扩展到 deny/stall/slow（err 沿用原 10 次种子置换公式）；
  - C2≡C4 参考（无复位/无不应期膜电位 = 单极点一阶 EMA）+ C3 参考
    （τ→∞ 纯计数，保留复位/不应期）——同一更新律重放，与引擎同基
    （run 起点归一）对拍；
  - 逐决策轮特征轨迹导出（u_prog/u_err/u_stuck/T̂/err10/succ10 + 语义域）。
- 产出：`../../存档/root-artifacts-2026-09-06/LIF_102RUNS_REPLAY_2026-08-31_V2.json`（102 runs / 4157 决策点）。
- **确定性对拍**：err/deny/stall/slow 四通道参考 LIF fires 与引擎
  `reference_parity_fail_runs = 0`（102 runs 全一致）——参考实现与生产引擎
  同构，对照结论可信。
- 逐通道结论（设计 §4.7 判据）：

| 通道 | runs/fires | C1 时间打乱 | C2≡C4 膜/EMA | C3 计数 | 结论 |
|---|---|---|---|---|---|
| err | 0 / 0 | — | — | — | 未决（生产谓词下本批无触发；旧谓词首轮证据见 §9.6.5） |
| deny | 3 / 4 | 1 变 / 2 不变 | 1 异 / 3 | 3 异 / 3 | 时间结构 + 泄漏在起作用（部分通过） |
| stall | 8 / 8 | 8 变 / 0 不变 | 2 异 / 8 | 8 异 / 8 | 间隔时间结构主导（C1/C3 通过；C2 部分） |
| slow | 2 / 2 | 1 变 / 1 不变 | 0 异 / 2 | 0 异 / 2 | 本批 fires ≡ 膜电位 ≡ 计数（样本极小，建议降级为连续电位） |
| stuck | 0 / 0 | — | 0 膜跨阈 | — | 未决（峰值 θ 比 0.6087 < 1，非线性面未做功） |

- 解释：err 通道在 R1/R2 生产谓词下本批 0 fires（web_fetch_candidate_cap
  移入 deny 后无宿主级错误残留），四门无触发可判；slow 仅 2 run 且
  fires=膜=计数——按 §4.7「任一不通过 → 该通道降级为普通统计特征」，slow
  的 fires 判定在本批证据上不承载超计数信息（样本极小，留档，不删除连续
  电位特征面）。fires 无注入面（§9.7），降级不影响模型表面。

### V2b §9.8 聚类对照 + 零误干预

- 新增 `assurance/lif_v2_cluster_analysis.py`（sklearn k-means k=3、
  random_state=0，z 归一，仅探索用）；产出
  `../../存档/root-artifacts-2026-09-06/LIF_102RUNS_CLUSTERING_2026-08-31_V2.json`：
  - 4157 决策点，簇 {0: 771, 1: 254, 2: 3132}；簇均值：簇 0 u_err 0.857/
    succ10 0.822（压力类比）、簇 1 u_prog 0.086/succ10 0.237（低进度/启动
    类比）、簇 2 u_err 0.106/succ10 0.839（干净正常）；
  - 语义域均值与 §9.8 参考结构对应：Normal (u_err 0.249/succ10 0.832)
    ↔ C1；Pressure (u_err 2.416/u_stuck 2.497/succ10 0.45) ↔ C0（§9.8
    u_err 2.61/u_stuck 3.8/succ10 0.75）；LowProgress (u_prog 0.276)
    ↔ C2——四类分离成立；数量差异源于 R1 生产谓词收窄 err 后压力域近空
    （6 点 vs §9.8 旧谓词 1233），域=事实描述非结局信号结论不变；
  - 零误干预：102 条事件链 0 个 fire 事件类型、schema 0 处 temporal_fire
    （`zero_misintervention.pass = true`）——fires 仅内部留痕成立。

## V3 — S4 冒烟复验与状态同步

- **S3 重建（2026-08-31 完成）**：`rust:1.97-slim` 容器增量构建，Linux musl
  三件套 orz 106,905,224 B / orz-signer 1,390,072 B / orz-acaf-provision
  1,207,936 B（00:27 HKT，编译 8m03s，日志
  `D:\tb-eval\orz-linux\build-20260831.log`）；bookworm 容器冒烟符合预期
  （marker 字符串在二进制内、provision usage / signer manifest 缺失 / orz
  `--real` TTY 要求 API key 的启动错误路径、三件套 `ldd` 静态链接）。
- **S4 实机冒烟（2026-08-31 完成）**：harbor + 真实二进制
  （`tb_agents.orz:Orz`，debian:bookworm-slim 任务容器，deepseek-v4-flash），
  本地探针任务 `temporal-partition-smoke`（job
  `D:\tb-eval\jobs-official\final-smoke-2026-08-31`）：
  - **1/1 reward 1.0、0 异常、总时长 64s**（回归：真实容器安装 ACAF 三件套 +
    真实任务完成，answer.txt 逐字节写入）；
  - **temporal 分区渲染端到端**：模型按提示精确发起 4 次
    `blackboard_read section="temporal"`（selector=now / recent k=5 /
    history k=5 / feature name=u_prog k=5），事件链 4 条
    tool_completed（exit 0、section=temporal），trajectory 记录模型消费到的
    渲染内容与设计 §3 一致：域标签 `start`、`入域 0 轮`/`驻留 2 轮`、
    `u_prog=0.00 u_err=0.00 u_stuck=0.00 T̂=8.0s err10=0.00 succ10=0.00`、
    history `(无迁移)`、feature 单值 `0.000`；
  - **零误干预保持**：全链 0 个 `temporal_fire` 事件（fires 仅内部留痕）；
  - **事件链机械门**：`run_event_journal_validation.py` 对本冒烟 journal
    **0 错误**（含 F11 receipt↔事件链逐段同构、policy_denial/failure_target/
    mechanical_audit/control_tickets 全量交叉检查）；
  - journal 落盘：`D:\tb-eval\gsa-volumes\final-smoke-2026-08-31\
    ea67733e-…\runs\RUN-CLI-6a94cc24\events.jsonl`。
- 状态同步（2026-08-31 完成）：V1/V2 证据入本审计；V3 证据入本审计；
  TODO（I1–I6 勾选 + V3 勾选 + 计数 38 → 32）/ BACKLOG / 设计 §7 /
  索引 v2.36（AUTH `partial` → `implemented`）。

## 证据汇总

| 项 | 结果 |
|---|---|
| orz-assurance lib | 192 passed（reducer 14，含 R5 新增 7） |
| orz-assurance v1_fake_provider | 8 passed（V1a） |
| orz-loop lib | 647 passed / 0 failed / 3 ignored（含 V1b） |
| Python journal 校验 | 245 passed（含 F11 新增 9） |
| 102-run 复验 | 4157 决策点；err 0、deny 3 run/4 fires、stall 8、slow 2、stuck 0（峰值 θ 比 0.6087）；四通道参考对拍零失配 |
| 聚类对照 | 4157 点 k=3；语义域均值与 §9.8 C1/C0/C2 结构对应 |
| 零误干预 | pass（事件链/schema 无 fire） |
| 静态门 | clippy 仅既有基线（orz-assurance 1 条）；`git diff --check` 干净（2026-08-31 复核） |
| S3 重建 | 三件套 00:27 HKT（orz 106,905,224 B / signer 1,390,072 B / provision 1,207,936 B），8m03s 增量；bookworm 冒烟符合预期 + 三件套静态链接 |
| S4 实机冒烟 | 1/1 reward 1.0、0 异常；temporal 四查询面渲染端到端一致；事件链 verifier 0 错误；0 temporal_fire |

## 未闭合（转 V3 或后续）

- ~~V3 S4 冒烟复验~~——**2026-08-31 已闭合**：S3 重建 + bookworm 冒烟 +
  实机冒烟（temporal 四查询面渲染 + 回归）完成，见上。
- slow 通道 fires 判定：本批 2 run 证据为 fires≡膜≡计数，建议后续以更大
  样本复核或按 §4.7 降级（连续电位保留）。
