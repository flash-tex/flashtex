#!/bin/bash
# The cold run's pass 2 -> 3 situation with an .aux point: pdftex pass 1 in a fresh copy (its .aux, .toc, .ind),
# then the resident engine compiles from there (pass A reads pass 1's files; pass B is L5 on what changed).
set -u
HB=${1:-/tmp/cs/ep/eng/flashtex-host}
W=/tmp/cs/w-l5
rm -rf "$W"
rsync -a --exclude '*.aux' --exclude '*.toc' --exclude '*.idx' --exclude '*.ind' --exclude '*.ilg' \
  --exclude '*.out' --exclude '*.log' --exclude 'infdesc.pdf' --exclude .flashtex ~/Documents/infdesc/ "$W/"
cd "$W"
export SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 FLASHTEX_POOL=/tmp/cs/ep/eng/pdftex.pool FLASHTEX_FORMATS=/tmp/cs/ep/fmt
[ -f /tmp/cs/pass1.tgz ] || { /Library/TeX/texbin/pdftex -fmt=pdflatex -interaction=batchmode -jobname=infdesc '\pdfsetrandomseed 1\relax\input{infdesc.tex}' >/dev/null 2>&1; tar czf /tmp/cs/pass1.tgz *.aux *.toc *.idx *.ind *.ilg *.out 2>/dev/null; }
tar xzf /tmp/cs/pass1.tgz
printf 'compile\nquit\n' | /usr/bin/time -l "$HB" iserve -- -fmt=pdflatex -interaction=nonstopmode -file-line-error -jobname=infdesc infdesc.tex > /tmp/cs/l5probe.out 2> /tmp/cs/l5probe.err
