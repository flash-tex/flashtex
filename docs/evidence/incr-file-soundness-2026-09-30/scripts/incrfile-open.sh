#!/usr/bin/env bash
# INCR-FILE-SOUNDNESS: open 2501.07356v3 once with tools; args: TAG [ENV=V ...]
B=${B:-/tmp/ifs-dev}
DOC=${DOC:-/home/kubar/.cache/flashtex-parity/src/arxiv/2501.07356v3:NodalMoments.tex}
T=$1; shift
cd ~/code/flashtex-incrfile || exit 1
export PATH=$HOME/texlive/2026/bin/x86_64-linux:$HOME/.nix-profile/bin:$PATH
export LD_LIBRARY_PATH=$HOME/p5x-libxcrypt-legacy/lib
O=/tmp/ifs-run-$T
rm -f $O.trace
envs=(--env FLASHTEX_FILE_TRACE=$O.trace)
for e in "$@"; do envs+=(--env "$e"); done
timeout 1200 python3 tools/external-tools/debug_open.py --host $B/eng/flashtex-host --formats $B/fmt --pool $B/eng/pdftex.pool \
  --doc $DOC --out $O "${envs[@]}" > $O.txt 2>&1
f=$(ls $O/cand/*.toc 2>/dev/null | head -1)
if [ -n "$f" ] && head -c 1 $f | od -An -c | grep -q "\\\\0"; then echo "$T: BAD toc"; else echo "$T: ok"; fi
