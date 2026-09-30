#!/bin/bash
# ab.sh ENGINE DOC AT GAP ROUNDS NAME=HOSTARGS...: interleaved A/B of host options, keystrokes
# GAP ms apart (the app's bench types 300 ms apart), no viewport (page 1 on screen), each
# configuration once per round; prints the host stages of each run (stages.py).
S=$(cd "$(dirname "$0")/../../../../tools/incr-bench" && pwd)
export INCR_BENCH_DIR=/tmp/p4f
E=$1; DOC=$2; AT=$3; G=$4; R=$5; shift 5
for r in $(seq 1 $R); do
  for cfg in "$@"; do
    name=${cfg%%=*}; args=${cfg#*=}
    HOSTARGS="$args" KEYS=${KEYS:-20} KEYARGS="--no-viewport --gap-ms $G $XARGS" bash $S/keys.sh $E $DOC $AT $name-r$r > /dev/null 2>&1
    echo "$name round $r $(uptime | sed 's/.*load averages: //'): $(python3 $S/stages.py /tmp/p4f/keys/$DOC-$AT-$name-r$r.jsonl | tail -1 | tr ' ' '\n' | grep -A1 -E '^(client_first_page|first_page_cpu|client_done)$' | grep -v -- -- | paste -sd' ' -)"
  done
done
