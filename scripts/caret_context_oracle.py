#!/usr/bin/env python3
"""pdflatex oracle for protocol/fixtures/caret-context-v1.json.

For every case it builds the document the caret lives in and compiles it twice:

  normalised: preamble + before + <insert> + after   -- MUST compile
  naive:      preamble + before + <proposal> + after -- what shipping today does

A case whose naive form fails and whose normalised form compiles is a rule that
earns its keep; the report names them. Cases where both compile are
no-regression checks: the rule must not break LaTeX that was already fine.

Usage: scripts/caret_context_oracle.py [--fixture PATH] [--flashtex BIN]
Exit status is non-zero if any normalised insertion fails to compile.
"""
import argparse, json, pathlib, shutil, subprocess, sys, tempfile

DEFAULT_FIXTURE = pathlib.Path(__file__).resolve().parent.parent / "protocol/fixtures/caret-context-v1.json"


def compile_with(engine, body, preamble, workdir):
    src = preamble + "\\begin{document}\n" + body + "\n\\end{document}\n"
    d = pathlib.Path(tempfile.mkdtemp(dir=workdir))
    (d / "main.tex").write_text(src)
    if engine == "pdflatex":
        r = subprocess.run(["pdflatex", "-interaction=nonstopmode", "-halt-on-error", "main.tex"],
                           cwd=d, capture_output=True, text=True)
        if r.returncode == 0:
            return True, ""
        for line in r.stdout.splitlines():
            if line.startswith("!"):
                return False, line.strip()[:90]
        return False, "pdflatex failed"
    r = subprocess.run([engine, "--tex", str(d / "main.tex"), "--v2", str(d / "out.json")],
                       capture_output=True, text=True)
    out = d / "out.json"
    if r.returncode != 0 or not out.exists():
        return False, (r.stderr.strip().splitlines() or ["no output"])[-1][:90]
    diags = json.loads(out.read_text()).get("diagnostics", [])
    errs = [x for x in diags if x.get("severity") == "error"]
    return (not errs), ("; ".join(x.get("message", "")[:80] for x in errs[:2]))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--fixture", type=pathlib.Path, default=DEFAULT_FIXTURE)
    ap.add_argument("--flashtex", default=shutil.which("flashtex-render"))
    args = ap.parse_args()
    data = json.loads(args.fixture.read_text())
    default_preamble = data["preamble"]
    engines = ["pdflatex"] + ([args.flashtex] if args.flashtex else [])
    rows, failures, earned = [], [], 0
    with tempfile.TemporaryDirectory() as workdir:
        for case in data["cases"]:
            pre = case.get("preamble", default_preamble)
            before, after = case["before"], case["after"]
            insert, proposal = case["insert"], case["proposal"]
            row = {"name": case["name"], "engines": {}}
            for engine in engines:
                label = "pdflatex" if engine == "pdflatex" else "flashtex"
                naive_ok, naive_why = compile_with(engine, before + proposal + after, pre, workdir)
                if insert is None:
                    # A refused insertion: nothing is written to the document, so
                    # the only claim to check is that the naive text was unsafe.
                    row["engines"][label] = {"naive": naive_ok, "naive_why": naive_why, "normalised": None}
                    continue
                ok, why = compile_with(engine, before + insert + after, pre, workdir)
                row["engines"][label] = {"naive": naive_ok, "naive_why": naive_why, "normalised": ok, "why": why}
                if label == "pdflatex":
                    if not ok:
                        failures.append(f"{case['name']}: normalised insertion does not compile: {why}")
                    if ok and not naive_ok:
                        earned += 1
            rows.append(row)

    width = max(len(r["name"]) for r in rows) + 2
    print(f"{'case'.ljust(width)}{'pdflatex naive':>16}{'pdflatex fixed':>16}" +
          ("".join(f"{'flashtex naive':>16}{'flashtex fixed':>16}") if len(engines) > 1 else ""))
    mark = {True: "compiles", False: "BROKEN", None: "(refused)"}
    for r in rows:
        line = r["name"].ljust(width)
        for label in ("pdflatex", "flashtex"):
            if label not in r["engines"]:
                continue
            e = r["engines"][label]
            line += f"{mark[e['naive']]:>16}{mark[e['normalised']]:>16}"
        print(line)
    print(f"\n{len(rows)} cases; {earned} where the rule turns a pdflatex error into a clean compile.")
    divergent = [r["name"] for r in rows if "flashtex" in r["engines"]
                 and r["engines"]["flashtex"]["naive"] != r["engines"]["pdflatex"]["naive"]]
    if divergent:
        print(f"\nflashtex accepts {len(divergent)} naive insertions pdflatex rejects "
              f"(the engine does not diagnose these; pdflatex is the oracle):")
        for name in divergent:
            print(f"  - {name}")
    for f in failures:
        print(f"FAIL {f}", file=sys.stderr)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
