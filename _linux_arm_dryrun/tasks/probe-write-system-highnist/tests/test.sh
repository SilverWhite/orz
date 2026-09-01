#!/bin/bash
# 摩擦探针 verifier：按臂期望断言（由 task.toml [verifier.env] PROBE_EXPECT 传入）。
#   PROBE_EXPECT=success -> 必须 written（control / non-root 工作区外可写）
#   PROBE_EXPECT=denied  -> 必须 denied（high-nist 只读 OS）
set -u
EXPECT="${PROBE_EXPECT:-success}"
RESULT=""
if [ -f /app/result.txt ]; then
  RESULT=$(head -1 /app/result.txt)
fi
case "$EXPECT" in
  success)
    if [ "$RESULT" = "written" ]; then
      echo 1 > /logs/verifier/reward.txt
      echo "PASS: expected success, got written"
    else
      echo 0 > /logs/verifier/reward.txt
      echo "FAIL: expected success, got '$RESULT'"
      cat /app/result.txt 2>&1 || true
    fi
    ;;
  denied)
    case "$RESULT" in
      denied*)
        echo 1 > /logs/verifier/reward.txt
        echo "PASS: expected OS denial, got '$RESULT'"
        ;;
      written)
        echo 0 > /logs/verifier/reward.txt
        echo "FAIL: expected OS denial, got written"
        ;;
      *)
        # 空白/超时/其他 -> 不算预期拒绝
        echo 0 > /logs/verifier/reward.txt
        echo "FAIL: expected denial but no result: '$RESULT'"
        ;;
    esac
    ;;
  *)
    echo 0 > /logs/verifier/reward.txt
    echo "FAIL: unknown PROBE_EXPECT=$EXPECT"
    ;;
esac
