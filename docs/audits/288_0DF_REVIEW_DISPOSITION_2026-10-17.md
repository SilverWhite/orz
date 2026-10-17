# 288 批：0df 全面审查处置——幂等旗封死恰一条＋非字符串锚对齐＋降级刊区分文案（2026-10-17）

> 性质：**审查处置批**（用户令「请对当前完成的0df内容进行全面检查，包括设计合理性、
> 实现合理性、设计与实现的符合性」＋「请对审查出的全部问题进行处理」）。287 批
> `6e107c77` 基线三面审查**全过、零 P0/P1**（双子代理深查 orz 落码与父仓合约／
> Python 镜像／fixtures＋主代理设计评估＋全套测试本机复跑核证为真）；审查出的问题
> 全部 P2，本批逐件处置。零跑批（未发起任何 orz 会话）；orz 子树落码提交
> `7966d171`（4 文件 +199/−11，**不推送**）；**未重建、未发行**——装机位仍 0.9.1；
> 计数 62 不变。

## §1 审查结论（处置的依据面）

- **设计合理性**：D1–D8 每项裁决均有实证支撑（283 只读复算之痛／TB 2.1 反例／r4
  档位 vs 推理量分列实证／哈希链不可改写），边界声明清晰。两点应知情边界（设计已
  如实声明）：金额闭环依赖派生台账配对工具（未立项）；transport 内部重试/降档轮
  的 token 不入计量行 ⇒ 对 provider 账面系统性低估（S4 对账以 `transport_retry`
  校准）。
- **实现合理性＋符合性**：D1–D8 逐条符合；机制逐项实证（折算点在取消检查点前、
  ModelResponse 唯一到达点无 double-fold、饱和求和＋incomplete 闩锁、毒化安全锁、
  4 构造点全透传、五终态臂恰一条、grill 丢弃、锚唯一写点、单一网关配置成立、模型
  面零泄漏）。三处已声明良性形态偏离：trait 名 `thinking_tier_label`→
  `usage_meter_labels`、复位 vs 新建、corpus +1→实落 +2。
- **合约面/镜像/fixtures**：五件合约、Python 镜像、fixtures 逐字核对＋机械实证
  全过（minimal 合法、constraint 恰单违反、锚 const 容忍旧 payload、法官六要点
  两侧同形、manifest 1488 抽三件重算逐位一致）。

## §2 处置清单（审查问题 → 本批落点）

| # | 审查问题（级） | 处置 |
|---|---|---|
| 1 | completed 臂落行后 `record_terminal` 罕见硬错上抛 Err 臂 ⇒ 第二条计量行（法官 `got 2`）（P2 理论边角） | `RunUsageMeter` 增 `journal_line_written` 幂等旗：`journal_usage_meter` 成功 record 后置位；record 失败不置位（Err 臂仍可重试落行）。「恰一条」生产侧封死，不只靠法官事后报 |
| 2 | 非字符串锚：Python 报错、Rust 经 `str_of` 静默跳过（P2 parity 微差） | Rust `verify_usage_metering` 锚扫描对齐 Python：锚键存在但值非字符串同走「unknown version」错误；缺键/显式 null 仍容忍。corpus +1 场景 `usage_metering_non_string_anchor`（钉值 261→262）锁进对拍 |
| 3 | 降级刊（锚已落盘后中途 journal 降级）`got 0` 无区分文案（P2 噪音源） | 法官族对该形态附区分性注（盘面如实＝降级写入器丢弃计量行；**执法不豁免**；0q 锚定族同行为类）；Python `_verify_v02_usage_metering` 同形落盘 |
| 4 | 崩溃刊判定语义未成文（P2） | 设计稿 §4 后勘注块如实登记：崩溃点在计量行前 ⇒ `got 0` 如实报；崩溃点在计量行后、终态前 ⇒ 无终态即无「终态前」义务，排序检查空转 ⇒ 静默通过（两侧同形） |
| 5 | 设计稿 D5/D8 文档级漂移未勘注（P2） | D5 inline 勘注（trait 实落 `usage_meter_labels->(String,String)`）＋D8 inline 勘注（corpus 259→261→262 演进）＋§4 后集中勘注块（六件） |
| 6 | Python 镜像头注计数漂移（v02=35 vs 实 41；P2 先例性） | 头注勘正 `v01=34 / v02=41`（dated 勘正注） |
| 7 | BACKLOG 0df 条目未同步达成态（P2 召回面残留） | P1 表行（`**S1＋S2 达成〔287〕＝S3/S4 待续；288 处置批**`，贴限收缩：0dd/0de 段重复的 `〔orz `1ecad57c`〕`×2 剥除，哈希留详情段）＋开放项段（S1 四项勘定结果逐项注记＋287/288 达成态＋三批档链接） |

不取项：降级刊/崩溃刊**不豁免执法**（盘面如实即应报）；ADR §14.86 语义不动
（v1.89 不 bump——本批无新裁决，只有实现收口与文档勘注）；manifest 外治理行无
触及（索引/TODO/BL 指针计数行随批照常同步）。

## §3 变更面（orz `7966d171`，4 文件 +199/−11）

| 面 | 落点 | 内容 |
|---|---|---|
| 幂等旗 | `orz-loop/src/usage_meter.rs` | `journal_line_written` 字段＋`journal_line_written()`/`mark_journal_line_written()` 两访问器（字段私有沿既有封装） |
| 写点 | `orz-loop/src/host_exec/facts.rs` | `journal_usage_meter`：旗已置 ⇒ 直接返回；成功 record 后置位（payload 构建与置位分两次取锁，锁内无 await） |
| 钉子 | `orz-loop/src/controller.rs` | 集成钉 `journal_usage_meter_is_idempotent_after_success`（同 writer 两调用恰一行） |
| 法官族 | `orz-assurance/src/journal/families.rs` | 锚扫描三臂（缺/ null/字符串/非字符串）＋降级区分注＋doc 注记＋corpus 场景 `usage_metering_non_string_anchor`（run_finished 载荷与 ok 场景同形防他族误触发）＋expected 表行＋钉值 261→262＋历史注链延伸＋直调钉两枚（`usage_metering_non_string_anchor_is_an_error` 三态／`usage_metering_got_zero_note_only_on_degraded_journal` 降级带注·崩溃素形） |
| Python 镜像 | `assurance/run_event_journal_validation.py` | `_verify_v02_usage_metering` 降级区分注同形＋头注计数勘正 |

## §4 验证读数

- `cargo test -p orz-loop --lib` ⇒ **883 过／0 败**（基线 882＋幂等钉 1）。
- `cargo test -p orz-assurance --lib` ⇒ **303 过／0 败**（基线 301＋直调钉 2；
  含 Rust↔Python parity **262 场景** × ALL_FAMILIES 逐格恰等全绿）。
- `cargo test -p orz-tui --lib` ⇒ **172 过／9 败**（同一 ACAF 预存失败集合，零新增）。
- `cargo test -p orz-bin --bins` ⇒ **15/0**；`cargo test -p orz-web --all-targets`
  ⇒ **47/0**（前端冒烟本批零触碰，287 读数沿用）。
- Python 镜像 `py_compile` 过＋五态行为实证（非串锚错／降级带注／崩溃素形／缺键
  容／null 容）全过。
- `cargo fmt`（触碰 crate）净；clippy 触碰面零新增。
- 门禁 `python scripts/check_repository.py` ⇒ `valid: true`。

## §5 生效路径

- 本批全为审查处置：①③属载体面行为微调（幂等旗只影响「罕见硬错双写」理论破口，
  正常路径零行为差；降级区分注只是法官文案），随下一版载体重建与 287/286 同车
  生效；②对 corpus/法官的增量即已在测试面生效。
- S4 真机读数判据不变（287 §5），另加一条：降级刊（如有）的 `got 0` 文案应带
  区分注，无注的素形 `got 0` 视为崩溃刊如实报。

## §6 台账

- 本档；设计稿 [`PER_TASK_METERING_DESIGN_2026-10-17`](../PER_TASK_METERING_DESIGN_2026-10-17.md)
  勘注三处；BACKLOG `0df` P1 表行＋开放项段同步；索引头行 v4.260 → **v4.261**；
  BACKLOG 指针行/计数行＋TODO 计数行/P1 路由行（0df 段加 288 处置批；0dd 段
  `run 末归档退役` 描述剥除贴限）；[`BACKLOG 第二卷 §1.228`](../BACKLOG_AND_PRIORITIES_2.md)。
- orz 子树落码提交 `7966d171`（不推送）；`orz_source_manifest.sha256` 重生成。
- 零跑批；未重建、未发行；装机位保持 0.9.1；计数 62 不变。
- 环境注记：本批执行中 D 盘两次满载（最低 155MB）致 orz-bin 编译失败与 loop 两枚
  cancel 类测试假红——清理 `orz/target/debug/incremental`（7.1G 纯缓存）后 7.0G
  可用、全部复绿；假红非源码问题（磁盘写入失败所致），如实登记。

## §7 关键词

288 批、0df 处置、审查处置、幂等旗、journal_line_written、got 2 破口、恰一条
生产侧封死、非字符串锚、unknown version、corpus 262、降级刊区分文案、
journal_degraded、崩溃刊语义、设计稿勘注、Python 头注勘正、BACKLOG 0df 同步、
parity 262、门禁 valid true、索引 v4.261、orz 7966d171。
