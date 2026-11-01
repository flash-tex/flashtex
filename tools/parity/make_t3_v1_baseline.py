#!/usr/bin/env python3
"""Build tools/parity/baselines/t3-v1-oneoff.json from v1's last P5 board run.

SCOREBOARD-POST (Commander kabir-claude, 2026-11-01): retirement stage S7 (#1888) deleted
the old engine (v1), so the board can no longer measure the old column of the T3 parity
tiers. As T4's decision 1 did for T4, that column becomes a committed one-off measurement:
v1's parity.py run (`parity-old/`) from the last complete p5-scoreboard run that measured
it. This script copies it verbatim, adds the run's provenance and checks it three ways:

  - scoreboard.py's own load_parity reads the run (fail closed on a format change), and
    no tier may be partial or invalid;
  - the per-document records (id, level, P-T2) re-derive every count in the summary;
  - with --board, the counts equal that run's board's old column.

    python3 tools/parity/make_t3_v1_baseline.py --artifact DIR --run-id 41203982275 \\
        --artifact-id 13046836253 --event schedule --created 2026-10-31T13:05:22Z \\
        --commit 5a42ab270243440446e4f103e17a63350a8cf788 --out tools/parity/baselines/t3-v1-oneoff.json

DIR is the downloaded `p5-scoreboard` artifact (gh run download N -n p5-scoreboard): its
parity-old/{scoreboard,documents}.json, oracle.json and board/scoreboard.json.
Python 3 standard library only. MIT, like the rest of tools/parity.
"""

import argparse
import hashlib
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import scoreboard as sb  # noqa: E402

REPO_URL = "https://github.com/flash-tex/flashtex"
DECISION = {
    "name": "SCOREBOARD-POST",
    "ruling": ("v1 was deleted in retirement stage S7 (#1888), so no board can run it. The old "
               "column of the T3 parity tiers is v1's last complete measurement on the board, "
               "committed once, as T4 decision 1 does for T4; new >= old is then compared on "
               "the baseline's own documents"),
    "ruled_by": "Commander kabir-claude (lane SCOREBOARD-POST)",
    "date": "2026-11-01",
    "url": REPO_URL + "/pull/1888",
    "remeasure_when": "never: v1 no longer exists (git keeps it); D13 froze it before that",
}


def sha256_file(path):
    with open(path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


def build(artifact, run_id, artifact_id, event, created, commit, board_path=None):
    pdir = os.path.join(artifact, "parity-old")
    tiers, ident = sb.load_parity(pdir)
    if ident.get("engine_kind") != "flashtex-cli":
        raise SystemExit("%s: engine_kind %r is not v1 (flashtex-cli)" % (pdir, ident.get("engine_kind")))
    with open(os.path.join(pdir, "scoreboard.json"), encoding="utf-8") as f:
        meta = json.load(f)["meta"]
    with open(os.path.join(artifact, "oracle.json"), encoding="utf-8") as f:
        oracle = json.load(f)
    out_tiers = {}
    for tier, row in sorted(tiers.items(), key=lambda kv: sb.T3_TIERS.index(kv[0])):
        bad = [m for m, c in row.items() if c.get("partial") or c.get("invalid")]
        if bad:
            raise SystemExit("%s %s: partial or invalid in that run (%s)" % (tier, bad, row[bad[0]]))
        recs, excluded = sb.parity_records(os.path.join(pdir, "documents.json"), tier)
        v1 = {m: [row[m]["passed"], row[m]["of"]] for m in sb.T3_V1_METRICS}
        derived = sb.parity_record_counts(recs)
        if derived != v1:
            raise SystemExit("%s: documents.json gives %r, scoreboard.json %r" % (tier, derived, v1))
        if row["P-T1"]["status"] != "n/a":
            raise SystemExit("%s: v1's P-T1 is %r, expected n/a" % (tier, row["P-T1"]["status"]))
        out_tiers[tier] = {
            "documents": len(recs) + len(excluded), "measured": len(recs),
            "excluded": dict(sorted(row["L0"]["excluded"].items())),
            "v1": v1, "P-T1_not_evaluated": row["P-T1"]["note"],
            "id_list_sha256": sb.id_list_sha256([r["id"] for r in recs]),
            "records": recs, "excluded_ids": excluded,
        }
    if board_path:
        with open(board_path, encoding="utf-8") as f:
            board = json.load(f)
        for r in board["rows"]:
            if r["tier"] in out_tiers and r["metric"] in sb.T3_V1_METRICS:
                got = [r["old"]["passed"], r["old"]["of"]]
                if got != out_tiers[r["tier"]]["v1"][r["metric"]]:
                    raise SystemExit("board %s %s old %r, baseline %r" % (r["tier"], r["metric"], got,
                                                                       out_tiers[r["tier"]]["v1"][r["metric"]]))
        if not board.get("all_green"):
            raise SystemExit("%s: that board was not all green; pick a complete run" % board_path)
        v1_commit = board["engines"]["old"].get("git_sha")
        if not sb.sha_matches(v1_commit, commit):
            raise SystemExit("the board's v1 commit %r is not --commit %s" % (v1_commit, commit))
    run_url = "%s/actions/runs/%s" % (REPO_URL, run_id)
    files = {name: sha256_file(os.path.join(artifact, name)) for name in
             ("parity-old/scoreboard.json", "parity-old/documents.json", "oracle.json")
             + (("board/scoreboard.json",) if board_path else ())}
    return {
        "schema": sb.T3_V1_SCHEMA,
        "status": "FINAL",
        "decision": DECISION,
        "measured_date": meta.get("date"),
        "source": [run_url],
        "run": {"workflow": "p5-scoreboard.yml", "id": run_id, "event": event, "created": created,
                "url": run_url, "commit": commit, "artifact": "p5-scoreboard", "artifact_id": artifact_id,
                "files_sha256": files,
                "board": "All green on that board (every row, every run complete)" if board_path else None},
        "engine": {"label": "old", "kind": ident["engine_kind"], "version": ident["engine_version"],
                   "sha256": ident["engine_sha256"], "host": ident["host"], "platform": ident["platform"],
                   "started_utc": meta.get("started_utc"), "command": ident["command"]},
        "oracle": oracle,
        "measured_by": "p5-scoreboard.yml (tools/parity/scoreboard-run.sh, parity.py on %s)" % ident["engine_version"],
        "tiers": out_tiers,
    }


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--artifact", required=True, help="the downloaded p5-scoreboard artifact directory")
    ap.add_argument("--run-id", type=int, required=True)
    ap.add_argument("--artifact-id", type=int, required=True)
    ap.add_argument("--event", required=True)
    ap.add_argument("--created", required=True)
    ap.add_argument("--commit", required=True, help="the run's head commit (gh run view N --json headSha)")
    ap.add_argument("--no-board", action="store_true", help="do not cross-check against board/scoreboard.json")
    ap.add_argument("--out", required=True)
    a = ap.parse_args(argv)
    board = None if a.no_board else os.path.join(a.artifact, "board", "scoreboard.json")
    b = build(a.artifact, a.run_id, a.artifact_id, a.event, a.created, a.commit, board)
    with open(a.out, "w", encoding="utf-8") as f:
        json.dump(b, f, indent=1)
        f.write("\n")
    check = sb.load_t3_v1_baseline(a.out)
    if not isinstance(check, dict):
        raise SystemExit(check)
    for t, v in b["tiers"].items():
        print("%s: %d measured; v1 %s" % (t, v["measured"], " ".join("%s %d/%d" % (m, *v["v1"][m])
                                                                     for m in sb.T3_V1_METRICS)))
    return 0


if __name__ == "__main__":
    sys.exit(main())
