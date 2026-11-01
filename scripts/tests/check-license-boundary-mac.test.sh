#!/usr/bin/env bash
# Self-test for scripts/check-license-boundary.sh, check E (the MIT Mac app
# links no GPL code). Runs the real script against a copy of
# scripts/tests/fixtures/license-boundary-mac, a tree with no cargo workspace
# whose only crate, gpl-host, is GPL:
#
#   1. as is, plus a symlink from apps/mac into the GPL crate: a linked GPL
#      library in Package.swift, -Xlinker in make-app.sh, a staticlib GPL
#      crate and the symlink must each be reported, and the check must exit 1.
#      Comments, .linkedFramework, a Swift source that names the host's path
#      and make-app.sh copying the host executable must not be.
#      So must the review's false negatives (#1400): a module map's
#      `link "..."`, a multi-line OTHER_LDFLAGS list in project.pbxproj naming
#      -l<gpl crate>, a shell case arm `*) ... -Xlinker`, and link flags in
#      scripts under apps/mac/scripts/lib/ and directly in apps/mac/.
#   2. with the FIXTURE-VIOLATION lines and the symlink removed: it must pass.
#
# Needs only bash and python3 (no cargo: the fixture has no workspace).
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHECK="$HERE/../check-license-boundary.sh"
FIXTURE="$HERE/fixtures/license-boundary-mac"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/flashtex-boundary-mac-test-XXXXXX")"
trap 'rm -rf "$WORK"' EXIT
cp -R "$FIXTURE" "$WORK/tree"
mkdir -p "$WORK/tree/crates/gpl-host/src"
ln -s ../../../crates/gpl-host/src "$WORK/tree/apps/mac/Sources/Vendored"

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
expect "E1 Package.swift naming the GPL crate is reported" grep -q 'E1  apps/mac/Package.swift:9 names the GPL crate gpl_host' "$WORK/out1"
expect "E2 the linkedLibrary is reported" grep -q 'E2  apps/mac/Package.swift:9 declares SwiftPM linked library' "$WORK/out1"
expect "E3 -Xlinker in make-app.sh is reported" grep -q 'E3  apps/mac/scripts/make-app.sh:4 passes link flags to the build' "$WORK/out1"
expect "E3 a staticlib GPL crate is reported" grep -q 'E3  crates/gpl-host/Cargo.toml:9 makes GPL crate gpl-host linkable' "$WORK/out1"
expect "E4 the symlink into the GPL crate is reported" grep -q 'E4  apps/mac/Sources/Vendored resolves into the GPL crate crates/gpl-host' "$WORK/out1"
expect "comments are not reported" bash -c "! grep -qE 'Package.swift:2 |make-app.sh:2 ' '$WORK/out1'"
expect ".linkedFramework is allowed" bash -c "! grep -q 'Package.swift:8 ' '$WORK/out1'"
expect "copying the host executable is allowed" bash -c "! grep -q 'make-app.sh:5 ' '$WORK/out1'"
expect "a Swift source naming the host's path is not scanned" bash -c "! grep -q 'Host.swift' '$WORK/out1'"
expect "E1 a module map linking the GPL crate is reported" grep -q 'E1  apps/mac/Sources/CGPL/module.modulemap:4 names the GPL crate gpl_host' "$WORK/out1"
expect "E2 a module map's link line is reported" grep -q 'E2  apps/mac/Sources/CGPL/module.modulemap:4 declares module-map linked library' "$WORK/out1"
expect "a module map's link framework is allowed" bash -c "! grep -q 'module.modulemap:5 ' '$WORK/out1'"
expect "E1 -l<GPL crate> in project.pbxproj is reported" grep -q 'E1  apps/mac/App.xcodeproj/project.pbxproj:12 names the GPL crate gpl_host' "$WORK/out1"
expect "E2 a multi-line OTHER_LDFLAGS list is reported" grep -q 'E2  apps/mac/App.xcodeproj/project.pbxproj:10 declares OTHER_LDFLAGS naming a library' "$WORK/out1"
expect "a pbxproj block comment and LIBRARY_SEARCH_PATHS = (\$(inherited)) are not reported" bash -c "! grep -qE 'project.pbxproj:(4|7) ' '$WORK/out1'"
expect "E3 a shell case arm '*)' with -Xlinker is reported" grep -q 'E3  apps/mac/scripts/release.sh:5 passes link flags to the build' "$WORK/out1"
expect "E3 a script in apps/mac/scripts/lib is scanned" grep -q 'E3  apps/mac/scripts/lib/build.sh:3 passes link flags to the build' "$WORK/out1"
expect "E3 a script directly in apps/mac is scanned" grep -q 'E3  apps/mac/build.sh:3 passes link flags to the build' "$WORK/out1"
expect "script comments are not reported" bash -c "! grep -qE 'release.sh:2 |lib/build.sh:2 |mac/build.sh:2 ' '$WORK/out1'"
expect "exactly twelve violations" grep -q 'licence boundary: 12 violation(s)' "$WORK/out1"

# 2. Without the violations, clean.
grep -rl 'FIXTURE-VIOLATION' "$WORK/tree" > "$WORK/violating"
expect "eight fixture files carry violations" test "$(wc -l < "$WORK/violating")" -eq 8
while IFS= read -r f; do
  grep -v 'FIXTURE-VIOLATION' "$f" > "$f.new"
  mv "$f.new" "$f"
done < "$WORK/violating"
rm "$WORK/tree/apps/mac/Sources/Vendored"
set +e
FLASHTEX_BOUNDARY_ROOT="$WORK/tree" "$CHECK" > "$WORK/out2" 2>&1
rc=$?
set -e
expect "clean tree exits 0 (got $rc)" test "$rc" -eq 0
expect "clean tree runs all four E checks" test "$(grep -cE '^ok +E[1-4] ' "$WORK/out2")" -eq 4

if (( FAILS )); then
  echo "--- violating run:"; cat "$WORK/out1"
  echo "--- clean run:"; cat "$WORK/out2"
  echo "check-license-boundary (apps/mac) self-test: $FAILS failure(s)" >&2
  exit 1
fi
echo "check-license-boundary (apps/mac) self-test: passed"
