#!/bin/bash
# book_matrix.sh ENGINE TAG: the owner's test case -- book.tex, edits on (0-based) pages 4, 129,
# 539, 999, at the start, middle and end of a paragraph's line, a letter (no reflow, mostly) and
# twelve words (reflow), each session KEYS keystrokes GAP ms apart (book.sh), one after another.
S=$(cd "$(dirname "$0")" && pwd)
E=$1; TAG=$2
for page in ${PAGES:-4 129 539 999}; do
  for where in start middle end; do
    for sent in "" --sentence; do
      echo "== $page $where ${sent:-letter} $(date +%T) $(uptime | sed 's/.*load averages: //')"
      KEYS=${KEYS:-6} bash $S/book.sh $E $TAG $page $where $sent
    done
  done
done
echo "== end $(date +%T)"
