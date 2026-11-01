#!/usr/bin/env bash
# cgdiff.sh A.cg B.cg [inc|ex] [N]: per-function Ir of B minus A (M)
VG=/nix/store/39iybd55afmpqlgf979ra06wj3q3wapa-valgrind-3.27.1/bin
m=${3:-ex}; n=${4:-40}; f=no; [ $m = inc ] && f=yes
$VG/callgrind_annotate --inclusive=$f --threshold=100 "$1" 2>/dev/null > /tmp/rt2-a.txt
$VG/callgrind_annotate --inclusive=$f --threshold=100 "$2" 2>/dev/null > /tmp/rt2-b.txt
python3 - $n <<"PY"
import re, sys
def load(f):
    d={}
    for l in open(f):
        m=re.match(r"\s*([\d,]+) \([^)]*\)\s+\S*:(.*?)( \[.*)?$",l)
        if m: d[m.group(2).strip()]=d.get(m.group(2).strip(),0)+int(m.group(1).replace(",",""))
    return d
a=load("/tmp/rt2-a.txt"); b=load("/tmp/rt2-b.txt")
rows=sorted(((b.get(k,0)-a.get(k,0),k) for k in set(a)|set(b)),key=lambda r:-abs(r[0]))
for d,k in rows[:int(sys.argv[1])]: print(f"{d/1e6:9.2f} {k[:140]}")
PY
