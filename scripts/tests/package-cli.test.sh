#!/usr/bin/env bash
# Self-test for scripts/ci/package-cli.sh and the release workflow's use of
# it (app-parity row D5): the CLI tarball ships the new CLI (flashtex-v3)
# and the engine host (flashtex-host) beside the old `flashtex`, with the
# host's string pool and licence; a release that requires them fails
# without them; and .github/workflows/release.yml builds both, requires them
# in the tarball and requires the host in the app (make-app.sh
# --require-engine-host). No cargo: the binaries are stand-in scripts.
#
# Usage: scripts/tests/package-cli.test.sh   (exit 1 on a failure)
# Portable: bash 3.2 (macOS) and Linux. Run by apps/mac/scripts/packaging-selftest.sh
# and by `scripts/gate.sh pr`.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PKG="$ROOT/scripts/ci/package-cli.sh"
REL="$ROOT/.github/workflows/release.yml"
MAKE="$ROOT/apps/mac/scripts/make-app.sh"
ENGINE="$ROOT/crates/flashtex-engine"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/package-cli-test.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

FAIL=0
ok() { echo "  ok   $1"; }
bad() { echo "  FAIL $1" >&2; FAIL=$((FAIL + 1)); }
check() { # $1=description, then the command
  local what="$1"; shift
  if "$@"; then ok "$what"; else bad "$what"; fi
}

mkdir -p "$WORK/bins"
for b in flashtex flashtex-render flashtex-v3 flashtex-host; do
  printf '#!/bin/sh\necho %s\n' "$b" > "$WORK/bins/$b"
  chmod +x "$WORK/bins/$b"
done
ALL=()
for b in flashtex flashtex-render flashtex-v3 flashtex-host; do ALL+=(--bin "$WORK/bins/$b"); done

echo "==> package-cli.sh: the new CLI and the host, required"
if tarball="$("$PKG" 9.9.9 selftest "$WORK/out1" "${ALL[@]}" --require flashtex-v3,flashtex-host 2>"$WORK/err1")"; then
  mkdir -p "$WORK/x1"
  tar -C "$WORK/x1" -xzf "$tarball"
  d="$WORK/x1/flashtex-cli-9.9.9-selftest"
  for f in bin/flashtex bin/flashtex-render bin/flashtex-v3 bin/flashtex-host; do
    check "$f is in the tarball, executable" test -x "$d/$f"
  done
  check "bin/pdftex.pool is the engine's pool, beside the host" cmp -s "$d/bin/pdftex.pool" "$ENGINE/pdftex.pool"
  check "share/flashtex/engine/LICENSE is the engine's licence" cmp -s "$d/share/flashtex/engine/LICENSE" "$ENGINE/LICENSE"
  check "share/flashtex/engine/THIRD-PARTY-NOTICES is the engine's notices" cmp -s "$d/share/flashtex/engine/THIRD-PARTY-NOTICES" "$ENGINE/THIRD-PARTY-NOTICES"
  check "the notices carry jemalloc's" grep -q 'Copyright (C) 2002-present Jason Evans' "$d/share/flashtex/engine/THIRD-PARTY-NOTICES"
  check "README lists bin/flashtex-v3 and bin/flashtex-host" grep -q 'bin/flashtex-host`$' "$d/README.md"
  check "README shows flashtex-v3's usage" grep -q '^bin/flashtex-v3 build main.tex' "$d/README.md"
  check "README states the host's licence" grep -q 'GNU General Public License, version 2 or later' "$d/README.md"
  check "README names where the host's source is" grep -qF 'https://github.com/flash-tex/flashtex/tree/v9.9.9/crates/flashtex-engine' "$d/README.md"
  check "README's code fences are plain (no backslash)" bash -c '! grep -qF "\\\`" "$1"' _ "$d/README.md"
else
  bad "package-cli.sh with every binary and --require failed: $(head -3 "$WORK/err1" | tr '\n' ' ')"
fi

echo "==> package-cli.sh: a required host that was not built"
rc=0
"$PKG" 9.9.9 selftest "$WORK/out2" --bin "$WORK/bins/flashtex" --bin "$WORK/bins/flashtex-v3" --bin "$WORK/missing/flashtex-host" \
  --require flashtex-v3,flashtex-host >/dev/null 2>"$WORK/err2" || rc=$?
check "exits 1" test "$rc" -eq 1
check "names the missing binary" grep -q 'required binary flashtex-host was not built' "$WORK/err2"
check "writes no tarball" test ! -e "$WORK/out2/flashtex-cli-9.9.9-selftest.tar.gz"

echo "==> package-cli.sh: without --require a missing host is listed, as before"
if tarball="$("$PKG" 9.9.9 selftest "$WORK/out3" --bin "$WORK/bins/flashtex" --bin "$WORK/bins/flashtex-v3" --bin "$WORK/missing/flashtex-host" 2>"$WORK/err3")"; then
  mkdir -p "$WORK/x3"
  tar -C "$WORK/x3" -xzf "$tarball"
  d="$WORK/x3/flashtex-cli-9.9.9-selftest"
  check "README lists flashtex-host as not built" grep -q 'bin/flashtex-host` — not built for selftest' "$d/README.md"
  check "no pool without the host" test ! -e "$d/bin/pdftex.pool"
  check "no engine licence without the host" test ! -e "$d/share/flashtex/engine"
else
  bad "package-cli.sh without --require failed: $(head -3 "$WORK/err3" | tr '\n' ' ')"
fi

echo "==> release.yml ships the new CLI and the host"
check "builds flashtex-host" grep -q -- '-p flashtex-engine --bin flashtex-host' "$REL"
check "builds flashtex-v3" grep -q -- '-p flashtex-build --bin flashtex-v3' "$REL"
check "requires the host in the app (make-app.sh --require-engine-host)" grep -q -- 'args=(--version "$VERSION" --dmg --require-engine-host)' "$REL"
check "requires flashtex-v3 and flashtex-host in the macOS CLI tarball" grep -q -- '--require flashtex-v3,flashtex-host' "$REL"
check "make-app.sh takes --require-engine-host" grep -q -- '--require-engine-host)' "$MAKE"
check "smoke steps never pipe a program into grep -q (SIGPIPE under pipefail)" bash -c '! grep -nE -- "--help \| grep -q" "$1"' _ "$REL"

if [[ "$FAIL" -gt 0 ]]; then
  echo "package-cli self-test: $FAIL failure(s)" >&2
  exit 1
fi
echo "package-cli self-test: all passed"
