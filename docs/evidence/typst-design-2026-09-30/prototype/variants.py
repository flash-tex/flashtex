"""Make feature-ablated variants of a generated bench document."""
import re, shutil, sys, os

src = sys.argv[1]
base = os.path.dirname(src.rstrip("/"))
name = os.path.basename(src.rstrip("/"))
text = open(os.path.join(src, "main.typ")).read()


def write(suffix, t):
    d = os.path.join(base, name + "-" + suffix)
    os.makedirs(d, exist_ok=True)
    for f in ("refs.bib", "fig.svg"):
        shutil.copy(os.path.join(src, f), d)
    open(os.path.join(d, "main.typ"), "w").write(t)
    print(d)


no_outline = text.replace("#outline()\n", "")
write("nooutline", no_outline)
no_cite = re.sub(r"@ref(\d+)", r"ref\1", text).replace('#bibliography("refs.bib")\n', "")
write("nocite", no_cite)
plain = re.sub(r"@ref(\d+)", r"ref\1", no_outline).replace('#bibliography("refs.bib")\n', "")
plain = re.sub(r"@eq(\d+)", r"eq \1", plain)
plain = re.sub(r"@fig(\d+)", r"fig \1", plain)
plain = re.sub(r" <(eq|fig)\d+>", "", plain)
plain = re.sub(r"#footnote\[[^\]]*\]", "", plain)
write("plain", plain)
# plain without header context and without heading numbering / equation numbering
bare = plain.replace(', header: context [_Bench_ #h(1fr) #counter(page).display()]', "")
bare = bare.replace('#set heading(numbering: "1.1")\n', "").replace('#set math.equation(numbering: "(1)")\n', "")
bare = bare.replace('numbering: "1"', "numbering: none")
write("bare", bare)
