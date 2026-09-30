#!/usr/bin/env bash
# gates.sh [GATE...]: the engine gates a lane runs before landing, on a Linux runner with TeX Live
# 2026 (the NixOS PC; first written for lane P4-FINISH). Engine NAME "gates" under INCR_BENCH_DIR.   default: build parity lockstep trip etrip drift positions tests sound-a
#                           sound-c sound-d sound-book gate
#   build      release engine, display-list crate, web2rust; the pdflatex format; the documents
#   parity     P-T1/P-T2 on the parity fixtures (tools/parity, --pt on)
#   lockstep   tools/lockstep (260 cases)
#   trip etrip drift
#   positions  tools/displaylist/check_positions.py (83 fixtures)
#   tests      cargo tests: incremental, host_incremental, display_list_host, intrinsics, lib
#   sound-a    soundness: 50 single-character edits + reverts, fixtures + plain-120 + full-100
#   sound-budget soundness: 20 edits + reverts under a 4 MB undo-log budget (retention always on)
#   sound-c    soundness: 20 structural edits, fixtures + refs-30/120 + full-100
#   sound-d    soundness: 12 interleaved (interrupted) edits
#   sound-book soundness: 8 single-character edits + 4 sentences (+ reverts) on the owner's
#              1,072-page book.tex (copied to $B/src-book/book.tex beforehand)
#   gate       scripts/gate.sh pr
# The checkout is this script's (on NixOS it needs PR #1232's rpath fix for libstdc++).
# Raw output: $R. Every engine run has a time limit (incr_bench.py, soundness.py, timeout(1)).
set -u
S=$(cd "$(dirname "$0")" && pwd)
W=$(cd "$S/../.." && pwd)
B=${INCR_BENCH_DIR:-/tmp/incr-bench}
export INCR_BENCH_DIR=$B
D=$B/gates
F=$B/fmt-gates
R=${R:-$B/raw}
J=${J:-12}
TL=$HOME/texlive/2026/bin/x86_64-linux
export PATH=$TL:$HOME/.nix-profile/bin:$PATH
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-12}
mkdir -p $R
cd $W
echo "engine $(git rev-parse --short HEAD) ($(git log -1 --format=%s | head -c 80)); $(uname -srm); start $(date -u +%FT%TZ) $(uptime)" >> $R/environment.txt
for g in ${@:-build parity lockstep trip etrip drift positions tests sound-a sound-budget sound-c sound-d sound-book gate}; do
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
      python3 $S/mkdocs.py >> $R/build.txt 2>&1 ;;
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
      PYTHONHASHSEED=0 timeout 36000 python3 $S/soundness.py gates -j $J --trials 50 --dir $B/sound-a --out $R/soundness-a.jsonl \
        --extra $B/src-plain-120:plain-120 --extra $B/src-full-100:full-100 > $R/soundness-a.txt 2>&1
      echo "soundness A exit $?" >> $R/soundness-a.txt ;;
    sound-budget)
      # retention (`thin`) under a 4 MB undo-log budget, so that every run drops and merges
      # checkpoints (the default 1 GB is never reached by these documents)
      PYTHONHASHSEED=0 timeout 36000 python3 $S/soundness.py gates -j $J --trials 20 --dir $B/sound-budget \
        --out $R/soundness-budget.jsonl --host-args "--budget 4194304" \
        --extra $B/src-plain-120:plain-120 --extra $B/src-full-100:full-100 > $R/soundness-budget.txt 2>&1
      echo "soundness budget exit $?" >> $R/soundness-budget.txt ;;
    sound-c)
      PYTHONHASHSEED=0 timeout 36000 python3 $S/soundness.py gates -j $J --trials 20 --dir $B/sound-c --out $R/soundness-c.jsonl \
        --kinds sentence,section,label,ref,cite,footnote,unlabel,unsection \
        --extra $B/src-refs-30:refs-30 --extra $B/src-refs-120:refs-120 --extra $B/src-full-100:full-100 > $R/soundness-c.txt 2>&1
      echo "soundness C exit $?" >> $R/soundness-c.txt ;;
    sound-d)
      PYTHONHASHSEED=0 timeout 36000 python3 $S/soundness.py gates -j $J --trials 12 --interleave --dir $B/sound-d --out $R/soundness-d.jsonl \
        --kinds replace,insert,sentence,section,label,ref,unlabel \
        --extra $B/src-refs-30:refs-30 --extra $B/src-refs-120:refs-120 --extra $B/src-full-100:full-100 > $R/soundness-d.txt 2>&1
      echo "soundness D exit $?" >> $R/soundness-d.txt ;;
    sound-book)
      PYTHONHASHSEED=0 timeout 36000 python3 $S/soundness.py gates -j 4 --trials 8 --no-fixtures --dir $B/sound-book --out $R/soundness-book.jsonl \
        --extra $B/src-book:book > $R/soundness-book.txt 2>&1
      PYTHONHASHSEED=0 timeout 36000 python3 $S/soundness.py gates -j 4 --trials 4 --no-fixtures --kinds sentence --dir $B/sound-book-s --out $R/soundness-book-s.jsonl \
        --extra $B/src-book:book >> $R/soundness-book.txt 2>&1
      echo "soundness book exit $?" >> $R/soundness-book.txt ;;
    gate)
      FLASHTEX_GATE_JOBS=$J timeout 10800 scripts/gate.sh pr > $R/gate-pr.txt 2>&1; echo "gate.sh pr exit $?" >> $R/gate-pr.txt ;;
  esac
done
echo "end $(date -u +%FT%TZ) $(uptime)" >> $R/environment.txt
