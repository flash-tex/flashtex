#!/usr/bin/env python3
"""Fail fast when the compiler inventory outgrows the Mac hover docs' exception lists.

`EditorIntelligence.CommandDocs.beyondCompiler` / `environmentsBeyondCompiler`
(apps/mac/Sources/FlashTeXMac/EditorIntelligence.swift) name the commands and
environments the hover documents although the compiler does not render them.
`CompletionTests.testCommandDocsNameOnlyKnownCommands` requires a name to leave
those lists as soon as the inventory renders it. That makes every engine PR that
newly supports one of them fail the macOS `mac app` job, but only after a
~15-minute Swift build, on the scarce macOS runners. Rerunning never helps.

This script applies the same rule to the canonical inventory
(crates/compiler/supported/supported-latex.json) with the standard library only,
so the ubuntu job and `apps/mac/scripts/sync-supported-latex.sh` report it in
seconds, together with the one-line edit that fixes it.

Exit 0: consistent. Exit 1: a listed name is now in the inventory. Exit 2: the
Swift lists or the inventory could not be read.
"""
import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SWIFT = Path("apps/mac/Sources/FlashTeXMac/EditorIntelligence.swift")
INVENTORY = Path("crates/compiler/supported/supported-latex.json")
STRING = re.compile(r'"((?:[^"\\]|\\.)*)"')


def swift_set(source: str, name: str) -> set:
    m = re.search(r"static let %s: Set<String> = \[(.*?)\]" % re.escape(name), source, re.S)
    if not m:
        raise ValueError("no `static let %s: Set<String> = [...]` in %s" % (name, SWIFT))
    return {re.sub(r"\\(.)", r"\1", s) for s in STRING.findall(m.group(1))}


def problems(swift_source: str, inventory: dict) -> list:
    """Mirrors the second half of CompletionTests.testCommandDocsNameOnlyKnownCommands."""
    rendered = {c["name"] for c in inventory["commands"] if c.get("renders")}
    environments = {e["name"][:-1] if e["name"].endswith("*") else e["name"] for e in inventory["environments"]}
    out = []
    for name in sorted(swift_set(swift_source, "beyondCompiler") & rendered):
        out.append('\\%s is rendered by the compiler now: remove "%s" from CommandDocs.beyondCompiler' % (name, name))
    for name in sorted(swift_set(swift_source, "environmentsBeyondCompiler") & environments):
        out.append('environment %s is supported now: remove "%s" from CommandDocs.environmentsBeyondCompiler' % (name, name))
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--root", type=Path, default=ROOT)
    args = ap.parse_args()
    try:
        source = (args.root / SWIFT).read_text(encoding="utf-8")
        inventory = json.loads((args.root / INVENTORY).read_text(encoding="utf-8"))
        found = problems(source, inventory)
    except (OSError, ValueError, KeyError) as e:
        print("check-beyond-compiler: %s" % e, file=sys.stderr)
        return 2
    if not found:
        print("check-beyond-compiler: ok, no beyondCompiler name is in %s" % INVENTORY)
        return 0
    for p in found:
        print("check-beyond-compiler: %s" % p, file=sys.stderr)
    print("check-beyond-compiler: edit %s (the lists are only read by CompletionTests; the macOS mac-app job fails "
          "testCommandDocsNameOnlyKnownCommands until they match the inventory)" % SWIFT, file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
