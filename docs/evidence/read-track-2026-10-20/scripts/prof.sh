#!/usr/bin/env bash
# prof.sh TREE TAG: build the Unicode host in TREE, b4 end 3 keys under callgrind, dumps kept
set -u
. ~/rt/env.sh
export PATH=/nix/store/39iybd55afmpqlgf979ra06wj3q3wapa-valgrind-3.27.1/bin:$PATH
tl=$HOME/texlive/2026
export FLASHTEX_FONT_DIRS=$tl/texmf-dist/fonts/opentype:$tl/texmf-dist/fonts/truetype
W=$1; T=$2
(cd $W/crates/flashtex-xetex && ~/.local/bin/ftx-run 3600 cargo build --release --bin flashtex-host-unicode) > ~/rt/build-$T.log 2>&1 || { echo build failed; tail ~/rt/build-$T.log; exit 1; }
O=~/rt/prof-$T; rm -rf $O; mkdir -p $O
export FLASHTEX_CACHE_DIR=$O/cache
cd ~/code/flashtex-readtrack
~/.local/bin/ftx-run 3600 python3 tools/incr-bench/unicode.py $W/crates/flashtex-xetex/target/release/flashtex-host-unicode tools/unicode-m3/bench/b4-fontspec-pages.tex --doc b4 --keys 3 --at end --callgrind $O/cg --callgrind-keep --host-env FLASHTEX_UNICODE_CONV_DEBUG=1 --out $O/r.jsonl > $O/out.txt 2> $O/err.txt; echo rc=$?
ls -t /tmp/fxu-*-m.err | head -1 | xargs cp -t $O/
