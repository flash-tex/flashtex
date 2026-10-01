#!/bin/bash
# xrun.sh ENG MODE OUT [extra xtools args]
E=/Users/dqi26/flashtex-wt/d1/$1; M=$2; O=/Users/dqi26/flashtex-wt/d1/$3; shift 3
W=/Users/dqi26/flashtex/.claude/worktrees/agent-a9b0d190d583b912c
export PATH=/Users/dqi26/flashtex-wt/d1-bin:$PATH SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1
rm -rf $O-work; mkdir -p $O-work
echo "start $(date -u +%T) load $(sysctl -n vm.loadavg)" > $O.txt
python3 $W/tools/external-tools/xtools.py $M --host $E/flashtex-host --formats $E/fmt --pool $E/pdftex.pool \
  --texbin /Users/dqi26/flashtex-wt/d1-bin --env FLASHTEX_TEXLIVE_BIN=/Users/dqi26/flashtex-wt/d1-bin \
  --list ${LIST:-/Users/dqi26/flashtex-wt/d1/corpus.txt} --work $O-work --out $O.jsonl "$@" >> $O.txt 2>&1
echo "exit $? end $(date -u +%T) load $(sysctl -n vm.loadavg)" >> $O.txt
