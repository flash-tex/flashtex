#!/bin/sh
# Knuth's trip test (tripman.tex Appendix A), for the engine crate.
#
# The trip test needs its own capacities (tripman.tex step 2): mem_min=mem_bot=1,
# mem_top=mem_max=3000, error_line=64, half_error_line=32, max_print_line=72,
# everything else at tex.web's normal settings, with `init`/`tini` and
# `stat`/`tats` null. So this regenerates src/generated/ with those values,
# builds, runs steps 3 and 4, and diffs against Knuth's master trip.log.
#
#   scripts/flashtex-trip.sh <dir-with-trip-fixtures>
#
# The fixture directory needs trip.tex, trip.pl and the master trip.log,
# tripin.log and trip.fot from
# <https://mirrors.ctan.org/systems/knuth/dist/tex/>. `pltotf` converts
# trip.pl to trip.tfm; that is oracle/fixture preparation, never the product
# path.
#
# IMPORTANT: this rewrites crates/flashtex-engine/src/generated/. Run
# `git checkout crates/flashtex-engine/src/generated` (or the regeneration
# command in tools/web2rust/README.md) afterwards to restore the committed,
# default-capacity build.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
fix=${1:?usage: flashtex-trip.sh <dir-with-trip-fixtures>}
fix=$(cd "$fix" && pwd)
work=${TRIP_WORK:-$(mktemp -d)}

cargo build --release -p web2rust
"$root/target/release/web2rust" "$root/third_party/knuth/tex.web" \
    --stat \
    --const mem_min=1 --const mem_max=3000 \
    --const error_line=64 --const half_error_line=32 \
    --const max_print_line=72 \
    --macro mem_bot=1 --macro mem_top=3000 \
    --scalar glue_ratio=f32 \
    --out-dir "$root/crates/flashtex-engine/src/generated" \
    --pool "$work/tex.pool"
cargo build --release -p flashtex-engine

initex="$root/target/release/flashtex-initex"
export FLASHTEX_POOL="$work/tex.pool"

cp "$fix/trip.tex" "$fix/trip.pl" "$work/"
(cd "$work" && pltotf trip.pl trip.tfm >/dev/null)

# Step 3: an empty first line, then `\input trip`; produces trip.fmt and the
# transcript that Appendix D calls tripin.log.
printf '\n\\input trip\n' | (cd "$work" && "$initex" >tripin.fot 2>&1) || true
mv "$work/trip.log" "$work/tripin.log"
# Step 4: ` &trip  trip ` -- the spaces are part of the test.
printf ' &trip  trip \n' | (cd "$work" && "$initex" >trip.fot 2>&1) || true

# Step 6: DVItype on trip.dvi, with tripman.tex's settings, versus trip.typ.
if command -v dvitype >/dev/null 2>&1; then
    (cd "$work" && dvitype -output-level=2 \
        -page-start='*.*.*.*.*.*.*.*.*.*' -max-pages=1000000 \
        -dpi=72.27 -magnification=0 trip.dvi >trip.typ 2>&1) || true
fi

echo "work dir: $work"
for f in tripin.log trip.log trip.fot trip.typ tripos.tex; do
    if [ -f "$work/$f" ] && [ -f "$fix/$f" ]; then
        n=$(diff "$fix/$f" "$work/$f" | grep -c '^[<>]' || true)
        a=$(wc -l <"$fix/$f" | tr -d ' ')
        b=$(wc -l <"$work/$f" | tr -d ' ')
        echo "$f: $n differing lines (master $a lines, ours $b lines)"
    else
        echo "$f: MISSING (master $( [ -f "$fix/$f" ] && echo present || echo absent), ours $( [ -f "$work/$f" ] && echo present || echo absent))"
    fi
done
