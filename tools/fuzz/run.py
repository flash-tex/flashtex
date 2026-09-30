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
import signal
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


STDERR_TAIL_BYTES = 64 * 1024
PANIC_LOC_RE = re.compile(r"([^\s'\",;()]+):(\d+):\d+")


def _as_text(output):
    if isinstance(output, bytes):
        return output.decode("utf-8", "replace")
    return output or ""


def signal_name(rc):
    """Signal name for a negative return code ("SIGABRT"), else None."""
    if rc is None or rc >= 0:
        return None
    try:
        return signal.Signals(-rc).name
    except ValueError:
        return "SIG%d" % (-rc,)


def panic_location(log):
    """Panic site as "<file>:<line>", or None when absent.

    Parses "panicked at <file>:<line>:<col>", dropping the column and any
    thread id ("thread 'main' (123) panicked at ..." still yields just
    the file and line), so a new panic site gets a new signature.
    """
    text = _as_text(log)
    if "panicked at" not in text:
        return None
    tail = text.split("panicked at", 1)[1]
    for scope in (tail.split("\n", 1)[0], tail):
        matches = PANIC_LOC_RE.findall(scope)
        if matches:
            path, line = matches[-1]
            return "%s:%s" % (path.strip("'\""), line)
    return None


def first_key_line(text):
    """First non-empty line with every digit replaced by N (max 200)."""
    for line in _as_text(text).splitlines():
        line = line.strip()
        if line:
            return re.sub(r"\d", "N", line[:200])
    return ""


def crash_signature(rc, log, stderr=""):
    """Dedupe signature for a crash (no class prefix).

    Panics key on the panic site ("panic:<file>:<line>"); signals key on
    the signal name plus the first stderr line ("signal:SIGABRT:<line>"),
    so different abort causes get different signatures. stderr (from a
    direct re-run, see crash_stderr) wins over the transcript log.
    """
    loc = panic_location(stderr) or panic_location(log)
    if loc is not None:
        return "panic:" + loc
    if rc is not None and rc < 0:
        first = first_key_line(stderr or log)
        if first:
            return "signal:%s:%s" % (signal_name(rc), first)
        return "signal:%s" % (signal_name(rc),)
    return "exit:%s" % (rc,)


def crash_stderr(text, binary, extra_env, timeout, fmt=None):
    """Re-run text directly on binary; return the last 64 KiB of stderr.

    capture() returns the transcript log and the return code but not the
    engine's stderr, so a crash signature is built from a direct re-run
    of the same input: same args and environment (pinned plus extra_env),
    its own temp dir, the same per-engine timeout, stdout discarded.
    Returns "" when the re-run fails to start or times out.
    """
    work = tempfile.mkdtemp(prefix="fuzz-stderr-")
    try:
        shutil.copy(lockstep_run.PRELUDE, os.path.join(work, "prelude.tex"))
        tex_path = os.path.join(work, "fuzz.tex")
        with open(tex_path, "w") as fh:
            fh.write(text)
        env = lockstep_run.pinned_env()
        if extra_env:
            env.update(extra_env)
        if fmt is None:
            args = (list(lockstep_run.ENGINE_ARGS)
                    + list(lockstep_run.ENGINE_SHELL_FLAGS))
        else:
            args = ([a for a in lockstep_run.ENGINE_ARGS
                     if a not in ("-ini", "-etex")]
                    + list(lockstep_run.ENGINE_SHELL_FLAGS)
                    + ["-fmt=" + fmt])
        argv0 = lockstep_run.engine_link(binary, work)
        try:
            proc = subprocess.Popen(
                [argv0] + args + [tex_path], cwd=work, env=env,
                stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                stderr=subprocess.PIPE, start_new_session=True)
            try:
                _, err = proc.communicate(timeout=timeout)
            except subprocess.TimeoutExpired:
                try:
                    os.killpg(proc.pid, signal.SIGKILL)
                except (OSError, ProcessLookupError):
                    pass
                _, err = proc.communicate()
                return ""
        except OSError:
            return ""
        return _as_text(err or b"")[-STDERR_TAIL_BYTES:]
    finally:
        shutil.rmtree(work, ignore_errors=True)


def signature(cls, cand_rc, cand_log, orc_rc, orc_log, diff,
              cand_stderr="", orc_stderr=""):
    """Dedupe signature for a non-equal result, or None when it has none."""
    if cls == "candidate-crash":
        return crash_signature(cand_rc, cand_log, cand_stderr)
    if cls == "oracle-crash":
        return crash_signature(orc_rc, orc_log, orc_stderr)
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
            cand_err, orc_err, panic_loc = "", "", None
            if cls == "candidate-crash":
                cand_err = crash_stderr(text, candidate, candidate_env(),
                                        timeout)
                panic_loc = (panic_location(cand_err)
                             or panic_location(cand_log))
            elif cls == "oracle-crash":
                orc_err = crash_stderr(text, oracle, None, timeout)
                panic_loc = (panic_location(orc_err)
                             or panic_location(orc_log))
            sig = signature(cls, cand_rc, cand_log, orc_rc, orc_log, diff,
                            cand_stderr=cand_err, orc_stderr=orc_err)
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
                               "panic_location": panic_loc,
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
