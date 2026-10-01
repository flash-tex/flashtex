#!/bin/bash
# Gate bench: waits for 1-min load < 4 before each run; latexmk + new engine (NEW), then the #1296 engine (base).
NEW=${NEW:-v3}
R=/Users/dqi26/flashtex-wt/d1/gate
mkdir -p $R
cd /Users/dqi26/flashtex-wt/d1
waitload() {
  while :; do
    l=$(sysctl -n vm.loadavg | awk '{print $2}')
    if awk -v l="$l" 'BEGIN{exit !(l < 4)}'; then break; fi
    sleep 15
  done
  echo "$(date -u +%T) load $(sysctl -n vm.loadavg)" >> $R/load.txt
}
for spec in "biblatex 120" "book 300"; do
  set -- $spec
  waitload
  python3 mpbench.py /Users/dqi26/flashtex-wt/d1/$NEW $R/$1-$2-$NEW --style $1 --pages $2 --reps 3 > $R/$1-$2-$NEW.log 2>&1
  waitload
  python3 mpbench.py /Users/dqi26/flashtex-wt/d1/base $R/$1-$2-base --style $1 --pages $2 --reps 3 --skip-lmk > $R/$1-$2-base.log 2>&1
  waitload
  python3 onepass.py $R/$1-$2-$NEW/lmk 3 > $R/$1-$2-pdflatex-onepass.json 2>&1
done
waitload
python3 iprobe.py /Users/dqi26/flashtex-wt/d1/$NEW $R/iprobe-$NEW --defer > $R/iprobe-$NEW.log 2>&1
echo "done $(date -u +%T)" >> $R/load.txt
