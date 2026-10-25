# 0dp S1 设计定稿——审批缺省回归全自动与用户命令接口（2026-10-25）

> **状态**：S1 设计定稿（348 批；用户放行令 2026-10-25「请开始0dp吧」）。设计权威＝
> ADR-0010 §14.91（v1.96，347 批即日转录）；本稿在 §14.91 授权的「形态 S1 勘定」范围内
> 定案，不新增语义、不改裁决。S2 落码、S3 Windows 载体重建换装随批序推进。
> **边界**：本稿只定形态与判据；零产品码。

## §0 定稿摘要（一屏）

- **缺省**：`initial_yolo` 缺省 `true` 于**全部车道、全部平台**——§14.90 条 3 的平台×活审批面
  缺省矩阵自 S2 起退役，`default_initial_yolo` 改造为 **`ORZ_YOLO` env 单一源**（读 env，缺省 true）。
- **旗**：`--yolo`（显式开）／`--no-yolo`（显式关→confirm-first），**无值精确形态**（`--allow-*`
  家规；`--yolo=1` 形式显式报错 exit 2）；二者同现报错 exit 2。旗是 **argv→env 桥**
  （`--yolo`→`ORZ_YOLO=1`、`--no-yolo`→`ORZ_YOLO=0`，`--allow-write`→`ORZ_ALLOW_WRITE` 同式）。
- **优先级**：旗（显式）＞ `ORZ_YOLO` env ＞ 内建缺省 true。
- **env 值域**：`1/true/yes/on`＝开、`0/false/no/off`＝关（`parse_acaf_fail_closed_env` 同族）；
  非法值**响亮报错**（orz-bin 启动面 exit 2；装配点兜底 spawn Err），绝不静默取缺省。
- **banner**：`[permission] mode=…` 改为**源感知**三形态（benchmark／interactive-yolo／
  confirm-first，附 `source=`），Web 启动面补同一行 stderr 回显；TUI 面不做 banner（登记不做）。
- **Web 工作台开关**：**不入 S2**（裁决降级为独立候选项，见 §6）——运行时通道
  （`set_yolo_mode`／TUI always-allow／Benchmark policy）全部不变。

## §1 勘定项与依据

### 1.1 旗名与形态（§14.91 条 2 候选定案）

- `--yolo`＝显式开（auto-approve）；`--no-yolo`＝显式关（confirm-first，逐次审批）。
- 无值精确形态家规（先例 `parse_benchmark_flags`，`orz-bin/src/main.rs:29-51`）：
  `--yolo=1`／`--no-yolo=off` 等 `=value` 形式**显式报错 exit 2**，绝不静默忽略；
  `--yolo --no-yolo` 同现**报错 exit 2**（矛盾指令不允许「后到者赢」）。
- 旗解析落点＝orz-bin `main` 全局 argv 扫描段（`--real`／`--allow-write` 同区，`main.rs:243-353`），
  **先于** `web`/`doctor`/`archive` 子命令分派（`main.rs:368` 起）——保证 `orz web --yolo` 可达。

### 1.2 env 单源（§14.91 条 2「`ORZ_YOLO` 单源」定案）

- **argv→env 桥**：`--yolo` → `set_var("ORZ_YOLO","1")`；`--no-yolo` → `set_var("ORZ_YOLO","0")`。
  先例＝`--allow-write`→`ORZ_ALLOW_WRITE`、`--real`→`ORZ_REAL`（`main.rs:250-269`）。
  桥式选择的理由：全部入口（TUI／`-p`／`--stdio`／`orz web` 及其 spawn 的 agent child、
  维护 CLI）都经 orz-bin `main` 一次；env 由子进程默认继承（`orz-web/server.rs:785` 无
  `env_clear`），单一转换点即可贯通所有车道，装配函数无需新增穿参。
- **单一源改造**：`PermissionBridge::default_initial_yolo`（`orz-host/src/permission.rs:228`）
  签名 `(gateway_live, hub_live) -> bool` → **无参** `default_initial_yolo() -> bool`：
  读 `ORZ_YOLO`，unset → `true`（§14.91 条 1 全车道缺省）。§14.90 条 3 的平台×活审批面
  条件表达式自 S2 起删除。调用点 `spawn_with_hub_and_policy`（`permission.rs:199`）与注释块
  （192-198）同步改写为 §14.91 口径。全部会话装配都经 `with_bridge_and_hub_policy` →
  `PermissionBridge::spawn_with_hub_and_policy`（`orz-host/src/lib.rs:1013`）唯一链，单源改造
  即全车道覆盖，无旁路。
- **值域与非法值**：新增 `parse_yolo_env(&str) -> Result<bool, ()>`（pub，与
  `parse_acaf_fail_closed_env` 同值族：`0/false/no/off`→false、`1/true/yes/on`→true、
  trim 后大小写不敏感），实现放 orz-host（`default_initial_yolo` 同文件）供 orz-bin 复用。
  非法值**响亮失败**：
  - orz-bin `main` 启动面早校验（`ORZ_YOLO` set 且 parse Err ⇒ eprintln＋exit 2；
    形态同 `acaf_fail_closed_enabled` 的 exit-2 面）——所有入口（含 `orz web`）先于分派拦截；
  - 装配点兜底：`default_initial_yolo` 遇非法值经 `spawn_with_hub_and_policy` 返回
    `Err("ORZ_YOLO=… is not a valid value (…)")`——库层不 `process::exit`，错误沿装配链
    显形（web 面呈会话创建失败、headless 面直接报错）。生产路径先被启动面拦截，此臂为
    直接消费 orz-host 的非常规入口兜底。
  - 禁止静默取缺省：非法值若静默按缺省 true 放行，用户「要关而没关成」是危险方向
    （家规「静默忽略绝不允许」，`main.rs:274`）。

### 1.3 语义矩阵（三态 × 关键交互面）

| 态 | argv | env | initial_yolo（全部平台/车道） | 语义 |
| --- | --- | --- | --- | --- |
| 缺省 | （无旗） | unset | **true** | §14.91 条 1：日常＝跑分姿态一致，全自动审批 |
| 显式开 | `--yolo` | （桥写 1） | true | 与缺省同效；banner 标 `source=--yolo` |
| 显式关 | `--no-yolo` | （桥写 0） | **false** | confirm-first：Ask 直达交互审批；**headless 无活审批面时 Ask 既有契约 fail-closed 到 Deny**（无人值守用户显式选择，行为同旧非 yolo 面） |
| env 覆盖 | （无旗） | `ORZ_YOLO=<v>` | 按 v | 合法值同上两行；非法值响亮报错（§1.2） |

- **优先级**：旗 ＞ env ＞ 缺省。旗直接覆写 env（桥式语义，后写生效），故「旗＞env」
  自然成立；用户 shell 里常驻 `ORZ_YOLO=0` 时 `--yolo` 仍可本会话显式开——与
  `ORZ_ACAF_FAIL_CLOSED`×`ORZ_ALLOW_*` 的 argv/env 关系同构。
- **Benchmark 交互**：`--allow-write`（Benchmark policy）与 yolo 旗**正交、不互斥**——
  Benchmark 面的确定性放行不经 initial_yolo（`PermissionPolicy::Benchmark` 独立臂，
  `orz-host/src/lib.rs:1759-1762`），§14.91 条 2「Benchmark policy 面不变」照旧。
  二者同给时 banner 显 benchmark 态（§3 优先级）。
- **不变面**（§14.91 条 2 明示）：运行时 `set_yolo_mode` 通道、TUI「always allow」流程、
  浏览器 download/script 强制审批类（yolo 下仍逐次人工）、Read-only 自动放行、
  `remember_tool_approvals=false`（无持久授予可乘）——全部照旧。

### 1.4 banner 文案（源感知三形态，S2 落码随附）

`permission_mode_banner`（`orz-bin/src/main.rs:687`）改为按「Benchmark ＞ yolo 态」优先级
解析，yolo 态附**来源**（缺省／旗／env）：

| 条件 | 文案（逐字） |
| --- | --- |
| `ORZ_ALLOW_WRITE` set | `[permission] mode=benchmark allow_write=on allow_shell={b} allow_network={b}`（不变） |
| yolo 开·缺省 | `[permission] mode=interactive-yolo (auto-approve, source=default; benchmark axes off)` |
| yolo 开·旗 | `[permission] mode=interactive-yolo (auto-approve, source=--yolo; benchmark axes off)` |
| yolo 开·env | `[permission] mode=interactive-yolo (auto-approve, source=ORZ_YOLO; benchmark axes off)` |
| yolo 关·旗 | `[permission] mode=confirm-first (per-action approval, source=--no-yolo; benchmark axes off)` |
| yolo 关·env | `[permission] mode=confirm-first (per-action approval, source=ORZ_YOLO; benchmark axes off)` |

- 性质：confirm-first **不存在缺省态**（缺省恒 yolo），凡 confirm-first 必显式有源——
  banner 如实反映「谁关的」，与 §14.91「姿态如实反映」对应。
- 现行文案 `[permission] mode=interactive-yolo (default auto-approve; …)` 在 `--no-yolo`
  下失真（347 §2 已登记），S2 以上表替换并同步函数 doc（`main.rs:682-686`；347 档
  「lib.rs:685」系路径笔误，实为 orz-bin `main.rs`）。
- **打印面**：`-p`/`--plan` 启动面照旧（`main.rs:491`）；**新增 Web 启动面同一行 stderr
  回显**（`run_web` 于 `parse_args` 后、server 起听前打印）——Web 是 Windows 日常主入口，
  `--no-yolo` 生效与否必须一处可见；复用同一函数，零新契约。
- **TUI 面**：不做 banner（登记不做，§6）；TUI 姿态由行为与 README 表达。

### 1.5 Web 面 argv 白名单

`orz_web::parse_args`（`orz-web/src/lib.rs:136`）现行「未知参数即报错」——`orz web --yolo`
会被拒。S2 定案：`--yolo`／`--no-yolo` 进 web 解析器**容忍白名单**（识别即吞、不落
WebConfig 字段——姿态已由 orz-bin main 经 env 桥承载，web 侧二次解析即双源漂移温床）；
用法串补两旗。`--real` 的「main 置 env＋web 解析器自留 bool」双识别形态不再复制
（历史兼容面不动，仅新增旗走单源）。

## §2 S2 落码清单（判据随附）

1. **orz-host**：`parse_yolo_env` 新增（值族单测）；`default_initial_yolo` 无参化＋env 单源
   ＋非法值 Err 臂；`spawn_with_hub_and_policy` 注释块 §14.90→§14.91；姿态钉重写
   （`permission.rs:773-796` 现钉矩阵失效）：
   - 缺省 true 四组合钉（windows×活/死面，编译期 cfg 分叉同现钉形态）；
   - env 钉（`ORZ_YOLO=0`→false、`=1`→true、非法→Err）——env 变更测试入 orz-host
     串行门（344 §4 既定串行纪律），用后即清。
2. **orz-bin**：旗扫描＋`＝value`/互斥报错＋env 桥＋启动面 ORZ_YOLO 早校验 exit 2；
   `parse_yolo_flags` 纯函数抽出供单测（`parse_benchmark_flags` 同式，tests 模块钉：
   精确形／`=value` 报错／互斥报错／无旗不动 env）；banner 源感知改写＋doc 勘正；
   `run_web` 启动回显；help 文本补行（`--yolo`/`--no-yolo` 语义与缺省，紧随
   `--allow-*` 行之后）。
3. **orz-web**：`parse_args` 白名单两旗＋用法串；`web_parse_args_tolerates_yolo_flags` 钉。
4. **344 五契约测试第三轮改写**（`*_follows_1490` → `*_follows_1491`，语义回摆非削弱）：

   | 测试（现名） | §14.91 期望 |
   | --- | --- |
   | `session_prompt_mutation_posture_with_dead_gateway_follows_1490`（acp_server.rs:8725） | 改名 `…_follows_1491`：**双平台回归 yolo 自动放行**（dead gateway 交互车道缺省全自动；allow_once＋ToolStarted 在场断言恢复 344 前原语义） |
   | `session_prompt_allowed_mutation_records_snapshot` | 放行来源回归**初始 yolo**（Benchmark 策略臂 344 临时形态退役）；快照时序断言原样保留 |
   | `workspace_write_order_executes_and_journals_following_1490`（codex_app.rs:1642） | `…_follows_1491`：**双平台 0 审批**；「工具真实执行＋落 journal」两臂断言保留；`answer_approvals_until_turn_completed` 助手若再无消费者则随批退役 |
   | `workspace_write_identical_orders_follow_1490_posture`（codex_app.rs:1883） | `…_follows_1491`：双平台 0 审批；双执行断言保留 |
   | `read_only_and_workspace_write_threads_coexist` | 审批断言回归 0/0；read-only 拒绝臂原样保留 |

5. **orz-host 五处注释/doc 三更**：`approval.rs:9`、`permission.rs:8`、`permission.rs:192-198`
   （随 2.1 同批）、`lib.rs:337`、`lib.rs:924-927`——§14.90 条 3 口径改 §14.91。
6. **README 双语姿态注释三更**：zh `README.md:70/102/106`＋`113`（0.9.12 更新行）
   与 en `docs/en/README.en.md:81/113/117/124`——§14.91 姿态（全缺省自动审批恢复＋
   `--yolo`/`--no-yolo`/`ORZ_YOLO` 用户接口）；行文标注「自 0.9.13 载体起」（S3 换装前
   在役 0.9.12 仍 confirm-first，README 主干随工作区走、发行面对齐随 S3）。

## §3 S3 范围（预告，不展开）

Windows-only 载体重建＋换装 bump **0.9.13**（343 先例：Linux 载体与发行留下一自然节点）；
备份 `-bak` 后缀／ACAF 重 provision／清单重生成／doctor 全门／目标工作区活体
（bridge/child 0.9.13、skew=false、零旗启动 banner `source=default`）／用户自行重启
（346 已登记偏好）。§3 过渡期注记（Benchmark 三旗过桥）随 S3 进体失效。

## §4 判据（S2 验收门）

- `cargo test -p orz-host --lib`（串行门）全绿：新姿态钉＋env 钉＋五契约 `*_follows_1491`；
- `cargo test -p orz-bin` 全绿：旗解析纯函数钉（精确形／`=value`／互斥／无旗）；
- `cargo test -p orz-web` 全绿：web 解析器白名单钉；
- clippy 差分无新增；`git diff --check` 干净；
- 真机活体判据随 S3（零旗启动 banner `source=default`＋写单不弹审批；`--no-yolo` 启动
  banner `source=--no-yolo`＋写单弹审批）。

## §5 权威与边界

- 本稿为 §14.91 授权的形态勘定记录；裁决语义零新增。ADR 不随本批改动（§14.91 已内嵌
  候选形态并显式委派 S1）。
- 现行法时序：§14.90 条 3（2026-10-23～10-25 过渡现行）→ §14.91 取代其缺省面（2026-10-25
  即日现行）→ S2 落码 → S3 进体（0.9.13 在役后 §14.91 全量生效于装机面）。
- 不做清单（本设计显式排除）：Web 工作台审批模式开关（§6 独立候选）；TUI 启动 banner；
  `remember_tool_approvals` 持久授予面；headless `--no-yolo` 的 Ask→Deny 契约变更
  （既有 fail-closed 不动）；审批超时（300s fail-closed）任何变更。

## §6 Web 工作台开关——独立候选登记（不入 S2）

§14.91 条 2 将 Web 开关列为「可选增项随 S1 裁决」。S1 裁决：**降级为独立候选项
（`candidate`，条件触发），不入 S2**。理由：

1. 裁决核心诉求（缺省全自动＋用户命令接口）已由旗＋env 满足；Web 开关是锦上添花，
   不在用户令三裁决的字面与意图内；
2. Web 开关＝新契约面：`POST /api/permission-mode`（或 `_orz` 扩展）→ 桥对在途会话
   `set_yolo_mode` ＋ UI 开关态与多端 attach 一致性＋12 面基线 S8（审批）修订——
   体量与契约风险独立成批，塞入 S2 会拖 0.9.13 换装节点；
3. 日常切姿已有零成本通路（启动旗／env），运行中切姿属罕见需求。

触发条件（满足其一才升级）：用户明确裁决要运行中切姿；或 0dn S8 面修订批顺带承载。
登记不动计数（`candidate` 口径，BACKLOG 条件触发区）。
