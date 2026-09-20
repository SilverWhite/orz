# ORZ-DEV-LINKER-CRASH-001 — 并行 `rust-lld` 崩溃与本机 target 缓存污染（案例候选）

- **状态**：`candidate`（2026-09-20 独立审计批案例化）
- **晋级理由**：同一次读数中「默认并行编译/链接失败」与「串行全绿」形成一个可复现的对照实验，
  而失败形态（零长度产物、`rlib format`／`invalid metadata` 报错）**看起来像代码缺陷**——
  若不先归因环境，会把本机链接器崩溃写成代码问题、污染审查结论（本次差点如此）。
  新增/变更构建面时必然复发，纪律价值高。
- **来源证据**：2026-09-20 独立审计批（[`0AT_0AU_0AV_0AW_0AX_S1 审计与裁决`](../../audits/0AT_0AU_0AV_0AW_0AX_S1_INDEPENDENT_AUDIT_AND_ADJUDICATION_2026-09-20.md) §3）
  ——对象＝0at/0au/0av/0aw/0ax S1 批工作树（父仓 `10f93c75` 基线）。
- **机理**：
  ①**并行链接脆点**——本机 `cargo test -p orz-bin` 默认多作业链接时，`rust-lld` 以
  `exit code 0xc000001d`（NTSTATUS `STATUS_ILLEGAL_INSTRUCTION`）崩溃；
  ②**崩溃残留进缓存**——崩溃留下**零长度 `.rmeta`** 与半成品 rlib，`cargo` 的 fingerprint 仍视其为 fresh
  ⇒ 后续构建报 `crate <X> required to be available in rlib format, but was not found in this form`、
  `can't find crate for <X>`、`found invalid metadata files for crate test`——
  这些报错**指向依赖而非崩溃本身**，误导性强；
  ③**同树对照坐实环境归因**——同一工作树 `-j 1` 串行链接**全绿**，`cargo check --workspace --all-targets` 亦干净。
- **能力（纪律）**：
  ①**红先核环境**：出现 `rlib format`／`invalid metadata`／`can't find crate` 三类报错时，
  先查是否为链接器崩溃残留（扫描 `target/**/deps` 零长度产物、看是否有 `rust-lld` 崩溃行），再谈代码；
  ②**重载目标串行复现**：以 `-j 1` 作为本机规范复现口径，并同时记录默认并行的失败形态（两读数并列登记）；
  ③**清理即恢复**：`cargo clean` 属恢复手段，**清理前后各记一次读数**（本次：并行失败形态 → 清理 35.7 GiB → 并行仍失败 → `-j 1` 全绿）；
  ④**外置载体不受 build 缓存影响**：换装件（`D:\tb-eval\orz-windows` 等）哈希与审计档记录一致即证明清理无损交付面。
- **回归入口**：本机 `cargo test -p orz-bin`（默认 vs `-j 1` 对照）；`cargo check --workspace --all-targets` 作为「代码面是否真的坏」的判别面。
- **同族先例**：`ORZ-DEV-TUNED-BOUND-001`（开发机调参上界与缓存掩盖——本条补「缓存被崩溃污染」机理）、
  `ORZ-TOOL-BINARY-COMPAT-001`（工具链缺件伪装成构建失败）、`ORZ-ENV-POLLUTION-001`（读数前先核环境）。
- **验证记录**：2026-09-20 ①默认并行：`orz`／`orz-signer`／`orz-acaf-provision`／`acaf_e2e`／`stdio_e2e` 多目标编译失败；
  ②`cargo clean`（53,878 文件／35.7 GiB）后重编仍复现同形态；③`cargo test -p orz-bin -j 1`：main 12/0（＋12 ignored）、
  orz-acaf-provision 2/0、orz-signer 15/0、acaf_e2e 23/0、real_flag 2/0、stdio_e2e 1/0 —— 与过夜批登记读数逐项一致；
  ④`cargo check --workspace --all-targets` 干净（仅一条既有 `unused variable` warning）。
