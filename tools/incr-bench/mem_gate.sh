#!/usr/bin/env bash
# mem_gate.sh [--build]: the resident host's memory gate (DESIGN.md §5.2's budget, §7; lanes
# P4-MEMORY, docs/evidence/p4-memory-2026-09-30/, and P4-MEMORY-BUDGET). Per document, two runs,
# each with its own host, and the gate is the higher of their peak resident memories:
#
#   typing  mem.py: KEYS keystrokes (default 8) on each of four pages (0, 30%, 60%, the last);
#   T7      t7.py: every T7 phase but reopen (letters at start, middle and end, a sentence, a
#           newline, a paragraph split, the preamble), T7_KEYS keystrokes each (default 8) and
#           T7_PREAMBLE preamble edits (default 2). Sentences and splits that change the page
#           count re-typeset the rest of the document, and the preamble runs it again from the
#           format: the phases that hold the most at once.
#
# Limits (the commander's ruling of 2026-10-06: the host's total RSS at its peak, over every T7
# phase, within 1.0 GB at 1,000 pages):
#
#   plain-120 0.6 GB   full-120 0.8 GB   plain-1000 1.0 GB   full-1000 1.0 GB
#
# A host above 6 GB is killed at once, so that a regression cannot take the machine down.
# MEM_GATE_DOCS overrides the list ("DOC:GB ..."); MEM_GATE_T7=0 leaves the T7 runs out. Needs
# TeX Live 2026 (the format build) and a release build of flashtex-engine and
# flashtex-display-list (--build makes one).
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
DOCS=${MEM_GATE_DOCS:-"plain-120:0.6 full-120:0.8 plain-1000:1.0 full-1000:1.0"}
PHASES=letter@start,letter@middle,letter@end,sentence@middle,newline@middle,split@middle,preamble
fail=0
for d in $DOCS; do
  doc=${d%%:*}; gb=${d##*:}
  n=${doc##*-}
  pages="0,$((n * 3 / 10)),$((n * 6 / 10)),$((n - 1))"
  python3 "$S/mem.py" memgate "$doc" --pages "$pages" --keys "${KEYS:-8}" --limit-gb 6 \
    --tag gate --timeout 3000 >/dev/null || true
  typing=$(python3 -c 'import json,sys
s=[json.loads(l) for l in open(sys.argv[1]) if "\"summary\"" in l]
print(s[-1]["peak_rss"] if s and not s[-1]["killed_at_limit"] else -1)' "$INCR_BENCH_DIR/mem/$doc-gate.jsonl")
  t7=0
  if [ "${MEM_GATE_T7:-1}" != 0 ]; then
    out=$INCR_BENCH_DIR/mem/t7-$doc
    rm -rf "$out"
    python3 "$S/t7.py" --engine memgate --docs "$doc" --phases "$PHASES" --keys "${T7_KEYS:-8}" \
      --preamble "${T7_PREAMBLE:-2}" --limit-gb 6 --out "$out" --timeout 3000 >"$out.log" 2>&1 || true
    t7=$(python3 -c 'import json,sys
try:
    s = json.load(open(sys.argv[1]))
except (OSError, ValueError):
    print(-1); sys.exit()
r = [d for d in s.get("raw", []) if d.get("doc") == sys.argv[2]]
ok = r and not s.get("error") and not r[0].get("killed_at_limit") and r[0].get("rss_peak")
print(r[0]["rss_peak"] if ok else -1)' "$out/summary.json" "$doc")
  fi
  python3 - "$doc" "$gb" "$typing" "$t7" <<'EOF' || fail=1
import sys
doc, gb, typing, t7 = sys.argv[1], float(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4])
mb = lambda b: f'{b / 2**20:.0f} MB' if b > 0 else ('not run' if b == 0 else 'FAILED to measure')
peak = max(typing, t7)
ok = typing > 0 and t7 >= 0 and peak <= gb * 2**30
print(f'mem gate: {doc} {"within" if ok else "FAILED:"} {gb} GB (typing {mb(typing)}, T7 phases {mb(t7)})')
sys.exit(0 if ok else 1)
EOF
done
python3 "$S/mem_table.py" "$INCR_BENCH_DIR"/mem/*-gate.jsonl
exit $fail
