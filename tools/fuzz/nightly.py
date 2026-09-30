#!/usr/bin/env python3
# MIT License. Nightly driver: run every T6 fuzzer inside a wall-clock
# budget, then summarise findings. Python 3 standard library only.
"""Run all T6 fuzzers (run, docgen, and the five parser fuzzers) one after
another as subprocesses, sizing iteration counts so the total stays inside
--budget-minutes. Writes OUT/summary.json and OUT/summary.md.

Exit codes: 0 = no findings, every finding is in known-findings.json,
or every unknown finding is known-benign (both-crash/both-hang/both-flood);
1 = at least one finding is not known; 2 = harness failure (including a
fuzzer killed for overrunning its wall-clock timeout).
"""
import argparse
import datetime
import json
import os
import re
import signal
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))

# Seconds of wall clock kept back for summary writing and process startup.
RESERVE_SECONDS = 20.0
# Iterations run up front per fuzzer to estimate seconds per iteration.
PROBE_ITERATIONS = 20

# Per-fuzzer constants. timeout: per-engine timeout passed to the fuzzer.
# base: target iterations when the budget allows; scaled down so the sum
# of base*seconds-per-iteration fits the budget. offset: added to --seed
# so each fuzzer draws a different stream (the follow-up run uses +1).
FUZZERS = (
    {"name": "run", "script": "run.py", "oracle": True, "seeds": True,
     "timeout": 10.0, "base": 400, "offset": 0},
    {"name": "docgen", "script": "docgen.py", "oracle": True, "seeds": False,
     "timeout": 30.0, "base": 150, "offset": 100000},
    {"name": "tfm", "script": "parsers/tfm.py", "oracle": False,
     "seeds": False, "timeout": 10.0, "base": 300, "offset": 200000},
    {"name": "type1", "script": "parsers/type1.py", "oracle": False,
     "seeds": False, "timeout": 10.0, "base": 300, "offset": 300000},
    {"name": "png", "script": "parsers/png.py", "oracle": False,
     "seeds": False, "timeout": 10.0, "base": 300, "offset": 400000},
    {"name": "jpeg", "script": "parsers/jpeg.py", "oracle": False,
     "seeds": False, "timeout": 15.0, "base": 300, "offset": 500000},
    {"name": "pdfinc", "script": "parsers/pdfinc.py", "oracle": False,
     "seeds": False, "timeout": 10.0, "base": 300, "offset": 600000},
)

DONE_RE = re.compile(r"^done: (\d+) iterations:(.*)$")
COUNT_RE = re.compile(r"(\S+)=(\d+)")

# Classes that are pdfTeX's own crashes/hangs too, not engine-diffs:
# listed in the summary but never fail the night (exit 0).
BENIGN_CLASSES = ("both-crash", "both-hang", "both-flood")

# Wall-clock enforcement: each fuzzer subprocess runs in its own process
# group with a timeout of share*TIMEOUT_SCALE + TIMEOUT_GRACE_SECONDS
# (share = its base-proportional slice of the budget). On overrun the
# whole group gets SIGTERM, then SIGKILL after KILL_AFTER_SECONDS. No new
# fuzzer starts once the budget + OVERBUDGET_GRACE_SECONDS has passed.
TIMEOUT_SCALE = 1.2
TIMEOUT_GRACE_SECONDS = 30.0
KILL_AFTER_SECONDS = 5.0
OVERBUDGET_GRACE_SECONDS = 60.0


def default_seed(today=None):
    """Days since 1970-01-01 in UTC: each night differs, replay with --seed."""
    today = today or datetime.datetime.now(datetime.timezone.utc).date()
    return (today - datetime.date(1970, 1, 1)).days


def load_known_findings(path):
    try:
        with open(path) as fh:
            data = json.load(fh)
    except (OSError, ValueError):
        return []
    return [e.get("signature", "") for e in data if isinstance(e, dict)]


def is_known(signature, patterns):
    """A pattern ending in '*' is a prefix match, else exact match."""
    for pat in patterns:
        if pat.endswith("*"):
            if signature.startswith(pat[:-1]):
                return True
        elif signature == pat:
            return True
    return False


def parse_final_line(text):
    """Parse a fuzzer's final 'done: N iterations: k=v ...' line."""
    for line in reversed(text.splitlines()):
        m = DONE_RE.match(line.strip())
        if m:
            counts = {k: int(v) for k, v in COUNT_RE.findall(m.group(2))}
            return m.group(0), int(m.group(1)), counts
    return "", 0, {}


def size_runs(budget_seconds, table, rates):
    """Scale each fuzzer's base target so sum(total*rate) fits the budget.

    rates maps fuzzer name -> measured seconds per iteration. Returns
    name -> total iteration count (probe iterations included).
    """
    total_cost = sum(f["base"] * max(rates.get(f["name"], 0.0), 1e-9)
                     for f in table)
    room = max(budget_seconds - RESERVE_SECONDS, 0.0)
    scale = min(1.0, room / total_cost) if total_cost > 0 else 1.0
    return {f["name"]: max(int(f["base"] * scale), 0) for f in table}


def fuzzer_timeouts(budget_seconds, table):
    """Wall-clock timeout per fuzzer: budget share * 1.2 + 30 s."""
    total = sum(f["base"] for f in table) or 1
    return {f["name"]: budget_seconds * f["base"] / total * TIMEOUT_SCALE
            + TIMEOUT_GRACE_SECONDS for f in table}


def run_fuzzer(spec, candidate, oracle, seeds_dir, out_dir, iterations,
               seed, timeout=None, extra_args=()):
    """Run one fuzzer in its own process group; return (rc, output, killed).

    killed is True when the fuzzer overran timeout and the whole process
    group had to be SIGTERMed (then SIGKILLed after KILL_AFTER_SECONDS).
    """
    cmd = [sys.executable, os.path.join(HERE, spec["script"]),
           "--candidate", candidate, "--out", out_dir,
           "--iterations", str(iterations), "--seed", str(seed),
           "--timeout", str(spec["timeout"])] + list(extra_args)
    if spec["oracle"]:
        cmd += ["--oracle", oracle]
    if spec["seeds"]:
        cmd += ["--seeds", seeds_dir]
    try:
        proc = subprocess.Popen(cmd, stdout=subprocess.PIPE,
                                stderr=subprocess.STDOUT, text=True,
                                start_new_session=True)
    except OSError as exc:
        return 2, "error: cannot start %s: %s\n" % (spec["name"], exc), False
    try:
        out, _ = proc.communicate(timeout=timeout)
        return proc.returncode, out or "", False
    except subprocess.TimeoutExpired:
        pass
    try:
        os.killpg(proc.pid, signal.SIGTERM)
    except (OSError, ProcessLookupError):
        pass
    try:
        out, _ = proc.communicate(timeout=KILL_AFTER_SECONDS)
    except subprocess.TimeoutExpired:
        try:
            os.killpg(proc.pid, signal.SIGKILL)
        except (OSError, ProcessLookupError):
            pass
        out, _ = proc.communicate()
    out = (out or "") + "\n[fuzzer %s killed: wall-clock timeout]\n" \
        % spec["name"]
    return proc.returncode, out, True


def finding_class(entry):
    """Stored class of a finding: the artifact's directory name."""
    return os.path.basename(os.path.dirname(entry["path"]))


def is_benign(entry):
    """Both-engines findings are pdfTeX's own fault: known-benign."""
    return finding_class(entry) in BENIGN_CLASSES


def collect_findings(fuzzer_out):
    """Every stored sidecar .json (minus signatures.json) is one finding."""
    findings = []
    for root, _dirs, files in os.walk(fuzzer_out):
        for name in sorted(files):
            if not name.endswith(".json") or name == "signatures.json":
                continue
            path = os.path.join(root, name)
            try:
                with open(path) as fh:
                    info = json.load(fh)
            except (OSError, ValueError):
                info = {}
            base = os.path.splitext(path)[0]
            artifact = base + ".json"
            for ext in (".tex", ".tfm", ".pfb", ".png", ".jpg",
                        ".jpeg", ".pdf"):
                if os.path.isfile(base + ext):
                    artifact = base + ext
                    break
            sig = info.get("signature") if isinstance(info, dict) else None
            findings.append({"path": artifact, "sidecar": path,
                             "signature": sig or "<no-signature>",
                             "info": info if isinstance(info, dict) else {}})
    return findings


def write_summary(out_dir, seed, budget_minutes, results, findings):
    summary = {
        "seed": seed,
        "budget_minutes": budget_minutes,
        "fuzzers": {
            name: {"iterations": r["iterations"],
                   "class_counts": r["counts"],
                   "new_signatures": r["new_signatures"],
                   "elapsed_seconds": round(r["elapsed"], 1),
                   "seed": r["seed"],
                   "out": r["out"],
                   "status": r.get("status", "ok"),
                   "final_line": r["final_line"]}
            for name, r in results.items()
        },
        "findings": [{"fuzzer": f, "signature": e["signature"],
                      "path": e["path"]} for f, e in findings],
    }
    with open(os.path.join(out_dir, "summary.json"), "w") as fh:
        json.dump(summary, fh, indent=2, sort_keys=True)
    lines = ["# Nightly fuzz summary", "",
             "| fuzzer | iterations | counts | new signatures | "
             "elapsed (s) | seed | status |",
             "| --- | --- | --- | --- | --- | --- | --- |"]
    for name, r in results.items():
        counts = " ".join("%s=%d" % kv
                          for kv in sorted(r["counts"].items()))
        lines.append("| %s | %d | %s | %d | %.1f | %d | %s |"
                     % (name, r["iterations"], counts or "-",
                        r["new_signatures"], r["elapsed"], r["seed"],
                        r.get("status", "ok")))
    lines += ["", "## Findings (%d)" % len(findings), ""]
    if not findings:
        lines.append("No findings.")
    for f, e in findings:
        lines += ["### %s / %s" % (f, e["signature"]), "",
                  "artifact: `%s`" % e["path"], "",
                  "```json",
                  json.dumps(e["info"], indent=2, sort_keys=True),
                  "```", ""]
    with open(os.path.join(out_dir, "summary.md"), "w") as fh:
        fh.write("\n".join(lines))


def main(argv=None):
    ap = argparse.ArgumentParser(description="nightly T6 fuzz driver")
    ap.add_argument("--candidate", required=True)
    ap.add_argument("--oracle", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, default=default_seed())
    ap.add_argument("--budget-minutes", type=float, default=60.0)
    ap.add_argument("--lockstep-cases", default=os.path.join(
        HERE, "..", "lockstep", "cases"))
    ap.add_argument("--known-findings", default=os.path.join(
        HERE, "known-findings.json"))
    args = ap.parse_args(argv)

    budget = args.budget_minutes * 60.0
    start = time.monotonic()
    os.makedirs(args.out, exist_ok=True)
    table = [dict(f) for f in FUZZERS]
    probes, rates, results = {}, {}, {}
    failed = False
    killed_any = False
    timeouts = fuzzer_timeouts(budget, table)

    def over_budget():
        return (time.monotonic() - start
                > budget + OVERBUDGET_GRACE_SECONDS)

    def skip_result(name, sub, seed):
        return {"iterations": 0, "counts": {}, "elapsed": 0.0,
                "seed": seed, "out": sub, "final_line": "", "rc": None,
                "new_signatures": 0, "status": "timed-out"}

    # Phase 1: probe each fuzzer to estimate seconds per iteration.
    for spec in table:
        name = spec["name"]
        sub = os.path.join(args.out, name)
        os.makedirs(sub, exist_ok=True)
        seed = args.seed + spec["offset"]
        if over_budget():
            # Budget exhausted: record without starting anything new.
            results[name] = skip_result(name, sub, seed)
            continue
        n = min(PROBE_ITERATIONS, spec["base"])
        t0 = time.monotonic()
        rc, out, killed = run_fuzzer(spec, args.candidate, args.oracle,
                                     args.lockstep_cases, sub, n, seed,
                                     timeout=timeouts[name])
        dt = max(time.monotonic() - t0, 1e-9)
        line, done, counts = parse_final_line(out)
        probes[name] = done
        rates[name] = dt / max(done, 1)
        results[name] = {"iterations": done, "counts": counts,
                         "elapsed": dt, "seed": seed, "out": sub,
                         "final_line": line, "rc": rc,
                         "new_signatures": 0,
                         "status": "timed-out" if killed else "ok"}
        if killed:
            killed_any = True
            failed = True
        elif rc != 0:
            failed = True

    # Phase 2: size the rest from the measured rates, then run it.
    totals = size_runs(budget - (time.monotonic() - start), table, rates)
    for spec in table:
        name = spec["name"]
        if name not in results:
            continue
        if results[name].get("status") == "timed-out" and \
                results[name]["iterations"] == 0:
            continue
        if over_budget():
            results[name]["status"] = "timed-out"
            continue
        extra = totals[name] - results[name]["iterations"]
        if extra <= 0:
            continue
        # Refit: never plan past the remaining budget.
        remaining = budget - (time.monotonic() - start) - RESERVE_SECONDS
        extra = max(min(extra, int(remaining / rates[name])), 0)
        if extra <= 0:
            continue
        sub = results[name]["out"]
        t0 = time.monotonic()
        rc, out, killed = run_fuzzer(spec, args.candidate, args.oracle,
                                     args.lockstep_cases, sub,
                                     extra, results[name]["seed"] + 1,
                                     timeout=timeouts[name])
        dt = time.monotonic() - t0
        line, done, counts = parse_final_line(out)
        for key, val in counts.items():
            results[name]["counts"][key] = \
                results[name]["counts"].get(key, 0) + val
        results[name]["iterations"] += done
        results[name]["elapsed"] += dt
        if line:
            results[name]["final_line"] = line
        if killed:
            results[name]["status"] = "timed-out"
            killed_any = True
            failed = True
        elif rc != 0:
            failed = True

    findings = []
    for spec in table:
        name = spec["name"]
        found = collect_findings(results[name]["out"])
        sigs = {e["signature"] for e in found}
        results[name]["new_signatures"] = len(sigs)
        findings += [(name, e) for e in found]

    write_summary(args.out, args.seed, args.budget_minutes, results,
                  findings)
    if failed:
        return 2
    patterns = load_known_findings(args.known_findings)
    if any(not is_known(e["signature"], patterns) and not is_benign(e)
           for _, e in findings):
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
