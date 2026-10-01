import os, subprocess, json, sys
D = "/Users/dqi26/flashtex-wt/d1/t31"
E = os.environ.get("ENG", "/Users/dqi26/flashtex-wt/d1/v2")
os.makedirs(D, exist_ok=True)
for f in os.listdir(D):
    os.remove(os.path.join(D, f))


def para(i, w):
    return (f"Paragraph {i} with the word {w}, and enough text to fill a few lines of the page so that the "
            f"document ships several pages; math $a^{i}+b$.\n\n")


def doc(which, depth, inner):
    s = "\\documentclass{article}\n\\begin{document}\n\\makeatletter\n" + "".join(para(i, "delta") for i in range(40))
    s += ("Toggle: \\@ifundefined{flagA}{unset}{\\flagA{first}{second}}; depth "
          "\\@ifundefined{depthmark}{unset}{\\depthmark}.\n\n")
    s += "\\immediate\\write\\@auxout{\\string\\global\\string\\let\\string\\flagA\\string\\" + which + "}\n"
    s += ("\\immediate\\write\\@auxout{\\unexpanded{" + inner * depth + "\\gdef\\depthmark{" + str(depth) + "}"
          + "}" * depth + "}}\n")
    s += "\\makeatother\n\\end{document}\n"
    return s


env = dict(os.environ, FLASHTEX_FORMATS=E + "/fmt", FLASHTEX_POOL=E + "/pdftex.pool", SOURCE_DATE_EPOCH="1700000000",
           FORCE_SOURCE_DATE="1", FLASHTEX_PIN_CLOCK="1700000000.123456", FLASHTEX_L5_DEBUG="1")
inner = sys.argv[1]
p = subprocess.Popen([E + "/flashtex-host", "iserve", "--", "-fmt=pdflatex", "-interaction=nonstopmode", "doc.tex"],
                     cwd=D, env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=open(D + "/err", "w"),
                     text=True)


def c(tag):
    p.stdin.write("compile\n")
    p.stdin.flush()
    r = json.loads(p.stdout.readline())
    print(tag, r["pass_modes"], [x[:300] for x in r["l5"]])


W = sys.argv[3].split(",") if len(sys.argv) > 3 else None
for k, d in enumerate([int(x) for x in sys.argv[2].split(",")]):
    open(D + "/doc.tex", "w").write(doc(W[k] if W else "@firstoftwo", d, inner))
    c(d)
p.stdin.write("quit\n")
p.stdin.flush()
p.wait()
