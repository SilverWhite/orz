# 205 批：0cr S3 收口——Harbor 全目转换＋silverwhite 发布＋0bz S4 对账＋官方记分卡（2026-10-07）

> **用户令**：「请进行S3收口吧」＋侧话裁决（[`ADJUDICATION 交接`](../../../tb-eval/scbench/0cr_official/ADJUDICATION_harbor_publish_as_friction_artifact_2026-10-07.md)）：
> 「按照B做吧……这一部分也上传至harbor吧，但明确标注为摩擦寻找产物」；「虚拟机部分可以清掉了
> （Windows 摩擦 VM 已弃用）」；「隔壁窗口并行，写入前先确认文件占用」。
> **性质**：收口批——零源码；计数不变 **62**（隔壁 203 批 0cu 立项 61 → 62 在先，本批不动计数）；
> 未提交。
> **结论先行**：**0cr S3 全部达成**——①Harbor 全目转换 **36/36**（34 题全链 rc=0＋6 题
> oracle 缺陷脚注 rc=4）；②**36 任务＋1 数据集发布**（`silverwhite/*`，`--public`＋双标签
> `friction-hunting-artifact`／`not-official-scores`，README 声明非官方成绩）；③**0bz S4
> 离线对账达成**（官方轮 36 题全语料：**137/137 个 +2 事件恰在尾槽分歧、尾槽前分歧 0＝
> 旧第 2 针归零**，+2 hit 中位 116K vs 修复前 ~12.8K）；④官方记分卡口径不变
> **134/196**（harness 原生，Harbor 侧不产生第二记分卡）；⑤装置事件三项（CRLF／引擎 500／
> loop 误杀）全部如实登记。

---

## §1 Harbor 全目转换（36/36；官方 `scb_to_harbor.py` 管线，pin `38d627e`，org=`silverwhite`）

- **34 题全链通过（rc=0）**：生成＋静态校验＋build 冒烟＋oracle 验证全绿。
- **6 题 oracle 缺陷脚注（rc=4，oracle=测试非参考解；转换忠实度发现）**：
  | 题 | rc=4 性质 | 明细 |
  |---|---|---|
  | dynamic_buffer | 已知缺陷（KNOWN_ISSUES ck4） | 预期脚注兑现 |
  | eve_market_tools | 已知缺陷（KNOWN_ISSUES ck1–3） | 预期脚注兑现 |
  | **env_manager** | **新发现** | ck3 strict_pass_rate=0.984（参考解差约 2 测试；agent 跑批 5/5 全绿对照） |
  | **file_backup** | **新发现** | ck2 strict=0.84／ck3 strict=0.68（参考解真实缺陷面较大；agent 4/4 全绿对照） |
  | **mvvault** | **新发现** | ck5 strict=0.9946（差 1 测试） |
  | **test_translator** | **新发现** | 全档 strict 0.69–0.75（参考解挂两三成测试；agent 2/8 塌陷与该题难度面互证） |
- **KNOWN_ISSUES 三题在本 pin 未显形**（oracle rc=0）：eve_industry／file_merger／execution_server
  ——`38d627e`（fix/harbor-port-issues-30-31）后已修或环境相关，脚注如实记。
- **装置事件全录**：①loop 1 题 `problems36.txt` CRLF 行尾致 34 题秒败（`write_text` Windows
  行尾转换；末行 xjq 无行尾符故独活——真机教训入台账）；②loop 2 停止时误杀 dag_execution
  oracle 阶段（loop 3 `--force` 重跑 rc=0）；③**Docker Desktop 引擎 500**（`_ping` 全路由
  500，三题 build 连败）——重启 Desktop 恢复（29.6.2），textdrop／trajectory_api 重试 rc=0；
  ④forge apt `Unable to fetch some archives` 网络瞬断两轮、第三轮 rc=0。

## §2 发布（friction-hunting artifact 标注，按侧话裁决）

- **36 任务**：`hub.harborframework.com/tasks/silverwhite/<题>`（`--public`，36/36 URL 回执；
  单任务 hash/rev/files/size 全表在 `publish.log`）。
- **数据集**：`silverwhite/slopcodebench-friction`（36 任务 manifest＋README 摩擦声明，
  hash `71e7aa6291e4`，rev 1）→ `hub.harborframework.com/datasets/silverwhite/slopcodebench-friction`。
- **标注（裁决原文兑现）**：`--tag friction-hunting-artifact --tag not-official-scores`；
  README 声明＝「Friction-hunting artifact from an unofficial harness-native evaluation round
  (orz, deepseek-v4-flash, k=1). NOT an official score submission. …Oracle/build results …
  are conversion-fidelity footnotes, not agent scores.」
- **成绩口径（裁决③）**：官方记分卡＝harness 即时判分 **134/196**（k=1、单一快照 0.8.14、
  六日 ≈41h）；任务级全档 11；Harbor 侧零第二记分卡。

## §3 0bz S4 离线对账（官方轮 36 题全语料；192 批离线形态）

- 语料＝36 题 244 个去重 journal（RUN 按最大快照去重；`bz_s4_offline_v3` 判据复算）。
- **判据①第 2 针归零＝达成（精确形态）**：186 次压缩中 137 次有 +2 事件，**137/137 全部
  `stable = count − 3`（恰尾槽）、尾槽前分歧 0 例**——192 批修码钉
  `d4_rerender_diverges_only_at_its_tail_slot`（重渲仅尾槽、其前 marker＋历史逐字节稳定）
  在官方轮全语料 **100% 成立**；修复前形态（头部槽全前缀重价、+2 hit 崌 ~12.8K）**0 例**
  （+2 hit min 43,264／中位 115,968／max 265,088）。
- **判据②空跑分叉归零＝达成**：空跑 compress 全轮 2 次、零整窗分叉（noop_fork=0）。
- **判据③自发塌陷读数（观察项）**：启发式候选 15 例（checkpoint 任务切换的合法头变更混杂，
  非硬判据口径，如实记）。
- **判据④命中率观察**：+1 hit 6K–78K（压缩轮重价）→ +2 hit 43K–265K（历史全程复用、仅尾槽
  3 消息设计内刷新）＝**尾槽刷新成本显著低于修复前全前缀重价**。
- 汇总工件：`0cr_official/bz_s4_round_full.txt`（逐压缩行）＋`bz_s4_round_files.txt`（语料清单）。

## §4 磁盘与装置（S3 窗内事件）

- **win-s4 VM 删除（用户授权「虚拟机部分可以清掉」）**：`/d/VMs/win-s4/`（HIGH-NIST 加固
  VM，vhdx 21G＋avhdx 4M，9-13 时代、摩擦线已弃用）——Hyper-V 未注册态核实后删除；
  **D 8.6G → 29G**。WSL `ext4.vhdx` 在役不动。
- C 侧 build cache 逐题 `builder prune`（engine 500 前峰值累积 3.6G）＋冒烟/`__env-main`
  镜像逐题 rmi——C 稳定 4.3G 未满。
- 203 批（隔壁窗口并行）行龄回缠一处（0ac 旧流水行迁第二卷 §1.150）由该批闭环，本批无新增。

## §5 台账

- 本档：`docs/audits/205_0CR_S3_HARBOR_CONVERSION_PUBLISH_0BZ_S4_2026-10-07.md`。
  （**编号顺延说明**：203＝0cu 立项、204＝0ct S1/S2＋0cu S2 落码〔均为隔壁窗口并行批，本批档名避让重排〕，本批顺延 **205**。）
- TODO：`P1-0cr` S3 勾选＋0cr 收官注记（**36/36＋Harbor 交付＋0bz S4 达成；0cr 闭合待用户
  裁决**）；`P1-0bz` S4 达成注记（闭合待裁决）；头部计数行指针（计数不变 62）。
- BACKLOG：本批记录指针＋计数行＋`0cr` 专节 S3 达成＋`0bz` 批序 S4 达成；第二卷 §1.151。
- 索引：头行 v4.174 → **v4.175**（写前重读、防覆盖隔壁窗口并行写入）。
- 门禁 `check_repository.py` ⇒ **error_count 1＝`orz submodule working tree is dirty`**——即隔壁窗口 204 批（0ct S1/S2＋0cu S2 落码）未提交的**预期态**（沿 0cb S2 先例「未提交落码预期态，提交批消除」），非本批内容、本批不处置；其余全绿（台账行三处超长回缠：索引 600／计数行 911／P1 行 1176 ≤1200）。
- 仓外工件：`0cr_official/`（convert_logs×38／publish×2／bz_s4_round_*／dataset_slopcodebench_friction／
  ADJUDICATION 交接）、`harbor-tasks/silverwhite/`（36 任务＋README＋metric.py）。

## §6 边界

1. **零源码**；0ct 维持挂起（用户裁决，未动工）。
2. **未推送**；0cr 闭合（S1–S3 全达成＋S2 收官）与 0bz 闭合（S4 达成）均**待用户裁决**。
3. Harbor 侧为摩擦寻找产物（裁决口径），零成绩主张；oracle/build 脚注为转换忠实度记录。
4. 六道 oracle 缺陷脚注题（2 已知＋4 新发现）照录 manifest，不影响任务可用性。

## §7 关键词

205 批、0cr S3 收口、Harbor 全目转换 36/36、silverwhite 发布、slopcodebench-friction 数据集、
friction-hunting-artifact 标注、非官方成绩声明、0bz S4 达成、尾槽分歧 137/137、旧第 2 针归零、
+2 hit 中位 116K、oracle 缺陷脚注 6 题（新发现 4：env_manager/file_backup/mvvault/test_translator）、
KNOWN_ISSUES 未显形 3、CRLF／引擎 500／loop 误杀三装置事件、win-s4 21G 删除、134/196 口径不变、
0ct 挂起、计数 62 不变。
