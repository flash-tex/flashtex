#!/usr/bin/env bash
# measure.sh TAG [classic|unicode|cold|all]: READ-TRACK-2 numbers for ~/rt/ib/TAG and ~/rt/ub/TAG (one host at a time)
set -u
. ~/rt/env.sh
VGB=/nix/store/39iybd55afmpqlgf979ra06wj3q3wapa-valgrind-3.27.1/bin
export PATH=$VGB:$PATH
T=$1; what=${2:-all}
M=~/rt/rt2/m-$T; mkdir -p $M
tl=$HOME/texlive/2026
if [ $what = all ] || [ $what = cold ]; then
  echo "== Classic cold"
  ~/.local/bin/ftx-run 7200 ~/rt/cg.sh ~/rt/ib/$T ~/rt/ib/fmt-$T plain-100 full-100 c1-lipsum c2-ifnextchar | tee $M/cold.txt
fi
if [ $what = all ] || [ $what = unicode ]; then
  echo "== Unicode M3 rows"
  (
  export FLASHTEX_FONT_DIRS=$tl/texmf-dist/fonts/opentype:$tl/texmf-dist/fonts/truetype
  cd ~/code/flashtex-rt2
  for at in end EDITPOINT; do
    for doc in tools/unicode-m3/bench/b*.tex; do
      d=$(basename $doc .tex)
      [ $at = EDITPOINT ] && [ $d != b4-fontspec-pages ] && continue
      rm -rf $M/ucache-$d; export FLASHTEX_CACHE_DIR=$M/ucache-$d
      ~/.local/bin/ftx-run 7200 python3 tools/incr-bench/unicode.py ~/rt/ub/$T/flashtex-host-unicode $doc --doc $d --keys 3 --at $at --callgrind $M/ucg-$at-$d --out $M/u-$at-$d.jsonl > /dev/null 2> $M/u-$at-$d.err; echo "$at $d rc=$?"
    done
  done
  rm -rf $M/ucache-* $M/ucg-*
  python3 - $M <<"PY"
import glob, json, os, sys
for f in sorted(glob.glob(os.path.join(sys.argv[1], "u-*.jsonl"))):
    rows = [json.loads(l) for l in open(f)]
    print(os.path.basename(f)[:-6], " | ".join("%s %s/%s" % (r["tag"], r.get("edited_instructions_m"), r.get("instructions_m")) for r in rows))
PY
  ) | tee $M/unicode.txt
fi
if [ $what = all ] || [ $what = classic ] || [ $what = c12 ]; then
  echo "== Classic sessions (pinned checkpoints)"
  (
  export FLASHTEX_TIMED_S=1000000000 FLASHTEX_SEGMENT_S=off FLASHTEX_PREAMBLE_LINE_S=off FLASHTEX_NO_PREPARE=1 FLASHTEX_HOST_KEEP_WARM_MS=0
  cd ~/rt
  run() { # d at k tag
    ~/.local/bin/ftx-run 7200 python3 rt_end.py ~/rt/ib/$T docs/$1.tex --fmt ~/rt/ib/fmt-$T --keys $3 --at "$2" --callgrind $M/$4.cg > $M/$4.txt 2>$M/$4.err
    echo "$1 at=$2 keys=$3 $(grep -m1 "^summary:\|^totals:" $M/$4.cg)"
  }
  run c1-lipsum end 0 warm
  for k in 0 1 2 3; do run c1-lipsum end $k c1-end-$k; run c2-ifnextchar end $k c2-end-$k; done
  [ $what = c12 ] || for k in 0 3; do run c1-lipsum "{Thr" $k c1-thr-$k; run plain-100 "Section 13" $k p100-$k; run full-100 "Section 13" $k f100-$k; done
  ) | tee $M/classic-$what.txt
fi
echo MEASURE-DONE
