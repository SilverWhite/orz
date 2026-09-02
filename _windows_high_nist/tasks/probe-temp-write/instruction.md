# 摩擦探针：写 %TEMP%（probe-temp-write）

通过 Python 标准库在当前进程的临时目录（`%TEMP%`）中创建文件
`OrzS4ProbeTmp_<pid>.txt`，写入标记 `probe-temp-write-ok`，读回确认一致后
删除该文件。

把尝试结果写入结果文件，供 verifier 断言。三臂预期均为 success：high-nist
臂的临时目录被重定向到工作区 `.tmp`（策略应保证重定向后的临时面可用，而非
误伤工具链），control / non-admin 臂使用系统临时目录。
