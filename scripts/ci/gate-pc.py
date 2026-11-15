#!/usr/bin/env python3
"""gate-pc.py: the merge-queue gate on the NixOS PC, with a hosted fallback (ci.yml).

When plan routes a gate run to the PC (`gate_route=pc`: a merge group with no fork
pull request, or a manual gate run that asks for it), the gate's Linux legs run as
`*-pc` jobs on the PC's existing runners, and `gate-pc` (a GitHub-hosted job) runs

    gate-pc.py supervise

which watches those jobs through the Actions API and ends in one of three ways:

  success   every expected PC job ran and passed: `CI required` takes that as the
            gate's result for those legs;
  failure   a PC job failed (or ran but did not pass): exit 1, so `CI required`
            fails, exactly as a failed hosted leg would;
  fallback  after GATE_PC_WAIT minutes at least one PC job had still not been
            picked up (the PC is busy behind a long job, or offline): exit 0 with
            `gate=fallback`, and the hosted jobs (build, quick, checks,
            engine-parity-hosted) run the whole gate instead.

A PC job that a runner picks up only after a fallback has nothing left to decide;

    gate-pc.py stood-down

(its first step) prints `GATE_STOOD_DOWN=1` for $GITHUB_ENV when `gate-pc` has
already finished, and every later step of the job is skipped, so a stale job holds
a PC runner for seconds rather than a whole gate.

Environment: GH_TOKEN, GITHUB_REPOSITORY, GITHUB_RUN_ID, GITHUB_RUN_ATTEMPT,
GITHUB_OUTPUT (supervise), and for supervise PC_EXPECTED (how many PC jobs plan
scheduled), GATE_PC_WAIT (minutes), POLL_S (default 20).
"""
import json
import os
import subprocess
import sys
import time

PC_LABEL = "flashtex-linux"
SUPERVISOR = "gate on the NixOS PC"  # gate-pc's name in ci.yml (a prefix)
ENGINE_PREFIX = "engine parity (NixOS"


def jobs():
    repo = os.environ["GITHUB_REPOSITORY"]
    run = os.environ["GITHUB_RUN_ID"]
    attempt = os.environ.get("GITHUB_RUN_ATTEMPT", "1")
    path = f"repos/{repo}/actions/runs/{run}/attempts/{attempt}/jobs?per_page=100"
    for i in range(5):
        try:
            out = subprocess.run(["gh", "api", "--paginate", path, "--jq", ".jobs[]"],
                                 check=True, capture_output=True, text=True).stdout
            return [json.loads(line) for line in out.splitlines() if line.strip()]
        except subprocess.CalledProcessError as e:
            print(f"gate-pc: jobs API failed ({e.stderr.strip()[:200]}); retry {i + 1}/5",
                  file=sys.stderr)
            time.sleep(10)
    raise SystemExit("gate-pc: the jobs API failed five times")


def pc_jobs(all_jobs):
    return [j for j in all_jobs
            if PC_LABEL in (j.get("labels") or []) and j.get("conclusion") != "skipped"]


def started(j):
    return j.get("status") in ("in_progress", "completed") and j.get("runner_name")


def output(**kv):
    with open(os.environ["GITHUB_OUTPUT"], "a") as fh:
        for k, v in kv.items():
            fh.write(f"{k}={v}\n")
    print(" ".join(f"{k}={v}" for k, v in kv.items()))


def summary(text):
    path = os.environ.get("GITHUB_STEP_SUMMARY")
    if path:
        with open(path, "a") as fh:
            fh.write(text + "\n")


def table(pc):
    rows = "\n".join(f"| {j['name']} | {j['status']} | {j.get('conclusion') or ''} | "
                     f"{j.get('runner_name') or ''} |" for j in sorted(pc, key=lambda j: j["name"]))
    return "| PC job | status | conclusion | runner |\n|---|---|---|---|\n" + rows


def supervise():
    expected = int(os.environ["PC_EXPECTED"])
    wait_s = float(os.environ.get("GATE_PC_WAIT", "5")) * 60
    poll = float(os.environ.get("POLL_S", "20"))
    t0 = time.monotonic()
    print(f"gate-pc: {expected} PC jobs expected; fallback if any is not picked up "
          f"within {wait_s / 60:g} min")
    while True:
        pc = pc_jobs(jobs())
        elapsed = time.monotonic() - t0
        failed = [j for j in pc if j["status"] == "completed" and j.get("conclusion") != "success"]
        if failed:
            summary("### Gate on the NixOS PC: failed\n\n" + table(pc))
            output(gate="failure", engine="failure" if any(
                j["name"].startswith(ENGINE_PREFIX) for j in failed) else "unknown")
            for j in failed:
                print(f"::error::{j['name']}: {j.get('conclusion')}")
            return 1
        if len(pc) >= expected and all(j["status"] == "completed" for j in pc):
            engine = [j for j in pc if j["name"].startswith(ENGINE_PREFIX)]
            summary("### Gate on the NixOS PC: passed\n\n" + table(pc))
            output(gate="success", engine="success" if engine else "none")
            return 0
        waiting = [j for j in pc if not started(j)]
        if elapsed >= wait_s and (waiting or len(pc) < expected):
            names = ", ".join(j["name"] for j in waiting) or f"{expected - len(pc)} not yet created"
            msg = (f"gate-pc: after {elapsed / 60:.1f} min not picked up on the PC: {names}; "
                   "the gate falls back to GitHub-hosted runners")
            print(f"::warning::{msg}")
            summary(f"### Gate on the NixOS PC: fallback\n\n{msg}\n\n" + table(pc))
            output(gate="fallback", engine="fallback")
            return 0
        time.sleep(poll)


def stood_down():
    # Never fatal: a failed lookup means "carry on", which costs only PC time.
    try:
        all_jobs = jobs()
    except SystemExit as e:
        print(f"gate-pc: {e}; not standing down", file=sys.stderr)
        return 0
    for j in all_jobs:
        if (j["name"].startswith(SUPERVISOR) and j["status"] == "completed"
                and j.get("conclusion") != "skipped"):
            print(f"gate-pc: '{j['name']}' already finished ({j.get('conclusion')}): "
                  "this PC job stands down", file=sys.stderr)
            print("GATE_STOOD_DOWN=1")
            return 0
    return 0


if __name__ == "__main__":
    cmd = sys.argv[1] if len(sys.argv) > 1 else ""
    if cmd == "supervise":
        sys.exit(supervise())
    if cmd == "stood-down":
        sys.exit(stood_down())
    sys.exit(__doc__)
