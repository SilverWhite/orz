# ORZ-WIN-PROC-002 — task_cancel 进程树终结（案例候选）

- **状态**：`candidate`（ADR-0010 §11.7 首批裁决晋级候选；未宣称产品级闭环）
- **晋级裁决**：ADR-0010 §11.7 —— `GROK_WINDOWS_CHILD_TREE_PROBE_2026-07-23.md` 的
  `task_cancel` 场景（background tree 运行中取消）具有明确 fixture、result/verification
  Schema、baseline/candidate 对照、digest 和零残留检查，晋级为精选案例候选。
- **来源证据**：`docs/GROK_WINDOWS_CHILD_TREE_PROBE_2026-07-23.md`
- **能力**：`background=true, timeout=0` 启动的 task 在取消时进程树确定性终结；
  触发后观察到 0 个残留进程。
- **观察记录**（探针原文）：
  - `0.2.111` PASS：trial `child-tree-0.2.111-task-cancel-v8`
    - run digest `ea230cf8985813bee09008f2c50b182fa66df9208917349a32c17c87e9cb5ecd`
    - verifier digest `33b43f9551d4b630dbe19648bd4d10db182c307168466fc5038aa4eacae8ea10`
  - `0.2.106` PASS：trial `child-tree-0.2.106-task-cancel-v1`
    - run digest `2bf178963db8cf6bdcd6eb20743e9af586f673c7b5e350627a12c4aae1108ce1`
- **回归入口**：`scripts/child_tree_fixture.py`（同 ORZ-WIN-PROC-001）
- **边界**：同 ORZ-WIN-PROC-001 —— 固定 fixture 机械一致性证据，不构成通用进程安全结论。
