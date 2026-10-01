# TB 2.1 V4.1 轮·下半场判定文档：重跑范围与续跑口径（2026-09-29）

> **用途**：下个窗口执行跑分下半场的**完整交接件**——重跑范围判定（机械规则＋逐题清单）、
> 代际身份、逐题监督 runbook、披露口径。新窗口按本文档执行即可，无需回看对话。
> **权威位置**：本档为下半场口径的**最终裁决**，取代他窗账面（TODO `P1-0cb` S4／轮次档 §8）
> 早先记录的「整轮 89 题全量重跑」用户令（成本否决：k=1 非成绩轮、从头重跑太费钱）；
> 两处账面已加取代注记。
> **状态权威链**：轮次档 [`TB21_V41_FULL_RERUN_START_2026-09-28.md`](TB21_V41_FULL_RERUN_START_2026-09-28.md)（§1–§8，上半场全程账）
> ＋本档（下半场判定）。**用户裁决（2026-09-29）**：k=1 本就发布不上去，从头重跑太费钱；
> ①先改完写控（0cb，已完成）；②只重跑**存在拦截错误**的题目；③社区发布时说明白情况即可；
> ④「再扫其他问题」用零 API 成本的离线扫描承载，不烧钱扫题；彻底重跑否决（留给将来 k=5 正式轮）。

## 1. 上半场账面（截至 2026-09-29 08:57 收口）

| 项 | 值 |
|---|---|
| 已完成 | **44 题＝1.0×35、0.0×9**（命中率 79.5%），全部上传 Harbor（公开作业） |
| 挂起 | b1-08 qemu-startup（上游 bullseye-security 仓库腐烂，3 次装置失败留证 `jobs-official/_incomplete/`；在 `jobs-official/deferred-tasks.txt` 延后清单内，**继续挂起**——上游自愈或用户裁决例外前不跑） |
| 未跑 | **44 题**：b3-13…b3-19（7）＋ B4 全部 18 ＋ B5 全部 19 |
| 上半场载体 | 0.8.4（`87941130…`）；44 题逐题身份哈希在各自 `-round.log` 的 plan 行 |
| 异常史 | A2 镜像源腐烂（b1-08）／A3 网络瞬断（b1-14 重跑过 1.0）／A4 中途暴死（b3-05 重跑过 1.0）／骨架误判（job_done 已修）——处置全记录在轮次档 §7 |

## 2. 重跑判定（机械规则＋逐题清单）

**规则（用户裁定的机械化）**：0 分题中**被机械写控拦截 ≥5 条命令**者重跑；<5 条者不重跑
（失败主因属模型侧，重跑＝重掷模型骰子）。被拦次数来自各题轨迹 journal 的
`blocked by the mechanical write control` 计数（实测值，见下表）。

### 2.1 重跑清单（5 题，按被拦次数降序）

| 题 | 被拦 | 失败定性 | 重跑理由 |
|---|---|---|---|
| b1-01 build-pov-ray | **12** | 结构性无解 | 题面要求 `install to /usr/local/bin/povray` 且自带 `+O/dev/null` 检查命令——两形状均在旧写控拦截面上；模型实际编译渲染全部成功。**此题翻盘＝0cb 修复实锤** |
| b3-06 extract-moves-from-video | **10** | 241/262 差一口气 | 拦截税挤占工作轮；新载体下有效轮数增加 |
| b1-16 git-multibranch | **8** | 撞满 900s | 8 轮白烧在拦截上，900s 紧题里是主变量 |
| b3-12 torch-pipeline-parallelism | **6** | 已交付实现，隐藏测试未过 | 已能交付并 submit，差的是有效轮数；兼内存重题 |
| b3-03 winning-avg-corewars | **5** | 撞满 3600s | 全程磨题，5 轮拦截税可观 |

### 2.2 不重跑清单（4 题，原试次即为最终成绩）

| 题 | 被拦 | 失败主因（模型侧，与写控弱相关） |
|---|---|---|
| b2-15 gpt2-codegolf | 3 | 检索习惯惯性（超时验证轮 r1–r3 实证同形） |
| b3-09 pytorch-model-recovery | 3 | 检索烧蚀＋浏览器容器内本无（设计内边界） |
| b2-14 fix-code-vulnerability | 2 | 模型流退化，哨兵按设计斩流（随机事件） |
| b2-11 tune-mjcf | 1 | 900s 内仿真迭代不足，与拦截无关 |

### 2.3 通过题（35 个 1.0）不重跑

全部带着旧写控摩擦税通过——对 orz 是**保守（偏低）估计**；替换反而有「择优重跑」观感。保持原成绩，披露说明即可。

## 3. 下半场代际身份（0.8.5；逐题作业自动核验）

| 件 | SHA256 | 备注 |
|---|---|---|
| `orz-linux/orz` | `ecf1d6650dd7da5b20be484f9b25eea09d35ace440e1808e22a91db0155e6007` | **0.8.5**＝0cb S2 写控保底化（五条灾难 block 规则封闭枚举；整树位置锁/重定向即写/前缀词扫退役），源冻结 orz `15bc1bfb`（feat 提交 `a8478ff4`：orz-tools 2983/0/6・assurance 278/0・runtime 378/0） |
| `orz-linux/orz-signer` | `9f0b925d889fb9874d25f96ab3b89ba36e6c7f95d822fae497731c7dd9e14cec` | 实测在位 |
| `orz-linux/orz-acaf-provision` | `28d690bc921f8af95b4894f295267c2564557011433a20b7553f3ec0cb44cbcd` | 实测在位 |
| `tb_agents/orz.py` | `6d55c26ec9415d579961371973504f0d44de8027fe8318bbd8a0e835811b0d17` | 未变（上半场同值） |
| 执行器身份门 | **已换装 0.8.5**（2026-09-29 本窗更新 `run_r0_heavy_official.py`；旧值 0.8.4 留注释） | 载体不符即中止 |

数据集 pin 不变：`terminal-bench/terminal-bench-2-1@sha256:7d7bdc1cbedad549fc1140404bd4dc45e5fd0ea7c4186773687d177ad3a0699a`；模型 `deepseek-v4-flash`；`--env-file D:/tb-eval/.env`（`ORZ_WEB_SEARCH_LOCAL=on` 等在役项沿上半场）。

## 4. 下半场 runbook（逐题监督，沿上半场形态）

**口径不变**：官方 harness＋官方每题 agent 超时经 `--ak max_wallclock` 透传（官方值＝唯一评测墙钟）、k=1、每题一作业、`-n 1` 串行、`--upload --public`；跑批窗口内不做其他重活；**计费高峰停新题**（在跑题自然收尾）；单题失败重试一次；断点续跑（有 `result.json` 即跳过；残目录自动移 `_incomplete/`）；**09:00 式截止＝简单起跑门**（到点停新题、存档进度，不做硬截止准入）。

**每题核对清单**（题毕通知后）：reward／用时 vs 官方墙钟／作业 exit 码与重试记录／Harbor 上传行／容器归零／镜像清理／磁盘（起跑余量 35+ GB）。**骨架误判防御已在**（`job_done` 校验 `finished_at`＋试次数据）。

### 步骤（顺序执行）

1. **预检**：`python D:/CLI/scripts/run_official_v41_full.py --dry-run --deadline "<窗口截止>"` 应见 **45 todo**（b3-13…b3-19＝7＋B4＝18＋B5＝19＋b1-08〔运行时被延后清单跳过〕）；44 个已完成题显示 DONE（含 5 道重跑题——它们经步骤 3 的专用入口另起新作业名，不走驱动器）；Harbor auth（`D:/tb-eval/venv/Scripts/harbor.exe auth status`）；Docker 零容器；`jobs-official/deferred-tasks.txt` 仍含 `qemu-startup`（保持）。
2. **离线摩擦扫描（零 API 成本，先做）**：`python D:/CLI/scripts/tb21_friction_scan.py --quiet` 读现有 44 题 journal——确认除写控外无第二系统性装置问题；发现新问题先修随 0.8.5 线处置再续跑。
3. **重跑 5 题（兼 0cb S4 验证）**：**不重用原作业名**（原题目录保留为原试次证据）——逐题直调 runner：
   `python D:/CLI/scripts/run_r0_heavy_official.py --tasks <题名> --job-name official-v41-rerun-<题名>`
   （一次一题、后台、题毕核对后放下一题）。**顺序：build-pov-ray 第一**——它翻盘＝0cb 修复实锤；
   若它在 0.8.5 上仍 0 分，**停止花钱**，先查原因再决定后续。读数回填：TODO `P1-0cb` S4。
4. **续跑 b3-13…**（**2026-09-30 退役——§10 用户裁决：b3-13 起未跑面直接重跑、不续跑，本步骤不再执行**）：~~`python D:/CLI/scripts/run_official_v41_full.py --stop-after 1 --deadline "<窗口截止>"`（驱动器自动从 b3-13 断点续，b1-08 靠延后清单跳过）；逐题监督至窗口截止；下窗再续。~~
5. **窗口收口**：到点停新题；进度存档追加进轮次档 §7/§8（表行逐题回填＋收口小结节）。
6. **全部 89 题处置完毕后**：轮次收尾批——逐题读数汇总、`tb21_round_gate.py`（参照线比对）、
   披露稿（见 §5）、台账入账（0cb S4 勾选、0bz/0by S4 读数、索引滚动回填；**提交/推送仍冻结待令**）。

## 5. 社区披露口径（定稿要点）

- **k=1 筛查轮**（官方流程＋单次尝试基线扫描；官方榜单需 ≥5 试次/题，本数据不构成榜单成绩）；
- **装置缺陷中途修复披露**：第 44 题后定位「机械写控过宽」（43/44 题拦 215 条命令、
  build-pov-ray 题面要求被结构性拦截），修复＝写入管控保底化（0.8.5）；**受影响题目按机械阈值
  （被拦 ≥5 条）重跑替换**，原试次全量留档可查（`jobs-official/`＋Harbor 历史）；
- 逐题作业身份哈希留档（载体/适配器/数据集 pin），代际可复核；
- 先例：同项目 TB 评测 F1 修复（适配器试次隔离）即按此口径登记，社区可查。

## 6. 随窗承载与边界

- **0bz S4**（前缀缓存指纹读数）：`face_fingerprint` 事件随每模型轮落 journal——下半场长题可顺带收取读数（第 2 针／空跑分叉定位），**不作为跑批阻断项**。
- **b1-08**：保持挂起（上游腐烂未愈；用户裁决例外前不动）。
- **Docker VM 内存**（≈7.7 GB vs 内存重题 8192 MB）：上半场 6/8 完成4过2未过——不临时改系统配置；OOM/暴死复发才升级处置。
- **提交/推送冻结**（用户令 09-28）持续至落账批。

## 7. 快速状态卡（新窗口起跑前照抄核对）

```
载体   D:/tb-eval/orz-linux/orz  = 3332b38f…（0.8.7）   适配器 = 6d55c26e…
身份门 run_r0_heavy_official.py EXPECTED = 3332b38f…（已换装；旧值 ecf1d665…＝0.8.5／
       979a38fa…＝0.8.6 留档，见 §3/§8/§9）
驱动器 D:/CLI/scripts/run_official_v41_second_half.py（下半场专用：--deadline "<窗口截止>"；
       done 题按 result.json 自动跳过＝断点续跑；run_official_v41_full.py 已退役留作历史）
runner D:/CLI/scripts/run_r0_heavy_official.py（重跑专用：--tasks <题> --job-name official-v41-rerun2-<题>）
总账   D:/tb-eval/jobs-official/official-v41-full-round.log（上半场）
       D:/tb-eval/jobs-official/official-v41-second-half-round.log（下半场，2026-09-30 起）
延后表 D:/tb-eval/jobs-official/deferred-tasks.txt（qemu-startup）
挂起题 b1-08（上游腐烂）｜重跑 5/5 完毕（2 翻盘，§11.1）｜下半场 12/44 完毕（§11.2），
       余 32 题＝b4-06…b4-18（13）＋B5×19（§11.3）
```

## 8. 追记（2026-09-29 晚）：build-pov-ray 重跑读数与 0cc 立项——重跑线暂停至 0.8.6

**读数**：§4 步骤 3 第一题 build-pov-ray 已按本档执行完毕（作业 `official-v41-rerun-build-pov-ray`，
run `RUN-CLI-6abbb013`，20:33–21:09，36 min／12000s 墙钟，exit 0，Harbor 公开上传），
**reward 仍 0 ⇒ 止损门触发**，未放后续题。

**根因（journal 实证）**：7 次拦截全部来自 0.8.5 封闭枚举的规则 5 `carrier-write`——
`/usr/local/bin` 安装 ×4（含 `/tmp/lnk` 软链绕道被目标位解析识破）＋ `.gsa` 会话卷 ×2 ＋
`_bgprobe` ×1。**0cb v2.1 修复面确证生效**：`+O/dev/null` 检查形状已放开（「正确源码构建」
测试通过；模型构建 POV-Ray 2.2 渲染 3/3 SSIM 一致，产物 `/app/povray-2.2`、`/opt/povray/bin`
＋`~/.profile` 幂等钩，结束自述 `reason=blocked`）。剩余冲突＝载体自保护把 `/usr/local/bin`
整目录划入保护集 × 验证器硬编码 `/usr/local/bin/povray`，属保护集粒度设计问题。

**用户裁决（2026-09-29 晚）**：「应该继续收窄，把灾难保底纯粹变成宿主机灾难保底吧，毕竟只要
不重建，orz实际上不会被即时破坏」⇒ 立项 **0cc**（规则 5 容器内载体自保护退役；`.gsa` 宿主
bind mount 面保留；Windows 原生面 S1 裁量；批序 S1 设计→S2 落码→S3 **0.8.6** 重建＋身份门
换装→S4 5 题重跑＋续跑）。爆炸半径（49 题测试面已扫）＝未跑面仅 **b5-14 build-pmars** 同构。

**对下半场口径的影响**：① 重跑线**暂停**，剩余 4 题（extract-moves-from-video 等）改在 0.8.6
上跑（代际统一，避免中途换载体造成披露分裂）；② build-pov-ray 0.8.5 试次作为结构性 0 留档
（披露口径 §5 叙事追加第二段：装置修复分两步——0.8.5 保底化＋0.8.6 宿主机保底收窄）；
③ b3-13 断点续跑同样等 0.8.6；④ 台账入账（0cb S4 首读＋0cc 立项）已随本批落 BACKLOG/
TODO/索引（118 批，未提交未推送）。

## 9. 追记（2026-09-30）：0cc S2 审查处理批——0.8.7 在役，重跑线解禁

**审查处置**（设计档 [`WRITE_CONTROL_BACKSTOP_REVISION_DESIGN_2026-09-29.md`](WRITE_CONTROL_BACKSTOP_REVISION_DESIGN_2026-09-29.md) v3.1）：
0cc 三维度全面审查（设计/实现/符合性，物证核证 S1–S3 全部吻合）发现 P2×1＋P3×3，
同批全部处置——P2＝规则 5 增宿主态**祖先链臂**（`ANCESTOR_SWEEP_VERBS` 删除/搬移
封闭子集恰 15；`rm -rf <install>` 扫荡不触规则却连带摧毁 keystore 信任锚的缺口闭合）；
P3-1＝`DESTRUCTIVE_VERBS` 补 `install`/`ln`（37→39）；P3-2/P3-3＝措辞修正与双覆盖
报告顺序点明。读数 orz-tools lib 2988/0/6、clippy 13 基线持平、契约面零变化。源冻结
orz `79a3e8e5`（feat `cb88d416`＋bump 0.8.6→0.8.7）。

**0.8.7 双平台重建换装进体（2026-09-30 凌晨，本窗实测核验）**：

| 件 | SHA256 | 备注 |
|---|---|---|
| `orz-linux/orz`（在役） | `3332b38f71cd4b635f6a1538bb14266bb8f2d739676e92ba976a26b566dd1362` | **0.8.7**；static-pie；祖先臂两 face 字面量＋版本串进体；退役面 `carrier:install-dir` 零残留 |
| `orz-linux/orz-signer` | `1defacf83d02243b07213048727fd6e4a6da5ae6fb91e7d838d5aa092f7d41fc` | 直写换装位 MATCH 3/3 |
| `orz-linux/orz-acaf-provision` | `bc7d55763bccb1acf5ad905281c43dd79639e318b131d3a84b87d524e9e1f733` | 同上 |
| `tb_agents/orz.py` | `6d55c26ec9415d579961371973504f0d44de8027fe8318bbd8a0e835811b0d17` | 未变（上半场同值） |

Windows 在役三件 `104b9bce…/ed8066b7…/ebf5b3f9…`（`orz-windows/`，与
`rel-122-stage/_live` 活体集逐位一致；ACAF 重 provision、carrier-manifest 00:57 更新）；
打包 `rel-122-stage` zip `a6b2c4b1…`／tar `bd68682b…`；两侧回滚点 `.0.8.6-bak`；
执行器身份门已换装 `3332b38f…`（旧值留注释）。

**对下半场口径的影响**：① §8 的「重跑线暂停至 0.8.6」被本追记取代——「祖先臂与动词
补齐进体后方跑 S4」判据已满足，**重跑线解禁**；② build-pov-ray 翻盘题改在 **0.8.7** 上
重跑（新作业名 `official-v41-rerun2-build-pov-ray`；0.8.5 结构性 0 试次
`official-v41-rerun-build-pov-ray` 留档）；**判据＝`/usr/local/bin/povray` 安装放行＋
3/3 测试＋reward 1.0；若在 0.8.7 上仍 0 分 ⇒ 止损门再度触发，停止花钱先查原因**；
③ 剩余 4 题重跑＋b3-13 断点续跑代际统一 0.8.7；④ 本批（0cc S2 审查处理＋0.8.7 重建）
未提交未推送（用户令 2026-09-30）。

## 10. 追记（2026-09-30）：build-pov-ray 0.8.7 翻盘读数与 b3-13 续跑线退役（用户裁决）

**首题读数（0cc S4 首读，止损门未触发）**：作业 `official-v41-rerun2-build-pov-ray`
（run `RUN-CLI-6abbf664`，01:33–01:47，14m17s／官方墙钟 12000s 的约 7%，exit 0，一次
成功无重试），**reward＝1.0（官方验证器，题面 3 测试全过）——翻盘实锤达成**，已公开
上传 Harbor（`hub.harborframework.com/jobs/0ed4c08a-d124-49b7-9342-e916207ebb4c`）。
身份门核验通过（载体 `3332b38f…`＝0.8.7、适配器 `6d55c26e…` 未变）。

**写控读数**：拦截恰 **1 条**（0.8.5 结构性 0 试次为 7 条）——且为 v3.1 新祖先链臂
**实战首拦**：模型探测 `/etc` 写侧被拒（`target /etc is an ancestor of the ACAF
keystore root (/etc/orz-acaf/keystore)`），按设计开火、未阻碍任务完成；
`/usr/local/bin` 安装面全放行（journal 中 4 处安装命令均执行）；结束自述
`reason=completed`（0.8.5 试次为 `reason=blocked`）。0cb/0cc 收窄与 v3.1 祖先臂
因果链双向闭环：退役面放行、保护面开火、任务翻盘。

**用户裁决（2026-09-30）——b3-13 起未跑面直接重跑、不续跑**：「b3-13 的话，后续
直接重跑，不续跑，毕竟换版本了，同一道题跨包体版本不太合适」。⇒ §4 步骤 4 的
`run_official_v41_full.py` 断点续跑形态**退役**（§7 状态卡驱动器行随此作废）；未跑
44 题（b3-13…b3-19＝7＋B4×18＋B5×19）改以 **0.8.7 代际逐题独立作业**的形态执行
（与 rerun2 重跑系列区分——未跑题非重跑，作业名不带 rerun 语义，具体系列名执行窗定；
b1-08 仍按延后清单挂起不动）。

**披露口径（§5）影响**：装置修复叙事追加第三段（0.8.5 保底化 → 0.8.6 宿主机保底
收窄 → 0.8.7 祖先链臂加固，build-pov-ray 翻盘实锤）；**轮内代际分裂须明示**——
上半场 44 题＝0.8.4、5 题重跑＋下半场 44 题＝0.8.7，下半场不表述为同一 run 的
「续跑」；k=1 筛查轮口径不变。

## 11. 追记（2026-09-30 晨）：下半场第一窗收口——重跑系列 5/5 完毕（2 翻盘）＋未跑面前 12 题（9 过）

**执行形态**：用户令「进行重跑，如无问题就顺着进入剩下 44 题，依旧官方口径且单题起跑，
像上半轮跑分一样，并在今天早上 9 点的峰值计费时间前停下」。重跑 4 题逐题直调
`run_r0_heavy_official.py --job-name official-v41-rerun2-<题>`；未跑面按 §10 裁决绕过
已退役驱动器、新写下半场专用驱动器 [`run_official_v41_second_half.py`](../scripts/run_official_v41_second_half.py)
（冻结批序＋`official-v41-bX-NN-<题>` 官方序名、job_done 骨架防御、残目录 park、
延后清单、镜像 last-use 清理、失败 90 s 重试一次、`--deadline` 简单起跑门——纪律逐条
同退役驱动器，独立轮次日志不写旧总账）；窗口门设 **08:50**（9:00 峰值前留安全余量）。
全窗身份门核验通过（载体 `3332b38f…`＝0.8.7、适配器 `6d55c26e…`）；逐题 `--upload --public`。

### 11.1 重跑系列收官（0cc S4 第 2–5 题；build-pov-ray 见 §10）

| 题 | 0.8.5 试次 | 0.8.7 重跑（rerun2） | 用时 |
|---|---|---|---|
| build-pov-ray | 0（结构性 7 拦） | **1.0 翻盘** | 14m17s（前窗，§10） |
| extract-moves-from-video | 0（10 拦） | 0.0 | 31.8 min |
| git-multibranch | 0（8 拦） | 0.0 | 16.5 min |
| torch-pipeline-parallelism | 0（6 拦） | **1.0 翻盘** | 21.2 min |
| winning-avg-corewars | 0（5 拦） | 0.0 | 43.6 min |

重跑线收官：**2/5 翻盘**（build-pov-ray、torch-pipeline-parallelism），3 题重跑后仍 0
（属模型侧余量，重跑不再追）；全部 exit 0、一次成功无重试、Harbor 公开上传。
0cb/0cc S4 读数面就此完备（收窄因果链多题实证）。

### 11.2 下半场第一窗 12 题（04:27–08:57，全部 0.8.7 代际公开上传）

| 题 | reward | 用时 | | 题 | reward | 用时 |
|---|---|---|---|---|---|---|
| b3-13 adaptive-rejection-sampler | 0.0 | 17.1 min | | b4-01 install-windows-3.11 | **1.0** | 38.6 min |
| b3-14 cancel-async-tasks | **1.0** | 11.4 min | | b4-02 fix-ocaml-gc | **1.0** | 43.7 min |
| b3-15 db-wal-recovery | **1.0** | 5.8 min | | b4-03 train-fasttext | 0.0 | 62.1 min |
| b3-16 password-recovery | **1.0** | 7.8 min | | b4-04 circuit-fibsqrt | **1.0** | 61.3 min |
| b3-17 kv-store-grpc | 0.0 | 3.8 min | | b4-05 path-tracing | **1.0** | 8.4 min |
| b3-18 multi-source-data-merger | **1.0** | 5.3 min | | b3-19 modernize-scientific-stack | **1.0** | 4.3 min |

B3 批下半场 7 题收官（5 过 2 不过）；B4 进行至 5/18（4 过 1 不过）。**b4-05 path-tracing
＝本窗最后一道起跑题**（08:48:54，赶在 08:50 门前沿），08:57 自然收尾后驱动器门控退出
（本窗 12 题、0 作业失败、逐题 rmi、收口容器归零、磁盘 25.18 GiB）。

### 11.3 轮累计与下窗姿态

- **任务级口径（每题恰一次，重跑替换原试次）**：已决 **56** 题＝上半场 44（含 5 题重跑
  替换后 37 过 7 不过）＋下半场 12（9 过 3 不过）⇒ **46/56＝82.1%**。试次级口径
  （重跑计额外试次）＝61 试次 46 过（75.4%）。披露时以任务级为准并明示替换关系。
- 剩余 **32 题**（b4-06…b4-18＝13＋B5×19）＋b1-08 挂起不动；下窗照抄 §7 状态卡起跑
  （驱动器换 `run_official_v41_second_half.py --deadline "<截止>"`，轮次日志
  `official-v41-second-half-round.log` 自动断点续跑——done 题按 result.json 跳过）。
- **难度先验与 80% 账（2026-09-30 晨复盘，用户问「剩下的题目难度如何，能看到过 80% 的希望吗」）**：
  守 80% 需剩余 **≥25/32（78.1%）**（b1-08 挂起时 71/88；若 b1-08 跑且挂则 71/89＝79.8%
  惜败，跑且过需 26/33）。结构＝短题 ≤900s×20／1200–1800s×8／2400–3600s×4；**上代
  unsolved15 池成员 10 道**（dna-insert、gcode-to-text、make-doom-for-mips、
  make-mips-interpreter、model-extraction-relu-logits、protein-assembly、filter-js-from-html、
  extract-elf 等，占 31%）——池成员本轮实测 1/4（path-tracing 过；adaptive-rejection、
  train-fasttext 挂；extract-moves 重跑仍 0），旧池标签有真实预测力。有利面＝非池短题
  ~15 道为最高转化段、build-pmars 结构性拦截已被 0cc 拆除（povray 同构翻盘先例）、
  torch-tensor-parallelism 与刚翻盘的 pipeline 同族、extract-elf 0.6.x 代 r3 拿过 1.0；
  风险面＝regex-chess／distribution-search／bn-fit-modify（三道 3600s）＋compile-compcert
  （2400s）＋qemu-alpine-ssh（b1-08 同族 QEMU 环境）。**中央预期剩余 19–22/32（59–69%），
  整轮落点约 72–77%；80% 可达但需偏乐观情形**（非池短题 ≥85% 转化＋从池/长题偷 2 道）。
  口径内无杠杆（k=1 不择优、官方环境不补强为已裁线）。
- **离线摩擦扫描结论（2026-09-30 晨，用户问「是否依旧存在可能致题目失败的框架摩擦」）**＝
  **无致败级摩擦，跑批不停车**：今晚 16 试次拦截税 0–3 条/题（全部 6 条拦截逐一核证均按
  设计开火——`.gsa` 会话卷 ×3／规则 1 `/dev` ×2／祖先臂 `/` ×1），6 道 0 分题全部归因
  模型侧（4 道 `run_invalidated{wallclock}` 未交付＋2 道自认完成验证器判负）；哨兵 0、
  ACAF 拒绝 0、web_search 零失败、浏览器 27 败为容器无浏览器已裁边界。残余两条均不动
  分数：① wallclock 终局竞速（harbor `AgentTimeoutError` vs orz 优雅收尾，exception.txt
  ＋journal 不收割进作业目录〔卷内完好〕，上半场撞墙钟题已有、非 0.8.7 新增）；② 专用读
  工具 `outside_workspace` 拒绝 ~40 条/62 试次（shell 车道可绕，已知形态）。
  **①经用户裁决不修**（2026-09-30「那就没必要修了，就这样即可」）。
- 本追记为窗口收口存档（§4 步骤 5）；轮次收尾批（§4 步骤 6：读数汇总＋round gate＋
  披露稿＋台账入账）待全部 89 题处置完毕后进行，提交/推送待令。

### §11.4 成绩归因讨论收束与提分杠杆裁决（2026-09-30 晨后，126 批；纯裁决入账零代码）

- **模型事实更正（用户告知）**：官方 `deepseek-v4-flash` 已**全面路由至 V4.1 Flash**——
  本轮在跑即 V4.1 代，「模型代际差」归因**撤回**；与本窗 46/56＝82.1% 对照，官方榜单
  DSH Minimal 90.6 与第三方 dsh-minimal ~87.9 的差距若实，**归因收敛到 harness 侧**。
- **墙钟可见性实证（代码＋journal 双侧）**：适配器 `--max-wallclock` → `ORZ_MAX_WALLCLOCK`
  双消费＝orz-bin 硬门（到点 `run_invalidated{status:wallclock}`）＋**session PULL 面**
  （`blackboard_read section=session` 渲染 `WALLCLOCK_ELAPSED/LIMIT/REMAINING`＋T̂ 剩余
  轮数估计行，TER T1.8＋0am S1 Part A 接线）——**PULL-only，零 PUSH**；本窗四题抽查
  （train-fasttext／adaptive-rejection／circuit-fibsqrt／install-windows）`blackboard_read
  section=session` **合计 0 次**——钟在墙上、模型从没看过表。墙钟解剖读数：
  train-fasttext 60 min 中 53.3 min 为工具执行（含 8 min 训练跑 ×2，模型侧仅 5.4 min）、
  adaptive-rejection 15 min 中思考 153K token（8.1K/轮）＋疑在飞长命令、撞墙前零交付动作。
- **提分杠杆四条提议与用户裁决（2026-09-30）**：①墙钟阈值 PUSH 注入——**不做单独的
  注入式机制**（用户裁决原话「墙钟部分实际上这个纯粹就是跑分产物，日常使用非常少，我觉得
  做在黑板里就已经足够甚至有点过了，实在不行跑分前的题目提示词里面加一句本轮墙钟为多少
  多少，可在黑板中进行即时查询，没必要做单独的注入式机制」）⇒ 黑板 PULL 面即终态；
  **可选形态＝跑分题目提示词加一句**（本轮墙钟 N 秒＋黑板可即时查询），属适配器级、
  随下轮窗口再议（适配器受身份门钉死，本轮不动）；②前台长命令提示——**不做**（「长命令
  不额外做提示来加重模型负担」）；③短题思考预算档——**不改**（「思考深度不改」）；
  ④slim eval 档——未裁决、维持现状。
- **结论**：0.8.8 提分批按裁决**收缩为零新机制**；时间轴管理维持「黑板 PULL」现状；
  剩余 32 题照官方口径续跑（判定档 §7 状态卡），harness 侧差距以整轮最终读数为准再评估。
- **裁决后追加执行计划（2026-09-30 午，用户令）**：①**本轮全部跑完后**修改适配器题目提示词——
  加一句「本轮墙钟为 N 秒，可在黑板中即时查询」（§11.4 可选形态激活；适配器 sha256 随实验
  换装、身份门同步更新）；②**将撞墙钟的题目单独拉出重跑**对照验证（「将撞了墙钟的题目单独
  拉出来重新跑着试试」）；③blackboard_read 工具描述补 guide 分区简注——已立项 **0cf**
  （59 → 60，随 0.8.8 代窗口）。下窗续跑 12:19 起跑（起跑门 13:50，峰值 14:00 前十分钟
  余量沿 08:50 先例；在跑题自然跑完）。
- **裁决取消（2026-09-30 深夜用户令「透传墙钟不是标准做法……修改适配器后做超时重跑的
  设计取消吧」）**：①适配器提示词加墙钟一句——**取消**；②撞墙钟题单独拉出重跑对照——
  **取消**；墙钟可见性维持黑板 PULL 面现状（不迁移、不加注）。**全轮实测
  （89 作业全扫，130 批）**：`blackboard_read section=session` 共 **8 次/7 作业**
  （b1-12／b2-06／b3-06／b3-09／b4-06 mailman／b5-08 protein-assembly／rerun-build-pov-ray
  ×2，≈8% 作业拉取过至少一次）；各分区拉取分布＝plan 217／notes 82／external_ret 45／
  exec 16／actions 7／session 8／guide **1**——墙钟面「挂在墙上、极少被看」，透传取消的
  判断与此吻合；guide 分区仅 1 次拉取（0cf 立项依据同此实测）。

### §11.5 第二窗收口存档（2026-09-30 12:19–13:58，128 批；低谷价格窗 12:00–14:00，起跑门 13:50）

> **勘误（2026-10-01，142 批）**：本节「轮累计任务级 53/63＝84.1%」**偏高 2 题**——§9 已计入的 2 道翻盘题
> （build-pov-ray／torch-pipeline-parallelism）被重复计入；逐作业 `result.json` 机械复核值＝**51/63＝81.0%**。
> 以 [`89 题整轮成绩报告 §10`](TB21_V41_89_FULL_ROUND_REPORT_2026-10-01.md) 口径为准。

- **7 题＝5 过 2 不过，0 作业失败、0 撞墙**（两道 0 分均为**自完判负**：b4-09
  pytorch-model-cli 55 轮/16.2 min、b4-10 pypi-server 16 轮/5.4 min，无 exception 无
  invalidated；本窗未再现撞墙死法）：

  | 题 | reward | 用时 |
  |---|---|---|
  | b4-06 mailman | **1.0** | 24.7 min |
  | b4-07 crack-7z-hash | **1.0** | 6.0 min |
  | b4-08 large-scale-text-editing | **1.0** | 12.4 min |
  | b4-09 pytorch-model-cli | 0.0 | 16.2 min |
  | b4-10 pypi-server | 0.0 | 5.4 min |
  | b4-11 regex-log | **1.0** | 13.2 min |
  | b4-12 torch-tensor-parallelism | **1.0** | 21.4 min |

- 13:58 门控收口（b4-12 在飞自然跑完）；逐题 rmi、容器归零；下一题 b4-13
  make-doom-for-mips（unsolved15 池成员）及其后 25 题（b4-13…18＝6＋B5×19）下窗续跑。
- **轮累计任务级 53/63＝84.1%**（试次级 51/68＝75.0%）；B4 推进 12/18（9 过 3 不过）。
  **80% 账更新**：整轮 89 题需 72 过（b1-08 挂起则 88 题需 71）——已 53，剩余需
  **19/25（76%）**（b1-08 挂起口径 18/25＝72%）；§11.3 中央预期 19–22/32 中已兑 5/7，
  剩余 25 题需 14–17 过即落中央带。

### §11.6 第三窗（夜间长窗 2026-09-30 18:21 起，129 批）与执行形态升级

- 用户确认夜间低谷至次日 09:00 ⇒ 门限 **2026-10-01 08:50**；**b4-13 make-doom-for-mips
  0.0**（17.1 min 自完判负；unsolved15 池成员未翻盘，与池先验一致）⇒ 轮累计任务级
  **53/64＝82.8%**。
- **执行形态升级（用户令「不仅仅是单题跑，而是单题跑且单题收结果」）**：新驱动器
  [`run_official_v41_inspected.py`](../scripts/run_official_v41_inspected.py)——复用全部
  作业纪律＋每题完成**立即裁决**（reward／死法形态／轮数，ALERT 行即时浮出异常）。
- **job_done 判定修正（b4-14/b1-01 双实证，129 批）**：本版 harbor 0.20 作业汇总无
  `trials` 数组（完成标记曾误读），旧实现靠「试次级文件存在」旁路碰巧工作——对
  kill 骨架（b1-01 首半场遗留、b4-14 双杀僵尸汇总 `n_completed=1` 但
  `verifier_result=None`）误判完成。正确口径＝**见到 reward 实体
  （`verifier_result.rewards.reward` 递归）才算完成**；另立 `RERUN_JOBS` 权威映射
  （5 题重跑替换原试次，主批扫描以 rerun2 作业为准）。判定面验证＝dry-run
  todo 26/89（25 待跑＋b4-14 僵尸回队列）。过程误起跑（b1-01 ~1 min／b4-14 两度／
  b4-15 ~3 min）均即时停车清理，**未产生任何官方结果、不计入任何口径**。

### §11.7 无墙钟对照实验立项（2026-09-30 深夜，132 批；用户担忧「有墙钟导致成功收敛，分数高了」）

- **担忧实质**：四个拉取通过题恰恰是「看过表」的题（拉取集按构造＝看过表），b1-12 仅剩
  3 分钟余量交付、mailman 拉表后在 24/30 分钟处收束——「钟驱动收敛」与「本来就会收敛」
  两种解释在现有数据上**不可区分**（131 批的无尾巴读数只排除拖延，不排除钟的收敛成因
  作用）。若担忧成立 ⇒ 官方轮分数部分是墙钟可见性的产物，且 **0cg（拆除 session 面墙钟
  行）实施将反向影响收敛**。
- **对照实验（[`run_puller_control.py`](../scripts/run_puller_control.py)，132 批）**：
  7 拉取题（configure-git-webserver／mteb-retrieve／extract-moves-from-video／
  pytorch-model-recovery／mailman／protein-assembly／build-pov-ray）无 `--ak max_wallclock`
  重跑——orz 内部墙钟门关闭（harbor task 级官方超时仍为外边界）、**不 `--upload` 不
  `--public`**（非官方口径对照，不占官方记录）、作业前缀 `official-v41-xp-`、输出
  `jobs-xp/`；官方窗退出后运行（夜间低谷）。
- **判读表**：四道 1.0 拉取题无墙钟仍 1.0 且用时相当 ⇒ 墙钟非收敛原因，0cg 照常实施；
  不收敛或大幅变慢 ⇒ 墙钟可见性为收敛成因之一，**0cg 实施重议**（§11.4 拆除裁决门控
  于此读数）。
- **0cg 实施门控**：S1/S2 落码可照做，S3 进体前置＝本对照读数。

### §11.8 披露稿强制明细（2026-10-01 凌晨，133 批；用户令「最后的跑分报告需要详细写明才行」——包体混合与两轮重跑虽不涉及框架根本性机制，但披露必须完整）

- **逐作业包体代际表（机械提取自各作业 plan 日志 `carrier_orz_sha256`；两代口径
  2026-10-01 用户裁决定案「0.8.5/0.8.6 都是修复后自动重建的中间版本，直接刨出去」）**：
  **主表两代＝0.8.4**（`87941130…`）×45 作业＝首半场 b1-01…b3-12 ＋ **0.8.7**
  （`3332b38f…`）×46 作业＝重跑系列与下半场全部。**谱系注记（脚注，不入主表）**：
  0.8.5（`ecf1d665…`）×1＝0cb S3 修复重建的验证跑
  （`official-v41-rerun-build-pov-ray`，仍结构性 0——该跑直接触发 0cc 立项；其结果
  已被 0.8.7 rerun2 权威替换）；0.8.6 零服役（被 0.8.7 取代）。⇒ 本轮官方分数＝
  **两代包体**（0.8.4＋0.8.7）。
- **对照实验口径（用户裁决「如果移除黑板墙钟不涉及重建，那就维持 0.8.4 和 0.8.7
  两代包体，只移除后重跑拉取了墙钟的题目即可」）**：移除黑板墙钟＝**env 级、不涉及
  重建**（无 `--ak max_wallclock` ⇒ `WALLCLOCK_LIMIT/REMAINING` 显示消失；ELAPSED 行
  按载体现状仍渲染——披露如实注明此边界；官方期限由 harbor task 级超时照常执行）。
  **拉取墙钟的题目在移除后重跑，结果替换原试次**（任务级口径沿重跑系列先例）；重跑
  集＝官方轮全部落定后动态重扫（[`run_puller_control.py`](../scripts/run_puller_control.py)
  已具动态重扫，作业前缀改 `official-v41-rerun3-`、`--upload --public` 官方级）。
- **两轮重跑（用户令口径）**：①**0cc S4 重跑系列**——build-pov-ray 三代谱系
  （0.8.4 原跑结构性 0 → 0.8.5 rerun 仍 0 → 0.8.7 rerun2＝**1.0 翻盘**）；重跑替换
  原试次（任务级口径，每题恰计一次）；②**b3-13 起未跑面整体重跑**（`run_official_
  v41_full.py` 续跑形态退役，判定档 §10）。
- **评测协议面**：k=1 不择优、单题串行、官方数据集 pin（`sha256:7d7bdc1c…`）、
  官方环境不补强、任务级口径＋试次级口径双列。
- **job 级失败与未决（如实列明）**：b5-13 qemu-alpine-ssh 两试均环境即死（零轮次
  无 result，上游 QEMU 腐烂，b1-08 同族）；b1-08 轮内挂起；b4-14 僵尸汇总作废重跑；
  129 批误起跑（b1-01/b4-14×2/b4-15）均无官方结果。未决题处置随轮收尾批裁决。
- **执行形态演进如实列明**：批量驱动器→单题收果模式（129 批）＋job_done 判定修正
  （reward 实体口径）；峰谷计费分窗（08:50／13:50／08:50 三门）。
- 以上全部进披露稿正文（§4 步骤 6）；缺一即轮收尾批门禁不通过（用户令）。

### §11.9 第三窗收口与全轮收官读数（2026-10-01 01:38，135 批）

> **勘误（2026-10-01，142 批）**：本节「任务级 74/87＝85.1%（全 89 口径 83.1%）」**偏高 2 题**（同一重复计入）；
> 逐作业 `result.json` 机械复核值＝首轮账面 **72/87＝82.8%（全 89 口径 80.9%）**；收官（含 rerun3 与 qemu
> 软件源修正重跑）＝**73/89＝82.0%**。以 [`89 题整轮成绩报告 §10`](TB21_V41_89_FULL_ROUND_REPORT_2026-10-01.md) 口径为准。

- **第三窗（夜间长窗）收口**：18:21–01:38，**24 题出结果＝21 过 3 负**
  （负＝b4-13 make-doom 0.0／b4-14 chess 撞墙 0.0／b5-17 hf-model-inference 0.0
  自完判负）＋1 作业级失败（b5-13 qemu-alpine-ssh 两试 setup 即死）；**b5-19
  overfull-hbox 1.0＝全场最后一题**；驱动器 `== 结束：本窗完成 23 题 ==` 干净退出
  （23＋前驱动器 b4-13＝24 出结果），容器归零、逐题 rmi、全程公开上传。
- **全轮收官读数（89 题全部处置）**：**任务级 74/87＝85.1%**（全 89 口径 83.1%；
  b1-08 挂起口径 84.1%；试次级 75% 上下）——**大幅越过 80% 目标**（§11.3 中央
  预期 72–77% 证伪）；**unsolved15 池成员 7/8 转化**（仅 make-doom 未翻，池悲观
  先验证伪）；§11.3 风险名单五拦路虎全部翻盘；擦墙交付 1.0 ×4（fibsqrt／
  filter-js／query-optimize／model-extraction）＝rerun3 研究对象集齐。
- **b5-13/b1-08 死因更正（2026-10-01 用户问「存在其他镜像源吗」触发取证）**：
  两题同族死因＝**适配器 agent setup 装依赖环节 `NonZeroAgentExitCodeError`**
  （22 s 级即死，非 QEMU 启动、非任务镜像源——镜像均正常拉取；此前「上游 QEMU
  腐烂」归因**更正**）。实锤＝容器内 `apt-get install curl ca-certificates procps`
  撞 **Debian security 源 404**（`deb.debian.org` Fastly CDN 旧索引＋上游包轮换；
  b1-08 同为 22 s `NonZeroAgentExitCodeError`）。处置选项：①直接重试（CDN 旧索引
  会刷新，22 s 级失败重试成本极低）；②适配器 install 命令加 APT 换源 fallback
  （USTC/阿里/腾讯 Debian 镜像；适配器改动→身份门换装）。**未决题两题随收口批
  用户裁决**。
- **b1-08＋b5-13 处置裁决（2026-10-01 凌晨用户令「跟着墙钟批一起重新跑」「先定下来
  裁决，不直接进行动作」）**：两题**并入 rerun3 批同条件重跑**（无 `--ak max_wallclock`
  ＋rerun3 前缀＋upload＋替换原试次；两题 setup 即死零轮次，显示条件对其无差，
  并批只为一次性收口）；`run_puller_control.py` 已加 `EXTRA_TASKS` 接线（qemu-startup／
  qemu-alpine-ssh 恒入集）。**不直接执行**——rerun3 启动随用户令。
- 待令：rerun3 启动（裁决已定，随令）；收口批（§4 步骤 6：读数汇总＋round gate＋
  披露稿＋台账入账）。

### §11.10 rerun3 收官——固定 18 题单题单跑读数（2026-10-01 08:21，140 批）

- **执行形态（用户令）**：「请进行rerun3吧，单题单题跑，并且每题结束后都收一次
  单题结果并进行检查」⇒ 18 题**逐题独立作业**（一题一跑、跑完即收单，不在同一
  进程连跑）；`official-v41-rerun3-<题>`／`-k 1 -n 1 --upload --public`／官方数据集
  pin／每题预拉＋跑完 rmi；**agent 侧墙钟不施加**（不传 `--ak max_wallclock`）。
  02:25–08:21（≈5 h 56 min）；每题落 `[rerun3-verdict]`＋`[rerun3-check]`。
- **逐题读数（本次／权威原始）**：configure-git-webserver **0.0/1.0**（硬杀 16.0，
  砍杀时 8080 仍 404）｜count-dataset-tokens 1.0/1.0｜mailman 1.0/1.0｜mteb-retrieve
  1.0/1.0｜protein-assembly 1.0/1.0｜**pytorch-model-recovery 1.0/0.0**｜gpt2-codegolf
  0.0/0.0（原始同为硬杀）｜headless-terminal 1.0/1.0｜rstan-to-pystan 1.0/1.0｜
  **caffe-cifar-10 0.0/1.0**（硬杀 61.2；原始 47.4 自然完成）｜fix-ocaml-gc 1.0/1.0｜
  git-multibranch 0.0/0.0（权威＝rerun2 亦硬杀）｜install-windows-3.11 1.0/1.0｜
  **kv-store-grpc 1.0/0.0**｜sqlite-with-gcov 1.0/1.0｜torch-tensor-parallelism 1.0/1.0
  （agent 阶段被硬杀，交付已完整仍判 1.0）｜qemu-startup／qemu-alpine-ssh 无成绩/无成绩。
- **总账＝与原始同数 12/18，但进出各两题**（升＝pytorch-model-recovery／kv-store-grpc；
  降＝configure-git-webserver／caffe-cifar-10）——**净效应为零**：无时间信号既让两题
  「不再提前收尾反而做完」，也让两题「不收拢而被砍」，两向抵消。**用户裁定口径**
  「结果不被可见墙钟抬升就好」⇒ 本 18 题范围内成立，**原分数不是被可见墙钟抬升的**；
  **不外推到全 89 题**（本研究集是墙钟暴露面全集＋2 题并批，非全轮抽样）。
- **墙钟条件核验（18/18）**：零 `max-wallclock` 残留（argv/env 通道关闭）＋零时间读数
  落档（LIMIT 数值／REMAINING／REMAINING_ROUNDS 全 0）＋零模型时间自述；仅 2 题读过
  session 面（读到「未施加上限」形态，正文不落 journal——**可核面＝启动参数＋全库字面
  残留**，非逐轮回放）。**因果边界**：本轮载体 0.8.7 与原始（0.8.4/0.8.5/0.8.7 混代）
  **非同代对照**，跨代差异与「无墙钟」条件未做分离实验 ⇒ 只声明条件成立与进出读数，
  不声明单因归因。
- **失败形态**：能力型 4 题**全部为「做到被官方时限硬杀」**（无一次「做完主动收尾」）；
  环境型 2 题为**作业级 setup 失败**——`apt-get install curl ca-certificates procps`
  再次撞 Debian security 源 **404**（与 §11.9 同族、逐字同类）⇒ **§11.9 处置选项①
  「直接重试（CDN 旧索引会刷新）」被本轮实测否定**（≥1 次重试后仍 404）；余下选项②
  「适配器 install 加 APT 换源 fallback（需身份门换装）」留裁决。
- **工具修正（本批落码，未提交）**：① 起跑参数错误——原手搓 argv 把镜像当 `-i`
  （harbor 0.20 `-i`＝`--include-task-name`）⇒ 筛零任务 rc=1 秒退（首题实证）；改复用
  `run_r0_heavy_official.build_argv`＋代际身份门＋预拉纪律。② 逐题驱动＋每题
  `[rerun3-check]`＋跑完 rmi。③ 判分读数改用逐试次 `verifier_result.rewards.reward`
  （作业级 `stats` 是 reward 直方图，不可当分数）。
- **框架层摩擦（转 0ch 立项 61 → 62）**：① **11/18 题**「设备目录写入被拒」（apt／dpkg／
  git／sshd 均写 `/dev/null`）；② **3 题**「根目录不可新建」（`mkdir /git` EACCES）。
  机制实锤＝L3 Landlock 授权表由 `/` 顶层条目枚举生成（`/` 自身不在表内 ⇒ make 族在 `/`
  不成立）＋`LINUX_DISASTER_KERNEL_FACES` 把 `/dev` **整树**不授权，而 L1/L2 规则 2 对
  `/dev/null` **显式豁免** ⇒ **同一条灾难兜底两层粒度不一致**。详见 BACKLOG `0ch`／第二卷 §1.92。
- 待令：收口批（§4 步骤 6）；0ch S1 勘定（随 0.8.8 代窗口）；本轮台账提交与推送。

### §11.11 qemu 两题软件源修正重跑——作业级失败换成真实试次（2026-10-01 14:23，141 批）

- **用户裁决**：「先试试换源能不能拉下来镜像，适配器一定需要修改吗？」→「请开始起跑吧」。
- **换源实证（三条，全部可复现）**：① 原样重放适配器命令（`deb.debian.org`）**稳定
  404**；② 换 USTC 源（**拿到新鲜索引**）仍 **404**（同一批文件名）、换
  `snapshot.debian.org` 存档无效（`InRelease` 过期 339 天被忽略）⇒ **根因在 suite
  而非站点**：镜像 `bullseye-security` 索引 advertise 的 `curl_7.74.0-1.3+deb11u16`／
  `libcurl4_…u16`／`libnghttp2-14_…u3`／`ca-certificates_…deb12u1` 在仓库池中已不存在；
  ③ 绕开该 suite（仅 `bullseye main`＋`bullseye-updates`）⇒ curl 7.74.0-1.3+deb11u13
  ＋procps 2:3.3.17-5 可装。
- **适配器是否需要修改：不需要**。改「挂载修正软件源」（容器 `/etc/apt/sources.list`
  ← `scripts/aptfix-bullseye-sources.list`）即让适配器 `install()` 原样通过；端到端探针
  （不上传）实测 **`orz install check: orz-ready`**＋完整试次 10m25s＋**0 异常**（此前
  22–38 s 作业级 `NonZeroAgentExitCodeError`）。**不动镜像／适配器／身份门／数据集 pin／
  任务文件**。通用兜底（适配器装包失败降级或换源重试）判为另一件（须换身份门）。
- **两题重跑（带公开上传；作业名 `-aptfix` 区分）**：**qemu-startup 0.0**（自然收尾
  15.0 min、0 异常）／**qemu-alpine-ssh 1.0**（`test_sshpass` PASSED；agent 阶段被官方
  时限硬杀 16.1 min，交付已完整故判分不受影响）。两题墙钟条件照旧成立。
- **总账更正（§11.10 追记）**：18 题口径由「12 通过／4 失败＋2 题无成绩」更正为
  **13 通过／5 失败／0 无成绩**；与原始对照口径＝「16 题同数 12/12＋2 题新增真实读数
  1 过 1 不过」（原始两题仍为作业级失败，无基线）。
- **残留面首次落档（对 0cg 的证据）**：qemu-alpine-ssh 那次模型读 session 面**一次**，
  读数＝**`WALLCLOCK_ELAPSED: 0s`＋`WALLCLOCK_LIMIT: none (评测墙钟未施加…)`**（留档于
  `.gsa/**/compaction/blocks/*.md`）⇒ 残留面**真实可达**（此前 18 题扫描全 0 系该段未
  被归档），但**无上限／无剩余、elapsed 为 0** ⇒ 无可用时间信号；0cg 两渲染面拆除仍应做。
- **披露偏差（必须写入成绩披露）**：这两题跑批时容器内软件源被替换过（仅
  `/etc/apt/sources.list` 一个文件；镜像其余内容、任务文件、测试与 pin 未动）；
  其余 16 题无此偏差。载体 0.8.7＋适配器 `6d55c26e…`（身份门起跑前逐次核验通过）。
- 待令：本批台账提交与推送；0ch S1 勘定（随 0.8.8 代窗口）。
