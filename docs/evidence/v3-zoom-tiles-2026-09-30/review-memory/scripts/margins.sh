#!/bin/bash
# Probe clip margins for cut path pages: tile-paths sweep per margin.
S=/private/tmp/claude-501/-Users-jay3332-Projects-flashtex/211342f1-98df-4cd5-b564-1eec24da3de0/scratchpad
cd /Users/jay3332/Projects/flashtex/.claude/worktrees/agent-a38de97ab0a6ef6aa/apps/mac || exit 1
OUT=$S/logs/a38-margins.log
uptime > $OUT
swift build -c release -Xswiftc -enable-testing --build-tests >> $S/logs/a38-margins-build.log 2>&1 || { echo BUILD-FAILED >> $OUT; echo EXIT >> $OUT; exit 1; }
for m in 0 4 16 64 256; do
  echo "== margin $m" >> $OUT
  FLASHTEX_V3_CUT_MARGIN=$m swift test -c release -Xswiftc -enable-testing --skip-build \
    --filter 'TileParityTests/testTilesAreExactWindowsOfTheCheckedInPages' 2>&1 | grep -E "tile sweep|Executed 1 test" >> $OUT
done
echo EXIT >> $OUT
