#!/usr/bin/env python3
"""Where Unicode mode's start-up goes (lane M3-FAST): callgrind profiles of
the engine's and the host's start-up, and of two bench documents, with the
cost attributed to the parts M3's latency row names (format load, font
loading, map files, kpathsea, process start).

    python3 tools/unicode-m3/profile_startup.py --engine BIN --host BIN --out DIR
        [--only NAME ...] [--fmt-dir DIR]

Each profile is one run under `valgrind --tool=callgrind` (user-space
instructions; the runner's `perf` cannot count). Written to DIR:

* `NAME.cg`: the raw profile;
* `NAME.incl.txt` and `NAME.self.txt`: `callgrind_annotate`, inclusive and
  self cost, the 120 costliest functions;
* `profile.json`: each profile's total and its buckets (below);
* `summary.md`: the buckets as a table.

The profiles:

* `engine-startup`: the engine as `xelatex`, from the process start to the
  end of `\\csname @@end\\endcsname` (latency.py's `startup` row: what every
  cold compile pays before the document);
* `engine-b1`, `engine-b2`: bench documents b1 (article) and b2 (fontspec,
  TeX Gyre Termes by name), PDF written, warm `.aux`;
* `host-hello`: `flashtex-host-unicode` from the process start to its HELLO
  reply (host_startup.py's run), with its format cache already built.

Buckets are inclusive costs of named functions (a function's cost includes
its callees; buckets can overlap where one calls another, and the table says
which contains which). A bucket is the largest inclusive cost among the
functions whose names match its pattern.
"""
import argparse
import json
import os
import re
import shutil
import socket
import subprocess
import sys
import tempfile
import time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(os.path.dirname(HERE), "xetex-lockstep"))
sys.path.insert(0, HERE)
import latex as L  # noqa: E402  (tools/xetex-lockstep/latex.py)
import host_startup as HS  # noqa: E402

STARTUP = "\\csname @@end\\endcsname"

# (bucket, regex over callgrind_annotate's function column). Inclusive.
BUCKETS = [
    ("total: main", r"(^|[ :])main$|::main$"),
    ("format: load_fmt_file", r"load_fmt_file"),
    ("format: open_fmt_file", r"open_fmt_file"),
    ("format: undump (zlib inflate)", r"\binflate\b|inflate_fast|gz_read|gzread|gz_decomp"),
    ("initialize", r"::initialize$|\binitialize\b"),
    ("init_prim", r"init_prim"),
    ("kpathsea: resolver set-up", r"resolver.*(new|for_progname|build|init|load)|kpse.*init|with_resolver"),
    ("kpathsea: ls-R", r"ls_r|lsr|LsR|parse_ls"),
    ("kpathsea: texmf.cnf", r"texmf_cnf|cnf::|parse_cnf|Cnf"),
    ("kpathsea: lookups", r"resolver.*find|find_ex|::find$"),
    ("font map: pdftex.map", r"fontmap|FontMap|map_file"),
    ("fonts: catalog (discovery walk)", r"catalog|font_discovery|Catalog|scan"),
    ("fonts: FreeType", r"^FT_|\bFT_|ft_|tt_face|cff_"),
    ("fonts: HarfBuzz", r"\bhb_|hb::|OT::"),
    ("fonts: native font load", r"load_native_font|find_native_font|font_inst|loadOTfont|createFont"),
    ("format: ensure (host)", r"format::ensure"),
    ("format: engine_build (SHA-256 of the binary)", r"engine_build"),
    ("format: still_valid", r"still_valid"),
    ("sha256", r"sha256|Sha256"),
    ("tools: Programs::discover", r"Programs.*discover"),
    ("TeX Live discovery", r"discover_texlive"),
    ("output: PDF write", r"out::pdf|write_pdf|finish_pdf|::pdf::"),
    ("dynamic linking (ld.so)", r"_dl_|dl_main|do_lookup_x|ld-linux"),
    ("memset/memcpy", r"memset|memcpy|memmove"),
    ("malloc/free", r"malloc|free|realloc|_int_"),
]


def have(cmd):
    return shutil.which(cmd) is not None


def callgrind(cmd, cwd, env, out, timeout=3600):
    log = out + ".log"
    r = subprocess.run(["valgrind", "--tool=callgrind", "--callgrind-out-file=" + out,
                        "--log-file=" + log] + cmd, cwd=cwd, env=env,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=timeout)
    return r.returncode


def annotate(cg, dest, inclusive):
    r = subprocess.run(["callgrind_annotate", "--inclusive=" + ("yes" if inclusive else "no"),
                        "--threshold=100", cg], stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                       text=True, errors="replace")
    with open(dest, "w") as f:
        f.write(r.stdout)
    return r.stdout


def rows(text):
    """[(cost, function)] of an annotate listing."""
    out = []
    for line in text.splitlines():
        m = re.match(r"^\s*(\d[\d,]*)\s+\(\s*[\d.]+%\)\s+(.*)$", line) or \
            re.match(r"^\s*(\d[\d,]*)\s+(.*)$", line)
        if not m:
            continue
        cost = int(m.group(1).replace(",", ""))
        fn = m.group(2)
        # "file:function [object]"
        fn = re.sub(r"\s*\[[^\]]*\]\s*$", "", fn)
        if ":" in fn and not fn.startswith("<"):
            head, _, tail = fn.partition(":")
            if "/" in head or head.endswith((".rs", ".c", ".cc", ".h", ".hh")) or head == "???":
                fn = tail
        out.append((cost, fn))
    return out


def total(text):
    m = re.search(r"^\s*(\d[\d,]*)\s+\(100\.0%\)\s+PROGRAM TOTALS", text, re.M) or \
        re.search(r"^\s*(\d[\d,]*)\s+PROGRAM TOTALS", text, re.M)
    return int(m.group(1).replace(",", "")) if m else None


def buckets(incl_rows):
    res = {}
    for name, pat in BUCKETS:
        best = None
        rx = re.compile(pat)
        for cost, fn in incl_rows:
            if rx.search(fn) and (best is None or cost > best[0]):
                best = (cost, fn)
        if best:
            res[name] = {"instructions": best[0], "function": best[1][:160]}
    return res


def profile(name, cmd, cwd, env, out_dir):
    cg = os.path.join(out_dir, name + ".cg")
    t = time.time()
    code = callgrind(cmd, cwd, env, cg)
    secs = time.time() - t
    if not os.path.exists(cg):
        print("%s: no profile (exit %s)" % (name, code), flush=True)
        return {"exit": code, "error": "no profile"}
    incl = annotate(cg, os.path.join(out_dir, name + ".incl.txt"), True)
    annotate(cg, os.path.join(out_dir, name + ".self.txt"), False)
    res = {"exit": code, "valgrind_s": round(secs, 1), "total": total(incl),
           "buckets": buckets(rows(incl))}
    print("%s: exit %s, %s instructions" % (name, code, res["total"]), flush=True)
    return res


def host_hello(host, tmp, out_dir, env):
    """The host under callgrind to its HELLO; ended as the app ends it."""
    sock = os.path.join(tmp, "prof.sock")
    cg = os.path.join(out_dir, "host-hello.cg")
    p = subprocess.Popen(["valgrind", "--tool=callgrind", "--callgrind-out-file=" + cg,
                          "--log-file=" + cg + ".log", host, "--socket", sock],
                         stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, env=env)
    t0 = time.time()
    s = None
    while time.time() - t0 < 900:
        try:
            s = socket.socket(socket.AF_UNIX)
            s.connect(sock)
            break
        except OSError:
            s.close()
            s = None
            time.sleep(0.05)
    if s is None:
        p.kill()
        return {"error": "no socket"}
    hello = json.dumps({"protocol": "display-list-v3", "version": [3, 3],
                        "client": "tools/unicode-m3/profile_startup.py"}).encode()
    s.sendall(HS.frame(HS.C_HELLO, hello))
    HS.read_frame(s)
    s.sendall(HS.frame(HS.BYE, b"{}"))
    s.close()
    time.sleep(1.0)
    p.terminate()
    code = p.wait()
    if not os.path.exists(cg):
        return {"exit": code, "error": "no profile"}
    incl = annotate(cg, os.path.join(out_dir, "host-hello.incl.txt"), True)
    annotate(cg, os.path.join(out_dir, "host-hello.self.txt"), False)
    res = {"exit": code, "total": total(incl), "buckets": buckets(rows(incl))}
    print("host-hello: exit %s, %s instructions" % (code, res["total"]), flush=True)
    return res


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--engine", required=True)
    ap.add_argument("--host")
    ap.add_argument("--out", required=True)
    ap.add_argument("--fmt-dir", help="a built xelatex.fmt (else built here by the engine)")
    ap.add_argument("--only", nargs="*")
    a = ap.parse_args(argv)
    if not have("valgrind") or not have("callgrind_annotate"):
        raise SystemExit("valgrind and callgrind_annotate are needed")
    os.makedirs(a.out, exist_ok=True)
    tmp = tempfile.mkdtemp(prefix="unicode-profile-")
    fmt = a.fmt_dir
    if not fmt:
        fmt = os.path.join(tmp, "fmt")
        os.makedirs(fmt)
        secs, code, _ = L.build_format(a.engine, fmt)
        print("format: exit %s, %.1f s" % (code, secs), flush=True)
        if code != 0:
            return 1
    env = L.env_for(fmt)
    want = (lambda n: not a.only or n in a.only)
    res = {}
    bench = os.path.join(HERE, "bench")
    jobs = [("engine-startup", None, L.RUN_ARGS)]
    pdf_args = [x for x in L.RUN_ARGS if x != "-no-pdf"]
    for name, fn in (("engine-b1", "b1-article"), ("engine-b2", "b2-fontspec-named")):
        jobs.append((name, fn, pdf_args))
    for name, doc, args in jobs:
        if not want(name):
            continue
        d = os.path.join(tmp, name)
        os.makedirs(d, exist_ok=True)
        exe = L.link(a.engine, os.path.join(d, ".bin"), "xelatex")
        if doc:
            shutil.copy(os.path.join(bench, doc + ".tex"), d)
            target = doc + ".tex"
            # warm .aux (and the font caches), as a keystroke's run
            subprocess.run([exe] + args + [target], cwd=d, env=env,
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        else:
            target = STARTUP
        res[name] = profile(name, [exe] + args + [target], d, env, a.out)
    if a.host and want("host-hello"):
        # host_startup.one starts the host with this process's environment
        os.environ.update(SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1", TZ="UTC")
        os.environ.setdefault("FLASHTEX_FORMAT_CACHE_DIR", os.path.join(tmp, "format-cache"))
        henv = dict(os.environ)
        # twice without callgrind: builds the format cache, warms the page cache
        HS.one(os.path.abspath(a.host), [], tmp, 0, False)
        HS.one(os.path.abspath(a.host), [], tmp, 1, False)
        res["host-hello"] = host_hello(os.path.abspath(a.host), tmp, a.out, henv)
    with open(os.path.join(a.out, "profile.json"), "w") as f:
        json.dump(res, f, indent=1)
    lines = ["| profile | bucket | M instr | function |", "|---|---|---:|---|"]
    for name, r in res.items():
        lines.append("| **%s** | total | %s | exit %s |" % (
            name, round(r["total"] / 1e6, 1) if r.get("total") else "?", r.get("exit")))
        for b, v in sorted(r.get("buckets", {}).items(), key=lambda kv: -kv[1]["instructions"]):
            lines.append("| %s | %s | %.1f | `%s` |" % (name, b, v["instructions"] / 1e6,
                                                         v["function"].replace("|", "\\|")[:90]))
    with open(os.path.join(a.out, "summary.md"), "w") as f:
        f.write("\n".join(lines) + "\n")
    print("\n".join(lines))
    shutil.rmtree(tmp, ignore_errors=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
