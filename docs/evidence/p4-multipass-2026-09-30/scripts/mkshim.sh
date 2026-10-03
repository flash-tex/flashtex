#!/bin/bash
B=/Users/dqi26/flashtex-wt/d1-bin
for f in /usr/local/texlive/2026/bin/universal-darwin/*; do n=$(basename $f); [ "$n" = biber ] || ln -sf $f $B/$n; done
ls $B | wc -l
export PATH=$B:$PATH
kpsewhich biblatex.sty; which latexmk biber
