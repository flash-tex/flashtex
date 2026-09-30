#!/usr/bin/env bash
# Self-test for scripts/check-license-boundary.sh, check C (no poppler or
# MuPDF on the MIT side). Runs the real script against a copy of
# scripts/tests/fixtures/license-boundary-copyleft-pdf:
#
#   1. as is: an MIT crate reaching mupdf-sys and an app linking mupdf must
#      both be reported, and the check must exit 1; the GPL crate using
#      poppler-rs and the MIT crate using pdfium-render must not be.
#   2. with the two FIXTURE-VIOLATION lines removed: the check must pass.
#
# Needs cargo (offline is fine: every dependency is a path) and python3.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHECK="$HERE/../check-license-boundary.sh"
FIXTURE="$HERE/fixtures/license-boundary-copyleft-pdf"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/flashtex-boundary-test-XXXXXX")"
trap 'rm -rf "$WORK"' EXIT
cp -R "$FIXTURE" "$WORK/tree"
export CARGO_TARGET_DIR="$WORK/target"

FAILS=0
expect() { # expect <description> <command...>
  local what="$1"; shift
  if "$@"; then printf 'ok    %s\n' "$what"; else printf 'FAIL  %s\n' "$what" >&2; FAILS=$((FAILS + 1)); fi
}

# 1. The violations fire.
set +e
FLASHTEX_BOUNDARY_ROOT="$WORK/tree" "$CHECK" > "$WORK/out1" 2>&1
rc=$?
set -e
expect "violating tree exits 1 (got $rc)" test "$rc" -eq 1
expect "mit-viewer -> mupdf-sys is reported" grep -q 'MIT crate mit-viewer reaches a copyleft PDF renderer: mupdf-sys <- mit-viewer' "$WORK/out1"
expect "the app's linkedLibrary(\"mupdf\") is reported" grep -q 'C2  app build input names a copyleft PDF renderer: apps/demo/Package.swift:8:' "$WORK/out1"
expect "the comment naming poppler is not reported" bash -c "! grep -q 'Package.swift:2:' '$WORK/out1'"
expect "GPL crate gpl-tool may use poppler-rs" bash -c "! grep -q 'gpl-tool' '$WORK/out1'"
expect "pdfium-render is allowed" bash -c "! grep -q 'pdfium' '$WORK/out1'"

# 2. Without the violations, clean.
for f in "$WORK/tree/crates/mit-viewer/Cargo.toml" "$WORK/tree/apps/demo/Package.swift"; do
  grep -v 'FIXTURE-VIOLATION' "$f" > "$f.new"
  mv "$f.new" "$f"
done
set +e
FLASHTEX_BOUNDARY_ROOT="$WORK/tree" "$CHECK" > "$WORK/out2" 2>&1
rc=$?
set -e
expect "clean tree exits 0 (got $rc)" test "$rc" -eq 0

if (( FAILS )); then
  echo "--- violating run:"; cat "$WORK/out1"
  echo "--- clean run:"; cat "$WORK/out2"
  echo "check-license-boundary self-test: $FAILS failure(s)" >&2
  exit 1
fi
echo "check-license-boundary self-test: passed"
