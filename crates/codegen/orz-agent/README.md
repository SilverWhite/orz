# `orz-agent`

Skills/plugins/agents-md discovery surfaces and workspace toolset assembly,
shared with `orz-workspace`.

0ce（2026-10-01）死代码清退后的保留面：

- `plugins` — 插件发现/信任/安装注册表（消费方 `orz-workspace::discovery`
  与 `folder_trust`）；
- `prompt::skills` / `prompt::agents_md` / `prompt::workspace_user` —
  skills 与 AGENTS.md 发现解析（消费方 `orz-workspace::discovery`）；
- `repo` — cwd→git-root 目录链原语（消费方 `orz-workspace::folder_trust`
  与 `project_config`）；
- `discovery::project_agent_dirs(_in)` — 项目级 agent 目录枚举
  （`.grok/agents` + `.claude/agents` 兼容）；
- `config::workspace_grok_build_toolset` — proxy 模式 workspace
  全量工具集装配（消费方 `orz-workspace::handle`）。

原 Agent 构建、system prompt 模板族（含加密模板）、agent 定义机制
（`Agent`/`AgentBuilder`/`AgentDefinition`/预设注册表）零服务消费，
已整体退役（0cd/0ce；TB 无头路径零提示词姿态不变）。
