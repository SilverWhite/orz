# 209 批：0ct S1/S2＋0cu S2 全民审查处置——P2/P3 全量处置、P1 挂起待裁（2026-10-07）

> **用户令**：「请先处理全部的P2和P3，这个P1我先再想一想」（对同日全民审查报告的处置指令）。
> **性质**：审查处置批——P2×1（ADR 誊写义务）＋P3×5 全量处置；**P1（exec_policy 读向豁免
> 写向旁路两族）挂起不动**，探针形状登记在案待用户裁决后修复；**未提交、未推送**（沿 204 批
> 状态，提交随 0cr 窗解除后的落账批一并处理）；计数不变 63（隔壁 208 批 0cv 立项 62 → 63 在途，本批不占计数）。
> **审查输入**：同日全民审查（三面＝设计合理性／实现合理性／设计与实现符合性；12 文件 diff
> 逐行＋关键上下文核证＋钉组实测复跑＋临时探针实锤，探针已删、现场复原 +991/−192 逐位核对）。

## §0 P1 挂起登记（未处置，待用户裁决）

exec_policy `read_direction_exempt_positions` 复制族分支两族写向旁路（审查实测 5 形状全
Allow、修复前全 Block）：

- 族 a：nullish 重定向目标（`> /dev/null`）与 fd-dup 词元（`2>&1`）参与"末个位置词元"竞选
  成"落点"，真实 `.gsa` 落点被当源位豁免——`cp /tmp/a /proj/.gsa/b > /dev/null`、
  `cp /tmp/a /proj/.gsa/b >/dev/null 2>&1` 均 Allow；
- 族 b：旗值落点（`cp -t`／`cp --target-directory`／`install -t`）不在位置词元判定语义内，
  真实落点被当源位豁免——三形状均 Allow。

**挂起口径**：本批零改动 `read_direction_exempt_positions`（用户令「这个P1我先再想一想」）；
S3 载体进体前须先裁决本项（修复方向两族已随审查报告在案：重定向后随词元/`2>&1` 形态排除出
位置词元收集；闭表外值型旗整体不豁免或旗值排除出源位集合）。

## §1 P2 处置：ADR-0010 §14.61 就地勘误（誊写义务补齐）

0ct 两段门确认性转换已实质推翻 §14.61 第 2 项（首读扣内容）与第 4 项对 notice 的适用
（notice→policy_denial 转译），204 批未随批同步 ADR（违反索引 §0.1 权威顺序；169/0cn 先例
均为同批就地勘误）。处置：

- **vol-14 §14.61 第 2 项就地勘误**（v1.86）：读全开放——首读内容照常返回＋一次性确认信封
  随内容同回；`notice → policy_denial{code=session_volume_notice} + exit 1` host 转译臂退役，
  code 保留 journal 闭集仅供历史回放（生产零写入，沿 `attention_ladder` kind 先例）；
  第 4 项拒绝信封纪律仅对真实拒绝类（CredentialsDenied／逃逸 Denied）继续适用；B4 审计随
  内容交付同轮落账；唯一不变量＝只读不改。
- **主文件「冻结版本补记」追加 v1.86**（沿 v1.85 169 批先例格式），指认 §14.61。

## §2 P3 处置（五件全落）

### 2.1 三读工具确认信封静默消耗——修码（deferred marking）

审查发现：三读工具都在存在性/执行判定**之前** `mark_noticed`＋`mark_opened`——首读失败
（不存在路径／grep 派生失败／列目录失败）与非文本交付路径（图像/PDF/超大文件）消耗掉一次性
确认而模型没看到，与 0ct §2.2 自身措辞「信封随内容同回」不符。处置＝**mark 与信封交付同点**：

- `read_file`：NoticeRequired 块只置信封文本；mark 移至文末 content 装配点（文本交付路径）。
  未交付即未消耗——失败读/非文本读不落已确认态，下次成功文本读仍附信封。
- `grep`：prepare_grep 只置信封；`GrepReady` 增 `volume_access` 句柄，mark 移至两处结果装配
  点（run 同步路径＋progress_stream 流式路径）；派生失败 Early 路径不消耗。
- `list_dir`：mark 移至列表装配点（唯一 Content 返回）。
- 同分区并行读的轻微竞态沿 AUTH-PULL-SELF-DESCRIPTION 既有边界允许（注释在案）。
- InternalAfterNotice 臂的 B4 `mark_opened` 维持 0p S2 原位（读前审计、先于 0ct 存在、纯
  journal 面）——本批不动，边界如实记。
- **新钉 1 条**：`read_file_two_stage_failed_first_read_does_not_consume_notice`（不存在路径
  首读＝FileNotFound 且 access_state 不落盘；随后成功首读仍附信封；canonicalize 失败回退
  词法路径 → verdict 仍 NoticeRequired 的前提经源码核证）。既有两段门组（read_file ①②③/
  损坏态/逃逸/凭据、grep/list_dir 双钉、host 桥接钉）随改全绿——语义不变（交付即消耗）。

### 2.2 0cu 文案护栏钉扩展——修钉

护栏钉 `findings_copy_appears_exactly_once_per_face` 原只盖 write/read 主描述与 guide；
**补 write section 参数描述恰一处**断言（0cu §5-7 设计内增量此前无机械上限）。现量合规、
钉面补口。

### 2.3 0cu 设计稿 §4「压缩折叠行池」表述——文档勘误

设计稿 §4「findings 行与 notes 行同规则进入近窗明细行池」与 v0.3 上游设计相悖（B 近窗明细
池只含 exec→edits→tool_actions，notes 从来不在池内；与本稿 §6「折叠展开集不含 notes 类分
区」自相矛盾）；203 批档 §2.2「压缩折叠行池全部随动 +1」同误。**实现（零 compact/summary
触点）结果恰好正确**（findings＝非折叠分区，压缩后经 `blackboard_read section=findings`
恢复，E 指针「等非折叠分区」措辞覆盖；fold-proof 自动成立）。处置＝设计稿增 **§4.1 勘误**
＋203 批档随注——S2 若按原句字面执行会错误膨胀触点，勘误防复发。

### 2.4 刺激面总线总清单黑板分区计数——文档勘误

`LIF_RLI_STIMULUS_TYPED_BUS_DESIGN` 总况行与 §3.3「黑板 PULL 分区 11」未随 0cu +1。处置＝
两处 dated 勘误（11 → 12、`findings` 插入 notes 之后与 `partition_revisions()` 固定序一致、
标题计数保留原文作历史快照）；BACKLOG 第二卷 §1.139 流水行属历史记录不回改（设计稿勘误为
权威面）。

### 2.5 204 批档测试读数「+6 漂移」——勘回（审查报告自身勘误）

审查报告曾记「204 档 orz-tools 3020 与实测 3026 差 6、来源未定位」——**系审查方口径错误**：
204 档 3020 为**通过数**，3026 为 `--list` 总数（含 **6 ignored**）；3012＋8 新钉＝3020 ✓、
处置批复测 3021 过（+1 新钉）＋6 ignored 逐位吻合——**零漂移，勘回**。204 档 §4 已随批补
dated 勘误注记。

## §3 验证

- **orz-tools lib 全量**：**3021 passed / 0 failed / 6 ignored**（＝204 基线 3020＋本批新钉
  1）；`two_stage` 组 8/8（含新钉）、`gsa_` 组 17/17。
- **orz-loop lib 全量**：857 passed / **2 failed** / 3 ignored——
  `retrieval::dispatch::user_cancel_closes_pending_activations_before_run_cancelled` 与
  `controller::cancel_during_permission_await_resolves_then_terminates`（user_cancel 竞速族）。
  **定谳＝非本批引入、非触碰面**：dispatch.rs 与 HEAD 逐位一致（不在 204/209 diff）；两测
  为 30ms/100ms 纯时间窗竞速；204 终轮同树全绿（859/0）；192/193 批「stash 冻结树同败」
  在案先例；本批对 controller.rs 的改动仅在另一测试函数体内（0cu 护栏钉参数描述断言）。
  HEAD 干净 worktree 对照实证两次尝试均被 **orz-assurance rustc ICE
  （STATUS_STACK_BUFFER_OVERRUN）阻断**——与 204 §6.2 同族、新 target 目录亦复现，本机
  工具链/内存态今日不稳的独立佐证；对照未遂如实记。host 并行全量 350/5 挂（负载敏感族＋
  symlink e2e 等）→ **`--test-threads=1` 单跑全量 355/0**（沿 204 先例负载敏感族单跑判读）。
- **clippy（orz-tools／orz-loop lib＋tests）**：零 error；orz-tools 26 条／orz-loop 3 条
  警告全部落存量行（本批触碰面 grep/list_dir/read_file/controller 零上榜——grep 的 4 处
  E0308 为编译期修正〔`ToolCallId` → `.as_str()`〕非 clippy 面）。
- **fmt**：orz-tools 全包净；orz-loop 触碰面净（blackboard.rs 4439 一处 204 测试 hunk 漂移
  随批 rustfmt 修平；rustfmt 整文件执行曾卷入 1299 处 HEAD 先存漂移面，已外科手术回退逐位
  复原、hunk 归属复核在案）；非触碰面（acaf/compact/console/controller_test_support 等
  43 处）HEAD 先存漂移不卷入（沿 204 §6.4 纪律）。
- **门禁 `check_repository.py`**：结果见 §5（预期 orz 子仓 dirty——204/209 两批未提交的
  直接结果，沿 204 同型如实记）。

## §4 台账

- 本档：`docs/audits/209_0CT_0CU_REVIEW_DISPOSITION_2026-10-07.md`。
- ADR：vol-14 §14.61 第 2 项就地勘误＋主文件 v1.86 补记（§1）。
- 设计稿/批档勘误：0cu 设计稿 §4.1、203 批档 §2.2、刺激面总线总清单两处、204 批档 §4
  读数勘回（§2.3/2.4/2.5）。
- BACKLOG：`0ct` 节 P1 挂起注记＋处置指针、`0cu` 节处置注记、本批记录指针、P1 总览行、
  开放项锚点行。
- TODO：`P1-0ct` S1/S2 行处置注记＋P1 挂起登记、`P1-0cu` S2 行处置注记、头部计数行（62
  不变）与 P1 路由行。
- 索引：头行 v4.177 → **v4.178**（本批指针，205 移前批链）。
- 第二卷：§1.156。

## §5 门禁

- `python -m scripts.check_repository` ⇒ **`valid: false`／error_count 5**，归属两处：
  ① **隔壁 208 批 0cv 在途四错**（`audits/208_0CV_DEEPSEEK_IFACE_ALIGN` 断链×3〔批档文件
  未落〕＋P1 节清单缺 0cv token〔节已建、锚点行未同步〕）——非本批面、不代修；② **「orz
  submodule working tree is dirty」**＝204（未提交）＋209（本批，同未提交）的直接结果、沿
  204 批同型如实记。**本批面全绿**：台账帽内（TODO P1 路由行 1186／BACKLOG 计数行 992／
  P1 总览行 1169／BACKLOG 指针行 1081／索引头行 749，均 ≤1200）＋台账一致性（计数 63 两侧
  一致、P1 token 三面一致）＋schema/fixtures。提交随 0cr 窗解除后的落账批一并处理（P1 挂起
  项与 0ct/0cu 代码同树，裁决后可同窗修码再一并提交）。

## §6 边界与不做清单

- **P1 修复面零改动**（用户令挂起）：`read_direction_exempt_positions` 三闭表与豁免函数
  本体逐位未动；审查探针已删、触发例 8 钉与写向对照 8 钉原样保留。
- 不动 InternalAfterNotice 的读前 B4 审计时点（0p S2 语义，先于 0ct）；不动两段门信封文本
  本体；不动凭据区/逃逸恒拒。
- 0cu 生产面零改动（仅测试钉扩展）；契约面（schema/判官/Python 镜像）零改动。
- loop 双挂不修（触碰面外竞速族＋环境证据链在案，沿「登记不修、随轮观察」先例）；
  非触碰面 fmt 漂移不卷入。
