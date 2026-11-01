#!/usr/bin/env python3
"""Copy ONLY the `digests` of named cases from a freshly recorded perf-bench
report into the committed baseline, leaving everything else byte-identical.

## Why this exists

`flashtex-perf-bench --json` / `--update-baseline` write a COMPLETE report:
digests, timing `metrics`, and `meta` (host, CPU, build profile, calibration,
`host_fingerprint`, ...). `metrics`/`meta` are only meaningful when recorded on
the pinned reference host (`--require-same-host` in the CI gate depends on
`meta.host_fingerprint` matching it); a full re-record on any other machine
would silently replace the Ryzen timing baseline with numbers from whatever
machine ran it, and disable the same-host timing gate for good.

`digests`, by contrast, are portable: they are sha256 hashes of rendered
output bytes only (see `crates/perf-bench/src/report.rs`), with no timing, RSS
or CPU input. So a PR that legitimately changes rendered output can have its
digests re-recorded on ANY machine, as long as `metrics`/`meta` are left
completely alone. This script does exactly that, and only that.

## What it refuses to do

There is no "update everything" mode and no default case list. A blanket
digest update is exactly how an unintended rendering regression gets
laundered into the baseline — the whole point of the digest gate is to force
a human to look at each changed case and say why the new output is correct.
Every case whose digests should change must be named explicitly with
`--case`. Passing zero `--case` flags is a legitimate no-op (used by the
round-trip check below), never an "update all" shorthand.

The script also refuses outright, before writing anything, if:
  * the baseline and new report don't cover the same set of case ids;
  * a named `--case` isn't present in both files;
  * a named `--case` is repeated on the command line;
  * after merging, anything other than the named cases' `digests` actually
    changed (a self-check against the untouched baseline, belt-and-braces
    on top of the text-splice approach below).

## How it works

Both files are the single-line, minified JSON `flashtex-perf-bench --json`
writes, with object keys in a fixed (alphabetical, via `BTreeMap`/sorted
`Value`) order. Rather than parsing to Python objects and re-serializing
(which risks silently reformatting floats, key order or whitespace elsewhere
in a 100+ KB line), this script parses only enough structure to find the
exact byte span of each case's `"digests": {...}` value, and splices that
span verbatim from the new report into a copy of the baseline text. Every
byte outside the named cases' digest spans is untouched, because it is never
re-emitted — it is copied through from the original file content.

## Usage

    # Round-trip check: no cases named, output must equal the input exactly.
    merge_digests.py --baseline BASE.json --new-report NEW.json --out /tmp/rt.json
    diff BASE.json /tmp/rt.json   # empty

    # Real re-record: only the named case(s) change.
    merge_digests.py --baseline BASE.json --new-report NEW.json \\
        --case lecture-notes --out BASE.json
"""

import argparse
import json
import sys


class ParseError(Exception):
    pass


def _skip_string(text, i):
    """`text[i]` is the opening `"` of a JSON string; return the index just
    past its closing `"`."""
    assert text[i] == '"'
    i += 1
    n = len(text)
    while i < n:
        c = text[i]
        if c == "\\":
            i += 2
            continue
        if c == '"':
            return i + 1
        i += 1
    raise ParseError("unterminated JSON string")


def _find_matching(text, open_idx):
    """`text[open_idx]` is `{` or `[`; return the index of its matching
    close bracket, skipping over string contents (so braces/brackets that
    appear inside quoted strings are never counted)."""
    open_ch = text[open_idx]
    close_ch = {"{": "}", "[": "]"}[open_ch]
    depth = 0
    i = open_idx
    n = len(text)
    while i < n:
        c = text[i]
        if c == '"':
            i = _skip_string(text, i)
            continue
        if c == open_ch:
            depth += 1
        elif c == close_ch:
            depth -= 1
            if depth == 0:
                return i
        i += 1
    raise ParseError(f"no matching {close_ch!r} for {open_ch!r} at offset {open_idx}")


def _find_value_span(text, key, start, end):
    """Find `"key": <value>` with the key occurring at or after `start` and
    strictly before `end`, and return (value_start, value_end_exclusive).
    Bounding by (start, end) matters: key names like "id" recur outside the
    object being searched (e.g. once per `targets` entry too)."""
    pat = f'"{key}"'
    idx = text.index(pat, start, end)
    i = idx + len(pat)
    while text[i] in " \t\r\n":
        i += 1
    if text[i] != ":":
        raise ParseError(f"expected ':' after {pat!r} at offset {i}")
    i += 1
    while text[i] in " \t\r\n":
        i += 1
    vstart = i
    if text[vstart] in "{[":
        vend = _find_matching(text, vstart) + 1
    elif text[vstart] == '"':
        vend = _skip_string(text, vstart)
    else:
        j = vstart
        while j < len(text) and text[j] not in ",}] \t\r\n":
            j += 1
        vend = j
    return vstart, vend


def _case_spans(text):
    """Return {case_id: (obj_start, obj_end_exclusive)} for every element of
    the top-level "cases" array."""
    arr_start, arr_end = _find_value_span(text, "cases", 0, len(text))
    if text[arr_start] != "[":
        raise ParseError("top-level \"cases\" is not an array")
    spans = {}
    i = arr_start + 1
    while True:
        while text[i] in " \t\r\n,":
            i += 1
        if text[i] == "]":
            break
        if text[i] != "{":
            raise ParseError(f"expected a case object at offset {i}")
        obj_start = i
        obj_end = _find_matching(text, obj_start) + 1
        id_start, id_end = _find_value_span(text, "id", obj_start, obj_end)
        case_id = json.loads(text[id_start:id_end])
        if case_id in spans:
            raise ParseError(f"duplicate case id {case_id!r} in input")
        spans[case_id] = (obj_start, obj_end)
        i = obj_end
    return spans


def _digests_span(text, obj_start, obj_end):
    return _find_value_span(text, "digests", obj_start, obj_end)


def _read(path):
    with open(path, "r", encoding="utf-8") as f:
        return f.read()


def main(argv=None):
    ap = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    ap.add_argument("--baseline", required=True, help="committed baseline JSON to update")
    ap.add_argument(
        "--new-report",
        required=True,
        help="a fresh `flashtex-perf-bench --json` report to copy digests from",
    )
    ap.add_argument(
        "--case",
        dest="cases",
        action="append",
        default=[],
        metavar="CASE_ID",
        help="case id whose digests to copy from --new-report into --baseline. "
        "Repeatable. May be omitted entirely (a no-op, not 'update all') "
        "to run the round-trip check.",
    )
    ap.add_argument(
        "--out",
        help="write the merged result here (default: overwrite --baseline in place)",
    )
    args = ap.parse_args(argv)

    dupes = sorted({c for c in args.cases if args.cases.count(c) > 1})
    if dupes:
        print(f"refusing: case id(s) repeated on the command line: {dupes}", file=sys.stderr)
        return 2

    baseline_text = _read(args.baseline)
    new_text = _read(args.new_report)

    baseline_cases = _case_spans(baseline_text)
    new_cases = _case_spans(new_text)

    if set(baseline_cases) != set(new_cases):
        only_baseline = sorted(set(baseline_cases) - set(new_cases))
        only_new = sorted(set(new_cases) - set(baseline_cases))
        print("refusing: case-id sets differ between baseline and new report", file=sys.stderr)
        if only_baseline:
            print(f"  only in --baseline:   {only_baseline}", file=sys.stderr)
        if only_new:
            print(f"  only in --new-report: {only_new}", file=sys.stderr)
        return 2

    missing = [c for c in args.cases if c not in baseline_cases]
    if missing:
        print(f"refusing: --case id(s) not present in either file: {sorted(missing)}", file=sys.stderr)
        return 2

    replacements = []  # (start, end, replacement_text)
    report_lines = []
    any_changed = False
    for case_id in args.cases:
        b_start, b_end = baseline_cases[case_id]
        n_start, n_end = new_cases[case_id]
        bd_start, bd_end = _digests_span(baseline_text, b_start, b_end)
        nd_start, nd_end = _digests_span(new_text, n_start, n_end)
        old_slice = baseline_text[bd_start:bd_end]
        new_slice = new_text[nd_start:nd_end]

        old_map = json.loads(old_slice)
        new_map = json.loads(new_slice)
        keys = sorted(set(old_map) | set(new_map))
        case_changed = False
        for k in keys:
            ov = old_map.get(k, "<absent>")
            nv = new_map.get(k, "<absent>")
            if ov != nv:
                case_changed = True
                any_changed = True
                report_lines.append(f"  {case_id}.{k}: {ov} -> {nv}")
        if case_changed:
            replacements.append((bd_start, bd_end, new_slice))
        else:
            report_lines.append(f"  {case_id}: digests unchanged (no-op)")

    replacements.sort(key=lambda r: r[0])
    pieces = []
    cursor = 0
    for start, end, text_slice in replacements:
        pieces.append(baseline_text[cursor:start])
        pieces.append(text_slice)
        cursor = end
    pieces.append(baseline_text[cursor:])
    merged_text = "".join(pieces)

    # Belt-and-braces: verify nothing outside the named cases' `digests`
    # actually changed before writing anything.
    if replacements:
        merged_json = json.loads(merged_text)
        baseline_json = json.loads(baseline_text)
        for key in ("schema_version", "meta", "targets", "unmeasured"):
            if merged_json.get(key) != baseline_json.get(key):
                print(f"internal error: {key!r} changed unexpectedly; refusing to write", file=sys.stderr)
                return 3
        by_id_old = {c["id"]: c for c in baseline_json["cases"]}
        for case in merged_json["cases"]:
            cid = case["id"]
            orig = by_id_old[cid]
            for field in orig:
                if field == "digests":
                    continue
                if case.get(field) != orig[field]:
                    print(
                        f"internal error: case {cid!r} field {field!r} changed unexpectedly; refusing to write",
                        file=sys.stderr,
                    )
                    return 3
            if cid not in args.cases and case["digests"] != orig["digests"]:
                print(f"internal error: unlisted case {cid!r} digests changed; refusing to write", file=sys.stderr)
                return 3

    print(f"cases compared: {len(baseline_cases)}; cases named: {len(args.cases)}")
    if report_lines:
        print("\n".join(report_lines))
    if args.cases and not any_changed:
        print("no digest values differed for the given case(s)")

    out_path = args.out or args.baseline
    with open(out_path, "w", encoding="utf-8") as f:
        f.write(merged_text)
    print(f"wrote {out_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
