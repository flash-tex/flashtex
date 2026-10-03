#!/bin/bash
# mkeng.sh NAME TARGET_DIR [PROFILE]: copy TARGET_DIR/PROFILE/flashtex-initex
# (PROFILE defaults to release) and this worktree's pool to /tmp/l6o/eng/NAME,
# and build its pdflatex format in /tmp/l6o/eng/NAME/fmt.
set -e
# Every engine run is killed after ${LIMIT:-600} s (perl's alarm survives the
# exec), so an engine that loops cannot hang this script.
lim() { perl -e 'alarm shift @ARGV; exec @ARGV or die "exec: $!\n"' "${LIMIT:-600}" "$@"; }
W=$(cd "$(dirname "$0")/../../../.." && pwd)
N=$1; T=$2; P=${3:-release}
D=/tmp/l6o/eng/$N
rm -rf "$D"; mkdir -p "$D/fmt"
cp "$T/$P/flashtex-initex" "$D/"
[ -f "$T/$P/flashtex-host" ] && cp "$T/$P/flashtex-host" "$D/"
cp "$W/crates/flashtex-engine/pdftex.pool" "$D/"
cd "$D/fmt"
SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 FLASHTEX_POOL=$D/pdftex.pool lim "$D/flashtex-initex" -ini -jobname=pdflatex \
  -progname=pdflatex -etex -translate-file=cp227.tcx pdflatex.ini </dev/null >/dev/null 2>&1 \
  || { echo "format build failed"; tail -5 pdflatex.log; exit 1; }
echo "$N: $(shasum -a 256 "$D/flashtex-initex" | cut -c1-16) fmt $(shasum -a 256 "$D/fmt/pdflatex.fmt" | cut -c1-16)"
