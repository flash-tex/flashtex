#!/bin/bash
# UNICODE-WALL: the edit and reopen rows through unicode.py on the reference Mac
set -uo pipefail
R=/Users/kubar/code/flashtex/.claude/worktrees/agent-a18004afa8e93b8a6
W=/private/tmp/claude-501/-Users-kubar-code-flashtex/440fe00e-c203-4f15-b706-f22dec3a333e/scratchpad/uwall
B=/private/tmp/claude-501/-Users-kubar-code-flashtex/440fe00e-c203-4f15-b706-f22dec3a333e/scratchpad/ubin
TL=/usr/local/texlive/2026/texmf-dist
export FLASHTEX_FONT_DIRS=$TL/fonts/opentype:$TL/fonts/truetype
export FLASHTEX_CACHE_DIR=$W/flashtex-cache
KEYS=${KEYS:-20}
mkdir -p $W/docs $W/lat
cp $R/tools/unicode-m3/bench/*.tex $W/docs/
python3 -c "
import sys; sys.argv=['x']; sys.path.insert(0,'$R/tools/unicode-m3')
import latency; open('$W/docs/b5-long.tex','w').write(latency.LONG)"
# warm the format cache once (not measured)
python3 $R/tools/incr-bench/unicode.py $B/flashtex-host-unicode $W/docs/b1-article.tex --doc b1-article --keys 1 \
  --host-env FLASHTEX_S0_CACHE=$W/s0-warm > /dev/null 2> $W/lat/warm.err
for doc in $W/docs/*.tex; do
  name=$(basename $doc .tex)
  for at in EDITPOINT end; do
    python3 $R/tools/incr-bench/unicode.py $B/flashtex-host-unicode $doc --doc $name \
      --keys $KEYS --at $at --reopen \
      --host-env FLASHTEX_S0_CACHE=$W/s0-$name-$at \
      --out $W/lat/$name-$at.jsonl > /dev/null 2> $W/lat/$name-$at.err \
      || echo "$name $at: unicode.py failed"
    echo "done $name $at"
  done
done
