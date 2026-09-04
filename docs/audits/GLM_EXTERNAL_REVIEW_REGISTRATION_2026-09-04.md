# GLM 外部只读审查登记（2026-09-04）

> **登记日期**：2026-09-04
> **来源**：用户提供的第三方外部只读审查报告（原文件：
> `C:\Users\1\OneDrive\Desktop\ORZ_全面审查报告_2026-09-04.md`；注意：用户
> 给出路径含目录段，实际文件位于 Desktop 根目录）。审查对象为提交
> `d3b6a9c`（父仓 main）+ `a88a566f`（orz feat/fusion-architecture），即
> Task D batch-1 之前的 HEAD。
> **登记性质**：本文件只登记经本窗口逐项复核的候选发现与其处置状态；
> 不实施修复、不排期、不把第三方结论当作当前事实（复核为准）。

---

## 1. 复核结论表

| 编号 | 原级别 | 复核结论 | 证据（本窗口复测） | 建议处置（待用户裁决） |
|---|---|---|---|---|
| F1 | P2 | **属实**：skills 沙箱豁免面过宽 | `orz-tools/src/types/resources.rs:503` `is_skill_markdown`：任何名为 `SKILL.md` 的文件或路径含（大小写不敏感）`skills` 目录组件的 `.md`，在 `is_path_within_workspace`（`:549`）里先于 canonical 包含判定直接放行；路径由模型提供，等价于可构造的越界只读通道 | 收窄为注册表式技能根目录豁免（如 `.agents/skills`、`~/.claude/skills`、`~/.codex/skills`） |
| F2 | P2 | **属实**：交互审批器是未登记存根 | `orz-host/src/approval.rs` 全文件为注释 + `// TODO: Implement approval prompter (Phase 1)`；`orz-host/src/lib.rs:313` 注明 "approval path (still a stub)"；索引/BACKLOG/TODO 无该存根的 gap 登记（仅存在同名无关的既有 flaky 测试条目） | 补登记为 gap（索引 + BACKLOG） |
| R-1 | P2 | **属实且范围更大**：`.gitignore:47` 新规则与已跟踪文件矛盾 | `git check-ignore -v --no-index _windows_high_nist/vm-acaf-reprovision-result.txt` → `.gitignore:47:_windows_high_nist/**/vm-*-result*.txt`（exit 0）；`git ls-files` 显示 `_windows_high_nist/` 下数十个 `vm-*-result*.txt` 仍被跟踪 | `git rm --cached` 转本地件，或收窄 ignore 模式 |
| R-2 | P3 | **属实**：manifest 全 CRLF | `orz_source_manifest.sha256` 1437 行全部 `\r\n`；`scripts/generate_orz_source_manifest.py:113` 用 `write_text("\n"...)`，Windows 文本模式自动转 CRLF，POSIX `sha256sum -c` 无法直接整文件校验 | 生成脚本显式以 LF（`newline="\n"` 或二进制）写出 |
| R-3 | P3 | **属实**：根目录一次性实验产物被跟踪 | `git ls-files` 命中 `LIF_102RUNS_*.json`×5、`_final_smoke_*.{json,ps1}`、`container-probe-2026-08-20.sh`、`gsa.py` | 归档至 `存档/` 或移出跟踪 |
| F3 | 已知项 | **口径可复现**：Python 参考实现规模约 6.6×Rust | 独立复算：`assurance/` 下 `*.py` = 189 文件 / 94,507 行（GLM 口径一致）；Rust `orz-assurance` 约 1.43 万行 | Task D batch-1（schema/链级法官 + 单源映射）已落地；S2 机械族盘点/排期按用户指示暂缓 |

## 2. 观察项（非缺陷，登记备查）

- **(a) TOCTOU**：沙箱判定与实际读取之间存在理论符号链接换向窗口；本地单用户威胁模型下可接受（与 Task C 边界登记一致）。
- **(b) vendored fork**：`third_party/async-openai` 为仓库内 vendored fork（DeepSeek reasoning_content 回放），来源有 README 说明。
- **(c) 权限判定分散**：移植层 `orz-workspace/permission/manager.rs`（8,756 行，复核存在）与自研 `orz-host/permission.rs`（1,331 行，复核存在）双处承担权限判定，长期是复杂度与审计盲区风险——建议与任务 D 一并纳入终局治理视野（另立观察，不入任务 D S2 盘点）。
- **(d) VM 密钥明文落盘**：VM 内 `ds-key-stage.txt`/`agent-key.txt` 明文暂存 + 运行后 scrub + hash 双向核对，属已知设计，风险受控。

## 3. 处置状态

- 上表 F1/F2/R-1/R-2/R-3 均为**候选登记，待用户裁决处置**；本批不实施修复，也不改写既有 BACKLOG/TODO 优先级（按用户指示：先不进行盘点与排期）。
- GLM 报告对门禁全绿、Task C 沙箱、ACAF fail-closed、journal 链、IPG 等的正向核验与既有审计一致，不重复登记。
- 后续如用户裁决，将逐项转 BACKLOG 对应优先级小节并附本登记为入口证据。
