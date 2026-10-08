#!/usr/bin/env bash
# memr/t7.sh ENGINE TAG [DOCS]: T7 (6 body phases, typing@50ms, preamble) with FLASHTEX_MEMSTAT and an smaps sampler
export INCR_BENCH_DIR=$HOME/ib-memr FLASHTEX_MEMSTAT=1
export PATH=$HOME/texlive/2026/bin/x86_64-linux:$HOME/.nix-profile/bin:$PATH
E=$1; TAG=$2; DOCS=${3:-plain-1000 full-1000}
O=$HOME/ib-memr/t7-$TAG; rm -rf $O; mkdir -p $O
for d in $DOCS; do
  python3 ~/smapsw.py "ib-memr/t7-work/s0-$d" $O/$d-smaps &
  W=$!
  python3 -u ~/code/flashtex-memr/tools/incr-bench/t7.py --engine $E --docs $d --phases ${PHASES:-letter@start,letter@middle,letter@end,sentence@middle,newline@middle,split@middle,typing@50ms,preamble} --out $O/$d ${T7ARGS:-} > $O/$d.log 2>&1
  wait $W
done
echo T7DONE >> $O/status
