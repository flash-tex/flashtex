#!/usr/bin/env python3
"""Tokenise a TANGLE-produced Pascal file into the same normalised one-token-
per-line form that `web2rust --emit-pascal` writes, so the two can be diffed.

This is the oracle check for the tangle stage: MacTeX's `tangle` is run on
`third_party/knuth/tex.web`, and its `tex.p` must tokenise to exactly what
web2rust produces.
"""
import sys

TWO = (":=", "<=", ">=", "<>", "..")


def tokens(src: str):
    i, n = 0, len(src)
    while i < n:
        c = src[i]
        if c in " \t\n\r":
            i += 1
        elif c == "{":
            # A Pascal comment. TANGLE never nests them (it substitutes
            # `[NNN:]` for `{NNN:}` inside meta-comments), but strings inside
            # one must still be skipped so a `}` in a string does not close it.
            i += 1
            while i < n and src[i] != "}":
                if src[i] == "'":
                    i += 1
                    while i < n:
                        if src[i] == "'":
                            if i + 1 < n and src[i + 1] == "'":
                                i += 2
                                continue
                            i += 1
                            break
                        i += 1
                    continue
                i += 1
            i += 1
        elif c == "'":
            i += 1
            s = []
            while i < n:
                if src[i] == "'":
                    if i + 1 < n and src[i + 1] == "'":
                        s.append("'")
                        i += 2
                        continue
                    i += 1
                    break
                s.append(src[i])
                i += 1
            yield "s " + "".join(s)
        elif c.isalpha():
            j = i
            while j < n and (src[j].isalnum() or src[j] == "_"):
                j += 1
            yield "i " + src[i:j].replace("_", "").lower()
            i = j
        elif c.isdigit():
            j = i
            while j < n and src[j].isdigit():
                j += 1
            real = False
            if j + 1 < n and src[j] == "." and src[j + 1].isdigit():
                real = True
                j += 1
                while j < n and src[j].isdigit():
                    j += 1
            if j < n and src[j] in "eE":
                k = j + 1
                if k < n and src[k] in "+-":
                    k += 1
                if k < n and src[k].isdigit():
                    real = True
                    j = k
                    while j < n and src[j].isdigit():
                        j += 1
            text = src[i:j]
            yield ("r " + text) if real else ("n " + str(int(text)))
            i = j
        elif src[i : i + 2] in TWO:
            yield "o " + src[i : i + 2]
            i += 2
        else:
            yield "o " + c
            i += 1


def main():
    src = open(sys.argv[1], encoding="latin-1").read()
    out = sys.stdout
    for t in tokens(src):
        out.write(t + "\n")


if __name__ == "__main__":
    main()
