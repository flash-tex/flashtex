#!/bin/sh
# Run snapshot-bench and write the evidence bundle.
#
# Timing here gates a design decision, and this host is shared with other build lanes, so
# the script refuses to measure under load: it waits for the 1-minute load average to fall
# below MAX_LOAD (default 3.0) and records the load average with every phase. A run taken
# under load is not comparable with one taken quiet -- observed spread on this machine was
# about 2x on the copy-fault measurement between a quiet run and a run with four `rustc`
# processes resident.
#
#   ./run.sh [OUT_DIR]
#
# Environment:
#   CARGO_BUILD_JOBS  parallel build cap (default 4)
#   MAX_LOAD          refuse to measure above this 1-minute load average (default 3.0)
#   WAIT_SECONDS      how long to wait for the machine to go quiet (default 1800)
#   REPS, PAGES, RETENTION_PAGES
#   SKIP_LOAD_CHECK=1 measure anyway, and mark the bundle as taken under load

set -eu
# `set -o pipefail` is not in POSIX sh; ask for it where the shell has it (dash, bash, zsh
# in sh mode all accept this form, and the `|| true` keeps a shell without it working).
(set -o pipefail) 2>/dev/null && set -o pipefail || true

HERE=$(cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(cd -- "$HERE/../.." && pwd)
OUT=${1:-$HERE/out}
: "${CARGO_BUILD_JOBS:=4}"
: "${MAX_LOAD:=3.0}"
: "${WAIT_SECONDS:=1800}"
: "${REPS:=30}"
: "${PAGES:=60}"
: "${RETENTION_PAGES:=1000}"
export CARGO_BUILD_JOBS

mkdir -p "$OUT"
BIN="$ROOT/target/release/snapshot-bench"

load1() { uptime | sed 's/.*load averages*: *//' | awk '{print $1}' | tr -d ','; }
quiet() { awk -v l="$(load1)" -v m="$MAX_LOAD" 'BEGIN { exit !(l < m) }'; }

echo "== building (CARGO_BUILD_JOBS=$CARGO_BUILD_JOBS) =="
( cd "$ROOT" && cargo build --release -p snapshot-bench )
echo "== correctness self-tests =="
( cd "$ROOT" && cargo test --release -p snapshot-bench )

if [ "${SKIP_LOAD_CHECK:-0}" != "1" ]; then
  waited=0
  while ! quiet; do
    if [ "$waited" -ge "$WAIT_SECONDS" ]; then
      echo "still loaded after ${WAIT_SECONDS}s (load $(load1) >= $MAX_LOAD)." >&2
      echo "re-run when the host is quiet, or set SKIP_LOAD_CHECK=1 to measure anyway." >&2
      exit 2
    fi
    [ "$waited" -eq 0 ] && echo "waiting for load < $MAX_LOAD (now $(load1))..."
    sleep 30
    waited=$((waited + 30))
  done
fi

{
  echo "host:        $(hostname)"
  echo "uname:       $(uname -a)"
  echo "cpu:         $(sysctl -n machdep.cpu.brand_string) ($(sysctl -n hw.ncpu) cpus)"
  echo "memory:      $(sysctl -n hw.memsize) bytes"
  echo "page size:   $(getconf PAGESIZE) bytes"
  echo "rustc:       $(rustc --version)"
  echo "date:        $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  echo "load before: $(load1)"
  echo "under load:  ${SKIP_LOAD_CHECK:-0}"
} > "$OUT/environment.txt"
cat "$OUT/environment.txt"

for phase in snapshot fault table dirty barrier hotloop retention; do
  echo "== phase $phase (load $(load1)) =="
  "$BIN" "$phase" \
    --reps "$REPS" --pages "$PAGES" --retention-pages "$RETENTION_PAGES" \
    --out "$OUT/$phase.jsonl" > "$OUT/$phase.md" 2>"$OUT/$phase.err" || {
      echo "phase $phase failed; see $OUT/$phase.err" >&2
      exit 1
    }
  echo "load after $phase: $(load1)" >> "$OUT/environment.txt"
done

echo "load after: $(load1)" >> "$OUT/environment.txt"
echo "== done: $OUT =="
