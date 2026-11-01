"""A freshly 'unpacked' 1,000-file project: plain-120's main.tex plus 1,000 small
files, folder and every file carrying one Safari-style quarantine event."""
import os, shutil, subprocess, time, uuid
W = '/Users/kubar/code/flashtex/.claude/worktrees/agent-a46f56f16e2460dff/.scratch/bench'
dst = os.path.join(W, 'arch-1000')
shutil.rmtree(dst, ignore_errors=True)
os.makedirs(os.path.join(dst, 'parts'))
shutil.copy(os.path.join(W, 'plain-120', 'main.tex'), os.path.join(dst, 'main.tex'))
for i in range(1000):
    with open(os.path.join(dst, 'parts', 'p%04d.tex' % i), 'w') as f:
        f.write('%% part %d\n' % i)
value = '0083;%x;Safari;%s' % (int(time.time()), str(uuid.uuid4()).upper())
paths = [dst] + [os.path.join(r, n) for r, ds, fs in os.walk(dst) for n in ds + fs]
for chunk in range(0, len(paths), 200):
    subprocess.run(['xattr', '-w', 'com.apple.quarantine', value] + paths[chunk:chunk + 200], check=True)
print(len(paths), 'items quarantined with', value)
