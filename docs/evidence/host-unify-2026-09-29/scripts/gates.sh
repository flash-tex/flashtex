#!/bin/bash
# The engine gates on /tmp/hu-gates/eng (see setup.sh); raw output in /tmp/hu-gates/raw.
W=/Users/kubar/code/flashtex/.claude/worktrees/agent-a2411aceaee51c6fc
D=/tmp/hu-gates/eng
F=/tmp/hu-gates/fmt
R=/tmp/hu-gates/raw
J=${J:-6}
mkdir -p $R
cd $W
echo "engine $(cat $D/HEAD); start $(date -u +%FT%TZ) $(uptime)" > $R/environment.txt
for g in ${GATES:-parity parity-dl positions lockstep trip etrip drift}; do
  echo "== $g $(date -u +%T) $(uptime)" >> $R/environment.txt
  case $g in
    parity)
      python3 tools/parity/parity.py --tier fixtures --engine $D/flashtex-initex \
        --engine-env FLASHTEX_FORMATS=$F --engine-env FLASHTEX_POOL=$D/pdftex.pool \
        --pt on --raster none -j $J --out /tmp/hu-gates/parity-off > $R/parity-fixtures.txt 2>&1 ;;
    parity-dl)
      python3 tools/parity/parity.py --tier fixtures --engine $D/flashtex-initex \
        --engine-env FLASHTEX_FORMATS=$F --engine-env FLASHTEX_POOL=$D/pdftex.pool \
        --engine-env FLASHTEX_DISPLAY_LIST=/dev/null \
        --pt on --raster none -j $J --out /tmp/hu-gates/parity-dl > $R/parity-fixtures-display-list.txt 2>&1 ;;
    positions)
      FLASHTEX_POOL=$D/pdftex.pool python3 tools/displaylist/check_positions.py --engine $D/flashtex-initex \
        --formats $F -j $J --json $R/positions-fixtures.json > $R/positions.txt 2>&1 ;;
    lockstep)
      FLASHTEX_POOL=$D/pdftex.pool python3 tools/lockstep/run.py --engine $D/flashtex-initex > $R/lockstep.txt 2>&1 ;;
    trip)
      scripts/flashtex-trip.sh > $R/trip.txt 2>&1; echo "trip exit $?" >> $R/trip.txt ;;
    etrip)
      scripts/flashtex-etrip.sh > $R/etrip.txt 2>&1; echo "etrip exit $?" >> $R/etrip.txt ;;
    drift)
      CARGO_BUILD_JOBS=6 cargo test --release -p web2rust --test drift > $R/drift.txt 2>&1; echo "drift exit $?" >> $R/drift.txt ;;
  esac
done
echo "end $(date -u +%FT%TZ) $(uptime)" >> $R/environment.txt
