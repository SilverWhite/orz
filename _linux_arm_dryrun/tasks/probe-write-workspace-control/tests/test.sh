#!/bin/bash
# 工作区探针：三臂都必须成功（验证策略不误伤合法路径）。
set -u
if [ -f /app/output.txt ] && [ "$(cat /app/output.txt)" = "probe-write-workspace-ok" ]; then
  echo 1 > /logs/verifier/reward.txt
  echo "PASS: workspace write ok"
else
  echo 0 > /logs/verifier/reward.txt
  echo "FAIL: workspace write missing or wrong"
  ls -la /app/ 2>&1 || true
  cat /app/output.txt 2>&1 || true
fi
