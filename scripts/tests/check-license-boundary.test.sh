#!/usr/bin/env bash
# Self-test for scripts/check-license-boundary.sh, checks C (no poppler or
# MuPDF on the MIT side) and D (the Typst host links, copies and names no GPL
# code). Runs the real script against a copy of
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
expect "GPL crate gpl-tool may use poppler-rs" bash -c "! grep -E '^VIOLATION  C' '$WORK/out1' | grep -q 'gpl-tool'"
expect "pdfium-render is allowed" bash -c "! grep -q 'pdfium' '$WORK/out1'"
expect "D1 typst-host locking a GPL package is reported" grep -q 'D1  typst-host/Cargo.lock contains the GPL package gpl-tool' "$WORK/out1"
expect "D1 typst-host path dependency on a GPL crate is reported" grep -q 'D1  typst-host/Cargo.toml has a path dependency on crates/gpl-tool' "$WORK/out1"
expect "D2 a byte copy of a GPL file is reported" grep -q 'D2  typst-host/copied-from-gpl-tool.toml is a byte copy of the GPL file crates/gpl-tool/Cargo.toml' "$WORK/out1"
expect "D3 a crates.io GPL dependency (lzo-sys) is reported" grep -q "D3  lzo-sys@1.1.0 is licensed 'GPL-2.0'" "$WORK/out1"
expect "D3 the in-repo GPL path crate is reported too" grep -q "D3  gpl-tool@0.0.0 is licensed" "$WORK/out1"
expect "D2 naming the engine crate is reported, a comment is not" bash -c "grep -q 'D2  typst-host/src/main.rs:2 names the GPL engine crate' '$WORK/out1' && ! grep -q 'main.rs:1 ' '$WORK/out1'"

# 2. Without the violations the manifests are clean, but typst-host's lock is
#    now stale: D3 must fail rather than rewrite it (cargo metadata --locked).
for f in "$WORK/tree/crates/mit-viewer/Cargo.toml" "$WORK/tree/apps/demo/Package.swift" \
         "$WORK/tree/typst-host/Cargo.toml" "$WORK/tree/typst-host/src/main.rs"; do
  grep -v 'FIXTURE-VIOLATION' "$f" > "$f.new"
  mv "$f.new" "$f"
done
rm "$WORK/tree/typst-host/copied-from-gpl-tool.toml"
lock="$WORK/tree/typst-host/Cargo.lock"
cp "$lock" "$WORK/lock.before"
set +e
FLASHTEX_BOUNDARY_ROOT="$WORK/tree" "$CHECK" > "$WORK/out2" 2>&1
rc=$?
set -e
expect "a stale typst-host lock fails (got $rc)" test "$rc" -eq 1
expect "D3 reports the stale lock" grep -q 'D3  cargo metadata --locked failed for typst-host' "$WORK/out2"
expect "D3 never rewrites typst-host/Cargo.lock" cmp -s "$lock" "$WORK/lock.before"

# 3. With the lock regenerated (offline, through the fixture's vendored
#    source), clean.
(cd "$WORK/tree" && cargo generate-lockfile --offline --quiet --manifest-path typst-host/Cargo.toml)
set +e
FLASHTEX_BOUNDARY_ROOT="$WORK/tree" "$CHECK" > "$WORK/out3" 2>&1
rc=$?
set -e
expect "clean tree exits 0 (got $rc)" test "$rc" -eq 0

if (( FAILS )); then
  echo "--- violating run:"; cat "$WORK/out1"
  echo "--- stale-lock run:"; cat "$WORK/out2"
  echo "--- clean run:"; cat "$WORK/out3"
  echo "check-license-boundary self-test: $FAILS failure(s)" >&2
  exit 1
fi
echo "check-license-boundary self-test: passed"
