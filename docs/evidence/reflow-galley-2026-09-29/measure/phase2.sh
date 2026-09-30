#!/bin/bash
# Phase 2: hyperref variants, real documents, sampling profiles. Run sequentially.
set -u
G=/private/tmp/claude-501/-Users-kubar-code-flashtex/80e4852c-78aa-4764-b1d3-86bb9edb4d82/scratchpad/galley
cd $G
while ! grep -q '^c gonly10' bench.out; do sleep 10; done
python3 bench2.py e 9
python3 bench2.py a 9
python3 bench2.py b 9
python3 bench2.py c 9
python3 bench_real.py real/texbytopic TeXbyTopic.tex 5
python3 bench_real.py real/memoir memman.tex 5
echo PHASE2-DONE
