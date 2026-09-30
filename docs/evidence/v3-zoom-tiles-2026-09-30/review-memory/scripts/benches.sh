#!/bin/bash
# The review's measurements: peak memory at 8/16/20 px/pt on a cut page with
# table rules, a beamer path page and a glyph page; 120 Hz scroll at 16 px/pt.
S=/private/tmp/claude-501/-Users-jay3332-Projects-flashtex/211342f1-98df-4cd5-b564-1eec24da3de0/scratchpad
T=/Users/jay3332/Projects/flashtex/.claude/worktrees/agent-a38de97ab0a6ef6aa/target/dl3-positions
true
echo "build $?" > $S/logs/a38-benches.log
for ppp in 8 16 20; do
  DOCFROM=$T/divergence-probes__min-tabular/src/main.tex bash $S/a38-bench-retry.sh table-$ppp $ppp
  DOCFROM=$T/real-world__beamer-madrid/src/main.tex bash $S/a38-bench-retry.sh beamer-$ppp $ppp
  DOCFROM=/Users/jay3332/Projects/flashtex/.claude/worktrees/agent-a38de97ab0a6ef6aa/docs/evidence/v3-zoom-tiles-2026-09-30/dense.tex bash $S/a38-bench-retry.sh glyph-$ppp $ppp
  echo "done $ppp" >> $S/logs/a38-benches.log
done
DOCFROM=$T/divergence-probes__min-tabular/src/main.tex bash $S/a38-bench-retry.sh table-16-b 16
# Scroll on cut pages at 16 px/pt: a longer document with stroked rules, and beamer again.
DOCFROM=/Users/jay3332/Projects/flashtex/.claude/worktrees/agent-a38de97ab0a6ef6aa/docs/evidence/v3-zoom-tiles-2026-09-30/dense-rules.tex bash $S/a38-bench-retry.sh rules-16-a 16
DOCFROM=/Users/jay3332/Projects/flashtex/.claude/worktrees/agent-a38de97ab0a6ef6aa/docs/evidence/v3-zoom-tiles-2026-09-30/dense-rules.tex bash $S/a38-bench-retry.sh rules-16-b 16
DOCFROM=$T/real-world__beamer-madrid/src/main.tex bash $S/a38-bench-retry.sh beamer-16-b 16
echo ALLDONE >> $S/logs/a38-benches.log
