#!/usr/bin/env bash
# umeasure.sh TAG: READ-TRACK-3's Unicode rows for ~/rt/ub/TAG: b1-b5 at EDITPOINT and at the end, 3 keystrokes, callgrind
set -u
. ~/rt/env.sh
export PATH=/nix/store/39iybd55afmpqlgf979ra06wj3q3wapa-valgrind-3.27.1/bin:$PATH
T=$1; M=~/rt/rt3/m-$T; mkdir -p $M
tl=$HOME/texlive/2026
export FLASHTEX_FONT_DIRS=$tl/texmf-dist/fonts/opentype:$tl/texmf-dist/fonts/truetype
cd ~/code/flashtex-rt2
for at in end EDITPOINT; do
  for doc in tools/unicode-m3/bench/b*.tex ~/rt/rt2/b5-long.tex; do
    d=$(basename $doc .tex)
    rm -rf $M/ucache-$d; export FLASHTEX_CACHE_DIR=$M/ucache-$d
    ~/.local/bin/ftx-run 7200 python3 tools/incr-bench/unicode.py ~/rt/ub/$T/flashtex-host-unicode $doc --doc $d --keys 3 --at $at --callgrind $M/ucg-$at-$d --out $M/u-$at-$d.jsonl > /dev/null 2> $M/u-$at-$d.err; echo "$at $d rc=$?"
    rm -rf $M/ucache-$d $M/ucg-$at-$d
  done
done
python3 - $M <<"PY"
import glob, json, os, sys
for f in sorted(glob.glob(os.path.join(sys.argv[1], "u-*.jsonl"))):
    rows = [json.loads(l) for l in open(f) if '"tag"' in l]
    print(os.path.basename(f)[:-6], " | ".join("%s %s/%s" % (r["tag"], r.get("edited_instructions_m"), r.get("instructions_m")) for r in rows))
PY
echo UMEASURE-DONE
