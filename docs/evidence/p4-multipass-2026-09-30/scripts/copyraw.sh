#!/bin/bash
R=/Users/dqi26/flashtex/.claude/worktrees/agent-a9b0d190d583b912c/docs/evidence/p4-multipass-2026-09-30/raw
S=/Users/dqi26/flashtex/.claude/worktrees/agent-a9b0d190d583b912c/docs/evidence/p4-multipass-2026-09-30/scripts
cd /Users/dqi26/flashtex-wt/d1
cp probe-base.log $R/iprobe-base-biblatex120.log
cp probe-v2.log $R/iprobe-v2-defer-biblatex120.log
cp t-incr.txt $R/tests-incremental.txt
cp t-host3.txt $R/tests-host.txt
cp clippy.txt $R/clippy.txt
cp xparity-v2.txt $R/xparity.txt
cp xparity-v2.jsonl $R/xparity.jsonl
python3 bytecmp.py xparity-v2-work xparity-v2.jsonl > $R/xparity-bytes.txt
python3 bytecmp.py xparity-base2-work xparity-base2.jsonl > $R/xparity-bytes-base-2docs.txt
cp run-base.log $R/checkpoint1-bench-base.jsonl
[ -f xsound-v3.jsonl ] && cp xsound-v3.jsonl $R/xsound.jsonl && cp xsound-v3.txt $R/xsound.txt
[ -d gate ] && mkdir -p $R/gate && cp gate/*.log gate/*.json gate/load.txt $R/gate/ 2>/dev/null
for f in gate/*/result.json; do [ -f "$f" ] && cp "$f" $R/gate/$(basename $(dirname $f)).result.json; done
cp mpbench.py iprobe.py onepass.py gatebench.sh mkcorpus.sh xrun2.sh bytecmp.py t31.py mkshim.sh mkeng.sh summ.py copyraw.sh $S/
ls -la $R | awk '{print $5, $9}'
