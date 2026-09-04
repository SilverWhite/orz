# P0-GOV 未提交批全面审查与 Task C 收口（审查处理审计）

> **审计日期**：2026-09-04
> **审查性质**：对当前未提交内容的三轴审查（设计合理性 / 实现合理性 /
> 设计与实现符合性）及处置登记；随后补齐并闭合 P0-GOV 任务 C。
> **范围**：父仓库未提交批（P0-GOV Phase 1–4 治理、权威文档校准、S4/TER
> 进度与桥接脚本、门禁与 manifest 修复）+ orz 子模块工作树在制品（Task C）。
> **方法**：只读实测 + 源码/证据静态核对；验证含 `check_repository.py` 门禁、
> `cargo check --workspace`、`cargo fmt --check`、`git diff --check`、定向与
> 全量单测、manifest/commit 核验、VM 桥接结果文件回查。
> **关联前序**：[`GLOBAL_ARCHITECTURE_AND_INTEGRITY_AUDIT_2026-09-04.md`](GLOBAL_ARCHITECTURE_AND_INTEGRITY_AUDIT_2026-09-04.md) /
> [`GLOBAL_ARCHITECTURE_DEEP_AUDIT_2026-09-04.md`](GLOBAL_ARCHITECTURE_DEEP_AUDIT_2026-09-04.md)。

---

## 1. 总评

批次方向正确、Phase 1–3 与任务 A/B 的修复真实可验证；但审查时仓库门禁
实际为红（唯一错误 = orz 子模块工作树脏），`cargo fmt`/`git diff --check`
不净，且 orz 内 8 个文件为未提交在制品（即任务 C 半程）。本审计随后完成
任务 C 并提交 orz（`3ccf9efe` 初版 + `a88a566f` canonical/解析单源收口），
重算 manifest，修复格式与文档问题，使门禁回到 Exit 0。剩余开放项为任务 D
（见 §6）；`vm-cred-set` 已按用户裁决移除 op（见 §3 R-3）。

## 2. 核验为真的事实（符合性正面证据）

- `check_repository.py` 新增 3 个 run-event v0.2 payload 夹具映射正确
  （夹具真实存在于 `runtime/fixtures/run-event-v0.2/payloads/`）。
- `orz_source_manifest.sha256` 按子模块 HEAD 的 git blob 生成，与工作树
  无关；重算后仍为 1434 个源文件。
- 任务 A：commit `7a5dc08e` 净移入 `render_fold.rs` 1338 行、`epoch.rs`
  净减 1363 行（现 epoch.rs 1970 行；render_fold.rs 含测试共 1938 行，
  TODO 的“1338 行”指剥离净移入量）；epoch.rs 保留 legacy plan-epoch
  归档/`--plan` 支持与兼容重导出，并非“退役约 3300 行死代码”。
- 任务 B：commit `e3d990ba` 将 workspace members 64 → 49，Cargo.lock
  削减 1131 行；15 个僵尸 crate 不再作为成员（残留于 workspace.dependencies
  仅为路径声明，无成员编译负担）。
- 权威文档与代码一致：8 工具直接执行 / console 动作单工具退役 / plan-first
  默认关闭 / `max_tool_rounds=0` 默认无硬限 / 会话黑板单实例 + 折叠渲染 +
  单 gzip 归档 / ACAF 未配置即 fail-closed——与 ADR §14.52/§14.55 相符。
- 两处 Markdown 相对死链修复正确；`releases/orz-0.3.0-linux-x86_64/`
  README 存在；README 0.3.0 引用成立。
- orz-loop 全量 725 测试通过（723 + 新增 2 项 env 解析单测）；沙箱定向
  12 项全过；orz-bin `acaf_fail_closed` env 单测过；全工作区 `cargo check`
  0 错误 0 告警（需将 `orz\bin` 加入 PATH 供 protoc）；`cargo fmt --all --check`
  净。
- Task C 沙箱语义实测：`.gsa` 会话内部面为真实目录（`session_cwd/.gsa` 由
  `create_dir_all` 创建；本机 D:\CLI\.gsa 亦为普通目录），canonical 级拦截
  不影响内部面读取。

### 2.1 任务 A/B 可靠性复核（2026-09-04）

- **任务 A（解耦可靠性）**：生产折叠渲染入口 `render_exec_folded` /
  `render_edits_folded` 现只定义于 `render_fold.rs`（controller.rs 经
  `crate::render_fold::…` 调用）；`render_fold.rs` 依赖仅为
  `blackboard`/`summary`/`lif::Domain`，不引用 `epoch` 状态；epoch.rs 保留
  兼容重导出（`pub use crate::render_fold::{…}`）与 legacy 归档读取
  （`with_blackboard_archive_dir`/`render_section` 等，服务 `--plan` 诊断与
  历史归档），不参与会话黑板生产折叠主链。行为保持证据：7a5dc08e 为纯搬移
  （render_fold +1337/−1、epoch −1351/+12），orz-loop 全量 725 测试
  （含 render_fold 折叠视图用例）通过。
- **任务 B（底座瘦身可靠性）**：`cargo tree -p orz-bin` 实测不含 15 个被剔除
  crate 中任何一个；workspace members 64 → 49；Cargo.lock −1131 行；源码
  目录均保留未删（仅移出编译成员，无数据丢失）；`cargo check --workspace`
  0 错误 0 告警，orz-loop/tools/bin/host 关键单测全绿。
- **表述修正**：原“隔离/退役 epoch.rs 约 3300 行死代码”夸大了范围——正确
  口径为“生产折叠渲染脱离 epoch 寄生；epoch.rs 保留 legacy 归档/`--plan`
  支持 1970 行与兼容重导出”。TODO/BACKLOG 已同步修正。

## 3. 发现与处置登记

| ID | 级别 | 位置 | 问题 | 处置 |
|---|---|---|---|---|
| R-1 | P0 | 仓库门禁 / orz 工作树 | 审查时 `check_repository.py` Exit 1：`orz submodule working tree is dirty`，与批次“全绿”声明冲突 | 完成 Task C 并提交 orz（`3ccf9efe` + canonical/解析收口 `a88a566f`），重算 manifest，门禁复跑 Exit 0（§5） |
| R-2 | P0 | `acaf_flow.rs` 等 | `cargo fmt --all --check` Exit 1；`git diff --check` 报 architecture/current README、BACKLOG、.gitignore 尾随空白/多余空行 | 已执行 `cargo fmt --all` 并手工修复文档空白，复跑均净（§5） |
| R-3 | P1 | `scripts/s4_vm_cred_set_v2.ps1` + admin bridge | `vm-cred-set` op 已接入桥接协议，但 09-03 8 次尝试 7 次 FAIL、1 次“OK”未做内容比对；末次 `CRED_BLOB_MATCH=False`；S4_PROGRESS 未记录 v2 尝试 | 2026-09-04 用户裁决移除：op 已从 admin bridge/request 摘除；v2 脚本保留本地并 `.gitignore` 排除；key 文件覆盖通道为唯一现行注入路径；S4_PROGRESS §16.19 已定案登记 |
| R-4 | P1 | TODO/BACKLOG/AUDIT | P0-GOV 闭合只有 checkbox、无证据边界的收口载体 | 本文档作为 P0-GOV 收口与审查处理载体，登记证据与遗留 |
| R-5 | P2 | `.ter_review_2026-09-04/` | 未跟踪审查工作目录（含 rust 源复制）未入 ignore | `.gitignore` 增加 `/.ter_review_*/` |
| R-6 | P2 | architecture/current README | 状态词 `current-projection` 不在索引 §0.2 允许状态集 | 改为 `current-design`（投影说明），同步消除 mermaid 空白行尾随空格 |
| R-7 | P2 | 索引 GAP-SUBAGENT-RUNTIME / retrieval dispatch 注释 | “独立 120 轮预算”措辞在 §14.55 轮预算转正后过时（0k 第二批已改档位默认 30/60/90） | 已复核并修正：索引条目与 dispatch 注释同步为“独立上限 + 档位默认 30/60/90 + `ORZ_RETRIEVAL_MAX_TOOL_ROUNDS` 覆盖，与主车道取 min”（orz `a88a566f`） |
| R-8 | P2 | TODO 任务 A 措辞 | “render_fold 1338 行”与现文件 1938 行字面出入 | 按“净移入 1338 行”理解，BACKLOG 措辞已同步 |

## 4. Task C 闭合记录（安全收敛）

### 4.1 实现清单（orz commit `3ccf9efe` + 收口 `a88a566f`）

- `crates/codegen/orz-tools/src/types/resources.rs`：抽公共
  `is_skill_markdown`（ASCII 大小写不敏感组件匹配，注释文档化）与
  `is_path_within_workspace`（canonical 级统一沙箱判定 + 语义文档）。
- `grok_build/read_file|grep|list_dir`：调用点统一越界拒绝（PermissionDenied /
  exit 1 信封），技能文档豁免；注释与 canonical 语义一致。
- `orz-loop/src/controller.rs`：`default_acaf_fail_closed()` 默认 enforce
  （`ORZ_ACAF_FAIL_CLOSED=0|false|no|off` 显式逃生；非法值回退 enforce +
  warning）；生产构造默认走 fail-closed，`cfg(test)` 保持默认 shadow 且可由
  env/`with_acaf_fail_closed(true)` 显式开启；`parse_acaf_fail_closed_env`
  为单一解析源（CLI exit 2 与库 warn+enforce 共用，含 trim 归一）。
- `orz-loop/src/acaf_flow.rs`：`ticket_flow` 无 signer 且 fail-closed 时
  立即 `SignerUnreachable` 拒票（startup fail-fast 之外的纵深防御）。
- `orz-host/src/acp_server.rs`、`orz-tui/src/runner.rs`：默认
  `acaf_fail_closed = default_acaf_fail_closed()`，与 CLI 行为对齐。
- 收口 `a88a566f` 附项：`retrieval/dispatch.rs` 陈旧 120 轮注释修正为 0k
  档位默认（30/60/90）+ env 覆盖语义；orz-bin main.rs 改用单一解析函数。

### 4.2 边界语义（登记）

- 拦截粒度 = canonical 级：目标存在时按真实落点判定——`..` 相对越级跳出
  cwd、绝对路径直接指向 cwd 外、以及工作区内符号链接/重解析点指向 cwd 外
  三类通道一律拒绝。
- 技能文档（`SKILL.md` 文件名或路径含 `skills` 组件）为只读知识注入豁免。
- `.gsa` 等会话内部面为真实目录（`session_cwd/.gsa` 由 `create_dir_all`
  创建，本机实测非重解析点），canonical 判定不影响；如未来引入重解析点
  内部面，需先显式登记豁免。
- 待创建路径（读前先写）回退词法归一化判定，不误拦工作区内新文件。
- 符号链接外逃拒绝由单测
  `path_within_workspace_rejects_symlink_escape_via_resolved_target` 锁定。

### 4.3 验证证据

- 沙箱单测 12 项全过（`..` 逃逸、绝对路径越界、符号链接越界拒绝、内部/
  待创建路径放行、skills 豁免、大小写组件匹配；read_file/grep/list_dir
  集成层各含绝对路径越界用例）。
- `cargo test -p orz-loop --lib`：725 passed / 0 failed / 3 ignored（含新增
  2 项 `parse_acaf_fail_closed_env` 单测）。
- `cargo test -p orz-bin --bin orz acaf_fail_closed_defaults_enforced_with_explicit_opt_out`：
  1 passed（CLI env 语义在单源化后保持）。
- `cargo check --workspace`：Exit 0，0 错误 0 告警；`cargo fmt --all --check`：
  Exit 0。
- 父仓门禁复跑：`python scripts/check_repository.py` Exit 0（error_count=0，
  valid=true，见 §5 记录）；`git diff --check` 净。

## 5. 复验记录（2026-09-04）

- `cargo fmt --all`：已修复 acaf_flow.rs 等格式；`--check` 复跑 Exit 0。
- `python scripts/generate_orz_source_manifest.py`：wrote 1434 entries。
- orz 工作树提交后 `git status --porcelain` 净。
- `python scripts/check_repository.py`：Exit 0（error_count=0，valid=true）。
- `git diff --check`：Exit 0（含父仓文档空白修复后）。
- 二轮收口（canonical 沙箱 + env 解析单源，orz `a88a566f`）：fmt/check 复跑
  净；沙箱 12 项 + orz-loop 725 + orz-bin env 单测全过；manifest 重算 1434。

## 6. 遗留开放与待用户裁决

- **任务 D（双实现终局治理）**：Rust `orz-assurance` 补齐规则、逐步退役
  Python `assurance` 双法官冗余——未开始，保持开放（TODO P0-GOV / BACKLOG 00）。
- **`vm-cred-set`（R-3，已处置）**：2026-09-04 用户裁决移除——op 已从
  `s4_admin_bridge.ps1`/`s4_bridge_request.ps1` 摘除；v2 脚本保留本地
  （`.gitignore` 排除，不入库）；key 文件覆盖通道为唯一现行注入路径。
- **检索子代理轮上限措辞（R-7）**：已复核修正（索引 + dispatch 注释同步为
  档位默认 30/60/90），不再遗留。
- 父仓 orz 指针（`3ccf9efe` → `a88a566f`）与 manifest 已更新未提交；本次
  TODO/BACKLOG/索引/架构 README/两份 GLOBAL 审计/.gitignore 更新同样未提交，
  等待用户审阅后统一提交。
