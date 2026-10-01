#!/bin/bash
# prof.sh ENGINE DOC N: sample (macOS `sample`, 1 ms) N compiles of DOC with
# $ROOT/eng/ENGINE ("pdflatex": TeX Live's pdftex) in the settled directory
# bench.py left ($ROOT/time/ENGINE-DOC), into $ROOT/prof/ENGINE-DOC-I.txt.
# Summarise with the L6 lane's sampletop.py. Derived from its prof.sh.
set -e
lim() { exec perl -e 'alarm shift @ARGV; exec @ARGV or die "exec: $!\n"' "${LIMIT:-600}" "$@"; }
ROOT=${ROOT:-$HOME/flashtex-wt/d2}
E=$1; DOC=$2; N=${3:-3}
mkdir -p "$ROOT/prof"
cd "$ROOT/time/$E-$DOC"
export SOURCE_DATE_EPOCH=1700000000 FORCE_SOURCE_DATE=1 TZ=UTC
for i in $(seq 1 "$N"); do
  if [ "$E" = pdflatex ]; then
    lim pdftex -fmt=pdflatex -interaction=batchmode "$DOC.tex" >/dev/null 2>&1 &
  else
    FLASHTEX_POOL=$ROOT/eng/$E/pdftex.pool FLASHTEX_FORMATS=$ROOT/eng/$E/fmt \
      lim "$ROOT/eng/$E/flashtex-initex" -fmt=pdflatex -interaction=batchmode "$DOC.tex" >/dev/null 2>&1 &
  fi
  pid=$!
  sample $pid 60 1 -mayDie -file "$ROOT/prof/$E-$DOC-$i.txt" >/dev/null 2>&1 || true
  wait $pid || true
done
ls "$ROOT"/prof/"$E-$DOC"-*.txt
