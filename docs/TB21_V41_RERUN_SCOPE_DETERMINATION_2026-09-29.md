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
驱动器 （续跑形态已退役——2026-09-30 §10 用户裁决：未跑 44 题改逐题重跑；
       run_official_v41_full.py 留作历史不再用）
runner D:/CLI/scripts/run_r0_heavy_official.py（重跑专用：--tasks <题> --job-name official-v41-rerun2-<题>；
       official-v41-rerun-build-pov-ray 已被 0.8.5 结构性 0 试次占用，0.8.7 重跑起新名）
总账   D:/tb-eval/jobs-official/official-v41-full-round.log
延后表 D:/tb-eval/jobs-official/deferred-tasks.txt（qemu-startup）
挂起题 b1-08（上游腐烂）｜未跑 44 题（b3-13…、B4×18、B5×19）｜重跑 5 题（§2.1）
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
