#!/bin/bash
# keys_matrix.sh ENGINE TAG FILE: keys_at.sh on (0-based) pages $PAGES (default 4 129 539 999) x
# start/middle/end of a prose line x a letter (no reflow, mostly) and twelve words (reflow), each
# session KEYS keystrokes GAP ms apart, one after another. Summary: keys_sum.py TAG FILE.
S=$(cd "$(dirname "$0")" && pwd)
E=$1; TAG=$2; FILE=$3
for page in ${PAGES:-4 129 539 999}; do
  for where in start middle end; do
    for sent in "" --sentence; do
      echo "== $page $where ${sent:-letter} $(date +%T) $(uptime | sed 's/.*load averages*: //')"
      KEYS=${KEYS:-6} bash $S/keys_at.sh $E $TAG "$FILE" $page $where $sent
    done
  done
done
echo "== end $(date +%T)"
