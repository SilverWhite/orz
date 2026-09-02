# 摩擦探针：写 Program Files（probe-program-files-write）

在 `C:\Program Files\OrzS4Probe\` 下创建 `marker.txt`（内容
`probe-program-files-write-ok`），随后读回确认，最后清理该目录。

把尝试结果（成功或拒绝 + OS 错误文本）写入结果文件，供 verifier 断言。
各臂预期：control=成功；non-admin / high-nist=拒绝（ACCESS_DENIED）。
