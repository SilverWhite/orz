# 204 批：0ct `.gsa` 读向拦截修复 S1＋S2（读全开放、两段门转确认性）＋0cu S2 落码（黑板 `findings` 第三分区）（2026-10-07）

> **用户令**：「接下来请做一轮长的夜间任务吧／目前0cr隔壁正在做，不太适合做提交／本窗口请完成0ct 代码部分+ 0cu S2，做完以后暂时不进行重建」。
> **性质**：落码批——0ct S1 定性勘定＋S2 落码＋钉子、0cu S2 落码＋钉子（设计稿 §8 全八类）；**计数不变 62**（两线此前已立项入账）；**未提交、未推送**（0cr S3 收口窗隔壁在途，提交面冻结）；**S3 载体重建暂不进行**（用户令），0ct S4 真机复核与 0cu S4 真机核证随下次载体与真机轮顺延。
> **红线**：`.gsa` 唯一不变量＝**只读不改**——写向保护（journal 本体、会话卷写/删）逐钉保留；凭据区（keystore/one_shot_permit/grok-home/chrome-profile*）与 symlink 逃逸恒拒面不动。

## §1 0ct S1 定性勘定（两处拦截面）

**触发例回查**（200 批 §2/§3）：0cr D4 meshctl `cp .gsa/rollback/3d66aa37/*.bak /tmp/meshctl_prev.py`
＝**读向复制**（源在 `.gsa`、落点在 /tmp）被 L2 写控 `carrier-write` `.gsa` 臂整命令拦。
勘定结论＝两处独立拦截面：

1. **L2 exec_policy 规则 5**（`orz-tools/src/types/exec_policy.rs`）：写段扫描对段内**全部**
   词元提取路径候选——复制动词的**源操作数**（读方向）与落点（写方向）不作区分，源位
   `.gsa` 词元命中卷根即拦。对照组：D2 `rm … .gsa/rollback` 与 D6 写备份入 `.gsa/rollback`
   均为**写向**（删/写发生在 `.gsa` 侧），按设计在案、不属本修。
2. **工具层 0p 两段门**（read_file/grep/list_dir 三处同形）：内部区首读返回**信封替代内容**
   （content withheld 直至二读）＋host 侧 `policy_denial{code=session_volume_notice}`＋exit 1
   ——用户定性「两段门只是确认性设计，不是限制性设计」，限制面须退役、教育面保留。

## §2 0ct S2 落码

### 2.1 exec_policy：读方向词位豁免（规则 5 扫描面收窄）

新增三张闭表＋段级豁免函数 `read_direction_exempt_positions`（写段内逐词豁免，表外动词/
程序返回空表＝0ct 前行为、保守全扫）：

- **`COPY_SOURCE_EXEMPT_VERBS`（7）**：`cp/copy/copy-item/cpi/xcopy/robocopy/install`
  ——复制/安装不改动源位，仅落点是写方向。目标位定位：末个位置词元
  （xcopy/robocopy 为 `src dst` 语序、目标位＝**第 2** 位置词元，`COPY_DEST_SECOND_VERBS`）；
  `-` 旗、重定向符号与其目标位不进位置词元收集（防「重定向目标位误当末位目标」把落点位
  `.gsa` 豁免成源位）；目标位不可辨时不豁免（命中即拒、绝不因解析失败放行）。
  **不入表**：`mv` 族（搬移即源位写——移出 `.gsa` 仍拦）、`ln`（硬链接目标位＝写别名向量）、
  `tee`（多落点同时写）。
- **`OPERAND_PURE_READ_PROGRAMS`（24）**：`cat/ls/head/tail/grep/egrep/fgrep/rg/stat/file/wc/
  diff/strings/less/more/md5sum/sha1sum/sha256sum/cksum/readlink/realpath/dirname/basename/which`
  ——重定向武装段中，除重定向目标位外全部词位＝读方向（`cat .gsa/x > /tmp/y` 是读不是写）。
  **不入表**：`find`（-delete/-exec）、`sort`（-o 落点）、`sed`（-i 原位写）、`awk`
  （system() 旁路）、`xargs`（派生执行）。
- 防膨胀钉：三表行数钉死＋「搬移/链接/多落点不入复制表」「写旁路读者不入纯读表」
  「复制族 ⊆ 写动词表」「纯读表 ∩ 写动词表 = ∅」四条保守边界断言。

**触发例回归钉（8 条新钉）**：meshctl 读向复制原文（缩尺）Allow；多源/`install`/robocopy
源位豁免＋落点位翻转即拦；`mv`/`ln`/`tee` 源位照扫；纯读段＋重定向 Allow 且目标位 `.gsa`
仍拦；纯读表排除项照扫；复制×重定向并存落点辨识。0cq 既有钉
`read_pattern_values_are_not_write_targets` 第三断言随确认性语义翻转（grep 操作数 `.gsa`
＝读方向放行），真阳性对照改重定向目标位。

**红线对照钉**（写向保留面，全保留）：`cp x .gsa/y`、`cp .gsa/x .gsa/y`、`robocopy /tmp/l
.gsa/o /MIR`、`install … .gsa/keystore`、`mv .gsa/x /tmp/y`、`grep … > .gsa/x`、
`cat /tmp/x > .gsa/o`、`cp /tmp/a .gsa/b > /tmp/log` 全拦（Block carrier-write）。

### 2.2 两段门确认性转换（读全开放、信封同回）

- **`resources.rs`**：`session_volume_notice_text` 改写为确认性文本（「本通知为一次性确认
  提示，内容读取不受影响（台账读全开放）」；删除「本轮不返回内容」「再次读取放行」两句）；
  `SessionVolumeAccess.mark_noticed` 只持久化「已确认」态、**per-call `notice` 旗标与
  `take_notice_this_call` seam 退役**；verdict 文档注释同步。
- **三读工具**（read_file/grep/list_dir 同形）：`NoticeRequired` 不再提前返回——
  `mark_noticed`＋`mark_opened_this_call`（B4 审计随内容交付落账）后**内容照常执行/返回**，
  一次性信封随内容同回（read_file＝文本 content/concise 前置〔raw_output 保持字节真〕；
  grep＝stdout 前置〔模型面读 stdout〕；list_dir＝列表文本前置）；`allowed_by_volume`
  补 `NoticeRequired`（三处——首读不再跌回 workspace 沙箱拒）。
- **host 转译臂退役**（`orz-host/src/lib.rs`）：`notice 旗标 → policy_denial{code=session_
  volume_notice} + exit 1` 臂删除；denial/opened 两臂保留。journal 闭集保留
  `session_volume_notice` code 供历史 journal 回放（生产零写入，沿 `attention_ladder`
  kind 先例）。
- **模型面文案同步（0cl 纪律）**：`model_face.rs` 分块表回放指针与 `replay_line`、
  `action_ledger.rs` 两处指针化文案——「首读若收到信封再读一次即放行」→「首次读 `.gsa`
  会附一次性确认提示，不影响内容读取」。
- **permission.rs 桥面零改**：`.gsa` 内部区读的桥面（`ReadOnly` → AllowOnce 放行至工具层）
  语义在 0p S2 即已就位——本批限制面实际位于工具层两段门＋host 转译臂，桥面无须改动
  （桥接全链钉 `bridge_yields_internal_reads_and_envelope_lands` 改写后实证）。
- **既有测试组改写为确认性钉**：read_file 两段门 ①②③＋损坏态重确认＋逃逸不放松＋凭据
  区恒拒六钉、grep/list_dir 各一钉——首读＝内容＋信封同回、二读起信封不再重复、
  escape/credentials 恒拒面全保留；矩阵 #（symlink 卷外 journal）子例随改。

## §3 0cu S2 落码（黑板 `findings` 第三分区，机制照抄 plan/notes）

触点（设计稿 §5 清单，行号漂移已复核；`EpochSnapshot` 实位于 `blackboard.rs:1115` 非
`epoch.rs`，设计稿 §5-15 就此勘误）：

1. `ModelNoteSection` +`Findings` 变体（as_str/parse 各一臂）。
2. 黑板根 `findings: Vec<NoteEntry>`（serde(default)，旧板/旧会话快照零迁移）。
3. `PartitionRevisions` +`findings: u64`。
4. `push_model_note` +1 臂（版本计数同一落点）；**`model_note_count` 保持不含 findings**
   （裁决 4，D1 责任不抵扣）。
5. `render_findings_section`（照抄 notes 渲染：时间正序、盖章头、空＝「（无）」）；
   `section=plan` 尾段只渲染 plan.model_notes（零交叉）。
6. `epoch_snapshot`/`restore_epoch_snapshot`/`EpochSnapshot`/`restore_conversation_snapshot`
   各 +1 字段/行（恢复后 `revisions.findings == 1`，与 notes 同点）。
7. `partition_revisions()` 固定序 +1 条（notes 之后）——PULL 徽章自动生效。
8. 工具面：read section enum +1 值、read 描述 +1 分句、write 描述 +1 短句＋write section
   enum +1 值＋参数描述 +1 分句、guide「plan/notes 模型可写」+findings（一词）；
   `tool_run.rs` 读派发 +1 臂、写侧非法 section 错误文案 plan|notes|findings。
9. **契约面（§6.1 勘误）**：`runtime/plan-write-event-payload-v0.2.schema.json` 的
   `section` 闭枚举 `["plan","notes"]` → **+`"findings"`**（0ae D0 增量在案、本稿 §6 初稿
   漏核 payload 文件）；`immediate_delivery.rs` 枚举同源钉同步三值。判官不校验 section 值、
   fixtures 不携带、Python 冻结镜像无该 schema 校验（复核）——旧 journal 回放零影响。
10. 文案护栏钉：write/read 描述与 guide **各恰含一处** `findings` 字样（0cl 瘦身不回涨的
    机械上限；主描述为此收为「恰一处」措辞）。

**§8 八类钉子落位**：①表级（parse 正/负例）、②端到端（write findings → plan_write 出账 →
read findings 逐字读回，tool_run.rs）、③徽章（partition_revisions +1＋固定序 notes→findings）、
④渲染（空「（无）」/时间正序/plan·notes 零交叉）、⑤快照（epoch＋会话双归位、恢复计 1）、
⑥D1 不含（model_note_count 断言）、⑦水位（live_compact_bytes 增长）、⑧文案恰一处——
在 blackboard.rs 钉组五钉＋tool_run.rs 端到端一钉＋controller.rs 文案一钉。

## §4 验证

- 触碰面＝orz-tools／orz-host／orz-loop 三包 lib＋tests；`cargo check` 三包零 error。
- **测试读数（终轮，降档 profile 见 §6.3）**：
  - **orz-tools 3020/0**（基线 3012〔193 批〕＋恰 8＝0ct 触发例钉组八钉；含两段门确认性
    改写组与复制/纯读豁免全组）；
  - **orz-loop 859/0**（193 批基线 851 过＋1 挂〔`user_cancel_closes` 30ms 竞速偶发、stash
    同败在案〕；本批 +恰 7＝0cu 钉七件，终轮全绿、偶发竞速本轮自过）；
  - **orz-host 353 过／2 挂**（＝`call_tool_timeout_kills_process_tree`／
    `run_tests_timeout_kills_process_tree`——**负载敏感族**：单跑（--test-threads=1）双绿；
    `call_tool_timeout` 在 **stash 冻结树同败**（HEAD 既有、非本批回归，沿 193 先例如实记）；
    环境含 0cr 隔壁负载与 0.9–4.2G 可用内存波动）；
  - 桥接全链确认性钉（`bridge_yields_internal_reads_and_envelope_lands`）与卷外 symlink e2e
    （`session_volume_symlink_windows_end_to_end`）随 0ct 重写后单跑/全量绿。
- **clippy（三包 lib＋tests）**：GREEN 零 error；警告 144 条全部落在存量行（逐行核对：
  exec_policy 1041/1096＝0cq 存量 collapsible-if、tool_run 529–6682＝存量、action_ledger
  76–757＝存量、blackboard 976／controller 397–4846＝存量；**本批新增代码零警告**——
  `read_direction_exempt_positions`／三闭表／三读工具改动／blackboard findings 面／两线新钉
  均未上榜）。
- fmt：触碰面 12 文件 rustfmt 净（级联报 acp_server.rs＝HEAD 存量漂移、非触碰面）。
- **红线抽查**：写向保留面 8 对照钉全拦（§2.1 清单）；凭据区/逃逸恒拒 4 钉全过（§2.2）。
- **门禁 `check_repository.py`**：`valid: false`／**error_count 1＝「orz submodule working
  tree is dirty」**——用户令「不太适合做提交」的直接结果（orz 落码不提交＋205 批同样未
  提交）；其余面（台账帽内／链接存在性／schema 260／fixtures）全绿。提交随 0cr 窗解除后
  的落账批一并处理。
- **先红后绿补证**：`bridge_yields_internal_reads_and_envelope_lands` 改写前在终轮前一轮
  实测红（"structured denial envelope on the bridged path"）→ 改写后绿——两段门语义翻转的
  红绿对在案。
- **读数口径勘误（2026-10-07，209 处置批）**：本节 orz-tools「3020/0」为**通过数**口径；
  209 批以 `cargo test --lib -- --list` 复核得总数 3026（含 **6 ignored**）曾误判为 +6 记账
  漂移——两口径相减恰为本批 +8 新钉（3012+8=3020 ✓），**零漂移、勘回**。orz-host 负载敏感
  族本批以 `--test-threads=1` 全量单跑复核 355/0（含 6 ignored），两挂族负载敏感定性维持。

## §5 台账

- 本档：`docs/audits/204_0CT_READ_DIRECTION_GSA_AND_0CU_FINDINGS_S2_2026-10-07.md`。
- 0cu 设计稿：§6 勘误（payload schema 枚举实有）＋§5-15 触点行号勘误（EpochSnapshot 位）；
  §10 批序 S2 达成注记。
- BACKLOG：`0ct` 节 S1/S2 达成、`0cu` 节 S2 达成、本批记录指针、P1 总览行、开放项锚点行。
- TODO：`P1-0ct` S1/S2 勾选、`P1-0cu` S2 勾选、头部计数行（62 不变）与 P1 路由行。
- 索引：头行 v4.175 → v4.176（本批时点；**隔壁 0cr S3 收口窗随后续 205 批推进至 v4.177**，
  204 批入其前批链——同窗并行落账、零冲突，如实记）。
- 第二卷：§1.152。

## §6 边界与环境注记（如实记）

1. **零提交、零推送**（用户令：0cr 隔壁在途）；**零载体重建**（用户令）——0ct S3／0cu S3
   随下一重建批、两线 S4 随下次真机轮。0cr 冻结面不受影响（本批源码不进官方轮载体）。
2. **target/debug 损坏事件**：批前 `cargo test` 持续 rustc ICE（STATUS_STACK_BUFFER_
   OVERRUN，HEAD 同崩）＋`panic_unwind` metadata stub 异常——判定 debug 制品损坏（D4 跑批
   磁盘清理误伤嫌疑，如实存疑），`rm -rf target/debug` 后恢复；重编译需
   `PROTOC=D:\CLI\orz\bin\protoc.exe`（build.ps1 单源，环境变量默认缺）。
3. **内存受限环境**：隔壁 0cr 在途，系统可用内存一度 0.9–4.2G，rustc 链接期 OOM
   （Allocation failed）多次——测试以降档 profile（debug=0/codegen-units=16/无增量/-j 2）
   ＋OOM 重试环执行；**读数以最终成功轮为准**。
4. **格式化钩子卷入与还原**：会话内格式化钩子对 orz-host／orz-loop 未触碰文件
   （HEAD 先存 fmt 漂移面）执行了 rustfmt——纯格式 diff、语义零变，按「无关文件不卷入」
   纪律全量 `git checkout --` 还原（还原后 `cargo check` 复核零 error）。
5. **先红后绿**：新钉先落、修码后落；红态观察受 §6.2/§6.3 环境阻断未及捕获，红态实证
   ＝生产 journal 在案（RUN-1a55a270-6 seq67 读向 block 为 0ct 触发例本身）＋修码构造面
   （豁免函数与旗标 seam 修复前不存在）。如实记，不补跑。

## §7 不做清单（沿两线定案）

- 不动 `.gsa` 写向保护（exec_policy 写向、write_control C1、journal 本体）；不动凭据区与
  symlink 逃逸恒拒；不动 0m/0p 教育信封**文本本体**（只改限制语义与措辞时点）。
- 0cu 不配提示规则/触发逻辑（P9 精神）；不设聚合档/TTL/轮换；findings 仍 PULL-only；
  与压缩白名单块互补不耦合；plan 描述原样不动。
