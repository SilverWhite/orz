# 188 批：recli 四跑真机轮（0am 预测段真机读数承载轮）＋0cq 零误拦复核（2026-10-04）

> **用户令**：「请进行第四跑吧」（承接 187 批落账后的下一步裁决讨论——用户记忆核证「已无足够触发验收的可使用狗粮轮」与账面一致，主会话建议以题目轮为验收载体，用户令行）；本批＝**纯真机观测轮＋离线收取，零源码**；边界＝不推送、不改码、计数不变 58。
> **载体**＝0.8.13（源冻结 orz `a7526cc2`；身份门 `0e1c10e7…`；187 批双平台进体）＝**预测段（184）＋写控误拦修复（185/186）首次真机承载**。RLI 影子缺省常开（unset=enable，rig 零 env 注入）。
> **同轮并收**＝0cq 零误拦复核（123 批口径）＋0bk S3 压缩读数＋0bs 复验五点机械可测项①＋0bc 零压力面读数。

---

## §0 结论速览

| 面 | 结果 |
|---|---|
| 发射前置 | 载体 manifest 0.8.13（`orz` sha256 `0e1c10e7…`＝身份门逐位一致）；Docker 29.6.2；C 13G／D 23G（159 批满盘教训检查项）；基镜像两件在位（`slop-code:python3.12` 6.03GB／uv trixie-slim 271MB）；`D:/tb-eval/.env` key 注入；**ACP 冒烟 rc 0**（initialize/session/new/prompt 15.7s、173 updates、`smoke.txt` 实写、journal `RUN-970c1614-0`；provision rc=0、signer `0e094d53…` 与 0.8.13 清单逐位一致） |
| 跑批形态 | **8/8 checkpoint 单会话全程**（journal 9 卷＝8 RUN＋1 ARC；harness exit 0）；**总墙钟 ≈3,548.6s ≈ 59m09s**（二跑 53m／三跑 2h05m）；149 模型步／226 工具轮／RLI 采样 316 步／压缩 5 次；单会话 `ca4ef39d`，全程零 `transport_retry` |
| 分数（参考面，k=1） | **solved 8/8＝100%**（iso solved 100%／core solved 100%；problems solved 100%）——**四跑最佳**（三跑 3/8、二跑 2/8）；逐档 core（marker 口径）**11/11→9/9→7/7→5/5→5/5→6/6→5/5→7/7（55/55 全绿）**；全测试 34/34→255/255 逐档全绿；mean test pass **1.000**；erosion 均分 0.6933±0.0476（四跑最高，全 solved 面上的结构分，如实记、k=1 不追因）；verbosity 0.3429／lint:loc 0.0596 |
| **预测段真机首读（主验收）** | 唯一 streak fire（RUN-7 seq346）＝`err×3（u=2.00≥θ=0.57；失配概率 0.07）；源：执行失败×2；前推(10·T̂): 2.00→0.00 ↓，段内回落`——**streak fire 行 1/1 全带预测段**、五值闭集内、注解扩展行在件、行宽 **323B ≤ 400B 新预算**；**预测-实际同轮核证**：fire 后通道纯衰减，会话终态侧车 err `u=-0.001`（θ=0.655）＝「段内回落」形态与实际轨迹一致（首个同轮样本，k=1） |
| P8 面 | 30 notice **三面恰等**（journal 30 行＝`notice_delivered_total` 30，全 `delivered:true`）；构成＝迁移确认 12／spike 进入 9／掩盖缺口 8／streak 1；域机 9 spike（进9回9）全 normal↔low_progress 对称振荡、pressure/stuck 未访（g=0.50 全程＝任务形态未产生该摩擦，如实记）；T̂ 8.00→27.59 自适应；**ctx 终态 u=1.046≥θ=0.689 未 fire＝178 批 Ctx 排除出 streak 观察域真机再证**；消费率＝0（`blackboard_read` 调用本轮 **0 次**，较三跑 19 次 0 rli 节进一步；182 批用户定性沿用＝注入式下可预见，不构成否证） |
| 0cp/0cn 回归 | **全过**：动作采样绑定（30 条全绑域事件/streak/缺口，零周期注入）、看门狗零时间样（全程无卡死）、fire 恰 `×3`、直投三面恰等、D6＋成因段形制全带 ≤400B、`rli-shadow-v2` 在案；0cn 零残留（`kind=budget`=0／`预算`=0／`墙钟约`=0／grid 引用=0） |
| **0cq 零误拦复核** | 写控审查 **83 次，block=0／allow=83——零拦截＝零误拦** ✓；权限 226/226 自动放行零拒绝；host_resource_denied／resource_exhausted／reclaim 全 0；counterexample_gate 8（每档 1，正常）；工具错误率 5/226≈2.2%（三跑 2.0% 同量级）。**边界如实记**：三跑同型命令（find `-not -path` 的 .gsa 排除模式／不存在路径 `rm -rf`）本轮未复现——同型直接回归证据未获得；零误拦读数的支撑＝本轮零拦截面＋185 批三条 verbatim 核证＋钉 7（带真阳性对照臂） |
| **0bk S3 压缩读数** | 压缩 5 次（三跑 11 次）：346,972→128,291（0.370）／383,147→195,618（0.511）／334,903→187,476（0.560）／415,309→218,655（0.526）／443,930→232,780（0.524）——**单次实得削减 44–63%**（0bk 登记轮 12 次压缩 6 次 ≤16%；语料不同 k=1 不直接可比，如实记） |
| 0bs 复验①（结束自述） | **8/8 run_finished 全带 `intent=conclude`／`reason=completed`／`summary=True`**——模型面结束自述通道真机全程在用 ✓（② launcher 编码／③ lsp 复跑为装置侧检查不在本轮；④ 剥离计数为宿主报告面场景本轮不适用；⑤ 回执构造的模型面可见性不可从 journal 单面核，如实不采集） |
| 0bc 零压力面 | host_resource_snapshot ×8（trigger=run_start，tier=watch）；commit_used 1.88G／limit 6.27G，**commit_notification 0 次**；`cpu_rate_limit` 读数面零出现（判据①再证）；resource denied 0（判据②「无硬拒」零压力面下成立） |
| 工件 | 全部仓外：语料 `D:/tb-eval/0am_p8_run8/corpus_run8/`（9 卷）、`scan_run8.py`／`readings_run8.txt`、跑批日志 `D:/tb-eval/scbench/recli_run8_0am_forecast.log`、输出树 `outputs/deepseek-v4-flash/orz_just-solve_none_20261004T1705/`。仓内零源码变更（本批仅文档落账） |

## §1 发射与跑批

- 发射命令（与 171/181 批逐字同形）：rig 目录内 `uv run slop-code run --agent orz --model deepseek/deepseek-v4-flash --problem recli`（seed 42 默认、pass_policy any、one_shot off）；key 经 `set -a`／`set +a` 显式注入进程环境；输出目录 `orz_just-solve_none_20261004T1705`。
- 节奏（checkpoint 评估点，s elapsed）：892.0→1,211.7→1,474.8→1,726.7→1,949.5→2,350.3→2,743.5→3,408.3，完成 @3,548.6；mean time/chkpt 364.3±239.1s；末档（ckpt8）工作窗 ≈11 分钟（2,853→3,408 评估）。
- 快的分解（59m vs 三跑 125m，如实记、k=1）：工作量面为主——149 vs 268 模型步、226 vs 253 工具轮、5 vs 11 压缩、**零写控拦截改道轮**（三跑 3 拦各耗数轮）；工具错误率同量级（5/226≈2.2% vs 3/253≈2.0%）。
- RLI 影子面：`rli_shadow_enabled_override` unset→true（179 批语义），rig 无显式注入点＝缺省常开直进；0.8.13 八通道路由＋预测段生产喂入。

## §2 官方评分器读数（逐档）

| ckpt | eval@elapsed | own-core(marker) | 全测试 | 三跑 own-core 对照 |
|---|---|---|---|---|
| 1 | 892.0 | **11/11** | 34/34 | 11/11 |
| 2 | 1,211.7 | **9/9** | 71/71 | 9/9 |
| 3 | 1,474.8 | **7/7** | 105/105 | 7/7 |
| 4 | 1,726.7 | **5/5** | 133/133 | 4/5 |
| 5 | 1,949.5 | **5/5** | 151/151 | 1/5 |
| 6 | 2,350.3 | **6/6** | 184/184 | 0/6 |
| 7 | 2,743.5 | **5/5** | 214/214 | 1/5 |
| 8 | 3,408.3 | **7/7** | 255/255 | 6/7 |

- **core_solved 8/8**＝四跑首次全 solved；marker 分母固定（11/9/7/5/5/6/5/7）与三跑逐档可对照。erosion 明细：mean 0.6933／逐档 0.6597–0.8088（全 solved 面上的结构漂移分，如实记；k=1 不作结论）。

## §3 预测段真机首读（0am 主验收）＋P8 面

- **唯一 streak fire 全文**（RUN-ca4ef39d-7 seq346，t=3102.3s）：
  `持续越线: err×3（u=2.00≥θ=0.57；失配概率 0.07）；源：执行失败×2；前推(10·T̂): 2.00→0.00 ↓，段内回落；T̂(单轮工具周期会话自适应估计，秒)：8.00→27.59 ↑〔连续k个动作采样点u≥θ；失配=ρ>1占比；前推=闭式自由演化至h·T̂；仅读数非阻断〕`
- **判据逐项**：① streak fire 行带预测段 1/1（本轮全部 streak fire 即此 1 条）；② 前推格式＝`前推(10·T̂): U0→UH <趋势>`（horizon 10 沿预注册分通道表，零新增参数）；③ 趋势箭头 `↓`（死区 0.05 口径）；④ 段内越线＝五值闭集 `段内回落` 在集；⑤ 行宽 323B ≤ 400B（184 批新预算）；⑥ 注解扩展行（`前推=闭式自由演化至h·T̂`）在件；⑦ 零新触发（P9）——预测段只在此既有 streak fire 行渲染，无独立提醒。
- **预测-实际同轮核证（首个样本，k=1）**：fire 后无新 err 喂入（后续无执行失败），通道闭式自由衰减；会话终态侧车（`rli-shadow-v2`，steps=316，T̂=27.589）err `u=-0.001`（θ=0.655）——预测「段内回落」（跌破 θ 且不回越）与实际轨迹**一致**；段长 10·T̂=275.9s，fire 至会话结束 ≈447s＞段长＝预测窗完整走完。
- **ctx 通道终态 u=1.046≥θ=0.689 未 fire**：5 次压缩喂入把 Ctx 推过 θ，但 178 批裁决 Ctx/Infra 显式排除出 streak 观察域——真机再次按排除面运行（三跑 ctx 0.940 临界同形），排除裁决两次真机一致。
- 域机：9 spike（进9回9）全 normal↔low_progress 对称振荡，密度 1/4.9min（三跑 1/6.2、二跑 1/4.1 同量级，无突发聚集）；pressure/stuck 全程未访（g=0.50 持续）＝任务形态未产生该类摩擦，如实记。
- 消费率＝0：本轮 `blackboard_read` 调用 **0 次**（三跑 19 次无 rli 节）——模型未拉取任何黑板节；182 批用户定性（注入式＋注解形态下拉取与否取决于模型所需信息量）沿用，不构成否证。
- 0cp 七判据回归全过（见 §0 行）；`rli-shadow-v2` 侧车在案、八通道全程在算（deny/verify/infra 终态 u≈0＝本轮无对应摩擦事件，如实记）。

## §4 0cq 零误拦复核＋摩擦面

- 写控审查 83 次：**allow 83／block 0**——本轮零拦截，即零误拦（123 批「零误拦」读数在本轮载体与任务面上成立）。
- **边界如实记**：三跑触发误拦的三条同型命令形态（find `-not -path '/workspace/.gsa/*'` 读排除模式、引号内 `.gsa/usr)` 伪词元、不存在路径 `rm -rf` 祖先链卷根）本轮**均未复现**——同型直接回归证据未获得；本轮读数的性质＝零拦截面＋修复在体（187 批字面量判据＋185 批三条 verbatim Allow／Allow／warn-only 核证＋钉 7 全带真阳性对照臂）。0cq 是否据此收口闭合留用户裁决。
- 摩擦面其余全净：权限 226/226 自动放行零拒绝；host_resource_denied／resource_exhausted／reclaim 全 0；transport_retry 0；plan 车道拒绝 0；counterexample_gate 8（每档 1，正常）；工具错误 5 次（2.2%，同量级）。

## §5 0bk S3 压缩读数

- 5 次 `context_compressed`（run/seq：0/218、2/193、4/131、6/157、7/406），trigger→target 与实得比：
  346,972→128,291（**0.370**）／383,147→195,618（0.511）／334,903→187,476（0.560）／415,309→218,655（0.526）／443,930→232,780（0.524）。
- 对照 0bk 立项轮读数（12 次压缩、6 次实得削减 ≤16%）：本轮单次削减 44–63%，量级显著更高；语料与载体不同（0.8.4 时代 vs 0.8.13），k=1 不直接可比，如实记。0bk S3 读数入账、闭合随用户裁决。

## §6 0bs 复验项与 0bc 读数

- **0bs ①（结束自述常驻尾行）**：8/8 `run_finished` 全带 `intent=conclude`／`reason=completed`／非空 `summary`（交付自述全文）——告知面（0bs S2 落码）真机全程被模型使用 ✓。②③（launcher 编码／lsp 复跑）为装置侧检查、不在本轮范围；④（剥离计数）宿主报告面场景本轮不适用；⑤（回执构造的模型面可见性）journal 单面不可核，如实不采集、留狗粮轮。
- **0bc**：8×`host_resource_snapshot`（trigger=run_start，tier=watch）；commit_used 1.88G／limit 6.27G（无临限）、`commit_notification` 0 次；`cpu_rate_limit` 读数面零出现（判据①）；resource denied 0（判据②「无硬拒」）。S1/S2 在体（0.6.6 起）、09-22 三项资源面读数＋本轮零压力面——**读数面已齐，S3/S4 勾选与闭合待用户裁决**。

## §8 官方结果真实性复核（2026-10-04 用户质询「全过了，怎会如此……确认不是假通过」触发；全链核证）

> **结论：官方 solved 8/8／255/255 为真，排除假通过。** 用户质询合理（四跑从 3/8 跳到 8/8），主会话按「评估器执行痕迹→测试文件篡改面→独立复跑」三链核证，并经历一次自身复跑环境差异的误警及其排解，全程如下。

1. **评估器确曾实跑**：`checkpoint_N/evaluation.json` 逐档带真实测试名清单与时长（ckpt8 duration 88.6s）；`evaluation/stdout.txt` 为真实 pytest 会话（255 items collected、逐条 PASSED、plugins json-report/timeout/ctrf）；`problem.yaml` 各档 `include_prior_tests: true`、entrypoint `uv run appctl.py`。
2. **测试面零接触（篡改不可能）**：① 官方题库 `scb-problems/recli/tests/`（9 文件）git 工作树干净、停于 `126d2bc`（2026-04-24 起 tests 未动，与 181 批 marker 口径注记一致）；② **8 个 checkpoint 快照均只含 `appctl.py`＋`requirements.txt`（＋`.gsa` 日志）——agent 工作区内从无测试文件**，评估时测试由评估器以 `.evaluation_tests/` 注入（agent 全程盲测不可见）；③ 8 档 `diff.json` 全量核对＝agent 写入面仅 `appctl.py`/`requirements.txt`/`.gsa` 三类，**对测试路径写入为零**。
3. **独立复跑（决定性）**：从 checkpoint 快照另行 staging（appctl.py＋requirements.txt＋官方题库测试），在与官方同条件（docker CLI 在 PATH、无 daemon 需求）的独立容器中复跑——**ckpt8 全套 255 passed（51.7s）／ckpt5 151 passed（17.0s）／ckpt1 34 passed（2.2s）**，早中晚三点全部复现官方读数；`appctl.py` 为 3,906 行/117 KB 实体实现（非 stub）。
4. **首次复跑 17 failed 的误警与排解（如实记）**：首次用裸 `uv:trixie-slim` 镜像复跑得 17 failed/238 passed——逐项对比官方 evaluation.json 确认官方确记通过后，单测失败输出揭示根因＝`Error: Container runtime is not installed`：**裸镜像无 docker CLI**，appctl 的容器编排功能按设计拒绝；agent 容器（`mount_workspace: true`，评估同容器）内有 docker CLI（终端日志实证：agent 曾自跑 `docker --version`／`docker volume inspect`／mine-vs-ref 对照流），该 17 项测试在官方环境通过为真。此误警反而加强核证力度——测试确实行使容器编排行为，且官方环境与复跑环境对齐后逐位复现。
5. **「8/8 为何可能」的轨迹面解释（k=1 不外推）**：三跑走了弯路（268 步/11 压缩/3 条写控误拦改道/ckpt4 起塌陷），四跑无误拦改道、5 压缩、149 步即全程保持全绿（ckpt1 起 core 即满、逐档不回落）；erosion 0.6933 偏高与全绿并存＝实现早期即成形、后段结构分漂移（官方结构指标，非正确性问题），两项读数不矛盾。

- 复核工件（仓外）：`D:/tb-eval/0am_p8_run8/reeval_ws/`（ckpt8 staging）、`reeval_ckpt1/`、`reeval_ckpt5/`。
- **赛博摆件登记（2026-10-04 用户令「这一轮完美recli留着当赛博摆件吧」）**：四跑输出树 `outputs/deepseek-v4-flash/orz_just-solve_none_20261004T1705/`（已落 `README.ORNAMENT.md` 铭牌）、跑批日志 `recli_run8_0am_forecast.log`、语料 `D:/tb-eval/0am_p8_run8/corpus_run8/`——**指定免清理留档**；run2 2/8→run3 3/8→run4 8/8，系列唯一满分。
- 台账补记：本档 §8 随批补入（amend 同批提交）。

## §9 sith 新题探针（2026-10-04 用户令「不做A/B比较了，开一道新题，找难度最高的，看一下具体情况」；同载体 0.8.13）

> **选题依据**＝158 批 0CJ S0 Hard 池勘察顺位（未跑题中最高）：**sith**（#2，仅次于已四跑的 recli）——代码智能 CLI（补全/类型推断/goto-def/引用/签名/搜索/重构 extract/rename/诊断/环境发现/项目配置），6 checkpoint 全 Core Tests、零外部依赖、测试 5,547 行＋夹具。备选 test_translator（8ck/421 测试文件）/meshctl/mocked_http 殿后。工作区 `tests/` 为任务夹具（`fx/`＋`test_sith.py`），官方评估测试仍以 `.evaluation_tests/` 注入＝盲测结构不变。

- **结果：未复现全绿——core solved 2/6（33.3%）、checkpoint solved 0/6、mean pass 0.895**；总墙钟 2,638.8s≈44m（harness exit 0，120 步）。逐档：ckpt1 core **6/6**（38/39）→ ckpt2 5/6（70/75）→ ckpt3 6/6（101/108）→ ckpt4 8/11（126/146）→ ckpt5 5/7（157/186）→ ckpt6 14/21（187/228）＝通过率 0.974→0.820 单调下滑、失败数 1→41。
- **失败面＝真实语义硬骨头**（能力边界显形，非噪声）：类型推断（None 收窄/isinstance 联合收窄/裸 return）、循环导入与 `..` 相对导入解析、stub 与 .py 源优先级、extract-function 重构族（多返回值元组解包/unified diff/参数序保持）、环境发现（virtualenv 扫描/symlink 去重/版本排序）。
- **RLI/LIF 面**（同轮收取）：6 notice（迁移 3/spike 2/fire 1）；唯一 fire＝`err×3（u=2.81≥θ=1.17）源：执行失败×4` 带预测段 `前推(10·T̂): 2.81→-0.27 ↓，段内回落`；压缩 8 次；写控 0 拦截、权限 111/111 自动放行。
- **框架摩擦全谱**（对标 181 §4b 口径）：写控审查 102 次＝allow 98＋**warn-only 4（留痕不阻断）＋block 0**；权限 111/111 自动放行零拒绝；infra 全零（无 transport_retry／resource_denied／reclaim）；counterexample_gate 6（每档 1 正常）；orientation 3；压缩 8；无截断/编码/长行摩擦。工具错误 11 次**全数归因模型侧**：2× 调用不存在的 `run_terminal_patch`（幻影工具名，框架如实 `Tool not found` 拒绝，仅 RUN-0 出现后自止——登记观察不立项）；9× exit_1 全为模型自检脚本的语义不匹配（补全排序/builtin 前缀过滤/signature 列越界等＝任务本体摩擦，框架忠实回传）。fire 成因段「执行失败×4」＝streak 窗口内喂入计数，与全轮 11 次工具错误两口径不同，如实记。**框架侧零干扰摩擦**。
- **归因含义**：同载体同模型，难题上即刻回落到 33% core solved——recli 8/8 不是 0.8.13 能力面的普遍水平，而是「该题轨迹利落＋方差正向」的组合；结果面双峰（保持干净→横扫/进坏分支→螺旋）在 sith 上获得独立对照点。
- 工件（仓外）：`D:/tb-eval/scbench/sith_run1_0813.log`、输出树 `outputs/deepseek-v4-flash/orz_just-solve_none_20261004T1910/`。
- **隔离面核证（用户问询「第四跑模型应该没有看前三跑的轨迹和日志吧」）**：① prompt＝6,623B 纯题面（recli checkpoint_1 规格全文），零前三跑痕迹（grep run3/RUN-fc2ae99e/0.8.12 均 0 命中）；② 容器＝每跑新建随机 temp 目录挂载（`Temp\tmp2u_ex131→/workspace`），宿主 outputs/语料从不进容器；③ 模型 API 无状态、每跑全新 ACP 会话（三跑 `fc2ae99e`/四跑 `ca4ef39d`）；④ 题库测试不在工作区（评估时注入）。**不可能见到前三跑轨迹/日志。**

## §10 台账

- 本档：`docs/audits/188_RECLI_RUN4_0AM_FORECAST_READINGS_2026-10-04.md`。
- TODO：`P1-0am` 增 recli 四跑读数行（真机读数项达成，闭合待裁决）；`P1-0cq` S3 真机复核读数入账（闭合待裁决）；`P1-0bk` S3 压缩读数入账；`P1-0bc` 读数齐注记；头部计数行指针（计数不变 58）。
- BACKLOG：`0am` 批序增补＋`0cq`/`0bk`/`0bc` 读数指针；计数不变。
- 索引：头部行换新（v4.160）。
- 不推送、不重建（0.8.13 在役）、零源码。
