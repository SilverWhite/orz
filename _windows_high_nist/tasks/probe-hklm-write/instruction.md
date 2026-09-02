# 摩擦探针：写 HKLM 注册表（probe-hklm-write）

在 `HKLM\SOFTWARE\OrzS4Probe` 下创建注册表项，写入字符串值
`Marker = probe-hklm-write-ok`，随后读回确认内容一致，最后清理该注册表项。

把尝试结果（成功或拒绝 + OS 错误文本）写入结果文件，供 verifier 断言。
各臂预期：control=成功；non-admin / high-nist=拒绝（ACCESS_DENIED）。
