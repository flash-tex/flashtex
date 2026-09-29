import sys,re
lines=open('tex.web',encoding='latin-1').read().split('\n')
n=0; sec=[0]*len(lines)
for i,l in enumerate(lines):
    if l.startswith('@ ') or l=='@' or l.startswith('@*') or l.startswith('@\t'): n+=1
    sec[i]=n
pat=re.compile(sys.argv[1])
for i,l in enumerate(lines):
    if pat.search(l): print(f'§{sec[i]} L{i+1}: {l.strip()[:150]}')
