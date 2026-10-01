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

# Classes that are pdfTeX's own crashes/hangs too, not engine-diffs
# (plus reference-nondeterministic, which is a property of the oracle,
# not a candidate finding): listed in the summary but never fail the
# night (exit 0).
BENIGN_CLASSES = ("both-crash", "both-hang", "both-flood",
                  "reference-nondeterministic")

# Wall-clock enforcement: each fuzzer subprocess runs in its own process
# group with a timeout of share*TIMEOUT_SCALE + TIMEOUT_GRACE_SECONDS
# (share = its base-proportional slice of the budget). On overrun the
# fuzzer's group is SIGSTOPped first (a frozen tree cannot spawn, so the
# snapshot below is complete), then the whole descendant tree gets
# SIGTERM, then SIGCONT so a TERM handler can run cleanup, then -- after
# KILL_AFTER_SECONDS -- the tree is frozen again (SIGSTOP the group and
# every known pid/group, re-snapshot, SIGSTOP anything new, repeating
# until a round finds no new pid, at most REFREEZE_ROUNDS rounds) and
# SIGKILLed (see _descendant_snapshot: engines started with
# start_new_session escape the fuzzer's process group, so killpg alone
# would orphan a hanging engine and leave it holding the output pipe).
# No new fuzzer starts once the budget + OVERBUDGET_GRACE_SECONDS has
# passed. Pids still alive after SIGKILL are recorded as unkilled_pids.
TIMEOUT_SCALE = 1.2
TIMEOUT_GRACE_SECONDS = 30.0
KILL_AFTER_SECONDS = 5.0
OVERBUDGET_GRACE_SECONDS = 60.0

# Rounds of `ps` listings merged into one descendant snapshot (a child
# spawned between two listings is still caught).
PS_ROUNDS = 3
PS_ROUND_DELAY_SECONDS = 0.05

# Freeze-then-resnapshot rounds after SIGCONT before the SIGKILL pass:
# each round SIGSTOPs newly found pids and snapshots again until a round
# finds no new pid. Past the cap everything known is SIGKILLed anyway
# and any pid still alive is recorded as unkilled.
REFREEZE_ROUNDS = 5


def _ps_table():
    """pid -> (ppid, pgid) from `ps`; {} when ps is unavailable."""
    try:
        proc = subprocess.run(["ps", "-axo", "pid=,ppid=,pgid="],
                              capture_output=True, text=True, timeout=10)
    except (OSError, subprocess.SubprocessError):
        return {}
    if proc.returncode != 0:
        return {}
    table = {}
    for line in (proc.stdout or "").splitlines():
        parts = line.split()
        if len(parts) != 3:
            continue
        try:
            pid, ppid, pgid = (int(parts[0]), int(parts[1]),
                               int(parts[2]))
        except ValueError:
            continue
        table[pid] = (ppid, pgid)
    return table


def _descendant_snapshot(root_pid):
    """Pids whose ppid chain leads to root_pid, plus their pgids.

    Returns (descendants, pgids). Parses `ps` repeatedly and unions the
    views so a child spawned mid-listing is still caught. Never raises;
    empty when ps is unavailable (the kill then covers only the group).
    """
    merged = {}
    for round_no in range(PS_ROUNDS):
        merged.update(_ps_table())
        if round_no < PS_ROUNDS - 1:
            time.sleep(PS_ROUND_DELAY_SECONDS)
    descendants = set()
    frontier = {root_pid}
    changed = True
    while changed:
        changed = False
        for pid, (ppid, _pgid) in merged.items():
            if ppid in frontier and pid not in frontier:
                frontier.add(pid)
                descendants.add(pid)
                changed = True
    pgids = {pid: merged[pid][1] for pid in descendants}
    return descendants, pgids


def _signal_tree(root_pid, pids, pgids, sig):
    """Signal the fuzzer group, every descendant pid, and its group."""
    try:
        os.killpg(root_pid, sig)
    except (OSError, ProcessLookupError):
        pass
    for pid in pids:
        try:
            os.kill(pid, sig)
        except (OSError, ProcessLookupError):
            pass
        try:
            os.killpg(pgids[pid], sig)
        except (KeyError, OSError, ProcessLookupError):
            pass


def _alive(pid):
    """True when pid exists (signal 0); a denied lookup counts as alive."""
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    except OSError:
        return False
    return True


def default_seed(today=None):
    """Days since 1970-01-01 in UTC: each night differs, replay with --seed."""
    today = today or datetime.datetime.now(datetime.timezone.utc).date()
    return (today - datetime.date(1970, 1, 1)).days


def load_known_findings(path):
    """Known-finding entries: [{"signature":..., "fuzzers":[...] | None}].

    The optional "fuzzers" list scopes an entry to findings from those
    fuzzers (FUZZERS names, e.g. "type1"); an entry without it is global.
    """
    try:
        with open(path) as fh:
            data = json.load(fh)
    except (OSError, ValueError):
        return []
    out = []
    for e in data:
        if not isinstance(e, dict):
            continue
        fuzzers = e.get("fuzzers")
        if fuzzers is not None:
            fuzzers = tuple(fuzzers)
        out.append({"signature": e.get("signature", ""),
                    "fuzzers": fuzzers})
    return out


def is_known(signature, patterns, fuzzer=None):
    """True when signature matches a known-finding entry.

    A pattern ending in '*' is a prefix match, else exact match. An
    entry scoped with "fuzzers" only matches findings from one of those
    fuzzers. Plain-string patterns (no scope) still work.
    """
    for pat in patterns:
        scoped = None
        if isinstance(pat, dict):
            scoped = pat.get("fuzzers")
            pat = pat.get("signature", "")
        if scoped is not None and fuzzer not in scoped:
            continue
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
               seed, timeout=None, extra_args=(), unkilled_pids=None):
    """Run one fuzzer in its own process group; return (rc, output, killed).

    killed is True when the fuzzer overran timeout and the whole
    descendant tree had to be SIGTERMed (then SIGKILLed after
    KILL_AFTER_SECONDS). unkilled_pids, when a list is given, is extended
    with any recorded tree pid still alive after SIGKILL.
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
    # Freeze the fuzzer's group BEFORE snapshotting: an engine started
    # between the snapshot and the fuzzer's death reparents to pid 1 and
    # would be missed. A stopped tree cannot spawn, so the snapshot is
    # complete; SIGTERM/SIGKILL then cover the whole set.
    try:
        os.killpg(proc.pid, signal.SIGSTOP)
    except (OSError, ProcessLookupError):
        pass
    first_pids, first_pgids = _descendant_snapshot(proc.pid)
    _signal_tree(proc.pid, first_pids, first_pgids, signal.SIGTERM)
    # Wake the tree back up: a SIGTERM sent to a STOPPED fuzzer stays
    # pending until SIGCONT, so without this the fuzzer never runs its
    # own SIGTERM cleanup and every kill waits the full
    # KILL_AFTER_SECONDS.
    _signal_tree(proc.pid, first_pids, first_pgids, signal.SIGCONT)
    try:
        out, _ = proc.communicate(timeout=KILL_AFTER_SECONDS)
        reaped = True
    except subprocess.TimeoutExpired:
        reaped = False
    # Re-freeze before the SIGKILL pass: the SIGCONT above may have
    # woken a TERM-ignoring spawner, which keeps forking detached
    # children through the grace period. SIGSTOP the fuzzer's group and
    # every known pid/group, snapshot, then SIGSTOP anything newly
    # found and snapshot again, repeating until a round finds no new
    # pid. A frozen process cannot fork, so nothing can be born between
    # the last snapshot and SIGKILL.
    all_pids = set(first_pids)
    all_pgids = dict(first_pgids)
    _signal_tree(proc.pid, all_pids, all_pgids, signal.SIGSTOP)
    for _ in range(REFREEZE_ROUNDS):
        fresh_pids, fresh_pgids = _descendant_snapshot(proc.pid)
        new = set(fresh_pids) - all_pids
        all_pids |= set(fresh_pids)
        all_pgids.update(fresh_pgids)
        if not new:
            break
        _signal_tree(proc.pid, new,
                     {p: fresh_pgids[p] for p in new}, signal.SIGSTOP)
    _signal_tree(proc.pid, all_pids, all_pgids, signal.SIGKILL)
    if not reaped:
        out, _ = proc.communicate()
    # Settle: SIGKILLed children need a beat to be reaped past zombie
    # state, and a zombie still answers signal 0 (a false survivor).
    time.sleep(0.2)
    survivors = sorted(p for p in all_pids if _alive(p))
    # Wake anything SIGKILL could not finish (already STOPped above) so
    # no frozen process is left behind, then report what survived.
    for pid in survivors:
        try:
            os.kill(pid, signal.SIGCONT)
        except (OSError, ProcessLookupError):
            pass
    if unkilled_pids is not None:
        unkilled_pids.extend(survivors)
    out = (out or "") + "\n[fuzzer %s killed: wall-clock timeout]\n" \
        % spec["name"]
    if survivors:
        out += "[fuzzer %s unkilled pids: %s]\n" \
            % (spec["name"], " ".join(str(p) for p in survivors))
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


def write_summary(out_dir, seed, budget_minutes, results, findings,
                unkilled_pids=()):
    summary = {
        "seed": seed,
        "budget_minutes": budget_minutes,
        "unkilled_pids": sorted(unkilled_pids),
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
    unkilled = []
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
                                     timeout=timeouts[name],
                                     unkilled_pids=unkilled)
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
                                     timeout=timeouts[name],
                                     unkilled_pids=unkilled)
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
                  findings, unkilled)
    if unkilled:
        failed = True
    if failed:
        return 2
    patterns = load_known_findings(args.known_findings)
    if any(not is_known(e["signature"], patterns, fuzzer=f)
           and not is_benign(e)
           for f, e in findings):
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
