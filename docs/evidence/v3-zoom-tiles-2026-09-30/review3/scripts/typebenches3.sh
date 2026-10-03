#!/bin/bash
# Re-review 3, finding 1: disk writes, footprint and keystroke->tile latency while typing zoomed.
S=/private/tmp/claude-501/-Users-jay3332-Projects-flashtex/211342f1-98df-4cd5-b564-1eec24da3de0/scratchpad
T=/Users/jay3332/Projects/flashtex/.claude/worktrees/agent-a38de97ab0a6ef6aa/target/dl3-positions
E=/Users/jay3332/Projects/flashtex/.claude/worktrees/agent-a38de97ab0a6ef6aa/docs/evidence/v3-zoom-tiles-2026-09-30
M=$T/real-world__beamer-madrid/src/main.tex
cd /Users/jay3332/Projects/flashtex/.claude/worktrees/agent-a38de97ab0a6ef6aa/apps/mac && swift build -c release --product FlashTeXMac > $S/logs/a38-release-build6.log 2>&1
echo "build $?" > $S/logs/a38-typebenches3.log
rm -f $S/logs/a38-typeretries.log
# Whole-drawn page (beamer-madrid p1: paths, forms, PDF fallback), 16 px/pt.
DOCFROM=$M AT="Why a " bash $S/a38-typebench-retry.sh t3-madrid16-300 16 300
KEYS=20 DOCFROM=$M AT="Why a " bash $S/a38-typebench-retry.sh t3-madrid16-2000 16 2000
KEYS=20 DOCFROM=$M AT="Why a " FLASHTEX_V3_WHOLE_DEBOUNCE_MS=0 bash $S/a38-typebench-retry.sh t3-madrid16-2000-nodebounce 16 2000
KEYS=20 DOCFROM=$M AT="Why a " FLASHTEX_V3_TILE_THRESHOLD=1000 bash $S/a38-typebench-retry.sh t3-madrid16-2000-wholepage 16 2000
# Glyph page, 16 px/pt.
DOCFROM=$E/dense.tex AT="with " bash $S/a38-typebench-retry.sh t3-glyph16-300 16 300
echo ALLDONE >> $S/logs/a38-typebenches3.log
