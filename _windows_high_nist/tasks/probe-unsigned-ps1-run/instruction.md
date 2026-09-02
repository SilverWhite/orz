# 摩擦探针：运行未签名 .ps1（probe-unsigned-ps1-run）

在 PowerShell 中运行一个未签名脚本：脚本在工作区写入标记文件
`ps1-marker.txt`（内容 `probe-unsigned-ps1-run-ok`）。调用参数固定为
`-NoProfile -NonInteractive -ExecutionPolicy Bypass -File`，刻意不过度
约束——本探针核对"墙下 PowerShell 命令树能否端到端执行 + 产物是否真实落盘"
（PowerShell 错误文本与退出码分裂是设计 §5 #1 摩擦源，verifier 以产物内容
为准而非只看退出码）。

把尝试结果（成功 / blocked + OS 错误文本）写入结果文件，供 verifier 断言。
control / non-admin 预期 success；high-nist 预期 blocked——实机发现
powershell.exe 作为 AppContainer 沙箱的孙进程启动时 DLL 初始化失败
（0xC0000142 STATUS_DLL_INIT_FAILED；墙探针以直子进程跑 PS 19/19 正常），
登记为缺口候选（对 ⑦ 模型侧 agent 在墙内跑 PS 有直接影响）。
