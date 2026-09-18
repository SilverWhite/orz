# ORZ-WIN-TEMP83-001 — CI runner TEMP 为 8.3 短路径：路径断言须同规范化形态（案例候选）

- **状态**：`candidate`（2026-09-18 RS-01 处置第四层收官当日案例化）
- **晋级理由**：Windows 原生环境的隐藏形态差——GitHub windows runner 的 `TEMP` 是 8.3 短路径（`C:\Users\RUNNER~1\...`），而生产代码 canonicalize（dunce）成长路径（`C:\Users\runneradmin\...`）；开发机 TEMP 本就是长形态，缺陷在测试全生命周期内被掩盖。路径相等断言是高频写法，复发面大。
- **来源证据**：[`docs/incidents/ORZ-CI-BLINDOUT-001.md`](../../incidents/ORZ-CI-BLINDOUT-001.md) 第 4 层；run 35334716461 `orz-host` 串行步 `session::tests::bootstrap_wires_snapshot_store_and_permit_signer`（`left: ...runneradmin... right: ...RUNNER~1...`）。
- **机理**：测试用 `std::env::temp_dir()`（继承 `TEMP` env＝短路径形态）构造基准路径，被测 bootstrap 用 dunce canonicalize 存储 worktree（长路径形态）；两个**同一目录的不同字符串表示**做 `assert_eq`。
- **能力（纪律）**：
  ①**路径比较须同规范化形态**（canonical-to-canonical）——测试基准路径先过与生产侧同一把尺子（`dunce::canonicalize`，幂等、开发机不受扰）；
  ②**`TEMP`/`TMP` 一切派生路径在 CI runner 上默认短形态**——凡断言涉及「env 派生路径 vs 规范化路径」即按本条处理；
  ③**开发机绿对 CI 无证据力（Windows 形态面）**——8.3 短名是否生成取决于卷配置与名字长度，不可假设两端同形。
- **回归入口**：`cargo test -p orz-host --lib -- session::tests::bootstrap_wires -- --test-threads=1`；CI rust-tests 串行步常驻。
- **同族先例**：`ORZ-WIN-ACL-001`/`ORZ-WIN-SBX-*`（Windows 原生形态差家族——本条补「路径表示形态」维）；`ORZ-DEV-TUNED-BOUND-001`（同为干净 CI 环境暴露族）。
- **验证记录**：2026-09-18 子仓 `546f9ec5` 修复（本地 1/0 过）；run 35338101577 orz-host 串行步全绿。
