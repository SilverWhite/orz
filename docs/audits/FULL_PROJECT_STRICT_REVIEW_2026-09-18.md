# 全项目全面严格审查报告（FULL PROJECT STRICT REVIEW，2026-09-18）

> 状态：`reference`；索引登记 `AUTH-FULL-PROJECT-STRICT-REVIEW`。
> 发起：用户令「请对本项目进行一轮全面严格审查」（2026-09-18）；处置令（同日）：「请先落一份审查报告，随后将审查出的全部问题都立项为本轮审查的待修项」。
> 方法：四路并行只读深查（①Rust 生产代码与未提交 0am 影子批；②文档-代码一致性；③测试·CI·保障体系；④仓库卫生·git·许可）＋主会话独立工具链复核（fmt／clippy／`cargo test` 实跑）与关键发现逐条亲核（CI 断链行、920K 残留文案、flaky 复跑、protoc 前置、`gh run list` 均为主会话复验）。
> 范围：父仓 main @ `93220794`；orz 子模块 feat/fusion-architecture @ `08ab194c`（工作树含未提交 0am 影子 RLI 批）。审查对象为 2026-09-18 时点状态。
> 处置：全部发现以 **RS-01…RS-18** 立项入 [BACKLOG 0aq](../../docs/BACKLOG_AND_PRIORITIES.md)（35 → 36）；RS-13a 本批当改随批闭合。
> 范围限制：未做真实 API 端到端行为评测、性能/内存剖析、Linux 运行时体验验证；未深审密码学实现正确性（ACAF 仅核机制存在性与门控逻辑）。此四项如需可另立专项。

---

## §1 总体结论

1. **治理体系大体经得起核查**：README 十项核心断言逐一到代码验证全部属实（恰好冻结 10 工具、320K/500K 阶梯、黑板水位【x.xM/10M】、submit 两阶段、事件 schema v0.2 与 Rust 定义零漂移等）；安全机制七项声明（ACAF fail-closed、权限桥、票据门、凭据脱敏、URL 门禁、多字节路径修复）全部有真实代码支撑，无一处虚标；git 指针/推送状态完全一致；全库扫描未发现任何真实密钥；根目录临时杂物全部未被 git 跟踪。未提交的 0am 影子 RLI 批质量为可提交水平、默认关闭零行为影响。
2. **唯一 P0 是一个被治理体系漏掉的盲区：父仓 CI 已连续红灯 16 天（2026-09-02 起，至本审查日 09-18 的最新提交仍然 failure），v0.6.0／0.6.1／0.6.2 三次发布全部在红灯下打出**。仓库账本的「门禁 error_count=1（唯一＝orz 子模块 0am 影子批未提交）」只记了子模块未提交态，完全没记录 CI 断流这一事实——账本与真实工程状态之间存在盲区。
3. **工具链独立实测与审计回执存在口径差**：orz-loop 796/0/3、orz-assurance 246/0 与回执一致；但 orz-host 默认并行跑出 4 个失败（单独复跑全过＝负载敏感 flaky，串行要求仅存于提交信息、未文档化）、orz-tui 9 个失败全部因本机未配置 ACAF signer（测试不封闭、离箱即红）、orz-bin 因 `orz-tools-api` 的 protoc 前置无法编译（README 未载该前置）、未提交批 example 有 1 处 fmt 违规。「全绿」读数依赖特定的本地环境与跑法，换环境不可复现。
4. 总评：这是一个纪律性罕见的代码库，安全声明零虚标、钉子测试文化真实有效；主要欠账在 CI 断流（护栏断流 16 天）、测试封闭性/可复现性、以及仓库历史与磁盘卫生，而非版本库内容或安全机制本身。

## §2 发现清单（RS 立项对照）

严重级为审查时点定级，随处置批复核；每条给出证据与建议修法。

### RS-01（P0）父仓 CI 断流 16 天，5 个测试步骤整体断流

- 现象：`gh run list --workflow=ci.yml` 显示自 2026-09-02 起 30+ 次运行全部 failure；2026-09-18 当日三笔（含 v0.6.2 tag）均 failure。
- 根因：[`_windows_high_nist/S4_PROGRESS_2026-09-02.md:1128`](../../_windows_high_nist/S4_PROGRESS_2026-09-02.md) 把链接写成 Windows 绝对路径 `D:/CLI/docs/TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md`；`scripts/check_repository.py` 的 `_check_markdown_links` 在 Windows 上能解析盘符路径、在 CI 的 Linux runner 上不能 → 4 个 matrix job 全部死在第一步「Check repository contracts」。这是检查器自身的平台不对称缺陷，不只是单条链接笔误。
- 连锁后果：compileall、PowerShell 语法检查、Grok integration、runtime、assurance P0–P5 五个测试步骤自 09-02 起一次都没执行过。
- 账本盲区：门禁 error_count 口径只记 orz 子模块未提交态，CI 红灯 16 天无任何账本记录。
- 建议修法：①该行改相对路径；②`_check_markdown_links` 拒绝/规范化盘符绝对路径（平台对称化，杜绝同类复发）；③修后在 CI 上确证 4 job 全绿，并将「CI 状态」纳入门禁/账本口径。

### RS-02（P0→P1）CI 零 Rust 测试覆盖

- 现象：父仓 CI 只跑 Python 侧（ assurance/runtime/integration unittest＋契约门禁）；orz 约 1,400 个 Rust 测试只靠本地手工跑；orz 子仓 CI 仅一条 rustfmt。
- 影响：产品主体（Rust workspace）的质量护栏完全依赖审计回执自述；本审查实测已证明该读数环境敏感（见 §3）。
- 建议修法：CI 增加最小 Rust 测试 job＝`cargo test -p orz-loop --lib`＋`cargo test -p orz-assurance --lib`＋`cargo test -p orz-host --lib -- --test-threads=1`（串行，见 RS-04）。

### RS-03（P1）模型面残留文案「920K 压缩」与现行口径冲突

- 证据：`orz/crates/orz-loop/src/controller.rs:3209`（`blackboard_write` 模型可见工具描述写 "at the 920K compression only blackboard content plus the retention tail survives"，同串另有异常连续空格）；`orz/crates/orz-loop/src/agent_loop.rs:4300`（首轮工作台提醒「920K 压缩时只有黑板内容与保留尾可依托」）。
- 影响：920K 注意力阶梯已整档退役（现行＝700K 守卫／320K H1 硬打断／500K T1 硬截断，`context_scale.rs:18-19`）；模型可见文本仍在教授已不存在的压缩语义。README/架构投影均为新口径，此两处为漏网残留。
- 建议修法：两处按现行口径改写（或中性化措辞），顺修同串连续空格；随下一 orz 提交批带上。

### RS-04（P1）测试封闭性与可复现性

- orz-tui 9 个测试离箱即红：全部因「ACAF fail-closed is enabled but no signer client is configured」（本机未配置 `ORZ_ACAF_MANIFEST`/`ORZ_ACAF_KEYSTORE`）；测试依赖真实用户机 provision，不封闭。建议 ignorable 化或提供测试 signer fixture。
- orz-host 串行要求未文档化：默认并行下 `sweep_reaps_a_matching_orphan_and_kills_it`、`run_terminal_cmd_truncation_carries_output_object`、`call_tool_timeout_kills_process_tree`、`session_volume_symlink_windows_end_to_end` 4 例失败（负载敏感；主会话单独复跑 4/4 通过）；「串行 332/0/5」口径仅存提交信息。
- assurance 全量 1,668 个测试 30 分钟级不可收敛、无 slow/e2e 分层标记（慢源：`test_disposable_reproduction.py`、`test_canonical_cli.py`、live/e2e 类文件）；CI 修复红灯后将暴露为 30min×4job。建议快/慢分层（marker 或默认排除 live/e2e）。

### RS-05（P1）锁中毒级联与生产路径 unwrap 热点

- 204 处生产段 `lock().unwrap()`（Mutex/RwLock 中毒传播）：任何一处持锁 panic 会把全进程所有锁打成永久 panic（历史实证：05db4b3d 修的多字节 panic 即发生在锁内并导致 journal 断链全场丢弃）。热点：orz-host `acp_server.rs`（43）、`lib.rs`（26）、`tool_run.rs`（20）、`cdp.rs`（18）、`retrieval/dispatch.rs`（15）。建议热 Mutex 改 `unwrap_or_else(PoisonError::into_inner)` 或集中封装，按热点渐进。
- 生产路径 unwrap/expect Top 10（建议逐处 Result 化或收窄守卫）：
  1. `orz-host/src/disposition.rs:323/465/499` `act.pending` 三连 unwrap（守卫与使用点之间隔多个 await）；
  2. `orz-host/src/local_browser/cdp.rs:694` `page.as_mut().unwrap()`（I/O 最重通道）；
  3. `orz-host/src/codex_app.rs:352` `entry.agent_item.clone().unwrap()`（三行前刚赋值，应直用）；
  4. `orz-assurance/src/journal/recorder.rs:621/635` `seal_event(...).expect(...)`（恰在 run 已 degraded 的最坏时点 panic 写者线程）；
  5. `orz-host/src/acp_server.rs:2161/2175` grill 会话 `expect("grill session still active")`（跨 await 时间性不变量）；
  6. `orz-loop/src/plan/state_machine.rs:121/146/168` `current_plan.unwrap()` ×3（守卫改返回 Option 即消）；
  7. `orz-host/src/gateway/transport.rs:2308` `last_mut().unwrap()`（改用 push 返回值）；
  8. `orz-host/src/local_browser/cdp.rs:538/542` prefs 种子写入 expect/unwrap（环境敏感路径 Result 化）；
  9. `orz-loop/src/action_ledger.rs:155` 双重复核 unwrap（if-let 一步到位）；
  10. orz-bin `main.rs:1086` 唯一 unwrap（有 is_string 守卫，备查）。
- unsafe 面 86 处全部为 `std::env::set_var/remove_var`（2024 edition）或 Win32/DPAPI 接缝，无 `unsafe fn`/`unsafe impl`——可接受。

### RS-06（P1）0am 影子批提交前钉子（随 O2 裁决批执行）

- [P1] `orz/Cargo.toml:142` libm 用范围版本 `"0.2.15"`，而 RLI 的 bit-exact 重放契约实际靠 Cargo.lock 单行维持，`cargo update` 升 patch 即可能跨平台悄悄改变浮点语义 → 改 `=0.2.15`。
- [P2] 未跟踪新文件 `orz/crates/orz-assurance/examples/rli_shadow_replay.rs` 有 fmt 违规（主会话 `cargo fmt --check` 实测；回执只查了 `-p orz-loop`）→ `cargo fmt` 后随批提交。
- [P3] 五小件：T̂₀=8.0 双源（`rli.rs:380` vs `estimator.rs:17` `T_HAT_INIT_SECS`，应单源）；快照序列化 `zeta` 但 `restore()` 不应用（`rli.rs:516/348-359`，同结构体两字段恢复策略不一致）；`ORZ_LIF_RLI_SHADOW`（大小写敏感不 trim）与 `ORZ_ACAF_FAIL_CLOSED`（trim＋小写）两套 env 解析口径（`controller.rs:151-157` vs `265-271`）；example 默认 root 硬编码 `D:\tb-eval\jobs-official` 路径（`rli_shadow_replay.rs:418`）；prog 通道 ω 热更新本步整段 Δt 用新周期演化的建模近似宜补注释（`rli.rs:428-441`）。
- 批内正面确认：门控默认关＋「影子关闭不触碰 1D」逐 bit 钉子；侧车 `serde(default, skip_serializing_if)` 零迁移；无新增 panic 路径；锁序纪律显式登记（`controller.rs:1455-1461`）。

### RS-07（P2）高危模块测试补强

- 权限桥 `orz-host/src/permission.rs`（1,586 行）仅 4 个单测；历史缺陷模式是「controller risk_class 表与权限桥 access_kind 表两表不同步」（permission.rs:554-600 自记三例），遍历护栏只护 ReadOnly 豁免一侧；无 host 侧对真实 registry 的 deny 路径集成测试。
- `orz-bin/tests/acaf_e2e.rs` `#![cfg(windows)]`：非 Windows 的 fail-closed 仅靠 shadow 路径覆盖，E2E 缺口待处置决策。
- orz-loop／orz-host 无 `tests/` 集成层（loop 靠 470 个模块内单测）；浏览器 `cdp.rs`（3,671 行、I/O 最重）仅单测。

### RS-08（P2）巨石文件拆分补线（并入既有勘察，不另开线）

- 生产车道最大四件：`orz-loop/src/agent_loop.rs` 7,664 行、`orz-host/src/host_exec/tool_run.rs` 6,981 行、`orz-assurance/src/journal/families.rs` 6,530 行、`orz-loop/src/controller.rs` 6,260 行。
- 既有 [`HEAVY_FILE_SPLIT_SURVEY_2026-09-13`](HEAVY_FILE_SPLIT_SURVEY_2026-09-13.md) 三候选为 host_exec.rs／transport.rs／families.rs；本审查补报 agent_loop.rs 与 controller.rs（下一刀建议切 orientation/问询面与 session 渲染面）。处置随该勘察线，不动计数。

### RS-09（P2）仓库历史瘦身

- 滞留 ref 钉住大 blob：远程 ref `origin/feat/fusion-architecture`（4952bc8c，领先 main 334 提交，携带整棵 vendored 树）与本地分支 `codex/np1-body` 钉住 13.07MB `bin/protoc.exe` 及十几个 1.14–1.23MB 的 controller.rs 历史 blob；`.git` 总量 205MB，另有 loose 对象 103.88MiB 大量 dangling（amend/checkpoint 残留＋3 个 `refs/codex/turn-diffs/checkpoints/**` 工具树 ref）。
- 建议修法：确认两 ref 可删后删除＋`git gc`/`git repower`；预计显著回收。删除 ref 属不可逆操作，留处置批裁决。

### RS-10（P2）入库日志文件清理

- 8 个 tracked `*.log`：`_linux_arm_dryrun/build/build-arm-20260901.log`（45KB）、`build-arm-cross-20260901.log`（63KB）、`_windows_high_nist/pip-direct-run.log`（1KB）、`_windows_high_nist/vm-create.log`（17B）、`存档/root-artifacts-2026-09-12/orz-0vc-run-console.log` 等 4 件（合计约 6.3KB）。均已脱敏（复查命中 `***REDACTED***`），但日志不入库应为默认；git rm＋按 [`LIFECYCLE`](../../scripts/LIFECYCLE.md) 纪律归档。

### RS-11（P2）.gsa/ 磁盘卫生与外泄面

- `.gsa/` 768MB（7,015 文件，未跟踪、已忽略）：其中 `cargo-target/` 614MB 构建缓存混进会话目录，建议迁出；`keystore/installation-key.dpapi`＋`.json` 为 DPAPI 机器级安装密钥，建议确认 `installation-key.json` 内容是否含可移植 secret。当前不入库无问题；风险面在「打包含工作区的备份/压缩包」时的连带外泄。

### RS-12（P2）README 源码构建前置未文档化

- `cargo build -p orz-bin` 依赖 `orz-tools-api` build.rs 的 protoc（tonic/prost 代码生成）；protoc 为未入库的本地依赖（`orz/bin/protoc.exe`，`git -C orz ls-files bin/` 为空）。在未预置 PATH/`PROTOC` 的机器上 orz-bin 无法编译，README「从源码运行」一节未提此前置。建议 README 补前置说明（或把 protoc 纳入受管供给）。

### RS-13（P3）文档漂移三件

- **RS-13a（本批当改，已闭合）**：BACKLOG 优先级总览 P1 行 0am 未带「2026-09-17 整体挂起，O2 为前置门」标注（§0am 明细与 TODO 均有，总览路由行滞后）——本批随 0aq 立项一并补记。
- **RS-13b**：`CLI_PROJECT_INDEX.md` §3.1 `GAP-TOOL-BUDGET` 条目停留「MAX_TOOL_ROUNDS 改为 120」（2026-08-09，implemented）；现行为 `MAX_TOOL_ROUNDS = 0`（TER T1.7 撤除硬限，`controller.rs:67`）。补后续变更注记。
- **RS-13c**：`releases/` 实有 0.1.0／0.2.0／0.3.0／0.4.0／0.5.0／0.5.1 六目录，GitHub tag 另有 v0.3.1 无对应目录（README「0.1.0–0.5.1」区间断档）；目录命名后缀两段式不统一（0.1.0–0.4.0 用 `-linux-x86_64`、0.5.0/0.5.1 用 `-x86_64`）。

### RS-14（P3）.gitignore/.gitattributes 补

- 根 `.pytest_cache/` 未显式列入 .gitignore（现靠 pytest 自生成内部 `.gitignore` 自蔽，脆弱）；`.agents/` 空目录未忽略（一旦有内容即以 `??` 冒头）；`.gitattributes` 未对 `*.exe`/`*.png` 等标 `binary`。

### RS-15（P3）一次性产物生命周期清理批

- `scripts/` 内 83 个 `s4_vm_*` 一次性 VM 调试脚本已入库堆积（09-02～09-10）；`candidate-gates/` 1,363 个未跟踪评测产物自 07-30 堆积；`tmp_retrieval_0z`／`tmp_s2_patch`／`tmp_s3b`／`tmp_slider_review` 四个一次性目录待清理；`_linux_arm_dryrun/` 路线已被 zig cc 交叉编译取代仍留文档。全部沿 [`LIFECYCLE`](../../scripts/LIFECYCLE.md) 纪律处置（归档或删除留裁决）。

### RS-16（P3）Python 测试面卫生

- `pytest.ini` 与 `pyproject.toml [tool.pytest.ini_options]` 内容重复；CI 用 `unittest discover` 而 pytest 配置仅本地生效（口径分裂）；`scripts/` 顶层散落 `test_real_api_call.py`／`test_web_retrieval.py`／`test_browser_retrieval_e2e.py` 不被任何 CI 发现；`evaluation/`／`regression/` 名为测试实为纯资产目录（pytest 收集 0），命名有误导性。

### RS-17（P3）TODO2 M3 复验启动决策

- `TODO2.md`（TER 明细权威，TODO P0 行仍指向它）最后实质更新 2026-09-13，头部勾选证据基线自认 0.3.x 时点（现载体 0.6.x）；尾部 M3 回归复验 T3.1–T3.5 全部未勾选。是否启动复验留用户裁决。

### RS-18（P3）巨型文档增长观察（挂观察，不单独行动）

- tracked 巨型文档：`adr/ADR-0010` 475KB、`docs/BACKLOG_AND_PRIORITIES.md` 296KB、`CLI_PROJECT_INDEX.md` 211KB、`TODO.md` 201KB；另有 1.3MB 证据 JSON（`存档/.../LIF_102RUNS_REPLAY_2026-08-31_V2.json`）。均有治理声明背书；超限随既有瘦身机制（0ab S1 行宽/行龄检查＋存档快照）处置，本条仅登记增长趋势。

## §3 工具链独立实测读数（2026-09-18，未提交工作树）

| 检查 | 实测 | 对照审计回执口径 |
|---|---|---|
| `cargo test -p orz-loop --lib` | **796 通过 / 0 失败 / 3 忽略**（10.6s） | 一致（796/0/3） |
| `cargo test -p orz-assurance --lib` | **246 通过 / 0 失败**（3.0s） | 一致（246/0） |
| `cargo test -p orz-host --lib`（默认并行） | 328 通过 / **4 失败** / 5 忽略；4 例单独复跑 **4/4 通过** → 负载敏感 flaky | 回执「串行 332/0/5」；串行要求未文档化 |
| `cargo test -p orz-tui --lib` | 169 通过 / **9 失败**（全部＝ACAF signer 未配置，fail-closed 拒启） | 回执从未列 orz-tui 读数 |
| `cargo test -p orz-bin` | **无法编译**：`orz-tools-api` build.rs 找不到 `protoc` | — |
| `cargo fmt --check`（生产 5 crate） | 1 处违规：未提交 `rli_shadow_replay.rs`（回执只查 `-p orz-loop`） | 回执「fmt 干净」限于 orz-loop |
| `cargo clippy`（生产 5 crate 链） | 生产 crate 依赖链上 vendored `xai-tty-utils` 2 条既有 warning、零新增；`orz-tools-api` build 失败同 protoc | 回执「与基线持平」一致 |

## §4 未提交 0am 影子 RLI 批审查（10 文件 +500/−12，2 新文件 1,576 行）

- **机制**：现行生产 LIF 为一阶（1D）通道；本批在 `LifEngine` 旁挂**二阶谐振泄漏积分器影子族**（`RliShadow`，`orz/crates/orz-assurance/src/lif/rli.rs:364`）：每通道二维状态 (u,v)、事件间隔闭式精确解自由演化、事件注入只踢 u；五通道 err/stall/slow/deny/prog；锚点含 u/v/短视预测/解析包络/节律计数 r/自校准阈值 θ；三角/指数走 pinned pure-Rust libm、状态 3 位小数定点化，同平台重放 bit-exact。门控 env `ORZ_LIF_RLI_SHADOW`（默认关＝影子不构造/不喂入/侧车无字段，零成本）；持久化随 `TemporalSessionSnapshot` 侧车（`serde(default, skip_serializing_if)` 零迁移）；example `rli_shadow_replay.rs` 为离线影子评估 harness（预注册判据标签冻结 2026-09-17）。
- **prompt.rs +280 行**＝S1 Part A「墙钟→轮次预算投影行」：1-2-5 阶梯向下取整（checked_mul 防溢出）、T̂ 非法 fail-soft、两个渲染件与签名扩展；None 时块字节不变；中性措辞有禁词测试。质量高。
- **风险结论**：env 未设时对已提交行为零影响（有逐 bit 钉子）；序列化兼容；无新增 panic 路径；调用方全部同步（13 处 `TemporalSessionSnapshot` 字面构造）。**混合工作树的编译/测试绿灯此前仅为回执自述，本审查以实跑补证**（见 §3）。
- 提交前钉子见 RS-06。

## §5 安全机制核查（属实面，零虚标）

| README 声明 | 判定 | 证据 |
|---|---|---|
| ACAF 未配置即 fail-closed | ✅ | `orz-host/src/acaf_flow.rs:293-308`（`acaf.is_none()`＋fail_closed → `fail_closed_refusal` → `Blocked`）；`orz-bin/src/main.rs:398-420`（unset＝enforce） |
| `ORZ_ACAF_FAIL_CLOSED=0` 可关闭 | ✅（实际更保守：转 shadow 模式，拒绝照记、动作放行） | `orz-loop/src/controller.rs:265-271`；`acaf_flow.rs:168`；非法值 CLI exit 2／库层 warn＋enforce 双入口 |
| `--allow-shell`/`--allow-network` 必须同用 `--allow-write` | ✅ | `main.rs:34-41`＋`:106-111`（exit 2）；`=value` 形式显式拒绝；测试 `main.rs:3228-3266` |
| web_fetch/browser_read 过票据门、web_search 不走 | ✅ | `orz-host/src/acaf.rs:779-786` `action_kind_for_tool`（web_search 不在映射）；票据 ToolStarted 前发放（`tool_run.rs:927-948`） |
| 凭据目标注册与脱敏 | ✅ | 密钥值入 orz-secrets 注册表（`orz-host/src/lib.rs:304-305`）；统一脱敏出口 `scrubbed_json_pretty`，脱敏失败不写盘（`acp_server.rs:401-427`）；journal 落盘先脱敏再哈希封印（`journal/recorder.rs:255,836`） |
| URL 门禁与来源加权 | ✅ | `local_browser/url_gate.rs:1-75`（scheme 白名单/内嵌凭据/localhost/私有 IP/云 metadata 拒绝/DNS 失败 fail-closed/重定向复检）；`source_weighting.rs`＋`serp.rs:492-508`＋`cdp.rs:463,599,1169` |
| candidate_is_under_raw 多字节 panic 已修复 | ✅ | `codegen/orz-tools/src/types/resources.rs:500-527` 字节级判定＋钉子 `:2418-2433`；修复提交 05db4b3d 在 HEAD |

其余属实面：冻结 10 工具面恰好 10 个（宿主 6：`tools.rs:383-390`＋控制器 4：blackboard_read/blackboard_write/context_compress/submit）；上下文阶梯 320K/500K（`context_scale.rs:18-19,126-150`）；水位【x.xM/10M】（`agent_loop.rs:1149`＋`render_fold.rs:31`）；`.gsa/archives/` 增量归档实物（`6aac0af5.json.gz`）；`--retrieval-enabled` 门＋legacy `--retrieval-mode` 兼容解析（`main.rs:130-149,342-367`）；超时/轮次/问询常量全对（360s/300s/180s/`MAX_TOOL_ROUNDS=0`/`ORIENTATION_THRESHOLD=50`）；submit 两阶段（`delivery.rs:170-241`）；`browser_launch_result` 事件＋runtime schema；事件 schema v0.2（`event.rs:285-306` `0.2.0-draft` ↔ `runtime/run-event-v0.2.schema.json`）；黑板单条 ≤8K（`MODEL_NOTE_MAX_CHARS=8192`）。事件 schema 抽查 4/4 一致、版本链闭合、Rust 侧有防漂移钉子。

## §6 文档一致性核查

- **新鲜度异常高**：README/TODO/BACKLOG/索引/ADR-0010/architecture/current 全部为 2026-09-18 当日提交；README 九个入口文件全部实存、无断链。
- **索引抽查全中**：§3.1 抽 6 个「已闭合」差距（GAP-LOCAL-BROWSER／GAP-WEB-SEARCH-SEMAPHORE／GAP-INCREMENTAL-ARCHIVE-HEADLESS／GAP-TOOLNAME-LITERAL-CONVERGENCE／GAP-BYTE-BOUNDARY-PANIC／GAP-SUFFICIENCY-SCHEMA）全部在代码或产物中证实。
- **TODO 三件套无实质矛盾**：BACKLOG＝唯一权威（35 项口径）；TODO.md＝派生勾选投影；TODO2.md＝TER 明细权威（TODO P0 行指向它，非遗留垃圾）。
- **ADR-0010**：存在，`accepted / frozen`，5,635 行；§14.65/14.69/14.72 均在（提交引用无误）。
- 漂移项即 RS-13 三件（总览行 0am 标注滞后／索引 GAP-TOOL-BUDGET 滞后／releases 断档命名）＋RS-03（920K 模型面残留）。

## §7 仓库卫生、git 状态与许可

- **git 状态全一致**：父仓 HEAD pin＝子模块 HEAD＝远端分支 tip（`08ab194c`）；子模块领先/落后远端均 0；父仓 main ahead/behind 0/0；工作区唯一改动即 `M orz`（0am 批），与账本「门禁 error_count=1」口径完全吻合。全仓无「未跟踪且未忽略」流浪文件（`git ls-files --others --exclude-standard` 为 0）。
- **敏感信息零命中**：`sk-` 19 处／`Bearer` 3 处／`BEGIN PRIVATE KEY` 1 处全为测试假值；`ORZ_DEEPSEEK_API_KEY=<值>` 0 处；8 个入库日志已脱敏；`.gsa/` 未跟踪且已忽略（外泄面见 RS-11）。
- **杂物判定**：根目录 `.tmp-*`／`tmp_*`／`__pycache__`／egg-info／`.pytest_cache` 全部未被跟踪且被 .gitignore 接住；tracked 垃圾仅 RS-10 的 8 个日志；历史包袱即 RS-09；gitignore 缺口即 RS-14。
- **许可合规**：父仓 LICENSE Apache-2.0＋NOTICE 规范；orz 子模块 LICENSE（Copyright 2023-2026 SpaceXAI）＋`SOURCE_REV` 钉上游提交（6372e41d…）＋逐包 `THIRD-PARTY-NOTICES`（Source/License/版权行/满足路径）——`xai-*`/`orz-*` vendored crate 全部 Apache-2.0 声明，与根许可兼容，NOTICE 义务由 orz/THIRD-PARTY-NOTICES 履行；README「复用 Grok Build 组件、参考 Codex」与实物吻合（子模块 origin 远端即 xai-org/grok-build）。`upstream/` 仅为组件注册 schema/lock JSON（名称易误导）；SOURCE_REV 更新纪律宜在 NOTICE 注明（随 RS-14 类小件顺带）。
- **CI/评测装置**：runtime schema 与 Rust 定义零漂移（§5）；journal conformance「Rust 单一执法＋Python 冻结 reference」架构成立、入口真实存在（`check_repository.py`，即 RS-01 断流面）；`evaluation/`/`regression/` 为纯资产无执行入口（RS-16）。

## §8 亮点确认（审查中验证的真实优点）

1. 文档新鲜度与断链控制：当日提交、入口全通、索引抽查全中。
2. 安全声明零虚标；`ORZ_ACAF_FAIL_CLOSED=0` 实际语义比 README 更保守（shadow 记账）。
3. 钉子测试文化真实有效：防漂移钉（`host_resource_fact_table_covers_producer_kinds`）、遍历护栏（READ_ONLY 豁免表）、逐 bit 影子隔离钉、禁词测试。
4. 未提交 0am 批为可提交质量（门控默认关、零迁移、锁序登记、确定性契约）。
5. 提交账本可信：门禁口径与实际 git 状态逐项吻合。
6. 单一源纪律已生效：工具名 `orz_assurance::tool_names` 单源＋空豁免表机械钉。

## §9 立项与计数

- **0aq**「全项目全面严格审查处置线」立项入 BACKLOG（P1；**35 → 36**），RS-01…RS-18 挂线内勾选；**RS-01/RS-02（CI 修复与 Rust 测试轨）是否提级 P0 当前工作集待用户裁决**。
- **RS-13a 本批当改**：BACKLOG 优先级总览 P1 行 0am 挂起标注随本批补记闭合。
- 本批零代码、零子仓改动；计数 35 → 36 一处；账本三处同步（BACKLOG／TODO／索引 v3.66）。

## §10 关键词

全面审查、RS 立项、CI 断流、markdown 链接平台不对称、Rust 测试轨、920K 残留文案、测试封闭性、flaky、锁中毒、unwrap 热点、RLI 影子批、libm 钉版、权限桥覆盖、巨石文件、滞留 ref、入库日志、.gsa 卫生、protoc 前置、文档漂移、0aq。
