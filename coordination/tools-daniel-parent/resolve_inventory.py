#!/usr/bin/env python3
"""Resolve the one conflict family that every flashtex compiler lane hits.

Lanes add entries to the same tables in crates/compiler/src/*.rs and then
regenerate the derived artefacts. Two lanes touching the same table always
conflict, and the resolution is ALWAYS "keep both entries" -- but *how* you
keep both depends on where git drew the conflict boundary, and guessing wrong
produces code that reads correctly and does not compile.

Three observed shapes, all real, all from 2026-09-18:

  UNION   both sides are complete items, e.g. single-line tuples
          ("iftoggle", "{name}...", "..."),
          -> concatenate. Adding a bridge here creates an EMPTY tuple.

  TUPLE   both sides are tuple BODIES sharing one "(" above the conflict and
          one ")," below it (git anchored on the fields, not the tuples)
          -> concatenate with "    ),\n    (\n" between them.
             A plain join fuses two tuples into one with 6 fields.

  BRACE   both sides are fn/impl fragments; the first is unclosed and borrows
          the "}" that sits below the conflict
          -> concatenate with "    }\n\n" between them.

Detection is structural, not guesswork: we look at the last non-blank line
above the conflict and at the first non-blank line below it, and we only act
when the two agree on a shape. Anything ambiguous is reported and left alone
for a human, because a wrong resolution here is silent.
"""
import re
import sys

CONFLICT = re.compile(
    r"^<<<<<<< [^\n]*\n(.*?)^=======\n(.*?)^>>>>>>> [^\n]*\n", re.S | re.M
)
MARKER = re.compile(r"^(<{7}|={7}|>{7})[ \n]", re.M)


def _last_code_line(text):
    for line in reversed(text.splitlines()):
        if line.strip():
            return line.rstrip()
    return ""


def _first_code_line(text):
    for line in text.splitlines():
        if line.strip():
            return line.rstrip()
    return ""


def _spans_structure(side):
    """True when a side closes an enclosing item and/or opens a new one.

    Learned from flashtex #764: HEAD's side was `TextScript(..),` then a
    column-0 `}` closing the enum, then a whole new `pub struct TextScriptRec {`
    left unclosed. Only the boundaries were inspected, so the BRACE rule fired
    and produced an enum variant stranded outside any type -- code that reads
    fine and does not compile. A side containing a top-level closing delimiter
    is not an addition to a list; it crosses a structural boundary, and no
    mechanical bridge is correct for it.
    """
    for line in side.splitlines():
        if line.rstrip() in ("}", "};", ")", ");", "]", "];"):
            return True
    return False


def classify(before, ours, theirs, after):
    """Return ('union'|'tuple'|'brace', reason) or (None, reason)."""
    above = _last_code_line(before)
    below = _first_code_line(after)
    ours_last = _last_code_line(ours)

    if _spans_structure(ours) or _spans_structure(theirs):
        return None, "a side closes an enclosing item (crosses a structural boundary)"

    # Both sides complete single-line tuples AND the opener is not dangling.
    ours_complete = ours_last.endswith("),") or ours_last.endswith("},")
    if above.strip() == "(":
        # git left the shared "(" above the region -> each side is a body.
        if below.strip().startswith(")"):
            return "tuple", f"shared '(' above, '{below.strip()}' below"
        return None, f"dangling '(' above but unexpected below: {below.strip()!r}"

    if not ours_complete and below.strip() in ("}", "};"):
        return "brace", f"our side unclosed, '{below.strip()}' below closes it"

    if ours_complete:
        return "union", "both sides are complete items"

    # A run of doc/line comments on BOTH sides: each side is self-contained
    # prose attached to whatever follows, so concatenating is always safe.
    def all_comment(s):
        lines = [l.strip() for l in s.splitlines() if l.strip()]
        return bool(lines) and all(l.startswith("//") for l in lines)

    if all_comment(ours) and all_comment(theirs):
        return "union", "both sides are comment runs"

    # A sequence of complete statements, where the text below is another
    # statement rather than a closing delimiter: nothing is being shared.
    if ours_last.endswith(";") and not below.strip().startswith(("}", ")", "]")):
        return "union", "both sides are complete statements"

    return None, f"ambiguous: above={above.strip()!r} ourslast={ours_last!r} below={below.strip()!r}"


BRIDGE = {"union": "", "tuple": "    ),\n    (\n", "brace": "    }\n\n"}


def resolve(path, dry_run=False):
    text = open(path, encoding="utf-8").read()
    if not MARKER.search(text):
        return 0, ["no conflict"]
    notes, count, pos = [], 0, 0
    while True:
        m = CONFLICT.search(text, pos)
        if not m:
            break
        shape, why = classify(text[: m.start()], m.group(1), m.group(2), text[m.end():])
        if shape is None:
            notes.append(f"  SKIPPED hunk at offset {m.start()}: {why}")
            pos = m.end()
            continue
        merged = m.group(1).rstrip("\n") + "\n" + BRIDGE[shape] + m.group(2)
        text = text[: m.start()] + merged + text[m.end():]
        notes.append(f"  {shape.upper()}: {why}")
        count += 1
        pos = m.start() + len(merged)
    # BRACE was justified by exactly ONE real conflict (#914 typeset.rs). On
    # flashtex #627 it fired four times in one file and still produced
    # "unexpected closing delimiter". Four applications of a rule evidenced by
    # one example is extrapolation, not evidence: a file needing several
    # structural bridges is not the "two lanes appended to the same table" case
    # this tool is for. Refuse it and leave the file untouched for a human.
    braces = sum(1 for n in notes if n.startswith("  BRACE"))
    if braces > 1:
        notes = [n for n in notes if not n.startswith("  ")]
        notes.append(
            f"  REFUSED: {braces} structural bridges needed in one file -- beyond "
            "what this tool is validated for; resolve by hand"
        )
        return len(MARKER.findall(open(path, encoding="utf-8").read())), notes

    if not dry_run and count:
        open(path, "w", encoding="utf-8").write(text)
    left = len(MARKER.findall(text))
    notes.append(f"  resolved={count} markers_left={left}")
    return left, notes


if __name__ == "__main__":
    args = [a for a in sys.argv[1:] if a != "--dry-run"]
    dry = "--dry-run" in sys.argv
    worst = 0
    for p in args:
        left, notes = resolve(p, dry)
        print(p)
        for n in notes:
            print(n)
        worst = max(worst, left)
    sys.exit(1 if worst else 0)
