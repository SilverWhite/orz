# orz 0.1.0 初版试用包

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
   tar -xzf orz-0.1.0-linux-x86_64.tar.gz
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

## 说明与边界

- 首轮要求先写计划（plan-first 门）；第 2 轮起可直接调用工具。
- 检索默认 `local_browser`，浏览器不可用时自动降级为原生 web 检索。
- 安全层（ACAF）默认强制开启；确需临时关闭请显式设置
  `ORZ_ACAF_FAIL_CLOSED=0`（不推荐用于正式工作）。
- 常用调优：`ORZ_STALL_TIMEOUT=<秒>`（卡住看门狗，默认 360）、
  `ORZ_TOOL_TIMEOUT_SECS=<秒>`（单工具超时，默认 300）。
- 版本信息：源码 orz `8bcf18c`（父仓库 `bbf5ec7`），构建 2026-08-25；
  本次为初版试用，仍处迭代中。
- 2026-08-25 初版打包前修复：DeepSeek 纯文本响应偶尔不带
  `reasoning_content`，回推时字段缺失会触发下一请求报错；已在传输层
  统一规范化（初版实测 3/6 → 修复后 6/6 通过）。
- 已知边界：未提供 macOS 原生包；内部子代理检索在小型工作区属低频
  能力；跑分数据尚未完整（正式评测待后续）。

## 完整性校验

- Linux：`sha256sum -c SHA256SUMS`
- Windows（PowerShell）：
  `Get-FileHash orz.exe,orz-signer.exe,orz-acaf-provision.exe,README.md -Algorithm SHA256`，
  与 `SHA256SUMS` 逐行对照
