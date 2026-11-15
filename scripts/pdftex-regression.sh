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
# exits 2. python3 (for the timeout) is required: without it the script
# exits 2.
#
# Upstream's scripts check only what they print or diff, so an engine that
# writes the right log and then crashes, or one that writes nothing and exits
# 0, passed some of them (#1207). The runner therefore also checks, for each
# test that passed upstream's script (a failure here turns its PASS into a
# FAIL; an expected failure is not checked):
#   - exits: every engine run's exit status, recorded by a wrapper that
#     stands in for `pdftex`, is the one TeX Live's pdftex 1.40.29 gives in
#     the same run (`expected_exits` below; a death by signal is never one);
#   - outputs: the files the test exists to make are there, are regular
#     files, and hold what TeX Live's pdftex writes into them
#     (`check_outputs` below: the version line, the PDF's header and trailer,
#     the logs' result lines, wcfname's \write file byte for byte);
#   - links: the test directory holds no symbolic link the engine made (an
#     output linked to upstream's expected file would satisfy `diff`).
# The engine can still read upstream's expected files (TEXINPUTS names their
# directory, as upstream's scripts set it): the checks above are what keep a
# copy of them from passing. scripts/tests/pdftex-regression.test.sh probes
# each of these paths with shim engines.
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
    engine=${CARGO_TARGET_DIR:-$root/target}/release/flashtex-initex
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
case $work in *"'"*) die "the work directory's path has a quote in it: $work" ;; esac
# `pdftex` is a wrapper that runs the engine (through a link of the same
# name, so the engine sees itself called `pdftex`) and appends each run's
# exit status to $REGRESSION_EXITS (set per test below).
mkdir -p "$work/engine"
ln -s "$engine" "$work/engine/pdftex"
cat >"$work/bin/pdftex" <<EOF
#!/bin/sh
'$work/engine/pdftex' "\$@"
rc=\$?
echo "\$rc" >>"\$REGRESSION_EXITS"
exit "\$rc"
EOF
chmod +x "$work/bin/pdftex"

# The tests in pdftex.am's order.
tests="pdftexdir/wprob.test pdftexdir/pdftex.test pdftexdir/pdfimage.test
pdftexdir/expanded.test pdftexdir/tests/cnfline.test
pdftexdir/tests/partoken.test pdftexdir/wcfname.test"

# Expected failures: `test|owner|reason`, one per line. REGRESSION_XFAIL
# replaces the list (the self-test uses it; CI never sets it).
xfail=${REGRESSION_XFAIL-}  # none: pdfimage.test passes since #1202 (image and PDF inclusion)

# expected_exits NAME: the exit statuses of the test's engine runs, in order,
# as TeX Live 2026's pdftex 1.40.29 gives them in this runner (measured
# 2026-11-08 with `--engine ~/texlive/2026/bin/x86_64-linux/pdftex`). A run
# that ends in an error exits 1 (web2c's final_cleanup): wprob's,
# expanded's (each `\show` counts as an error, §1293) and partoken-xfail's 1
# are part of the test.
expected_exits() {
    case $1 in
    wprob) echo 1 ;;
    pdftex) echo 0 0 ;;
    pdfimage) echo 0 0 ;;
    expanded) echo 1 ;;
    cnfline) echo 0 ;;
    partoken) echo 0 1 ;;
    wcfname) echo 0 0 0 0 ;;
    *) echo "?" ;;
    esac
}

# has_line FILE LINE: FILE is a regular file with a line exactly LINE.
has_line() { [ -f "$1" ] && [ ! -L "$1" ] && grep -Fxq -- "$2" "$1"; }

# check_outputs NAME DIR: what the test exists to make, against what TeX
# Live's pdftex makes in the same run (the lines and bytes below are its).
# Prints each problem; returns 1 if there is one.
check_outputs() {
    n=$1 d=$2 bad=0
    case $n in
    pdftex)
        # The version line, whole: `pdfTeX 3.141592653-2.6-1.40.290 (...)`
        # or `pdfTeX 11.40.29` must not pass. The parenthesis is the
        # distribution (TeX Live's `(TeX Live 2026)`, this engine's own).
        v=$(sed -n 1p "$d/test.out")
        case $v in
        "pdfTeX 3.141592653-2.6-1.40.29 ("*")") ;;
        *) echo "        --version's first line is not pdfTeX 1.40.29's: '$v'"; bad=1 ;;
        esac
        has_line "$d/test.out" 'Usage: pdftex [OPTION]... [TEXNAME[.tex]] [COMMANDS]' ||
            { echo "        --help printed no 'Usage: pdftex' line"; bad=1; }
        ;;
    pdfimage)
        for f in pdfimage.fmt pdfimage.pdf; do
            if [ ! -f "$d/$f" ] || [ -L "$d/$f" ] || [ ! -s "$d/$f" ]; then
                echo "        $f: not written (or empty, or not a regular file)"; bad=1
            fi
        done
        if [ -f "$d/pdfimage.pdf" ] && [ ! -L "$d/pdfimage.pdf" ]; then
            [ "$(head -c 5 "$d/pdfimage.pdf")" = "%PDF-" ] ||
                { echo "        pdfimage.pdf does not start with %PDF-"; bad=1; }
            [ "$(tail -n 1 "$d/pdfimage.pdf")" = "%%EOF" ] ||
                { echo "        pdfimage.pdf does not end with %%EOF"; bad=1; }
        fi
        grep -q '^Output written on pdfimage\.pdf (3 pages, [0-9]* bytes)\.$' "$d/pdfimage.log" 2>/dev/null ||
            { echo "        pdfimage.log: no 'Output written on pdfimage.pdf (3 pages, ...)'"; bad=1; }
        ;;
    partoken)
        grep -q '^Output written on partoken-ok\.dvi (1 page, [0-9]* bytes)\.$' "$d/partoken-ok.log" 2>/dev/null ||
            { echo "        partoken-ok.log: no 'Output written on partoken-ok.dvi (1 page, ...)'"; bad=1; }
        if grep -q '^!' "$d/partoken-ok.log" 2>/dev/null; then
            echo "        partoken-ok.log has an error message"; bad=1
        fi
        has_line "$d/partoken-xfail.log" '! Paragraph ended before \macro was complete.' ||
            { echo "        partoken-xfail.log: no '! Paragraph ended before \\macro was complete.'"; bad=1; }
        ;;
    expanded)
        # The run reached \end: upstream's diff sees only START..END.
        has_line "$d/expanded.log" "No pages of output." ||
            { echo "        expanded.log: no 'No pages of output.' (the run did not reach \\end)"; bad=1; }
        ;;
    cnfline)
        grep -q '^Output written on cnfline\.dvi (1 page, [0-9]* bytes)\.$' "$d/cnfline.log" 2>/dev/null ||
            { echo "        cnfline.log: no 'Output written on cnfline.dvi (1 page, ...)'"; bad=1; }
        ;;
    wcfname)
        # Each document writes this one line into \jobname.txt
        # (fn-generate.perl; without -8bit an INITEX writes bytes >= 128 as
        # ^^xx), and its recorder file (--recorder) lists that output and the
        # `\input` of fn±×÷§¶-utf8.tex by their names as given, unencoded.
        want='abc ^^ce^^b1^^ce^^b2^^ce^^b3 ^^d0^^b0^^d0^^b1^^d0^^b2 ^^e3^^81^^82^^e3^^82^^a2^^e2^^80^^bb^^ef^^bf^^a5 ^^e5^^a4^^a9^^e5^^9c^^b0^^e4^^ba^^ba'
        for doc in fn-utf8 fn£¥µÆÇñß-utf8 fnさざ波-utf8 fnΔДदダ打다𝕯🎉-utf8; do
            j=$d/pdftests/$doc-pdf
            if [ ! -f "$j.txt" ] || [ -L "$j.txt" ] || ! printf '%s\n' "$want" | cmp -s - "$j.txt"; then
                echo "        $doc-pdf.txt: not the line pdfTeX writes"; bad=1
            fi
            has_line "$j.fls" "OUTPUT $doc-pdf.txt" ||
                { echo "        $doc-pdf.fls: no 'OUTPUT $doc-pdf.txt'"; bad=1; }
            has_line "$j.fls" "INPUT pdftests/fn±×÷§¶-utf8.tex" ||
                { echo "        $doc-pdf.fls: no 'INPUT pdftests/fn±×÷§¶-utf8.tex'"; bad=1; }
            has_line "$j.log" "No pages of output." ||
                { echo "        $doc-pdf.log: no 'No pages of output.'"; bad=1; }
        done
        ;;
    esac
    # No links but the runner's own: an output linked to an expected file
    # would satisfy upstream's diff.
    links=$(find "$d" -type l ! -path "$d/pdftex" -print)
    if [ -n "$links" ]; then
        echo "        symbolic links in the test directory: $links"; bad=1
    fi
    return $bad
}

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
            # Not silent in CI: an annotation on the run.
            [ "${GITHUB_ACTIONS:-}" != true ] || echo "::warning::pdftex-regression: $t skipped (needs kpsewhich and perl)"
            nskip=$((nskip + 1))
            continue
        fi
        export KpsDir
        KpsDir=$(dirname "$kpsewhich")
    fi
    REGRESSION_EXITS=$d/engine-exits.txt
    export REGRESSION_EXITS
    : >"$REGRESSION_EXITS"
    status=0
    run_test "$d" "$srcdir/$t" || status=$?
    # Upstream's script passed: hold it to the exits and outputs too.
    problems=
    if [ "$status" = 0 ] && [ "$expected" = pass ]; then
        exits=$(tr '\n' ' ' <"$REGRESSION_EXITS" | sed 's/ $//')
        want_exits=$(expected_exits "$name")
        if [ "$exits" != "$want_exits" ]; then
            problems="        the engine's exit statuses were '$exits', TeX Live's pdftex gives '$want_exits'
"
        fi
        problems=$problems$(check_outputs "$name" "$d") || true
    fi
    if [ -n "$problems" ]; then
        echo "FAIL  $t: upstream's script passed, but:"
        printf '%s\n' "$problems"
        nfail=$((nfail + 1))
        fail=1
    elif [ "$status" = 124 ]; then
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
