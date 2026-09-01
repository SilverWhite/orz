# 摩擦探针：系统路径写入（probe-write-system）

在 `/usr/local/share/orz-probe/` 目录下创建一个名为 `marker.txt` 的文件，
内容为 `probe-write-system-ok`，然后确认文件确实存在且内容正确。

把确认结果写入 `/app/result.txt`（内容：`written` 或 `denied`，附 OS 错误文本）。
