#!/bin/sh
# pdfTeX's own regression tests (DESIGN.md section 8, tier T0) against the
# pdftex.web engine of crates/flashtex-engine.
#
#   scripts/pdftex-regression.sh                  # build the engine, run all
#   scripts/pdftex-regression.sh --engine BIN     # an engine already built
#
# The tests are TeX Live 2026's `pdftex_tests` (texk/web2c/pdftexdir/am/
# pdftex.am), unmodified, with the files they read, pinned in
# third_party/pdftex/regression/ (see its README.md). Each test script is run
# exactly as TeX Live's `make check` runs it -- `sh TEST` with `srcdir`
# pointing at the texk/web2c tree and `BinDir` at a directory whose `pdftex`
# is this engine -- in a directory of its own. The engine sets kpathsea up
# from its own path, as web2c does (FLASHTEX_RESOLVER=kpathsea-self), so
# texmf.cnf is the pinned texk/kpathsea/texmf.cnf the tests name in
# TEXMFCNF, and no TeX installation is needed.
#
# Exit status 0 iff every test passes, except those listed below as expected
# failures, each with the lane that owns it. A test script's status 77 means
# "skipped" (automake's convention); wcfname.test is skipped here when there
# is no `kpsewhich` or `perl` to run it with.
#
# The gate fails closed (#1207): an expected failure that now passes (XPASS),
# a test that runs past REGRESSION_TIMEOUT seconds (default 120; its whole
# process group is killed), and a run in which no test passed all exit 1. With
# REGRESSION_REQUIRE_ALL=1 a skipped test fails too. HOME, TMPDIR and the
# TEXMF{VAR,CONFIG,HOME} trees point into the work directory, so a run reads
# and writes nothing of the user's. A missing or non-executable --engine
# exits 2. scripts/tests/pdftex-regression.test.sh probes each of these with
# shim engines. python3 (for the timeout) is required: without it the script
# exits 2.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
srcdir=$root/third_party/pdftex/regression/texk/web2c
# Git Bash/MSYS2 on Windows: the tests put srcdir into TEXINPUTS as
# `DIR;.`, which MSYS's path conversion cannot translate from `/d/a/...`;
# `cygpath -m` gives the engine `D:/a/...` directly. No cygpath elsewhere.
srcdir=$(cygpath -m "$srcdir" 2>/dev/null || printf '%s' "$srcdir")

die() { echo "pdftex-regression: $*" >&2; exit 2; }

engine=
if [ "${1:-}" = "--engine" ]; then
    [ -n "${2:-}" ] || die "--engine needs a path"
    [ -d "$(dirname "$2")" ] || die "no such engine: $2"
    engine=$(cd "$(dirname "$2")" && pwd)/$(basename "$2")
    [ $# -le 2 ] || die "unexpected argument after --engine $2: $3"
elif [ -n "${1:-}" ]; then
    die "unknown argument: $1 (usage: $0 [--engine BIN])"
fi
if [ -z "$engine" ]; then
    (cd "$root" && cargo build --release --locked -p flashtex-engine)
    engine=$root/target/release/flashtex-initex
fi
[ -f "$engine" ] || die "no such engine: $engine"
[ -x "$engine" ] || die "engine is not executable: $engine"

timeout_s=${REGRESSION_TIMEOUT:-120}
case $timeout_s in
'' | *[!0-9]* | 0) die "REGRESSION_TIMEOUT must be a positive number of seconds, not '$timeout_s'" ;;
esac
# Seconds between SIGTERM and SIGKILL after a timeout (the self-test uses 1).
grace_s=${REGRESSION_KILL_GRACE:-5}
case $grace_s in
'' | *[!0-9]*) die "REGRESSION_KILL_GRACE must be a whole number of seconds, not '$grace_s'" ;;
esac
require_all=${REGRESSION_REQUIRE_ALL:-0}
# The timeout's process-group kill (run_test) needs python3: without it a
# hung engine would hold the job, so its absence is an error, not a warning.
command -v python3 >/dev/null 2>&1 || die "python3 is needed for the per-test timeout"

work=${REGRESSION_WORK:-$(mktemp -d)}
rm -rf "$work"
mkdir -p "$work/bin"
# A canonical path: kpathsea reads `//` in a path as "search this whole
# subtree", and TEXMFVAR and friends below point into $work. macOS's TMPDIR
# ends in `/`, so "$TMPDIR/x" has one, and partoken.test then searched until
# the timeout (#1386 review).
work=$(cd "$work" && pwd -P)
ln -s "$engine" "$work/bin/pdftex"

# The tests in pdftex.am's order.
tests="pdftexdir/wprob.test pdftexdir/pdftex.test pdftexdir/pdfimage.test
pdftexdir/expanded.test pdftexdir/tests/cnfline.test
pdftexdir/tests/partoken.test pdftexdir/wcfname.test"

# Expected failures: `test|owner|reason`, one per line. REGRESSION_XFAIL
# replaces the list (the self-test uses it; CI never sets it).
xfail=${REGRESSION_XFAIL-}  # none: pdfimage.test passes since #1202 (image and PDF inclusion)

export srcdir BinDir="$work/bin" ExeExt=
export FLASHTEX_POOL="$root/crates/flashtex-engine/pdftex.pool"
export FLASHTEX_RESOLVER=kpathsea-self
export SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1

# Nothing outside the work directory: no user texmf trees or caches, and
# nothing written to the real HOME or TMPDIR.
mkdir -p "$work/home" "$work/tmp" "$work/texmf-var" "$work/texmf-config" "$work/texmf-home"
export HOME="$work/home" TMPDIR="$work/tmp" TEXMFVAR="$work/texmf-var"
export TEXMFCONFIG="$work/texmf-config" TEXMFHOME="$work/texmf-home"

# run_test DIR TEST: `sh TEST` in DIR with stdin from /dev/null and output to
# DIR/test.out, killed with its whole process group (SIGTERM, then SIGKILL
# after $grace_s seconds) past $timeout_s seconds; prints nothing, returns the test's exit
# status, or 124 on a timeout. SIGKILL goes to the group after the grace
# period whether or not the test's own shell has exited: an engine that
# ignores SIGTERM must not outlive the run. Needs python3 (checked above).
run_test() {
    python3 - "$timeout_s" "$grace_s" "$1" "$2" <<'PY'
import os, signal, subprocess, sys
limit, grace, d, test = float(sys.argv[1]), float(sys.argv[2]), sys.argv[3], sys.argv[4]
with open(os.path.join(d, "test.out"), "wb") as out:
    p = subprocess.Popen(["sh", test], cwd=d, stdin=subprocess.DEVNULL,
                         stdout=out, stderr=subprocess.STDOUT,
                         start_new_session=True)
    try:
        sys.exit(p.wait(timeout=limit))
    except subprocess.TimeoutExpired:
        pass
    try:
        os.killpg(p.pid, signal.SIGTERM)
    except ProcessLookupError:
        pass
    try:
        p.wait(timeout=grace)
    except subprocess.TimeoutExpired:
        pass
    try:
        os.killpg(p.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    p.wait()
    sys.exit(124)
PY
}

kpsewhich=$(command -v kpsewhich || true)
fail=0
npass=0 nfail=0 nxfail=0 nxpass=0 nskip=0
for t in $tests; do
    name=$(basename "$t" .test)
    d=$work/$name
    mkdir -p "$d"
    ln -s "$work/bin/pdftex" "$d/pdftex" # wprob.test runs ./pdftex
    expected=pass
    owner=
    reason=
    old_ifs=$IFS
    IFS='
'
    for x in $xfail; do
        case $x in
        "$t|"*)
            expected=fail
            owner=$(echo "$x" | cut -d'|' -f2)
            reason=$(echo "$x" | cut -d'|' -f3-)
            ;;
        esac
    done
    IFS=$old_ifs
    if [ "$name" = wcfname ]; then
        if [ -z "$kpsewhich" ] || ! command -v perl >/dev/null 2>&1; then
            echo "SKIP  $t: needs kpsewhich and perl"
            nskip=$((nskip + 1))
            continue
        fi
        export KpsDir
        KpsDir=$(dirname "$kpsewhich")
    fi
    status=0
    run_test "$d" "$srcdir/$t" || status=$?
    if [ "$status" = 124 ]; then
        echo "FAIL  $t: timed out after ${timeout_s}s (process group killed); the last lines of its output:"
        tail -15 "$d/test.out" | sed 's/^/        /'
        nfail=$((nfail + 1))
        fail=1
    elif [ "$status" = 77 ]; then
        echo "SKIP  $t (the test skipped itself)"
        nskip=$((nskip + 1))
    elif [ "$status" = 0 ] && [ "$expected" = pass ]; then
        echo "PASS  $t"
        npass=$((npass + 1))
    elif [ "$status" = 0 ]; then
        echo "XPASS $t: listed as an expected failure ($owner: $reason); remove it from the list"
        nxpass=$((nxpass + 1))
        fail=1
    elif [ "$expected" = fail ]; then
        echo "XFAIL $t (exit $status): $owner -- $reason"
        nxfail=$((nxfail + 1))
    else
        echo "FAIL  $t (exit $status); the last lines of its output:"
        tail -15 "$d/test.out" | sed 's/^/        /'
        nfail=$((nfail + 1))
        fail=1
    fi
done
echo "pass $npass, fail $nfail, xfail $nxfail, xpass $nxpass, skip $nskip"
if [ "$npass" = 0 ]; then
    echo "no test passed: nothing was verified"
    fail=1
fi
if [ "$nskip" != 0 ] && [ "$require_all" = 1 ]; then
    echo "REGRESSION_REQUIRE_ALL=1 and $nskip test(s) skipped"
    fail=1
fi
echo "work dir: $work"
exit $fail
