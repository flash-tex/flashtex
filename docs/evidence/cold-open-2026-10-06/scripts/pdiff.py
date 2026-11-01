import subprocess, sys, re, collections
def load(f):
    out = subprocess.run(['nix','shell','nixpkgs#perf','-c','perf','report','-i',f,'--no-children','--sort','symbol','--stdio','-F','period,sym'],capture_output=True,text=True).stdout
    d=collections.Counter()
    for l in out.splitlines():
        m=re.match(r'\s*(\d+)\s+\[\.\]\s+(.*?)(\s+-\s+-)?\s*$', l)
        if m: d[m.group(2).strip()[:110]]+=int(m.group(1))
    return d
a=load(sys.argv[1]); b=load(sys.argv[2])
ta,tb=sum(a.values()),sum(b.values())
print(f'total {ta/1e9:.1f} G -> {tb/1e9:.1f} G (sampled periods)')
top=sorted(set(a)|set(b), key=lambda k: -max(a[k],b[k]))[:25]
for k in top: print(f'{a[k]/1e9:7.2f} {b[k]/1e9:7.2f} {(b[k]-a[k])/1e9:+6.2f}  {k}')
print('--- biggest increases')
for k in sorted(set(a)|set(b), key=lambda k: -(b[k]-a[k]))[:15]: print(f'{a[k]/1e9:7.2f} {b[k]/1e9:7.2f} {(b[k]-a[k])/1e9:+6.2f}  {k}')
print('--- biggest decreases')
for k in sorted(set(a)|set(b), key=lambda k: (b[k]-a[k]))[:8]: print(f'{a[k]/1e9:7.2f} {b[k]/1e9:7.2f} {(b[k]-a[k])/1e9:+6.2f}  {k}')
