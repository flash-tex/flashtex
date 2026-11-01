#!/usr/bin/env bash
# prof.sh TAG: b4 end 3 keys under callgrind, each page dump kept, conv notes on
set -u
. ~/rt/env.sh
export PATH=/nix/store/39iybd55afmpqlgf979ra06wj3q3wapa-valgrind-3.27.1/bin:$PATH
tl=$HOME/texlive/2026
export FLASHTEX_FONT_DIRS=$tl/texmf-dist/fonts/opentype:$tl/texmf-dist/fonts/truetype
T=$1; O=~/rt/rt2/prof-$T; rm -rf $O; mkdir -p $O
export FLASHTEX_CACHE_DIR=$O/cache
cd ~/code/flashtex-rt2
~/.local/bin/ftx-run 3600 python3 tools/incr-bench/unicode.py ~/rt/ub/$T/flashtex-host-unicode tools/unicode-m3/bench/b4-fontspec-pages.tex --doc b4 --keys 3 --at end --callgrind $O/cg --callgrind-keep --host-env FLASHTEX_UNICODE_CONV_DEBUG=1 --out $O/r.jsonl > $O/out.txt 2> $O/err.txt; echo rc=$?
rm -rf $O/cache
