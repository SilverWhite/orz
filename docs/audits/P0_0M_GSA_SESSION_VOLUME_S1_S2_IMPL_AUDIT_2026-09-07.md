# P0-0m GSA-SESSION-VOLUME S1+S2 实施审计（2026-09-07）

> 范围：P0-0m S1（D1–D5 代码）+ S2（§5 测试矩阵 + 全量回归）。
> 设计权威：[`GSA_SESSION_VOLUME_BOTTOM_LAYER_DESIGN_2026-09-06`](../GSA_SESSION_VOLUME_BOTTOM_LAYER_DESIGN_2026-09-06.md)
> / ADR-0010 §14.56。排期依据：2026-09-06 用户裁决放行（BACKLOG 0m / TODO P0-0m）。
> 边界：S3 接线复验（terminal 补读链 / run_tests 窗口端到端 + `.gsa` symlink
> 实机构造）与 S4 收口未开始；GAP-GSA-SYMLINK-STALE-TEST 演进注记归 S3。

## 1. S1 落地明细（按设计条目）

### D1 SessionVolume 类型化资源

- `orz-tools/src/types/resources.rs`：新增 `SessionVolumeRoot(pub PathBuf)`
  资源（SkillRoots 先例，ephemeral 不 serde 注册）。语义 = host 装配期对
  `{cwd}/.gsa` **一次** symlink-aware canonical 解析（失败回退词法路径）
  后的卷根；**fail-closed**：资源缺席时窗口全关，沙箱退回纯 workspace
  二元判定（Task C 现状）。
- `orz-tools/src/registry/types.rs`：`SessionContext` 新增
  `session_volume_root: Option<PathBuf>` 字段（默认 None），finalize 在
  SkillRoots 注入点之后装配 `SessionVolumeRoot`（None = 不注入 = 窗口全关）。

### D2 工具级沙箱三分判定

- `resources.rs` 新增单点入口 `is_path_allowed_for_read(cwd, joined_path,
  resolved_path, skill_roots, session_volume)`，判定序：①已注册技能根豁免
  （GLM F1，优先级不变）→ ②会话卷域（`is_path_in_session_volume_domain`：
  词法形态 ∈ `{cwd}/.gsa` 或 canonical ∈ 卷根）→ 按 D3 窗口契约 → ③
  workspace（`is_path_within_workspace` 原语义不动）→ ④拒。
- 会话卷域判定**先于** workspace：`.gsa` 真实目录在 cwd 内原属 Task C
  workspace 放行面，必须由窗口契约接管（journal 等内部面 agent-invisible），
  否则白名单形同虚设。
- 三个读工具统一改接单点（原 `is_path_within_workspace` 直调替换）：
  `read_file/mod.rs`（resolved=Some）、`grep/mod.rs`（workdir, None）、
  `list_dir/mod.rs`（None）。
- 词法路径口径：`session_volume_lexical_path` = 相对路径按 cwd 拼接 +
  `normalize_lexically`（对应 permission.rs `normalize_lexical(cwd.join(p))`
  口径）。白名单形态只认模型可寻址词法路径；canonical 落点只用于包容性
  对照——`.gsa` 本身是符号链接（评测容器卷挂载）时模型路径词法形态不变，
  窗口判定不因解析落点在 cwd 外而失明。

### D3 窗口契约（单源下沉）

- `resources.rs::is_session_volume_window_path`：只读窗口仅两个，与
  permission.rs 现行语义逐条对应——
  - 窗口 1 `session/terminal/*.log`：词法形态 ∈ `{cwd}/.gsa/session/terminal/`
    且扩展名 `.log`（大小写不敏感），且 canonical 落点 ∈ 卷根（防二级
    symlink 逃逸），且词法路径 ∈ cwd；
  - 窗口 2 `run_tests_output.txt`：精确固定文件名，canonical 对照词法与
    卷根两种拼写（symlink-aware；名字被 symlink 顶替到卷内其它落点仍拒），
    且词法路径 ∈ cwd。
  - 其余 `.gsa` 内部面（journal、会话状态、keystore、快照、PDF 证据、
    conversations 等）agent-invisible，读请求拒。
- `read_file` 的 deny 消息对会话卷域区分文案：`path is inside the
  agent-invisible session volume`（沿用 `PermissionDenied` 信封，未新开
  事件面；`grep`/`list_dir` 沿用既有 sandbox 拒绝文案）。

### D4 gitignore 交互

- `read_file/mod.rs`：会话卷域路径跳过 GitignoreFilter（含 `RespectGitignore`
  面）——会话卷不是工作区内容，gitignore 对它无语义，恢复 OUTPUT-
  DEGENERATION-GUARD「gitignored 也必须可读」完整意图；orz 生产面本不注入
  GitignoreFilter，本条收口上游 grok_build harness 集成面。`grep`/`list_dir`
  的 ignore 行为为模型显式参数（`no_ignore`）与 walker 语义，不在本条范围。

### D5 权限层衔接（非裁撤）

- `orz-host/src/permission.rs::access_in_scope`：`.gsa` 白名单段加
  RETIRED-IN-PLACE 注记（方法级 + 段级）——语义已单源下沉 orz-tools 沙箱，
  该段作为权限门第一道的等义镜像**保留不演进**（用户裁决：权限层不裁撤），
  不得据此声称第二判定权威；权限双实现整体收敛仍随 OBS-PERMISSION-DUAL-IMPL
  终局治理排期。行为零改动（既有 permission.rs 测试全数照旧通过，见 §3）。

### host 装配注入

- `orz-host/src/tools.rs::build_toolset`：装配期一次
  `dunce::canonicalize(cwd/.gsa)`（失败回退词法路径）后经
  `SessionContext.session_volume_root` 注入。上游通用面
  （`orz-agent/builder.rs`、`orz-workspace/session/tool_config.rs` 两处
  SessionContext 字面量）传 `None`（无落点知情权，fail-closed）。

## 2. S2 测试矩阵（设计 §5，11 项）

纯函数级（`resources.rs` tests，8 个新测试函数）+ 工具级（`read_file`
tests，5 个新 async 测试）双层覆盖：

| # | 场景 | 覆盖层 | 结果 |
|---|---|---|---|
| 1 | workspace 内普通路径放行（Task C 回归） | 纯函数 `allowed_for_read_keeps_task_c_workspace_semantics` | 绿 |
| 2 | workspace 外绝对路径拒（Task C 回归） | 同上 | 绿 |
| 3 | `.gsa` 真实目录 + `session/terminal/*.log` 放行 | 纯函数 `real_gsa_volume_windows_are_allowed` + 工具级 `read_file_allows_real_gsa_terminal_log_window` | 绿 |
| 4 | `.gsa` 真实目录 + `run_tests_output.txt` 放行 | 纯函数同上 + 工具级 `read_file_allows_run_tests_output_window` | 绿 |
| 5 | `.gsa` 真实目录 + journal/会话状态拒（invisible） | 纯函数 `real_gsa_volume_internal_faces_are_invisible` + 工具级 `read_file_denies_gsa_journal_as_agent_invisible`（journal/state/目录三路径） | 绿 |
| 6 | `.gsa` symlink→cwd 外卷 + terminal log 放行 | 纯函数 `symlinked_volume_terminal_log_allowed_but_journal_denied` + 工具级 `read_file_allows_symlinked_volume_terminal_log_window` | 绿 |
| 7 | `.gsa` symlink→卷 + 卷内 journal 拒 | 纯函数同 #6 | 绿 |
| 8 | `.gsa` symlink→任意非会话卷路径拒 | 纯函数 `gsa_symlink_to_non_volume_path_denied` + 既有工具级拒读回归测试保持 | 绿 |
| 9 | `session/terminal/` 内二级 symlink→卷外拒 | 纯函数 `terminal_dir_inner_symlink_escape_denied` | 绿 |
| 10 | 窗口路径被 `.gsa/`+`*.log` gitignore 覆盖仍放行（D4） | 工具级 `read_file_bypasses_gitignore_for_session_volume_window` | 绿 |
| 11 | SessionVolume 资源缺席 → 窗口全关退回 Task C | 纯函数 `absent_session_volume_falls_back_to_task_c_semantics` | 绿 |

附加负测：`run_tests_name_symlinked_to_other_volume_target_denied`——
run_tests 文件名被 symlink 顶替到卷内其它落点（journal）仍拒（窗口 2
canonical 双拼写对照语义钉死）。

## 3. 回归与验证证据

- **orz-tools `--lib`**：2829 passed / 0 failed / 6 ignored（含
  types::resources 82、read_file 110、grep 46、list_dir 40）。
- **orz-loop `--lib`**：729 passed / 0 failed / 3 ignored（零改动面，零回归）。
- **orz-agent**：573 passed。**orz-workspace**：22 passed（21+1）/ 3 ignored。
- **orz-host**：214 passed / 34 failed / 4 ignored（`--test-threads=1`，
  skip 1）。**失败集与基线（git stash 后同口径重跑）逐项 diff 完全一致**
  ——34 项失败与 1 项挂死（`codex_app::tests::
  streamed_chunks_concatenate_to_response_text`，单跑复现且进程 CPU 零增长
  死等，timeout 杀出 exit 143）均为**存量基线问题**，与本次改动无关。
  存量失败抽样归因：ACAF fail-closed 下测试 run 拒启动（`ACAF fail-closed
  is enabled but no signer client is configured`），与 TODO 0j 登记的
  acaf_e2e 失败族同源；机器环境无 ORZ_ACAF_* 泄漏（env 与 User 级均空）。
  本批不处置，登记观察待独立轮次。
- **workspace `cargo check`**：零警告零错误。**clippy**（orz-tools/host/
  agent/workspace）：本批新增代码零告警（修复 1 处
  `unnecessary_lazy_evaluations`：`unwrap_or_else(|_| lexical_norm)` →
  `unwrap_or(lexical_norm)`）；剩余告警均为存量（terminal.rs 1558 等）。
- **fmt**：`cargo fmt --check` 净；`git diff --check` 净。
- **Python 侧不受影响核对**：`assurance/` 的 `.gsa` 引用仅为运行产物落盘
  路径约定（cli.py run-root 等），无任何 `.gsa` 访问判定；grep 核对无
  session_volume 概念泄漏。manifest 重算 1440 条。
- **门禁**：实施期间唯一报错为 `orz submodule working tree is dirty`
  （批次纪律内，本批提交后消除）。

## 4. 语义单源核验（设计 §6，S1 范围内）

- `.gsa` 访问判定单点 = `orz-tools resources::is_path_allowed_for_read`
  （域判定 + 窗口契约），三读工具共用。
- permission.rs `.gsa` 段按 D5 保留为等义镜像（非第二判定权威，注记
  在案）；设计 §6「无残留第二判定」验收项按 2026-09-06 用户裁决
  （权限层不裁撤）修正为「工具层单源权威 + permission 镜像冻结」，
  S4 收口时在索引/登记面按此口径表述。

## 5. 遗留与观察（不阻塞 S1/S2 闭合）

1. orz-host 存量测试失败 34 + 挂死 1（ACAF signer 配置族）——独立轮次
   定位，与 0j S5-2 验证期发现登记同族归并观察。
2. `grep`/`list_dir` 的 deny 消息未区分会话卷域文案（与 read_file 不同）：
   两工具输出走结构化信封（exit code 1 stderr / PermissionDenied 变体），
   文案统一非语义项，S3 复验时视实机反馈定夺。
3. 上游 `orz-agent`/`orz-workspace` 的 SessionContext 无会话卷注入
   （None = 窗口全关）：Grok Build 血统面无 `.gsa` 运行时，fail-closed
   符合设计；若未来上游需要窗口，须由其 host 层显式裁决注入。
