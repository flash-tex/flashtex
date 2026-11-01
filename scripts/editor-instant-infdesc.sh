#!/usr/bin/env bash
# APP-EDITOR-INSTANT evidence: type into the editor during a real cold compile of
# Infinite Descent ×2 (1,142 pages; tools/parity/corpus/infdesc_x2.py) and report
# key → glyph latency and main-thread stalls (EditorInstantTests.testTypingDuringARealColdCompile).
#
# Needs: a Mac with a full TeX Live (MacTeX), Rust, Xcode. NOT the owner's Mac.
#
# Usage: scripts/editor-instant-infdesc.sh [out.json]
#   KEYS=120            keys typed during the cold compile (50 ms apart)
#   BUDGET_MS=8         fail when key → glyph p99 exceeds this (default: report only)
#   WORK=/tmp/ei-infdesc   where the book is unpacked
set -euo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
out="${1:-$PWD/editor-instant-infdesc.json}"
work="${WORK:-${TMPDIR:-/tmp}/ei-infdesc}"
url="https://codeberg.org/cnewstead/infdesc/archive/48825c5e50bf311b818d666cece6e731d8a0191d.tar.gz"
sha="40570a5cde51807c60bdb322bfc17d9dc96993dae1125ce13b6361333e4b929e"

mkdir -p "$work"
tgz="$work/infdesc.tar.gz"
if [ ! -f "$tgz" ] || [ "$(shasum -a 256 "$tgz" | cut -d' ' -f1)" != "$sha" ]; then
  curl -fsSL "$url" -o "$tgz"
fi
[ "$(shasum -a 256 "$tgz" | cut -d' ' -f1)" = "$sha" ] || { echo "infdesc: sha256 mismatch" >&2; exit 1; }
rm -rf "$work/infdesc" && tar -xzf "$tgz" -C "$work"

gen="$here/tools/parity/corpus/infdesc_x2.py"
if [ ! -f "$gen" ]; then
  gen="$work/infdesc_x2.py"
  git -C "$here" show origin/agent/kabir-claude/infdesc-x2:tools/parity/corpus/infdesc_x2.py > "$gen"
fi
doc="$(python3 "$gen" "$work/infdesc")"
echo "document: $doc"

(cd "$here" && cargo build --release --locked -p flashtex-engine --bin flashtex-host)
export FLASHTEX_HOST="$here/target/release/flashtex-host"
export FLASHTEX_REQUIRE_HOST=1
export FLASHTEX_EDITOR_INSTANT_DOC="$doc"
export FLASHTEX_EDITOR_INSTANT_OUT="$out"
export FLASHTEX_EDITOR_INSTANT_KEYS="${KEYS:-120}"
export FLASHTEX_V3_CACHE="$work/v3-cache"
[ -n "${BUDGET_MS:-}" ] && export FLASHTEX_EDITOR_KEY_BUDGET_MS="$BUDGET_MS"
cd "$here/apps/mac"
swift build -c release -Xswiftc -enable-testing --build-tests
swift test -c release -Xswiftc -enable-testing --skip-build --filter 'EditorInstantTests' 2>&1 | grep -E 'EditorInstant:|error|passed|failed|skipped' || true
echo "summary: $out"
