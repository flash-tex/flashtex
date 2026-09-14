#!/usr/bin/env python3
"""The font/metric diagnostic classification, read from the one place it lives.

`crates/render-pipeline/src/fontdiag.rs` is the source of truth; it generates
`crates/render-pipeline/supported/font-diagnostics.json`
(`flashtex-render --font-diagnostics json`), and
`crates/render-pipeline/tests/fontdiag.rs` fails if that file is stale or if
`src/` emits a code the table does not classify.

Three harnesses used to keep their own hand-written copy of this list and had
drifted apart, most dangerously `apps/mac/scripts/texmf-acceptance.sh`, whose
copy omitted `ec_metrics_unavailable`, `font_outline_substituted` and
`math_font_unavailable` -- so the acceptance gate for the shipped app's font
discovery printed "0 missing-metric diagnostics = ok" while a real
substitution had happened. Import this module (or call it) instead of writing
a fourth copy.

Views:
  substitution   the document did not get a face or metrics it asked for.
                 Font-discovery and packaging gates count these.
  geometry_void  positions are no longer pdflatex's reference geometry, so a
                 position comparison against a pdflatex reference is void.
  advisory       substitution but not geometry_void: reported, yet positions
                 still compare (today: font_outline_substituted).

Usage as a command:
  scripts/font_diagnostics.py --view substitution [--format lines|regex|json]
"""
import argparse
import json
import os
import sys
from pathlib import Path

SCHEMA = "flashtex-font-diagnostics/1"
RELATIVE = Path("crates/render-pipeline/supported/font-diagnostics.json")
VIEWS = ("substitution", "geometry_void", "advisory", "all")


def repo_root(start=None):
    """The checkout holding crates/, found by walking up from `start`."""
    here = Path(start or __file__).resolve()
    for d in (here, *here.parents):
        if (d / RELATIVE).is_file():
            return d
    raise FileNotFoundError(
        f"no {RELATIVE} above {here}; regenerate it with: "
        f"cargo run --release --manifest-path crates/render-pipeline/Cargo.toml "
        f"--bin flashtex-render -- --font-diagnostics json > {RELATIVE}"
    )


def load(root=None):
    """The parsed manifest. Raises rather than returning a partial list: a
    harness that cannot read this must fail, not silently count nothing."""
    root = Path(root) if root else repo_root()
    path = root / RELATIVE
    data = json.loads(path.read_text())
    if data.get("schema") != SCHEMA:
        raise ValueError(f"{path}: schema {data.get('schema')!r}, expected {SCHEMA!r}")
    codes = data.get("codes")
    if not codes:
        raise ValueError(f"{path}: no codes")
    return data


def codes(view="substitution", root=None):
    """The codes in `view`, sorted. Never empty for the real views."""
    if view not in VIEWS:
        raise ValueError(f"unknown view {view!r}; expected one of {', '.join(VIEWS)}")
    entries = load(root)["codes"]
    if view == "all":
        picked = entries
    elif view == "advisory":
        picked = [e for e in entries if e["substitution"] and not e["geometry_void"]]
    else:
        picked = [e for e in entries if e[view]]
    out = tuple(sorted(e["code"] for e in picked))
    if not out:
        raise ValueError(f"view {view!r} is empty; the manifest is wrong, not this run")
    return out


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--view", choices=VIEWS, default="substitution")
    ap.add_argument("--format", choices=("lines", "regex", "json"), default="lines")
    ap.add_argument("--root", type=Path, default=None, help="repository checkout")
    args = ap.parse_args(argv)

    picked = codes(args.view, args.root)
    if args.format == "lines":
        print("\n".join(picked))
    elif args.format == "regex":
        # An alternation for `grep -oE`, e.g. texmf-acceptance.sh's count.
        print("|".join(picked))
    else:
        print(json.dumps(list(picked)))
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001 - a harness must see why it stopped
        print(f"font_diagnostics.py: {e}", file=sys.stderr)
        sys.exit(2)
