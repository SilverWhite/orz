# 结构化操作协议 v0.1（Structured Operation Protocol）

- 状态：Draft（参考实现已就位，生产接线待裁决）
- 日期：2026-08-13
- 配套 Schema：[`structured-operation-protocol-v0.1.schema.json`](structured-operation-protocol-v0.1.schema.json) / [`structured-operation-audit-v0.1.schema.json`](structured-operation-audit-v0.1.schema.json)
- 参考实现：[`../assurance/ops_executor.py`](../assurance/ops_executor.py)（Python 跨平台） / [`../scripts/ops_executor.ps1`](../scripts/ops_executor.ps1)（Windows PowerShell）
- 桥接实现：[`../scripts/ops_bridge.ps1`](../scripts/ops_bridge.ps1)（Windows→WSL） / [`../scripts/ops_bridge.sh`](../scripts/ops_bridge.sh)（POSIX→Docker/SSH）
- 设计关系：与 [`ADR-0011`](../adr/ADR-0011-authenticated-control-and-action-fabric.md) 的 ACAF 动作票据互补——票据面负责“谁被授权执行”，本协议负责“动作载荷与执行层安全策略”；不改变 ADR-0010 任何条款。

## 1. 背景与问题

命令字符串跨解释器透传时，引号、转义、通配符、换行符和编码会在每一层被重新解析，删除类命令一旦语义漂移就会打错目标。按项目根目录推断执行环境（“Windows 项目就用 PowerShell”）只是另一种启发式，会引入新的误判。脚本作为默认通道则把转义负担压回模型。

本协议的目标：**模型只表达意图，判断全部下沉到机械层**。环境由实际执行宿主自报，动作载荷是结构化数据，删除默认回收站、缓存分类放行、非缓存超限拒绝，全程 JSONL 审计。

## 2. 核心原则

1. 环境由实际执行宿主机械自报，不做路径格式推断。
2. 模型只提交结构化操作；必填字段最少（`op` + `targets`）。
3. 路径解析、分类、容量查询、策略裁决、审计全部在固定执行器内完成。
4. 删除默认进回收站；非缓存且超回收站容量 → 拒绝执行并留记录（fail-closed）。
5. 缓存识别基于“名称 + 结构证据 + 项目声明”，未知一律走保守策略。
6. 跨环境只传 JSON，经固定桥接脚本；禁止命令文本透传。
7. 不设逐次人工确认；策略拦截以“拒绝 + 记录”呈现，任务级预授权是唯一例外。
8. 脚本是逃生舱，不是默认通道；必须经固定启动器、落盘、哈希、审计。

## 3. 协议封装

操作信封为单个 JSON 对象，严格校验，未知字段一律拒绝：

```json
{
  "proto": 1,
  "id": "REQ-20260813-001",
  "op": "file.delete",
  "targets": ["target"],
  "reason": "清理编译产物"
}
```

| 字段 | 必填 | 说明 |
|---|---|---|
| `proto` | 是 | 恒为 `1` |
| `op` | 是 | 操作枚举，见下 |
| `targets` | 是 | 目标路径数组；`env.info`/`process.run` 可为空数组 |
| `id` | 否 | 请求 id，`^[A-Za-z0-9._-]{1,80}$`；缺省由执行器生成 |
| `reason` | 否 | 一句话意图说明（≤500 字符），只用于审计，不作为分类依据 |

### 操作目录 v0.1

| 操作 | 附加必填 | 附加可选 | 说明 |
|---|---|---|---|
| `file.read` | — | `max_bytes` | 读取文件文本；默认上限 10 MiB |
| `file.write` | `content` | — | 原子写入（临时文件 + 替换）；目标恰一个 |
| `file.move` | `dest` | — | 移动；目标恰一个；`dest` 必须在作用域内 |
| `file.copy` | `dest` | — | 复制；目标恰一个；`dest` 必须在作用域内 |
| `file.delete` | — | `dry_run` | 删除；模式由执行器裁决，模型不填 `mode`/`recursive` |
| `dir.list` | — | `depth`, `max_entries` | 目录列举；默认深度 1、上限 10000 条 |
| `process.run` | `exe` | `args`, `cwd`, `timeout_ms` | 参数数组执行，**不经 shell**；属变更操作，需允许根目录且 `exe` 在允许根内或命中 `OPS_PROCESS_ALLOW`，缺省拒绝 |
| `env.info` | — | — | 返回执行宿主环境指纹 |

除 `env.info`/`process.run` 外，`targets` 至少 1 项（Schema 已强制）。模型不得填写 `mode`、`recursive`、`disposable`、容量等机械判定字段。

## 4. 环境检测

执行器启动时自报环境指纹，不根据项目根目录推断：

```json
{
  "host": "windows",
  "platform": "Windows-10-...",
  "shell": "powershell",
  "shell_version": "5.1",
  "path_style": "windows",
  "eol": "crlf",
  "encoding": "utf-8",
  "cwd": "D:\\CLI",
  "allow_roots": ["D:\\CLI"]
}
```

同一操作在不同宿主上语义一致。跨环境目标是显式端点（`wsl:Ubuntu`、`docker:name`、`ssh:host`），端点配置在桥接脚本侧，模型只传端点名。

## 5. 路径与作用域

- 允许根目录来自 `OPS_ALLOW_ROOTS`（`os.pathsep` 分隔）或命令行；**未配置时拒绝所有变更操作**（`env.info` 除外）。
- 所有路径先 `realpath`/`GetFullPath` 规范化，再判断是否位于允许根目录内；符号链接逃逸因此被消除。
- 拒绝把允许根目录本身、盘根、文件系统根作为删除目标。
- 变更操作（写、移动、复制）的目标与目的路径都必须在作用域内。
- 删除操作强制 `-LiteralPath` 语义；模型提交的是路径字符串，不是 shell 表达式。
- `process.run` 属变更操作：未配置允许根目录时拒绝；`exe` 解析后必须位于允许根目录内，或命中 `OPS_PROCESS_ALLOW`（`os.pathsep` 分隔的绝对路径或可执行名），否则拒绝（exit 2）。
- 执行器自身文件、`.ops.json`、审计日志路径以及 `OPS_PROTECTED`（`os.pathsep` 分隔）声明的路径为受保护文件，变更操作一律拒绝；模型不得通过 `file.write` 创建或修改 `.ops.json`。

## 6. 删除策略：分类 + 超限

执行流程：解析 → 分类 → 计量 → 容量 → 裁决 → 执行/拒绝 → 审计。

### 6.1 分类结果

| 类别 | 含义 |
|---|---|
| `cache` | 可再生成内容；允许直接删除，必须记录分类证据 |
| `unknown` | 无法机械确认为缓存；走保守策略 |
| `protected` | 项目声明不可删；直接拒绝 |

### 6.2 项目声明 `.ops.json`

在目标所在项目内（沿父目录上溯到允许根目录）寻找 `.ops.json`：

```json
{
  "disposable": ["**/build", "**/dist"],
  "protected": ["**/data", "**/results.json"]
}
```

`protected` 优先于 `disposable`。模式按绝对路径（正斜杠）匹配；模型不得声明该文件。

`.ops.json` 存在但无法解析或结构非法时按 `protected` 处理（fail-closed），证据记录 `malformed .ops.json`；不得静默忽略。

### 6.3 内置硬规则（名称 + 结构证据）

| 目录 | 结构证据 |
|---|---|
| `__pycache__` | 内部文件全部为 `.pyc`/`.pyo` |
| `target` | 存在 `debug/`、`release/`、`.fingerprint/` 或 `CACHEDIR.TAG` |
| `node_modules` | 存在 `.package-lock.json` 或 `.bin`，或父目录有 package manifest |
| `.cargo/registry` | 路径结构即证据 |
| `.gradle/caches` | 路径结构即证据 |
| `.cache` / 任意含 `CACHEDIR.TAG` 的目录 | 存在 `CACHEDIR.TAG` |

证据字符串写入审计（例如 `builtin: target/debug,.fingerprint`）。

分类沿目标向上逐级匹配：目标或其任一祖先（上溯到允许根目录）命中内置硬规则或项目声明时，按命中类别归类；`protected` 优先于 `cache`。

### 6.4 策略矩阵

| 分类 | 总量 ≤ 容量 | 总量 > 容量 |
|---|---|---|
| 全部为 `cache` | 直接删除（记录证据） | 直接删除（记录证据） |
| 任一 `protected` | 拒绝 | 拒绝 |
| 含 `unknown` | 进回收站 | 拒绝（fail-closed） |

### 6.5 容量

- Windows：查询目标卷回收站 `MaxCapacity`（注册表 `BitBucket\Volume\{GUID}`）；缺失时按卷容量 10% 估算。
- POSIX：XDG 回收站无硬上限，使用策略阈值 `OPS_TRASH_MAX_BYTES`（默认 5 GiB）。

注意：**进回收站不等于可恢复**。回收站满时会挤出旧条目；审计必须区分 `trash` / `permanent` / `rejected` 三种结果。

## 7. 跨环境桥接

- 跨边界只传 JSON；桥接脚本把操作写为临时文件后交给目标端固定执行器，避免 stdin 编码问题。
- 端点白名单在 [`ops-bridges.json`](../scripts/ops-bridges.json)；未知端点拒绝。
- `ops-bridges.json` 是本地端点配置，其中路径值需按实际环境调整，提交到仓库仅作默认示例。
- `wsl`：Windows 临时路径转换为 `/mnt/<drive>/...`，目标端执行 `executor.py --op-file <临时文件>`。
- `docker`/`ssh`：v0.1 通过 UTF-8 stdin 传递（`--op-file -`），由桥接脚本显式保证字节流。
- 桥接脚本向目标端注入 `OPS_SOURCE=bridge:<endpoint>`；`wsl` 端点同时把 `OPS_ALLOW_ROOTS`/`OPS_AUDIT_LOG` 转换为 `/mnt` 路径注入。未知或不支持的端点一律 exit 1。

## 8. 编码契约

- 所有跨进程、跨环境、跨文件文本统一 UTF-8（无 BOM）；不依赖系统代码页。
- 解码兼容由机械门控承担，模型对编码零感知、零负担：
  1. **调用侧（生产者强制）**：固定代码按检测到的宿主注入 UTF-8 强制——PowerShell 5.1 启动时设置
     `[Console]::OutputEncoding`/`$OutputEncoding` 为 UTF-8；Python 子进程注入
     `PYTHONIOENCODING=utf-8`/`PYTHONUTF8=1`；pwsh 默认 UTF-8，仍显式设置兜底。模型不参与任何编码参数。
  2. **捕获侧（消费者兜底）**：解码链固定为“剥离 BOM → UTF-8 严格 → GB18030 → lossy”，
     每次记录实际命中的编码；模型只看到规范化后的文本。
  3. **文件侧**：读取走同一检测链并记录编码 provenance；写入统一 UTF-8 无 BOM。
- Windows PowerShell 5.1 是主要编码风险源（GB2312 控制台、`Add-Content` BOM、无
  `ProcessStartInfo.ArgumentList`）：优先 `pwsh`，回退 `powershell.exe` 时必须显式设置输出编码；
  禁止把“升级 shell”当作编码修复的唯一手段。
- 审计事件记录 `output_encoding`（可选字段；Schema 已扩展，producer 已接线）：`file.read` 记录文件命中编码，`process.run` 记录输出解码编码。
- 文件侧读取结果同时返回 `encoding`（命中的解码链编码），审计侧记录 `output_encoding`。
- orz 侧实施已闭合：`GAP-ENCODING-GATE`（[`CLI_PROJECT_INDEX.md` §3.1](../CLI_PROJECT_INDEX.md)；[实施审计](../docs/audits/GAP_ENCODING_GATE_IMPL_AUDIT_2026-08-13.md)），本协议语义不变。

## 9. 脚本逃生舱

操作集无法表达时才允许脚本，且必须满足：

1. 经固定启动器执行，模型不能把命令文本直接塞给解释器；
2. 脚本先落盘并计算哈希，随审计记录全文；
3. 执行时注入 `OPS_ALLOW_ROOTS`、`OPS_AUDIT_LOG`、超时和工作目录；
4. 破坏性原语必须调用执行器安全 API（回收站、作用域校验），不裸 `rm`/`Remove-Item`；
5. 逃生舱是显式通道：动作记录必须完整可回放。

## 10. 审计

每次执行追加一行 JSONL（append-only），字段见 [`structured-operation-audit-v0.1.schema.json`](structured-operation-audit-v0.1.schema.json)：

- `request_id`、`timestamp`、`op`、`targets`（原始输入）
- `resolved[]`：解析后绝对路径、分类、证据、大小、文件/目录数
- `policy`：裁决（`permit`/`reject`）、模式（`trash`/`permanent`）、理由
- `result`：状态（`ok`/`dry-run`/`rejected`/`invalid`/`error`）、详情、耗时
- `env`：环境指纹；`source`：请求来源（如 `bridge:wsl:Ubuntu`）；`output_encoding`：可选，本次执行命中/使用的文本编码

无效输入（含无法解析的 JSON）也必须产生一条审计记录；审计写入失败按 `error` 处理（exit 1），不得静默吞掉。

审计文件由 `OPS_AUDIT_LOG` 指定，缺省为当前工作目录下 `ops-audit.jsonl`。会话内必须保留人类可读的动作摘要：动作、解析后路径、裁决、结果。

## 11. 错误与失败模式

| 情况 | 行为 |
|---|---|
| 未知 `op` / 未知字段 / 类型错误 | 拒绝（exit 1） |
| 未配置允许根目录 | 拒绝变更操作（exit 2） |
| 目标不在作用域内 | 拒绝（exit 2） |
| 非缓存超限删除 | 拒绝（exit 2），不打断自动化，只留记录 |
| `process.run` 未配置允许根目录或 `exe` 未授权 | 拒绝（exit 2） |
| 分类证据缺失 | 归 `unknown`，走保守策略 |
| 桥接端点未知 | 拒绝（exit 1） |
| 审计写入失败 | 结果按 `error`（exit 1），不得伪装成功 |

## 12. 与现有设计的关系

- **ADR-0011 / ACAF**：动作票据负责授权面（`command_exec_v1`、`file_write_v1`、`network_v1`、`resolved_target_sha256`、fail-closed）。本协议定义动作载荷与执行层安全策略；生产接线顺序为“先验票，再由本协议执行器执行”。
- **ADR-0010 §3.8**：permission hard gate 的执行侧一致；本协议不改变任何设计条款。
- **Python assurance 路径**：参考实现遵循“reference/conformance、Schema authority”的既有定位；Rust 生产接线待登记差距。

## 13. 演进

- `proto` 只做加法；新操作必须同步 Schema、双执行器和测试。
- v0.2 候选：`cache.clean`（按 `kind` 定位缓存目录）、git-tracked 保护、任务级授权 token、回收站恢复入口。

## 14. 验证清单

- Schema 校验通过（`jsonschema` 或等效实现）。
- `env.info` 返回环境指纹。
- `file.delete` + `dry_run: true` 对缓存目录返回 `permanent` 计划且不产生变更。
- 未知字段 / 越界路径 / 非缓存超限 / 未授权 `process.run` 分别被拒绝。
- 小文件非缓存删除进入回收站。
- 每次执行（含无效 JSON）产生一条完整且符合审计 Schema 的 JSONL。
