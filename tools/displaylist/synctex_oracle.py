"""SyncTeX oracle for the engine-v3 preview's source mapping (lane P3-APP-V3).

For every fixture that tools/displaylist/check_positions.py left in
target/dl3-positions/<fixture>/ (display.dl3 and src/ with the converged run's
.aux/.bbl/... files), copy src/ to oracle/ and run the pinned pdflatex there
once with -synctex=1: the same layout as the engine's converged pass (its PDF
is byte-identical to pdflatex's for the parity fixtures), plus pdflatex's own
SyncTeX file. The Swift test FlashTeXPreviewV3Tests.SourceMapOracleTests then
asks the `synctex` CLI (TeX Live) about positions the display list names.

    python3 tools/displaylist/synctex_oracle.py [--work target/dl3-positions] [-j 2]

Oracle tooling only: pdflatex and synctex are never in the product path.
"""

import argparse
import concurrent.futures
import os
import shutil
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", ".."))
SKIP = (".pdf", ".synctex.gz", ".synctex", ".log", ".dl3")


def main_tex(src):
    for name in sorted(os.listdir(src)):
        if name.endswith(".tex") and os.path.exists(os.path.join(src, name[:-4] + ".pdf")):
            return name
    return None


def one(fx_dir):
    src = os.path.join(fx_dir, "src")
    name = main_tex(src) if os.path.isdir(src) else None
    if not name:
        return (fx_dir, "skip: no engine PDF")
    out = os.path.join(fx_dir, "oracle")
    shutil.rmtree(out, ignore_errors=True)
    shutil.copytree(src, out, ignore=lambda d, names: [n for n in names if n.endswith(SKIP)])
    r = subprocess.run(["nice", "-n", "15", "pdflatex", "-synctex=1", "-interaction=nonstopmode", name],
                       cwd=out, capture_output=True, timeout=300)
    pdf = os.path.join(out, name[:-4] + ".pdf")
    ok = os.path.exists(pdf) and os.path.exists(os.path.join(out, name[:-4] + ".synctex.gz"))
    return (fx_dir, "ok" if ok else "failed (exit %d)" % r.returncode)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--work", default=os.path.join(REPO, "target", "dl3-positions"))
    ap.add_argument("-j", type=int, default=2)
    a = ap.parse_args()
    dirs = sorted(os.path.join(a.work, d) for d in os.listdir(a.work) if os.path.isdir(os.path.join(a.work, d)))
    n_ok = 0
    with concurrent.futures.ThreadPoolExecutor(a.j) as ex:
        for fx, status in ex.map(one, dirs):
            n_ok += status == "ok"
            print("%-60s %s" % (os.path.basename(fx), status), flush=True)
    print("synctex oracle: %d/%d fixtures" % (n_ok, len(dirs)))
    return 0 if n_ok else 1


if __name__ == "__main__":
    sys.exit(main())
