# orz 0.2.0 发布包

融合架构的终端 AI 编程代理（DeepSeek 后端）。支持 **Windows x86_64**
与 **Linux x86_64**（musl 静态链接）两个平台包。

## 内容

- `orz`（Windows 下为 `orz.exe`）— 主程序（交互 TUI / `-p` 无头模式）
- `orz-signer` — ACAF 安全签发器（独立进程）
- `orz-acaf-provision` — ACAF 一次性初始化工具（需与 `orz-signer` 同目录）

## Windows 快速开始

1. 解压，把 `orz.exe`、`orz-signer.exe`、`orz-acaf-provision.exe` 放入
   同一目录（例如 `C:\orz`）。

2. 配置 DeepSeek API Key——统一走 Windows 凭据管理器（Generic 凭据，
   目标名 `orz-deepseek/agent`）。在 PowerShell 或 CMD 里执行一次：

   ```powershell
   cmdkey /generic:orz-deepseek/agent /user:agent /pass:你的DeepSeek_API_Key
   ```

3. 初始化安全签发（一次性；ACAF 默认 fail-closed，未配置会拒绝启动）：

   ```powershell
   .\orz-acaf-provision.exe "$env:USERPROFILE\.orz-acaf\keystore" "$env:USERPROFILE\.orz-acaf\signer-manifest.json"
   ```

   按输出设置启动环境（PowerShell）：

   ```powershell
   $env:ORZ_ACAF_KEYSTORE = "$env:USERPROFILE\.orz-acaf\keystore"
   $env:ORZ_ACAF_MANIFEST = "$env:USERPROFILE\.orz-acaf\signer-manifest.json"
   $env:ORZ_ACAF_BINARY = "C:\orz\orz-signer.exe"
   ```

4. 运行：

   ```powershell
   .\orz.exe                    # 交互 TUI
   .\orz.exe -p "你的任务" --real   # 无头模式
   ```

## Linux 快速开始

1. 解压并赋予执行权限：

   ```sh
   mkdir -p ~/orz && cd ~/orz
   tar -xzf orz-0.2.0-linux-x86_64.tar.gz
   chmod +x orz orz-signer orz-acaf-provision
   ```

2. 配置 DeepSeek API Key（Linux 无凭据管理器，用环境变量；这是
   Windows 凭据管理器通道的显式例外）：

   ```sh
   export ORZ_DEEPSEEK_API_KEY=你的DeepSeek_API_Key
   ```

3. 初始化安全签发（一次性）：

   ```sh
   ./orz-acaf-provision "$HOME/.orz-acaf/keystore" "$HOME/.orz-acaf/signer-manifest.json"
   export ORZ_ACAF_KEYSTORE="$HOME/.orz-acaf/keystore"
   export ORZ_ACAF_MANIFEST="$HOME/.orz-acaf/signer-manifest.json"
   export ORZ_ACAF_BINARY="$HOME/orz/orz-signer"
   ```

4. 运行：

   ```sh
   ./orz                       # 交互 TUI
   ./orz -p "你的任务" --real   # 无头模式
   ```

## 0.2.0 更新（相对 0.1.0）

- **机械层数学计算体（P2-10）I1–I6**：T̂/LIF 时间性计算器（102 runs
  离线复验、四对照门）+ 失败目标身份入事件面 + temporal 分区运行时
  （语义特征域 + 域切换 spike 侧车存档，模型无感）+ 类型化结果信封 +
  一层 pipe 归约；阶段 3 V1–V3 验证闭环（FakeProvider 测试面、F11
  receipt↔事件链同构、S3 Linux musl 重建 + bookworm 冒烟 + S4 实机冒烟）。
- **R2 deny 通道接线**：`ToolOutcome::Deny` + 拒绝路径喂入（deny 轮计数
  与事件面留痕）。
- **R5 grep→read 管线**：match 选择 + span→offset 映射 + 不兼容 pipe
  类型化拒绝。
- **检索编排机械层（0k）**：同轮读类并行、子代理 run 级预算/超时、
  `[DOC]` 回传截断、浏览器容器参数/资源拦截/等待语义；主面恢复
  web_search 单一派发入口、browser_read 主面封存；方向 C 结构化结果
  收口（`[RESULT_JSON]` 组织块契约删除、机械 ledger 单轨）。
- **0k 第二批**：project_doc_index v2（git 基线 + 增量 + Blake3 + 驻留 +
  写后失效）、会话级 tab 池 + 同轮多页并行 + DNS 缓存、委托契约复杂度
  分档（standard/extended/deep）。
- **THIN-HARNESS-REDESIGN-V2**：prompt 置空、orientation 软门、submit
  门修复、复读后置化、半助理层、终端 300s 中间回报、分层默认超时、
  fold bridge reasoning retention。
- **输出退化守卫**：复读检测序列内容门（DNA/RNA/蛋白序列族双门槛，
  合法序列引用不再误杀）。
- **工程重构**：controller.rs 29,091 → 4,142 行分批拆解（行为不变，
  事件序列与 journal 哈希链不动）。

## 说明与边界

- 首轮要求先写计划（plan-first 门）；第 2 轮起可直接调用工具。
- 检索默认 `local_browser`，浏览器不可用时自动降级为原生 web 检索。
- 安全层（ACAF）默认强制开启；确需临时关闭请显式设置
  `ORZ_ACAF_FAIL_CLOSED=0`（不推荐用于正式工作）。
- 常用调优：`ORZ_STALL_TIMEOUT=<秒>`（卡住看门狗，默认 360）、
  `ORZ_TOOL_TIMEOUT_SECS=<秒>`（单工具超时，默认 300）。
- 版本信息：源码 orz `a539a21b`（父仓库 `925f929`），构建 2026-08-31。
- 已知边界：未提供 macOS 原生包；内部子代理检索在小型工作区属低频
  能力；跑分数据尚未完整（正式评测待后续）。

## 完整性校验

- Linux：`sha256sum -c SHA256SUMS`
- Windows（PowerShell）：
  `Get-FileHash orz.exe,orz-signer.exe,orz-acaf-provision.exe,README.md -Algorithm SHA256`，
  与 `SHA256SUMS` 逐行对照
