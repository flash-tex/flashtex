#!/bin/bash
# Time to first tile (from the first queued job) on path pages at 16/20 px/pt and a glyph page.
S=/private/tmp/claude-501/-Users-jay3332-Projects-flashtex/211342f1-98df-4cd5-b564-1eec24da3de0/scratchpad
T=/Users/jay3332/Projects/flashtex/.claude/worktrees/agent-a38de97ab0a6ef6aa/target/dl3-positions
E=/Users/jay3332/Projects/flashtex/.claude/worktrees/agent-a38de97ab0a6ef6aa/docs/evidence/v3-zoom-tiles-2026-09-30
cd /Users/jay3332/Projects/flashtex/.claude/worktrees/agent-a38de97ab0a6ef6aa/apps/mac && swift build -c release --product FlashTeXMac > $S/logs/a38-release-build4.log 2>&1
echo "build $?" > $S/logs/a38-benches4.log
for ppp in 16 20; do
  DOCFROM=$T/real-world__beamer-madrid/src/main.tex bash $S/a38-bench-retry.sh r4-madrid-$ppp $ppp
  DOCFROM=$T/real-world__beamer-visuals/src/main.tex bash $S/a38-bench-retry.sh r4-visuals-$ppp $ppp
  DOCFROM=$E/dense.tex bash $S/a38-bench-retry.sh r4-glyph-$ppp $ppp
done
echo ALLDONE >> $S/logs/a38-benches4.log
