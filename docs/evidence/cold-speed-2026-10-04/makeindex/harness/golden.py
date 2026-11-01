#!/usr/bin/env python3
"""Write crates/makeindex/tests/golden/: small cases with TeX Live 2026's
makeindex output (the oracle) as the expected result, so the crate's own
test (`cargo test -p flashtex-makeindex`) checks the port without TeX Live.

    golden.py <repo root>

Each case directory holds its input files, `args` (one argument per line),
and `expected/` (every file the oracle left besides the inputs, plus
`stdout`, `stderr` and `status`). Style files are given as `./NAME`
(kpathsea's answer for a file in the working directory).
"""

import os
import shutil
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mkicmp  # noqa: E402

E = "\\indexentry"


def ie(k, p):
    return "%s{%s}{%s}\n" % (E, k, p)


CASES = {
    "basic": (
        {"doc.idx": "".join([
            ie("foo", 1), ie("bar!baz|textbf", 2), ie("bar!qux", 3),
            ie("alpha@$\\alpha$", 4), ie("Zeta", 5), ie("zeta", 5), ie("1984", 6),
            ie("+plus", 7), ie("spacing|(", 2), ie("spacing|)", 9),
            ie("leading|see{spacing}", 3), ie("x|seealso{y}", 4),
            ie("a!b!c", 1), ie("a!b!c", 2), ie("a!b!c", 3), ie("a!b!d", 3),
        ])},
        ["doc.idx"],
    ),
    "merge": (
        {"doc.idx": "".join(ie("p", n) for n in (1, 2, 3, 5, 7, 8, 12, 12, 13))},
        ["-r", "doc"],
    ),
    "errors": (
        {"doc.idx": "".join([
            ie("ok", 1), "\\idxentry{x}{1}\n", E + "{a}\n", E + "{b}{1 2}\n",
            E + "{c}x{1}\n", E + "{d!}{1}\n", E + "{@e}{1}\n", E + "{f|g|h}{1}\n",
            ie("i", "1a"), ie("j", "-"), ie("k", "IIII"), E + "{l}{1}junk\n",
            ie("m|(", 1), ie("m|(", 2), ie("n|)", 3), E + "{trunc",
        ])},
        ["doc.idx"],
    ),
    "style": (
        {
            "doc.idx": "".join([ie("apple", 1), ie("Banana", 2), ie("cherry", "iv"),
                                ie("1st", 3), ie("$x$", 4), ie("date", "B")]),
            "head.ist": "headings_flag 1\nheading_prefix \"{\\\\bfseries \"\n"
                        "heading_suffix \"}\\\\hfil\\\\nopagebreak\\n\"\n"
                        "delim_0 \"\\\\dotfill \"\nline_max 30\nindent_space \"    \"\n"
                        "indent_length 4\npage_precedence \"nrA\"\n"
                        "unknown_thing \"x\"\nquote 'ab'\n",
        },
        ["-s", "head.ist", "doc.idx"],
    ),
    "pages": (
        {"doc.idx": "".join([
            ie("r", "i"), ie("r", "ii"), ie("r", "iii"), ie("r", "x"), ie("u", "IV"),
            ie("a", "a"), ie("a", "b"), ie("A", "C"), ie("c", "2-1"), ie("c", "2-2"),
            ie("c", "2-3"), ie("c", "3-1"), ie("m", "iv-2"), ie("m", "A-7"), ie("big", "123456"),
        ])},
        ["-q", "doc.idx"],
    ),
    "letter-and-german": (
        {
            "doc.idx": "".join([ie("a b", 1), ie("ab", 2), ie("a  c", 3), ie("\"Ubel", 4),
                                ie("Uber", 5), ie("Mu\"sse", 6), ie("Muster", 7)]),
            "q.ist": "quote '+'\n",
        },
        ["-l", "-g", "-s", "q.ist", "doc.idx"],
    ),
    "eleven-fields": (
        {"doc.idx": ie("a", "-".join(["1"] * 11)) + ie("b", 1) + "\x02{c}{2}\n"},
        ["doc.idx"],
    ),
    "compress-and-crlf": (
        {"doc.idx": (E + "{  a   b  }{1}\r\n" + E + "{a b}{2}\r\n" + E + "{ c\t d }{3}\r\n")},
        ["-c", "doc.idx"],
    ),
}


def main():
    root = sys.argv[1]
    out = os.path.join(root, "crates/makeindex/tests/golden")
    shutil.rmtree(out, ignore_errors=True)
    for name, (files, args) in CASES.items():
        files = {k: v.encode("latin-1") for k, v in files.items()}
        r = mkicmp.run_one(mkicmp.ORACLE, files, args)
        d = os.path.join(out, name)
        os.makedirs(os.path.join(d, "expected"))
        for k, v in files.items():
            open(os.path.join(d, k), "wb").write(v)
        open(os.path.join(d, "args"), "w").write("\n".join(args) + "\n")
        for k, v in r["files"].items():
            if k not in files:
                open(os.path.join(d, "expected", k), "wb").write(v)
        open(os.path.join(d, "expected", "stdout"), "wb").write(r["stdout"])
        open(os.path.join(d, "expected", "stderr"), "wb").write(r["stderr"])
        open(os.path.join(d, "expected", "status"), "w").write("%s\n" % r["status"])
        print(name, r["status"], sorted(r["files"]))


if __name__ == "__main__":
    main()
