#!/bin/bash
# Survey PASSES of #1306's fixtures (read-only git show from the fetched branch).
cd /Users/dqi26/flashtex/.claude/worktrees/agent-a9b0d190d583b912c
R=origin/agent/daniel-muse-lead/multipass-fixtures
for d in $(git ls-tree --name-only $R fixtures/multipass/); do
  echo "$(basename $d): $(git show $R:$d/PASSES | tr '\n' '|' | sed 's/SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 //g; s/-interaction=nonstopmode -halt-on-error //g')"
done
git ls-tree -r --name-only $R fixtures/multipass/ | grep -v 'PASSES$\|README$\|main.tex$'
