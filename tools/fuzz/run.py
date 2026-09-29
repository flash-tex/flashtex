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
import re
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


def run_one(text, candidate, oracle, timeout, return_logs=False):
    """Run text on both engines; return (class, cand_rc, orc_rc, diff).

    With return_logs=True, append (cand_log, orc_log) to the tuple.
    """
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
        if return_logs:
            return cls, cand_rc, orc_rc, diff, cand_log, orc_log
        return cls, cand_rc, orc_rc, diff
    finally:
        shutil.rmtree(workdir, ignore_errors=True)


def panic_location(log):
    """Extract the panic location: text after "panicked at" up to the
    first colon-number pair (e.g. "src/main.rs:123"). None when absent."""
    if not log or "panicked at" not in log:
        return None
    after = log.split("panicked at", 1)[1]
    m = re.search(r":\d+", after)
    if not m:
        return None
    return after[:m.start()].strip()


def crash_signature(rc, log):
    if panic_location(log) is not None:
        return "panic:" + panic_location(log)
    if rc is not None and rc < 0:
        return "signal:%d" % (-rc,)
    return "exit:%s" % (rc,)


def signature(cls, cand_rc, cand_log, orc_rc, orc_log, diff):
    """Dedupe signature for a non-equal result, or None when it has none."""
    if cls == "candidate-crash":
        return "candidate-crash:" + crash_signature(cand_rc, cand_log)
    if cls == "oracle-crash":
        return "oracle-crash:" + crash_signature(orc_rc, orc_log)
    if cls == "diverge":
        return "diverge:" + re.sub(r"\d", "N", diff or "")
    if cls == "timeout":
        return "timeout"
    return None


def load_known_signatures(out_dir):
    """Signatures already stored on disk under OUT (from signatures.json
    plus per-case .json sidecars), so repeat runs do not re-store them."""
    known = set()
    try:
        with open(os.path.join(out_dir, "signatures.json")) as fh:
            data = json.load(fh)
        if isinstance(data, dict):
            known.update(data.keys())
    except (OSError, ValueError):
        pass
    try:
        for root, _dirs, files in os.walk(out_dir):
            for name in files:
                if not name.endswith(".json") or name == "signatures.json":
                    continue
                try:
                    with open(os.path.join(root, name)) as fh:
                        info = json.load(fh)
                    if isinstance(info, dict) and info.get("signature"):
                        known.add(info["signature"])
                except (OSError, ValueError):
                    continue
    except OSError:
        pass
    return known


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


def draw_input(rng, seeds):
    """Pick a seed (or fresh input) and mutate it; return (text, mutation,
    origin). Re-draws the mutation (up to 3 extra times, without running
    any engine) when the mutated text is identical to its seed."""
    if seeds and rng.random() < 0.5:
        name, base = seeds[rng.randrange(len(seeds))]
        origin = name
    else:
        base = gen.generate(rng)
        origin = "generated"
    text, mutation = gen.mutate_with_info(base, rng)
    for _ in range(3):
        if text != base:
            break
        text, mutation = gen.mutate_with_info(base, rng)
    return text, mutation, origin


def run_fuzz(candidate, oracle, seeds_dir, out_dir, iterations, seed,
             timeout):
    rng = random.Random(seed)
    seeds = load_seeds(seeds_dir)
    known = load_known_signatures(out_dir)
    seen = set()
    sig_counts = {}
    counts = {c: 0 for c in CLASSES}
    for i in range(iterations):
        text, mutation, origin = draw_input(rng, seeds)
        cls, cand_rc, orc_rc, diff, cand_log, orc_log = run_one(
            text, candidate, oracle, timeout, return_logs=True)
        counts[cls] += 1
        if cls in STORE:
            sig = signature(cls, cand_rc, cand_log, orc_rc, orc_log, diff)
            if sig is not None:
                sig_counts[sig] = sig_counts.get(sig, 0) + 1
            if sig is None or (sig not in seen and sig not in known):
                if sig is not None:
                    seen.add(sig)
                digest = hashlib.sha256(
                    text.encode("utf-8")).hexdigest()[:16]
                cls_dir = os.path.join(out_dir, cls)
                os.makedirs(cls_dir, exist_ok=True)
                with open(os.path.join(cls_dir, digest + ".tex"), "w") as fh:
                    fh.write(text)
                with open(os.path.join(cls_dir, digest + ".json"), "w") as fh:
                    json.dump({"iteration": i, "seed": origin,
                               "mutation": mutation,
                               "candidate_returncode": cand_rc,
                               "oracle_returncode": orc_rc,
                               "first_diff": diff,
                               "signature": sig}, fh, indent=2)
        if (i + 1) % 100 == 0:
            print("fuzz %d/%d: %s"
                  % (i + 1, iterations,
                     " ".join("%s=%d" % (c, counts[c]) for c in CLASSES)))
    if sig_counts:
        try:
            path = os.path.join(out_dir, "signatures.json")
            merged = dict(sig_counts)
            try:
                with open(path) as fh:
                    old = json.load(fh)
                if isinstance(old, dict):
                    for key, val in old.items():
                        merged[key] = merged.get(key, 0) + val
            except (OSError, ValueError):
                pass
            os.makedirs(out_dir, exist_ok=True)
            with open(path, "w") as fh:
                json.dump(merged, fh, indent=2, sort_keys=True)
        except OSError:
            pass
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
