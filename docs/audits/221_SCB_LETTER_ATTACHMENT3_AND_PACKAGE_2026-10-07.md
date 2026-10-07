# 221 批：SCB 询问信附件 3 产出与四件打包（2026-10-07）

> **用户令**：「请进行吧」（承接 220 批尾「附件 3 是唯一还需要新写的……要我接着写 3、然后把四件
> 打成一个包吗？」）。
> **性质**：**附件产出＋打包批**——新增英文缺陷明细档（仓内）＋附件包（仓外 `0cr_official/`）；
> 零源码、零跑批；**计数不变 54**；未提交、未推送。
> **编号注记**：218＝邻窗 0.8.15 重建批；219＝询问信修订；220＝附件 1／2；本批 221。
> **结论先行**：附件 3（4 缺陷机械证据）落档、四件打成 `slopcodebench-orz-attachments-2026-10-07.zip`
> （52 文件／198 KB／sha256 `ca5a52e6…`）；**关键诚实修正＝对照粒度由「逐测试」改为「逐档」**
> （官方工具只输出逐档通过率，逐测试日志所在的临时目录已随会话清理，无法回捞）。

---

## §1 附件 3：4 缺陷机械证据（新档）

- **新档**：[`docs/en/SCB_REFERENCE_SOLUTION_DEFECTS_2026-10-07.md`](../en/SCB_REFERENCE_SOLUTION_DEFECTS_2026-10-07.md)
  （英文，直接给对方题目维护者复核）。
- **证据来源**：官方 `scb_to_harbor.py --validate-with-oracle` 的 oracle 验证步骤——**这是转换工具
  自己的裁决**（rc=4＝参考解未通过自身测试），不是我们对题目的判断；署名＝Harbor 转换 oracle
  环节＋AI 复核（GLM 5.3 Flash）。
- **四题逐档证据（工具输出原文照录）**：
  - **env_manager** ck3 strict `0.983957219251337`；agent 同档 **isolated 1.0000**／core 3/3。
  - **file_backup** ck2 strict `0.84`／**core 0.0**／isolated `0.5556`；ck3 strict `0.6764705882352942`
    ／**core 0.0**／isolated `0.2222222222222222`；ck4 strict `0.7528089887640449`；agent 同档
    core 1/1 ×3。
  - **mvvault** ck5 strict `0.9945945945945946`／core `0.875`／isolated `0.9666666666666667`；
    agent 同档 **isolated 1.0000**／core 8/8。
  - **test_translator** ck1–ck6 strict 0.688–0.756（ck1/2/4/5 **core 0.6667**）＋**ck7 缺
    `strict_pass_rate`、ck8 缺 step result**（结构性观察，独立于通过率）；agent 同档 ck1 core 21/21、
    ck3 **isolated 1.0000**／core 6/6。
- **对照口径**：参考解侧读数由官方 oracle 验证产出，agent 侧读数取自 220 批附件 2 的逐档记录——
  同一 harness、同一档边界、同一测试提取；**证据形态＝「参考解在自己的档上失 Core／失自身测试，
  而 agent 同档全绿」**。
- **已登记项对照（非新增）**：dynamic_buffer（ck4 oracle 0.7093/core 0.5/isolated 0.3824 × agent
  core 7/20）与 eve_market_tools（ck4 oracle 0.9867/core 0.875 × agent core 3/8）——**两侧同档皆败**，
  与 KNOWN_ISSUES 一致，明示非新缺陷。
- **转换全貌**：34 题 rc=0；四个缺陷题 rc=4；forge／textdrop／trajectory_api 为瞬时 rc=3 基础设施
  失败、重试收敛 rc=0（非缺陷）。

## §2 四件打包（仓外工件）

- **打包器**：`0cr_official/package_attachments.py`（确定性 zip：固定时间戳 2026-10-07T00:00:00Z）。
- **产物**：`0cr_official/slopcodebench-orz-attachments-2026-10-07.zip`＝**52 文件／198,029 B**／
  sha256 `ca5a52e6a10af1ea3f9cdc196aa467b60810884d3971902cae7139cf6d8247a9`；解压暂存
  `0cr_official/attach-2026-10-07/`。
- **包内结构**：`1_SCB_full_round_report_EN.md`（附件 1）／`2_merged_checkpoint_records/`（4 件，附件 2）／
  `3_reference_solution_defects/`（证据档＋6 份官方转换日志＋`convert_summary3.txt`）／
  `4_per_problem_readings/`（`readings_*.txt`×36＋两个提取器）／`MANIFEST.md`／`SHA256SUMS`。
- **尺寸合规**：198 KB ≪ GitHub issue 附件上限（25 MB），单件均可直传；Discord 免费档 10 MB/件亦满足。

## §3 台账

- **信件本体**：附件清单 1–4 改写为包内实名并补「打包件」行；第 3 条的对照粒度如实改为**逐档**；
  发送前核对清单「附件包生成（上列四件）」**勾选完成**。
- BACKLOG：计数行（**54 不变**）＋本批指针；第二卷 §1.167。
- TODO：头部计数行（54 不变）本批指针。
- 索引：头行 v4.193 → **v4.194**。

## §4 边界

1. **零源码、零跑批**；打包器只读既有工件，未改写任何原始件。
2. **对照粒度订正**：「逐测试对照」→「逐档对照」——oracle 逐测试日志所在临时目录
   （`%TEMP%\scb-to-harbor-*\oracle-*`）已随会话清理、无法回捞；先查过未果，如实降级并写明
   （「若需逐测试名，可由维护者按 §5 复跑参考解直接得到」）。
3. 附件 3 系**工具产物的转述**，不含我们对题目根因的判断；主张严格限于「官方管线在此 pin 上
   oracle 验证失败＋逐档对照形态」。
4. 附件仍为待发状态：**信未发送**；发送动作与 issue 链接随后续批次落账。
5. 未提交、未推送（与邻窗在途改动同树）。

## §5 关键词

221 批、附件 3、参考解缺陷机械证据、oracle rc=4、逐档对照（订正自逐测试）、env_manager／
file_backup／mvvault／test_translator、KNOWN_ISSUES 对照、四件打包、slopcodebench-orz-attachments
-2026-10-07.zip、52 文件 198 KB、sha256 ca5a52e6、计数 54 不变、索引 v4.194、信未发送。
