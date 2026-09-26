# ORZ-PS1-BULK-REWRITE-001 — PowerShell 5.1 管道批量改写把 UTF-8 源码写花（事故登记）

- **状态**：已归因 / 已恢复（2026-09-27，0bv 结转轮 run `RUN-CLI-6ab7f32f` 期间发生并同 run 恢复；登记于该轮报告 §3 F-1）
- **分类**：`harness_environment`（脚本工具链编码）
- **现象**：agent 以 PowerShell 5.1 `Get-Content -Raw` ＋ `Set-Content -Encoding UTF8` 做「锁中毒统一」批量替换，31 个 Rust 源文件被按 ANSI（本机 GBK）解码后重写——CJK 注释成 mojibake、UTF-8 BOM 被注入；`git diff --stat` 异常放大（单文件 +3757 行量级）暴露。
- **根因**：PS 5.1 对无 BOM 文件 `Get-Content` 按 ANSI 解码（与 [ORZ-PS1-BOM-001](ORZ-PS1-BOM-001.md) 同根），`Set-Content -Encoding UTF8` 再把**已解码损坏**的文本以 UTF-8 写回 ⇒ 双重破坏（mojibake 固化＋BOM 注入）。与 BOM-001 的失败形态不同：本例**不报语法错、编译可过**（注释损坏不挡编译、测试可全绿），属**静默 corruption**，唯一可靠检出信号是 diff 体量。
- **处置**：`git checkout --` 31 文件回 HEAD → 改用 **Python 显式 UTF-8 读写**重放替换（198 处）→ 逐文件 check。过程件（`.tmp-lock-sweep.py`／`.tmp-tests-*.log`）已清理。主会话独立复核：本轮全部 39 个改动文件 UTF-8 严格解码零 U+FFFD（2026-09-27）。
- **教训（处置约定）**：批量改码**禁用 PS 5.1 管道读写**；必须显式 UTF-8 读写（如 Python `open(..., encoding="utf-8")`）；替换批完成后必做 `git diff --stat` 体量自检（行数异常放大＝第一信号，先回退再归因）。
- **案例化理由**：见 [`案例候选`](../cases/harness_environment/ORZ-PS1-BULK-REWRITE-001-ps1-bulk-rewrite-mojibake.md)。
