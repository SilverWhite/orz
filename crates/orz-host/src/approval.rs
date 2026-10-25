//! Interactive approval prompter (stub — intentionally unimplemented).
//! Bridges orz-workspace::permission::Manager decisions to ACP ApprovalRequest channel.
//! IP6 hard-gate invariants enforced at the permission decision point.
//! Goose principle: SecurityFinding ≠ PermissionDecision (separation enforced).
//!
//! 状态（083 审查裁决②，2026-09-26；0bv 文档落字批落字 2026-09-27；姿态勘误
//! 随 ADR-0010 §14.91 条 1/2，0dp/348 批 2026-10-25）：人工审批面为
//! **未来可选扩展、当前未实现**——权限桥缺省姿态单一源：全部车道缺省
//! yolo 自动放行，`--yolo`/`--no-yolo` 旗／`ORZ_YOLO` env 为显式用户
//! 开关（`--no-yolo` 得 confirm-first；见 `default_initial_yolo`）；
//! 本模块保持空壳、不接线、无排期承诺；`PermitSource` 枚举
//! 已为将来恢复审批保留观测位。本文件不是待办项，不要据「TODO」推断审批
//! 功能在途。

// TODO(候选扩展，未排期): Implement approval prompter — 仅在用户显式裁决恢复审批腿时启动设计。
