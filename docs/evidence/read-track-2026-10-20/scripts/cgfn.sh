#!/usr/bin/env bash
# cgfn.sh ENG FMT DOC OUT: callgrind profile of a CLI compile, annotated by function
. ~/rt/env.sh
VG=/nix/store/39iybd55afmpqlgf979ra06wj3q3wapa-valgrind-3.27.1/bin
E=$1; F=$2; d=$3; O=$4
W=$(mktemp -d); cp ~/rt/docs/*.tex $W/; cd $W
env FLASHTEX_POOL=$E/pdftex.pool FLASHTEX_FORMATS=$F SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 $E/flashtex-initex -fmt=pdflatex -interaction=batchmode $d.tex >/dev/null 2>&1
env FLASHTEX_POOL=$E/pdftex.pool FLASHTEX_FORMATS=$F SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 $VG/valgrind --tool=callgrind --callgrind-out-file=$O.cg $E/flashtex-initex -fmt=pdflatex -interaction=batchmode $d.tex >/dev/null 2>&1
$VG/callgrind_annotate --inclusive=no $O.cg 2>/dev/null | head -45 > $O.txt
rm -rf $W
