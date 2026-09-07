# 0p S2 全面复审与处理批（2026-09-07）

> **范围**：orz `7d7d89e7`（0p S2 两段门 + 权限门拒绝信封 + key 拦截）
> 对照设计 [`BLACKBOARD_SELF_HISTORY_AND_GSA_TWO_STAGE_GATE_DESIGN_2026-09-07`](../BLACKBOARD_SELF_HISTORY_AND_GSA_TWO_STAGE_GATE_DESIGN_2026-09-07.md)
> （ADR-0010 §14.61）的全面复审——设计合理性 / 实现合理性 / 设计-实现
> 符合性三面；三路子代理并行深审（两段门 / 拒绝信封事件面 / key 拦截）
> + 载荷性证据主车道亲核。**处理**：用户裁决「P1-2 先补实现，其余按建议
> 直接修复」——全部 P1/P2 同批修复闭合。

## 1. 复审结论（修复前）

机制本体（工具层两段门语义、四漏斗工程、结构化 seam、逃逸恒拒、
fail-closed 方向、审计对落 journal、schema 同步）实现质量高、与实施
注记①–④逐条对应；但作为端到端系统被一个结构性事实压倒：

- **P1-a（两段门生产不可达）**：生产调用链上权限桥 `permission.rs
  access_in_scope` 的冻结镜像（`!path_under(gsa_root) && !path_under
  (&gsa_canon)`）在工具执行**前**把 `.gsa` 内部区读恒拒（permission.rs
  :385-390，测试 permission.rs:779-790 反向锁定 Deny）；主车道全部 direct
  调用 `permission_gated = !lane_self_execute`（agent_loop.rs:2110）必过
  桥（host_exec.rs:734）。桥 deny 走 host_exec.rs:756-843 的**无
  ToolCompleted** 拒绝路径（注释原文明示 event-less 契约）→ 两段门
  （通知→放行）与 D-3 事件面闭合在带桥生产路径（CLI -p / ACP）不可达，
  「D-3 闭合」仅对 outside_workspace 子集与绕桥测试通道成立；S4 判据 2
  按修复前代码不会发生。根因：设计 §14.61 修订 §14.56 时只写了工具层
  单点，未点名桥镜像需同步让路（设计文本的结构性遗漏）。
- **P1-b（key 不落卷不变量未闭合）**：穷举 `.gsa` 卷内持久化写入面，
  四漏斗之外尚有五条未接脱敏的落卷路径——`run_tests_output.txt`（裸
  `fs::write`，恒直读窗口）/ `runs/<run>/retrieval-results/*.json`（与
  journal 同目录绕过漏斗 1）/ `compaction/*.md`（会话快照最大体量落卷
  面，冻结台账原文以内存数据重现）/ `blackboard/epoch-*.json` /
  `grill/*.jsonl`——设计判据「含 key 命令 → 全卷零 sk-」可被证伪；
  另「已知 key 值字面替换」未实现、B1 凭据永久拒类零实现（卷内既有
  keystore//one_shot_permit//grok-home/ 通知后即事实放开）、终端日志
  崩溃/`shutdown_all` 路径永不 sweep 且 sweep IO 失败静默 fail-open。
- **P2**：schema 两处描述漏 `session_volume_agent_invisible`；
  `session_volume_opened` 描述「first post-notice access」与实现「每次
  放行置位」（B4 字面）口径冲突；grep 带 path 的 deny 被 S1 F-C 误盖
  `failure_agg exit_1` 章；共享瞬态旗标在并发批（join_all）下跨调用
  错配/丢失；结构预览缺 B2 明文的轮次范围/条目数；`search_replace` 对
  `.gsa` 无机械写守卫；未开门时 workspace 根 `--hidden` grep 可遍历
  直读内部区（先存旁路）；grep/list_dir 两段门零测试；全链（桥→工具门
  →journal 载荷）无贯穿断言。
- **P3（登记不修/顺带修）**：`access_state.json` 非原子写（损坏 fail 向
  重通知、方向安全）；通知 = status=error + LIF Deny 与硬拒绝同形（符合
  裁决「与失败一致」，S4 软判据观察）；Ok 臂 policy_denial 冗余双写
  （本批已删）；设计 §3.C 文本 `source=permission_gate` 与实现/schema
  `permission` 不一致（schema 为权威，设计文本瑕疵）；判决 5 态而非
  「四态」措辞；T0 bump 0.3.2 未做（随本批补做）。

## 2. 处理批内容（本批 orz 提交：`542c35d5` 修复批 + `7b00bbc9` T0 bump）

### P1 修复

1. **桥镜像让路（P1-a）**：`access_in_scope` else 臂改放行（判定权单点
   归 orz-tools 两段门），doc 注记随批退役「等义镜像继续拒绝」段；窗口
   幽灵形态守卫（terminal_log/run_tests 臂 canonical 双条件）原样保留；
   permission.rs 测试对齐（内部区/`..` 折叠/`.bak` 形态 → AllowOnce，
   指向工具层单点执法）；新增 `bridge_yields_internal_reads_and_envelope_
   lands` 全链测试（带桥 host：桥 Allow → 首读通知信封 policy_denial
   code=session_volume_notice exit 1 → 二读放行 + session_volume_opened）。
2. **第 5 漏斗 + 已知 key 注册表 + 凭据恒拒（P1-b）**：
   - `run_tests_output.txt`：文本终稿处统一 `redact_secrets`（落盘与
     对话尾窗同源）；回归 `run_tests_output_scrubbed_of_secrets`。
   - retrieval-results 工件 / compaction 存档（write_archive_retry）/epoch
     归档（restore 为退役诊断面，接受脱敏失真）/grill 两写点：写盘点接
     redact。
   - orz-secrets 增 `register_known_secret` / `register_known_secrets_
     from_env`（8–512 字符、无路径分隔符启发式；注册表空时行为与纯
     shape 检测一致零误伤）；`read_agent_api_key` 三平台成功路径显式登记
     模型 API key（覆盖 Windows 凭据管理器通道）；`OrzHost` 装配期 env
     扫描登记。
   - B1 凭据区落判决：`SessionVolumeReadVerdict::CredentialsDenied`
     （keystore/one_shot_permit/grok-home/chrome-profile*，词法或
     canonical 任一命中即拒、通知不放开），三读工具接线 + 通知信封增
     凭据职责句；`access_state.json` 机制文件按 B3 直读（WindowAllowed）。
   - `shutdown_all` kill 后补 `flush_and_truncate_output_file`；sweep 读/
     写 IO 失败 `tracing::warn!` fail loud。

### P2 修复

- 旗标按 call-id 键控（`SessionVolumeCallFlags` 表 + `finish_call` 清理 +
  容量上限兜底 Err 路径）；host `call_tool_inner` 以 call_id 取旗标。
- host_exec：policy_denial 载荷不盖 `failure_agg exit_{n}` 章（grep 带
  path 的 deny 不再污染聚合面）；删除冗余双写块。
- `search_replace` 增 `.gsa` 会话卷写守卫（词法或 canonical 落域即拒，
  InvalidInput 形态同 gitignore 拒编）。
- grep 未开门时追加 `**/.gsa/**` exclude glob（与 Managed Read-deny 同
  机制），开门后内部区属已放行面不再排除。
- 结构预览补台账三要素：条目行数 + 轮次范围（行首 `轮次 N` 盖章流式
  扫描，20000 行上限）+ 字节。
- schema 描述修正：三 code 齐备；opened 口径 = 每次放行置位（B4 字面）
  + 凭据区/机制文件不入置位面。
- 测试补全（+14）：read_file×3（坏状态重通知 / 通知后逃逸恒拒 / 凭据区
  通知后仍拒 + access_state 直读）、grep×3（两段门通知→放行 / 未开门
  遍历排除→开门放行 / 凭据区恒拒）、list_dir×2（两段门 / 凭据区）、
  search_replace×1（写守卫 + workspace 不误伤）、orz-host×3（全链桥让
  路 / 并发旗标归属 / run_tests 脱敏）、orz-assurance×2（三形态载荷直调
  法官族 + v0.2 schema 校验 const-true）。

## 3. 验证

- orz-tools **2844** passed / 0 failed（+9）；orz-host **254** / 0
  （+3，串行）；orz-loop **745** / 0；orz-assurance **207** / 0（+2）。
- schema JSON 合法性校验通过；设计文档注记 ⑤⑥⑦ / BACKLOG 0p / TODO
  P0-0p S2 勾选随批。
- T0 bump 0.3.2 随批补做（orz `7b00bbc9` 独立提交，排期顺序恢复：S1/S2
  代码先于基线版本落地的偏差纠正）。

## 4. 边界与遗留登记

- **snapshots/**（IP5 预变更工作区字节副本）：脱敏会破坏 restore 字节
  完整性，与「全卷零 sk-」存在结构性张力——登记为已接受边界（工作区
  自带密钥被改动时随副本入卷）；grok-home/ 为外部 CLI 写手，同属边界。
- **桥 deny 其余路径仍 event-less**（写域 deny / shell deny 等）：本批
  D-3 闭合面 = 读沙箱三 code + 两段门通知/放行；其余拒绝路径的信封化
  归 0n（GAP-APPROVAL-PROMPTER）词汇统一时一并处理。
- **sweep 崩溃窗口**：进程硬崩时终端日志来不及 sweep（B6 同族的已接受
  残余风险，S4 判据 3 以收尾 sweep 后的全卷扫描为准）。
- **通知教育信息会话内生存 vs notice_shown 卷级永续**的张力：新会话首读
  即得内容、未见职责图——S4 观察项（判据 2 附带记录首读行为）。
