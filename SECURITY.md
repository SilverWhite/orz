# Security Policy

## 报告渠道

安全漏洞请经 HackerOne 计划报告：

https://hackerone.com/x

请勿为安全报告开设公开 GitHub issue。

## 安全模型概览（机械层）

orz 运行时安全由**机械层**承载（模型提议、机械层执行/校验）：

- **读面沙箱**：文件读取收窄于工作区＋注册技能根＋会话卷只读窗口（`.gsa/session/terminal/*.log`、`run_tests_output.txt`）；
- **写入管控**（0bw，2026-09-26）：写目标机械锁死（系统核心路径＋载体自保护集）＋`run_terminal_cmd` 命令面机械审查（block / warn 留痕）；
- **ACAF 票据门**：命令、写入与联网访问经权限桥与 ACAF 一次性票据（`orz-signer` 独立进程签发；未配置即 fail-closed）；
- **journal**：会话事件逐条留痕，支持 `--replay` 回放复查。

## 写入管控（0bw）——能力与边界

**定位：高阻力＋强审计，非绝对保证。** 三档措辞：

- **保证（机制可证）**：工具面（`search_replace`）对锁死面写目标**硬拒**——系统核心（Windows：systemroot / `Program Files` / `Program Files (x86)` / `ProgramData`；Linux：`/boot` `/etc` `/usr` `/lib*` `/bin` `/sbin` `/dev` `/proc` `/sys`）与载体自保护集（`.gsa` 会话卷、orz 安装目录与三件套 `orz`/`orz-signer`/`orz-acaf-provision`、`grok-home`）。命令面（`run_terminal_cmd`）对「安全机制翻转／系统核心写入／载体写入」类命令**拒绝执行**（机械文案随 tool 结果入 journal）。
- **阻力（高成本）**：命令面审查为 **best-effort 风险闸**——覆盖常见破坏形态（安全机制翻转、system-core 写入、载体写入、根级/通配删除、提权）；对变量拼接、脚本内命令、别名、`.NET` 直调、未存在面的路径混淆等**不保证**覆盖。可写面默认为真机环境（**不做可写根 allowlist**）；锁死面之外不新增限制。
- **审计（事后可查）**：block/warn 事实随 tool 结果进入 journal（warn 在结果头部附 `[写入管控·提示]` 行）；会话可经 `--replay` 复查。

**设计就绪、待落地**：Windows 受控文件夹访问（CFA）enforce 面（audit 探针先行）、Linux Landlock 写授权收窄、运行时载体完整性自检。设计权威（相对本目录）：[`../docs/WRITE_CONTROL_MECHANICAL_DESIGN_2026-09-26.md`](../docs/WRITE_CONTROL_MECHANICAL_DESIGN_2026-09-26.md)。

注意：调试开关（如 `ORZ_ACAF_FAIL_CLOSED=0`）会降低防护等级，不推荐用于正式工作。

## 报告建议包含

- orz 版本与平台（`orz --version`；OS 版本）；
- 复现步骤与命令原文；
- 预期与实际行为；
- 涉及面（读面沙箱／写入管控规则 id／ACAF 票据链／journal 完整性）。
