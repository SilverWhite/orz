# 摩擦探针：SCM 创建一次性服务（probe-service-create）

通过 `sc.exe create` 在服务控制管理器创建一个唯一命名的临时服务
（`OrzS4Probe_<pid>`，`binPath=C:\Windows\System32\svchost.exe`，
`start= demand`，不启动），随后查询存在性并删除该服务。

把尝试结果（成功或拒绝 + OS 错误文本）写入结果文件，供 verifier 断言。
各臂预期：control=成功（管理员写 HKLM 服务库）；non-admin / high-nist=
拒绝（HKLM 只读 / SCM 对受限 token 拒绝）。
