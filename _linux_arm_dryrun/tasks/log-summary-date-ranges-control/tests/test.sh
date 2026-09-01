#!/bin/bash
# 移植 verifier：断言 /app/summary.csv 存在且覆盖全部 6 个日期（count>0）。
set -u
if [ ! -f /app/summary.csv ]; then
  echo 0 > /logs/verifier/reward.txt
  echo "FAIL: summary.csv missing"
  ls -la /app/ 2>&1 || true
  exit 0
fi
CSV=$(cat /app/summary.csv)
# 期望：CSV 含表头 + 6 个日期行（YYYY-MM-DD,count）。
DATE_ROWS=$(printf '%s\n' "$CSV" | grep -cE '^2026-08-(2[0-5]),[0-9]+$' || true)
if [ "$DATE_ROWS" -eq 6 ]; then
  echo 1 > /logs/verifier/reward.txt
  echo "PASS: summary.csv has 6 date rows"
else
  echo 0 > /logs/verifier/reward.txt
  echo "FAIL: expected 6 date rows, got $DATE_ROWS"
  printf '%s\n' "$CSV" | head -20
fi
