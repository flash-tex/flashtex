#!/bin/sh
# Fetch pinned TeX Live triptrap/etrip inputs into a cache outside the repo.
# Nothing upstream is committed to FlashTeX; run.py reads this cache at run time.
#
#   tools/trip/fetch.sh [--cache DIR] [--rev SHA]
#
# Defaults: cache $FLASHTEX_TRIP_CACHE or ~/.cache/flashtex-trip, rev pinned
# in tools/trip/PINS.txt. Network use: one shallow sparse clone of the public
# texlive-source mirror (lane exception only).
set -eu

CACHE="${FLASHTEX_TRIP_CACHE:-$HOME/.cache/flashtex-trip}"
REV=""
HERE="$(cd "$(dirname "$0")" && pwd)"

while [ $# -gt 0 ]; do
  case "$1" in
    --cache) CACHE="$2"; shift 2;;
    --rev) REV="$2"; shift 2;;
    *) echo "usage: fetch.sh [--cache DIR] [--rev SHA]" >&2; exit 2;;
  esac
done

if [ -z "$REV" ]; then
  REV="$(awk '/^rev /{print $2}' "$HERE/PINS.txt")"
fi

SRC="$CACHE/src/texlive-source"
if [ ! -d "$SRC/.git" ]; then
  mkdir -p "$CACHE/src"
  git clone --depth 1 --filter=blob:none --sparse \
    https://github.com/TeX-Live/texlive-source "$SRC"
fi
cd "$SRC"
git fetch --depth 1 origin "$REV" 2>/dev/null || true
git checkout -q "$REV"
git sparse-checkout set texk/web2c/triptrap texk/web2c/etexdir/etrip
SHA="$(git rev-parse HEAD)"
echo "pinned: $SHA"

mkdir -p "$CACHE/triptrap" "$CACHE/etrip"
for f in texmf.cnf trip.tex trip.pl trip1.in trip2.in \
         tripin.log trip.fot trip.log trip.typ tripos.tex; do
  cp "texk/web2c/triptrap/$f" "$CACHE/triptrap/$f"
done
for f in texmf.cnf etrip.tex etrip.pl etrip2.in etrip3.in etrip1.in trip2.in \
         etripin.log etrip.fot etrip.log etrip.typ etrip.out; do
  cp "texk/web2c/etexdir/etrip/$f" "$CACHE/etrip/$f"
done
echo "cache ready: $CACHE"
