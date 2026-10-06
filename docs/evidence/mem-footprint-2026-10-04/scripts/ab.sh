#!/bin/bash
# ab.sh TAG "ENG..." : the three documents, engines interleaved, one host at a time.
T=$1; shift
for e in $1; do
  python3 /tmp/mfp/run.py $T-$e $e full-100 --edits main.tex:10,main.tex:50,main.tex:90 --keys 8 2>/dev/null | tail -1
done
for e in $1; do
  python3 /tmp/mfp/run.py $T-$e $e long-deck --edits main.tex:38:124,main.tex:94:295 --keys 8 2>/dev/null | tail -1
done
if [ -z "$NOINF" ]; then
for e in $1; do
  python3 /tmp/mfp/run.py $T-$e $e infdesc --main infdesc.tex --edits book/logical-structure/propositional-logic.tex:40,book/number-theory/divisibility.tex:250,book/real-numbers/series-sums.tex:400 --keys 6 --limit-gb 6 2>/dev/null | tail -1
done
fi
echo ALLDONE
