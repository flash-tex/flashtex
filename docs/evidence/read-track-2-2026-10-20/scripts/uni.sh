#!/usr/bin/env bash
# uni.sh TAG OUT [unicode.py args]: b4, 3 keystrokes at the end, conv notes on (no callgrind unless asked)
set -u
. ~/rt/env.sh
export PATH=/nix/store/39iybd55afmpqlgf979ra06wj3q3wapa-valgrind-3.27.1/bin:$PATH
tl=$HOME/texlive/2026
export FLASHTEX_FONT_DIRS=$tl/texmf-dist/fonts/opentype:$tl/texmf-dist/fonts/truetype
T=$1; O=$2; shift 2
rm -rf $O; mkdir -p $O; export FLASHTEX_CACHE_DIR=$O/cache
cd ~/code/flashtex-rt2
~/.local/bin/ftx-run 3600 python3 tools/incr-bench/unicode.py ~/rt/ub/$T/flashtex-host-unicode tools/unicode-m3/bench/b4-fontspec-pages.tex --doc b4 --keys 3 --at end --host-env FLASHTEX_UNICODE_CONV_DEBUG=1 --out $O/r.jsonl "$@" > $O/out.txt 2> $O/err.txt; echo rc=$?
ls -t /tmp/fxu-*-m.err 2>/dev/null | head -1 | xargs -r cp -t $O/
rm -rf $O/cache
