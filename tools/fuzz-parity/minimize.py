#!/usr/bin/env python3
"""Delta-debug a fuzz-parity difference down to a small document. MIT.

    python3 tools/fuzz-parity/minimize.py --candidate BIN [--oracle pdftex]
        (--seed N | --tex FILE) [--out min.tex] [--any]

Keeps the preamble's `\\input plain` and the final `\\end`, and removes
material with ddmin (Zeller & Hildebrandt) in three passes: whole chunks
(the generator's `%%FZ-CHUNK` units), then lines, then space-separated
tokens. A candidate input is kept when it still differs in the same way:
the same class and the same digit-masked signature as the original (with
--any, any non-equal class). Both engines run on every probe; nothing is
cached across runs.
"""
import argparse
import os
import shutil
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import fuzz  # noqa: E402
import gen  # noqa: E402
import gen_align  # noqa: E402

PLAIN_HEAD = "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6\n\\input plain\n"


def ddmin(items, test):
    """Smallest (1-minimal) sublist of items for which test(sublist) holds."""
    n = 2
    while len(items) >= 2:
        size = max(1, len(items) // n)
        subsets = [items[i:i + size] for i in range(0, len(items), size)]
        reduced = False
        for i in range(len(subsets)):
            complement = [x for j, s in enumerate(subsets) if j != i for x in s]
            if test(complement):
                items = complement
                n = max(n - 1, 2)
                reduced = True
                break
        if not reduced:
            if n >= len(items):
                break
            n = min(len(items), n * 2)
    return items


class Probe:
    def __init__(self, args, want, head, tail):
        self.args = args
        self.want = want
        self.head, self.tail = head, tail
        self.runs = 0

    def __call__(self, body):
        self.runs += 1
        text = self.head + body + self.tail
        work = tempfile.mkdtemp(prefix="fzp-min-", dir=self.args.tmp)
        try:
            r = fuzz.compare(text, self.args.candidate, self.args.oracle, work,
                             self.args.timeout, self.args.formats)
        finally:
            shutil.rmtree(work, ignore_errors=True)
        if r["cls"] == "equal":
            return False
        if self.args.any:
            return True
        return r["cls"] == self.want["cls"] and r.get("sig") == self.want.get("sig")


def split_doc(text):
    """(head, chunks, tail): the fixed head (`\\input plain` for a plain
    document, nothing for LaTeX, whose head lines stay deletable), the
    deletable chunks and the fixed tail."""
    latex = fuzz.doc_fmt(text) is not None
    tail = gen.LATEX_TAIL if latex else gen.PLAIN_TAIL
    head = ""
    body = text
    if not latex:
        i = text.find("\\input plain\n")
        if i >= 0:
            body = text[i + len("\\input plain\n"):]
        head = PLAIN_HEAD
    j = body.find("%%FZ-END")
    if j >= 0:
        body = body[:j]
    elif body.endswith(tail):
        body = body[:-len(tail)]
    return head, body.split("%%FZ-CHUNK\n"), tail


def split_body(text):
    return split_doc(text)[1]


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--candidate", required=True)
    ap.add_argument("--oracle", default="pdftex")
    ap.add_argument("--seed", type=int)
    ap.add_argument("--tex")
    ap.add_argument("--out", default="min.tex")
    ap.add_argument("--tmp", default=None)
    ap.add_argument("--timeout", type=float, default=60)
    ap.add_argument("--any", action="store_true")
    ap.add_argument("--grammar", choices=("default", "align", "align-plain"), default="default")
    ap.add_argument("--formats", default=None)
    args = ap.parse_args(argv)
    if args.formats:
        args.formats = os.path.abspath(args.formats)
    args.oracle = shutil.which(args.oracle) or args.oracle
    args.candidate = os.path.abspath(args.candidate)
    fuzz.cap_output()
    if args.seed is not None:
        mod = {"align": gen_align.generate, "align-plain": gen_align.generate_plain}.get(args.grammar, gen.generate)
        text = mod(args.seed, latex=None if args.formats else False).render()
    else:
        with open(args.tex, encoding="latin-1") as fh:
            text = fh.read()
    head, chunks, tail = split_doc(text)
    work = tempfile.mkdtemp(prefix="fzp-min-", dir=args.tmp)
    try:
        want = fuzz.compare(head + "".join(chunks) + tail, args.candidate, args.oracle, work,
                            args.timeout, args.formats)
    finally:
        shutil.rmtree(work, ignore_errors=True)
    if want["cls"] == "equal":
        print("input does not differ; nothing to minimise")
        return 2
    print("target: %s %s" % (want["cls"], want.get("sig", "")))
    probe = Probe(args, want, head, tail)
    chunks = ddmin(chunks, lambda cs: probe("".join(cs)))
    lines = "".join(chunks).split("\n")
    lines = ddmin(lines, lambda ls: probe("\n".join(ls)))
    toks = "\n".join(lines).split(" ")
    toks = ddmin(toks, lambda ts: probe(" ".join(ts)))
    body = " ".join(toks)
    with open(args.out, "w", encoding="latin-1") as fh:
        fh.write(head + body + ("\n" if not body.endswith("\n") else "") + tail)
    print("%d probes; wrote %s (%d bytes of body)" % (probe.runs, args.out, len(body)))
    return 0


if __name__ == "__main__":
    sys.exit(main())
