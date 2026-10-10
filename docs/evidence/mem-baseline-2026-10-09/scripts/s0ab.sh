#!/bin/bash
# s0ab.sh TAG "ENGINES" "MODES" "DOCS": per engine and doc, a first run that writes S0 (discarded),
# then the measured run that opens it (as the app reopens a document).
T=$1; ENG=$2; MODES=$3; DOCS=$4
cd /private/tmp/mb
for d in $DOCS; do
  for e in $ENG; do
    for m in $MODES; do
      case $d in
        blank) x=(--edits "0:3") ;;
        art1) x=(--edits "0") ;;
        art4) x=(--edits "0;3") ;;
        hw1) x=(--main HW1.tex --edits "0:37") ;;
        beamer-default) x=(--edits "1:19") ;;
        full-100) x=(--edits "10;50;90") ;;
        infdesc) x=(--main infdesc.tex --keys 6 --edits "40::book/logical-structure/propositional-logic.tex;300::book/number-theory/divisibility.tex") ;;
      esac
      rm -rf "/private/tmp/mb/s0/$T-$e-$d-$m" "/private/tmp/mb/w/$T-$e-$d-$m"
      python3 mb.py "$e" "$d" "$m" --tag "$T-$e" --idle 3 --edits "" --s0 "/private/tmp/mb/s0/$T-$e-$d-$m" > /dev/null 2>&1
      sed -i '' '$d' out/summary.jsonl
      python3 mb.py "$e" "$d" "$m" --tag "$T-$e" --idle 8 "${x[@]}" --s0 "/private/tmp/mb/s0/$T-$e-$d-$m" 2>&1 | grep -v '^{' | head -3
      python3 first.py "$T-$e-$d-$m" | cut -c1-120
    done
  done
done
python3 tab.py "$T-"
