#!/bin/bash
# runall.sh ENGINE TAG [MODES] [DOCS]: one mb.py run per doc and mode, sequentially.
E=$1; T=$2
MODES=${3:-low-memory balanced high-performance}
DOCS=${4:-blank art1 art4 hw1 beamer-default full-100}
cd /private/tmp/mb
for m in $MODES; do
  for d in $DOCS; do
    case $d in
      blank) x=(--edits "0:3") ;;
      art1) x=(--edits "0") ;;
      art4) x=(--edits "0;3") ;;
      hw1) x=(--main HW1.tex --edits "0:37") ;;
      beamer-default) x=(--edits "1:19") ;;
      full-100) x=(--edits "10;50;90") ;;
      infdesc) x=(--main infdesc.tex --keys 6 --edits "40::book/logical-structure/propositional-logic.tex;300::book/number-theory/divisibility.tex") ;;
    esac
    python3 mb.py "$E" "$d" "$m" --tag "$T" --idle 8 "${x[@]}" 2>&1 | grep -v '^{' | head -3
  done
done
