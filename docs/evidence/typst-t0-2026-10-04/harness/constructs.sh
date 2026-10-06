#!/bin/bash
# Construct-class rows measurable with the text harness: non-black colour and
# alpha on text (T0, 2026-10-04). Writes ../raw/constructs.jsonl.
cd "$(dirname "$0")/proto-copy"
T=../target/release/tbench
OUT=../raw/constructs.jsonl
: > $OUT
mkdir -p px/colour px/alpha
$T dump docs/colour 0 px/colour/c1.json 2>/dev/null
$T dump docs/alpha 0 px/alpha/a1.json 2>/dev/null
../venv/bin/python px/pdfpos2.py px/colour/c1.json px/colour/c1.pdf 1 px/colour/c1-pos2.json
../venv/bin/python px/pdfpos2.py px/alpha/a1.json px/alpha/a1.pdf 1 px/alpha/a1-pos2.json
for spec in "colour/c1-pos2.json colour/c1.pdf colour" "alpha/a1-pos2.json alpha/a1.pdf alpha"; do
  set -- $spec
  for s in 2 3; do
    for col in frame device icc; do
      R=$(POS=cg SHAPES=pdf SUBQ=0 COLOR=$col ../pxdiff2 px/$1 px/$2 1 $s ../png/$3-$s-$col)
      echo "{\"doc\":\"$3\",\"load\":\"$(uptime | sed 's/.*load averages: //')\",\"result\":$R}" | tee -a $OUT
    done
  done
done
python3 -c "
import json
for f in ['px/colour/c1-pos2.json','px/alpha/a1-pos2.json']:
    d=json.load(open(f))
    print(f, {k:{'kind':v['kind'],'n':v.get('n')} for k,v in d['colorspaces'].items()}, d['extgstates'])
    print(sorted({(p['cs'],tuple(round(c,6) for c in p['comps']),p['ca']) for p in d['run_paint']}))
"
