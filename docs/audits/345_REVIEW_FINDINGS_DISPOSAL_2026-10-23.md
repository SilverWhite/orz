# 345 批：344 审查发现全量处置（2026-10-23）

> **用户令**：「请对审查出的全部内容进行处理，可拉起子代理协助」——对象为 344 批交付物（父仓 `85c60810` ＋ orz 子仓 `ccaa2017`）的三面复审（设计／实现／符合性）所列全部发现。复审方法：五子代理分区对账＋主会话关键项双侧复核＋四门测试独立重跑；复审报告全录于会话，其可处置项全部落体于本档。
> **数据纪律**：代码触点由主会话亲落（包含门、锁面）；机械性注释勘误由子代理执行、主会话测试门验证；ADR 触点主会话亲落。344 审计档为历史记录不回改，账目勘误全部登记于本档 §3.3。

## §0 结论

- 344 批交付物复审结论：声明高度可信（八项代码触点、四同步面、五测试改写、四门测试数字全部逐点对上；fixture 重跑零差异复现），处置意见为「收口未闭环」三处（失效标注登记式未落地／包含门词法回落／非 Windows 臂零覆盖）＋账目瑕疵四处。本批将上述全部发现处置完毕。
- 本批新增代码行为面两处：包含门近祖先 canonical 解析（闭预存链接目录穿越面）；毒锁 `warn` 留痕＋混式锁收口。其余为注释/文档/CI/测试面。

## §1 复审发现 → 处置映射

| # | 复审发现（级别） | 处置 |
| --- | --- | --- |
| 1 | 失效标注「登记式而非落地式」：vol-03 两处 120、vol-02 主 UI、§14.72-11 十工具列举被标注文件零改动（P1） | 就地落地：vol-03 四处（§3.3 条 7／§3.4 条 6＋同扫出 §3.5 条 4／§3.7 条 7 两处残留）、vol-02 §2.4 条 8、vol-14 §14.72 条 11 各追加失效标注指针（条款正文一字不动）；§14.89 条 4 重写为落地状态＋勘正两处小节号（§3.3／§2.4 条 8）；入口页补记 v1.95 |
| 2 | search_replace 新建/解析失败目标臂词法回落＝预存 symlink/junction 目录穿越面；`best_effort_canonical` 现成未用（P2） | `util/fs.rs` 新增异步 `best_effort_canonical_with_timeout`（镜像同步版语义：canonicalize 失败即上溯存在祖先）；包含门改用之；新增 junction/symlink 目录回归测试（拒绝＋不落盘断言） |
| 3 | 非 Windows 臂全自动化零编译零执行（CI 仅 ubuntu fmt），未登记残余（P2） | orz workflow 新增 `linux-tests` job：assurance/loop/tools 三门必阻断＋orz-host 串行观察模式（`continue-on-error`，Linux 姿态未经验证，首轮读数后转必阻断，见 §5.2） |
| 4 | 344 档账目四处出入：556/557 两记、94 份审计实测 84、五字段实为四字段、「证据零触碰」一硬反例（P2） | 本档 §3.3 逐条勘误登记（344 档不回改）；proxy-fail.log 失登记见 §3.3 条 4；索引头部 557 表述同步勘正 |
| 5 | 毒锁容忍静默吞毒缺留痕＋三簇混式裸锁（console 双锁／`board_stamp_pin`／`epoch_archive_errors`×4）（P3） | controller.rs 新增 `poison_proceed` 助手（warn 留痕＋into_inner）；5 处裸锁收口经助手；344 三处恢复面锁改经助手补留痕 |
| 6 | orz-host 五处「默认 yolo」文档注释与新姿态矛盾（acp_server.rs build_host 最误导）＋测试 5 两段旧注释、一处 vacuous 断言（`call-act-1` 永不匹配）（P3） | 五处注释按 §14.90 双姿态改写；测试 5 注释与断言同改（vacuous 串考古后改真实 `call-act-ro` 负断言＋`call-act-ww` 精确完成锚正断言） |
| 7 | encoded-exec 覆盖残余（误报族：引号散文、`grep -ec`；漏报族：iex、`python -c`、`|&` 等）宜登记（P3，warn 级可接受） | `exec_policy.rs` WARN_RULES doc 后追加「已知残余（warn 级，登记待 block 化读数）」注释；逻辑扩展随 block 化批处理（复审建议原样保留） |

## §2 代码触点（orz 子仓）

| # | 触点 | 内容 |
| --- | --- | --- |
| 1 | `orz-tools/src/util/fs.rs` | 新增 `best_effort_canonical_with_timeout`：目标不存在或 canonicalize 失败时上溯至首个可解析祖先 canonical 解析、余段词法拼接（`write_control::best_effort_canonical` 的异步兄弟，走本模块 try_canonicalize 包装） |
| 2 | `orz-tools/.../search_replace/mod.rs` | 包含门目标判定由词法 `path` 改经近祖先 canonical（344 版「canonicalize 失败回落词法」的穿越面闭合）；新增回归测试 `search_replace_resolves_preexisting_link_dirs_for_containment`（Windows junction／Unix symlink 目录→区外目标必拒且不落盘） |
| 3 | `orz-loop/src/controller.rs` | 新增 `poison_proceed(what, err)` 毒锁助手（`warn` 留痕＋into_inner）；344 三处恢复面锁（`restore_temporal_spikes`／`rli_complexity_reading`／`restore_lif_session`）改经助手补留痕；三簇混式裸锁收口：`epoch_archive_errors`×4（try_with_plan／apply_structured_plan）、`console_traces`（console_direct_begin）、`console_mode_state`（console_direct_end）、`board_stamp_pin`（effective_blackboard_stamp） |
| 4 | `orz-host/src/permission.rs`／`lib.rs`（×2）／`acp_server.rs`（build_host）／`approval.rs` | 五处「默认 yolo」文档注释按 §14.90 双姿态改写（Windows 交互面 confirm-first／headless 与非 Windows 维持 yolo），原 2026-09-26 用户令效力范围显式限定 |
| 5 | `orz-host/src/codex_app.rs` | 测试 `read_only_and_workspace_write_threads_coexist` 两段旧注释改与 `cfg!(windows)` 双臂一致；vacuous 串 `call-act-1`（`git log -S` 考古：原 `ord-000001`，id 重命名后空转）改真实 `call-act-ro` 负断言（deny 路径不得产生完成事件）＋`call-act-ww` 精确完成锚正断言 |
| 6 | `orz-tools/src/types/exec_policy.rs` | WARN_RULES doc 后追加已知残余登记注释（误报族：引号散文／`grep -ec`；漏报族：iex／`python -c`／`|&`／`-command -`／pwsh 旗缩写 4 形态），block 化批处理时一并收敛 |
| 7 | `.github/workflows/rustfmt.yml` | 新增 `linux-tests` job（同 pin 1.97.1＋protobuf-compiler＋rust-cache）：orz-assurance／orz-loop／orz-tools 三门必阻断；orz-host 串行观察模式（continue-on-error） |

## §3 父仓触点

### 3.1 ADR（失效标注落地＋清单勘正）

| 触点 | 内容 |
| --- | --- |
| `adr/ADR-0010-vol-03-isomorphic-agents.md` | 四处就地失效标注（仅追加标注指针、正文不动）：§3.3 条 7／§3.4 条 6（→§14.55 条 2 取代，默认无限制）；§3.5 条 4／§3.7 条 7（344 清单未覆盖的同语境 120 残留，本批同扫同标） |
| `adr/ADR-0010-vol-02-core-decisions.md` | §2.4 条 8「主 UI 为 `orz-tui`」就地失效标注（→§14.78 Web 优先／0dk 稿） |
| `adr/ADR-0010-vol-14-addenda-index.md` | §14.72 条 11 十工具列举就地补指针（→§14.89 现行净 12）；§14.89 条 4 重写：集中登记→就地落地状态、勘正两处小节号（「§3.4 两处」实为 §3.3 条 7＋§3.4 条 6；「§2.4.8」实为 §2.4 条 8）、补登两处新增标注；节标题记 v1.95 |
| `adr/ADR-0010-fusion-runtime-and-agent-architecture.md` | 入口页补记追加 v1.95（345 批处置摘要） |

### 3.2 CI

orz workflow 增 `linux-tests`（见 §2 条 7）——闭复审发现 3 的覆盖缺口；orz-host 观察模式为有意分级（对应 §14.90「先留痕后收紧」同款灰度纪律），转必阻断条件见 §5.2。

### 3.3 344 档账目勘误登记（344 档不回改，以此为准）

1. **tmp 清理账目**：344 档 §0 记「556 项」、§3.5 与提交信息记「557 项」（557+206=763≠762 总数自相矛盾）。tmp 全部未跟踪、删除无 manifest，事后无法定谳；按自洽口径（762−206=556）采 **556**，557 记为待疑笔误。唯一可复验事实＝存活证据 206 项（本批复验在盘）。
2. **「94 份审计引用 .tmp-」**：实测 docs/audits 内 **84** 份；docs＋adr 全量 96 份。94 为无出处中间口径，以 84（docs/audits 口径）为准。
3. **「run_finished 落后 0dm 五字段」**：实差 **四字段**（`terminal_source`／`terminal_nudged`／`terminal_nudge_count`／`self_check_called`；第 5 个 diff 行为 `tool_rounds` 补逗号，非新字段）。
4. **「被审计引用的证据面零触碰」有一硬反例**：`.tmp-062-linux-build.proxy-fail.log`（062 档 ：69 明记「留档」、:161「首轮失败原样」、:167 取证件）已于 344 卫生批删除，全仓不存在。边缘项 `.tmp-0bd-hostfull-par{,2}.log`（0BD 档取证件）同失。既无法恢复，登记为失效：062 档 F-062-1 的证据面以 062 档文字记载与 `.tmp-062-linux-build.log`（在盘）为准；后续卫生批删除前须对「档内明记留档」条目做白名单复核。
5. 344 档 §2.2「watchdog 路径的毒锁容忍式」表述易读作 343 引入——实际容忍式早于 343（2026-08-28 `6cc8586c` 即有），343 修的是数值中毒非锁中毒；344 修复方向不受影响。

## §4 验证

| 门 | 结果 |
| --- | --- |
| `cargo check -p orz-loop --lib`／`-p orz-tools --lib` | ✅ 干净（编辑后即检） |
| 新增包含门回归测试（junction 臂） | ✅ 2 passed（含 344 双臂原测试） |
| `cargo test -p orz-tools --lib` | ✅ 3046 passed / 0 failed / 6 ignored |
| `cargo test -p orz-loop --lib` | ✅ 908 passed / 0 failed / 3 ignored |
| `cargo test -p orz-host --lib`（串行） | ✅ 431 passed / 0 failed / 6 ignored（含测试 5 断言修复后的新锚） |
| `cargo test -p orz-assurance --lib` | ✅ 306 passed / 0 failed |
| `git diff --check`（两仓） | ✅ 无空白错误 |

> 复审阶段基线（处置前）四门独立重跑：3045／306／908／431 全绿，与 344 档 §4 数字一致——处置改动于此基线上净增 1 测试（包含门 junction 臂）。

## §5 残余与后续批

1. **encoded-exec 逻辑收敛**（引号感知、解释器解释面覆盖）——按复审建议随 block 化批处理（exec_policy.rs 注释已登记残余清单）。
2. **orz-host Linux 观察模式转必阻断**：待 `linux-tests` 首轮 CI 读数（进程树击杀／超时覆盖／输出截断／会话卷 symlink 五测试的 Linux 姿态未知）；转绿后移除 `continue-on-error`。
3. **毒锁助手全量统一**：controller.rs 其余 ~70 处闭包式容忍（`|e| e.into_inner()`）与 user_authored_inputs 三处路径式未动（行为相同、缺留痕）——机械清扫面大、发版前不重入主循环，登记 0do S2+ 与巨石拆分同批顺手处理。
4. **卫生批白名单纪律**：删除 tmp 前对「档内明记留档」条目做引用复核（§3.3 条 4 教训）；删除清单留 manifest 供事后对账。
5. **发版**：本批修复（包含门补口／锁留痕／CI）随 0.9.12 载体重建进体（344 修复同车）。
