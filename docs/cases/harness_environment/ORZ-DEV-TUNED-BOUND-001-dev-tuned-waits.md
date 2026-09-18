# ORZ-DEV-TUNED-BOUND-001 — 开发机调参上界与缓存掩盖：干净环境即裸奔（案例候选）

- **状态**：`candidate`（2026-09-18 RS-02 处置收官当日案例化）
- **晋级理由**：同一次 CI 接入中两度命中同一族机理——「开发机上成立的边界值/环境前置，到干净慢环境全部失效」，且事先不可见（缓存与时间余量把问题藏住）。新增 CI 轨道时必然复发，读数纪律价值高。
- **来源证据**：[`docs/incidents/ORZ-CI-BLINDOUT-001.md`](../../incidents/ORZ-CI-BLINDOUT-001.md) 第 3/5 层；run 35326045846（orz-tools-api build.rs `protoc` 缺件）与 run 35334716461（`codex_app.rs` approval 测试 `timed out waiting for turn/completed`）。
- **机理两形态**：
  ①**缓存掩盖缺件**——`cargo` 的 target 缓存吞掉 build script 重跑：开发机早年一次成功运行把 `orz-tools-api`（需 `protoc`）的产物留在 `target/`，此后本机无 protoc 也一路绿；CI 全新 checkout 无缓存可吃，`cargo test -p orz-loop --lib` 首步即崩。
  ②**时间上界开发机调参**——测试等待硬编码 5s/8s（开发机手调值），2 核 CI runner 串行高载下等不到 `turn/completed`。
- **能力（纪律）**：
  ①**等待上界单源常量并按最慢目标环境取值**（`TURN_WAIT = 30s`，五处收编；真挂死仍有界）；
  ②**「本机能过」先问缓存**——构建/测试前置（protoc 类）在可信环境外是否可得？CI 步骤显式安装并钉版本（protoc 35.1 与 vendored 同源）；
  ③**载敏测试（时序/进程/负载）入 CI 必须串行＋放宽上界**，系统性分层归 RS-04。
- **回归入口**：rust-tests job 三步（CI 常驻即回归）；本地模拟＝清 `target` 后无 `PROTOC` 环境构建。
- **同族先例**：`ORZ-TOOL-BINARY-COMPAT-001`（「容器缺件伪装成构建/测试失败」——本条补 CI runner 形态＋缓存掩盖机理）、`ORZ-ENV-POLLUTION-001`（读数口径前置核验）。
- **验证记录**：2026-09-18 run 35338101577 rust-tests 八步全绿（安装步＋放宽后串行套件通过）；**同日第三实例**——codex_app EOF 测试固定 900ms 睡眠在 CI 等不到 journal 落盘收尾（run 35341286834 `valid: false`；同树 run 35342374965 绿坐真 flaky），子仓 `9a1c3b2c` 改轮询 run_finished 终态／TURN_WAIT 封顶后 run 35344882087 绿。
