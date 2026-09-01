#!/bin/bash
# egress 探针 verifier：按臂期望断言。
#   PROBE_EXPECT=reachable -> control/non-root（全 egress）
#   PROBE_EXPECT=blocked   -> high-nist（allowlist 不含 example.com）
set -u
EXPECT="${PROBE_EXPECT:-reachable}"
RESULT=""
if [ -f /app/result.txt ]; then
  RESULT=$(head -1 /app/result.txt)
fi
case "$EXPECT" in
  reachable)
    if [ "$RESULT" = "reachable" ]; then
      echo 1 > /logs/verifier/reward.txt
    else
      echo 0 > /logs/verifier/reward.txt
      echo "FAIL: expected reachable, got '$RESULT'"
      cat /app/result.txt 2>&1 || true
    fi
    ;;
  blocked)
    if [ "$RESULT" = "blocked" ]; then
      echo 1 > /logs/verifier/reward.txt
    else
      echo 0 > /logs/verifier/reward.txt
      echo "FAIL: expected blocked, got '$RESULT'"
      cat /app/result.txt 2>&1 || true
    fi
    ;;
  *)
    echo 0 > /logs/verifier/reward.txt
    echo "FAIL: unknown PROBE_EXPECT=$EXPECT"
    ;;
esac
