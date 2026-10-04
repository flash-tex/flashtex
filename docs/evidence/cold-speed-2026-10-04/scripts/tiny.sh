#!/bin/bash
mkdir -p /tmp/cs/tiny; cd /tmp/cs/tiny
printf '\\documentclass{article}\\begin{document}x\\end{document}\n' > t.tex
export SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 FLASHTEX_POOL=/tmp/cs/ep/eng/pdftex.pool FLASHTEX_FORMATS=/tmp/cs/ep/fmt
for e in /tmp/cs/ep/eng/pdftex /Library/TeX/texbin/pdftex; do
 for i in 1 2 3; do /usr/bin/time -l $e -fmt=pdflatex -interaction=batchmode t.tex 2>&1 >/dev/null | grep -E 'real|instructions retired' | tr '\n' ' '; echo " $e"; done
 /usr/bin/time -l $e -fmt=pdflatex -interaction=batchmode '\csname @@end\endcsname' 2>&1 >/dev/null | grep -E 'real|instructions retired' | tr '\n' ' '; echo " fmt-only $e"
done
