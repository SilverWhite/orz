# 241 批：0da S2 落码＋钉子——D1 清零态兜底链修复＋D2/D5 归一守卫＋清理操作多样化 replace 族在役（2026-10-11）

> **用户令（两段）**：①「命名没关系，只要通用，方便模型使用就行」（O1 终审＝clear 族命名定案）
> ②「请开始S2」。
> **性质**：落码批——orz 子树 8 文件（零契约破坏：判官/Python 镜像/schema/WORK_TOOLS 三处零改动；
> 零新事件类型；零新工具）；计数不变 59；未提交、未推送。
> 设计权威＝[`0da S1 设计稿`](../MODEL_CONTEXT_CONTROL_R2_DESIGN_2026-10-11.md)（240 批放行随 O1 裁决生效）。

## §1 实现面（按设计稿 §4–§7 逐条落地）

1. **D1 修复（model_face）**：`build_model_face` 拆除清零态无条件短路——有块清零走**常规装配**
   （`hidden_message_ranges` 的 `(0, clear_end)` 前置臂＋滑块/压缩/截断/700K 守卫全链钳制恢复，
   0cz §5.3/§9「机制不退役」成立）；`build_cleared_face` 保留为**无块清零面**专用（无块早退
   不应用 hidden 区间的三岔口）；**白名单豁免**双路径落地（常规装配前置段 `i >= clear_end ∨
   whitelist` 过滤＋主循环白名单永不隐藏；未清零 clear_end=0 零行为变化）；`estimate/count`
   与 build 三分同口径（`face_markers` 恰算一次，双重扫描＝D4 吸收）。
2. **D1 三消费点（agent_loop）**：可压集合选择／截断 live 过滤／T1 可压计数统一补
   `!is_round_cleared(first_round)`——清零块不再误入压缩/截断集合（幽灵块开窗、档案重落、
   freed 虚构消除）。
3. **D2 归一（compact）**：清零 marker 插入点＝**保留窗第一轮起点** `ranges[cur-keep].0`、
   边界参数 `cur-keep`——物理隐藏 `[0, marker_idx)`／事件信封边界／marker 文本／clear.md
   覆盖四方一致；**随钉暴露并修复档头标签同族 bug**（`clear_transcript_markdown` 档头轮次
   原传 `cur`，keep>0 时虚大——改传 `cur-keep`；keep=0 恰好重合故 0cz 未暴露）。
4. **D5 守卫（compact）**：二次清零**单调守卫**（既有边界 ≥ 新边界 ⇒ 中性 exit 0「现有清零
   边界已覆盖目标区间」）＋dump 资格补「旧边界已覆盖 ⇒ 跳过」（档案零重写）。
5. **replace 族（O1 终审＝clear 族）**：
   - `blackboard_write` op 扩 `clear`（section 单分区或 `all`；`all` **仅与 op=clear 组合合法**，
     其余显式拒绝 exit 1）——`clear_model_note_sections` 单源落点（分区清空为空列表＋版本
     计数 bump 恰 1）；journal＝plan_write 族顶层 `op="clear"`／`content_chars=0`／goal 槽
     「（清空 {section}）」；op 解析前移至 content 校验前（clear 无需 content）；**护栏不对称
     ＝无 handover**。
   - `context_manage` mode 扩 `clear_all`（一键清理）——执行序＝防裸清（handover 必填承继）
     → **先清黑板三分区（与 op=clear 同一落点函数单源）→ 后写 handover（板面恰一条
     【交接摘要】，锚点保序）** → clear 执行核承继；事件复用 `ContextCompressed`
     `mode="clear_all"` 全量载荷（增 `board_entries_removed`）。
6. **提醒面（context_scale/controller）**：权限句扩**四选择单源句**（压缩 mode=compress／清零
   mode=clear／一键 mode=clear_all／清黑板 op=clear）入六块；黑板 guide 增「一键清理／清空
   黑板分区」词条与组件关系句；声明面三处（mode 枚举 +clear_all、op 枚举 +clear、section
   枚举 +all）＋工具描述同步（clear_all/blackboard 清空教学）。
7. **0ao 豁免补登（tool_names）**：guide「清空黑板分区」教学句的生产字面按 0ao 口径具名登记
   `LITERAL_EXEMPTS` 第二条（同段已有常量拼接位）。

## §2 测试与验证

- **钉组全落（设计稿 §4.5/§5.3/§6.5）**：
  - **回落钉**（D1 核心）：清零态＋后边界闭合块打压缩 marker ⇒ face 排除块消息＋估算回落＋
    读数可压集合恰零＋块表「已清零/已压缩」双态＋**estimate ≡ build 逐条对拍**；
  - **白名单豁免钉**：跨清零保留在有块（常规装配前置段）与无块（保留臂）两路径都成立，
    原 prompt 仍随边界隐藏；无块面 count ≡ build len；
  - **keep 钉**（D2）：keep=2 ⇒ 事件边界/marker 文本/marker 落点（紧邻保留窗首声明前＝物理
    隐藏终点）/clear.md 覆盖四方归一（clear.md 档头恰「轮 1–2」、保留窗轮 3 不入档）；
  - **单调守卫钉**（D5）：keep 致边界倒退 ⇒ 工具回执中性「现有清零边界（第 3 轮）已覆盖目标
    区间」＋零新 marker＋零边界事件；正常二次清零（边界前进）⇒ 生效；
  - **replace 族钉**：op=clear 单分区清空（exit 0＋entries_removed＝2＋journal op=clear/
    content_chars=0）＋section=all 三分区全清（revisions 4＝写2+clear+all-clear）＋all+append
    显式拒绝＋非法 op 拒绝（D3 e2e）＋clear_all e2e（板面恰一条【交接摘要】＋
    board_entries_removed=2＋boundary=4＋marker＋一键回执信封）；
  - **权限句钉**：四选择七关键词断言＋六块各携。
- **读数**：orz-loop **869/0＋1 环境性预存**（869 过＝864＋5 新钉；1＝`user_cancel` 30ms 取消
  窗竞速——**stash 干净 HEAD 3/3 同败实证＝环境性劣化非本批引入**，185/238 批「负载敏感、
  冻结树同败」先例同款）；orz-assurance **301/0**（0ao 豁免登记后转绿）；orz-host/orz-bin 编译
  过；触碰面 fmt 净；clippy **96＝96 零新增**（stash 干净树对拍）。
- **实现批修正三件（如实记）**：①D2 随钉暴露 `clear_transcript_markdown` 档头标签同族 bug
  （§1-3）；②`parse_clear_boundary` 前缀判断与 `parse_clear_boundary` 内部检查重复致 clippy
  collapsible-if——合并为单一 `if let`；③单调守卫 `if let`＋`if` 嵌套改 `is_some_and`。

## §3 预存问题处置（如实记）

- **`user_cancel_closes_pending_activations_before_run_cancelled`**：本批全程稳定失败（隔离
  5/5＋全量套件），**stash 干净 HEAD 复测 3/3 同败** ⇒ 环境性（30ms 取消窗 vs 运行进度竞速，
  与晨间 867/0 全绿时点相比机器节奏劣化）——非本批引入，不在本批处置（185/238 批先例同款）。
- **D6 勘误落账**：[`238 批档`](238_0CZ_S2_CONTEXT_MANAGE_IMPL_2026-10-11.md)尾部补勘误注记
  （loop 864→867；「13 文件」＋12 个纯 rustfmt 漂移＝提交时 25 文件口径）。

## §4 批序

→ **S3 载体重建进体**（与 0cz S3 合并同一次重建，待载体重建令；字节判据＝mode 枚举
+clear_all＋op 枚举 +clear＋section 枚举 +all＋权限句换版＋声明描述扩写）→ **S4 狗粮长轮
实测**（并入 0cz S4／0cy 联动轮；判据＝D7 五观察项〔任务漂移／清零后再触兜底对照／二次清零
形态／keep 使用形态／replace 族四操作使用分布〕＋0cz 既有判据）。

## §5 台账

- 本档：`docs/audits/241_0DA_S2_CONTEXT_MANAGE_R2_IMPL_2026-10-11.md`。
- BACKLOG：`0da` 批序行 S2 达成注记；TODO：`P1-0da` S2 勾选；BACKLOG 第二卷 §1.187。
- 索引：头行 v4.213 → **v4.214**；§6/§8 `0da` 条目 S2 注记。
- 门禁 `scripts/check_repository.py` ⇒ 落账后重跑（预期 errors=1＝orz 子树 dirty）。

## §6 关联与关键词

[`设计稿`](../MODEL_CONTEXT_CONTROL_R2_DESIGN_2026-10-11.md)／[`240 批档`](240_0DA_S1_DESIGN_2026-10-11.md)／
[`239 批档`](239_0DA_MODEL_CONTEXT_CONTROL_R2_2026-10-11.md)（审查权威）／
[`238 批档`](238_0CZ_S2_CONTEXT_MANAGE_IMPL_2026-10-11.md)（处置对象＋勘误注记）。

关键词：241 批、0da S2 落码、D1 清零态兜底链修复、常规装配叠清零区间、白名单豁免、无块清零面、
三消费点过滤、回落钉、D2 marker 插入点归一、clear.md 档头标签修复、D5 单调守卫、
mode=clear_all、op=clear、section=all、clear_model_note_sections、四选择权限句、LITERAL_EXEMPTS
补登、869/0＋1 环境性预存、301/0、clippy 96 零新增、计数不变 59、索引 v4.214。
