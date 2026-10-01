#!/bin/bash
# collect.sh: copy the benchmark records and summarised profiles of
# /tmp/l6o into raw/ (the sample files themselves are gzipped beside them).
set -e
H=$(cd "$(dirname "$0")/.." && pwd)
R=$H/raw
S=$H/scripts/sampletop.py
mkdir -p "$R/profiles" "$R/bench-mac"
cp /tmp/l6o/bench-*.jsonl "$R/bench-mac/"
summ() { # out files...
  local out=$1; shift
  python3 "$S" "$@" --top 60 > "$R/profiles/$out.txt"
}
summ plain-1000-base /tmp/l6o/sample-base-[123].txt
summ plain-1000-fix /tmp/l6o/sample-fix-plain-1000-[123].txt
summ plain-1000-u1 /tmp/l6o/sample-u1-plain-1000-[123].txt
summ full-1000-u1 /tmp/l6o/sample-u1-full-1000-1.txt
summ hello-u1 /tmp/l6o/sample-u1-hello-*.txt
tar -czf "$R/profiles/samples.tar.gz" -C /tmp/l6o $(cd /tmp/l6o && ls sample-base-[123].txt sample-fix-plain-1000-*.txt sample-u1-*.txt)
ls -la "$R/profiles" "$R/bench-mac"
