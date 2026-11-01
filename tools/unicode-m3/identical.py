#!/usr/bin/env python3
"""Byte identity of two builds of the Unicode engine (lane M3-FAST): a
speed-up must leave every output byte as it was.

    python3 tools/unicode-m3/identical.py --base BIN --head BIN --cases-dir DIR [...]
        [--base-host BIN --head-host BIN --client dl3-client] [--jobs 4] [--json OUT]

For each case (latex.py's case layout) and each build, from a format that
build makes itself:

* the engine as `xelatex` in PDF mode, run until the `.aux` no longer
  changes (at most 4 runs): the PDF and the log;
* one more run with `-no-pdf`: the XDV;
* with `--*-host` and `--client`: the case through that build's
  `flashtex-host-unicode` in a host of its own (dl3-client, the host's
  `.aux` passes): the PDF the host writes.

Compared byte for byte, the logs after the job's own directory is replaced
and the format line (`(preloaded format=xelatex 2026.10.12)`, its build
date) is dropped. The pinned clock (SOURCE_DATE_EPOCH=0,
FORCE_SOURCE_DATE=1) makes the rest reproducible. Exit status 1 when any
output differs.

`--loaded-state`: also the state of `--head`'s word space right after the
format load (`FLASHTEX_LOADED_STATE`), with INITEX's table entries set up
first and without (changes/virtex.ch); the two must be equal block for
block, so a run is the same from there on.
"""
import argparse
import hashlib
import json
import os
import re
import shutil
import socket
import subprocess
import sys
import tempfile
import time
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(os.path.dirname(HERE), "xetex-lockstep"))
import latex as L  # noqa: E402

PDF_ARGS = [a for a in L.RUN_ARGS if a != "-no-pdf"]

# Scalars INITEX's code sets and the format load does not (run 38211022101
# found exactly these):
# * the clock (`sys_*`, and `epochseconds`/`microseconds`, the random
#   seed's and \pdfelapsedtime's base): xetex.web §1337 reads it again
#   (`fix_date_and_time`) right after the load, in every run;
# * what `init_prim` leaves: `prim_used` (read only by `primitive`, which
#   only INITEX calls) and `cur_val` (the last primitive's location). A
#   production run of web2c's xetex never runs `init_prim` and starts with
#   both at 0, as this port's production runs now do.
LATER_SET = {"sys_time", "sys_day", "sys_month", "sys_year", "epochseconds", "microseconds",
             "prim_used", "cur_val"}


def digest(p):
    if not os.path.exists(p):
        return None
    return hashlib.sha256(open(p, "rb").read()).hexdigest()


def log_digest(p, d):
    if not os.path.exists(p):
        return None
    t = open(p, "rb").read().decode("utf-8", "surrogateescape")
    t = t.replace(d, "<DIR>")
    t = re.sub(r"\(preloaded format=[^)]*\)", "(preloaded format)", t)
    t = re.sub(r"^This is XeTeX.*$", "This is XeTeX", t, count=1, flags=re.M)
    return hashlib.sha256(t.encode("utf-8", "surrogateescape")).hexdigest()


def engine_case(name, src, binary, fmt, root, timeout):
    meta = L.case_meta(src)
    d = os.path.join(root, name)
    if os.path.exists(d):
        shutil.rmtree(d)
    L.stage(src, name, d)
    exe = L.link(binary, os.path.join(d, ".bin"), "xelatex")
    env = L.env_for(fmt)
    arg = L.doc_arg(name, meta, False)
    aux = os.path.join(d, name + ".aux")
    prev = None
    codes = []
    for _ in range(4):
        try:
            r = subprocess.run([exe] + PDF_ARGS + [arg], cwd=d, env=env, stdin=subprocess.DEVNULL,
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=timeout)
            codes.append(r.returncode)
        except subprocess.TimeoutExpired:
            codes.append("timeout")
            break
        now = digest(aux)
        if now == prev:
            break
        prev = now
    out = {"exit": codes, "pdf": digest(os.path.join(d, name + ".pdf")),
           "log": log_digest(os.path.join(d, name + ".log"), d)}
    try:
        r = subprocess.run([exe] + L.RUN_ARGS + [arg], cwd=d, env=env, stdin=subprocess.DEVNULL,
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=timeout)
        out["xdv_exit"] = r.returncode
    except subprocess.TimeoutExpired:
        out["xdv_exit"] = "timeout"
    out["xdv"] = digest(os.path.join(d, name + ".xdv"))
    shutil.rmtree(d, ignore_errors=True)
    return out


def host_cases(host, client, srcs, root):
    """{name: PDF digest} through one host, every case in turn."""
    os.makedirs(root, exist_ok=True)
    sock = os.path.join(root, "h.sock")
    env = dict(os.environ)
    env.update(SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1", TZ="UTC")
    env["FLASHTEX_FORMAT_CACHE_DIR"] = os.path.join(root, "format-cache")
    log = open(os.path.join(root, "host.log"), "w")
    p = subprocess.Popen([host, "--socket", sock], stdout=log, stderr=subprocess.STDOUT, env=env)
    try:
        for _ in range(9000):
            if os.path.exists(sock):
                try:
                    s = socket.socket(socket.AF_UNIX)
                    s.connect(sock)
                    s.close()
                    break
                except OSError:
                    pass
            if p.poll() is not None:
                raise SystemExit("the host exited: see %s" % log.name)
            time.sleep(0.1)
        out = {}
        for name, src in sorted(srcs.items()):
            d = os.path.join(root, "docs", name)
            L.stage(src, name, d)
            od = os.path.join(root, "build", name)
            os.makedirs(od, exist_ok=True)
            r = subprocess.run([client, "--socket", sock, "--root", d, "--main", name + ".tex",
                                "--output-dir", od, "--quiet"], stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE, text=True, timeout=3600)
            rows = [json.loads(x) for x in r.stdout.splitlines() if x.startswith("{")]
            out[name] = {"status": (rows[-1] if rows else {}).get("status"),
                         "pdf": digest(os.path.join(od, name + ".pdf"))}
        return out
    finally:
        p.terminate()
        p.wait()


def loaded_state(binary, fmt, tmp):
    """The state after the format load (FLASHTEX_LOADED_STATE, system.rs)
    of a production run of `binary`, with INITEX's table entries set up
    first (FLASHTEX_INITEX_TABLES=1, as before changes/virtex.ch's
    `initex_tables`) and without (as web2c's production runs): the arrays
    whose blocks differ, an empty list when the two are the same."""
    d = os.path.join(tmp, "loaded-state")
    os.makedirs(d, exist_ok=True)
    exe = L.link(binary, os.path.join(d, ".bin"), "xelatex")
    states = []
    for tables in ("1", "0"):
        out = os.path.join(d, "state-%s.txt" % tables)
        env = L.env_for(fmt)
        env.update(FLASHTEX_LOADED_STATE=out, FLASHTEX_INITEX_TABLES=tables)
        subprocess.run([exe] + L.RUN_ARGS + ["\\csname @@end\\endcsname"], cwd=d, env=env,
                       stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        states.append(open(out).read().splitlines() if os.path.exists(out) else None)
    if None in states or not states[0]:
        return {"error": "no state written"}
    a, b = states
    names = scalar_names()
    diff, scalars = set(), []
    for x, y in zip(a, b):
        if x == y:
            continue
        key = x.split("\t")[0]
        if key.startswith("scalar "):
            i = int(key.split()[1])
            scalars.append(names[i] if i < len(names) else key)
        elif key != "(scalars)":  # the spilled scalars, named above
            diff.add(key)
    return {"blocks": len(a), "same_length": len(a) == len(b), "arrays_differ": sorted(diff),
            "scalars_differ": scalars}


def scalar_names():
    """The scalar globals in `visit_scalars`' order (src/generated/globals.rs)."""
    src = os.path.join(os.path.dirname(os.path.dirname(HERE)), "crates", "flashtex-xetex", "src",
                       "generated", "globals.rs")
    text = open(src).read()
    body = text[text.index("pub fn visit_scalars"):]
    body = body[:body.index("\n    }\n")]
    return re.findall(r"v\.(?:pod|arr_len)\(&mut self\.(\w+)\)", body)


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--base", required=True)
    ap.add_argument("--head", required=True)
    ap.add_argument("--base-host")
    ap.add_argument("--head-host")
    ap.add_argument("--client")
    ap.add_argument("--cases-dir", action="append", required=True)
    ap.add_argument("--jobs", type=int, default=4)
    ap.add_argument("--timeout", type=float, default=1800)
    ap.add_argument("--loaded-state", action="store_true",
                    help="also compare --head's state after the format load with and without "
                         "INITEX's table entries")
    ap.add_argument("--json")
    a = ap.parse_args(argv)
    srcs = L.find_cases(a.cases_dir)
    tmp = tempfile.mkdtemp(prefix="unicode-identical-")
    builds = {"base": os.path.abspath(a.base), "head": os.path.abspath(a.head)}
    fmts = {}
    for who, b in builds.items():
        d = os.path.join(tmp, "fmt-" + who)
        os.makedirs(d)
        secs, code, _ = L.build_format(b, d)
        print("format (%s): exit %s, %.1f s" % (who, code, secs), flush=True)
        if code != 0:
            return 2
        fmts[who] = d
    state = None
    if a.loaded_state:
        state = loaded_state(builds["head"], fmts["head"], tmp)
        print("state after the format load, INITEX tables against none: %s" % json.dumps(state),
              flush=True)
    res = {}
    with ThreadPoolExecutor(a.jobs) as ex:
        futs = {(n, who): ex.submit(engine_case, n, srcs[n], builds[who], fmts[who],
                                    os.path.join(tmp, "run-" + who), a.timeout)
                for n in sorted(srcs) for who in builds}
        for n in sorted(srcs):
            b, h = futs[(n, "base")].result(), futs[(n, "head")].result()
            diff = [k for k in ("exit", "pdf", "log", "xdv_exit", "xdv") if b.get(k) != h.get(k)]
            res[n] = {"base": b, "head": h, "differs": diff}
            print("%s %-36s pdf %s xdv %s%s" % ("SAME" if not diff else "DIFF", n,
                                                (h["pdf"] or "-")[:12], (h["xdv"] or "-")[:12],
                                                (" differs: " + ",".join(diff)) if diff else ""),
                  flush=True)
    if a.base_host and a.head_host and a.client:
        hb = host_cases(os.path.abspath(a.base_host), os.path.abspath(a.client), srcs,
                        os.path.join(tmp, "host-base"))
        hh = host_cases(os.path.abspath(a.head_host), os.path.abspath(a.client), srcs,
                        os.path.join(tmp, "host-head"))
        for n in sorted(srcs):
            same = hb.get(n) == hh.get(n)
            res[n]["host"] = {"base": hb.get(n), "head": hh.get(n)}
            if not same:
                res[n]["differs"].append("host-pdf")
            print("%s %-36s host pdf %s %s" % ("SAME" if same else "DIFF", n,
                                               ((hh.get(n) or {}).get("pdf") or "-")[:12],
                                               (hh.get(n) or {}).get("status")), flush=True)
    bad = [n for n, r in res.items() if r["differs"]]
    nonnull = sum(1 for r in res.values() if r["head"]["pdf"])
    print("\n%d cases, %d with a PDF; %d identical, %d differ%s" % (
        len(res), nonnull, len(res) - len(bad), len(bad), (": " + ", ".join(bad)) if bad else ""))
    state_bad = state is not None and (
        state.get("error") or state["arrays_differ"] or not state["same_length"]
        or set(state["scalars_differ"]) - LATER_SET)
    if state is not None:
        print("loaded state: %s" % ("SAME" if not state_bad else "DIFFERS %s" % json.dumps(state)))
    if a.json:
        with open(a.json, "w") as f:
            json.dump({"cases": res, "loaded_state": state}, f, indent=1)
    shutil.rmtree(tmp, ignore_errors=True)
    return 1 if bad or state_bad else 0


if __name__ == "__main__":
    sys.exit(main())
