# ADR-0002：延期云端 runtime，保持 Windows 本地执行

状态：accepted / deferred；2026-07-21。

## 决策

当前阶段不构建、部署或运维 cloud brain、公共 relay、Cloudflare Worker、远程命令队列、云端记忆服务或
云端定时任务。正式执行面保持在 Windows 本机，由 Grok Build 与窄 LIF wrapper 组成。

用户确认现有移动端设计已经满足当前远程使用需要；该移动端实现不属于本仓库，本 ADR 不对其安全性或
完整性作额外证明。Project D 的云端代码只保留为历史模式来源，不成为待迁移 backlog。

## 原因

- 云端执行面会引入持续运维、鉴权、密钥轮换、日志留存、成本和可用性责任；当前没有必要承担这些责任。
- 本阶段的关键风险是本地模型/工具过程是否可观察、可停止、可追溯，而不是跨网络调度。
- Windows-first 能减少环境迁移和额外存储投入，也更容易让凭据与原始研究材料留在本机。

## 保留的未来接缝

延期不等于把 Grok 或 LIF 协议绑定到单机 UI。以下接缝可以保留，但本阶段不实现云端服务：

1. headless/ACP 入口与稳定 session/run ID；
2. 可导出的 manifest、event journal、trace 与 artifact digest；
3. 明确的本地 supervisor 边界；
4. 将来可加的、经过鉴权且只传递必要数据的 relay 接口。

## 重新评估触发条件

只有在本地加移动端仍无法满足需求，并且出现无人值守执行、多设备协调或明确的云端可用性要求，同时已
准备运维预算、威胁模型与数据留存策略时，才重新打开云端设计。重新打开应创建新的 ADR，不回写本决策来
伪装历史连续性。

本决策不修改 [`protocol/PROTOCOL_DRAFT_v0.1.md`](../protocol/PROTOCOL_DRAFT_v0.1.md) 的语义。
