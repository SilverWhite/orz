# 干跑记账

`accounting.py` 对照 BoundaryBench 官方名单（task_applicability.py 的
not_applicable 7 题 + adapted-verifier 5 题）与机械层 OS 错误通道口径，
对 harbor trial 目录做记账汇总。

## 输出字段

- `arm`：control / non-root / high-nist（从 trial 名推断）。
- `reward`：verifier reward.txt。
- `not_applicable` / `adapted_verifier`：官方名单分类。
- `os_errors` / `os_error_channels`：事件链中的 OS 错误文本 → 通道
  （err/deny/slow 口径，对应设计 §7 判据 1 的干跑先行版）。

## 使用

```powershell
python analysis/accounting.py D:\tb-eval\jobs-official\<job> --out summary.json
```
