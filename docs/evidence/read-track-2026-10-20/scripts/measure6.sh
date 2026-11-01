#!/usr/bin/env bash
# measure5.sh: the final branch (rt13): Classic cold compiles, deterministic sessions, Unicode M3 rows
set -u
. ~/rt/env.sh
export PATH=/nix/store/39iybd55afmpqlgf979ra06wj3q3wapa-valgrind-3.27.1/bin:$PATH
M=~/rt/final6; rm -rf $M; mkdir -p $M
echo "== Classic cold (Ir, pdf md5)"
for e in base rt13; do ~/.local/bin/ftx-run 7200 ~/rt/cg.sh ~/rt/ib/$e ~/rt/ib/fmt-$e plain-100 full-100 c1-lipsum c2-ifnextchar; done
echo "== Classic sessions (deterministic checkpoints)"
(
export FLASHTEX_TIMED_S=1000000000 FLASHTEX_SEGMENT_S=off FLASHTEX_PREAMBLE_LINE_S=off FLASHTEX_NO_PREPARE=1 FLASHTEX_HOST_KEEP_WARM_MS=0
cd ~/rt
run() { # e d at k tag
  ~/.local/bin/ftx-run 7200 python3 rt_end.py ~/rt/ib/$1 docs/$2.tex --fmt ~/rt/ib/fmt-$1 --keys $4 --at "$3" --callgrind $M/$5.cg > $M/$5.txt 2>$M/$5.err
  echo "$1 $2 at=$3 keys=$4 $(grep -m1 "^summary:\|^totals:" $M/$5.cg)"
}
run rt13 c1-lipsum end 0 warm
for k in 0 1 2 3; do run rt13 c1-lipsum end $k rt13-c1-end-$k; run rt13 c2-ifnextchar end $k rt13-c2-end-$k; done
for k in 0 3; do run rt13 c1-lipsum "{Thr" $k rt13-c1-thr-$k; run rt13 plain-100 "Section 13" $k rt13-p100-$k; run rt13 full-100 "Section 13" $k rt13-f100-$k; done
)
echo MEASURE6-DONE
