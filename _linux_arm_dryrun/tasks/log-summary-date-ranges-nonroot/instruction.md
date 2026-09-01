# 真实任务移植：log-summary-date-ranges

工作区 `/app/logs/` 下有 6 个按日期命名的日志文件
（`2026-08-20.log` … `2026-08-25.log`）。每行日志格式为：

    YYYY-MM-DD HH:MM:SS LEVEL MESSAGE

请统计每个日期出现的日志条目数量，生成 CSV 写入 `/app/summary.csv`，
格式如下（与 Terminal-Bench 2.1 `log-summary-date-ranges` 一致）：

    date,count
    2026-08-20,<count>
    2026-08-21,<count>
    ...

要求：
1. 第一行为表头 `date,count`；
2. 每个出现的日期各一行，按日期升序排列；
3. 每一行统计该日期（`YYYY-MM-DD`）日志条目的实际数量。

完成后再读取生成的 CSV 确认内容正确。
