# ORZ-BUILD-MOUNT-001 — 容器构建挂载契约（案例候选）

- **状态**：`candidate`（2026-08-17 晋级候选；未宣称产品级闭环）
- **晋级裁决**：构建/评测环境类事故具备明确复现命令、根因、机械守卫与文档修正，
  晋级为精选案例候选（`harness_environment` 类首批）。
- **来源证据**：`docs/incidents/ORZ-BUILD-MOUNT-001.md`
- **能力**：orz（内嵌仓库）容器构建挂载契约的正确性防护——挂载错误时秒级失败并给出
  正确挂载方式，而非在依赖下载/编译约 20 分钟后于 `include_str!` 处失败。
- **回归入口**：
  - 前置守卫：`D:\tb-eval\build_orz_aliyun.sh` 顶部挂载检查
    （`/orz/runtime/candidate-prefilter-config-v0.1.json` 与
    `/orz/orz/Cargo.toml` 存在性；缺失即 exit 1 并指明挂载方式）。
  - 文档命令：`docs/TERMINAL_BENCH_2_EVAL_2026-08-08.md` §2 /
    `docs/TB_HARD_BATCH5_VALIDATION_2026-08-09.md` 步骤 1（父仓库 `/orz` +
    工作目录 `/orz/orz`）。
  - 源码注释契约：`orz/crates/orz-assurance/src/candidate_prefilter.rs` /
    `source_weighting.rs` 的 `embedded()`（ORZ-BUILD-MOUNT-001 引用）。
- **验证记录**：2026-08-17 修正挂载后重建成功（15m45s），
  orz / orz-signer / orz-acaf-provision 三件套输出正常。
- **边界**：守卫只覆盖本仓库的挂载形态（`D:/CLI:/orz` + `/orz/orz`）；其他挂载布局
  或 CI 环境需按同一契约另行配置；`include_str!` 契约仍以父仓库 `runtime/` 提交为前提
  （`orz` 单独 checkout 无法编译，属既有 repo-boundary 注记）。
