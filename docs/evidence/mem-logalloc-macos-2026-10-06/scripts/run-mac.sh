#!/usr/bin/env bash
# run-mac.sh OUTDIR: the macOS measurements of the logalloc port (lane MEM-IMPL-HOST, item 5), on a
# team Mac through the self-hosted runner (.github/workflows/mem-macos-footprint.yml); never on the
# owner's Mac.
#
#  1. memfp.c (docs/evidence/mem-research-2026-10-06/ §2.7): what each way of giving memory back does
#     to phys_footprint, 512 MB each; MallocReportConfig=1 on `malloc`. No memory_pressure step: the
#     runner's Mac is shared, so a simulated pressure would hit everyone else's work.
#  2. The release host typing on plain-1000, full-1000 (and full-100): open, 6 letters at the middle,
#     3 sentences, 4 letters at the end, 8 s idle, an unchanged open (scripts/fpsession.py). A/B in
#     one binary: log mappings on (B, the default) and off (A, FLASHTEX_NO_LOG_MAPS=1, libmalloc for
#     everything as on main), alternating, ROUNDS rounds.
set -euo pipefail
OUT=${1:?OUTDIR}
ROUNDS=${ROUNDS:-2}
DOCS=${DOCS:-"full-1000 plain-1000 full-100"}
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../../../.." && pwd)
mkdir -p "$OUT"
{
  sw_vers; sysctl -n hw.model machdep.cpu.brand_string hw.memsize hw.pagesize
  uptime; vm_stat | head -12; memory_pressure -Q 2>/dev/null | tail -1 || true
} > "$OUT/machine.txt" 2>&1
cat "$OUT/machine.txt"

if [ "${SKIP_MEMFP:-0}" != 1 ]; then
  clang -O2 -Wall -o "$OUT/memfp" "$HERE/memfp.c"
  f="$OUT/memfp-$(sw_vers -productVersion)-$(sysctl -n hw.model).txt"
  for c in munmap remap dontneed free reusable reusable_noreuse purgeable file_shared file_private malloc; do
    nice "$OUT/memfp" $c 512
  done > "$f" 2>&1
  MallocReportConfig=1 "$OUT/memfp" malloc 512 > "$OUT/memfp-malloc-reportconfig.txt" 2>&1
  # idle compression without pressure (no memory_pressure here, see above)
  WAIT=30 "$OUT/memfp" compressed 512 > "$OUT/memfp-compressed-nopressure.txt" 2>&1
  rm -f "$OUT/memfp"
  cat "$f"
fi

export INCR_BENCH_DIR=${INCR_BENCH_DIR:-$RUNNER_TEMP/ib}
mkdir -p "$INCR_BENCH_DIR"
(cd "$ROOT" && cargo build --release --locked -p flashtex-engine -p flashtex-display-list)
(cd "$ROOT" && bash tools/incr-bench/mkeng.sh ft)
(cd "$ROOT" && python3 tools/incr-bench/mkdocs.py > /dev/null)
(cd "$ROOT" && cargo test --release --locked -p flashtex-engine --lib logalloc) | tail -5

for r in $(seq 1 "$ROUNDS"); do
  for d in $DOCS; do
    for v in B A; do
      env_args=()
      [ $v = A ] && env_args=(--env FLASHTEX_NO_LOG_MAPS=1)
      python3 "$HERE/fpsession.py" "$INCR_BENCH_DIR/ft" "$INCR_BENCH_DIR/docs/$d" "$OUT/$d-$v-r$r" \
        ${env_args[@]+"${env_args[@]}"} --step 0.5:letter:6 --step 0.5:sentence:3 --step 0.98:letter:4 \
        | tee -a "$OUT/sessions.txt" || echo "session $d $v r$r failed" | tee -a "$OUT/sessions.txt"
      rm -rf "$OUT/$d-$v-r$r-s0"
    done
  done
done
grep SUMMARY "$OUT/sessions.txt" || true
