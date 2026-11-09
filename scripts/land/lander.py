#!/usr/bin/env python3
"""One lander process for every PR (replaces the per-PR shell landers).

Usage:
  lander.py start --tsv FILE [--dir DIR] [--dry-run] [--interval 300]
      Detach (nohup + new session, like `nohup setsid`), write DIR/lander.pid,
      log to DIR/lander.log. Refuses if the PID file names a live process.
  lander.py run   --tsv FILE [--dir DIR] [--dry-run] [--once]
      Same loop in the foreground (what `start` executes).
  lander.py stop  [--dir DIR]
      SIGTERM the PID in DIR/lander.pid (only that PID; never pkill).
  lander.py status [--dir DIR]
      Print DIR/state.json (the last cycle's decision per PR).

TSV columns (tab-separated, '#' starts a comment line):
  pr  pinned_head_sha  needs  deadline
  needs:    none | sweeps:<name>[,<name>...] | after:<PR>   ('+' joins several)
  deadline: ISO-8601 UTC (2026-11-10T03:45:00Z) or '-' for none.

Every --interval seconds (default 300) it makes ONE GraphQL query covering
every PR (head oid, `CI required` and sweep check runs on the head, merge-queue
entry, auto-merge, recent comments). If GraphQL fails it falls back to REST,
which can observe but cannot enable auto-merge (that is GraphQL-only), so an
enable found during REST fallback waits for the next GraphQL cycle.

Rate limits and errors are always "retry later", never a decision. A PR is
stopped (and only that PR) on: a head that moved off the pin, a red
`CI required`, or a comment starting with "STOP-LANDER" by the Commander login.
`after:<N>` enables a PR only once #N has merged. Enabling uses GraphQL
enablePullRequestAutoMerge with expectedHeadOid = the pin, so GitHub itself
refuses if the head moved in between. The lander never posts comments.
"""
from __future__ import annotations

import argparse
import dataclasses
import datetime as dt
import json
import os
import signal
import subprocess
import sys
import time
from typing import Any

REPO_OWNER = "flash-tex"
REPO_NAME = "flashtex"
REQUIRED_CHECK = "CI required"
STOP_PREFIX = "STOP-LANDER"
DEFAULT_DIR = os.path.expanduser("~/.cache/flashtex-agents/lander")
RED = {"FAILURE", "TIMED_OUT", "STARTUP_FAILURE", "ACTION_REQUIRED"}
# CANCELLED / SKIPPED / STALE / NEUTRAL are "not run": wait (a cancelled
# `CI required` is rerun by hand; it is neither a pass nor a fail).

# Decision kinds.
WAIT, ENABLE, DONE, STOP, EXPIRED = "wait", "enable", "done", "stop", "expired"
TERMINAL = {DONE, STOP, EXPIRED}


@dataclasses.dataclass
class Row:
    pr: int
    sha: str
    after: int | None = None
    sweeps: tuple[str, ...] = ()
    deadline: dt.datetime | None = None


@dataclasses.dataclass
class Decision:
    kind: str
    reason: str


def parse_tsv(text: str) -> list[Row]:
    rows: list[Row] = []
    for n, raw in enumerate(text.splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        cols = line.split("\t")
        if len(cols) < 2 or cols[0].lower() == "pr":
            continue  # header or junk
        pr, sha = int(cols[0]), cols[1].strip()
        if len(sha) != 40:
            raise ValueError(f"line {n}: pinned sha must be 40 hex chars: {sha!r}")
        row = Row(pr=pr, sha=sha)
        needs = cols[2].strip() if len(cols) > 2 else "none"
        for need in filter(None, needs.split("+")):
            if need == "none":
                continue
            kind, _, val = need.partition(":")
            if kind == "after":
                row.after = int(val)
            elif kind == "sweeps":
                row.sweeps = tuple(s for s in val.split(",") if s)
            else:
                raise ValueError(f"line {n}: unknown need {need!r}")
        dl = cols[3].strip() if len(cols) > 3 else "-"
        if dl and dl != "-":
            row.deadline = dt.datetime.fromisoformat(dl.replace("Z", "+00:00"))
        rows.append(row)
    return rows


# ---------------------------------------------------------------- snapshots
# A snapshot is the normalised view of one PR, built from either GraphQL or
# REST, so decide() does not care which one answered:
#   {"merged": bool, "state": "OPEN"|"CLOSED"|"MERGED", "head": sha,
#    "base": "main", "auto_merge": bool, "in_queue": bool|None, "id": node id,
#    "checks": [{"name": str, "status": str, "conclusion": str|None}],
#    "comments": [{"login": str, "body": str}]}
# A missing snapshot (None) means "no data this cycle".


def decide(row: Row, snap: dict | None, snaps: dict[int, dict | None],
           commander: str, now: dt.datetime) -> Decision:
    """Pure decision for one PR. Errors/unknowns are always WAIT."""
    if snap is None:
        return Decision(WAIT, "no data this cycle (error or rate limit); retry later")
    if snap.get("merged"):
        return Decision(DONE, "merged")
    for c in snap.get("comments", []):
        if c.get("login") == commander and c.get("body", "").lstrip().startswith(STOP_PREFIX):
            return Decision(STOP, f"{STOP_PREFIX} comment by {commander}")
    head = snap.get("head")
    if head and head != row.sha:
        return Decision(STOP, f"head moved to {head} (pinned {row.sha})")
    ci = [c for c in snap.get("checks", []) if c.get("name") == REQUIRED_CHECK]
    red = [c for c in ci if (c.get("conclusion") or "").upper() in RED]
    if red:
        return Decision(STOP, f"{REQUIRED_CHECK} red: {red[0]['conclusion']}")
    if snap.get("auto_merge") or snap.get("in_queue"):
        return Decision(DONE, "auto-merge already on / in merge queue")
    if snap.get("state") != "OPEN":
        return Decision(WAIT, f"PR state {snap.get('state')}; nothing to enable")
    if row.deadline and now >= row.deadline:
        return Decision(EXPIRED, f"deadline {row.deadline.isoformat()} passed")
    if row.after is not None:
        pre = snaps.get(row.after)
        if pre is None:
            return Decision(WAIT, f"no data for predecessor #{row.after}; retry later")
        if not pre.get("merged"):
            return Decision(WAIT, f"waiting for #{row.after} to merge")
    if snap.get("base") != "main":
        return Decision(WAIT, f"base is {snap.get('base')!r}, not main (auto-merge "
                        "on a feature base merges at once)")
    if not ci:
        return Decision(WAIT, f"{REQUIRED_CHECK} not reported on the pinned head yet")
    if not all((c.get("conclusion") or "").upper() == "SUCCESS" for c in ci):
        states = ",".join(sorted({(c.get("conclusion") or c.get("status") or "?").upper() for c in ci}))
        return Decision(WAIT, f"{REQUIRED_CHECK} {states}")
    for sweep in row.sweeps:
        runs = [c for c in snap.get("checks", [])
                if sweep in c.get("name", "") or sweep in c.get("workflow", "")]
        if not runs:
            return Decision(WAIT, f"sweep {sweep!r} not run on the pinned head")
        if any((c.get("conclusion") or "").upper() in RED for c in runs):
            # A red sweep is a real finding: never enable, but it is not one of
            # the three stop conditions, so hold and keep logging.
            return Decision(WAIT, f"sweep {sweep!r} red on the pinned head; holding")
        if not all((c.get("conclusion") or "").upper() == "SUCCESS" for c in runs):
            return Decision(WAIT, f"sweep {sweep!r} not green yet")
    return Decision(ENABLE, "all conditions met")


# ---------------------------------------------------------------- GitHub I/O


class Retry(Exception):
    """Any GitHub failure: retry later, never a decision."""


def gh(args: list[str], timeout: int = 120) -> Any:
    try:
        p = subprocess.run(["gh", *args], capture_output=True, text=True, timeout=timeout)
    except (OSError, subprocess.TimeoutExpired) as e:
        raise Retry(f"gh failed to run: {e}") from e
    if p.returncode != 0:
        raise Retry(f"gh exit {p.returncode}: {p.stderr.strip()[:300]}")
    try:
        return json.loads(p.stdout) if p.stdout.strip() else None
    except json.JSONDecodeError as e:
        raise Retry(f"gh returned non-JSON: {e}") from e


PR_FIELDS = """
  id number state merged headRefOid baseRefName isInMergeQueue
  mergeQueueEntry { state }
  autoMergeRequest { enabledAt }
  comments(last: 50) { nodes { author { login } body } }
  commits(last: 1) { nodes { commit { oid statusCheckRollup { contexts(first: 100) {
    nodes { __typename
      ... on CheckRun { name status conclusion
            checkSuite { workflowRun { workflow { name } } } }
      ... on StatusContext { context state } } } } } } }
"""


def build_query(prs: list[int]) -> str:
    parts = [f"p{n}: pullRequest(number: {n}) {{{PR_FIELDS}}}" for n in prs]
    return ("query { rateLimit { cost remaining resetAt } "
            f"repository(owner: \"{REPO_OWNER}\", name: \"{REPO_NAME}\") {{ "
            + " ".join(parts) + " } }")


def snap_from_graphql(node: dict | None) -> dict | None:
    if not node:
        return None
    checks = []
    commits = (node.get("commits") or {}).get("nodes") or []
    if commits:
        roll = (commits[0].get("commit") or {}).get("statusCheckRollup") or {}
        for c in (roll.get("contexts") or {}).get("nodes") or []:
            if c.get("__typename") == "CheckRun":
                wf = (((c.get("checkSuite") or {}).get("workflowRun") or {})
                      .get("workflow") or {}).get("name", "")
                checks.append({"name": c.get("name", ""), "status": c.get("status"),
                               "conclusion": c.get("conclusion"), "workflow": wf})
            elif c.get("__typename") == "StatusContext":
                st = (c.get("state") or "").upper()
                checks.append({"name": c.get("context", ""), "status": st,
                               "conclusion": {"SUCCESS": "SUCCESS", "FAILURE": "FAILURE",
                                              "ERROR": "FAILURE"}.get(st), "workflow": ""})
    return {
        "id": node.get("id"),
        "merged": bool(node.get("merged")),
        "state": node.get("state"),
        "head": node.get("headRefOid"),
        "base": node.get("baseRefName"),
        "auto_merge": node.get("autoMergeRequest") is not None,
        "in_queue": bool(node.get("isInMergeQueue") or node.get("mergeQueueEntry")),
        "checks": checks,
        "comments": [{"login": (c.get("author") or {}).get("login", ""), "body": c.get("body", "")}
                     for c in (node.get("comments") or {}).get("nodes") or []],
    }


def fetch_graphql(prs: list[int]) -> tuple[dict[int, dict | None], dict]:
    data = gh(["api", "graphql", "-f", f"query={build_query(prs)}"])
    if not isinstance(data, dict) or "data" not in data or data["data"] is None:
        raise Retry(f"graphql: no data ({str(data)[:300]})")
    repo = data["data"].get("repository") or {}
    # Per-PR errors (e.g. a deleted PR) leave that alias null: "no data", not fatal.
    return ({n: snap_from_graphql(repo.get(f"p{n}")) for n in prs},
            data["data"].get("rateLimit") or {})


def fetch_rest(prs: list[int], log) -> dict[int, dict | None]:
    out: dict[int, dict | None] = {}
    base = f"repos/{REPO_OWNER}/{REPO_NAME}"
    for n in prs:
        try:
            pr = gh(["api", f"{base}/pulls/{n}"])
            sha = pr["head"]["sha"]
            runs = gh(["api", f"{base}/commits/{sha}/check-runs?per_page=100"])
            # Newest 100 comments: the API sorts ascending, so ask for the last page.
            com = gh(["api", f"{base}/issues/{n}/comments?per_page=100&page="
                      f"{max(1, -(-pr.get('comments', 0) // 100))}"])
            out[n] = {
                "id": pr.get("node_id"),
                "merged": bool(pr.get("merged")),
                "state": "MERGED" if pr.get("merged") else (pr.get("state") or "").upper(),
                "head": sha,
                "base": pr["base"]["ref"],
                "auto_merge": pr.get("auto_merge") is not None,
                "in_queue": None,  # not exposed over REST
                "checks": [{"name": r.get("name", ""), "status": (r.get("status") or "").upper(),
                            "conclusion": (r.get("conclusion") or "").upper() or None,
                            "workflow": ""} for r in (runs or {}).get("check_runs", [])],
                "comments": [{"login": (c.get("user") or {}).get("login", ""), "body": c.get("body", "")}
                             for c in com or []],
            }
        except (Retry, KeyError, TypeError) as e:
            log(f"#{n} REST: {e}; retry later")
            out[n] = None
    return out


ENABLE_MUTATION = """mutation($id: ID!, $sha: GitObjectID!) {
  enablePullRequestAutoMerge(input: {pullRequestId: $id, mergeMethod: SQUASH,
                                     expectedHeadOid: $sha}) {
    pullRequest { number autoMergeRequest { enabledAt } } } }"""


def enable_auto_merge(node_id: str, sha: str) -> None:
    data = gh(["api", "graphql", "-f", f"query={ENABLE_MUTATION}",
               "-f", f"id={node_id}", "-f", f"sha={sha}"])
    if not isinstance(data, dict) or data.get("errors") or not data.get("data"):
        raise Retry(f"enablePullRequestAutoMerge: {str(data)[:300]}")


# ---------------------------------------------------------------- loop


def run(args) -> int:
    os.makedirs(args.dir, exist_ok=True)
    logf = open(os.path.join(args.dir, "lander.log"), "a", buffering=1)

    def log(msg: str) -> None:
        logf.write(f"{dt.datetime.now(dt.timezone.utc):%Y-%m-%dT%H:%M:%SZ} {msg}\n")

    pidfile = os.path.join(args.dir, "lander.pid")
    if not args.once:
        old = read_pid(pidfile)
        if old and old != os.getpid() and alive(old):
            log(f"another lander is running (pid {old}); exiting")
            return 1
        with open(pidfile, "w") as f:
            f.write(f"{os.getpid()}\n")

    rows = parse_tsv(open(args.tsv).read())
    commander = args.commander or _login()
    log(f"start pid {os.getpid()} rows={len(rows)} dry_run={args.dry_run} "
        f"commander={commander} interval={args.interval}s tsv={args.tsv}")
    final: dict[int, Decision] = {}
    last_reason: dict[int, str] = {}
    stopping = []
    signal.signal(signal.SIGTERM, lambda *_: stopping.append(1))
    while True:
        live = [r for r in rows if r.pr not in final]
        if not live:
            log("every PR is done, stopped or expired; exiting")
            break
        watch = sorted({r.pr for r in live} | {r.after for r in live if r.after})
        source = "graphql"
        try:
            snaps, rate = fetch_graphql(watch)
            log(f"cycle graphql prs={len(watch)} cost={rate.get('cost')} "
                f"remaining={rate.get('remaining')} reset={rate.get('resetAt')}")
        except Retry as e:
            log(f"graphql: {e}; falling back to REST")
            source = "rest"
            snaps = fetch_rest(watch, log)
        now = dt.datetime.now(dt.timezone.utc)
        state = {}
        for r in live:
            d = decide(r, snaps.get(r.pr), snaps, commander, now)
            if d.kind == ENABLE:
                snap = snaps[r.pr] or {}
                if args.dry_run:
                    d = Decision(WAIT, "dry-run: would enable auto-merge")
                elif source != "graphql" or not snap.get("id"):
                    d = Decision(WAIT, "ready, but enabling needs GraphQL; retry later")
                else:
                    try:
                        enable_auto_merge(snap["id"], r.sha)
                        d = Decision(DONE, f"auto-merge enabled on {r.sha}")
                    except Retry as e:
                        d = Decision(WAIT, f"enable failed, retry later: {e}")
            if d.kind in TERMINAL:
                final[r.pr] = d
                log(f"#{r.pr} {d.kind.upper()}: {d.reason}")
            elif last_reason.get(r.pr) != d.reason:
                log(f"#{r.pr} wait: {d.reason}")
            last_reason[r.pr] = d.reason
            state[r.pr] = dataclasses.asdict(d)
        for pr, d in final.items():
            state[pr] = dataclasses.asdict(d)
        tmp = os.path.join(args.dir, "state.json.tmp")
        with open(tmp, "w") as f:
            json.dump({"at": now.isoformat(), "source": source,
                       "prs": {str(k): v for k, v in sorted(state.items())}}, f, indent=1)
        os.replace(tmp, os.path.join(args.dir, "state.json"))
        if args.once or stopping:
            break
        for _ in range(args.interval):
            if stopping:
                break
            time.sleep(1)
        if stopping:
            log("SIGTERM; exiting")
            break
    if not args.once and read_pid(pidfile) == os.getpid():
        os.remove(pidfile)
    return 0


def _login() -> str:
    try:
        p = subprocess.run(["gh", "api", "user", "-q", ".login"], capture_output=True,
                           text=True, timeout=60)
        if p.returncode == 0 and p.stdout.strip():
            return p.stdout.strip()
    except (OSError, subprocess.TimeoutExpired):
        pass
    raise SystemExit("cannot determine the Commander login; pass --commander")


def read_pid(path: str) -> int | None:
    try:
        return int(open(path).read().strip())
    except (OSError, ValueError):
        return None


def alive(pid: int) -> bool:
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False
    except PermissionError:
        return True


def start(args) -> int:
    os.makedirs(args.dir, exist_ok=True)
    pidfile = os.path.join(args.dir, "lander.pid")
    old = read_pid(pidfile)
    if old and alive(old):
        print(f"lander already running: pid {old} ({pidfile})", file=sys.stderr)
        return 1
    cmd = ["nohup", sys.executable, os.path.abspath(__file__), "run", "--tsv",
           os.path.abspath(args.tsv), "--dir", args.dir, "--interval", str(args.interval)]
    if args.dry_run:
        cmd.append("--dry-run")
    if args.commander:
        cmd += ["--commander", args.commander]
    out = open(os.path.join(args.dir, "lander.out"), "a")
    # start_new_session=True is setsid(2): the lander survives the agent's turn
    # ending (exit 144) and its process group. macOS ships no `setsid` binary.
    p = subprocess.Popen(cmd, stdin=subprocess.DEVNULL, stdout=out, stderr=out,
                         start_new_session=True, close_fds=True)
    with open(pidfile, "w") as f:
        f.write(f"{p.pid}\n")
    print(f"lander started: pid {p.pid}; log {os.path.join(args.dir, 'lander.log')}")
    return 0


def stop(args) -> int:
    pid = read_pid(os.path.join(args.dir, "lander.pid"))
    if not pid or not alive(pid):
        print("no running lander", file=sys.stderr)
        return 1
    os.kill(pid, signal.SIGTERM)
    print(f"sent SIGTERM to {pid}")
    return 0


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    for name in ("start", "run", "stop", "status"):
        sp = sub.add_parser(name)
        sp.add_argument("--dir", default=DEFAULT_DIR, help="PID, log and state dir")
        if name in ("start", "run"):
            sp.add_argument("--tsv", required=True)
            sp.add_argument("--dry-run", action="store_true", help="never mutate")
            sp.add_argument("--interval", type=int, default=300)
            sp.add_argument("--commander", help="login whose STOP-LANDER comments count "
                            "(default: the authenticated gh user)")
        if name == "run":
            sp.add_argument("--once", action="store_true", help="one cycle, no PID file")
    args = ap.parse_args(argv)
    if args.cmd == "start":
        return start(args)
    if args.cmd == "run":
        return run(args)
    if args.cmd == "stop":
        return stop(args)
    try:
        print(open(os.path.join(args.dir, "state.json")).read())
        return 0
    except OSError:
        print("no state yet", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
