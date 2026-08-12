#!/usr/bin/env bash
#
# Fixed bridge launcher for the Structured Operation Protocol v0.1 (POSIX).
#
# Usage:
#   ops_bridge.sh <endpoint> < op.json
#
# v0.1 implements docker and ssh endpoints. The operation JSON is streamed via
# stdin (UTF-8) to the fixed executor inside the target; no command text
# crosses the boundary. Endpoint allowlist: ops-bridges.json.

set -euo pipefail

endpoint="${1:?usage: ops_bridge.sh <endpoint>}"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
config="$script_dir/ops-bridges.json"
tmp="$(mktemp "${TMPDIR:-/tmp}/ops-op-XXXXXX.json")"
trap 'rm -f "$tmp"' EXIT

cat > "$tmp"

ep_line="$(python3 - "$config" "$endpoint" <<'PY'
import json
import sys

cfg = json.load(open(sys.argv[1], encoding="utf-8"))
try:
    ep = cfg["endpoints"][sys.argv[2]]
except KeyError:
    print("unknown bridge endpoint: " + sys.argv[2], file=sys.stderr)
    sys.exit(1)
print(ep["type"], ep["target"], ep["interpreter"], ep["executor"])
PY
)"
if [ $? -ne 0 ]; then
    echo "bridge endpoint lookup failed for: $endpoint" >&2
    exit 1
fi

read -r ep_type ep_target ep_interpreter ep_executor <<< "$ep_line"

if [ -z "$ep_type" ]; then
    echo "bridge endpoint configuration missing for: $endpoint" >&2
    exit 1
fi

case "$ep_type" in
    docker)
        docker exec -i -e "OPS_SOURCE=bridge:$endpoint" "$ep_target" "$ep_interpreter" "$ep_executor" --op-file - < "$tmp"
        ;;
    ssh)
        ssh "$ep_target" "OPS_SOURCE=bridge:$endpoint $ep_interpreter $ep_executor --op-file -" < "$tmp"
        ;;
    *)
        echo "unsupported endpoint type: $ep_type" >&2
        exit 1
        ;;
esac
