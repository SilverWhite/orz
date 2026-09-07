# 模型自信息面补强与 `.gsa` 两段门设计（2026-09-07）

> **状态**：设计定稿，同日用户裁决排期实施（BACKLOG 0p / TODO P0-0p）；
> orz 版本基线自本批起 **0.3.2**。
> **上级**：[`BACKLOG 0p`](BACKLOG_AND_PRIORITIES.md) /
> [`TODO P0-0p`](../TODO.md) / ADR-0010 §14.61。
> **定位**：本设计修订 AUTH-GSA-SESSION-VOLUME（ADR-0010 §14.56）的
> `.gsa` agent-invisible 语义——由「仅两窗口直读」修订为「内部区两段式
> 有界开放（首读通知→二读放行）」；并补强黑板自历史按需面。不改变会话
> 卷的底层类型化系统状态域地位与 fail-closed 基线。
> **证据基础**：[`W2_ORZ_DEFECT_EXTRACTION`](audits/W2_ORZ_DEFECT_EXTRACTION_2026-09-07.md)
> D-1/D-2/D-3（`.gsa` shell 旁路 + 工具层 12 次拒绝 + 8 次盲重试 +
> blackboard_read 全 run 仅 3 次 + grep pattern 实证自历史恢复需求）。

## 1. 背景与需求链（一段话）

W2 压测实证：长会话模型需要恢复自身历史（train-fasttext 269 轮，grep
pattern `fasttext|MemoryError|bad allocation|g\+\+|编译|pip install` 直指
早期失败细节），正规通道供给不足（blackboard_read 折叠视图默认不含早期
失败行、黑板不可 grep、全 run 仅 3 次调用），而 `.gsa` 结构可见（
`ls -Force`）且终端日志窗口合法可读，模型自然泛化到内部区；工具层拒绝
无解释（D-3），模型视为障碍而非边界，8 次换工具/换路径/写脚本绕行，最
终经 shell 通道直读成功（D-1，31 条命令全 exit 0）。结论：缺口是框架的
信息供给面，不是模型的越界倾向。

## 2. 用户裁决（2026-09-07，逐字口径登记）

1. 「黑板得进一步补强了，黑板必须暴露模型需要的全部信息」。
2. 「我考虑将台账放开，但第一次读取会返回.gsa的职责和简短说明，指向黑
   板，并询问模型是否真的需要台账，如果模型第二次读取就放开台账」。
3. 「模型如果能分清，那ta就可以读，框架本身就是帮助模型的，不是限制
   的，有需要就开放，审计部分不怕，依旧原样做」。
4. 「重跑 train-fasttext 确实值得，修完以后重跑一轮这个吧」。
5. 「权限门拒绝后也要返回具体内容，和失败返回一致」。
6. 追加确认：两段门按「响应内容、非阻塞轮」理解（正确）；`.gsa` 访问
   状态**会话级别持久化**；首读响应带**结构预览**（好方法）；**key 必
   须要拦截**，这一内容本身就不应该出现在 .gsa 里面；**落 journal 必须**；
   **先补黑板后动 .gsa**，不然没什么意义；做 **0.3.2** 版本即可。

## 3. 设计

### A 黑板自历史补强（先落；`blackboard_read`）

- **A1 失败聚合按需面**：`section=exec` 新增 `failures_only=true`——
  返回 failure_agg 行集（复用 P2-12 行语义：F4 身份 (kind,id)、epoch
  内累计计数、首末墙钟、错误码集），上限 3K 截断标注。
  > **0p S1 复审勘误（2026-09-07）**：原文「receipt 指针」删除——
  > `FailureAgg` 不存 receipt 数据（`record()` 无此参数），行级
  > receipt 指针不存在；回查指针仅保留 domain/round 展开面（真实
  > 可得）。同批复审最小闭合：命令级失败补盖章——工具 Ok 臂且
  > `exit_code≠0` 且携带 F4 身份（failure_target 四族，
  > run_terminal_cmd/run_tests → cmd_target）时记入 failure_agg
  > （结构化 code = `exit_{n}`），聚合覆盖面与 W2 命令失败场景对齐。
- **A2 自历史检索**：`section=exec` 新增 `search=<literal>`——大小写
  不敏感**字面子串**（非正则）扫动作/结果摘要，命中 ≤20 行（行 =
  round + 摘要 + exit + receipt 指针），截断显式标注。
  > **0p S1 复审注记（2026-09-07）**：`exit` 语义落地 = `ExecEntry
  > .exit_code`（serde default 兼容旧板）——exec 行 `exit=N` 为命令
  > 真实退出码（N≠0 即命令级失败）、`exit=ok/err` 为工具级成功/host
  > 级 ToolError；行级 receipt 指针仅 receipt 行存在（order_id）。
  > 扫描源 = exec results/errors + actions receipt 摘要（receipt 为
  > 「动作摘要」在 console 订单面的落点，结果栏 50 条上限外历史唯一
  > 可检索视图）。
- **A3 工具描述教学**：描述补三句——早期轮次用 `domain/round_from/
  round_to` 展开；失败总览用 `failures_only`；找内容用 `search`。
- 边界：全部 PULL、有界响应；不新增 PUSH 注入、不改预算面。

### B `.gsa` 两段门（后落；修订 §14.56 agent-invisible 语义）

> **0p S2 实施注记（2026-09-07）**：
> ① §7 待确认点 1 落定——`resources_state.json` 按 B1 直读类实现
> （免通知直读面，与两白名单窗口同级）；
> ② **逃逸恒拒**（设计文本之外的安全语义保持）——词法在域内但
> canonical 逸出卷外的路径（种在域内的二级 symlink / 幽灵白名单
> 形态）不进入两段门，恒拒且不因通知而放开（GAP-GSA-SYMLINK-
> STALE-TEST 安全语义不被 B4 放松）；
> ③ B4「journal 记 open_after_notice」落地为 `tool_completed` v0.2
> 载荷可选字段 `session_volume_opened`（const true；非独立事件
> 类型），schema 已同步，与前序 `policy_denial{code=session_volume_
> notice}` 完成事件构成「通知→放行」审计对；
> ④ 设计 C 的 permission 源扩展覆盖内建读沙箱 deny（read_file/
> grep/list_dir；code：outside_workspace / session_volume_notice /
> session_volume_agent_invisible），schema `policy_denial` 描述已
> 修订（原文「内建桥权限拒绝不落事件」退役）；词汇与 0n 对齐：
> allow / deny / notice（session_volume_notice）/
> open_after_notice（session_volume_opened）。
> **0p S2 复审处理批注记（2026-09-07，全面复审后用户裁决直接修复）**：
> ⑤ **桥镜像让路（P1 修复）**——复审实证权限桥 `access_in_scope`
> 对 `.gsa` 内部区的冻结镜像在工具执行前恒拒（且走无 ToolCompleted
> 的 event-less 路径 = W2 D-3 原形），两段门在所有带桥生产路径不可
> 达；修复为镜像内部区拒绝臂退役、判定权单点归 orz-tools 两段门，
> 桥仅保留两窗口 canonical 逃逸守卫与 run_tests 精确文件名守卫；
> §14.56 语义修订的执法点自此真正单源。
> ⑥ **key 拦截覆盖面闭合（P1 修复）**——复审穷举证实四漏斗之外尚有
> 五条落卷路径（run_tests_output.txt 恒直读窗口 / runs/retrieval-
> results 工件 / compaction 存档 / blackboard epoch 归档 / grill 日志）
> 未接脱敏，「全卷零 sk-」判据可被证伪；修复为第 5 漏斗统一在写盘点
> 接 orz-secrets + 已知 key 值字面替换注册表（B5 明文后半落地）+
> B1 凭据区（keystore/one_shot_permit/grok-home/chrome-profile*）落
> 判决 CredentialsDenied 恒拒 + shutdown_all 补 sweep；snapshots/
> （回滚字节完整性）与 grok-home 外部写手登记为边界。
> ⑦ **opened 置位口径校准**——实现按 B4 字面「每次内部区放行都置位」
> （审计对 = 1×N），schema 描述原文「first post-notice access」随批
> 勘误；旗标按 call-id 键控修复并发批错配；通知信封增凭据区职责句、
> 结构预览补轮次范围/条目数（B2 三要素齐备）。

- **B1 区域分类**（设计默认，可调）：
  - 两段式：`ledger/**`、`runs/**`（journal 事件）、`conversations/`
    侧车；
  - 直读（免通知）：`session/terminal/*.log`、`run_tests_output.txt`
    （既有窗口）、`resources_state.json`（只读）；
  - 永久拒：凭据/秘密类（若未来出现）。
- **B2 首读通知信封**（非内容；`policy_denial` 形态，
  `code=session_volume_notice`）：`.gsa` 职责图（各区域一句话）+ 区域
  分类 + **台账结构预览**（覆盖轮次范围/条目数/字节）+ 黑板指针（
  `section=exec` 的 `failures_only`/`search` 用法一句）+ 询问句（若确
  需台账请再次读取，将直接放行）。
- **B3 状态持久化**：会话卷级（`.gsa\access_state.json` 记
  notice_shown 时间戳），跨 prompt 一次；卷缺席 fail-closed 基线不变；
  `access_state.json` 自身不受两段门管辖（机制文件）。
- **B4 二读放行**：同会话卷内后续内部区读取直接放行（read_file/grep），
  journal 记 `open_after_notice`。
- **B5 key 拦截（放开前提不变量）**：`.gsa` 全部持久化写入路径（终端
  日志/台账/会话快照/侧车/journal）统一接 orz-secrets 脱敏（sk-shape
  + 已知 key 值字面替换占位符）；回归测试：含 key 命令落终端日志 →
  全卷零 sk- 命中。key 不落卷是 B 放开的先决条件。
- **B6 shell 通道定性**：不做命令串内容检查（放弃 policing 修法）；
  shell 直读内部区定性为「跳过教育的旁路」，文档标注不对称性，不作为
  缺陷追打。

### C 权限门拒绝信封统一

- deny 一律返回结构化 `policy_denial {source=permission_gate, code,
  reason}`（与失败返回同纪律；FUS-CONSOLE-POLICY-DENIAL 积木复用），
  `ToolCompleted.policy_denial` 落 journal（闭合 D-3）。
- `.gsa` 类首读 = 通知信封（B2），二读 = 放行；其余类按 code 分类
  （如 `outside_workspace`）。
- 词汇与 0n GAP-APPROVAL-PROMPTER 对齐注记（allow/deny/notice/
  open_after_notice）。

### D 小项

- `run_finished.turn_count` 改真实会话轮计数（`controller.rs:3541`
  硬编码 1 退役；单提示 run 语义不变仍为 1，多轮会话报真实值）。

## 4. 判据（机械化核对口径）

- **黑板**：`failures_only`/`search` 在 schema/工具描述/verifier 可见；
  单测覆盖长会话 fixture（早期轮失败行可被 search 命中、failures_only
  返回聚合行、上限截断标注）。
- **两段门**：单测矩阵——首读返回通知非内容且含预览字段；状态跨 prompt
  持久（二次会话不再通知）；二读放行；窗口类与 resources_state 不受影
  响；卷缺席 fail-closed。
- **key 拦截**：回归——含 key 的终端命令 → 全卷（含终端日志/台账/侧
  车）零 sk- 命中。
- **turn_count**：单测多轮会话 N>1；单提示=1 回归。
- **重跑 train-fasttext（S4）判据表**：
  1. 证据完整性：终端日志 + artifact-manifest 随批回收齐备；
  2. 事件面：若模型触内部区——首读通知事件与二读放行事件在 journal
     可审计；deny 后同路径连续重试 ≤2（软判据）；
  3. key 拦截：全卷零 sk- 命中；
  4. blackboard_read 用量对照基线 3 次（观察项，不硬闭合）；
  5. 通用：零真实 400、命中率 ≥90%、官方墙钟唯一、交付面照旧。

## 5. 排期（S1–S4，BACKLOG 0p / TODO P0-0p）

1. **T0**：orz 版本 bump **0.3.2**（独立提交；源冻结对该批解除）。
2. **S1**：黑板补强（A1–A3）+ turn_count（D）代码与测试；orz 侧测试
   全绿 + clippy 零新增。
3. **S2**：两段门（B1–B6）+ 权限门信封（C）代码与测试；schema/verifier
   同步（policy_denial 事件面）。
4. **T2**：双平台重建（Windows 三件套 + Linux musl 顺带翻新）+ VM 同步
   + acaf manifest 重刷 + DryRun + enforcement-probe。
5. **S4**：重跑 train-fasttext（RunTag `tf-selfhistory-032`，官方 3600s
   墙钟，§4 判据表）→ 分析落 `docs/audits/`。
6. **S5 收口**：BACKLOG/TODO/索引/ADR 注记同步 + manifest 重算 + 门禁
   Exit 0。

## 6. 边界与非目标

- 不做 shell 命令串内容检查；不追打 shell 旁路（B6 定性）。
- 不为跑分特化：黑板补强与两段门均为通用长会话能力（P2-12/P2-13 谱系
  延续），不绑定 train-fasttext。
- chunk3 / 批次 L / 批次 O 自本批起以 0.3.2 为基线（随各批次运行时裁
  确认）。
- 0n approval 的交互审批呈现不在本批（词汇对齐注记级）。

## 7. 待确认点（实施中可调，不阻塞排期）

1. `resources_state.json` 按只读直读登记（可改两段式）。
2. `access_state.json` 文件名与放置（会话卷根）。
3. notice 信封的具体字段命名（schema 评审时定稿）。
