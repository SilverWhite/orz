# 精选案例库

本目录接收从事故台账晋级的脱敏案例，按 `windows_native`、`cross_platform_agent`、`harness_environment` 分类。案例用于自动回归或人工复核，晋级标准和最小字段以 ADR-0010 的 Windows 适配章节为准。

现有分类：

- `windows/` — Windows 原生平台案例（`ORZ-WIN-PROC-001/002/003`、`ORZ-WIN-PS-001/002`、
  `ORZ-WIN-SBX-001/002/003/004`、`ORZ-WIN-ACL-001`、`ORZ-WIN-CTYPES-001`、
  `ORZ-WIN-PIP-001` 候选；SBX/ACL/CTYPES 系 2026-09-01/09-02 S4 加固与三臂
  排障晋级，SBX-004/PIP-001 系 2026-09-02 ⑥ batch-2 high-nist 主载晋级）。
- `harness_environment/` — 构建/评测环境案例（`ORZ-BUILD-MOUNT-001` 容器构建挂载契约候选、
  `ORZ-TOOL-BINARY-COMPAT-001` 打包工具二进制兼容候选，2026-08-17 首批；
  `ORZ-PLATFORM-TARGET-001` 平台目标覆盖候选、`ORZ-VERDICT-EPOCH-001` 结论代际纪律候选，
  2026-09-13 第二批）。

归因纪律（2026-08-17 用户裁决）：命令或操作出现错误、或结果与已知事实明显矛盾时，
先排查环境与机械因素（二进制/运行时兼容、工具包装是否吞错误、路径/作用域解析、
沙箱/权限/ignore 语义、构建打包来源），再归因模型或命令纪律；不得仅凭工具退出码
断言「工具行为正确」。详见 `harness_environment/ORZ-TOOL-BINARY-COMPAT-001-*`。

同族追加（2026-09-13）：①`ORZ-TOOL-BINARY-COMPAT-001` 补入「容器缺件（protoc / ripgrep /
python3）伪装成构建或测试失败」的验证记录；②新增一条同族纪律——**数据正确不等于结论当前
有效**（`ORZ-VERDICT-EPOCH-001`：证据时点 + 载体版本 + 依据裁决 ID/日期 + 先回查后判断）。
两条纪律都不新增阻断门，只作为人工 / 批次核对入口。
