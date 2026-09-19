#!/usr/bin/env python3
"""Verify a Beads soak run: identical ledgers, no lost tasks, no double closes.

Inputs:
  --a-list / --b-list   `bd list --all -n 0 --json` output from each machine,
                        taken after both loops have quiesced
  --logs                every agent JSONL log (soak_agent.py) and collider log
"""
import argparse
import collections
import json
import sys

KEYS = ("id", "title", "status", "assignee", "notes", "close_reason", "labels", "issue_type", "priority")


def norm(path):
    data = json.load(open(path))
    rows = {}
    for i in data:
        rows[i["id"]] = {k: (sorted(i.get(k) or []) if k == "labels" else i.get(k)) for k in KEYS}
    return rows


ap = argparse.ArgumentParser()
ap.add_argument("--a-list", required=True)
ap.add_argument("--b-list", required=True)
ap.add_argument("--logs", nargs="+", required=True)
a = ap.parse_args()
A, B = norm(a.a_list), norm(a.b_list)
report = {"beads_A": len(A), "beads_B": len(B)}
report["only_in_A"] = sorted(set(A) - set(B))
report["only_in_B"] = sorted(set(B) - set(A))
report["field_mismatches"] = [k for k in sorted(set(A) & set(B)) if A[k] != B[k]][:20]
report["identical"] = not report["only_in_A"] and not report["only_in_B"] and not report["field_mismatches"]

events = []
for p in a.logs:
    for line in open(p):
        line = line.strip()
        if line:
            events.append(json.loads(line))
claims = collections.Counter(e.get("result") for e in events if e["ev"] == "claim")
counted = collections.defaultdict(set)
for e in events:
    if e["ev"] == "claim" and e.get("result") == "CLAIMED":
        counted[e["id"]].add(e["actor"])
closes = collections.defaultdict(list)
for e in events:
    if e["ev"] == "close" and e.get("rc") == 0:
        closes[e["id"]].append(e["actor"])
created = [e["id"] for e in events if e["ev"] == "create" and e.get("rc") == 0 and e.get("id")]
report["agents"] = len({e["actor"] for e in events if e["ev"] in ("claim", "idle", "exit")})
report["claim_results"] = dict(claims)
report["cycles_closed"] = sum(len(v) for v in closes.values())
report["tasks_counted_claimed_by_two_agents"] = {k: sorted(v) for k, v in counted.items() if len(v) > 1}
report["tasks_closed_by_two_agents"] = {k: v for k, v in closes.items() if len(set(v)) > 1}
report["created_followups"] = len(created)
report["created_missing_from_ledger"] = [i for i in created if i not in A or i not in B]
wrong_closer = []
for k, v in closes.items():
    row = A.get(k)
    if not row or row["status"] != "closed" or row["assignee"] not in v:
        wrong_closer.append({"id": k, "closers": v, "ledger": row and {x: row[x] for x in ("status", "assignee", "close_reason")}})
report["closed_in_log_but_ledger_disagrees"] = wrong_closer[:20]
report["abandoned_not_mine"] = sum(1 for e in events if e["ev"] == "abandon_not_mine")
coll = [e for e in events if e["ev"] == "collision"]
report["deliberate_collisions"] = len(coll)
report["deliberate_collisions_single_winner"] = sum(1 for e in coll if e.get("winners") == 1)
ok = (report["identical"] and not report["tasks_counted_claimed_by_two_agents"] and not report["tasks_closed_by_two_agents"]
      and not report["created_missing_from_ledger"] and not report["closed_in_log_but_ledger_disagrees"])
report["PASS"] = ok
print(json.dumps(report, indent=2))
sys.exit(0 if ok else 1)
