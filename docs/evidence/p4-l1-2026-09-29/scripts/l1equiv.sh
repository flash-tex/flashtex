#!/bin/bash
# L1 equivalence: compile DOC in the resident host (cold, then from S0 after
# edits to EDITFILE), then compile the same final sources with the CLI engine
# (twice, as a user would: the second run reads the first run's .aux), and
# compare the job's .pdf/.log/.aux byte for byte.
# usage: l1equiv.sh BINNAME DOC EDITFILE REPS
name=$1; doc=$2; edit=$3; reps=${4:-2}
a=/tmp/p4l1/eqA-$doc; b=/tmp/p4l1/eqB-$doc
rm -rf $a $b; mkdir -p $a $b; cp /tmp/p4l1/docs/* $a/
/tmp/p4l1/host.sh $name $a bench --reps $reps --edit $edit -- -fmt=pdflatex -interaction=batchmode $doc.tex | tail -1
cp /tmp/p4l1/docs/* $b/; cp $a/$edit $b/$edit
cd $b
export FLASHTEX_POOL=/tmp/p4l1/$name/pdftex.pool FLASHTEX_FORMATS=/tmp/p4l1/fmt-$name SOURCE_DATE_EPOCH=1700000000 FORCE_SOURCE_DATE=1 FLASHTEX_PIN_CLOCK=1700000000.5
/tmp/p4l1/$name/pdftex -fmt=pdflatex -interaction=batchmode $doc.tex >/dev/null 2>&1
# the host's last compile read the .aux its previous compile wrote; give the CLI the same one
cp $a/$doc.aux $b/$doc.aux
/tmp/p4l1/$name/pdftex -fmt=pdflatex -interaction=batchmode $doc.tex >/dev/null 2>&1
st=0
for e in pdf log aux; do
  if cmp -s $a/$doc.$e $b/$doc.$e; then echo "$doc.$e identical ($(wc -c < $a/$doc.$e) bytes)"; else echo "$doc.$e DIFFERS"; st=1; fi
done
exit $st
