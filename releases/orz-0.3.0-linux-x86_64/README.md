# orz 0.3.0 发布包

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
   tar -xzf orz-0.3.0-linux-x86_64.tar.gz
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

## 0.3.0 更新（相对 0.2.0）

- **黑板会话作用域（P2-13 B1）**：一个对话一个黑板（取代 plan-epoch
  生产语义，`--plan` 诊断保留）；记录按 (round, domain) 结构化写时盖章；
  live 黑板随会话侧车持久化、续载/恢复；temporal / failure_agg 轴收口
  为会话相对；失败 run 不写回。
- **渲染折叠（P2-13 B2）**：`blackboard_read` 折叠态渲染（默认展开 =
  当前域段 ∪ 最近 K 轮 ∪ 最近 20% 行，K=10），更早内容按域段折叠为标注
  行（段内计数/首末轮/摘要预览），可用 `domain` + `round_from` /
  `round_to` 显式展开；edits / tool_actions 渲染 cap 补齐；空槽统一
  「（无）」。
- **会话疲劳提醒（P2-13 B3）**：疲劳 = 黑板 live 字节水位（默认
  W=10 MiB，env `ORZ_BLACKBOARD_LIVE_BUDGET_BYTES` 覆盖）；50/70/90
  三档用户侧事实提醒（单档越线一次、巨幅跳跃只报最高未提醒档），
  70 档建议换对话；无压缩轮数门槛。
- **会话存档单包（P2-13 B3）**：close 时对话 + 黑板合成单一 gzip 包
  （`session_archive` 事件 + ARC 前缀 run journal）；close 时有
  in-flight run 则推迟到 run 收尾补触发；存档 IO 走 blocking 池。
- **失败目标聚合（P2-12）**：`failure_agg` 黑板分区 + F4 失败目标身份
  写时盖章 + 压缩注意事项槽聚合渲染（错误码集合/首末时间，零 LLM）。
- **依赖图主线与 PULL 自描述（P2-11）**：read→write 锚点边 +
  工具→实体变更边 + `blackboard_read section=deps` 查询面；分区版本
  计数增量头 + temporal 一次返回；强制模板轮/DC 硬信号清理退役。
- **实机复验修复**：F11 receipt 同一 call_id 至多一条完成事件；
  `[SOURCE]` 声明行全角括号切分与 source_title 非空回退。

## 说明与边界

- 生产路径不强制首轮计划门（plan 门休眠、近零系统提示 + 固定工具面）；
  任务交付走 submit 两阶段（请求 → 确认）。
- 检索为显式三态（`local_browser` / `framework_fallback` / `off`，
  默认 `off`）；联网检索推荐 `local_browser`，浏览器启动失败由机械层
  降级为原生 web 检索并记录 transition。
- 安全层（ACAF）默认强制开启；确需临时关闭请显式设置
  `ORZ_ACAF_FAIL_CLOSED=0`（不推荐用于正式工作）。
- 常用调优：`ORZ_STALL_TIMEOUT=<秒>`（卡住看门狗，默认 360）、
  `ORZ_TOOL_TIMEOUT_SECS=<秒>`（单工具超时，默认 300）。
- 版本信息：源码 orz `d4a37fdb`（父仓库 `d442df3`），构建 2026-09-03。
- 已知边界：未提供 macOS 原生包；黑板会话疲劳档位/折叠阈值按真实长
  会话遥测复核路径继续校准；跑分数据尚未完整（正式评测待后续）。

## 完整性校验

- Linux：`sha256sum -c SHA256SUMS`
- Windows（PowerShell）：
  `Get-FileHash orz.exe,orz-signer.exe,orz-acaf-provision.exe,README.md -Algorithm SHA256`，
  与 `SHA256SUMS` 逐行对照
