# 0bv 并件余项与 REV-083-21/22 修复批（过夜批，2026-09-27）

> **用户令**：「下一轮不做狗粮轮了，我考虑目前狗粮轮的数据已经初步足够，请你做0bv并件余项和REV-083-21／22 修复批吧，这个作为今晚的过夜任务」（2026-09-27 晚）。
> **基线**：父仓 `f32cbfdc`（092 批）；orz 子仓 `c558d6eb`（0.7.3）；两侧工作树含 0BV S4 轮三件未提交树面件（S4 读数档＋BACKLOG/TODO S4 块）起步。
> **形态**：work batch——**不提交 / 不推送 / 不重建**（载体重建与切换矩阵 ③④ 重走留下一令）；台账（索引/BACKLOG/TODO）树面同步、随落账批提交（D-1 惯例）。
> **落点**：本档（唯一父仓审计落点）；`D-*` ＝ 就地裁决；源清单＝[`0BV S4 真机复验`](0BV_S4_LIVE_VERIFICATION_2026-09-27.md) §4 余项＋[`0BV 结转报告`](0BV_CARRYOVER_REV083_0BW_2026-09-27.md) §5 结转第 2/3 项。

## §0 总览

| 项 | 状态 | 证据／说明 |
|---|---|---|
| REV-083-21（F-1 服务端 verbatim 恒 403） | **✅ 落码** | `orz-web/src/server.rs`：`strip_verbatim`（`\\?\UNC\`→`\\`、`\\?\`/`\\.\` 剥除）＋`canonicalize_usable`（dunce 合规形态）双入口（switch/`resolve_root`）；4 条新单测含真实 canonicalize 形态回归钉；orz-web lib **46/0**（复审后 47/0，见 §13） |
| REV-083-22（F-2 前端 sendJsonBody 未定义） | **✅ 落码** | `orz-web/assets/app/api.js`：`sendJson` 扩 body 载荷（Content-Type/JSON body），`switchWorkspace` 改走之；冒烟新增**真实 fetch 链路**回归钉；`frontend_smoke.mjs` 全过 |
| REV-083-11（web_fetch SSRF pin） | **✅ 落码** | `ssrf.rs` 新 `resolve_and_check`（校验并返回已校验地址集）；`http.rs` 重写为 `build_pinned_client`（`resolve_to_addrs` pin，共享客户端/ArcSwap 缓存退役）；`fetch_url` 每跳解析→校验→pin；web_fetch 测试 **131/0** |
| 0bw① Linux Landlock 落码 | **✅ 落码（真机读数待 Linux 载体；复审修复见 §13）** | `orz-sandbox/src/child_write_guard.rs`（直写 syscall 444–446；ABI 探测 v1/v2/v3 位收窄；`prepare_allow_dirs` 枚举 `/` 顶层排除表 B；顶层 symlink 不授权＋O_NOFOLLOW＋架构 cfg 门——复审 P1-1）；`terminal.rs` 三 spawn 点 pre_exec 接线（装挂失败永不 fail spawn——复审 P2⑦）；Docker Linux 容器跨目标 check 与 `child_write_guard` 测试 4/0 见 §9/§11 附记 |
| 0bw② 运行时载体完整性自检 | **✅ 落码（manifest 随下一载体重建批生成）** | `orz-host/src/carrier_integrity.rs`（清单加载/形态哨/逐文件 sha256/键按单 Normal 组件判定＋未列文件检测——复审 P2④/P3⑤）＋`orz-bin/main.rs` 启动接线（告警横幅审计腿，不拒启动）；父仓 `scripts/generate_carrier_manifest.py`（打包后调用，显式 LF）；8 单测 |
| 0bw③ 专用 journal 事件族（block/warn 结构化） | **✅ 落码全链（复审修复见 §13）** | `write_control_review` 事件族：schema（规则名封闭集入 allOf——复审 P2①）＋registry＋envelope 枚举＋Rust producer（空命令防线——复审 P2②；timeout 臂补收——复审 P1-2）＋Rust 判官族＋Python 镜像（冻结入口＋对拍名册；三处畸形输入对齐＋配对——复审）＋fixtures 7 件＋Rust/loop 两侧测试＋对拍语料两场景（256→258）；orz-assurance **278/0**、orz-loop **854/0** |
| 0bw④ git 检查点/journal undo（L4 余项） | **部分（undo 面✅；git 检查点维持设计登记）** | undo 面＝`orz-host/src/rollback_maintenance.rs`（list/restore；**restore 目标域收敛 cwd＋归一后 C1 判定＋check_write_target 第四消费点＋原子写回**——复审 P0-1/P2⑤；CLI 旗标/多余参数显式报错）＋`orz rollback` CLI 子命令＋write_face meta 侧车（目标路径随行）；**git 自动检查点的容器/触发/回收裁决依设计 §6 留用户**（D-6）；6 单测＋CLI 冒烟 |
| REV-083-05 集成测试目录（0bv 增量） | **✅** | `orz-loop/tests/run_turn_integration.rs`：公开 API（run_turn×LoopHost×FakeProvider）端到端两测（文本轮生命周期链序／工具轮 Started-Completed 对＋run_id 归因） |
| REV-083-12 panic 契约（D-12 路线） | **✅ 落字** | `orz/README.md` 新「Exit codes」节（0/1/2/101 四档契约＋panic=abort 下 101 不可达的如实声明）；`main.rs` join-Err 分支就地注记 |
| REV-083-14 判据钉纪律 | **✅（转录已在＋本批执行实例）** | 权威面＝ADR-0010 §14.80.2（0bv 落字批已转录，D-14 文本在档）；本批 0bw③ 即「首个适用机制定稿同步产 fixture（先红后绿）」的执行实例——schema/判官/镜像定稿与 fixtures（正例 3＋反例 2）同批产出 |
| REV-083-19 流程纪律 | **记录（随落账批执行）** | 落账批按 D-19：小步提交＋双会话「落账前 status 双确认」 |

**完成度一句话**：S4 登记的「REV-083-05/11/12/14/19 与 0bw①–④」九项全部处置——七项落码/落字闭合、0bw④ 完成 undo 半边（git 检查点留裁决）、0bw①/② 的真机读数与 manifest 生成随下一载体重建批；REV-083-21/22 修复落码，**切换矩阵 ③④ 重走与链首就绪承接采样待载体重建后补验**。

## §1 REV-083-21/22 修复（S4 两缺陷）

- **F-1 服务端**：根因＝`std::fs::canonicalize`（Windows 恒 `\\?\` verbatim）×`path_eq` 只归一分隔符/大小写。修复＝三层：① `path_eq` 归一前剥 verbatim/设备前缀（`\\?\UNC\host\share`→`\\host\share`）——纵深防御；② `canonicalize_usable` 统一 canonical 入口（**dunce::canonicalize**＝clippy.toml 合规形态），switch 与 `resolve_root` 两路共用，cwd 存储/展示不再带前缀；③ 四条单测（真实 canonicalize 形态判等【S4 实测 403 的直接回归钉】／三类前缀剥除＋负例／usable 无前缀／`resolve_root` 当前工作区自反）。
- **F-2 前端**：`sendJson(method, url, body?)` 扩展载荷形态（JSON＋Content-Type 头），`switchWorkspace` 改调用之（未定义符号 `sendJsonBody` 退役）。冒烟钉＝拦截 fetch 断言「恰一次 POST＋token query＋JSON 载荷含目标路径」——S4 记录的「函数存在断言抓不到」缺口就此封堵。
- **边界（如实）**：矩阵 ③「切后旧会话标旧工作区会话」④「新 cwd spawn 新会话」的真机到达须重建载体（0.7.4）后重走；「浏览器就绪后 web_search 链首承接」变体采样同轮补。

## §2 REV-083-11 SSRF pin（封 DNS rebinding TOCTOU）

- 原缺口：`check_ssrf` 校验后 reqwest 自行重解析，校验与连接之间可换地址（原注释自认 partial TOCTOU）。
- 修复形态：**每跳「自解析→校验→pin 到连接」**——`resolve_and_check` 返回校验通过的地址集，`build_pinned_client` 以 `resolve_to_addrs(host, addrs)` 构建 hop 专用客户端，连接只能落在已校验地址上；同宿主重定向逐跳重复（保留既有 per-hop 重检语义）。
- 连带：共享客户端（ArcSwapOption 缓存＋transport 失效重建）退役——每 fetch 独立连接池下，原「防连接池中毒」语义由构造天然承接；代理形态（proxy_endpoint）下目标 host 不在本地解析，pin 无效果但无害，SSRF 校验仍先行。
- 已知代价（如实）：每 fetch/hop 重建 TLS 配置（数 ms 级）；web_fetch 低量面（candidate cap 8/activation）可忽略。

## §3 0bw① Landlock（L3-Linux）

- 决策（设计 §3.3 决策点）：**直写 syscall**（nono `Sandbox` 是整进程启动期形态，pre_exec 场景不适用）；机制入 `orz-sandbox::child_write_guard`，策略单一源仍在 orz-tools `write_control::LINUX_SYSTEM_CORE`（0bw 表 B），由调用方枚举后传入——机制/策略分离，依赖方向不反转。
- 形态：`prepare_allow_dirs`（ABI 探测＋`/` 顶层枚举、排除表 B 与非目录项）→ spawn 点 `pre_exec` 装挂 ruleset（handled＝写族基线 +v2 REFER +v3 TRUNCATE；读族不入＝读不设限）→ `allow_dirs` 逐项 PATH_BENEATH allow；表 B 不授权＝默认拒（Landlock 无 deny 规则，allowlist 即 deny 表的补集）。
- **失败姿态（D-3）＝best-effort fail-open**：内核不支持（<5.13）/枚举失败 ⇒ 不装 pre_exec、`tracing::warn` 一次——L1 工具面/L2 命令面仍硬拒锁死面，L3 定位「附加阻力、非绝对保证」（设计 §1/§9），不因内核能力缺失瘫痪命令执行。
- 装挂点＝`terminal.rs` 三处 shell spawn（与 seccomp 网络过滤同点同型；命令执行唯一入口面）。

## §4 0bw② 载体完整性自检

- 生成器＝父仓 `scripts/generate_carrier_manifest.py`：扫描载体目录顶层常规文件（flat v1），写 `carrier-manifest.json`（kind 形态哨＋version＋entries{sha256,size}）；**随下一载体重建批在打包后调用**（涉发布链，本批不重建故 manifest 未生成——0.7.3 在役载体维持无清单形态＝NoManifest 静默）。
- 运行时＝`orz_host::carrier_integrity`＋`main_inner` 启动接线（只读、先于会话装配）：清单在位且全一致 ⇒ debug 日志；失配/缺失/清单键非法（穿越/绝对/根相对）⇒ stderr 告警横幅＋tracing；**失败姿态（D-4）＝审计腿不拒启动**——清单缺失不可作为错误（打包前开发树常态；能删清单的攻击者本就能改二进制，拒启动是超能力声明，违反三档措辞纪律）。

## §5 0bw③ `write_control_review` 事件族（全链）

- **契约面**：`runtime/write-control-review-event-payload-v0.2.schema.json`（tool/call_id/review/rule/detail/command_sha256/command_len；review 封闭三值；XOR＝allow⇒rule/detail 恒 null、warn/block⇒恒在且规则名封闭集）＋registry v02 登记＋envelope 枚举尾追加。
- **Producer**：`exec_policy::CommandReview::report()`＋`EnqueuedCommandReview`（sha256＋长度关联 tool_started 原文，命令不重复入账）→ bash 工具审查点入队（block 臂在 Err 返回**前**入队；队列槽位由 `get_or_default` 自动补建、入队不失败）→ 宿主 `collect_write_control_reviews`（Ok/Err/timeout 三臂各收一次；timeout 臂为 2026-09-27 复审 P1-2 补齐）→ `LoopHost::drain_write_control_reviews` → loop 工具边界（Ok/Err 两臂）与 run 收尾 drain 落事件。
- **判官**：Rust `verify_write_control_review`（XOR＋封闭集＋review↔rule 分类配对〔2026-09-27 复审 P2 增〕＋关联键形状；无覆盖率要求——best-effort 闸边界如实）＋ALL_FAMILIES＋dispatch＋Python 镜像 `_verify_v02_write_control_review`（逐条同形）＋冻结入口接线＋对拍名册同步。
- **判据钉（REV-083-14 纪律的执行实例）**：fixtures 六件（minimal block 正例／allow 正例／warn 正例／review 出集反例／allow-with-rule XOR 反例＋envelope 正例）随判官同批产出；schema 直测接受集＝正例 4 件全过、反例 2 件各 1 错（复审增补第 7 件 unknown-rule 反例见 §13）。
- **TUI**：`WriteControlReview` 并入宿主事实族 Unknown 降级臂（无专用卡片，与其余审计事实同姿态）。

## §6 0bw④ 回退窗口 undo 面

- `orz rollback list`：枚举 `.gsa/rollback/`（指针/字节数/毫秒时间戳/目标路径）。
- `orz rollback restore <pointer> [target]`：快照字节**原样**写回（BOM/行尾保真，0bm⑦ 存储语义）；目标缺省取 meta 侧车、显式传参覆盖；**覆写前自动把目标当前内容再存一格回退快照（undo 自身可逆）**；`.gsa` 域目标拒绝（C1 对齐）、指针穿越/绝对形态拒绝、目标目录缺失显式报错（不做超范围动作）。
- write_face：meta 侧车随快照写入（目标路径）并随保留窗口修剪同行——store 格式向后兼容（旧快照无 meta ⇒ restore 须显式给目标，list 如实标「无 meta」）。
- **git 自动检查点维持设计登记（D-6）**：容器/触发/回收（避免污染用户仓库历史）在设计 §6 显式留待用户裁决，本批不落码——0bw④ 不因此闭合，闭合随 git 半边的裁决与实施。

## §7 REV-083-05 集成测试目录

- `orz-loop/tests/run_turn_integration.rs`：run_agent_loop 主链首次获得 crate 外公开 API 视野的端到端测试（此前全部住在 lib 内部）。两测：①文本轮 `run_turn` 返回最终回答＋journal 链头无前像＋prompt→output→finished 全序；②工具轮 ToolStarted/ToolCompleted 对＋call_id 归因＋终态收尾。
- 测试宿主为公开 LoopHost 的最小实现（必需五件套）；env 免疫装配（`with_acaf_fail_closed(false)` 显式——ORZ-ENV-POLLUTION-001 纪律：狗粮启动 env 不入测试判定）。

## §8 REV-083-12／14／19

- **12**：`orz/README.md`「Exit codes」节（0＝成功含机械拒绝随结果面；1＝运行失败；2＝用法错误；101＝panic hop、仅 unwind 档可达——默认 dev/release 均 `panic="abort"`，panic 直接走平台 abort）。`main.rs` join-Err 分支就地注记。统一 panic hook 路线维持备档不采纳（D-12 延续）。
- **14**：ADR §14.80.2 已辖纪律文本；本批 0bw③ 为纪律的首个执行实例（定稿与 fixture 同批、先红后绿）。至此 REV-083-14 的「转录＋实例」两面齐。
- **19**：流程纪律随落账批执行（小步提交＋双会话 status 双确认）；本批不提交故此处仅登记。

## §9 验证与回归

| 面 | 读数 |
|---|---|
| orz-tools lib | **2972 passed / 0 failed / 6 ignored**（含 web_fetch 131、write_face、exec_policy report 新测；＋1＝复审空命令防线钉 `empty_command_skips_review_and_enqueue`） |
| orz-host lib（串行） | **353 / 0 / 5**（carrier_integrity 8、rollback_maintenance 6——含复审目标域/原子写/未列文件 6 钉；并行首跑 3 项负载敏感红（`run_tests_records_workspace_delta`/`call_tool_timeout_kills_process_tree`/`run_terminal_cmd_truncation_carries_output_object`＝083 审查已登记的既有负载敏感集），隔离复跑与串行全绿） |
| orz-loop lib | **854 / 0 / 3**（含 write_control_review 链路新测） |
| orz-loop 集成测试 | **2 / 0**（新目录首跑） |
| orz-assurance lib | **278 / 0**（含 write_control_review 判官单测＋Rust↔Python 对拍名册；对拍语料 256→**258** 场景——write_control_review_ok/_violations，该族 parity 自此非平凡） |
| orz-web lib ＋ 冒烟 | **47 / 0**（＋1＝`strip_verbatim` 大小写变体钉）；frontend smoke 全过（含 REV-083-22 真实 fetch 钉） |
| `orz rollback` CLI 冒烟（复审） | 无动词/未知旗标/多余参数 ⇒ exit 2 显式报错；空窗口 list ⇒ 显式报错 exit 1 |
| fmt | 本批改动文件 rustfmt 已整理（复审四文件重整）；全仓 `cargo fmt --check` 其余差异＝既有遗留（`orz-sampling-types/src/error.rs` 等 092 批前遗留＋本批未触碰件，按「fmt 仅整理本批改动文件」口径不动） |
| clippy（--lib） | 本批零新增；基线既有告警维持（`orz-host/permission.rs` unused imports＝092 批 18g 遗留；orz-assurance 4 条＝immediate_feedback/lif 既有，非本批） |
| Docker Linux 跨目标 | `rust:1.97-slim` 容器（ORZ-BUILD-MOUNT-001 挂载形态）：`cargo check -p orz-bin -p orz-sandbox --all-targets`＝**Finished（全绿）**；`cargo test -p orz-sandbox --lib child_write_guard`＝**4 passed / 0**（复审后——含 symlink 排除新钉；容器门再次拦截一处 Linux-only 签名错＝`Path` 限定缺失，Windows 侧不可见，当轮修复复跑全绿） |
| 父仓门禁 | `check_repository.py`：**error_count 1＝「orz submodule working tree is dirty」**（work batch 预期；落账提交后即绿）；fixture 映射（含复审 unknown-rule 反例）无 unmapped/missing |
| 源清单 | `orz_source_manifest.sha256` 清单覆盖 orz **已跟踪**文件（仓库字节口径）——门禁复验维持通过；**随 093 落账批（orz 提交后）再重生成一次**以纳入 4 个新文件 |

## §10 决策台账（D-*）

| 编号 | 决策 | 理由 |
|---|---|---|
| D-1 | 台账树面同步、随落账批提交；本批不提交/不推送 | 0bv S4 轮 D-1 惯例延续 |
| D-2 | SSRF pin 采用「每跳 pin 客户端」，共享客户端退役 | fail-closed 方向：pin 缺席＝构建错误而非静默回退；每 fetch 独立池天然承接防中毒语义 |
| D-3 | 0bw① L3 失败姿态＝fail-open＋warn 一次 | 设计 §1/§9「高阻力＋强审计、非绝对保证」；L1/L2 已硬拒；不因内核缺能力瘫痪命令执行 |
| D-4 | 0bw② 失败姿态＝告警审计腿、不拒启动 | 清单缺席是开发树常态；拒启动对「能改二进制的攻击者」无意义（其可删清单）＝超能力声明，违反三档措辞 |
| D-5 | 0bw③ 机制/策略分离：Landlock deny 表由调用方传入 | orz-tools→orz-sandbox 依赖方向不可反转；单一源保持 write_control |
| D-6 | 0bw④ 只落 undo 半边；git 自动检查点维持设计登记 | 设计 §6「容器/触发/回收须先裁定」——git 半边涉用户仓库历史污染面，留用户裁决 |
| D-7 | `write_control_review` 无覆盖率要求 | best-effort 闸（设计 §9）如实：无命令即无事件，判官只核形状/XOR/封闭集 |
| D-8 | 载体重建与矩阵 ③④ 重走不在本批 | 用户令「修复批」以落码为轴；重建涉 release 链与磁盘余量（本机 D: 已满载清理），留下一令 |
| D-9 | 0bw④ restore 目标域收敛 cwd＋归一后 C1 判定＋`check_write_target` 接入（deny 单一源第四消费点，设计档 §3.4） | undo CLI 是写路径且模型可达（L2 动词表不可达其命令文本）——回退写面必须与工具面同表同门（复审 P0 处置） |
| D-10 | Landlock 顶层 symlink 一律不授权＋`O_NOFOLLOW` 纵深 | `ln -s /etc /w` 两步旁路表 B；非核心 symlink 丢授权＝默认拒＝fail-closed 方向（复审 P1 处置） |
| D-11 | review↔rule 分类配对由判官核证（Rust/Python 双侧同形） | schema 无法表达跨字段映射；对拍语料补两场景使该族 parity 非平凡（复审 P2 处置） |
| D-12 | Landlock 装挂失败永不 fail spawn（`write(2)` 提示后照常 exec） | §3.3「不因 L3 故障瘫痪命令执行」的兑现；三分支全部 fail-open（复审 P2 处置） |
| D-13 | Landlock 架构 cfg 门＝linux ∧ (x86_64 ∨ aarch64)，其余恒不启用 | 统一 syscall 号仅此两架构成立；错号探测可能命中无关 syscall，不得尝试（复审 P2 处置） |
| D-14 | `canonicalize_usable` 不做二次剥除（dunce 保留 verbatim 即安全判定） | 长路径等形态剥除不安全；判等由 `path_eq` 两侧剥前缀承接（复审 P2 处置） |
| D-15 | 载体自检 tracing 腿改声明不改代码（stderr 横幅为有效腿） | subscriber 时序所限；就地如实注记优于为告警提前拉起 subscriber（复审 P3 处置） |

## §11 树面清单与结转

**orz 子仓（修改，未提交；24 改＋4 新；§12 复审后再改其中 13 件）**：`orz-web/{src/server.rs,Cargo.toml,assets/app/api.js,tests/frontend_smoke.mjs}`；`orz-tools …/web_fetch/{ssrf,http,client}.rs`、`computer/local/terminal.rs`、`grok_build/bash/mod.rs`、`types/{exec_policy,write_control}.rs`、`util/{mod,write_face}.rs`、`Cargo.toml`（复审：arc-swap 移除）；`orz-sandbox/src/{lib.rs,child_write_guard.rs（新）}`；`orz-host/src/{lib.rs,carrier_integrity.rs（新）,rollback_maintenance.rs（新）}`；`orz-loop/src/{controller.rs,host.rs,host_exec/{facts,tool_run}.rs}`＋`tests/run_turn_integration.rs`（新目录）；`orz-assurance/src/journal/{event,families}.rs`；`orz-tui/src/bridge.rs`；`orz-bin/src/main.rs`；`README.md`；`Cargo.lock`（dunce → orz-web）。**过 scour**：rustfmt 曾级联格式化 orz-host 全部子模块（处理 `lib.rs` 时波及），非本批 13 文件的纯格式 churn 已按「fmt 仅整理本批改动文件」口径 `git checkout --` 回滚，批次 diff 保持最小。
**父仓（未提交）**：本档＋[`写入管控设计`](../WRITE_CONTROL_MECHANICAL_DESIGN_2026-09-26.md)（§3.3/§3.4/§6/§9 复审增补）；`runtime/write-control-review-event-payload-v0.2.schema.json`（新，规则封闭集 allOf——复审）＋registry＋envelope 枚举＋fixtures **7 件**（新；含复审 unknown-rule 反例＋生成器同步的 fixtures README.md）；`scripts/generate_run_event_fixtures.py`＋`scripts/check_repository.py`（映射登记）＋`scripts/generate_carrier_manifest.py`（新，显式 LF）；`orz_source_manifest.sha256`（重生成复核，零 diff）；`assurance/run_event_journal_validation.py`（镜像＋复审配对/对齐）；`docs/BACKLOG_AND_PRIORITIES.md`＋`TODO.md`（S4 块＋本批块＋复审块）。

**结转（下一批，留用户令）**：
1. **落账批（093）**：台账（索引头行翻新＋BACKLOG/TODO 本批块）＋小步提交＋推送（D-19 纪律）；提交后重生成源清单（纳入 orz 4 个新文件）。
2. **载体重建批（0.7.4）**：打包后调 `generate_carrier_manifest.py` 落 manifest（0bw② 首个有清单载体）→ 换装 → **重走切换矩阵补 ③④ 面**＋`orz rollback` 真机面＋Linux 载体上 Landlock 实装读数（本批容器 check 为最低验证）。
3. **0bw⑤ CFA enforce 翻转**：仍待 1124 读数（维持不翻）。
4. **0bq 闭合翻转**：仍待用户裁决（58→57，宜与 0bw 同批）。

### 附记：Docker Linux 跨目标读数（实读）

- 容器＝`rust:1.97-slim`＋protobuf-compiler（ORZ-BUILD-MOUNT-001 挂载形态：父仓 → `/orz`，工作目录 `/orz/orz`）。
- `cargo check -p orz-bin -p orz-sandbox --all-targets`：**Finished（全绿）**——`orz-bin` 全链（含 `terminal.rs` Landlock 三 spawn 接线、orz-tools/loop/host/web/tui）在 Linux 目标编过；`child_write_guard` 的 Linux-only 测试目标一并编译。
- `cargo test -p orz-sandbox --lib child_write_guard`：**3 passed / 0 failed**——Linux 真环境跑通（ABI 探测＋`/` 顶层枚举排除表 B＋可写基线断言）。
- 过程留痕：首跑 E0308（`CString::to_str` 返回 `Result` 非 `Option`——Windows 侧不可见的 Linux-only 测试错误，恰证跨目标门的必要性）当轮修复复跑全绿。
- 边界（如实）：这是 **check/单测级验证**；Landlock 装挂的 exec 后真机读数（子进程写 `/etc` 被内核拒）仍随 Linux 载体/狗粮批收取。

## §12 复审与修复（2026-09-27 同日；用户令「由你进行裁决，补足设计面缺口，并对审查出的全部问题进行处理」）

四路并行只读深查（Landlock/命令面、事件族全链、SSRF/载体自检、web 修复/集成测试/rollback）＋主会话亲核与实测（门禁、判官单测、集成测试、orz-web 46/0、rollback_maintenance 全文、timeout 臂、枚举段）。**裁决与处置全表**：

| 编号 | 级别 | 发现 | 裁决／处置 |
|---|---|---|---|
| R-1 | **P0** | 0bw④ restore 目标无域约束：绝对/`..` 越界直写任意路径、`./.gsa/…` 与 `.GSA` 变体绕过 C1、模型可经命令面调用且 L2 动词表不可达——deny 单一源外第四条无管控写路径，「C1 对齐」声明被架空 | **D-9：目标域收敛**——归一化（反斜杠归一＋词法组件，`..` 越界/绝对/盘符恒拒）后须在 cwd 内；C1 归一后判（大小写不敏感）；**接 `check_write_target`＝deny 单一源第四消费点**（设计档新增 §3.4）；写回原子化（临时件＋rename）；6 新单测＋CLI 冒烟 |
| R-2 | **P1** | Landlock 顶层 symlink 跟随（`metadata` 判目录＋`O_PATH` 解析）＝root 载体下 `ln -s /etc /w` 两步旁路表 B | **D-10：顶层 symlink 一律不授权**（`DirEntry::file_type` 不跟随，symlink 项直接跳过＝fail-closed）＋`O_NOFOLLOW` 打开纵深（枚举后换链 ⇒ 打不开 ⇒ 默认拒）；merged-usr 四件本被表 B 名字排除不受影响；新枚举测试（临时根注入 symlink 用例） |
| R-3 | **P1** | timeout 臂不收审查事件（Ok/Err 两臂 collect、`started_exec` 杀树臂直接 return）⇒ 批档「三臂各收」失真、超时（命令面最长路径）warn 留痕可丢 | **修复**：timeout 臂杀树后补 `collect_write_control_reviews()`——三臂纪律自此与代码一致 |
| R-4 | **P2** | schema 不锁规则名封闭集（warn/block 臂仅 minLength:1，未知名过 schema 直测） | **修复**：allOf then 臂加五规则 `enum`（判官/schema 双锁）＋第 7 件 fixture `unknown-rule.constraint.invalid`（恰 1 错）＋生成器/门禁双登记 |
| R-5 | **P2** | 空命令可达审查点 ⇒ 产出 `command_len:0` 违例事件（判官红自家产出；exec_policy 注释无机制支撑） | **修复**：bash 审查点空命令防线——空/纯空白不审查不入队（D-7「无命令即无事件」机械化）＋`empty_command_skips_review_and_enqueue` 正反双钉 |
| R-6 | **P2** | review↔rule 分类配对无任何层面校验（`warn+safety-mechanism-flip` 双侧判官均过） | **D-11：配对入判官**——Rust/Python 镜像各加分类映射核证（schema 无法表达跨字段映射，判官独任）；判官单测＋对拍语料两场景（256→258，该族 parity 自此非平凡） |
| R-7 | **P2** | Rust↔Python 镜像畸形输入三处不一致：① allow 行 rule=非串非 null（Rust 净/Python 违例）② rule=`""` 流程分叉 ③ `command_len` >u64 溢出整形 Python 放行；另 Python 非 dict payload 会 AttributeError | **对齐**：① allow 臂改按 `Value::Null` 判（非串类型同样违例）② carry-and-continue 仅限 None/非串，空串落封闭集臂 ③ Python 补 u64 上限 ④ 非 dict payload 报 review 形状错不崩（与 Rust 同形）——verdict 与错误条数双对齐 |
| R-8 | **P2** | Landlock 子进程装挂失败 ⇒ spawn 硬失败（设计未登记的第三分支，与 §3.3「不因 L3 瘫痪」相悖） | **D-12：装挂永不 fail spawn**——`install_best_effort`：失败 ⇒ `write(2)` 直写 stderr 一行提示（async-signal-safe，pre_exec 窗口唯一可行可见面）后照常 exec；三 fail-open 分支收敛入设计档 |
| R-9 | **P2** | syscall 号 444–446 仅 x86_64/aarch64 成立，其余架构错号探测可能命中无关 syscall | **D-13：架构 cfg 门**——`prepare/kernel_abi/install` 仅 `linux ∧ (x86_64 ∨ aarch64)` 编真身，其余恒 None/stub（fail-open 方向）；设计档 §3.3/§9 登记 |
| R-10 | **P2** | ABI v1 内核 rename/link 不受约束（REFER v2 才有） | **登记**（内核明载能力缺口，非实现缺陷）：设计档 §9 |
| R-11 | **P2** | carrier 清单键盘符相对形（`C:foo`）三项检查全放行、与代码自述不符；`rel.contains("..")` 子串判定误杀 `a..b.txt` | **修复**：键校验收敛为「单个 Normal 组件」判定（盘符＝Prefix+Normal 两组件被拒；`a..b.txt` 单组件放行）＋`C:foo`（cfg windows）与 `a..b.txt` 两钉 |
| R-12 | **P2** | restore 写回非原子（truncate-in-place，中断留截断目标） | **修复**：同目录临时文件＋rename（`MOVEFILE_REPLACE_EXISTING`），失败清理临时件；无残留钉 |
| R-13 | **P2** | `canonicalize_usable` dunce 之后无条件 strip 击穿 dunce 对长路径的安全保留 | **D-14：去二次剥除**——dunce 返回形态即最终形态（保留 verbatim＝安全判定）；判等由 `path_eq` 两侧剥前缀承接 |
| R-14 | **P3** | `strip_verbatim` 前缀匹配大小写敏感（`\\?\unc\` 手写变体不剥） | **修复**：ASCII 大小写不敏感匹配＋变体钉（orz-web 47/0） |
| R-15 | **P3** | carrier 未列文件不告警（清单外新增不可检） | **修复**：顶层清单外常规文件 ⇒ findings（`未列文件`）；生成器显式 LF（仓内 manifest LF 先例）；坏清单 ⇒ NoManifest 静默的退化面入设计档 §6 |
| R-16 | **P3** | `run_rollback` 未知旗标静默过滤、多余位置参数静默忽略 | **修复**：显式报错 exit 2（CLI 冒烟三例） |
| R-17 | **P3** | `arc-swap` 依赖残留（共享客户端退役未清） | **修复**：orz-tools Cargo.toml 移除（workspace 声明留作他 crate 在用） |
| R-18 | **P3** | 启动自检 tracing 腿恒 no-op（subscriber 时序） | **D-15：改声明不改代码**——stderr 横幅为有效腿，main.rs 就地如实注记；批档措辞同步 |
| R-19 | **P3** | 批档 §5 四处措辞强于/偏离代码（三臂、队列不可得只 warn、5 正例、tracing） | **修正**：三臂随 R-3 成真；「get_or_default 自动补建」如实；正例 4 件口径；tracing 随 R-18 |
| R-20 | **P3** | 生成器 docstring「55 events」过期；fixtures README 未含新族口径 | **修正**：56 事件口径（第 7 件 fixture 落盘时 README 由生成器同步） |
| R-21 | **P3** | 「必需五件套」口径（trait 实为 2 必需＋3 行为覆写）；对拍语料该族无场景；controller.rs +226 约九成 rustfmt 噪声；warn-once＝每 spawn 点一次；per-spawn 重复枚举；`--build-info` 先于自检；「candidate cap 8」为检索面既有配置非 web_fetch 本地常量 | **裁决不采纳/已覆盖**：五件套措辞随 §7 不改（实现正确）；语料随 R-6 补齐；controller fmt 属本批改动文件整理口径维持；其余登记设计档 §9/本节（均为如实注记，无行为改动） |

**设计档同步**：`WRITE_CONTROL_MECHANICAL_DESIGN_2026-09-26.md` 新增 §3.4（undo 面第四消费点）、§3.3 复审增补（symlink/装挂三分支/架构门）、§6 载体自检边界、§9 四组复审边界。
**门禁读数**：orz-tools **2972/0**、orz-host 串行 **353/0**、orz-loop **854/0**＋集成 **2/0**、orz-assurance **278/0**（对拍 258 场景）、orz-web **47/0**＋冒烟过、Docker Linux check 全绿＋child_write_guard **4/0**、父仓门禁仅余「orz 子仓未提交」预期项。

## §13 关联与关键词

[`0BV S4 真机复验`](0BV_S4_LIVE_VERIFICATION_2026-09-27.md) ／
[`0BV 结转报告`](0BV_CARRYOVER_REV083_0BW_2026-09-27.md) ／
[`写入管控设计`](../WRITE_CONTROL_MECHANICAL_DESIGN_2026-09-26.md) ／
[`083 全面审查档`](083_FULL_PROJECT_REVIEW_2026-09-26.md) ／
[`092 落账档`](092_SUBMIT_PUSH_2026-09-27.md)
BACKLOG `0bv`／`0bq`／`0bw`

关键词：过夜批、REV-083-21 verbatim 剥除、REV-083-22 sendJson 载荷、SSRF pin、
resolve_and_check、build_pinned_client、Landlock child_write_guard、表 B 单一源、
载体完整性自检、carrier-manifest.json、write_control_review 事件族、XOR 判官、
回退窗口 undo、orz rollback、集成测试目录、退出码契约、判据钉执行实例、
复审修复、restore 目标域收敛、deny 单一源第四消费点、顶层 symlink 不授权、
规则封闭集双锁、review↔rule 配对判官。
