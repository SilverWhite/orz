#!/bin/bash
if [ -f /app/answer.txt ] && [ "$(cat /app/answer.txt)" = "dependency-graph-smoke-ok-2026-09-01" ]; then
  echo 1 > /logs/verifier/reward.txt
else
  echo 0 > /logs/verifier/reward.txt
  echo "answer.txt content:"
  cat /app/answer.txt 2>&1 || true
fi
