#!/usr/bin/env bash
# Self-test for scripts/pdftex-regression.sh: the false-pass and hang paths of
# #1207 that apply to it, each probed with a shim engine.
#
#   1. a missing or non-executable engine, a missing --engine argument, an
#      extra or unknown argument and a bad REGRESSION_TIMEOUT exit 2;
#   2. an engine that never returns fails within REGRESSION_TIMEOUT, its
#      process group killed (no surviving sleeper);
#   3. an engine that fails everything, with every test listed as an expected
#      failure, is "nothing verified" and fails;
#   4. HOME and TMPDIR seen by the engine are inside the work directory, and
#      contain no `//` even when the work path given does;
#   with the real engine (target/release/flashtex-initex, or $FLASHTEX_ENGINE),
#   skipped where it is not built:
#   5. it passes every test;
#   6. a test listed as an expected failure that passes (XPASS) fails the run;
#   7. with no kpsewhich on PATH wcfname skips: exit 0, but 1 under
#      REGRESSION_REQUIRE_ALL=1.
#
# Needs python3 (the runner's timeout). Takes about 20 seconds.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
RUN="$ROOT/scripts/pdftex-regression.sh"
# Without TMPDIR's trailing `/` (macOS has one); the runner normalises its own
# work path too, and case 4 checks the paths it hands the engine.
tmp="${TMPDIR:-/tmp}"
WORK="$(mktemp -d "${tmp%/}/flashtex-regression-test-XXXXXX")"
WORK="$(cd "$WORK" && pwd -P)"
trap 'rm -rf "$WORK"' EXIT

FAILS=0
expect() { # expect <description> <command...>
  local what="$1"; shift
  if "$@"; then printf 'ok    %s\n' "$what"; else printf 'FAIL  %s\n' "$what" >&2; FAILS=$((FAILS + 1)); fi
}
shim() { # shim <name> <body>: an executable sh script in $WORK/shims
  mkdir -p "$WORK/shims"
  printf '#!/bin/sh\n%s\n' "$2" > "$WORK/shims/$1"
  chmod +x "$WORK/shims/$1"
  echo "$WORK/shims/$1"
}
run() { # run <name> [env...] -- <args...>: the runner's exit status in $rc, output in $WORK/<name>.out
  local name="$1"; shift
  local envs=()
  while [[ $# -gt 0 && "$1" != -- ]]; do envs+=("$1"); shift; done
  shift
  set +e
  env ${envs[@]+"${envs[@]}"} REGRESSION_WORK="$WORK/run-$name" "$RUN" "$@" > "$WORK/$name.out" 2>&1
  rc=$?
  set -e
}

ALL_XFAIL="$(for t in pdftexdir/wprob.test pdftexdir/pdftex.test pdftexdir/pdfimage.test \
  pdftexdir/expanded.test pdftexdir/tests/cnfline.test pdftexdir/tests/partoken.test \
  pdftexdir/wcfname.test; do printf '%s|self-test|shim\n' "$t"; done)"

# 1. Engine arguments.
run missing -- --engine "$WORK/no-such-dir/pdftex"
expect "a missing engine exits 2 (got $rc)" test "$rc" -eq 2
touch "$WORK/not-exec"
run notexec -- --engine "$WORK/not-exec"
expect "a non-executable engine exits 2 (got $rc)" test "$rc" -eq 2
run noarg -- --engine
expect "--engine without a path exits 2 (got $rc)" test "$rc" -eq 2
run badarg -- --bogus
expect "an unknown argument exits 2 (got $rc)" test "$rc" -eq 2
run badtimeout REGRESSION_TIMEOUT=abc -- --engine "$(shim ok 'exit 0')"
expect "a non-numeric REGRESSION_TIMEOUT exits 2 (got $rc)" test "$rc" -eq 2
run extraarg -- --engine "$(shim ok 'exit 0')" stray
expect "an argument after --engine BIN exits 2 (got $rc)" test "$rc" -eq 2

# 2. A hanging engine: each test is killed at the timeout, the run fails, and
#    no engine survives. Each hanging engine records its pid, ignores SIGTERM
#    and loops, so only the SIGKILL after the grace period stops it.
hang="$(shim hang "echo \$\$ >> '$WORK/hang.pids'; trap '' TERM; while :; do sleep 1; done")"
start=$SECONDS
run hang REGRESSION_TIMEOUT=1 REGRESSION_KILL_GRACE=1 -- --engine "$hang"
took=$((SECONDS - start))
expect "a hanging engine fails the run (got $rc)" test "$rc" -eq 1
expect "a hanging engine is reported as a timeout" grep -q 'timed out after 1s' "$WORK/hang.out"
expect "a hanging engine costs about the timeout per test, not forever (${took}s)" test "$took" -lt 60
survivors() { # the recorded hanging engines still running (a zombie
  # waiting for its new parent to reap it is not running)
  local pid st
  while read -r pid; do
    st="$(ps -o stat= -p "$pid" 2>/dev/null || true)"
    [[ -n "$st" && "$st" != Z* ]] && echo "$pid"
  done < "$WORK/hang.pids"
  return 0
}
sleep 1
expect "the hanging engines ran" test -s "$WORK/hang.pids"
expect "no hanging engine survives (alive: $(survivors | tr '\n' ' '))" test -z "$(survivors)"

# 3. Nothing verified: every test an expected failure, an engine that fails.
fail_engine="$(shim fail 'exit 1')"
run nothing "REGRESSION_XFAIL=$ALL_XFAIL" -- --engine "$fail_engine"
expect "a run in which no test passed fails (got $rc)" test "$rc" -eq 1
expect "it says nothing was verified" grep -q 'no test passed' "$WORK/nothing.out"

# 4. The environment the engine sees.
spy="$(shim spy "echo \"\$HOME|\$TMPDIR|\$TEXMFVAR|\$TEXMFHOME\" >> '$WORK/spy.txt'; exit 1")"
run spy -- --engine "$spy"
expect "the spy engine ran" test -s "$WORK/spy.txt"
expect "HOME, TMPDIR and TEXMF* are inside the work dir" \
  bash -c "! grep -v '^$WORK/run-spy/home|$WORK/run-spy/tmp|$WORK/run-spy/texmf-var|$WORK/run-spy/texmf-home\$' '$WORK/spy.txt'"

# 4b. A work path with `//` (macOS's TMPDIR ends in `/`): kpathsea reads it
#     as "search the whole subtree", so the runner must canonicalise it.
: > "$WORK/spy.txt"
set +e
env REGRESSION_WORK="$WORK//run-slashes" "$RUN" --engine "$spy" > "$WORK/slashes.out" 2>&1
set -e
expect "the engine sees no // in its paths" bash -c "test -s '$WORK/spy.txt' && ! grep -q '//' '$WORK/spy.txt'"

# 5-7. The real engine.
ENGINE="${FLASHTEX_ENGINE:-$ROOT/target/release/flashtex-initex}"
if [[ -x "$ENGINE" ]]; then
  run real -- --engine "$ENGINE"
  expect "the real engine passes (got $rc)" test "$rc" -eq 0
  expect "the summary counts 7 passes or 6 and a skip" \
    grep -Eq '^pass (7, fail 0, xfail 0, xpass 0, skip 0|6, fail 0, xfail 0, xpass 0, skip 1)$' "$WORK/real.out"

  run xpass "REGRESSION_XFAIL=pdftexdir/wprob.test|self-test|shim" -- --engine "$ENGINE"
  expect "an expected failure that passes fails the run (got $rc)" test "$rc" -eq 1
  expect "it is reported as XPASS" grep -q '^XPASS pdftexdir/wprob.test' "$WORK/xpass.out"

  # A PATH with everything but kpsewhich: wcfname skips.
  mkdir -p "$WORK/nokpse"
  for tool in sh cat cp mv rm mkdir grep sed tail cut dirname basename ln \
              perl python3 diff cmp locale env sort uniq tr head wc touch ls test expr; do
    p="$(command -v "$tool" 2>/dev/null || true)"
    [[ -n "$p" && "$p" == /* ]] && ln -sf "$p" "$WORK/nokpse/$tool"
  done
  run skip "PATH=$WORK/nokpse" -- --engine "$ENGINE"
  expect "without kpsewhich wcfname skips" grep -q '^SKIP  pdftexdir/wcfname.test' "$WORK/skip.out"
  expect "a skip alone does not fail the run (got $rc)" test "$rc" -eq 0
  run skipstrict "PATH=$WORK/nokpse" REGRESSION_REQUIRE_ALL=1 -- --engine "$ENGINE"
  expect "REGRESSION_REQUIRE_ALL=1 fails on a skip (got $rc)" test "$rc" -eq 1
else
  echo "skip  the real-engine cases: no engine at $ENGINE (cargo build --release -p flashtex-engine)"
fi

if [[ $FAILS -ne 0 ]]; then
  echo "$FAILS check(s) failed" >&2
  exit 1
fi
echo "all checks passed"
