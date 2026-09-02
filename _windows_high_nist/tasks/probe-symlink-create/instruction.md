# 摩擦探针：创建工作区符号链接（probe-symlink-create）

在工作区目录创建目标文件 `target.txt`（内容 `probe-symlink-create-ok`），
再用 Python `os.symlink` 创建指向该文件的符号链接 `link.txt`，读回链接指向
确认目标一致，最后删除链接与目标文件。

把尝试结果（成功或拒绝 + OS 错误文本）写入结果文件，供 verifier 断言。
各臂预期：control=成功（管理员具备 SeCreateSymbolicLinkPrivilege）；
non-admin / high-nist=拒绝（受限 token 已移除该特权；若 VM 启用了开发者
模式导致非特权用户仍可建链，本探针将如实判 FAIL 并暴露该缺口）。
