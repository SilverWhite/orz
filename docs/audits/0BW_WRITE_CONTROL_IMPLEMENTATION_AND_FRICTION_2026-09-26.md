# 0bw 写入管控：实施实况与摩擦（2026-09-26 批）

> **批号**：0bw「写入管控：机械层收窄锁死＋命令审查留痕＋补偿自检」。
> **执行形态**：S1 设计档先行＋CFA audit 探针（非提权可达面）＋S2 v1 落码（工具面＋命令面＋载体自保护）＋措辞面（SECURITY/README）。
> **边界**：**不提交、不推送、不重建**（用户令 2026-09-26）；台账/TODO/BACKLOG 不动（随落账批同步）。
> **设计权威**：[`docs/WRITE_CONTROL_MECHANICAL_DESIGN_2026-09-26.md`](../WRITE_CONTROL_MECHANICAL_DESIGN_2026-09-26.md)（design v1.0，§0–§10）。
> **三条底线**：①读写对齐 ②危险命令机械审查＋系统盘核心删改锁死 ③载体不可破坏（硬）。**allowlist 不做**；定位＝**高阻力＋强审计**（保证／阻力／审计三档措辞，非绝对保证）。

## §1 摘要

- **S1 设计档已落**：deny 单一源表（Win/Linux/载体 C1–C4）、匹配语义（谓词前缀剥除／近祖先 canonical／fail-closed／env 回退）、三落地（工具面/命令面/进程面）、L2 规则表、CFA 探针方案、L4 补偿、0z 边界分工表、判据钉清单、边界声明。
- **S2 代码 v1 已落**（orz-tools：2 新文件＋5 文件接线；含 install_dir ⊆ cwd 的**文件级降级规则**）。
- **测试读数（全绿，0 failed）**：`write_control` 过滤 12 passed（含 2 个 bash 接线钉）；`exec_policy` 5 passed；`search_replace` 过滤 136 passed（含 2 个新钉＋既有 `.gsa` 断言保持）；`bash` 过滤 233 passed（含 2 个新钉）。
- **CFA 探针**：非提权读数完整可达（Defender Operational 日志非管理员可读；1124 现为空）＋runbook 脚本 `.tmp-0bw-cfa-audit.ps1`；enable/revert 需提权，本轮**不触发 UAC**（裁决 D2）。
- **措辞面**：`orz/SECURITY.md` 重写（三档措辞＋边界；HackerOne 渠道保留）；父 `README.md` §机械层增「写入管控」行。
- **门禁读数**：`python scripts/generate_orz_source_manifest.py --check` → `valid`（脚本按 **HEAD blob** 比对，工作树未提交改动不触发 drift；落账批 pin 新提交后需重生成）。
- 裁决与摩擦清单见 §4–§5；未竟见 §6。

## §2 交付物一览

| # | 文件 | 状态 | 说明 |
|---|---|---|---|
| 1 | `docs/WRITE_CONTROL_MECHANICAL_DESIGN_2026-09-26.md` | 新增（未提交） | S1 设计档 v1.0（118 行，§0–§10） |
| 2 | `orz/…/orz-tools/src/types/write_control.rs` | 新增 | deny 表＋`check_write_target`／`write_block_message`＋10 测试（461 行） |
| 3 | `orz/…/orz-tools/src/types/exec_policy.rs` | 新增 | L2 三分类＋tokenizer／程序位／短语／路径抽取＋5 组表驱动测试（832 行） |
| 4 | `orz/…/orz-tools/src/types/mod.rs` | 修改 | 注册 `exec_policy` / `write_control` |
| 5 | `orz/…/orz-tools/src/types/resources.rs` | 修改 | `candidate_is_under` → `pub(crate)`（复用读面同族语义，不复制实现） |
| 6 | `orz/…/implementations/grok_build/search_replace/mod.rs` | 修改 | 写前检查收敛为统一单点（原 `.gsa` 块被替换；**会话卷文案保持**）＋2 新测试 |
| 7 | `orz/…/implementations/grok_build/bash/mod.rs` | 修改 | run() 预检（block/warn）＋warn 三处落痕（summary×2＋前台 BashOutput 头部行）＋2 新测试 |
| 8 | `.tmp-0bw-cfa-audit.ps1`（父仓根） | 新增 | CFA runbook 脚本（`-Report` 免提权；`-EnableAudit`/`-Revert` 需管理员） |
| 9 | `orz/SECURITY.md` | 修改 | 重写（37 行）：机械层概览＋写入管控三档＋边界＋报告建议 |
| 10 | `README.md`（父仓） | 修改 | §机械层「安全」后新增「写入管控」行（三档措辞＋链接） |
| 11 | `docs/audits/0BW_WRITE_CONTROL_IMPLEMENTATION_AND_FRICTION_2026-09-26.md` | 新增 | 本报告 |

git 快照（本轮末，未提交）：父仓 `M README.md`／`m orz`／`?? docs/WRITE_CONTROL_MECHANICAL_DESIGN_2026-09-26.md`；orz 仓 `M SECURITY.md`、`M bash/mod.rs`、`M search_replace/mod.rs`、`M types/mod.rs`、`M types/resources.rs`、`?? types/exec_policy.rs`、`?? types/write_control.rs`。

## §3 实施实况

### 3.1 S1 设计档（§0–§10 概览）

一句话设计（§0）：机械层持有一张**单一源 deny 常量表**，在三处消费（工具面/命令面/进程面）；写层不做 allowlist，锁死面之外真机直通。§1 形态总纲 L0–L4 收窄版；§2 deny 表（W1–W4／Linux 12 项／C1–C4＋降级规则＋匹配语义＋D3 编译内置）；§3 三落地；§4 L2 规则表（3 block ＋ 2 warn；默认 allow）；§5 CFA 探针；§6 L4；§7 0z 边界分工；§8 判据钉；§9 边界声明；§10 交付记。

### 3.2 S2 代码（要点）

- **工具面**（`search_replace`）：写前统一单点 `write_control::check_write_target`——C1 会话卷域（**委派**既有 `is_path_in_session_volume_domain`，语义不重写、文案保持 `not model-writable`）→ C2/C3 安装目录（`current_exe()` 父目录；**install_dir ⊆ cwd 时降级为文件级**：三件套 `orz`/`orz-signer`/`orz-acaf-provision`＋`grok-home`）→ A/B 系统核心（env 缺失回退字面默认；`\\?\`／`\\.\` 剥除；近祖先 canonical）。
- **命令面**（`run_terminal_cmd`）：既有机械验证链（后台算子/pkill 自匹配）之后、`--- Prefix ---` 与执行分派之前插入 `exec_policy::review_command`。
  - `Block` → `ToolError::permission_denied`（机械文案含规则 id；**命令不执行**）；
  - `Warn` → 执行照常＋结果头部 `[写入管控·提示]` 行（显式后台 summary／auto·user 后台 summary／前台 `BashOutput` 的 `output`＋`output_for_prompt`＋`total_bytes` 三处同步）；
  - `Allow` → 零改动。
- **实现注记**：`format_default_prompt` 以 `output_for_prompt` 优先（warn 行须在重排前注入——见摩擦 F2）；L2 匹配＝程序位词元精确＋短语（`-Command`/`/c` 等内容位递归展开一层）；命令内路径 token 过的**同一 deny 表**（单一源）；`unicode_confusables::normalize_confusables` 仅作扫描归一（不承载安全语义）。
- **测试读数**（本轮过滤运行，`cargo test -p orz-tools --lib <filter>`）：
  - `write_control` → **12 passed / 0 failed**（2961 filtered；含 bash 侧 `write_control_blocks_safety_flip_command`、`write_control_warn_header_attaches`）。
  - `exec_policy` → **5 passed / 0 failed**（正向放行集＋逐规则负向集＋文案断言）。
  - `search_replace` → **136 passed / 0 failed**（含 `search_replace_refuses_system_core_targets`、`search_replace_refuses_install_dir_targets`「拒绝后零存在」钉；既有会话卷测试保持绿）。
  - `bash` → **233 passed / 0 failed**。

### 3.3 CFA 探针（非提权读数＋runbook）

本机读数（2026-09-26，非管理员；原文见 §8）：

```
(Get-MpPreference).EnableControlledFolderAccess = 0（Disabled）
(Get-MpPreference).ControlledFolderAccessProtectedFolders = (none)
Get-MpComputerStatus：AMServiceEnabled=True / AntivirusEnabled=True / RealTimeProtectionEnabled=True
                     / AMRunningMode=Normal / AMServiceVersion=4.18.26080.4
Defender Operational 日志：IsEnabled=True / RecordCount=2436 / LastWriteTime=2026-09-26 20:10（非提权可读）
1124 查询（非提权）：可执行 → “找不到任何与指定的选择条件匹配的事件”（当前为空——符合 CFA 从未 enable）
IsAdmin = False
```

- **结论（D2 裁决重申）**：enable 需交互提权（本会话不触发 UAC、不改系统状态）；audit 轮与 1124 目标分布读数 → enforce 翻转由用户/后续批裁定。
- **runbook 脚本**：`.tmp-0bw-cfa-audit.ps1`（`-Report` 免提权；`-EnableAudit`＝`Set-MpPreference -EnableControlledFolderAccess AuditMode`；`-Revert`＝`Disabled`；含 1123/1124 读取与目标路径建议）。
- **探针可用性发现**：Defender Operational 日志与 1124 查询**非提权可达**——enforce 决策的数据面不依赖常驻管理员会话（仅翻转动作需提权）。

### 3.4 措辞面

- `orz/SECURITY.md`：HackerOne 报告渠道保留；新增「安全模型概览（机械层）」「写入管控（0bw）——能力与边界」三档措辞（保证＝工具面硬拒＋命令面 block；阻力＝best-effort 闸清单与不保证面；审计＝journal 留痕与 `--replay`）＋「设计就绪、待落地」清单（CFA enforce／Landlock／运行态自检）＋报告建议。
- 父 `README.md` §机械层：新增「写入管控」行（锁死面摘要＋block/warn 行为＋三档定位＋设计档与 SECURITY.md 链接）。

### 3.5 门禁读数

`python scripts/generate_orz_source_manifest.py --check` → **valid**（EXIT=0）。脚本实现＝`git -C orz ls-files`＋`git cat-file --batch HEAD:<path>`（canonical blob）——**工作树未提交改动不触发 drift**；落账批（pin 新子模块提交）需重生成 manifest（现有 1503 条 → 新计数）。

## §4 裁决记录

### 4.1 范围与形态（D1–D6；用户令内既有裁决，重申存档）

- **D1 范围**：S3（载体重建）与 S4（真机复验）不可达（无重建/staged 形态）→ 本轮做 S1 设计档＋CFA 探针（可达部分）＋S2 落码（工具面＋命令面＋载体自保护）＋README/SECURITY 措辞＋报告。
- **D2 CFA 探针**：**不触发 UAC**、不改系统状态；enable 步骤入 runbook；enforce 翻转待狗粮 1124 读数后由用户/后续批裁定。
- **D3 管控配置**：编译内置常量——无运行时配置文件、无 escape hatch；配置自身保护＝载体二进制（C2/C3）＋manifest 校验。
- **D4 载体自保护**：install dir 子树；**install_dir ⊆ cwd 时降级为文件级**（三件套＋`grok-home`）；`.gsa` 域经既有判定复用。
- **D5 L2 留痕 v1**：block＝机械拒绝文案（随 `ToolError` 入 journal）；warn＝结果头部 `[写入管控·提示]` 行；专用事件族（schema＋verifier＋fixture 全链）列为后续扩展。
- **D6 平台面**：Linux Landlock＝仅设计注记（无 Linux 实测面，避免不可验证代码）；L4＝既有回退窗口（0bm⑦）＋设计。

### 4.2 本轮执行中新增裁决（留痕）

| # | 事项 | 裁决 | 理由/备注 |
|---|---|---|---|
| X1 | block 的 `ToolError` 形态 | `permission_denied`（403 形态） | 语义正确（策略拒绝）；既有工具内无同型先例，取「权限」语义；文案含规则 id 供审计 |
| X2 | 未存在面路径解析 | **近祖先 canonical**（上溯首个存在祖先 canonical 化＋余段词法拼接） | 覆盖 8.3 短名等仅存在面可解形态；解析失败回退原样（fail-closed：不因解析失败放行） |
| X3 | Unicode confusables | 仅作扫描归一辅助，**不设安全闸语义、不扩表** | 既有表为排版类字符；同形字绕过登记为已知边界（设计 §9） |
| X4 | elevation warn 噪音 | 保留（v1 不做去噪） | 留痕优先；狗粮观察噪音实况后再裁定 |
| X5 | 台账/TODO/BACKLOG | **不动**；报告即留痕 | 用户令「报告即可」；落账批一并同步 |
| X6 | REV-083-06（journal「防篡改」口径） | 对外措辞统一**降口径为「完整性/损坏检测」** | 与本批新增 SECURITY/README 行同向（未出现绝对化表述）；存量行如出现更强表述建议落账批复核（未动） |
| X7 | exec_policy 重复测试模块事故 | 剪切至首个 `#[cfg(test)]` 后**单次重挂** | 编译期 E0428（双 `mod tests`）；如实记录（摩擦 F3） |

## §5 摩擦记录

- **F1（勘定新增规则）install_dir ⊆ cwd 降级**：整树拒写在「源码树内运行/构建输出即安装目录」形态下会**封锁工作区**（如 `orz/target/release` ⊆ `orz/`）。处理＝降级文件级（三件套＋`grok-home`）；**残余风险（DLL 侧植等）不保证**——已入设计 §2.1/§9。
- **F2（接线发现）warn 头部行三处同步**：`format_default_prompt` 以 `output_for_prompt` 优先（非从 `output` 重排），故 warn 行须在重排前注入 `output_for_prompt`，并同步 `output` 字节与 `total_bytes`（否则 shown/total 失配）。三处（前台／显式后台 summary／auto·user 后台 summary）逐一接线。
- **F3（操作事故）测试模块重复插入**：会话压缩窗口打断使一次 append 的成功回执未达，重复 append → E0428（同文件双 `mod tests`）。修复＝PowerShell 剪切至首个 `#[cfg(test)]` 后单次重挂（无残留）。教训：工具回执中断后应先核对文件再重试。
- **F4（预期修正）manifest drift 误判**：原以为「未落账新文件 → manifest drift」；实测脚本按 **HEAD blob**（`cat-file HEAD:<path>`）比对，工作树改动不触发 —— `--check`＝**valid**。落账批重生成即可。
- **F5（可达性）CFA enable 需提权**：本会话非管理员（`IsAdmin=False`）。发现：**读数面不依赖提权**（Operational 日志可读、1124 可查、偏好可读）；仅翻转动作需提权（D2）。
- **F6（平台面）Landlock 无实测面**：本机无 Linux；避免不可验证 unsafe 代码，仅设计注记（D6）。
- **F7（环境）PowerShell 5.1 非 ASCII 乱码**：控制台输出中文需每命令置 `[Console]::OutputEncoding = UTF8`；未置时读取/回显出现乱码（本次多轮已规避）。
- **F8（规模）编译与测试体量**：orz-tools lib test ≈2969 用例；全量编译约 3m50s（增量 ~20s）。以过滤运行（`--lib <filter>`）控制成本；本 run 出现一次内存 commit 软提示（5.15GiB 线），动作照常。
- **F9（噪音风险）warn 条款**：Linux 容器常见流程（`sudo …`、`rm -rf /tmp/…` 类绝对路径递归删除）会触发 elevation / broad-destructive warn——best-effort 闸的既有代价；块级规则不受影响。
- **F10（边界）路径形态限定**：引号内复合命令形态（拆片兜底）、未存在 8.3/subst/junction、UNC、脚本内命令/别名/`.NET` 直调不保证覆盖——设计 §9 已声明，测试面以正向放行集护栏（`rg \"set-mppreference\" docs` 不误伤等）。

## §6 未竟与后续

- **S2 余项**：无——v1 计划内（工具面＋命令面＋载体自保护＋降级规则）全落且测试全绿。
- **后续批（登记）**：①Linux Landlock 落码（复用 `orz-sandbox` 或直写 syscall 的决策点随批）；②运行时载体完整性自检（发布物 manifest＋启动校验；涉发布/重建链）；③专用 journal 事件族（block/warn 结构化）；④git 检查点/journal undo（0bw L4 余项）；⑤CFA enforce 翻转裁决（待 dogfood 1124 读数，脚本已备）；⑥载体重建后真机复验（S3/S4）。
- **落账批（本批禁止项）**：提交/推送；`orz_source_manifest` 重生成；台账/TODO/BACKLOG 状态同步；`dist`/发布备注。
- **狗粮观察项**：warn 噪音实况；`[写入管控·提示]` 在 TUI/journal 的渲染与截断面；命令面 block 触发的实际姿态（合规流程是否被误伤）。

## §7 判据钉对照（设计 §8 → 本轮读数）

| # | 判据钉 | 读数 |
|---|---|---|
| 1 | `write_control` 表命中/放行 fixture（前缀剥除/近祖先 canonical/大小写折叠/env 回退） | `write_control` 10 测试 passed（`windows_roots_hit_and_boundaries`、`windows_case_folding_and_verbatim_prefix_stripping`、`linux_roots_hit_and_boundaries`、`system_core_roots_env_fallback_binds_literals`、`install_dir_*`×2、`session_volume_*`、`check_write_target_reports_system_core_rule`、`best_effort_canonical_*`、`relative_under_folds_case`） |
| 2 | `.gsa` 委派等价（既有测试保持绿） | `search_replace` 过滤 136 passed，含 `search_replace_refuses_session_volume_targets`（文案 `not model-writable` 原文断言） |
| 3 | `exec_policy` 三分类 fixture（正向放行集＋逐规则负向） | `exec_policy` 5 组表驱动 passed（dev 例行 11 条放行；flip 16 条；system-core/carrier 11 条；warn 5 条；文案 1） |
| 4 | 接线钉：拒绝后零存在／block 不执行／warn 头行 | `search_replace_refuses_system_core_targets`、`search_replace_refuses_install_dir_targets`（零存在断言）；`write_control_blocks_safety_flip_command`（block 文案＋规则 id）；`write_control_warn_header_attaches`（头行＋`output`/`output_for_prompt`）——全部 passed |
| 5 | 文案钉（规则 id＋目标路径） | `block_message_and_warn_line_carry_rule_identity` passed |

## §8 附：命令读数原文（节选）

**CFA 非提权读数**（2026-09-26）：

```
=== CFA state ===
EnableControlledFolderAccess = 0
=== protected folders ===
(none)
=== computer status ===
AMServiceEnabled          : True
AntivirusEnabled          : True
RealTimeProtectionEnabled : True
AMRunningMode             : Normal
AMServiceVersion          : 4.18.26080.4
=== defender log reachability ===
LogName       : Microsoft-Windows-Windows Defender/Operational
IsEnabled     : True
RecordCount   : 2436
LastWriteTime : 9/26/2026 8:10:41 PM
=== 1124 read attempt ===
1124 read: 找不到任何与指定的选择条件匹配的事件。
=== admin? ===
False
```

**测试读数**（节选原样）：

```
write_control 过滤：test result: ok. 12 passed; 0 failed; ... 2961 filtered out
exec_policy 过滤：test result: ok. 5 passed; 0 failed; ... 2964 filtered out
search_replace 过滤：test result: ok. 136 passed; 0 failed; ... 2837 filtered out
bash 过滤：test result: ok. 233 passed; 0 failed; ... 2740 filtered out
```

**manifest 校验**：

```
python scripts/generate_orz_source_manifest.py --check → valid (EXIT=0)
```

**git 快照（未提交）**：父仓 `M README.md / m orz / ?? docs/WRITE_CONTROL_MECHANICAL_DESIGN_2026-09-26.md`；orz 仓 `M SECURITY.md, M bash/mod.rs, M search_replace/mod.rs, M types/mod.rs, M types/resources.rs, ?? types/exec_policy.rs, ?? types/write_control.rs`。

**附注**：本轮为**审计留痕批**（不提交/不推送/不重建）；后续落账批须完成：子模块提交与 pin、manifest 重生成、台账/TODO/BACKLOG 同步、狗粮读数（CFA audit 轮）后再裁 enforce。

## §9 处置回执：F7 定案 B 方案并收口（2026-09-26 主会话批，086）

用户裁决：「机械层改模型的命令确实是一大风险，读取侧转码确实更好，就按照B方案做吧」。

- **实证澄清（journal 复算）**：本轮 94 个带 `output_encoding` 的 `tool_completed` 标签分布＝utf-8×88／utf-8-sig×1／**gb18030×5**，工具结果**零乱码**样本——**B 方案主链（terminal 后端 → `decode_text` 梯：UTF-8 严格 → GB18030 → 行粒度 lossy＋显式占位＋降级标签）在 0.7.0 捕获侧已在役并实际咬合**（GAP-ENCODING-GATE＋0z D＋0as 资产）；本报告 F7 所记模型侧 30+ 次 `[Console]::OutputEncoding` 前缀属习惯性规避，journal 中未观察到实际乱码结果。
- **收口三处残余 lossy 角落（树面，未提交）**：① `bash/mod.rs format_default_prompt` 兜底臂 `from_utf8_lossy` → `decode_text` 梯（提示词回退不再产 U+FFFD）；② `orz-host/project_doc_index.rs` 命中内容预览（`include_content`，16KB 截断片）→ 梯；③ 同文件两处标题抽取（`extract_headings` 与 256KB 头读路径）→ 梯。
- **钉子两枚**：`default_prompt_fallback_decodes_gb18030_not_replacement_chars`（GBK 字节 → 提示词含「中文」且零 U+FFFD）；orz-host `gbk_headings_decode_via_capture_ladder`（GBK 字节标题 → `中文标题`）。读数：orz-tools `default_prompt` 组 **10/0**、`gb18030` 组 **11/0**；orz-host 过滤 **1/0**；fmt 触碰面干净（`read_file/mod.rs` 三处为既有 F11 rustfmt 版本噪声面、本批未触碰）；clippy 触碰面**零警告**（树内既有警告＝0bw 轮 `exec_policy`×2／`search_replace`×3 等，随落账批复核基线）。
- **边界如实登记**：PS 5.1 **内部误读**（`Get-Content` 对无 BOM UTF-8 按 ANSI；`$out = 原生UTF8工具` 按 `[Console]::OutputEncoding` 捕获再回排）发生在字节到达 orz 之前，**读取侧转码不可达**——由模型侧习惯（本轮实证有效）与工具描述纪律（0bt ③ 面）承接；capture 侧不猜「干净 GBK 解码文本本身是否已是双跳产物」。

