#!/bin/bash
# The engine gates for lane P5-DIAGNOSTICS, as run on the NixOS PC (Commander, 2026-09-30:
# heavy sweeps off the Mac). Usage: gates.sh [GATE...]; default: all.
#   build     release engine, display-list crate, web2rust; the pdflatex format
#   parity    P-T1/P-T2 on the parity fixtures, side channel off (the product's CLI engine)
#   parity-dl the same with a display list written (FLASHTEX_DISPLAY_LIST=/dev/null)
#   parity-dg the same with the diagnostics side channel's hooks on (FLASHTEX_DIAGNOSTICS=1)
#   lockstep  tools/lockstep (260 cases) with the hooks on
#   trip etrip drift
#   tests     cargo tests: diagnostics (corpus vs pdflatex, soundness), incremental, host_incremental
#   compare   tools/diag-oracle/compare.py (precision vs pdflatex on this host)
# Raw output: $R. The checkout: $W (a local merge of this branch with PR #1232's rpath fix).
set -u
W=${W:-$HOME/code/flashtex-p5diag}
B=${B:-/tmp/p5d-gates}
D=$B/eng
F=$B/fmt
R=$B/raw
J=${J:-12}
# TeX Live 2026 first: ~/.nix-profile has a TeX Live 2025 whose texmf.cnf the engine would read.
TL=$HOME/texlive/2026/bin/x86_64-linux
export PATH=$TL:$HOME/.nix-profile/bin:$PATH
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-12}
mkdir -p $R
cd $W
echo "engine $(git rev-parse --short HEAD) ($(git log -1 --format=%s | head -c 80)); $(uname -srm); start $(date -u +%FT%TZ) $(uptime)" > $R/environment.txt
for g in ${@:-build parity parity-dl parity-dg lockstep trip etrip drift tests compare}; do
  echo "== $g $(date -u +%T) $(uptime)" >> $R/environment.txt
  case $g in
    build)
      cargo build --release -p flashtex-engine -p flashtex-display-list -p web2rust > $R/build.txt 2>&1 || { echo "build failed"; exit 1; }
      rm -rf $D $F; mkdir -p $D $F
      cp target/release/flashtex-initex target/release/flashtex-host $D/
      cp crates/flashtex-engine/pdftex.pool $D/
      ln -sf $D/flashtex-initex $D/pdftex
      (cd $F && SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 FLASHTEX_POOL=$D/pdftex.pool $D/flashtex-initex -ini \
         -jobname=pdflatex -progname=pdflatex -etex -translate-file=cp227.tcx pdflatex.ini </dev/null >/dev/null 2>&1)
      ls -la $F/pdflatex.fmt >> $R/build.txt ;;
    parity)
      python3 tools/parity/parity.py --tier fixtures --engine $D/flashtex-initex \
        --engine-env FLASHTEX_FORMATS=$F --engine-env FLASHTEX_POOL=$D/pdftex.pool \
        --oracle-pdftex $TL/pdftex --texbin $TL --pt on --raster none -j $J --out $B/parity-off > $R/parity-fixtures.txt 2>&1 ;;
    parity-dl)
      python3 tools/parity/parity.py --tier fixtures --engine $D/flashtex-initex \
        --engine-env FLASHTEX_FORMATS=$F --engine-env FLASHTEX_POOL=$D/pdftex.pool \
        --engine-env FLASHTEX_DISPLAY_LIST=/dev/null \
        --oracle-pdftex $TL/pdftex --texbin $TL --pt on --raster none -j $J --out $B/parity-dl > $R/parity-fixtures-display-list.txt 2>&1 ;;
    parity-dg)
      python3 tools/parity/parity.py --tier fixtures --engine $D/flashtex-initex \
        --engine-env FLASHTEX_FORMATS=$F --engine-env FLASHTEX_POOL=$D/pdftex.pool \
        --engine-env FLASHTEX_DISPLAY_LIST=/dev/null --engine-env FLASHTEX_DIAGNOSTICS=1 \
        --oracle-pdftex $TL/pdftex --texbin $TL --pt on --raster none -j $J --out $B/parity-dg > $R/parity-fixtures-diagnostics.txt 2>&1 ;;
    lockstep)
      FLASHTEX_DIAGNOSTICS=1 FLASHTEX_POOL=$D/pdftex.pool python3 tools/lockstep/run.py --engine $D/flashtex-initex > $R/lockstep.txt 2>&1 ;;
    trip)
      scripts/flashtex-trip.sh > $R/trip.txt 2>&1; echo "trip exit $?" >> $R/trip.txt ;;
    etrip)
      scripts/flashtex-etrip.sh > $R/etrip.txt 2>&1; echo "etrip exit $?" >> $R/etrip.txt ;;
    drift)
      cargo test --release -p web2rust --test drift > $R/drift.txt 2>&1; echo "drift exit $?" >> $R/drift.txt ;;
    tests)
      cargo test --release -p flashtex-engine --test diagnostics --test incremental --test host_incremental \
        -- --nocapture > $R/tests.txt 2>&1; echo "tests exit $?" >> $R/tests.txt
      cargo test --release -p flashtex-engine --lib > $R/tests-lib.txt 2>&1; echo "lib tests exit $?" >> $R/tests-lib.txt
      cargo test --release -p flashtex-display-list > $R/tests-display-list.txt 2>&1; echo "display-list tests exit $?" >> $R/tests-display-list.txt ;;
    compare)
      python3 tools/diag-oracle/compare.py --host $D/flashtex-host --formats $F --pool $D/pdftex.pool \
        --json $R/compare-linux.json > $R/compare-linux.txt 2>&1; echo "compare exit $?" >> $R/compare-linux.txt ;;
  esac
done
echo "end $(date -u +%FT%TZ) $(uptime)" >> $R/environment.txt
