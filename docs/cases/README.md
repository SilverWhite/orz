# 精选案例库

本目录接收从事故台账晋级的脱敏案例，按 `windows_native`、`cross_platform_agent`、`harness_environment` 分类。案例用于自动回归或人工复核，晋级标准和最小字段以 ADR-0010 的 Windows 适配章节为准。

现有分类：

- `windows/` — Windows 原生平台案例（`ORZ-WIN-PROC-001/002/003`、`ORZ-WIN-PS-001/002`、
  `ORZ-WIN-SBX-001/002/003/004`、`ORZ-WIN-ACL-001`、`ORZ-WIN-CTYPES-001`、
  `ORZ-WIN-PIP-001` 候选；SBX/ACL/CTYPES 系 2026-09-01/09-02 S4 加固与三臂
  排障晋级，SBX-004/PIP-001 系 2026-09-02 ⑥ batch-2 high-nist 主载晋级；
  2026-09-18 第四批：`ORZ-WIN-TEMP83-001` CI runner TEMP 8.3 短路径断言候选，
  来源同 0aq RS-01/RS-02 处置）。
- `harness_environment/` — 构建/评测环境案例（`ORZ-BUILD-MOUNT-001` 容器构建挂载契约候选、
  `ORZ-TOOL-BINARY-COMPAT-001` 打包工具二进制兼容候选，2026-08-17 首批；
  `ORZ-PLATFORM-TARGET-001` 平台目标覆盖候选、`ORZ-VERDICT-EPOCH-001` 结论代际纪律候选，
  2026-09-13 第二批；`ORZ-ENV-POLLUTION-001` 环境污染假红候选、`ORZ-PS1-BOM-001`
  PowerShell BOM 编码候选，2026-09-17 第三批，来源＝摩擦处理狗粮 run `RUN-CLI-6aac0af5`
  新摩擦 FR-N02/FR-N04；2026-09-18 第四批：`ORZ-CI-BLINDOUT-001` 红灯掩盖经济学、
  `ORZ-GATE-ASYM-001` 守门者平台不对称伪绿、`ORZ-DEV-TUNED-BOUND-001` 开发机调参上界与
  缓存掩盖、`ORZ-GLOB-TIEBREAK-001` 非确定 top-K，来源＝0aq RS-01/RS-02 处置
  〔CI 断流 16 天修复，事故台账 `incidents/ORZ-CI-BLINDOUT-001.md`〕）；2026-09-19 第五批：
  `ORZ-RUN-SEPARATION-001` 跨 run 产物归属隔离候选，来源＝0ar S1 狗粮轮摩擦 F2
  （`RUN-CLI-6aad91f0` 误杀后 WIP 交接现场；用户裁决「每不同 run 的产物明确归不同 run，
  关键是不能混为一谈」，处置终点＝案例登记不立工程项）。

归因纪律（2026-08-17 用户裁决）：命令或操作出现错误、或结果与已知事实明显矛盾时，
先排查环境与机械因素（二进制/运行时兼容、工具包装是否吞错误、路径/作用域解析、
沙箱/权限/ignore 语义、构建打包来源），再归因模型或命令纪律；不得仅凭工具退出码
断言「工具行为正确」。详见 `harness_environment/ORZ-TOOL-BINARY-COMPAT-001-*`。

同族追加（2026-09-13）：①`ORZ-TOOL-BINARY-COMPAT-001` 补入「容器缺件（protoc / ripgrep /
python3）伪装成构建或测试失败」的验证记录；②新增一条同族纪律——**数据正确不等于结论当前
有效**（`ORZ-VERDICT-EPOCH-001`：证据时点 + 载体版本 + 依据裁决 ID/日期 + 先回查后判断）。
两条纪律都不新增阻断门，只作为人工 / 批次核对入口。

同族追加（2026-09-17）：③**测试读数先核 env**（`ORZ-ENV-POLLUTION-001`：测试进程继承
狗粮启动 env ⇒ 批量假红语义可信；「红／绿」结论产出前先核 `ORZ_*`/`GROK_HOME` 预置，
读数口径注明清 env＋串行）；④**`.ps1` 带 UTF-8 BOM 或纯 ASCII**（`ORZ-PS1-BOM-001`：
PowerShell 5.1 无 BOM 按 ANSI 解码，「字符串缺少终止符」先查编码）。两条仍不新增阻断门。

同族追加（2026-09-18）：⑤**干净环境缺件先查构建前置与构建缓存**（`ORZ-DEV-TUNED-BOUND-001`
承接 `ORZ-TOOL-BINARY-COMPAT-001` 同族：target 缓存吞 build script 重跑 ⇒ protoc 缺件本机
不可见、CI 干净 checkout 即裸奔；CI 步骤显式安装并钉版 protoc 35.1）；⑥**CI 状态入账本口径**
（`ORZ-CI-BLINDOUT-001`：本地 error_count 门禁与远端 CI 是两套真相，红灯期发布须显式标记）。
两条仍不新增阻断门。

同族追加（2026-09-19）：⑦**读数先核 run 归属**（`ORZ-RUN-SEPARATION-001`：产物落盘即绑定
run id/session8/时间戳、交接声明来源、断链 run 产物显式标记孤儿态；与 ③ 同属「读数结论
产出前先核来源」族）。不新增阻断门。
