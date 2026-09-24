# ORZ-PS1-BOM-001 — PowerShell 5.1 无 BOM 读 `.ps1` 按 ANSI（案例候选）

- **状态**：`candidate`（2026-09-17 发现当日案例化候选）
- **晋级理由**：小而高频复发面——本仓 `scripts/` 与工作脚本以 PowerShell 为主要载体、注释惯例为中文；该缺陷表现为「语法错误」而非「编码错误」，归因绕远（首版 `dogfood_launch.ps1` 即踩）。约定一旦固化即零成本规避。
- **来源证据**：[`docs/incidents/ORZ-PS1-BOM-001.md`](../../incidents/ORZ-PS1-BOM-001.md)（登记号 FR-N04，处理批报告 §1.12/§2）
- **能力**：①**BOM 约定**——`.ps1` 带 UTF-8 BOM（PowerShell 5.1 与 7+ 双兼容）或保持纯 ASCII；②**症状识别**——「字符串缺少终止符」类语法错 + 文件含中文 ⇒ 先查 BOM 而非逐行找引号；③**验收项**——脚本批自测须在 Windows PowerShell 5.1（`powershell.exe`，非 `pwsh`）下实际解析通过。
- **回归入口**：
  - 反例：UTF-8 无 BOM + 中文注释的 `.ps1` 在 `powershell -File` 下解析失败；
  - 正例：同内容加 BOM 后 `powershell -File scripts/dogfood_launch.ps1 -TaskFile .tmp-friction-task.txt -DryRun` 通过（2026-09-17/09-18 两日实测，含主会话复核）。
- **同族先例**：`GAP-ENCODING-GATE`（orz 工具面 UTF-8/GB18030 解码门——同一「解码假设错位」家族在脚本工具链的镜像）。
- **验证记录**：2026-09-17 run `RUN-CLI-6aac0af5` 内首踩即修（BOM 后双例自测过）；2026-09-18 主会话复核复跑正负例通过。
- **同族追加（2026-09-24）**：**BOM 断言必须落字节面**——0bi 轮（run `RUN-CLI-6ab3dbe5`）① 的 `.ps1` 解析钉首版用
  `[IO.File]::ReadAllText` 后取首字符判 BOM，而该 API **读回时已剥掉 BOM** ⇒ 断言结构性假红；改为
  `ReadAllBytes` 判前三字节 `EF BB BF` ＋ `[scriptblock]::Create` 实际解析后转绿。教训＝BOM 这类**字节面事实**
  不能用字符面 API 核验（读文本先剥 BOM 是库语义，不是文件事实）。
