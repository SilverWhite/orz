#!/bin/bash
# S3 smoke for the S5-2 build (2026-08-29): marker strings + bookworm
# startup behaviour of the Linux musl three-piece.
set +e
echo '== marker counts in /out/orz =='
for m in "tool_running" "still running after" "TIMED OUT" "hide_background_input" "terminal_tier_default_timeout_ms" "mid_run" "Command still running"; do
  printf '%-32s ' "$m"
  grep -a -c -F "$m" /out/orz
done
echo '== provision usage =='
/out/orz-acaf-provision 2>&1 | head -3
echo "exit=$?"
echo '== signer manifest missing =='
/out/orz-signer 2>&1 | head -3
echo "exit=$?"
echo '== orz tty io =='
/out/orz --real </dev/null 2>&1 | head -3
echo "exit=$?"
