#!/bin/bash
# Phase 3: re-run phase-1 matrix (first pass was taken under load avg ~10-15),
# then sampling profiles on the re-signed arm64 pdftex copy.
set -u
G=/private/tmp/claude-501/-Users-kubar-code-flashtex/80e4852c-78aa-4764-b1d3-86bb9edb4d82/scratchpad/galley
cd $G
while ! grep -q 'PHASE2-DONE' phase2.out; do sleep 10; done
python3 bench.py a 7
python3 bench.py b 7
python3 bench.py c 7
cd $G
for b in a b c; do
  python3 prof.py plain-$b-full prof pdflatex -interaction=batchmode -jobname=pf-$b "\def\BODY{../body-$b-0.inc}\def\MODE{plain}\input{../mainh}" > prof/plain-$b-full.txt
  python3 prof.py plain-$b-light prof pdflatex -interaction=batchmode -jobname=pl-$b "\def\HYLIGHT{}\def\BODY{../body-$b-0.inc}\def\MODE{plain}\input{../mainh}" > prof/plain-$b-light.txt
done
python3 prof.py real-tbt real/texbytopic pdflatex -interaction=batchmode TeXbyTopic.tex > prof/real-tbt.txt
python3 prof.py real-memman real/memoir pdflatex -interaction=batchmode memman.tex > prof/real-memman.txt
echo PHASE3-DONE
