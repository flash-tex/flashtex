import sys,re
lines=open('tex.web',encoding='latin-1').read().split('\n')
n=0; start=None
secs={}
for i,l in enumerate(lines):
    if l.startswith('@ ') or l=='@' or l.startswith('@*') or l.startswith('@\t'):
        if n==0 and start is None: pass
        n+=1; secs[n]=i
# limbo before first section
want=[int(x) for x in sys.argv[1:]]
keys=sorted(secs)
for w in want:
    s=secs[w]; e=secs.get(w+1,len(lines))
    print(f'==== §{w} (line {s+1})'); print('\n'.join(lines[s:e]))
