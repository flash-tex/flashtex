#!/usr/bin/env python3
"""Summarise results2.jsonl (split/gonly/plain, 9 reps) and results-or.jsonl."""
import json, os, statistics as st

HERE = os.path.dirname(os.path.abspath(__file__))
PAGES = {"a": 997, "b": 999, "c": 1002, "e": 1000}


def mmm(xs, scale):
    xs = [x * scale for x in xs]
    return "%.2f / %.2f / %.2f" % (min(xs), st.median(xs), max(xs))


recs = {}
for line in open(os.path.join(HERE, "results2.jsonl")):
    r = json.loads(line)
    recs[(r["body"], r.get("driver", "mainh"), r["config"])] = r["runs"]

print("## split: ms/page (min / median / max over 9 runs)\n")
print("| body | preamble | hyperref | G (galley) | P pdf | P no-shipout | P dvi | G-only run: G | full pdflatex wall (minus empty) |")
print("|---|---|---|---|---|---|---|---|---|")
for body in "abc":
    for drv in ("mainh-nosi", "mainh"):
        for hy in ("full", "light", "none"):
            k = lambda c: recs.get((body, drv, c + "-" + hy))
            sp = k("split1-pdf")
            if not sp:
                continue
            n = PAGES[body]
            s = 1000.0 / n
            sd, go, dv, pl = k("split1-discard"), k("gonly1"), k("split1-dvi"), k("plain-pdf")
            emp = recs.get(("e", drv, "empty-pdf-" + hy))
            empty = st.median(x["wall"] for x in emp) if emp else None
            print("| %s | %s | %s | %s | %s | %s | %s | %s | %s |" % (
                body, "no siunitx" if drv == "mainh-nosi" else "+siunitx", hy,
                mmm([x["G"] for x in sp], s), mmm([x["P"] for x in sp], s),
                mmm([x["P"] for x in sd], s) if sd else "-",
                mmm([x["P"] for x in dv], s) if dv else "-",
                mmm([x["G"] for x in go], s) if go else "-",
                mmm([x["wall"] - empty for x in pl], s) if (pl and empty) else "-"))

print("\n## near-empty pages (x\\par\\newpage x1000): ms/page = (T_1000 - T_empty)/1000, medians\n")
print("| preamble | hyperref | pdf | no-shipout | dvi |")
print("|---|---|---|---|---|")
for drv in ("mainh-nosi", "mainh"):
    for hy in ("full", "light", "none"):
        emp = recs.get(("e", drv, "empty-pdf-" + hy))
        empd = recs.get(("e", drv, "empty-dvi-" + hy))
        if not emp:
            continue
        e = st.median(x["wall"] for x in emp)
        ed = st.median(x["wall"] for x in empd)
        g = lambda c: recs.get(("e", drv, c + "-" + hy))
        f = lambda c, base: "%.2f" % (st.median(x["wall"] for x in g(c)) - base) if g(c) else "-"
        print("| %s | %s | %s | %s | %s |" % ("no siunitx" if drv == "mainh-nosi" else "+siunitx", hy,
                                           f("plain-pdf", e), f("plain-discard", e), f("plain-dvi", ed)))

print("\n## output-routine timer (ortime.tex), 7 runs, seconds for whole document\n")
print("| target | pages | OR invocations | non-OR body (G + build_page) med | OR pdf min/med | OR no-shipout min/med |")
print("|---|---|---|---|---|---|")
RP = {"TeXbyTopic": 311, "memman": 525}
for line in open(os.path.join(HERE, "results-or.jsonl")):
    r = json.loads(line)
    t = r["target"]
    pdf, dis = r["runs"]["pdf"], r["runs"]["discard"]
    pages = RP.get(t, PAGES.get(t[0]))
    print("| %s | %d | %d | %.3f | %.3f / %.3f | %.3f / %.3f |" % (
        t, pages, pdf[0]["n"], st.median(x["body"] - x["OR"] for x in pdf),
        min(x["OR"] for x in pdf), st.median(x["OR"] for x in pdf),
        min(x["OR"] for x in dis), st.median(x["OR"] for x in dis)))
