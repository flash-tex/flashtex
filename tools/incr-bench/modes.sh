#!/usr/bin/env bash
# modes.sh ENGINE [MODE...]: the performance modes' memory and keystroke cost (lane PERF-MODES;
# DESIGN.md §1.2 "Performance modes", §5.2). For each document and mode (default all three) it
# types through the socket host started with `--profile MODE` (mem.py: KEYS letters, default 8, on
# each of four pages in turn, 300 ms apart) and writes $INCR_BENCH_DIR/mem/DOC-mode-MODE.jsonl;
# then modes_table.py prints peak RSS and the instructions per keystroke side by side.
#
# Documents: MODES_DOCS (default "plain-1000 full-1000", from mkdocs.py) and, when it exists,
# Infinite Descent x2 (MODES_INFDESC, default ~/bench-docs/infdesc: infdesc-x2.tex, made by
# tools/parity/corpus/infdesc_x2.py), typed in two chapters in each copy of the book.
# ENGINE is an mkeng.sh engine under $INCR_BENCH_DIR. Runs one host at a time; a host above
# MODES_LIMIT_GB (default 8) is killed.
set -u
S=$(cd "$(dirname "$0")" && pwd)
B=${INCR_BENCH_DIR:-/tmp/incr-bench}
E=$1; shift
MODES=${*:-low-memory balanced high-performance}
KEYS=${KEYS:-8}
LIMIT=${MODES_LIMIT_GB:-8}
INF=${MODES_INFDESC:-$HOME/bench-docs/infdesc}
out=()
for m in $MODES; do
  for d in ${MODES_DOCS:-plain-1000 full-1000}; do
    n=${d##*-}
    python3 "$S/mem.py" "$E" "$d" --profile "$m" --pages "0,$((n * 3 / 10)),$((n * 6 / 10)),$((n - 1))" \
      --keys "$KEYS" --limit-gb "$LIMIT" --tag "mode-$m" --timeout 7200 | tail -1 | cut -c1-300
    out+=("$B/mem/$d-mode-$m.jsonl")
  done
  if [ -f "$INF/infdesc-x2.tex" ]; then
    # copy 1: sets (≈ p. 110), number theory (≈ p. 270); copy 2: the same, ≈ 575 pages on
    python3 "$S/mem.py" "$E" infdesc-x2 --src "$INF" --main infdesc-x2.tex --profile "$m" \
      --edit book/sets/set-operations.tex,book/number-theory/modular-arithmetic.tex,book/sets/set-operations.tex,book/number-theory/modular-arithmetic.tex \
      --pages 110,270,685,845 --keys "$KEYS" --limit-gb "$LIMIT" --tag "mode-$m" --timeout 14400 \
      | tail -1 | cut -c1-300
    out+=("$B/mem/infdesc-x2-mode-$m.jsonl")
  fi
done
python3 "$S/modes_table.py" "${out[@]}"
