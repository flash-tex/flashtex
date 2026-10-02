#!/bin/bash
# same.sh ENGINE DOC...: compile each benchmark DOC once with $ROOT/eng/ENGINE
# from a copy of the settled directory bench.py left for the baseline engine
# ($ROOT/time/${BASE:-base}-DOC), and compare the PDF and the log byte for
# byte with the baseline's own compile there. A quick identity check between
# gate runs; the gates themselves (lockstep, parity, trip, etrip) decide.
set -e
ROOT=${ROOT:-$HOME/flashtex-wt/d2}
B=${BASE:-base}
E=$1; shift
export SOURCE_DATE_EPOCH=1700000000 FORCE_SOURCE_DATE=1 TZ=UTC
for d in "$@"; do
  w=$ROOT/same/$E-$d
  rm -rf "$w"; mkdir -p "$w"; cp "$ROOT/time/$B-$d/$d".{tex,aux} "$w/"
  [ -f "$ROOT/time/$B-$d/$d.out" ] && cp "$ROOT/time/$B-$d/$d.out" "$w/"
  ( cd "$w" && FLASHTEX_POOL=$ROOT/eng/$E/pdftex.pool FLASHTEX_FORMATS=$ROOT/eng/$E/fmt \
      "$ROOT/eng/$E/flashtex-initex" -fmt=pdflatex -interaction=batchmode "$d.tex" >/dev/null 2>&1 || true )
  p=same; cmp -s "$w/$d.pdf" "$ROOT/time/$B-$d/$d.pdf" || p=DIFFERENT
  l=same; cmp -s "$w/$d.log" "$ROOT/time/$B-$d/$d.log" || l=DIFFERENT
  echo "$E $d: pdf $p, log $l"
done
