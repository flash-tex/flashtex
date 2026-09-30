#!/usr/bin/env python3
"""DESIGN §8 T4: the nightly ~5,000-document corpus, sharded and ratcheted.

Oracle tooling only (standard library). This is a driver around
`parity.py`, not a second harness: every document is scored by
`parity.py --shard K/N`, exactly as the T3 tiers are, against the pinned
pdfTeX and pdflatex **of the host it runs on**. Nothing expected is
committed (DESIGN §8): the references are made by that host's oracle and
cached by source hash; the ratchet baseline is recorded on that host, carries
its label, and is refused on any other.

    # score every shard not yet done (resumable), merge, write the artifact
    python3 tools/parity/nightly.py run --host-label nixos-7800x3d --shards 50 \\
        --tier nightly-5k --tier arxiv --tier templates --tier packages \\
        --pt1-sample nightly-5k=0.05 --engine <flashtex-initex> --engine-env FLASHTEX_FORMATS=<dir> ... \\
        --out <artifact dir>
    # the ratchet: no document below its recorded level (exit 1 on a regression)
    python3 tools/parity/nightly.py ratchet --host-label nixos-7800x3d --results <artifact dir>
    # record the baseline (manual workflow_dispatch only, never automatic)
    python3 tools/parity/nightly.py ratchet --host-label nixos-7800x3d --results <artifact dir> --record
    # the new engine's formats, the fmtutil way
    python3 tools/parity/nightly.py make-formats --engine <flashtex-initex> --pool <pdftex.pool> --out <dir>

State lives outside the job's working directory (which a runner wipes per
job), in `--state` (default `$FLASHTEX_NIGHTLY_HOME`, else
`~/.cache/flashtex-nightly`):

    runs/<run key>/run.json          what the key hashes (engine, harness, manifests, settings)
    runs/<run key>/shard-KK/         parity.py's scoreboard.json + documents.json for shard K
    baseline/<host label>.json       the ratchet baseline, recorded on that host only
    baseline/<host label>-<utc>.json every baseline it replaced

A run key is the SHA-256 of the engine binary, the harness sources, the
manifests and the settings, so a job that stops (timeout, reboot) and is
started again with the same inputs resumes at the first unfinished shard.

Per document, the artifact records the P-T1 / P-T2 result, the cumulative
level L0-L4 and, for every document that is not a full pass, a root-cause
class (the classes of the arXiv scoreboard, tools/parity/reports/):

  a  a package or font the engine could not find (DESIGN §4.4)
  b  an engine difference: the first differing log, box or content line
  c  a harness issue in tools/parity (fetch failure, harness exception,
     an unfinished shard)
  d  pdflatex fails too: excluded from the denominator
  e  excluded by the convergence rule: pdflatex compiles it but its log asks
     for a rerun on every pass
"""

import argparse
import collections
import glob
import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, HERE)

import corpus as pcorpus  # noqa: E402

LEVELS = ["L0", "L1", "L2", "L3", "L4"]
CLASSES = {"a": "package or font the engine could not find (§4.4)",
           "b": "engine difference",
           "c": "harness issue (tools/parity)",
           "d": "pdflatex fails too (excluded)",
           "e": "excluded by the convergence rule: pdflatex compiles it but keeps asking for a rerun"}
MISSING_FILE = re.compile(r"File `([^']+)' not found|Font \\?(\S+)=?\S* not loadable|"
                          r"I can't find file `([^']+)'")
BASELINE_SCHEMA = "flashtex-nightly-baseline/1"
# settings that make expected data: a baseline recorded under other values is
# another measurement, and the ratchet refuses it rather than compare
FINGERPRINT_KEYS = ("oracle_pdftex_version", "pdflatex", "shell_escape", "argv0", "pt", "levels", "pt1_sample",
                    "pt1_sample_seed")


def default_state():
    return os.environ.get("FLASHTEX_NIGHTLY_HOME") or os.path.join(os.path.expanduser("~"), ".cache",
                                                                    "flashtex-nightly")


def log(msg):
    print(msg, file=sys.stderr, flush=True)


def sha_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def write_json(path, obj):
    tmp = f"{path}.{os.getpid()}.tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(obj, f, indent=1, ensure_ascii=False, sort_keys=True)
        f.write("\n")
    os.replace(tmp, path)


def read_json(path):
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def git_sha():
    if os.environ.get("GITHUB_SHA"):
        return os.environ["GITHUB_SHA"]
    try:
        return subprocess.run(["git", "-C", REPO, "rev-parse", "HEAD"], capture_output=True, text=True,
                              timeout=30).stdout.strip() or None
    except (OSError, subprocess.SubprocessError):
        return None


# ----------------------------------------------------------------------------
# formats for the new engine (PR #1198's candidate invocation)


def cmd_make_formats(args):
    """pdflatex.fmt and pdftex.fmt for `--engine`, built by the engine itself
    from the host's TeX Live, the way fmtutil does."""
    os.makedirs(args.out, exist_ok=True)
    env = dict(os.environ, FLASHTEX_POOL=os.path.abspath(args.pool))
    for job, ini in (("pdflatex", "pdflatex.ini"), ("pdftex", "pdfetex.ini")):
        p = subprocess.run([args.engine, "-ini", f"-jobname={job}", f"-progname={job}",
                            "-translate-file=cp227.tcx", "*" + ini], cwd=args.out, env=env,
                           stdin=subprocess.DEVNULL, capture_output=True, timeout=600)
        fmt = os.path.join(args.out, job + ".fmt")
        if p.returncode != 0 or not os.path.isfile(fmt):
            tail = (p.stdout + p.stderr).decode("utf-8", "replace")[-1500:]
            log(f"{job}.fmt: engine exit {p.returncode}\n{tail}")
            return 1
        log(f"{fmt}: {os.path.getsize(fmt)} bytes")
    return 0


# ----------------------------------------------------------------------------
# run: every shard, resumable


HARNESS_FILES = ("parity.py", "tiers.py", "capture.py", "corpus.py", "glyphkeys.py", "definers.py", "nightly.py")


def run_key(args, tiers, manifests):
    inputs = {
        "engine_sha256": sha_file(args.engine),
        "engine_env": sorted(args.engine_env),
        "harness": {f: sha_file(os.path.join(HERE, f)) for f in HARNESS_FILES if os.path.isfile(os.path.join(HERE, f))},
        "manifests": {os.path.basename(m): sha_file(m) for m in manifests},
        "tiers": tiers, "shards": args.shards, "pt1_sample": sorted(args.pt1_sample), "spread": args.spread,
        "parity_args": args.parity_args, "host_label": args.host_label,
    }
    return hashlib.sha256(json.dumps(inputs, sort_keys=True).encode()).hexdigest()[:20], inputs


def parse_shards(text, n):
    if not text:
        return list(range(n))
    lo, _, hi = text.partition("-")
    return list(range(int(lo), (int(hi) if hi else int(lo)) + 1))


def cmd_run(args):
    known = ["fixtures"] + pcorpus.manifest_tiers()
    tiers = []
    for t in args.tier:
        if t in known:
            tiers.append(t)
        else:
            # e.g. `packages` before its manifest lands: said, not fatal
            print(f"::notice::tier {t!r} has no manifest in tools/parity/corpus yet; skipped")
    if not tiers:
        log("no tier to run")
        return 2
    manifests = [m for m in pcorpus.manifests(include_on_demand=True) if pcorpus.manifest_tier(m) in tiers]
    key, inputs = run_key(args, tiers, manifests)
    rdir = os.path.join(args.state, "runs", key)
    os.makedirs(rdir, exist_ok=True)
    write_json(os.path.join(rdir, "run.json"), dict(inputs, key=key, engine=os.path.abspath(args.engine)))
    log(f"run {key}: {len(tiers)} tiers ({' '.join(tiers)}), {args.shards} shards, state {rdir}")
    started = time.time()
    todo = parse_shards(args.shard_range, args.shards)
    for k in todo:
        sdir = os.path.join(rdir, f"shard-{k:03d}")
        if os.path.isfile(os.path.join(sdir, "done.json")):
            continue
        if args.deadline_minutes and time.time() - started > 60 * args.deadline_minutes:
            log(f"deadline of {args.deadline_minutes} min reached before shard {k}; re-run to resume")
            break
        tmp = sdir + ".partial"
        shutil.rmtree(tmp, ignore_errors=True)
        work = os.path.join(args.work, f"shard-{k:03d}")
        argv = [sys.executable, os.path.join(HERE, "parity.py"), "--shard", f"{k}/{args.shards}",
                "--engine", args.engine, "--out", tmp, "--work", work, "-j", str(args.jobs)]
        for t in tiers:
            argv += ["--tier", t]
        for e in args.engine_env:
            argv += ["--engine-env", e]
        for s in args.pt1_sample:
            argv += ["--pt1-sample", s]
        if args.spread:
            argv += ["--spread", str(args.spread)]
        argv += args.parity_args
        t0 = time.time()
        log(f"shard {k}/{args.shards}: " + " ".join(argv[2:]))
        p = subprocess.run(argv, cwd=REPO)
        shutil.rmtree(work, ignore_errors=True)
        if p.returncode != 0 or not os.path.isfile(os.path.join(tmp, "documents.json")):
            log(f"shard {k}: parity.py exit {p.returncode}; left unfinished (class c), the next run retries it")
            if p.returncode == 2:  # usage error, missing engine: every shard would fail the same way
                return 2
            continue
        write_json(os.path.join(tmp, "done.json"), {"shard": k, "of": args.shards, "exit": p.returncode,
                                                     "seconds": round(time.time() - t0, 1)})
        shutil.rmtree(sdir, ignore_errors=True)
        os.replace(tmp, sdir)
    return summarize_run(rdir, args, tiers)


# ----------------------------------------------------------------------------
# classification and merge


def full_pass(r):
    pt = r.get("pt") or {}
    return (r.get("level") == len(LEVELS) - 1 and pt.get("P-T1") in (True, None)
            and pt.get("P-T2") in (True, None))


def first_difference(r):
    """The first differing line the run recorded, as one string."""
    pt = r.get("pt") or {}
    pt1, pt2 = pt.get("pt1") or {}, pt.get("pt2") or {}
    if pt.get("P-T1") is False:
        if pt1.get("log_line"):
            d = pt1["log_line"]
            return f"P-T1 log line {d['line']}: oracle `{d['oracle']}` vs `{d['candidate']}`"
        if pt1.get("box_line"):
            d = pt1["box_line"]
            return (f"P-T1 shipout {pt1.get('first_shipout')} box line {d['line']}: "
                    f"oracle `{d['oracle']}` vs `{d['candidate']}`")
        if pt1.get("why"):
            return "P-T1: " + pt1["why"]
    if pt.get("P-T2") is False:
        if pt2.get("why"):
            return "P-T2: " + pt2["why"]
        if not pt2.get("fonts_equal", True):
            f = pt2.get("fonts") or {}
            return (f"P-T2 fonts: missing {f.get('missing')}, extra {f.get('extra')}, "
                    f"program {f.get('different_program')}")
        fp = pt2.get("first_page") or {}
        s = f"P-T2 page {fp.get('page')}: {', '.join(fp.get('differs') or [])}"
        if fp.get("content_line"):
            d = fp["content_line"]
            s += f"; content line {d['line']}: oracle `{d['oracle']}` vs `{d['candidate']}`"
        return s
    if r.get("level") is not None and r["level"] < len(LEVELS) - 1:
        nxt = LEVELS[r["level"] + 1]
        return f"{nxt}: " + ("; ".join((r.get("blockers") or [])[:2]) or (r.get("divergence") or "fails"))
    return None


def classify(r):
    """(class, cause) for one record that is not a full pass."""
    ex = r.get("excluded")
    if ex:
        if "did not converge" in ex:
            return "e", ex
        if ex.startswith("oracle"):
            return "d", ex
        return "c", ex
    cand = r.get("candidate") or {}
    pt = r.get("pt") or {}
    if (pt.get("excluded") or "").startswith("oracle"):
        return ("e" if "did not converge" in pt["excluded"] else "d"), "P-T oracle pdfTeX: " + pt["excluded"]
    if r.get("level") == -1:
        why = " ".join(str(cand.get(k) or "") for k in ("status", "stderr_tail"))
        m = MISSING_FILE.search(why)
        if m:
            return "a", "missing " + next(g for g in m.groups() if g)
        return "b", "L0: " + (why.strip()[:200] or "no PDF")
    if pt.get("P-T1") in (True, None) and pt.get("P-T2") in (True, None) and pt.get("P-T1") is not None:
        # identical traces and PDFs, yet a level fails: the measurement is wrong
        return "c", f"P-T1 and P-T2 pass but level {LEVELS[r['level']] if r['level'] >= 0 else 'below L0'}"
    return "b", first_difference(r) or f"level {r.get('level')}"


def signature(cause):
    """A cause with its numbers and quoted specifics folded, so the same
    difference in many documents ranks as one root cause."""
    s = re.sub(r"(?<!P-T)(?<!\bL)\d+(\.\d+)?", "#", cause or "")  # keeps P-T1, P-T2, L0-L4
    s = re.sub(r"(log|box|content) line #", r"\1 line", s)
    return s[:160]


def level_name(r):
    if r.get("excluded"):
        return None
    lv = r.get("level")
    return None if lv is None else ("below L0" if lv < 0 else LEVELS[lv])


def compact(r):
    """The artifact's per-document record: results and causes, no logs."""
    pt = r.get("pt") or {}
    out = {"id": r["id"], "tier": r["tier"], "category": r.get("category"), "level": level_name(r),
           "level_index": None if r.get("excluded") else r.get("level"),
           "P-T1": pt.get("P-T1"), "P-T2": pt.get("P-T2"), "seconds": r.get("seconds")}
    if r.get("excluded"):
        out["excluded"] = r["excluded"][:300]
    if pt.get("P-T1") is None and not r.get("excluded"):
        why = (pt.get("why") or {}).get("P-T1")
        if why:
            out["pt1_not_evaluated"] = why[:200]
    if not full_pass(r):
        cls, cause = classify(r)
        out["class"], out["cause"] = cls, (cause or "")[:400]
        fd = first_difference(r)
        if fd:
            out["first_difference"] = fd[:400]
    if r.get("blocker_groups"):
        out["blockers"] = r["blocker_groups"][:5]
    if r.get("first_diverging_page"):
        out["first_diverging_page"] = r["first_diverging_page"]
    return out


def tier_row(recs):
    """P-T1, P-T2 and cumulative L0-L4 as [passed, of] over the documents the
    oracle compiles; None when not evaluated."""
    measured = [r for r in recs if not r.get("excluded")]
    row = {"documents": len(recs), "measured": len(measured),
           "excluded": dict(collections.Counter(r["excluded"].split(":", 1)[0] for r in recs if r.get("excluded")))}
    for t in ("P-T1", "P-T2"):
        ev = [r for r in measured if r.get(t) is not None]
        row[t] = [sum(1 for r in ev if r[t]), len(ev)] if ev else None
        if t == "P-T1":
            row["P-T1_not_evaluated"] = sum(1 for r in measured if r.get("pt1_not_evaluated"))
    for k, name in enumerate(LEVELS):
        row[name] = [sum(1 for r in measured if (r.get("level_index") if r.get("level_index") is not None
                                                 else -1) >= k), len(measured)]
    return row


def cell(v):
    if not v:
        return "n/a"
    return f"{v[0]}/{v[1]} ({100.0 * v[0] / v[1]:.1f}%)" if v[1] else "0/0"


def summarize_run(rdir, args, tiers):
    shard_dirs = sorted(glob.glob(os.path.join(rdir, "shard-*")))
    done = [d for d in shard_dirs if os.path.isfile(os.path.join(d, "done.json"))]
    boards, docs = [], []
    for d in done:
        boards.append(read_json(os.path.join(d, "scoreboard.json")))
        for t, rs in read_json(os.path.join(d, "documents.json")).items():
            docs += rs
    missing = sorted(set(range(args.shards)) - {read_json(os.path.join(d, "done.json"))["shard"] for d in done})
    import parity  # noqa: E402  (the grouping of blockers across shards is parity.rank_causes)
    causes = parity.rank_causes(docs, field="blocker_groups", top=15)
    recs = [compact(r) for r in sorted(docs, key=lambda r: (r["tier"], r["id"]))]
    meta = boards[0]["meta"] if boards else {}
    host = {"label": args.host_label, "node": platform.node(),
            "platform": f"{platform.system()} {platform.release()} {platform.machine()}"}
    fingerprint = {k: meta.get(k) for k in FINGERPRINT_KEYS}
    by_tier = collections.defaultdict(list)
    for r in recs:
        by_tier[r["tier"]].append(r)
    classes = collections.Counter(r["class"] for r in recs if r.get("class"))
    roots = collections.defaultdict(lambda: {"documents": 0, "class": None, "examples": []})
    for r in recs:
        if r.get("class") in ("a", "b", "c"):
            s = roots[(r["class"], signature(r["cause"]))]
            s["documents"] += 1
            s["class"] = r["class"]
            if len(s["examples"]) < 4:
                s["examples"].append(f"{r['tier']}/{r['id']}")
    top = sorted(({"class": c, "cause": sig, **v} for (c, sig), v in roots.items()),
                 key=lambda x: (-x["documents"], x["class"], x["cause"]))[:15]
    summary = {
        "schema": "flashtex-nightly/1", "run_key": os.path.basename(rdir), "host": host,
        "git_sha": git_sha(), "fingerprint": fingerprint,
        "engine": {"path": meta.get("flashtex"), "version": meta.get("engine_version"),
                   "kind": meta.get("engine_kind"), "sha256": meta.get("flashtex_sha256")},
        "shards": {"total": args.shards, "done": len(done), "missing": missing},
        "tiers": {t: tier_row(by_tier.get(t, [])) for t in tiers},
        "classes": {k: classes.get(k, 0) for k in CLASSES},
        "top_root_causes": top, "blocker_groups": causes,
        "generated_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    }
    os.makedirs(args.out, exist_ok=True)
    write_json(os.path.join(args.out, "summary.json"), summary)
    write_json(os.path.join(args.out, "documents.json"), {"schema": "flashtex-nightly-documents/1",
                                                          "run_key": summary["run_key"], "host": host,
                                                          "documents": recs})
    with open(os.path.join(args.out, "summary.md"), "w", encoding="utf-8") as f:
        f.write(render_summary(summary))
    log(f"summary: {os.path.join(args.out, 'summary.md')} ({len(recs)} documents, "
        f"{len(done)}/{args.shards} shards)")
    if missing:
        print(f"::error::{len(missing)} of {args.shards} shards unfinished ({missing[:10]}...); "
              "re-run with the same inputs to resume")
        return 4
    return 0


def render_summary(s):
    L = []
    w = L.append
    fp = s["fingerprint"]
    w(f"# Nightly corpus (DESIGN §8 T4) on {s['host']['label']}")
    w("")
    w(f"Measured on **{s['host']['label']}** ({s['host']['node']}, {s['host']['platform']}). Host-dependent data: "
      "it is no other host's baseline (DESIGN §8).")
    w("")
    e = s["engine"]
    w(f"- engine: `{e['version']}` ({e['kind']}), sha256 `{(e['sha256'] or '')[:12]}`, git `{(s['git_sha'] or '')[:12]}`")
    w(f"- oracle: `{fp.get('oracle_pdftex_version')}`; pdflatex `{fp.get('pdflatex')}`; \\write18 "
      f"`{fp.get('shell_escape')}`")
    w(f"- P-T1 sample: `{fp.get('pt1_sample') or 'every document'}` (seed `{fp.get('pt1_sample_seed')}`); "
      "P-T2 and L0–L4 on every document")
    w(f"- shards: {s['shards']['done']}/{s['shards']['total']} done; run key `{s['run_key']}`")
    w("")
    w("| tier | documents | excluded | P-T1 | P-T2 | L0 | L1 | L2 | L3 | L4 |")
    w("|---|---:|---|---|---|---|---|---|---|---|")
    for t, row in s["tiers"].items():
        ex = ", ".join(f"{k} {v}" for k, v in sorted(row["excluded"].items())) or "—"
        w(f"| {t} | {row['documents']} | {ex} | {cell(row['P-T1'])} | {cell(row['P-T2'])} | "
          + " | ".join(cell(row[n]) for n in LEVELS) + " |")
    w("")
    w("P-T1 not evaluated (outside the sample, or not run): "
      + ", ".join(f"{t} {row['P-T1_not_evaluated']}" for t, row in s["tiers"].items()) + ".")
    w("")
    w("## Every document that is not P-T1 + P-T2 + L4, by class")
    w("")
    for k, v in s["classes"].items():
        w(f"- ({k}) {CLASSES[k]}: **{v}**")
    w("")
    w("## Top root causes (classes a–c, numbers folded)")
    w("")
    w("| # | documents | class | cause | examples |")
    w("|---:|---:|---|---|---|")
    for i, c in enumerate(s["top_root_causes"], 1):
        w(f"| {i} | {c['documents']} | {c['class']} | `{c['cause'].replace('|', '/').replace('`', '')}` | "
          + ", ".join(c["examples"]) + " |")
    w("")
    if s["blocker_groups"]:
        w("## Blockers below L4, grouped by definer (parity.rank_causes)")
        w("")
        w("| # | cause | documents | sole | next level |")
        w("|---:|---|---:|---:|---|")
        for c in s["blocker_groups"]:
            w(f"| {c['rank']} | `{c['cause'].replace('|', '/')}` | {c['documents']} | {c['sole']} | "
              + ", ".join(f"{k}: {v}" for k, v in sorted(c["next_level"].items(), key=str)) + " |")
        w("")
    w("Per-document results: `documents.json` in this artifact.")
    w("")
    return "\n".join(L)


# ----------------------------------------------------------------------------
# the ratchet


def doc_key(r):
    return f"{r['tier']}/{r['id']}"


def cmd_ratchet(args):
    summary = read_json(os.path.join(args.results, "summary.json"))
    docs = read_json(os.path.join(args.results, "documents.json"))["documents"]
    if summary["host"]["label"] != args.host_label:
        log(f"refusing: these results were measured on {summary['host']['label']!r}, not {args.host_label!r}")
        return 2
    path = args.baseline or os.path.join(args.state, "baseline", f"{args.host_label}.json")
    if args.record:
        return record_baseline(path, summary, docs, args)
    if not os.path.isfile(path):
        print(f"::error::no ratchet baseline for host {args.host_label!r} at {path}. Record it ON THIS HOST "
              "with a manual workflow_dispatch (record_baseline: true); it is never seeded from another host.")
        return 1
    base = read_json(path)
    if base.get("schema") != BASELINE_SCHEMA or base.get("host_label") != args.host_label:
        log(f"refusing: {path} is the baseline of {base.get('host_label')!r}, not {args.host_label!r} (DESIGN §8)")
        return 2
    changed = {k: [base["fingerprint"].get(k), summary["fingerprint"].get(k)] for k in FINGERPRINT_KEYS
               if base["fingerprint"].get(k) != summary["fingerprint"].get(k)}
    if changed:
        log("refusing: the oracle or the measurement changed since the baseline was recorded, so the expected "
            f"data is not the same: {json.dumps(changed)}. Re-record deliberately (workflow_dispatch).")
        return 2
    now = {doc_key(r): r for r in docs}
    regressions, improvements, unmeasured = [], [], []
    for key, b in sorted(base["documents"].items()):
        r = now.get(key)
        if b["level"] is None and b.get("P-T2") is None:
            continue  # excluded when recorded: nothing to hold
        if r is None or (r.get("excluded") or "").startswith("fetch:"):
            unmeasured.append(key)
            continue
        cur = r.get("level_index")
        why = []
        if b["level"] is not None and (cur is None or cur < b["level"]):
            why.append(f"level {name_of(b['level'])} -> {name_of(cur)}")
        for t in ("P-T2", "P-T1"):
            if b.get(t) is True and r.get(t) is False:
                why.append(f"{t} pass -> fail")
            elif b.get(t) is True and r.get(t) is None and t == "P-T2":
                why.append("P-T2 pass -> not evaluated")
        if why:
            regressions.append({"document": key, "was": b, "now": {"level": cur, "P-T1": r.get("P-T1"),
                                                                    "P-T2": r.get("P-T2")},
                                "why": why, "cause": r.get("cause") or r.get("excluded")})
        elif rank(cur) > rank(b["level"]) or \
                any(b.get(t) is False and r.get(t) is True for t in ("P-T1", "P-T2")):
            improvements.append(key)
    out = {"baseline": {"path": path, "recorded_utc": base.get("recorded_utc"), "run_key": base.get("run_key"),
                        "git_sha": base.get("git_sha")},
           "held": len(base["documents"]), "regressions": regressions, "improvements": len(improvements),
           "improved_examples": improvements[:20], "unmeasured": unmeasured}
    write_json(os.path.join(args.results, "ratchet.json"), out)
    with open(os.path.join(args.results, "summary.md"), "a", encoding="utf-8") as f:
        f.write(f"\n## Ratchet against the {args.host_label} baseline ({base.get('recorded_utc')})\n\n")
        f.write(f"- {len(regressions)} regressions, {len(improvements)} improvements, "
                f"{len(unmeasured)} unmeasured (source unavailable or shard unfinished) of "
                f"{len(base['documents'])} recorded documents.\n")
        for g in regressions[:50]:
            f.write(f"- **{g['document']}**: {'; '.join(g['why'])} — {(g['cause'] or '')[:160]}\n")
        if improvements:
            f.write("- Improvements are not recorded automatically; a person re-records the baseline "
                    "(workflow_dispatch, record_baseline: true).\n")
    for g in regressions:
        print(f"::error::ratchet: {g['document']}: {'; '.join(g['why'])}")
    log(f"ratchet: {len(regressions)} regressions, {len(improvements)} improvements, {len(unmeasured)} unmeasured")
    return 1 if regressions else 0


def rank(level):
    """A level as a number for comparison: excluded (None) is below below-L0."""
    return -2 if level is None else level


def name_of(level):
    return "excluded" if level is None else ("below L0" if level < 0 else LEVELS[level])


def record_baseline(path, summary, docs, args):
    if summary["shards"]["missing"] and not args.allow_partial:
        log(f"refusing to record from an unfinished run ({len(summary['shards']['missing'])} shards missing)")
        return 2
    os.makedirs(os.path.dirname(path), exist_ok=True)
    stamp = time.strftime("%Y%m%dT%H%M%SZ", time.gmtime())
    if os.path.isfile(path):
        old = read_json(path)
        shutil.copyfile(path, path[:-5] + f"-{stamp}.json")
        lowered = sum(1 for r in docs if doc_key(r) in old["documents"]
                      and rank(old["documents"][doc_key(r)]["level"]) > rank(r.get("level_index")))
        log(f"replacing the baseline of {old.get('recorded_utc')}; {lowered} documents are recorded lower")
    base = {"schema": BASELINE_SCHEMA, "host_label": args.host_label, "host": summary["host"],
            "recorded_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), "run_key": summary["run_key"],
            "git_sha": summary["git_sha"], "engine": summary["engine"], "fingerprint": summary["fingerprint"],
            "documents": {doc_key(r): {"level": r.get("level_index"), "P-T1": r.get("P-T1"), "P-T2": r.get("P-T2")}
                          for r in docs}}
    write_json(path, base)
    shutil.copyfile(path, os.path.join(args.results, "baseline.json"))
    log(f"recorded {path}: {len(docs)} documents on {args.host_label}")
    return 0


# ----------------------------------------------------------------------------


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--state", default=default_state())
    sub = ap.add_subparsers(dest="cmd", required=True)
    r = sub.add_parser("run", help="score every unfinished shard, then merge and write the artifact")
    r.add_argument("--host-label", required=True)
    r.add_argument("--tier", action="append", default=[])
    r.add_argument("--shards", type=int, default=50)
    r.add_argument("--shard-range", default=None, metavar="A-B", help="only shards A..B (inclusive)")
    r.add_argument("--engine", required=True)
    r.add_argument("--engine-env", action="append", default=[], metavar="KEY=VALUE")
    r.add_argument("--pt1-sample", action="append", default=[], metavar="TIER=FRACTION")
    r.add_argument("--spread", type=int, default=0, help="N documents per tier, evenly spaced (local proofs)")
    r.add_argument("-j", "--jobs", type=int, default=max(1, (os.cpu_count() or 2) // 2))
    r.add_argument("--work", default=None, help="scratch for the candidate runs (default <state>/work)")
    r.add_argument("--deadline-minutes", type=float, default=0, help="start no shard after this long")
    r.add_argument("--out", required=True, help="artifact directory")
    r.add_argument("parity_args", nargs=argparse.REMAINDER,
                   help="after --: passed to parity.py unchanged (--texbin, --oracle-pdftex, --texmf, --cache, ...)")
    k = sub.add_parser("ratchet", help="check (or --record) the host baseline")
    k.add_argument("--host-label", required=True)
    k.add_argument("--results", required=True, help="the artifact directory `run` wrote")
    k.add_argument("--baseline", default=None, help="default <state>/baseline/<host label>.json")
    k.add_argument("--record", action="store_true", help="record the baseline from these results (manual only)")
    k.add_argument("--allow-partial", action="store_true")
    f = sub.add_parser("make-formats", help="pdflatex.fmt and pdftex.fmt for the new engine")
    f.add_argument("--engine", required=True)
    f.add_argument("--pool", required=True)
    f.add_argument("--out", required=True)
    args = ap.parse_args(argv)
    if args.cmd == "run":
        if args.parity_args[:1] == ["--"]:
            args.parity_args = args.parity_args[1:]
        args.engine = os.path.abspath(args.engine)
        args.work = args.work or os.path.join(args.state, "work")
        return cmd_run(args)
    if args.cmd == "ratchet":
        return cmd_ratchet(args)
    return cmd_make_formats(args)


if __name__ == "__main__":
    sys.exit(main())
