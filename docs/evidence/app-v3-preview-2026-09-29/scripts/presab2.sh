#!/bin/bash
# Probe perturbation check on plain-10: tx probe (t1) vs independent-present probe (t0) vs no probe (np).
S=/Users/kubar/code/flashtex/.claude/worktrees/agent-a61b967844fa7b489/.scratch
export FLASHTEX_V3_BENCH_FRONT=1
for r in 1 2; do
  TAG=t1$r DOCS=10 KEYS=40 bash $S/runbench.sh > /dev/null 2>&1
  FLASHTEX_V3_PRESENT_TX=0 TAG=t0$r DOCS=10 KEYS=40 bash $S/runbench.sh > /dev/null 2>&1
  FLASHTEX_V3_PRESENT=0 TAG=np$r DOCS=10 KEYS=40 bash $S/runbench.sh > /dev/null 2>&1
done
uptime
python3 - <<'EOF'
import json, glob
S='/Users/kubar/code/flashtex/.claude/worktrees/agent-a61b967844fa7b489/.scratch/bench/'
def pct(v,p):
    v=sorted(v)
    if not v: return None
    k=(len(v)-1)*p/100; f=int(k); c=min(f+1,len(v)-1); return round(v[f]+(v[c]-v[f])*(k-f),2)
for tag in ['t1','t0','np']:
    kc=[];cp=[];ca=[];vs=[]
    for f in glob.glob(S+f'{tag}[0-9]-plain-10.json'):
        d=json.load(open(f))
        for s in d['samplesDetail']:
            kc.append((s['commitNs']-s['keyNs'])/1e6)
            if s.get('installNs') and s['commitNs']>=s['installNs']: ca.append((s['commitNs']-s['installNs'])/1e6)
            if s.get('presentedNs'): cp.append((s['presentedNs']-s['commitNs'])/1e6)
            if s.get('vsyncNs'): vs.append((s['vsyncNs']-s['commitNs'])/1e6)
    print(tag, 'n',len(kc),'key->commit',pct(kc,50),pct(kc,95),'install->commit(flush)',pct(ca,50),pct(ca,95),'commit->presented',pct(cp,50),pct(cp,95),'min',min(cp) if cp else None,'commit->linkvsync',pct(vs,50))
EOF
