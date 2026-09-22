#!/usr/bin/env python3
"""Staleness gates for the committed generated tables (proposal §3.7).

    scripts/check-generated.py            # manifest gate: no TeX, runs in CI
    scripts/check-generated.py --run      # also run every generator's --check
    scripts/check-generated.py --update   # --run, then re-record the manifest

Each generator below has a `--check` mode that re-derives its output and
compares it with the committed file. Exit codes are shared: 0 up to date,
1 stale (it prints a diff excerpt), 2 cannot check here (a tool or input is
missing, e.g. no TeX Live on a CI runner, and the message says which).

Most generators need TeX Live 2026 (MacTeX), so CI cannot re-derive them.
What CI can check without TeX is the manifest, scripts/generated-manifest.json,
which records the SHA-256 of every generated file and of its generator. The
manifest gate fails when a generated file or a generator changed without
`--update` being run afterwards. That catches both a table edited by hand and
a generator edited but never re-run. `--update` re-derives every output
first and refuses to record a stale one.
`--update` records digests only after that generator's --check returned 0
(or 2, cannot check here, which it reports loudly). It never records one that
returned 1.

`--run` exits 1 if any generator reports stale. It exits 0 when some
generators could not be checked (2), but lists them. With `--strict`, those
fail too.
"""
import argparse
import hashlib
import json
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
MANIFEST = os.path.join(ROOT, "scripts", "generated-manifest.json")


def glyphlist():
    env = os.environ.get("FLASHTEX_GLYPHLIST")
    if env:
        return env
    try:
        return subprocess.run(["kpsewhich", "glyphlist.txt"], capture_output=True,
                              text=True).stdout.strip() or "glyphlist.txt"
    except OSError:
        return "glyphlist.txt"


# name -> (generator, outputs, --check command). Commands run from ROOT.
GENERATORS = {
    "tex-text-encoding": (
        "crates/tex-text-encoding/tools/extract_tables.py",
        ["crates/tex-text-encoding/src/generated.rs"],
        lambda: ["crates/tex-text-encoding/tools/extract_tables.py", "--check"]),
    "font-engine core14": (
        "crates/font-engine/tools/gen_tables.py",
        ["crates/font-engine/src/generated.rs"],
        # Inputs are not committed (see crates/font-engine/README.md,
        # "Reproducibility"); CI fetches them and the generator verifies their
        # SHA-256 against the committed header before comparing.
        lambda: ["crates/font-engine/tools/gen_tables.py", "--check",
                 "--afm-dir", os.environ.get("FLASHTEX_CORE14_AFM_DIR", "/nonexistent-afm-dir"),
                 "--glyphlist", glyphlist(),
                 "--out", "crates/font-engine/src/generated.rs"]),
    "math-layout cm": (
        "crates/math-layout/tools/gen_cm_tfm.py",
        ["crates/math-layout/src/cm_tfm.rs"],
        lambda: ["crates/math-layout/tools/gen_cm_tfm.py", "--check"]),
    "math-layout ams": (
        "crates/math-layout/tools/gen_cm_tfm.py",
        ["crates/math-layout/src/ams_tfm.rs"],
        lambda: ["crates/math-layout/tools/gen_cm_tfm.py", "--ams", "--check"]),
    "compiler text fontdimens": (
        "crates/compiler/scripts/gen_text_fontdimens.py",
        ["crates/compiler/src/text_fontdimens.rs"],
        lambda: ["crates/compiler/scripts/gen_text_fontdimens.py", "--check"]
        + (["--texbin", os.environ["FLASHTEX_TEXBIN"]] if "FLASHTEX_TEXBIN" in os.environ else [])),
    "compiler color names": (
        "crates/compiler/scripts/color_names.py",
        ["crates/compiler/src/color_names.rs"],
        lambda: ["crates/compiler/scripts/color_names.py", "--check"]
        + (["--texbin", os.environ["FLASHTEX_TEXBIN"]] if "FLASHTEX_TEXBIN" in os.environ else [])),
    "compiler kernel lengths": (
        "crates/compiler/scripts/gen_kernel_lengths.py",
        ["crates/compiler/src/kernel_lengths.rs"],
        lambda: ["crates/compiler/scripts/gen_kernel_lengths.py", "--check"]
        + (["--texbin", os.environ["FLASHTEX_TEXBIN"]] if "FLASHTEX_TEXBIN" in os.environ else [])),
    "compiler canonical latex": (
        "crates/compiler/scripts/canonical_latex.py",
        ["crates/compiler/supported/canonical-latex.tsv"],
        lambda: ["crates/compiler/scripts/canonical_latex.py", "--check"]),
}


def sha256(rel):
    with open(os.path.join(ROOT, rel), "rb") as fh:
        return hashlib.sha256(fh.read()).hexdigest()


def current():
    files = {}
    for gen, outs, _ in GENERATORS.values():
        for rel in [gen] + outs:
            files[rel] = sha256(rel)
    return dict(sorted(files.items()))


def verify_manifest():
    try:
        with open(MANIFEST) as fh:
            recorded = json.load(fh)["sha256"]
    except FileNotFoundError:
        print("FAIL  %s is missing; run scripts/check-generated.py --update" % os.path.relpath(MANIFEST))
        return 1
    now = current()
    bad = 0
    for rel, digest in now.items():
        if recorded.get(rel) != digest:
            kind = "generator" if any(rel == g for g, _, _ in GENERATORS.values()) else "generated file"
            print("FAIL  %s: %s changed since the manifest was recorded" % (rel, kind))
            bad = 1
    for rel in sorted(set(recorded) - set(now)):
        print("FAIL  %s is in the manifest but no longer checked here" % rel)
        bad = 1
    if bad:
        print("\nA generated table and its generator must change together, through the generator:")
        print("regenerate (the command is in the generator's docstring), then run")
        print("  scripts/check-generated.py --update")
        print("on a machine with TeX Live 2026. Never edit a generated file by hand.")
    else:
        print("ok    %d generated files and generators match %s"
              % (len(now), os.path.relpath(MANIFEST, ROOT)))
    return bad


def run_checks(only=None):
    results = {}
    for name, (_, _, cmd) in GENERATORS.items():
        if only and name not in only:
            continue
        print("==> %s" % name, flush=True)
        rc = subprocess.run([sys.executable] + cmd(), cwd=ROOT).returncode
        results[name] = rc if rc in (0, 1, 2) else 1
    return results


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--run", action="store_true", help="run every generator's --check too")
    ap.add_argument("--only", action="append", help="with --run: only this generator (repeatable)")
    ap.add_argument("--strict", action="store_true", help="with --run: 'cannot check here' fails")
    ap.add_argument("--update", action="store_true", help="--run, then re-record the manifest")
    args = ap.parse_args()
    if args.only and set(args.only) - set(GENERATORS):
        ap.error("unknown generator(s): %s; known: %s"
                 % (", ".join(sorted(set(args.only) - set(GENERATORS))), ", ".join(GENERATORS)))

    if not (args.run or args.update):
        return verify_manifest()

    results = run_checks(None if args.update else args.only)
    stale = [n for n, rc in results.items() if rc == 1]
    unchecked = [n for n, rc in results.items() if rc == 2]
    print()
    for n, rc in results.items():
        print("%-26s %s" % (n, {0: "up to date", 1: "STALE", 2: "not checked here"}[rc]))
    if stale:
        print("\nstale: %s -- regenerate before recording" % ", ".join(stale))
        return 1
    if args.update:
        if unchecked:
            print("\nWARNING: recording without re-deriving %s (not checked here)." % ", ".join(unchecked))
        with open(MANIFEST, "w") as fh:
            json.dump({"about": "SHA-256 of each generated table and its generator; "
                                "verified by scripts/check-generated.py, re-recorded by --update.",
                       "sha256": current()}, fh, indent=2)
            fh.write("\n")
        print("recorded %s" % os.path.relpath(MANIFEST, ROOT))
        return 0
    if unchecked and args.strict:
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
