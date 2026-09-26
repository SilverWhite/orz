//! Interactive approval prompter (stub — intentionally unimplemented).
//! Bridges orz-workspace::permission::Manager decisions to ACP ApprovalRequest channel.
//! IP6 hard-gate invariants enforced at the permission decision point.
//! Goose principle: SecurityFinding ≠ PermissionDecision (separation enforced).
//!
//! 状态（083 审查裁决②，2026-09-26；0bv 文档落字批落字 2026-09-27）：人工审批面为
//! **未来可选扩展、当前未实现**——权限桥默认 yolo 自动放行，本模块保持空壳、不接线、
//! 无排期承诺；`PermitSource` 枚举已为将来恢复审批保留观测位。本文件不是待办项，
//! 不要据「TODO」推断审批功能在途。

// TODO(候选扩展，未排期): Implement approval prompter — 仅在用户显式裁决恢复审批腿时启动设计。
