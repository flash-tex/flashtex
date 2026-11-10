#!/usr/bin/env python3
"""Differential fuzzer for BOX-MEMO and D9 replay (README.md).

Every case (gen.py) runs in INITEX three times per subsystem -- off, on and
verify -- and any difference in the log, the DVI or PDF, the verifier's own
counts, or a verify exit of 3, is a finding. Findings are grouped by a
signature, and minimize.py shrinks one to a minimal repro.

    fuzz_replay.py run --engine BIN [--pool POOL] --seeds 0-999 --jobs 2 --out DIR
    fuzz_replay.py check --engine BIN DIR     (a directory holding t.tex, u.tex)

Stdlib only.
"""
import argparse
import concurrent.futures
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import gen  # noqa: E402

# --grammar align: tools/fuzz-parity/gen_align.py, alignments and output
# routines inside \measure (shared with the parity fuzzer).
_AL = os.path.join(os.path.dirname(HERE), "fuzz-parity")

REPO = os.path.dirname(os.path.dirname(HERE))
DEFAULT_POOL = os.path.join(REPO, "crates", "flashtex-engine", "pdftex.pool")
TIMEOUT = 20

# subsystem -> (mode variable, names variable, stats variable, the other's mode variable)
SUBSYSTEMS = {
    "bm": ("FLASHTEX_BOXMEMO", "FLASHTEX_BOXMEMO_NAMES",
           "FLASHTEX_BOXMEMO_STATS", "FLASHTEX_INTRINSICS"),
    "d9": ("FLASHTEX_INTRINSICS", "FLASHTEX_INTRINSIC_NAMES",
           "FLASHTEX_INTRINSICS_STATS", "FLASHTEX_BOXMEMO"),
}
MODES = ("off", "on", "verify")


def parse_stats(text):
    """The top-level `name: number` fields of a `{:#?}` statistics dump
    (nested maps are skipped)."""
    out = {}
    depth = 0
    for line in text.splitlines():
        s = line.strip()
        m = re.match(r'^"?([A-Za-z_][A-Za-z0-9_]*)"?:\s*(-?\d+),?$', s)
        if m and depth == 1:
            out.setdefault(m.group(1), int(m.group(2)))
        depth += s.count("{") - s.count("}")
    return out


def run_one(engine, pool, d, sub, mode, timeout=TIMEOUT):
    var, names, stats_var, other = SUBSYSTEMS[sub]
    stats = os.path.join(d, f"stats-{sub}-{mode}.txt")
    for f in ("t.log", "t.dvi", "t.pdf", stats):
        try:
            os.remove(os.path.join(d, f))
        except FileNotFoundError:
            pass
    env = dict(os.environ)
    for k in list(env):
        if (k.startswith("FLASHTEX_BOXMEMO") or k.startswith("FLASHTEX_INTRINSIC")) \
                and not k.endswith("_FAULT"):
            del env[k]
    env.update({
        "FLASHTEX_POOL": pool,
        "FLASHTEX_RESOLVER": "cwd",
        "SOURCE_DATE_EPOCH": "0",
        "FORCE_SOURCE_DATE": "1",
        var: mode,
        names: "measure",
        stats_var: stats,
        other: "off",
    })
    if sub == "bm":
        env["FLASHTEX_BOXMEMO_VERIFY_FAIL"] = "1"
    try:
        p = subprocess.run([engine, "-ini", "-etex", r"\input t"], cwd=d,
                           env=env, stdin=subprocess.DEVNULL,
                           stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                           timeout=timeout)
        code, err = p.returncode, p.stderr.decode("utf-8", "replace")
    except subprocess.TimeoutExpired:
        code, err = "timeout", ""

    def rd(name):
        try:
            with open(os.path.join(d, name), "rb") as f:
                return f.read()
        except FileNotFoundError:
            return None
    st = rd(stats)
    return {
        "exit": code,
        "log": rd("t.log"),
        "out": (rd("t.dvi"), rd("t.pdf")),
        "stats": parse_stats(st.decode("utf-8", "replace")) if st else {},
        "verify_msgs": [l for l in err.splitlines() if "verify" in l.lower()],
    }


def first_diff(a, b):
    a, b = a or b"", b or b""
    al, bl = a.splitlines(), b.splitlines()
    for i, (x, y) in enumerate(zip(al, bl)):
        if x != y:
            return f"l.{i + 1}: {x[:160]!r} -> {y[:160]!r}"
    return f"length {len(al)} -> {len(bl)} lines"


def verify_field(msgs):
    """A short class for the verifier's first message: the first state field
    it names (`param_ptr`, `first`, `hash[...]`, ...)."""
    for m in msgs:
        if "FLASHTEX_BOXMEMO_VERIFY_FAIL is set" in m:
            continue
        # BOX-MEMO's whole-state check: "... comparison reads: param_ptr: 0x0 -> 0x1"
        f = re.search(r"comparison reads: ([A-Za-z_][A-Za-z_0-9]*)(\[\d+\])?", m)
        if f:
            return f.group(1) + ("[]" if f.group(2) else "")
        # the node walk: "... structures differ: box dimension differs (668 vs 840) ..."
        f = re.search(r"structures differ: ([^(:]*)", m)
        if f:
            return re.sub(r"\s+", "_", re.sub(r"-?\d+", "N", f.group(1).strip()))[:40]
        # the structural checks: the first difference, a quoted string
        f = re.search(r'"([^"]*)"', m)
        if f:
            head = f.group(1).split(":")[0]
            return re.sub(r"\s+", "_", re.sub(r"-?\d+", "N", head)).strip("_")[:40]
    return ""


def compare(res):
    """Findings of one subsystem's three runs: a list of kinds, and details."""
    off, on, ver = res["off"], res["on"], res["verify"]
    kinds, det = [], {}
    for m in MODES:
        if res[m]["exit"] == "timeout":
            kinds.append(f"timeout-{m}")
        elif isinstance(res[m]["exit"], int) and (res[m]["exit"] < 0 or res[m]["exit"] == 101):
            kinds.append(f"crash-{m}")
    if off["log"] != on["log"]:
        kinds.append("on-log")
        det["on-log"] = first_diff(off["log"], on["log"])
    if off["out"] != on["out"]:
        kinds.append("on-out")
    if off["log"] != ver["log"]:
        kinds.append("verify-log")
        det["verify-log"] = first_diff(off["log"], ver["log"])
    if off["out"] != ver["out"]:
        kinds.append("verify-out")
    if ver["exit"] == 3:
        kinds.append("verify-exit3")
    vs = ver["stats"]
    if vs.get("verify_differences", 0) or vs.get("full_differences", 0):
        kinds.append("verify-diff")
    if off["exit"] != on["exit"] and "on-log" not in kinds:
        kinds.append("on-exit")
    if ver["verify_msgs"]:
        det["verify_msgs"] = ver["verify_msgs"][:3]
    return kinds, det


def check_dir(engine, pool, d, subs=("bm", "d9")):
    """Run the case in `d` under each subsystem; {sub: (kinds, details, hits)}."""
    out = {}
    for sub in subs:
        res = {m: run_one(engine, pool, d, sub, m) for m in MODES}
        kinds, det = compare(res)
        hits = res["on"]["stats"].get("hits", res["on"]["stats"].get("replays", 0))
        out[sub] = (kinds, det, hits)
    return out


def signature(sub, kinds, det):
    k = sorted(set(kinds) - {"verify-log", "verify-out", "on-exit"}) or sorted(kinds)
    return f"{sub}:{'+'.join(k)}:{verify_field(det.get('verify_msgs', []))}"


def write_case(d, files, tfms=()):
    os.makedirs(d, exist_ok=True)
    for name, text in files.items():
        with open(os.path.join(d, name), "w") as f:
            f.write(text)
    for t in tfms:
        shutil.copy(t, d)


def find_tfms(names=("cmr10.tfm", "cmr7.tfm")):
    """cmr10.tfm and cmr7.tfm from kpsewhich, or () when TeX Live is absent
    (the generator then makes no font items). Data only: the engine under
    test reads them from the case directory."""
    paths = []
    for n in names:
        try:
            p = subprocess.run(["kpsewhich", n], capture_output=True, text=True,
                               timeout=10).stdout.strip()
        except (OSError, subprocess.TimeoutExpired):
            return ()
        if not p:
            return ()
        paths.append(p)
    return tuple(paths)


_ALIGN = []


def load_align():
    """fuzz-parity's gen_align, loaded once (before any worker thread): it
    imports fuzz-parity's own gen.py, which shares this gen.py's name."""
    if not _ALIGN:
        import importlib.util
        spec = importlib.util.spec_from_file_location("gen_align", os.path.join(_AL, "gen_align.py"))
        mod = importlib.util.module_from_spec(spec)
        saved = sys.modules.pop("gen")
        sys.path.insert(0, _AL)
        try:
            spec.loader.exec_module(mod)
        finally:
            sys.path.remove(_AL)
            sys.modules["gen"] = saved
        _ALIGN.append(mod)
    return _ALIGN[0]


def generate(seed, fonts, grammar="default"):
    if grammar == "align":
        return load_align().generate_replay(seed, fonts=fonts)
    if grammar == "align-plain":
        return load_align().generate_replay_plain(seed)
    return gen.generate(seed, fonts=fonts)


def fuzz_seed(engine, pool, seed, fonts, tfms, work, grammar="default"):
    files = generate(seed, fonts, grammar)
    d = tempfile.mkdtemp(prefix=f"s{seed}-", dir=work)
    try:
        write_case(d, files, tfms if fonts else ())
        res = check_dir(engine, pool, d)
    finally:
        shutil.rmtree(d, ignore_errors=True)
    return seed, files, res


def parse_seeds(spec):
    out = []
    for part in spec.split(","):
        if "-" in part:
            a, b = part.split("-")
            out.extend(range(int(a), int(b) + 1))
        elif part:
            out.append(int(part))
    return out


def cmd_run(a):
    if a.nice:
        try:
            os.nice(19 - os.nice(0))
        except OSError:  # already at the lowest priority, or not permitted
            pass
    tfms = find_tfms() if a.fonts else ()
    if a.grammar in ("align", "align-plain"):
        load_align()
    if a.grammar == "align-plain":
        # plain.tex, hyphen.tex and its fonts' TFMs, copied like the TFMs
        tfms = find_tfms(load_align().PLAIN_FILES)
        a.fonts = True
    fonts = bool(tfms)
    os.makedirs(a.out, exist_ok=True)
    work = tempfile.mkdtemp(prefix="fr-work-", dir=a.work or None)
    seeds = parse_seeds(a.seeds)
    sigs = {}
    totals = {"cases": 0, "hits_bm": 0, "hits_d9": 0, "cases_hit_bm": 0, "cases_hit_d9": 0}
    try:
        with concurrent.futures.ThreadPoolExecutor(a.jobs) as ex:
            futs = [ex.submit(fuzz_seed, a.engine, a.pool, s, fonts, tfms, work, a.grammar)
                    for s in seeds]
            for fu in concurrent.futures.as_completed(futs):
                seed, files, res = fu.result()
                totals["cases"] += 1
                for sub, (kinds, det, hits) in res.items():
                    totals[f"hits_{sub}"] += hits
                    totals[f"cases_hit_{sub}"] += 1 if hits else 0
                    if not kinds:
                        continue
                    sig = signature(sub, kinds, det)
                    ent = sigs.setdefault(sig, {"seeds": [], "example": None})
                    ent["seeds"].append(seed)
                    if ent["example"] is None or len(files["t.tex"]) < len(ent["example"]["files"]["t.tex"]):
                        ent["example"] = {"seed": seed, "files": files, "kinds": kinds, "details": det}
                    if not a.quiet:
                        print(f"seed {seed}: {sig}", flush=True)
    finally:
        shutil.rmtree(work, ignore_errors=True)
    for sig, ent in sigs.items():
        h = hashlib.sha1(sig.encode()).hexdigest()[:10]
        d = os.path.join(a.out, h)
        write_case(d, ent["example"]["files"], tfms if fonts else ())
        with open(os.path.join(d, "finding.json"), "w") as f:
            json.dump({"signature": sig, "seeds": sorted(ent["seeds"]),
                       "fonts": fonts, **{k: v for k, v in ent["example"].items() if k != "files"}},
                      f, indent=1)
    summary = {"totals": totals,
               "signatures": {s: sorted(e["seeds"]) for s, e in sigs.items()}}
    with open(os.path.join(a.out, "summary.json"), "w") as f:
        json.dump(summary, f, indent=1)
    print(json.dumps(summary["totals"]))
    for s, e in sorted(sigs.items()):
        print(f"{len(e['seeds']):5d}  {s}  (e.g. seed {e['example']['seed']})")
    return 1 if sigs else 0


def cmd_check(a):
    res = check_dir(a.engine, a.pool, a.dir)
    bad = 0
    for sub, (kinds, det, hits) in res.items():
        print(f"{sub}: hits {hits} kinds {kinds or 'clean'}")
        for k, v in det.items():
            print(f"   {k}: {v}")
        bad |= bool(kinds)
    return 1 if bad else 0


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sp = ap.add_subparsers(dest="cmd", required=True)
    r = sp.add_parser("run")
    r.add_argument("--engine", required=True)
    r.add_argument("--pool", default=DEFAULT_POOL)
    r.add_argument("--seeds", default="0-99")
    r.add_argument("--jobs", type=int, default=2)
    r.add_argument("--out", required=True)
    r.add_argument("--work", default=None)
    r.add_argument("--fonts", action="store_true",
                   help="copy cmr10/cmr7 from kpsewhich and generate font items")
    r.add_argument("--nice", action="store_true")
    r.add_argument("--grammar", choices=("default", "align", "align-plain"), default="default",
                   help="align: tools/fuzz-parity/gen_align.py's replay cases; align-plain: "
                        "its plain.tex documents, which fuzz-parity's fuzz.py also runs")
    r.add_argument("--quiet", action="store_true")
    c = sp.add_parser("check")
    c.add_argument("--engine", required=True)
    c.add_argument("--pool", default=DEFAULT_POOL)
    c.add_argument("dir")
    a = ap.parse_args(argv)
    return cmd_run(a) if a.cmd == "run" else cmd_check(a)


if __name__ == "__main__":
    sys.exit(main())
