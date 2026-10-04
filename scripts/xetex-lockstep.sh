#!/bin/sh
# The XeTeX port's P-T1 and XDV gate (docs/design/xetex/PLAN.md, phase S0):
#
#   scripts/xetex-lockstep.sh [run.py options, e.g. --cases 'x0*' --jobs 4]
#
# builds crates/flashtex-xetex (its own workspace, lockfile and target
# directory: XETEX_TARGET, default $TMPDIR/flashtex-xetex-target) and runs
# tools/xetex-lockstep/run.py against TeX Live 2026's `xetex` (an oracle only,
# never in the product path): every case's log (box dumps included) and exit
# status, and its XDV file after normalisation, must equal the reference's.
# Exit status 0 iff every case is equal. Needs TeX Live 2026 on PATH.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
target=${XETEX_TARGET:-${TMPDIR:-/tmp}/flashtex-xetex-target}
(cd "$root/crates/flashtex-xetex" &&
    CARGO_TARGET_DIR="$target" cargo build --release --locked)
exec python3 "$root/tools/xetex-lockstep/run.py" \
    --engine "$target/release/flashtex-xetex" "$@"
