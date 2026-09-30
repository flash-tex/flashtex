#!/usr/bin/env python3
"""External tools through the engine host, against latexmk (lane
P5-EXTERNAL-TOOLS; DESIGN.md §4.4-§4.5, §5.3-§5.5).

`flashtex-host` runs bibtex, biber and makeindex from the user's TeX Live
when latexmk would, and compiles again with what they made (protocol 3.2,
`"external_tools": "auto"`; crates/flashtex-engine/src/host/external.rs).
This measures it. latexmk and pdflatex are the oracle only.

    xtools.py parity --host BIN --formats DIR --pool FILE --doc DIR:MAIN ... [--corpus-arxiv N]
    xtools.py sound  --host BIN --formats DIR --pool FILE --doc DIR:MAIN[:KIND] ...
    xtools.py bench  --host BIN --formats DIR --pool FILE --pages 10,120 --reps 5

* `parity`: for each document, a copy of its sources is compiled by
  `latexmk -pdf` (the oracle), and another copy by a fresh host with
  `external_tools: auto`, until the host says the tools are `settled`; then
  the host exports (`"export": true`: pdflatex's compressed PDF from the
  resident state's files). Passes when the two PDFs are P-T2-identical
  (tools/parity/tiers.compare_pt2) and every `.bbl` and `.ind` the oracle
  made is byte-identical to the host's.
* `sound`: incremental soundness. One host keeps the document open
  (`incremental`); each edit (a new `\\cite` of a `.bib` key not cited yet,
  an edited `.bib` field, a new `\\index` entry) goes in a `COMPILE`
  (`edits`), and after `settled` every page the client holds, the `.bbl`,
  `.ind` and `.aux` files and the exported PDF must equal a from-scratch
  sequence on a copy of the same sources: a fresh host (pages, files) and
  `latexmk -pdf` (P-T2, files).
* `bench`: generated 10- and 120-page documents with a 400-entry `.bib`:
  the time of each tool run and from a `\\cite` edit's `COMPILE` to
  `settled` (citations resolved on every page).

Every run has a timeout. Nothing kills processes by name.
"""
import argparse
import json
import os
import random
import re
import shutil
import socket
import struct
import subprocess
import sys
import tempfile
import time

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, os.path.join(REPO, "tools", "parity"))

K_HELLO, K_COMPILE, K_CANCEL, K_BYE = 0x01, 0x02, 0x03, 0x04
NAMES = {0x41: "hello", 0x42: "started", 0x43: "font", 0x44: "image", 0x45: "page", 0x46: "form",
         0x47: "sources", 0x48: "diagnostic", 0x49: "done", 0x4A: "error", 0x4B: "pages", 0x4C: "tool"}
JSON_KINDS = {"hello", "started", "image", "diagnostic", "done", "error", "pages", "tool"}
GENERATED = (".aux", ".bbl", ".blg", ".bcf", ".run.xml", ".idx", ".ind", ".ilg", ".log", ".pdf", ".out",
             ".toc", ".lof", ".lot", ".fls", ".fdb_latexmk", ".synctex.gz", ".nav", ".snm", ".vrb")
ENV_PIN = {"SOURCE_DATE_EPOCH": "0", "FORCE_SOURCE_DATE": "1"}


def now():
    return time.monotonic()


class Host:
    """A `flashtex-host --socket` process and one 3.2 connection."""

    def __init__(self, bin_, formats, pool, work, env_extra=None, timeout=900):
        self.sock_path = os.path.join(work, "host.sock")
        env = dict(os.environ)
        env.update(ENV_PIN)
        env.update({"FLASHTEX_FORMATS": formats, "FLASHTEX_POOL": pool, "FLASHTEX_PIN_CLOCK": "0"})
        env.update(env_extra or {})
        self.log = open(os.path.join(work, "host.out"), "w")
        self.p = subprocess.Popen([bin_, "--socket", self.sock_path, "--no-warm"], stdin=subprocess.DEVNULL,
                                  stdout=subprocess.PIPE, stderr=self.log, env=env, text=True)
        t0 = now()
        self.startup = []
        while True:
            line = self.p.stdout.readline()
            if not line:
                raise RuntimeError("the host exited before listening")
            self.startup.append(line.rstrip())
            if "listening on" in line:
                break
            if now() - t0 > 120:
                raise RuntimeError("the host did not start")
        self.s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.s.connect(self.sock_path)
        self.s.settimeout(timeout)
        self.buf = b""
        self.send(K_HELLO, {"protocol": "display-list-v3", "version": [3, 2], "client": "xtools"})
        k, self.hello = self.recv()
        assert k == "hello", (k, self.hello)
        self.pages = {}  # index -> content hash (hex), as an incremental client holds them

    def send(self, k, obj):
        body = json.dumps(obj).encode()
        self.s.sendall(struct.pack("<I", len(body) + 1) + bytes([k]) + body)

    def _read(self, n):
        while len(self.buf) < n:
            d = self.s.recv(1 << 20)
            if not d:
                raise EOFError("host closed")
            self.buf += d
        out, self.buf = self.buf[:n], self.buf[n:]
        return out

    def recv(self):
        (n,) = struct.unpack("<I", self._read(4))
        rest = self._read(n)
        k = NAMES.get(rest[0], f"kind{rest[0]:#x}")
        body = rest[1:]
        if k in JSON_KINDS:
            return k, json.loads(body.decode())
        return k, body

    def cycle(self, req, deadline=900):
        """Send `req`, then read until its cycle is settled (`TOOL`
        `settled` with its id). Returns the events of interest."""
        self.send(K_COMPILE, req)
        t0 = now()
        ev = {"dones": [], "tools": [], "diagnostics": [], "t_done": None, "t_settled": None, "errors": []}
        while True:
            if now() - t0 > deadline:
                raise TimeoutError("no settled")
            k, b = self.recv()
            if k == "page":
                idx = struct.unpack_from("<I", b, 0)[0]
                self.pages[idx] = b[88:120].hex()
            elif k == "pages":
                if b.get("complete"):
                    for i in [i for i in self.pages if i >= b["count"]]:
                        del self.pages[i]
            elif k == "done":
                ev["dones"].append(b)
                if ev["t_done"] is None:
                    ev["t_done"] = now() - t0
                if not req.get("incremental"):
                    pass
            elif k == "tool":
                b["t"] = now() - t0
                ev["tools"].append(b)
                if b.get("event") == "settled" and b.get("id") == req["id"]:
                    ev["t_settled"] = now() - t0
                    return ev
            elif k == "diagnostic":
                ev["diagnostics"].append(b)
            elif k == "error":
                ev["errors"].append(b)
                if b.get("id") == req["id"]:
                    return ev

    def export(self, req, deadline=900):
        self.send(K_COMPILE, dict(req, export=True))
        t0 = now()
        while True:
            if now() - t0 > deadline:
                raise TimeoutError("export")
            k, b = self.recv()
            if k == "done" and b.get("id") == req["id"]:
                return b
            if k == "error" and b.get("id") == req["id"]:
                return b

    def close(self):
        try:
            self.send(K_BYE, {})
        except OSError:
            pass
        try:
            self.s.close()
        except OSError:
            pass
        try:
            self.p.terminate()
            self.p.wait(timeout=20)
        except Exception:
            self.p.kill()
        self.log.close()


def copy_sources(src, dst):
    """The document's sources, without what a compile makes (a `.bbl` stays:
    an arXiv source ships one as its source)."""
    if os.path.exists(dst):
        shutil.rmtree(dst)

    def ignore(d, names):
        return [n for n in names if n.endswith(GENERATED) and not n.endswith(".bbl")
                or n in (".parity-copied",)]
    shutil.copytree(src, dst, ignore=ignore, symlinks=False)


def latexmk(texbin, d, main, timeout=900):
    env = dict(os.environ)
    env.update(ENV_PIN)
    env["PATH"] = texbin + os.pathsep + env.get("PATH", "")
    t0 = now()
    try:
        p = subprocess.run([os.path.join(texbin, "latexmk"), "-pdf", "-interaction=nonstopmode", main], cwd=d,
                           stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, env=env,
                           timeout=timeout)
        out = p.stdout.decode("utf-8", "replace")
        rc = p.returncode
    except subprocess.TimeoutExpired:
        return {"ok": False, "why": "latexmk timeout", "seconds": timeout}
    runs = re.findall(r"^Latexmk: applying rule '([^']+)'", out, re.M) or re.findall(r"Run number \d+ of rule '([^']+)'", out)
    pdf = os.path.join(d, os.path.splitext(main)[0] + ".pdf")
    return {"ok": rc == 0 and os.path.isfile(pdf), "rc": rc, "rules": runs, "seconds": round(now() - t0, 3),
            "pdf": pdf if os.path.isfile(pdf) else None, "tail": out[-1500:] if rc else ""}


def made_files(d, base_exts=(".bbl", ".ind")):
    out = {}
    for dp, _, fs in os.walk(d):
        for f in fs:
            if f.endswith(base_exts):
                p = os.path.join(dp, f)
                out[os.path.relpath(p, d)] = open(p, "rb").read()
    return out


def pt2(ref_pdf, cand_pdf, work):
    import tiers
    return tiers.compare_pt2(ref_pdf, cand_pdf, work)


def host_run(a, d, main, work, extra=None):
    """A fresh host compiles `d` until settled, then exports."""
    h = Host(a.host, a.formats, a.pool, work, env_extra=a.env)
    try:
        req = {"id": 1, "root": d, "main": main, "output_dir": d, "external_tools": "auto", "incremental": True}
        req.update(extra or {})
        t0 = now()
        ev = h.cycle(req)
        settle = now() - t0
        ex = h.export({"id": 2, "root": d, "main": main, "output_dir": d})
        return h, ev, ex, settle
    except Exception:
        h.close()
        raise


def summary_tools(ev):
    return [{k: t.get(k) for k in ("event", "tool", "file", "status", "ms", "changed", "reason")}
            for t in ev["tools"] if t.get("event") != "settled"]


def parity_one(a, name, src, main):
    work = os.path.join(a.work, "parity", name)
    shutil.rmtree(work, ignore_errors=True)
    os.makedirs(work)
    od, cd = os.path.join(work, "oracle"), os.path.join(work, "cand")
    copy_sources(src, od)
    copy_sources(src, cd)
    rec = {"doc": name, "main": main}
    o = latexmk(a.texbin, od, main)
    rec["oracle"] = {k: o.get(k) for k in ("ok", "rc", "rules", "seconds", "why", "tail")}
    if not o["ok"]:
        rec["result"] = "excluded: latexmk failed"
        return rec
    try:
        h, ev, ex, settle = host_run(a, cd, main, work)
    except Exception as e:  # noqa: BLE001
        rec["result"] = f"FAIL: host: {e}"
        return rec
    h.close()
    rec["host"] = {"settle_s": round(settle, 3), "compiles": len(ev["dones"]),
                   "modes": [d.get("mode") for d in ev["dones"]],
                   "statuses": [d.get("status") for d in ev["dones"]],
                   "tools": summary_tools(ev), "export": ex.get("status")}
    tool_rules = sorted({f"{t['tool']} {t['file']}" for t in ev["tools"] if t.get("event") == "run"})
    rec["host"]["rules"] = tool_rules
    if not ex.get("pdf"):
        rec["result"] = "FAIL: no export PDF"
        return rec
    r = pt2(o["pdf"], ex["pdf"], os.path.join(work, "pt2"))
    rec["pt2"] = r.get("ok")
    if not r.get("ok"):
        rec["pt2_detail"] = {k: r.get(k) for k in ("why", "pages", "first", "fonts_equal")}
    fo, fc = made_files(od), made_files(cd)
    rec["files"] = {"oracle": sorted(fo), "same": sorted(k for k in fo if fc.get(k) == fo[k]),
                    "differ": sorted(k for k in fo if k in fc and fc[k] != fo[k]),
                    "missing": sorted(k for k in fo if k not in fc)}
    ok = r.get("ok") and not rec["files"]["differ"] and not rec["files"]["missing"]
    rec["result"] = "PASS" if ok else "FAIL"
    return rec


# ---------------------------------------------------------------------------
# soundness

def bib_keys(d):
    keys = []
    for dp, _, fs in os.walk(d):
        for f in fs:
            if f.endswith(".bib"):
                t = open(os.path.join(dp, f), encoding="utf-8", errors="replace").read()
                keys += re.findall(r"@\s*(?!string|comment|preamble)\w+\s*\{\s*([^,\s]+)\s*,", t, re.I)
    return keys


def cited(d):
    c = set()
    for dp, _, fs in os.walk(d):
        for f in fs:
            if f.endswith(".aux"):
                for m in re.findall(r"\\citation\{([^}]*)\}", open(os.path.join(dp, f), errors="replace").read()):
                    c.update(x.strip() for x in m.split(","))
            if f.endswith(".bcf"):
                c.update(re.findall(r"<bcf:citekey[^>]*>([^<]+)</bcf:citekey>",
                                    open(os.path.join(dp, f), errors="replace").read()))
    return c


def tex_files(d):
    out = []
    for dp, _, fs in os.walk(d):
        for f in fs:
            if f.endswith(".tex"):
                out.append(os.path.relpath(os.path.join(dp, f), d))
    return sorted(out)


def edit_cite(d, main, rng):
    """Insert `\\cite{KEY}` (a key not cited yet) after a sentence of the
    main file's body: (path, offset, delete, insert)."""
    keys = [k for k in bib_keys(d) if k not in cited(d)]
    if not keys:
        return None
    key = rng.choice(keys)
    t = open(os.path.join(d, main), "rb").read()
    b = t.find(b"\\begin{document}")
    e = t.rfind(b"\\end{document}")
    if b < 0 or e < 0:
        return None
    cands = [m.end() for m in re.finditer(rb"[a-z]{3}\. ", t[b:e])]
    # keep out of verbatim-like and comment lines: a plain prose position
    good = []
    for c in cands:
        at = b + c
        ls = t.rfind(b"\n", 0, at) + 1
        line = t[ls:t.find(b"\n", at)]
        if b"%" in line or b"\\verb" in line or b"&" in line or b"$" in t[ls:at]:
            continue
        good.append(at)
    if not good:
        return None
    at = rng.choice(good[: max(1, len(good) // 2)])
    return {"path": main, "offset": at, "delete": 0, "insert": f"See \\cite{{{key}}}. "}, key


def edit_bib(d, rng):
    """Change a title of a cited entry in a `.bib`: (edit, description)."""
    c = cited(d)
    for dp, _, fs in os.walk(d):
        for f in sorted(fs):
            if not f.endswith(".bib"):
                continue
            p = os.path.join(dp, f)
            t = open(p, "rb").read()
            for m in re.finditer(rb"@\s*\w+\s*\{\s*([^,\s]+)\s*,", t):
                key = m.group(1).decode("utf-8", "replace")
                if key not in c:
                    continue
                nxt = t.find(b"\n@", m.end())
                seg = t[m.end(): nxt if nxt > 0 else len(t)]
                tm = re.search(rb"title\s*=\s*[{\"]", seg, re.I)
                if not tm or tm.start() > 0 and seg[tm.start() - 1:tm.start()].isalpha():
                    continue
                at = m.end() + tm.end()
                return {"path": os.path.relpath(p, d), "offset": at, "delete": 0, "insert": "Revised "}, key
    return None


def edit_index(d, main, rng):
    t = open(os.path.join(d, main), "rb").read()
    if b"\\index{" not in t:
        return None
    b = t.find(b"\\begin{document}")
    ms = [m.end() for m in re.finditer(rb"[a-z]{3}\. ", t[b:])]
    if not ms:
        return None
    at = b + rng.choice(ms)
    word = rng.choice(["zeta", "alpha", "omega", "kappa"]) + str(rng.randint(1, 99))
    return {"path": main, "offset": at, "delete": 0, "insert": f"\\index{{{word}}}"}, word


def apply_edit(d, e):
    p = os.path.join(d, e["path"])
    t = open(p, "rb").read()
    t = t[:e["offset"]] + e["insert"].encode() + t[e["offset"] + e["delete"]:]
    open(p, "wb").write(t)


def files_of(d, exts=(".bbl", ".ind", ".aux")):
    return {k: v for k, v in made_files(d, exts).items()}


def sound_one(a, name, src, main, kinds):
    rng = random.Random(a.seed + sum(map(ord, name)))
    work = os.path.join(a.work, "sound", name)
    shutil.rmtree(work, ignore_errors=True)
    os.makedirs(work)
    cd = os.path.join(work, "cand")
    copy_sources(src, cd)
    recs = []
    try:
        h, ev, ex, settle = host_run(a, cd, main, os.path.join(work))
    except Exception as e:  # noqa: BLE001
        return [{"doc": name, "result": f"FAIL: host open: {e}"}]
    try:
        rid = 10
        for kind in kinds:
            for trial in range(a.trials):
                if kind == "cite":
                    r = edit_cite(cd, main, rng)
                elif kind == "bib":
                    r = edit_bib(cd, rng)
                else:
                    r = edit_index(cd, main, rng)
                if not r:
                    recs.append({"doc": name, "kind": kind, "result": "n/a: no edit site"})
                    break
                e, what = r
                rid += 1
                req = {"id": rid, "root": cd, "main": main, "output_dir": cd, "external_tools": "auto",
                       "incremental": True, "edits": [e]}
                t0 = now()
                ev = h.cycle(req)
                t_settled = now() - t0
                cand_pages = dict(h.pages)
                cand_files = files_of(cd)
                rid += 1
                ex = h.export({"id": rid, "root": cd, "main": main, "output_dir": cd})
                rec = {"doc": name, "kind": kind, "trial": trial, "edit": what, "settled_s": round(t_settled, 3),
                       "first_done_s": round(ev["t_done"] or 0, 3),
                       "compiles": len(ev["dones"]), "modes": [x.get("mode") for x in ev["dones"]],
                       "tools": summary_tools(ev)}
                # from scratch: a fresh host, and latexmk
                fd = os.path.join(work, f"fresh-{rid}")
                copy_sources(cd, fd)
                od = os.path.join(work, f"oracle-{rid}")
                copy_sources(cd, od)
                try:
                    h2, ev2, ex2, _ = host_run(a, fd, main, fd + "-host")
                    fresh_pages = dict(h2.pages)
                    h2.close()
                except Exception as x:  # noqa: BLE001
                    rec["result"] = f"FAIL: fresh host: {x}"
                    recs.append(rec)
                    continue
                fresh_files = files_of(fd)
                mism = []
                if fresh_pages != cand_pages:
                    diff = sorted(i for i in set(fresh_pages) | set(cand_pages)
                                  if fresh_pages.get(i) != cand_pages.get(i))
                    mism.append(f"pages {diff[:10]} of {len(fresh_pages)}")
                for k in sorted(set(fresh_files) | set(cand_files)):
                    if fresh_files.get(k) != cand_files.get(k):
                        mism.append(f"file {k}")
                o = latexmk(a.texbin, od, main)
                if o["ok"] and ex.get("pdf"):
                    r = pt2(o["pdf"], ex["pdf"], os.path.join(work, f"pt2-{rid}"))
                    rec["pt2_vs_latexmk"] = r.get("ok")
                    if not r.get("ok"):
                        mism.append("P-T2 vs latexmk")
                    of = files_of(od, (".bbl", ".ind"))
                    for k in of:
                        if cand_files.get(k) != of[k]:
                            mism.append(f"{k} vs latexmk")
                else:
                    rec["pt2_vs_latexmk"] = None
                    rec["oracle"] = o.get("why") or o.get("tail", "")[-300:]
                rec["mismatches"] = mism
                rec["result"] = "PASS" if not mism else "FAIL"
                recs.append(rec)
                shutil.rmtree(fd, ignore_errors=True)
                shutil.rmtree(fd + "-host", ignore_errors=True)
                shutil.rmtree(od, ignore_errors=True)
    finally:
        h.close()
    return recs


# ---------------------------------------------------------------------------
# bench

def gen_doc(d, pages, style="natbib", nbib=400):
    os.makedirs(d, exist_ok=True)
    rng = random.Random(pages)
    words = ("lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore "
             "et dolore magna aliqua enim ad minim veniam quis nostrud exercitation ullamco laboris nisi aliquip "
             "ex ea commodo consequat duis aute irure in reprehenderit voluptate velit esse cillum").split()
    with open(os.path.join(d, "refs.bib"), "w") as f:
        for i in range(nbib):
            f.write(f"@article{{k{i},\n  author = {{Author{i} and Other{i % 7}}},\n  title = {{Title number {i} "
                    f"on {rng.choice(words)} {rng.choice(words)}}},\n  journal = {{J. {rng.choice(words)}}},\n"
                    f"  year = {{{1950 + i % 70}}},\n  volume = {{{i % 40}}},\n  pages = {{{i}--{i + 9}}}\n}}\n")
    if style == "biblatex":
        pre = "\\usepackage[backend=biber,style=numeric]{biblatex}\n\\addbibresource{refs.bib}\n"
        post = "\\printbibliography\n"
    else:
        pre = "\\usepackage[numbers]{natbib}\n"
        post = "\\bibliographystyle{plainnat}\n\\bibliography{refs}\n"
    body = []
    per_page = 5
    n = 0
    for p in range(pages * per_page):
        if p % (per_page * 3) == 0:
            body.append(f"\\section{{Part {p}}}\n")
        s = " ".join(rng.choice(words) for _ in range(90))
        cites = ""
        if p % 2 == 0 and n < nbib - 40:
            cites = f" \\cite{{k{n}}}."
            n += 1
        body.append(s.capitalize() + "." + cites + "\n\n")
    with open(os.path.join(d, "main.tex"), "w") as f:
        f.write("\\documentclass{article}\n" + pre + "\\begin{document}\n" + "".join(body) + post + "\\end{document}\n")
    return n


def bench(a):
    out = []
    for pages in [int(x) for x in a.pages.split(",")]:
        for style in a.styles.split(","):
            name = f"{style}-{pages}"
            work = os.path.join(a.work, "bench", name)
            shutil.rmtree(work, ignore_errors=True)
            d = os.path.join(work, "doc")
            ncited = gen_doc(d, pages, style)
            h, ev, ex, settle = host_run(a, d, "main.tex", work)
            npages = len(h.pages)
            rec = {"doc": name, "pages": npages, "open_settle_s": round(settle, 3), "open_tools": summary_tools(ev)}
            reps = []
            try:
                t = open(os.path.join(d, "main.tex"), "rb").read()
                for r in range(a.reps):
                    # a \cite of a key not cited yet, in the middle of the document
                    key = f"k{ncited + r}"
                    mid = t.find(b". ", len(t) // 2) + 2
                    e = {"path": "main.tex", "offset": mid, "delete": 0, "insert": f"See \\cite{{{key}}}. "}
                    req = {"id": 100 + r, "root": d, "main": "main.tex", "output_dir": d,
                           "external_tools": "auto", "incremental": True, "edits": [e]}
                    t0 = now()
                    ev = h.cycle(req)
                    tot = now() - t0
                    t = open(os.path.join(d, "main.tex"), "rb").read()
                    aux = open(os.path.join(d, "main.aux"), errors="replace").read()
                    resolved = f"\\bibcite{{{key}}}" in aux or f"{{{key}}}" in aux
                    tools = [x for x in ev["tools"] if x.get("event") == "done"]
                    reps.append({"first_done_ms": round((ev["t_done"] or 0) * 1e3, 1),
                                 "settled_ms": round(tot * 1e3, 1),
                                 "tool_ms": {x["tool"]: x.get("ms") for x in tools},
                                 "compiles": len(ev["dones"]),
                                 "modes": [x.get("mode") for x in ev["dones"]],
                                 "run_ms": [x.get("run_ms") for x in ev["dones"]],
                                 "resolved": resolved})
            finally:
                h.close()
            rec["edits"] = reps
            out.append(rec)
            print(json.dumps(rec), flush=True)
    return out


# ---------------------------------------------------------------------------

def docs_of(a):
    out = []
    for spec in a.doc:
        parts = spec.split(":")
        d, main = parts[0], parts[1] if len(parts) > 1 else "main.tex"
        kinds = parts[2].split(",") if len(parts) > 2 else None
        out.append((os.path.basename(os.path.normpath(d)), os.path.abspath(d), main, kinds))
    if a.list:
        for line in open(a.list):
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            parts = line.split()
            d = os.path.expanduser(parts[0])
            main = parts[1] if len(parts) > 1 else "main.tex"
            kinds = parts[2].split(",") if len(parts) > 2 else None
            out.append((os.path.basename(os.path.normpath(d)), os.path.abspath(d), main, kinds))
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("mode", choices=["parity", "sound", "bench"])
    ap.add_argument("--host", required=True)
    ap.add_argument("--formats", required=True)
    ap.add_argument("--pool", required=True)
    ap.add_argument("--texbin", default=os.path.dirname(shutil.which("latexmk") or "/Library/TeX/texbin/latexmk"))
    ap.add_argument("--doc", action="append", default=[], help="DIR[:MAIN[:KINDS]]")
    ap.add_argument("--list", help="a file of 'DIR MAIN [KINDS]' lines")
    ap.add_argument("--work", default=os.path.join(tempfile.gettempdir(), "xtools"))
    ap.add_argument("--env", action="append", default=[], help="KEY=VALUE for the host")
    ap.add_argument("--out", help="JSON lines here")
    ap.add_argument("-j", type=int, default=4)
    ap.add_argument("--trials", type=int, default=2)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--pages", default="10,120")
    ap.add_argument("--styles", default="natbib,biblatex")
    ap.add_argument("--reps", type=int, default=5)
    a = ap.parse_args()
    a.env = dict(kv.split("=", 1) for kv in a.env)
    a.host = os.path.abspath(a.host)
    os.makedirs(a.work, exist_ok=True)
    out = open(a.out, "w") if a.out else None

    def emit(r):
        line = json.dumps(r)
        print(line, flush=True)
        if out:
            out.write(line + "\n")
            out.flush()

    if a.mode == "bench":
        for r in bench(a):
            if out:
                out.write(json.dumps(r) + "\n")
        return 0
    import concurrent.futures as cf
    docs = docs_of(a)
    bad = 0
    with cf.ThreadPoolExecutor(a.j) as ex:
        if a.mode == "parity":
            futs = [ex.submit(parity_one, a, n, d, m) for n, d, m, _ in docs]
            for f in futs:
                r = f.result()
                bad += not r["result"].startswith(("PASS", "excluded"))
                emit(r)
        else:
            futs = [ex.submit(sound_one, a, n, d, m, k or ["cite", "bib"]) for n, d, m, k in docs]
            for f in futs:
                for r in f.result():
                    bad += r["result"].startswith("FAIL")
                    emit(r)
    print(f"# {a.mode}: {len(docs)} documents, {bad} failing", flush=True)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
