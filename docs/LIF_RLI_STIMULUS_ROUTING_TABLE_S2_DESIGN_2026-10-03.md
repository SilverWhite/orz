# LIF/RLI 刺激面路由表设计（S2）——标签→通道认领矩阵

> **版本**:v1.1;日期:2026-10-03(v1.0 首版落卷〔174 批〕;v1.1 S3 重放批勘误〔175 批〕:写入管控 block 路由细化、验证词表匹配口径、snapshot 跨档过滤、S3/P8 边界注、侵蚀检出负结果登记)。
> **定位**:0am 刺激面改革批序的 **S2 产物**（上游＝[`LIF_RLI_STIMULUS_TYPED_BUS_DESIGN_2026-10-03`](LIF_RLI_STIMULUS_TYPED_BUS_DESIGN_2026-10-03.md) v1.2 §4.1 授权:标签→通道的认领矩阵、封闭性原则、死窗规则、P8 标签成因段可用标签集）。本档只定义**通道看见什么**;通道如何判断(动力学/阈值/触发沿)与**何时叫**(触发条件)一概不在本档(P9)。
> **转正裁决(2026-10-03 用户令)**:「请开始进行0am吧」——0am 线自 2026-09-17 的挂起态就此解挂,本档为解挂后首件。**边界**:O2 审计发现本身(idle-kill 对输出已重定向进程盲)仍独立缓议,不随本线开启而自动处置。
> **硬约束**:上游 §2 十原则 P1–P9 全程适用;本档违反 P9 的表述一律无效。

---

## §1 源码实态复核(刺激漏斗 as-is,2026-10-03 实读)

**唯一刺激入口**＝`classify_event_outcome`(`orz-assurance/src/lif/channels.rs:174`):对 `tool_completed` 载荷判四值——`policy_denial` 标记或结构化拒绝码→**Deny**;`timed_out` 或 `status=error ∧ 无 exit_code`→**Error**;`exit_code=0`→**Success**;其余(含**非零 exit_code**)→**Other**。

**生产喂入两点**:①命令完成臂(`orz-loop/src/host_exec/tool_run.rs:4057` 附近,四值与漏斗逐字同形);②拒绝公共入口 `feed_lif_deny`(`host_exec/failure.rs:40`)。**喂入负载仅 `{outcome, wall_ms}` 两键**。

**现行路由**(LIF 1D 与 RLI 影子同流):Error→Err(注入 1.0)/Deny→Deny(1.0)/Success→Prog(set 1.0)/**Other→无任何通道**/`wall_ms`>SLOW 阈→Slow(时长加权,P4 先例)/动作间隔>STALL 阈→Stall(1.0)。

**四洞定案**(与上游 §1 三线证据一一对应):

| # | 洞 | 机制 | 证据 |
|---|---|---|---|
| H1 | 产出混叠 | 一切 Success(变更/检索/验证/阅读)全入 Prog——u_prog 语义被稀释 | 165 批 u_prog≈0.975 悖论(域机钉 normal)的刺激面根源 |
| H2 | Other 黑洞 | 非零退出(**测试失败、构建失败**)=Other=全通道不可见 | 「侵蚀全程未检出」(171 批)的机制根源——持续验证失败在刺激面上不存在 |
| H3 | 标签维度丢弃 | 工具名(`tc.name`)、命令串(`tc.arguments`)、结构化拒绝码在喂入点作用域内**被弃**——通道只知四值,不知看见了什么 | P8 标签成因段(「验证类动作 0 次」)无从谈起 |
| H4 | 时长无类感 | 验证类命令的合理长时与摩擦性长时同入 Slow | 171 批:61s 成功测试→slow×3 衰减尾迹回声 |

结构化拒绝码词汇(`is_denial_code`,25+ 码)已在合约面——Deny 通道有现成标签维可用,无需新采集。

## §2 通道集定案(5→7 kind)

每通道语义**独占**(P1);新通道＝新增小参数集(P2);周期以 T̂ 倍数预注册(有单位、有指涉、S3 开跑前冻结、只按物理依据调整、永不对结局拟合——P3)。

| kind | 语义(独占) | 看见什么(认领→注入) | 注入规则 | 参数(预注册初值) |
|---|---|---|---|---|
| `Prog` | **产出节律**——durable 工件变更的成功节律 | 变更类工具成功(`search_replace`;终端命令不入——无可靠词法变更判据,保守不认领) | set 1.0 | 周期不变(`RLI_PROG_PERIOD_ROUNDS×T̂`) |
| `Verify` | **验证摩擦**——验证类动作的失败累积(侵蚀面) | 验证类动作结果(`run_tests` 工具＋终端验证词表,§5.1) | **失败→1.0;成功→零注入**(基线参考语义:u_verify 只装失败摩擦) | 新通道;周期 **8·T̂**(验证节律高于产出——TDD 循环保守量级) |
| `Err` | 执行失败 | host error/timeout/**非验证类非零退出**/非终端工具错误 | 1.0 | 不变 |
| `Deny` | 纪律拒绝 | `policy_denial`＋结构化拒绝码(**带拒绝类标签维**,§5.2) | 1.0 | 不变 |
| `Slow` | 摩擦长时 | **非验证类**动作 `wall_ms`>阈值(验证类豁免——H4 修正) | 时长加权(P4 先例保留) | 不变 |
| `Stall` | 动作失节律 | 动作间隔>阈值 | 1.0 | 不变 |
| `Ctx` | **认知负载**——上下文机械的活动节律 | `context_compressed`＋`ledger_fold_advance` | 1.0 | 新通道;周期 **32·T̂**(会话尺度——压缩为 O(每 run 数次) 罕见事件,防单事件尖峰) |
| `Infra` | **供给摩擦**——环境/供给层劣化(非动作质量) | `transport_retry`＋`tool_availability_check` 翻转＋`host_resource_denied`＋`resource_limit_hit`＋`host_resource_snapshot` 跨档＋`ledger_fold_write_failed` | 1.0 | 新通道;周期 **64·T̂**(最罕见族,单事件不成节律) |

**两侧形态**:RLI 谐振通道 bank 5→7(同构扩展,新 kind 各配极点按上游 §11 极点配置表语义分配——Ctx/Infra 属水平/新鲜度类走实极点);LIF 1D FirstOrder bank 机械镜像 5→7。**域机规则不动**(域判定读哪些通道是既有动力学,本档不触碰)。

**自激环禁令**:`mechanical_audit_update` 等**输出面**(模型面注入行)永不入刺激面——通道状态产生输出行,输出行不得反喂通道。

## §3 标签→通道认领矩阵(上游 §3 总清单逐族认领)

> 认领三态:**认领**(该标签按所指通道注入)/**不认领**(不采集——封闭性)/**标签维**(不作注入、作 P8 成因段标签)。产出条件按当日源码枚举与批次史复核。

| 族(上游 §3) | 认领 | 备注 |
|---|---|---|
| 生命周期(7) | 不认领 | run 边界由引擎既有复位语义消费,非刺激 |
| 宿主资源(6) | **认领→Infra**:`host_resource_snapshot`(**跨档过滤,v1.1 细化**:run 首测＝基线不计,tier 变更才计〔S3 实证:二跑 8 见 0 跨、首跑 8 见 0 跨〕)/`host_resource_denied`/`resource_limit_hit`;不认领:`resource_exhausted`/`process_tree_reaped`/`reclaim_performed` | 后三者为终局/清扫语义,run 已临终局,刺激无消费者 |
| 提示/模型(5) | 不认领 | 节律原语(T̂/轮间隔/工具间隔)由估计器内在消费;输出内容面 P7 禁入 |
| ACP(2) | 不认领 | 载体事实 |
| 工具(7) | **认领**:`tool_started`/`tool_completed`→动作类标签器路由(§5.1:Prog/Verify/Err/Slow/中性五路);不认领:`tool_proposal`/`permission_requested`/`permission_decision`(拒绝事实已经拒绝臂入 Deny,防双计)/`tool_running`(与完成事件双计)/`budget_cue_injected`(默认 off) | 喂入点已备工具名与命令串(H3 修正的落点,§6) |
| 保证/门(5) | 不认领(v1) | future 路由条目候选——若 S3 读数显示门触发密度有信号价值,按 P2 以路由条目增补,先登记不采集 |
| 检索子代理(7) | 不认领(v1) | 条件产出(检索启用会话才产);死窗规则见 §4;`retrieval_mode_transition` 产侧退役 |
| mechanical_audit_update(1) | 不认领 | 输出面,自激环禁令(§2) |
| ACAF(3) | **标签维**:`control_ticket_rejected`(经拒绝码入 Deny);不认领:issued/consumed | 破坏性动作授权语义 v1 不入刺激 |
| 上下文(4) | **认领→Ctx**:`context_compressed`/`ledger_fold_advance`;**认领→Infra**:`ledger_fold_write_failed`;不认领:`context_recovery_truncated`(生产零写入)/`session_archive` | |
| 快照/工件(3) | 不认领 | |
| 计划面(8) | 不认领(v1) | 工作形态面非摩擦/产出面;`console_order_written` 退役 |
| 传输(1) | **认领→Infra**:`transport_retry` | |
| 写入管控(1) | 不直接认领 | **block 事实入 Deny（v1.1 细化）**:S3 实证两轮各有一例写控 block 完成事件**无 `policy_denial` 标记、无结构化拒绝码、无 exit_code**——legacy 落 Err（171 批 streak×3 回声源），S2 路由按 `call_id` join `write_control_review(review=block)` 判 Deny(写控类)；**warn 不入 Deny**（warn 后命令照常执行，完成事件按自身结果路由——防双计）|
| face_fingerprint(1) | 不认领 | 缓存归因面 |
| 遗留回放(4) | 不认领 | 产侧已退役,仅历史回放 |
| 字段级:`dep_graph` | 不认领 | 依赖图事实 |
| 字段级:`failure_target`/`failure_agg` | 不直接认领(标签维) | 失败事实经工具喂入臂入通道;身份聚合供 P8 标签成因 |
| audit kind(§3.2,10) | 不认领 | 输出面(自激环禁令) |
| 黑板 PULL 分区(§3.3,11) | 不认领 | 模型读出面,非事件刺激 |
| 会话侧车/状态面(§3.4,6 组) | 不认领(内部态) | T̂/复杂度等已为引擎内在量;非外部刺激 |
| 判官族(§3.5,48) | 不认领 | 离线执法面,非运行时刺激 |

**封闭性原则**:未认领标签**即不采集**——刺激面不存在隐藏入口;任何新增认领必须修订本档(路由条目形态,P2),禁止实现层静默扩面。

**死窗规则**:条件标签(检索族/资源族/门族)零输入＝其通道**基线休眠**,无需特判、不产生自激(J3 判据钉);分会话型预期死窗如实登记(非检索会话检索族恒零;非 hard 档资源族恒零)。退役/零产标签(8 变体)不设认领,历史回放不受影响。

## §4 机械标签词汇表(路由与 P8 成因段共用,闭集)

### §4.1 动作类标签器(按工具名＋终端命令词,机械词法,零输出内容读取——P7)

| 类 | 机械判据(闭集初版) |
|---|---|
| 变更类 | `search_replace`(结构化编辑工具) |
| 验证类 | `run_tests` 工具;终端验证词表(大小写不敏感、首词/子命令前缀):`cargo test/build/check/clippy/fmt --check`、`pytest`/`python -m pytest`/`python -m unittest`、`make`、`npm test`/`npm run build`/`npx tsc`、`go test/build/vet`、`gradle …test`/`mvn …test`、`dotnet test/build`、`cmake --build`、`rake`、`eslint`/`ruff`/`pylint`/`mypy` **〔v1.1 匹配口径〕**:大小写不敏感子串＋两侧字母数字词边界(复合命令 `cd x && python -m unittest …` 的子命令可命中;`Makefile` 不误命中 `make`;S3 重放件实现同此) |
| 阅读/检索类 | `read_file`/`list_dir`/`grep`/`search_tool`/`web_search`/`web_fetch`/`lsp` |
| 黑板/会话类 | `blackboard_read`/`blackboard_write`/`context_compress`/`compaction_whitelist_add`/`todo_write`/`update_goal`/`ask_user_question` |
| 提交类 | `submit` |
| 中性 | 其余终端命令与非工具事件——仅计步,不注入任何通道 |

词表扩展只随批次修订本档(闭集纪律,禁实现层临时加词)。

### §4.2 拒绝类分组(Deny 标签维,对 `is_denial_code` 词汇分组)

权限/票据(`permission_*`/`control_ticket_rejected:*`/`control_tool_lane_denied`)｜计划/车道(`plan_*`/`submit_*`/`console_*_lane_denied`/`order_slot_busy`)｜门/护栏(`content_anchor_mismatch`/`sealed_tool_denied`/`retired_tool_denied`/`*_candidate_*`/`round_inject_budget_exceeded`)｜检索启用(`retrieval_not_enabled`/`retrieval_mode_*`/`retrieval_role_*`/`nested_subagent_dispatch_refused`)｜其他(`missing_test_runner`)。

### §4.3 P8 成因段可用标签集(闭集)

①**机制成因**(现状即可机械导出):通道名＋u 读数＋θ(或域值)＋计数窗(k)。②**标签成因**(随本档落地):动作类计数(「验证失败 N 次/通过 M 次/变更 K 次」窗内)、拒绝类名＋计数、Ctx/Infra 事件名＋计数。**行宽预算**:现 ≤240B 加成因段必然越限,新值由 P8 实现批按所选标签集实测重订(上游 §4.4 口径,本档不定值)。

## §5 喂入负载扩展(路由前提,P8 落码;本档只定合约)

现负载 `{outcome, wall_ms}` → 增机械路由键:**`tool_name`、`command`(终端命令串)、`denial_code`、动作类标签**(由标签器在喂入点计算)。journal 载荷已含全部字段——**零新增采集面**(上游核心结论「journal 已是类型化事件总线,只差读侧路由纪律」的兑现点)。标签器落 `orz-assurance`(独立 router 模块),重放件 `rli_shadow_replay` **同源复用**——S3 对拍一致性由构造保证。

## §6 S3 离线重放预注册判据(框架内事实,P6;S3 开跑前可修订,开跑即冻结)

> **〔v1.1 S3 读数已入档〔175 批〕**;判据原文开跑前冻结未动;S3/P8 边界＝Verify/Ctx/Infra 三新通道的**动力学**不在 S3(S3 只产出其输入序列账目),既有五通道新旧馈对比走真实引擎。重放件:`rli_shadow_replay --s3`(example 级扩展,零生产面改动;标签器即 P8 orz-assurance 模块的种子,同源复用)。语料＝两轮 recli journal(首跑 RUN-65f2858a-0..7／二跑 RUN-72764d4e-0..7,snapshot 快照)。读数档:`D:/tb-eval/0am_s3/s3_run1_20261002.json`／`s3_run2_20261003.json`。〕

- **J1 认领一致性**:每笔注入可归源到唯一通道(矩阵合规,机械断言——双计/漏计即红)。**读数＝过**:两轮 closure 恰等(二跑 172 完成＝67 认领注入＋35 verify 零注入＋70 显式中性;首跑 230＝115 认领事件〔118 注入,含 3 笔 prog+slow 双标签〕＋46 零注入＋69 中性);路由分派为 (事件,标签) 的确定性函数。
- **J2 封闭对账**:注入总数＝认领事件总数;并含 171 批三回声**逐例对拍**:(a) 写控 `format` 误报→恰落 Deny 类一次、无 Err 回声;(b) 61s 成功测试→Verify 零注入∧Slow 零注入;(c) 去源后 streak fire 不复现——若仍复现,登记为动力学面残留移交后续批(本档不扩参数,P2)。**读数＝过,三例全实锤**:(a) 二跑 `RUN-72764d4e-2` seq88 写控块(format 误报)legacy 落 Err→S2 恰 Deny(写控类)一次;首跑同型一例(RUN-65f2858a-1 seq22);(b) 二跑 seq42 wall=60815ms(`python -m unittest discover`——命令串经 `model_output.tool_calls` 按 call_id 结构化取回)legacy [prog+slow]→S2 零注入;首跑 RUN-7 七连发 79–98s 测试命令全豁免(首跑 Slow 通道 9 注入全部为验证类时长＝H4 实锤);(c) 二跑 legacy err 输入恰 1(即误报块)——去源后 err streak 驱动消失;slow×3 驱动(61s unittest)归零,S2 slow 输入＝0。
- **J3 死窗与基线**:零输入通道全程基线(无自激)。**读数＝过**:二跑 S2 slow 全程 u≡0(max 0.0);Ctx 输入二跑 4/首跑 10(compressed;fold_advance 两轮均 0)、Infra 17/18(availability 17＋retry 首跑 1;snapshot 跨档两轮均 0——run_start 基线不计,v1.1 过滤);Verify 输入两轮均 0(见 J4 负结果)。
- **J4 语义分离读数**:u_prog 分布新路由 vs 旧路由对照(预期:纯探索/验证 run 的 u_prog 下移＝真实低进度显形;165 批悖论复验);侵蚀检出能力:历史含连续验证失败窗的 run(若两轮 journal 存在)Verify 通道抬升与否——**含负结果如实入档**。**读数**:=分布重塑＝**过**——二跑 u_prog mean 0.769→0.461/median 0.848→0.439,首跑 0.730→0.507/0.843→0.578;165 悖论复现于重放(首跑 legacy u_prog max **0.9756**、≥0.975 共 2 轮),S2 馈下慢性饱和带收窄(首跑 ge975 2→1)且量程拉开(编辑簇真突发可达 0.99);=**窗一致性(公平口径)**＝**过**——S2 语义进度实际(窗内变更类成功)下,S2 馈一致性**双轮均升**(首跑 50→57/136,二跑 68→106/169),legacy「一切成功＝进度」口径下降属定义翻转预期;=**侵蚀检出＝负结果如实登记**——两轮 journal 的验证类命令**全部通过**(零 verify 失败注入),本 workload 的侵蚀形态(基准评分面功能退化)不经动作面验证失败可见,Verify 通道在此两轮为死窗;这不推翻 Verify 语义(对真实「测试红」型侵蚀有效),登记为已知边界。
- **J5 双引擎同馈一致性**:LIF 1D 与 RLI 在同一路由流上锚点对拍(同源无分叉)。**读数＝同源成立**(两引擎消费同一变换流,构造保证);1D-vs-RLI 域一致率 legacy 0.970/0.934→S2 0.899/0.853——语义锐化后双引擎域机分歧增大,属预期观察(RLI 与 LIF 域语义不必同格,2026-09-20 用户裁决);域机阈值重校准属动力学面(非触发规则),留 P8 按 P3 物理依据酌处。

## §7 不变面(P9 兑现清单)

- **零触发条件**:本档无任何「当 X 出现即 Y」式规则;streak k=3、θ 分位自校准、域机规则、看门狗 181s、各提醒 kind 触发沿——全部原样(P8 成因段只描述动力学已触发的事实,不构成新触发条件)。
- 8 工具面、零注入/模型无感、journal 契约面(零 schema 变更)不变。
- 既有 5 通道参数不动;新通道参数为预注册初值(§2 表,物理依据在档)。
- **生产语义变更面(如实登记;S3 判据门通过后随 P8 生效)**:①Prog 收窄为变更类成功;②Slow 验证类豁免;③Other 黑洞填平(非验证类非零退出入 Err)。三项均为刺激面语义修正——域机规则不改而读数分布预期移动(J4 量化;若 S3 读数显示回归,回退变体随 S3 一并重放对比)。

## §8 P8 实现批序承接(自上游 §4.4 细化,放行门＝S3 判据读数入档)

1. 喂入负载扩展＋标签器模块(`orz-assurance`;重放件同源复用);
2. `ChannelKind` 5→7＋RLI/LIF 双 bank 扩展＋锚点表/`feature` 面同步(新锚点名随实现批定);
3. 成因段渲染(P8 两档:机制成因可先行,标签成因随 §4.3 词汇表)＋行宽重订＋0cp D6「无成因」子句勘误;
4. **RS-06 随批**(libm 精确钉版 `=0.2.15`、`rli_shadow_replay.rs` fmt、P3 五小件);
5. B 类随线项随批(FR-B02 设计 §8 边界注;FR-B03 过阈率读数随 S3;FR-B06 钉版)。

## §9 关键词

刺激面路由表、标签→通道认领矩阵、七通道、产出节律、验证摩擦、认知负载、供给摩擦、
Other 黑洞、动作类标签器、验证词表、拒绝类分组、封闭性原则、死窗规则、喂入负载扩展、
预注册判据 J1–J5、零触发条件、转正裁决、S2 设计档。
