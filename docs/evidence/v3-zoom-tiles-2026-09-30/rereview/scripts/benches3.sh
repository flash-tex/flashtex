#!/bin/bash
# Re-review of #1287 @4308a9365: peak footprint and time to first tile at 8/16/20 px/pt on
# path pages (beamer-madrid: paths and forms; beamer-visuals: forms, PDF fallbacks), a
# table page (stroked rules: clipped) and a glyph page; 120 Hz scroll throughout.
S=/private/tmp/claude-501/-Users-jay3332-Projects-flashtex/211342f1-98df-4cd5-b564-1eec24da3de0/scratchpad
T=/Users/jay3332/Projects/flashtex/.claude/worktrees/agent-a38de97ab0a6ef6aa/target/dl3-positions
E=/Users/jay3332/Projects/flashtex/.claude/worktrees/agent-a38de97ab0a6ef6aa/docs/evidence/v3-zoom-tiles-2026-09-30
cd /Users/jay3332/Projects/flashtex/.claude/worktrees/agent-a38de97ab0a6ef6aa/apps/mac && swift build -c release --product FlashTeXMac > $S/logs/a38-release-build3.log 2>&1
echo "build $?" > $S/logs/a38-benches3.log
rm -f $S/logs/a38-retries.log
for ppp in 8 16 20; do
  DOCFROM=$T/real-world__beamer-madrid/src/main.tex bash $S/a38-bench-retry.sh r3-madrid-$ppp $ppp
  DOCFROM=$T/real-world__beamer-visuals/src/main.tex bash $S/a38-bench-retry.sh r3-visuals-$ppp $ppp
  DOCFROM=$T/divergence-probes__min-tabular/src/main.tex bash $S/a38-bench-retry.sh r3-table-$ppp $ppp
  DOCFROM=$E/dense.tex bash $S/a38-bench-retry.sh r3-glyph-$ppp $ppp
  echo "done $ppp" >> $S/logs/a38-benches3.log
done
DOCFROM=$T/real-world__beamer-madrid/src/main.tex bash $S/a38-bench-retry.sh r3-madrid-16-b 16
DOCFROM=$T/real-world__beamer-visuals/src/main.tex bash $S/a38-bench-retry.sh r3-visuals-20-b 20
echo ALLDONE >> $S/logs/a38-benches3.log
