#!/bin/bash
# instr2.sh OUTDIR "ENG[:ENV] ..." DOCS ROUNDS: keystroke CPU and instructions, interleaved
cd /private/tmp/mb
O=$1; ENGS=$2; DOCS=$3; R=${4:-2}
for r in $(seq 1 $R); do
  mkdir -p $O/$r
  for d in $DOCS; do
    case $d in
      art4) x=(--edits "0;3") ;;
      art1) x=(--edits "0") ;;
      full-100) x=(--edits "10;50;90") ;;
      beamer-default) x=(--edits "1:19") ;;
      blank) x=(--edits "0:3") ;;
    esac
    for spec in $ENGS; do
      e=${spec%%:*}; env=""; [ "$spec" != "$e" ] && env=${spec#*:}
      name=$(echo "$spec" | tr ':=' '__')
      python3 mb.py "$e" blank balanced --tag warm --idle 0 --edits "" > /dev/null 2>&1
      sed -i '' '$d' out/summary.jsonl
      python3 mb.py "$e" "$d" balanced --tag "in-$name" --idle 3 --keys 16 --env "$env" "${x[@]}" > /dev/null 2>&1
      sed -i '' '$d' out/summary.jsonl
      mkdir -p "$O/$r/in-$name-$d-balanced"
      cp "w/in-$name-$d-balanced/recs.jsonl" "$O/$r/in-$name-$d-balanced/"
    done
  done
done
echo done
