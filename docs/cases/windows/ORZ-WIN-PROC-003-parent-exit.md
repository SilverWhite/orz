# ORZ-WIN-PROC-003 — parent_exit 孤儿树终结（案例候选）

- **状态**：`candidate`（ADR-0010 §11.7 首批裁决晋级候选；未宣称产品级闭环）
- **晋级裁决**：ADR-0010 §11.7 —— `GROK_WINDOWS_CHILD_TREE_PROBE_2026-07-23.md` 的
  `parent_exit` 场景（owner 被故意终止后 background tree 不残留）具有明确 fixture、
  result/verification Schema、baseline/candidate 对照、digest 和零残留检查，
  晋级为精选案例候选。
- **来源证据**：`docs/GROK_WINDOWS_CHILD_TREE_PROBE_2026-07-23.md`
- **能力**：外层进程退出（parent_exit）后，background 进程树不存活为孤儿；
  该场景只要求三角色已启动和 teardown 无残留，不把未 drain 的输出伪装成完整
  （owner 终止时故意不 drain capture）。
- **观察记录**（探针原文）：
  - `0.2.111` PASS：trial `child-tree-0.2.111-parent-exit-v1`
    - run digest `3e0b8ef697cdd81f2f51ffba1ade02d6a26d30cd8251a49277502bcd8c8704b3`
    - verifier digest `2cc08349694abfa823f3535aef1a55d8929b115657e94873c19385a1c86dba31`
  - `0.2.106` PASS：trial `child-tree-0.2.106-parent-exit-v1`
    - run digest `19461bbe38186420b5cd8303206a94a3ecba6c31bfb618683a39ca58b3c23186`
- **回归入口**：`regression/` 探针 fixture（同 ORZ-WIN-PROC-001/002）
- **边界**：同 ORZ-WIN-PROC-001 —— 固定 fixture 机械一致性证据；parent_exit 的
  capture 未 drain 是场景设计（故意终止 owner），不是完整输出证据。
