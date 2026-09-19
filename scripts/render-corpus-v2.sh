#!/bin/bash
# Renders every fixture under fixtures/real-world and fixtures/divergence-probes
# to a rendering-v2 display list with one `flashtex-render` binary, so two
# builds can be compared byte for byte:
#
#   scripts/render-corpus-v2.sh target/debug/flashtex-render /tmp/before
#   ...change the pipeline...
#   scripts/render-corpus-v2.sh target/debug/flashtex-render /tmp/after
#   diff -rq /tmp/before /tmp/after && echo byte-identical
#
# The font environment is the one the render-pipeline tests use
# (`FLASHTEX_FONT_DIRS` at the bundled Latin Modern faces, `FLASHTEX_TFM_DIRS`
# at the TeX Live metrics); both may be exported beforehand to override.
# stderr (diagnostics) is captured next to each list so a change in what is
# reported shows up too. Nothing here invokes TeX: the fixtures' references
# are not touched.
set -u
bin="${1:?usage: render-corpus-v2.sh <flashtex-render> <outdir>}"
out="${2:?usage: render-corpus-v2.sh <flashtex-render> <outdir>}"
repo="$(cd "$(dirname "$0")/.." && pwd)"
export FLASHTEX_FONT_DIRS="${FLASHTEX_FONT_DIRS:-$repo/apps/mac/Fonts}"
if [ -z "${FLASHTEX_TFM_DIRS:-}" ] && [ -d /usr/local/texlive/2026/texmf-dist/fonts/tfm ]; then
  FLASHTEX_TFM_DIRS=$(find /usr/local/texlive/2026/texmf-dist/fonts/tfm -type d \( -name lm -o -name ec -o -name symbols \) | tr '\n' ':')
  export FLASHTEX_TFM_DIRS
fi
mkdir -p "$out"
n=0
for dir in "$repo"/fixtures/real-world/*/ "$repo"/fixtures/divergence-probes/*/; do
  name="$(basename "$dir")"
  tex="$dir/main.tex"
  if [ ! -f "$tex" ]; then
    tex="$(ls "$dir"/*.tex 2>/dev/null | head -1)"
  fi
  [ -n "$tex" ] && [ -f "$tex" ] || continue
  "$bin" --tex "$tex" --v2 "$out/$name.json" >"$out/$name.stdout" 2>"$out/$name.stderr.raw"
  echo "exit=$?" >>"$out/$name.stderr.raw"
  # The summary line names the output path, which differs per run directory.
  sed "s| -> v2 .*||" "$out/$name.stderr.raw" >"$out/$name.stderr"
  rm -f "$out/$name.stderr.raw"
  n=$((n + 1))
done
echo "rendered $n fixtures into $out"
