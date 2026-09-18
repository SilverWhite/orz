# ORZ-GLOB-TIEBREAK-001 — 非确定 top-K：稳定排序保 glob 枚举序（案例候选）

- **状态**：`candidate`（2026-09-18 RS-01 处置第三层收官当日案例化）
- **晋级理由**：检索/排序面的并列分截断是典型的「单机不可见非确定性」——开发机工作树碰巧把目标文件送进 top-K，干净 checkout 没有；且随语料增长必然恶化（09-18 文档对齐批与本审查文档都在加竞争者）。误读代价＝把环境差异误判为检索质量回归。
- **来源证据**：[`docs/incidents/ORZ-CI-BLINDOUT-001.md`](../../incidents/ORZ-CI-BLINDOUT-001.md) 第 2 层；run 35328029850 四 Python job `test_search_p3_action_authorization` 单点失败（`Should find instruction_gate.py`）。
- **机理**：`results.sort(key=score, reverse=True)` 是稳定排序——并列分保持**插入序**，而插入序来自 `Path.glob` 的文件系统枚举序（机器间/语料间不同）；`[:max_results]` 在并列带内截断 ⇒ top-K 随环境漂移。
- **能力（纪律）**：
  ①**排序面并列必显式 tie-break**——「分数降序＋路径升序」（`project_doc_index.py` 修法），禁依赖稳定排序隐式保序；
  ②**断言窗口抗语料漂移**——找源码用 `categories=["source_code"]` 限域检索，而非扩大 K 去追广谱排名；
  ③**非确定性排查特征**——「本机绿 CI 挂、无断言逻辑差异、失败点在集合包含类断言」时优先怀疑枚举序/并列截断。
- **回归入口**：`python -m unittest assurance.tests.test_retrieval_subagent_real`（26 测试）；CI assurance 步常驻。
- **同族先例**：`ORZ-VERDICT-EPOCH-001`（结论的时点性——本条补「结论的环境性」：同一输入不同机器不同序）。
- **验证记录**：2026-09-18 修法落地后本地 26/26；run 35338101577 assurance 步全绿。
