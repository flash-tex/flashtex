#!/bin/bash
# Key -> presented A/B: frame-rate boost on (a) vs off (b), interleaved; window in front (no activation).
S=/Users/kubar/code/flashtex/.claude/worktrees/agent-a61b967844fa7b489/.scratch
export FLASHTEX_V3_BENCH_FRONT=1
for r in ${ROUNDS:-1 2}; do
  for n in ${DOCS:-10 120 1000}; do
    TAG=pa$r DOCS=$n KEYS=${KEYS:-40} bash $S/runbench.sh > /dev/null 2>&1
    FLASHTEX_V3_BOOST=0 TAG=pb$r DOCS=$n KEYS=${KEYS:-40} bash $S/runbench.sh > /dev/null 2>&1
  done
done
uptime
python3 - <<'EOF'
import json, glob, statistics as st
S='/Users/kubar/code/flashtex/.claude/worktrees/agent-a61b967844fa7b489/.scratch/bench/'
def pct(v,p):
    v=sorted(v);
    if not v: return None
    k=(len(v)-1)*p/100; f=int(k); c=min(f+1,len(v)-1); return round(v[f]+(v[c]-v[f])*(k-f),1)
for n in [10,120,1000]:
    for tag,name in [('pa','boost'),('pb','no boost')]:
        kc=[];kp=[];cp=[];notp=0;vis=True;fr=[]
        for f in glob.glob(S+f'{tag}[0-9]-plain-{n}.json'):
            d=json.load(open(f)); vis=vis and d.get('windowVisible',False); fr.append(d.get('boostFrameMs'))
            for s in d['samplesDetail']:
                kc.append((s['commitNs']-s['keyNs'])/1e6)
                p=s.get('presentedNs')
                if p==0: notp+=1
                if p: kp.append((p-s['keyNs'])/1e6); cp.append((p-s['commitNs'])/1e6)
        print(f'plain-{n:<5} {name:9} n={len(kc):3} key->commit {pct(kc,50)}/{pct(kc,95)}  commit->presented {pct(cp,50)}/{pct(cp,95)}  key->presented {pct(kp,50)}/{pct(kp,95)}  (presented n={len(kp)}, not shown {notp}, visible {vis}, frame {fr})')
EOF
