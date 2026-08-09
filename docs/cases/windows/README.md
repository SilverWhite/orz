# Windows 精选案例库

本目录接收从事故台账晋级的脱敏案例。首批按 ADR-0010 §11.7 裁决：

- `ORZ-WIN-PROC-001` — tool_timeout 进程树终结（候选）
- `ORZ-WIN-PROC-002` — task_cancel 进程树终结（候选）
- `ORZ-WIN-PROC-003` — parent_exit 孤儿树终结（候选）

**状态口径**：三个条目均为 `candidate`（晋级自
`GROK_WINDOWS_CHILD_TREE_PROBE_2026-07-23.md` 的三场景，fixture/verifier/digest
齐全），但未宣称产品级闭环——探针是 fake-only fixture 证据，不构成通用进程安全
或科学正确性证明。

待晋级/不晋级清单（ADR-0010 §11.7）：
- Observer 子进程泄漏保留在事故路由（`docs/incidents/windows/`），待 raw artifact
  identity 与结构化 verification 补齐。
- Windows Native Sandbox raw TCP 残留为 open limitation（`WIN-LIM-001`），不晋级。
- Credential hardening 为 offline implementation evidence，不晋级为 Credential
  Manager 实机兼容案例，直至真实 credential-read 路径完成脱敏验证。
