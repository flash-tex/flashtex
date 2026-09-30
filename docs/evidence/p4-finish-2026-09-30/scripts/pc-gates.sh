#!/bin/bash
# The gates of lane P4-FINISH on the NixOS PC (heavy sweeps off the Mac). Usage:
#   pc-gates.sh [GATE...]   default: build parity lockstep trip etrip drift positions tests sound-a
#                           sound-c sound-d gate
#   build      release engine, display-list crate, web2rust; the pdflatex format; the documents
#   parity     P-T1/P-T2 on the parity fixtures (tools/parity, --pt on)
#   lockstep   tools/lockstep (260 cases)
#   trip etrip drift
#   positions  tools/displaylist/check_positions.py (83 fixtures)
#   tests      cargo tests: incremental, host_incremental, display_list_host, intrinsics, lib
#   sound-a    soundness: 50 single-character edits + reverts, fixtures + plain-120 + full-100
#   sound-c    soundness: 20 structural edits, fixtures + refs-30/120 + full-100
#   sound-d    soundness: 12 interleaved (interrupted) edits
#   gate       scripts/gate.sh pr
# The checkout $W is this branch merged locally with PR #1232's rpath fix (NixOS's libstdc++).
# Raw output: $R. Every engine run has a time limit (incr_bench.py, soundness.py, timeout(1)).
set -u
W=${W:-$HOME/code/flashtex-p4finish}
B=/tmp/p4f
D=$B/pc
F=$B/fmt-pc
R=$B/raw
J=${J:-12}
TL=$HOME/texlive/2026/bin/x86_64-linux
export PATH=$TL:$HOME/.nix-profile/bin:$PATH
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-12}
S=$W/docs/evidence/p4-finish-2026-09-30/scripts
mkdir -p $R
cd $W
echo "engine $(git rev-parse --short HEAD) ($(git log -1 --format=%s | head -c 80)); $(uname -srm); start $(date -u +%FT%TZ) $(uptime)" >> $R/environment.txt
for g in ${@:-build parity lockstep trip etrip drift positions tests sound-a sound-c sound-d gate}; do
  echo "== $g $(date -u +%T) $(uptime)" >> $R/environment.txt
  case $g in
    build)
      cargo build --release -p flashtex-engine -p flashtex-display-list -p web2rust > $R/build.txt 2>&1 || { echo "build failed"; exit 1; }
      rm -rf $D $F; mkdir -p $D $F
      cp target/release/flashtex-initex target/release/flashtex-host target/release/dl3-keys target/release/dl3-client $D/
      cp crates/flashtex-engine/pdftex.pool $D/
      ln -sf $D/flashtex-initex $D/pdftex
      (cd $F && SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 FLASHTEX_POOL=$D/pdftex.pool timeout 300 $D/flashtex-initex -ini \
         -jobname=pdflatex -progname=pdflatex -etex -translate-file=cp227.tcx pdflatex.ini </dev/null >/dev/null 2>&1)
      ls -la $F/pdflatex.fmt >> $R/build.txt
      python3 - <<PY >> $R/build.txt 2>&1
import os, sys
sys.path.insert(0, "$S")
import gen
for n in (10, 100, 120, 300, 1000):
    for full in (False, True):
        doc = f"{'full' if full else 'plain'}-{n}"
        d = f"$B/src-{doc}"
        os.makedirs(d, exist_ok=True)
        open(f"{d}/{doc}.tex", "w").write(gen.doc(n, full))
print("documents written")
PY
      python3 $S/genrefs.py $B >> $R/build.txt 2>&1 ;;
    parity)
      timeout 7200 python3 tools/parity/parity.py --tier fixtures --engine $D/flashtex-initex \
        --engine-env FLASHTEX_FORMATS=$F --engine-env FLASHTEX_POOL=$D/pdftex.pool \
        --oracle-pdftex $TL/pdftex --texbin $TL --pt on --raster none -j $J --out $B/parity > $R/parity-fixtures.txt 2>&1 ;;
    lockstep)
      FLASHTEX_POOL=$D/pdftex.pool timeout 7200 python3 tools/lockstep/run.py --engine $D/flashtex-initex > $R/lockstep.txt 2>&1 ;;
    trip)
      timeout 3600 scripts/flashtex-trip.sh > $R/trip.txt 2>&1; echo "trip exit $?" >> $R/trip.txt ;;
    etrip)
      timeout 3600 scripts/flashtex-etrip.sh > $R/etrip.txt 2>&1; echo "etrip exit $?" >> $R/etrip.txt ;;
    drift)
      timeout 3600 cargo test --release -p web2rust --test drift > $R/drift.txt 2>&1; echo "drift exit $?" >> $R/drift.txt ;;
    positions)
      FLASHTEX_POOL=$D/pdftex.pool timeout 7200 python3 tools/displaylist/check_positions.py \
        --engine $D/flashtex-initex --formats $F --pool $D/pdftex.pool --oracle $TL/pdftex --dump $W/target/release/dl3-dump -j $J > $R/positions.txt 2>&1; echo "positions exit $?" >> $R/positions.txt ;;
    tests)
      timeout 7200 cargo test --release -p flashtex-engine --test incremental --test host_incremental \
        --test display_list_host --test intrinsics > $R/tests.txt 2>&1; echo "tests exit $?" >> $R/tests.txt
      timeout 3600 cargo test --release -p flashtex-engine --lib > $R/tests-lib.txt 2>&1; echo "lib tests exit $?" >> $R/tests-lib.txt
      timeout 3600 cargo test --release -p flashtex-display-list > $R/tests-display-list.txt 2>&1; echo "display-list tests exit $?" >> $R/tests-display-list.txt ;;
    sound-a)
      PYTHONHASHSEED=0 timeout 36000 python3 $S/soundness.py pc -j $J --trials 50 --dir $B/sound-a --out $R/soundness-a.jsonl \
        --extra $B/src-plain-120:plain-120 --extra $B/src-full-100:full-100 > $R/soundness-a.txt 2>&1
      echo "soundness A exit $?" >> $R/soundness-a.txt ;;
    sound-c)
      PYTHONHASHSEED=0 timeout 36000 python3 $S/soundness.py pc -j $J --trials 20 --dir $B/sound-c --out $R/soundness-c.jsonl \
        --kinds sentence,section,label,ref,cite,footnote,unlabel,unsection \
        --extra $B/src-refs-30:refs-30 --extra $B/src-refs-120:refs-120 --extra $B/src-full-100:full-100 > $R/soundness-c.txt 2>&1
      echo "soundness C exit $?" >> $R/soundness-c.txt ;;
    sound-d)
      PYTHONHASHSEED=0 timeout 36000 python3 $S/soundness.py pc -j $J --trials 12 --interleave --dir $B/sound-d --out $R/soundness-d.jsonl \
        --kinds replace,insert,sentence,section,label,ref,unlabel \
        --extra $B/src-refs-30:refs-30 --extra $B/src-refs-120:refs-120 --extra $B/src-full-100:full-100 > $R/soundness-d.txt 2>&1
      echo "soundness D exit $?" >> $R/soundness-d.txt ;;
    gate)
      FLASHTEX_GATE_JOBS=$J timeout 10800 scripts/gate.sh pr > $R/gate-pr.txt 2>&1; echo "gate.sh pr exit $?" >> $R/gate-pr.txt ;;
  esac
done
echo "end $(date -u +%FT%TZ) $(uptime)" >> $R/environment.txt
