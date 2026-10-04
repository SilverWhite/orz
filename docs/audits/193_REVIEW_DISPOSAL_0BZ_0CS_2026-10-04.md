# 193 批：审查处置——0bz S3′＋0cs S1 全面审查发现全量处置（2026-10-04）

> **用户令**：「请对当前完成的0bz S3和0cs S1进行全面审查……」〔审查轮〕→「请对审查出的全部问题进行处理」〔本批〕。
> **审查形态**：两路并行只读子代理深查（0bz＝model_face.rs D4 移尾；0cs＝registry/types.rs did-you-mean）＋主会话亲核两处关键发现＋**离线对账脚本独立复跑**（`bz_s4_offline_v3.py` 对 run7/run8 全语料：GRAND comp=16／with_+2=10、9 次 div@3（D4 槽）＋1 次 div@50 尾刷新（hit 187,008）——与 192 档 §1.1 逐格吻合；noop=0、唯一深塌陷＝T1 硬截断标记）。
> **审查总评**：两批均通过；源码无 P0/P1；**1 项 P2（0bz 批档声明失真）＋约 10 项 P3**；0cs 重点疑点（client 前缀失配）排除（裸名口径全链一致）。
> **性质**＝落码＋勘误批；orz `54667109`（4 文件，+157/−28）；计数不变 60。

---

## §0 结论速览

| 面 | 结果 |
|---|---|
| 0bz 处置 | P2-1 勘误入档（192 档 §6）＋语义面交换与常态尾巴 +\|D4\| 成本补记（192 档 §6）＋旧口径注释同步 9 处（model_face 5＋agent_loop 2 先存漂移顺带＋tool_run 2）＋新钉变体 `d4_rerender_diverges_only_at_its_tail_slot_with_marker`＋原新钉补「已成块」显式断言 |
| 0cs 处置 | 表命中回显**活注册名**替代表内字面（生产行为微修，message 文案面；零契约面）＋钉＋3（`levenshtein_only_arm` 隔离臂／`table_hit_echoes_live_casing` 活名回显／`two_candidates_render_both` 双候选信封形态）＋钉 10 补 `kind == NotFound` 显式断言＋勘误共前缀 5→4＋两观察登记（表查找大小写加宽／表命中独占短路阴影） |
| 台账处置 | TODO P1-0bz **S2 框勾选**（闭合口径统一＝判据「定位到具体段落」已由 192 批离线对账满足，真机单轮采集由离线形态替代，与 BACKLOG「S2 定位达成」同口径；193 批追认）＋TODO P1-0bz S3′ 行/P2-0cs S1 行处置注记＋BACKLOG `0bz` 状态行/`0cs` 批序行处置注记 |
| 随批勘误 | BACKLOG 头部计数行 192 批后未同步本批指针（仍记 191 批）＋190 批条款重复两次——一并修正 |
| 验证 | orz-tools **3012/0**（3009＋3 恰新增）；orz-loop **851/1**（851＝850＋1 新变体钉；1＝`user_cancel_closes` 在案 30ms 竞速偶发，**stash 冻结树同败**如实登记）；orz-assurance **301/0**（未触碰读数有效）；clippy **107=107** 零新增（`-p orz-loop -p orz-tools --all-targets` stash 对拍）；触碰面四文件 fmt 净（types.rs 一处 and_then 折行当批修复） |
| 源清单 | 随 orz `54667109` 再生成：差异 4 行（types.rs＋model_face.rs＋agent_loop.rs＋tool_run.rs 哈希） |
| 边界 | 不重建不推送；0cs 的 message 文案面微修随 S2 重建批（0.8.14）进载体；0bz 机械判据（+2 针归零）仍待修复后载体真机轮（S4） |

## §1 审查发现清单与处置对账

| # | 级别 | 线 | 发现 | 处置 |
|---|---|---|---|---|
| 1 | P2 | 0bz | 192 档 §2.3「顺带修复有 D4 无 ledger 组合丢 D4 边臂」系对父代码误读——父码两路径 ledger/D4 均独立 `if let`，d4-only 组合从不丢 D4；实为位置统一 | 192 档 §6.1 勘误（原表述退役）；本批档 §2 亲核证据 |
| 2 | P3 | 0bz | D4 移尾的语义面交换（显著性）未在档面登记 | 192 档 §6.2 补记 |
| 3 | P3 | 0bz | 常态尾巴每轮重价面 +\|D4\| 未量化 | 192 档 §6.3 补记 |
| 4 | P3 | 0bz | model_face.rs 三处函数 doc/块注残留「指针＋D4 头部」旧口径（与同文件新构成式矛盾） | orz `54667109` 同步（§2.1） |
| 5 | P3 | 0bz | agent_loop.rs 两处构成式先存漂移（旧序「分块表＋D4」） | orz `54667109` 顺带收口（§2.1） |
| 6 | P3 | 0bz | tool_run.rs 两处协议形状注释「常驻头：指针＋D4」旧口径 | orz `54667109` 同步（§2.1） |
| 7 | P3 | 0bz | 新钉 fixture 无 marker 在场而断言文案称「含 marker」；且未显式断言已成块（开窗路径） | 新钉变体＋成块断言（§2.2） |
| 8 | P3 | 0cs | 钉 6 非隔离（`run_terminal_cpt` 共前缀 14 亦经 prefix 臂），全测试集无仅-Lev 用例 | 新增 `levenshtein_only_arm`（§2.3）＋钉 6 doc 注记 |
| 9 | P3 | 0cs | 批档共前缀算术差 1（「run_c=5」应为 `run_`=4） | 191 档 §6.1 勘误＋源码钉注释同步 |
| 10 | P3 | 0cs | 表命中回显表内字面 casing（name_override 大小写变体场景二次误导） | 修复回显活注册名＋`table_hit_echoes_live_casing` 钉（§2.3） |
| 11 | P3 | 0cs | 钉 10 未显式断言 kind==NotFound | 补断言（§2.3） |
| 12 | 观察 | 0cs | 表查找大小写不敏感略超声明字面（无害加宽） | 191 档 §6.2 登记，不修码 |
| 13 | 观察 | 0cs | 表命中独占短路对未来注册演化的阴影效应 | 191 档 §6.3 登记备查（受表长钉＋随批证据门约束） |
| 14 | 观察 | 台账 | TODO P1-0bz S2 框 `[ ]` 与 BACKLOG「S2 定位达成」闭合口径歧义 | S2 框勾选＋口径注记（§3） |
| 15 | 勘误 | 台账 | BACKLOG 头部行 192 批后未同步＋190 批条款重复 | 随批修正（§3） |

## §2 落码明细（orz `54667109`，4 文件 +157/−28）

### 2.1 注释同步（零生产行为变化）

- **model_face.rs 5 处**：`build_model_face` doc（常驻头＝指针，0bz S3′ 后 D4 落窗口尾）；`blocks.is_empty()` 臂块注首句；`estimate_model_face_tokens` 计数注（指针头部＋D4 窗口尾，计数与位置无关）；`try_clone_messages_with_resident_head` doc（指针注入点＋D4 归尾口径）；钉①注释（指针 face[1]＋D4 尾位）。`0bc S2④` 预留上界注（前置＋指针/D4＋尾部消息＋块表＝+4）为计数口径不动。
- **agent_loop.rs 2 处（先存漂移顺带收口）**：阶梯①装配构成式与投影层视图构成式，旧序「分块表＋D4 机械段＋各分块＋主滑块」改实际装配序「各分块＋主滑块＋D4＋分块表＋结束自述行」并引 model_face 模块头。
- **tool_run.rs 2 处**：两处协议形状注改「user＋指针（常驻头）＋assistant declaration＋tool result＋D4（窗口尾）」。

### 2.2 0bz 钉子（＋1 变体＋1 断言）

- `d4_rerender_diverges_only_at_its_tail_slot`：新增 `!blocks_outside_slider(...).is_empty()` 显式断言（钉面＝开窗路径）；注释精确化（本 fixture 无 marker，marker 形状见变体钉）。
- **新** `d4_rerender_diverges_only_at_its_tail_slot_with_marker`：fixture＝conversation(60, 8K) 成块后按生产 `insert_at` 口径插压缩 marker；断言链＝已成块 ∧ marker 在 face 内 ∧ 面长不变 ∧ 首分歧 ≥ len−3 ∧ `after[div]`＝新 D4 ∧ 其前（含 marker 与历史）逐字节相等——生产第 2 针的准确形状（压缩落地后 +2 请求）自此有直接机械锁。

### 2.3 0cs 修复与钉子（＋3 钉＋1 断言＋1 生产行为微修）

- **表命中回显活注册名**：`not_found_suggestions` 表命中臂由 `.any(...).then_some(*target)`（回显表字面）改 `.find(...).copied()`（回显活注册拼写）——`name_override` 大小写变体场景下提示指向真正可派发的名字，不诱导二次 case-sensitive not-found。message 文案面微修，`ToolErrorKind::NotFound`/`ToolId`/`details` 形状/schema 全不动＝零契约面；随 S2 重建批（0.8.14）进载体。
- **新** `not_found_suggestion_levenshtein_only_arm`：`rn_terminal_cmd`→`run_terminal_cmd`（共前缀 1＜8、编辑距离 1——仅 Lev 臂可达，隔离钉）。
- **新** `not_found_suggestion_table_hit_echoes_live_casing`：face 含 `RUN_TERMINAL_CMD` 变体时 `run_command` 提示回显活名。
- **新** `not_found_error_two_candidates_render_both`：双候选 `"a" or "b"` 信封形态＋距离排序实证（共前缀同 14、距离 1<2 定序）。
- 钉 10 补 `assert_eq!(err.kind, ToolErrorKind::NotFound)`；钉 6 doc 注记非隔离；`not_found_suggestion_table_arm` doc 共前缀勘误 `run_c`=5 → `run_`=4。

## §3 台账处置明细

- **TODO P1-0bz S2 框 `[ ]` → `[x]`**：闭合口径统一＝判据「把三源定位到具体段落」已由 192 批离线对账满足（零新跑批），真机单轮采集由离线形态替代，与 BACKLOG「S2 定位达成」同口径；193 批追认。终验义务不减免（S4 框仍开放）。
- TODO P1-0bz S3′ 行、P2-0cs S1 行追加 193 批处置注记；TODO 头部计数行本批指针 192→193。
- BACKLOG `0bz` 状态行追加审查处置条款；`0cs` 批序行追加处置条款；BACKLOG 头部计数行本批指针同步（顺带勘误 192 批漏更＋190 条款重复）。
- 192 批档 §6（勘误＋补记）、191 批档 §6（勘误＋观察登记）随批落。
- 索引头行 v4.164 → v4.165（本批为本批条目）。

## §4 验证明细

- orz-tools **3012/0**（3009→3012＝＋3 恰钉数；6 ignored 同基线；fmt 折行修复后复跑仍 3012/0）。
- orz-loop **851/1**：model_face 子套 **21/0**（20＋1 新变体钉）；全量唯一失败＝`user_cancel_closes_pending_activations_before_run_cancelled`（`dispatch.rs:4004`，30ms 竞速在案偶发，111/184/192 批登记）——**stash 冻结树同败**本批如实复证。
- orz-assurance **301/0**（未触碰，读数有效）。
- clippy **107=107** 零新增（`cargo clippy -p orz-loop -p orz-tools --all-targets` 工作树 vs stash 冻结树对拍）。
- fmt：触碰面四文件 `rustfmt --check` 净（types.rs `and_then` 折行当批修复后复检）。
- 源清单随 orz `54667109` 再生成：`orz_source_manifest.sha256` 差异 4 行＝本批四文件哈希。

## §5 边界与后续

- **0bz S4 终验**：机械判据（+2 针归零、空跑零分叉）仍待修复后载体的真机轮（本批不改变该排期）；marker 变体钉与成块断言强化了真机前的机械前提。
- **0cs S2 载体**：随下一重建批（0.8.14，先于 0cr S2）——本批的活名回显微修随载体进体；0cr 冻结面载体版本号随重建批顺延。
- **登记观察维持**：`face_d4_block` epoch 冻结（I6）重渲时序精确交错仍登记观察（移尾后不影响成本面）；0cs 两观察（大小写加宽／表短路阴影）留档不动作。
- 不重建不推送。
