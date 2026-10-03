#!/bin/bash
S=/private/tmp/claude-501/-Users-jay3332-Projects-flashtex/211342f1-98df-4cd5-b564-1eec24da3de0/scratchpad
M=/Users/jay3332/Projects/flashtex/.claude/worktrees/agent-a38de97ab0a6ef6aa/target/dl3-positions/real-world__beamer-madrid/src/main.tex
rm -f $S/logs/a38-round6b.done
until [ -f $S/logs/a38-round6.done ]; do sleep 10; done
KEYS=20 DOCFROM=$M AT="Why a " bash $S/a38-typebench-retry.sh t6-madrid16-2000 16 2000
KEYS=20 DOCFROM=$M AT="Why a " FLASHTEX_V3_WHOLE_THROTTLE_MS=0 bash $S/a38-typebench-retry.sh t6-madrid16-2000-off 16 2000
echo ALLDONE > $S/logs/a38-round6b.done
