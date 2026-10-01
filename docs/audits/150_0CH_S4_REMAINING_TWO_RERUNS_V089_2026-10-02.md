# 150 批：0ch S4 余两题重跑（caffe-cifar-10・git-multibranch・0.8.9 在役）——双 1.0 翻盘，S4 三题收官（2026-10-02）

> **用户令**：「请进行0ch剩余两题重跑吧」。
> **本批**＝0ch S4 重跑线收官批：余两题按 rerun3 同条件单题单跑＋每题收单检查
> （驱动 [`run_puller_control.py`](../../scripts/run_puller_control.py) 逐题模式，`--job-suffix -0chs4`
> 防覆写 rerun3 留档卷）。**零代码／零载体／零身份门改动**（跑批用 147 批已进体的
> 0.8.9 与现行身份门锁定值）。**计数 57 不变**（0ch 闭合待用户裁决）。
> **父仓未提交未推送**（沿惯例待令）。

## §0 结论速览

| 面 | 结果 |
|---|---|
| 身份门 | carrier `4e35f410…`（0.8.9 在役件逐位）／adapter `6d55c26e…`——OK（2026-10-02 01:27:33）；预拉 2/2 OK（caffe 本地已有 763MB、digest `929a6d63…`；git-multibranch 新拉 33s、digest `5bc44992…`） |
| caffe-cifar-10 | 作业 `official-v41-rerun3-caffe-cifar-10-0chs4`（BATCH3，官方 agent 超时 3600s）；01:28:02–01:57:19，rc=0，elapsed 29.3 min；**reward＝1.0，mode=self-completed**（`run_finished reason=completed`，run `RUN-CLI-6abe9839`，51 模型轮／83 工具）；`[rerun3-check]` PASS files=69 泄漏=0 上限读数=0 剩余读数=0 轮次换算=0｜残留全 0｜时间自述=0 |
| git-multibranch | 作业 `official-v41-rerun3-git-multibranch-0chs4`（BATCH1，官方 900s）；01:57:21–02:12:30，rc=0，elapsed 15.1 min；**reward＝1.0，mode=self-completed**（run `RUN-CLI-6abe9f15`，15 模型轮／26 工具，context_compressed×2）；check PASS files=27 泄漏=0 上限读数=0 剩余读数=0 轮次换算=0｜残留全 0 |
| 翻盘对照 | rerun3（140 批）同题 caffe＝0.0 harbor-timeout（3600s 烧尽）；权威试次 rerun2 git-multibranch＝0.0 harbor-timeout——本批两题均 1.0 self-completed，**2/2 翻盘** |
| 0ch 判读 | 两目标摩擦在三题一致消除：① `/dev` 面零摩擦（caffe terminal 面 /dev 零提及＝无需重定向；git-multibranch `>/dev/null` 类 21 处零拒〔rerun3 同题 `/dev` 写拒 179〕）；② 根级 make 实战放行（git-multibranch `/git` 建仓全程；caffe 根级 probe 实证 make/写 ✓、REMOVE/rename 拒＝保底正确）；③ **零改系统二进制**（rerun3 caffe `/dev` 260 行＋螺旋温床；本批两题 sed 改系统二进制＝0） |
| 保底保留实证 | 写控审查 caffe allow 49／warn 5／**block 3**（`carrier-write`＝.gsa 会话卷域写入×2＋**keystore 祖先臂**＝`/` 为 keystore 祖先×1〔v3.1 祖先臂实战〕；三拦全为保底目标面、模型绕行后 1.0 收官）；git-multibranch allow 17／warn 1；审批面 permission_decision 全 allow_once（72／26）、零拒绝 |
| 残余摩擦观察（不立项，如实记） | ① dpkg usrmerge 根条目（`/lib.usr-is-merged.dpkg-new`）REMOVE 拒再现＝145 批已接受边界家族〔已知代价〕——caffe 模型黑板记坑（「环境关键坑：dpkg 卡在 /lib.usr-is-merged.dpkg-new」）绕行，未死亡螺旋；② git-multibranch pty 分配拒（`/dev/ptmx` open FAIL／`Failed to get a pseudo terminal`，ssh 密码登录测试面）＝`DEVICE_SAFE_NODES` 文件级 `WRITE_FILE` 授权不覆盖 /dev/pts 分配链，非 0ch 回归（0.8.7 全保护同拒），模型绕行后 1.0 收官；③ git-multibranch 模型探针 cp 模板 hook 到 `/git/project/hooks/` 报 denied（机制未定谳、未阻断任务完成路径） |
| 资源 | snapshot pre D 9.20 GiB／images 13／containers 0 → post 9.19 GiB／images 11／containers 0；两镜像跑完即 rmi rc=0×2 |
| 台账 | 本档；BACKLOG（计数行／P1 总览行 0ch bullet／0ch 条目 S4 收官 bullet）；TODO（计数行／P1 路由行／P1-0ch S4 勾选注记）；第二卷 §1.102；索引 v4.118 → **v4.119**（头行＋§8 pending 桶 0ch 条目） |

## §1 跑批执行

- 驱动：`python scripts/run_puller_control.py caffe-cifar-10 git-multibranch --job-suffix -0chs4`
  （官方 argv 单一真源 `run_r0_heavy_official.build_argv`；条件与 rerun3 完全一致＝
  **不带 `--ak max_wallclock`**、官方 task 级超时唯一外边界、`--upload --public`、
  预拉纪律＋跑完即 rmi；逐题模式自动校验题名 ∈ 冻结 18 题集）。
- 起跑前检查：Docker 29.6.2 在跑、容器 0（无在跑批）、D 盘 9.20 GiB（高于 148 批耗尽线，
  未触发清缓存）、rerun3 两题原卷在档（后缀防覆写生效）。
- 逐题收单：`[rerun3-verdict]`（逐试次 `verifier_result.rewards.reward` 口径）＋
  `[rerun3-check]`（全库扫描：泄漏文件／上限读数／剩余读数／轮次换算行＋残留披露面），
  每题跑完即收（轮次日志 `D:/tb-eval/jobs-official/official-v41-rerun3-round.log`）。

## §2 S4 判读：两目标摩擦消除在三题一致成立

0ch S4 判据＝三题（configure-git-webserver〔145 批〕＋本批两题）复验两项修复：
`DEVICE_SAFE_NODES` 设备面文件级放行、`ROOT_MAKE_GRANT` 根级 make 族放行。

| 通道 | configure-git-webserver（145 批） | caffe-cifar-10（本批） | git-multibranch（本批） |
|---|---|---|---|
| `/dev` 写拒报错行 | rerun3=327 → **0** | rerun3=260 → **0**（terminal 面 /dev 零提及） | rerun3=179 → **0**（`/dev/null` 类 21 处零拒） |
| 根级 make 放行 | `mkdir -p /git` ✓ 轮内亲测 | probe make ✓（REMOVE/rename 拒＝保底正确） | `/git` 建仓全程实战放行 |
| sed 改系统二进制 | rerun3=4 → **0** | **0** | **0** |
| reward | 0.0（900s 硬杀；dpkg 边界＋检索偏航） | **1.0** self-completed | **1.0** self-completed |

**结论**：两目标摩擦消除三题一致成立；死亡螺旋（为绕 `/dev/null` 写拒逐个改系统二进制）
在 0.8.9 在役代零再现。caffe 本批 29.3 min 完成构建＋训练＋评测管线（rerun3 同题烧尽
3600s 判 0）——摩擦消除直接转化为任务完成。

## §3 边界与如实记

1. **0ch 闭合待裁决**：S4 三题全部跑完，闭合（`pending` → `implemented`，57 → 56）留用户裁决
   （沿 0cc 先例：闭合由用户令定）；本批计数不动。
2. **未提交未推送**：本批纯跑批＋落账（文档面），沿惯例随下一提交批一并。
3. **跑批条件披露**：与 rerun3 同条件（无 agent 侧墙钟）；caffe 对照＝rerun3、
   git-multibranch 对照＝权威试次 rerun2，均非同代对照；单题单跑不外推统计结论。
4. **残余摩擦三点**（§0 表）均不立项：dpkg usrmerge＝已接受边界家族再现；
   pty 分配拒非 0ch 回归且被绕行；hook cp denied 机制未定谳（若未来题面大面积撞墙再议，
   沿 145 批 §6.5 同口径）。
5. **session 面读取=0**：两题模型全程未拉 `blackboard_read section=session`
   （时间面已随 0cg 拆除，无需求）。
6. **发布面**：0.8.8／0.8.9 仍未推送未发行（停 v0.8.7；沿 113/116/121/144/147 惯例随推送批一并）。

## §4 关联与关键词

[`0ch 设计档 v4.0 §7`](../WRITE_CONTROL_BACKSTOP_REVISION_DESIGN_2026-09-29.md)／
[`145 批档`](145_0CH_S4_FIRST_RERUN_V088_2026-10-01.md)（S4 首题）／
[`144 批档`](144_CARRIER_REBUILD_V088_0CH_S3_2026-10-01.md)（S3 0.8.8）／
[`147 批档`](147_CARRIER_REBUILD_V089_0CF_0CG_2026-10-01.md)（0.8.9 进体）／
BACKLOG `0ch`／TODO `P1-0ch`。

关键词：150 批、0ch S4 收官、caffe-cifar-10、git-multibranch、0.8.9 在役复验、`-0chs4` 后缀、
双 1.0 翻盘、self-completed、零改系统二进制、`/dev` 零摩擦、ROOT_MAKE_GRANT 实战、
keystore 祖先臂实战、carrier-write 保底拦截、dpkg usrmerge 已接受边界、pty 分配拒观察、
闭合待裁决、计数 57 不变、未提交未推送。
