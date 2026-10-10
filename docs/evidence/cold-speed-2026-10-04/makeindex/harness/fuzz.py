#!/usr/bin/env python3
"""Random makeindex inputs, oracle vs port, byte for byte.

    fuzz.py --port <binary> [--seeds A:B] [--jobs N] [--out results.jsonl]

Each seed makes one case: a random .idx (keys with the `@ ! | "` and `\\`
specials, up to three levels, encapsulators, `|(`/`|)` ranges, see and
seealso, page numbers of every type and composites, and malformed lines),
often a random style file, random options (-q -c -l -r -g -s -o -t -p,
rarely -L/-T and -i), sometimes a .log for -p odd/even/any, and sometimes
a second .idx. Every case is deterministic in its seed.

Not generated, because the C program's behaviour there is undefined and not
reproducible (see the README): a style file that ends inside a string, an
empty page_precedence or page_compositor, delimiter characters set to NUL.
"""

import argparse
import glob
import json
import multiprocessing
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mkicmp  # noqa: E402

TL_STYLES = sorted(os.path.basename(p) for p in glob.glob(
    "/usr/local/texlive/2026/texmf-dist/makeindex/**/*.ist", recursive=True))

WORDS = [
    "alpha", "Alpha", "beta", "gamma", "delta", "Delta", "a", "A", "b", "z", "Z",
    "x", "foo", "bar", "baz", "Foo", "zeta", "eta", "theta", "pi", "mu",
    "1", "2", "10", "02", "123", "99999999999", "999999999999999999999999", "0",
    "$x$", "\\alpha", "\\LaTeX", "{\\em a}", "+", "-", "*", "?", "[a]", "~",
    "caf\xc3\xa9", "\xc3\x89cole", "\xc3\xa9t\xc3\xa9", "\xe9t\xe9", "\xd6l", "\xff",
    "\xe0\xb8\x81", "\xe0a", "na\xc3\xafve", "a b", "a  b", " a", "a ", "a\tb",
    "\"a", "\"o", "\"s", "\"U", "\\\"a", "a\"!b", "a\"@b", "a\"|b", "\"\"",
    "a@b", "@", "!", "|", "\\", "\\\\", "{", "}", "{x}", "{{y}}",
]

ENCAPS = [
    "", "", "", "", "|textbf", "|emph", "|(", "|)", "|(textbf", "|)textbf",
    "|)emph", "|see{x}", "|seealso{y}", "|", "|hyperpage", "|(hyperpage",
    "|)hyperpage", "|textit", "|(", "|)", "|!", "|@", "|\"|x", "|a\"|b",
]


def roman(n, upper):
    vals = [(1000, "m"), (900, "cm"), (500, "d"), (400, "cd"), (100, "c"),
            (90, "xc"), (50, "l"), (40, "xl"), (10, "x"), (9, "ix"), (5, "v"),
            (4, "iv"), (1, "i")]
    s = ""
    for v, r in vals:
        while n >= v:
            s += r
            n -= v
    return s.upper() if upper else s


def page(r):
    k = r.random()
    if k < 0.55:
        return str(r.randint(1, 30))
    if k < 0.62:
        return str(r.randint(1, 3000))
    if k < 0.70:
        return roman(r.randint(1, 40), False)
    if k < 0.74:
        return roman(r.randint(1, 40), True)
    if k < 0.79:
        return r.choice("abcdefghijklmnopqrstuvwxyz")
    if k < 0.82:
        return r.choice("ABCDEFGHIJKLMNOPQRSTUVWXYZ")
    if k < 0.90:
        n = r.randint(2, 4)
        return r.choice(["-", "-", ".", "--"]).join(page(r) for _ in range(n))
    if k < 0.92:
        return "-".join(str(r.randint(1, 9)) for _ in range(r.randint(9, 12)))
    return r.choice([
        "", " ", "1 2", "1a", "a1", "iiv", "IIII", "0", "00", "-1", "1-",
        "-", "x-y", "?", "12345678901234567890", "1" * 100, "ivx" * 40,
        "A-", "a-1-", "MCMXCIX", "\xe9", "1\t", " 3",
    ])


def key(r, levels):
    parts = []
    for _ in range(levels):
        w = r.choice(WORDS)
        if r.random() < 0.15:
            w = w + r.choice(WORDS)
        if r.random() < 0.25:
            w = w + "@" + r.choice(WORDS + ["\\textbf{" + w + "}", "$" + w + "$"])
        parts.append(w)
    return "!".join(parts)


def idx_line(r, kw="\\indexentry", ao="{", ac="}"):
    k = r.random()
    lv = r.choice([1, 1, 1, 1, 2, 2, 3, 3, 4])
    body = key(r, lv) + r.choice(ENCAPS)
    pg = page(r)
    if k < 0.90:
        sep = r.choice(["", "", "", " ", "\t"])
        return "%s%s%s%s%s%s%s%s" % (kw, ao, body, ac, sep, ao, pg, ac)
    return r.choice([
        "%s%s%s%s" % (kw, ao, body, ac),
        "%s%s%s" % (kw, ao, body),
        "%s %s%s%s%s%s%s" % (kw, ao, body, ac, ao, pg, ac),
        "\\idxentry{%s}{%s}" % (body, pg),
        "%s{%s}{%s} junk" % (kw, body, pg),
        "%s{%s}x{%s}" % (kw, body, pg),
        "%s{%s{%s}" % (kw, body, pg),
        "%s{%s}{%s" % (kw, body, pg),
        "%s{}{%s}" % (kw, pg),
        "%s{%s}{}" % (kw, body),
        "%s{%s}{1 2}" % (kw, body),
        "%s{%s\"}{%s}" % (kw, body, pg),
        "%s{%s\\}{%s}" % (kw, body, pg),
        "garbage",
        "",
        "   ",
        "%s{%s}{%s}%s{%s}{%s}" % (kw, body, pg, kw, key(r, 1), page(r)),
        "%s{%s}" % (kw, "x" * r.choice([5, 300])),
        "%s{a!b!c!d}{1}" % kw,
        "%s{!a}{1}" % kw,
        "%s{a!!b}{1}" % kw,
        "%s{a!}{1}" % kw,
        "%s{@a}{1}" % kw,
        "%s{a@b@c}{1}" % kw,
        "%s{a|b|c}{1}" % kw,
        "%s{a\x00b}{1}" % kw,
        "%s{a\"\x00b}{1}" % kw,
    ])


def make_idx(r):
    n = r.choice([0, 1, 2, 3, 5, 8, 13, 20, 30, 50, 80, 120]) if r.random() < 0.9 else r.choice([400, 1100, 1600])
    kw, ao, ac = "\\indexentry", "{", "}"
    lines = [idx_line(r, kw, ao, ac) for _ in range(n)]
    # bursts of the same key to exercise merging, ranges and duplicates
    if r.random() < 0.5 and n:
        k = key(r, r.choice([1, 2]))
        for _ in range(r.randint(2, 12)):
            lines.insert(r.randint(0, len(lines)), "%s{%s%s}{%s}" % (kw, k, r.choice(ENCAPS[:12]), page(r)))
    eol = r.choice(["\n"] * 8 + ["\r\n", "\r"])
    s = eol.join(lines)
    if r.random() < 0.8:
        s += eol
    if r.random() < 0.05:
        s += "\\indexentry{trunc"
    return s.encode("latin-1")


def sstr(r, s):
    return '"%s"' % s


STYLE_STRINGS = {
    "preamble": ["\\begin{theindex}\n", "\\begin{idx}\n\n", "", "PRE\\n"],
    "postamble": ["\n\n\\end{theindex}\n", "\\end{idx}", "\n"],
    "group_skip": ["\n\n  \\indexspace\n", "\n", "", "\\n\\n  \\medskip\\n"],
    "heading_prefix": ["{\\bfseries ", "\\n  \\item \\textbf{", ""],
    "heading_suffix": ["}\\hfil\\nopagebreak\n", "}\\n", ""],
    "symhead_positive": ["Symbols", "SYM"],
    "symhead_negative": ["symbols", "sym"],
    "numhead_positive": ["Numbers", "NUM"],
    "numhead_negative": ["numbers"],
    "item_0": ["\n  \\item ", "\\n\\item "],
    "item_1": ["\n    \\subitem ", "\\n  \\sub "],
    "item_2": ["\n      \\subsubitem "],
    "item_01": ["\n    \\subitem ", "\\n  \\SUB "],
    "item_x1": ["\n    \\subitem ", "\\n  \\X "],
    "item_12": ["\n      \\subsubitem "],
    "item_x2": ["\n      \\subsubitem "],
    "delim_0": [", ", "\\dotfill ", " "],
    "delim_1": [", ", "\\dotfill "],
    "delim_2": [", "],
    "delim_n": [", ", "; "],
    "delim_r": ["--", "-"],
    "delim_t": ["", "."],
    "suffix_2p": ["", "f.", "\\,f."],
    "suffix_3p": ["", "ff."],
    "suffix_mp": ["", "ff."],
    "encap_prefix": ["\\\\", "\\\\\\\\"],
    "encap_infix": ["{"],
    "encap_suffix": ["}"],
    "indent_space": ["\t\t", "    "],
    "page_compositor": ["-", ".", "--"],
    "page_precedence": ["rnaRA", "nrRaA", "n", "rn", "Rn", "aAn", "nA", "RrnaA", "rnaRAx", "rr", "nn", "rnaRAn", "q"],
    "setpage_prefix": ["\n  \\setcounter{page}{", "\\n  PAGE("],
    "setpage_suffix": ["}\n", ")\\n"],
}
STYLE_NUMBERS = {
    "headings_flag": ["0", "1", "-1", "2", "x"],
    "line_max": ["72", "40", "20", "200", "0", "-5"],
    "indent_length": ["16", "4", "0", "-1"],
}
STYLE_CHARS = {
    "quote": ["'\"'", "'+'", "'\\\\'"],
    "escape": ["'\\\\'", "'!'"],
    "level": ["'!'", "'>'"],
    "actual": ["'@'", "'='"],
    "encap": ["'|'", "'&'"],
    "range_open": ["'('", "'<'"],
    "range_close": ["')'", "'>'"],
}


def make_style(r):
    lines = []
    for _ in range(r.randint(0, 10)):
        k = r.random()
        if k < 0.55:
            a = r.choice(list(STYLE_STRINGS))
            v = r.choice(STYLE_STRINGS[a])
            v = v.replace("\n", r.choice(["\\n", "\n"]))
            lines.append('%s "%s"' % (a, v))
        elif k < 0.75:
            a = r.choice(list(STYLE_NUMBERS))
            lines.append("%s %s" % (a, r.choice(STYLE_NUMBERS[a])))
        elif k < 0.88:
            a = r.choice(list(STYLE_CHARS))
            lines.append("%s %s" % (a, r.choice(STYLE_CHARS[a])))
        else:
            lines.append(r.choice([
                "% a comment", "unknown_thing \"x\"", "preamble x", "quote 'ab'",
                "quote ''", "level '\\!'", "delim_0\n\"x\"", "  item_0   \"q\"",
                "HEADINGS_FLAG 1", "line_max\n  30", "keyword \"\\\\indexentry\"",
                "arg_open '{'", "arg_close '}'", "escape '\\\\'",
                "%", "symhead_positive % no string", "headings_flag",
            ]))
    s = "\n".join(lines)
    if r.random() < 0.8:
        s += "\n"
    return s.encode("latin-1")


def make_case(seed):
    r = random.Random(seed)
    files = {}
    files["doc.idx"] = make_idx(r)
    args = []
    flags = ""
    for f in "qclrg":
        if r.random() < {"q": 0.4, "c": 0.2, "l": 0.2, "r": 0.15, "g": 0.1}[f]:
            flags += f
    if r.random() < 0.02:
        flags += r.choice(["L", "T"])
    if flags:
        if r.random() < 0.5:
            args.append("-" + flags)
        else:
            args += ["-" + f for f in flags]
    if r.random() < 0.45:
        files["doc.ist"] = make_style(r)
        args += ["-s", r.choice(["doc.ist", "doc"])]
    elif r.random() < 0.15:
        # TeX Live's own styles, found by kpathsea
        args += ["-s", r.choice(TL_STYLES + ["gind", "nonexistent.ist"])]
    elif r.random() < 0.05:
        files["doc.mst"] = make_style(r)
    if r.random() < 0.1:
        args += ["-o", "out.ind"]
    if r.random() < 0.1:
        args += ["-t", "out.ilg"]
    if r.random() < 0.02:
        # names kpathsea's openout_any = p refuses, or that cannot be made
        args += [r.choice(["-o", "-t"]), r.choice([".dot.ind", "../up.ind", "/nonexistent-dir/abs.ind", "a/../b.ind", "nodir/x.ind"])]
    if r.random() < 0.1:
        p = r.choice(["even", "odd", "any", "7", "123", "x"])
        args += ["-p", p]
        if p in ("even", "odd", "any") and r.random() < 0.9:
            log = "This is a log\n[1] [2] [3\n] [%d]\n%s" % (r.randint(1, 200), r.choice(["", "Output\n", "[ 9]x", "\r\n"]))
            files["doc.log"] = log.encode()
    second = r.random() < 0.08
    if second:
        files["two.idx"] = make_idx(r)
    stdin = None
    k = r.random()
    if k < 0.03:
        args.append("-i")
        stdin = make_idx(r)
        if r.random() < 0.5:
            args.append(r.choice(["doc.idx", "doc"]))
    else:
        args.append(r.choice(["doc.idx", "doc.idx", "doc"]))
        if second:
            args.append("two.idx")
    if r.random() < 0.01:
        args = r.choice([["-s"], ["-o"], ["-x", "doc.idx"], ["-", "doc.idx"], ["doc.idx", "-q"], ["missing"], ["missing.idx"], ["-p", "1" * 120, "doc.idx"]])
    env = None
    if r.random() < 0.1:
        env = dict(os.environ)
        env["LC_ALL"] = r.choice(["C", "en_US.ISO8859-1", "th_TH.UTF-8"])
    return files, args, stdin, env


def check(seed_port):
    seed, port = seed_port
    files, args, stdin, env = make_case(seed)
    try:
        d = mkicmp.compare(port, files, args, stdin, env)
    except Exception as e:  # a timeout or a harness failure counts as a mismatch
        return {"seed": seed, "ok": False, "what": "harness: %r" % e}
    st = mkicmp.LAST_STATUS.get("oracle")
    if d is None:
        return {"seed": seed, "ok": True, "status": st}
    return {"seed": seed, "ok": False, "status": st, "what": mkicmp.describe(d), "args": args}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", required=True)
    ap.add_argument("--seeds", default="0:1000")
    ap.add_argument("--jobs", type=int, default=8)
    ap.add_argument("--out")
    a = ap.parse_args()
    lo, hi = (int(x) for x in a.seeds.split(":"))
    port = os.path.abspath(a.port)
    bad = 0
    n = 0
    statuses = {}
    out = open(a.out, "w") if a.out else None
    with multiprocessing.Pool(a.jobs) as pool:
        for res in pool.imap_unordered(check, [(s, port) for s in range(lo, hi)], chunksize=4):
            n += 1
            k = str(res.get("status"))
            statuses[k] = statuses.get(k, 0) + 1
            if out:
                out.write(json.dumps(res) + "\n")
            if not res["ok"]:
                bad += 1
                if bad <= 20:
                    print("seed %d: %s" % (res["seed"], res["what"]), flush=True)
    print("cases %d mismatches %d; oracle exit statuses %s" % (n, bad, json.dumps(statuses, sort_keys=True)))
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
