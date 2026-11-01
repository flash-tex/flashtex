#!/usr/bin/env bash
# l6ab.sh WORK REF...: build each ref (release, a worktree each) and run l6ab.py over them (lane
# L6, docs/design/engine-v2/L6.md §4; .github/workflows/l6-ab.yml runs it on the NixOS runner).
#
#   WORK   a scratch directory: the worktrees, the engines, the documents, the results (WORK/r)
#   REF    git refs to build and compare; the first is the baseline of every Δ
#
# From the environment (all optional):
#   L6AB_DOCS      l6ab.py --docs (books such as infdesc are fetched with mkbook.py)
#   L6AB_VARIANTS  NAME=VAR=VALUE VAR=VALUE;NAME=...  (';' between variants; default: base only)
#   L6AB_REPS      measured compiles per cell (default 3)
#   L6AB_PERF      1: also a perf record of the first ref per document
#   L6AB_MACRO     documents for one FLASHTEX_MACRO_PROFILE run per engine and variant
#   L6AB_CACHE     where mkbook.py's archives are kept (default ~/.cache/flashtex-ci/l6ab)
#   L6AB_TESTS     flashtex-engine test targets (names only), run in the last ref's worktree in the
#                  release profile before any compile (`cargo test --release -p flashtex-engine
#                  --test NAME`; the tail of each is in WORK/r/tests.txt)
#   CARGO_TARGET_DIR_BASE  one target directory per ref slot: BASE-0, BASE-1, ...
# Every build and compile runs under nice.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORK=${1:?WORK}
shift
[ $# -ge 1 ] || { echo "l6ab.sh: no ref" >&2; exit 2; }
mkdir -p "$WORK/r" "$WORK/tmp"
WORK=$(cd "$WORK" && pwd)
export INCR_BENCH_DIR="$WORK/ib" TMPDIR="$WORK/tmp"
mkdir -p "$INCR_BENCH_DIR"
CACHE=${L6AB_CACHE:-$HOME/.cache/flashtex-ci/l6ab}
mkdir -p "$CACHE/cache"
ln -sfn "$CACHE/cache" "$INCR_BENCH_DIR/cache"
TBASE=${CARGO_TARGET_DIR_BASE:-$WORK/target}

names=()
i=0
for ref in "$@"; do
  n="e$i-$(echo "$ref" | tr -c 'A-Za-z0-9._\n-' '_' | tail -c 40)"
  names+=("$n")
  src="$WORK/src-$i"
  git -C "$ROOT" worktree remove --force "$src" 2>/dev/null || true
  git -C "$ROOT" worktree add --detach "$src" "$ref" >/dev/null
  echo "build $ref = $(git -C "$src" rev-parse --short HEAD) ($(git -C "$src" log -1 --format=%s | head -c 90))"
  (cd "$src" && CARGO_TARGET_DIR="$TBASE-$i" nice -n 5 cargo build --release --locked -q \
     -p flashtex-engine -p flashtex-display-list)
  (cd "$src" && CARGO_TARGET_DIR="$TBASE-$i" bash tools/incr-bench/mkeng.sh "$n" >/dev/null)
  if [ $((i + 1)) = $# ] && [ -n "${L6AB_TESTS:-}" ]; then
    for t in $L6AB_TESTS; do
      [[ $t =~ ^[a-z0-9_]+$ ]] || { echo "l6ab.sh: not a test name: $t" >&2; exit 2; }
      if (cd "$src" && CARGO_TARGET_DIR="$TBASE-$i" nice -n 5 cargo test --release --locked -q -p flashtex-engine \
            --test "$t" > "$WORK/r/test-$t.log" 2>&1); then r=ok; else r=FAILED; fi
      { echo "test $t ($ref): $r"; tail -n 6 "$WORK/r/test-$t.log"; } | tee -a "$WORK/r/tests.txt"
    done
  fi
  git -C "$ROOT" worktree remove --force "$src"
  i=$((i + 1))
done

python3 "$ROOT/tools/incr-bench/mkdocs.py" >/dev/null
DOCS=${L6AB_DOCS:-plain-100,full-100,beamer-30,fx:beamer-madrid,fx:framed-tikz-theorems}
for b in infdesc infdesc-x2; do
  case ",$DOCS,${L6AB_MACRO:-}," in *",$b,"*) python3 "$ROOT/tools/incr-bench/mkbook.py" "$b" | tail -1 ;; esac
done

args=(--engines "$(IFS=,; echo "${names[*]}")" --docs "$DOCS" --reps "${L6AB_REPS:-3}" --out "$WORK/r")
IFS=';' read -r -a vs <<< "${L6AB_VARIANTS:-}"
for v in "${vs[@]}"; do [ -n "$v" ] && args+=(--variant "$v"); done
[ "${L6AB_PERF:-0}" = 1 ] && args+=(--perf-record)
[ -n "${L6AB_MACRO:-}" ] && args+=(--macro-profile "$L6AB_MACRO")
for n in "${names[@]}"; do echo "$n: $(cat "$INCR_BENCH_DIR/$n/HEAD")"; done | tee "$WORK/r/engines.txt"
status=0
nice -n 5 python3 "$ROOT/tools/incr-bench/l6ab.py" "${args[@]}" || status=$?
if grep -q ': FAILED$' "$WORK/r/tests.txt" 2>/dev/null; then echo "l6ab.sh: a test failed" >&2; status=1; fi
exit $status
