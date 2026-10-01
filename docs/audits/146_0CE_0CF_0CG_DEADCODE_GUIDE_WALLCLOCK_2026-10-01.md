# 146 批：0ce 闭合＋0cf/0cg S1/S2 落码（0.8.9 代窗口）（2026-10-01）

> **用户令**：「请直接进行0ce/0cf/0cg三项吧」。
> **本批**＝三件：① 0ce 死代码清退（S1–S4 全达成，**闭合 61 → 60**）；② 0cf blackboard_read
> 说明书简注（S1/S2）；③ 0cg 墙钟可见性拆除（S1/S2，含 orz-bin argv/env 收口）。
> 0cf/0cg 的 S3＝随 0.8.9 代载体重建进体（0.8.8 在役不动），S4 闭合随之。
> **零载体／零身份门改动**；orz 侧批提交 `be4f90ff`；**父仓未提交未推送**（沿惯例待令）。

## §0 结论速览

| 面 | 结果 |
|---|---|
| 0ce | `pending` → `implemented`，**61 → 60**：orz-agent 模板族 15 文件＋orz-subagent-resolution 孤儿 crate（3,018 行/7 文件）＋root `[workspace.dependencies]` 注册行清退；保留面六边实证；orz-agent 302/0、clippy 触碰面零新增 |
| 0cf | S1/S2 达成：`blackboard_read` 描述分区段末尾一句（framework usage manual / `section=guide`）＋section enum 增 `guide`＋blackboard 双钉；S3 待 0.8.9 代重建 |
| 0cg | S1/S2 达成：session 面 WALLCLOCK 三行＋T̂ 换算行＋任务状态轮次预算行同拆（TOOL_ROUND_*/status 保留）；orz-bin argv→env 桥退役＋OnceLock 静态化＋`remove_var`＋Linux argv/environ 内存 best-effort 零化（`argv_scrub` 四钉）；到期硬门 `run_invalidated{status: wallclock}` 不变；非 Linux 原位擦写缺位＝**已知边界**；S3 待 0.8.9 代重建 |
| 连带修复 | serde_json `preserve_order` 特性统一链断裂（0ce 剪依赖暴露）：orz-workspace/orz-host serde_json 边显式声明（消费事实落边）；cargo tree 差分定因 |
| 预存漂移 | 三处事件计数断言对齐（orz-host acp_server 9→10×2、orz-bin stdio_e2e 11→10）——0bz `face_fingerprint`（09-28）晚于 0bi 对齐（09-23）；**0.8.8 源冻结 `2da7dba0` stash 差分同形失败＝漂移先于本批**；gate 触发面漂移（极小流不触发 counterexample_gate＋第二轮 model_output）另记观察项 |
| 台账 | 本档；BACKLOG（指针行/计数行 60/P2 总览行/P2 开放项行/0ce 闭合 bullet/0cf・0cg S1/S2 bullet）；TODO（计数行 60/P2 路由行/P2-0ce 四勾/P2-0cf・0cg S1/S2 勾）；第二卷 §1.98；索引 v4.114 → **v4.115**（头行＋§8 implemented 增 0ce、pending 去 0ce）；[`0am 设计档 §0 Part A 拆除注记`](../LIF_DYNAMICS_PROJECTION_AND_ROUND_BUDGET_DESIGN_2026-09-16.md)；[`组件册 orz-subagent-resolution → retired`](../../upstream/fusion-component-register-v0.1.yaml)；门禁 `check_repository.py` `valid: true` |

## §1 0ce S1 勘定（per-module 消费表）

全仓 `orz_agent::` 路径逐一清点（含 orz-workspace `use orz_agent` 导入面），外部消费实证**六边**——
BACKLOG 立项时「仅 plugins＋prompt::skills 两面」为低估，勘定后六边全保留：

| 保留面 | 消费方（orz-workspace） |
|---|---|
| `plugins`（discovery/trust/project_plugin_dirs_in 等全模块） | discovery.rs、folder_trust.rs |
| `prompt::skills`（list_skills/SkillsConfig/CompatConfig） | discovery.rs |
| `prompt::agents_md`（read_agents_config_with_paths/AgentConfigFile）＋`prompt::ignore`/`workspace_user` | discovery.rs、agents_md 内部 |
| `repo::RepoDirChain` | folder_trust.rs:313、project_config.rs:9 |
| `discovery::project_agent_dirs(_in)` | folder_trust.rs:421（与 plugins 目录枚举互为镜像） |
| `config::workspace_grok_build_toolset` | handle.rs:3916 |

死面证据：`Agent`/`AgentBuilder`/`PromptContext`/`TemplateOverride`/`AgentDefinition`/预设注册表
（`preset_names`/`toolset_for_preset`/`register_toolset_preset` 全家族）外部消费者＝**零**（唯一
外部引用方 orz-subagent-resolution 本身死）；`apply_patch_template_source()`（`grok prompt
--section` 产品出口）**全仓零调用＝出口已不存在**，随族退役；compaction/system_reminder/timing
中 timing 因 plugins/skills 打点保留，其余零消费退役。

## §2 0ce S2 落码清退

- **删除 15 文件**：`templates/`（prompt.md 51 行 base＋apply_patch_prompt.md 275＋subagent_prompt.md 79）、`scripts/encrypt_templates.py`、`src/prompt/{template,prompt_encrypted,context,subagent_prompts,user_message}.rs`、`src/{agent,builder,compaction,system_reminder,error}.rs`。
- **孤儿 crate 整删**：`orz-subagent-resolution/`（3,018 行/7 文件；非 workspace 成员、零依赖方，仅 root 注册行悬空）＋root `Cargo.toml:299` 注册行退役。
- **收敛改写**：`config.rs` 2,587 → 工具集装配最小件（bash/task/task_output/wait/kill 五改名装配＋default＋workspace 两函数）；`discovery.rs` 1,501 → `PROJECT_AGENT_SUBDIRS`＋两目录函数＋2 新钉；`lib.rs` 六模块重写（config/discovery/plugins/prompt/repo/timing＋`workspace_grok_build_toolset` 再出口）；README 重写；`prompt/mod.rs` 四模块。
- **skills.rs 三个新死注入函数随批退役**（`format_skill_for_injection`/`format_skills_for_injection`/`resolve_preloaded_skills`——原消费方为已删 builder/context）。
- **Cargo.toml 剪七依赖**：minijinja/serde_yaml/strum/zeroize/xai-token-estimation/orz-sampling-types/xai-tool-types（保留面实际引用面复核后 定版：chrono/dunce/dirs/git2/ignore/orz-assurance/orz-config/orz-hooks/orz-tools/regex/serde/serde_json/sha2/thiserror/tokio/tracing/xai-tty-utils）。
- **边界兑现**：TB 零提示词姿态不动（零触碰）；加密模板连 plaintext 源一并清退、零转写留存（0bs 先例）；skills/plugins 在役面零触碰（新死函数除外）。
- **orz-memory** `storage.rs` 文档注记对齐（原引 `orz_agent::config::MemoryScope` 随机制退役；该 crate 非 workspace 成员，纯注释改动）。

### 连带修复：serde_json `preserve_order` 特性统一链断裂（如实记）

S2 后 orz-workspace 权限金样三测败（`permission_decision_render_golden` 等）——实际键序字典序化
vs 期望插入序。定因（cargo tree `-e features -i serde_json` 两态差分）：该特性原经
**orz-agent → orz-sampling-types** 链隐式统一开启；剪依赖后，orz-workspace 图内仅剩
**build-dependencies**（tree-sitter）使能——resolver v2 下 build-dep 特性不回流正常依赖面。
修复＝`orz-workspace`/`orz-host` 的 serde_json 边**显式声明** `preserve_order`（消费事实落边，
不再依赖跨 crate 隐式统一）。复测：orz-workspace 41/41、全套件 1,500/0。

## §3 0cf（S1/S2）

- 描述分区段末尾（`…零注入, nothing archived).` 之后）补：`0cf (2026-10-01): the framework
  usage manual lives on the blackboard too — section=guide reads it (mechanism-only, live-only,
  zero badges).`（英文中性 ≤1 句；单一源＝controller.rs:3860 描述串，全仓无第二副本）。
- section enum 增 `guide`（runtime 验证面 `tool_run.rs:2592` 本就放行 guide 分支，enum 缺位为
  文档缺口——随本注解一并补齐，模型面自洽）。
- 双钉（blackboard F-012 测试扩展）：enum 含 `guide`＋描述含简注字面。
- 边界：不加 PUSH、不加注入；其余工具描述零触碰。

## §4 0cg（S1/S2）

### ① 两渲染面拆除

- `prompt.rs`：`session_face_block_with_wallclock` 退役，`session_face_block(used, budget,
  status_line)` 收敛三参；`build_status_line` 去 `rounds_line` 参；`WALLCLOCK_ROUNDS_PREFIX`/
  `WALLCLOCK_ROUNDS_DISCLAIMER`/`ROUNDS_BUDGET_LABEL`/`round_budget_ladder_floor`/
  `round_budget_bucket`/`wallclock_rounds_line`/`rounds_budget_line` 全族退役。
- `controller.rs`：`wallclock_rounds_line` 方法退役；session pull 臂改调三参
  `session_face_block`；`render_status_line` 去投影行。**`run_elapsed_wallclock_secs` 保留**
  （F6 push 档仍消费——0cg 范围不含 F6：默认 off、TER 治理、机械注入面）；`TOOL_ROUND_*` 行与
  status 行保留；`main_wallclock_limit_secs_override` 保留（F6＋retrieval 派发保留量）。
- retrieval 派发保留量（`batch_close.rs` WALLCLOCK_RESERVED 族）＝机械面模型不可见，不动。

### ②③ orz-bin argv/env 收口

- argv→env 桥（P0-2 形态 `--max-wallclock` → `set_var(ORZ_MAX_WALLCLOCK)`）**退役**；
- `MAX_WALLCLOCK_INPUT: OnceLock<Option<String>>` 启动期一次解析（argv 优先、宿主常驻 env 次之
  ＝F-012 兼容）；随后 `env::remove_var`（派生 shell 的 `env` 与 `/proc/*/environ` 继承面收口）；
- Linux（`argv_scrub` 模块）：`/proc/self/stat` 字段 48–51 定界 argv/environ 区，
  `/proc/self/mem` 原位零化 `--max-wallclock`(+value) token 与 `ORZ_MAX_WALLCLOCK=` 条目——
  `ps`/`pgrep`（137 批实证 14 卷命中）与 `/proc/<pid>/environ` 外部读数不再可见；
  **best-effort fail-soft**（无 /proc、权限收紧即静默跳过；擦写是混淆面不是门）；
- **已知边界**：非 Linux 平台无原位等价语义（Windows GetCommandLine 不可擦）——评测在 Linux
  容器内，Windows 原生面留此边界；
- 到期硬门（`tokio::time::timeout` → `run_invalidated{status: wallclock}` 终态、journal 链
  完整）**语义不变**；两件既有守卫测试（RUN-WALLCLOCK/RUN-WALLCLOCK-DROP）原样通过。
- 测试：prompt.rs 五件 0am Part A 钉子退役＋新负向钉 `session_face_carries_no_wallclock_after_0cg`
  （session 面任何形态无 `WALLCLOCK_` 字面）；controller/blackboard session 断言改造；
  `argv_scrub` 纯函数四钉（token 零化/无后随 token/选择性 env 零化/stat 解析含空格括号 comm）跨平台实跑。

## §4-A 预存漂移对齐（三处；stash 差分实证＝先于本批）

| 测试 | 现象 | 差分证据 | 处置 |
|---|---|---|---|
| orz-host `acp_server::tests::session_prompt_produces_valid_journal_chain` / `second_prompt_continues_hash_chain` | 实得 10 vs 期望 9 | 0.8.8 冻结 `2da7dba0` stash 同败 10≠9 | 9 → 10（0bz `face_fingerprint` 逐轮事件 09-28 入列，晚于 0bi 09-23 的 11→9 对齐） |
| orz-bin `stdio_e2e::session_new_and_prompt_over_real_frames` | 实得 10 vs 期望 11 | 同上同败 10≠11 | 11 → 10（期望清单补 `face_fingerprint`；旧清单中 `counterexample_gate`＋第二轮 `model_output` 在 0bi 收窄后已不在极小流出现） |

**观察项（不占计数）**：极小 stdio/ACP 流上 `counterexample_gate` 与第二轮 `model_output` 不再
出现——0bi ⑩ 收窄语义的既成事实，触发面漂移根因不在本批定因，留后议。

## §5 S3 验证读数

| 面 | 读数 |
|---|---|
| 编译 | Windows `cargo check --workspace` 绿（PROTOC 显式）；Linux docker `rust:1.97-slim`（ORZ-BUILD-MOUNT-001 契约挂载＋apt protobuf-compiler）`cargo check -j 2 -p orz-bin -p orz-loop` **Finished 7m42s exit 0**＝`scrub_linux` cfg(linux) 路径编译证据（附录回填：构建于本批会话内完成，exit 码经任务通知确认） |
| 测试 | orz-agent lib **302/0**；orz-loop lib **855/0**（＋integration 2/0，3 ignored）；orz-bin 全靶 **60/0**（16+12ig／2／15／24／2／1）；orz-workspace **1,500/0**（＋21/0＋1/0）；orz-host lib **350/0**（并行负载敏感族失败集逐次漂移、逐件 `--exact` 复跑全过＝09-18 严格审查先例形态，如实记） |
| clippy | 触碰面基线差分（stash ↔ 现态）：**104 ↔ 104、逐文件分布一致＝零新增** |
| fmt | 触碰四 crate `cargo fmt`；`stdio_e2e.rs`/`acp_server.rs` 随批 |
| 源清单 | `generate_orz_source_manifest.py` → **1,485 条**（1506 − 21 恰合删除面：−22 删＋1 增）；门禁 `check_repository.py` **`valid: true`** |
| orz 提交 | `be4f90ff`（批提交，含 fmt 后终态；0.8.8 在役不动、零 bump——0.8.9 版本号随重建批） |

## §6 边界与如实记

1. **未提交未推送（父仓）**：本批纯落账＋orz 侧批提交；父仓提交/推送沿惯例待令；0.8.8 发布面仍未推送未发行。
2. **0cf/0cg 维持 `pending`**（计数 60 含两者）：S3＝0.8.9 代载体重建进体（0.8.8 在役件无新面），S4 闭合随之。
3. **0cg 已知边界**：非 Linux 平台 argv/env 原位擦写缺位；Linux 擦写为 best-effort（/proc 缺席即跳过）——门语义不依赖擦写。
4. ** preserve_order 教训**：跨 crate 隐式特性统一不可依赖——已落显式声明；同类面（其余 feature 依赖）未逐一排查（图无其它断裂迹象：Cargo.lock 包集零变化）。
5. **环境面（如实记）**：rustc 0xc0000409 三次（-j 2 重跑通过）；D 盘两次耗尽（清 incremental 31.5 GB＋linux-gnu debug 11 GB 续行）；Docker Desktop 守护进程一次崩溃重启；protoc dotslash 包装路径失效、以 `D:\CLI\orz\bin\protoc.exe` 显式 `PROTOC` 直连；杂散件 `tb_tree.json`（GitHub 404 响应残留，09-30）删除。
6. **0am Part A 设计档**：§0 已加拆除注记（146 批/0cg 指针），防文档-代码漂移。

## §7 关联与关键词

[`0ce BACKLOG`](../BACKLOG_AND_PRIORITIES.md)／[`0cg BACKLOG`](../BACKLOG_AND_PRIORITIES.md)／
[`0bh ⑭⑮ 设计档`](../BLACKBOARD_GUIDE_AND_POINTER_DESIGN_2026-09-22.md)／
[`LIF_DYNAMICS 设计档`](../LIF_DYNAMICS_PROJECTION_AND_ROUND_BUDGET_DESIGN_2026-09-16.md)／
[`144 批档`](144_CARRIER_REBUILD_V088_0CH_S3_2026-10-01.md)（0.8.8 在役与 0.8.9 顺延出处）／
第二卷 §1.98。

关键词：146 批、0ce 闭合、死代码清退、orz-agent 模板族、加密模板零转写、orz-subagent-resolution
孤儿 crate、preserve_order 特性统一、resolver v2 build-dep 不回流、0cf guide 简注、section enum、
0cg 墙钟可见性拆除、session_face_block 三参、轮次预算行退役、argv→env 桥退役、OnceLock、
remove_var、argv_scrub、/proc/self/mem 原位零化、best-effort fail-soft、非 Linux 已知边界、
face_fingerprint 漂移对齐、stash 差分、计数 61 → 60、orz `be4f90ff`。
