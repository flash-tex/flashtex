#!/usr/bin/env python3
"""Write crates/bibtex/tests/golden/ from TeX Live's bibtex (the oracle).

    golden.py REPO_ROOT

Each case is a directory: its input files (the .aux, the .bib files and the
.bst it uses, copied in so that no TeX Live is needed to run it), `args`
(one argument a line), and `expected/` with the oracle's `status`, `stdout`,
`stderr` and every file it wrote. `cargo test -p flashtex-bibtex --test
golden` runs the port on each with a host that finds files in the case's
directory and answers texmf.cnf's values for bibtex (max_strings=200000,
ent_str_size=500, glob_str_size=200000, max_print_line=79).

The styles copied are TeX Live's standard ones (plain, alpha, abbrv, unsrt:
"Copying of this file is authorized only if ... you make absolutely no
changes", and they are unchanged) and small styles of the fuzzer's own; the
fuzzer's damaged styles come only from the standard ones, renamed
(`mystyle.bst`), as their terms ask.
"""

import os
import shutil
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import btcmp  # noqa: E402
import fuzz  # noqa: E402

STANDARD = ["plain", "alpha", "abbrv", "unsrt"]


def kpse(fmt, name):
    o = subprocess.run(["kpsewhich", "-progname=bibtex", "-format=" + fmt, name],
                       capture_output=True, check=False)
    return o.stdout.decode().strip()


def read(p):
    with open(p, "rb") as f:
        return f.read()


def standard_file(name):
    return read(kpse("bst", name + ".bst"))


def hand_cases():
    xampl = read(kpse("bib", "xampl.bib"))
    aux = b"\\relax\n\\citation{knuth-ab}\n\\citation{*}\n\\bibstyle{%s}\n\\bibdata{xampl}\n"
    yield "xampl-plain", {"doc.aux": aux % b"plain", "xampl.bib": xampl, "plain.bst": standard_file("plain")}, ["doc"]
    yield "xampl-alpha-terse", {"doc.aux": aux % b"alpha", "xampl.bib": xampl, "alpha.bst": standard_file("alpha")}, ["-terse", "doc.aux"]
    yield "usage", {}, []
    yield "help", {}, ["--help"]
    yield "no-aux", {}, ["missing"]
    yield "bad-option", {}, ["-x", "doc"]


def standard_derived(data):
    return b"BibTeX standard bibliography style" in data


def fuzz_cases(want):
    """Seeds whose styles are the standard ones, a damaged copy of one, or
    the fuzzer's own; one for each coverage tag in `want`."""
    have = set()
    for seed in range(0, 3000):
        files, args = fuzz.make_case(seed)
        aux = files["doc.aux"]
        style = None
        for line in aux.replace(b"\r", b"").split(b"\n"):
            if line.startswith(b"\\bibstyle{"):
                style = line[len(b"\\bibstyle{"):-1].decode()
                break
        if style in STANDARD:
            files[style + ".bst"] = standard_file(style)
        elif style == "mystyle":
            if not standard_derived(files["mystyle.bst"]):
                continue
        elif style not in (None, "tiny"):
            continue
        if sum(len(v) for v in files.values()) > 120_000:
            continue  # keep the committed cases small
        r = btcmp.run_one(btcmp.ORACLE, files, args)
        if sum(len(v) for v in r["files"].values()) > 240_000:
            continue
        if not isinstance(r["status"], int) or r["status"] < 0:
            continue  # a crash (UB in C): not a golden case
        blg = next((v for k, v in r["files"].items() if k.endswith(".blg")), b"")
        tags = {t for t, pat in fuzz.TAGS if pat in blg} | {"status%d" % r["status"]}
        if b"\r\n" in aux:
            tags.add("crlf")
        new = (tags & want) - have
        if new:
            have |= new
            yield "fuzz-%d" % seed, files, args
        if want <= have:
            return


def main():
    root = sys.argv[1]
    out = os.path.join(root, "crates/bibtex/tests/golden")
    shutil.rmtree(out, ignore_errors=True)
    want = {"realloc", "fatal", "crossref", "empty-stack", "status0", "status1", "status2",
            "status3", "bib-error", "repeated", "warning", "crlf", "ent-str"}
    cases = list(hand_cases()) + list(fuzz_cases(want))
    for name, files, args in cases:
        d = os.path.join(out, name)
        os.makedirs(os.path.join(d, "expected"))
        for n, data in files.items():
            with open(os.path.join(d, n), "wb") as f:
                f.write(data)
        with open(os.path.join(d, "args"), "w") as f:
            f.write("".join(a + "\n" for a in args))
        r = btcmp.run_one(btcmp.ORACLE, files, args)
        e = os.path.join(d, "expected")
        with open(os.path.join(e, "status"), "w") as f:
            f.write("%s\n" % r["status"])
        for k in ("stdout", "stderr"):
            with open(os.path.join(e, k), "wb") as f:
                f.write(r[k])
        for n, data in r["files"].items():
            if n not in files:
                with open(os.path.join(e, n), "wb") as f:
                    f.write(data)
        print(name, r["status"], sorted(n for n in r["files"] if n not in files))


if __name__ == "__main__":
    main()
