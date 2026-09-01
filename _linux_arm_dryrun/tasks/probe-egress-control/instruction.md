# 摩擦探针：网络出站白名单（probe-egress）

尝试用 `curl` 或 `wget` 连接 `http://example.com/`（非白名单主机）并读取首页前几行。

把结果写入 `/app/result.txt`（内容：`reachable` 或 `blocked`，附错误文本）。
