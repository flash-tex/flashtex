"""Side-by-side scoreboard of several parity runs (DESIGN §8 T3, §12 P5 gate).

    python3 tools/parity/engines.py --run new=<out dir> --run v1=<out dir> --run pdflatex=<out dir> \\
        --subject new [--sha new=<git sha> ...] [--notes <notes.json>] --out tools/parity/reports/<name>

Each `<out dir>` is what `parity.py --out` wrote (`scoreboard.json` and
`documents.json`). This writes `<name>.json` and `<name>.md`: small,
deterministic (sorted, no wall-clock times) and labelled with the host that
measured them, so a report is never mistaken for another host's data (§8).

Per tier and engine: P-T1, P-T2 and the cumulative levels L0-L4. For the
`--subject` engine, every document that is not a full pass (P-T1, P-T2 and
L4) gets one root-cause class:

  a  a package or font the user's TeX Live lacks (the §4.4 bundle fallback)
  b  an engine difference: the first differing log, box or content line
  c  a harness issue in tools/parity
  d  pdflatex fails too: excluded from the denominator
  e  excluded by the harness's convergence rule: pdflatex compiles it, but
     its log asks for a rerun on every pass (e.g. natbib's endless
     `Rerun to get citations correct.`), so no reference is recorded

The automatic rule reads only the run's records. `--notes` (JSON: document
id -> {"class", "cause", "section", "owner", "issue"}) adds what a person
found by reading the logs, e.g. the pdftex.web section and the issue link;
a note's class overrides the automatic one and the report marks it.
"""

import argparse
import collections
import json
import os
import re
import sys

LEVELS = ["L0", "L1", "L2", "L3", "L4"]
CLASSES = {"a": "package or font missing from the user's TeX Live (§4.4 bundle fallback)",
           "b": "engine difference",
           "c": "harness issue (tools/parity)",
           "d": "pdflatex fails too (excluded)",
           "e": "excluded by the convergence rule: pdflatex compiles it but keeps asking for a rerun"}
MISSING_FILE = re.compile(r"File `([^']+)' not found|Font \\?(\S+)=?\S* not loadable|"
                          r"I can't find file `([^']+)'")


def load_run(path):
    with open(os.path.join(path, "scoreboard.json"), encoding="utf-8") as f:
        board = json.load(f)
    with open(os.path.join(path, "documents.json"), encoding="utf-8") as f:
        docs = json.load(f)
    return board, docs


def tier_row(summary):
    """P-T1, P-T2 and L0-L4 as [passed, of] (None when not evaluated)."""
    pt = summary.get("pt") or {}
    row = {}
    for t in ("P-T1", "P-T2"):
        s = pt.get(t) or {}
        row[t] = [s["passed"], s["evaluated"]] if s.get("evaluated") else None
    for name in LEVELS:
        c = (summary.get("at_least") or {}).get(name) or {}
        row[name] = [c["documents"], summary["measured"]] if c.get("documents") is not None else None
    row["documents"] = summary["documents"]
    row["excluded"] = dict(sorted((summary.get("excluded") or {}).items()))
    return row


def full_pass(r):
    pt = r.get("pt") or {}
    return (r.get("level") == len(LEVELS) - 1 and pt.get("P-T1") in (True, None)
            and pt.get("P-T2") in (True, None))


def first_difference(r):
    """The first differing line the run recorded, as one string."""
    pt = r.get("pt") or {}
    pt1, pt2 = pt.get("pt1") or {}, pt.get("pt2") or {}
    if pt1.get("log_line"):
        d = pt1["log_line"]
        return f"log line {d['line']}: oracle `{d['oracle']}` vs `{d['candidate']}`"
    if pt1.get("box_line"):
        d = pt1["box_line"]
        return f"shipout {pt1.get('first_shipout')} box line {d['line']}: oracle `{d['oracle']}` vs `{d['candidate']}`"
    if pt1.get("streamed") and pt1.get("ok") is False:  # compared as streams: where, not the text
        if not pt1.get("boxes_equal"):
            return f"shipout {pt1.get('first_shipout')} of {pt1.get('shipouts')} (streamed)"
        w = pt1.get("log_lines") or {}
        return f"log lines {w.get('from')}-{w.get('to')} (streamed)"
    if pt1.get("why"):
        return pt1["why"]
    if pt2.get("first_page"):
        fp = pt2["first_page"]
        s = f"P-T2 page {fp['page']}: {', '.join(fp['differs'])}"
        if fp.get("content_line"):
            d = fp["content_line"]
            s += f"; content line {d['line']}: oracle `{d['oracle']}` vs `{d['candidate']}`"
        return s
    if pt2 and not pt2.get("fonts_equal", True):
        f = pt2.get("fonts") or {}
        return f"P-T2 fonts: missing {f.get('missing')}, extra {f.get('extra')}, program {f.get('different_program')}"
    if pt2.get("why"):
        return pt2["why"]
    return None


def classify(r):
    """(class, cause) for one subject record that is not a full pass."""
    ex = r.get("excluded")
    if ex:
        if ex.startswith("oracle: did not converge"):
            return "e", ex
        if ex.startswith("oracle"):
            return "d", ex
        return "c", ex
    cand = r.get("candidate") or {}
    pt = r.get("pt") or {}
    if (pt.get("excluded") or "").startswith("oracle"):
        return ("e" if "did not converge" in pt["excluded"] else "d"), "P-T oracle pdfTeX: " + pt["excluded"]
    if r.get("worker_died"):
        # counted as failed in every denominator; the cause is unknown (memory?)
        return "c", cand.get("status") or "the worker process died"
    if r.get("level") == -1:
        why = " ".join(str(cand.get(k) or "") for k in ("status", "stderr_tail"))
        m = MISSING_FILE.search(why)
        if m:
            return "a", "missing " + next(g for g in m.groups() if g)
        return "b", "L0: " + (why.strip()[:200] or "no PDF")
    if "did not finish in the capture's" in ((pt.get("pt1") or {}).get("why") or ""):
        # the capture's time limit stopped the traced pass: a limit of the harness, not a
        # difference (a traced pass that crashed by itself falls through to b)
        return "c", pt["pt1"]["why"]
    if pt.get("P-T1") in (True, None) and pt.get("P-T2") in (True, None) and pt.get("P-T1") is not None:
        # identical traces and PDFs, yet a level fails: the measurement is wrong
        return "c", f"P-T1 and P-T2 pass but level {LEVELS[r['level']] if r['level'] >= 0 else 'below L0'}"
    return "b", first_difference(r) or f"level {r.get('level')}"


def build(runs, subject, shas=None, notes=None, host_label=None):
    shas, notes = shas or {}, notes or {}
    out = {"schema": "flashtex-parity-engines/1", "subject": subject, "host_label": host_label,
           "engines": {}, "tiers": {}, "documents": {}}
    tiers = []
    for label, (board, docs) in runs.items():
        m = board["meta"]
        out["engines"][label] = {
            "engine_version": m.get("engine_version"), "engine_kind": m.get("engine_kind"),
            "engine_sha256": m.get("flashtex_sha256"), "git_sha": shas.get(label),
            "shell_escape": m.get("shell_escape"), "pt": m.get("pt"), "raster": m.get("rasterizer"),
            "oracle_pdftex": m.get("oracle_pdftex_version"), "tex_live": m.get("pdflatex"),
            "host": host_label or m.get("host"), "platform": m.get("platform"), "date": m.get("date")}
        for t, v in board["tiers"].items():
            if t not in tiers:
                tiers.append(t)
            out["tiers"].setdefault(t, {})[label] = tier_row(v["summary"])
    for t in tiers:
        recs = {label: {r["id"]: r for r in docs.get(t, [])} for label, (_, docs) in runs.items()}
        ids = sorted(set().union(*(set(v) for v in recs.values())))
        rows = []
        for i in ids:
            row = {"id": i}
            for label in runs:
                r = recs[label].get(i)
                if r is None:
                    row[label] = None
                    continue
                pt = r.get("pt") or {}
                row[label] = {"level": None if r.get("excluded") else r.get("level"),
                              "P-T1": pt.get("P-T1"), "P-T2": pt.get("P-T2")}
            r = recs.get(subject, {}).get(i)
            pt = (r or {}).get("pt") or {}
            if r is not None and not r.get("excluded") and pt.get("P-T1") is None and not pt.get("excluded"):
                why = (pt.get("why") or {}).get("P-T1") or "not run"
                if not why.startswith("n/a"):  # the CLI has no P-T1 at all; that is not a skip
                    row["pt1_not_evaluated"] = why
            if r is not None and not full_pass(r):
                cls, cause = classify(r)
                note = notes.get(f"{t}/{i}") or notes.get(i) or {}
                row["class"] = note.get("class", cls)
                row["cause"] = cause
                for k in ("section", "owner", "issue", "note"):
                    if note.get(k):
                        row[k] = note[k]
                if note.get("class") and note["class"] != cls:
                    row["auto_class"] = cls
            rows.append(row)
        out["documents"][t] = rows
    counts = collections.Counter(row["class"] for rows in out["documents"].values() for row in rows if "class" in row)
    out["classes"] = {k: counts.get(k, 0) for k in CLASSES}
    return out


def cell(v):
    if v is None:
        return "n/a"
    p, n = v
    return f"{p}/{n} ({100.0 * p / n:.1f}%)" if n else "0/0"


def markdown(rep, title):
    L = [f"# {title}", ""]
    if rep.get("host_label"):
        L += [f"Measured on **{rep['host_label']}**. Host-dependent data: it is not another host's baseline "
              "(DESIGN §8).", ""]
    labels = list(rep["engines"])
    L.append("| engine | version | git SHA | engine sha256 | host | TeX Live | \\write18 |")
    L.append("|---|---|---|---|---|---|---|")
    for k, e in rep["engines"].items():
        L.append(f"| {k} | {e['engine_version']} | {e['git_sha'] or '—'} | {(e['engine_sha256'] or '')[:12]} | "
                 f"{e['host']} ({e['platform']}) | {e['tex_live']} | {e['shell_escape']} |")
    L.append("")
    for t, per in rep["tiers"].items():
        L.append(f"## {t}")
        L.append("")
        L.append("| engine | documents | excluded | P-T1 | P-T2 | " + " | ".join(LEVELS) + " |")
        L.append("|---|---|---|---|---|" + "---|" * len(LEVELS))
        for k in labels:
            row = per.get(k)
            if row is None:
                continue
            ex = ", ".join(f"{a} {b}" for a, b in row["excluded"].items()) or "0"
            L.append(f"| {k} | {row['documents']} | {ex} | {cell(row['P-T1'])} | {cell(row['P-T2'])} | "
                     + " | ".join(cell(row[n]) for n in LEVELS) + " |")
        L.append("")
    subj = rep["subject"]
    L.append(f"## Root causes ({subj}): every document that is not P-T1 + P-T2 + L4")
    L.append("")
    for k, v in CLASSES.items():
        L.append(f"- ({k}) {v}: **{rep['classes'][k]}**")
    L.append("")
    L.append(f"| tier | document | {subj} level | P-T1 | P-T2 | class | cause | § | owner | issue |")
    L.append("|---|---|---|---|---|---|---|---|---|---|")
    for t, rows in rep["documents"].items():
        for row in rows:
            if "class" not in row:
                continue
            s = row.get(subj) or {}
            lvl = s.get("level")
            lvl = "excluded" if lvl is None else ("below L0" if lvl == -1 else LEVELS[lvl])
            mark = {True: "pass", False: "fail", None: "n/a"}
            cls = row["class"] + (f" (auto {row['auto_class']})" if row.get("auto_class") else "")
            cause = (row.get("note") or row["cause"] or "").replace("|", "\\|").replace("\n", " ")[:220]
            L.append(f"| {t} | {row['id']} | {lvl} | {mark[s.get('P-T1')]} | {mark[s.get('P-T2')]} | {cls} | "
                     f"{cause} | {row.get('section', '')} | {row.get('owner', '')} | {row.get('issue', '')} |")
    L.append("")
    skipped = [(t, row) for t, rows in rep["documents"].items() for row in rows if row.get("pt1_not_evaluated")]
    if skipped:
        L.append(f"## P-T1 not evaluated ({subj}): {len(skipped)} documents")
        L.append("")
        L.append("They are in no P-T1 denominator. Their P-T2 and levels are measured as usual.")
        L.append("")
        for t, row in skipped:
            L.append(f"- {t}/{row['id']}: {row['pt1_not_evaluated']}")
        L.append("")
    return "\n".join(L)


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--run", action="append", required=True, metavar="LABEL=DIR")
    ap.add_argument("--subject", required=True, help="the engine whose failures are classified")
    ap.add_argument("--sha", action="append", default=[], metavar="LABEL=SHA")
    ap.add_argument("--notes", default=None)
    ap.add_argument("--title", default="Parity scoreboard: engines side by side")
    ap.add_argument("--host-label", default=None, help="the machine alias that measured the runs")
    ap.add_argument("--out", required=True, help="path stem: writes <out>.json and <out>.md")
    args = ap.parse_args(argv)
    runs = {}
    for kv in args.run:
        label, path = kv.split("=", 1)
        runs[label] = load_run(path)
    if args.subject not in runs:
        print(f"--subject {args.subject} is not one of {sorted(runs)}", file=sys.stderr)
        return 2
    notes = {}
    if args.notes:
        with open(args.notes, encoding="utf-8") as f:
            notes = json.load(f)
    rep = build(runs, args.subject, dict(kv.split("=", 1) for kv in args.sha), notes, args.host_label)
    with open(args.out + ".json", "w", encoding="utf-8") as f:
        json.dump(rep, f, indent=1, sort_keys=True, ensure_ascii=False)
        f.write("\n")
    with open(args.out + ".md", "w", encoding="utf-8") as f:
        f.write(markdown(rep, args.title))
    print(json.dumps(rep["classes"]))
    return 0


if __name__ == "__main__":
    sys.exit(main())
