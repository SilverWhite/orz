# ORZ-PS1-BOM-001 — PowerShell 5.1 无 BOM 读 `.ps1` 按 ANSI：含中文脚本解析失败（事故登记）

- **状态**：已归因 / 处置约定已固化（2026-09-17，摩擦处理狗粮 run `RUN-CLI-6aac0af5` 期间发现；登记号 **FR-N04**，由 orz 内 agent 自主发现并登记于处理批报告 §2）
- **分类**：`harness_environment`（脚本工具链编码）
- **现象**：Windows PowerShell 5.1 对无 BOM 的 `.ps1` 按 ANSI（本机 GBK）解码——含中文注释/字符串的脚本解析失败，报 `字符串缺少终止符`（本批 `scripts/dogfood_launch.ps1` 首版即踩；加 UTF-8 BOM 后正负例自测通过）。
- **根因**：PowerShell 5.1 无 BOM 时不做 UTF-8 猜测（PowerShell 7+/pwsh 默认 UTF-8，无此问题）；编辑器写出的 UTF-8 无 BOM 文件与运行时解码假设错位，中文字节序列在 ANSI 解码下破坏字符串/注释定界。
- **处置约定**：`.ps1` **带 UTF-8 BOM**（兼容 5.1 与 7+），或脚本保持纯 ASCII；脚本批验收把「能被 5.1 解析」列为检查项。
- **案例化理由**：见 [`案例候选`](../cases/harness_environment/ORZ-PS1-BOM-001-ps1-bom-encoding.md)。
