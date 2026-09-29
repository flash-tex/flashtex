#!/usr/bin/env python3
"""Structure-aware differential fuzzer: candidate engine vs oracle. MIT.

Mutates seed cases (or generates fresh inputs) with tools/fuzz/gen.py,
runs each on both engines through the lockstep capture(), and records
where they disagree or where the candidate crashes. Stdlib only.
"""
import argparse
import hashlib
import importlib.util
import json
import os
import random
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import gen

_spec = importlib.util.spec_from_file_location(
    "lockstep_run", os.path.join(HERE, "..", "lockstep", "run.py"))
lockstep_run = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(lockstep_run)

CLASSES = ("equal", "diverge", "candidate-crash", "oracle-crash",
           "both-fail", "timeout")
STORE = ("diverge", "candidate-crash", "oracle-crash", "timeout")
# Env the candidate may need; passed through to the candidate's capture()
# call only (extra_env on top of the pinned environment), never the oracle.
CANDIDATE_ENV_VARS = ("FLASHTEX_POOL", "FLASHTEX_FORMATS")


def candidate_env():
    env = {k: v for k, v in os.environ.items() if k in CANDIDATE_ENV_VARS}
    return env or None


def is_crash(returncode, log):
    return (returncode is not None
            and (returncode < 0 or returncode == 101
                 or "panicked at" in (log or "")))


def first_diff(cand_rc, cand_log, orc_rc, orc_log):
    if cand_rc != orc_rc:
        return "returncode candidate=%s oracle=%s" % (cand_rc, orc_rc)
    a = lockstep_run.compared_lines(cand_log or "")
    b = lockstep_run.compared_lines(orc_log or "")
    n = max(len(a), len(b))
    for i in range(n):
        x = a[i] if i < len(a) else "<EOF>"
        y = b[i] if i < len(b) else "<EOF>"
        if x != y:
            return "line %d: %r vs %r" % (i + 1, x[:160], y[:160])
    return None


def classify(cand_rc, cand_log, orc_rc, orc_log, timeouts):
    if timeouts:
        return "timeout"
    cand_crash = is_crash(cand_rc, cand_log)
    orc_crash = is_crash(orc_rc, orc_log)
    if cand_crash:
        return "candidate-crash"
    if orc_crash:
        return "oracle-crash"
    if cand_rc != 0 and orc_rc != 0:
        return "both-fail"
    if first_diff(cand_rc, cand_log, orc_rc, orc_log) is not None:
        return "diverge"
    return "equal"


def run_one(text, candidate, oracle, timeout):
    """Run text on both engines; return (class, cand, orc, timeouts)."""
    workdir = tempfile.mkdtemp(prefix="fuzz-")
    try:
        shutil.copy(lockstep_run.PRELUDE, os.path.join(workdir, "prelude.tex"))
        tex_path = os.path.join(workdir, "fuzz.tex")
        with open(tex_path, "w") as fh:
            fh.write(text)
        results = []
        timeouts = []
        for binary, extra in ((candidate, candidate_env()), (oracle, None)):
            try:
                cap = lockstep_run.capture(tex_path, binary, workdir,
                                           extra_env=extra, timeout=timeout)
                results.append((cap.returncode, cap.log))
                timeouts.append(False)
            except subprocess.TimeoutExpired:
                results.append((None, ""))
                timeouts.append(True)
        (cand_rc, cand_log), (orc_rc, orc_log) = results
        timed_out = timeouts[0] or timeouts[1]
        cls = classify(cand_rc, cand_log, orc_rc, orc_log, timed_out)
        if timed_out:
            diff = "timeout: %s" % "/".join(
                s for s, t in (("candidate", timeouts[0]),
                               ("oracle", timeouts[1])) if t)
        else:
            diff = first_diff(cand_rc, cand_log, orc_rc, orc_log)
        return cls, cand_rc, orc_rc, diff
    finally:
        shutil.rmtree(workdir, ignore_errors=True)


def load_seeds(seeds_dir):
    try:
        names = sorted(f for f in os.listdir(seeds_dir) if f.endswith(".tex"))
    except OSError:
        return []
    out = []
    for name in names:
        try:
            with open(os.path.join(seeds_dir, name)) as fh:
                out.append((name, fh.read()))
        except OSError:
            continue
    return out


def run_fuzz(candidate, oracle, seeds_dir, out_dir, iterations, seed,
             timeout):
    rng = random.Random(seed)
    seeds = load_seeds(seeds_dir)
    counts = {c: 0 for c in CLASSES}
    for i in range(iterations):
        if seeds and rng.random() < 0.5:
            name, base = seeds[rng.randrange(len(seeds))]
            text, mutation = gen.mutate_with_info(base, rng)
            origin = name
        else:
            text, mutation = gen.mutate_with_info(gen.generate(rng), rng)
            origin = "generated"
        cls, cand_rc, orc_rc, diff = run_one(text, candidate, oracle,
                                             timeout)
        counts[cls] += 1
        if cls in STORE:
            digest = hashlib.sha256(text.encode("utf-8")).hexdigest()[:16]
            cls_dir = os.path.join(out_dir, cls)
            os.makedirs(cls_dir, exist_ok=True)
            with open(os.path.join(cls_dir, digest + ".tex"), "w") as fh:
                fh.write(text)
            with open(os.path.join(cls_dir, digest + ".json"), "w") as fh:
                json.dump({"iteration": i, "seed": origin,
                           "mutation": mutation,
                           "candidate_returncode": cand_rc,
                           "oracle_returncode": orc_rc,
                           "first_diff": diff}, fh, indent=2)
        if (i + 1) % 100 == 0:
            print("fuzz %d/%d: %s"
                  % (i + 1, iterations,
                     " ".join("%s=%d" % (c, counts[c]) for c in CLASSES)))
    return counts


def main(argv=None):
    ap = argparse.ArgumentParser(description="differential TeX fuzzer")
    ap.add_argument("--candidate", required=True, help="candidate engine")
    ap.add_argument("--oracle", required=True, help="oracle engine")
    ap.add_argument("--seeds", required=True, help="seed .tex directory")
    ap.add_argument("--out", required=True, help="output directory")
    ap.add_argument("--iterations", type=int, required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--timeout", type=float, required=True,
                    help="per-engine timeout in seconds")
    args = ap.parse_args(argv)
    for label, binary in (("candidate", args.candidate),
                          ("oracle", args.oracle)):
        if not (os.path.isfile(binary) or shutil.which(binary)):
            print("error: %s not found: %s" % (label, binary),
                  file=sys.stderr)
            return 2
    if args.iterations < 0:
        print("error: --iterations must be >= 0", file=sys.stderr)
        return 2
    counts = run_fuzz(args.candidate, args.oracle, args.seeds, args.out,
                      args.iterations, args.seed, args.timeout)
    print("done: %d iterations: %s"
          % (args.iterations,
             " ".join("%s=%d" % (c, counts[c]) for c in CLASSES)))
    return counts


if __name__ == "__main__":
    result = main()
    sys.exit(0 if isinstance(result, dict) else result)
