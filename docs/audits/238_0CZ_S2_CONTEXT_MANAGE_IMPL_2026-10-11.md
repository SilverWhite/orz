# 238 批：0cz S2 落码＋钉子——context_manage 第十一主工具（方案 A：压缩承继＋主动清零＋黑板全量编辑）（2026-10-11）

> **用户令（两段）**：①「就按照方案A走吧」（O1 裁决＝`context_compress` 声明面退役吸收）②「请开始S2」。
> **性质**：落码批——orz 子树 13 文件（零契约破坏：journal schema v0.2 判别规则零改动）；计数不变 58；
> 未提交、未推送。设计权威＝[`设计稿 v1.0`](../MODEL_CONTEXT_CONTROL_DESIGN_2026-10-11.md)（S1 放行随 O1 裁决一并生效）。

## §1 实现面（按设计稿 §4–§7 逐条落地）

1. **tool_names 单源**：`CONTEXT_MANAGE_TOOL_NAME` 入表（第十一；例外 +3 注记）；
   `CONTEXT_COMPRESS_TOOL_NAME` 注记声明面退役、常量与表位保留（历史 journal 回放，同
   `compaction_whitelist_add` 封存形态）。
2. **WORK_TOOLS 三处同批**（0aj/0ap 先例照抄；判别规则零改动）：判官表
   （`journal::families`）＋探针面（`tool_probe::WORK_TOOLS` 25→26、probe_storage 判据）
   ＋Python 镜像（`assurance/run_event_journal_validation.py::_WORK_TOOLS`）。
3. **声明面（controller）**：`context_manage` 无条件声明（ReadOnly 类）——参数
   `mode ∈ {compress, clear}`（必填）＋`whitelist`（0am FR4 承继，≤16 条/16K）＋
   `handover`（clear 必填 ≤8K，防裸清）＋`keep_recent_rounds`（默认 0，≤16）；
   `blackboard_write` 增 `op ∈ {append, replace_all}`（默认 append 零回归）。
   **方案 A：`context_compress` 注入块同批退役**（管线/白名单/事件由 context_manage 承继）。
4. **派发（tool_run）**：mode 分流——compress＝0ap 原逻辑逐字保留（三态防抖＋whitelist
   落盘链）；clear＝执行核 `apply_context_clear`（compact.rs）：边界计算（当前轮声明处）→
   防裸清校验（缺/空/超长 ⇒ 拒绝 exit 1；无可清 ⇒ 中性 exit 0）→ **交接写入黑板先行**
   （notes 分区【交接摘要】盖章条目；fail-closed）→ 回放落盘（已闭合 Live 块逐块＋边界前
   现场整段 `{tag}-clear.md`；best-effort 失败如实标注）→ 边界 marker 插入（**只插入**，
   I3 本地面零覆盖）→ 软水位键重置；事件＝`ContextCompressed{mode=clear}` 全量载荷
   （boundary_round/trigger/target/rounds/blocks/freed/archive_write_failed/handover）。
5. **投影层（model_face/prompt）**：`[前文上下文已清零 v0.1-清零]` marker——注册
   `is_injected_block_text`（绝不持久化）＋`is_restore_retained_block`（恢复重建，零新
   侧车字段）；`BlockState::Cleared`＋`FaceMarkers.cleared_before_round`（最后 marker
   优先）＋**`clear_marker_index` 权威边界**（实现批修正：marker〔role=User〕会被轮区间
   吸进前一轮，轮端点推导会吞掉 marker 本体——以 marker 消息下标为界，轮推导仅作回退；
   设计稿 §5.1 的「marker 在边界轮起点」在此精确化）；build/estimate/count 三处清零态
   同口径装配（白名单存活＋指针＋边界起逐字＋D4＋块表）；块表「已清零」态；滑块读数
   清零块不进可压集合。I1 显式例外落地（模型主动＋交接前置＋边界可回放；清零区间不受
   滑块钳制，钳制对压缩/截断块照旧）。
6. **黑板全量编辑（blackboard）**：`NoteEntry.op`（serde default 零迁移）＋
   `replace_model_note_section`（整节替换单一新条目＋版本计数 bump＝「整体替换计 1」
   先例）＋渲染【整节重写】/【交接摘要】章；journal＝既有 plan_write 族顶层增 `op`
   （判官/镜像零约束，重放即审计——设计 §6.3「零新事件类型」）。
7. **兜底联动一步提醒（context_scale）**：`context_permission_line()` 单源句（点名
   context_manage/mode=clear/handover/blackboard_write）被**六块**提醒引用（soft／hard／
   H1 窗口／T1 必定压缩两级／T1 截断告知／700K 守卫——设计稿列四块，实现批按「全部
   压缩截断提醒面」口径覆盖六块，防漏面）；H1/T1 文案的工具名随方案 A 改指
   context_manage（mode=compress）；黑板 guide 增「清零」词条＋组件关系句；
   `action_category`＝「other」折叠（与压缩族同款，防误折「read」）。

## §2 测试与验证

- **六钉全落**（设计稿 §10 六钉全数在案）：①防裸清负例（无 handover ⇒ exit 1＋零事件
  ＋零 marker＋黑板零写入）②本地面零 diff（I3——原消息逐字保留＋恰一枚 marker 插入）
  ③marker 恢复重建（反解边界＋注入/retained 双注册＋同数组重放同面）④replace_all
  （恰一条＋版本计 1＋【整节重写】＋旧 JSON 零迁移）⑤单源权限句（七处断言：句本体
  ＋六块各携）⑤′水位键重置（软档＋first_block 复位、320k 保留）⑥字节单调（清零现场
  存档＋按块回放落盘在案＋逐字含被清内容）。
- **读数**：orz-loop **864/0**（3 ignored）；orz-assurance **301/0**；orz-host/orz-bin
  编译过；触碰面 fmt 净；clippy 零新增（告警全在预存 doc 缩进族）。
- **断言随批更新七处**（方案 A 面名单/派发）：面投影 ×3（benchmark/readonly/R1 list）、
  探针 ×2（membership 25→26、snapshot complete 列）、compress e2e ×2（whitelist 与
  端到端改走 context_manage mode=compress，管线断言零改动）。
- **工程修正两件（实现批如实记）**：①marker 边界以消息下标为权威（轮端点推导吞
  marker，见 §1-5）；②dump 资格按边界轮直接比较（原用 is_round_cleared 判后置
  marker＝鸡生蛋，恰零 dump）。

## §3 预存问题处置（如实记）

- **0ao 扫描钉 HEAD 红**：`blackboard_write` 字面在 exec_policy 备份指引文案（204 批
  `93672eca` 的 0ct 214 考虑项落地漏登豁免；stash 实测 HEAD 预存、非本批引入）——
  本批按 0ao 口径**首次登记 LITERAL_EXEMPTS**（逐条具名＋理由），钉转绿。
- **`user_cancel_closes_pending_activations_before_run_cancelled` 预存红**：时序敏感族
  （30ms 取消窗 vs 60ms 分块流；负载敏感）——stash 实测干净树同红，与本批零关涉，
  185 批「冻结树同款先存」先例同款，不在本批处置。

## §4 批序

→ **S3 载体重建进体**（待下一载体重建令；字节判据＝声明面 +1 context_manage／−1
context_compress＋marker 双注册＋op 参数面）→ **S4 狗粮长轮实测**（前置＝0cy S1+S2
落地〔orz 承接〕；判据＝使用率/时机/清零后成绩维持〔对照 235 批仅台账组基线〕/handover
质量/与兜底触发交互）。

## §5 台账

- 本档：`docs/audits/238_0CZ_S2_CONTEXT_MANAGE_IMPL_2026-10-11.md`。
- BACKLOG：`0cz` 批序行 S2 达成注记。
- TODO：`P1-0cz` S2 勾选〔238 批〕＋S3 待令。
- BACKLOG 第二卷：§1.184。
- 索引：头行 v4.210 → **v4.211**；§6 `0cz` 条目 S2 达成注记；§8 pending 桶同步。
- 门禁 `scripts/check_repository.py` ⇒ 落账后重跑：**errors=1＝`orz submodule working
  tree is dirty`**——即本批 S2 落码的未提交中间态（13 文件在役待提交令；manifest 对
  HEAD digest 全合、零 mismatch），随用户提交令消除；其余全绿（头行/计数行长度、链接
  存在性、digest 对账）。

## §6 关联与关键词

[`设计稿`](../MODEL_CONTEXT_CONTROL_DESIGN_2026-10-11.md)／[`237 批档`](237_0CZ_S1_DESIGN_2026-10-11.md)／
[`236 批档`](236_0CZ_MODEL_CONTEXT_CONTROL_2026-10-11.md)／0ap（承继）／0am FR4（whitelist）／
0cu（findings 同构）／0ct（读开放＝清零可回读前提）／0ak（会话持久化＝逐字留档承载）。

关键词：238 批、0cz S2 落码、context_manage、第十一主工具在役、方案 A 吸收退役、
清零 marker、clear_marker_index、防裸清、handover 强制交接、op=replace_all、零新事件类型、
单源权限句、六钉、864/0、301/0、LITERAL_EXEMPTS 首登、计数不变 58、索引 v4.211。

---

## 勘误注记（2026-10-11 补记，随 0da S2〔241 批〕落账；正文不改动）

1. **读数勘误**：本档「orz-loop **864/0**」→ 复核实测主靶 **865**（合计 867/0；3 ignored 一致、
   零失败；疑记账时点差异）。239 批全面审查以 867/0 为准。
2. **文件数补记**：本档「orz 子树 13 文件」为 0cz 实质触碰面；同期工作树另有 **12 个纯
   rustfmt 漂移文件**（console/console_exec/acaf/delivery/gateway-fake/serp/disposition/
   controller_test_support/credential/activation/lif_replay/rli_forecast_probe；去空白比对
   逐一核实为纯格式化、零语义），随提交一并进版本——提交时计数口径＝25 文件。
