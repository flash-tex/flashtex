#!/bin/bash
# prof.sh ENGINE DOC N: sample (macOS `sample`, 1 ms) N compiles of
# /tmp/l6o/docs/DOC.tex with /tmp/l6o/eng/ENGINE ("pdflatex": TeX Live's) in
# the settled directory bench.py left (/tmp/l6o/time/ENGINE-DOC), into
# /tmp/l6o/sample-ENGINE-DOC-I.txt. Summarise with sampletop.py.
set -e
E=$1; DOC=$2; N=${3:-3}
cd /tmp/l6o/time/$E-$DOC
export SOURCE_DATE_EPOCH=1700000000 FORCE_SOURCE_DATE=1 TZ=UTC
for i in $(seq 1 $N); do
  if [ "$E" = pdflatex ]; then
    # pdflatex is a symlink to pdftex; sample the process itself.
    pdftex -fmt=pdflatex -interaction=batchmode $DOC.tex >/dev/null 2>&1 &
  else
    FLASHTEX_POOL=/tmp/l6o/eng/$E/pdftex.pool FLASHTEX_FORMATS=/tmp/l6o/eng/$E/fmt \
      /tmp/l6o/eng/$E/flashtex-initex -fmt=pdflatex -interaction=batchmode $DOC.tex >/dev/null 2>&1 &
  fi
  pid=$!
  sample $pid 30 1 -mayDie -file /tmp/l6o/sample-$E-$DOC-$i.txt >/dev/null 2>&1 || true
  wait $pid || true
done
ls /tmp/l6o/sample-$E-$DOC-*.txt
