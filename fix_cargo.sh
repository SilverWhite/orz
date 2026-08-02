#!/bin/bash
# Fix all Cargo.toml files: remove dead deps, rename xai-grok-* -> orz-*
# Run from /b/orz

set -e
cd /b/orz

# Deleted crate names
DELETED="xai-sqlite-journal|xai-grok-telemetry|xai-grok-auth|xai-grok-update|xai-grok-voice|xai-grok-announcements|xai-grok-plugin-marketplace|xai-mixpanel|xai-grok-subagent-resolution|xai-grok-sampler|xai-codebase-graph|xai-grok-secrets|xai-grok-shell-base|xai-grok-shell-session-support|xai-chat-state|xai-grok-mermaid|xai-grok-test-support"

for f in $(find crates -name "Cargo.toml" -not -path "*/target/*"); do
  echo "Processing: $f"
  # Remove lines that depend on deleted crates
  sed -i -E "/\"?($DELETED)\"? = /d" "$f"
  # Also remove feature references to deleted crates
  sed -i -E "s/\"dep:($DELETED)\",?//g" "$f"
  # Rename xai-grok-* to orz-* in paths and dep names (but NOT xai-grok-pager*)
  perl -i -pe 's/xai-grok-(?!pager)(\w[\w-]*)/orz-$1/g' "$f"
  # Fix path references for renamed crates (xai-grok- -> orz-)
  perl -i -pe 's|"(\.\./)+xai-grok-(?!pager)(\w[\w-]*)/|"$1../orz-$2/|g' "$f"
  perl -i -pe 's|crates/codegen/xai-grok-(?!pager)(\w[\w-]*)|crates/codegen/orz-$1|g' "$f"
  perl -i -pe 's|crates/common/xai-grok-(\w[\w-]*)|crates/common/orz-$1|g' "$f"
done

echo "All Cargo.toml files processed."
