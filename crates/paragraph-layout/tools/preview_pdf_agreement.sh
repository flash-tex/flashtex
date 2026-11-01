#!/usr/bin/env bash
# Preview/PDF agreement check (FT-019 rev 3): the same runtime-v1
# compile_result — one text item per word, emitted by this crate for the
# page-geometry oracle document — is rendered by (1) `flashtex-pdf`
# (crates/pdf, `--default-face times`) and (2) `tools/coretext_render.swift`
# (CoreText at the supplied origins, no app code). PDFKit word boxes of both
# PDFs are then compared with each other and with the emitted origins.
#
# Requires macOS (swiftc, PDFKit), cargo, python3. pdflatex is NOT used here.
#
# usage: tools/preview_pdf_agreement.sh [work-dir] [geometry1in]
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$(cd "$HERE/.." && pwd)"
REPO="$(cd "$CRATE/../.." && pwd)"
WORK="${1:-${TMPDIR:-/tmp}/flashtex-preview-pdf-agreement}"
VARIANT="${2:-}"
mkdir -p "$WORK"

echo "== emit runtime-v1 compile_result ($( [[ -n "$VARIANT" ]] && echo "$VARIANT" || echo default ))"
(cd "$CRATE" && cargo run -q --example emit_runtime_v1 -- $VARIANT) > "$WORK/compile_result.json"

echo "== flashtex-pdf (crates/pdf on this checkout)"
(cd "$REPO/crates/pdf" && cargo build -q --release)
"$("$REPO/scripts/crate-target-dir.sh" "$REPO/crates/pdf")/release/flashtex-pdf" "$WORK/compile_result.json" --out "$WORK/flashtex-pdf.pdf" --verify --default-face times

echo "== CoreText renderer"
swiftc -O "$HERE/coretext_render.swift" -o "$WORK/coretext_render"
"$WORK/coretext_render" "$WORK/compile_result.json" "$WORK/coretext.pdf"

echo "== PDFKit word boxes"
EXTRACT="$REPO/tools/native-validation/oracle_extract.swift"
if [[ ! -f "$EXTRACT" ]]; then
  # The extractor lives on the native-verification branch until integrated.
  git -C "$REPO" show origin/agent/mac-validation/native-verification:tools/native-validation/oracle_extract.swift > "$WORK/oracle_extract.swift"
  EXTRACT="$WORK/oracle_extract.swift"
fi
swiftc -O "$EXTRACT" -o "$WORK/oracle_extract"
"$WORK/oracle_extract" "$WORK/flashtex-pdf.pdf" > "$WORK/words-flashtex-pdf.json"
"$WORK/oracle_extract" "$WORK/coretext.pdf" > "$WORK/words-coretext.json"

echo "== comparison"
python3 "$HERE/compare_word_boxes.py" "$WORK/compile_result.json" "$WORK/words-flashtex-pdf.json" "$WORK/words-coretext.json" flashtex-pdf coretext | tee "$WORK/report.txt"
