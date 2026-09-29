#!/bin/sh
# Regenerate the TANGLE oracle files of this directory. Needs Knuth's `tangle`
# (MacTeX / TeX Live); it is an oracle only, never part of the build.
#
#   tools/web2rust/tests/changefile/regenerate.sh
#
# sample.tangle.p and sample.tangle.pool are what TANGLE writes for
# `tangle sample.web sample.ch`; sample.tangle.toks is sample.tangle.p in the
# token form of `web2rust --emit-pascal` (tools/pascal_tokens.py).
# tools/web2rust/tests/changefile.rs compares web2rust against them.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cp "$here/sample.web" "$here/sample.ch" "$work/"
(cd "$work" && tangle sample.web sample.ch)
cp "$work/sample.p" "$here/sample.tangle.p"
cp "$work/sample.pool" "$here/sample.tangle.pool"
python3 "$here/../../tools/pascal_tokens.py" "$here/sample.tangle.p" >"$here/sample.tangle.toks"
# The two failing cases must fail in TANGLE too (exit status 1).
for bad in partial nomatch; do
    cp "$here/$bad.ch" "$work/"
    if (cd "$work" && tangle sample.web "$bad.ch" >/dev/null 2>&1); then
        echo "regenerate.sh: TANGLE accepted $bad.ch" >&2
        exit 1
    fi
done
tangle --version | head -1
