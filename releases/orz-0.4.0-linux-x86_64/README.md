# orz 0.4.0 发布包

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
   tar -xzf orz-0.4.0-linux-x86_64.tar.gz
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

## 0.4.0 更新（相对 0.3.0）

- **里程碑说明**：0.3.1 / 0.3.2 为过程验证内部基线（2026-09-07/08，
  未单独发版）；0.4.0 为 0.3.0（2026-09-03）后首个发布里程碑，含
  TER M1、Task C/D、0m/0p/0q/0t 与 P2-14 S1/S2 等 47 个 orz 提交。
- **检索子代理双车道（0t）**：启用检索的会话 external 子代理双族恒在
  （本地浏览器 `browser_read` + `browser_control` / 原生 `web_search` +
  `web_fetch`），工具带静态车道标注（本地浏览器优先）；三值检索模式
  （γ）退役——换道由模型自主，浏览器可用性事实化
  （`browser_launch_result`），导航失败按真实类别普通回传；未启用检索
  的会话不注册检索工具（授权门 fail-closed 保留）。
- **浏览器控制 Phase 1**：`browser_control` 导航级动作面
  （navigate / back / forward / refresh / wait_load / snapshot），会话级
  控制 tab、有界日志特征回传；PDF 下载外置偏好修复（日常真实浏览器
  路径可用）。
- **工具执行层改革 M1（TER）**：去自身硬超时（超阈自动后台化 + 空闲/CPU
  兜底 + 10h 绝对兜底）、轮预算撤默认 120 硬限（默认无上限）、黑板
  `processes` live 视图、W-F11 env PULL 快照、W-F12 快速确定性失败、
  W-F13 64KB 读档 + 截断输出检索对象、F6 预算档默认 off、
  idle-kill journal 事件生产者。
- **GSA 会话卷底层层 + 两段门（0m/0p）**：`.gsa` 会话卷类型化资源
  （symlink-aware canonical 单源）；read_file/grep/list_dir 三分判定与
  窗口契约（terminal / run_tests 只读窗口直读、内部区通知→放行两段门、
  gitignore 绕过收口）。
- **自历史面与 key 不落卷（0p）**：`blackboard_read` 自历史
  （`failures_only` 失败聚合回查 + `search=<literal>` 历史检索 ≤20 行）；
  exec 行 exit 真实码渲染；turn_count 真实会话轮计数；journal / 台账 /
  ACP 侧车 / 终端日志全链 key 脱敏；结构化拒绝信封（policy_denial +
  session_volume_opened）。
- **统一失败事件管线（0q）**：写入侧单一漏斗 `stamp_failure`（四写点
  退役逐点对拍）；`failure_agg_absent` XOR 标记；console 订单
  `action_target` 第五族；Rust 法官 `failure_agg_coverage`（31 族）。
- **run-event 执法权翻转（Task D）**：期刊校验 Rust 单法官
  （`journal-conformance` CLI）+ registry JSON 唯一权威 + Python 冻结
  reference；31 族 Rust↔Python 对拍 0 差。
- **压缩 marker v0.3（P2-14）**：压缩点冻结黑板折叠视图快照（近窗明细 +
  旧段聚合，20K 定档）；rhythm/fallback/session_end/恢复预检 e2e 全绿。
- **其它治理收口**：Task C canonical 沙箱（读工具词法/canonical 越界硬
  拦 + ACAF fail-closed 默认下沉 + 解析单源化）、workspace 僵尸 crate
  剔除（64 → 49）、render_fold 解耦、GSA symlink 拒读安全回归。

## 说明与边界

- 生产路径不强制首轮计划门（plan 门休眠、近零系统提示 + 固定工具面）；
  任务交付走 submit 两阶段（请求 → 确认）。
- 检索为**启用门 + 双车道**形态：未启用会话不注册检索工具（fail-closed）；
  启用会话双族恒在（静态标注推荐本地浏览器优先），换道/重试由模型自主；
  浏览器启动/探活落 `browser_launch_result` 事实事件。宿主机日常真实
  浏览器可用为验收口径（评测容器注入不做特化）。
- 安全层（ACAF）默认强制开启；确需临时关闭请显式设置
  `ORZ_ACAF_FAIL_CLOSED=0`（不推荐用于正式工作）。
- 常用调优：`ORZ_STALL_TIMEOUT=<秒>`（卡住看门狗，默认 360）、
  `ORZ_TOOL_TIMEOUT_SECS=<秒>`（单工具超时，默认 300）；长命令超阈自动
  后台化，10h 绝对兜底（TER M1）。
- 版本信息：源码 orz `a467d0f9`（父仓库 `6a97820`），构建 2026-09-09。
- 已知边界：未提供 macOS 原生包；0t S4 实机复验（检索双车道判据：
  双车道存在性 / 静态标注 / γ 退役兼容 / 失败回传形态 / 通用统计 /
  宿主机日常可用性）待续。

## 完整性校验

- Linux：`sha256sum -c SHA256SUMS`
- Windows（PowerShell）：
  `Get-FileHash orz.exe,orz-signer.exe,orz-acaf-provision.exe,README.md -Algorithm SHA256`，
  与 `SHA256SUMS` 逐行对照
