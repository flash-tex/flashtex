#!/usr/bin/env python3
"""memr.py ENGINE_DIR SRC_DIR MAIN OUT [--step FILE:AT:KIND:KEYS ...] [--host-args ...]
One production host (FLASHTEX_MEMSTAT=1) on a copy of SRC_DIR: cold open, then each step
(dl3-keys --edit FILE --at AT --kind KIND --keys KEYS), then idle 8 s and an unchanged open
(the steady record). Samples RSS every 50 ms, /proc smaps breakdown at the peak and at the end.
Writes OUT.json (per-phase peak RSS, DONE.mem per phase, smaps) and OUT.jsonl (dl3-keys lines)."""
import argparse, json, os, re, shutil, subprocess, sys, threading, time
sys.path.insert(0, os.path.expanduser("~/code/flashtex-memr/tools/incr-bench"))
import t7

a = argparse.ArgumentParser()
a.add_argument("eng"); a.add_argument("src"); a.add_argument("main"); a.add_argument("out")
a.add_argument("--step", action="append", default=[])
a.add_argument("--host-args", default="")
a.add_argument("--gap-ms", default="300")
a.add_argument("--interval-ms", default="")
a = a.parse_args()
work = f"{a.out}-work"
shutil.rmtree(work, ignore_errors=True)
shutil.copytree(a.src, work, symlinks=True)
os.makedirs(f"{work}/out", exist_ok=True)
sock = f"/tmp/memr-{os.getpid()}.sock"
os.environ["FLASHTEX_MEMSTAT"] = "1"

def smaps(pid):
    g, cur = {}, None
    for line in open(f"/proc/{pid}/smaps"):
        m = re.match(r"^([0-9a-f]+)-([0-9a-f]+) (\S+) \S+ \S+ \S+\s*(.*)$", line)
        if m:
            size = int(m.group(2), 16) - int(m.group(1), 16)
            name = m.group(4).strip()
            if not name:
                mb = size >> 20
                name = ("[anon >=256M]" if mb >= 256 else "[anon 64M arena]" if 60 <= mb <= 64 and size % (1<<20)==0
                        else "[anon 1-60M]" if mb >= 1 else "[anon <1M]")
            elif name.startswith("/"):
                name = "file:" + os.path.basename(name)
            cur = g.setdefault(name, dict(n=0, size=0, rss=0, pd=0, pc=0, anon=0, swap=0))
            cur["n"] += 1; cur["size"] += size
            continue
        k, _, v = line.partition(":")
        if cur is None or not v.strip().endswith("kB"):
            continue
        val = int(v.split()[0]) * 1024
        key = {"Rss": "rss", "Private_Dirty": "pd", "Private_Clean": "pc", "Anonymous": "anon", "Swap": "swap"}.get(k)
        if key: cur[key] += val
    return dict(sorted(g.items(), key=lambda kv: -kv[1]["rss"]))

def rss(pid):
    for line in open(f"/proc/{pid}/status"):
        if line.startswith("VmRSS:"):
            return int(line.split()[1]) * 1024
    return 0

h = t7.Host(a.eng, sock, f"{a.out}-s0", f"{a.out}.host-stderr", a.host_args.split())
pid = h.p.pid
state = dict(phase="start", peak={}, bd={}, stop=False, series=[])
def sampler():
    t0 = time.time(); last_bd = {}
    while not state["stop"]:
        try:
            r = rss(pid)
        except OSError:
            break
        ph = state["phase"]
        state["series"].append((round(time.time() - t0, 2), ph, r))
        if r > state["peak"].get(ph, 0):
            state["peak"][ph] = r
            if r > last_bd.get(ph, 0) + (16 << 20):
                try:
                    state["bd"][ph] = dict(rss=r, groups=smaps(pid)); last_bd[ph] = r
                except OSError:
                    pass
        time.sleep(0.05)
threading.Thread(target=sampler, daemon=True).start()

def dl3(args, phase):
    state["phase"] = phase
    cmd = [f"{t7.S}/to.sh", "3600", f"{a.eng}/dl3-keys", "--socket", sock, "--root", work, "--main", a.main,
           "--output-dir", f"{work}/out"] + args
    t = time.time()
    p = subprocess.run(cmd, capture_output=True, text=True)
    with open(f"{a.out}.jsonl", "a") as f:
        for l in p.stdout.splitlines():
            f.write(json.dumps(dict(phase=phase, line=l)) + "\n")
    mems = []
    for l in p.stdout.splitlines():
        if l.startswith("{"):
            try:
                r = json.loads(l)
            except ValueError:
                continue
            m = (r.get("host") or {}).get("mem")
            if m: mems.append(m)
    print(f"{phase}: rc {p.returncode} {time.time()-t:.1f}s peak {state['peak'].get(phase,0)>>20} MB", flush=True)
    return p.returncode, mems

res = dict(phases=[])
try:
    rc, m = dl3(["--keys", "0", "--at", "0.5"], "open")
    res["phases"].append(dict(name="open", rc=rc, mem=m[-1:] if m else []))
    for s in a.step:
        f, at, kind, keys = s.split(":")
        extra = ["--edit", f] if f else []
        if a.interval_ms and kind == "typing":
            args = extra + ["--kind", "letter", "--at", at, "--keys", keys, "--interval-ms", a.interval_ms]
        else:
            args = extra + ["--kind", kind, "--at", at, "--keys", keys, "--gap-ms", a.gap_ms]
        rc, m = dl3(args, s)
        res["phases"].append(dict(name=s, rc=rc, mem=[m[0], m[-1]] if m else []))
    state["phase"] = "idle"
    for k in ("FLASHTEX_DUMP_LOGS", "FLASHTEX_DUMP_PAGES"):
        if os.environ.get(k):
            open(os.environ[k] + ".go", "w").close()
    time.sleep(8)
    rc, m = dl3(["--keys", "0", "--at", "0.5"], "steady")
    res["phases"].append(dict(name="steady", rc=rc, mem=m[-1:] if m else []))
    time.sleep(1)
    res["steady_rss"] = rss(pid)
    res["steady_smaps"] = smaps(pid)
    res["status"] = open(f"/proc/{pid}/status").read()
finally:
    state["stop"] = True
    h.stop()
res["peak"] = state["peak"]; res["peak_smaps"] = state["bd"]
json.dump(res, open(f"{a.out}.json", "w"), indent=1)
open(f"{a.out}.series", "w").write("\n".join(f"{t} {p} {r}" for t, p, r in state["series"]))
print("PEAK", max(state["peak"].values()) >> 20, "MB", "STEADY", res.get("steady_rss", 0) >> 20, "MB")
