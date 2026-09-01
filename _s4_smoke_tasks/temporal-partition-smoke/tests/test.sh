#!/bin/bash
if [ -f /app/answer.txt ] && [ "$(cat /app/answer.txt)" = "temporal-smoke-ok-2026-08-31" ]; then
  echo 1 > /logs/verifier/reward.txt
else
  echo 0 > /logs/verifier/reward.txt
  echo "answer.txt content:"
  cat /app/answer.txt 2>&1 || true
fi
