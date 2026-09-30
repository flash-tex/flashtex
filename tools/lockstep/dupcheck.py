#!/usr/bin/env python3
"""Near-duplicate check for lockstep cases. MIT, standard library only.

A new case that repeats an existing one adds no coverage. This compares every case
of one or more NEW directories against every case of the REFERENCE directories
(main's cases, other open waves, ...) and against the other new cases:

  * strip the comment lines and trailing comments, the `\\input prelude` and `\\end`
    lines and the identification tokens of the case (its number in messages and
    dimensions), normalise whitespace;
  * replace every number (with its unit) by N, so a case that only changes one value
    of another case looks the same;
  * take the SET of tokens (control sequences, single characters) and score the
    Jaccard similarity |A and B| / |A or B| against each reference case.

Two signals are combined, because neither is enough alone (measured against the 11
duplicates a reviewer found by reading one wave against 1,100 cases):

  * CODE: the Jaccard similarity of the token trigrams above (same primitives in the
    same arrangement). Alone it puts a real duplicate first in only some cases.
  * TOPIC: the Jaccard similarity of the words of the case name and of its first-line
    description (what the case says it verifies). Alone it puts the known duplicate first
    or second for 9 of the 11.

A case is reported when its TOPIC score reaches --topic (default 0.40) or its CODE score
reaches --threshold (default 0.60), with the nearest case by each signal. The scores are
a heuristic that finds candidates to read side by side; a person still decides, and a
report must justify or replace each flagged case.

  python3 tools/lockstep/dupcheck.py --new DIR [--new DIR ...] --ref DIR [--ref DIR ...]
                                     [--threshold 0.6] [--glob 'NNNN-*.tex']

Exit status: 0 no case at or above the threshold, 1 otherwise, 2 usage error.
"""
import argparse
import glob
import os
import re
import sys

NUM = re.compile(r"[-+]?\d+(?:\.\d+)?(?:truept|pt|mu|sp|em|ex|in|bp|cm|mm|pc|dd|cc)?")
TOKEN = re.compile(r"\\[A-Za-z]+|\\.|[^\s]")


def normalise(text, ident=None):
    lines = []
    for ln in text.split("\n"):
        s = ln.strip()
        if not s or s.startswith("%"):
            continue
        if s.startswith("\\input prelude") or s == "\\end":
            continue
        s = re.sub(r"(?<!\\)%.*$", "", s)  # trailing comment
        lines.append(s)
    body = " ".join(lines)
    if ident:
        body = body.replace(ident, "N")
    return NUM.sub("N", body)


def tokens(text, ident=None):
    return set(TOKEN.findall(normalise(text, ident)))


def jaccard(a, b):
    if not a and not b:
        return 1.0
    return len(a & b) / len(a | b)


STOP = set("a an the of and or to in on is are with by for as at that it its be from this when "
           "then not no does do into over under after before than same one two each every first "
           "next only also both keeps keep make makes verifies verify tex web pdftex".split())


def topic_words(name, first_line):
    slug = name[name.index("-") + 1:-4] if "-" in name else name[:-4]
    desc = first_line.split(":", 1)[1] if ":" in first_line else first_line
    desc = re.sub(r"\((?:pdf)?tex\.web[^)]*\)", "", desc)
    words = set(re.findall(r"[A-Za-z]+", slug.replace("-", " ")))
    words |= set(re.findall(r"\\[A-Za-z]+|[A-Za-z]{4,}", desc))
    out = set()
    for w in words:
        w = re.sub(r"(ing|ed|es|s)$", "", w.lower().lstrip("\\"))
        if len(w) > 2 and w not in STOP:
            out.add(w)
    return out


def trigrams(text, ident=None):
    seq = TOKEN.findall(normalise(text, ident))
    return set(zip(seq, seq[1:], seq[2:]))


def load(dirs, pattern):
    """{name: (path, code trigrams, topic words)}; the first directory wins on a name."""
    cases = {}
    for d in dirs:
        for path in sorted(glob.glob(os.path.join(d, pattern))):
            name = os.path.basename(path)
            with open(path, encoding="utf-8", errors="replace") as fh:
                text = fh.read()
            first = text.split("\n", 1)[0]
            cases.setdefault(name, (path, trigrams(text, name[:4]), topic_words(name, first)))
    return cases


def nearest(name, everyone, idx):
    best = (0.0, None)
    for other, entry in everyone.items():
        if other == name:
            continue
        sc = jaccard(everyone[name][idx], entry[idx])
        if sc > best[0]:
            best = (sc, other)
    return best


def compare(new, ref, threshold, topic=0.40):
    """[(name, (code score, code neighbour), (topic score, topic neighbour))] for flagged cases."""
    everyone = dict(ref)
    for n, e in new.items():
        everyone.setdefault(n, e)
    hits = []
    for name in new:
        code = nearest(name, everyone, 1)
        top = nearest(name, everyone, 2)
        if code[0] >= threshold or top[0] >= topic:
            hits.append((name, code, top))
    return sorted(hits, key=lambda h: -max(h[1][0], h[2][0]))


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--new", action="append", required=True)
    ap.add_argument("--ref", action="append", required=True)
    ap.add_argument("--threshold", type=float, default=0.6, help="code (trigram) score")
    ap.add_argument("--topic", type=float, default=0.40, help="topic (name and description) score")
    ap.add_argument("--glob", default="*.tex")
    args = ap.parse_args(argv)
    for d in args.new + args.ref:
        if not os.path.isdir(d):
            print("error: not a directory: %s" % d, file=sys.stderr)
            return 2
    new = load(args.new, args.glob)
    ref = load(args.ref, args.glob)
    if not new:
        print("error: no new cases matched", file=sys.stderr)
        return 2
    hits = compare(new, ref, args.threshold, args.topic)
    print("%d new cases checked against %d reference cases" % (len(new), len(ref)))
    for name, code, top in hits:
        print("  %s\n      code  %.2f ~ %s\n      topic %.2f ~ %s" % (name, code[0], code[1], top[0], top[1]))
    print("%d flagged (code >= %.2f or topic >= %.2f)" % (len(hits), args.threshold, args.topic))
    return 1 if hits else 0


if __name__ == "__main__":
    sys.exit(main())
