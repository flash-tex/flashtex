#!/bin/sh
# Oracle check for the tangle stage of tools/web2rust.
#
# MacTeX's `tangle` is used as an oracle only; it is never in the product path.
# We tangle third_party/knuth/tex.web with it, tokenise the resulting tex.p, and
# require web2rust's own token stream and string pool to agree exactly.
#
# Usage: scripts/web2rust-oracle-check.sh   (from the repository root)
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
web=$root/third_party/knuth/tex.web

if ! command -v tangle >/dev/null 2>&1; then
    echo "web2rust-oracle-check: no \`tangle\` on PATH; skipping (oracle unavailable)" >&2
    exit 0
fi

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

cp "$web" "$work/tex.web"
(cd "$work" && tangle tex.web >/dev/null)

cargo build --release -p web2rust
"$root/target/release/web2rust" "$web" \
    --pool "$work/mine.pool" --emit-pascal "$work/mine.toks"

python3 "$root/tools/web2rust/tools/pascal_tokens.py" "$work/tex.p" >"$work/oracle.toks"

fail=0
if diff -q "$work/tex.pool" "$work/mine.pool" >/dev/null; then
    echo "pool:   OK (byte-identical, $(wc -l <"$work/mine.pool" | tr -d ' ') lines)"
else
    echo "pool:   MISMATCH"
    diff "$work/tex.pool" "$work/mine.pool" | head -20
    fail=1
fi
if diff -q "$work/oracle.toks" "$work/mine.toks" >/dev/null; then
    echo "tokens: OK ($(wc -l <"$work/mine.toks" | tr -d ' ') tokens identical)"
else
    echo "tokens: MISMATCH"
    diff "$work/oracle.toks" "$work/mine.toks" | head -40
    fail=1
fi
exit $fail
