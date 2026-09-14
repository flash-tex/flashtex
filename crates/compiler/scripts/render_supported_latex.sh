#!/bin/sh
# Regenerates every artefact derived from `flashtex-compiler --supported`:
#   docs/user/compiler.md            "Supported LaTeX" section (between markers)
#   crates/compiler/supported/supported-latex.json   machine-readable inventory
#                                    (the Mac Completion vocabulary's data source)
#   crates/compiler/supported/coverage.md            README benchmark table
# and then syncs the Mac app's bundled copy
#   (apps/mac/Sources/FlashTeXMac/Resources/supported-latex.json)
#   via apps/mac/scripts/sync-supported-latex.sh, so a re-render can never
#   leave the bundled copy stale.
# `cargo test --test supported_latex` fails when any of them is stale.
# The canonical denominator is regenerated separately (needs TeX Live):
#   python3 crates/compiler/scripts/canonical_latex.py
set -eu
compiler=$(cd "$(dirname "$0")/.." && pwd)
docs="$compiler/../../docs/user/compiler.md"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"

cargo build --quiet --manifest-path "$compiler/Cargo.toml" --bin flashtex-compiler
bin="$compiler/target/debug/flashtex-compiler"
if [ -n "${CARGO_TARGET_DIR:-}" ]; then bin="$CARGO_TARGET_DIR/debug/flashtex-compiler"; fi

"$bin" --supported json > "$compiler/supported/supported-latex.json"
"$bin" --supported coverage > "$compiler/supported/coverage.md"
"$bin" --supported markdown > "$compiler/supported/.section.md"

python3 - "$docs" "$compiler/supported/.section.md" <<'EOF'
import sys
from pathlib import Path
docs, section = Path(sys.argv[1]), Path(sys.argv[2]).read_text(encoding="utf-8")
begin = section.splitlines()[0]
end = "<!-- END GENERATED supported-latex -->"
text = docs.read_text(encoding="utf-8") if docs.exists() else ""
start = text.find("<!-- BEGIN GENERATED supported-latex")
if start < 0:
    text = text.rstrip("\n") + ("\n\n" if text else "") + section
else:
    stop = text.index(end, start) + len(end) + 1
    text = text[:start] + section + text[stop:]
docs.parent.mkdir(parents=True, exist_ok=True)
docs.write_text(text, encoding="utf-8")
EOF
rm "$compiler/supported/.section.md"
echo "regenerated $docs and $compiler/supported/{supported-latex.json,coverage.md}"

# Keep the Mac app's bundled copy in lockstep: regenerating without syncing is
# what kept tripping main's CI inventory-sync check. Both paths below derive
# from $compiler (which is absolute and caller-cwd-independent), so this works
# from any current working directory. No --check: this script regenerates.
sync="$compiler/../../apps/mac/scripts/sync-supported-latex.sh"
if [ ! -f "$sync" ]; then
  echo "render_supported_latex.sh: missing $sync" >&2
  exit 1
fi
"$sync" --root "$compiler/../.." || {
  echo "render_supported_latex.sh: $sync failed" >&2
  exit 1
}
