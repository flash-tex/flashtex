#!/bin/bash
# ab.sh TAG "ENGINES" "MODES" "DOCS" [ROUNDS]: interleaved A/B runs, each engine warmed
# first (an open of blank, discarded: the engine's font-map and ls-R caches exist).
T=$1; ENG=$2; MODES=$3; DOCS=$4; R=${5:-1}
for r in $(seq 1 $R); do
  for d in $DOCS; do
    for e in $ENG; do
      python3 /private/tmp/mb/mb.py "$e" blank balanced --tag warm --idle 0 --edits "" > /dev/null 2>&1
      bash /private/tmp/mb/runall.sh "$e" "$T-$e" "$MODES" "$d"
    done
  done
done
python3 /private/tmp/mb/tab.py "$T-"
