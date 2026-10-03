#!/bin/bash
# onoff.sh ENGINE DOC [REPS]: compile /tmp/l6/docs/DOC.tex with the engine in /tmp/l6/ENGINE
# with FLASHTEX_INTRINSICS=off and =on (default), twice each (the second run reads the
# .aux), compare the .log (memory statistics lines excluded, as P-T1 does), .aux and
# .pdf byte for byte, and print the second runs' user+sys CPU (median of REPS, default 3).
set -e
E=$1; DOC=$2; REPS=${3:-3}
export SOURCE_DATE_EPOCH=1700000000 FORCE_SOURCE_DATE=1 TZ=UTC
export FLASHTEX_POOL=/tmp/l6/$E/pdftex.pool FLASHTEX_FORMATS=/tmp/l6/fmt-$E
for m in off on; do
  R=/tmp/l6/onoff-$E-$DOC-$m
  rm -rf "$R"; mkdir -p "$R"; cp /tmp/l6/docs/$DOC.tex "$R/"; cd "$R"
  FLASHTEX_INTRINSICS=$m /tmp/l6/$E/flashtex-initex -fmt=pdflatex -interaction=batchmode $DOC.tex >/dev/null 2>&1 || true
  ts=()
  for i in $(seq $REPS); do
    FLASHTEX_INTRINSICS=$m FLASHTEX_INTRINSICS_STATS=$R/stats.txt /usr/bin/time -p /tmp/l6/$E/flashtex-initex -fmt=pdflatex -interaction=batchmode $DOC.tex >/dev/null 2>time.txt || true
    ts+=($(awk '/^user/{u=$2} /^sys/{s=$2} END{print u+s}' time.txt))
  done
  med=$(printf '%s\n' "${ts[@]}" | sort -n | awk '{a[NR]=$1} END{print a[int((NR+1)/2)]}')
  echo "$m cpu_median=${med}s runs=${ts[*]}"
done
A=/tmp/l6/onoff-$E-$DOC-off; B=/tmp/l6/onoff-$E-$DOC-on
filt() { grep -v -e 'words of memory' -e 'Memory usage' -e 'still untouched' "$1"; }
if cmp -s <(filt $A/$DOC.log) <(filt $B/$DOC.log) && cmp -s $A/$DOC.aux $B/$DOC.aux && cmp -s $A/$DOC.pdf $B/$DOC.pdf; then
  echo "identical: log (less memory statistics), aux, pdf"
else
  echo "DIFFERENT"; diff <(filt $A/$DOC.log) <(filt $B/$DOC.log) | head -20; cmp $A/$DOC.pdf $B/$DOC.pdf || true
fi
grep -E "calls|replays:" $B/stats.txt | tr -d ' ,' | tr '\n' ' '; echo
