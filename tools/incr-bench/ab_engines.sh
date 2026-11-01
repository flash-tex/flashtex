#!/usr/bin/env bash
# ab_engines.sh DOC PAGE GAP ROUNDS NAME=ENGINE:HOSTARGS...: interleaved runs of keys.sh on
# $INCR_BENCH_DIR/docs/DOC, typing in the middle of a prose line on (0-based) page PAGE, keystrokes
# GAP ms apart, no viewport (as the app sends it with page 1 on screen); one line of stages per run.
IB=${INCR_BENCH_DIR:-/tmp/incr-bench}
S=$(cd "$(dirname "$0")" && pwd)
DOC=$1; PAGE=$2; G=$3; R=$4; shift 4
for r in $(seq 1 $R); do
  for cfg in "$@"; do
    name=${cfg%%=*}; rest=${cfg#*=}; eng=${rest%%:*}; args=${rest#*:}
    HOSTARGS="$args" KEYS=${KEYS:-30} KEYARGS="--no-viewport --gap-ms $G --page $PAGE" \
      bash $S/keys.sh $eng $DOC 0 $name-r$r > /dev/null 2>&1
    f=$IB/keys/$DOC-0-$name-r$r.jsonl
    echo "$DOC $name round $r ($(uptime | sed 's/.*load averages*: //')): $(python3 $S/stages.py $f | tail -1 | tr ' ' '\n' | grep -A1 -E '^(client_first_page|client_edited_page|first_page_cpu|client_done)$' | grep -v -- -- | paste -sd' ' -)"
  done
done
