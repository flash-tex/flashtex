import json,sys
for l in open(sys.argv[1]):
    t,_,j=l.partition(' ')
    if j.startswith('{'):
        r=json.loads(j); print(t, r['mode'], r['restart_pages'], r['rerun_pages'], r['pass_modes'], r['pass_s'], [x[:int(sys.argv[2]) if len(sys.argv)>2 else 200] for x in r['l5']], r['wall'])
    else: print(l.strip()[:300])
