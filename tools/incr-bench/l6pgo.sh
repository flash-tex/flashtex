#!/usr/bin/env bash
# l6pgo.sh WORK: L6's E1 ablation (docs/design/engine-v2/L6.md §6): this checkout built twice, the
# default release build and scripts/build-engine-dist.sh's profile-guided one, then l6ab.py's
# interleaved cold compiles of both (.github/workflows/l6-pgo.yml runs it on GitHub's M1 VMs).
#
# The documents are ones the PGO training does not compile: build-engine-dist.sh trains on every
# fixture and on plain-/full-10 and -100, so the default set is plain-300, full-300, beamer-30 and
# Infinite Descent (L6PGO_DOCS overrides; L6PGO_REPS, default 5). The two engines must give the
# same PDF bytes on every document (l6ab.py fails the run otherwise). Writes WORK/r.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORK=${1:?WORK}
mkdir -p "$WORK/r"
WORK=$(cd "$WORK" && pwd)
export INCR_BENCH_DIR="$WORK/ib"
mkdir -p "$INCR_BENCH_DIR"
cd "$ROOT"

mk() { # NAME BINARY-DIRECTORY: an engine directory and its pdflatex format, as mkeng.sh makes them
  local d="$INCR_BENCH_DIR/$1" f="$INCR_BENCH_DIR/fmt-$1"
  mkdir -p "$d" "$f"
  cp "$2/flashtex-initex" "$d/"
  cp crates/flashtex-engine/pdftex.pool "$d/"
  git rev-parse --short HEAD > "$d/HEAD"
  (cd "$f" && SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 FLASHTEX_POOL="$d/pdftex.pool" "$d/flashtex-initex" -ini \
     -jobname=pdflatex -progname=pdflatex -etex -translate-file=cp227.tcx pdflatex.ini </dev/null >/dev/null 2>&1) || true
  [ -s "$f/pdflatex.fmt" ] || { tail -5 "$f/pdflatex.log" || true; echo "l6pgo.sh: no format for $1" >&2; exit 1; }
}

# The instrumented build's frames are larger: on GitHub's M1 VMs its format build overflowed the
# main thread's 8 MB stack (a segmentation fault at once, run 41334149491). Both builds run under
# the same, larger limit (the command line runs TeX on the main thread).
ulimit -s 65520 2>/dev/null || ulimit -s hard 2>/dev/null || true
echo "stack limit: $(ulimit -s)"
echo "== the default release build ($(git rev-parse --short HEAD))"
cargo build --release --locked -q -p flashtex-engine
mk def target/release
echo "== the profile-guided build (scripts/build-engine-dist.sh)"
# The instrumented initex stopped at once in LLVM's value profiling
# (`__llvm_profile_instrument_target`, EXC_BAD_ACCESS at 0x200000000; run 41346722990). L6PGO_GEN
# adds rustc flags to the instrumented build only, in a copy of the script beside it (its root is
# its directory's parent); the default turns value profiling off, which keeps the branch and block
# counts PGO uses and drops only indirect-call promotion. L6PGO_GEN= runs the script as it is.
GEN=${L6PGO_GEN--Cllvm-args=-disable-vp=true}
BED=scripts/build-engine-dist.sh
if [ -n "$GEN" ]; then
  BED=scripts/.l6pgo-build-engine-dist.sh
  sed "s|-Cprofile-generate=\$RAW|-Cprofile-generate=\$RAW $GEN|" scripts/build-engine-dist.sh > "$BED"
  chmod +x "$BED"
  grep -q -- "$GEN" "$BED" || { echo "l6pgo.sh: L6PGO_GEN not applied" >&2; exit 1; }
  echo "instrumented build: RUSTFLAGS += $GEN"
fi
"$BED" --out "$WORK/dist" || {
  tail -n 30 target/pgo-train/fmt/pdflatex.log 2>/dev/null || true
  cp target/pgo-train/fmt/pdflatex.log "$WORK/r/pgo-format.log" 2>/dev/null || true
  # where the instrumented engine stops (it segfaulted at once on GitHub's M1 VMs, with an 8 MB
  # and a 64 MB stack alike: runs 41334149491, 41339976306)
  if command -v lldb >/dev/null; then
    (cd target/pgo-train/fmt && SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 \
       FLASHTEX_POOL="$ROOT/crates/flashtex-engine/pdftex.pool" LLVM_PROFILE_FILE="$WORK/tmp-%p.profraw" \
       lldb --batch -o run -o "bt 40" -o "register read" -- "$ROOT/target/pgo-gen/release/flashtex-initex" -ini \
       -jobname=pdflatex -progname=pdflatex -etex -translate-file=cp227.tcx pdflatex.ini </dev/null 2>&1 \
       | tail -n 80 | tee "$WORK/r/pgo-crash.txt") || true
  fi
  exit 1
}
mk pgo "$WORK/dist"
cp "$WORK/dist/flashtex-engine.profdata" "$WORK/r/" 2>/dev/null || true

python3 tools/incr-bench/mkdocs.py >/dev/null
DOCS=${L6PGO_DOCS:-plain-300,full-300,beamer-30,infdesc}
case ",$DOCS," in *,infdesc,*) python3 tools/incr-bench/mkbook.py infdesc | tail -1 ;; esac
printf 'def: default release build\npgo: scripts/build-engine-dist.sh\n' > "$WORK/r/engines.txt"
python3 tools/incr-bench/l6ab.py --engines def,pgo --docs "$DOCS" --reps "${L6PGO_REPS:-5}" --out "$WORK/r"
