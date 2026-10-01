#!/bin/bash
# run.sh ENGINE DOC [extra env assignments...]: compile /tmp/l6/docs/DOC.tex twice
# (the second run reads the .aux) in /tmp/l6/run-ENGINE-DOC with the engine in
# /tmp/l6/ENGINE ("pdflatex" = TeX Live's, the oracle), and print the second
# run's user+sys CPU seconds and the page count.
set -e
E=$1; DOC=$2; shift 2
R=/tmp/l6/run-$E-$DOC
mkdir -p "$R"; cp /tmp/l6/docs/$DOC.tex "$R/"; cd "$R"
export SOURCE_DATE_EPOCH=1700000000 FORCE_SOURCE_DATE=1 TZ=UTC
for kv in "$@"; do export "$kv"; done
if [ "$E" = pdflatex ]; then
  cmd=(pdflatex -interaction=batchmode $DOC.tex)
else
  export FLASHTEX_POOL=/tmp/l6/$E/pdftex.pool FLASHTEX_FORMATS=/tmp/l6/fmt-$E
  cmd=(/tmp/l6/$E/flashtex-initex -fmt=pdflatex -interaction=batchmode $DOC.tex)
fi
"${cmd[@]}" >/dev/null 2>&1 || true
/usr/bin/time -p "${cmd[@]}" >/dev/null 2>time.txt || true
u=$(awk '/^user/{print $2}' time.txt); s=$(awk '/^sys/{print $2}' time.txt)
pages=$(grep -o 'Output written on [^(]*([0-9]* pages' $DOC.log | grep -o '[0-9]* pages')
echo "$E $DOC cpu=$(echo "$u + $s" | bc) s  $pages"
