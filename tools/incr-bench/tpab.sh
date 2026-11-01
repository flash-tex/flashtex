#!/usr/bin/env bash
# tpab.sh WORK REF...: build each ref (release, a worktree each) and run tpab.py over them, with
# TeX Live's pdftex beside them (lane THROUGHPUT; adapted from lane L6's l6ab.sh;
# .github/workflows/throughput-ab.yml runs it on the NixOS runner). Measurement only.
#
#   WORK   a scratch directory: the worktrees, the engines, the documents, the results (WORK/r)
#   REF    git refs to build and compare; the first is the baseline of every delta
#
# From the environment (all optional):
#   TPAB_DOCS      tpab.py --docs (default: arxiv13, the 13 e-prints of arxiv-2025-01 shard 0/12)
#   TPAB_REPS      measured compiles per cell (default 3)
#   TPAB_PERF      1: also a perf record of the first ref per document (and of pdftex)
#   TPAB_CACHE     where the arXiv sources are kept (default ~/.cache/flashtex-agents/throughput)
#   CARGO_TARGET_DIR_BASE  one target directory per ref slot: BASE-0, BASE-1, ...
# Every build and compile runs under nice.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORK=${1:?WORK}
shift
[ $# -ge 1 ] || { echo "tpab.sh: no ref" >&2; exit 2; }
mkdir -p "$WORK/r" "$WORK/tmp"
WORK=$(cd "$WORK" && pwd)
export INCR_BENCH_DIR="$WORK/ib" TMPDIR="$WORK/tmp"
mkdir -p "$INCR_BENCH_DIR"
CACHE=${TPAB_CACHE:-$HOME/.cache/flashtex-agents/throughput}
mkdir -p "$CACHE/parity"
TBASE=${CARGO_TARGET_DIR_BASE:-$WORK/target}
# TPAB_ANNOTATE: line tables for perf annotate (the perf profile's setting; same codegen)
if [ -n "${TPAB_ANNOTATE:-}" ]; then
  export CARGO_PROFILE_RELEASE_DEBUG=line-tables-only CARGO_PROFILE_RELEASE_STRIP=none
  TBASE=$TBASE-lines
fi

names=()
i=0
for spec in "$@"; do
  # REF or REF@FEATURE,FEATURE (flashtex-engine's benchmarking features, e.g. unchecked-reads)
  ref=${spec%%@*}
  feat=""
  [ "$spec" != "$ref" ] && feat=${spec#*@}
  n="e$i-$(echo "$spec" | tr -c 'A-Za-z0-9._\n-' '_' | tail -c 40)"
  names+=("$n")
  src="$WORK/src-$i"
  git -C "$ROOT" worktree remove --force "$src" 2>/dev/null || true
  git -C "$ROOT" worktree add --detach "$src" "$ref" >/dev/null
  echo "build $spec = $(git -C "$src" rev-parse --short HEAD) ($(git -C "$src" log -1 --format=%s | head -c 90))"
  fargs=()
  [ -n "$feat" ] && fargs=(--features "$(echo "$feat" | sed -E 's/(^|,)/ flashtex-engine\//g')")
  (cd "$src" && CARGO_TARGET_DIR="$TBASE-$i" nice -n 5 cargo build --release --locked -q \
     -p flashtex-engine -p flashtex-display-list "${fargs[@]}")
  (cd "$src" && CARGO_TARGET_DIR="$TBASE-$i" bash tools/incr-bench/mkeng.sh "$n" >/dev/null)
  git -C "$ROOT" worktree remove --force "$src"
  i=$((i + 1))
done

DOCS=${TPAB_DOCS:-arxiv13}
case ",$DOCS," in
  *,arxiv13,*|*,ax:*)
    python3 "$ROOT/tools/parity/corpus.py" --cache "$CACHE/parity" --delay 3 fetch \
      --manifest "$ROOT/tools/parity/corpus/arxiv-2025-01.json" --shard 0/12 | tail -2 ;;
esac
case ",$DOCS," in *,plain-*|*,full-*) python3 "$ROOT/tools/incr-bench/mkdocs.py" >/dev/null ;; esac

args=(--engines "$(IFS=,; echo "${names[*]}")" --docs "$DOCS" --reps "${TPAB_REPS:-3}" --out "$WORK/r"
      --arxiv-cache "$CACHE/parity")
[ "${TPAB_PERF:-0}" = 1 ] && args+=(--perf-record)
[ -n "${TPAB_ANNOTATE:-}" ] && args+=(--annotate "$TPAB_ANNOTATE")
for n in "${names[@]}"; do echo "$n: $(cat "$INCR_BENCH_DIR/$n/HEAD")"; done | tee "$WORK/r/engines.txt"
nice -n 5 python3 "$ROOT/tools/incr-bench/tpab.py" "${args[@]}"
