#!/usr/bin/env bash
# t7-reference.sh: the P4 gate's reference run, in one command (DESIGN.md §8 T7, §12; owner
# decision on Q1, 2026-10-05: a manual T7 run on an idle, plugged-in Apple-Silicon Mac with Low
# Power Mode off).
#
#   scripts/t7-reference.sh [--out DIR] [--max-load L] [--wait S] [--docs LIST] [--quick]
#                           [--no-build] [--check] [--force] [-- T7-ARGS...]
#
# It refuses to start unless this is an Apple-Silicon Mac on mains power, with Low Power Mode off,
# no thermal or CPU speed limit, a clean checkout, and TeX Live 2026 on PATH. It then builds
# the engine in release, waits until the 1-minute load is below L, and runs all of
# tools/incr-bench/t7.py: every document and phase, the typing rows included, with
# --require-reference. It writes the evidence directory:
#
#   DIR/README.md     the verdict, the commit, the machine and the command
#   DIR/table.md      T7's table
#   DIR/summary.json.gz, DIR/raw.tgz, DIR/environment.txt
#
# DIR defaults to docs/evidence/t7-reference-<UTC date>-<short host name>. Commit it on a branch
# and open a PR; the script itself never commits or pushes.
#
#   --max-load L  the load1 the run waits for and holds (default 2.0). Above it, at the start,
#                 during the run or at its end, T7 marks the run non-reference
#   --wait S      wait at most S seconds for the load (default 1800)
#   --docs LIST   t7.py --docs (default: all eight documents)
#   --quick       t7.py --quick (plain-10 and full-100, fewer keys): a smoke run, not the gate
#   --no-build    use the engine built by an earlier run (INCR_BENCH_DIR/ref)
#   --check       only the idle and machine checks, then exit (0: ready)
#   --force       run even where a check fails (another machine, battery...): for trying the
#                 script out. T7 then marks the run non-reference, and it exits 3 if nothing
#                 else failed
#
# Exit status: t7.py's (0: every gated row meets its target on a reference run; 1: a target
# missed; 2: harness error; 3: non-reference run), or 4 when a check refused to start.
# Expect about an hour with the build. Keep the Mac awake and plugged in, and leave it alone:
# t7.py holds caffeinate for the run.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

OUT=""
MAX_LOAD=2.0
WAIT=1800
DOCS=""
QUICK=0
BUILD=1
CHECK_ONLY=0
FORCE=0
T7_EXTRA=()

usage() { awk 'NR == 1 { next } /^#/ { sub(/^# ?/, ""); print; next } { exit }' "${BASH_SOURCE[0]}"; }

while [[ $# -gt 0 ]]; do
  case "$1" in
    --out) OUT="$2"; shift 2 ;;
    --max-load) MAX_LOAD="$2"; shift 2 ;;
    --wait) WAIT="$2"; shift 2 ;;
    --docs) DOCS="$2"; shift 2 ;;
    --quick) QUICK=1; shift ;;
    --no-build) BUILD=0; shift ;;
    --check) CHECK_ONLY=1; shift ;;
    --force) FORCE=1; shift ;;
    -h|--help) usage; exit 0 ;;
    --) shift; T7_EXTRA=("$@"); break ;;
    *) echo "t7-reference: unknown option $1 (see --help)" >&2; exit 4 ;;
  esac
done

OS="$(uname -s)"
ARCH="$(uname -m)"
HOST="$(hostname -s 2>/dev/null || hostname)"
DATE="$(date -u +%Y-%m-%d)"
[[ -n "$OUT" ]] || OUT="$ROOT/docs/evidence/t7-reference-$DATE-$HOST"
# (/tmp, not $TMPDIR: macOS's is long, and the hosts' socket paths must fit in 104 bytes)
export INCR_BENCH_DIR="${INCR_BENCH_DIR:-/tmp/flashtex-t7-reference}"
INCR_BENCH_DIR="${INCR_BENCH_DIR%/}"

problems=()
note() { echo "t7-reference: $*"; }

load1() {
  if [[ "$OS" == Darwin ]]; then
    sysctl -n vm.loadavg | awk '{ print $2 }'
  else
    cut -d' ' -f1 /proc/loadavg
  fi
}

# The machine and its state. Each failed check is one line in `problems`.
check_machine() {
  problems=()
  if [[ "$OS" != Darwin || "$ARCH" != arm64 ]]; then
    problems+=("not an Apple-Silicon Mac ($OS $ARCH)")
  fi
  if [[ "$OS" == Darwin ]]; then
    local batt therm
    batt="$(pmset -g batt 2>/dev/null || true)"
    if ! grep -q "'AC Power'" <<<"$batt"; then
      problems+=("not on mains power: $(head -1 <<<"$batt")")
    fi
    if pmset -g 2>/dev/null | awk '$1 == "lowpowermode" && $2 != "0" { found = 1 } END { exit !found }'; then
      problems+=("Low Power Mode is on (System Settings > Battery)")
    fi
    therm="$(pmset -g therm 2>/dev/null || true)"
    if awk -F'=' '/Speed_Limit/ { gsub(/ /, "", $2); if ($2 + 0 < 100) found = 1 } END { exit !found }' <<<"$therm"; then
      problems+=("a CPU speed limit is in force: $(grep Speed_Limit <<<"$therm" | tr '\n' ' ')")
    fi
  elif [[ -d /sys/class/power_supply ]]; then
    local d
    for d in /sys/class/power_supply/*; do
      [[ -f "$d/type" && "$(cat "$d/type")" == Mains && "$(cat "$d/online" 2>/dev/null)" == 0 ]] &&
        problems+=("not on mains power ($d)")
    done
  fi
  if [[ -n "$(git status --porcelain --untracked-files=no)" ]]; then
    problems+=("the checkout has uncommitted changes: a reference run is of a commit")
  fi
  local t
  for t in cargo python3 kpsewhich; do
    command -v "$t" >/dev/null || problems+=("$t is not on PATH")
  done
  if command -v kpsewhich >/dev/null && ! kpsewhich pdflatex.ini >/dev/null 2>&1; then
    problems+=("kpsewhich cannot find pdflatex.ini: TeX Live 2026's bin directory must be on PATH")
  fi
}

check_machine
note "$(date -u +%FT%TZ) $HOST $OS $ARCH, load1 $(load1), commit $(git rev-parse --short HEAD)"
if [[ ${#problems[@]} -gt 0 ]]; then
  for p in "${problems[@]}"; do note "check failed: $p"; done
  if [[ $FORCE == 0 ]]; then
    note "not started (--force runs anyway, as a non-reference run)"
    exit 4
  fi
  note "--force: running anyway; the run is not a reference run"
else
  note "machine checks pass"
fi
if [[ $CHECK_ONLY == 1 ]]; then
  awk -v l="$(load1)" -v m="$MAX_LOAD" 'BEGIN { exit !(l + 0 < m + 0) }' &&
    note "load1 $(load1) < $MAX_LOAD: ready" || note "load1 $(load1) >= $MAX_LOAD: the run would wait for it"
  exit 0
fi

if [[ -e "$OUT" ]]; then
  note "$OUT exists; give another --out"
  exit 4
fi
mkdir -p "$OUT"
RUN="$INCR_BENCH_DIR/t7-reference-run"
rm -rf "$RUN"

args=(--engine ref --out "$RUN" --max-load "$MAX_LOAD" --wait-load "$MAX_LOAD" --wait-max "$WAIT")
[[ $BUILD == 1 ]] && args+=(--build)
[[ $FORCE == 1 ]] || args+=(--require-reference)
[[ $QUICK == 1 ]] && args+=(--quick)
[[ -n "$DOCS" ]] && args+=(--docs "$DOCS")
args+=("${T7_EXTRA[@]+"${T7_EXTRA[@]}"}")
NCPU="$( (sysctl -n hw.ncpu 2>/dev/null || nproc) | head -1)"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-$NCPU}"
note "python3 tools/incr-bench/t7.py ${args[*]}"
set +e
python3 tools/incr-bench/t7.py "${args[@]}" 2>&1 | tee "$OUT/t7-output.txt"
code=${PIPESTATUS[0]}
set -e

# The evidence directory.
[[ -f "$RUN/table.md" ]] && cp "$RUN/table.md" "$OUT/table.md"
[[ -f "$RUN/summary.json" ]] && gzip -c "$RUN/summary.json" > "$OUT/summary.json.gz"
[[ -d "$RUN/raw" ]] && tar -czf "$OUT/raw.tgz" -C "$RUN" raw
{
  [[ -f "$RUN/environment.txt" ]] && cat "$RUN/environment.txt"
  echo "commit $(git rev-parse HEAD)"
  if [[ "$OS" == Darwin ]]; then
    sw_vers 2>/dev/null | tr '\n' ' '; echo
    sysctl -n hw.model machdep.cpu.brand_string hw.ncpu hw.memsize 2>/dev/null | tr '\n' ' '; echo
    pmset -g batt 2>/dev/null | head -1
    pmset -g 2>/dev/null | grep -i lowpowermode || true
  fi
} > "$OUT/environment.txt"
verdict="$(grep -E '^T7: ' "$OUT/t7-output.txt" | head -1 || true)"
case $code in
  0) meaning="every gated row meets its target, on a reference run" ;;
  1) meaning="a gated target missed (the table's MISS rows)" ;;
  2) meaning="harness error: see t7-output.txt" ;;
  3) meaning="non-reference run: see the table's first line" ;;
  *) meaning="t7.py exit $code" ;;
esac
{
  echo "# T7 reference run, $DATE, $HOST"
  echo
  echo "P4 gate (DESIGN.md §12; Q1 decided 2026-10-05): \`scripts/t7-reference.sh\`, the full T7 run on an"
  echo "idle, plugged-in Apple-Silicon Mac with Low Power Mode off."
  echo
  echo "- **Verdict:** ${verdict:-none (see t7-output.txt)}"
  echo "- **Exit:** $code: $meaning"
  echo "- **Commit:** \`$(git rev-parse --short HEAD)\` ($(git log -1 --format=%s | head -c 100))"
  echo "- **Machine:** $(if [[ "$OS" == Darwin ]]; then sysctl -n hw.model machdep.cpu.brand_string 2>/dev/null | tr '\n' ' '; sw_vers -productVersion 2>/dev/null; else uname -srm; fi)"
  [[ ${#problems[@]} -gt 0 ]] && echo "- **Checks failed (--force):** $(IFS=';'; echo "${problems[*]}")"
  echo "- **Command:** \`scripts/t7-reference.sh $(printf '%q ' "${args[@]}" | sed 's/ $//')\` (as t7.py's arguments)"
  echo
  echo "Files: \`table.md\` (the rows), \`summary.json.gz\` (\`t7.py --check\` re-evaluates it after"
  echo "gunzip), \`raw.tgz\` (dl3-keys' lines), \`environment.txt\`, \`t7-output.txt\`."
  echo
  [[ -f "$OUT/table.md" ]] && cat "$OUT/table.md"
} > "$OUT/README.md"
rm -rf "$RUN"
note "exit $code ($meaning); evidence in $OUT"
note "next: git checkout -b <your-branch>; git add '$OUT'; commit; open a PR"
exit "$code"
