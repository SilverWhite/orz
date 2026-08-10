# ADR-0006：DeepSeek API 凭据目标名注册表 + 注入方式裁决

- 状态：accepted
- 日期：2026-08-06
- 关联：GAK-CRED-001（`DEEPSEEK_CREDENTIAL_HARDENING_2026-07-23.md`）、`assurance/deepseek_adapter.py`、`assurance/retrieval_subagent.py`、`orz-loop/src/gateway/credentials.rs`

## 1. 背景

DeepSeek API key 此前注册于 Windows Credential Manager，目标名为初步测试时随手起的
三个名字（`FEP-Agent/DeepSeek`、`FEP-Agent/DeepSeek-Retrieval`、`deepseek-retrieval-subagent`），
命名风格不统一，且仅在 Python 侧（GAK-CRED-001）消费；Rust 侧 live 测试使用
`ORZ_TEST_API_KEY` 环境变量注入。本 ADR 固化凭据目标名（唯一事实源）并裁决注入方式。

## 2. 决策

### 2.1 凭据目标名注册表（不可变）

| 角色 | 目标名 | 旧名（2026-08-06 废弃） |
|---|---|---|
| 主 agent | `orz-deepseek/agent` | `FEP-Agent/DeepSeek` |
| 外部检索子代理 | `orz-deepseek/1` | `FEP-Agent/DeepSeek-Retrieval` |
| 内部检索子代理 | `orz-deepseek/2` | `deepseek-retrieval-subagent` |

- 统一前缀 `orz-deepseek/`；`agent` = 主 agent，数字序号 = 检索子代理（1 = 外部、2 = 内部）。
- **web_search（2026-08-11 裁决）**：不新增独立注册目标——web_search 执行器复用主
  agent 凭据（`orz-deepseek/agent`）。曾提议 `orz-grok/search`（xAI Grok 搜索后端独立
  key），被用户裁决否决：检索必须来自当前接入的 provider（DeepSeek 服务端 web
  search，同一把 key），不依赖外部检索 API——框架检索能力不是外挂。
- 目标名是**不可变注册项**：任何改名必须先新增本表条目并迁移凭据，再删除旧条目
  （旧条目留存期间属孤儿条目，凭据审计应报告）。
- 代码引用纪律：目标名只允许出现在常量定义处（Python：`deepseek_adapter.py`
  `DEFAULT_CREDENTIAL_TARGET`、`retrieval_subagent.py` 两个常量；Rust：
  `credentials.rs` `AGENT_CREDENTIAL_TARGET`），其余代码一律引用常量——禁止字符串
  字面量（2026-08-06 已清除 `cli.py` 8 处硬编码 + schema const + PS1 脚本引用）。

### 2.2 注入方式裁决：统一 Windows Credential Manager

- **所有 API key 注入统一走 Windows Credential Manager（Generic 凭据，CredReadW）**；
  不再引入环境变量/配置文件等并行注入机制。
- Python 侧：GAK-CRED-001 既有机制（`_read_windows_credential` + `CredentialGuard`
  RAII 零化 + 环境脱敏 + 审计）维持，仅目标名随注册表。
- Rust 侧：`orz-loop/src/gateway/credentials.rs` `read_agent_api_key()`（CredReadW +
  blob 零化后 CredFree；非 Windows fail-closed）；`--real` 生产分支已开通
  （`transport.rs` `real_gateway_from_credentials`，orz-bin/orz-codex `build_gateway`
  fail-closed exit(2)）。
- `ORZ_TEST_API_KEY` 环境变量机制**废除**（2026-08-06 live 测试已改读凭据管理器；
  环境变量仅保留 `ORZ_TEST_LIVE` 测试开关——开关非凭据，不属注入）。
- **web_search（2026-08-11 方向修正）**：执行器复用主 DeepSeek key——`orz-host`
  `credentials.rs` 的 `DeepSeekCredentialReader` 委托 `orz-loop` `read_agent_api_key()`
  （Windows Credential Manager / 非 Windows `ORZ_DEEPSEEK_API_KEY` env，即 §2.3
  Linux 容器通道；eval 容器零新增 env）。无新注册目标、无新
  env 变量、无第二供应商。

### 2.3 扩展：Linux 容器通道（2026-08-07 登记）

- 2026-08-07 TB2/eval 实机运行登记：Linux 容器内无 Windows Credential Manager，
  `read_agent_api_key()` 非 Windows 分支读 `ORZ_DEEPSEEK_API_KEY` env。eval `.env`
  由 `write_env_key.py` 从 Windows 凭据写入（静默、不打印），经 harbor `--env-file`
  注入容器，不落盘不打印。
- 该 env 通道是 §2.2「统一 Windows Credential Manager」裁决的**显式例外**：仅限无
  Credential Manager 的平台（Linux 容器/CI）；有 Credential Manager 的平台不得走 env。
- 2026-08-11 起 web_search 执行器复用同一通道（§2.2 web_search 方向修正），eval
  容器零新增 env 变量。
- 来源：`TERMINAL_BENCH_2_EVAL_2026-08-08.md`（"ADR-0006 扩展（Linux 容器通道，
  注释已记录）"）；实现注释见 `orz-loop/src/gateway/credentials.rs` 非 Windows 分支。

## 3. 后果

- Python：三个常量值更新 + `cli.py` 引用常量 + schema const（deepseek-api-observation-result、
  deepseek-external-readiness、deepseek-real-development-plan）同步 + 测试/PS1 引用同步。
- Rust：`credentials.rs` 新模块 + live 测试改注入 + `windows` crate 走 workspace 既有
  feature（`Win32_Security_Credentials` 已启用，零 feature 新增）。
- 凭据注册：2026-08-06 用户**重新注册**三个新 key（内容非旧条目迁移——旧条目已删，
  内容与旧 key 不同，digest 核对不一致属预期）；三新条目可读验证通过（各 35 字符）。
- 后续新增凭据角色：先更新本表 → 迁移/注册凭据 → 加常量 → 引用。
