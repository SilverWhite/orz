# ORZ-PS1-BULK-REWRITE-001 — PowerShell 5.1 管道批量改写把 UTF-8 源码写花（案例候选）

- **状态**：`candidate`（2026-09-27 发现当日案例化候选）
- **晋级理由**：批量重构／统一替换是高频工程动作；本例损坏形态**静默**（注释损坏不挡编译、测试可全绿、panic 文案语义可信），不设防则可穿过整条实现＋测试链直接入库。约定一旦固化即零成本规避，检出手段（diff 体量自检）一行命令。
- **来源证据**：[`docs/incidents/ORZ-PS1-BULK-REWRITE-001.md`](../../incidents/ORZ-PS1-BULK-REWRITE-001.md)（0bv 结转轮报告 §3 F-1，run `RUN-CLI-6ab7f32f`）
- **能力**：
  ①**工具约定**——批量读写源码用显式 UTF-8 工具（Python `open(..., encoding="utf-8")` 等）；禁用 PS 5.1 `Get-Content`／`Set-Content` 管道做仓库文本改写（`-Encoding UTF8` 只管写侧，救不回读侧已解码损坏的字节）；
  ②**症状识别**——`git diff --stat` 行数异常放大（如纯替换批出 +3757 行）＝编码损坏第一信号，先回退再归因；
  ③**恢复套路**——`git checkout --` 受损文件回 HEAD → 显式 UTF-8 重放替换 → 逐文件 check；
  ④**验收项**——改动文件 UTF-8 严格解码零 U+FFFD（`errors="strict"` 打不开即红）。
- **回归入口**：
  - 反例：PS 5.1 `Get-Content -Raw`＋`Set-Content -Encoding UTF8` 处理含 CJK 注释的无 BOM 文件 ⇒ 注释 mojibake＋BOM 注入（`git diff` 体量放大）；
  - 正例：同替换以 Python 显式 UTF-8 读写 ⇒ diff 仅含目标行、解码严格通过（2026-09-27 run `RUN-CLI-6ab7f32f` 同 run 实测，主会话复核 39 文件零替换字符）。
- **同族先例**：`ORZ-PS1-BOM-001`（同根：PS 5.1 ANSI 缺省读；彼为脚本解析报错、此为源码静默写花）、`GAP-ENCODING-GATE`／`GAP-ENCODING-LOSSY-REFINEMENT`（「解码假设错位」家族在 orz 工具面的镜像）、`ORZ-ENV-POLLUTION-001`（产出结论前先核来源的归因纪律族——本例「diff 体量自检」同属产出前先核）。
- **验证记录**：2026-09-27 run `RUN-CLI-6ab7f32f` 内首踩即恢复（checkout＋Python 重放）；同日主会话独立复核全部改动文件 UTF-8 严格解码通过。
