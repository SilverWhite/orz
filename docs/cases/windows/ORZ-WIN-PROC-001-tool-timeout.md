# ORZ-WIN-PROC-001 — tool_timeout 进程树终结（案例候选）

- **状态**：`candidate`（ADR-0010 §11.7 首批裁决晋级候选；未宣称产品级闭环）
- **晋级裁决**：ADR-0010 §11.7 —— `GROK_WINDOWS_CHILD_TREE_PROBE_2026-07-23.md` 的
  `tool_timeout` 场景具有明确 fixture、result/verification Schema、baseline/candidate 对照、
  digest 和零残留检查，晋级为精选案例候选。
- **来源证据**：`docs/GROK_WINDOWS_CHILD_TREE_PROBE_2026-07-23.md`
- **能力**：工具超时（`tool_timeout`）时 Grok/orz 进程树的确定性终结；三个角色
  （owner/provider/capture）的 stdout/stderr ready marker 可回收，teardown 零残留。
- **观察记录**（探针原文）：
  - `0.2.111` PASS：trial `child-tree-0.2.111-tool-timeout-v3`
    - run digest `289c61f3447115fafe0d3f06a8d3608391693a68b3164746c17e956814eafbaf`
    - verifier digest `71bedab122be31b7774f953f0ba2e1bce0c3adb50c1c51490c280c132298055b`
  - `0.2.106` FAIL（基线）：Grok 返回终态后仍未退出
    - failure `106e125440c813362800f1b380a3660943572bade35ce6db8335e6599424d9c1`
- **回归入口**：`scripts/child_tree_fixture.py`（三角色原子写入
  PID/PPID、nonce、脚本 digest、启动时间）
- **边界**：Schema PASS 与 verifier PASS 只证明固定 fixture 的机械一致性，不证明
  通用进程安全，更不证明科学正确性（探针文档 §安全边界）。
