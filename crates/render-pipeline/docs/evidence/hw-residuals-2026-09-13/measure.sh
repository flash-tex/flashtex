#!/bin/sh
# Reproduce the HW1/HW2 per-word residual table against MacTeX pdflatex.
# usage: measure.sh <repo-root> <out-dir> [ref]
#   ref  also (re)build the pdflatex oracle PDFs (MacTeX, oracle only)
# Output: <out-dir>/ours/HWn.pdf, <out-dir>/ref/HWn.pdf, <out-dir>/residuals.md
set -u
R="$1"; O="$2"; MODE="${3:-}"
mkdir -p "$O/ours" "$O/ref"
if [ "$MODE" = ref ]; then
  cp "$R/fixtures/real-world/hw1/HW1.tex" "$R/fixtures/real-world/hw2/HW2.tex" "$O/ref/"
  (
    cd "$O/ref" || exit 1
    SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1
    export SOURCE_DATE_EPOCH FORCE_SOURCE_DATE
    for n in HW1 HW2; do
      /Library/TeX/texbin/pdflatex -interaction=batchmode "$n.tex" >/dev/null 2>&1
      /Library/TeX/texbin/pdflatex -interaction=batchmode "$n.tex" >/dev/null 2>&1
      grep "Output written" "$n.log"
    done
  )
fi
BIN="$R/crates/render-pipeline/target/release/flashtex-render"
FLASHTEX_FONT_DIRS="$R/apps/mac/Fonts"
export FLASHTEX_FONT_DIRS
for h in hw1/HW1 hw2/HW2; do
  n=$(basename "$h")
  (cd "$R/fixtures/real-world" && "$BIN" --tex "$h.tex" --v2 "$O/ours/$n.json" --pdf "$O/ours/$n.pdf" >"$O/ours/$n.out" 2>&1)
  echo "$n render exit $? overfull $(grep -ci overfull "$O/ours/$n.out")"
done
python3 "$(dirname "$0")/residuals.py" "$R" "$O" > "$O/residuals.md"
head -40 "$O/residuals.md"
