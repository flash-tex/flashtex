#!/usr/bin/env python3
"""Random BibTeX inputs, oracle vs port, byte for byte.

    fuzz.py --port <binary> [--seeds A:B] [--jobs N] [--out results.jsonl]

Each seed makes one case in a fresh directory: a random .aux (citations of
existing, missing, duplicate and differently-cased keys and `*`, one or
many `\\bibdata` files, `\\bibstyle` present, missing or repeated, nested
`\\@input` files, malformed lines, CR LF), random .bib files (every entry
type, `@string` and `#` concatenation, `@preamble`, `@comment`, crossrefs,
names with von parts, `and others` and accents, numbers, 8-bit bytes,
syntax errors, very long fields), and a style: a TeX Live .bst, a TeX Live
.bst with random damage (to reach the style-file error paths), or a small
random style program over every built-in function. Sometimes there are
hundreds of cites, many global strings, long functions or deep literal
stacks, so that every BIB_XRETALLOC runs. Options: sometimes `-terse` and
`-min-crossrefs=N`. Every case is deterministic in its seed.
"""

import argparse
import glob
import json
import multiprocessing
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import btcmp  # noqa: E402

TL = "/usr/local/texlive/2026/texmf-dist"
STYLES = ["plain", "alpha", "abbrv", "unsrt", "ieeetr", "acm", "apalike", "siam",
          "amsplain", "amsalpha", "plainnat", "abbrvnat", "unsrtnat", "IEEEtran",
          "chicago", "named", "jss", "apsrev4-2", "agsm", "elsarticle-num"]
STYLE_TEXT = {}
for s in STYLES:
    p = glob.glob(TL + "/bibtex/bst/**/%s.bst" % s, recursive=True)
    if p:
        with open(p[0], "rb") as f:
            STYLE_TEXT[s] = f.read()

TYPES = ["article", "book", "booklet", "conference", "inbook", "incollection",
         "inproceedings", "manual", "mastersthesis", "misc", "phdthesis",
         "proceedings", "techreport", "unpublished", "online", "ARTICLE", "Book",
         "weird", "collection"]
FIELDS = ["author", "title", "journal", "year", "volume", "number", "pages",
          "publisher", "address", "editor", "booktitle", "month", "note", "key",
          "edition", "series", "chapter", "school", "institution", "organization",
          "howpublished", "type", "crossref", "url", "doi", "eprint", "isbn",
          "AUTHOR", "Title", "annote", "abstract"]
FIRST = ["Donald E.", "Leslie", "J.", "Jean-Paul", "Anne-Marie", "{\\'E}mile",
         "Kurt", "Erwin", "{\\AA}ke", "Ruth", "Max", "J. K.", "M.~Y.", "X", ""]
LAST = ["Knuth", "Lamport", "von Neumann", "de la Fontaine", "van der Waerden",
        "Gödel", "G{\\\"o}del", "Schr{\\\"o}dinger", "{Barnes and Noble}",
        "Smith", "Doe", "Ford, Jr.", "{\\relax Ch}ebyshev", "O'Neil", "d'Alembert",
        "Brinch Hansen", "{IEEE}", "Ng", "Zz"]
WORDS = ["The", "art", "of", "computer", "programming", "A", "note", "on",
         "{TeX}", "$\\alpha$-stable", "Über", "\\emph{very}", "{\\bf bold}",
         "Theory", "and", "Practice", "{L}aTeX", "--", "---", "{\\'e}t{\\'e}",
         "x\xe9y", "\xff", "1984", "\\'{a}", "{\\ss}", "Ångström", "of the",
         "{An} {Introduction}", "multi-word", "e.g.", "i.e.,", "!", "?", ":"]


def name(r):
    f = r.choice(FIRST)
    l = r.choice(LAST)
    k = r.random()
    if k < 0.4:
        return ("%s %s" % (f, l)).strip()
    if k < 0.7:
        return "%s, %s" % (l, f)
    if k < 0.8:
        return "%s, Jr., %s" % (l, f)
    return l


def authors(r):
    n = r.choice([1, 1, 2, 3, 4, 7, 12])
    v = [name(r) for _ in range(n)]
    if r.random() < 0.1:
        v.append("others")
    sep = " and " if r.random() < 0.95 else r.choice([" AND ", " and\n  ", "and", " , "])
    return sep.join(v)


def text(r, n=None):
    n = n or r.randint(1, 12)
    return " ".join(r.choice(WORDS) for _ in range(n))


def value(r, f, strings):
    if f.lower() in ("author", "editor"):
        v = "{%s}" % authors(r)
    elif f.lower() == "year":
        v = r.choice(['"1984"', "1990", "{2001}", '"19xx"', "{}", "2026"])
    elif f.lower() == "month":
        v = r.choice(["jan", "feb", "dec", '"May"', "{June}", "jan # \"~1\"", "foo"])
    elif f.lower() == "pages":
        v = r.choice(['"1--10"', "{12-34}", "{7}", '"1, 3, 5"', "{1+}"])
    elif f.lower() == "crossref":
        v = "{%s}" % r.choice(["proc1", "book1", "PROC1", "missing", "a1"])
    else:
        k = r.random()
        if k < 0.6:
            v = "{%s}" % text(r)
        elif k < 0.75:
            v = '"%s"' % text(r).replace('"', "")
        elif k < 0.85 and strings:
            v = "%s # {%s}" % (r.choice(strings), text(r, 2))
        elif k < 0.9:
            v = str(r.randint(0, 99999))
        elif k < 0.93:
            v = "{%s}" % ("x" * r.choice([200, 600, 5000, 25000]))
        else:
            v = "{%s" % text(r)  # unbalanced
    return v


def entry(r, key, strings):
    t = r.choice(TYPES)
    lines = []
    for _ in range(r.randint(0, 9)):
        f = r.choice(FIELDS)
        lines.append("  %s = %s" % (f, value(r, f, strings)))
    sep = ",\n" if r.random() < 0.97 else "\n"
    open_, close = ("{", "}") if r.random() < 0.9 else ("(", ")")
    body = sep.join(lines)
    if r.random() < 0.5 and lines:
        body += ","
    return "@%s%s%s,\n%s\n%s\n" % (t, open_, key, body, close)


def bibfile(r, keys):
    out = []
    strings = []
    if r.random() < 0.4:
        out.append("@preamble{ \"\\newcommand{\\noop}[1]{}\" }\n")
    for i in range(r.randint(0, 4)):
        s = r.choice(["jacm", "acm", "STOC", "pub", "sx%d" % i])
        strings.append(s)
        out.append("@string{%s = {%s}}\n" % (s, text(r, 3)))
    if r.random() < 0.2:
        out.append("@comment{ this is ignored }\n")
    for k in keys:
        out.append(entry(r, k, strings))
        if r.random() < 0.05:
            out.append(r.choice(["@article{,}\n", "@{x}\n", "garbage @ here\n",
                                 "@string{x = }\n", "@book{dup, title=}\n",
                                 "@article{k, author = {A} # }\n", "@\n", "@misc{q title={x}}\n",
                                 "@preamble{\"x\" # {y}}\n", "@string(fooo = \"bar\")\n"]))
    if r.random() < 0.2:
        out.append(entry(r, "proc1", strings).replace(r.choice(TYPES), "proceedings", 1))
    d = "".join(out)
    if r.random() < 0.1:
        d = d.replace("\n", "\r\n")
    return d.encode("latin-1", "replace") if r.random() < 0.3 else d.encode("utf-8")


BUILTINS = ["=", ">", "<", "+", "-", "*", ":=", "add.period$", "call.type$",
            "change.case$", "chr.to.int$", "cite$", "duplicate$", "empty$",
            "format.name$", "if$", "int.to.chr$", "int.to.str$", "missing$",
            "newline$", "num.names$", "pop$", "preamble$", "purify$", "quote$",
            "skip$", "stack$", "substring$", "swap$", "text.length$",
            "text.prefix$", "top$", "type$", "warning$", "while$", "width$", "write$"]


def tiny_style(r):
    """A small random .bst over every built-in function."""
    n_glob = r.choice([1, 3, 12, 25])
    gs = ["g%d" % i for i in range(n_glob)]
    out = ["ENTRY { author title year crossref } { ei } { label }\n",
           "INTEGERS { i n }\nSTRINGS { %s }\n" % " ".join(gs)]
    lits = ['"abc"', '"{\\\'E}mile Zola"', "#1", "#-7", "'skip$", "author", "title",
            '"{vv~}{ll}{, jj}{, f.}"', '"t"', '"u"', '"l"', "#0", "#3", "label", "ei"] + gs
    def body(depth):
        toks = []
        for _ in range(r.randint(1, 30 if depth == 0 else 8)):
            k = r.random()
            if k < 0.45:
                toks.append(r.choice(lits))
            elif k < 0.9:
                toks.append(r.choice(BUILTINS))
            elif depth < 2:
                toks.append("{ %s }" % body(depth + 1))
        return " ".join(toks)
    for i in range(r.randint(1, 6)):
        out.append("FUNCTION {f%d}\n{ %s }\n" % (i, body(0)))
    if r.random() < 0.3:
        out.append("FUNCTION {long}\n{ %s }\n" % " ".join(r.choice(lits) + " pop$" for _ in range(r.randint(30, 200))))
    if r.random() < 0.3:
        out.append("FUNCTION {deep}\n{ %s }\n" % " ".join(r.choice(lits) for _ in range(r.randint(40, 120))))
    out.append("FUNCTION {article}\n{ f0 }\nFUNCTION {default.type}\n{ f0 }\n")
    out.append("MACRO {jan} {\"January\"}\nREAD\n")
    if r.random() < 0.7:
        out.append("ITERATE {call.type$}\n")
    if r.random() < 0.5:
        out.append("FUNCTION {presort} { cite$ 'label := label purify$ }\nITERATE {presort}\nSORT\n")
    out.append("EXECUTE {f%d}\n" % r.randint(0, 3))
    if r.random() < 0.3:
        out.append("REVERSE {f0}\n")
    return "".join(out).encode()


def damage(r, data):
    lines = data.split(b"\n")
    for _ in range(r.randint(1, 4)):
        if not lines:
            break
        i = r.randrange(len(lines))
        k = r.random()
        if k < 0.3:
            del lines[i]
        elif k < 0.5:
            lines[i] = lines[i].replace(b"{", b"", 1)
        elif k < 0.7:
            lines[i] = lines[i].replace(b"}", b"", 1)
        elif k < 0.8:
            lines.insert(i, r.choice([b"FUNCTION", b"bogus", b"ENTRY {", b"#", b"'", b"\"",
                                      b"MACRO {x}", b"EXECUTE {nope}", b"READ", b"SORT"]))
        else:
            w = lines[i].split(b" ")
            if w:
                j = r.randrange(len(w))
                w[j] = r.choice([b"x$", b"foo", b"#99", b"'nofunc", b"\"unterminated"])
                lines[i] = b" ".join(w)
    return b"\n".join(lines)


def make_case(seed):
    r = random.Random(seed)
    files = {}
    nbib = r.choice([1, 1, 1, 2, 3, 22]) if r.random() < 0.97 else 0
    nkeys = r.choice([0, 1, 3, 10, 30, 30, 120, 800])
    allkeys = ["k%d" % i for i in range(nkeys)] + ["a1", "book1", "proc1", "Knuth:84"]
    bibnames = ["db%d" % i for i in range(nbib)]
    for b in bibnames:
        ks = r.sample(allkeys, min(len(allkeys), r.randint(0, len(allkeys))))
        files[b + ".bib"] = bibfile(r, ks)
    # the style
    k = r.random()
    if k < 0.45:
        style = r.choice(sorted(STYLE_TEXT))
    elif k < 0.75:
        style = "mystyle"
        files["mystyle.bst"] = damage(r, STYLE_TEXT[r.choice(sorted(STYLE_TEXT))])
    else:
        style = "tiny"
        files["tiny.bst"] = tiny_style(r)
    aux = [b"\\relax\n"]
    cites = []
    for _ in range(r.choice([0, 1, 5, 20, 900])):
        c = r.choice(allkeys + ["nokey", "K0", "A1"])
        cites.append(c)
    if r.random() < 0.3:
        cites.append("*")
    i = 0
    while i < len(cites):
        n = r.randint(1, 4)
        aux.append(("\\citation{%s}\n" % ",".join(cites[i:i + n])).encode())
        i += n
    if r.random() < 0.15:
        files["sub.aux"] = b"\\citation{a1}\n\\citation{Knuth:84,zz}\n" + (b"\\@input{sub2.aux}\n" if r.random() < 0.5 else b"")
        aux.append(b"\\@input{sub.aux}\n")
        if r.random() < 0.5:
            files["sub2.aux"] = b"\\citation{k1}\n"
    if r.random() < 0.05:
        aux.append(b"\\@input{missing.aux}\n")
    if r.random() < 0.03:
        # deeper than aux_stack_size (20): a fatal error
        depth = r.choice([19, 20, 21, 30])
        for d in range(depth):
            files["d%d.aux" % d] = ("\\citation{k%d}\n\\@input{d%d.aux}\n" % (d, d + 1)).encode()
        aux.append(b"\\@input{d0.aux}\n")
    if r.random() < 0.95:
        aux.append(("\\bibstyle{%s}\n" % style).encode())
    if r.random() < 0.05:
        aux.append(b"\\bibstyle{plain}\n")
    if nbib and r.random() < 0.97:
        names = bibnames + (["nofile"] if r.random() < 0.05 else [])
        aux.append(("\\bibdata{%s}\n" % ",".join(names)).encode())
    if r.random() < 0.05:
        aux.append(r.choice([b"\\citation{\n", b"\\bibdata{}\n", b"\\citation{a,,b}\n", b"\\bibstyle{a,b}\n"]))
    data = b"".join(aux)
    if r.random() < 0.1:
        data = data.replace(b"\n", b"\r\n")
    files["doc.aux"] = data
    args = []
    if r.random() < 0.2:
        args.append("-terse")
    if r.random() < 0.15:
        args.append("-min-crossrefs=%d" % r.choice([0, 1, 3, 100]))
    args.append(r.choice(["doc", "doc.aux"]) if r.random() < 0.98 else r.choice(["nodoc", "doc.au", "sub/doc"]))
    return files, args


TAGS = [
    ("realloc", b"Reallocated"), ("warning", b"Warning--"), ("bib-error", b"I was expecting"),
    ("aux-error", b"---line"), ("bst-error", b"---while executing"), ("bst-line", b"--line "),
    ("fatal", b"(That was a fatal error)"), ("overflow", b"Sorry---you've exceeded"),
    ("confusion", b"this can't happen"), ("aborted", b"Aborted at line"),
    ("no-open", b"I couldn't open"), ("crossref", b"cross reference"),
    ("ent-str", b"the entry-string-size"), ("glob-str", b"the global-string-size"),
    ("repeated", b"Repeated entry"), ("illegal", b"Illegal"), ("stack", b"stack"),
    ("empty-stack", b"empty literal stack"), ("nonempty", b"is not empty"),
]


def job(spec):
    seed, port = spec
    files, args = make_case(seed)
    try:
        d = btcmp.compare(port, files, args, timeout=20)
    except Exception as e:  # noqa: BLE001
        return {"seed": seed, "ok": False, "what": "harness: %r" % e}
    st = btcmp.LAST_STATUS.get("oracle")
    blg = btcmp.LAST_STATUS.get("blg", b"")
    tags = sorted({t for t, pat in TAGS if pat in blg})
    if d is None:
        return {"seed": seed, "ok": True, "status": st, "realloc": blg.count(b"Reallocated"), "tags": tags}
    return {"seed": seed, "ok": False, "status": st, "what": btcmp.describe(d)}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", required=True)
    ap.add_argument("--seeds", default="0:1000")
    ap.add_argument("--jobs", type=int, default=4)
    ap.add_argument("--out")
    a = ap.parse_args()
    lo, hi = (int(x) for x in a.seeds.split(":"))
    out = open(a.out, "w") if a.out else None
    bad = 0
    statuses = {}
    realloc = 0
    tagcount = {}
    with multiprocessing.Pool(a.jobs) as pool:
        for r in pool.imap_unordered(job, [(s, a.port) for s in range(lo, hi)]):
            statuses[str(r.get("status"))] = statuses.get(str(r.get("status")), 0) + 1
            realloc += 1 if r.get("realloc") else 0
            for t in r.get("tags", []):
                tagcount[t] = tagcount.get(t, 0) + 1
            if out:
                out.write(json.dumps(r) + "\n")
                out.flush()
            if not r["ok"]:
                bad += 1
                print("MISMATCH seed", r["seed"], "\n ", r["what"].replace("\n", "\n  "), flush=True)
    print("cases %d mismatches %d; oracle exit statuses %s; cases with a reallocation %d"
          % (hi - lo, bad, json.dumps(statuses, sort_keys=True), realloc))
    print("oracle .blg coverage (cases): %s" % json.dumps(tagcount, sort_keys=True))
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
