# ORZ-BUILD-MOUNT-001 — 容器构建挂载契约缺失导致 orz-assurance 编译失败（事故登记）

- **状态**：已归因 / 已处置（2026-08-17；预防措施已落地）
- **分类**：`harness_environment`（构建容器挂载）
- **现象**：按文档命令（`-v D:/CLI/orz:/orz -w /orz ...`）重建 Linux musl 二进制时，
  编译 orz-assurance 失败：
  `error: couldn't read /orz/crates/orz-assurance/../../../runtime/candidate-prefilter-config-v0.1.json`
  （`source-quality-seed-lists-v0.1.json` 同型错误）——失败发生在依赖下载/编译
  约 20 分钟后，浪费一整个构建轮次。
- **根因**：orz-assurance 两处
  `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../runtime/*.json"))`
  相对 `CARGO_MANIFEST_DIR` 解析到 `<父仓库>/runtime/`；文档挂载命令只挂载
  `D:/CLI/orz`（内嵌子模块），容器内 `../../../runtime` 解析到不存在的 `/runtime`。
  `include_str!` 在编译期报错，且报错信息不含挂载要求。
- **复现条件**：`MSYS_NO_PATHCONV=1 docker run --rm -v D:/CLI/orz:/orz
  -v D:/tb-eval/cargo-config.toml:/root/.cargo/config.toml
  -v D:/tb-eval/orz-target:/target -v D:/tb-eval/orz-linux:/out -w /orz
  rust:1.97-slim bash /build.sh`（`build_orz_aliyun.sh`）。
- **证据**：2026-08-17 GAP-ACAF-HARNESS-PASSTHROUGH 重建窗口；构建输出含
  `couldn't read .../runtime/candidate-prefilter-config-v0.1.json`；用户确认此前
  至少出现过一次同类失败（2026-08-17 早期重建窗口），本次为第二次复现。
- **正确挂载**：父仓库挂载为 `/orz`、工作目录 `/orz/orz`：
  `MSYS_NO_PATHCONV=1 docker run --rm -v D:/CLI:/orz -v ... -w /orz/orz ...`
  （`/orz/orz/crates/orz-assurance/../../../runtime` = `/orz/runtime`）。
- **处置**（2026-08-17）：
  1. 修正两份评测操作文档的构建命令（`TERMINAL_BENCH_2_EVAL` §2 /
     `TB_HARD_BATCH5_VALIDATION` 步骤 1）。
  2. `D:\tb-eval\build_orz_aliyun.sh` 增加前置挂载守卫（秒级失败 + 明确错误信息）。
  3. orz 源码两处 `include_str!` 现场补充容器挂载契约注释
     （`candidate_prefilter.rs` / `source_weighting.rs` 的 `embedded()`）。
  4. 晋级案例候选：`docs/cases/harness_environment/ORZ-BUILD-MOUNT-001-container-mount.md`。
- **避免复发**：构建脚本前置守卫 + 案例回归入口（见案例文件）；两份文档命令已修正；
  内嵌仓库源码注释已写明挂载契约。
