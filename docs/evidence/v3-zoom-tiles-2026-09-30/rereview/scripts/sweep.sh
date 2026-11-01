#!/bin/bash
# usage: sweep.sh <label> <scales|all> [filter]
S=/private/tmp/claude-501/-Users-jay3332-Projects-flashtex/211342f1-98df-4cd5-b564-1eec24da3de0/scratchpad
cd /Users/jay3332/Projects/flashtex/.claude/worktrees/agent-a38de97ab0a6ef6aa/apps/mac || exit 1
export FLASHTEX_V3_TILE_SWEEP=1
[ "$2" != "all" ] && export FLASHTEX_V3_TILE_SCALES="$2"
export FLASHTEX_V3_TILE_SWEEP_OUT=$S/a38-sweep-$1.json
FILTER="${3:-TileParityTests}"
uptime > $S/logs/a38-sweep-$1.log
swift test -c release -Xswiftc -enable-testing --filter "$FILTER" >> $S/logs/a38-sweep-$1.log 2>&1
echo EXIT $? >> $S/logs/a38-sweep-$1.log
uptime >> $S/logs/a38-sweep-$1.log
