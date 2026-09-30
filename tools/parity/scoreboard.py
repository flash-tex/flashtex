#!/usr/bin/env python3
"""P5 scoreboard: the new engine against v1 on every tier, pdflatex as the oracle.

DESIGN §12 P5's exit gate is "Scoreboard: new engine >= old on every tier;
arXiv L1 >= 90%; retirement complete", over the §8 tiers. This module
measures nothing itself. It reads what the existing harnesses already wrote
and puts them in one table:

  source          harness (writes)                          tiers / metrics
  --parity        tools/parity/parity.py (scoreboard.json,  every corpus tier in the run:
                  documents.json)                            fixtures, arxiv, templates,
                                                             packages (#1288); P-T1, P-T2, L0-L4
  --nightly       tools/parity/nightly.py (summary.json;     nightly-5k (T4, #1276) and any
                  #1276)                                     other tier it ran
  --latex-suites  tools/latex-suites/run.py (stdout)         T2: tests that agree with pdfTeX
  --package-smoke tools/package-smoke/run.py (stdout)        documents equal to pdfTeX
  --fonts         tools/font-census/census.py (census.json)  fonts identical to pdfTeX

Each source is given per engine as LABEL=PATH, where LABEL is `new` or
`old` (v1, the `flashtex` CLI). Expected data is only ever the oracle's:
every harness above compares against pdfTeX 1.40.29 / pdflatex itself.

Honest denominators:
- a row's denominator is the harness's own (documents pdflatex compiles,
  tests run, fonts tested); what the harness excluded is printed with its
  reason, never dropped;
- a run limited with --limit/--only/--shard/--spread, a nightly run with
  missing shards or unmeasured documents, or a board built with
  --sample-note is marked partial, and a partial board is never all-green;
- a run in which a worker died (parity.py exit 3) is invalid;
- a harness that cannot drive the v1 CLI (it needs a pdfTeX-compatible
  binary: latex-suites, package-smoke, fonts; P-T1 in parity.py) gives old
  "n/a" with that reason. new >= old then holds only if new was measured,
  and the verdict says "old n/a" so nobody reads it as a comparison;
- a tier with no result for an engine is "missing", which is not green.

Verdicts: ahead, equal, ahead (old n/a) are green; behind, below target,
denominators differ, host mismatch, invalid, missing are not. Targets beyond
new >= old: arXiv L1 >= 90% (§12 P5) and 0 unexpected T2 failures (the
retirement plan's S5 precondition).

Outputs (--out DIR): scoreboard.json, scoreboard.md (the one table, for the
artifact), and with --summary FILE a short committed summary. --issues
dry-run|apply opens or updates one GitHub issue per tier where new < old
(marker `<!-- p5-scoreboard:tier=NAME -->` in the body) and closes the
scoreboard's own issue once the tier recovers.

    python3 tools/parity/scoreboard.py \\
        --parity new=<dir> --parity old=<dir> --nightly new=<dir> \\
        --latex-suites new=<file> --package-smoke new=<file> --fonts new=<dir> \\
        --sha new=<sha> --sha old=<sha> --out <dir> [--summary <file>] [--issues dry-run]

Python 3 standard library only. MIT, like the rest of tools/parity.
"""

import argparse
import json
import os
import platform
import re
import shutil
import subprocess
import sys

SCHEMA = "flashtex-p5-scoreboard/1"
LABELS = ("new", "old")
HERE = os.path.dirname(os.path.abspath(__file__))
STAGES_FILE = os.path.join(HERE, "retirement-stages.json")

PARITY_METRICS = ("P-T1", "P-T2", "L0", "L1", "L2", "L3", "L4")
METRIC_ORDER = PARITY_METRICS + ("tests", "documents", "fonts", "(any)")
GREEN = ("ahead", "equal", "ahead (old n/a)")
ARXIV_L1_TARGET = 90.0
CENSUS_PER_FAMILY = 6  # tools/font-census/census.py --per-family default: the tier's definition
# parity.py options that select a subset of a tier's manifest.
SUBSET_FLAGS = ("--limit", "--only", "--shard", "--spread")

TIER_TITLES = {
    "fixtures": "T3 fixtures",
    "arxiv": "T3 arXiv",
    "templates": "T3 templates",
    "packages": "T3 packages (#1288)",
    "nightly-5k": "T4 5k corpus (#1276)",
    "latex-suites": "T2 LaTeX suites",
    "package-smoke": "package-smoke",
    "fonts": "fonts (font census)",
}
TIER_ORDER = ("fixtures", "arxiv", "templates", "packages", "nightly-5k",
              "latex-suites", "package-smoke", "fonts")

NA_TEX_ONLY = ("the harness drives a pdfTeX-compatible binary; the v1 "
               "flashtex CLI is not one, so it cannot be measured here")


# --------------------------------------------------------------------------
# cells


def cell(status, passed=None, of=None, **extra):
    """One engine's result on one row. status: measured | n/a | not run."""
    c = {"status": status, "passed": passed, "of": of, "excluded": {},
         "skipped": 0, "note": None, "partial": None, "invalid": None}
    c.update(extra)
    return c


def pct(c):
    if c["status"] != "measured" or not c["of"]:
        return None
    return round(100.0 * c["passed"] / c["of"], 1)


def fmt_cell(c):
    if c is None:
        return "missing"
    if c["status"] == "n/a":
        return "n/a"
    if c["status"] != "measured":
        return "not run"
    p = pct(c)
    s = "%d/%d" % (c["passed"], c["of"])
    if p is not None:
        s += " (%.1f%%)" % p
    extras = []
    if c.get("skipped"):
        extras.append("skipped %d" % c["skipped"])
    if c.get("partial"):
        extras.append("partial")
    if c.get("sample"):
        extras.append("sample")
    if c.get("invalid"):
        extras.append("INVALID")
    return s + (" [" + ", ".join(extras) + "]" if extras else "")


# --------------------------------------------------------------------------
# adapters: one per harness. Each returns {tier: {metric: cell}} plus the
# run's identity (host, oracle, engine) for the same-host check.


def _read_json(path):
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def subset_flags(command):
    """The subset-selecting flags in a recorded parity.py command line."""
    toks = (command or "").split()
    return sorted({t.split("=")[0] for t in toks if t.split("=")[0] in SUBSET_FLAGS})


MANIFEST_DIR = os.path.join(HERE, "corpus")


def manifest_sizes(manifest_dir=MANIFEST_DIR):
    """{tier: number of manifest entries} for every corpus manifest present."""
    out = {}
    if os.path.isdir(manifest_dir):
        for name in sorted(os.listdir(manifest_dir)):
            if name.endswith(".json"):
                try:
                    m = _read_json(os.path.join(manifest_dir, name))
                except (OSError, ValueError):
                    continue
                if m.get("tier") and isinstance(m.get("entries"), list):
                    out[m["tier"]] = out.get(m["tier"], 0) + len(m["entries"])
    return out


def short_of_manifest(tier, documents, sizes):
    """A run that saw fewer documents than its tier's manifest lists is partial."""
    n = sizes.get(tier)
    if n is not None and documents is not None and documents < n:
        return "%d of %d manifest entries" % (documents, n)
    return None


def load_parity(path, sizes=None):
    """parity.py's --out directory (scoreboard.json + documents.json)."""
    sizes = manifest_sizes() if sizes is None else sizes
    sb = _read_json(os.path.join(path, "scoreboard.json"))
    meta = sb.get("meta", {})
    died = {}
    docs_path = os.path.join(path, "documents.json")
    if os.path.exists(docs_path):
        docs = _read_json(docs_path)
        for tier, rows in docs.items():
            if isinstance(rows, list):
                died[tier] = sum(1 for d in rows if d.get("worker_died"))
    flags = subset_flags(meta.get("command"))
    tiers = {}
    for tier, body in (sb.get("tiers") or {}).items():
        s = body.get("summary", {})
        why = [w for w in (("subset: " + " ".join(flags)) if flags else None,
                           short_of_manifest(tier, s.get("documents"), sizes)) if w]
        partial = "; ".join(why) or None
        measured = s.get("measured", 0)
        excluded = dict(s.get("excluded") or {})
        invalid = ("worker died on %d document(s)" % died[tier]) if died.get(tier) else None
        row = {}
        pt = s.get("pt") or {}
        for m in ("P-T1", "P-T2"):
            t = pt.get(m)
            if t is None:
                row[m] = cell("not run", note="no %s result in this run" % m)
                continue
            ne = t.get("not_evaluated")
            if not t.get("evaluated") and ne:
                status = "n/a" if ne.startswith("n/a") else "not run"
                row[m] = cell(status, note=ne)
                continue
            c = cell("measured", t.get("passed", 0), t.get("evaluated", 0),
                     skipped=t.get("skipped", 0), partial=partial, invalid=invalid,
                     note=ne)
            c["excluded"] = dict(excluded, **(pt.get("excluded") or {}))
            if t.get("skipped"):
                c["excluded"]["P-T1 not evaluated (--pt1-skip / log cap)"] = t["skipped"]
            row[m] = c
        for m in ("L0", "L1", "L2", "L3", "L4"):
            a = (s.get("at_least") or {}).get(m) or {}
            if a.get("documents") is None:
                row[m] = cell("not run", note="not evaluated in this run")
                continue
            c = cell("measured", a["documents"], measured, partial=partial, invalid=invalid)
            c["excluded"] = dict(excluded)
            row[m] = c
        tiers[tier] = row
    ident = {"host": meta.get("host"), "platform": meta.get("platform"),
             "oracle": meta.get("oracle_pdftex_version") or meta.get("pdflatex"),
             "engine_kind": meta.get("engine_kind"),
             "engine_version": meta.get("engine_version"),
             "engine_sha256": meta.get("flashtex_sha256"),
             "date": meta.get("date"), "command": meta.get("command")}
    return tiers, ident


def load_nightly(path, sizes=None):
    """nightly.py's --out directory (summary.json, schema flashtex-nightly/1)."""
    sizes = manifest_sizes() if sizes is None else sizes
    p = os.path.join(path, "summary.json") if os.path.isdir(path) else path
    sm = _read_json(p)
    shards = sm.get("shards") or {}
    missing = shards.get("missing")
    nmissing = len(missing) if isinstance(missing, list) else (missing or 0)
    tiers = {}
    for tier, t in (sm.get("tiers") or {}).items():
        unmeasured = t.get("unmeasured") or 0
        notes = []
        if nmissing:
            notes.append("%d shard(s) missing" % nmissing)
        if unmeasured:
            notes.append("%d document(s) unmeasured" % unmeasured)
        if sm.get("mixed_fingerprint"):
            notes.append("mixed fingerprints")
        short = short_of_manifest(tier, t.get("documents"), sizes)
        if short:
            notes.append(short)
        partial = "; ".join(notes) or None
        excluded = dict(t.get("excluded") or {})
        row = {}
        kind = (sm.get("engine") or {}).get("kind")
        # L4: nightly.py counts an un-rasterised L4 as [0, measured], which cannot be told
        # apart from a measured 0, so it is left out (L4 is not in the P5 gate list either).
        for m in PARITY_METRICS[:-1]:
            v = t.get(m)
            if v is None:
                if m == "P-T1" and kind == "flashtex-cli":
                    row[m] = cell("n/a", note="n/a: the flashtex CLI is not a TeX engine and "
                                  "writes no box dumps or \\tracingall log")
                else:
                    row[m] = cell("not run", note="not evaluated in this run")
                continue
            c = cell("measured", v[0], v[1], partial=partial)
            c["excluded"] = dict(excluded)
            if m == "P-T1":
                ne = t.get("P-T1_not_evaluated") or 0
                cap = t.get("P-T1_over_cap") or 0
                if ne:
                    c["skipped"] = ne
                    c["excluded"]["P-T1 not evaluated (outside the --pt1-sample, or over the log cap: %d)"
                                  % cap] = ne
            row[m] = c
        tiers[tier] = row
    host = sm.get("host") or {}
    eng = sm.get("engine") or {}
    ident = {"host": host.get("node") or host.get("label"), "host_label": host.get("label"),
             "platform": host.get("platform"), "oracle": None,
             "engine_kind": eng.get("kind"), "engine_version": eng.get("version"),
             "engine_sha256": eng.get("sha256"), "date": sm.get("generated_utc"),
             "git_sha": sm.get("git_sha")}
    return tiers, ident


SUITE_LINE = re.compile(r"^(\S.*): PASS (\d+) / FAIL (\d+) / SKIP (\d+)\s*$")
UNEXPECTED_LINE = re.compile(r"^UNEXPECTED failures: (.*)$")
OK_LINE = re.compile(r"^OK: (\d+) ran, (\d+) failed")


FAILING_LINE = re.compile(r"^  (\S+) \[(UNEXPECTED|expected)")  # as tools/latex-suites/compare_failures.py


def failing_tests(text):
    """The names under `failing tests:` in a run.py transcript."""
    out, on = set(), False
    for line in text.splitlines():
        if line.startswith("failing tests:"):
            on = True
            continue
        m = FAILING_LINE.match(line)
        if on and m:
            out.add(m.group(1))
        elif on and not line.startswith("  "):
            on = False
    return out


def parse_latex_suites(text, reference=None):
    """tools/latex-suites/run.py's stdout -> one row.

    passed = tests whose result agrees with pdfTeX; of = tests run. Target:
    0 unexpected failures. Without `reference`, "unexpected" is run.py's own
    verdict against EXPECTED-FAILURES.txt (pdfTeX 1.40.29 on the pinned TeX
    Live). With `reference` (the transcript of the same run through this
    host's pdfTeX), it is every test the engine fails and pdfTeX passes, as
    scripts/engine-parity.sh's T2 step decides on a newer TeX Live.
    """
    dirs, unexpected, ok = {}, None, False
    for line in text.splitlines():
        m = SUITE_LINE.match(line)
        if m:
            dirs[m.group(1)] = (int(m.group(2)), int(m.group(3)))
            continue
        m = UNEXPECTED_LINE.match(line)
        if m:
            unexpected = [x.strip() for x in m.group(1).split(",") if x.strip()]
        elif OK_LINE.match(line):
            ok = True
    if not dirs:
        return {"tests": cell("measured", 0, 0, invalid="no per-directory PASS/FAIL line "
                              "(infra error or crash in run.py)")}
    ran = sum(p + f for p, f in dirs.values())
    failed = sum(f for _, f in dirs.values())
    if unexpected is None and not ok:
        return {"tests": cell("measured", 0, ran, invalid="no OK/UNEXPECTED summary (run.py stopped early)")}
    basis = "EXPECTED-FAILURES.txt"
    if reference is not None:
        if not any(SUITE_LINE.match(x) for x in reference.splitlines()):
            return {"tests": cell("measured", 0, ran, invalid="the pdfTeX reference transcript has no "
                                  "PASS/FAIL line")}
        unexpected = sorted(failing_tests(text) - failing_tests(reference))
        basis = "this host's pdfTeX (reference run)"
    bad = len(unexpected or ())
    c = cell("measured", ran - bad, ran,
             note="%d failed, %d unexpected against %s; dirs: %s" % (
                 failed, bad, basis, ", ".join("%s %d/%d" % (k, p, p + f)
                                               for k, (p, f) in sorted(dirs.items()))))
    c["unexpected"] = unexpected or []
    return {"tests": c}


SMOKE_LINE = re.compile(r"^(\S+)\s+(equal|DIFFERENT)(?: \((.*)\))?\s*$")
SMOKE_TOTAL = re.compile(r"^(\d+) documents, (\d+) differ\s*$")


SMOKE_DIR = os.path.join(os.path.dirname(HERE), "package-smoke")


def smoke_documents(smoke_dir=SMOKE_DIR):
    """How many documents run.py runs with no package arguments (its *.tex files)."""
    try:
        return sum(1 for n in os.listdir(smoke_dir) if n.endswith(".tex"))
    except OSError:
        return None


def parse_package_smoke(text, expected=None):
    """tools/package-smoke/run.py's stdout -> one row (documents equal to pdfTeX).

    expected: the number of documents a full run covers; fewer is partial."""
    equal, differ, total = [], [], None
    for line in text.splitlines():
        m = SMOKE_LINE.match(line)
        if m:
            (equal if m.group(2) == "equal" else differ).append(m.group(1))
            continue
        m = SMOKE_TOTAL.match(line)
        if m:
            total = (int(m.group(1)), int(m.group(2)))
    n = len(equal) + len(differ)
    if total is None:
        return {"documents": cell("measured", len(equal), n,
                                  invalid="no 'N documents, M differ' line (harness error)")}
    if total != (n, len(differ)):
        return {"documents": cell("measured", len(equal), total[0],
                                  invalid="summary %d/%d disagrees with %d result lines"
                                  % (total[0], total[1], n))}
    c = cell("measured", len(equal), n, note=("differ: " + ", ".join(differ)) if differ else None)
    if expected is not None and n < expected:
        c["partial"] = "%d of %d package-smoke documents" % (n, expected)
    return {"documents": c}


def load_fonts(path):
    """tools/font-census/census.py's --out directory (census.json) -> one row.

    passed = fonts whose PDF is identical to the oracle's; of = fonts tested
    minus those the oracle fails on too (listed as excluded)."""
    p = os.path.join(path, "census.json") if os.path.isdir(path) else path
    s = _read_json(p).get("summary", {})
    kinds = s.get("by_kind") or {}
    a = kinds.get("all")
    if a is None:
        return {"fonts": cell("measured", 0, 0, invalid="census.json has no by_kind.all")}
    both = a.get("both-fail", 0)
    c = cell("measured", a.get("identical", 0), a.get("tested", 0) - both,
             note="per kind: " + ", ".join(
                 "%s %d/%d" % (k, v.get("identical", 0), v.get("tested", 0) - v.get("both-fail", 0))
                 for k, v in sorted(kinds.items()) if k != "all"))
    if both:
        c["excluded"] = {"oracle fails too": both}
    pf = s.get("per_family")
    if pf is not None and pf < CENSUS_PER_FAMILY:
        c["sample"] = "%s font(s) per family (census default %d)" % (pf, CENSUS_PER_FAMILY)
    return {"fonts": c}


# --------------------------------------------------------------------------
# the board


def verdict(tier, metric, new, old, same_host):
    if new is None or new["status"] != "measured":
        return "missing"
    if new.get("invalid") or (old or {}).get("invalid"):
        return "invalid"
    if old is None or old["status"] == "not run":
        return "missing"
    if old["status"] == "n/a":
        v = "ahead (old n/a)"
    else:
        if not same_host:
            return "host mismatch"
        if new["of"] != old["of"]:
            return "denominators differ"
        v = "ahead" if new["passed"] > old["passed"] else (
            "equal" if new["passed"] == old["passed"] else "behind")
    if v == "behind":
        return v
    if tier == "arxiv" and metric == "L1" and (pct(new) or 0.0) < ARXIV_L1_TARGET:
        return "below target"
    if tier == "latex-suites" and new.get("unexpected"):
        return "below target"
    return v


def target_of(tier, metric):
    if tier == "arxiv" and metric == "L1":
        return "new >= old; >= 90%"
    if tier == "latex-suites":
        return "new >= old; 0 unexpected"
    return "new >= old"


def load_stages(path=STAGES_FILE):
    return _read_json(path)["stages"]


def gated_stages(tier, metric, stages):
    out = []
    for st in stages:
        g = st.get("scoreboard_gate")
        if g == "all" or (isinstance(g, list) and "%s:%s" % (tier, metric) in g):
            out.append(st["id"])
    return out


def build(sources, shas=None, sample_note=None, stages=None, host_label=None):
    """sources: {label: [(kind, tiers, ident)]} -> the board dict."""
    stages = load_stages() if stages is None else stages
    shas = shas or {}
    cells = {lab: {} for lab in LABELS}
    idents = {lab: [] for lab in LABELS}
    for lab in LABELS:
        for kind, tiers, ident in sources.get(lab, ()):
            idents[lab].append(dict(ident, source=kind))
            for tier, row in tiers.items():
                for metric, c in row.items():
                    c = dict(c, source=kind, host=ident.get("host"), oracle=ident.get("oracle"))
                    prev = cells[lab].setdefault(tier, {}).get(metric)
                    if prev is not None and prev["status"] == "measured" and c["status"] != "measured":
                        continue  # keep the measured one
                    if prev is not None and prev["status"] == "measured" and c["status"] == "measured":
                        c = dict(c, invalid="two %s results for %s %s" % (lab, tier, metric))
                    cells[lab][tier][metric] = c
    # the v1 CLI cannot run harnesses that need a TeX binary.
    old_kinds = {i.get("engine_kind") for i in idents["old"] if i.get("engine_kind")}
    for tier, metric in (("latex-suites", "tests"), ("package-smoke", "documents"),
                         ("fonts", "fonts")):
        if tier in cells["new"] and metric not in cells["old"].get(tier, {}) \
                and old_kinds == {"flashtex-cli"}:
            cells["old"].setdefault(tier, {})[metric] = cell("n/a", note=NA_TEX_ONLY)
    all_tiers = set(cells["new"]) | set(cells["old"])
    for t in TIER_ORDER:
        all_tiers.add(t)  # a tier nobody ran is still a row: "missing"
    rows = []
    for tier in sorted(all_tiers, key=lambda t: (TIER_ORDER.index(t) if t in TIER_ORDER else 99, t)):
        metrics = set(cells["new"].get(tier, {})) | set(cells["old"].get(tier, {}))
        if not metrics:
            metrics = {"(any)"}
        for metric in sorted(metrics, key=lambda m: METRIC_ORDER.index(m) if m in METRIC_ORDER else 99):
            new = cells["new"].get(tier, {}).get(metric)
            old = cells["old"].get(tier, {}).get(metric)
            # L4 not rasterised by either engine is not a row anyone measured.
            if metric == "L4" and all(c is None or c["status"] != "measured" for c in (new, old)):
                continue
            same_host = True
            if new and old and new["status"] == old["status"] == "measured":
                same_host = (new.get("host") == old.get("host")
                             and (new.get("oracle") == old.get("oracle")
                                  or None in (new.get("oracle"), old.get("oracle"))))
            v = verdict(tier, metric, new, old, same_host)
            rows.append({"tier": tier, "metric": metric, "new": new, "old": old,
                         "verdict": v, "target": target_of(tier, metric),
                         "gates": gated_stages(tier, metric, stages)})
    partial = sorted({"%s %s: %s" % (r["tier"], r["metric"], c["partial"])
                      for r in rows for c in (r["new"], r["old"]) if c and c.get("partial")})
    samples = sorted({"%s: %s" % (r["tier"], c["sample"])
                      for r in rows for c in (r["new"], r["old"]) if c and c.get("sample")})
    red = [r for r in rows if r["verdict"] not in GREEN]
    all_green = not red and not partial and not samples and not sample_note
    behind_tiers = sorted({r["tier"] for r in rows if r["verdict"] == "behind"})
    stage_rows = []
    for st in stages:
        g = st.get("scoreboard_gate")
        if g is None:
            gate = "none"
        elif g == "all":
            gate = "met" if all_green else "not met"
        else:
            sel = [r for r in rows if "%s:%s" % (r["tier"], r["metric"]) in g]
            gate = "met" if sel and all(r["verdict"] in GREEN for r in sel) and \
                not any(c and c.get("partial") for r in sel for c in (r["new"], r["old"])) \
                and not sample_note else "not met"
        stage_rows.append({"id": st["id"], "name": st["name"], "status": st["status"],
                           "scoreboard_gate": g, "gate_state": gate,
                           "other_preconditions": st.get("other_preconditions")})
    return {"schema": SCHEMA, "host_label": host_label, "sample_note": sample_note,
            "engines": {lab: {"git_sha": shas.get(lab), "runs": idents[lab]} for lab in LABELS},
            "rows": rows, "all_green": all_green, "red": len(red),
            "behind_tiers": behind_tiers, "partial": partial, "samples": samples,
            "retirement": stage_rows,
            "retirement_may_start": all_green}


# --------------------------------------------------------------------------
# rendering


def _engine_line(board, lab):
    e = board["engines"][lab]
    runs = e["runs"]
    vers = sorted({r["engine_version"] for r in runs if r.get("engine_version")})
    hosts = sorted({r.get("host") or "?" for r in runs})
    return "| %s | %s | %s | %s | %s |" % (
        lab, "; ".join(vers) or "—", (e.get("git_sha") or "—"),
        "; ".join(hosts) or "—", ", ".join(sorted({r["source"] for r in runs})) or "—")


def fmt_gates(gates, board):
    """S3, S5, S6, ... S8 -> 'S3, S5+' when every stage from S5 on is gated by every row."""
    every = [s["id"] for s in board["retirement"] if s["scoreboard_gate"] == "all"]
    if every and all(g in gates for g in every):
        rest = [g for g in gates if g not in every]
        return ", ".join(rest + [every[0] + "+"])
    return ", ".join(gates) or "—"


def render_table(board):
    out = ["| tier | metric | new | old | verdict | target | gates |",
           "|---|---|---|---|---|---|---|"]
    for r in board["rows"]:
        v = r["verdict"]
        mark = v if v in GREEN else "**%s**" % v
        out.append("| %s | %s | %s | %s | %s | %s | %s |" % (
            TIER_TITLES.get(r["tier"], r["tier"]), r["metric"], fmt_cell(r["new"]),
            fmt_cell(r["old"]), mark, r["target"], fmt_gates(r["gates"], board)))
    return "\n".join(out)


def render_status(board):
    lines = []
    if board["all_green"]:
        lines.append("**All green**: new >= old on every tier, targets met, every run complete. "
                     "Retirement (S5 onward) may start.")
    else:
        why = []
        if board["red"]:
            why.append("%d row(s) not green" % board["red"])
        if board["partial"] or board["samples"] or board["sample_note"]:
            why.append("partial or sampled runs")
        lines.append("**Not all green** (%s). Retirement from S5 on does not start." % "; ".join(why))
    if board["sample_note"]:
        lines.append("Sample: %s" % board["sample_note"])
    return "\n\n".join(lines)


def render_md(board, title="P5 scoreboard: new engine vs v1, pdflatex as the oracle"):
    out = ["# " + title, "",
           "DESIGN §12 P5 gate: new engine >= old on every tier; arXiv L1 >= 90%; retirement complete. "
           "Expected data is only the oracle's (pdfTeX 1.40.29 / pdflatex).", ""]
    if board.get("host_label"):
        out += ["Measured on **%s**. Host-dependent data: not another host's baseline (DESIGN §8)."
                % board["host_label"], ""]
    out += ["| engine | version | git SHA | host | sources |", "|---|---|---|---|---|",
            _engine_line(board, "new"), _engine_line(board, "old"), "",
            render_status(board), "", render_table(board), ""]
    # one line per (tier, engine, note), listing the metrics it applies to
    grouped = {}
    for r in board["rows"]:
        for lab in LABELS:
            c = r[lab]
            if not c:
                continue
            bits = []
            if c.get("excluded"):
                bits.append("excluded " + ", ".join("%s %d" % kv for kv in sorted(c["excluded"].items())))
            for k in ("note", "partial", "invalid", "sample"):
                if c.get(k):
                    bits.append("%s: %s" % (k, c[k]))
            for bit in bits:
                grouped.setdefault((r["tier"], lab, bit), []).append(r["metric"])
    notes = ["- %s (%s), %s: %s" % (TIER_TITLES.get(t, t), ", ".join(ms), lab, bit)
             for (t, lab, bit), ms in grouped.items()]
    if notes:
        out += ["## Denominators and notes", ""] + notes + [""]
    out += ["## Retirement stages (#1236)", "",
            "Only the scoreboard part of each precondition is evaluated here; the rest is listed.", "",
            "| stage | name | recorded status | scoreboard gate | gate | other preconditions |",
            "|---|---|---|---|---|---|"]
    for s in board["retirement"]:
        g = s["scoreboard_gate"]
        gs = "every row" if g == "all" else (", ".join(g) if g else "—")
        out.append("| %s | %s | %s | %s | %s | %s |" % (s["id"], s["name"], s["status"], gs,
                                                    s["gate_state"], s["other_preconditions"] or "—"))
    return "\n".join(out) + "\n"


def render_summary(board, run_url=None):
    """The short committed summary: status, the table, a link to the artifact."""
    out = ["# P5 scoreboard summary", ""]
    if board.get("host_label"):
        out.append("Host: %s." % board["host_label"])
    shas = ", ".join("%s %s" % (lab, board["engines"][lab].get("git_sha") or "?") for lab in LABELS)
    out += ["Engines: %s." % shas, ""]
    if run_url:
        out += ["Full table and notes: %s" % run_url, ""]
    out += [render_status(board), "", render_table(board), ""]
    return "\n".join(out)


# --------------------------------------------------------------------------
# issues: one per tier where new < old


MARKER = "<!-- p5-scoreboard:tier=%s -->"
MARKER_RE = re.compile(r"<!-- p5-scoreboard:tier=([A-Za-z0-9_.-]+) -->")


def issue_title(tier):
    return "P5 scoreboard: new engine behind v1 on %s" % TIER_TITLES.get(tier, tier)


def issue_body(board, tier, run_url=None):
    rows = [r for r in board["rows"] if r["tier"] == tier]
    out = [MARKER % tier, "",
           "The P5 scoreboard (tools/parity/scoreboard.py) measured the new engine **behind** v1 "
           "on this tier. DESIGN §12 P5 requires new >= old on every tier. This issue is opened and "
           "updated by the nightly scoreboard and closed by it once the tier recovers.", ""]
    if run_url:
        out += ["Run: %s" % run_url, ""]
    out += ["| metric | new | old | verdict |", "|---|---|---|---|"]
    for r in rows:
        out.append("| %s | %s | %s | %s |" % (r["metric"], fmt_cell(r["new"]), fmt_cell(r["old"]),
                                              r["verdict"]))
    shas = ", ".join("%s %s" % (lab, board["engines"][lab].get("git_sha") or "?") for lab in LABELS)
    out += ["", "Engines: %s. Host: %s." % (shas, board.get("host_label") or "?")]
    return "\n".join(out) + "\n"


def run_gh(args, stdin=None):
    p = subprocess.run(["gh"] + args, input=stdin, capture_output=True, text=True)
    if p.returncode != 0:
        raise RuntimeError("gh %s: %s" % (" ".join(args[:3]), p.stderr.strip()[:300]))
    return p.stdout


def plan_issues(board, existing, run_url=None):
    """existing: [{number, title, body}] open issues. -> list of actions."""
    by_tier = {}
    for iss in existing:
        m = MARKER_RE.search(iss.get("body") or "")
        if m:
            by_tier.setdefault(m.group(1), iss)
    actions = []
    behind = set(board["behind_tiers"])
    for tier in sorted(behind):
        body = issue_body(board, tier, run_url)
        if tier in by_tier:
            actions.append(("edit", by_tier[tier]["number"], tier, body))
        else:
            actions.append(("create", None, tier, body))
    measured_tiers = {r["tier"] for r in board["rows"]
                      if r["verdict"] in GREEN and not any(
                          c and c.get("partial") for c in (r["new"], r["old"]))}
    for tier, iss in sorted(by_tier.items()):
        tier_rows = [r for r in board["rows"] if r["tier"] == tier]
        if tier not in behind and tier_rows and all(r["tier"] in measured_tiers for r in tier_rows):
            actions.append(("close", iss["number"], tier,
                            "The P5 scoreboard now measures new >= old on every row of this tier%s. "
                            "Closing; it reopens as a new issue if the tier falls behind again."
                            % ((" (" + run_url + ")") if run_url else "")))
    return actions


def apply_issues(actions, repo, gh=run_gh):
    done = []
    for kind, number, tier, body in actions:
        if kind == "create":
            out = gh(["issue", "create", "--repo", repo, "--title", issue_title(tier),
                      "--body-file", "-"], stdin=body)
            done.append((kind, out.strip(), tier))
        elif kind == "edit":
            gh(["issue", "edit", str(number), "--repo", repo, "--body-file", "-"], stdin=body)
            done.append((kind, number, tier))
        elif kind == "close":
            gh(["issue", "close", str(number), "--repo", repo, "--comment", body])
            done.append((kind, number, tier))
    return done


def existing_issues(repo, gh=run_gh):
    out = gh(["issue", "list", "--repo", repo, "--state", "open", "--limit", "200",
              "--search", "\"p5-scoreboard:tier=\" in:body", "--json", "number,title,body"])
    return json.loads(out or "[]")


# --------------------------------------------------------------------------
# CLI


def _pairs(values, what):
    out = []
    for v in values or ():
        lab, sep, path = v.partition("=")
        if not sep or lab not in LABELS or not path:
            raise SystemExit("--%s wants LABEL=PATH with LABEL in %s, got %r" % (what, LABELS, v))
        out.append((lab, path))
    return out


def gather(args):
    sizes = {}
    for d in args.manifests or [MANIFEST_DIR]:
        for tier, n in manifest_sizes(d).items():
            sizes[tier] = max(n, sizes.get(tier, 0))
    sources = {lab: [] for lab in LABELS}
    for lab, path in _pairs(args.parity, "parity"):
        tiers, ident = load_parity(path, sizes)
        sources[lab].append(("parity", tiers, ident))
    for lab, path in _pairs(args.nightly, "nightly"):
        tiers, ident = load_nightly(path, sizes)
        sources[lab].append(("nightly", tiers, ident))
    reference = None
    if args.latex_suites_reference:
        with open(args.latex_suites_reference, encoding="utf-8", errors="replace") as f:
            reference = f.read()
    for lab, path in _pairs(args.latex_suites, "latex-suites"):
        with open(path, encoding="utf-8", errors="replace") as f:
            sources[lab].append(("latex-suites", {"latex-suites": parse_latex_suites(f.read(), reference)},
                                 {"host": args.host}))
    for lab, path in _pairs(args.package_smoke, "package-smoke"):
        with open(path, encoding="utf-8", errors="replace") as f:
            sources[lab].append(("package-smoke", {"package-smoke": parse_package_smoke(
                f.read(), smoke_documents())},
                                 {"host": args.host}))
    for lab, path in _pairs(args.fonts, "fonts"):
        sources[lab].append(("fonts", {"fonts": load_fonts(path)}, {"host": args.host}))
    return sources


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0],
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--parity", action="append", metavar="LABEL=DIR")
    ap.add_argument("--nightly", action="append", metavar="LABEL=DIR")
    ap.add_argument("--latex-suites", action="append", metavar="LABEL=FILE")
    ap.add_argument("--latex-suites-reference", metavar="FILE",
                    help="run.py transcript of the same suites through this host's pdfTeX: the T2 "
                         "baseline when the host's TeX Live is not the pinned one")
    ap.add_argument("--package-smoke", action="append", metavar="LABEL=FILE")
    ap.add_argument("--fonts", action="append", metavar="LABEL=DIR")
    ap.add_argument("--sha", action="append", metavar="LABEL=SHA")
    ap.add_argument("--host", default=platform.node(),
                    help="host of the text-output harnesses (latex-suites, package-smoke, fonts); "
                         "default this machine's node name, as parity.py records it")
    ap.add_argument("--host-label")
    ap.add_argument("--sample-note", help="mark the whole board as a sample (never all-green)")
    ap.add_argument("--manifests", action="append", metavar="DIR",
                    help="corpus manifest directory (repeatable; default tools/parity/corpus): "
                         "a tier run on fewer documents than its manifest lists is partial")
    ap.add_argument("--stages", default=STAGES_FILE)
    ap.add_argument("--out", required=True)
    ap.add_argument("--summary", help="also write the short committed summary here")
    ap.add_argument("--run-url")
    ap.add_argument("--issues", choices=("off", "dry-run", "apply"), default="off")
    ap.add_argument("--repo", default="flash-tex/flashtex")
    ap.add_argument("--require-green", action="store_true", help="exit 1 unless the board is all-green")
    args = ap.parse_args(argv)
    shas = dict(_pairs(args.sha, "sha"))
    board = build(gather(args), shas=shas, sample_note=args.sample_note,
                  stages=load_stages(args.stages), host_label=args.host_label)
    os.makedirs(args.out, exist_ok=True)
    with open(os.path.join(args.out, "scoreboard.json"), "w", encoding="utf-8") as f:
        json.dump(board, f, indent=1, sort_keys=True)
        f.write("\n")
    with open(os.path.join(args.out, "scoreboard.md"), "w", encoding="utf-8") as f:
        f.write(render_md(board))
    if args.summary:
        with open(args.summary, "w", encoding="utf-8") as f:
            f.write(render_summary(board, args.run_url))
    print(render_status(board))
    print(render_table(board))
    if args.issues != "off":
        existing = existing_issues(args.repo) if args.issues == "apply" or shutil.which("gh") else []
        actions = plan_issues(board, existing, args.run_url)
        for kind, number, tier, _ in actions:
            print("issue %s %s %s" % (kind, number or "(new)", tier))
        if args.issues == "apply":
            apply_issues(actions, args.repo)
    return 1 if args.require_green and not board["all_green"] else 0


if __name__ == "__main__":
    sys.exit(main())
