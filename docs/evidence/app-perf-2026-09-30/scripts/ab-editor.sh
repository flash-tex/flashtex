#!/usr/bin/env bash
# APP-PERF-AUDIT: interleaved before/after of the editor fixes, editor-only
# (no producer). "old" = FLASHTEX_PERF_OLD=1, a temporary switch (removed
# before commit) that restored the pre-fix code paths in the same binary.
# usage: ab-editor.sh <doc> <at-needle> <rounds>
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOC="$1"; AT="$2"; ROUNDS="$3"
TAG="$(echo "$AT" | tr -c 'A-Za-z0-9' '_' | cut -c1-20)"
for r in $(seq 1 "$ROUNDS"); do
  "$HERE/editor-only.sh" "old$r-$TAG" "$DOC" "$AT" --env FLASHTEX_PERF_OLD=1 | sed -n '1,3p'
  "$HERE/editor-only.sh" "new$r-$TAG" "$DOC" "$AT" | sed -n '1,3p'
done
