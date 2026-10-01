#!/bin/bash
S=/private/tmp/claude-501/-Users-jay3332-Projects-flashtex/211342f1-98df-4cd5-b564-1eec24da3de0/scratchpad
T=/Users/jay3332/Projects/flashtex/.claude/worktrees/agent-a38de97ab0a6ef6aa/target/dl3-positions
E=/Users/jay3332/Projects/flashtex/.claude/worktrees/agent-a38de97ab0a6ef6aa/docs/evidence/v3-zoom-tiles-2026-09-30
W=/Users/jay3332/Projects/flashtex/.claude/worktrees/agent-a38de97ab0a6ef6aa
M=$T/real-world__beamer-madrid/src/main.tex
rm -f $S/logs/a38-round6.done
bash $S/a38-sweep.sh focus6 4 'TileParityTests|EngineV3PageTilesTests|EngineV3ZoomTilesTests|EngineV3OpenTests|EngineV3InstanceTests|PreviewParityTests'
cd $W/apps/mac && swift build -c release --product FlashTeXMac > $S/logs/a38-release-build8.log 2>&1
# Paired, back to back (same load window): throttle on / off, keys every 2 s; then 300 ms; then glyphs.
KEYS=20 DOCFROM=$M AT="Why a " bash $S/a38-typebench-retry.sh t6-madrid16-2000 16 2000
KEYS=20 DOCFROM=$M AT="Why a " FLASHTEX_V3_WHOLE_THROTTLE_MS=0 bash $S/a38-typebench-retry.sh t6-madrid16-2000-off 16 2000
DOCFROM=$M AT="Why a " bash $S/a38-typebench-retry.sh t6-madrid16-300 16 300
DOCFROM=$M AT="Why a " FLASHTEX_V3_WHOLE_THROTTLE_MS=0 bash $S/a38-typebench-retry.sh t6-madrid16-300-off 16 300
DOCFROM=$E/dense.tex AT="with " bash $S/a38-typebench-retry.sh t6-glyph16-300 16 300
cd $W && { scripts/gate.sh pr 2>&1; echo "EXIT $?"; } > $S/logs/a38-gate6.log 2>&1
echo ALLDONE > $S/logs/a38-round6.done
