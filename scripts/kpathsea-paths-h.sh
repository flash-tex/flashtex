#!/bin/sh
# Regenerate crates/flashtex-engine/kpathsea-config/kpathsea/paths.h from the
# vendored third_party/kpathsea/texmf.cnf, with exactly the pipeline of
# kpathsea's Makefile.am (target `stamp-paths`). paths.h only supplies
# kpathsea's compiled-in fallback search paths.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
src=$root/third_party/kpathsea
out=$root/crates/flashtex-engine/kpathsea-config/kpathsea/paths.h
mkdir -p "$(dirname "$out")"
{
    echo "/* paths.h: Generated from texmf.cnf. */"
    awk -f "$src/bsnl.awk" "$src/texmf.cnf" |
        sed -e 's/%.*//' -e 's/^[ 	]*//' -e 's/[ 	]*$//' |
        awk -f "$src/cnf-to-paths.awk"
} >"$out"
echo "wrote $out"
