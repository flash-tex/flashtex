#!/usr/bin/env bash
# mem_gate.sh [--build]: the resident host's memory gate (DESIGN.md §5.2's retention budget, §7;
# lane P4-MEMORY, docs/evidence/p4-memory-2026-09-30/). For each document it types on four pages
# (0, 30%, 60%, the last; KEYS keystrokes each, default 8) through the socket and fails if the
# host's peak resident memory passes the document's limit:
#
#   plain-120 0.6 GB   full-120 0.8 GB   plain-1000 1.2 GB   full-1000 1.8 GB
#
# (the budget: at most 1 GB of undo logs plus 0.5 GB of everything else; the measured peaks are
# in the evidence). A host above 6 GB is killed at once, so a regression cannot take the machine
# down. MEM_GATE_DOCS overrides the list ("DOC:GB ..."). Needs TeX Live 2026 (the format build)
# and a release build of flashtex-engine and flashtex-display-list (--build makes one).
set -euo pipefail
S=$(cd "$(dirname "$0")" && pwd)
W=$(cd "$S/../.." && pwd)
export INCR_BENCH_DIR=${INCR_BENCH_DIR:-${RUNNER_TEMP:-/tmp}/incr-bench-mem}
cd "$W"
if [ "${1:-}" = "--build" ]; then
  cargo build --release --locked -p flashtex-engine -p flashtex-display-list
fi
bash "$S/mkeng.sh" memgate
python3 "$S/mkdocs.py"
DOCS=${MEM_GATE_DOCS:-"plain-120:0.6 full-120:0.8 plain-1000:1.2 full-1000:1.8"}
fail=0
for d in $DOCS; do
  doc=${d%%:*}; gb=${d##*:}
  n=${doc##*-}
  pages="0,$((n * 3 / 10)),$((n * 6 / 10)),$((n - 1))"
  if python3 "$S/mem.py" memgate "$doc" --pages "$pages" --keys "${KEYS:-8}" --limit-gb 6 \
      --gate-gb "$gb" --tag gate --timeout 3000; then
    echo "mem gate: $doc within $gb GB"
  else
    echo "mem gate: $doc FAILED (limit $gb GB; see $INCR_BENCH_DIR/mem/$doc-gate.jsonl)"
    fail=1
  fi
done
python3 "$S/mem_table.py" "$INCR_BENCH_DIR"/mem/*-gate.jsonl
exit $fail
