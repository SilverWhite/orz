# ADR-0010 分卷 06：§6 Windows 设计分离

> 本卷为 [`ADR-0010：ORZ 融合运行时、同构 Agent 与设计权威重整`](ADR-0010-fusion-runtime-and-agent-architecture.md)（AUTH-ADR-0010，ORZ 当前自然语言设计的唯一权威基线）的物理分卷 06（§6 Windows 设计分离）；规范正文以所指分卷章节为准，分卷总目录见主文件。分卷为物理拆分、**语义零增删零改写**——本行以下正文与分卷前原文逐字一致；卷题与本声明为分卷批新增（分卷批：0ca，2026-09-28）。

## 6. Windows 设计分离

1. `存档/architecture/runtime-spikes/WINDOWS_RUNTIME_CONTRACT_v0.1.md` 第 1–5 节继续只描述早期 process runtime spike，不再承载
   产品级 Windows beta、发行、案例治理或兼容性声明。
2. Windows-first 产品定位、环境矩阵、兼容性边界、安装/升级体验和 beta 要求进入独立的
   `architecture/WINDOWS_PLATFORM_COMPATIBILITY_DESIGN_v0.1.md`。
3. Windows 证据分为：追加式事故台账、经审计的精选案例、自动回归/人工复核、beta 环境证据。
4. 候选事故不得因写入设计文档就晋级为案例；必须复核 provenance、根因边界、分类、脱敏和回归。
5. `windows_native`、`cross_platform_agent`、`harness_environment` 分开统计；任务跑分不能替代
   Windows 工程成熟度。
6. 在上述制品建立前，状态只能写 planned/not started 或 maintainer-observed，不得写“案例闭环完成”。

Windows beta 的观察面至少覆盖：Job Object/ConPTY/取消与输出 drain；PowerShell 5.1/7、CMD、Git Bash、
MSYS 参数与转义；盘符、UNC、长路径、Unicode、CRLF、链接与文件占用；Credential Manager、
`GROK_HOME` 和写点分层；Git/MSVC/Python/Node/Rust/.NET 工具链；ACL、AppContainer、网络隔离；
Windows 10/11、企业策略、EDR、Docker Desktop/WSL；安装、升级、卸载和诊断体验。

事故晋级案例至少需要稳定 case ID、环境与 binary/commit provenance、症状、预期、复现、原始证据、
根因或明确失败边界、分类、修复归属、剩余限制、回归入口、复核结果和脱敏状态。单一维护者机器通过
只能写 maintainer-observed；正式兼容性声明必须同时提供已验证环境、已知限制和案例/测试入口。

