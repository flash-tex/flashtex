#!/bin/sh
# Fetch the pinned texlive-source tests into a git-ignored cache.
# No upstream (GPL) file is ever copied into this repo: run.py reads the
# inputs from the cache at run time.
#
# Usage: sh tools/pdftex-regress/fetch.sh
set -u
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
DEST="$ROOT/tools/pdftex-regress/.cache/texlive"
URL="https://github.com/TeX-Live/texlive-source"
SHA="$(sed -n 's/^sha: //p' "$ROOT/tools/pdftex-regress/PINS.txt")"
if [ -z "$SHA" ]; then
  echo "fetch.sh: no 'sha:' line in PINS.txt" >&2
  exit 1
fi

if [ -d "$DEST/.git" ]; then
  HAVE="$(git -C "$DEST" rev-parse HEAD 2>/dev/null || echo none)"
  if [ "$HAVE" = "$SHA" ]; then
    echo "cache already at $SHA"
    exit 0
  fi
  echo "cache at $HAVE, re-fetching $SHA"
  rm -rf "$DEST"
fi

mkdir -p "$(dirname "$DEST")"
git clone --depth 1 --filter=blob:none --sparse "$URL" "$DEST" || exit 1
git -C "$DEST" sparse-checkout set texk/web2c || exit 1
git -C "$DEST" fetch --depth 1 origin "$SHA" || exit 1
git -C "$DEST" checkout "$SHA" || exit 1
echo "fetched $SHA into $DEST"
