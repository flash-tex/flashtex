#!/bin/bash
# usage: iserve_run.sh NAME HOSTBIN [script lines...] -> /tmp/cs/iserve-NAME.{out,err}
# A clean copy of the book; `flashtex-host iserve` (the resident incremental engine) in it, cwd = the copy.
set -u
NAME=$1; HB=$2; shift 2
W=/tmp/cs/w-is-$NAME
rm -rf "$W"
rsync -a --exclude '*.aux' --exclude '*.toc' --exclude '*.idx' --exclude '*.ind' --exclude '*.ilg' \
  --exclude '*.out' --exclude '*.log' --exclude 'infdesc.pdf' --exclude .flashtex ~/Documents/infdesc/ "$W/"
cd "$W"
export SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 FLASHTEX_POOL=/tmp/cs/ep/eng/pdftex.pool FLASHTEX_FORMATS=/tmp/cs/ep/fmt
echo "load before: $(sysctl -n vm.loadavg)" > /tmp/cs/iserve-$NAME.out
if [ $# -eq 0 ]; then set -- compile quit; fi
printf '%s\n' "$@" | /usr/bin/time -l "$HB" iserve -- -fmt=pdflatex -interaction=nonstopmode -file-line-error -jobname=infdesc infdesc.tex >> /tmp/cs/iserve-$NAME.out 2> /tmp/cs/iserve-$NAME.err
echo "load after: $(sysctl -n vm.loadavg)" >> /tmp/cs/iserve-$NAME.out
