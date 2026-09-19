# 0aq 余项与 F1–F8 摩擦裁决登记（2026-09-19）

> 范围：0aq 审查处置线余下未闭合 RS 项（RS-04 残口／RS-05 残口／RS-06／RS-07 残口／RS-09／RS-15／RS-17／RS-18）与 0ar S1 报告 §7 狗粮摩擦 F1–F8 的归宿裁决。
> 依据：用户 2026-09-19 两轮裁决（原话见各节「裁决」行）；处置前主会话只读侦查（ref 拓扑、产物清单、LIFECYCLE 分层、启动器机理）。
> 纪律：本批零子仓改动、不动计数（RS-09/15 闭合属 0aq 线内勾选）、不提交不推送；`dogfood_launch.ps1` 为父仓脚本小改。

## §1｜RS-09 仓库历史瘦身 —— 闭合（仅记录、不额外处理）

- **裁决**：用户令「RS-09该处理还是要处理，删掉吧」，经处置前侦查出示修正发现后改裁决为
  **「那RS-09和RS-15就仅记录就好，不额外处理了」**——闭合，现状维持，不删任何 ref、不跑 gc。
- **侦查修正发现（审查报告"滞留 ref"定性不成立，须防未来照单误删）**：
  1. **`SilverWhite/CLI.git` 单仓库双仓承载**：父仓 `D:\CLI` 的 origin 与 orz 子仓的 remote
     `cli` 指向**同一个 GitHub 仓库**。远端三条分支中 `main`／`codex/np1-body` 属父仓，
     **`feat/fusion-architecture` 属 orz 子仓**（orz 推送其全部历史的上游备份分支）。
  2. 审查报告所称「远程 ref `origin/feat/fusion-architecture`（4952bc8c，领先 main 334 提交，
     携带整棵 vendored 树）」实为**父仓 fetch 时把 orz 的整棵对象树拉进了自己的 `.git`**：
     本地 tracking ref 停在 orz 0ah v8 提交 `4952bc8c`，远端 tip 已推进至 orz 当前 HEAD
     `9a1c3b2c7785…`（父仓本地无此对象，`git cat-file` fatal 实证）；「领先 334 提交／携带
     整棵树」是两条独立仓库历史线并列的假象，**该分支是 orz 在 GitHub 上的唯一云端备份**。
  3. `codex/np1-body` **不是滞留 ref**：0y 安卓支线活跃分支（P0 开放项），本地 worktree
     `D:/CLI/.tools/np1-body` 在检，tip 与远端一致（`3626f6fe`）。
  4. `.git` 总量 205MB／loose 103.88MiB dangling 的构成：大头＝fetch 拉入的 orz 对象树
     （随 tracking ref 钉住）＋ amend/checkpoint 残留（含 3 个 `refs/codex/turn-diffs/checkpoints/**`
     工具树 ref）；13.07MB `bin/protoc.exe` 历史 blob 钉在 `codex/np1-body` 的分叉历史里。
  5. **若按审查报告执行「删 ref」**：删远端 `feat/fusion-architecture`＝毁 orz 唯一云端备份；
     删 `codex/np1-body`＝毁 0y 支线唯一远端。此即「仅记录不处理」裁决的依据。
- **维持现状的含义**：205MB `.git` 与 dangling loose 接受不动；若未来确需瘦身，正确路径是
  「父仓本地删 tracking ref `origin/feat/fusion-architecture`＋fetch refspec 过滤（只拉
  `main`/`codex/np1-body`）防再拉入＋清 checkpoint refs＋`git gc --prune=now`」，**远端两条
  分支不动**；13MB blob 的释放需重写 0y 历史并强推，另行裁决（不值得）。

## §2｜RS-15 一次性产物生命周期清理批 —— 闭合（仅记录、不额外处理）

- **裁决**：用户先令「RS-15做归档」，经处置前侦查出示分层结论后改为**「仅记录就好，不额外
  处理了」**——闭合，全部产物维持现状。
- **侦查分层结论（记录在案，防未来整体归档误伤）**：83 个 `s4_vm_*` **不是可整体归档件**——
  ① ~70 件 consumed 状态被案例**在位引用**（`ORZ-WIN-SBX-001/002/003`、`ORZ-WIN-CTYPES-001` 等），
  [`LIFECYCLE`](../../scripts/LIFECYCLE.md) 明文「保留在位勿移动」；② VM 生命周期件为
  `retained`，转出条件（0l ⑥⑦ 闭合）未到；③ 真正无牵挂件仅 `_s4br_probe5`/`__wtest*`
  3 件脚手架＋未跟踪的 `candidate-gates/`（1,363 件，16MB）与 `tmp_*` 四目录＋tracked 的
  `_linux_arm_dryrun/`（71 件过时路线文档，399KB）。
- **闭合含义**：上述四类现状维持；后续单件退出追溯价值时按 LIFECYCLE 既有规则随批处理，
  不再作为整批任务追打。

## §3｜RS-17／RS-18 —— 维持开放（用户裁决：不急）

- **RS-17（TODO2 M3 复验启动决策）**：用户裁决「RS-17/18不急」——维持 `[ ]` 开放，
  不排期、不挂起；M3 复验启动时以执行时点载体重取证据（既有 S-16 口径）。
- **RS-18（巨型文档增长观察）**：同上维持观察态；超限随既有瘦身机制（0ab S1 行龄/行宽
  检查＋归档快照）处置，无新增动作。

## §4｜F1–F8 摩擦归宿（0ar S1 报告 §7 的「是否立项留用户裁决」就此关闭）

| 摩擦 | 裁决 | 处置 |
|---|---|---|
| F1 启动器误杀（严重） | （前批已定）已随 S1 批修复 | `dogfood_launch.ps1` 局部 `Continue`；本批补 F3 机理注记 |
| F2 半成品交接脆弱（严重） | **案例库沉淀，处置终点** | 新案例 [`ORZ-RUN-SEPARATION-001`](../cases/harness_environment/ORZ-RUN-SEPARATION-001-run-artifact-separation.md)；用户口径「每不同 run 的产物明确归不同 run，关键是不能混为一谈」「额外能做的也没有什么了感觉」原样入案例；不立工程项 |
| F3 输出不可观测 | **要改——已落改** | 启动器落改＝**活性侧车心跳**（每 30s 写 `<log>.live`：时长＋载体进程 PID/CPU/内存，进程退出自记终态；日志静止时以侧车判活性）；**机理勘误随批入档**：F3 病灶在 run 内长命令的子进程块缓冲（python 无 `-u` 的 unittest 圆点），启动器 Tee 对 Rust 按行 flush 载体本身实时落盘——改法清单（`python -u`／分段落盘／`Start-Process` 直写）属**模型运行时命令纪律**，机械层不做命令适配（F5/F6 同族边界）；自测＝PS 5.1 语法解析＋`-DryRun` 装配断言 exit 0＋BOM 保持（EF BB BF） |
| F4 控制台 GBK 乱码 | **已立项，归 0as** | 用户确认「F4已经立项」——并入 0as 编码观感线（工具结果解码面＋控制台回显面），BACKLOG/TODO 0as 补注随批；绕行纪律（关键内容 `read_file` 核证）见 S1 报告 §7-F4 |
| F5 命令面习惯冲突 | **不立项（设计边界）** | 用户口径：「环境的预检测与回报」相关面已有（`FUS-TOOL-PROBE` 机械可用性探针族）；「如果模型没有明确指定命令语法的终端环境是什么，命令面的习惯冲突还是要看模型自己在运行时做命令调整了，这一部分不是机械层能够进行补强的，机械层不能参与命令语法的转换和适配」——**登记为设计边界：orz 是通用框架、不强绑定环境，机械层不做命令语法转换与适配** |
| F6 PowerShell 解析陷阱（`$n:`） | **不立项（同上）** | 与 F5 同族：「有一些是模型自己在运行过程中不得不自己踩的坑，因为orz不可以强绑定环境，这是个通用框架」；绕行写法（`${n}`）见 S1 报告 §7-F6 |
| F7 本机套件长跑量级 | **观测维持** | 观测注记（runtime ≈4m45s/366、assurance ≈12m39s/1668、grok ≈45s/46），供后续 run 时间预算参考，非缺陷 |

- **总结论**：F1–F8 全部有归宿——F1 已修／F2 案例沉淀／F3 启动器落改＋纪律入档／F4 归 0as／
  F5·F6 定性为设计边界不立项／F7 观测。**不新立 FR-\* 工程项，计数不变**；0ar S1 报告 §7
  的裁决悬置就此关闭。

## §5｜随批改动清单（父仓，未提交）

1. `docs/cases/harness_environment/ORZ-RUN-SEPARATION-001-run-artifact-separation.md`（新增，F2）；
2. `docs/cases/README.md`（第五批登记＋同族追加⑦）；
3. `scripts/dogfood_launch.ps1`（F3：心跳侧车＋F1/F3 注记块；`-DryRun` 自测通过）;
4. `docs/BACKLOG_AND_PRIORITIES.md`（0aq RS-09/15 闭合、RS-17/18 注记、0as 补 F4 并线注记）;
5. `TODO.md`（RS-09/15 勾选、RS-17/18 注记、P1-0as 补 F4 行）;
6. `CLI_PROJECT_INDEX.md`（v3.83 头行＋案例桶登记）;
7. 本文档。

## §6｜读数

- `dogfood_launch.ps1`：PS 5.1 tokenize 0 错（673 tokens）／BOM 头 3 字节 EF BB BF 保持／
  `-DryRun` 装配断言全绿 exit 0（cwd 断言、载体三件套、ACAF manifest、题面可读、env 装配面）。
- 门禁 `check_repository.py`：随批末跑（见索引头行读数）；`error_count` 预期维持 1
  （唯一＝orz 子仓脏树预期态，0ar S2 落码＋0am 影子批随 O2 裁决）。
- 计数：**37 不变**（RS-09/15 闭合为 0aq 线内勾选；零新立项）。
