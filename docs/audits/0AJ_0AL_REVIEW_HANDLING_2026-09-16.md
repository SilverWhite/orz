# 0aj／0al 独立复核与问题处理（2026-09-16）

> 入口：用户 2026-09-16 指示「请对当前实现的 0aj 和 0al 部分进行全面检查，包括
> 设计合理性、实现合理性、设计与实现的符合性」→「请先直接对审查出的全部问题
> 进行处理」。复核对象：orz `12396e6a`（0aj 修复）／父仓 `3846bb17`（0aj 载体
> ＋0al 修复）／账本 v3.47–v3.48。本批**不新增未闭合项**（计数维持 36）。
> 关联：[`0aj／0al 摩擦修复与 0.5.3 载体`](0AJ_0AL_FRICTION_FIX_AND_053_CARRIER_2026-09-16.md) /
> [`0AE 深审（QUAD 批）`](QUAD_BATCH_DEEP_REVIEW_2026-09-15.md) /
> [`ADR-0010 §14.67`](../../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)。

## 1. 独立复核读数（主会话复现，非转抄）

- **run `RUN-CLI-6aa999d6`（2097 事件）**：`blackboard_write` 4 次
  `permission_requested{risk:"ReadOnly"}` → 4 次 `permission_decision{deny}`
  （seq 128/129、148/149、216/217、632/633）；`plan_write` **0** 条；
  `tool_started` 255 vs 权限事件 259（差的正是被拒在派发前的这 4 次）；
  `request_header_change` 声明 **7** 件（含 `blackboard_write`）vs
  `tool_availability_check.complete` **5** 件 —— 声明面/探针面脱同步的实证。
- **三处工作工具表逐项一致**（顺序与内容）：`orz-loop/src/tool_probe.rs`、
  `orz-assurance/src/journal/families.rs`、`assurance/run_event_journal_validation.py`。
- **测试实跑**：orz-loop `--lib` **821/0/3**（819 → +2 新钉）；orz-host
  `permission::tests` **20/20**；orz-assurance `--lib` **229**；
  `assurance/tests/test_gate_root_anchor_nails.py` **7/7**（4 → +3 新钉）；
  `runtime/tests/test_run_event_journal_validation.py` **262 passed**；
  门禁 `valid: true`。既有无关红灯 `test_v02_all_51_event_types_covered`
  （`65 != 55`，账本既有登记）形态不变。
- **载体进件复算**（`D:\tb-eval\orz-windows`，只读）：三件 SHA256 与审计表逐字相符
  （`orz.exe 79cbb1d6…`／`orz-signer 969e7e66…`／`orz-acaf-provision 68f26118…`）；
  `orz.exe` 内 `blackboard_write` 字面量 **0.5.2-bak = 94 → 0.5.3 = 95**；
  `.0.5.2-bak` 备份先于覆盖存在（备份纪律可查）。
- **0al A/B（真实克隆形态）**：把 0ai 狗粮克隆整树复制到临时目录后——修复前脚本
  `ValueError: 'D:\CLI\runtime\acp-initialize-event-payload-v0.1.schema.json' is not in
  the subpath of '<克隆根>'`（与 agent 报告 F2 逐字同形）；修复后脚本不崩、无错误树，
  剩余 8 条错全为克隆既有事项（TER 失效链接 ×7 ＝ 既有摩擦 F6；1 条为复刻时剔除
  `.git` 导致 orz 子模块清单不可列）。独立探针另复现：修复前形态
  `ref_module = D:\CLI\assurance\…`；修复后 `ref_module = <克隆>\assurance\…` 且锚定复核返回空集。

## 2. 审查发现与处理

| # | 级别 | 发现 | 处理（本批） |
|---|---|---|---|
| A1 | P2 | 根因归因不完整：漏网有**两个**直接原因（跨表护栏样本表漏列 ＋ 2026-09-15 深审 §4.1「orz-host 无需改动」误判），原审计只记了前者 | 深审 §4.1 **就地更正**；本批审计 §1.1 归因补全，并统一「同形**第四例**」口径（按 ADR-0010 §14.66 计数：project_doc_index／browser_read／browser_control／blackboard_write） |
| A2 | P2 | 跨表护栏仍是**手写样本表**（补样本不等于消除手工——第四次同形风险仍在） | **单一源 + 遍历式**：控制器侧新增 `ToolDispatcher::READ_ONLY_EXEMPT_TOOLS` 并驱动 `risk_class`；宿主护栏遍历该表，另断言「代表参数表**恰好覆盖**单一源」（漏补参数同样报红）；同批新增**声明面分类护栏**（声明面工具必须 ∈ 工作工具 ∪ 规则式非工作族，含 `RUN-CLI-6aa999d6` 7 件冻结样本） |
| A3 | P3 | `families.rs::verify_policy_denial` 注释「EXACT name set（26 entries）」过期（实为 23+4=27 → 24+4=28） | 注释改为派生式表述（不再写死会过期的数字） |
| A4 | P3 | fixture 生成器 `tool_availability_check` 分区未随 23 → 24（「全分区」样本失真） | 生成器两处 payload ＋ 3 件生成物同步补 `blackboard_write`；逐文件 SHA256 与生成器新输出**全部一致**（见 §3） |
| A5 | P3 | ADR-0010 §14 自 §14.66（2026-09-11）起停更；0aj 按 0v F1 先例本应有条目；§14.66 第 10 项「第三次」现已是第四次 | 追加 **§14.67 / v1.68**（0aj／0al 复核处理与加固转录）；§14.66 第 10 项就地注记第四次更正 |
| A6 | — | 0aj 判据要求「无头 run 内 allow → `plan_write` → 读回」，本批无 run | 维持 `pending`（用户已指示暂不开始新狗粮线）——判据链「表级＋端到端」两半已机械绿，剩余仅实跑取证 |
| B1 | P2 | 0al 判据未定义「冻结克隆」的构造方式，而两种构造结论相反：git 派生克隆在本仓库**必然红**（门禁链接检查依赖未入库工作件；实测 tracked-only 克隆 1745 条 broken link） | 三面账本判据行补口径「**整树复制（保留未入库工作件）**」；本审计 §1 与 §4 同步 |
| B2 | P3 | `main()` 异常路径丢弃已收集 errors ⇒ 错误树场景下 `gate would validate the wrong tree` 诊断会被吞 | `check_repository()` 锚定 **fail-fast**（错误树立即收口返回，只带锚定错误）＋ B2 钉子（`error_count==1`、`counts=={}`、含 `wrong tree`） |
| B3 | P3 | 门禁临时目录排除仅 `("tmp",)` 前缀，`.tmp*` 被当仓库内容（实测可把门禁打成 `valid: false`） | 前缀补 `.tmp` ＋ B3 钉子（`.tmp*`／`tmp*` 排除、真实内容不过度排除） |
| B4 | P3 | 锚定判据用字符串比较，`-m` 形态下 cwd 与 ROOT 字符串形态不同 ⇒ 重复插入同树条目 | 改按**解析后路径**比较 ＋ B4 钉子（同树位置 0 时不新增 sys.path 项） |

## 3. A4 的等价性取证（为什么可以手工同步 fixture）

本批**未整体重跑**生成器（原因见 §4-O1）。等价性按「生成器新输出 ↔ 入库文件」逐件
SHA256 对照：

| 文件 | 入库 SHA256 | 生成器新输出 | 结论 |
|---|---|---|---|
| `runtime/fixtures/run-event-v0.2/payloads/tool-availability-check.minimal.valid.json` | `D22225D6…C3DBD` | 同 | MATCH |
| `runtime/fixtures/run-event-v0.2/payloads/tool-availability-check.constraint.invalid.json` | `48A5D1F2…3FA47` | 同 | MATCH |
| `runtime/fixtures/run-event-v0.2/envelope/tool-availability-check.valid.json` | `931C1501…35414` | 同 | MATCH |

对照方法：把父仓 `scripts/`＋`assurance/`＋`runtime/` 复制到临时目录（生成器按脚本
位置推导 ROOT），在该副本内跑生成器，再逐件比对哈希（临时副本已清理，仓库未受影响）。
生成器定义两处（`PAYLOAD_GOOD_V02`／`PAYLOAD_BAD_V02`）与三件生成物同步补入
`blackboard_write`，位置与 `WORK_TOOLS` 同序（紧随 `blackboard_read`）。

## 4. 过程新观察（未立项，留用户裁决）

**O1（P2）**：`scripts/generate_run_event_fixtures.py` 的 docstring 自称
“the single source of truth for the good/bad fixture shapes”，但**重跑会删除 38 个
已入库 fixture**（0z 资源族 `host-resource-denied`／`host-resource-snapshot`／
`process-tree-reaped`／`reclaim-performed`／`resource-exhausted`／`resource-limit-hit`、
0ac 检索族 `retrieval-progress`／`retrieval-result-segment`／`result-delivered`、
`run-terminated` 等；生成器源码对这些事件名**零命中**）。取证：临时副本内跑生成器 ⇒
fixture 文件数 **347 → 309**（差 38，恰为上述集）。建议二选一：①把这些 payload 表
并入生成器（恢复 single source of truth）；②把生成器 docstring 与 README 口径改为
「部分来源」，并登记手工维护集。**是否立项留用户裁决**（本批计数不动）。

## 5. 未闭合与遗留（如实登记）

1. **0aj／0al 判据闭环**：仍搭下一轮狗粮 run 收取（用户已指示暂不开始新狗粮线）。
2. **orz 源超前于 0.5.3 载体**：本批 orz 改动为**行为中性**（ReadOnly 名单单一源重构 ＋
   测试 ＋ 注释），0.5.3 仍是 0aj 行为的忠实制品；下一次需要载体时随批 bump 重建
   （Linux musl 三件套与 GitHub Release 一并补齐）。
3. **O1**（生成器与入库 fixture 树分叉）未立项，见 §4。
4. **既有无关红灯**（形态不变，非本批引入）：`test_v02_all_51_event_types_covered`
   （`65 != 55`）；`assurance/tests/test_retrieval_subagent_real.py::
   test_search_p3_action_authorization`（排位漂移）；orz-tui 三例夹具红。
