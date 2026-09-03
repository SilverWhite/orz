# TER T1.12 W-F11 环境快照 section=env 实施审计（2026-09-04）

> 范围：TODO2 M1 步 T1.12——W-F11：机械层代码工具环境快照（工具/语言/
> 包/版本存在性与版本、关键输入在场判定；连通性判定 ≤1–2s/项）落黑板
> `section=env`（PULL 白名单区，≤5s）；验收 = 快照 ≤5s；env 分区 PULL
> 渲染 + 越权边界。
> 上级：[TODO2.md](../../TODO2.md) / 设计稿
> [TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md](../TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md)
> §3.4（W-F11）/ §10 S1-9；T0.2 审计 §5.2/§5.3（env 内容与越权边界：
> 白名单键集合实现侧登记、越权键渲染层拒绝、live-only 不入归档）。

## 1. 目标与验收

- `blackboard_read section=env` 提供 PULL 白名单分区：读取时从 host 现算
  机械层快照（工具/语言/包/版本、关键输入在场；不含任务专属结论 /
  allowlist 内容 / 压测提示）。
- 快照生成 ≤5s（版本探测单项 ≤1s 超时、并发执行——实测 ≈1.6s）。
- 越权边界：epoch / receipt_id 组合显式报错（exit_code 1 + error）；
  kind 白名单由渲染层登记，越权 kind（如 secret_allowlist /
  task_conclusion）渲染层拒绝；live-only 不入黑板持久分区/存档。
- 连通性判定由 W-F12（M2 T2.2）快速确定性失败闭环后接入——本步
  fail-closed 不产生 connectivity 行。

## 2. 代码改动

### 2.1 orz-loop（黑板 env 分区）

- `host.rs`：`EnvSnapshotFact { kind, key, value }` + 
  `LoopHost::env_snapshot_facts()`（默认空，fail-closed）。
- 新 `env.rs`：纯渲染 `render_env_text()`——`== env (live) ==` 头、kind
  白名单（tool/language/package/input/connectivity，越权 kind 拒绝）、
  kind→key 排序、8KiB 预算截断、空态「（无）」。
- `controller.rs`：`render_env_section()`（live-only 守卫）+ blackboard_read
  描述与 section 枚举加入 env。
- `host_exec.rs`：env 分支（异步取 host 快照渲染；epoch/receipt 显式
  报错）；非法 section 文案与 8KiB 整响应边界同步。

### 2.2 orz-host（快照探测）

- 新 `env_snapshot.rs`：固定注册表探测（python3/python/node/cargo/rustc/
  git/go/gcc/clang/make/cmake/java/npm/pip3/pip/pwsh，kind 白名单内）；
  PATH 扫描存在性 + `--version` 并发探测（单项 1s 超时）；关键输入在场 =
  cwd 存在可见文件；绝不输出 allowlist/任务结论。
- `lib.rs`：`LoopHost::env_snapshot_facts()` 实现。

## 3. 测试

| 位置 | 用例 | 覆盖 |
|---|---|---|
| orz-loop env.rs | `unknown_kinds_are_rejected_at_render` | 越权 kind（allowlist/任务结论）渲染层拒绝 |
| orz-loop env.rs | `rows_sorted_and_empty_state` | kind→key 排序 + 空态 |
| orz-loop env.rs | `oversized_env_is_truncated_with_marker` | 8KiB 预算截断 |
| blackboard.rs | `blackboard_read_serves_env_section` | 真实工具链回达 + section 枚举声明 |
| blackboard.rs | `blackboard_read_env_combination_errors_are_explicit` | epoch/receipt_id 越权显式报错 |
| orz-host env_snapshot.rs | `version_value_is_capped_to_one_line` | 版本值单行截断 |
| orz-host env_snapshot.rs | `snapshot_is_bounded_and_structured` | ≤5s、kind 全在白名单、input 在场、key 唯一 |

## 4. 验证证据

- `cargo test -p orz-loop env`：13 passed（env 渲染 3 + 既有 env 族 +
  回达/越权 2）——实际 env 相关用例全绿。
- **`cargo test -p orz-loop --lib`：709 passed / 0 failed / 3 ignored**。
- `cargo test -p orz-host env_snapshot`：2 passed（快照实测 ≈1.6s < 5s）。
- `cargo fmt -p orz-loop -p orz-host -- --check`：净。
- orz 仓库 `git diff --check`：exit 0。

## 5. 边界声明

- 快照内容为通用机械事实（存在性/版本/输入在场布尔），不携带
  “无 mips 工具链”类压测结论、不携带 allowlist 内容（P4 纪律）；kind
  白名单是渲染层唯一放行口，越权键静默拒绝（fail-closed）。
- 连通性判定缺口登记：W-F12（M2 T2.2 本地透明层）落地后在同一
  `EnvSnapshotFact` 面追加 connectivity 行（单项 ≤1–2s）；本步不伪造。
- env 分区 live-only：读取即算即弃，不进黑板持久分区/会话存档单包
  （Blackboard 结构未增分区，`partition_revisions` 不受影响）。
