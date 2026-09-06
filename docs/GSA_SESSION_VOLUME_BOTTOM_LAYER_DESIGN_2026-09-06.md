# GSA SESSION VOLUME BOTTOM LAYER DESIGN（`.gsa` 会话卷底层部件化设计）

> **设计日期**：2026-09-06；状态：设计定稿（用户裁决方向，2026-09-06）。
> **上级**：ADR-0010 §14.56（本设计转录）；关联 [Task C](P0_GOV_UNCOMMITTED_REVIEW_HANDLING_2026-09-04.md)
> / [GLM 处置](P0_GOV_GLM_DISPOSITION_AND_TASK_D_S2_SCHEDULE_2026-09-06.md) §5
> / [GAP-GSA-SYMLINK-STALE-TEST](P0_GOV_GLM_DISPOSITION_AND_TASK_D_S2_SCHEDULE_2026-09-06.md)
> / [OBS-PERMISSION-DUAL-IMPL](../CLI_PROJECT_INDEX.md)。
> **背景定位**：`.gsa` 不是评测附件，而是主项目 LIF 的科学性组件——
> journal 证据链、会话黑板侧车、压缩快照、终端输出落盘、run_tests 完整
> 输出窗口、PDF 证据存储的物理载体（FUS-STATE-RECOVERY 可审计状态链的
> 落盘面）。用户裁决（2026-09-06）：该组件必须保留并下沉为底层部件。

---

## 1. 问题陈述

`.gsa` 的访问语义目前散落在多层，各自为政：

| 层 | 对 `.gsa` 的认知 | 问题 |
|---|---|---|
| orz-tools 工具沙箱（Task C，2026-09-04） | 无 `.gsa` 概念，workspace 内/外二元判定 | 不知道会话卷存在 |
| orz-host `permission.rs` | 证据面默认不可见 + terminal-log / run_tests 两个白名单窗口（symlink-aware canonical 比较） | 被 Task C 架空，窗口不可达 |
| orz-workspace permission manager（8,756 行） | 另一套权限判定 | OBS-PERMISSION-DUAL-IMPL 审计盲区 |
| 上游 grok_build harness GitignoreFilter | `.gsa` 被当普通 gitignored 内容拦 | 与 OUTPUT-DEGENERATION-GUARD「gitignored 也必须可读」矛盾 |

已暴露的矛盾：Task C 工具级拒读使 permission.rs 的会话卷白名单对
read_file 不可达——评测容器把会话卷挂载为 `.gsa` 符号链接的形态下，
终端截断补读链与 run_tests 完整输出窗口断裂。根因是 `.gsa` 没有单一
权威的底层身份，每层各自用路径字符串和临时判定去"认出"它。

## 2. 目标与非目标

**目标**：把 `.gsa` 正式化为底层（工具/机械层）拥有的类型化系统状态域
`SessionVolume`，访问语义单源下沉至工具级沙箱，消除分层矛盾；恢复
terminal-log / run_tests 两个科学功能窗口；fail-closed 默认不放松。

**非目标（防膨胀）**：

1. **不退役权限层**。权限层保留并将继续演进；用户已定方向（2026-09-06）：
   后续在助理层运行中拦截系统核心路径、且仅作**删除保护**——该演进另行
   立项，本设计不做任何权限层裁撤承诺，只在 §5 登记衔接点。
2. 不新增任何写入面：两个窗口均为只读；`.gsa` 写入仍全部由 runtime
   自有路径完成，模型不可写。
3. 不扩大窗口：仅 terminal-log 与 run_tests_output.txt 两个白名单形态，
   其余 `.gsa` 内部（journal、会话状态、keystore、快照、PDF 证据等）保持
   agent-invisible。安全面任何进一步放开必须单独立项裁决。

## 3. 设计

### D1 SessionVolume 类型化资源（走 SkillRoots 先例）

- `Resources` 新增 `SessionVolume { canonical_root: PathBuf }`（命名可定
  `SessionVolumeRoot`），由 orz-host 装配期注入。
- host 是唯一知道 `.gsa` 真实落点（含 fallback 链、评测容器会话卷挂载）
  的角色：装配期做**一次** symlink-aware canonical 解析
  （`dunce::canonicalize(cwd/.gsa)`，失败回退词法路径），解析结果即
  canonical_root——下层不再各自 canonicalize 再猜。
- **fail-closed**：SessionVolume 缺席（资源未注入）时窗口全关，沙箱退回
  纯 workspace 二元判定（即 Task C 现状语义）。

### D2 工具级沙箱三分判定

`is_path_within_workspace` 判定域扩展为三分（read_file / grep / list_dir
三个读工具统一）：

1. workspace 内 → 放行（Task C 现状不变）；
2. SessionVolume 内 → 按 D3 窗口契约判定；
3. 两者之外 → 拒（canonical 级 fail-closed 不放松）。

判定顺序：先 workspace，后 session volume；SkillRoots 豁免优先级不变。

### D3 窗口契约（单源下沉）

`.gsa` 面可见性语义唯一权威 = 本节；permission.rs 现行语义原样下沉：

- **默认 agent-invisible**：journal、会话状态、keystore、快照、PDF 证据、
  conversations 等一切 `.gsa` 内部面不可读。
- **只读窗口仅两个**：
  - `session/terminal/*.log`（终端截断补读链，OUTPUT-DEGENERATION-GUARD）；
  - `run_tests_output.txt`（run_tests 完整输出窗口，GAP-RUN-TESTS）。
- **窗口判定 = 双条件**：canonical 落点 ∈ canonical_root
  **且** 词法路径 ∈ cwd 且命中白名单文件形态。双条件保证：
  - `.gsa` 本身是符号链接（会话卷挂载）→ 放行（canonical 条件对卷根
    解析后比较）；
  - 种在 `session/terminal/` 内的二级 symlink 逃向卷外或任意宿主路径 →
    canonical 条件拒绝；
  - 白名单外的 `.gsa` 路径（即使 workspace 判定之外的词法形态）→ 拒。
- 窗口外即拒，回包沿用 `PermissionDenied` 信封（含 `session_volume_
  invisible` 类语义的 reason 可后续定名，不新开事件面）。

### D4 gitignore 交互

会话卷不是工作区内容，gitignore 对它无语义。D2 判定为 session volume
路径时**绕过 GitignoreFilter**（含 `.gsa/`、`*.log` 命中情形）——恢复
OUTPUT-DEGENERATION-GUARD 原设计「即使 gitignored 也必须可读」的完整
意图。orz 生产面本不注入 GitignoreFilter（已核实），本条主要收口上游
grok_build harness 集成面。

### D5 权限层衔接（非裁撤）

- permission.rs 的 `.gsa` 特判（`access_in_scope` 中 gsa 白名单段）在
  D3 落地后标记退役（语义已由工具层单源承担），但**权限层本身保留**。
- 权限层后续演进方向（用户 2026-09-06 定向）：助理层运行中拦截系统核心
  路径、仅作**删除保护**。该方向另行立项，不在本设计范围内；本设计仅
  保证 D1–D4 不预占、不阻碍该演进（SessionVolume 资源对权限层同样可见，
  可复用 canonical_root）。
- OBS-PERMISSION-DUAL-IMPL 部分消解：`.gsa` 面获得单一 owner；双权限
  实现的整体收敛仍随终局治理视野排期。

### D6 与 GAP-GSA-SYMLINK-STALE-TEST 的关系（精确化，非回退）

- 被否决的旧行为：任意 symlink 越界可读（gitignore canonicalize 副作用，
  无边界）——保持否决。
- 被保留的：fail-closed 默认、越卷 symlink 拒读测试
  （`read_file_rejects_gsa_symlink_resolving_outside_git_root_even_when_
  gitignored`）语义继续成立。
- 精确化新增：会话卷内两个白名单窗口放行（D3）。实施批落地时在 GAP
  收口注记追加演进记录（拒读 → 窗口化，附本设计裁决）。

## 4. 实施切步（最小可验收单元，可随时停）

- **S1 代码**：D1 资源 + host 装配注入（canonical 单源解析）+ D2 三分
  判定 + D3 窗口契约 + D4 gitignore 绕过；permission.rs `.gsa` 段退役
  标注（D5）。验收：`cargo check`/`clippy` 无新增告警。
- **S2 测试**：§5 测试矩阵全绿 + orz-tools/orz-host/orz-loop 全量回归 +
  Python 侧不受影响核对（assurance 不涉及 `.gsa` 判定）。
- **S3 接线复验**：terminal 截断补读链与 run_tests 输出窗口的端到端
  （含 `.gsa` 为符号链接指向临时会话卷目录的实机构造）；GAP 收口注记、
  索引/BACKLOG/TODO 状态同步。
- **S4 收口**：处置审计/索引 v++、门禁 Exit 0。

## 5. 测试矩阵

| # | 场景 | 预期 |
|---|---|---|
| 1 | workspace 内普通路径 | 放行（现状回归） |
| 2 | workspace 外绝对路径 | 拒（Task C 回归） |
| 3 | `.gsa` 真实目录（cwd 内）+ `session/terminal/*.log` | 放行（新窗口） |
| 4 | `.gsa` 真实目录 + `run_tests_output.txt` | 放行（新窗口） |
| 5 | `.gsa` 真实目录 + journal/会话状态路径 | 拒（invisible 默认） |
| 6 | `.gsa` 为 symlink → 会话卷目录（cwd 外），读 terminal log | 放行（评测容器形态） |
| 7 | `.gsa` 为 symlink → 会话卷，读卷内 journal | 拒 |
| 8 | `.gsa` 为 symlink → 任意非会话卷路径 | 拒（现行拒读测试语义保持） |
| 9 | `session/terminal/` 内二级 symlink → 卷外 | 拒（防逃逸） |
| 10 | 窗口路径被 `.gsa/` + `*.log` gitignore 模式覆盖 | 放行（D4，harness 面） |
| 11 | SessionVolume 资源缺席 | 窗口全关，退回 Task C 现状（fail-closed） |

## 6. 验收口径

- 测试矩阵 11 项全绿；orz 全工作区 `cargo test`/`fmt`/`clippy`（存量
  告警不变）；门禁 Exit 0；manifest 重算。
- 语义单源核验：`.gsa` 访问判定在 orz-tools 沙箱单点实现，permission.rs
  无残留第二判定（grep 核对）。
- 索引/BACKLOG/TODO/GAP 注记同步；本设计文档与 ADR-0010 §14.56 一致。
