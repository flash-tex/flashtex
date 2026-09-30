#!/usr/bin/env bash
# Reproduce docs/evidence/distribution-2026-09-29/README.md (needs TeX Live
# 2026 as the source of the bundle and as the P-T oracle, and qpdf for P-T2).
#
#   docs/evidence/distribution-2026-09-29/run.sh [WORK]
#
# 1. TeX Live discovery vs kpsewhich, in a login shell and a launchd-like
#    environment.
# 2. The format cache: first-use build and hit validation (pdflatex, pdftex);
#    end-to-end cost of a hit.
# 3. The parity fixtures: hand-built formats (baseline), the format cache,
#    and a bundle made from the fixtures' read set.
# 4. The bundle: cold / warm / offline fetch of a minimal article from a
#    local HTTP fixture server; the index size of a whole-TeX-Live bundle.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
W="${1:-${TMPDIR:-/tmp}/flashtex-distribution-evidence}"
BIN="$ROOT/target/release/flashtex-initex"
DIST="$ROOT/target/release/flashtex-dist"
export FLASHTEX_POOL="$ROOT/crates/flashtex-engine/pdftex.pool"
mkdir -p "$W"
(cd "$ROOT" && cargo build --release -p flashtex-engine)

echo "== 1. TeX Live discovery"
"$DIST" texlive --compare | tail -1
env -i HOME="$HOME" USER="$USER" TMPDIR="${TMPDIR:-/tmp}" PATH=/usr/bin:/bin:/usr/sbin:/sbin \
    "$DIST" texlive --compare | grep -E "^TeX Live:|identical"

echo "== 2. format cache"
rm -rf "$W/fmt"
for f in pdflatex pdftex; do
    FLASHTEX_FORMAT_CACHE_DIR="$W/fmt" "$DIST" format $f --hits 30
done
rm -rf "$W/hit"; mkdir -p "$W/hit/doc" "$W/hit/direct"
printf '\\documentclass{article}\n\\begin{document}\nHello, world.\n\\end{document}\n' > "$W/hit/doc/main.tex"
(cd "$W/hit/doc" && FLASHTEX_FORMAT_CACHE_DIR="$W/hit/cache" "$BIN" -fmt=pdflatex -interaction=batchmode main.tex >/dev/null)
cp "$W"/hit/cache/pdflatex-*/*.fmt "$W/hit/direct/pdflatex.fmt"
python3 - "$BIN" "$W/hit" <<'EOF'
import os, subprocess, sys, time, statistics
exe, W = sys.argv[1], sys.argv[2]
def run(env):
    t = time.perf_counter()
    subprocess.run([exe, "-fmt=pdflatex", "-interaction=batchmode", "main.tex"], cwd=f"{W}/doc",
                   env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)
    return (time.perf_counter() - t) * 1e3
d = dict(os.environ, FLASHTEX_FORMATS=f"{W}/direct"); c = dict(os.environ, FLASHTEX_FORMAT_CACHE_DIR=f"{W}/cache")
a, b = [], []
for _ in range(25):
    a.append(run(d)); b.append(run(c))
print(f"minimal article: direct {statistics.median(a):.1f} ms, cache hit {statistics.median(b):.1f} ms (medians of 25)")
EOF

echo "== 3. parity fixtures"
PT="$W/pt"; mkdir -p "$PT/hand"
(cd "$PT/hand" && FLASHTEX_FORMAT_CACHE=off "$BIN" -ini -jobname=pdflatex -progname=pdflatex -translate-file=cp227.tcx '*pdflatex.ini' </dev/null >/dev/null)
python3 "$ROOT/tools/parity/parity.py" --tier fixtures --engine "$BIN" \
    --engine-env FLASHTEX_POOL="$FLASHTEX_POOL" --engine-env FLASHTEX_FORMATS="$PT/hand" \
    --cache "$PT/oracle" --out "$PT/base.json" --work "$PT/work-base" -j 6 | tail -1
rm -f "$PT/fixtures.readset"
python3 "$ROOT/tools/parity/parity.py" --tier fixtures --engine "$BIN" \
    --engine-env FLASHTEX_POOL="$FLASHTEX_POOL" --engine-env FLASHTEX_FORMAT_CACHE_DIR="$PT/fmtcache" \
    --engine-env FLASHTEX_READ_SET="$PT/fixtures.readset" \
    --cache "$PT/oracle" --out "$PT/cache.json" --work "$PT/work-cache" -j 6 | tail -1
# The bundle: whole packages of every file read; core = the pdflatex format
# build's and a minimal article's files.
rm -f "$W/minimal.readset"
(cd "$W/hit/doc" && FLASHTEX_FORMAT_CACHE_DIR="$PT/fmtcache" FLASHTEX_READ_SET="$W/minimal.readset" \
    "$BIN" -fmt=pdflatex -interaction=batchmode main.tex >/dev/null)
FLASHTEX_FORMAT_CACHE_DIR="$PT/fmtcache" "$DIST" format pdftex >/dev/null
MANS=(); for m in "$PT"/fmtcache/*/manifest; do MANS+=(--manifest "$m"); done
"$DIST" bundle-pack --out "$W/fixtures.ttb" --read "$PT/fixtures.readset" --read "$W/minimal.readset" \
    "${MANS[@]}" --core-read "$W/minimal.readset" --core-manifest "$(ls "$PT"/fmtcache/pdflatex-*/manifest | head -1)" \
    --whole-packages
rm -rf "$PT/bundle"
python3 "$ROOT/docs/evidence/distribution-2026-09-29/pt_bundle.py" --engine "$BIN" \
    --bundle "$W/fixtures.ttb" --work "$PT/bundle" --cache "$PT/oracle" -j 6 --out "$PT/bundle.json" | tail -1

echo "== 4. bundle fetch"
"$DIST" bundle-measure --bundle "$W/fixtures.ttb" --doc "$W/hit/doc/main.tex" --work "$W/measure"
"$DIST" index-estimate
