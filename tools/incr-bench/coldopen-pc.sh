#!/usr/bin/env bash
# coldopen-pc.sh WORK REPS PERF REF...: the first open of Infinite Descent x2 (1,152 pages) on the
# NixOS PC's self-hosted runner, against pdflatex (lane COLD-FIRST-2; .github/workflows/coldopen-pc.yml).
#
#   WORK   a scratch directory (the runner's temp): engines, results, the oracle's run
#   REPS   cold opens per ref, interleaved (ref 1, ref 2, ..., then again)
#   PERF   1: also one `perf record` (instructions:u) of the first ref's open, its top symbols
#   REF    git refs to build (release) and open, each in a worktree of this checkout
#
# The book is mkbook.py's (fetched at its pinned commit, checked by SHA-256, cached under
# $COLDOPEN_CACHE, default ~/.cache/flashtex-ci/coldopen). pdflatex runs the document's three passes
# as latexmk would (imakeidx runs makeindex itself in restricted shell escape), each under
# `perf stat`. The host runs as the app opens a document: coldopen.py, `--profile balanced`,
# `--external-tools auto`, a fresh copy and a fresh S0 cache each rep. Everything runs under nice;
# the runner service is already in flashtex.slice. Writes WORK/r/*.jsonl, WORK/r/pdflatex.txt,
# WORK/r/perf-top.txt and WORK/r/summary.md.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORK=${1:?WORK}
REPS=${2:?REPS}
PERF=${3:?PERF}
shift 3
[ $# -ge 1 ] || { echo "coldopen-pc.sh: no ref" >&2; exit 2; }
CACHE=${COLDOPEN_CACHE:-$HOME/.cache/flashtex-ci/coldopen}
mkdir -p "$WORK/r" "$CACHE"
export INCR_BENCH_DIR="$WORK/ib" TMPDIR="$WORK/tmp"
mkdir -p "$TMPDIR"
mkdir -p "$INCR_BENCH_DIR"
# the archive cache is kept between runs; the unpacked document is made fresh
mkdir -p "$CACHE/cache"
ln -sfn "$CACHE/cache" "$INCR_BENCH_DIR/cache"
DOC=$(python3 "$ROOT/tools/incr-bench/mkbook.py" infdesc-x2 | tail -1)
echo "document: $DOC"

# A `perf` command: the system's, else nixpkgs'.
if command -v perf >/dev/null 2>&1; then PERF_CMD=(perf); else PERF_CMD=(nix shell nixpkgs#perf -c perf); fi

names=()
for ref in "$@"; do
  n=$(echo "$ref" | tr -c 'A-Za-z0-9._\n-' '_')
  names+=("$n")
  src="$WORK/src-$n"
  git -C "$ROOT" worktree remove --force "$src" 2>/dev/null || true
  git -C "$ROOT" worktree add --detach "$src" "$ref"
  echo "build $ref ($(git -C "$src" rev-parse --short HEAD))"
  (cd "$src" && nice -n 5 cargo build --release --locked -p flashtex-engine -p flashtex-display-list)
  (cd "$src" && bash tools/incr-bench/mkeng.sh "$n" >/dev/null)
  git -C "$ROOT" worktree remove --force "$src"
done

# passes LABEL ENV CMD...: the document's three passes (a fresh copy), each under perf stat
# (instructions and cycles of user space, children included: makeindex under \write18) and timed,
# with the environment ENV (comma-separated assignments; @DL: a display-list file); a line a pass.
passes() {
  local label=$1 envs=$2; shift 2
  local d="$WORK/p-$label" i s e ins cyc
  local -a ev=()
  if [ -n "$envs" ]; then IFS=, read -r -a ev <<< "${envs//@DL/$WORK/r/$label.dl}"; fi
  rm -rf "$d"; cp -R "$DOC" "$d"
  for i in 1 2 3; do
    s=$(date +%s.%N)
    (cd "$d" && env SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 "${ev[@]}" nice -n 5 \
      "${PERF_CMD[@]}" stat -x, -o "$WORK/r/$label-stat-$i.csv" -e instructions:u,cycles:u \
      "$@" -interaction=batchmode infdesc-x2.tex >/dev/null 2>&1) || true
    e=$(date +%s.%N)
    ins=$(awk -F, '/instructions/ {printf "%.1f", $1/1e9}' "$WORK/r/$label-stat-$i.csv" 2>/dev/null || true)
    cyc=$(awk -F, '/cycles/ {printf "%.1f", $1/1e9}' "$WORK/r/$label-stat-$i.csv" 2>/dev/null || true)
    echo "$label pass $i: $(awk "BEGIN { printf \"%.1f\", $e - $s }") s, ${ins:-?} G instructions, ${cyc:-?} G cycles, load1 $(cut -d' ' -f1 /proc/loadavg); $(grep -o 'Output written.*' "$d/infdesc-x2.log" || echo 'no output')"
  done
  rm -rf "$d" "$WORK/r/$label.dl"
}

# pdflatex: three passes, each under perf stat (instructions, cycles of user space) and timed.
{
  echo "pdflatex: $(pdflatex --version | head -1)"
  passes pdflatex "" pdflatex
} 2>&1 | tee "$WORK/r/pdflatex.txt"

# The first ref's engine as a plain command-line compile (no host), in the variants CLI_VARIANTS
# (space-separated NAME or NAME:ENV,ENV...), e.g. `plain intr-off:FLASHTEX_INTRINSICS=off
# dl:FLASHTEX_DISPLAY_LIST=@DL`. CLI_PERF=1: also a perf record of the first variant's pass 1.
n0=${names[0]}
cli_env="PATH=$INCR_BENCH_DIR/$n0:$PATH,FLASHTEX_FORMATS=$INCR_BENCH_DIR/fmt-$n0,FLASHTEX_POOL=$INCR_BENCH_DIR/$n0/pdftex.pool"
for v in ${CLI_VARIANTS:-}; do
  name=${v%%:*}; extra=""
  [ "$v" != "$name" ] && extra=",${v#*:}"
  passes "cli-$name" "$cli_env$extra" "$INCR_BENCH_DIR/$n0/pdftex" -fmt=pdflatex
done 2>&1 | tee "$WORK/r/cli.txt"
if [ "${CLI_PERF:-0}" = 1 ] && [ -n "${CLI_VARIANTS:-}" ]; then
  v=${CLI_VARIANTS%% *}; extra=""
  [ "$v" != "${v%%:*}" ] && extra=",${v#*:}"
  IFS=, read -r -a ev <<< "$cli_env$extra"
  rm -rf "$WORK/p-perf"; cp -R "$DOC" "$WORK/p-perf"
  (cd "$WORK/p-perf" && env "${ev[@]}" nice -n 5 "${PERF_CMD[@]}" record -m 8 -e instructions:u -F 500 \
    -o "$WORK/r/cli-perf.data" -- "$INCR_BENCH_DIR/$n0/pdftex" -fmt=pdflatex -interaction=batchmode infdesc-x2.tex \
    >/dev/null 2>&1) || true
  "${PERF_CMD[@]}" report -i "$WORK/r/cli-perf.data" --no-children --sort symbol --stdio -F overhead,sym 2>/dev/null \
    | grep -v '^#' | grep -v '^$' | sort -t% -k1 -rn | head -150 > "$WORK/r/cli-perf-top.txt" || true
  rm -rf "$WORK/r/cli-perf.data" "$WORK/p-perf"
fi

# The first ref's host in the variants HOST_VARIANTS (NAME:ENV,ENV...; a cold open each), e.g.
# `nodiag:FLASHTEX_NO_DIAGNOSTICS=1 nobm:FLASHTEX_BOXMEMO=off`.
for v in ${HOST_VARIANTS:-}; do
  name=${v%%:*}
  IFS=, read -r -a ev <<< "${v#*:}"
  env "${ev[@]}" nice -n 5 python3 "$ROOT/tools/incr-bench/coldopen.py" "$INCR_BENCH_DIR/$n0" "$DOC" infdesc-x2.tex \
    "$WORK/r/host-$name.jsonl" --external-tools auto --host-arg=--profile --host-arg=balanced --tag "host-$name" 2>&1 \
    | tail -1 | sed "s/^/host-$name: /"
done

for rep in $(seq 1 "$REPS"); do
  for n in "${names[@]}"; do
    nice -n 5 python3 "$ROOT/tools/incr-bench/coldopen.py" "$INCR_BENCH_DIR/$n" "$DOC" infdesc-x2.tex \
      "$WORK/r/$n.jsonl" --external-tools auto --host-arg=--profile --host-arg=balanced --tag "$n" 2>&1 \
      | tail -1 | sed "s/^/$n: /"
  done
done

if [ "$PERF" = 1 ]; then
  n=${names[0]}
  nice -n 5 "${PERF_CMD[@]}" record -m 8 -e instructions:u -F 500 -o "$WORK/r/perf.data" -- \
    python3 "$ROOT/tools/incr-bench/coldopen.py" "$INCR_BENCH_DIR/$n" "$DOC" infdesc-x2.tex \
    "$WORK/r/perf-$n.jsonl" --external-tools auto --host-arg=--profile --host-arg=balanced --tag "perf-$n" \
    > "$WORK/r/perf.out" 2>&1 || true
  "${PERF_CMD[@]}" report -i "$WORK/r/perf.data" --no-children --sort symbol --stdio -F overhead,sym 2>/dev/null \
    | grep -v '^#' | grep -v '^$' | sort -t% -k1 -rn | head -150 > "$WORK/r/perf-top.txt" || true
  rm -f "$WORK/r/perf.data"
fi

python3 - "$WORK/r" "${names[@]}" > "$WORK/r/summary.md" <<'EOF'
import glob, json, os, sys
d, names = sys.argv[1], sys.argv[2:]
print('| infdesc-x2 cold open (PC) | first page | pass 1 done | all pages seen | DONE | pages/s (pass 1) '
      '| instructions per pass | last pass cycles | peak RSS | load1 |')
print('|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|')
files = [(n, f'{d}/{n}.jsonl') for n in names]
files += [(os.path.basename(f)[:-6], f) for f in sorted(glob.glob(f'{d}/host-*.jsonl'))]
for n, path in files:
    if not os.path.exists(path):
        continue
    for line in open(path):
        r = json.loads(line)
        dones = [x for x in r.get('dones') or [] if x[0] == 1]
        p1 = dones[0][1] / 1000 if dones else 0
        per = ' / '.join(f'{x[5] / 1e6:.1f}' for x in dones if x[5])
        cyc = (r.get('host') or {}).get('stages', {}).get('cycles_k', 0) / 1e6
        print(f"| {n} | {r['first_page_ms']:.0f} ms | {p1:.1f} s | {r['all_seen_ms']/1000:.1f} s | {r['done_ms']/1000:.1f} s "
              f"| {1142 / p1 if p1 else 0:.1f} | {per} G | {cyc:.1f} G | {r['peak_rss'] / 1e6 if r.get('peak_rss') else 0:,.0f} MB "
              f"| {r['load1'][0]}-{r['load1'][1]} |")
for f in ('pdflatex.txt', 'cli.txt'):
    if os.path.exists(f'{d}/{f}'):
        print(f'\n{f}:\n```\n' + open(f'{d}/{f}').read() + '```')
EOF
cat "$WORK/r/summary.md"
