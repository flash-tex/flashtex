#!/usr/bin/env bash
# The new engine's parity gates (docs/design/engine-v2/DESIGN.md §8 and §9.2),
# exactly as CI's `engine parity` job runs them, so the job only confirms a
# result you can get first on a machine with TeX Live:
#
#   build     cargo build --release -p flashtex-engine (flashtex-initex), then
#             pdflatex.fmt and pdftex.fmt from TeX Live's .ini files, as
#             fmtutil makes them (-etex, cp227.tcx)
#   lockstep  T1: tools/lockstep, every case, against TeX Live's pdftex, in
#             --jobs interleaved shards run at once (run.py --shard K/N)
#   parity    P-T1 and P-T2 on the parity fixtures tier against TeX Live's
#             pdftex; fails unless EVERY measured fixture passes both
#   tests     cargo test --release -p flashtex-engine with
#             FLASHTEX_REQUIRE_TEXLIVE=1, so a test that would skip for want
#             of TeX Live fails instead
#
# and two nightly steps (nightly.yml), not in the default set:
#
#   t2        T2: the LaTeX team's suites (tools/latex-suites, fetched at
#             their pinned SHAs) through TeX Live's pdftex and through the
#             engine; fails if the engine fails a test the reference passes.
#             Needs `build` first
#   soundness the fixture-wide incremental soundness test, which `cargo test`
#             skips (#[ignore]): every fixture edited through the socket host
#             must equal a from-scratch compile after every edit; each
#             fixtures/multipass document, with the bibtex/makeindex runs
#             its PASSES file gives, must equal its from-scratch sequence
#
# Usage: scripts/engine-parity.sh [--jobs N] [--shard K/N] [--work DIR] [STEP...]
# Default steps: build lockstep parity tests. A step after `build` reuses the
# binary and format in the work directory (default: $RUNNER_TEMP or /tmp,
# under engine-parity/).
#
# --shard K/N (0 <= K < N, as tools/parity's) runs slice K of N of `parity`
# (every N-th fixture, parity.py --shard) and of `tests` (every N-th test
# target of flashtex-engine: the library, the binaries, each tests/*.rs; the
# doc tests in slice 0). The N slices together are the whole step. CI's
# merge-queue gate runs the steps as separate jobs: `build lockstep`, and
# `build parity` and `tests` in two slices each (ci.yml, engine-parity-hosted).
#
# Needs TeX Live 2026 FIRST on PATH (pdfTeX 1.40.29, the pinned oracle;
# another TeX Live's kpsewhich earlier on PATH makes the engine read the wrong
# tree), python3 and qpdf. Prints one `TIME <step> <seconds>` line per step,
# and appends a table of them to $GITHUB_STEP_SUMMARY when that is set.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

JOBS="$(( $(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 4) / 2 ))"
WORK="${RUNNER_TEMP:-/tmp}/engine-parity"
STEPS=()
SHARD=''
while [[ $# -gt 0 ]]; do
  case "$1" in
    --jobs) JOBS="${2:?--jobs needs a number}"; shift 2 ;;
    --work) WORK="${2:?--work needs a directory}"; shift 2 ;;
    --shard) SHARD="${2:?--shard needs K/N}"; shift 2 ;;
    -h|--help) awk 'NR == 1 { next } /^#/ { sub(/^# ?/, ""); print; next } { exit }' "${BASH_SOURCE[0]}"; exit 0 ;;
    build|lockstep|parity|tests|t2|soundness) STEPS+=("$1"); shift ;;
    *) echo "engine-parity.sh: unknown argument: $1" >&2; exit 2 ;;
  esac
done
[[ ${#STEPS[@]} -gt 0 ]] || STEPS=(build lockstep parity tests)
[[ "$JOBS" -ge 1 ]] || JOBS=1
SHARD_K=0 SHARD_N=1
if [[ -n "$SHARD" ]]; then
  if ! [[ "$SHARD" =~ ^([0-9]+)/([0-9]+)$ ]] || (( BASH_REMATCH[1] >= BASH_REMATCH[2] )); then
    echo "engine-parity.sh: --shard wants K/N with 0 <= K < N, got $SHARD" >&2
    exit 2
  fi
  SHARD_K="${BASH_REMATCH[1]}" SHARD_N="${BASH_REMATCH[2]}"
fi

ENG="$WORK/eng"
FMT="$WORK/fmt"
POOL="$ENG/pdftex.pool"
INITEX="$ENG/flashtex-initex"

die() { echo "::error::$*" >&2; exit 1; }

# --- preflight: the oracle and its tools ------------------------------------
PDFTEX="$(command -v pdftex || true)"
[[ -n "$PDFTEX" ]] || die "no pdftex on PATH: TeX Live 2026 must be installed and first on PATH"
ver="$("$PDFTEX" --version | head -1)"
case "$ver" in
  *1.40.29*) ;;
  *) die "pdftex on PATH is '$ver', not the pinned pdfTeX 1.40.29 (TeX Live 2026)" ;;
esac
TEXBIN="$(dirname "$PDFTEX")"
[[ "$(command -v kpsewhich)" == "$TEXBIN/kpsewhich" ]] ||
  die "kpsewhich on PATH ($(command -v kpsewhich || echo none)) is not TeX Live 2026's ($TEXBIN)"
command -v qpdf >/dev/null || die "no qpdf on PATH: P-T2 needs it"
command -v python3 >/dev/null || die "no python3 on PATH"
echo "oracle: $ver ($PDFTEX); qpdf: $(qpdf --version | head -1); jobs: $JOBS; work: $WORK"

# No libstdc++ rpath here: where the compiler's libstdc++ lies outside the
# loader's default path (NixOS), crates/flashtex-engine/build.rs records it as
# an rpath of the engine's binaries and tests itself (rpath_cxx_runtime).

# Reproducible dates for every engine and oracle run.
export SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1

declare -a T_NAMES=() T_SECS=()
timed() {
  local name="$1"; shift
  local t0=$SECONDS rc=0
  ( "$@" ) || rc=$?
  local dt=$(( SECONDS - t0 ))
  echo "TIME $name ${dt}s rc=$rc"
  T_NAMES+=("$name$([[ $rc -eq 0 ]] || echo ' (FAILED)')"); T_SECS+=("$dt")
  return "$rc"
}

step_build() {
  cargo build --release --locked -p flashtex-engine --bin flashtex-initex
  rm -rf "$ENG" "$FMT"; mkdir -p "$ENG" "$FMT"
  cp "${CARGO_TARGET_DIR:-target}/release/flashtex-initex" "$ENG/"
  cp crates/flashtex-engine/pdftex.pool "$POOL"
  # tools/lockstep and tools/parity run the candidate through a link named
  # `pdftex`, as TeX Live runs its own.
  ln -sf "$INITEX" "$ENG/pdftex"
  # The two formats TeX Live's fmtutil makes for pdftex, with its options:
  # pdflatex.fmt for the fixtures, pdftex.fmt for l3build's unpack step (T2).
  local f ini
  for f in pdflatex pdftex; do
    ini=$f.ini; [[ $f == pdftex ]] && ini=pdfetex.ini
    if ! (cd "$FMT" && FLASHTEX_POOL="$POOL" "$ENG/pdftex" -ini -jobname=$f -progname=$f \
            -etex -translate-file=cp227.tcx "$ini" </dev/null >"$FMT/$f.out" 2>&1) ||
       [[ ! -s "$FMT/$f.fmt" ]]; then
      tail -n 40 "$FMT/$f.out" "$FMT/$f.log" 2>/dev/null || true
      die "the engine did not build $f.fmt from TeX Live's $ini"
    fi
  done
  ls -l "$FMT"/*.fmt
}

need_engine() {
  [[ -x "$INITEX" && -s "$FMT/pdflatex.fmt" ]] || die "no engine in $WORK: run the build step first"
}

step_lockstep() {
  need_engine
  # tools/lockstep/run.py runs its cases one after another (two engine runs
  # each, ~1,500 cases: over 8 minutes on a busy runner), so the cases are
  # split into $JOBS interleaved slices (run.py --shard K/N) run at once.
  local out="$WORK/lockstep.txt" n="$JOBS" k rc=0
  local -a pids=()
  rm -f "$WORK"/lockstep.shard-*.txt
  for (( k = 0; k < n; k++ )); do
    FLASHTEX_POOL="$POOL" python3 tools/lockstep/run.py --engine "$INITEX" --reference "$PDFTEX" \
      --shard "$k/$n" >"$WORK/lockstep.shard-$k.txt" 2>&1 &
    pids+=("$!")
  done
  for k in "${!pids[@]}"; do
    wait "${pids[$k]}" || { echo "lockstep slice $k/$n: exit $?"; rc=1; }
  done
  for (( k = 0; k < n; k++ )); do cat "$WORK/lockstep.shard-$k.txt"; done >"$out"
  grep -vE '^PASS ' "$out" | grep -vE '^[0-9]+ cases, |^accounting: ' | tail -n 40 || true
  # One total line, as an unsharded run prints it.
  awk '/^[0-9]+ cases, [0-9]+ equal, [0-9]+ differ$/ { c += $1; e += $3; d += $5 }
       /^accounting: [0-9]+ / { a += $2 }
       END { printf "%d cases, %d equal, %d differ (%d slices)\naccounting: %d cases differ\n", c, e, d, '"$n"', a }' "$out"
  [[ $rc -eq 0 ]] || die "lockstep: the engine differs from pdfTeX ($out)"
}

step_parity() {
  need_engine
  python3 tools/parity/parity.py --tier fixtures --engine "$INITEX" \
    --engine-env "FLASHTEX_FORMATS=$FMT" --engine-env "FLASHTEX_POOL=$POOL" \
    --oracle-pdftex "$PDFTEX" --texbin "$TEXBIN" \
    --pt on --raster none -j "$JOBS" \
    --out "$WORK/parity" --work "$WORK/parity-work" \
    ${SHARD:+--shard "$SHARD"} --require-pt
}

step_tests() {
  local -a sel=()
  if (( SHARD_N > 1 )); then
    # Every N-th test target, from the K-th: the library, the binaries, then
    # each integration test (tests/*.rs, in name order). cargo runs test
    # binaries one after another, so this is what splits the wall time.
    local -a targets=(--lib --bins)
    local f i
    for f in crates/flashtex-engine/tests/*.rs; do
      f="${f##*/}"
      targets+=("--test=${f%.rs}")
    done
    for i in "${!targets[@]}"; do
      if (( i % SHARD_N == SHARD_K )); then sel+=("${targets[$i]}"); fi
    done
    echo "test targets of slice $SHARD: ${sel[*]}"
  fi
  FLASHTEX_REQUIRE_TEXLIVE=1 cargo test --release --locked -p flashtex-engine --no-fail-fast ${sel[@]+"${sel[@]}"}
  # A target selection leaves out the doc tests: slice 0 runs them.
  if (( SHARD_N > 1 && SHARD_K == 0 )); then
    FLASHTEX_REQUIRE_TEXLIVE=1 cargo test --release --locked -p flashtex-engine --doc
  fi
}

step_t2() {
  need_engine
  sh tools/latex-suites/fetch.sh
  # The suites' EXPECTED-FAILURES.txt matches the TeX Live PINS.txt names;
  # the runner's TeX Live may be newer (on the NixOS PC, TeX Live's own
  # pdftex has 13 failures that file does not list). So the baseline is the
  # reference on THIS TeX Live: the suites through TeX Live's pdftex, then
  # through the engine, which may fail nothing the reference passes.
  local rc=0
  python3 tools/latex-suites/run.py --engine "$PDFTEX" --suite all >"$WORK/t2-reference.txt" 2>&1 || rc=$?
  if [[ $rc -gt 1 ]]; then tail -n 30 "$WORK/t2-reference.txt"; die "T2: the reference run did not complete (exit $rc)"; fi
  rc=0
  # l3build's unpack and check runs take this engine's formats and pool from
  # the shim run.py puts first on PATH, which exports exactly the
  # --engine-env pairs. They must be flags: run.py removes every FLASHTEX_*
  # variable from the environment it inherits, so the reference run can
  # never see one. (Passed through the environment, as until 2026-10-05,
  # they never reached the engine: it looked for pdftex.fmt itself, tried to
  # build it and stopped at the `**` prompt, and all 1,385 tests failed.)
  python3 tools/latex-suites/run.py --engine "$INITEX" --suite all \
    --engine-env "FLASHTEX_FORMATS=$FMT" --engine-env "FLASHTEX_POOL=$POOL" \
    >"$WORK/t2-engine.txt" 2>&1 || rc=$?
  if [[ $rc -gt 1 ]]; then tail -n 30 "$WORK/t2-engine.txt"; die "T2: the engine run did not complete (exit $rc)"; fi
  grep -E ': PASS [0-9]+ / FAIL' "$WORK/t2-reference.txt" | sed 's/^/reference: /' || true
  grep -E ': PASS [0-9]+ / FAIL' "$WORK/t2-engine.txt" | sed 's/^/engine:    /' || true
  python3 tools/latex-suites/compare_failures.py "$WORK/t2-reference.txt" "$WORK/t2-engine.txt"
}

step_soundness() {
  FLASHTEX_REQUIRE_TEXLIVE=1 cargo test --release --locked -p flashtex-engine --test host_incremental \
    -- --ignored --exact every_fixture_edits_equal_scratch_compiles --nocapture
}

# The timings table, also when a step fails: a red run still says how far it got.
summary() {
  [[ -n "${GITHUB_STEP_SUMMARY:-}" ]] || return 0
  {
    echo "### engine parity ($(uname -sm), $JOBS jobs)"
    echo
    echo "| step | seconds |"
    echo "|---|---:|"
    for i in "${!T_NAMES[@]}"; do echo "| ${T_NAMES[$i]} | ${T_SECS[$i]} |"; done
    if [[ -f "$WORK/parity/report.md" ]]; then echo; sed -n '1,40p' "$WORK/parity/report.md"; fi
  } >>"$GITHUB_STEP_SUMMARY"
}
trap summary EXIT

for s in "${STEPS[@]}"; do
  echo "== $s"
  timed "$s" "step_$s"
done
