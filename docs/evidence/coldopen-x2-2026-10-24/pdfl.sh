#!/bin/bash
rm -rf /tmp/fth-co/pdfl
cp -R /private/tmp/claude-501/-Users-kubar-code-flashtex/440fe00e-c203-4f15-b706-f22dec3a333e/scratchpad/clive/docs/infdesc-x2 /tmp/fth-co/pdfl
cd /tmp/fth-co/pdfl
for i in 1 2 3 4; do
  s=$(python3 -c 'import time;print(time.time())')
  SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 nice -n 5 pdflatex -interaction=batchmode infdesc-x2.tex >/dev/null 2>&1
  e=$(python3 -c 'import time;print(time.time())')
  echo "pass $i $(python3 -c "print(round($e-$s,1))") s; reruns $(grep -c 'Rerun' infdesc-x2.log); $(grep -o 'Output written.*' infdesc-x2.log)"
done
