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
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
srcdir=$root/third_party/pdftex/regression/texk/web2c

engine=
if [ "${1:-}" = "--engine" ]; then
    engine=$(cd "$(dirname "$2")" && pwd)/$(basename "$2")
fi
if [ -z "$engine" ]; then
    (cd "$root" && cargo build --release --locked -p flashtex-engine)
    engine=$root/target/release/flashtex-initex
fi

work=${REGRESSION_WORK:-$(mktemp -d)}
rm -rf "$work"
mkdir -p "$work/bin"
ln -s "$engine" "$work/bin/pdftex"

# The tests in pdftex.am's order.
tests="pdftexdir/wprob.test pdftexdir/pdftex.test pdftexdir/pdfimage.test
pdftexdir/expanded.test pdftexdir/tests/cnfline.test
pdftexdir/tests/partoken.test pdftexdir/wcfname.test"

# Expected failures: `test|owner|reason`.
xfail="pdftexdir/pdfimage.test|P3|\\pdfximage (JPEG, PDF and PNG inclusion) is lane P3-FONTS's"

export srcdir BinDir="$work/bin" ExeExt=
export FLASHTEX_POOL="$root/crates/flashtex-engine/pdftex.pool"
export FLASHTEX_RESOLVER=kpathsea-self
export SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1

kpsewhich=$(command -v kpsewhich || true)
fail=0
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
            continue
        fi
        export KpsDir
        KpsDir=$(dirname "$kpsewhich")
    fi
    status=0
    (cd "$d" && sh "$srcdir/$t") >"$d/test.out" 2>&1 || status=$?
    if [ "$status" = 77 ]; then
        echo "SKIP  $t (the test skipped itself)"
    elif [ "$status" = 0 ] && [ "$expected" = pass ]; then
        echo "PASS  $t"
    elif [ "$status" = 0 ]; then
        echo "XPASS $t: listed as an expected failure ($owner: $reason); remove it from the list"
    elif [ "$expected" = fail ]; then
        echo "XFAIL $t (exit $status): $owner -- $reason"
    else
        echo "FAIL  $t (exit $status); the last lines of its output:"
        tail -15 "$d/test.out" | sed 's/^/        /'
        fail=1
    fi
done
echo "work dir: $work"
exit $fail
