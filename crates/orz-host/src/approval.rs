//! Interactive approval prompter (stub — intentionally unimplemented).
//! Bridges orz-workspace::permission::Manager decisions to ACP ApprovalRequest channel.
//! IP6 hard-gate invariants enforced at the permission decision point.
//! Goose principle: SecurityFinding ≠ PermissionDecision (separation enforced).
//!
//! 状态（083 审查裁决②，2026-09-26；0bv 文档落字批落字 2026-09-27；姿态勘误
//! 随 ADR-0010 §14.90 条 3，0do/344 批 2026-10-23）：人工审批面为
//! **未来可选扩展、当前未实现**——权限桥缺省姿态双轨：Windows 交互面
//! （gateway/hub 任一在位）confirm-first（见 `default_initial_yolo`），
//! headless 与非 Windows 维持 yolo 自动放行（用户令 2026-09-26 自此收窄
//! 至该两车道）；本模块保持空壳、不接线、无排期承诺；`PermitSource` 枚举
//! 已为将来恢复审批保留观测位。本文件不是待办项，不要据「TODO」推断审批
//! 功能在途。

// TODO(候选扩展，未排期): Implement approval prompter — 仅在用户显式裁决恢复审批腿时启动设计。
