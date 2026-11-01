#!/bin/bash
# Lane P5-EXTERNAL-TOOLS on the NixOS PC. Usage: p5x-gates.sh [GATE...]
#   build     release engine + host + display-list + web2rust; pdflatex.fmt
#   unit      cargo tests: host unit tests, display-list crate, host_incremental
#   parity    P-T1/P-T2 on the parity fixtures
#   lockstep trip etrip drift
#   xparity   tools/external-tools parity on the corpus list ($L)
#   xsound    tools/external-tools soundness on the corpus list ($LS)
#   xbench    tools/external-tools bench (10/120 pages)
#   gatepr    scripts/gate.sh pr
set -u
W=${W:-$HOME/code/flashtex-p5tools}
B=${B:-/tmp/p5x-gates}
D=$B/eng
F=$B/fmt
R=$B/raw
J=${J:-8}
TL=$HOME/texlive/2026/bin/x86_64-linux
export PATH=$TL:$HOME/.nix-profile/bin:$PATH
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-10}
# biber (a PAR-packed binary) needs libcrypt.so.1, which NixOS lacks: libxcrypt-legacy
XL=$HOME/p5x-libxcrypt-legacy/lib
mkdir -p $R
cd $W
echo "engine $(git rev-parse --short HEAD) ($(git log -1 --format=%s | head -c 80)); $(uname -srm); start $(date -u +%FT%TZ) $(uptime)" >> $R/environment.txt
for g in ${@:-build unit parity lockstep trip etrip drift}; do
  echo "== $g $(date -u +%T) $(uptime)" >> $R/environment.txt
  case $g in
    build)
      cargo build --release -p flashtex-engine -p flashtex-display-list -p web2rust > $R/build.txt 2>&1 || { echo "build failed"; tail -30 $R/build.txt; exit 1; }
      rm -rf $D $F; mkdir -p $D $F
      cp target/release/flashtex-initex target/release/flashtex-host $D/
      cp crates/flashtex-engine/pdftex.pool $D/
      ln -sf $D/flashtex-initex $D/pdftex
      (cd $F && SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 FLASHTEX_POOL=$D/pdftex.pool $D/flashtex-initex -ini \
         -jobname=pdflatex -progname=pdflatex -etex -translate-file=cp227.tcx pdflatex.ini </dev/null >/dev/null 2>&1)
      ls -la $F/pdflatex.fmt >> $R/build.txt ;;
    unit)
      cargo test --release -p flashtex-engine --lib host:: > $R/tests-host-unit.txt 2>&1; echo "exit $?" >> $R/tests-host-unit.txt
      cargo test --release -p flashtex-display-list > $R/tests-display-list.txt 2>&1; echo "exit $?" >> $R/tests-display-list.txt
      cargo test --release -p flashtex-engine --test host_incremental --test display_list_host --test host_tools --test incremental -- --nocapture > $R/tests-host.txt 2>&1; echo "exit $?" >> $R/tests-host.txt ;;
    parity)
      python3 tools/parity/parity.py --tier fixtures --engine $D/flashtex-initex \
        --engine-env FLASHTEX_FORMATS=$F --engine-env FLASHTEX_POOL=$D/pdftex.pool \
        --oracle-pdftex $TL/pdftex --texbin $TL --pt on --raster none -j $J --out $B/parity > $R/parity-fixtures.txt 2>&1 ;;
    lockstep)
      FLASHTEX_POOL=$D/pdftex.pool python3 tools/lockstep/run.py --engine $D/flashtex-initex > $R/lockstep.txt 2>&1 ;;
    trip)
      scripts/flashtex-trip.sh > $R/trip.txt 2>&1; echo "trip exit $?" >> $R/trip.txt ;;
    etrip)
      scripts/flashtex-etrip.sh > $R/etrip.txt 2>&1; echo "etrip exit $?" >> $R/etrip.txt ;;
    drift)
      cargo test --release -p web2rust --test drift > $R/drift.txt 2>&1; echo "drift exit $?" >> $R/drift.txt ;;
    xparity)
      LD_LIBRARY_PATH=$XL python3 tools/external-tools/xtools.py parity --host $D/flashtex-host --formats $F \
        --pool $D/pdftex.pool --texbin $TL --list ${L:-$HOME/p5x-corpus.txt} --work $B/xwork -j ${XJ:-6} \
        --out $R/xparity.jsonl > $R/xparity.txt 2>&1; echo "exit $?" >> $R/xparity.txt ;;
    xsound)
      LD_LIBRARY_PATH=$XL python3 tools/external-tools/xtools.py sound --host $D/flashtex-host --formats $F \
        --pool $D/pdftex.pool --texbin $TL --list ${LS:-$HOME/p5x-sound.txt} --work $B/xwork -j ${XJ:-6} \
        --trials ${TRIALS:-2} --out $R/xsound.jsonl > $R/xsound.txt 2>&1; echo "exit $?" >> $R/xsound.txt ;;
    xbench)
      LD_LIBRARY_PATH=$XL python3 tools/external-tools/xtools.py bench --host $D/flashtex-host --formats $F \
        --pool $D/pdftex.pool --texbin $TL --work $B/xwork --pages ${PAGES:-10,120} --reps ${REPS:-5} \
        --out $R/xbench.jsonl > $R/xbench.txt 2>&1; echo "exit $?" >> $R/xbench.txt ;;
    gatepr)
      scripts/gate.sh pr > $R/gate-pr.txt 2>&1; echo "gate exit $?" >> $R/gate-pr.txt ;;
  esac
done
echo "end $(date -u +%FT%TZ) $(uptime)" >> $R/environment.txt
