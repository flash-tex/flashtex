#!/usr/bin/env python3
"""Sampling-profile a pdflatex run and attribute samples to pdfTeX routines.

prof.py OUTNAME WORKDIR ENGINE ARG...   (ENGINE = pdflatex|latex, using the
re-signed arm64 copy in ./tl so that sample(1) can attach; the TeX Live binary is
hardened-runtime and stripped. Function names come from LC_FUNCTION_STARTS via
`dyld_info -function_starts`; unnamed static functions are shown as sym+off.)
Writes prof/OUTNAME.sample.txt and prints inclusive/self tables.
"""
import bisect, os, re, subprocess, sys, time, collections

HERE = os.path.dirname(os.path.abspath(__file__))
out, wd, engine, args = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4:]
BIN = os.path.join(HERE, "tl/bin/universal-darwin", engine)
starts = []
for line in open(os.path.join(HERE, "prof/fstarts.txt")):
    m = re.match(r"\s+0x([0-9A-F]+)\s+(\S+)", line)
    if m:
        starts.append((int(m.group(1), 16), m.group(2)))
starts.sort()
addrs = [s[0] for s in starts]


def fname(off):
    i = bisect.bisect_right(addrs, off) - 1
    return starts[i][1] if i >= 0 else "?"


sfile = os.path.join(HERE, "prof", out + ".sample.txt")
if "--parse-only" not in sys.argv:
    p = subprocess.Popen([BIN] + args, cwd=wd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    s = subprocess.run(["sample", str(p.pid), "120", "1", "-mayDie", "-file", sfile],
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    p.wait()

txt = open(sfile).read()
load = None
m = re.search(r"pdftex \(\d+\).*", txt)
# Call graph section
cg = txt.split("Call graph:")[1].split("Total number in stack")[0]
lines = cg.splitlines()
nodes = []  # (depth, count, func)
for ln in lines:
    m = re.match(r"^([\s+!:|]*)(\d+)\s+(.*)$", ln)
    if not m:
        continue
    depth = len(m.group(1))
    count = int(m.group(2))
    rest = m.group(3)
    f = None
    ma = re.search(r"\(in pdftex\).*load address 0x([0-9a-f]+) \+ 0x([0-9a-f]+)", rest)
    if ma:
        f = fname(0x100000000 + int(ma.group(2), 16))
    else:
        mb = re.search(r"^(\S+)\s+\(in ([^)]+)\)", rest)
        if mb:
            f = mb.group(1) + "@" + mb.group(2)
            if mb.group(2) == "pdftex":
                f = "_" + mb.group(1)
        else:
            f = rest.split()[0]
    nodes.append((depth, count, f))

incl = collections.Counter()
selfc = collections.Counter()
stack = []  # (depth, func, count, childsum)
total = 0


def pop_to(depth):
    while stack and stack[-1][0] >= depth:
        d, f, c, cs = stack.pop()
        selfc[f] += c - cs[0]


for depth, count, f in nodes:
    pop_to(depth)
    if stack:
        stack[-1][3][0] += count
    else:
        total += count
    if f not in [s[1] for s in stack]:
        incl[f] += count
    stack.append((depth, f, count, [0]))
pop_to(-1)
T = sum(c for (d, c, f) in nodes if d == min(n[0] for n in nodes))
print("samples", T)
KEYS = ["_maincontrol", "_zlinebreak", "_ztrybreak", "_zhpack", "_mlisttohlist", "_zvpackage",
        "_buildpage", "_zfireup", "_zshipout", "_zpdfshipout", "_hlistout", "_pdfhlistout",
        "_expand", "_macrocall", "_getnext", "_getxtoken", "_writezip", "_zidlookup",
        "_zfinalcleanup", "_closefilesandterminate", "_finishpdffile", "_zpdfbeginpage", "_zpdfendpage"]
print("%-28s %8s %6s" % ("inclusive", "samples", "%"))
for k in KEYS:
    print("%-28s %8d %6.1f" % (k, incl[k], 100.0 * incl[k] / T))
print("\n%-40s %8s %6s" % ("top self", "samples", "%"))
for f, c in selfc.most_common(40):
    print("%-40s %8d %6.1f" % (f, c, 100.0 * c / T))
