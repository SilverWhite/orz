# 全仓未处理摩擦项盘点（只读盘点，零代码零账本改动）

> 日期：2026-09-17；触发＝用户两项指示：①「LIF 组件暂时不做，留给狗粮线」（0am 线整体挂起）；②「统计目前暂存未被处理的全部摩擦项，不管该摩擦项是否有 journal 证据，先都整理出来看看」。
> 性质＝**盘点档**：只收登记在案的摩擦/挂账项并归类，不新立案、不动代码、不动 BACKLOG/TODO/索引（账本路由与逐项处置全部留用户裁决）。不提交不推送（工作树状态随收口批口径）。
> 计数速览：**未处理摩擦/挂账 36 条**（A 类 14 ＋ B 类 6 ＋ C 类 8 ＋ D 类 4 ＋ E 类 4）＋ 注记级 1 条；另有 **F 类**（摩擦衍生、已立项未闭环）7 项与 **G 类**（已有归属的移交/裁决面）4 条单列引用，**H 类**（已处置/裁决不处理）为排除面。
> **2026-09-17 判定批注（§11，用户裁决）**：不处理 4（FR-A01/A02/A05/A09）／**FR-C05 已核实销项闭合、FR-C04 采②已落码（未提交）**／判定需要处理 30 条＋注记级 1（实施与排期待另行放行）。

---

## 1. 纳入/剔除口径与方法边界

- **纳入**：2026-09-07 以来各狗粮 run 报告、深审/审查报告、装置侧审计中登记为摩擦/缺陷/挂账、且当前无「已闭合／已立项（转 F 类）／用户裁决不处理（转 H 类）」终态的条目。**不论是否有 journal 证据**（按用户口径，环境类与观察类一并收录，逐条标注证据面）。
- **剔除（H 类，见 §8）**：已有处置终态者——已闭合（如 0af、GAP-ORZ-HOST-RESOURCE-SNAPSHOT-DROP）、已并入设计并由后续批次承载（如 0ac 深审摩擦 A）、用户明示不处理（如虚构「协调者指令」）、随裁决消解（如 W2 D-1）。
- **方法**：`grep 摩擦` 扫 `docs/audits/` 全部命中件（30 份）；通读三份狗粮报告（0ai 拆分报告 §7、0am 报告 §5/§6、0am 收口报告 §6/§7）；通读 [0AC_S3B_RUN_DEEP_REVIEW](0AC_S3B_RUN_DEEP_REVIEW_2026-09-15.md) §5/§8 与 [QUAD_BATCH_DEEP_REVIEW](QUAD_BATCH_DEEP_REVIEW_2026-09-15.md) §6/§7/§10；装置侧取 [TB21_V41_ROUND0_MEMORY_HEAVY_START](TB21_V41_ROUND0_MEMORY_HEAVY_START_2026-09-13.md)、[FRAMEWORK_TIME_BUDGET_SEMANTICS_AUDIT](FRAMEWORK_TIME_BUDGET_SEMANTICS_AUDIT_2026-09-13.md)、[TB21_V41_EVAL_CONTAINER_AUDIT](TB21_V41_EVAL_CONTAINER_AUDIT_2026-09-13.md)、[054_CARRIER_REBUILD](054_CARRIER_REBUILD_2026-09-16.md)；W2 线取 [W2_ORZ_DEFECT_EXTRACTION](W2_ORZ_DEFECT_EXTRACTION_2026-09-07.md)。立项状态逐条对照 [BACKLOG](../BACKLOG_AND_PRIORITIES.md)／[TODO](../../TODO.md)／[索引](../../CLI_PROJECT_INDEX.md)。
- **边界（如实）**：①2026-09-07 之前的历史审计未逐一重扫——其发现当时已按 BACKLOG 流程处置或被后续线取代；②本盘点只收**登记在案**项，未对 orz 源码做新增扫描、不新立案；③W2 线 D-3 所指权限门 deny 事件面形态在 0p/0q 落码后**未复核**，按登记原文收录并标注；④0ai F4（台账无摘要）在 v8 收口清理后台账族大改，**形态是否仍成立未复核**，按登记原文收录并标注。

---

## 2. A 类：框架/环境类摩擦（14 条；原始登记未立项，与 0am 挂起无关、恒有效）

| ID | 摩擦 | 来源（复发） | 证据面 | 建议去向（登记原文） |
|---|---|---|---|---|
| FR-A01 | 沙箱缺 `protoc`，clippy/冷启动 test 需显式 `PROTOC=D:\CLI\orz\bin\protoc.exe` 否则构建失败 | [0ai 报告 F1](0AI_HOST_EXEC_SPLIT_REPORT_2026-09-16.md)＝[0am 报告 F1](0AM_DOGFOOD_2026-09-17.md)，**复发 ≥2** | journal 无（构建面） | CLI 启动面注入子进程 env，或 README 首行提示 |
| FR-A02 | 终端中文乱码：GBK 代码页渲染 UTF-8 为 mojibake；`>` 重定向落盘 UTF-16 致解码错 | [0ai F3](0AI_HOST_EXEC_SPLIT_REPORT_2026-09-16.md)＝[0am F2](0AM_DOGFOOD_2026-09-17.md)，**复发 ≥2** | journal 无 | 沙箱 shell 统一 `chcp 65001`，或文档注明 `-Encoding UTF8` |
| FR-A03 | 工具结果序列化失败 ×2（`invalid type: null, expected a string`）：`search_replace` old==new 拒绝面、批量 `replace_all` 结果流；操作语义正常但模型面出现孤立错误行 | [0am F3](0AM_DOGFOOD_2026-09-17.md) | **journal 可检索 1 处**；计数口径模型面 2 处/journal 1 处不一致（收口报告 M-3） | 排查 `search_replace` 错误结果类型面的 null 字段；排查时两侧并查 |
| FR-A04 | 陈旧 TodoWrite 列表跨会话残留（与当期任务无关的 todo 出现在早期窗口），只读 lane 无清理/替换面 | [0am F4](0AM_DOGFOOD_2026-09-17.md) | journal 无 | 会话启动时标注跨会话 todo 来源或清空；只读 lane 给清空面 |
| FR-A05 | 宿主页面文件/内存限制：全量 `cargo check --workspace --all-targets` 默认并行度 StackOverflow（`0xC00000FD`），`-j 4` 改报页面文件太小（os error 1455）；分面 check 全过 | [0am F6](0AM_DOGFOOD_2026-09-17.md) | journal 无（构建面） | 狗粮机加大页面文件；或约定全量 check 按 `-j N` 分批 |
| FR-A06 | 压缩/窗口交互：工具密集回合模型漏读压缩提示 ≥3 次；硬提醒面「再不给 ⇒ 不兜底」依赖模型补交 `SEMANTIC_SUMMARY` | [0am F7](0AM_DOGFOOD_2026-09-17.md) | journal 有压缩事件在案（行为观察） | 摘要产出做成动作面（`compress(blocks,summary)` 式），或每条工具结果尾部轻携「待压块」徽标 |
| FR-A07 | 折叠台账（`.gsa/ledger/current.md`）仅存轮号＋工具名＋sha256、无内容摘要，不可按行检索 | [0ai F4](0AI_HOST_EXEC_SPLIT_REPORT_2026-09-16.md) | journal 无（落盘台账面） | 折叠行附 ≤80 字符命令/结果摘要。**注：v8 收口清理后台账族大改，形态是否仍成立待复核** |
| FR-A08 | TER 旧文断链 ×7（`TER_*` 引用未随仓的 `../../.ter_review_2026-09-04/`），门禁常驻 7 条错误淹没真实错误 | [0ai F6](0AI_HOST_EXEC_SPLIT_REPORT_2026-09-16.md) | 门禁报错可复现（常驻） | 相关旧文入 `存档/` 或改写链接 |
| FR-A09 | 沙箱终端工具面：无 `grep/head/tail/sed/find`、不支持 `&&`，每次绕行 `Select-String`/Python/`;` | [0ai F7](0AI_HOST_EXEC_SPLIT_REPORT_2026-09-16.md) | journal 无 | runner/sandbox 侧一次性补齐或文档固化绕行口径 |
| FR-A10 | orz-host 并行测试抖动：进程树/Job/卷符号链接用例并行负载下失败数漂移（321/4 ↔ 317/8），须 `--test-threads=1` 才可复现；多出 4 条未逐一复验 | [0ai F8](0AI_HOST_EXEC_SPLIT_REPORT_2026-09-16.md) | 实测读数在案 | 因果归因复核；套件读数固定串行口径 |
| FR-A11 | `orz-host` `grok_home` 4 条既有失败 | [0ai 收口 §4](0AI_DOGFOOD_ASSESSMENT_CLOSURE_2026-09-16.md) | 实测读数在案 | **独立立项候选，留裁决** |
| FR-A12 | 语料口径漂移：`jobs-official` 已滚动至 134 runs（含 final-smoke/arm-dryrun 等追加），与「102-run 语料」立项口径不同步 | [0am F10](0AM_DOGFOOD_2026-09-17.md) | 回放器实测（自适应发现，无需改动） | 文档口径注明「滚动集、以实测 runs 计数为准」（轻量） |
| FR-A13 | WIP 跨会话交接无状态行：中断留未提交 WIP 时后续会话需额外侦察（0am 实证 294 行 WIP＋2 编译错） | [0am F11](0AM_DOGFOOD_2026-09-17.md)；归因已由收口报告 M-2 更正 | 时间线取证在案 | 跨会话中断时在 `.tmp-`/报告注记留「WIP 状态行」（流程类） |
| FR-A14 | 狗粮启动器未脚本化：env＋信任＋载体路径＋显式 cwd 断言未一次装配；0am M-1 误启动（以 orz 子模块为 cwd 起跑后终止）为此付出 4.5 分钟活跃工作与 WIP 残留代价 | [0ai 收尾建议](0AI_DOGFOOD_ASSESSMENT_CLOSURE_2026-09-16.md)＋[0am 收口 M-1](0AM_DOGFOOD_CLOSURE_2026-09-17.md)，**两次坐实** | M-1 误启动 run 时间线取证在案 | 狗粮启动器脚本化（含启动前 `pwd` 断言入 checklist） |

---

## 3. B 类：0am/LIF 线随挂起项（6 条；按用户 2026-09-17 裁决「LIF 组件暂时不做，留给狗粮线」整体挂起）

登记原文见 [0am 报告 §5-F5/F12、§6](0AM_DOGFOOD_2026-09-17.md)；判据/裁决语境见 [RLI 基座设计](../RLI_RESONANT_CHANNEL_BASE_DESIGN_2026-09-16.md)、[BACKLOG 0am](../BACKLOG_AND_PRIORITIES.md)、ADR-0010 §14.71。

| ID | 项 | 内容摘要 |
|---|---|---|
| FR-B01 | 0am F5 节律容差并集 | 容差 `π/(4ω_d)` 下 k 匹配窗并集恰好无空隙 ⇒ a4 字面实现退化为「非突发间隔计数」（实测 \|Δr\| ≈0.02），C1「a4 必须崩塌」在该容差下不可判；翻转批建议收紧至 `π/(16ω_d)` 或改相对偏差判据（诊断开关 `set_rhythm_tolerance` 已备） |
| FR-B02 | 0am F12 快照定点化参数面边界 | ω 量级 ~1e-2，3 位小数存储引入 ~0.3% 周期漂移 ⇒ 本批改「状态面定点＋参数面（ω/ζ）精确」；设计 §8 表述建议翻转批补注该边界 |
| FR-B03 | 0am O3 分位数自校准未收敛 | 过阈率实测 err 3.0%／prog 5.4%／其余 1.7–1.9%，未达 1−q=5% 目标；η=0.05 为语义推导初值，影子批复核 η/θ₀ 量级 |
| FR-B04 | 0am O4 stuck 影子 w 语义未落 | 设计 §6 明文留「影子批语义推导」；现行 1D stuck 闭式通道维持生产（v1 并存裁决） |
| FR-B05 | 0am O5 A2 分层读数未做 | 设计 §4 A2「按长工具轮占比分层如实报告」仅完成钉子，未在真实 run 分层读数（需带 `ORZ_MAX_WALLCLOCK` 的 run 样本） |
| FR-B06 | 0am O6 跨平台 bit-exact 未实测 | pinned libm 已落码、同平台 bit-exact 已测；跨平台（Linux 容器）对照建议随下次重建批收取 |

边界注记：0am 的 **O1（标签口径）已由 2026-09-17 账本批定稿**（S3 负结果按「无解码器裸锚点秩检验」口径登记）；**O2（RLI 转正/维持影子）仍留用户裁决**——O2 不决则 B 类六条与 0am 落实批序全部继续挂起（与用户本轮「留给狗粮线」指示一致）。

---

## 4. C 类：审查类挂账（8 条＋注记级 1；过夜四联批修复批（orz `183fbb08`/`1f303cf4`）后留裁决/未修）

登记原文见 [QUAD_BATCH_DEEP_REVIEW §7/§10](QUAD_BATCH_DEEP_REVIEW_2026-09-15.md)。

| ID | 项 | 状态与内容 |
|---|---|---|
| FR-C01 | 0AC-A3 接线钉缺口 | 首批端到端钉 ×3 已落；**B1 投递/宿主 drain 两枚接线钉仍缺**（需 fake host 基建扩展，建议随下批） |
| FR-C02 | 0AC-A5 ⑥ 第三触发 | 「结果已形成」触发未落码，静默缩围已转**显式挂账**（审查件 §2.2 维持登记） |
| FR-C03 | 0AC-A6 Err 路径 close_drop | loop 终止多点 `?` 传播使 `model_compression_close` 丢弃；单一收口点不存在，涉及结构改造，留裁决（统一出口 `finalize_model_compression_close` 已覆盖四点，残余仅 Err 传播路径） |
| FR-C04 | 0AE-C11 编辑清单跨 run 归属标签 | 行为语义裁决项，未动 |
| FR-C05 | 0AE-C12 跳变齐发 | 行为语义裁决项，未动 |
| FR-C06 | RET-B1 完整解析器（DDG/Google） | 用户裁决「解析要补充」；架构口径＝解析器属纯 HTTP 后备车道，与浏览器车道正交；**Google 纯 HTTP 直取为 consent 页** ⇒ 实施批待排、须带一手 SERP 样本（含 RET-B10 `/url?q=` 形态核对） |
| FR-C07 | RET-B5–B10（复合条） | 检索面五项观察，S4 真机样本前置 |
| FR-C08 | 0AE-C4 阶梯/fold 死面 | 用户裁决「暂缓」（0ae 整体待改）；**v8 收口清理后两族实现面均已退役，大概率已消解——正式销项待账本批复核** |
| — | 注记级 | 「blackboard_write」名字在 controller 注册处与新过滤函数仍为两份字面副本（已注释互指）；重名面收敛待顺手批 |

---

## 5. D 类：评测/装置侧（4 条）

| ID | 项 | 状态与内容 |
|---|---|---|
| FR-D01 | 适配器旗标契约漂移 | `candidate` 立案仅记录（索引 GAP-ORZ-ADAPTER-FLAG-DRIFT）：`--max-tool-rounds 999` 静默忽略、`--retrieval-mode` 已弃用 WARN 后忽略、`eval_browser` 未传——三者最易把装置缺件误读成镜像缺陷；候选动作＝`run_official_2.1.sh` 起跑前旗标对账清单。来源 [TB21 R0 §6.9](TB21_V41_ROUND0_MEMORY_HEAVY_START_2026-09-13.md) |
| FR-D02 | 孤儿 orz 修复未实机复验 | 装置侧修复已落（`run_r0_heavy_official.py` 传 `--ak max_wallclock`，commit `419b1c8f`）但**未实机复验**；同批 pending＝verifier 通道 900 s 吃满定向复现（审计建议 R6），随 0ac S4/89 题重跑收取。来源 [时间预算审计 §T2/R6](FRAMEWORK_TIME_BUDGET_SEMANTICS_AUDIT_2026-09-13.md) |
| FR-D03 | Docker 代理切换摩擦（复发性） | 每次 Linux musl 载体重建须按 052/054 先例手工切换 Docker 代理＋收尾字节级还原（0.5.2 五连败→0.6.0 一次通过的波动亦有）；靠先例处置、未立项。来源 [054_CARRIER_REBUILD §网络摩擦](054_CARRIER_REBUILD_2026-09-16.md) |
| FR-D04 | 0z 磁盘轴容器形态不可取证 | 容器内磁盘读数是盘内视角，0z 磁盘轴在 TB 跑批容器形态不可取证——**边界登记（非缺陷）**，取数须回宿主面。来源 [TB21 容器审计](TB21_V41_EVAL_CONTAINER_AUDIT_2026-09-13.md) |

---

## 6. E 类：W2 线冻结期暂存观察项（4 条；处置口径均为「解冻后」，线现状休眠）

登记原文见 [W2_ORZ_DEFECT_EXTRACTION §1](W2_ORZ_DEFECT_EXTRACTION_2026-09-07.md)。

| ID | 项 | 内容摘要 |
|---|---|---|
| FR-E01 | W2 D-2 `run_finished.turn_count` 硬编码 1 | 单提示 run 碰巧正确；跨 prompt 会话语义下失真，事件面消费者不可用作真实轮次。处置＝解冻后改真实会话轮计数或字段退役（schema 语义核对）。**注：登记行号 `controller.rs:3541` 为 2026-09-07 时点，现已漂移** |
| FR-E02 | W2 D-3 权限门 deny 信封不入事件面 | deny 后无 tool_started/completed/policy_denial 事件，拒绝原因 journal 不可审计（只剩薄 `permission_decision`）；压测归因不可判定。处置＝解冻后评估 deny 结果信封（reason 脱敏）入 journal。**注：0p/0q 落码后形态未复核，收录按登记原文** |
| FR-E03 | W2 D-4 key 经 env 对 shell 子进程可见 | 模型可持 `$env:ORZ_DEEPSEEK_API_KEY` 自调 API（chunk1 实证）。防除路径（子进程 env 剥离）解冻后立项 |
| FR-E04 | W2 D-5 tool_running/自动后台化真实面零触发 | 6/6 任务无 ≥180s 前台命令，机制面已由 fake 覆盖、真实面悬空；是否保留该真实面判据随 chunk3 任务形态评估 |

（W2 D-1 `.gsa` shell 旁路**不在本类**：已被 ADR-0010 §14.61 两段门＋「shell 直读＝跳过教育的旁路」裁决吸收，见索引 OBS-GSA-READ-LANE-ASYMMETRY 修订。）

---

## 7. F 类（引用）：摩擦衍生、已立项未闭环（7 项；明细以 BACKLOG/TODO 为准，本档不重复定义）

1. **0aj** 黑板写权限放行——判据面满足（0am run：8 调用全放行→`plan_write` ×8→读回 exit 0），闭合待裁决；
2. **0ak** 无头三键存档——0am run 未跨 500K 不可判，待自然跨 500K 的长 run 收取；
3. **0al** 门禁冻结克隆树——真克隆整链复验待日后 run；
4. **0ac S4**——机械层即时回报实机复验＋89 题整轮重跑待放行（同时是 FR-D02 的收取载体）；
5. **0ae**——A/B 判据留 S4 实机；
6. **0ah**——0.77 换算与五项真机读数已收（0am run），S2 三形态裁决与尾批（块轴）待放行；
7. **0am**——本轮用户裁决挂起（「LIF 组件暂时不做，留给狗粮线」）；orz 10 文件未提交改动是否合回留裁决（血缘注意项见 0am 收口报告 M-2）。

（0af 真机效果读数待下一轮狗粮 run——随 run 收取，不计立项。其余一般开放项（0aa、0w、0y、0z S4、组件审计、P2 系等）非摩擦衍生，不在本盘点范围，见 [BACKLOG](../BACKLOG_AND_PRIORITIES.md)。）

## 8. G 类（引用）：已有归属的移交/裁决面（4 条；不计入开放摩擦）

1. **0.77 换算深水区漂移**（est/real 1.43–1.48 vs 浅水 1.32–1.34）是否按深度分段修正——读数已收，留 0ah 线裁决；
2. **H1 注入 × I6 前缀纪律**缓存成本（三次 H1 两次破缓存，全量 miss ≈440K real）——0ah 线复核；
3. **`context_compressed` 部分字段 null 的读数载体形态**——随 0ah S2 三形态裁决一并看；
4. **O2（RLI 转正/维持影子）**——B 类之门，留用户裁决（agent 建议＝维持一阶基座、影子默认关、补数据后复审）。

## 9. H 类：排除面（已处置/裁决不处理，仅列以防重收）

- 0ac 深审摩擦 A（fold 失忆）→ 并入 0ae D4＋v8 分块承载；摩擦 B → 0af（2026-09-16 闭合，orz `b6ed78d9`）；摩擦 C → 0ai（闭合，orz `b682a67f`）；摩擦 D-① 虚构「协调者指令」→ 用户裁决留档不处理；摩擦 D-②「先跑已写单测」→ 流程建议随任务书 carry；
- W2 D-1 → §14.61 裁决吸收（见 §6 尾注）；
- fixture 生成器重跑删 38 件观察 → v3.50 批闭合（生成器恢复 single source of truth）；
- `host_resource_snapshot` 判定表缺项 → GAP-ORZ-HOST-RESOURCE-SNAPSHOT-DROP 闭合（修复进 0.5.2 载体）；
- orz-tui 非穷尽 match 构建断裂 → 052 重建批机械修复（降级 `TuiEvent::Unknown`）；
- 0am F8（模型侧参数名误用，已自纠、非框架）／F9（大输出管理面，正面项）／M-2（归因更正本身已闭环）；
- 容器内无浏览器 → 设计内（0ac 判据边界「不新增容器内浏览器」）。

---

## 10. 建议去向（仅供裁决，本档未动任何代码与账本实体）

- **低成本一次性修正簇**：FR-A01/A02/A08/A09/A12（env 注入/编码/断链归档/工具面/文档注记）＋ FR-A14（启动器脚本化，两次坐实优先级最高）；
- **唯一有 journal 证据的代码缺陷**：FR-A03（序列化 null），排查面最小、建议随下批顺手定位；
- **随线收取簇**：FR-D02 随 0ac S4；FR-B01–B06 随 0am 复审（O2 裁决后）；FR-C07 随 S4；
- **留裁决簇**：FR-A11（立项候选）、FR-C01–C05、FR-E01–E04（解冻后）；
- **待复核销项**：FR-A07（v8 后形态）、FR-C08（v8 后大概率消解）、FR-E02（0p/0q 后形态）。

## 11. 处置批注（2026-09-17 用户裁决；本批零实施，仅落盘判定）

1. **不处理（4 条）**：FR-A01／FR-A02／FR-A05／FR-A09。用户口径＝沙箱环境存在的问题，orz 日常使用直接进真机环境。**落档勘误（如实）**：A01（沙箱缺 protoc）／A02（沙箱终端 GBK 编码）／A09（沙箱 shell 无 unix 工具面）确为沙箱 shell 环境面；**A05 按登记原文实为宿主（真机）页面文件/内存限制**（0am F6 原判「宿主页面文件/内存限制（非代码问题）」，建议为「狗粮机加大页面文件或 -j 分批」），非沙箱特有——四项均按用户裁决记**不处理**，A05 归类勘误留档（真机日常若撞到全量 check 资源失败，此条须翻案重议）。
2. **留进一步考虑（2 条 → 同日已双双处置，见下）**：FR-C04（0AE-C11 编辑清单跨 run 归属标签）／FR-C05（0AE-C12 阶梯跳变齐发）——用户要求详解后另行裁决，详解已随本批提交主会话。**落档前代码核对补充**：C05 原面 `attention_ladder.rs` 已随 D2 下线整档删除（文件不在码），后继 v8 阶梯的同形问题已由 2026-09-16 R-12 处置批修复（单轮暴涨只注入最高档、低档记 `suppressed_superseded_by_higher_tier`，`agent_loop.rs` 在码）；C04 为存活语义裁决项——D4 段当前仍以「【本 run 自编辑文件】」标签渲染**会话级累积**的 edits 分区（`action_ledger.rs:142`＋`agent_loop.rs:2034`；v8「按 epoch 冻结」仅约束重渲时机〔前缀稳定 I6〕、不改变数据范围），选项＝改标签口径／分 run 标注／按 run 重置（第三者不建议，会话恢复场景会重新制造失忆）。
   - **FR-C05 处置（2026-09-17 用户令核实）＝销项成立，闭合**：①原面 `attention_ladder.rs` 不在码（D2 下线，0ah S1 批 orz `61982a56`）；②后继 v8 阶梯跳变语义正确——`ContextScale::due()` 一轮可返回全部新越档（`context_scale.rs:212`），注入点**一轮内只注入最高档**（`agent_loop.rs` 约 :2103，「只读审查 R-12③ 处置」注释在码），低档水位与事件照记、`form=suppressed_superseded_by_higher_tier`，T1 同轮时全部让位截断告知块（`deferred_to_truncation_notice`，告知块自带截断后读数）；③跳变场景与 T1 同轮两枚回归钉在码（`agent_loop.rs` 测试，断言 250k standalone＋100k/150k/200k suppressed）。无残留「逐档齐发」路径。
   - **FR-C04 处置（2026-09-17 用户裁决采②分 run 标注）＝已落码（未提交）**：`EditRecord` 写时 run 章（`#[serde(default)] run: String`，旧黑板/存档零迁移，空＝无章旧行）；生产写入点唯一（`host_exec/tool_run.rs` 盖 `writer.run_id()`）；D4 渲染（`action_ledger.rs render_run_context_block`）按 run 章分「【本 run 自编辑文件】／【会话历史自编辑文件（此前 N 个 run／＋无章旧行）】」两组，`current_run=None` 时全部归历史段；3 枚钉子（分组不混／单 run 会话不虚设历史段／无章可对不渲染本 run 段）。读数：orz-loop **790/0/3**（0am 批基线 787＋新钉 3）、`fmt` 干净、clippy **50 处既有位置零新增**（逐位置核对均不在本批改动区间）。测试字面量补字段 14 处（blackboard/epoch/controller/planning/summary/render_fold，均为测试构造）。**改动与 orz 工作树在场的 0am 未提交批 hunks 不相交**（唯 blackboard.rs 两批同文件、改动分属 EditRecord 与 TemporalSessionSnapshot 字面量两处），合回/提交批切留用户裁决。
3. **判定需要处理（30 条＋注记级 1）**：其余全部当前未处理项——A 类余 10（FR-A03/04/06/07/08/10/11/12/13/14）＋ B 类 6（FR-B01–06）＋ C 类余 6（FR-C01/02/03/06/07/08）＋ D 类 4（FR-D01–04）＋ E 类 4（FR-E01–04）＋ 注记级 1（blackboard_write 字面双副本）。**处理排期/批次/优先级另行裁决，本轮不直接实施。**随带口径：B 类实施以 0am/狗粮线与 O2 裁决为前置门（用户 2026-09-17「LIF 组件暂时不做，留给狗粮线」）；FR-C07／FR-D02 等 S4 前置项随其门收取；FR-A07／FR-C08／FR-E02 的「处理」第一步为形态复核（复核后销项或处置）；F/G/H 类维持不变。

---

> 维护口径：本档为 2026-09-17 时点快照；逐项处置后应在对应行补处置批注（沿用 QUAD §10 先例），全部处置完毕后整档转 `historical` 并入 `存档/`。计数行（36＋1/7/4）若被后续批引用，以本档 §0 为唯一口径。
