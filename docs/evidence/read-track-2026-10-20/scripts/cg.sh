#!/usr/bin/env bash
# cg.sh ENGDIR FMTDIR DOC...: instructions (callgrind Ir) of a CLI compile of each DOC in ~/rt/docs
. ~/rt/env.sh
VG=/nix/store/39iybd55afmpqlgf979ra06wj3q3wapa-valgrind-3.27.1/bin/valgrind
E=$1; F=$2; shift 2
W=$(mktemp -d); cp ~/rt/docs/*.tex $W/; cd $W
for d in "$@"; do
  env FLASHTEX_POOL=$E/pdftex.pool FLASHTEX_FORMATS=$F SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 $E/flashtex-initex -fmt=pdflatex -interaction=batchmode $d.tex >/dev/null 2>&1
  ir=$(env FLASHTEX_POOL=$E/pdftex.pool FLASHTEX_FORMATS=$F SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 $VG --tool=callgrind --callgrind-out-file=/dev/null $E/flashtex-initex -fmt=pdflatex -interaction=batchmode $d.tex 2>&1 >/dev/null | grep -o "Collected : [0-9]*" | grep -o "[0-9]*")
  echo "$(basename $E) $d $ir $(md5sum < $d.pdf | cut -c1-8)"
done
rm -rf $W
