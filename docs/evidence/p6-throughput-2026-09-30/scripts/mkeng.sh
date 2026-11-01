#!/bin/bash
# mkeng.sh NAME TARGET_DIR [PROFILE]: copy TARGET_DIR/PROFILE/flashtex-initex
# (PROFILE defaults to release) and this worktree's pool to $ROOT/eng/NAME
# (ROOT defaults to ~/flashtex-wt/d2), and build its pdflatex format in
# $ROOT/eng/NAME/fmt. Derived from the L6-OPTIMIZATIONS mkeng.sh.
set -e
lim() { perl -e 'alarm shift @ARGV; exec @ARGV or die "exec: $!\n"' "${LIMIT:-600}" "$@"; }
W=$(cd "$(dirname "$0")/../../../.." && pwd)
ROOT=${ROOT:-$HOME/flashtex-wt/d2}
N=$1; T=$2; P=${3:-release}
D=$ROOT/eng/$N
rm -rf "$D"; mkdir -p "$D/fmt"
cp "$T/$P/flashtex-initex" "$D/"
cp "$W/crates/flashtex-engine/pdftex.pool" "$D/"
cd "$D/fmt"
SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 FLASHTEX_POOL=$D/pdftex.pool lim "$D/flashtex-initex" -ini -jobname=pdflatex \
  -progname=pdflatex -etex -translate-file=cp227.tcx pdflatex.ini </dev/null >/dev/null 2>&1 \
  || { echo "format build failed"; tail -5 pdflatex.log; exit 1; }
echo "$N: $(shasum -a 256 "$D/flashtex-initex" | cut -c1-16) fmt $(shasum -a 256 "$D/fmt/pdflatex.fmt" | cut -c1-16)"
