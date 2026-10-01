#!/bin/bash
# Pixel-parity matrix: Core Graphics rendering of typst-pdf's page vs Core Text
# drawing of the display list, for three position sources.
cd "$(dirname "$0")"
MB=841.8898
for spec in "p2 p2.pdf 2" "d300p150 d300p150.pdf 150"; do
  set -- $spec; J=$1; P=$2; PG=$3
  for src in typst pdfpos pdfpos-sp; do
    if [ "$src" = typst ]; then F=$J.json; else F=$J-$src.json; fi
    for s in 1 2 3; do
      for q in default subq0; do
        if [ "$q" = subq0 ]; then R=$(SUBQ=0 MBH=$MB ./pxdiff $F $P $PG $s); else R=$(MBH=$MB ./pxdiff $F $P $PG $s); fi
        echo "{\"doc\":\"$J\",\"positions\":\"$src\",\"ctx\":\"$q\",\"result\":$R}"
      done
    done
  done
done
