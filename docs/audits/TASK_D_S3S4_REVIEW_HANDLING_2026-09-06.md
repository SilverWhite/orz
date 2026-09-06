# 任务 D S3/S4 翻转批复审处理审计（2026-09-06）

> **性质**：S3/S4 翻转实施批（父仓库 `20fd762` / orz `5053bc7f`）三路全面
> 复审的处置登记——A 路（语义符合性 + 设计合理性 + 生产面核实）与 B 路
> （实现质量）由主代理直审（子代理通道持续并发受限，三次重试未成），C 路
> （治理登记与数字核验）由子代理完成。用户指示：审查出的全部问题直接处理。
> **处置总表**：P1×1 修复 + P2×4 修复/勘误 + P3×8 采纳或登记，无 P0。
> **关联**：[翻转实施审计](TASK_D_S3_S4_FLIP_IMPL_AUDIT_2026-09-06.md) /
> [退役边界深挖](TASK_D_S3_S4_PYTHON_RETIREMENT_BOUNDARY_2026-09-06.md)。

## 1. 复审结论（三路）

| 路 | 维度 | 结论 |
|---|---|---|
| A | 语义符合性 | **PASS**——三项裁决（独立 CLI / v0.1 纳入 / 方案 α）逐条落实，无夹带（orz 2 文件 +219 / 父仓库 12 文件 +272−355 全在声明范围） |
| A | 设计合理性 | **PASS with notes**——权威链无循环、exit 契约覆盖全部失败形态；「冻结 reference」为标注级约束（α 口径自选，登记 B 边界） |
| A | 生产面核实 | **PASS**——门禁零残留 Python 判官调用、双轨 18 期刊循环实测在位、orz pointer 同步且干净、门禁 Exit 0、281 + orz 全量绿 |
| B | 实现质量 | **PASS with notes，P1×1**——门禁 subprocess 默认编码崩溃风险（修复，见 R1） |
| C | 治理登记与数字核验 | **有缺口（均登记面）**——数字 14/15 精确吻合；P2×3 登记缺口 + P3×4（C4/C5 属本表 R4/R5） |

## 2. 复审发现与处置总表

| # | 级别 | 发现 | 处置 |
|---|---|---|---|
| R1 | **P1** | `_rust_journal_conformance_errors` 两处 `subprocess.run(text=True)` 用本地 ANSI 编码解码 CLI 输出：期刊 payload 非 ASCII 内容进入判官错误消息（实测 `"非法状态值中文" is not one of ["completed"]`）时，非 UTF-8 机器抛 `UnicodeDecodeError`（ValueError，未被 `except (OSError, TimeoutExpired)` 捕获）→ 门禁崩溃；本机 preferred=utf-8 掩盖 | **修复**：两处（CLI 调用 + cargo 兜底构建）改 `encoding="utf-8", errors="replace"` 并注记与 orz 对拍同源的编码陷阱；修复后中文错误消息经门禁助手**无损返回**（实测，6 错误逐字含原文） |
| R2 | P2 | 深挖文档 §4 裁决状态表仍标「D-1/D-2/D-3 待裁决 / S3 待放行」 | **采纳**：状态表四行就地更新为「已裁 = α / 已放行并实施」，头部加复审处理勘误注 |
| R3 | P2 | BACKLOG 00 节头部开放项列表仍列 00；BACKLOG `:71` 与 TODO S2 父项仍 `[ ]`（子项全勾） | **采纳**：00 头部摘除并注明闭合；S2 父项两处补勾（全部子批 2026-09-06 闭合、31→30 族） |
| R4 | P2 | 翻转实施审计登记数字精度：「两个硬编码 dict 共 225 行」实为 **218 行**（`git show` 实测被替换区间 49–266）；「篡改 = 9 错误」为审计时点特定篡改（v0.2 plain-run 首摘要翻转）计数，18 期刊同法实测 3–70 错不等、非跨 fixture 常数 | **采纳**：两处就地更正/补复现条件注；负测定性结论（篡改必拒 + 原刊必过）由 CLI 集成测试永久钉住、与计数无关 |
| R5 | P3 | S2d 收口审计 §2.3/§2.4 残留「门禁只校 12 v0.2 期刊」旧口径（勘误注已声明但单节阅读不自洽） | **采纳**：§2.3/§2.4 就地更正为「双轨 18 期刊」并加翻转后状态注；S3 裁决点标注已定案 |
| R6 | P3 | 深挖文档行号为翻转前快照（模块 3,627→3,464 行、两 dict 定义消失、门禁循环 `:2364→:2422`/`:2404→:2462` 等） | **采纳**：并入 R2 头部勘误注，逐项列出实测后行号 |
| R7 | P3 | 「契约 §8 变更流程」节号笔误（实为 §9，§8 = Rust 镜像纪律）：深挖文档 2 处 + batch-1 审计 §6 + S2d 收口审计 §5 | **采纳**：全部就地更正为 §9 并加笔误注 |
| R8 | P3 | `tool_probe.rs` WORK_TOOLS doc 称 Python 模块为 "the Python verifier"（翻转后为 reference） | **采纳**：改「the Python reference's … (frozen reference since the Task D S3 flip, 2026-09-06)」；orz `05f29fdc` |
| R9 | P3 | CLI `--repo-root` 重复（后值生效）与其余旗标拒绝行为未入文档 | **采纳**：journal-conformance.rs usage doc 补记；orz `05f29fdc` |
| R10 | P3 | registry JSON 信任边界：派生代码不拒绝 `schema` 字段绝对路径/`..` 越界 | **登记不改**：registry 为仓内受门禁同步守卫的受控文件、非模型可写面；变为外部可注入面时须先加 containment 校验（翻转实施审计 §3.5） |
| R11 | P3 | CLI 正测仅 2 fixture（双轨各一） | **登记不改**：CLI 壳薄，18 期刊库级全量由 `fixture_journal_conformance` 承担、两层互补（翻转实施审计 §3.6） |
| R12 | P3 | 索引头部缺本批版本摘要（本批实际修改了 IMPL-PYTHON-REFERENCE 条目） | **采纳**：头部补 v2.60 摘要行 |
| R13 | P3 | `generate_run_event_fixtures.py` 与 orz `conformance.rs` 对 Python 模块的注释提及 | **登记不改**：提及语义仍真（模块保留为 reference/对齐约定出处），provenance 性质 |

## 3. 复审处理批验证

| 项 | 结果 |
|---|---|
| R1 修复实测：中文 payload 非法期刊经门禁助手 | 6 错误、消息含 `"非法状态值中文"` 逐字无损、无崩溃 |
| orz `05f29fdc`：`cargo check -p orz-loop` 零警告 + `journal_conformance_cli` 4 passed + fmt 净 | ✅ |
| Python 三套件（281）/ orz-assurance 全量（204+9+4+9+1）不受本批影响（仅门禁脚本 + 注释变更） | ✅（门禁终验覆核） |
| manifest 重算 | 见 §4（orz 注释变更 → 内容哈希变化 → 重算必需） |
| 门禁 | **Exit 0**（见 §4） |

## 4. 变更清单与提交

- orz 子模块：`05f29fdc`（R8/R9，2 文件全注释，+8/−6）。
- 父仓库（本提交）：R1 门禁编码修复（`scripts/check_repository.py`）+
  R2/R6/R7 深挖文档勘误 + R3 BACKLOG/TODO 勾选收口 + R4 翻转审计勘误 +
  R5 S2d 收口审计口径更正 + R7 batch-1 审计更正 + R12 索引 v2.60 +
  本审计文档；manifest 重算（orz 注释变更后内容哈希刷新，条目数 1440 不变）。
- 门禁终验：`python scripts/check_repository.py` → **Exit 0**。
