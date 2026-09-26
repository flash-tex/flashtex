#!/usr/bin/env python3
"""Simulated FlashTeX agent for the Beads soak test (trial repo only).

Cycle: pick a ready `soak` task at random -> scripts/beads/claim (claim rule) ->
work (sleep) -> append progress notes -> re-read (still mine?) -> close ->
sometimes create a follow-up task. The machine sync loop does all pulls and
background pushes. The agent pushes only inside `claim`.

Every event goes to --log as JSON Lines, for tests/soak_verify.py.
"""
import argparse
import json
import os
import random
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
BEADS = os.path.dirname(HERE)
BD = os.path.join(BEADS, "bd")
CLAIM = os.path.join(BEADS, "claim")


def sh(cmd, repo):
    t = time.time()
    p = subprocess.run(cmd, cwd=repo, capture_output=True, text=True)
    return p.returncode, p.stdout, p.stderr, round(time.time() - t, 2)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--repo", required=True)
    ap.add_argument("--actor", required=True)
    ap.add_argument("--duration", type=int, default=1800)
    ap.add_argument("--log", required=True)
    ap.add_argument("--work-min", type=int, default=60)
    ap.add_argument("--work-max", type=int, default=150)
    ap.add_argument("--followup-p", type=float, default=0.35)
    ap.add_argument("--label", default="soak")
    a = ap.parse_args()
    rnd = random.Random(f"{a.actor}-{os.getpid()}")
    end = time.time() + a.duration
    logf = open(a.log, "a")

    def log(ev, **kw):
        kw.update(ev=ev, actor=a.actor, utc=time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), t=round(time.time(), 2))
        logf.write(json.dumps(kw) + "\n")
        logf.flush()

    time.sleep(rnd.uniform(0, 20))
    while time.time() < end:
        rc, out, err, dur = sh([BD, "ready", "--json", "-l", a.label, "-n", "40"], a.repo)
        if rc != 0:
            log("ready_error", rc=rc, err=err.strip()[-200:], dur=dur)
            time.sleep(10)
            continue
        try:
            ready = [i for i in json.loads(out) if not i.get("assignee")]
        except ValueError:
            ready = []
        if not ready:
            log("idle")
            time.sleep(15)
            continue
        task = rnd.choice(ready)["id"]
        rc, out, err, dur = sh([CLAIM, task, "--actor", a.actor, "--no-pull", "--json"], a.repo)
        try:
            res = json.loads(out.strip().splitlines()[-1])
        except (ValueError, IndexError):
            res = {"result": "claim_output_unparsed", "raw": (out + err)[-200:]}
        log("claim", id=task, rc=rc, dur=dur, result=res.get("result"), detail=res)
        if rc != 0:
            time.sleep(rnd.uniform(3, 10))
            continue
        time.sleep(rnd.uniform(a.work_min, a.work_max) / 2)
        rc, _, err, dur = sh([BD, "update", task, "--append-notes", f"progress by {a.actor} at {time.time():.0f}", "--actor", a.actor], a.repo)
        log("progress", id=task, rc=rc, dur=dur, err=err.strip()[-120:] if rc else "")
        time.sleep(rnd.uniform(a.work_min, a.work_max) / 2)
        rc, out, err, dur = sh([BD, "show", task, "--json"], a.repo)
        cur = {}
        try:
            d = json.loads(out)
            cur = d[0] if isinstance(d, list) else d
        except ValueError:
            pass
        if cur.get("assignee") != a.actor or cur.get("status") != "in_progress":
            log("abandon_not_mine", id=task, assignee=cur.get("assignee"), status=cur.get("status"))
            continue
        rc, _, err, dur = sh([BD, "close", task, "-r", f"done by {a.actor}", "--actor", a.actor], a.repo)
        log("close", id=task, rc=rc, dur=dur, err=err.strip()[-160:] if rc else "")
        if rc == 0 and rnd.random() < a.followup_p:
            rc, out, err, dur = sh([BD, "create", f"follow-up of {task} by {a.actor}", "-t", "task", "-l", a.label,
                                    "--deps", f"discovered-from:{task}", "--silent", "--actor", a.actor], a.repo)
            log("create", id=out.strip(), parent=task, rc=rc, dur=dur, err=err.strip()[-160:] if rc else "")
    log("exit")


if __name__ == "__main__":
    sys.exit(main())
