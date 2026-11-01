#!/usr/bin/env bash
# uni.sh HOSTBIN TAG [extra unicode.py args]: b4, 3 keystrokes before \end{document}, conv notes on
set -u
export PATH=$HOME/texlive/2026/bin/x86_64-linux:$PATH
tl=$HOME/texlive/2026
export FLASHTEX_FONT_DIRS=$tl/texmf-dist/fonts/opentype:$tl/texmf-dist/fonts/truetype FLASHTEX_CACHE_DIR=$HOME/rt/ucache
H=$1; T=$2; shift 2
cd ~/code/flashtex-readtrack
python3 tools/incr-bench/unicode.py $H tools/unicode-m3/bench/b4-fontspec-pages.tex --doc b4 --keys 3 --at end --host-env FLASHTEX_UNICODE_CONV_DEBUG=1 --out ~/rt/u-$T.jsonl "$@" > /dev/null 2> ~/rt/u-$T.err; echo "rc=$?"
python3 -c "
import json,sys
for l in open(sys.argv[1]):
    r=json.loads(l); print(r.get(\"tag\"), {k:r.get(k) for k in (\"edited_instructions_m\",\"instructions_m\",\"edited_page\",\"pages\",\"starts\",\"edited_ms\",\"done_ms\")})
" ~/rt/u-$T.jsonl
